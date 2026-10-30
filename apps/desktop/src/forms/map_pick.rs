//! Haritadan seç (docs/adr/0759 §1): a form's list from another layer takes its value from an object
//! clicked on the map. The window steps aside while the pick runs (a docked form stays), the click takes
//! one object of the list's layer only, and the form's core judges it as the list would: its value is
//! written as if chosen from the list, or why it gives none is said and the value stays.

use iced::Task;
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_interaction::pick_objects::PickObjects;

use crate::app::{App, Message};
use crate::forms::entry::{Entry, Event};

/// Haritadan seç while it runs.
pub struct FormPick {
    /// The form's item the value is for.
    item: String,
    /// The window, put aside while the map is clicked; none for a docked form.
    entry: Option<Entry>,
    /// The selection before the pick, given back after it.
    before: Vec<Slot>,
}

impl App {
    /// Haritadan seç pressed on the item `item` of the open form.
    pub(crate) fn pick_on_map(&mut self, item: &str) -> Task<Message> {
        let (Some(e), Some(doc)) = (self.form_entry.as_ref(), self.document.as_ref()) else {
            return Task::none();
        };
        let Some((layer, _, label)) = e.map_pick_of(item) else {
            return Task::none();
        };
        let tree = doc.model.layers();
        let Some(node) = tree.get(&layer) else {
            self.say(
                Level::Warn,
                "Listenin katmanı çizimde yok; formun listesini Formlar penceresinde düzeltin.",
            );
            return Task::none();
        };
        let name = node.name.clone();
        if tree.is_frozen(&layer) || !tree.is_visible(&layer) {
            self.say(
                Level::Warn,
                format!(
                    "“{name}” katmanı görünmüyor; haritadan seçmek için katmanı açın ya da çözün."
                ),
            );
            return Task::none();
        }
        let docked = e.docked;
        let entry = if docked { None } else { self.form_entry.take() };
        self.form_pick = Some(FormPick {
            item: item.to_owned(),
            entry,
            before: self.selection.ids().to_vec(),
        });
        self.selection.clear();
        self.field = None;
        self.snap = None;
        self.say(
            Level::Command,
            format!("Formla düzenle: “{label}” için “{name}” katmanından bir nesne seçin"),
        );
        self.session
            .run(Box::new(PickObjects::on_layer(label, layer)));
        self.with_tool(|s, cx| s.activate(cx));
        Task::none()
    }

    /// The object picked (or none, Esc): the window comes back and the value is written, or why not is
    /// said. Returns false when no form was picking.
    pub(crate) fn form_entry_picked(&mut self, keep: bool) -> bool {
        let Some(pick) = self.form_pick.take() else {
            return false;
        };
        let picked = keep
            .then(|| self.selection.ids().first().copied())
            .flatten();
        self.selection.set(pick.before);
        if let Some(entry) = pick.entry {
            self.form_entry = Some(entry);
        }
        let (Some(slot), Some(doc), Some(e)) =
            (picked, self.document.as_ref(), self.form_entry.as_ref())
        else {
            return true;
        };
        let (Some(uid), Some(object)) = (doc.model.uid(slot), doc.model.get(slot)) else {
            return true;
        };
        let Some((_, key, _)) = e.map_pick_of(&pick.item) else {
            return true;
        };
        match e.map_picked(&pick.item, &uid.to_string(), &object.base().attrs) {
            Some(Ok((code, _))) => {
                let _ = self.form_entry_event(Event::Value(key, code));
            }
            Some(Err(miss)) => self.say(Level::Warn, miss.message()),
            None => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use kentos_domain::Slot;

    use crate::app::{App, Message};
    use crate::forms::entry::Event;

    /// The building form of the sources scene on its first building (in parcel 101/5).
    fn building() -> App {
        let mut app = crate::files_testing::app_with_drawing();
        crate::form_scenes::sources(&mut app);
        app.selection.set([Slot(5)]);
        crate::tools_scenes::run(&mut app, "data.formEdit");
        assert!(app.form_entry.is_some(), "the form is open");
        app
    }

    fn typed(app: &App) -> Option<String> {
        app.form_entry.as_ref().and_then(|e| e.value_of("Parsel"))
    }

    /// Haritadan seç on the parcel item and a click on the map at `at` (from the drawing's origin).
    fn picked(app: &mut App, at: [f64; 2]) {
        let _ = app.update(Message::FormEntry(Event::PickOnMap("parsel".into())));
        assert!(app.form_entry.is_none(), "the window steps aside");
        assert!(app.form_pick.is_some());
        crate::tools_scenes::click(app, at);
        assert!(app.form_pick.is_none(), "the click ends the pick");
    }

    #[test]
    fn the_list_item_offers_a_pick_and_a_read_only_one_does_not() {
        let app = building();
        let e = app.form_entry.as_ref().expect("the form");
        assert!(e.map_pick_of("parsel").is_some());
        assert!(e.map_pick_of("kat").is_none(), "no list");
        assert!(
            e.map_pick_of("mahalle").is_some(),
            "a list without geometry"
        );
    }

    #[test]
    fn the_parcel_holding_the_building_gives_its_value() {
        let mut app = building();
        // Inside 101/5, away from the building on it.
        picked(&mut app, [3.0, 21.0]);
        assert!(app.form_entry.is_some(), "the window is back");
        let uid = app
            .document
            .as_ref()
            .and_then(|d| d.model.uid(Slot(1)))
            .expect("the parcel's id");
        assert_eq!(typed(&app), Some(uid.to_string()));
        assert_eq!(app.selection.ids(), [Slot(5)], "the selection given back");
    }

    #[test]
    fn a_parcel_apart_gives_nothing_and_says_why() {
        let mut app = building();
        // Inside 101/12, beside it.
        picked(&mut app, [45.0, 12.0]);
        assert_eq!(typed(&app), None);
        assert!(
            crate::files_testing::last_said(&app).contains("kaydı içermiyor"),
            "{}",
            crate::files_testing::last_said(&app)
        );
    }

    #[test]
    fn esc_brings_the_window_back_untouched() {
        let mut app = building();
        let _ = app.update(Message::FormEntry(Event::PickOnMap("parsel".into())));
        crate::tools_scenes::click(&mut app, [100.0, 100.0]);
        assert!(app.form_pick.is_some(), "a click on nothing takes nothing");
        app.cancel();
        assert!(app.form_pick.is_none());
        assert!(app.form_entry.is_some());
        assert_eq!(typed(&app), None);
    }

    #[test]
    fn a_hidden_list_layer_is_not_picked_from() {
        let mut app = building();
        if let Some(doc) = app.document.as_mut() {
            doc.model.set_layer_visible("parsel", false);
        }
        let _ = app.update(Message::FormEntry(Event::PickOnMap("parsel".into())));
        assert!(app.form_entry.is_some(), "the window stays");
        assert!(app.form_pick.is_none());
        assert!(crate::files_testing::last_said(&app).contains("görünmüyor"));
    }
}
