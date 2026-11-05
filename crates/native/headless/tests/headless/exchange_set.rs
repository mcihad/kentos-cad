//! CAD-ADV-17's acceptance (docs/adr/1061 §6): the 2D file set ezdxf writes (`fixtures/exchange/v1/set.dxf`,
//! `scripts/fixtures/exchange_set_cases.py`) opened, edited, exported and read again. Opening keeps the Z values,
//! the unit blocks, the ellipse and the NURBS, the two sheets, and says what it could not take; importing it with the
//! blocks exploded turns the nonuniform arc into an exact NURBS and keeps the wide edges' strips; 2B düzleme indir is
//! an explicit, previewed, undoable step; an export never writes over the source unasked. The expected values are the
//! file's own numbers and analytic geometry, not KentOS's output. ezdxf reads the export in `--roundtrip`.

use super::*;

fn set() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/exchange/v1/set.dxf")
}

fn call(s: &mut Session, command: &str, op: &str, input: Value) -> Value {
    s.call(json!({ "command": command, "op": op, "input": input }))
        .expect("answers")
}

fn objects(s: &Session) -> Vec<Value> {
    s.document()
        .entities()
        .map(|e| serde_json::to_value(e).expect("json"))
        .collect()
}

fn uid_of(s: &Session, kind: &str, pick: impl Fn(&Value) -> bool) -> String {
    let e = s
        .document()
        .entities()
        .find(|e| {
            let v = serde_json::to_value(e).expect("json");
            v["kind"] == kind && pick(&v)
        })
        .unwrap_or_else(|| panic!("no {kind}"));
    s.document()
        .uid(kentos_domain::Slot(e.base().id))
        .expect("uid")
        .to_string()
}

#[test]
fn the_exchange_set_opens_edits_and_goes_back_with_every_loss_said() {
    let dir = scratch("exchange-set");
    let source = dir.join("set.dxf");
    std::fs::copy(set(), &source).expect("copied");
    let original = std::fs::read(&source).expect("reads");
    let mut s = Session::open(&source).expect("opens");
    let all = objects(&s);
    // Z is kept on opening: nothing sets it to 0 or drops it unasked.
    let line = all
        .iter()
        .find(|e| e["kind"] == "line" && e["za"].is_number())
        .expect("the elevated line");
    assert_eq!(
        (line["za"].as_f64(), line["zb"].as_f64()),
        (Some(3.5), Some(4.25)),
        "{line}"
    );
    let point = all
        .iter()
        .find(|e| e["kind"] == "point")
        .expect("the point");
    assert_eq!(point["z"].as_f64(), Some(12.0), "{point}");
    // Analytic kinds stay analytic.
    assert!(all.iter().any(|e| e["kind"] == "ellipse"), "the ellipse");
    let nurbs = all
        .iter()
        .find(|e| e["kind"] == "nurbs")
        .expect("the rational spline");
    assert_eq!(nurbs["weights"], json!([1.0, 0.5, 1.0]), "{nurbs}");
    // Nested blocks with their own units: the door's 900 mm line is 0.9 m inside the storey.
    let door = s
        .document()
        .blocks()
        .iter()
        .find(|b| b.name == "KAPI")
        .expect("KAPI")
        .clone();
    let kentos_commands::Entity::Line(l) = &door.entities[0] else {
        panic!("the door's line")
    };
    assert!(((l.b.x - l.a.x) - 0.9).abs() < 1e-12, "{l:?}");
    // The two sheets come as sheets.
    let sheets = serde_json::to_value(s.document().to_snapshot_v2()).expect("json");
    let book = &sheets;
    assert!(
        book.to_string().contains("Pafta A") && book.to_string().contains("Pafta B"),
        "{book}"
    );

    // Imported with the blocks exploded into a drawing: the nonuniform arc is an exact NURBS on the mapped ellipse
    // (centre (2, 50), semi-axes 2 and 0.5), the wide straight edges keep their strips (0.2 × 1.5 / |M t|).
    let mut into = new_drawing();

    let imported = call(
        &mut into,
        "file.import.dxf",
        "execute",
        json!({
        "path": source, "explodeBlocks": true, "allowPolylineLoss": true }),
    );
    assert_eq!(imported["status"], "completed", "{imported}");
    let pieces = objects(&into);
    let exact = pieces
        .iter()
        .filter(|e| e["kind"] == "nurbs" && e["degree"] == 2)
        .find(|e| {
            e["weights"]
                .as_array()
                .is_some_and(|w| w.iter().any(|x| x.as_f64().is_some_and(|x| x < 0.999)))
        })
        .unwrap_or_else(|| panic!("no exact NURBS: {imported}"));
    let ctrl: Vec<(f64, f64)> = exact["ctrl"]
        .as_array()
        .expect("ctrl")
        .iter()
        .map(|p| (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap()))
        .collect();
    let w: Vec<f64> = exact["weights"]
        .as_array()
        .expect("w")
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    let origin = (ctrl[0].0, ctrl[0].1 - 0.0);
    for piece in 0..2 {
        for k in 0..=16 {
            let t = f64::from(k) / 16.0;
            let b = [(1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t];
            let (mut x, mut y, mut sw) = (0.0, 0.0, 0.0);
            for i in 0..3 {
                let (cx, cy) = ctrl[piece * 2 + i];
                x += b[i] * w[piece * 2 + i] * cx;
                y += b[i] * w[piece * 2 + i] * cy;
                sw += b[i] * w[piece * 2 + i];
            }
            let (x, y) = (x / sw - origin.0, y / sw - origin.1);
            // The half circle from (0, 0) to (2, 0) about (1, 0), under x×2, y×0.5: centre (2, 0) from its start.
            let r = ((x - 2.0) / 2.0).powi(2) + (y / 0.5).powi(2);
            assert!((r - 1.0).abs() < 1e-9, "piece {piece}, t {t}: {r}");
        }
    }
    let wide: Vec<f64> = pieces
        .iter()
        .filter(|e| e["kind"] == "polyline")
        .filter_map(|e| e["widths"].as_array().cloned())
        .flatten()
        .filter_map(|w| w[0].as_f64())
        .filter(|w| *w > 0.0)
        .collect();
    let has = |v: f64| wide.iter().any(|w| (w - v).abs() < 1e-12);
    assert!(
        has(0.2 * 1.5 / 3.0) && has(0.2 * 1.5 / 0.5),
        "widths {wide:?}"
    );

    // What the open could not take as it was is said: the foreign DIMENSION came as its picture (lines, ticks, text),
    // not as a KentOS dimension; the complex line type's text is not drawn.
    let opened = kentos_native_application::cad_open::read(
        &source,
        &original,
        kentos_native_application::cad_open::Format::Dxf,
        &mut kentos_formats::watch::Quiet,
    )
    .expect("opens");
    let said: Vec<String> = opened
        .report
        .notes
        .iter()
        .chain(&opened.report.skipped)
        .map(|r| format!("{}: {}", r.what, r.reason))
        .collect();
    assert!(
        said.iter()
            .any(|w| w.to_uppercase().contains("DIMENSION") || w.contains("Ölçü")),
        "{said:#?}"
    );
    assert!(
        !all.iter().any(|e| e["kind"] == "dimension"),
        "a foreign dimension is no KentOS dimension here"
    );

    // 2B düzleme indir: previewed, one undo step; opening did not do it.
    let line_uid = uid_of(&s, "line", |v| v["za"].is_number());
    let point_uid = uid_of(&s, "point", |_| true);
    let preview = call(
        &mut s,
        "cad.entities.flatten",
        "plan",
        json!({ "uids": [line_uid, point_uid] }),
    );
    assert_eq!(preview["status"], "completed", "{preview}");
    assert_eq!(
        (
            preview["output"]["vertices"].as_u64(),
            preview["output"]["zMin"].as_f64(),
            preview["output"]["zMax"].as_f64()
        ),
        (Some(3), Some(3.5), Some(12.0)),
        "{preview}"
    );
    let before = objects(&s);
    assert_eq!(before, all, "a preview writes nothing");
    let flat = call(
        &mut s,
        "cad.entities.flatten",
        "execute",
        json!({ "uids": [line_uid, point_uid] }),
    );
    assert_eq!(flat["status"], "completed", "{flat}");
    assert!(
        objects(&s)
            .iter()
            .all(|e| e["za"].is_null() && e["z"].is_null()),
        "flattened"
    );
    assert_eq!(s.undo().as_deref(), Some("2B düzleme indir"));
    assert_eq!(objects(&s), all, "one undo brings the elevations back");

    // An export over the source is refused unasked, and the source is the same; asked, it is written.
    let refused = call(
        &mut s,
        "file.export.dxf",
        "execute",
        json!({ "path": source, "overwrite": true, "allowPolylineLoss": true }),
    );
    assert_eq!(refused["error"]["code"], "source_overwrite", "{refused}");
    assert_eq!(
        std::fs::read(&source).expect("reads"),
        original,
        "the source is untouched"
    );
    let copy = dir.join("kopya.dxf");
    std::fs::copy(&source, &copy).expect("copied");
    let replaced = call(
        &mut s,
        "file.export.dxf",
        "execute",
        json!({
        "path": copy, "overwrite": true, "replaceSource": true, "allowPolylineLoss": true }),
    );
    assert_eq!(replaced["status"], "completed", "{replaced}");
    // KentOS's own export is no foreign source: written over again with overwrite alone.
    let again = call(
        &mut s,
        "file.export.dxf",
        "execute",
        json!({ "path": copy, "overwrite": true, "allowPolylineLoss": true }),
    );
    assert_eq!(again["status"], "completed", "{again}");

    // Read again: every kind the export wrote is a kind KentOS reads back, the elevations kept; what changed on the
    // way is in the export's report.
    let back = Session::open(&copy).expect("reads back");
    let kinds = |v: &[Value]| {
        let mut k: Vec<String> = v
            .iter()
            .map(|e| e["kind"].as_str().unwrap_or("").to_owned())
            .collect();
        k.sort();
        k.dedup();
        k
    };
    let again_objects = objects(&back);
    for k in kinds(&all) {
        assert!(
            kinds(&again_objects).contains(&k),
            "{k} lost on the way back"
        );
    }
    let line = again_objects
        .iter()
        .find(|e| e["kind"] == "line" && e["za"].is_number())
        .expect("z kept");
    assert_eq!(
        (line["za"].as_f64(), line["zb"].as_f64()),
        (Some(3.5), Some(4.25))
    );
    assert!(replaced["output"]["report"].is_object(), "{replaced}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The ownership rule of an export target (docs/adr/1061 §5), on its own.
#[test]
fn a_foreign_cad_drawing_is_told_apart_from_a_kentos_export() {
    let dir = scratch("foreign-cad");
    let foreign = dir.join("a.dxf");
    std::fs::copy(set(), &foreign).expect("copied");
    assert!(kentos_headless::file_commands::foreign_cad(&foreign));
    let dwg = dir.join("b.dwg");
    std::fs::write(&dwg, b"AC1032\0\0\0").expect("written");
    assert!(kentos_headless::file_commands::foreign_cad(&dwg));
    let text = dir.join("c.dxf");
    std::fs::write(&text, b"ORIGINAL TARGET").expect("written");
    assert!(
        !kentos_headless::file_commands::foreign_cad(&text),
        "not a CAD drawing"
    );
    assert!(!kentos_headless::file_commands::foreign_cad(
        &dir.join("yok.dxf")
    ));
    let _ = std::fs::remove_dir_all(&dir);
}
