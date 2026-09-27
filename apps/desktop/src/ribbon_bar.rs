//! The ribbon's quick access bar, right-click menus and split buttons'
//! choices on the desktop (docs/adr/0117, docs/specs/ribbon.md §2–§4): the
//! bar is the fixed three and what the user added, kept in the layout
//! (`ribbonQuickAccess`); a split button shows the entry last chosen from
//! its list (`ribbonSplits`). The rules are ribbon_plan.rs's and
//! layout_plan.rs's; here is what they do in the app and how their rows
//! become menus.

use std::time::Instant;

use iced::Task;
use kentos_ui::widget::Menu;

use crate::app::{App, Message};
use crate::catalog::{Entry, catalog};
use crate::layout_plan::{quick_access_of, split_choice_key, split_current};
use crate::ribbon_plan::{self as plan, Row, SplitEntry};

impl App {
    /// The quick access bar: the fixed three, then what the user added
    /// that this app has, in the order added.
    pub(crate) fn quick_bar(&self) -> Vec<String> {
        quick_access_of(&self.layout.quick_access(), |id| {
            catalog().get(id).is_some()
        })
    }

    /// A command put on the bar (at its end) or taken off; the fixed three
    /// (the web's, as the inventory writes them) stay.
    pub(crate) fn quick_access_changed(&mut self, id: &str, on: bool) {
        if catalog().quick().contains(&id) {
            return;
        }
        let kept = plan::with_quick_access(&self.layout.quick_access(), id, on);
        self.layout.keep_quick_access(&kept, Instant::now());
    }

    /// A split button's entry on top: the one last chosen from its list.
    pub(crate) fn split_on_top<'e>(&self, key: &str, entries: &'e [Entry]) -> Option<&'e Entry> {
        let pairs: Vec<(&str, Option<&str>)> = entries.iter().map(|e| (e.id, e.option)).collect();
        entries.get(split_current(&pairs, self.layout.split_choice(key)))
    }

    /// An entry chosen from a split button's list: it goes on top (kept by
    /// the button's key) and runs, its method's option given as if typed.
    pub(crate) fn split_chosen(
        &mut self,
        key: &'static str,
        id: &'static str,
        option: Option<&'static str>,
        label: &'static str,
    ) -> Task<Message> {
        self.layout
            .keep_split_choice(key, split_choice_key(id, option), Instant::now());
        match option {
            Some(option) => self.run_method(id, option, label),
            None => self.run(id),
        }
    }
}

/// The plan's rows as a menu: a command's own row, or a quick access row
/// that puts its command on the bar or takes it off. `folded`: whether the
/// ribbon is folded (the fold's check).
pub(crate) fn rows_menu(rows: &[Row], folded: bool) -> Menu<Message> {
    rows.iter().fold(Menu::new(), |menu, row| match row {
        Row::Header(label) => menu.header(*label),
        Row::Separator => menu.separator(),
        Row::Command(id) => match catalog().get(id) {
            Some(command) => {
                let menu = menu.check(command.title, folded, Message::Run(command.id));
                match command.shortcuts.first() {
                    Some(keys) => menu.shortcut(*keys),
                    None => menu,
                }
            }
            None => menu,
        },
        Row::Quick(q) => {
            let title = catalog()
                .get(&q.command)
                .map_or(q.command.as_str(), |c| c.title);
            let label = q.label.unwrap_or(title);
            let run = (!q.disabled)
                .then_some(q.set)
                .flatten()
                .map(|on| Message::QuickAccess(q.command.clone(), on));
            let menu = match q.checked {
                Some(checked) => menu.check(label, checked, run),
                None => menu.item(label, run),
            };
            let menu = match q.icon {
                Some(icon) if q.checked.is_none() => menu.icon(crate::icons::from_web(Some(icon))),
                _ => menu,
            };
            match q.hint {
                Some(hint) => menu.hint(hint),
                None => menu,
            }
        }
    })
}

/// A catalog entry as the plan reads it.
pub(crate) fn split_entry(e: &Entry) -> SplitEntry<'static> {
    SplitEntry {
        command: e.id,
        option: e.option,
        title: e.title,
        label: e.label,
        description: e.description,
    }
}

/// A split button's list (the web's `splitMenu`): a tool's methods under
/// its name, a family without a title; each row the entry's icon, words,
/// what it does and the command's shortcut. Choosing one keeps it on top
/// and runs it; a command not on the desktop is off.
pub(crate) fn split_list(key: &'static str, entries: &[Entry]) -> Menu<Message> {
    let plan_entries: Vec<SplitEntry> = entries.iter().map(split_entry).collect();
    let (header, rows) = plan::split_menu(&plan_entries);
    let menu = match header {
        Some(title) => Menu::new().header(title),
        None => Menu::new(),
    };
    entries.iter().zip(rows).fold(menu, |menu, (entry, row)| {
        let Some(command) = catalog().get(entry.id) else {
            return menu;
        };
        let run = (command.standing == crate::catalog::Standing::Ported).then_some(
            Message::SplitChosen {
                key,
                id: entry.id,
                option: entry.option,
                label: entry.label,
            },
        );
        let menu = menu.item(row.label, run).icon(command.icon);
        let menu = match row.hint {
            Some(hint) => menu.hint(hint),
            None => menu,
        };
        match command.shortcuts.first() {
            Some(keys) => menu.shortcut(*keys),
            None => menu,
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::app::Message;
    use crate::catalog::{Entry, Item, catalog};
    use crate::files_testing::{app_with_drawing, last_said};

    /// Daire's split button: its key and its methods, as the ribbon lists them.
    fn circle() -> (&'static str, Vec<Entry>) {
        catalog()
            .tabs()
            .flat_map(|t| t.panels.iter())
            .flat_map(|p| p.items.iter())
            .find_map(|item| match item {
                Item::Split { key, entries, .. }
                    if entries.first().is_some_and(|e| e.id == "tool.circle") =>
                {
                    Some((*key, entries.clone()))
                }
                _ => None,
            })
            .expect("Daire's split button")
    }

    #[test]
    fn a_command_is_put_on_the_bar_and_taken_off_and_the_fixed_ones_stay() {
        // The fixed three the rules name are the web's, as the inventory writes them.
        assert_eq!(catalog().quick(), crate::layout_plan::QUICK_ACCESS);
        let mut app = app_with_drawing();
        assert_eq!(app.quick_bar(), ["file.save", "edit.undo", "edit.redo"]);
        let _ = app.update(Message::QuickAccess("view.zoomExtents".into(), true));
        let _ = app.update(Message::QuickAccess("tool.line".into(), true));
        assert_eq!(
            app.quick_bar(),
            [
                "file.save",
                "edit.undo",
                "edit.redo",
                "view.zoomExtents",
                "tool.line"
            ]
        );
        assert_eq!(
            app.layout.quick_access(),
            ["view.zoomExtents", "tool.line"],
            "kept in the layout"
        );
        assert!(app.layout.due().is_some(), "written a moment later");
        let _ = app.update(Message::QuickAccess("view.zoomExtents".into(), false));
        assert_eq!(app.layout.quick_access(), ["tool.line"]);
        // The fixed three are neither taken off nor added again.
        let _ = app.update(Message::QuickAccess("file.save".into(), false));
        let _ = app.update(Message::QuickAccess("edit.undo".into(), true));
        assert_eq!(
            app.quick_bar(),
            ["file.save", "edit.undo", "edit.redo", "tool.line"]
        );
    }

    #[test]
    fn a_split_choice_goes_on_top_is_kept_and_runs_with_its_option() {
        let mut app = app_with_drawing();
        let (key, entries) = circle();
        assert_eq!(key, "circle", "the web's key: the tool with methods");
        assert_eq!(
            app.split_on_top(key, &entries).map(|e| e.label),
            Some("Merkez, yarıçap")
        );
        let _ = app.update(Message::SplitChosen {
            key,
            id: "tool.circle",
            option: Some("3N"),
            label: "3 nokta",
        });
        assert_eq!(app.layout.split_choice(key), Some("tool.circle|3N"));
        assert_eq!(
            app.split_on_top(key, &entries).map(|e| e.label),
            Some("3 nokta")
        );
        assert!(app.session.is_running(), "the tool runs");
        assert_eq!(app.session.tool_id(), "circle");
        let prompt = app.session.prompt().text();
        assert!(
            prompt.starts_with("Daire: ilk noktayı belirtin"),
            "{prompt}"
        );
    }

    #[test]
    fn a_method_the_tool_does_not_take_is_said_in_the_log() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::SplitChosen {
            key: "circle",
            id: "tool.circle",
            option: Some("KTT"),
            label: "Kayıp teğet",
        });
        assert_eq!(last_said(&app), "“Daire: Kayıp teğet” şu an başlatılamadı.");
        assert_eq!(app.layout.split_choice("circle"), Some("tool.circle|KTT"));
    }
}

/// The ribbon's bar, menus and split lists, the web's scenes
/// (apps/web/scripts/e2e/shots.mjs `ribbon`), in `.run/shots/serit-*`:
///
/// ```text
/// cargo test -p kentos-desktop ribbon_bar::screens -- --ignored --nocapture
/// ```
#[cfg(test)]
mod screens {
    use iced::{Point, Size, mouse};
    use kentos_ui::snapshot::Snapshot;

    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, find_text};

    fn press(snapshot: &mut Snapshot, app: &mut App, at: Point, button: mouse::Button) {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.step(
            app,
            App::view,
            &mut update,
            &[
                iced::Event::Mouse(mouse::Event::CursorMoved { position: at }),
                iced::Event::Mouse(mouse::Event::ButtonPressed(button)),
                iced::Event::Mouse(mouse::Event::ButtonReleased(button)),
            ],
        );
        snapshot.settle(app, App::view, &mut update);
    }

    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let scenes = [
            "cubuk-menusu",
            "sag-ekle",
            "sag-kaldir",
            "sag-sabit",
            "sag-baska",
            "yontemler",
            "aile",
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
                    // As on the web's pictures: Tümünü göster and Çizgi on the bar.
                    let _ = app.update(Message::QuickAccess("view.zoomExtents".into(), true));
                    let _ = app.update(Message::QuickAccess("tool.line".into(), true));
                    let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.settle(&mut app, App::view, &mut update);
                    let first_tab = find_text(&mut snapshot, &app, "Dosya").expect("the tabs");
                    let row = first_tab.center_y();
                    // Where the bar's buttons are, from the ribbon's own spacing: the
                    // first tab's padding (14), the bar's (8), its divider and gap (1 + 6),
                    // the ▾ (17 wide) and the buttons (24 wide, 1 apart).
                    let tab_left = first_tab.x - 14.0;
                    let more_left = tab_left - 8.0 - 1.0 - 6.0 - 17.0;
                    match scene {
                        "cubuk-menusu" => press(
                            &mut snapshot,
                            &mut app,
                            Point::new(more_left + 8.5, row),
                            mouse::Button::Left,
                        ),
                        // Kaydet, the first of five on the bar: fixed.
                        "sag-sabit" => press(
                            &mut snapshot,
                            &mut app,
                            Point::new(more_left - 5.0 * 25.0 + 12.0, row),
                            mouse::Button::Right,
                        ),
                        "sag-baska" => {
                            let tab = find_text(&mut snapshot, &app, "Çizim").expect("Çizim");
                            press(&mut snapshot, &mut app, tab.center(), mouse::Button::Right);
                        }
                        _ => {
                            let caption = match scene {
                                "sag-ekle" => "Kapalı alan",
                                "sag-kaldir" => "Çizgi",
                                "yontemler" => "Daire",
                                _ => "Dikdörtgen",
                            };
                            let at = find_text(&mut snapshot, &app, caption).expect(caption);
                            match scene {
                                "sag-ekle" | "sag-kaldir" => {
                                    press(
                                        &mut snapshot,
                                        &mut app,
                                        at.center(),
                                        mouse::Button::Right,
                                    );
                                }
                                // A large split's label opens its list.
                                "yontemler" => {
                                    press(
                                        &mut snapshot,
                                        &mut app,
                                        at.center(),
                                        mouse::Button::Left,
                                    );
                                }
                                // A small split's arrow is right of its label.
                                _ => {
                                    let arrow = Point::new(at.x + at.width + 10.0, at.center_y());
                                    press(&mut snapshot, &mut app, arrow, mouse::Button::Left);
                                }
                            }
                        }
                    }
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
