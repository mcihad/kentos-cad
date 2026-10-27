//! `cad.slabs.create` v1 (docs/adr/0722 §4): slabs on a named layer, each with its outline drawn or found from the
//! walls round its seed, its edges moved by an edge offset, one undo step “Döşeme”. The slab tool previews the plan's
//! slab and writes through `execute`; Python and an agent give the same outline or point and get the same slab. The
//! shared cases are `fixtures/commands/v1/cad.slabs.create.json`.
//!
//! The checks, in order (the first that fails answers): 1. at least one slab (`no_objects`), at most `MAX_SLABS`
//! (`invalid_slab`); 2. each slab in the input's order: its numbers (`not_finite`), an outline drawn or a point
//! (`invalid_slab`), its level (`level_not_found`), its outline from the walls (`slab_not_enclosed`), its edge offset
//! (`invalid_slab` past 10 m, `slab_offset_failed`), its fields (`invalid_slab`); 3. the line weight; 4. the expected
//! revision; 5. the layer; 6. the attributes.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, SlabEntity, SlabFields, SlabsCreate, SlabsCreated, SlabsPlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::slabs;
use crate::spaces::{self, fail};

/// The step's name.
pub const STEP: &str = "Döşeme";

pub fn validate(cx: &ExecutionContext<'_>, input: &SlabsCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((warnings, _)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SlabsCreate) -> CommandResult<SlabsPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the slab tool draws as its preview.
pub fn plan_in(doc: &Document, input: &SlabsCreate) -> CommandResult<SlabsPlan> {
    match check(doc, input) {
        Ok((warnings, made)) => CommandResult::Completed {
            output: SlabsPlan {
                slabs: made.into_iter().map(|(s, a)| entity(input, s, a)).collect(),
                revision: doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: SlabsCreate) -> CommandResult<SlabsCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SLABS_CREATE,
        kentos_commands::CAD_SLABS_CREATE_VERSION,
        input,
        write_execute,
    )
}

pub(crate) fn write_execute(cx: &mut ExecutionContext<'_>, input: SlabsCreate) -> CommandResult<SlabsCreated> {
    let (warnings, made) = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let entities: Vec<Entity> = made.into_iter().map(|(s, a)| entity(&input, s, a)).collect();
    let slots = match doc.add_many(entities, STEP) {
        Ok(slots) => slots,
        Err(full) => {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
        }
    };
    CommandResult::Completed {
        output: SlabsCreated {
            uids: slots.iter().filter_map(|s| doc.uid(*s)).map(|u| u.to_string()).collect(),
            ids: slots.iter().map(|s| s.0).collect(),
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

type Attrs = Option<std::collections::BTreeMap<String, String>>;
type Checked = (Vec<CommandWarning>, Vec<(SlabFields, Attrs)>);

fn check(doc: &Document, input: &SlabsCreate) -> Result<Checked, Stop> {
    if input.slabs.is_empty() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Yazılacak döşeme verilmedi. En az bir döşemenin sınırını ya da duvarlarla çevrili bir iç noktasını verin."
                .into(),
            "slabs",
        ));
    }
    if input.slabs.len() > kentos_commands::MAX_SLABS {
        return Err(fail(
            codes::INVALID_SLAB,
            format!(
                "Bir çağrıda en çok {} döşeme yazılır; {} verildi.",
                kentos_commands::MAX_SLABS,
                input.slabs.len()
            ),
            "slabs",
        ));
    }
    let mut made = Vec::with_capacity(input.slabs.len());
    for (i, n) in input.slabs.iter().enumerate() {
        let at = format!("slabs[{i}]");
        if let Some(p) = n.seed {
            checks::point(p, "Döşemenin iç noktasının", &format!("{at}.seed"))?;
        }
        if let Some(parts) = &n.parts {
            slabs::parts_finite(parts, &format!("{at}.parts"))?;
        }
        for (v, what, field) in [
            (n.thickness, "Kalınlık", "thickness"),
            (n.offset, "Kot farkı", "offset"),
            (n.edge_offset, "Kenar ötelemesi", "edgeOffset"),
        ] {
            if let Some(v) = v {
                checks::finite(v, what, "Sonlu bir sayı verin.", &format!("{at}.{field}"))?;
            }
        }
        if let Some(e) = &n.edge {
            slabs::edge_finite(e, &format!("{at}.edge"))?;
        }
        let arch = n.arch.clone().unwrap_or_default();
        if n.parts.is_none() != n.seed.is_none() {
            spaces::level_known(doc, &arch, &format!("{at}.arch.level"))?;
        }
        let parts = slabs::outline(doc, n.parts.as_ref(), n.seed, n.boundary, arch.level.as_deref(), &at)?;
        let parts = slabs::moved(&parts, n.edge_offset.unwrap_or(0.0), &format!("{at}.edgeOffset"))?;
        let s = SlabFields {
            parts,
            role: n.role.unwrap_or_default(),
            thickness: n.thickness.unwrap_or(kentos_commands::SLAB_THICKNESS),
            offset: n.offset.unwrap_or(0.0),
            arch,
            material: n.material.clone(),
            edge: n.edge.as_ref().map(slabs::edge_of),
            excluded: n.excluded.unwrap_or(false),
        };
        slabs::valid(&s, &at)?;
        // Its edge's runs as the document writes them, so the plan draws them (docs/adr/0722 §3).
        made.push(doc.slab_with_runs(&s));
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let mut out = Vec::with_capacity(made.len());
    for (i, (s, n)) in made.into_iter().zip(&input.slabs).enumerate() {
        let attrs = checks::object_attrs(doc, &input.layer_id, n.attrs.as_ref(), &format!("slabs[{i}].attrs"))?;
        out.push((s, attrs));
    }
    Ok((warnings, out))
}

/// The slab as the document stores it; its `id` is the document's to give (0 in a plan).
fn entity(input: &SlabsCreate, slab: SlabFields, attrs: Attrs) -> Entity {
    Entity::Slab(SlabEntity {
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
        slab,
    })
}
