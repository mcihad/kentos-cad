//! A symbol on a sample object, for its picture (the web's
//! `render/symbolPreview.ts`): the style manager's and the layer style
//! window's thumbnails, the symbol designer's view, the legend. The web
//! paints previews with Canvas2D from the core's primitives; the desktop
//! builds the sample as a styled layer, so the GPU draws the picture with the
//! very pipelines and atlas the drawing uses.
//!
//! The sample is in paper millimetres at a plot scale of 1:1000 (one world
//! unit is one millimetre), centred on the origin, and sized to the picture:
//! a line across it, a rectangle or an area with a hole inside its margins, a
//! point in the middle. A point symbol is fitted by how far it reaches.

use std::collections::BTreeMap;

use kentos_contracts::{Entity, LayerStyle, LineType};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use serde_json::{Value, json};

use crate::StylePalette;
use crate::batches::{DecodeOptions, StyledLayer, decode};
use crate::library::StyleLibrary;
use crate::program::{BuildOptions, build_layer};
use crate::renderer::GeometryClass;

/// Sample values for the attributes MPYY's parametric symbols read (`SAMPLE_ATTRS`).
pub const SAMPLE_ATTRS: [(&str, &str); 10] = [
    ("Kat", "4"),
    ("TAKS", "0.40"),
    ("KAKS", "1.60"),
    ("Emsal", "1.60"),
    ("Yençok", "12.50"),
    ("Ön bahçe", "5"),
    ("Yan bahçe", "3"),
    ("Parsel", "12"),
    ("Ada", "1245"),
    ("Genişlik", "15"),
];

/// The sample object a symbol is shown on (`PreviewGeometry`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Geometry {
    Point,
    Line,
    Bent,
    Area,
    Hole,
}

impl Geometry {
    /// The class of object it is: what a layer's symbol set gives it.
    pub fn class(self) -> GeometryClass {
        match self {
            Geometry::Point => GeometryClass::Marker,
            Geometry::Line | Geometry::Bent => GeometryClass::Line,
            Geometry::Area | Geometry::Hole => GeometryClass::Fill,
        }
    }

    /// The sample that shows a symbol best: a point, a straight line or a rectangle (`defaultGeometry`).
    pub fn of_symbol(symbol: &Value) -> Geometry {
        match symbol.get("type").and_then(Value::as_str) {
            Some("marker") => Geometry::Point,
            Some("line") => Geometry::Line,
            _ => Geometry::Area,
        }
    }
}

/// The sample object in mm, `w` × `h` of paper around the origin (`sampleEntity`).
pub fn sample_entity(kind: Geometry, w: f64, h: f64) -> Option<Entity> {
    let mx = (w * 0.12).min(6.0);
    let my = (h * 0.14).min(5.0);
    let (x0, x1) = (-w / 2.0 + mx, w / 2.0 - mx);
    let (y0, y1) = (-h / 2.0 + my, h / 2.0 - my);
    let v = |x: f64, y: f64| json!({ "x": x, "y": y });
    let mut e = match kind {
        Geometry::Point => json!({ "kind": "point", "p": v(0.0, 0.0) }),
        Geometry::Line => json!({ "kind": "polyline", "pts": [v(x0, 0.0), v(x1, 0.0)] }),
        Geometry::Bent => json!({ "kind": "polyline", "pts": [
            v(x0, y0 * 0.55),
            v(x0 + (x1 - x0) * 0.42, y1 * 0.55),
            v(x1 - (x1 - x0) * 0.18, y0 * 0.2),
            v(x1, y1 * 0.5),
        ] }),
        Geometry::Area => {
            json!({ "kind": "polygon", "pts": [v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y1)] })
        }
        Geometry::Hole => {
            let hx = (x1 - x0) * 0.16;
            let hy = (y1 - y0) * 0.2;
            json!({
                "kind": "polygon",
                "pts": [v(x0, y0), v(x1, y0), v(x1, y1 * 0.35), v(x0 + (x1 - x0) * 0.55, y1), v(x0, y1)],
                "holes": [{ "pts": [
                    v(x0 + hx * 2.0, y0 + hy),
                    v(x0 + hx * 3.4, y0 + hy),
                    v(x0 + hx * 3.4, y0 + hy * 2.4),
                    v(x0 + hx * 2.0, y0 + hy * 2.4),
                ] }],
            })
        }
    };
    let attrs: BTreeMap<&str, &str> = SAMPLE_ATTRS.into_iter().collect();
    if let Some(o) = e.as_object_mut() {
        o.insert("id".into(), json!(1));
        o.insert("layerId".into(), json!("preview"));
        o.insert("attrs".into(), json!(attrs));
    }
    serde_json::from_value(e).ok()
}

/// A preview's style unit: the text box's height to its size (`TEXT_BOX`).
const TEXT_BOX: f64 = 1.25;

/// How far a point symbol reaches from its point, in mm: its markers, their
/// offsets, a text's rough width (`markerReach`); from the core's
/// primitives of the symbol on the sample.
fn marker_reach(symbol: &Value, sample: &Entity) -> f64 {
    let args = json!([{
        "symbol": symbol,
        "entity": sample,
        "layerName": "Önizleme",
        "kindLabel": "Nokta",
        "vertices": 1,
        "plotScale": 1000,
        "assets": {},
    }]);
    let Ok(out) = kentos_style_core::api::run_named("styleCompile", &args.to_string()) else {
        return 0.0;
    };
    let Ok(out) = serde_json::from_str::<Value>(&out) else {
        return 0.0;
    };
    let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let mut reach: f64 = 0.0;
    for m in out["markers"].as_array().into_iter().flatten() {
        let s = &m["style"];
        let common = &s["common"];
        if common["unit"] == "px" {
            continue;
        }
        let size = num(s, "size");
        let kind = s["kind"].as_str().unwrap_or("");
        let text_len = s["text"].as_str().map_or(0, |t| t.encode_utf16().count()) as f64;
        let w = if kind == "text" {
            size * 0.6 * text_len
        } else {
            size
        };
        let h = match kind {
            "shape" => Some(num(s, "height")).filter(|h| *h != 0.0).unwrap_or(size),
            "text" => size * TEXT_BOX,
            _ => size,
        };
        let offset = common["offset"].as_array();
        let at = |i: usize| {
            offset
                .and_then(|o| o.get(i))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        };
        reach = reach.max(at(0).hypot(at(1)) + w.max(h) / 2.0);
    }
    reach
}

/// A symbol's picture as a styled layer, and the scale it is drawn at.
#[derive(Debug)]
pub struct Preview {
    pub layer: StyledLayer,
    /// Logical pixels per paper millimetre (the camera's scale; the sample is centred on the origin).
    pub px_per_mm: f64,
}

/// The symbol `symbol` (resolved: not a library reference) on the sample
/// `geometry` (the symbol's own kind by default), for a picture `w` × `h`
/// logical pixels. `px_per_mm`: a fixed scale; by default a sample of
/// 36 × 22 mm fits the picture, and a point symbol is fitted by its reach.
pub fn preview(
    symbol: &Value,
    geometry: Option<Geometry>,
    size: (f64, f64),
    px_per_mm: Option<f64>,
    palette: &StylePalette,
    library: &StyleLibrary,
) -> Result<Preview, String> {
    let (w, h) = size;
    let kind = geometry.unwrap_or_else(|| Geometry::of_symbol(symbol));
    let mut k = px_per_mm.unwrap_or_else(|| (w / 36.0).min(h / 22.0));
    if px_per_mm.is_none() && kind == Geometry::Point {
        let sample = sample_entity(kind, w / k, h / k).ok_or("Önizleme nesnesi kurulamadı.")?;
        let reach = marker_reach(symbol, &sample);
        if reach > 0.0 {
            k = (w.min(h) * 0.4 / reach).clamp(1.5, 30.0);
        }
    }
    let sample = sample_entity(kind, w / k, h / k).ok_or("Önizleme nesnesi kurulamadı.")?;
    let mut store = Store::new();
    store.put_many(std::iter::once((1.0, "preview", false, shape(&sample))));
    let style = LayerStyle {
        color: "fg".into(),
        line_type: LineType::Continuous,
        line_weight: 0.25,
        fill: None,
        point: None,
        label: None,
        pick_interior: None,
        renderer: Some(json!({ "type": "single", "symbols": { kind.class().key(): symbol } })),
        labels: None,
    };
    let opts = BuildOptions {
        origin: Vec2::new(0.0, 0.0),
        plot_scale: 1000.0,
        screen: false,
        hairlines: false,
        clip: None,
        library,
        layer_name: &|_| "Önizleme".to_owned(),
        view: Default::default(),
        frame: None,
    };
    let (_, batches) = build_layer(&store, &style, &[&sample], &opts)?;
    let layer = decode(
        batches,
        &DecodeOptions {
            palette,
            plot_scale: 1000.0,
            library,
            view: Default::default(),
        },
    )?;
    Ok(Preview {
        layer,
        px_per_mm: k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> StylePalette {
        StylePalette {
            fg: "#1E2833".into(),
            fg_dim: "#5E6B78".into(),
            ink: "#000000".into(),
            paper: "#F8F9FA".into(),
        }
    }

    #[test]
    fn a_point_symbol_is_fitted_by_its_reach() {
        let library = StyleLibrary::default();
        let circle = json!({ "type": "marker", "layers": [
            { "id": "0", "type": "shape", "shape": "circle", "size": 2.4, "fill": "ink" },
        ] });
        let p = preview(&circle, None, (64.0, 36.0), None, &palette(), &library).expect("builds");
        // 40 % of the picture's short side over the circle's radius.
        assert!(
            (p.px_per_mm - 36.0 * 0.4 / 1.2).abs() < 1e-9,
            "{}",
            p.px_per_mm
        );
        assert!(!p.layer.batches.is_empty());
    }

    #[test]
    fn an_area_symbol_fills_a_sample_of_36_by_22_mm() {
        let library = StyleLibrary::default();
        let fill = json!({ "type": "fill", "layers": [
            { "id": "0", "type": "simpleFill", "color": "#C9D6E3" },
            { "id": "1", "type": "simpleLine", "color": "ink", "width": 0.2 },
        ] });
        let p = preview(&fill, None, (64.0, 36.0), None, &palette(), &library).expect("builds");
        assert!((p.px_per_mm - (64.0_f64 / 36.0).min(36.0 / 22.0)).abs() < 1e-12);
        assert!(p.layer.batches.len() >= 2, "a fill and its edge");
        for g in [
            Geometry::Line,
            Geometry::Bent,
            Geometry::Hole,
            Geometry::Point,
        ] {
            assert!(sample_entity(g, 40.0, 22.0).is_some(), "{g:?}");
        }
    }
}
