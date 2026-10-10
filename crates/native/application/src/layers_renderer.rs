//! `cad.layers.renderer` v1 (docs/adr/0213 §5): a layer's renderer written,
//! or taken away (the layer's simple look), as one undo step “Katman
//! stili”; a layer that already has it is left as it is. The desktop's
//! handler over the native document; the web's is
//! `apps/web/src/product/layersRenderer.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.layers.renderer.json`.
//!
//! The checks, in the contract's order: the style core's rules of a renderer
//! (`style::rules::renderer_problem`, the web's through WASM: one answer);
//! the expected revision; the layer is the drawing's, a layer, not drawn
//! from a service.

use kentos_contracts::{
    CommandResult, LayerNode, LayerNodeType, LayerStyle, LayersRendered, LayersRenderer,
    LayersRendererPlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Katman stili";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

/// Why a renderer (its JSON) cannot be a layer's, as the style core says it; none when it can.
pub fn renderer_problem(renderer: &serde_json::Value) -> Option<String> {
    kentos_style_core::style::rules::renderer_problem_text(&renderer.to_string())
}

/// The layer and its style as execute would leave it.
fn check(doc: &Document, input: &LayersRenderer) -> Result<(LayerNode, LayerStyle), Stop> {
    let renderer = input.renderer.as_ref().filter(|r| !r.is_null());
    if let Some(why) = renderer.and_then(renderer_problem) {
        return Err(fail(
            codes::INVALID_RENDERER,
            format!("İşleyici: {why}"),
            "renderer",
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
                "“{name}” bir katman grubu; işleyici yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!(
                "“{name}” servisten çizilir ve nesne tutmaz; işleyici nesneleri olan katmanın olur."
            ),
            "layer",
        ));
    }
    let mut style = node.style.clone();
    style.renderer = renderer.cloned();
    Ok((node.clone(), style))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersRenderer) -> CommandResult<()> {
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
    input: &LayersRenderer,
) -> CommandResult<LayersRendererPlan> {
    let (node, style) = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let changed = node.style != style;
    CommandResult::Completed {
        output: LayersRendererPlan {
            node: LayerNode { style, ..node },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the renderer as one undo step “Katman stili” when it differs.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersRenderer,
) -> CommandResult<LayersRendered> {
    let style = match check(cx.doc, &input) {
        Ok((_, style)) => style,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let changed = doc.set_layer_style(&input.layer, style, LABEL);
    CommandResult::Completed {
        output: LayersRendered {
            layer: input.layer,
            changed,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
