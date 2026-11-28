//! Write receipts in a GeoPackage (docs/adr/9126): its `kentos_writes` table (`id` the primary key, `at` when, `by` who,
//! `written` JSON), registered as the `kentos_writes` extension in `gpkg_extensions` as `kentos_projects` is and never
//! listed as a layer. A write puts its receipt in with its rows in one transaction. A file's commit has a sure answer;
//! the receipts give its writes the same record a database's have. The table is made only when the owner sets the
//! receipts up.

use kentos_data::{DataError, RECEIPTS_TABLE, Receipt};
use kentos_geopackage::GeoPackage;
use rusqlite::{OptionalExtension, params};

use crate::source::{GpkgSource, failed, prepare};

/// The table and its extension's row, made once.
const MAKE: &str = "CREATE TABLE IF NOT EXISTS kentos_writes (
    id TEXT PRIMARY KEY NOT NULL, at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), by TEXT,
    written TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS gpkg_extensions (
    table_name TEXT, column_name TEXT, extension_name TEXT NOT NULL, definition TEXT NOT NULL,
    scope TEXT NOT NULL, CONSTRAINT ge_tce UNIQUE (table_name, column_name, extension_name));
  INSERT INTO gpkg_extensions
    SELECT 'kentos_writes', NULL, 'kentos_writes', 'https://kentos.app/geopackage/writes (docs/adr/9126)', 'read-write'
    WHERE NOT EXISTS (SELECT 1 FROM gpkg_extensions
      WHERE table_name = 'kentos_writes' AND column_name IS NULL AND extension_name = 'kentos_writes');";

/// Whether the file has the table.
pub(crate) fn has_table(conn: &rusqlite::Connection) -> Result<bool, DataError> {
    conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [RECEIPTS_TABLE],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .map_err(failed)
}

/// A receipt written on `conn`, inside its transaction.
pub(crate) fn insert(conn: &rusqlite::Connection, receipt: &Receipt) -> Result<(), DataError> {
    conn.execute(
        "INSERT INTO kentos_writes (id, by, written) VALUES (?1, ?2, ?3)",
        params![receipt.id, receipt.by, receipt.written],
    )
    .map(|_| ())
    .map_err(failed)
}

impl GpkgSource {
    pub(crate) fn receipts_in(&self) -> Result<bool, DataError> {
        self.with(|gp| has_table(gp.connection()))
    }

    pub(crate) fn set_up_receipts_in(&self) -> Result<(), DataError> {
        if !self.writable() {
            return Err(DataError::Denied(format!(
                "“{}” salt okunur; dosyanın izinlerini denetleyin.",
                self.path().display()
            )));
        }
        let gp =
            GeoPackage::open(self.path(), true).map_err(|e| DataError::Denied(e.to_string()))?;
        prepare(&gp)?;
        gp.connection()
            .execute_batch(&format!("BEGIN IMMEDIATE; {MAKE} COMMIT;"))
            .map_err(failed)
    }

    pub(crate) fn receipt_in(&self, id: &str) -> Result<Option<Receipt>, DataError> {
        self.with(|gp| {
            let conn = gp.connection();
            if !has_table(conn)? {
                return Ok(None);
            }
            conn.query_row(
                "SELECT by, written FROM kentos_writes WHERE id = ?1",
                [id],
                |r| {
                    Ok(Receipt {
                        id: id.to_owned(),
                        by: r.get(0)?,
                        written: r.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(failed)
        })
    }
}
