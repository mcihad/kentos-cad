//! `cad.layers.indexes` v1 (Dizinler, docs/adr/0758): a layer's indexes written as one undo step “Dizinler”; a layer
//! that already has them as given is left as it is. The shared cases are `fixtures/commands/v1/cad.layers.indexes.json`.
//!
//! The checks, in the contract's order: the expected revision; the layer is the drawing's, a layer, not drawn from a
//! service; the indexes fit its fields (`layer_indexes_problem`); a unique index that is new, or stricter than it was,
//! is not already broken by the layer's records (as PostgreSQL refuses CREATE UNIQUE INDEX over duplicate rows).

use std::collections::HashMap;

use kentos_commands::{
    CommandResult, CommandWarning, LayerIndex, LayerNode, LayerNodeType, LayersComposePlan,
    LayersComposed, LayersIndexes, index_values, layer_indexes_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Dizinler";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// Whether `x` asks more of the records than the layer's indexes did: unique where no unique one over the same fields
/// was, or taking empty values as the same where it did not.
fn stricter(x: &LayerIndex, before: &[LayerIndex]) -> bool {
    x.unique
        && !before
            .iter()
            .any(|b| b.fields == x.fields && b.unique && (b.nulls_equal || !x.nulls_equal))
}

/// The first set of values two or more of the layer's records hold under `x`, with their ids in the drawing's order.
pub fn duplicates(
    doc: &Document,
    node: &LayerNode,
    x: &LayerIndex,
) -> Option<(Vec<Option<String>>, Vec<String>)> {
    let fields: Vec<&kentos_commands::LayerField> = x
        .fields
        .iter()
        .filter_map(|k| node.fields.iter().find(|f| f.key() == k))
        .collect();
    if fields.len() != x.fields.len() {
        return None;
    }
    let mut seen: HashMap<Vec<Option<&str>>, Vec<String>> = HashMap::new();
    let mut first: Vec<Vec<Option<&str>>> = Vec::new();
    for (uid, e) in doc.by_layer_with_uids(&node.id) {
        let attrs = &e.base().attrs;
        let Some(values) = index_values(x, &fields, |n| attrs.get(n).map(String::as_str)) else {
            continue;
        };
        let ids = seen.entry(values.clone()).or_default();
        ids.push(uid.to_string());
        if ids.len() == 2 {
            first.push(values);
        }
    }
    let values = first.into_iter().next()?;
    let ids = seen.remove(&values)?;
    Some((
        values.into_iter().map(|v| v.map(str::to_owned)).collect(),
        ids,
    ))
}

/// The values of an index's fields as a sentence says them: (“101”, “5”), a single one bare; an empty one “boş”.
pub fn said_values(fields: &[&kentos_commands::LayerField], values: &[Option<String>]) -> String {
    let said: Vec<String> = fields
        .iter()
        .zip(values)
        .map(|(f, v)| {
            v.as_ref().map_or_else(
                || "boş".to_owned(),
                |v| format!("“{}”", kentos_commands::display_value(f, v)),
            )
        })
        .collect();
    if said.len() == 1 {
        said.into_iter().next().unwrap_or_default()
    } else {
        format!("({})", said.join(", "))
    }
}

fn check(doc: &Document, input: &LayersIndexes) -> Result<LayerNode, Stop> {
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
                "“{name}” bir katman grubu; dizinler yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{name}” servisten çizilir ve nesne tutmaz; dizini yoktur."),
            "layer",
        ));
    }
    if let Some((at, problem)) = layer_indexes_problem(&node.fields, &input.indexes) {
        let path = if at.is_empty() {
            "indexes".to_owned()
        } else {
            format!("indexes/{at}")
        };
        return Err(fail(codes::INVALID_INDEXES, problem, &path));
    }
    for (i, x) in input.indexes.iter().enumerate() {
        if !stricter(x, &node.indexes) {
            continue;
        }
        if let Some((values, uids)) = duplicates(doc, node, x) {
            let fields: Vec<&kentos_commands::LayerField> = x
                .fields
                .iter()
                .filter_map(|k| node.fields.iter().find(|f| f.key() == k))
                .collect();
            let mut refusal = error(
                codes::INDEX_NOT_UNIQUE,
                format!(
                    "“{}” dizini tekil olamaz: {} {} kayıtta var. Önce kayıtları düzeltin ya da dizini tekil yapmayın.",
                    x.name,
                    said_values(&fields, &values),
                    uids.len()
                ),
                Some(format!("indexes/{i}/unique")),
            );
            refusal.uids = uids;
            return Err(Stop::failed(refusal));
        }
    }
    Ok(node.clone())
}

/// A bound layer's indexes are KentOS's: the table does not get them (docs/adr/0758 §4).
fn warnings(doc: &Document, input: &LayersIndexes, node: &LayerNode) -> Vec<CommandWarning> {
    let bound = doc
        .settings()
        .sources
        .iter()
        .any(|s| s.layer == input.layer);
    if !bound || input.indexes.is_empty() || input.indexes == node.indexes {
        return Vec::new();
    }
    vec![CommandWarning {
        code: codes::INDEX_NOT_IN_SOURCE.into(),
        message: format!(
            "“{}” bir tabloya bağlı: dizinleri KentOS denetler, tabloda kurulmaz. Tablonun kendi tekil kısıtları yine sorulur.",
            node.name
        ),
        path: Some("indexes".into()),
    }]
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersIndexes) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layer as execute would leave it; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersIndexes) -> CommandResult<LayersComposePlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let warnings = warnings(cx.doc, input, &node);
    let changed = node.indexes != input.indexes;
    CommandResult::Completed {
        output: LayersComposePlan {
            node: LayerNode {
                indexes: input.indexes.clone(),
                ..node
            },
            changed,
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}

/// Writes the indexes as one undo step “Dizinler” when they differ.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersIndexes,
) -> CommandResult<LayersComposed> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_INDEXES,
        kentos_commands::CAD_LAYERS_INDEXES_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersIndexes,
) -> CommandResult<LayersComposed> {
    let node = match check(cx.doc, &input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let warnings = warnings(cx.doc, &input, &node);
    let doc = &mut *cx.doc;
    match doc.set_layer_indexes(&input.layer, input.indexes, LABEL) {
        Ok(changed) => CommandResult::Completed {
            output: LayersComposed {
                layer: input.layer,
                changed,
                revision: doc.revision().to_string(),
            },
            warnings,
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}
