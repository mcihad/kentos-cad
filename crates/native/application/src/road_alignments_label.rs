//! `road.alignments.label` v1 (plan/road.md Y3, docs/adr/0901): an alignment's km written on a layer as lines and
//! texts, one undo step “Aks km yazısı” (through `cad.entities.create`'s write), so it goes to sheets and DXF as
//! Netcad's and Civil 3D's station labels do. A tick crosses the alignment at every round `every` metres of km (the km
//! read through its equations), a longer one at every `major` with its km written beside it on the right; the start's
//! and the end's ticks carry their km to the millimetre; the curves' TS, SC, CS, ST (TC, CT without clothoids) are
//! written on the left with their km. Lengths on paper: a tick 1.5 mm each side, a major one 3 mm, texts 1 mm off.
//! The shared cases are `fixtures/commands/v1/road.alignments.label.json`.

use kentos_commands::{
    AlignmentLabelRow, AlignmentsLabel, AlignmentsLabelled, CommandResult, CommandWarning, CreateOperation,
    EntitiesCreate, EntityGeometry, NewObject, TextAlign, TextFace, Vec2,
};
use kentos_road::alignment::km_text;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments::{self as common, Built};
use crate::road_terrains::fail;

/// The most ticks one label writes.
const MOST: usize = 100_000;

struct Checked {
    id: String,
    built: Built,
    every: f64,
    major: f64,
    height: f64,
    curves: bool,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &kentos_domain::Document, input: &AlignmentsLabel) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = common::alignments(doc);
    let (_, def) = common::find(own, &input.alignment, "alignment")?;
    let every = input.every.unwrap_or(20.0);
    let major = input.major.unwrap_or(100.0);
    let height = input.height.unwrap_or(2.5);
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !positive(every) {
        return Err(fail(codes::INVALID_LABEL, "İşaret aralığı (every) 0'dan büyük olmalı.".into(), "every"));
    }
    let ratio = major / every;
    if !positive(major) || (ratio - ratio.round()).abs() > 1e-9 || ratio.round() < 1.0 {
        return Err(fail(
            codes::INVALID_LABEL,
            "Km yazılan aralık (major) işaret aralığının tam katı olmalı.".into(),
            "major",
        ));
    }
    if !positive(height) {
        return Err(fail(codes::INVALID_LABEL, "Yazı yüksekliği (height) 0'dan büyük olmalı.".into(), "height"));
    }
    let built = common::build(own, def, "")?;
    if built.core.length / every > MOST as f64 {
        return Err(fail(
            codes::INVALID_LABEL,
            format!("Bu aralıkla {MOST}'den çok işaret çıkar; every'yi büyütün."),
            "every",
        ));
    }
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked {
        id: def.id.clone(),
        built,
        every,
        major,
        height,
        curves: input.curves.unwrap_or(true),
        warnings,
    })
}

/// The labels in their order along the alignment.
fn labels(c: &Checked) -> Vec<AlignmentLabelRow> {
    let b = &c.built;
    let length = b.core.length;
    let near = |a: f64, z: f64| (a - z).abs() <= 1e-6;
    let mut out = vec![AlignmentLabelRow {
        kind: "start".into(),
        station: 0.0,
        km: b.km(0.0),
        text: Some(km_text(b.km(0.0), 3)),
    }];
    let last = b.regions.len().saturating_sub(1);
    for (i, r) in b.regions.iter().enumerate() {
        let (a, z) = (r.ahead, r.ahead + r.dir * (r.until - r.at));
        let (lo, hi) = (a.min(z), a.max(z));
        let mut n = (lo / c.every).ceil();
        let mut here = Vec::new();
        while n * c.every <= hi + 1e-9 {
            let k = n * c.every;
            let s = r.at + r.dir * (k - r.ahead);
            if s >= r.at - 1e-9 && (s < r.until - 1e-9 || (i == last && s <= r.until + 1e-9)) {
                here.push((s.clamp(r.at, r.until), k));
            }
            n += 1.0;
        }
        here.sort_by(|x, y| x.0.total_cmp(&y.0));
        for (s, k) in here {
            if near(s, 0.0) || near(s, length) {
                continue;
            }
            let q = k / c.major;
            let major = (q - q.round()).abs() <= 1e-9;
            out.push(AlignmentLabelRow {
                kind: if major { "major" } else { "tick" }.into(),
                station: s,
                km: k,
                text: major.then(|| km_text(k, 0)),
            });
        }
    }
    if c.curves {
        for curve in &b.core.curves {
            let names = if curve.spiral_in > 0.0 || curve.spiral_out > 0.0 {
                ["TS", "SC", "CS", "ST"]
            } else {
                ["TC", "TC", "CT", "CT"]
            };
            let mut seen: Vec<f64> = Vec::new();
            for (name, s) in names.iter().zip(curve.stations) {
                if seen.iter().any(|t| near(*t, s)) {
                    continue;
                }
                seen.push(s);
                let km = b.km(s);
                out.push(AlignmentLabelRow {
                    kind: (*name).into(),
                    station: s,
                    km,
                    text: Some(format!("{name} {}", km_text(km, 3))),
                });
            }
        }
    }
    out.push(AlignmentLabelRow {
        kind: "end".into(),
        station: length,
        km: b.km(length),
        text: Some(km_text(b.km(length), 3)),
    });
    out.sort_by(|x, y| x.station.total_cmp(&y.station));
    out
}

fn object(geometry: EntityGeometry) -> NewObject {
    NewObject {
        geometry,
        color: None,
        line_weight: None,
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

fn objects(doc: &kentos_domain::Document, c: &Checked, rows: &[AlignmentLabelRow]) -> Vec<NewObject> {
    let mm = doc.settings().plot_scale / 1000.0;
    let mut out = Vec::new();
    for r in rows {
        let Some(at) = c.built.place(r.station) else { continue };
        let curve = !matches!(r.kind.as_str(), "tick" | "major" | "start" | "end");
        let half = if r.kind == "tick" { 1.5 * mm } else { 3.0 * mm };
        // Right of the direction; curve points are written on the left.
        let side = if curve { -1.0 } else { 1.0 };
        let (nx, ny) = (side * at.direction.sin(), -side * at.direction.cos());
        out.push(object(EntityGeometry::Line {
            a: Vec2 { x: at.x - nx * half, y: at.y - ny * half },
            b: Vec2 { x: at.x + nx * half, y: at.y + ny * half },
            zs: None,
            ms: None,
        }));
        let Some(words) = &r.text else { continue };
        let mut angle = ny.atan2(nx).to_degrees();
        let mut align = TextAlign::MiddleLeft;
        // Read left to right: a text pointing left turns round and is held at its right end.
        if !(-90.0..=90.0).contains(&angle) {
            angle += if angle > 0.0 { -180.0 } else { 180.0 };
            align = TextAlign::MiddleRight;
        }
        let gap = half + mm;
        out.push(object(EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: at.x + nx * gap, y: at.y + ny * gap },
            text: words.clone(),
            height: c.height * mm,
            rotation: angle,
            align: Some(align),
            width_factor: None,
            mask: false,
            box_width: None,
            line_spacing: None,
            runs: Vec::new(),
            face: TextFace::default(),
            path: None,
        }));
    }
    out
}

fn write(cx: &mut ExecutionContext<'_>, input: &AlignmentsLabel, c: Checked) -> CommandResult<AlignmentsLabelled> {
    let rows = labels(&c);
    let objects = objects(cx.doc, &c, &rows);
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects,
        operation: Some(CreateOperation::AlignmentLabels),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: AlignmentsLabelled {
            alignment: c.id,
            labels: rows,
            uids: created.created,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AlignmentsLabel) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &AlignmentsLabel) -> CommandResult<AlignmentsLabelled> {
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

/// Writes the labels: one undo step.
/// Draws `input` now, checked and written in the step it runs in, its drawing not kept (`road.project.rebuild` keeps it,
/// docs/adr/0911 §3).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &kentos_commands::AlignmentsLabel) -> CommandResult<kentos_commands::AlignmentsLabelled> {
    write_execute(cx, input.clone())
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: AlignmentsLabel) -> CommandResult<AlignmentsLabelled> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_ALIGNMENTS_LABEL,
        kentos_commands::ROAD_ALIGNMENTS_LABEL_VERSION,
        input,
        |cx, input| {
            let r = write_execute(cx, input.clone());
            crate::road_drawings::kept(cx, kentos_commands::ROAD_ALIGNMENTS_LABEL, &input, (kentos_commands::RoadSourceKind::Alignment, "alignment"), r, |o| &o.uids)
        },
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: AlignmentsLabel) -> CommandResult<AlignmentsLabelled> {
    match check(cx.doc, &input) {
        Ok(c) => write(cx, &input, c),
        Err(stop) => stop.into(),
    }
}

