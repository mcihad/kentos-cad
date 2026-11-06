//! `road.profiles.draw` v1 (plan/road.md Y4, docs/adr/0902 §8): a profile drawn on a layer as lines and texts, one undo
//! step “Boykesit çiz” (through `cad.entities.create`'s write), so it goes to sheets and DXF as Netcad's and Civil 3D's
//! profile views do. From the first PVI to the last: a frame whose lower left is `at` (the first PVI's station at the
//! datum), a metre of station a drawing unit and a metre of elevation `exaggeration` units; the elevation grid with its
//! heights; the terrain (black, off the surface left out) through the terrain profile's points (docs/adr/0902 §1); the
//! design (red) line, a curve in 16 pieces; each curve's sizes and K. Under the frame the bands Km, Siyah kot, Kırmızı
//! kot, Fark, Eğim: a column at every round km and at the ends, the grades between the PVIs. Lengths on paper: texts
//! `height` mm, a band 2.4 heights, the bands' names 14 heights wide. The shared cases are
//! `fixtures/commands/v1/road.profiles.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, ProfileAt, ProfileBands,
    ProfilesDraw, ProfilesDrawn, TextAlign, TextFace, Vec2,
};
use kentos_road::alignment::km_text;
use kentos_road::profile::{Piece, VCurve};
use kentos_road::surface::Surface;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_profiles::{self as common, Built};
use crate::road_terrains::fail;

/// The most columns one drawing writes.
const MOST: usize = 100_000;
/// The bands under the frame, top down.
pub const BANDS: [&str; 5] = ["Km", "Siyah kot", "Kırmızı kot", "Fark", "Eğim"];
/// KGM's bands (docs/adr/0980; KGM 2019 technical specification: "Profiller alt kısımda sırası ile Kırmızı Kot, Siyah
/// Kot, Kilometre, Yatay Geometri, Düşey Geometri ve Dever Bandı"), top down, and each one's depth in text heights:
/// the values of the first three are written up the band, as on KGM's and the schools' sheets.
pub const KGM_BANDS: [(&str, f64); 6] =
    [("Kırmızı Kot", 7.0), ("Siyah Kot", 7.0), ("Kilometre", 9.0), ("Yatay Geometri", 6.0), ("Düşey Geometri", 7.0), ("Dever", 7.0)];
const RED: &str = "#d62828";

/// A number as Rust writes an f64.
fn num(v: f64) -> String {
    format!("{v}")
}

struct Checked {
    id: String,
    built: Built,
    surface: Option<Surface>,
    datum: f64,
    top: f64,
    exaggeration: f64,
    height: f64,
    every: f64,
    /// The PVIs' stations and elevations, in order.
    pvis: Vec<(f64, f64)>,
    /// The stations drawn: the profile's ends or the range asked for.
    s0: f64,
    s1: f64,
    warnings: Vec<CommandWarning>,
    bands: ProfileBands,
    /// The Dever band's schedule (docs/adr/0980): its name, break points and normal slopes.
    dever: Option<(String, kentos_road::superelevation::Schedule, kentos_road::superelevation::Crossfall)>,
    /// KGM's slope and platform rows above the frame (docs/adr/0989).
    corridor: Option<[Vec<crate::road_corridor_bands::Cell>; 6]>,
}

/// The profile's range, its first and last PVI's stations.
fn range(built: &Built) -> (f64, f64) {
    (built.vertical.start, built.vertical.end)
}

/// The terrain's points inside the range `s0`–`s1`, and its ends: station, height (none off the surface).
fn terrain(built: &Built, surface: &Surface, s0: f64, s1: f64) -> Vec<(f64, Option<f64>)> {
    let core = &built.alignment.core;
    let ends: Vec<f64> =
        std::iter::once(0.0).chain(core.elements.iter().map(|e| e.station)).chain(std::iter::once(core.length)).collect();
    let black = |s: f64| core.at(s).and_then(|p| surface.height(p.x, p.y).z);
    let mut out = vec![(s0, black(s0))];
    out.extend(
        kentos_road::profile::terrain(surface, core)
            .into_iter()
            .filter(|p| (p.crossing || ends.contains(&p.station)) && p.station > s0 && p.station < s1)
            .map(|p| (p.station, p.z)),
    );
    out.push((s1, black(s1)));
    out
}

/// The stations whose red elevations bound the design line between `s0` and `s1`: its pieces' ends and its curves'
/// highest or lowest points there, and the range's ends.
fn design_stations(built: &Built, s0: f64, s1: f64) -> Vec<f64> {
    let mut out: Vec<f64> = built.vertical.pieces.iter().flat_map(|p| {
        let (a, b) = p.range();
        [a, b]
    }).collect();
    out.extend(built.vertical.curves.iter().filter_map(|c| c.extreme.map(|e| e[0])));
    out.retain(|s| *s >= s0 - 1e-9 && *s <= s1 + 1e-9);
    out.extend([s0, s1]);
    out
}

fn check(doc: &kentos_domain::Document, input: &ProfilesDraw) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = common::profiles(doc);
    let (_, def) = common::find(own, &input.profile, "profile")?;
    let every = input.every.unwrap_or(50.0);
    let height = input.height.unwrap_or(2.5);
    let exaggeration = input.exaggeration.unwrap_or(10.0);
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !positive(every) {
        return Err(fail(codes::INVALID_LABEL, "Km aralığı (every) 0'dan büyük olmalı.".into(), "every"));
    }
    if !positive(height) {
        return Err(fail(codes::INVALID_LABEL, "Yazı yüksekliği (height) 0'dan büyük olmalı.".into(), "height"));
    }
    if !positive(exaggeration) {
        return Err(fail(
            codes::INVALID_LABEL,
            "Düşey abartma (exaggeration) 0'dan büyük olmalı.".into(),
            "exaggeration",
        ));
    }
    let built = common::build(doc, def)?;
    let (p0, p1) = range(&built);
    let span = format!("boykesit {} ile {} m arası", num(p0), num(p1));
    let s0 = input.from.unwrap_or(p0);
    let s1 = input.to.unwrap_or(p1);
    if !s0.is_finite() || s0 < p0 - 1e-9 || s0 > p1 + 1e-9 {
        return Err(fail(codes::INVALID_LABEL, format!("{} m boykesitin üstünde değil: {span}.", num(s0)), "from"));
    }
    if !s1.is_finite() || s1 < p0 - 1e-9 || s1 > p1 + 1e-9 {
        return Err(fail(codes::INVALID_LABEL, format!("{} m boykesitin üstünde değil: {span}.", num(s1)), "to"));
    }
    if s1 <= s0 {
        return Err(fail(codes::INVALID_LABEL, "Aralığın sonu başından sonra olmalı.".into(), "to"));
    }
    let (s0, s1) = (s0.clamp(p0, p1), s1.clamp(p0, p1));
    if (s1 - s0) / every > MOST as f64 {
        return Err(fail(
            codes::INVALID_LABEL,
            format!("Bu aralıkla {MOST}'den çok sütun çıkar; every'yi büyütün."),
            "every",
        ));
    }
    let mut warnings = Vec::new();
    let surface = match common::surface(doc, def) {
        Some(Ok(s)) => Some(s),
        Some(Err(w)) => {
            warnings.push(w);
            None
        }
        None => None,
    };
    // The elevations shown: the design line's bounds and the terrain's points in the range.
    let mut zs: Vec<f64> =
        design_stations(&built, s0, s1).into_iter().filter_map(|s| built.vertical.at(s).map(|r| r.0)).collect();
    if let Some(s) = &surface {
        zs.extend(terrain(&built, s, s0, s1).into_iter().filter_map(|(_, z)| z));
    }
    let (lo, hi) = zs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(*z), hi.max(*z)));
    let datum = match input.datum {
        Some(d) if !d.is_finite() => {
            return Err(fail(codes::INVALID_LABEL, "Kıyas kotu (datum) sonlu bir sayı olmalı.".into(), "datum"));
        }
        Some(d) if d > lo + 1e-9 => {
            return Err(fail(
                codes::INVALID_LABEL,
                format!("Kıyas kotu (datum) en alçak kottan ({lo:.2}) yüksek olamaz."),
                "datum",
            ));
        }
        Some(d) => d,
        None => lo.floor() - 1.0,
    };
    let top = hi.ceil() + 1.0;
    let mut layer_warnings = checks::layer(doc, &input.layer_id)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (stale, now) = common::stale(doc, def);
    if stale {
        warnings.insert(
            0,
            CommandWarning {
                code: codes::PROFILE_STALE.into(),
                message: format!(
                    "“{}” boykesitinin aksı değişti (boykesit {}. revizyona göre kurulu, aks şimdi {now}.); PVI'ları denetleyip aksa yeniden bağlayın.",
                    def.name, def.alignment_revision
                ),
                path: Some("profile".into()),
            },
        );
    }
    layer_warnings.extend(warnings);
    let bands = input.bands.unwrap_or(ProfileBands::Kentos);
    let dever = match (&input.superelevation, bands) {
        (None, _) => None,
        (Some(_), ProfileBands::Kentos) => {
            return Err(fail(
                codes::INVALID_LABEL,
                "Dever çizelgesi yalnız KGM bantlarında (bands: kgm) Dever bandına çizilir.".into(),
                "superelevation",
            ));
        }
        (Some(name), ProfileBands::Kgm) => {
            let (_, sdef) = crate::road_superelevations::find(doc.road_superelevations(), name, "superelevation")?;
            if sdef.alignment != def.alignment {
                return Err(fail(
                    codes::INVALID_LABEL,
                    format!("“{}” dever çizelgesi boykesitin aksında değil.", sdef.name),
                    "superelevation",
                ));
            }
            let on = crate::road_superelevations::alignment_of(doc, sdef)?;
            let (schedule, _) = crate::road_superelevations::schedule(sdef, &on)?;
            layer_warnings.extend(crate::road_superelevations::stale_warning(doc, sdef));
            Some((sdef.name.clone(), schedule, crate::road_superelevations::normal(sdef)))
        }
    };
    // KGM's slope and platform rows above the frame (docs/adr/0989).
    let corridor = match (&input.corridor, bands) {
        (None, _) => None,
        (Some(_), ProfileBands::Kentos) => {
            return Err(fail(
                codes::INVALID_LABEL,
                "Şev ve platform satırları yalnız KGM bantlarıyla (bands: kgm) çizilir.".into(),
                "corridor",
            ));
        }
        (Some(name), ProfileBands::Kgm) => {
            let (_, cdef) = crate::road_corridors::find(doc.road_corridors(), name, "corridor")?;
            if cdef.alignment != def.alignment {
                return Err(fail(
                    codes::INVALID_LABEL,
                    format!("“{}” koridoru boykesitin aksında değil.", cdef.name),
                    "corridor",
                ));
            }
            Some(crate::road_corridor_bands::rows(doc, cdef, s0, s1)?)
        }
    };
    Ok(Checked {
        id: def.id.clone(),
        built,
        surface,
        datum,
        top,
        exaggeration,
        height,
        every,
        pvis: def.pvis.iter().map(|p| (p.station, p.elevation)).collect(),
        s0,
        s1,
        warnings: layer_warnings,
        bands,
        dever,
        corridor,
    })
}

/// The elevation grid's step: a round one giving at most ten lines.
pub fn grid_step(datum: f64, top: f64) -> f64 {
    [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0]
        .into_iter()
        .find(|g| (top - datum) / g <= 10.0)
        .unwrap_or(1000.0)
}

/// The bands' columns: every round km in the range and its ends, in station order.
fn columns(c: &Checked) -> Result<Vec<ProfileAt>, Stop> {
    let (s0, s1) = (c.s0, c.s1);
    let mut stations: Vec<f64> = alignments::tick_stations(&c.built.alignment, c.every)?
        .into_iter()
        .filter(|s| *s >= s0 - 1e-9 && *s <= s1 + 1e-9)
        .map(|s| s.clamp(s0, s1))
        .collect();
    for end in [s0, s1] {
        if stations.iter().all(|s| (s - end).abs() > 1e-6) {
            stations.push(end);
        }
    }
    stations.sort_by(f64::total_cmp);
    Ok(stations.into_iter().filter_map(|s| common::at(&c.built, c.surface.as_ref(), s)).collect())
}

fn object(geometry: EntityGeometry, color: Option<&str>, weight: Option<f64>) -> NewObject {
    NewObject {
        geometry,
        color: color.map(str::to_owned),
        line_weight: weight,
        line_type: None,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

fn line(a: [f64; 2], b: [f64; 2], color: Option<&str>, weight: Option<f64>) -> NewObject {
    object(
        EntityGeometry::Line {
            a: Vec2 { x: a[0], y: a[1] },
            b: Vec2 { x: b[0], y: b[1] },
            zs: None,
            ms: None,
        },
        color,
        weight,
    )
}

fn path(pts: &[[f64; 2]], closed: bool, color: Option<&str>, weight: Option<f64>) -> NewObject {
    object(
        EntityGeometry::Polyline {
            pts: pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
            bulges: None,
            zs: None,
            ms: None,
            parts: None,
            closed,
            widths: None,
        },
        color,
        weight,
    )
}

fn text(at: [f64; 2], words: String, height: f64, align: TextAlign, color: Option<&str>) -> NewObject {
    turned(at, words, height, align, color, 0.0)
}

/// A text turned `rotation` degrees counterclockwise (up the band: 90; the contract's unit).
fn turned(at: [f64; 2], words: String, height: f64, align: TextAlign, color: Option<&str>, rotation: f64) -> NewObject {
    object(
        EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: at[0], y: at[1] },
            text: words,
            height,
            rotation,
            align: Some(align),
            width_factor: None,
            mask: false,
            box_width: None,
            line_spacing: None,
            runs: Vec::new(),
            face: TextFace::default(),
            path: None,
        },
        color,
        None,
    )
}

/// The runs of the terrain drawn: a stretch between two points is drawn where both have a height and its middle lies
/// on the surface.
pub fn terrain_runs(points: &[(f64, Option<f64>)], on: impl Fn(f64) -> bool) -> Vec<Vec<[f64; 2]>> {
    let mut runs: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut open = false;
    for w in points.windows(2) {
        let ((sa, za), (sb, zb)) = (w[0], w[1]);
        match (za, zb) {
            (Some(za), Some(zb)) if on(0.5 * (sa + sb)) => {
                if !open {
                    runs.push(vec![[sa, za]]);
                    open = true;
                }
                if let Some(run) = runs.last_mut() {
                    run.push([sb, zb]);
                }
            }
            _ => open = false,
        }
    }
    runs
}

fn objects(doc: &kentos_domain::Document, c: &Checked, columns: &[ProfileAt], at: [f64; 2]) -> Vec<NewObject> {
    let mm = doc.settings().plot_scale / 1000.0;
    let h = c.height * mm;
    let row = 2.4 * h;
    let names = 14.0 * h;
    let (s0, s1) = (c.s0, c.s1);
    let x = |s: f64| at[0] + (s - s0);
    let y = |z: f64| at[1] + (z - c.datum) * c.exaggeration;
    let mut out = Vec::new();
    // The frame and the grid.
    let (x0, x1, y0, y1) = (x(s0), x(s1), y(c.datum), y(c.top));
    out.push(path(&[[x0, y0], [x1, y0], [x1, y1], [x0, y1]], true, None, Some(0.35)));
    let g = grid_step(c.datum, c.top);
    let mut k = (c.datum / g).ceil();
    while k * g <= c.top + 1e-9 {
        let v = k * g;
        if v > c.datum + 1e-9 && v < c.top - 1e-9 {
            out.push(line([x0, y(v)], [x1, y(v)], None, Some(0.13)));
        }
        out.push(text([x0 - h, y(v)], format!("{v:.2}"), h, TextAlign::MiddleRight, None));
        k += 1.0;
    }
    // The terrain.
    if let Some(surface) = &c.surface {
        let core = &c.built.alignment.core;
        let on = |s: f64| core.at(s).is_some_and(|p| surface.height(p.x, p.y).z.is_some());
        for run in terrain_runs(&terrain(&c.built, surface, s0, s1), on) {
            let pts: Vec<[f64; 2]> = run.iter().map(|p| [x(p[0]), y(p[1])]).collect();
            out.push(path(&pts, false, None, Some(0.25)));
        }
    }
    // The design line, a curve in 16 pieces, and each curve's sizes.
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for p in &c.built.vertical.pieces {
        let (a, b) = p.range();
        if b <= s0 + 1e-9 || a >= s1 - 1e-9 {
            continue;
        }
        let (a, b) = (a.max(s0), b.min(s1));
        let n = if matches!(p, Piece::Grade { .. }) { 1 } else { 16 };
        for i in 0..=n {
            let s = a + (b - a) * i as f64 / n as f64;
            let z = p.at(s).0;
            if pts.last().is_none_or(|q| (q[0] - x(s)).abs() > 1e-9) {
                pts.push([x(s), y(z)]);
            }
        }
    }
    out.push(path(&pts, false, Some(RED), Some(0.5)));
    for cu in c.built.vertical.curves.iter().filter(|cu| cu.start[0] < s1 - 1e-9 && cu.end[0] > s0 + 1e-9) {
        let size = match cu.curve {
            VCurve::Asymmetric { length_in, length_out } => format!("L1={length_in:.2} L2={length_out:.2}"),
            VCurve::Circle { radius } => format!("R={radius:.2} L={:.2}", cu.end[0] - cu.start[0]),
            _ => format!("L={:.2}", cu.end[0] - cu.start[0]),
        };
        let top = cu.start[1].max(cu.end[1]).max(cu.extreme.map_or(f64::NEG_INFINITY, |e| e[1]));
        out.push(text(
            [x(0.5 * (cu.start[0].max(s0) + cu.end[0].min(s1))), y(top) + 2.0 * h],
            format!("{} {size} K={:.1}", cu.pvi, cu.k.unwrap_or(0.0)),
            h,
            TextAlign::BottomCenter,
            Some(RED),
        ));
    }
    // The project's ends on the design line (docs/adr/0987; KGM 2019 şartname s. 5: "Proje başı ve sonunda kot ve
    // koordinat değerleri yazılacaktır"; the June 2024 sample sheets: a red leader from the line's end to PROJE BAŞI,
    // KM : 0+000.00 and KOT = 964.670). The leader runs 12 mm away from the frame's nearer edge; the words read down.
    if let (Some(&first), Some(&last)) = (c.pvis.first(), c.pvis.last()) {
        let pitch = 1.6 * h;
        for (words, (s, z), start) in [("PROJE BAŞI", first, true), ("PROJE SONU", last, false)] {
            if s < s0 - 1e-9 || s > s1 + 1e-9 {
                continue;
            }
            let p = [x(s), y(z)];
            let down = p[1] > 0.5 * (y0 + y1);
            // Slanted 6 mm into the frame, off its edge, as the 2024 sheets draw it.
            let inward = if start { 6.0 * mm } else { -6.0 * mm };
            let end = [p[0] + inward, if down { p[1] - 12.0 * mm } else { p[1] + 12.0 * mm }];
            out.push(line(p, end, Some(RED), Some(0.25)));
            let (dx, align) = if start { (0.5 * h, TextAlign::MiddleLeft) } else { (-0.5 * h, TextAlign::MiddleRight) };
            let lines = [words.to_owned(), format!("KM : {}", km_text(c.built.alignment.km(s), 2)), format!("KOT = {z:.3}")];
            for (i, l) in lines.into_iter().enumerate() {
                // Down: from the leader's end downward; up: the last line at the leader's end.
                let k = if down { -(i as f64) } else { (2 - i) as f64 };
                let mut words = text([end[0] + dx, end[1] + k * pitch], l, h, align, if i == 0 { Some(RED) } else { None });
                // Masked: the grid's lines and the ground line ran through them (Y81's first picture).
                if let EntityGeometry::Text { mask, .. } = &mut words.geometry {
                    *mask = true;
                }
                out.push(words);
            }
        }
    }
    // KGM's slope and platform rows right above the frame (docs/adr/0989): the names' column, a line over each row,
    // each run of a value its words in the middle and a line where it changes.
    if let Some(rows) = &c.corridor {
        let n = rows.len() as f64;
        out.push(line([x0 - names, y1], [x0 - names, y1 + n * row], None, Some(0.25)));
        out.push(line([x0, y1], [x0, y1 + n * row], None, Some(0.25)));
        for (k, cells) in rows.iter().enumerate() {
            let (bottom, top) = (y1 + row * (n - 1.0 - k as f64), y1 + row * (n - k as f64));
            out.push(line([x0 - names, top], [x1, top], None, Some(0.18)));
            let mid = 0.5 * (bottom + top);
            out.push(text([x0 - names + 0.5 * h, mid], crate::road_corridor_bands::ROWS[k].to_owned(), h, TextAlign::MiddleLeft, None));
            for (i, cell) in cells.iter().enumerate() {
                if !cell.text.is_empty() {
                    out.push(text([x(0.5 * (cell.from + cell.to)), mid], cell.text.clone(), h, TextAlign::MiddleCenter, None));
                }
                if i + 1 < cells.len() {
                    out.push(line([x(cell.to), bottom], [x(cell.to), top], None, Some(0.18)));
                }
            }
        }
    }
    if c.bands == ProfileBands::Kgm {
        kgm_bands(&mut out, c, columns, at, h, names);
        return out;
    }
    // The bands: their lines and names, then a column at every round km and at the ends, then the grades.
    for (i, name) in BANDS.iter().enumerate() {
        let bottom = y0 - row * (i + 1) as f64;
        out.push(line([x0 - names, bottom], [x1, bottom], None, Some(0.13)));
        out.push(text([x0 - names + 0.5 * h, bottom + 0.5 * row], (*name).to_owned(), h, TextAlign::MiddleLeft, None));
    }
    let mid = |i: usize| y0 - row * (i as f64 + 0.5);
    for col in columns {
        let cx = x(col.station);
        out.push(line([cx, y0], [cx, y0 - row * 4.0], None, Some(0.13)));
        let round = (col.km / c.every - (col.km / c.every).round()).abs() <= 1e-9;
        out.push(text([cx, mid(0)], km_text(col.km, if round { 0 } else { 2 }), h, TextAlign::MiddleCenter, None));
        if let Some(b) = col.black {
            out.push(text([cx, mid(1)], format!("{b:.2}"), h, TextAlign::MiddleCenter, None));
        }
        if let Some(r) = col.red {
            out.push(text([cx, mid(2)], format!("{r:.2}"), h, TextAlign::MiddleCenter, Some(RED)));
        }
        if let Some(d) = col.difference {
            out.push(text([cx, mid(3)], format!("{d:+.2}"), h, TextAlign::MiddleCenter, None));
        }
    }
    for w in c.pvis.windows(2) {
        let ((sa, za), (sb, zb)) = (w[0], w[1]);
        if sb <= s0 + 1e-9 || sa >= s1 - 1e-9 {
            continue;
        }
        let g = (zb - za) / (sb - sa);
        out.push(text(
            [x(0.5 * (sa.max(s0) + sb.min(s1))), mid(4)],
            format!("{} L={:.2}", kentos_road::profile::grade_text(g), sb - sa),
            h,
            TextAlign::MiddleCenter,
            None,
        ));
    }
    out
}

fn write(cx: &mut ExecutionContext<'_>, input: &ProfilesDraw, c: Checked) -> CommandResult<ProfilesDrawn> {
    let cols = match columns(&c) {
        Ok(cols) => cols,
        Err(stop) => return stop.into(),
    };
    let objects = objects(cx.doc, &c, &cols, input.at);
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects,
        operation: Some(CreateOperation::ProfileDrawing),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    let (s0, s1) = (c.s0, c.s1);
    CommandResult::Completed {
        output: ProfilesDrawn {
            profile: c.id,
            frame: [
                [input.at[0], input.at[1]],
                [input.at[0] + (s1 - s0), input.at[1] + (c.top - c.datum) * c.exaggeration],
            ],
            datum: c.datum,
            top: c.top,
            exaggeration: c.exaggeration,
            grid: grid_step(c.datum, c.top),
            columns: cols,
            uids: created.created,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// A profile view as a sheet set draws it (docs/adr/0908 §10): its objects, what `road.profiles.draw` would answer
/// (no objects written yet) and its warnings.
pub(crate) fn view(
    doc: &kentos_domain::Document,
    input: &ProfilesDraw,
) -> Result<(Vec<NewObject>, ProfilesDrawn, Vec<CommandWarning>), Stop> {
    let c = check(doc, input)?;
    let cols = columns(&c)?;
    let objects = objects(doc, &c, &cols, input.at);
    let drawn = ProfilesDrawn {
        profile: c.id.clone(),
        frame: [
            [input.at[0], input.at[1]],
            [input.at[0] + (c.s1 - c.s0), input.at[1] + (c.top - c.datum) * c.exaggeration],
        ],
        datum: c.datum,
        top: c.top,
        exaggeration: c.exaggeration,
        grid: grid_step(c.datum, c.top),
        columns: cols,
        uids: Vec::new(),
        revision: doc.revision().to_string(),
    };
    Ok((objects, drawn, c.warnings))
}

/// How tall KGM's slope and platform rows over a view are (docs/adr/0989), in drawing units, with texts `height` paper
/// millimetres high.
pub(crate) fn corridor_rows_height(doc: &kentos_domain::Document, height: f64) -> f64 {
    crate::road_corridor_bands::ROWS.len() as f64 * 2.4 * height * doc.settings().plot_scale / 1000.0
}

/// How deep the bands under a view are, in drawing units, with texts `height` paper millimetres high.
pub(crate) fn bands_depth(doc: &kentos_domain::Document, height: f64) -> f64 {
    BANDS.len() as f64 * 2.4 * height * doc.settings().plot_scale / 1000.0
}

/// The bands KGM's 2019 technical specification asks for (docs/adr/0980), under the frame whose lower left is `at`:
/// Kırmızı Kot, Siyah Kot, Kilometre (each column's value written up the band), Yatay Geometri (a tangent on the
/// band's axis, a curve a step up for a right turn and down for a left one, a clothoid the slope between; R, the deflection A (KGM: Δ is not in the drawing font, docs/adr/0982) in
/// grad, L, a clothoid's A), Düşey Geometri (the grades as KGM writes them, a crest a step up and a sag down with its
/// length and K, each PVI's km and elevation) and Dever (the left side's slope solid, the right's dashed, about the
/// band's axis, each break point's two slopes).
fn kgm_bands(out: &mut Vec<NewObject>, c: &Checked, columns: &[ProfileAt], at: [f64; 2], h: f64, names: f64) {
    use kentos_road::alignment::Kind;
    use kentos_road::profile::grade_text;
    let (s0, s1) = (c.s0, c.s1);
    let x = |s: f64| at[0] + (s - s0);
    let (x0, x1, y0) = (x(s0), x(s1), at[1]);
    // Each band's top and bottom.
    let mut rows = Vec::new();
    let mut top = y0;
    for (name, depth) in KGM_BANDS {
        let bottom = top - depth * h;
        out.push(line([x0 - names, bottom], [x1, bottom], None, Some(0.13)));
        out.push(text([x0 - names + 0.5 * h, 0.5 * (top + bottom)], name.to_owned(), h, TextAlign::MiddleLeft, None));
        rows.push((top, bottom));
        top = bottom;
    }
    out.push(line([x0 - names, y0], [x0 - names, top], None, Some(0.13)));
    let mid = |i: usize| 0.5 * (rows[i].0 + rows[i].1);
    let up = 90.0;
    // Kırmızı Kot, Siyah Kot, Kilometre: a column at every round km and at the ends.
    for col in columns {
        let cx = x(col.station);
        out.push(line([cx, y0], [cx, rows[2].1], None, Some(0.13)));
        let round = (col.km / c.every - (col.km / c.every).round()).abs() <= 1e-9;
        let km = if round { km_text(col.km, 0) } else { km_text(col.km, 2) };
        if let Some(r) = col.red {
            out.push(turned([cx - 0.2 * h, mid(0)], format!("{r:.2}"), h, TextAlign::BottomCenter, Some(RED), up));
        }
        if let Some(b) = col.black {
            out.push(turned([cx - 0.2 * h, mid(1)], format!("{b:.2}"), h, TextAlign::BottomCenter, None, up));
        }
        out.push(turned([cx - 0.2 * h, mid(2)], km, h, TextAlign::BottomCenter, None, up));
    }
    // Yatay Geometri: the elements in the range, each at its step.
    let core = &c.built.alignment.core;
    let step = 1.5 * h;
    let off = |k: f64| if k > 1e-12 { -step } else if k < -1e-12 { step } else { 0.0 };
    let ym = mid(3);
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for e in core.elements.iter().filter(|e| e.station < s1 - 1e-9 && e.station + e.length > s0 + 1e-9) {
        let (a, b) = (e.station.max(s0), (e.station + e.length).min(s1));
        let (oa, ob) = (off(e.curvature(a - e.station)), off(e.curvature(b - e.station)));
        // A clothoid from or to a tangent slopes between the two steps; one between two curves stays at theirs.
        let (oa, ob) = match e.kind {
            Kind::Spiral if e.k0 == 0.0 || e.k1 == 0.0 => (off(e.k0) * (1.0 - (a - e.station) / e.length) + off(e.k1) * ((a - e.station) / e.length),
                                                              off(e.k0) * (1.0 - (b - e.station) / e.length) + off(e.k1) * ((b - e.station) / e.length)),
            _ => (oa, ob),
        };
        pts.push([x(a), ym + oa]);
        pts.push([x(b), ym + ob]);
        out.push(line([x(a), rows[3].0], [x(a), rows[3].1], None, Some(0.13)));
        let cx = x(0.5 * (a + b));
        let words = match e.kind {
            Kind::Tangent => format!("L={:.2}", e.length),
            Kind::Arc => {
                let delta = core.curves.iter().find(|cu| e.id.starts_with(&format!("{}/", cu.pi))).map(|cu| cu.delta.abs());
                match delta {
                    Some(d) => format!("R={:.2} A={:.4}g L={:.2}", 1.0 / e.k0.abs(), d * 200.0 / std::f64::consts::PI, e.length),
                    None => format!("R={:.2} L={:.2}", 1.0 / e.k0.abs(), e.length),
                }
            }
            Kind::Spiral => format!("Kl. A={:.2} L={:.2}", e.a().unwrap_or(0.0), e.length),
        };
        // The words on the step's far side from the axis, a tangent's over it.
        let side = 0.5 * (oa + ob);
        let (wy, align) = if side < 0.0 { (ym + side - 0.3 * h, TextAlign::TopCenter) } else { (ym + side + 0.3 * h, TextAlign::BottomCenter) };
        out.push(text([cx, wy], words, 0.8 * h, align, None));
    }
    pts.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9);
    if pts.len() >= 2 {
        out.push(path(&pts, false, None, Some(0.35)));
    }
    // Düşey Geometri: the grades between the PVIs, a crest a step up and a sag down, each PVI's km and elevation. Three
    // lines of words that do not meet: the curves' along the top, the grades over the axis, the PVIs' along the bottom.
    let ym = rows[4].0 - 3.2 * h;
    let step = h;
    let km_of = |s: f64| kentos_road::alignment::Stationing::km(&c.built.alignment.regions, s);
    let mut vpts: Vec<[f64; 2]> = vec![[x0, ym]];
    for cu in c.built.vertical.curves.iter().filter(|cu| cu.start[0] < s1 - 1e-9 && cu.end[0] > s0 + 1e-9) {
        let (a, b) = (cu.start[0].max(s0), cu.end[0].min(s1));
        let o = if cu.crest { step } else { -step };
        vpts.extend([[x(a), ym], [x(a), ym + o], [x(b), ym + o], [x(b), ym]]);
        let length = cu.end[0] - cu.start[0];
        out.push(text(
            [x(0.5 * (a + b)), rows[4].0 - 0.3 * h],
            format!("{} ŞK={length:.2} K={:.1}", cu.pvi, cu.k.unwrap_or(0.0)),
            0.8 * h,
            TextAlign::TopCenter,
            None,
        ));
    }
    vpts.push([x1, ym]);
    out.push(path(&vpts, false, None, Some(0.35)));
    for w in c.pvis.windows(2) {
        let ((sa, za), (sb, zb)) = (w[0], w[1]);
        if sb <= s0 + 1e-9 || sa >= s1 - 1e-9 {
            continue;
        }
        let g = (zb - za) / (sb - sa);
        out.push(text(
            [x(0.5 * (sa.max(s0) + sb.min(s1))), ym + step + 0.3 * h],
            format!("{} L={:.2}", grade_text(g), sb - sa),
            0.8 * h,
            TextAlign::BottomCenter,
            None,
        ));
    }
    for &(sp, zp) in c.pvis.iter().filter(|(sp, _)| *sp >= s0 - 1e-9 && *sp <= s1 + 1e-9) {
        out.push(line([x(sp), rows[4].1], [x(sp), rows[4].1 + 0.8 * h], None, Some(0.13)));
        out.push(text([x(sp), rows[4].1 + 0.15 * h], format!("{} {zp:.2}", km_text(km_of(sp), 3)), 0.7 * h, TextAlign::BottomCenter, None));
    }
    // Dever: about the band's axis, the left solid and the right dashed, each break point's slopes.
    let (top5, bottom5) = rows[5];
    let ym = mid(5);
    match &c.dever {
        None => out.push(text([0.5 * (x0 + x1), ym], "Dever çizelgesi verilmedi".into(), 0.8 * h, TextAlign::MiddleCenter, None)),
        Some((name, schedule, normal)) => {
            out.push(line([x0, ym], [x1, ym], None, Some(0.13)));
            let side = |points: &[[f64; 2]], n: f64| -> Vec<[f64; 2]> {
                let mut v: Vec<[f64; 2]> = vec![[s0, points.first().map_or(n, |p| p[1])]];
                v.extend(points.iter().copied().filter(|p| p[0] > s0 && p[0] < s1));
                v.push([s1, points.last().map_or(n, |p| p[1])]);
                // The ends' slopes as the schedule has them there.
                v[0][1] = value_at(points, s0, n);
                let last = v.len() - 1;
                v[last][1] = value_at(points, s1, n);
                v
            };
            let (l, r) = (side(&schedule.left, normal.left), side(&schedule.right, normal.right));
            let most = l.iter().chain(&r).map(|p| p[1].abs()).fold(0.0_f64, f64::max).max(1e-9);
            let scale = (0.5 * (top5 - bottom5) - 1.2 * h) / most;
            let yv = |e: f64| ym + e * scale;
            let left: Vec<[f64; 2]> = l.iter().map(|p| [x(p[0]), yv(p[1])]).collect();
            out.push(path(&left, false, None, Some(0.35)));
            // The right side dashed, as on KGM's sample sheets.
            let right: Vec<[f64; 2]> = r.iter().map(|p| [x(p[0]), yv(p[1])]).collect();
            let mut dashed = path(&right, false, None, Some(0.35));
            dashed.line_type = Some(kentos_commands::layer::LineType::Dashed);
            out.push(dashed);
            let mut breaks: Vec<f64> = schedule.left.iter().chain(&schedule.right).map(|p| p[0]).filter(|s| *s >= s0 && *s <= s1).collect();
            breaks.sort_by(f64::total_cmp);
            breaks.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            for s in breaks {
                let (el, er) = (value_at(&schedule.left, s, normal.left), value_at(&schedule.right, s, normal.right));
                out.push(line([x(s), bottom5], [x(s), bottom5 + 0.6 * h], None, Some(0.13)));
                out.push(turned(
                    [x(s) - 0.2 * h, bottom5 + 0.7 * h],
                    format!("{} {}", grade_text(el), grade_text(er)),
                    0.6 * h,
                    TextAlign::MiddleLeft,
                    None,
                    up,
                ));
            }
            out.push(text([x0 + 0.5 * h, top5 - 0.3 * h], format!("{name}: sol düz, sağ kesikli"), 0.7 * h, TextAlign::TopLeft, None));
        }
    }
}

/// A schedule side's slope at `s`: linear between its break points, its first before them and its last after; the
/// normal slope when it has none.
fn value_at(points: &[[f64; 2]], s: f64, normal: f64) -> f64 {
    match points {
        [] => normal,
        [only] => only[1],
        _ if s <= points[0][0] => points[0][1],
        _ if s >= points[points.len() - 1][0] => points[points.len() - 1][1],
        _ => {
            let i = points.windows(2).position(|w| s >= w[0][0] && s <= w[1][0]).unwrap_or(0);
            let (a, b) = (points[i], points[i + 1]);
            if b[0] > a[0] { a[1] + (b[1] - a[1]) * (s - a[0]) / (b[0] - a[0]) } else { b[1] }
        }
    }
}

/// How deep KGM's bands are, in drawing units, with texts `height` paper millimetres high (docs/adr/0980).
pub(crate) fn kgm_depth(doc: &kentos_domain::Document, height: f64) -> f64 {
    KGM_BANDS.iter().map(|(_, d)| d).sum::<f64>() * height * doc.settings().plot_scale / 1000.0
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesDraw) -> CommandResult<ProfilesDrawn> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), input, c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.uids.clear();
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the drawing: one undo step.
/// Draws `input` now, checked and written in the step it runs in, its drawing not kept (`road.project.rebuild` keeps it,
/// docs/adr/0911 §3).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &kentos_commands::ProfilesDraw) -> CommandResult<kentos_commands::ProfilesDrawn> {
    write_execute(cx, input.clone())
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesDraw) -> CommandResult<ProfilesDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_PROFILES_DRAW,
        kentos_commands::ROAD_PROFILES_DRAW_VERSION,
        input,
        |cx, input| {
            let r = write_execute(cx, input.clone());
            crate::road_drawings::kept(cx, kentos_commands::ROAD_PROFILES_DRAW, &input, (kentos_commands::RoadSourceKind::Profile, "profile"), r, |o| &o.uids)
        },
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ProfilesDraw) -> CommandResult<ProfilesDrawn> {
    match check(cx.doc, &input) {
        Ok(c) => write(cx, &input, c),
        Err(stop) => stop.into(),
    }
}
