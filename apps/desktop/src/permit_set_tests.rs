//! Ruhsat takımı (M37, docs/adr/3037): a two-storey building of four flats on a 400 m² parcel; its particulars given
//! once, the set made in MO's order on the MO sheet, PAİY 57(10)'s table reading the drawing, and the set published as
//! PDF/A-2 files named by the digital project regulation's EK-2.

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn run(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// A 15 × 6 m building of two storeys, each a flat, a stair hall and a flat; a 400 m² parcel of emsal 1.50, TAKS 0.40.
pub(crate) fn building() -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    run(
        &mut app,
        "cad.settings.set",
        json!({ "name": "Gündoğdu Konutu", "arch": { "levels": [
            { "id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0 },
            { "id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0 } ],
            "zoning": { "parcelArea": 400.0, "far": 1.5, "coverage": 0.4 } } }),
    );
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    for level in ["zemin", "k1"] {
        let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}});
        run(
            &mut app,
            "cad.walls.create",
            json!({"layerId": layer, "walls": [
                wall((0.125, 0.125), (14.875, 0.125), 0.25), wall((14.875, 0.125), (14.875, 5.875), 0.25),
                wall((14.875, 5.875), (0.125, 5.875), 0.25), wall((0.125, 5.875), (0.125, 0.125), 0.25),
                wall((5.0, 0.125), (5.0, 5.875), 0.15), wall((10.0, 0.125), (10.0, 5.875), 0.15)
            ]}),
        );
    }
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"}, {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "d3", "number": "3", "nature": "Daire"}, {"id": "d4", "number": "4", "nature": "Daire"},
            {"id": "ortak", "number": "O", "name": "Merdiven evi", "kind": "common"}
        ]}}),
    );
    let room = |x: f64, level: &str, unit: &str, name: &str| json!({"seed": p(x, 3.0), "arch": {"level": level}, "unit": unit, "name": name});
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            room(2.5, "zemin", "d1", "Salon"), room(7.5, "zemin", "ortak", "Merdiven evi"), room(12.5, "zemin", "d2", "Salon"),
            room(2.5, "k1", "d3", "Salon"), room(7.5, "k1", "ortak", "Merdiven evi"), room(12.5, "k1", "d4", "Salon")
        ]}),
    );
    app
}

/// What the owner types once in Ruhsat takımı.
pub(crate) fn particulars() -> Vec<(String, String)> {
    [
        ("yapi_sahibi", "Ayşe Yılmaz"),
        ("il", "Sivas"),
        ("ilce", "Merkez"),
        ("mahalle", "Kardeşler"),
        ("ada", "1234"),
        ("parsel", "7"),
        ("tasiyici_sistem", "Betonarme karkas"),
        ("mimar", "Cihad Gündoğdu"),
        ("sicil_no", "41234"),
        ("btb_no", "58-2026-0012"),
        ("buro_adres", "İstasyon Cad. 12, Merkez / Sivas"),
        ("proje_kodu", "260457"),
    ]
    .iter()
    .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
    .collect()
}

/// The texts a sheet writes, as the PDF writes them (the drawing's variables and facts as the publication reads them).
fn texts(app: &App, sheet: &str) -> Vec<String> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let (book, _) =
        kentos_sheet::project::read_project_sheets(doc.sheets().expect("a book")).expect("reads");
    let inputs = kentos_plot::book::base_inputs(doc, "test", "2026-11-09");
    let list = kentos_sheet::display::display_list(&book, sheet, &inputs).expect("drawn");
    list.prims
        .iter()
        .filter_map(|p| match p {
            kentos_sheet::display::Prim::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_permit_set_is_made_from_the_drawing_and_published_as_pdf_a_named_by_ek_2() {
    let mut app = building();
    if let Some(out) = std::env::var_os("KENTOS_PERMIT_OUT") {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("ruhsat.kcad"), bytes).expect("written");
    }
    {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let missing: Vec<&str> = kentos_native_application::permit_set::missing_particulars(doc)
            .iter()
            .map(|(n, _)| *n)
            .collect();
        assert!(
            missing.contains(&"mimar") && missing.contains(&"ada"),
            "{missing:?}"
        );
        assert!(
            !missing.contains(&"proje_adi"),
            "the work's name is the drawing's"
        );
    }
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    // The calculation tables the drawing lacks, made beside it (docs/adr/3037 §3).
    let tables = kentos_interaction::calc_tables::ensure(doc).expect("the tables");
    let labels: Vec<&str> = tables.iter().map(|t| t.label).collect();
    assert_eq!(
        labels,
        [
            "Metrekare cetveli",
            "Emsal ve TAKS",
            "Kat irtifakı",
            "Otopark hesabı",
            "Sığınak hesabı"
        ]
    );
    let made_tables = doc
        .entities()
        .filter(|e| matches!(e, kentos_commands::Entity::Table(_)))
        .count();
    assert_eq!(made_tables, 5);
    // Found, not made again.
    let again = kentos_interaction::calc_tables::ensure(doc).expect("the tables");
    assert_eq!(again, tables);
    assert_eq!(
        doc.entities()
            .filter(|e| matches!(e, kentos_commands::Entity::Table(_)))
            .count(),
        5
    );
    let input = kentos_native_application::permit_set::PermitSet {
        phase: kentos_sheet::permit::Phase::Permit,
        values: particulars(),
        tables: tables
            .iter()
            .map(|t| (t.label.to_owned(), t.bounds))
            .collect(),
    };
    let (edited, made) = kentos_native_application::permit_set::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        &input,
    )
    .expect("made");
    assert!(edited.label.contains("Pafta") || !edited.label.is_empty());
    let titles: Vec<String> = made
        .iter()
        .map(|m| format!("{} {}", m.number, m.title))
        .collect();
    assert_eq!(
        titles,
        [
            "UP-01 Proje bilgileri",
            "UP-02 Vaziyet planı",
            "UP-03 Zemin kat planı",
            "UP-04 1. kat planı",
            "UP-05 Mimari hesap raporu"
        ]
    );
    assert_eq!(
        kentos_native_application::permit_set::missing_particulars(doc),
        Vec::<(&str, &str)>::new(),
        "every particular has a value"
    );
    // PAİY 57(10)'s table reads the drawing: the parcel, the floors, the units, the use.
    let info = texts(&app, &made[0].sheet);
    let has = |t: &str| info.iter().any(|x| x.contains(t));
    for want in [
        "İLK PAFTA BİLGİ TABLOSU (PAİY 57/10)",
        "Sivas / Merkez / Kardeşler",
        "— / 1234 / 7",
        "400.00 m²",
        "2 (bodrum 0)",
        "1.50 / 0.40",
        "Konut",
        "Cihad Gündoğdu, Mimar · sicil 41234",
        "PAFTA LİSTESİ",
        "UP-04",
    ] {
        assert!(has(want), "{want} on the first sheet: {info:?}");
    }
    // The title block: the work, the sheet, its number and its EK-2 file name.
    let plan = texts(&app, &made[2].sheet);
    for want in [
        "Gündoğdu Konutu",
        "Zemin kat planı",
        "UP-03",
        "260457_00_MM_XXXX_PL_MMC_K00_01_000",
        "41234 / 58-2026-0012",
    ] {
        assert!(
            plan.iter().any(|x| x.contains(want)),
            "{want} in the title block: {plan:?}"
        );
    }
    // The calculation sheet: a locked map on each table, captioned, at the plot scale; the tables' own rows.
    let calc = texts(&app, &made[4].sheet);
    for want in [
        "Metrekare cetveli — Ölçek 1/100",
        "Otopark hesabı — Ölçek 1/100",
        "Sığınak hesabı — Ölçek 1/100",
        "UP-05",
        "260457_00_MM_XXXX_HS_MIR_NNN_01_000",
    ] {
        assert!(
            calc.iter().any(|x| x.contains(want)),
            "{want} on the calculation sheet: {calc:?}"
        );
    }
    // Published as PDF/A-2 files named by EK-2.
    let dir = std::env::temp_dir().join(format!("kentos-ruhsat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let target = dir.join("Ruhsat");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let answer = kentos_headless::file_commands::call(
        doc,
        kentos_commands::CallEnvelope {
            command: kentos_commands::FILE_PUBLISH_SHEETS.into(),
            version: Some(1),
            op: kentos_commands::CallOp::Execute,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input: json!({
                "path": target,
                "sheets": made.iter().map(|m| m.sheet.clone()).collect::<Vec<_>>(),
                "mode": "separate",
                "pdfa": true,
                "naming": kentos_sheet::permit::FILE_NAMING
            }),
        },
    );
    let got = serde_json::to_value(&answer.result).expect("an answer");
    assert_eq!(got["status"], "completed", "{got}");
    let names: Vec<String> = got["output"]["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|f| f["name"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "260457_00_MM_XXXX_GN_MMC_NNN_01_000.pdf",
            "260457_00_MM_XXXX_PL_VAZ_NNN_01_000.pdf",
            "260457_00_MM_XXXX_PL_MMC_K00_01_000.pdf",
            "260457_00_MM_XXXX_PL_MMC_K01_01_000.pdf",
            "260457_00_MM_XXXX_HS_MIR_NNN_01_000.pdf",
        ]
    );
    for n in &names {
        let bytes = std::fs::read(target.join(n)).expect("delivered");
        let report = kentos_formats::pdf::pdfa::pdfa2_report(&bytes).expect("read back");
        assert!(report.conforms(), "{n}: {:?}", report.findings);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
