//! QGIS's layer styles in a GeoPackage (docs/adr/0627 §7): the `layer_styles` table QGIS makes and reads, an attribute
//! table of `gpkg_contents`, each row a style's QML and SLD for a table; the default one (`useAsDefault`, else the
//! latest) is the look QGIS opens the table's layer with. Not an OGC extension: QGIS's own table, which GDAL keeps.

use rusqlite::params;

use crate::{GeoPackage, Result};

/// QGIS's table of layer styles.
pub const STYLES_TABLE: &str = "layer_styles";

/// QGIS 4.2's `layer_styles`, as it makes it.
const STYLES_DDL: &str = "CREATE TABLE \"layer_styles\" ( \"id\" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, \
     \"f_table_catalog\" TEXT(256), \"f_table_schema\" TEXT(256), \"f_table_name\" TEXT(256), \
     \"f_geometry_column\" TEXT(256), \"styleName\" TEXT(30), \"styleQML\" TEXT, \"styleSLD\" TEXT, \
     \"useAsDefault\" BOOLEAN, \"description\" TEXT, \"owner\" TEXT(30), \"ui\" TEXT(30), \
     \"update_time\" DATETIME DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')))";

/// A style to write: its table and the table's geometry column, its name, QML and SLD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewStyle {
    pub table: String,
    pub geometry: String,
    pub name: String,
    pub qml: String,
    pub sld: Option<String>,
}

impl GeoPackage {
    /// Adds each style as its table's default, in one transaction, `layer_styles` made as QGIS makes it (and listed in
    /// `gpkg_contents`) when the file has none; a table's earlier default is no longer one.
    pub fn add_default_styles(&mut self, styles: &[NewStyle]) -> Result<()> {
        let tx = self.conn.transaction()?;
        let have: i64 = tx.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [STYLES_TABLE],
            |r| r.get(0),
        )?;
        if have == 0 {
            tx.execute_batch(STYLES_DDL)?;
            tx.execute(
                "INSERT INTO gpkg_contents (table_name, data_type, identifier, description, last_change)
                 VALUES (?1, 'attributes', ?1, '', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                [STYLES_TABLE],
            )?;
        }
        for s in styles {
            tx.execute(
                "UPDATE layer_styles SET useAsDefault = 0 WHERE f_table_name = ?1",
                [&s.table],
            )?;
            tx.execute(
                "INSERT INTO layer_styles (f_table_catalog, f_table_schema, f_table_name, f_geometry_column, styleName,
                   styleQML, styleSLD, useAsDefault, description, owner, ui)
                 VALUES ('', '', ?1, ?2, ?3, ?4, ?5, 1, 'KentOS', '', NULL)",
                params![s.table, s.geometry, s.name, s.qml, s.sld],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Each table's default style as QGIS picks it (marked default first, then the latest written), its QML; none
    /// when the file has no `layer_styles` or a row has no QML.
    pub fn default_styles(&self) -> Result<Vec<(String, String)>> {
        let have: i64 = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [STYLES_TABLE],
            |r| r.get(0),
        )?;
        if have == 0 {
            return Ok(Vec::new());
        }
        let mut st = self.conn.prepare(
            "SELECT f_table_name, styleQML FROM layer_styles
             WHERE styleQML IS NOT NULL AND f_table_name IS NOT NULL
             ORDER BY f_table_name, CASE WHEN useAsDefault THEN 0 ELSE 1 END, update_time DESC, id DESC",
        )?;
        let rows: Vec<(String, String)> = st
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut out: Vec<(String, String)> = Vec::new();
        for (table, qml) in rows {
            if out.last().is_none_or(|(t, _)| *t != table) {
                out.push((table, qml));
            }
        }
        Ok(out)
    }
}
