//! The sheet designer's work from the catalog (docs/adr/9305): every sheet command of the catalog runs here but
//! Karelaj, each as the designer's own message; while no sheet is in front it is off, Komut ara says what to do, and
//! each module's program offers them, Atlas only where maps are.

use kentos_launch::Module;
use kentos_sheet::kinds::ItemKind;
use kentos_sheet_ui::Message as Sheet;
use kentos_sheet_ui::questions::QuestionsMessage;

use super::COMMANDS;
use crate::app::{App, Message};
use crate::catalog::{PORTED, Standing, catalog};
use crate::files_testing::{app_with_drawing, last_said};

/// The sheet commands commands.rs ran before (no sheet in front needed).
const BEFORE: [&str; 4] = ["sheet.model", "sheet.drawingSet", "sheet.new", "sheet.fromTemplate"];
/// The web's sheet commands the designer cannot do yet: Karelaj puts a coordinate grid on the chosen map, and the
/// designer's Karelaj button has nothing behind it.
const WAITING: [&str; 1] = ["sheet.grid"];

/// Yeni pafta: the template's questions answered with its own values.
fn new_sheet(app: &mut App) {
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    if app.sheets.questions().is_some() {
        let _ = app.update(Message::Sheet(Sheet::Questions(QuestionsMessage::Make)));
    }
}

fn run(app: &mut App, id: &'static str) {
    let _ = app.update(Message::Run(id));
}

/// The chosen items, all of the sheet's that are not locked and in no group (Ctrl+A's).
fn choose_all(app: &mut App) -> usize {
    run(app, "sheet.selectAll");
    app.sheets.selection().len()
}

/// Every sheet command of the catalog runs on the desktop but those the designer cannot do yet (Karelaj); the ones
/// wired here are in PORTED in their order, after the four that ran before.
#[test]
fn every_sheet_command_runs_here_but_karelaj() {
    let waiting: Vec<&str> = catalog()
        .commands()
        .iter()
        .filter(|c| c.id.starts_with("sheet.") && c.standing != Standing::Ported)
        .map(|c| c.id)
        .collect();
    assert_eq!(waiting, WAITING);
    let wired: Vec<&str> = PORTED
        .iter()
        .copied()
        .filter(|id| id.starts_with("sheet.") && !BEFORE.contains(id))
        .collect();
    assert_eq!(wired, COMMANDS);
    for id in COMMANDS {
        let command = catalog().get(id).unwrap_or_else(|| panic!("{id} is a command"));
        assert_eq!(command.standing, Standing::Ported, "{id}");
        // The sheet mode takes its keys itself: none of them is the drawing's.
        assert!(command.shortcuts.is_empty(), "{id}: {:?}", command.shortcuts);
    }
}

/// Off while the project has no sheet, while the drawing is in front and while a map's paper is given over to the
/// model, each time saying what to do; the book's own commands (PAFTA, .kpafta, the paper's undo) do not need one in
/// front.
#[test]
fn a_command_on_the_paper_needs_a_sheet_in_front_and_says_how() {
    let mut app = app_with_drawing();
    let no_sheets = app.why_disabled("sheet.align.left").expect("off");
    assert!(no_sheets.starts_with("Projede pafta yok"), "{no_sheets}");
    assert!(!app.available("sheet.open"));
    assert!(app.available("sheet.importKpafta"));
    assert!(!app.available("sheet.undo"));
    run(&mut app, "sheet.zoomIn");
    assert_eq!(last_said(&app), no_sheets, "run anyway, it says why");

    new_sheet(&mut app);
    assert!(app.sheets.is_active());
    run(&mut app, "sheet.model");
    assert!(!app.sheets.is_active());
    let behind = app.why_disabled("sheet.zoomIn").expect("off");
    assert!(behind.starts_with("Önde pafta yok"), "{behind}");
    // Typed in the command line under the drawing, by its title: the same word.
    let _ = app.run_typed("Sola hizala");
    assert_eq!(last_said(&app), behind);
    assert!(app.available("sheet.undo"), "the new sheet is the paper's step");
    // PAFTA brings the first sheet in front.
    run(&mut app, "sheet.open");
    assert!(app.sheets.is_active());
    assert_eq!(app.why_disabled("sheet.zoomIn"), None);

    // A map's paper given over to the model: its paper's commands wait for Kâğıda dön.
    let map = app
        .sheets
        .open_sheet()
        .and_then(|s| s.items.iter().find(|i| matches!(i.kind, ItemKind::Map(_))))
        .map(|i| i.id.clone())
        .expect("the template's map");
    let _ = app.update(Message::Sheet(Sheet::EnterModel(map)));
    assert!(app.sheets.model_scope().is_some());
    let scope = app.why_disabled("sheet.zoomPage").expect("off");
    assert!(scope.contains("Kâğıda dön"), "{scope}");
    let _ = app.update(Message::Sheet(Sheet::ReturnPaper));
    assert_eq!(app.why_disabled("sheet.zoomPage"), None);
}

/// Each command does what the designer's own button, list, tree or key does, as one step of the paper's undo.
#[test]
fn each_command_is_the_designer_s_own_message() {
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    let count = |app: &App| app.sheets.open_sheet().map_or(0, |s| s.items.len());
    // The chosen items: off with none, one or two where each needs more.
    let _ = app.update(Message::Sheet(Sheet::Select(Vec::new())));
    for (id, why) in [
        ("sheet.align.left", "Önce paftada bir öğe seçin."),
        ("sheet.group", "En az iki öğe seçin."),
        ("sheet.distribute.hGaps", "En az üç öğe seçin."),
        ("sheet.renameItem", "Tek bir öğe seçin."),
        ("sheet.ungroup", "Önce bir grup seçin."),
        ("sheet.moveLeft", "Pafta zaten en solda."),
        ("sheet.moveRight", "Pafta zaten en sağda."),
    ] {
        assert_eq!(app.why_disabled(id).as_deref(), Some(why), "{id}");
        assert!(!app.available(id), "{id}");
    }
    let chosen = choose_all(&mut app);
    assert!(chosen >= 3, "the template's sheet has several free items: {chosen}");

    for (id, step) in [
        ("sheet.align.left", "Hizala"),
        ("sheet.alignTo.page", "Hizala"),
        ("sheet.distribute.hCenters", "Dağıt"),
        ("sheet.matchSize.width", "Aynı boy"),
        ("sheet.order.front", "Öne getir"),
    ] {
        run(&mut app, id);
        let label = app.sheets.undo_label().unwrap_or_default().to_owned();
        assert!(label.starts_with(step), "{id}: {label}");
        assert!(app.sheets.error().is_none(), "{id}: {:?}", app.sheets.error());
    }

    // Gizle ya da göster, Kilitle ya da aç: every chosen item, one step each way.
    let flags = |app: &App, lock: bool| -> Vec<bool> {
        let book = app.sheets.book();
        app.sheets
            .selection()
            .iter()
            .filter_map(|id| book.item(id))
            .map(|i| if lock { i.locked } else { i.hidden })
            .collect()
    };
    let step = |app: &App| app.sheets.undo_label().unwrap_or_default().to_owned();
    run(&mut app, "sheet.hideItems");
    assert!(flags(&app, false).iter().all(|h| *h));
    assert!(step(&app).starts_with("Gizle: "), "{}", step(&app));
    run(&mut app, "sheet.hideItems");
    assert!(flags(&app, false).iter().all(|h| !*h));
    assert!(step(&app).starts_with("Göster: "), "{}", step(&app));
    run(&mut app, "sheet.lockItems");
    assert!(step(&app).starts_with("Kilitle: "), "{}", step(&app));
    assert!(flags(&app, true).iter().all(|l| *l));
    assert_eq!(
        app.why_disabled("sheet.deleteItems").as_deref(),
        Some("Seçili öğeler kilitli: önce kilitlerini açın.")
    );
    run(&mut app, "sheet.lockItems");
    assert!(flags(&app, true).iter().all(|l| !*l));

    // Grupla and Grubu çöz; Çoğalt and Sil.
    let before = count(&app);
    run(&mut app, "sheet.group");
    assert_eq!(count(&app), before + 1, "a group item");
    assert!(app.available("sheet.ungroup"));
    run(&mut app, "sheet.ungroup");
    assert_eq!(count(&app), before);
    let _ = app.update(Message::Sheet(Sheet::Select(
        app.sheets.selection().first().cloned().into_iter().collect(),
    )));
    assert!(app.available("sheet.renameItem"));
    run(&mut app, "sheet.duplicateItems");
    assert_eq!(count(&app), before + 1);
    run(&mut app, "sheet.deleteItems");
    assert_eq!(count(&app), before);
    // The paper's undo and redo.
    run(&mut app, "sheet.undo");
    assert_eq!(count(&app), before + 1);
    run(&mut app, "sheet.redo");
    assert_eq!(count(&app), before);

    // The paper's tools, lit as they are in hand; Esc goes back to Seç, then lets the choice go.
    choose_all(&mut app);
    run(&mut app, "sheet.tool.hand");
    assert_eq!(app.checked("sheet.tool.hand"), Some(true));
    assert_eq!(app.checked("sheet.tool.select"), Some(false));
    run(&mut app, "sheet.escape");
    assert_eq!(app.checked("sheet.tool.select"), Some(true));
    assert!(!app.sheets.selection().is_empty());
    run(&mut app, "sheet.escape");
    assert!(app.sheets.selection().is_empty());

    // The view: a step in and out about the middle.
    run(&mut app, "sheet.zoomPage");
    let fitted = app.sheets.zoom_percent().expect("a sheet in front");
    run(&mut app, "sheet.zoomIn");
    assert!(app.sheets.zoom_percent().expect("zoomed") > fitted);
    run(&mut app, "sheet.zoomOut");
    run(&mut app, "sheet.zoomOut");
    assert!(app.sheets.zoom_percent().expect("zoomed") < fitted);

    // The sheets: Çoğalt (in front), Sola taşı, Sağa taşı and Sil, said, Geri al bringing it back.
    run(&mut app, "sheet.duplicate");
    let book = |app: &App| app.sheets.book().sheets.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
    let two = book(&app);
    assert_eq!(two.len(), 2);
    assert_eq!(app.sheets.open_sheet().map(|s| s.id.clone()), Some(two[1].clone()));
    run(&mut app, "sheet.moveLeft");
    assert_eq!(book(&app), [two[1].clone(), two[0].clone()]);
    run(&mut app, "sheet.moveRight");
    assert_eq!(book(&app), two);
    run(&mut app, "sheet.delete");
    assert_eq!(book(&app), [two[0].clone()]);
    assert!(last_said(&app).contains("Geri al ile geri gelir"), "{}", last_said(&app));
    // Kaydır has no direction: it says the arrow keys.
    choose_all(&mut app);
    run(&mut app, "sheet.nudge");
    assert!(last_said(&app).contains("ok tuşları"), "{}", last_said(&app));
    // No command the designer has is missing its handler.
    assert!(
        !app.log.lines().any(|l| l.text.contains("işleyicisi eksik")),
        "{:?}",
        app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
}

/// The windows and tabs: Sayfa ayarları and Ön denetim are the inspector's tabs, Değişkenler, Şablon olarak kaydet and
/// the PDF's export window open, and every command runs without a missing handler.
#[test]
fn the_windows_and_tabs_open_as_the_ribbon_opens_them() {
    use kentos_sheet_ui::InspectorTab;
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    run(&mut app, "sheet.pageSetup");
    assert_eq!(app.sheets.inspector_tab(), InspectorTab::Page);
    run(&mut app, "sheet.preflight");
    assert_eq!(app.sheets.inspector_tab(), InspectorTab::Preflight);
    for id in ["sheet.variables", "sheet.saveTemplate", "sheet.export.pdf", "sheet.print"] {
        run(&mut app, id);
        assert!(app.sheets.window(std::rc::Rc::new(app.sheet_painter())).is_some(), "{id} opens a window");
        let _ = app.update(Message::Sheet(Sheet::CloseDialog));
    }
    // Every command once, as Komut ara would run it where it can.
    for id in COMMANDS {
        if app.available(id) {
            run(&mut app, id);
            let _ = app.update(Message::Sheet(Sheet::CloseDialog));
        }
    }
    assert!(
        !app.log.lines().any(|l| l.text.contains("işleyicisi eksik")),
        "{:?}",
        app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
}

/// Komut ara finds a sheet command, dims it and says why while no sheet is in front, and runs it with one.
#[test]
fn komut_ara_runs_a_sheet_command_or_says_why_it_is_off() {
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    run(&mut app, "sheet.model");
    // The drawing's Yakınlaştır runs; the paper's is dimmed and says why.
    app.search_typed("Yakınlaştır".into());
    let rows: Vec<(bool, String)> = app
        .search_rows()
        .into_iter()
        .filter(|r| r.title == "Yakınlaştır")
        .map(|r| (r.enabled, r.detail.into_owned()))
        .collect();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(
        rows.iter().any(|(on, why)| !on && why.starts_with("Önde pafta yok")),
        "{rows:?}"
    );
    assert!(rows.iter().any(|(on, _)| *on), "{rows:?}");
    run(&mut app, "sheet.open");
    app.search_typed("Sola hizala".into());
    choose_all(&mut app);
    let found = app.search_rows();
    let (at, sola) = found
        .iter()
        .enumerate()
        .find(|(_, r)| r.title == "Sola hizala")
        .expect("Komut ara finds Sola hizala");
    assert!(sola.enabled, "{}", sola.detail);
    assert_eq!(sola.detail, "Pafta");
    let _ = app.search_run(at);
    assert!(app.sheets.undo_label().unwrap_or_default().starts_with("Hizala"));
}

/// The modules' decision: every module's program has sheets (its Çıktı › Pafta) and offers the designer's commands;
/// Atlas, a page for each object of a coverage layer, is KentOS Harita's and KentOS GIS's.
#[test]
fn each_module_offers_the_sheet_commands_and_atlas_where_maps_are() {
    for module in Module::ALL {
        for id in COMMANDS {
            let atlas = *id == "sheet.atlas";
            let expected = !atlas || matches!(module, Module::Map | Module::Gis);
            assert_eq!(catalog().module_has(module, id), expected, "{module:?} {id}");
        }
        assert!(catalog().module_has(module, "sheet.new"), "{module:?} makes sheets");
    }
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Architecture);
    let why = app.why_disabled("sheet.atlas").expect("not KentOS Mimari's");
    assert!(why.contains("Harita") && why.contains("GIS"), "{why}");
}

/// Pictures for the owner: Komut ara offering Sola hizala with a sheet in front, and the paper's Yakınlaştır dimmed
/// with its reason while the drawing is in front; light 1440 and dark 1100 wide,
/// `.run/gui-evidence/pafta-komutlari-9305/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheets::command_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "writes pictures"]
fn screens() {
    use iced::Size;
    use iced::futures::StreamExt as _;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/pafta-komutlari-9305");
    std::fs::create_dir_all(&out).expect("a folder");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let settle = |snapshot: &mut Snapshot, app: &mut App| {
        snapshot.settle(app, App::view, &mut |app: &mut App, m: Message| {
            let _ = app.update(m);
        });
    };
    // Alt+Q: the box takes the keyboard (its focus is a widget operation), then the text is typed.
    let search = |snapshot: &mut Snapshot, app: &mut App, text: &str| {
        settle(snapshot, app);
        let task = app.update(Message::Run("view.commandSearch"));
        if let Some(mut stream) = iced_runtime::task::into_stream(task) {
            while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                if let iced_runtime::Action::Widget(operation) = action {
                    snapshot.operate(app.view(), operation);
                }
            }
        }
        let _ = app.update(Message::RibbonSearch(text.into()));
    };
    for (theme, size, tag) in [
        ("light", Size::new(1440.0, 900.0), "acik-1440"),
        ("dark", Size::new(1100.0, 760.0), "koyu-1100"),
    ] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        // The sheet's map looks at the drawing: a sheet is made where the drawing area looks.
        if let Some(b) = app.document.as_ref().and_then(kentos_render_wgpu::scene::extents) {
            app.viewport.camera.center =
                kentos_render_wgpu::Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let shot = |snapshot: &mut Snapshot, app: &mut App, name: &str| {
            settle(snapshot, app);
            let file = out.join(format!("{name}-{tag}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        new_sheet(&mut app);
        choose_all(&mut app);
        search(&mut snapshot, &mut app, "hizala");
        shot(&mut snapshot, &mut app, "1-pafta-onde-hizala");
        run(&mut app, "sheet.model");
        search(&mut snapshot, &mut app, "Yakınlaştır");
        shot(&mut snapshot, &mut app, "2-cizim-onde-yakinlastir");
    }
}
