//! The user's keys and names (keymap.rs, settings_keys.rs, docs/adr/9308): a key recorded in Uygulama ayarları ›
//! Kısayollar runs its command after Kaydet, no restart; a key another command has is said with that command and
//! moved only on Taşı; the keys KentOS and the operating system keep are refused with why; Varsayılana dön and Hepsini
//! varsayılana döndür; the settings file keeps them, entries this version cannot read too; they show where the
//! default keys show; a module's program leaves a key of a command it lacks inert; the user's names run in the command
//! line before the classic CAD names.

use iced::keyboard::key::{Named, NativeCode, Physical};
use iced::keyboard::{Key, Modifiers};
use kentos_commands::{ALIASES_SETTING, SHORTCUTS_SETTING};
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Dialog, Message};
use crate::catalog::catalog;
use crate::files_testing::app_with_drawing;
use crate::keymap::{Keymap, Source, System, system_key_on};
use crate::settings::Settings;
use crate::settings_keys::{KeysEdit, Pending};
use crate::settings_sections::Section;
use crate::settings_view::Edit;

fn keys(app: &mut App, edit: KeysEdit) {
    let _ = app.update(Message::Settings(Edit::Keys(edit)));
}

/// A key pressed as the window reports it (no widget took it).
fn press(app: &mut App, key: Key, modifiers: Modifiers) {
    let text = match &key {
        Key::Character(c) => Some(c.to_string()),
        _ => None,
    };
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key,
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers,
        text,
        repeat: false,
    }));
}

fn ctrl_shift_k(app: &mut App) {
    press(app, Key::Character("K".into()), Modifiers::CTRL | Modifiers::SHIFT);
}

/// Uygulama ayarları on Kısayollar, a command chosen.
fn section_on(app: &mut App, id: &'static str) {
    let _ = app.run("tools.options");
    let _ = app.update(Message::Settings(Edit::Section(Section::Shortcuts)));
    keys(app, KeysEdit::Choose(id));
}

fn note(app: &App) -> String {
    app.settings_draft
        .as_ref()
        .and_then(|d| d.keys.note.clone())
        .map(|(_, text)| text)
        .unwrap_or_default()
}

fn draft_value(app: &App, key: &str) -> Value {
    app.settings_draft.as_ref().and_then(|d| d.values.get(key).cloned()).unwrap_or(Value::Null)
}

/// The user's keys and names as the settings keep them, put in use.
fn with(app: &mut App, shortcuts: Value, aliases: Value) {
    let refused = app.settings.choose(&[(SHORTCUTS_SETTING, shortcuts), (ALIASES_SETTING, aliases)]);
    assert!(refused.is_empty(), "{refused:?}");
    app.apply_settings();
}

#[test]
fn a_recorded_key_goes_to_its_command_and_runs_it_after_kaydet() {
    let mut app = app_with_drawing();
    assert_eq!(app.shortcut("Ctrl+Shift+K"), None, "a key no command has");
    section_on(&mut app, "tool.rectangle");
    keys(&mut app, KeysEdit::Record);
    assert!(app.settings_draft.as_ref().is_some_and(|d| d.keys.recording));
    // A modifier alone waits for the key; the key pressed is given, through the window's router.
    press(&mut app, Key::Named(Named::Shift), Modifiers::SHIFT);
    assert!(app.settings_draft.as_ref().is_some_and(|d| d.keys.recording));
    ctrl_shift_k(&mut app);
    assert!(!app.settings_draft.as_ref().is_some_and(|d| d.keys.recording));
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({ "tool.rectangle": ["R", "Ctrl+Shift+K"] }));
    assert!(note(&app).contains("Dikdörtgen komutuna verildi"), "{}", note(&app));
    // Nothing is used before Kaydet.
    assert_eq!(app.shortcut("Ctrl+Shift+K"), None);
    let _ = app.update(Message::Settings(Edit::Save));
    assert_eq!(app.dialog, None);
    assert_eq!(app.shortcut("Ctrl+Shift+K"), Some("tool.rectangle"), "in use at once, no restart");
    assert_eq!(app.shortcut("R"), Some("tool.rectangle"), "its own key stays");
    ctrl_shift_k(&mut app);
    assert_eq!(app.session.tool_id(), "rectangle");
    // Kept with the other settings, so in the settings file and its export.
    assert!(app.settings.export_text().contains("\"keyboard.shortcuts\""));
}

#[test]
fn a_key_another_command_has_is_said_and_moves_only_on_tasi() {
    let mut app = app_with_drawing();
    section_on(&mut app, "tool.rectangle");
    keys(&mut app, KeysEdit::Recorded("F7".into()));
    let pending = app.settings_draft.as_ref().and_then(|d| d.keys.pending.clone());
    assert_eq!(pending, Some(Pending::Key { chord: "F7".into(), from: "draft.grid" }));
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({}), "nothing moves before Taşı");
    keys(&mut app, KeysEdit::Keep);
    assert!(note(&app).contains("Izgara komutunda kaldı"), "{}", note(&app));
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({}));
    keys(&mut app, KeysEdit::Recorded("F7".into()));
    keys(&mut app, KeysEdit::Move);
    assert!(note(&app).contains("Izgara komutundan alınıp Dikdörtgen komutuna verildi"), "{}", note(&app));
    let _ = app.update(Message::Settings(Edit::Save));
    assert_eq!(app.shortcut("F7"), Some("tool.rectangle"));
    let grid = catalog().get("draft.grid").expect("Izgara");
    assert!(crate::keymap::keys(grid).is_empty(), "Izgara lost F7 while the user's stands");
    // The user's own key moved on: taken from their entry, the command back to its own keys.
    section_on(&mut app, "tool.circle");
    keys(&mut app, KeysEdit::Recorded("F7".into()));
    let pending = app.settings_draft.as_ref().and_then(|d| d.keys.pending.clone());
    assert_eq!(pending, Some(Pending::Key { chord: "F7".into(), from: "tool.rectangle" }));
    keys(&mut app, KeysEdit::Move);
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({ "tool.circle": ["C", "F7"] }));
}

#[test]
fn the_keys_kentos_and_the_system_keep_are_refused_with_why() {
    let mut app = app_with_drawing();
    section_on(&mut app, "tool.rectangle");
    for (chord, why) in [
        ("Esc", "Esc çalışan komutu"),
        ("Enter", "Enter komutu onaylar"),
        ("Tab", "Tab kenet adayları"),
        ("5", "Rakamlar"),
        ("Shift+2", "Rakamlar"),
        ("Down", "Ok tuşu"),
    ] {
        keys(&mut app, KeysEdit::Recorded(chord.into()));
        assert!(note(&app).contains(why), "{chord}: {}", note(&app));
        assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({}), "{chord}");
    }
    // Each system's own keys, on any system.
    assert!(system_key_on(System::MacOs, "Ctrl+Q").is_some_and(|w| w.contains("çıkar")));
    assert!(system_key_on(System::Windows, "Alt+F4").is_some_and(|w| w.contains("kapatır")));
    assert!(system_key_on(System::Linux, "Alt+F2").is_some());
    assert_eq!(system_key_on(System::Windows, "Ctrl+Q"), None, "a key another system keeps is free here");
    let ours = match System::this() {
        System::MacOs => "Ctrl+Q",
        System::Windows => "Alt+F4",
        System::Linux => "Alt+F2",
    };
    keys(&mut app, KeysEdit::Recorded(ours.into()));
    assert!(note(&app).contains("kullanılamaz"), "{}", note(&app));
    // Esc stops waiting for a key without giving it.
    keys(&mut app, KeysEdit::Record);
    press(&mut app, Key::Named(Named::Escape), Modifiers::default());
    assert!(!app.settings_draft.as_ref().is_some_and(|d| d.keys.recording));
    assert_eq!(app.dialog, Some(Dialog::Settings), "Esc while waiting leaves the window open");
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({}));
    // The sheet designer's and Esc and Enter's commands take no key of the user's.
    let sheet = catalog().commands().iter().find(|c| c.id.starts_with("sheet.")).expect("a sheet command");
    assert!(crate::keymap::not_bindable(sheet).is_some());
    assert!(crate::keymap::not_bindable(catalog().get("tool.cancel").expect("İptal")).is_some());
}

#[test]
fn varsayilana_don_and_hepsini_varsayilana_dondur() {
    let mut app = app_with_drawing();
    section_on(&mut app, "tool.rectangle");
    keys(&mut app, KeysEdit::Recorded("Ctrl+Shift+K".into()));
    keys(&mut app, KeysEdit::RemoveKey("R".into()));
    keys(&mut app, KeysEdit::NameTyped("dkg".into()));
    keys(&mut app, KeysEdit::AddName);
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({ "tool.rectangle": ["Ctrl+Shift+K"] }));
    assert_eq!(draft_value(&app, ALIASES_SETTING), json!({ "DKG": "tool.rectangle" }));
    keys(&mut app, KeysEdit::CommandDefaults);
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({}));
    assert_eq!(draft_value(&app, ALIASES_SETTING), json!({}));
    keys(&mut app, KeysEdit::Recorded("Ctrl+Shift+K".into()));
    keys(&mut app, KeysEdit::Choose("tool.circle"));
    keys(&mut app, KeysEdit::NameTyped("dr".into()));
    keys(&mut app, KeysEdit::AddName);
    keys(&mut app, KeysEdit::AllDefaults);
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({}));
    assert_eq!(draft_value(&app, ALIASES_SETTING), json!({}));
    // The window's own Bu bölümü varsayılana döndür does the same for the section.
    keys(&mut app, KeysEdit::Recorded("Ctrl+Shift+K".into()));
    let _ = app.update(Message::Settings(Edit::ResetSection));
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), json!({}));
}

#[test]
fn a_name_kentos_or_a_module_uses_is_refused_and_another_users_moves_on_tasi() {
    let mut app = app_with_drawing();
    section_on(&mut app, "tool.rectangle");
    for (typed, why) in [
        ("L", "KentOS'ta Çizgi komutunun adı"),
        ("3d", "harfle başlar"),
        ("iki ad", "yalnız harf ve rakam"),
        ("DU", "KentOS Mimari'de Duvar komutunun adı"),
    ] {
        keys(&mut app, KeysEdit::NameTyped(typed.into()));
        keys(&mut app, KeysEdit::AddName);
        assert!(note(&app).contains(why), "{typed}: {}", note(&app));
    }
    keys(&mut app, KeysEdit::NameTyped("kutu".into()));
    keys(&mut app, KeysEdit::AddName);
    assert!(note(&app).contains("zaten Dikdörtgen komutunun adı"), "{}", note(&app));
    keys(&mut app, KeysEdit::NameTyped("dkg".into()));
    keys(&mut app, KeysEdit::AddName);
    keys(&mut app, KeysEdit::Choose("tool.circle"));
    keys(&mut app, KeysEdit::NameTyped("DKG".into()));
    keys(&mut app, KeysEdit::AddName);
    let pending = app.settings_draft.as_ref().and_then(|d| d.keys.pending.clone());
    assert_eq!(pending, Some(Pending::Name { name: "DKG".into(), from: "tool.rectangle" }));
    keys(&mut app, KeysEdit::Move);
    assert_eq!(draft_value(&app, ALIASES_SETTING), json!({ "DKG": "tool.circle" }));
}

#[test]
fn the_users_names_run_in_the_command_line_before_the_classic_names() {
    let mut app = app_with_drawing();
    with(&mut app, json!({}), json!({ "DKG": "tool.rectangle", "LA": "tool.circle" }));
    let _ = app.update(Message::CommandRun("dkg".into()));
    assert_eq!(app.session.tool_id(), "rectangle", "folded as KentOS's names are");
    let _ = app.run("tool.cancel");
    // The classic profile's LA is Katman's; the user's own name comes first.
    let _ = app.settings.choose(&[(crate::alias_profile::SETTING, Value::Bool(true))]);
    app.apply_settings();
    let _ = app.update(Message::CommandRun("LA".into()));
    assert_eq!(app.session.tool_id(), "circle");
    let _ = app.run("tool.cancel");
    // The line's list offers the user's names, and Komut ara finds them.
    use kentos_ui::widget::command_line::{Prompt, Suggested, suggested};
    let first = suggested(&app.line_commands(), None::<&Prompt<'_, Message>>, "DKG").into_iter().next();
    assert!(matches!(&first, Some(Suggested::Command(name)) if name == "REC"), "{first:?}");
    app.ribbon_search = "dkg".into();
    assert!(app.hits().iter().any(|h| matches!(h, crate::ribbon_search::Hit::Command(c) if c.id == "tool.rectangle")));
}

#[test]
fn the_users_key_shows_where_the_default_keys_show() {
    let mut app = app_with_drawing();
    with(&mut app, json!({ "tool.rectangle": ["Ctrl+Shift+K"] }), json!({}));
    let rectangle = catalog().get("tool.rectangle").expect("Dikdörtgen");
    // Ribbon tips, menus, the status bar and the start screen read the first key here.
    assert_eq!(crate::keymap::first_key(rectangle).as_deref(), Some("Ctrl+Shift+K"));
    assert_eq!(crate::keymap::active().keys_source("tool.rectangle"), Source::User);
    // Klavye kısayolları finds it by its key, and no longer by R.
    assert_eq!(crate::shortcuts::listed(&app, "ctrl+shift+k"), vec!["Dikdörtgen"]);
    assert!(!crate::shortcuts::listed(&app, "Dikdörtgen").is_empty());
    assert_eq!(app.shortcut("R"), None, "the user's list replaces its keys");
    // Komut ara's row says the key.
    app.ribbon_search = "dikdörtgen".into();
    let rows = app.search_rows();
    let row = rows.iter().find(|r| r.title == "Dikdörtgen").expect("found");
    assert_eq!(row.shortcut.as_deref(), Some("Ctrl+Shift+K"));
    // The key tips still open by Şerit harf ipuçları' key, the user's when moved.
    with(&mut app, json!({ "view.keyTips": ["Ctrl+Shift+K"] }), json!({}));
    ctrl_shift_k(&mut app);
    assert!(app.key_tips.is_some(), "the key tips open by the user's key");
}

#[test]
fn a_key_of_a_command_a_module_lacks_does_nothing_in_its_program() {
    // KentOS Mimari's Duvar, given Ctrl+Shift+K by the user: Duvar there, nothing in KentOS Harita.
    for (program, expected) in [(Some(Module::Architecture), Some("tool.wall")), (Some(Module::Map), None), (None, Some("tool.wall"))] {
        let (mut app, _) = App::boot(None);
        app.program = program;
        with(&mut app, json!({ "tool.wall": ["Ctrl+Shift+K"], "tool.rectangle": ["F7"] }), json!({}));
        assert_eq!(app.shortcut("Ctrl+Shift+K"), expected, "{program:?}");
        // A key the user gave is no other command's anywhere: F7 is not Izgara's.
        assert_eq!(app.shortcut("F7"), app.offers("tool.rectangle").then_some("tool.rectangle"), "{program:?}");
    }
}

#[test]
fn the_keymap_keeps_what_it_cannot_use_and_says_why() {
    let map = Keymap::new(
        &json!({ "tool.yok": ["F9"], "tool.line": ["Esc", "Ctrl+Ş", "Ctrl+Shift+L"], "sheet.zoomIn": ["F9"], "file.save": ["Ctrl+Shift+K"] }),
        &json!({ "L": "tool.rectangle", "3D": "tool.line", "ÇZ2": "tool.line", "BOŞ": "tool.olmayan" }),
    );
    let unused: Vec<(&str, &str)> = map.unused_entries().iter().map(|u| (u.setting, u.entry.as_str())).collect();
    for entry in ["tool.yok", "Çizgi: Esc", "Çizgi: Ctrl+Ş", "L", "3D", "BOŞ"] {
        assert!(unused.iter().any(|(_, e)| *e == entry), "{entry} in {unused:?}");
    }
    assert!(unused.iter().any(|(s, e)| *s == SHORTCUTS_SETTING && e.contains("Yakınlaştır")), "{unused:?}");
    let line = catalog().get("tool.line").expect("Çizgi");
    assert_eq!(map.keys_of(line), vec!["Ctrl+Shift+L"]);
    assert_eq!(map.names_of("tool.line"), vec!["ÇZ2"]);
    // A text field's keys follow: Kaydet's own Ctrl+S is the user's Ctrl+Shift+K now.
    assert_eq!(map.command_in_input("Ctrl+Shift+K"), Some("file.save"));
    assert_eq!(map.command_in_input("Ctrl+S"), None);
    // Unread entries stay in the setting through an edit.
    let edited = map.shortcuts_with("tool.rectangle", "F11");
    assert_eq!(edited["tool.yok"], json!(["F9"]));
    assert_eq!(edited["tool.rectangle"], json!(["R", "F11"]));
}

#[test]
fn the_keys_and_names_are_kept_with_the_settings_and_travel_with_their_file() {
    let dir = crate::files_testing::scratch("keymap-settings");
    let mut settings = Settings::open(&dir, std::time::SystemTime::UNIX_EPOCH);
    let shortcuts = json!({ "tool.rectangle": ["Ctrl+Shift+K"], "tool.gelecek": ["Ctrl+F30"] });
    let aliases = json!({ "DKG": "tool.rectangle" });
    let refused = settings.choose(&[(SHORTCUTS_SETTING, shortcuts.clone()), (ALIASES_SETTING, aliases.clone())]);
    assert!(refused.is_empty(), "{refused:?}");
    let reopened = Settings::open(&dir, std::time::SystemTime::UNIX_EPOCH);
    assert_eq!(reopened.effective(SHORTCUTS_SETTING), shortcuts, "an entry this version cannot use is kept too");
    assert_eq!(reopened.effective(ALIASES_SETTING), aliases);
    assert!(reopened.report.recovered.is_none(), "nothing in it was refused");
    // Export and import carry them; the window's İçe aktar takes them into its draft.
    let text = reopened.export_text();
    let mut other = Settings::memory();
    assert_eq!(other.import_text(&text), Ok(Vec::new()));
    assert_eq!(other.effective(ALIASES_SETTING), aliases);
    let mut app = app_with_drawing();
    let _ = app.run("tools.options");
    let _ = app.update(Message::Settings(Edit::Imported(Some(Ok(("kentos-ayarlar.json".into(), text))))));
    assert_eq!(draft_value(&app, SHORTCUTS_SETTING), shortcuts);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_section_draws_and_klavye_kisayollari_leads_to_it() {
    let mut app = app_with_drawing();
    let _ = app.run("help.shortcuts");
    keys(&mut app, KeysEdit::Open);
    assert_eq!(app.dialog, Some(Dialog::Settings));
    assert_eq!(app.settings_draft.as_ref().map(|d| d.section), Some(Section::Shortcuts));
    keys(&mut app, KeysEdit::Search("dikdört".into()));
    let (listed, _) = app.keys_listed(&crate::settings_keys::draft_keymap(app.settings_draft.as_ref().expect("open")), "dikdört");
    assert!(listed.iter().any(|c| c.id == "tool.rectangle"));
    keys(&mut app, KeysEdit::Choose("tool.rectangle"));
    keys(&mut app, KeysEdit::Recorded("F7".into()));
    let _ = app.view();
}

/// The section for the owner, a key of the user's and a key taken from another command, dark and light, at both
/// sizes; Klavye kısayolları with the user's key. `.run/gui-evidence/kisayollar/`.
/// `KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-desktop --lib keymap_tests::screens -- --ignored --nocapture`
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kisayollar");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["kullanici", "cakisma", "klavye-penceresi"] {
                let mut app = app_with_drawing();
                let _ = app.settings.choose(&[("appearance.theme", Value::from(mode))]);
                with(&mut app, json!({ "tool.rectangle": ["R", "Ctrl+Shift+K"] }), json!({ "DKG": "tool.rectangle" }));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "kullanici" => section_on(&mut app, "tool.rectangle"),
                    "cakisma" => {
                        section_on(&mut app, "tool.circle");
                        keys(&mut app, KeysEdit::Recorded("F7".into()));
                    }
                    _ => {
                        let _ = app.run("help.shortcuts");
                        let _ = app.update(Message::ShortcutsSearch("dikdörtgen".into()));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
