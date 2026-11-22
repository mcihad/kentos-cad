//! `cad.layers.data` v1 (docs/adr/9101 §4): a data layer added (“Veri katmanı ekle”), changed (“Veri katmanını
//! değiştir”: its name, its table's place, its fields) or removed (“Veri katmanını kaldır”), one undo step each. A data
//! layer holds no objects: its rows stay in the source and are read as the view, the table or a tool needs them
//! (plan/veri.md VY-03, VY-04). The host reads the table's catalog and gives what it found; the command reads no source.
//!
//! The checks, in the contract's order: the layer an update or a removal needs; the name; the table on an add; the
//! table's and the fields' rules; the expected revision; the layer is the drawing's, a layer, a data layer.

use kentos_commands::{
    CommandResult, LayerDataOperation, LayerNode, LayerNodeType, LayersData, LayersDataDone,
    LayersDataPlan, fields_problem,
};
use kentos_domain::{Document, NewLayer, ServedBy};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_blank};
use crate::codes;

/// The undo step's name.
pub fn label(op: LayerDataOperation) -> &'static str {
    match op {
        LayerDataOperation::Add => "Veri katmanı ekle",
        LayerDataOperation::Update => "Veri katmanını değiştir",
        LayerDataOperation::Remove => "Veri katmanını kaldır",
    }
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// `update`, `remove`: the layer; none for `add`.
fn check(doc: &Document, input: &LayersData) -> Result<Option<LayerNode>, Stop> {
    use LayerDataOperation as Op;
    let op = input.operation;
    let changes = matches!(op, Op::Update | Op::Remove);
    if changes && input.layer.as_deref().is_none_or(str::is_empty) {
        return Err(fail(
            codes::NO_LAYER,
            "Değiştirilecek ya da kaldırılacak katmanın kimliğini (layer) verin.".into(),
            "layer",
        ));
    }
    if (op == Op::Add || input.name.is_some()) && is_blank(input.name.as_deref().unwrap_or("")) {
        return Err(fail(
            codes::EMPTY_NAME,
            "Katmanın adı boş olamaz; bir ad verin.".into(),
            "name",
        ));
    }
    if op == Op::Add && input.data.is_none() {
        return Err(fail(
            codes::NO_DATA,
            "Eklenecek veri tablosu (data) verilmedi: sürücüyü, kaynağın adresini, tabloyu ve anahtarı verin."
                .into(),
            "data",
        ));
    }
    if let Some(problem) = input.data.as_ref().and_then(|d| d.problem()) {
        return Err(fail(codes::INVALID_DATA, problem, "data"));
    }
    if let Some(problem) = input
        .fields
        .as_deref()
        .filter(|f| !f.is_empty())
        .and_then(fields_problem)
    {
        return Err(fail(codes::INVALID_FIELDS, problem, "fields"));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    if !changes {
        return Ok(None);
    }
    let id = input.layer.as_deref().unwrap_or_default();
    let Some(n) = doc.layers().get(id) else {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            "layer",
        ));
    };
    if n.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{}” bir katman grubu; veri tablosu yalnız katmanın olur.",
                n.name
            ),
            "layer",
        ));
    }
    if n.data.is_none() {
        return Err(fail(
            codes::NOT_A_DATA_LAYER,
            format!(
                "“{}” bir veri katmanı değil; bu komut yalnız veri katmanını değiştirir ya da kaldırır.",
                n.name
            ),
            "layer",
        ));
    }
    Ok(Some(n.clone()))
}

/// The new layer of `add`, as the tree would make it: typed by what its table draws.
fn new_layer(input: &LayersData) -> NewLayer {
    let data = input.data.clone();
    NewLayer {
        geometry: data.as_ref().and_then(|d| d.family()),
        data,
        fields: input.fields.clone().unwrap_or_default(),
        ..NewLayer::layer(input.name.as_deref().unwrap_or_default().trim())
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersData) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersData) -> CommandResult<LayersDataPlan> {
    let node = match check(cx.doc, input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let node = match input.operation {
        LayerDataOperation::Add => Some(cx.doc.layers().preview(new_layer(input))),
        LayerDataOperation::Update => node.map(|n| LayerNode {
            name: input
                .name
                .as_deref()
                .map_or_else(|| n.name.clone(), |s| s.trim().to_owned()),
            geometry: input
                .data
                .as_ref()
                .map_or(n.geometry, |d| d.family()),
            data: input.data.clone().or_else(|| n.data.clone()),
            fields: input.fields.clone().unwrap_or_else(|| n.fields.clone()),
            ..n
        }),
        LayerDataOperation::Remove => None,
    };
    CommandResult::Completed {
        output: LayersDataPlan {
            node,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the change as one undo step named after the operation.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersData) -> CommandResult<LayersDataDone> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_DATA,
        kentos_commands::CAD_LAYERS_DATA_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the document, all or nothing, and journals it.
/// [`execute`]'s write without its own journal row: a command that adds a data layer inside its own step (an export's
/// `addToMap`, docs/adr/9106).
pub(crate) fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersData,
) -> CommandResult<LayersDataDone> {
    write_activated(cx, input, false)
}

/// [`write_execute`], the layer added made active in the same step when `activate` (undo makes the one before active
/// again): a layer moved into a source takes the active role of the one it replaces (docs/adr/9107 §8).
pub(crate) fn write_activated(
    cx: &mut ExecutionContext<'_>,
    input: LayersData,
    activate: bool,
) -> CommandResult<LayersDataDone> {
    let node = match check(cx.doc, &input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let label = label(input.operation);
    let written = match input.operation {
        LayerDataOperation::Add => doc.add_layer_at(
            new_layer(&input),
            input.parent.as_deref(),
            input.index.map(|i| i as usize),
            Some(label),
            activate,
        ),
        LayerDataOperation::Update => {
            let Some(n) = node else {
                return fail(codes::LAYER_NOT_FOUND, "katman yok".into(), "layer").into();
            };
            let id = n.id.clone();
            let next = ServedBy {
                data: input.data.clone().or_else(|| n.data.clone()),
                ..ServedBy::default()
            };
            let geometry = input.data.as_ref().map(|d| d.family());
            doc.transact(label, |doc| {
                doc.set_layer_service(&id, next, input.name.as_deref(), label)?;
                if let Some(fields) = &input.fields {
                    doc.set_layer_fields(&id, fields.clone(), &[])?;
                }
                if let Some(g) = geometry {
                    doc.set_layer_geometry(&id, g, label)?;
                }
                Ok(())
            })
            .map(|()| id)
        }
        LayerDataOperation::Remove => {
            let id = node.map(|n| n.id).unwrap_or_default();
            doc.transact(label, |doc| doc.remove_layer(&id).map(|_| ()))
                .map(|()| id)
        }
    };
    match written {
        Ok(layer) => CommandResult::Completed {
            output: LayersDataDone {
                layer,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}
