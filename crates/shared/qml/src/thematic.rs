//! Thematic renderers in QML (docs/adr/0635 §5): KentOS's heat map as QGIS's `heatmapRenderer` (its radius, weight,
//! maximum, quality and colour ramp), its inverted areas as `invertedPolygonRenderer`, its clusters as `pointCluster`
//! and its displaced points as `pointDisplacement` (each wrapping its inner renderer and centre symbol), and back. The
//! other thematic renderers (continuous colour, proportional symbol, bivariate colour, dot density, chart) are named
//! lost by the writer.

use std::fmt::Write as _;

use serde_json::{Map, Value, json};

use super::{
    Node, child, colour_in, esc, kentos_value, num, qgis_value, read_symbol, rgba, set_symbol,
    symbol_xml,
};

/// Pixels a millimetre is at 96 dpi.
const PX_PER_MM: f64 = 96.0 / 25.4;

fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// A distance's QGIS unit: KentOS's screen px or metres.
fn unit_out(renderer: &Value) -> &'static str {
    if renderer.get("unit").and_then(Value::as_str) == Some("m") {
        "RenderMetersInMapUnits"
    } else {
        "Pixel"
    }
}

/// The renderer `inner` (KentOS's JSON) as an element for `key`'s symbols; said when it has none.
fn inner_xml(
    inner: Option<&Value>,
    key: &'static str,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
    what: &str,
) -> (String, usize) {
    let Some(v) = inner.filter(|v| v.is_object()) else {
        lost.push(format!("{what}: iç görünüşü yok; yazılmadı"));
        return (String::new(), 0);
    };
    if matches!(
        v.get("type").and_then(Value::as_str),
        Some("heatmap" | "inverted" | "cluster" | "displacement")
    ) {
        lost.push(format!(
            "{what}: iç görünüşü yine bir tematik görünüş; QML'e yazılmadı"
        ));
        return (String::new(), 0);
    }
    super::element(v, key, library, lost)
}

/// A centre symbol (a marker or a reference to one) as QGIS's `centerSymbol`.
fn centre_xml(
    sym: Option<&Value>,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
    what: &str,
) -> String {
    let Some(s) = sym.filter(|s| s.is_object()) else {
        return String::new();
    };
    let set = json!({ "marker": s });
    set_symbol(Some(&set), "marker", library, lost, what)
        .and_then(|m| symbol_xml(&m, "marker", "centerSymbol", "    ", lost, what))
        .unwrap_or_default()
}

const OPEN: &str = "enableorderby=\"0\" forceraster=\"0\" symbollevels=\"0\" referencescale=\"-1\"";

/// A thematic renderer as QGIS's `<renderer-v2>` element and the classes it writes (one for a heat map).
pub(super) fn element(
    renderer: &Value,
    key: &'static str,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
) -> (String, usize) {
    let num_of = |k: &str| renderer.get(k).and_then(Value::as_f64);
    match renderer.get("type").and_then(Value::as_str).unwrap_or("") {
        "heatmap" => {
            let what = "Isı haritası";
            let radius = num_of("radius").unwrap_or(10.0);
            let unit = if renderer.get("unit").and_then(Value::as_str) == Some("m") {
                1
            } else {
                2
            };
            let weight = match renderer
                .get("weight")
                .and_then(Value::as_str)
                .filter(|w| !w.trim().is_empty())
            {
                Some(w) => match qgis_value(w) {
                    Some(q) => q,
                    None => {
                        lost.push(format!(
                            "{what}: ağırlığın “{w}” ifadesi QGIS'in ifadesine çevrilemedi; ağırlıksız yazıldı"
                        ));
                        String::new()
                    }
                },
                None => String::new(),
            };
            if num_of("opacity").is_some_and(|o| o < 1.0) {
                lost.push(format!(
                    "{what}: saydamlığı QGIS'te katmanın saydamlığıdır; yazılmadı"
                ));
            }
            let ramp: Vec<String> = renderer
                .get("ramp")
                .and_then(Value::as_array)
                .map(|r| r.iter().filter_map(rgba).collect())
                .unwrap_or_default();
            let (c1, c2) = match ramp.as_slice() {
                [] => ("255,255,255,0".to_owned(), "255,0,0,255".to_owned()),
                [one] => (one.clone(), one.clone()),
                all => (all[0].clone(), all[all.len() - 1].clone()),
            };
            let n = ramp.len();
            let stops: Vec<String> = (1..n.saturating_sub(1))
                .map(|k| format!("{};{};rgb;ccw", num(r3(k as f64 / (n - 1) as f64)), ramp[k]))
                .collect();
            let mut s = format!(
                "  <renderer-v2 type=\"heatmapRenderer\" radius=\"{}\" radius_unit=\"{unit}\" weight_expression=\"{}\" max_value=\"{}\" quality=\"{}\" {OPEN}>\n",
                num(radius),
                esc(&weight),
                num(num_of("max").unwrap_or(0.0)),
                num_of("quality").map_or(2, |q| q.round().clamp(1.0, 5.0) as u8),
            );
            s.push_str("    <colorramp name=\"[source]\" type=\"gradient\"><Option type=\"Map\">");
            let _ = write!(
                s,
                "<Option name=\"color1\" type=\"QString\" value=\"{c1}\"/><Option name=\"color2\" type=\"QString\" value=\"{c2}\"/><Option name=\"discrete\" type=\"QString\" value=\"0\"/><Option name=\"rampType\" type=\"QString\" value=\"gradient\"/>"
            );
            if !stops.is_empty() {
                let _ = write!(
                    s,
                    "<Option name=\"stops\" type=\"QString\" value=\"{}\"/>",
                    stops.join(":")
                );
            }
            s.push_str("</Option></colorramp>\n  </renderer-v2>\n");
            (s, 1)
        }
        "inverted" => {
            let what = "Ters alan";
            let single = json!({ "type": "single", "symbols": renderer.get("symbols").cloned().unwrap_or(Value::Null) });
            let (inner, rules) = inner_xml(Some(&single), "fill", library, lost, what);
            if rules == 0 {
                return (String::new(), 0);
            }
            let merge = u8::from(renderer.get("merge") == Some(&Value::Bool(true)));
            (
                format!(
                    "  <renderer-v2 type=\"invertedPolygonRenderer\" preprocessing=\"{merge}\" {OPEN}>\n{inner}  </renderer-v2>\n"
                ),
                rules,
            )
        }
        "cluster" => {
            let what = "Kümeleme";
            let (inner, rules) = inner_xml(renderer.get("renderer"), key, library, lost, what);
            if rules == 0 {
                return (String::new(), 0);
            }
            if renderer.get("count") != Some(&Value::Bool(false)) {
                lost.push(format!(
                    "{what}: kümenin sayısı QGIS'te kümenin simgesindeki yazı işaretidir; yazılmadı"
                ));
            }
            if renderer.get("grow") == Some(&Value::Bool(true)) {
                lost.push(format!(
                    "{what}: sayıyla büyüyen küme simgesi QML'e yazılmadı"
                ));
            }
            let centre = centre_xml(renderer.get("symbol"), library, lost, what);
            (
                format!(
                    "  <renderer-v2 type=\"pointCluster\" tolerance=\"{}\" toleranceUnit=\"{}\" {OPEN}>\n{inner}{centre}  </renderer-v2>\n",
                    num(num_of("distance").unwrap_or(10.0)),
                    unit_out(renderer)
                ),
                rules,
            )
        }
        "displacement" => {
            let what = "Yayma";
            let (inner, rules) = inner_xml(renderer.get("renderer"), key, library, lost, what);
            if rules == 0 {
                return (String::new(), 0);
            }
            let placement = match renderer.get("placement").and_then(Value::as_str) {
                Some("rings") => 1,
                Some("grid") => 2,
                _ => 0,
            };
            let (colour, width) = match renderer.get("circle") {
                Some(c) if c.is_object() => (
                    c.get("color")
                        .and_then(rgba)
                        .unwrap_or_else(|| "128,128,128,255".into()),
                    c.get("width").and_then(Value::as_f64).unwrap_or(0.0),
                ),
                _ => ("128,128,128,0".to_owned(), 0.0),
            };
            let centre = centre_xml(renderer.get("center"), library, lost, what);
            (
                format!(
                    "  <renderer-v2 type=\"pointDisplacement\" tolerance=\"{}\" toleranceUnit=\"{}\" placement=\"{placement}\" circleColor=\"{colour}\" circleWidth=\"{}\" circleRadiusAddition=\"{}\" labelAttributeName=\"\" {OPEN}>\n{inner}{centre}  </renderer-v2>\n",
                    num(num_of("tolerance").unwrap_or(10.0)),
                    unit_out(renderer),
                    num(width),
                    num(num_of("spacing").unwrap_or(0.0)),
                ),
                rules,
            )
        }
        _ => (String::new(), 0),
    }
}

// ─── Reading ──────────────────────────────────────────────────────────────────────────────────────────

fn attr_num(n: Node<'_, '_>, name: &str) -> Option<f64> {
    n.attribute(name)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

/// A distance in QGIS's unit as KentOS's: (value, `px` or `m`); none for another.
fn distance_in(v: f64, unit: Option<&str>) -> Option<(f64, &'static str)> {
    Some(match unit.unwrap_or("MM") {
        "Pixel" => (v, "px"),
        "MM" => (r3(v * PX_PER_MM), "px"),
        "Point" => (r3(v * 96.0 / 72.0), "px"),
        "MapUnit" | "RenderMetersInMapUnits" => (v, "m"),
        _ => return None,
    })
}

/// A ramp's colour: as `colour_in`, but a transparent one kept (`#rrggbb00`, a ramp's clear start).
fn ramp_colour(text: Option<&str>) -> Option<String> {
    let t = text?.trim();
    let p: Vec<u8> = t
        .split(',')
        .take(4)
        .filter_map(|x| x.trim().parse::<u8>().ok())
        .collect();
    if p.len() == 4 && p[3] == 0 {
        return Some(format!("#{:02x}{:02x}{:02x}00", p[0], p[1], p[2]));
    }
    colour_in(Some(t))
}

/// The ramp of QGIS's gradient colour ramp: its first colour, its stops, its last; offsets other than even ones said.
fn ramp_of(r: Node<'_, '_>, lost: &mut Vec<String>) -> Vec<String> {
    let Some(cr) = child(r, "colorramp") else {
        return Vec::new();
    };
    let mut opts = Map::new();
    if let Some(m) = child(cr, "Option") {
        for o in m.children().filter(|c| c.is_element()) {
            if let (Some(n), Some(v)) = (o.attribute("name"), o.attribute("value")) {
                opts.insert(n.to_owned(), json!(v));
            }
        }
    }
    for p in cr
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "prop")
    {
        if let (Some(k), Some(v)) = (p.attribute("k"), p.attribute("v")) {
            opts.entry(k.to_owned()).or_insert(json!(v));
        }
    }
    let colour = |k: &str| {
        opts.get(k)
            .and_then(Value::as_str)
            .and_then(|t| ramp_colour(Some(t)))
    };
    let mut out = Vec::new();
    if let Some(c) = colour("color1") {
        out.push(c);
    }
    let mut uneven = false;
    if let Some(stops) = opts
        .get("stops")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        // QGIS 4 writes a stop's colour with its spec (`r,g,b,a,rgb:…`): a piece not starting with an offset is the
        // colour's tail.
        let mut list: Vec<String> = Vec::new();
        for piece in stops.split(':') {
            let starts = piece
                .split_once(';')
                .is_some_and(|(o, _)| !o.is_empty() && o.trim().parse::<f64>().is_ok());
            match list.last_mut() {
                Some(last) if !starts => {
                    last.push(':');
                    last.push_str(piece);
                }
                _ => list.push(piece.to_owned()),
            }
        }
        let n = list.len() + 1;
        for (k, st) in list.iter().enumerate() {
            let mut parts = st.split(';');
            let off = parts.next().and_then(|o| o.parse::<f64>().ok());
            if let Some(c) = parts.next().and_then(|c| ramp_colour(Some(c))) {
                out.push(c);
            }
            if off.is_some_and(|o| (o - (k + 1) as f64 / n as f64).abs() > 0.001) {
                uneven = true;
            }
        }
    }
    if let Some(c) = colour("color2") {
        out.push(c);
    }
    if uneven {
        lost.push(
            "ısı haritasının renk durakları eşit aralıklı alındı (KentOS'un rampası eşit aralıklıdır)".into(),
        );
    }
    out
}

fn inner_in(r: Node<'_, '_>, lost: &mut Vec<String>, what: &str) -> Result<(Value, usize), String> {
    let Some(inner) = child(r, "renderer-v2") else {
        return Err(format!("{what} görünüşünün iç görünüşü yok."));
    };
    let got = super::renderer_in(inner, false, lost)?;
    if got.0.is_null() {
        return Err(format!("{what} görünüşünün iç görünüşü çizilemiyor."));
    }
    Ok(got)
}

fn centre_in(r: Node<'_, '_>, lost: &mut Vec<String>, what: &str) -> Option<Value> {
    let s = r.children().find(|c| {
        c.is_element()
            && c.tag_name().name() == "symbol"
            && c.attribute("name") == Some("centerSymbol")
    })?;
    read_symbol(s, lost, &format!("{what} simgesi")).map(|(_, v)| v)
}

/// KentOS's renderer of a thematic `<renderer-v2>`, or none when it is not one of the four.
pub(super) fn read(
    r: Node<'_, '_>,
    lost: &mut Vec<String>,
) -> Option<Result<(Value, usize), String>> {
    Some(match r.attribute("type").unwrap_or("") {
        "heatmapRenderer" => {
            let radius = attr_num(r, "radius").unwrap_or(10.0);
            let (radius, unit) = match r.attribute("radius_unit").unwrap_or("0") {
                "2" => (radius, "px"),
                "1" | "6" => (radius, "m"),
                _ => (r3(radius * PX_PER_MM), "px"),
            };
            let mut v = json!({ "type": "heatmap", "radius": radius, "unit": unit,
                "ramp": kentos_formats::style_labels::heat_ramp(ramp_of(r, lost), "Isı haritası", lost) });
            if let Some(w) = r
                .attribute("weight_expression")
                .filter(|w| !w.trim().is_empty())
            {
                match kentos_value(w) {
                    Some(k) => v["weight"] = json!(k),
                    None => lost.push(format!(
                        "ısı haritasının ağırlığının “{w}” ifadesi KentOS'un ifadesine çevrilemedi; ağırlıksız alındı"
                    )),
                }
            }
            if let Some(m) = attr_num(r, "max_value").filter(|m| *m > 0.0) {
                v["max"] = json!(m);
            }
            if let Some(q) = attr_num(r, "quality") {
                v["quality"] = json!(q.round().clamp(1.0, 5.0));
            }
            Ok((v, 1))
        }
        "invertedPolygonRenderer" => inner_in(r, lost, "Ters alan").and_then(|(inner, rules)| {
            if inner["type"] != "single" {
                return Err(
                    "Ters alanın iç görünüşü tek simge değil; KentOS'un ters alanı tek simgelidir."
                        .into(),
                );
            }
            let mut v = json!({ "type": "inverted", "symbols": inner["symbols"] });
            if r.attribute("preprocessing") == Some("1") {
                v["merge"] = json!(true);
            }
            Ok((v, rules))
        }),
        kind @ ("pointCluster" | "pointDisplacement") => {
            let what = if kind == "pointCluster" {
                "Kümeleme"
            } else {
                "Yayma"
            };
            inner_in(r, lost, what).map(|(inner, rules)| {
                let tol = attr_num(r, "tolerance").unwrap_or(10.0);
                let (tol, unit) = distance_in(tol, r.attribute("toleranceUnit")).unwrap_or_else(|| {
                    lost.push(format!("{what}: uzaklığının birimi KentOS'ta yok; piksel alındı"));
                    (tol, "px")
                });
                let centre = centre_in(r, lost, what);
                let mut v = if kind == "pointCluster" {
                    json!({ "type": "cluster", "distance": tol, "unit": unit, "count": false, "renderer": inner })
                } else {
                    let placement = match r.attribute("placement") {
                        Some("1") => "rings",
                        Some("2") => "grid",
                        _ => "ring",
                    };
                    let mut d = json!({ "type": "displacement", "tolerance": tol, "unit": unit,
                        "placement": placement, "renderer": inner });
                    if let Some(sp) = attr_num(r, "circleRadiusAddition").filter(|x| *x > 0.0) {
                        d["spacing"] = json!(sp);
                    }
                    if let Some(c) = colour_in(r.attribute("circleColor")) {
                        d["circle"] = json!({ "color": c, "width": attr_num(r, "circleWidth").unwrap_or(0.0) });
                    }
                    d
                };
                if let Some(c) = centre {
                    v[if kind == "pointCluster" { "symbol" } else { "center" }] = c;
                }
                (v, rules)
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::{Kinds, read, write};

    fn none(_: &str) -> Option<Value> {
        None
    }

    fn marker(c: &str) -> Value {
        json!({ "marker": { "type": "marker", "layers": [
            { "id": "m", "type": "shape", "shape": "circle", "size": 2.0, "unit": "mm", "fill": c } ] } })
    }

    const POINTS: Kinds = Kinds {
        point: true,
        line: false,
        area: false,
    };

    #[test]
    fn a_heat_map_goes_out_and_comes_back() {
        let r = json!({ "type": "heatmap", "radius": 12.0, "unit": "px", "weight": "[w]", "max": 50.0,
            "ramp": ["#0000ff", "#ffff00", "#ff0000"], "quality": 3.0 });
        let w = write(&r, &[], POINTS, &none);
        assert!(w.xml.contains("type=\"heatmapRenderer\""), "{}", w.xml);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        let back = read(&w.xml).expect("reads").renderer;
        assert_eq!(back, r);
    }

    #[test]
    fn wrapping_renderers_go_out_and_come_back() {
        let cluster = json!({ "type": "cluster", "distance": 20.0, "unit": "px", "count": false,
            "renderer": { "type": "single", "symbols": marker("#ff0000") }, "symbol": marker("#00ff00")["marker"] });
        let w = write(&cluster, &[], POINTS, &none);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        let back = read(&w.xml).expect("reads").renderer;
        assert_eq!(back["type"], "cluster");
        assert_eq!(back["distance"], 20.0);
        assert_eq!(back["renderer"]["type"], "single");
        assert!(back["symbol"].is_object(), "{back}");
        let disp = json!({ "type": "displacement", "tolerance": 5.0, "unit": "m", "placement": "rings",
            "spacing": 1.0, "circle": { "color": "#808080", "width": 0.5 },
            "renderer": { "type": "single", "symbols": marker("#ff0000") } });
        let back = read(&write(&disp, &[], POINTS, &none).xml)
            .expect("reads")
            .renderer;
        assert_eq!(
            (back["placement"].as_str(), back["unit"].as_str()),
            (Some("rings"), Some("m"))
        );
        assert_eq!(back["circle"]["color"], "#808080");
        let inv = json!({ "type": "inverted", "merge": true, "symbols": { "fill": { "type": "fill", "layers": [
            { "id": "f", "type": "simpleFill", "color": "#000000" } ] } } });
        let areas = Kinds {
            point: false,
            line: false,
            area: true,
        };
        let back = read(&write(&inv, &[], areas, &none).xml)
            .expect("reads")
            .renderer;
        assert_eq!(back["type"], "inverted");
        assert_eq!(back["merge"], true);
    }
}
