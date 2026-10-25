//! Labels read from a MapLibre style (docs/adr/0635 §4): its `symbol` layers with a `text-field` (not a text marker)
//! as KentOS's label classes, the layers a KentOS style wrote for one class (`metadata.kentos:label`, one per
//! geometry) as that class again. The text is a `get`, a `to-string`, a `concat` of them and texts, or a `{field}`
//! token string; the size, colour, halo, font's weight and slant, anchor and offset, line placement and spacing,
//! sort key, overlap and zoom range become the class's style.

use serde_json::{Map, Value, json};

use super::{StyleLayer, css_in, filter_in, is_font_marker, round4, scale_of_zoom};
use kentos_formats::sld::{colour_with, filter_text};
use kentos_formats::style_labels::{self as sl, ClassIn, Piece};

/// Whether a style layer is a label (a `symbol` layer with a text that is no text marker).
pub(super) fn is_label(l: &StyleLayer) -> bool {
    l.kind == "symbol" && l.layout.contains_key("text-field") && !is_font_marker(l)
}

/// Layout and paint keys the reader takes.
const TAKEN: [&str; 21] = [
    "text-field",
    "text-font",
    "text-size",
    "text-anchor",
    "text-variable-anchor",
    "text-radial-offset",
    "text-offset",
    "text-justify",
    "symbol-placement",
    "symbol-spacing",
    "symbol-sort-key",
    "text-allow-overlap",
    "visibility",
    "text-color",
    "text-opacity",
    "text-halo-color",
    "text-halo-width",
    "text-max-angle",
    "text-keep-upright",
    "text-ignore-placement",
    "text-padding",
];

fn text_pieces(v: &Value, out: &mut Vec<Piece>) -> Result<(), String> {
    match v {
        Value::String(s) if s.contains('{') => out.extend(sl::template(s)),
        Value::String(s) => out.push(Piece::Text(s.clone())),
        Value::Number(n) => out.push(Piece::Text(n.to_string())),
        Value::Array(a) => match a.first().and_then(Value::as_str) {
            Some("get") if a.len() == 2 => match a[1].as_str() {
                Some(f) => out.push(Piece::Field(f.to_owned())),
                None => return Err("“get”in alanı bir ad değil".into()),
            },
            Some("to-string") if a.len() == 2 => text_pieces(&a[1], out)?,
            Some("concat") => {
                for x in &a[1..] {
                    text_pieces(x, out)?;
                }
            }
            Some(op) => {
                return Err(format!(
                    "metnindeki “{op}” ifadesi KentOS'un etiket metnine çevrilemedi"
                ));
            }
            None => return Err("metni bir ifade değil".into()),
        },
        _ => return Err("metni bir yazı ya da ifade değil".into()),
    }
    Ok(())
}

fn number(l: &StyleLayer, key: &str) -> Option<f64> {
    l.layout
        .get(key)
        .or_else(|| l.paint.get(key))
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
}

/// The geometry a layer's filter says (`Point`, `LineString`, `Polygon`), if one.
fn geometry_of(l: &StyleLayer) -> Option<String> {
    let g = filter_in(l.filter.as_ref()).ok()?.geometry?;
    (g.len() == 1).then(|| g[0].clone())
}

/// The label classes of the style layers that are labels; the font families they name are said once by the caller.
pub(super) fn read(
    layers: &[&StyleLayer],
    lost: &mut Vec<String>,
    families: &mut Vec<String>,
) -> Vec<ClassIn> {
    let mut names: Vec<String> = Vec::new();
    let mut groups: Vec<Vec<&StyleLayer>> = Vec::new();
    for l in layers.iter().copied().filter(|l| is_label(l)) {
        let name = l
            .metadata
            .get("kentos:label")
            .and_then(Value::as_str)
            .map_or_else(|| l.id.clone(), str::to_owned);
        match names.iter().position(|n| *n == name) {
            Some(k) if l.metadata.contains_key("kentos:label") => groups[k].push(l),
            _ => {
                names.push(name);
                groups.push(vec![l]);
            }
        }
    }
    let mut out = Vec::new();
    for (name, group) in names.into_iter().zip(groups) {
        let first = group[0];
        let what = format!("“{name}” etiketi");
        if first.layout.get("visibility") == Some(&json!("none")) {
            lost.push(format!("{what}: gizli etiket katmanı alınmadı"));
            continue;
        }
        let mut pieces = Vec::new();
        if let Err(why) = text_pieces(&first.layout["text-field"], &mut pieces) {
            lost.push(format!("{what}: {why}; alınmadı"));
            continue;
        }
        let pieces: Vec<Piece> = pieces
            .into_iter()
            .filter(|p| !matches!(p, Piece::Text(t) if t.is_empty()))
            .collect();
        if pieces.is_empty() {
            lost.push(format!("{what}: metni boş; alınmadı"));
            continue;
        }
        let when = match filter_in(first.filter.as_ref()) {
            Ok(f) => f.filter.map(|f| filter_text(&f)),
            Err(why) => {
                lost.push(format!("{what}: {why}; alınmadı"));
                continue;
            }
        };
        let mut more = Map::new();
        let size = match first.layout.get("text-size") {
            None => 16.0,
            Some(v) => match v.as_f64() {
                Some(s) => round4(s),
                None => {
                    lost.push(format!(
                        "{what}: yakınlaştırmaya göre yazı boyu KentOS'un etiketinde yok; 16 piksel alındı"
                    ));
                    16.0
                }
            },
        };
        if let Some(font) = first
            .layout
            .get("text-font")
            .and_then(Value::as_array)
            .and_then(|f| f.first())
            .and_then(Value::as_str)
        {
            if font.contains("Bold") || font.contains("Semibold") {
                more.insert("weight".into(), json!(600));
            }
            if font.contains("Italic") || font.contains("Oblique") {
                more.insert("italic".into(), json!(true));
            }
            let family = font
                .split_whitespace()
                .filter(|w| {
                    !matches!(
                        *w,
                        "Regular" | "Bold" | "Italic" | "Semibold" | "Oblique" | "Medium"
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            if !family.is_empty() && !families.contains(&family) {
                families.push(family);
            }
        }
        let opacity = number(first, "text-opacity").unwrap_or(1.0).clamp(0.0, 1.0);
        match first.paint.get("text-color") {
            Some(Value::String(c)) => match css_in(c) {
                Some((rgb, a)) => {
                    more.insert("color".into(), json!(colour_with(&rgb, Some(a * opacity))));
                }
                None => lost.push(format!("{what}: “{c}” rengi okunamadı; siyah alındı")),
            },
            Some(_) => lost.push(format!(
                "{what}: ifadeyle rengi KentOS'un etiketinde yok; siyah alındı"
            )),
            None => {}
        }
        if let Some(w) = number(first, "text-halo-width").filter(|w| *w > 0.0) {
            let c = first
                .paint
                .get("text-halo-color")
                .and_then(Value::as_str)
                .and_then(css_in)
                .map_or_else(|| "#000000".into(), |(rgb, a)| colour_with(&rgb, Some(a)));
            more.insert("halo".into(), json!({ "width": round4(w), "color": c }));
        }
        for l in &group {
            let line = matches!(
                l.layout.get("symbol-placement").and_then(Value::as_str),
                Some("line" | "line-center")
            );
            let geometry = geometry_of(l);
            if line {
                more.insert("line".into(), json!("parallel"));
                if let Some(s) = number(l, "symbol-spacing").filter(|s| *s > 0.0) {
                    more.insert("repeat".into(), json!(round4(s)));
                }
                if let Some(y) = l
                    .layout
                    .get("text-offset")
                    .and_then(Value::as_array)
                    .and_then(|o| o.get(1))
                    .and_then(Value::as_f64)
                    .filter(|y| *y != 0.0)
                {
                    more.insert(
                        "position".into(),
                        json!(if y < 0.0 { "above" } else { "below" }),
                    );
                    more.insert("distance".into(), json!(round4(y.abs() * size)));
                }
            } else if geometry.as_deref() == Some("Polygon") {
                more.insert("area".into(), json!("horizontal"));
            } else {
                let centred = l.layout.get("text-anchor").and_then(Value::as_str) == Some("center")
                    || (!l.layout.contains_key("text-anchor")
                        && !l.layout.contains_key("text-variable-anchor"));
                more.insert(
                    "point".into(),
                    json!(if centred { "center" } else { "around" }),
                );
                if let Some(r) = number(l, "text-radial-offset").filter(|r| *r > 0.0) {
                    more.insert("distance".into(), json!(round4(r * size)));
                }
            }
        }
        if let Some(k) = number(first, "symbol-sort-key") {
            more.insert(
                "priority".into(),
                json!((10.0 - k).round().clamp(0.0, 10.0) as u8),
            );
        }
        if first.layout.get("text-allow-overlap") == Some(&json!(true)) {
            more.insert("overlap".into(), json!("always"));
        }
        if let Some(a) = number(first, "text-max-angle").filter(|a| *a != 45.0 && *a > 0.0) {
            more.insert("maxAngle".into(), json!(a));
        }
        if let Some(z) = first.min_zoom.filter(|z| *z > 0.0) {
            more.insert("minScale".into(), json!(sl::ppm(scale_of_zoom(z))));
        }
        if let Some(z) = first.max_zoom {
            more.insert("maxScale".into(), json!(sl::ppm(scale_of_zoom(z))));
        }
        let mut ignored: Vec<&str> = first
            .layout
            .keys()
            .chain(first.paint.keys())
            .map(String::as_str)
            .filter(|k| !TAKEN.contains(k))
            .collect();
        if !ignored.is_empty() {
            ignored.sort_unstable();
            lost.push(format!(
                "{what}: {} özellikleri alınmadı",
                ignored.join(", ")
            ));
        }
        out.push(sl::class_in(
            &name,
            when,
            size,
            sl::kentos_text(&pieces),
            more,
            lost,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    fn none(_: &str) -> Option<Value> {
        None
    }

    fn classes_json() -> Value {
        json!({ "mode": "rules", "classes": [
            { "name": "Konut", "when": "[tür] = 'konut'", "style": {
                "placement": "beside", "size": 12, "text": "[ada] || '/' || [parsel]", "color": "#aa0000",
                "italic": true, "weight": 600, "halo": { "width": 2, "color": "#ffffff" }, "priority": 8,
                "point": "around", "distance": 3, "minScale": 0.5, "maxScale": 20 } },
            { "name": "Diğer", "style": { "placement": "center", "size": 9, "text": "[ad]" } } ] })
    }

    fn renderer() -> Value {
        json!({ "type": "single", "symbols": { "marker": { "type": "marker", "layers": [
            { "id": "m", "type": "shape", "shape": "circle", "size": 2, "unit": "mm", "fill": "#00ff00" } ] } } })
    }

    fn points() -> kentos_formats::sld::Kinds {
        kentos_formats::sld::Kinds {
            point: true,
            line: false,
            area: false,
        }
    }

    fn same(l: &kentos_formats::style_labels::Labelling) {
        let c = &l.labels["classes"];
        assert_eq!(l.labels["mode"], "rules", "{l:?}");
        assert_eq!(c[0]["name"], "Konut");
        assert_eq!(c[0]["when"], "[tür] = 'konut'");
        let s = &c[0]["style"];
        assert_eq!(s["text"], "[ada] || '/' || [parsel]");
        assert_eq!(s["size"], 12.0);
        assert_eq!(s["color"], "#aa0000");
        assert_eq!(s["italic"], true);
        assert_eq!(s["weight"], 600);
        assert_eq!(s["halo"]["width"], 2.0);
        assert_eq!(s["priority"], 8);
        assert_eq!(s["point"], "around");
        assert_eq!(c[1]["style"]["text"], "[ad]");
        assert_eq!(c[1]["style"]["point"], "center");
    }

    #[test]
    fn labels_go_through_maplibre() {
        let labels = classes_json();
        let r = renderer();
        let layer = crate::LayerIn {
            id: "durak",
            name: "Durak",
            renderer: &r,
            kinds: points(),
            opacity: 1.0,
            source: "durak",
            source_layer: None,
            label: None,
            labels: Some(&labels),
        };
        let w = crate::write("s", &[layer], &Map::new(), None, &none);
        let style = crate::parse(&w.json).expect("parses");
        let all: Vec<&crate::StyleLayer> = style.layers.iter().collect();
        let back = crate::renderer_of(&all).expect("reads");
        let l = back.labels.expect("labels");
        same(&l);
    }
}
