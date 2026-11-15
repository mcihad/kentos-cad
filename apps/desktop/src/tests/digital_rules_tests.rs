//! Dijital proje kuralları (docs/adr/8062; `arch.digitalRules`, Mimari › Proje ▾): the window lists the regulation's
//! rule catalog with every row's annex, table and page, narrows it, and says KentOS's decisions and the catalog's
//! differences from the IFC 4.3 schema, each settled.

use crate::app::{App, Dialog, Message};
use crate::arch;
use crate::digital_rules::{Asked, Event, Listed};

fn open() -> App {
    let mut app = crate::permit_set_tests::building();
    let _ = app.update(Message::Run("arch.digitalRules"));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Arch(arch::Event::DigitalRules(e)));
}

#[test]
fn the_window_lists_the_architectural_model_s_rules_and_narrows_them() {
    let mut app = open();
    assert_eq!(app.dialog, Some(Dialog::Arch(arch::ArchDialogKind::DigitalRules)));
    let w = app.arch.digital_rules.as_ref().expect("the window");
    assert!(w.filter.architecture, "the architectural model's rules first");
    let (all, shown) = (w.rules.len(), w.shown.len());
    assert!(shown > 300 && shown < all, "{shown} of {all}");
    // EK-6, mandatory, “duvar yangın”: the wall's fire rating alone, with its page.
    event(&mut app, Event::Annex(6));
    event(&mut app, Event::Asked(Asked::Mandatory));
    event(&mut app, Event::Words("duvar yangın".into()));
    let rows = app.arch.digital_rules.as_ref().unwrap().rows();
    assert_eq!(
        rows,
        [[
            "Yangın Dayanım Sınıfı",
            "Duvar (IfcWall)",
            "Pset_WallCommon.FireRating",
            "Zorunlu",
            "EK-6 Tablo 6.66 satır 6 · PDF s. 107"
        ]]
    );
    event(&mut app, Event::Choose(0));
    assert_eq!(app.arch.digital_rules.as_ref().unwrap().chosen, Some(0));
    // Every rule, not only the architectural model's.
    event(&mut app, Event::Words(String::new()));
    event(&mut app, Event::Architecture(false));
    let more = app.arch.digital_rules.as_ref().unwrap().shown.len();
    event(&mut app, Event::Architecture(true));
    let fewer = app.arch.digital_rules.as_ref().unwrap().shown.len();
    assert!(more > fewer, "{more} > {fewer}");
}

#[test]
fn decisions_and_schema_differences_are_listed_each_settled() {
    let mut app = open();
    event(&mut app, Event::List(Listed::Decisions));
    let w = app.arch.digital_rules.as_ref().unwrap();
    let rows = w.rows();
    assert_eq!(rows.len(), w.decisions.len());
    let pdfa = rows.iter().find(|r| r[0] == "K13").expect("the PDF/A part's decision");
    assert_eq!(pdfa[3], "açık (KentOS: PDF/A-2b)", "an open point keeps no value");
    assert!(w.open_decisions() > 0);
    event(&mut app, Event::List(Listed::Findings));
    let rows = app.arch.digital_rules.as_ref().unwrap().rows();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r[2] != "karar yok"), "{rows:?}");
    // The filter narrows the differences too: Tablo 6.92's misspelled class, cited with its page.
    event(&mut app, Event::Words("distrubition".into()));
    let rows = app.arch.digital_rules.as_ref().unwrap().rows();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0][1], "EK-6 Tablo 6.92 · PDF s. 150–151");
    assert_eq!(rows[0][2], "K30");
    event(&mut app, Event::List(Listed::Decisions));
    event(&mut app, Event::Words("datum".into()));
    let rows = app.arch.digital_rules.as_ref().unwrap().rows();
    assert_eq!(rows.iter().map(|r| r[0].as_str()).collect::<Vec<_>>(), ["K11"]);
    event(&mut app, Event::Close);
    assert!(app.dialog.is_none() && app.arch.digital_rules.is_none());
}

/// The window and its place on the ribbon, light and dark; `.run/gui-evidence/dijital-kurallar-8062/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib digital_rules_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dijital-kurallar-8062");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1600.0, 1000.0), "acik"), ("dark", Size::new(1440.0, 900.0), "koyu")] {
        let mut app = crate::permit_set_tests::building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |snapshot: &mut Snapshot, app: &mut App, file: String| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(file);
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        // Its place: Mimari › Proje ▾, after Model kalite kontrol formu.
        let _ = app.update(Message::RibbonTab("arch"));
        shot(&mut snapshot, &mut app, format!("1-serit-{name}.png"));
        crate::tools_screens::open_more(&mut snapshot, &mut app, "Proje");
        shot(&mut snapshot, &mut app, format!("2-proje-menusu-{name}.png"));
        crate::tools_screens::press_caption(&mut snapshot, &mut app, "Dijital proje kuralları", false, false);
        assert_eq!(app.dialog, Some(Dialog::Arch(arch::ArchDialogKind::DigitalRules)), "the menu's item opens it");
        shot(&mut snapshot, &mut app, format!("3-kurallar-{name}.png"));
        event(&mut app, Event::Annex(6));
        event(&mut app, Event::Asked(Asked::Mandatory));
        event(&mut app, Event::Words("ifcwall".into()));
        let fire = app
            .arch
            .digital_rules
            .as_ref()
            .and_then(|w| w.rows().iter().position(|r| r[2] == "Pset_WallCommon.FireRating"))
            .expect("the wall's fire rating");
        event(&mut app, Event::Choose(fire));
        shot(&mut snapshot, &mut app, format!("4-duvar-{name}.png"));
        // Every decision and every difference: the filter cleared.
        event(&mut app, Event::Annex(0));
        event(&mut app, Event::Asked(Asked::All));
        event(&mut app, Event::Words(String::new()));
        event(&mut app, Event::List(Listed::Decisions));
        let k13 = app
            .arch
            .digital_rules
            .as_ref()
            .and_then(|w| w.decisions.iter().position(|d| d.id == "K13"))
            .unwrap_or(0);
        event(&mut app, Event::Choose(k13));
        shot(&mut snapshot, &mut app, format!("5-kararlar-{name}.png"));
        event(&mut app, Event::List(Listed::Findings));
        event(&mut app, Event::Choose(0));
        shot(&mut snapshot, &mut app, format!("6-sema-farklari-{name}.png"));
    }
}
