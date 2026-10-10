//! ADR 0213 §6's budgets on synthetic layers, built as the desktop builds a
//! layer (the page's call and the style core's batches, decoded for the
//! GPU): 10 000 districts in Sürekli renk and in pies, 100 000 dots in 1 000
//! districts, a heat map and clusters of 100 000 points, 10 000 parcels'
//! outside. The view is 1440 × 900 px over 2 km. Prints each time (the
//! fastest of ten, after one to warm up) against its budget; release, by hand:
//!
//! ```text
//! cargo test --release -p kentos-native-style --test all renderer_timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::batches::{DecodeOptions, StyledLayer, decode};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, build_layer};
use kentos_native_style::{StylePalette, View};
use serde_json::{Value, json};

const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;
/// The view: 1440 × 900 px over 2 000 m.
const PX_PER_M: f64 = 1440.0 / 2000.0;

fn square(id: u32, x: f64, y: f64, w: f64, attrs: Value) -> Value {
    json!({ "kind": "polygon", "id": id, "layerId": "k", "attrs": attrs,
        "pts": [{ "x": x, "y": y }, { "x": x + w, "y": y }, { "x": x + w, "y": y + w }, { "x": x, "y": y + w }] })
}

/// `n` districts on a grid, `w` metres each, numbers that vary.
fn districts(n: u32, w: f64) -> Vec<Entity> {
    let side = f64::from(n).sqrt().ceil() as u32;
    (0..n)
        .map(|i| {
            let (c, r) = (i % side, i / side);
            let attrs = json!({
                "Nüfus": ((i * 7919) % 40_000).to_string(),
                "Konut": ((i * 31) % 100 + 1).to_string(),
                "Ticaret": ((i * 17) % 60 + 1).to_string(),
                "Yeşil": ((i * 13) % 30 + 1).to_string(),
                "Erkek": "500",
                "Kadın": "500",
            });
            let e = square(
                i + 1,
                X0 + f64::from(c) * w,
                Y0 + f64::from(r) * w,
                w * 0.96,
                attrs,
            );
            serde_json::from_value(e).expect("an area")
        })
        .collect()
}

/// `n` points spread over 2 km × 1.25 km in clumps (a fixed walk, no randomness).
fn points(n: u32) -> Vec<Entity> {
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let centres: Vec<(f64, f64)> = (0..40)
        .map(|_| (next() * 2000.0, next() * 1250.0))
        .collect();
    (0..n)
        .map(|i| {
            let (cx, cy) = centres[(i as usize) % centres.len()];
            let r = next() * 120.0;
            let a = next() * std::f64::consts::TAU;
            let e = json!({ "kind": "point", "id": i + 1, "layerId": "k", "attrs": { "Önem": ((i % 5) + 1).to_string() },
                "p": { "x": X0 + cx + r * a.cos(), "y": Y0 + cy + r * a.sin() } });
            serde_json::from_value(e).expect("a point")
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

fn style_of(renderer: Value) -> LayerStyle {
    serde_json::from_value(json!({
        "color": "#4E79A7", "lineType": "continuous", "lineWeight": 0.25, "renderer": renderer,
    }))
    .expect("a style")
}

/// The view's box, and a heat map's (half of it more on every side).
fn view_box() -> Bounds {
    Bounds {
        min_x: X0,
        min_y: Y0,
        max_x: X0 + 2000.0,
        max_y: Y0 + 1250.0,
    }
}

fn grown(b: Bounds, k: f64) -> Bounds {
    let (w, h) = ((b.max_x - b.min_x) * k, (b.max_y - b.min_y) * k);
    Bounds {
        min_x: b.min_x - w,
        min_y: b.min_y - h,
        max_x: b.max_x + w,
        max_y: b.max_y + h,
    }
}

/// One build as the desktop's scene makes it: the page, the core, the GPU's batches.
fn build(
    entities: &[Entity],
    store: &Store,
    style: &LayerStyle,
    clip: Option<Bounds>,
    frame: bool,
) -> StyledLayer {
    let library = StyleLibrary::default();
    let names = |_: &str| "Katman".to_owned();
    let opts = BuildOptions {
        origin: Vec2::new(X0, Y0),
        plot_scale: 5000.0,
        screen: false,
        hairlines: false,
        clip,
        library: &library,
        layer_name: &names,
        view: View::default(),
        frame: frame.then(|| (PX_PER_M, "heat:k:1".to_owned())),
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
            plot_scale: 5000.0,
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

fn row(what: &str, ms: f64, budget: f64, made: &StyledLayer) {
    let numbers: usize = made.data.len();
    println!(
        "{what:<52} {ms:>9.2} ms   bütçe {budget:>5} ms  {}   ({} topluluk, {numbers} sayı)",
        if ms <= budget { "✓" } else { "✗" },
        made.batches.len()
    );
}

#[test]
#[ignore = "a measurement, run by hand in release"]
fn renderer_timing() {
    println!("ADR 0213 §6: release, 1440 × 900 px over 2 km");
    let areas = districts(10_000, 20.0);
    let areas_store = store_of(&areas);
    let unclassed = style_of(
        json!({ "type": "unclassed", "expr": "Nüfus", "min": 0, "max": 40000,
        "ramp": ["#FFF5B8", "#FDB863", "#E66101", "#A50F15"],
        "symbols": { "fill": { "type": "fill", "layers": [
            { "id": "f", "type": "simpleFill", "color": "#000000" },
            { "id": "l", "type": "simpleLine", "color": "#FFFFFF", "width": 0.3 }] } } }),
    );
    let (ms, made) = time(|| build(&areas, &areas_store, &unclassed, None, false));
    row("Sürekli renk, 10 000 alan", ms, 30.0, &made);
    let pies = style_of(
        json!({ "type": "chart", "kind": "pie", "size": 6, "unit": "mm",
        "fields": [{ "expr": "Konut", "color": "#E15759" }, { "expr": "Ticaret", "color": "#4E79A7" },
            { "expr": "Yeşil", "color": "#59A14F" }],
        "outline": { "color": "#FFFFFF", "width": 0.2 } }),
    );
    let (ms, made) = time(|| build(&areas, &areas_store, &pies, None, false));
    row("Pasta grafik, 10 000 alan (çerçeveli)", ms, 70.0, &made);
    let few = districts(1_000, 60.0);
    let few_store = store_of(&few);
    let dots = style_of(
        json!({ "type": "dotDensity", "dotValue": 10, "dotSize": 0.5, "unit": "mm", "seed": 1,
        "fields": [{ "expr": "Erkek", "color": "#4E79A7" }, { "expr": "Kadın", "color": "#E15759" }] }),
    );
    let (ms, made) = time(|| build(&few, &few_store, &dots, None, false));
    row(
        "Nokta yoğunluğu, 1 000 alanda 100 000 nokta",
        ms,
        60.0,
        &made,
    );
    let pts = points(100_000);
    let pts_store = store_of(&pts);
    let heat = style_of(
        json!({ "type": "heatmap", "radius": 20, "unit": "px", "quality": 2,
        "ramp": ["#2B83BA00", "#2B83BA", "#ABDDA4", "#FFFFBF", "#FDAE61", "#D7191C"] }),
    );
    let heat_box = grown(view_box(), 0.5);
    let (ms, made) = time(|| build(&pts, &pts_store, &heat, Some(heat_box), true));
    row(
        "Isı haritası, 100 000 nokta (20 px, kalite 2)",
        ms,
        40.0,
        &made,
    );
    assert_eq!(made.pictures.len(), 1, "the heat map's picture");
    let cluster = style_of(json!({ "type": "cluster", "distance": 40, "unit": "px" }));
    let (ms, made) = time(|| {
        build(
            &pts,
            &pts_store,
            &cluster,
            Some(grown(view_box(), 3.0)),
            true,
        )
    });
    row("Kümeleme, 100 000 nokta (40 px)", ms, 30.0, &made);
    let inverted = style_of(
        json!({ "type": "inverted", "symbols": { "fill": { "type": "fill", "layers": [
        { "id": "f", "type": "simpleFill", "color": "#FFFFFFB3" },
        { "id": "l", "type": "simpleLine", "color": "#8E4EC6", "width": 0.6 }] } } }),
    );
    let (ms, made) = time(|| {
        build(
            &areas,
            &areas_store,
            &inverted,
            Some(grown(view_box(), 3.0)),
            false,
        )
    });
    row("Ters alan, 10 000 parsel", ms, 40.0, &made);
}
