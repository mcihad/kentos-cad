//! A data layer's edit session in the window (docs/adr/9103 §5, plan/veri.md VY-05): Katmanlar's Düzenlemeyi başlat and
//! Düzenlemeyi bitir on a data layer, Değişiklikleri kaydet and Değişiklikleri geri al on it or on every edited layer,
//! the question when a layer with changes stops being edited or the drawing is left (Kaydet, Kaydetme, Vazgeç), and the
//! save off the interface's thread: the layer's changes read off the drawing (`data_write::writes_of`), written to its
//! table on a thread of its own in one transaction (`data_write::write`), taken out of the drawing once written
//! (`Document::data_saved`). A save the table refuses leaves the changes as they were and says why, selecting the
//! object at fault when one was. Opening, letting go and closing go through `cad.layers.session`.

use std::collections::HashSet;

use iced::{Element, Task};
use kentos_commands::{CommandResult, LayersSession, LayersSessionDone, SessionAction};
use kentos_native_application::ExecutionContext;
use kentos_native_application::data_write::{self, LayerWrites, WriteFailure};
use kentos_native_application::layers_session;

use crate::app::{App, Dialog, Message, Then};

/// The window's side of the edit sessions.
#[derive(Debug, Default)]
pub(crate) struct DataEdits {
    /// The layers whose save is under way.
    saving: HashSet<String>,
    /// The question on screen.
    pub(crate) asking: Option<Ask>,
    /// What comes once the saves under way all went through: the answered question's, with its layers.
    after: Option<(After, Vec<String>)>,
    /// The unsaved rows' outlines and the drawing's generation they were drawn for.
    pending: Option<(u64, crate::approved_layer::Approved)>,
}

/// The question: these layers have changes; what comes after the answer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ask {
    pub layers: Vec<String>,
    pub after: After,
}

/// What a question was asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum After {
    /// The layers stop being edited.
    Stop,
    /// The drawing is left for this.
    Leave(Then),
}

/// The question's answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Save,
    Discard,
    Cancel,
}

/// What the session's controls and its saves say.
#[derive(Debug, Clone)]
pub enum Event {
    /// Düzenlemeyi başlat: the layer opened and made the active one.
    Start(String),
    /// Düzenlemeyi bitir: asked first when it has changes.
    Stop(String),
    /// Değişiklikleri kaydet: these layers, or every edited one when none is named.
    Save(Vec<String>),
    /// Değişiklikleri geri al: let go, the layers staying open; every edited one when none is named.
    Rollback(Vec<String>),
    Answer(Answer),
    /// A save's outcome: what was written and the new rows' keys, or why nothing was.
    Saved(String, Box<Result<(LayerWrites, Vec<i64>), WriteFailure>>),
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::DataEdits(e))
}

/// A Katmanlar menu item's message.
pub(crate) fn event(e: Event) -> Message {
    message(e)
}

/// “1 yeni, 2 değişen, 1 silinen satır”: what a layer writes, its zero counts left out.
pub(crate) fn counts_words(added: usize, changed: usize, deleted: usize) -> String {
    let parts: Vec<String> = [(added, "yeni"), (changed, "değişen"), (deleted, "silinen")]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| format!("{n} {w}"))
        .collect();
    if parts.is_empty() {
        "değişiklik yok".to_owned()
    } else {
        format!("{} satır", parts.join(", "))
    }
}

impl App {
    /// `cad.layers.session` on the open drawing; its refusal's sentence.
    fn data_session(&mut self, action: SessionAction, layers: Vec<String>) -> Result<LayersSessionDone, String> {
        let Some(doc) = self.document.as_mut() else {
            return Err("Açık çizim yok.".into());
        };
        let input = LayersSession {
            action,
            layers: (!layers.is_empty()).then_some(layers),
            keys: None,
        };
        match layers_session::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Düzenleme oturumu değişmedi.".into()),
        }
    }

    /// A layer's name, for the words.
    fn data_layer_name(&self, id: &str) -> String {
        self.document
            .as_ref()
            .and_then(|d| d.model.layers().get(id).map(|n| n.name.clone()))
            .unwrap_or_else(|| id.to_owned())
    }

    /// The edited layers with changes among `layers` (every edited one when empty), in the tree's order.
    fn data_modified_layers(&self, layers: &[String]) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        doc.model
            .edited_layers()
            .into_iter()
            .filter(|l| layers.is_empty() || layers.contains(l))
            .filter(|l| doc.model.data_modified(l))
            .collect()
    }

    /// Whether a save of a data layer is under way.
    pub(crate) fn data_saving(&self) -> bool {
        !self.gis.data_edits.saving.is_empty()
    }

    /// Before the drawing is left for `then`: the question when an edited layer has changes, a word while a save is under
    /// way; none when nothing waits.
    pub(crate) fn data_edits_before_leave(&mut self, then: Then) -> Option<Task<Message>> {
        if self.data_saving() {
            self.warn("Katman değişiklikleri tabloya yazılıyor; yazma bitince yeniden deneyin.");
            return Some(Task::none());
        }
        let layers = self.data_modified_layers(&[]);
        if layers.is_empty() {
            return None;
        }
        self.ask_data_edits(layers, After::Leave(then));
        Some(Task::none())
    }

    fn ask_data_edits(&mut self, layers: Vec<String>, after: After) {
        self.gis.data_edits.asking = Some(Ask { layers, after });
        self.dialog = Some(Dialog::Gis(crate::gis::GisDialogKind::DataEdits));
    }

    pub(crate) fn data_edits_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Start(id) => {
                match self.data_session(SessionAction::Start, vec![id.clone()]) {
                    Ok(_) => {
                        if let Some(doc) = self.document.as_mut() {
                            doc.model.set_active_layer(&id);
                        }
                        let name = self.data_layer_name(&id);
                        self.output(format!(
                            "“{name}” düzenlemeye açıldı ve etkin katman yapıldı. Çizilen nesneler ve değiştirilen satırlar Kaydet'e kadar katmanda bekler."
                        ));
                    }
                    Err(why) => self.warn(why),
                }
                Task::none()
            }
            Event::Stop(id) => {
                if self.gis.data_edits.saving.contains(&id) {
                    self.warn("Katmanın değişiklikleri tabloya yazılıyor; yazma bitince yeniden deneyin.");
                } else if self.data_modified_layers(std::slice::from_ref(&id)).is_empty() {
                    self.stop_data_layers(vec![id]);
                } else {
                    self.ask_data_edits(vec![id], After::Stop);
                }
                Task::none()
            }
            Event::Save(layers) => self.save_data_layers(&layers),
            Event::Rollback(layers) => {
                let modified = self.data_modified_layers(&layers);
                if modified.is_empty() {
                    return Task::none();
                }
                let names: Vec<String> = modified.iter().map(|l| self.data_layer_name(l)).collect();
                match self.data_session(SessionAction::Rollback, modified) {
                    Ok(_) => self.output(format!(
                        "Katman değişiklikleri geri alındı ({}): tablo olduğu gibi kaldı.",
                        names.join(", ")
                    )),
                    Err(why) => self.warn(why),
                }
                Task::none()
            }
            Event::Answer(answer) => self.data_edits_answer(answer),
            Event::Saved(id, outcome) => self.data_saved(id, *outcome),
        }
    }

    /// Closes the layers' sessions (their changes saved or let go).
    fn stop_data_layers(&mut self, layers: Vec<String>) {
        let names: Vec<String> = layers.iter().map(|l| self.data_layer_name(l)).collect();
        match self.data_session(SessionAction::Stop, layers) {
            Ok(_) => self.output(format!("Düzenleme bitti: {}.", names.join(", "))),
            Err(why) => self.warn(why),
        }
    }

    fn data_edits_answer(&mut self, answer: Answer) -> Task<Message> {
        let Some(ask) = self.gis.data_edits.asking.take() else {
            return Task::none();
        };
        if matches!(self.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataEdits))) {
            self.dialog = None;
        }
        match answer {
            Answer::Cancel => {
                self.gis.data_edits.after = None;
                Task::none()
            }
            Answer::Save => {
                self.gis.data_edits.after = Some((ask.after, ask.layers.clone()));
                let task = self.save_data_layers(&ask.layers);
                // Nothing went to be written: what comes after waits for no save.
                if self.gis.data_edits.saving.is_empty() {
                    return Task::batch([task, self.data_edits_then()]);
                }
                task
            }
            Answer::Discard => {
                let names: Vec<String> = ask.layers.iter().map(|l| self.data_layer_name(l)).collect();
                let action = match ask.after {
                    After::Stop => SessionAction::Cancel,
                    After::Leave(_) => SessionAction::Rollback,
                };
                if let Err(why) = self.data_session(action, ask.layers.clone()) {
                    self.warn(why);
                    return Task::none();
                }
                self.output(format!("Katman değişiklikleri kaydedilmedi ({}).", names.join(", ")));
                match ask.after {
                    After::Stop => Task::none(),
                    After::Leave(then) => self.leave(then),
                }
            }
        }
    }

    /// What the answered question was for, once its saves went through.
    fn data_edits_then(&mut self) -> Task<Message> {
        let Some((after, layers)) = self.gis.data_edits.after.take() else {
            return Task::none();
        };
        // A layer still with changes (its save failed) keeps its session and the drawing.
        if !self.data_modified_layers(&layers).is_empty() {
            return Task::none();
        }
        match after {
            After::Stop => {
                self.stop_data_layers(layers);
                Task::none()
            }
            After::Leave(then) => self.leave(then),
        }
    }

    /// Writes the layers' changes to their tables, each on a thread of its own.
    fn save_data_layers(&mut self, layers: &[String]) -> Task<Message> {
        let modified = self.data_modified_layers(layers);
        let Some(doc) = &self.document else {
            return Task::none();
        };
        if modified.is_empty() {
            if !layers.is_empty() {
                self.output("Kaydedilecek katman değişikliği yok.");
            }
            return Task::none();
        }
        let settings = doc.model.settings().clone();
        let mut tasks = Vec::new();
        for id in modified {
            if self.gis.data_edits.saving.contains(&id) {
                continue;
            }
            let Some(writes) = self.document.as_ref().and_then(|d| data_write::writes_of(&d.model, &id)) else {
                continue;
            };
            self.gis.data_edits.saving.insert(id.clone());
            let settings = settings.clone();
            let layer = id.clone();
            tasks.push(crate::db_connections::off_thread(
                move || data_write::write(&writes, &settings).map(|keys| (writes, keys)),
                move |outcome| message(Event::Saved(layer, Box::new(outcome))),
            ));
        }
        Task::batch(tasks)
    }

    /// A save's outcome: what was written taken out of the drawing, or why nothing was.
    fn data_saved(&mut self, id: String, outcome: Result<(LayerWrites, Vec<i64>), WriteFailure>) -> Task<Message> {
        self.gis.data_edits.saving.remove(&id);
        let name = self.data_layer_name(&id);
        match outcome {
            Ok((writes, keys)) => {
                let written = data_write::written(&writes, &keys);
                let words = counts_words(written.added.len(), written.changed.len(), written.deleted.len());
                let taken = self
                    .document
                    .as_mut()
                    .map(|d| d.model.data_saved(&id, written))
                    .unwrap_or(Ok(()));
                match taken {
                    Ok(()) => {
                        self.say(
                            kentos_interaction::Level::Success,
                            format!("“{name}” kaydedildi: {words} tabloya yazıldı."),
                        );
                        self.toast_saved(format!("“{name}” kaydedildi"), format!("{words} tabloya yazıldı."));
                    }
                    Err(why) => self.warn(why),
                }
            }
            Err(failure) => {
                self.gis.data_edits.after = None;
                self.warn(failure.message().to_owned());
                if let WriteFailure::Refused(_, Some(uid)) = &failure {
                    self.select_data_objects(&[*uid]);
                }
            }
        }
        if self.gis.data_edits.saving.is_empty() {
            return self.data_edits_then();
        }
        Task::none()
    }

    /// Selects objects by their persistent ids (the object a refused save was about).
    fn select_data_objects(&mut self, uids: &[kentos_domain::Uuid]) {
        let Some(doc) = &self.document else {
            return;
        };
        let slots: Vec<kentos_domain::Slot> = uids.iter().filter_map(|u| doc.model.slot_of(*u)).collect();
        if !slots.is_empty() {
            self.selection.set(slots);
        }
    }

    /// After every message: the outlines of the edited layers' rows not saved yet, when the drawing changed.
    pub(crate) fn data_edits_tasks(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            self.gis.data_edits.pending = None;
            return;
        };
        let model = &doc.model;
        let generation = model.generation();
        if self.gis.data_edits.pending.as_ref().is_some_and(|(g, _)| *g == generation) {
            return;
        }
        let mut objects = Vec::new();
        for layer in model.edited_layers().iter().filter(|l| model.layers().is_visible(l)) {
            let c = model.data_changes(layer);
            objects.extend(c.added.iter().map(|(_, e)| (**e).clone()));
            objects.extend(c.changed.iter().map(|d| (*d.now).clone()));
        }
        self.gis.data_edits.pending = Some((generation, crate::approved_layer::Approved::of(&objects, 0)));
    }

    /// The edited layers' rows not saved yet, dashed in the warning colour over the drawing: a new row whose style draws
    /// nothing (a class it has no value for yet) still shows.
    pub(crate) fn data_edits_layer(&self, color: iced::Color) -> Option<Element<'_, Message>> {
        let (_, outlines) = self.gis.data_edits.pending.as_ref()?;
        crate::approved_layer::layer(Some(outlines), 0, &self.viewport.camera, color)
    }

    /// The question: Kaydet, Kaydetme, Vazgeç.
    pub(crate) fn data_edits_question(&self) -> Element<'_, Message> {
        let ask = self.gis.data_edits.asking.clone().unwrap_or(Ask {
            layers: Vec::new(),
            after: After::Stop,
        });
        let names: Vec<String> = ask.layers.iter().map(|l| format!("“{}”", self.data_layer_name(l))).collect();
        let whose = match names.as_slice() {
            [one] => format!("{one} katmanındaki"),
            [a, b] => format!("{a} ve {b} katmanlarındaki"),
            many => format!("{} katmandaki", many.len()),
        };
        let detail: Vec<String> = ask
            .layers
            .iter()
            .filter_map(|l| {
                let c = self.document.as_ref()?.model.data_changes(l);
                Some(format!(
                    "{}: {}.",
                    self.data_layer_name(l),
                    counts_words(c.added.len(), c.changed.len(), c.deleted.len())
                ))
            })
            .collect();
        let title = match ask.after {
            After::Stop => "Düzenlemeyi bitir",
            After::Leave(_) => "Kaydedilmemiş katman değişiklikleri",
        };
        kentos_ui::widget::overlay::modal(
            kentos_ui::widget::Confirm::new(title, message(Event::Answer(Answer::Discard)), message(Event::Answer(Answer::Cancel)))
                .message(format!("{whose} değişiklikler tablosuna kaydedilsin mi?"))
                .detail(format!(
                    "{} Kaydetmezseniz bu değişiklikler bırakılır; tablo olduğu gibi kalır.",
                    detail.join(" ")
                ))
                .confirm("Kaydetme")
                .destructive()
                .keep("Kaydet", message(Event::Answer(Answer::Save))),
            message(Event::Answer(Answer::Cancel)),
        )
    }
}
