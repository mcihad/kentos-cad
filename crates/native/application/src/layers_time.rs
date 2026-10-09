//! `cad.layers.time` v1 (docs/adr/0210 §11): a layer's time setting written,
//! or taken away, as one undo step “Zaman ayarları”; a layer that already has
//! it as given is left as it is. The desktop's handler over the native
//! document; the web's is `apps/web/src/product/layersTime.ts`. Both pass the
//! shared cases in `fixtures/commands/v1/cad.layers.time.json`.
//!
//! The checks, in the contract's order: the setting's rules; the expected
//! revision; the layer is the drawing's, a layer, not drawn from a service.

use kentos_contracts::{
    CommandResult, LayerNode, LayerNodeType, LayersTime, LayersTimePlan, LayersTimed,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Zaman ayarları";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersTime) -> Result<LayerNode, Stop> {
    if let Some(problem) = input.time.as_ref().and_then(|t| t.problem()) {
        return Err(fail(
            codes::INVALID_TIME,
            format!("Katmanın zamanı: {problem}."),
            "time",
        ));
    }
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
                "“{name}” bir katman grubu; zaman yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() && input.time.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!(
                "“{name}” servisten çizilir ve nesne tutmaz; zaman nesneleri olan katmanın olur."
            ),
            "layer",
        ));
    }
    Ok(node.clone())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersTime) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersTime) -> CommandResult<LayersTimePlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let changed = node.time != input.time;
    CommandResult::Completed {
        output: LayersTimePlan {
            node: LayerNode {
                time: input.time.clone(),
                ..node
            },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the setting as one undo step “Zaman ayarları” when it differs.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersTime) -> CommandResult<LayersTimed> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    let doc = &mut *cx.doc;
    match doc.set_layer_time(&input.layer, input.time, LABEL) {
        Ok(changed) => CommandResult::Completed {
            output: LayersTimed {
                layer: input.layer,
                changed,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}
