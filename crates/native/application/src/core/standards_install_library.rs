//! `cad.standards.install`'s block and sheet-template rows (docs/adr/1100 §2–§3). A block reference is read from the
//! first library holding its publication ([`Libraries`]) and compared with the drawing's block of the reference's
//! name: its base, objects (a nested insert by its block's name), attributes, unit, dynamic behaviour and publication
//! (numbers within 1e-9; its id, name, description, category and tags aside). A sheet template reference is the
//! template revision the standard approved, compared with the drawing's record of that template id. A reference no
//! library holds is `unavailable`: kept, reported with what the nearest entry differs in, never silently skipped.
//!
//! Writing: an added block takes the library's id unless the drawing has it; a replaced one keeps the drawing's id
//! and name, so its inserts follow it (a dynamic insert's cached geometry is refreshed in the same step); a nested
//! definition the library block inserts is the drawing's of that name, else added from the same library. Nothing is
//! removed. A recorded template revision changes no sheet.

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    ApprovedSheetTemplate, BlockDefinition, BlockId, Entity, StandardInstallAction as Action,
    StandardInstallRow, StandardOrigin, StandardResourceKind as Kind, StandardsResolved,
};
use kentos_domain::{Document, Slot};
use serde_json::Value;

use crate::checks::{Stop, error};
use crate::codes;
use crate::standard_libraries::{Found, Libraries};
use crate::standards::style_differences;

/// What a block or template row installs.
pub(crate) enum Incoming {
    Block {
        def: BlockDefinition,
        library: Vec<BlockDefinition>,
    },
    Template {
        template_id: String,
        revision: u32,
    },
}

fn origin(
    map: &std::collections::BTreeMap<String, StandardOrigin>,
    name: &str,
    r: &StandardsResolved,
) -> StandardOrigin {
    map.get(name)
        .cloned()
        .unwrap_or_else(|| r.provenance.rules.clone())
}

/// A definition's look as the install compares it.
fn look(def: &BlockDefinition, name_of: &dyn Fn(BlockId) -> Option<String>) -> Value {
    let mut v = serde_json::to_value(def).unwrap_or(Value::Null);
    if let Some(o) = v.as_object_mut() {
        for field in ["id", "name", "description", "card"] {
            o.remove(field);
        }
        let publication = def.card.as_ref().and_then(|c| c.publication.as_ref());
        o.insert(
            "publication".into(),
            serde_json::to_value(publication).unwrap_or(Value::Null),
        );
        if let Some(Value::Array(objects)) = o.get_mut("entities") {
            for (value, e) in objects.iter_mut().zip(&def.entities) {
                if let (Entity::Insert(i), Some(fields)) = (e, value.as_object_mut()) {
                    fields.insert(
                        "block".into(),
                        Value::String(name_of(i.block).unwrap_or_default()),
                    );
                }
            }
        }
    }
    v
}

/// How many inserts place `id`: the drawing's objects and the objects of its definitions.
fn inserts(doc: &Document, id: BlockId) -> u32 {
    let places = |e: &Entity| matches!(e, Entity::Insert(i) if i.block == id);
    let n = doc.entities().filter(|e| places(e)).count()
        + doc
            .blocks()
            .iter()
            .flat_map(|b| &b.entities)
            .filter(|e| places(e))
            .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The block and sheet-template rows, in the standards' order, each with what it would install.
pub(crate) fn rows(
    doc: &Document,
    r: &StandardsResolved,
    libraries: &Libraries,
) -> Vec<(StandardInstallRow, Option<Incoming>)> {
    let mut out = Vec::new();
    for reference in &r.resources.blocks {
        let local = doc.block_named(&reference.name);
        let mut row = StandardInstallRow {
            kind: Kind::Block,
            name: reference.name.clone(),
            status: "unavailable".into(),
            action: Action::KeepLocal,
            new_name: None,
            fields: Vec::new(),
            library: None,
            uses: local.map_or(0, |b| inserts(doc, b.id)),
            source: origin(&r.provenance.blocks, &reference.name, r),
        };
        let incoming = match libraries.block(reference) {
            Found::Not { fields } => {
                row.fields = fields;
                None
            }
            Found::In {
                library,
                item: (def, all),
            } => {
                row.library = Some(library.to_owned());
                match local {
                    None => {
                        row.status = "missing".into();
                        row.action = Action::Add;
                    }
                    Some(local) => {
                        let in_library =
                            |id: BlockId| all.iter().find(|b| b.id == id).map(|b| b.name.clone());
                        let in_drawing = |id: BlockId| doc.block(id).map(|b| b.name.clone());
                        row.fields =
                            style_differences(&look(local, &in_drawing), &look(def, &in_library));
                        row.status = if row.fields.is_empty() {
                            "same"
                        } else {
                            "differs"
                        }
                        .into();
                    }
                }
                Some(Incoming::Block {
                    def: def.clone(),
                    library: all.to_vec(),
                })
            }
        };
        out.push((row, incoming));
    }
    let approved = &doc.settings().approved_sheet_templates;
    let sheets = made_from(doc);
    for reference in &r.resources.sheet_templates {
        let id = reference.template_id.to_lowercase();
        let local = approved.iter().find(|t| t.template_id.to_lowercase() == id);
        let mut row = StandardInstallRow {
            kind: Kind::SheetTemplate,
            name: reference.template_id.clone(),
            status: "unavailable".into(),
            action: Action::KeepLocal,
            new_name: None,
            fields: Vec::new(),
            library: None,
            uses: sheets.get(&id).copied().unwrap_or(0),
            source: origin(&r.provenance.sheet_templates, &reference.template_id, r),
        };
        let incoming = match libraries.template(reference) {
            Found::Not { fields } => {
                row.fields = fields;
                None
            }
            Found::In { library, .. } => {
                row.library = Some(library.to_owned());
                match local {
                    None => {
                        row.status = "missing".into();
                        row.action = Action::Add;
                    }
                    Some(t) if t.revision == reference.revision => row.status = "same".into(),
                    Some(_) => {
                        row.status = "differs".into();
                        row.fields = vec!["revision".into()];
                    }
                }
                Some(Incoming::Template {
                    template_id: reference.template_id.clone(),
                    revision: reference.revision,
                })
            }
        };
        out.push((row, incoming));
    }
    out
}

/// The sheets made from each template, by its id lowercased.
fn made_from(doc: &Document) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    let Some(sheets) = doc.sheets() else {
        return out;
    };
    let Ok((book, _)) = kentos_sheet::project::read_project_sheets(sheets) else {
        return out;
    };
    for sheet in &book.sheets {
        if let Some(o) = &sheet.origin {
            *out.entry(o.template_id.to_lowercase()).or_insert(0) += 1;
        }
    }
    out
}

/// Why a choice the row's state does not allow is refused, when the new kinds say it otherwise (none: the
/// general rule's words).
pub(crate) fn refused_action(
    doc: &Document,
    row: &StandardInstallRow,
    action: Action,
) -> Option<&'static str> {
    if row.status == "unavailable" {
        return (action != Action::KeepLocal).then_some(
            "okunabilen hiçbir kitaplıkta yok: kurulamaz, yalnız çizimdeki korunabilir",
        );
    }
    match (row.kind, row.status.as_str(), action) {
        (Kind::SheetTemplate, "differs", Action::RenameIncoming) => Some(
            "kimliğiyle kaydedilir, yeni adla eklenemez: değiştirilebilir ya da çizimdeki korunabilir",
        ),
        (Kind::Block, "differs", Action::Replace)
            if doc.block_named(&row.name).is_some_and(|b| b.xref.is_some()) =>
        {
            Some(
                "çizimde bir dış başvuru: kitaplık tanımıyla değiştirilemez; yeni adla eklenebilir ya da korunabilir",
            )
        }
        _ => None,
    }
}

/// The definitions the rows add and replace, and the dynamic inserts refreshed with them; `fresh` gives a new id.
pub(crate) struct BlockWrite {
    pub blocks: Vec<BlockDefinition>,
    pub changes: Vec<(Slot, Entity)>,
}

fn refused(message: String) -> Stop {
    Stop::failed(error(codes::BLOCK_REFUSED, message, Some("choices".into())))
}

pub(crate) fn plan_blocks(
    doc: &Document,
    rows: &[(StandardInstallRow, Option<Incoming>)],
    mut fresh: impl FnMut() -> BlockId,
) -> Result<BlockWrite, Stop> {
    let mut put: Vec<BlockDefinition> = Vec::new();
    let mut ids_taken: HashSet<BlockId> = doc.blocks().iter().map(|b| b.id).collect();
    // A library definition brought in for a nested insert, by its library id: the id it has in the drawing.
    let mut brought: HashMap<BlockId, BlockId> = HashMap::new();
    let mut take_id = |want: BlockId, taken: &mut HashSet<BlockId>| {
        let id = if taken.contains(&want) { fresh() } else { want };
        taken.insert(id);
        id
    };
    for (row, incoming) in rows {
        let Some(Incoming::Block { def, library }) = incoming else {
            continue;
        };
        if row.status == "same" || row.action == Action::KeepLocal {
            continue;
        }
        let mut next = def.clone();
        match row.action {
            Action::Replace => {
                let Some(local) = doc.block_named(&row.name) else {
                    continue;
                };
                next.id = local.id;
                next.name.clone_from(&local.name);
            }
            _ => {
                next.id = take_id(def.id, &mut ids_taken);
                next.name = row.new_name.clone().unwrap_or_else(|| row.name.clone());
            }
        }
        // Nested inserts: the drawing's definition of that name, else the library's, brought once.
        let mut stack = vec![next];
        while let Some(mut b) = stack.pop() {
            let mut waiting = Vec::new();
            for e in &mut b.entities {
                let Entity::Insert(i) = e else { continue };
                let Some(inner) = library.iter().find(|x| x.id == i.block) else {
                    return Err(refused(format!(
                        "“{}” kitaplık bloğu kitaplıkta da çizimde de olmayan bir bloğu yerleştiriyor; kitaplığı düzeltin ya da bloğu atlayın.",
                        row.name
                    )));
                };
                if let Some(have) = doc.block_named(&inner.name) {
                    i.block = have.id;
                } else if let Some(id) = brought.get(&inner.id) {
                    i.block = *id;
                } else {
                    let mut copy = inner.clone();
                    copy.id = take_id(inner.id, &mut ids_taken);
                    brought.insert(inner.id, copy.id);
                    i.block = copy.id;
                    waiting.push(copy);
                }
            }
            put.push(b);
            stack.extend(waiting);
        }
    }
    if put.is_empty() {
        return Ok(BlockWrite {
            blocks: put,
            changes: Vec::new(),
        });
    }
    let incoming: HashSet<BlockId> = put.iter().map(|b| b.id).collect();
    let mut next: Vec<BlockDefinition> = doc
        .blocks()
        .iter()
        .map(|d| put.iter().find(|b| b.id == d.id).unwrap_or(d).clone())
        .collect();
    next.extend(put.iter().filter(|b| doc.block(b.id).is_none()).cloned());
    let changes = crate::block_params::refresh_definitions(doc, &mut next, &incoming)?;
    let refs: Vec<&BlockDefinition> = next.iter().collect();
    crate::blocks_define::block_rules(&refs)?;
    let blocks = next
        .into_iter()
        .filter(|b| incoming.contains(&b.id))
        .collect();
    Ok(BlockWrite { blocks, changes })
}

/// The approved sheet template revisions after the rows.
pub(crate) fn approved_after(
    doc: &Document,
    rows: &[(StandardInstallRow, Option<Incoming>)],
) -> Vec<ApprovedSheetTemplate> {
    let mut out = doc.settings().approved_sheet_templates.clone();
    for (row, incoming) in rows {
        let Some(Incoming::Template {
            template_id,
            revision,
        }) = incoming
        else {
            continue;
        };
        let record = ApprovedSheetTemplate {
            template_id: template_id.clone(),
            revision: *revision,
            standard: row.source.id.clone(),
            standard_revision: row.source.revision.clone(),
        };
        match (row.status.as_str(), row.action) {
            ("missing", Action::Add) => out.push(record),
            ("differs", Action::Replace) => {
                let id = template_id.to_lowercase();
                if let Some(t) = out.iter_mut().find(|t| t.template_id.to_lowercase() == id) {
                    t.revision = record.revision;
                    t.standard = record.standard;
                    t.standard_revision = record.standard_revision;
                }
            }
            _ => {}
        }
    }
    out
}

/// Writes the blocks: definitions merged and refreshed inserts updated; why not.
pub(crate) fn write_blocks(doc: &mut Document, w: BlockWrite) -> Result<(), String> {
    if w.blocks.is_empty() {
        return Ok(());
    }
    doc.merge_blocks(&w.blocks).map_err(|r| r.to_string())?;
    for (slot, entity) in w.changes {
        doc.update(slot, entity);
    }
    Ok(())
}
