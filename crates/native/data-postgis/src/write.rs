//! A PostGIS transaction (docs/adr/9100 §6): one pooled connection in `BEGIN` from the first write to `COMMIT` or
//! `ROLLBACK` (dropped unfinished, it rolls back). A value is written to its field cast to the field's own type after
//! [`kentos_data::coerce::coerce`] has fitted it; a geometry fitted to the column's type and dimensions (a part made a
//! multi of one, Z and M added as 0 or dropped). New tables get an identity key, typed columns, a typed geometry
//! column and a GiST index.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use kentos_data::coerce::coerce;
use kentos_data::fit::fit;
use kentos_data::geom::write_wkb;
use kentos_data::{Change, DataError, FieldKind, Geometry, NewFeature, NewTable, Transaction};
use sqlx::{Postgres, Row};

use crate::catalog::{Shape, qualified, quote, shape};
use crate::connect::{PgSource, error_of};
use crate::read::bound;
use crate::runtime;
use crate::sql::{Arg, Sql, write_arg};

pub struct PgTransaction {
    tx: Option<sqlx::Transaction<'static, Postgres>>,
    shapes: Arc<RwLock<HashMap<String, Shape>>>,
    /// Shapes read inside this transaction (it sees the tables it made).
    own: HashMap<String, Shape>,
}

/// A geometry's expression for its column: the WKB bound, the system set, the dimensions forced to the column's.
fn geometry_value(shape: &Shape, g: &Geometry, s: &mut Sql) -> Result<String, DataError> {
    let Some(col) = &shape.schema.geometry else {
        return Err(DataError::Unsupported(format!(
            "“{}” tablosunun geometrisi yok",
            shape.relation
        )));
    };
    let g = fit(g.clone(), &col.type_name);
    let p = s.bind(Arg::Bytes(write_wkb(&g)));
    let force = match (col.z, col.m) {
        (true, true) => "ST_Force4D",
        (true, false) => "ST_Force3DZ",
        (false, true) => "ST_Force3DM",
        (false, false) => "ST_Force2D",
    };
    let srid = col.srid.unwrap_or(0);
    let mut e = format!("{force}(ST_SetSRID(ST_GeomFromWKB({p}), {srid}))");
    if shape.geography {
        e.push_str("::geography");
    }
    Ok(e)
}

/// PostgreSQL's type for a new field.
fn native_of(kind: &FieldKind) -> String {
    match kind {
        FieldKind::Bool => "boolean".into(),
        FieldKind::Int => "bigint".into(),
        FieldKind::Real => "double precision".into(),
        FieldKind::Decimal {
            precision: Some(p),
            scale,
        } => format!("numeric({p},{})", scale.unwrap_or(0)),
        FieldKind::Decimal { .. } => "numeric".into(),
        FieldKind::Text { max: Some(m) } => format!("varchar({m})"),
        FieldKind::Text { max: None } | FieldKind::Other => "text".into(),
        FieldKind::Date => "date".into(),
        FieldKind::DateTime => "timestamp".into(),
        FieldKind::Time => "time".into(),
        FieldKind::Blob => "bytea".into(),
        FieldKind::Json => "jsonb".into(),
        FieldKind::Uuid => "uuid".into(),
    }
}

impl PgTransaction {
    pub(crate) fn begin(source: &PgSource) -> Result<PgTransaction, DataError> {
        let tx = runtime()?.block_on(source.pool.begin()).map_err(error_of)?;
        Ok(PgTransaction {
            tx: Some(tx),
            shapes: source.shapes.clone(),
            own: HashMap::new(),
        })
    }

    fn tx(&mut self) -> Result<&mut sqlx::Transaction<'static, Postgres>, DataError> {
        self.tx
            .as_mut()
            .ok_or_else(|| DataError::Failed("işlem bitti".into()))
    }

    fn shape(&mut self, table: &str) -> Result<Shape, DataError> {
        if let Some(s) = self.own.get(table) {
            return Ok(s.clone());
        }
        let rt = runtime()?;
        let tx = self.tx()?;
        let s = rt.block_on(shape(tx, table))?;
        self.own.insert(table.to_owned(), s.clone());
        Ok(s)
    }

    /// The named values as `(column, expression)`, each fitted to its field; generated fields left out.
    fn assignments(
        shape: &Shape,
        table: &str,
        values: &[(String, kentos_data::Value)],
        s: &mut Sql,
    ) -> Result<Vec<(String, String)>, DataError> {
        let mut out = Vec::with_capacity(values.len());
        for (name, v) in values {
            let f = shape.schema.field(name).ok_or_else(|| DataError::NoField {
                table: table.to_owned(),
                field: name.clone(),
            })?;
            if f.generated {
                continue;
            }
            let v = coerce(f, v)?;
            out.push((quote(name), write_arg(&v, &f.native, s)));
        }
        Ok(out)
    }

    fn run(&mut self, sql: &str, args: &[Arg]) -> Result<u64, DataError> {
        let rt = runtime()?;
        let tx = self.tx()?;
        rt.block_on(bound(sql, args).execute(&mut **tx))
            .map(|r| r.rows_affected())
            .map_err(error_of)
    }

    fn finish(&mut self, commit: bool) -> Result<(), DataError> {
        let tx = self
            .tx
            .take()
            .ok_or_else(|| DataError::Failed("işlem bitti".into()))?;
        let r = runtime()?
            .block_on(async {
                if commit {
                    tx.commit().await
                } else {
                    tx.rollback().await
                }
            })
            .map_err(error_of);
        if let Ok(mut m) = self.shapes.write() {
            m.clear();
        }
        r
    }
}

impl Transaction for PgTransaction {
    fn rows(&mut self, table: &str, fids: &[i64]) -> Result<Vec<kentos_data::Feature>, DataError> {
        if fids.is_empty() {
            return Ok(Vec::new());
        }
        let shape = self.shape(table)?;
        let mut q = Sql::default();
        let keys = q.bind(Arg::Ints(fids.to_vec()));
        let mut select = vec![format!("t.{}::int8", quote(&shape.schema.key))];
        select.push(match crate::sql::geometry_expr(&shape) {
            Some(g) => format!("ST_AsBinary({g})"),
            None => "NULL::bytea".to_owned(),
        });
        select.extend(
            shape
                .schema
                .fields
                .iter()
                .map(|f| crate::sql::read_expr(&format!("t.{}", quote(&f.name)), &f.kind)),
        );
        // Locked until the transaction ends: no other writer changes them meanwhile.
        let sql = format!(
            "SELECT {} FROM {} t WHERE t.{}::int8 = ANY({keys}::int8[]) ORDER BY 1 FOR UPDATE OF t",
            select.join(", "),
            shape.sql,
            quote(&shape.schema.key)
        );
        let rt = runtime()?;
        let tx = self.tx()?;
        let rows = rt
            .block_on(bound(&sql, &q.args).fetch_all(&mut **tx))
            .map_err(error_of)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in &rows {
            let fid: i64 = r.try_get(0).map_err(error_of)?;
            let wkb: Option<Vec<u8>> = r.try_get(1).map_err(error_of)?;
            let mut values = Vec::with_capacity(shape.schema.fields.len());
            for (i, f) in shape.schema.fields.iter().enumerate() {
                values.push(crate::sql::value_of(r.try_get(i + 2).map_err(error_of)?, &f.kind));
            }
            out.push(kentos_data::Feature {
                fid,
                geometry: wkb.map(|b| kentos_data::geom::read_wkb(&b).map_err(|e| e.to_string())),
                values,
            });
        }
        Ok(out)
    }

    fn insert(&mut self, table: &str, rows: &[NewFeature]) -> Result<Vec<i64>, DataError> {
        let shape = self.shape(table)?;
        let mut keys = Vec::with_capacity(rows.len());
        for row in rows {
            let mut s = Sql::default();
            let mut cols = Self::assignments(&shape, table, &row.values, &mut s)?;
            if let Some(fid) = row.fid {
                let p = s.bind(Arg::Int(fid));
                cols.push((quote(&shape.schema.key), p));
            }
            if let (Some(col), Some(g)) = (&shape.schema.geometry, &row.geometry) {
                let e = geometry_value(&shape, g, &mut s)?;
                cols.push((quote(&col.name), e));
            }
            let sql = if cols.is_empty() {
                format!(
                    "INSERT INTO {} DEFAULT VALUES RETURNING {}::int8",
                    shape.sql,
                    quote(&shape.schema.key)
                )
            } else {
                format!(
                    "INSERT INTO {} ({}) VALUES ({}) RETURNING {}::int8",
                    shape.sql,
                    cols.iter()
                        .map(|(c, _)| c.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    cols.iter()
                        .map(|(_, e)| e.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    quote(&shape.schema.key)
                )
            };
            let rt = runtime()?;
            let tx = self.tx()?;
            let key: i64 = rt
                .block_on(bound(&sql, &s.args).fetch_one(&mut **tx))
                .and_then(|r| r.try_get(0))
                .map_err(error_of)?;
            keys.push(key);
        }
        Ok(keys)
    }

    fn update(&mut self, table: &str, changes: &[Change]) -> Result<(), DataError> {
        let shape = self.shape(table)?;
        for c in changes {
            let mut s = Sql::default();
            let mut cols = Self::assignments(&shape, table, &c.values, &mut s)?;
            if let Some(g) = &c.geometry {
                let col = shape.schema.geometry.as_ref().ok_or_else(|| {
                    DataError::Unsupported(format!("“{table}” tablosunun geometrisi yok"))
                })?;
                let e = match g {
                    Some(g) => geometry_value(&shape, g, &mut s)?,
                    None => "NULL".into(),
                };
                cols.push((quote(&col.name), e));
            }
            let fid = s.bind(Arg::Int(c.fid));
            let sql = if cols.is_empty() {
                format!(
                    "UPDATE {} SET {k} = {k} WHERE {k} = {fid}",
                    shape.sql,
                    k = quote(&shape.schema.key)
                )
            } else {
                format!(
                    "UPDATE {} SET {} WHERE {} = {fid}",
                    shape.sql,
                    cols.iter()
                        .map(|(c, e)| format!("{c} = {e}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    quote(&shape.schema.key)
                )
            };
            if self.run(&sql, &s.args)? != 1 {
                return Err(DataError::Conflict(format!(
                    "“{table}” tablosunda {} numaralı kayıt yok (silinmiş olabilir)",
                    c.fid
                )));
            }
        }
        Ok(())
    }

    fn delete(&mut self, table: &str, fids: &[i64]) -> Result<(), DataError> {
        if fids.is_empty() {
            return Ok(());
        }
        let shape = self.shape(table)?;
        let sql = format!(
            "DELETE FROM {} WHERE {}::int8 = ANY($1::int8[])",
            shape.sql,
            quote(&shape.schema.key)
        );
        let n = self.run(&sql, &[Arg::Ints(fids.to_vec())])?;
        if n != fids.len() as u64 {
            return Err(DataError::Conflict(format!(
                "“{table}” tablosunda silinecek {} kaydın {} tanesi yok (silinmiş olabilir)",
                fids.len(),
                fids.len() as u64 - n
            )));
        }
        Ok(())
    }

    fn create_table(&mut self, t: &NewTable, replace: bool) -> Result<(), DataError> {
        if t.name.trim().is_empty() {
            return Err(DataError::Invalid("Tablonun adı boş olamaz.".into()));
        }
        let (sql, _, rel) = qualified(&t.name);
        if replace {
            self.run(&format!("DROP TABLE IF EXISTS {sql}"), &[])?;
        }
        let key = if t.key.trim().is_empty() {
            "fid"
        } else {
            t.key.as_str()
        };
        let mut cols = vec![format!(
            "{} bigint GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY",
            quote(key)
        )];
        for f in &t.fields {
            let mut c = format!("{} {}", quote(&f.name), native_of(&f.kind));
            if !f.nullable {
                c.push_str(" NOT NULL");
            }
            if let Some(d) = &f.default {
                c.push_str(&format!(" DEFAULT {d}"));
            }
            cols.push(c);
        }
        if let Some(g) = &t.geometry {
            let dims = match (g.z, g.m) {
                (true, true) => "ZM",
                (true, false) => "Z",
                (false, true) => "M",
                (false, false) => "",
            };
            let base = if g.type_name.eq_ignore_ascii_case("GEOMETRY") {
                "Geometry".to_owned()
            } else {
                g.type_name.to_ascii_uppercase()
            };
            cols.push(format!(
                "{} geometry({base}{dims}, {})",
                quote(&g.name),
                g.srid.unwrap_or(0)
            ));
        }
        self.run(&format!("CREATE TABLE {sql} ({})", cols.join(", ")), &[])?;
        if let (Some(g), true) = (&t.geometry, t.spatial_index) {
            self.run(
                &format!(
                    "CREATE INDEX {} ON {sql} USING GIST ({})",
                    quote(&format!("{rel}_{}_gist", g.name)),
                    quote(&g.name)
                ),
                &[],
            )?;
        }
        if let Some(d) = t.description.as_ref().or(t.title.as_ref()) {
            self.run(
                &format!("COMMENT ON TABLE {sql} IS '{}'", d.replace('\'', "''")),
                &[],
            )?;
        }
        self.own.remove(&t.name);
        Ok(())
    }

    fn drop_table(&mut self, table: &str) -> Result<(), DataError> {
        let (sql, ..) = qualified(table);
        self.run(&format!("DROP TABLE {sql}"), &[])?;
        self.own.remove(table);
        Ok(())
    }

    fn commit(mut self: Box<Self>) -> Result<(), DataError> {
        self.finish(true)
    }

    fn rollback(mut self: Box<Self>) -> Result<(), DataError> {
        self.finish(false)
    }
}
