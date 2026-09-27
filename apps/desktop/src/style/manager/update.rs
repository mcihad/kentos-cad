//! What Stil yöneticisi's events do (the web's `StyleManager` methods):
//! opening and closing, choosing in the tree and the list, the chosen
//! item's fields written back when they are left, copies, deletions, the
//! tree's categories, imports and exports. Every change to the library is
//! kept at once: Kitaplığım in its file, the project's part in the drawing.

use std::time::Instant;

use iced::Task;
use iced::keyboard::key::Named;
use kentos_native_style::file::import_styles;
use kentos_native_style::library::{ItemKind, Source};
use serde_json::{Map, Value, json};

use super::files::import_said;
use super::{
    DOUBLE_CLICK, Event, Field, Manager, PickTarget, SEARCH, details, find_node, node_key,
};
use crate::app::{App, Dialog, Message};

impl App {
    /// Opens Stil yöneticisi (`style.manager`), or in pick mode over Katman
    /// stili or for the selection; the search field takes the keyboard.
    pub(crate) fn open_style_manager(
        &mut self,
        pick: Option<super::Pick>,
        select: Option<String>,
    ) -> Task<Message> {
        self.styles.manager = Some(Manager::new(&self.styles.library, pick, select));
        self.dialog = Some(Dialog::StyleManager);
        iced::widget::operation::focus(SEARCH)
    }

    /// Closes the window: back to Katman stili when it picked for it.
    pub(super) fn close_style_manager(&mut self) {
        let over_layer_style = self.styles.manager.as_ref().is_some_and(|m| {
            matches!(
                m.pick.as_ref().map(|p| &p.target),
                Some(PickTarget::Slot(..))
            )
        });
        self.styles.manager = None;
        self.dialog =
            (over_layer_style && self.styles.layer_style.is_some()).then_some(Dialog::LayerStyle);
    }

    /// Esc, Kapat or the backdrop: a question or a rename open first closes
    /// that; else the window closes, its typed fields kept, back to Katman
    /// stili when it picked for it. Sets the dialog itself (`close_dialog`
    /// has taken it).
    pub(crate) fn style_manager_close_request(&mut self) {
        let Some(m) = &mut self.styles.manager else {
            self.dialog = None;
            return;
        };
        if m.deleting.take().is_some() || m.renaming.take().is_some() {
            self.dialog = Some(Dialog::StyleManager);
            return;
        }
        self.commit_fields();
        self.close_style_manager();
    }

    /// After an edit of the library: Kitaplığım to its file, the project's part into the drawing.
    pub(in crate::style) fn library_changed(&mut self, source: Source) {
        let doc = self.document.as_mut().map(|d| &mut d.model);
        if let Some(problem) = self.styles.changed(source, doc) {
            self.warn(problem);
        }
    }

    /// Whether `source` can be written now: the project's only with a drawing open.
    pub(in crate::style) fn writable(&self, source: Source) -> bool {
        match source {
            Source::System => false,
            Source::User => true,
            Source::Project => self.document.is_some(),
        }
    }

    /// The window's status line, or the command history when it is closed.
    pub(super) fn manager_say(&mut self, text: impl Into<String>, warn: bool) {
        match &mut self.styles.manager {
            Some(m) => m.say(text, warn),
            None if warn => self.warn(text),
            None => self.output(text),
        }
    }

    /// The chosen item's typed fields into the library (Enter, another item, closing).
    pub(super) fn commit_fields(&mut self) {
        let Some(m) = &mut self.styles.manager else {
            return;
        };
        let typed = std::mem::take(&mut m.typed);
        let Some(id) = m.selected.clone() else {
            return;
        };
        for (field, text) in typed {
            self.commit_field(&id, field, &text);
        }
    }

    /// One typed field by its Enter (Ctrl+Enter in Açıklama).
    fn commit_one(&mut self, field: Field) {
        let (Some(id), Some(text)) = (
            self.styles
                .manager
                .as_ref()
                .and_then(|m| m.selected.clone()),
            self.styles
                .manager
                .as_mut()
                .and_then(|m| m.typed.remove(&field)),
        ) else {
            return;
        };
        if self.commit_field(&id, field, &text) && field == Field::Path {
            self.follow_item(&id);
        }
    }

    /// Writes one typed field of item `id`; whether the library changed.
    fn commit_field(&mut self, id: &str, field: Field, text: &str) -> bool {
        let lib = &mut self.styles.library;
        let Some((item, _)) = lib.get(id) else {
            return false;
        };
        let trimmed = kentos_processing::text::js_trim(text);
        let same = match field {
            Field::Name => item.name() == trimmed,
            Field::Path => item.path().join(" / ") == trimmed,
            Field::Reference => item.text("reference").unwrap_or("") == trimmed,
            Field::Description => item.text("description").unwrap_or("") == trimmed,
            Field::Tags => item.tags().join(", ") == trimmed,
        };
        if same {
            return false;
        }
        let result = match field {
            Field::Name => lib.rename(id, trimmed),
            Field::Path => {
                let path: Vec<String> = trimmed
                    .split('/')
                    .map(|s| kentos_processing::text::js_trim(s).to_owned())
                    .filter(|s| !s.is_empty())
                    .collect();
                lib.move_to(id, &path)
            }
            Field::Reference | Field::Description => {
                let key = if field == Field::Reference {
                    "reference"
                } else {
                    "description"
                };
                let mut patch = Map::new();
                // Cleared: the field goes (the web keeps the old text, docs/adr/0092).
                patch.insert(
                    key.into(),
                    if trimmed.is_empty() {
                        Value::Null
                    } else {
                        Value::from(trimmed)
                    },
                );
                lib.update(id, &patch)
            }
            Field::Tags => {
                let tags: Vec<&str> = trimmed
                    .split(',')
                    .map(kentos_processing::text::js_trim)
                    .filter(|t| !t.is_empty())
                    .collect();
                let mut patch = Map::new();
                patch.insert("tags".into(), json!(tags));
                lib.update(id, &patch)
            }
        };
        match result {
            Ok(source) => {
                self.library_changed(source);
                true
            }
            Err(e) => {
                self.manager_say(e, true);
                false
            }
        }
    }

    /// After Kategori's Enter or a copy: the list goes with the item to its category
    /// (the web's stays where it was, and the item leaves it).
    fn follow_item(&mut self, id: &str) {
        let Some((item, source)) = self.styles.library.get(id) else {
            return;
        };
        let path: Vec<String> = item.path().into_iter().map(str::to_owned).collect();
        if let Some(m) = &mut self.styles.manager
            && m.query.is_empty()
        {
            for i in 0..=path.len() {
                m.expanded.insert(node_key(source, &path[..i]));
            }
            m.at = (source, path);
        }
    }

    /// Düzenle, or a double click outside pick mode: the item opens in its
    /// editor (`edit`). A system symbol is copied into Kitaplığım first
    /// (under Sembollerim and its own category) and the copy opens.
    fn edit_item(&mut self, id: &str) {
        let Some((item, source)) = self.styles.library.get(id) else {
            return;
        };
        match item.kind() {
            ItemKind::Symbol => {}
            ItemKind::Asset if item.format() == Some("svg") => {
                self.manager_say(details::SVG_NOT_YET, true);
                return;
            }
            // A picture has nothing to edit, as on the web.
            ItemKind::Asset => return,
        }
        if source.editable() {
            self.open_designer(crate::style::designer::Opening {
                id: Some(id.to_owned()),
                kind: "fill",
                path: None,
                source,
            });
            return;
        }
        let name = item.name().to_owned();
        let mut path = vec!["Sembollerim".to_owned()];
        path.extend(item.path().last().map(|p| (*p).to_owned()));
        match self.styles.library.copy(id, Source::User, None, Some(&path)) {
            Ok(copy) => {
                self.library_changed(Source::User);
                let copy = copy.id().to_owned();
                self.reveal_in_manager(&copy);
                self.manager_say(
                    format!("“{name}” sistem sembolü; kopyası Kitaplığım'a alındı ve açıldı."),
                    false,
                );
                self.open_designer(crate::style::designer::Opening {
                    id: Some(copy),
                    kind: "fill",
                    path: None,
                    source: Source::User,
                });
            }
            Err(e) => self.manager_say(e, true),
        }
    }

    /// Yeni sembol: a new symbol of `kind` in the designer, in the category
    /// the list shows when it is the user's or the project's (and no search
    /// is on), else in Kitaplığım's Sembollerim (`design`).
    fn new_symbol(&mut self, kind: &'static str) {
        let Some(m) = &self.styles.manager else {
            return;
        };
        let editable = m.at.0.editable() && m.query.is_empty() && self.writable(m.at.0);
        let (source, path) = if editable {
            (
                m.at.0,
                (!m.at.1.is_empty()).then(|| m.at.1.clone()),
            )
        } else {
            (Source::User, None)
        };
        self.open_designer(crate::style::designer::Opening {
            id: None,
            kind,
            path,
            source,
        });
    }

    /// Shows an item where it lives (a copy just made, a symbol just saved):
    /// its source and category open, the search cleared, its card chosen (`reveal`).
    pub(in crate::style) fn reveal_in_manager(&mut self, id: &str) {
        if self.styles.manager.is_none() {
            return;
        }
        self.commit_fields();
        let lib = &self.styles.library;
        if let Some(m) = &mut self.styles.manager {
            m.query.clear();
            m.choose_item(lib, Some(id.to_owned()));
        }
        self.follow_item(id);
    }

    /// F2, Delete and Enter with no field holding the keyboard: rename the
    /// chosen category, ask to delete the chosen item, answer the question
    /// or pick (the web's tree F2; the others are the desktop's).
    pub(crate) fn style_manager_key(&mut self, key: Named) -> Task<Message> {
        let Some(m) = &self.styles.manager else {
            return Task::none();
        };
        if m.renaming.is_some() {
            return Task::none();
        }
        let event = match key {
            Named::Enter if m.deleting.is_some() => Event::DeleteConfirmed,
            Named::Enter
                if m.pick.is_some()
                    && m.selected
                        .as_deref()
                        .is_some_and(|id| m.pickable(&self.styles.library, id)) =>
            {
                Event::Choose
            }
            Named::F2
                if m.deleting.is_none()
                    && m.at.0.editable()
                    && !m.at.1.is_empty()
                    && m.query.is_empty() =>
            {
                Event::Rename(m.at.0, m.at.1.clone())
            }
            Named::Delete if m.deleting.is_none() && m.import.is_none() => {
                match m.selected.clone() {
                    Some(id) if self.styles.library.can_edit(&id) => Event::Delete(id),
                    _ => return Task::none(),
                }
            }
            _ => return Task::none(),
        };
        self.style_manager_event(event)
    }

    pub(crate) fn style_manager_event(&mut self, event: Event) -> Task<Message> {
        if self.styles.manager.is_none() {
            return Task::none();
        }
        match event {
            Event::Close => self.style_manager_close_request(),
            Event::Choose => self.choose(),
            Event::Press(id) => self.press(id),
            Event::Search(q) => {
                self.commit_fields();
                if let Some(m) = &mut self.styles.manager {
                    m.query = q;
                }
            }
            Event::Kind(k) => {
                if let Some(m) = &mut self.styles.manager {
                    m.kind = k;
                }
            }
            Event::Toggle(key) => {
                if let Some(m) = &mut self.styles.manager
                    && !m.expanded.remove(&key)
                {
                    m.expanded.insert(key);
                }
            }
            Event::Go(source, path, kids) => {
                self.commit_fields();
                if let Some(m) = &mut self.styles.manager {
                    // A press opens a node, a second press on the chosen one closes it.
                    let key = node_key(source, &path);
                    let again = m.at.0 == source && m.at.1 == path;
                    if kids && (again || !m.expanded.contains(&key)) && !m.expanded.remove(&key) {
                        m.expanded.insert(key);
                    }
                    m.at = (source, path);
                    m.query.clear();
                    m.import = None;
                }
            }
            Event::Field(field, text) => {
                if let Some(m) = &mut self.styles.manager {
                    m.typed.insert(field, text);
                }
            }
            Event::Commit(field) => self.commit_one(field),
            Event::Describe(action) => {
                if let Some(m) = &mut self.styles.manager
                    && let Some((_, notes)) = &mut m.notes
                {
                    let edit = action.is_edit();
                    notes.perform(action);
                    if edit {
                        m.typed.insert(Field::Description, notes.text());
                    }
                }
            }
            Event::Geometry(kind, g) => {
                if let Some(m) = &mut self.styles.manager {
                    m.geometry.insert(kind, g);
                }
            }
            Event::Copy(id, to) => self.copy_item(&id, to),
            Event::Export(ids, name) => return self.export_items(&ids, &name),
            Event::ExportListed | Event::ExportClipboard => {
                let Some(m) = &self.styles.manager else {
                    return Task::none();
                };
                let listed = m.listed(&self.styles.library);
                let ids: Vec<String> = if listed.is_empty() {
                    m.selected.clone().into_iter().collect()
                } else {
                    listed.iter().map(|(id, _)| id.clone()).collect()
                };
                if ids.is_empty() {
                    self.manager_say(
                        "Dışa aktarılacak öğe yok: bir kategori seçin ya da arayın.",
                        true,
                    );
                    return Task::none();
                }
                if matches!(event, Event::ExportClipboard) {
                    return self.export_to_clipboard(&ids);
                }
                let query = kentos_processing::text::js_trim(&m.query);
                let name = if query.is_empty() {
                    m.at.1
                        .last()
                        .cloned()
                        .unwrap_or_else(|| m.at.0.label().to_owned())
                } else {
                    query.to_owned()
                };
                return self.export_items(&ids, &name);
            }
            Event::Written(outcome) => {
                if let (Some(m), Some(outcome)) = (&mut self.styles.manager, outcome) {
                    match outcome {
                        Ok(text) => m.say(text, false),
                        Err(e) => m.say(
                            format!(
                                "Dosya yazılamadı: {e}. Başka bir klasör seçip yeniden deneyin."
                            ),
                            true,
                        ),
                    }
                }
            }
            Event::Delete(id) => {
                if let Some(m) = &mut self.styles.manager {
                    m.deleting = Some(id);
                }
            }
            Event::DeleteCancelled => {
                if let Some(m) = &mut self.styles.manager {
                    m.deleting = None;
                }
            }
            Event::DeleteConfirmed => self.delete_item(),
            Event::Edit(id) => {
                self.commit_fields();
                self.edit_item(&id);
            }
            Event::NewSymbol(kind) => {
                self.commit_fields();
                self.new_symbol(kind);
            }
            Event::Apply(id) => {
                self.commit_fields();
                let text = self.assign_symbol(Some(&id));
                let warn = text.starts_with("0 ") && text.contains("kilitli");
                self.manager_say(text, warn);
            }
            Event::ImportFile => return Self::pick_style_file(),
            Event::ImportClipboard => {
                return iced::clipboard::read().map(|text| super::ev(Event::Pasted(text)));
            }
            Event::Picked(None) => {}
            Event::Picked(Some((name, bytes))) => self.take_file(&name, &bytes),
            Event::Pasted(text) => match text {
                Some(text) => self.offer_import("Pano", &text, true),
                None => self.manager_say(
                    "Pano boş ya da okunamadı; dosyadan içe aktarmayı deneyin.",
                    true,
                ),
            },
            Event::ImportTo(to) => {
                if let Some(d) = self.styles.manager.as_mut().and_then(|m| m.import.as_mut()) {
                    d.to = to;
                }
            }
            Event::ImportMode(mode) => {
                if let Some(d) = self.styles.manager.as_mut().and_then(|m| m.import.as_mut()) {
                    d.mode = mode;
                }
            }
            Event::ImportCancel => {
                if let Some(m) = &mut self.styles.manager {
                    m.import = None;
                }
            }
            Event::ImportRun => self.run_import(),
            Event::NewCategory(source, parent) => return self.new_category(source, parent),
            Event::Rename(source, path) => {
                if let Some(m) = &mut self.styles.manager
                    && let Some(name) = path.last().cloned()
                {
                    m.renaming = Some((source, path, name));
                }
                return iced::widget::operation::focus(kentos_ui::widget::tree_view::RENAME);
            }
            Event::RenameInput(text) => {
                if let Some((_, _, name)) = self
                    .styles
                    .manager
                    .as_mut()
                    .and_then(|m| m.renaming.as_mut())
                {
                    *name = text;
                }
            }
            Event::RenameCancel => {
                if let Some(m) = &mut self.styles.manager {
                    m.renaming = None;
                }
            }
            Event::RenameDone => self.rename_category(),
        }
        Task::none()
    }

    /// A card pressed: chosen; twice quickly, picked (pick mode) or opened to edit.
    fn press(&mut self, id: String) {
        let now = Instant::now();
        let Some(m) = &mut self.styles.manager else {
            return;
        };
        let double = m
            .last_press
            .as_ref()
            .is_some_and(|(last, at)| *last == id && now.duration_since(*at) <= DOUBLE_CLICK);
        m.last_press = (!double).then(|| (id.clone(), now));
        m.import = None;
        if m.selected.as_deref() != Some(id.as_str()) {
            self.commit_fields();
            let lib = &self.styles.library;
            if let Some(m) = &mut self.styles.manager {
                m.choose_item(lib, Some(id.clone()));
            }
        }
        if double {
            if self
                .styles
                .manager
                .as_ref()
                .is_some_and(|m| m.pick.is_some())
            {
                self.choose();
            } else {
                self.edit_item(&id);
            }
        }
    }

    /// Kopyala → Kitaplığıma or Projeye: the copy is chosen.
    fn copy_item(&mut self, id: &str, to: Source) {
        self.commit_fields();
        if !self.writable(to) {
            self.manager_say(
                "Açık çizim yok: proje kitaplığına ancak bir çizim açıkken kopyalanır.",
                true,
            );
            return;
        }
        let name = self
            .styles
            .library
            .get(id)
            .map_or_else(String::new, |(i, _)| i.name().to_owned());
        match self.styles.library.copy(id, to, None, None) {
            Ok(copy) => {
                self.library_changed(to);
                let lib = &self.styles.library;
                if let Some(m) = &mut self.styles.manager {
                    let whose = if to == Source::User {
                        "Kitaplığım"
                    } else {
                        "Proje"
                    };
                    m.say(format!("“{name}” {whose} kitaplığına kopyalandı."), false);
                    m.choose_item(lib, Some(copy.id().to_owned()));
                    // The list goes to the copy, so the chosen card is in view
                    // (the web's stayed where it was).
                    m.query.clear();
                }
                self.follow_item(copy.id());
            }
            Err(e) => self.manager_say(e, true),
        }
    }

    /// The question's Sil: the item leaves its library.
    fn delete_item(&mut self) {
        let Some(id) = self.styles.manager.as_mut().and_then(|m| m.deleting.take()) else {
            return;
        };
        let name = self
            .styles
            .library
            .get(&id)
            .map_or_else(String::new, |(i, _)| i.name().to_owned());
        match self.styles.library.remove(&id) {
            Ok(source) => {
                self.library_changed(source);
                let lib = &self.styles.library;
                if let Some(m) = &mut self.styles.manager {
                    m.choose_item(lib, None);
                    m.say(format!("“{name}” silindi."), false);
                }
            }
            Err(e) => self.manager_say(e, true),
        }
    }

    /// İçe aktar in the import panel: the file's items into the chosen library.
    fn run_import(&mut self) {
        let Some(draft) = self.styles.manager.as_mut().and_then(|m| m.import.take()) else {
            return;
        };
        if !self.writable(draft.to) {
            self.manager_say(
                "Açık çizim yok: projeye ancak bir çizim açıkken aktarılır.",
                true,
            );
            return;
        }
        let r = import_styles(&mut self.styles.library, &draft.file, draft.to, draft.mode);
        self.library_changed(draft.to);
        if let Some(m) = &mut self.styles.manager {
            m.say(import_said(r.added, r.replaced, r.skipped), false);
            m.at = (draft.to, Vec::new());
            m.query.clear();
            m.expanded.insert(node_key(draft.to, &[]));
        }
    }

    /// Yeni alt kategori: “Yeni kategori” (numbered when taken) under the node, named at once.
    fn new_category(&mut self, source: Source, parent: Vec<String>) -> Task<Message> {
        if !self.writable(source) {
            self.manager_say(
                "Açık çizim yok: proje kitaplığı ancak bir çizim açıkken değişir.",
                true,
            );
            return Task::none();
        }
        let Some(m) = &self.styles.manager else {
            return Task::none();
        };
        let trees = m.trees(&self.styles.library);
        let tree = trees
            .iter()
            .find(|(s, _)| *s == source)
            .map_or(&[][..], |(_, t)| t.as_slice());
        let siblings: Vec<String> = if parent.is_empty() {
            tree.iter().map(|c| c.name.clone()).collect()
        } else {
            find_node(tree, &parent)
                .map(|n| n.children.iter().map(|c| c.name.clone()).collect())
                .unwrap_or_default()
        };
        let mut name = "Yeni kategori".to_owned();
        let mut k = 2;
        while siblings.contains(&name) {
            name = format!("Yeni kategori {k}");
            k += 1;
        }
        let mut path = parent.clone();
        path.push(name.clone());
        self.styles
            .library
            .add_category(source, json!({ "path": path }));
        self.library_changed(source);
        if let Some(m) = &mut self.styles.manager {
            for i in 0..=parent.len() {
                m.expanded.insert(node_key(source, &parent[..i]));
            }
            m.renaming = Some((source, path, name));
        }
        iced::widget::operation::focus(kentos_ui::widget::tree_view::RENAME)
    }

    /// The tree's name field left with Enter or a click elsewhere: the category
    /// is renamed, and the list follows it (the web's stays on the old name).
    fn rename_category(&mut self) {
        let Some((source, path, name)) =
            self.styles.manager.as_mut().and_then(|m| m.renaming.take())
        else {
            return;
        };
        let name = kentos_processing::text::js_trim(&name).to_owned();
        if name.is_empty() || path.last() == Some(&name) {
            return;
        }
        self.styles.library.rename_category(source, &path, &name);
        self.library_changed(source);
        if let Some(m) = &mut self.styles.manager {
            let mut renamed = path.clone();
            if let Some(last) = renamed.last_mut() {
                *last = name;
            }
            if m.at.0 == source && m.at.1.starts_with(&path) {
                let mut at = renamed.clone();
                at.extend(m.at.1[path.len()..].iter().cloned());
                m.at.1 = at;
            }
            // The open nodes under it stay open under the new name.
            let old = node_key(source, &path);
            let moved: Vec<String> = m
                .expanded
                .iter()
                .filter(|k| **k == old || k.starts_with(&format!("{old}\u{1}")))
                .cloned()
                .collect();
            let new = node_key(source, &renamed);
            for k in moved {
                m.expanded.remove(&k);
                m.expanded.insert(format!("{new}{}", &k[old.len()..]));
            }
        }
    }
}
