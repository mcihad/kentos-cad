//! An object's transparency with a symbol of its own (docs/adr/9219): the program carries the library symbol as the
//! style core sees it through, under an id of its own: each layer's opacity times `1 − t`, a marker's on its own
//! layers. Without transparency, the library symbol as it is. An NCZ's protection band drawn with the zoning legend's
//! filled symbol hid the railway's line under it.

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::store::Store;
use kentos_native_style::program::{BuildOptions, ReadingUnit, layer_call};
use kentos_native_style::system;
use kentos_scene_query::geometry::shape;
use serde_json::{Value, json};

const BAND: &str = "mpyy.uip.yapi-sinirlamasi.demiryollari-koruma-kusagi";

fn band(transparency: Option<f64>) -> Entity {
    let mut e = json!({
        "kind": "polygon", "id": 1, "layerId": "a", "attrs": {}, "symbol": BAND,
        "pts": [{"x": 0.0, "y": 0.0}, {"x": 40.0, "y": 0.0}, {"x": 40.0, "y": 10.0}, {"x": 0.0, "y": 10.0}]
    });
    if let Some(t) = transparency {
        e["transparency"] = json!(t);
    }
    serde_json::from_value(e).expect("a polygon")
}

/// The program's symbols for one band.
fn symbols(e: &Entity) -> serde_json::Map<String, Value> {
    let style: LayerStyle = serde_json::from_value(
        json!({"color": "#336699", "lineType": "continuous", "lineWeight": 0.25}),
    )
    .expect("a style");
    let mut store = Store::new();
    store.put_many([(1.0, "a", false, shape(e))]);
    let library = system::library();
    let names = |_: &str| "A".to_owned();
    let opts = BuildOptions {
        line_types: None,
        layer_styles: None,
        origin: Vec2::new(0.0, 0.0),
        plot_scale: 1000.0,
        screen: false,
        hairlines: false,
        screen_widths: false,
        clip: None,
        library: &library,
        layer_name: &names,
        view: Default::default(),
        frame: None,
        reading_unit: ReadingUnit::METRE,
    };
    let call = layer_call(&style, &[e], &opts, &store);
    let program: Value = serde_json::from_str(&call.program).expect("json");
    program["symbols"].as_object().cloned().expect("symbols")
}

/// Every opacity the symbol draws at, a marker's on its own layers (absent: 1).
fn opacities(symbol: &Value) -> Vec<f64> {
    symbol["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .flat_map(|l| match l.get("marker") {
            Some(m @ Value::Object(_)) => opacities(m),
            _ => vec![l.get("opacity").and_then(Value::as_f64).unwrap_or(1.0)],
        })
        .collect()
}

#[test]
fn an_objects_transparency_sees_its_own_symbol_through() {
    let plain = symbols(&band(None));
    assert_eq!(plain.keys().collect::<Vec<_>>(), [BAND]);
    let seen = symbols(&band(Some(0.6)));
    let (id, symbol) = seen.iter().next().expect("one symbol");
    assert_ne!(
        id, BAND,
        "an id of its own: the library symbol stays as it is for the others"
    );
    let (was, now) = (opacities(&plain[BAND]), opacities(symbol));
    // The fill, the dashed edge, the edge's crosses and code, the centroid's squares, cross and code.
    assert_eq!(was.len(), 8);
    assert_eq!(now.len(), was.len());
    for (now, was) in now.iter().zip(&was) {
        assert!((now - was * 0.4).abs() < 1e-9, "{now} {was}");
    }
}
