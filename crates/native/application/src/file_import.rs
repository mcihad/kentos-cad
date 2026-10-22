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
use std::io::Read;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    ClosedPolylines, CommandResult, CommandWarning, DxfImported, DxfReadOptions, FileUnit,
    ImportDxf, ImportLayer, ImportResult, ImportedLayer, LayerNodeType, LayerStyle, RenamedBlock,
    UnitDecision, UnitSource, Workspace,
};
use kentos_domain::Document;
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::import_apply::{self, ImportPlan, LayerTarget, Styles, fold_turkish, layer_named};

/// The source codec, independent of the filename. UI and automation share it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CadFormat { Dxf, Dwg }

impl CadFormat {
    pub fn command(self) -> &'static str {
        match self { Self::Dxf => kentos_commands::FILE_IMPORT_DXF, Self::Dwg => kentos_commands::FILE_IMPORT_DWG }
    }
    pub fn label(self) -> &'static str { match self { Self::Dxf => "DXF", Self::Dwg => "DWG" } }
    pub fn step_name(self, file: &str) -> String { format!("{}: {file}", self.label()) }
}

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
/// paper mm at and the typeface a missing one takes (docs/adr/0183 §7); what
/// its closed polylines become, the project's type's choice when none is given
/// (docs/adr/0258 §1).
pub fn read_options(
    doc: &Document,
    file_unit: Option<FileUnit>,
    explode: bool,
    closed: Option<ClosedPolylines>,
) -> DxfReadOptions {
    let settings = doc.settings();
    DxfReadOptions {
        allow_polyline_loss: false,
        max_entities: 0,
        explode_blocks: explode,
        unit: (!settings.has_system()).then(|| settings.unit()),
        file_unit,
        scale: Some(settings.plot_scale),
        drawing_font: settings.drawing_font,
        closed_polylines: closed.unwrap_or_else(|| closed_by_type(doc)),
    }
}

/// What a project's type makes of a file's closed polylines (docs/adr/0258 §1): a CAD
/// drawing's stay lines, any other project takes them as areas.
pub fn closed_by_type(doc: &Document) -> ClosedPolylines {
    match doc.settings().project_type() {
        Some(Workspace::Cad) => ClosedPolylines::Line,
        _ => ClosedPolylines::Area,
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

/// Bounded from the open file descriptor, including a file that grows while it is read.
pub fn read_cad_bytes(path: &Path) -> Result<Vec<u8>, Box<kentos_commands::CommandError>> {
    let fail = |code: &str, message: String| Box::new(error(code, message, Some("path".into())));
    let file = std::fs::File::open(path).map_err(|e| fail(if e.kind() == std::io::ErrorKind::NotFound { codes::FILE_NOT_FOUND } else { codes::IMPORT_UNREADABLE }, format!("{} okunamadı: {e}. Dosyanın izinlerini denetleyin.", path.display())))?;
    let limit = kentos_formats::dxf::preflight::MAX_BYTES;
    if file.metadata().is_ok_and(|m| m.len() > limit as u64) {
        return Err(fail(codes::CAD_FILE_TOO_LARGE, "CAD dosyası 256 MiB sınırını aşıyor; dosyayı bölün.".into()));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).map_err(|e| fail(codes::IMPORT_UNREADABLE, format!("{} okunamadı: {e}", path.display())))?;
    if bytes.len() > limit { return Err(fail(codes::CAD_FILE_TOO_LARGE, "CAD dosyası okuma sırasında 256 MiB sınırını aştı.".into())); }
    Ok(bytes)
}

/// Reads the file with the project's options; the refusal when it is not there or not a DXF.
fn read_file(doc: &Document, input: &ImportDxf, format: CadFormat) -> Result<ReadDxf, Stop> {
    let path = Path::new(&input.path);
    if !path.exists() {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{} yok; dosyanın yolunu denetleyin.", input.path),
            "path",
        ));
    }
    let bytes = read_cad_bytes(path).map_err(|e| Stop::failed(*e))?;
    let name = path
        .file_name()
        .map_or_else(|| input.path.clone(), |n| n.to_string_lossy().into_owned());
    let mut options = read_options(
        doc,
        input.file_unit,
        input.explode_blocks,
        input.closed_polylines,
    );
    options.allow_polyline_loss = input.allow_polyline_loss;
    let mut result = match format {
        CadFormat::Dxf => kentos_formats::dxf::read_checked(&bytes, &options).map_err(|e| {
            refuse(e.code, format!("{name} DXF olarak okunamadı: {e}"), if e.code == codes::POLYLINE_GEOMETRY_LOSS { "allowPolylineLoss" } else { "path" })
        })?,
        CadFormat::Dwg => kentos_dwg::read(&bytes, &options).map_err(|e| {
            refuse(e.code, format!("{name}: {e}"), if e.code == codes::POLYLINE_GEOMETRY_LOSS { "allowPolylineLoss" } else { "path" })
        })?,
    };
    prepare_cad(doc, path, &mut result)?;
    if !input.allow_polyline_loss && let Some(error) = kentos_formats::dxf::polyline_policy::refusal(result.report.notes.iter().chain(&result.report.skipped)) { return Err(refuse(error.code, error.message, "allowPolylineLoss")); }
    let unusable = import_apply::unusable(&result.entities)
        .map(|(place, kind)| import_apply::unusable_text(place, kind))
        .or_else(|| import_apply::unusable_block(&result.blocks));
    if let Some(message) = unusable {
        return Err(refuse(codes::IMPORT_UNREADABLE, message, "path"));
    }
    Ok(ReadDxf::of(&name, &bytes, result))
}

/// The same resolved paths and sheet validation for a window's already-read bytes and a command's read.
fn prepare_cad(doc: &Document, path: &Path, result: &mut ImportResult) -> Result<(), Stop> {
    crate::cad_layouts::merge(doc, &result.layouts).map_err(|message| refuse(codes::IMPORT_UNREADABLE, message, "path"))?;
    for note in crate::cad_layouts::diagnostics(&result.layouts) {
        if !result.report.notes.iter().any(|n| n.what == note.what && n.reason == note.reason) { result.report.notes.push(note); }
    }
    for block in &mut result.blocks {
        if let Some(x) = &mut block.xref
            && !Path::new(&x.path).is_absolute() && !x.path.contains(':') {
                x.path = path.parent().unwrap_or_else(|| Path::new(".")).join(x.path.replace('\\', "/")).to_string_lossy().into_owned();
        }
    }
    Ok(())
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
    format: CadFormat,
    paper_only: bool,
) -> Result<ImportPlan, Stop> {
    let routing = Routing {
        skip_layers: &input.skip_layers,
        layers: &input.layers,
        all_onto: None,
        group: input.group.as_ref(),
    };
    let (layers, objects) = route_layers(doc, &routing, source)?;
    if !paper_only { nothing_to_import(
        file,
        source.is_empty(),
        layers.is_empty(),
        objects,
        &input.skip_layers,
    )?; }
    Ok(ImportPlan {
        label: format.step_name(file),
        layers,
        group: group_of(routing.group, file),
    })
}

/// How a file's layers are sent to the drawing's, the DXF's and the GIS files' alike (docs/adr/0257 §3, 0563 §1).
pub(crate) struct Routing<'a> {
    /// The file's layers (their names in the file) left out.
    pub skip_layers: &'a [String],
    /// A file's layer → the drawing's layer (its id).
    pub layers: &'a BTreeMap<String, String>,
    /// The drawing's layer every layer `layers` does not name goes onto, at `layer`.
    pub all_onto: Option<&'a String>,
    pub group: Option<&'a String>,
}

/// The group the new layers go into: the one named, else one named after the file, none when the name is empty.
pub(crate) fn group_of(group: Option<&String>, file: &str) -> Option<String> {
    match group {
        None => Some(file.to_owned()),
        Some(name) if name.trim().is_empty() => None,
        Some(name) => Some(name.trim().to_owned()),
    }
}

/// The checks of a target the objects go onto: a layer of the drawing, not a group, not locked.
fn existing_target(
    doc: &Document,
    id: &String,
    source: &str,
    at: &str,
) -> Result<LayerTarget, Stop> {
    match doc.layers().get(id) {
        None => Err(refuse(
            codes::LAYER_NOT_FOUND,
            format!(
                "“{id}” kimlikli katman çizimde yok; “{source}” katmanının nesnelerini var olan bir katmanın kimliğine yönlendirin."
            ),
            at,
        )),
        Some(node) if node.kind != LayerNodeType::Layer => Err(refuse(
            codes::NOT_A_LAYER,
            format!(
                "“{}” bir grup; nesneler yalnız katmana konur. Grubun altındaki bir katmanı verin.",
                node.name
            ),
            at,
        )),
        Some(node) if doc.layers().is_locked(id) => Err(refuse(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir katman verin.",
                node.name
            ),
            at,
        )),
        Some(_) => Ok(LayerTarget::Existing(id.clone())),
    }
}

/// Where each source layer goes, and how many objects go: the layers left out skipped, the routed ones (`layers`,
/// else `all_onto`) checked, the others onto the drawing's layer of the same name or a new one with the file's look.
pub(crate) fn route_layers(
    doc: &Document,
    routing: &Routing<'_>,
    source: &[ImportLayer],
) -> Result<(Vec<(String, LayerTarget)>, u64), Stop> {
    let skip: HashSet<String> = routing
        .skip_layers
        .iter()
        .map(|n| fold_turkish(n))
        .collect();
    let routes: HashMap<String, (&String, &String)> = routing
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
            existing_target(doc, id, &l.name, &format!("layers.{source}"))?
        } else if let Some(id) = routing.all_onto {
            existing_target(doc, id, &l.name, "layer")?
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
                plot: l.plot,
                frozen: l.frozen,
                name: l.name.clone(),
                style: Box::new(LayerStyle {
                    color: l.color.clone(),
                    line_type: l.line_type,
                    line_pattern: l.line_pattern.clone(),
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
    Ok((layers, objects))
}

/// The refusal of an import with no object left to take.
pub(crate) fn nothing_to_import(
    file: &str,
    no_layers_read: bool,
    none_chosen: bool,
    objects: u64,
    skip_layers: &[String],
) -> Result<(), Stop> {
    if objects > 0 {
        return Ok(());
    }
    Err(refuse(
        codes::NOTHING_TO_IMPORT,
        if no_layers_read {
            format!("{file} dosyasında alınacak nesne yok.")
        } else {
            format!("{file} dosyasının seçilen katmanlarında nesne yok; skipLayers'ı denetleyin.")
        },
        if none_chosen && !skip_layers.is_empty() {
            "skipLayers"
        } else {
            "path"
        },
    ))
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
) -> Result<ImportPlan, Box<kentos_commands::CommandError>> {
    import_plan_format(doc, input, layers, file, CadFormat::Dxf)
}

pub fn import_plan_format(
    doc: &Document, input: &ImportDxf, layers: &[ImportLayer], file: &str, format: CadFormat,
) -> Result<ImportPlan, Box<kentos_commands::CommandError>> {
    plan_of(doc, input, layers, file, format, false).map_err(|stop| Box::new(stop.into_error()))
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
    if kentos_formats::dxf::polyline_policy::refusal(result.report.notes.iter()).is_some() {
        out.push(CommandWarning {
            code: codes::IMPORT_LOSSES.into(),
            message: "Kabul edilen çoklu çizgi dönüşüm kayıpları report.notes içinde ayrıntılıdır.".into(),
            path: Some("allowPolylineLoss".into()),
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
fn write(doc: &mut Document, input: &ImportDxf, read: ReadDxf, format: CadFormat) -> CommandResult<DxfImported> {
    let closed_polylines = input
        .closed_polylines
        .unwrap_or_else(|| closed_by_type(doc));
    let plan = match plan_of(doc, input, &read.result.layers, &read.file, format, read.result.entities.is_empty() && read.result.layouts.iter().any(|l| !l.entities.is_empty() || !l.viewports.is_empty())) {
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
        layouts: result.layouts,
        text: result.text_styles,
        dimension: result.dimension_styles,
        line_types: result.line_types,
        line_type_scale: result.line_type_scale,
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
                closed_polylines,
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
fn check(doc: &Document, input: &ImportDxf, format: CadFormat) -> Result<ReadDxf, Stop> {
    check_path(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let read = read_file(doc, input, format)?;
    plan_of(doc, input, &read.result.layers, &read.file, format, read.result.entities.is_empty() && read.result.layouts.iter().any(|l| !l.entities.is_empty() || !l.viewports.is_empty()))?;
    Ok(read)
}

/// Checks `input` against the document and the file, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportDxf) -> CommandResult<()> {
    validate_format(cx, input, CadFormat::Dxf)
}

pub fn validate_format(cx: &ExecutionContext<'_>, input: &ImportDxf, format: CadFormat) -> CommandResult<()> {
    match check(cx.doc, input, format) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would take in, on a reading copy; the drawing is left as it is.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportDxf) -> CommandResult<DxfImported> {
    plan_format(cx, input, CadFormat::Dxf)
}

pub fn plan_format(cx: &ExecutionContext<'_>, input: &ImportDxf, format: CadFormat) -> CommandResult<DxfImported> {
    let read = match check(cx.doc, input, format) {
        Ok(read) => read,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut copy, input, read, format);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Reads the file and takes it in as one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportDxf) -> CommandResult<DxfImported> {
    execute_format(cx, input, CadFormat::Dxf)
}

pub fn execute_format(cx: &mut ExecutionContext<'_>, input: ImportDxf, format: CadFormat) -> CommandResult<DxfImported> {
    crate::call::recorded(
        cx,
        format.command(),
        1,
        input,
        |cx, input| write_execute(cx, input, None, format),
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
    execute_read_format(cx, input, read, CadFormat::Dxf)
}

pub fn execute_read_format(
    cx: &mut ExecutionContext<'_>, input: ImportDxf, read: ReadDxf, format: CadFormat,
) -> CommandResult<DxfImported> {
    crate::call::recorded(cx, format.command(), 1, input,
        move |cx, input| write_execute(cx, input, Some(read), format))
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
    record_progressive_format(doc, input, file, created, warnings, base, CadFormat::Dxf)
}

pub fn record_progressive_format(
    doc: &mut Document, input: &ImportDxf, file: &str, created: &[kentos_domain::Slot],
    warnings: &[CommandWarning], base: u64, format: CadFormat,
) -> u64 {
    let input = serde_json::to_value(input).unwrap_or(Value::Null);
    crate::call::record_written(
        doc,
        format.command(),
        1,
        &input,
        &format.step_name(file),
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
    format: CadFormat,
) -> CommandResult<DxfImported> {
    let mut read = match read {
        Some(read) => {
            if let Err(stop) = checks::revision(cx.doc, input.expected_revision.as_deref()) {
                return stop.into();
            }
            read
        }
        None => {
            let checked = check_path(&input)
                .and_then(|()| checks::revision(cx.doc, input.expected_revision.as_deref()))
                .and_then(|()| read_file(cx.doc, &input, format));
            match checked {
                Ok(read) => read,
                Err(stop) => return stop.into(),
            }
        }
    };
    if !input.allow_polyline_loss && let Some(error) = kentos_formats::dxf::polyline_policy::refusal(read.result.report.notes.iter().chain(&read.result.report.skipped)) {
        return refuse(error.code, error.message, "allowPolylineLoss").into();
    }
    if let Err(stop) = prepare_cad(cx.doc, Path::new(&input.path), &mut read.result) { return stop.into(); }
    if !input.allow_polyline_loss && let Some(error) = kentos_formats::dxf::polyline_policy::refusal(read.result.report.notes.iter().chain(&read.result.report.skipped)) { return refuse(error.code, error.message, "allowPolylineLoss").into(); }
    write(cx.doc, &input, read, format)
}
