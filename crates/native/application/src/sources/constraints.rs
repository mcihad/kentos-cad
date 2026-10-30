//! A bound table's CHECK and unique constraints (docs/adr/0755): read with the table into its link, so a form and a
//! command ask them before the table does, and the table's refusal of a write said by the constraint it names. The
//! independent reference is `scripts/fixtures/source_constraints_cases.py`.

use std::path::Path;

use kentos_commands::{CommandWarning, LayerField, TableCheck, TableUnique};
use kentos_forms::table_checks::{self, Dialect, TableChecks};
use kentos_geopackage::{self as gpkg, GeoPackage};

use crate::checks::Stop;
use crate::codes;
use crate::provider::{self, ConstraintKind, ProviderError, TableConstraints};

use super::refuse;

/// A GeoPackage table's constraints: each CHECK with the table's columns it reads, left to the table when one of them
/// has a collation other than byte order or SQLite's affinity other than its field kind's; its unique constraints and
/// plain unique indexes.
pub(crate) fn of_file(
    path: &Path,
    layer: &gpkg::Layer,
    fields: &[LayerField],
) -> Result<TableConstraints, String> {
    let file = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let found = file.constraints(&layer.name).map_err(|e| e.0)?;
    let mut names: Vec<String> = vec![layer.fid.clone()];
    names.extend(layer.geometry.as_ref().map(|g| g.name.clone()));
    names.extend(layer.columns.iter().map(|c| c.name.clone()));
    let mut out = TableConstraints::default();
    for c in found.checks {
        if c.sql.chars().count() > kentos_commands::MAX_CHECK_SQL
            || out.checks.len() >= kentos_commands::MAX_TABLE_CONSTRAINTS
        {
            continue;
        }
        let columns = table_checks::columns_of(&c.sql, Dialect::Sqlite, &names);
        let table_only = columns.iter().any(|n| {
            found.collated.contains(n)
                || layer
                    .columns
                    .iter()
                    .find(|col| col.name == *n)
                    .is_some_and(|col| {
                        fields
                            .iter()
                            .find(|f| f.name == *n)
                            .is_some_and(|f| !table_checks::sqlite_fits(&col.decl, f.kind))
                    })
        });
        out.checks.push(TableCheck {
            name: c.name,
            sql: c.sql,
            columns,
            table_only,
        });
    }
    for u in found.uniques {
        if out.uniques.len() < kentos_commands::MAX_TABLE_CONSTRAINTS {
            out.uniques.push(TableUnique {
                name: u.name,
                columns: u.columns,
                nulls_equal: false,
            });
        }
    }
    Ok(out)
}

/// The warning of constraints not read (docs/adr/0755 §1).
pub(crate) fn unread(table: &str, why: &str, at: &str) -> CommandWarning {
    CommandWarning {
        code: codes::CONSTRAINTS_UNREAD.into(),
        message: format!(
            "“{table}” tablosunun CHECK ve tekil kısıtları okunamadı ({why}); KentOS onları denetlemez, tablo yine denetler."
        ),
        path: Some(at.to_owned()),
    }
}

/// A database table's, by the host's provider; none where the host registered none.
pub(crate) fn of_database(address: &str, table: &str) -> Result<TableConstraints, String> {
    let Some(p) = provider::postgis() else {
        return Ok(TableConstraints::default());
    };
    p.constraints(address, table).map_err(|e| match e {
        ProviderError::Unreachable(w)
        | ProviderError::NoTable(w)
        | ProviderError::Refused(w)
        | ProviderError::Unreadable(w) => w,
        other => format!("{other:?}"),
    })
}

/// The link's constraint the table named (its name, or SQLite's text of one without a name): its own name (none for
/// one without) and its sentence (else its text).
fn named_check(
    checks: &[TableCheck],
    fields: &[LayerField],
    d: Dialect,
    named: &str,
) -> Option<(Option<String>, String)> {
    let c = checks.iter().find(|c| c.said() == named)?;
    Some((
        c.name.clone(),
        TableChecks::new(std::slice::from_ref(c), fields, d).said(0),
    ))
}

/// The table's refusal of a write by one of its constraints (docs/adr/0755 §5), by its code: the constraint's name,
/// the column the database named (a unique constraint's columns for SQLite, which names it by them) and its words;
/// `table` when known, and the bound layer's constraints and fields, whose sentence a CHECK's refusal says. Nothing
/// was written.
pub(crate) fn refusal(
    kind: ConstraintKind,
    name: Option<&str>,
    column: Option<&str>,
    detail: &str,
    table: Option<&str>,
    known: Option<(&[TableCheck], &[LayerField], Dialect)>,
    at: &str,
) -> Stop {
    let whose = table.map_or_else(|| "Tablonun".to_owned(), |t| format!("“{t}” tablosunun"));
    let named = name.map(|n| format!(" “{n}”")).unwrap_or_default();
    let told = |s: &str| {
        if s.trim().is_empty() {
            String::new()
        } else {
            format!(" Veritabanı: {}", s.trim())
        }
    };
    match kind {
        ConstraintKind::Check => {
            let found = name
                .zip(known)
                .and_then(|(n, (checks, fields, d))| named_check(checks, fields, d, n));
            let (own, said) = match found {
                Some((own, said)) => (own, Some(said)),
                None => (name.map(str::to_owned), None),
            };
            let named = own.map(|n| format!(" “{n}”")).unwrap_or_default();
            refuse(
                codes::SOURCE_CHECK,
                match said {
                    Some(s) => format!(
                        "{whose}{named} kısıtı yazmayı reddetti: {s}. Hiçbir şey yazılmadı; değeri kısıta göre düzeltip yeniden yazın."
                    ),
                    None => format!(
                        "{whose}{named} kısıtı yazmayı reddetti; hiçbir şey yazılmadı.{}",
                        told(detail)
                    ),
                },
                at,
            )
        }
        ConstraintKind::Unique => {
            let which = column.map_or(named, |c| format!(" ({c})"));
            refuse(
                codes::SOURCE_UNIQUE,
                format!(
                    "{whose} tekil kısıtı{which} yazmayı reddetti: aynı değerler başka bir satırda var; hiçbir şey yazılmadı.{}",
                    told(detail)
                ),
                at,
            )
        }
        ConstraintKind::Foreign => refuse(
            codes::SOURCE_REFERENCE,
            format!(
                "{whose} yabancı anahtarı{named} yazmayı reddetti: başvurulan satır yok ya da silinen satıra başka satırlar başvuruyor; hiçbir şey yazılmadı.{}",
                told(detail)
            ),
            at,
        ),
        ConstraintKind::NotNull => refuse(
            codes::SOURCE_NOT_NULL,
            match column {
                Some(c) => format!(
                    "{whose} “{c}” sütunu boş bırakılamaz (NOT NULL); hiçbir şey yazılmadı. Değeri verip yeniden yazın."
                ),
                None => format!(
                    "{whose} boş bırakılamayan (NOT NULL) bir sütunu boş kaldı; hiçbir şey yazılmadı.{}",
                    told(detail)
                ),
            },
            at,
        ),
    }
}

/// SQLite's refusal of a write as one of the table's constraints', if it is one (a unique constraint said by its
/// columns, as SQLite names it).
pub(crate) fn file_refusal(
    e: &gpkg::Error,
    table: &str,
    known: Option<(&[TableCheck], &[LayerField])>,
    at: &str,
) -> Option<Stop> {
    let (kind, name, column) = match e.violation()? {
        gpkg::constraints::Violation::Check(n) => (ConstraintKind::Check, Some(n), None),
        gpkg::constraints::Violation::Unique(cols) => {
            (ConstraintKind::Unique, None, Some(cols.join(", ")))
        }
        gpkg::constraints::Violation::NotNull(c) => (ConstraintKind::NotNull, None, Some(c)),
        gpkg::constraints::Violation::Foreign => (ConstraintKind::Foreign, None, None),
    };
    Some(refusal(
        kind,
        name.as_deref(),
        column.as_deref(),
        "",
        Some(table),
        known.map(|(c, f)| (c, f, Dialect::Sqlite)),
        at,
    ))
}

/// The link's constraint SQLite named, among `checks` (its name, or the text of one without a name).
pub(crate) fn names_check(e: &gpkg::Error, checks: &[TableCheck]) -> bool {
    matches!(e.violation(), Some(gpkg::constraints::Violation::Check(n)) if checks.iter().any(|c| c.said() == n))
}
