//! One layer through the style engine (the web's `buildStyledLayer`,
//! `render/styledLayer.ts`; docs/STYLE.md §6): each object gets its own
//! symbol, else its layer's renderer, else the layer's simple look. This
//! side says how each object is drawn (four numbers: the mode, the set or
//! own symbol, the simple look's set, the colour), gives the core the
//! symbols, sets and assets they refer to and the objects' values for the
//! expressions; the core compiles and packs the batches next to the geometry
//! store (`kentos_style_core::style::build::build_layer`).

use std::collections::{HashMap, HashSet};

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::{js_max, js_round};
use kentos_geometry_core::store::Store;
use kentos_style_core::style::batch::Batches;
use kentos_style_core::style::build::{
    LayerObjects, MODE_DIMENSION, MODE_OWN, MODE_RENDERER, MODE_SET, MODE_SKIP, Program,
    build_layer as core_build,
};
use serde_json::{Map, Value, json};

use crate::library::StyleLibrary;
use crate::simple::{hatch_symbol_of, symbols_of_layer_style};
use crate::table::{ExprTable, expr_table};

/// Metres of paper per CSS pixel at 96 dpi (0.26458 mm), as the web's `METRES_PER_PX`.
pub const METRES_PER_PX: f64 = 0.00026458;

/// The scale paper-mm symbol sizes are compiled at (`symbolScaleOf`): the
/// project's plot scale, or with screen-sized symbols (Semboller → Ekranda
/// sabit) the view's own scale denominator in quarter-octave steps, so a
/// zoom does not rebuild every layer at every wheel tick. `px_per_m`: the
/// view's logical pixels per metre.
pub fn symbol_scale_of(screen: bool, plot_scale: f64, px_per_m: f64) -> f64 {
    if !screen {
        return plot_scale;
    }
    let denominator = 1.0 / (px_per_m * METRES_PER_PX);
    libm::exp2(js_round(libm::log2(js_max(denominator, 1.0)) * 4.0) / 4.0)
}

/// What a layer build needs besides the layer.
pub struct BuildOptions<'a> {
    /// The local origin the batches' float32 numbers are taken from.
    pub origin: Vec2,
    /// The scale symbols are compiled at ([`symbol_scale_of`]).
    pub plot_scale: f64,
    /// Symbol sizes on the screen: paper mm drawn as px.
    pub screen: bool,
    /// Line weights hidden (Kalınlık off): the simple look draws one-pixel lines.
    pub hairlines: bool,
    /// The box construction lines are clipped to.
    pub clip: Option<Bounds>,
    pub library: &'a StyleLibrary,
    /// A layer's name by id (`$katman`).
    pub layer_name: &'a dyn Fn(&str) -> String,
}

/// What the page gives the core for a layer: the program's JSON, the
/// objects' four numbers each, the value table. Kept for the fixtures,
/// which hold both platforms to the same call.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerCall {
    pub program: String,
    pub objects: Vec<i32>,
    pub table: ExprTable,
    /// An expression reads an object's place in the run (`$sıra`): the
    /// layer's objects must be built as one run to draw as they do.
    pub reads_index: bool,
}

/// The library symbols a renderer's sets refer to (`rendererRefs`).
fn renderer_refs(r: &Value, out: &mut Vec<String>) {
    fn set(s: &Value, out: &mut Vec<String>) {
        for slot in ["marker", "line", "fill"] {
            if let Some(id) = s
                .get(slot)
                .and_then(|v| v.get("ref"))
                .and_then(Value::as_str)
            {
                out.push(id.to_owned());
            }
        }
    }
    fn rules(list: &Value, out: &mut Vec<String>) {
        for r in list.as_array().into_iter().flatten() {
            if let Some(s) = r.get("symbols") {
                set(s, out);
            }
            if let Some(children) = r.get("children") {
                rules(children, out);
            }
        }
    }
    let each = |k: &str, out: &mut Vec<String>| {
        for c in r.get(k).and_then(Value::as_array).into_iter().flatten() {
            if let Some(s) = c.get("symbols") {
                set(s, out);
            }
        }
    };
    match r.get("type").and_then(Value::as_str) {
        Some("single") => {
            if let Some(s) = r.get("symbols") {
                set(s, out);
            }
        }
        Some("categorized") => {
            each("categories", out);
            if let Some(s) = r.get("other") {
                set(s, out);
            }
        }
        Some("graduated") => each("classes", out),
        Some("rules") => {
            if let Some(list) = r.get("rules") {
                rules(list, out);
            }
        }
        _ => {}
    }
}

/// Image assets the symbols tile (`assetsOf`): their sizes go to the core,
/// which keeps the images' proportions in tiles.
fn tiled_assets(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Array(list) => list.iter().for_each(|v| tiled_assets(v, out)),
        Value::Object(o) => {
            if o.get("type").and_then(Value::as_str) == Some("imageFill")
                && let Some(asset) = o.get("asset").and_then(Value::as_str)
                && !out.iter().any(|a| a == asset)
            {
                out.push(asset.to_owned());
            }
            o.values().for_each(|v| tiled_assets(v, out));
        }
        _ => {}
    }
}

/// Sets, colours and own symbols in the order they are first needed, each once.
#[derive(Default)]
struct Interned {
    sets: Vec<Value>,
    set_index: HashMap<String, i32>,
    colors: Vec<String>,
    color_index: HashMap<String, i32>,
    refs: Vec<String>,
    ref_index: HashMap<String, i32>,
    /// The simple look's set by colour.
    simple: HashMap<String, i32>,
}

impl Interned {
    fn set(&mut self, s: Value) -> i32 {
        let key = s.to_string();
        if let Some(&k) = self.set_index.get(&key) {
            return k;
        }
        let k = self.sets.len() as i32;
        self.sets.push(s);
        self.set_index.insert(key, k);
        k
    }

    fn color(&mut self, c: &str) -> i32 {
        if let Some(&k) = self.color_index.get(c) {
            return k;
        }
        let k = self.colors.len() as i32;
        self.colors.push(c.to_owned());
        self.color_index.insert(c.to_owned(), k);
        k
    }

    fn own(&mut self, id: &str) -> i32 {
        if let Some(&k) = self.ref_index.get(id) {
            return k;
        }
        let k = self.refs.len() as i32;
        self.refs.push(id.to_owned());
        self.ref_index.insert(id.to_owned(), k);
        k
    }
}

/// The page's part of a layer build: the program, the objects' numbers and the value table.
pub fn layer_call(style: &LayerStyle, entities: &[&Entity], opts: &BuildOptions) -> LayerCall {
    let mut it = Interned::default();
    let mut objects = Vec::with_capacity(4 * entities.len());
    for e in entities {
        let base = e.base();
        let color = base.color.as_deref().unwrap_or(&style.color);
        let c = it.color(color);
        let s = match it.simple.get(color) {
            Some(&s) => s,
            None => {
                let s = it.set(symbols_of_layer_style(style, color, opts.hairlines));
                it.simple.insert(color.to_owned(), s);
                s
            }
        };
        let (mode, a) = match e {
            Entity::Text(_) => (MODE_SKIP, 0),
            Entity::Dimension(_) => (MODE_DIMENSION, 0),
            Entity::Hatch(h) => (
                MODE_SET,
                it.set(json!({ "fill": hatch_symbol_of(h, color) })),
            ),
            _ => match &base.symbol {
                Some(id) => (MODE_OWN, it.own(id)),
                None if style.renderer.is_none() => (MODE_SET, s),
                None => (MODE_RENDERER, 0),
            },
        };
        objects.extend([mode, a, s, c]);
    }
    let mut used = it.refs.clone();
    if let Some(r) = &style.renderer {
        renderer_refs(r, &mut used);
    }
    let mut seen = HashSet::new();
    let mut symbols = Map::new();
    for id in used {
        if seen.insert(id.clone())
            && let Some(sym) = opts.library.symbol(&id)
        {
            symbols.insert(id, sym.clone());
        }
    }
    let symbols = Value::Object(symbols);
    let sets = Value::Array(it.sets);
    let renderer = style.renderer.clone().unwrap_or(Value::Null);
    let mut tiled = Vec::new();
    tiled_assets(&symbols, &mut tiled);
    tiled_assets(&sets, &mut tiled);
    tiled_assets(&renderer, &mut tiled);
    let mut assets = Map::new();
    for id in tiled {
        if let Some((w, h)) = opts.library.asset(&id).and_then(|a| a.size()) {
            assets.insert(id, json!([w, h]));
        }
    }
    let program = json!({
        "symbols": symbols,
        "renderer": renderer,
        "sets": sets,
        "refs": it.refs,
        "colors": it.colors,
        "assets": assets,
    })
    .to_string();
    LayerCall {
        program,
        objects,
        table: ExprTable::default(),
        reads_index: false,
    }
}

/// Builds one layer: the page's call and the core's batches, in draw order.
pub fn build_layer(
    store: &Store,
    style: &LayerStyle,
    entities: &[&Entity],
    opts: &BuildOptions,
) -> Result<(LayerCall, Batches), String> {
    let mut call = layer_call(style, entities, opts);
    let program = Program::read(&call.program)?;
    call.reads_index = program.needs.index;
    call.table = expr_table(&program.fields, program.needs, entities, opts.layer_name);
    let ids: Vec<f64> = entities.iter().map(|e| f64::from(e.base().id)).collect();
    let objects = LayerObjects {
        ids: &ids,
        objects: &call.objects,
        texts: &call.table.texts,
        text_lens: &call.table.lens,
        numbers: &call.table.numbers,
    };
    let batches = core_build(
        store,
        &program,
        &objects,
        opts.clip.as_ref(),
        opts.origin,
        opts.plot_scale,
        opts.screen,
    )?;
    Ok((call, batches))
}
