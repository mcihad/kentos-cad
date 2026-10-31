//! `cad.walls.create` v1 (docs/adr/0339 §5, plan/cad.md `CAD-ARCH-05`): walls on a named layer along their paths as
//! drawn, one undo step “Duvar”. The wall tool previews the plan's walls and writes through `execute`; Python and an
//! agent give the same path and get the same walls. The shared cases are `fixtures/commands/v1/cad.walls.create.json`.
//!
//! The checks, in order (the first that fails answers): 1. at least one wall (`no_objects`); 2. each wall in the
//! input's order (`walls::build`: its numbers, its style, its fields as the file checks them, its faces, its level);
//! 3. the line weight; 4. the expected revision; 5. the layer; 6. each wall's attributes by the layer's fields.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, WallEntity, WallFields, WallStyleDef,
    WallsCreate, WallsCreated, WallsPlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::walls;

/// The step's name.
pub const STEP: &str = "Duvar";

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &WallsCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((warnings, ..)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &WallsCreate) -> CommandResult<WallsPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the wall tool draws as its preview.
pub fn plan_in(doc: &Document, input: &WallsCreate) -> CommandResult<WallsPlan> {
    match check(doc, input) {
        Ok((warnings, walls, added)) => CommandResult::Completed {
            output: WallsPlan {
                walls: walls
                    .into_iter()
                    .zip(&input.walls)
                    .map(|((w, a), n)| entity(input, n, w, a))
                    .collect(),
                styles_added: added,
                adopted: Vec::new(),
                revision: doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the walls as one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: WallsCreate) -> CommandResult<WallsCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_WALLS_CREATE,
        kentos_commands::CAD_WALLS_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: WallsCreate) -> CommandResult<WallsCreated> {
    let (warnings, walls, added) = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    walls::take_styles(doc, &added);
    let entities: Vec<Entity> = walls
        .into_iter()
        .zip(&input.walls)
        .map(|((w, a), n)| entity(&input, n, w, a))
        .collect();
    let slots = match doc.add_many(entities, STEP) {
        Ok(slots) => slots,
        Err(full) => {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
        }
    };
    CommandResult::Completed {
        output: WallsCreated {
            uids: slots
                .iter()
                .filter_map(|s| doc.uid(*s))
                .map(|u| u.to_string())
                .collect(),
            ids: slots.iter().map(|s| s.0).collect(),
            styles_added: added.into_iter().map(|s| s.id).collect(),
            adopted: Vec::new(),
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

/// The warnings, each wall with its attributes as written (canonical, the layer's defaults filled) and KentOS's styles
/// the project takes.
type Checked = (
    Vec<CommandWarning>,
    Vec<(WallFields, Attrs)>,
    Vec<WallStyleDef>,
);
type Attrs = Option<std::collections::BTreeMap<String, String>>;

fn check(doc: &Document, input: &WallsCreate) -> Result<Checked, Stop> {
    if input.walls.is_empty() {
        return Err(Stop::failed(error(
            codes::NO_OBJECTS,
            "Yazılacak duvar verilmedi. En az bir duvarın yolunu verin.".into(),
            Some("walls".into()),
        )));
    }
    if input.walls.len() > kentos_commands::MAX_WALLS {
        return Err(Stop::failed(error(
            codes::INVALID_WALL,
            format!(
                "Bir çağrıda en çok {} duvar yazılır; {} verildi.",
                kentos_commands::MAX_WALLS,
                input.walls.len()
            ),
            Some("walls".into()),
        )));
    }
    let mut added = Vec::new();
    let mut out = Vec::with_capacity(input.walls.len());
    for (i, w) in input.walls.iter().enumerate() {
        out.push(walls::build(doc, w, &format!("walls[{i}]"), &mut added)?);
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let mut walls = Vec::with_capacity(out.len());
    for (i, (wall, w)) in out.into_iter().zip(&input.walls).enumerate() {
        let attrs = checks::object_attrs(
            doc,
            &input.layer_id,
            w.attrs.as_ref(),
            &format!("walls[{i}].attrs"),
        )?;
        walls.push((wall, attrs));
    }
    // What the new walls do not join (docs/adr/0340 §2), said with what resolves it.
    let mut warnings = warnings;
    let made: Vec<WallEntity> = walls
        .iter()
        .zip(&input.walls)
        .filter_map(|((w, _), n)| match entity(input, n, w.clone(), None) {
            Entity::Wall(e) => Some(e),
            _ => None,
        })
        .collect();
    let changed: Vec<(Option<kentos_domain::Slot>, &WallEntity)> =
        made.iter().map(|e| (None, e)).collect();
    warnings.extend(walls::join_warnings(doc, &changed, &[]));
    Ok((warnings, walls, added))
}

/// The wall as the document stores it; its `id` is the document's to give (0 in a plan).
fn entity(
    input: &WallsCreate,
    n: &kentos_commands::NewWall,
    wall: WallFields,
    attrs: Attrs,
) -> Entity {
    Entity::Wall(WallEntity {
        base: EntityBase {
            look: kentos_commands::ObjectLook {
                line_type: input.line_type,
                ..Default::default()
            },
            id: 0,
            layer_id: input.layer_id.clone(),
            color: input.color.clone(),
            attrs: attrs.unwrap_or_default(),
            label: n.label.clone(),
            symbol: None,
            line_weight: input.line_weight,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        wall,
    })
}
