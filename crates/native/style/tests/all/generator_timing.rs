//! ADR 0226 §7's budget for Geometri üretici on a synthetic layer, built as
//! the desktop builds one (the page's call and the style core's batches,
//! decoded for the GPU): 10 000 photo points whose symbol has a two-step
//! generator (a 2 mm buffer, then a slice of their view from Yön) besides its
//! dot, against the same points with the dot alone; and 10 000 parcels with
//! a generator of their centroid's regular hexagon. The cost is the
//! difference. Prints each time (the fastest of ten, after one to warm up)
//! against its budget; release, by hand:
//!
//! ```text
//! cargo test --release -p kentos-native-style --test all generator_timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::batches::{DecodeOptions, StyledLayer, decode};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, build_layer};
use kentos_native_style::{StylePalette, View};
use serde_json::{Value, json};

const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;

/// `n` points on a 2 km square, each with its Yön (a fixed walk, no randomness).
fn points(n: u32) -> Vec<Entity> {
    let side = f64::from(n).sqrt().ceil() as u32;
    (0..n)
        .map(|i| {
            let (c, r) = (i % side, i / side);
            let e = json!({ "kind": "point", "id": i + 1, "layerId": "k",
                "attrs": { "Yön": ((i * 37) % 360).to_string() },
                "p": { "x": X0 + f64::from(c) * 20.0, "y": Y0 + f64::from(r) * 20.0 } });
            serde_json::from_value(e).expect("a point")
        })
        .collect()
}

/// `n` parcels on a grid, 18 m each.
fn parcels(n: u32) -> Vec<Entity> {
    let side = f64::from(n).sqrt().ceil() as u32;
    (0..n)
        .map(|i| {
            let (c, r) = (i % side, i / side);
            let (x, y, w) = (X0 + f64::from(c) * 20.0, Y0 + f64::from(r) * 20.0, 18.0);
            let e = json!({ "kind": "polygon", "id": i + 1, "layerId": "k", "attrs": {},
                "pts": [{ "x": x, "y": y }, { "x": x + w, "y": y }, { "x": x + w, "y": y + w }, { "x": x, "y": y + w }] });
            serde_json::from_value(e).expect("an area")
        })
        .collect()
}

fn store_of(entities: &[Entity]) -> Store {
    let mut store = Store::new();
    store.put_many(entities.iter().map(|e| {
        let b = e.base();
        (f64::from(b.id), b.layer_id.as_str(), false, shape(e))
    }));
    store
}

fn single(symbols: Value) -> LayerStyle {
    serde_json::from_value(json!({
        "color": "#4E79A7", "lineType": "continuous", "lineWeight": 0.25,
        "renderer": { "type": "single", "symbols": symbols },
    }))
    .expect("a style")
}

fn build(entities: &[Entity], store: &Store, style: &LayerStyle) -> StyledLayer {
    let library = StyleLibrary::default();
    let names = |_: &str| "Katman".to_owned();
    let opts = BuildOptions {
        origin: Vec2::new(X0, Y0),
        plot_scale: 1000.0,
        screen: false,
        hairlines: false,
        clip: None,
        library: &library,
        layer_name: &names,
        view: View::default(),
        frame: None,
    };
    let list: Vec<&Entity> = entities.iter().collect();
    let (_, batches) = build_layer(store, style, &list, &opts).expect("the core builds");
    let palette = StylePalette {
        fg: "#E6E6E6".into(),
        fg_dim: "#8A8A8A".into(),
        ink: "#000000".into(),
        paper: "#FFFFFF".into(),
    };
    decode(
        batches,
        &DecodeOptions {
            palette: &palette,
            plot_scale: 1000.0,
            library: &library,
            view: Default::default(),
        },
    )
    .expect("decodes")
}

/// The fastest of ten builds after one to warm up, ms.
fn time(mut f: impl FnMut() -> StyledLayer) -> (f64, StyledLayer) {
    let mut out = f();
    let mut best = f64::INFINITY;
    for _ in 0..10 {
        let t = Instant::now();
        out = f();
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
    }
    (best, out)
}

#[test]
#[ignore = "a measurement, run by hand in release"]
fn generator_timing() {
    println!("ADR 0226 §7: release, 1:1000");
    let dot = json!({ "id": "c", "type": "shape", "shape": "circle", "size": 2, "fill": "#E5484D" });
    let pts = points(10_000);
    let store = store_of(&pts);
    let plain = single(json!({ "marker": { "type": "marker", "layers": [dot.clone()] } }));
    let generated = single(json!({ "marker": { "type": "marker", "layers": [
        { "id": "g", "type": "geometryGenerator",
          "steps": [
              { "kind": "buffer", "distance": 2 },
              { "kind": "wedge", "direction": { "expr": "[Yön]", "fallback": 0 }, "width": 60, "radius": 8 }
          ],
          "symbol": { "type": "fill", "layers": [
              { "id": "f", "type": "simpleFill", "color": "#E5484D40" },
              { "id": "l", "type": "simpleLine", "color": "#E5484D", "width": 0.25 }] } },
        dot ] } }));
    let (base, _) = time(|| build(&pts, &store, &plain));
    let (with, made) = time(|| build(&pts, &store, &generated));
    let added = with - base;
    println!(
        "{:<60} {added:>9.2} ms   bütçe   150 ms  {}   (yalnız nokta {base:.2} ms, üreticiyle {with:.2} ms; {} topluluk, {} sayı)",
        "İki adımlı üretici (tampon, daire dilimi), 10 000 nokta",
        if added <= 150.0 { "✓" } else { "✗" },
        made.batches.len(),
        made.data.len()
    );
    let areas = parcels(10_000);
    let store = store_of(&areas);
    let fill = json!({ "id": "f", "type": "simpleFill", "color": "#C9D6E3" });
    let plain = single(json!({ "fill": { "type": "fill", "layers": [fill.clone()] } }));
    let generated = single(json!({ "fill": { "type": "fill", "layers": [
        fill,
        { "id": "g", "type": "geometryGenerator",
          "steps": [{ "kind": "centroid" }, { "kind": "polygon", "sides": 6, "radius": 3 }],
          "symbol": { "type": "line", "layers": [{ "id": "l", "type": "simpleLine", "color": "#4E79A7", "width": 0.3 }] } } ] } }));
    let (base, _) = time(|| build(&areas, &store, &plain));
    let (with, made) = time(|| build(&areas, &store, &generated));
    let added = with - base;
    println!(
        "{:<60} {added:>9.2} ms   bütçe   150 ms  {}   (yalnız dolgu {base:.2} ms, üreticiyle {with:.2} ms; {} topluluk, {} sayı)",
        "Ağırlık merkezinde düzgün altıgen, 10 000 parsel",
        if added <= 150.0 { "✓" } else { "✗" },
        made.batches.len(),
        made.data.len()
    );
}
