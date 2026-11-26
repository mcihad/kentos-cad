//! A data layer's box and a small picture of its rows as its source has them now (docs/adr/9327). Tümünü göster,
//! Katmana yakınlaştır and Genel bakış ask the hosts for them off the interface's thread: the layer keeps the box its
//! table had when it was added (`DataLayer::extent`), and rows written since (an edit session's save, another program)
//! fall outside it. A layer made empty kept no box at all, and Tümünü göster found nothing to show.
//!
//! - [`data_extent`]: the table's box as the source keeps it (a GeoPackage's contents, which its writes keep; a
//!   database's estimate), measured when it keeps none; in the table's system and the project's.
//! - [`data_outline`]: the rows' geometry only, no field, lines and areas simplified to a part of the box, at most so
//!   many rows, moved into the project's system: what an overview of the whole layer draws, in a store of its own. A
//!   table with more rows than that is drawn as its first rows over its box, so the picture does not end where the
//!   rows read did.

use kentos_commands::{DataLayer, ProjectSettings};
use kentos_data::{Cancel, Coord, Curve, Feature, Geometry, Request};
use kentos_geometry_core::store::Store;

use crate::sources::{TableSystem, extent_in_project};

/// A table's box in its own system and in the project's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableBox {
    pub table: [f64; 4],
    pub project: [f64; 4],
}

/// The rows' outline: a store of their shapes on the layer, how many rows it holds and whether the table had more.
pub struct Outline {
    pub store: Store,
    pub rows: usize,
    pub more: bool,
}

/// Rows read at a time.
const CHUNK: usize = 4096;

fn source_of(data: &DataLayer) -> Result<std::sync::Arc<dyn kentos_data::Source>, String> {
    kentos_data::registry()
        .source(&data.address)
        .map_err(|e| format!("“{}” tablosu açılamadı: {e}", data.table))
}

/// `data`'s table's box now: the source's own, measured when it keeps none; none for a table without geometry or rows.
/// The message of why it could not be read or put in the project's system.
pub fn data_extent(settings: &ProjectSettings, data: &DataLayer) -> Result<Option<TableBox>, String> {
    let Some(geometry) = &data.geometry else {
        return Ok(None);
    };
    let source = source_of(data)?;
    let table = source
        .extent(&data.table, true)
        .map_err(|e| format!("“{}” tablosunun kapsamı okunamadı: {e}", data.table))?;
    let Some(table) = table else {
        return Ok(None);
    };
    let project = extent_in_project(settings, geometry.srid, table)?;
    Ok(Some(TableBox { table, project }))
}

/// `data`'s rows as the layer `layer`'s shapes for a picture `pixels` wide of its box `within`: no field read, lines and
/// areas simplified to one of those pixels, at most `most` rows, moved into the project's system; with more rows than
/// that, the box after them as an area. A row whose geometry has no place in the project's system is left out.
pub fn data_outline(
    settings: &ProjectSettings,
    data: &DataLayer,
    layer: &str,
    within: &TableBox,
    pixels: f64,
    most: u64,
    cancel: &Cancel,
) -> Result<Outline, String> {
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    let system = TableSystem::of(settings, srid)?;
    let source = source_of(data)?;
    let t = within.table;
    let span = (t[2] - t[0]).max(t[3] - t[1]);
    let request = Request {
        columns: Some(Vec::new()),
        geometry: true,
        simplify: (span.is_finite() && span > 0.0 && pixels > 0.0).then(|| span / pixels),
        // One more than kept: whether the table has more.
        limit: Some(most.saturating_add(1)),
        chunk: CHUNK,
        ..Request::default()
    };
    let mut store = Store::new();
    let (mut rows, mut more) = (0usize, false);
    let mut next = 1u32;
    source
        .features(&data.table, &request, cancel, &mut |part: Vec<Feature>| {
            for f in part {
                if rows as u64 >= most {
                    more = true;
                    break;
                }
                let Some(g) = f.shape() else {
                    continue;
                };
                let g = match &system {
                    Some(s) => match s.moved(g) {
                        Some(m) => m,
                        None => continue,
                    },
                    None => g.clone(),
                };
                let Some(e) = crate::geopackage::object_of(&g, layer, next, serde_json::Map::new()) else {
                    continue;
                };
                store.put(f64::from(next), layer, false, crate::geometry::shape(&e));
                next = next.wrapping_add(1);
                rows += 1;
            }
            !more && !cancel.is_cancelled()
        })
        .map_err(|e| format!("“{}” tablosunun satırları okunamadı: {e}", data.table))?;
    if more {
        let [x0, y0, x1, y1] = within.project;
        let ring: Vec<Coord> = [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]
            .into_iter()
            .map(|(x, y)| Coord::xy(x, y))
            .collect();
        let area = Geometry::Polygon(vec![Curve::Line(ring)]);
        if let Some(e) = crate::geopackage::object_of(&area, layer, next, serde_json::Map::new()) {
            store.put(f64::from(next), layer, false, crate::geometry::shape(&e));
        }
    }
    Ok(Outline { store, rows, more })
}
