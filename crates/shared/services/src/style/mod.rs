//! A MapLibre style, the subset docs/adr/0208 §9 draws: its sources (vector
//! and raster, by TileJSON address or by templates), its layers in order
//! (background, fill, line, circle, symbol's text, raster; fill-extrusion
//! as a flat fill), each layer's zoom range, filter, paint and layout as
//! expressions. What a style uses that KentOS does not draw is gathered in
//! its notes, not refused.

pub mod build;
pub mod color;
pub mod expr;

use serde_json::Value as Json;

use crate::query::resolve;
use expr::{Expr, Notes};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Vector,
    Raster,
    /// GeoJSON, image, video, raster-dem: not drawn.
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub id: String,
    pub kind: SourceKind,
    /// A TileJSON's address (read for the templates).
    pub url: Option<String>,
    pub tiles: Vec<String>,
    pub min_zoom: u32,
    pub max_zoom: u32,
    pub tile_size: u32,
    pub tms: bool,
    pub attribution: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    Background,
    Fill,
    Line,
    Circle,
    Symbol,
    Raster,
    FillExtrusion,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleLayer {
    pub id: String,
    pub kind: LayerKind,
    pub source: Option<String>,
    pub source_layer: Option<String>,
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub filter: Option<Expr>,
    pub paint: Vec<(String, Expr)>,
    pub layout: Vec<(String, Expr)>,
    pub visible: bool,
}

impl StyleLayer {
    pub fn paint(&self, name: &str) -> Option<&Expr> {
        self.paint.iter().find(|(n, _)| n == name).map(|(_, e)| e)
    }

    pub fn layout(&self, name: &str) -> Option<&Expr> {
        self.layout.iter().find(|(n, _)| n == name).map(|(_, e)| e)
    }

    /// Whether it draws at `zoom` (`minzoom` ≤ zoom < `maxzoom`).
    pub fn shows_at(&self, zoom: f64) -> bool {
        self.visible && zoom >= self.min_zoom && zoom < self.max_zoom
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub name: String,
    pub sources: Vec<Source>,
    pub layers: Vec<StyleLayer>,
    /// What it uses that KentOS does not draw.
    pub notes: Vec<String>,
}

impl Style {
    pub fn source(&self, id: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.id == id)
    }

    /// The vector source its layers draw most: the one a basemap's tiles come from.
    pub fn main_vector(&self) -> Option<&Source> {
        let mut best: Option<(&Source, usize)> = None;
        for s in self.sources.iter().filter(|s| s.kind == SourceKind::Vector) {
            let n = self
                .layers
                .iter()
                .filter(|l| l.source.as_deref() == Some(s.id.as_str()))
                .count();
            if best.is_none_or(|(_, m)| n > m) {
                best = Some((s, n));
            }
        }
        best.map(|(s, _)| s)
    }
}

fn props(v: &Json, notes: &mut Notes) -> Vec<(String, Expr)> {
    v.as_object()
        .map(|o| {
            o.iter()
                .map(|(k, x)| (k.clone(), expr::parse(x, notes)))
                .collect()
        })
        .unwrap_or_default()
}

/// Reads a style (version 8) fetched from `base` (its addresses resolved against it).
pub fn parse(text: &str, base: &str) -> Result<Style, String> {
    let v: Json = crate::caps::json(text)?;
    if !v["layers"].is_array() || !v["sources"].is_object() {
        return Err("Belge bir MapLibre stili değil (sources ve layers yok).".to_owned());
    }
    let mut notes = Notes::default();
    if v["version"].as_u64() != Some(8) {
        notes.add("stilin sürümü 8 değil");
    }
    let mut sources = Vec::new();
    for (id, s) in v["sources"].as_object().into_iter().flatten() {
        let kind = match s["type"].as_str() {
            Some("vector") => SourceKind::Vector,
            Some("raster") => SourceKind::Raster,
            Some(other) => {
                notes.add(format!("“{other}” kaynağı"));
                SourceKind::Other
            }
            None => SourceKind::Other,
        };
        let zoom = |k: &str, d: u32| {
            s[k].as_u64()
                .and_then(|z| u32::try_from(z).ok())
                .unwrap_or(d)
                .min(kentos_contracts::MAX_ZOOM)
        };
        sources.push(Source {
            id: id.clone(),
            kind,
            url: s["url"].as_str().map(|u| resolve(base, u)),
            tiles: s["tiles"]
                .as_array()
                .map(|t| {
                    t.iter()
                        .filter_map(Json::as_str)
                        .map(|u| resolve(base, u))
                        .collect()
                })
                .unwrap_or_default(),
            min_zoom: zoom("minzoom", 0),
            max_zoom: zoom("maxzoom", if kind == SourceKind::Raster { 22 } else { 14 }),
            tile_size: s["tileSize"]
                .as_u64()
                .and_then(|t| u32::try_from(t).ok())
                // The style specification's default for both kinds.
                .unwrap_or(512),
            tms: s["scheme"].as_str() == Some("tms"),
            attribution: s["attribution"].as_str().map(str::to_owned),
        });
    }
    let layers = v["layers"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|l| {
                    let kind = match l["type"].as_str() {
                        Some("background") => LayerKind::Background,
                        Some("fill") => LayerKind::Fill,
                        Some("line") => LayerKind::Line,
                        Some("circle") => LayerKind::Circle,
                        Some("symbol") => LayerKind::Symbol,
                        Some("raster") => LayerKind::Raster,
                        Some("fill-extrusion") => LayerKind::FillExtrusion,
                        Some(other) => {
                            notes.add(format!("“{other}” katmanı"));
                            LayerKind::Other
                        }
                        None => LayerKind::Other,
                    };
                    let layout = props(&l["layout"], &mut notes);
                    let visible = layout
                        .iter()
                        .find(|(n, _)| n == "visibility")
                        .is_none_or(|(_, e)| *e != Expr::Lit(expr::Val::Str("none".into())));
                    StyleLayer {
                        id: l["id"].as_str().unwrap_or("").to_owned(),
                        kind,
                        source: l["source"].as_str().map(str::to_owned),
                        source_layer: l["source-layer"].as_str().map(str::to_owned),
                        min_zoom: l["minzoom"].as_f64().unwrap_or(0.0),
                        max_zoom: l["maxzoom"].as_f64().unwrap_or(24.0),
                        filter: (!l["filter"].is_null())
                            .then(|| expr::filter(&l["filter"], &mut notes)),
                        paint: props(&l["paint"], &mut notes),
                        layout,
                        visible,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Style {
        name: v["name"].as_str().unwrap_or("").to_owned(),
        sources,
        layers,
        notes: notes.0,
    })
}

/// A plain style for vector tiles that come without one (a TileJSON, a
/// template): each of `layers` (its vector layers' names; OpenMapTiles'
/// when none are known) as quiet fills and lines over a paper background,
/// water blue, green ground green, buildings beige, roads white; drawn the
/// same on both platforms.
pub fn basic(
    source: &str,
    template: &str,
    min_zoom: u32,
    max_zoom: u32,
    layers: &[String],
) -> Style {
    const OMT: [&str; 8] = [
        "water",
        "landcover",
        "landuse",
        "park",
        "waterway",
        "transportation",
        "building",
        "boundary",
    ];
    let names: Vec<String> = if layers.is_empty() {
        OMT.iter().map(|n| (*n).to_owned()).collect()
    } else {
        layers.to_vec()
    };
    let colour = |name: &str| -> (&'static str, &'static str) {
        let n = name.to_ascii_lowercase();
        if n.contains("water") || n.contains("ocean") || n.contains("sea") {
            ("#a0c8f0", "#7ab0e0")
        } else if n.contains("park")
            || n.contains("landcover")
            || n.contains("wood")
            || n.contains("grass")
        {
            ("#d8e8c8", "#b8d0a0")
        } else if n.contains("building") {
            ("#e0d6cb", "#c8b8a8")
        } else if n.contains("road") || n.contains("transport") || n.contains("street") {
            ("#f4f0ec", "#ffffff")
        } else if n.contains("boundary") || n.contains("admin") {
            ("#00000000", "#9e9cab")
        } else {
            ("#ebe7e0", "#b8b2a8")
        }
    };
    let mut list = vec![serde_json::json!({
        "id": "background", "type": "background", "paint": {"background-color": "#f8f4f0"}
    })];
    for name in &names {
        let (fill, line) = colour(name);
        list.push(serde_json::json!({
            "id": format!("{name}-fill"), "type": "fill", "source": source, "source-layer": name,
            "filter": ["==", "$type", "Polygon"], "paint": {"fill-color": fill}
        }));
        let roads = name.contains("transport") || name.contains("road");
        list.push(serde_json::json!({
            "id": format!("{name}-line"), "type": "line", "source": source, "source-layer": name,
            "filter": ["==", "$type", "LineString"],
            "paint": {"line-color": line, "line-width": if roads {
                serde_json::json!(["interpolate", ["linear"], ["zoom"], 8, 0.5, 14, 2, 18, 8])
            } else { serde_json::json!(0.8) }}
        }));
    }
    let text = serde_json::json!({
        "version": 8,
        "name": "KentOS sade",
        "sources": {source: {"type": "vector", "tiles": [template], "minzoom": min_zoom, "maxzoom": max_zoom}},
        "layers": list,
    })
    .to_string();
    parse(&text, template).unwrap_or(Style {
        name: String::new(),
        sources: Vec::new(),
        layers: Vec::new(),
        notes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_style_for_tiles_without_one() {
        let s = basic(
            "v",
            "https://x/{z}/{x}/{y}.pbf",
            0,
            14,
            &["water".into(), "roads".into()],
        );
        assert_eq!(
            s.main_vector().map(|v| v.tiles[0].as_str()),
            Some("https://x/{z}/{x}/{y}.pbf")
        );
        assert_eq!(s.layers.len(), 5);
        assert_eq!(s.layers[1].source_layer.as_deref(), Some("water"));
        assert!(s.notes.is_empty(), "{:?}", s.notes);
        assert_eq!(
            basic("v", "https://x/{z}/{x}/{y}.pbf", 0, 14, &[])
                .layers
                .len(),
            17
        );
    }

    #[test]
    fn sources_layers_and_notes() {
        let s = parse(
            r##"{"version":8,"name":"Liberty","sources":{
                "openmaptiles":{"type":"vector","url":"https://tiles.openfreemap.org/planet"},
                "ne2":{"type":"raster","tiles":["https://x/{z}/{x}/{y}.png"],"maxzoom":6,"tileSize":256},
                "dem":{"type":"raster-dem","url":"https://x/dem.json"}},
              "layers":[
                {"id":"background","type":"background","paint":{"background-color":"#f8f4f0"}},
                {"id":"water","type":"fill","source":"openmaptiles","source-layer":"water","filter":["==","$type","Polygon"],"paint":{"fill-color":"#a0c8f0"}},
                {"id":"hill","type":"hillshade","source":"dem"},
                {"id":"poi","type":"symbol","source":"openmaptiles","source-layer":"poi","layout":{"visibility":"none"}}]}"##,
            "https://tiles.openfreemap.org/styles/liberty",
        )
        .expect("reads");
        assert_eq!(s.name, "Liberty");
        assert_eq!(s.main_vector().map(|v| v.id.as_str()), Some("openmaptiles"));
        assert_eq!(
            s.source("ne2").map(|r| (r.kind, r.max_zoom, r.tile_size)),
            Some((SourceKind::Raster, 6, 256))
        );
        assert_eq!(s.layers.len(), 4);
        assert!(!s.layers[3].visible);
        assert!(s.layers[1].filter.is_some());
        assert_eq!(s.notes, vec!["“raster-dem” kaynağı", "“hillshade” katmanı"]);
        assert!(parse("{}", "https://x").is_err());
    }
}
