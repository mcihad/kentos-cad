//! The ribbon's key tips and its folded peek on the desktop (docs/specs/
//! ribbon.md §1 and §6, docs/adr/0118).
//!
//! - **Key tips:** F6 (`view.keyTips`) or Alt tapped alone shows a digit on
//!   the quick access bar's usable buttons and a letter or two on each tab;
//!   typing a tab's letters opens it and gives its usable controls theirs,
//!   in reading order (panels left to right; in each, its buttons, then its
//!   ▾ and its launcher). A tip typed in full runs its control: a button
//!   runs, a menu opens from the keyboard. The rules (letters, tips, keys)
//!   are keytips.rs's; the tips go to what the ribbon shows at the window's
//!   width, fitted by the ribbon's own rule. A click anywhere, the window
//!   losing the focus or changing size sends them away.
//! - **Peek:** on a folded ribbon a tab opens over the drawing without
//!   moving it; the same tab again, a command, Esc or a click outside
//!   closes it.

use iced::Task;
use iced::advanced::widget;
use iced::keyboard::key::Named;
use iced::{Event, event, keyboard, mouse, window};
use kentos_ui::widget::KeyTip;
use kentos_ui::widget::ribbon::{Group, fit};

use crate::app::{App, Message};
use crate::catalog::{Item, Panel as RibbonPanel, Standing, Tab, catalog};
use crate::keys::KeyPress;
use crate::keytips::{Level, Step, assign_key_tips, first_level_tips, key_tip_step};
use crate::ribbon_plan::texts;

/// What a key tip is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TipKey {
    Tab(&'static str),
    Quick(&'static str),
    Command(&'static str),
    /// A split button's top (by the button's key).
    SplitTop(&'static str),
    SplitArrow(&'static str),
    /// A drop-down button (by its label).
    Menu(&'static str),
    /// A panel's ▾, launcher and folded button (by the panel's label).
    More(&'static str),
    Launcher(&'static str),
    Folded(&'static str),
    /// One of the user's models in the Modeller panel (by its id).
    Model(String),
}

/// What typing a key tip in full does.
#[derive(Clone, Debug)]
pub(crate) enum TipAction {
    Run(Message),
    /// A tab opens and its controls get tips.
    Tab(&'static str),
    /// A menu opens from the keyboard.
    Menu(widget::Id),
}

/// The key tips while they show.
pub(crate) struct KeyTips {
    pub level: Level,
    /// The letters typed so far.
    pub typed: String,
    /// The tips shown: what each is on, its letters and what typing it does.
    targets: Vec<(TipKey, String, TipAction)>,
}

/// The menus' ids, so the key tips can open them.
pub(crate) fn split_menu_id(key: &str) -> widget::Id {
    widget::Id::from(format!("serit:bolunmus:{key}"))
}

pub(crate) fn menu_id(label: &str) -> widget::Id {
    widget::Id::from(format!("serit:menu:{label}"))
}

pub(crate) fn more_id(panel: &str) -> widget::Id {
    widget::Id::from(format!("serit:diger:{panel}"))
}

pub(crate) fn folded_id(panel: &str) -> widget::Id {
    widget::Id::from(format!("serit:katli:{panel}"))
}

/// A command the desktop runs and that can be used now.
fn usable(app: &App, id: &'static str) -> bool {
    catalog()
        .get(id)
        .is_some_and(|c| c.standing == Standing::Ported)
        && app.available(id)
}

/// A key as the web names it (`KeyboardEvent.key`): the character it
/// types, else its name (`Escape`, `Backspace`, `Alt` …).
fn web_key(press: &KeyPress) -> String {
    match &press.key {
        keyboard::Key::Character(c) => c.to_string(),
        keyboard::Key::Named(Named::Space) => " ".to_owned(),
        keyboard::Key::Named(named) => format!("{named:?}"),
        keyboard::Key::Unidentified => String::new(),
    }
}

/// While the tips show (or Alt is down alone): a press of any mouse button
/// or the window losing the focus sends them away.
pub(crate) fn away_events(
    event: Event,
    _status: event::Status,
    _window: window::Id,
) -> Option<Message> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) | Event::Window(window::Event::Unfocused) => {
            Some(Message::KeyTipsAway)
        }
        _ => None,
    }
}

impl App {
    /// The open tab's panels as the ribbon shows them, each with its
    /// panel (none for Görünüm's own look, appearance.rs).
    pub(crate) fn tab_groups(
        &self,
        tab: &'static Tab,
    ) -> Vec<(Option<&'static RibbonPanel>, Group<'static, Message>)> {
        let mut groups: Vec<(Option<&'static RibbonPanel>, Group<'static, Message>)> = tab
            .panels
            .iter()
            .filter_map(|panel| self.ribbon_group(tab.id, panel).map(|g| (Some(panel), g)))
            .collect();
        if tab.id == "view" {
            groups.extend(self.appearance_groups().into_iter().map(|g| (None, g)));
        }
        groups
    }

    /// The tip on something the ribbon draws, dimmed when it does not
    /// start with what is typed.
    pub(crate) fn key_tip(&self, key: &TipKey) -> Option<KeyTip> {
        let tips = self.key_tips.as_ref()?;
        tips.targets
            .iter()
            .find(|(k, _, _)| k == key)
            .map(|(_, tip, _)| KeyTip::new(tip.clone(), !tip.starts_with(&tips.typed)))
    }

    /// The tips of a level: the bar's digits and the tabs' letters, or the
    /// open tab's controls' letters.
    fn tip_targets(&self, level: Level) -> Vec<(TipKey, String, TipAction)> {
        match level {
            Level::Tabs => {
                let bar: Vec<&'static str> = self
                    .quick_bar()
                    .iter()
                    .filter_map(|id| catalog().get(id).map(|c| c.id))
                    .filter(|id| usable(self, id))
                    .collect();
                let mut tabs: Vec<&'static Tab> = self.ribbon_tabs().collect();
                if !self.selection.is_empty() {
                    tabs.extend(self.contextual_tabs());
                }
                let labels: Vec<&str> = tabs.iter().map(|t| t.label).collect();
                let (digits, letters) = first_level_tips(bar.len(), &labels);
                let mut out: Vec<(TipKey, String, TipAction)> = bar
                    .iter()
                    .zip(digits)
                    .map(|(id, d)| (TipKey::Quick(id), d, TipAction::Run(Message::Run(id))))
                    .collect();
                out.extend(
                    tabs.iter()
                        .zip(letters)
                        .filter(|(_, l)| !l.is_empty())
                        .map(|(t, l)| (TipKey::Tab(t.id), l, TipAction::Tab(t.id))),
                );
                out
            }
            Level::Controls => {
                let controls = self.tip_controls();
                let labels: Vec<&str> = controls.iter().map(|(_, l, _)| l.as_str()).collect();
                let tips = assign_key_tips(&labels, &[]);
                controls
                    .into_iter()
                    .zip(tips)
                    .filter(|(_, tip)| !tip.is_empty())
                    .map(|((key, _, action), tip)| (key, tip, action))
                    .collect()
            }
        }
    }

    /// The open tab's usable controls in reading order with their names, as
    /// the ribbon shows them at the window's width.
    fn tip_controls(&self) -> Vec<(TipKey, String, TipAction)> {
        let shown = self.shown_tab();
        let Some(tab) = self
            .ribbon_tabs()
            .chain(self.contextual_tabs())
            .find(|t| t.id == shown)
        else {
            return Vec::new();
        };
        let groups = self.tab_groups(tab);
        let widths: Vec<[f32; 4]> = groups.iter().map(|(_, g)| g.widths()).collect();
        let keep: Vec<bool> = groups.iter().map(|(_, g)| g.keeps()).collect();
        let (levels, _) = fit(&widths, &keep, self.window_size.width);
        let mut out = Vec::new();
        for ((panel, _), level) in groups.iter().zip(levels) {
            // Görünüm's own look has its own controls, without tips.
            let Some(panel) = panel else { continue };
            if level >= 3 {
                out.push((
                    TipKey::Folded(panel.label),
                    panel.label.to_owned(),
                    TipAction::Menu(folded_id(panel.label)),
                ));
                continue;
            }
            for item in &panel.items {
                self.item_controls(tab.id, item, &mut out);
            }
            // The user's models in the model library's panel (view.rs `user_model_buttons`).
            if panel.items.iter().any(|item| {
                matches!(
                    item,
                    Item::Command {
                        id: "processing.newModel",
                        ..
                    }
                )
            }) {
                for m in self.user_models() {
                    out.push((
                        TipKey::Model(m.id.clone()),
                        format!("{}…", m.label),
                        TipAction::Run(crate::processing::panel_message(
                            crate::processing::panel::Event::RunModel(m.id.clone()),
                        )),
                    ));
                }
            }
            if !panel.overflow.is_empty() {
                out.push((
                    TipKey::More(panel.label),
                    texts::panel_more(panel.label),
                    TipAction::Menu(more_id(panel.label)),
                ));
            }
            if let Some(launcher) = &panel.launcher
                && let Some(message) = crate::view::launch(launcher)
            {
                out.push((
                    TipKey::Launcher(panel.label),
                    launcher.title.to_owned(),
                    TipAction::Run(message),
                ));
            }
        }
        out
    }

    /// A panel item's controls: a button, a split button's top and arrow, a drop-down.
    fn item_controls(&self, tab: &str, item: &Item, out: &mut Vec<(TipKey, String, TipAction)>) {
        let catalog = catalog();
        match item {
            Item::Command { id, .. } => {
                if let Some(command) = catalog.get(id)
                    && usable(self, command.id)
                {
                    out.push((
                        TipKey::Command(command.id),
                        command.title.to_owned(),
                        TipAction::Run(Message::Run(command.id)),
                    ));
                }
            }
            Item::Split { key, entries, .. } => {
                let Some(top) = self.split_on_top(key, entries) else {
                    return;
                };
                if usable(self, top.id) {
                    let (_, aria, _) =
                        crate::ribbon_plan::split_face(&crate::ribbon_bar::split_entry(top));
                    let run = match top.option {
                        Some(option) => Message::RunMethod {
                            id: top.id,
                            option,
                            label: top.label,
                        },
                        None => Message::Run(top.id),
                    };
                    out.push((TipKey::SplitTop(key), aria, TipAction::Run(run)));
                }
                if entries.iter().any(|e| {
                    catalog
                        .get(e.id)
                        .is_some_and(|c| c.standing == Standing::Ported)
                }) {
                    out.push((
                        TipKey::SplitArrow(key),
                        texts::others(top.title.trim_end_matches('…')),
                        TipAction::Menu(split_menu_id(key)),
                    ));
                }
            }
            Item::Menu { label, ids, .. } => {
                // Görünüm's Tema and Çizim motoru are the desktop's own groups there.
                if tab == "view" && matches!(*label, "Tema" | "Çizim motoru") {
                    return;
                }
                if ids.iter().any(|id| {
                    catalog
                        .get(id)
                        .is_some_and(|c| c.standing == Standing::Ported)
                }) {
                    out.push((
                        TipKey::Menu(label),
                        (*label).to_owned(),
                        TipAction::Menu(menu_id(label)),
                    ));
                }
            }
            Item::Builtin(_) => {}
        }
    }

    /// F6, `view.keyTips` or Alt tapped: the tips show, or go if they do.
    /// The keyboard leaves any text box, so the letters come here.
    pub(crate) fn toggle_key_tips(&mut self) -> Task<Message> {
        if self.key_tips.is_some() {
            self.key_tips = None;
            return Task::none();
        }
        if self.dialog.is_some() || self.app_menu.is_some() {
            return Task::none();
        }
        self.key_tips = Some(KeyTips {
            level: Level::Tabs,
            typed: String::new(),
            targets: Vec::new(),
        });
        self.refresh_key_tips();
        crate::input::release_keyboard()
    }

    /// After every message: the tips follow what the ribbon shows now.
    pub(crate) fn refresh_key_tips(&mut self) {
        let Some(level) = self.key_tips.as_ref().map(|t| t.level) else {
            return;
        };
        let targets = self.tip_targets(level);
        if let Some(tips) = self.key_tips.as_mut() {
            tips.targets = targets;
        }
    }

    /// A key while the tips show; none when they do not (the key goes on).
    pub(crate) fn key_tips_key(&mut self, press: &KeyPress) -> Option<Task<Message>> {
        let tips = self.key_tips.as_ref()?;
        let letters: Vec<String> = tips.targets.iter().map(|(_, t, _)| t.clone()).collect();
        let ctrl = press.modifiers.control() || press.modifiers.logo();
        let step = key_tip_step(tips.level, &tips.typed, &letters, &web_key(press), ctrl);
        match step {
            Step::Ignore => {}
            Step::Typed(typed) => {
                if let Some(tips) = self.key_tips.as_mut() {
                    tips.typed = typed;
                }
            }
            Step::Back => {
                if let Some(tips) = self.key_tips.as_mut() {
                    tips.level = Level::Tabs;
                    tips.typed.clear();
                }
                self.ribbon_peek = false;
                self.refresh_key_tips();
            }
            Step::Hide => self.key_tips = None,
            Step::Run(tip) => {
                let action = self
                    .key_tips
                    .as_ref()
                    .and_then(|t| t.targets.iter().find(|(_, x, _)| *x == tip))
                    .map(|(_, _, action)| action.clone());
                match action {
                    Some(TipAction::Tab(id)) => {
                        self.open_tab(id);
                        if let Some(tips) = self.key_tips.as_mut() {
                            tips.level = Level::Controls;
                            tips.typed.clear();
                        }
                        self.refresh_key_tips();
                    }
                    Some(TipAction::Run(message)) => {
                        self.key_tips = None;
                        return Some(Task::done(message));
                    }
                    Some(TipAction::Menu(id)) => {
                        self.key_tips = None;
                        return Some(kentos_ui::widget::context_menu::open_menu(id));
                    }
                    None => self.key_tips = None,
                }
            }
        }
        Some(Task::none())
    }

    /// Alt pressed alone arms the tap; any other key disarms it.
    pub(crate) fn alt_tap(&mut self, press: &KeyPress) -> bool {
        let alone = press.named() == Some(Named::Alt)
            && !press.repeat
            && !press.modifiers.control()
            && !press.modifiers.shift()
            && !press.modifiers.logo();
        self.alt_armed = alone;
        alone
    }

    /// The modifiers changed: Alt let go after being tapped alone shows the
    /// tips (no window or menu open).
    pub(crate) fn alt_released(&mut self, modifiers: keyboard::Modifiers) -> Task<Message> {
        if self.alt_armed && !modifiers.alt() {
            self.alt_armed = false;
            if self.dialog.is_none() && self.app_menu.is_none() {
                return self.toggle_key_tips();
            }
        }
        Task::none()
    }

    /// A tab clicked (or opened by its key tip). On a folded ribbon it opens
    /// over the drawing; the same tab again closes it.
    pub(crate) fn tab_clicked(&mut self, id: &'static str) {
        if self.ribbon_collapsed {
            if self.ribbon_peek && self.shown_tab() == id {
                self.ribbon_peek = false;
                return;
            }
            self.choose_tab(id);
            self.ribbon_peek = true;
        } else {
            self.choose_tab(id);
        }
    }

    /// A tab opened by its key tip: over the drawing when the ribbon is folded.
    fn open_tab(&mut self, id: &'static str) {
        self.choose_tab(id);
        if self.ribbon_collapsed {
            self.ribbon_peek = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Key, Modifiers, key::Named};
    use kentos_contracts::Workspace;
    use serde_json::Value;

    use super::TipKey;
    use crate::app::{App, Message};
    use crate::catalog::catalog;
    use crate::files_testing::app_with_drawing;
    use crate::keytips::Level;

    const FIXTURE: &str = include_str!("../../../fixtures/shell/v1/ribbon.json");

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

    fn letter(app: &mut App, c: &str) {
        press(app, Key::Character(c.into()), Modifiers::default());
    }

    fn named(app: &mut App, key: Named) {
        press(app, Key::Named(key), Modifiers::default());
    }

    fn tip(app: &App, key: TipKey) -> Option<String> {
        app.key_tip(&key).map(|t| t.tip)
    }

    /// The tabs each work mode shows are the web's, in its order (ribbon.json `tabs`).
    #[test]
    fn the_tabs_are_the_web_s_in_every_mode() {
        let fixture: Value = serde_json::from_str(FIXTURE).expect("ribbon.json reads");
        for (mode, workspace) in [
            ("hybrid", Workspace::Hybrid),
            ("cad", Workspace::Cad),
            ("gis", Workspace::Gis),
        ] {
            let ours: Vec<(String, String, bool)> = catalog()
                .tabs_in(workspace)
                .map(|t| (t.id.to_owned(), t.label.to_owned(), false))
                .chain(
                    catalog()
                        .contextual_in(workspace)
                        .map(|t| (t.id.to_owned(), t.label.to_owned(), true)),
                )
                .collect();
            let theirs: Vec<(String, String, bool)> = fixture["tabs"][mode]
                .as_array()
                .into_iter()
                .flatten()
                .map(|t| {
                    (
                        t["id"].as_str().unwrap_or_default().to_owned(),
                        t["label"].as_str().unwrap_or_default().to_owned(),
                        t["contextual"].as_bool().unwrap_or_default(),
                    )
                })
                .collect();
            assert_eq!(ours, theirs, "{mode}");
        }
    }

    #[test]
    fn f6_shows_digits_on_the_bar_and_letters_on_the_tabs() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("view.keyTips"));
        assert_eq!(app.key_tips.as_ref().map(|t| t.level), Some(Level::Tabs));
        // Kaydet is the only usable one of the fixed three (nothing to undo yet).
        assert_eq!(tip(&app, TipKey::Quick("file.save")), Some("1".into()));
        assert_eq!(tip(&app, TipKey::Quick("edit.undo")), None);
        assert_eq!(tip(&app, TipKey::Tab("home")), Some("GI".into()));
        assert_eq!(tip(&app, TipKey::Tab("draw")), Some("C".into()));
        assert_eq!(tip(&app, TipKey::Tab("view")), Some("GO".into()));
        // F6 again: away.
        let _ = app.update(Message::Run("view.keyTips"));
        assert!(app.key_tips.is_none());
    }

    #[test]
    fn a_tab_s_letters_open_it_and_its_controls_get_letters_that_run_them() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("view.keyTips"));
        letter(&mut app, "ç");
        assert_eq!(app.shown_tab(), "draw", "Çizim opens (ç folds to C)");
        assert_eq!(
            app.key_tips.as_ref().map(|t| t.level),
            Some(Level::Controls)
        );
        let line = tip(&app, TipKey::Command("tool.line")).expect("Çizgi has a tip");
        for c in line.chars() {
            letter(&mut app, &c.to_string());
        }
        assert!(app.key_tips.is_none(), "the tips go when a control runs");
        // The command runs as the next message (a task).
        let _ = app.update(Message::Run("tool.line"));
        assert_eq!(app.session.tool_id(), "line");
    }

    #[test]
    fn esc_backspace_and_a_click_go_back_or_away() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("view.keyTips"));
        letter(&mut app, "g");
        assert_eq!(app.key_tips.as_ref().map(|t| t.typed.as_str()), Some("G"));
        let dim = app.key_tip(&TipKey::Tab("draw")).expect("Çizim's tip");
        assert!(dim.dim, "what does not start with G dims");
        named(&mut app, Named::Backspace);
        assert_eq!(app.key_tips.as_ref().map(|t| t.typed.as_str()), Some(""));
        letter(&mut app, "c");
        assert_eq!(
            app.key_tips.as_ref().map(|t| t.level),
            Some(Level::Controls)
        );
        named(&mut app, Named::Escape);
        assert_eq!(
            app.key_tips.as_ref().map(|t| t.level),
            Some(Level::Tabs),
            "back a level"
        );
        named(&mut app, Named::Escape);
        assert!(app.key_tips.is_none(), "away");
        let _ = app.update(Message::Run("view.keyTips"));
        let _ = app.update(Message::KeyTipsAway);
        assert!(app.key_tips.is_none(), "a click sends them away");
    }

    #[test]
    fn alt_tapped_alone_shows_them_and_with_another_key_does_not() {
        let mut app = app_with_drawing();
        press(&mut app, Key::Named(Named::Alt), Modifiers::ALT);
        let _ = app.update(Message::Modifiers(Modifiers::default()));
        assert!(app.key_tips.is_some(), "Alt tapped alone");
        named(&mut app, Named::Escape);
        assert!(app.key_tips.is_none());
        press(&mut app, Key::Named(Named::Alt), Modifiers::ALT);
        press(&mut app, Key::Character("q".into()), Modifiers::ALT);
        let _ = app.update(Message::Modifiers(Modifiers::default()));
        assert!(app.key_tips.is_none(), "Alt with Q is a chord, not a tap");
    }

    #[test]
    fn a_folded_ribbon_s_tab_opens_over_the_drawing_and_closes() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("view.ribbonCollapse"));
        assert!(app.ribbon_collapsed);
        let _ = app.update(Message::RibbonTab("draw"));
        assert!(app.ribbon_peek, "the tab opens over the drawing");
        assert_eq!(app.shown_tab(), "draw");
        let _ = app.update(Message::RibbonTab("draw"));
        assert!(!app.ribbon_peek, "the same tab closes it");
        let _ = app.update(Message::RibbonTab("modify"));
        let _ = app.update(Message::Run("tool.line"));
        assert!(!app.ribbon_peek, "a command closes it");
        let _ = app.update(Message::RibbonTab("modify"));
        named(&mut app, Named::Escape);
        assert!(!app.ribbon_peek, "Esc closes it");
        let _ = app.update(Message::RibbonTab("modify"));
        let _ = app.update(Message::RibbonPeekAway);
        assert!(!app.ribbon_peek, "a press outside closes it");
        // The key tips open a folded ribbon's tab over the drawing too.
        let _ = app.update(Message::Run("view.keyTips"));
        letter(&mut app, "c");
        assert!(app.ribbon_peek);
        assert!(app.ribbon_collapsed, "the fold is kept");
    }
}

/// The key tips' and the folded ribbon's pictures, the web's scenes
/// (apps/web/scripts/e2e/shots.mjs `ribbon`: keytips-tabs, keytips-controls,
/// keytips-typed, folded-open), in `.run/shots/serit-*`:
///
/// ```text
/// cargo test -p kentos-desktop ribbon_keys::screens -- --ignored --nocapture
/// ```
#[cfg(test)]
mod screens {
    use iced::Size;
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Key, Modifiers};
    use kentos_ui::snapshot::Snapshot;

    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;

    fn letter(app: &mut App, c: &str) {
        let _ = app.update(Message::Key(crate::keys::KeyPress {
            key: Key::Character(c.into()),
            physical: Physical::Unidentified(NativeCode::Unidentified),
            modifiers: Modifiers::default(),
            text: Some(c.to_owned()),
            repeat: false,
        }));
    }

    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let scenes = [
            "ipucu-sekmeler",
            "ipucu-denetimler",
            "ipucu-yazilan",
            "katli-acik",
            "katli-ipucu",
        ];
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                for scene in scenes {
                    if only
                        .as_ref()
                        .is_some_and(|o| !o.split(',').any(|s| s == scene))
                    {
                        continue;
                    }
                    let mut app = app_with_drawing();
                    let _ = app
                        .settings
                        .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                    app.apply_settings();
                    let _ = app.update(Message::WindowResized(Size::new(width, height)));
                    let _ = app.update(Message::QuickAccess("view.zoomExtents".into(), true));
                    let _ = app.update(Message::QuickAccess("tool.line".into(), true));
                    match scene {
                        "ipucu-sekmeler" => {
                            let _ = app.update(Message::Run("view.keyTips"));
                        }
                        "ipucu-denetimler" => {
                            let _ = app.update(Message::Run("view.keyTips"));
                            letter(&mut app, "g");
                            letter(&mut app, "i");
                        }
                        "ipucu-yazilan" => {
                            let _ = app.update(Message::Run("view.keyTips"));
                            letter(&mut app, "g");
                        }
                        "katli-acik" => {
                            let _ = app.update(Message::Run("view.ribbonCollapse"));
                            let _ = app.update(Message::RibbonTab("draw"));
                        }
                        _ => {
                            let _ = app.update(Message::Run("view.ribbonCollapse"));
                            let _ = app.update(Message::Run("view.keyTips"));
                            letter(&mut app, "c");
                        }
                    }
                    let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.settle(&mut app, App::view, &mut update);
                    let file = out.join(format!("serit-{scene}-{width}x{height}{suffix}.png"));
                    snapshot
                        .render(app.view(), &app.theme())
                        .save(&file)
                        .expect("writes the picture");
                    println!("{}", file.display());
                }
            }
        }
    }
}
