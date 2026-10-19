//! Katmanların sırası (docs/adr/0248 §1, `cad.layers.edit`'s `move`): a row of the Katmanlar tree dragged before,
//! after or into another row, and Sıra ▸ of a row's menu (Yukarı taşı, Aşağı taşı, En üste taşı, En alta taşı,
//! Gruptan çıkar). Each is one undo step “Katmanı taşı”; a locked node or group refuses it in the command's words.

use kentos_contracts::{LayerEditOperation, LayersEdit};
use kentos_interaction::Level;
use kentos_ui::widget::tree_view::Place;

use crate::app::App;

/// A step of Sıra ▸.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    Top,
    Bottom,
    /// Out of its group, just after it.
    Out,
}

impl App {
    /// The ids of the tree's rows as Katmanlar shows them now (the open groups' children, the search's rows).
    fn layer_row_ids(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let search = self.layer_search_state();
        crate::layer_tree::open_rows(doc.layers(), &search)
            .iter()
            .map(|row| row.node.id.clone())
            .collect()
    }

    /// A row dropped on another: before or after it among its siblings, or into it (a group), last.
    pub(crate) fn layer_dropped(&mut self, from: usize, to: usize, place: Place) {
        let rows = self.layer_row_ids();
        let (Some(source), Some(target)) = (rows.get(from).cloned(), rows.get(to).cloned()) else {
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let tree = doc.model.layers();
        let (parent, index) = match place {
            Place::Into => (Some(target.clone()), None),
            Place::Before | Place::After => {
                let (Some((to_parent, to_at)), Some((from_parent, from_at))) =
                    (tree.place_of(&target), tree.place_of(&source))
                else {
                    return;
                };
                let mut at = if place == Place::After {
                    to_at + 1
                } else {
                    to_at
                };
                // The place is the one it takes once it left its own (`LayerTree::move_place`).
                if from_parent == to_parent && from_at < to_at {
                    at -= 1;
                }
                (to_parent, Some(at))
            }
        };
        self.move_layer(source, parent, index);
    }

    /// Sıra ▸ on a node.
    pub(crate) fn layer_step(&mut self, id: String, step: Step) {
        let Some(doc) = &self.document else {
            return;
        };
        let tree = doc.model.layers();
        let Some((parent, at)) = tree.place_of(&id) else {
            return;
        };
        let (parent, index) = match step {
            Step::Up if at == 0 => return self.say(Level::Info, "Katman zaten en üstte."),
            Step::Up => (parent, Some(at - 1)),
            Step::Down => (parent, Some(at + 1)),
            Step::Top => (parent, Some(0)),
            Step::Bottom => (parent, None),
            Step::Out => {
                let Some(group) = parent else {
                    return self.say(Level::Info, "Katman bir grupta değil.");
                };
                let Some((above, group_at)) = tree.place_of(&group) else {
                    return;
                };
                (above, Some(group_at + 1))
            }
        };
        self.move_layer(id, parent, index);
    }

    /// `cad.layers.edit`'s move, said.
    fn move_layer(&mut self, id: String, parent: Option<String>, index: Option<usize>) {
        if let Some(why) = self.tree_locked() {
            self.warn(why);
            return;
        }
        let name = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&id).map(|n| n.name.clone()))
            .unwrap_or_default();
        let mut input = LayersEdit::new(LayerEditOperation::Move);
        input.layer = Some(id);
        input.parent = parent.clone();
        input.index = index.and_then(|i| u32::try_from(i).ok());
        let Some(done) = self.edit_layers(input) else {
            return;
        };
        if !done.changed {
            return;
        }
        let into = parent
            .and_then(|p| {
                self.document
                    .as_ref()
                    .and_then(|d| d.model.layers().get(&p).map(|n| n.name.clone()))
            })
            .map_or_else(|| " üst düzeyde".to_owned(), |g| format!(" “{g}” grubunda"));
        self.say(
            Level::Info,
            format!("“{name}”{into} yeni yerine taşındı. Ctrl+Z geri alır."),
        );
    }
}

#[cfg(test)]
mod tests {
    use kentos_ui::widget::tree_view::Place;

    use super::Step;
    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, last_said};
    use crate::layering::Event;

    /// The tree's ids by group, top first: each node as `group/id`.
    fn order(app: &App) -> Vec<String> {
        fn walk(nodes: &[kentos_contracts::LayerNode], above: &str, out: &mut Vec<String>) {
            for n in nodes {
                out.push(format!("{above}/{}", n.id));
                walk(&n.children, &n.id, out);
            }
        }
        let mut out = Vec::new();
        walk(app.document.as_ref().expect("open").layers(), "", &mut out);
        out
    }

    fn layer(app: &mut App, e: Event) {
        let _ = app.update(Message::Layer(e));
    }

    /// Sıra ▸ and a drag both move through `cad.layers.edit`'s move, each one undo step.
    #[test]
    fn a_layer_moves_by_its_menu_and_by_a_drag() {
        let mut app = app_with_drawing();
        let before = order(&app);
        let parsel = before
            .iter()
            .position(|n| n.ends_with("/parsel"))
            .expect("parsel");
        let bina = before
            .iter()
            .position(|n| n.ends_with("/bina"))
            .expect("bina");
        assert!(parsel < bina, "{before:?}");
        // Aşağı taşı: Parsel after Bina in Kadastro.
        layer(&mut app, Event::Order("parsel".into(), Step::Down));
        let now = order(&app);
        assert!(
            now.iter().position(|n| n.ends_with("/parsel"))
                > now.iter().position(|n| n.ends_with("/bina")),
            "{now:?}"
        );
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(order(&app), before, "one undo step");
        // A locked layer stays where it is, in the command's words.
        layer(&mut app, Event::Order("bina".into(), Step::Out));
        assert_eq!(order(&app), before);
        assert!(last_said(&app).contains("kilitli"), "{}", last_said(&app));
        // Gruptan çıkar: Parsel just after the Kadastro group, at the top.
        layer(&mut app, Event::Order("parsel".into(), Step::Out));
        let now = order(&app);
        assert_eq!(
            now,
            ["/layer-g", "layer-g/bina", "/parsel", "/cizim"],
            "{}",
            last_said(&app)
        );
        let _ = app.update(Message::Run("edit.undo"));
        // Dragged: the rows as the tree shows them; Çizim dropped into Kadastro.
        let rows = app.layer_row_ids();
        let from = rows.iter().position(|r| r == "cizim").expect("cizim row");
        let to = rows
            .iter()
            .position(|r| r == "layer-g")
            .expect("kadastro row");
        layer(&mut app, Event::Move(from, to, Place::Into));
        assert!(
            order(&app).contains(&"layer-g/cizim".to_owned()),
            "{:?}",
            order(&app)
        );
        // And back before Kadastro.
        let rows = app.layer_row_ids();
        let from = rows.iter().position(|r| r == "cizim").expect("cizim row");
        let to = rows
            .iter()
            .position(|r| r == "layer-g")
            .expect("kadastro row");
        layer(&mut app, Event::Move(from, to, Place::Before));
        let now = order(&app);
        assert!(now.contains(&"/cizim".to_owned()), "{now:?}");
        assert!(
            now.iter().position(|n| n == "/cizim") < now.iter().position(|n| n == "/layer-g"),
            "{now:?}"
        );
    }
}
