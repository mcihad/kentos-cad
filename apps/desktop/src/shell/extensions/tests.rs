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
    assert_eq!(
        gis.extensions.part,
        Part::Available,
        "nothing to update: what can be installed"
    );
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

/// A package's own icons (docs/adr/8057 §3): the 20×20 SVGs checked when it was installed are drawn on its buttons and
/// beside its name in the manager; an item without one keeps the icon of what it does.
#[test]
fn a_packages_own_icons_are_drawn() {
    use kentos_ui::icon::Icon;

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/extensions/v1/ornek-kent/assets");
    let svg = |name: &str| {
        Icon::Svg(super::ribbon::intern(
            &std::fs::read_to_string(fixture.join(name)).unwrap(),
        ))
    };
    let root = temp("ikon");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "kent-rehberi.kek", package("ornek-kent", |_| {}));
    let tab = gis.extension_tabs().next().expect("the tab");
    assert_eq!(
        tab.panels[1].items[0].icon,
        svg("denetim.svg"),
        "the form's own icon"
    );
    assert_ne!(tab.panels[0].items[0].icon, svg("denetim.svg"));
    assert_eq!(
        tab.panels[0].items[0].icon,
        crate::icons::from_web(Some("serviceSettings")),
        "a data source without one: the service icon"
    );
    assert_eq!(
        gis.extensions.icons.get(KENT),
        Some(&svg("kent-rehberi.svg")),
        "the package's icon"
    );
    gis.open_extensions();
    let _ = gis.extensions_view();
    let _ = gis.ribbon(false);
    let _ = std::fs::remove_dir_all(&root);
}

/// docs/adr/8057 §21: a built-in icon as a manifest writes it (`zoom-extents`) is the icon set's `zoomExtents`, drawn
/// as the web draws it; before, the manifest could not name the 1133 icons the set writes in camelCase.
#[test]
fn a_built_in_icon_is_named_with_dashes() {
    use kentos_ui::icon::Icon;

    let root = temp("ikon-adi");
    let mut gis = app(&root, Some(Module::Gis));
    install(
        &mut gis,
        "kent-rehberi.kek",
        package("ornek-kent", |m| {
            m["contributions"]["ribbonTabs"][0]["panels"][0]["items"][0]["icon"] =
                serde_json::json!("zoom-extents");
        }),
    );
    let markup = crate::icons::web_markup("zoomExtents").expect("the set's icon");
    let tab = gis.extension_tabs().next().expect("the tab");
    assert_eq!(tab.panels[0].items[0].icon, Icon::Svg(markup));
    let _ = std::fs::remove_dir_all(&root);
}

/// Eklentiler's views (D1): Kurulu, Bulunabilir, Güncellenebilir, Yeni and Sorunlu with their counts; a word and the
/// program filter the lists; Yeni is what came to the catalog since its previous opening.
#[test]
fn the_manager_lists_by_part_word_and_program() {
    let root = temp("gorunum");
    let folder = temp("gorunum-katalog");
    let write_catalog = |packages: &[(&str, Vec<u8>)]| {
        for (file, bytes) in packages {
            std::fs::write(folder.join(file), bytes).unwrap();
        }
        let list: Vec<(String, Vec<u8>)> = packages
            .iter()
            .map(|(f, b)| ((*f).to_owned(), b.clone()))
            .collect();
        let index = kentos_extensions::catalog::index(
            "Örnek Belediyesi eklentileri",
            &list,
            &Limits::default(),
        )
        .unwrap();
        std::fs::write(folder.join("index.json"), index.to_json()).unwrap();
    };
    let kent = package("ornek-kent", |_| {});
    let altlik = package("ornek-altlik", |_| {});
    write_catalog(&[
        ("kent-1.0.0.kek", kent.clone()),
        ("altlik-1.0.0.kek", altlik.clone()),
    ]);
    let mut gis = app(&root, Some(Module::Gis));
    gis.open_extensions();
    let _ = gis.extensions_event(Event::PickedCatalog(Some(folder.clone())));
    let counts =
        |app: &App| -> Vec<usize> { Part::ALL.iter().map(|p| app.extensions.count(*p)).collect() };
    // Kurulu, Bulunabilir, Güncellenebilir, Yeni (a first opening: all), Sorunlu.
    assert_eq!(counts(&gis), [0, 2, 0, 2, 0]);
    assert_eq!(gis.extensions.part, Part::Available);
    for part in Part::ALL {
        let _ = gis.extensions_event(Event::Part(part));
        let _ = gis.extensions_view();
    }
    // Kent Rehberi from the catalog.
    let _ = gis.extensions_event(Event::Get("kent-1.0.0.kek".into()));
    let _ = gis.extensions_event(Event::Install);
    assert_eq!(counts(&gis), [1, 1, 0, 2, 0]);
    // A word: its name, its summary, a publisher; nothing found says so.
    let _ = gis.extensions_event(Event::Query("altlık".into()));
    assert_eq!(counts(&gis), [0, 1, 0, 1, 0]);
    let _ = gis.extensions_event(Event::Query("öznitelik tablosu".into()));
    assert_eq!(counts(&gis), [1, 0, 0, 1, 0], "Kent Rehberi's summary");
    let _ = gis.extensions_event(Event::Query("yok böyle".into()));
    assert_eq!(counts(&gis), [0, 0, 0, 0, 0]);
    let _ = gis.extensions_view();
    let _ = gis.extensions_event(Event::Query(String::new()));
    // The program: KentOS Mimari's has neither; KentOS Harita's has Örnek altlıklar alone.
    let mimari = super::browse::PROGRAMS
        .iter()
        .position(|(m, _)| *m == Some(kentos_extensions::manifest::Module::Mimari))
        .unwrap();
    let harita = super::browse::PROGRAMS
        .iter()
        .position(|(m, _)| *m == Some(kentos_extensions::manifest::Module::Harita))
        .unwrap();
    let _ = gis.extensions_event(Event::ProgramFilter(mimari));
    assert_eq!(counts(&gis), [0, 0, 0, 0, 0]);
    let _ = gis.extensions_event(Event::ProgramFilter(harita));
    assert_eq!(counts(&gis), [0, 1, 0, 1, 0]);
    let _ = gis.extensions_event(Event::ProgramFilter(0));
    // Kent Rehberi 1.1.0 comes to the catalog: an update, and the one new thing since the previous opening.
    let newer = package("ornek-kent", |m| m["version"] = "1.1.0".into());
    write_catalog(&[
        ("kent-1.0.0.kek", kent),
        ("kent-1.1.0.kek", newer),
        ("altlik-1.0.0.kek", altlik),
    ]);
    gis.open_extension_catalog(&folder, true);
    assert_eq!(
        gis.extensions.part,
        Part::Updates,
        "updates first when there are"
    );
    assert_eq!(counts(&gis), [1, 1, 1, 1, 0]);
    assert_eq!(gis.extensions.entries_of(Part::New)[0].version, "1.1.0");
    // Sorunlu: an extension with an error standing.
    gis.extension_failed(
        KENT,
        "form “bina-denetimi”",
        "form_invalid",
        "Form okunamadı.",
        None,
    );
    assert_eq!(counts(&gis)[4], 1);
    let _ = gis.extensions_event(Event::Part(Part::Troubled));
    assert_eq!(gis.extensions.chosen.as_deref(), Some(KENT));
    let _ = gis.extensions_view();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&folder);
}

/// A `.kek` double-clicked (PY-KEK-15): the program started with it opens its review over Eklentiler; opening it
/// installs and runs nothing; a file that cannot be read says so.
#[test]
fn a_kek_file_opened_is_reviewed_not_installed() {
    let root = temp("cift-tik");
    let files = temp("cift-tik-dosya");
    let file = files.join("kent-rehberi.kek");
    std::fs::write(&file, package("ornek-kent", |_| {})).unwrap();
    let mut gis = app(&root, Some(Module::Gis));
    gis.review_kek_file(&file);
    assert_eq!(gis.dialog, Some(Dialog::Extensions));
    let pending = gis.extensions.review.as_ref().expect("its review");
    assert_eq!(pending.source.kind, SourceKind::File);
    assert!(pending.review.installable);
    assert!(
        gis.extensions
            .registry
            .as_ref()
            .unwrap()
            .extensions()
            .is_empty(),
        "nothing installed by opening it"
    );
    assert!(gis.extension_review_window().is_some());
    gis.extensions.review = None;
    gis.review_kek_file(&files.join("yok.kek"));
    assert!(
        gis.extensions
            .note
            .as_ref()
            .is_some_and(|(error, words)| *error && words.contains("okunamadı")),
        "{:?}",
        gis.extensions.note
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&files);
}

/// Python ve eklentiler (plan/eklenti.md ADIM 18A, P01/P02): off by default, nothing looked for; an extension's Python
/// button says why it does not run: Python off, uv not ready, or the runner still to come; the choice is written at
/// once; turned off, what was found is dropped.
#[test]
fn an_extensions_python_buttons_say_why_by_the_users_python() {
    use crate::python_setup::{OwnPython, PythonEdit};
    use kentos_python_env::python::{Cause, InstallError, Interpreter, Progress};
    use kentos_python_env::uv::UvStatus;
    use std::sync::Arc;

    let root = temp("python");
    let python_root = temp("python-ayar");
    let mut gis = app(&root, Some(Module::Gis));
    gis.python_setup.root = Some(python_root.clone());
    // Without its dependency on another example extension: MEBİS alone runs.
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = serde_json::json!([]);
        }),
    );
    let why = |app: &App| -> String {
        let tab = app
            .extension_tabs()
            .find(|t| t.extension == "tr.ornek.mebis")
            .expect("MEBİS's tab");
        match &tab.panels[0].items[0].action {
            Action::Unavailable(why) => why.clone(),
            other => panic!("{other:?}"),
        }
    };
    assert!(!gis.python_setup.settings.enabled, "off by default");
    assert!(gis.python_setup.uv.is_none(), "nothing looked for");
    assert!(
        why(&gis).contains("Python kapalı: Uygulama ayarları › Python ve eklentiler"),
        "{}",
        why(&gis)
    );
    // Turned on: written at once, uv looked for in the background.
    let _ = gis.python_edit(PythonEdit::Enable(true));
    assert!(gis.python_setup.looking);
    assert!(
        kentos_python_env::settings::PythonSettings::load(&python_root).enabled,
        "written at once"
    );
    assert!(why(&gis).contains("uv aranıyor"));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::NotFound {
        looked: Vec::new(),
        own: false,
    }));
    assert!(why(&gis).contains("uv hazır değil"), "{}", why(&gis));
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/ev/.local/bin/uv".into(),
        version: "0.11.26".into(),
        bundled: false,
    }));
    // uv ready: KentOS's own Python looked for in the background.
    assert_eq!(gis.python_setup.python, Some(OwnPython::Looking));
    assert!(
        why(&gis).contains("KentOS'un Python'u aranıyor"),
        "{}",
        why(&gis)
    );
    let _ = gis.python_edit(PythonEdit::Found(Ok((None, 0))));
    assert_eq!(gis.python_setup.python, Some(OwnPython::Missing));
    assert!(
        why(&gis).contains("KentOS'un Python'u kurulu değil"),
        "{}",
        why(&gis)
    );
    let _ = gis.python_section();
    // Installed only when the user presses the button (the consent); Durdur stops it, and what ended it is said.
    assert!(
        gis.python_setup.installing.is_none(),
        "nothing installed unasked"
    );
    let _ = gis.python_edit(PythonEdit::Install);
    let stop = gis.python_setup.installing.clone().expect("installing");
    assert!(why(&gis).contains("kuruluyor"), "{}", why(&gis));
    let _ = gis.python_edit(PythonEdit::Install);
    assert!(
        Arc::ptr_eq(&stop, gis.python_setup.installing.as_ref().unwrap()),
        "one install at a time"
    );
    let _ = gis.python_section();
    // How far it is, as the bar says it: uv's size and the bytes so far, then unpacking.
    assert_eq!(gis.python_setup.progress, Some(Progress::Starting));
    let at = Progress::Downloading {
        done: 13_002_342,
        total: Some(34_183_578),
    };
    let _ = gis.python_edit(PythonEdit::Progressed(at));
    assert_eq!(gis.python_setup.progress, Some(at));
    assert_eq!(
        crate::python_setup::progress_words(at),
        "İndiriliyor: 12,4 MB / 32,6 MB · %38"
    );
    assert_eq!(
        crate::python_setup::progress_words(Progress::Downloading {
            done: 3_418_399,
            total: Some(34_183_578)
        }),
        "İndiriliyor: 3,3 MB / 32,6 MB · %10"
    );
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::Progressed(Progress::Unpacking));
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::StopInstall);
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed), "Durdur");
    let toasts = gis.toasts.len();
    let _ = gis.python_edit(PythonEdit::Installed(Err(InstallError::Cancelled)));
    assert!(gis.python_setup.installing.is_none() && gis.python_setup.progress.is_none());
    // Said by a notice; nothing stays in the section, where the button waits again.
    assert_eq!(gis.toasts.len(), toasts + 1);
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python kurulumu durduruldu")
    );
    assert!(gis.python_setup.note.is_none());
    let _ = gis.python_edit(PythonEdit::Install);
    let _ = gis.python_edit(PythonEdit::Installed(Err(InstallError::Failed {
        cause: Cause::Network,
        said: "dns error".into(),
    })));
    // A failure: in the section, and a notice that waits to be closed.
    assert!(
        matches!(&gis.python_setup.note, Some((_, said)) if said.contains("internete ulaşılamadı")),
        "{:?}",
        gis.python_setup.note
    );
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python kurulamadı")
    );
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::Install);
    let _ = gis.python_edit(PythonEdit::Installed(Ok(())));
    assert!(gis.python_setup.note.is_none());
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python 3.12 kuruldu")
    );
    assert_eq!(
        gis.python_setup.python,
        Some(OwnPython::Looking),
        "looked for again"
    );
    let installed = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((
        Some(installed.clone()),
        102_640_000,
    ))));
    // KentOS's Python ready: the button now says what the extension's own environment needs (environment.rs).
    assert!(
        why(&gis).contains("Bu eklentinin Python ortamı hazırlanmadı"),
        "{}",
        why(&gis)
    );
    let _ = gis.python_section();
    // Removed from the same place: Kaldır asks first, Vazgeç keeps it, Evet, kaldır removes it and says the room freed.
    let _ = gis.python_edit(PythonEdit::Remove);
    assert!(gis.python_setup.confirm_remove, "asks first");
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::RemoveCancelled);
    assert!(!gis.python_setup.confirm_remove && !gis.python_setup.removing);
    let _ = gis.python_edit(PythonEdit::Remove);
    let _ = gis.python_edit(PythonEdit::RemoveConfirmed);
    assert!(gis.python_setup.removing && !gis.python_setup.confirm_remove);
    assert!(why(&gis).contains("kaldırılıyor"), "{}", why(&gis));
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::Install);
    assert!(gis.python_setup.installing.is_none(), "not while removing");
    let _ = gis.python_edit(PythonEdit::Removed(Ok(102_640_000)));
    assert!(!gis.python_setup.removing);
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python kaldırıldı"),
        "a notice"
    );
    assert_eq!(crate::python_setup::size_words(102_640_000), "98 MB");
    let _ = gis.python_edit(PythonEdit::Found(Ok((None, 0))));
    assert_eq!(gis.python_setup.python, Some(OwnPython::Missing));
    // A removal that fails says why, in the section and by a notice.
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(installed), 102_640_000))));
    let _ = gis.python_edit(PythonEdit::Remove);
    let _ = gis.python_edit(PythonEdit::RemoveConfirmed);
    let _ = gis.python_edit(PythonEdit::Removed(Err(
        "Python kaldırılamadı: dosyaları kullanılıyor.".into(),
    )));
    assert!(
        matches!(&gis.python_setup.note, Some((_, said)) if said.contains("dosyaları kullanılıyor")),
        "{:?}",
        gis.python_setup.note
    );
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python kaldırılamadı")
    );
    // A path by hand, kept.
    let _ = gis.python_edit(PythonEdit::UvTyped("/opt/araclar/uv".into()));
    let _ = gis.python_edit(PythonEdit::UvApply);
    assert_eq!(
        kentos_python_env::settings::PythonSettings::load(&python_root)
            .uv_path
            .as_deref(),
        Some("/opt/araclar/uv")
    );
    // Off again: what was found is dropped; nothing looked for; an install under way is stopped.
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/ev/.local/bin/uv".into(),
        version: "0.11.26".into(),
        bundled: false,
    }));
    let _ = gis.python_edit(PythonEdit::Install);
    let stop = gis.python_setup.installing.clone().expect("installing");
    let _ = gis.python_edit(PythonEdit::Enable(false));
    assert!(
        stop.load(std::sync::atomic::Ordering::Relaxed),
        "stopped when turned off"
    );
    assert!(gis.python_setup.uv.is_none() && !gis.python_setup.looking);
    assert!(gis.python_setup.python.is_none());
    assert!(why(&gis).contains("Python kapalı"));
    // The settings window's section draws.
    gis.open_settings_at(crate::settings_sections::Section::Python);
    let _ = gis.settings_dialog();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&python_root);
}

/// Kişisel ortam (plan/eklenti.md ADIM 18A, P04): on KentOS's Python, made on the user's press; a package typed as an
/// option never reaches uv; a change is previewed (asked for, dependencies) and made only when agreed, with how far it
/// is and Durdur; packages and the environment are removed from the same place, the environment asking first; each
/// outcome a notice (the owner, 13 November 2026).
#[test]
fn the_personal_environment_is_made_previewed_changed_and_removed() {
    use crate::python_personal::{Busy, OwnEnv, PersonalEdit};
    use crate::python_setup::{OwnPython, PythonEdit};
    use kentos_python_env::personal::{Change, Outdated, Package, Personal, Preview};
    use kentos_python_env::python::{InstallError, Interpreter, Progress};
    use kentos_python_env::uv::UvStatus;

    let root = temp("kisisel");
    let python_root = temp("kisisel-ayar");
    let mut gis = app(&root, Some(Module::Gis));
    gis.python_setup.root = Some(python_root.clone());
    let edit = |app: &mut App, e: PersonalEdit| {
        let _ = app.python_edit(PythonEdit::Personal(e));
    };
    let titles = |app: &App| -> Vec<String> {
        app.toasts
            .iter()
            .map(|(_, toast, _)| toast.title().to_owned())
            .collect()
    };
    let _ = gis.python_edit(PythonEdit::Enable(true));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/k/tools/uv".into(),
        version: "0.11.26".into(),
        bundled: true,
    }));
    assert!(
        gis.python_setup.personal.env.is_none(),
        "not before KentOS's Python"
    );
    let python = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python), 97_885_768))));
    assert!(matches!(
        gis.python_setup.python,
        Some(OwnPython::Ready(..))
    ));
    assert_eq!(gis.python_setup.personal.env, Some(OwnEnv::Looking));
    edit(&mut gis, PersonalEdit::Found(Ok((None, 4096))));
    assert_eq!(gis.python_setup.personal.env, Some(OwnEnv::Missing));
    let _ = gis.python_section();
    // Made on the user's press.
    edit(&mut gis, PersonalEdit::Create);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Creating));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Created(Ok(())));
    assert!(titles(&gis).contains(&"Kişisel ortam oluşturuldu".to_owned()));
    let made = Personal {
        python: python_root.join("kisisel/.venv/bin/python"),
        size: 362_000_000,
        sdk: Some("0.1.0".into()),
        packages: Vec::new(),
    };
    edit(
        &mut gis,
        PersonalEdit::Found(Ok((Some(made.clone()), 4096))),
    );
    assert!(matches!(
        &gis.python_setup.personal.env,
        Some(OwnEnv::Ready(_))
    ));
    let _ = gis.python_section();
    // An option typed as a package never reaches uv.
    edit(
        &mut gis,
        PersonalEdit::Typed("--index-url=https://ornek.invalid".into()),
    );
    edit(&mut gis, PersonalEdit::PreviewAdd);
    assert!(gis.python_setup.personal.busy.is_none(), "uv not asked");
    assert!(gis.python_setup.personal.note.is_some());
    // Previewed: what is asked for, what comes with it.
    edit(&mut gis, PersonalEdit::Typed("pandas".into()));
    edit(&mut gis, PersonalEdit::PreviewAdd);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Previewing));
    let _ = gis.python_section();
    let package = |name: &str, version: &str, direct| Package {
        name: name.into(),
        version: version.into(),
        direct,
    };
    let adding = Preview {
        change: Change::Add(vec!["pandas".into()]),
        added: vec![
            package("pandas", "3.1.1", true),
            package("numpy", "2.5.4", false),
            package("python-dateutil", "4.0.0", false),
            package("tzdata", "2026.6", false),
        ],
        removed: Vec::new(),
        changed: Vec::new(),
        direct: vec!["pandas".into()],
    };
    assert_eq!(
        crate::python_personal::preview_words(&adding),
        [
            "Kurulacak: pandas 3.1.1.",
            "Bağımlılık olarak gelecek (3): numpy 2.5.4, python-dateutil 4.0.0, tzdata 2026.6."
        ]
    );
    edit(&mut gis, PersonalEdit::Previewed(Ok(adding.clone())));
    assert!(gis.python_setup.personal.preview.is_some());
    let _ = gis.python_section();
    // Agreed, then Durdur: the environment went back.
    edit(&mut gis, PersonalEdit::Apply);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Applying));
    let stop = gis.python_setup.personal.stop.clone().expect("applying");
    edit(
        &mut gis,
        PersonalEdit::Progressed(Progress::Downloading {
            done: 12_058_624,
            total: Some(28_835_840),
        }),
    );
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Stop);
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed));
    edit(
        &mut gis,
        PersonalEdit::Applied(Err(InstallError::Cancelled)),
    );
    assert!(titles(&gis).contains(&"Paket kurulumu durduruldu".to_owned()));
    assert!(
        gis.python_setup.personal.busy.is_none() && gis.python_setup.personal.preview.is_none()
    );
    // Again, to the end.
    edit(&mut gis, PersonalEdit::Previewed(Ok(adding)));
    edit(&mut gis, PersonalEdit::Apply);
    edit(&mut gis, PersonalEdit::Applied(Ok(())));
    assert!(titles(&gis).contains(&"pandas 3.1.1 kuruldu".to_owned()));
    assert!(gis.python_setup.personal.typed.is_empty());
    let with_pandas = Personal {
        packages: vec![
            package("pandas", "3.1.1", true),
            package("numpy", "2.5.4", false),
            package("python-dateutil", "4.0.0", false),
            package("tzdata", "2026.6", false),
        ],
        ..made
    };
    edit(
        &mut gis,
        PersonalEdit::Found(Ok((Some(with_pandas), 30_000_000))),
    );
    let _ = gis.python_section();
    // A package removed from the same place, previewed too.
    edit(&mut gis, PersonalEdit::PreviewRemove("pandas".into()));
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Previewing));
    let removing = Preview {
        change: Change::Remove(vec!["pandas".into()]),
        added: Vec::new(),
        removed: vec![
            package("pandas", "3.1.1", true),
            package("numpy", "2.5.4", false),
            package("python-dateutil", "4.0.0", false),
            package("tzdata", "2026.6", false),
        ],
        changed: Vec::new(),
        direct: Vec::new(),
    };
    edit(&mut gis, PersonalEdit::Previewed(Ok(removing)));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Apply);
    edit(&mut gis, PersonalEdit::Applied(Ok(())));
    assert!(titles(&gis).contains(&"pandas kaldırıldı".to_owned()));
    // Picked from the recommended list, several at once, previewed together.
    edit(&mut gis, PersonalEdit::Typed("koordinat".into()));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Typed(String::new()));
    edit(&mut gis, PersonalEdit::ShowAll(true));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Pick("shapely".into(), true));
    edit(&mut gis, PersonalEdit::Pick("pyproj".into(), true));
    edit(&mut gis, PersonalEdit::Pick("numpy".into(), true));
    edit(&mut gis, PersonalEdit::Pick("numpy".into(), false));
    assert_eq!(
        gis.python_setup
            .personal
            .picked
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        ["pyproj", "shapely"]
    );
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::PreviewPicked);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Previewing));
    edit(
        &mut gis,
        PersonalEdit::Previewed(Ok(Preview {
            change: Change::Add(vec!["pyproj".into(), "shapely".into()]),
            added: vec![
                package("pyproj", "3.7.2", true),
                package("shapely", "2.1.2", true),
                package("certifi", "2026.10.5", false),
            ],
            removed: Vec::new(),
            changed: Vec::new(),
            direct: vec!["pyproj".into(), "shapely".into()],
        })),
    );
    edit(&mut gis, PersonalEdit::Apply);
    edit(&mut gis, PersonalEdit::Applied(Ok(())));
    assert!(titles(&gis).contains(&"pyproj 3.7.2, shapely 2.1.2 kuruldu".to_owned()));
    assert!(
        gis.python_setup.personal.picked.is_empty(),
        "ticks cleared once made"
    );
    // A name not among the recommended ones is asked for as it is.
    edit(&mut gis, PersonalEdit::Typed("pyogrio-yok-boyle".into()));
    let _ = gis.python_section();
    // Updates: asked of uv; one or all previewed as an upgrade.
    let with_old = Personal {
        packages: vec![
            package("pandas", "2.3.3", true),
            package("numpy", "2.5.4", false),
        ],
        python: python_root.join("kisisel/.venv/bin/python"),
        size: 400_000_000,
        sdk: Some("0.1.0".into()),
    };
    edit(&mut gis, PersonalEdit::Found(Ok((Some(with_old), 1))));
    edit(&mut gis, PersonalEdit::CheckUpdates);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Checking));
    edit(
        &mut gis,
        PersonalEdit::Checked(Ok(vec![
            Outdated {
                name: "pandas".into(),
                version: "2.3.3".into(),
                latest: "3.1.1".into(),
            },
            Outdated {
                name: "numpy".into(),
                version: "2.5.4".into(),
                latest: "2.6.0".into(),
            },
        ])),
    );
    assert!(titles(&gis).contains(&"2 paketin yeni sürümü var".to_owned()));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Upgrade(None));
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Previewing));
    edit(
        &mut gis,
        PersonalEdit::Previewed(Ok(Preview {
            change: Change::Upgrade {
                raise: vec![("pandas".into(), "3.1.1".into())],
                others: vec!["numpy".into()],
            },
            added: Vec::new(),
            removed: Vec::new(),
            changed: vec![
                ("numpy".into(), "2.5.4".into(), "2.6.0".into()),
                ("pandas".into(), "2.3.3".into(), "3.1.1".into()),
            ],
            direct: vec!["pandas".into()],
        })),
    );
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Apply);
    edit(&mut gis, PersonalEdit::Applied(Ok(())));
    assert!(titles(&gis).contains(&"Paketler güncellendi".to_owned()));
    assert!(
        gis.python_setup.personal.outdated.is_none(),
        "asked again when wanted"
    );
    // The dependency tree, shown and hidden.
    edit(&mut gis, PersonalEdit::ToggleTree);
    assert_eq!(gis.python_setup.personal.tree.as_deref(), Some(""));
    edit(
        &mut gis,
        PersonalEdit::Tree(Ok("kentos-kisisel v0\n└── pandas v3.1.1\n".into())),
    );
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::ToggleTree);
    assert!(gis.python_setup.personal.tree.is_none());
    edit(&mut gis, PersonalEdit::Tree(Ok("geç gelen".into())));
    assert!(
        gis.python_setup.personal.tree.is_none(),
        "hidden meanwhile: stays hidden"
    );
    // A failure is said in the part and by a notice that waits.
    edit(&mut gis, PersonalEdit::Typed("numpy".into()));
    edit(&mut gis, PersonalEdit::PreviewAdd);
    edit(
        &mut gis,
        PersonalEdit::Previewed(Err("Paket dizinine ulaşılamadı.".into())),
    );
    assert_eq!(
        gis.python_setup.personal.note.as_deref(),
        Some("Paket dizinine ulaşılamadı.")
    );
    // The cache, apart.
    edit(&mut gis, PersonalEdit::CleanCache);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Cleaning));
    edit(&mut gis, PersonalEdit::Cleaned(Ok(())));
    assert!(titles(&gis).contains(&"Paket önbelleği temizlendi".to_owned()));
    // KentOS's Python's removal says the environment goes with it.
    let _ = gis.python_edit(PythonEdit::Remove);
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::RemoveCancelled);
    // The environment removed, asked first.
    edit(&mut gis, PersonalEdit::Remove);
    assert!(gis.python_setup.personal.confirm_remove);
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::RemoveCancelled);
    assert!(!gis.python_setup.personal.confirm_remove);
    edit(&mut gis, PersonalEdit::Remove);
    edit(&mut gis, PersonalEdit::RemoveConfirmed);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Removing));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Removed(Ok(())));
    assert!(titles(&gis).contains(&"Kişisel ortam kaldırıldı".to_owned()));
    // Python off: the environment is let go of.
    let _ = gis.python_edit(PythonEdit::Enable(false));
    assert!(gis.python_setup.personal.env.is_none());
    gis.open_settings_at(crate::settings_sections::Section::Python);
    let _ = gis.settings_dialog();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&python_root);
}

/// Bir eklentinin Python ortamı (plan/eklenti.md ADIM 18A, P05, P07): in the extension's details, once KentOS's Python
/// is ready: looked for when chosen; what preparing would install previewed from its lock, its optional feature ticked
/// into it; made with a bar and Durdur, each outcome a notice; the ribbon's button says, for that extension, whether
/// its environment is ready (files on disk); removed from the same place, and with the extension.
#[test]
fn an_extensions_python_environment_is_previewed_prepared_and_removed() {
    use super::environment::{EnvBusy, EnvEvent, EnvStatus};
    use crate::python_setup::{OwnPython, PythonEdit};
    use kentos_python_env::extension::{self, Listed, Ready};
    use kentos_python_env::python::{InstallError, Interpreter, Progress};
    use kentos_python_env::uv::UvStatus;

    const MEBIS: &str = "tr.ornek.mebis";
    let root = temp("ortam");
    let python_root = temp("ortam-python");
    let mut gis = app(&root, Some(Module::Gis));
    gis.python_setup.root = Some(python_root.clone());
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = serde_json::json!([]);
        }),
    );
    let env = |app: &mut App, e: EnvEvent| {
        let _ = app.extensions_event(Event::Env(e));
    };
    let titles = |app: &App| -> Vec<String> {
        app.toasts
            .iter()
            .map(|(_, toast, _)| toast.title().to_owned())
            .collect()
    };
    let why = |app: &App| -> String {
        let tab = app
            .extension_tabs()
            .find(|t| t.extension == MEBIS)
            .expect("MEBİS's tab");
        match &tab.panels[0].items[0].action {
            Action::Unavailable(why) => why.clone(),
            other => panic!("{other:?}"),
        }
    };
    // Before KentOS's Python: the section says what comes first; nothing looked for.
    let _ = gis.extensions_event(Event::Choose(MEBIS.into()));
    assert!(!gis.extensions.envs.contains_key(MEBIS));
    let _ = gis.extensions_view();
    // KentOS's Python ready.
    let _ = gis.python_edit(PythonEdit::Enable(true));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/k/tools/uv".into(),
        version: "0.11.26".into(),
        bundled: true,
    }));
    let python = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python.clone()), 97_885_768))));
    assert!(matches!(
        gis.python_setup.python,
        Some(OwnPython::Ready(..))
    ));
    assert!(
        why(&gis).contains("Bu eklentinin Python ortamı hazırlanmadı: Eklentiler ›"),
        "{}",
        why(&gis)
    );
    // Looked for when the window opens on it, without a click: only a click looked, and until one the section said
    // "Ortam aranıyor…".
    gis.open_extensions();
    assert_eq!(gis.extensions.look_wanted.as_deref(), Some(MEBIS));
    let _ = gis.ext_look_tasks();
    assert_eq!(gis.extensions.look_wanted, None);
    assert_eq!(gis.extensions.envs[MEBIS].status, Some(EnvStatus::Looking));
    gis.extensions.envs.remove(MEBIS);
    // Looked for when chosen.
    let _ = gis.extensions_event(Event::Choose(MEBIS.into()));
    assert_eq!(gis.extensions.envs[MEBIS].status, Some(EnvStatus::Looking));
    env(&mut gis, EnvEvent::Found(MEBIS.into(), Ok(None)));
    assert_eq!(gis.extensions.envs[MEBIS].status, Some(EnvStatus::Missing));
    let _ = gis.extensions_view();
    // Its optional feature ticked; previewed from the lock.
    env(
        &mut gis,
        EnvEvent::Extra(MEBIS.into(), "charts".into(), true),
    );
    assert!(gis.extensions.envs[MEBIS].extras.contains("charts"));
    env(&mut gis, EnvEvent::Preview(MEBIS.into()));
    assert_eq!(gis.extensions.envs[MEBIS].busy, Some(EnvBusy::Previewing));
    let listed = vec![
        Listed {
            name: "numpy".into(),
            version: "2.5.4".into(),
        },
        Listed {
            name: "shapely".into(),
            version: "2.2.0".into(),
        },
    ];
    assert_eq!(
        super::environment::listed_words(&listed),
        "Kurulacak paketler (2): numpy 2.5.4, shapely 2.2.0."
    );
    env(
        &mut gis,
        EnvEvent::Previewed(MEBIS.into(), Ok(listed.clone())),
    );
    assert!(gis.extensions.envs[MEBIS].preview.is_some());
    let _ = gis.extensions_view();
    // Prepared on the press, with a bar; Durdur stops it.
    env(&mut gis, EnvEvent::Prepare(MEBIS.into()));
    assert_eq!(gis.extensions.envs[MEBIS].busy, Some(EnvBusy::Preparing));
    let stop = gis.extensions.envs[MEBIS].stop.clone().expect("preparing");
    env(
        &mut gis,
        EnvEvent::Progressed(
            MEBIS.into(),
            Progress::Downloading {
                done: 9_000_000,
                total: Some(17_000_000),
            },
        ),
    );
    let _ = gis.extensions_view();
    env(&mut gis, EnvEvent::Stop(MEBIS.into()));
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed));
    env(
        &mut gis,
        EnvEvent::Prepared(MEBIS.into(), Err(InstallError::Cancelled)),
    );
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("ortam hazırlama durduruldu"))
    );
    // Ready: made on disk as the module makes it, so the ribbon's words follow (files only, offline).
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(MEBIS))
        .unwrap();
    let key = extension::key_for(
        &package,
        "uv.lock",
        MEBIS,
        "1.0.0",
        &python,
        &["charts".into()],
    )
    .unwrap();
    let dir = extension::place(&python_root, &key);
    std::fs::create_dir_all(dir.join(".venv/bin")).unwrap();
    std::fs::write(dir.join(".venv/bin/python"), b"").unwrap();
    let ready = Ready {
        format: "kentos.extension-env".into(),
        key: key.clone(),
        python: dir.join(".venv/bin/python"),
        packages: listed,
        sdk: String::new(),
        made_at: 1,
    };
    std::fs::write(
        dir.join(extension::RECORD),
        serde_json::to_string(&ready).unwrap(),
    )
    .unwrap();
    env(&mut gis, EnvEvent::Prepare(MEBIS.into()));
    env(
        &mut gis,
        EnvEvent::Prepared(MEBIS.into(), Ok(Box::new(ready.clone()))),
    );
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("Python ortamı hazır"))
    );
    // Its command now runs (ADIM 19: an_extensions_command_runs_in_its_own_process_through_the_broker).
    let action = gis
        .extension_tabs()
        .find(|t| t.extension == MEBIS)
        .expect("MEBİS's tab")
        .panels[0]
        .items[0]
        .action
        .clone();
    assert!(
        matches!(action, Action::Python(ref e, ref k, ref l) if e == MEBIS && k == "command" && l == "init-schema"),
        "{action:?}"
    );
    env(
        &mut gis,
        EnvEvent::Found(MEBIS.into(), Ok(Some(Box::new(ready)))),
    );
    assert!(matches!(
        gis.extensions.envs[MEBIS].status,
        Some(EnvStatus::Ready(_))
    ));
    let _ = gis.extensions_view();
    // A failure is said in the section and by a notice that waits.
    env(&mut gis, EnvEvent::Prepare(MEBIS.into()));
    env(
        &mut gis,
        EnvEvent::Prepared(
            MEBIS.into(),
            Err(InstallError::Failed {
                cause: kentos_python_env::python::Cause::Network,
                said: "dns error".into(),
            }),
        ),
    );
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("Python ortamı hazırlanamadı"))
    );
    assert!(
        gis.extensions.envs[MEBIS]
            .note
            .as_deref()
            .is_some_and(|n| n.contains("internete ulaşılamadı"))
    );
    // Removed with the extension.
    assert!(dir.exists());
    let _ = gis.extensions_event(Event::Remove(MEBIS.into()));
    assert!(!dir.exists(), "its environments go with it");
    assert!(!gis.extensions.envs.contains_key(MEBIS));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&python_root);
}

/// Bir eklentinin komutu çalışırken (plan/eklenti.md ADIM 19): its ribbon button runs once its environment is ready;
/// its process is started with a hello and the call; the broker answers its reads on the open drawing and refuses
/// what it was not granted (İmar asked to read, not to write) and what is the user's (undo); its report comes back
/// as a notice and the messages; a failure is logged with its traceback; Durdur asks the code, then ends a process
/// that does not answer; a process that ends by itself is a crash. The process's lines are given here as it would
/// send them (its own side: python/tests/test_ext_host.py).
#[cfg(unix)]
#[test]
fn an_extensions_command_runs_in_its_own_process_through_the_broker() {
    use super::runner::row_words;
    use crate::python::host::Said;
    use crate::python_setup::PythonEdit;
    use kentos_python_env::extension::{self, Ready};
    use kentos_python_env::python::Interpreter;
    use kentos_python_env::uv::UvStatus;
    use serde_json::json;

    const IMAR: &str = "tr.ornek.imar-denetim";
    let root = temp("yurutucu");
    let python_root = temp("yurutucu-python");
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&root, false).0;
    gis.extensions_changed();
    gis.python_setup.root = Some(python_root.clone());
    install(&mut gis, "imar.kek", package("ornek-imar", |_| {}));
    let _ = gis.python_edit(PythonEdit::Enable(true));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/k/tools/uv".into(),
        version: "0.11.26".into(),
        bundled: true,
    }));
    let python = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python.clone()), 1))));
    let button = |app: &App| {
        app.extension_tabs()
            .find(|t| t.extension == IMAR)
            .expect("İmar's tab")
            .panels[0]
            .items[0]
            .action
            .clone()
    };
    assert!(
        matches!(button(&gis), Action::Unavailable(_)),
        "no environment yet"
    );
    // Its environment ready (a record on disk; its Python anything that starts: its lines are given here).
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(IMAR))
        .unwrap();
    let key = extension::key_for(&package, "uv.lock", IMAR, "1.1.0", &python, &[]).unwrap();
    let dir = extension::place(&python_root, &key);
    std::fs::create_dir_all(&dir).unwrap();
    let ready = Ready {
        format: "kentos.extension-env".into(),
        key,
        python: "/bin/sh".into(),
        packages: Vec::new(),
        sdk: String::new(),
        made_at: 1,
    };
    std::fs::write(
        dir.join(extension::RECORD),
        serde_json::to_string(&ready).unwrap(),
    )
    .unwrap();
    gis.extensions_changed();
    assert!(
        matches!(button(&gis), Action::Python(ref e, ref k, ref l) if e == IMAR && k == "command" && l == "check-setbacks"),
        "{:?}",
        button(&gis)
    );
    // Pressed: the environment found in the background, then the process started and the call sent.
    let launch = super::runner::Launch::for_test(
        "/bin/sh".into(),
        package.clone(),
        "imar.logic:check_setbacks",
        "Yaklaşma mesafelerini denetle",
        false,
    );
    let _ = gis.extensions_event(Event::Launch(
        IMAR.into(),
        "command".into(),
        "check-setbacks".into(),
        Ok(launch.clone()),
    ));
    let generation = gis.extensions.runners[IMAR].generation;
    let run = gis.extensions.runners[IMAR]
        .run
        .as_ref()
        .expect("running")
        .id;
    let say = |app: &mut App, m: serde_json::Value| {
        let _ = app.extensions_event(Event::Runner(IMAR.into(), generation, Said::Message(m)));
    };
    let titles = |app: &App| -> Vec<String> {
        app.toasts
            .iter()
            .map(|(_, toast, _)| toast.title().to_owned())
            .collect()
    };
    assert!(
        titles(&gis).iter().any(|t| t.ends_with("çalışıyor")),
        "{:?}",
        titles(&gis)
    );
    say(&mut gis, json!({"type": "ready", "protocol": 1}));
    assert!(gis.extensions.runners[IMAR].ready);
    // The broker: reads answered on the open drawing; a write not granted, and the user's undo, refused.
    let summary = gis.ext_answer_for_test(IMAR, "summary", &json!({}));
    assert!(summary.is_ok(), "{summary:?}");
    let layers = gis.ext_answer_for_test(IMAR, "layers", &json!({}));
    assert!(layers.is_ok(), "{layers:?}");
    let write = gis
        .ext_answer_for_test(
            IMAR,
            "run",
            &json!({"command": "cad.polygon.create", "op": "execute", "input": {}}),
        )
        .unwrap_err();
    assert_eq!(write.code, "capability");
    assert!(write.message.contains("document.write"));
    let undo = gis
        .ext_answer_for_test(IMAR, "undo", &json!({}))
        .unwrap_err();
    assert_eq!(undo.code, "not_for_extensions");
    // Its report: a notice and the rows in the messages.
    say(
        &mut gis,
        json!({"type": "progress", "id": run, "done": 1, "total": 3}),
    );
    say(
        &mut gis,
        json!({"type": "log", "id": run, "level": "info", "text": "3 yapı denetlendi"}),
    );
    say(
        &mut gis,
        json!({"type": "done", "id": run, "ok": true, "result": {
            "kind": "report", "title": "Yaklaşma mesafesi denetimi", "message": "3 yapı denetlendi; 1 yapı sınırın içinde.",
            "rows": [{"yapı": "B1", "mesafe_m": 2.0}]
        }}),
    );
    assert!(gis.extensions.runners[IMAR].run.is_none());
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("Yaklaşma mesafesi denetimi")),
        "{:?}",
        titles(&gis)
    );
    assert_eq!(
        gis.extensions
            .result
            .as_ref()
            .map(|(e, t, _)| (e.as_str(), t.as_str())),
        Some((IMAR, "Yaklaşma mesafesi denetimi"))
    );
    // Its columns in the order it gave them (a JSON object's keys come sorted).
    assert_eq!(
        row_words(
            &["yapı", "mesafe_m"],
            &json!({"yapı": "B1", "mesafe_m": 2.0, "not": "x"})
        ),
        "yapı: B1 · mesafe_m: 2.0 · not: x"
    );
    let logged = |app: &App, code: &str| {
        app.extensions
            .registry
            .as_ref()
            .unwrap()
            .log_of(IMAR)
            .iter()
            .any(|r| r.code == code)
    };
    assert!(logged(&gis, "run_started") && logged(&gis, "run_log") && logged(&gis, "run_done"));
    // A failure: logged with its traceback, said by a notice naming the extension.
    let _ = gis.extensions_event(Event::Launch(
        IMAR.into(),
        "command".into(),
        "check-setbacks".into(),
        Ok(launch.clone()),
    ));
    let run = gis.extensions.runners[IMAR].run.as_ref().unwrap().id;
    say(
        &mut gis,
        json!({"type": "done", "id": run, "ok": false, "exception": "ZeroDivisionError", "message": "division by zero",
               "error": "Traceback (most recent call last):\n  File \"logic.py\", line 3\nZeroDivisionError: division by zero"}),
    );
    let failed = gis
        .extensions
        .registry
        .as_ref()
        .unwrap()
        .log_of(IMAR)
        .iter()
        .rev()
        .find(|r| r.code == "run_failed")
        .cloned()
        .expect("logged");
    assert!(failed.message.contains("ZeroDivisionError"));
    assert!(
        failed
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("Traceback"))
    );
    // Said where its start was said, with the way to its log.
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("“Yaklaşma mesafelerini denetle” hata verdi")),
        "{:?}",
        titles(&gis)
    );
    // The messages' line points at where it failed, not at the traceback's first line.
    assert!(
        gis.log
            .lines()
            .any(|l| l.text.contains("[logic.py:3]") && l.text.contains("ZeroDivisionError")),
        "{:?}",
        gis.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
    // Ayrıntı: the manager on it, its environment looked for as a click on its row would.
    let _ = gis.extensions_event(Event::Show(IMAR.into()));
    assert_eq!(gis.extensions.chosen.as_deref(), Some(IMAR));
    assert!(gis.extensions.envs.contains_key(IMAR), "looked for");
    // Unused for a while: its process closed (D6); the next call starts a new one.
    let generation = gis.extensions.runners[IMAR].generation;
    let _ = gis.extensions_event(Event::RunnerIdle(IMAR.into(), generation));
    assert!(
        gis.extensions.runners.contains_key(IMAR),
        "not unused long enough"
    );
    let long_ago = std::time::Instant::now().checked_sub(kentos_python_env::process::IDLE);
    gis.extensions.runners.get_mut(IMAR).unwrap().ended = long_ago;
    let _ = gis.extensions_event(Event::RunnerIdle(IMAR.into(), generation));
    assert!(
        !gis.extensions.runners.contains_key(IMAR),
        "closed when unused"
    );
    assert!(logged(&gis, "runner_idle"));
    // Durdur: asked of the code; a process that does not answer is ended.
    let _ = gis.extensions_event(Event::Launch(
        IMAR.into(),
        "command".into(),
        "check-setbacks".into(),
        Ok(launch.clone()),
    ));
    let run = gis.extensions.runners[IMAR].run.as_ref().unwrap().id;
    let _ = gis.extensions_event(Event::StopRun(IMAR.into()));
    assert!(
        gis.extensions.runners[IMAR]
            .run
            .as_ref()
            .unwrap()
            .stopping
            .is_some()
    );
    let _ = gis.extensions_event(Event::RunOverdue(IMAR.into(), run));
    assert!(
        !gis.extensions.runners.contains_key(IMAR),
        "its process ended"
    );
    assert!(titles(&gis).iter().any(|t| t.ends_with("durduruldu")));
    // A process that ends by itself during a run: a crash, logged.
    let _ = gis.extensions_event(Event::Launch(
        IMAR.into(),
        "command".into(),
        "check-setbacks".into(),
        Ok(launch),
    ));
    let generation = gis.extensions.runners[IMAR].generation;
    let _ = gis.extensions_event(Event::Runner(IMAR.into(), generation, Said::Closed));
    assert!(!gis.extensions.runners.contains_key(IMAR));
    assert!(logged(&gis, "runner_crashed"));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&python_root);
}

/// Bir eklenti çizime yazarken (ADIM 19): its run is one undo step named after its command, kept when it ends well and
/// taken back when it fails or is stopped; meanwhile the drawing takes no other edit (the user is told once), and
/// neither the console, an agent nor another writing run goes; an ability this KentOS lacks is said calmly.
#[cfg(unix)]
#[test]
fn an_extensions_writes_are_one_undo_step_and_the_drawing_waits_for_them() {
    use crate::python::host::Said;
    use serde_json::json;

    const MEBIS: &str = "tr.ornek.mebis";
    let root = temp("yazan");
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&root, false).0;
    gis.extensions_changed();
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = json!([]);
        }),
    );
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(MEBIS))
        .unwrap();
    let launch = super::runner::Launch::for_test(
        "/bin/sh".into(),
        package,
        "mebis.logic:init_schema",
        "MEBİS katmanlarını kur",
        true,
    );
    let start = |app: &mut App| {
        let _ = app.extensions_event(Event::Launch(
            MEBIS.into(),
            "command".into(),
            "init-schema".into(),
            Ok(launch.clone()),
        ));
        let runner = &app.extensions.runners[MEBIS];
        (runner.generation, runner.run.as_ref().expect("running").id)
    };
    let say = |app: &mut App, generation: u64, m: serde_json::Value| {
        let _ = app.extensions_event(Event::Runner(MEBIS.into(), generation, Said::Message(m)));
    };
    let add_layer = |app: &mut App, name: &str| {
        app.ext_answer_for_test(
            MEBIS,
            "run",
            &json!({"command": "cad.layers.edit", "op": "execute",
                    "input": {"operation": "add", "name": name, "geometry": "area"}}),
        )
    };
    let has_layer = |app: &App, name: &str| {
        app.document
            .as_ref()
            .unwrap()
            .model
            .layers()
            .nodes()
            .iter()
            .any(|n| n.name == name)
    };
    let titles = |app: &App| -> Vec<String> {
        app.toasts
            .iter()
            .map(|(_, toast, _)| toast.title().to_owned())
            .collect()
    };
    // Running: the drawing waits for it, said once; the console and agents wait too.
    let (generation, run) = start(&mut gis);
    assert_eq!(
        gis.extension_writing().as_deref(),
        Some("Mezarlık Bilgi Sistemi (MEBİS): “MEBİS katmanlarını kur”")
    );
    let before = gis.log.lines().count();
    assert!(
        gis.while_extension_writes(&Message::CommandSubmitted)
            .is_some()
    );
    assert!(
        gis.while_extension_writes(&Message::CommandSubmitted)
            .is_some()
    );
    let said: Vec<String> = gis
        .log
        .lines()
        .skip(before)
        .map(|l| l.text.clone())
        .collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("düzenlenemez"));
    // A command typed meanwhile waits too (found in the real window, 14 November 2026: Çizgi started).
    let tool = gis.session.tool_id().to_owned();
    let _ = gis.update(Message::CommandInput("CIZGI".into()));
    let _ = gis.update(Message::CommandSubmitted);
    assert_eq!(gis.session.tool_id(), tool, "no command starts");
    let busy = gis
        .agent_request(&json!({"method": "summary"}), "Ajan")
        .unwrap_err();
    assert_eq!(busy.0, "busy");
    assert!(busy.1.contains("MEBİS katmanlarını kur"));
    let _ = gis.python_run_titled("print(1)".into(), "deneme.py".into());
    assert!(gis.python.running.is_none(), "the console waits");
    // Its writes: one undo step named after the command, kept when it ends well.
    add_layer(&mut gis, "mezar_ada").expect("written");
    add_layer(&mut gis, "mezar_yeri").expect("written");
    say(
        &mut gis,
        generation,
        json!({"type": "done", "id": run, "ok": true, "result": {"kind": "message", "message": "Kuruldu."}}),
    );
    assert!(gis.extension_writing().is_none());
    assert!(
        gis.while_extension_writes(&Message::CommandSubmitted)
            .is_none()
    );
    assert!(has_layer(&gis, "mezar_ada") && has_layer(&gis, "mezar_yeri"));
    let doc = gis.document.as_mut().unwrap();
    assert_eq!(
        doc.model.undo_labels().first().map(String::as_str),
        Some("MEBİS katmanlarını kur")
    );
    doc.model.undo();
    assert!(
        !has_layer(&gis, "mezar_ada") && !has_layer(&gis, "mezar_yeri"),
        "one step took both back"
    );
    // A failure takes back what it wrote.
    let (generation, run) = start(&mut gis);
    add_layer(&mut gis, "mezar_ada").expect("written");
    say(
        &mut gis,
        generation,
        json!({"type": "done", "id": run, "ok": false, "exception": "KeyError", "message": "x", "error": "Traceback …"}),
    );
    assert!(!has_layer(&gis, "mezar_ada"), "taken back");
    // Durdur, answered by the code: taken back.
    let (generation, run) = start(&mut gis);
    add_layer(&mut gis, "mezar_ada").expect("written");
    let _ = gis.extensions_event(Event::StopRun(MEBIS.into()));
    say(
        &mut gis,
        generation,
        json!({"type": "done", "id": run, "ok": false, "stopped": true}),
    );
    assert!(!has_layer(&gis, "mezar_ada"), "taken back");
    assert!(titles(&gis).iter().any(|t| t.ends_with("durduruldu")));
    // An ability this KentOS lacks: said calmly, logged as a warning; what it wrote taken back.
    let (generation, run) = start(&mut gis);
    add_layer(&mut gis, "mezar_ada").expect("written");
    say(
        &mut gis,
        generation,
        json!({"type": "done", "id": run, "ok": false, "unavailable": true,
               "message": "Katman şeması kurulumu (schema.json) bu KentOS sürümünde yok."}),
    );
    assert!(!has_layer(&gis, "mezar_ada"), "taken back");
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("bu sürümde çalışmaz")),
        "{:?}",
        titles(&gis)
    );
    let unavailable = gis
        .extensions
        .registry
        .as_ref()
        .unwrap()
        .log_of(MEBIS)
        .iter()
        .rev()
        .find(|r| r.code == "run_unavailable")
        .cloned()
        .expect("logged");
    assert!(unavailable.detail.is_none(), "no traceback");
    // Another writing run waits for one that writes.
    let (_generation, _run) = start(&mut gis);
    let before = gis.extensions.run_counter;
    let _ = gis.extensions_event(Event::Launch(
        MEBIS.into(),
        "command".into(),
        "init-schema".into(),
        Ok(launch.clone()),
    ));
    assert_eq!(gis.extensions.run_counter, before, "not started twice");
    let _ = std::fs::remove_dir_all(&root);
}

/// An extension's process writes no bytecode beside its package (whose integrity check refuses added files): its
/// interpreter's arguments send it to the environment's folder (real window, 14 November 2026).
#[test]
fn an_extensions_bytecode_goes_to_its_environment_not_its_package() {
    let args = super::runner::interpreter_args(std::path::Path::new("/k/ortamlar/ab12/pyc"));
    let args: Vec<String> = args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        args,
        [
            "-I",
            "-X",
            "pycache_prefix=/k/ortamlar/ab12/pyc",
            "-m",
            "kentos.ext_host"
        ]
    );
}

#[test]
fn a_tool_the_desktop_would_not_make_is_refused_before_kur() {
    // plan/eklenti.md ADIM 19, "tanımın kurulumda tam denetimi": the package's own check reads the descriptor's format,
    // version and id; a parameter Python cannot name (`class`) passed it, the install said "kuruldu" and the tool was
    // found out only when it was listed, in a log line. The review now asks the desktop's own tool check
    // (`kentos_processing::external::from_descriptor`) and says it before Kur.
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/extensions/v1/ornek-imar");
    let mut files = kentos_extension_host::store::read_files(&root).unwrap();
    let mut tool: serde_json::Value =
        serde_json::from_slice(&files["tools/taks.tool.json"]).unwrap();
    tool["parameters"][0]["name"] = serde_json::json!("class");
    files.insert(
        "tools/taks.tool.json".into(),
        serde_json::to_vec_pretty(&tool).unwrap(),
    );
    let bytes = kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("the package's own check passes it: {p:#?}"))
        .bytes;
    let dir = temp("arac-tanimi");
    // Not `app`: its extensions_changed writes the process's functions, which other tests read.
    let (mut gis, _) = App::boot(None);
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&dir, false).0;
    gis.open_extensions();
    let _ = gis.extensions_event(Event::Picked(Some(("imar.kek".to_owned(), bytes.clone()))));
    let review = gis.extensions.review.as_ref().expect("reviewed");
    let problem = review
        .review
        .inspection
        .problems
        .iter()
        .find(|p| p.code == "tool_descriptor")
        .unwrap_or_else(|| panic!("{:#?}", review.review.inspection.problems));
    assert_eq!(problem.file.as_deref(), Some("tools/taks.tool.json"));
    assert!(
        problem.message.contains("“taks” aracı") && problem.message.contains("class"),
        "{}",
        problem.message
    );
    assert!(!review.review.installable, "not installable");
    // Installing the bytes anyway (another way in) is refused the same.
    let registry = gis.extensions.registry.as_mut().expect("a registry");
    let source = kentos_extension_host::store::Source {
        kind: kentos_extension_host::store::SourceKind::File,
        location: "imar.kek".to_owned(),
    };
    let refused = registry
        .install(&bytes, &source, &kentos_extension_host::Choice::default())
        .expect_err("refused");
    assert_eq!(refused[0].code, "tool_descriptor");
}

/// The demo extension's version, as its manifest says (the tests follow its releases).
fn vitrin_version() -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/eklenti-vitrini/eklenti/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
    manifest["version"].as_str().unwrap().to_owned()
}

/// samples/eklenti-vitrini's extension packed from its source, as `hazirla.sh` packs it.
fn vitrin_package() -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/eklenti");
    let files: std::collections::BTreeMap<String, Vec<u8>> =
        kentos_extension_host::store::read_files(&root)
            .unwrap()
            .into_iter()
            .filter(|(path, _)| !path.contains("__pycache__"))
            .collect();
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

#[test]
fn the_demo_extension_installs_with_every_contribution_on_its_tab() {
    // samples/eklenti-vitrini: the extension every demo uses (the owner, 14 November 2026). The review asks the
    // desktop's own tool check (its TAKS tool passes), and its tab shows each of its Python contributions.
    let dir = temp("vitrin");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let tab = gis
        .extension_tabs()
        .find(|t| t.extension == "tr.ornek.vitrin")
        .expect("Vitrin's tab");
    let labels: Vec<String> = tab
        .panels
        .iter()
        .flat_map(|p| p.items.iter().map(|i| i.label.clone()))
        .collect();
    assert_eq!(
        labels,
        [
            "Katman özeti",
            "Yapıları numarala",
            "Hata örneği",
            "Katmanları kur",
            "TAKS",
            "Yapı numarası",
            "Yapı yerleştir"
        ]
    );
}

#[test]
fn a_forms_action_opens_an_extensions_tool_pinned_and_says_why_when_it_cannot() {
    // docs/adr/8057 §12, FORM-API-13: a form's button on Vitrin's TAKS tool. Missing, its code not runnable yet or
    // turned off, the button is off and says so; the designer pins the extension and says when it drifts.
    use crate::extensions::PinnedTool;
    use crate::form_scenes::{designer, entry, meters_rebodied};
    use crate::forms::{designer as d, entry as en};
    use kentos_commands::{ExtensionPin, FormItem};
    const TOOL: &str = "ext.tr.ornek.vitrin.taks";
    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("form-eylemi");
    let mut gis = app(&dir, Some(Module::Gis));
    meters_rebodied(
        &mut gis,
        serde_json::json!([
            { "type": "field", "id": "ad", "field": "Ad" },
            { "type": "action", "id": "taks", "label": "TAKS hesapla", "tool": TOOL }
        ]),
    );
    let unavailable = |state: PinnedTool| match state {
        PinnedTool::Unavailable(why) => why,
        other => panic!("{other:?}"),
    };
    let stale = ExtensionPin {
        extension: VITRIN.into(),
        version: "0.9.0".into(),
        digest: format!("sha256:{}", "0".repeat(64)),
    };
    // Not installed: which extension, and with a pin which version the form expects.
    let why = unavailable(gis.pinned_tool_state(TOOL, None));
    assert!(
        why.contains("tr.ornek.vitrin eklentisi bu bilgisayarda kurulu değil"),
        "{why}"
    );
    let why = unavailable(gis.pinned_tool_state(TOOL, Some(&stale)));
    assert!(
        why.contains("tr.ornek.vitrin 0.9.0 bu bilgisayarda kurulu değil"),
        "{why}"
    );
    let why = unavailable(gis.pinned_tool_state("vitrin.yok", None));
    assert!(why.contains("kayıtlı bir işlem aracı değil"), "{why}");
    // Installed, its Python not ready (no Python in tests): pending, the press opens nothing.
    install(&mut gis, "vitrin.kek", vitrin_package());
    let pending = gis.pinned_tool_state(TOOL, None);
    assert!(matches!(pending, PinnedTool::Pending(_)), "{pending:?}");
    gis.selection.set([kentos_domain::Slot(1)]);
    crate::tools_scenes::run(&mut gis, "data.formEdit");
    assert!(gis.gis.form_entry.is_some(), "the form");
    entry(&mut gis, en::Event::RunAction("taks".into()));
    assert!(
        gis.gis.processing.dialog.is_none(),
        "pending: nothing opened"
    );
    // Its code can run: the press opens the tool on the record.
    gis.extensions.python_reasons.insert(VITRIN.into(), None);
    assert_eq!(gis.pinned_tool_state(TOOL, None), PinnedTool::Ready);
    entry(&mut gis, en::Event::RunAction("taks".into()));
    assert_eq!(
        gis.gis
            .processing
            .dialog
            .as_ref()
            .map(|d| d.tool.id.as_str()),
        Some(TOOL),
        "the tool's window"
    );
    assert_eq!(gis.selection.ids(), [kentos_domain::Slot(1)]);
    let values = &gis.gis.processing.dialog.as_ref().unwrap().values;
    assert_eq!(
        values["parseller"],
        serde_json::json!({ "scope": "selection" }),
        "the record is the tool's input"
    );
    assert_eq!(
        values["yapilar"]["layerId"], "yapi",
        "its other input as it was"
    );
    // The pin: the installed version and digest; another version, another build or none is said.
    let pin = gis.tool_pin_of(TOOL).expect("installed");
    assert_eq!(pin.version, vitrin_version());
    assert!(pin.digest.starts_with("sha256:") && pin.digest.len() == 71);
    assert_eq!(gis.tool_pin_drift(TOOL, Some(&pin)), None);
    let drift = gis
        .tool_pin_drift(TOOL, Some(&stale))
        .expect("another version");
    assert!(
        drift.contains(&format!(
            "0.9.0 ile seçildi, kurulu sürüm {}",
            vitrin_version()
        )),
        "{drift}"
    );
    let rebuilt = ExtensionPin {
        digest: format!("sha256:{}", "1".repeat(64)),
        ..pin.clone()
    };
    let drift = gis
        .tool_pin_drift(TOOL, Some(&rebuilt))
        .expect("another build");
    assert!(drift.contains("içeriği farklı"), "{drift}");
    let drift = gis.tool_pin_drift(TOOL, None).expect("not pinned");
    assert!(drift.contains("sabitlenmemiş"), "{drift}");
    assert_eq!(gis.tool_pin_drift("attributes.calculate", None), None);
    // The designer: the preflight says the missing pin; choosing the tool pins it, the warning goes.
    crate::tools_scenes::run(&mut gis, "data.forms");
    designer(&mut gis, d::Event::SelectItem("taks".into()));
    let fields = gis.layer_fields_of("bina");
    let codes = |gis: &App| -> Vec<String> {
        let body = gis.gis.forms_window.as_ref().unwrap().body.clone().unwrap();
        gis.issues_of(&body, &fields)
            .into_iter()
            .map(|i| i.code)
            .collect()
    };
    assert!(codes(&gis).contains(&kentos_forms::codes::ACTION_PIN.to_owned()));
    designer(&mut gis, d::Event::PinnedTool(TOOL.into()));
    let body = gis.gis.forms_window.as_ref().unwrap().body.clone().unwrap();
    let pinned = body.walk().into_iter().find_map(|(_, i)| match i {
        FormItem::Action { pin, .. } => pin.clone(),
        _ => None,
    });
    assert_eq!(pinned, Some(pin));
    assert!(!codes(&gis).contains(&kentos_forms::codes::ACTION_PIN.to_owned()));
    // Turned off: unavailable with the extension's state; the preflight says the button is off.
    let _ = gis.extensions_event(Event::Disable(VITRIN.into()));
    let why = unavailable(gis.pinned_tool_state(TOOL, None));
    assert!(why.contains("Eklenti Vitrini — Devre dışı"), "{why}");
    assert!(codes(&gis).contains(&kentos_forms::codes::ACTION_UNKNOWN.to_owned()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_demo_drawings_form_pins_the_demo_extension_as_it_packs_now() {
    // samples/eklenti-vitrini/vitrin.kcad's Parsel form opens Vitrin's TAKS tool pinned to the package's digest
    // (docs/adr/8057 §12): a package changed without the drawing written again is said here, not in a demo.
    use kentos_commands::FormItem;
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/vitrin.kcad");
    let snapshot = kentos_kcad::decode(&std::fs::read(path).unwrap()).expect("the demo drawing");
    let form = snapshot
        .settings
        .forms
        .iter()
        .find(|f| f.id == "parsel")
        .expect("the Parsel form");
    let pin = form
        .draft
        .walk()
        .into_iter()
        .find_map(|(_, i)| match i {
            FormItem::Action {
                tool: Some(tool),
                pin,
                ..
            } if tool == "ext.tr.ornek.vitrin.taks" => pin.clone(),
            _ => None,
        })
        .expect("its TAKS button, pinned");
    let dir = temp("vitrin-cizim");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    assert_eq!(
        gis.tool_pin_drift("ext.tr.ornek.vitrin.taks", Some(&pin)),
        None,
        "vitrin.kcad's TAKS button is pinned to another package: run `.run/py/bin/python \
         samples/eklenti-vitrini/vitrin_cizim.py`"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_models_step_on_an_extensions_tool_is_pinned_and_says_where_it_stands() {
    // docs/adr/8057 §13, PY-TOOL-20: Model tasarımcısı adds Vitrin's TAKS pinned to the extension as installed; the
    // step's line says the version (another one too), pending without Python, unavailable once the extension is off.
    use crate::processing::designer::Event as Designer;
    use kentos_commands::ExtensionPin;
    const TOOL: &str = "ext.tr.ornek.vitrin.taks";
    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("model-sabitleme");
    // A drawing open (the designer asks for one), the extensions apart from the user's own.
    let mut gis = crate::processing::tests::app_with_parcels();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&dir, false).0;
    gis.extensions_changed();
    install(&mut gis, "vitrin.kek", vitrin_package());
    let _ = gis.update(Message::Run("processing.newModel"));
    let _ = gis.update(Message::Gis(crate::gis::Event::ModelDesigner(
        Designer::AddTool {
            tool: TOOL.into(),
            at: None,
        },
    )));
    let step = gis
        .gis
        .processing
        .designer
        .as_deref()
        .expect("the designer")
        .draft
        .steps[0]
        .clone();
    let pin = step.pin.clone().expect("pinned");
    assert_eq!(
        (pin.extension.as_str(), pin.version.as_str()),
        (VITRIN, vitrin_version().as_str())
    );
    assert_eq!(
        Some(pin.clone()),
        gis.tool_pin_of(TOOL),
        "the installed package's"
    );
    let note = gis.step_pin_note(TOOL, Some(&pin)).expect("a line");
    assert!(note.starts_with("Bekliyor: "), "{note}");
    gis.extensions.python_reasons.insert(VITRIN.into(), None);
    assert_eq!(
        gis.step_pin_note(TOOL, Some(&pin)).as_deref(),
        Some(format!("Eklenti Vitrini {} ile eklendi.", vitrin_version()).as_str())
    );
    let older = ExtensionPin {
        version: "1.0.0".into(),
        ..pin.clone()
    };
    let note = gis.step_pin_note(TOOL, Some(&older)).expect("a line");
    assert!(
        note.contains(&format!(
            "1.0.0 ile eklendi; kurulu sürüm {}",
            vitrin_version()
        )),
        "{note}"
    );
    assert_eq!(gis.step_pin_note("attributes.calculate", None), None);
    // The inspector draws it.
    gis.gis.processing.designer.as_deref_mut().unwrap().selected = Some(
        kentos_processing::model_edit::NodeRef::Step(step.id.clone()),
    );
    let _ = gis.model_designer_view();
    // Turned off: unavailable with the extension's state.
    let _ = gis.extensions_event(Event::Disable(VITRIN.into()));
    let note = gis.step_pin_note(TOOL, Some(&pin)).expect("a line");
    assert!(
        note.starts_with("Kullanılamaz: Eklenti Vitrini — Devre dışı"),
        "{note}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_extension_runs_another_extensions_tool_only_when_it_depends_on_it() {
    // PY-RUN-15 (docs/adr/8057 §16): through the broker an extension's code may run its own tools and those of the
    // extensions its manifest depends on; another's is refused with what to write.
    let dir = temp("eklentiden-eklentiye");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let run = |tool: &str| {
        serde_json::json!({
            "command": "cad.processing.run",
            "input": { "tool": tool, "parameters": {} },
            "op": "execute",
        })
    };
    let taks = run("ext.tr.ornek.vitrin.taks");
    // MEBİS without the dependency: refused, saying where to declare it.
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = serde_json::json!([]);
        }),
    );
    let why = gis.calls_refused("tr.ornek.mebis", &taks).expect("refused");
    assert!(
        why.contains("“tr.ornek.vitrin”") && why.contains("dependencies.extensions"),
        "{why}"
    );
    // Its own tools and calls that run no extension's tool pass.
    assert_eq!(gis.calls_refused("tr.ornek.vitrin", &taks), None);
    assert_eq!(
        gis.calls_refused("tr.ornek.mebis", &run("attributes.calculate")),
        None
    );
    let entities = serde_json::json!({ "command": "cad.entities.set", "input": {} });
    assert_eq!(gis.calls_refused("tr.ornek.mebis", &entities), None);
    // With the dependency declared (the registry keeps Vitrin installed, enabled and of a fitting version): it runs.
    let _ = gis.extensions_event(Event::Remove("tr.ornek.mebis".into()));
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] =
                serde_json::json!([{ "id": "tr.ornek.vitrin", "version": ">=1.0" }]);
        }),
    );
    assert_eq!(gis.calls_refused("tr.ornek.mebis", &taks), None);
    let batch = serde_json::json!({
        "command": "cad.processing.batch",
        "input": { "tool": "ext.tr.ornek.vitrin.taks", "rows": [] },
    });
    assert_eq!(gis.calls_refused("tr.ornek.mebis", &batch), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_audit_rules_findings_are_listed_shown_and_fixed_as_one_step() {
    // docs/adr/8057 §17 (plan ADIM 20): the rule's answer read into Eklenti denetimi; Göster selects the object, Düzelt
    // writes the rule's fix as one undo step named by it; a fix of an extension not allowed to write is refused.
    use super::audits::{AuditDone, AuditState, Severity, findings_of};
    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("denetim");
    let mut gis = crate::processing::tests::app_with_parcels();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&dir, false).0;
    gis.extensions_changed();
    install(&mut gis, "vitrin.kek", vitrin_package());
    let doc = gis.document.as_ref().unwrap();
    let slot = doc
        .model
        .entities()
        .next()
        .map(|e| kentos_domain::Slot(e.base().id))
        .unwrap();
    let uid = doc.model.uid(slot).unwrap().to_string();
    let session = doc.session;
    let done = serde_json::json!({
        "ok": true,
        "result": { "kind": "findings", "findings": [
            { "severity": "warning", "message": "Yapının numarası yok; sıradaki numara Y-01.", "featureId": uid,
              "fix": { "title": "Y-01 yaz", "command": "cad.entities.set",
                       "args": { "uids": [uid], "operation": "attributes", "attrs": { "no": "Y-01" } } } },
            { "severity": "info", "message": "Çizimin yalnız ilk nesneleri denetlendi." },
            { "severity": "ölümcül", "message": "okunmaz" }
        ] }
    });
    let (findings, dropped) = findings_of(&done).expect("read");
    assert_eq!(
        (findings.len(), dropped),
        (2, 0),
        "an unknown severity is left out"
    );
    assert_eq!(findings[0].severity, Severity::Warning);
    let failed = findings_of(
        &serde_json::json!({ "ok": false, "exception": "TypeError", "message": "liste değil",
                                                 "error": "Traceback…" }),
    );
    assert_eq!(failed.unwrap_err().0, "TypeError: liste değil");
    // The run ends: its findings in the tab.
    gis.extensions.audit = Some(AuditState {
        ext: VITRIN.into(),
        local: "yapi-no".into(),
        title: "Yapı numarası".into(),
        running: Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
            false,
        ))),
        findings: Vec::new(),
        dropped: 0,
        cut: false,
        ms: 0,
        error: None,
        session,
    });
    gis.ext_audit_done(AuditDone {
        ext: VITRIN.into(),
        local: "yapi-no".into(),
        answer: Ok(done),
        ms: 12,
    });
    let state = gis.extensions.audit.as_ref().unwrap();
    assert!(state.running.is_none() && state.error.is_none());
    assert_eq!(state.findings.len(), 2);
    let _ = gis.ext_audit_tab();
    // Göster: the object selected.
    let _ = gis.ext_audit_show(0);
    assert_eq!(gis.selection.ids(), [slot]);
    // Düzelt: one undo step named by the fix; undone, the attribute goes.
    assert_eq!(gis.ext_audit_fix_refused(0), None);
    gis.ext_audit_fix(0);
    let attr = |gis: &App| {
        gis.document
            .as_ref()
            .unwrap()
            .model
            .get(slot)
            .unwrap()
            .base()
            .attrs
            .get("no")
            .cloned()
    };
    assert_eq!(attr(&gis).as_deref(), Some("Y-01"));
    assert!(gis.extensions.audit.as_ref().unwrap().findings[0].fixed);
    assert_eq!(gis.ext_audit_fix_refused(0).as_deref(), Some("Düzeltildi."));
    let undone = gis.document.as_mut().unwrap().model.undo();
    assert_eq!(undone.as_deref(), Some("Y-01 yaz"));
    assert_eq!(attr(&gis), None);
    // An extension not allowed to write: its fix is refused with why.
    gis.extensions.audit.as_mut().unwrap().findings[0].fixed = false;
    gis.extensions.audit.as_mut().unwrap().ext = "tr.ornek.yok".into();
    let why = gis.ext_audit_fix_refused(0).expect("refused");
    assert!(why.contains("document.write"), "{why}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_blueprint_installs_with_every_contribution_and_passes_the_desktops_own_checks() {
    // docs/adr/8057 §19: what `extension.blueprint` gives packs, passes the desktop's review (its tool check
    // included) and installs, its tab holding each example.
    use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
    let files = blueprint(&Ask {
        id: "tr.belediye.mezarlik-bilgi".into(),
        name: "Mezarlık bilgi".into(),
        summary: "Mezarlık adalarını ve mezar yerlerini yönetir.".into(),
        vertical: true,
        module: kentos_extensions::manifest::Module::Cbs,
        contributions: Contribution::ALL.to_vec(),
    })
    .unwrap();
    let files: std::collections::BTreeMap<String, Vec<u8>> = files
        .into_iter()
        .map(|(k, v)| (k, v.into_bytes()))
        .collect();
    let bytes = kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes;
    let dir = temp("iskelet");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "iskelet.kek", bytes);
    let tab = gis
        .extension_tabs()
        .find(|t| t.extension == "tr.belediye.mezarlik-bilgi")
        .expect("its tab");
    let labels: Vec<String> = tab
        .panels
        .iter()
        .flat_map(|p| p.items.iter().map(|i| i.label.clone()))
        .collect();
    assert_eq!(
        labels,
        [
            "Katman listesi",
            "Alan özeti",
            "Adsız nesneler",
            "Katmanları kur",
            "Alan çiz"
        ],
        "with a schema, its command; the map tool draws into its layer (docs/adr/8057 §23)"
    );
    assert!(
        gis.gis
            .processing
            .registry
            .get("ext.tr.belediye.mezarlik-bilgi.alan-ozeti")
            .is_some(),
        "its İşlemler tool is listed"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
