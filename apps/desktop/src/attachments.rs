//! Nesne ekleri (docs/adr/0225 §1, §2; the web's `app/attachments.ts`,
//! `ui/attachments/`): an object's photos and documents as Öznitelikler, the
//! photo viewer, the hover card and the Bilgi tool show them, and the writes
//! that add, rename, embed and remove them through `cad.entities.set`
//! (operation `attachments`, one step “Ekler”).
//!
//! - Dosya ekle… links the files chosen (photos and documents): relative to
//!   the drawing's folder when they are under it, else by their whole path.
//! - Adres ekle… and Düzenle… are the attachment window: an address's name,
//!   address and note, or an attachment's name and note, checked as typed.
//! - Göm keeps a linked photo in the project's library under Ekler (an edit,
//!   no undo step: docs/adr/0092) and makes the attachment embedded.
//! - Aç opens a photo in the viewer, a document and an address in the
//!   system's opener (a document or a picture only: no program is run).
//! - The photos shown are decoded off the UI thread, each once
//!   (`photo_view`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::{Column, button, column, container, row, text_editor};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{Attachment, AttachmentKind, EntitiesSetProperties, PropertiesOperation};
use kentos_domain::Slot;
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message, Picker};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::photo_view::{self, Fit, Picture, Source};

/// The windows' titles, which a trace names them by.
pub const LINK_TITLE: &str = "Adres ekle";
pub const EDIT_TITLE: &str = "Eki düzenle";
pub const VIEWER_TITLE: &str = "Fotoğraf";
/// The library folder an object's embedded photos go under.
pub const PHOTO_PATH: &str = "Ekler";

/// The files Dosya ekle… offers: photos and documents (docs/adr/0225 §1).
const FILES: [&str; 16] = [
    "jpg", "jpeg", "png", "pdf", "txt", "csv", "doc", "docx", "xls", "xlsx", "odt", "ods", "tif",
    "tiff", "dwg", "dxf",
];

/// An attachment as the views show it.
#[derive(Clone, Debug)]
pub(crate) struct AttachmentView {
    pub index: usize,
    pub attachment: Attachment,
    pub kind: AttachmentKind,
    /// The photo's key in the store (`asset:<id>`, `file:<path>`); none for a
    /// document, an address, or a photo's address (the desktop fetches none).
    pub key: Option<String>,
    /// Where it is: “Gömülü fotoğraf (1200×800 piksel)”, “Bağlı dosya: tapu.pdf”, the address.
    pub source: String,
}

/// A kind's word: Fotoğraf, Belge, Bağlantı.
pub(crate) fn kind_word(k: AttachmentKind) -> &'static str {
    match k {
        AttachmentKind::Photo => "Fotoğraf",
        AttachmentKind::Document => "Belge",
        AttachmentKind::Link => "Bağlantı",
    }
}

/// A kind's web icon.
pub(crate) fn kind_icon(k: AttachmentKind) -> &'static str {
    match k {
        AttachmentKind::Photo => "photo",
        AttachmentKind::Document => "document",
        AttachmentKind::Link => "attachmentLink",
    }
}

/// A photo attachment's key in the store.
pub(crate) fn photo_key(a: &Attachment) -> Option<String> {
    if let Some(asset) = &a.asset {
        return Some(format!("asset:{asset}"));
    }
    match (&a.file, a.kind()) {
        (Some(file), AttachmentKind::Photo) => Some(format!("file:{file}")),
        _ => None,
    }
}

/// What is wrong with an attachment as the window says it: its first letter capital, a full stop after it.
fn attachment_problem_text(a: &Attachment) -> Option<String> {
    a.problem().map(|p| {
        let mut chars = p.chars();
        let first: String = chars
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default();
        format!("{first}{}.", chars.as_str())
    })
}

/// A path's last part: a file's name.
fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
}

#[derive(Debug, Clone)]
pub enum Event {
    Add(Slot),
    /// Ekle of a form's photos item (docs/adr/0407 §1): only photos are offered.
    AddPhotos(Slot),
    AddLink(Slot),
    Edit(Slot, usize),
    Remove(Slot, usize),
    Embed(Slot, usize),
    /// Aç: a photo in the viewer, a document or an address outside.
    Open(Slot, usize),
    Picked(Slot, Vec<PathBuf>),
    Decoded(String, Option<Arc<Picture>>),
    FormName(String),
    FormUrl(String),
    FormNote(text_editor::Action),
    FormSave,
    FormCancel,
    ViewerStep(i32),
    ViewerNote(text_editor::Action),
    ViewerSaveNote,
    ViewerOutside,
    ViewerClose,
}

fn msg(e: Event) -> Message {
    Message::Attachment(e)
}

/// What the attachment window edits.
#[derive(Debug, Clone)]
enum Mode {
    Link(Slot),
    Edit(Slot, usize, Attachment),
}

/// Adres ekle and Eki düzenle.
#[derive(Debug)]
pub struct Form {
    mode: Mode,
    name: String,
    url: String,
    note: text_editor::Content,
}

impl Form {
    /// The attachment as typed: the note only when it says something; an
    /// address's own name or its host.
    fn made(&self) -> Attachment {
        let text = self.note.text().replace("\r\n", "\n").replace('\r', "\n");
        let text = text.strip_suffix('\n').map_or(text.clone(), str::to_owned);
        let note = (!text.trim().is_empty()).then_some(text);
        match &self.mode {
            Mode::Edit(_, _, a) => Attachment {
                name: Some(self.name.trim())
                    .filter(|n| !n.is_empty())
                    .map_or_else(|| a.name.clone(), str::to_owned),
                note,
                ..a.clone()
            },
            Mode::Link(_) => {
                let address = self.url.trim().to_owned();
                let mut name = self.name.trim().to_owned();
                if name.is_empty() {
                    name = address
                        .split_once("://")
                        .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or(rest))
                        .filter(|h| !h.is_empty())
                        .unwrap_or(&address)
                        .to_owned();
                }
                Attachment {
                    name,
                    asset: None,
                    file: None,
                    url: Some(address),
                    note,
                }
            }
        }
    }
}

/// The photo viewer: an object's photos one at a time.
#[derive(Debug)]
pub struct Viewer {
    slot: Slot,
    /// The attachment shown, by its place in the object's list.
    index: usize,
    note: text_editor::Content,
}

/// Runs `work` off the UI thread and hands its answer to the app.
fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) -> Message + Send + 'static,
) -> Task<Message> {
    iced_runtime::task::blocking(
        move |mut out: iced::futures::channel::mpsc::Sender<Message>| {
            let answer = done(work());
            let _ =
                iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, answer));
        },
    )
}

impl App {
    /// The attachment window's title: Adres ekle or Eki düzenle.
    pub(crate) fn attachment_title(&self) -> &'static str {
        match self.attachment_form.as_ref().map(|f| &f.mode) {
            Some(Mode::Edit(..)) => EDIT_TITLE,
            _ => LINK_TITLE,
        }
    }

    /// The drawing's folder, a linked file's base.
    pub(crate) fn drawing_folder(&self) -> Option<PathBuf> {
        self.document
            .as_ref()
            .and_then(|d| d.path.as_deref())
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    }

    /// The object's attachments as the views show them.
    pub(crate) fn attachment_views(&self, slot: Slot) -> Vec<AttachmentView> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let Some(e) = doc.model.get(slot) else {
            return Vec::new();
        };
        let folder = self.drawing_folder();
        e.base()
            .attachments
            .iter()
            .enumerate()
            .map(|(index, a)| {
                let kind = a.kind();
                let source = if let Some(asset) = &a.asset {
                    let item = doc.model.styles().items.iter().find(|it| {
                        it.get("kind").and_then(|k| k.as_str()) == Some("asset")
                            && it.get("id").and_then(|k| k.as_str()) == Some(asset)
                    });
                    match item {
                        Some(it) => format!(
                            "Gömülü fotoğraf ({}×{} piksel)",
                            it.get("width")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0),
                            it.get("height")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0)
                        ),
                        None => "Gömülü fotoğraf (kitaplıkta yok)".to_owned(),
                    }
                } else if let Some(file) = &a.file {
                    let here = crate::pictures::resolve(file, folder.as_deref()).exists();
                    format!(
                        "Bağlı dosya: {}{}",
                        base_name(file),
                        if here { "" } else { " (bulunamadı)" }
                    )
                } else {
                    a.url.clone().unwrap_or_default()
                };
                AttachmentView {
                    index,
                    key: photo_key(a),
                    attachment: a.clone(),
                    kind,
                    source,
                }
            })
            .collect()
    }

    /// Where a photo's bytes are, by its key.
    fn photo_source(&self, key: &str) -> Option<Source> {
        if let Some(id) = key.strip_prefix("asset:") {
            let doc = self.document.as_ref()?;
            return doc
                .model
                .styles()
                .items
                .iter()
                .find(|it| {
                    it.get("kind").and_then(|k| k.as_str()) == Some("asset")
                        && it.get("id").and_then(|k| k.as_str()) == Some(id)
                })
                .and_then(|it| it.get("data").and_then(serde_json::Value::as_str))
                .map(|d| Source::Data(d.to_owned()));
        }
        let file = key.strip_prefix("file:")?;
        Some(Source::File(crate::pictures::resolve(
            file,
            self.drawing_folder().as_deref(),
        )))
    }

    /// The photos the shell shows now, decoded off the thread when they are
    /// not yet: the one object selected (Öznitelikler), the hover card's first,
    /// the Bilgi tool's card's, the viewer's.
    pub(crate) fn photo_tasks(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();
        for key in self.wanted_photos() {
            if !self.photos.ask(&key) {
                continue;
            }
            match self.photo_source(&key) {
                Some(source) => tasks.push(off_thread(
                    move || photo_view::decode(&source),
                    move |picture| msg(Event::Decoded(key, picture)),
                )),
                None => self.photos.done(key, None),
            }
        }
        Task::batch(tasks)
    }

    /// The photos the shell shows now, by their keys: the one object
    /// selected's, the hover card's first, the Bilgi card's, the viewer's.
    /// Read from the objects' lists alone: no file is looked at after every
    /// message.
    fn wanted_photos(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let keys_of = |slot: Slot| -> Vec<String> {
            doc.model
                .get(slot)
                .map(|e| e.base().attachments.iter().filter_map(photo_key).collect())
                .unwrap_or_default()
        };
        let mut keys: Vec<String> = Vec::new();
        if let [one] = self.selection.ids() {
            keys.extend(keys_of(*one));
        }
        if let Some(s) = self.hover_card {
            keys.extend(keys_of(s).into_iter().next());
        }
        if let Some((s, _)) = self.info_card {
            keys.extend(keys_of(s));
        }
        if let Some(v) = &self.photo_viewer
            && let Some(key) = doc
                .model
                .get(v.slot)
                .and_then(|e| e.base().attachments.get(v.index))
                .and_then(photo_key)
        {
            keys.push(key);
        }
        keys
    }

    /// The photo `key` names, when it is decoded.
    pub(crate) fn photo_picture(&self, key: Option<&str>) -> Option<Arc<Picture>> {
        key.and_then(|k| self.photos.ready(k))
    }

    /// Whether the object takes edits: its layer is not locked.
    pub(crate) fn attachable(&self, slot: Slot) -> bool {
        self.document.as_ref().is_some_and(|d| {
            d.model
                .get(slot)
                .is_some_and(|e| !d.model.layers().is_locked(&e.base().layer_id))
        })
    }

    /// Writes the object's whole list of attachments, one step “Ekler”; whether it was written.
    pub(crate) fn write_attachments(&mut self, slot: Slot, list: Vec<Attachment>) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let model = &mut doc.model;
        let Some(uid) = model.uid(slot) else {
            return false;
        };
        let input = EntitiesSetProperties {
            line_pattern: None,
            line_type: None,
            line_type_scale: None,
            transparency: None,
            wipeout: None,
            uids: vec![uid.to_string()],
            layer_id: None,
            color: None,
            line_weight: None,
            symbol: None,
            attrs: None,
            label: None,
            operation: PropertiesOperation::Attachments,
            expected_revision: None,
            unlink: false,
            attachments: Some(list),
        };
        let said = kentos_interaction::properties::set_properties(model, input);
        let ok = said.is_empty();
        for s in said {
            self.warn(s);
        }
        ok
    }

    /// The one object selected, when exactly one is; said when not.
    pub(crate) fn attachment_target(&mut self) -> Option<Slot> {
        let slot = (self.selection.len() == 1)
            .then(|| self.selection.ids().iter().next().copied())
            .flatten();
        let Some(slot) = slot else {
            self.warn("Ek tek bir nesneye eklenir: bir nesne seçin.");
            return None;
        };
        if !self.attachable(slot) {
            let layer = self
                .document
                .as_ref()
                .and_then(|d| d.model.get(slot))
                .map(|e| e.base().layer_id.clone())
                .unwrap_or_default();
            self.warn(format!(
                "“{}” katmanı kilitli; nesnesine ek eklenemez. Kilidi Katmanlar panelinden açın.",
                self.layer_name(&layer)
            ));
            return None;
        }
        Some(slot)
    }

    /// Dosya ekle… and Adres ekle… on the one object selected.
    pub(crate) fn attachment_command(&mut self, id: &str) -> Task<Message> {
        let Some(slot) = self.attachment_target() else {
            return Task::none();
        };
        match id {
            "attachment.add" => self.attachment_event(Event::Add(slot)),
            _ => self.attachment_event(Event::AddLink(slot)),
        }
    }

    pub(crate) fn attachment_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Add(slot) | Event::AddPhotos(slot) => match &self.picker {
                Picker::File(path) => {
                    let path = path.clone();
                    return Task::done(msg(Event::Picked(slot, vec![path])));
                }
                Picker::Dialog => {
                    let photos = matches!(e, Event::AddPhotos(_));
                    return Task::perform(
                        async move {
                            let (what, kinds): (&str, &[&str]) = if photos {
                                ("Fotoğraf", &FILES[..3])
                            } else {
                                ("Fotoğraf ve belge", &FILES)
                            };
                            rfd::AsyncFileDialog::new()
                                .set_title(if photos {
                                    "Fotoğraf ekle"
                                } else {
                                    "Dosya ekle"
                                })
                                .add_filter(what, kinds)
                                .pick_files()
                                .await
                                .map(|files| {
                                    files.into_iter().map(|f| f.path().to_path_buf()).collect()
                                })
                                .unwrap_or_default()
                        },
                        move |paths: Vec<PathBuf>| msg(Event::Picked(slot, paths)),
                    );
                }
            },
            Event::Picked(slot, paths) => {
                self.attach_files(slot, paths);
                // A form showing the record counts its attachments again (docs/adr/0407 §1).
                return self.refresh_entry();
            }
            Event::AddLink(slot) => {
                self.attachment_form = Some(Form {
                    mode: Mode::Link(slot),
                    name: String::new(),
                    url: String::new(),
                    note: text_editor::Content::new(),
                });
                self.dialog = Some(Dialog::Attachment);
                return iced::widget::operation::focus(url_id());
            }
            Event::Edit(slot, index) => {
                let Some(a) = self
                    .attachment_views(slot)
                    .get(index)
                    .map(|v| v.attachment.clone())
                else {
                    return Task::none();
                };
                self.attachment_form = Some(Form {
                    name: a.name.clone(),
                    url: String::new(),
                    note: text_editor::Content::with_text(a.note.as_deref().unwrap_or("")),
                    mode: Mode::Edit(slot, index, a),
                });
                self.dialog = Some(Dialog::Attachment);
                return iced::widget::operation::focus(name_id());
            }
            Event::Remove(slot, index) => {
                let mut list: Vec<Attachment> = self
                    .attachment_views(slot)
                    .into_iter()
                    .map(|v| v.attachment)
                    .collect();
                if index < list.len() {
                    list.remove(index);
                    self.write_attachments(slot, list);
                    return self.refresh_entry();
                }
            }
            Event::Embed(slot, index) => self.embed_attachment(slot, index),
            Event::Open(slot, index) => self.open_attachment(slot, index),
            Event::Decoded(key, picture) => self.photos.done(key, picture),
            Event::FormName(t) => {
                if let Some(f) = &mut self.attachment_form {
                    f.name = t;
                }
            }
            Event::FormUrl(t) => {
                if let Some(f) = &mut self.attachment_form {
                    f.url = t;
                }
            }
            Event::FormNote(action) => {
                if let Some(f) = &mut self.attachment_form {
                    f.note.perform(action);
                }
            }
            Event::FormSave => self.attachment_form_save(),
            Event::FormCancel => {
                self.attachment_form = None;
                self.dialog = None;
            }
            Event::ViewerStep(d) => self.viewer_step(d),
            Event::ViewerNote(action) => {
                if let Some(v) = &mut self.photo_viewer {
                    v.note.perform(action);
                }
            }
            Event::ViewerSaveNote => self.viewer_save_note(),
            Event::ViewerOutside => self.viewer_outside(),
            Event::ViewerClose => {
                self.photo_viewer = None;
                // A photo opened from Formla düzenle goes back to its form.
                self.dialog = self.form_entry.is_some().then_some(Dialog::FormEntry);
            }
        }
        Task::none()
    }

    /// The files chosen, linked to the object after its own attachments.
    fn attach_files(&mut self, slot: Slot, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        let folder = self.drawing_folder();
        let mut list: Vec<Attachment> = self
            .attachment_views(slot)
            .into_iter()
            .map(|v| v.attachment)
            .collect();
        let n = paths.len();
        let first = paths
            .first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for path in paths {
            let name = path.file_name().map_or_else(
                || path.to_string_lossy().into_owned(),
                |n| n.to_string_lossy().into_owned(),
            );
            // Under the drawing's folder: relative, so the folder moves with it.
            let link = folder
                .as_deref()
                .and_then(|f| path.strip_prefix(f).ok())
                .map_or_else(
                    || path.to_string_lossy().into_owned(),
                    |r| r.to_string_lossy().replace('\\', "/"),
                );
            list.push(Attachment {
                name,
                asset: None,
                file: Some(link),
                url: None,
                note: None,
            });
        }
        if let Some(problem) = kentos_commands::attachments_problem(&list) {
            self.warn(format!("Eklenemedi: {problem}."));
            return;
        }
        if self.write_attachments(slot, list) {
            self.say(
                kentos_interaction::Level::Success,
                if n == 1 {
                    format!("“{first}” eklendi.")
                } else {
                    format!("{n} dosya eklendi.")
                },
            );
        }
    }

    /// Göm: a linked photo's file kept in the project's library under Ekler,
    /// the attachment made embedded, one step “Ekler”.
    fn embed_attachment(&mut self, slot: Slot, index: usize) {
        let Some(v) = self.attachment_views(slot).get(index).cloned() else {
            return;
        };
        let Some(file) = &v.attachment.file else {
            return;
        };
        let path = crate::pictures::resolve(file, self.drawing_folder().as_deref());
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.warn(format!("“{}” okunamadı: {e}.", path.display()));
                return;
            }
        };
        let (id, mut item) = match crate::pictures::library_item(&v.attachment.name, &bytes) {
            Ok(made) => made,
            Err(why) => {
                self.warn(why);
                return;
            }
        };
        item["path"] = serde_json::json!([PHOTO_PATH]);
        if let Some(doc) = self.document.as_mut() {
            crate::pictures::keep_in_library(&mut doc.model, &id, item);
        }
        let mut list: Vec<Attachment> = self
            .attachment_views(slot)
            .into_iter()
            .map(|v| v.attachment)
            .collect();
        if let Some(a) = list.get_mut(index) {
            *a = Attachment {
                name: a.name.clone(),
                asset: Some(id),
                file: None,
                url: None,
                note: a.note.clone(),
            };
        }
        self.write_attachments(slot, list);
    }

    /// Aç: a photo in the viewer; a document and an address in the system's opener.
    fn open_attachment(&mut self, slot: Slot, index: usize) {
        let Some(v) = self.attachment_views(slot).get(index).cloned() else {
            return;
        };
        if v.key.is_some() {
            self.photo_viewer = Some(Viewer {
                slot,
                index,
                note: text_editor::Content::with_text(v.attachment.note.as_deref().unwrap_or("")),
            });
            self.dialog = Some(Dialog::PhotoViewer);
            return;
        }
        if let Some(url) = &v.attachment.url {
            if let Err(e) = crate::sheet_pdf::open_with_viewer(Path::new(url)) {
                self.warn(format!("“{}” açılamadı: {e}.", v.attachment.name));
            }
            return;
        }
        if let Some(file) = &v.attachment.file {
            self.open_document(&v.attachment.name, file);
        }
    }

    fn attachment_form_save(&mut self) {
        let Some(f) = &self.attachment_form else {
            return;
        };
        let made = f.made();
        if made.problem().is_some() {
            return;
        }
        let ok = match f.mode.clone() {
            Mode::Link(slot) => {
                let mut list: Vec<Attachment> = self
                    .attachment_views(slot)
                    .into_iter()
                    .map(|v| v.attachment)
                    .collect();
                list.push(made);
                self.write_attachments(slot, list)
            }
            Mode::Edit(slot, index, _) => {
                let mut list: Vec<Attachment> = self
                    .attachment_views(slot)
                    .into_iter()
                    .map(|v| v.attachment)
                    .collect();
                match list.get_mut(index) {
                    Some(a) => {
                        *a = made;
                        self.write_attachments(slot, list)
                    }
                    None => false,
                }
            }
        };
        if ok {
            self.attachment_form = None;
            self.dialog = None;
        }
    }

    /// The viewer's photos: the object's photo attachments, by their place in its list.
    fn viewer_photos(&self, slot: Slot) -> Vec<AttachmentView> {
        self.attachment_views(slot)
            .into_iter()
            .filter(|v| v.key.is_some())
            .collect()
    }

    fn viewer_step(&mut self, d: i32) {
        let Some(v) = &self.photo_viewer else {
            return;
        };
        let photos = self.viewer_photos(v.slot);
        if photos.is_empty() {
            return;
        }
        let at = photos.iter().position(|p| p.index == v.index).unwrap_or(0) as i32;
        let next = (at + d).rem_euclid(photos.len() as i32) as usize;
        let shown = &photos[next];
        let note = text_editor::Content::with_text(shown.attachment.note.as_deref().unwrap_or(""));
        if let Some(v) = &mut self.photo_viewer {
            v.index = shown.index;
            v.note = note;
        }
    }

    fn viewer_save_note(&mut self) {
        let Some(v) = &self.photo_viewer else {
            return;
        };
        let (slot, index) = (v.slot, v.index);
        let text = v.note.text().replace("\r\n", "\n");
        let text = text.strip_suffix('\n').map_or(text.clone(), str::to_owned);
        let mut list: Vec<Attachment> = self
            .attachment_views(slot)
            .into_iter()
            .map(|v| v.attachment)
            .collect();
        if let Some(a) = list.get_mut(index) {
            a.note = (!text.trim().is_empty()).then_some(text);
            self.write_attachments(slot, list);
        }
    }

    /// Dışarıda aç: a linked photo's file, an embedded one's bytes written beside the cache, in the system's viewer.
    fn viewer_outside(&mut self) {
        let Some(v) = &self.photo_viewer else {
            return;
        };
        let Some(shown) = self.attachment_views(v.slot).get(v.index).cloned() else {
            return;
        };
        let path = match (&shown.attachment.file, &shown.key) {
            (Some(file), _) => crate::pictures::resolve(file, self.drawing_folder().as_deref()),
            (None, Some(key)) => {
                let Some(Source::Data(url)) = self.photo_source(key) else {
                    return;
                };
                let Some(bytes) = url
                    .split_once(";base64,")
                    .and_then(|(_, d)| kentos_sheet::template::base64_decode(d))
                else {
                    return;
                };
                let ext = if bytes.starts_with(&[0x89, b'P']) {
                    "png"
                } else {
                    "jpg"
                };
                let path = std::env::temp_dir()
                    .join(format!("kentos-{}.{ext}", key.trim_start_matches("asset:")));
                if let Err(e) = std::fs::write(&path, bytes) {
                    self.warn(format!("Fotoğraf açılamadı: {e}."));
                    return;
                }
                path
            }
            _ => return,
        };
        if let Err(e) = crate::sheet_pdf::open_with_viewer(&path) {
            self.warn(format!("“{}” açılamadı: {e}.", shown.attachment.name));
        }
    }

    pub(crate) fn attachment_form_view(&self) -> Element<'_, Message> {
        let Some(f) = &self.attachment_form else {
            return label::caption("").into();
        };
        let made = f.made();
        let problem = attachment_problem_text(&made);
        let editing = matches!(f.mode, Mode::Edit(..));
        let input =
            |placeholder: &str, value: &str, id: iced::widget::Id, on: fn(String) -> Event| {
                kentos_ui::widget::text_field(placeholder, value)
                    .id(id)
                    .on_input(move |t| msg(on(t)))
                    .on_submit_maybe(problem.is_none().then(|| msg(Event::FormSave)))
            };
        let note = text_editor(&f.note)
            .on_action(|a| msg(Event::FormNote(a)))
            .height(72)
            .padding(6)
            .size(typography::body());
        let mut body = Column::new().spacing(12);
        if !editing {
            body = body.push(words::field(
                "Adres",
                input("https://", &f.url, url_id(), Event::FormUrl),
                Some("http:// ya da https:// ile başlar.".to_owned()),
            ));
        }
        body = body
            .push(words::field(
                "Ad",
                input(
                    if editing { "" } else { "Kurum sayfası" },
                    &f.name,
                    name_id(),
                    Event::FormName,
                ),
                (!editing).then(|| "Boşsa adresin sunucusu.".to_owned()),
            ))
            .push(words::field(
                "Not",
                note,
                editing.then(|| "Satırlar olabilir; en çok 1 000 karakter.".to_owned()),
            ))
            .push(words::summary(vec![match &problem {
                Some(p) => words::text_line(Kind::Warn, p.clone()),
                None => words::text_line(
                    Kind::Ok,
                    if editing {
                        "Ekin adı ve notu yazılacak."
                    } else {
                        "Adres nesneye eklenecek."
                    },
                ),
            }]));
        Frame::new(if editing { EDIT_TITLE } else { LINK_TITLE })
            .push(body)
            .action(words::secondary("Vazgeç", Some(msg(Event::FormCancel))))
            .action(words::primary(
                "Kaydet",
                problem.is_none().then(|| msg(Event::FormSave)),
            ))
            .width(520.0)
            .modal(msg(Event::FormCancel))
    }

    pub(crate) fn photo_viewer_view(&self) -> Element<'_, Message> {
        let Some(v) = &self.photo_viewer else {
            return label::caption("").into();
        };
        let photos = self.viewer_photos(v.slot);
        let Some(at) = photos.iter().position(|p| p.index == v.index) else {
            return label::caption("").into();
        };
        let shown = &photos[at];
        let picture = self.photo_picture(shown.key.as_deref());
        let missing = matches!(
            shown.key.as_deref().and_then(|k| self.photos.get(k)),
            Some(photo_view::Slot::Missing)
        );
        let frame: Element<'_, Message> = if missing {
            container(
                column![
                    icon(from_web(Some("photo"))).size(32.0),
                    label::muted("Fotoğraf okunamadı ya da bulunamadı"),
                ]
                .spacing(8)
                .align_x(Center),
            )
            .width(Fill)
            .height(520)
            .center_x(Fill)
            .center_y(520)
            .into()
        } else {
            container(photo_view::photo(picture, 880.0, 520.0, Fit::Contain))
                .width(Fill)
                .center_x(Fill)
                .into()
        };
        let editable = self.attachable(v.slot);
        let caption = row![
            label::strong(shown.attachment.name.clone()),
            label::mono(format!("{}/{}", at + 1, photos.len())),
            container(label::muted(shown.source.clone())).width(Fill),
        ]
        .spacing(12)
        .align_y(Center);
        let note = text_editor(&v.note)
            .placeholder("Not yok")
            .on_action(move |a| msg(Event::ViewerNote(a)))
            .height(56)
            .padding(6)
            .size(typography::body());
        let changed =
            v.note.text().trim_end_matches('\n') != shown.attachment.note.as_deref().unwrap_or("");
        let mut notes = row![container(note).width(Fill)].spacing(8);
        if editable {
            notes = notes.push(
                button(label::body("Notu kaydet"))
                    .on_press_maybe(changed.then(|| msg(Event::ViewerSaveNote)))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::secondary),
            );
        }
        let many = photos.len() > 1;
        Frame::new(VIEWER_TITLE)
            .push(column![frame, caption, notes].spacing(10))
            .action(words::secondary(
                "Önceki",
                many.then(|| msg(Event::ViewerStep(-1))),
            ))
            .action(words::secondary(
                "Sonraki",
                many.then(|| msg(Event::ViewerStep(1))),
            ))
            .action(words::secondary(
                "Dışarıda aç",
                Some(msg(Event::ViewerOutside)),
            ))
            .action(words::primary("Kapat", Some(msg(Event::ViewerClose))))
            .width(940.0)
            .modal(msg(Event::ViewerClose))
    }
}

fn url_id() -> iced::widget::Id {
    iced::widget::Id::new("ek-adres")
}

fn name_id() -> iced::widget::Id {
    iced::widget::Id::new("ek-ad")
}

#[cfg(test)]
impl App {
    /// The photos the shell shows, decoded now as `photo_tasks` decodes them
    /// off the thread (the pictures' scenes run no tasks: a photo an update
    /// asked for is still waiting for its task).
    pub(crate) fn photos_now(&mut self) {
        for key in self.wanted_photos() {
            if self.photos.ready(&key).is_some() {
                continue;
            }
            self.photos.ask(&key);
            let picture = self.photo_source(&key).and_then(|s| photo_view::decode(&s));
            self.photos.done(key, picture);
        }
    }
}
