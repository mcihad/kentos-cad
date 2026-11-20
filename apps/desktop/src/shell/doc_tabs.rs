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
    /// Its Hesap windows' tables, solutions and names (UR2-049: one drawing's levelling went on in the next), its
    /// window closed.
    calc: crate::calc::Calc,
    /// Its Nokta name series, code and height (UR2-060).
    point: (kentos_interaction::Name, kentos_interaction::Name, Option<f64>),
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
        // An extension's command that may write holds its drawing as the console does (docs/adr/8057 §125): set aside
        // mid-run, its undo step stayed open on it and took the user's later edits there.
        if let Some(what) = self.extension_writing() {
            return Some(format!(
                "{what} çalışıyor; bitince ya da Durdur'dan sonra çizim değiştirilebilir."
            ));
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
            level: (
                std::mem::take(&mut self.arch.active_level),
                std::mem::take(&mut self.level_view),
            ),
            calc: std::mem::take(&mut self.gis.calc).set_aside(),
            point: self.memory.take_point(),
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
        let Parked { document, camera, selection, views, level, calc, point } = self.doc_tabs.parked.remove(index);
        self.doc_tabs.at = index;
        let name = document.display_name().into_owned();
        self.show_document(document);
        self.gis.calc = calc;
        self.memory.give_point(point);
        self.viewport.put_camera(camera);
        self.selection = selection;
        self.view_history = views;
        self.give_level_state(level);
        self.output(format!("Önde: {name}."));
    }

    /// Çizimi kapat: the drawing on screen asked about first when it has unsaved work.
    fn close_tab(&mut self, k: usize) -> iced::Task<Message> {
        if k != self.doc_tabs.at && self.document.is_some() {
            // A tab behind, by its own drawing (UR2-059: bringing it in front first ended the front drawing's command,
            // its corners lost, even when the question was then turned down).
            let kp = if k > self.doc_tabs.at { k - 1 } else { k };
            let Some(behind) = self.doc_tabs.parked.get(kp).map(|p| &p.document) else {
                return iced::Task::none();
            };
            let name = behind.display_name().into_owned();
            // Nothing unsaved, not a cloud project's: it goes where it stands, the front drawing as it is.
            if !behind.dirty() && behind.cloud_source().is_none() {
                self.doc_tabs.parked.remove(kp);
                if kp < self.doc_tabs.at {
                    self.doc_tabs.at -= 1;
                }
                self.say(Level::Info, format!("“{name}” kapatıldı."));
                return iced::Task::none();
            }
            // Its unsaved work is asked about in front: a command running here is not ended for that.
            if self.session.is_running() {
                let running = self.session.prompt().tool.unwrap_or("Komut");
                self.warn(format!(
                    "“{name}” sekmesinde kaydedilmemiş değişiklik var; kapatmadan önce sorulacak. Öndeki {running} sürüyor: bitirin ya da Esc'ye basın, sonra sekmeyi kapatın."
                ));
                return iced::Task::none();
            }
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
                let Parked { document, camera, selection, views, level, calc, point } = self.doc_tabs.parked.remove(index);
                self.doc_tabs.at = index;
                self.show_document(document);
                self.gis.calc = calc;
                self.memory.give_point(point);
                self.viewport.put_camera(camera);
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

    /// The drawing set aside in another tab whose file is `path` (by its canonical place: another folder's file of the
    /// same name is not it, a link to it is): its name and file (UR2-056: Farklı kaydet wrote over it).
    pub(crate) fn open_in_another_tab(&self, path: &std::path::Path) -> Option<(String, std::path::PathBuf)> {
        let want = canonical(path);
        self.doc_tabs
            .parked
            .iter()
            .find(|p| is_file(&p.document, &want))
            .map(|p| (p.document.display_name().into_owned(), p.document.path.clone().unwrap_or_default()))
    }

    /// The drawing whose file is `path` (by its canonical place), brought in front if it is open, said; false when
    /// no open drawing is that file.
    pub(crate) fn show_open_file(&mut self, path: &std::path::Path) -> bool {
        let want = canonical(path);
        let same = |d: &Document| is_file(d, &want);
        let front = self.document.as_ref().is_some_and(same);
        let at = if front {
            Some(self.doc_tabs.at)
        } else {
            self.doc_tabs.parked.iter().position(|p| same(&p.document)).map(|k| if k >= self.doc_tabs.at { k + 1 } else { k })
        };
        let Some(k) = at else {
            return false;
        };
        if !front {
            self.show_tab(k);
        }
        let name = self.document.as_ref().map(|d| d.name().to_owned()).unwrap_or_default();
        self.say(
            Level::Info,
            format!("“{name}” zaten açık; sekmesine geçildi. Aynı dosyanın iki kopyası birbirinin kaydını silerdi."),
        );
        true
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
        let docs: Vec<&Document> = before
            .iter()
            .map(|p| &p.document)
            .chain(std::iter::once(doc))
            .chain(after.iter().map(|p| &p.document))
            .collect();
        let titles = tab_titles(&docs);
        let tabs: Vec<Tab> = docs
            .iter()
            .zip(titles)
            .map(|(d, title)| Tab::new(title).icon(Icon::Document).dirty(d.dirty()))
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

/// The tabs' names: each drawing's own, and where two say the same, what tells them apart: the file's name with its
/// extension (a.dxf, a.dwg, a.kcad), then its folder; a drawing with no file its place among those of the name.
pub(crate) fn tab_titles(docs: &[&Document]) -> Vec<String> {
    let count = |name: &str| docs.iter().filter(|d| d.name() == name).count();
    let file = |d: &Document| {
        d.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy().into_owned())
    };
    let mut out: Vec<String> = docs
        .iter()
        .map(|d| match (count(d.name()) > 1, file(d)) {
            (true, Some(f)) => f,
            _ => d.name().to_owned(),
        })
        .collect();
    // Still the same (one file name in two folders, or no file): the folder, else a number.
    let firsts = out.clone();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (k, d) in docs.iter().enumerate() {
        if firsts.iter().filter(|t| **t == firsts[k]).count() < 2 {
            continue;
        }
        let folder = d
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy().into_owned());
        out[k] = match folder {
            Some(folder) => format!("{} — {folder}", firsts[k]),
            None => {
                let n = seen.entry(firsts[k].clone()).or_insert(0);
                *n += 1;
                format!("{} ({n})", firsts[k])
            }
        };
    }
    // Projects may have the same filename and parent directory name under different roots.
    let short = out.clone();
    for (k, doc) in docs.iter().enumerate() {
        if let Some(path) = doc.path.as_deref()
            && short.iter().filter(|title| *title == &short[k]).take(2).count() == 2
        {
            out[k] = path.to_string_lossy().into_owned();
        }
    }
    out
}

/// A file's place with its links followed; as it is when it cannot be read (not written yet).
fn canonical(p: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Whether the drawing's own file (not a cloud project's copy) is at the canonical place `want`.
fn is_file(d: &Document, want: &std::path::Path) -> bool {
    d.path.as_deref().is_some_and(|p| canonical(p) == want) && d.cloud_source().is_none()
}

#[cfg(test)]
mod tests {
    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, drawing};

    /// UR-086: two drawings of one name are told apart on their tabs by their files' extensions, then folders, then
    /// a number; a name alone stays as it is.
    #[test]
    fn tabs_of_one_name_are_told_apart() {
        let named = |path: Option<&str>| {
            let mut d = drawing(0);
            d.model.set_name("kentos-alan-iliski");
            d.path = path.map(std::path::PathBuf::from);
            d
        };
        let (a, b, c) = (
            named(Some("/tmp/qa/kentos-alan-iliski.dxf")),
            named(Some("/tmp/qa/kentos-alan-iliski.dwg")),
            drawing(0),
        );
        let titles = super::tab_titles(&[&a, &b, &c]);
        assert_eq!(titles[0], "kentos-alan-iliski.dxf");
        assert_eq!(titles[1], "kentos-alan-iliski.dwg");
        assert_eq!(titles[2], c.name());
        let (d, e) = (named(Some("/tmp/bir/kentos-alan-iliski.kcad")), named(Some("/tmp/iki/kentos-alan-iliski.kcad")));
        assert_eq!(
            super::tab_titles(&[&d, &e]),
            ["kentos-alan-iliski.kcad — bir", "kentos-alan-iliski.kcad — iki"]
        );
        let h = named(Some("/tmp/baska/bir/kentos-alan-iliski.kcad"));
        assert_eq!(
            super::tab_titles(&[&d, &h]),
            ["/tmp/bir/kentos-alan-iliski.kcad", "/tmp/baska/bir/kentos-alan-iliski.kcad"]
        );
        let (f, g) = (named(None), named(None));
        assert_eq!(super::tab_titles(&[&f, &g]), ["kentos-alan-iliski (1)", "kentos-alan-iliski (2)"]);
    }

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
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!("../../../../fixtures/interaction/v1/block-attributes.kcad")).unwrap();
        app.show_document(crate::document::Document::new(snapshot, None).unwrap());
        let block = app.document.as_ref().unwrap().model.blocks()[0].id;
        app.open_attribute_values(block);
        assert!(app.gis.attribute_values.is_some());
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.update(Message::CapturedEscape);
        assert!(
            app.gis.attribute_values.is_none(),
            "one Esc cancels even when a text field consumed it"
        );
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        app.open_attribute_values(block);
        app.show_document(drawing(1));
        assert!(app.gis.attribute_values.is_none());
        assert!(app.dialog.is_none());
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.update(Message::Gis(crate::gis::Event::AttributeValues(
            crate::attribute_values::Event::Place,
        )));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
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

    /// UR2-059: closing a tab behind leaves the front drawing's command as it is: a clean one goes at once; one with
    /// unsaved work is not brought in front while a command runs here, the command and its corners kept.
    #[test]
    fn closing_a_tab_behind_keeps_the_front_command() {
        let mut app = app_with_drawing();
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(2))))));
        let front_session = front(&app);
        pending_hatch_value(&mut app);
        // The first drawing, behind, has nothing unsaved: it goes, the hatch runs on.
        let _ = app.update(Message::DocTabs(Event::Close(0)));
        assert_eq!(app.doc_tabs.count(true), 1);
        assert_eq!(front(&app), front_session);
        assert_eq!(app.session.tool_id(), "hatch", "the front command runs on");
        // A drawing behind with unsaved work: refused while the command runs.
        let mut app = app_with_drawing();
        let first = front(&app);
        app.document.as_mut().expect("open").model.set_name("değişti");
        assert!(app.document.as_ref().expect("open").dirty(), "unsaved work");
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(2))))));
        pending_hatch_value(&mut app);
        let _ = app.update(Message::DocTabs(Event::Close(0)));
        assert_eq!(app.doc_tabs.count(true), 2, "kept");
        assert_ne!(front(&app), first, "not brought in front");
        assert_eq!(app.session.tool_id(), "hatch");
        assert!(crate::files_testing::last_said(&app).contains("kaydedilmemiş değişiklik"));
    }

    /// UR2-060: Nokta's name series, code and height are the drawing's: another drawing opened starts with none, so
    /// its points get no other project's measured height; back in the first, its series goes on.
    #[test]
    fn nokta_memory_stays_with_its_drawing() {
        use kentos_interaction::Name;
        let mut app = app_with_drawing();
        app.memory.point_name = Name::new("BATI-711").expect("short");
        app.memory.point_code = Name::new("BATI-KANAL").expect("short");
        app.memory.point_z = Some(173.625);
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(2))))));
        assert_eq!((app.memory.point_name, app.memory.point_code, app.memory.point_z), (Name::EMPTY, Name::EMPTY, None));
        app.memory.point_name = Name::new("B-102").expect("short");
        let _ = app.update(Message::DocTabs(Event::Show(0)));
        assert_eq!(app.memory.point_name, Name::new("BATI-711").expect("short"));
        assert_eq!(app.memory.point_code, Name::new("BATI-KANAL").expect("short"));
        assert_eq!(app.memory.point_z, Some(173.625));
        let _ = app.update(Message::DocTabs(Event::Show(1)));
        assert_eq!((app.memory.point_name, app.memory.point_z), (Name::new("B-102").expect("short"), None));
    }

    /// UR2-056: Farklı kaydet onto the file of a drawing open in another tab (by its own path or a link to it) is
    /// refused and said with its name and file; the file stays as it was. Another folder's file of the same name is
    /// not it.
    #[test]
    fn save_as_over_another_tabs_file_is_refused() {
        use crate::saving::Target;
        let dir = std::env::temp_dir().join(format!("kentos-ur2-056-{}", std::process::id()));
        let (a, b) = (dir.join("a"), dir.join("b"));
        std::fs::create_dir_all(&a).expect("a folder");
        std::fs::create_dir_all(&b).expect("a folder");
        let theirs = b.join("Parsel.kcad");
        std::fs::write(&theirs, "B'nin dosyası").expect("written");
        let link = dir.join("bag.kcad");
        let _ = std::fs::remove_file(&link);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&theirs, &link).expect("a link");
        let mut app = app_with_drawing();
        app.document.as_mut().expect("a drawing").path = Some(a.join("Parsel.kcad"));
        assert!(app.park_active());
        let mut other = drawing(2);
        other.path = Some(theirs.clone());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(other)))));
        let _ = app.update(Message::DocTabs(Event::Show(0)));
        for over in [theirs.clone(), link.clone()] {
            let _ = app.start_save(Target::File(over.clone()));
            assert!(app.saving.is_none(), "nothing is written over {over:?}");
            let said = crate::files_testing::last_said(&app);
            assert!(said.contains("başka bir sekmede açık") && said.contains("Parsel.kcad"), "{said}");
            let _ = app.start_save(Target::Copy(over));
            assert!(app.saving.is_none());
        }
        assert_eq!(std::fs::read_to_string(&theirs).expect("kept"), "B'nin dosyası");
        // The same name in this drawing's own folder is another file: written.
        let _ = app.start_save(Target::File(a.join("Parsel 2.kcad")));
        assert!(app.saving.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// UR2-057, UR2-055: a tab brought back keeps its middle and scale, but takes the size the drawing area has now:
    /// the strip that came with the second drawing (14 px) or the split view that went changed it, and a camera of
    /// the old size drew the picture away from where the pointer reads it.
    #[test]
    fn a_tab_brought_back_takes_the_areas_size_now() {
        use crate::viewport::Event as View;
        use iced::{Point, Rectangle, Size};
        let sized = |app: &mut App, y: f32, w: f32, h: f32| {
            let _ = app.update(Message::Viewport(View::Resized(Rectangle::new(Point::new(0.0, y), Size::new(w, h)))));
        };
        // One drawing, then a second in a tab: the strip takes 14 px off the area.
        let mut app = app_with_drawing();
        sized(&mut app, 0.0, 1000.0, 800.0);
        let (center, scale) = (app.viewport.camera.center, app.viewport.camera.scale);
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(2))))));
        sized(&mut app, 14.0, 1000.0, 786.0);
        let _ = app.update(Message::DocTabs(Event::Show(0)));
        let c = app.viewport.camera;
        assert_eq!((c.width, c.height), (1000.0, 786.0), "the area's size now");
        assert_eq!((c.center, c.scale), (center, scale), "its own middle and scale");
        // Left in a split view (the active view half the width), brought back after the split went.
        let mut app = app_with_drawing();
        sized(&mut app, 14.0, 500.0, 786.0);
        let _ = app.update(Message::Run("view.split"));
        assert!(app.split.is_some());
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(2))))));
        sized(&mut app, 14.0, 1000.0, 786.0);
        let _ = app.update(Message::DocTabs(Event::Show(0)));
        assert!(app.split.is_none(), "the split stays with the drawing it was made on");
        let c = app.viewport.camera;
        assert_eq!((c.width, c.height), (1000.0, 786.0));
    }

    /// UR2-046: a drawing opened after one turned by Görünümü döndür opens north up; the turned one, brought in front
    /// again, keeps its own turn, and the drawing opened stays north up.
    #[test]
    fn a_drawing_opens_north_up_and_a_tab_keeps_its_own_turn() {
        let mut app = app_with_drawing();
        let turn = std::f64::consts::FRAC_PI_4; // 50 gon
        app.viewport.camera.twist = turn;
        assert!(app.park_active());
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(2))))));
        assert_eq!(app.viewport.camera.twist, 0.0, "the drawing opened is north up");
        let _ = app.update(Message::DocTabs(Event::Show(0)));
        assert_eq!(app.viewport.camera.twist, turn, "the first drawing keeps its turn");
        let _ = app.update(Message::DocTabs(Event::Show(1)));
        assert_eq!(app.viewport.camera.twist, 0.0, "the second stays north up");
        // Opened in the tab's place, without setting it aside: north up too.
        app.viewport.camera.twist = turn;
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(3))))));
        assert_eq!(app.viewport.camera.twist, 0.0);
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

#[cfg(test)]
mod same_file_tests {
    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, drive, last_said, scratch};

    /// UR-088: a file already open is not opened in a second tab (its tab comes in front), and a save over a file
    /// another program changed since it was read is refused with the way on, the other work kept.
    #[test]
    fn one_file_is_one_tab_and_a_changed_file_is_not_written_over() {
        let dir = scratch("same-file");
        let path = dir.join("ayni.kcad");
        let mut app: App = app_with_drawing();
        app.picker = crate::app::Picker::File(path.clone());
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        assert!(last_said(&app).starts_with("Kaydedildi:"), "{}", last_said(&app));
        let tabs = app.doc_tabs.count(true);
        let _ = app.update(Message::Opening(crate::opening::Event::PickedTab(Some(path.clone()))));
        assert_eq!(app.doc_tabs.count(true), tabs, "no second tab of the file");
        assert!(last_said(&app).contains("zaten açık"), "{}", last_said(&app));
        // Another program writes the file meanwhile.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let other = std::fs::read(&path).unwrap();
        std::fs::write(&path, [other.as_slice(), b" "].concat()).unwrap();
        let before = std::fs::read(&path).unwrap();
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        assert!(last_said(&app).contains("diskte değişti"), "{}", last_said(&app));
        assert_eq!(std::fs::read(&path).unwrap(), before, "the other write is kept");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
