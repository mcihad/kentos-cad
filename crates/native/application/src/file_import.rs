//! `file.import.dxf` v1 (docs/adr/0257 §3): a DXF taken into the drawing as
//! the desktop's DXF içe aktar takes it, as one undo step “DXF: ad.dxf”. The
//! file is read by the shared reader with the project's options
//! ([`read_options`]: its unit, its plot scale, its typeface) and the unit
//! the caller chose; each source layer goes onto the layer `layers` names,
//! else onto the drawing's layer of the same name, else onto a new layer in
//! the import's group; the rules of putting a reader's result in are
//! [`crate::import_apply`]'s, which the desktop's windows use too.
//!
//! The desktop's window reads the file on a thread of its own and shows
//! what it read; it writes with [`execute_read`], the command with the file
//! it already read, so the window and a script write alike and the journal
//! names the command. The shared cases are
//! `fixtures/commands/v1/file.import.dxf.json`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use kentos_contracts::encoding::sha256_hex;
use kentos_contracts::{
    CommandResult, CommandWarning, DxfImported, DxfReadOptions, FileUnit, ImportDxf, ImportLayer,
    ImportResult, ImportedLayer, LayerNodeType, LayerStyle, RenamedBlock, UnitDecision, UnitSource,
};
use kentos_domain::Document;
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::import_apply::{self, ImportPlan, LayerTarget, Styles, fold_turkish, layer_named};

/// The undo step's name: “DXF: ad.dxf”.
pub fn step_name(file: &str) -> String {
    format!("DXF: {file}")
}

/// A DXF read for an import, and what the file was.
#[derive(Clone, Debug)]
pub struct ReadDxf {
    pub result: ImportResult,
    /// The file's name, its size and its SHA-256.
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
}

impl ReadDxf {
    /// What reading `bytes` (the file `name`) gave.
    pub fn of(name: &str, bytes: &[u8], result: ImportResult) -> Self {
        Self {
            result,
            file: name.to_owned(),
            bytes: bytes.len() as u64,
            sha256: sha256_hex(bytes),
        }
    }
}

/// How the drawing reads a DXF, the window's and the command's alike: a local
/// project's unit, none with a coordinate system (docs/adr/0165 §2); the unit
/// chosen for the file (docs/adr/0257 §1); the plot scale its styles are
/// paper mm at and the typeface a missing one takes (docs/adr/0183 §7).
pub fn read_options(doc: &Document, file_unit: Option<FileUnit>, explode: bool) -> DxfReadOptions {
    let settings = doc.settings();
    DxfReadOptions {
        max_entities: 0,
        explode_blocks: explode,
        unit: (!settings.has_system()).then(|| settings.unit()),
        file_unit,
        scale: Some(settings.plot_scale),
        drawing_font: settings.drawing_font,
    }
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The path: absolute and not a folder.
fn check_path(input: &ImportDxf) -> Result<(), Stop> {
    let path = Path::new(&input.path);
    if input.path.trim().is_empty() || !path.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” mutlak bir dosya yolu değil; okunacak dosyanın tam yolunu verin (/klasör/ad.dxf).",
                input.path
            ),
            "path",
        ));
    }
    if path.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "{} bir klasör; okunacak DXF dosyasının adını da verin.",
                input.path
            ),
            "path",
        ));
    }
    Ok(())
}

/// Reads the file with the project's options; the refusal when it is not there or not a DXF.
fn read_file(doc: &Document, input: &ImportDxf) -> Result<ReadDxf, Stop> {
    let path = Path::new(&input.path);
    if !path.exists() {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{} yok; dosyanın yolunu denetleyin.", input.path),
            "path",
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!(
                "{} okunamadı: {e}. Dosyanın izinlerini denetleyin.",
                input.path
            ),
            "path",
        )
    })?;
    let name = path
        .file_name()
        .map_or_else(|| input.path.clone(), |n| n.to_string_lossy().into_owned());
    let options = read_options(doc, input.file_unit, input.explode_blocks);
    let result = kentos_formats::dxf::read(&bytes, &options).map_err(|e| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!("{name} DXF olarak okunamadı: {e}"),
            "path",
        )
    })?;
    let unusable = import_apply::unusable(&result.entities)
        .map(|(place, kind)| import_apply::unusable_text(place, kind))
        .or_else(|| import_apply::unusable_block(&result.blocks));
    if let Some(message) = unusable {
        return Err(refuse(codes::IMPORT_UNREADABLE, message, "path"));
    }
    Ok(ReadDxf::of(&name, &bytes, result))
}

/// Where each source layer goes, the plan [`import_apply`] writes by: the
/// layers left out skipped, the routed ones checked, the others onto the
/// drawing's layer of the same name or a new one; the refusal of a target
/// the objects cannot go onto, or of an import with nothing to take.
fn plan_of(
    doc: &Document,
    input: &ImportDxf,
    source: &[ImportLayer],
    file: &str,
) -> Result<ImportPlan, Stop> {
    let skip: HashSet<String> = input.skip_layers.iter().map(|n| fold_turkish(n)).collect();
    let routes: HashMap<String, (&String, &String)> = input
        .layers
        .iter()
        .map(|(source, id)| (fold_turkish(source), (source, id)))
        .collect();
    let mut layers = Vec::new();
    let mut objects = 0u64;
    for l in source {
        let key = fold_turkish(&l.name);
        if skip.contains(&key) {
            continue;
        }
        let target = if let Some((source, id)) = routes.get(&key) {
            let at = format!("layers.{source}");
            match doc.layers().get(id) {
                None => {
                    return Err(refuse(
                        codes::LAYER_NOT_FOUND,
                        format!(
                            "“{id}” kimlikli katman çizimde yok; “{}” katmanının nesnelerini var olan bir katmanın kimliğine yönlendirin.",
                            l.name
                        ),
                        &at,
                    ));
                }
                Some(node) if node.kind != LayerNodeType::Layer => {
                    return Err(refuse(
                        codes::NOT_A_LAYER,
                        format!(
                            "“{}” bir grup; nesneler yalnız katmana konur. Grubun altındaki bir katmanı verin.",
                            node.name
                        ),
                        &at,
                    ));
                }
                Some(node) if doc.layers().is_locked(id) => {
                    return Err(refuse(
                        codes::LAYER_LOCKED,
                        format!(
                            "“{}” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir katman verin.",
                            node.name
                        ),
                        &at,
                    ));
                }
                Some(_) => LayerTarget::Existing((*id).clone()),
            }
        } else if let Some(node) = layer_named(doc, &l.name) {
            if doc.layers().is_locked(&node.id) {
                return Err(refuse(
                    codes::LAYER_LOCKED,
                    format!(
                        "Dosyanın “{}” katmanı çizimdeki aynı adlı “{}” katmanına gider ama o katman kilitli. Kilidi açın, katmanı skipLayers ile dışarıda bırakın ya da layers ile başka bir katmana yönlendirin.",
                        l.name, node.name
                    ),
                    "layers",
                ));
            }
            LayerTarget::Existing(node.id.clone())
        } else {
            let default = kentos_domain::default_style();
            LayerTarget::New {
                name: l.name.clone(),
                style: Box::new(LayerStyle {
                    color: l.color.clone(),
                    line_type: l.line_type,
                    // None or 0 is the default weight, as the web's import reads it.
                    line_weight: l
                        .line_weight
                        .filter(|w| *w != 0.0)
                        .unwrap_or(default.line_weight),
                    ..default
                }),
                visible: l.visible,
                locked: l.locked,
                fields: l.fields.clone(),
            }
        };
        objects += u64::from(l.count);
        layers.push((l.name.clone(), target));
    }
    if objects == 0 {
        return Err(refuse(
            codes::NOTHING_TO_IMPORT,
            if source.is_empty() {
                format!("{file} dosyasında alınacak nesne yok.")
            } else {
                format!(
                    "{file} dosyasının seçilen katmanlarında nesne yok; skipLayers'ı denetleyin."
                )
            },
            if layers.is_empty() && !input.skip_layers.is_empty() {
                "skipLayers"
            } else {
                "path"
            },
        ));
    }
    let group = match &input.group {
        None => Some(file.to_owned()),
        Some(name) if name.trim().is_empty() => None,
        Some(name) => Some(name.trim().to_owned()),
    };
    Ok(ImportPlan {
        label: step_name(file),
        layers,
        group,
    })
}

/// The plan `input` takes `read` (the file `file`) in by, the command's own
/// ([`execute`]'s): for a host that writes the objects itself, a frame at
/// a time (the desktop's large import, docs/adr/0138). The refusal as the
/// command would answer it.
pub fn import_plan(
    doc: &Document,
    input: &ImportDxf,
    layers: &[ImportLayer],
    file: &str,
) -> Result<ImportPlan, Box<kentos_contracts::CommandError>> {
    plan_of(doc, input, layers, file).map_err(|stop| Box::new(stop.into_error()))
}

/// What the command says of the read: the unit's warnings (docs/adr/0257
/// §1, §2) and what the reader could not take.
fn warnings_of(result: &ImportResult) -> Vec<CommandWarning> {
    let mut out: Vec<CommandWarning> = result
        .unit
        .as_ref()
        .map(UnitDecision::warnings)
        .unwrap_or_default()
        .into_iter()
        .map(|(code, message)| CommandWarning {
            code: code.to_owned(),
            message,
            path: Some("fileUnit".into()),
        })
        .collect();
    let skipped = &result.report.skipped;
    if !skipped.is_empty() {
        let count: u64 = skipped.iter().map(|s| u64::from(s.count.max(1))).sum();
        let kinds: Vec<&str> = skipped.iter().take(3).map(|s| s.what.as_str()).collect();
        out.push(CommandWarning {
            code: codes::IMPORT_LOSSES.into(),
            message: format!(
                "Okuyucu dosyanın {count} öğesini alamadı ({}{}); nedenleri report.skipped'te.",
                kinds.join(", "),
                if skipped.len() > kinds.len() {
                    " …"
                } else {
                    ""
                }
            ),
            path: None,
        });
    }
    out
}

/// The unit a file without a decision was read in (a reader that says
/// none): metres as they are.
fn metres() -> UnitDecision {
    UnitDecision {
        declared: None,
        used: 6,
        name: "metre".into(),
        source: UnitSource::Metres,
        factor: 1.0,
        assumed: false,
        blocks: Vec::new(),
    }
}

/// Puts `read` into `doc` by `input` (one step: the layers, the blocks, the
/// objects and the styles).
fn write(doc: &mut Document, input: &ImportDxf, read: ReadDxf) -> CommandResult<DxfImported> {
    let plan = match plan_of(doc, input, &read.result.layers, &read.file) {
        Ok(plan) => plan,
        Err(stop) => return stop.into(),
    };
    let ReadDxf {
        result,
        file,
        bytes,
        sha256,
    } = read;
    let warnings = warnings_of(&result);
    let report = serde_json::to_value(&result.report).unwrap_or(Value::Null);
    let unit = result.unit.clone().unwrap_or_else(metres);
    let styles = Styles {
        text: result.text_styles,
        dimension: result.dimension_styles,
    };
    match import_apply::apply_styled_import(doc, result.entities, result.blocks, styles, &plan) {
        Err(message) => CommandResult::Failed {
            error: error(codes::IMPORT_UNREADABLE, message, Some("path".into())),
        },
        Ok(applied) => CommandResult::Completed {
            output: DxfImported {
                file,
                bytes,
                sha256,
                unit,
                objects: u32::try_from(applied.slots.len()).unwrap_or(u32::MAX),
                targets: applied.targets.into_iter().collect::<BTreeMap<_, _>>(),
                layers: applied
                    .made
                    .into_iter()
                    .map(|(id, name)| ImportedLayer { id, name })
                    .collect(),
                group: applied.group,
                blocks: u32::try_from(applied.blocks).unwrap_or(u32::MAX),
                renamed: applied
                    .renamed
                    .into_iter()
                    .map(|(from, to)| RenamedBlock { from, to })
                    .collect(),
                text_styles: u32::try_from(applied.styles.0).unwrap_or(u32::MAX),
                dimension_styles: u32::try_from(applied.styles.1).unwrap_or(u32::MAX),
                report,
                revision: doc.revision().to_string(),
            },
            warnings,
        },
    }
}

/// Every check, the file read: what plan and execute stop with.
fn check(doc: &Document, input: &ImportDxf) -> Result<ReadDxf, Stop> {
    check_path(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let read = read_file(doc, input)?;
    plan_of(doc, input, &read.result.layers, &read.file)?;
    Ok(read)
}

/// Checks `input` against the document and the file, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportDxf) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would take in, on a reading copy; the drawing is left as it is.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportDxf) -> CommandResult<DxfImported> {
    let read = match check(cx.doc, input) {
        Ok(read) => read,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut copy, input, read);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Reads the file and takes it in as one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportDxf) -> CommandResult<DxfImported> {
    crate::call::recorded(
        cx,
        kentos_contracts::FILE_IMPORT_DXF,
        kentos_contracts::FILE_IMPORT_DXF_VERSION,
        input,
        |cx, input| write_execute(cx, input, None),
    )
}

/// [`execute`] with the file its host already read (the desktop's window
/// reads it on a thread of its own and shows it first): `read` must be
/// what [`read_options`] reads of `input.path` with `input`'s unit and
/// blocks. The journal names the command and its input, as [`execute`]'s.
pub fn execute_read(
    cx: &mut ExecutionContext<'_>,
    input: ImportDxf,
    read: ReadDxf,
) -> CommandResult<DxfImported> {
    crate::call::recorded(
        cx,
        kentos_contracts::FILE_IMPORT_DXF,
        kentos_contracts::FILE_IMPORT_DXF_VERSION,
        input,
        move |cx, input| write_execute(cx, input, Some(read)),
    )
}

/// The journal's row of a large import the desktop wrote a frame at a time
/// with [`crate::import_apply::Progressive`] (docs/adr/0138), by the same
/// rules: the command, its input and the objects made, as [`execute`] would
/// have recorded them. `base` is the revision before the import.
pub fn record_progressive(
    doc: &mut Document,
    input: &ImportDxf,
    file: &str,
    created: &[kentos_domain::Slot],
    warnings: &[CommandWarning],
    base: u64,
) -> u64 {
    let input = serde_json::to_value(input).unwrap_or(Value::Null);
    crate::call::record_written(
        doc,
        kentos_contracts::FILE_IMPORT_DXF,
        kentos_contracts::FILE_IMPORT_DXF_VERSION,
        &input,
        &step_name(file),
        created,
        warnings,
        base,
    )
}

/// What the command says of a file its host read ([`execute_read`]'s
/// warnings, for a host that writes the objects itself).
pub fn warnings_of_read(read: &ImportResult) -> Vec<CommandWarning> {
    warnings_of(read)
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ImportDxf,
    read: Option<ReadDxf>,
) -> CommandResult<DxfImported> {
    let read = match read {
        Some(read) => {
            if let Err(stop) = checks::revision(cx.doc, input.expected_revision.as_deref()) {
                return stop.into();
            }
            read
        }
        None => {
            let checked = check_path(&input)
                .and_then(|()| checks::revision(cx.doc, input.expected_revision.as_deref()))
                .and_then(|()| read_file(cx.doc, &input));
            match checked {
                Ok(read) => read,
                Err(stop) => return stop.into(),
            }
        }
    };
    write(cx.doc, &input, read)
}
