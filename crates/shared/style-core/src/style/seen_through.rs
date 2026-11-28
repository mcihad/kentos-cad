//! An object's own transparency on its own symbol (docs/adr/9219): the symbol as the object draws it, seen through.
//! The desktop's layer program calls it (crates/native/style/src/program.rs); it is the core's, not the program's,
//! for the clients to come (docs/adr/0228 §3, §4).
//!
//! An object's transparency was kept for its simple look and lost with a symbol of its own: an NCZ's railway
//! protection band drawn with the zoning legend's filled symbol hid the railway's line under it (24 November 2026).

use kentos_geometry_core::api::json::Json;

/// The most an object is seen through: an object's transparency is 0 to 0.9 (docs/adr/0260 §3).
pub const MOST: f64 = 0.9;

/// `symbol` as an object `transparency` transparent draws it: every layer's opacity (absent: 1) times `1 − t`, `t`
/// the transparency within 0 and 0.9 (a number out of it, or none, the nearest end; not a number, 0).
///
/// A layer that draws a marker (along a line, at an area's centroid, in a pattern) passes it on to the marker's own
/// layers: the compiler draws a marker at its own layers' opacities, not its carrier's.
pub fn seen_through(symbol: &Json, transparency: f64) -> Json {
    let t = if transparency.is_nan() {
        0.0
    } else {
        transparency.clamp(0.0, MOST)
    };
    let mut out = symbol.clone();
    fade(&mut out, 1.0 - t);
    out
}

/// Each layer of `symbol` (a symbol, or a layer's marker) drawn `by` as opaque as it was.
fn fade(symbol: &mut Json, by: f64) {
    let Json::Obj(fields) = symbol else { return };
    let Some(Json::Arr(layers)) = fields
        .iter_mut()
        .find(|(k, _)| k == "layers")
        .map(|(_, v)| v)
    else {
        return;
    };
    for layer in layers {
        let Json::Obj(layer) = layer else { continue };
        if let Some((_, marker)) = layer
            .iter_mut()
            .find(|(k, v)| k == "marker" && matches!(v, Json::Obj(_)))
        {
            fade(marker, by);
            continue;
        }
        let was = match layer.iter().find(|(k, _)| k == "opacity") {
            Some((_, Json::Num(o))) => *o,
            _ => 1.0,
        };
        let opacity = Json::Num(was * by);
        match layer.iter_mut().find(|(k, _)| k == "opacity") {
            Some((_, v)) => *v = opacity,
            None => layer.push(("opacity".to_owned(), opacity)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opacities(symbol: &Json) -> Vec<f64> {
        let Json::Arr(layers) = symbol.get("layers") else {
            return Vec::new();
        };
        layers
            .iter()
            .flat_map(|l| match l.get("marker") {
                Json::Obj(_) => opacities(l.get("marker")),
                _ => vec![match l.get("opacity") {
                    Json::Num(o) => *o,
                    _ => f64::NAN,
                }],
            })
            .collect()
    }

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12)
    }

    const BAND: &str = r##"{"type":"fill","layers":[
        {"type":"simpleFill","color":"#F57A7A"},
        {"type":"simpleLine","color":"#FF0000","width":0.3,"opacity":0.5},
        {"type":"markerLine","marker":{"type":"marker","layers":[{"type":"shape","shape":"x","size":3}]}},
        {"type":"centroidMarker","marker":{"type":"marker","layers":[
            {"type":"shape","shape":"square","size":10},
            {"type":"text","text":"DKK","size":3.8,"opacity":0.8}]}}]}"##;

    #[test]
    fn every_layer_and_every_marks_layer_is_seen_through() {
        let band = Json::parse(BAND).unwrap();
        assert!(close(
            &opacities(&seen_through(&band, 0.6)),
            &[0.4, 0.2, 0.4, 0.4, 0.32]
        ));
        // The carrier of a marker keeps its own: the compiler does not read it.
        let seen = seen_through(&band, 0.6);
        let Json::Arr(layers) = seen.get("layers") else {
            panic!()
        };
        assert_eq!(layers[2].get("opacity"), &Json::Null);
        assert_eq!(seen.get("type"), &Json::Str("fill".into()));
    }

    #[test]
    fn a_transparency_out_of_its_range_is_its_nearest_end() {
        let band = Json::parse(BAND).unwrap();
        let at = |t: f64| opacities(&seen_through(&band, t));
        assert!(close(&at(0.95), &at(MOST)));
        assert!(close(&at(-1.0), &[1.0, 0.5, 1.0, 1.0, 0.8]));
        assert!(close(&at(f64::NAN), &at(0.0)));
        assert!(close(&at(f64::INFINITY), &at(MOST)));
    }

    #[test]
    fn what_is_not_a_symbol_comes_back_as_it_was() {
        for text in [
            "null",
            "\"mpyy.uip.x\"",
            "{\"type\":\"fill\"}",
            "{\"layers\":[1,\"a\"]}",
        ] {
            let v = Json::parse(text).unwrap();
            assert_eq!(seen_through(&v, 0.6), v);
        }
    }
}
