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
        .filter(|p| (p.len() == 3 && p[0] == "Genel") || (p.len() == 2 && p[0] == "Kent"))
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
            let _ = app.update(Message::StyleManager(Box::new(Event::Go(Source::System, path[..path.len() - 1].to_vec(), true))));
            let _ = app.update(Message::StyleManager(Box::new(Event::Go(Source::System, path.clone(), false))));
            let _ = app.update(Message::StyleManager(Box::new(Event::Press(first["id"].as_str().unwrap_or("").to_owned()))));
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let name = slug(&path.join("-")).trim_start_matches("genel-").to_owned();
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
        let _ = app.update(Message::Opened(Some(Ok(Box::new(neighbourhood())))));
        snapshot.settle(&mut app, App::view, &mut update);
        fit(&mut app, bounds(0.0, 0.0, 200.0, 140.0));
        picture(&mut snapshot, &mut app, dir.join(format!("mahalle{suffix}.png")));
        fit(&mut app, bounds(108.0, 78.0, 200.0, 140.0));
        picture(&mut snapshot, &mut app, dir.join(format!("mahalle-park{suffix}.png")));
        fit(&mut app, bounds(60.0, 34.0, 136.0, 100.0));
        picture(&mut snapshot, &mut app, dir.join(format!("mahalle-kavsak{suffix}.png")));
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
        if !((path.len() == 3 && path[0] == "Genel") || (path.len() == 2 && path[0] == "Kent")) {
            continue;
        }
        let mut members: Vec<&Value> = items
            .iter()
            .filter(|i| i["kind"] == "symbol" && i["path"].as_array().is_some_and(|p| p.iter().filter_map(Value::as_str).eq(path.iter().copied())))
            .collect();
        // Points, then lines, then areas; each by name.
        let rank = |v: &Value| match v["symbol"]["type"].as_str() {
            Some("marker") => 0,
            Some("line") => 1,
            _ => 2,
        };
        members.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a["name"].as_str().cmp(&b["name"].as_str())));
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
            let name = slug(&path.join("-")).trim_start_matches("genel-").to_owned();
            picture(&mut snapshot, &mut app, dir.join(format!("katalog-{name}{suffix}.png")));
        }
    }
}

/// A sheet of one category's symbols at 1:1000, each on a sample object with its name, kind by kind: points in rows
/// of five, then lines one under another with a bend (to show the joins), then areas in rows of four.
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
    let points: Vec<&&Value> = members.iter().filter(|m| m["symbol"]["type"] == "marker").collect();
    let lines: Vec<&&Value> = members.iter().filter(|m| m["symbol"]["type"] == "line").collect();
    let areas: Vec<&&Value> = members.iter().filter(|m| m["symbol"]["type"] == "fill").collect();
    let (mut x1, mut y) = (0.0_f64, 0.0_f64);
    let top = if points.is_empty() && lines.is_empty() { 28.0 } else { 10.0 };
    for (k, item) in points.iter().enumerate() {
        let (col, row) = ((k % 5) as f64, (k / 5) as f64);
        let (x, py) = (col * 28.0 + 14.0, y - row * 17.0);
        add(json!({ "kind": "point", "layerId": "ornek", "p": at(x, py), "symbol": item["id"] }));
        add(name(item["name"].as_str().unwrap_or(""), x, py - 6.0, "middleCenter"));
        x1 = x1.max(col * 28.0 + 28.0);
    }
    if !points.is_empty() {
        y -= (points.len().div_ceil(5) as f64) * 17.0 + 2.0;
    }
    for (k, item) in lines.iter().enumerate() {
        let ly = y - k as f64 * 10.0 - 4.0;
        let pts = [(0.0, ly), (40.0, ly), (52.0, ly + 5.0)];
        add(json!({ "kind": "polyline", "layerId": "ornek", "pts": pts.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>(),
                    "bulges": [0, 0, 0], "symbol": item["id"] }));
        add(name(item["name"].as_str().unwrap_or(""), 58.0, ly, "middleLeft"));
        x1 = x1.max(110.0);
    }
    if !lines.is_empty() {
        y -= lines.len() as f64 * 10.0 + 4.0;
    }
    for (k, item) in areas.iter().enumerate() {
        let (col, row) = ((k % 4) as f64, (k / 4) as f64);
        let (x, ay) = (col * 48.0, y - 28.0 - row * 38.0);
        let pts = [(x, ay), (x + 40.0, ay), (x + 40.0, ay + 24.0), (x, ay + 24.0)];
        add(json!({ "kind": "polygon", "layerId": "ornek", "pts": pts.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>(),
                    "bulges": [0, 0, 0, 0], "symbol": item["id"] }));
        add(name(item["name"].as_str().unwrap_or(""), x + 20.0, ay - 4.5, "middleCenter"));
        x1 = x1.max(col * 48.0 + 40.0);
    }
    if !areas.is_empty() {
        y -= 28.0 + (areas.len().div_ceil(4) as f64 - 1.0) * 38.0 + 10.0;
    }
    let layer = |id: &str, name: &str| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Sembol kataloğu",
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
    (doc, bounds(-6.0, y - 2.0, x1 + 6.0, top))
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

/// A neighbourhood at 1:500 drawn with the Kent branch only (docs/adr/9119): a junction with its sidewalks, kerbs,
/// lane markings, crossings, arrows, lights and signs; a park with its lawn, paths, fountain, beds, shrubs, hedges,
/// trees and playground; a plaza with its street furniture; houses with front gardens; utilities along the streets.
pub(super) fn neighbourhood() -> Document {
    let mut entities: Vec<Value> = Vec::new();
    let mut add = |mut e: Value| {
        e["id"] = json!(entities.len() + 1);
        if e.get("attrs").is_none() {
            e["attrs"] = json!({});
        }
        entities.push(e);
    };
    let poly = |layer: &str, pts: &[(f64, f64)], symbol: &str| {
        json!({ "kind": "polygon", "layerId": layer, "pts": pts.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>(),
                "bulges": vec![0; pts.len()], "symbol": symbol })
    };
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    let line = |pts: &[(f64, f64)], symbol: &str| {
        json!({ "kind": "polyline", "layerId": "cizgiler", "pts": pts.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>(),
                "bulges": vec![0; pts.len()], "symbol": format!("kent.cizgi.{symbol}") })
    };
    let point = |x: f64, y: f64, symbol: &str| {
        json!({ "kind": "point", "layerId": "noktalar", "p": at(x, y), "symbol": symbol })
    };
    let with = |mut e: Value, attrs: Value| {
        e["attrs"] = attrs;
        e
    };
    let circle_pts = |cx: f64, cy: f64, r: f64| -> Vec<(f64, f64)> {
        (0..20).map(|k| {
            let a = k as f64 / 20.0 * std::f64::consts::TAU;
            (cx + r * a.cos(), cy + r * a.sin())
        }).collect()
    };
    let k = |s: &str| format!("kent.alan.{s}");

    // Paving: roads, sidewalks, the plaza, the parking bays.
    add(poly("alanlar", &rect(0.0, 60.0, 200.0, 74.0), &k("kaplama.asfalt")));
    add(poly("alanlar", &rect(92.0, 0.0, 104.0, 140.0), &k("kaplama.asfalt")));
    for r in [rect(0.0, 56.0, 92.0, 60.0), rect(88.0, 0.0, 92.0, 56.0), rect(104.0, 56.0, 200.0, 60.0),
              rect(104.0, 0.0, 108.0, 56.0), rect(0.0, 74.0, 92.0, 78.0), rect(88.0, 78.0, 92.0, 140.0),
              rect(104.0, 74.0, 200.0, 78.0), rect(104.0, 78.0, 108.0, 140.0)] {
        add(poly("alanlar", &r, &k("kaplama.kaldirim")));
    }
    add(poly("alanlar", &rect(110.0, 4.0, 198.0, 54.0), &k("kaplama.kilit-parke")));
    add(poly("alanlar", &rect(10.0, 51.0, 70.0, 55.8), &k("kaplama.park-cebi")));
    add(poly("alanlar", &rect(70.0, 51.0, 76.0, 55.8), &k("kaplama.engelli-park")));
    add(poly("alanlar", &rect(4.0, 4.0, 84.0, 26.0), &k("kaplama.toprak")));
    // The park.
    add(poly("alanlar", &rect(110.0, 80.0, 198.0, 138.0), &k("bitki.cim")));
    add(poly("alanlar", &rect(110.0, 105.0, 198.0, 109.0), &k("park.stabilize")));
    add(poly("alanlar", &rect(150.0, 80.0, 154.0, 138.0), &k("park.stabilize")));
    add(poly("alanlar", &circle_pts(152.0, 107.0, 7.0), &k("kaplama.dogal-tas")));
    add(poly("alanlar", &circle_pts(152.0, 107.0, 4.5), &k("park.sus-havuzu")));
    add(poly("alanlar", &rect(160.0, 114.0, 196.0, 136.0), &k("oyun.kaucuk-zemin")));
    add(poly("alanlar", &rect(180.0, 116.0, 188.0, 124.0), &k("oyun.kum-zemin")));
    add(poly("alanlar", &rect(116.0, 112.0, 146.0, 118.0), &k("bitki.cicek-tarhi")));
    add(poly("alanlar", &rect(116.0, 124.0, 144.0, 134.0), &k("bitki.cali-grubu")));
    add(poly("alanlar", &rect(158.0, 84.0, 170.0, 100.0), &k("bitki.otsu")));
    add(poly("alanlar", &rect(186.0, 96.0, 196.0, 103.0), &k("park.ahsap-deck")));
    // Front gardens.
    add(poly("alanlar", &rect(4.0, 80.0, 86.0, 92.0), &k("bitki.cim")));
    add(poly("alanlar", &rect(40.0, 82.0, 50.0, 90.0), &k("bitki.mevsimlik")));
    // Buildings (the general branch's).
    for r in [rect(10.0, 30.0, 40.0, 48.0), rect(48.0, 30.0, 80.0, 48.0), rect(10.0, 96.0, 40.0, 124.0),
              rect(50.0, 96.0, 80.0, 124.0), rect(128.0, 14.0, 188.0, 38.0)] {
        add(poly("binalar", &r, "genel.alan.yapi.bina"));
    }
    // Kerbs, markings, crossings, a speed hump, a drain along the kerb, hedges, a row of trees, bollards.
    for (a, b) in [((0.0, 60.0), (92.0, 60.0)), ((104.0, 60.0), (200.0, 60.0)), ((0.0, 74.0), (92.0, 74.0)),
                   ((104.0, 74.0), (200.0, 74.0)), ((92.0, 0.0), (92.0, 60.0)), ((104.0, 0.0), (104.0, 60.0)),
                   ((92.0, 74.0), (92.0, 140.0)), ((104.0, 74.0), (104.0, 140.0))] {
        add(line(&[a, b], "yol.bordur"));
    }
    add(line(&[(0.0, 67.0), (80.0, 67.0)], "yol.serit-kesikli"));
    add(line(&[(116.0, 67.0), (200.0, 67.0)], "yol.serit-kesikli"));
    add(line(&[(98.0, 0.0), (98.0, 46.0)], "yol.serit-kesikli"));
    add(line(&[(98.0, 88.0), (98.0, 140.0)], "yol.serit-kesikli"));
    add(line(&[(80.0, 67.0), (84.0, 67.0)], "yol.cift-surekli"));
    add(line(&[(82.0, 60.0), (82.0, 67.0)], "yol.durma-cizgisi"));
    add(line(&[(114.0, 67.0), (114.0, 74.0)], "yol.durma-cizgisi"));
    add(line(&[(98.0, 48.0), (104.0, 48.0)], "yol.durma-cizgisi"));
    add(line(&[(92.0, 86.0), (98.0, 86.0)], "yol.durma-cizgisi"));
    add(line(&[(86.0, 60.0), (86.0, 74.0)], "yol.yaya-gecidi"));
    add(line(&[(110.0, 60.0), (110.0, 74.0)], "yol.yaya-gecidi"));
    add(line(&[(92.0, 52.0), (104.0, 52.0)], "yol.yaya-gecidi"));
    add(line(&[(92.0, 82.0), (104.0, 82.0)], "yol.yaya-gecidi"));
    add(line(&[(92.0, 18.0), (104.0, 18.0)], "yol.hiz-kesici"));
    add(line(&[(120.0, 61.6), (200.0, 61.6)], "yol.otobus-seridi"));
    add(line(&[(0.0, 60.8), (82.0, 60.8)], "altyapi.drenaj"));
    add(line(&[(110.0, 80.0), (198.0, 80.0)], "bitki.cit-bitkisi"));
    add(line(&[(110.0, 80.5), (110.0, 138.0)], "bitki.cit-bitkisi"));
    add(line(&[(4.0, 92.5), (86.0, 92.5)], "bitki.dogal-cit"));
    add(line(&[(6.0, 76.2), (86.0, 76.2)], "agac.agac-sirasi"));
    add(line(&[(112.0, 55.0), (196.0, 55.0)], "mobilya.babalar"));
    add(line(&[(116.0, 109.0), (146.0, 109.0)], "bitki.bordur-bitkisi"));
    // Road arrows, turned by their Açı.
    for (x, y, s, turn) in [(70.0, 63.5, "yol.ok-ileri", -90.0), (60.0, 63.5, "yol.ok-sag", -90.0),
                            (128.0, 70.5, "yol.ok-ileri", 90.0), (101.0, 36.0, "yol.ok-ileri-sag", 0.0),
                            (95.0, 100.0, "yol.ok-sol", 180.0)] {
        add(with(point(x, y, &format!("kent.nokta.{s}")), json!({ "Açı": turn.to_string() })));
    }
    // Trees and shrubs, at their crowns' size.
    for (x, y, s, crown) in [(120.0, 90.0, "agac.yaprakli", 8.0), (136.0, 92.0, "agac.yaprakli", 6.0),
                             (126.0, 99.0, "agac.yaprakli", 5.0), (190.0, 90.0, "agac.igne-yaprakli", 5.0),
                             (162.0, 128.0, "agac.yaprakli", 7.0), (174.0, 92.0, "agac.anit", 10.0),
                             (140.0, 128.0, "agac.servi", 2.0), (146.0, 98.0, "agac.palmiye", 4.0),
                             (20.0, 15.0, "agac.kesilecek", 6.0), (34.0, 15.0, "agac.tasinacak", 5.0),
                             (48.0, 15.0, "agac.dikilecek", 5.0), (62.0, 15.0, "agac.dikilecek", 5.0),
                             (76.0, 15.0, "agac.korunacak", 7.0), (30.0, 132.0, "agac.meyve", 5.0),
                             (60.0, 132.0, "agac.meyve", 5.0), (196.0, 118.0, "bitki.cali", 1.6),
                             (114.0, 120.0, "bitki.ardic", 1.4), (148.0, 88.0, "bitki.cali", 2.0),
                             (8.0, 88.0, "agac.fidan", 1.5), (20.0, 88.0, "agac.fidan", 1.5)] {
        add(with(point(x, y, &format!("kent.nokta.{s}")), json!({ "Taç": crown.to_string() })));
    }
    // Park and play.
    for (x, y, s) in [(152.0, 107.0, "park.fiskiye"), (162.0, 99.0, "park.cesme"), (191.0, 99.5, "park.pergola"),
                      (122.0, 106.0, "park.kamelya"), (152.0, 96.0, "park.heykel"), (196.0, 84.0, "park.wc"),
                      (114.0, 84.0, "park.pano"), (130.0, 84.0, "park.kopek-parki"), (166.0, 131.0, "oyun.salincak"),
                      (176.0, 131.0, "oyun.kaydirak"), (190.0, 131.0, "oyun.tahterevalli"), (170.0, 120.0, "oyun.donence"),
                      (184.0, 120.0, "oyun.kum-havuzu"), (193.0, 120.0, "oyun.tirmanma"), (140.0, 103.0, "mobilya.bank"),
                      (164.0, 103.0, "mobilya.bank"), (156.0, 90.0, "mobilya.bank"), (148.0, 122.0, "mobilya.bank"),
                      (144.0, 111.0, "mobilya.cop-kutusu"), (160.0, 111.0, "mobilya.cop-kutusu"),
                      (130.0, 103.0, "park.park-lambasi"), (172.0, 103.0, "park.park-lambasi"),
                      (156.0, 84.0, "park.park-lambasi"), (156.0, 132.0, "park.park-lambasi")] {
        add(point(x, y, &format!("kent.nokta.{s}")));
    }
    // The plaza's and the streets' furniture.
    for (k, s) in ["bufe", "atm", "posta-kutusu", "bisiklet-park", "geri-donusum", "konteyner", "reklam-panosu", "saat",
                   "kamera", "bilgi-ekrani"].iter().enumerate() {
        add(point(116.0 + k as f64 * 8.5, 47.0, &format!("kent.nokta.mobilya.{s}")));
    }
    for (x, y) in [(116.0, 30.0), (116.0, 38.0), (194.0, 30.0), (194.0, 38.0)] {
        add(point(x, y, "kent.nokta.mobilya.saksi"));
    }
    add(point(40.0, 58.0, "kent.nokta.mobilya.durak"));
    add(point(52.0, 58.0, "kent.nokta.mobilya.parkmetre"));
    add(point(78.0, 58.0, "kent.nokta.mobilya.engelli"));
    for (x, y) in [(10.0, 58.0), (30.0, 58.0), (64.0, 58.0), (20.0, 76.0), (45.0, 76.0), (70.0, 76.0), (124.0, 76.0),
                   (150.0, 76.0), (176.0, 76.0), (124.0, 58.0), (160.0, 58.0), (190.0, 58.0), (106.0, 20.0),
                   (90.0, 30.0), (90.0, 110.0), (106.0, 120.0)] {
        add(point(x, y, "kent.nokta.mobilya.sokak-lambasi"));
    }
    add(point(98.0, 67.0, "kent.nokta.mobilya.sokak-lambasi-cift"));
    // Lights and signs.
    for (x, y) in [(90.5, 58.5), (105.5, 58.5), (90.5, 75.5), (105.5, 75.5)] {
        add(point(x, y, "kent.nokta.trafik.trafik-isigi"));
    }
    add(point(84.0, 57.5, "kent.nokta.trafik.yaya-isigi"));
    add(point(112.0, 76.5, "kent.nokta.trafik.yaya-isigi"));
    add(point(106.0, 46.0, "kent.nokta.trafik.dur"));
    add(point(84.0, 77.0, "kent.nokta.trafik.yaya-gecidi"));
    add(with(point(90.0, 20.0, "kent.nokta.trafik.hiz-siniri"), json!({ "Hız": "30" })));
    add(point(90.0, 96.0, "kent.nokta.trafik.park-yapilmaz"));
    add(point(106.0, 92.0, "kent.nokta.trafik.yaya-gecidi-uyari"));
    // Utilities.
    for (x, y, s) in [(40.0, 67.0, "altyapi.muayene-bacasi"), (150.0, 67.0, "altyapi.muayene-bacasi"),
                      (98.0, 30.0, "altyapi.muayene-bacasi"), (98.0, 110.0, "altyapi.muayene-bacasi"),
                      (80.0, 60.7, "altyapi.yagmur-suyu-bacasi"), (116.0, 73.3, "altyapi.yagmur-suyu-bacasi"),
                      (92.7, 40.0, "altyapi.yagmur-suyu-bacasi"), (103.3, 100.0, "altyapi.yagmur-suyu-bacasi"),
                      (106.0, 58.0, "altyapi.vana-odasi"), (90.0, 79.5, "altyapi.elektrik-panosu"),
                      (106.0, 79.5, "altyapi.fiber-kutusu"), (60.0, 58.0, "altyapi.telekom-menholu"),
                      (186.0, 112.0, "altyapi.yagmurlama"), (126.0, 114.0, "altyapi.yagmurlama"),
                      (112.0, 92.0, "altyapi.sulama-vanasi"), (20.0, 58.0, "altyapi.su-sayaci")] {
        add(point(x, y, &format!("kent.nokta.{s}")));
    }
    add(point(91.0, 62.0, "genel.nokta.altyapi.hidrant"));
    // Names.
    for (x, y, t, h, turn) in [(14.0, 63.7, "Cumhuriyet Caddesi", 1.8, 0.0), (101.2, 112.0, "Atatürk Bulvarı", 1.8, 90.0),
                               (124.0, 94.0, "Kent Parkı", 2.4, 0.0), (140.0, 26.0, "Çarşı Meydanı", 2.4, 0.0)] {
        add(json!({ "kind": "text", "layerId": "yazilar", "p": at(x, y), "text": t, "height": h, "rotation": turn }));
    }

    let layer = |id: &str, name: &str| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Kent sembolleri mahallesi",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 500, "workspace": "gis", "drawingFont": "barlow" },
        "origin": at(0.0, 0.0),
        "layers": [layer("yazilar", "Yazılar"), layer("noktalar", "Noktalar"), layer("cizgiler", "Çizgiler"),
                   layer("binalar", "Binalar"), layer("alanlar", "Alanlar")],
        "activeLayer": "noktalar",
        "entities": entities,
        "styles": { "items": [], "categories": [] },
        "homeView": { "minX": O.0, "minY": O.1, "maxX": O.0 + 200.0, "maxY": O.1 + 140.0 },
    });
    let snapshot = DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("the neighbourhood reads");
    Document::new(snapshot, None).expect("the neighbourhood opens")
}
