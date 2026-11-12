//! `cad.layers.find` v1 (docs/adr/0311 §4): the layers found by a name
//! pattern (AutoCAD's `wcmatch`, `kentos_commands::pattern_matches`) and
//! their properties, AutoCAD's layer filters as one query. The Katmanlar
//! panel's search and its Süzgeç ▾ ask [`find`]; Python and MCP run the
//! command as it is. It writes nothing. The shared cases are
//! `fixtures/commands/v1/cad.layers.find.json`
//! (`scripts/fixtures/layer_find_cases.py`).

use kentos_commands::{
    CommandResult, LayerFound, LayerNode, LayerNodeType, LayersFind, LayersFound, fold_name,
    layer_color_ok, pattern_matches, pattern_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersFind) -> Result<(), Stop> {
    if let Some(pattern) = input.name.as_deref()
        && let Some(problem) = pattern_problem(pattern)
    {
        return Err(fail(
            codes::INVALID_PATTERN,
            format!(
                "“{pattern}” bir katman adı örüntüsü değil: {problem}. * herhangi bir dizi, ? tek karakter, [..] küme olur."
            ),
            "name",
        ));
    }
    if let Some(color) = input.color.as_deref()
        && !layer_color_ok(color)
    {
        return Err(fail(
            codes::INVALID_COLOR,
            format!(
                "“{color}” bir katman rengi değil; #rrggbb ya da fg, fg-dim, ink, paper yazın."
            ),
            "color",
        ));
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    if let Some(group) = input.group.as_deref()
        && doc.layers().get(group).is_none()
    {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!(
                "“{group}” kimlikli katman ya da grup çizimde yok. Var olan bir düğümün kimliğini verin."
            ),
            "group",
        ));
    }
    Ok(())
}

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersFind) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The layers; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersFind) -> CommandResult<LayersFound> {
    answer(cx.doc, input)
}

/// The layers. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersFind) -> CommandResult<LayersFound> {
    answer(cx.doc, &input)
}

fn answer(doc: &Document, input: &LayersFind) -> CommandResult<LayersFound> {
    if let Err(stop) = check(doc, input) {
        return stop.into();
    }
    CommandResult::Completed {
        output: LayersFound {
            layers: find(doc, input),
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

fn leaves<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
    for n in nodes {
        if n.kind == LayerNodeType::Layer {
            out.push(n);
        }
        leaves(&n.children, out);
    }
}

/// The layers `input` asks for, in the tree's order (its checks passed).
pub fn find(doc: &Document, input: &LayersFind) -> Vec<LayerFound> {
    let tree = doc.layers();
    let mut all = Vec::new();
    match input.group.as_deref().and_then(|g| tree.get(g)) {
        Some(group) => leaves(std::slice::from_ref(group), &mut all),
        None => leaves(tree.nodes(), &mut all),
    }
    let color = input.color.as_deref().map(fold_name);
    all.into_iter()
        .filter_map(|n| {
            // The store's layer index counts them: no walk over every object.
            let objects = u32::try_from(doc.count(&n.id)).unwrap_or(u32::MAX);
            let found = LayerFound {
                id: n.id.clone(),
                name: n.name.clone(),
                path: tree.path(&n.id),
                objects,
                color: n.style.color.clone(),
                line_type: n.style.line_type,
                line_weight: n.style.line_weight,
                on: tree.is_on(&n.id),
                frozen: tree.is_frozen(&n.id),
                locked: tree.is_locked(&n.id),
                plot: tree.printed(&n.id),
            };
            let empty = objects == 0 && n.service.is_none();
            let keep = input
                .name
                .as_deref()
                .is_none_or(|p| pattern_matches(p, &n.name))
                && color
                    .as_deref()
                    .is_none_or(|c| fold_name(&n.style.color) == c)
                && input.line_type.is_none_or(|t| t == n.style.line_type)
                && input
                    .line_weight
                    .is_none_or(|w| (w - n.style.line_weight).abs() < 1e-9)
                && input.on.is_none_or(|v| v == found.on)
                && input.frozen.is_none_or(|v| v == found.frozen)
                && input.locked.is_none_or(|v| v == found.locked)
                && input.plot.is_none_or(|v| v == found.plot)
                && input.empty.is_none_or(|v| v == empty);
            keep.then_some(found)
        })
        .collect()
}
