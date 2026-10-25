//! `cad.gis.detach` v1 (docs/adr/0634 §2): a derived dataset's tables made the GIS's own data. The other way out of
//! a GIS edit than `cad.gis.takeback`: instead of writing the change back to the drawing, the table stops being a
//! derivative. Its derivation record (`https://kentos.app/derivation/1`) becomes a note of what it was, when and why
//! the link was broken (`https://kentos.app/derivation-detached/1`), and its rows leave the drawing's lasting ids
//! (`kentos_uids`), in one SQLite transaction. Afterwards the table is bound, read and imported as any table, its rows
//! new objects; the drawing does not change.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kentos_commands::{
    CommandResult, DERIVATION_STANDARD, DETACHED_FORMAT, DETACHED_STANDARD, DerivationDetached,
    DerivationRecord, DetachedTable, GisDetach, GisDetached,
};
use kentos_geopackage::GeoPackage;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::gis_derive;
use crate::gis_set::Container;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// A table to detach: its name, its record, the rows holding the drawing's ids.
struct Detaching {
    table: String,
    record: DerivationRecord,
    rows: usize,
}

/// The file's tables to detach, or the first refusal.
fn detaching(input: &GisDetach) -> Result<(String, Container, Vec<Detaching>, u32), Stop> {
    let path = input.path.as_str();
    crate::gis_derivation::file_or_database(path)?;
    let set =
        crate::gis_set::open_any(path).map_err(|(code, message)| refuse(code, message, "path"))?;
    let derived: Vec<(String, DerivationRecord, usize)> = set
        .tables
        .iter()
        .map(|t| (t.layer.name.clone(), t.record.clone(), t.uids.len()))
        .collect();
    let mut seen = BTreeSet::new();
    for (i, t) in input.tables.iter().enumerate() {
        if !seen.insert(t.as_str()) {
            return Err(refuse(
                codes::INVALID_COUNT,
                format!("“{t}” tablosu iki kez verildi."),
                &format!("tables.{i}"),
            ));
        }
        if !derived.iter().any(|(n, _, _)| n == t) {
            return Err(refuse(
                codes::DERIVATION_ROW_NOT_FOUND,
                format!("Dosyada “{t}” adlı türev tablo yok."),
                &format!("tables.{i}"),
            ));
        }
    }
    let total = derived.len();
    let mut out = Vec::new();
    for (table, record, rows) in derived {
        if !input.tables.is_empty() && !input.tables.contains(&table) {
            continue;
        }
        out.push(Detaching {
            table,
            record,
            rows,
        });
    }
    let linked = (total - out.len()) as u32;
    Ok((set.file, set.container, out, linked))
}

fn answer(name: String, tables: &[Detaching], linked: u32) -> GisDetached {
    GisDetached {
        file: name,
        tables: tables
            .iter()
            .map(|d| DetachedTable {
                table: d.table.clone(),
                source: d.record.source.name.clone(),
                rows: d.rows as u32,
            })
            .collect(),
        linked,
    }
}

/// Checks `input` against the file, writing nothing.
pub fn validate(_cx: &ExecutionContext<'_>, input: &GisDetach) -> CommandResult<()> {
    match detaching(input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would break; writes nothing.
pub fn plan(_cx: &ExecutionContext<'_>, input: &GisDetach) -> CommandResult<GisDetached> {
    match detaching(input) {
        Ok((name, _, t, linked)) => CommandResult::Completed {
            output: answer(name, &t, linked),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Breaks the tables' links in the file in one transaction; the drawing does not change.
pub fn execute(cx: &mut ExecutionContext<'_>, input: GisDetach) -> CommandResult<GisDetached> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_GIS_DETACH,
        kentos_commands::CAD_GIS_DETACH_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(_cx: &mut ExecutionContext<'_>, input: GisDetach) -> CommandResult<GisDetached> {
    let (name, container, t, linked) = match detaching(&input) {
        Ok(x) => x,
        Err(stop) => return stop.into(),
    };
    let unwritable = |why: String| -> CommandResult<GisDetached> {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("{} yazılamadı: {why}", input.path),
            "path",
        )
        .into()
    };
    let at = gis_derive::utc_now();
    let reason = input
        .reason
        .as_ref()
        .map(|r| r.trim().to_owned())
        .filter(|r| !r.is_empty());
    let note = |d: &Detaching| DerivationDetached {
        format: DETACHED_FORMAT.into(),
        version: 1,
        at: at.clone(),
        reason: reason.clone(),
        derivation: d.record.clone(),
    };
    let path = Path::new(&input.path);
    let written = match container {
        Container::GeoPackage => {
            let notes: Vec<(String, String)> = t
                .iter()
                .map(|d| {
                    (
                        d.table.clone(),
                        serde_json::to_string(&note(d)).unwrap_or_default(),
                    )
                })
                .collect();
            GeoPackage::open(path, true)
                .and_then(|mut f| {
                    f.replace_metadata(&notes, DERIVATION_STANDARD, DETACHED_STANDARD, &at)
                })
                .map(|_| ())
                .map_err(|e| e.0)
        }
        Container::Shapefile | Container::GeoJson => {
            let notes: BTreeMap<String, serde_json::Value> = t
                .iter()
                .map(|d| {
                    (
                        d.table.clone(),
                        serde_json::to_value(note(d)).unwrap_or_default(),
                    )
                })
                .collect();
            crate::gis_set::detach_file(path, &notes)
        }
        Container::PostGis => {
            let notes: Vec<(String, String)> = t
                .iter()
                .map(|d| {
                    (
                        d.table.clone(),
                        serde_json::to_string(&note(d)).unwrap_or_default(),
                    )
                })
                .collect();
            match crate::provider::postgis() {
                Some(db) => db
                    .detach(&input.path, &notes)
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}")),
                None => Err("bu ortamda PostGIS sağlayıcısı yok".into()),
            }
        }
    };
    if let Err(why) = written {
        return unwritable(why);
    }
    CommandResult::Completed {
        output: answer(name, &t, linked),
        warnings: Vec::new(),
    }
}
