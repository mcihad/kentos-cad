//! `cad.layers.service` v1 (docs/adr/0208 §15): one change of the layer
//! tree's service layers as one undo step named after the operation: a
//! layer drawn from a map service added (“Harita servisi ekle”), a layer for
//! a service's objects added with its fields (“Veri katmanı ekle”), one
//! changed (“Servis katmanını değiştir”: its name, service or source) or a
//! service layer removed (“Servis katmanını sil”). The project's connections
//! the input brings are added or replace those of the same id first;
//! project settings are not undo steps, so they stay after an undo. The
//! desktop's handler over the native document; the web's is
//! `apps/web/src/product/layersService.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.layers.service.json`.
//!
//! The checks, in the contract's order: the layer an update or a removal
//! needs; the name; what the operation needs (a service, a source, not
//! both); the service's, the source's, the fields' and the connections'
//! rules; the expected revision; the layer is the drawing's, a layer, one of
//! the kind the operation changes; a connection named is the project's
//! (after the input's).

use kentos_contracts::{
    CommandResult, LayerNode, LayerNodeType, LayerServiceOperation, LayersService,
    LayersServicePlan, LayersServiced, ProjectSettings, ServiceConnection, connections_problem,
    fields_problem,
};
use kentos_domain::{Document, NewLayer, ServedBy};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_blank};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// The undo step's name (docs/adr/0208 §15).
pub fn label(op: LayerServiceOperation) -> &'static str {
    match op {
        LayerServiceOperation::Add => "Harita servisi ekle",
        LayerServiceOperation::AddFeed => "Veri katmanı ekle",
        LayerServiceOperation::Update => "Servis katmanını değiştir",
        LayerServiceOperation::Remove => "Servis katmanını sil",
    }
}

struct Checked {
    /// The project's connections after the input's.
    connections: Vec<ServiceConnection>,
    /// `update`, `remove`: the layer.
    node: Option<LayerNode>,
}

/// The project's connections with the input's added, or replacing those of the same id in their place.
fn merged(
    own: &[ServiceConnection],
    given: Option<&[ServiceConnection]>,
) -> Vec<ServiceConnection> {
    let mut out = own.to_vec();
    for c in given.unwrap_or_default() {
        match out.iter().position(|o| o.id == c.id) {
            Some(at) => out[at] = c.clone(),
            None => out.push(c.clone()),
        }
    }
    out
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersService) -> Result<Checked, Stop> {
    use LayerServiceOperation as Op;
    let op = input.operation;
    let changes = matches!(op, Op::Update | Op::Remove);
    if changes && input.layer.as_deref().is_none_or(str::is_empty) {
        return Err(fail(
            codes::NO_LAYER,
            "Değiştirilecek ya da silinecek katmanın kimliğini (layer) verin.".into(),
            "layer",
        ));
    }
    if (matches!(op, Op::Add | Op::AddFeed) || input.name.is_some())
        && is_blank(input.name.as_deref().unwrap_or(""))
    {
        return Err(fail(
            codes::EMPTY_NAME,
            "Katmanın adı boş olamaz; bir ad verin.".into(),
            "name",
        ));
    }
    if op == Op::Add && input.service.is_none() {
        return Err(fail(
            codes::NO_SERVICE,
            "Eklenecek servis (service) verilmedi; servisin türünü ve adresini verin.".into(),
            "service",
        ));
    }
    if op == Op::AddFeed && input.feed.is_none() {
        return Err(fail(
            codes::NO_FEED,
            "Veri katmanının kaynağı (feed) verilmedi; servisin türünü, adresini ve tür adını verin.".into(),
            "feed",
        ));
    }
    if input.service.is_some() && input.feed.is_some() {
        return Err(fail(
            codes::SERVICE_AND_FEED,
            "Bir katman ya servisten çizilir ya nesnelerini bir kaynaktan alır; service ile feed birlikte verilmez.".into(),
            "feed",
        ));
    }
    if let Some(problem) = input.service.as_ref().and_then(|s| s.problem()) {
        return Err(fail(codes::INVALID_SERVICE, problem, "service"));
    }
    if let Some(problem) = input.feed.as_ref().and_then(|f| f.problem()) {
        return Err(fail(codes::INVALID_FEED, problem, "feed"));
    }
    if let Some(problem) = input
        .fields
        .as_deref()
        .filter(|f| !f.is_empty())
        .and_then(fields_problem)
    {
        return Err(fail(codes::INVALID_FIELDS, problem, "fields"));
    }
    let connections = merged(&doc.settings().connections, input.connections.as_deref());
    if input.connections.is_some()
        && let Some(problem) = connections_problem(&connections)
    {
        return Err(fail(codes::INVALID_CONNECTION, problem, "connections"));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut node = None;
    if changes {
        let id = input.layer.as_deref().unwrap_or_default();
        let Some(n) = doc.layers().get(id) else {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                "layer",
            ));
        };
        let name = &n.name;
        if n.kind != LayerNodeType::Layer {
            return Err(fail(
                codes::NOT_A_LAYER,
                format!("“{name}” bir katman grubu; servis ve veri kaynağı yalnız katmanın olur."),
                "layer",
            ));
        }
        if (op == Op::Remove || input.service.is_some()) && n.service.is_none() {
            let message = if op == Op::Remove {
                format!(
                    "“{name}” bir servis katmanı değil; bu komut yalnız servis katmanını siler. Katmanı Katmanlar panelinden silin."
                )
            } else {
                format!(
                    "“{name}” bir servis katmanı değil; servisi yalnız servis katmanının değişir."
                )
            };
            return Err(fail(codes::NOT_A_SERVICE_LAYER, message, "layer"));
        }
        if input.feed.is_some() && n.feed.is_none() {
            return Err(fail(
                codes::NOT_A_SERVICE_LAYER,
                format!(
                    "“{name}” katmanının veri kaynağı yok; kaynak yalnız veri katmanının değişir."
                ),
                "layer",
            ));
        }
        if input.service.is_some() && doc.by_layer(id).next().is_some() {
            return Err(fail(
                codes::LAYER_HAS_OBJECTS,
                format!("“{name}” katmanında nesne var; servis katmanı nesne tutmaz."),
                "layer",
            ));
        }
        node = Some(n.clone());
    }
    for (named, path) in [
        (
            input.service.as_ref().and_then(|s| s.connection.as_deref()),
            "service.connection",
        ),
        (
            input.feed.as_ref().and_then(|f| f.connection.as_deref()),
            "feed.connection",
        ),
    ] {
        if let Some(c) = named
            && !connections.iter().any(|k| k.id == c)
        {
            return Err(fail(
                codes::UNKNOWN_CONNECTION,
                format!(
                    "“{c}” bağlantısı projede yok; bağlantıyı connections ile birlikte verin ya da Bağlantılar penceresinden ekleyin."
                ),
                path,
            ));
        }
    }
    Ok(Checked { connections, node })
}

/// The new layer of `add` and `addFeed`, as the tree would make it.
fn new_layer(input: &LayersService) -> NewLayer {
    NewLayer {
        service: input.service.clone(),
        feed: input.feed.clone(),
        fields: input.fields.clone().unwrap_or_default(),
        ..NewLayer::layer(input.name.as_deref().unwrap_or_default().trim())
    }
}

/// The layer after an update: its new name, service or source.
fn updated(node: &LayerNode, input: &LayersService) -> ServedBy {
    ServedBy {
        service: input.service.clone().or_else(|| node.service.clone()),
        feed: input.feed.clone().or_else(|| node.feed.clone()),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersService) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersService) -> CommandResult<LayersServicePlan> {
    let checked = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let node = match input.operation {
        LayerServiceOperation::Add | LayerServiceOperation::AddFeed => {
            Some(cx.doc.layers().preview(new_layer(input)))
        }
        LayerServiceOperation::Update => checked.node.as_ref().map(|n| {
            let by = updated(n, input);
            LayerNode {
                name: input
                    .name
                    .as_deref()
                    .map_or_else(|| n.name.clone(), |s| s.trim().to_owned()),
                service: by.service,
                feed: by.feed,
                ..n.clone()
            }
        }),
        LayerServiceOperation::Remove => None,
    };
    CommandResult::Completed {
        output: LayersServicePlan {
            node,
            connections: checked.connections,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Writes the change as one undo step named after the operation.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersService,
) -> CommandResult<LayersServiced> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let label = label(input.operation);
    if input.connections.is_some() {
        doc.set_settings(ProjectSettings {
            connections: checked.connections.clone(),
            ..doc.settings().clone()
        });
    }
    let written = match input.operation {
        LayerServiceOperation::Add | LayerServiceOperation::AddFeed => doc.add_layer_at(
            new_layer(&input),
            input.parent.as_deref(),
            input.index.map(|i| i as usize),
            Some(label),
            false,
        ),
        LayerServiceOperation::Update => {
            let n = checked
                .node
                .as_ref()
                .map(|n| (n.id.clone(), updated(n, &input)));
            match n {
                Some((id, by)) => doc
                    .set_layer_service(&id, by, input.name.as_deref(), label)
                    .map(|_| id),
                None => Ok(String::new()),
            }
        }
        LayerServiceOperation::Remove => {
            let id = checked
                .node
                .as_ref()
                .map(|n| n.id.clone())
                .unwrap_or_default();
            doc.transact(label, |doc| doc.remove_layer(&id).map(|_| ()))
                .map(|()| id)
        }
    };
    match written {
        Ok(layer) => CommandResult::Completed {
            output: LayersServiced {
                layer,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}
