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
    // Kaynaklar's Eklentilerden: the data source, whose it is.
    let sources = gis.extension_sources();
    assert_eq!(sources.len(), 1);
    assert_eq!(
        (
            sources[0].0.as_str(),
            sources[0].1.as_str(),
            sources[0].2.id.as_str()
        ),
        (KENT, "Kent Rehberi", "ortofoto")
    );
    assert_eq!(super::kind_word(sources[0].2.kind), "WMTS");
    // The same registry in KentOS Mimari: no tab, no source.
    let mimari = app(&root, Some(Module::Architecture));
    assert!(mimari.extension_sources().is_empty());
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

/// The owner's wish (13 November): when an extension fails we know clearly what, where and why. A data source that
/// cannot be reached is the extension's error with its name in Uyarılar, not a silent window; the manager opens on it,
/// shows the problem first and copies a report; a start that cannot run it says so as an error with its name and which
/// file changed, and the next start says it is not running.
#[test]
fn a_failing_extension_is_said_with_its_name_and_kept_in_its_log() {
    let root = temp("hata");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "kent-rehberi.kek", package("ornek-kent", |_| {}));
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/document/v1/sample.json"
    ))
    .expect("the demo file reads");
    gis.document = Some(crate::document::Document::new(snapshot, None).expect("opens"));

    // Its data source, and the server does not answer.
    let _ = gis.extensions_event(Event::OpenSource(KENT.into(), "ortofoto".into()));
    let origin = gis.service_window.as_ref().and_then(|w| w.origin.clone());
    assert_eq!(
        origin,
        Some((KENT.to_owned(), "Ortofoto (2026)".to_owned()))
    );
    let _ = gis.update(Message::Services(crate::services::app::Event::Window(
        crate::services::window::Event::Connected(Err(
            "Bağlantı reddedildi: harita.ornek.bel.tr:443".to_owned(),
        )),
    )));
    let said = gis.log.last().expect("said");
    assert_eq!(said.level, Level::Error);
    assert!(
        said.text.starts_with(
            "Kent Rehberi eklentisi hata verdi — veri kaynağı “Ortofoto (2026)”: Servise bağlanılamadı"
        ) && said.text.contains("[Bağlantı reddedildi: harita.ornek.bel.tr:443]")
            && said.text.ends_with("Ayrıntı ve rapor: Eklentiler › Kent Rehberi."),
        "{}",
        said.text
    );
    let view = gis.extensions.registry.as_ref().unwrap().extensions();
    let problem = view[0].problem.clone().expect("the problem");
    assert_eq!(
        (problem.code.as_str(), problem.version.as_deref()),
        ("source_unreachable", Some("1.0.0"))
    );
    assert_eq!(gis.extensions.chosen.as_deref(), Some(KENT));
    // The status bar's cell stays until it is mended; clicked, the manager on it.
    assert_eq!(
        gis.extensions
            .troubles
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["Kent Rehberi"]
    );
    assert!(gis.extension_trouble_cell().is_some());
    gis.extensions.chosen = None;
    let _ = gis.extensions_event(Event::ShowTrouble);
    assert_eq!(gis.dialog, Some(Dialog::Extensions));
    assert_eq!(gis.extensions.chosen.as_deref(), Some(KENT));

    // The manager: the problem first, the log by level, the report.
    gis.open_extensions();
    let _ = gis.extensions_view();
    let _ = gis.extensions_event(Event::FullLog(true));
    let _ = gis.extensions_view();
    let report = gis.extension_report(KENT).expect("a report");
    for part in [
        "# Eklenti raporu: Kent Rehberi 1.0.0",
        "- Kimlik: `tr.ornek.kent-rehberi`",
        "## Sorun",
        "veri kaynağı “Ortofoto (2026)” · source_unreachable",
        "Bağlantı reddedildi: harita.ornek.bel.tr:443",
        "## Son kayıtlar (yeniden eskiye)",
        "installed",
    ] {
        assert!(report.contains(part), "{part}:\n{report}");
    }
    // The server answers at last (the service window's Bağlan read it): the source works again, its error no longer
    // stands and the cell goes.
    gis.extension_worked(
        KENT,
        "veri kaynağı “Ortofoto (2026)”",
        "source_connected",
        "Servise bağlanıldı.",
    );
    assert!(
        gis.extensions.troubles.is_empty(),
        "{:#?}",
        gis.extensions.troubles
    );
    assert_eq!(
        gis.extensions.registry.as_ref().unwrap().extensions()[0].problem,
        None
    );
    let _ = gis.extensions_event(Event::CopyReport(KENT.into()));
    assert!(
        gis.extensions
            .note
            .as_ref()
            .is_some_and(|(error, words)| !error && words.contains("panoya kopyalandı")),
        "{:?}",
        gis.extensions.note
    );

    // A file changed on disk: the next start says it as an error with the name and the file.
    drop(gis);
    let form = root
        .join("packages")
        .join(KENT)
        .join("1.0.0")
        .join("forms")
        .join("bina_denetimi.kform");
    let mut permissions = std::fs::metadata(&form).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&form, permissions).unwrap();
    std::fs::write(&form, "{}").unwrap();
    let (_, notes) = Extensions::open(&root, false);
    let error = notes
        .iter()
        .find(|(level, _)| *level == Level::Error)
        .map(|(_, words)| words.as_str())
        .unwrap_or_default();
    assert!(
        error.starts_with("Kent Rehberi eklentisi çalıştırılmadı: Kurulu dosyalar kurulduğu hâlinden farklı: forms/bina_denetimi.kform değişmiş."),
        "{notes:#?}"
    );
    // The start after: not running, said with its state.
    let (opened, notes) = Extensions::open(&root, false);
    assert!(
        notes.iter().any(|(level, words)| *level == Level::Warn
            && words.starts_with("Çalışmayan eklenti: Kent Rehberi (Hata).")),
        "{notes:#?}"
    );
    let problem = opened.registry.as_ref().unwrap().extensions()[0]
        .problem
        .clone()
        .expect("the problem");
    assert_eq!(problem.code, "integrity_mismatch");
    let troubles = problems::troubles(&opened);
    assert_eq!(troubles.len(), 1);
    assert!(
        troubles[0]
            .words
            .starts_with("etkinleştirme: Dosyaları kurulduğu gibi değil"),
        "{troubles:#?}"
    );
    // A start without extensions is no trouble: nothing runs on purpose.
    let (safe, _) = Extensions::open(&root, true);
    assert!(problems::troubles(&safe).is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

/// Komut ara finds an extension's buttons in its own program (docs/adr/8057 §3): by the label, by the tab's and the
/// extension's name; the row says where the button lives and whose it is; Enter runs it, Alt+Enter opens its tab and
/// outlines it; a button that runs a KentOS command is not listed twice; KentOS Mimari does not find KentOS GIS's.
#[test]
fn komut_ara_finds_an_extensions_buttons_in_its_own_program() {
    let root = temp("ara");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "kent-rehberi.kek", package("ornek-kent", |_| {}));
    // After KentOS's own command whose alias is the very word (Raster ekle…, ORTOFOTO): an alias equal comes first.
    gis.ribbon_search = "ortofoto".into();
    let rows = gis.search_rows();
    let at = rows
        .iter()
        .position(|r| r.title == "Ortofoto (2026)")
        .expect("the extension's button");
    let row = &rows[at];
    assert_eq!(
        (row.detail.as_ref(), row.enabled, row.revealable),
        ("Kent Rehberi › Altlık · Kent Rehberi eklentisi", true, true)
    );
    // By the extension's name: its own buttons, not the KentOS command it also shows (Öznitelik tablosu).
    gis.ribbon_search = "kent rehberi".into();
    let titles: Vec<String> = gis
        .search_rows()
        .iter()
        .map(|r| r.title.to_string())
        .collect();
    assert!(
        titles.contains(&"Ortofoto (2026)".to_owned())
            && titles.contains(&"Bina denetimi formu".to_owned()),
        "{titles:?}"
    );
    assert!(
        !titles.contains(&"Öznitelik tablosu".to_owned()),
        "{titles:?}"
    );
    // Alt+Enter: its tab, the button outlined.
    gis.ribbon_search = "ortofoto".into();
    let _ = gis.search_reveal(at);
    assert_eq!(gis.regular_tab(), "ext.tr.ornek.kent-rehberi.kent");
    let hit = gis.extensions.flash.expect("outlined");
    assert_eq!((hit.panel, hit.item), (0, 0));
    let _ = gis.ribbon(false);
    let _ = gis.extensions_event(Event::FlashEnd(hit.tab, hit.panel, hit.item));
    assert_eq!(gis.extensions.flash, None);
    // Enter: what pressing it does (no drawing: the service window says so, nothing breaks).
    gis.ribbon_search = "ortofoto".into();
    let _ = gis.search_run(at);
    assert!(gis.ribbon_search.is_empty());
    // KentOS Mimari: not its program's button.
    let mut mimari = app(&root, Some(Module::Architecture));
    mimari.ribbon_search = "ortofoto".into();
    assert!(
        mimari
            .search_rows()
            .iter()
            .all(|r| r.title != "Ortofoto (2026)")
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Alt or F6 (docs/specs/ribbon.md §1): an extension's tab has a key tip after KentOS's own tabs, which keep theirs;
/// typed, the tab opens and its buttons get theirs; a button's tip typed runs it.
#[test]
fn an_extensions_tab_and_its_buttons_have_key_tips() {
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Key, Modifiers};

    use crate::ribbon_keys::TipKey;

    let letter = |app: &mut App, c: &str| {
        let _ = app.update(Message::Key(crate::keys::KeyPress {
            key: Key::Character(c.into()),
            physical: Physical::Unidentified(NativeCode::Unidentified),
            modifiers: Modifiers::default(),
            text: Some(c.to_owned()),
            repeat: false,
        }));
    };
    let tip = |app: &App, key: TipKey| app.key_tip(&key).map(|t| t.tip);

    let root = temp("ipucu");
    let bare_root = temp("ipucu-bos");
    let mut bare = app(&bare_root, Some(Module::Gis));
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "kent-rehberi.kek", package("ornek-kent", |_| {}));
    gis.open_extensions();
    let _ = gis.extensions_event(Event::Close);
    let _ = bare.update(Message::Run("view.keyTips"));
    let _ = gis.update(Message::Run("view.keyTips"));
    // KentOS's own tabs keep their tips.
    let ids: Vec<&'static str> = bare.ribbon_tabs().map(|t| t.id).collect();
    for id in ids {
        assert_eq!(
            tip(&gis, TipKey::Tab(id)),
            tip(&bare, TipKey::Tab(id)),
            "{id}"
        );
    }
    let tab = "ext.tr.ornek.kent-rehberi.kent";
    assert_eq!(tip(&gis, TipKey::Tab(tab)).as_deref(), Some("K"));
    let _ = gis.ribbon(false);
    // Typed: the tab opens, its buttons get tips; a builtin button too, where the program offers it.
    letter(&mut gis, "k");
    assert_eq!(gis.shown_tab(), tab);
    let ortofoto = tip(&gis, TipKey::ExtItem(tab, 0, 0)).expect("Ortofoto's tip");
    assert!(tip(&gis, TipKey::ExtItem(tab, 1, 0)).is_some(), "the form");
    assert!(
        tip(&gis, TipKey::ExtItem(tab, 2, 0)).is_some(),
        "Öznitelik tablosu"
    );
    let _ = gis.ribbon(false);
    // What Ortofoto's tip runs: the source, as its button does.
    let controls = gis.extension_tip_controls(tab);
    assert!(
        matches!(
            &controls[0],
            (
                TipKey::ExtItem(_, 0, 0),
                label,
                crate::ribbon_keys::TipAction::Run(Message::Extensions(Event::OpenSource(e, l)))
            ) if label == "Ortofoto (2026)" && e == KENT && l == "ortofoto"
        ),
        "{:?}",
        controls.iter().map(|(_, l, _)| l).collect::<Vec<_>>()
    );
    // Typed in full, it runs and the tips go.
    for c in ortofoto.chars() {
        letter(&mut gis, &c.to_lowercase().to_string());
    }
    assert!(gis.key_tips.is_none(), "a control ran");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&bare_root);
}

/// Sorunlu eklentiyi bul in the window (docs/adr/8057 §4): the manager's button opens it; Başla holds every running
/// extension off (its tab goes) without changing the user's choices; closed in the middle it goes on, and a restart
/// asks again; the answer names the culprit, brings every extension back, and the one found can be disabled there.
#[test]
fn sorunlu_eklentiyi_bul_finds_the_culprit_in_the_window() {
    use super::bisect::BisectStep;

    let root = temp("sorun-bul");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "kent-rehberi.kek", package("ornek-kent", |_| {}));
    assert_eq!(gis.extension_tabs().count(), 1);
    let _ = gis.extensions_event(Event::BisectOpen);
    assert_eq!(gis.extensions.bisect, Some(BisectStep::Intro));
    assert!(gis.extension_bisect_window().is_some());
    let _ = gis.extensions_event(Event::BisectStart);
    assert_eq!(gis.extensions.bisect, Some(BisectStep::Asking));
    assert_eq!(gis.extension_tabs().count(), 0, "held off: its tab goes");
    assert_eq!(
        gis.extensions.registry.as_ref().unwrap().state(KENT),
        Some(State::Held)
    );
    assert!(gis.extensions.troubles.is_empty(), "held is no trouble");
    assert!(gis.extension_bisect_window().is_some());
    assert!(gis.bisect_line().is_some(), "the manager says it goes on");
    let _ = gis.extensions_view();
    // Closed in the middle: it goes on; a restart asks again.
    let _ = gis.extensions_event(Event::BisectClose);
    assert_eq!(gis.extensions.bisect, None);
    let (again, notes) = Extensions::open(&root, false);
    assert_eq!(again.bisect, Some(BisectStep::Asking));
    assert!(
        notes.iter().any(|(level, words)| *level == Level::Warn
            && words.starts_with("Sorunlu eklentiyi bulma sürüyor (tur 1)")),
        "{notes:#?}"
    );
    drop(again);
    // Not seen with none running: with one suspect, it is that one.
    let _ = gis.extensions_event(Event::BisectOpen);
    assert_eq!(gis.extensions.bisect, Some(BisectStep::Asking));
    let _ = gis.extensions_event(Event::BisectAnswer(false));
    assert_eq!(
        gis.extensions.bisect,
        Some(BisectStep::Found {
            id: KENT.into(),
            rounds: 1
        })
    );
    assert_eq!(gis.extension_tabs().count(), 1, "back");
    assert!(gis.bisect_line().is_none());
    assert!(gis.extension_bisect_window().is_some());
    // Named in its log: the status bar's cell says so until it is mended or disabled.
    assert_eq!(gis.extensions.troubles.len(), 1);
    let _ = gis.extensions_event(Event::BisectDisable(KENT.into()));
    assert_eq!(gis.extensions.bisect, None);
    assert_eq!(
        gis.extensions.registry.as_ref().unwrap().state(KENT),
        Some(State::Disabled)
    );
    assert!(
        gis.extensions.troubles.is_empty(),
        "disabled: no trouble of now"
    );
    let _ = std::fs::remove_dir_all(&root);
}
