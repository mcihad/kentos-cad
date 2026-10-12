//! `cad.layers.info` v1 (docs/adr/0225 §4, §5): a layer's actions and
//! information card written as one undo step “Bilgi kartı”; a layer that
//! already has them as given is left as it is. A card with nothing in it is
//! no card. The desktop's handler over the native document; the web's is
//! `apps/web/src/product/layersInfo.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.layers.info.json`.
//!
//! The checks, in the contract's order: the actions' rules, the card's; the
//! templates split and their expressions compile (`$sıra` and `$ölçek`
//! refused), the actions' targets in order, then the card's title and text;
//! the expected revision; the layer is the drawing's, a layer, not drawn from
//! a service.

use kentos_contracts::{
    CommandResult, InfoCard, LayerNode, LayerNodeType, LayersInfo, LayersInfoPlan, LayersInformed,
    actions_problem,
};
use kentos_domain::{Document, LayerInfo};
use kentos_expression::Unit;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;
use crate::info_templates::compile_template;

/// The undo step's name.
pub const LABEL: &str = "Bilgi kartı";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

/// The input as the layer will keep it: a card with nothing in it is none.
fn info_of(input: &LayersInfo) -> LayerInfo {
    LayerInfo {
        actions: input.actions.clone(),
        card: input.card.clone().filter(|c| *c != InfoCard::default()),
    }
}

fn check(doc: &Document, input: &LayersInfo) -> Result<LayerNode, Stop> {
    if let Some(problem) = actions_problem(&input.actions) {
        return Err(fail(
            codes::INVALID_ACTIONS,
            format!("Katmanın eylemleri: {problem}."),
            "actions",
        ));
    }
    if let Some(problem) = input.card.as_ref().and_then(InfoCard::problem) {
        return Err(fail(
            codes::INVALID_CARD,
            format!("Katmanın bilgi kartı: {problem}."),
            "card",
        ));
    }
    for (i, a) in input.actions.iter().enumerate() {
        compile_template(&a.target, Unit::METRE).map_err(|why| {
            fail(
                codes::INVALID_TEMPLATE,
                format!("“{}” eyleminin hedefi: {why}", a.name),
                &format!("actions/{i}/target"),
            )
        })?;
    }
    if let Some(card) = &input.card {
        for (text, what, path) in [
            (&card.title, "Kartın başlığı", "card/title"),
            (&card.text, "Kartın metni", "card/text"),
        ] {
            if let Some(text) = text {
                compile_template(text, Unit::METRE)
                    .map_err(|why| fail(codes::INVALID_TEMPLATE, format!("{what}: {why}"), path))?;
            }
        }
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
    let info = info_of(input);
    let given = !info.actions.is_empty() || info.card.is_some();
    if node.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{name}” bir katman grubu; eylemler ve bilgi kartı yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() && given {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!(
                "“{name}” servisten çizilir ve nesne tutmaz; eylemler ve bilgi kartı nesneleri olan katmanın olur."
            ),
            "layer",
        ));
    }
    Ok(node.clone())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersInfo) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersInfo) -> CommandResult<LayersInfoPlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let info = info_of(input);
    let changed = node.actions != info.actions || node.card != info.card;
    CommandResult::Completed {
        output: LayersInfoPlan {
            node: LayerNode {
                actions: info.actions,
                card: info.card,
                ..node
            },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the actions and the card as one undo step “Bilgi kartı” when they differ.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersInfo) -> CommandResult<LayersInformed> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    let doc = &mut *cx.doc;
    match doc.set_layer_info(&input.layer, info_of(&input), LABEL) {
        Ok(changed) => CommandResult::Completed {
            output: LayersInformed {
                layer: input.layer,
                changed,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}
