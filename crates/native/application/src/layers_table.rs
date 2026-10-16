//! `cad.layers.table` v1 (docs/adr/0413 §2): a layer's table rules (Koşullu biçim) written as one undo
//! step “Koşullu biçim”; a layer that already has them as given is left as it is. The shared cases
//! are `fixtures/commands/v1/cad.layers.table.json`.
//!
//! The checks, in the contract's order: the rules' rule; the expected revision; the layer is the
//! drawing's, a layer, not drawn from a service; each rule's field is the layer's and its condition
//! compiles as the table's filter does (`$sıra` and `$ölçek` refused).

use kentos_contracts::{
    CommandResult, LayerNode, LayerNodeType, LayersComposePlan, LayersComposed, LayersTable,
    table_styles_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::expression_unit::reading_unit;

/// The undo step's name.
pub const LABEL: &str = "Koşullu biçim";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersTable) -> Result<LayerNode, Stop> {
    if let Some((i, problem)) = table_styles_problem(&input.styles) {
        return Err(fail(
            codes::INVALID_TABLE_STYLES,
            problem,
            &format!("styles/{i}"),
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
                "“{name}” bir katman grubu; koşullu biçim yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{name}” servisten çizilir ve nesne tutmaz; tablosu yoktur."),
            "layer",
        ));
    }
    let unit = reading_unit(doc.settings());
    for (i, s) in input.styles.iter().enumerate() {
        if let Some(field) = &s.field
            && !node.fields.iter().any(|f| f.name == *field)
        {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{name}” katmanında “{field}” alanı yok."),
                &format!("styles/{i}/field"),
            ));
        }
        compile_condition(&s.condition, unit).map_err(|why| {
            fail(
                codes::INVALID_EXPRESSION,
                format!("{}. kuralın koşulu: {why}", i + 1),
                &format!("styles/{i}/condition"),
            )
        })?;
    }
    Ok(node.clone())
}

/// A rule's condition compiled as the table's filter is (the layer's fields by name, the project's
/// reading unit); a rule does not read `$sıra` or `$ölçek`, which the table's order and the drawing's
/// scale would change.
pub fn compile_condition(
    text: &str,
    unit: kentos_expression::Unit,
) -> Result<kentos_expression::Expr, String> {
    let schema = kentos_expression::Schema {
        unit,
        ..kentos_expression::Schema::default()
    };
    let e = kentos_expression::compile_with(text, &schema).map_err(|e| e.text())?;
    if e.needs.index {
        return Err(
            "Koşullu biçimde $sıra kullanılamaz: biçim tablonun sırasına göre değişmemeli.".into(),
        );
    }
    if e.needs.scale {
        return Err(
            "Koşullu biçimde $ölçek kullanılamaz: biçim çizimin ölçeğine göre değişmemeli.".into(),
        );
    }
    Ok(e)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersTable) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersTable) -> CommandResult<LayersComposePlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let changed = node.table_styles != input.styles;
    CommandResult::Completed {
        output: LayersComposePlan {
            node: LayerNode {
                table_styles: input.styles.clone(),
                ..node
            },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the rules as one undo step “Koşullu biçim” when they differ.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersTable) -> CommandResult<LayersComposed> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_LAYERS_TABLE,
        kentos_contracts::CAD_LAYERS_TABLE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersTable,
) -> CommandResult<LayersComposed> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    let doc = &mut *cx.doc;
    match doc.set_layer_table_styles(&input.layer, input.styles, LABEL) {
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
