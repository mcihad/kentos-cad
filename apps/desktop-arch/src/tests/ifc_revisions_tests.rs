//! IFC revizyonlarını karşılaştır and BCF sorunları (docs/adr/8103; plan/ifc-uyum.md IFC-10) worked by hand: the
//! section building's two revisions (KentOS's file and the one IfcOpenShell changed: an interior wall moved 0.50 m
//! north, the east wall REI 60 → REI 90) compared in the window; both rows marked and written as issues; the file read
//! back in BCF sorunları, whose Çizimde seç finds the drawing's wall the moved wall's GlobalId stands for (the shared
//! cases' drawing holds it: fixtures/commands/v1/cad.bcf.read.json).
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_revisions_tests::screens -- --ignored --nocapture
//! ```

use crate::arch::ArchState;
use std::path::PathBuf;

use kentos_launch::Module;

use crate::app::{App, Message, Picker};
use crate::bcf_window::Event as Bcf;
use crate::files_testing::{drive, scratch};
use crate::ifc_compare_window::{Event, Outcome, Side, Tab};
use crate::recovery::Recovery;
use crate::settings::Settings;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.bcf.read.json");

fn fixture(rel: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(rel)
        .canonicalize()
        .expect(rel)
        .display()
        .to_string()
}

fn send(app: &mut App, e: Event) {
    let task = app.update(Message::from(crate::arch::Event::IfcCompare(e)));
    drive(app, task);
}

fn bcf(app: &mut App, e: Bcf) {
    let task = app.update(Message::from(crate::arch::Event::Bcf(e)));
    drive(app, task);
}

/// The BCF cases' drawing: one wall, whose persistent id KentOS writes as the moved wall's GlobalId.
fn opened() -> App {
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
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("the cases read");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The two revisions compared in the window.
fn compared(app: &mut App) {
    let _ = app.update(Message::Run("arch.ifcCompare"));
    send(
        app,
        Event::Path(Side::Older, fixture("arch/v1/ifc/kesit-binasi-tam.ifc")),
    );
    send(
        app,
        Event::Path(Side::Newer, fixture("ifc/v1/revizyon/kesit-binasi-r2.ifc")),
    );
    send(app, Event::Compare);
}

/// A comparison's answer is its own run's (docs/adr/9401 §5): the answer of a run started before Karşılaştır was
/// pressed again does not write over the newer run's.
#[test]
fn an_earlier_comparison_s_answer_does_not_write_over_a_newer_one() {
    let mut app = opened();
    let _ = app.update(Message::Run("arch.ifcCompare"));
    send(
        &mut app,
        Event::Path(Side::Older, fixture("arch/v1/ifc/kesit-binasi-tam.ifc")),
    );
    send(
        &mut app,
        Event::Path(Side::Newer, fixture("ifc/v1/revizyon/kesit-binasi-r2.ifc")),
    );
    // The first run's answer is held back.
    let earlier = app.update(Message::from(crate::arch::Event::IfcCompare(
        Event::Compare,
    )));
    // Another file, compared again: this run is refused (there is no such file).
    send(
        &mut app,
        Event::Path(Side::Newer, "boyle-bir-dosya-yok.ifc".into()),
    );
    send(&mut app, Event::Compare);
    let outcome = |app: &App| {
        let w = app.modules.arch().ifc_compare_window.as_ref();
        format!("{:?}", w.expect("the window").outcome)
    };
    let newer = outcome(&app);
    assert!(newer.contains("Refused"), "{newer}");
    drive(&mut app, earlier);
    assert_eq!(
        outcome(&app),
        newer,
        "the earlier run's answer is not the window's"
    );
}

/// Oku reads the BCF and its model on a thread of its own (docs/adr/9401 §6): the click answers before the file is
/// read, and the window says it is reading until the answer comes.
#[test]
fn the_issues_are_read_off_the_interface_thread() {
    let mut app = opened();
    let _ = app.update(Message::Run("arch.bcfIssues"));
    bcf(
        &mut app,
        Bcf::Path(
            crate::bcf_window::Field::Bcf,
            fixture("ifc/v1/bcf/disaridan-v3.bcf"),
        ),
    );
    let reading = app.update(Message::from(crate::arch::Event::Bcf(Bcf::Read)));
    let w = app.modules.arch().bcf_window.as_ref().expect("the window");
    assert!(
        w.read.is_none() && w.reading(),
        "the click does not wait for the file: {:?}",
        w.read.as_ref().map(|r| r.is_ok())
    );
    drive(&mut app, reading);
    let w = app.modules.arch().bcf_window.as_ref().expect("the window");
    assert!(
        matches!(&w.read, Some(Ok(r)) if !r.topics.is_empty()) && !w.reading(),
        "{:?}",
        w.read.as_ref().map(|r| r.as_ref().map(|o| o.topics.len()))
    );
}

#[test]
fn the_revisions_are_compared_written_as_issues_and_read_back_to_the_drawing() {
    let mut app = opened();
    compared(&mut app);
    let w = app
        .modules
        .arch()
        .ifc_compare_window
        .as_ref()
        .expect("the window");
    let Some(Outcome::Done(c)) = &w.outcome else {
        panic!("{:?}", w.outcome)
    };
    assert_eq!(w.tab, Tab::Changed);
    let summary = crate::ifc_compare_window::summary(c);
    assert!(
        summary.starts_with("2 nesne değişti · 0 eklendi · 0 silindi"),
        "{summary}"
    );
    let words: Vec<(String, String)> = c
        .changed
        .iter()
        .flat_map(|x| {
            x.changes
                .iter()
                .map(crate::ifc_compare_window::change_words)
        })
        .collect();
    assert_eq!(
        words[0],
        (
            "Pset_WallCommon.FireRating".to_owned(),
            "REI 60 → REI 90".to_owned()
        )
    );
    assert_eq!(
        words[1],
        (
            "Taşındı".to_owned(),
            "0,500 m (x 0,000 · y 0,500 · z 0,000)".to_owned()
        )
    );

    // Both rows marked and written as issues.
    send(&mut app, Event::Mark(0, true));
    send(&mut app, Event::Mark(1, true));
    send(&mut app, Event::Author("mimar@buro".into()));
    let path = scratch("bcf").join("sorunlar.bcf");
    app.picker = Picker::File(path.clone());
    send(&mut app, Event::Write);
    let said = app
        .modules
        .arch()
        .ifc_compare_window
        .as_ref()
        .unwrap()
        .said
        .clone()
        .expect("said");
    assert!(said.1.starts_with("2 sorun"), "{said:?}");

    // Read back: the moved wall's topic finds the drawing's wall.
    let _ = app.update(Message::Run("arch.bcfIssues"));
    bcf(
        &mut app,
        Bcf::Path(crate::bcf_window::Field::Bcf, path.display().to_string()),
    );
    bcf(
        &mut app,
        Bcf::Path(
            crate::bcf_window::Field::Model,
            fixture("ifc/v1/revizyon/kesit-binasi-r2.ifc"),
        ),
    );
    bcf(&mut app, Bcf::Read);
    let w = app.modules.arch().bcf_window.as_ref().expect("the window");
    let Some(Ok(r)) = &w.read else {
        panic!("{:?}", w.read)
    };
    assert_eq!(r.topics.len(), 2);
    let moved = r
        .topics
        .iter()
        .position(|t| t.title.contains("Taşındı"))
        .expect("the moved wall's topic");
    bcf(&mut app, Bcf::Choose(moved));
    bcf(&mut app, Bcf::Select);
    let doc = app.document.as_ref().expect("a drawing");
    let chosen: Vec<String> = app
        .selection
        .ids()
        .iter()
        .filter_map(|s| doc.model.uid(*s))
        .map(|u| u.to_string())
        .collect();
    assert_eq!(chosen, ["0192f5a1-4242-7000-8000-000000000007"]);
}

#[test]
fn nothing_marked_or_no_author_is_said_before_any_file() {
    let mut app = opened();
    compared(&mut app);
    send(&mut app, Event::Write);
    let said = app
        .modules
        .arch()
        .ifc_compare_window
        .as_ref()
        .unwrap()
        .said
        .clone()
        .expect("said");
    assert!(said.1.contains("işaretleyin"), "{said:?}");
    send(&mut app, Event::Mark(0, true));
    send(&mut app, Event::Author(" ".into()));
    send(&mut app, Event::Write);
    let said = app
        .modules
        .arch()
        .ifc_compare_window
        .as_ref()
        .unwrap()
        .said
        .clone()
        .expect("said");
    assert!(said.1.contains("Yazarı"), "{said:?}");
}

/// The windows' pictures (`.run/gui-evidence/ifc-revizyon-8103/`), light and dark: the comparison with the moved wall
/// chosen; the issues read back with the moved wall's topic; their place on the ribbon.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ifc-revizyon-8103");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        let mut app = opened();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let shot = |app: &mut App, snapshot: &mut Snapshot, file: &str| {
            app.toasts.clear();
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        compared(&mut app);
        send(&mut app, Event::Mark(0, true));
        send(&mut app, Event::Mark(1, true));
        send(&mut app, Event::Choose(1));
        send(&mut app, Event::Author("mimar@buro".into()));
        shot(&mut app, &mut snapshot, "1-karsilastirma");
        let path = scratch("bcf-resim").join("sorunlar.bcf");
        app.picker = Picker::File(path.clone());
        send(&mut app, Event::Write);
        shot(&mut app, &mut snapshot, "2-bcf-yazildi");
        send(&mut app, Event::Close);
        let _ = app.update(Message::Run("arch.bcfIssues"));
        bcf(
            &mut app,
            Bcf::Path(
                crate::bcf_window::Field::Bcf,
                fixture("ifc/v1/bcf/disaridan-v3.bcf"),
            ),
        );
        bcf(
            &mut app,
            Bcf::Path(
                crate::bcf_window::Field::Model,
                fixture("ifc/v1/revizyon/kesit-binasi-r2.ifc"),
            ),
        );
        bcf(&mut app, Bcf::Read);
        let r = app
            .modules
            .arch()
            .bcf_window
            .as_ref()
            .and_then(|w| w.read.clone())
            .and_then(Result::ok)
            .expect("read");
        let moved = r
            .topics
            .iter()
            .position(|t| t.title.contains("kuzey"))
            .unwrap_or(0);
        bcf(&mut app, Bcf::Choose(moved));
        bcf(&mut app, Bcf::Select);
        shot(&mut app, &mut snapshot, "3-bcf-sorunlari");
        bcf(&mut app, Bcf::Close);
        // Their place on the ribbon: Mimari › Proje ▾.
        let _ = app.update(Message::RibbonSearch(
            "IFC revizyonlarını karşılaştır".into(),
        ));
        let _ = app.update(Message::RibbonSearchReveal(0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Click(iced::Point::new(1278.0, 133.0)),
        );
        shot(&mut app, &mut snapshot, "4-serit-yeri");
    }
}
