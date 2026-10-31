//! docs/adr/0313's timings: 100 000 lines of a layer drawn with a LIN file's pattern (the project's line types at
//! LTSCALE 0.5, a tenth of the lines a pattern of their own) against the same layer with a built-in dashed type,
//! built as the desktop builds a layer (the page's call, the core's batches, decoded for the GPU). Prints the
//! fastest of ten after one to warm up; release, by hand:
//!
//! ```text
//! cargo test --release -p kentos-native-style --test all line_type_timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_contracts::{Entity, LayerStyle, ProjectSettings};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::batches::{DecodeOptions, StyledLayer, decode};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::line_types::LineTypes;
use kentos_native_style::program::{BuildOptions, ReadingUnit, build_layer};
use kentos_native_style::{StylePalette, View};
use serde_json::json;

fn lines(own: bool) -> Vec<Entity> {
    (0..100_000u32)
        .map(|i| {
            let (x, y) = (f64::from(i % 300) * 30.0, f64::from(i / 300) * 3.0);
            let mut e = json!({"kind":"line","id": i + 1,"layerId":"k","attrs":{},
                "a":{"x":x,"y":y},"b":{"x":x + 25.0,"y":y + 1.0}});
            if own && i % 10 == 0 {
                e["lineType"] = json!("dashed");
                e["linePattern"] = json!("EKSEN");
            }
            serde_json::from_value(e).expect("a line")
        })
        .collect()
}

fn build(
    entities: &[Entity],
    store: &Store,
    style: &LayerStyle,
    types: Option<&LineTypes>,
) -> StyledLayer {
    let library = StyleLibrary::default();
    let names = |_: &str| "Katman".to_owned();
    let opts = BuildOptions {
        line_types: types,
        layer_styles: None,
        origin: Vec2::new(0.0, 0.0),
        plot_scale: 500.0,
        screen: false,
        hairlines: false,
        clip: None,
        library: &library,
        layer_name: &names,
        view: View::default(),
        frame: None,
        reading_unit: ReadingUnit::METRE,
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
            plot_scale: 500.0,
            library: &library,
            view: Default::default(),
            hold_weights: false,
        },
    )
    .expect("decodes")
}

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
fn line_type_timing() {
    // A LIN file's two types, in metres: SINIR `A,6,-2,6,-2,0,-2`, EKSEN `A,12,-3,2,-3`; LTSCALE 0.5.
    let settings: ProjectSettings = serde_json::from_value(json!({
        "srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 500,
        "lineTypeScale": 0.5,
        "lineTypes": [
            { "name": "SINIR", "elements": [{"length": 6}, {"length": -2}, {"length": 6}, {"length": -2},
              {"length": 0}, {"length": -2}], "unit": "drawing" },
            { "name": "EKSEN", "elements": [{"length": 12}, {"length": -3}, {"length": 2}, {"length": -3}],
              "unit": "drawing" }
        ]
    }))
    .expect("the settings");
    let t = Instant::now();
    let types = LineTypes::of(&settings);
    let of_ms = t.elapsed().as_secs_f64() * 1000.0;
    let plain = lines(false);
    let mixed = lines(true);
    let store = |e: &[Entity]| {
        let mut s = Store::new();
        s.put_many(e.iter().map(|e| {
            let b = e.base();
            (f64::from(b.id), b.layer_id.as_str(), false, shape(e))
        }));
        s
    };
    let (plain_store, mixed_store) = (store(&plain), store(&mixed));
    let dashed: LayerStyle = serde_json::from_value(
        json!({ "color": "#3E63DD", "lineType": "dashed", "lineWeight": 0.35 }),
    )
    .expect("a style");
    let mut patterned = dashed.clone();
    patterned.line_pattern = Some("SINIR".into());
    println!("ADR 0313: release, 100 000 çizgi, 1:500");
    println!("Projenin çizgi tipleri okunur: {of_ms:.3} ms");
    for (what, entities, store, style, types) in [
        ("Yerleşik kesikli", &plain, &plain_store, &dashed, None),
        (
            "LIN deseni (SINIR)",
            &plain,
            &plain_store,
            &patterned,
            Some(&types),
        ),
        (
            "LIN deseni, onda biri kendi deseniyle",
            &mixed,
            &mixed_store,
            &patterned,
            Some(&types),
        ),
    ] {
        let (ms, made) = time(|| build(entities, store, style, types));
        println!(
            "{what:<42} {ms:>8.2} ms   ({} topluluk)",
            made.batches.len()
        );
    }
}
