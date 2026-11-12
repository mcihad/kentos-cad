//! The shift between two orthophotos measured in the images themselves where they overlap (plan/map-cad.md MAP-12,
//! docs/adr/2460), `cad.orthophoto.compare`'s `seam`: no point read by hand.
//!
//! - **Places:** every `step` metres across the overlap, inside it by half the window and the search (north to south,
//!   west to east). A place is the centre of the first image's pixel it falls in.
//! - **Match:** the first image's `window` × `window` patch (its grey: the mean of the first three bands) sought in the
//!   second within `search` pixels each way by zero-mean normalized cross-correlation, from the second's pixel whose
//!   centre is nearest the place; the peak (the first of equals, rows then columns) refined to a fraction of a pixel by
//!   a parabola through it and its two neighbours, each way apart; or (docs/adr/3260, `subpixel: bicubic`) by the
//!   second image itself resampled with cubic convolution (Keys, a = −1/2) at fractions of a pixel round the peak, the
//!   correlation's highest on a grid of 1/8 pixel within half a pixel, then of 1/64 pixel within 1/8 of it (the first of
//!   equals, rows then columns). The correlation said is the whole pixel's peak's either way.
//! - **Shift:** the second's matched place − the place (metres).
//! - **State:** `flat` when either patch has no texture (no correlation); `edge` when the peak is on the search's edge
//!   (the true one may lie beyond; no shift); `low` below the correlation asked (its shift said, not counted); else `ok`.
//!
//! The two images must have pixels of one size and no turn (north up); or (docs/adr/3660, `resample`) the second is
//! resampled onto the first's grid: at each place, the block of the first's pixel centres its patches and search use
//! (one pixel more round them for `bicubic`), each centre mapped into the second's pixels by the inverse of its affine
//! and read there by cubic convolution (Keys, a = −1/2). A place whose block reaches past the second's pixels (the
//! cubic's 4 × 4 included) is outside the overlap. The match, the refinement and the shift are then the first grid's;
//! `bicubic` samples the second the same way at fractions of the first's pixel. A turned first (docs/adr/4060) is the
//! grid itself: a place is the centre of its pixel the place falls in, its whole affine (GDAL's: east = t0 + col·t1 +
//! row·t2, north = t3 + col·t4 + row·t5) takes its pixels to the ground and its inverse the ground to them, and a place
//! whose window leaves its pixels is outside the overlap; the shift is said in east and north. Nothing is written.
//!
//! Along a drawn seam line (`line`, docs/adr/2860): the places are every `step` metres along it from its start (the
//! k-th at k × step, the line's length included when it falls on a step), each the point at that distance on its edges,
//! straight or (docs/adr/3260) arcs, an arc's distance along the arc; those inside the overlap (as the grid's) are
//! measured, in the line's order, each with its distance; the rest are counted as outside.

use std::path::Path;

use kentos_commands::{
    Entity, OrthoPatch, OrthoPoint, OrthoSeam, OrthoSeamAsk, OrthoSeamLine, OrthoSeamShared, OrthoSubpixel, RasterEntity,
};
use kentos_domain::{Document, Uuid};
use kentos_formats::raster::source::Region;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::bulge::{BulgeArc, bulge_arc, bulge_at};

use crate::checks::{Stop, error};
use crate::codes;
use crate::orthophoto_check::stats;
use crate::raster_files::{self, Bytes, Opened};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The seam question's own rules.
pub(crate) fn asked(a: &OrthoSeamAsk) -> Result<(), Stop> {
    let bad = |m: &str, path: &str| Err(fail(codes::INVALID_INPUT, m.to_owned(), path));
    if !(a.step.is_finite() && a.step > 0.0) {
        return bad("Kayıklık ölçümünün adımı 0'dan büyük bir uzunluk olmalı (m).", "seam.step");
    }
    if a.window % 2 == 0 || !(5..=255).contains(&a.window) {
        return bad("Pencere tek sayıda piksel olmalı, 5 ile 255 arasında.", "seam.window");
    }
    if !(1..=64).contains(&a.search) {
        return bad("Arama 1 ile 64 piksel arasında olmalı.", "seam.search");
    }
    if let Some(c) = a.correlation
        && !(c.is_finite() && c > 0.0 && c <= 1.0)
    {
        return bad("En az korelasyon 0'dan büyük, en çok 1 olmalı; her yer sayılacaksa boş bırakın.", "seam.correlation");
    }
    // docs/adr/4460: a drawn line or the seamline polygons, not both; two polygons.
    if a.line.is_some() && a.polygons.is_some() {
        return bad(
            "Ölçüyü ya çizilmiş bir birleşme çizgisi (line) ya da iki mozaik elemanı (polygons) boyunca isteyin; ikisi birden olmaz.",
            "seam.polygons",
        );
    }
    if let Some([p, q]) = &a.polygons
        && p.trim() == q.trim()
    {
        return bad("İki mozaik elemanı aynı nesne; iki görüntünün ayrı mozaik elemanlarını verin.", "seam.polygons");
    }
    Ok(())
}

/// A raster's pixels opened: its library bytes or its file, found from `folder` when relative.
pub(crate) fn opened(doc: &Document, r: &RasterEntity, folder: Option<&str>, path: &str) -> Result<Opened, Stop> {
    let bytes = if let Some(id) = &r.raster.asset {
        let Some(b) = crate::rasters_georef::asset_bytes(doc, id) else {
            return Err(fail(codes::FILE_NOT_FOUND, format!("Ortofotonun görüntüsü “{id}” projenin kitaplığında yok."), path));
        };
        Bytes::Memory(b)
    } else if let Some(file) = &r.raster.file {
        if Path::new(file).is_relative() && folder.is_none() {
            return Err(fail(
                codes::INVALID_FILE_PATH,
                format!("Ortofotonun dosyası “{file}” göreli bir yol; çizimin klasörünü (seam.folder) verin."),
                path,
            ));
        }
        let found = crate::links::find_in(file, None, folder.map(Path::new), &doc.settings().xref_paths);
        let Some(at) = found.at else {
            return Err(fail(codes::FILE_NOT_FOUND, format!("Ortofotonun dosyası “{file}” bulunamadı."), path));
        };
        Bytes::file(&at).map_err(|e| fail(codes::IMPORT_UNREADABLE, format!("Ortofotonun dosyası okunamadı: {e}"), path))?
    } else {
        return Err(fail(codes::FILE_NOT_FOUND, "Ortofotonun görüntüsü yok: ne dosyası ne kitaplıktaki baytları.".into(), path));
    };
    raster_files::open(bytes).map_err(|e| fail(codes::IMPORT_UNREADABLE, format!("Ortofotonun görüntüsü okunamadı: {e}"), path))
}

/// A region's grey at (i, j): the mean of its first three bands (one band: that band).
fn grey(r: &Region, i: u32, j: u32) -> f64 {
    let n = r.bands.clamp(1, 3);
    (0..n).map(|b| r.at(i, j, b)).sum::<f64>() / f64::from(n)
}

/// The zero-mean normalized cross-correlation of two equal patches; none when either has no texture.
fn ncc(a: &[f64], b: &[f64]) -> Option<f64> {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        let (p, q) = (x - ma, y - mb);
        sab += p * q;
        saa += p * p;
        sbb += q * q;
    }
    (saa > 0.0 && sbb > 0.0).then(|| sab / (saa * sbb).sqrt())
}

/// Keys' cubic convolution kernel with a = −1/2 (docs/adr/3260).
pub(crate) fn keys(t: f64) -> f64 {
    let t = t.abs();
    if t <= 1.0 {
        (1.5 * t - 2.5) * t * t + 1.0
    } else if t < 2.0 {
        ((-0.5 * t + 2.5) * t - 4.0) * t + 2.0
    } else {
        0.0
    }
}

/// The grey of region `r` at (`x`, `y`) in its pixels (whole numbers: the pixels' centres) by cubic convolution: the 4
/// × 4 pixels round it, each row's sum weighted by its column weights, then the rows' by theirs.
fn bicubic(r: &Region, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (tx, ty) = (x - x0, y - y0);
    let (i0, j0) = (x0 as i64, y0 as i64);
    let mut sum = 0.0;
    for n in -1..=2_i64 {
        let mut row = 0.0;
        for m in -1..=2_i64 {
            row += grey(r, (i0 + m) as u32, (j0 + n) as u32) * keys(tx - m as f64);
        }
        sum += row * keys(ty - n as f64);
    }
    sum
}

/// docs/adr/3260: the whole pixel's peak, the second patch's top left at (`x`, `y`) in the block's pixels, refined by
/// the second image resampled at fractions of a pixel (`at`: the second's grey at a point of the block, by cubic
/// convolution): the correlation's highest on a grid of 1/8 pixel within half a pixel, then of 1/64 pixel within 1/8 of
/// it, rows then columns, the first of equals kept. The fraction found.
fn refine_bicubic(first: &[f64], at: &dyn Fn(f64, f64) -> f64, w: u32, x: f64, y: f64) -> (f64, f64) {
    let score = |fu: f64, fv: f64| -> Option<f64> {
        let second: Vec<f64> = (0..w)
            .flat_map(|j| (0..w).map(move |i| (i, j)))
            .map(|(i, j)| at(x + f64::from(i) + fu, y + f64::from(j) + fv))
            .collect();
        ncc(first, &second)
    };
    // A grid of `step` within `reach` steps of (cu, cv), rows then columns, the first of equals kept.
    let grid = |cu: f64, cv: f64, step: f64, reach: i32| -> Option<(f64, f64)> {
        let mut found: Option<(f64, f64, f64)> = None;
        for kv in -reach..=reach {
            for ku in -reach..=reach {
                let (fu, fv) = (cu + f64::from(ku) * step, cv + f64::from(kv) * step);
                if let Some(c) = score(fu, fv)
                    && found.is_none_or(|f| c > f.2)
                {
                    found = Some((fu, fv, c));
                }
            }
        }
        found.map(|f| (f.0, f.1))
    };
    let Some((cu, cv)) = grid(0.0, 0.0, 0.125, 4) else { return (0.0, 0.0) };
    grid(cu, cv, 0.015625, 8).unwrap_or((cu, cv))
}

/// The parabola's vertex through (−1, l), (0, c), (1, r): the peak's fraction of a pixel.
fn vertex(l: f64, c: f64, r: f64) -> f64 {
    let den = l - 2.0 * c + r;
    if den < 0.0 { (l - r) / (2.0 * den) } else { 0.0 }
}

/// The seam line's vertices (east, north) and each edge's bulge (docs/adr/3260: an arc's too): a line, or a polyline
/// (a closed one back to its start); refused at `seam.line` as the contract says.
fn seam_line(doc: &Document, uid: &str) -> Result<(Vec<[f64; 2]>, Vec<f64>), Stop> {
    let bad = |m: String| fail(codes::INVALID_INPUT, m, "seam.line");
    let e = Uuid::parse_str(uid.trim())
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| bad(format!("“{uid}” kimlikli birleşme çizgisi çizimde yok.")))?;
    Ok(match e {
        Entity::Line(l) => (vec![[l.a.x, l.a.y], [l.b.x, l.b.y]], vec![0.0]),
        Entity::Polyline(p) => {
            let mut pts: Vec<[f64; 2]> = p.pts.iter().map(|v| [v.x, v.y]).collect();
            let edges = if p.closed { pts.len() } else { pts.len().saturating_sub(1) };
            let bulges = (0..edges).map(|i| bulge_at(p.bulges.as_deref(), i)).collect();
            if p.closed && let Some(first) = pts.first().copied() {
                pts.push(first);
            }
            (pts, bulges)
        }
        _ => return Err(bad("Birleşme çizgisi bir çizgi ya da çoklu çizgi olmalı.".into())),
    })
}

/// An edge from `a` to `b` of `bulge`: its length and, an arc's, the arc (docs/adr/3260).
fn edge(a: [f64; 2], b: [f64; 2], bulge: f64) -> (f64, Option<BulgeArc>) {
    match bulge_arc(Vec2::new(a[0], a[1]), Vec2::new(b[0], b[1]), bulge) {
        Some(arc) => (arc.r * arc.sweep.abs(), Some(arc)),
        None => ((b[0] - a[0]).hypot(b[1] - a[1]), None),
    }
}

/// The line's length: its edges', an arc's along the arc.
fn line_length(pts: &[[f64; 2]], bulges: &[f64]) -> f64 {
    pts.windows(2).enumerate().map(|(k, w)| edge(w[0], w[1], bulges.get(k).copied().unwrap_or(0.0)).0).sum()
}

/// The point at distance `s` along the line (edges of no length skipped; past the end, the end); on an arc, the point
/// at that distance along the arc from its start.
fn along(pts: &[[f64; 2]], bulges: &[f64], s: f64) -> [f64; 2] {
    let mut walked = 0.0;
    for (k, w) in pts.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let (length, arc) = edge(a, b, bulges.get(k).copied().unwrap_or(0.0));
        if length == 0.0 {
            continue;
        }
        if s <= walked + length {
            let t = s - walked;
            return match arc {
                Some(arc) => {
                    let angle = arc.a0 + arc.sweep.signum() * t / arc.r;
                    [arc.c.x + arc.r * angle.cos(), arc.c.y + arc.r * angle.sin()]
                }
                None => {
                    let f = t / length;
                    [a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1])]
                }
            };
        }
        walked += length;
    }
    pts.last().copied().unwrap_or([0.0, 0.0])
}

/// The overlap of the two images measured; `limit` the shift's (metres).
pub(crate) fn measure(
    doc: &Document,
    first: &RasterEntity,
    second: &RasterEntity,
    a: &OrthoSeamAsk,
    limit: f64,
) -> Result<OrthoSeam, Stop> {
    let (fa, sa) = (first.raster.affine, second.raster.affine);
    let same = |p: f64, q: f64| (p - q).abs() <= 1e-9 * p.abs().max(1.0);
    let unturned = |t: [f64; 6]| t[2] == 0.0 && t[4] == 0.0 && t[1] > 0.0 && t[5] < 0.0;
    let alike = unturned(fa) && unturned(sa) && same(fa[1], sa[1]) && same(fa[5], sa[5]);
    // docs/adr/3660: the second resampled onto the first's grid, asked; the grid faces north.
    let resample = !alike && a.resample;
    let det = sa[1] * sa[5] - sa[2] * sa[4];
    if !alike && !a.resample {
        return Err(fail(
            codes::INVALID_INPUT,
            "Kayıklık ölçümü pikselleri aynı boyda ve kuzeye bakan (dönüksüz) iki görüntüde yapılır; ikincisini birincinin ızgarasına yeniden örneklemek için resample verin.".into(),
            "seam",
        ));
    }
    // docs/adr/4060: a turned first is the grid itself (only resampled: two alike images face north).
    let turned = !unturned(fa);
    let det_a = fa[1] * fa[5] - fa[2] * fa[4];
    if turned && !(det_a.is_finite() && det_a != 0.0) {
        return Err(fail(codes::INVALID_INPUT, "Birinci görüntünün piksel dönüşümü tersinir değil; ızgara olamaz.".into(), "seam.resample"));
    }
    if resample && !(det.is_finite() && det != 0.0) {
        return Err(fail(codes::INVALID_INPUT, "İkinci görüntünün piksel dönüşümü tersinir değil; yeniden örneklenemez.".into(), "seam.resample"));
    }
    // The margin's pixel: the first's along its axes; a turned first's longer side.
    let (px, py) = if turned {
        let side = fa[1].hypot(fa[4]).max(fa[2].hypot(fa[5]));
        (side, side)
    } else {
        (fa[1], -fa[5])
    };
    let extent = |t: [f64; 6], w: u32, h: u32| (t[0], t[0] + f64::from(w) * t[1], t[3] + f64::from(h) * t[5], t[3]);
    // A turned or resized second: the box of its corners (the places then kept where its pixels reach, below).
    let corners = |t: [f64; 6], w: u32, h: u32| {
        let pts = [(0.0, 0.0), (f64::from(w), 0.0), (0.0, f64::from(h)), (f64::from(w), f64::from(h))]
            .map(|(i, j)| (t[0] + i * t[1] + j * t[2], t[3] + i * t[4] + j * t[5]));
        let (xs, ys) = (pts.map(|p| p.0), pts.map(|p| p.1));
        let lo = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
        let hi = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
        (lo(xs), hi(xs), lo(ys), hi(ys))
    };
    let (ax0, ax1, ay0, ay1) =
        if turned { corners(fa, first.raster.width, first.raster.height) } else { extent(fa, first.raster.width, first.raster.height) };
    let (bx0, bx1, by0, by1) =
        if resample { corners(sa, second.raster.width, second.raster.height) } else { extent(sa, second.raster.width, second.raster.height) };
    let half = i64::from(a.window / 2);
    let reach = half + i64::from(a.search) + 1;
    let (ox0, ox1) = (ax0.max(bx0) + reach as f64 * px, ax1.min(bx1) - reach as f64 * px);
    let (oy0, oy1) = (ay0.max(by0) + reach as f64 * py, ay1.min(by1) - reach as f64 * py);
    if !(ox0 <= ox1 && oy0 <= oy1) {
        return Err(fail(
            codes::INVALID_INPUT,
            "Görüntüler örtüşmüyor ya da örtüşmeleri pencere ve arama için dar; pencereyi ya da aramayı küçültün.".into(),
            "seam",
        ));
    }
    // A point of the first's pixels (whole numbers: their corners) on the ground: a turned first's whole affine
    // (docs/adr/4060).
    let ground_of = move |i: f64, j: f64| -> [f64; 2] {
        if turned { [fa[0] + i * fa[1] + j * fa[2], fa[3] + i * fa[4] + j * fa[5]] } else { [fa[0] + i * fa[1], fa[3] + j * fa[5]] }
    };
    // docs/adr/3660: where the first's pixel centre (i, j) falls in the second's pixels (whole numbers: their centres).
    let into_second = |i: f64, j: f64| -> (f64, f64) {
        let [x, y] = ground_of(i + 0.5, j + 0.5);
        let (dx, dy) = (x - sa[0], y - sa[3]);
        ((sa[5] * dx - sa[2] * dy) / det - 0.5, (-sa[4] * dx + sa[1] * dy) / det - 0.5)
    };
    let bicubic_asked = a.subpixel == Some(OrthoSubpixel::Bicubic);
    let margin = i64::from(bicubic_asked);
    let r = i64::from(a.search);
    // The block of the first's pixels a place reads (its patch, the search, the cubic's one more), and the second's
    // pixels it reaches when resampled: none when past them.
    let block = |ci: i64, cj: i64| -> Option<(i64, i64, u32, u32)> {
        let k = half + r + margin;
        let pts = [(ci - k, cj - k), (ci + k, cj - k), (ci - k, cj + k), (ci + k, cj + k)].map(|(i, j)| into_second(i as f64, j as f64));
        let lo = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
        let hi = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
        let (x0, x1) = (lo(pts.map(|p| p.0)).floor() as i64 - 1, hi(pts.map(|p| p.0)).floor() as i64 + 2);
        let (y0, y1) = (lo(pts.map(|p| p.1)).floor() as i64 - 1, hi(pts.map(|p| p.1)).floor() as i64 + 2);
        let fits = x0 >= 0 && y0 >= 0 && x1 < i64::from(second.raster.width) && y1 < i64::from(second.raster.height);
        fits.then(|| (x0, y0, (x1 - x0 + 1) as u32, (y1 - y0 + 1) as u32))
    };
    // The first's pixel a ground point falls in: a turned first's by its affine's inverse (docs/adr/4060).
    let pixel_of = |p: [f64; 2]| {
        if turned {
            let (dx, dy) = (p[0] - fa[0], p[1] - fa[3]);
            (((fa[5] * dx - fa[2] * dy) / det_a).floor() as i64, ((-fa[4] * dx + fa[1] * dy) / det_a).floor() as i64)
        } else {
            (((p[0] - fa[0]) / fa[1]).floor() as i64, ((p[1] - fa[3]) / fa[5]).floor() as i64)
        }
    };
    // A turned first's window inside its pixels (a north-up one's is, inside the overlap's margin).
    let (fw, fh) = (i64::from(first.raster.width), i64::from(first.raster.height));
    let window_in = |ci: i64, cj: i64| ci - half >= 0 && cj - half >= 0 && ci + half < fw && cj + half < fh;
    // The places: the overlap's grid (north to south, west to east), or every step along the seam line (docs/adr/2860);
    // resampled, only those whose block the second's pixels hold.
    let inside = |p: [f64; 2]| {
        p[0] >= ox0 && p[0] <= ox1 && p[1] >= oy0 && p[1] <= oy1 && (!resample || {
            let (ci, cj) = pixel_of(p);
            (!turned || window_in(ci, cj)) && block(ci, cj).is_some()
        })
    };
    // Each place, its distance along its line and (docs/adr/4460) the shared boundary's part it is on.
    let mut places: Vec<([f64; 2], Option<f64>, Option<u32>)> = Vec::new();
    let mut line = None;
    let mut shared = None;
    match (&a.line, &a.polygons) {
        // docs/adr/4460: every step along each part of the seamline polygons' shared boundary, from its start.
        (_, Some(pair)) => {
            let runs = crate::orthophoto_mosaic::shared_of(doc, pair)?;
            let (mut total, mut outside) = (0u32, 0u32);
            for (n, run) in runs.iter().enumerate() {
                let straight = vec![0.0; run.len().saturating_sub(1)];
                let length = crate::orthophoto_mosaic::run_length(run);
                let mut k = 0u64;
                loop {
                    let s = k as f64 * a.step;
                    if s > length {
                        break;
                    }
                    let p = along(run, &straight, s);
                    total += 1;
                    if inside(p) {
                        places.push((p, Some(s), Some(u32::try_from(n).unwrap_or(u32::MAX))));
                    } else {
                        outside += 1;
                    }
                    k += 1;
                }
            }
            if places.is_empty() {
                return Err(fail(
                    codes::INVALID_INPUT,
                    "Mozaik elemanlarının ortak sınırının hiçbir yeri görüntülerin örtüşmesinde değil (pencere ve arama payıyla); örtüşmede buluşan mozaik elemanlarını verin ya da adımı küçültün.".into(),
                    "seam.polygons",
                ));
            }
            let length = runs.iter().map(|r| crate::orthophoto_mosaic::run_length(r)).sum();
            let polygons = [pair[0].trim().to_owned(), pair[1].trim().to_owned()];
            shared = Some(OrthoSeamShared { polygons, parts: runs, length, places: total, outside });
        }
        (Some(uid), None) => {
            let (pts, bulges) = seam_line(doc, uid)?;
            let length = line_length(&pts, &bulges);
            if !(length > 0.0) {
                return Err(fail(codes::INVALID_INPUT, "Birleşme çizgisinin uzunluğu yok.".into(), "seam.line"));
            }
            let (mut total, mut outside) = (0u32, 0u32);
            let mut k = 0u64;
            loop {
                let s = k as f64 * a.step;
                if s > length {
                    break;
                }
                let p = along(&pts, &bulges, s);
                total += 1;
                if inside(p) {
                    places.push((p, Some(s), None));
                } else {
                    outside += 1;
                }
                k += 1;
            }
            if places.is_empty() {
                return Err(fail(
                    codes::INVALID_INPUT,
                    "Birleşme çizgisinin hiçbir yeri görüntülerin örtüşmesinde değil (pencere ve arama payıyla); çizgiyi örtüşmeden geçirin ya da adımı küçültün.".into(),
                    "seam.line",
                ));
            }
            line = Some(OrthoSeamLine { uid: uid.trim().to_owned(), length, places: total, outside });
        }
        (None, None) => {
            let mut y = oy1;
            while y >= oy0 {
                let mut x = ox0;
                while x <= ox1 {
                    if inside([x, y]) {
                        places.push(([x, y], None, None));
                    }
                    x += a.step;
                }
                y -= a.step;
            }
            if places.is_empty() {
                return Err(fail(
                    codes::INVALID_INPUT,
                    "Görüntüler örtüşmüyor ya da örtüşmeleri pencere ve arama için dar; pencereyi ya da aramayı küçültün.".into(),
                    "seam",
                ));
            }
        }
    }
    let mut ia = opened(doc, first, a.folder.as_deref(), "first")?;
    let mut ib = opened(doc, second, a.folder.as_deref(), "second")?;
    let read = |o: &mut Opened, x: i64, y: i64, w: u32, h: u32, path: &str| -> Result<Region, Stop> {
        o.region(x, y, w, h)
            .map_err(|e| fail(codes::IMPORT_UNREADABLE, format!("Ortofotonun pikselleri okunamadı: {e}"), path))?
            .ok_or_else(|| fail(codes::IMPORT_UNREADABLE, "Ortofotonun pikselleri okunamadı.".into(), path))
    };
    let w = a.window;
    // docs/adr/3260: resampling reads one pixel more round the search (the cubic's reach).
    let big = w + 2 * a.search + 2 * margin as u32;
    let mut patches = Vec::new();
    for &([x, y], station, part) in &places {
        {
            // The place: the centre of the first image's pixel it falls in.
            let (ci, cj) = pixel_of([x, y]);
            let at = ground_of(ci as f64 + 0.5, cj as f64 + 0.5);
            let pa = read(&mut ia, ci - half, cj - half, w, w, "first")?;
            let first_patch: Vec<f64> = (0..w).flat_map(|j| (0..w).map(move |i| (i, j))).map(|(i, j)| grey(&pa, i, j)).collect();
            // The second's block: its own pixels round the one whose centre is nearest the place, or (docs/adr/3660) its
            // pixels resampled at the first's pixel centres round the place. `base`: the block's middle pixel; `sample`:
            // the second's grey at a point of the block (whole numbers: the block's pixels), for the bicubic refinement.
            let (block_grey, base, sample): (Vec<f64>, (i64, i64), Box<dyn Fn(f64, f64) -> f64>) = if resample {
                let Some((rx, ry, rw, rh)) = block(ci, cj) else { continue };
                let pb = read(&mut ib, rx, ry, rw, rh, "second")?;
                let (gi, gj) = (ci - half - r - margin, cj - half - r - margin);
                let at_block = move |u: f64, v: f64| {
                    let (xb, yb) = into_second(gi as f64 + u, gj as f64 + v);
                    bicubic(&pb, xb - rx as f64, yb - ry as f64)
                };
                let values = (0..big).flat_map(|j| (0..big).map(move |i| (i, j))).map(|(i, j)| at_block(f64::from(i), f64::from(j))).collect();
                (values, (ci, cj), Box::new(at_block))
            } else {
                // The second image's pixel whose centre is nearest it.
                let (bi, bj) = (((at[0] - sa[0]) / sa[1] - 0.5).round() as i64, ((at[1] - sa[3]) / sa[5] - 0.5).round() as i64);
                let pb = read(&mut ib, bi - half - r - margin, bj - half - r - margin, big, big, "second")?;
                let values = (0..big).flat_map(|j| (0..big).map(move |i| (i, j))).map(|(i, j)| grey(&pb, i, j)).collect();
                (values, (bi, bj), Box::new(move |u: f64, v: f64| bicubic(&pb, u, v)))
            };
            let (bi, bj) = base;
            let side = (2 * r + 1) as usize;
            let mut scores: Vec<Option<f64>> = vec![None; side * side];
            for v in -r..=r {
                for u in -r..=r {
                    let (oi, oj) = ((u + r + margin) as u32, (v + r + margin) as u32);
                    let second_patch: Vec<f64> = (0..w)
                        .flat_map(|j| (0..w).map(move |i| (i, j)))
                        .map(|(i, j)| block_grey[((oj + j) * big + oi + i) as usize])
                        .collect();
                    scores[(v + r) as usize * side + (u + r) as usize] = ncc(&first_patch, &second_patch);
                }
            }
            let best = scores
                .iter()
                .enumerate()
                .filter_map(|(k, s)| s.map(|s| (k, s)))
                .fold(None, |m: Option<(usize, f64)>, (k, s)| if m.is_none_or(|m| s > m.1) { Some((k, s)) } else { m });
            let patch = match best {
                None => OrthoPatch { at, dx: None, dy: None, d: None, correlation: None, state: "flat".into(), over: false, station, part },
                Some((k, c)) => {
                    let (u, v) = ((k % side) as i64 - r, (k / side) as i64 - r);
                    if u.abs() == r || v.abs() == r {
                        OrthoPatch { at, dx: None, dy: None, d: None, correlation: Some(c), state: "edge".into(), over: false, station, part }
                    } else {
                        let s = |du: i64, dv: i64| scores[(v + dv + r) as usize * side + (u + du + r) as usize].unwrap_or(c);
                        let (fu, fv) = if bicubic_asked {
                            let (x, y) = ((u + r + margin) as f64, (v + r + margin) as f64);
                            refine_bicubic(&first_patch, &*sample, w, x, y)
                        } else {
                            (vertex(s(-1, 0), c, s(1, 0)), vertex(s(0, -1), c, s(0, 1)))
                        };
                        // The match on the block's grid: the second's own, or (resampled) the first's.
                        let (i, j) = ((bi + u) as f64 + fu + 0.5, (bj + v) as f64 + fv + 0.5);
                        let [xb, yb] = if resample { ground_of(i, j) } else { [sa[0] + i * sa[1], sa[3] + j * sa[5]] };
                        let (dx, dy) = (xb - at[0], yb - at[1]);
                        let d = dx.hypot(dy);
                        let ok = a.correlation.is_none_or(|m| c >= m);
                        OrthoPatch {
                            at,
                            dx: Some(dx),
                            dy: Some(dy),
                            d: Some(d),
                            correlation: Some(c),
                            state: if ok { "ok" } else { "low" }.into(),
                            over: ok && d > limit,
                            station,
                            part,
                        }
                    }
                }
            };
            patches.push(patch);
        }
    }
    let count = |state: &str| patches.iter().filter(|p| p.state == state).count() as u32;
    let points: Vec<OrthoPoint> = patches
        .iter()
        .enumerate()
        .filter(|(_, p)| p.state == "ok")
        .map(|(k, p)| OrthoPoint {
            name: format!("{}", k + 1),
            role: "check".into(),
            dx: p.dx.unwrap_or(0.0),
            dy: p.dy.unwrap_or(0.0),
            dz: None,
            d: p.d.unwrap_or(0.0),
        })
        .collect();
    let ok = count("ok");
    Ok(OrthoSeam {
        ok,
        low: count("low"),
        edge: count("edge"),
        flat: count("flat"),
        shift: (ok > 0).then(|| stats(&points.iter().collect::<Vec<_>>())),
        passed: (ok > 0).then(|| patches.iter().all(|p| !p.over)),
        patches,
        line,
        shared,
        resampled: resample,
        turned,
    })
}
