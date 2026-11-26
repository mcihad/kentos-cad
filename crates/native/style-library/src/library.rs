//! The style library (the web's `style/library.ts`): symbols, assets and
//! object templates (docs/adr/0176) from three sources. System items ship with KentOS and can be copied but
//! never changed or deleted; the user's items follow the user across
//! projects; project items travel in the project file, so everyone who opens
//! it sees the same symbols. Items sit in a category tree of any depth
//! (“MPYY › Uygulama İmar Planı › Sınırlar”).
//!
//! Items are kept as the JSON they are saved as, so fields this side does
//! not read (notes, tags, what a later version adds) survive a round trip,
//! and in the order they came, as the web's maps keep them. A symbol is the
//! style core's JSON; the core reads it when a layer is built. The library
//! is data only: the app keeps the user's part on disk and the project's in
//! the drawing when an edit says which changed.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use kentos_style_core::js::collate::compare_tr;
use kentos_style_core::js::text::fold_turkish;
use serde_json::{Map, Value, json};

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

    /// Its name in the JSON (`LibrarySource`).
    pub fn key(self) -> &'static str {
        match self {
            Source::System => "system",
            Source::User => "user",
            Source::Project => "project",
        }
    }

    /// Whether its items may be changed (the user's and the project's).
    pub fn editable(self) -> bool {
        self != Source::System
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Symbol,
    Asset,
    /// An object template (docs/adr/0176, [`crate::object_template`]).
    Template,
}

/// A library item: a symbol, an asset (an SVG drawing or a raster image) or an object template.
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
            "template" => ItemKind::Template,
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

    /// A text field of the item (`description`, `reference`), when it has one.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.value.get(key).and_then(Value::as_str)
    }

    /// Its tags.
    pub fn tags(&self) -> Vec<&str> {
        self.value
            .get("tags")
            .and_then(Value::as_array)
            .map(|t| t.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// The symbol (the style core's JSON), for a symbol item.
    pub fn symbol(&self) -> Option<&Value> {
        match self.kind {
            ItemKind::Symbol => self.value.get("symbol"),
            ItemKind::Asset | ItemKind::Template => None,
        }
    }

    /// The recipe of a template item (docs/adr/0176 §1).
    pub fn template(&self) -> Option<&Value> {
        match self.kind {
            ItemKind::Template => self.value.get("template"),
            ItemKind::Symbol | ItemKind::Asset => None,
        }
    }

    /// The library symbol a template draws with, by id.
    pub fn template_symbol(&self) -> Option<&str> {
        self.template()?.get("symbol")?.as_str()
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

/// A source's items in the order they came, with an index by id.
#[derive(Clone, Debug, Default)]
struct Store {
    items: Vec<Item>,
    index: HashMap<String, usize>,
}

impl Store {
    fn from_items(items: impl IntoIterator<Item = Item>) -> Store {
        let mut s = Store::default();
        for i in items {
            s.set(i);
        }
        s
    }

    fn get(&self, id: &str) -> Option<&Item> {
        self.index.get(id).map(|&k| &self.items[k])
    }

    fn contains(&self, id: &str) -> bool {
        self.index.contains_key(id)
    }

    /// Adds the item, or replaces the one of its id where it stands.
    fn set(&mut self, item: Item) {
        match self.index.get(&item.id) {
            Some(&k) => self.items[k] = item,
            None => {
                self.index.insert(item.id.clone(), self.items.len());
                self.items.push(item);
            }
        }
    }

    fn remove(&mut self, id: &str) -> Option<Item> {
        let k = self.index.remove(id)?;
        let item = self.items.remove(k);
        for (i, it) in self.items.iter().enumerate().skip(k) {
            self.index.insert(it.id.clone(), i);
        }
        Some(item)
    }
}

/// A category of the tree: its name, path, items (in Turkish order), sub-categories and description.
#[derive(Clone, Debug, PartialEq)]
pub struct CategoryNode {
    pub name: String,
    pub path: Vec<String>,
    pub items: Vec<(Item, Source)>,
    pub children: Vec<CategoryNode>,
    pub description: Option<String>,
}

impl CategoryNode {
    /// Every item under it, its sub-categories' included.
    pub fn all_items(&self) -> Vec<&(Item, Source)> {
        let mut out: Vec<&(Item, Source)> = self.items.iter().collect();
        for c in &self.children {
            out.extend(c.all_items());
        }
        out
    }
}

/// What a tree shows: one source or all, the items a search finds, one kind.
#[derive(Clone, Copy, Debug, Default)]
pub struct TreeFilter<'a> {
    pub source: Option<Source>,
    pub query: Option<&'a str>,
    pub kind: Option<ItemKind>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// A fresh id for a user or project item (the web's `newItemId`): the time
/// and six random letters in base 36, after the prefix (`u`, `p`, `a`).
pub fn new_item_id(prefix: &str) -> String {
    fn base36(mut n: u128, out: &mut String) {
        const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut buf = Vec::new();
        loop {
            buf.push(DIGITS[(n % 36) as usize]);
            n /= 36;
            if n == 0 {
                break;
            }
        }
        buf.reverse();
        out.push_str(std::str::from_utf8(&buf).unwrap_or("0"));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let mut out = format!("{prefix}-");
    base36(now.as_millis(), &mut out);
    // A splitmix64 step over the clock and a counter: six base-36 letters that do not repeat.
    let mut z = (now.as_nanos() as u64).wrapping_add(
        NEXT_ID
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15),
    );
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let mut tail = String::new();
    base36(u128::from(z), &mut tail);
    out.push_str(&format!("{tail:0>6}")[..6]);
    out
}

/// Asset ids a symbol draws with (SVG and raster markers, image fills), nested markers included (`assetsOfSymbol`).
pub fn assets_of_symbol(symbol: &Value) -> Vec<String> {
    fn walk(s: &Value, out: &mut Vec<String>) {
        for l in s
            .get("layers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(a) = l.get("asset").and_then(Value::as_str)
                && !out.iter().any(|x| x == a)
            {
                out.push(a.to_owned());
            }
            if let Some(m) = l.get("marker").filter(|m| m.is_object()) {
                walk(m, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(symbol, &mut out);
    out
}

/// The library items `v` refers to (`{"ref": id}` anywhere in it: a renderer's symbols, a symbol's markers), each once.
pub fn refs_in(v: &Value) -> Vec<String> {
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(o) => {
                if let Some(id) = o.get("ref").and_then(Value::as_str)
                    && !id.is_empty()
                    && !out.iter().any(|x| x == id)
                {
                    out.push(id.to_owned());
                }
                for x in o.values() {
                    walk(x, out);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(v, &mut out);
    out
}

fn path_key(p: &[String]) -> String {
    p.join("\u{0}")
}

fn path_of(v: &Value) -> Vec<String> {
    v.get("path")
        .and_then(Value::as_array)
        .map(|p| {
            p.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The library: the three sources' items and their categories.
#[derive(Clone, Debug, Default)]
pub struct StyleLibrary {
    system: Store,
    user: Store,
    project: Store,
    system_categories: Vec<Value>,
    user_categories: Vec<Value>,
    project_categories: Vec<Value>,
    /// Bumped on every change, so a drawing built with the library knows it is stale.
    version: u64,
    /// Bumped when an asset (a drawing, an image) comes, changes or goes: a
    /// symbol's picture depends on these alone besides its own JSON.
    assets_version: u64,
}

impl StyleLibrary {
    /// A library with `items` and `categories` as its system part.
    pub fn with_system(items: Vec<Item>, categories: Vec<Value>) -> StyleLibrary {
        StyleLibrary {
            system: Store::from_items(items),
            system_categories: categories,
            ..StyleLibrary::default()
        }
    }

    fn store(&self, source: Source) -> &Store {
        match source {
            Source::System => &self.system,
            Source::User => &self.user,
            Source::Project => &self.project,
        }
    }

    fn store_mut(&mut self, source: Source) -> &mut Store {
        match source {
            Source::System => &mut self.system,
            Source::User => &mut self.user,
            Source::Project => &mut self.project,
        }
    }

    fn categories(&self, source: Source) -> &Vec<Value> {
        match source {
            Source::System => &self.system_categories,
            Source::User => &self.user_categories,
            Source::Project => &self.project_categories,
        }
    }

    fn categories_mut(&mut self, source: Source) -> &mut Vec<Value> {
        match source {
            Source::System => &mut self.system_categories,
            Source::User => &mut self.user_categories,
            Source::Project => &mut self.project_categories,
        }
    }

    /// Replaces a source's content (loading the user's library, opening a
    /// project). Items that shadow a system id are left out, as on the web.
    pub fn load(&mut self, source: Source, items: &[Value], categories: &[Value]) {
        if source == Source::System {
            return;
        }
        let system = &self.system;
        let store = Store::from_items(
            items
                .iter()
                .cloned()
                .filter_map(Item::from_value)
                .filter(|i| !system.contains(&i.id)),
        );
        *self.store_mut(source) = store;
        *self.categories_mut(source) = categories.to_vec();
        self.version += 1;
        self.assets_version += 1;
    }

    /// A source's items and categories as they are saved, items in the order they came.
    pub fn dump(&self, source: Source) -> (Vec<Value>, Vec<Value>) {
        (
            self.store(source)
                .items
                .iter()
                .map(|i| i.value.clone())
                .collect(),
            self.categories(source).clone(),
        )
    }

    // ── Reading ──────────────────────────────────────────────────────

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

    /// Every item of a source (all sources when None), in the order they came.
    pub fn items(&self, source: Option<Source>) -> Vec<(&Item, Source)> {
        let sources = match source {
            Some(s) => vec![s],
            None => vec![Source::System, Source::User, Source::Project],
        };
        sources
            .into_iter()
            .flat_map(|s| self.store(s).items.iter().map(move |i| (i, s)))
            .collect()
    }

    /// Whether an item may be changed: the user's and the project's.
    pub fn can_edit(&self, id: &str) -> bool {
        self.get(id).is_some_and(|(_, s)| s.editable())
    }

    /// Symbols that draw with an asset (to warn before removing it) (`usersOf`).
    pub fn users_of(&self, asset: &str) -> Vec<(&Item, Source)> {
        self.items(None)
            .into_iter()
            .filter(|(i, _)| {
                i.symbol()
                    .is_some_and(|s| assets_of_symbol(s).iter().any(|a| a == asset))
            })
            .collect()
    }

    /// Changes with every change to the library.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Changes when an asset comes, changes or goes.
    pub fn assets_version(&self) -> u64 {
        self.assets_version
    }

    /// The category tree of the chosen items (all by default). Siblings are
    /// ordered by their category's `order`, then by name in Turkish order;
    /// a search leaves the empty categories out (`tree`).
    pub fn tree(&self, filter: TreeFilter<'_>) -> Vec<CategoryNode> {
        let words: Vec<String> = filter
            .query
            .map(|q| {
                fold_turkish(q)
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let items: Vec<(&Item, Source)> = self
            .items(filter.source)
            .into_iter()
            .filter(|(i, _)| {
                if filter.kind.is_some_and(|k| i.kind != k) {
                    return false;
                }
                if words.is_empty() {
                    return true;
                }
                let mut hay = vec![i.name().to_owned()];
                hay.extend(i.path().into_iter().map(str::to_owned));
                hay.extend(i.tags().into_iter().map(str::to_owned));
                if i.kind != ItemKind::Asset {
                    hay.push(i.text("description").unwrap_or("").to_owned());
                }
                let hay = fold_turkish(&hay.join(" "));
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .collect();
        let all_categories = || {
            self.system_categories
                .iter()
                .chain(&self.user_categories)
                .chain(&self.project_categories)
        };
        let meta: HashMap<String, &Value> = all_categories()
            .map(|c| (path_key(&path_of(c)), c))
            .collect();

        struct Build {
            name: String,
            path: Vec<String>,
            items: Vec<(Item, Source)>,
            children: Vec<Build>,
        }
        fn ensure<'b>(root: &'b mut Build, path: &[String]) -> &'b mut Build {
            let mut node = root;
            for i in 0..path.len() {
                let k = match node.children.iter().position(|c| c.name == path[i]) {
                    Some(k) => k,
                    None => {
                        node.children.push(Build {
                            name: path[i].clone(),
                            path: path[..=i].to_vec(),
                            items: Vec::new(),
                            children: Vec::new(),
                        });
                        node.children.len() - 1
                    }
                };
                node = &mut node.children[k];
            }
            node
        }
        let mut root = Build {
            name: String::new(),
            path: Vec::new(),
            items: Vec::new(),
            children: Vec::new(),
        };
        if words.is_empty() {
            let categories: Vec<&Value> = match filter.source {
                Some(s) => self.categories(s).iter().collect(),
                None => all_categories().collect(),
            };
            for c in categories {
                ensure(&mut root, &path_of(c));
            }
        }
        for (i, s) in items {
            let path: Vec<String> = i.path().into_iter().map(str::to_owned).collect();
            ensure(&mut root, &path).items.push((i.clone(), s));
        }
        let order = |p: &[String]| {
            meta.get(&path_key(p))
                .and_then(|c| c.get("order"))
                .and_then(Value::as_f64)
                .unwrap_or(1e9)
        };
        fn finish(
            b: Build,
            meta: &HashMap<String, &Value>,
            order: &dyn Fn(&[String]) -> f64,
        ) -> CategoryNode {
            let mut items = b.items;
            items.sort_by(|x, y| compare_tr(x.0.name(), y.0.name()));
            let mut children = b.children;
            children.sort_by(|x, y| {
                order(&x.path)
                    .total_cmp(&order(&y.path))
                    .then_with(|| compare_tr(&x.name, &y.name))
            });
            CategoryNode {
                description: meta
                    .get(&path_key(&b.path))
                    .and_then(|c| c.get("description"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                name: b.name,
                path: b.path,
                items,
                children: children
                    .into_iter()
                    .map(|c| finish(c, meta, order))
                    .collect(),
            }
        }
        finish(root, &meta, &order).children
    }

    // ── Changing ─────────────────────────────────────────────────────

    /// The source of an item that may change, or why it may not.
    fn editable(&self, id: &str) -> Result<Source, String> {
        match self.get(id) {
            None => Err(format!("Kitaplıkta yok: {id}")),
            Some((i, Source::System)) => Err(format!(
                "“{}” bir sistem öğesidir; değiştirilemez ya da silinemez. Kopyasını alıp onu düzenleyin.",
                i.name()
            )),
            Some((_, s)) => Ok(s),
        }
    }

    /// A change; `assets`: an asset came, changed or went.
    fn touched(&mut self, assets: bool) {
        self.version += 1;
        if assets {
            self.assets_version += 1;
        }
    }

    /// Adds an item to the user's or the project's library.
    pub fn add(&mut self, source: Source, item: Value) -> Result<Item, String> {
        if kentos_contracts::shape_resource::is_resource(&item) {
            kentos_contracts::shape_resource::read(&item)?;
        }
        let item = Item::from_value(item).ok_or("Öğenin kimliği ya da türü yok.")?;
        if !source.editable() {
            return Err("Sistem kitaplığına öğe eklenemez.".into());
        }
        if self.get(&item.id).is_some() {
            return Err(format!("Bu kimlikle bir öğe zaten var: {}", item.id));
        }
        let asset = item.kind == ItemKind::Asset;
        self.store_mut(source).set(item.clone());
        self.touched(asset);
        Ok(item)
    }

    /// Changes an item's content, name, path, tags … (never its id or kind).
    /// A field set to null is removed.
    pub fn update(&mut self, id: &str, patch: &Map<String, Value>) -> Result<Source, String> {
        let source = self.editable(id)?;
        let Some(item) = self.store(source).get(id).cloned() else {
            return Err(format!("Kitaplıkta yok: {id}"));
        };
        let mut value = item.value;
        if let Some(o) = value.as_object_mut() {
            for (k, v) in patch {
                if k == "id" || k == "kind" {
                    continue;
                }
                if v.is_null() {
                    o.remove(k);
                } else {
                    o.insert(k.clone(), v.clone());
                }
            }
        }
        if kentos_contracts::shape_resource::is_resource(&value) {
            kentos_contracts::shape_resource::read(&value)?;
        }
        if let Some(changed) = Item::from_value(value) {
            self.store_mut(source).set(changed);
        }
        self.touched(item.kind == ItemKind::Asset);
        Ok(source)
    }

    /// Renames an item; a blank name is “Adsız”.
    pub fn rename(&mut self, id: &str, name: &str) -> Result<Source, String> {
        let name = kentos_style_core::js::text::trim(name);
        let name = if name.is_empty() { "Adsız" } else { name };
        let mut patch = Map::new();
        patch.insert("name".into(), Value::from(name));
        self.update(id, &patch)
    }

    /// Moves an item to another category.
    pub fn move_to(&mut self, id: &str, path: &[String]) -> Result<Source, String> {
        let mut patch = Map::new();
        patch.insert("path".into(), json!(path));
        self.update(id, &patch)
    }

    /// Removes a user or project item. Symbols that use a removed asset keep
    /// their reference (and draw nothing for it).
    pub fn remove(&mut self, id: &str) -> Result<Source, String> {
        let source = self.editable(id)?;
        let kind = self.store_mut(source).remove(id).map(|i| i.kind);
        self.touched(kind == Some(ItemKind::Asset));
        Ok(source)
    }

    /// Copies any item (a system one included) into the user's or the
    /// project's library under a new id (`copy`). A symbol copied into a
    /// project takes along the user assets it draws with, and a template its user
    /// symbol with that symbol's assets, so the project stays complete for
    /// colleagues (system items are always there).
    pub fn copy(
        &mut self,
        id: &str,
        to: Source,
        name: Option<&str>,
        path: Option<&[String]>,
    ) -> Result<Item, String> {
        let Some((src, _)) = self.get(id) else {
            return Err(format!("Kitaplıkta yok: {id}"));
        };
        if !to.editable() {
            return Err("Sistem kitaplığına öğe eklenemez.".into());
        }
        if kentos_contracts::shape_resource::is_resource(src.value()) {
            if self.store(to).contains(id) {
                return Err(
                    "Bu SHX kaynağı hedef kitaplıkta zaten var; aynı dosya adı çoğaltılamaz."
                        .into(),
                );
            }
            let item = src.clone();
            self.store_mut(to).set(item.clone());
            self.touched(true);
            return Ok(item);
        }
        let mut value = src.value.clone();
        let new_id = new_item_id(if to == Source::Project { "p" } else { "u" });
        let name = name.map_or_else(|| format!("{} (kopya)", src.name()), str::to_owned);
        let path: Vec<String> = path.map_or_else(
            || src.path().into_iter().map(str::to_owned).collect(),
            <[String]>::to_vec,
        );
        if let Some(o) = value.as_object_mut() {
            o.insert("id".into(), Value::from(new_id));
            o.insert("name".into(), Value::from(name));
            o.insert("path".into(), json!(path));
        }
        let item = Item::from_value(value).ok_or("Öğe kopyalanamadı.")?;
        if to == Source::Project {
            // What the item draws with, taken along when it is the user's.
            let mut refs: Vec<String> = Vec::new();
            let template_symbol = item.template_symbol().map(str::to_owned);
            if let Some(s) = &template_symbol {
                refs.push(s.clone());
            }
            let symbol = item.symbol().cloned().or_else(|| {
                template_symbol
                    .as_deref()
                    .and_then(|s| self.symbol(s).cloned())
            });
            if let Some(symbol) = symbol {
                refs.extend(assets_of_symbol(&symbol));
            }
            for r in refs {
                if let Some((taken, Source::User)) = self.get(&r)
                    && !self.project.contains(&r)
                {
                    let taken = taken.clone();
                    self.project.set(taken);
                }
            }
        }
        let asset = item.kind == ItemKind::Asset;
        self.store_mut(to).set(item.clone());
        // A copied symbol may have brought its drawings into the project.
        self.touched(asset || to == Source::Project);
        Ok(item)
    }

    /// What `item` draws with besides itself: a template's symbol, a symbol's drawings and the items it refers to.
    fn needs_of(item: &Item) -> Vec<String> {
        let mut out: Vec<String> = item.template_symbol().map(str::to_owned).into_iter().collect();
        if let Some(symbol) = item.symbol() {
            out.extend(assets_of_symbol(symbol));
            out.extend(refs_in(symbol));
        }
        out
    }

    /// The id a project uses for the item `id` (docs/adr/9317): a user item's copy in the project, the one there already
    /// (of its kind, name and content) or a new one under its name and category with what it draws with, as a symbol
    /// from a style is applied as a copy; a system or project item's own id. An error when the library has no `id`.
    pub fn project_copy(&mut self, id: &str) -> Result<String, String> {
        let Some((item, source)) = self.get(id) else {
            return Err(format!("Kitaplıkta yok: {id}"));
        };
        if source != Source::User {
            return Ok(id.to_owned());
        }
        fn content(i: &Item) -> [Option<&Value>; 3] {
            [i.symbol(), i.template(), i.value.get("data")]
        }
        let (name, path, own) = (item.name().to_owned(), item.path().join("\u{0}"), content(item));
        let found = self.project.items.iter().find(|p| {
            p.kind == item.kind && p.name() == name && p.path().join("\u{0}") == path && content(p) == own
        });
        if let Some(found) = found {
            return Ok(found.id.clone());
        }
        let path: Vec<String> = item.path().into_iter().map(str::to_owned).collect();
        let made = self.copy(id, Source::Project, Some(&name), Some(&path))?;
        // What it refers to besides its drawings (a marker in a symbol), taken along too.
        self.take_along(Self::needs_of(&made));
        Ok(made.id)
    }

    /// Takes the user's items among `ids` that the project does not hold into the project under their own ids, with
    /// what they draw with (docs/adr/9317), as `copy` takes a copied symbol's drawings along: a drawing that refers to
    /// Kitaplığım then draws the same on paper, on another computer and in the cloud. A project item's own needs are
    /// followed too. The ids taken, in the order taken.
    pub fn take_along<I: IntoIterator<Item = String>>(&mut self, ids: I) -> Vec<String> {
        let mut queue: std::collections::VecDeque<String> = ids.into_iter().collect();
        let mut seen = std::collections::HashSet::new();
        let mut taken = Vec::new();
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id.clone()) {
                continue;
            }
            match self.get(&id) {
                Some((item, Source::User)) => {
                    let item = item.clone();
                    queue.extend(Self::needs_of(&item));
                    self.project.set(item);
                    taken.push(id);
                }
                Some((item, Source::Project)) => queue.extend(Self::needs_of(item)),
                _ => {}
            }
        }
        if !taken.is_empty() {
            self.touched(true);
        }
        taken
    }

    /// Adds an empty category, or sets its description and order (`addCategory`).
    pub fn add_category(&mut self, source: Source, category: Value) {
        if !source.editable() {
            return;
        }
        let key = path_key(&path_of(&category));
        let list = self.categories_mut(source);
        list.retain(|c| path_key(&path_of(c)) != key);
        list.push(category);
        self.touched(false);
    }

    /// Renames a category and moves everything under it (user and project items only).
    pub fn rename_category(&mut self, source: Source, path: &[String], name: &str) {
        if !source.editable() || path.is_empty() {
            return;
        }
        let depth = path.len() - 1;
        let under = |p: &[String]| p.len() >= path.len() && path.iter().zip(p).all(|(a, b)| a == b);
        let renamed = |p: &[String]| -> Vec<String> {
            p.iter()
                .enumerate()
                .map(|(i, s)| {
                    if i == depth {
                        name.to_owned()
                    } else {
                        s.clone()
                    }
                })
                .collect()
        };
        let store = self.store_mut(source);
        for it in &mut store.items {
            let p = path_of(&it.value);
            if under(&p)
                && let Some(o) = it.value.as_object_mut()
            {
                o.insert("path".into(), json!(renamed(&p)));
            }
        }
        for c in self.categories_mut(source).iter_mut() {
            let p = path_of(c);
            if under(&p)
                && let Some(o) = c.as_object_mut()
            {
                o.insert("path".into(), json!(renamed(&p)));
            }
        }
        self.touched(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> StyleLibrary {
        let sys = vec![
            Item::from_value(json!({ "kind": "asset", "id": "a-sys", "name": "Çizim", "path": ["Çizimler"], "format": "svg", "data": "<svg/>", "width": 1, "height": 1 })).unwrap(),
            Item::from_value(json!({ "kind": "symbol", "id": "s-sys", "name": "Ağaç", "path": ["MPYY", "Piktogramlar"], "tags": ["yeşil"], "symbol": { "type": "marker", "layers": [{ "id": "v", "type": "svg", "asset": "a-sys", "size": 4 }] } })).unwrap(),
        ];
        let mut lib = StyleLibrary::with_system(
            sys,
            vec![json!({ "path": ["MPYY"], "order": 1, "description": "Mekânsal planlar" })],
        );
        lib.load(
            Source::User,
            &[
                json!({ "kind": "asset", "id": "a-u", "name": "Benim", "path": ["Çizimler"], "format": "svg", "data": "<svg/>", "width": 1, "height": 1 }),
                json!({ "kind": "symbol", "id": "s-u", "name": "Benim sembolüm", "path": ["Semboller"], "symbol": { "type": "marker", "layers": [{ "id": "v", "type": "svg", "asset": "a-u", "size": 4 }] } }),
                json!({ "kind": "symbol", "id": "s-sys", "name": "Gölge", "path": [], "symbol": { "type": "fill", "layers": [] } }),
            ],
            &[],
        );
        lib
    }

    #[test]
    fn system_items_stay_and_edits_keep_the_order() {
        let mut lib = lib();
        // A user item shadowing a system id is left out.
        assert_eq!(lib.items(Some(Source::User)).len(), 2);
        assert!(
            lib.rename("s-sys", "x")
                .unwrap_err()
                .contains("sistem öğesidir")
        );
        assert_eq!(lib.rename("s-u", "  ").unwrap(), Source::User);
        assert_eq!(lib.get("s-u").unwrap().0.name(), "Adsız");
        let (items, _) = lib.dump(Source::User);
        assert_eq!(
            items[1]["id"], "s-u",
            "an edit keeps the item where it stands"
        );
        let copy = lib.copy("s-u", Source::Project, None, None).unwrap();
        assert_eq!(copy.name(), "Adsız (kopya)");
        assert!(copy.id().starts_with("p-"));
        // The user asset the symbol draws with travels along.
        let (project, _) = lib.dump(Source::Project);
        assert_eq!(project[0]["id"], "a-u");
        assert_eq!(lib.users_of("a-u").len(), 2);
        lib.remove("s-u").unwrap();
        assert!(lib.get("s-u").is_none());
    }

    /// docs/adr/9317: a user symbol a project uses is applied as the project's copy (one for the same symbol), with its
    /// drawing; what the drawing still refers to of Kitaplığım is taken along under its own id, and nothing else.
    #[test]
    fn a_project_takes_along_the_user_items_it_uses() {
        let mut lib = lib();
        let copy = lib.project_copy("s-u").unwrap();
        assert!(copy.starts_with("p-"), "{copy}");
        assert_eq!(lib.get(&copy).unwrap().0.name(), "Benim sembolüm", "the copy keeps its name");
        assert_eq!(lib.get("a-u").unwrap().1, Source::Project, "its drawing came along");
        assert_eq!(lib.project_copy("s-u").unwrap(), copy, "one copy for the same symbol");
        assert_eq!(lib.project_copy("s-sys").unwrap(), "s-sys", "a system item is used as it is");
        assert!(lib.project_copy("yok").is_err());

        let mut lib = self::lib();
        lib.load(
            Source::User,
            &[
                json!({ "kind": "asset", "id": "a-u", "name": "Benim", "path": [], "format": "svg", "data": "<svg/>", "width": 1, "height": 1 }),
                json!({ "kind": "symbol", "id": "s-u", "name": "İşaret", "path": [], "symbol": { "type": "marker", "layers": [{ "id": "v", "type": "svg", "asset": "a-u", "size": 4 }] } }),
                json!({ "kind": "symbol", "id": "f-u", "name": "Dolgu", "path": [], "symbol": { "type": "fill", "layers": [{ "id": "m", "type": "centroidMarker", "marker": { "ref": "s-u" } }] } }),
                json!({ "kind": "symbol", "id": "x-u", "name": "Kullanılmayan", "path": [], "symbol": { "type": "fill", "layers": [] } }),
            ],
            &[],
        );
        let taken = lib.take_along(["f-u".to_owned(), "s-sys".to_owned(), "yok".to_owned()]);
        assert_eq!(taken, ["f-u", "s-u", "a-u"], "the symbol, the marker it refers to and that marker's drawing");
        assert!(lib.get("x-u").is_some_and(|(_, s)| s == Source::User), "an item not used stays the user's alone");
        assert!(lib.take_along(["f-u".to_owned()]).is_empty(), "held already");
        let mut refs = refs_in(&json!({ "symbols": { "fill": { "ref": "a" }, "line": { "ref": "a" } }, "classes": [{ "symbols": { "marker": { "ref": "b" } } }] }));
        refs.sort();
        assert_eq!(refs, ["a", "b"], "every reference once");
    }

    #[test]
    fn the_tree_orders_categories_and_finds_by_folded_words() {
        let mut lib = lib();
        lib.add_category(Source::User, json!({ "path": ["Semboller", "Boş"] }));
        let tree = lib.tree(TreeFilter::default());
        let names: Vec<&str> = tree.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            ["MPYY", "Çizimler", "Semboller"],
            "ordered first, then Turkish"
        );
        assert_eq!(tree[0].description.as_deref(), Some("Mekânsal planlar"));
        assert_eq!(tree[2].children[0].name, "Boş");
        let found = lib.tree(TreeFilter {
            query: Some("YESIL agac"),
            ..TreeFilter::default()
        });
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].all_items().len(), 1);
        lib.rename_category(Source::User, &["Semboller".to_owned()], "Benimkiler");
        assert_eq!(lib.get("s-u").unwrap().0.path(), ["Benimkiler"]);
        let ids: Vec<String> = (0..50).map(|_| new_item_id("u")).collect();
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
    }
}
