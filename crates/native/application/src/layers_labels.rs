//! `cad.layers.labels` v1 (docs/adr/0212 §5): a layer's labelling (its
//! single label's style and how it is labelled) written as one undo step
//! “Etiketler”; a layer that already has it as given is left as it is. The
//! desktop's handler over the native document; the web's is
//! `apps/web/src/product/layersLabels.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.layers.labels.json`.
//!
//! The checks, in the contract's order: the style's and the labelling's
//! rules; their texts' and conditions' expressions compile (`$sıra` and
//! `$ölçek` refused); the expected revision; the layer is the drawing's, a
//! layer, not drawn from a service.

use kentos_contracts::{
    CommandResult, LayerNode, LayerNodeType, LayerStyle, LayersLabelled, LayersLabels,
    LayersLabelsPlan, layer_labels_problem, style_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;
use crate::label_texts::compile_label_expression;

/// The undo step's name.
pub const LABEL: &str = "Etiketler";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

/// The expressions' compiling: a text or a condition that does not compile answers.
fn expression(source: Option<&str>, what: String, path: String) -> Result<(), Stop> {
    match source {
        Some(s) => compile_label_expression(s)
            .map(|_| ())
            .map_err(|why| fail(codes::INVALID_EXPRESSION, format!("{what}: {why}"), &path)),
        None => Ok(()),
    }
}

/// The layer and its style as execute would leave it.
fn check(doc: &Document, input: &LayersLabels) -> Result<(LayerNode, LayerStyle), Stop> {
    if let Some(Some(style)) = &input.label
        && let Some(p) = style_problem(style)
    {
        return Err(fail(
            codes::INVALID_LABELS,
            format!("Etiketin stili: {p}."),
            "label",
        ));
    }
    if let Some(Some(labels)) = &input.labels
        && let Some(p) = layer_labels_problem(labels)
    {
        return Err(fail(
            codes::INVALID_LABELS,
            format!("Etiketleme: {p}."),
            "labels",
        ));
    }
    if let Some(Some(style)) = &input.label {
        expression(
            style.text.as_deref(),
            "Etiketin metni".into(),
            "label/text".into(),
        )?;
    }
    if let Some(Some(labels)) = &input.labels {
        for (i, c) in labels.classes.iter().enumerate() {
            expression(
                c.when.as_deref(),
                format!("“{}” sınıfının koşulu", c.name),
                format!("labels/classes/{i}/when"),
            )?;
            expression(
                c.style.text.as_deref(),
                format!("“{}” sınıfının metni", c.name),
                format!("labels/classes/{i}/style/text"),
            )?;
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
    if node.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{name}” bir katman grubu; etiketleme yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    let gives = matches!(input.label, Some(Some(_))) || matches!(input.labels, Some(Some(_)));
    if node.service.is_some() && gives {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!(
                "“{name}” servisten çizilir ve nesne tutmaz; etiketleme nesneleri olan katmanın olur."
            ),
            "layer",
        ));
    }
    let mut style = node.style.clone();
    if let Some(label) = &input.label {
        style.label = label.clone();
    }
    if let Some(labels) = &input.labels {
        style.labels = labels.clone();
    }
    Ok((node.clone(), style))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersLabels) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersLabels) -> CommandResult<LayersLabelsPlan> {
    let (node, style) = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let changed = node.style != style;
    CommandResult::Completed {
        output: LayersLabelsPlan {
            node: LayerNode { style, ..node },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the labelling as one undo step “Etiketler” when it differs.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersLabels,
) -> CommandResult<LayersLabelled> {
    let style = match check(cx.doc, &input) {
        Ok((_, style)) => style,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let changed = doc.set_layer_style(&input.layer, style, LABEL);
    CommandResult::Completed {
        output: LayersLabelled {
            layer: input.layer,
            changed,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
