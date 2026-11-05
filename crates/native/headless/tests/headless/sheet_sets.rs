//! CAD-ADV-16's acceptance (docs/adr/1090): one model's 1:1, 1:5, 1:20 and 1:100 details in one set; two viewports of
//! one detail with their own layer and annotation representation, one changed, the other kept; the PDFs read back
//! independently (the plot tests' own PDF reader and, when installed, Poppler's pdftotext/pdffonts) for physical
//! lengths, vectors and searchable text; a stale sheet and a stopped publication reported, the stopped one resumed.
//! The expected lengths are the model's lengths times 1000 / the ratio's denominator, computed here, never read from
//! KentOS's output.
use kentos_commands::{CallEnvelope, CallOp, CommandResult};
use kentos_domain::{Document, Slot};
use serde_json::{Value, json};
#[path = "../../../plot/tests/support/files.rs"]
mod files;

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.sheets.details.json");

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

fn model() -> Document {
    let fx: Value = serde_json::from_str(CASES).unwrap();
    Document::from_snapshot(serde_json::from_value(fx["setup"].clone()).unwrap()).unwrap()
}

fn uid(doc: &Document, id: u32) -> String {
    doc.uid(Slot(id)).unwrap().to_string()
}

/// A document command's output; panics on any other result.
fn run(doc: &mut Document, command: &str, input: Value) -> Value {
    let answer =
        kentos_native_application::call::call(doc, None, env(command, CallOp::Execute, input));
    match answer.result {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{command}: {other:?}"),
    }
}

/// A file command's result as JSON.
fn file(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Value {
    serde_json::to_value(kentos_headless::file_commands::call(doc, env(command, op, input)).result)
        .unwrap()
}

fn r(paper: f64, model: f64) -> Value {
    json!({"paper": paper, "model": model})
}

/// The set: one detail a sheet, each 300 × 150 mm, then two 1:5 viewports of one place on a fifth sheet.
fn set(doc: &mut Document) -> Value {
    run(
        doc,
        "cad.sheets.details",
        json!({"options": {"values": [{"name": "kurum", "value": "KentOS"}, {"name": "birim", "value": "Proje"}, {"name": "referans", "value": "ADR 1090"}, {"name": "onaylayan", "value": "Denetçi"}, {"name": "cizim_no", "value": "D-1"}]}, "details": [
            {"name": "Bulon", "bounds": [-0.1, -0.05, 0.2, 0.1], "ratio": r(1., 1.)},
            {"name": "Levha", "bounds": [-0.5, -0.25, 1.0, 0.5], "ratio": r(1., 5.)},
            {"name": "Bağlantı", "bounds": [-2.0, -1.0, 4.0, 2.0], "ratio": r(1., 20.)},
            {"name": "Genel", "bounds": [-15.0, -7.5, 15.0, 7.5], "ratio": r(1., 100.)},
        ]}),
    );
    run(
        doc,
        "cad.sheets.details",
        json!({"options": {"values": [{"name": "kurum", "value": "KentOS"}, {"name": "birim", "value": "Proje"}, {"name": "referans", "value": "ADR 1090"}, {"name": "onaylayan", "value": "Denetçi"}, {"name": "cizim_no", "value": "D-1"}]}, "details": [
            {"name": "Ölçülü", "bounds": [-0.05, -0.3, 0.45, 0.1], "ratio": r(1., 5.), "annotation": {"type": "fixed", "scale": 5.0}},
            {"name": "Ölçüsüz", "bounds": [-0.05, -0.3, 0.45, 0.1], "ratio": r(1., 5.),
             "layerOverrides": [{"layer": "olcu", "frozen": true}]},
        ]}),
    )
}

fn book(doc: &Document) -> kentos_sheet::SheetBook {
    kentos_sheet::project::read_project_sheets(doc.sheets().unwrap())
        .unwrap()
        .0
}

fn map_of<'a>(b: &'a kentos_sheet::SheetBook, name: &str) -> &'a kentos_sheet::Item {
    b.sheets
        .iter()
        .flat_map(|s| s.items.iter())
        .find(|i| i.name == name)
        .unwrap()
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-sheet-sets-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn lengths(page: &files::Page) -> Vec<f64> {
    page.segments
        .iter()
        .map(|p| (p[2] - p[0]).hypot(p[3] - p[1]))
        .collect()
}

fn has(all: &[f64], want: f64) -> usize {
    all.iter().filter(|l| (*l - want).abs() < 0.01).count()
}

#[test]
fn four_scales_publish_with_physical_vectors_searchable_text_and_kept_representations() {
    let mut doc = model();
    let made = set(&mut doc);
    assert_eq!(made["made"][0]["sheet"], "detay-5", "{made}");
    // Two viewports of one place: changing one's representation keeps the other's.
    let before = book(&doc);
    let kept = serde_json::to_value(map_of(&before, "Ölçülü")).unwrap();
    let id = map_of(&before, "Ölçüsüz").id.clone();
    run(
        &mut doc,
        "cad.sheets.edit",
        json!({"ops": [{"op": "setViewport", "id": id, "annotation": {"type": "fixed", "scale": 10.0},
                         "layerOverrides": [{"layer": "olcu", "frozen": true}, {"layer": "detay", "color": "#0000ff"}]}]}),
    );
    let after = book(&doc);
    assert_eq!(
        serde_json::to_value(map_of(&after, "Ölçülü")).unwrap(),
        kept
    );
    assert_ne!(
        serde_json::to_value(map_of(&after, "Ölçüsüz")).unwrap(),
        serde_json::to_value(map_of(&before, "Ölçüsüz")).unwrap()
    );
    // The set's recipe reads back and makes the same viewports in a fresh drawing.
    let set_now = kentos_native_application::call::call(
        &mut doc,
        None,
        env("cad.sheets.set", CallOp::Plan, json!({"recipes": true})),
    );
    let CommandResult::Completed { output: listed, .. } = set_now.result else {
        panic!("{set_now:?}")
    };
    let recipe: Vec<Value> = listed["sheets"].as_array().unwrap()[..4]
        .iter()
        .map(|s| s["frames"][0]["detail"].clone())
        .collect();
    assert!(recipe.iter().all(|d| d["locked"] == true), "{recipe:?}");
    let mut fresh = model();
    run(
        &mut fresh,
        "cad.sheets.details",
        json!({"options": {"values": [{"name": "kurum", "value": "KentOS"}, {"name": "birim", "value": "Proje"}, {"name": "referans", "value": "ADR 1090"}, {"name": "onaylayan", "value": "Denetçi"}, {"name": "cizim_no", "value": "D-1"}]}, "details": recipe}),
    );
    let (a, b) = (book(&doc), book(&fresh));
    for name in ["Bulon", "Levha", "Bağlantı", "Genel"] {
        let (x, y) = (map_of(&a, name), map_of(&b, name));
        assert_eq!(x.frame, y.frame, "{name}");
        let (kentos_sheet::ItemKind::Map(mx), kentos_sheet::ItemKind::Map(my)) = (&x.kind, &y.kind)
        else {
            panic!()
        };
        let (kentos_sheet::MapView::Fixed(vx), kentos_sheet::MapView::Fixed(vy)) =
            (&mx.view, &my.view)
        else {
            panic!()
        };
        assert_eq!(vx.ratio, vy.ratio, "{name}");
        let (cx, cy) = (vx.center.unwrap(), vy.center.unwrap());
        assert!(
            (cx.x - cy.x).abs() < 1e-9 && (cx.y - cy.y).abs() < 1e-9,
            "{name}"
        );
        assert_eq!(mx.view_locked, my.view_locked);
    }
    // Publish the set, a PDF a sheet.
    let dir = temp("four");
    let target = dir.join("teslim");
    let published = file(
        &mut doc,
        "file.publish.sheets",
        CallOp::Execute,
        json!({"path": target, "mode": "separate", "naming": "{index}-{number}"}),
    );
    assert_eq!(published["status"], "completed", "{published}");
    let out = &published["output"];
    assert_eq!(out["scope"].as_array().unwrap().len(), 5);
    assert_eq!(out["scope"][1]["frames"][0]["denominator"], 5.0);
    assert_eq!(out["scope"][1]["frames"][0]["locked"], true);
    let pages = out["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 5);
    for p in pages {
        assert!(
            p["strokes"].as_u64().unwrap() > 0 && p["texts"].as_u64().unwrap() > 0,
            "{p}"
        );
        assert!(
            !p["fonts"].as_array().unwrap().is_empty() && p["clips"].as_u64().unwrap() >= 1,
            "{p}"
        );
        assert_eq!(p["rasterMaps"], 0);
        assert_eq!(p["digest"].as_str().unwrap().len(), 64);
        // The drawing's own layer visibility is followed: said on every page.
        assert!(
            p["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["code"] == "viewport_live_layers"),
            "{p}"
        );
    }
    // The lines: line 1 is 0.116619 m, line 2 11.6619 m, line 3 (Ölçü) 0.37 m long.
    let (l1, l2, l3) = ((0.01f64 + 0.0036).sqrt(), (100f64 + 36.).sqrt(), 0.37);
    let mm = |m: f64, d: f64| m * 1000. / d;
    let files_out = out["files"].as_array().unwrap();
    let read_page = |i: usize| {
        let bytes = std::fs::read(target.join(files_out[i]["name"].as_str().unwrap())).unwrap();
        let p = files::pdf_pages(&bytes);
        assert_eq!(p.len(), 1);
        assert!((p[0].width - 420.).abs() < 0.002 && (p[0].height - 297.).abs() < 0.002);
        lengths(&p[0])
    };
    let (one, five, twenty, hundred, pair) = (
        read_page(0),
        read_page(1),
        read_page(2),
        read_page(3),
        read_page(4),
    );
    assert!(has(&one, mm(l1, 1.)) >= 1, "1:1 {one:?}");
    assert!(
        has(&five, mm(l1, 5.)) >= 1 && has(&five, mm(l3, 5.)) >= 1,
        "1:5 {five:?}"
    );
    assert!(
        has(&twenty, mm(l1, 20.)) >= 1 && has(&twenty, mm(l3, 20.)) >= 1,
        "1:20"
    );
    assert!(has(&hundred, mm(l2, 100.)) >= 1, "1:100");
    // Ölçü drawn in one of the two 1:5 viewports only: frozen in the other.
    assert_eq!(has(&pair, mm(l3, 5.)), 1, "{pair:?}");
    // Poppler, an independent reader: the titles are searchable text with the scale they read; the faces embedded.
    let pdftotext = std::process::Command::new("pdftotext").arg("-v").output();
    if pdftotext.is_ok() {
        for (i, want) in [
            (0, "Bulon — Ölçek 1/1"),
            (1, "Levha — Ölçek 1/5"),
            (2, "Bağlantı — Ölçek 1/20"),
            (3, "Genel — Ölçek 1/100"),
        ] {
            let path = target.join(files_out[i]["name"].as_str().unwrap());
            let text = std::process::Command::new("pdftotext")
                .arg(&path)
                .arg("-")
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&text.stdout);
            assert!(text.contains(want), "{want}: {text}");
            let fonts = std::process::Command::new("pdffonts")
                .arg(&path)
                .output()
                .unwrap();
            let fonts = String::from_utf8_lossy(&fonts.stdout);
            for line in fonts.lines().skip(2) {
                assert!(
                    line.split_whitespace().rev().nth(4) == Some("yes"),
                    "embedded: {line}"
                );
            }
        }
    } else {
        eprintln!("pdftotext yok: Poppler denetimi atlandı");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn only_the_sheet_a_change_affects_goes_stale_and_is_made_again_alone() {
    let mut doc = model();
    set(&mut doc);
    let dir = temp("stale");
    let target = dir.join("teslim");
    let p = file(
        &mut doc,
        "file.publish.sheets",
        CallOp::Execute,
        json!({"path": target, "mode": "separate"}),
    );
    assert_eq!(p["status"], "completed", "{p}");
    let st = file(
        &mut doc,
        "file.publish.status",
        CallOp::Plan,
        json!({"path": target}),
    );
    assert_eq!(st["status"], "completed", "{st}");
    assert_eq!(st["output"]["state"], "published");
    assert!(st["output"]["stale"].as_array().unwrap().is_empty(), "{st}");
    assert!(
        st["output"]["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["state"] == "current"),
        "{st}"
    );
    // Line 3 (Ölçü, at y −0.2) moved 1 cm along itself: the 1:1 detail (y −0.05…0.1) does not show it.
    let line = uid(&doc, 3);
    let affected = kentos_native_application::call::call(
        &mut doc,
        None,
        env("cad.sheets.set", CallOp::Plan, json!({"uids": [line]})),
    );
    let CommandResult::Completed { output: a, .. } = affected.result else {
        panic!()
    };
    let sheets: Vec<&str> = a["affected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["sheet"].as_str().unwrap())
        .collect();
    // The viewport that freezes Ölçü is not affected by an Ölçü line; its twin that draws it is (docs/adr/1090 §6).
    let b = book(&doc);
    let items: Vec<&str> = a["affected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["item"].as_str().unwrap())
        .collect();
    assert!(
        items.contains(&map_of(&b, "Ölçülü").id.as_str())
            && !items.contains(&map_of(&b, "Ölçüsüz").id.as_str()),
        "{a}"
    );
    run(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": [line], "transform": {"kind": "move", "dx": 0.01, "dy": 0.0}}),
    );
    let st = file(
        &mut doc,
        "file.publish.status",
        CallOp::Plan,
        json!({"path": target}),
    );
    let o = &st["output"];
    let stale: Vec<&str> = o["stale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(!stale.is_empty() && !stale.contains(&"detay-1"), "{o}");
    // The frames the change affects are on exactly the stale sheets; the published and current pins stay apart.
    let mut want: Vec<&str> = sheets.clone();
    want.dedup();
    assert_eq!(stale, want, "{a}");
    assert_ne!(o["current"]["sha256"], o["published"]["sha256"]);
    // Made again alone: its digest is the status's, the others' would be their published ones.
    let again = dir.join("yeniden");
    let p2 = file(
        &mut doc,
        "file.publish.sheets",
        CallOp::Execute,
        json!({"path": again, "mode": "separate", "sheets": stale}),
    );
    assert_eq!(p2["status"], "completed", "{p2}");
    for page in p2["output"]["pages"].as_array().unwrap() {
        let now = o["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["sheet"] == page["sheet"])
            .unwrap();
        assert_eq!(page["digest"], now["digest"]);
        assert_ne!(now["digest"], now["published"]);
    }
    // A delivered PDF altered on disk is said so.
    let first = p["output"]["files"][0]["name"].as_str().unwrap();
    std::fs::write(target.join(first), b"not a pdf").unwrap();
    let st = file(
        &mut doc,
        "file.publish.status",
        CallOp::Plan,
        json!({"path": target}),
    );
    assert_eq!(st["output"]["sheets"][0]["state"], "fileChanged", "{st}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_stopped_publication_is_reported_and_resumed_from_its_next_page() {
    let mut doc = model();
    set(&mut doc);
    let dir = temp("resume");
    let target = dir.join("teslim");
    let journal = dir.join(".teslim.kentos-devam").join("devam.json");
    let input = json!({"path": target, "mode": "separate", "resumable": true});
    let mut job = kentos_native_application::jobs::Job::start_with(
        "resume".into(),
        kentos_commands::FILE_PUBLISH_SHEETS.into(),
        None,
        &doc,
        kentos_headless::file_commands::job_work(
            env(
                kentos_commands::FILE_PUBLISH_SHEETS,
                CallOp::Execute,
                input.clone(),
            ),
            Vec::new(),
        ),
    );
    // Stopped as soon as the first finished page is journalled.
    let answer = loop {
        if journal.exists() {
            job.cancel();
        }
        let status = job.status(&mut doc);
        if let Some(answer) = status.answer {
            break answer;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert!(
        matches!(answer.result, CommandResult::Cancelled),
        "{answer:?}"
    );
    assert!(!target.exists(), "no delivery folder after a stop");
    let st = file(
        &mut doc,
        "file.publish.status",
        CallOp::Plan,
        json!({"path": target}),
    );
    let o = &st["output"];
    assert_eq!(o["state"], "stopped", "{st}");
    let stopped = &o["stopped"];
    assert_eq!(stopped["reason"], "cancelled");
    assert_eq!(stopped["resumable"], true);
    let done: Vec<Value> = stopped["done"].as_array().unwrap().clone();
    assert!(
        !done.is_empty() && !stopped["remaining"].as_array().unwrap().is_empty(),
        "the stop came after the last page: {stopped}"
    );
    // Resumed: the finished pages are kept, the rest made, the folder delivered whole.
    let mut resume = input.clone();
    resume["resume"] = true.into();
    let p = file(
        &mut doc,
        "file.publish.sheets",
        CallOp::Execute,
        resume.clone(),
    );
    assert_eq!(p["status"], "completed", "{p}");
    assert_eq!(p["output"]["resumed"], json!(done));
    assert_eq!(p["output"]["files"].as_array().unwrap().len(), 5);
    assert!(target.join("yayin.json").exists() && !journal.exists());
    // Nothing to resume now; a changed drawing is not resumed with the old pages.
    let again = file(
        &mut doc,
        "file.publish.sheets",
        CallOp::Execute,
        json!({"path": dir.join("baska"), "mode": "separate", "resume": true}),
    );
    assert_eq!(again["error"]["code"], "no_resume", "{again}");
    std::fs::remove_dir_all(dir).unwrap();
}
