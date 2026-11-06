//! Veri çıkarımı's core (docs/adr/1030): a recipe (`kentos_contracts::ExtractionRecipe`) worked out over drawings
//! already read, with a joined table already read. Pure: the host reads the files, gives each drawing's objects by
//! their persistent ids, and writes the dimension's shown text by the project's format (`Shown`); everything else is
//! here, so a server or the web's WASM works a recipe out alike.
//!
//! Each drawing's objects are visited in their order; with `scope.nested` (or into an external reference with
//! `scope.xrefs`) an insert's block is looked into, its pieces placed by the inserts above them (a dynamic block's
//! as its values vary them, `Dynamic::vary_traced`). Every occurrence has a path: the drawing (its path as the
//! recipe names it, empty for the open one), `#`, the object's persistent id, then each nested piece's local id
//! (`/12`; an external reference's piece by its source id, `/x:<id>`); an associative array counted whole is
//! `#dizi:<id>`. A path is counted once (tekil sayım). Rows are worked out from the unrounded values; only the
//! cells round them (`display::fixed`).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use kentos_contracts::{
    ArrayCount, AssocArray, BlockDefinition, DimensionEntity, Entity, ExtractValue,
    ExtractionRecipe, LayerNode, extract_factor, extract_heading, extract_kind_label,
};
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::block::{insert_matrix_xy, place};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::{
    Entity as CoreEntity, Shape, dimension_geom, enclosed_area, entity_area, entity_length,
};
use kentos_geometry_core::geom::affine::{Affine, IDENTITY, compose};
use kentos_geometry_core::geom::dimension::layout_dimension;

use crate::blocks::{core_dynamic, core_params, shape};

/// The word a Toplam row's first word cell holds.
pub const TOTAL: &str = "Toplam";
/// The words a Drawing column writes for the open drawing.
pub const THIS_DRAWING: &str = "Bu çizim";
/// What separates a grouped row's word cells in its key: a control character no cell holds.
pub const KEY_SEPARATOR: char = '\u{1f}';

/// A drawing as the host read it.
pub struct Drawing<'a> {
    /// Its path as the recipe names it; empty for the open drawing.
    pub path: &'a str,
    /// Its file's name (Bu çizim for the open one).
    pub name: String,
    /// Its objects in its order, each with its persistent id.
    pub objects: Vec<(String, &'a Entity)>,
    pub blocks: Vec<&'a BlockDefinition>,
    pub arrays: Vec<&'a AssocArray>,
    pub layers: &'a [LayerNode],
    /// The external references (by block id) that name a drawing the recipe reads itself: never looked into.
    pub read_apart: HashSet<kentos_contracts::BlockId>,
}

/// The project's decimals a quantity takes by default.
#[derive(Clone, Copy, Debug)]
pub struct Decimals {
    pub length: u32,
    pub area: u32,
}

/// A refusal: its code, the input's path and the words.
pub type Refusal = (&'static str, String, String);

/// One row worked out.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Its key (a grouped row's word cells, else its occurrence's path).
    pub key: String,
    pub cells: Vec<String>,
    /// Each column's unrounded value in its unit; none for a word column or an empty cell.
    pub values: Vec<Option<f64>>,
    /// Its occurrences' paths, in reading order.
    pub members: Vec<String>,
}

/// What the extraction left out or could not match, for the report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notes {
    /// Pieces a visibility state hides and objects on a hidden or frozen layer that would have been counted.
    pub hidden: usize,
    /// Of those, the pieces a dynamic block's visibility state hides (an array action's copies of them too,
    /// docs/adr/1131 §2).
    pub hidden_states: usize,
    /// External references whose content was not loaded, by block name.
    pub unloaded: Vec<String>,
    /// External references not looked into because the recipe reads their drawing itself, by block name.
    pub read_apart: Vec<String>,
    /// Paths reached twice and counted once.
    pub repeated: usize,
    /// Dimensions whose measure is an angle: no length for a Gerçek ölçü column.
    pub angles: usize,
    /// Dimensions whose text overrides their measure (their Yazılan is words, not summed).
    pub overrides: usize,
    /// Rows whose join key the joined table does not have, by the key's words.
    pub unmatched: Vec<String>,
    /// The joined table's keys no row used.
    pub unused: Vec<String>,
    /// Nested blocks not looked into past the deepest level.
    pub too_deep: usize,
}

/// A recipe worked out.
#[derive(Clone, Debug, PartialEq)]
pub struct Extracted {
    pub headings: Vec<String>,
    /// Which columns are quantities.
    pub quantities: Vec<bool>,
    pub rows: Vec<Row>,
    /// The Toplam row's values (its quantity columns'), when the recipe totals.
    pub totals: Option<Vec<Option<f64>>>,
    /// How many occurrences were counted.
    pub occurrences: usize,
    pub notes: Notes,
}

impl Extracted {
    /// The table's cells: the headings, the rows, the Toplam row.
    pub fn cells(&self, recipe: &ExtractionRecipe, decimals: Decimals) -> Vec<Vec<String>> {
        let mut out = vec![self.headings.clone()];
        out.extend(self.rows.iter().map(|r| r.cells.clone()));
        if let Some(t) = &self.totals {
            let label = recipe.columns.iter().position(|c| !c.value.is_quantity());
            out.push(
                t.iter()
                    .enumerate()
                    .map(|(j, v)| match v {
                        Some(x) => fixed(*x, column_decimals(recipe, j, decimals) as usize),
                        None if Some(j) == label => TOTAL.to_owned(),
                        None => String::new(),
                    })
                    .collect(),
            );
        }
        out
    }
}

/// A quantity column's decimals: its own, else the project's for its kind (a computed column's, its source's).
pub fn column_decimals(recipe: &ExtractionRecipe, j: usize, d: Decimals) -> u32 {
    let Some(c) = recipe.columns.get(j) else {
        return 0;
    };
    if let Some(x) = c.decimals {
        return x;
    }
    match &c.value {
        ExtractValue::Count => 0,
        ExtractValue::Length { .. } | ExtractValue::Measured { .. } => d.length,
        ExtractValue::Area { .. } => d.area,
        ExtractValue::Computed { column, .. } if (*column as usize) < j => {
            column_decimals(recipe, *column as usize, d)
        }
        _ => 0,
    }
}

/// Neumaier's compensated sum: the running sum and its lost low part.
#[derive(Clone, Copy, Default)]
struct Sum {
    s: f64,
    c: f64,
    any: bool,
}

impl Sum {
    fn add(&mut self, x: f64) {
        let t = self.s + x;
        if self.s.abs() >= x.abs() {
            self.c += (self.s - t) + x;
        } else {
            self.c += (x - t) + self.s;
        }
        self.s = t;
        self.any = true;
    }

    fn value(&self) -> Option<f64> {
        self.any.then_some(self.s + self.c)
    }
}

/// One occurrence as read: its path, its words and values per column.
struct Occurrence {
    path: String,
    words: Vec<String>,
    values: Vec<Option<f64>>,
}

/// A layer's name and whether it is shown, by its id.
fn layer_map(layers: &[LayerNode]) -> HashMap<&str, (&str, bool)> {
    fn walk<'a>(nodes: &'a [LayerNode], hidden: bool, out: &mut HashMap<&'a str, (&'a str, bool)>) {
        for n in nodes {
            let h = hidden || !n.visible || n.frozen == Some(true);
            out.insert(n.id.as_str(), (n.name.as_str(), h));
            walk(&n.children, h, out);
        }
    }
    let mut out = HashMap::new();
    walk(layers, false, &mut out);
    out
}

/// A name as block names compare (Turkish case folded).
fn folded(name: &str) -> String {
    kentos_contracts::blocks::name_key(name)
}

/// A dynamic value as a Variant cell writes it.
fn said(v: &kentos_contracts::ParamValue) -> String {
    use kentos_contracts::ParamValue as V;
    let number = |x: f64| {
        let r = (x * 1e6).round() / 1e6;
        if r == 0.0 {
            "0".to_owned()
        } else {
            format!("{r}")
        }
    };
    match v {
        V::Flag(b) => if *b { "evet" } else { "hayır" }.to_owned(),
        V::Number(x) => number(*x),
        V::Text(t) => t.clone(),
        V::Point(p) => format!("({}, {})", number(p.x), number(p.y)),
    }
}

/// Where a piece is looked at from: the affine its inserts place it by, whether they are all similarities, the
/// layer a piece on the block's layer takes, whether it is hidden already, and how deep.
#[derive(Clone)]
struct Frame<'a> {
    m: Affine,
    similar: bool,
    layer: Option<&'a str>,
    hidden: bool,
    /// Hidden by a dynamic block's visibility state, here or above.
    state: bool,
    depth: usize,
}

struct Walk<'a, 'r> {
    recipe: &'r ExtractionRecipe,
    drawing: &'r Drawing<'a>,
    layers: HashMap<&'a str, (&'a str, bool)>,
    blocks: HashMap<kentos_contracts::BlockId, &'a BlockDefinition>,
    kinds: HashSet<&'r str>,
    block_names: HashSet<String>,
    layer_names: HashSet<&'r str>,
    shown: &'r dyn Fn(&DimensionEntity, &str, &str, f64) -> String,
    seen: &'r mut HashSet<String>,
    out: &'r mut Vec<Occurrence>,
    notes: &'r mut Notes,
    limit: bool,
}

impl<'a> Walk<'a, '_> {
    fn layer_of(&self, own: &'a str, frame: &Frame<'a>) -> &'a str {
        if own.is_empty() {
            frame.layer.unwrap_or(own)
        } else {
            own
        }
    }

    fn counted(&mut self, path: String) -> bool {
        if self.out.len() >= kentos_contracts::MAX_EXTRACT_OCCURRENCES {
            self.limit = true;
            return false;
        }
        if !self.seen.insert(path) {
            self.notes.repeated += 1;
            return false;
        }
        true
    }

    /// The object `e` at `path` seen from `frame` (its own `id` the local id when nested).
    fn visit(&mut self, e: &'a Entity, core: Option<&Shape>, path: String, frame: &Frame<'a>) {
        if self.limit {
            return;
        }
        let layer_id = self.layer_of(e.base().layer_id.as_str(), frame);
        let (layer_name, layer_hidden) = self
            .layers
            .get(layer_id)
            .copied()
            .unwrap_or((layer_id, false));
        let hidden = frame.hidden || layer_hidden;
        let kind = e.kind();
        let takes_layer = self.layer_names.is_empty() || self.layer_names.contains(layer_name);
        if let Entity::Insert(i) = e {
            let def = self.blocks.get(&i.block).copied();
            let name = def.map_or("", |b| b.name.as_str());
            let xref = def.and_then(|b| b.xref.as_ref());
            // An external reference looked into is its content, not an insert of its own.
            let into_xref = xref.is_some() && self.recipe.scope.xrefs;
            let takes = !into_xref
                && self.kinds.contains("insert")
                && takes_layer
                && (self.block_names.is_empty() || self.block_names.contains(&folded(name)));
            if takes {
                if hidden && !self.recipe.scope.hidden {
                    self.notes.hidden += 1;
                    if frame.state {
                        self.notes.hidden_states += 1;
                    }
                } else if self.counted(path.clone()) {
                    let occurrence = self.read(e, core, def, layer_name, &path, frame);
                    self.out.push(occurrence);
                }
            }
            let look = match (def, xref) {
                (Some(_), Some(x)) if self.recipe.scope.xrefs => {
                    if x.unloaded {
                        if !self.notes.unloaded.iter().any(|n| n == name) {
                            self.notes.unloaded.push(name.to_owned());
                        }
                        false
                    } else if self.drawing.read_apart.contains(&i.block) {
                        if !self.notes.read_apart.iter().any(|n| n == name) {
                            self.notes.read_apart.push(name.to_owned());
                        }
                        false
                    } else {
                        true
                    }
                }
                (Some(_), Some(_)) => false,
                (Some(_), None) => self.recipe.scope.nested,
                _ => false,
            };
            if let (true, Some(def)) = (look, def) {
                if frame.depth >= kentos_contracts::MAX_EXTRACT_DEPTH {
                    self.notes.too_deep += 1;
                    return;
                }
                self.enter_block(i, core, def, layer_id, hidden, &path, frame);
            }
            return;
        }
        if !self.kinds.contains(kind) || !takes_layer {
            return;
        }
        if hidden && !self.recipe.scope.hidden {
            self.notes.hidden += 1;
            if frame.state {
                self.notes.hidden_states += 1;
            }
            return;
        }
        if self.counted(path.clone()) {
            let occurrence = self.read(e, core, None, layer_name, &path, frame);
            self.out.push(occurrence);
        }
    }

    /// An insert's block looked into: its pieces as its values draw them, placed by it and the inserts above.
    #[allow(clippy::too_many_arguments)]
    fn enter_block(
        &mut self,
        i: &'a kentos_contracts::InsertEntity,
        core: Option<&Shape>,
        def: &'a BlockDefinition,
        layer_id: &'a str,
        hidden: bool,
        path: &str,
        frame: &Frame<'a>,
    ) {
        // The insert as placed (a nested one by the varied shape its parent gave it).
        let own = shape(&Entity::Insert(i.clone()));
        let placed = core.unwrap_or(&own);
        let Shape::Insert {
            p,
            scale,
            y_scale,
            rotation,
            mirror,
            ..
        } = placed
        else {
            return;
        };
        let local = insert_matrix_xy(
            kentos_geometry_core::Vec2::new(def.base.x, def.base.y),
            *p,
            (*scale, y_scale.unwrap_or(*scale)),
            *rotation,
            *mirror == Some(true),
        );
        let next = Frame {
            m: compose(&frame.m, &local),
            similar: frame.similar && y_scale.is_none_or(|y| y == *scale),
            layer: Some(layer_id),
            hidden,
            state: frame.state,
            depth: frame.depth + 1,
        };
        let xref = def.xref.as_ref();
        let segment = |k: usize, local_id: u32| match xref.and_then(|x| x.objects.get(k)) {
            Some(source) => format!("{path}/x:{source}"),
            None => format!("{path}/{local_id}"),
        };
        let dynamic = def.dynamic.as_ref().and_then(core_dynamic);
        let Some(d) = dynamic else {
            for (k, piece) in def.entities.iter().enumerate() {
                self.visit(piece, None, segment(k, piece.base().id), &next);
            }
            return;
        };
        let cores: Vec<CoreEntity> = def
            .entities
            .iter()
            .map(|e| CoreEntity {
                shape: shape(e),
                rest: vec![("id".to_owned(), Json::Num(f64::from(e.base().id)))],
            })
            .collect();
        let params = core_params(&i.parameters);
        // The pieces as the values draw them, and those the visibility state hides where the actions put them (an
        // array action's copies of a hidden piece are hidden too, docs/adr/1131 §2): counted only with `scope.hidden`.
        let (drawn, hidden) = d.vary_split(&cores, params.as_ref());
        let hidden_frame = Frame {
            hidden: true,
            state: true,
            ..next.clone()
        };
        for (list, at) in [(&drawn, &next), (&hidden, &hidden_frame)] {
            for (e, source) in list {
                let Some(piece) = def.entities.get(*source) else {
                    continue;
                };
                let id = e
                    .rest
                    .iter()
                    .find_map(|(k, v)| match (k.as_str(), v) {
                        ("id", Json::Num(n)) if *n >= 0.0 => Some(*n as u32),
                        _ => None,
                    })
                    .unwrap_or(piece.base().id);
                self.visit(piece, Some(&e.shape), segment(*source, id), at);
            }
        }
    }

    /// An occurrence's words and values, column by column.
    fn read(
        &mut self,
        e: &Entity,
        core: Option<&Shape>,
        def: Option<&BlockDefinition>,
        layer: &str,
        path: &str,
        frame: &Frame<'_>,
    ) -> Occurrence {
        let own;
        let base = match core {
            Some(s) => s,
            None => {
                own = shape(e);
                &own
            }
        };
        let placed = if frame.m == IDENTITY {
            base.clone()
        } else {
            place(base, &frame.m, frame.similar)
        };
        let dimension = match e {
            Entity::Dimension(d) => {
                let laid = dimension_geom(&placed).and_then(|g| layout_dimension(&g));
                let text = d.text.as_deref().filter(|t| !t.is_empty());
                let overridden = text.is_some_and(|t| !t.contains("<>"));
                if overridden {
                    self.notes.overrides += 1;
                }
                if laid.as_ref().is_some_and(|l| l.unit != "length") {
                    self.notes.angles += 1;
                }
                Some((d, laid, overridden))
            }
            _ => None,
        };
        let mut words = Vec::with_capacity(self.recipe.columns.len());
        let mut values: Vec<Option<f64>> = Vec::with_capacity(self.recipe.columns.len());
        for c in &self.recipe.columns {
            let (w, v) = match &c.value {
                ExtractValue::Drawing => (self.drawing.name.clone(), None),
                ExtractValue::Kind => (extract_kind_label(e.kind()).unwrap_or("").to_owned(), None),
                ExtractValue::Block => (def.map_or(String::new(), |b| b.name.clone()), None),
                ExtractValue::Layer => (layer.to_owned(), None),
                ExtractValue::Variant => match e {
                    Entity::Insert(i) if def.is_some_and(|b| b.dynamic.is_some()) => (
                        i.parameters
                            .iter()
                            .map(|(k, v)| format!("{k}={}", said(v)))
                            .collect::<Vec<_>>()
                            .join("; "),
                        None,
                    ),
                    _ => (String::new(), None),
                },
                ExtractValue::Attribute { tag } => {
                    let v = match (e, def) {
                        (Entity::Insert(i), Some(b)) => {
                            b.attribute_value(i, tag).map(str::to_owned)
                        }
                        _ => None,
                    }
                    .or_else(|| e.base().attrs.get(tag).cloned())
                    .unwrap_or_default();
                    // A cell is one line: a multi-line value's lines joined by a space.
                    (
                        v.split(['\n', '\r'])
                            .filter(|l| !l.is_empty())
                            .collect::<Vec<_>>()
                            .join(" "),
                        None,
                    )
                }
                ExtractValue::Count => (String::new(), Some(1.0)),
                ExtractValue::Length { unit } => (
                    String::new(),
                    match e {
                        Entity::Insert(_)
                        | Entity::Text(_)
                        | Entity::Point(_)
                        | Entity::Dimension(_) => None,
                        _ => entity_length(&placed),
                    }
                    .map(|l| l * unit.per_metre()),
                ),
                ExtractValue::Area { unit } => (
                    String::new(),
                    entity_area(&placed)
                        .or_else(|| enclosed_area(&placed))
                        .map(|a| a / unit.square_metres()),
                ),
                ExtractValue::Measured { unit } => (
                    String::new(),
                    dimension
                        .as_ref()
                        .and_then(|(_, l, _)| l.as_ref())
                        .filter(|l| l.unit == "length" && l.value.is_finite())
                        .map(|l| l.value * unit.per_metre()),
                ),
                ExtractValue::Shown => (
                    match &dimension {
                        Some((d, Some(l), _)) => (self.shown)(d, l.prefix, l.unit, l.value),
                        _ => String::new(),
                    },
                    None,
                ),
                ExtractValue::Overridden => (
                    match &dimension {
                        Some((_, _, true)) => "evet".to_owned(),
                        Some((_, _, false)) => "hayır".to_owned(),
                        None => String::new(),
                    },
                    None,
                ),
                ExtractValue::Computed { column, factor, .. } => (
                    String::new(),
                    values
                        .get(*column as usize)
                        .copied()
                        .flatten()
                        .zip(extract_factor(factor))
                        .map(|(x, k)| x * k),
                ),
                // Filled in by the join.
                ExtractValue::Joined { .. } => (String::new(), None),
            };
            words.push(w);
            values.push(v.filter(|x| x.is_finite()));
        }
        Occurrence {
            path: path.to_owned(),
            words,
            values,
        }
    }
}

/// The joined table made ready: its key column's rows by their key, and each Joined column's index in it.
struct Joined {
    by_key: HashMap<String, usize>,
    keys: Vec<String>,
    fields: Vec<Option<usize>>,
    rows: Vec<Vec<String>>,
}

fn joined(
    recipe: &ExtractionRecipe,
    table: Option<&[Vec<String>]>,
) -> Result<Option<Joined>, Refusal> {
    let Some(j) = &recipe.join else {
        return Ok(None);
    };
    let Some((head, rest)) = table.and_then(<[Vec<String>]>::split_first) else {
        return Err((
            "join_unusable",
            "join.path".into(),
            format!(
                "“{}” dış tablosunda başlık satırı yok; ilk satır sütunların adlarıdır.",
                j.path
            ),
        ));
    };
    let find = |name: &str| head.iter().position(|h| h.trim() == name.trim());
    let Some(key) = find(&j.key) else {
        return Err((
            "join_unusable",
            "join.key".into(),
            format!(
                "“{}” dış tablosunda “{}” başlıklı sütun yok; anahtar sütununun başlığını yazın.",
                j.path, j.key
            ),
        ));
    };
    let mut fields = Vec::with_capacity(recipe.columns.len());
    for (i, c) in recipe.columns.iter().enumerate() {
        fields.push(match &c.value {
            ExtractValue::Joined { field } => Some(find(field).ok_or_else(|| {
                (
                    "join_unusable",
                    format!("columns[{i}].value.field"),
                    format!("“{}” dış tablosunda “{field}” başlıklı sütun yok.", j.path),
                )
            })?),
            _ => None,
        });
    }
    let mut by_key = HashMap::new();
    let mut keys = Vec::new();
    for (r, row) in rest.iter().enumerate() {
        let k = row.get(key).map(|s| s.trim()).unwrap_or("");
        if k.is_empty() {
            return Err((
                "join_unusable",
                "join.key".into(),
                format!(
                    "“{}” dış tablosunun {}. veri satırının anahtarı boş; her satırın anahtarı olmalı.",
                    j.path,
                    r + 1
                ),
            ));
        }
        if by_key.insert(k.to_owned(), r).is_some() {
            return Err((
                "join_unusable",
                "join.key".into(),
                format!(
                    "“{k}” anahtarı “{}” dış tablosunda birden çok kez geçiyor; satırlar anahtarla eşlenemiyor.",
                    j.path
                ),
            ));
        }
        keys.push(k.to_owned());
    }
    Ok(Some(Joined {
        by_key,
        keys,
        fields,
        rows: rest.to_vec(),
    }))
}

/// How far a running extraction is and whether it is to stop (docs/adr/1131 §3): `total` the drawings' top-level
/// objects, `done` those walked; `stop` is looked at every [`WATCH_EVERY`] objects, and a stopped extraction answers
/// [`CANCELLED`] with nothing half made.
#[derive(Debug, Default)]
pub struct Watch {
    pub done: AtomicUsize,
    pub total: AtomicUsize,
    pub stop: AtomicBool,
}

/// How many top-level objects are walked between two looks at [`Watch::stop`].
pub const WATCH_EVERY: usize = 256;
/// The refusal of a stopped extraction.
pub const CANCELLED: &str = "cancelled";

/// Works `recipe` out over `drawings` (the open one first when it reads it), `join` the joined table's rows (its
/// first its headings) when it joins one. `shown` writes a dimension's text as the project shows it (its prefix,
/// unit and measured value given).
pub fn extract(
    recipe: &ExtractionRecipe,
    drawings: &[Drawing<'_>],
    join: Option<&[Vec<String>]>,
    decimals: Decimals,
    shown: &dyn Fn(&DimensionEntity, &str, &str, f64) -> String,
) -> Result<Extracted, Refusal> {
    extract_watched(recipe, drawings, join, decimals, shown, &Watch::default())
}

/// [`extract`] that says how far it is and stops when asked (`watch`).
pub fn extract_watched(
    recipe: &ExtractionRecipe,
    drawings: &[Drawing<'_>],
    join: Option<&[Vec<String>]>,
    decimals: Decimals,
    shown: &dyn Fn(&DimensionEntity, &str, &str, f64) -> String,
    watch: &Watch,
) -> Result<Extracted, Refusal> {
    watch
        .total
        .store(drawings.iter().map(|d| d.objects.len()).sum(), Ordering::Relaxed);
    if let Some((at, words)) = kentos_contracts::extraction_recipe_problem(recipe) {
        return Err(("invalid_extraction_recipe", at, words));
    }
    let table = joined(recipe, join)?;
    let kinds: HashSet<&str> = if recipe.kinds.is_empty() {
        std::iter::once("insert").collect()
    } else {
        recipe.kinds.iter().map(String::as_str).collect()
    };
    let block_names: HashSet<String> = recipe.blocks.iter().map(|n| folded(n)).collect();
    let layer_names: HashSet<&str> = recipe.layers.iter().map(String::as_str).collect();
    let mut seen = HashSet::new();
    let mut out: Vec<Occurrence> = Vec::new();
    let mut notes = Notes::default();
    for d in drawings {
        let mut walk = Walk {
            recipe,
            drawing: d,
            layers: layer_map(d.layers),
            blocks: d.blocks.iter().map(|b| (b.id, *b)).collect(),
            kinds: kinds.clone(),
            block_names: block_names.clone(),
            layer_names: layer_names.clone(),
            shown,
            seen: &mut seen,
            out: &mut out,
            notes: &mut notes,
            limit: false,
        };
        // An array counted whole stands where its first item is; its items are neither counted nor looked into.
        let mut item_of: HashMap<String, usize> = HashMap::new();
        if recipe.scope.arrays == ArrayCount::Whole {
            for (k, a) in d.arrays.iter().enumerate() {
                for item in &a.items {
                    if let Some(u) = item.uid {
                        item_of.insert(u.to_text(), k);
                    }
                }
            }
        }
        let root = Frame {
            m: IDENTITY,
            similar: true,
            layer: None,
            hidden: false,
            state: false,
            depth: 0,
        };
        let mut placed_arrays = vec![false; d.arrays.len()];
        for (n, (uid, e)) in d.objects.iter().enumerate() {
            watch.done.fetch_add(1, Ordering::Relaxed);
            if n % WATCH_EVERY == 0 && watch.stop.load(Ordering::Relaxed) {
                return Err((CANCELLED, "recipe".into(), "Veri çıkarımı durduruldu; tablo yazılmadı.".into()));
            }
            if let Some(&k) = item_of.get(uid) {
                if !placed_arrays[k] {
                    placed_arrays[k] = true;
                    walk.array(d.arrays[k]);
                }
                continue;
            }
            walk.visit(e, None, format!("{}#{uid}", d.path), &root);
        }
        if walk.limit {
            return Err((
                "extraction_too_large",
                "recipe".into(),
                format!(
                    "Tarif {} oluşumdan çok nesne okuyor; kapsamı (tür, blok, katman) daraltın.",
                    kentos_contracts::MAX_EXTRACT_OCCURRENCES
                ),
            ));
        }
    }
    let width = recipe.columns.len();
    let quantities: Vec<bool> = recipe
        .columns
        .iter()
        .map(|c| c.value.is_quantity())
        .collect();
    // The joined columns' words, by each occurrence's key.
    if let (Some(t), Some(j)) = (&table, &recipe.join) {
        let on = j.on as usize;
        let mut used = HashSet::new();
        for o in &mut out {
            let key = o.words[on].trim().to_owned();
            match t.by_key.get(&key) {
                Some(&r) => {
                    used.insert(r);
                    for (c, f) in t.fields.iter().enumerate() {
                        if let Some(f) = f {
                            o.words[c] = t.rows[r].get(*f).cloned().unwrap_or_default();
                        }
                    }
                }
                None => {
                    if !notes.unmatched.contains(&key) {
                        notes.unmatched.push(key);
                    }
                }
            }
        }
        notes.unused = t
            .keys
            .iter()
            .enumerate()
            .filter(|(r, _)| !used.contains(r))
            .map(|(_, k)| k.clone())
            .collect();
    }
    let joined_column = |c: usize| matches!(recipe.columns[c].value, ExtractValue::Joined { .. });
    let cell = |j: usize, v: Option<f64>, w: &str| match v {
        Some(x) if quantities[j] => fixed(x, column_decimals(recipe, j, decimals) as usize),
        _ if quantities[j] => String::new(),
        _ => w.to_owned(),
    };
    let mut rows: Vec<Row> = Vec::new();
    if recipe.group {
        let mut at: HashMap<String, usize> = HashMap::new();
        let mut sums: Vec<Vec<Sum>> = Vec::new();
        for o in &out {
            let key: String = (0..width)
                .filter(|&c| !quantities[c] && !joined_column(c))
                .map(|c| o.words[c].as_str())
                .collect::<Vec<_>>()
                .join(&KEY_SEPARATOR.to_string());
            let r = *at.entry(key.clone()).or_insert_with(|| {
                rows.push(Row {
                    key,
                    cells: o.words.clone(),
                    values: vec![None; width],
                    members: Vec::new(),
                });
                sums.push(vec![Sum::default(); width]);
                rows.len() - 1
            });
            rows[r].members.push(o.path.clone());
            for (c, v) in o.values.iter().enumerate() {
                if let (true, Some(x)) = (quantities[c], v) {
                    sums[r][c].add(*x);
                }
            }
        }
        for (row, s) in rows.iter_mut().zip(&sums) {
            for c in 0..width {
                if quantities[c] {
                    row.values[c] = s[c].value();
                    row.cells[c] = cell(c, row.values[c], "");
                }
            }
        }
    } else {
        for o in &out {
            let cells = (0..width)
                .map(|c| cell(c, o.values[c], &o.words[c]))
                .collect();
            rows.push(Row {
                key: o.path.clone(),
                cells,
                values: (0..width)
                    .map(|c| if quantities[c] { o.values[c] } else { None })
                    .collect(),
                members: vec![o.path.clone()],
            });
        }
    }
    if !recipe.sort.is_empty() {
        rows.sort_by(|a, b| {
            for s in &recipe.sort {
                let c = s.column as usize;
                let o = if quantities[c] {
                    match (a.values[c], b.values[c]) {
                        (Some(x), Some(y)) => {
                            let o = x.total_cmp(&y);
                            if s.descending { o.reverse() } else { o }
                        }
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                } else {
                    let o = a.cells[c].cmp(&b.cells[c]);
                    if s.descending { o.reverse() } else { o }
                };
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            std::cmp::Ordering::Equal
        });
    }
    let totals = recipe.total.then(|| {
        (0..width)
            .map(|c| {
                quantities[c].then(|| {
                    let mut s = Sum::default();
                    for o in &out {
                        if let Some(x) = o.values[c] {
                            s.add(x);
                        }
                    }
                    s.value().unwrap_or(0.0)
                })
            })
            .collect()
    });
    let headings = recipe.columns.iter().map(extract_heading).collect();
    Ok(Extracted {
        headings,
        quantities,
        rows,
        totals,
        occurrences: out.len(),
        notes,
    })
}

impl Walk<'_, '_> {
    /// An associative array counted whole: one occurrence of its block on its layer.
    fn array(&mut self, a: &AssocArray) {
        let def = self.blocks.get(&a.block).copied();
        let name = def.map_or("", |b| b.name.as_str());
        let (layer_name, hidden) = self
            .layers
            .get(a.layer_id.as_str())
            .copied()
            .unwrap_or((a.layer_id.as_str(), false));
        let takes = self.kinds.contains("insert")
            && (self.layer_names.is_empty() || self.layer_names.contains(layer_name))
            && (self.block_names.is_empty() || self.block_names.contains(&folded(name)));
        if !takes {
            return;
        }
        if hidden && !self.recipe.scope.hidden {
            self.notes.hidden += 1;
            return;
        }
        let path = format!("{}#dizi:{}", self.drawing.path, a.id.to_text());
        if !self.counted(path.clone()) {
            return;
        }
        let words = self
            .recipe
            .columns
            .iter()
            .map(|c| match &c.value {
                ExtractValue::Drawing => self.drawing.name.clone(),
                ExtractValue::Kind => extract_kind_label("array").unwrap_or("").to_owned(),
                ExtractValue::Block => name.to_owned(),
                ExtractValue::Layer => layer_name.to_owned(),
                _ => String::new(),
            })
            .collect();
        let mut values: Vec<Option<f64>> = Vec::with_capacity(self.recipe.columns.len());
        for c in &self.recipe.columns {
            let v = match &c.value {
                ExtractValue::Count => Some(1.0),
                ExtractValue::Computed { column, factor, .. } => values
                    .get(*column as usize)
                    .copied()
                    .flatten()
                    .zip(extract_factor(factor))
                    .map(|(x, k)| x * k),
                _ => None,
            };
            values.push(v);
        }
        self.out.push(Occurrence {
            path,
            words,
            values,
        });
    }
}

/// A recipe file's format (Tarifi kaydet, docs/adr/1030 §2): `{"format": "kentos.extraction-recipe", "version": 1,
/// "recipe": …}`, the recipe as the contract writes it.
pub const RECIPE_FORMAT: &str = "kentos.extraction-recipe";

/// `recipe` as a recipe file's text.
pub fn recipe_file(recipe: &ExtractionRecipe) -> String {
    let body = serde_json::json!({ "format": RECIPE_FORMAT, "version": 1, "recipe": recipe });
    serde_json::to_string_pretty(&body).unwrap_or_default() + "\n"
}

/// The recipe a recipe file holds, or why it holds none: another format or version, a recipe against its rules.
pub fn read_recipe_file(text: &str) -> Result<ExtractionRecipe, String> {
    let v: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("Tarif dosyası JSON olarak okunamadı: {e}"))?;
    if v.get("format").and_then(serde_json::Value::as_str) != Some(RECIPE_FORMAT) {
        return Err(format!(
            "Bu bir veri çıkarımı tarifi değil: biçimi “{RECIPE_FORMAT}” olmalı."
        ));
    }
    if v.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err("Tarif dosyasının sürümü 1 değil: bu sürümün okumadığı bir dosya.".into());
    }
    let recipe: ExtractionRecipe =
        serde_json::from_value(v.get("recipe").cloned().unwrap_or_default())
            .map_err(|e| format!("Tarif okunamadı: {e}"))?;
    if let Some((at, words)) = kentos_contracts::extraction_recipe_problem(&recipe) {
        return Err(format!("Tarif kurallara uymuyor ({at}): {words}"));
    }
    Ok(recipe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recipe_file_reads_back_as_written_and_refuses_another() {
        let recipe: ExtractionRecipe = serde_json::from_value(serde_json::json!({
            "name": "Bloklar", "scope": {"nested": true},
            "columns": [{"label": "Blok", "value": {"of": "block"}}, {"label": "Adet", "value": {"of": "count"}}],
            "group": true, "total": true
        }))
        .unwrap();
        let text = recipe_file(&recipe);
        assert_eq!(read_recipe_file(&text).unwrap(), recipe);
        assert!(read_recipe_file(&text.replace(RECIPE_FORMAT, "kentos.style")).is_err());
        assert!(read_recipe_file(&text.replace("\"Bloklar\"", "\" \"")).is_err());
    }
}
