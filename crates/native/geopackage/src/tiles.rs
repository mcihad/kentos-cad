//! GeoPackage tile pyramids (OGC 12-128r19 §2.2, docs/adr/0556, 0627): a tile table (`data_type` `tiles`) with its
//! tile matrix set and its tile matrices, tiles read by zoom, column and row, and a pyramid written in one transaction.
//! A gridded coverage (OGC 17-066r1, `data_type` `2d-gridded-coverage`) is a tile table too, with its ancillary rows:
//! the coverage's data type, scale, offset and null, and each tile's scale and offset. Tile bytes are opaque here (PNG,
//! JPEG, WebP or TIFF); decoding them is the formats core's. Row 0 of a matrix is its top row, column 0 its left one,
//! and the matrix's origin is the tile matrix set's top-left corner.

use rusqlite::params;

use crate::{Error, GeoPackage, Result, quote};

/// A zoom level of a tile table (`gpkg_tile_matrix`).
#[derive(Clone, Debug, PartialEq)]
pub struct TileMatrix {
    pub zoom: i64,
    pub matrix_width: i64,
    pub matrix_height: i64,
    pub tile_width: i64,
    pub tile_height: i64,
    pub pixel_x_size: f64,
    pub pixel_y_size: f64,
}

/// A gridded coverage's description (`gpkg_2d_gridded_coverage_ancillary`).
#[derive(Clone, Debug, PartialEq)]
pub struct Coverage {
    /// `integer` (16-bit PNG tiles) or `float` (32-bit float TIFF tiles).
    pub datatype: String,
    pub scale: f64,
    pub offset: f64,
    pub precision: Option<f64>,
    /// The null: a stored (unscaled) value for `integer`, a value for `float`.
    pub data_null: Option<f64>,
    pub grid_cell_encoding: Option<String>,
    pub uom: Option<String>,
    pub field_name: Option<String>,
    pub quantity_definition: Option<String>,
}

impl Coverage {
    /// Whether its tiles are 16-bit PNG integers (else 32-bit float TIFFs).
    pub fn integer(&self) -> bool {
        !self.datatype.eq_ignore_ascii_case("float")
    }
}

/// A tile table: its tile matrix set and its matrices by zoom.
#[derive(Clone, Debug, PartialEq)]
pub struct TileSet {
    pub name: String,
    pub identifier: Option<String>,
    pub description: Option<String>,
    pub srs_id: i32,
    /// The tile matrix set's box (the grid's origin is its top-left corner): min x, min y, max x, max y.
    pub bounds: [f64; 4],
    /// `gpkg_contents`'s box: the data's extent, when it has one.
    pub data_bounds: Option<[f64; 4]>,
    /// The matrices, by zoom level ascending.
    pub matrices: Vec<TileMatrix>,
    /// A gridded coverage's description; none for a table of pictures.
    pub coverage: Option<Coverage>,
}

impl TileSet {
    /// The matrix of a zoom level.
    pub fn matrix(&self, zoom: i64) -> Option<&TileMatrix> {
        self.matrices.iter().find(|m| m.zoom == zoom)
    }
}

/// A tile a pyramid is written with: `row` 0 is the top row.
#[derive(Clone, Debug, PartialEq)]
pub struct NewTile {
    pub zoom: i64,
    pub column: i64,
    pub row: i64,
    pub data: Vec<u8>,
    /// A gridded coverage's tile: its values' statistics (`gpkg_2d_gridded_tile_ancillary`, scale 1, offset 0).
    pub stats: Option<TileStats>,
}

/// A coverage tile's values, nulls left out: none when it holds none.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TileStats {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub std_dev: Option<f64>,
}

/// A tile read: its bytes and, of a gridded coverage, its scale and offset (1 and 0 otherwise).
#[derive(Clone, Debug, PartialEq)]
pub struct TileBlob {
    pub data: Vec<u8>,
    pub scale: f64,
    pub offset: f64,
}

/// A gridded coverage to write (`gpkg_2d_gridded_coverage_ancillary`'s row).
#[derive(Clone, Debug, PartialEq)]
pub struct NewCoverage {
    /// `integer` or `float`.
    pub datatype: String,
    pub scale: f64,
    pub offset: f64,
    pub data_null: Option<f64>,
    pub uom: Option<String>,
}

/// A tile table to write.
#[derive(Clone, Debug, PartialEq)]
pub struct NewTileSet {
    pub name: String,
    pub identifier: Option<String>,
    pub description: Option<String>,
    pub srs_id: i32,
    pub bounds: [f64; 4],
    pub data_bounds: [f64; 4],
    pub matrices: Vec<TileMatrix>,
    /// A gridded coverage's row; none for a table of pictures.
    pub coverage: Option<NewCoverage>,
    /// Its tiles are WebP: the table is registered with `gpkg_webp`.
    pub webp: bool,
}

/// What a tile's first bytes say it is: `png`, `jpeg`, `webp`, `tiff`, or `other`.
pub fn tile_format(data: &[u8]) -> &'static str {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        "png"
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpeg"
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "webp"
    } else if data.starts_with(b"II*\0") || data.starts_with(b"MM\0*") {
        "tiff"
    } else {
        "other"
    }
}

/// The OGC 17-066r1 extension's definition, as `gpkg_extensions` names it.
const COVERAGE_DEFINITION: &str = "http://docs.opengeospatial.org/is/17-066r1/17-066r1.html";

/// The ancillary tables of a gridded coverage, as GDAL makes them.
const COVERAGE_TABLES: &str = "
CREATE TABLE IF NOT EXISTS gpkg_2d_gridded_coverage_ancillary (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, tile_matrix_set_name TEXT NOT NULL UNIQUE,
  datatype TEXT NOT NULL DEFAULT 'integer', scale REAL NOT NULL DEFAULT 1.0, offset REAL NOT NULL DEFAULT 0.0,
  precision REAL DEFAULT 1.0, data_null REAL, grid_cell_encoding TEXT DEFAULT 'grid-value-is-center', uom TEXT,
  field_name TEXT DEFAULT 'Height', quantity_definition TEXT DEFAULT 'Height',
  CONSTRAINT fk_g2dgtct_name FOREIGN KEY(tile_matrix_set_name) REFERENCES gpkg_tile_matrix_set ( table_name )
  CHECK (datatype in ('integer','float')));
CREATE TABLE IF NOT EXISTS gpkg_2d_gridded_tile_ancillary (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, tpudt_name TEXT NOT NULL, tpudt_id INTEGER NOT NULL,
  scale REAL NOT NULL DEFAULT 1.0, offset REAL NOT NULL DEFAULT 0.0, min REAL DEFAULT NULL, max REAL DEFAULT NULL,
  mean REAL DEFAULT NULL, std_dev REAL DEFAULT NULL,
  CONSTRAINT fk_g2dgtat_name FOREIGN KEY (tpudt_name) REFERENCES gpkg_contents(table_name),
  UNIQUE (tpudt_name, tpudt_id));
";

fn has_table(conn: &rusqlite::Connection, name: &str) -> Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

impl GeoPackage {
    /// The file's tile tables with their tile matrix sets and matrices, in `gpkg_contents`'s order: tables of
    /// pictures and gridded coverages (with their ancillary row; a coverage without one is read as an integer one of
    /// scale 1, offset 0 and no null). A tile table without a row in `gpkg_tile_matrix_set` is left out: it has no
    /// grid to read by.
    pub fn tile_sets(&self) -> Result<Vec<TileSet>> {
        let mut st = self.conn.prepare(
            "SELECT c.table_name, c.identifier, c.description, c.min_x, c.min_y, c.max_x, c.max_y,
                    s.srs_id, s.min_x, s.min_y, s.max_x, s.max_y, c.data_type
             FROM gpkg_contents c JOIN gpkg_tile_matrix_set s ON s.table_name = c.table_name
             WHERE c.data_type IN ('tiles', '2d-gridded-coverage') ORDER BY c.rowid",
        )?;
        let base: Vec<(TileSet, bool)> = st
            .query_map([], |r| {
                let b: [Option<f64>; 4] = [r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?];
                let kind: String = r.get(12)?;
                Ok((
                    TileSet {
                        name: r.get(0)?,
                        identifier: r.get(1)?,
                        description: r.get(2)?,
                        data_bounds: match b {
                            [Some(a), Some(b), Some(c), Some(d)] => Some([a, b, c, d]),
                            _ => None,
                        },
                        srs_id: r.get(7)?,
                        bounds: [r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?],
                        matrices: Vec::new(),
                        coverage: None,
                    },
                    kind == "2d-gridded-coverage",
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let ancillary = has_table(&self.conn, "gpkg_2d_gridded_coverage_ancillary")?;
        let mut out = Vec::with_capacity(base.len());
        for (mut set, coverage) in base {
            if coverage {
                let row = if ancillary {
                    let mut c = self.conn.prepare(
                        "SELECT datatype, scale, offset, precision, data_null, grid_cell_encoding, uom, field_name,
                                quantity_definition
                         FROM gpkg_2d_gridded_coverage_ancillary WHERE tile_matrix_set_name = ?1",
                    )?;
                    let mut rows = c.query_map([&set.name], |r| {
                        Ok(Coverage {
                            datatype: r.get(0)?,
                            scale: r.get(1)?,
                            offset: r.get(2)?,
                            precision: r.get(3)?,
                            data_null: r.get(4)?,
                            grid_cell_encoding: r.get(5)?,
                            uom: r.get(6)?,
                            field_name: r.get(7)?,
                            quantity_definition: r.get(8)?,
                        })
                    })?;
                    rows.next().transpose()?
                } else {
                    None
                };
                set.coverage = Some(row.unwrap_or(Coverage {
                    datatype: "integer".into(),
                    scale: 1.0,
                    offset: 0.0,
                    precision: None,
                    data_null: None,
                    grid_cell_encoding: None,
                    uom: None,
                    field_name: None,
                    quantity_definition: None,
                }));
            }
            let mut m = self.conn.prepare(
                "SELECT zoom_level, matrix_width, matrix_height, tile_width, tile_height, pixel_x_size, pixel_y_size
                 FROM gpkg_tile_matrix WHERE table_name = ?1 ORDER BY zoom_level",
            )?;
            set.matrices = m
                .query_map([&set.name], |r| {
                    Ok(TileMatrix {
                        zoom: r.get(0)?,
                        matrix_width: r.get(1)?,
                        matrix_height: r.get(2)?,
                        tile_width: r.get(3)?,
                        tile_height: r.get(4)?,
                        pixel_x_size: r.get(5)?,
                        pixel_y_size: r.get(6)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            out.push(set);
        }
        Ok(out)
    }

    /// How many tiles a tile table holds at a zoom level, and the formats of its tiles (the first bytes of up to
    /// 64 tiles, sorted by name).
    pub fn tile_census(&self, table: &str, zoom: i64) -> Result<(u64, Vec<&'static str>)> {
        let t = quote(table);
        let n: i64 = self.conn.query_row(
            &format!("SELECT count(*) FROM {t} WHERE zoom_level = ?1"),
            [zoom],
            |r| r.get(0),
        )?;
        let mut st = self.conn.prepare(&format!(
            "SELECT substr(tile_data, 1, 12) FROM {t} WHERE zoom_level = ?1 LIMIT 64"
        ))?;
        let heads: Vec<Vec<u8>> = st
            .query_map([zoom], |r| r.get::<_, Vec<u8>>(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut formats: Vec<&'static str> = Vec::new();
        for h in &heads {
            let f = tile_format(h);
            if !formats.contains(&f) {
                formats.push(f);
            }
        }
        formats.sort_unstable();
        Ok((u64::try_from(n).unwrap_or(0), formats))
    }

    /// The zooms of a table that hold at least one tile, coarsest first: GDAL's overviews are these, a zoom its matrix
    /// lists but no tile fills is none (a file written without overviews; docs/adr/0627 §3).
    pub fn zooms_with_tiles(&self, table: &str) -> Result<Vec<i64>> {
        let t = quote(table);
        let mut st = self.conn.prepare(&format!(
            "SELECT m.zoom_level FROM gpkg_tile_matrix m WHERE m.table_name = ?1
             AND EXISTS (SELECT 1 FROM {t} WHERE zoom_level = m.zoom_level) ORDER BY m.zoom_level"
        ))?;
        let zooms = st
            .query_map([table], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(zooms)
    }

    /// What a gridded coverage's tiles' own scales and offsets allow, in GDAL's words (its GPKG driver's checks, NULL
    /// counting as plain): every tile's scale 1 and offset 0, and every tile's scale 1 and offset 0 or 1 (true for a
    /// table without tile rows).
    pub fn coverage_tile_scales(&self, table: &str) -> Result<(bool, bool)> {
        if !has_table(&self.conn, "gpkg_2d_gridded_tile_ancillary")? {
            return Ok((true, true));
        }
        Ok(self.conn.query_row(
            "SELECT NOT EXISTS (SELECT 1 FROM gpkg_2d_gridded_tile_ancillary WHERE tpudt_name = ?1
                                AND NOT (offset = 0.0 AND scale = 1.0)),
                    NOT EXISTS (SELECT 1 FROM gpkg_2d_gridded_tile_ancillary WHERE tpudt_name = ?1
                                AND NOT ((offset = 0.0 OR offset = 1.0) AND scale = 1.0))",
            [table],
            |r| Ok((r.get::<_, bool>(0)?, r.get::<_, bool>(1)?)),
        )?)
    }

    /// One tile by zoom, column and row, with a gridded coverage's tile scale and offset (`ancillary`; 1 and 0 for a
    /// tile without its row); none when the table holds no such tile.
    pub fn tile(
        &self,
        table: &str,
        zoom: i64,
        column: i64,
        row: i64,
        ancillary: bool,
    ) -> Result<Option<TileBlob>> {
        let t = quote(table);
        let sql = if ancillary && has_table(&self.conn, "gpkg_2d_gridded_tile_ancillary")? {
            format!(
                "SELECT t.tile_data, a.scale, a.offset FROM {t} t
                 LEFT JOIN gpkg_2d_gridded_tile_ancillary a ON a.tpudt_name = ?4 AND a.tpudt_id = t.id
                 WHERE t.zoom_level = ?1 AND t.tile_column = ?2 AND t.tile_row = ?3"
            )
        } else {
            format!(
                "SELECT tile_data, 1.0, 0.0 FROM {t} WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3
                 AND ?4 IS NOT NULL"
            )
        };
        let mut st = self.conn.prepare_cached(&sql)?;
        let mut rows = st.query_map(params![zoom, column, row, table], |r| {
            Ok(TileBlob {
                data: r.get(0)?,
                scale: r.get::<_, Option<f64>>(1)?.unwrap_or(1.0),
                offset: r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// How many tiles a zoom level holds in a box of columns and rows (both ends included).
    pub fn tile_count_in(
        &self,
        table: &str,
        zoom: i64,
        columns: (i64, i64),
        rows: (i64, i64),
    ) -> Result<u64> {
        let t = quote(table);
        let n: i64 = self.conn.query_row(
            &format!(
                "SELECT count(*) FROM {t}
                 WHERE zoom_level = ?1 AND tile_column BETWEEN ?2 AND ?3 AND tile_row BETWEEN ?4 AND ?5"
            ),
            params![zoom, columns.0, columns.1, rows.0, rows.1],
            |r| r.get(0),
        )?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Up to `most` tiles of a zoom level in a box of columns and rows, in row and column order: what a reader looks
    /// at before it reads them all (their formats and sizes).
    pub fn tile_sample(
        &self,
        table: &str,
        zoom: i64,
        columns: (i64, i64),
        rows: (i64, i64),
        most: u32,
    ) -> Result<Vec<(i64, i64, Vec<u8>)>> {
        let t = quote(table);
        let mut st = self.conn.prepare(&format!(
            "SELECT tile_column, tile_row, tile_data FROM {t}
             WHERE zoom_level = ?1 AND tile_column BETWEEN ?2 AND ?3 AND tile_row BETWEEN ?4 AND ?5
             ORDER BY tile_row, tile_column LIMIT ?6"
        ))?;
        let rows = st
            .query_map(
                params![zoom, columns.0, columns.1, rows.0, rows.1, most],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// The tiles of a zoom level in a box of columns and rows (both ends included), in column and row order:
    /// `(column, row, bytes)`.
    pub fn tiles_in(
        &self,
        table: &str,
        zoom: i64,
        columns: (i64, i64),
        rows: (i64, i64),
    ) -> Result<Vec<(i64, i64, Vec<u8>)>> {
        let t = quote(table);
        let mut st = self.conn.prepare(&format!(
            "SELECT tile_column, tile_row, tile_data FROM {t}
             WHERE zoom_level = ?1 AND tile_column BETWEEN ?2 AND ?3 AND tile_row BETWEEN ?4 AND ?5
             ORDER BY tile_row, tile_column"
        ))?;
        let rows = st
            .query_map(params![zoom, columns.0, columns.1, rows.0, rows.1], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// Writes a tile table whole in one transaction: its user table, its contents, tile matrix set and tile matrices
    /// rows, and the tiles `next` gives until it returns none. The file's spatial reference system `set.srs_id` must
    /// be there already ([`GeoPackage::add_srs`]). Nothing is written when any step fails. Returns the tiles written.
    pub fn write_tile_set(
        &mut self,
        set: &NewTileSet,
        next: &mut dyn FnMut() -> Result<Option<NewTile>>,
    ) -> Result<u64> {
        if set.name.is_empty() || set.matrices.is_empty() {
            return Err(Error(
                "Karo tablosunun adı ve en az bir karo matrisi olmalı".into(),
            ));
        }
        let tx = self.conn.transaction()?;
        let t = quote(&set.name);
        tx.execute_batch(&format!(
            "CREATE TABLE {t} (id INTEGER PRIMARY KEY AUTOINCREMENT, zoom_level INTEGER NOT NULL, \
             tile_column INTEGER NOT NULL, tile_row INTEGER NOT NULL, tile_data BLOB NOT NULL, \
             UNIQUE (zoom_level, tile_column, tile_row));"
        ))?;
        let [dx0, dy0, dx1, dy1] = set.data_bounds;
        let data_type = if set.coverage.is_some() {
            "2d-gridded-coverage"
        } else {
            "tiles"
        };
        tx.execute(
            "INSERT INTO gpkg_contents (table_name, data_type, identifier, description, min_x, min_y, max_x, max_y, srs_id)
             VALUES (?1, ?9, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                set.name,
                set.identifier.as_deref().unwrap_or(&set.name),
                set.description.as_deref().unwrap_or(""),
                dx0,
                dy0,
                dx1,
                dy1,
                set.srs_id,
                data_type
            ],
        )?;
        let [x0, y0, x1, y1] = set.bounds;
        tx.execute(
            "INSERT INTO gpkg_tile_matrix_set (table_name, srs_id, min_x, min_y, max_x, max_y) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![set.name, set.srs_id, x0, y0, x1, y1],
        )?;
        for m in &set.matrices {
            tx.execute(
                "INSERT INTO gpkg_tile_matrix (table_name, zoom_level, matrix_width, matrix_height, tile_width, tile_height, pixel_x_size, pixel_y_size)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    set.name,
                    m.zoom,
                    m.matrix_width,
                    m.matrix_height,
                    m.tile_width,
                    m.tile_height,
                    m.pixel_x_size,
                    m.pixel_y_size
                ],
            )?;
        }
        if set.webp || set.coverage.is_some() {
            // A file another writer made may have no extensions table yet.
            tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS gpkg_extensions (
                   table_name TEXT, column_name TEXT, extension_name TEXT NOT NULL, definition TEXT NOT NULL,
                   scope TEXT NOT NULL, CONSTRAINT ge_tce UNIQUE (table_name, column_name, extension_name));",
            )?;
        }
        if set.webp {
            tx.execute(
                "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, 'tile_data', 'gpkg_webp', 'http://www.geopackage.org/spec120/#extension_tiles_webp', 'read-write')",
                [&set.name],
            )?;
        }
        if let Some(c) = &set.coverage {
            // OGC 17-066r1: the ancillary tables, the WGS 84 3D system the extension asks for, the extension's rows.
            tx.execute_batch(COVERAGE_TABLES)?;
            tx.execute(
                "INSERT OR IGNORE INTO gpkg_spatial_ref_sys VALUES ('WGS 84 3D', 4979, 'EPSG', 4979, 'undefined', NULL)",
                [],
            )?;
            for (table, column) in [
                ("gpkg_2d_gridded_coverage_ancillary", None),
                ("gpkg_2d_gridded_tile_ancillary", None),
                (set.name.as_str(), Some("tile_data")),
            ] {
                tx.execute(
                    "INSERT OR IGNORE INTO gpkg_extensions VALUES (?1, ?2, 'gpkg_2d_gridded_coverage', ?3, 'read-write')",
                    params![table, column, COVERAGE_DEFINITION],
                )?;
            }
            tx.execute(
                "INSERT INTO gpkg_2d_gridded_coverage_ancillary (tile_matrix_set_name, datatype, scale, offset, precision,
                   data_null, grid_cell_encoding, uom, field_name, quantity_definition)
                 VALUES (?1, ?2, ?3, ?4, 1.0, ?5, 'grid-value-is-center', ?6, 'Height', 'Height')",
                params![set.name, c.datatype, c.scale, c.offset, c.data_null, c.uom],
            )?;
        }
        let mut count = 0u64;
        {
            let mut ins = tx.prepare(&format!(
                "INSERT INTO {t} (zoom_level, tile_column, tile_row, tile_data) VALUES (?1, ?2, ?3, ?4)"
            ))?;
            let mut stats = match &set.coverage {
                Some(_) => Some(tx.prepare(
                    "INSERT INTO gpkg_2d_gridded_tile_ancillary (tpudt_name, tpudt_id, scale, offset, min, max, mean, std_dev)
                     VALUES (?1, ?2, 1.0, 0.0, ?3, ?4, ?5, ?6)",
                )?),
                None => None,
            };
            while let Some(tile) = next()? {
                let Some(m) = set.matrix(tile.zoom) else {
                    return Err(Error(format!(
                        "{}. zoom düzeyinin karo matrisi yok",
                        tile.zoom
                    )));
                };
                if tile.column < 0
                    || tile.column >= m.matrix_width
                    || tile.row < 0
                    || tile.row >= m.matrix_height
                {
                    return Err(Error(format!(
                        "{}/{}/{} karosu {} × {} matrisin dışında",
                        tile.zoom, tile.column, tile.row, m.matrix_width, m.matrix_height
                    )));
                }
                ins.execute(params![tile.zoom, tile.column, tile.row, tile.data])?;
                if let Some(st) = stats.as_mut() {
                    let s = tile.stats.unwrap_or_default();
                    st.execute(params![
                        set.name,
                        tx.last_insert_rowid(),
                        s.min,
                        s.max,
                        s.mean,
                        s.std_dev
                    ])?;
                }
                count += 1;
            }
        }
        tx.commit()?;
        Ok(count)
    }
}

impl NewTileSet {
    fn matrix(&self, zoom: i64) -> Option<&TileMatrix> {
        self.matrices.iter().find(|m| m.zoom == zoom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Srs;

    fn srs(id: i32) -> Srs {
        Srs {
            name: "Test".into(),
            id,
            organization: "EPSG".into(),
            organization_id: id,
            definition: "PROJCS[\"t\"]".into(),
            description: None,
        }
    }

    fn matrices() -> Vec<TileMatrix> {
        vec![
            TileMatrix {
                zoom: 0,
                matrix_width: 1,
                matrix_height: 1,
                tile_width: 256,
                tile_height: 256,
                pixel_x_size: 2.0,
                pixel_y_size: 2.0,
            },
            TileMatrix {
                zoom: 1,
                matrix_width: 2,
                matrix_height: 1,
                tile_width: 256,
                tile_height: 256,
                pixel_x_size: 1.0,
                pixel_y_size: 1.0,
            },
        ]
    }

    #[test]
    fn a_pyramid_is_written_and_read_back_by_zoom_column_and_row() {
        let dir = std::env::temp_dir().join(format!("kentos-tiles-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.gpkg");
        let _ = std::fs::remove_file(&path);
        let mut gp = GeoPackage::create(&path).unwrap();
        gp.add_srs(&srs(5256)).unwrap();
        let set = NewTileSet {
            name: "orto".into(),
            identifier: Some("Ortofoto".into()),
            description: None,
            srs_id: 5256,
            bounds: [100.0, 0.0, 612.0, 512.0],
            data_bounds: [100.0, 200.0, 500.0, 512.0],
            matrices: matrices(),
            coverage: None,
            webp: false,
        };
        let tiles = vec![
            NewTile {
                zoom: 0,
                column: 0,
                row: 0,
                data: b"\x89PNG\r\n\x1a\n0".to_vec(),
                stats: None,
            },
            NewTile {
                zoom: 1,
                column: 1,
                row: 0,
                data: b"\xFF\xD8\xFF1".to_vec(),
                stats: None,
            },
            NewTile {
                zoom: 1,
                column: 0,
                row: 0,
                data: b"\x89PNG\r\n\x1a\n2".to_vec(),
                stats: None,
            },
        ];
        let mut it = tiles.clone().into_iter();
        let n = gp.write_tile_set(&set, &mut || Ok(it.next())).unwrap();
        assert_eq!(n, 3);
        gp.close().unwrap();
        let gp = GeoPackage::open(&path, false).unwrap();
        let sets = gp.tile_sets().unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].name, "orto");
        assert_eq!(sets[0].identifier.as_deref(), Some("Ortofoto"));
        assert_eq!(sets[0].bounds, [100.0, 0.0, 612.0, 512.0]);
        assert_eq!(sets[0].data_bounds, Some([100.0, 200.0, 500.0, 512.0]));
        assert_eq!(sets[0].matrices, matrices());
        let (count, formats) = gp.tile_census("orto", 1).unwrap();
        assert_eq!((count, formats), (2, vec!["jpeg", "png"]));
        let got = gp.tiles_in("orto", 1, (0, 1), (0, 0)).unwrap();
        assert_eq!(
            got.iter().map(|t| (t.0, t.1)).collect::<Vec<_>>(),
            vec![(0, 0), (1, 0)]
        );
        assert_eq!(got[1].2, tiles[1].data);
        assert!(gp.tiles_in("orto", 1, (5, 5), (0, 0)).unwrap().is_empty());
        drop(gp);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tile_outside_its_matrix_writes_nothing() {
        let dir = std::env::temp_dir().join(format!("kentos-tiles-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.gpkg");
        let _ = std::fs::remove_file(&path);
        let mut gp = GeoPackage::create(&path).unwrap();
        gp.add_srs(&srs(5256)).unwrap();
        let set = NewTileSet {
            name: "orto".into(),
            identifier: None,
            description: None,
            srs_id: 5256,
            bounds: [0.0, 0.0, 512.0, 512.0],
            data_bounds: [0.0, 0.0, 512.0, 512.0],
            matrices: matrices(),
            coverage: None,
            webp: false,
        };
        let mut sent = false;
        let r = gp.write_tile_set(&set, &mut || {
            if sent {
                return Ok(None);
            }
            sent = true;
            Ok(Some(NewTile {
                zoom: 1,
                column: 2,
                row: 0,
                data: vec![1],
                stats: None,
            }))
        });
        assert!(r.is_err());
        assert!(gp.tile_sets().unwrap().is_empty());
        assert!(!gp.table_exists("orto").unwrap());
        drop(gp);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn formats_are_told_by_their_first_bytes() {
        assert_eq!(tile_format(b"\x89PNG\r\n\x1a\nrest"), "png");
        assert_eq!(tile_format(&[0xFF, 0xD8, 0xFF, 0xE0]), "jpeg");
        assert_eq!(tile_format(b"RIFF\0\0\0\0WEBPVP8 "), "webp");
        assert_eq!(tile_format(b"GIF89a"), "other");
        assert_eq!(tile_format(b""), "other");
    }
}
