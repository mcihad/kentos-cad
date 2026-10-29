//! `cad.north.create` v1 (“Kuzey oku”, docs/adr/0721 §3): the north arrow placed at a point, turned to the plan's
//! north (the drawing's up), the true north at the point (the project's system's meridian convergence) or a given
//! angle, sized on paper at the drawing's scale. The drawing's own block “Kuzey oku” is placed when it has one; else
//! KentOS's library's is added in the same undo step “Kuzey oku”. Kuzey oku writes through it; the headless server,
//! Python and MCP run it from the catalog. The shared cases are `fixtures/commands/v1/cad.north.create.json`
//! (`scripts/fixtures/north_cases.py`).
//!
//! The checks, in order (the first that fails answers): `at`, the angle and the size finite; the size above 0 and at
//! most 200 mm (`invalid_scale`); the expected revision; true north known at `at` (`no_true_north`); the layer, as
//! every create command's.

use kentos_commands::{
    BlockDefinition, BlockId, CommandResult, CommandWarning, Entity, EntityBase, InsertEntity, MAX_NORTH_SIZE,
    NORTH_BLOCK, NORTH_SIZE, NorthCreate, NorthCreated, NorthDirection, NorthPlan,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::library_symbols;

/// The undo step's name.
pub const STEP: &str = "Kuzey oku";

/// What may be written.
struct Checked {
    insert: Entity,
    /// The library's definition to add first.
    block: Option<BlockDefinition>,
    rotation: f64,
    convergence: Option<f64>,
    warnings: Vec<CommandWarning>,
}

/// True north's angle from the plan's north at `at` in the project's system: none in a local project, or where the
/// system does not reach the point.
pub fn true_north(doc: &Document, at: kentos_commands::Vec2) -> Option<f64> {
    let named = kentos_project::systems::own(doc.settings())?;
    named.system?.true_north(kentos_geometry_core::Vec2::new(at.x, at.y))
}

fn check(doc: &Document, input: &NorthCreate, fresh: impl FnOnce() -> BlockId) -> Result<Checked, Stop> {
    checks::point(input.at, "Kuzey okunun yerinin", "at")?;
    if let NorthDirection::Angle { angle } = input.north {
        checks::finite(angle, "Kuzeyin açısı", "Açıyı radyan olarak sonlu bir sayıyla verin.", "north.angle")?;
    }
    let size = input.size.unwrap_or(NORTH_SIZE);
    checks::finite(size, "Kuzey okunun boyu", "Boyu kâğıt milimetresi olarak sonlu bir sayıyla verin.", "size")?;
    if !(size > 0.0 && size <= MAX_NORTH_SIZE) {
        return Err(Stop::failed(error(
            codes::INVALID_SCALE,
            format!(
                "Kuzey okunun boyu sıfırdan büyük, en çok {MAX_NORTH_SIZE} mm olmalı; {size} verildi. Kâğıttaki boyunu milimetre olarak verin."
            ),
            Some("size".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (rotation, convergence) = match input.north {
        NorthDirection::Plan => (0.0, None),
        NorthDirection::Angle { angle } => (angle, None),
        NorthDirection::True => {
            let Some(c) = true_north(doc, input.at) else {
                return Err(Stop::failed(error(
                    codes::NO_TRUE_NORTH,
                    "Projenin koordinat sisteminde bu noktadan geçen meridyen yok (yerel proje ya da sistemin dışında bir nokta): coğrafi kuzey bulunamaz. Plan kuzeyini ya da bir açı verin.".into(),
                    Some("north".into()),
                )));
            };
            (c, Some(c))
        }
    };
    let warnings = checks::layer(doc, &input.layer_id)?;
    // KentOS's library always has it; a drawing's own of the name wins.
    let Some(symbol) = library_symbols::symbol(doc, NORTH_BLOCK, fresh) else {
        return Err(Stop::failed(error(
            codes::UNKNOWN_BLOCK,
            "KentOS blok kitaplığında “Kuzey oku” yok.".into(),
            None,
        )));
    };
    let insert = Entity::Insert(InsertEntity {
        arch: Default::default(),
        clipping: None,
        parameters: Default::default(),
        attribute_places: Default::default(),
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: input.layer_id.clone(),
            color: input.color.clone(),
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        block: symbol.def.id,
        p: input.at,
        scale: library_symbols::scale_for(doc, &symbol.def, size),
        y_scale: None,
        rotation,
        mirror: false,
    });
    Ok(Checked {
        insert,
        block: symbol.added.then_some(symbol.def),
        rotation,
        convergence,
        warnings,
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &NorthCreate) -> CommandResult<()> {
    match check(cx.doc, input, || BlockId([0; 16])) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing (a library block keeps its id here; execute may give it another).
pub fn plan(cx: &ExecutionContext<'_>, input: &NorthCreate) -> CommandResult<NorthPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document (Kuzey oku's ghost).
pub fn plan_in(doc: &Document, input: &NorthCreate) -> CommandResult<NorthPlan> {
    match check(doc, input, || BlockId([0; 16])) {
        Ok(c) => CommandResult::Completed {
            output: NorthPlan {
                insert: c.insert,
                block: c.block,
                rotation: c.rotation,
                convergence: c.convergence,
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the arrow, and the library's block when the drawing lacks it, as one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: NorthCreate) -> CommandResult<NorthCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_NORTH_CREATE,
        kentos_commands::CAD_NORTH_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: NorthCreate) -> CommandResult<NorthCreated> {
    let c = match check(cx.doc, &input, || BlockId(*Uuid::now_v7().as_bytes())) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    cx.doc.name_step(STEP);
    let added = c.block.is_some();
    if let Some(def) = c.block
        && let Err(refused) = cx.doc.add_block(def)
    {
        return Stop::failed(error(codes::BLOCK_REFUSED, refused.to_string(), None)).into();
    }
    let Entity::Insert(i) = &c.insert else {
        unreachable!("the plan is an insert");
    };
    let block = i.block;
    match checks::add(cx.doc, c.insert) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: NorthCreated {
                uid,
                id,
                block,
                added,
                rotation: c.rotation,
                convergence: c.convergence,
                revision,
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}
