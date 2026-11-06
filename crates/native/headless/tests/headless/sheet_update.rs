//! MAP-18's update publication (docs/adr/1320): the shared cases of `fixtures/commands/v1/file.publish.sheets.update.json`
//! (scripts/fixtures/sheet_update_cases.py computes stale sheets, page sources and object digests from boxes and JSON,
//! never from KentOS), and the 1:500/1:1000/1:5000 acceptance read back by independent readers: the plot tests' PDF
//! reader for paper lengths and, when installed, GDAL (`gdalinfo`) for the pages' ground coordinates and Poppler
//! (`pdftotext`) for the printed revision and date.
use kentos_commands::{CallEnvelope, CallOp, CommandResult, encoding::sha256_hex};
use kentos_domain::{Document, Slot};
use kentos_sheet::{SheetBook, project::project_sheets};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[path = "../../../plot/tests/support/files.rs"]
mod files;

const CASES: &str =
    include_str!("../../../../../fixtures/commands/v1/file.publish.sheets.update.json");
/// The earlier delivery's day, written into its manifest so a carried page's date is told from today's.
const EARLIER: &str = "2026-10-01";

fn fixture() -> Value {
    serde_json::from_str(CASES).unwrap()
}

fn model(fx: &Value) -> Document {
    let mut doc =
        Document::from_snapshot(serde_json::from_value(fx["setup"].clone()).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    doc
}

fn env(command: &str, op: CallOp, input: Value) -> CallEnvelope {
    CallEnvelope {
        command: command.into(),
        version: Some(1),
        op,
        request_id: None,
        expected: None,
        relax_constraints: false,
        input,
    }
}

/// `$uidOf:N` → slot N's persistent id, everywhere in `v`.
fn with_uids(doc: &Document, v: &Value) -> Value {
    match v {
        Value::String(s) if s.starts_with("$uidOf:") => {
            json!(doc.uid(Slot(s[7..].parse().unwrap())).unwrap().to_string())
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| with_uids(doc, x)).collect()),
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, x)| (k.clone(), with_uids(doc, x)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// A persistent id back as the fixture names it: `$uidOf:N`, or `$new` for an object the case made.
fn named(slots: &BTreeMap<String, u32>, made: &[String], u: &Value) -> Value {
    let u = u.as_str().unwrap();
    if made.iter().any(|m| m == u) {
        return json!("$new");
    }
    json!(
        slots
            .get(u)
            .map_or_else(|| format!("$unknown:{u}"), |s| format!("$uidOf:{s}"))
    )
}

fn file_call(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Value {
    serde_json::to_value(kentos_headless::file_commands::call(doc, env(command, op, input)).result)
        .unwrap()
}

/// Every file of a folder by name with its SHA-256.
fn listing(dir: &Path) -> BTreeMap<String, String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                sha256_hex(&std::fs::read(e.path()).unwrap()),
            )
        })
        .collect()
}

fn temp(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kentos-sheet-update-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn manifest(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("yayin.json")).unwrap()).unwrap()
}

/// The file of `sheet` in a manifest.
fn file_of<'a>(m: &'a Value, sheet: &str) -> &'a Value {
    m["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["sheets"] == json!([sheet]))
        .unwrap()
}

fn page_of<'a>(m: &'a Value, sheet: &str) -> &'a Value {
    m["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["sheet"] == sheet)
        .unwrap()
}

#[test]
fn independent_update_cases_carry_unchanged_pages_and_make_the_changed_again() {
    let fx = fixture();
    let cases = fx["cases"].as_array().unwrap();
    assert!(cases.len() >= 20);
    let today = kentos_plot::book::today();
    let mut problems = Vec::new();
    for (n, case) in cases.iter().enumerate() {
        let name = case["name"].as_str().unwrap();
        let dir = temp(&n.to_string());
        let (first, second, empty) = (dir.join("first"), dir.join("second"), dir.join("empty"));
        std::fs::create_dir(&empty).unwrap();
        let mut doc = model(&fx);
        let mut input = json!({"path": first, "mode": "separate"});
        for (k, v) in case["first"].as_object().unwrap() {
            input[k] = v.clone();
        }
        let p = file_call(&mut doc, "file.publish.sheets", CallOp::Execute, input);
        assert_eq!(p["status"], "completed", "{name}: {p}");
        // The page sources and object digests are the independent ones.
        let mut m = manifest(&first);
        let sources = &m["sources"];
        assert_eq!(sources["digest"], "kentos.object-digest/1");
        for page in sources["pages"].as_array().unwrap() {
            let sheet = page["sheet"].as_str().unwrap();
            assert_eq!(
                with_uids(&doc, &fx["sources"][sheet]),
                page["objects"],
                "{name}: {sheet}'s sources"
            );
        }
        for (slot, digest) in fx["digests"].as_object().unwrap() {
            let uid = doc.uid(Slot(slot.parse().unwrap())).unwrap().to_string();
            assert_eq!(
                &sources["objects"][&uid], digest,
                "{name}: object {slot}'s digest"
            );
        }
        // An earlier day's delivery.
        m["date"] = json!(EARLIER);
        for page in m["pages"].as_array_mut().unwrap() {
            page["date"] = json!(EARLIER);
        }
        std::fs::write(
            first.join("yayin.json"),
            serde_json::to_vec_pretty(&m).unwrap(),
        )
        .unwrap();
        for (sheet, how) in case["tamper"]["files"].as_object().into_iter().flatten() {
            let at = first.join(file_of(&m, sheet.as_str())["name"].as_str().unwrap());
            if how == "remove" {
                std::fs::remove_file(at).unwrap();
            } else {
                let mut bytes = std::fs::read(&at).unwrap();
                bytes.extend_from_slice(b"\n% eklendi\n");
                std::fs::write(at, bytes).unwrap();
            }
        }
        let issued = listing(&first);
        let mut made: Vec<String> = Vec::new();
        // The slots as published: a deleted object is still named by the slot it had.
        let slots: BTreeMap<String, u32> = doc
            .entities_with_uids()
            .map(|(u, e)| (u.to_string(), e.base().id))
            .collect();
        for step in case["edits"].as_array().unwrap() {
            let before: Vec<String> = doc
                .entities_with_uids()
                .map(|(u, _)| u.to_string())
                .collect();
            let input = with_uids(&doc, &step["input"]);
            let answer = kentos_native_application::call::call(
                &mut doc,
                None,
                env(step["command"].as_str().unwrap(), CallOp::Execute, input),
            );
            assert!(
                matches!(answer.result, CommandResult::Completed { .. }),
                "{name}: {:?}",
                answer.result
            );
            made.extend(
                doc.entities_with_uids()
                    .map(|(u, _)| u.to_string())
                    .filter(|u| !before.contains(u)),
            );
        }
        let mut current_digest = BTreeMap::new();
        if let Some(want) = case.get("status") {
            let st = file_call(
                &mut doc,
                "file.publish.status",
                CallOp::Plan,
                json!({"path": first}),
            );
            let o = &st["output"];
            if o["stale"] != want["stale"] {
                problems.push(format!("{name}: stale {} ≠ {}", o["stale"], want["stale"]));
            }
            for row in o["sheets"].as_array().unwrap() {
                let sheet = row["sheet"].as_str().unwrap();
                let w = &want["sheets"][sheet];
                current_digest.insert(sheet.to_owned(), row["digest"].clone());
                if row["state"] != w["state"] {
                    problems.push(format!(
                        "{name}: {sheet} is {} not {}",
                        row["state"], w["state"]
                    ));
                }
                assert_eq!(row["date"], EARLIER, "{name}: {sheet}");
                let c = &row["changes"];
                let mut got = c.clone();
                for k in ["changed", "added", "removed"] {
                    got[k] = Value::Array(
                        c[k].as_array()
                            .unwrap()
                            .iter()
                            .map(|u| named(&slots, &made, u))
                            .collect(),
                    );
                }
                if got != w["changes"] {
                    problems.push(format!(
                        "{name}: {sheet}'s changes {got} ≠ {}",
                        w["changes"]
                    ));
                }
            }
        }
        let u = &case["update"];
        let mut input = json!({"path": second, "mode": "separate", "previous": first});
        for (k, v) in u["input"].as_object().unwrap() {
            input[k] = match v.as_str() {
                Some("$first") => json!(first),
                Some("$empty") => json!(empty),
                _ => v.clone(),
            };
        }
        let op = if u["op"] == "plan" {
            CallOp::Plan
        } else {
            CallOp::Execute
        };
        let got = file_call(&mut doc, "file.publish.sheets", op, input);
        assert_eq!(got["status"], u["result"]["status"], "{name}: {got}");
        assert_eq!(
            listing(&first),
            issued,
            "{name}: the earlier delivery is left as it was"
        );
        if got["status"] != "completed" {
            assert_eq!(
                got["error"]["code"], u["result"]["error"]["code"],
                "{name}: {got}"
            );
            assert!(!second.exists(), "{name}");
            std::fs::remove_dir_all(dir).unwrap();
            continue;
        }
        let o = &got["output"];
        assert_eq!(o["sheets"], u["sheets"], "{name}");
        if o["carried"].as_array().map_or(json!([]), |a| json!(a)) != u["carried"] {
            problems.push(format!(
                "{name}: carried {} ≠ {}",
                o["carried"], u["carried"]
            ));
        }
        let noted: Vec<&str> = o["issues"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["code"] == "revision_unchanged")
            .map(|i| i["sheet"].as_str().unwrap())
            .collect();
        if json!(noted) != u["notes"] {
            problems.push(format!("{name}: revision notes {noted:?} ≠ {}", u["notes"]));
        }
        assert_eq!(o["previous"]["path"], json!(first));
        assert_eq!(o["previous"]["date"], EARLIER);
        if op == CallOp::Plan {
            assert!(!second.exists(), "{name}: a plan writes nothing");
            std::fs::remove_dir_all(dir).unwrap();
            continue;
        }
        let now = manifest(&second);
        let was = manifest(&first);
        for sheet in u["sheets"].as_array().unwrap() {
            let sheet = sheet.as_str().unwrap();
            let file = file_of(&now, sheet);
            let bytes = std::fs::read(second.join(file["name"].as_str().unwrap())).unwrap();
            assert_eq!(json!(sha256_hex(&bytes)), file["sha256"], "{name}: {sheet}");
            let page = page_of(&now, sheet);
            if u["carried"].as_array().unwrap().iter().any(|s| s == sheet) {
                assert_eq!(
                    file["sha256"],
                    file_of(&was, sheet)["sha256"],
                    "{name}: {sheet} carried byte for byte"
                );
                assert_eq!(page["date"], EARLIER, "{name}: {sheet} keeps its date");
            } else {
                assert_eq!(page["date"], json!(today), "{name}: {sheet} made today");
                if let Some(d) = current_digest.get(sheet) {
                    assert_eq!(&page["digest"], d, "{name}: {sheet} draws the drawing now");
                }
            }
        }
        // The update is a whole delivery of the drawing now.
        let st = file_call(
            &mut doc,
            "file.publish.status",
            CallOp::Plan,
            json!({"path": second}),
        );
        assert!(
            st["output"]["stale"].as_array().unwrap().is_empty(),
            "{name}: {st}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// GDAL's page georeference: X, Y of a page point given in mm from the page's top left.
fn gdal_ground(pdf: &Path) -> Option<(impl Fn(f64, f64) -> (f64, f64), String)> {
    let out = std::process::Command::new("gdalinfo")
        .arg("-json")
        .arg(pdf)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    let gt: Vec<f64> = v["geoTransform"]
        .as_array()?
        .iter()
        .filter_map(Value::as_f64)
        .collect();
    // GDAL rasterises the page to whole pixels on each axis (A3: 2480 × 1754 at 150 dpi) and georeferences that raster.
    let (sx, sy) = (v["size"][0].as_f64()? / 420., v["size"][1].as_f64()? / 297.);
    let wkt = v["coordinateSystem"]["wkt"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    Some((
        move |x: f64, y: f64| {
            let (px, py) = (x * sx, y * sy);
            (
                gt[0] + px * gt[1] + py * gt[2],
                gt[3] + px * gt[4] + py * gt[5],
            )
        },
        wkt,
    ))
}

#[test]
fn the_same_parcels_at_1_500_1_1000_and_1_5000_read_back_at_their_scales_and_ground() {
    let fx = fixture();
    let mut doc = model(&fx);
    let dir = temp("acceptance");
    let target = dir.join("teslim");
    let p = file_call(
        &mut doc,
        "file.publish.sheets",
        CallOp::Execute,
        json!({"path": target, "mode": "separate"}),
    );
    assert_eq!(p["status"], "completed", "{p}");
    let m = manifest(&target);
    let frame = &fx["acceptance"]["frame"];
    let (left, top, width, height) = (
        frame["left"].as_f64().unwrap(),
        frame["top"].as_f64().unwrap(),
        frame["width"].as_f64().unwrap(),
        frame["height"].as_f64().unwrap(),
    );
    let gdal = std::process::Command::new("gdalinfo")
        .arg("--version")
        .output()
        .is_ok();
    let poppler = std::process::Command::new("pdftotext")
        .arg("-v")
        .output()
        .is_ok();
    for a in fx["acceptance"]["sheets"].as_array().unwrap() {
        let sheet = a["sheet"].as_str().unwrap();
        let path = target.join(file_of(&m, sheet)["name"].as_str().unwrap());
        let bytes = std::fs::read(&path).unwrap();
        let pages = files::pdf_pages(&bytes);
        assert_eq!(pages.len(), 1);
        let page = &pages[0];
        assert!((page.width - 420.).abs() < 0.002 && (page.height - 297.).abs() < 0.002);
        // A 20 m parcel edge is 20 000 / scale mm long on paper.
        let edge = a["edge"].as_f64().unwrap();
        let edges = page
            .segments
            .iter()
            .filter(|s| ((s[2] - s[0]).hypot(s[3] - s[1]) - edge).abs() < 0.005)
            .count();
        assert!(
            edges > 0,
            "{sheet}: no {edge} mm edge among {} segments",
            page.segments.len()
        );
        if gdal {
            let (ground, wkt) =
                gdal_ground(&path).unwrap_or_else(|| panic!("{sheet}: GDAL reads no georeference"));
            assert!(
                wkt.contains("TM30") || wkt.contains("5254"),
                "{sheet}: {wkt}"
            );
            let g: Vec<f64> = a["ground"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            let (x0, y1) = ground(left, top);
            let (x1, y0) = ground(left + width, top + height);
            for (got, want) in [(x0, g[0]), (y0, g[1]), (x1, g[2]), (y1, g[3])] {
                assert!(
                    (got - want).abs() < 0.05,
                    "{sheet}: GDAL reads {got:.3}, the frame's ground is {want:.3}"
                );
            }
            eprintln!(
                "{sheet}: GDAL corners ({x0:.3}, {y1:.3}) – ({x1:.3}, {y0:.3}); edge {edge} mm × {edges}"
            );
        }
        if poppler {
            let out = std::process::Command::new("pdftotext")
                .arg(&path)
                .arg("-")
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(
                text.contains("Rev. A") && text.contains(&kentos_plot::book::today()),
                "{sheet}: {text}"
            );
        }
    }
    // One parcel revised: only its pages are made again, the others are carried byte for byte.
    let parcel = doc.uid(Slot(37)).unwrap().to_string();
    let answer = kentos_native_application::call::call(
        &mut doc,
        None,
        env(
            "cad.entities.transform",
            CallOp::Execute,
            json!({"uids": [parcel], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}),
        ),
    );
    assert!(matches!(answer.result, CommandResult::Completed { .. }));
    let again = dir.join("teslim-2");
    let u = file_call(
        &mut doc,
        "file.publish.sheets",
        CallOp::Execute,
        json!({"path": again, "mode": "separate", "previous": target}),
    );
    assert_eq!(u["status"], "completed", "{u}");
    assert_eq!(
        u["output"]["carried"],
        json!(["bati-500", "genel-1000"]),
        "{u}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// The cost of a page source record on a large drawing (docs/adr/1320 §6): 100 000 parcels under the four sheets.
#[test]
#[ignore = "timing, run alone"]
fn page_sources_of_a_hundred_thousand_parcels() {
    let fx = fixture();
    let mut setup = fx["setup"].clone();
    let one = setup["entities"][0].clone();
    let mut all = Vec::with_capacity(100_000);
    for n in 0..100_000u32 {
        let (c, r) = (f64::from(n % 500), f64::from(n / 500));
        let mut e = one.clone();
        e["id"] = json!(n + 1);
        e["pts"] = json!([
            {"x": 499500.0 + c * 2.0, "y": 4539800.0 + r * 2.0}, {"x": 499501.5 + c * 2.0, "y": 4539800.0 + r * 2.0},
            {"x": 499501.5 + c * 2.0, "y": 4539801.5 + r * 2.0}, {"x": 499500.0 + c * 2.0, "y": 4539801.5 + r * 2.0}
        ]);
        all.push(e);
    }
    setup["entities"] = json!(all);
    let mut doc = Document::from_snapshot(serde_json::from_value(setup).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let ids: Vec<String> = book.sheets.iter().map(|s| s.id.clone()).collect();
    let t = std::time::Instant::now();
    let pages = kentos_native_application::sheets_sources::page_sources(&doc, &ids).unwrap();
    let listed = t.elapsed();
    let digests = kentos_native_application::sheets_sources::object_digests(
        &doc,
        pages.iter().flat_map(|(_, u)| u.iter().flatten()),
    );
    let digested = t.elapsed() - listed;
    eprintln!(
        "page sources of 100 000 objects: {listed:?} ({} listed on {} pages), digests {digested:?} ({} objects)",
        pages
            .iter()
            .map(|(_, u)| u.as_ref().map_or(0, Vec::len))
            .sum::<usize>(),
        pages.len(),
        digests.len()
    );
}
