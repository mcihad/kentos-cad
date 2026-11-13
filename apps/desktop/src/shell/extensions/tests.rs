//! Eklentiler in the window (docs/adr/8057 §3): a package reviewed and installed through the manager, a vertical
//! extension's tab in its own program only, gone without a ghost when the extension stops, an update's new
//! capability shown before it is granted.

use std::path::{Path, PathBuf};

use kentos_extension_host::{ContributionKind, State};
use kentos_extensions::manifest::Capability;
use kentos_launch::Module;

use super::ribbon::Action;
use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-eklentiler-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A golden fixture folder packed, its manifest changed by `edit`.
fn package(name: &str, edit: impl Fn(&mut serde_json::Value)) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/extensions/v1")
        .join(name);
    let mut files = kentos_extension_host::store::read_files(&root).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    edit(&mut manifest);
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

fn app(root: &Path, program: Option<Module>) -> App {
    let (mut app, _) = App::boot(None);
    app.program = program;
    app.extensions = Extensions::open(root, false).0;
    app.extensions_changed();
    app
}

/// The manager's flow: the file picked, reviewed, installed with everything granted.
fn install(app: &mut App, file: &str, bytes: Vec<u8>) {
    app.open_extensions();
    let _ = app.extensions_event(Event::Picked(Some((file.to_owned(), bytes))));
    let review = app.extensions.review.as_ref().expect("reviewed");
    assert!(
        review.review.inspection.is_valid(),
        "{:#?}",
        review.review.inspection.problems
    );
    let _ = app.extensions_event(Event::Install);
    assert!(app.extensions.review.is_none(), "{:?}", app.extensions.note);
}

const KENT: &str = "tr.ornek.kent-rehberi";

#[test]
fn a_vertical_extensions_tab_shows_in_its_own_program_only() {
    let root = temp("sekme");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "kent-rehberi.kek", package("ornek-kent", |_| {}));
    assert_eq!(
        gis.extensions.note.as_ref().map(|(e, _)| *e),
        Some(false),
        "{:?}",
        gis.extensions.note
    );
    let tab = gis.extension_tabs().next().expect("the tab in KentOS GIS");
    assert_eq!(
        (tab.id, tab.label.as_str()),
        ("ext.tr.ornek.kent-rehberi.kent", "Kent Rehberi")
    );
    let actions: Vec<&Action> = tab
        .panels
        .iter()
        .flat_map(|p| p.items.iter().map(|i| &i.action))
        .collect();
    assert_eq!(
        actions,
        [
            &Action::Source(KENT.into(), "ortofoto".into()),
            &Action::Form(KENT.into(), "bina-denetimi".into()),
            &Action::Builtin("data.featureTable"),
        ]
    );
    assert_eq!(gis.extension_groups(tab).len(), 3);
    // Chosen, the tab stays chosen; the ribbon draws it.
    let id = tab.id;
    gis.choose_tab(id);
    assert_eq!(gis.regular_tab(), "ext.tr.ornek.kent-rehberi.kent");
    let _ = gis.ribbon(false);
    // The same registry in KentOS Mimari: no tab.
    let mimari = app(&root, Some(Module::Architecture));
    assert_eq!(mimari.extension_tabs().count(), 0);
    assert!(
        mimari
            .extension_tab("ext.tr.ornek.kent-rehberi.kent")
            .is_none()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_disabled_extension_leaves_no_tab_and_no_button_behind() {
    let root = temp("hayalet");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "kent-rehberi.kek", package("ornek-kent", |_| {}));
    let id = gis.extension_tabs().next().unwrap().id;
    gis.choose_tab(id);
    let _ = gis.extensions_event(Event::Disable(KENT.into()));
    assert_eq!(gis.extension_tabs().count(), 0);
    assert_ne!(
        gis.regular_tab(),
        id,
        "the chosen tab falls back to the program's own"
    );
    let registry = gis.extensions.registry.as_ref().unwrap();
    assert!(registry.contributions().is_empty());
    assert_eq!(registry.state(KENT), Some(State::Disabled));
    let _ = gis.ribbon(false);
    let _ = gis.extensions_event(Event::Enable(KENT.into()));
    assert_eq!(gis.extension_tabs().count(), 1);
    // Removed: nothing left either.
    let _ = gis.extensions_event(Event::Remove(KENT.into()));
    assert_eq!(gis.extension_tabs().count(), 0);
    assert!(
        gis.extensions
            .registry
            .as_ref()
            .unwrap()
            .extensions()
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_updates_new_capability_is_shown_and_left_out_unless_granted() {
    let root = temp("guncelleme");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "kent-rehberi.kek", package("ornek-kent", |_| {}));
    let newer = package("ornek-kent", |m| {
        m["version"] = "1.1.0".into();
        m["contributions"]["contextMenus"] = serde_json::json!([{ "builtin": "data.featureTable", "label": "Öznitelik tablosu", "target": "layer" }]);
        m["requestedCapabilities"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "ui.contextMenu", "scope": "plugin-owned", "reason": "Katmanın sağ tık menüsüne öznitelik tablosunu ekler."
        }));
    });
    let _ = gis.extensions_event(Event::Picked(Some((
        "kent-rehberi-1.1.0.kek".into(),
        newer,
    ))));
    let pending = gis.extensions.review.as_ref().unwrap();
    let line = pending
        .review
        .capabilities
        .iter()
        .find(|c| c.capability == Capability::UiContextMenu)
        .unwrap();
    assert_eq!(line.change, kentos_extension_host::Change::Added);
    assert_eq!(
        pending.review.installed.as_ref().map(|(v, _)| v.as_str()),
        Some("1.0.0")
    );
    let _ = gis
        .extension_review_window()
        .expect("the review over the manager");
    // The user leaves the new capability out: the update installs, the menu item does not register.
    let _ = gis.extensions_event(Event::Grant(Capability::UiContextMenu, false));
    let _ = gis.extensions_event(Event::Install);
    let registry = gis.extensions.registry.as_ref().unwrap();
    assert_eq!(registry.lease(KENT).unwrap().version, "1.1.0");
    assert!(
        !registry
            .contributions()
            .iter()
            .any(|c| c.kind == ContributionKind::ContextMenuItem)
    );
    assert!(gis.extensions.menus.is_empty(), "not granted: no menu item");
    assert_eq!(
        gis.extension_tabs().count(),
        1,
        "the tab came back on the new lease"
    );
    // Granted after all (the same package again with another choice): the layer menu has it, in KentOS GIS only.
    let again = package("ornek-kent", |m| {
        m["version"] = "1.1.0".into();
        m["contributions"]["contextMenus"] = serde_json::json!([{ "builtin": "data.featureTable", "label": "Öznitelik tablosu", "target": "layer" }]);
        m["requestedCapabilities"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "ui.contextMenu", "scope": "plugin-owned", "reason": "Katmanın sağ tık menüsüne öznitelik tablosunu ekler."
        }));
    });
    install(&mut gis, "kent-rehberi-1.1.0.kek", again);
    assert_eq!(gis.extensions.menus.len(), 1);
    let item = &gis.extensions.menus[0];
    assert_eq!(item.target, kentos_extensions::manifest::MenuTarget::Layer);
    assert_eq!(
        (item.item.label.as_str(), &item.item.action),
        ("Öznitelik tablosu", &Action::Builtin("data.featureTable"))
    );
    let _ = gis.extension_menu(
        kentos_extensions::manifest::MenuTarget::Layer,
        kentos_ui::widget::Menu::new(),
    );
    // Back to 1.0.0 from the manager.
    let _ = gis.extensions_event(Event::Switch(KENT.into(), "1.0.0".into()));
    assert_eq!(
        gis.extensions
            .registry
            .as_ref()
            .unwrap()
            .lease(KENT)
            .unwrap()
            .version,
        "1.0.0"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_broken_package_is_reviewed_not_installed_and_the_windows_draw() {
    let root = temp("bozuk");
    let mut gis = app(&root, Some(Module::Gis));
    gis.open_extensions();
    let _ = gis.extensions_view();
    let _ = gis.extensions_event(Event::Picked(Some((
        "bozuk.kek".into(),
        b"PK\x03\x04 yarim".to_vec(),
    ))));
    let pending = gis.extensions.review.as_ref().unwrap();
    assert!(!pending.review.inspection.is_valid() && !pending.review.installable);
    let _ = gis
        .extension_review_window()
        .expect("the problems are shown");
    // Esc closes the review first, then the manager.
    gis.close_dialog();
    assert!(gis.extensions.review.is_none() && gis.dialog == Some(Dialog::Extensions));
    gis.close_dialog();
    assert_eq!(gis.dialog, None);
    // Without a registry the window says why.
    let (mut bare, _) = App::boot(None);
    bare.open_extensions();
    let _ = bare.extensions_view();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_catalog_folder_lists_its_packages_and_installs_the_verified_one() {
    let root = temp("katalog");
    let folder = temp("katalog-klasoru");
    let bytes = package("ornek-kent", |_| {});
    std::fs::write(folder.join("kent-rehberi-1.0.0.kek"), &bytes).unwrap();
    let index = kentos_extensions::catalog::index(
        "Örnek Belediyesi eklentileri",
        &[("kent-rehberi-1.0.0.kek".to_owned(), bytes)],
        &Limits::default(),
    )
    .unwrap();
    std::fs::write(folder.join("index.json"), index.to_json()).unwrap();
    let mut gis = app(&root, Some(Module::Gis));
    gis.open_extensions();
    let _ = gis.extensions_event(Event::PickedCatalog(Some(folder.clone())));
    assert_eq!(gis.extensions.part, Part::Catalog);
    assert_eq!(
        gis.extensions.chosen.as_deref(),
        Some("kent-rehberi-1.0.0.kek")
    );
    let _ = gis.extensions_view();
    let _ = gis.extensions_event(Event::Get("kent-rehberi-1.0.0.kek".into()));
    let pending = gis.extensions.review.as_ref().unwrap();
    assert_eq!(pending.source.kind, SourceKind::Organization);
    assert!(
        pending
            .changelog
            .as_deref()
            .unwrap()
            .contains("Kent Rehberi sekmesi")
    );
    let _ = gis.extensions_event(Event::Install);
    assert_eq!(gis.extension_tabs().count(), 1);
    // A file changed in the folder after indexing is refused.
    std::fs::write(folder.join("kent-rehberi-1.0.0.kek"), b"degisti").unwrap();
    let _ = gis.extensions_event(Event::Get("kent-rehberi-1.0.0.kek".into()));
    let refused = gis
        .extensions
        .note
        .as_ref()
        .is_some_and(|(error, words)| *error && words.contains("katalogdakiyle"));
    assert!(refused, "{:?}", gis.extensions.note);
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&folder);
}
