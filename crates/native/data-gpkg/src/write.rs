//! A GeoPackage transaction (docs/adr/9100 §5): its own writing connection, `BEGIN IMMEDIATE` at once (a second writer
//! waits on the lock instead of failing half way), every statement inside it, `COMMIT` or `ROLLBACK` at the end and
//! `ROLLBACK` when it is dropped unfinished. A changed table's box in `gpkg_contents` and its time are set when it
//! commits; the spatial index follows by the file's own triggers. A value is written only to the fields a change
//! names; a field the table makes itself is never written.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};

use kentos_data::coerce::coerce;
use kentos_data::fit::fit;
use kentos_data::{Change, DataError, NewFeature, NewTable, Transaction};
use kentos_geopackage::{
    DataType, GeoPackage, TableDef, create_table_in, drop_table_in, quote, refresh_extent_in,
    registered_name_in, write_blob,
};
use rusqlite::types::Value as Sql;

use crate::convert::{Bind, column_of, fields_of, sql_of};
use crate::source::{GpkgSource, Shape, failed, prepare};

/// A table as written: its key, geometry column (name, type, system) and fields.
struct Target {
    key: String,
    geometry: Option<(String, String, i32)>,
    fields: Vec<kentos_data::Field>,
}

pub struct GpkgTransaction {
    gp: Option<GeoPackage>,
    shapes: Arc<RwLock<HashMap<String, Shape>>>,
    targets: HashMap<String, Target>,
    /// Feature tables whose rows changed: their box is set on commit.
    touched: BTreeSet<(String, String)>,
}

fn constraint_of(e: &rusqlite::Error) -> Option<DataError> {
    use kentos_data::ConstraintKind;
    let rusqlite::Error::SqliteFailure(f, msg) = e else {
        return None;
    };
    let kind = match f.extended_code {
        rusqlite::ffi::SQLITE_CONSTRAINT_CHECK => ConstraintKind::Check,
        rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE | rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY => {
            ConstraintKind::Unique
        }
        rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY => ConstraintKind::Foreign,
        rusqlite::ffi::SQLITE_CONSTRAINT_NOTNULL => ConstraintKind::NotNull,
        _ => return None,
    };
    let detail = msg.clone().unwrap_or_default();
    // SQLite says “NOT NULL constraint failed: table.column”, “CHECK constraint failed: name”.
    let tail = detail.rsplit(": ").next().map(str::to_owned);
    let (name, column) = match kind {
        ConstraintKind::NotNull | ConstraintKind::Unique => (
            None,
            tail.and_then(|t| {
                t.split(", ")
                    .next()
                    .and_then(|c| c.rsplit('.').next())
                    .map(str::to_owned)
            }),
        ),
        _ => (tail, None),
    };
    Some(DataError::Constraint {
        kind,
        name,
        column,
        detail,
    })
}

fn refused(e: rusqlite::Error) -> DataError {
    constraint_of(&e).unwrap_or_else(|| failed(e))
}

impl GpkgTransaction {
    pub(crate) fn begin(source: &GpkgSource) -> Result<GpkgTransaction, DataError> {
        let gp =
            GeoPackage::open(source.path(), true).map_err(|e| DataError::Denied(e.to_string()))?;
        prepare(&gp)?;
        gp.connection()
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| DataError::Unreachable {
                reach: kentos_data::Reach::Failed,
                message: format!(
                    "“{}” şu an başka bir yazıcının elinde (dosya kilitli); biraz sonra yeniden deneyin. {e}",
                    source.path().display()
                ),
            })?;
        Ok(GpkgTransaction {
            gp: Some(gp),
            shapes: source.shapes.clone(),
            targets: HashMap::new(),
            touched: BTreeSet::new(),
        })
    }

    fn gp(&self) -> Result<&GeoPackage, DataError> {
        self.gp
            .as_ref()
            .ok_or_else(|| DataError::Failed("işlem bitti".into()))
    }

    /// What writing `table` needs, read once inside the transaction.
    fn target(&mut self, table: &str) -> Result<&Target, DataError> {
        if !self.targets.contains_key(table) {
            let layer = self
                .gp()?
                .layers()
                .map_err(failed)?
                .into_iter()
                .find(|l| l.name == table)
                .ok_or_else(|| DataError::NoTable(table.to_owned()))?;
            if !matches!(layer.data_type, DataType::Features | DataType::Attributes)
                || layer.fid == "rowid"
            {
                return Err(DataError::Unsupported(format!(
                    "“{table}” tamsayı anahtarı olan bir öğe ya da öznitelik tablosu değil"
                )));
            }
            let geometry = layer.geometry.as_ref().map(|g| {
                (
                    g.name.clone(),
                    g.type_name.clone(),
                    layer.srs_id.unwrap_or(-1),
                )
            });
            let fields = fields_of(&layer);
            self.targets.insert(
                table.to_owned(),
                Target {
                    key: layer.fid.clone(),
                    geometry,
                    fields,
                },
            );
        }
        self.targets
            .get(table)
            .ok_or_else(|| DataError::NoTable(table.to_owned()))
    }

    /// The named values as columns and bound values, each fitted to its field: unknown fields refused, generated
    /// ones never written.
    fn columns(
        target: &Target,
        table: &str,
        values: &[(String, kentos_data::Value)],
    ) -> Result<Vec<(String, Sql)>, DataError> {
        let mut out = Vec::with_capacity(values.len());
        for (name, v) in values {
            let Some(f) = target.fields.iter().find(|f| &f.name == name) else {
                return Err(DataError::NoField {
                    table: table.to_owned(),
                    field: name.clone(),
                });
            };
            if !f.generated {
                out.push((quote(name), sql_of(&coerce(f, v)?)));
            }
        }
        Ok(out)
    }

    fn finish(&mut self, sql: &str) -> Result<(), DataError> {
        let gp = self
            .gp
            .take()
            .ok_or_else(|| DataError::Failed("işlem bitti".into()))?;
        let r = gp.connection().execute_batch(sql).map_err(refused);
        if let Ok(mut m) = self.shapes.write() {
            m.clear();
        }
        r
    }
}

impl Transaction for GpkgTransaction {
    fn rows(&mut self, table: &str, fids: &[i64]) -> Result<Vec<kentos_data::Feature>, DataError> {
        if fids.is_empty() {
            return Ok(Vec::new());
        }
        let target = self.target(table)?;
        let (key, geometry, fields) = (target.key.clone(), target.geometry.clone(), target.fields.clone());
        let mut select = vec![quote(&key)];
        select.push(geometry.as_ref().map_or_else(|| "NULL".to_owned(), |g| quote(&g.0)));
        select.extend(fields.iter().map(|f| quote(&f.name)));
        let keys = fids.iter().map(i64::to_string).collect::<Vec<_>>().join(", ");
        // The transaction holds the file (BEGIN IMMEDIATE): no other writer changes these rows before it ends.
        let sql = format!(
            "SELECT {} FROM {} WHERE {} IN ({keys}) ORDER BY {}",
            select.join(", "),
            quote(table),
            quote(&key),
            quote(&key)
        );
        let conn = self.gp()?.connection();
        let mut st = conn.prepare(&sql).map_err(failed)?;
        let mut rows = st.query([]).map_err(failed)?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().map_err(failed)? {
            let fid: i64 = r.get(0).map_err(failed)?;
            let geometry = match r.get_ref(1).map_err(failed)? {
                rusqlite::types::ValueRef::Blob(b) => Some(
                    kentos_geopackage::read_blob(b)
                        .map(|blob| blob.geometry)
                        .map_err(|e| e.to_string()),
                ),
                _ => None,
            };
            let mut values = Vec::with_capacity(fields.len());
            for (i, f) in fields.iter().enumerate() {
                values.push(crate::convert::value_of(r.get_ref(i + 2).map_err(failed)?, &f.kind));
            }
            out.push(kentos_data::Feature { fid, geometry, values });
        }
        Ok(out)
    }

    fn insert(&mut self, table: &str, rows: &[NewFeature]) -> Result<Vec<i64>, DataError> {
        let (key, geometry) = {
            let t = self.target(table)?;
            (t.key.clone(), t.geometry.clone())
        };
        let mut keys = Vec::with_capacity(rows.len());
        for row in rows {
            let target = self
                .targets
                .get(table)
                .ok_or_else(|| DataError::NoTable(table.to_owned()))?;
            let mut cols = Self::columns(target, table, &row.values)?;
            if let Some(fid) = row.fid {
                cols.push((quote(&key), Sql::Integer(fid)));
            }
            if let (Some((name, type_name, srs)), Some(g)) = (&geometry, &row.geometry) {
                let g = fit(g.clone(), type_name);
                cols.push((quote(name), Sql::Blob(write_blob(*srs, &g))));
            }
            let sql = if cols.is_empty() {
                format!("INSERT INTO {} DEFAULT VALUES", quote(table))
            } else {
                format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    quote(table),
                    cols.iter()
                        .map(|(c, _)| c.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    vec!["?"; cols.len()].join(", ")
                )
            };
            let conn = self.gp()?.connection();
            let binds: Vec<Bind<'_>> = cols.iter().map(|(_, v)| Bind(v)).collect();
            conn.prepare_cached(&sql)
                .and_then(|mut st| st.execute(rusqlite::params_from_iter(binds.iter())))
                .map_err(refused)?;
            keys.push(conn.last_insert_rowid());
        }
        if let Some((name, ..)) = geometry {
            self.touched.insert((table.to_owned(), name));
        }
        Ok(keys)
    }

    fn update(&mut self, table: &str, changes: &[Change]) -> Result<(), DataError> {
        let (key, geometry) = {
            let t = self.target(table)?;
            (t.key.clone(), t.geometry.clone())
        };
        for c in changes {
            let target = self
                .targets
                .get(table)
                .ok_or_else(|| DataError::NoTable(table.to_owned()))?;
            let mut cols = Self::columns(target, table, &c.values)?;
            match (&geometry, &c.geometry) {
                (Some((name, type_name, srs)), Some(g)) => cols.push((
                    quote(name),
                    g.as_ref().map_or(Sql::Null, |g| {
                        Sql::Blob(write_blob(*srs, &fit(g.clone(), type_name)))
                    }),
                )),
                (None, Some(_)) => {
                    return Err(DataError::Unsupported(format!(
                        "“{table}” tablosunun geometrisi yok"
                    )));
                }
                _ => {}
            }
            let conn = self.gp()?.connection();
            let found = if cols.is_empty() {
                conn.query_row(
                    &format!(
                        "SELECT count(*) FROM {} WHERE {} = ?1",
                        quote(table),
                        quote(&key)
                    ),
                    [c.fid],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(failed)?
                    == 1
            } else {
                let sets: Vec<String> = cols.iter().map(|(n, _)| format!("{n} = ?")).collect();
                let mut binds: Vec<Bind<'_>> = cols.iter().map(|(_, v)| Bind(v)).collect();
                let fid = Sql::Integer(c.fid);
                binds.push(Bind(&fid));
                conn.prepare_cached(&format!(
                    "UPDATE {} SET {} WHERE {} = ?",
                    quote(table),
                    sets.join(", "),
                    quote(&key)
                ))
                .and_then(|mut st| st.execute(rusqlite::params_from_iter(binds.iter())))
                .map_err(refused)?
                    == 1
            };
            if !found {
                return Err(DataError::Conflict(format!(
                    "“{table}” tablosunda {} numaralı kayıt yok (silinmiş olabilir)",
                    c.fid
                )));
            }
        }
        if let Some((name, ..)) = geometry {
            self.touched.insert((table.to_owned(), name));
        }
        Ok(())
    }

    fn delete(&mut self, table: &str, fids: &[i64]) -> Result<(), DataError> {
        let (key, geometry) = {
            let t = self.target(table)?;
            (t.key.clone(), t.geometry.clone())
        };
        let conn = self.gp()?.connection();
        let has_uids: bool = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'kentos_uids'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map_err(failed)?
            > 0;
        for fid in fids {
            let n = conn
                .prepare_cached(&format!(
                    "DELETE FROM {} WHERE {} = ?1",
                    quote(table),
                    quote(&key)
                ))
                .and_then(|mut st| st.execute([fid]))
                .map_err(refused)?;
            if n != 1 {
                return Err(DataError::Conflict(format!(
                    "“{table}” tablosunda {fid} numaralı kayıt yok (silinmiş olabilir)"
                )));
            }
            if has_uids {
                conn.execute(
                    "DELETE FROM kentos_uids WHERE table_name = ?1 AND fid = ?2",
                    rusqlite::params![table, fid],
                )
                .map_err(failed)?;
            }
        }
        if let Some((name, ..)) = geometry {
            self.touched.insert((table.to_owned(), name));
        }
        Ok(())
    }

    fn create_table(&mut self, t: &NewTable, replace: bool) -> Result<(), DataError> {
        if t.name.trim().is_empty() {
            return Err(DataError::Invalid("Tablonun adı boş olamaz.".into()));
        }
        let conn = self.gp()?.connection();
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = ?1 COLLATE NOCASE AND type IN ('table', 'view')",
                [&t.name],
                |r| r.get(0),
            )
            .map_err(failed)?;
        if exists > 0 {
            if !replace {
                return Err(DataError::Invalid(format!(
                    "Dosyada “{}” adlı bir tablo zaten var; başka bir ad verin ya da üstüne yazmayı seçin.",
                    t.name
                )));
            }
            if registered_name_in(conn, &t.name).map_err(failed)?.is_none() {
                return Err(DataError::Unsupported(format!(
                    "“{}” GeoPackage'ın kendi tablolarından biri; üstüne yazılamaz",
                    t.name
                )));
            }
            drop_table_in(conn, &t.name).map_err(failed)?;
        }
        if let Some(srid) = t.geometry.as_ref().and_then(|g| g.srid)
            && srid > 0
        {
            let known: i64 = conn
                .query_row(
                    "SELECT count(*) FROM gpkg_spatial_ref_sys WHERE srs_id = ?1",
                    [srid],
                    |r| r.get(0),
                )
                .map_err(failed)?;
            if known == 0 {
                // The system's name and WKT as the table brings them; without them it is named by its EPSG code and
                // its definition is the code's (a reader resolves it from there).
                let (name, definition) = match &t.system {
                    Some(s) => (s.name.clone(), s.wkt.clone()),
                    None => (format!("EPSG:{srid}"), "undefined".to_owned()),
                };
                conn.execute(
                    "INSERT INTO gpkg_spatial_ref_sys VALUES (?1, ?2, 'EPSG', ?2, ?3, NULL)",
                    rusqlite::params![name, srid, definition],
                )
                .map_err(failed)?;
            }
        }
        let def = TableDef {
            name: t.name.clone(),
            data_type: if t.geometry.is_some() {
                DataType::Features
            } else {
                DataType::Attributes
            },
            identifier: Some(free_identifier(conn, t.title.as_deref().unwrap_or(&t.name), &t.name).map_err(failed)?),
            description: t.description.clone(),
            srs_id: t.geometry.as_ref().map(|g| g.srid.unwrap_or(-1)),
            geometry: t
                .geometry
                .as_ref()
                .map(|g| kentos_geopackage::GeometryColumn {
                    name: g.name.clone(),
                    type_name: g.type_name.to_ascii_uppercase(),
                    z: u8::from(g.z),
                    m: u8::from(g.m),
                }),
            columns: t.fields.iter().map(column_of).collect(),
        };
        create_table_in(conn, &def).map_err(failed)?;
        self.targets.remove(&t.name);
        Ok(())
    }

    fn drop_table(&mut self, table: &str) -> Result<(), DataError> {
        let conn = self.gp()?.connection();
        if registered_name_in(conn, table).map_err(failed)?.is_none() {
            return Err(DataError::NoTable(table.to_owned()));
        }
        drop_table_in(conn, table).map_err(failed)?;
        self.targets.remove(table);
        Ok(())
    }

    fn commit(mut self: Box<Self>) -> Result<(), DataError> {
        let touched = std::mem::take(&mut self.touched);
        for (table, column) in &touched {
            refresh_extent_in(self.gp()?.connection(), table, column).map_err(failed)?;
        }
        self.finish("COMMIT")
    }

    fn rollback(mut self: Box<Self>) -> Result<(), DataError> {
        self.finish("ROLLBACK")
    }
}

impl Drop for GpkgTransaction {
    fn drop(&mut self) {
        if let Some(gp) = self.gp.take() {
            let _ = gp.connection().execute_batch("ROLLBACK");
        }
    }
}

/// The identifier a new table is listed under: its title, unless another table is listed under it
/// (`gpkg_contents.identifier` is unique; the same layer written twice as two tables met it); then the title with the
/// table's name, then with a number.
fn free_identifier(conn: &rusqlite::Connection, title: &str, table: &str) -> rusqlite::Result<String> {
    let taken = |id: &str| -> rusqlite::Result<bool> {
        conn.query_row(
            "SELECT count(*) FROM gpkg_contents WHERE identifier = ?1 AND table_name <> ?2 COLLATE NOCASE",
            rusqlite::params![id, table],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
    };
    if !taken(title)? {
        return Ok(title.to_owned());
    }
    let named = format!("{title} ({table})");
    if title != table && !taken(&named)? {
        return Ok(named);
    }
    let mut n = 2u32;
    loop {
        let numbered = format!("{title} ({n})");
        if !taken(&numbered)? {
            return Ok(numbered);
        }
        n += 1;
    }
}
