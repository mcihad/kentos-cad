//! What the sheet mode needs of the drawing to draw a sheet
//! (docs/sheet/design.md §8 `RenderInputs`; the web's `app/sheet/inputs.ts`):
//! the coordinate system with its transverse Mercator parameters (the core
//! works out the meridian convergence itself), the rows of the layer tables
//! (each object's attributes, area and length as the drawing measures them,
//! whether it is inside the map the table names), the coordinate lists'
//! points (the chosen objects', or a layer's), and the legends' rows: the
//! layers their map shows, as the Lejant window lists them
//! (`kentos_native_style::legend`), each symbol drawn as a patch, a line or a
//! marker of its colours (the web draws the symbol's own small picture). The
//! core filters, sorts, lays out and writes them; this only reads the
//! drawing. “Inside the map” is asked of the drawing's index with the ground
//! each map frame covers, which the core gives (`MapPrim.extent`).

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::Entity;
use kentos_domain::{Document, Slot};
use kentos_interaction::{Spatial, Vec2, spatial};
use kentos_native_style::legend::{LegendLayer, LegendSources, legend_layers, legend_of};
use kentos_native_style::library::StyleLibrary;
use kentos_sheet::display::{
    Attribute, CoordPoint, CoordinateInput, CrsInfo, DisplayList, FeatureInput, LegendEntryInput,
    LegendInput, LegendSymbol, LineSymbol, MarkerShape, MarkerSymbol, PatchSymbol, Prim,
    TableInput,
};
use kentos_sheet::geodesy::TmParams;
use kentos_sheet::kinds::{CoordSource, ItemKind, MapLayers, TableSource};
use kentos_sheet::model::{Item, Sheet, SheetBook, VarValue};
use kentos_sheet::style::Stroke;
use serde_json::Value;

/// The host's part of a sheet's inputs: tables, coordinate lists, legends.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SheetData {
    pub tables: Vec<TableInput>,
    pub coordinates: Vec<CoordinateInput>,
    pub legends: Vec<LegendInput>,
}

/// A coordinate system with its transverse Mercator parameters, from the
/// shared registry (fixtures/crs/v1/registry.json, the web's geo/crs.ts).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Registry {
    systems: Vec<System>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct System {
    srid: u32,
    name: String,
    #[serde(default)]
    datum: String,
    ellipsoid: String,
    projection: Option<String>,
    central_meridian: Option<f64>,
    scale_factor: Option<f64>,
    false_easting: Option<f64>,
    false_northing: Option<f64>,
}

/// The CRS registry's systems (`fixtures/crs/v1/registry.json`, the web's geo/crs.ts).
fn system(srid: u32) -> Option<&'static System> {
    static SYSTEMS: std::sync::OnceLock<Vec<System>> = std::sync::OnceLock::new();
    SYSTEMS
        .get_or_init(|| {
            serde_json::from_str::<Registry>(include_str!("../../../fixtures/crs/v1/registry.json"))
                .map(|r| r.systems)
                .unwrap_or_default()
        })
        .iter()
        .find(|s| s.srid == srid)
}

/// The project's system for a sheet PDF's GeoPDF: its WKT (the core's `tm_wkt` from the
/// registry's values) and its EPSG code; none for a system the core cannot invert (not TM).
pub fn pdf_crs(srid: u32) -> Option<kentos_sheet::pdf::PdfCrs> {
    let s = system(srid)?;
    let tm = crs_info(srid)?.tm?;
    Some(kentos_sheet::pdf::PdfCrs {
        wkt: kentos_sheet::pdf::tm_wkt(&kentos_sheet::pdf::TmCrs {
            name: s.name.clone(),
            datum: s.datum.clone(),
            ellipsoid: s.ellipsoid.clone(),
            epsg: Some(srid),
            tm,
        }),
        epsg: Some(srid),
    })
}

pub fn crs_info(srid: u32) -> Option<CrsInfo> {
    let s = system(srid)?;
    // The ellipsoids by name: semi-major axis and inverse flattening (GRS80: IUGG 1979;
    // WGS 84: NIMA TR8350.2; International 1924: Hayford), as the web's ELLIPSOID.
    let ellipsoid = match s.ellipsoid.as_str() {
        "GRS80" => Some((6_378_137.0, 298.257_222_101)),
        "WGS84" => Some((6_378_137.0, 298.257_223_563)),
        "International 1924" => Some((6_378_388.0, 297.0)),
        _ => None,
    };
    let tm = matches!(s.projection.as_deref(), Some("Transverse Mercator" | "UTM"));
    let params = match (tm, s.central_meridian, ellipsoid) {
        (true, Some(cm), Some((a, f))) => Some(TmParams {
            central_meridian: cm,
            scale_factor: s.scale_factor.unwrap_or(1.0),
            false_easting: s.false_easting.unwrap_or(0.0),
            false_northing: s.false_northing.unwrap_or(0.0),
            semi_major: a,
            inverse_flattening: f,
        }),
        _ => None,
    };
    Some(CrsInfo {
        name: s.name.clone(),
        tm: params,
    })
}

/// An attribute's text as the core's value: a plain decimal number is a number (a table's decimals
/// apply to it), as the web's `attrValue` (`/^-?\d+(\.\d+)?$/`).
fn attr_value(text: &str) -> VarValue {
    let t = text.trim();
    let unsigned = t.strip_prefix('-').unwrap_or(t);
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let plain = match unsigned.split_once('.') {
        Some((whole, frac)) => digits(whole) && digits(frac),
        None => digits(unsigned),
    };
    match t.parse::<f64>() {
        Ok(v) if plain && v.is_finite() => VarValue::Number(v),
        _ => VarValue::Text(text.to_owned()),
    }
}

/// A layer a table or a list names: by its id, else by its name (Turkish letters folded).
fn layer_of(doc: &Document, name: &str) -> Option<String> {
    let layers = doc.layers();
    if layers
        .get(name)
        .is_some_and(|l| l.kind == kentos_contracts::LayerNodeType::Layer)
    {
        return Some(name.to_owned());
    }
    let want = kentos_sheet_ui::text::fold(name);
    layers
        .leaves()
        .into_iter()
        .find(|l| kentos_sheet_ui::text::fold(&l.name) == want)
        .map(|l| l.id.clone())
}

/// The items of a sheet and of its master page (both are drawn, both get their inputs).
fn items_of<'a>(book: &'a SheetBook, sheet: &'a Sheet) -> Vec<&'a Item> {
    let master = sheet
        .master
        .as_deref()
        .and_then(|m| book.masters.iter().find(|x| x.id == m));
    master
        .into_iter()
        .flat_map(|m| m.items.iter())
        .chain(sheet.items.iter())
        .collect()
}

/// The ground each map frame covers, by item id (from a list drawn before).
pub fn extents_of(list: Option<&DisplayList>) -> BTreeMap<String, [f64; 4]> {
    list.into_iter()
        .flat_map(|l| l.prims.iter())
        .filter_map(|p| match p {
            Prim::Map(m) => m.extent.map(|e| (m.item.clone(), e)),
            _ => None,
        })
        .collect()
}

/// The drawing and the library, as the legend reads them (the Lejant window's).
struct Sources<'a> {
    doc: &'a Document,
    library: &'a StyleLibrary,
}

impl LegendSources for Sources<'_> {
    fn entities(&self, layer: &str) -> Vec<&Entity> {
        self.doc.by_layer(layer).collect()
    }

    fn symbol(&self, id: &str) -> Option<Value> {
        self.library.symbol(id).cloned()
    }

    fn item_name(&self, id: &str) -> Option<String> {
        self.library.get(id).map(|(i, _)| i.name().to_owned())
    }
}

/// A symbol's colour as the paper draws it: the theme's ink is black on paper.
fn ink(v: Option<&Value>) -> Option<String> {
    match v.and_then(Value::as_str) {
        None | Some("" | "none" | "transparent") => None,
        Some("fg" | "fg-dim" | "label") => Some("#000000".to_owned()),
        Some(c) if c.starts_with('#') => Some(c.to_owned()),
        Some(_) => Some("#000000".to_owned()),
    }
}

/// A line width of a symbol layer in micrometres (mm by default; a pixel a quarter millimetre).
fn width_um(layer: &Value, key: &str) -> i32 {
    let w = layer.get(key).and_then(Value::as_f64).unwrap_or(0.25);
    let mm = match layer.get("unit").and_then(Value::as_str) {
        Some("px") => w * 0.25,
        _ => w,
    };
    ((mm * 1000.0).round() as i32).clamp(50, 3000)
}

/// A legend symbol of the core from a library symbol: its fill and outline, its line, or its marker.
fn symbol_of(symbol: Option<&Value>) -> LegendSymbol {
    let Some(s) = symbol else {
        return LegendSymbol::Patch(PatchSymbol {
            fill: Some("#e6e6e6".into()),
            stroke: Some(Stroke::solid("#000000", 180)),
            hatch: None,
        });
    };
    let layers: Vec<&Value> = s
        .get("layers")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    let of = |t: &str| {
        layers
            .iter()
            .find(|l| l.get("type").and_then(Value::as_str) == Some(t))
    };
    let line = |l: &Value| {
        ink(l.get("color")).map(|c| Stroke {
            dash: l
                .get("dash")
                .and_then(Value::as_array)
                .map(|d| {
                    d.iter()
                        .filter_map(Value::as_f64)
                        .map(|x| (x * 1000.0).round() as i32)
                        .collect()
                })
                .unwrap_or_default(),
            ..Stroke::solid(&c, width_um(l, "width"))
        })
    };
    match s.get("type").and_then(Value::as_str) {
        Some("line") => match of("simpleLine").and_then(|l| line(l)) {
            Some(stroke) => LegendSymbol::Line(LineSymbol { stroke }),
            None => LegendSymbol::Line(LineSymbol {
                stroke: Stroke::solid("#000000", 250),
            }),
        },
        Some("marker") => {
            let m = of("shape").or_else(|| layers.first());
            let shape = match m.and_then(|m| m.get("shape")).and_then(Value::as_str) {
                Some("square" | "diamond") => MarkerShape::Square,
                Some("triangle") => MarkerShape::Triangle,
                Some("cross" | "x" | "plus") => MarkerShape::Cross,
                _ => MarkerShape::Circle,
            };
            let size = m
                .and_then(|m| m.get("size"))
                .and_then(Value::as_f64)
                .unwrap_or(2.0);
            let mm = match m.and_then(|m| m.get("unit")).and_then(Value::as_str) {
                Some("px") => size * 0.25,
                _ => size,
            };
            LegendSymbol::Marker(MarkerSymbol {
                shape,
                size: ((mm * 1000.0).round() as i32).clamp(800, 6000),
                fill: m.and_then(|m| ink(m.get("fill"))),
                stroke: m
                    .and_then(|m| ink(m.get("stroke")))
                    .map(|c| Stroke::solid(&c, m.map_or(250, |m| width_um(m, "strokeWidth")))),
            })
        }
        _ => LegendSymbol::Patch(PatchSymbol {
            fill: of("simpleFill").and_then(|l| ink(l.get("color"))),
            stroke: of("simpleLine").and_then(|l| line(l)),
            hatch: of("hatch")
                .or_else(|| of("lineHatch"))
                .and_then(|l| ink(l.get("color"))),
        }),
    }
}

/// The objects whose boxes touch the ground of a map, or none (no map named, or it is not drawn yet).
fn inside(
    spatial: &Spatial,
    extents: &BTreeMap<String, [f64; 4]>,
    map: Option<&str>,
) -> Option<HashSet<Slot>> {
    let e = extents.get(map?)?;
    Some(
        spatial
            .in_rect(Vec2::new(e[0], e[1]), Vec2::new(e[2], e[3]), true)
            .into_iter()
            .collect(),
    )
}

/// A layer table's rows: every object of the layer with its fields and measures.
fn features(doc: &Document, layer: &str, shown: Option<&HashSet<Slot>>) -> Vec<FeatureInput> {
    doc.by_layer(layer)
        .map(|e| {
            let base = e.base();
            let (area, length) = spatial::measures(e);
            FeatureInput {
                id: base.id.to_string(),
                attributes: base
                    .attrs
                    .iter()
                    .map(|(name, value)| Attribute {
                        name: name.clone(),
                        value: attr_value(value),
                    })
                    .collect(),
                area: area.filter(|a| *a > 0.0),
                length: length.filter(|l| *l > 0.0),
                in_map: shown.is_none_or(|s| s.contains(&Slot(base.id))),
                in_atlas: false,
            }
        })
        .collect()
}

/// A coordinate list's points: one closed figure's corners (with its area), or the points and vertices of all.
fn coordinates_of(item: &str, list: &[&Entity]) -> CoordinateInput {
    if let [Entity::Polygon(p)] = list {
        let (area, _) = spatial::measures(list[0]);
        return CoordinateInput {
            item: item.to_owned(),
            points: p
                .pts
                .iter()
                .map(|v| CoordPoint {
                    name: None,
                    x: v.x,
                    y: v.y,
                    z: None,
                })
                .collect(),
            closed: true,
            area: area.filter(|a| *a > 0.0),
        };
    }
    let mut points = Vec::new();
    for e in list {
        match e {
            Entity::Point(p) => points.push(CoordPoint {
                name: p.base.label.clone().filter(|l| !l.is_empty()),
                x: p.p.x,
                y: p.p.y,
                z: p.z,
            }),
            Entity::Polygon(_) | Entity::Polyline(_) | Entity::Line(_) => {
                for v in spatial::vertices(e) {
                    points.push(CoordPoint {
                        name: None,
                        x: v.x,
                        y: v.y,
                        z: None,
                    });
                }
            }
            _ => {}
        }
    }
    CoordinateInput {
        item: item.to_owned(),
        points,
        closed: false,
        area: None,
    }
}

/// A legend's rows: the layers its map shows, as the Lejant window lists them.
fn legend_entries(
    doc: &Document,
    library: &StyleLibrary,
    layers: &MapLayers,
    shown: Option<&HashSet<Slot>>,
) -> Vec<LegendEntryInput> {
    let tree = doc.layers();
    let leaves: Vec<(LegendLayer<'_>, bool)> = tree
        .leaves()
        .into_iter()
        .filter(|l| match layers {
            MapLayers::List(list) => list.layers.contains(&l.id),
            _ => true,
        })
        .map(|n| {
            (
                LegendLayer {
                    id: &n.id,
                    name: &n.name,
                    style: &n.style,
                },
                tree.is_visible(&n.id),
            )
        })
        .collect();
    let groups = legend_of(&legend_layers(&leaves, true), &Sources { doc, library });
    let mut out = Vec::new();
    for g in groups {
        let parent = tree
            .parent(&g.layer_id)
            .filter(|p| p.kind == kentos_contracts::LayerNodeType::Group)
            .map(|p| p.name.clone());
        let in_map = shown.is_none_or(|s| {
            doc.by_layer(&g.layer_id)
                .any(|e| s.contains(&Slot(e.base().id)))
        });
        let many = g.entries.len() > 1;
        for entry in &g.entries {
            out.push(LegendEntryInput {
                layer: g.layer_id.clone(),
                label: if many {
                    entry.label.clone()
                } else {
                    g.layer_name.clone()
                },
                group: if many {
                    Some(g.layer_name.clone())
                } else {
                    parent.clone()
                },
                symbol: symbol_of(entry.symbol.as_ref()),
                in_map,
                in_atlas: false,
            });
        }
    }
    out
}

/// The host's data for a sheet's tables, coordinate lists and legends.
pub fn sheet_data(
    doc: &Document,
    spatial: &Spatial,
    library: &StyleLibrary,
    selection: &[Slot],
    book: &SheetBook,
    sheet: &Sheet,
    list: Option<&DisplayList>,
) -> SheetData {
    let items = items_of(book, sheet);
    let extents = extents_of(list);
    let mut out = SheetData::default();
    for it in &items {
        match &it.kind {
            ItemKind::Table(t) => {
                if let TableSource::Layer(src) = &t.source
                    && let Some(layer) = layer_of(doc, &src.layer)
                {
                    let shown = inside(spatial, &extents, src.only_in_map.as_deref());
                    out.tables.push(TableInput {
                        item: it.id.clone(),
                        features: features(doc, &layer, shown.as_ref()),
                    });
                }
            }
            ItemKind::CoordinateList(c) => {
                let list: Vec<&Entity> = match &c.source {
                    CoordSource::Selection(_) => {
                        selection.iter().filter_map(|s| doc.get(*s)).collect()
                    }
                    CoordSource::Layer(l) => layer_of(doc, &l.layer)
                        .map(|layer| doc.by_layer(&layer).collect())
                        .unwrap_or_default(),
                };
                out.coordinates.push(coordinates_of(&it.id, &list));
            }
            ItemKind::Legend(l) => {
                let map = l
                    .map
                    .as_deref()
                    .and_then(|m| items.iter().find(|x| x.id == m));
                let layers = match map.map(|m| &m.kind) {
                    Some(ItemKind::Map(m)) => m.layers.clone(),
                    _ => MapLayers::default(),
                };
                let shown = inside(spatial, &extents, l.map.as_deref());
                out.legends.push(LegendInput {
                    item: it.id.clone(),
                    entries: legend_entries(doc, library, &layers, shown.as_ref()),
                });
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_attribute_is_a_number_only_when_it_reads_as_one() {
        assert_eq!(attr_value("12.50"), VarValue::Number(12.5));
        assert_eq!(attr_value("-3"), VarValue::Number(-3.0));
        assert_eq!(attr_value("1245"), VarValue::Number(1245.0));
        assert_eq!(attr_value("12,5"), VarValue::Text("12,5".into()));
        assert_eq!(attr_value("1e3"), VarValue::Text("1e3".into()));
        assert_eq!(attr_value("A-12"), VarValue::Text("A-12".into()));
        assert_eq!(attr_value(""), VarValue::Text(String::new()));
    }

    #[test]
    fn a_transverse_mercator_system_gives_its_parameters() {
        let tm27 = crs_info(5253).expect("TUREF / TM27");
        let p = tm27.tm.expect("TM parameters");
        assert_eq!(
            (p.central_meridian, p.false_easting, p.semi_major),
            (27.0, 500_000.0, 6_378_137.0)
        );
        // A geographic system has none.
        assert!(crs_info(4326).is_some_and(|c| c.tm.is_none()));
        assert!(crs_info(1).is_none());
    }

    #[test]
    fn a_library_symbol_becomes_a_patch_a_line_or_a_marker() {
        let fill: Value = serde_json::json!({"type": "fill", "layers": [
            {"id": "f", "type": "simpleFill", "color": "#E1575933"},
            {"id": "o", "type": "simpleLine", "color": "fg", "width": 0.35, "unit": "mm"}]});
        let LegendSymbol::Patch(p) = symbol_of(Some(&fill)) else {
            panic!("a patch");
        };
        assert_eq!(p.fill.as_deref(), Some("#E1575933"));
        assert_eq!(
            p.stroke.map(|s| (s.color, s.width)),
            Some(("#000000".into(), 350))
        );
        let ring: Value = serde_json::json!({"type": "marker", "layers": [
            {"id": "p", "type": "shape", "shape": "ring", "size": 7, "unit": "px", "stroke": "#E15759", "strokeWidth": 1.3}]});
        assert!(matches!(
            symbol_of(Some(&ring)),
            LegendSymbol::Marker(MarkerSymbol {
                shape: MarkerShape::Circle,
                size: 1750,
                ..
            })
        ));
    }

    /// The sample drawing's parcel layer as a table, its parcels in the map or not.
    #[test]
    fn a_layer_table_lists_its_objects_with_their_fields_and_measures() {
        let app = crate::files_testing::app_with_drawing();
        let doc = &app.document.as_ref().expect("open").model;
        let mut spatial = Spatial::new();
        spatial.reload(doc);
        let layer = doc
            .layers()
            .leaves()
            .into_iter()
            .find(|l| doc.by_layer(&l.id).any(|e| matches!(e, Entity::Polygon(_))))
            .map(|l| l.id.clone())
            .expect("a layer of areas");
        let all = features(doc, &layer, None);
        assert!(!all.is_empty());
        assert!(all.iter().any(|f| f.area.is_some_and(|a| a > 0.0)));
        assert!(all.iter().all(|f| f.in_map));
        // A map far from the drawing holds none of them.
        let far = BTreeMap::from([("m".to_owned(), [0.0, 0.0, 10.0, 10.0])]);
        let shown = inside(&spatial, &far, Some("m"));
        assert!(
            features(doc, &layer, shown.as_ref())
                .iter()
                .all(|f| !f.in_map)
        );
    }
}
