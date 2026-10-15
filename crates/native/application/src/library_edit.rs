//! `cad.library.edit` v1 (docs/adr/0314 §3; plan/cad.md CAD-STYLE-14): the project's style library through a
//! command: items created, updated, copied, renamed, moved and removed, a `.kstil` file's items taken in and given
//! out, a symbol given to objects or a layer. The library is a project part outside the undo steps (an edit,
//! docs/adr/0092); giving a symbol is one undo step through `cad.entities.set` (“Sembol ata”) or
//! `cad.layers.renderer` (“Katman stili”). The file's rules are the style library's (`kentos_style_library::file`):
//! a `.kstil` file is untrusted data, its drawings are cleaned. The shared cases are
//! `fixtures/commands/v1/cad.library.edit.json` (`scripts/fixtures/library_command_cases.py`).

use kentos_contracts::{
    CommandResult, Entity, LayerNode, LayersRenderer, LibraryConflict, LibraryEdit, LibraryEdited,
    LibraryOperation, ProjectStyles,
};
use kentos_domain::Document;
use kentos_style_library::file::{
    ConflictMode, export_styles, import_styles, parse_style_file, sanitize_svg, validate_item,
};
use kentos_style_library::library::{ItemKind, Source, StyleLibrary};
use serde_json::{Map, Value, json};

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes};

/// The project library's items as a summary lists them: id, kind, name and path, in the library's order.
pub fn listed(doc: &Document) -> Vec<Value> {
    doc.styles()
        .items
        .iter()
        .map(|i| {
            json!({
                "id": i.get("id").cloned().unwrap_or(Value::Null),
                "kind": i.get("kind").cloned().unwrap_or(Value::Null),
                "name": i.get("name").cloned().unwrap_or(Value::Null),
                "path": i.get("path").cloned().unwrap_or_else(|| json!([])),
            })
        })
        .collect()
}

/// Issues as one sentence: joined, one full stop at the end.
fn said(issues: &[String]) -> String {
    format!("{}.", issues.join("; ").trim_end_matches('.'))
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The project's library as the style library keeps it.
fn library_of(doc: &Document) -> StyleLibrary {
    let mut lib = StyleLibrary::with_system(Vec::new(), Vec::new());
    let styles = doc.styles();
    lib.load(Source::Project, &styles.items, &styles.categories);
    lib
}

fn styles_of(lib: &StyleLibrary) -> ProjectStyles {
    let (items, categories) = lib.dump(Source::Project);
    ProjectStyles { items, categories }
}

/// An item checked by the file's rules, its drawing cleaned.
fn checked_item(item: &Value, path: &str) -> Result<Value, Stop> {
    let issues = validate_item(item, 0);
    if !issues.is_empty() {
        return Err(fail(
            codes::INVALID_LIBRARY_ITEM,
            format!("Kitaplık öğesi kurallara uymuyor: {}", said(&issues)),
            path,
        ));
    }
    let mut item = item.clone();
    if item.get("kind").and_then(Value::as_str) == Some("asset")
        && item.get("format").and_then(Value::as_str) == Some("svg")
        && let Some(data) = item.get("data").and_then(Value::as_str)
    {
        item["data"] = Value::from(sanitize_svg(data));
    }
    Ok(item)
}

/// Whether `v` refers to the library item `id` (`{"ref": id}`), anywhere in it.
fn refers(v: &Value, id: &str) -> bool {
    match v {
        Value::Object(o) => {
            o.get("ref").and_then(Value::as_str) == Some(id) || o.values().any(|x| refers(x, id))
        }
        Value::Array(a) => a.iter().any(|x| refers(x, id)),
        _ => false,
    }
}

/// What uses `id`: objects (the drawing's and the blocks'), layers' renderers, the library's own items.
fn uses(doc: &Document, lib: &StyleLibrary, id: &str) -> (usize, usize, usize) {
    let mine = |e: &Entity| e.base().symbol.as_deref() == Some(id);
    let objects = doc.entities().filter(|e| mine(e)).count()
        + doc
            .blocks()
            .iter()
            .flat_map(|b| &b.entities)
            .filter(|e| mine(e))
            .count();
    fn layers(nodes: &[LayerNode], id: &str) -> usize {
        nodes
            .iter()
            .map(|n| {
                usize::from(n.style.renderer.as_ref().is_some_and(|r| refers(r, id)))
                    + layers(&n.children, id)
            })
            .sum()
    }
    let items = lib
        .items(Some(Source::Project))
        .into_iter()
        .filter(|(item, _)| {
            item.id() != id
                && (item.template_symbol() == Some(id)
                    || item.symbol().is_some_and(|s| {
                        kentos_style_library::library::assets_of_symbol(s)
                            .iter()
                            .any(|a| a == id)
                    }))
        })
        .count();
    (objects, layers(doc.layers().nodes(), id), items)
}

/// The input's own checks, in the contract's order (before the revision).
fn needs(input: &LibraryEdit) -> Result<(), Stop> {
    use LibraryOperation as Op;
    match input.operation {
        Op::Create if input.item.is_none() => Err(fail(
            codes::NO_ITEM,
            "Eklenecek öğeyi verin (item): kimliği, türü, adı ve yolu ile.".into(),
            "item",
        )),
        Op::Update | Op::Duplicate | Op::Rename | Op::Move | Op::Remove | Op::Assign
            if input.id.as_deref().is_none_or(checks::is_blank) =>
        {
            Err(fail(
                codes::NO_ID,
                "Öğenin kimliğini verin (id).".into(),
                "id",
            ))
        }
        Op::Import if input.kstil.as_deref().is_none_or(|t| t.trim().is_empty()) => Err(fail(
            codes::NO_KSTIL,
            "Alınacak .kstil dosyasının metnini verin (kstil).".into(),
            "kstil",
        )),
        Op::Assign if input.uids.as_ref().is_none_or(Vec::is_empty) && input.layer.is_none() => {
            Err(fail(
                codes::NO_TARGETS,
                "Sembolün verileceği nesneleri (uids) ya da katmanı (layer) verin.".into(),
                "uids",
            ))
        }
        _ => Ok(()),
    }
}

/// The library as it would be and what changed, writing nothing. `Err` refuses.
fn edited(doc: &Document, input: &LibraryEdit) -> Result<(StyleLibrary, LibraryEdited), Stop> {
    use LibraryOperation as Op;
    needs(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut lib = library_of(doc);
    let mut out = LibraryEdited {
        changed: false,
        added: Vec::new(),
        replaced: Vec::new(),
        skipped: Vec::new(),
        removed: Vec::new(),
        kstil: None,
        objects: Vec::new(),
        layer: None,
        revision: doc.revision().to_string(),
    };
    let id = input.id.clone().unwrap_or_default();
    let have =
        |lib: &StyleLibrary, id: &str| lib.get(id).is_some_and(|(_, s)| s == Source::Project);
    let missing = |id: &str| {
        fail(
            codes::LIBRARY_ITEM_NOT_FOUND,
            format!("“{id}” kimlikli öğe projenin kitaplığında yok."),
            "id",
        )
    };
    let refused = |e: String, path: &str| fail(codes::INVALID_LIBRARY_ITEM, e, path);
    match input.operation {
        Op::Create => {
            let item = checked_item(input.item.as_ref().unwrap_or(&Value::Null), "item")?;
            let new = item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if lib.get(&new).is_some() {
                return Err(fail(
                    codes::LIBRARY_ITEM_EXISTS,
                    format!(
                        "Projenin kitaplığında “{new}” kimlikli bir öğe var; başka bir kimlik verin."
                    ),
                    "item.id",
                ));
            }
            lib.add(Source::Project, item)
                .map_err(|e| refused(e, "item"))?;
            out.added.push(new);
        }
        Op::Update => {
            let (item, _) = lib
                .get(&id)
                .filter(|(_, s)| *s == Source::Project)
                .ok_or_else(|| missing(&id))?;
            let Some(patch) = input.patch.as_ref().and_then(Value::as_object) else {
                return Err(fail(
                    codes::INVALID_LIBRARY_ITEM,
                    "Değişecek alanları bir nesne olarak verin (patch).".into(),
                    "patch",
                ));
            };
            let mut next = item.value().clone();
            if let Some(o) = next.as_object_mut() {
                for (k, v) in patch.iter().filter(|(k, _)| *k != "id" && *k != "kind") {
                    if v.is_null() {
                        o.remove(k);
                    } else {
                        o.insert(k.clone(), v.clone());
                    }
                }
            }
            let next = checked_item(&next, "patch")?;
            let fields: Map<String, Value> = next.as_object().cloned().unwrap_or_default();
            let mut patch = patch.clone();
            for (k, v) in &mut patch {
                if let Some(clean) = fields.get(k) {
                    *v = clean.clone();
                }
            }
            lib.update(&id, &patch).map_err(|e| refused(e, "patch"))?;
            out.replaced.push(id);
        }
        Op::Duplicate => {
            if !have(&lib, &id) {
                return Err(missing(&id));
            }
            if let Some(new) = &input.new_id
                && lib.get(new).is_some()
            {
                return Err(fail(
                    codes::LIBRARY_ITEM_EXISTS,
                    format!(
                        "Projenin kitaplığında “{new}” kimlikli bir öğe var; başka bir kimlik verin."
                    ),
                    "newId",
                ));
            }
            let copy = lib
                .copy(
                    &id,
                    Source::Project,
                    input.name.as_deref(),
                    input.path.as_deref(),
                )
                .map_err(|e| refused(e, "id"))?;
            let mut made = copy.id().to_owned();
            if let Some(new) = &input.new_id {
                let mut value = copy.value().clone();
                value["id"] = Value::from(new.clone());
                lib.remove(&made).map_err(|e| refused(e, "id"))?;
                lib.add(Source::Project, value)
                    .map_err(|e| refused(e, "newId"))?;
                made.clone_from(new);
            }
            out.added.push(made);
        }
        Op::Rename => {
            if !have(&lib, &id) {
                return Err(missing(&id));
            }
            lib.rename(&id, input.name.as_deref().unwrap_or_default())
                .map_err(|e| refused(e, "name"))?;
            out.replaced.push(id);
        }
        Op::Move => {
            if !have(&lib, &id) {
                return Err(missing(&id));
            }
            lib.move_to(&id, input.path.as_deref().unwrap_or_default())
                .map_err(|e| refused(e, "path"))?;
            out.replaced.push(id);
        }
        Op::Remove => {
            if !have(&lib, &id) {
                return Err(missing(&id));
            }
            let (objects, layers, items) = uses(doc, &lib, &id);
            if objects + layers + items > 0 {
                return Err(fail(
                    codes::LIBRARY_ITEM_IN_USE,
                    format!(
                        "“{id}” öğesini {objects} nesne, {layers} katman ve kitaplığın {items} öğesi kullanıyor; önce onlardan kaldırın."
                    ),
                    "id",
                ));
            }
            lib.remove(&id).map_err(|e| refused(e, "id"))?;
            out.removed.push(id);
        }
        Op::Import => {
            let file =
                parse_style_file(input.kstil.as_deref().unwrap_or_default()).map_err(|issues| {
                    fail(
                        codes::INVALID_KSTIL,
                        format!(".kstil dosyası okunamadı: {}", said(&issues)),
                        "kstil",
                    )
                })?;
            let mode = match input.conflict {
                LibraryConflict::Skip => ConflictMode::Skip,
                LibraryConflict::Replace => ConflictMode::Replace,
            };
            for item in &file.items {
                let id = item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                match (lib.get(&id).is_some(), mode) {
                    (false, _) => out.added.push(id),
                    (true, ConflictMode::Replace) if have(&lib, &id) => out.replaced.push(id),
                    _ => out.skipped.push(id),
                }
            }
            import_styles(&mut lib, &file, Source::Project, mode);
        }
        Op::Export => {
            let all: Vec<String> = lib
                .items(Some(Source::Project))
                .into_iter()
                .map(|(i, _)| i.id().to_owned())
                .collect();
            let ids: Vec<String> = input.ids.clone().unwrap_or(all);
            for (i, id) in ids.iter().enumerate() {
                if lib.get(id).is_none() {
                    return Err(fail(
                        codes::LIBRARY_ITEM_NOT_FOUND,
                        format!("“{id}” kimlikli öğe projenin kitaplığında yok."),
                        &format!("ids[{i}]"),
                    ));
                }
            }
            let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
            let mut file = export_styles(&lib, &refs);
            // The same items give the same text: no time in it.
            if let Some(o) = file.as_object_mut() {
                o.remove("exported");
            }
            out.kstil = Some(kentos_contracts::canonical::canonical_json(&file));
        }
        Op::Assign => {
            let Some((item, _)) = lib.get(&id).filter(|(_, s)| *s == Source::Project) else {
                return Err(missing(&id));
            };
            if item.kind() != ItemKind::Symbol {
                return Err(fail(
                    codes::NOT_A_SYMBOL,
                    format!(
                        "“{}” bir sembol değil; nesnelere ve katmanlara yalnız sembol verilir.",
                        item.name()
                    ),
                    "id",
                ));
            }
        }
    }
    Ok((lib, out))
}

/// The layer's renderer with the symbol in its single renderer's slot of the symbol's kind.
fn single_with(old: Option<&Value>, slot: &str, id: &str) -> Value {
    let mut renderer = match old {
        Some(r) if r.get("type").and_then(Value::as_str) == Some("single") => r.clone(),
        _ => json!({ "type": "single", "symbols": {} }),
    };
    if !renderer.get("symbols").is_some_and(Value::is_object) {
        renderer["symbols"] = json!({});
    }
    renderer["symbols"][slot] = json!({ "ref": id });
    renderer
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LibraryEdit) -> CommandResult<()> {
    match edited(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What the edit would do: the same write on a reading copy; the drawing stays.
pub fn plan(cx: &ExecutionContext<'_>, input: &LibraryEdit) -> CommandResult<LibraryEdited> {
    if let Err(stop) = needs(input) {
        return stop.into();
    }
    let mut copy = cx.doc.reading_copy();
    let mut inner = ExecutionContext::new(&mut copy);
    match write(&mut inner, input.clone()) {
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

pub fn execute(cx: &mut ExecutionContext<'_>, input: LibraryEdit) -> CommandResult<LibraryEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_LIBRARY_EDIT,
        kentos_contracts::CAD_LIBRARY_EDIT_VERSION,
        input,
        write,
    )
}

fn write(cx: &mut ExecutionContext<'_>, input: LibraryEdit) -> CommandResult<LibraryEdited> {
    let (lib, mut out) = match edited(cx.doc, &input) {
        Ok(x) => x,
        Err(stop) => return stop.into(),
    };
    let mut warnings = Vec::new();
    if input.operation == LibraryOperation::Assign {
        let id = input.id.clone().unwrap_or_default();
        let slot = lib
            .symbol(&id)
            .and_then(|s| s.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("marker")
            .to_owned();
        if let Some(uids) = input.uids.clone().filter(|u| !u.is_empty()) {
            let set = serde_json::from_value(
                json!({ "uids": uids, "symbol": id, "operation": "symbol" }),
            );
            let Ok(set) = set else {
                return Stop::failed(error(
                    codes::NO_TARGETS,
                    "Nesneler okunamadı.".into(),
                    Some("uids".into()),
                ))
                .into();
            };
            match crate::set::execute(cx, set) {
                CommandResult::Completed {
                    output,
                    warnings: w,
                } => {
                    out.objects = output.changed;
                    warnings.extend(w);
                }
                other => return crate::trim::relay(other),
            }
        }
        if let Some(layer) = input.layer.clone() {
            let old = find(cx.doc.layers().nodes(), &layer).and_then(|n| n.style.renderer.clone());
            let renderer = single_with(old.as_ref(), &slot, &id);
            let rendered = crate::layers_renderer::execute(
                cx,
                LayersRenderer {
                    layer: layer.clone(),
                    renderer: Some(renderer),
                    expected_revision: None,
                },
            );
            match rendered {
                CommandResult::Completed { warnings: w, .. } => {
                    out.layer = Some(layer);
                    warnings.extend(w);
                }
                CommandResult::Failed { mut error } => {
                    if error.path.as_deref() == Some("layer") || error.path.is_none() {
                        error.path = Some("layer".into());
                    }
                    return CommandResult::Failed { error };
                }
                other => return crate::trim::relay(other),
            }
        }
        out.changed = !out.objects.is_empty() || out.layer.is_some();
    } else {
        let styles = styles_of(&lib);
        if styles != *cx.doc.styles() {
            cx.doc.set_styles(styles);
            out.changed = true;
        }
    }
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed {
        output: out,
        warnings,
    }
}

fn find<'a>(nodes: &'a [LayerNode], id: &str) -> Option<&'a LayerNode> {
    nodes.iter().find_map(|n| {
        if n.id == id {
            Some(n)
        } else {
            find(&n.children, id)
        }
    })
}
