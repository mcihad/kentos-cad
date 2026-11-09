//! `cad.asbuilt.check` v1 (plan/map-cad.md MAP-19, docs/adr/1700): Uygulama kontrolü, read only. The measured points
//! of a layer set against the design's points, surface and lines, as Trimble Business Center's Point and String
//! Conformance reports set them: by name, else (a radius given) by the one design point within it, a proposal; two
//! within it ambiguous. Every limit is the user's; a point's own σ beyond its limit is a flag apart from its
//! differences. The design's digest says whether the design changed since the field record was made. Nothing is
//! written. The shared cases are `fixtures/commands/v1/cad.asbuilt.check.json`.
//!
//! With the package's design (docs/adr/2820) the package's surface too, when it carries one (docs/adr/3220): its
//! `yuzey.xml` read back to its SHA-256 is the surface the record is set against; the drawing's surface, given, then
//! only says whether it changed since (another surface, or its sources' digest not the package's).
//!
//! The digest is SHA-256 of the design's points (`p<TAB>name<TAB>x<TAB>y<TAB>z` with six decimals, `-` for no height)
//! and then its lines (`l<TAB>x,y;x,y…`), each on its own line, in the drawing's order.

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{AsbuiltSurfaceBy, 
    ASBUILT_FLAGS, ASBUILT_LIMIT, AsbuiltCheck, AsbuiltChecked, AsbuiltControl, AsbuiltKind, AsbuiltMissing,
    AsbuiltPackage, AsbuiltPoint, CommandResult, Entity, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_road::surface::Surface;
use kentos_road::volume::Ground;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::road_terrains as terrains;

/// The names' attribute when none is given.
pub const NAME_FIELD: &str = "Ad";

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The surface the record is set against: the drawing's, or the one the package carries (docs/adr/3220); with its
/// triangles laid out for the nearest place (docs/adr/4020).
enum Heights<'a> {
    Drawing(&'a Surface, Ground),
    Package(&'a Ground),
}

impl Heights<'_> {
    /// The surface's height at a place; none off it (or in a void).
    fn at(&self, p: Vec2) -> Option<f64> {
        match self {
            Heights::Drawing(s, _) => s.height(p.x, p.y).z,
            Heights::Package(g) => g.height([p.x, p.y]),
        }
    }

    /// The surface's nearest place to `p` and the distance, within `reach` (docs/adr/4020).
    fn nearest(&self, p: [f64; 3], reach: f64) -> Option<([f64; 3], f64)> {
        match self {
            Heights::Drawing(_, g) => g.nearest(p, reach),
            Heights::Package(g) => g.nearest(p, reach),
        }
    }
}

/// A surface's kept triangles as the core's ground takes them.
fn ground_of(s: &Surface) -> Ground {
    let tris: Vec<[[f64; 3]; 3]> = s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect();
    Ground::new(&tris)
}

fn plain(text: &str) -> Option<f64> {
    let t = text.trim();
    let body = t.strip_prefix('-').unwrap_or(t);
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let ok = match body.split_once(['.', ',']) {
        Some((w, f)) => digits(w) && digits(f),
        None => digits(body),
    };
    ok.then(|| t.replace(',', ".").parse::<f64>().ok()).flatten().filter(|v| v.is_finite())
}

pub(crate) struct Point {
    /// Its object's persistent id (a package's design point: the id the package wrote, docs/adr/2820).
    pub(crate) uid: String,
    pub(crate) name: String,
    pub(crate) p: Vec2,
    pub(crate) z: Option<f64>,
    pub(crate) sigma: Option<f64>,
}

/// The points of a layer (a group: its layers), in the drawing's order.
pub(crate) fn points(doc: &Document, layer: &str, field: &str, quality: Option<&str>) -> Vec<Point> {
    let ids: Vec<String> = doc.layers().leaves_of(layer).into_iter().map(|n| n.id.clone()).collect();
    doc.entities()
        .filter(|e| ids.contains(&e.base().layer_id))
        .filter_map(|e| match e {
            Entity::Point(p) => Some(Point {
                uid: doc.uid(Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default(),
                name: e.base().attrs.get(field).map(|n| n.trim().to_owned()).unwrap_or_default(),
                p: p.p,
                z: p.z,
                sigma: quality.and_then(|q| e.base().attrs.get(q)).and_then(|v| plain(v)),
            }),
            _ => None,
        })
        .collect()
}

/// A design line's vertices and edges' bulges.
pub(crate) struct Line {
    /// Its object's persistent id, as [`Point::uid`].
    pub(crate) uid: String,
    pub(crate) pts: Vec<Vec2>,
    pub(crate) bulges: Vec<f64>,
}

pub(crate) fn lines(doc: &Document, layer: &str) -> Vec<Line> {
    let ids: Vec<String> = doc.layers().leaves_of(layer).into_iter().map(|n| n.id.clone()).collect();
    let uid_of = |e: &Entity| doc.uid(Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default();
    doc.entities()
        .filter(|e| ids.contains(&e.base().layer_id))
        .filter_map(|e| match e {
            Entity::Line(l) => Some(Line { uid: uid_of(e), pts: vec![l.a, l.b], bulges: vec![0.0] }),
            Entity::Polyline(p) if p.pts.len() >= 2 => {
                let mut pts = p.pts.clone();
                if p.closed {
                    pts.push(p.pts[0]);
                }
                let n = pts.len() - 1;
                let bulges = (0..n).map(|i| p.bulges.as_ref().and_then(|b| b.get(i).copied()).unwrap_or(0.0)).collect();
                Some(Line { uid: uid_of(e), pts, bulges })
            }
            _ => None,
        })
        .collect()
}

/// The signed distance from `p` to the edge `a`→`b` (left of its direction positive): a straight edge's to its
/// nearest point; a bulged edge's to its arc where `p`'s ray from the centre crosses it, else to its nearer end.
fn signed(p: Vec2, a: Vec2, b: Vec2, bulge: f64) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let side = |d: f64, cross: f64| if cross < 0.0 { -d } else { d };
    let chord = dx.hypot(dy);
    if bulge != 0.0 && chord > 0.0 {
        let k = chord * (1.0 - bulge * bulge) / (4.0 * bulge);
        let (ux, uy) = (dx / chord, dy / chord);
        let c = Vec2 { x: (a.x + b.x) / 2.0 - uy * k, y: (a.y + b.y) / 2.0 + ux * k };
        let theta = 4.0 * bulge.atan();
        let r = chord / (2.0 * (theta / 2.0).sin().abs());
        let start = if bulge > 0.0 { a } else { b };
        let a0 = (start.y - c.y).atan2(start.x - c.x);
        let rel = ((p.y - c.y).atan2(p.x - c.x) - a0).rem_euclid(std::f64::consts::TAU);
        if rel <= theta.abs() {
            let d = (p.x - c.x).hypot(p.y - c.y);
            // A counter-clockwise arc has its centre on the left: inside the circle is left.
            return if bulge > 0.0 { r - d } else { d - r };
        }
        let (da, db) = ((p.x - a.x).hypot(p.y - a.y), (p.x - b.x).hypot(p.y - b.y));
        return side(da.min(db), dx * (p.y - a.y) - dy * (p.x - a.x));
    }
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0) };
    let d = (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy);
    side(d, dx * (p.y - a.y) - dy * (p.x - a.x))
}

pub(crate) fn digest(design: &[Point], lines: &[Line]) -> String {
    let mut text = String::new();
    for p in design {
        let z = p.z.map_or_else(|| "-".to_owned(), |z| format!("{z:.6}"));
        text.push_str(&format!("p\t{}\t{:.6}\t{:.6}\t{z}\n", p.name, p.p.x, p.p.y));
    }
    for l in lines {
        let pts: Vec<String> = l.pts.iter().map(|v| format!("{:.6},{:.6}", v.x, v.y)).collect();
        text.push_str(&format!("l\t{}\n", pts.join(";")));
    }
    sha256_hex(text.as_bytes())
}

fn asked(input: &AsbuiltCheck) -> Result<(), Stop> {
    let bad = |m: &str, path: &str| Err(refuse(codes::INVALID_ASBUILT_CHECK, m.to_owned(), path));
    // A monitoring job (docs/adr/3620): the same marks at two epochs, by name; the design's other parts are not its.
    if input.kind == Some(AsbuiltKind::Monitoring) {
        if input.design.is_none() {
            return bad("İzleme işinde önceki dönemin noktalarının katmanını (design) verin; noktalar adla karşılaştırılır.", "design");
        }
        let foreign = [
            (input.surface.is_some(), "surface"),
            (input.lines.is_some(), "lines"),
            (input.search.is_some(), "search"),
            (input.package.is_some() || input.package_design, "package"),
        ];
        if let Some((_, path)) = foreign.into_iter().find(|(given, _)| *given) {
            return bad(
                "İzleme işi önceki dönemin noktalarıyla adla karşılaştırılır; yüzey, çizgi, yerle eşleme ve saha paketi tasarım denetimlerinindir.",
                path,
            );
        }
    } else if !input.control.is_empty() {
        return bad("Kontrol noktaları izleme işinindir (kind: monitoring).", "control");
    }
    if input.control.iter().any(|c| c.trim().is_empty()) {
        return bad("Kontrol noktasının adı boş; adını yazın.", "control");
    }
    if input.design.is_none() && input.surface.is_none() && input.lines.is_none() && !input.package_design {
        return bad("Karşılaştırılacak tasarım yok: tasarım noktalarının katmanını, yüzeyi ya da tasarım çizgilerinin katmanını verin.", "design");
    }
    for (v, path, words) in [
        (input.search, "search", "Arama yarıçapı"),
        (input.horizontal, "horizontal", "Yatay tolerans"),
        (input.vertical, "vertical", "Düşey tolerans"),
        (input.quality, "quality", "Kalite sınırı"),
    ] {
        if let Some(v) = v
            && !(v.is_finite() && v > 0.0)
        {
            return Err(refuse(
                codes::INVALID_ASBUILT_CHECK,
                format!("{words} 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın."),
                path,
            ));
        }
    }
    if input.field.as_deref().is_some_and(|f| f.trim().is_empty()) {
        return bad("Ad özniteliği boş; özniteliğin adını yazın ya da boş bırakın (Ad).", "field");
    }
    if input.quality_field.as_deref().is_some_and(|f| f.trim().is_empty()) {
        return bad("Kalite özniteliği boş; σ'nın yazıldığı özniteliğin adını yazın.", "qualityField");
    }
    if input.quality.is_some() && input.quality_field.is_none() {
        return bad("Kalite sınırı verildi ama σ'nın özniteliği yok; qualityField'ı da verin.", "qualityField");
    }
    if input.package_design && input.package.is_none() {
        return bad("Paketin tasarımıyla denetlemek için kaydın saha paketini (package) de verin.", "packageDesign");
    }
    // Docs/adr/4420: a monitoring job's series and its days.
    crate::asbuilt_series::asked(input)
}

fn run(doc: &Document, input: &AsbuiltCheck) -> Result<AsbuiltChecked, Stop> {
    asked(input)?;
    // The field package the record was made with (docs/adr/2420): its digest expected, its limits where the input
    // gives none (the quality limit only with the σ's attribute).
    let package = input.package.as_deref().map(crate::asbuilt_package::read).transpose()?;
    let mut taken: Vec<String> = Vec::new();
    let effective;
    let input = match &package {
        Some(p) => {
            if input.expected_design.as_deref().is_some_and(|e| e != p.digest) {
                return Err(refuse(
                    codes::INVALID_ASBUILT_CHECK,
                    "Beklenen tasarım özeti saha paketininkinden farklı; yalnız paketi ya da yalnız özeti verin.".into(),
                    "expectedDesign",
                ));
            }
            let mut e = input.clone();
            if e.horizontal.is_none() && p.horizontal.is_some() {
                e.horizontal = p.horizontal;
                taken.push("horizontal".into());
            }
            if e.vertical.is_none() && p.vertical.is_some() {
                e.vertical = p.vertical;
                taken.push("vertical".into());
            }
            if e.quality.is_none() && p.quality.is_some() && e.quality_field.is_some() {
                e.quality = p.quality;
                taken.push("quality".into());
            }
            e.expected_design = Some(p.digest.clone());
            effective = e;
            &effective
        }
        None => input,
    };
    let layer = |id: &str, path: &str| {
        if doc.layers().get(id).is_some() {
            Ok(())
        } else {
            Err(refuse(codes::LAYER_NOT_FOUND, format!("“{id}” kimlikli katman çizimde yok."), path))
        }
    };
    layer(&input.measured, "measured")?;
    if let Some(d) = &input.design {
        layer(d, "design")?;
    }
    if let Some(l) = &input.lines {
        layer(l, "lines")?;
    }
    crate::asbuilt_series::layers(doc, input)?;
    // docs/adr/3220: with the package's design, the surface the package carries is the one compared; the drawing's
    // surface, given, is then not made, only its sources' digest read. Either way a surface the package names says
    // whether the drawing's (given) changed since: another surface, or its sources' digest not the package's.
    let carried = package
        .as_ref()
        .and_then(|p| p.surface.as_ref())
        .and_then(|s| s.file.as_ref())
        .filter(|_| input.package_design);
    let made;
    let mut surface_changed = false;
    let surface = match &input.surface {
        Some(name) => {
            terrains::terrain_project(doc)?;
            let id = crate::road_profiles::surface_id(doc, name, "surface")?;
            let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
            let named = package.as_ref().and_then(|p| p.surface.as_ref());
            match carried {
                Some(_) => {
                    surface_changed = named.is_some_and(|s| s.id != def.id || s.digest != terrains::sources_digest(doc, &def));
                    None
                }
                None => {
                    made = terrains::make(doc, &def);
                    surface_changed = named.is_some_and(|s| s.id != def.id || s.digest != made.gathered.digest);
                    Some(made.built()?)
                }
            }
        }
        None => None,
    };
    let field = input.field.as_deref().map(str::trim).unwrap_or(NAME_FIELD);
    let quality = input.quality_field.as_deref().map(str::trim);
    let measured = points(doc, &input.measured, field, quality);
    // Docs/adr/4420: the points measured now tracked through a monitoring job's series.
    let tracks = crate::asbuilt_series::tracks(doc, input, field, &measured);
    if measured.len() > ASBUILT_LIMIT {
        return Err(refuse(
            codes::INVALID_ASBUILT_CHECK,
            format!("{} ölçülen nokta var; en çok {ASBUILT_LIMIT} nokta denetlenir. Katmanı bölün.", measured.len()),
            "measured",
        ));
    }
    let here = input.design.as_deref().map_or_else(Vec::new, |d| points(doc, d, field, None));
    let here_lines = input.lines.as_deref().map_or_else(Vec::new, |l| lines(doc, l));
    // The design compared: the drawing's, or the package's own (docs/adr/2820), read back to its digest; then whether
    // the drawing's design (the layers given) is the package's.
    let (design, design_lines, design_changed) = match (input.package_design, &package) {
        (true, Some(p)) => {
            let (pts, lns) = crate::asbuilt_package::design(&p.path)?;
            if digest(&pts, &lns) != p.digest {
                return Err(refuse(
                    codes::INVALID_ASBUILT_CHECK,
                    format!(
                        "“{}” paketinin tasarım dosyaları (noktalar.csv, cizgiler.csv) özetiyle tutmuyor; paket bozulmuş \
                         ya da elle değiştirilmiş. Paketi Saha paketi yaz… ile yeniden yazın.",
                        file_name(&p.path)
                    ),
                    "package",
                ));
            }
            let changed = (input.design.is_some() || input.lines.is_some()) && digest(&here, &here_lines) != p.digest;
            (pts, lns, changed)
        }
        _ => {
            let changed = input.expected_design.as_deref().is_some_and(|e| e != digest(&here, &here_lines));
            (here, here_lines, changed)
        }
    };
    let design_digest = digest(&design, &design_lines);
    let ground = match (carried, &package) {
        (Some((file, sha256)), Some(p)) => Some(crate::asbuilt_package::ground(&p.path, file, sha256)?),
        _ => None,
    };
    let heights = match (&ground, surface) {
        (Some(g), _) => Some(Heights::Package(g)),
        (None, Some(s)) => Some(Heights::Drawing(s, ground_of(s))),
        (None, None) => None,
    };
    // By name: a name one design point has.
    let by_name = |name: &str| -> Vec<usize> {
        if name.is_empty() {
            return Vec::new();
        }
        design.iter().enumerate().filter(|(_, d)| d.name == name).map(|(i, _)| i).collect()
    };
    let claimed: Vec<bool> = {
        let mut c = vec![false; design.len()];
        for m in &measured {
            if let [i] = by_name(&m.name)[..] {
                c[i] = true;
            }
        }
        c
    };
    let mut used = vec![false; design.len()];
    let mut out = Vec::new();
    let (mut sum_h, mut n_h, mut sum_v, mut n_v) = (0.0, 0usize, 0.0, 0usize);
    let (mut max_h, mut max_v): (Option<f64>, Option<f64>) = (None, None);
    let mut counts = [0u32; 5];
    for m in &measured {
        let mut flags: Vec<&str> = Vec::new();
        let named = by_name(&m.name);
        let (matched, by, ambiguous) = match named[..] {
            [i] => (Some(i), Some("name"), false),
            [_, _, ..] => (None, None, true),
            [] => match input.search {
                Some(r) => {
                    let near: Vec<usize> = design
                        .iter()
                        .enumerate()
                        .filter(|(i, d)| !claimed[*i] && (d.p.x - m.p.x).hypot(d.p.y - m.p.y) <= r)
                        .map(|(i, _)| i)
                        .collect();
                    match near[..] {
                        [i] => (Some(i), Some("place"), false),
                        [] => (None, None, false),
                        _ => (None, None, true),
                    }
                }
                None => (None, None, false),
            },
        };
        if by == Some("place") {
            flags.push("proposed");
        }
        let mut point = AsbuiltPoint {
            uid: m.uid.clone(),
            name: m.name.clone(),
            design_uid: None,
            design_name: None,
            by: by.map(str::to_owned),
            de: None,
            dn: None,
            dh: None,
            horizontal: None,
            surface: None,
            nearest: None,
            nearest_at: None,
            normal: None,
            line_uid: None,
            sigma: m.sigma,
            status: String::new(),
            flags: Vec::new(),
        };
        let mut outside = false;
        if let Some(i) = matched {
            used[i] = true;
            let d = &design[i];
            let (de, dn) = (m.p.x - d.p.x, m.p.y - d.p.y);
            let h = de.hypot(dn);
            point.design_uid = Some(d.uid.clone());
            point.design_name = Some(d.name.clone());
            point.de = Some(de);
            point.dn = Some(dn);
            point.horizontal = Some(h);
            sum_h += h * h;
            n_h += 1;
            max_h = Some(max_h.map_or(h, |x: f64| x.max(h)));
            if input.horizontal.is_some_and(|t| h > t) {
                flags.push("outsideHorizontal");
                outside = true;
            }
            if let (Some(mz), Some(dz)) = (m.z, d.z) {
                let dh = mz - dz;
                point.dh = Some(dh);
                sum_v += dh * dh;
                n_v += 1;
                max_v = Some(max_v.map_or(dh.abs(), |x: f64| x.max(dh.abs())));
                if input.vertical.is_some_and(|t| dh.abs() > t) {
                    flags.push("outsideVertical");
                    outside = true;
                }
            }
        }
        if let Some(h) = &heights {
            match h.at(m.p) {
                Some(g) => {
                    if let Some(mz) = m.z {
                        let d = mz - g;
                        point.surface = Some(d);
                        // Docs/adr/4020: the nearest place in space, within the vertical distance (the foot under or
                        // over the point is on the surface), signed as the difference.
                        let near = h.nearest([m.p.x, m.p.y, mz], d.abs()).map(|(q, n)| (q, n.min(d.abs()).copysign(d)));
                        point.nearest = near.map(|(_, n)| n).or(Some(d));
                        point.nearest_at = near.map(|(q, _)| q).or(Some([m.p.x, m.p.y, g]));
                        let held = match input.surface_by.unwrap_or_default() {
                            AsbuiltSurfaceBy::Vertical => d,
                            AsbuiltSurfaceBy::Nearest => point.nearest.unwrap_or(d),
                        };
                        if input.vertical.is_some_and(|t| held.abs() > t) {
                            flags.push("outsideSurface");
                            outside = true;
                        }
                    }
                }
                None => flags.push("offSurface"),
            }
        }
        // A line point: one no design point matches (a stake point is checked against its own) and, a radius given,
        // within it of a line; its normal offset to the nearest line.
        let mut on_line = false;
        if !design_lines.is_empty() && matched.is_none() && !ambiguous {
            let mut best: Option<(f64, &str)> = None;
            for l in &design_lines {
                for (k, w) in l.pts.windows(2).enumerate() {
                    let d = signed(m.p, w[0], w[1], l.bulges[k]);
                    if best.is_none_or(|b| d.abs() < b.0.abs()) {
                        best = Some((d, l.uid.as_str()));
                    }
                }
            }
            if let Some((d, slot)) = best
                && input.search.is_none_or(|r| d.abs() <= r)
            {
                on_line = true;
                point.normal = Some(d);
                point.line_uid = Some(slot.to_owned());
                if input.horizontal.is_some_and(|t| d.abs() > t) {
                    flags.push("outsideLine");
                    outside = true;
                }
            }
        }
        if input.quality.zip(m.sigma).is_some_and(|(q, s)| s > q) {
            flags.push("lowQuality");
            counts[4] += 1;
        }
        if m.z.is_none() && (heights.is_some() || matched.is_some_and(|i| design[i].z.is_some())) {
            flags.push("noHeight");
        }
        // Matched to a design point or a line point: by its limits; neither, with points or lines to match: unmatched;
        // with a surface only: by the surface.
        // A design of points: the layer's, or the package's (docs/adr/2820).
        let points_compared = input.design.is_some() || input.package_design;
        let only_surface = !points_compared && design_lines.is_empty();
        point.status = if points_compared && ambiguous {
            counts[3] += 1;
            "ambiguous"
        } else if matched.is_none() && !on_line && !only_surface {
            counts[2] += 1;
            "unmatched"
        } else if outside {
            counts[1] += 1;
            "outside"
        } else {
            counts[0] += 1;
            "within"
        }
        .to_owned();
        flags.sort_by_key(|f| ASBUILT_FLAGS.iter().position(|k| k == f));
        point.flags = flags.into_iter().map(str::to_owned).collect();
        out.push(point);
    }
    let missing = design
        .iter()
        .zip(&used)
        .filter(|(_, u)| !**u)
        .map(|(d, _)| AsbuiltMissing { uid: d.uid.clone(), name: d.name.clone() })
        .collect();
    let rms = |sum: f64, n: usize| (n > 0).then(|| (sum / n as f64).sqrt());
    let surface_taken = ground.is_some();
    // A monitoring job's control points (docs/adr/3620): matched by name at both epochs, moved when a limit is broken.
    let control = (input.kind == Some(AsbuiltKind::Monitoring) && !input.control.is_empty()).then(|| {
        let mut c = AsbuiltControl { checked: Vec::new(), moved: Vec::new(), missing: Vec::new() };
        for name in &input.control {
            let name = name.trim();
            match out.iter().find(|p| p.name == name && p.by.as_deref() == Some("name")) {
                Some(p) => {
                    c.checked.push(name.to_owned());
                    if p.flags.iter().any(|f| f == "outsideHorizontal" || f == "outsideVertical") {
                        c.moved.push(name.to_owned());
                    }
                }
                None => c.missing.push(name.to_owned()),
            }
        }
        c
    });
    Ok(AsbuiltChecked {
        kind: input.kind,
        control,
        tracks,
        design_changed,
        surface_changed,
        design_digest,
        points: out,
        missing,
        within: counts[0],
        outside: counts[1],
        unmatched: counts[2],
        ambiguous: counts[3],
        low_quality: counts[4],
        rms_horizontal: rms(sum_h, n_h),
        rms_vertical: rms(sum_v, n_v),
        max_horizontal: max_h,
        max_vertical: max_v,
        package: package.map(|p| {
            let facts = kentos_project::facts::coordinate_facts(doc.settings());
            AsbuiltPackage {
                same_system: p.epsg.as_deref().unwrap_or("") == facts.epsg,
                same_epoch: p.epoch == facts.epoch,
                same_heights: p.heights.as_deref().unwrap_or("") == facts.heights,
                path: p.path,
                stamp: p.stamp,
                revision: p.revision,
                digest: p.digest,
                epsg: p.epsg,
                epoch: p.epoch,
                heights: p.heights,
                horizontal: p.horizontal,
                vertical: p.vertical,
                quality: p.quality,
                taken,
                design: input.package_design,
                surface: surface_taken,
            }
        }),
    })
}

/// A path's file name, as a refusal names it.
fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// Checks the question, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AsbuiltCheck) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The check; the same as execute.
pub fn plan(cx: &ExecutionContext<'_>, input: &AsbuiltCheck) -> CommandResult<AsbuiltChecked> {
    match run(cx.doc, input) {
        Ok(out) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The check, read only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AsbuiltCheck) -> CommandResult<AsbuiltChecked> {
    plan(cx, &input)
}
