//! The SVG editor's files (the web's `svgFile.ts`, Inkscape's File menu):
//! a new drawing, an SVG opened as a new drawing or into this one (a file,
//! the clipboard, a file dropped on the window), a drawing of the library,
//! saving under a new name, export, document properties, the tracing
//! reference, bitmap tracing and the XML source. Everything that changes
//! the drawing goes through the editor's own undo.
//!
//! - `read.rs`: SVG text into the model; `import.rs`: the import window;
//! - `export.rs`, `docprops.rs`, `trace.rs`, `library.rs`: their windows;
//! - `source.rs`: the XML source under the canvas; `reference.rs`: the
//!   tracing reference and its bar.

pub mod docprops;
pub mod export;
pub mod import;
pub mod library;
pub mod preview;
pub mod read;
pub mod reference;
pub mod source;
pub mod trace;

use iced::Task;
use kentos_native_style::library::{ItemKind, Source};
use kentos_svg_core::export::{SvgTextOptions, write};
use kentos_svg_core::import::{ImportOptions, SymbolColor};
use kentos_svg_core::model::transform_shape;
use kentos_svg_core::shape::Obj;
use kentos_ui::icon::Icon;
use kentos_ui::widget::Menu;

use super::doc::{Drawing, id_of, shape_id};
use super::state::{Original, Question, SvgEditor};
use super::{change, ev};
use crate::app::{App, Message};

/// What the file menu and keys ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileCmd {
    New,
    OpenFile,
    OpenLibrary,
    Paste,
    AddReference,
    Trace,
    Export,
    CopySvg,
    SaveAs,
    DocProps,
    ToggleSource,
}

/// What a picked file is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Open,
    Reference,
    Trace,
}

/// The files' events that need the application (the library, files, the clipboard).
#[derive(Clone, Debug)]
pub enum Event {
    Cmd(FileCmd),
    /// A picture file asked for (Bitmap izle's Görüntü seç…).
    Pick(Purpose),
    Picked(Purpose, Option<(String, Vec<u8>)>),
    Pasted(Option<String>),
    /// A file dropped on the window while the editor is open.
    Dropped(std::path::PathBuf),
    /// Farklı kaydet's Kaydet.
    SaveAs,
    /// A library drawing chosen to open.
    OpenAsset(String),
    /// Dışa aktar's Kaydet… and Panoya kopyala.
    ExportSave,
    ExportCopy,
    ExportWritten(Option<String>),
    /// The source's Kopyala.
    CopySource,
}

/// Another drawing waiting for the unsaved question's answer.
#[derive(Clone, Debug)]
pub enum Pending {
    New,
    Asset(String),
    /// An imported drawing opened as new, with what the status line says after.
    Doc {
        doc: Box<Drawing>,
        name: String,
        said: (String, bool),
        reference: Option<kentos_svg_core::import::ReferenceSpec>,
    },
}

/// Farklı kaydet's fields.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveAs {
    pub name: String,
    pub path: String,
    pub to: Source,
}

/// A window over the editor.
pub enum FileDialog {
    Import(import::ImportDialog),
    Export(export::ExportDialog),
    DocProps(docprops::DocProps),
    Trace(Box<trace::TraceDialog>),
    SaveAs(SaveAs),
    Library(library::Picker),
    /// The reference's position and size.
    Place,
    /// A picture dropped: a reference, or traced.
    Dropped(String, Vec<u8>),
}

#[derive(Default)]
pub struct FileState {
    pub source: Option<source::SourcePanel>,
    pub reference: Option<reference::Reference>,
    pub dialog: Option<FileDialog>,
}

/// The file menu (`menuItems`).
pub fn file_menu(ed: &SvgEditor) -> Menu<Message> {
    let has = !ed.doc.shapes.is_empty();
    let cmd = |c: FileCmd| ev(super::Event::File(Event::Cmd(c)));
    Menu::new()
        .item("Yeni çizim", cmd(FileCmd::New))
        .icon(Icon::DocumentNew)
        .detail("Boş 100 × 100 tuval")
        .item("SVG dosyası aç…", cmd(FileCmd::OpenFile))
        .icon(Icon::Folder)
        .shortcut("Ctrl+O")
        .detail("Yeni çizim olarak ya da bu çizime ekleyerek; dosyayı pencereye bırakmak da olur")
        .item("Kitaplıktan aç…", cmd(FileCmd::OpenLibrary))
        .icon(crate::icons::from_web(Some("styles")))
        .detail("Stil kitaplığındaki bir SVG çizimi (sistem çiziminin kopyası açılır)")
        .item("Panodan içe al", cmd(FileCmd::Paste))
        .icon(crate::icons::from_web(Some("paste")))
        .detail("Panodaki SVG metni")
        .separator()
        .item("İzleme altlığı ekle…", cmd(FileCmd::AddReference))
        .icon(crate::icons::from_web(Some("layers")))
        .item("Bitmap izle…", cmd(FileCmd::Trace))
        .icon(crate::icons::from_web(Some("spline")))
        .separator()
        .item("Dışa aktar…", cmd(FileCmd::Export))
        .icon(Icon::Export)
        .shortcut("Ctrl+Shift+E")
        .item("SVG’yi panoya kopyala", has.then(|| cmd(FileCmd::CopySvg)))
        .icon(crate::icons::from_web(Some("copy")))
        .detail("Sembol SVG’si (currentColor ve param(stroke) ile)")
        .separator()
        .item("Farklı kaydet…", cmd(FileCmd::SaveAs))
        .icon(Icon::SaveAs)
        .shortcut("Ctrl+Shift+S")
        .item("Belge özellikleri…", cmd(FileCmd::DocProps))
        .icon(crate::icons::from_web(Some("settings")))
        .detail("Tuval ve görünüm kutusu, sembol boyu (mm), önizleme zemini")
        .check(
            "XML kaynağı",
            ed.files.source.is_some(),
            cmd(FileCmd::ToggleSource),
        )
        .shortcut("Ctrl+Shift+X")
}

/// A shape's stroke scaled with its geometry (transforms move points, not stroke widths).
pub fn scale_stroke(mut s: Obj, k: f64) -> Obj {
    if k == 1.0 {
        return s;
    }
    s.set_num("strokeWidth", s.num("strokeWidth") * k);
    if let kentos_geometry_core::api::json::Json::Arr(d) = s.get("dash").clone() {
        s.set(
            "dash",
            kentos_geometry_core::api::json::Json::Arr(
                d.into_iter()
                    .map(|v| match v {
                        kentos_geometry_core::api::json::Json::Num(x) => {
                            kentos_geometry_core::api::json::Json::Num(x * k)
                        }
                        other => other,
                    })
                    .collect(),
            ),
        );
    }
    s
}

/// A file name from a drawing's name (`fileSlug`).
pub fn file_slug(s: &str) -> String {
    let lower = kentos_expression::js::text::lower_tr(s);
    let mut out = String::new();
    let mut dash = false;
    for c in lower.chars() {
        let c = match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' | 'i' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            c => c,
        };
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "cizim".to_owned()
    } else {
        out
    }
}

/// The name a file gives a drawing: its stem.
pub fn base_name(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric()) => {
            stem.to_owned()
        }
        _ if name.is_empty() => "Çizim".to_owned(),
        _ => name.to_owned(),
    }
}

impl SvgEditor {
    /// The drawing as the library keeps it (`svgText` with the kept reference).
    pub fn svg_text(&self, pretty: bool) -> String {
        let opts = SvgTextOptions {
            colors: None,
            only: None,
            pretty,
            reference: self.files.reference.as_ref().and_then(|r| r.kept()),
        };
        write(&self.doc.to_obj(), &opts, false)
            .map(|w| w.text)
            .unwrap_or_default()
    }

    /// Another drawing's shapes fitted into this canvas (one undo step); the new ids.
    pub fn add_shapes(&mut self, from: &Drawing) -> Vec<String> {
        let k = (self.doc.width / from.width).min(self.doc.height / from.height);
        let dx = (self.doc.width - from.width * k) / 2.0;
        let dy = (self.doc.height - from.height * k) / 2.0;
        let mut groups: Vec<(String, String)> = Vec::new();
        let added: Vec<Obj> = from
            .shapes
            .iter()
            .filter_map(|s| {
                let t = transform_shape(s, &[k, 0.0, 0.0, k, dx, dy])
                    .ok()
                    .flatten()?;
                let mut t = scale_stroke(t, k);
                t.set_text("id", &shape_id());
                let g = s.text("group").filter(|g| !g.is_empty()).map(|g| {
                    match groups.iter().find(|(o, _)| o == g) {
                        Some((_, n)) => n.clone(),
                        None => {
                            let n = shape_id();
                            groups.push((g.to_owned(), n.clone()));
                            n
                        }
                    }
                });
                t.put("group", g.map(kentos_geometry_core::api::json::Json::Str));
                Some(t)
            })
            .collect();
        if added.is_empty() {
            return Vec::new();
        }
        let ids: Vec<String> = added.iter().map(|s| id_of(s).to_owned()).collect();
        self.edit("import", |ed| ed.doc.shapes.extend(added));
        self.select(ids.clone());
        ids
    }

    /// Asks before another drawing replaces unsaved work (`confirmReplace`); true when it may go on now.
    pub fn may_replace(&mut self, pending: Pending) -> bool {
        if !self.dirty() || !self.doc.has_visible() {
            return true;
        }
        self.question = Some(Question::Replace(Box::new(pending)));
        false
    }

    pub fn toggle_source(&mut self) {
        self.files.source = match self.files.source.take() {
            Some(_) => None,
            None => Some(source::SourcePanel::new(self)),
        };
    }
}

impl App {
    /// The files' events (`SvgFiles`).
    pub(crate) fn svgedit_file(&mut self, e: Event) -> Task<Message> {
        let Some(ed) = self.styles.svg_editor.as_mut() else {
            return Task::none();
        };
        match e {
            Event::Cmd(cmd) => return self.svgedit_cmd(cmd),
            Event::Pick(purpose) => return pick_file(purpose),
            Event::Picked(_, None) => {}
            Event::Picked(purpose, Some((name, bytes))) => {
                return self.svgedit_picked(purpose, &name, bytes);
            }
            Event::Pasted(text) => {
                let text = text.unwrap_or_default();
                if !read::looks_like_svg(&text) {
                    ed.warn("Panoda SVG metni yok: bir çizimin kaynağını (<svg …>) kopyalayın.");
                } else {
                    import::open(ed, &text, "Pano");
                }
            }
            Event::Dropped(path) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                match std::fs::read(&path) {
                    Ok(bytes) => return self.svgedit_dropped(&name, bytes),
                    Err(err) => ed.warn(format!("“{name}” okunamadı: {err}.")),
                }
            }
            Event::SaveAs => self.svgedit_save_as(),
            Event::OpenAsset(id) => {
                ed.files.dialog = None;
                if ed.may_replace(Pending::Asset(id.clone())) {
                    self.svgedit_open_asset(&id);
                }
            }
            Event::ExportSave => return self.svgedit_export_save(),
            Event::ExportCopy => return export::copy(ed),
            Event::ExportWritten(said) => {
                if let Some(said) = said {
                    if !said.starts_with("Dışa aktarılamadı") {
                        ed.files.dialog = None;
                    }
                    ed.say(said);
                }
            }
            Event::CopySource => {
                if let Some(text) = ed.files.source.as_ref().map(|s| s.text()) {
                    ed.say("Kaynak panoya kopyalandı.");
                    return iced::clipboard::write(text);
                }
            }
        }
        Task::none()
    }

    fn svgedit_cmd(&mut self, cmd: FileCmd) -> Task<Message> {
        let Some(ed) = self.styles.svg_editor.as_mut() else {
            return Task::none();
        };
        match cmd {
            FileCmd::New => {
                if ed.may_replace(Pending::New) {
                    new_drawing(ed);
                }
            }
            FileCmd::OpenFile => return pick_file(Purpose::Open),
            FileCmd::OpenLibrary => {
                ed.files.dialog = Some(FileDialog::Library(library::Picker::default()));
            }
            FileCmd::Paste => {
                return iced::clipboard::read().map(|t| ev(super::Event::File(Event::Pasted(t))));
            }
            FileCmd::AddReference => return pick_file(Purpose::Reference),
            FileCmd::Trace => trace::open(ed, None),
            FileCmd::Export => export::open(ed),
            FileCmd::CopySvg => {
                if !ed.doc.shapes.is_empty() {
                    let text = ed.svg_text(true);
                    ed.say("Sembol SVG’si panoya kopyalandı.");
                    return iced::clipboard::write(text);
                }
            }
            FileCmd::SaveAs => {
                if !ed.doc.has_visible() {
                    ed.warn("Boş çizim kaydedilmez: önce bir şekil çizin.");
                } else {
                    let name = if ed.original.is_some() {
                        format!("{} (kopya)", ed.save_name())
                    } else {
                        ed.save_name()
                    };
                    ed.files.dialog = Some(FileDialog::SaveAs(SaveAs {
                        name,
                        path: ed.save_path().join(" / "),
                        to: Source::User,
                    }));
                    return iced::widget::operation::focus(iced::widget::Id::from("svge:saveas"));
                }
            }
            FileCmd::DocProps => {
                ed.files.dialog = Some(FileDialog::DocProps(docprops::DocProps::new(ed)));
            }
            FileCmd::ToggleSource => ed.toggle_source(),
        }
        ed.touch();
        Task::none()
    }

    fn svgedit_picked(&mut self, purpose: Purpose, name: &str, bytes: Vec<u8>) -> Task<Message> {
        let Some(ed) = self.styles.svg_editor.as_mut() else {
            return Task::none();
        };
        match purpose {
            Purpose::Open => {
                let text = String::from_utf8_lossy(&bytes);
                import::open(ed, &text, &base_name(name));
            }
            Purpose::Reference => reference::load(ed, name, &bytes, &self.styles.images),
            Purpose::Trace => trace::open(ed, Some((name.to_owned(), bytes))),
        }
        ed.touch();
        Task::none()
    }

    /// A dropped file: an SVG is imported, a picture asks whether it is a reference or traced.
    fn svgedit_dropped(&mut self, name: &str, bytes: Vec<u8>) -> Task<Message> {
        let Some(ed) = self.styles.svg_editor.as_mut() else {
            return Task::none();
        };
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".svg") || read::looks_like_svg(&String::from_utf8_lossy(&bytes)) {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            import::open(ed, &text, &base_name(name));
        } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) || bytes.starts_with(&[0xFF, 0xD8]) {
            ed.files.dialog = Some(FileDialog::Dropped(name.to_owned(), bytes));
        } else {
            ed.warn(format!(
                "“{name}” alınamadı: SVG, PNG ya da JPEG dosyası bırakın."
            ));
        }
        ed.touch();
        Task::none()
    }

    /// Opens a library drawing in the window (a system one as the user's copy).
    pub(crate) fn svgedit_open_asset(&mut self, id: &str) {
        let editable = self
            .styles
            .library
            .get(id)
            .is_some_and(|(_, s)| s.editable());
        let (mut asset_id, mut note) = (id.to_owned(), String::new());
        if !editable {
            // System drawings are read-only: the copy is opened.
            let path = self.styles.library.get(id).map(|(item, _)| {
                let mut p = vec!["Çizimlerim".to_owned()];
                if let Some(last) = item.path().last() {
                    p.push((*last).to_owned());
                }
                p
            });
            match self
                .styles
                .library
                .copy(id, Source::User, None, path.as_deref())
            {
                Ok(item) => {
                    asset_id = item.id().to_owned();
                    note = " Sistem çizimi: kopyası Kitaplığım’a alındı.".to_owned();
                    self.library_changed(Source::User);
                }
                Err(e) => {
                    if let Some(ed) = self.styles.svg_editor.as_mut() {
                        ed.warn(e);
                    }
                    return;
                }
            }
        }
        let Some((item, source)) = self.styles.library.get(&asset_id) else {
            return;
        };
        let (name, path) = (
            item.name().to_owned(),
            item.path()
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        );
        let Some(ed) = self.styles.svg_editor.as_mut() else {
            return;
        };
        if item.kind() != ItemKind::Asset || item.format() != Some("svg") {
            ed.warn("Yalnızca SVG çizimleri açılır; görüntüler (PNG, JPEG) düzenlenmez.");
            return;
        }
        let data = item.data().unwrap_or("").to_owned();
        match read::read_svg(&data, &editor_options()) {
            Ok(r) => {
                let reference = r.imported.reference.clone();
                ed.open(
                    r.doc,
                    Some(Original {
                        id: asset_id,
                        source,
                        editable: true,
                    }),
                    name.clone(),
                    Some(path),
                );
                reference::restore(ed, reference, &self.styles.images);
                ed.say(format!("“{name}” açıldı.{note}"));
            }
            Err(e) => ed.warn(format!("“{name}” açılamadı: {e}")),
        }
    }

    /// Farklı kaydet's Kaydet: a new library item, which the window now edits.
    fn svgedit_save_as(&mut self) {
        let Some(ed) = self.styles.svg_editor.as_mut() else {
            return;
        };
        let Some(FileDialog::SaveAs(form)) = &ed.files.dialog else {
            return;
        };
        let form = form.clone();
        let name = {
            let t = kentos_processing::text::js_trim(&form.name);
            if t.is_empty() {
                "Adsız çizim".to_owned()
            } else {
                t.to_owned()
            }
        };
        let mut parts: Vec<String> = form
            .path
            .split('/')
            .map(|s| kentos_processing::text::js_trim(s).to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            parts.push("Çizimlerim".to_owned());
        }
        let svg = kentos_native_style::file::sanitize_svg(&ed.svg_text(false));
        let asset = kentos_native_style::file::svg_asset(
            &name,
            &parts,
            &svg,
            &kentos_native_style::library::new_item_id("a"),
        );
        match self.styles.library.add(form.to, asset) {
            Ok(item) => {
                let id = item.id().to_owned();
                self.library_changed(form.to);
                let Some(ed) = self.styles.svg_editor.as_mut() else {
                    return;
                };
                ed.files.dialog = None;
                ed.original = Some(Original {
                    id: id.clone(),
                    source: form.to,
                    editable: true,
                });
                ed.name = name.clone();
                ed.path_text = parts.join(" / ");
                ed.saved();
                ed.say(format!(
                    "“{name}” {} kitaplığına kaydedildi.",
                    if form.to == Source::User {
                        "Kitaplığım"
                    } else {
                        "Proje"
                    }
                ));
                self.svgedit_saved(&id);
            }
            Err(e) => {
                if let Some(ed) = self.styles.svg_editor.as_mut() {
                    ed.warn(e);
                }
            }
        }
    }
}

/// The options the editor reads its own files with (ids, names and groups kept, colours as they are).
pub fn editor_options() -> ImportOptions {
    ImportOptions {
        symbol_color: SymbolColor::Target(None),
        second_color: None,
        editor: true,
    }
}

/// A blank 100 × 100 drawing in the window (`newDrawing`).
pub fn new_drawing(ed: &mut SvgEditor) {
    ed.open(
        Drawing::new(100.0, 100.0),
        None,
        "Yeni çizim".to_owned(),
        None,
    );
    ed.files.reference = None;
    ed.say("Yeni çizim: soldaki araçlarla çizin ya da bir SVG dosyasını pencereye bırakın.");
}

/// The drawing waiting for the question's answer, now opened.
pub fn go_on(app: &mut App, pending: Pending) {
    let images = app.styles.images.clone();
    match pending {
        Pending::New => {
            if let Some(ed) = app.styles.svg_editor.as_mut() {
                new_drawing(ed);
            }
        }
        Pending::Asset(id) => app.svgedit_open_asset(&id),
        Pending::Doc {
            doc,
            name,
            said,
            reference,
        } => {
            if let Some(ed) = app.styles.svg_editor.as_mut() {
                ed.open(*doc, None, name, None);
                reference::restore(ed, reference, &images);
                ed.status(said.0, said.1);
            }
        }
    }
}

fn pick_file(purpose: Purpose) -> Task<Message> {
    let (title, filter, exts): (&str, &str, &[&str]) = match purpose {
        Purpose::Open => ("SVG dosyası aç", "SVG", &["svg"]),
        Purpose::Reference | Purpose::Trace => (
            if purpose == Purpose::Reference {
                "İzleme altlığı seç"
            } else {
                "İzlenecek görüntüyü seç"
            },
            "PNG, JPEG, SVG",
            &["png", "jpg", "jpeg", "svg"],
        ),
    };
    Task::perform(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title(title)
                .add_filter(filter, exts)
                .pick_file()
                .await?;
            let name = file.file_name();
            let bytes = file.read().await;
            Some((name, bytes))
        },
        move |picked| ev(super::Event::File(Event::Picked(purpose, picked))),
    )
}

/// A change for the Farklı kaydet fields.
pub fn save_as_change(f: impl Fn(&mut SaveAs) + Send + Sync + 'static) -> Message {
    change(move |ed| {
        if let Some(FileDialog::SaveAs(form)) = &mut ed.files.dialog {
            f(form);
        }
    })
}
