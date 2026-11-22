//! The general symbol library (docs/adr/9118) as the engine reads it: every symbol's id says its kind and sits in a
//! category that exists, every layer is one its kind draws, every shape, font and pictogram is known (an unknown
//! shape would draw as a circle, an unknown font as the default), every pictogram is an SVG drawing the desktop reads,
//! every symbol draws a preview, and every category holds symbols. The file is written by
//! `scripts/style/general_library.py`.

use std::collections::BTreeSet;

use kentos_native_style::batches::{SHAPE_IDS, font_family};
use kentos_native_style::library::{ItemKind, Source};
use kentos_native_style::preview::preview;
use kentos_native_style::{StylePalette, system};
use serde_json::Value;

/// The layer types a symbol of each kind draws.
fn allowed(kind: &str) -> &'static [&'static str] {
    match kind {
        "marker" => &["shape", "svg", "text"],
        "line" => &["simpleLine", "markerLine"],
        "fill" => &["simpleFill", "hatchFill", "patternFill", "gradientFill", "centroidMarker", "simpleLine", "markerLine"],
        _ => &[],
    }
}

/// A marker's layers (inside a marker line, a pattern or a centroid marker) are a marker symbol's.
fn check_layers(id: &str, kind: &str, layers: &[Value], lib: &kentos_native_style::StyleLibrary, problems: &mut Vec<String>) {
    for layer in layers {
        let t = layer["type"].as_str().unwrap_or("");
        if !allowed(kind).contains(&t) {
            problems.push(format!("{id}: a {kind} symbol does not draw a “{t}” layer"));
        }
        match t {
            "shape" => {
                let shape = layer["shape"].as_str().unwrap_or("");
                if !SHAPE_IDS.contains(&shape) {
                    problems.push(format!("{id}: shape “{shape}” is not one the shader draws"));
                }
            }
            "svg" => {
                let asset = layer["asset"].as_str().unwrap_or("");
                if lib.asset(asset).is_none() {
                    problems.push(format!("{id}: pictogram “{asset}” is not in the library"));
                }
            }
            "text" => {
                let font = layer["font"].as_str().unwrap_or("");
                if font_family(font).is_none() {
                    problems.push(format!("{id}: font “{font}” is not a text marker's face"));
                }
            }
            _ => {}
        }
        if let Some(inner) = layer["marker"]["layers"].as_array() {
            check_layers(id, "marker", inner, lib, problems);
        }
    }
}

#[test]
fn every_general_symbol_is_one_the_engine_draws_in_a_category_that_exists() {
    let lib = system::library();
    let (items, categories) = lib.dump(Source::System);
    let paths: BTreeSet<Vec<String>> = categories
        .iter()
        .filter_map(|c| c["path"].as_array())
        .map(|p| p.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .collect();
    let general: Vec<&Value> = items.iter().filter(|i| i["id"].as_str().is_some_and(|id| id.starts_with("genel."))).collect();
    let symbols: Vec<&Value> = general.iter().copied().filter(|i| i["kind"] == "symbol").collect();
    assert!(symbols.len() >= 190, "{} general symbols", symbols.len());
    let palette = StylePalette::paper();
    let mut problems = Vec::new();
    let mut used: BTreeSet<Vec<String>> = BTreeSet::new();
    for item in &symbols {
        let id = item["id"].as_str().unwrap_or("");
        let kind = item["symbol"]["type"].as_str().unwrap_or("");
        let short = match kind {
            "marker" => "genel.nokta.",
            "line" => "genel.cizgi.",
            "fill" => "genel.alan.",
            other => {
                problems.push(format!("{id}: kind “{other}”"));
                continue;
            }
        };
        if !id.starts_with(short) {
            problems.push(format!("{id}: a {kind} symbol's id starts with {short}"));
        }
        let path: Vec<String> = item["path"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect();
        // The category and each one above it are listed.
        for n in 1..=path.len() {
            if !paths.contains(&path[..n]) {
                problems.push(format!("{id}: category {:?} is not listed", &path[..n]));
            }
        }
        used.insert(path.clone());
        if item["name"].as_str().is_none_or(str::is_empty) {
            problems.push(format!("{id}: no name"));
        }
        let layers = item["symbol"]["layers"].as_array().cloned().unwrap_or_default();
        if layers.is_empty() {
            problems.push(format!("{id}: no layers"));
        }
        check_layers(id, kind, &layers, &lib, &mut problems);
        if let Err(why) = preview(&item["symbol"], None, (96.0, 64.0), None, &palette, &lib) {
            problems.push(format!("{id}: no preview: {why}"));
        } else if preview(&item["symbol"], None, (96.0, 64.0), None, &palette, &lib).is_ok_and(|p| p.layer.batches.is_empty()) {
            problems.push(format!("{id}: the preview draws nothing"));
        }
    }
    // Every category under Genel but the pictograms' holds symbols, directly or below.
    for path in paths.iter().filter(|p| p.first().is_some_and(|f| f == "Genel")) {
        let pictograms = path.get(1).is_some_and(|s| s == "Piktogramlar");
        if !pictograms && !used.iter().any(|u| u.starts_with(path)) {
            problems.push(format!("category {path:?} holds no symbol"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_general_pictogram_is_an_svg_drawing_the_desktop_reads() {
    let lib = system::library();
    let assets: Vec<_> = lib
        .items(Some(Source::System))
        .into_iter()
        .filter(|(i, _)| i.kind() == ItemKind::Asset && i.id().starts_with("genel.svg."))
        .collect();
    assert!(assets.len() >= 50, "{} pictograms", assets.len());
    let no_glyphs = |_: &str, _: Option<&str>, _: f64, _: bool| None;
    for (item, _) in assets {
        let data = item.value()["data"].as_str().unwrap_or("");
        match kentos_native_style::svg::picture(data, &no_glyphs) {
            Some(kentos_native_style::picture::Picture::Vector { view, nodes }) => {
                assert_eq!(view, [0.0, 0.0, 100.0, 100.0], "{}", item.id());
                assert!(!nodes.is_empty(), "{} draws nothing", item.id());
            }
            other => panic!("{} is not a vector drawing: {other:?}", item.id()),
        }
    }
}
