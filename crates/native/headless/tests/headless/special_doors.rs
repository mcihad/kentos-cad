//! Revolving, pocket and folding doors and the sliding door's track out to DXF (docs/adr/0777 §7): each door's plan
//! symbol as plain parts (a revolving door's drum a circle, its walls arcs, its wings lines; a pocket door's leaf a
//! closed polyline; a folding door's panels lines; the track a line), the wall drawn with the pocket's cavity, the door
//! read back as itself from its KENTOS data, nothing lost.
use kentos_commands::{DxfReadOptions, Entity, OpeningOperation};
use kentos_headless::Session;
use serde_json::json;

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/special-doors.json");

fn session() -> Session {
    let model: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    Session::from_bytes(&kentos_kcad::encode_verified(&model).unwrap()).unwrap()
}

fn read(path: &std::path::Path) -> Vec<Entity> {
    kentos_formats::dxf::read(&std::fs::read(path).unwrap(), &DxfReadOptions::default())
        .unwrap()
        .entities
}

#[test]
fn special_doors_go_to_dxf_as_their_plan_symbols_and_come_back() {
    let mut session = session();
    let folder = super::scratch("special-doors-dxf");
    let path = folder.join("ozel-kapilar.dxf");
    let written = session
        .call(json!({"command":"file.export.dxf","input":{"path":path}}))
        .unwrap();
    assert_eq!(written["status"], "completed", "{written}");
    // Nothing of the doors is lost: the only note on them says they went as their block view, read back whole.
    let notes = written["output"]["report"]["notes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let doors: Vec<_> = notes.iter().filter(|n| n["what"] == "Açıklık").collect();
    assert_eq!(doors.len(), 1, "{notes:?}");
    assert_eq!(doors[0]["count"], 4);
    assert!(
        doors[0]["reason"]
            .as_str()
            .unwrap()
            .contains("KentOS verisinden geri okur"),
        "{doors:?}"
    );
    // The symbols' parts other programs see, in the doors' blocks: the drum a circle 1 m about (2, 0), its two walls
    // arcs 1,03 m, the panels and wings lines.
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let pairs: Vec<(&str, &str)> = lines
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| (c[0], c[1]))
        .collect();
    let entity = |name: &str| -> Vec<Vec<(&str, &str)>> {
        let mut out = Vec::new();
        for (i, p) in pairs.iter().enumerate() {
            if *p == ("0", name) {
                out.push(
                    pairs[i + 1..]
                        .iter()
                        .take_while(|q| q.0 != "0")
                        .copied()
                        .collect(),
                );
            }
        }
        out
    };
    let value = |e: &[(&str, &str)], code: &str| -> f64 {
        e.iter()
            .find(|p| p.0 == code)
            .map(|p| p.1.parse().unwrap())
            .unwrap()
    };
    let circles = entity("CIRCLE");
    assert_eq!(circles.len(), 1, "the revolving door's drum");
    assert!((value(&circles[0], "40") - 1.0).abs() < 1e-9);
    assert!((value(&circles[0], "10") - 2.0).abs() < 1e-9 && value(&circles[0], "20").abs() < 1e-9);
    let walls = entity("ARC")
        .iter()
        .filter(|a| (value(a, "40") - 1.03).abs() < 1e-9)
        .count();
    assert_eq!(walls, 2, "the drum's two walls");
    // Read back whole: the doors themselves, their operations and constructions.
    let back = read(&path);
    let ops: Vec<OpeningOperation> = back
        .iter()
        .filter_map(|e| {
            if let Entity::Opening(o) = e {
                Some(o.opening.operation)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        ops,
        vec![
            OpeningOperation::Revolving,
            OpeningOperation::Pocket,
            OpeningOperation::Folding,
            OpeningOperation::Slide
        ]
    );
    let pocket = back
        .iter()
        .find_map(|e| match e {
            Entity::Opening(o) if o.opening.operation == OpeningOperation::Pocket => Some(o),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        pocket.opening.build.as_ref().and_then(|b| b.lip),
        Some(0.06)
    );
    let _ = std::fs::remove_dir_all(folder);
}

/// Pixels per millimetre of the crops (600 dpi).
const PX: f64 = 600.0 / 25.4;
/// The map's centre on the page (A3 landscape, 20 mm left and 10 mm top margins) and in the drawing, mm and m.
const PAGE: [f64; 2] = [20.0 + 390.0 / 2.0, 10.0 + 277.0 / 2.0];
const AT: [f64; 2] = [6.0, 0.0];

/// Where a point of the drawing is on the 1:50 page, mm.
fn on_page(x: f64, y: f64) -> [f64; 2] {
    [PAGE[0] + (x - AT[0]) * 20.0, PAGE[1] - (y - AT[1]) * 20.0]
}

/// The page's grey pixels on `[x, y, w, h]` mm, as poppler's `pdftoppm` draws them: width, height, rows.
fn crop(pdf: &std::path::Path, r: [f64; 4]) -> (usize, usize, Vec<u8>) {
    let [x, y, w, h] = r.map(|v| (v * PX).round() as usize);
    let prefix = pdf.with_extension(format!("{x}-{y}"));
    let ran = std::process::Command::new("pdftoppm")
        .args(["-r", "600", "-gray", "-singlefile", "-f", "1", "-l", "1"])
        .args([
            "-x",
            &x.to_string(),
            "-y",
            &y.to_string(),
            "-W",
            &w.to_string(),
            "-H",
            &h.to_string(),
        ])
        .arg(pdf)
        .arg(&prefix)
        .status()
        .expect("pdftoppm runs");
    assert!(ran.success());
    let bytes = std::fs::read(format!("{}.pgm", prefix.display())).expect("the crop");
    // "P5", width, height, maximum, each after white space; the pixels after the last one's single white space.
    let mut fields = Vec::new();
    let mut i = 0;
    while fields.len() < 4 {
        while bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        fields.push(String::from_utf8_lossy(&bytes[start..i]).into_owned());
    }
    let (cw, ch): (usize, usize) = (fields[1].parse().unwrap(), fields[2].parse().unwrap());
    (cw, ch, bytes[i + 1..i + 1 + cw * ch].to_vec())
}

/// The runs of dark pixels down column `col` of a crop.
fn runs_down(w: usize, h: usize, px: &[u8], col: usize) -> usize {
    let mut runs = 0;
    let mut inside = false;
    for row in 0..h {
        let dark = px[row * w + col] < 160;
        if dark && !inside {
            runs += 1;
        }
        inside = dark;
    }
    runs
}

#[test]
fn special_doors_print_at_1_50_with_the_drum_and_the_pocket() {
    let mut session = session();
    let sheet = json!({ "op": "addSheet", "sheet": {
        "id": "ozel", "name": "Özel kapılar 1:50",
        "page": { "paper": "a3", "orientation": "landscape", "size": { "width": 420000, "height": 297000 },
            "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } },
        "items": [{ "id": "ozel-harita", "name": "Harita",
            "frame": { "left": 20000, "top": 10000, "width": 390000, "height": 277000 },
            "kind": { "type": "map", "view": { "type": "fixed", "center": { "x": AT[0], "y": AT[1] },
                "scale": 50, "rotation": 0 } } }]
    }});
    let done = session
        .call(json!({"command":"cad.sheets.edit","input":{"ops":[sheet]}}))
        .unwrap();
    assert_eq!(done["status"], "completed", "{done}");
    let folder = super::scratch("special-doors-pdf");
    let path = folder.join("ozel-kapilar.pdf");
    let pdf = kentos_plot::book::book_pdf(session.document(), &[], "Özel", "2026-10-31").unwrap();
    std::fs::write(&path, pdf.bytes).unwrap();
    // The drum: a circle 20 mm on paper about the revolving door's middle; where nothing else is (its wings at
    // 45°, its walls about the wall's line) ink stands on it all round.
    let c = on_page(2.0, 0.0);
    let (w, h, px) = crop(&path, [c[0] - 25.0, c[1] - 25.0, 50.0, 50.0]);
    for deg in [60.0_f64, 75.0, 90.0, 105.0, 120.0, 240.0, 270.0, 300.0] {
        let a = deg.to_radians();
        let near = (0..=12).any(|k| {
            let r = (19.5 + 0.08 * f64::from(k)) * PX;
            let (x, y) = (25.0 * PX + r * a.cos(), 25.0 * PX - r * a.sin());
            px[(y.round() as usize).min(h - 1) * w + (x.round() as usize).min(w - 1)] < 160
        });
        assert!(near, "the drum at {deg}°");
    }
    // The pocket: across the wall beside the pocket door's start jamb, its two faces and the cavity's two sides; past
    // the pocket only the faces.
    let at = on_page(4.4, 0.0);
    let (w, h, px) = crop(&path, [at[0], at[1] - 5.0, 20.0, 10.0]);
    let col = |x: f64| ((on_page(x, 0.0)[0] - at[0]) * PX).round() as usize;
    assert_eq!(
        runs_down(w, h, &px, col(5.17)),
        4,
        "faces and the pocket's sides"
    );
    assert_eq!(runs_down(w, h, &px, col(4.55)), 2, "the faces only");
    let _ = std::fs::remove_dir_all(folder);
}
