//! Stil yöneticisi on the desktop (docs/STYLE.md §5 and §7, docs/adr/0092;
//! the web's `ui/style/StyleManager.ts`, `managerDetails.ts`,
//! `styleFiles.ts`): the style library as a tree of sources and categories
//! on the left, the items of the chosen category (or of a search) as
//! pictures in the middle, the chosen one's details and actions on the
//! right. System items are read-only and copyable; the user's (Kitaplığım,
//! kept in `kitaplik.kstil`) and the project's (kept in the drawing) can be
//! edited, moved and deleted. The same window picks a symbol for a slot of
//! Katman stili or for the selected objects (`Pick`).
//!
//! Files travel as .kstil (read, checked and cleaned by
//! `kentos_native_style::file`, held to `fixtures/style/v1/kstil.json`), or
//! as text on the system clipboard; PNG and JPEG pictures come in as images.
//!
//! This module holds the window's state; `update.rs` what its events do,
//! `assign.rs` giving symbols to objects and slots, `files.rs` files and the
//! clipboard, `view.rs` and `details.rs` how it looks.

mod assign;
pub(crate) mod details;
pub(crate) mod files;
#[cfg(test)]
mod tests;
mod update;
mod view;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::widget::text_editor;
use kentos_native_style::file::{ConflictMode, StyleFile};
use kentos_native_style::library::{CategoryNode, ItemKind, Source, StyleLibrary, TreeFilter};
use kentos_native_style::preview::Geometry;
use kentos_native_style::renderer::GeometryClass;
use serde_json::Value;

use crate::app::Message;
use crate::style::layer_style::SetAt;

/// Two presses on one card closer than this are a double click.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// The search field, focused when the window opens (the web's).
pub const SEARCH: &str = "kentos-style-manager-search";

/// The sources in the tree's order.
pub const SOURCES: [Source; 3] = [Source::System, Source::User, Source::Project];

/// What the middle shows (`KindFilter`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KindFilter {
    All,
    Fill,
    Line,
    Marker,
    Asset,
    /// Object templates (docs/adr/0176).
    Template,
}

impl std::fmt::Display for KindFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KindFilter::All => "Tümü",
            KindFilter::Fill => "Alan",
            KindFilter::Line => "Çizgi",
            KindFilter::Marker => "İşaret",
            KindFilter::Asset => "Çizim",
            KindFilter::Template => "Şablon",
        })
    }
}

pub const KINDS: [KindFilter; 6] = [
    KindFilter::All,
    KindFilter::Fill,
    KindFilter::Line,
    KindFilter::Marker,
    KindFilter::Asset,
    KindFilter::Template,
];

impl KindFilter {
    /// The kind a symbol for this geometry class is.
    pub fn of_class(class: GeometryClass) -> KindFilter {
        match class {
            GeometryClass::Fill => KindFilter::Fill,
            GeometryClass::Line => KindFilter::Line,
            GeometryClass::Marker => KindFilter::Marker,
        }
    }

    /// Whether an item shows under this kind.
    fn takes(self, kind: ItemKind, symbol: Option<&Value>) -> bool {
        let of = kentos_native_style::file::kind_of(kind, symbol);
        match self {
            KindFilter::All => true,
            KindFilter::Fill => of == "fill",
            KindFilter::Line => of == "line",
            KindFilter::Marker => of == "marker",
            KindFilter::Asset => of == "asset",
            KindFilter::Template => of == "template",
        }
    }
}

/// What a picked symbol is for.
#[derive(Clone, Debug, PartialEq)]
pub enum PickTarget {
    /// A slot of the open Katman stili window.
    Slot(SetAt, GeometryClass),
    /// The selected objects (`style.assign`).
    Selection,
}

/// The window in pick mode (`PickOptions`).
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    pub kind: Option<KindFilter>,
    pub title: String,
    pub current: Option<String>,
    pub target: PickTarget,
}

/// A .kstil file being taken in: what it holds, where it goes, what an id the library has does.
#[derive(Clone, Debug)]
pub struct ImportDraft {
    pub name: String,
    pub file: StyleFile,
    pub to: Source,
    pub mode: ConflictMode,
}

/// A detail field of the chosen item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    Name,
    Path,
    Reference,
    Description,
    Tags,
}

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    Close,
    Search(String),
    Kind(KindFilter),
    /// A tree node opened or closed, by its key.
    Toggle(String),
    /// A source or a category chosen, and whether it has sub-categories: a
    /// press opens it, a second press on the chosen one closes it (the web's
    /// `clickToggles`).
    Go(Source, Vec<String>, bool),
    /// A card pressed (twice quickly: chosen, or opened to edit).
    Press(String),
    /// Seç (pick mode).
    Choose,
    Field(Field, String),
    Commit(Field),
    /// An edit in the Açıklama box (several lines, as on the web).
    Describe(text_editor::Action),
    /// The preview's sample for a symbol type.
    Geometry(&'static str, Geometry),
    Copy(String, Source),
    /// Dışa aktar: these items to a file named after `name`, or to the clipboard.
    Export(Vec<String>, String),
    ExportListed,
    ExportClipboard,
    Written(Option<Result<String, String>>),
    Delete(String),
    DeleteConfirmed,
    DeleteCancelled,
    /// Seçili nesnelere uygula.
    Apply(String),
    /// Düzenle: a symbol opens in Sembol tasarımcısı (a system one's copy in Kitaplığım).
    Edit(String),
    /// Yeni sembol: a new symbol of a kind (fill, line, marker) in the designer.
    NewSymbol(&'static str),
    /// SVG çizimi (düzenleyicide)…: a new drawing in the SVG editor.
    NewDrawing,
    ImportFile,
    ImportClipboard,
    Picked(Option<(String, Vec<u8>)>),
    Pasted(Option<String>),
    ImportTo(Source),
    ImportMode(ConflictMode),
    ImportRun,
    ImportCancel,
    NewCategory(Source, Vec<String>),
    Rename(Source, Vec<String>),
    RenameInput(String),
    RenameDone,
    RenameCancel,
}

/// The Stil yöneticisi window.
pub struct Manager {
    pub pick: Option<Pick>,
    pub expanded: BTreeSet<String>,
    /// The source and category the middle shows (without a search).
    pub at: (Source, Vec<String>),
    pub query: String,
    pub kind: KindFilter,
    pub selected: Option<String>,
    said: Option<(String, bool)>,
    pub import: Option<ImportDraft>,
    /// A category being renamed in the tree: where, and the name as typed.
    pub renaming: Option<(Source, Vec<String>, String)>,
    /// The chosen item's fields as typed, until Enter or another item.
    pub typed: BTreeMap<Field, String>,
    /// The preview's sample per symbol type, kept while the window is open.
    pub geometry: HashMap<&'static str, Geometry>,
    /// The item Sil asks about.
    pub deleting: Option<String>,
    /// The chosen item's Açıklama box: whose it is, and its text.
    pub notes: Option<(String, text_editor::Content)>,
    last_press: Option<(String, Instant)>,
    cache: RefCell<Cache>,
}

/// What the view reads of the library, built again only when the library
/// changed: the three sources' trees, and the items listed last.
#[derive(Default)]
struct Cache {
    version: Option<u64>,
    trees: Trees,
    listed: Option<(ListKey, Listed)>,
}

/// Each source with its category tree, in the tree's order.
pub type Trees = Arc<Vec<(Source, Vec<CategoryNode>)>>;

/// The listed items' ids and sources.
pub type Listed = Arc<Vec<(String, Source)>>;

/// What the listed items depend on besides the library.
type ListKey = (Source, Vec<String>, String, KindFilter);

/// A tree node's key: a source, or a category under it.
pub fn node_key(source: Source, path: &[String]) -> String {
    if path.is_empty() {
        format!("s:{}", source.key())
    } else {
        format!("c:{}:{}", source.key(), path.join("\u{1}"))
    }
}

fn flatten(c: &CategoryNode) -> Vec<String> {
    c.all_items()
        .iter()
        .map(|(i, _)| i.id().to_owned())
        .collect()
}

impl Manager {
    pub fn new(lib: &StyleLibrary, pick: Option<Pick>, select: Option<String>) -> Manager {
        let mut m = Manager {
            kind: pick
                .as_ref()
                .and_then(|p| p.kind)
                .unwrap_or(KindFilter::All),
            pick,
            expanded: SOURCES.into_iter().map(|s| node_key(s, &[])).collect(),
            at: (Source::System, Vec::new()),
            query: String::new(),
            selected: None,
            said: None,
            import: None,
            renaming: None,
            typed: BTreeMap::new(),
            geometry: HashMap::new(),
            deleting: None,
            notes: None,
            last_press: None,
            cache: RefCell::new(Cache::default()),
        };
        let start = select.or_else(|| m.pick.as_ref().and_then(|p| p.current.clone()));
        if let Some((item, source)) = start.as_deref().and_then(|id| lib.get(id)) {
            let path: Vec<String> = item.path().into_iter().map(str::to_owned).collect();
            for i in 1..=path.len() {
                m.expanded.insert(node_key(source, &path[..i]));
            }
            m.at = (source, path);
            m.choose_item(lib, Some(item.id().to_owned()));
        } else if !lib.items(Some(Source::User)).is_empty() {
            m.at = (Source::User, Vec::new());
        }
        m
    }

    pub fn say(&mut self, text: impl Into<String>, warn: bool) {
        self.said = Some((text.into(), warn));
    }

    pub fn said(&self) -> Option<&(String, bool)> {
        self.said.as_ref()
    }

    /// Makes `id` the chosen item: its fields start from the library again.
    pub fn choose_item(&mut self, lib: &StyleLibrary, id: Option<String>) {
        self.typed.clear();
        self.notes = id.as_deref().and_then(|id| lib.get(id)).map(|(item, _)| {
            let text = item.text("description").unwrap_or("");
            (item.id().to_owned(), text_editor::Content::with_text(text))
        });
        self.selected = id;
    }

    /// The three sources' category trees, built again only when the library changed.
    pub fn trees(&self, lib: &StyleLibrary) -> Trees {
        let mut cache = self.cache.borrow_mut();
        if cache.version != Some(lib.version()) {
            cache.version = Some(lib.version());
            cache.trees = Arc::new(
                SOURCES
                    .into_iter()
                    .map(|source| {
                        let tree = lib.tree(TreeFilter {
                            source: Some(source),
                            ..TreeFilter::default()
                        });
                        (source, tree)
                    })
                    .collect(),
            );
            cache.listed = None;
        }
        cache.trees.clone()
    }

    /// The items the middle shows: a search across the library, or everything
    /// under the chosen source or category; of the chosen kind (`listItems`).
    pub fn listed(&self, lib: &StyleLibrary) -> Listed {
        let trees = self.trees(lib);
        let query = kentos_processing::text::js_trim(&self.query).to_owned();
        let key: ListKey = (self.at.0, self.at.1.clone(), query.clone(), self.kind);
        if let Some((k, list)) = &self.cache.borrow().listed
            && *k == key
        {
            return list.clone();
        }
        let mut items: Vec<(String, Source)> = Vec::new();
        let mut take = |c: &CategoryNode| {
            for (item, source) in c.all_items() {
                if self.kind.takes(item.kind(), item.symbol()) {
                    items.push((item.id().to_owned(), *source));
                }
            }
        };
        if query.is_empty() {
            let tree = trees
                .iter()
                .find(|(s, _)| *s == self.at.0)
                .map_or(&[][..], |(_, t)| t.as_slice());
            match find_node(tree, &self.at.1) {
                Some(node) => take(node),
                None if self.at.1.is_empty() => tree.iter().for_each(&mut take),
                None => {}
            }
        } else {
            let found = lib.tree(TreeFilter {
                query: Some(&query),
                ..TreeFilter::default()
            });
            found.iter().for_each(&mut take);
        }
        let list = Arc::new(items);
        self.cache.borrow_mut().listed = Some((key, list.clone()));
        list
    }

    /// Whether the pick mode may choose an item: a symbol (or an SVG drawing
    /// when it picks one) (`pickable`).
    pub fn pickable(&self, lib: &StyleLibrary, id: &str) -> bool {
        let Some((item, _)) = lib.get(id) else {
            return false;
        };
        match self.pick.as_ref().and_then(|p| p.kind) {
            Some(KindFilter::Asset) => {
                item.kind() == ItemKind::Asset && item.format() == Some("svg")
            }
            _ => item.kind() == ItemKind::Symbol,
        }
    }
}

/// The category at `path` of a tree.
pub fn find_node<'a>(tree: &'a [CategoryNode], path: &[String]) -> Option<&'a CategoryNode> {
    let mut list = tree;
    let mut node = None;
    for part in path {
        let found = list.iter().find(|c| &c.name == part)?;
        list = &found.children;
        node = Some(found);
    }
    node
}

fn ev(e: Event) -> Message {
    Message::StyleManager(Box::new(e))
}

/// The ids under a node of the tree (a source or a category), for Dışa aktar.
pub fn ids_under(lib: &StyleLibrary, source: Source, path: &[String]) -> Vec<String> {
    let tree = lib.tree(TreeFilter {
        source: Some(source),
        ..TreeFilter::default()
    });
    if path.is_empty() {
        return tree.iter().flat_map(flatten).collect();
    }
    find_node(&tree, path).map(flatten).unwrap_or_default()
}
