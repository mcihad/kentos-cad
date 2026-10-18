//! One layer through the style engine (the web's `buildStyledLayer`,
//! `render/styledLayer.ts`; docs/STYLE.md §6): each object gets its own
//! symbol, else its layer's renderer, else the layer's simple look. This
//! side says how each object is drawn (four numbers: the mode, the set or
//! own symbol, the simple look's set, the colour), gives the core the
//! symbols, sets and assets they refer to and the objects' values for the
//! expressions; the core compiles and packs the batches next to the geometry
//! store (`kentos_style_core::style::build::build_layer`). A block's insert
//! (docs/adr/0144) also says, for each piece of its block, the set its simple
//! look draws the piece with: the piece's own colour and line weight, else
//! the insert's, else the layer's; a hatch piece its pattern.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::{js_max, js_round};
use kentos_geometry_core::store::Store;
use kentos_style_core::style::batch::Batches;
use kentos_style_core::style::build::{
    LayerObjects, MODE_DIMENSION, MODE_IMAGE, MODE_OWN, MODE_POINTCLOUD, MODE_RASTER,
    MODE_RENDERER, MODE_SET, MODE_SKIP, Program, View, ViewFrame, build_layer_in as core_build,
};
use serde_json::{Map, Value, json};

/// The unit a layer's renderer reads geometry in, and the project's: see [`BuildOptions::reading_unit`].
pub use kentos_native_application::expression_unit::{Unit as ReadingUnit, reading_unit};
use kentos_native_application::geometry::{edit_geometry, entity_of};

use crate::library::StyleLibrary;
use crate::simple::{Stroke, hatch_symbol_of, leader_symbols_of, symbols_of_layer_style};
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
    /// What Görünüm kipleri leave out (docs/adr/0195): fills, areas' edges.
    pub view: View,
    /// A view-dependent renderer's view (docs/adr/0213 §3): the view's logical
    /// px per metre in quarter octaves, and the key of a heat map's picture.
    pub frame: Option<(f64, String)>,
    /// The unit the layer's renderer reads geometry in where it compares what
    /// Öznitelikler shows (`$alan`, `$uzunluk`, `$y` … in a category's or a
    /// class's value, a rule's condition, a colour's, a size's, a weight's
    /// number): the project's drawing unit (`reading_unit` of its settings;
    /// docs/adr/0165 §2, the 13 Ekim addendum), so the classes Katman stili made
    /// from the values it listed meet the values the layer draws by. A symbol's
    /// own expressions (data-defined sizes, distances, the geometry generator)
    /// stay in metres, and so does a preview on a sample object
    /// (`ReadingUnit::METRE`).
    pub reading_unit: ReadingUnit,
    /// The layers' looks by id, for a block's pieces that follow their own
    /// layer (`byLayer`, docs/adr/0260 §1); none: they take the insert's.
    pub layer_styles: Option<&'a std::collections::HashMap<String, LayerStyle>>,
    /// The project's line types and line type scale (docs/adr/0313 §3); none: the built-in types as they are.
    pub line_types: Option<&'a crate::line_types::LineTypes>,
}

/// What the page gives the core for a layer: the program's JSON, the
/// objects' four numbers each, the value table. Kept for the fixtures,
/// which hold both platforms to the same call.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerCall {
    pub program: String,
    pub objects: Vec<i32>,
    /// Every insert's pieces' sets, insert after insert (`LayerObjects::pieces`).
    pub pieces: Vec<i32>,
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
        // The thematic renderers (docs/adr/0213): their sets, a cluster's and a displacement's own symbol and inner renderer.
        Some("unclassed" | "proportional" | "bivariate") => {
            for k in ["symbols", "other"] {
                if let Some(s) = r.get(k) {
                    set(s, out);
                }
            }
        }
        Some("dotDensity" | "chart" | "inverted") => {
            if let Some(s) = r.get("symbols") {
                set(s, out);
            }
        }
        Some(kind @ ("cluster" | "displacement")) => {
            let own = if kind == "cluster" {
                "symbol"
            } else {
                "center"
            };
            if let Some(id) = r
                .get(own)
                .and_then(|v| v.get("ref"))
                .and_then(Value::as_str)
            {
                out.push(id.to_owned());
            }
            if let Some(inner) = r.get("renderer") {
                renderer_refs(inner, out);
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
    /// The simple look's set by colour and line weight (its bits).
    simple: HashMap<(String, u64), i32>,
    /// An object's own stroke's set by colour, weight, stroke and pattern (docs/adr/0260, 0313).
    stroked: HashMap<StrokeKey, i32>,
    /// The project's line types (docs/adr/0313 §3).
    lines: crate::line_types::LineTypes,
}

/// An own stroke's key: colour, weight bits, the stroke's line type, scale and transparency bits and the pattern it
/// draws with.
type StrokeKey = (String, u64, (u8, u64, u64), Option<String>);

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

    /// The simple look's set for a colour and a line weight, made once.
    fn simple(&mut self, style: &LayerStyle, color: &str, weight: f64, hairlines: bool) -> i32 {
        let key = (color.to_owned(), weight.to_bits());
        if let Some(&s) = self.simple.get(&key) {
            return s;
        }
        // The layer's pattern and the project's scale (docs/adr/0313 §3).
        let symbols = self.lines.apply(
            style.line_pattern.as_deref(),
            symbols_of_layer_style(style, color, weight, hairlines),
        );
        let s = self.set(symbols);
        self.simple.insert(key, s);
        s
    }

    /// A mask's set (docs/adr/0310 §2), made once for a colour, weight and frame.
    fn wipeout(
        &mut self,
        style: &LayerStyle,
        color: &str,
        weight: f64,
        hairlines: bool,
        frame: bool,
    ) -> i32 {
        let key = (format!("wipeout:{frame}:{color}"), weight.to_bits());
        if let Some(&s) = self.simple.get(&key) {
            return s;
        }
        let s = self.set(crate::simple::symbols_of_wipeout(
            style, color, weight, hairlines, frame,
        ));
        self.simple.insert(key, s);
        s
    }

    /// The simple look's set for an object with its own stroke, made once.
    fn stroked(
        &mut self,
        style: &LayerStyle,
        color: &str,
        weight: f64,
        hairlines: bool,
        stroke: Stroke,
        pattern: Option<&str>,
    ) -> i32 {
        let key = (
            color.to_owned(),
            weight.to_bits(),
            stroke.key(),
            pattern.map(str::to_owned),
        );
        if let Some(&s) = self.stroked.get(&key) {
            return s;
        }
        let s = self.set(crate::simple::symbols_of_object_in(
            style,
            color,
            weight,
            hairlines,
            stroke,
            pattern,
            &self.lines,
        ));
        self.stroked.insert(key, s);
        s
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

/// A layer's bands with masks (docs/adr/0310 §2): the objects come in their drawing order, and within each order a
/// mask is a band of its own between what is drawn before it and what after, so it covers the first (their lines
/// too: a band's fills are not drawn before the lines of the band below) and not the second. The bands are the ranks
/// of (order, place among the order's masks).
fn mask_bands(entities: &[&Entity]) -> Vec<i32> {
    let mut keys = Vec::with_capacity(entities.len());
    let mut last: Option<i32> = None;
    let mut sub = 0u32;
    for e in entities {
        let order = e.base().look.order.unwrap_or(0);
        if last != Some(order) {
            last = Some(order);
            sub = 0;
        }
        if e.base().look.wipeout.is_some() {
            keys.push((order, sub + 1));
            sub += 2;
        } else {
            keys.push((order, sub));
        }
    }
    let mut distinct = keys.clone();
    distinct.sort_unstable();
    distinct.dedup();
    keys.iter()
        .map(|k| distinct.binary_search(k).unwrap_or(0) as i32)
        .collect()
}

/// The page's part of a layer build: the program, the objects' numbers and
/// the value table; `store` holds the inserts' pieces.
pub fn layer_call(
    style: &LayerStyle,
    entities: &[&Entity],
    opts: &BuildOptions,
    store: &Store,
) -> LayerCall {
    let mut it = Interned {
        lines: opts.line_types.cloned().unwrap_or_default(),
        ..Interned::default()
    };
    let mut objects = Vec::with_capacity(4 * entities.len());
    let mut pieces = Vec::new();
    // Most objects take the layer's colour and weight: the last ones' indices are kept, not looked up again.
    let mut last: Option<(&str, u64, i32, i32)> = None;
    for e in entities {
        let base = e.base();
        let color = base.color.as_deref().unwrap_or(&style.color);
        // The object's own weight, else its layer's (docs/adr/0139).
        let weight = base.line_weight.unwrap_or(style.line_weight);
        // Its own line type, pattern scale and transparency (docs/adr/0260).
        let stroke = Stroke::of(&base.look);
        let (c, s) = if let Some(w) = base.look.wipeout {
            // An area's mask (docs/adr/0310 §2): the ground's fill, its frame or none.
            (
                it.color(color),
                it.wipeout(style, color, weight, opts.hairlines, w.frame),
            )
        } else if !stroke.plain() || base.look.line_pattern.is_some() {
            // Its own pattern, else its own built-in type, else its layer's pattern (docs/adr/0313 §3).
            let pattern = base
                .look
                .line_pattern
                .as_deref()
                .or(if stroke.line_type.is_none() {
                    style.line_pattern.as_deref()
                } else {
                    None
                });
            (
                it.color(color),
                it.stroked(style, color, weight, opts.hairlines, stroke, pattern),
            )
        } else {
            match last {
                Some((lc, lw, c, s)) if lc == color && lw == weight.to_bits() => (c, s),
                _ => {
                    let c = it.color(color);
                    let s = it.simple(style, color, weight, opts.hairlines);
                    last = Some((color, weight.to_bits(), c, s));
                    (c, s)
                }
            }
        };
        // A block's insert, a wall, an opening: each piece's own look (docs/adr/0144, 0339, 0341).
        if let Entity::Insert(_)
        | Entity::Wall(_)
        | Entity::Opening(_)
        | Entity::Space(_)
        | Entity::Grid(_)
        | Entity::Column(_)
        | Entity::Stair(_)
        | Entity::Ramp(_)
        | Entity::Tag(_)
        | Entity::ViewMark(_) = e
            && let Some(x) = store
                .get(f64::from(base.id))
                .and_then(|item| item.expanded.as_deref())
        {
            for (i, shape) in x.shapes.iter().enumerate() {
                // A part that follows a layer takes that layer's (docs/adr/0260 §1, 0315 §2): its own, or on the
                // block's layer (`""`) the insert's; a layer the drawing does not have is the insert's too.
                let layers = x.by_layer.get(i).map(Vec::as_slice).unwrap_or_default();
                let layer_of = |bit: u8| -> Option<&LayerStyle> {
                    let (id, _) = layers.iter().find(|(_, m)| m & bit != 0)?;
                    Some(match opts.layer_styles.and_then(|l| l.get(id)) {
                        Some(s) if !id.is_empty() => s,
                        _ => style,
                    })
                };
                let follows = |bit: u8| layer_of(bit).is_some();
                let piece_style =
                    layer_of(kentos_geometry_core::block::BY_LAYER_LINE_TYPE).unwrap_or(style);
                let color = x.colors.get(i).cloned().flatten().or_else(|| {
                    layer_of(kentos_geometry_core::block::BY_LAYER_COLOR).map(|s| s.color.clone())
                });
                let color = color.as_deref().unwrap_or(color_of(base, style));
                let weight = x.weights.get(i).copied().flatten().unwrap_or(
                    match layer_of(kentos_geometry_core::block::BY_LAYER_WEIGHT) {
                        Some(s) => s.line_weight,
                        None => weight,
                    },
                );
                let style = piece_style;
                // Its line type, scale and transparency (docs/adr/0312 §4): its own, else a part that follows its
                // layer that layer's (the piece's style), else the insert's own, else the insert's layer's.
                let own_stroke = x.strokes.get(i).cloned().unwrap_or_default();
                let insert_stroke = Stroke::of(&base.look);
                let piece_stroke = Stroke {
                    line_type: own_stroke
                        .line_type
                        .as_deref()
                        .and_then(kentos_contracts::LineType::from_name)
                        .or_else(|| {
                            (!follows(kentos_geometry_core::block::BY_LAYER_LINE_TYPE))
                                .then_some(insert_stroke.line_type)
                                .flatten()
                        }),
                    scale: own_stroke.scale.unwrap_or(insert_stroke.scale),
                    opacity: match own_stroke.transparency {
                        Some(t) => 1.0 - t,
                        None if follows(kentos_geometry_core::block::BY_LAYER_TRANSPARENCY) => 1.0,
                        None => insert_stroke.opacity,
                    },
                };
                // Its pattern (docs/adr/0313 §3): its own, else none with its own built-in type, else a pattern its layer
                // gives (following it), else the insert's own (BYBLOCK), else none with the insert's own built-in type.
                let piece_pattern: Option<&str> = match own_stroke.pattern.as_deref() {
                    Some(p) => Some(p),
                    None if own_stroke.line_type.is_some() => None,
                    None if follows(kentos_geometry_core::block::BY_LAYER_LINE_TYPE) => {
                        style.line_pattern.as_deref()
                    }
                    None if base.look.line_pattern.is_some() => base.look.line_pattern.as_deref(),
                    None if base.look.line_type.is_some() => None,
                    None => style.line_pattern.as_deref(),
                };
                let hatch = match shape {
                    Shape::Hatch { .. } => {
                        edit_geometry(shape.clone()).map(|g| entity_of(&g, base.clone()))
                    }
                    _ => None,
                };
                pieces.push(match (hatch, shape) {
                    (Some(Entity::Hatch(h)), _) => {
                        it.set(piece_stroke.apply(json!({ "fill": hatch_symbol_of(&h, color) })))
                    }
                    // A leader its own look (docs/adr/0146 §5).
                    (_, Shape::Leader { .. }) => it.set(piece_stroke.apply(leader_symbols_of(
                        style,
                        color,
                        weight,
                        opts.hairlines,
                    ))),
                    _ if !piece_stroke.plain() || piece_pattern.is_some() => it.stroked(
                        style,
                        color,
                        weight,
                        opts.hairlines,
                        piece_stroke,
                        piece_pattern,
                    ),
                    _ => it.simple(style, color, weight, opts.hairlines),
                });
            }
        }
        let (mode, a) = match e {
            Entity::Text(_) => (MODE_SKIP, 0),
            Entity::Dimension(_) => (MODE_DIMENSION, 0),
            // Its own bytes over its frame, its frame a hairline (docs/adr/0192 §3).
            Entity::Image(_) => (MODE_IMAGE, 0),
            // Its tiles over its frame, its frame a hairline (docs/adr/0204 §5).
            Entity::Raster(_) => (MODE_RASTER, 0),
            Entity::PointCloud(_) => (MODE_POINTCLOUD, 0),
            Entity::Hatch(h) => (
                MODE_SET,
                it.set(stroke.apply(json!({ "fill": hatch_symbol_of(h, color) }))),
            ),
            // Its lines in its colour and weight, its arrowhead solid (docs/adr/0146 §5); as a hatch, never another
            // symbol. A table's lines so, its frame's band solid (docs/adr/0184 §2).
            Entity::Leader(_) | Entity::Table(_) => (
                MODE_SET,
                it.set(stroke.apply(leader_symbols_of(style, color, weight, opts.hairlines))),
            ),
            // A wide polyline's band solid in its colour, its edges without width its line, as
            // AutoCAD draws one; its widths are its look (docs/adr/0258 §2).
            Entity::Polyline(p) if p.widths.is_some() => (
                MODE_SET,
                it.set(stroke.apply(leader_symbols_of(style, color, weight, opts.hairlines))),
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
    let mut program = json!({
        "symbols": symbols,
        "renderer": renderer,
        "sets": sets,
        "refs": it.refs,
        "colors": it.colors,
        "assets": assets,
    });
    // Kalınlık off takes a dimension's own weights too (docs/adr/0205 §6); written as the web's page writes it.
    if opts.hairlines {
        program["hairlines"] = json!(true);
    }
    let program = program.to_string();
    LayerCall {
        program,
        objects,
        pieces,
        table: ExprTable::default(),
        reads_index: false,
    }
}

/// An object's colour: its own, else its layer's.
fn color_of<'a>(base: &'a kentos_contracts::EntityBase, style: &'a LayerStyle) -> &'a str {
    base.color.as_deref().unwrap_or(&style.color)
}

/// Builds one layer: the page's call and the core's batches, in draw order.
pub fn build_layer(
    store: &Store,
    style: &LayerStyle,
    entities: &[&Entity],
    opts: &BuildOptions,
) -> Result<(LayerCall, Batches), String> {
    let mut call = layer_call(style, entities, opts, store);
    let program = Program::read_in(&call.program, opts.reading_unit)?;
    call.reads_index = program.needs.index;
    call.table = expr_table(&program.fields, program.needs, entities, opts.layer_name);
    let ids: Vec<f64> = entities.iter().map(|e| f64::from(e.base().id)).collect();
    // The draw order bands (docs/adr/0260 §3), only when an object has its own order or is a mask.
    let bands: Vec<i32> = if entities.iter().any(|e| e.base().look.wipeout.is_some()) {
        mask_bands(entities)
    } else if entities.iter().any(|e| e.base().look.order.is_some()) {
        entities
            .iter()
            .map(|e| e.base().look.order.unwrap_or(0))
            .collect()
    } else {
        Vec::new()
    };
    let objects = LayerObjects {
        bands: &bands,
        ids: &ids,
        objects: &call.objects,
        pieces: &call.pieces,
        texts: &call.table.texts,
        text_lens: &call.table.lens,
        numbers: &call.table.numbers,
    };
    let frame = opts.frame.as_ref().map(|(px_per_m, picture)| ViewFrame {
        px_per_m: *px_per_m,
        picture,
    });
    let batches = core_build(
        store,
        &program,
        &objects,
        opts.clip.as_ref(),
        opts.origin,
        opts.plot_scale,
        opts.screen,
        opts.view,
        frame,
    )?;
    Ok((call, batches))
}
