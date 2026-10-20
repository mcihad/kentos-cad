//! `file.import.block` v1 (docs/adr/0316 §3; plan/cad.md CAD-BLOCK-07): blocks from a drawing file as one undo step
//! “Dosyadan blok: ad.kcad”, the semantic API of Çizimler arası alışveriş's block works (docs/adr/0193 §2, §3):
//!
//! - `drawing`: the file's objects but pictures and tables as one block (Dosyadan blok ekle), its base the lower
//!   left of their extent, named `name` or after the file;
//! - `blocks`: the file's blocks by name with those they place (Başka çizimden al's blocks); a name the drawing
//!   has is kept, or with `replace` redefined (Başka çizimden yeniden tanımla): every insert shows the file's.
//!
//! The rules are `kentos_domain::exchange`'s (`file_block`, `take`), the writing `exchange_write`'s (`put_nodes`,
//! `put_blocks`), as the desktop's windows have them. The file is a `.kcad` drawing: v2, or a v1 JSON file.

use std::path::Path;

use kentos_commands::{
    BlockFileImported, BlockFileMode, BlockId, CommandResult, DocumentSnapshotV1,
    DocumentSnapshotV2, ImportBlock,
};
use kentos_domain::Document;
use kentos_domain::exchange::{self, Picks, Same, block_key};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::exchange_write::{Counts, put_blocks, put_nodes};

/// The undo step's name: “Dosyadan blok: ad.kcad”.
pub fn step_name(file: &str) -> String {
    format!("Dosyadan blok: {file}")
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// A drawing file's content: a `.kcad` v2, or a v1 JSON file.
pub fn read_drawing(bytes: &[u8]) -> Result<DocumentSnapshotV2, String> {
    match kentos_kcad::decode(bytes) {
        Ok(doc) => Ok(doc),
        Err(v2) => {
            let text = std::str::from_utf8(bytes).map_err(|_| v2.to_string())?;
            let v1 = DocumentSnapshotV1::from_json(text).map_err(|_| v2.to_string())?;
            Ok(Document::from_snapshot(v1)?.to_snapshot_v2())
        }
    }
}

/// The file read, or the refusal.
fn read(input: &ImportBlock) -> Result<DocumentSnapshotV2, Stop> {
    let p = Path::new(&input.path);
    if input.path.trim().is_empty() || !p.is_absolute() {
        return Err(Stop::failed(error(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad).",
                input.path
            ),
            Some("path".into()),
        )));
    }
    let bytes = std::fs::read(p).map_err(|_| {
        Stop::failed(error(
            codes::FILE_NOT_FOUND,
            format!("{} okunamadı: dosya yok ya da açılamıyor.", input.path),
            Some("path".into()),
        ))
    })?;
    read_drawing(&bytes).map_err(|why| {
        Stop::failed(error(
            codes::IMPORT_UNREADABLE,
            format!(
                "{} bir KentOS çizimi olarak okunamadı: {why}",
                file_name(&input.path)
            ),
            Some("path".into()),
        ))
    })
}

/// What is written: the drawing with what came in, and what the answer says.
struct Checked {
    drawing: DocumentSnapshotV2,
    block: Option<BlockId>,
    skipped: Vec<String>,
    /// The pictures and tables a block cannot hold, left out of `drawing` mode.
    left: (usize, usize),
}

/// The file's blocks whose ids the drawing already has (the same file brought in twice) take new ids, their
/// inserts with them, so a new definition never lands on one the drawing has by id.
fn fresh_ids(ours: &DocumentSnapshotV2, theirs: &mut DocumentSnapshotV2) {
    let mut moved = std::collections::HashMap::new();
    for b in &mut theirs.blocks {
        if ours.blocks.iter().any(|m| m.id == b.id) {
            let id = BlockId(*kentos_domain::Uuid::now_v7().as_bytes());
            moved.insert(b.id, id);
            b.id = id;
        }
    }
    if moved.is_empty() {
        return;
    }
    let follow = |e: &mut kentos_commands::Entity| {
        if let kentos_commands::Entity::Insert(i) = e
            && let Some(to) = moved.get(&i.block)
        {
            i.block = *to;
        }
    };
    theirs
        .blocks
        .iter_mut()
        .flat_map(|b| b.entities.iter_mut())
        .for_each(follow);
    theirs.entities.iter_mut().for_each(follow);
}

fn check(doc: &Document, input: &ImportBlock) -> Result<Checked, Stop> {
    let mut theirs = read(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let ours = doc.to_snapshot_v2();
    fresh_ids(&ours, &mut theirs);
    match input.mode {
        BlockFileMode::Drawing => {
            let id = BlockId(*kentos_domain::Uuid::now_v7().as_bytes());
            let name = input
                .name
                .clone()
                .filter(|n| !checks::is_blank(n))
                .unwrap_or_else(|| stem(&input.path));
            let made = exchange::file_block(&ours, &theirs, &name, id).map_err(|why| {
                Stop::failed(error(codes::NOTHING_TO_IMPORT, why, Some("path".into())))
            })?;
            Ok(Checked {
                drawing: made.drawing,
                block: Some(id),
                skipped: Vec::new(),
                left: (made.images, made.tables),
            })
        }
        BlockFileMode::Blocks => {
            let all: Vec<String> = theirs.blocks.iter().map(|b| b.name.clone()).collect();
            let names = match &input.names {
                Some(list) => {
                    let mut named = Vec::with_capacity(list.len());
                    for (i, n) in list.iter().enumerate() {
                        let Some(found) = all.iter().find(|b| block_key(b) == block_key(n)) else {
                            return Err(Stop::failed(error(
                                codes::UNKNOWN_BLOCK,
                                format!("“{n}” bloğu {} içinde yok.", file_name(&input.path)),
                                Some(format!("names[{i}]")),
                            )));
                        };
                        if !named.contains(found) {
                            named.push(found.clone());
                        }
                    }
                    named
                }
                None => all,
            };
            if names.is_empty() {
                let (why, path) = if input.names.is_some() {
                    (
                        "Alınacak blok adı verilmedi; names en az bir ad ister.".to_owned(),
                        "names",
                    )
                } else {
                    (
                        format!("{} içinde blok yok.", file_name(&input.path)),
                        "path",
                    )
                };
                return Err(Stop::failed(error(
                    codes::NOTHING_TO_IMPORT,
                    why,
                    Some(path.into()),
                )));
            }
            let replace = input.replace == Some(true);
            let skipped = if replace {
                Vec::new()
            } else {
                names
                    .iter()
                    .filter(|n| {
                        ours.blocks
                            .iter()
                            .any(|b| block_key(&b.name) == block_key(n))
                    })
                    .cloned()
                    .collect()
            };
            let picks = Picks {
                blocks: names,
                ..Picks::default()
            };
            let same = if replace { Same::Replace } else { Same::Skip };
            let drawing = exchange::take(&ours, &theirs, &picks, same).map_err(|why| {
                Stop::failed(error(codes::IMPORT_UNREADABLE, why, Some("path".into())))
            })?;
            Ok(Checked {
                drawing,
                block: None,
                skipped,
                left: (0, 0),
            })
        }
    }
}

/// Checks `input` against the document, reading the file, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportBlock) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would add and redefine, writing nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportBlock) -> CommandResult<BlockFileImported> {
    let mut probe = cx.doc.clone();
    match write(&mut probe, input) {
        CommandResult::Completed {
            mut output,
            warnings,
        } => {
            // Nothing is written: the revision is the drawing's own.
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        other => other,
    }
}

/// Reads the file and writes what it brings as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ImportBlock,
) -> CommandResult<BlockFileImported> {
    crate::call::recorded(
        cx,
        kentos_commands::FILE_IMPORT_BLOCK,
        kentos_commands::FILE_IMPORT_BLOCK_VERSION,
        input,
        |cx, input| write(cx.doc, &input),
    )
}

fn write(doc: &mut Document, input: &ImportBlock) -> CommandResult<BlockFileImported> {
    let checked = match check(doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let before: Vec<kentos_commands::BlockDefinition> =
        doc.blocks().iter().map(|b| (**b).clone()).collect();
    let label = step_name(&file_name(&input.path));
    let group = doc.begin_group(&label);
    let mut counts = Counts::default();
    let done = put_nodes(doc, &checked.drawing.layers, None, &mut counts, &label)
        .and_then(|()| put_blocks(doc, &checked.drawing.blocks, &mut counts));
    if let Err(why) = done {
        doc.cancel_group(group);
        return Stop::failed(error(
            codes::BLOCK_REFUSED,
            format!("{why} Hiçbir şey alınmadı."),
            Some("path".into()),
        ))
        .into();
    }
    doc.end_group(group);
    // The text and dimension styles and the library items the pieces use are settings, as Başka çizimden al has
    // them (docs/adr/0193 §2): written beside the step, not in it.
    let mut settings = doc.settings().clone();
    let theirs = &checked.drawing.settings;
    if settings.text_styles != theirs.text_styles
        || settings.dimension_styles != theirs.dimension_styles
    {
        settings.text_styles.clone_from(&theirs.text_styles);
        settings
            .dimension_styles
            .clone_from(&theirs.dimension_styles);
        doc.set_settings(settings);
    }
    if doc.styles() != &checked.drawing.styles {
        doc.set_styles(checked.drawing.styles.clone());
    }
    let added = doc
        .blocks()
        .iter()
        .map(|b| b.id)
        .filter(|id| !before.iter().any(|b| b.id == *id))
        .collect();
    let changed = doc
        .blocks()
        .iter()
        .filter(|b| before.iter().any(|old| old.id == b.id && old != &***b))
        .map(|b| b.id)
        .collect();
    let mut warnings = Vec::new();
    let (images, tables) = checked.left;
    if images + tables > 0 {
        let what: Vec<String> = [
            (images > 0).then(|| format!("{images} resim")),
            (tables > 0).then(|| format!("{tables} tablo")),
        ]
        .into_iter()
        .flatten()
        .collect();
        warnings.push(kentos_commands::CommandWarning {
            code: codes::OBJECTS_LEFT_OUT.into(),
            message: format!("{} bloğa konamadığı için alınmadı.", what.join(" ve ")),
            path: Some("path".into()),
        });
    }
    CommandResult::Completed {
        output: BlockFileImported {
            block: checked.block,
            added,
            changed,
            skipped: checked.skipped,
            layers: u32::try_from(counts.layers).unwrap_or(u32::MAX),
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}
