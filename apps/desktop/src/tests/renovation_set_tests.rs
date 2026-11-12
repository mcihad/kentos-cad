//! Tadilat takımı (docs/adr/8039 §6; PAİY 4(1)(dddd), 58; Mimarlar Odası: "Projede mevcut, iptal ve ilaveler üç farklı
//! teknikle ifade edilmelidir"): Ruhsat takımı of a tadilat draws each storey three times, as it stands, with its
//! changes in the three techniques and as it will stand, each map linked to the set's named view of that state; the site
//! plan to Tadilat projesi at its own scale. EK-2's sequence counts the three on; made again, the views are renewed.

use kentos_commands::{Entity, PhaseView};
use kentos_native_application::permit_set::{self, PermitSet};
use kentos_sheet::permit::Phase;
use serde_json::{Value, json};

use crate::app::App;
use crate::permit_set_tests::{building, particulars, texts};

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

fn model(app: &App) -> &kentos_domain::Document {
    &app.document.as_ref().expect("a drawing").model
}

/// The map a sheet draws: the state and the scale its linked view gives, the storey its own.
fn map_of(app: &App, sheet: &str) -> kentos_sheet::display::MapPrim {
    let doc = model(app);
    let (book, _) =
        kentos_sheet::project::read_project_sheets(doc.sheets().expect("a book")).expect("reads");
    let inputs = kentos_plot::book::base_inputs(doc, "test", "2026-11-09");
    let list = kentos_sheet::display::display_list(&book, sheet, &inputs).expect("drawn");
    list.prims
        .into_iter()
        .find_map(|p| match p {
            kentos_sheet::display::Prim::Map(m) => Some(m),
            _ => None,
        })
        .expect("its map")
}

/// The ground storey's wall between the stair hall and the right flat pulled down (İptal), a new partition in the left
/// flat (İlave): a tadilat of the two-storey building. Its rooms say nothing of it yet.
fn tadilat(app: &mut App) {
    let doc = model(app);
    let wall = doc
        .entities()
        .find(|e| {
            e.own_level() == Some("zemin")
                && matches!(e, Entity::Wall(w) if w.wall.pts.iter().all(|p| (p.x - 10.0).abs() < 1e-9))
        })
        .and_then(|e| doc.uid(kentos_domain::Slot(e.base().id)))
        .expect("the wall at 10 m")
        .to_string();
    let layer = doc.layers().active().to_owned();
    run(
        app,
        "cad.walls.edit",
        json!({"uids": [wall], "operation": {"kind": "set", "phase": "demolish"}}),
    );
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    run(
        app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [{"pts": [p(0.125, 4.0), p(5.0, 4.0)], "width": 0.1,
            "arch": {"level": "zemin", "phase": "new"}}]}),
    );
}

/// The rooms the tadilat changes said so (docs/adr/8039 §6; docs/adr/0798 §3): the left flat's Salon, the stair hall
/// and the right flat's Salon İptal; in their places Yeni durum's rooms (İlave), bounded by the walls that stand after:
/// the left flat's Salon and Mutfak either side of the partition, the stair hall and the right Salon one room.
fn restated(app: &mut App) {
    let doc = model(app);
    let changed: Vec<String> = doc
        .entities()
        .filter(|e| e.own_level() == Some("zemin") && matches!(e, Entity::Space(_)))
        .filter_map(|e| doc.uid(kentos_domain::Slot(e.base().id)))
        .map(|u| u.to_string())
        .collect();
    assert_eq!(changed.len(), 3, "the ground storey's three rooms");
    let layer = doc.layers().active().to_owned();
    run(
        app,
        "cad.spaces.edit",
        json!({"uids": changed, "operation": {"kind": "set", "phase": "demolish"}}),
    );
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let room = |x: f64, y: f64, name: &str| json!({"seed": p(x, y), "arch": {"level": "zemin", "phase": "new"}, "name": name});
    run(
        app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            room(2.5, 2.0, "Salon"), room(2.5, 5.0, "Mutfak"), room(7.5, 3.0, "Salon")
        ]}),
    );
}

fn made_set(app: &mut App) -> Vec<kentos_sheet::permit::Made> {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let input = PermitSet {
        phase: Phase::Permit,
        values: particulars(),
        renovation: true,
        ..Default::default()
    };
    let (_, made, notes) = permit_set::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        &input,
    )
    .expect("made");
    assert!(notes.is_empty(), "{notes:?}");
    made
}

/// The project's named views' names.
fn view_names(app: &App) -> Vec<String> {
    model(app)
        .settings()
        .named_views
        .iter()
        .map(|v| v.name.clone())
        .collect()
}

#[test]
fn a_tadilat_set_draws_each_storey_as_it_stands_with_its_changes_and_as_it_will() {
    let mut app = building();
    assert!(
        !permit_set::renovates(model(&app)),
        "walls without a state: nothing removed or added"
    );
    tadilat(&mut app);
    assert!(permit_set::renovates(model(&app)));
    // Its rooms say nothing of it: these would keep on the sheets a wall that is not there (docs/adr/8039 §6).
    assert_eq!(
        permit_set::rooms_to_restate(model(&app)),
        ["Salon", "Merdiven evi", "Salon"]
    );
    restated(&mut app);
    assert_eq!(
        permit_set::rooms_to_restate(model(&app)),
        Vec::<String>::new()
    );
    let made = made_set(&mut app);
    let titles: Vec<String> = made
        .iter()
        .map(|m| format!("{} {}", m.number, m.title))
        .collect();
    assert_eq!(
        titles,
        [
            "UP-01 Proje bilgileri",
            "UP-02 Vaziyet planı",
            "UP-03 Zemin kat planı (mevcut durum)",
            "UP-04 Zemin kat planı (tadilat projesi)",
            "UP-05 Zemin kat planı (yeni durum)",
            "UP-06 1. kat planı (mevcut durum)",
            "UP-07 1. kat planı (tadilat projesi)",
            "UP-08 1. kat planı (yeni durum)",
        ]
    );
    // The set's views, written into the project: the three plans' at the phase's scale, the site plan's at its own.
    assert_eq!(
        view_names(&app),
        [
            "Ruhsat takımı · Mevcut durum",
            "Ruhsat takımı · Tadilat projesi",
            "Ruhsat takımı · Yeni durum",
            "Ruhsat takımı · Vaziyet planı",
        ]
    );
    // Each plan's map draws its storey as its view's state shows it, at 1/100; the site plan with the three techniques.
    let states = [
        PhaseView::Existing,
        PhaseView::Renovation,
        PhaseView::Proposed,
    ];
    for (k, m) in made.iter().enumerate().skip(1) {
        let map = map_of(&app, &m.sheet);
        let (phase, level, scale) = match k {
            1 => (PhaseView::Renovation, "zemin", None),
            2..=4 => (states[k - 2], "zemin", Some(100.0)),
            _ => (states[k - 5], "k1", Some(100.0)),
        };
        assert_eq!(map.phase, Some(phase), "{}", m.title);
        assert_eq!(
            map.level.as_ref().map(|l| l.id.as_str()),
            Some(level),
            "{}",
            m.title
        );
        if let Some(scale) = scale {
            assert!((map.view.scale - scale).abs() < 1e-9, "{}", m.title);
        } else {
            assert!(
                [500.0, 1000.0].contains(&map.view.scale),
                "the site plan's scale: {}",
                map.view.scale
            );
        }
    }
    // The title block: the sheet's state in its name, EK-2's sequence 01, 02, 03 on a storey.
    for (k, seq) in [(2, "01"), (3, "02"), (4, "03"), (7, "03")] {
        let sheet = texts(&app, &made[k].sheet);
        let place = if k < 5 { "K00" } else { "K01" };
        for want in [
            made[k].title.clone(),
            format!("260457_00_MM_XXXX_PL_MMC_{place}_{seq}_000"),
        ] {
            assert!(
                sheet.iter().any(|x| x.contains(&want)),
                "{want} on {}: {sheet:?}",
                made[k].title
            );
        }
    }
    // Published as PDF/A-2 files named by EK-2: each storey's three plans 01, 02, 03 on its code.
    let dir = std::env::temp_dir().join(format!("kentos-tadilat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let target = dir.join("Ruhsat-tadilat");
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
            "260457_00_MM_XXXX_PL_MMC_K00_02_000.pdf",
            "260457_00_MM_XXXX_PL_MMC_K00_03_000.pdf",
            "260457_00_MM_XXXX_PL_MMC_K01_01_000.pdf",
            "260457_00_MM_XXXX_PL_MMC_K01_02_000.pdf",
            "260457_00_MM_XXXX_PL_MMC_K01_03_000.pdf",
        ]
    );
    for n in &names {
        let bytes = std::fs::read(target.join(n)).expect("delivered");
        let report = kentos_formats::pdf::pdfa::pdfa2_report(&bytes).expect("read back");
        assert!(report.conforms(), "{n}: {:?}", report.findings);
    }
    let _ = std::fs::remove_dir_all(&dir);
    // Made again: the numbers go on, the views renewed in place, not added.
    let again = made_set(&mut app);
    assert_eq!(again[0].number, "UP-09");
    assert_eq!(view_names(&app).len(), 4);
    assert_eq!(
        map_of(&app, &again[3].sheet).phase,
        Some(PhaseView::Renovation)
    );
    // The set's views gone: the maps draw nothing rather than the drawing as it is shown (a link fails closed).
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut settings = doc.settings().clone();
    settings.named_views.clear();
    doc.set_settings(settings);
    let (book, _) =
        kentos_sheet::project::read_project_sheets(doc.sheets().expect("a book")).expect("reads");
    let inputs = kentos_plot::book::base_inputs(doc, "test", "2026-11-09");
    let list = kentos_sheet::display::display_list(&book, &made[3].sheet, &inputs).expect("drawn");
    assert!(
        !list
            .prims
            .iter()
            .any(|p| matches!(p, kentos_sheet::display::Prim::Map(_)))
    );
}

/// Ruhsat takımı's box (docs/adr/8039 §6): ticked when the drawing is a tadilat, the set said a tadilat's, its files
/// in a folder of their own.
#[test]
fn the_window_ticks_tadilat_projesi_for_a_tadilat_and_says_so() {
    use crate::exchange::Window;
    use crate::exchange::permit_set::Event;
    let mut app = building();
    let _ = app.permit_set_open();
    let ticked = |app: &App| match &app.exchange {
        Some(Window::PermitSet(s)) => s.renovation,
        _ => panic!("Ruhsat takımı is open"),
    };
    assert!(!ticked(&app), "a new building");
    app.close_exchange();
    tadilat(&mut app);
    let _ = app.permit_set_open();
    assert!(ticked(&app), "a tadilat");
    // The rooms the tadilat changes but that do not say so, said before the set is made.
    match &app.exchange {
        Some(Window::PermitSet(s)) => assert_eq!(s.restate, ["Salon", "Merdiven evi", "Salon"]),
        _ => panic!("Ruhsat takımı is open"),
    }
    let _ = app.permit_set_event(Event::Renovation(false));
    assert!(!ticked(&app));
    let _ = app.permit_set_event(Event::Renovation(true));
    let _ = app.permit_set_event(Event::Make);
    let said = crate::files_testing::last_said(&app);
    assert!(
        said.starts_with("Ruhsat takımı (tadilat projesi): 10 pafta (UP-01 … UP-10)"),
        "{said}"
    );
    // The rooms the tadilat changes but that do not say so, by name (docs/adr/8039 §6).
    assert!(
        said.contains(
            "Tadilatta değişen ama İptal yapılmamış mahaller: Salon, Merdiven evi, Salon."
        ),
        "{said}"
    );
    match &app.exchange {
        Some(Window::SheetPublish(p)) => {
            let folder = std::path::Path::new(&p.input.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            assert_eq!(
                folder,
                format!("Ruhsat-tadilat-{}", kentos_plot::book::today())
            );
            assert_eq!(p.rows.iter().filter(|r| r.selected).count(), 10);
        }
        _ => panic!("Paftaları toplu yayınla opens on the set"),
    }
}
