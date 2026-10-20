//! `cad.layers.contingents` v1 (Değer bileşimleri, docs/adr/0423 §3): a layer's value groups written as
//! one undo step “Değer bileşimleri”; a layer that already has them as given is left as it is. The
//! shared cases are `fixtures/commands/v1/cad.layers.contingents.json`.
//!
//! The checks, in the contract's order: the expected revision; the layer is the drawing's, a layer,
//! not drawn from a service; the groups fit its fields and subtypes (`contingents_problem`).

use kentos_commands::{
    CommandResult, LayerNode, LayerNodeType, LayersComposePlan, LayersComposed, LayersContingents,
    contingents_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Değer bileşimleri";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersContingents) -> Result<LayerNode, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let id = input.layer.as_str();
    let Some(node) = doc.layers().get(id) else {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            "layer",
        ));
    };
    let name = &node.name;
    if node.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{name}” bir katman grubu; değer bileşimleri yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{name}” servisten çizilir ve nesne tutmaz; değer grubu yoktur."),
            "layer",
        ));
    }
    if let Some((at, problem)) =
        contingents_problem(&node.fields, node.subtypes.as_ref(), &input.groups)
    {
        let path = if at.is_empty() {
            "groups".to_owned()
        } else {
            format!("groups/{at}")
        };
        return Err(fail(codes::INVALID_CONTINGENTS, problem, &path));
    }
    Ok(node.clone())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersContingents) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &LayersContingents,
) -> CommandResult<LayersComposePlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let changed = node.contingents != input.groups;
    CommandResult::Completed {
        output: LayersComposePlan {
            node: LayerNode {
                contingents: input.groups.clone(),
                ..node
            },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the groups as one undo step “Değer bileşimleri” when they differ.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersContingents,
) -> CommandResult<LayersComposed> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_CONTINGENTS,
        kentos_commands::CAD_LAYERS_CONTINGENTS_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersContingents,
) -> CommandResult<LayersComposed> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    let doc = &mut *cx.doc;
    match doc.set_layer_contingents(&input.layer, input.groups, LABEL) {
        Ok(changed) => CommandResult::Completed {
            output: LayersComposed {
                layer: input.layer,
                changed,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}
