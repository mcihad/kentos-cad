//! `cad.blocks.edit`'s library operations (docs/adr/0314 §3; plan/cad.md CAD-STYLE-14): a copy of a definition
//! under a new name (“Blok kopyala”), definitions given out as a `kentos.blocks` file's text with those they insert
//! (writing nothing), and a file's definitions taken in (“Blokları al”): a name the drawing has is kept, or with
//! `replace` redefined in its place; inserts in the file's definitions point at the drawing's definitions of their
//! names. Each write is one undo step; the block rules hold after it, or nothing is written.

use std::collections::HashMap;

use kentos_contracts::{
    BLOCKS_FORMAT, BLOCKS_VERSION, BlockDefinition, BlockEditOperation, BlockId, BlocksEdit,
    BlocksEditPlan, BlocksEdited, CommandResult, Entity,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::blocks_define::{block_rules, name};
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo steps' names.
pub const DUPLICATE: &str = "Blok kopyala";
pub const IMPORT: &str = "Blokları al";

/// Whether the operation is one of these.
pub fn is_library(operation: BlockEditOperation) -> bool {
    matches!(
        operation,
        BlockEditOperation::Duplicate | BlockEditOperation::Export | BlockEditOperation::Import
    )
}

/// What may be written: definitions to add and to put in their places, the file's names kept, an export's text.
struct Planned {
    added: Vec<BlockDefinition>,
    replaced: Vec<BlockDefinition>,
    skipped: Vec<String>,
    library: Option<String>,
}

fn unknown(id: BlockId, path: String) -> Stop {
    Stop::failed(error(
        codes::UNKNOWN_BLOCK,
        format!(
            "“{}” kimlikli blok çizimde tanımlı değil: silinmiş ya da başka bir çizimin olabilir. Çizimde tanımlı bir bloğun kimliğini verin.",
            id.to_text()
        ),
        Some(path),
    ))
}

fn invalid(why: String) -> Stop {
    Stop::failed(error(
        codes::INVALID_LIBRARY,
        format!(
            "Blok dosyası okunamadı: {why}. KentOS'un “Blokları dışa aktar”ının yazdığı bir kentos.blocks dosyası verin."
        ),
        Some("library".into()),
    ))
}

/// The definitions `ids` and those they insert, each once, a definition after those it inserts.
fn with_inserted(doc: &Document, ids: &[BlockId]) -> Vec<BlockDefinition> {
    fn put(doc: &Document, id: BlockId, out: &mut Vec<BlockDefinition>) {
        if out.iter().any(|b| b.id == id) {
            return;
        }
        let Some(b) = doc.block(id) else { return };
        for e in &b.entities {
            if let Entity::Insert(i) = e {
                put(doc, i.block, out);
            }
        }
        out.push(b.clone());
    }
    let mut out = Vec::new();
    for id in ids {
        put(doc, *id, &mut out);
    }
    out
}

/// The checks in the contract's order, and what the write would be; `fresh` gives a new definition's id.
fn check(
    doc: &Document,
    input: &BlocksEdit,
    mut fresh: impl FnMut() -> BlockId,
) -> Result<Planned, Stop> {
    let mut planned = Planned {
        added: Vec::new(),
        replaced: Vec::new(),
        skipped: Vec::new(),
        library: None,
    };
    match input.operation {
        BlockEditOperation::Duplicate => {
            let Some(id) = input.block else {
                return Err(Stop::failed(error(
                    codes::NO_BLOCK,
                    "Kopyalanacak blok verilmedi. Bloğun kimliğini verin.".into(),
                    Some("block".into()),
                )));
            };
            name(input.name.as_deref().unwrap_or(""))?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            let old = doc.block(id).ok_or_else(|| unknown(id, "block".into()))?;
            let mut copy = old.clone();
            copy.id = fresh();
            copy.name = input.name.clone().unwrap_or_default();
            let mut next: Vec<&BlockDefinition> = doc.blocks().iter().map(|b| &**b).collect();
            next.push(&copy);
            block_rules(&next)?;
            planned.added.push(copy);
        }
        BlockEditOperation::Export => {
            checks::revision(doc, input.expected_revision.as_deref())?;
            let ids: Vec<BlockId> = match &input.blocks {
                Some(list) => {
                    for (i, id) in list.iter().enumerate() {
                        if doc.block(*id).is_none() {
                            return Err(unknown(*id, format!("blocks[{i}]")));
                        }
                    }
                    list.clone()
                }
                None => doc.blocks().iter().map(|b| b.id).collect(),
            };
            let blocks = with_inserted(doc, &ids);
            let file = serde_json::json!({ "format": BLOCKS_FORMAT, "version": BLOCKS_VERSION, "blocks": blocks });
            planned.library = Some(kentos_contracts::canonical::canonical_json(&file));
        }
        BlockEditOperation::Import => {
            let Some(text) = input.library.as_deref().filter(|t| !t.trim().is_empty()) else {
                return Err(Stop::failed(error(
                    codes::NO_LIBRARY,
                    "Alınacak blok dosyasının metnini verin (library).".into(),
                    Some("library".into()),
                )));
            };
            let file: serde_json::Value =
                serde_json::from_str(text).map_err(|e| invalid(format!("JSON değil ({e})")))?;
            if file.get("format").and_then(serde_json::Value::as_str) != Some(BLOCKS_FORMAT) {
                return Err(invalid("biçimi kentos.blocks değil".into()));
            }
            match file.get("version").and_then(serde_json::Value::as_u64) {
                Some(v) if v == u64::from(BLOCKS_VERSION) => {}
                Some(v) => {
                    return Err(invalid(format!(
                        "sürümü {v}; bu KentOS {BLOCKS_VERSION}. sürümü okur"
                    )));
                }
                None => return Err(invalid("sürümü yok".into())),
            }
            let blocks: Vec<BlockDefinition> = serde_json::from_value(
                file.get("blocks")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )
            .map_err(|e| invalid(format!("blokları okunmuyor ({e})")))?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            take(
                doc,
                blocks,
                input.replace == Some(true),
                &mut planned,
                &mut fresh,
            )?;
        }
        _ => {}
    }
    Ok(planned)
}

/// The file's definitions into `planned`: names the drawing has kept or redefined, the others added; inserts in
/// them pointed at the drawing's ids; the block rules over the drawing as it would be.
fn take(
    doc: &Document,
    blocks: Vec<BlockDefinition>,
    replace: bool,
    planned: &mut Planned,
    fresh: &mut impl FnMut() -> BlockId,
) -> Result<(), Stop> {
    let mut ids: HashMap<BlockId, BlockId> = HashMap::new();
    let mut taken: Vec<(BlockDefinition, bool)> = Vec::new();
    for mut b in blocks {
        match doc.block_named(&b.name) {
            Some(have) if !replace => {
                ids.insert(b.id, have.id);
                planned.skipped.push(b.name.clone());
            }
            Some(have) => {
                ids.insert(b.id, have.id);
                b.id = have.id;
                taken.push((b, true));
            }
            None => {
                let id = if doc.block(b.id).is_some() || ids.values().any(|v| *v == b.id) {
                    fresh()
                } else {
                    b.id
                };
                ids.insert(b.id, id);
                b.id = id;
                taken.push((b, false));
            }
        }
    }
    for (b, _) in &mut taken {
        for e in &mut b.entities {
            if let Entity::Insert(i) = e {
                match ids.get(&i.block) {
                    Some(to) => i.block = *to,
                    None => {
                        return Err(invalid(format!(
                            "“{}” bloğu dosyada da çizimde de olmayan bir bloğu yerleştiriyor",
                            b.name
                        )));
                    }
                }
            }
        }
    }
    let mut next: Vec<&BlockDefinition> = doc
        .blocks()
        .iter()
        .map(|d| {
            taken
                .iter()
                .find(|(b, put)| *put && b.id == d.id)
                .map_or(&**d, |(b, _)| b)
        })
        .collect();
    next.extend(taken.iter().filter(|(_, put)| !put).map(|(b, _)| b));
    block_rules(&next).map_err(|stop| {
        let mut e = stop.into_error();
        e.path = Some("library".into());
        Stop::failed(e)
    })?;
    for (b, put) in taken {
        if put {
            planned.replaced.push(b);
        } else {
            planned.added.push(b);
        }
    }
    Ok(())
}

pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksEdit) -> CommandResult<()> {
    match check(cx.doc, input, || BlockId([0; 16])) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksEdit) -> CommandResult<BlocksEditPlan> {
    match check(cx.doc, input, || BlockId([0; 16])) {
        Ok(p) => CommandResult::Completed {
            output: BlocksEditPlan {
                changed: p.replaced,
                added: p.added,
                removed: Vec::new(),
                insert: None,
                deleted: Vec::new(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the plan as one undo step.
pub fn write(cx: &mut ExecutionContext<'_>, input: &BlocksEdit) -> CommandResult<BlocksEdited> {
    let p = match check(cx.doc, input, || BlockId(*Uuid::now_v7().as_bytes())) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let label = if input.operation == BlockEditOperation::Duplicate {
        DUPLICATE
    } else {
        IMPORT
    };
    let added: Vec<BlockId> = p.added.iter().map(|b| b.id).collect();
    let changed: Vec<BlockId> = p.replaced.iter().map(|b| b.id).collect();
    if !p.added.is_empty() || !p.replaced.is_empty() {
        let written = cx.doc.transact(label, |doc| {
            for b in p.replaced {
                doc.update_block(b).map_err(|r| r.to_string())?;
            }
            for b in p.added {
                doc.add_block(b).map_err(|r| r.to_string())?;
            }
            Ok::<(), String>(())
        });
        if let Err(message) = written {
            return Stop::failed(error(codes::BLOCK_REFUSED, message, None)).into();
        }
    }
    CommandResult::Completed {
        output: BlocksEdited {
            sync: None,
            changed,
            removed: Vec::new(),
            insert: None,
            id: None,
            deleted: Vec::new(),
            added,
            skipped: p.skipped,
            library: p.library,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
