//! `cad.elevations.query` v1 (docs/adr/0799 §4): the drawing's derived interior elevations (the layers whose derivation
//! record names `cad.elevations.edit`) in the layer tree's order, each with its space, its sides and whether it still is
//! what the model would draw (`current`), was overtaken by the model or lost objects (`stale`) or lost its space
//! (`gone`); one by its layer, or the one an object is a member of. Read-only. The shared cases are
//! `fixtures/commands/v1/cad.elevations.query.json`.

use kentos_commands::{
    CommandResult, ElevationRow, ElevationsQueried, ElevationsQuery, Entity, LayerNode,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::elevations;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn walk<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
    for n in nodes {
        out.push(n);
        walk(&n.children, out);
    }
}

/// The row of the elevation on `layer`; none when the layer holds none.
pub fn row(doc: &Document, layer: &str) -> Option<ElevationRow> {
    let node = doc.layers().get(layer)?;
    let (d, r) = elevations::on_layer(doc, layer)?;
    let s = elevations::standing(doc, layer, &r);
    let space_label = Uuid::parse_str(&r.space)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|slot| doc.get(slot))
        .and_then(|e| match e {
            Entity::Space(sp) => {
                let l = elevations::label_of(sp);
                (l != "Mahal").then_some(l)
            }
            _ => None,
        });
    Some(ElevationRow {
        layer: layer.to_owned(),
        name: node.name.clone(),
        space: r.space.clone(),
        space_label,
        sides: r.sides.clone(),
        state: s.state,
        members: r.members.len() as u32,
        missing: s.missing,
        edited: s.edited,
        derived_at: d.document_revision,
    })
}

fn check(doc: &Document, input: &ElevationsQuery) -> Result<Vec<ElevationRow>, Stop> {
    let mut only: Option<String> = None;
    if let Some(l) = &input.layer {
        let Some(node) = doc.layers().get(l) else {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!("“{l}” kimlikli katman çizimde yok."),
                "layer",
            ));
        };
        if elevations::on_layer(doc, l).is_none() {
            return Err(fail(
                codes::ELEVATION_NOT_FOUND,
                format!("“{}” katmanı bir oda iç görünüşü değil (türetim kaydı yok).", node.name),
                "layer",
            ));
        }
        only = Some(l.clone());
    }
    if let Some(u) = &input.uid {
        if !checks::is_uid_text(u) {
            return Err(fail(
                codes::INVALID_UID,
                format!("“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
                "uid",
            ));
        }
        if Uuid::parse_str(u).ok().and_then(|x| doc.slot_of(x)).is_none() {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!("“{u}” kimlikli nesne çizimde yok."),
                "uid",
            ));
        }
        let mut nodes = Vec::new();
        walk(doc.layers().nodes(), &mut nodes);
        let owner = nodes.iter().find_map(|n| {
            let (_, r) = elevations::on_layer(doc, &n.id)?;
            r.members.values().any(|m| m == u).then(|| n.id.clone())
        });
        let Some(owner) = owner else {
            return Err(fail(
                codes::ELEVATION_NOT_FOUND,
                "Nesne bir oda iç görünüşünün nesnesi değil.".into(),
                "uid",
            ));
        };
        if only.as_ref().is_some_and(|l| *l != owner) {
            return Err(fail(
                codes::ELEVATION_NOT_FOUND,
                "Nesne verilen katmanın iç görünüşünün nesnesi değil.".into(),
                "uid",
            ));
        }
        only = Some(owner);
    }
    if let Some(l) = only {
        return Ok(row(doc, &l).into_iter().collect());
    }
    let mut nodes = Vec::new();
    walk(doc.layers().nodes(), &mut nodes);
    Ok(nodes.iter().filter_map(|n| row(doc, &n.id)).collect())
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ElevationsQuery) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ElevationsQuery) -> CommandResult<ElevationsQueried> {
    execute_in(cx.doc, input)
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ElevationsQuery) -> CommandResult<ElevationsQueried> {
    execute_in(cx.doc, &input)
}

/// [`execute`] on a document: what Öznitelikler and the tool read.
pub fn execute_in(doc: &Document, input: &ElevationsQuery) -> CommandResult<ElevationsQueried> {
    match check(doc, input) {
        Ok(elevations) => CommandResult::Completed {
            output: ElevationsQueried {
                elevations,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
