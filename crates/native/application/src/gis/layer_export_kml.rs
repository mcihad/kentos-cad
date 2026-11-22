//! What a KML export's placemarks are drawn with (docs/adr/9112 §4): the layer's colour (one style, as the KML writer
//! always gave a layer's objects), the layer's style, or none.
//!
//! The layer's style: each class of its renderer (a category and “Diğer değerler”, a graduated class, a rule) becomes a
//! KML style made from its symbols, and each placemark takes its class's, by the drawing's own rule (`ClassReader`): an
//! object goes to its category or class when it is switched on, a rule's object to the last drawing rule that takes it.
//! A placemark no switched-on class draws has no style. A renderer of one symbol (and the thematic ones, by their base
//! symbol) gives one style; a layer without a renderer, its colour's.
//!
//! A symbol becomes KML as near as KML goes: an area's first plain, hatched or gradient fill its PolyStyle colour, its
//! first simple line its outline; a line's first simple line its LineStyle; a point's first shape or SVG marker its icon
//! colour and size. Opacities multiply into the colour's alpha; widths in millimetres (the symbols' default unit),
//! points or pixels become pixels. A colour that is not `#rrggbb` (a theme's name, an expression) and a library
//! symbol's reference fall back to the layer's colour.

use kentos_commands::{Entity, ExportFormat, ExportSymbols};
use kentos_formats::kml::PlaceStyle;
use kentos_formats::kml::style::{MM_PER_PIXEL, to_kml};
use kentos_scene_query::style_classes::{ClassReader, classes_kind, style_classes};
use serde_json::Value;

use crate::layer_export::Plan;

/// The geometry the file's placemarks have, as a style is made for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Area,
    Line,
    Point,
}

fn kind_of(plan: &Plan<'_>) -> Kind {
    let name = plan.schema.geometry.as_ref().map(|g| g.type_name.to_ascii_uppercase()).unwrap_or_default();
    if name.contains("POLYGON") {
        Kind::Area
    } else if name.contains("POINT") {
        Kind::Point
    } else {
        Kind::Line
    }
}

fn symbols_of(plan: &Plan<'_>) -> ExportSymbols {
    plan.input.options.as_ref().and_then(|o| o.symbols).unwrap_or_default()
}

/// The fields the classes read when the file takes the layer's style: a data layer's rows are read with them.
pub(crate) fn fields_read(plan: &Plan<'_>) -> Vec<String> {
    if plan.input.format != ExportFormat::Kml || symbols_of(plan) != ExportSymbols::Style {
        return Vec::new();
    }
    plan.node
        .style
        .renderer
        .as_ref()
        .and_then(|r| ClassReader::new(r, crate::expression_unit::reading_unit(plan.settings)))
        .map(|r| r.fields().to_vec())
        .unwrap_or_default()
}

/// The export's KML styles and how each placemark takes one.
pub(crate) struct KmlSymbols {
    pub styles: Vec<PlaceStyle>,
    /// The style of each class, in [`style_classes`]' order; none: a class with nothing to draw with.
    by_class: Vec<Option<usize>>,
    reader: Option<ClassReader>,
    /// Every placemark's style when the renderer has no classes.
    every: Option<usize>,
    layer_name: String,
}

impl KmlSymbols {
    pub(crate) fn of(plan: &Plan<'_>) -> KmlSymbols {
        let kind = kind_of(plan);
        let color = color_style(&plan.node.style.color, kind);
        let mut out = KmlSymbols {
            styles: Vec::new(),
            by_class: Vec::new(),
            reader: None,
            every: None,
            layer_name: plan.node.name.clone(),
        };
        match symbols_of(plan) {
            ExportSymbols::None => {}
            ExportSymbols::Color => {
                out.styles.push(color);
                out.every = Some(0);
            }
            ExportSymbols::Style => {
                let renderer = plan.node.style.renderer.as_ref();
                match renderer.filter(|r| classes_kind(r).is_some()) {
                    Some(r) => {
                        for class in style_classes(r) {
                            let style = class.symbols.as_ref().filter(|_| class.enabled).map(|s| set_style(s, kind, &color));
                            out.by_class.push(style.map(|s| {
                                out.styles.push(s);
                                out.styles.len() - 1
                            }));
                        }
                        out.reader = ClassReader::new(r, crate::expression_unit::reading_unit(plan.settings));
                    }
                    None => {
                        let style = renderer
                            .and_then(|r| r.get("symbols"))
                            .filter(|s| !s.is_null())
                            .map_or_else(|| color.clone(), |s| set_style(s, kind, &color));
                        out.styles.push(style);
                        out.every = Some(0);
                    }
                }
            }
        }
        out
    }

    /// Each object's style (its place in [`KmlSymbols::styles`]), in their order.
    pub(crate) fn styles_of(&self, objects: &[&Entity]) -> Vec<Option<usize>> {
        let Some(reader) = &self.reader else {
            return vec![self.every; objects.len()];
        };
        let shapes: Vec<kentos_geometry_core::entity::Shape> = if reader.reads_geometry() {
            objects.iter().map(|e| crate::geometry::shape(e)).collect()
        } else {
            Vec::new()
        };
        let name = self.layer_name.clone();
        reader
            .drawing_classes(objects, &|_| name.clone(), &|i| shapes.get(i))
            .into_iter()
            .map(|c| c.and_then(|c| self.by_class.get(c).copied().flatten()))
            .collect()
    }
}

/// The style the KML writer gives a layer's objects that have no colour of their own: the layer's colour, a 0.25 mm
/// line, an area's fill two fifths opaque, a point's icon in the colour.
fn color_style(layer_color: &str, kind: Kind) -> PlaceStyle {
    let color = to_kml(layer_color, 0xff).unwrap_or_else(|| "ff000000".to_owned());
    let width = ((0.25 / MM_PER_PIXEL) * 100.0).round().max(1.0) / 100.0;
    PlaceStyle {
        icon: (kind == Kind::Point).then(|| color.clone()),
        icon_scale: None,
        fill: (kind == Kind::Area).then(|| format!("66{}", &color[2..])),
        line: Some((color, width)),
        area: kind == Kind::Area,
    }
}

fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn alpha(opacity: f64) -> u8 {
    (opacity.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// A width in the layer's unit as pixels: millimetres (the default), points or pixels; another unit (the map's)
/// cannot be one and draws one pixel.
fn pixels(width: f64, unit: Option<&str>) -> f64 {
    let px = match unit {
        None | Some("mm") => width / MM_PER_PIXEL,
        Some("pt") => width * 96.0 / 72.0,
        Some("px") => width,
        Some(_) => 1.0,
    };
    let px = if px > 0.0 { px } else { 1.0 };
    (px * 100.0).round() / 100.0
}

/// A symbol set's symbol for the placemarks' geometry as a KML style; the layer's colour's when it gives nothing KML
/// can hold.
fn set_style(set: &Value, kind: Kind, fallback: &PlaceStyle) -> PlaceStyle {
    let key = match kind {
        Kind::Area => "fill",
        Kind::Line => "line",
        Kind::Point => "marker",
    };
    let Some(symbol) = set.get(key).filter(|s| s.get("layers").is_some()) else {
        return fallback.clone();
    };
    let whole = number(symbol, "opacity").unwrap_or(1.0);
    let layers: Vec<&Value> = symbol
        .get("layers")
        .and_then(Value::as_array)
        .map(|l| l.iter().filter(|l| l.get("enabled") != Some(&Value::Bool(false))).collect())
        .unwrap_or_default();
    let colored = |l: &Value, field: &str| {
        let opacity = whole * number(l, "opacity").unwrap_or(1.0);
        l.get(field).and_then(Value::as_str).and_then(|c| to_kml(c, alpha(opacity)))
    };
    let of_type = |types: &[&str]| {
        layers
            .iter()
            .copied()
            .find(|l| l.get("type").and_then(Value::as_str).is_some_and(|t| types.contains(&t)))
    };
    let line = of_type(&["simpleLine"]).and_then(|l| {
        let width = number(l, "width").unwrap_or(0.25);
        colored(l, "color").map(|c| (c, pixels(width, l.get("unit").and_then(Value::as_str))))
    });
    let style = match kind {
        Kind::Area => PlaceStyle {
            icon: None,
            icon_scale: None,
            fill: of_type(&["simpleFill", "hatchFill", "gradientFill"]).and_then(|l| colored(l, "color")),
            line,
            area: true,
        },
        Kind::Line => PlaceStyle { icon: None, icon_scale: None, fill: None, line, area: false },
        Kind::Point => {
            let marker = of_type(&["shape", "svg"]);
            let icon = marker.and_then(|l| colored(l, "fill").or_else(|| colored(l, "color")));
            let icon_scale = marker.and_then(|l| number(l, "size")).map(|mm| {
                let scale = pixels(mm, marker.and_then(|l| l.get("unit")).and_then(Value::as_str)) / 32.0;
                (scale.clamp(0.1, 10.0) * 100.0).round() / 100.0
            });
            PlaceStyle { icon, icon_scale, fill: None, line: None, area: false }
        }
    };
    let empty = style.icon.is_none() && style.fill.is_none() && style.line.is_none();
    if empty { fallback.clone() } else { style }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fallback() -> PlaceStyle {
        color_style("#000000", Kind::Area)
    }

    #[test]
    fn an_areas_fill_and_outline_take_their_colours_opacities_and_width() {
        let set = json!({ "fill": { "type": "fill", "opacity": 0.5, "layers": [
            { "type": "simpleFill", "color": "#d9c27a", "opacity": 0.8 },
            { "type": "simpleLine", "color": "#333333", "width": 0.5 }
        ] } });
        let s = set_style(&set, Kind::Area, &fallback());
        // 0.5 × 0.8 = 0.4 → 0x66; the outline at the symbol's 0.5 → 0x80; 0.5 mm ≈ 1.89 px.
        assert_eq!(s.fill.as_deref(), Some("667ac2d9"));
        assert_eq!(s.line, Some(("80333333".to_owned(), 1.89)));
        assert!(s.area);
    }

    #[test]
    fn a_point_takes_its_markers_fill_and_size_a_line_its_simple_line() {
        let set = json!({
            "marker": { "type": "marker", "layers": [{ "type": "shape", "shape": "circle", "fill": "#ff0000", "size": 4.0 }] },
            "line": { "type": "line", "layers": [{ "type": "simpleLine", "color": "#00ff00", "width": 3, "unit": "px" }] }
        });
        let p = set_style(&set, Kind::Point, &fallback());
        assert_eq!(p.icon.as_deref(), Some("ff0000ff"));
        assert_eq!(p.icon_scale, Some(0.47), "4 mm ≈ 15.1 px of KML's 32");
        let l = set_style(&set, Kind::Line, &fallback());
        assert_eq!(l.line, Some(("ff00ff00".to_owned(), 3.0)));
        assert!(!l.area && l.fill.is_none());
    }

    #[test]
    fn what_kml_cannot_hold_falls_back_to_the_layers_colour() {
        // A theme's colour, an expression and a library reference.
        let set = json!({ "fill": { "type": "fill", "layers": [{ "type": "simpleFill", "color": "ink" }] } });
        assert_eq!(set_style(&set, Kind::Area, &fallback()), fallback());
        let set = json!({ "fill": { "ref": "kitaplik:park" } });
        assert_eq!(set_style(&set, Kind::Area, &fallback()), fallback());
        // A switched-off layer is not taken.
        let set = json!({ "fill": { "type": "fill", "layers": [
            { "type": "simpleFill", "color": "#ffffff", "enabled": false },
            { "type": "simpleFill", "color": "#123456" }
        ] } });
        assert_eq!(set_style(&set, Kind::Area, &fallback()).fill.as_deref(), Some("ff563412"));
    }
}
