//! `cad.datasets.edit` v1 (docs/adr/0633 §1): a layer's description as a dataset written as one undo step “Veri
//! kümesi bilgisi”; a layer that already has it as given is left as it is, none takes it away. Its derivation record
//! is not the command's: the processing run that made the layer writes it.
//!
//! The checks, in the contract's order: the description's rules; the expected revision; the layer is the drawing's
//! and a layer.

use kentos_commands::{
    CommandResult, DatasetInfo, DatasetsEdit, DatasetsEditPlan, DatasetsEdited, LayerNode,
    LayerNodeType, dataset_problem,
};
use kentos_domain::{Document, LayerInfo};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Veri kümesi bilgisi";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The description as the layer will keep it: an empty one is none.
fn kept(input: &DatasetsEdit) -> Option<DatasetInfo> {
    input.dataset.clone().filter(|d| !d.is_empty())
}

fn check(doc: &Document, input: &DatasetsEdit) -> Result<LayerNode, Stop> {
    if let Some(problem) = kept(input).as_ref().and_then(dataset_problem) {
        return Err(fail(codes::INVALID_DATASET, problem, "dataset"));
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
                "“{}” bir katman grubu; veri kümesi bilgisi yalnız katmanın olur. Grubun bir katmanını verin.",
                node.name
            ),
            "layer",
        ));
    }
    Ok(node.clone())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &DatasetsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &DatasetsEdit) -> CommandResult<DatasetsEditPlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let dataset = kept(input);
    let changed = node.dataset != dataset;
    CommandResult::Completed {
        output: DatasetsEditPlan {
            node: LayerNode { dataset, ..node },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the description as one undo step “Veri kümesi bilgisi” when it differs.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: DatasetsEdit,
) -> CommandResult<DatasetsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_DATASETS_EDIT,
        kentos_commands::CAD_DATASETS_EDIT_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the document, all or nothing, and journals it.
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: DatasetsEdit,
) -> CommandResult<DatasetsEdited> {
    let node = match check(cx.doc, &input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let info = LayerInfo {
        actions: node.actions.clone(),
        card: node.card.clone(),
        dataset: kept(&input),
        derivation: node.derivation.clone(),
    };
    let doc = &mut *cx.doc;
    match doc.set_layer_info(&input.layer, info, LABEL) {
        Ok(changed) => CommandResult::Completed {
            output: DatasetsEdited {
                layer: input.layer,
                changed,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::INVALID_DATASET, refusal.0, "dataset").into(),
    }
}
