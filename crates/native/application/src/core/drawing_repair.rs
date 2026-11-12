//! `cad.drawing.repair` v1 (Onar, docs/adr/1005 §2, docs/adr/1012): reviewed health findings fixed, in this order:
//! - a T junction: the line's end moved along the line onto the inside of the target's nearest straight edge, by at
//!   most `maxGap` (the inspection's own threshold, so a fix never moves more than the finding said);
//! - an angle deviation: the line turned about its pivot end onto the nearest axis, its length kept, when it is off
//!   by at most `maxAngle`;
//! - duplicate block definitions: the inserts of the merged ones (in the drawing and in other definitions) take the
//!   kept one, and the merged ones go; only definitions of one content (the health check's key) merge.
//! A line that a constraint or an associative dimension reads is refused: measured geometry is not repaired by a
//! general routine. All or nothing; one undo step “Onar”; what moved and the recipe applied are reported. The shared
//! cases are `fixtures/commands/v1/cad.drawing.repair.json` (`scripts/fixtures/drawing_repair_cases.py`).

use std::collections::BTreeSet;

use kentos_commands::{
    BlockDefinition, BlockId, BlocksMerged, CommandResult, DRAWING_REPAIR_STEP, DrawingRepair, DrawingRepaired,
    EdgeStraightened, Entity, JunctionFixed, LineEnd, RepairRecipe, RepairStep, Vec2,
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

/// What a repair writes and says.
struct Planned {
    /// Lines moved or turned, inserts relinked.
    writes: Vec<(Slot, Entity)>,
    /// Definitions whose inserts were relinked.
    holders: Vec<BlockDefinition>,
    /// Definitions merged away.
    removed: Vec<BlockId>,
    output: DrawingRepaired,
}

/// The line `uid` of a fix, its slot; why not.
fn line_of<'a>(doc: &'a Document, text: &str, at: &str) -> Result<(Slot, &'a kentos_commands::LineEntity), Stop> {
    let (slot, e) = Uuid::parse_str(text)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| Some((s, doc.get(s)?)))
        .ok_or_else(|| {
            fail(codes::ENTITY_NOT_FOUND, format!("“{text}” kimlikli nesne çizimde yok; denetimi yenileyin."), at.into())
        })?;
    let Entity::Line(line) = e else {
        return Err(fail(
            codes::REPAIR_UNSUPPORTED,
            "Onarım yalnız çizgi ucunu taşır; bu nesne çizgi değil.".into(),
            at.into(),
        ));
    };
    let layers = doc.layers();
    if layers.is_locked(&e.base().layer_id) {
        let name = layers.get(&e.base().layer_id).map_or(e.base().layer_id.clone(), |n| n.name.clone());
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{name}” katmanı kilitli; çizgi onarılamaz, hiçbir şey yazılmadı. Kilidi Katmanlar panelinden açın."),
            at.into(),
        ));
    }
    if measured(doc, text) {
        return Err(fail(
            codes::REPAIR_REFUSED_MEASURED,
            "Çizgi bir kısıtın ya da ilişkili ölçünün kaynağı; ölçülmüş geometri onarımla taşınmaz. Ucu kısıt ya da parametreyle düzeltin.".into(),
            at.into(),
        ));
    }
    Ok((slot, line))
}

/// The nearest axis direction to `d` (a unit vector): exact, so a straightened line is on it to the bit.
fn axis(angle: f64) -> (f64, Vec2) {
    let quarter = std::f64::consts::FRAC_PI_2;
    let k = (angle / quarter).round();
    let dir = match (k as i64).rem_euclid(4) {
        0 => Vec2 { x: 1.0, y: 0.0 },
        1 => Vec2 { x: 0.0, y: 1.0 },
        2 => Vec2 { x: -1.0, y: 0.0 },
        _ => Vec2 { x: 0.0, y: -1.0 },
    };
    ((angle - k * quarter).abs(), dir)
}

/// The writes, each fix's report; why not.
fn check(doc: &Document, input: &DrawingRepair) -> Result<Planned, Stop> {
    let total = input.fixes.len() + input.angles.len() + input.blocks.len();
    if total == 0 || total > 1000 {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Onarım 1 ile 1000 düzeltme alır; {total} verildi."),
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
    let mut turned = BTreeSet::new();
    for (i, f) in input.angles.iter().enumerate() {
        if !checks::is_uid_text(&f.uid) {
            return Err(fail(
                codes::INVALID_UID,
                format!("“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği denetimin bulgusundan alın.", f.uid),
                format!("angles[{i}].uid"),
            ));
        }
        if !turned.insert(f.uid.as_str()) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{}” çizgisi iki kez döndürülemez.", f.uid),
                format!("angles[{i}]"),
            ));
        }
    }
    let mut named = BTreeSet::new();
    for (i, m) in input.blocks.iter().enumerate() {
        if m.merge.is_empty() || m.merge.len() > 100 {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Bir birleştirme 1 ile 100 tanım alır; {} verildi.", m.merge.len()),
                format!("blocks[{i}].merge"),
            ));
        }
        for (name, at) in std::iter::once((&m.keep, format!("blocks[{i}].keep")))
            .chain(m.merge.iter().enumerate().map(|(j, n)| (n, format!("blocks[{i}].merge[{j}]"))))
        {
            if !named.insert(name.as_str()) {
                return Err(fail(
                    codes::INVALID_INPUT,
                    format!("“{name}” tanımı iki kez verildi; bir tanım bir birleştirmede bir kez yer alır."),
                    at,
                ));
            }
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
    let max_angle = input.max_angle.unwrap_or(0.5);
    if !max_angle.is_finite() || max_angle <= 0.0 || max_angle > 10.0 {
        return Err(fail(
            codes::INVALID_ANGLE_TOLERANCE,
            "Açı eşiği 0'dan büyük, en çok 10 derece olmalı.".into(),
            "maxAngle".into(),
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
        // The order of the refusals is the contract's: both ids found, then what kind of object.
        find(&f.uid, "uid")?;
        let (_, target) = find(&f.target, "target")?;
        if !matches!(doc.slot_of(Uuid::parse_str(&f.uid).expect("checked")).and_then(|s| doc.get(s)), Some(Entity::Line(_))) {
            return Err(fail(
                codes::REPAIR_UNSUPPORTED,
                "Onarım yalnız çizgi ucunu taşır; bu nesne çizgi değil.".into(),
                format!("{at}.uid"),
            ));
        }
        if f.uid == f.target {
            return Err(fail(
                codes::REPAIR_UNSUPPORTED,
                "Çizgi kendi kenarına getirilemez; başka bir hedef verin.".into(),
                format!("{at}.target"),
            ));
        }
        let (slot, line) = line_of(doc, &f.uid, &format!("{at}.uid"))?;
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
    let mut steps: Vec<RepairStep> = input
        .fixes
        .iter()
        .map(|f| RepairStep::Junction { uid: f.uid.clone(), end: f.end, target: f.target.clone() })
        .collect();
    // Angles, after the junctions: a line both moved and turned turns as it was moved.
    let mut straightened = Vec::new();
    let tolerance = max_angle.to_radians();
    for (i, f) in input.angles.iter().enumerate() {
        let at = format!("angles[{i}].uid");
        let (slot, line) = line_of(doc, &f.uid, &at)?;
        let current = lines.get(&slot).cloned().unwrap_or_else(|| line.clone());
        let (pivot, other) = match f.pivot {
            LineEnd::Start => (current.a, current.b),
            LineEnd::End => (current.b, current.a),
        };
        let length = len(pivot, other);
        if length <= NEAR {
            return Err(fail(codes::REPAIR_UNSUPPORTED, "Boyu sıfır çizginin doğrultusu yok; döndürülemez.".into(), at));
        }
        let (deviation, dir) = axis((other.y - pivot.y).atan2(other.x - pivot.x));
        if deviation > tolerance {
            return Err(fail(
                codes::REPAIR_ANGLE_TOO_LARGE,
                format!(
                    "Çizgi eksenden {:.4}° sapıyor; eşik {max_angle}°. Bilerek eğikse bırakın, değilse eşiği büyütün ya da Döndür'le düzeltin.",
                    deviation.to_degrees()
                ),
                at,
            ));
        }
        let to = Vec2 { x: pivot.x + length * dir.x, y: pivot.y + length * dir.y };
        let mut next = current.clone();
        match f.pivot {
            LineEnd::Start => next.b = to,
            LineEnd::End => next.a = to,
        }
        straightened.push(EdgeStraightened {
            uid: f.uid.clone(),
            pivot: f.pivot,
            deviation: deviation.to_degrees(),
            moved: len(other, to),
        });
        steps.push(RepairStep::Angle { uid: f.uid.clone(), pivot: f.pivot });
        lines.insert(slot, next);
    }
    let mut writes: Vec<(Slot, Entity)> = lines.into_iter().map(|(s, l)| (s, Entity::Line(l))).collect();
    // Block merges.
    let mut relink: std::collections::BTreeMap<BlockId, BlockId> = std::collections::BTreeMap::new();
    let mut merged = Vec::new();
    for (i, m) in input.blocks.iter().enumerate() {
        let find = |name: &str, at: String| -> Result<&BlockDefinition, Stop> {
            let b = doc.block_named(name).ok_or_else(|| {
                fail(codes::UNKNOWN_BLOCK, format!("“{name}” adlı blok tanımı çizimde yok; denetimi yenileyin."), at.clone())
            })?;
            if b.xref.is_some() {
                return Err(fail(
                    codes::REPAIR_UNSUPPORTED,
                    format!("“{name}” bir dış başvurunun tanımı; içeriği kaynağındadır, burada birleştirilmez."),
                    at,
                ));
            }
            Ok(b)
        };
        let keep = find(&m.keep, format!("blocks[{i}].keep"))?;
        let key = crate::drawing_inspect::health::content_key(keep);
        for (j, name) in m.merge.iter().enumerate() {
            let at = format!("blocks[{i}].merge[{j}]");
            let b = find(name, at.clone())?;
            if crate::drawing_inspect::health::content_key(b) != key {
                return Err(fail(
                    codes::REPAIR_BLOCKS_DIFFER,
                    format!("“{name}” ile “{}” tanımlarının içeriği aynı değil; biri öbürünün yerine geçemez.", m.keep),
                    at,
                ));
            }
            if doc.arrays().iter().any(|a| a.block == b.id) {
                return Err(fail(
                    codes::REPAIR_UNSUPPORTED,
                    format!("“{name}” bir ilişkili dizinin bloğu; dizi kendi tanımına bağlı kalır, birleştirilmez."),
                    at,
                ));
            }
            relink.insert(b.id, keep.id);
        }
        merged.push((i, m.keep.clone(), m.merge.clone()));
        steps.push(RepairStep::Blocks { keep: m.keep.clone(), merge: m.merge.clone() });
    }
    let mut counts = vec![0u32; input.blocks.len()];
    let which = |id: BlockId| input.blocks.iter().position(|m| m.merge.iter().any(|n| doc.block_named(n).is_some_and(|b| b.id == id)));
    let mut holders = Vec::new();
    if !relink.is_empty() {
        for (uid, e) in doc.entities_with_uids() {
            if let Entity::Insert(ins) = e
                && let Some(&to) = relink.get(&ins.block)
                && let Some(slot) = doc.slot_of(uid)
            {
                if let Some(k) = which(ins.block) {
                    counts[k] += 1;
                }
                let mut next = ins.clone();
                next.block = to;
                writes.push((slot, Entity::Insert(next)));
            }
        }
        for b in doc.blocks().iter().filter(|b| !relink.contains_key(&b.id)) {
            let mut next = (**b).clone();
            let mut changed = false;
            for e in &mut next.entities {
                if let Entity::Insert(ins) = e
                    && let Some(&to) = relink.get(&ins.block)
                {
                    if let Some(k) = which(ins.block) {
                        counts[k] += 1;
                    }
                    ins.block = to;
                    changed = true;
                }
            }
            if changed {
                holders.push(next);
            }
        }
    }
    let merged: Vec<BlocksMerged> = merged
        .into_iter()
        .map(|(i, keep, names)| BlocksMerged { keep, merged: names, inserts: counts[i] })
        .collect();
    let max_moved = fixed.iter().map(|f| f.moved).fold(0.0, f64::max);
    let recipe = RepairRecipe {
        steps,
        max_gap: gap,
        max_angle,
        objects: (writes.len() + holders.len()) as u32,
        definitions_removed: relink.len() as u32,
    };
    let output = DrawingRepaired { fixed, straightened, merged, recipe, max_moved, revision: doc.revision().to_string() };
    Ok(Planned { writes, holders, removed: relink.into_keys().collect(), output })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &DrawingRepair) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &DrawingRepair) -> CommandResult<DrawingRepaired> {
    match check(cx.doc, input) {
        Ok(planned) => CommandResult::Completed { output: planned.output, warnings: Vec::new() },
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
            let planned = match check(cx.doc, &input) {
                Ok(done) => done,
                Err(stop) => return stop.into(),
            };
            let Planned { writes, holders, removed, mut output } = planned;
            cx.doc.name_step(DRAWING_REPAIR_STEP);
            // One step: the lines and inserts, the definitions holding relinked inserts, the merged ones removed. A
            // definition the rules will not let go takes all of it back.
            let done = cx.doc.transact(DRAWING_REPAIR_STEP, |d| {
                d.update_many(writes, DRAWING_REPAIR_STEP);
                for b in holders {
                    d.update_block(b).map_err(|r| r.0)?;
                }
                for id in removed {
                    d.remove_block(id).map_err(|r| r.0)?;
                }
                Ok::<(), String>(())
            });
            if let Err(reason) = done {
                return Stop::failed(error(codes::REPAIR_UNSUPPORTED, reason, Some("blocks".into()))).into();
            }
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings: Vec::new() }
        },
    )
}
