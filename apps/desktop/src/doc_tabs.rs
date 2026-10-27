//! Belge sekmeleri (docs/adr/0351 §3; plan/cad.md `CAD-REF-01`, `02`): more than one drawing open, one on screen.
//! A drawing set aside keeps what is its own (the document with its undo, its camera, its selection and its view
//! history) in its tab; the one on screen is the app's `document`, as always, so every tool, panel and command reads
//! the drawing in front and nothing else. Bringing a tab in front puts its drawing on screen the way an open does
//! (`show_document`: the geometry store rebuilt, the drawing's caches and late answers let go) and gives it back its
//! camera, selection and views.
//!
//! What runs on a drawing stays with it: a console run, an open, Blok düzenleyici hold the tabs where they are; a
//! cloud project is not set aside (its link follows the drawing on screen); a late answer of a drawing left (a save,
//! İşlemler, an agent's call) finds its drawing by its session or is dropped (`python`, `processing`, `agents`).

use iced::Element;
use iced::widget::container;
use kentos_render_wgpu::Camera;
use kentos_ui::icon::Icon;
use kentos_ui::widget::{Tab, Tabs};

use kentos_interaction::{Level, Selection, ViewHistory};

use crate::app::{App, Message, Then};
use crate::document::Document;
use crate::opening::Purpose;

/// A drawing set aside in its tab.
pub struct Parked {
    pub document: Document,
    camera: Camera,
    pub(crate) selection: Selection,
    views: ViewHistory,
    /// Its Etkin kat and Kat görünümü (docs/adr/0720 §2).
    level: (crate::active_level::Chosen, kentos_interaction::level_view::LevelView),
}

/// The drawings set aside, and where the one on screen stands among them.
#[derive(Default)]
pub struct DocTabs {
    pub parked: Vec<Parked>,
    /// The drawing on screen's place in the strip: `parked[..at]` before it, the others after.
    pub at: usize,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A tab of the strip pressed (its place among all).
    Show(usize),
    /// A tab's ×.
    Close(usize),
    /// The strip's +: Yeni sekmede aç.
    New,
}

pub const COMMANDS: [&str; 3] = ["file.openTab", "file.closeTab", "file.nextTab"];

fn msg(e: Event) -> Message {
    Message::DocTabs(e)
}

impl DocTabs {
    /// How many drawings are open, the one on screen counted.
    pub fn count(&self, on_screen: bool) -> usize {
        self.parked.len() + usize::from(on_screen)
    }

    /// The set aside drawing of this session.
    pub fn find_mut(&mut self, session: u64) -> Option<&mut Document> {
        self.parked.iter_mut().map(|p| &mut p.document).find(|d| d.session == session)
    }

    /// The set aside drawings with unsaved work, by name.
    pub fn unsaved(&self) -> Vec<String> {
        self.parked
            .iter()
            .filter(|p| p.document.dirty() && !p.document.is_database())
            .map(|p| p.document.name().to_owned())
            .collect()
    }
}

impl App {
    /// Why the drawing on screen cannot be set aside now; none when it can.
    pub(crate) fn tabs_held(&self) -> Option<String> {
        if self.block_editor.is_some() {
            return Some(crate::block_editor::OPEN.to_owned());
        }
        if self.python.running.is_some() {
            return Some("Python konsolunda kod çalışıyor; bitince ya da Durdur'dan sonra çizim değiştirilebilir.".into());
        }
        if self.opening.is_some() || self.cloud.opening.is_some() {
            return Some("Bir çizim açılıyor; açılış bitince çizim değiştirilebilir.".into());
        }
        if self.document.as_ref().is_some_and(|d| d.cloud_source().is_some()) {
            return Some(
                "Bulut projesi sekmede bekletilemez: bağlantısı ekrandaki çizimi izler. Yeni çizimi bu sekmede açın ya da projeyi kapatın."
                    .into(),
            );
        }
        None
    }

    /// The drawing on screen into its tab, the screen left empty for the next; false (and said) when it cannot go.
    pub(crate) fn park_active(&mut self) -> bool {
        if let Some(why) = self.tabs_held() {
            self.warn(why);
            return false;
        }
        if self.document.is_none() {
            return true;
        }
        // The selection is the drawing's, kept before the running tool lets go (Esc's work clears it).
        let selection = std::mem::replace(&mut self.selection, Selection::new());
        self.cancel();
        let Some(document) = self.document.take() else {
            return true;
        };
        let parked = Parked {
            document,
            camera: self.viewport.camera,
            selection,
            views: std::mem::replace(&mut self.view_history, ViewHistory::new()),
            level: (std::mem::take(&mut self.active_level), std::mem::take(&mut self.level_view)),
        };
        let at = self.doc_tabs.at.min(self.doc_tabs.parked.len());
        self.doc_tabs.parked.insert(at, parked);
        // The new drawing comes right after the one set aside.
        self.doc_tabs.at = at + 1;
        true
    }

    /// The drawing at `k` among all in front.
    fn show_tab(&mut self, k: usize) {
        if k == self.doc_tabs.at || self.document.is_none() {
            return;
        }
        let k_parked = if k > self.doc_tabs.at { k - 1 } else { k };
        if k_parked >= self.doc_tabs.parked.len() {
            return;
        }
        if !self.park_active() {
            return;
        }
        // Parking put the drawing on screen at `at - 1`: the one asked for shifted when it stood after.
        let index = if k_parked >= self.doc_tabs.at - 1 { k_parked + 1 } else { k_parked };
        let Parked { document, camera, selection, views, level } = self.doc_tabs.parked.remove(index);
        self.doc_tabs.at = index;
        let name = document.name().to_owned();
        self.show_document(document);
        self.viewport.camera = camera;
        self.selection = selection;
        self.view_history = views;
        self.give_level_state(level);
        self.output(format!("Önde: {name}."));
    }

    /// Çizimi kapat: the drawing on screen asked about first when it has unsaved work.
    fn close_tab(&mut self, k: usize) -> iced::Task<Message> {
        if k != self.doc_tabs.at {
            self.show_tab(k);
            if k != self.doc_tabs.at {
                return iced::Task::none();
            }
        }
        if let Some(why) = self.tabs_held().filter(|_| self.document.as_ref().is_none_or(|d| d.cloud_source().is_none())) {
            self.warn(why);
            return iced::Task::none();
        }
        self.leave(Then::CloseTab)
    }

    /// The drawing on screen goes; the tab after it (else before it) comes in front.
    pub(crate) fn close_tab_now(&mut self) {
        let name = self.document.as_ref().map(|d| d.name().to_owned());
        self.close_cloud_project();
        self.cancel();
        self.document = None;
        let next = if self.doc_tabs.at < self.doc_tabs.parked.len() {
            Some(self.doc_tabs.at)
        } else {
            self.doc_tabs.parked.len().checked_sub(1)
        };
        match next {
            Some(index) => {
                let Parked { document, camera, selection, views, level } = self.doc_tabs.parked.remove(index);
                self.doc_tabs.at = index;
                self.show_document(document);
                self.viewport.camera = camera;
                self.selection = selection;
                self.view_history = views;
                self.give_level_state(level);
            }
            None => {
                self.doc_tabs.at = 0;
                self.selection = Selection::new();
                self.view_history = ViewHistory::new();
                self.followed = None;
            }
        }
        if let Some(name) = name {
            self.say(Level::Info, format!("“{name}” kapatıldı."));
        }
    }

    pub(crate) fn doc_tabs_command(&mut self, id: &str) -> iced::Task<Message> {
        match id {
            "file.openTab" => {
                if self.document.is_some()
                    && let Some(why) = self.tabs_held()
                {
                    self.warn(why);
                    return iced::Task::none();
                }
                self.open_into(Purpose::Tab)
            }
            "file.closeTab" => {
                if self.document.is_none() {
                    self.output("Kapatılacak çizim yok.");
                    return iced::Task::none();
                }
                self.close_tab(self.doc_tabs.at)
            }
            "file.nextTab" => {
                let all = self.doc_tabs.count(self.document.is_some());
                if all < 2 {
                    self.output("Açık başka çizim yok; Yeni sekmede aç ile bir çizim daha açın.");
                } else {
                    self.show_tab((self.doc_tabs.at + 1) % all);
                }
                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    pub(crate) fn doc_tabs_event(&mut self, event: Event) -> iced::Task<Message> {
        match event {
            Event::Show(k) => {
                self.show_tab(k);
                iced::Task::none()
            }
            Event::Close(k) => self.close_tab(k),
            Event::New => self.doc_tabs_command("file.openTab"),
        }
    }

    /// The strip over the drawing while more than one drawing is open: each drawing's name, a dot where it has
    /// unsaved work, × and +.
    pub(crate) fn doc_tabs_view(&self) -> Option<Element<'_, Message>> {
        let doc = self.document.as_ref()?;
        if self.doc_tabs.parked.is_empty() {
            return None;
        }
        let (before, after) = self.doc_tabs.parked.split_at(self.doc_tabs.at.min(self.doc_tabs.parked.len()));
        let tab = |d: &Document| Tab::new(d.name().to_owned()).icon(Icon::Document).dirty(d.dirty());
        let tabs: Vec<Tab> = before
            .iter()
            .map(|p| tab(&p.document))
            .chain(std::iter::once(tab(doc)))
            .chain(after.iter().map(|p| tab(&p.document)))
            .collect();
        Some(
            container(
                Tabs::new(tabs, self.doc_tabs.at, |k| msg(Event::Show(k)))
                    .on_close(|k| msg(Event::Close(k)))
                    .on_new(msg(Event::New)),
            )
            .width(iced::Fill)
            .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, drawing};

    use super::Event;

    fn front(app: &App) -> u64 {
        app.document.as_ref().map(|d| d.session).expect("a drawing")
    }

    #[test]
    fn a_drawing_set_aside_keeps_its_camera_selection_and_undo_and_comes_back() {
        let mut app = app_with_drawing();
        let first = front(&app);
        let one = app
            .document
            .as_ref()
            .and_then(|d| d.model.entities().next().map(|e| kentos_domain::Slot(e.base().id)))
            .expect("an object");
        app.selection.set([one]);
        let camera = app.viewport.camera;
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(5))))));
        let second = front(&app);
        assert_ne!(first, second);
        assert_eq!(app.doc_tabs.count(true), 2);
        assert!(app.selection.ids().is_empty(), "the selection is the drawing's");
        assert!(app.doc_tabs_view().is_some(), "the strip shows with two drawings");
        // The first back in front, with its selection and camera.
        let _ = app.update(Message::DocTabs(Event::Show(0)));
        assert_eq!(front(&app), first);
        assert_eq!(app.selection.ids(), [one]);
        assert_eq!(app.viewport.camera, camera);
        // Sonraki çizim goes round.
        let _ = app.run("file.nextTab");
        assert_eq!(front(&app), second);
        let _ = app.run("file.nextTab");
        assert_eq!(front(&app), first);
    }

    #[test]
    fn a_save_that_ends_after_its_drawing_was_set_aside_is_its_own() {
        let mut app = app_with_drawing();
        let first = front(&app);
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(1))))));
        let path = std::env::temp_dir().join("kentos-sekme-kaydi.kcad");
        let _ = app.update(Message::Saved(Some(Ok(crate::app::Written {
            session: first,
            path: path.clone(),
            revision: 0,
        }))));
        assert_eq!(app.doc_tabs.parked[0].document.path.as_deref(), Some(path.as_path()));
        assert!(app.document.as_ref().is_some_and(|d| d.path.is_none()), "the drawing in front keeps no file");
    }

    #[test]
    fn closing_the_last_drawing_leaves_the_screen_empty_and_the_strip_goes() {
        let mut app = app_with_drawing();
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(2))))));
        app.close_tab_now();
        assert!(app.document.is_some());
        assert!(app.doc_tabs.parked.is_empty());
        assert!(app.doc_tabs_view().is_none(), "one drawing: no strip");
        app.close_tab_now();
        assert!(app.document.is_none());
    }
}
