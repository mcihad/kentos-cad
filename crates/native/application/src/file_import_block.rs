//! `file.import.block` v1 (docs/adr/0316 §3; plan/cad.md CAD-BLOCK-07): blocks from a drawing file as one undo step
//! “Dosyadan blok: ad.kcad”, the semantic API of Çizimler arası alışveriş's block works (docs/adr/0193 §2, §3):
//!
//! - `drawing`: the file's objects but pictures and tables as one block (Dosyadan blok ekle), its base the lower
//!   left of their extent, named `name` or after the file;
//! - `blocks`: the file's blocks by name with those they place (Başka çizimden al's blocks); a name the drawing
//!   has is kept, or with `replace` redefined (Başka çizimden yeniden tanımla): every insert shows the file's.
//!
//! The rules are `kentos_domain::exchange`'s (`file_block`, `take`), the writing `exchange_write`'s (`put_nodes`,
//! `put_blocks`), as the desktop's windows have them. The file is a `.kcad` drawing (v2, or a v1 JSON file), or a
//! DXF or DWG file read as Aç reads it (`cad_open`, docs/adr/0726 §2): in the unit it names, or `sourceUnit`; one
//! naming none without `sourceUnit` is refused. `category` goes on the cards of the blocks taken that have none, and
//! the answer gives each block's size.

use std::path::Path;

use kentos_commands::{
    BlockCard, BlockFileImported, BlockFileMode, BlockId, BlockSize, CommandResult,
    DocumentSnapshotV1, DocumentSnapshotV2, DrawingUnit, FileUnit, ImportBlock, UnitDecision,
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

/// The CAD format `path` names by its extension: a DXF or a DWG; none for a KentOS drawing.
fn cad_format(path: &str) -> Option<crate::cad_open::Format> {
    use crate::cad_open::Format;
    Format::of(Path::new(path)).filter(|f| matches!(f, Format::Dxf | Format::Dwg))
}

/// The reader's unit for a drawing unit.
fn file_unit(u: DrawingUnit) -> FileUnit {
    match u {
        DrawingUnit::Mm => FileUnit::Mm,
        DrawingUnit::Cm => FileUnit::Cm,
        DrawingUnit::M => FileUnit::M,
        DrawingUnit::In => FileUnit::In,
        DrawingUnit::Ft => FileUnit::Ft,
    }
}

/// A block's unit for a DXF `$INSUNITS` code: those a block may name (docs/adr/0315 §4).
fn block_units_of(code: i64) -> Option<kentos_commands::BlockUnits> {
    use kentos_commands::BlockUnits as B;
    match code {
        1 => Some(B::In),
        2 => Some(B::Ft),
        4 => Some(B::Mm),
        5 => Some(B::Cm),
        6 => Some(B::M),
        _ => None,
    }
}

/// What a file brings: its drawing, a CAD file's unit as decided, and the objects its reader could not take.
struct Read {
    drawing: DocumentSnapshotV2,
    unit: Option<UnitDecision>,
    unread: Vec<String>,
}

/// The input's own faults, before the file is read: the path, the category, a unit for a KentOS drawing.
fn input_checks(input: &ImportBlock) -> Result<(), Stop> {
    if input.path.trim().is_empty() || !Path::new(&input.path).is_absolute() {
        return Err(Stop::failed(error(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad).",
                input.path
            ),
            Some("path".into()),
        )));
    }
    if let Some(c) = &input.category {
        let card = BlockCard {
            category: Some(c.clone()),
            tags: Vec::new(),
            version: 1,
        };
        let bad_part = c.split('/').any(|part| part.trim().is_empty());
        if let Some(why) = kentos_commands::blocks::card_problem(&card)
            .or_else(|| bad_part.then(|| format!("“{c}” kategorisinin bir parçası boş; “/” iki ad arasında olur.")))
        {
            return Err(Stop::failed(error(
                codes::INVALID_BLOCK_CARD,
                format!("Kategori kurallarına uymuyor: {why}"),
                Some("category".into()),
            )));
        }
    }
    if input.source_unit.is_some() && cad_format(&input.path).is_none() {
        return Err(Stop::failed(error(
            codes::INVALID_INPUT,
            format!(
                "{} bir KentOS çizimi; birimini kendisi taşır. sourceUnit yalnız DXF ve DWG dosyasında verilir.",
                file_name(&input.path)
            ),
            Some("sourceUnit".into()),
        )));
    }
    Ok(())
}

/// The file read, or the refusal.
fn read(input: &ImportBlock) -> Result<Read, Stop> {
    input_checks(input)?;
    let bytes = std::fs::read(Path::new(&input.path)).map_err(|_| {
        Stop::failed(error(
            codes::FILE_NOT_FOUND,
            format!("{} okunamadı: dosya yok ya da açılamıyor.", input.path),
            Some("path".into()),
        ))
    })?;
    let Some(format) = cad_format(&input.path) else {
        let drawing = read_drawing(&bytes).map_err(|why| {
            Stop::failed(error(
                codes::IMPORT_UNREADABLE,
                format!(
                    "{} bir KentOS çizimi olarak okunamadı: {why}",
                    file_name(&input.path)
                ),
                Some("path".into()),
            ))
        })?;
        return Ok(Read {
            drawing,
            unit: None,
            unread: Vec::new(),
        });
    };
    let opened = crate::cad_open::read_in(
        Path::new(&input.path),
        &bytes,
        format,
        input.source_unit.map(file_unit),
        &mut kentos_formats::watch::Quiet,
    )
    .map_err(|r| {
        let code = if r.code == codes::UNSUPPORTED_CAD_VERSION {
            codes::UNSUPPORTED_CAD_VERSION
        } else {
            codes::IMPORT_UNREADABLE
        };
        Stop::failed(error(
            code,
            format!("{} okunamadı: {}", file_name(&input.path), r.message),
            Some("path".into()),
        ))
    })?;
    // A file naming no unit is not given one by guess (docs/adr/0726 §2).
    if input.source_unit.is_none() && opened.unit.as_ref().is_none_or(|u| u.assumed) {
        return Err(Stop::failed(error(
            codes::SOURCE_UNIT_UNKNOWN,
            format!(
                "{} birimini bildirmiyor; blokların boyu bilinmeden alınmaz. Dosyanın birimini verin (sourceUnit: mm, cm, m, in, ft).",
                file_name(&input.path)
            ),
            Some("sourceUnit".into()),
        )));
    }
    let unread = opened
        .report
        .skipped
        .iter()
        .map(|item| format!("{} ({})", item.what, item.count))
        .collect();
    Ok(Read {
        drawing: opened.document.to_snapshot_v2(),
        unit: opened.unit,
        unread,
    })
}

/// What is written: the drawing with what came in, and what the answer says.
struct Checked {
    drawing: DocumentSnapshotV2,
    block: Option<BlockId>,
    skipped: Vec<String>,
    /// The pictures and tables a block cannot hold, left out of `drawing` mode.
    left: (usize, usize),
    unit: Option<UnitDecision>,
    unread: Vec<String>,
}

/// `category` on the cards of the blocks coming in that have none: those the drawing has not, or has otherwise.
fn categorised(ours: &DocumentSnapshotV2, drawing: &mut DocumentSnapshotV2, category: &str) {
    for b in &mut drawing.blocks {
        if ours.blocks.iter().any(|o| o.id == b.id && o == b) {
            continue;
        }
        let card = b.card.get_or_insert_with(|| BlockCard {
            category: None,
            tags: Vec::new(),
            version: 1,
        });
        if card.category.is_none() {
            card.category = Some(category.to_owned());
        }
    }
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
    let mut checked = check_read(doc, input)?;
    if let Some(category) = &input.category {
        let ours = doc.to_snapshot_v2();
        categorised(&ours, &mut checked.drawing, category);
    }
    Ok(checked)
}

fn check_read(doc: &Document, input: &ImportBlock) -> Result<Checked, Stop> {
    let Read {
        drawing: mut theirs,
        unit,
        unread,
    } = read(input)?;
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
            let mut made = exchange::file_block(&ours, &theirs, &name, id).map_err(|why| {
                Stop::failed(error(codes::NOTHING_TO_IMPORT, why, Some("path".into())))
            })?;
            // A CAD file's drawing as a block is drawn in the file's unit (docs/adr/0726 §2).
            if let Some(units) = unit.as_ref().and_then(|u| block_units_of(u.used))
                && let Some(b) = made.drawing.blocks.iter_mut().find(|b| b.id == id)
            {
                b.units = Some(units);
            }
            Ok(Checked {
                drawing: made.drawing,
                block: Some(id),
                skipped: Vec::new(),
                left: (made.images, made.tables),
                unit,
                unread,
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
                unit,
                unread,
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
    let added: Vec<BlockId> = doc
        .blocks()
        .iter()
        .map(|b| b.id)
        .filter(|id| !before.iter().any(|b| b.id == *id))
        .collect();
    let changed: Vec<BlockId> = doc
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
    if !checked.unread.is_empty() {
        warnings.push(kentos_commands::CommandWarning {
            code: codes::OBJECTS_LEFT_OUT.into(),
            message: format!(
                "Dosyanın okunamayan nesneleri alınmadı: {}.",
                checked.unread.join(", ")
            ),
            path: Some("path".into()),
        });
    }
    let sizes = sizes_of(doc, |id| added.contains(&id) || changed.contains(&id));
    CommandResult::Completed {
        output: BlockFileImported {
            block: checked.block,
            added,
            changed,
            skipped: checked.skipped,
            layers: u32::try_from(counts.layers).unwrap_or(u32::MAX),
            sizes,
            unit: checked.unit,
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

/// The sizes of the drawing's blocks `which` takes, in its order: those that draw something.
fn sizes_of(doc: &Document, which: impl Fn(BlockId) -> bool) -> Vec<BlockSize> {
    let core = crate::blocks::core_blocks(doc.blocks());
    let font = crate::geometry::drawing_font(doc.settings().drawing_font);
    doc.blocks()
        .iter()
        .filter(|b| which(b.id))
        .filter_map(|b| {
            let [x0, y0, x1, y1] = crate::blocks::definition_box_in(&core, font, b.id, None)?;
            Some(BlockSize {
                block: b.id,
                name: b.name.clone(),
                units: b.units,
                width: x1 - x0,
                depth: y1 - y0,
            })
        })
        .collect()
}
