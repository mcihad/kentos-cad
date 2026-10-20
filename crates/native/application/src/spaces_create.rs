//! `cad.spaces.create` v1 (docs/adr/0342 §4, plan/cad.md `CAD-ARCH-23`, `24`, `25`): spaces on a named layer, each
//! found from the walls round its seed or with its boundary drawn, one undo step “Mahal”. The space tool previews the
//! plan's space and writes through `execute`; Python and an agent give the same seed and get the same space. The shared
//! cases are `fixtures/commands/v1/cad.spaces.create.json`.
//!
//! The checks, in order (the first that fails answers): 1. at least one space (`no_objects`), at most `MAX_SPACES`
//! (`invalid_space`); 2. each space in the input's order: its numbers (`not_finite`), its boundary (from the walls:
//! `space_not_enclosed`; drawn; neither: `invalid_space`), its fields (`invalid_space`), its level (`level_not_found`);
//! 3. the line weight; 4. the expected revision; 5. the layer; 6. the attributes.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, SpaceEntity, SpaceFields, SpacesCreate,
    SpacesCreated, SpacesPlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::spaces::{self, fail};

/// The step's name.
pub const STEP: &str = "Mahal";

pub fn validate(cx: &ExecutionContext<'_>, input: &SpacesCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((warnings, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SpacesCreate) -> CommandResult<SpacesPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the space tool draws as its preview.
pub fn plan_in(doc: &Document, input: &SpacesCreate) -> CommandResult<SpacesPlan> {
    match check(doc, input) {
        Ok((warnings, made)) => CommandResult::Completed {
            output: SpacesPlan {
                spaces: made.into_iter().map(|(s, a)| entity(input, s, a)).collect(),
                revision: doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: SpacesCreate) -> CommandResult<SpacesCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SPACES_CREATE,
        kentos_commands::CAD_SPACES_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: SpacesCreate,
) -> CommandResult<SpacesCreated> {
    let (warnings, made) = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let entities: Vec<Entity> = made
        .into_iter()
        .map(|(s, a)| entity(&input, s, a))
        .collect();
    let slots = match doc.add_many(entities, STEP) {
        Ok(slots) => slots,
        Err(full) => {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
        }
    };
    CommandResult::Completed {
        output: SpacesCreated {
            uids: slots
                .iter()
                .filter_map(|s| doc.uid(*s))
                .map(|u| u.to_string())
                .collect(),
            ids: slots.iter().map(|s| s.0).collect(),
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

type Attrs = Option<std::collections::BTreeMap<String, String>>;
type Checked = (Vec<CommandWarning>, Vec<(SpaceFields, Attrs)>);

fn check(doc: &Document, input: &SpacesCreate) -> Result<Checked, Stop> {
    if input.spaces.is_empty() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Yazılacak mahal verilmedi. En az bir mahalin iç noktasını ya da sınırını verin."
                .into(),
            "spaces",
        ));
    }
    if input.spaces.len() > kentos_commands::MAX_SPACES {
        return Err(fail(
            codes::INVALID_SPACE,
            format!(
                "Bir çağrıda en çok {} mahal yazılır; {} verildi.",
                kentos_commands::MAX_SPACES,
                input.spaces.len()
            ),
            "spaces",
        ));
    }
    let mut made = Vec::with_capacity(input.spaces.len());
    for (i, n) in input.spaces.iter().enumerate() {
        let at = format!("spaces[{i}]");
        if let Some(p) = n.seed {
            checks::point(p, "Mahalin iç noktasının", &format!("{at}.seed"))?;
        }
        if let Some(h) = n.height {
            checks::finite(
                h,
                "Yükseklik",
                "Sonlu bir sayı verin.",
                &format!("{at}.height"),
            )?;
        }
        for (k, p) in n.parts.iter().flatten().enumerate() {
            for (j, q) in p.outer.pts.iter().enumerate() {
                checks::point(
                    *q,
                    &format!("{}. köşenin", j + 1),
                    &format!("{at}.parts[{k}].outer.pts[{j}]"),
                )?;
            }
        }
        let method = n.method.unwrap_or_else(|| spaces::project_method(doc));
        let mut s = SpaceFields {
            parts: n.parts.clone().unwrap_or_default(),
            seed: n.seed,
            method,
            number: n.number.clone(),
            name: n.name.clone(),
            usage: n.usage.clone(),
            arch: n.arch.clone().unwrap_or_default(),
            height: n.height,
            finishes: n.finishes.clone().unwrap_or_default(),
            broken: None,
        };
        match (n.seed, &n.parts) {
            (Some(_), _) => {
                s = doc.space_found(&s);
                if s.broken.is_some() {
                    return Err(spaces::not_enclosed(&format!("{at}.seed")));
                }
            }
            (None, Some(_)) => {}
            (None, None) => {
                return Err(fail(
                    codes::INVALID_SPACE,
                    "Mahalin yeri verilmedi: duvarlarla çevrili bir iç nokta (seed) ya da sınırını (parts) verin.".into(),
                    &at,
                ));
            }
        }
        spaces::valid(&s, &at)?;
        spaces::level_known(doc, &s.arch, &format!("{at}.arch.level"))?;
        made.push(s);
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let mut out = Vec::with_capacity(made.len());
    for (i, (s, n)) in made.into_iter().zip(&input.spaces).enumerate() {
        let attrs = checks::object_attrs(
            doc,
            &input.layer_id,
            n.attrs.as_ref(),
            &format!("spaces[{i}].attrs"),
        )?;
        out.push((s, attrs));
    }
    Ok((warnings, out))
}

/// The space as the document stores it; its `id` is the document's to give (0 in a plan).
fn entity(input: &SpacesCreate, space: SpaceFields, attrs: Attrs) -> Entity {
    Entity::Space(SpaceEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: input.layer_id.clone(),
            color: input.color.clone(),
            attrs: attrs.unwrap_or_default(),
            label: None,
            symbol: None,
            line_weight: input.line_weight,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        space,
    })
}
