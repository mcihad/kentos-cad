//! Pictures of the general symbol library (docs/adr/9118) for the owner: Stil yöneticisi open at each of its
//! categories with its first symbol chosen (`yonetici-*.png`), and a small town drawn with its symbols only
//! (`kasaba-*.png`: the whole town and its middle closer), light and dark, in `.run/gui-evidence/genel-kitaplik/`.
//! Not a test of correctness and not run by default:
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib general_screens -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use iced::Size;
use kentos_commands::DocumentSnapshotV1;
use kentos_ui::snapshot::Snapshot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;

fn out() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/genel-kitaplik");
    std::fs::create_dir_all(&dir).expect("a folder for the pictures");
    dir
}

/// Frames until the atlas holds every image the view needs, then the picture.
fn picture(snapshot: &mut Snapshot, app: &mut App, file: PathBuf) {
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.settle(app, App::view, &mut update);
    for _ in 0..60 {
        let _ = snapshot.render(app.view(), &app.theme());
        if !app.viewport.status().images_pending {
            break;
        }
    }
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
    println!("{}", file.display());
}

fn themed(app: &mut App, mode: &str) {
    let _ = app.settings.choose(&[("appearance.theme", Value::from(mode))]);
    app.apply_settings();
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn manager_pictures() {
    use crate::style::manager::Event;
    use kentos_native_style::library::Source;
    let library = kentos_native_style::system::library();
    let (items, categories) = library.dump(Source::System);
    let paths: Vec<Vec<String>> = categories
        .iter()
        .filter_map(|c| c["path"].as_array())
        .map(|p| p.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>())
        .filter(|p| p.len() == 3 && p[0] == "Genel")
        .collect();
    let dir = out();
    for (mode, suffix) in [("light", ""), ("dark", "-koyu")] {
        for path in &paths {
            let Some(first) = items.iter().find(|i| {
                i["kind"] == "symbol"
                    && i["path"].as_array().is_some_and(|p| p.iter().filter_map(Value::as_str).eq(path.iter().map(String::as_str)))
            }) else {
                continue;
            };
            let (mut app, _) = App::boot(None);
            let _ = app.update(Message::Opened(Some(Ok(Box::new(town())))));
            themed(&mut app, mode);
            let _ = app.update(Message::Run("style.manager"));
            let _ = app.update(Message::StyleManager(Box::new(Event::Go(Source::System, path[..2].to_vec(), true))));
            let _ = app.update(Message::StyleManager(Box::new(Event::Go(Source::System, path.clone(), false))));
            let _ = app.update(Message::StyleManager(Box::new(Event::Press(first["id"].as_str().unwrap_or("").to_owned()))));
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let name = slug(&format!("{}-{}", path[1], path[2]));
            picture(&mut snapshot, &mut app, dir.join(format!("yonetici-{name}{suffix}.png")));
        }
    }
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn town_pictures() {
    let dir = out();
    for (mode, suffix) in [("light", ""), ("dark", "-koyu")] {
        let (mut app, _) = App::boot(None);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(town())))));
        themed(&mut app, mode);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let fit = |app: &mut App, b: kentos_render_wgpu::Bounds| app.viewport.camera.fit(&b, 24.0);
        fit(&mut app, bounds(0.0, 0.0, 520.0, 320.0));
        picture(&mut snapshot, &mut app, dir.join(format!("kasaba{suffix}.png")));
        fit(&mut app, bounds(170.0, 30.0, 380.0, 170.0));
        picture(&mut snapshot, &mut app, dir.join(format!("kasaba-orta{suffix}.png")));
        fit(&mut app, bounds(20.0, 140.0, 260.0, 320.0));
        picture(&mut snapshot, &mut app, dir.join(format!("kasaba-gol-orman{suffix}.png")));
    }
}

/// Each category of the library as a sheet at 1:1000 (`katalog-*.png`): every symbol on a sample object with its name.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn catalog_pictures() {
    use kentos_native_style::library::Source;
    let library = kentos_native_style::system::library();
    let (items, categories) = library.dump(Source::System);
    let dir = out();
    for c in &categories {
        let path: Vec<&str> = c["path"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        if path.len() != 3 || path[0] != "Genel" {
            continue;
        }
        let mut members: Vec<&Value> = items
            .iter()
            .filter(|i| i["kind"] == "symbol" && i["path"].as_array().is_some_and(|p| p.iter().filter_map(Value::as_str).eq(path.iter().copied())))
            .collect();
        members.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        if members.is_empty() {
            continue;
        }
        let (doc, b) = catalog(&members);
        for (mode, suffix) in [("light", ""), ("dark", "-koyu")] {
            let (mut app, _) = App::boot(None);
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc.clone())))));
            themed(&mut app, mode);
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            app.viewport.camera.fit(&b, 24.0);
            let name = slug(&format!("{}-{}", path[1], path[2]));
            picture(&mut snapshot, &mut app, dir.join(format!("katalog-{name}{suffix}.png")));
        }
    }
}

/// A sheet of one category's symbols at 1:1000, each on a sample object with its name: points in rows of five, lines
/// one under another with a bend (to show the joins), areas in rows of four.
fn catalog(members: &[&Value]) -> (Document, kentos_render_wgpu::Bounds) {
    let mut entities: Vec<Value> = Vec::new();
    let mut add = |mut e: Value| {
        e["id"] = json!(entities.len() + 1);
        e["attrs"] = json!({});
        entities.push(e);
    };
    let name = |t: &str, x: f64, y: f64, align: &str| {
        json!({ "kind": "text", "layerId": "yazilar", "p": at(x, y), "text": t, "height": 2.2, "rotation": 0,
                "align": align })
    };
    let kind = members[0]["symbol"]["type"].as_str().unwrap_or("");
    let (mut x1, mut y0) = (0.0_f64, 0.0_f64);
    for (k, item) in members.iter().enumerate() {
        let id = item["id"].as_str().unwrap_or("");
        let label = item["name"].as_str().unwrap_or("");
        match kind {
            "marker" => {
                let (col, row) = ((k % 5) as f64, (k / 5) as f64);
                let (x, y) = (col * 28.0 + 14.0, -row * 17.0);
                add(json!({ "kind": "point", "layerId": "ornek", "p": at(x, y), "symbol": id }));
                add(name(label, x, y - 5.5, "middleCenter"));
                x1 = x1.max(col * 28.0 + 28.0);
                y0 = y0.min(y - 9.0);
            }
            "line" => {
                let y = -(k as f64) * 10.0;
                let pts = [(0.0, y), (40.0, y), (52.0, y + 5.0)];
                add(json!({ "kind": "polyline", "layerId": "ornek", "pts": pts.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>(),
                            "bulges": [0, 0, 0], "symbol": id }));
                add(name(label, 58.0, y, "middleLeft"));
                x1 = x1.max(110.0);
                y0 = y0.min(y - 6.0);
            }
            _ => {
                let (col, row) = ((k % 4) as f64, (k / 4) as f64);
                let (x, y) = (col * 48.0, -row * 38.0);
                let pts = [(x, y), (x + 40.0, y), (x + 40.0, y + 24.0), (x, y + 24.0)];
                add(json!({ "kind": "polygon", "layerId": "ornek", "pts": pts.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>(),
                            "bulges": [0, 0, 0, 0], "symbol": id }));
                add(name(label, x + 20.0, y - 4.5, "middleCenter"));
                x1 = x1.max(col * 48.0 + 40.0);
                y0 = y0.min(y - 8.0);
            }
        }
    }
    let top = match kind {
        "fill" => 28.0,
        "marker" => 6.0,
        _ => 10.0,
    };
    let layer = |id: &str, name: &str| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Genel semboller kataloğu",
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
    (doc, bounds(-6.0, y0, x1 + 6.0, top))
}

fn slug(s: &str) -> String {
    s.replace('İ', "i")
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// The town's origin: a TM36 point, so the coordinates below are metres east and north of it.
const O: (f64, f64) = (486_000.0, 4_420_000.0);

fn bounds(x0: f64, y0: f64, x1: f64, y1: f64) -> kentos_render_wgpu::Bounds {
    kentos_render_wgpu::Bounds { min_x: O.0 + x0, min_y: O.1 + y0, max_x: O.0 + x1, max_y: O.1 + y1 }
}

fn at(x: f64, y: f64) -> Value {
    json!({ "x": O.0 + x, "y": O.1 + y })
}

/// A drawing of a small town with the general library's symbols on its objects (`symbol`), at 1:2000.
pub(super) fn town() -> Document {
    let mut entities: Vec<Value> = Vec::new();
    let mut add = |mut e: Value| {
        e["id"] = json!(entities.len() + 1);
        e["attrs"] = json!({});
        entities.push(e);
    };
    let area = |layer: &str, pts: &[(f64, f64)], symbol: &str| {
        json!({ "kind": "polygon", "layerId": layer, "pts": pts.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>(),
                "bulges": vec![0; pts.len()], "symbol": format!("genel.alan.{symbol}") })
    };
    let line = |pts: &[(f64, f64)], symbol: &str| {
        json!({ "kind": "polyline", "layerId": "cizgiler", "pts": pts.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>(),
                "bulges": vec![0; pts.len()], "symbol": format!("genel.cizgi.{symbol}") })
    };
    let point = |x: f64, y: f64, symbol: &str| {
        json!({ "kind": "point", "layerId": "noktalar", "p": at(x, y), "symbol": format!("genel.nokta.{symbol}") })
    };
    let label = |x: f64, y: f64, t: &str, h: f64, rotation: f64| {
        json!({ "kind": "text", "layerId": "yazilar", "p": at(x, y), "text": t, "height": h, "rotation": rotation })
    };
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    let ring = |cx: f64, cy: f64, r: f64, n: usize, wobble: f64| -> Vec<(f64, f64)> {
        (0..n)
            .map(|k| {
                let a = k as f64 / n as f64 * std::f64::consts::TAU;
                let rr = r * (1.0 + wobble * (3.0 * a).sin() * 0.5 + wobble * (5.0 * a).cos() * 0.3);
                (cx + rr * a.cos(), cy + rr * a.sin() * 0.8)
            })
            .collect()
    };

    // Land cover and use.
    add(area("alanlar", &[(0.0, 230.0), (95.0, 236.0), (125.0, 320.0), (0.0, 320.0)], "arazi.orman"));
    add(area("alanlar", &[(125.0, 262.0), (250.0, 252.0), (268.0, 320.0), (125.0, 320.0)], "arazi.orman-ibreli"));
    add(area("alanlar", &[(250.0, 252.0), (335.0, 262.0), (345.0, 320.0), (268.0, 320.0)], "arazi.fundalik"));
    add(area("alanlar", &[(370.0, 248.0), (470.0, 240.0), (520.0, 290.0), (520.0, 320.0), (350.0, 320.0)], "arazi.kayalik"));
    add(area("alanlar", &[(0.0, 112.0), (62.0, 116.0), (66.0, 170.0), (40.0, 214.0), (0.0, 222.0)], "arazi.cayir"));
    add(area("alanlar", &[(430.0, 118.0), (520.0, 118.0), (520.0, 232.0), (440.0, 236.0)], "arazi.tarim"));
    add(area("alanlar", &[(338.0, 178.0), (428.0, 178.0), (432.0, 234.0), (346.0, 244.0)], "arazi.bag"));
    add(area("alanlar", &ring(112.0, 190.0, 42.0, 18, 0.18), "su.gol"));
    add(area("alanlar", &[(146.0, 168.0), (186.0, 172.0), (190.0, 214.0), (150.0, 216.0)], "su.sulak-alan"));
    add(area("alanlar", &[(60.0, 112.0), (110.0, 112.0), (112.0, 22.0), (66.0, 22.0)], "kisit.taskin"));
    add(area("alanlar", &rect(198.0, 120.0, 332.0, 166.0), "kullanim.konut"));
    add(area("alanlar", &rect(198.0, 34.0, 282.0, 96.0), "kullanim.ticaret"));
    add(area("alanlar", &rect(382.0, 32.0, 520.0, 96.0), "kullanim.sanayi"));
    add(area("alanlar", &rect(288.0, 34.0, 362.0, 96.0), "kullanim.park"));
    add(area("alanlar", &rect(122.0, 30.0, 176.0, 70.0), "kullanim.mezarlik"));
    add(area("alanlar", &rect(342.0, 120.0, 404.0, 166.0), "kullanim.spor"));
    add(area("alanlar", &rect(132.0, 76.0, 180.0, 98.0), "kullanim.otopark"));
    // Buildings.
    for (k, x) in [204.0, 222.0, 240.0, 270.0, 288.0, 306.0].iter().enumerate() {
        add(area("binalar", &rect(*x, 128.0, x + 13.0, 138.0), if k == 1 { "yapi.kamu-binasi" } else { "yapi.bina" }));
        add(area("binalar", &rect(*x, 148.0, x + 13.0, 160.0), "yapi.bina"));
    }
    for x in [204.0, 226.0, 248.0] {
        add(area("binalar", &rect(x, 44.0, x + 16.0, 58.0), "yapi.bina"));
        add(area("binalar", &rect(x, 70.0, x + 16.0, 86.0), "yapi.bina"));
    }
    add(area("binalar", &rect(392.0, 44.0, 430.0, 70.0), "yapi.bina"));
    add(area("binalar", &rect(446.0, 44.0, 500.0, 64.0), "yapi.insaat"));
    add(area("binalar", &rect(446.0, 72.0, 506.0, 90.0), "yapi.sera"));
    // Water, rail, power, contours, roads.
    add(line(&[(98.0, 152.0), (84.0, 128.0), (82.0, 104.0), (94.0, 74.0), (88.0, 44.0), (84.0, 0.0)], "su.nehir"));
    add(line(&[(244.0, 252.0), (226.0, 236.0), (204.0, 230.0), (186.0, 214.0)], "su.akis-yonlu"));
    for (r, kind) in [(18.0, "topo.es-yukselti-ara"), (32.0, "topo.es-yukselti-ara"), (46.0, "topo.es-yukselti-ana")] {
        let mut pts = ring(452.0, 292.0, r, 28, 0.08);
        pts.push(pts[0]);
        add(line(&pts, kind));
    }
    add(line(&[(180.0, 320.0), (520.0, 160.0)], "rayli.demiryolu"));
    add(line(&[(312.0, 320.0), (520.0, 214.0)], "hat.yuksek-gerilim"));
    add(line(&[(20.0, 246.0), (52.0, 274.0), (96.0, 300.0), (130.0, 316.0)], "yol.patika"));
    add(line(&[(290.0, 30.0), (362.0, 30.0)], "yol.bisiklet"));
    add(line(&[(260.0, 114.0), (260.0, 170.0)], "yol.sokak"));
    add(line(&[(192.0, 142.0), (336.0, 142.0)], "yol.sokak"));
    add(line(&[(192.0, 64.0), (284.0, 64.0)], "yol.sokak"));
    add(line(&[(190.0, 14.0), (190.0, 246.0)], "yol.cadde"));
    add(line(&[(0.0, 104.0), (140.0, 106.0), (260.0, 108.0), (400.0, 106.0), (520.0, 112.0)], "yol.devlet-yolu"));
    add(line(&[(0.0, 12.0), (520.0, 20.0)], "yol.otoyol"));
    add(line(&[(76.0, 105.0), (90.0, 105.0)], "yol.kopru"));
    add(line(&[(192.0, 116.0), (338.0, 116.0), (338.0, 170.0), (192.0, 170.0), (192.0, 116.0)], "sinir.mahalle"));
    // Places.
    for (x, y, s) in [
        (196.0, 96.0, "ulasim.otobus-duragi"),
        (150.0, 112.0, "ulasim.akaryakit"),
        (302.0, 266.0, "ulasim.tren-istasyonu"),
        (232.0, 132.0, "egitim.okul"),
        (318.0, 132.0, "saglik.hastane"),
        (238.0, 64.0, "saglik.eczane"),
        (214.0, 112.0, "ibadet.cami"),
        (266.0, 50.0, "ticaret.market"),
        (212.0, 50.0, "kamu.polis"),
        (300.0, 84.0, "ticaret.kafe"),
        (326.0, 58.0, "spor.park"),
        (348.0, 46.0, "spor.oyun-alani"),
        (372.0, 144.0, "spor.spor-sahasi"),
        (276.0, 84.0, "ticaret.otel"),
        (316.0, 46.0, "saglik.toplanma-alani"),
        (58.0, 288.0, "ticaret.kamp"),
        (476.0, 302.0, "ticaret.seyir"),
        (452.0, 292.0, "olcme.nirengi"),
        (246.0, 254.0, "doga.kaynak"),
        (150.0, 54.0, "ibadet.mezarlik"),
    ] {
        add(point(x, y, s));
    }
    for x in [204.0, 216.0, 228.0, 240.0, 252.0, 272.0, 284.0, 296.0, 308.0, 320.0] {
        add(point(x, 120.0, "doga.agac"));
    }
    for (x, y) in [(330.0, 300.0), (344.0, 286.0), (360.0, 276.0)] {
        add(point(x, y, "doga.konifer"));
    }
    // Names.
    add(label(92.0, 192.0, "Kale Gölü", 7.0, 0.0));
    add(label(195.0, 196.0, "Atatürk Caddesi", 4.5, 90.0));
    add(label(30.0, 108.0, "D-100", 4.5, 0.0));
    add(label(214.0, 152.0, "Cumhuriyet Mahallesi", 4.5, 0.0));
    add(label(420.0, 76.0, "Sanayi", 6.0, 0.0));
    add(label(170.0, 292.0, "Çam Ormanı", 6.5, 0.0));
    add(label(460.0, 160.0, "Tarla", 6.0, 0.0));

    let layer = |id: &str, name: &str| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Genel semboller kasabası",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 2000, "workspace": "gis", "drawingFont": "barlow" },
        "origin": at(0.0, 0.0),
        "layers": [layer("yazilar", "Yazılar"), layer("noktalar", "Noktalar"), layer("cizgiler", "Çizgiler"),
                   layer("binalar", "Binalar"), layer("alanlar", "Alanlar")],
        "activeLayer": "noktalar",
        "entities": entities,
        "styles": { "items": [], "categories": [] },
        "homeView": { "minX": O.0, "minY": O.1, "maxX": O.0 + 520.0, "maxY": O.1 + 320.0 },
    });
    let snapshot = DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("the town reads");
    Document::new(snapshot, None).expect("the town opens")
}
