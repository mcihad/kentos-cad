//! The pictures of docs/adr/1001 (long line type patterns): a DXF whose LTYPE has 16 elements opened whole and drawn
//! with its whole pattern beside the 12-element one, Çizgi tipleri showing it, and the import's note for a 300-element
//! pattern cut to 256. `tools_screens` takes them:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=uzun-desen-cizim,uzun-desen-pencere cargo test -p kentos-desktop tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::{App, Message};
use crate::line_types_window::Event;
use crate::tools_screens::Scene;

fn ltype(name: &str, lengths: &[f64]) -> String {
    let total: f64 = lengths.iter().map(|l| l.abs()).sum();
    let mut out = format!("0\nLTYPE\n2\n{name}\n70\n0\n3\n{} öğe\n72\n65\n73\n{}\n40\n{total}\n", lengths.len(), lengths.len());
    for l in lengths {
        out.push_str(&format!("49\n{l}\n74\n0\n"));
    }
    out
}

fn layer(name: &str, ltype: &str, color: u8) -> String {
    format!("0\nLAYER\n2\n{name}\n70\n0\n62\n{color}\n6\n{ltype}\n")
}

fn line(layer: &str, y: f64) -> String {
    format!("0\nLINE\n8\n{layer}\n10\n0.0\n20\n{y}\n30\n0.0\n11\n60.0\n21\n{y}\n31\n0.0\n")
}

/// Sixteen elements: four dashes growing 1, 2, 3, 4 m each with a 0,5 m gap, twice; the same as twelve cut at 12.
fn drawing_file() -> std::path::PathBuf {
    let sixteen: Vec<f64> = (0..8).flat_map(|i| [f64::from(i % 4 + 1), -0.5]).collect();
    let twelve: Vec<f64> = sixteen[..12].to_vec();
    let many: Vec<f64> = (0..150).flat_map(|_| [1.0, -0.5]).collect();
    let text = format!(
        "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLTYPE\n{}{}{}0\nENDTAB\n0\nTABLE\n2\nLAYER\n{}{}{}0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{}{}{}0\nENDSEC\n0\nEOF\n",
        ltype("UZUN16", &sixteen),
        ltype("KESIK12", &twelve),
        ltype("UZUN300", &many),
        layer("UZUN", "UZUN16", 1),
        layer("KISA", "KESIK12", 5),
        layer("COK", "UZUN300", 3),
        line("UZUN", 10.0),
        line("KISA", 6.0),
        line("COK", 2.0),
    );
    let path = std::env::temp_dir().join("kentos-uzun-desen").join("uzun-desen.dxf");
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder");
    std::fs::write(&path, text).expect("the file");
    path
}

fn opened(app: &mut App) {
    // The app's own open: the reader's report (the cut 300-element pattern) goes to Komut geçmişi.
    let task = app.update(Message::Opening(crate::opening::Event::Picked(Some(drawing_file()))));
    crate::files_testing::drive(app, task);
    crate::files_testing::make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: -2.0, min_y: 0.0, max_x: 62.0, max_y: 12.0 },
        24.0,
    );
}

/// The same three types put straight into a drawing's settings (no DXF open): which path draws them.
fn edited(app: &mut App) {
    let bytes = std::fs::read(drawing_file()).expect("the file");
    let read = kentos_formats::dxf::read(
        &bytes,
        &kentos_commands::DxfReadOptions { unit: Some(kentos_commands::DrawingUnit::M), ..Default::default() },
    )
    .expect("reads");
    let mut o = crate::tools_scenes::Objects::new();
    o.line("uzun", [0.0, 10.0], [60.0, 10.0]);
    o.line("kisa", [0.0, 6.0], [60.0, 6.0]);
    o.line("cok", [0.0, 2.0], [60.0, 2.0]);
    crate::tools_scenes::open_edited(app, o, |d| {
        let layer = |id: &str, name: &str, color: &str, pattern: &str| {
            let mut l = crate::tools_scenes::layer(id, name, color, 0.35);
            l["style"]["lineType"] = serde_json::json!("dashed");
            l["style"]["linePattern"] = serde_json::json!(pattern);
            l
        };
        d["layers"] = serde_json::json!([
            layer("uzun", "UZUN", "#E5484D", "UZUN16"),
            layer("kisa", "KISA", "#3E63DD", "KESIK12"),
            layer("cok", "COK", "#30A46C", "UZUN300"),
        ]);
        d["activeLayer"] = serde_json::json!("uzun");
        d["settings"]["lineTypes"] = serde_json::to_value(&read.line_types).expect("the types");
    });
    crate::files_testing::make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: crate::tools_scenes::E - 2.0,
            min_y: crate::tools_scenes::N,
            max_x: crate::tools_scenes::E + 62.0,
            max_y: crate::tools_scenes::N + 12.0,
        },
        24.0,
    );
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("uzun-desen-cizim", opened),
        ("uzun-desen-ayar", edited),
        ("uzun-desen-pencere", |app| {
            opened(app);
            let _ = app.update(Message::Run("lineTypes.manage"));
            let _ = app.update(Message::LineTypes(Event::Choose("UZUN16".into())));
        }),
    ]
}
