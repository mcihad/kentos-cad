//! `cad.layers.edit` v1 (docs/adr/0248 §1): the layer tree changed as one
//! undo step: a layer or group added, restyled, moved or removed with its
//! objects, a layer copied, layers merged, a layer's fields written, what
//! nothing uses purged. The Katmanlar panel, Kopyasını oluştur,
//! Katmanları birleştir, Alanlar and Kullanılmayanları temizle write
//! through it on the desktop; Python and MCP run it as they are. The
//! shared cases are in `fixtures/commands/v1/cad.layers.edit.json`.
//!
//! Everything is the drawing's own rule (`kentos_domain`): `add_layer_at`,
//! `set_layer_style`, `move_layer`, `remove_layer` (docs/adr/0072),
//! `set_layer_fields` (docs/adr/0199 §3), `layer_purge` (docs/adr/0177 §5);
//! a copy's objects are written by `cad.entities.create`, a merge's moved by
//! `cad.entities.set`, in the same step. The checks are in the contract's
//! order (`kentos_contracts::LayersEdit`).

use std::collections::HashSet;

use kentos_contracts::{
    BlockId, CommandResult, CommandWarning, EntitiesCreate, EntitiesSetProperties, Entity,
    LayerEditOperation, LayerNode, LayerNodeType, LayersEdit, LayersEdited, NewObject,
    PropertiesOperation, fields_problem, layer_color_ok,
};
use kentos_domain::layer_purge::{self, PurgeIds};
use kentos_domain::{Document, NewLayer, Slot, default_style};

use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::{edit_geometry, shape};
use crate::{ExecutionContext, create, set};

/// The steps' names that are not the drawing's own.
pub const LAYER_COLOR: &str = "Katman rengi";
pub const LINE_TYPE: &str = "Çizgi tipi";
pub const LINE_WEIGHT: &str = "Çizgi kalınlığı";
pub const LAYER_STYLE: &str = "Katman stili";
pub const DUPLICATE: &str = "Katmanı kopyala";
pub const MERGE: &str = "Katmanları birleştir";
pub const PURGE: &str = "Kullanılmayanları temizle";

/// A new layer's name, and a group's, when none is given.
pub const NEW_LAYER: &str = "Yeni katman";
pub const NEW_GROUP: &str = "Yeni grup";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// An object as `cad.entities.create` writes its copy in place: its
/// geometry, its own colour, weight, attributes, label and symbol (Katmana
/// kopyala, Kopyasını oluştur; the web's `newObjectOf`).
pub fn new_object(e: &Entity) -> Option<NewObject> {
    Some(NewObject {
        line_type: None,
        geometry: edit_geometry(shape(e))?,
        color: e.base().color.clone(),
        line_weight: e.base().line_weight,
        attrs: Some(e.base().attrs.clone()),
        label: e.base().label.clone(),
        label_of: None,
        label_scale: None,
        symbol: e.base().symbol.clone(),
    })
}

/// Merge's sources: the target and repeats passed over, in their order.
fn sources(input: &LayersEdit) -> Vec<String> {
    let target = input.layer.as_deref();
    let mut seen = HashSet::new();
    input
        .layers
        .iter()
        .flatten()
        .filter(|id| Some(id.as_str()) != target && seen.insert(id.as_str()))
        .cloned()
        .collect()
}

/// The input's own rules, before the drawing is read.
fn check_input(input: &LayersEdit) -> Result<(), Stop> {
    use LayerEditOperation as Op;
    let op = input.operation;
    let needs_layer = matches!(
        op,
        Op::Style | Op::Move | Op::Remove | Op::Duplicate | Op::Merge | Op::Fields
    );
    if needs_layer && input.layer.is_none() {
        return Err(fail(
            codes::NO_LAYER,
            "İşlemin katmanını ya da grubunu kimliğiyle verin (layer).".into(),
            "layer",
        ));
    }
    if op == Op::Merge && sources(input).is_empty() {
        return Err(fail(
            codes::NO_LAYERS,
            "Hedefe birleşecek katmanları kimlikleriyle verin (layers); hedefin kendisi sayılmaz."
                .into(),
            "layers",
        ));
    }
    if matches!(op, Op::Add | Op::Duplicate) && input.name.as_deref().is_some_and(checks::is_blank)
    {
        return Err(fail(
            codes::EMPTY_NAME,
            "Katmanın adı boş olamaz; bir ad yazın ya da adı vermeyin.".into(),
            "name",
        ));
    }
    if op == Op::Style
        && input.color.is_none()
        && input.line_type.is_none()
        && input.line_weight.is_none()
    {
        return Err(Stop::failed(error(
            codes::NOTHING_TO_SET,
            "Değişecek bir şey yok: color, lineType ya da lineWeight verin.".into(),
            None,
        )));
    }
    if matches!(op, Op::Add | Op::Style) {
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
    }
    if op == Op::Fields
        && let Some(problem) = input.fields.as_deref().and_then(fields_problem)
    {
        return Err(fail(codes::INVALID_FIELDS, problem, "fields"));
    }
    Ok(())
}

fn node<'a>(doc: &'a Document, id: &str, path: &str) -> Result<&'a LayerNode, Stop> {
    doc.layers().get(id).ok_or_else(|| {
        fail(
            codes::LAYER_NOT_FOUND,
            format!(
                "“{id}” kimlikli katman ya da grup çizimde yok. Var olan bir düğümün kimliğini verin."
            ),
            path,
        )
    })
}

/// A layer holding objects: not a group, not drawn from a service.
fn holding_layer<'a>(doc: &'a Document, id: &str, path: &str) -> Result<&'a LayerNode, Stop> {
    let n = node(doc, id, path)?;
    let name = &n.name;
    if n.kind == LayerNodeType::Group {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{name}” bir katman grubu; bu işlem yalnız katmanın olur. Grubun bir katmanını verin."
            ),
            path,
        ));
    }
    if n.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!(
                "“{name}” servisten çizilir ve nesne tutmaz; bu işlem nesneleri olan katmanın olur."
            ),
            path,
        ));
    }
    Ok(n)
}

/// Refused when the node is locked, by itself or a group above; `what`
/// ends the sentence (“kopyası oluşturulmaz”).
fn unlocked(doc: &Document, n: &LayerNode, what: &str, path: &str) -> Result<(), Stop> {
    if doc.layers().is_locked(&n.id) {
        let kind = match n.kind {
            LayerNodeType::Group => "grubu",
            LayerNodeType::Layer => "katmanı",
        };
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” {kind} kilitli; {what}. Kilidini Katmanlar panelinden açın.",
                n.name
            ),
            path,
        ));
    }
    Ok(())
}

/// What the drawing must hold for the input, in the contract's order.
fn check_doc(doc: &Document, input: &LayersEdit) -> Result<(), Stop> {
    use LayerEditOperation as Op;
    let layer = input.layer.as_deref().unwrap_or_default();
    match input.operation {
        Op::Add => {
            if let Some(parent) = input.parent.as_deref() {
                node(doc, parent, "parent")?;
            }
        }
        Op::Style | Op::Remove => {
            node(doc, layer, "layer")?;
        }
        Op::Move => {
            let n = node(doc, layer, "layer")?;
            if let Some(parent) = input.parent.as_deref() {
                let group = node(doc, parent, "parent")?;
                if group.kind != LayerNodeType::Group {
                    return Err(fail(
                        codes::NOT_A_GROUP,
                        format!(
                            "“{}” bir katman; düğüm yalnız bir grubun içine taşınır. Bir grubun kimliğini verin ya da parent'ı vermeyin (en üst).",
                            group.name
                        ),
                        "parent",
                    ));
                }
                if doc.layers().move_place(layer, Some(parent), None).is_none() {
                    return Err(fail(
                        codes::MOVE_INTO_ITSELF,
                        format!(
                            "“{}” kendi içine ya da altındaki bir gruba taşınamaz.",
                            n.name
                        ),
                        "parent",
                    ));
                }
            }
            unlocked(doc, n, "taşınamaz", "layer")?;
            if let Some(parent) = input.parent.as_deref() {
                let group = node(doc, parent, "parent")?;
                unlocked(doc, group, "içine düğüm taşınamaz", "parent")?;
            }
        }
        Op::Duplicate => {
            let n = holding_layer(doc, layer, "layer")?;
            unlocked(doc, n, "kopyası oluşturulmaz", "layer")?;
        }
        Op::Merge => {
            let n = holding_layer(doc, layer, "layer")?;
            unlocked(doc, n, "birleştirilemez", "layer")?;
            for (i, id) in input.layers.iter().flatten().enumerate() {
                if id == layer {
                    continue;
                }
                let path = format!("layers[{i}]");
                let n = holding_layer(doc, id, &path)?;
                unlocked(doc, n, "birleştirilemez", &path)?;
            }
        }
        Op::Fields => {
            holding_layer(doc, layer, "layer")?;
        }
        Op::Purge => {
            for (i, id) in input.layers.iter().flatten().enumerate() {
                node(doc, id, &format!("layers[{i}]"))?;
            }
            for (i, id) in input.blocks.iter().flatten().enumerate() {
                let known = BlockId::parse(id).is_some_and(|b| doc.block(b).is_some());
                if !known {
                    return Err(fail(
                        codes::UNKNOWN_BLOCK,
                        format!("“{id}” kimlikli blok tanımı çizimde yok."),
                        &format!("blocks[{i}]"),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn check(doc: &Document, input: &LayersEdit) -> Result<(), Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    check_doc(doc, input)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do now: the same write on a reading copy of the
/// drawing (its objects shared, not copied); the drawing is left as it is,
/// and the answer's revision is the one the plan was made against.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersEdit) -> CommandResult<LayersEdited> {
    if let Err(stop) = check(cx.doc, input) {
        return stop.into();
    }
    let mut copy = cx.doc.reading_copy();
    match write(&mut copy, input.clone()) {
        CommandResult::Completed {
            mut output,
            warnings,
        } => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        other => other,
    }
}

/// Writes the change as one undo step named after it.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersEdit) -> CommandResult<LayersEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_LAYERS_EDIT,
        kentos_contracts::CAD_LAYERS_EDIT_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: LayersEdit) -> CommandResult<LayersEdited> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    write(cx.doc, input)
}

fn done(doc: &Document, layer: Option<String>, changed: bool) -> LayersEdited {
    LayersEdited {
        layer,
        changed,
        objects: 0,
        removed: Vec::new(),
        blocks: Vec::new(),
        kept: 0,
        revision: doc.revision().to_string(),
    }
}

fn completed(output: LayersEdited, warnings: Vec<CommandWarning>) -> CommandResult<LayersEdited> {
    CommandResult::Completed { output, warnings }
}

fn refused<T>(code: &str, message: String, path: &str) -> CommandResult<T> {
    fail(code, message, path).into()
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The objects on a layer, in the drawing's order.
fn objects_on(doc: &Document, layer: &str) -> Vec<Slot> {
    doc.entities()
        .filter(|e| e.base().layer_id == layer)
        .map(|e| Slot(e.base().id))
        .collect()
}

/// A command's refusal as this one's, or `fallback`'s words.
fn passed_on<T, U>(result: CommandResult<T>, fallback: &str) -> CommandResult<U> {
    match result {
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        _ => refused(codes::LAYER_REFUSED, fallback.to_owned(), "layer"),
    }
}

fn write(doc: &mut Document, input: LayersEdit) -> CommandResult<LayersEdited> {
    use LayerEditOperation as Op;
    let layer = input.layer.clone().unwrap_or_default();
    match input.operation {
        Op::Add => add(doc, input),
        Op::Style => {
            let Some(mut style) = doc.layers().get(&layer).map(|n| n.style.clone()) else {
                return completed(done(doc, Some(layer), false), Vec::new());
            };
            let given = [
                input.color.is_some(),
                input.line_type.is_some(),
                input.line_weight.is_some(),
            ];
            let label = match given {
                [true, false, false] => LAYER_COLOR,
                [false, true, false] => LINE_TYPE,
                [false, false, true] => LINE_WEIGHT,
                _ => LAYER_STYLE,
            };
            if let Some(c) = input.color {
                style.color = c;
            }
            if let Some(t) = input.line_type {
                style.line_type = t;
            }
            if let Some(w) = input.line_weight {
                style.line_weight = w;
            }
            let changed = doc.set_layer_style(&layer, style, label);
            completed(done(doc, Some(layer), changed), Vec::new())
        }
        Op::Move => {
            let index = input.index.map(|i| i as usize);
            match doc.move_layer(&layer, input.parent.as_deref(), index) {
                Ok(changed) => completed(done(doc, Some(layer), changed), Vec::new()),
                Err(r) => refused(codes::LAYER_REFUSED, r.0, "parent"),
            }
        }
        Op::Remove => match doc.remove_layer(&layer) {
            Ok(gone) => {
                let mut out = done(doc, Some(layer), true);
                out.objects = count(gone);
                completed(out, Vec::new())
            }
            Err(r) => refused(codes::LAYER_REFUSED, r.0, "layer"),
        },
        Op::Duplicate => duplicate(doc, &layer, input.name.as_deref()),
        Op::Merge => {
            let from = sources(&input);
            merge(doc, &layer, &from)
        }
        Op::Fields => {
            let renames: Vec<(String, String)> = input
                .renames
                .unwrap_or_default()
                .into_iter()
                .map(|r| (r.from, r.to))
                .collect();
            match doc.set_layer_fields(&layer, input.fields.unwrap_or_default(), &renames) {
                Ok(changed) => completed(done(doc, Some(layer), changed), Vec::new()),
                Err(r) => refused(codes::LAYER_REFUSED, r.0, "renames"),
            }
        }
        Op::Purge => purge(doc, input.layers.as_deref(), input.blocks.as_deref()),
    }
}

fn add(doc: &mut Document, input: LayersEdit) -> CommandResult<LayersEdited> {
    let kind = input.kind.unwrap_or(LayerNodeType::Layer);
    let name = match input.name.as_deref() {
        Some(n) => n.trim().to_owned(),
        None => doc.layers().unique_name(match kind {
            LayerNodeType::Layer => NEW_LAYER,
            LayerNodeType::Group => NEW_GROUP,
        }),
    };
    let mut style = default_style();
    if let Some(c) = input.color {
        style.color = c;
    }
    if let Some(t) = input.line_type {
        style.line_type = t;
    }
    if let Some(w) = input.line_weight {
        style.line_weight = w;
    }
    let new = NewLayer {
        visible: input.visible.unwrap_or(true),
        locked: input.locked.unwrap_or(false),
        style,
        ..match kind {
            LayerNodeType::Layer => NewLayer::layer(name),
            LayerNodeType::Group => NewLayer::group(name),
        }
    };
    let index = input.index.map(|i| i as usize);
    let activate = input.activate.unwrap_or(false);
    match doc.add_layer_at(new, input.parent.as_deref(), index, None, activate) {
        Ok(id) => completed(done(doc, Some(id), true), Vec::new()),
        Err(r) => refused(codes::LAYER_REFUSED, r.0, "layer"),
    }
}

/// Kopyasını oluştur: a new layer last in the same group, named “<ad>
/// kopyası” (one of its kind in the tree) unless named, with the same
/// style, fields and visibility, unlocked, and the copies of its objects;
/// one undo step “Katmanı kopyala”.
fn duplicate(doc: &mut Document, id: &str, name: Option<&str>) -> CommandResult<LayersEdited> {
    let Some(source) = doc.layers().get(id).cloned() else {
        return completed(done(doc, None, false), Vec::new());
    };
    let name = match name {
        Some(n) => n.trim().to_owned(),
        None => doc
            .layers()
            .unique_name(&format!("{} kopyası", source.name)),
    };
    let objects: Vec<NewObject> = objects_on(doc, id)
        .into_iter()
        .filter_map(|slot| doc.get(slot).and_then(new_object))
        .collect();
    let group = doc.begin_group(DUPLICATE);
    let mut new = NewLayer::layer(name);
    new.visible = source.visible;
    new.style = source.style.clone();
    let layer = match doc.add_layer(new, Some(id), false) {
        Ok(layer) => layer,
        Err(r) => {
            doc.cancel_group(group);
            return refused(codes::LAYER_REFUSED, r.0, "layer");
        }
    };
    let mut made = 0;
    let mut warnings = Vec::new();
    if !objects.is_empty() {
        let input = EntitiesCreate {
            layer_id: layer.clone(),
            objects,
            operation: None,
            expected_revision: None,
        };
        match create::execute(&mut ExecutionContext::new(doc), input) {
            CommandResult::Completed {
                output,
                warnings: w,
            } => {
                made = output.ids.len();
                warnings.extend(w);
            }
            other => {
                doc.cancel_group(group);
                return passed_on(other, "Nesneler kopyalanamadı.");
            }
        }
    }
    // Its fields after the copies (docs/adr/0199 §1): their values go as they are, kept or not.
    if !source.fields.is_empty()
        && let Err(r) = doc.set_layer_fields(&layer, source.fields.clone(), &[])
    {
        doc.cancel_group(group);
        return refused(codes::LAYER_REFUSED, r.0, "layer");
    }
    doc.end_group(group);
    let mut out = done(doc, Some(layer), true);
    out.objects = count(made);
    completed(out, warnings)
}

/// Katmanları birleştir: the sources' objects to the target through
/// `cad.entities.set`, the sources removed, one undo step; a source that is
/// the active layer gives that to the target first (a tree change, outside
/// the step).
fn merge(doc: &mut Document, target: &str, from: &[String]) -> CommandResult<LayersEdited> {
    let uids: Vec<String> = from
        .iter()
        .flat_map(|id| objects_on(doc, id))
        .filter_map(|slot| doc.uid(slot))
        .map(|uid| uid.to_string())
        .collect();
    if from.iter().any(|id| id == doc.layers().active()) {
        doc.set_active_layer(target);
    }
    let group = doc.begin_group(MERGE);
    let mut warnings = Vec::new();
    let moved = uids.len();
    if !uids.is_empty() {
        let input = EntitiesSetProperties {
            line_type: None,
            line_type_scale: None,
            transparency: None,
            uids,
            layer_id: Some(target.to_owned()),
            color: None,
            line_weight: None,
            symbol: None,
            attrs: None,
            label: None,
            operation: PropertiesOperation::Layer,
            expected_revision: None,
            unlink: false,
            attachments: None,
        };
        match set::execute(&mut ExecutionContext::new(doc), input) {
            CommandResult::Completed { warnings: w, .. } => warnings.extend(w),
            other => {
                doc.cancel_group(group);
                return passed_on(other, "Nesneler taşınamadı.");
            }
        }
    }
    for id in from {
        if let Err(r) = doc.remove_layer(id) {
            doc.cancel_group(group);
            return refused(codes::LAYER_REFUSED, r.0, "layers");
        }
    }
    doc.end_group(group);
    let mut out = done(doc, Some(target.to_owned()), true);
    out.objects = count(moved);
    out.removed = from.to_vec();
    completed(out, warnings)
}

/// Kullanılmayanları temizle over the layers, groups and definitions: the
/// asked ones (absent: every one found but a locked layer) that nothing
/// staying uses, in one undo step.
fn purge(
    doc: &mut Document,
    layers: Option<&[String]>,
    blocks: Option<&[String]>,
) -> CommandResult<LayersEdited> {
    let (removed, kept) = {
        let src = layer_purge::source_of(doc, Vec::new());
        let found = layer_purge::found(&src);
        let mut asked = PurgeIds::default();
        match layers {
            Some(ids) => {
                for id in ids {
                    let group = doc
                        .layers()
                        .get(id)
                        .is_some_and(|n| n.kind == LayerNodeType::Group);
                    if group {
                        asked.groups.push(id.clone());
                    } else {
                        asked.layers.push(id.clone());
                    }
                }
            }
            None => {
                asked.layers = found
                    .layers
                    .iter()
                    .filter(|f| !f.locked)
                    .map(|f| f.id.clone())
                    .collect();
                asked.groups = found.groups.iter().map(|f| f.id.clone()).collect();
            }
        }
        asked.blocks = match blocks {
            Some(ids) => ids.to_vec(),
            None => found.blocks.iter().map(|f| f.id.clone()).collect(),
        };
        layer_purge::removed(&src, &asked)
    };
    let changed =
        !(removed.layers.is_empty() && removed.groups.is_empty() && removed.blocks.is_empty());
    if changed {
        let group = doc.begin_group(PURGE);
        for id in removed.blocks.iter().filter_map(|t| BlockId::parse(t)) {
            if let Err(r) = doc.remove_block(id) {
                doc.cancel_group(group);
                return refused(codes::LAYER_REFUSED, r.0, "blocks");
            }
        }
        for id in removed.layers.iter().chain(&removed.groups) {
            if let Err(r) = doc.remove_layer(id) {
                doc.cancel_group(group);
                return refused(codes::LAYER_REFUSED, r.0, "layers");
            }
        }
        doc.end_group(group);
    }
    let mut out = done(doc, None, changed);
    out.removed = removed
        .layers
        .iter()
        .chain(&removed.groups)
        .cloned()
        .collect();
    out.blocks = removed.blocks;
    out.kept = count(kept);
    completed(out, Vec::new())
}
