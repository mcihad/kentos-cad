//! A GeoPackage keeps projects in its `kentos_projects` table (docs/adr/9107): `name` (the key), `metadata` (JSON: when
//! and by whom it was last written, its format) and `content` (the `.kcad` bytes). The table is registered as the
//! `kentos_projects` extension in `gpkg_extensions` (OGC 12-128r18 §2.3, as `kentos_uids` and `kentos_lineage` are) and
//! made by the first write; it is not in `gpkg_contents`, so it is never listed as a layer, and the file's other tables
//! stay as they are. A write is one transaction (`BEGIN IMMEDIATE`) on a connection of its own.

use kentos_data::DataError;
use kentos_data::projects::{PROJECTS_TABLE, ProjectStore, StoredProject, checked_name, ordered};
use kentos_geopackage::GeoPackage;
use rusqlite::{OptionalExtension, params};

use crate::source::{GpkgSource, failed, prepare};

/// The table, and the extensions table when a file another writer made has none yet. The extension's row is added
/// once: `INSERT OR IGNORE` does not stop a second one, as a NULL `column_name` never equals another in a UNIQUE
/// constraint.
const MAKE: &str = "CREATE TABLE IF NOT EXISTS kentos_projects (
    name TEXT PRIMARY KEY NOT NULL, metadata TEXT NOT NULL, content BLOB NOT NULL);
  CREATE TABLE IF NOT EXISTS gpkg_extensions (
    table_name TEXT, column_name TEXT, extension_name TEXT NOT NULL, definition TEXT NOT NULL,
    scope TEXT NOT NULL, CONSTRAINT ge_tce UNIQUE (table_name, column_name, extension_name));
  INSERT INTO gpkg_extensions
    SELECT 'kentos_projects', NULL, 'kentos_projects', 'https://kentos.app/geopackage/projects (docs/adr/9107)', 'read-write'
    WHERE NOT EXISTS (SELECT 1 FROM gpkg_extensions
      WHERE table_name = 'kentos_projects' AND column_name IS NULL AND extension_name = 'kentos_projects');";

/// Whether the file has the table yet.
fn has_table(gp: &GeoPackage) -> Result<bool, DataError> {
    gp.connection()
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [PROJECTS_TABLE],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .map_err(failed)
}

impl GpkgSource {
    /// Runs `f` in a transaction on a writable connection of its own: written when it succeeds, nothing otherwise.
    fn writing<T>(
        &self,
        f: impl FnOnce(&rusqlite::Connection) -> Result<T, DataError>,
    ) -> Result<T, DataError> {
        if !self.writable() {
            return Err(DataError::Denied(format!(
                "“{}” salt okunur; dosyanın izinlerini denetleyin.",
                self.path().display()
            )));
        }
        let gp =
            GeoPackage::open(self.path(), true).map_err(|e| DataError::Denied(e.to_string()))?;
        prepare(&gp)?;
        let c = gp.connection();
        c.execute_batch("BEGIN IMMEDIATE").map_err(failed)?;
        match f(c) {
            Ok(out) => {
                c.execute_batch("COMMIT").map_err(failed)?;
                Ok(out)
            }
            Err(e) => {
                let _ = c.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
}

impl ProjectStore for GpkgSource {
    fn projects(&self, _schema: Option<&str>) -> Result<Vec<StoredProject>, DataError> {
        self.with(|gp| {
            if !has_table(gp)? {
                return Ok(Vec::new());
            }
            let mut st = gp
                .connection()
                .prepare(
                    "SELECT name, json_extract(metadata, '$.modified'), json_extract(metadata, '$.user'),
                            length(content) FROM kentos_projects",
                )
                .map_err(failed)?;
            let rows = st
                .query_map([], |r| {
                    Ok(StoredProject {
                        name: r.get(0)?,
                        modified: r.get(1)?,
                        user: r.get(2)?,
                        size: r.get::<_, i64>(3)?.max(0) as u64,
                    })
                })
                .map_err(failed)?;
            Ok(ordered(rows.collect::<Result<Vec<_>, _>>().map_err(failed)?))
        })
    }

    fn read_project(&self, _schema: Option<&str>, name: &str) -> Result<Vec<u8>, DataError> {
        let name = checked_name(name)?;
        self.with(|gp| {
            if !has_table(gp)? {
                return Err(DataError::NoProject(name.to_owned()));
            }
            gp.connection()
                .query_row(
                    "SELECT content FROM kentos_projects WHERE name = ?1",
                    [name],
                    |r| r.get::<_, Vec<u8>>(0),
                )
                .optional()
                .map_err(failed)?
                .ok_or_else(|| DataError::NoProject(name.to_owned()))
        })
    }

    fn write_project(
        &self,
        _schema: Option<&str>,
        name: &str,
        content: &[u8],
        user: Option<&str>,
    ) -> Result<(), DataError> {
        let name = checked_name(name)?;
        self.writing(|c| {
            c.execute_batch(MAKE).map_err(failed)?;
            c.execute(
                "INSERT INTO kentos_projects (name, metadata, content)
                 VALUES (?1, json_object('modified', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 'user', ?2, 'format', 'kcad'), ?3)
                 ON CONFLICT (name) DO UPDATE SET metadata = excluded.metadata, content = excluded.content",
                params![name, user, content],
            )
            .map_err(failed)?;
            Ok(())
        })
    }

    fn delete_project(&self, _schema: Option<&str>, name: &str) -> Result<(), DataError> {
        let name = checked_name(name)?;
        self.writing(|c| {
            let there: i64 = c
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [PROJECTS_TABLE],
                    |r| r.get(0),
                )
                .map_err(failed)?;
            let gone = if there > 0 {
                c.execute("DELETE FROM kentos_projects WHERE name = ?1", [name])
                    .map_err(failed)?
            } else {
                0
            };
            if gone == 0 {
                return Err(DataError::NoProject(name.to_owned()));
            }
            Ok(())
        })
    }
}
