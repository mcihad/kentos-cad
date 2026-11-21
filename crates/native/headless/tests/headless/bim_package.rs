//! The BIM permit package (docs/adr/8075; plan/bim-plan.md BIM-14): the section building's permit set published with
//! its IFC model and its model quality control form, by the standards package's real rule. As drawn, the rule finds
//! what the model still lacks (its north, its fire classes …) and the questions left to the author, and nothing is
//! published. Declared as an author does (docs/adr/8076: the system, the parcel, the fire classes and exit roles, the
//! finishes, the postal box), the same building passes and goes out whole: the model read back from the folder, the
//! form's record, the delivery list and `yayin.json` naming both.

use std::fs;
use std::path::{Path, PathBuf};

use kentos_commands::{CallEnvelope, CallOp, CommandResult, SheetsPublished, encoding::sha256_hex};
use kentos_domain::Document;
use serde_json::{Value, Value as Json, json};

fn run(doc: &mut Document, name: &str, input: Value) -> Value {
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        name,
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// The author's declaration with no document behind it.
fn author(value: &str) -> Value {
    json!({"value": value, "basis": {"kind": "author"}})
}

/// What an author completes for the permit's model (docs/adr/8076): the rooms' finishes, the project's system
/// (TUREF / TM36, Antalya heights, ±0.00 at 1285.40 m), the parcel in the Vaziyet planı, the fire classes and exit
/// roles, the building's fire protection class.
fn declare(snap: &mut kentos_commands::DocumentSnapshotV2) {
    for e in &mut snap.entities {
        if let kentos_commands::Entity::Space(s) = e {
            s.space.finishes.floor = Some("Seramik".into());
            s.space.finishes.wall = Some("Plastik boya".into());
            s.space.finishes.ceiling = Some("Plastik boya".into());
        }
    }
    // Dijital teslim kapsamı: a new building (EK-7 Tablo 7.1's project type, EK-6 Tablo 6.7's construction method).
    snap.settings.variables.push(kentos_commands::ProjectVariable {
        name: "yapi_turu".into(),
        label: "Yapı türü".into(),
        kind: kentos_commands::VariableKind::Text,
        value: kentos_commands::VariableValue::Text("yeni bina".into()),
    });
    // The Vaziyet planı's own layer, apart from the model's (a free line on a model layer stops the package).
    snap.layers.push(
        serde_json::from_value(json!({
            "id": "vaziyet", "name": "Vaziyet", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []
        }))
        .expect("a layer"),
    );
    snap.settings.srid = 5256;
    snap.settings.heights = Some(kentos_commands::HeightReference {
        kind: kentos_commands::HeightKind::Orthometric,
        vertical_srid: Some(5775),
        geoid: None,
    });
}

/// The parcel around the walls (5 m to the road, 3 m at the sides and back), its ±0.00's elevation, and the author's
/// fire classes and exit roles by `cad.arch.performance`.
fn declare_in(doc: &mut Document) {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for e in doc.entities() {
        if let kentos_commands::Entity::Wall(w) = e {
            for p in &w.wall.pts {
                lo = [lo[0].min(p.x), lo[1].min(p.y)];
                hi = [hi[0].max(p.x), hi[1].max(p.y)];
            }
        }
    }
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let made = run(
        doc,
        "cad.entities.create",
        json!({"layerId": "vaziyet", "objects": [{"geometry": {"kind": "polyline", "closed": true, "pts": [
            p(lo[0] - 3.0, lo[1] - 5.0), p(hi[0] + 3.0, lo[1] - 5.0), p(hi[0] + 3.0, hi[1] + 3.0), p(lo[0] - 3.0, hi[1] + 3.0)
        ]}}]}),
    );
    let parcel = made["output"]["created"][0].as_str().expect("the parcel").to_owned();
    let mut settings = doc.settings().clone();
    use kentos_commands::{SiteEdge, SiteEdgeKind as K};
    let edge = |kind| SiteEdge { kind, setback: None };
    settings.arch.get_or_insert_with(Default::default).site = Some(kentos_commands::SitePlan {
        parcel: Some(parcel),
        edges: vec![edge(K::Front), edge(K::Side), edge(K::Back), edge(K::Side)],
        datum: Some(1285.4),
        envelope: None,
    });
    doc.set_settings(settings);
    let uids = |doc: &Document, keep: &dyn Fn(&kentos_commands::Entity) -> bool| -> Vec<String> {
        doc.entities_with_uids().filter(|(_, e)| keep(e)).map(|(u, _)| u.to_string()).collect()
    };
    use kentos_commands::Entity as E;
    // Each kind the drawing has (an apartment may have no stair).
    let structure = uids(doc, &|e| matches!(e, E::Wall(_) | E::Slab(_) | E::Column(_) | E::Beam(_) | E::Stair(_)));
    if !structure.is_empty() {
        run(doc, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": structure, "fireRating": author("REI 60")}}));
    }
    let doors = uids(doc, &|e| matches!(e, E::Opening(o) if o.opening.form == kentos_commands::OpeningForm::Door));
    if !doors.is_empty() {
        run(doc, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": doors, "fireRating": author("EI 30"),
            "exit": {"exit": false, "basis": {"kind": "author"}}}}));
    }
    let stairs = uids(doc, &|e| matches!(e, E::Stair(_)));
    if !stairs.is_empty() {
        run(doc, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": stairs, "exit": {"exit": true, "basis": {"kind": "author"}}}}));
    }
    run(doc, "cad.arch.performance", json!({"operation": {"kind": "building", "fireProtectionClass": author("Konut, düşük tehlike")}}));
}

/// The section building with a künye and its permit set made (Ruhsat takımı, docs/adr/8037); `declared`, completed
/// for the model as an author does.
fn permit_drawing(declared: bool) -> Document {
    let snap: kentos_commands::DocumentSnapshotV2 =
        kentos_kcad::decode(include_bytes!("../../../../../fixtures/arch/v1/section-building.kcad")).unwrap();
    permit_drawing_of(snap, declared)
}

/// [`permit_drawing`] on any drawing's snapshot.
fn permit_drawing_of(mut snap: kentos_commands::DocumentSnapshotV2, declared: bool) -> Document {
    if declared {
        declare(&mut snap);
    }
    // The parcel's İmar durumu (the first sheet's table reads its area), as Mimari › Parsel gives it.
    snap.settings.arch.get_or_insert_with(Default::default).zoning =
        Some(kentos_commands::Zoning { parcel_area: 400.0, far: Some(1.5), coverage: Some(0.4), ..Default::default() });
    // A drawing without a building program says its independent units and its use by hand.
    for (name, label, value) in [("bb_sayisi", "Bağımsız bölüm sayısı", "2"), ("kullanim_amaci", "Kullanım amacı", "Konut")] {
        snap.settings.variables.push(kentos_commands::ProjectVariable {
            name: name.into(),
            label: label.into(),
            kind: kentos_commands::VariableKind::Text,
            value: kentos_commands::VariableValue::Text(value.into()),
        });
    }
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    if declared {
        declare_in(&mut doc);
    }
    let values: Vec<(String, String)> = [
        ("proje_kodu", "260123"),
        ("il", "Sivas"),
        ("ilce", "Merkez"),
        ("mahalle", "Esentepe"),
        ("ada", "1234"),
        ("parsel", "5"),
        ("mimar", "Ayşe Nur Yılmaz"),
        ("mimar_soyadi", "Yılmaz"),
        ("sicil_no", "41234"),
        ("buro_adi", "Örnek Mimarlık"),
        ("vergi_no", "1234567890"),
        ("buro_adres", "Atatürk Cad. No:1 Sivas"),
        ("proje_aciklama", "İki katlı konut"),
        ("btb_no", "58-2026-0012"),
        ("yapi_sahibi", "Ayşe Yılmaz"),
        ("tasiyici_sistem", "Betonarme karkas"),
        ("posta_kutusu", "Yok"),
        ("tapu_kaydi", "12/345"),
    ]
    .iter()
    .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
    .collect();
    let input = kentos_native_application::permit_set::PermitSet {
        phase: kentos_sheet::permit::Phase::Permit,
        values,
        tables: Vec::new(),
        lists: Vec::new(),
        renovation: false,
        building: None,
    };
    kentos_native_application::permit_set::execute(&mut kentos_native_application::ExecutionContext::new(&mut doc), &input)
        .expect("the permit set");
    doc
}

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-bim-paket-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn publish(doc: &mut Document, path: &Path, op: CallOp, answers: Value) -> kentos_commands::CallAnswer {
    let input = json!({
        "path": path.display().to_string(),
        "mode": "separate",
        "pdfa": true,
        "naming": kentos_sheet::permit::FILE_NAMING,
        "bim": {"answers": answers},
    });
    kentos_headless::file_commands::call(
        doc,
        CallEnvelope {
            command: "file.publish.sheets".into(),
            version: None,
            op,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input,
        },
    )
}

fn report(answer: &kentos_commands::CallAnswer) -> SheetsPublished {
    let CommandResult::Completed { output, .. } = &answer.result else { panic!("{answer:?}") };
    serde_json::from_value(output.clone()).unwrap()
}

/// The author's answers to the questions KentOS leaves to them (the file size, the area tables); the 7th, the
/// development level, is the written file's (docs/adr/8133).
fn answers() -> Value {
    json!([{"no": 1, "yes": true, "note": "Sınır yayımlanmadı"}, {"no": 10, "yes": true}])
}

#[test]
fn the_real_rule_stops_what_the_model_lacks_and_publishes_a_declared_building_whole() {
    let mut doc = permit_drawing(false);
    let path = folder("gercek");
    // The plan: the model read from memory, the form answered, its five results and rows, its findings as errors.
    let planned = report(&publish(&mut doc, &path, CallOp::Plan, json!([])));
    assert!(!planned.ready);
    let bim = planned.bim.as_ref().expect("the package's part");
    assert_eq!(bim.model, "260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc");
    assert_eq!(bim.rows.len(), 21);
    assert_eq!(bim.results.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(), ["okunuyor", "sema", "profil", "model_pafta", "idare"]);
    assert_eq!(bim.results.iter().map(|r| r.holds).collect::<Vec<_>>(), [Some(true), Some(true), Some(false), Some(true), None]);
    let codes: Vec<&str> = planned.issues.iter().filter(|i| i.error).map(|i| i.code.as_str()).collect();
    // The 10th (docs/adr/8126) and the 7th (docs/adr/8133) are answered from the file: one left to the author.
    assert_eq!(codes.iter().filter(|c| **c == "digital_form_unanswered").count(), 1, "{codes:?}");
    assert!(codes.contains(&"digital_required_field"), "{codes:?}");
    // A drawing without a system or a north arrow: the model's north is not known, and the finding says what gives it.
    let north = planned.issues.iter().find(|i| i.message.contains("(TrueNorth)")).expect("the context's north");
    assert_eq!(north.sheet, bim.model);
    assert!(north.fix.contains("Kuzey oku"), "{}", north.fix);
    // With the answers the questions are no finding; the model's missing fields still stop the publication.
    let answered = report(&publish(&mut doc, &path, CallOp::Plan, answers()));
    assert!(!answered.issues.iter().any(|i| i.code == "digital_form_unanswered"));
    let done = publish(&mut doc, &path, CallOp::Execute, answers());
    let CommandResult::Failed { error } = &done.result else { panic!("{done:?}") };
    assert_eq!(error.code, "publish_blocked");
    assert!(error.message.contains("TrueNorth"), "{}", error.message);
    assert!(!path.exists(), "nothing published");
    // Declared as an author does, the same building passes the real rule and goes out whole.
    a_declared_building_is_published_whole();
}

fn a_declared_building_is_published_whole() {
    let mut doc = permit_drawing(true);
    // `KENTOS_WRITE_BIM_DRAWING=1` keeps the declared drawing, its set made and no answer given, for the agents' test
    // (crates/native/mcp/tests/bim_package.rs): fixtures/arch/v1/kesit-binasi-tam.kcad.
    if std::env::var_os("KENTOS_WRITE_BIM_DRAWING").is_some() {
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/arch/v1/kesit-binasi-tam.kcad");
        fs::write(out, kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes")).expect("the drawing kept");
    }
    // The author's answers kept in the drawing (`cad.form.answer`, docs/adr/8077): the publication reads them, none given
    // to it.
    run(&mut doc, "cad.form.answer", json!({"answers": answers()}));
    // `KENTOS_BIM_PAKET=<folder>` keeps the published package there, for the independent readers (docs/adr/8075 §6).
    let keep = std::env::var_os("KENTOS_BIM_PAKET").map(PathBuf::from);
    let path = keep.clone().unwrap_or_else(|| folder("tam"));
    let answer = publish(&mut doc, &path, CallOp::Execute, json!([]));
    let published = report(&answer);
    assert!(published.ready, "{:#?}", published.issues.iter().filter(|i| i.error).map(|i| (&i.code, &i.message)).collect::<Vec<_>>());
    let bim = published.bim.as_ref().expect("the package's part");
    // The model's profile holds; the administration's acceptance is not KentOS's to say.
    assert_eq!(bim.results.iter().map(|r| r.holds).collect::<Vec<_>>(), [Some(true), Some(true), Some(true), Some(true), None]);
    // The model as delivered: read back from the folder, its digest the delivery's.
    let model = path.join(&bim.model);
    let bytes = fs::read(&model).expect("the model in the folder");
    let listed = published.files.iter().find(|f| f.name == bim.model).expect("the model among the files");
    assert_eq!((listed.bytes, listed.sha256.as_str()), (bytes.len() as u64, sha256_hex(&bytes).as_str()));
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("FILE_SCHEMA(('IFC4X3_ADD2'))"));
    assert!(text.contains("'PostalBox'") && text.contains("'ElevationOfSSLRelative'") && text.contains("'WallCoveringThickness'"));
    // The form's record names the model by its digest and keeps the author's answers and the real rule's version.
    let record: Value = serde_json::from_slice(&fs::read(path.join(&bim.form)).expect("the form's record")).unwrap();
    assert_eq!(record["model"]["sha256"], listed.sha256);
    assert_eq!(record["answers"][0]["note"], "Sınır yayımlanmadı");
    assert_eq!(record["rows"].as_array().map(Vec::len), Some(21));
    assert!(record["versions"]["standards"].as_str().is_some_and(|v| v.starts_with("dijital-proje ")), "{}", record["versions"]);
    // Every question is answered yes, KentOS's evidence or the author's.
    assert!(record["rows"].as_array().unwrap().iter().all(|r| r["answer"] == "Evet"), "{}", record["rows"]);
    // The delivery list counts the sheets and the model, each once with its digest.
    let list = fs::read_to_string(path.join("teslim-listesi.csv")).expect("the delivery list");
    for f in &published.files {
        assert_eq!(list.matches(&format!(";{};{};", f.name, f.sha256)).count(), 1, "{}", f.name);
    }
    assert!(list.contains(";IFC modeli;"), "{list}");
    // The form's record is KentOS's own, beside yayin.json: not a delivered file, named there with its digest.
    assert!(!published.files.iter().any(|f| f.name == bim.form) && !list.contains(&bim.form), "{list}");
    let manifest: Value = serde_json::from_slice(&fs::read(path.join("yayin.json")).unwrap()).unwrap();
    assert_eq!(manifest["bim"]["model"], bim.model);
    let record_bytes = fs::read(path.join(&bim.form)).unwrap();
    assert_eq!(manifest["bim"]["form"]["sha256"], sha256_hex(&record_bytes));
    if keep.is_none() {
        let _ = fs::remove_dir_all(&path);
    }
}

/// The acceptance flow's 2+1 apartment (fixtures/acceptance/v1/flat.json, drawn by its commands) declared as an author
/// does, its package published with the real rule (docs/adr/8076, 8077; plan/bim-plan.md BIM-14 "2+1'de paket").
/// What the apartment's author completes for the model beyond the declarations (the real rule's first findings on it):
/// the walls' styles by their widths (Dış duvar 25 outside, İç duvar 15 inside: their materials and plasters), the
/// floor slab's material and layers (Zemine oturan döşeme), a terrace slab over the storey (its height and ceiling) and
/// the cooktop's height.
fn complete_flat(snap: &mut kentos_commands::DocumentSnapshotV2) {
    use kentos_commands::Entity as E;
    let styles: Vec<kentos_commands::WallStyleDef> = kentos_commands::builtin_wall_styles()
        .into_iter()
        .filter(|s| s.id == "kentos-dis-duvar-25" || s.id == "kentos-ic-duvar-15")
        .collect();
    for e in &mut snap.entities {
        if let E::Wall(w) = e {
            let id = if w.wall.width >= 0.2 { "kentos-dis-duvar-25" } else { "kentos-ic-duvar-15" };
            let style = styles.iter().find(|s| s.id == id).expect("the style");
            w.wall.style = Some(id.into());
            w.wall.components = style.components.clone();
        }
    }
    snap.settings.wall_styles.extend(styles);
    let height = snap.settings.arch.as_ref().and_then(|a| a.levels.first()).and_then(|l| l.height).expect("its height");
    let at = snap.entities.iter().position(|e| matches!(e, E::Slab(_))).expect("the floor slab");
    let layered = |over: Json, under: Json| -> Json { json!({"over": over, "under": under}) };
    let mut floor = serde_json::to_value(&snap.entities[at]).expect("the slab");
    floor["material"] = json!("Betonarme");
    floor["layers"] = layered(
        json!([{"kind": "finish", "thickness": 0.02}, {"kind": "screed", "thickness": 0.05},
               {"kind": "insulation", "thickness": 0.05}, {"kind": "waterproofing", "thickness": 0.01}]),
        json!([]),
    );
    let mut terrace = floor.clone();
    snap.entities[at] = serde_json::from_value(floor).expect("the floor slab");
    let next = snap.entities.iter().map(|e| e.base().id).max().unwrap_or(0) + 1;
    terrace["id"] = json!(next);
    terrace["offset"] = json!(height);
    terrace["role"] = json!("terrace");
    terrace["layers"] = layered(
        json!([{"kind": "finish", "thickness": 0.03}, {"kind": "waterproofing", "thickness": 0.01},
               {"kind": "screed", "thickness": 0.05}, {"kind": "insulation", "thickness": 0.08}]),
        json!([{"kind": "plaster", "thickness": 0.02}]),
    );
    snap.entities.push(serde_json::from_value(terrace).expect("the terrace"));
    snap.uids.push(serde_json::from_value(json!("0192f5a1-8077-7000-8000-00000000f1a7")).expect("an id"));
    // The cooktop's height on its block's card, the counter's 0.90 m as the sink's: a library appliance has none until
    // its author enters it (docs/adr/8070 §5), and an element without a body fails the 7th question (docs/adr/8133 K319).
    let cooktop = snap.blocks.iter_mut().find(|b| b.name == "Ocak").expect("the kitchen run's cooktop");
    cooktop.card.as_mut().and_then(|c| c.element.as_mut()).expect("an appliance").height = Some(0.9);
}

#[test]
fn the_flat_s_declared_package_holds() {
    let (session, _) = crate::flat_timing::drawn();
    let mut snap = session.document().to_snapshot_v2();
    complete_flat(&mut snap);
    let mut doc = permit_drawing_of(snap, true);
    // The 5th question too: the apartment's doors and windows are named without a description (K49), the author's call.
    // And the 7th: the kitchen's and the bathroom's library blocks are boxes over their outlines, so the level of
    // development is the author's review (docs/adr/8133 K318).
    let mut given = answers();
    let list = given.as_array_mut().expect("a list");
    list.push(json!({"no": 5, "yes": true, "note": "Açıklama EK-5 kalıbında isteğe bağlı"}));
    list.push(json!({"no": 7, "yes": true, "note": "Kitaplık blokları dış ölçüleriyle yeterli"}));
    run(&mut doc, "cad.form.answer", json!({"answers": given}));
    let path = folder("daire");
    let published = report(&publish(&mut doc, &path, CallOp::Execute, json!([])));
    let errors: Vec<(&str, &str)> =
        published.issues.iter().filter(|i| i.error).map(|i| (i.code.as_str(), i.message.as_str())).collect();
    assert!(published.ready, "{errors:#?}");
    let bim = published.bim.as_ref().expect("the package's part");
    assert_eq!(bim.results.iter().map(|r| r.holds).collect::<Vec<_>>(), [Some(true), Some(true), Some(true), Some(true), None]);
    // The model delivered beside the sheets; the form's record keeps the author's four answers.
    assert!(path.join(&bim.model).is_file());
    let record: Value = serde_json::from_slice(&fs::read(path.join(&bim.form)).expect("the form's record")).unwrap();
    let nos: Vec<u64> = record["answers"].as_array().expect("answers").iter().filter_map(|a| a["no"].as_u64()).collect();
    assert_eq!(nos, [1, 5, 7, 10]);
    let _ = fs::remove_dir_all(&path);
}

/// An answer remembers the evidence it answered (docs/adr/8077 §7): answered with the report's rows' KentOS notes, the
/// package holds; once the 1st question's evidence (the file size, the one question of the declared section building
/// left to its author: the 7th and the 10th are the file's, docs/adr/8126, 8133) is another than its answer's, the
/// answer is stale, the question open again and the publication stopped (`digital_form_answer_stale`).
#[test]
fn a_stale_answer_stops_the_package() {
    let mut doc = permit_drawing(true);
    let path = folder("eski");
    let first = report(&publish(&mut doc, &path, CallOp::Plan, json!([])));
    let rows = &first.bim.as_ref().expect("the package's part").rows;
    let basis = |no: u8| rows.iter().find(|r| r.no == no).map(|r| r.basis.clone()).expect("the row");
    let given: Vec<Value> = [1u8].iter().map(|&no| json!({"no": no, "yes": true, "basis": basis(no)})).collect();
    run(&mut doc, "cad.form.answer", json!({"answers": given}));
    let answered = report(&publish(&mut doc, &path, CallOp::Plan, json!([])));
    assert!(answered.ready, "{:?}", answered.issues.iter().filter(|i| i.error).map(|i| &i.code).collect::<Vec<_>>());
    // The evidence it answered is not the form's now: stale.
    run(&mut doc, "cad.form.answer", json!({"answers": [{"no": 1, "yes": true, "basis": "eski kanıt"}]}));
    let stale = report(&publish(&mut doc, &path, CallOp::Plan, json!([])));
    assert!(!stale.ready);
    let codes: Vec<&str> = stale.issues.iter().filter(|i| i.error).map(|i| i.code.as_str()).collect();
    assert_eq!(codes, ["digital_form_answer_stale"]);
    let row = stale.bim.as_ref().expect("the part").rows.iter().find(|r| r.no == 1).expect("row 1");
    assert_eq!(row.answer, "", "the stale answer is not the form's");
    assert!(!path.exists());
}
