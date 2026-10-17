//! `cad.schema.describe` v1 (docs/adr/0443 §1): the project's data schema a page at a time, writing
//! nothing. The shared cases are `fixtures/commands/v1/cad.schema.describe.json`.
//!
//! The checks, in order: the limit 1–100; the cursor a decimal offset this listing could give; the layer
//! the drawing's. Listed in the order layers (the tree's, layers only), forms, value lists, relations
//! (the project's orders); a query keeps those whose id or name holds its words (Turkish folded).

use kentos_contracts::{
    CommandResult, DESCRIBED, LayerNodeType, MOST_DESCRIBED, SchemaDescribe, SchemaDescribed,
    SchemaItem, SchemaKind,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A text lower-cased the Turkish way (`I` ı, `İ` i; a letter whose lower case is more than one keeps
/// itself).
fn folded(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'I' => 'ı',
            'İ' => 'i',
            _ => {
                let mut low = c.to_lowercase();
                match (low.next(), low.next()) {
                    (Some(l), None) => l,
                    _ => c,
                }
            }
        })
        .collect()
}

fn check(doc: &Document, input: &SchemaDescribe) -> Result<(usize, usize), Stop> {
    let limit = input.limit.unwrap_or(DESCRIBED);
    if limit == 0 || limit > MOST_DESCRIBED {
        return Err(fail(
            codes::INVALID_COUNT,
            format!("Bir sayfada 1 ile {MOST_DESCRIBED} arasında öğe listelenir; {limit} verildi."),
            "limit",
        ));
    }
    let offset = match &input.cursor {
        None => 0,
        Some(c) => c
            .parse::<usize>()
            .ok()
            .filter(|n| c == &n.to_string())
            .ok_or_else(|| {
                fail(
                    codes::INVALID_CURSOR,
                    format!(
                        "“{c}” bu listenin sayfa imleci değil; bir önceki cevabın next'ini verin."
                    ),
                    "cursor",
                )
            })?,
    };
    if let Some(layer) = &input.layer
        && doc
            .layers()
            .get(layer)
            .is_none_or(|n| n.kind != LayerNodeType::Layer)
    {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!(
                "“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
            ),
            "layer",
        ));
    }
    Ok((offset, limit as usize))
}

/// Everything found, in the listing's order.
fn found(doc: &Document, input: &SchemaDescribe) -> Vec<SchemaItem> {
    let wanted = |k: SchemaKind| input.kinds.is_empty() || input.kinds.contains(&k);
    let words = input
        .query
        .as_deref()
        .map(folded)
        .filter(|q| !q.trim().is_empty());
    let matches = |id: &str, name: &str| {
        words
            .as_ref()
            .is_none_or(|q| folded(id).contains(q.as_str()) || folded(name).contains(q.as_str()))
    };
    let layer = input.layer.as_deref();
    let settings = doc.settings();
    let mut out = Vec::new();
    if wanted(SchemaKind::Layers) {
        for n in doc.layers().leaves() {
            if n.kind != LayerNodeType::Layer
                || layer.is_some_and(|l| l != n.id)
                || !matches(&n.id, &n.name)
            {
                continue;
            }
            out.push(SchemaItem {
                kind: SchemaKind::Layers,
                id: n.id.clone(),
                name: n.name.clone(),
                layer: None,
                child: None,
                count: Some(n.fields.len() as u32),
                version: None,
                detail: input
                    .expand
                    .then(|| serde_json::to_value(&n.fields).unwrap_or_default()),
            });
        }
    }
    if wanted(SchemaKind::Forms) {
        for f in &settings.forms {
            if layer.is_some_and(|l| l != f.layer) || !matches(&f.id, &f.name) {
                continue;
            }
            out.push(SchemaItem {
                kind: SchemaKind::Forms,
                id: f.id.clone(),
                name: f.name.clone(),
                layer: Some(f.layer.clone()),
                child: None,
                count: Some(f.draft.walk().len() as u32),
                version: f.releases.last().map(|r| r.version),
                detail: input.expand.then(|| {
                    serde_json::json!({
                        "draft": f.draft,
                        "releases": f.releases.iter().map(|r| serde_json::json!({ "version": r.version, "digest": r.digest })).collect::<Vec<_>>(),
                        "enforce": f.enforce,
                    })
                }),
            });
        }
    }
    if wanted(SchemaKind::Domains) {
        // A layer's lists: those its fields link.
        let linked: Vec<String> = layer
            .and_then(|l| doc.layers().get(l))
            .map(|n| n.fields.iter().filter_map(|f| f.domain.clone()).collect())
            .unwrap_or_default();
        for d in &settings.domains {
            if (layer.is_some() && !linked.contains(&d.id)) || !matches(&d.id, &d.name) {
                continue;
            }
            out.push(SchemaItem {
                kind: SchemaKind::Domains,
                id: d.id.clone(),
                name: d.name.clone(),
                layer: None,
                child: None,
                count: Some(d.values.as_ref().map_or(0, |v| v.len() as u32)),
                version: None,
                detail: input
                    .expand
                    .then(|| serde_json::to_value(d).unwrap_or_default()),
            });
        }
    }
    if wanted(SchemaKind::Relations) {
        for r in &settings.relations {
            if layer.is_some_and(|l| l != r.parent && l != r.child) || !matches(&r.id, &r.name) {
                continue;
            }
            out.push(SchemaItem {
                kind: SchemaKind::Relations,
                id: r.id.clone(),
                name: r.name.clone(),
                layer: Some(r.parent.clone()),
                child: Some(r.child.clone()),
                count: None,
                version: None,
                detail: input
                    .expand
                    .then(|| serde_json::to_value(r).unwrap_or_default()),
            });
        }
    }
    out
}

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &SchemaDescribe) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The page; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &SchemaDescribe) -> CommandResult<SchemaDescribed> {
    answer(cx.doc, input)
}

/// The page. Writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SchemaDescribe,
) -> CommandResult<SchemaDescribed> {
    answer(cx.doc, &input)
}

fn answer(doc: &Document, input: &SchemaDescribe) -> CommandResult<SchemaDescribed> {
    let (offset, limit) = match check(doc, input) {
        Ok(x) => x,
        Err(stop) => return stop.into(),
    };
    let all = found(doc, input);
    let total = all.len();
    if offset > total {
        return Stop::failed(error(
            codes::INVALID_CURSOR,
            format!(
                "“{offset}” bu listenin sayfa imleci değil: liste {total} öğe; bir önceki cevabın next'ini verin."
            ),
            Some("cursor".to_owned()),
        ))
        .into();
    }
    let end = (offset + limit).min(total);
    CommandResult::Completed {
        output: SchemaDescribed {
            items: all[offset..end].to_vec(),
            total: total as u32,
            next: (end < total).then(|| end.to_string()),
        },
        warnings: Vec::new(),
    }
}
