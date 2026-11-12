//! `cad.layers.subtypes` v1 (Alt türler, docs/adr/0422 §3): a layer's subtypes written as one undo step
//! “Alt türler”; a layer that already has them as given is left as it is. The shared cases are
//! `fixtures/commands/v1/cad.layers.subtypes.json`.
//!
//! The checks, in the contract's order: the expected revision; the layer is the drawing's, a layer,
//! not drawn from a service; the subtypes fit its fields (`subtypes_problem`).

use kentos_commands::{
    CommandResult, LayerNode, LayerNodeType, LayersComposePlan, LayersComposed, LayersSubtypes,
    subtypes_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Alt türler";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersSubtypes) -> Result<LayerNode, Stop> {
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
                "“{name}” bir katman grubu; alt türler yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{name}” servisten çizilir ve nesne tutmaz; alt türü yoktur."),
            "layer",
        ));
    }
    if let Some(s) = &input.subtypes
        && let Some((at, problem)) = subtypes_problem(&node.fields, s)
    {
        return Err(fail(
            codes::INVALID_SUBTYPES,
            problem,
            &format!("subtypes/{at}"),
        ));
    }
    Ok(node.clone())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersSubtypes) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersSubtypes) -> CommandResult<LayersComposePlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let changed = node.subtypes != input.subtypes;
    CommandResult::Completed {
        output: LayersComposePlan {
            node: LayerNode {
                subtypes: input.subtypes.clone(),
                ..node
            },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the subtypes as one undo step “Alt türler” when they differ.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersSubtypes,
) -> CommandResult<LayersComposed> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_SUBTYPES,
        kentos_commands::CAD_LAYERS_SUBTYPES_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersSubtypes,
) -> CommandResult<LayersComposed> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    let doc = &mut *cx.doc;
    match doc.set_layer_subtypes(&input.layer, input.subtypes, LABEL) {
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
