//! GeoPackage (OGC 12-128r19, docs/adr/0502, plan/gis.md CBS-03) over SQLite: a file's feature and attribute tables
//! with their columns, spatial reference systems and extensions; rows read a page at a time (by the spatial index's
//! box when it has one, then by fid); tables written in one transaction with the spatial index (`gpkg_rtree_index`,
//! its GeoPackage 1.3 triggers and the `ST_*` functions they call) and KentOS's lasting ids per fid (the `kentos_uids`
//! extension, which other programs leave alone). It knows nothing of a drawing: the product commands turn rows into
//! objects.
//!
//! A file written here is GeoPackage 1.3 (`application_id` GPKG, `user_version` 10300). Unknown extensions and tables
//! are not touched. SQLite is compiled in (rusqlite's `bundled`).

pub mod geom;

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
        let r = self.conn.query_row(
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

    /// The rows `q` asks for, in fid order.
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
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
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

    /// Adds a spatial reference system, unless one with its id is there.
    pub fn add_srs(&self, s: &Srs) -> Result<()> {
        self.conn.execute(
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

    /// Writes a table with its rows in one transaction: its contents and geometry column rows, its box, its spatial
    /// index and, when `uids` is given, each row's lasting id. Returns the rows' fids in their order.
    pub fn write_table(
        &mut self,
        def: &TableDef,
        rows: &[NewRow],
        uids: Option<&[String]>,
    ) -> Result<Vec<i64>> {
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
        let tx = self.conn.transaction()?;
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
        tx.execute_batch(&format!("CREATE TABLE {t} ({});", cols.join(", ")))?;
        tx.execute(
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
            tx.execute(
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
        let srs = def.srs_id.unwrap_or(-1);
        let mut names = vec!["\"fid\"".to_owned()];
        if let Some(g) = &def.geometry {
            names.push(quote(&g.name));
        }
        names.extend(def.columns.iter().map(|c| quote(&c.name)));
        let marks = vec!["?"; names.len()].join(", ");
        let mut fids = Vec::with_capacity(rows.len());
        let mut bounds: Option<[f64; 4]> = None;
        let mut curved = false;
        {
            let mut st = tx.prepare(&format!(
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
                fids.push(tx.last_insert_rowid());
            }
        }
        if let Some(b) = bounds {
            tx.execute(
                "UPDATE gpkg_contents SET min_x = ?2, min_y = ?3, max_x = ?4, max_y = ?5 WHERE table_name = ?1",
                params![def.name, b[0], b[1], b[2], b[3]],
            )?;
        }
        if let Some(g) = &def.geometry {
            index(&tx, &def.name, &g.name)?;
            if curved {
                tx.execute(
                    "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, ?2, ?3, 'http://www.geopackage.org/spec/#extension_geometry_types', 'read-write')",
                    params![def.name, g.name, format!("gpkg_geom_{}", g.type_name)],
                )?;
            }
        }
        if let Some(uids) = uids {
            tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS kentos_uids (table_name TEXT NOT NULL, fid INTEGER NOT NULL, uid TEXT NOT NULL,
                 PRIMARY KEY (table_name, fid));",
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO gpkg_extensions VALUES ('kentos_uids', NULL, ?1, 'https://kentos.app/geopackage/uids (docs/adr/0502)', 'read-write')",
                [UIDS_EXTENSION],
            )?;
            let mut st = tx.prepare("INSERT OR REPLACE INTO kentos_uids VALUES (?1, ?2, ?3)")?;
            for (fid, uid) in fids.iter().zip(uids) {
                st.execute(params![def.name, fid, uid])?;
            }
        }
        tx.commit()?;
        Ok(fids)
    }

    /// Closes the file, writing everything out.
    pub fn close(self) -> Result<()> {
        self.conn.close().map_err(|(_, e)| e.into())
    }
}

/// The spatial index of a table's geometry column (§F.3, GeoPackage 1.3's triggers).
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
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE {r} USING rtree(id, minx, maxx, miny, maxy);
         INSERT OR REPLACE INTO {r} SELECT \"fid\", ST_MinX({c}), ST_MaxX({c}), ST_MinY({c}), ST_MaxY({c}) FROM {t}
           WHERE {c} NOT NULL AND NOT ST_IsEmpty({c});
         CREATE TRIGGER {ins} AFTER INSERT ON {t} WHEN (new.{c} NOT NULL AND NOT ST_IsEmpty(NEW.{c}))
           BEGIN INSERT OR REPLACE INTO {r} VALUES {new}; END;
         CREATE TRIGGER {u1} AFTER UPDATE OF {c} ON {t} WHEN OLD.\"fid\" = NEW.\"fid\" AND (NEW.{c} NOTNULL AND NOT ST_IsEmpty(NEW.{c}))
           BEGIN INSERT OR REPLACE INTO {r} VALUES {new}; END;
         CREATE TRIGGER {u2} AFTER UPDATE OF {c} ON {t} WHEN OLD.\"fid\" = NEW.\"fid\" AND (NEW.{c} ISNULL OR ST_IsEmpty(NEW.{c}))
           BEGIN DELETE FROM {r} WHERE id = OLD.\"fid\"; END;
         CREATE TRIGGER {u3} AFTER UPDATE ON {t} WHEN OLD.\"fid\" != NEW.\"fid\" AND (NEW.{c} NOTNULL AND NOT ST_IsEmpty(NEW.{c}))
           BEGIN DELETE FROM {r} WHERE id = OLD.\"fid\"; INSERT OR REPLACE INTO {r} VALUES {new}; END;
         CREATE TRIGGER {u4} AFTER UPDATE ON {t} WHEN OLD.\"fid\" != NEW.\"fid\" AND (NEW.{c} ISNULL OR ST_IsEmpty(NEW.{c}))
           BEGIN DELETE FROM {r} WHERE id IN (OLD.\"fid\", NEW.\"fid\"); END;
         CREATE TRIGGER {del} AFTER DELETE ON {t} WHEN old.{c} NOT NULL
           BEGIN DELETE FROM {r} WHERE id = OLD.\"fid\"; END;",
        ins = trig("insert"),
        u1 = trig("update1"),
        u2 = trig("update2"),
        u3 = trig("update3"),
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
