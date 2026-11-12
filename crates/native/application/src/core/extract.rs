//! `cad.extract.run` v1 and `cad.extract.table` v1 (docs/adr/1030, plan/cad-advanced.md `CAD-ADV-13`): a data
//! extraction recipe worked out over the open drawing and other drawings read only (`kentos_formats::extract`), and
//! written as a table that keeps its recipe (one undo step “Veri çıkarımı tablosu”). Tabloyu güncelle writes such a
//! table again ([`refresh`]): the extracted cells from the sources as they are now; a cell changed by hand kept while
//! its source is as it was, the user's further columns, styles, fields, formulas and row heights following each row's
//! key; a hand edit whose source changed too, or a row with the user's words that the sources no longer give, keeps
//! the whole table as it is (`refresh_kept`) unless the rule is to write over.
//!
//! The other drawings and the joined file are read (their bytes and SHA-256), never written. The shared cases are
//! `fixtures/commands/v1/cad.extract.{run,table}.json`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, CommandWarning, DocumentSnapshotV2, Entity, EntityBase, ExtractNotes,
    ExtractRan, ExtractRead, ExtractRow, ExtractRun, ExtractTable, ExtractTablePlan, ExtractTabled,
    ExtractionRecipe, ExtractionSource, SourceOverrides, TableAlign, TableEntity, TableSource,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_formats::extract::{Decimals, Drawing, Extracted};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Veri çıkarımı tablosu";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn warning(code: &str, message: String) -> CommandWarning {
    CommandWarning {
        code: code.to_owned(),
        message,
        path: None,
    }
}

/// Where a path the recipe names is: as given when absolute, else from `base`.
fn resolved(path: &str, base: Option<&str>) -> PathBuf {
    let p = Path::new(path);
    match base {
        Some(b) if !p.is_absolute() => Path::new(b).join(p),
        _ => p.to_path_buf(),
    }
}

/// A path made comparable: its canonical form when it exists, else as written.
fn same_place(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// A file's name, for a Drawing column.
fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// A source that could not be read: its code and words.
pub struct Unread {
    pub code: &'static str,
    pub words: String,
    pub path: String,
}

/// The other drawings and the joined file a recipe reads, read.
pub struct Sources {
    drawings: Vec<(String, PathBuf, DocumentSnapshotV2)>,
    join: Option<Vec<Vec<String>>>,
    pub read: Vec<ExtractRead>,
}

/// Reads what `recipe` names besides the open drawing (`drawing_path` its own file, refused when named).
pub fn read_sources(
    recipe: &ExtractionRecipe,
    base: Option<&str>,
    drawing_path: Option<&str>,
) -> Result<Sources, Unread> {
    let own = drawing_path.map(|p| same_place(&resolved(p, base)));
    let mut drawings = Vec::new();
    let mut read = Vec::new();
    for (i, d) in recipe.drawings.iter().enumerate() {
        let at = resolved(d, base);
        let place = same_place(&at);
        if recipe.current && own.as_ref() == Some(&place) {
            return Err(Unread {
                code: codes::DRAWING_READ_TWICE,
                words: format!(
                    "“{d}” açık çizimin kendi dosyası; açık çizim zaten okunuyor, her çizim bir kez sayılır."
                ),
                path: format!("recipe.drawings[{i}]"),
            });
        }
        if drawings
            .iter()
            .any(|(_, p, _): &(String, PathBuf, DocumentSnapshotV2)| *p == place)
        {
            return Err(Unread {
                code: codes::DRAWING_READ_TWICE,
                words: format!(
                    "“{d}” tarifte başka bir yolla zaten okunuyor; her çizim bir kez sayılır."
                ),
                path: format!("recipe.drawings[{i}]"),
            });
        }
        let Ok(bytes) = std::fs::read(&at) else {
            return Err(Unread {
                code: codes::EXTRACTION_SOURCE_MISSING,
                words: format!("“{d}” çizimi yolunda yok ya da okunamıyor; yolu denetleyin."),
                path: format!("recipe.drawings[{i}]"),
            });
        };
        let snapshot = crate::file_import_block::read_drawing(&bytes).map_err(|why| Unread {
            code: codes::EXTRACTION_SOURCE_UNREADABLE,
            words: format!("“{d}” bir KentOS çizimi olarak okunamadı: {why}"),
            path: format!("recipe.drawings[{i}]"),
        })?;
        read.push(ExtractRead {
            path: d.clone(),
            digest: crate::tables_edit::digest_of(&bytes),
        });
        drawings.push((d.clone(), place, snapshot));
    }
    let join = match &recipe.join {
        None => None,
        Some(j) => {
            let Ok(bytes) = std::fs::read(resolved(&j.path, base)) else {
                return Err(Unread {
                    code: codes::EXTRACTION_SOURCE_MISSING,
                    words: format!(
                        "“{}” dış tablosu yolunda yok ya da okunamıyor; yolu denetleyin.",
                        j.path
                    ),
                    path: "recipe.join.path".into(),
                });
            };
            let book = kentos_formats::table_file::read(&bytes);
            if let Some(p) = book.problem {
                return Err(Unread {
                    code: codes::EXTRACTION_SOURCE_UNREADABLE,
                    words: format!("“{}” dış tablosu okunamadı: {p}", j.path),
                    path: "recipe.join.path".into(),
                });
            }
            let sheet = match &j.sheet {
                None => book.sheets.into_iter().next(),
                Some(want) => book
                    .sheets
                    .into_iter()
                    .find(|s| s.name.as_deref() == Some(want)),
            };
            let Some(sheet) = sheet else {
                return Err(Unread {
                    code: codes::EXTRACTION_SOURCE_UNREADABLE,
                    words: format!(
                        "“{}” dış tablosunda “{}” sayfası yok.",
                        j.path,
                        j.sheet.clone().unwrap_or_default()
                    ),
                    path: "recipe.join.sheet".into(),
                });
            };
            read.push(ExtractRead {
                path: j.path.clone(),
                digest: crate::tables_edit::digest_of(&bytes),
            });
            Some(sheet.rows)
        }
    };
    Ok(Sources {
        drawings,
        join,
        read,
    })
}

/// The external references among `blocks` whose drawing is one the recipe reads itself (by its place or its
/// content's digest): never looked into.
fn read_apart(
    blocks: &[&kentos_commands::BlockDefinition],
    folder: Option<&Path>,
    others: &[(PathBuf, &str)],
) -> HashSet<kentos_commands::BlockId> {
    blocks
        .iter()
        .filter_map(|b| {
            let x = b.xref.as_ref()?;
            let mut places = vec![same_place(Path::new(&x.path))];
            if let (Some(r), Some(f)) = (&x.relative, folder) {
                places.push(same_place(&f.join(r)));
            }
            others
                .iter()
                .any(|(p, digest)| places.contains(p) || x.digest == *digest)
                .then_some(b.id)
        })
        .collect()
}

/// The recipe worked out over the open drawing and `sources`.
pub(crate) fn work_out(
    doc: &Document,
    recipe: &ExtractionRecipe,
    sources: &Sources,
    base: Option<&str>,
) -> Result<Extracted, Stop> {
    work_out_watched(doc, recipe, sources, base, &kentos_formats::extract::Watch::default())
}

/// [`work_out`] that says how far it is and stops when asked (docs/adr/1131 §3).
pub(crate) fn work_out_watched(
    doc: &Document,
    recipe: &ExtractionRecipe,
    sources: &Sources,
    base: Option<&str>,
    watch: &kentos_formats::extract::Watch,
) -> Result<Extracted, Stop> {
    let format = kentos_project::format::Format::of(doc.settings());
    let shown = |d: &kentos_commands::DimensionEntity, prefix: &str, unit: &str, value: f64| {
        format.dimension_value(
            prefix,
            unit,
            value,
            &d.look,
            d.text.as_deref().filter(|t| !t.is_empty()),
        )
    };
    let others: Vec<(PathBuf, &str)> = sources
        .drawings
        .iter()
        .zip(&sources.read)
        .map(|((_, place, _), r)| (place.clone(), r.digest.as_str()))
        .collect();
    let mut drawings: Vec<Drawing<'_>> = Vec::new();
    if recipe.current {
        let blocks: Vec<&kentos_commands::BlockDefinition> =
            doc.blocks().iter().map(|b| &**b).collect();
        let apart = read_apart(&blocks, base.map(Path::new), &others);
        drawings.push(Drawing {
            path: "",
            name: kentos_formats::extract::THIS_DRAWING.to_owned(),
            objects: doc
                .entities_with_uids()
                .map(|(u, e)| (u.to_string(), e))
                .collect(),
            blocks,
            arrays: doc.arrays().iter().map(|a| &**a).collect(),
            layers: doc.layers().nodes(),
            read_apart: apart,
        });
    }
    for (path, place, snapshot) in &sources.drawings {
        let blocks: Vec<&kentos_commands::BlockDefinition> = snapshot.blocks.iter().collect();
        let mine: Vec<(PathBuf, &str)> =
            others.iter().filter(|(p, _)| p != place).cloned().collect();
        let apart = read_apart(&blocks, place.parent(), &mine);
        drawings.push(Drawing {
            path,
            name: file_name(path),
            objects: snapshot
                .uids
                .iter()
                .map(|u| u.to_text())
                .zip(snapshot.entities.iter())
                .collect(),
            blocks,
            arrays: snapshot.arrays.iter().collect(),
            layers: &snapshot.layers,
            read_apart: apart,
        });
    }
    let s = doc.settings();
    let decimals = Decimals {
        length: s.length_decimals,
        area: s.area_decimals,
    };
    kentos_formats::extract::extract_watched(recipe, &drawings, sources.join.as_deref(), decimals, &shown, watch)
        .map_err(|(code, at, words)| fail(code, words, &format!("recipe.{at}")))
}

/// The project's decimals.
fn decimals_of(doc: &Document) -> Decimals {
    let s = doc.settings();
    Decimals {
        length: s.length_decimals,
        area: s.area_decimals,
    }
}

fn notes_of(e: &Extracted) -> ExtractNotes {
    let n = &e.notes;
    let count = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
    ExtractNotes {
        hidden: count(n.hidden),
        hidden_states: count(n.hidden_states),
        unloaded: n.unloaded.clone(),
        read_apart: n.read_apart.clone(),
        repeated: count(n.repeated),
        angles: count(n.angles),
        overrides: count(n.overrides),
        unmatched: n.unmatched.clone(),
        unused: n.unused.clone(),
        too_deep: count(n.too_deep),
    }
}

/// What the extraction says besides its rows.
fn warnings_of(e: &Extracted) -> Vec<CommandWarning> {
    let n = &e.notes;
    let mut out = Vec::new();
    if n.hidden > 0 {
        // The count says why: a dynamic block's visibility state, a hidden or frozen layer (docs/adr/1131 §2).
        let layers = n.hidden - n.hidden_states.min(n.hidden);
        let why = match (n.hidden_states, layers) {
            (0, _) => format!("{} nesne gizli ya da dondurulmuş katmanda olduğu için", n.hidden),
            (s, 0) => format!("{s} parça dinamik bloğun görünürlük durumu gizlediği için"),
            (s, l) => format!(
                "{s} parça dinamik bloğun görünürlük durumu gizlediği, {l} nesne gizli ya da dondurulmuş katmanda olduğu için"
            ),
        };
        out.push(warning(
            codes::EXTRACTION_HIDDEN,
            format!("{why} sayılmadı ve listelenmedi; saymak için Görünmeyen parçaları seçin."),
        ));
    }
    if !n.unloaded.is_empty() {
        out.push(warning(
            codes::XREF_UNLOADED,
            format!("{} dış referansının içeriği yüklü değil; içine bakılmadı. Referansı yeniden yükleyin.", n.unloaded.join(", ")),
        ));
    }
    if !n.read_apart.is_empty() {
        out.push(warning(
            codes::XREF_READ_APART,
            format!(
                "{} dış referansının çizimi tarifte ayrıca okunuyor; içeriği iki kez sayılmadı, kendi çizimi olarak bir kez sayıldı.",
                n.read_apart.join(", ")
            ),
        ));
    }
    if !n.unmatched.is_empty() {
        out.push(warning(
            codes::JOIN_UNMATCHED,
            format!(
                "Dış tabloda şu anahtarlar yok, eşlenen hücreleri boş kaldı: {}.",
                n.unmatched.join(", ")
            ),
        ));
    }
    if n.overrides > 0 {
        out.push(warning(
            codes::DIMENSION_OVERRIDES,
            format!(
                "{} ölçünün yazısı elle yazılmış: Yazılan sütunu metindir ve toplanmaz; miktar için Gerçek ölçü kullanılır.",
                n.overrides
            ),
        ));
    }
    if n.too_deep > 0 {
        out.push(warning(
            codes::EXTRACTION_TOO_DEEP,
            format!(
                "{} iç içe bloğa en derin düzeyi ({}) geçtiği için bakılmadı.",
                n.too_deep,
                kentos_commands::MAX_EXTRACT_DEPTH
            ),
        ));
    }
    out
}

fn ran(
    e: &Extracted,
    recipe: &ExtractionRecipe,
    sources: &Sources,
    doc: &Document,
    stale: Option<bool>,
) -> ExtractRan {
    let cells = e.cells(recipe, decimals_of(doc));
    ExtractRan {
        headings: e.headings.clone(),
        quantities: e.quantities.clone(),
        rows: e
            .rows
            .iter()
            .map(|r| ExtractRow {
                key: r.key.clone(),
                cells: r.cells.clone(),
                values: r.values.clone(),
                members: r.members.clone(),
            })
            .collect(),
        total: e.totals.as_ref().and_then(|_| cells.last().cloned()),
        totals: e.totals.clone(),
        occurrences: u32::try_from(e.occurrences).unwrap_or(u32::MAX),
        read: sources.read.clone(),
        notes: notes_of(e),
        stale,
        revision: doc.revision().to_string(),
    }
}

/// The extraction table at `uid`: its slot, persistent id and table.
fn extraction_at(doc: &Document, uid: &str, path: &str) -> Result<(Slot, Uuid, TableEntity), Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin."
            ),
            path,
        ));
    }
    let found = Uuid::parse_str(uid)
        .ok()
        .and_then(|u| Some((u, doc.slot_of(u)?)));
    let Some((u, slot)) = found else {
        return Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!("“{uid}” kimlikli nesne çizimde yok."),
            path,
        ));
    };
    match doc.get(slot) {
        Some(Entity::Table(t)) if matches!(t.source, Some(TableSource::Extraction(_))) => Ok((slot, u, t.clone())),
        Some(Entity::Table(_)) => Err(fail(
            codes::NOT_AN_EXTRACTION,
            "Tablonun veri çıkarımı tarifi yok; Veri çıkarımı ile yazılmış bir tablonun kimliğini verin.".into(),
            path,
        )),
        _ => Err(fail(codes::NOT_A_TABLE, "Nesne bir tablo değil; bir tablonun kimliğini verin.".into(), path)),
    }
}

fn unread(u: Unread) -> Stop {
    fail(u.code, u.words, &u.path)
}

pub mod run {
    use super::*;

    fn answer(
        doc: &Document,
        input: &ExtractRun,
    ) -> Result<(ExtractRan, Vec<CommandWarning>), Stop> {
        answer_watched(doc, input, &kentos_formats::extract::Watch::default())
    }

    fn answer_watched(
        doc: &Document,
        input: &ExtractRun,
        watch: &kentos_formats::extract::Watch,
    ) -> Result<(ExtractRan, Vec<CommandWarning>), Stop> {
        let (recipe, table) = match (&input.recipe, &input.table) {
            (Some(r), None) => (r.clone(), None),
            (None, Some(uid)) => {
                let (_, _, t) = extraction_at(doc, uid, "table")?;
                let Some(TableSource::Extraction(x)) = t.source.clone() else {
                    unreachable!()
                };
                (x.recipe.clone(), Some((t, x)))
            }
            _ => {
                return Err(fail(
                    codes::INVALID_INPUT,
                    "Ya bir tarif (recipe) ya da bir çıkarım tablosu (table) verin; ikisi birden ya da hiçbiri olmaz.".into(),
                    "recipe",
                ));
            }
        };
        if let Some((at, words)) = kentos_commands::extraction_recipe_problem(&recipe) {
            return Err(fail(
                codes::INVALID_EXTRACTION_RECIPE,
                words,
                &format!("recipe.{at}"),
            ));
        }
        checks::revision(doc, input.expected_revision.as_deref())?;
        let sources = read_sources(
            &recipe,
            input.base.as_deref(),
            input.drawing_path.as_deref(),
        )
        .map_err(unread)?;
        let e = work_out_watched(doc, &recipe, &sources, input.base.as_deref(), watch)?;
        let stale = table.map(|(_, x)| {
            let cells = e.cells(&recipe, decimals_of(doc));
            let keys: Vec<&str> = e.rows.iter().map(|r| r.key.as_str()).collect();
            cells != x.copy
                || keys != x.keys.iter().map(String::as_str).collect::<Vec<_>>()
                || sources.read != x.read
        });
        let warnings = warnings_of(&e);
        Ok((ran(&e, &recipe, &sources, doc, stale), warnings))
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &ExtractRun) -> CommandResult<()> {
        match answer(cx.doc, input) {
            Ok((_, warnings)) => CommandResult::Completed {
                output: (),
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &ExtractRun) -> CommandResult<ExtractRan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over a document: what the Veri çıkarımı window previews.
    pub fn plan_in(doc: &Document, input: &ExtractRun) -> CommandResult<ExtractRan> {
        match answer(doc, input) {
            Ok((output, warnings)) => CommandResult::Completed { output, warnings },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: ExtractRun) -> CommandResult<ExtractRan> {
        plan_in(cx.doc, &input)
    }

    /// [`plan_in`] that says how far it is and stops when asked (the window's preview of a large drawing on its own
    /// thread, docs/adr/1131 §3): a stopped one is [`CommandResult::Cancelled`].
    pub fn plan_watched(
        doc: &Document,
        input: &ExtractRun,
        watch: &kentos_formats::extract::Watch,
    ) -> CommandResult<ExtractRan> {
        match answer_watched(doc, input, watch) {
            Ok((output, warnings)) => CommandResult::Completed { output, warnings },
            Err(stop) => {
                let r: CommandResult<ExtractRan> = stop.into();
                match r {
                    CommandResult::Failed { error } if error.code == kentos_formats::extract::CANCELLED => {
                        CommandResult::Cancelled
                    }
                    other => other,
                }
            }
        }
    }
}

/// A table of `cells` (heading, rows, total) keeping `source`, at `at` on `layer`, its words `height` high:
/// quantities right aligned.
fn table_of(
    doc: &Document,
    input: &ExtractTable,
    cells: Vec<Vec<String>>,
    source: ExtractionSource,
) -> TableEntity {
    let height = input.height.unwrap_or_else(|| {
        doc.settings()
            .annotation_height(kentos_commands::AnnotationKind::Table)
    });
    let aligns: Vec<TableAlign> = source
        .recipe
        .columns
        .iter()
        .map(|c| {
            if c.value.is_quantity() {
                TableAlign::Right
            } else {
                TableAlign::Left
            }
        })
        .collect();
    let mut t = TableEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: input
                .layer_id
                .clone()
                .unwrap_or_else(|| doc.layers().active().to_owned()),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: input.at,
        rotation: 0.0,
        height,
        rows: Vec::new(),
        columns: Vec::new(),
        cells: Vec::new(),
        merges: Vec::new(),
        aligns: None,
        header: true,
        grid: None,
        frame: None,
        face: Default::default(),
        source: Some(TableSource::Extraction(Box::new(source))),
        styles: Vec::new(),
        formulas: Vec::new(),
        fields: Vec::new(),
        breaks: input.breaks,
        table_style: None,
    };
    doc.take_rows(&mut t, cells);
    t.aligns = Some(aligns);
    t
}

pub mod table {
    use super::*;

    fn check(
        doc: &Document,
        input: &ExtractTable,
    ) -> Result<(TableEntity, Extracted, Vec<CommandWarning>), Stop> {
        checks::point(input.at, "Tablonun köşesinin", "at")?;
        if let Some((at, words)) = kentos_commands::extraction_recipe_problem(&input.recipe) {
            return Err(fail(
                codes::INVALID_EXTRACTION_RECIPE,
                words,
                &format!("recipe.{at}"),
            ));
        }
        if let Some(h) = input.height {
            checks::finite(h, "Yazı yüksekliği", "Sonlu bir sayı verin.", "height")?;
            if h <= 0.0 || h > kentos_commands::MAX_TABLE_SIZE {
                return Err(fail(
                    codes::INVALID_TABLE,
                    format!("Tablonun yazı yüksekliği {h}; sıfırdan büyük olmalı."),
                    "height",
                ));
            }
        }
        checks::revision(doc, input.expected_revision.as_deref())?;
        let layer = input
            .layer_id
            .clone()
            .unwrap_or_else(|| doc.layers().active().to_owned());
        let mut warnings = checks::layer(doc, &layer)?;
        let sources = read_sources(
            &input.recipe,
            input.base.as_deref(),
            input.drawing_path.as_deref(),
        )
        .map_err(unread)?;
        let e = work_out(doc, &input.recipe, &sources, input.base.as_deref())?;
        if e.rows.is_empty() {
            return Err(fail(
                codes::EXTRACTION_EMPTY,
                "Tarif hiçbir nesne bulmadı: türleri, blokları, katmanları ya da sayım kapsamını denetleyin.".into(),
                "recipe",
            ));
        }
        let cells = e.cells(&input.recipe, decimals_of(doc));
        let source = ExtractionSource {
            recipe: input.recipe.clone(),
            read: sources.read.clone(),
            keys: e.rows.iter().map(|r| r.key.clone()).collect(),
            copy: cells.clone(),
            overrides: input.overrides.unwrap_or_default(),
        };
        let t = doc.table_worked_out(&table_of(doc, input, cells, source));
        if let Some((field, words)) = t.problem() {
            return Err(fail(codes::INVALID_TABLE, words, field));
        }
        warnings.extend(warnings_of(&e));
        Ok((t, e, warnings))
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &ExtractTable) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok((_, _, warnings)) => CommandResult::Completed {
                output: (),
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &ExtractTable,
    ) -> CommandResult<ExtractTablePlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over a document: the table the window places.
    pub fn plan_in(doc: &Document, input: &ExtractTable) -> CommandResult<ExtractTablePlan> {
        match check(doc, input) {
            Ok((t, e, warnings)) => CommandResult::Completed {
                output: ExtractTablePlan {
                    table: Entity::Table(t),
                    rows: u32::try_from(e.rows.len()).unwrap_or(u32::MAX),
                    occurrences: u32::try_from(e.occurrences).unwrap_or(u32::MAX),
                    notes: notes_of(&e),
                    revision: doc.revision().to_string(),
                },
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ExtractTable,
    ) -> CommandResult<ExtractTabled> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_EXTRACT_TABLE,
            kentos_commands::CAD_EXTRACT_TABLE_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: ExtractTable,
    ) -> CommandResult<ExtractTabled> {
        let (t, e, warnings) = match check(cx.doc, &input) {
            Ok(checked) => checked,
            Err(stop) => return stop.into(),
        };
        let slot = match cx.doc.add_many(vec![Entity::Table(t)], STEP) {
            Ok(slots) => slots.first().copied(),
            Err(full) => {
                return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
            }
        };
        let Some(slot) = slot else {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, "Yuva yok.".into(), None)).into();
        };
        CommandResult::Completed {
            output: ExtractTabled {
                uid: cx.doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
                rows: u32::try_from(e.rows.len()).unwrap_or(u32::MAX),
                occurrences: u32::try_from(e.occurrences).unwrap_or(u32::MAX),
                notes: notes_of(&e),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        }
    }
}

// ── Tabloyu güncelle ──────────────────────────────────────────────────────

/// What a refresh came to.
pub enum Refreshed {
    /// The table written again: how many hand edits were kept and written over.
    Changed {
        table: Box<TableEntity>,
        kept: u32,
        replaced: u32,
        warnings: Vec<CommandWarning>,
    },
    /// The sources give what the table holds.
    Unchanged { warnings: Vec<CommandWarning> },
    /// A source is not where its path says.
    Missing(CommandWarning),
    /// A source cannot be read, or the refresh would lose the user's words: the table is kept.
    Kept(CommandWarning),
}

/// Whether the cell at `r`, `c` is the table's own working out (a formula's or a field's), not a hand edit.
fn computed(t: &TableEntity, r: usize, c: usize) -> bool {
    t.formula_at(r, c).is_some() || t.field_at(r, c).is_some()
}

/// The table's rows in the new order: each old row's new place (none: gone), and the new rows' old ones.
struct Moves {
    old_to_new: Vec<Option<usize>>,
    new_to_old: Vec<Option<usize>>,
}

/// `t` (an extraction table whose source is `src`) written again from its sources as they are now.
pub fn refresh(
    doc: &Document,
    t: &TableEntity,
    src: &ExtractionSource,
    base: Option<&str>,
    drawing_path: Option<&str>,
) -> Refreshed {
    let recipe = &src.recipe;
    let sources = match read_sources(recipe, base, drawing_path) {
        Ok(s) => s,
        Err(u) if u.code == codes::EXTRACTION_SOURCE_MISSING => {
            return Refreshed::Missing(warning(
                codes::EXTRACTION_SOURCE_MISSING,
                format!("{} Tablo olduğu gibi kaldı.", u.words),
            ));
        }
        Err(u) => {
            return Refreshed::Kept(warning(
                codes::EXTRACTION_SOURCE_UNREADABLE,
                format!("{} Tablo olduğu gibi kaldı.", u.words),
            ));
        }
    };
    let e = match work_out(doc, recipe, &sources, base) {
        Ok(e) => e,
        Err(stop) => {
            let err = stop.into_error();
            let code = if err.code == codes::JOIN_UNUSABLE {
                codes::JOIN_UNUSABLE
            } else {
                codes::EXTRACTION_SOURCE_UNREADABLE
            };
            return Refreshed::Kept(warning(
                code,
                format!("{} Tablo olduğu gibi kaldı.", err.message),
            ));
        }
    };
    let warnings = warnings_of(&e);
    let fresh = e.cells(recipe, decimals_of(doc));
    let keys: Vec<String> = e.rows.iter().map(|r| r.key.clone()).collect();
    match rewritten(doc, t, src, &fresh, &keys) {
        Err(why) => Refreshed::Kept(warning(codes::REFRESH_KEPT, why)),
        Ok((mut next, kept, replaced)) => {
            if let Some(TableSource::Extraction(x)) = &mut next.source {
                x.read = sources.read.clone();
                x.keys = keys;
                x.copy = fresh;
            }
            let next = doc.table_worked_out(&next);
            if next == *t {
                Refreshed::Unchanged { warnings }
            } else if let Some((_, words)) = next.problem() {
                Refreshed::Kept(warning(
                    codes::REFRESH_KEPT,
                    format!("Yenilenen tablo kurallara uymuyor: {words} Tablo olduğu gibi kaldı."),
                ))
            } else {
                Refreshed::Changed {
                    table: Box::new(next),
                    kept,
                    replaced,
                    warnings,
                }
            }
        }
    }
}

/// `t` with `fresh` (heading, rows keyed `keys`, total) as its extracted cells: the cells changed by hand kept as the
/// rule says, the user's further columns, styles, merges, fields, formulas and row heights following the keys.
fn rewritten(
    doc: &Document,
    t: &TableEntity,
    src: &ExtractionSource,
    fresh: &[Vec<String>],
    keys: &[String],
) -> Result<(TableEntity, u32, u32), String> {
    let copy = &src.copy;
    let width = src.recipe.columns.len();
    let total = src.recipe.total;
    if t.cells.len() != copy.len() {
        return Err(
            "Tabloya satır eklenmiş ya da tablodan satır silinmiş; çıkarım satırları anahtarlarıyla eşlenemiyor. Eklenen satırları kaldırın ya da açıklamaları yeni bir sütuna yazın; tablo olduğu gibi kaldı."
                .into(),
        );
    }
    let m = t.columns.len().max(width);
    let keep = src.overrides == SourceOverrides::Keep;
    let old_keys: HashMap<&str, usize> = src
        .keys
        .iter()
        .enumerate()
        .map(|(i, k)| (k.as_str(), i + 1))
        .collect();
    let (n_old, n_new) = (copy.len(), fresh.len());
    let old_total = total.then(|| n_old - 1);
    let new_total = total.then(|| n_new - 1);
    let mut moves = Moves {
        old_to_new: vec![None; n_old],
        new_to_old: vec![None; n_new],
    };
    moves.old_to_new[0] = Some(0);
    moves.new_to_old[0] = Some(0);
    if let (Some(o), Some(n)) = (old_total, new_total) {
        moves.old_to_new[o] = Some(n);
        moves.new_to_old[n] = Some(o);
    }
    for (i, k) in keys.iter().enumerate() {
        if let Some(&old) = old_keys.get(k.as_str()) {
            moves.old_to_new[old] = Some(i + 1);
            moves.new_to_old[i + 1] = Some(old);
        }
    }
    let (mut kept, mut replaced) = (0u32, 0u32);
    // A row the sources no longer give: lost only by the rule to write over, and said so.
    for (old, to) in moves.old_to_new.iter().enumerate() {
        if to.is_some() {
            continue;
        }
        let own = &t.cells[old];
        let edited = (0..width)
            .filter(|&c| own[c] != copy[old][c] && !computed(t, old, c))
            .count();
        let users = (width..own.len()).filter(|&c| !own[c].is_empty()).count();
        if edited + users == 0 {
            continue;
        }
        let key = src.keys.get(old.wrapping_sub(1)).map_or("", String::as_str);
        let shown = key
            .split(kentos_formats::extract::KEY_SEPARATOR)
            .collect::<Vec<_>>()
            .join(" / ");
        if keep {
            return Err(format!(
                "“{shown}” satırı kaynaklardan kalktı ama elle girilmiş {} hücresi var; kaybolmamaları için tablo olduğu gibi kaldı. Değerleri başka yere alın ya da yenileme kuralını Dosyayla değiştir yapın.",
                edited + users
            ));
        }
        replaced += u32::try_from(edited + users).unwrap_or(u32::MAX);
    }
    let mut cells: Vec<Vec<String>> = Vec::with_capacity(n_new);
    for (r, row) in fresh.iter().enumerate() {
        let mut out = row.clone();
        out.resize(m, String::new());
        if let Some(old) = moves.new_to_old[r] {
            for c in 0..width {
                let (own, was, new) = (&t.cells[old][c], &copy[old][c], &row[c]);
                if own == was || computed(t, old, c) {
                    continue;
                }
                if new == was {
                    out[c] = own.clone();
                    kept += 1;
                } else if keep {
                    let label = if r == 0 {
                        "başlığın".to_owned()
                    } else {
                        format!("{}. satırın", r)
                    };
                    return Err(format!(
                        "{label} {}. hücresini hem kaynak hem kullanıcı değiştirdi (“{own}” → kaynakta “{new}”); düzeltme sessizce silinmedi, tablo olduğu gibi kaldı. İnceleyip elle çözün ya da yenileme kuralını Dosyayla değiştir yapın.",
                        c + 1
                    ));
                } else {
                    replaced += 1;
                }
            }
            for (c, cell) in out.iter_mut().enumerate().skip(width) {
                *cell = t.cells[old].get(c).cloned().unwrap_or_default();
            }
        }
        cells.push(out);
    }
    let mut next = t.clone();
    let heights = t.rows.clone();
    doc.take_rows(&mut next, cells);
    for (r, old) in moves.new_to_old.iter().enumerate() {
        if let (Some(old), Some(h)) = (old, next.rows.get_mut(r)) {
            *h = heights[*old];
        }
    }
    // Whole-data spans of formulas (Toplam's `=TOPLA(F2:F9)`) follow the data rows as they grow or shrink.
    let span = (n_old > 1 + usize::from(total) && n_new > 1 + usize::from(total)).then(|| {
        (
            (1, n_old - 1 - usize::from(total)),
            (1, n_new - 1 - usize::from(total)),
        )
    });
    let mut formulas = Vec::with_capacity(t.formulas.len());
    for f in &t.formulas {
        let row = moves
            .old_to_new
            .get(f.row as usize)
            .copied()
            .flatten()
            .ok_or_else(|| {
                "Formüllü bir satır kaynaklardan kalktı; tablo olduğu gibi kaldı.".to_owned()
            })?;
        let words = kentos_commands::table_formula::remap_rows_spanning(
            &f.formula,
            &moves.old_to_new,
            span,
        )
        .ok_or_else(|| {
            "Bir formülün başvurduğu satır kalktı ya da aralığı bölündü; tablo olduğu gibi kaldı."
                .to_owned()
        })?;
        let mut g = f.clone();
        g.row = row as u32;
        g.formula = words;
        formulas.push(g);
    }
    let mut fields = Vec::with_capacity(t.fields.len());
    for f in &t.fields {
        let row = moves
            .old_to_new
            .get(f.row as usize)
            .copied()
            .flatten()
            .ok_or_else(|| {
                "Alan (field) hücreli bir satır kaynaklardan kalktı; tablo olduğu gibi kaldı."
                    .to_owned()
            })?;
        let mut g = f.clone();
        g.row = row as u32;
        fields.push(g);
    }
    let identity = moves
        .old_to_new
        .iter()
        .enumerate()
        .all(|(i, to)| *to == Some(i))
        && n_old == n_new;
    let mut merges = Vec::with_capacity(t.merges.len());
    for range in &t.merges {
        if identity {
            merges.push(*range);
            continue;
        }
        if range.rows != 1 {
            return Err("Satır sırası değişti; birden çok satıra yayılan birleşim güvenle taşınamadığı için tablo olduğu gibi kaldı.".into());
        }
        let row = moves
            .old_to_new
            .get(range.row as usize)
            .copied()
            .flatten()
            .ok_or_else(|| {
                "Birleşik hücreli bir satır kaynaklardan kalktı; tablo olduğu gibi kaldı."
                    .to_owned()
            })?;
        merges.push(kentos_commands::CellRange {
            row: row as u32,
            ..*range
        });
    }
    let mut styles = Vec::with_capacity(t.styles.len());
    for s in &t.styles {
        let range = s.range;
        let (top, rows) = (range.row as usize, range.rows as usize);
        if identity || (top == 0 && rows == 1) {
            styles.push(s.clone());
            continue;
        }
        // A style over the whole table or every data row stays over them.
        if top == 0 && rows == n_old {
            let mut g = s.clone();
            g.range.rows = n_new as u32;
            styles.push(g);
            continue;
        }
        let data_old = n_old - 1 - usize::from(total);
        if top == 1 && rows == data_old {
            let mut g = s.clone();
            g.range.rows = (n_new - 1 - usize::from(total)) as u32;
            styles.push(g);
            continue;
        }
        let mut at: Vec<usize> = (top..top + rows)
            .filter_map(|o| moves.old_to_new.get(o).copied().flatten())
            .collect();
        at.sort_unstable();
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for r in at {
            match runs.last_mut() {
                Some((start, count)) if *start + *count == r => *count += 1,
                _ => runs.push((r, 1)),
            }
        }
        for (start, count) in runs {
            let mut g = s.clone();
            g.range.row = start as u32;
            g.range.rows = count as u32;
            styles.push(g);
        }
    }
    if styles.len() > kentos_commands::MAX_CELL_STYLES {
        return Err(
            "Yenilenen tablo çok fazla hücre stili gerektiriyor; tablo olduğu gibi kaldı.".into(),
        );
    }
    next.formulas = formulas;
    next.fields = fields;
    next.merges = merges;
    next.styles = styles;
    if let Some(a) = &t.aligns {
        next.aligns = Some(a.clone());
    }
    Ok((next, kept, replaced))
}
