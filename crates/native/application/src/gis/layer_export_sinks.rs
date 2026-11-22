//! Where `cad.layers.export` writes (docs/adr/9106 §3): the rows made from the layer's objects (layer_export.rs) handed
//! to a target as they come.
//!
//! - A GeoPackage or PostGIS table through its driver, in one transaction: made (or made again, `replaceLayer`), or
//!   added to (`append`, a field the table lacks left out); a GeoPackage file made when it is not there or written anew
//!   (`overwriteFile`). After the commit a GeoPackage's table gets the layer's dataset information as its metadata.
//! - A CSV file written as the rows come: the fields, then the geometry as WKT (or X and Y), in UTF-8 or Windows-1254.
//! - GeoJSON, Shapefile and KML files through the formats core's streams, the rows made objects again (in the system
//!   written) and written as they come (layer_export_streams.rs, docs/adr/9112 §1).
//!
//! Nothing is kept of a target when the export stops or fails: the transaction is rolled back, and a file it makes is
//! written beside its place (`<file>.part`) and put there only at the end, so a file there before stays as it was.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kentos_commands::{ExportEncoding, ExportExisting, ExportFormat, LayerField};
use kentos_data::{NewFeature, NewTable, Source, Transaction, Value};
use kentos_domain::Document;

use crate::checks::Stop;
use crate::codes;
use crate::layer_export::{Plan, fail, format_words, table_name_of, unreachable};

/// A target of the export.
pub(crate) trait Sink {
    /// Writes rows (each in the system written, its values the fields' kinds); `objects` are the layer's objects they
    /// were made from, in their order (a KML file's styles are their classes', docs/adr/9112 §4).
    fn write(&mut self, rows: &[NewFeature], objects: &[&kentos_commands::Entity]) -> Result<(), String>;
    /// The fields an `append` left out (the table lacks them).
    fn left_out(&self) -> Vec<String> {
        Vec::new()
    }
    /// Ends the write: the files written.
    fn finish(self: Box<Self>) -> Result<Vec<String>, String>;
    /// Ends it writing nothing: what was begun is let go.
    fn abandon(self: Box<Self>);
}

/// A plan's target: nothing is written.
struct Nothing;

impl Sink for Nothing {
    fn write(&mut self, _rows: &[NewFeature], _objects: &[&kentos_commands::Entity]) -> Result<(), String> {
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    fn abandon(self: Box<Self>) {}
}

/// The table written: a GeoPackage's table name, PostGIS's `schema.table`; none for a file.
pub(crate) fn table_of(plan: &Plan<'_>) -> Option<String> {
    let name = plan.input.name.clone().unwrap_or_else(|| table_name_of(&plan.node.name));
    match plan.input.format {
        ExportFormat::Gpkg => Some(name),
        ExportFormat::Postgis => Some(format!(
            "{}.{}",
            plan.input.schema.clone().unwrap_or_else(|| "public".to_owned()),
            name
        )),
        _ => None,
    }
}

/// The target for the plan; a plan's writes nothing. Refused when it is there and not to be written over.
pub(crate) fn sink_for(doc: &Document, plan: &Plan<'_>, write: bool) -> Result<Box<dyn Sink>, Stop> {
    let existing = plan.input.existing.unwrap_or_default();
    let path = plan.input.path.as_deref().map(|p| PathBuf::from(p.trim()));
    // A file format's file there already.
    if let (Some(p), false) = (&path, matches!(plan.input.format, ExportFormat::Postgis | ExportFormat::Gpkg))
        && p.exists()
        && existing == ExportExisting::Refuse
    {
        return Err(exists(&format!("“{}” dosyası var.", p.display())));
    }
    match plan.input.format {
        ExportFormat::Gpkg | ExportFormat::Postgis => Table::open(doc, plan, path.as_deref(), existing, write),
        _ if !write => Ok(Box::new(Nothing)),
        ExportFormat::Csv => Ok(Box::new(Csv::open(plan, path.as_deref().unwrap_or(Path::new("")))?)),
        _ => crate::layer_export_streams::open(plan, path.unwrap_or_default()),
    }
}

/// Where a file is written while the export runs: beside its place, put there at the end (a file there stays as it
/// was when the export stops or fails).
pub(crate) fn part_of(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

/// A GeoPackage part's address: named as a GeoPackage (`gpkg:`), its file not ending in `.gpkg`.
fn part_address(part: &Path) -> String {
    format!("gpkg:{}", part.display())
}

/// Puts a finished part in its place, over a file there; the part is removed when it cannot be.
pub(crate) fn put_in_place(part: &Path, path: &Path) -> Result<(), String> {
    std::fs::rename(part, path).map_err(|e| {
        let _ = std::fs::remove_file(part);
        format!("“{}” yerine konamadı: {e}", path.display())
    })
}

fn exists(what: &str) -> Stop {
    fail(
        codes::TARGET_EXISTS,
        format!("{what} Üstüne yazmayı, katmanı değiştirmeyi ya da eklemeyi seçin (existing)."),
        "existing",
    )
}

// ── GeoPackage and PostGIS tables ──────────────────────────────────────────────────────────────────────────────

struct Table {
    tx: Option<Box<dyn Transaction>>,
    name: String,
    /// The target's fields an `append` writes (none: every field).
    kept: Option<Vec<String>>,
    left_out: Vec<String>,
    /// A GeoPackage's file, and its part while it is made (a file written into has none).
    file: Option<(PathBuf, Option<PathBuf>)>,
    /// The layer's dataset information, written as a GeoPackage table's metadata after the commit.
    metadata: Option<String>,
}

impl Table {
    fn open(
        _doc: &Document,
        plan: &Plan<'_>,
        path: Option<&Path>,
        existing: ExportExisting,
        write: bool,
    ) -> Result<Box<dyn Sink>, Stop> {
        let name = table_of(plan).unwrap_or_default();
        let (address, file) = match plan.input.format {
            ExportFormat::Gpkg => {
                let p = path.unwrap_or(Path::new("")).to_path_buf();
                let address = p.display().to_string();
                let there = p.exists();
                if !write {
                    // A plan: a file not there has no table; one there is asked whether it has the table.
                    if !there {
                        return Ok(Box::new(Nothing));
                    }
                }
                let made = !there || (write && existing == ExportExisting::OverwriteFile);
                if write && made {
                    // Made as its part (a part a crash left is made anew), put in its place at the end.
                    if let Some(folder) = p.parent().filter(|f| !f.as_os_str().is_empty()) {
                        std::fs::create_dir_all(folder).map_err(unreachable)?;
                    }
                    let part = part_of(&p);
                    kentos_data::registry().forget(&part_address(&part));
                    let _ = std::fs::remove_file(&part);
                    kentos_geopackage::GeoPackage::create(&part).map_err(|e| unreachable(e.0))?;
                    (part_address(&part), Some((p, Some(part))))
                } else {
                    (address, Some((p, None)))
                }
            }
            _ => (plan.input.address.clone().unwrap_or_default().trim().to_owned(), None),
        };
        let source: Arc<dyn Source> = kentos_data::registry().source(&address).map_err(unreachable)?;
        let tables = source.tables().map_err(unreachable)?;
        let there = tables.iter().any(|t| t.name == name);
        let overwrite_file = existing == ExportExisting::OverwriteFile && plan.input.format == ExportFormat::Gpkg;
        if there && !overwrite_file {
            match existing {
                ExportExisting::Refuse => {
                    return Err(exists(&format!("“{name}” tablosu {} kaynağında var.", format_words(plan.input.format))));
                }
                ExportExisting::Append | ExportExisting::ReplaceLayer | ExportExisting::OverwriteFile => {}
            }
        }
        let mut kept = None;
        let mut left_out = Vec::new();
        if there && existing == ExportExisting::Append {
            let schema = source.schema(&name).map_err(unreachable)?;
            let names: Vec<String> = plan.schema.fields.iter().map(|f| f.name.clone()).collect();
            left_out = names.iter().filter(|n| schema.index(n).is_none()).cloned().collect();
            kept = Some(names.into_iter().filter(|n| schema.index(n).is_some()).collect());
        }
        if !write {
            return Ok(Box::new(Nothing));
        }
        let mut tx = source.begin().map_err(unreachable)?;
        if !(there && existing == ExportExisting::Append) {
            let table = NewTable {
                name: name.clone(),
                // Named as it is written (its identifier is the table's name, as the layer added is).
                title: Some(name.clone()),
                description: plan.input.options.as_ref().and_then(|o| o.description.clone()),
                key: plan.schema.key.clone(),
                fields: plan.schema.fields.clone(),
                geometry: plan.schema.geometry.clone(),
                spatial_index: plan.input.options.as_ref().and_then(|o| o.no_spatial_index) != Some(true),
                system: system_text(plan.schema.geometry.as_ref().and_then(|g| g.srid)),
            };
            tx.create_table(&table, there).map_err(unreachable)?;
        }
        let metadata = (plan.input.metadata == Some(true))
            .then(|| plan.node.dataset.as_ref().and_then(|d| serde_json::to_string(d).ok()))
            .flatten();
        Ok(Box::new(Table {
            tx: Some(tx),
            name,
            kept,
            left_out,
            file,
            metadata,
        }))
    }
}

impl Sink for Table {
    fn write(&mut self, rows: &[NewFeature], _objects: &[&kentos_commands::Entity]) -> Result<(), String> {
        let Some(tx) = self.tx.as_mut() else {
            return Ok(());
        };
        let rows: Vec<NewFeature> = match &self.kept {
            Some(kept) => rows
                .iter()
                .map(|r| NewFeature {
                    values: r.values.iter().filter(|(n, _)| kept.contains(n)).cloned().collect(),
                    ..r.clone()
                })
                .collect(),
            None => rows.to_vec(),
        };
        tx.insert(&self.name, &rows).map(|_| ()).map_err(|e| e.to_string())
    }

    fn left_out(&self) -> Vec<String> {
        self.left_out.clone()
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<String>, String> {
        if let Some(tx) = self.tx.take() {
            tx.commit().map_err(|e| e.to_string())?;
        }
        let Some((path, part)) = &self.file else {
            return Ok(Vec::new());
        };
        if let Some(part) = part {
            // Its connections closed before it is moved, and the file's own opened anew when it is next read.
            kentos_data::registry().forget(&part_address(part));
            kentos_data::registry().forget(&path.display().to_string());
            put_in_place(part, path)?;
        }
        if let Some(text) = &self.metadata {
            let gp = kentos_geopackage::GeoPackage::open(path, true).map_err(|e| e.0)?;
            let (date, time) = crate::exchange_files::now();
            let now = format!("{date}T{time}.000Z");
            gp.add_metadata(&self.name, crate::gpkg_profile::DATASET_STANDARD, "application/json", text, &now)
                .map_err(|e| e.0)?;
        }
        Ok(vec![path.display().to_string()])
    }

    fn abandon(mut self: Box<Self>) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.rollback();
        }
        if let Some((_, Some(part))) = &self.file {
            kentos_data::registry().forget(&part_address(part));
            let _ = std::fs::remove_file(part);
        }
    }
}

/// The system's name and OGC WKT 1 written with a GeoPackage's table (as geopackage.rs's `srs_of` writes the project's):
/// other programs read the system from them. None for a code the catalogue lacks: named by its code then.
pub(crate) fn system_text(srid: Option<i32>) -> Option<kentos_data::SystemText> {
    let s = kentos_project::crs::system(u32::try_from(srid?).ok()?)?;
    let core = s.transform_system()?;
    let wkt = kentos_geometry_core::crs::text::write_wkt(&s.name, &core, None)?;
    Some(kentos_data::SystemText {
        name: s.name.clone(),
        wkt,
    })
}

/// The data layer the written table makes: where it is, its fields, its name; none for a file.
pub(crate) fn data_layer_of(plan: &Plan<'_>) -> Option<(kentos_commands::DataLayer, Vec<LayerField>, String)> {
    let table = table_of(plan)?;
    let (driver, address) = match plan.input.format {
        ExportFormat::Gpkg => ("gpkg", plan.input.path.clone()?.trim().to_owned()),
        ExportFormat::Postgis => ("postgis", plan.input.address.clone()?.trim().to_owned()),
        _ => return None,
    };
    let geometry = plan.schema.geometry.as_ref().map(|g| kentos_commands::DataGeometry {
        column: g.name.clone(),
        type_name: g.type_name.clone(),
        srid: g.srid.and_then(|s| u32::try_from(s).ok()),
        z: g.z,
        m: g.m,
    });
    let data = kentos_commands::DataLayer {
        driver: driver.to_owned(),
        address,
        relative: None,
        table,
        key: plan.schema.key.clone(),
        geometry,
        extent: None,
        read_only: false,
        text: None,
    };
    let fields = plan.fields.iter().map(|f| f.written()).collect();
    // The layer added is named as the table written (its bare name, without the schema).
    let name = plan.input.name.clone().unwrap_or_else(|| table_name_of(&plan.node.name));
    Some((data, fields, name))
}

// ── CSV ────────────────────────────────────────────────────────────────────────────────────────────────────────

struct Csv {
    out: Option<std::io::BufWriter<std::fs::File>>,
    path: PathBuf,
    /// Where it is written until it is put in its place.
    part: PathBuf,
    separator: char,
    xy: bool,
    geometry: bool,
    encoding: ExportEncoding,
    fields: Vec<String>,
}

impl Csv {
    fn open(plan: &Plan<'_>, path: &Path) -> Result<Csv, Stop> {
        if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder).map_err(unreachable)?;
        }
        let part = part_of(path);
        let file = std::fs::File::create(&part).map_err(unreachable)?;
        let o = plan.input.options.clone().unwrap_or_default();
        let mut csv = Csv {
            out: Some(std::io::BufWriter::new(file)),
            path: path.to_path_buf(),
            part,
            separator: o.separator.and_then(|s| s.chars().next()).unwrap_or(','),
            xy: o.xy == Some(true),
            geometry: plan.schema.geometry.is_some(),
            encoding: plan.input.encoding.unwrap_or_default(),
            fields: plan.schema.fields.iter().map(|f| f.name.clone()).collect(),
        };
        let mut head: Vec<String> = csv.fields.clone();
        if csv.geometry {
            if csv.xy {
                head.extend(["x".to_owned(), "y".to_owned()]);
            } else {
                head.push("wkt".to_owned());
            }
        }
        let line = csv.line(&head);
        csv.put(&line).map_err(unreachable)?;
        Ok(csv)
    }

    /// A row of cells: quoted when it holds the separator, a quote or a line break.
    fn line(&self, cells: &[String]) -> String {
        let quoted = |c: &str| {
            if c.contains(self.separator) || c.contains('"') || c.contains('\n') || c.contains('\r') {
                format!("\"{}\"", c.replace('"', "\"\""))
            } else {
                c.to_owned()
            }
        };
        let mut out = cells.iter().map(|c| quoted(c)).collect::<Vec<_>>().join(&self.separator.to_string());
        out.push_str("\r\n");
        out
    }

    fn put(&mut self, text: &str) -> std::io::Result<()> {
        let Some(out) = self.out.as_mut() else {
            return Ok(());
        };
        match self.encoding {
            ExportEncoding::Utf8 => out.write_all(text.as_bytes()),
            ExportEncoding::Windows1254 => out.write_all(&windows_1254(text)),
        }
    }
}

/// A text in Windows-1254; a character the code page lacks is `?`.
fn windows_1254(text: &str) -> Vec<u8> {
    text.chars().map(|c| kentos_formats::text::windows1254_byte(c).unwrap_or(b'?')).collect()
}

/// A value as a CSV cell.
pub(crate) fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        other => crate::geopackage::attribute_text(other).unwrap_or_default(),
    }
}

impl Sink for Csv {
    fn write(&mut self, rows: &[NewFeature], _objects: &[&kentos_commands::Entity]) -> Result<(), String> {
        for r in rows {
            let mut cells: Vec<String> = self
                .fields
                .iter()
                .map(|f| r.values.iter().find(|(n, _)| n == f).map(|(_, v)| cell(v)).unwrap_or_default())
                .collect();
            if self.geometry {
                match (&r.geometry, self.xy) {
                    (Some(kentos_data::Geometry::Point(Some(p))), true) => {
                        cells.extend([format!("{}", p.x), format!("{}", p.y)]);
                    }
                    (_, true) => cells.extend([String::new(), String::new()]),
                    (Some(g), false) => cells.push(kentos_data::wkt::write_wkt(g)),
                    (None, false) => cells.push(String::new()),
                }
            }
            let line = self.line(&cells);
            self.put(&line).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<String>, String> {
        if let Some(mut out) = self.out.take() {
            out.flush().map_err(|e| e.to_string())?;
        }
        put_in_place(&self.part, &self.path)?;
        Ok(vec![self.path.display().to_string()])
    }

    fn abandon(mut self: Box<Self>) {
        self.out = None;
        let _ = std::fs::remove_file(&self.part);
    }
}
