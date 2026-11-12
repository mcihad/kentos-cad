//! Açılışta sağlık denetimi (docs/adr/1009; plan/cad-advanced.md CAD-ADV-18): with `appearance.openHealthCheck` on,
//! a CAD drawing just opened is checked for the health group's candidates (docs/adr/1005) off the UI thread, read only,
//! for at most `BUDGET` (and the inspection's own work limit). What it found goes to CAD denetimi as that panel's
//! result, and one line to Komut geçmişi says how many and whether it was cut. The same file at the same size,
//! modification time and revision opened again in this session takes the last answer instead of checking again.
//! Opening another drawing stops a check still running; its late answer is dropped.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use iced::Task;
use kentos_commands::{CommandResult, DrawingInspect, DrawingInspected, InspectionGroup};

use crate::app::{App, Message};
use crate::cad_inspection::Event;

/// The longest an opening's check runs.
pub(crate) const BUDGET: Duration = Duration::from_secs(2);
/// How many answers are kept.
const KEPT: usize = 16;

/// What makes an answer reusable: the file as it is on disk and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    path: PathBuf,
    len: u64,
    modified: Option<SystemTime>,
    revision: String,
}

fn key(path: &Path, revision: String) -> Option<Key> {
    let meta = std::fs::metadata(path).ok()?;
    Some(Key { path: path.to_owned(), len: meta.len(), modified: meta.modified().ok(), revision })
}

#[derive(Debug)]
pub(crate) struct Opening {
    token: u64,
    cancel: Arc<AtomicBool>,
    kept: HashMap<Key, DrawingInspected>,
}

impl Default for Opening {
    fn default() -> Self {
        Self { token: 0, cancel: Arc::new(AtomicBool::new(false)), kept: HashMap::new() }
    }
}

impl Drop for Opening {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// The check's input: the health group over the visible drawing, the panel's default gap and limit.
fn input(revision: String) -> DrawingInspect {
    DrawingInspect {
        uids: None,
        checks: vec![InspectionGroup::Health],
        max_gap: Some(0.01),
        limit: Some(200),
        max_angle: None,
        include_hidden: false,
        expected_revision: Some(revision),
    }
}

/// The line Komut geçmişi gets.
pub(crate) fn words(found: &DrawingInspected, again: bool) -> String {
    let n = found.findings.len();
    let mut line = if n == 0 {
        format!("Açılış sağlık denetimi: aday bulunmadı ({} nesne incelendi); çizim değiştirilmedi.", found.examined)
    } else {
        format!(
            "Açılış sağlık denetimi: {n} aday ({} nesne incelendi); çizim değiştirilmedi. Ayrıntı CAD denetimi sekmesinde.",
            found.examined
        )
    };
    if found.truncated {
        line.push_str(" Süre ya da iş sınırına ulaşıldı: sonuç kısmi; tam denetim için CAD denetimi'nde Denetle'ye basın.");
    }
    if again {
        line.push_str(" Dosya ve revizyon değişmediği için önceki sonuç kullanıldı.");
    }
    line
}

impl App {
    /// After a drawing opened: the check, when asked for and the drawing is a CAD one.
    pub(crate) fn open_health_check(&mut self) -> Task<Message> {
        let opening = &mut self.cad_inspection.opening;
        opening.cancel.store(true, Ordering::Relaxed);
        opening.token = opening.token.wrapping_add(1);
        if !self.settings.bool("appearance.openHealthCheck") || !self.cad_axes() {
            return Task::none();
        }
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let revision = doc.model.revision().to_string();
        let (session, generation) = (doc.session, doc.model.generation());
        let key = doc.path.as_deref().and_then(|p| key(p, revision.clone()));
        if let Some(found) = key.as_ref().and_then(|k| self.cad_inspection.opening.kept.get(k)).cloned() {
            self.output(words(&found, true));
            self.cad_inspection.adopt(session, generation, found);
            return Task::none();
        }
        let model = doc.model.clone();
        let input = input(revision);
        let opening = &mut self.cad_inspection.opening;
        opening.cancel = Arc::new(AtomicBool::new(false));
        let (token, cancel) = (opening.token, Arc::clone(&opening.cancel));
        crate::db_connections::off_thread(
            move || {
                let deadline = Instant::now() + BUDGET;
                kentos_native_application::drawing_inspect::inspect_until(
                    &model,
                    &input,
                    || cancel.load(Ordering::Relaxed),
                    Some(deadline),
                )
            },
            move |answer| Message::CadInspection(Event::Opened(token, session, generation, key, Box::new(answer))),
        )
    }

    /// The check's answer: dropped when another drawing opened or this one changed meanwhile.
    pub(crate) fn open_health_ready(
        &mut self,
        token: u64,
        session: u64,
        generation: u64,
        key: Option<Key>,
        answer: CommandResult<DrawingInspected>,
    ) -> Task<Message> {
        let current = self
            .document
            .as_ref()
            .is_some_and(|d| d.session == session && d.model.generation() == generation);
        if token != self.cad_inspection.opening.token || !current {
            return Task::none();
        }
        match answer {
            CommandResult::Completed { output, .. } => {
                self.output(words(&output, false));
                if let Some(key) = key {
                    let kept = &mut self.cad_inspection.opening.kept;
                    if kept.len() >= KEPT {
                        kept.clear();
                    }
                    kept.insert(key, output.clone());
                }
                self.cad_inspection.adopt(session, generation, output);
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                self.output(format!("Açılış sağlık denetimi yapılamadı: {}", error.message));
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::{drive, scratch};
    use crate::tools_scenes::Objects;
    use serde_json::{Value, json};

    const EMPTY: &str = include_str!("../../../../fixtures/interaction/v1/empty.kcad");

    /// A CAD drawing at `path`: a line whose end stops 5 mm short of another's inside when `short`, else two lines
    /// that do not come near each other.
    fn drawing(path: &Path, short: bool) -> crate::document::Document {
        drawing_written(path, short, true)
    }

    /// [`drawing`], the file on disk written only when `write`.
    fn drawing_written(path: &Path, short: bool, write: bool) -> crate::document::Document {
        let mut o = Objects::new();
        o.line("cizim", [0.0, 0.0], [10.0, 0.0]);
        if short {
            o.line("cizim", [5.0, 0.005], [5.0, 5.0]);
        } else {
            o.line("cizim", [0.0, 20.0], [10.0, 20.0]);
        }
        let mut d: Value = serde_json::from_str(EMPTY).expect("the empty drawing reads");
        d["layers"] = json!([crate::tools_scenes::layer("cizim", "Çizim", "fg", 0.25)]);
        d["activeLayer"] = json!("cizim");
        d["entities"] = Value::Array(o.0);
        d["settings"]["workspace"] = json!("cad");
        if write {
            std::fs::write(path, d.to_string()).expect("the file");
        }
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&d.to_string()).expect("reads");
        crate::document::Document::new(snapshot, Some(path.to_owned())).expect("opens")
    }

    fn app(on: bool) -> App {
        let (mut app, _) = App::boot(None);
        assert!(app.settings.choose(&[("appearance.openHealthCheck", Value::Bool(on))]).is_empty());
        app
    }

    fn said(app: &App) -> Vec<String> {
        app.log.lines().map(|l| l.text.clone()).filter(|t| t.starts_with("Açılış sağlık")).collect()
    }

    #[test]
    fn an_opened_drawing_is_checked_once_and_the_same_file_takes_the_kept_answer() {
        let dir = scratch("acilis-saglik");
        let path = dir.join("t.kcad");
        let mut app = app(true);
        let task = app.update(Message::Opened(Some(Ok(Box::new(drawing(&path, true))))));
        let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
        drive(&mut app, task);
        let found = app.cad_inspection.result().expect("the panel has the answer");
        assert_eq!(found.findings.len(), 1, "{:?}", found.findings);
        assert_eq!(found.findings[0].code, "t_junction_short");
        assert!(!found.truncated);
        assert_eq!(said(&app), ["Açılış sağlık denetimi: 1 aday (2 nesne incelendi); çizim değiştirilmedi. Ayrıntı CAD denetimi sekmesinde."]);
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.model.revision().to_string(), before, "read only");
        assert!(!doc.model.can_undo(), "nothing to undo");
        // The same file again, untouched: the kept answer, no check.
        let task = app.update(Message::Opened(Some(Ok(Box::new(drawing_written(&path, true, false))))));
        drive(&mut app, task);
        // One line only, the kept answer's: a check that ran would have said a third.
        assert_eq!(said(&app).len(), 2, "{:?}", said(&app));
        assert_eq!(app.cad_inspection.result().map(|r| r.findings.len()), Some(1));
        assert!(said(&app)[1].ends_with("Dosya ve revizyon değişmediği için önceki sonuç kullanıldı."), "{:?}", said(&app));
        // The file changed on disk: checked again.
        std::thread::sleep(Duration::from_millis(20));
        let changed = drawing(&path, false);
        let task = app.update(Message::Opened(Some(Ok(Box::new(changed)))));
        drive(&mut app, task);
        assert_eq!(said(&app)[2], "Açılış sağlık denetimi: aday bulunmadı (2 nesne incelendi); çizim değiştirilmedi.");
    }

    #[test]
    fn off_by_default_and_when_off_nothing_runs() {
        assert!(!App::boot(None).0.settings.bool("appearance.openHealthCheck"));
        let dir = scratch("acilis-saglik-kapali");
        let mut app = app(false);
        let task = app.update(Message::Opened(Some(Ok(Box::new(drawing(&dir.join("t.kcad"), true))))));
        drive(&mut app, task);
        assert!(said(&app).is_empty());
        assert!(app.cad_inspection.result().is_none());
    }

    #[test]
    fn a_check_of_a_drawing_no_longer_open_is_dropped() {
        let dir = scratch("acilis-saglik-gec");
        let mut app = app(true);
        let first = app.update(Message::Opened(Some(Ok(Box::new(drawing(&dir.join("a.kcad"), true))))));
        let second = app.update(Message::Opened(Some(Ok(Box::new(drawing(&dir.join("b.kcad"), false))))));
        drive(&mut app, first);
        assert!(said(&app).is_empty(), "the first drawing's late answer: {:?}", said(&app));
        drive(&mut app, second);
        assert_eq!(said(&app).len(), 1);
        assert!(said(&app)[0].contains("aday bulunmadı"));
    }

    #[test]
    fn the_time_runs_out_into_a_partial_answer_said_so() {
        let dir = scratch("acilis-saglik-sure");
        let doc = drawing(&dir.join("t.kcad"), true);
        let answer = kentos_native_application::drawing_inspect::inspect_until(
            &doc.model,
            &input(doc.model.revision().to_string()),
            || false,
            Some(Instant::now()),
        );
        let CommandResult::Completed { output, warnings } = answer else { panic!("completes") };
        assert!(output.truncated);
        assert_eq!(output.examined, 0);
        assert_eq!(warnings[0].code, "inspection_truncated");
        assert!(words(&output, false).contains("Süre ya da iş sınırına ulaşıldı: sonuç kısmi"));
    }
}

/// The pictures of docs/adr/1009: a drawing opened with the check on (Komut geçmişi's line, CAD denetimi's candidates
/// on the drawing) and the switch in Ayarlar. `KENTOS_SHOTS_ONLY=acilis-saglik,acilis-saglik-ayar cargo test -p
/// kentos-desktop --lib tools_screens::screens -- --ignored --nocapture`.
#[cfg(test)]
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![("acilis-saglik", opened_scene)]
}

/// Ayarlar → Görünüş scrolled to its end, where Açılış is.
#[cfg(test)]
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![(
        "acilis-saglik-ayar",
        |app| app.open_settings_at(crate::settings_sections::Section::Appearance),
        |snapshot, app| {
            struct End;
            impl iced::advanced::widget::Operation for End {
                fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
                    operate(self);
                }
                fn scrollable(
                    &mut self,
                    _: Option<&iced::advanced::widget::Id>,
                    bounds: iced::Rectangle,
                    _: iced::Rectangle,
                    _: iced::Vector,
                    state: &mut dyn iced::advanced::widget::operation::Scrollable,
                ) {
                    // The dialog's page: right of its section list, tall.
                    if bounds.x > 400.0 && bounds.x < 900.0 && bounds.height > 300.0 {
                        state.snap_to(iced::widget::scrollable::RelativeOffset::END.into());
                    }
                }
            }
            snapshot.operate(app.view(), Box::new(End));
        },
    )]
}

#[cfg(test)]
fn opened_scene(app: &mut App) {
    use crate::tools_scenes::{E, N, Objects};
    use serde_json::{Value, json};
    assert!(app.settings.choose(&[("appearance.openHealthCheck", Value::Bool(true))]).is_empty());
    // A frame of four walls' lines, two inner lines stopping short of it and one passing over, a slightly turned edge.
    let mut o = Objects::new();
    o.line("cizim", [0.0, 0.0], [20.0, 0.0]);
    o.line("cizim", [20.0, 0.0], [20.0, 12.0]);
    o.line("cizim", [20.0, 12.0], [0.0, 12.0]);
    o.line("cizim", [0.0, 12.0], [0.0, 0.0]);
    o.line("cizim", [8.0, 0.006], [8.0, 12.0]);
    o.line("cizim", [8.0, 6.0], [20.008, 6.0]);
    o.line("cizim", [0.004, 9.0], [8.0, 9.0]);
    o.line("cizim", [2.0, 2.0], [6.0, 2.03]);
    let mut d: Value =
        serde_json::from_str(include_str!("../../../../fixtures/interaction/v1/empty.kcad")).expect("reads");
    d["layers"] = json!([crate::tools_scenes::layer("cizim", "Çizim", "fg", 0.35)]);
    d["activeLayer"] = json!("cizim");
    d["entities"] = Value::Array(o.0);
    d["settings"]["workspace"] = json!("cad");
    let dir = crate::files_testing::scratch("acilis-saglik-resim");
    let path = dir.join("kat-plani.kcad");
    std::fs::write(&path, d.to_string()).expect("the file");
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&d.to_string()).expect("reads");
    let doc = crate::document::Document::new(snapshot, Some(path)).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::drive(app, task);
    app.show_bottom(crate::bottom::BottomTab::CadInspection);
    let bar = f64::from(kentos_ui::widget::tabs::height());
    app.bottom_dragged(Some((crate::search::PANEL_HEIGHT - bar) as f32), std::time::Instant::now());
    let _ = app.cad_inspection_event(Event::Show(0));
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E - 2.0, min_y: N - 2.0, max_x: E + 22.0, max_y: N + 14.0 },
        24.0,
    );
}
