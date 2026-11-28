//! A data layer (docs/adr/9101 §2): a layer drawn from a table of a GeoPackage or a database as it is now, its rows
//! never copied into the drawing. The layer keeps where the table is (the driver, the source's address, the table),
//! its key, and its geometry column's type and system as last read; the rows are asked of the source as the view, the
//! table or a tool needs them (plan/veri.md VY-02, VY-03). No password is kept: a database's address names its user,
//! the device keeps the password.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The drivers a data layer may name (docs/adr/9100): a GeoPackage file, a PostGIS database.
pub const DATA_DRIVERS: [&str; 2] = ["gpkg", "postgis"];

/// Longest address, table and column names kept.
pub const MAX_DATA_TEXT: usize = 2048;

/// How the project's data layers' edit sessions go together (docs/adr/9104 §2): which layers open, save, let go and
/// close together, and which are written in one transaction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EditGrouping {
    /// Each layer alone: its changes in a transaction of their own.
    #[default]
    Layer,
    /// The layers of a source (a GeoPackage, a database) together: opened and closed together, written in one
    /// transaction.
    Source,
    /// Every editable data layer together: opened and closed together, written in one transaction a source.
    All,
}

impl EditGrouping {
    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            EditGrouping::Layer => "Her katman ayrı",
            EditGrouping::Source => "Aynı kaynaktakiler birlikte",
            EditGrouping::All => "Bütün katmanlar birlikte",
        }
    }
}

/// The geometry column of a data layer's table as last read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DataGeometry {
    pub column: String,
    /// OGC's name of its type, upper case (`MULTIPOLYGON`, `POINT`, `GEOMETRY` …).
    pub type_name: String,
    /// Its system's EPSG code; absent: undefined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// Its positions have Z; only `true` is written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub z: bool,
    /// Its positions have M; only `true` is written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub m: bool,
}

/// Where a data layer's rows are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DataLayer {
    /// The driver's id: `gpkg` or `postgis`.
    pub driver: String,
    /// A GeoPackage's absolute path; a database's `postgresql://user@host:port/database` (no password).
    pub address: String,
    /// A GeoPackage's path from the project file's folder, `/` between its parts (docs/adr/0506): written on saving
    /// when the file lies on the same disk; read when `address` is not there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub relative: Option<String>,
    /// The table: its name in a file, `schema.table` in a database.
    pub table: String,
    /// The integer key column.
    pub key: String,
    /// The geometry column; absent: a table of records without geometry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<DataGeometry>,
    /// The table's box as its catalogue last said it (minimum east, north, maximum east, north, in the table's system):
    /// what Katmana yakınlaş and Tümünü göster frame without asking the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub extent: Option<[f64; 4]>,
    /// The layer is not edited even where the source allows it (the user's choice); only `true` is written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub read_only: bool,
    /// Its rows are texts (docs/adr/9114 §1): which columns hold their words, height, angle, alignment and width
    /// factor. Drawn as the drawing's texts are; absent: its rows are drawn by its style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<crate::DataText>,
}

fn blank_or_long(s: &str) -> bool {
    s.trim().is_empty() || s.len() > MAX_DATA_TEXT
}

/// OGC's names of geometry types a data layer may read.
pub const DATA_GEOMETRY_TYPES: [&str; 13] = [
    "GEOMETRY",
    "POINT",
    "LINESTRING",
    "POLYGON",
    "MULTIPOINT",
    "MULTILINESTRING",
    "MULTIPOLYGON",
    "GEOMETRYCOLLECTION",
    "CIRCULARSTRING",
    "COMPOUNDCURVE",
    "CURVEPOLYGON",
    "MULTICURVE",
    "MULTISURFACE",
];

impl DataLayer {
    /// What is wrong with it, in Turkish; none when a layer may keep it.
    pub fn problem(&self) -> Option<String> {
        if !DATA_DRIVERS.contains(&self.driver.as_str()) {
            return Some(format!(
                "veri katmanının sürücüsü “{}” olamaz: gpkg ya da postgis",
                self.driver
            ));
        }
        if blank_or_long(&self.address) {
            return Some(format!(
                "veri katmanının kaynağı boş ya da {MAX_DATA_TEXT} karakterden uzun"
            ));
        }
        if self.driver == "postgis" {
            if !(self.address.starts_with("postgresql://")
                || self.address.starts_with("postgres://"))
            {
                return Some("veritabanı adresi postgresql:// ile başlar".into());
            }
            if let Some(p) = crate::address_problem(&self.address) {
                return Some(p);
            }
            if self.relative.is_some() {
                return Some("veritabanının göreli yolu olmaz".into());
            }
        }
        if let Some(r) = &self.relative
            && (blank_or_long(r) || r.contains('\\'))
        {
            return Some("göreli yol boş, çok uzun ya da / yerine \\ ile yazılmış".into());
        }
        if blank_or_long(&self.table) {
            return Some(format!(
                "veri katmanının tablosu boş ya da {MAX_DATA_TEXT} karakterden uzun"
            ));
        }
        if blank_or_long(&self.key) {
            return Some("veri katmanının anahtar sütunu boş".into());
        }
        if let Some(b) = &self.extent
            && (b.iter().any(|v| !v.is_finite()) || b[0] > b[2] || b[1] > b[3])
        {
            return Some(
                "tablonun kutusu dört sonlu sayıdır ve en küçükler en büyüklerden büyük olamaz"
                    .into(),
            );
        }
        if self.extent.is_some() && self.geometry.is_none() {
            return Some("geometrisiz tablonun kutusu olmaz".into());
        }
        if let Some(g) = &self.geometry {
            if blank_or_long(&g.column) {
                return Some("geometri sütununun adı boş".into());
            }
            if !DATA_GEOMETRY_TYPES.contains(&g.type_name.as_str()) {
                return Some(format!(
                    "geometri türü “{}” tanınmıyor (büyük harfle OGC adı: POINT, MULTIPOLYGON …)",
                    g.type_name
                ));
            }
            if g.srid == Some(0) {
                return Some("tanımsız sistemin EPSG kodu yazılmaz".into());
            }
            if g.column == self.key {
                return Some("geometri sütunu anahtar olamaz".into());
            }
        }
        if let Some(t) = &self.text {
            if let Some(p) = t.problem() {
                return Some(p);
            }
            if self.family() != Some(crate::LayerGeometry::Point) {
                return Some(
                    "yazıların tablosunun geometrisi nokta olmalı (POINT ya da MULTIPOINT)".into(),
                );
            }
            let geometry = self.geometry.as_ref().map(|g| g.column.as_str());
            if let Some(c) = t
                .columns()
                .into_iter()
                .find(|c| *c == self.key || Some(*c) == geometry)
            {
                return Some(format!(
                    "“{c}” sütunu hem yazının parçası hem anahtar ya da geometri olamaz"
                ));
            }
        }
        None
    }

    /// What it draws: points, lines, areas or anything (a generic column, a collection); none without geometry.
    pub fn family(&self) -> Option<crate::LayerGeometry> {
        let g = self.geometry.as_ref()?;
        Some(match g.type_name.as_str() {
            "POINT" | "MULTIPOINT" => crate::LayerGeometry::Point,
            "LINESTRING" | "MULTILINESTRING" | "CIRCULARSTRING" | "COMPOUNDCURVE"
            | "MULTICURVE" => crate::LayerGeometry::Line,
            "POLYGON" | "MULTIPOLYGON" | "CURVEPOLYGON" | "MULTISURFACE" => {
                crate::LayerGeometry::Area
            }
            _ => return None,
        })
    }

    /// The kind's name in the interface: “Veri katmanı (alan)”, “Tablo” …
    pub fn kind_words(&self) -> &'static str {
        if self.text.is_some() {
            return "Veri katmanı (yazı)";
        }
        match (self.geometry.is_some(), self.family()) {
            (false, _) => "Tablo",
            (true, Some(crate::LayerGeometry::Point)) => "Veri katmanı (nokta)",
            (true, Some(crate::LayerGeometry::Line)) => "Veri katmanı (çizgi)",
            (true, Some(crate::LayerGeometry::Area)) => "Veri katmanı (alan)",
            (true, _) => "Veri katmanı",
        }
    }

    /// Where it is in words: “kadastro.gpkg › parseller”, “kent@db:5432/kbs › public.parseller”.
    pub fn place_words(&self) -> String {
        let source = if self.driver == "gpkg" {
            self.address
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&self.address)
                .to_owned()
        } else {
            self.address
                .split_once("://")
                .map_or(self.address.as_str(), |(_, r)| r)
                .to_owned()
        };
        format!("{source} › {}", self.table_words())
    }

    /// Its table as the interface says it: a query read as a table (docs/adr/9113 §1: its text begins with
    /// `-- kentos:`) is “SQL sorgusu”, not its text.
    pub fn table_words(&self) -> &str {
        if self.table.starts_with("-- kentos:") {
            "SQL sorgusu"
        } else {
            &self.table
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parcels() -> DataLayer {
        DataLayer {
            driver: "postgis".into(),
            address: "postgresql://ali@db.kent.bel.tr:5432/kbs".into(),
            relative: None,
            table: "public.parseller".into(),
            key: "fid".into(),
            geometry: Some(DataGeometry {
                column: "geom".into(),
                type_name: "MULTIPOLYGON".into(),
                srid: Some(5256),
                z: false,
                m: false,
            }),
            extent: Some([587000.0, 4400000.0, 617000.0, 4430000.0]),
            read_only: false,
            text: None,
        }
    }

    #[test]
    fn a_layer_is_checked_whole() {
        assert_eq!(parcels().problem(), None);
        assert_eq!(parcels().family(), Some(crate::LayerGeometry::Area));
        assert_eq!(parcels().kind_words(), "Veri katmanı (alan)");
        assert_eq!(
            parcels().place_words(),
            "ali@db.kent.bel.tr:5432/kbs › public.parseller"
        );
        let mut p = parcels();
        p.address = "postgresql://ali:gizli@db/kbs".into();
        assert!(p.problem().is_some_and(|w| w.contains("parola")));
        let mut p = parcels();
        p.driver = "oracle".into();
        assert!(p.problem().is_some());
        let mut p = parcels();
        p.geometry = Some(DataGeometry {
            type_name: "Polygon".into(),
            ..p.geometry.clone().expect("geometry")
        });
        assert!(p.problem().is_some());
        let file = DataLayer {
            driver: "gpkg".into(),
            address: "/veri/kadastro.gpkg".into(),
            relative: Some("veri/kadastro.gpkg".into()),
            geometry: None,
            extent: None,
            ..parcels()
        };
        let mut p = parcels();
        p.extent = Some([1.0, 0.0, 0.0, 1.0]);
        assert!(p.problem().is_some());
        assert_eq!(file.problem(), None);
        assert_eq!(file.kind_words(), "Tablo");
        assert_eq!(file.place_words(), "kadastro.gpkg › public.parseller");
    }

    #[test]
    fn a_text_table_is_a_table_of_points() {
        let point = DataGeometry {
            column: "geom".into(),
            type_name: "POINT".into(),
            srid: Some(5256),
            z: false,
            m: false,
        };
        let texts = DataLayer {
            geometry: Some(point.clone()),
            text: Some(crate::DataText::written()),
            ..parcels()
        };
        assert_eq!(texts.problem(), None);
        assert_eq!(texts.kind_words(), "Veri katmanı (yazı)");
        let areas = DataLayer {
            text: Some(crate::DataText::written()),
            ..parcels()
        };
        assert_eq!(
            areas.problem().as_deref(),
            Some("yazıların tablosunun geometrisi nokta olmalı (POINT ya da MULTIPOINT)")
        );
        let none = DataLayer {
            geometry: None,
            text: Some(crate::DataText::written()),
            ..parcels()
        };
        assert!(none.problem().is_some(), "a text needs its point");
        let keyed = DataLayer {
            text: Some(crate::DataText {
                text: "fid".into(),
                ..crate::DataText::written()
            }),
            ..texts.clone()
        };
        assert_eq!(
            keyed.problem().as_deref(),
            Some("“fid” sütunu hem yazının parçası hem anahtar ya da geometri olamaz")
        );
        // Read from the file as written, the mapping kept.
        let json = serde_json::to_value(&texts).expect("json");
        assert_eq!(json["text"]["text"], "metin");
        assert_eq!(
            serde_json::from_value::<DataLayer>(json).expect("read"),
            texts
        );
    }
}
