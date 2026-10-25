//! Heat maps in a MapLibre style (docs/adr/0635 §5): KentOS's heat map as a `heatmap` layer (its radius in pixels or
//! as metres growing with the zoom, its weight a field's number divided by its maximum, its ramp on the density, its
//! opacity), and such a layer read back.

use serde_json::{Map, Value, json};

use super::{Emit, Len, Prop, StyleLayer, all_of, css, css_in, field_of, n, prop_of, round4};
use kentos_formats::sld::colour_with;

/// The heat map of `emit`'s layer as a `heatmap` style layer; one written, or none.
pub(super) fn write(emit: &mut Emit<'_>, lost: &mut Vec<String>) -> usize {
    let layer = emit.layer;
    let r = layer.renderer;
    let what = "Isı haritası";
    if !layer.kinds.point {
        lost.push(format!(
            "{what}: MapLibre'nin ısı haritası noktaların; katmanın noktası yok, yazılmadı"
        ));
        return 0;
    }
    let radius = r.get("radius").and_then(Value::as_f64).unwrap_or(10.0);
    let unit = if r.get("unit").and_then(Value::as_str) == Some("m") {
        "m"
    } else {
        "px"
    };
    let mut paint = Map::new();
    paint.insert("heatmap-radius".into(), Len::of(radius, unit).json());
    if let Some(w) = r
        .get("weight")
        .and_then(Value::as_str)
        .filter(|w| !w.trim().is_empty())
    {
        match field_of(w) {
            Some(f) => {
                let get = json!(["to-number", ["get", f], 0]);
                let weight = match r.get("max").and_then(Value::as_f64).filter(|m| *m > 0.0) {
                    Some(m) => json!(["/", get, n(m)]),
                    None => get,
                };
                paint.insert("heatmap-weight".into(), weight);
            }
            None => lost.push(format!(
                "{what}: ağırlığın “{w}” ifadesi bir alan değil; MapLibre'nin ağırlığına çevrilemedi, ağırlıksız yazıldı"
            )),
        }
    }
    let ramp: Vec<String> = r
        .get("ramp")
        .and_then(Value::as_array)
        .map(|c| c.iter().filter_map(css).collect())
        .unwrap_or_default();
    if ramp.len() >= 2 {
        let k = (ramp.len() - 1) as f64;
        let mut color = vec![
            json!("interpolate"),
            json!(["linear"]),
            json!(["heatmap-density"]),
        ];
        for (i, c) in ramp.iter().enumerate() {
            color.push(n(round4(i as f64 / k)));
            color.push(json!(c));
        }
        paint.insert("heatmap-color".into(), Value::Array(color));
    }
    let opacity = r.get("opacity").and_then(Value::as_f64).unwrap_or(1.0) * layer.opacity;
    if opacity < 1.0 {
        paint.insert("heatmap-opacity".into(), n(round4(opacity.clamp(0.0, 1.0))));
    }
    let id = format!("{}/{}", layer.id, emit.layers.len() + 1);
    let mut l = Map::new();
    l.insert("id".into(), json!(id));
    l.insert("type".into(), json!("heatmap"));
    l.insert("source".into(), json!(layer.source));
    if let Some(s) = layer.source_layer {
        l.insert("source-layer".into(), json!(s));
    }
    let geometry = emit
        .mixed
        .then(|| json!(["==", ["geometry-type"], "Point"]));
    if let Some(f) = all_of(geometry.into_iter().collect()) {
        l.insert("filter".into(), f);
    }
    l.insert("paint".into(), Value::Object(paint));
    let mut meta = json!({ "kentos:layer": layer.id, "kentos:renderer": "heatmap" });
    if let Some(q) = r.get("quality").and_then(Value::as_f64) {
        meta["kentos:quality"] = json!(q);
    }
    l.insert("metadata".into(), meta);
    emit.layers.push(Value::Object(l));
    if unit == "m" {
        emit.zoomed = true;
    }
    1
}

/// A field's number of a weight: `["get", f]`, its `to-number`, or that divided by a number; (field, max).
fn weight_of(v: &Value) -> Option<(String, Option<f64>)> {
    let a = v.as_array()?;
    match a.first()?.as_str()? {
        "get" if a.len() == 2 => Some((a[1].as_str()?.to_owned(), None)),
        "to-number" if a.len() >= 2 => weight_of(&a[1]).filter(|(_, m)| m.is_none()),
        "/" if a.len() == 3 => {
            let m = a[2].as_f64().filter(|m| *m > 0.0)?;
            weight_of(&a[1])
                .filter(|(_, x)| x.is_none())
                .map(|(f, _)| (f, Some(m)))
        }
        _ => None,
    }
}

/// KentOS's heat map of a `heatmap` style layer.
pub(super) fn read(l: &StyleLayer, lost: &mut Vec<String>) -> Value {
    let what = format!("“{}” ısı haritası", l.id);
    let mut v = json!({ "type": "heatmap", "radius": 30.0, "unit": "px" });
    match l.paint.get("heatmap-radius") {
        None => {}
        Some(x) => match prop_of(x, "heatmap-radius", lost, &what) {
            Some(Prop::Const(c)) if c.as_f64().is_some() => v["radius"] = c,
            Some(Prop::Metres(m)) => {
                v["radius"] = json!(round4(m));
                v["unit"] = json!("m");
            }
            _ => lost.push(format!("{what}: yarıçapı okunamadı; 30 piksel alındı")),
        },
    }
    if let Some(w) = l.paint.get("heatmap-weight") {
        match weight_of(w) {
            Some((f, max)) => {
                v["weight"] = json!(format!("[{f}]"));
                if let Some(m) = max {
                    v["max"] = json!(m);
                }
            }
            None if w.is_number() => {}
            None => lost.push(format!(
                "{what}: ağırlığı bir alanın sayısı değil; ağırlıksız alındı"
            )),
        }
    }
    let mut ramp: Vec<String> = Vec::new();
    if let Some(c) = l.paint.get("heatmap-color") {
        let a = c.as_array().cloned().unwrap_or_default();
        let ok = a.len() >= 7
            && a[0] == json!("interpolate")
            && a[2] == json!(["heatmap-density"])
            && a.len() % 2 == 1;
        if ok {
            let stops: Vec<(f64, Option<(String, f64)>)> = a[3..]
                .chunks(2)
                .map(|p| {
                    (
                        p[0].as_f64().unwrap_or(-1.0),
                        p[1].as_str().and_then(css_in),
                    )
                })
                .collect();
            let k = (stops.len() - 1) as f64;
            let uneven = stops
                .iter()
                .enumerate()
                .any(|(i, (o, _))| (o - i as f64 / k).abs() > 0.001);
            if uneven {
                lost.push(format!(
                    "{what}: renk durakları eşit aralıklı alındı (KentOS'un rampası eşit aralıklıdır)"
                ));
            }
            ramp = stops
                .into_iter()
                .filter_map(|(_, c)| c.map(|(rgb, a)| colour_with(&rgb, Some(a))))
                .collect();
        } else {
            lost.push(format!(
                "{what}: rengi yoğunluğa göre bir interpolate değil; KentOS'un rampası alındı"
            ));
        }
    }
    v["ramp"] = json!(kentos_formats::style_labels::heat_ramp(ramp, &what, lost));
    if let Some(o) = l.paint.get("heatmap-opacity").and_then(Value::as_f64) {
        v["opacity"] = json!(o.clamp(0.0, 1.0));
    }
    if let Some(q) = l.metadata.get("kentos:quality").and_then(Value::as_f64) {
        v["quality"] = json!(q);
    }
    let ignored: Vec<&str> = l
        .paint
        .keys()
        .map(String::as_str)
        .filter(|k| {
            !matches!(
                *k,
                "heatmap-radius" | "heatmap-weight" | "heatmap-color" | "heatmap-opacity"
            )
        })
        .collect();
    if !ignored.is_empty() {
        lost.push(format!(
            "{what}: {} özellikleri alınmadı",
            ignored.join(", ")
        ));
    }
    v
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use crate::{Kinds, LayerIn, StyleLayer, parse, renderer_of, write};

    fn none(_: &str) -> Option<Value> {
        None
    }

    #[test]
    fn a_heat_map_goes_out_as_a_heatmap_layer_and_comes_back() {
        let r = json!({ "type": "heatmap", "radius": 12.0, "unit": "px", "weight": "[w]", "max": 50.0,
            "ramp": ["#0000ff", "#ffff00", "#ff0000"], "opacity": 0.8 });
        let layer = LayerIn {
            id: "durak",
            name: "Durak",
            renderer: &r,
            kinds: Kinds {
                point: true,
                line: false,
                area: false,
            },
            opacity: 1.0,
            source: "durak",
            source_layer: None,
            label: None,
            labels: None,
        };
        let w = write("s", &[layer], &Map::new(), None, &none);
        assert!(w.json.contains("\"heatmap\""), "{}", w.json);
        let style = parse(&w.json).expect("parses");
        let all: Vec<&StyleLayer> = style.layers.iter().collect();
        let back = renderer_of(&all).expect("reads");
        assert_eq!(back.renderer["type"], "heatmap", "{:?}", back.lost);
        assert_eq!(back.renderer["weight"], "[w]");
        assert_eq!(back.renderer["max"], 50.0);
        assert_eq!(
            back.renderer["ramp"],
            json!(["#0000ff", "#ffff00", "#ff0000"])
        );
        assert_eq!(back.renderer["opacity"], 0.8);
    }
}
