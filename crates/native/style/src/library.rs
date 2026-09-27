//! The style library (the web's `style/library.ts`): symbols and assets
//! from three sources. System items ship with KentOS and can be copied but
//! never changed or deleted; the user's items follow the user across
//! projects; project items travel in the project file, so everyone who opens
//! it sees the same symbols. Items sit in a category tree of any depth
//! (“MPYY › Uygulama İmar Planı › Sınırlar”).
//!
//! Items are kept as the JSON they are saved as, so fields this side does
//! not read (notes, tags, what a later version adds) survive a round trip.
//! A symbol is the style core's JSON; the core reads it when a layer is built.

use std::collections::HashMap;

use serde_json::Value;

/// Where an item lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    System,
    User,
    Project,
}

impl Source {
    /// The name the interface gives the source.
    pub fn label(self) -> &'static str {
        match self {
            Source::System => "Sistem",
            Source::User => "Kitaplığım",
            Source::Project => "Proje",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Symbol,
    Asset,
}

/// A library item: a symbol or an asset (an SVG drawing or a raster image).
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    id: String,
    kind: ItemKind,
    value: Value,
}

impl Item {
    /// An item from its saved JSON; None when it has no id or no known kind.
    pub fn from_value(value: Value) -> Option<Item> {
        let id = value.get("id")?.as_str()?.to_owned();
        let kind = match value.get("kind")?.as_str()? {
            "symbol" => ItemKind::Symbol,
            "asset" => ItemKind::Asset,
            _ => return None,
        };
        Some(Item { id, kind, value })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> ItemKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        self.value.get("name").and_then(Value::as_str).unwrap_or("")
    }

    /// The category path.
    pub fn path(&self) -> Vec<&str> {
        self.value
            .get("path")
            .and_then(Value::as_array)
            .map(|p| p.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// The symbol (the style core's JSON), for a symbol item.
    pub fn symbol(&self) -> Option<&Value> {
        match self.kind {
            ItemKind::Symbol => self.value.get("symbol"),
            ItemKind::Asset => None,
        }
    }

    /// `svg`, `png` or `jpeg`, for an asset.
    pub fn format(&self) -> Option<&str> {
        self.value.get("format").and_then(Value::as_str)
    }

    /// An asset's data: the SVG text, or a `data:` address of the image.
    pub fn data(&self) -> Option<&str> {
        self.value.get("data").and_then(Value::as_str)
    }

    /// An asset's width and height (from its viewBox, or its pixels).
    pub fn size(&self) -> Option<(f64, f64)> {
        let w = self.value.get("width")?.as_f64()?;
        let h = self.value.get("height")?.as_f64()?;
        Some((w, h))
    }

    /// The item as it is saved.
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// The library: the three sources' items and their categories.
#[derive(Clone, Debug, Default)]
pub struct StyleLibrary {
    system: HashMap<String, Item>,
    user: HashMap<String, Item>,
    project: HashMap<String, Item>,
    system_categories: Vec<Value>,
    user_categories: Vec<Value>,
    project_categories: Vec<Value>,
    /// Bumped on every change, so a drawing built with the library knows it is stale.
    version: u64,
}

impl StyleLibrary {
    /// A library with `items` and `categories` as its system part.
    pub fn with_system(items: Vec<Item>, categories: Vec<Value>) -> StyleLibrary {
        StyleLibrary {
            system: items.into_iter().map(|i| (i.id.clone(), i)).collect(),
            system_categories: categories,
            ..StyleLibrary::default()
        }
    }

    fn store(&self, source: Source) -> &HashMap<String, Item> {
        match source {
            Source::System => &self.system,
            Source::User => &self.user,
            Source::Project => &self.project,
        }
    }

    /// Replaces a source's content (loading the user's library, opening a
    /// project). Items that shadow a system id are left out, as on the web.
    pub fn load(&mut self, source: Source, items: &[Value], categories: &[Value]) {
        let items: HashMap<String, Item> = items
            .iter()
            .cloned()
            .filter_map(Item::from_value)
            .filter(|i| !self.system.contains_key(&i.id))
            .map(|i| (i.id.clone(), i))
            .collect();
        match source {
            Source::System => {}
            Source::User => {
                self.user = items;
                self.user_categories = categories.to_vec();
            }
            Source::Project => {
                self.project = items;
                self.project_categories = categories.to_vec();
            }
        }
        self.version += 1;
    }

    /// A source's items and categories as they are saved.
    pub fn dump(&self, source: Source) -> (Vec<Value>, Vec<Value>) {
        let mut items: Vec<&Item> = self.store(source).values().collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        let categories = match source {
            Source::System => &self.system_categories,
            Source::User => &self.user_categories,
            Source::Project => &self.project_categories,
        };
        (
            items.into_iter().map(|i| i.value.clone()).collect(),
            categories.clone(),
        )
    }

    /// An item by id: the project's first, then the user's, then the system's.
    pub fn get(&self, id: &str) -> Option<(&Item, Source)> {
        [Source::Project, Source::User, Source::System]
            .into_iter()
            .find_map(|s| self.store(s).get(id).map(|i| (i, s)))
    }

    pub fn symbol(&self, id: &str) -> Option<&Value> {
        self.get(id).and_then(|(i, _)| i.symbol())
    }

    pub fn asset(&self, id: &str) -> Option<&Item> {
        self.get(id)
            .map(|(i, _)| i)
            .filter(|i| i.kind == ItemKind::Asset)
    }

    /// Every item of a source (all sources when None).
    pub fn items(&self, source: Option<Source>) -> Vec<(&Item, Source)> {
        let sources = match source {
            Some(s) => vec![s],
            None => vec![Source::System, Source::User, Source::Project],
        };
        sources
            .into_iter()
            .flat_map(|s| self.store(s).values().map(move |i| (i, s)))
            .collect()
    }

    /// Changes with every change to the library.
    pub fn version(&self) -> u64 {
        self.version
    }
}
