//! Harita işi (map_work_panel.rs, plan/map-cad.md MAP-01, docs/adr/1200): in one map project a GNSS campaign, an
//! orthophoto, a CAD reference and a design surface; every selected object's source is found in Öznitelikler; the
//! campaign relinked from one place leaves its derivations out of date, undo brings them back; an item added from the
//! panel and approved; saved and opened again (and read by someone who only reads) the relation is the same; KentOS
//! Mimari and KentOS GIS show neither the panel nor the rows. And its pictures.

use kentos_commands::{WorkObjectRole, WorkState as S, WorkStatus as W};
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message, Panel};
use crate::map_work_panel::Event as P;

/// The fixtures' folder, absolute.
fn fixtures() -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .canonicalize()
        .expect("the fixtures")
        .to_string_lossy()
        .into_owned()
}

/// `map.work.query`'s drawing (map_work_cases.py): the campaign, the two orthophotos, the CAD reference held by a
/// group, the design surface and the free drawing, its paths made absolute.
fn snapshot() -> kentos_commands::DocumentSnapshotV2 {
    let text = include_str!("../../../../fixtures/commands/v1/map.work.query.json")
        .replace("$fixture:", &format!("{}/", fixtures()));
    let cases: Value = serde_json::from_str(&text).expect("cases");
    serde_json::from_value(cases["setup"].clone()).expect("the drawing reads")
}

fn opened(snapshot: kentos_commands::DocumentSnapshotV2, program: Option<Module>) -> App {
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::MapWork(e));
}

fn tree(app: &App) -> kentos_commands::MapWorkTree {
    app.map_work_read().expect("a drawing").expect("a map project")
}

/// Öznitelikler's Kaynağı rows of the object in `slot`, selected alone.
fn source_rows(app: &mut App, slot: u32) -> Vec<(String, String)> {
    app.selection.set(vec![Slot(slot)]);
    let doc = app.document.as_ref().expect("a drawing");
    let panel = app.properties_panel(doc);
    panel
        .sections
        .iter()
        .find(|s| s.id == "mapWork")
        .map(|s| s.rows.iter().map(|r| (r.label.to_string(), r.value.clone())).collect())
        .unwrap_or_default()
}

fn row<'a>(rows: &'a [(String, String)], label: &str) -> Option<&'a str> {
    rows.iter().find(|(l, _)| l == label).map(|(_, v)| v.as_str())
}

#[test]
fn every_objects_source_is_found_and_follows_its_source() {
    let mut app = opened(snapshot(), Some(Module::Map));
    assert!(app.shows_panel(Panel::MapWork));
    let t = tree(&app);
    assert_eq!(t.items.len(), 5);
    assert_eq!((t.stale, t.problems), (0, 0), "{t:?}");
    let kinds: Vec<_> = t.items.iter().map(|i| i.kind).collect();
    use kentos_commands::WorkKind as K;
    assert_eq!(kinds, [K::Campaign, K::Reference, K::Reference, K::Design, K::Reference]);
    // The GNSS point: derived from the campaign by its import; the orthophoto's raster and the CAD reference's line
    // (under the reference's group): source data; the design surface's line: derived; the free line: the user's own.
    let g1 = source_rows(&mut app, 1);
    assert_eq!(row(&g1, "Rol"), Some("Kaynaktan türetildi"));
    assert_eq!(row(&g1, "Kaynağı"), Some("GNSS ölçümü 2026-10 (is-1)"));
    assert_eq!(row(&g1, "Üretim işlemi"), Some("GNSS içe aktarma"));
    assert_eq!(row(&g1, "Güncellik"), Some("Güncel"));
    assert_eq!(row(&g1, "Kaynak revizyonu").map(str::len), Some(12));
    assert_eq!(row(&source_rows(&mut app, 4), "Rol"), Some("Kaynak veri"));
    let a1 = source_rows(&mut app, 5);
    assert_eq!(row(&a1, "İş öğesi"), Some("Kadastro CAD altlığı (is-3)"));
    let t1 = source_rows(&mut app, 6);
    assert_eq!(row(&t1, "İş öğesi"), Some("Tasarım yüzeyi (is-4)"));
    assert_eq!(row(&t1, "Rol"), Some("Kaynaktan türetildi"));
    assert_eq!(row(&source_rows(&mut app, 8), "Rol"), Some("Bağımsız çizim"));
    // The campaign's log moved and grew: relinked from the panel, its two derivations are out of date, shown in the
    // tree and in the properties; one undo step takes the relink back.
    let later = format!("{}/map-work/v1/kampanya-2026-11.nmea", fixtures());
    send(&mut app, P::Relinked("is-1".into(), Some(later.clone().into())));
    let t = tree(&app);
    assert_eq!(t.stale, 2);
    assert!(t.derivations.iter().all(|d| d.state == S::Stale), "{t:?}");
    assert_eq!(t.items[0].file.as_ref().map(|f| f.path.as_str()), Some(later.as_str()));
    assert_eq!(row(&source_rows(&mut app, 1), "Güncellik"), Some("Güncel değil: kaynağı değişti"));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(tree(&app).stale, 0, "undone, up to date again");
    // A design item added from the panel with the layer chosen in Katmanlar, sent to review and approved (the office's
    // record: the tree says approved, nothing more).
    app.selected_layer = Some("bos".into());
    send(&mut app, P::Kind("design"));
    send(&mut app, P::Name("Kot tasarımı".into()));
    send(&mut app, P::Add(false));
    let t = tree(&app);
    let added = t.items.iter().find(|i| i.name == "Kot tasarımı").expect("added");
    assert_eq!((added.id.as_str(), added.layers.as_slice()), ("is-6", ["bos".to_owned()].as_slice()));
    send(&mut app, P::Status("is-6".into(), W::Review));
    send(&mut app, P::Status("is-6".into(), W::Approved));
    assert_eq!(tree(&app).items[5].status, W::Approved);
    // An item's row selects its layers' objects; a derivation's its objects.
    send(&mut app, P::Item("is-1".into()));
    assert_eq!(app.selection.ids().len(), 3);
    send(&mut app, P::Derivation("tr-2".into()));
    assert_eq!(app.selection.ids(), [Slot(6), Slot(7)]);
}

#[test]
fn saved_and_opened_again_the_relation_is_the_same_for_a_reader_too() {
    let mut app = opened(snapshot(), Some(Module::Map));
    let later = format!("{}/map-work/v1/kampanya-2026-11.nmea", fixtures());
    send(&mut app, P::Relinked("is-1".into(), Some(later.into())));
    let before = tree(&app);
    let model = &app.document.as_ref().expect("a drawing").model;
    let bytes = kentos_kcad::encode(&model.to_snapshot_v2()).expect("encodes");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(back.settings.map_work, model.map_work().cloned(), "the work structure is saved whole");
    // Opened again: the same items, derivations and object source; reading changes nothing (a reader only reads).
    let mut again = opened(back, Some(Module::Map));
    let after = tree(&again);
    assert_eq!(after.items, before.items);
    assert_eq!(after.derivations, before.derivations);
    let doc = &again.document.as_ref().expect("a drawing").model;
    let (revision, dirty) = (doc.revision(), doc.is_dirty());
    let found = kentos_native_application::map_work::source_of(doc, &doc.uid(Slot(1)).expect("G1").to_string())
        .expect("G1's source");
    assert_eq!((found.role, found.current), (WorkObjectRole::Derived, Some(false)));
    send(&mut again, P::Deep(true));
    let _ = tree(&again);
    let doc = &again.document.as_ref().expect("a drawing").model;
    assert_eq!((doc.revision(), doc.is_dirty()), (revision, dirty), "reading writes nothing");
}

#[test]
fn mimari_and_gis_do_not_show_the_map_work() {
    for module in [Module::Architecture, Module::Gis, Module::Road] {
        let mut app = opened(snapshot(), Some(module));
        assert!(!app.shows_panel(Panel::MapWork), "{module:?}");
        assert!(!app.shows_property_rows(crate::module_surfaces::PropertyRows::MapWork), "{module:?}");
        assert!(source_rows(&mut app, 1).is_empty(), "{module:?}");
    }
}

/// The panel in the dock, wide and light, narrow and dark: the five items, the campaign relinked to its later log
/// (two derivations out of date), the GNSS point's Kaynağı; `.run/gui-evidence/harita-isi-*`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_work_panel_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened(snapshot(), Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapWork));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("harita-isi-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        // Every heading open for the picture (a quiet branch starts folded).
        for key in ["kind:campaign", "kind:reference", "kind:design", "item:is-1"] {
            send(&mut app, P::Fold(key.into()));
        }
        app.selection.set(vec![Slot(1)]);
        shot(&mut app, "guncel");
        let later = format!("{}/map-work/v1/kampanya-2026-11.nmea", fixtures());
        send(&mut app, P::Relinked("is-1".into(), Some(later.into())));
        // The campaign's branch now wants attention and starts open: the user's flips taken back.
        for key in ["kind:campaign", "item:is-1"] {
            send(&mut app, P::Fold(key.into()));
        }
        app.selection.set(vec![Slot(1)]);
        shot(&mut app, "kaynak-degisti");
    }
}
