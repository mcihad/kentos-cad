//! A data layer's edit session in the window (docs/adr/9103 §5, plan/veri.md VY-05): Katmanlar's Düzenlemeyi başlat and
//! Düzenlemeyi bitir on a data layer, Değişiklikleri kaydet and Değişiklikleri geri al on it or on every edited layer,
//! the question when a layer with changes stops being edited or the drawing is left (Kaydet, Kaydetme, Vazgeç; leaving, also
//! Projede tut: the changes wait with the project, docs/adr/9111 §3), and the
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
    /// A save that met rows others changed (docs/adr/9104 §3): its layers and the words; Üstüne yaz writes over them.
    pub(crate) conflict: Option<(Vec<String>, String)>,
    /// Projede tut answered for this leaving: the drawing is left without its changes being asked about again; they wait
    /// with the project (docs/adr/9111 §3).
    kept_for: Option<Then>,
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
    /// Projede tut (leaving only): the changes are not written; saved with the project, they come back when it is opened
    /// (docs/adr/9111 §3).
    Keep,
}

/// A group's save as its thread hands it back: the layers written and each one's new keys, or why nothing was.
pub type GroupSaved = Result<(Vec<LayerWrites>, Vec<Vec<i64>>), WriteFailure>;

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
    /// A group's save: its layers, what was written and each layer's new keys, or why nothing was.
    Saved(Vec<String>, Box<GroupSaved>),
    /// Üstüne yaz: the layers saved over the rows others changed.
    SaveOver(Vec<String>),
    /// The conflict's question closed without writing.
    KeepConflict,
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
            force: None,
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
        if self.gis.data_edits.kept_for.take() == Some(then) {
            return None;
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
                // The layers that stop with it (docs/adr/9104 §2), their changes asked about together.
                let group = self.document.as_ref().map_or_else(Vec::new, |d| {
                    data_write::grouped(&d.model, std::slice::from_ref(&id), d.model.settings().edit_grouping(), true)
                });
                if group.iter().any(|l| self.gis.data_edits.saving.contains(l)) {
                    self.warn("Katmanın değişiklikleri tabloya yazılıyor; yazma bitince yeniden deneyin.");
                } else {
                    let modified = self.data_modified_layers(&group);
                    if modified.is_empty() {
                        self.stop_data_layers(vec![id]);
                    } else {
                        self.ask_data_edits(modified, After::Stop);
                    }
                }
                Task::none()
            }
            Event::Save(layers) => self.save_data_layers(&layers, false),
            Event::SaveOver(layers) => {
                self.gis.data_edits.conflict = None;
                if matches!(self.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataConflict))) {
                    self.dialog = None;
                }
                self.save_data_layers(&layers, true)
            }
            Event::KeepConflict => {
                self.gis.data_edits.conflict = None;
                if matches!(self.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataConflict))) {
                    self.dialog = None;
                }
                Task::none()
            }
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
            Answer::Keep => {
                let After::Leave(then) = ask.after else {
                    return Task::none();
                };
                self.gis.data_edits.kept_for = Some(then);
                self.output(
                    "Katman değişiklikleri tabloya yazılmadı; proje kaydedilince bu cihazda projeyle saklanır, proje açılınca geri gelir.",
                );
                self.leave(then)
            }
            Answer::Save => {
                self.gis.data_edits.after = Some((ask.after, ask.layers.clone()));
                let task = self.save_data_layers(&ask.layers, false);
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

    /// Writes the layers' changes to their tables, each group on a thread of its own: each layer alone, or a source's
    /// layers together in one transaction, as the project groups them (docs/adr/9104 §2). `force` writes over the rows
    /// others changed since they were taken.
    fn save_data_layers(&mut self, layers: &[String], force: bool) -> Task<Message> {
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
        let groups = data_write::groups_of(&doc.model, &modified, settings.edit_grouping());
        let mut tasks = Vec::new();
        for group in groups {
            let ids: Vec<String> = group.iter().map(|w| w.layer.clone()).collect();
            if ids.iter().any(|l| self.gis.data_edits.saving.contains(l)) {
                continue;
            }
            self.gis.data_edits.saving.extend(ids.iter().cloned());
            let settings = settings.clone();
            tasks.push(crate::db_connections::off_thread(
                move || data_write::write_group(&group, &settings, force).map(|keys| (group, keys)),
                move |outcome| message(Event::Saved(ids, Box::new(outcome))),
            ));
        }
        Task::batch(tasks)
    }

    /// A group's save: what was written taken out of the drawing, or why nothing was; a conflict asks to write over it.
    fn data_saved(
        &mut self,
        ids: Vec<String>,
        outcome: GroupSaved,
    ) -> Task<Message> {
        for id in &ids {
            self.gis.data_edits.saving.remove(id);
        }
        match outcome {
            Ok((group, keys)) => {
                let mut said = Vec::new();
                for (w, keys) in group.iter().zip(keys) {
                    let written = data_write::written(w, &keys);
                    let words = counts_words(written.added.len(), written.changed.len(), written.deleted.len());
                    let taken = self
                        .document
                        .as_mut()
                        .map(|d| d.model.data_saved(&w.layer, written))
                        .unwrap_or(Ok(()));
                    match taken {
                        Ok(()) => said.push(format!("“{}”: {words}", w.name)),
                        Err(why) => self.warn(why),
                    }
                }
                if !said.is_empty() {
                    let names: Vec<String> = group.iter().map(|w| format!("“{}”", w.name)).collect();
                    let title = format!("{} kaydedildi", names.join(", "));
                    let body = format!("{} tabloya yazıldı.", said.join("; "));
                    self.say(kentos_interaction::Level::Success, format!("{title}: {body}"));
                    self.toast_saved(title, body);
                }
            }
            Err(failure) => {
                self.gis.data_edits.after = None;
                match &failure {
                    WriteFailure::Conflict(words, uids) => {
                        self.select_data_objects(uids);
                        self.gis.data_edits.conflict = Some((ids.clone(), words.clone()));
                        self.dialog = Some(Dialog::Gis(crate::gis::GisDialogKind::DataConflict));
                    }
                    WriteFailure::Refused(_, Some(uid)) => {
                        self.warn(failure.message().to_owned());
                        self.select_data_objects(&[*uid]);
                    }
                    _ => self.warn(failure.message().to_owned()),
                }
            }
        }
        if self.gis.data_edits.saving.is_empty() {
            return self.data_edits_then();
        }
        Task::none()
    }

    /// The conflict's question: Üstüne yaz, Vazgeç.
    pub(crate) fn data_conflict_question(&self) -> Element<'_, Message> {
        let (layers, words) = self.gis.data_edits.conflict.clone().unwrap_or_default();
        kentos_ui::widget::overlay::modal(
            kentos_ui::widget::Confirm::new(
                "Satırlar başkasınca değiştirilmiş",
                message(Event::SaveOver(layers)),
                message(Event::KeepConflict),
            )
            .message(words)
            .detail(
                "Üstüne yaz yalnız sizin değiştirdiğiniz alanları ve şekli yazar: aynısını başkası da değiştirdiyse \
                 onunki gider, öbür değişiklikleri kalır. Değişikliğinizi bırakmak için Vazgeç'i, sonra katmanın \
                 menüsünde Değişiklikleri geri al'ı seçin: satırlar tablodaki şimdiki hâliyle görünür.",
            )
            .confirm("Üstüne yaz")
            .destructive(),
            message(Event::KeepConflict),
        )
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
        // Leaving a project kept on this computer, the changes may wait with it (docs/adr/9111 §3).
        let can_keep = matches!(ask.after, After::Leave(_))
            && self.document.as_ref().is_some_and(|d| d.cloud_source().is_none());
        let keeps = if can_keep {
            " Projede tut, tabloya yazmadan projeyle saklar: proje açılınca geri gelir."
        } else {
            ""
        };
        let question =
            kentos_ui::widget::Confirm::new(title, message(Event::Answer(Answer::Discard)), message(Event::Answer(Answer::Cancel)))
                .message(format!("{whose} değişiklikler tablosuna kaydedilsin mi?"))
                .detail(format!(
                    "{} Kaydetmezseniz bu değişiklikler bırakılır; tablo olduğu gibi kalır.{keeps}",
                    detail.join(" ")
                ))
                .confirm("Kaydetme")
                .destructive()
                .keep("Kaydet", message(Event::Answer(Answer::Save)));
        let question = if can_keep { question.also("Projede tut", message(Event::Answer(Answer::Keep))) } else { question };
        kentos_ui::widget::overlay::modal(question, message(Event::Answer(Answer::Cancel)))
    }
}
