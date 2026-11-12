//! `cad.layers.rules` v1 (Kayıt kuralları, docs/adr/0438 §1): a layer's record rules written as one
//! undo step “Kayıt kuralları”; a layer that already has them as given is left as it is. The shared
//! cases are `fixtures/commands/v1/cad.layers.rules.json`.
//!
//! The checks, in the contract's order: the expected revision; the layer is the drawing's, a layer,
//! not drawn from a service; the rules fit its fields (`layer_rules_problem`) and compile.

use kentos_commands::{
    CommandResult, LayerNode, LayerNodeType, LayersComposePlan, LayersComposed, LayersRules,
    layer_rules_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Kayıt kuralları";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersRules) -> Result<LayerNode, Stop> {
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
                "“{name}” bir katman grubu; kayıt kuralları yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{name}” servisten çizilir ve nesne tutmaz; kayıt kuralı yoktur."),
            "layer",
        ));
    }
    if let Some((at, problem)) = layer_rules_problem(&node.fields, &input.rules) {
        let path = if at.is_empty() {
            "rules".to_owned()
        } else {
            format!("rules/{at}")
        };
        return Err(fail(codes::INVALID_RULES, problem, &path));
    }
    // An expression that does not compile, or reads a field the layer lacks, is refused here: the
    // writes that ask the rule could not.
    if let Err((i, problem)) = kentos_forms::LayerRules::new(
        &input.rules,
        &node.fields,
        &crate::forms::variables_all(doc),
        crate::expression_unit::reading_unit(doc.settings()),
    ) {
        return Err(fail(
            codes::INVALID_RULES,
            problem,
            &format!("rules/{i}/rule"),
        ));
    }
    Ok(node.clone())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersRules) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersRules) -> CommandResult<LayersComposePlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let changed = node.rules != input.rules;
    CommandResult::Completed {
        output: LayersComposePlan {
            node: LayerNode {
                rules: input.rules.clone(),
                ..node
            },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the rules as one undo step “Kayıt kuralları” when they differ.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersRules) -> CommandResult<LayersComposed> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_RULES,
        kentos_commands::CAD_LAYERS_RULES_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersRules,
) -> CommandResult<LayersComposed> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    let doc = &mut *cx.doc;
    match doc.set_layer_rules(&input.layer, input.rules, LABEL) {
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
