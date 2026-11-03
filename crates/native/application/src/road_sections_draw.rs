//! `road.sections.draw` v1 (plan/road.md Y5, docs/adr/0903 §7): a sample line set's sections drawn on a layer as
//! section views in a grid, lines and texts, one undo step “Enkesit çiz” (through `cad.entities.create`'s write), so
//! they go to sheets and DXF as Civil 3D's section view groups and Netcad's section sheets do. The views lie in rows of
//! `columns` from `at` (the sheet's upper left), left to right then down; a view is the swath wide (a metre of offset a
//! drawing unit) and the set's tallest elevation range high (a metre of elevation `exaggeration` units), its datum a
//! metre under its lowest elevation, floored. In each: the frame, the elevation and offset grids with their numbers,
//! the terrain (black), the platform and slopes (red), the axis, the km over the frame, the axis elevation and the cut
//! and fill areas under it. Lengths on paper: texts `height` mm; a text row 2.4 heights; the elevations' column 8
//! heights; 4 heights between views. The shared cases are `fixtures/commands/v1/road.sections.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, SectionRow,
    SectionView, SectionsDraw, SectionsDrawn, TextAlign, TextFace, Vec2,
};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_profiles_draw::grid_step;
use crate::road_sections as common;
use crate::road_terrains::fail;

const RED: &str = "#d62828";
const AXIS: &str = "#2563eb";

struct Checked {
    id: String,
    left: f64,
    right: f64,
    rows: Vec<SectionRow>,
    /// Each view's datum.
    datums: Vec<f64>,
    span: f64,
    columns: usize,
    exaggeration: f64,
    height: f64,
    warnings: Vec<CommandWarning>,
}

/// The offset grid's step: a round one giving at most ten spaces over the swath.
pub fn offset_step(width: f64) -> f64 {
    [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0]
        .into_iter()
        .find(|g| width / g <= 10.0)
        .unwrap_or(1000.0)
}

/// A row's design line, from its left end to its right; empty without a platform.
pub fn design_line(row: &SectionRow) -> Vec<[f64; 2]> {
    match (&row.left, &row.right) {
        (Some(l), Some(r)) => l.points.iter().rev().chain(r.points.iter().skip(1)).copied().collect(),
        _ => Vec::new(),
    }
}

/// A row's elevations: its terrain's, its design line's and its axis'.
fn elevations(row: &SectionRow) -> Vec<f64> {
    row.ground.iter().filter_map(|g| g.z).chain(design_line(row).into_iter().map(|p| p[1])).chain(row.axis).collect()
}

/// The terrain's runs: a stretch is drawn where both ends have a height and it lies on the surface.
pub fn ground_runs(row: &SectionRow) -> Vec<Vec<[f64; 2]>> {
    let mut runs: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut open = false;
    for w in row.ground.windows(2) {
        match (w[0].z, w[1].z) {
            (Some(za), Some(zb)) if !w[1].gap => {
                if !open {
                    runs.push(vec![[w[0].offset, za]]);
                    open = true;
                }
                if let Some(run) = runs.last_mut() {
                    run.push([w[1].offset, zb]);
                }
            }
            _ => open = false,
        }
    }
    runs
}

fn check(doc: &kentos_domain::Document, input: &SectionsDraw) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find_set(doc.road_sections(), &input.set, "set")?;
    let columns = input.columns.unwrap_or(5);
    let height = input.height.unwrap_or(2.5);
    let exaggeration = input.exaggeration.unwrap_or(1.0);
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !(1..=100).contains(&columns) {
        return Err(fail(codes::INVALID_LABEL, "Bir satırda 1 ile 100 arası görünüş (columns) olur.".into(), "columns"));
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
    let built = common::alignment_of(doc, def)?;
    let all = common::stations_of(doc, def, &built)?;
    let mut stations = Vec::new();
    for (i, s) in input.stations.iter().enumerate() {
        match all.iter().find(|k| (*k - s).abs() < common::SAME_STATION) {
            Some(k) => stations.push(*k),
            None => {
                return Err(fail(
                    codes::INVALID_PLACES,
                    format!("{s} m “{}” kesit takımının km'lerinden biri değil.", def.name),
                    &format!("stations[{i}]"),
                ));
            }
        }
    }
    if stations.is_empty() {
        stations = all;
    }
    match common::surface_of(doc, def) {
        None => {
            return Err(fail(
                codes::NO_TERRAIN,
                format!("“{}” kesit takımının yüzeyi yok: enkesit çizmek için önce bir yüzey verin.", def.name),
                "set",
            ));
        }
        Some(Err(_)) => {
            return Err(fail(
                codes::NO_TERRAIN,
                format!("“{}” kesit takımının yüzeyi projede yok ya da kurulamıyor: önce yüzeyi düzeltin.", def.name),
                "set",
            ));
        }
        Some(Ok(_)) => {}
    }
    let (rows, more) = common::sections(doc, def, &built, &stations);
    let ranges: Vec<Option<(f64, f64)>> = rows
        .iter()
        .map(|r| {
            let zs = elevations(r);
            let lo = zs.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = zs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (lo.is_finite() && hi.is_finite()).then(|| (lo.floor() - 1.0, hi.ceil() + 1.0))
        })
        .collect();
    let Some(lowest) = ranges.iter().flatten().map(|r| r.0).reduce(f64::min) else {
        return Err(fail(
            codes::NO_TERRAIN,
            format!("“{}” kesit takımının kesit hatlarında arazi yok: hatlar yüzeyin dışında.", def.name),
            "set",
        ));
    };
    let span = ranges.iter().flatten().map(|r| r.1 - r.0).fold(0.0, f64::max);
    let datums = ranges.iter().map(|r| r.map_or(lowest, |r| r.0)).collect();
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    warnings.extend(common::stale_warning(doc, def));
    warnings.extend(more);
    Ok(Checked {
        id: def.id.clone(),
        left: def.left,
        right: def.right,
        rows,
        datums,
        span,
        columns: columns as usize,
        exaggeration,
        height,
        warnings,
    })
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
        EntityGeometry::Line { a: Vec2 { x: a[0], y: a[1] }, b: Vec2 { x: b[0], y: b[1] }, zs: None, ms: None },
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
    object(
        EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: at[0], y: at[1] },
            text: words,
            height,
            rotation: 0.0,
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

/// The views' frames: the lower left and upper right of each, in order.
fn frames(c: &Checked, at: [f64; 2], h: f64) -> Vec<[[f64; 2]; 2]> {
    let row = 2.4 * h;
    let width = c.left + c.right;
    let tall = c.span * c.exaggeration;
    let pitch_x = 8.0 * h + width + 4.0 * h;
    let pitch_y = row + tall + 3.0 * row + 4.0 * h;
    (0..c.rows.len())
        .map(|i| {
            let (col, line) = ((i % c.columns) as f64, (i / c.columns) as f64);
            let x0 = at[0] + col * pitch_x + 8.0 * h;
            let y1 = at[1] - line * pitch_y - row;
            [[x0, y1 - tall], [x0 + width, y1]]
        })
        .collect()
}

fn objects(c: &Checked, frames: &[[[f64; 2]; 2]], h: f64) -> Vec<NewObject> {
    let row = 2.4 * h;
    let grid = grid_step(0.0, c.span);
    let step = offset_step(c.left + c.right);
    let mut out = Vec::new();
    for ((r, datum), f) in c.rows.iter().zip(&c.datums).zip(frames) {
        let ([x0, y0], [x1, y1]) = (f[0], f[1]);
        let top = datum + c.span;
        let x = |o: f64| x0 + (o + c.left);
        let y = |z: f64| y0 + (z - datum) * c.exaggeration;
        out.push(path(&[[x0, y0], [x1, y0], [x1, y1], [x0, y1]], true, None, Some(0.35)));
        let mut k = (datum / grid).ceil();
        while k * grid <= top + 1e-9 {
            let v = k * grid;
            if v > datum + 1e-9 && v < top - 1e-9 {
                out.push(line([x0, y(v)], [x1, y(v)], None, Some(0.13)));
            }
            out.push(text([x0 - h, y(v)], format!("{v:.2}"), h, TextAlign::MiddleRight, None));
            k += 1.0;
        }
        let mut k = (-c.left / step).ceil();
        while k * step <= c.right + 1e-9 {
            let o = k * step + 0.0;
            if o > -c.left + 1e-9 && o < c.right - 1e-9 {
                out.push(line([x(o), y0], [x(o), y1], None, Some(0.13)));
            }
            out.push(text([x(o), y0 - 0.5 * row], format!("{o}"), h, TextAlign::MiddleCenter, None));
            k += 1.0;
        }
        out.push(line([x(0.0), y0], [x(0.0), y1], Some(AXIS), Some(0.18)));
        for run in ground_runs(r) {
            let pts: Vec<[f64; 2]> = run.iter().map(|p| [x(p[0]), y(p[1])]).collect();
            out.push(path(&pts, false, None, Some(0.25)));
        }
        let design = design_line(r);
        if !design.is_empty() {
            let pts: Vec<[f64; 2]> = design.iter().map(|p| [x(p[0]), y(p[1])]).collect();
            out.push(path(&pts, false, Some(RED), Some(0.5)));
        }
        let mid = 0.5 * (x0 + x1);
        out.push(text([mid, y1 + 0.5 * row], format!("km {}", r.km_text), h, TextAlign::MiddleCenter, None));
        if let Some(a) = r.axis {
            out.push(text([mid, y0 - 1.5 * row], format!("Eksen kotu {a:.2}"), h, TextAlign::MiddleCenter, Some(RED)));
        }
        if let (Some(cut), Some(fill)) = (r.cut, r.fill) {
            out.push(text(
                [mid, y0 - 2.5 * row],
                format!("Yarma {cut:.2} m²  Dolgu {fill:.2} m²"),
                h,
                TextAlign::MiddleCenter,
                None,
            ));
        }
    }
    out
}

fn write(cx: &mut ExecutionContext<'_>, input: &SectionsDraw, c: Checked) -> CommandResult<SectionsDrawn> {
    let h = c.height * cx.doc.settings().plot_scale / 1000.0;
    let frames = frames(&c, input.at, h);
    let objects = objects(&c, &frames, h);
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects,
        operation: Some(CreateOperation::SectionDrawing),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    let views = c
        .rows
        .iter()
        .zip(&c.datums)
        .zip(&frames)
        .map(|((r, d), f)| SectionView {
            station: r.station,
            km: r.km,
            km_text: r.km_text.clone(),
            frame: *f,
            datum: *d,
            top: d + c.span,
            cut: r.cut,
            fill: r.fill,
        })
        .collect();
    CommandResult::Completed {
        output: SectionsDrawn {
            set: c.id,
            views,
            grid: grid_step(0.0, c.span),
            offsets: offset_step(c.left + c.right),
            exaggeration: c.exaggeration,
            uids: created.created,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// The stations drawn, a view's pitch across and down, and the drawing's warnings.
pub(crate) type Pitch = (Vec<f64>, [f64; 2], Vec<CommandWarning>);

/// A set's views as section sheets lay them (docs/adr/0908 §11): the stations drawn, in order, and a view's pitch
/// across and down in drawing units (its column of elevations, the frame and the gap; the km row, the frame, the rows
/// under it and the gap).
pub(crate) fn pitch(doc: &kentos_domain::Document, input: &SectionsDraw) -> Result<Pitch, Stop> {
    let c = check(doc, input)?;
    let h = c.height * doc.settings().plot_scale / 1000.0;
    let row = 2.4 * h;
    let across = 8.0 * h + c.left + c.right + 4.0 * h;
    let down = row + c.span * c.exaggeration + 3.0 * row + 4.0 * h;
    Ok((c.rows.iter().map(|r| r.station).collect(), [across, down], c.warnings))
}

/// The views of `input` as a section sheet draws them: their objects, what `road.sections.draw` would answer (no
/// objects written yet) and its warnings.
pub(crate) fn view(
    doc: &kentos_domain::Document,
    input: &SectionsDraw,
) -> Result<(Vec<NewObject>, SectionsDrawn, Vec<CommandWarning>), Stop> {
    let c = check(doc, input)?;
    let h = c.height * doc.settings().plot_scale / 1000.0;
    let frames = frames(&c, input.at, h);
    let objects = objects(&c, &frames, h);
    let views = c
        .rows
        .iter()
        .zip(&c.datums)
        .zip(&frames)
        .map(|((r, d), f)| SectionView {
            station: r.station,
            km: r.km,
            km_text: r.km_text.clone(),
            frame: *f,
            datum: *d,
            top: d + c.span,
            cut: r.cut,
            fill: r.fill,
        })
        .collect();
    let drawn = SectionsDrawn {
        set: c.id.clone(),
        views,
        grid: grid_step(0.0, c.span),
        offsets: offset_step(c.left + c.right),
        exaggeration: c.exaggeration,
        uids: Vec::new(),
        revision: doc.revision().to_string(),
    };
    Ok((objects, drawn, c.warnings))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &SectionsDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &SectionsDraw) -> CommandResult<SectionsDrawn> {
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
pub fn execute(cx: &mut ExecutionContext<'_>, input: SectionsDraw) -> CommandResult<SectionsDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_SECTIONS_DRAW,
        kentos_commands::ROAD_SECTIONS_DRAW_VERSION,
        input,
        |cx, input| {
            let r = match check(cx.doc, &input) {
                Ok(c) => write(cx, &input, c),
                Err(stop) => stop.into(),
            };
            crate::road_drawings::kept(cx, kentos_commands::ROAD_SECTIONS_DRAW, &input, (kentos_commands::RoadSourceKind::Sections, "set"), r, |o| &o.uids)
        },
    )
}
