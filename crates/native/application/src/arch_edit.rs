//! `cad.arch.edit` v1 (docs/adr/0343 §4, plan/cad.md `CAD-ARCH-31`–`36`): grids, columns, stairs and ramps named by
//! their persistent ids changed as one undo step “Mimari nesneyi düzenle”, all or none. Değiştir sets their own fields
//! (Öznitelikler writes through it); Numarala labels grids or tags columns in the order given; Akslara bağla ties
//! columns to the crossing of two grids; Bağı kopar frees them where they stand. The shared cases are
//! `fixtures/commands/v1/cad.arch.edit.json`.

use kentos_commands::{
    ArchChange, ArchEdit, ArchEdited, ArchOperation, CommandResult, CommandWarning, Entity,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::arch_elements::{
    self, column_valid, fail, grid_valid, ramp_valid, settled_on, stair_valid,
};
use crate::checks::{self, Stop};
use crate::codes;

pub const STEP: &str = "Mimari nesneyi düzenle";

/// The kind's name in a sentence.
fn kind_name(e: &Entity) -> &'static str {
    match e {
        Entity::Grid(_) => "aks",
        Entity::Column(_) => "kolon",
        Entity::Stair(_) => "merdiven",
        Entity::Ramp(_) => "rampa",
        _ => "nesne",
    }
}

fn not_for(field: &str, e: &Entity) -> Stop {
    fail(
        codes::FIELD_NOT_FOR_KIND,
        format!("“{field}” alanı {} nesnesinde yok.", kind_name(e)),
        &format!("operation.{field}"),
    )
}

/// The fields `c` gives, by their names on the wire, with the kinds that have each.
fn given(c: &ArchChange) -> Vec<(&'static str, &'static [&'static str])> {
    const GRID: &[&str] = &["grid"];
    const COLUMN: &[&str] = &["column"];
    const STAIR: &[&str] = &["stair"];
    const RAMP: &[&str] = &["ramp"];
    const WIDE: &[&str] = &["column", "stair", "ramp"];
    const FLIGHT: &[&str] = &["stair", "ramp"];
    const ALL: &[&str] = &["grid", "column", "stair", "ramp"];
    let mut out = Vec::new();
    let mut add = |on: bool, name: &'static str, kinds: &'static [&'static str]| {
        if on {
            out.push((name, kinds));
        }
    };
    add(c.label.is_some(), "label", GRID);
    add(c.bubbles.is_some(), "bubbles", GRID);
    add(c.bubble_size.is_some(), "bubbleSize", GRID);
    add(c.shape.is_some(), "shape", COLUMN);
    add(c.width.is_some(), "width", WIDE);
    add(c.depth.is_some(), "depth", COLUMN);
    add(c.rotation.is_some(), "rotation", COLUMN);
    add(c.kind.is_some(), "type", COLUMN);
    add(c.tag.is_some(), "tag", COLUMN);
    add(c.fill.is_some(), "fill", COLUMN);
    add(c.risers.is_some(), "risers", STAIR);
    add(c.riser_height.is_some(), "riserHeight", STAIR);
    add(c.tread.is_some(), "tread", STAIR);
    add(c.landing.is_some(), "landing", FLIGHT);
    add(c.cut.is_some(), "cut", STAIR);
    add(c.numbering.is_some(), "numbering", STAIR);
    add(c.railings.is_some(), "railings", FLIGHT);
    add(c.rise.is_some(), "rise", RAMP);
    add(c.slope.is_some(), "slope", RAMP);
    add(c.base.is_some(), "base", RAMP);
    add(c.base_offset.is_some(), "baseOffset", RAMP);
    add(c.level.is_some(), "level", ALL);
    add(c.phase.is_some(), "phase", ALL);
    out
}

fn tag_of(e: &Entity) -> &'static str {
    match e {
        Entity::Grid(_) => "grid",
        Entity::Column(_) => "column",
        Entity::Stair(_) => "stair",
        Entity::Ramp(_) => "ramp",
        _ => "",
    }
}

fn numbers(c: &ArchChange) -> Result<(), Stop> {
    let flat = |v: Option<Option<f64>>| v.flatten();
    for (v, field, name) in [
        (flat(c.bubble_size), "bubbleSize", "Balon çapı"),
        (c.width, "width", "Genişlik"),
        (flat(c.depth), "depth", "Boy"),
        (c.rotation, "rotation", "Dönüş"),
        (c.riser_height, "riserHeight", "Rıht yüksekliği"),
        (c.tread, "tread", "Basamak derinliği"),
        (flat(c.landing), "landing", "Sahanlık"),
        (c.rise, "rise", "Yükseklik"),
        (flat(c.slope), "slope", "Eğim"),
        (flat(c.base), "base", "Başlangıç kotu"),
        (flat(c.base_offset), "baseOffset", "Kata göre başlangıç kotu"),
    ] {
        if let Some(x) = v {
            checks::finite(
                x,
                name,
                "Sonlu bir sayı verin.",
                &format!("operation.{field}"),
            )?;
        }
    }
    Ok(())
}

/// `e` with the fields `c` gives, as yet unchecked.
fn changed(e: &Entity, c: &ArchChange) -> Entity {
    let mut e = e.clone();
    match &mut e {
        Entity::Grid(g) => {
            let f = &mut g.grid;
            if let Some(v) = &c.label {
                f.label.clone_from(v);
            }
            if let Some(v) = c.bubbles {
                f.bubbles = v;
            }
            if let Some(v) = c.bubble_size {
                f.bubble_size = v;
            }
        }
        Entity::Column(col) => {
            let f = &mut col.column;
            if let Some(v) = c.shape {
                f.shape = v;
            }
            if let Some(v) = c.width {
                f.width = v;
            }
            if let Some(v) = c.depth {
                f.depth = v;
            }
            if let Some(v) = c.rotation {
                f.rotation = v;
            }
            if let Some(v) = &c.kind {
                f.kind.clone_from(v);
            }
            if let Some(v) = &c.tag {
                f.tag.clone_from(v);
            }
            if let Some(v) = c.fill {
                f.fill = v;
            }
        }
        Entity::Stair(s) => {
            let f = &mut s.stair;
            if let Some(v) = c.width {
                f.width = v;
            }
            if let Some(v) = c.risers {
                f.risers = v;
            }
            if let Some(v) = c.riser_height {
                f.riser_height = v;
            }
            if let Some(v) = c.tread {
                f.tread = v;
            }
            if let Some(v) = c.landing {
                f.landing = v;
            }
            if let Some(v) = c.cut {
                f.cut = v;
            }
            if let Some(v) = c.numbering {
                f.numbering = v;
            }
            if let Some(v) = c.railings {
                f.railings = v;
            }
        }
        Entity::Ramp(r) => {
            let f = &mut r.ramp;
            if let Some(v) = c.width {
                f.width = v;
            }
            if let Some(v) = c.rise {
                f.rise = v;
            }
            if let Some(v) = c.slope {
                f.slope = v;
            }
            if let Some(v) = c.base {
                // A base of its own frees the ramp from its level (docs/adr/0723 §2).
                f.base = v;
                f.base_offset = None;
            }
            if let Some(v) = c.base_offset {
                // Freed from its level, its base stays where it is.
                f.base_offset = v;
            }
            if let Some(v) = c.landing {
                f.landing = v;
            }
            if let Some(v) = c.railings {
                f.railings = v;
            }
        }
        _ => {}
    }
    if let Some(arch) = arch_mut(&mut e) {
        if let Some(v) = &c.level {
            arch.level.clone_from(v);
        }
        if let Some(v) = c.phase {
            arch.phase = v;
        }
    }
    e
}

fn arch_mut(e: &mut Entity) -> Option<&mut kentos_commands::ArchInfo> {
    match e {
        Entity::Grid(g) => Some(&mut g.grid.arch),
        Entity::Column(c) => Some(&mut c.column.arch),
        Entity::Stair(s) => Some(&mut s.stair.arch),
        Entity::Ramp(r) => Some(&mut r.ramp.arch),
        _ => None,
    }
}

/// `e` checked at `path`, a column settled on its grids, with the warnings a stair or a ramp gives.
fn settled(doc: &Document, e: Entity, path: &str) -> Result<(Entity, Vec<CommandWarning>), Stop> {
    let mut warnings = Vec::new();
    let e = match e {
        Entity::Grid(g) => {
            grid_valid(&g.grid, path)?;
            Entity::Grid(g)
        }
        Entity::Column(mut c) => {
            column_valid(&c.column, path)?;
            c.column = settled_on(doc, c.column, true, path)?;
            Entity::Column(c)
        }
        Entity::Stair(s) => {
            stair_valid(&s.stair, path)?;
            warnings = arch_elements::stair_warnings(doc, &s.stair, path);
            Entity::Stair(s)
        }
        Entity::Ramp(r) => {
            ramp_valid(&r.ramp, path)?;
            Entity::Ramp(r)
        }
        other => other,
    };
    if let Some(arch) = arch_elements::arch_of(&e) {
        crate::walls::level_known(doc, arch, "operation.level")?;
    }
    let e = match e {
        // Its base read from its level, now known (docs/adr/0723 §2), before its warnings.
        Entity::Ramp(mut r) => {
            arch_elements::level_bound(doc, &mut r.ramp);
            warnings = arch_elements::ramp_warnings(&r.ramp, path);
            Entity::Ramp(r)
        }
        other => other,
    };
    Ok((e, warnings))
}

type Work = (Vec<CommandWarning>, Vec<(Slot, Entity)>);

fn check(doc: &Document, input: &ArchEdit) -> Result<Work, Stop> {
    checks::uids(
        &input.uids,
        "Düzenlenecek aks, kolon, merdiven ya da rampa verilmedi.",
    )?;
    let found = checks::named(doc, &input.uids)?;
    for (at, _, e, _) in &found {
        if arch_elements::arch_of(e).is_none() {
            return Err(fail(
                codes::NOT_ARCH_ELEMENT,
                "Nesne aks, kolon, merdiven ya da rampa değil; Mimari nesneyi düzenle yalnız bunları değiştirir."
                    .into(),
                &format!("uids[{at}]"),
            ));
        }
    }
    let mut warnings = Vec::new();
    let mut out = Vec::with_capacity(found.len());
    match &input.operation {
        ArchOperation::Set(c) => {
            let fields = given(c);
            if fields.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Değiştirilecek bir alan verilmedi.".into(),
                    "operation",
                ));
            }
            for (_, _, e, _) in &found {
                if let Some((name, _)) =
                    fields.iter().find(|(_, kinds)| !kinds.contains(&tag_of(e)))
                {
                    return Err(not_for(name, e));
                }
            }
            numbers(c)?;
            if c.base.is_some() && c.base_offset.is_some() {
                return Err(fail(
                    codes::INVALID_RAMP,
                    "Başlangıç kotu ya kendisi (base) ya da kata göre (baseOffset) verilir; ikisi birden verilmez."
                        .into(),
                    "operation.baseOffset",
                ));
            }
            for (at, slot, e, _) in &found {
                let (e, mut w) = settled(doc, changed(e, c), &format!("uids[{at}]"))?;
                warnings.append(&mut w);
                out.push((*slot, e));
            }
        }
        ArchOperation::Renumber { start } => {
            let first = tag_of(found[0].2);
            if !matches!(first, "grid" | "column")
                || found.iter().any(|(_, _, e, _)| tag_of(e) != first)
            {
                return Err(fail(
                    codes::MIXED_KINDS,
                    "Numarala yalnız aksları ya da yalnız kolonları numaralar; ikisi birlikte ya da başka tür verilmez."
                        .into(),
                    "uids",
                ));
            }
            let mut label = start.clone();
            for (k, (at, slot, e, _)) in found.iter().enumerate() {
                if k > 0 {
                    label = kentos_commands::next_label(&label);
                }
                let mut e = (*e).clone();
                match &mut e {
                    Entity::Grid(g) => g.grid.label.clone_from(&label),
                    Entity::Column(c) => c.column.tag = Some(label.clone()),
                    _ => {}
                }
                let path = format!("uids[{at}]");
                match &e {
                    Entity::Grid(g) => grid_valid(&g.grid, &path)?,
                    Entity::Column(c) => column_valid(&c.column, &path)?,
                    _ => {}
                }
                out.push((*slot, e));
            }
        }
        ArchOperation::Attach { grids } => {
            for (_, _, e, _) in &found {
                if !matches!(e, Entity::Column(_)) {
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!("Akslara yalnız kolon bağlanır; {} bağlanmaz.", kind_name(e)),
                        "operation.grids",
                    ));
                }
            }
            for (_, slot, e, _) in &found {
                let Entity::Column(c) = e else { continue };
                let mut c = c.clone();
                c.column.grids = Some(grids.clone());
                c.column.place = None;
                c.column.broken = None;
                c.column = settled_on(doc, c.column, true, "operation")?;
                out.push((*slot, Entity::Column(c)));
            }
        }
        ArchOperation::Distance { side, distance } => {
            // docs/adr/0712: one column, not tied to grids, moved square to a side.
            let [(at, slot, e, _)] = found.as_slice() else {
                return Err(fail(
                    codes::INVALID_COLUMN,
                    "Mesafe tek kolonla çalışır; bir kolon verin.".into(),
                    "uids",
                ));
            };
            let Entity::Column(c) = e else {
                return Err(fail(
                    codes::NOT_A_COLUMN,
                    format!("Mesafe yalnız kolonu kaydırır; {} kaydırılmaz.", kind_name(e)),
                    &format!("uids[{at}]"),
                ));
            };
            checks::finite(
                *distance,
                "Mesafe",
                "Sonlu bir uzunluk verin.",
                "operation.distance",
            )?;
            if *distance < 0.0 {
                return Err(fail(
                    codes::INVALID_COLUMN,
                    format!("Mesafe {distance}; sıfır ya da daha büyük olmalı."),
                    "operation.distance",
                ));
            }
            if c.column.grids.is_some() {
                let place = c
                    .column
                    .place
                    .as_deref()
                    .map(|p| format!(" ({p})"))
                    .unwrap_or_default();
                return Err(fail(
                    codes::COLUMN_ON_GRIDS,
                    format!(
                        "Kolon akslara bağlı{place}; yeri aksların kesişimidir. Kaydırmak için önce Bağı kopar'ı seçin ya da aksı taşıyın."
                    ),
                    &format!("uids[{at}]"),
                ));
            }
            let around = crate::column_distances::Around::of(doc, *slot);
            let (walls, columns) = (around.walls(), around.columns());
            let to = kentos_geometry_core::ops::column_shift::moved(
                &walls,
                &columns,
                0,
                *side as usize,
                *distance,
            )
            .map_err(|why| fail(codes::INVALID_COLUMN, why, "operation.side"))?;
            let from = c.column.at;
            if (to.x - from.x).hypot(to.y - from.y) <= kentos_geometry_core::ops::wall_shift::TIE {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Kolon zaten bu konumda; kaydırılacak bir şey yok.".into(),
                    "operation",
                ));
            }
            let mut c = c.clone();
            c.column.at = kentos_commands::Vec2 { x: to.x, y: to.y };
            column_valid(&c.column, &format!("uids[{at}]"))?;
            out.push((*slot, Entity::Column(c)));
        }
        ArchOperation::Detach => {
            for (_, slot, e, _) in &found {
                let Entity::Column(c) = e else {
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!(
                            "Bağı yalnız kolonun kopar; {} bağlı değildir.",
                            kind_name(e)
                        ),
                        "operation",
                    ));
                };
                let mut c = c.clone();
                c.column.grids = None;
                c.column.place = None;
                c.column.broken = None;
                out.push((*slot, Entity::Column(c)));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc
                .layers()
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki {} düzenlenemez. Kilidi Katmanlar panelinden açın.",
                    kind_name(e)
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    Ok((warnings, out))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ArchEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((warnings, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ArchEdit) -> CommandResult<ArchEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the tools and Öznitelikler preview.
pub fn plan_in(doc: &Document, input: &ArchEdit) -> CommandResult<ArchEdited> {
    match check(doc, input) {
        Ok((warnings, work)) => CommandResult::Completed {
            output: ArchEdited {
                elements: work.into_iter().map(|(_, e)| e).collect(),
                revision: doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ArchEdit) -> CommandResult<ArchEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ARCH_EDIT,
        kentos_commands::CAD_ARCH_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ArchEdit) -> CommandResult<ArchEdited> {
    let (warnings, work) = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let slots: Vec<Slot> = work.iter().map(|(s, _)| *s).collect();
    cx.doc.update_many(work, STEP);
    // As written: a column follows its grids in the step.
    let elements = slots
        .iter()
        .filter_map(|s| cx.doc.get(*s).cloned())
        .collect();
    CommandResult::Completed {
        output: ArchEdited {
            elements,
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}
