//! `cad.layers.compose` v1 (docs/adr/0226 §3, §4): a layer's opacity,
//! blending mode and mask written as one undo step “Katman çizimi”; the
//! layer's composition becomes exactly what is given, a layer that already
//! has it is left as it is. The desktop's handler over the native document;
//! the web's is `apps/web/src/product/layersCompose.ts`. Both pass the shared
//! cases in `fixtures/commands/v1/cad.layers.compose.json`.
//!
//! The checks, in the contract's order: the opacity, the mask's rule; the
//! expected revision; the layer is the drawing's and a layer; each of the
//! mask's targets is another layer of the drawing.

use kentos_contracts::{
    CommandResult, LayerNode, LayerNodeType, LayersCompose, LayersComposePlan, LayersComposed,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Katman çizimi";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersCompose) -> Result<LayerNode, Stop> {
    if let Some(o) = input.opacity
        && (!o.is_finite() || !(0.0..=1.0).contains(&o))
    {
        return Err(fail(
            codes::INVALID_COMPOSE,
            "Katman çizimi: saydamlık 0 ile 1 arasında olmalı.".to_owned(),
            "opacity",
        ));
    }
    if let Some(problem) = input.mask.as_ref().and_then(|m| m.problem()) {
        return Err(fail(
            codes::INVALID_COMPOSE,
            format!("Katman çizimi: {problem}."),
            "mask",
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
    if node.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{}” bir katman grubu; katman çizimi yalnız katmanın olur. Grubun bir katmanını verin.",
                node.name
            ),
            "layer",
        ));
    }
    for (i, t) in input.mask.iter().flat_map(|m| m.targets.iter()).enumerate() {
        let path = format!("mask/targets/{i}");
        if t == id {
            return Err(fail(
                codes::MASK_TARGET_SELF,
                format!(
                    "“{}” kendini maskeleyemez; maskelenecek başka bir katman verin.",
                    node.name
                ),
                &path,
            ));
        }
        match doc.layers().get(t) {
            None => {
                return Err(fail(
                    codes::MASK_TARGET_NOT_FOUND,
                    format!(
                        "Maskelenecek “{t}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    &path,
                ));
            }
            Some(target) if target.kind != LayerNodeType::Layer => {
                return Err(fail(
                    codes::MASK_TARGET_NOT_A_LAYER,
                    format!(
                        "“{}” bir katman grubu; maske katmana uygulanır. Grubun katmanlarını verin.",
                        target.name
                    ),
                    &path,
                ));
            }
            Some(_) => {}
        }
    }
    Ok(node.clone())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersCompose) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersCompose) -> CommandResult<LayersComposePlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let compose = input.composition();
    let changed = node.compose != compose;
    CommandResult::Completed {
        output: LayersComposePlan {
            node: LayerNode { compose, ..node },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the composition as one undo step “Katman çizimi” when it differs.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersCompose,
) -> CommandResult<LayersComposed> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    let doc = &mut *cx.doc;
    match doc.set_layer_compose(&input.layer, input.composition(), LABEL) {
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
