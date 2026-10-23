//! GeoPackage (OGC 12-128r19, docs/adr/0502, plan/gis.md CBS-03) over SQLite: a file's feature and attribute tables
//! with their columns, spatial reference systems and extensions; rows read a page at a time (by the spatial index's
//! box when it has one, then by fid); tables written in one transaction with the spatial index (`gpkg_rtree_index`,
//! its GeoPackage 1.3 triggers and the `ST_*` functions they call) and KentOS's lasting ids per fid (the `kentos_uids`
//! extension, which other programs leave alone). It knows nothing of a drawing: the product commands turn rows into
//! objects.
//!
//! A file written here is GeoPackage 1.3 (`application_id` GPKG, `user_version` 10300). Unknown extensions and tables
//! are not touched. SQLite is compiled in (rusqlite's `bundled`).

pub mod fgb;
pub mod geom;
pub mod relations;
pub mod tiles;
pub mod wkt;

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use rusqlite::functions::FunctionFlags;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, params};

pub use geom::{
    Blob, BlobError, Coord, Curve, Geometry, read_blob, read_wkb, write_blob, write_wkb,
};

/// Why the GeoPackage could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        Error(format!("SQLite: {e}"))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// `application_id`: “GPKG”.
pub const APPLICATION_ID: i32 = 0x4750_4B47;
/// `user_version` of the files written here: GeoPackage 1.3.0.
pub const USER_VERSION: i32 = 10300;
/// The extension that keeps KentOS's lasting id of each row.
pub const UIDS_EXTENSION: &str = "kentos_uids";

/// What a table of `gpkg_contents` holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataType {
    Features,
    Attributes,
    Tiles,
    Other(String),
}

impl DataType {
    fn read(s: &str) -> DataType {
        match s {
            "features" => DataType::Features,
            "attributes" => DataType::Attributes,
            "tiles" => DataType::Tiles,
            o => DataType::Other(o.to_owned()),
        }
    }

    fn key(&self) -> &str {
        match self {
            DataType::Features => "features",
            DataType::Attributes => "attributes",
            DataType::Tiles => "tiles",
            DataType::Other(o) => o,
        }
    }
}

/// A column's GeoPackage type (§1.1.1.1.3, Table 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnKind {
    Boolean,
    /// TINYINT, SMALLINT, MEDIUMINT, INT, INTEGER: a 64-bit integer at most.
    Integer,
    /// FLOAT, DOUBLE, REAL.
    Real,
    /// TEXT, with its largest length when it has one.
    Text(Option<u32>),
    Blob(Option<u32>),
    Date,
    DateTime,
    /// Anything else, as declared.
    Other(String),
}

impl ColumnKind {
    pub fn read(decl: &str) -> ColumnKind {
        let up = decl.trim().to_ascii_uppercase();
        let (base, len) = match up.split_once('(') {
            Some((b, rest)) => (
                b.trim().to_owned(),
                rest.trim_end_matches(')').trim().parse::<u32>().ok(),
            ),
            None => (up.clone(), None),
        };
        match base.as_str() {
            "BOOLEAN" => ColumnKind::Boolean,
            "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" => ColumnKind::Integer,
            "FLOAT" | "DOUBLE" | "REAL" => ColumnKind::Real,
            "TEXT" => ColumnKind::Text(len),
            "BLOB" => ColumnKind::Blob(len),
            "DATE" => ColumnKind::Date,
            "DATETIME" => ColumnKind::DateTime,
            _ => ColumnKind::Other(decl.trim().to_owned()),
        }
    }

    pub fn decl(&self) -> String {
        match self {
            ColumnKind::Boolean => "BOOLEAN".into(),
            ColumnKind::Integer => "INTEGER".into(),
            ColumnKind::Real => "DOUBLE".into(),
            ColumnKind::Text(None) => "TEXT".into(),
            ColumnKind::Text(Some(n)) => format!("TEXT({n})"),
            ColumnKind::Blob(None) => "BLOB".into(),
            ColumnKind::Blob(Some(n)) => format!("BLOB({n})"),
            ColumnKind::Date => "DATE".into(),
            ColumnKind::DateTime => "DATETIME".into(),
            ColumnKind::Other(s) => s.clone(),
        }
    }
}

/// A table's column other than its fid and geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    /// Its type as declared (`MEDIUMINT`, `TEXT(40)` …), written again as it was; empty: `kind`'s.
    pub decl: String,
    pub kind: ColumnKind,
    pub not_null: bool,
    /// Its default as SQL text.
    pub default: Option<String>,
}

/// A geometry column (`gpkg_geometry_columns`): its type name and whether Z and M are prohibited (0), mandatory (1)
/// or optional (2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryColumn {
    pub name: String,
    pub type_name: String,
    pub z: u8,
    pub m: u8,
}

/// A table of the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    pub data_type: DataType,
    pub identifier: Option<String>,
    pub description: Option<String>,
    /// `gpkg_contents`'s box: min x, min y, max x, max y.
    pub bounds: Option<[f64; 4]>,
    pub srs_id: Option<i32>,
    pub geometry: Option<GeometryColumn>,
    /// The integer primary key.
    pub fid: String,
    pub columns: Vec<Column>,
    /// Whether it has a spatial index.
    pub indexed: bool,
}

/// A spatial reference system (`gpkg_spatial_ref_sys`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Srs {
    pub name: String,
    pub id: i32,
    pub organization: String,
    pub organization_id: i32,
    /// Its WKT (WKT 1, as the core table holds it), or `undefined`.
    pub definition: String,
    pub description: Option<String>,
}

/// An extension (`gpkg_extensions`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    pub table: Option<String>,
    pub column: Option<String>,
    pub name: String,
    pub definition: String,
    pub scope: String,
}

/// A cell's value as SQLite holds it.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Value {
    fn of(v: ValueRef<'_>) -> Value {
        match v {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(i) => Value::Integer(i),
            ValueRef::Real(r) => Value::Real(r),
            ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into_owned()),
            ValueRef::Blob(b) => Value::Blob(b.to_vec()),
        }
    }
}

impl rusqlite::ToSql for Value {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        use rusqlite::types::{ToSqlOutput, ValueRef as R};
        Ok(ToSqlOutput::Borrowed(match self {
            Value::Null => R::Null,
            Value::Integer(i) => R::Integer(*i),
            Value::Real(r) => R::Real(*r),
            Value::Text(t) => R::Text(t.as_bytes()),
            Value::Blob(b) => R::Blob(b),
        }))
    }
}

/// A row read: its fid, its geometry (`None` for SQL NULL; an unreadable blob is an error of the row), and its
/// columns' values in the layer's order.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub fid: i64,
    pub geometry: Option<std::result::Result<Geometry, BlobError>>,
    pub values: Vec<Value>,
}

/// Which rows to read: those whose box meets `bbox` (by the spatial index; without one, every row), after `after`
/// in fid order, at most `limit`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    pub bbox: Option<[f64; 4]>,
    pub after: Option<i64>,
    pub limit: Option<usize>,
}

/// A table read a part at a time (`GeoPackage::scan`, docs/adr/0581 §2): rows whose box meets `bbox`, of which
/// `filter`'s SQL condition holds (its `?` bound to its values, its columns named `t."name"`), after `after` in fid
/// order; their values are `columns`' (indices into the layer's columns; none: all), their geometry read when
/// `geometry`. At most `chunk` rows a part.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scan<'a> {
    pub bbox: Option<[f64; 4]>,
    pub columns: Option<&'a [usize]>,
    pub geometry: bool,
    pub filter: Option<(&'a str, &'a [Value])>,
    pub after: Option<i64>,
    pub chunk: usize,
    /// Another order than the key's; `after` is then unused.
    pub order: Option<ScanOrder>,
}

/// A scan's order by a column (docs/adr/0591 §1): `column` an index into the layer's columns (one of those read),
/// which way; a row without a value after every row with one; equal ones by key. `after` is where the previous page
/// ended: the column's value there and the row's key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScanOrder {
    pub column: usize,
    pub descending: bool,
    pub after: Option<(Value, i64)>,
}

/// The condition that keeps the rows after `(value, key)` in an order by `column` (NULLs last, keys rising), and its
/// values: the keyset of the next part.
pub fn keyset(column: &str, key: &str, descending: bool, after: &Option<(Value, i64)>) -> (String, Vec<Value>) {
    match after {
        None => (String::new(), Vec::new()),
        Some((Value::Null, k)) => (format!("({column} IS NULL AND {key} > ?)"), vec![Value::Integer(*k)]),
        Some((v, k)) => {
            let past = if descending { "<" } else { ">" };
            (
                format!("({column} {past} ? OR ({column} = ? AND {key} > ?) OR {column} IS NULL)"),
                vec![v.clone(), v.clone(), Value::Integer(*k)],
            )
        }
    }
}

/// A table to write.
#[derive(Clone, Debug, PartialEq)]
pub struct TableDef {
    pub name: String,
    pub data_type: DataType,
    pub identifier: Option<String>,
    pub description: Option<String>,
    pub srs_id: Option<i32>,
    pub geometry: Option<GeometryColumn>,
    pub columns: Vec<Column>,
}

/// A row to write: its fid (none: the next), geometry and values in the table's column order.
#[derive(Clone, Debug, PartialEq)]
pub struct NewRow {
    pub fid: Option<i64>,
    pub geometry: Option<Geometry>,
    pub values: Vec<Value>,
}

/// The edits of a table: columns to add, rows to update by fid, rows to insert with their lasting ids, fids to delete.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edits {
    pub add_columns: Vec<Column>,
    pub update: Vec<(i64, NewRow)>,
    pub insert: Vec<(String, NewRow)>,
    pub delete: Vec<i64>,
}

/// A GeoPackage file open.
pub struct GeoPackage {
    conn: Connection,
}

/// `name` as an SQL identifier.
pub fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn bounds_of(v: ValueRef<'_>) -> Option<[f64; 4]> {
    match v {
        ValueRef::Blob(b) => read_blob(b).ok().and_then(|g| g.geometry.bounds()),
        _ => None,
    }
}

/// The `ST_*` functions the spatial index's triggers call (§F.3), over this crate's blob reader.
fn register(conn: &Connection) -> Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC;
    for (name, i) in [
        ("ST_MinX", 0usize),
        ("ST_MinY", 1),
        ("ST_MaxX", 2),
        ("ST_MaxY", 3),
    ] {
        conn.create_scalar_function(name, 1, flags, move |ctx| {
            Ok(bounds_of(ctx.get_raw(0)).map(|b| b[i]))
        })?;
    }
    conn.create_scalar_function("ST_IsEmpty", 1, flags, |ctx| {
        Ok(match ctx.get_raw(0) {
            ValueRef::Blob(b) => read_blob(b).map(|g| g.geometry.is_empty() as i64).ok(),
            _ => None,
        })
    })?;
    Ok(())
}

const CORE: &str = "
CREATE TABLE gpkg_spatial_ref_sys (
  srs_name TEXT NOT NULL, srs_id INTEGER NOT NULL PRIMARY KEY, organization TEXT NOT NULL,
  organization_coordsys_id INTEGER NOT NULL, definition TEXT NOT NULL, description TEXT);
CREATE TABLE gpkg_contents (
  table_name TEXT NOT NULL PRIMARY KEY, data_type TEXT NOT NULL, identifier TEXT UNIQUE, description TEXT DEFAULT '',
  last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER,
  CONSTRAINT fk_gc_r_srs_id FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys(srs_id));
CREATE TABLE gpkg_geometry_columns (
  table_name TEXT NOT NULL, column_name TEXT NOT NULL, geometry_type_name TEXT NOT NULL, srs_id INTEGER NOT NULL,
  z TINYINT NOT NULL, m TINYINT NOT NULL,
  CONSTRAINT pk_geom_cols PRIMARY KEY (table_name, column_name),
  CONSTRAINT uk_gc_table_name UNIQUE (table_name),
  CONSTRAINT fk_gc_tn FOREIGN KEY (table_name) REFERENCES gpkg_contents(table_name),
  CONSTRAINT fk_gc_srs FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys (srs_id));
CREATE TABLE gpkg_extensions (
  table_name TEXT, column_name TEXT, extension_name TEXT NOT NULL, definition TEXT NOT NULL, scope TEXT NOT NULL,
  CONSTRAINT ge_tce UNIQUE (table_name, column_name, extension_name));
CREATE TABLE gpkg_tile_matrix_set (
  table_name TEXT NOT NULL PRIMARY KEY, srs_id INTEGER NOT NULL, min_x DOUBLE NOT NULL, min_y DOUBLE NOT NULL,
  max_x DOUBLE NOT NULL, max_y DOUBLE NOT NULL,
  CONSTRAINT fk_gtms_table_name FOREIGN KEY (table_name) REFERENCES gpkg_contents(table_name),
  CONSTRAINT fk_gtms_srs FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys (srs_id));
CREATE TABLE gpkg_tile_matrix (
  table_name TEXT NOT NULL, zoom_level INTEGER NOT NULL, matrix_width INTEGER NOT NULL, matrix_height INTEGER NOT NULL,
  tile_width INTEGER NOT NULL, tile_height INTEGER NOT NULL, pixel_x_size DOUBLE NOT NULL, pixel_y_size DOUBLE NOT NULL,
  CONSTRAINT pk_ttm PRIMARY KEY (table_name, zoom_level),
  CONSTRAINT fk_tmm_table_name FOREIGN KEY (table_name) REFERENCES gpkg_contents(table_name));
INSERT INTO gpkg_spatial_ref_sys VALUES
  ('Undefined cartesian SRS', -1, 'NONE', -1, 'undefined', 'undefined cartesian coordinate reference system'),
  ('Undefined geographic SRS', 0, 'NONE', 0, 'undefined', 'undefined geographic coordinate reference system'),
  ('WGS 84 geodetic', 4326, 'EPSG', 4326,
   'GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563,AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY[\"EPSG\",\"6326\"]],PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\",\"9122\"]],AXIS[\"Latitude\",NORTH],AXIS[\"Longitude\",EAST],AUTHORITY[\"EPSG\",\"4326\"]]',
   'longitude/latitude coordinates in decimal degrees on the WGS 84 spheroid');
";

impl GeoPackage {
    /// Opens a file to read, or to read and write.
    pub fn open(path: &Path, write: bool) -> Result<GeoPackage> {
        let flags = if write {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)
            .map_err(|e| Error(format!("“{}” açılamadı: {e}", path.display())))?;
        let gp = GeoPackage { conn };
        let app: i32 = gp
            .conn
            .query_row("PRAGMA application_id", [], |r| r.get(0))
            .map_err(|_| Error(format!("“{}” bir SQLite dosyası değil", path.display())))?;
        let has_contents: i64 = gp.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'gpkg_contents'",
            [],
            |r| r.get(0),
        )?;
        // GeoPackage 1.0 was “GP10”; both, and a file without the id that has the core tables, are read.
        if app != APPLICATION_ID && app != 0x4750_3130 && has_contents == 0 {
            return Err(Error(format!(
                "“{}” bir GeoPackage değil: gpkg_contents tablosu ve GPKG kimliği yok",
                path.display()
            )));
        }
        register(&gp.conn)?;
        Ok(gp)
    }

    /// Makes a new GeoPackage; the file must not exist.
    pub fn create(path: &Path) -> Result<GeoPackage> {
        if path.exists() {
            return Err(Error(format!("“{}” zaten var", path.display())));
        }
        let conn = Connection::open(path)
            .map_err(|e| Error(format!("“{}” yazılamadı: {e}", path.display())))?;
        conn.execute_batch(&format!(
            "PRAGMA application_id = {APPLICATION_ID}; PRAGMA user_version = {USER_VERSION}; BEGIN; {CORE} COMMIT;"
        ))?;
        register(&conn)?;
        Ok(GeoPackage { conn })
    }

    /// The tables of `gpkg_contents`, in its order, with their columns.
    pub fn layers(&self) -> Result<Vec<Layer>> {
        let mut st = self.conn.prepare(
            "SELECT table_name, data_type, identifier, description, min_x, min_y, max_x, max_y, srs_id
             FROM gpkg_contents ORDER BY rowid",
        )?;
        let base: Vec<_> = st
            .query_map([], |r| {
                let b: [Option<f64>; 4] = [r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?];
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    match b {
                        [Some(a), Some(b), Some(c), Some(d)] => Some([a, b, c, d]),
                        _ => None,
                    },
                    r.get::<_, Option<i32>>(8)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = Vec::with_capacity(base.len());
        for (name, data_type, identifier, description, bounds, srs_id) in base {
            let geometry = self
                .conn
                .query_row(
                    "SELECT column_name, geometry_type_name, z, m FROM gpkg_geometry_columns WHERE table_name = ?1",
                    [&name],
                    |r| {
                        Ok(GeometryColumn {
                            name: r.get(0)?,
                            type_name: r.get(1)?,
                            z: r.get(2)?,
                            m: r.get(3)?,
                        })
                    },
                )
                .ok();
            let mut fid = String::new();
            let mut columns = Vec::new();
            let mut info = self
                .conn
                .prepare(&format!("PRAGMA table_info({})", quote(&name)))?;
            let cols: Vec<(String, String, bool, Option<String>, i64)> = info
                .query_map([], |r| {
                    Ok((
                        r.get(1)?,
                        r.get(2)?,
                        r.get::<_, i64>(3)? != 0,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                })?
                .collect::<rusqlite::Result<_>>()?;
            for (col, decl, not_null, default, pk) in cols {
                if pk == 1 && fid.is_empty() && ColumnKind::read(&decl) == ColumnKind::Integer {
                    fid = col;
                } else if geometry
                    .as_ref()
                    .is_some_and(|g| g.name.eq_ignore_ascii_case(&col))
                {
                } else {
                    columns.push(Column {
                        name: col,
                        kind: ColumnKind::read(&decl),
                        decl,
                        not_null,
                        default,
                    });
                }
            }
            if fid.is_empty() {
                fid = "rowid".into();
            }
            let indexed = match &geometry {
                Some(g) => self.has_index(&name, &g.name)?,
                None => false,
            };
            out.push(Layer {
                name,
                data_type: DataType::read(&data_type),
                identifier,
                description,
                bounds,
                srs_id,
                geometry,
                fid,
                columns,
                indexed,
            });
        }
        Ok(out)
    }

    fn has_index(&self, table: &str, column: &str) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [format!("rtree_{table}_{column}")],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// A spatial reference system by its id.
    pub fn srs(&self, id: i32) -> Result<Option<Srs>> {
        read_srs(&self.conn, id)
    }

    /// The file's extensions.
    pub fn extensions(&self) -> Result<Vec<Extension>> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'gpkg_extensions'",
            [],
            |r| r.get(0),
        )?;
        if n == 0 {
            return Ok(Vec::new());
        }
        let mut st = self.conn.prepare(
            "SELECT table_name, column_name, extension_name, definition, scope FROM gpkg_extensions ORDER BY rowid",
        )?;
        let rows = st
            .query_map([], |r| {
                Ok(Extension {
                    table: r.get(0)?,
                    column: r.get(1)?,
                    name: r.get(2)?,
                    definition: r.get(3)?,
                    scope: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// How many rows the layer has.
    pub fn count(&self, layer: &Layer) -> Result<u64> {
        let n: i64 = self.conn.query_row(
            &format!("SELECT count(*) FROM {}", quote(&layer.name)),
            [],
            |r| r.get(0),
        )?;
        Ok(n.max(0) as u64)
    }

    /// Every row's fid, in order.
    pub fn fids(&self, layer: &Layer) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {} FROM {} ORDER BY 1",
            quote(&layer.fid),
            quote(&layer.name)
        ))?;
        let rows = st
            .query_map([], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// The rows `q` asks for, in fid order. A box takes the rows whose geometry's box meets it (sides included): through
    /// the spatial index, or without one by each geometry's own box; a row without geometry is not in a box.
    pub fn rows(&self, layer: &Layer, q: &Query) -> Result<Vec<Row>> {
        let t = quote(&layer.name);
        let fid = quote(&layer.fid);
        let mut cols = vec![format!("t.{fid}")];
        let geom = layer.geometry.as_ref().map(|g| quote(&g.name));
        cols.push(geom.as_ref().map_or("NULL".into(), |g| format!("t.{g}")));
        cols.extend(
            layer
                .columns
                .iter()
                .map(|c| format!("t.{}", quote(&c.name))),
        );
        let mut sql = format!("SELECT {} FROM {t} AS t", cols.join(", "));
        let mut wh = Vec::new();
        let mut args: Vec<Value> = Vec::new();
        if let (Some(b), Some(g)) = (q.bbox, &layer.geometry)
            && layer.indexed
        {
            sql.push_str(&format!(
                " JOIN {} AS r ON r.id = t.{fid}",
                quote(&format!("rtree_{}_{}", layer.name, g.name))
            ));
            wh.push("r.maxx >= ? AND r.minx <= ? AND r.maxy >= ? AND r.miny <= ?".to_owned());
            args.extend([
                Value::Real(b[0]),
                Value::Real(b[2]),
                Value::Real(b[1]),
                Value::Real(b[3]),
            ]);
        }
        if let Some(after) = q.after {
            wh.push(format!("t.{fid} > ?"));
            args.push(Value::Integer(after));
        }
        if !wh.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&wh.join(" AND "));
        }
        sql.push_str(&format!(" ORDER BY t.{fid}"));
        if let Some(n) = q.limit {
            sql.push_str(&format!(" LIMIT {n}"));
        }
        let mut st = self.conn.prepare(&sql)?;
        let n = layer.columns.len();
        let rows = st
            .query_map(rusqlite::params_from_iter(args.iter()), |r| {
                let geometry = match r.get_ref(1)? {
                    ValueRef::Null => None,
                    ValueRef::Blob(b) => Some(read_blob(b).map(|g| g.geometry)),
                    _ => Some(Err(BlobError(
                        "geometri sütununda geometri olmayan değer".into(),
                    ))),
                };
                let mut values = Vec::with_capacity(n);
                for i in 0..n {
                    values.push(Value::of(r.get_ref(2 + i)?));
                }
                Ok(Row {
                    fid: r.get(0)?,
                    geometry,
                    values,
                })
            })?
            .collect::<rusqlite::Result<Vec<Row>>>()?;
        // Without a spatial index the box is each geometry's own.
        if let Some(b) = q.bbox
            && !(layer.indexed && layer.geometry.is_some())
        {
            return Ok(rows
                .into_iter()
                .filter(|r| match &r.geometry {
                    Some(Ok(g)) => g.bounds().is_some_and(|g| {
                        g[2] >= b[0] && g[0] <= b[2] && g[3] >= b[1] && g[1] <= b[3]
                    }),
                    _ => false,
                })
                .collect());
        }
        Ok(rows)
    }

    /// The rows `s` asks for, handed to `f` a part at a time in fid order (docs/adr/0581 §2): each part one query of
    /// at most `s.chunk` rows after the last fid read, so memory stays with the part. A box is as in [`rows`]: the
    /// spatial index's, or each geometry's own box when the table has none. `f` returns false to stop. The rows'
    /// values are the columns `s.columns` names, in that order. Returns how many rows the file handed over (before the
    /// box of a table without an index).
    ///
    /// [`rows`]: GeoPackage::rows
    pub fn scan(
        &self,
        layer: &Layer,
        s: &Scan<'_>,
        f: &mut dyn FnMut(Vec<Row>) -> bool,
    ) -> Result<u64> {
        let t = quote(&layer.name);
        let fid = quote(&layer.fid);
        let all: Vec<usize> = (0..layer.columns.len()).collect();
        let columns = s.columns.unwrap_or(&all);
        if let Some(&i) = columns.iter().find(|&&i| i >= layer.columns.len()) {
            return Err(Error(format!("{i}. sütun tabloda yok")));
        }
        let geom = layer.geometry.as_ref().map(|g| quote(&g.name));
        let indexed_box = s.bbox.is_some() && layer.indexed && layer.geometry.is_some();
        // Without an index the box needs each geometry, read or not.
        let read_geometry = s.geometry || (s.bbox.is_some() && !indexed_box);
        let mut cols = vec![format!("t.{fid}")];
        cols.push(match (&geom, read_geometry) {
            (Some(g), true) => format!("t.{g}"),
            _ => "NULL".into(),
        });
        cols.extend(
            columns
                .iter()
                .map(|&i| format!("t.{}", quote(&layer.columns[i].name))),
        );
        let mut sql = format!("SELECT {} FROM {t} AS t", cols.join(", "));
        let mut wh = Vec::new();
        let mut args: Vec<Value> = Vec::new();
        if let (true, Some(b), Some(g)) = (indexed_box, s.bbox, &layer.geometry) {
            sql.push_str(&format!(
                " JOIN {} AS r ON r.id = t.{fid}",
                quote(&format!("rtree_{}_{}", layer.name, g.name))
            ));
            wh.push("r.maxx >= ? AND r.minx <= ? AND r.maxy >= ? AND r.miny <= ?".to_owned());
            args.extend([
                Value::Real(b[0]),
                Value::Real(b[2]),
                Value::Real(b[1]),
                Value::Real(b[3]),
            ]);
        }
        if let Some((cond, values)) = s.filter
            && !cond.is_empty()
        {
            wh.push(format!("({cond})"));
            args.extend(values.iter().cloned());
        }
        // The order's column among those read, and where each part starts: the key, or the column's value and the key.
        let ordered = match &s.order {
            None => None,
            Some(o) => {
                let at = columns
                    .iter()
                    .position(|&i| i == o.column)
                    .ok_or_else(|| Error(format!("{}. sütun sıralanıyor ama okunmuyor", o.column)))?;
                Some((at, quote(&layer.columns[o.column].name), o.descending))
            }
        };
        let base = sql.clone();
        let base_where = wh.clone();
        let make = |cond: &str| -> String {
            let mut sql = base.clone();
            let mut wh = base_where.clone();
            if !cond.is_empty() {
                wh.push(cond.to_owned());
            }
            if !wh.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&wh.join(" AND "));
            }
            match &ordered {
                None => sql.push_str(&format!(" ORDER BY t.{fid} LIMIT {}", s.chunk.max(1))),
                Some((_, c, d)) => sql.push_str(&format!(
                    " ORDER BY t.{c} {} NULLS LAST, t.{fid} LIMIT {}",
                    if *d { "DESC" } else { "ASC" },
                    s.chunk.max(1)
                )),
            }
            sql
        };
        let n = columns.len();
        let mut after = s.after.unwrap_or(i64::MIN);
        let mut place: Option<(Value, i64)> = s.order.as_ref().and_then(|o| o.after.clone());
        let mut handed = 0u64;
        loop {
            let mut bound = args.clone();
            let sql = match &ordered {
                None => {
                    bound.push(Value::Integer(after));
                    make(&format!("t.{fid} > ?"))
                }
                Some((_, c, d)) => {
                    let (cond, values) = keyset(&format!("t.{c}"), &format!("t.{fid}"), *d, &place);
                    bound.extend(values);
                    make(&cond)
                }
            };
            let mut st = self.conn.prepare_cached(&sql)?;
            let rows = st
                .query_map(rusqlite::params_from_iter(bound.iter()), |r| {
                    let geometry = match r.get_ref(1)? {
                        ValueRef::Null => None,
                        ValueRef::Blob(b) => Some(read_blob(b).map(|g| g.geometry)),
                        _ => Some(Err(BlobError(
                            "geometri sütununda geometri olmayan değer".into(),
                        ))),
                    };
                    let mut values = Vec::with_capacity(n);
                    for i in 0..n {
                        values.push(Value::of(r.get_ref(2 + i)?));
                    }
                    Ok(Row {
                        fid: r.get(0)?,
                        geometry,
                        values,
                    })
                })?
                .collect::<rusqlite::Result<Vec<Row>>>()?;
            let Some(last) = rows.last().map(|r| r.fid) else {
                return Ok(handed);
            };
            let full = rows.len() >= s.chunk.max(1);
            after = last;
            if let (Some((at, ..)), Some(r)) = (&ordered, rows.last()) {
                place = Some((r.values[*at].clone(), r.fid));
            }
            handed += rows.len() as u64;
            let mut rows = rows;
            if let Some(b) = s.bbox
                && !indexed_box
            {
                rows.retain(|r| match &r.geometry {
                    Some(Ok(g)) => g.bounds().is_some_and(|g| {
                        g[2] >= b[0] && g[0] <= b[2] && g[3] >= b[1] && g[1] <= b[3]
                    }),
                    _ => false,
                });
            }
            if !s.geometry {
                for r in &mut rows {
                    r.geometry = None;
                }
            }
            if !rows.is_empty() && !f(rows) {
                return Ok(handed);
            }
            if !full {
                return Ok(handed);
            }
        }
    }

    /// Whether the file has KentOS's lasting ids (the `kentos_uids` extension).
    fn has_uids(&self) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'kentos_uids'",
            [],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// The lasting ids of `fids` only, by fid, looked up through the extension's key a few hundred at a time
    /// (docs/adr/0597: a part's rows read without the table's every id).
    pub fn uids_of(&self, table: &str, fids: &[i64]) -> Result<HashMap<i64, String>> {
        let mut out = HashMap::new();
        if fids.is_empty() || !self.has_uids()? {
            return Ok(out);
        }
        for chunk in fids.chunks(500) {
            let marks = vec!["?"; chunk.len()].join(", ");
            let mut st = self.conn.prepare(&format!(
                "SELECT fid, uid FROM kentos_uids WHERE table_name = ? AND fid IN ({marks})"
            ))?;
            let mut values: Vec<rusqlite::types::Value> = vec![table.to_owned().into()];
            values.extend(chunk.iter().map(|f| rusqlite::types::Value::from(*f)));
            let rows = st.query_map(rusqlite::params_from_iter(values), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (fid, uid) = row?;
                out.insert(fid, uid);
            }
        }
        Ok(out)
    }

    /// Every row's fid and its lasting id (none when it has none), in fid order, handed one at a time: a table's ids
    /// walked without holding them all (docs/adr/0597).
    pub fn each_uid(
        &self,
        layer: &Layer,
        mut f: impl FnMut(i64, Option<&str>),
    ) -> Result<()> {
        let (fid, table) = (quote(&layer.fid), quote(&layer.name));
        let ids = self.has_uids()?;
        let mut st = self.conn.prepare(&if ids {
            format!(
                "SELECT t.{fid}, u.uid FROM {table} t LEFT JOIN kentos_uids u ON u.table_name = ?1 AND u.fid = t.{fid} ORDER BY 1"
            )
        } else {
            format!("SELECT {fid}, NULL FROM {table} ORDER BY 1")
        })?;
        let mut rows = if ids {
            st.query([&layer.name])?
        } else {
            st.query([])?
        };
        while let Some(r) = rows.next()? {
            let fid: i64 = r.get(0)?;
            let uid: Option<String> = r.get(1)?;
            f(fid, uid.as_deref());
        }
        Ok(())
    }

    /// KentOS's lasting ids of a table's rows, by fid (the `kentos_uids` extension; none when the file lacks it).
    pub fn uids(&self, table: &str) -> Result<HashMap<i64, String>> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'kentos_uids'",
            [],
            |r| r.get(0),
        )?;
        if n == 0 {
            return Ok(HashMap::new());
        }
        let mut st = self
            .conn
            .prepare("SELECT fid, uid FROM kentos_uids WHERE table_name = ?1")?;
        let rows = st
            .query_map([table], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// Adds a table's metadata (the GeoPackage metadata extension, `gpkg_metadata` and `gpkg_metadata_reference`, made
    /// when the file lacks them): one dataset-scope document of `standard` (a URI) and `mime` type, referenced to
    /// `table` at `timestamp` (`YYYY-MM-DDTHH:MM:SS.SSSZ`). Returns its id.
    pub fn add_metadata(
        &self,
        table: &str,
        standard: &str,
        mime: &str,
        metadata: &str,
        timestamp: &str,
    ) -> Result<i64> {
        add_metadata_in(&self.conn, table, standard, mime, metadata, timestamp)
    }

    /// The metadata documents `table` carries (docs/adr/0511, 0530): each one's standard URI and its text, in id
    /// order; none when the file has no metadata extension.
    pub fn metadata(&self, table: &str) -> Result<Vec<(String, String)>> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('gpkg_metadata', 'gpkg_metadata_reference')",
            [],
            |r| r.get(0),
        )?;
        if n < 2 {
            return Ok(Vec::new());
        }
        let mut st = self.conn.prepare(
            "SELECT m.md_standard_uri, m.metadata FROM gpkg_metadata m
               JOIN gpkg_metadata_reference r ON r.md_file_id = m.id
              WHERE r.table_name = ?1 ORDER BY m.id",
        )?;
        let rows = st
            .query_map([table], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// Adds a spatial reference system, unless one with its id is there.
    pub fn add_srs(&self, s: &Srs) -> Result<()> {
        insert_srs(&self.conn, s)
    }

    /// Writes a table with its rows in one transaction: its contents and geometry column rows, its box, its spatial
    /// index and, when `uids` is given, each row's lasting id. Returns the rows' fids in their order.
    pub fn write_table(
        &mut self,
        def: &TableDef,
        rows: &[NewRow],
        uids: Option<&[String]>,
    ) -> Result<Vec<i64>> {
        let tx = self.conn.transaction()?;
        let mut w = start_in(&tx, def)?;
        let fids = append_in(&tx, &mut w, rows, uids)?;
        finish_in(&tx, w)?;
        tx.commit()?;
        Ok(fids)
    }

    /// Opens a transaction that writes tables piece by piece (docs/adr/0549): tables made, rows appended in batches,
    /// tables replaced, all of it committed together or, when the session is dropped or rolled back, none of it.
    pub fn session(&mut self) -> Result<Session<'_>> {
        Ok(Session {
            tx: self.conn.transaction()?,
        })
    }


    /// Edits a table of the file in place, in one transaction (docs/adr/0503): its missing columns added (as
    /// declared), rows updated by fid (their geometry, when the table has a column for it, and every value in the
    /// layer's column order, the added ones last), rows inserted with their lasting ids (`kentos_uids`, made when the
    /// file lacks it), rows deleted with theirs. The spatial index follows by its triggers; unknown tables and
    /// extensions are not touched. Returns the inserted rows' fids. Nothing is written when any step fails.
    pub fn edit(&mut self, layer: &Layer, edits: &Edits) -> Result<Vec<i64>> {
        let tx = self.conn.transaction()?;
        let fids = edit_in(&tx, layer, edits)?;
        tx.commit()?;
        Ok(fids)
    }

    /// Edits several tables, and the mapping tables of their relations, in one transaction (docs/adr/0532 §1): all of
    /// it or none of it. Returns each table's inserted rows' fids in the jobs' order.
    pub fn edit_many(
        &mut self,
        jobs: &[(&Layer, &Edits)],
        maps: &[relations::MapEdit],
    ) -> Result<Vec<Vec<i64>>> {
        let tx = self.conn.transaction()?;
        let mut fids = Vec::with_capacity(jobs.len());
        for (layer, edits) in jobs {
            fids.push(edit_in(&tx, layer, edits)?);
        }
        for m in maps {
            relations::write_mapping(&tx, m, &fids)?;
        }
        tx.commit()?;
        Ok(fids)
    }

    /// Closes the file, writing everything out.
    pub fn close(self) -> Result<()> {
        self.conn.close().map_err(|(_, e)| e.into())
    }
}

fn edit_in(tx: &rusqlite::Transaction<'_>, layer: &Layer, edits: &Edits) -> Result<Vec<i64>> {
        let t = quote(&layer.name);
        for c in &edits.add_columns {
            let decl = if c.decl.trim().is_empty() {
                c.kind.decl()
            } else {
                c.decl.clone()
            };
            tx.execute_batch(&format!(
                "ALTER TABLE {t} ADD COLUMN {} {decl};",
                quote(&c.name)
            ))?;
        }
        let mut cols: Vec<String> = layer.columns.iter().map(|c| quote(&c.name)).collect();
        cols.extend(edits.add_columns.iter().map(|c| quote(&c.name)));
        let geom = layer.geometry.as_ref().map(|g| quote(&g.name));
        let srs = layer.srs_id.unwrap_or(-1);
        let width = cols.len();
        let values = |row: &NewRow| -> Result<Vec<Value>> {
            if row.values.len() != width {
                return Err(Error(format!(
                    "“{}” tablosunun satırında {} değer var, {width} sütun bekleniyor",
                    layer.name,
                    row.values.len()
                )));
            }
            let mut v = Vec::with_capacity(width + 2);
            if geom.is_some() {
                v.push(
                    row.geometry
                        .as_ref()
                        .map_or(Value::Null, |g| Value::Blob(write_blob(srs, g))),
                );
            }
            v.extend(row.values.iter().cloned());
            Ok(v)
        };
        let mut names: Vec<String> = Vec::new();
        if let Some(g) = &geom {
            names.push(g.clone());
        }
        names.extend(cols.iter().cloned());
        if !edits.update.is_empty() && !names.is_empty() {
            let sets = names
                .iter()
                .map(|n| format!("{n} = ?"))
                .collect::<Vec<_>>()
                .join(", ");
            let mut st = tx.prepare(&format!(
                "UPDATE {t} SET {sets} WHERE {} = ?",
                quote(&layer.fid)
            ))?;
            for (fid, row) in &edits.update {
                let mut v = values(row)?;
                v.push(Value::Integer(*fid));
                if st.execute(rusqlite::params_from_iter(v.iter()))? != 1 {
                    return Err(Error(format!(
                        "“{}” tablosunda {fid} numaralı satır yok",
                        layer.name
                    )));
                }
            }
        }
        let mut fids = Vec::with_capacity(edits.insert.len());
        if !edits.insert.is_empty() {
            let marks = vec!["?"; names.len()].join(", ");
            let sql = if names.is_empty() {
                format!("INSERT INTO {t} DEFAULT VALUES")
            } else {
                format!("INSERT INTO {t} ({}) VALUES ({marks})", names.join(", "))
            };
            let mut st = tx.prepare(&sql)?;
            for (_, row) in &edits.insert {
                st.execute(rusqlite::params_from_iter(values(row)?.iter()))?;
                fids.push(tx.last_insert_rowid());
            }
        }
        if !edits.delete.is_empty() {
            let mut st = tx.prepare(&format!("DELETE FROM {t} WHERE {} = ?", quote(&layer.fid)))?;
            for fid in &edits.delete {
                st.execute([fid])?;
            }
        }
        if !edits.insert.is_empty() || !edits.delete.is_empty() {
            tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS kentos_uids (table_name TEXT NOT NULL, fid INTEGER NOT NULL, uid TEXT NOT NULL,
                 PRIMARY KEY (table_name, fid));",
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO gpkg_extensions VALUES ('kentos_uids', NULL, ?1, 'https://kentos.app/geopackage/uids (docs/adr/0502)', 'read-write')",
                [UIDS_EXTENSION],
            )?;
            let mut put = tx.prepare("INSERT OR REPLACE INTO kentos_uids VALUES (?1, ?2, ?3)")?;
            for (fid, (uid, _)) in fids.iter().zip(&edits.insert) {
                put.execute(params![layer.name, fid, uid])?;
            }
            let mut gone =
                tx.prepare("DELETE FROM kentos_uids WHERE table_name = ?1 AND fid = ?2")?;
            for fid in &edits.delete {
                gone.execute(params![layer.name, fid])?;
            }
        }
        // The contents' time: what changed is said where other programs look (a table left as it is keeps its time).
        let changed = !edits.add_columns.is_empty()
            || !edits.update.is_empty()
            || !edits.insert.is_empty()
            || !edits.delete.is_empty();
        if changed {
            tx.execute(
                "UPDATE gpkg_contents SET last_change = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE table_name = ?1",
                [&layer.name],
            )?;
        }
        Ok(fids)
}

/// The spatial index of a table's geometry column (§F.3): GeoPackage 1.3's triggers in a file of its own, the file's
/// own edition's in an older or newer one (1.4 has `update2`, `4`, `5`, `6` and `7`; a table added to QGIS's 1.4 file
/// gets the triggers its validator wants, docs/adr/0549).
fn index(conn: &Connection, table: &str, column: &str) -> Result<()> {
    let t = quote(table);
    let c = quote(column);
    let r = quote(&format!("rtree_{table}_{column}"));
    let trig = |s: &str| quote(&format!("rtree_{table}_{column}_{s}"));
    let row = |new: &str| {
        format!(
            "({new}.\"fid\", ST_MinX({new}.{c}), ST_MaxX({new}.{c}), ST_MinY({new}.{c}), ST_MaxY({new}.{c}))"
        )
    };
    let edition: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let updates = if edition >= 10400 {
        format!(
            "CREATE TRIGGER {u6} AFTER UPDATE OF {c} ON {t} WHEN OLD.\"fid\" = NEW.\"fid\" AND (NEW.{c} NOTNULL AND NOT ST_IsEmpty(NEW.{c})) AND (OLD.{c} NOTNULL AND NOT ST_IsEmpty(OLD.{c}))
               BEGIN UPDATE {r} SET minx = ST_MinX(NEW.{c}), maxx = ST_MaxX(NEW.{c}), miny = ST_MinY(NEW.{c}), maxy = ST_MaxY(NEW.{c}) WHERE id = NEW.\"fid\"; END;
             CREATE TRIGGER {u7} AFTER UPDATE OF {c} ON {t} WHEN OLD.\"fid\" = NEW.\"fid\" AND (NEW.{c} NOTNULL AND NOT ST_IsEmpty(NEW.{c})) AND (OLD.{c} ISNULL OR ST_IsEmpty(OLD.{c}))
               BEGIN INSERT INTO {r} VALUES {new}; END;
             CREATE TRIGGER {u5} AFTER UPDATE ON {t} WHEN OLD.\"fid\" != NEW.\"fid\" AND (NEW.{c} NOTNULL AND NOT ST_IsEmpty(NEW.{c}))
               BEGIN DELETE FROM {r} WHERE id = OLD.\"fid\"; INSERT OR REPLACE INTO {r} VALUES {new}; END;",
            u5 = trig("update5"),
            u6 = trig("update6"),
            u7 = trig("update7"),
            new = row("NEW"),
        )
    } else {
        format!(
            "CREATE TRIGGER {u1} AFTER UPDATE OF {c} ON {t} WHEN OLD.\"fid\" = NEW.\"fid\" AND (NEW.{c} NOTNULL AND NOT ST_IsEmpty(NEW.{c}))
               BEGIN INSERT OR REPLACE INTO {r} VALUES {new}; END;
             CREATE TRIGGER {u3} AFTER UPDATE ON {t} WHEN OLD.\"fid\" != NEW.\"fid\" AND (NEW.{c} NOTNULL AND NOT ST_IsEmpty(NEW.{c}))
               BEGIN DELETE FROM {r} WHERE id = OLD.\"fid\"; INSERT OR REPLACE INTO {r} VALUES {new}; END;",
            u1 = trig("update1"),
            u3 = trig("update3"),
            new = row("NEW"),
        )
    };
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE {r} USING rtree(id, minx, maxx, miny, maxy);
         INSERT OR REPLACE INTO {r} SELECT \"fid\", ST_MinX({c}), ST_MaxX({c}), ST_MinY({c}), ST_MaxY({c}) FROM {t}
           WHERE {c} NOT NULL AND NOT ST_IsEmpty({c});
         CREATE TRIGGER {ins} AFTER INSERT ON {t} WHEN (new.{c} NOT NULL AND NOT ST_IsEmpty(NEW.{c}))
           BEGIN INSERT OR REPLACE INTO {r} VALUES {new}; END;
         {updates}
         CREATE TRIGGER {u2} AFTER UPDATE OF {c} ON {t} WHEN OLD.\"fid\" = NEW.\"fid\" AND (NEW.{c} ISNULL OR ST_IsEmpty(NEW.{c}))
           BEGIN DELETE FROM {r} WHERE id = OLD.\"fid\"; END;
         CREATE TRIGGER {u4} AFTER UPDATE ON {t} WHEN OLD.\"fid\" != NEW.\"fid\" AND (NEW.{c} ISNULL OR ST_IsEmpty(NEW.{c}))
           BEGIN DELETE FROM {r} WHERE id IN (OLD.\"fid\", NEW.\"fid\"); END;
         CREATE TRIGGER {del} AFTER DELETE ON {t} WHEN old.{c} NOT NULL
           BEGIN DELETE FROM {r} WHERE id = OLD.\"fid\"; END;",
        ins = trig("insert"),
        u2 = trig("update2"),
        u4 = trig("update4"),
        del = trig("delete"),
        new = row("NEW"),
    ))?;
    conn.execute(
        "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, ?2, 'gpkg_rtree_index', 'http://www.geopackage.org/spec/#extension_rtree', 'write-only')",
        params![table, column],
    )?;
    Ok(())
}

/// A table being written piece by piece: what it must still do when its rows are all in (its box, its index).
#[derive(Debug)]
pub struct TableWrite {
    def: TableDef,
    bounds: Option<[f64; 4]>,
    curved: bool,
    rows: u64,
}

impl TableWrite {
    /// The rows appended so far.
    pub fn rows(&self) -> u64 {
        self.rows
    }
}

/// What a name is in a file (docs/adr/0549): free, a feature or attribute table a session may replace, or something
/// it must not touch (a tile table, an unregistered SQLite table, an extension's table, a view).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Taken {
    Free,
    Data(DataType),
    Other,
}

/// A transaction writing tables (docs/adr/0549): nothing is in the file until [`Session::commit`]; dropping the session,
/// or [`Session::rollback`], leaves the file as it was.
pub struct Session<'a> {
    tx: rusqlite::Transaction<'a>,
}

impl Session<'_> {
    /// What `name` is in the file; SQLite's names are one whatever their case.
    pub fn taken(&self, name: &str) -> Result<Taken> {
        let in_master: i64 = self.tx.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = ?1 COLLATE NOCASE AND type IN ('table', 'view')",
            [name],
            |r| r.get(0),
        )?;
        if in_master == 0 {
            return Ok(Taken::Free);
        }
        let kind: Option<String> = self
            .tx
            .query_row(
                "SELECT data_type FROM gpkg_contents WHERE table_name = ?1 COLLATE NOCASE",
                [name],
                |r| r.get(0),
            )
            .ok();
        Ok(match kind.as_deref().map(DataType::read) {
            Some(t @ (DataType::Features | DataType::Attributes)) => Taken::Data(t),
            _ => Taken::Other,
        })
    }

    /// The exact name `gpkg_contents` keeps a table under (the case it was written in).
    pub fn registered_name(&self, name: &str) -> Result<Option<String>> {
        Ok(self
            .tx
            .query_row(
                "SELECT table_name FROM gpkg_contents WHERE table_name = ?1 COLLATE NOCASE",
                [name],
                |r| r.get(0),
            )
            .ok())
    }

    /// The spatial reference system of `id`, when the file has one.
    pub fn srs(&self, id: i32) -> Result<Option<Srs>> {
        read_srs(&self.tx, id)
    }

    /// Adds a spatial reference system, unless one with its id is there.
    pub fn add_srs(&self, s: &Srs) -> Result<()> {
        insert_srs(&self.tx, s)
    }

    /// Takes a feature or attribute table out of the file, with everything of it: its rows, its contents and geometry
    /// column rows, its extension rows, its spatial index and its triggers, its lasting ids, and the metadata
    /// documents only it referenced. Other tables, and tables other programs keep about it, are not touched.
    pub fn drop_table(&self, name: &str) -> Result<()> {
        let exact = self.registered_name(name)?.unwrap_or_else(|| name.to_owned());
        let geometry: Option<String> = self
            .tx
            .query_row(
                "SELECT column_name FROM gpkg_geometry_columns WHERE table_name = ?1",
                [&exact],
                |r| r.get(0),
            )
            .ok();
        let has = |t: &str| -> Result<bool> {
            let n: i64 = self.tx.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [t],
                |r| r.get(0),
            )?;
            Ok(n > 0)
        };
        if let Some(column) = &geometry {
            let r = format!("rtree_{exact}_{column}");
            if has(&r)? {
                self.tx
                    .execute_batch(&format!("DROP TABLE {};", quote(&r)))?;
            }
            self.tx.execute(
                "DELETE FROM gpkg_extensions WHERE table_name = ?1 AND column_name = ?2",
                params![exact, column],
            )?;
        }
        if has("gpkg_metadata_reference")? && has("gpkg_metadata")? {
            let docs: Vec<i64> = {
                let mut st = self.tx.prepare(
                    "SELECT md_file_id FROM gpkg_metadata_reference WHERE table_name = ?1",
                )?;
                st.query_map([&exact], |r| r.get(0))?
                    .collect::<rusqlite::Result<_>>()?
            };
            self.tx.execute(
                "DELETE FROM gpkg_metadata_reference WHERE table_name = ?1",
                [&exact],
            )?;
            for id in docs {
                self.tx.execute(
                    "DELETE FROM gpkg_metadata WHERE id = ?1
                       AND NOT EXISTS (SELECT 1 FROM gpkg_metadata_reference WHERE md_file_id = ?1 OR md_parent_id = ?1)",
                    [id],
                )?;
            }
        }
        if has("kentos_uids")? {
            self.tx
                .execute("DELETE FROM kentos_uids WHERE table_name = ?1", [&exact])?;
        }
        if has("gpkg_ogr_contents")? {
            self.tx.execute(
                "DELETE FROM gpkg_ogr_contents WHERE table_name = ?1",
                [&exact],
            )?;
        }
        self.tx
            .execute("DELETE FROM gpkg_extensions WHERE table_name = ?1", [&exact])?;
        self.tx.execute(
            "DELETE FROM gpkg_geometry_columns WHERE table_name = ?1",
            [&exact],
        )?;
        self.tx.execute(
            "DELETE FROM gpkg_contents WHERE table_name = ?1",
            [&exact],
        )?;
        self.tx
            .execute_batch(&format!("DROP TABLE {};", quote(&exact)))?;
        Ok(())
    }

    /// Starts a table: its SQL table, its contents and its geometry column rows. Rows follow with [`Session::append`].
    pub fn start_table(&self, def: &TableDef) -> Result<TableWrite> {
        start_in(&self.tx, def)
    }

    /// Appends a batch of rows to a table started here, with each row's lasting id when `uids` is given. Returns the
    /// rows' fids in their order.
    pub fn append(
        &self,
        w: &mut TableWrite,
        rows: &[NewRow],
        uids: Option<&[String]>,
    ) -> Result<Vec<i64>> {
        append_in(&self.tx, w, rows, uids)
    }

    /// Ends a table: its box, its spatial index and the extension of its curved geometry type.
    pub fn finish_table(&self, w: TableWrite) -> Result<()> {
        finish_in(&self.tx, w)
    }

    /// A table's metadata document, as [`GeoPackage::add_metadata`].
    pub fn add_metadata(
        &self,
        table: &str,
        standard: &str,
        mime: &str,
        metadata: &str,
        timestamp: &str,
    ) -> Result<i64> {
        add_metadata_in(&self.tx, table, standard, mime, metadata, timestamp)
    }

    /// Writes everything out: the file is as the session made it.
    pub fn commit(self) -> Result<()> {
        self.tx.commit()?;
        Ok(())
    }

    /// Takes everything back: the file is as it was.
    pub fn rollback(self) -> Result<()> {
        self.tx.rollback()?;
        Ok(())
    }
}

fn start_in(conn: &Connection, def: &TableDef) -> Result<TableWrite> {
    if def
        .columns
        .iter()
        .any(|c| c.name.eq_ignore_ascii_case("fid"))
        || def
            .geometry
            .as_ref()
            .is_some_and(|g| g.name.eq_ignore_ascii_case("fid"))
    {
        return Err(Error(format!(
            "“{}” tablosunun bir sütunu fid adını taşıyor",
            def.name
        )));
    }
    let t = quote(&def.name);
    let mut cols = vec!["\"fid\" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL".to_owned()];
    if let Some(g) = &def.geometry {
        cols.push(format!("{} {}", quote(&g.name), g.type_name));
    }
    for c in &def.columns {
        let decl = if c.decl.trim().is_empty() {
            c.kind.decl()
        } else {
            c.decl.clone()
        };
        let mut s = format!("{} {decl}", quote(&c.name));
        if c.not_null {
            s.push_str(" NOT NULL");
        }
        if let Some(d) = &c.default {
            s.push_str(&format!(" DEFAULT {d}"));
        }
        cols.push(s);
    }
    conn.execute_batch(&format!("CREATE TABLE {t} ({});", cols.join(", ")))?;
    conn.execute(
        "INSERT INTO gpkg_contents (table_name, data_type, identifier, description, srs_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            def.name,
            def.data_type.key(),
            def.identifier.clone().unwrap_or_else(|| def.name.clone()),
            def.description.clone().unwrap_or_default(),
            def.srs_id
        ],
    )?;
    if let Some(g) = &def.geometry {
        conn.execute(
            "INSERT INTO gpkg_geometry_columns VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                def.name,
                g.name,
                g.type_name,
                def.srs_id.unwrap_or(-1),
                g.z,
                g.m
            ],
        )?;
    }
    Ok(TableWrite {
        def: def.clone(),
        bounds: None,
        curved: false,
        rows: 0,
    })
}

fn append_in(
    conn: &Connection,
    w: &mut TableWrite,
    rows: &[NewRow],
    uids: Option<&[String]>,
) -> Result<Vec<i64>> {
    let def = &w.def;
    let t = quote(&def.name);
    let srs = def.srs_id.unwrap_or(-1);
    let mut names = vec!["\"fid\"".to_owned()];
    if let Some(g) = &def.geometry {
        names.push(quote(&g.name));
    }
    names.extend(def.columns.iter().map(|c| quote(&c.name)));
    let marks = vec!["?"; names.len()].join(", ");
    let mut fids = Vec::with_capacity(rows.len());
    let (mut bounds, mut curved) = (w.bounds, w.curved);
    {
        let mut st = conn.prepare_cached(&format!(
            "INSERT INTO {t} ({}) VALUES ({marks})",
            names.join(", ")
        ))?;
        for row in rows {
            if row.values.len() != def.columns.len() {
                return Err(Error(format!(
                    "“{}” tablosunun satırında {} değer var, {} sütun bekleniyor",
                    def.name,
                    row.values.len(),
                    def.columns.len()
                )));
            }
            let mut args = vec![row.fid.map_or(Value::Null, Value::Integer)];
            if def.geometry.is_some() {
                args.push(match &row.geometry {
                    Some(g) => {
                        curved |= g.is_curved();
                        if let Some(b) = g.bounds() {
                            bounds = Some(match bounds {
                                None => b,
                                Some(a) => [
                                    a[0].min(b[0]),
                                    a[1].min(b[1]),
                                    a[2].max(b[2]),
                                    a[3].max(b[3]),
                                ],
                            });
                        }
                        Value::Blob(write_blob(srs, g))
                    }
                    None => Value::Null,
                });
            }
            args.extend(row.values.iter().cloned());
            st.execute(rusqlite::params_from_iter(args.iter()))?;
            fids.push(conn.last_insert_rowid());
        }
    }
    if let Some(uids) = uids {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kentos_uids (table_name TEXT NOT NULL, fid INTEGER NOT NULL, uid TEXT NOT NULL,
             PRIMARY KEY (table_name, fid));",
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO gpkg_extensions VALUES ('kentos_uids', NULL, ?1, 'https://kentos.app/geopackage/uids (docs/adr/0502)', 'read-write')",
            [UIDS_EXTENSION],
        )?;
        let mut st = conn.prepare_cached("INSERT OR REPLACE INTO kentos_uids VALUES (?1, ?2, ?3)")?;
        for (fid, uid) in fids.iter().zip(uids) {
            st.execute(params![def.name, fid, uid])?;
        }
    }
    w.bounds = bounds;
    w.curved = curved;
    w.rows += rows.len() as u64;
    Ok(fids)
}

fn finish_in(conn: &Connection, w: TableWrite) -> Result<()> {
    let def = &w.def;
    if let Some(b) = w.bounds {
        conn.execute(
            "UPDATE gpkg_contents SET min_x = ?2, min_y = ?3, max_x = ?4, max_y = ?5 WHERE table_name = ?1",
            params![def.name, b[0], b[1], b[2], b[3]],
        )?;
    }
    if let Some(g) = &def.geometry {
        index(conn, &def.name, &g.name)?;
        if w.curved {
            conn.execute(
                "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, ?2, ?3, 'http://www.geopackage.org/spec/#extension_geometry_types', 'read-write')",
                params![def.name, g.name, format!("gpkg_geom_{}", g.type_name)],
            )?;
        }
    }
    Ok(())
}

fn add_metadata_in(
    conn: &Connection,
    table: &str,
    standard: &str,
    mime: &str,
    metadata: &str,
    timestamp: &str,
) -> Result<i64> {
    const DEFINITION: &str = "http://www.geopackage.org/spec120/#extension_metadata";
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS gpkg_metadata (
           id INTEGER CONSTRAINT m_pk PRIMARY KEY ASC NOT NULL,
           md_scope TEXT NOT NULL DEFAULT 'dataset',
           md_standard_uri TEXT NOT NULL,
           mime_type TEXT NOT NULL DEFAULT 'text/xml',
           metadata TEXT NOT NULL DEFAULT '');
         CREATE TABLE IF NOT EXISTS gpkg_metadata_reference (
           reference_scope TEXT NOT NULL,
           table_name TEXT,
           column_name TEXT,
           row_id_value INTEGER,
           timestamp DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
           md_file_id INTEGER NOT NULL,
           md_parent_id INTEGER,
           CONSTRAINT crmr_mfi_fk FOREIGN KEY (md_file_id) REFERENCES gpkg_metadata(id),
           CONSTRAINT crmr_mpi_fk FOREIGN KEY (md_parent_id) REFERENCES gpkg_metadata(id));",
    )?;
    for t in ["gpkg_metadata", "gpkg_metadata_reference"] {
        conn.execute(
            "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, NULL, 'gpkg_metadata', ?2, 'read-write')",
            params![t, DEFINITION],
        )?;
    }
    conn.execute(
        "INSERT INTO gpkg_metadata (md_scope, md_standard_uri, mime_type, metadata) VALUES ('dataset', ?1, ?2, ?3)",
        params![standard, mime, metadata],
    )?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO gpkg_metadata_reference (reference_scope, table_name, timestamp, md_file_id) VALUES ('table', ?1, ?2, ?3)",
        params![table, timestamp, id],
    )?;
    Ok(id)
}

fn read_srs(conn: &Connection, id: i32) -> Result<Option<Srs>> {
    let r = conn.query_row(
        "SELECT srs_name, srs_id, organization, organization_coordsys_id, definition, description
         FROM gpkg_spatial_ref_sys WHERE srs_id = ?1",
        [id],
        |r| {
            Ok(Srs {
                name: r.get(0)?,
                id: r.get(1)?,
                organization: r.get(2)?,
                organization_id: r.get(3)?,
                definition: r.get(4)?,
                description: r.get(5)?,
            })
        },
    );
    match r {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn insert_srs(conn: &Connection, s: &Srs) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO gpkg_spatial_ref_sys VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            s.name,
            s.id,
            s.organization,
            s.organization_id,
            s.definition,
            s.description
        ],
    )?;
    Ok(())
}
