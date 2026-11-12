//! Opening Formla düzenle: one record or many of a layer, docked or in a window; the editor's state
//! from the form and the record, the designer's preview and the moment a form's rules read.

use super::*;
use crate::gis;

impl App {
    /// Formla düzenle on `slot` through the form `form`.
    pub(crate) fn open_entry(&mut self, slot: Slot, form: &str) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(f) = self.project_forms().iter().find(|f| f.id == form).cloned() else {
            return Task::none();
        };
        let Some(uid) = doc.model.uid(slot).map(|u| u.to_string()) else {
            return Task::none();
        };
        let (body, draft) = match f.releases.last() {
            Some(r) => (r.body.clone(), false),
            None => (f.draft.clone(), true),
        };
        let (moment, user) = self.form_moment();
        let screen = match record_screen(
            &doc.model,
            &f.id,
            &uid,
            draft,
            moment.as_deref(),
            user.as_deref(),
        ) {
            Ok(s) => s,
            Err(error) => {
                self.say(Level::Warn, error.message);
                return Task::none();
            }
        };
        let entry = self.entry_of(
            &f,
            body,
            draft,
            uid,
            screen,
            moment,
            user,
            false,
            BTreeMap::new(),
        );
        self.gis.form_entry = Some(entry);
        self.dialog = Some(Dialog::Gis(gis::GisDialogKind::FormEntry));
        self.ask_entry_sources()
    }

    /// The record's form in the attribute table's Form view (docs/adr/0416): the same editor, no
    /// window.
    pub(crate) fn open_entry_docked(&mut self, slot: Slot, form: &str) -> Task<Message> {
        let task = self.open_entry(slot, form);
        if let Some(e) = self.gis.form_entry.as_mut() {
            e.docked = true;
            if self.dialog == Some(Dialog::Gis(gis::GisDialogKind::FormEntry)) {
                self.dialog = None;
            }
        }
        task
    }

    /// The docked editor again on its record, as the drawing has it now.
    pub(super) fn redock(&mut self, uid: &str, form: &str) -> Task<Message> {
        let slot = self
            .document
            .as_ref()
            .and_then(|d| record_slot(&d.model, uid));
        match slot {
            Some(slot) => self.open_entry_docked(slot, form),
            None => {
                self.gis.form_entry = None;
                Task::none()
            }
        }
    }

    /// Formla düzenle on many records of one layer (docs/adr/0403 §6): what they have the same shows,
    /// a value typed goes to all of them through `cad.forms.fill`.
    pub(crate) fn open_entry_many(&mut self, slots: &[Slot], form: &str) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(f) = self.project_forms().iter().find(|f| f.id == form).cloned() else {
            return Task::none();
        };
        let uids: Vec<String> = slots
            .iter()
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .collect();
        let (body, draft) = match f.releases.last() {
            Some(r) => (r.body.clone(), false),
            None => (f.draft.clone(), true),
        };
        let (moment, user) = self.form_moment();
        let mut screens = Vec::new();
        for uid in &uids {
            match record_screen(
                &doc.model,
                &f.id,
                uid,
                draft,
                moment.as_deref(),
                user.as_deref(),
            ) {
                Ok(s) => screens.push((uid.clone(), s)),
                Err(error) => {
                    self.say(Level::Warn, error.message);
                    return Task::none();
                }
            }
        }
        let members: Vec<Member> = screens
            .iter()
            .map(|(uid, s)| Member {
                uid: uid.clone(),
                attrs: s.attrs.clone(),
                object: s.object.clone(),
            })
            .collect();
        let Some((uid, screen)) = screens.into_iter().next() else {
            return Task::none();
        };
        let mut entry = self.entry_of(
            &f,
            body,
            draft,
            uid,
            screen,
            moment,
            user,
            false,
            BTreeMap::new(),
        );
        let all: Vec<&Attrs> = members.iter().map(|m| &m.attrs).collect();
        let (same, _) = common(&all);
        entry.before = same;
        entry.new_record = false;
        entry.values.clear();
        entry.members = members;
        if let Some(doc) = &self.document {
            entry.recompute(&doc.model);
        }
        for (key, content) in entry.notes.iter_mut() {
            if let Some(f) = entry.fields.iter().find(|f| f.key() == key) {
                *content = text_editor::Content::with_text(
                    entry.after.get(&f.name).map_or("", String::as_str),
                );
            }
        }
        self.gis.form_entry = Some(entry);
        self.dialog = Some(Dialog::Gis(gis::GisDialogKind::FormEntry));
        Task::none()
    }

    /// The window's state for a record through `body`: a new record's defaults and last values, its
    /// groups as the form starts them, the record as the form completes it (`typed`: values kept
    /// from a preview being built again).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn entry_of(
        &self,
        f: &kentos_commands::ProjectForm,
        body: FormBody,
        draft: bool,
        uid: String,
        screen: RecordScreen,
        moment: Option<String>,
        user: Option<String>,
        preview: bool,
        typed: BTreeMap<String, String>,
    ) -> Entry {
        let fields = self.layer_fields_of(&f.layer);
        let before = screen.attrs.clone();
        let new_record = !screen.prepared.keys().iter().any(|k| {
            fields
                .iter()
                .find(|f| f.key() == *k)
                .is_some_and(|f| before.contains_key(&f.name))
        });
        // A new record's "reuse last" fields start with the session's last values, as if typed.
        let mut values = BTreeMap::new();
        if new_record && let Some(last) = self.gis.form_memory.last.get(&f.id) {
            for key in screen.prepared.reuse_keys() {
                if let Some(v) = last.get(key) {
                    values.insert(key.to_owned(), v.clone());
                }
            }
        }
        values.extend(typed);
        let closed = collapsed_groups(&body.items);
        let mut notes = BTreeMap::new();
        for key in fields.iter().map(|f| f.key().to_owned()) {
            notes.insert(key, text_editor::Content::new());
        }
        let revision = self
            .document
            .as_ref()
            .map_or_else(String::new, |d| d.model.revision().to_string());
        let mut entry = Entry {
            form: f.id.clone(),
            name: f.name.clone(),
            draft,
            uid,
            screen,
            body,
            fields,
            before,
            moment,
            user,
            revision,
            preview,
            docked: false,
            new_record,
            values,
            notes,
            tab: 0,
            tried: false,
            conflict: None,
            overwrite: false,
            release_changed: false,
            reviewing: false,
            commit: false,
            field_notes: BTreeMap::new(),
            facts: None,
            language: None,
            issue_at: 0,
            asking_close: false,
            read_only: false,
            removing: None,
            sources: Vec::new(),
            sources_asked: 0,
            sources_busy: false,
            touched: BTreeSet::new(),
            left: kentos_forms::Left::new(),
            after: Attrs::new(),
            items: Vec::new(),
            issues: Vec::new(),
            closed,
            unique_asked: Vec::new(),
            unique: Vec::new(),
            members: Vec::new(),
            mixed: BTreeSet::new(),
            suggestions: BTreeMap::new(),
            queries: BTreeMap::new(),
            searched: BTreeMap::new(),
            pending: BTreeMap::new(),
            turn: 0,
            pages: BTreeMap::new(),
            whole: BTreeMap::new(),
            links: Vec::new(),
            related_pages: BTreeMap::new(),
            snapshot: None,
            place: None,
            follows: BTreeSet::new(),
        };
        if let (Some(doc), false) = (&self.document, preview) {
            let mut at = None;
            let mut n = 0;
            for (u, _) in doc.model.by_layer_with_uids(&f.layer) {
                if u.to_string() == entry.uid {
                    at = Some(n);
                }
                n += 1;
            }
            entry.place = at.map(|i| (i, n));
        }
        if let Some(doc) = &self.document {
            entry.suggestions = kentos_native_application::forms::suggestions(
                &doc.model,
                &f.layer,
                &entry.screen.prepared,
            )
            .into_iter()
            .collect();
        }
        // An overridable calculation follows while it is empty or holds what its calculation gives.
        let overridable: Vec<(String, String)> = entry
            .body
            .fields()
            .into_iter()
            .filter(|f| f.overridable && f.calculate.is_some())
            .filter_map(|f| {
                let name = entry
                    .fields
                    .iter()
                    .find(|x| x.key() == f.field)?
                    .name
                    .clone();
                Some((f.field.clone(), name))
            })
            .collect();
        for (key, name) in overridable {
            let mut emptied = entry.before.clone();
            let held = emptied.remove(&name);
            let (worked, _) =
                entry
                    .screen
                    .prepared
                    .complete(&emptied, &entry.screen.object, entry.new_record);
            if held.is_none() || held == worked.get(&name).cloned() {
                entry.follows.insert(key);
            }
        }
        if let Some(doc) = &self.document {
            entry.recompute(&doc.model);
        }
        // The editors of the long texts start with the values the record shows.
        for (key, content) in entry.notes.iter_mut() {
            if let Some(f) = entry.fields.iter().find(|f| f.key() == key) {
                *content = text_editor::Content::with_text(
                    entry.after.get(&f.name).map_or("", String::as_str),
                );
            }
        }
        entry
    }

    /// The designer's preview of its copy (docs/adr/0403 §3) on the chosen object of the form's
    /// layer, else on a new, empty record; the values typed into the last one kept. The reason it
    /// cannot be shown otherwise.
    pub(crate) fn rebuild_preview(&mut self, body: &FormBody, form: &str) -> Result<(), String> {
        let Some(doc) = &self.document else {
            return Err("Açık çizim yok.".into());
        };
        let Some(f) = self.project_forms().iter().find(|f| f.id == form).cloned() else {
            return Err("Form yok.".into());
        };
        let uid = self
            .selection
            .ids()
            .first()
            .filter(|_| self.selection.len() == 1)
            .and_then(|slot| {
                let e = doc.model.get(*slot)?;
                (e.base().layer_id == f.layer).then(|| doc.model.uid(*slot))?
            })
            .map(|u| u.to_string());
        let (moment, user) = self.form_moment();
        let screen = preview_screen(
            &doc.model,
            &f.id,
            body,
            uid.as_deref(),
            moment.as_deref(),
            user.as_deref(),
        )
        .map_err(|e| e.message)?;
        let kept = self.gis.form_preview.as_ref().map(|e| {
            // The collapsed groups the designer opened.
            let opened: BTreeSet<String> = collapsed_groups(&e.body.items)
                .difference(&e.closed)
                .cloned()
                .collect();
            (e.values.clone(), e.tab, e.touched.clone(), opened)
        });
        let mut entry = self.entry_of(
            &f,
            body.clone(),
            true,
            uid.unwrap_or_default(),
            screen,
            moment,
            user,
            true,
            kept.as_ref().map(|k| k.0.clone()).unwrap_or_default(),
        );
        if let Some((_, tab, touched, opened)) = kept {
            entry.tab = tab;
            entry.touched = touched;
            // A group the designer opened stays open; a new collapsed one starts closed.
            entry.closed.retain(|g| !opened.contains(g));
        }
        // The designer sees every issue the rules give, touched or not.
        entry.tried = true;
        self.gis.form_preview = Some(entry);
        Ok(())
    }

    /// The moment and the person a form's rules read (docs/adr/0402 §5): this device's clock in its
    /// zone to the second (a test's fixed moment instead) and the signed-in person, if any.
    pub(crate) fn form_moment(&self) -> (Option<String>, Option<String>) {
        let moment = self.gis.form_memory.clock.clone().unwrap_or_else(|| {
            let utc = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64);
            let offset = crate::cloud::local_time::Zone::system().offset_at(utc);
            kentos_geometry_core::time::write(((utc + i64::from(offset)) * 1000) as f64, false)
        });
        let user = self
            .gis
            .form_memory
            .person
            .clone()
            .or_else(|| {
                self.cloud
                    .me
                    .as_ref()
                    .map(|m| m.user.display_name.trim().to_owned())
            })
            .filter(|n| !n.is_empty());
        (Some(moment), user)
    }

    pub(crate) fn form_entry_event(&mut self, event: Event) -> Task<Message> {
        self.entry_event(false, event)
    }

    /// An event of the designer's preview: typing, tabs and groups; nothing is written.
    pub(crate) fn form_preview_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Save | Event::Cancel | Event::FirstIssue | Event::Reload | Event::Overwrite => {
                Task::none()
            }
            event => self.entry_event(true, event),
        }
    }
}
