//! The editor's events: typing, choosing, tabs and groups, workflow steps, the issues gone to, and Kaydet
//! on one record or many.

use super::*;
use crate::gis;

impl App {
    pub(super) fn entry_event(&mut self, preview: bool, event: Event) -> Task<Message> {
        let slot = if preview {
            &mut self.gis.form_preview
        } else {
            &mut self.gis.form_entry
        };
        let (Some(e), Some(doc)) = (slot.as_mut(), self.document.as_ref()) else {
            return Task::none();
        };
        match event {
            Event::Query(key, text) => {
                e.queries.insert(key.clone(), text);
                e.pages.remove(&key);
                // Typing pauses before the list is searched; a newer key press takes the turn.
                e.turn += 1;
                e.pending.insert(key.clone(), e.turn);
                let after = e.msg(Event::Search(key, e.turn));
                return crate::hover_card::after(SEARCH_PAUSE, after);
            }
            Event::Search(key, turn) => {
                if e.pending.get(&key) == Some(&turn) {
                    e.pending.remove(&key);
                    if let Some(text) = e.queries.get(&key) {
                        e.searched.insert(key, text.clone());
                    }
                }
                return Task::none();
            }
            Event::More(key) => {
                *e.pages.entry(key).or_insert(1) += 1;
                return Task::none();
            }
            Event::RelatedPage(item, page) => {
                e.related_pages.insert(item, page);
                return Task::none();
            }
            Event::Value(key, text) => {
                e.queries.remove(&key);
                e.searched.remove(&key);
                e.pending.remove(&key);
                e.pages.remove(&key);
                // A long text's editor shows a value given as a whole (a trace's, a pick's) too.
                if let Some(content) = e.notes.get_mut(&key)
                    && content.text().trim_end_matches('\n') != text
                {
                    *content = text_editor::Content::with_text(&text);
                }
                e.touched.insert(key.clone());
                if let Some(item) = chosen_item(&e.body.items, &e.fields, &key) {
                    e.left.insert(item);
                }
                // Typing over an overridable calculation keeps the value; emptying it follows again.
                if e.overridable(&key) {
                    if text.is_empty() {
                        e.follows.insert(key.clone());
                    } else {
                        e.follows.remove(&key);
                    }
                }
                // Many records: emptying a field they had no one value in leaves it as it was.
                let untouched = !e.members.is_empty()
                    && text.is_empty()
                    && e.name_of(&key).is_some_and(|n| !e.before.contains_key(n));
                if untouched {
                    e.values.remove(&key);
                } else {
                    e.values.insert(key, text);
                }
                e.recompute(&doc.model);
            }
            Event::ClearMisfits => {
                for key in e.misfits() {
                    e.touched.insert(key.clone());
                    e.values.insert(key, String::new());
                }
                e.recompute(&doc.model);
            }
            Event::Note(key, action) => {
                let edit = action.is_edit();
                if let Some(content) = e.notes.get_mut(&key) {
                    content.perform(action);
                    if edit {
                        let text = content.text();
                        let text = text.strip_suffix('\n').unwrap_or(&text).to_owned();
                        e.touched.insert(key.clone());
                        e.values.insert(key, text);
                        e.recompute(&doc.model);
                    }
                }
            }
            Event::Tab(i) => {
                // Adım adım (docs/adr/0437): a step with an error is not left forward.
                match e.blocking_step(i) {
                    Some(stuck) => {
                        e.tried = true;
                        e.tab = stuck;
                        self.say(
                            Level::Warn,
                            "Bu adımda sorun var; düzeltmeden sonraki adıma geçilmez.",
                        );
                    }
                    None => e.tab = i,
                }
            }
            Event::OpenRelated(item, uid) => return self.open_related(&item, &uid),
            Event::SourcesAnswered(asked, found) => {
                if let Some(e) = self
                    .gis
                    .form_entry
                    .as_mut()
                    .filter(|e| e.sources_asked == asked)
                {
                    e.sources = found;
                    e.sources_busy = false;
                }
            }
            Event::SourcesAgain => return self.ask_entry_sources(),
            Event::SourceWay(layer, way) => return self.entry_source_way(layer, way),
            Event::DeleteRelated(item, uid) => {
                if let Some(e) = self.gis.form_entry.as_mut() {
                    e.removing = Some((item, uid));
                }
            }
            Event::KeepChild => {
                if let Some(e) = self.gis.form_entry.as_mut() {
                    e.removing = None;
                }
            }
            Event::ConfirmDelete => return self.delete_related(),
            Event::PickOnMap(item) => return self.pick_on_map(&item),
            Event::Focus(item, on) => {
                // Leaving a field asks its checks kept for leaving (docs/adr/0760 §2); entering one says
                // nothing. A field never typed in is left too: its checks see an empty value and pass.
                if !on && e.left.insert(item) {
                    e.recompute(&doc.model);
                }
            }
            Event::RunAction(item) => {
                use kentos_commands::ActionKind as K;
                let Some(FormItem::Action {
                    kind,
                    tool,
                    step,
                    runs,
                    pin,
                    ..
                }) = crate::forms::designer::find(&e.body.items, &item).cloned()
                else {
                    return Task::none();
                };
                let slot = record_slot(&doc.model, &e.uid);
                // The record's layer: the tool's input of objects on it reads the record (docs/adr/0441 §6).
                let layer = slot
                    .and_then(|s| doc.model.get(s))
                    .map(|o| o.base().layer_id.clone());
                match kind {
                    // The record chosen, the tool's window opened with it (a registered tool whose code
                    // can run: docs/adr/8057 §12), where it runs as the action says (docs/adr/0442 §6).
                    K::Tool => {
                        let Some(tool) = tool.filter(|t| {
                            self.pinned_tool_state(t, pin.as_ref())
                                == crate::extensions::PinnedTool::Ready
                        }) else {
                            return Task::none();
                        };
                        if let Some(slot) = slot {
                            self.selection.set([slot]);
                        }
                        self.open_processing(&tool, None);
                        if let Some(d) = self.gis.processing.dialog.as_mut()
                            && let Some(input) = record_input(&d.tool, &d.values, layer.as_deref())
                        {
                            // Its kinds kept: only where it reads from changes.
                            let kinds = d
                                .values
                                .get(&input)
                                .and_then(kentos_processing::FeaturesValue::read)
                                .and_then(|f| f.kinds);
                            let selection = kentos_processing::FeaturesValue {
                                scope: kentos_processing::Scope::Selection,
                                kinds,
                            };
                            d.values.insert(input, selection.to_json());
                            self.refresh_processing();
                        }
                        let place = match runs {
                            kentos_commands::ActionRuns::Auto => None,
                            kentos_commands::ActionRuns::Here => Some("client"),
                            kentos_commands::ActionRuns::Background => Some("worker"),
                        };
                        if let Some(place) = place {
                            let _ = self.update(Message::Gis(gis::Event::Processing(
                                crate::processing::Event::Target(place.into()),
                            )));
                        }
                    }
                    // The calculations again in this window; Kaydet writes them (docs/adr/0442 §5).
                    K::Recompute => {
                        if let (Some(e), Some(doc)) =
                            (self.gis.form_entry.as_mut(), self.document.as_ref())
                        {
                            e.follows = e
                                .body
                                .walk()
                                .into_iter()
                                .filter_map(|(_, i)| match i {
                                    FormItem::Field(f) if f.calculate.is_some() => {
                                        Some(f.field.clone())
                                    }
                                    _ => None,
                                })
                                .collect();
                            for k in e.follows.clone() {
                                e.values.remove(&k);
                            }
                            e.recompute(&doc.model);
                        }
                        self.say(
                            Level::Info,
                            "Hesaplar kayıtta yeniden çalıştı; yazmak için Kaydet’e basın.",
                        );
                    }
                    K::Step => {
                        if let Some(step) = step {
                            return self.take_step(step);
                        }
                    }
                    K::Show => {
                        if let Some(slot) = slot {
                            self.selection.set([slot]);
                            return self.update(Message::Run("view.zoomSelection"));
                        }
                    }
                }
                return Task::none();
            }
            Event::InspectAttachments => {
                if e.facts.is_some() {
                    if let Some(e) = self.gis.form_entry.as_mut() {
                        e.facts = None;
                    }
                    return Task::none();
                }
                let folder = doc
                    .path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(|p| p.display().to_string());
                let input = kentos_commands::AttachmentsInspect {
                    uids: vec![e.uid.clone()],
                    folder,
                };
                // Writes nothing: run on the drawing itself, no copy.
                let Some(d) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = kentos_native_application::attachments_inspect::execute(
                    &mut kentos_native_application::ExecutionContext::new(&mut d.model),
                    input,
                );
                if let kentos_commands::CommandResult::Completed { output, .. } = result
                    && let Some(e) = self.gis.form_entry.as_mut()
                {
                    e.facts = output.objects.into_iter().next().map(|o| o.attachments);
                }
                return Task::none();
            }
            Event::Commit(on) => {
                if let Some(e) = self.gis.form_entry.as_mut() {
                    e.commit = on;
                }
                return Task::none();
            }
            Event::Reviewing(on) => {
                if let Some(e) = self.gis.form_entry.as_mut() {
                    e.reviewing = on;
                    if !on {
                        e.field_notes.clear();
                    }
                }
                return Task::none();
            }
            Event::FieldNote(key, text) => {
                if let Some(e) = self.gis.form_entry.as_mut() {
                    if text.is_empty() {
                        e.field_notes.remove(&key);
                    } else {
                        e.field_notes.insert(key, text);
                    }
                }
                return Task::none();
            }
            Event::Link(item, uid) => return self.link_related(&item, &uid, true),
            Event::Unlink(item, uid) => return self.link_related(&item, &uid, false),
            Event::AttachAdd(_)
            | Event::AttachOpen(_)
            | Event::AttachRemove(_)
            | Event::Step(_)
            | Event::NewRelated(_)
                if preview => {}
            Event::NewRelated(item) => return self.new_related(&item),
            Event::Step(step) => {
                if e.dirty() {
                    self.say(
                        Level::Warn,
                        "Kaydedilmemiş değişiklik var: önce Kaydet’e basın ya da Vazgeç’le bırakın.",
                    );
                    return Task::none();
                }
                let Some((at, _)) = e.place else {
                    return Task::none();
                };
                let form = e.form.clone();
                let layer = e.screen.layer.clone();
                let Some(to) = at.checked_add_signed(step as isize) else {
                    return Task::none();
                };
                let slot = doc
                    .model
                    .by_layer_with_uids(&layer)
                    .nth(to)
                    .and_then(|(u, _)| doc.model.slot_of(u));
                if let Some(slot) = slot {
                    self.selection.set([slot]);
                    return if e.docked {
                        self.open_entry_docked(slot, &form)
                    } else {
                        self.open_entry(slot, &form)
                    };
                }
            }
            Event::AttachAdd(photos) => {
                if let Some(slot) = record_slot(&doc.model, &e.uid) {
                    let add = if photos {
                        crate::attachments::Event::AddPhotos(slot)
                    } else {
                        crate::attachments::Event::Add(slot)
                    };
                    return Task::done(Message::Attachment(add));
                }
            }
            Event::AttachOpen(index) => {
                if let Some(slot) = record_slot(&doc.model, &e.uid) {
                    return self.attachment_event(crate::attachments::Event::Open(slot, index));
                }
            }
            Event::AttachRemove(index) => {
                if let Some(slot) = record_slot(&doc.model, &e.uid) {
                    return self.attachment_event(crate::attachments::Event::Remove(slot, index));
                }
            }
            Event::ShowObject(uid) => {
                let slot = kentos_domain::Uuid::parse_str(&uid)
                    .ok()
                    .and_then(|u| doc.model.slot_of(u));
                if let Some(slot) = slot {
                    self.selection.set([slot]);
                    self.zoom_selection();
                }
            }
            Event::Group(id) => {
                if !e.closed.remove(&id) {
                    e.closed.insert(id);
                }
            }
            Event::FirstIssue => return self.go_to_next_issue(),
            Event::Language(tag) => {
                e.language = tag.filter(|t| e.body.translations.contains_key(t));
            }
            Event::Cancel if e.dirty() && !e.preview && !e.asking_close => {
                e.asking_close = true;
            }
            // Esc or × on the question goes back to the form: it is the question's Vazgeç.
            Event::Cancel if e.asking_close => e.asking_close = false,
            Event::KeepEditing => e.asking_close = false,
            Event::Cancel | Event::Discard => {
                e.asking_close = false;
                if e.docked {
                    let (uid, form) = (e.uid.clone(), e.form.clone());
                    return self.redock(&uid, &form);
                }
                return self.entry_closed();
            }
            Event::Save => {
                // Kaydet ve kapat: a refused write goes back to the form, its issues shown.
                e.asking_close = false;
                return self.save_entry();
            }
            Event::Reload => {
                // A new release: the record's form is its body now; values of fields it no longer
                // has are dropped and said (docs/adr/0442 §2).
                let dropped = self.gis.form_entry.as_mut().map_or_else(Vec::new, |e| {
                    if !e.release_changed {
                        return Vec::new();
                    }
                    e.release_changed = false;
                    let Some(body) = self
                        .document
                        .as_ref()
                        .and_then(|d| {
                            d.model
                                .settings()
                                .forms
                                .iter()
                                .find(|f| f.id == e.form)
                                .cloned()
                        })
                        .and_then(|f| {
                            if e.draft {
                                Some(f.draft)
                            } else {
                                f.releases.last().map(|r| r.body.clone())
                            }
                        })
                    else {
                        return Vec::new();
                    };
                    let bound: Vec<String> = body
                        .walk()
                        .into_iter()
                        .filter_map(|(_, i)| match i {
                            FormItem::Field(f) => Some(f.field.clone()),
                            _ => None,
                        })
                        .collect();
                    let gone: Vec<String> = e
                        .values
                        .keys()
                        .filter(|k| !bound.contains(k))
                        .cloned()
                        .collect();
                    for k in &gone {
                        e.values.remove(k);
                    }
                    e.body = body;
                    gone
                });
                if !dropped.is_empty() {
                    self.say(
                        Level::Warn,
                        format!(
                            "Yeni sürümde olmayan alanlara yazdıklarınız bırakıldı: {}.",
                            dropped.join(", ")
                        ),
                    );
                }
                let task = self.refresh_entry();
                if let Some(e) = self.gis.form_entry.as_mut() {
                    e.before = e.screen.attrs.clone();
                    e.conflict = None;
                }
                if let (Some(e), Some(doc)) = (self.gis.form_entry.as_mut(), self.document.as_ref())
                {
                    e.recompute(&doc.model);
                }
                self.say(
                    Level::Info,
                    "Kayıt yeniden yüklendi; yazdıklarınız duruyor.",
                );
                return task;
            }
            Event::Overwrite => {
                if let Some(e) = self.gis.form_entry.as_mut() {
                    e.overwrite = true;
                    e.conflict = None;
                }
                return self.save_entry();
            }
            Event::Transition(label) => return self.take_step(label),
        }
        Task::none()
    }

    /// A workflow step (docs/adr/0428): the record with the values typed, through `cad.forms.transition`;
    /// the window closes when it went in.
    pub(super) fn take_step(&mut self, label: String) -> Task<Message> {
        let Some(e) = self.gis.form_entry.as_ref() else {
            return Task::none();
        };
        if !e.members.is_empty() || e.preview || e.uid.is_empty() {
            return Task::none();
        }
        let values: BTreeMap<String, String> = e
            .written()
            .into_iter()
            .filter(|(k, _)| {
                let name = e
                    .fields
                    .iter()
                    .find(|f| f.key() == k.as_str())
                    .map(|f| f.name.clone());
                name.is_some_and(|n| e.after.get(&n) != e.before.get(&n))
            })
            .collect();
        let input = kentos_commands::FormsTransition {
            form: e.form.clone(),
            draft: e.draft,
            uid: e.uid.clone(),
            step: label.clone(),
            values,
            notes: e
                .field_notes
                .iter()
                .filter(|(_, v)| !v.trim().is_empty())
                .map(|(k, v)| (k.clone(), v.trim().to_owned()))
                .collect(),
            moment: e.moment.clone(),
            user: e.user.clone(),
            expected_revision: Some(e.revision.clone()),
        };
        let form = e.form.clone();
        let docked = e.docked.then(|| e.uid.clone());
        let state_words = |code: &str| -> String {
            e.screen
                .prepared
                .workflow()
                .and_then(|w| e.fields.iter().find(|f| f.key() == w.field))
                .and_then(|f| f.values.as_ref())
                .and_then(|v| v.iter().find(|c| c.code == code))
                .map_or_else(|| code.to_owned(), |c| c.label.clone())
        };
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = kentos_native_application::forms_transition::execute(
            &mut ExecutionContext::new(&mut doc.model),
            input,
        );
        // The words of the new state, read before the entry is closed.
        let words_of = |done: &kentos_commands::FormsTransitioned| state_words(&done.to);
        match result {
            kentos_commands::CommandResult::Completed { output, .. } => {
                let to = words_of(&output);
                self.gis.form_entry = None;
                self.dialog = None;
                if let Some(uid) = &docked {
                    let _ = self.redock(uid, &form);
                }
                self.say(
                    Level::Success,
                    format!("“{label}”: kayıt “{to}” durumunda."),
                );
                Task::none()
            }
            other => {
                let _ = self.answered(other);
                self.go_to_first_issue()
            }
        }
    }

    pub(super) fn go_to_first_issue(&mut self) -> Task<Message> {
        if let Some(e) = self.gis.form_entry.as_mut() {
            e.issue_at = 0;
        }
        self.go_to_next_issue()
    }

    /// İlk soruna git, then Sonraki sorun (docs/adr/0443 §6): each press the next error in the form's
    /// order, round again after the last; with no error the first issue.
    pub(super) fn go_to_next_issue(&mut self) -> Task<Message> {
        let Some(e) = self.gis.form_entry.as_mut() else {
            return Task::none();
        };
        e.tried = true;
        let errors: Vec<FormIssue> = e.errors().cloned().collect();
        let first = if errors.is_empty() {
            e.issues.first().cloned()
        } else {
            let at = e.issue_at % errors.len();
            e.issue_at = at + 1;
            Some(errors[at].clone())
        };
        let Some(first) = first else {
            return Task::none();
        };
        // A closed group holding it opens.
        if let Some(item) = first.item.as_deref() {
            let holding: Vec<String> = e
                .closed
                .iter()
                .filter(|g| {
                    crate::forms::designer::find(&e.body.items, g)
                        .is_some_and(|group| kentos_forms_find(group.items(), item))
                })
                .cloned()
                .collect();
            for g in holding {
                e.closed.remove(&g);
            }
        }
        if let Some(t) = first.item.as_deref().and_then(|i| e.tab_of(i)) {
            e.tab = t;
        }
        match first.field {
            Some(key) => iced::widget::operation::focus(input_id(&key)),
            None => Task::none(),
        }
    }

    /// Kaydet: the record through the form; the window closes when it went in.
    pub(super) fn save_entry(&mut self) -> Task<Message> {
        // The checks kept for saving are asked now (docs/adr/0438 §2).
        if let (Some(e), Some(doc)) = (self.gis.form_entry.as_mut(), self.document.as_ref())
            && !e.tried
            && e.members.is_empty()
        {
            e.tried = true;
            e.recompute(&doc.model);
        }
        let Some(e) = self.gis.form_entry.as_ref() else {
            return Task::none();
        };
        let first = e.errors().next().map(|i| i.message.clone());
        if let Some(first) = first {
            self.say(
                Level::Warn,
                format!("Kayıt formun kurallarına uymuyor: {first}"),
            );
            return self.go_to_first_issue();
        }
        if !e.members.is_empty() {
            return self.save_many();
        }
        // Only what changed goes in; a new record's defaults with it; a following overridable
        // calculation goes in empty, to be worked out again (docs/adr/0417).
        let values: BTreeMap<String, String> = e
            .written()
            .into_iter()
            .filter(|(k, _)| {
                let name = e
                    .fields
                    .iter()
                    .find(|f| f.key() == k.as_str())
                    .map(|f| f.name.clone());
                name.is_some_and(|n| e.after.get(&n) != e.before.get(&n))
            })
            .collect();
        let input = FormsSubmit {
            form: e.form.clone(),
            draft: e.draft,
            uid: e.uid.clone(),
            values,
            defaults: e.new_record,
            moment: e.moment.clone(),
            user: e.user.clone(),
            // The record and the release the window opened with (docs/adr/0438 §6, §7): another write
            // to this record, or a new release, is said instead of written over; Üzerine yaz drops them.
            expected_revision: None,
            opened: (!e.overwrite).then(|| e.before.clone()),
            children: e.links.iter().map(|(_, c)| c.clone()).collect(),
            commit: e.commit,
            release: if e.draft || e.overwrite {
                None
            } else {
                e.screen.version
            },
        };
        let name = e.name.clone();
        let form = e.form.clone();
        let written_uid = e.uid.clone();
        let docked = e.docked.then(|| e.uid.clone());
        // The "reuse last" fields' values, kept for the next new record when this one goes in.
        let reused: Vec<(String, String)> = e
            .screen
            .prepared
            .reuse_keys()
            .iter()
            .filter_map(|k| {
                let v = e.after.get(e.name_of(k)?)?;
                Some(((*k).to_owned(), v.clone()))
            })
            .collect();
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        // A calculation calling an extension's function waits for its answer on saving (docs/adr/8057 §8): the value
        // goes into the drawing, it is not left empty because the answer had not come while typing.
        let submit = || forms_submit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let result = match kentos_processing::external::expression_calls() {
            Some(waiting) => kentos_forms::answering_with(waiting, submit),
            None => submit(),
        };
        // Changed since the window opened: the bar says what, the values typed stay.
        if let kentos_commands::CommandResult::Conflict { error } = &result
            && matches!(
                error.code.as_str(),
                kentos_commands::codes::RECORD_CHANGED
                    | kentos_commands::codes::FORM_RELEASE_CHANGED
            )
        {
            let words = error.message.clone();
            let release = error.code == kentos_commands::codes::FORM_RELEASE_CHANGED;
            if let Some(e) = self.gis.form_entry.as_mut() {
                e.conflict = Some(words.clone());
                e.overwrite = false;
                e.release_changed = release;
            }
            self.say(Level::Warn, words);
            return Task::none();
        }
        match self.answered(result) {
            Some(done) => {
                let back = self.entry_closed();
                if let Some(uid) = &docked {
                    let _ = self.redock(uid, &form);
                }
                if !reused.is_empty() {
                    self.gis
                        .form_memory
                        .last
                        .entry(form)
                        .or_default()
                        .extend(reused);
                }
                let mut said = match (done.changes.len(), done.children.len()) {
                    (0, _) => "Kayıtta değişiklik yok.".to_owned(),
                    (_, 0) => format!("Kayıt “{name}” formuyla yazıldı."),
                    (_, n) => {
                        format!("Kayıt “{name}” formuyla, {n} alt kaydıyla birlikte yazıldı.")
                    }
                };
                // With Kaynağa da yaz, the file's side of the same step (docs/adr/0532 §1).
                if let Some(committed) = &done.committed {
                    let path = self
                        .document
                        .as_ref()
                        .and_then(|d| {
                            d.model
                                .settings()
                                .sources
                                .iter()
                                .find(|s| {
                                    committed.layers.first().is_some_and(|l| l.layer == s.layer)
                                })
                                .or_else(|| d.model.settings().sources.first())
                                .map(|s| s.path.clone())
                        })
                        .unwrap_or_default();
                    said.push(' ');
                    said.push_str(&self.together_words(committed, &path));
                }
                self.say(Level::Success, said);
                // A value its layer's style draws nothing for: the object leaves the map, said (not taken for deleted).
                if let Some(note) = self.unstyled_note(&[written_uid]) {
                    self.say(Level::Warn, note);
                }
                back
            }
            None => self.go_to_first_issue(),
        }
    }

    /// Kaydet on many records: the values typed go to every one through `cad.forms.fill`.
    pub(super) fn save_many(&mut self) -> Task<Message> {
        let Some(e) = self.gis.form_entry.as_ref() else {
            return Task::none();
        };
        let input = FormsFill {
            form: e.form.clone(),
            draft: e.draft,
            uids: e.members.iter().map(|m| m.uid.clone()).collect(),
            values: e.values.clone(),
            moment: e.moment.clone(),
            user: e.user.clone(),
            expected_revision: Some(e.revision.clone()),
        };
        let (name, count) = (e.name.clone(), e.members.len());
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        // A calculation calling an extension's function waits for its answers (docs/adr/8057 §8).
        let fill = || forms_fill::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let result = match kentos_processing::external::expression_calls() {
            Some(waiting) => kentos_forms::answering_with(waiting, fill),
            None => fill(),
        };
        match self.answered(result) {
            Some(done) => {
                self.gis.form_entry = None;
                self.dialog = None;
                let said = if done.changed.is_empty() {
                    "Kayıtlarda değişiklik yok.".to_owned()
                } else {
                    format!(
                        "“{name}” formuyla {} / {count} kayıt yazıldı.",
                        done.changed.len()
                    )
                };
                self.say(Level::Success, said);
                if let Some(note) = self.unstyled_note(&done.changed) {
                    self.say(Level::Warn, note);
                }
                Task::none()
            }
            None => self.go_to_first_issue(),
        }
    }

    /// How the record's form is opened, a line a fact, and whether its release's fields changed since it
    /// was published (docs/adr/0444 §2, §3).
    pub(super) fn entry_how(&self, e: &Entry, locked: bool) -> (Vec<String>, bool) {
        let form = self
            .project_forms()
            .iter()
            .find(|f| f.id == e.form)
            .cloned();
        let mut lines = Vec::new();
        let release = form.as_ref().and_then(|f| f.releases.last().cloned());
        lines.push(match (&release, e.draft) {
            (_, true) | (None, _) => "Taslağıyla açıldı (yayın yok).".to_owned(),
            (Some(r), false) => format!("{}. yayınla açıldı.", r.version),
        });
        if form.as_ref().is_some_and(|f| f.enforce) {
            lines.push("Kuralları katmanın yazma kuralı: başka yoldan yazılan kayıt da bu formla denetlenir.".into());
        }
        let needs = kentos_forms::requirements(&e.body);
        lines.push(if needs.is_empty() {
            "Saf form: yalnız kaydın değerleriyle çalışır.".into()
        } else {
            format!(
                "Gerektirdikleri: {}; bu bilgisayar hepsini sağlar.",
                needs.join(", ")
            )
        });
        if locked {
            lines.push("Salt okunur: katman kilitli.".into());
        } else if e.screen.prepared.locked_by_state(&e.before) {
            lines.push("Salt okunur: kaydın iş akışı durumu kilitli.".into());
        } else {
            lines.push("Yazılabilir: yazma komutun kendi kurallarıyla.".into());
        }
        let stale = !e.draft
            && release.as_ref().is_some_and(|r| {
                r.fields
                    .as_ref()
                    .is_some_and(|d| *d != kentos_commands::bound_fields_digest(&r.body, &e.fields))
            });
        if stale {
            lines.push("Katmanın alanları bu yayından sonra değişti.".into());
        }
        (lines, stale)
    }
}

/// The tool's input of objects that reads the record a form's action opens it on (docs/adr/0441 §6): the one set to
/// the record's layer, or the first that may read the selection. TAKS opened from a parcel's form reads that parcel,
/// not its layer's every parcel, and its buildings stay as they were.
fn record_input(
    tool: &kentos_processing::Tool,
    values: &kentos_processing::Values,
    layer: Option<&str>,
) -> Option<String> {
    use kentos_processing::{ParamKind, ScopeKind};
    let inputs: Vec<&kentos_processing::ParamDef> = tool
        .parameters
        .iter()
        .filter(|p| match &p.kind {
            ParamKind::Features { scopes, .. } => scopes
                .as_ref()
                .is_none_or(|s| s.contains(&ScopeKind::Selection)),
            _ => false,
        })
        .collect();
    let on_layer = |p: &&&kentos_processing::ParamDef| {
        layer.is_some_and(|l| {
            values
                .get(&p.name)
                .and_then(kentos_processing::FeaturesValue::read)
                .is_some_and(|f| f.scope == kentos_processing::Scope::Layer(l.to_owned()))
        })
    };
    inputs
        .iter()
        .find(on_layer)
        .or(inputs.first())
        .map(|p| p.name.clone())
}
