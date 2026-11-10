//! `cad.orthophoto.seamlines` v1 (plan/map-cad.md MAP-12, docs/adr/4700): Mozaik birleşme çizgisi. Two orthophotos'
//! seamline (birleşme çizgisi) and mosaic elements (mozaik elemanı) built in KentOS, the geometry in `kentos_geometry_core::ops::seamlines`:
//!
//! - **Footprints**: the rasters' extents (their four corners) or the areas given (`footprints`).
//! - **Voronoi**: the seam equally far from the two images' own parts (Pan et al. 2009), from its start to its end.
//! - **Radiometry**: the grid is the first image's pixels taken `cell` × `cell`, about its pixel (0, 0). A pixel is
//!   read when its centre lies in the overlap (its boundary too) and the second can be read there: the second's own
//!   pixel when their centres coincide (within 10⁻⁶ pixel), else Keys' cubic convolution (a = −1/2) of its 16 pixels
//!   round the point, all of them in the second. Its difference is the largest of the common bands' absolute
//!   differences. A cell is searched when every pixel of it is read, its centre lies within `corridor` of the Voronoi
//!   seam (when given) and none of its pixel centres lies in an `avoid` area; its difference is its pixels' largest.
//!   The path (`grid_path`) joins the searched cells nearest the Voronoi seam's start and end (by the squared
//!   distance; the first of equals by index); the seam is the start, the path's turning cells' centres, the end.
//! - **Polygons**: the first's ring from the end back to the start round its own side, then the seam; the second's
//!   from the start round its side to the end, then the seam back (`Footprints::rings`); both must be simple.
//!
//! Execute writes the two polygons (areas, each with `Görüntü`: its raster's id and `Yöntem`) on the layer in one undo
//! step “Mozaik elemanları”; the layer `mozaik-elemanlari` is made in the same step when none is given and the drawing
//! has none. The shared cases are `fixtures/commands/v1/cad.orthophoto.seamlines.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, Entity, EntityBase, OrthoSeamDifference, OrthoSeamEnd, OrthoSeamMethod, OrthoSeamPolygon,
    OrthophotoSeamlines, OrthophotoSeamlinesBuilt, PathEntity, RasterEntity, SEAMLINE_LAYER, SEAMLINE_LAYER_NAME, Vec2,
};
use kentos_domain::{Document, NewLayer, Uuid};
use kentos_formats::raster::source::Region;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region::inside_area;
use kentos_geometry_core::ops::seamlines::{self as geo, Footprints, Pt, SeamFault};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::orthophoto_seams::{keys, opened};

/// The undo step's name.
pub const STEP: &str = "Mozaik elemanları";
/// The most pixels of the first image the radiometric grid reads.
const MOST_PIXELS: u64 = 25_000_000;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn bad(message: &str, path: &str) -> Stop {
    fail(codes::INVALID_INPUT, message.to_owned(), path)
}

/// The input's own numbers and choices, before the drawing is read.
fn asked(input: &OrthophotoSeamlines) -> Result<(), Stop> {
    if input.first.trim() == input.second.trim() {
        return Err(bad("Aynı ortofoto iki kez verildi; birleşme çizgisi iki ayrı görüntü arasında kurulur.", "second"));
    }
    if let Some(s) = input.step
        && !(s.is_finite() && s > 0.0)
    {
        return Err(bad("Adım 0'dan büyük olmalı (metre).", "step"));
    }
    let voronoi = input.method == OrthoSeamMethod::Voronoi;
    if let Some(c) = input.cell {
        if voronoi {
            return Err(bad("Hücre yalnız radyometrik fark yönteminde verilir.", "cell"));
        }
        if !(1..=64).contains(&c) {
            return Err(bad("Hücre 1 ile 64 piksel arasında olmalı.", "cell"));
        }
    }
    if let Some(c) = input.corridor {
        if voronoi {
            return Err(bad("Arama bandı yalnız radyometrik fark yönteminde verilir.", "corridor"));
        }
        if !(c.is_finite() && c > 0.0) {
            return Err(bad("Arama bandı 0'dan büyük olmalı (metre); bütün örtüşme için boş bırakın.", "corridor"));
        }
    }
    if input.avoid.is_some() && voronoi {
        return Err(bad("Geçilmeyecek alanlar yalnız radyometrik fark yönteminde verilir.", "avoid"));
    }
    if let Some([p, q]) = &input.footprints
        && p.trim() == q.trim()
    {
        return Err(bad("Aynı ayak izi iki kez verildi; her görüntünün kendi ayak izini verin.", "footprints"));
    }
    Ok(())
}

/// A raster of the input, checked: the entity.
fn raster<'a>(doc: &'a Document, uid: &str, path: &str) -> Result<(Uuid, &'a RasterEntity), Stop> {
    let uid = uid.trim();
    if !checks::is_uid_text(uid) {
        return Err(fail(codes::INVALID_UID, format!("“{uid}” geçerli bir nesne kimliği değil."), path));
    }
    let Some((id, e)) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u).and_then(|s| doc.get(s)).map(|e| (u, e))) else {
        return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), path));
    };
    let Entity::Raster(r) = e else {
        return Err(fail(codes::NOT_A_RASTER, format!("“{uid}” bir raster değil; ortofotoyu seçin."), path));
    };
    let project = doc.settings().srid;
    if r.raster.srid != 0 && r.raster.srid != project {
        return Err(fail(
            codes::CRS_MISMATCH,
            format!(
                "Ortofotonun koordinat sistemi EPSG:{}, projeninki EPSG:{project}; birleşme çizgisi aynı sistemdeki görüntüler arasında kurulur. Önce projeye çevirin (Raster ekle › Projenin sistemine çevir ve ekle).",
                r.raster.srid
            ),
            path,
        ));
    }
    Ok((id, r))
}

/// A footprint given: its one ring (an area without holes or parts, or a closed polyline), straight edges.
fn footprint(doc: &Document, uid: &str, path: &str) -> Result<Vec<Pt>, Stop> {
    let e = Uuid::parse_str(uid.trim())
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| fail(codes::INVALID_INPUT, format!("“{uid}” kimlikli ayak izi çizimde yok."), path))?;
    let straight = |b: &Option<Vec<f64>>| b.as_ref().is_none_or(|b| b.iter().all(|x| *x == 0.0));
    let ring = |pts: &[Vec2]| -> Vec<Pt> { pts.iter().map(|v| [v.x, v.y]).collect() };
    match e {
        Entity::Polygon(p) if p.holes.as_ref().is_none_or(Vec::is_empty) && p.parts.as_ref().is_none_or(Vec::is_empty) => {
            if !straight(&p.bulges) {
                return Err(bad("Ayak izinin kenarları düz olmalı; yaylı ayak izi kullanılmaz.", path));
            }
            Ok(ring(&p.pts))
        }
        Entity::Polygon(_) => Err(bad("Ayak izi tek parçalı ve deliksiz bir alan olmalı.", path)),
        Entity::Polyline(p) if p.closed => {
            if !straight(&p.bulges) {
                return Err(bad("Ayak izinin kenarları düz olmalı; yaylı ayak izi kullanılmaz.", path));
            }
            Ok(ring(&p.pts))
        }
        _ => Err(bad("Ayak izi bir alan ya da kapalı bir çoklu çizgi olmalı.", path)),
    }
}

/// The footprints' fault in the user's words.
fn fault(f: SeamFault, given: bool) -> Stop {
    let which = |k: usize| if k == 0 { "Birinci" } else { "İkinci" };
    let what = if given { "ayak izi" } else { "görüntünün sınırı" };
    let path = if given { "footprints" } else { "second" };
    match f {
        SeamFault::Degenerate(k) => bad(&format!("{} {what} alan kaplamıyor (üçten az köşe ya da sıfır alan).", which(k)), path),
        SeamFault::NotConvex(k) => bad(
            &format!("{} {what} dışbükey değil; birleşme çizgisi dışbükey ayak izleri arasında kurulur. İçbükey ayak izini parçalara bölün.", which(k)),
            path,
        ),
        SeamFault::Apart => bad("İki görüntü örtüşmüyor (ya da yalnız değiyor); birleşme çizgisi ancak örtüşmede kurulur.", path),
        SeamFault::Inside(k) => bad(
            &format!(
                "{} görüntü öbürünün içinde kalıyor: öbürü onu bütünüyle örtüyor, aralarında birleşme çizgisi yok. Mozaikte üstte kalanı seçin.",
                which(k)
            ),
            path,
        ),
        SeamFault::Crossed => bad(
            "Örtüşmenin sınırı iki görüntü arasında ikiden çok kez el değiştiriyor (ayak izleri artı gibi kesişiyor); birleşme çizgisi tek çizgiyle kurulamaz.",
            path,
        ),
    }
}

/// The areas of the `avoid` layer (areas, every part and hole, and closed polylines).
fn avoided(doc: &Document, layer: &str) -> Result<Vec<Area>, Stop> {
    if doc.layers().get(layer).is_none() {
        return Err(fail(codes::LAYER_NOT_FOUND, format!("Geçilmeyecek alanların katmanı “{layer}” çizimde yok."), "avoid"));
    }
    let ring = |pts: &[Vec2], bulges: Option<&Vec<f64>>| Ring {
        pts: pts.iter().map(|p| kentos_geometry_core::Vec2::new(p.x, p.y)).collect(),
        bulges: bulges.cloned(),
    };
    let mut out = Vec::new();
    for e in doc.entities().filter(|e| e.base().layer_id == layer) {
        match e {
            Entity::Polygon(p) => {
                let holes = |h: &Option<Vec<kentos_commands::RingGeometry>>| {
                    h.iter().flatten().map(|r| ring(&r.pts, r.bulges.as_ref())).collect::<Vec<_>>()
                };
                out.push(Area { outer: ring(&p.pts, p.bulges.as_ref()), holes: holes(&p.holes) });
                for part in p.parts.iter().flatten() {
                    out.push(Area { outer: ring(&part.pts, part.bulges.as_ref()), holes: holes(&part.holes) });
                }
            }
            Entity::Polyline(p) if p.closed => out.push(Area { outer: ring(&p.pts, p.bulges.as_ref()), holes: Vec::new() }),
            _ => {}
        }
    }
    Ok(out)
}

/// A pixel centre's (or any point's) place on the ground from a raster's pixel coordinates.
fn ground(r: &RasterEntity, i: f64, j: f64) -> Pt {
    let (x, y) = r.raster.at(i, j);
    [x, y]
}

/// The radiometric seam and its differences (the module's rules).
fn radiometry(
    doc: &Document,
    input: &OrthophotoSeamlines,
    fp: &Footprints,
    voronoi: &[Pt],
    rasters: [&RasterEntity; 2],
) -> Result<(Vec<Pt>, OrthoSeamDifference), Stop> {
    let [r1, r2] = rasters;
    let c = i64::from(input.cell.unwrap_or(1));
    let avoid = match &input.avoid {
        Some(layer) => avoided(doc, layer.trim())?,
        None => Vec::new(),
    };
    // The overlap's box in the first's pixels, whole cells.
    let corners: Vec<(f64, f64)> = fp.overlap().iter().filter_map(|p| r1.raster.pixel_of(p[0], p[1])).collect();
    let (w1, h1) = (i64::from(r1.raster.width), i64::from(r1.raster.height));
    let lo = |f: fn(&(f64, f64)) -> f64| corners.iter().map(f).fold(f64::INFINITY, f64::min);
    let hi = |f: fn(&(f64, f64)) -> f64| corners.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
    let (i0, i1) = ((lo(|p| p.0).floor() as i64).max(0), (hi(|p| p.0).ceil() as i64).min(w1));
    let (j0, j1) = ((lo(|p| p.1).floor() as i64).max(0), (hi(|p| p.1).ceil() as i64).min(h1));
    let (ci0, cj0) = (i0.div_euclid(c), j0.div_euclid(c));
    let (ci1, cj1) = ((i1 - 1).div_euclid(c), (j1 - 1).div_euclid(c));
    if i1 <= i0 || j1 <= j0 {
        return Err(bad("Örtüşmede birinci görüntünün pikseli yok.", "cell"));
    }
    let (gw, gh) = ((ci1 - ci0 + 1) as usize, (cj1 - cj0 + 1) as usize);
    let (px0, py0) = (ci0 * c, cj0 * c);
    let (pw, ph) = ((gw as i64 * c) as u32, (gh as i64 * c) as u32);
    if u64::from(pw) * u64::from(ph) > MOST_PIXELS {
        return Err(bad(
            &format!(
                "Örtüşme birinci görüntünün {} pikselini kaplıyor; radyometrik fark yöntemi en çok {} piksel okur. Birleşme çizgisini eşit uzaklıkla (Voronoi) kurun ya da daha küçük ayak izleri verin.",
                u64::from(pw) * u64::from(ph),
                MOST_PIXELS
            ),
            "cell",
        ));
    }
    let mut o1 = opened(doc, r1, input.folder.as_deref(), "first")?;
    let mut o2 = opened(doc, r2, input.folder.as_deref(), "second")?;
    let read = |o: &mut crate::raster_files::Opened, x: i64, y: i64, w: u32, h: u32, path: &str| -> Result<Region, Stop> {
        o.region(x, y, w, h)
            .map_err(|e| fail(codes::IMPORT_UNREADABLE, format!("Ortofotonun pikselleri okunamadı: {e}"), path))?
            .ok_or_else(|| fail(codes::IMPORT_UNREADABLE, "Ortofotonun pikselleri okunamadı.".into(), path))
    };
    let first = read(&mut o1, px0, py0, pw, ph, "first")?;
    // The second's pixels under the box, two more round it for the cubic.
    let box_in_second: Vec<(f64, f64)> = [(px0, py0), (px0 + i64::from(pw), py0), (px0, py0 + i64::from(ph)), (px0 + i64::from(pw), py0 + i64::from(ph))]
        .iter()
        .filter_map(|&(i, j)| {
            let p = ground(r1, i as f64, j as f64);
            r2.raster.pixel_of(p[0], p[1])
        })
        .collect();
    let (w2, h2) = (i64::from(r2.raster.width), i64::from(r2.raster.height));
    let sx0 = (box_in_second.iter().map(|p| p.0).fold(f64::INFINITY, f64::min).floor() as i64 - 2).max(0);
    let sy0 = (box_in_second.iter().map(|p| p.1).fold(f64::INFINITY, f64::min).floor() as i64 - 2).max(0);
    let sx1 = (box_in_second.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max).ceil() as i64 + 2).min(w2);
    let sy1 = (box_in_second.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max).ceil() as i64 + 2).min(h2);
    if sx1 <= sx0 || sy1 <= sy0 {
        return Err(bad("Örtüşmede ikinci görüntünün pikseli yok.", "cell"));
    }
    let second = read(&mut o2, sx0, sy0, (sx1 - sx0) as u32, (sy1 - sy0) as u32, "second")?;
    let bands = r1.raster.bands.min(r2.raster.bands).max(1);
    // The second's bands at a point of its pixels (pixel centres at whole + 0.5), none when not readable there.
    let sample = |u: f64, v: f64| -> Option<Vec<f64>> {
        let (cu, cv) = (u - 0.5, v - 0.5);
        let (ru, rv) = (cu.round(), cv.round());
        if (cu - ru).abs() <= 1e-6 && (cv - rv).abs() <= 1e-6 {
            let (i, j) = (ru as i64, rv as i64);
            if i < 0 || j < 0 || i >= w2 || j >= h2 {
                return None;
            }
            let (li, lj) = ((i - sx0) as u32, (j - sy0) as u32);
            return Some((0..bands).map(|b| second.at(li, lj, b)).collect());
        }
        let (fu, fv) = (cu.floor(), cv.floor());
        let (iu, iv) = (fu as i64, fv as i64);
        if iu - 1 < 0 || iv - 1 < 0 || iu + 2 >= w2 || iv + 2 >= h2 {
            return None;
        }
        let (tx, ty) = (cu - fu, cv - fv);
        Some(
            (0..bands)
                .map(|b| {
                    let mut sum = 0.0;
                    for n in -1..=2_i64 {
                        let mut row = 0.0;
                        for m in -1..=2_i64 {
                            row += second.at((iu + m - sx0) as u32, (iv + n - sy0) as u32, b) * keys(tx - m as f64);
                        }
                        sum += row * keys(ty - n as f64);
                    }
                    sum
                })
                .collect(),
        )
    };
    let in_avoid = |p: Pt| avoid.iter().any(|a| inside_area(a, kentos_geometry_core::Vec2::new(p[0], p[1])));
    let mut diff = vec![f64::NAN; gw * gh];
    let mut searched = 0_u32;
    let mut sum = 0.0;
    for gj in 0..gh {
        for gi in 0..gw {
            let mut worst: Option<f64> = Some(0.0);
            'pixels: for dj in 0..c {
                for di in 0..c {
                    let (i, j) = (px0 + gi as i64 * c + di, py0 + gj as i64 * c + dj);
                    if i >= w1 || j >= h1 {
                        worst = None;
                        break 'pixels;
                    }
                    let centre = ground(r1, i as f64 + 0.5, j as f64 + 0.5);
                    if !fp.in_overlap(centre) || in_avoid(centre) {
                        worst = None;
                        break 'pixels;
                    }
                    let Some((u, v)) = r2.raster.pixel_of(centre[0], centre[1]) else {
                        worst = None;
                        break 'pixels;
                    };
                    let Some(b2) = sample(u, v) else {
                        worst = None;
                        break 'pixels;
                    };
                    let (li, lj) = ((i - px0) as u32, (j - py0) as u32);
                    let d = (0..bands).map(|b| (first.at(li, lj, b) - b2[b as usize]).abs()).fold(0.0, f64::max);
                    worst = worst.map(|w| w.max(d));
                }
            }
            let Some(d) = worst else { continue };
            let centre = ground(r1, ((ci0 + gi as i64) * c) as f64 + c as f64 / 2.0, ((cj0 + gj as i64) * c) as f64 + c as f64 / 2.0);
            if let Some(corridor) = input.corridor
                && geo::to_polyline(centre, voronoi) > corridor
            {
                continue;
            }
            diff[gj * gw + gi] = d;
            searched += 1;
            sum += d;
        }
    }
    if searched == 0 {
        return Err(bad(
            "Örtüşmede aranacak hücre kalmadı (geçilmeyecek alanlar, arama bandı ya da görüntülerin kenarı); arama bandını genişletin ya da hücreyi küçültün.",
            "cell",
        ));
    }
    let centre_of = |k: usize| {
        let (gi, gj) = ((k % gw) as i64, (k / gw) as i64);
        ground(r1, ((ci0 + gi) * c) as f64 + c as f64 / 2.0, ((cj0 + gj) * c) as f64 + c as f64 / 2.0)
    };
    let nearest = |p: Pt| -> usize {
        let mut best = (f64::INFINITY, 0);
        for (k, d) in diff.iter().enumerate() {
            if d.is_nan() {
                continue;
            }
            let q = centre_of(k);
            // Squared: two languages' hypot may differ in the last bit and turn a tie.
            let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
            let dist = dx * dx + dy * dy;
            if dist < best.0 {
                best = (dist, k);
            }
        }
        best.1
    };
    let [(start, _), (end, _)] = fp.ends();
    let (from, to) = (nearest(start), nearest(end));
    let Some(path) = geo::grid_path(gw, gh, &diff, from, to) else {
        return Err(bad(
            "Birleşme çizgisinin iki ucu arasında aranacak hücrelerden geçen yol yok (geçilmeyecek alanlar ya da arama bandı yolu kesiyor); arama bandını genişletin ya da geçilmeyecek alanları gözden geçirin.",
            "avoid",
        ));
    };
    let mean = path.cells.iter().map(|&k| diff[k]).sum::<f64>() / path.cells.len() as f64;
    let mut seam = vec![start];
    seam.extend(geo::turns(&path.cells, gw).into_iter().map(centre_of));
    seam.push(end);
    let det = r1.raster.determinant().abs().sqrt();
    Ok((
        seam,
        OrthoSeamDifference {
            bottleneck: path.bottleneck,
            mean,
            searched_mean: sum / f64::from(searched),
            cells: path.cells.len() as u32,
            searched,
            size: det * c as f64,
        },
    ))
}

/// What the command builds, writing nothing: the output and the two polygons as they would be written.
fn run(doc: &Document, input: &OrthophotoSeamlines) -> Result<(OrthophotoSeamlinesBuilt, [Entity; 2]), Stop> {
    asked(input)?;
    let (id1, r1) = raster(doc, &input.first, "first")?;
    let (id2, r2) = raster(doc, &input.second, "second")?;
    let layer = input.layer.as_deref().map(str::trim).unwrap_or(SEAMLINE_LAYER).to_owned();
    let opened = match doc.layers().get(&layer) {
        Some(node) => {
            if doc.layers().is_locked(&layer) {
                return Err(fail(
                    codes::LAYER_LOCKED,
                    format!("“{}” katmanı kilitli; mozaik elemanları bu katmana yazılır. Kilidi Katmanlar panelinden açın.", node.name),
                    "layer",
                ));
            }
            false
        }
        None if input.layer.is_some() => {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{layer}” katmanı çizimde yok."), "layer"));
        }
        None => true,
    };
    let given = input.footprints.is_some();
    let (f1, f2) = match &input.footprints {
        Some([a, b]) => (footprint(doc, a, "footprints[0]")?, footprint(doc, b, "footprints[1]")?),
        None => {
            let ring = |r: &RasterEntity| -> Vec<Pt> { r.raster.corners().iter().map(|&(x, y)| [x, y]).collect() };
            (ring(r1), ring(r2))
        }
    };
    let fp = Footprints::new(&f1, &f2).map_err(|f| fault(f, given))?;
    let step = input.step.unwrap_or(1.0);
    let voronoi = fp.voronoi(step);
    let (seam, difference) = match input.method {
        OrthoSeamMethod::Voronoi => (voronoi, None),
        OrthoSeamMethod::Radiometry => {
            let (seam, d) = radiometry(doc, input, &fp, &voronoi, [r1, r2])?;
            (seam, Some(d))
        }
    };
    let rings = fp.rings(&seam);
    if !rings.iter().all(|r| geo::simple(r)) {
        return Err(bad(
            "Kurulan mozaik elemanı kendini kesiyor; hücreyi küçültün ya da arama bandı verin.",
            "method",
        ));
    }
    let length: f64 = seam.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
    let overlap_area = fp.overlap_area();
    let method_word = match input.method {
        OrthoSeamMethod::Voronoi => "eşit uzaklık (Voronoi)",
        OrthoSeamMethod::Radiometry => "radyometrik fark",
    };
    let ids = [id1.to_string(), id2.to_string()];
    let polygons: [OrthoSeamPolygon; 2] = [0, 1].map(|k| {
        let area = geo::ring_area(&rings[k]);
        OrthoSeamPolygon {
            image: ids[k].clone(),
            ring: rings[k].clone(),
            area,
            kept: area - (fp.area(k) - overlap_area),
            uid: None,
        }
    });
    let entity = |k: usize| -> Entity {
        let mut attrs = BTreeMap::new();
        attrs.insert("Görüntü".to_owned(), ids[k].clone());
        attrs.insert("Yöntem".to_owned(), method_word.to_owned());
        Entity::Polygon(PathEntity {
            space_separator: None,
            base: EntityBase {
                look: Default::default(),
                id: 0,
                layer_id: layer.clone(),
                color: None,
                attrs,
                label: None,
                symbol: None,
                line_weight: None,
                label_pins: Vec::new(),
                attachments: Vec::new(),
            },
            pts: rings[k].iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
            bulges: None,
            holes: None,
            zs: None,
            ms: None,
            parts: None,
            closed: false,
            widths: None,
        })
    };
    let [(s, sc), (e, ec)] = fp.ends();
    let end = |at: Pt, crossing: bool| OrthoSeamEnd { at, kind: if crossing { "crossing" } else { "shared" }.into() };
    let mut notes = vec![format!(
        "Mozaik elemanları ({}): örtüşme {:.2} m², birleşme çizgisi {:.2} m; birinci görüntü örtüşmenin {:.2} m²'sini, ikinci {:.2} m²'sini kullanır.",
        method_word, overlap_area, length, polygons[0].kept, polygons[1].kept
    )];
    notes.push(match (sc, ec) {
        (true, true) => "Birleşme çizgisinin uçları iki ayak izinin sınırlarının kesiştiği yerlerde.".into(),
        (false, false) => "Birleşme çizgisinin uçları iki ayak izinin ortak sınırında, iki görüntünün kendi bölgelerinden eşit uzakta.".into(),
        _ => "Birleşme çizgisinin bir ucu sınırların kesiştiği yerde, öbürü ortak sınırda iki görüntünün kendi bölgelerinden eşit uzakta.".into(),
    });
    if let Some(d) = &difference {
        notes.push(format!(
            "Radyometrik fark: birleşme çizgisi boyunca en büyük {:.1}, ortalama {:.1}; aranan {} hücrenin ortalaması {:.1}; hücre {:.2} m.",
            d.bottleneck, d.mean, d.searched, d.searched_mean, d.size
        ));
        notes.push("Birleşme çizgisinden önce ortak alanda radyometrik düzeltme yapılmalıdır (BÖHHBÜY m. 67(6)); fark ondan sonra anlamlıdır.".into());
    }
    if input.avoid.is_some() {
        notes.push("Geçilmeyecek alanların piksellerini içeren hücre aranmadı: birleşme çizgisi onları kesmez.".into());
    }
    notes.push("Mozaik elemanı görüntünün mozaikte kullanılan bölümüdür (TUCBS Ortogörüntü); BÖHHBÜY birleşme çizgisinin yerine kural koymaz, kayıklık m. 67(6) ile ayrıca denetlenir (Ortofoto karşılaştırması › Kayıklık ölçümü).".into());
    let out = OrthophotoSeamlinesBuilt {
        method: input.method,
        overlap: fp.overlap(),
        overlap_area,
        seam,
        length,
        ends: [end(s, sc), end(e, ec)],
        polygons,
        difference,
        layer: layer.clone(),
        opened,
        notes,
        revision: doc.revision().to_string(),
    };
    Ok((out, [entity(0), entity(1)]))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &OrthophotoSeamlines) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &OrthophotoSeamlines) -> CommandResult<OrthophotoSeamlinesBuilt> {
    match run(cx.doc, input) {
        Ok((out, _)) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: OrthophotoSeamlines) -> CommandResult<OrthophotoSeamlinesBuilt> {
    if let Err(stop) = checks::revision(cx.doc, input.expected_revision.as_deref()) {
        return stop.into();
    }
    let (mut out, entities) = match run(cx.doc, &input) {
        Ok(done) => done,
        Err(stop) => return stop.into(),
    };
    let group = cx.doc.begin_group(STEP);
    if out.opened {
        let layer = NewLayer { id: Some(SEAMLINE_LAYER.to_owned()), ..NewLayer::layer(SEAMLINE_LAYER_NAME) };
        if let Err(refusal) = cx.doc.add_layer(layer, None, false) {
            cx.doc.cancel_group(group);
            return Stop::failed(error(codes::LAYER_NOT_FOUND, refusal.to_string(), Some("layer".into()))).into();
        }
    }
    let slots = match cx.doc.add_many(entities.to_vec(), STEP) {
        Ok(slots) => slots,
        Err(_) => {
            cx.doc.cancel_group(group);
            return Stop::failed(error(codes::INVALID_INPUT, "Çizimde yeni nesneye yer kalmadı.".into(), None)).into();
        }
    };
    cx.doc.end_group(group);
    for (k, slot) in slots.iter().enumerate() {
        out.polygons[k].uid = cx.doc.uid(*slot).map(|u| u.to_string());
    }
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: OrthophotoSeamlines) -> CommandResult<OrthophotoSeamlinesBuilt> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ORTHOPHOTO_SEAMLINES,
        kentos_commands::CAD_ORTHOPHOTO_SEAMLINES_VERSION,
        input,
        write,
    )
}
