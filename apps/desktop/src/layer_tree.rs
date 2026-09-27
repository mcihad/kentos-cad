//! The layer tree's rows, search and keyboard (the web's `TreeView` and the
//! LayersPanel's “Katman ara”; `ui/widgets/TreeView.ts`,
//! `ui/layers/LayersPanel.ts`; docs/adr/0075):
//!
//! - Katman ara keeps the nodes whose name holds the text (lower case, the
//!   Turkish way) and the groups above them, every such group open while it
//!   searches; with none left the tree says “Aramayla eşleşen katman yok.”.
//! - A pressed row gives the tree the keyboard (the web's focused tree) until
//!   a press on the drawing, the command line taking it, or a command from
//!   the ribbon or a menu:
//!   - ↑ ↓ Home End choose a row;
//!   - → opens a group, or goes into an open one; ← closes an open group,
//!     or goes to the group above;
//!   - Enter makes a layer active, or opens or closes a group;
//!   - Boşluk shows or hides; F2 renames; Delete removes, as the row's menu
//!     does. With no row chosen Delete deletes the drawing's selection.
//!
//!   The keys start from the rows the drawing's selection shows selected, as
//!   those are what is seen; a chosen row is scrolled into view. The other
//!   keys work as anywhere (shortcuts, typing into the command line).

use iced::keyboard::key::Named;
use iced::widget::container;
use iced::{Element, Fill, Task};
use kentos_contracts::{LayerNode, LayerNodeType};
use kentos_ui::widget::SearchBox;

use crate::app::{App, Message};
use crate::keys::KeyPress;
use crate::layering::Event;

/// Katman ara's field.
const LAYER_SEARCH: &str = "layer-search";

/// A row of the tree as shown: its depth, its node and whether the groups
/// above it are shown.
pub(crate) struct OpenRow<'a> {
    pub depth: usize,
    pub node: &'a LayerNode,
    pub parent_visible: bool,
}

/// The tree's rows as shown, depth first: the nodes the search keeps
/// (`query` as [`query`] makes it; empty keeps every node) and the children
/// of the open groups. The search keeps a match with everything under it (a
/// matching group shows its layers) and the groups above it; while it
/// searches every group it keeps is open (the web's `render`, since c63cd77).
pub(crate) fn open_rows<'a>(nodes: &'a [LayerNode], query: &str) -> Vec<OpenRow<'a>> {
    fn matches(node: &LayerNode, query: &str) -> bool {
        !query.is_empty() && lower_tr(&node.name).contains(query)
    }
    fn keep(node: &LayerNode, query: &str) -> bool {
        query.is_empty()
            || matches(node, query)
            || node.children.iter().any(|child| keep(child, query))
    }
    fn walk<'a>(
        nodes: &'a [LayerNode],
        query: &str,
        depth: usize,
        visible: bool,
        under: bool,
        rows: &mut Vec<OpenRow<'a>>,
    ) {
        for node in nodes.iter().filter(|node| under || keep(node, query)) {
            rows.push(OpenRow {
                depth,
                node,
                parent_visible: visible,
            });
            if open(node, query) {
                let under = under || matches(node, query);
                walk(
                    &node.children,
                    query,
                    depth + 1,
                    visible && node.visible,
                    under,
                    rows,
                );
            }
        }
    }
    let mut rows = Vec::new();
    walk(nodes, query, 0, true, false, &mut rows);
    rows
}

/// Whether a node's children are shown: a group with children that is open,
/// or any while searching.
fn open(node: &LayerNode, query: &str) -> bool {
    node.kind == LayerNodeType::Group
        && !node.children.is_empty()
        && (node.expanded || !query.is_empty())
}

/// The search as the tree compares it: trimmed, lower case the Turkish way.
pub(crate) fn query(text: &str) -> String {
    lower_tr(text.trim())
}

/// `toLocaleLowerCase('tr-TR')`: I is ı and İ is i, I with a combining dot
/// above is i; everything else as Unicode says (kentos-style-core's
/// `lower_tr`, which the desktop does not depend on).
pub(crate) fn lower_tr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'I' if chars.peek() == Some(&'\u{307}') => {
                chars.next();
                out.push('i');
            }
            'I' => out.push('ı'),
            'İ' => out.push('i'),
            c => out.push(c),
        }
    }
    out.to_lowercase()
}

impl App {
    /// The search box over the tree (the web's `panel__toolbar`): a field the
    /// panel's width, its magnifier inside on the left. ↓ takes the keyboard
    /// into the tree at its first row; Esc clears the text, and on an empty
    /// box gives the keyboard back to the drawing (the web's since c63cd77).
    pub(crate) fn layer_search_view(&self) -> Element<'_, Message> {
        let field = SearchBox::new(self.layer_query.clone(), "Katman ara", Message::LayerSearch)
            .id(LAYER_SEARCH)
            .fill()
            .height(28.0)
            .on_down(Message::LayerSearchDown);
        container(field).padding([6, 8]).width(Fill).into()
    }

    /// ↓ in Katman ara: the tree has the keyboard, its first row chosen
    /// (the web's `enterFirst`); the box lets go of the keys.
    pub(crate) fn layer_search_down(&mut self) -> Task<Message> {
        let Some(first) = self.tree_lines().first().map(|(id, ..)| id.clone()) else {
            return Task::none();
        };
        self.layers_keyboard = true;
        self.choose_row(first);
        crate::input::release_keyboard()
    }

    /// Katman ara: the tree shows what the text finds.
    pub(crate) fn layer_search(&mut self, text: String) {
        self.layer_query = text;
        // Typing in the search is not the tree's keyboard (the web's search box is outside it).
        self.layers_keyboard = false;
    }

    /// The tree's rows as shown now, their ids and depths, and whether each
    /// has children (the keys walk these).
    fn tree_lines(&self) -> Vec<(String, usize, bool)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        open_rows(doc.layers(), &query(&self.layer_query))
            .into_iter()
            .map(|row| {
                (
                    row.node.id.clone(),
                    row.depth,
                    !row.node.children.is_empty(),
                )
            })
            .collect()
    }

    /// A key while the tree has the keyboard: none when it is not the
    /// tree's, so it goes on as anywhere else. As on the web the modifiers
    /// do not matter.
    pub(crate) fn layer_key(&mut self, press: &KeyPress) -> Option<Task<Message>> {
        let key = press.named()?;
        let lines = self.tree_lines();
        // The rows the drawing's selection shows come first: they are what is seen selected.
        let chosen = if self.layers_follow {
            lines
                .iter()
                .position(|(id, ..)| self.selection_layers.contains(id))
        } else {
            None
        }
        .or_else(|| {
            let id = self.selected_layer.as_deref()?;
            lines.iter().position(|(line, ..)| line == id)
        });
        let current = chosen.and_then(|i| lines.get(i).map(|(id, ..)| id.clone()));
        let last = lines.len().checked_sub(1);
        let go = |app: &mut App, k: isize| {
            if let Some(last) = last {
                let k = k.clamp(0, last as isize) as usize;
                app.choose_row(lines[k].0.clone());
            }
        };
        let i = chosen.map_or(-1, |i| i as isize);
        match key {
            Named::ArrowDown => go(self, i + 1),
            Named::ArrowUp => go(self, i - 1),
            Named::Home => go(self, 0),
            Named::End => go(self, last.map_or(0, |l| l as isize)),
            // A group's own open state decides, as on the web (a search shows it open anyway).
            Named::ArrowRight => {
                if let (Some(i), Some(id)) = (chosen, &current)
                    && lines[i].2
                {
                    if self.expanded(id) {
                        go(self, i as isize + 1);
                    } else if let Some(doc) = &mut self.document {
                        doc.model.set_layer_expanded(id, true);
                    }
                }
            }
            Named::ArrowLeft => {
                if let (Some(i), Some(id)) = (chosen, &current) {
                    if lines[i].2 && self.expanded(id) {
                        if let Some(doc) = &mut self.document {
                            doc.model.set_layer_expanded(id, false);
                        }
                    } else if let Some(parent) = (0..i).rev().find(|&k| lines[k].1 < lines[i].1) {
                        go(self, parent as isize);
                    }
                }
            }
            Named::Enter => {
                if let Some(id) = current {
                    self.activate_row(&id);
                }
            }
            Named::Space => {
                if let Some(doc) = &mut self.document
                    && let Some(id) = current
                {
                    doc.model.toggle_layer_visible(&id);
                }
            }
            Named::F2 => {
                if let Some(id) = current {
                    return Some(self.layer_event(Event::Rename(id)));
                }
            }
            // Only a chosen row takes Delete: else it deletes the drawing's selection.
            Named::Delete => {
                let id = current?;
                return Some(self.layer_event(Event::Remove(id)));
            }
            _ => return None,
        }
        Some(Task::none())
    }

    /// Whether a group is open (the web's `isExpanded`).
    fn expanded(&self, id: &str) -> bool {
        self.document
            .as_ref()
            .and_then(|doc| doc.model.layers().get(id))
            .is_some_and(|node| node.expanded)
    }

    /// The keyboard chose a row: it is the tree's selected row, scrolled into view.
    fn choose_row(&mut self, id: String) {
        self.selected_layer = Some(id.clone());
        self.layers_follow = false;
        self.layer_reveal = Some(id);
    }

    /// Enter or a double click on a row (the web's `onActivate`): a layer
    /// becomes the active one, a group opens or closes.
    pub(crate) fn activate_row(&mut self, id: &str) {
        let Some(doc) = &mut self.document else {
            return;
        };
        match doc.model.layers().get(id).map(|n| (n.kind, n.expanded)) {
            Some((LayerNodeType::Layer, _)) => {
                doc.model.set_active_layer(id);
            }
            Some((LayerNodeType::Group, expanded)) => doc.model.set_layer_expanded(id, !expanded),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Key, Modifiers};

    use super::*;
    use crate::files_testing::app_with_drawing;
    use crate::viewport;

    fn key(app: &mut App, named: Named) {
        let _ = app.update(Message::Key(KeyPress {
            key: Key::Named(named),
            physical: Physical::Unidentified(NativeCode::Unidentified),
            modifiers: Modifiers::default(),
            text: None,
            repeat: false,
        }));
    }

    fn ids(app: &App) -> Vec<String> {
        let doc = app.document.as_ref().expect("open");
        open_rows(doc.layers(), &query(&app.layer_query))
            .iter()
            .map(|row| row.node.id.clone())
            .collect()
    }

    fn chosen(app: &App) -> Option<&str> {
        app.selected_layer.as_deref()
    }

    /// The sample's tree: Kadastro (`layer-g`) with Parsel and Bina, then
    /// Çizim. The search keeps the matching nodes with what is under them
    /// and the groups above them, open; the Turkish I's fold. ↓ in the box
    /// gives the tree the keyboard at its first row.
    #[test]
    fn the_search_keeps_what_it_finds_and_the_groups_above() {
        let mut app = app_with_drawing();
        assert_eq!(ids(&app), ["layer-g", "parsel", "bina", "cizim"]);
        let _ = app.update(Message::LayerSearch("  Bİ ".into()));
        assert_eq!(ids(&app), ["layer-g", "bina"]);
        // Closed, the group still shows what the search found under it.
        let doc = app.document.as_mut().expect("open");
        doc.model.set_layer_expanded("layer-g", false);
        assert_eq!(ids(&app), ["layer-g", "bina"]);
        // A matching group keeps its layers (the web's since c63cd77).
        let _ = app.update(Message::LayerSearch("kadastro".into()));
        assert_eq!(ids(&app), ["layer-g", "parsel", "bina"]);
        let _ = app.update(Message::LayerSearch("yok".into()));
        assert!(ids(&app).is_empty());
        let _ = app.update(Message::LayerSearch(String::new()));
        assert_eq!(
            ids(&app),
            ["layer-g", "cizim"],
            "closed again without the search"
        );
        assert_eq!(query("  İMAR "), "imar");
        assert_eq!(query("IŞIK"), "ışık");

        let _ = app.update(Message::LayerSearch("bina".into()));
        let _ = app.update(Message::LayerSearchDown);
        assert!(app.layers_keyboard);
        assert_eq!(chosen(&app), Some("layer-g"), "the first listed row");
    }

    /// A pressed row gives the tree the keyboard: ↑ ↓ Home End walk the
    /// rows, ← goes to the group above and closes an open group, → opens it
    /// and goes in; Enter makes a layer active, Boşluk hides it, F2 renames
    /// it, Delete says why it cannot go (locked) or asks to remove it with
    /// its objects.
    #[test]
    fn a_pressed_row_takes_the_keys() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::LayerSelected("parsel".into()));
        assert!(app.layers_keyboard);
        key(&mut app, Named::ArrowDown);
        assert_eq!(chosen(&app), Some("bina"));
        assert_eq!(app.layer_reveal.as_deref(), Some("bina"), "kept in view");
        key(&mut app, Named::ArrowDown);
        key(&mut app, Named::ArrowDown);
        assert_eq!(chosen(&app), Some("cizim"), "the last row stays");
        key(&mut app, Named::Home);
        assert_eq!(chosen(&app), Some("layer-g"));
        key(&mut app, Named::End);
        key(&mut app, Named::ArrowUp);
        assert_eq!(chosen(&app), Some("bina"));
        key(&mut app, Named::ArrowLeft);
        assert_eq!(chosen(&app), Some("layer-g"), "to the group above");
        key(&mut app, Named::ArrowLeft);
        assert_eq!(ids(&app), ["layer-g", "cizim"], "closed");
        key(&mut app, Named::ArrowRight);
        assert_eq!(ids(&app), ["layer-g", "parsel", "bina", "cizim"], "open");
        key(&mut app, Named::ArrowRight);
        assert_eq!(chosen(&app), Some("parsel"), "and in");

        key(&mut app, Named::Enter);
        let layers = |app: &App| app.document.as_ref().expect("open").model.layers().clone();
        assert_eq!(layers(&app).active(), "parsel");
        key(&mut app, Named::Space);
        assert!(!layers(&app).get("parsel").expect("parsel").visible);
        key(&mut app, Named::F2);
        assert_eq!(app.renaming, Some(("parsel".into(), "Parsel".into())));
        let _ = app.update(Message::Layer(Event::RenameCancel));
        // Enter on a group opens or closes it.
        key(&mut app, Named::Home);
        key(&mut app, Named::Enter);
        assert_eq!(ids(&app), ["layer-g", "cizim"]);
        key(&mut app, Named::Enter);
        key(&mut app, Named::End);
        key(&mut app, Named::ArrowUp);
        assert_eq!(chosen(&app), Some("bina"));
        // Delete is the row menu's Sil, with its refusals: the sample's Bina is locked.
        key(&mut app, Named::Delete);
        assert_eq!(
            crate::files_testing::last_said(&app),
            "“Bina” katmanı kilitli; silinemez. Kilidi Katmanlar panelinden açın."
        );
        let _ = app.update(Message::LayerLocked("bina".into()));
        key(&mut app, Named::Delete);
        assert_eq!(app.removing_layer.as_deref(), Some("bina"));
        assert_eq!(app.dialog, Some(crate::app::Dialog::RemoveLayer));
    }

    /// The keys start from the rows the drawing's selection shows; with no
    /// row chosen Delete is the drawing's.
    #[test]
    fn the_keys_start_from_the_selection_rows_and_delete_needs_a_row() {
        let mut app = app_with_drawing();
        let slot = {
            let model = &app.document.as_ref().expect("open").model;
            model
                .entities()
                .find(|e| e.base().layer_id == "bina")
                .map(|e| kentos_domain::Slot(e.base().id))
                .expect("an object on Bina")
        };
        app.selection.set([slot]);
        app.follow_selection_layers();
        assert!(app.layers_follow);
        app.layers_keyboard = true;
        key(&mut app, Named::ArrowDown);
        assert_eq!(chosen(&app), Some("cizim"));
        assert!(!app.layers_follow, "the keys chose a row");

        let mut app = app_with_drawing();
        app.layers_keyboard = true;
        let delete = KeyPress {
            key: Key::Named(Named::Delete),
            physical: Physical::Unidentified(NativeCode::Unidentified),
            modifiers: Modifiers::default(),
            text: None,
            repeat: false,
        };
        assert!(
            app.layer_key(&delete).is_none(),
            "no row chosen: not the tree's"
        );
    }

    /// A press on the drawing, the command line or a command from the ribbon
    /// takes the keyboard back; typing in the search is not the tree's.
    #[test]
    fn the_drawing_the_command_line_or_a_command_takes_the_keys_back() {
        let mut app = app_with_drawing();
        for take in 0..4 {
            let _ = app.update(Message::LayerSelected("parsel".into()));
            assert!(app.layers_keyboard);
            match take {
                0 => {
                    let at = iced::Point::new(300.0, 200.0);
                    let _ = app.update(Message::Viewport(viewport::Event::Pressed(at)));
                    let _ = app.update(Message::Viewport(viewport::Event::Released(at)));
                }
                1 => {
                    let _ = app.update(Message::CommandFocus(true));
                    let _ = app.update(Message::CommandFocus(false));
                }
                2 => {
                    let _ = app.update(Message::Run("view.zoomExtents"));
                }
                _ => {
                    let _ = app.update(Message::LayerSearch("p".into()));
                    let _ = app.update(Message::LayerSearch(String::new()));
                }
            }
            assert!(!app.layers_keyboard, "taken back ({take})");
            key(&mut app, Named::ArrowDown);
            assert_eq!(
                chosen(&app),
                Some("parsel"),
                "the tree did not move ({take})"
            );
        }
    }
}

/// Pictures for the owner: Katman ara with “bi” typed (the tree keeps Bina
/// under its group) and one that finds nothing; a row chosen by the keys;
/// `.run/shots/katman-ara-*`, `katman-klavye-*`.
/// `cargo test -p kentos-desktop layer_tree::screens -- --ignored --nocapture`
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Key, Modifiers};
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["katman-ara", "katman-ara-yok", "katman-klavye"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "katman-ara" => {
                        let _ = app.update(Message::LayerSearch("bi".into()));
                    }
                    "katman-ara-yok" => {
                        let _ = app.update(Message::LayerSearch("yol".into()));
                    }
                    _ => {
                        let _ = app.update(Message::LayerSelected("parsel".into()));
                        for named in [Named::ArrowDown, Named::ArrowDown] {
                            let _ = app.update(Message::Key(KeyPress {
                                key: Key::Named(named),
                                physical: Physical::Unidentified(NativeCode::Unidentified),
                                modifiers: Modifiers::default(),
                                text: None,
                                repeat: false,
                            }));
                        }
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
