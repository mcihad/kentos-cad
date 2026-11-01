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
        // The selection is the drawing's; the unfinished interaction is not parked with it.
        let selection = std::mem::replace(&mut self.selection, Selection::new());
        self.leave_document_interaction();
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
        self.leave_document_interaction();
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

    fn pending_hatch_value(app: &mut App) {
        let _ = app.start_tool("hatch");
        assert_eq!(app.with_tool(|s, cx| s.input("Ö", cx)), Some(true));
        assert!(app.session.prompt().step.contains("ölçeğini yazın"));
    }

    #[test]
    fn scoped_replies_keep_document_jobs_but_reject_old_tool_runs_and_drawings() {
        let mut app = app_with_drawing();
        let document = front(&app);
        let _ = app.start_tool("hatch");
        let run = app.session.runs();
        let reply = |run, text: &str| Message::ScopedReply {
            document,
            run,
            reply: Box::new(Message::CommandInput(text.into())),
        };
        let _ = app.start_tool("line");
        let _ = app.update(reply(Some(run), "old tool"));
        assert_ne!(app.command_input, "old tool");
        let _ = app.update(reply(None, "document job"));
        assert_eq!(app.command_input, "document job");
        app.show_document(drawing(1));
        let _ = app.update(reply(None, "old document"));
        assert_ne!(app.command_input, "old document");
    }

    #[test]
    fn leaving_a_drawing_discards_a_block_definition_dialog_and_its_late_create() {
        let mut app = app_with_drawing();
        let slots: Vec<_> = app
            .document
            .as_ref()
            .unwrap()
            .model
            .entities()
            .take(2)
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        app.selection.set(slots);
        app.open_block_define(kentos_interaction::Vec2::new(0., 0.));
        assert_eq!(app.dialog, Some(crate::app::Dialog::BlockDefine));
        app.show_document(drawing(1));
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        assert!(app.dialog.is_none());
        let _ = app.update(Message::Blocks(crate::blocks::Event::Create));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }

    #[test]
    fn a_late_file_reply_cannot_feed_a_new_run_or_another_document() {
        use crate::app::Picker;
        use crate::files_testing::{drive, scratch};
        let folder = scratch("late-tool-file");
        let path = folder.join("old.txt");
        std::fs::write(&path, "Old drawing only").unwrap();
        for replace_document in [false, true] {
            let mut app = app_with_drawing();
            app.picker = Picker::File(path.clone());
            let _ = app.start_tool("placeTextFile");
            let late = app.text_file_tasks();
            if replace_document {
                app.show_document(drawing(1));
            }
            let _ = app.start_tool("placeTextFile");
            // Do not request a second file while driving the deliberately delayed first reply.
            app.text_file_wanted = false;
            let before = app.session.prompt().text();
            let snapshot = app.document.as_ref().unwrap().model.to_snapshot_v2();
            drive(&mut app, late);
            assert_eq!(
                app.session.prompt().text(),
                before,
                "old file must not supply the new run"
            );
            assert_eq!(
                app.document.as_ref().unwrap().model.to_snapshot_v2(),
                snapshot
            );
            app.text_file_wanted = true;
            let valid = app.text_file_tasks();
            drive(&mut app, valid);
            assert!(
                app.session.prompt().text().contains("old.txt"),
                "the current run receives its own file"
            );
        }
    }

    #[test]
    fn an_attribute_dialog_cannot_place_into_the_next_drawing() {
        let mut app = app_with_drawing();
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!("../../../fixtures/interaction/v1/block-attributes.kcad")).unwrap();
        app.show_document(crate::document::Document::new(snapshot, None).unwrap());
        let block = app.document.as_ref().unwrap().model.blocks()[0].id;
        app.open_attribute_values(block);
        assert!(app.attribute_values.is_some());
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.update(Message::CapturedEscape);
        assert!(app.attribute_values.is_none(), "one Esc cancels even when a text field consumed it");
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
        app.open_attribute_values(block);
        app.show_document(drawing(1));
        assert!(app.attribute_values.is_none());
        assert!(app.dialog.is_none());
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.update(Message::AttributeValues(crate::attribute_values::Event::Place));
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
    }

    #[test]
    fn opening_another_drawing_drops_the_whole_command_not_just_its_value_step() {
        let mut app = app_with_drawing();
        pending_hatch_value(&mut app);
        let revision = app.document.as_ref().unwrap().model.revision();
        let next = drawing(2);
        let next_revision = next.model.revision();
        let next_count = next.model.len();
        let _ = app.update(Message::Opened(Some(Ok(Box::new(next)))));
        assert!(
            !app.session.is_running(),
            "a value prompt must not become a hatch command on the new drawing"
        );
        assert!(app.session.preview(&app.format()).is_none());
        assert_eq!(
            app.document.as_ref().unwrap().model.revision(),
            next_revision
        );
        assert_eq!(app.document.as_ref().unwrap().model.len(), next_count);
        assert_eq!(revision, next_revision, "neither drawing was edited");
        assert_eq!(
            app.session.last(),
            Some("hatch"),
            "repeat remains available, without its old draft"
        );
    }

    #[test]
    fn changing_tabs_drops_the_transparent_command_and_its_suspended_parent() {
        let mut app = app_with_drawing();
        let first = front(&app);
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(2))))));
        pending_hatch_value(&mut app);
        let _ = app.start_tool("pan");
        assert_eq!(app.session.tool_id(), "hatch", "pan suspends the hatch");
        let _ = app.update(Message::DocTabs(Event::Show(0)));
        assert_eq!(front(&app), first);
        assert!(
            !app.session.is_running(),
            "neither the child nor its parent may follow the tab"
        );
        assert!(app.session.preview(&app.format()).is_none());
        let _ = app.update(Message::DocTabs(Event::Show(1)));
        assert!(
            !app.session.is_running(),
            "returning to the source tab does not resume a discarded draft"
        );
        assert_eq!(app.document.as_ref().unwrap().model.len(), 15);
        assert!(!app.document.as_ref().unwrap().dirty());
    }

    #[test]
    fn closing_the_last_drawing_drops_a_pending_value_and_drawing_aids() {
        let mut app = app_with_drawing();
        pending_hatch_value(&mut app);
        let point = kentos_interaction::Vec2::new(10.0, 20.0);
        app.tracking.acquire_point(point);
        app.locks.lock_length(3.0, point);
        app.snap_once = Some((kentos_interaction::SnapKind::Endpoint, "Uç nokta"));
        app.command_input = "yarım girdi".into();
        app.close_tab_now();
        assert!(app.document.is_none());
        assert!(!app.session.is_running());
        assert!(app.tracking.acquired().is_empty());
        assert!(!app.locks.any());
        assert!(app.snap_once.is_none());
        assert!(app.command_input.is_empty());
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
