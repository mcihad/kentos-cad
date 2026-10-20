//! `file.export.maplibre` and `file.import.maplibre` end to end (docs/adr/0560): categorized, graduated, rule-based
//! and single looks written as MapLibre GL styles with their GeoJSON beside them; what a style cannot say is said;
//! each read back by file.import.maplibre gives the same look. With KENTOS_WRITE_MAPLIBRE=1 the styles go to
//! fixtures/maplibre/v1/out for scripts/fixtures/maplibre_cases.py, which runs MapLibre's own validator and expression
//! evaluator over them.

use serde_json::{Value, json};

use super::{new_drawing, scratch};

fn id_of(took: &Value, name: &str) -> String {
    took["output"]["layers"]
        .as_array()
        .and_then(|ls| ls.iter().find(|l| l["name"] == name))
        .and_then(|l| l["id"].as_str())
        .unwrap_or_else(|| panic!("{name}"))
        .to_owned()
}

fn fill(c: &str) -> Value {
    json!({ "fill": { "type": "fill", "layers": [
        { "id": "f", "type": "simpleFill", "color": c },
        { "id": "o", "type": "simpleLine", "color": "#222222", "width": 0.4, "unit": "mm", "cap": "butt", "join": "miter" } ] } })
}

fn keep(path: &std::path::Path, name: &str) {
    if std::env::var_os("KENTOS_WRITE_MAPLIBRE").is_some() {
        let out_dir = format!(
            "{}/../../../fixtures/maplibre/v1/out",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::create_dir_all(&out_dir).expect("a folder");
        std::fs::copy(path, format!("{out_dir}/{name}")).expect("kept");
    }
}

#[test]
fn layer_looks_go_out_as_maplibre_styles_and_come_back() {
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let (parsel, yol, nokta) = (
        id_of(&took, "parsel"),
        id_of(&took, "yol"),
        id_of(&took, "nokta"),
    );
    let looks = [
        (
            "kentos-kategorili",
            parsel.clone(),
            json!({ "type": "categorized", "expr": "[ada]", "categories": [
                { "value": "A", "label": "Ada A", "symbols": fill("#e41a1c") },
                { "value": "B", "label": "Ada B", "symbols": fill("#377eb8") } ],
                "other": fill("#99999980") }),
            "categorized",
        ),
        (
            "kentos-aralikli",
            parsel.clone(),
            json!({ "type": "graduated", "expr": "[alan]", "classes": [
                { "min": 0.0, "max": 500.0, "label": "küçük", "symbols": fill("#fee5d9") },
                { "min": 500.0, "max": 1000.0, "label": "orta", "symbols": fill("#fcae91") },
                { "min": 1000.0, "max": 5000.0, "label": "büyük", "symbols": fill("#de2d26") } ] }),
            "graduated",
        ),
        (
            "kentos-kurallar",
            parsel.clone(),
            json!({ "type": "rules", "rules": [
                { "id": "r1", "label": "Tapulu", "filter": "[tapulu] = 1", "symbols": fill("#1b9e77"),
                  "children": [ { "id": "r2", "label": "Büyük", "filter": "[alan] > 1000 ve [ada] <> 'B'",
                    "minScale": 500, "maxScale": 10000, "symbols": fill("#d95f02") } ] },
                { "id": "r3", "label": "Diğer", "isElse": true, "symbols": fill("#7570b3") } ] }),
            "rules",
        ),
        (
            "kentos-cizgi",
            yol.clone(),
            json!({ "type": "single", "symbols": { "line": { "type": "line", "opacity": 0.8, "layers": [
                { "id": "l", "type": "simpleLine", "color": "#3366cc", "width": 0.8, "unit": "mm",
                  "dash": [3.0, 1.5], "cap": "round", "join": "round" },
                { "id": "m", "type": "simpleLine", "color": "#ffffff", "width": 2.0, "unit": "m" } ] } } }),
            "single",
        ),
        (
            "kentos-nokta",
            nokta.clone(),
            json!({ "type": "single", "symbols": { "marker": { "type": "marker", "layers": [
                { "id": "p", "type": "shape", "shape": "circle", "size": 3.0, "unit": "mm", "fill": "#ff0000",
                  "stroke": "#000000", "strokeWidth": 0.3 },
                { "id": "g", "type": "shape", "shape": "triangle", "size": 2.0, "unit": "mm", "fill": "#00ff00" } ] } } }),
            "single",
        ),
    ];
    let dir = scratch("maplibre");
    for (name, layer, renderer, kind) in &looks {
        let set = s
            .call(json!({ "command": "cad.layers.renderer", "input": { "layer": layer, "renderer": renderer } }))
            .expect("answers");
        assert_eq!(set["status"], "completed", "{set}");
        let path = dir.join(format!("{name}.json"));
        let data = dir.join(format!("{name}.geojson"));
        let planned = s
            .call(json!({ "command": "file.export.maplibre", "op": "plan",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{planned}");
        assert!(!path.exists() && !data.exists(), "plan writes nothing");
        let out = s
            .call(json!({ "command": "file.export.maplibre",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        assert_eq!(out["output"]["bytes"], planned["output"]["bytes"]);
        assert_eq!(out["output"]["layers"], json!([layer]));
        let style: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("written")).expect("json");
        assert_eq!(style["version"], 8);
        assert_eq!(style["sources"][layer]["type"], "geojson");
        assert_eq!(
            style["sources"][layer]["data"],
            format!("{name}.geojson"),
            "the data is beside the style"
        );
        // The GeoJSON: WGS 84 longitude and latitude of the layer's objects.
        let geo: Value =
            serde_json::from_str(&std::fs::read_to_string(&data).expect("written")).expect("json");
        let features = geo["features"].as_array().expect("features");
        assert_eq!(
            out["output"]["data"][0]["features"],
            json!(features.len()),
            "{out}"
        );
        let lon_lat = |v: &Value| -> bool {
            v.as_array().is_some_and(|a| {
                a.len() >= 2
                    && a[0].as_f64().is_some_and(|x| (20.0..46.0).contains(&x))
                    && a[1].as_f64().is_some_and(|y| (34.0..43.0).contains(&y))
            })
        };
        fn first(v: &Value) -> &Value {
            match v.as_array().and_then(|a| a.first()) {
                Some(f) if f.is_array() => first(f),
                _ => v,
            }
        }
        let g = &features[0]["geometry"]["coordinates"];
        let mut c = g;
        while c
            .as_array()
            .is_some_and(|a| a.first().is_some_and(Value::is_array))
        {
            c = &c[0];
        }
        assert!(lon_lat(c) || lon_lat(first(g)), "{g}");
        let lost = out["output"]["lost"].to_string();
        match *name {
            "kentos-nokta" => assert!(lost.contains("“triangle” şekli"), "{out}"),
            "kentos-cizgi" => assert!(lost.contains("metre cinsinden"), "{out}"),
            "kentos-kurallar" => assert!(lost.contains("ölçek aralıkları"), "{out}"),
            _ => assert!(out["output"].get("lost").is_none(), "{name}: {out}"),
        }
        // Read back into the same layer.
        let back = s
            .call(json!({ "command": "file.import.maplibre",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(back["status"], "completed", "{back}");
        let l = &back["output"]["layers"][0];
        assert_eq!(l["kind"], *kind, "{back}");
        let r = &l["renderer"];
        match *name {
            "kentos-kategorili" => {
                assert_eq!(r["categories"][1]["value"], "B");
                assert_eq!(r["categories"][1]["label"], "Ada B");
                assert_eq!(r["other"]["fill"]["layers"][0]["color"], "#99999980");
                // 0.4 mm is 1.5118 px: sizes come back in the pixels they were written in.
                assert_eq!(
                    r["categories"][0]["symbols"]["fill"]["layers"][1]["unit"],
                    "px"
                );
            }
            "kentos-aralikli" => {
                assert_eq!(r["classes"][2]["min"], 1000.0);
                assert_eq!(r["classes"][2]["max"], 5000.0);
                assert_eq!(r["classes"][1]["label"], "orta");
            }
            "kentos-kurallar" => {
                // The nested rule is the parent's filter and its own; the else is none of its elders.
                assert_eq!(r["rules"][0]["filter"], "[tapulu] = 1");
                assert_eq!(
                    r["rules"][1]["filter"],
                    "[tapulu] = 1 ve [alan] > 1000 ve [ada] <> 'B'"
                );
                assert!(
                    r["rules"][1]["maxScale"]
                        .as_f64()
                        .is_some_and(|m| (m - 10000.0).abs() < 2.0)
                );
                assert!(
                    r["rules"][2]["filter"]
                        .as_str()
                        .is_some_and(|f| f.starts_with("değil ("))
                );
            }
            "kentos-cizgi" => {
                let line = &r["symbols"]["line"];
                assert_eq!(line["opacity"], 0.8);
                assert_eq!(line["layers"][0]["unit"], "px");
                assert_eq!(line["layers"][0]["cap"], "round");
                assert_eq!(line["layers"][1]["unit"], "m");
                assert!((line["layers"][1]["width"].as_f64().expect("w") - 2.0).abs() < 1e-3);
            }
            "kentos-nokta" => {
                let m = &r["symbols"]["marker"]["layers"][0];
                assert_eq!(m["shape"], "circle");
                assert_eq!(m["fill"], "#ff0000");
            }
            _ => {}
        }
        keep(&path, &format!("{name}.json"));
        // Written over only when asked.
        let again = s
            .call(json!({ "command": "file.export.maplibre",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(again["error"]["code"], "file_exists", "{again}");
    }
    // The visible layers as one style: the bottom one first, a GeoJSON file each.
    let all = dir.join("kentos-cok.json");
    let out = s
        .call(json!({ "command": "file.export.maplibre",
            "input": { "path": all.display().to_string() } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    let ids: Vec<String> = out["output"]["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .map(|l| l.as_str().expect("id").to_owned())
        .collect();
    assert!(ids.len() >= 3, "{out}");
    let pos = |id: &str| ids.iter().position(|x| x == id).expect(id);
    assert!(
        pos(&nokta) < pos(&yol) && pos(&yol) < pos(&parsel),
        "the layer tree lists the top layer first: {ids:?}"
    );
    assert_eq!(
        out["output"]["data"].as_array().map(Vec::len),
        Some(ids.len())
    );
    let style: Value =
        serde_json::from_str(&std::fs::read_to_string(&all).expect("written")).expect("json");
    assert_eq!(
        style["sources"].as_object().map(|m| m.len()),
        Some(ids.len())
    );
    keep(&all, "kentos-cok.json");
    // Back, into every layer the style is for (its metadata names them): one undo step.
    let back = s
        .call(json!({ "command": "file.import.maplibre", "input": { "path": all.display().to_string() } }))
        .expect("answers");
    assert_eq!(back["status"], "completed", "{back}");
    assert!(
        back["output"]["layers"]
            .as_array()
            .is_some_and(|l| l.len() >= 3)
    );
    // A source URL instead of the GeoJSON: vector tiles of each layer's name.
    let tiles = dir.join("karo.json");
    let out = s
        .call(json!({ "command": "file.export.maplibre",
            "input": { "path": tiles.display().to_string(), "layer": yol,
                "sourceUrl": "https://karo.example.org/{z}/{x}/{y}.pbf" } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    assert!(out["output"].get("data").is_none(), "{out}");
    let style: Value =
        serde_json::from_str(&std::fs::read_to_string(&tiles).expect("written")).expect("json");
    assert_eq!(style["sources"]["kentos"]["type"], "vector");
    assert_eq!(
        style["sources"]["kentos"]["tiles"][0],
        "https://karo.example.org/{z}/{x}/{y}.pbf"
    );
    assert_eq!(style["layers"][0]["source-layer"], "yol");
    assert!(!dir.join("karo.geojson").exists());
    // One GeoJSON address cannot hold several layers.
    let bad = s
        .call(json!({ "command": "file.export.maplibre",
            "input": { "path": dir.join("kotu.json").display().to_string(),
                "sourceUrl": "https://example.org/veri.geojson" } }))
        .expect("answers");
    assert_eq!(bad["error"]["code"], "invalid_input", "{bad}");
    let missing = s
        .call(json!({ "command": "file.export.maplibre",
            "input": { "layer": "yok", "path": dir.join("x.json").display().to_string() } }))
        .expect("answers");
    assert_eq!(missing["error"]["code"], "layer_not_found", "{missing}");
    let _ = std::fs::remove_dir_all(&dir);
}
