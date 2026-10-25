//! Labels in the style files end to end (docs/adr/0635): a polygon, a line and a point layer labelled (rules with
//! conditions, a curved single label, a point's label around it) go out by file.export.sld, .qml and .maplibre and
//! come back by the imports into the same layers with the same classes. With KENTOS_WRITE_LABELS=1 the files go to
//! fixtures/style-labels/v1/out for scripts/fixtures/style_labels_cases.py, which opens the SLDs and QMLs in QGIS 4.2.

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

fn keep(path: &std::path::Path, name: &str) {
    if std::env::var_os("KENTOS_WRITE_LABELS").is_some() {
        let out_dir = format!(
            "{}/../../../fixtures/style-labels/v1/out",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::create_dir_all(&out_dir).expect("a folder");
        std::fs::copy(path, format!("{out_dir}/{name}")).expect("kept");
    }
}

/// A single symbol for the layer's geometry.
fn look(name: &str) -> Value {
    match name {
        "parsel" => json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
            { "id": "f", "type": "simpleFill", "color": "#f2e6c9" },
            { "id": "o", "type": "simpleLine", "color": "#8a6d3b", "width": 0.3, "unit": "mm" } ] } } }),
        "yol" => json!({ "type": "single", "symbols": { "line": { "type": "line", "layers": [
            { "id": "l", "type": "simpleLine", "color": "#555555", "width": 0.6, "unit": "mm" } ] } } }),
        _ => json!({ "type": "single", "symbols": { "marker": { "type": "marker", "layers": [
            { "id": "p", "type": "shape", "shape": "circle", "size": 2.0, "unit": "mm", "fill": "#d62728" } ] } } }),
    }
}

/// The labelling each layer gets: (layer name, label, labels).
fn labellings() -> Vec<(&'static str, Value, Value)> {
    vec![
        (
            "parsel",
            Value::Null,
            json!({ "mode": "rules", "classes": [
                { "name": "Ada A", "when": "[ada] = 'A'", "style": {
                    "placement": "center", "size": 12.0, "text": "[ada] || '/' || [parsel_no]", "color": "#aa0000",
                    "weight": 600, "italic": true, "halo": { "width": 2.0, "color": "#ffffff" }, "priority": 8,
                    "minScale": 0.5, "maxScale": 20.0 } },
                { "name": "Diğer", "style": { "placement": "center", "size": 9.0, "text": "[parsel_no]",
                    "color": "#333333" } },
                { "name": "İşlevli", "when": "uzunluk([ada]) > 0", "style": { "placement": "center", "size": 8.0,
                    "text": "büyük([ada]) || '-' || metin([parsel_no])" } } ] }),
        ),
        (
            "yol",
            json!({ "placement": "along", "size": 10.0, "text": "[ad]", "line": "curved", "color": "#1f4e9e",
                "halo": { "width": 1.5, "color": "#ffffffcc" }, "repeat": 120.0 }),
            json!({ "mode": "single" }),
        ),
        (
            "nokta",
            json!({ "placement": "beside", "size": 10.0, "text": "'Kot ' || [kot]", "distance": 3.0,
                "color": "#000000", "priority": 6 }),
            json!({ "mode": "single" }),
        ),
    ]
}

#[test]
fn labels_go_out_in_the_style_files_and_come_back() {
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let dir = scratch("style-labels");
    for (name, label, labels) in labellings() {
        let layer = id_of(&took, name);
        // A symbol the files can draw (QGIS loads no SLD without one).
        let set = s
            .call(json!({ "command": "cad.layers.renderer",
                "input": { "layer": layer, "renderer": look(name) } }))
            .expect("answers");
        assert_eq!(set["status"], "completed", "{set}");
        for (command, ext, import) in [
            ("file.export.sld", "sld", "file.import.sld"),
            ("file.export.qml", "qml", "file.import.qml"),
            ("file.export.maplibre", "json", "file.import.maplibre"),
        ] {
            // Each file from the layer's own labelling (a read back changes it).
            let set = s
                .call(json!({ "command": "cad.layers.labels",
                    "input": { "layer": layer, "label": label, "labels": labels } }))
                .expect("answers");
            assert_eq!(set["status"], "completed", "{set}");
            let path = dir.join(format!("etiket-{name}.{ext}"));
            let out = s
                .call(json!({ "command": command,
                    "input": { "layer": layer, "path": path.display().to_string() } }))
                .expect("answers");
            assert_eq!(out["status"], "completed", "{out}");
            assert!(
                out["output"]["labels"].as_u64().unwrap_or(0) > 0,
                "{command} {name}: {out}"
            );
            keep(&path, &format!("etiket-{name}.{ext}"));
            // Off, then read back: the file's labelling is the layer's again.
            let off = s
                .call(json!({ "command": "cad.layers.labels",
                    "input": { "layer": layer, "label": null, "labels": { "mode": "off" } } }))
                .expect("answers");
            assert_eq!(off["status"], "completed", "{off}");
            let input = json!({ "layer": layer, "path": path.display().to_string() });
            let back = s
                .call(json!({ "command": import, "input": input }))
                .expect("answers");
            assert_eq!(back["status"], "completed", "{import} {name}: {back}");
            let got = if import == "file.import.maplibre" {
                back["output"]["layers"][0].clone()
            } else {
                back["output"].clone()
            };
            let layers = got["labels"].clone();
            match name {
                "parsel" => {
                    assert_eq!(layers["mode"], "rules", "{import}: {got}");
                    let c = &layers["classes"];
                    assert_eq!(c[0]["name"], "Ada A", "{import}: {got}");
                    assert_eq!(c[0]["when"], "[ada] = 'A'", "{import}: {got}");
                    assert_eq!(c[0]["style"]["text"], "[ada] || '/' || [parsel_no]");
                    assert_eq!(c[0]["style"]["color"], "#aa0000");
                    assert_eq!(c[0]["style"]["weight"], 600);
                    assert_eq!(c[0]["style"]["priority"], 8, "{import}: {got}");
                    assert_eq!(c[1]["style"]["text"], "[parsel_no]");
                }
                "yol" => {
                    assert_eq!(layers["mode"], "single", "{import}: {got}");
                    assert_eq!(got["label"]["text"], "[ad]");
                    assert_eq!(got["label"]["placement"], "along", "{import}: {got}");
                }
                _ => {
                    assert_eq!(layers["mode"], "single", "{import}: {got}");
                    assert_eq!(got["label"]["text"], "'Kot ' || [kot]");
                    assert_eq!(got["label"]["point"], "around", "{import}: {got}");
                    assert_eq!(got["label"]["distance"], 3.0, "{import}: {got}");
                }
            }
        }
    }
}

/// Thematic renderers QML can hold (docs/adr/0635 §5) go out and come back: a heat map, clusters, inverted areas.
#[test]
fn thematic_renderers_go_out_as_qml_and_come_back() {
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let marker = json!({ "marker": { "type": "marker", "layers": [
        { "id": "p", "type": "shape", "shape": "circle", "size": 2.0, "unit": "mm", "fill": "#d62728" } ] } });
    let looks = [
        (
            "tema-isi",
            "nokta",
            json!({ "type": "heatmap", "radius": 15.0, "unit": "px", "weight": "[kot]", "max": 100.0,
                "ramp": ["#2b83ba00", "#ffffbf", "#d7191c"], "quality": 3.0 }),
        ),
        (
            "tema-kume",
            "nokta",
            json!({ "type": "cluster", "distance": 25.0, "unit": "px", "count": false,
                "renderer": { "type": "single", "symbols": marker }, "symbol": marker["marker"] }),
        ),
        (
            "tema-ters",
            "parsel",
            json!({ "type": "inverted", "merge": true, "symbols": { "fill": { "type": "fill", "layers": [
                { "id": "f", "type": "simpleFill", "color": "#00000080" } ] } } }),
        ),
    ];
    let dir = scratch("style-thematic");
    for (file, name, renderer) in looks {
        let layer = id_of(&took, name);
        let set = s
            .call(json!({ "command": "cad.layers.renderer",
                "input": { "layer": layer, "renderer": renderer } }))
            .expect("answers");
        assert_eq!(set["status"], "completed", "{set}");
        let path = dir.join(format!("{file}.qml"));
        let out = s
            .call(json!({ "command": "file.export.qml",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        keep(&path, &format!("{file}.qml"));
        let back = s
            .call(json!({ "command": "file.import.qml",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(back["status"], "completed", "{back}");
        assert_eq!(
            back["output"]["renderer"]["type"], renderer["type"],
            "{back}"
        );
        // Read again: the same look, nothing changes (the first read may rename symbol layers' ids).
        let again = s
            .call(json!({ "command": "file.import.qml",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(again["output"]["changed"], false, "{file}: {again}");
    }
}

/// Shape markers other than the circle go out as a sprite's icons and come back as the same markers (docs/adr/0635
/// §9).
#[test]
fn shape_markers_go_out_as_sprite_icons_and_come_back() {
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let layer = id_of(&took, "nokta");
    let renderer = json!({ "type": "single", "symbols": { "marker": { "type": "marker", "layers": [
        { "id": "a", "type": "shape", "shape": "star", "size": 4.0, "unit": "mm", "fill": "#ffcc00",
          "stroke": "#7a5c00", "strokeWidth": 0.3 },
        { "id": "b", "type": "shape", "shape": "cross", "size": 3.0, "unit": "mm", "stroke": "#000000",
          "strokeWidth": 0.4, "rotation": 45.0 } ] } } });
    let set = s
        .call(json!({ "command": "cad.layers.renderer",
            "input": { "layer": layer, "renderer": renderer } }))
        .expect("answers");
    assert_eq!(set["status"], "completed", "{set}");
    let dir = scratch("style-sprite");
    let path = dir.join("tema-sprite.json");
    let out = s
        .call(json!({ "command": "file.export.maplibre",
            "input": { "layer": layer, "path": path.display().to_string() } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    let sprite = out["output"]["sprite"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(sprite.len(), 4, "{out}");
    for f in &sprite {
        let p = std::path::PathBuf::from(f.as_str().unwrap_or_default());
        assert!(p.exists(), "{}", p.display());
        keep(&p, &p.file_name().unwrap_or_default().to_string_lossy());
    }
    keep(&path, "tema-sprite.json");
    let style: Value = serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
    assert_eq!(style["sprite"], "tema-sprite-sprite");
    let back = s
        .call(json!({ "command": "file.import.maplibre",
            "input": { "layer": layer, "path": path.display().to_string() } }))
        .expect("answers");
    assert_eq!(back["status"], "completed", "{back}");
    let layers = &back["output"]["layers"][0]["renderer"]["symbols"]["marker"]["layers"];
    assert_eq!(layers[0]["shape"], "star", "{back}");
    assert_eq!(layers[0]["fill"], "#ffcc00");
    assert_eq!(layers[1]["shape"], "cross");
    assert_eq!(layers[1]["rotation"], 45.0);
}
