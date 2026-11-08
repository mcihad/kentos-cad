//! The shift between two orthophotos measured in the images themselves where they overlap (plan/map-cad.md MAP-12,
//! docs/adr/2460), `cad.orthophoto.compare`'s `seam`: no point read by hand.
//!
//! - **Places:** every `step` metres across the overlap, inside it by half the window and the search (north to south,
//!   west to east). A place is the centre of the first image's pixel it falls in.
//! - **Match:** the first image's `window` × `window` patch (its grey: the mean of the first three bands) sought in the
//!   second within `search` pixels each way by zero-mean normalized cross-correlation, from the second's pixel whose
//!   centre is nearest the place; the peak (the first of equals, rows then columns) refined to a fraction of a pixel by
//!   a parabola through it and its two neighbours, each way apart.
//! - **Shift:** the second's matched place − the place (metres).
//! - **State:** `flat` when either patch has no texture (no correlation); `edge` when the peak is on the search's edge
//!   (the true one may lie beyond; no shift); `low` below the correlation asked (its shift said, not counted); else `ok`.
//!
//! The two images must have pixels of one size and no turn (north up); resampling one to the other is left for later.
//! Nothing is written.

use std::path::Path;

use kentos_commands::{OrthoPatch, OrthoPoint, OrthoSeam, OrthoSeamAsk, RasterEntity};
use kentos_domain::Document;
use kentos_formats::raster::source::Region;

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
        return bad("Dikiş ölçümünün adımı 0'dan büyük bir uzunluk olmalı (m).", "seam.step");
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
    Ok(())
}

/// A raster's pixels opened: its library bytes or its file, found from `folder` when relative.
fn opened(doc: &Document, r: &RasterEntity, folder: Option<&str>, path: &str) -> Result<Opened, Stop> {
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

/// The parabola's vertex through (−1, l), (0, c), (1, r): the peak's fraction of a pixel.
fn vertex(l: f64, c: f64, r: f64) -> f64 {
    let den = l - 2.0 * c + r;
    if den < 0.0 { (l - r) / (2.0 * den) } else { 0.0 }
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
    if !(unturned(fa) && unturned(sa) && same(fa[1], sa[1]) && same(fa[5], sa[5])) {
        return Err(fail(
            codes::INVALID_INPUT,
            "Dikiş ölçümü pikselleri aynı boyda ve kuzeye bakan (dönüksüz) iki görüntüde yapılır; birini öbürünün ızgarasına yeniden örnekleyin.".into(),
            "seam",
        ));
    }
    let (px, py) = (fa[1], -fa[5]);
    let extent = |t: [f64; 6], w: u32, h: u32| (t[0], t[0] + f64::from(w) * t[1], t[3] + f64::from(h) * t[5], t[3]);
    let (ax0, ax1, ay0, ay1) = extent(fa, first.raster.width, first.raster.height);
    let (bx0, bx1, by0, by1) = extent(sa, second.raster.width, second.raster.height);
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
    let mut ia = opened(doc, first, a.folder.as_deref(), "first")?;
    let mut ib = opened(doc, second, a.folder.as_deref(), "second")?;
    let read = |o: &mut Opened, x: i64, y: i64, w: u32, h: u32, path: &str| -> Result<Region, Stop> {
        o.region(x, y, w, h)
            .map_err(|e| fail(codes::IMPORT_UNREADABLE, format!("Ortofotonun pikselleri okunamadı: {e}"), path))?
            .ok_or_else(|| fail(codes::IMPORT_UNREADABLE, "Ortofotonun pikselleri okunamadı.".into(), path))
    };
    let w = a.window;
    let r = i64::from(a.search);
    let big = w + 2 * a.search;
    let mut patches = Vec::new();
    let mut y = oy1;
    while y >= oy0 {
        let mut x = ox0;
        while x <= ox1 {
            // The place: the centre of the first image's pixel it falls in.
            let (ci, cj) = (((x - fa[0]) / fa[1]).floor() as i64, ((y - fa[3]) / fa[5]).floor() as i64);
            let at = [fa[0] + (ci as f64 + 0.5) * fa[1], fa[3] + (cj as f64 + 0.5) * fa[5]];
            // The second image's pixel whose centre is nearest it.
            let (bi, bj) = (((at[0] - sa[0]) / sa[1] - 0.5).round() as i64, ((at[1] - sa[3]) / sa[5] - 0.5).round() as i64);
            let pa = read(&mut ia, ci - half, cj - half, w, w, "first")?;
            let pb = read(&mut ib, bi - half - r, bj - half - r, big, big, "second")?;
            let first_patch: Vec<f64> = (0..w).flat_map(|j| (0..w).map(move |i| (i, j))).map(|(i, j)| grey(&pa, i, j)).collect();
            let side = (2 * r + 1) as usize;
            let mut scores: Vec<Option<f64>> = vec![None; side * side];
            for v in -r..=r {
                for u in -r..=r {
                    let (oi, oj) = ((u + r) as u32, (v + r) as u32);
                    let second_patch: Vec<f64> =
                        (0..w).flat_map(|j| (0..w).map(move |i| (i, j))).map(|(i, j)| grey(&pb, oi + i, oj + j)).collect();
                    scores[(v + r) as usize * side + (u + r) as usize] = ncc(&first_patch, &second_patch);
                }
            }
            let best = scores
                .iter()
                .enumerate()
                .filter_map(|(k, s)| s.map(|s| (k, s)))
                .fold(None, |m: Option<(usize, f64)>, (k, s)| if m.is_none_or(|m| s > m.1) { Some((k, s)) } else { m });
            let patch = match best {
                None => OrthoPatch { at, dx: None, dy: None, d: None, correlation: None, state: "flat".into(), over: false },
                Some((k, c)) => {
                    let (u, v) = ((k % side) as i64 - r, (k / side) as i64 - r);
                    if u.abs() == r || v.abs() == r {
                        OrthoPatch { at, dx: None, dy: None, d: None, correlation: Some(c), state: "edge".into(), over: false }
                    } else {
                        let s = |du: i64, dv: i64| scores[(v + dv + r) as usize * side + (u + du + r) as usize].unwrap_or(c);
                        let (fu, fv) = (vertex(s(-1, 0), c, s(1, 0)), vertex(s(0, -1), c, s(0, 1)));
                        let xb = sa[0] + ((bi + u) as f64 + fu + 0.5) * sa[1];
                        let yb = sa[3] + ((bj + v) as f64 + fv + 0.5) * sa[5];
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
                        }
                    }
                }
            };
            patches.push(patch);
            x += a.step;
        }
        y -= a.step;
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
    })
}
