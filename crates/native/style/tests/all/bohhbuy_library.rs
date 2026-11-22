//! The BÖHHBÜY library as the engine reads it (docs/adr/9120): the 277 signs of the large-scale map regulation's Ek-3,
//! each under its class with its number leading its name, every layer one its kind draws (the shared check of the
//! general library), every sign drawing a preview, and every pictogram an SVG drawing the desktop reads. The file is
//! written by `scripts/style/bohhbuy_library.py`.

use std::collections::{BTreeMap, BTreeSet};

use kentos_native_style::library::{ItemKind, Source};
use kentos_native_style::preview::preview;
use kentos_native_style::{StylePalette, system};
use serde_json::Value;

use super::general_library::check_layers;

/// Ek-3's classes and how many signs each holds: the 272 of its sign list and the five its drawings add (0645, 0646,
/// 0647, 0829, 1027).
const CLASSES: [(&str, usize); 13] = [
    ("0000 Eşyükseklik eğrileri ve kod numaraları", 6),
    ("0200 Karayolları ve tesisleri", 20),
    ("0400 Demiryolu, metro, tramvay ve tesisleri", 7),
    ("0600 Hidrografik bilgiler", 47),
    ("0800 Nakil hatları ve haberleşme tesisleri", 29),
    ("1000 Sınırlar", 27),
    ("1200 Yapılar", 36),
    ("1400 Bitki örtüsü ve tarım alanları", 26),
    ("1600 Taşlık ve kayalıklar", 10),
    ("1800 Şevler", 11),
    ("2000 Tamamlayıcı işaretler", 37),
    ("2400 Kontrol noktaları işaretleri", 5),
    ("2600 Yazı ve rakam işaretleri", 16),
];

#[test]
fn every_bohhbuy_sign_sits_in_its_class_with_its_number_and_draws() {
    let lib = system::library();
    let (items, categories) = lib.dump(Source::System);
    let paths: BTreeSet<Vec<String>> = categories
        .iter()
        .filter_map(|c| c["path"].as_array())
        .map(|p| p.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .collect();
    let signs: Vec<&Value> =
        items.iter().filter(|i| i["kind"] == "symbol" && i["id"].as_str().is_some_and(|id| id.starts_with("bohhbuy."))).collect();
    let palette = StylePalette::paper();
    let mut problems = Vec::new();
    let mut per_class: BTreeMap<String, usize> = BTreeMap::new();
    for item in &signs {
        let id = item["id"].as_str().unwrap_or("");
        let number = &id["bohhbuy.".len()..];
        if number.len() != 4 || !number.chars().all(|c| c.is_ascii_digit()) {
            problems.push(format!("{id}: not bohhbuy.<four digits>"));
            continue;
        }
        if !item["name"].as_str().is_some_and(|n| n.starts_with(&format!("{number} ")) && n.len() > 5) {
            problems.push(format!("{id}: the name does not lead with its number"));
        }
        let path: Vec<String> = item["path"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect();
        // The class is the number's hundreds rounded down to an even one: 0607 is under 0600, 1211 under 1200.
        let head: u32 = number[..2].parse().unwrap_or(0);
        let code = format!("{:02}00", head - head % 2);
        if path.len() != 2 || path[0] != "BÖHHBÜY" || !path[1].starts_with(&format!("{code} ")) {
            problems.push(format!("{id}: under {path:?}, not BÖHHBÜY › {code}"));
        }
        for n in 1..=path.len() {
            if !paths.contains(&path[..n]) {
                problems.push(format!("{id}: category {:?} is not listed", &path[..n]));
            }
        }
        if let Some(class) = path.get(1) {
            *per_class.entry(class.clone()).or_default() += 1;
        }
        if !item["reference"].as_str().is_some_and(|r| r.starts_with("BÖHHBÜY Ek-3")) {
            problems.push(format!("{id}: no reference to Ek-3"));
        }
        if item["description"].as_str().is_none_or(str::is_empty) {
            problems.push(format!("{id}: no description"));
        }
        let kind = item["symbol"]["type"].as_str().unwrap_or("");
        let layers = item["symbol"]["layers"].as_array().cloned().unwrap_or_default();
        if layers.is_empty() {
            problems.push(format!("{id}: no layers"));
        }
        check_layers(id, kind, &layers, &lib, &mut problems);
        match preview(&item["symbol"], None, (96.0, 64.0), None, &palette, &lib) {
            Err(why) => problems.push(format!("{id}: no preview: {why}")),
            Ok(p) if p.layer.batches.is_empty() => problems.push(format!("{id}: the preview draws nothing")),
            Ok(_) => {}
        }
    }
    for (class, count) in CLASSES {
        let found = per_class.get(class).copied().unwrap_or(0);
        if found != count {
            problems.push(format!("{class}: {found} signs, Ek-3 has {count}"));
        }
    }
    assert_eq!(signs.len(), 277, "{} signs", signs.len());
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_bohhbuy_pictogram_is_an_svg_drawing_the_desktop_reads() {
    let lib = system::library();
    let assets: Vec<_> = lib
        .items(Some(Source::System))
        .into_iter()
        .filter(|(i, _)| i.kind() == ItemKind::Asset && i.id().starts_with("bohhbuy.svg."))
        .collect();
    assert!(assets.len() >= 30, "{} pictograms", assets.len());
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
