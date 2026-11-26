//! The GeoPackage schema extension (`gpkg_schema`, docs/adr/1075): a column's shown name and description in
//! `gpkg_data_columns`, and the rule it points to in `gpkg_data_column_constraints` — a range (least and most, each in
//! or out) or a list of values with their labels. Other programs read these as a field's alternative name and its
//! domain; KentOS writes them from a layer's fields and reads them back.

use rusqlite::{OptionalExtension, params};

use crate::{GeoPackage, Result};

const EXTENSION: &str = "gpkg_schema";
const DEFINITION: &str = "http://www.geopackage.org/spec121/#extension_schema";
const COLUMNS: &str = "gpkg_data_columns";
const CONSTRAINTS: &str = "gpkg_data_column_constraints";

/// A row of `gpkg_data_columns`: a table's column, its shown name, description and the rule it points to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataColumn {
    pub table: String,
    pub column: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub constraint: Option<String>,
}

/// A rule of `gpkg_data_column_constraints`.
#[derive(Clone, Debug, PartialEq)]
pub enum Rule {
    /// A number's range: the least and the most, each included or not; either may be open.
    Range {
        min: Option<f64>,
        min_included: bool,
        max: Option<f64>,
        max_included: bool,
    },
    /// The values a column takes, each with its label.
    Values(Vec<(String, Option<String>)>),
}

/// The extension's tables and their registration, made in `conn` when the file lacks them (inside the caller's
/// transaction).
pub fn ensure_schema_in(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {COLUMNS} (
           table_name TEXT NOT NULL, column_name TEXT NOT NULL, name TEXT, title TEXT, description TEXT,
           mime_type TEXT, constraint_name TEXT,
           CONSTRAINT pk_gdc PRIMARY KEY (table_name, column_name),
           CONSTRAINT gdc_tn UNIQUE (table_name, name));
         CREATE TABLE IF NOT EXISTS {CONSTRAINTS} (
           constraint_name TEXT NOT NULL, constraint_type TEXT NOT NULL, value TEXT, min NUMERIC,
           min_is_inclusive BOOLEAN, max NUMERIC, max_is_inclusive BOOLEAN, description TEXT,
           CONSTRAINT gdcc_ntv UNIQUE (constraint_name, constraint_type, value));"
    ))?;
    super::ensure_extensions(conn)?;
    for table in [COLUMNS, CONSTRAINTS] {
        conn.execute(
            "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, NULL, ?2, ?3, 'read-write')",
            params![table, EXTENSION, DEFINITION],
        )?;
    }
    Ok(())
}

/// The rows of `rule` under `name`, added (an earlier row of the same name, type and value is kept).
fn insert_rule(conn: &rusqlite::Connection, name: &str, rule: &Rule) -> rusqlite::Result<()> {
    match rule {
        Rule::Range { min, min_included, max, max_included } => {
            conn.execute(
                &format!(
                    "INSERT OR IGNORE INTO {CONSTRAINTS} (constraint_name, constraint_type, value, min,
                       min_is_inclusive, max, max_is_inclusive) VALUES (?1, 'range', NULL, ?2, ?3, ?4, ?5)"
                ),
                params![name, min, min.map(|_| *min_included), max, max.map(|_| *max_included)],
            )?;
        }
        Rule::Values(values) => {
            for (value, label) in values {
                conn.execute(
                    &format!(
                        "INSERT OR IGNORE INTO {CONSTRAINTS} (constraint_name, constraint_type, value,
                           description) VALUES (?1, 'enum', ?2, ?3)"
                    ),
                    params![name, value, label],
                )?;
            }
        }
    }
    Ok(())
}

/// `column` of `table` given `rule` under the name `name`, or its rule taken away (none), inside the caller's
/// transaction (docs/adr/9324): the rule's earlier rows go first, so a list written again holds what it holds now.
pub fn replace_rule_in(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
    name: &str,
    rule: Option<&Rule>,
) -> rusqlite::Result<()> {
    ensure_schema_in(conn)?;
    conn.execute(&format!("DELETE FROM {CONSTRAINTS} WHERE constraint_name = ?1"), [name])?;
    match rule {
        Some(rule) => {
            insert_rule(conn, name, rule)?;
            conn.execute(
                &format!(
                    "INSERT INTO {COLUMNS} (table_name, column_name, constraint_name) VALUES (?1, ?2, ?3)
                     ON CONFLICT (table_name, column_name) DO UPDATE SET constraint_name = excluded.constraint_name"
                ),
                params![table, column, name],
            )?;
        }
        None => {
            conn.execute(
                &format!("UPDATE {COLUMNS} SET constraint_name = NULL WHERE table_name = ?1 AND column_name = ?2"),
                params![table, column],
            )?;
        }
    }
    Ok(())
}

impl GeoPackage {
    /// Writes the shown names and rules (made with the extension's tables when the file lacks them), all in one
    /// transaction; a rule named again replaces nothing (each name is written once).
    pub fn write_schema(&mut self, columns: &[DataColumn], rules: &[(String, Rule)]) -> Result<()> {
        if columns.is_empty() && rules.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        ensure_schema_in(&tx)?;
        for (name, rule) in rules {
            insert_rule(&tx, name, rule)?;
        }
        for c in columns {
            tx.execute(
                &format!(
                    "INSERT OR REPLACE INTO {COLUMNS} (table_name, column_name, name, description, constraint_name)
                     VALUES (?1, ?2, ?3, ?4, ?5)"
                ),
                params![c.table, c.column, c.name, c.description, c.constraint],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The shown names and rules of `table`'s columns, by column; none when the file has no schema extension.
    pub fn data_columns(&self, table: &str) -> Result<Vec<(DataColumn, Option<Rule>)>> {
        let has = |name: &str| -> Result<bool> {
            Ok(self
                .conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [name],
                    |_| Ok(()),
                )
                .optional()?
                .is_some())
        };
        if !has(COLUMNS)? {
            return Ok(Vec::new());
        }
        let mut st = self.conn.prepare(&format!(
            "SELECT column_name, name, description, constraint_name FROM {COLUMNS} WHERE table_name = ?1"
        ))?;
        let columns: Vec<DataColumn> = st
            .query_map([table], |r| {
                Ok(DataColumn {
                    table: table.to_owned(),
                    column: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    constraint: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let rules = has(CONSTRAINTS)?;
        let mut out = Vec::with_capacity(columns.len());
        for c in columns {
            let rule = match (&c.constraint, rules) {
                (Some(name), true) => self.rule(name)?,
                _ => None,
            };
            out.push((c, rule));
        }
        Ok(out)
    }

    /// The rule named `name`: its range, or its values in the file's order; none when it is neither (a glob).
    fn rule(&self, name: &str) -> Result<Option<Rule>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT constraint_type, value, min, min_is_inclusive, max, max_is_inclusive, description
               FROM {CONSTRAINTS} WHERE constraint_name = ?1 ORDER BY rowid"
        ))?;
        /// A constraint's row: its type, value, least and whether it is in, most and whether it is in, description.
        type Row = (String, Option<String>, Option<f64>, Option<bool>, Option<f64>, Option<bool>, Option<String>);
        let rows: Vec<Row> =
            st.query_map([name], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        if let Some((_, _, min, min_in, max, max_in, _)) = rows.iter().find(|r| r.0.eq_ignore_ascii_case("range")) {
            return Ok(Some(Rule::Range {
                min: *min,
                min_included: min_in.unwrap_or(true),
                max: *max,
                max_included: max_in.unwrap_or(true),
            }));
        }
        let values: Vec<(String, Option<String>)> = rows
            .into_iter()
            .filter(|r| r.0.eq_ignore_ascii_case("enum"))
            .filter_map(|r| Some((r.1?, r.6)))
            .collect();
        Ok((!values.is_empty()).then_some(Rule::Values(values)))
    }
}
