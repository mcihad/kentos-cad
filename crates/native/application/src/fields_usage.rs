//! `cad.fields.usage` v1 (docs/adr/0414 §3): where a layer's field is named. Writes nothing; the
//! places and their words are `kentos_forms::field_refs`'s. The shared cases are
//! `fixtures/commands/v1/cad.fields.usage.json`.
//!
//! The checks, in order: the layer is the drawing's, a layer, not drawn from a service; the field is
//! the layer's, by its exact name.

use kentos_commands::{CommandResult, FieldsUsage, FieldsUsed, LayerField, LayerNodeType};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &FieldsUsage) -> Result<LayerField, Stop> {
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
                "“{name}” bir katman grubu; alanlar yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{name}” servisten çizilir ve nesne tutmaz; alanı yoktur."),
            "layer",
        ));
    }
    node.fields
        .iter()
        .find(|f| f.name == input.field)
        .cloned()
        .ok_or_else(|| {
            fail(
                codes::FIELD_NOT_FOUND,
                format!("“{name}” katmanında “{}” alanı yok.", input.field),
                "field",
            )
        })
}

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &FieldsUsage) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The places; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &FieldsUsage) -> CommandResult<FieldsUsed> {
    answer(cx.doc, input)
}

/// The places. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FieldsUsage) -> CommandResult<FieldsUsed> {
    answer(cx.doc, &input)
}

fn answer(doc: &Document, input: &FieldsUsage) -> CommandResult<FieldsUsed> {
    let field = match check(doc, input) {
        Ok(f) => f,
        Err(stop) => return stop.into(),
    };
    CommandResult::Completed {
        output: FieldsUsed {
            layer: input.layer.clone(),
            field: input.field.clone(),
            uses: kentos_forms::field_refs::uses(
                doc.settings(),
                doc.layers().nodes(),
                &input.layer,
                &field,
            ),
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
