//! GeoPackage related tables (OGC 18-000, docs/adr/0532 §2): the relations a file declares in `gpkgext_relations`,
//! each with a mapping table of `base_id` and `related_id` rows. KentOS writes a many-to-many reference (a child
//! field holding several parents' ids) as one such relation, named `kentos_references`, with the child's table as the
//! base and the parent's as the related table; any other relation in a file is read and reported, never rewritten.

use rusqlite::{Transaction, params};

use crate::{Error, GeoPackage, Layer, Result, quote};

/// The extension's name in `gpkg_extensions`.
pub const RELATED_TABLES: &str = "gpkg_related_tables";
/// The table that lists the file's relations.
pub const RELATIONS_TABLE: &str = "gpkgext_relations";
/// The relation name KentOS writes (an extension's relation name: author and name, lowercase).
pub const KENTOS_RELATION: &str = "kentos_references";
const DEFINITION: &str = "http://www.opengis.net/doc/IS/gpkg-rte/1.0";

/// A relation of the file (a row of `gpkgext_relations`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relation {
    pub id: i64,
    pub base_table: String,
    pub base_column: String,
    pub related_table: String,
    pub related_column: String,
    pub name: String,
    pub mapping: String,
}

/// A row of a table a mapping refers to: one the file holds (by fid) or one the same transaction inserts (the
/// table's job and the row's place among its inserts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Fid(i64),
    Inserted(usize, usize),
}

/// A mapping table's whole content after the edit: its pairs of base and related rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapEdit {
    pub mapping: String,
    pub pairs: Vec<(Key, Key)>,
}

fn has_table(conn: &rusqlite::Connection, name: &str) -> Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

impl GeoPackage {
    /// Whether the file has a table (or view) of the name, a KentOS table or SQLite's own included.
    pub fn table_exists(&self, name: &str) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type IN ('table', 'view') AND lower(name) = lower(?1)",
            [name],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// The relations the file declares, in order; none when it has no `gpkgext_relations`.
    pub fn relations(&self) -> Result<Vec<Relation>> {
        if !has_table(&self.conn, RELATIONS_TABLE)? {
            return Ok(Vec::new());
        }
        let mut st = self.conn.prepare(&format!(
            "SELECT id, base_table_name, base_primary_column, related_table_name, related_primary_column,
                    relation_name, mapping_table_name FROM {RELATIONS_TABLE} ORDER BY id"
        ))?;
        let rows = st
            .query_map([], |r| {
                Ok(Relation {
                    id: r.get(0)?,
                    base_table: r.get(1)?,
                    base_column: r.get(2)?,
                    related_table: r.get(3)?,
                    related_column: r.get(4)?,
                    name: r.get(5)?,
                    mapping: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// The tables that hold no data of their own (docs/adr/0627 §6, §7): the relations' mapping tables and QGIS's
    /// `layer_styles`, whose content comes in as reference fields and layers' looks; a file's table list leaves them out.
    pub fn side_tables(&self) -> Result<std::collections::BTreeSet<String>> {
        Ok(self
            .relations()?
            .into_iter()
            .map(|r| r.mapping)
            .chain([crate::styles::STYLES_TABLE.to_owned()])
            .collect())
    }

    /// Why a relation is not one KentOS reads, if it is not: its mapping table is missing or lacks the integer columns
    /// `base_id` and `related_id`, or a table or primary column it names is not in the file.
    pub fn relation_problem(&self, r: &Relation, layers: &[Layer]) -> Result<Option<String>> {
        for (t, c) in [
            (&r.base_table, &r.base_column),
            (&r.related_table, &r.related_column),
        ] {
            let Some(l) = layers.iter().find(|l| &l.name == t) else {
                return Ok(Some(format!("“{t}” tablosu dosyada yok")));
            };
            if !l.fid.eq_ignore_ascii_case(c) {
                return Ok(Some(format!(
                    "“{t}” tablosunun anahtarı “{}”, ilişki “{c}” diyor",
                    l.fid
                )));
            }
        }
        if !has_table(&self.conn, &r.mapping)? {
            return Ok(Some(format!("“{}” eşleme tablosu dosyada yok", r.mapping)));
        }
        let mut st = self
            .conn
            .prepare(&format!("PRAGMA table_info({})", quote(&r.mapping)))?;
        let columns: Vec<(String, String)> = st
            .query_map([], |c| Ok((c.get::<_, String>(1)?, c.get::<_, String>(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for need in ["base_id", "related_id"] {
            match columns.iter().find(|(n, _)| n == need) {
                None => {
                    return Ok(Some(format!(
                        "“{}” eşleme tablosunda {need} sütunu yok",
                        r.mapping
                    )));
                }
                Some((_, decl)) if !decl.to_ascii_uppercase().contains("INT") => {
                    return Ok(Some(format!(
                        "“{}” eşleme tablosunun {need} sütunu tam sayı değil",
                        r.mapping
                    )));
                }
                Some(_) => {}
            }
        }
        Ok(None)
    }

    /// A mapping table's rows, in the order they were written.
    pub fn mapping_rows(&self, mapping: &str) -> Result<Vec<(i64, i64)>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT base_id, related_id FROM {} ORDER BY rowid",
            quote(mapping)
        ))?;
        let rows = st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// Declares a relation of KentOS's kind between two tables of the file, with its mapping table made and filled
    /// with `pairs` (base fid, related fid), in one transaction. Refuses a mapping table name the file already uses.
    pub fn add_relation(
        &mut self,
        base: &Layer,
        related: &Layer,
        mapping: &str,
        pairs: &[(i64, i64)],
    ) -> Result<i64> {
        if has_table(&self.conn, mapping)? {
            return Err(Error(format!(
                "“{mapping}” adlı bir tablo dosyada zaten var"
            )));
        }
        let tx = self.conn.transaction()?;
        tx.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {RELATIONS_TABLE} (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               base_table_name TEXT NOT NULL,
               base_primary_column TEXT NOT NULL DEFAULT 'id',
               related_table_name TEXT NOT NULL,
               related_primary_column TEXT NOT NULL DEFAULT 'id',
               relation_name TEXT NOT NULL,
               mapping_table_name TEXT NOT NULL UNIQUE);
             CREATE TABLE {} (base_id INTEGER NOT NULL, related_id INTEGER NOT NULL);",
            quote(mapping)
        ))?;
        super::ensure_extensions(&tx)?;
        tx.execute(
            "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, NULL, ?2, ?3, 'read-write')",
            params![RELATIONS_TABLE, RELATED_TABLES, DEFINITION],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, NULL, ?2, ?3, 'read-write')",
            params![mapping, RELATED_TABLES, DEFINITION],
        )?;
        tx.execute(
            &format!(
                "INSERT INTO {RELATIONS_TABLE} (base_table_name, base_primary_column, related_table_name,
                   related_primary_column, relation_name, mapping_table_name) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            ),
            params![
                base.name,
                base.fid,
                related.name,
                related.fid,
                KENTOS_RELATION,
                mapping
            ],
        )?;
        let id = tx.last_insert_rowid();
        {
            let mut put = tx.prepare(&format!(
                "INSERT INTO {} (base_id, related_id) VALUES (?1, ?2)",
                quote(mapping)
            ))?;
            for (b, r) in pairs {
                put.execute(params![b, r])?;
            }
        }
        tx.commit()?;
        Ok(id)
    }

    /// Takes a relation of KentOS's kind out of the file: its row, its mapping table's extension row and the table.
    pub fn drop_relation(&mut self, relation: &Relation) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            &format!("DELETE FROM {RELATIONS_TABLE} WHERE id = ?1"),
            [relation.id],
        )?;
        tx.execute(
            "DELETE FROM gpkg_extensions WHERE table_name = ?1 AND extension_name = ?2",
            params![relation.mapping, RELATED_TABLES],
        )?;
        tx.execute_batch(&format!("DROP TABLE {};", quote(&relation.mapping)))?;
        // The last relation gone, the file is as it was before the first: no relations table, no extension row.
        let left: i64 = tx.query_row(
            &format!("SELECT count(*) FROM {RELATIONS_TABLE}"),
            [],
            |r| r.get(0),
        )?;
        if left == 0 {
            tx.execute_batch(&format!("DROP TABLE {RELATIONS_TABLE};"))?;
            tx.execute(
                "DELETE FROM gpkg_extensions WHERE table_name = ?1 AND extension_name = ?2",
                params![RELATIONS_TABLE, RELATED_TABLES],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

/// Replaces a mapping table's rows with `edit`'s pairs, resolving the rows its transaction inserted by `fids` (each
/// job's inserted fids).
pub(crate) fn write_mapping(tx: &Transaction<'_>, edit: &MapEdit, fids: &[Vec<i64>]) -> Result<()> {
    let resolve = |k: &Key| -> Result<i64> {
        match k {
            Key::Fid(f) => Ok(*f),
            Key::Inserted(job, i) => {
                fids.get(*job)
                    .and_then(|f| f.get(*i))
                    .copied()
                    .ok_or_else(|| {
                        Error(format!(
                            "“{}” eşlemesi eklenmemiş bir satıra bağlı",
                            edit.mapping
                        ))
                    })
            }
        }
    };
    let t = quote(&edit.mapping);
    tx.execute_batch(&format!("DELETE FROM {t};"))?;
    let mut put = tx.prepare(&format!(
        "INSERT INTO {t} (base_id, related_id) VALUES (?1, ?2)"
    ))?;
    for (b, r) in &edit.pairs {
        put.execute(params![resolve(b)?, resolve(r)?])?;
    }
    Ok(())
}
