//! Bilgi on a local raster (docs/adr/9137): a click of Bilgi asks the shown rasters under the point for their cell, off
//! the interface's thread, as Koordinat oku does. Each cell says its raster's layer, every band's value as the file
//! holds it (the file's no-data value said as no value), the cell's column and row and the point's coordinates (Y, X).
//! On no object of the drawing the first raster's cell is the card; on an object or a data layer's row the card offers
//! the raster's value beside it and back. A click outside every shown raster is said.

use kentos_commands::Entity;
use kentos_interaction::Vec2;

use crate::app::App;

/// A raster's cell under a click.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// The raster's layer, by its name.
    pub layer: String,
    /// Each band's value; none where the file holds its no-data value.
    pub values: Vec<Option<f64>>,
    pub column: i64,
    pub row: i64,
    pub at: Vec2,
    /// The values' unit, when the file says it.
    pub units: Option<String>,
    /// The file holds its values as 32-bit floats: shown as such, not as their double's every digit.
    pub float32: bool,
}

/// A raster to ask: its layer's name, its tiles' key and origin, the cell, its unit.
pub(crate) type Wanted = (String, String, super::tiles::Origin, i64, i64, Option<String>, bool);

/// The cells read from the files, the asked order kept; a cell the file cannot give is left out.
pub(crate) fn read(wanted: Vec<Wanted>, at: Vec2) -> Vec<Cell> {
    wanted
        .into_iter()
        .filter_map(|(layer, key, origin, column, row, units, float32)| {
            let (values, nodata) = super::tiles::cell_at(&key, origin, column, row)?;
            let values = values
                .into_iter()
                .map(|v| (!v.is_nan() && nodata.is_none_or(|n| v != n)).then_some(v))
                .collect();
            Some(Cell { layer, values, column, row, at, units, float32 })
        })
        .collect()
}

impl App {
    /// The shown rasters under `at`, the top one first, to ask; and whether any raster is shown at all (a click outside
    /// all of them is said).
    pub(crate) fn rasters_under(&self, at: Vec2) -> (Vec<Wanted>, bool) {
        let Some(doc) = &self.document else {
            return (Vec::new(), false);
        };
        let folder = doc.path.as_deref().and_then(std::path::Path::parent).map(std::path::Path::to_path_buf);
        let layers = doc.model.layers();
        let mut shown = false;
        let mut wanted = Vec::new();
        for e in doc.model.entities() {
            let Entity::Raster(r) = e else {
                continue;
            };
            if !layers.is_visible(&r.base.layer_id) {
                continue;
            }
            shown = true;
            let x = &r.raster;
            let Some(q) =
                kentos_geometry_core::geom::raster::pixel_of(x.affine, kentos_geometry_core::vec2::Vec2::new(at.x, at.y))
            else {
                continue;
            };
            if !(q.x >= 0.0 && q.y >= 0.0 && q.x < f64::from(x.width) && q.y < f64::from(x.height)) {
                continue;
            }
            let key = super::key_of(x);
            if let Some(origin) = super::origin_of(&key, &doc.model, folder.as_deref()) {
                // The files read here name no unit for a band's values: none is said rather than one guessed.
                let units: Option<String> = None;
                let float32 = x.sample == kentos_commands::RasterSample::F32;
                wanted.push((
                    self.layer_name(&r.base.layer_id),
                    key,
                    origin,
                    q.x.floor() as i64,
                    q.y.floor() as i64,
                    units,
                    float32,
                ));
            }
        }
        // The top raster first, as the map shows them.
        wanted.reverse();
        (wanted, shown)
    }

    /// The raster card of the first cell: its layer, its bands' values, its cell and the point.
    pub(crate) fn raster_card(&self) -> Option<(crate::object_info::ObjectCard, Vec2)> {
        let cell = self.gis.data_info.cells.first()?;
        let f = self
            .document
            .as_ref()
            .map_or_else(kentos_interaction::Format::default, |d| kentos_interaction::Format::of(d.model.settings()));
        let unit = cell.units.as_deref().map(|u| format!(" {u}")).unwrap_or_default();
        let mut rows: Vec<(String, String)> = cell
            .values
            .iter()
            .enumerate()
            .map(|(b, v)| {
                (
                    format!("Bant {}", b + 1),
                    v.map_or_else(
                        || "değer yok (NoData)".to_owned(),
                        // A 32-bit float's shortest digits that give it back: the value the file holds.
                        |v| {
                            let shown = if cell.float32 { (v as f32).to_string() } else { crate::crs::js_number(v) };
                            format!("{shown}{unit}")
                        },
                    ),
                )
            })
            .collect();
        rows.push(("Hücre".to_owned(), format!("sütun {}, satır {}", cell.column, cell.row)));
        rows.push(("Konum".to_owned(), format!("Y {} · X {}", f.coord(cell.at.x), f.coord(cell.at.y))));
        let card = crate::object_info::ObjectCard {
            title: format!("Raster: {}", cell.layer),
            rows,
            ..Default::default()
        };
        Some((card, cell.at))
    }
}

#[cfg(test)]
#[path = "cell_info_tests.rs"]
mod cell_info_tests;
