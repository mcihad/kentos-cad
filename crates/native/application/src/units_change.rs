//! `cad.units.change` v1 (docs/adr/0256 §1): a local project's drawing unit
//! changed, keeping the sizes (only the unit changes) or the numbers (the
//! drawing scaled by the units' ratio), as one undo step “Birimi değiştir”.
//! The desktop's handler over the native document, run by Proje ayarları
//! (its question shows [`plan`], its answer [`execute`]), the headless
//! server, Python and MCP. The shared cases are
//! `fixtures/commands/v1/cad.units.change.json`.
//!
//! What a scale multiplies is the contracts' rule (`kentos_commands::units`),
//! the one DXF's units take; the unit, the home view and the anchor are
//! written by the step's own op (`Document::set_unit_state`), so undo gives
//! them back with the objects.

use kentos_commands::units::{Scale, scale_block, scale_bounds, scale_entity};
use kentos_commands::{
    CommandError, CommandResult, CommandWarning, DrawingUnit, Entity, UnitGeometry, UnitsChange,
    UnitsChanged, Vec2,
};
use kentos_domain::{Document, Slot, UnitState};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Birimi değiştir";

/// The most objects a refusal lists by id.
const LISTED: usize = 100;

/// The drawing's unit now (none: metres).
pub fn unit_of(doc: &Document) -> DrawingUnit {
    doc.settings().drawing_unit.unwrap_or(DrawingUnit::M)
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn warning(code: &str, message: String) -> CommandWarning {
    CommandWarning {
        code: code.to_owned(),
        message,
        path: None,
    }
}

/// What the checks found: the scale (none: the sizes stay) and what it says.
struct Checked {
    scale: Option<Scale>,
    warnings: Vec<CommandWarning>,
    objects: u32,
    blocks: u32,
}

/// The objects on a locked layer (the layer or a group above it), in the
/// drawing's order; the layers by their names with how many each holds.
fn on_locked_layers(doc: &Document) -> (Vec<String>, Vec<(String, usize)>) {
    let mut uids = Vec::new();
    let mut layers: Vec<(String, String, usize)> = Vec::new();
    for e in doc.entities() {
        let layer = &e.base().layer_id;
        if !doc.layers().is_locked(layer) {
            continue;
        }
        if uids.len() < LISTED
            && let Some(uid) = doc.uid(Slot(e.base().id))
        {
            uids.push(uid.to_string());
        }
        match layers.iter_mut().find(|(id, _, _)| id == layer) {
            Some((_, _, n)) => *n += 1,
            None => {
                let name = doc
                    .layers()
                    .get(layer)
                    .map_or_else(|| layer.clone(), |n| n.name.clone());
                layers.push((layer.clone(), name, 1));
            }
        }
    }
    (
        uids,
        layers.into_iter().map(|(_, name, n)| (name, n)).collect(),
    )
}

/// The checks, in the contract's order.
fn check(doc: &Document, input: &UnitsChange) -> Result<Checked, Stop> {
    if input.from == input.to {
        return Err(refuse(
            codes::SAME_UNIT,
            format!(
                "Çizimin birimi zaten {}; değişecek bir şey yok.",
                input.to.mark()
            ),
            "to",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    if doc.settings().has_system() {
        return Err(refuse(
            codes::UNIT_NEEDS_LOCAL,
            "Koordinat sistemi olan projenin birimi metredir; mm ya da cm yalnız sistemsiz yerel projede seçilir.".into(),
            "to",
        ));
    }
    let now = unit_of(doc);
    if input.from != now {
        return Err(refuse(
            codes::UNIT_MISMATCH,
            format!(
                "Çizimin birimi {}, {} değil; from çizimin şimdiki birimi olmalı. Komutu çizimin şimdiki birimiyle yeniden hazırlayın.",
                now.mark(),
                input.from.mark()
            ),
            "from",
        ));
    }
    if input.geometry == UnitGeometry::Keep {
        return Ok(Checked {
            scale: None,
            warnings: Vec::new(),
            objects: 0,
            blocks: 0,
        });
    }
    let (uids, layers) = on_locked_layers(doc);
    if !layers.is_empty() {
        let named = layers
            .iter()
            .map(|(name, n)| format!("{name}: {n} nesne"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(Stop::failed(CommandError {
            uids,
            ..error(
                codes::LAYER_LOCKED,
                format!(
                    "Kilitli katmanda nesne var ({named}); ölçeklemek bütün çizimi değiştirir, hiçbir şey yazılmadı. Katmanın kilidini açın ya da boyları koruyun (geometry: keep)."
                ),
                Some("geometry".into()),
            )
        }));
    }
    let clouds = doc
        .entities()
        .filter(|e| matches!(e, Entity::PointCloud(_)))
        .count();
    let mut warnings = Vec::new();
    if clouds > 0 {
        warnings.push(warning(
            codes::POINT_CLOUDS_KEPT,
            format!("Nokta bulutlarının noktaları dosyalarındadır; {clouds} bulut ölçeklenmedi."),
        ));
    }
    if doc.sheets().is_some() {
        warnings.push(warning(
            codes::SHEETS_KEPT,
            "Pafta kitabı ölçeklenmedi; paftaların haritalarını yeni boylara göre ayarlayın."
                .into(),
        ));
    }
    let settings = doc.settings();
    if settings.topology.is_some() || !settings.networks.is_empty() {
        warnings.push(warning(
            codes::TOLERANCES_KEPT,
            "Projenin toleransları (topoloji, ağlar) değişmedi; yeni boylara göre gözden geçirin."
                .into(),
        ));
    }
    Ok(Checked {
        scale: Some(Scale::keeping_numbers(input.from, input.to)),
        warnings,
        objects: u32::try_from(doc.len() - clouds).unwrap_or(u32::MAX),
        blocks: u32::try_from(doc.blocks().len()).unwrap_or(u32::MAX),
    })
}

fn output(input: &UnitsChange, checked: &Checked, revision: String) -> UnitsChanged {
    UnitsChanged {
        from: input.from,
        to: input.to,
        geometry: input.geometry,
        factor: checked.scale.map_or(1.0, Scale::factor),
        objects: checked.objects,
        blocks: checked.blocks,
        revision,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &UnitsChange) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What the change would do, writing nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &UnitsChange) -> CommandResult<UnitsChanged> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document alone (Proje ayarları's question).
pub fn plan_in(doc: &Document, input: &UnitsChange) -> CommandResult<UnitsChanged> {
    match check(doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: output(input, &checked, doc.revision().to_string()),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the change as one undo step, into the
/// open transaction or group if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: UnitsChange) -> CommandResult<UnitsChanged> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_UNITS_CHANGE,
        kentos_commands::CAD_UNITS_CHANGE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: UnitsChange) -> CommandResult<UnitsChanged> {
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let before = doc.unit_state();
    let mut next = UnitState {
        drawing_unit: Some(input.to),
        ..before
    };
    if let Some(s) = checked.scale {
        if let Some(view) = &mut next.home_view {
            scale_bounds(view, s);
        }
        next.origin = Vec2 {
            x: s.apply(before.origin.x),
            y: s.apply(before.origin.y),
        };
    }
    // The unit first: it names the step.
    doc.set_unit_state(next, STEP);
    if let Some(s) = checked.scale {
        let changes: Vec<(Slot, Entity)> = doc
            .entities()
            .filter(|e| !matches!(e, Entity::PointCloud(_)))
            .map(|e| {
                let mut e = e.clone();
                scale_entity(&mut e, s);
                (Slot(e.base().id), e)
            })
            .collect();
        doc.update_many(changes, STEP);
        let blocks: Vec<_> = doc
            .blocks()
            .iter()
            .map(|b| {
                let mut b = (**b).clone();
                scale_block(&mut b, s);
                b
            })
            .collect();
        for block in blocks {
            if let Err(refusal) = doc.update_block(block) {
                return CommandResult::Failed {
                    error: error(codes::INTEGRITY_FAILED, refusal.0, None),
                };
            }
        }
    }
    CommandResult::Completed {
        output: output(&input, &checked, doc.revision().to_string()),
        warnings: checked.warnings,
    }
}
