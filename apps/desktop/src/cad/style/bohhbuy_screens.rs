//! Pictures of the BÖHHBÜY library (docs/adr/9120) for the owner, light and dark, in `.run/gui-evidence/bohhbuy-kitaplik/`:
//! each class as sheets at 1:1000, every sign on a sample object with its number and name and the sample values its
//! texts read (`katalog-*.png`), and Stil yöneticisi open at each class (`yonetici-*.png`). Not a test of correctness
//! and not run by default:
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib bohhbuy_screens -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use iced::Size;
use kentos_commands::DocumentSnapshotV1;
use kentos_ui::snapshot::Snapshot;
use serde_json::{Value, json};

use super::general_screens::{at, bounds, picture, slug, themed, town};
use crate::app::{App, Message};
use crate::document::Document;

fn out() -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bohhbuy-kitaplik");
    std::fs::create_dir_all(&dir).expect("a folder for the pictures");
    dir
}

/// The values a sign's texts read on the sample (the regulation's own examples).
fn sample(id: &str) -> Value {
    match id.trim_start_matches("bohhbuy.") {
        "0004" => json!({ "Kot": "125.30" }),
        "0005" => json!({ "Kot": "1250" }),
        "0214" => json!({ "Numara": "100" }),
        "0821" => json!({ "Boy": "23" }),
        "0827" => json!({ "Cins": "Petrol vanası" }),
        "2601" => json!({ "Numara": "11200", "Kot": "332.716" }),
        "2602" => json!({ "Numara": "2171", "Kot": "125.30" }),
        "2603" => json!({ "Numara": "5233", "Kot": "56.3" }),
        "2604" => json!({ "Numara": "Rs. 73", "Kot": "523.113" }),
        "2605" => json!({ "Numara": "Rs. 23", "Kot": "158.813" }),
        "2606" => json!({ "Ad": "Yenimahalle" }),
        "2607" => json!({ "Ad": "Vatan Caddesi" }),
        "2608" => json!({ "Ad": "Belediye" }),
        "2609" => json!({ "Ad": "Esenboğa" }),
        "2610" => json!({ "Ad": "Çeşme" }),
        "2611" => json!({ "Kat": "5" }),
        "2612" => json!({ "Ada": "55" }),
        "2613" => json!({ "Parsel": "7" }),
        "2614" => json!({ "Ad": "Karadeniz" }),
        "2615" => json!({ "Ad": "Van Gölü" }),
        "2616" => json!({ "Ad": "İncesu Deresi" }),
        _ => json!({}),
    }
}

/// Twelve signs to a sheet, six rows in two columns, in their numbers' order: a point at the row's start, a line with
/// a bend, an area as a 26 × 14 m box; the number and name to its right.
fn sheet(members: &[&Value]) -> (Document, kentos_render_wgpu::Bounds) {
    let mut entities: Vec<Value> = Vec::new();
    let mut add = |mut e: Value, attrs: Value| {
        e["id"] = json!(entities.len() + 1);
        e["attrs"] = attrs;
        entities.push(e);
    };
    let pts = |list: &[(f64, f64)]| list.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>();
    for (k, item) in members.iter().enumerate() {
        let (x, y) = ((k / 6) as f64 * 135.0, -((k % 6) as f64) * 20.0);
        let id = item["id"].as_str().unwrap_or("");
        let attrs = sample(id);
        match item["symbol"]["type"].as_str() {
            Some("marker") => add(
                json!({ "kind": "point", "layerId": "ornek", "p": at(x + 13.0, y), "symbol": id }),
                attrs,
            ),
            Some("line") => add(
                json!({ "kind": "polyline", "layerId": "ornek",
                                        "pts": pts(&[(x, y - 2.0), (x + 20.0, y - 2.0), (x + 28.0, y + 4.0)]),
                                        "bulges": [0, 0, 0], "symbol": id }),
                attrs,
            ),
            _ => add(
                json!({ "kind": "polygon", "layerId": "ornek",
                             "pts": pts(&[(x, y - 7.0), (x + 26.0, y - 7.0), (x + 26.0, y + 7.0), (x, y + 7.0)]),
                             "bulges": [0, 0, 0, 0], "symbol": id }),
                attrs,
            ),
        }
        add(
            json!({ "kind": "text", "layerId": "yazilar", "p": at(x + 33.0, y), "text": item["name"], "height": 2.6,
                    "rotation": 0, "align": "middleLeft" }),
            json!({}),
        );
    }
    let layer = |id: &str, name: &str| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "BÖHHBÜY işaret kataloğu",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow" },
        "origin": at(0.0, 0.0),
        "layers": [layer("yazilar", "Adlar"), layer("ornek", "Örnekler")],
        "activeLayer": "ornek",
        "entities": entities,
        "styles": { "items": [], "categories": [] },
    });
    let snapshot = DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("the sheet reads");
    let doc = Document::new(snapshot, None).expect("the sheet opens");
    let right = if members.len() > 6 { 264.0 } else { 129.0 };
    (doc, bounds(-6.0, -112.0, right, 12.0))
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn bohhbuy_catalog_pictures() {
    use kentos_native_style::library::Source;
    let library = kentos_native_style::system::library();
    let (items, categories) = library.dump(Source::System);
    let dir = out();
    let only = std::env::var("BOHHBUY_CLASS").ok();
    let themes: &[(&str, &str)] = if std::env::var("BOHHBUY_LIGHT").is_ok() {
        &[("light", "")]
    } else {
        &[("light", ""), ("dark", "-koyu")]
    };
    for c in &categories {
        let path: Vec<&str> = c["path"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if path.len() != 2 || path[0] != "BÖHHBÜY" || path[1] == "Piktogramlar" {
            continue;
        }
        if only.as_deref().is_some_and(|o| !path[1].starts_with(o)) {
            continue;
        }
        let mut members: Vec<&Value> = items
            .iter()
            .filter(|i| {
                i["kind"] == "symbol"
                    && i["path"].as_array().is_some_and(|p| {
                        p.iter().filter_map(Value::as_str).eq(path.iter().copied())
                    })
            })
            .collect();
        members.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
        let class = slug(path[1].split(' ').next().unwrap_or(""));
        for (page, chunk) in members.chunks(12).enumerate() {
            let (doc, b) = sheet(chunk);
            for (mode, suffix) in themes {
                let (mut app, _) = App::boot(None);
                let _ = app.update(Message::Opened(Some(Ok(Box::new(doc.clone())))));
                themed(&mut app, mode);
                // Twice the pixels: the signs' fine lines and dots stay legible when the picture is enlarged.
                let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0))
                    .expect("a renderer")
                    .scale(2.0);
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                app.viewport.camera.fit(&b, 24.0);
                picture(
                    &mut snapshot,
                    &mut app,
                    dir.join(format!("katalog-{class}-{}{suffix}.png", page + 1)),
                );
            }
        }
    }
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn bohhbuy_manager_pictures() {
    use crate::style::manager::Event;
    use kentos_native_style::library::Source;
    let library = kentos_native_style::system::library();
    let (items, categories) = library.dump(Source::System);
    let dir = out();
    for (mode, suffix) in [("light", ""), ("dark", "-koyu")] {
        for c in &categories {
            let path: Vec<String> = c["path"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            if path.len() != 2 || path[0] != "BÖHHBÜY" || path[1] == "Piktogramlar" {
                continue;
            }
            let Some(first) = items.iter().find(|i| {
                i["kind"] == "symbol"
                    && i["path"].as_array().is_some_and(|p| {
                        p.iter()
                            .filter_map(Value::as_str)
                            .eq(path.iter().map(String::as_str))
                    })
            }) else {
                continue;
            };
            let (mut app, _) = App::boot(None);
            let _ = app.update(Message::Opened(Some(Ok(Box::new(town())))));
            themed(&mut app, mode);
            let _ = app.update(Message::Run("style.manager"));
            let _ = app.update(Message::StyleManager(Box::new(Event::Go(
                Source::System,
                path[..1].to_vec(),
                true,
            ))));
            let _ = app.update(Message::StyleManager(Box::new(Event::Go(
                Source::System,
                path.clone(),
                false,
            ))));
            let _ = app.update(Message::StyleManager(Box::new(Event::Press(
                first["id"].as_str().unwrap_or("").to_owned(),
            ))));
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let class = slug(path[1].split(' ').next().unwrap_or(""));
            picture(
                &mut snapshot,
                &mut app,
                dir.join(format!("yonetici-{class}{suffix}.png")),
            );
        }
    }
}

/// The signs close up for checking their fine lines (`yakin-*.png`): sixteen to a picture in a 4 × 4 grid of 24 × 16 m
/// cells at 1:1000, each with its number, all classes in order.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn bohhbuy_close_pictures() {
    use kentos_native_style::library::Source;
    let library = kentos_native_style::system::library();
    let (items, _) = library.dump(Source::System);
    let mut signs: Vec<&Value> = items
        .iter()
        .filter(|i| {
            i["kind"] == "symbol"
                && i["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("bohhbuy."))
        })
        .collect();
    signs.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let only = std::env::var("BOHHBUY_PAGE")
        .ok()
        .and_then(|p| p.parse::<usize>().ok());
    let dir = out();
    for (page, chunk) in signs.chunks(16).enumerate() {
        if only.is_some_and(|o| o != page + 1) {
            continue;
        }
        let mut entities: Vec<Value> = Vec::new();
        let mut add = |mut e: Value, attrs: Value| {
            e["id"] = json!(entities.len() + 1);
            e["attrs"] = attrs;
            entities.push(e);
        };
        let pts = |list: &[(f64, f64)]| list.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>();
        for (k, item) in chunk.iter().enumerate() {
            let (cx, cy) = ((k % 4) as f64 * 24.0 + 12.0, -((k / 4) as f64) * 16.0 - 8.0);
            let id = item["id"].as_str().unwrap_or("");
            let attrs = sample(id);
            match item["symbol"]["type"].as_str() {
                Some("marker") => add(
                    json!({ "kind": "point", "layerId": "ornek", "p": at(cx, cy + 1.0), "symbol": id }),
                    attrs,
                ),
                Some("line") => add(
                    json!({ "kind": "polyline", "layerId": "ornek",
                                            "pts": pts(&[(cx - 10.0, cy - 1.0), (cx + 4.0, cy - 1.0), (cx + 10.0, cy + 4.0)]),
                                            "bulges": [0, 0, 0], "symbol": id }),
                    attrs,
                ),
                _ => add(
                    json!({ "kind": "polygon", "layerId": "ornek",
                                 "pts": pts(&[(cx - 9.0, cy - 4.5), (cx + 9.0, cy - 4.5), (cx + 9.0, cy + 5.5), (cx - 9.0, cy + 5.5)]),
                                 "bulges": [0, 0, 0, 0], "symbol": id }),
                    attrs,
                ),
            }
            add(
                json!({ "kind": "text", "layerId": "yazilar", "p": at(cx - 11.5, cy + 7.0), "text": &id[8..], "height": 1.6,
                        "rotation": 0, "align": "topLeft" }),
                json!({}),
            );
        }
        let layer = |id: &str, name: &str| {
            json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
                    "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
        };
        let snapshot = json!({
            "format": "kentos.document", "version": 1, "name": "BÖHHBÜY işaretleri yakından",
            "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                          "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow" },
            "origin": at(0.0, 0.0),
            "layers": [layer("yazilar", "Numaralar"), layer("ornek", "Örnekler")],
            "activeLayer": "ornek",
            "entities": entities,
            "styles": { "items": [], "categories": [] },
        });
        let snapshot =
            DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("the sheet reads");
        let doc = Document::new(snapshot, None).expect("the sheet opens");
        let (mut app, _) = App::boot(None);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        themed(&mut app, "light");
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0))
            .expect("a renderer")
            .scale(2.0);
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        app.viewport.camera.fit(&bounds(0.0, -64.0, 96.0, 0.0), 8.0);
        picture(
            &mut snapshot,
            &mut app,
            dir.join(format!("yakin-{:02}.png", page + 1)),
        );
    }
}
