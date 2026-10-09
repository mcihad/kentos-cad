//! `cad.layers.filter` v1 (docs/adr/0211 §5): a layer's filter written, or
//! taken away, as one undo step “Katman süzgeci”; a layer that already has it
//! as given is left as it is. The answer counts the layer's objects that pass
//! it. The desktop's handler over the native document; the web's is
//! `apps/web/src/product/layersFilter.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.layers.filter.json`.
//!
//! The checks, in the contract's order: the filter's rules; its condition
//! compiles (`$sıra` and `$ölçek` refused); the expected revision; the layer
//! is the drawing's, a layer, not drawn from a service.

use kentos_contracts::{
    CommandResult, LayerFilter, LayerNode, LayerNodeType, LayersFilter, LayersFilterPlan,
    LayersFiltered,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;
use crate::layer_filter::{CompiledFilter, compile_filter};

/// The undo step's name.
pub const LABEL: &str = "Katman süzgeci";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

fn check(
    doc: &Document,
    input: &LayersFilter,
) -> Result<(LayerNode, Option<CompiledFilter>), Stop> {
    let compiled = match &input.filter {
        None => None,
        Some(f) => {
            if let Some(problem) = f.problem() {
                return Err(fail(
                    codes::INVALID_FILTER,
                    format!("Katmanın süzgeci: {problem}."),
                    "filter",
                ));
            }
            Some(compile_filter(f).map_err(|why| {
                fail(
                    codes::INVALID_EXPRESSION,
                    format!("Süzgecin ifadesi: {why}"),
                    "filter/expression",
                )
            })?)
        }
    };
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
                "“{name}” bir katman grubu; süzgeç yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() && input.filter.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!(
                "“{name}” servisten çizilir ve nesne tutmaz; süzgeç nesneleri olan katmanın olur."
            ),
            "layer",
        ));
    }
    Ok((node.clone(), compiled))
}

/// How many of the layer's objects pass `filter`, and how many it has.
pub fn count(doc: &Document, layer: &str, filter: Option<&CompiledFilter>) -> (u32, u32) {
    let list: Vec<_> = doc
        .by_layer_with_uids(layer)
        .map(|(uid, e)| (e, kentos_contracts::EntityId(*uid.as_bytes())))
        .collect();
    let total = list.len() as u32;
    let Some(filter) = filter else {
        return (total, total);
    };
    let shapes: Vec<_> = if filter.reads_geometry() {
        list.iter()
            .map(|(e, _)| crate::geometry::shape(e))
            .collect()
    } else {
        Vec::new()
    };
    let names = |id: &str| {
        doc.layers()
            .get(id)
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    };
    let passed = filter
        .passes(&list, &names, |i| shapes.get(i))
        .into_iter()
        .filter(|p| *p)
        .count() as u32;
    (passed, total)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersFilter) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, and how many objects would pass; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersFilter) -> CommandResult<LayersFilterPlan> {
    let (node, compiled) = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let changed = node.filter != input.filter;
    let (passed, total) = count(cx.doc, &node.id, compiled.as_ref());
    CommandResult::Completed {
        output: LayersFilterPlan {
            node: LayerNode {
                filter: input.filter.clone(),
                ..node
            },
            changed,
            passed,
            total,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the filter as one undo step “Katman süzgeci” when it differs.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersFilter,
) -> CommandResult<LayersFiltered> {
    let compiled = match check(cx.doc, &input) {
        Ok((_, c)) => c,
        Err(stop) => return stop.into(),
    };
    let (passed, total) = count(cx.doc, &input.layer, compiled.as_ref());
    let doc = &mut *cx.doc;
    let filter: Option<LayerFilter> = input.filter;
    match doc.set_layer_filter(&input.layer, filter, LABEL) {
        Ok(changed) => CommandResult::Completed {
            output: LayersFiltered {
                layer: input.layer,
                changed,
                passed,
                total,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}
