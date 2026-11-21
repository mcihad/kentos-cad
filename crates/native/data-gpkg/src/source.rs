//! A GeoPackage file as a source (docs/adr/9100 §5): its catalog from `gpkg_contents`, a table's shape cached until a
//! transaction of ours commits, and every read one statement on a kept read-only connection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use kentos_data::driver::{Capabilities, Read};
use kentos_data::simplify::simplify;
use kentos_data::{
    Cancel, Condition, DataError, Feature, Reach, Request, Schema, Source, TableInfo, TableKind,
    Transaction, Value,
};
use kentos_geopackage::{DataType, GeoPackage, Layer, quote, read_blob};

use crate::convert::{Bind, fields_of, geometry_of, value_of};
use crate::sql::{order_and_page, register_fold, where_of};
use crate::write::GpkgTransaction;

/// Read connections kept idle at most.
const KEPT: usize = 4;
/// How long a statement waits on a writer's lock before it gives up.
pub(crate) const BUSY: Duration = Duration::from_secs(10);

/// A table's shape with what reading it needs.
#[derive(Clone)]
pub(crate) struct Shape {
    pub schema: Schema,
    pub layer: Layer,
}

/// An opened GeoPackage.
pub struct GpkgSource {
    address: String,
    path: PathBuf,
    idle: Mutex<Vec<GeoPackage>>,
    /// Shared with this source's transactions, which forget it when they write.
    pub(crate) shapes: Arc<RwLock<HashMap<String, Shape>>>,
}

pub(crate) fn failed(e: impl std::fmt::Display) -> DataError {
    DataError::Failed(e.to_string())
}

/// A connection made ready for our statements.
pub(crate) fn prepare(gp: &GeoPackage) -> Result<(), DataError> {
    let c = gp.connection();
    c.busy_timeout(BUSY).map_err(failed)?;
    register_fold(c).map_err(failed)
}

impl GpkgSource {
    pub fn open(address: &str, path: &Path) -> Result<GpkgSource, DataError> {
        let gp = GeoPackage::open(path, false).map_err(|e| DataError::Unreachable {
            reach: Reach::Failed,
            message: e.to_string(),
        })?;
        prepare(&gp)?;
        Ok(GpkgSource {
            address: address.to_owned(),
            path: path.to_owned(),
            idle: Mutex::new(vec![gp]),
            shapes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs `f` on a kept read connection (a new one when none is idle), keeping it again afterwards.
    pub(crate) fn with<T>(
        &self,
        f: impl FnOnce(&GeoPackage) -> Result<T, DataError>,
    ) -> Result<T, DataError> {
        let taken = self.idle.lock().ok().and_then(|mut v| v.pop());
        let gp = match taken {
            Some(gp) => gp,
            None => {
                let gp =
                    GeoPackage::open(&self.path, false).map_err(|e| DataError::Unreachable {
                        reach: Reach::Offline,
                        message: e.to_string(),
                    })?;
                prepare(&gp)?;
                gp
            }
        };
        let out = f(&gp);
        if let Ok(mut v) = self.idle.lock()
            && v.len() < KEPT
        {
            v.push(gp);
        }
        out
    }

    /// The shape of `table`, read once.
    pub(crate) fn shape(&self, table: &str) -> Result<Shape, DataError> {
        if let Some(s) = self.shapes.read().ok().and_then(|m| m.get(table).cloned()) {
            return Ok(s);
        }
        let layer = self.with(|gp| {
            gp.layers()
                .map_err(failed)?
                .into_iter()
                .find(|l| l.name == table)
                .ok_or_else(|| DataError::NoTable(table.to_owned()))
        })?;
        let shape = Shape {
            schema: Schema {
                key: layer.fid.clone(),
                // An INTEGER PRIMARY KEY is the row's id: SQLite gives a new row the next one.
                key_made: true,
                fields: fields_of(&layer),
                geometry: geometry_of(&layer),
            },
            layer,
        };
        if let Ok(mut m) = self.shapes.write() {
            m.insert(table.to_owned(), shape.clone());
        }
        Ok(shape)
    }

    /// Forgets the shapes read (a transaction changed tables).
    pub(crate) fn forget_shapes(&self) {
        if let Ok(mut m) = self.shapes.write() {
            m.clear();
        }
    }

    pub(crate) fn writable(&self) -> bool {
        std::fs::metadata(&self.path).is_ok_and(|m| !m.permissions().readonly())
    }
}

fn table_kind(layer: &Layer) -> Option<TableKind> {
    match layer.data_type {
        DataType::Features => Some(TableKind::Features),
        DataType::Attributes => Some(TableKind::Attributes),
        _ => None,
    }
}

impl Source for GpkgSource {
    fn driver(&self) -> &'static str {
        "gpkg"
    }

    fn address(&self) -> &str {
        &self.address
    }

    fn tables(&self) -> Result<Vec<TableInfo>, DataError> {
        let writable = self.writable();
        self.with(|gp| {
            let layers = gp.layers().map_err(failed)?;
            let mut out = Vec::new();
            for l in &layers {
                let Some(kind) = table_kind(l) else {
                    continue;
                };
                let rows = gp.count(l).ok();
                out.push(TableInfo {
                    name: l.name.clone(),
                    title: l.identifier.clone().filter(|i| i != &l.name),
                    kind,
                    geometry: geometry_of(l),
                    rows,
                    extent: l.bounds,
                    writable: writable && l.fid != "rowid",
                    comment: l.description.clone().filter(|d| !d.is_empty()),
                });
            }
            Ok(out)
        })
    }

    fn schema(&self, table: &str) -> Result<Schema, DataError> {
        Ok(self.shape(table)?.schema)
    }

    fn capabilities(&self, table: &str) -> Result<Capabilities, DataError> {
        let s = self.shape(table)?;
        let w = self.writable() && s.layer.fid != "rowid";
        Ok(Capabilities {
            read: true,
            insert: w,
            update_values: w,
            update_geometry: w && s.schema.geometry.is_some(),
            delete: w,
            transactions: w,
            spatial_index: s.layer.indexed,
            simplify_at_source: false,
            query_at_source: true,
            curves: true,
            create_table: self.writable(),
        })
    }

    fn extent(&self, table: &str, estimate: bool) -> Result<Option<[f64; 4]>, DataError> {
        let s = self.shape(table)?;
        if estimate && s.layer.bounds.is_some() {
            return Ok(s.layer.bounds);
        }
        let Some(g) = &s.schema.geometry else {
            return Ok(None);
        };
        let sql = if s.layer.indexed {
            format!(
                "SELECT min(minx), min(miny), max(maxx), max(maxy) FROM {}",
                quote(&format!("rtree_{table}_{}", g.name))
            )
        } else {
            let c = quote(&g.name);
            format!(
                "SELECT min(ST_MinX({c})), min(ST_MinY({c})), max(ST_MaxX({c})), max(ST_MaxY({c})) FROM {}",
                quote(table)
            )
        };
        self.with(|gp| {
            let b: [Option<f64>; 4] = gp
                .connection()
                .query_row(&sql, [], |r| {
                    Ok([r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?])
                })
                .map_err(failed)?;
            Ok(match b {
                [Some(a), Some(b), Some(c), Some(d)] => Some([a, b, c, d]),
                _ => None,
            })
        })
    }

    fn count(&self, table: &str, request: &Request, _estimate: bool) -> Result<u64, DataError> {
        let s = self.shape(table)?;
        let w = where_of(request, &s.schema, table, s.layer.indexed)?;
        let sql = format!("SELECT count(*) FROM {} t{}", quote(table), w.text());
        self.with(|gp| {
            let binds: Vec<Bind<'_>> = w.args.iter().map(Bind).collect();
            gp.connection()
                .query_row(&sql, rusqlite::params_from_iter(binds.iter()), |r| {
                    r.get::<_, i64>(0)
                })
                .map(|n| n.max(0) as u64)
                .map_err(failed)
        })
    }

    fn features(
        &self,
        table: &str,
        request: &Request,
        cancel: &Cancel,
        sink: &mut dyn FnMut(Vec<Feature>) -> bool,
    ) -> Result<Read, DataError> {
        let s = self.shape(table)?;
        let names: Vec<String> = match &request.columns {
            None => s.schema.fields.iter().map(|f| f.name.clone()).collect(),
            Some(list) => list.clone(),
        };
        let mut kinds = Vec::with_capacity(names.len());
        for n in &names {
            let f = s.schema.field(n).ok_or_else(|| DataError::NoField {
                table: table.to_owned(),
                field: n.clone(),
            })?;
            kinds.push(f.kind.clone());
        }
        let mut select = vec![format!("t.{}", quote(&s.schema.key))];
        let geometry = request.geometry && s.schema.geometry.is_some();
        match (&s.schema.geometry, geometry) {
            (Some(g), true) => select.push(format!("t.{}", quote(&g.name))),
            _ => select.push("NULL".into()),
        }
        select.extend(names.iter().map(|n| format!("t.{}", quote(n))));
        let w = where_of(request, &s.schema, table, s.layer.indexed)?;
        let sql = format!(
            "SELECT {} FROM {} t{}{}",
            select.join(", "),
            quote(table),
            w.text(),
            order_and_page(request, &s.schema, table)?
        );
        let chunk = request.chunk.max(1);
        let tolerance = request.simplify.filter(|t| *t > 0.0);
        let exact = request.exact.then_some(request.bbox).flatten();
        self.with(|gp| {
            let mut st = gp.connection().prepare(&sql).map_err(failed)?;
            let binds: Vec<Bind<'_>> = w.args.iter().map(Bind).collect();
            let mut rows = st
                .query(rusqlite::params_from_iter(binds.iter()))
                .map_err(failed)?;
            let mut part: Vec<Feature> = Vec::with_capacity(chunk);
            let mut read = Read::default();
            loop {
                if cancel.is_cancelled() {
                    read.stopped = true;
                    break;
                }
                let Some(r) = rows.next().map_err(failed)? else {
                    break;
                };
                let fid: i64 = r.get(0).map_err(failed)?;
                let geometry = if geometry {
                    match r.get_ref(1).map_err(failed)? {
                        rusqlite::types::ValueRef::Blob(b) => Some(
                            read_blob(b)
                                .map(|blob| match tolerance {
                                    Some(t) => simplify(&blob.geometry, t),
                                    None => blob.geometry,
                                })
                                .map_err(|e| e.to_string()),
                        ),
                        _ => None,
                    }
                } else {
                    None
                };
                if let (Some(b), Some(Ok(g))) = (exact, &geometry)
                    && !kentos_data::request::meets(g.bounds().unwrap_or([f64::NAN; 4]), b)
                {
                    continue;
                }
                let mut values = Vec::with_capacity(kinds.len());
                for (i, k) in kinds.iter().enumerate() {
                    values.push(value_of(r.get_ref(i + 2).map_err(failed)?, k));
                }
                part.push(Feature {
                    fid,
                    geometry,
                    values,
                });
                if part.len() == chunk {
                    read.rows += part.len() as u64;
                    let go = sink(std::mem::replace(&mut part, Vec::with_capacity(chunk)));
                    if !go || cancel.is_cancelled() {
                        read.stopped = true;
                        return Ok(read);
                    }
                }
            }
            if !part.is_empty() && !read.stopped {
                read.rows += part.len() as u64;
                if !sink(part) {
                    read.stopped = true;
                }
            }
            Ok(read)
        })
    }

    fn unique(
        &self,
        table: &str,
        field: &str,
        condition: Option<&Condition>,
        limit: usize,
    ) -> Result<Vec<Value>, DataError> {
        let s = self.shape(table)?;
        let kind = s
            .schema
            .field(field)
            .map(|f| f.kind.clone())
            .ok_or_else(|| DataError::NoField {
                table: table.to_owned(),
                field: field.to_owned(),
            })?;
        let request = Request {
            condition: Some(match condition {
                Some(c) => Condition::And(vec![c.clone(), Condition::NotNull(field.to_owned())]),
                None => Condition::NotNull(field.to_owned()),
            }),
            ..Request::default()
        };
        let w = where_of(&request, &s.schema, table, s.layer.indexed)?;
        let col = format!("t.{}", quote(field));
        let sql = format!(
            "SELECT DISTINCT {col} FROM {} t{} ORDER BY {col} LIMIT {limit}",
            quote(table),
            w.text()
        );
        self.with(|gp| {
            let mut st = gp.connection().prepare(&sql).map_err(failed)?;
            let binds: Vec<Bind<'_>> = w.args.iter().map(Bind).collect();
            let mut rows = st
                .query(rusqlite::params_from_iter(binds.iter()))
                .map_err(failed)?;
            let mut out = Vec::new();
            while let Some(r) = rows.next().map_err(failed)? {
                out.push(value_of(r.get_ref(0).map_err(failed)?, &kind));
            }
            Ok(out)
        })
    }

    fn begin(&self) -> Result<Box<dyn Transaction>, DataError> {
        if !self.writable() {
            return Err(DataError::Denied(format!(
                "“{}” salt okunur; dosyanın izinlerini denetleyin.",
                self.path.display()
            )));
        }
        // A transaction changes tables: the shapes read before are read again afterwards.
        self.forget_shapes();
        Ok(Box::new(GpkgTransaction::begin(self)?))
    }

    fn project_store(&self) -> Option<&dyn kentos_data::ProjectStore> {
        Some(self)
    }
}
