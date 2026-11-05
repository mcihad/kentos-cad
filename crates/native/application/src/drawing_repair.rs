//! `cad.drawing.repair` v1 (Onar, docs/adr/1005 §2): a reviewed T-junction finding fixed: the line's end moved along
//! the line onto the inside of the target's nearest straight edge, by at most `maxGap` (the inspection's own
//! threshold, so a fix never moves more than the finding said). A line that a constraint or an associative
//! dimension reads is refused: measured geometry is not repaired by a general routine. All or nothing; one undo step
//! “Onar”; each end's move and the line's length before and after are reported. The shared cases are
//! `fixtures/commands/v1/cad.drawing.repair.json` (`scripts/fixtures/drawing_repair_cases.py`).

use std::collections::BTreeSet;

use kentos_commands::{
    CommandResult, DRAWING_REPAIR_STEP, DrawingRepair, DrawingRepaired, Entity, JunctionFixed, LineEnd, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::shape;

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

const NEAR: f64 = 1e-9;

fn len(a: Vec2, b: Vec2) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

/// Whether a constraint or an associative dimension reads `uid`.
fn measured(doc: &Document, uid: &str) -> bool {
    let constraints = serde_json::to_string(doc.constraints()).unwrap_or_default();
    if constraints.contains(uid) {
        return true;
    }
    doc.entities().any(|e| match e {
        Entity::Dimension(d) => d.refs.as_ref().is_some_and(|r| serde_json::to_string(r).unwrap_or_default().contains(uid)),
        _ => false,
    })
}

/// The point where the line from `from` through `end` meets the inside of one of `target`'s straight edges, nearest
/// to `end` along the line and within `gap`; none when there is none.
fn meeting(target: &Entity, from: Vec2, end: Vec2, gap: f64) -> Option<Vec2> {
    let length = len(from, end);
    if length <= NEAR {
        return None;
    }
    let d = Vec2 { x: (end.x - from.x) / length, y: (end.y - from.y) / length };
    let mut best: Option<(f64, Vec2)> = None;
    for edge in kentos_geometry_core::ops::edges::entity_edges(&shape(target)) {
        let kentos_geometry_core::geom::intersect::Edge::Seg { a, b } = edge else {
            continue;
        };
        let (ux, uy) = (b.x - a.x, b.y - a.y);
        let den = d.x * uy - d.y * ux;
        let edge_len = ux.hypot(uy);
        if den.abs() <= NEAR * edge_len {
            continue;
        }
        let (wx, wy) = (a.x - end.x, a.y - end.y);
        let t = (wx * uy - wy * ux) / den;
        let u = (wx * d.y - wy * d.x) / den;
        let inside = NEAR / edge_len.max(NEAR);
        if u > inside && u < 1.0 - inside && t.abs() <= gap && -t < length && best.is_none_or(|(bt, _)| t.abs() < bt.abs()) {
            best = Some((t, Vec2 { x: end.x + t * d.x, y: end.y + t * d.y }));
        }
    }
    best.map(|(_, p)| p)
}

/// The writes, each fix's report; why not.
fn check(doc: &Document, input: &DrawingRepair) -> Result<(Vec<(Slot, Entity)>, Vec<JunctionFixed>), Stop> {
    if input.fixes.is_empty() || input.fixes.len() > 1000 {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Onarım 1 ile 1000 düzeltme alır; {} verildi.", input.fixes.len()),
            "fixes".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for (i, f) in input.fixes.iter().enumerate() {
        for (text, field) in [(&f.uid, "uid"), (&f.target, "target")] {
            if !checks::is_uid_text(text) {
                return Err(fail(
                    codes::INVALID_UID,
                    format!("“{text}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği denetimin bulgusundan alın."),
                    format!("fixes[{i}].{field}"),
                ));
            }
        }
        if !seen.insert((f.uid.as_str(), f.end == LineEnd::End)) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{}” çizgisinin aynı ucu iki kez verildi.", f.uid),
                format!("fixes[{i}]"),
            ));
        }
    }
    let gap = input.max_gap.unwrap_or(0.01);
    if !gap.is_finite() || gap <= 0.0 || gap > 1000.0 {
        return Err(fail(
            codes::INVALID_GAP,
            "Onarım eşiği 0'dan büyük, en çok 1000 metre olmalı.".into(),
            "maxGap".into(),
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut lines: std::collections::BTreeMap<Slot, kentos_commands::LineEntity> = std::collections::BTreeMap::new();
    let mut fixed = Vec::new();
    for (i, f) in input.fixes.iter().enumerate() {
        let at = format!("fixes[{i}]");
        let find = |text: &str, field: &str| -> Result<(Slot, &Entity), Stop> {
            Uuid::parse_str(text)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| Some((s, doc.get(s)?)))
                .ok_or_else(|| {
                    fail(
                        codes::ENTITY_NOT_FOUND,
                        format!("“{text}” kimlikli nesne çizimde yok; denetimi yenileyin."),
                        format!("{at}.{field}"),
                    )
                })
        };
        let (slot, e) = find(&f.uid, "uid")?;
        let (_, target) = find(&f.target, "target")?;
        let Entity::Line(line) = e else {
            return Err(fail(
                codes::REPAIR_UNSUPPORTED,
                "Onarım yalnız çizgi ucunu taşır; bu nesne çizgi değil.".into(),
                format!("{at}.uid"),
            ));
        };
        if f.uid == f.target {
            return Err(fail(
                codes::REPAIR_UNSUPPORTED,
                "Çizgi kendi kenarına getirilemez; başka bir hedef verin.".into(),
                format!("{at}.target"),
            ));
        }
        let layers = doc.layers();
        if layers.is_locked(&e.base().layer_id) {
            let name = layers.get(&e.base().layer_id).map_or(e.base().layer_id.clone(), |n| n.name.clone());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!("“{name}” katmanı kilitli; çizgi onarılamaz, hiçbir şey yazılmadı. Kilidi Katmanlar panelinden açın."),
                format!("{at}.uid"),
            ));
        }
        if measured(doc, &f.uid) {
            return Err(fail(
                codes::REPAIR_REFUSED_MEASURED,
                "Çizgi bir kısıtın ya da ilişkili ölçünün kaynağı; ölçülmüş geometri onarımla taşınmaz. Ucu kısıt ya da parametreyle düzeltin.".into(),
                format!("{at}.uid"),
            ));
        }
        let current = lines.get(&slot).cloned().unwrap_or_else(|| line.clone());
        let (from, end) = match f.end {
            LineEnd::End => (current.a, current.b),
            LineEnd::Start => (current.b, current.a),
        };
        let Some(to) = meeting(target, from, end, gap) else {
            return Err(fail(
                codes::REPAIR_TARGET_MISSING,
                format!("Çizginin ucu {gap} m içinde hedefin bir doğru kenarına değmiyor; denetimi yenileyin ya da eşiği büyütün."),
                format!("{at}.target"),
            ));
        };
        let before = len(current.a, current.b);
        let mut next = current.clone();
        match f.end {
            LineEnd::End => next.b = to,
            LineEnd::Start => next.a = to,
        }
        fixed.push(JunctionFixed {
            uid: f.uid.clone(),
            end: f.end,
            moved: len(end, to),
            length_before: before,
            length_after: len(next.a, next.b),
        });
        lines.insert(slot, next);
    }
    let writes = lines.into_iter().map(|(s, l)| (s, Entity::Line(l))).collect();
    Ok((writes, fixed))
}

fn answered(fixed: Vec<JunctionFixed>, revision: String) -> DrawingRepaired {
    let max_moved = fixed.iter().map(|f| f.moved).fold(0.0, f64::max);
    DrawingRepaired { fixed, max_moved, revision }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &DrawingRepair) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &DrawingRepair) -> CommandResult<DrawingRepaired> {
    match check(cx.doc, input) {
        Ok((_, fixed)) => CommandResult::Completed { output: answered(fixed, cx.doc.revision().to_string()), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: DrawingRepair) -> CommandResult<DrawingRepaired> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_DRAWING_REPAIR,
        kentos_commands::CAD_DRAWING_REPAIR_VERSION,
        input,
        |cx, input| {
            let (writes, fixed) = match check(cx.doc, &input) {
                Ok(done) => done,
                Err(stop) => return stop.into(),
            };
            cx.doc.name_step(DRAWING_REPAIR_STEP);
            cx.doc.update_many(writes, DRAWING_REPAIR_STEP);
            CommandResult::Completed { output: answered(fixed, cx.doc.revision().to_string()), warnings: Vec::new() }
        },
    )
}
