//! Bloklar › Kitaplık (docs/adr/0726 §1): the device's block libraries, folders of DWG, DXF and KCAD drawings, as
//! AutoCAD's Blocks palette shows its Libraries. A library shows its folders and drawings in the natural order; a
//! drawing opened shows itself as one block (Dosyanın tamamı) and its blocks with their pictures, real sizes and
//! units, read off the drawing's thread. Yerleştir takes a block in through `file.import.block` with the library's
//! category (the library's name, the folders down to the file and the file's name) and starts Blok ekle with it. A
//! file that names no unit asks for one first: its sizes are said only once it is chosen, and the choice stays the
//! file's for the session. The libraries are kept beside the recent files (`blok-kitapliklari.json`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::{Canvas, Column, button, column, container, row, text, tooltip};
use iced::{Center, Element, Fill, Task, Theme};
use kentos_commands::blocks::name_key;
use kentos_commands::{
    BlockFileMode, BlockUnits, CommandResult, DrawingUnit, Entity, ImportBlock,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, file_import_block};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{ContextMenu, Menu, SearchBox, Tip, VirtualList, tip};
use kentos_ui::{label, style};
use serde::{Deserialize, Serialize};

use crate::app::{App, Message};
use crate::blocks_panel::{Thumb, thumb_tile};

const FORMAT: &str = "kentos.block-libraries";
const FILE: &str = "blok-kitapliklari.json";
const VERSION: u32 = 1;
/// A row's height and its picture's side.
const ROW: f32 = 46.0;
const THUMB: f32 = 34.0;
/// The units a file that names none may be read in, in the order offered.
const UNITS: [(DrawingUnit, &str); 5] = [
    (DrawingUnit::Mm, "mm"),
    (DrawingUnit::Cm, "cm"),
    (DrawingUnit::M, "m"),
    (DrawingUnit::In, "inç"),
    (DrawingUnit::Ft, "fit"),
];

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    format: String,
    version: u32,
    folders: Vec<PathBuf>,
}

/// What the Kitaplık view keeps while the app runs: the libraries (and where they are kept), the folder shown, the
/// drawing opened, the drawings read and the units chosen for those that name none.
#[derive(Debug, Default)]
pub struct Library {
    file: Option<PathBuf>,
    pub(crate) roots: Vec<PathBuf>,
    /// The folder shown; none: the libraries.
    pub(crate) at: Option<PathBuf>,
    /// The drawing whose blocks are shown.
    pub(crate) open: Option<PathBuf>,
    read: HashMap<PathBuf, Reading>,
    units: HashMap<PathBuf, DrawingUnit>,
    query: String,
}

/// A drawing being read, read, or not readable.
#[derive(Debug, Clone)]
pub(crate) enum Reading {
    Busy,
    Done(Arc<FileBlocks>),
    Failed(String),
}

/// What a drawing offers: itself as one block, and its blocks; whether it named its unit.
#[derive(Debug, Clone, Default)]
pub struct FileBlocks {
    /// The unit it was read in; none when it names none and none was chosen: no size is said then.
    pub unit: Option<DrawingUnit>,
    pub whole: Option<Item>,
    pub blocks: Vec<Item>,
}

/// A block a drawing offers: its name, its real size (metres), its unit, its outline at the origin.
#[derive(Debug, Clone, Default)]
pub struct Item {
    pub name: String,
    pub size: Option<(f64, f64)>,
    pub units: Option<BlockUnits>,
    pub paths: Vec<f64>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Search(String),
    AddRoot,
    RootChosen(Option<PathBuf>),
    RemoveRoot(PathBuf),
    /// A folder shown; none: the libraries.
    Go(Option<PathBuf>),
    /// A drawing's blocks shown, read when they are not yet.
    Open(PathBuf),
    Read(PathBuf, Option<DrawingUnit>, Result<Arc<FileBlocks>, String>),
    /// The unit a drawing naming none is read in.
    Unit(PathBuf, DrawingUnit),
    /// Yerleştir: a block of the drawing by its name, or the whole drawing.
    Place(PathBuf, Option<String>),
}

fn msg(e: Event) -> Message {
    Message::BlocksPanel(crate::blocks_panel::Event::Library(e))
}

/// Whether `path` is a drawing a library offers: DWG, DXF or KCAD.
pub(crate) fn is_drawing(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "dwg" | "dxf" | "kcad"))
}

fn is_cad(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "dwg" | "dxf"))
}

fn file_unit(u: DrawingUnit) -> kentos_commands::FileUnit {
    use kentos_commands::FileUnit as F;
    match u {
        DrawingUnit::Mm => F::Mm,
        DrawingUnit::Cm => F::Cm,
        DrawingUnit::M => F::M,
        DrawingUnit::In => F::In,
        DrawingUnit::Ft => F::Ft,
    }
}

fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn natural(names: Vec<String>) -> Vec<String> {
    kentos_geometry_core::ops::point_editor::natural_order(&names)
        .into_iter()
        .map(|i| names[i as usize].clone())
        .collect()
}

/// A folder's folders and drawings, in the natural order; those whose name begins with a dot are left out.
pub(crate) fn listing(folder: &Path) -> (Vec<String>, Vec<String>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return (Vec::new(), Vec::new());
    };
    let (mut dirs, mut files) = (Vec::new(), Vec::new());
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = e.path();
        if path.is_dir() {
            dirs.push(name);
        } else if is_drawing(&path) {
            files.push(name);
        }
    }
    (natural(dirs), natural(files))
}

/// A drawing's blocks as a library offers them (off the drawing's thread): read as Aç reads a CAD file, in `unit`
/// when given; the sizes said only when the unit is known (docs/adr/0726 §2).
pub fn read_file(path: &Path, unit: Option<DrawingUnit>) -> Result<FileBlocks, String> {
    let bytes = std::fs::read(path)
        .map_err(|_| format!("{} okunamadı: dosya yok ya da açılamıyor.", name_of(path)))?;
    let (doc, known) = if is_cad(path) {
        let format = kentos_native_application::cad_open::Format::of(path)
            .ok_or_else(|| format!("{} bir CAD çizimi değil.", name_of(path)))?;
        let opened = kentos_native_application::cad_open::read_in(
            path,
            &bytes,
            format,
            unit.map(file_unit),
            &mut kentos_formats::watch::Quiet,
        )
        .map_err(|r| r.message)?;
        let known = unit.is_some() || opened.unit.as_ref().is_some_and(|u| !u.assumed);
        (opened.document, known)
    } else {
        let snapshot = file_import_block::read_drawing(&bytes)?;
        (kentos_domain::Document::from_snapshot_v2(snapshot)?, true)
    };
    let spatial = kentos_interaction::Spatial::of(&doc);
    let blocks = doc
        .blocks()
        .iter()
        .filter(|b| !b.name.starts_with('*') && b.xref.is_none())
        .map(|b| Item {
            name: b.name.clone(),
            size: known
                .then(|| kentos_native_application::blocks::definition_box(&doc, b.id, None))
                .flatten()
                .map(|[x0, y0, x1, y1]| (x1 - x0, y1 - y0)),
            units: b.units,
            paths: spatial.store().insert_outlines(
                &b.id.to_text(),
                kentos_interaction::Vec2::new(0.0, 0.0),
                1.0,
                0.0,
                false,
            ),
        })
        .collect();
    // The drawing as one block: its objects but pictures, rasters and tables (docs/adr/0316 §3).
    let kept = doc
        .entities()
        .filter(|e| !matches!(e, Entity::Image(_) | Entity::Raster(_) | Entity::Table(_)));
    let whole = kentos_native_application::sheets_fit::extent_of(&doc, kept).map(|[x0, y0, x1, y1]| Item {
        name: path
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned()),
        size: known.then_some((x1 - x0, y1 - y0)),
        units: None,
        paths: Vec::new(),
    });
    Ok(FileBlocks {
        unit: if known { unit.or(Some(doc.settings().unit())) } else { None },
        whole,
        blocks,
    })
}

impl Library {
    /// The libraries kept in `folder`; an unreadable file is none.
    pub fn open(folder: &Path) -> Self {
        let stored = std::fs::read_to_string(folder.join(FILE))
            .ok()
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok())
            .filter(|s| s.format == FORMAT && s.version == VERSION)
            .unwrap_or_default();
        Self {
            file: Some(folder.join(FILE)),
            roots: stored.folders,
            ..Self::default()
        }
    }

    fn keep(&self) {
        let Some(file) = &self.file else {
            return;
        };
        let stored = Stored {
            format: FORMAT.into(),
            version: VERSION,
            folders: self.roots.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&stored) {
            let _ = std::fs::write(file, text + "\n");
        }
    }

    /// Adds a library; false when it is there already.
    pub fn add(&mut self, folder: &Path) -> bool {
        if self.roots.iter().any(|r| r == folder) {
            return false;
        }
        self.roots.push(folder.to_path_buf());
        self.keep();
        true
    }

    fn remove(&mut self, folder: &Path) {
        self.roots.retain(|r| r != folder);
        if self.at.as_deref().is_some_and(|a| a.starts_with(folder)) {
            self.at = None;
            self.open = None;
        }
        self.keep();
    }

    /// The library `path` is in.
    fn root_of(&self, path: &Path) -> Option<&PathBuf> {
        self.roots.iter().find(|r| path.starts_with(r))
    }

    /// The category a library's block goes in (docs/adr/0726 §1): the library's name, the folders down to the file,
    /// and for a block of a drawing the drawing's name.
    pub fn category(&self, file: &Path, whole: bool) -> Option<String> {
        let root = self.root_of(file)?;
        let mut parts = vec![name_of(root)];
        if let Ok(rest) = file.parent().unwrap_or(file).strip_prefix(root) {
            parts.extend(rest.components().map(|c| c.as_os_str().to_string_lossy().into_owned()));
        }
        if !whole && let Some(stem) = file.file_stem() {
            parts.push(stem.to_string_lossy().into_owned());
        }
        let parts: Vec<String> = parts.into_iter().map(|p| p.trim().to_owned()).filter(|p| !p.is_empty()).collect();
        (!parts.is_empty()).then(|| parts.join("/"))
    }
}

impl App {
    pub(crate) fn blocks_library_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Search(t) => self.blocks_panel.library.query = t,
            Event::AddRoot => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Blok kitaplığı ekle")
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |path| msg(Event::RootChosen(path)),
                );
            }
            Event::RootChosen(None) => {}
            Event::RootChosen(Some(path)) => {
                if self.blocks_panel.library.add(&path) {
                    self.say(
                        Level::Success,
                        format!("Bloklar: “{}” kitaplığı eklendi.", name_of(&path)),
                    );
                }
                self.blocks_panel.library.at = Some(path);
                self.blocks_panel.library.open = None;
            }
            Event::RemoveRoot(path) => {
                self.blocks_panel.library.remove(&path);
                self.say(Level::Info, format!("Bloklar: “{}” kitaplığı listeden çıkarıldı; dosyalarına dokunulmadı.", name_of(&path)));
            }
            Event::Go(folder) => {
                self.blocks_panel.library.at = folder;
                self.blocks_panel.library.open = None;
            }
            Event::Open(path) => {
                self.blocks_panel.library.open = Some(path.clone());
                return self.library_read(path);
            }
            Event::Read(path, unit, done) => {
                let lib = &mut self.blocks_panel.library;
                // A reading for another unit than the one chosen now is late: dropped.
                if lib.units.get(&path).copied() != unit {
                    return Task::none();
                }
                let reading = match done {
                    Ok(f) => Reading::Done(f),
                    Err(why) => Reading::Failed(why),
                };
                lib.read.insert(path, reading);
            }
            Event::Unit(path, unit) => {
                self.blocks_panel.library.units.insert(path.clone(), unit);
                return self.library_read(path);
            }
            Event::Place(path, block) => return self.library_place(&path, block),
        }
        Task::none()
    }

    /// Reads a drawing off the drawing's thread in the unit chosen for it.
    fn library_read(&mut self, path: PathBuf) -> Task<Message> {
        let lib = &mut self.blocks_panel.library;
        let unit = lib.units.get(&path).copied();
        lib.read.insert(path.clone(), Reading::Busy);
        crate::db_connections::off_thread(
            {
                let path = path.clone();
                move || read_file(&path, unit).map(Arc::new)
            },
            move |done| msg(Event::Read(path, unit, done)),
        )
    }

    /// Yerleştir (docs/adr/0726 §1): the block (or the drawing) taken in with the library's category, then Blok ekle.
    fn library_place(&mut self, path: &Path, block: Option<String>) -> Task<Message> {
        let lib = &self.blocks_panel.library;
        let category = lib.category(path, block.is_none());
        let source_unit = lib.units.get(path).copied();
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = ImportBlock {
            path: path.display().to_string(),
            mode: if block.is_some() {
                BlockFileMode::Blocks
            } else {
                BlockFileMode::Drawing
            },
            name: None,
            names: block.clone().map(|n| vec![n]),
            replace: None,
            source_unit: source_unit.filter(|_| is_cad(path)),
            category,
            expected_revision: None,
        };
        let (output, warnings) =
            match file_import_block::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                CommandResult::Completed { output, warnings } => (output, warnings),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    self.say(Level::Warn, format!("Bloklar: {}", error.message));
                    return Task::none();
                }
                _ => return Task::none(),
            };
        let id = match (&block, output.block) {
            (_, Some(id)) => Some(id),
            (Some(name), None) => doc.model.block_named(name).map(|b| b.id),
            (None, None) => None,
        };
        let Some(id) = id else {
            return Task::none();
        };
        let name = doc.model.block(id).map(|b| b.name.clone()).unwrap_or_default();
        let left: Vec<String> = warnings.into_iter().map(|w| w.message).collect();
        let said = if output.skipped.is_empty() {
            format!(
                "“{name}” kitaplıktan alındı ({} blok){}. Yerleştirme noktasına tıklayın.",
                output.added.len() + output.changed.len(),
                if left.is_empty() { String::new() } else { format!("; {}", left.join(" ").trim_end_matches('.')) }
            )
        } else {
            format!(
                "Çizimdeki “{name}” kullanılıyor; kitaplıktaki alınmadı (Başka çizimden yeniden tanımla… ile değiştirilir). Yerleştirme noktasına tıklayın."
            )
        };
        self.say(Level::Success, said);
        self.memory.block_insert = Some(id);
        self.start_tool(kentos_interaction::block_insert::ID)
    }

    /// The Kitaplık view: the search, the place shown, and the libraries, folders, drawings or blocks.
    pub(crate) fn blocks_library_view(&self) -> Element<'_, Message> {
        let lib = &self.blocks_panel.library;
        let search = SearchBox::new(lib.query.clone(), "Kitaplıkta ara", |t| msg(Event::Search(t)))
            .fill()
            .height(28.0);
        let add = tip(
            button(icon(Icon::Plus).size(14.0))
                .on_press(msg(Event::AddRoot))
                .padding(6)
                .style(style::button::ghost),
            Tip::new("Kitaplık ekle…").body("DWG, DXF ya da KCAD çizimlerinin bulunduğu bir klasörü kitaplık yapar."),
            tooltip::Position::Bottom,
        );
        let bar = container(row![search, add].spacing(2).align_y(Center))
            .padding([6, 8])
            .width(Fill);
        let crumbs = self.library_crumbs();
        let body: Element<'_, Message> = match (&lib.open, &lib.at) {
            (Some(file), _) => self.library_file_view(file),
            (None, Some(folder)) => self.library_folder_view(folder),
            (None, None) => self.library_roots_view(),
        };
        column![bar, crumbs, body].into()
    }

    /// Kitaplıklar › Mobilya › Yatak odası › yatak-seti.dwg: each part goes back to it.
    fn library_crumbs(&self) -> Element<'_, Message> {
        let lib = &self.blocks_panel.library;
        let part = |words: String, to: Option<PathBuf>, strong: bool| -> Element<'_, Message> {
            let words = if strong {
                label::strong(words)
            } else {
                label::caption(words)
            };
            button(words.wrapping(text::Wrapping::None))
                .on_press(msg(Event::Go(to)))
                .padding([2, 4])
                .style(style::button::ghost)
                .into()
        };
        let mut parts: Vec<Element<'_, Message>> = vec![part("Kitaplıklar".into(), None, lib.at.is_none())];
        if let Some(at) = &lib.at
            && let Some(root) = lib.root_of(at)
        {
            let mut here = root.clone();
            parts.push(icon(Icon::ChevronRight).size(10.0).into());
            parts.push(part(name_of(root), Some(here.clone()), lib.open.is_none() && at == root));
            if let Ok(rest) = at.strip_prefix(root) {
                for c in rest.components() {
                    here = here.join(c);
                    parts.push(icon(Icon::ChevronRight).size(10.0).into());
                    parts.push(part(name_of(&here), Some(here.clone()), lib.open.is_none() && &here == at));
                }
            }
        }
        if let Some(file) = &lib.open {
            parts.push(icon(Icon::ChevronRight).size(10.0).into());
            parts.push(label::strong(name_of(file)).wrapping(text::Wrapping::None).into());
        }
        container(row(parts).spacing(2).align_y(Center))
            .padding([0, 8])
            .width(Fill)
            .clip(true)
            .into()
    }

    fn library_line<'a>(
        &'a self,
        glyph: Element<'a, Message>,
        words: String,
        meta: Option<String>,
        press: Message,
        tip_words: Option<(String, String)>,
    ) -> Element<'a, Message> {
        let mut names = Column::new().spacing(1).width(Fill).push(
            text(words)
                .font(typography::ui())
                .size(typography::body())
                .wrapping(text::Wrapping::None),
        );
        if let Some(meta) = meta {
            names = names.push(label::caption(meta).wrapping(text::Wrapping::None).style(|t: &Theme| {
                text::Style {
                    color: Some(Tokens::of(t).faint),
                }
            }));
        }
        let face = button(row![glyph, names].spacing(8).padding([0, 8]).align_y(Center).height(Fill))
            .on_press(press)
            .padding(0)
            .width(Fill)
            .height(Fill)
            .style(style::button::row(false));
        match tip_words {
            Some((title, body)) => tip(face, Tip::new(title).body(body), tooltip::Position::Left),
            None => face.into(),
        }
    }

    fn library_roots_view(&self) -> Element<'_, Message> {
        let lib = &self.blocks_panel.library;
        if lib.roots.is_empty() {
            return container(label::caption(
                "Kitaplık yok. + ile DWG, DXF ya da KCAD blok çizimlerinin bulunduğu bir klasörü ekleyin; alt klasörler kategori olur.",
            ))
            .padding([8, 12])
            .width(Fill)
            .into();
        }
        let query = name_key(lib.query.trim());
        let roots: Vec<PathBuf> = lib
            .roots
            .iter()
            .filter(|r| query.is_empty() || name_key(&name_of(r)).contains(&query))
            .cloned()
            .collect();
        VirtualList::new(roots.len(), typography::scaled(ROW), move |i| {
            let root = roots[i].clone();
            let line = self.library_line(
                icon(Icon::Folder).size(15.0).into(),
                name_of(&root),
                Some(root.display().to_string()),
                msg(Event::Go(Some(root.clone()))),
                None,
            );
            let remove = root.clone();
            ContextMenu::new(line, move |_| {
                Menu::new().item("Listeden çıkar", msg(Event::RemoveRoot(remove.clone())))
            })
            .into()
        })
        .height(Fill)
        .into()
    }

    fn library_folder_view(&self, folder: &Path) -> Element<'_, Message> {
        let lib = &self.blocks_panel.library;
        let (dirs, files) = listing(folder);
        let query = name_key(lib.query.trim());
        let keep = |n: &String| query.is_empty() || name_key(n).contains(&query);
        let lines: Vec<(bool, PathBuf)> = dirs
            .into_iter()
            .filter(keep)
            .map(|d| (true, folder.join(d)))
            .chain(files.into_iter().filter(keep).map(|f| (false, folder.join(f))))
            .collect();
        if lines.is_empty() {
            return container(label::caption(if query.is_empty() {
                "Bu klasörde DWG, DXF ya da KCAD çizimi yok."
            } else {
                "Aramayla eşleşen klasör ya da çizim yok."
            }))
            .padding([8, 12])
            .width(Fill)
            .into();
        }
        VirtualList::new(lines.len(), typography::scaled(ROW), move |i| {
            let (dir, path) = &lines[i];
            if *dir {
                self.library_line(
                    icon(Icon::Folder).size(15.0).into(),
                    name_of(path),
                    None,
                    msg(Event::Go(Some(path.clone()))),
                    None,
                )
            } else {
                let kind = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_uppercase())
                    .unwrap_or_default();
                self.library_line(
                    icon(crate::icons::from_web(Some("importDxf"))).size(15.0).into(),
                    name_of(path),
                    Some(format!("{kind} çizimi")),
                    msg(Event::Open(path.clone())),
                    None,
                )
            }
        })
        .height(Fill)
        .into()
    }

    fn library_file_view(&self, file: &Path) -> Element<'_, Message> {
        let lib = &self.blocks_panel.library;
        let format = self.format();
        let reading = lib.read.get(file).cloned().unwrap_or(Reading::Busy);
        let read = match reading {
            Reading::Busy => {
                return container(label::caption("Çizim okunuyor…")).padding([8, 12]).width(Fill).into();
            }
            Reading::Failed(why) => {
                return container(label::caption(why)).padding([8, 12]).width(Fill).into();
            }
            Reading::Done(f) => f,
        };
        let mut head = Column::new().spacing(4).padding([4, 8]);
        // A file naming no unit: its unit chosen before any size is said (docs/adr/0726 §2).
        if read.unit.is_none() || lib.units.contains_key(file) {
            let chosen = lib.units.get(file).copied();
            let mut units = row![].spacing(2).align_y(Center);
            for (u, words) in UNITS {
                units = units.push(
                    button(label::caption(words))
                        .on_press(msg(Event::Unit(file.to_path_buf(), u)))
                        .padding([2, 8])
                        .style(style::button::toggle(chosen == Some(u))),
                );
            }
            let say = if chosen.is_none() {
                "Bu dosya birimini bildirmiyor; blokların boyu birimi seçilince söylenir:"
            } else {
                "Dosyanın birimi (seçilen):"
            };
            head = head.push(label::caption(say).wrapping(text::Wrapping::WordOrGlyph)).push(units);
        }
        let file_path = file.to_path_buf();
        let query = name_key(lib.query.trim());
        let mut items: Vec<(Option<String>, Item)> = Vec::new();
        if let Some(whole) = &read.whole
            && (query.is_empty() || name_key(&whole.name).contains(&query))
        {
            items.push((None, whole.clone()));
        }
        items.extend(
            read.blocks
                .iter()
                .filter(|b| query.is_empty() || name_key(&b.name).contains(&query))
                .map(|b| (Some(b.name.clone()), b.clone())),
        );
        let can_place = read.unit.is_some();
        let list = VirtualList::new(items.len(), typography::scaled(ROW), move |i| {
            let (block, item) = &items[i];
            let side = typography::scaled(THUMB);
            let picture: Element<'_, Message> = if item.paths.is_empty() {
                container(icon(crate::icons::from_web(Some("importDxf"))).size(18.0))
                    .center(side)
                    .style(move |theme: &Theme| thumb_tile(theme, false))
                    .into()
            } else {
                container(Canvas::new(Thumb { paths: item.paths.clone() }).width(side).height(side))
                    .style(move |theme: &Theme| thumb_tile(theme, false))
                    .into()
            };
            let size = item.size.map(|(w, d)| {
                format!("{} × {} {}", format.length_bare(w), format.length_bare(d), format.length_unit_label())
            });
            let meta = match (block, size) {
                (None, Some(s)) => format!("Dosyanın tamamı · {s}"),
                (None, None) => "Dosyanın tamamı".to_owned(),
                (Some(_), Some(s)) => format!("{s} · {}", kentos_interaction::block_insert::unit_words(item.units)),
                (Some(_), None) => "Boyu birim seçilince".to_owned(),
            };
            let name = item.name.clone();
            let words = column![
                text(name.clone()).font(typography::ui()).size(typography::body()).wrapping(text::Wrapping::None),
                label::caption(meta).wrapping(text::Wrapping::None).style(|t: &Theme| text::Style {
                    color: Some(Tokens::of(t).faint),
                }),
            ]
            .spacing(1)
            .width(Fill);
            let place = msg(Event::Place(file_path.clone(), block.clone()));
            let face = button(row![picture, words].spacing(8).padding([0, 8]).align_y(Center).height(Fill))
                .on_press_maybe(can_place.then_some(place.clone()))
                .padding(0)
                .width(Fill)
                .height(Fill)
                .style(style::button::row(false));
            let face = tip(
                face,
                Tip::new(name).body(if can_place {
                    "Basın: çizime alınır ve Blok ekle başlar; kategorisi kitaplığın klasörleri.".to_owned()
                } else {
                    "Önce dosyanın birimini seçin.".to_owned()
                }),
                tooltip::Position::Left,
            );
            if can_place {
                ContextMenu::new(face, move |_| Menu::new().item("Yerleştir", place.clone())).into()
            } else {
                face
            }
        })
        .height(Fill);
        column![head, list].into()
    }
}
