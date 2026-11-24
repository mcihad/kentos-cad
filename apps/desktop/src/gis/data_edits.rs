//! A data layer's edit session in the window (docs/adr/9103 §5, plan/veri.md VY-05): Katmanlar's Düzenlemeyi başlat and
//! Düzenlemeyi bitir on a data layer, Değişiklikleri kaydet and Değişiklikleri geri al on it or on every edited layer,
//! the question when a layer with changes stops being edited or the drawing is left (Kaynağa yaz, Değişiklikleri geri al,
//! Vazgeç; leaving, also Taslak olarak tut: the changes wait with the project on this device, docs/adr/9111 §3, 9123),
//! and the save off the interface's thread: the layer's changes read off the drawing (`data_write::writes_of`), written to its
//! table on a thread of its own in one transaction (`data_write::write`), taken out of the drawing once written
//! (`Document::data_saved`). A save the table refuses leaves the changes as they were and says why, selecting the
//! object at fault when one was. Opening, letting go and closing go through `cad.layers.session`.

use std::collections::{BTreeMap, HashSet};

use iced::{Element, Task};
use kentos_commands::{CommandResult, Entity, LayersSession, LayersSessionDone, SessionAction};
use serde::{Deserialize, Serialize};
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
    /// A save that met rows others changed (docs/adr/9104 §3): its layers, the words and each row's clash with what is
    /// chosen for it (docs/adr/9148, data_conflict.rs).
    pub(crate) conflict: Option<crate::data_conflict::Conflict>,
    /// The taken rows a save found deleted in their source, by layer (docs/adr/9152): the table lists them among its
    /// rows until the layer is saved, let go or the row's draft is let go.
    pub(crate) gone: BTreeMap<String, std::collections::BTreeSet<i64>>,
    /// Taslak olarak tut answered for this leaving: the drawing is left without its changes being asked about again; they wait
    /// with the project (docs/adr/9111 §3).
    kept_for: Option<Then>,
    /// Why the last write of a layer's changes to its source did not go, by layer, while they wait (docs/adr/9123):
    /// the status bar's cell and Katman değişiklikleri say it; a write that goes, or the changes let go, clear it.
    pub(crate) failed: BTreeMap<String, String>,
    /// A write whose commit's answer was lost (docs/adr/9126), by each layer it wrote: what it sent and its receipt.
    /// The next Kaynağa yaz of those layers asks the source first; one without a receipt is not sent again by itself.
    pub(crate) uncertain: BTreeMap<String, Uncertain>,
    /// When each data layer's source last answered (docs/adr/9163), milliseconds since 1970: a page of its table read,
    /// a save that went. It goes with the project's journal, so a project opened again offline says how old its check is.
    pub(crate) verified: BTreeMap<String, u64>,
}

/// Now, in milliseconds since 1970: a source's answer's time (docs/adr/9163).
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// A write whose commit's answer was lost (docs/adr/9126): the source may hold it all or none of it. Kept with the
/// project's journal on this device, so a project opened again still asks before it writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Uncertain {
    /// The receipt that went in with the rows; none when the source keeps no receipts and cannot be asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
    pub words: String,
    /// What each layer of the group sent.
    pub sent: Vec<Sent>,
}

/// What one layer of an uncertain write sent: its new rows by object id, its changed rows, its deleted keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sent {
    pub layer: String,
    pub table: String,
    #[serde(default)]
    pub added: Vec<(String, Entity)>,
    #[serde(default)]
    pub changed: Vec<(String, Entity)>,
    #[serde(default)]
    pub deleted: Vec<i64>,
}

impl Uncertain {
    fn of(group: &[LayerWrites], words: String, receipt: Option<String>) -> Self {
        let sent = group
            .iter()
            .map(|w| Sent {
                layer: w.layer.clone(),
                table: w.data.table.clone(),
                added: w.changes.added.iter().map(|(u, e)| (u.to_string(), (**e).clone())).collect(),
                changed: w.changes.changed.iter().map(|c| (c.uid.to_string(), (*c.now).clone())).collect(),
                deleted: w.changes.deleted.clone(),
            })
            .collect();
        Self { receipt, words, sent }
    }

    pub(crate) fn layers(&self) -> Vec<String> {
        self.sent.iter().map(|s| s.layer.clone()).collect()
    }

    /// What the write wrote, its receipt's keys given to its new rows: each layer's part, for the drawing to take out.
    fn written(&self, keys: &data_write::ReceiptKeys) -> Vec<(String, kentos_domain::DataWritten)> {
        let uid = |u: &str| kentos_domain::Uuid::parse_str(u).ok();
        self.sent
            .iter()
            .map(|s| {
                let given = keys.get(&s.table).cloned().unwrap_or_default();
                let added = s
                    .added
                    .iter()
                    .filter_map(|(u, e)| {
                        let u = uid(u)?;
                        let key = given.iter().find(|(g, _)| *g == u)?.1;
                        Some((u, std::sync::Arc::new(e.clone()), key))
                    })
                    .collect();
                let changed = s.changed.iter().filter_map(|(u, e)| Some((uid(u)?, std::sync::Arc::new(e.clone())))).collect();
                (s.layer.clone(), kentos_domain::DataWritten { added, changed, deleted: s.deleted.clone() })
            })
            .collect()
    }
}

impl DataEdits {
    /// Why the last write of `layer`'s changes did not go, while they wait.
    pub(crate) fn failed_of(&self, layer: &str) -> Option<&str> {
        self.failed.get(layer).map(String::as_str)
    }
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
    /// The ribbon's Değişiklikleri geri al (docs/adr/9301): the layers' changes let go, their sessions kept open. Asked
    /// first, since letting go is no undo step.
    Rollback,
}

/// The question's answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Save,
    Discard,
    Cancel,
    /// Taslak olarak tut (leaving only): the changes are not written; saved with the project, they come back when it is opened
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
    /// A group's commit whose answer was lost (docs/adr/9126): its layers and what it sent.
    Unsure(Vec<String>, Box<Uncertain>),
    /// The source asked by an uncertain write's receipt: its new rows' keys when it committed, none when it did not, or
    /// why it could not be asked.
    Asked(Vec<String>, Box<Uncertain>, Box<Result<Option<data_write::ReceiptKeys>, String>>),
    /// Yeniden gönder: an uncertain write without a receipt sent again, the owner having looked at the source.
    Resend(Vec<String>),
    /// The conflict's question closed without writing.
    KeepConflict,
    /// A clash's choice (docs/adr/9148): the row, its choice, the user's (true) or the table's.
    Choose(usize, usize, bool),
    /// Seçilenleri kaydet: the rows settled as chosen, then saved again.
    Settle,
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
            choices: None,
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
    pub(crate) fn data_layer_name(&self, id: &str) -> String {
        self.document
            .as_ref()
            .and_then(|d| d.model.layers().get(id).map(|n| n.name.clone()))
            .unwrap_or_else(|| id.to_owned())
    }

    /// The edited layers with changes among `layers` (every edited one when empty), in the tree's order.
    pub(crate) fn data_modified_layers(&self, layers: &[String]) -> Vec<String> {
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

    /// Değişiklikleri geri al on the ribbon (docs/adr/9301): every edited layer with changes, asked about once; nothing
    /// asked when none has any, a word while a save is under way.
    pub(crate) fn ask_data_rollback(&mut self) {
        if self.data_saving() {
            self.warn("Katman değişiklikleri tabloya yazılıyor; yazma bitince yeniden deneyin.");
            return;
        }
        let layers = self.data_modified_layers(&[]);
        if !layers.is_empty() {
            self.ask_data_edits(layers, After::Rollback);
        }
    }

    pub fn data_edits_event(&mut self, event: Event) -> Task<Message> {
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
            Event::Choose(row, at, mine) => {
                self.data_conflict_choose(row, at, mine);
                Task::none()
            }
            Event::Settle => {
                let Some(layers) = self.data_conflict_settle() else {
                    return Task::none();
                };
                if matches!(self.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataConflict))) {
                    self.dialog = None;
                }
                self.save_data_layers(&layers, false)
            }
            Event::Rollback(layers) => {
                let modified = self.data_modified_layers(&layers);
                if modified.is_empty() {
                    return Task::none();
                }
                // Each layer with what it lets go, said (docs/adr/9152): the whole group's rows, not one.
                let names: Vec<String> = modified
                    .iter()
                    .map(|l| {
                        let c = self.document.as_ref().map(|d| d.model.data_changes(l)).unwrap_or_default();
                        format!(
                            "{}: {}",
                            self.data_layer_name(l),
                            counts_words(c.added.len(), c.changed.len(), c.deleted.len())
                        )
                    })
                    .collect();
                match self.data_session(SessionAction::Rollback, modified.clone()) {
                    Ok(_) => {
                        for l in &modified {
                            self.gis.data_edits.failed.remove(l);
                            self.gis.data_edits.uncertain.remove(l);
                            self.gis.data_edits.gone.remove(l);
                        }
                        self.journal_uncertain();
                        self.output(format!(
                            "Katman değişiklikleri geri alındı ({}): bunlar bırakıldı, kaynak olduğu gibi kaldı.",
                            names.join("; ")
                        ))
                    }
                    Err(why) => self.warn(why),
                }
                Task::none()
            }
            Event::Answer(answer) => self.data_edits_answer(answer),
            Event::Saved(id, outcome) => self.data_saved(id, *outcome),
            Event::Unsure(ids, u) => self.data_unsure(ids, *u),
            Event::Asked(ids, u, asked) => self.data_receipt_asked(ids, *u, *asked),
            Event::Resend(layers) => {
                for l in &layers {
                    if let Some(u) = self.gis.data_edits.uncertain.remove(l) {
                        for other in u.layers() {
                            self.gis.data_edits.uncertain.remove(&other);
                        }
                    }
                    self.gis.data_edits.failed.remove(l);
                }
                self.journal_uncertain();
                self.save_data_layers(&layers, false)
            }
        }
    }

    /// Closes the layers' sessions (their changes saved or let go).
    fn stop_data_layers(&mut self, layers: Vec<String>) {
        let names: Vec<String> = layers.iter().map(|l| self.data_layer_name(l)).collect();
        for l in &layers {
            self.gis.data_edits.failed.remove(l);
            self.gis.data_edits.uncertain.remove(l);
        }
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
                    "Katman değişiklikleri kaynağa yazılmadı; proje kaydedilince bu cihazda taslak olarak saklanır, proje bu bilgisayarda açılınca geri gelir.",
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
                    After::Leave(_) | After::Rollback => SessionAction::Rollback,
                };
                if let Err(why) = self.data_session(action, ask.layers.clone()) {
                    self.warn(why);
                    return Task::none();
                }
                for l in &ask.layers {
                    self.gis.data_edits.failed.remove(l);
                    self.gis.data_edits.uncertain.remove(l);
                }
                self.output(format!("Katman değişiklikleri geri alındı ({}): kaynak olduğu gibi kaldı.", names.join(", ")));
                match ask.after {
                    After::Stop | After::Rollback => Task::none(),
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
            // Saved instead of let go: the sessions stay open.
            After::Rollback => Task::none(),
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
            // A write of these layers whose answer was lost (docs/adr/9126): the source is asked by its receipt first;
            // one without a receipt is not sent again until the owner says so.
            if let Some(u) = ids.iter().find_map(|l| self.gis.data_edits.uncertain.get(l)).cloned() {
                let Some(receipt) = u.receipt.clone() else {
                    let names: Vec<String> = u.layers().iter().map(|l| self.data_layer_name(l)).collect();
                    self.warn(format!(
                        "{} için önceki yazımın sonucu bilinmiyor ve bu kaynak işlem kaydı tutmuyor: kayıt kendiliğinden \
                         yeniden gönderilmedi, yeni satırlar ikinci kez eklenebilirdi. Kaynakta satırları denetleyin, sonra \
                         Katman değişiklikleri'nde Yeniden gönder'i ya da Değişiklikleri geri al'ı seçin.",
                        names.join(", ")
                    ));
                    let first = ids.first().cloned();
                    tasks.push(self.update(crate::data_pending::open_at_message(first)));
                    continue;
                };
                let Some(data) = group.first().map(|w| w.data.clone()) else {
                    continue;
                };
                self.gis.data_edits.saving.extend(ids.iter().cloned());
                tasks.push(crate::db_connections::off_thread(
                    move || data_write::receipt_keys(&data, &receipt),
                    move |asked| message(Event::Asked(ids, Box::new(u), Box::new(asked))),
                ));
                continue;
            }
            self.gis.data_edits.saving.extend(ids.iter().cloned());
            let settings = settings.clone();
            // The write's receipt, given before it starts (docs/adr/9126).
            let receipting = data_write::Receipting {
                id: kentos_domain::Uuid::now_v7().to_string(),
                by: kentos_native_application::project_store::user(),
            };
            tasks.push(crate::db_connections::off_thread(
                move || match data_write::write_group_receipted(&group, &settings, force, Some(&receipting)) {
                    Ok(keys) => Ok(Ok((group, keys))),
                    Err(WriteFailure::Uncertain(words, receipt)) => Err(Uncertain::of(&group, words, receipt)),
                    Err(failure) => Ok(Err(failure)),
                },
                move |done: Result<GroupSaved, Uncertain>| match done {
                    Ok(outcome) => message(Event::Saved(ids, Box::new(outcome))),
                    Err(u) => message(Event::Unsure(ids, Box::new(u))),
                },
            ));
        }
        Task::batch(tasks)
    }

    /// A commit whose answer was lost (docs/adr/9126): the changes wait as they were and the write is kept, with its
    /// receipt, until the source says whether it committed; neither success nor failure is said.
    fn data_unsure(&mut self, ids: Vec<String>, u: Uncertain) -> Task<Message> {
        for id in &ids {
            self.gis.data_edits.saving.remove(id);
            self.gis.data_edits.failed.insert(id.clone(), u.words.clone());
            self.gis.data_edits.uncertain.insert(id.clone(), u.clone());
        }
        self.gis.data_edits.after = None;
        let next = if u.receipt.is_some() {
            "Değişiklikler katmanda duruyor; bir dahaki Kaynağa yaz önce kaynaktaki işlem kaydına bu yazımı sorar, yazılmışsa \
             ikinci kez yazmaz."
        } else {
            "Değişiklikler katmanda duruyor. Bu kaynak işlem kaydı tutmadığı için kayıt kendiliğinden yeniden gönderilmez: \
             kaynakta satırları denetleyin; işlem kaydı tablosunu Katman değişiklikleri'nden oluşturabilirsiniz."
        };
        self.warn(format!("{} {next}", u.words));
        self.journal_uncertain();
        Task::none()
    }

    /// The source asked by an uncertain write's receipt (docs/adr/9126): committed, the write is taken out of the drawing
    /// with the keys it gave and the rest written; not committed, the changes are written now; not asked, they wait.
    fn data_receipt_asked(
        &mut self,
        ids: Vec<String>,
        u: Uncertain,
        asked: Result<Option<data_write::ReceiptKeys>, String>,
    ) -> Task<Message> {
        for id in &ids {
            self.gis.data_edits.saving.remove(id);
        }
        match asked {
            Err(why) => {
                self.warn(format!(
                    "Önceki yazımın sonucu kaynağa sorulamadı: {why} Değişiklikler duruyor; bağlantı dönünce Kaynağa yaz yeniden sorar."
                ));
                Task::none()
            }
            Ok(found) => {
                for l in u.layers() {
                    self.gis.data_edits.uncertain.remove(&l);
                    self.gis.data_edits.failed.remove(&l);
                }
                match found {
                    Some(keys) => {
                        let mut said = Vec::new();
                        for (layer, written) in u.written(&keys) {
                            let words = counts_words(written.added.len(), written.changed.len(), written.deleted.len());
                            let name = self.data_layer_name(&layer);
                            match self.document.as_mut().map(|d| d.model.data_saved(&layer, written)) {
                                Some(Err(why)) => self.warn(why),
                                _ => said.push(format!("“{name}”: {words}")),
                            }
                        }
                        self.say(
                            kentos_interaction::Level::Success,
                            format!(
                                "Önceki yazım kaynağa ulaşmıştı (işlem kaydında bulundu): {}. İkinci kez yazılmadı.",
                                said.join("; ")
                            ),
                        );
                    }
                    None => self.output(
                        "Önceki yazım kaynağa ulaşmamıştı (işlem kaydında yok): değişiklikler şimdi yazılıyor.",
                    ),
                }
                self.journal_uncertain();
                // What waits after it (changed since, or all of it when it had not committed) is written now.
                self.save_data_layers(&ids, false)
            }
        }
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
                        Ok(()) => {
                            self.gis.data_edits.failed.remove(&w.layer);
                            self.gis.data_edits.gone.remove(&w.layer);
                            // The source answered: checked now (docs/adr/9163).
                            self.gis.data_edits.verified.insert(w.layer.clone(), now_ms());
                            said.push(format!("“{}”: {words}", w.name));
                        }
                        Err(why) => self.warn(why),
                    }
                }
                if !said.is_empty() {
                    let names: Vec<String> = group.iter().map(|w| format!("“{}”", w.name)).collect();
                    let title = format!("{} kaydedildi", names.join(", "));
                    let body = format!("{} kaynağa yazıldı.", said.join("; "));
                    self.say(kentos_interaction::Level::Success, format!("{title}: {body}"));
                    self.toast_saved(title, body);
                }
            }
            Err(failure) => {
                self.gis.data_edits.after = None;
                // The changes wait as they were; the status bar and Katman değişiklikleri say why (docs/adr/9123).
                for id in &ids {
                    self.gis.data_edits.failed.insert(id.clone(), failure.message().to_owned());
                }
                match &failure {
                    WriteFailure::Conflict(_, uids, rows) => {
                        self.select_data_objects(uids);
                        // The rows the source no longer has stay listed in the table (docs/adr/9152).
                        for r in rows.iter().filter(|r| r.gone() && r.mine.is_some()) {
                            self.gis.data_edits.gone.entry(r.layer.clone()).or_default().insert(r.key);
                        }
                        self.gis.data_edits.conflict = Some(crate::data_conflict::Conflict::of(ids.clone(), rows.clone()));
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
        let Some((_, outlines)) = self.gis.data_edits.pending.as_ref() else {
            self.gpu_layers.edits.clear();
            return None;
        };
        crate::approved_layer::layer(Some(outlines), 0, &self.viewport.camera, color, &self.gpu_layers.edits)
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
            After::Leave(_) => "Kaynağa yazılmamış katman değişiklikleri",
            After::Rollback => "Değişiklikleri geri al",
        };
        // Letting go asks only that (docs/adr/9301): the changes are gone for good, the source stays as it was.
        if matches!(ask.after, After::Rollback) {
            let question = kentos_ui::widget::Confirm::new(
                title,
                message(Event::Answer(Answer::Discard)),
                message(Event::Answer(Answer::Cancel)),
            )
            .message(format!("{whose} değişiklikler bırakılsın mı?"))
            .detail(format!(
                "{} Bırakılan değişiklik geri alınamaz; kaynak olduğu gibi kalır, katmanlar düzenlemeye açık kalır.",
                detail.join(" ")
            ))
            .confirm("Değişiklikleri geri al")
            .destructive();
            return kentos_ui::widget::overlay::modal(question, message(Event::Answer(Answer::Cancel)));
        }
        // Leaving a project kept on this computer, the changes may wait with it (docs/adr/9111 §3); each answer says
        // what it does to the source and to this device (docs/adr/9123).
        let can_keep = matches!(ask.after, After::Leave(_))
            && self.document.as_ref().is_some_and(|d| d.cloud_source().is_none());
        let keeps = if can_keep {
            " Taslak olarak tut kaynağa yazmaz: proje kaydedilince değişiklikler bu bilgisayarda saklanır, proje burada \
             açılınca geri gelir; proje dosyasını başka bir bilgisayara kopyalamak onları taşımaz."
        } else {
            ""
        };
        let question =
            kentos_ui::widget::Confirm::new(title, message(Event::Answer(Answer::Discard)), message(Event::Answer(Answer::Cancel)))
                .message(format!("{whose} değişiklikler kaynağa yazılsın mı?"))
                .detail(format!(
                    "{} Kaynağa yaz satırları yazar; başka kullanıcılar ve programlar onları o zaman görür. Değişiklikleri \
                     geri al onları bırakır; kaynak olduğu gibi kalır.{keeps}",
                    detail.join(" ")
                ))
                .confirm("Değişiklikleri geri al")
                .destructive()
                .keep("Kaynağa yaz", message(Event::Answer(Answer::Save)));
        let question =
            if can_keep { question.also("Taslak olarak tut", message(Event::Answer(Answer::Keep))) } else { question };
        kentos_ui::widget::overlay::modal(question, message(Event::Answer(Answer::Cancel)))
    }
}
