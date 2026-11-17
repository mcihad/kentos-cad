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

/// The model's north (docs/adr/8076 §6; EK-7 Tablo 7.8 `TrueNorth`): the angle, radians counter-clockwise from the
/// plan's north, of true north at the middle of the drawing's walls in the project's system; in a project without one,
/// the drawing's north arrows' common turn (the author's north). Why none, said with its remedy: no system and no
/// arrow, arrows that disagree, or a system that does not reach the building.
pub fn model_north(doc: &Document) -> Result<f64, String> {
    let mut walls = doc.entities().filter_map(|e| match e {
        Entity::Wall(w) => Some(w.wall.pts.iter()),
        _ => None,
    });
    let first = walls.next().and_then(|mut p| p.next()).copied();
    let Some(first) = first else {
        return Err("Çizimde duvar yok: binanın yeri bilinmiyor.".into());
    };
    let (mut lo, mut hi) = (first, first);
    for p in doc.entities().filter_map(|e| match e {
        Entity::Wall(w) => Some(w.wall.pts.iter()),
        _ => None,
    }).flatten() {
        lo = kentos_commands::Vec2 { x: lo.x.min(p.x), y: lo.y.min(p.y) };
        hi = kentos_commands::Vec2 { x: hi.x.max(p.x), y: hi.y.max(p.y) };
    }
    let at = kentos_commands::Vec2 { x: (lo.x + hi.x) / 2.0, y: (lo.y + hi.y) / 2.0 };
    if kentos_project::systems::own(doc.settings()).is_some_and(|n| n.system.is_some()) {
        return true_north(doc, at).ok_or_else(|| {
            "Projenin koordinat sistemi binanın yerine ulaşmıyor: coğrafi kuzey bulunamadı. Projenin koordinat \
             sistemini ya da çizimin yerini düzeltin."
                .into()
        });
    }
    // No system: the author's north arrows, all turned alike.
    let turns: Vec<f64> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Insert(i) if doc.block(i.block).is_some_and(|b| b.name == NORTH_BLOCK) => Some(i.rotation),
            _ => None,
        })
        .collect();
    let Some(&turn) = turns.first() else {
        return Err("Projenin koordinat sistemi yok ve çizimde kuzey oku yok: modelin kuzeyi bilinmiyor. Projeye \
                    koordinat sistemi verin ya da Mimari › Kuzey oku ile kuzeyi gösterin."
            .into());
    };
    let apart = |a: f64, b: f64| {
        let d = (a - b).rem_euclid(std::f64::consts::TAU);
        d.min(std::f64::consts::TAU - d) > 1e-9
    };
    if turns.iter().any(|t| apart(*t, turn)) {
        return Err(format!(
            "Çizimdeki {} kuzey oku farklı yönleri gösteriyor: modelin kuzeyi belirsiz. Okları aynı yöne çevirin ya da \
             fazlasını silin.",
            turns.len()
        ));
    }
    Ok(turn)
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
