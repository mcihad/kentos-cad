//! A layer through the style engine in one call (formerly the TypeScript
//! `render/styledLayer.ts`, which now builds the program; docs/STYLE.md
//! §6): each object gets its own symbol, else its layer's renderer, else
//! the layer's simple look; the symbols are compiled and packed into
//! batches. A symbol missing from the library (or none for the object's
//! kind of geometry) falls back to the simple look, so an object never
//! silently disappears; an area without a fill symbol draws its edges with
//! the line symbol. Dimensions keep their hairlines. The geometry comes from the geometry store (curves
//! tessellated, rings oriented), and so do the values `$alan`, `$uzunluk`,
//! `$y`, `$x` read; the page gives the objects' other values as a table.

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::{Shape, is_closed_outline};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::js_max;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::draw::{
    FILL, FILLS, LINE, MARKER, MARKERS, MIXED, REVERSED, SOURCE, drawn, measure_record,
};

use super::batch::{BatchSink, Batches, Picture};
use super::compile::{
    Env, Geom, MM_PER_PX, Values, compile_symbol, read_number, read_text, read_truth, to_drawn,
    to_world,
};
use super::model::{
    ChartKind, Dd, Exprs, Field, Layer, Placement, Reader, Renderer, Symbol, SymbolRef, SymbolSet,
    SymbolType, Symbols, Unit, ViewNeeds,
};
use super::place::{interior_point, line_middle};
use super::prim::{
    Common, FillPaint, Look, MarkerStyle, PrimUnit, PrimitiveList, Sink, StrokeStyle,
};
use super::resolve::{Patch, Resolved, Scale, resolve_renderer};
use super::{charts, dots, groups, heat, inverted, thematic};
use crate::expr::rows::{Layout, RowsInput, Table};
use crate::expr::{Expr, Measured, Needs, Scope, Value};

/// Symbol levels across classes: every fill before any line, every line before any marker.
const LEVEL_FILL: f64 = 0.0;
const LEVEL_LINE: f64 = 1000.0;
const LEVEL_MARKER: f64 = 2000.0;

fn level_base(kind: SymbolType) -> f64 {
    match kind {
        SymbolType::Fill => LEVEL_FILL,
        SymbolType::Line => LEVEL_LINE,
        SymbolType::Marker => LEVEL_MARKER,
    }
}

/// The class of a geometry, as a symbol type.
fn class_of(g: &Geom) -> SymbolType {
    match g {
        Geom::Marker(_) => SymbolType::Marker,
        Geom::Line(_) => SymbolType::Line,
        Geom::Fill(_) | Geom::Fills(_) => SymbolType::Fill,
    }
}

fn slot(set: &SymbolSet, kind: SymbolType) -> Option<&SymbolRef> {
    match kind {
        SymbolType::Marker => set.marker.as_ref(),
        SymbolType::Line => set.line.as_ref(),
        SymbolType::Fill => set.fill.as_ref(),
    }
}

// ── Geometry ───────────────────────────────────────────────────────────

/// The object's own points of path or ring `k`, which a record refers to instead of copying them.
fn own_points(s: &Shape, k: usize) -> Vec<Vec2> {
    match s {
        Shape::Line { a, b } => vec![*a, *b],
        Shape::Polyline { pts, .. } => pts.clone(),
        Shape::Polygon { pts, holes, .. } => {
            if k == 0 {
                pts.clone()
            } else {
                holes
                    .as_ref()
                    .and_then(|h| h.get(k - 1))
                    .map_or_else(Vec::new, |h| h.pts.clone())
            }
        }
        Shape::Hatch { ring, holes, .. } => {
            if k == 0 {
                ring.clone()
            } else {
                holes
                    .as_ref()
                    .and_then(|h| h.get(k - 1))
                    .cloned()
                    .unwrap_or_default()
            }
        }
        _ => Vec::new(),
    }
}

struct Record<'a> {
    b: &'a [f64],
    at: usize,
    shape: &'a Shape,
}

impl Record<'_> {
    fn next(&mut self) -> f64 {
        let x = self.b.get(self.at).copied().unwrap_or(0.0);
        self.at += 1;
        x
    }

    fn points(&mut self, k: usize) -> Vec<Vec2> {
        let n = self.next();
        if n == SOURCE {
            return own_points(self.shape, k);
        }
        if n == REVERSED {
            let mut p = own_points(self.shape, k);
            p.reverse();
            return p;
        }
        let n = n as usize;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let x = self.next();
            let y = self.next();
            out.push(Vec2::new(x, y));
        }
        out
    }

    /// A `LINE` record's paths, each with whether it is closed.
    fn paths(&mut self) -> Vec<(Vec<Vec2>, bool)> {
        let count = self.next() as usize;
        let mut paths = Vec::with_capacity(count.min(1 << 16));
        for k in 0..count {
            let closed = self.next() == 1.0;
            paths.push((self.points(k), closed));
        }
        paths
    }

    /// A `FILL` record's rings: the outer ring first, its holes after.
    fn rings(&mut self) -> Vec<Vec<Vec2>> {
        let count = self.next() as usize;
        (0..count).map(|k| self.points(k)).collect()
    }
}

/// What an object draws for the style engine (`drawn`, oriented): None for
/// text, and for a construction line outside `clip`; a leader's lines (its
/// arrowhead's area is `styled_parts`').
pub fn styled_geometry(s: &Shape, clip: Option<&Bounds>, buf: &mut Vec<f64>) -> Option<Geom> {
    styled_parts(s, clip, buf).into_iter().next()
}

/// `styled_geometry`, every part: one geometry for most objects; a
/// leader's lines, then its filled arrowhead's or dot's area when it has
/// one (`MIXED`, docs/adr/0146 §5), each through its own kind of symbol; a
/// multi-point object's every point (`MARKERS`, docs/adr/0174).
pub fn styled_parts(s: &Shape, clip: Option<&Bounds>, buf: &mut Vec<f64>) -> Vec<Geom> {
    buf.clear();
    drawn(s, true, clip, buf);
    let mut r = Record {
        b: buf,
        at: 0,
        shape: s,
    };
    let kind = r.next();
    if kind == MIXED {
        let lines = Geom::Line(r.paths());
        let rings = r.rings();
        // A leader's one area; a dimension's arrowheads and dots, each an area of its own (docs/adr/0183 §3).
        return match rings.len() {
            0 => vec![lines],
            1 => vec![lines, Geom::Fill(rings)],
            _ => vec![
                lines,
                Geom::Fills(rings.into_iter().map(|r| vec![r]).collect()),
            ],
        };
    }
    if kind == MARKERS {
        // A multi-point object's points, each through the point symbol (docs/adr/0174).
        let n = (r.next() as usize).min(r.b.len() / 2);
        return (0..n)
            .map(|_| {
                let x = r.next();
                let y = r.next();
                Geom::Marker(Vec2::new(x, y))
            })
            .collect();
    }
    let one = if kind == MARKER {
        let x = r.next();
        let y = r.next();
        Some(Geom::Marker(Vec2::new(x, y)))
    } else if kind == LINE {
        Some(Geom::Line(r.paths()))
    } else if kind == FILL {
        Some(Geom::Fill(r.rings()))
    } else if kind == FILLS {
        // A multi-part area's parts, each its rings, their points always given (docs/adr/0143).
        let parts = r.next() as usize;
        let mut out = Vec::with_capacity(parts.min(1 << 16));
        for _ in 0..parts {
            let count = r.next() as usize;
            out.push((0..count).map(|_| r.points(0)).collect());
        }
        Some(Geom::Fills(out))
    } else {
        None
    };
    one.into_iter().collect()
}

// ── The program: a layer's symbols and expressions ─────────────────────

/// How the page says each object is drawn (the first of its four numbers).
pub const MODE_SKIP: i32 = 0;
/// A dimension: its layout lines as hairlines.
pub const MODE_DIMENSION: i32 = 1;
/// A set of the program (a hatch's own pattern, the layer's simple look).
pub const MODE_SET: i32 = 2;
/// The object's own library symbol (by its index in `refs`).
pub const MODE_OWN: i32 = 3;
/// The layer's renderer.
pub const MODE_RENDERER: i32 = 4;
/// A picture: its own bytes over its frame and its frame as a hairline
/// (docs/adr/0192 §3); never a symbol.
pub const MODE_IMAGE: i32 = 5;
/// A raster: its tiles over its frame, drawn by the raster pass, and its
/// frame as a hairline (docs/adr/0204 §5); never a symbol.
pub const MODE_RASTER: i32 = 6;
/// A point cloud: its picture over its plan, drawn by the points pass, and
/// its plan as a hairline (docs/adr/0207 §6); never a symbol.
pub const MODE_POINTCLOUD: i32 = 7;

/// Everything one layer build draws with: the renderer, the symbol sets
/// (the layer's simple look for each colour, hatches' own patterns), the
/// library symbols they and the objects refer to, and the expressions,
/// compiled once. The page builds its table of values from `fields` and
/// `needs`.
pub struct Program {
    exprs: Exprs,
    symbols: Symbols,
    renderer: Option<Renderer>,
    sets: Vec<SymbolSet>,
    refs: Vec<SymbolRef>,
    colors: Vec<String>,
    aspects: HashMap<String, f64>,
    /// Line weights hidden (Kalınlık off): every line one pixel, a
    /// dimension's own weights too (docs/adr/0205 §6).
    hairlines: bool,
    /// The fields any expression reads, in order of first use.
    pub fields: Vec<String>,
    /// Each expression's fields as slots of `fields`.
    slots: Vec<Vec<usize>>,
    pub needs: Needs,
}

fn union(a: Needs, b: Needs) -> Needs {
    a.union(b)
}

/// Image assets' proportions (height over width) from their sizes, `{id: [width, height]}`.
fn aspects_of(v: &Json) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    if let Json::Obj(fields) = v {
        for (id, size) in fields {
            if let Json::Arr(wh) = size
                && let [Json::Num(w), Json::Num(h)] = wh.as_slice()
            {
                out.insert(id.clone(), h / w);
            }
        }
    }
    out
}

impl Program {
    /// `{ symbols: {id: Symbol}, renderer?, sets: [SymbolSet], refs: [id], colors: [Color], assets: {asset: [width, height]} }`.
    pub fn read(text: &str) -> Result<Program, String> {
        let v = Json::parse(text)?;
        let mut exprs = Exprs::default();
        let mut symbols = Symbols::default();
        let mut r = Reader {
            exprs: &mut exprs,
            symbols: &mut symbols,
        };
        r.library(v.get("symbols"));
        let renderer = r.renderer(v.get("renderer"));
        let sets = match v.get("sets") {
            Json::Arr(items) => items.iter().map(|s| r.set(s)).collect(),
            _ => Vec::new(),
        };
        let refs = match v.get("refs") {
            Json::Arr(items) => items
                .iter()
                .map(|id| match id {
                    Json::Str(id) => r.library_ref(id),
                    _ => SymbolRef::Library(None),
                })
                .collect(),
            _ => Vec::new(),
        };
        let colors = match v.get("colors") {
            Json::Arr(items) => items
                .iter()
                .map(|c| match c {
                    Json::Str(c) => c.clone(),
                    _ => String::new(),
                })
                .collect(),
            _ => Vec::new(),
        };
        let aspects = aspects_of(v.get("assets"));
        let hairlines = matches!(v.get("hairlines"), Json::Bool(true));
        let mut fields: Vec<String> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut needs = Needs::default();
        let mut slots = Vec::with_capacity(exprs.list.len());
        for e in &exprs.list {
            let mut s = Vec::new();
            if let Some(e) = e {
                for f in &e.fields {
                    let k = *index.entry(f.clone()).or_insert_with(|| {
                        fields.push(f.clone());
                        fields.len() - 1
                    });
                    s.push(k);
                }
                needs = union(needs, e.needs);
            }
            slots.push(s);
        }
        Ok(Program {
            exprs,
            symbols,
            renderer,
            sets,
            refs,
            colors,
            aspects,
            hairlines,
            fields,
            slots,
            needs,
        })
    }

    fn symbol(&self, r: Option<&SymbolRef>) -> Option<&Symbol> {
        self.symbols.get(r?)
    }

    /// What a build of it depends on beyond the objects (docs/adr/0213 §3).
    pub fn view_needs(&self) -> ViewNeeds {
        self.renderer
            .as_ref()
            .map_or_else(ViewNeeds::default, Renderer::view_needs)
    }

    /// Whether its layer is built whole (its objects drawn together).
    pub fn whole(&self) -> bool {
        self.renderer.as_ref().is_some_and(Renderer::whole)
    }
}

/// An object's values from the page's table.
struct RowValues<'a, 'b> {
    program: &'b Program,
    table: &'b Table<'a>,
    i: usize,
}

impl RowValues<'_, '_> {
    fn eval<R>(&self, expr: usize, read: impl FnOnce(Value<'_>) -> Option<R>) -> Option<R> {
        let e = self.program.exprs.list.get(expr)?.as_ref()?;
        let row = self
            .table
            .row(self.i, self.program.slots.get(expr).map(Vec::as_slice));
        read(e.evaluate(&row))
    }
}

impl Values for RowValues<'_, '_> {
    fn number(&self, expr: usize) -> Option<f64> {
        self.eval(expr, read_number)
    }

    fn text(&self, expr: usize) -> Option<String> {
        self.eval(expr, read_text)
    }

    fn truth(&self, expr: usize) -> Option<bool> {
        self.eval(expr, read_truth)
    }
}

/// The page's part of a layer build: the objects (four numbers each: how
/// it is drawn, the set or symbol, the simple look's set, the colour) and
/// their values for the program's expressions (`expr::rows` layout).
pub struct LayerObjects<'a> {
    pub ids: &'a [f64],
    pub objects: &'a [i32],
    pub texts: &'a str,
    pub text_lens: &'a [i32],
    pub numbers: &'a [f64],
    /// A block's inserts (docs/adr/0144): for every insert among `ids`, in
    /// their order, one set per piece of its block (`Store::block_pieces_json`):
    /// the set its simple look draws that piece with (its own colour and line
    /// weight, else the insert's, else the layer's; a hatch piece its
    /// pattern). An insert is drawn piece by piece, each with the object's
    /// symbols and values; the simple look (and the fallback) by its piece's set.
    pub pieces: &'a [i32],
}

/// What Görünüm kipleri leave out of a layer (docs/adr/0195 §1): the fills
/// and hatches of closed objects (pictures stay), the edges of areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    pub fills: bool,
    pub area_edges: bool,
}

impl Default for View {
    fn default() -> Self {
        View {
            fills: true,
            area_edges: true,
        }
    }
}

impl View {
    /// What of `shape` is left out: its fills (a closed object's: an area, a
    /// circle, a hatch, a closed curve), its strokes (an area's edges).
    fn hides(self, shape: &Shape) -> (bool, bool) {
        (
            !self.fills && is_closed_outline(shape),
            !self.area_edges && matches!(shape, Shape::Polygon { .. }),
        )
    }
}

/// Draws the objects of one layer: the batches in draw order.
pub fn build_layer(
    store: &Store,
    program: &Program,
    o: &LayerObjects,
    clip: Option<&Bounds>,
    origin: Vec2,
    plot_scale: f64,
    screen: bool,
) -> Result<Batches, String> {
    build_layer_with(
        store,
        program,
        o,
        clip,
        origin,
        plot_scale,
        screen,
        View::default(),
    )
}

/// [`build_layer`] as Görünüm kipleri show it (docs/adr/0195).
#[allow(clippy::too_many_arguments)]
pub fn build_layer_with(
    store: &Store,
    program: &Program,
    o: &LayerObjects,
    clip: Option<&Bounds>,
    origin: Vec2,
    plot_scale: f64,
    screen: bool,
    view: View,
) -> Result<Batches, String> {
    build_layer_in(
        store, program, o, clip, origin, plot_scale, screen, view, None,
    )
}

/// A view-dependent build's view (docs/adr/0213 §3): the view's pixels per
/// metre (rounded to quarter octaves by the page), and the key the page
/// keeps a heat map's picture by.
#[derive(Clone, Copy, Debug)]
pub struct ViewFrame<'a> {
    pub px_per_m: f64,
    pub picture: &'a str,
}

/// The most dots a layer draws (docs/adr/0213 §2.4).
pub const MOST_DOTS: u64 = 1_000_000;

/// The thematic renderers' changed symbols (docs/adr/0213 §2.10), each made once a build.
#[derive(Default)]
struct Patches {
    made: HashMap<(usize, u64), Symbol>,
}

impl Patches {
    fn of<'s>(&'s mut self, symbol: &'s Symbol, patch: Patch) -> &'s Symbol {
        let key = match patch {
            Patch::None => return symbol,
            Patch::Color(c) => (1u64 << 40) | u64::from(c),
            Patch::Size { step, unit, .. } => (2u64 << 40) | ((unit as u64) << 8) | u64::from(step),
        };
        let id = std::ptr::from_ref(symbol) as usize;
        self.made.entry((id, key)).or_insert_with(|| match patch {
            Patch::Color(c) => thematic::with_color(symbol, &format!("#{c:06X}")),
            Patch::Size { size, unit, .. } => thematic::with_size(symbol, size, unit),
            Patch::None => symbol.clone(),
        })
    }
}

/// A build's own state: the changed symbols, the dots still allowed, the view's scale.
struct Cx {
    patches: Patches,
    dots_left: u64,
    px_per_m: Option<f64>,
}

impl Cx {
    /// A screen length as metres: by the view's scale, else as paper at the plot scale.
    fn px_world(&self, v: f64, env: &Env<'_>) -> f64 {
        match self.px_per_m {
            Some(p) if p > 0.0 => v / p,
            _ => to_world(v, Unit::Px, env),
        }
    }
}

/// [`build_layer_with`] for a view (docs/adr/0213 §3): a heat map's, a
/// cluster's or a displacement's scale and the heat map's picture.
#[allow(clippy::too_many_arguments)]
pub fn build_layer_in(
    store: &Store,
    program: &Program,
    o: &LayerObjects,
    clip: Option<&Bounds>,
    origin: Vec2,
    plot_scale: f64,
    screen: bool,
    view: View,
    frame: Option<ViewFrame<'_>>,
) -> Result<Batches, String> {
    let n = o.ids.len();
    if o.objects.len() != 4 * n {
        return Err(format!(
            "Katman kurulumu: {n} nesne için {} sayı geldi.",
            o.objects.len()
        ));
    }
    // `$alan`, `$uzunluk`, `$y`, `$x` from the store, for the whole layer, only when read.
    let measures = if program.needs.measured {
        store.measures(o.ids)
    } else {
        Vec::new()
    };
    let table = Table::new(
        RowsInput {
            n,
            texts: o.texts,
            text_lens: o.text_lens,
            numbers: o.numbers,
            measures: &measures,
            scale: plot_scale,
        },
        Layout::new(program.fields.len(), program.needs),
    )?;
    let env = Env {
        plot_scale,
        aspects: &program.aspects,
        screen,
        moment: store.time_window().map(|w| match w {
            kentos_geometry_core::time::Window::Instant(a) => a,
            kentos_geometry_core::time::Window::Range(_, b) => b - 1.0,
        }),
    };
    let mut sink = BatchSink::new(origin);
    let mut buf = Vec::new();
    let mut piece_at = 0usize;
    let mut cx = Cx {
        patches: Patches::default(),
        dots_left: MOST_DOTS,
        px_per_m: frame.map(|f| f.px_per_m),
    };
    let whole = program.renderer.as_ref().filter(|r| r.whole());
    let mut taken = Taken::default();
    for (i, &id) in o.ids.iter().enumerate() {
        let [mode, a, simple, color] = [
            o.objects[4 * i],
            o.objects[4 * i + 1],
            o.objects[4 * i + 2],
            o.objects[4 * i + 3],
        ];
        let Some(item) = store.get(id) else {
            continue;
        };
        // An insert: its pieces, each with its own simple look (docs/adr/0144).
        let pieces = item.expanded.as_ref().map(|x| {
            let sets = o.pieces.get(piece_at..piece_at + x.shapes.len());
            piece_at += x.shapes.len();
            (x, sets)
        });
        if mode == MODE_SKIP {
            continue;
        }
        let values = RowValues {
            program,
            table: &table,
            i,
        };
        // A whole-layer renderer takes the objects it draws together; the rest are drawn as they are.
        if let Some(r) = whole
            && mode == MODE_RENDERER
            && pieces.is_none()
        {
            let (fills, strokes) = view.hides(&item.shape);
            sink.hide(fills, strokes);
            take(
                r,
                &item.shape,
                [a, simple, color],
                i,
                program,
                &values,
                &env,
                clip,
                &mut buf,
                &mut sink,
                &mut cx,
                &mut taken,
            );
            continue;
        }
        let mut one = |shape: &Shape, a: i32, simple: i32, sink: &mut BatchSink| {
            let (fills, strokes) = view.hides(shape);
            sink.hide(fills, strokes);
            draw_object(
                shape,
                [mode, a, simple, color],
                program,
                &values,
                &env,
                clip,
                &mut buf,
                sink,
                &mut cx,
            );
        };
        match pieces {
            Some((x, sets)) => {
                for (k, shape) in x.shapes.iter().enumerate() {
                    let set = sets.and_then(|s| s.get(k)).copied().unwrap_or(simple);
                    one(
                        shape,
                        if mode == MODE_SET { set } else { a },
                        set,
                        &mut sink,
                    );
                }
            }
            None => one(&item.shape, a, simple, &mut sink),
        }
    }
    if let Some(r) = whole {
        sink.hide(false, false);
        draw_whole(
            r, &taken, program, &table, &env, clip, frame, &mut sink, &mut cx,
        );
    }
    Ok(sink.finish())
}

/// One object's geometry (or one piece of an insert's) through its symbols:
/// `[mode, a, simple, color]` as the page gave them for the object.
#[allow(clippy::too_many_arguments)]
fn draw_object(
    shape: &Shape,
    [mode, a, simple, color]: [i32; 4],
    program: &Program,
    values: &RowValues<'_, '_>,
    env: &Env<'_>,
    clip: Option<&Bounds>,
    buf: &mut Vec<f64>,
    sink: &mut BatchSink,
    cx: &mut Cx,
) {
    let parts = styled_parts(shape, clip, buf);
    if mode == MODE_IMAGE {
        draw_image(shape, &parts, color, program, sink);
        return;
    }
    if mode == MODE_RASTER {
        draw_raster(shape, &parts, color, program, env.moment, sink);
        return;
    }
    if mode == MODE_POINTCLOUD {
        draw_pointcloud(shape, &parts, color, program, sink);
        return;
    }
    if mode == MODE_DIMENSION {
        // Dimensions keep their own look: their layout lines, hairlines in the dimension's
        // colour unless its look names the dimension line's or the extension lines' colour,
        // weight and type (docs/adr/0205 §6). Drawn at every scale (the TypeScript kept the
        // previous object's rule range here).
        let ink = usize::try_from(color)
            .ok()
            .and_then(|c| program.colors.get(c))
            .cloned()
            .unwrap_or_default();
        let lines = match shape {
            Shape::Dimension { look, .. } => look.lines.as_deref(),
            _ => None,
        };
        let line = dimension_stroke(
            lines.map(|l| (&l.dim_line_color, l.dim_line_weight, &l.dim_line_type)),
            &ink,
            program.hairlines,
            env,
        );
        let ext = dimension_stroke(
            lines.map(|l| (&l.ext_color, l.ext_weight, &l.ext_line_type)),
            &ink,
            program.hairlines,
            env,
        );
        if let Some(Geom::Line(paths)) = parts.first() {
            // Which of its lines are extension lines: the layout's.
            let exts = kentos_geometry_core::entity::dimension_geom(shape)
                .and_then(|g| kentos_geometry_core::geom::dimension::layout_dimension(&g))
                .map(|l| l.ext)
                .unwrap_or_default();
            sink.set_scale(Scale::default());
            for (i, (pts, _)) in paths.iter().enumerate() {
                sink.stroke(if exts.contains(&i) { &ext } else { &line }, pts, false);
            }
        }
        // Filled arrowheads and dots, solid in the dimension line's colour (docs/adr/0183 §3).
        let solid = FillPaint::Solid {
            color: line.color.clone(),
            opacity: 1.0,
            level: LEVEL_LINE + 500.0,
        };
        for part in parts.iter().skip(1) {
            match part {
                Geom::Fill(rings) => sink.fill(&solid, rings),
                Geom::Fills(areas) => {
                    for rings in areas {
                        sink.fill(&solid, rings);
                    }
                }
                _ => {}
            }
        }
        return;
    }
    // A leader's lines through the line symbol, its arrowhead through the fill symbol (docs/adr/0146 §5).
    for geom in &parts {
        draw_part(geom, [mode, a, simple], program, values, env, sink, cx);
    }
    // Nokta yoğunluğu's dots and Grafik's chart over what the object draws (docs/adr/0213 §2.4, §2.5).
    if mode == MODE_RENDERER {
        match program.renderer.as_ref() {
            Some(Renderer::DotDensity {
                fields,
                dot_value,
                dot_size,
                unit,
                seed,
                ..
            }) => draw_dots(
                &parts, fields, *dot_value, *dot_size, *unit, *seed, values, env, sink, cx,
            ),
            Some(Renderer::Chart {
                kind,
                fields,
                size,
                unit,
                size_by,
                max_value,
                bar_width,
                outline,
                ..
            }) => {
                let chart = Chart {
                    kind: *kind,
                    fields,
                    size: *size,
                    unit: *unit,
                    size_by: *size_by,
                    max_value: *max_value,
                    bar_width: *bar_width,
                    outline: outline.as_ref(),
                };
                draw_chart(&parts, &chart, values, env, sink, cx);
            }
            _ => {}
        }
    }
}

/// A line type's dashes on paper, mm (the layers' simple lines'); none: continuous.
pub fn line_type_dash(name: &str) -> Option<&'static [f64]> {
    match name {
        "dashed" => Some(&[3.0, 1.5]),
        "dashdot" => Some(&[5.0, 1.2, 0.6, 1.2]),
        "dotted" => Some(&[0.6, 1.2]),
        _ => None,
    }
}

/// A dimension's line's stroke (docs/adr/0205 §6): its colour (`#RRGGBB`,
/// else the dimension's `ink`), its weight on paper (mm, else a hairline)
/// and its type's dashes, drawn as the layers' lines are (paper mm at the
/// plot scale, or on the screen); `hairlines`: every weight one pixel.
fn dimension_stroke(
    part: Option<(&Option<String>, Option<f64>, &Option<String>)>,
    ink: &str,
    hairlines: bool,
    env: &Env<'_>,
) -> StrokeStyle {
    use super::compile::{to_drawn, to_px};
    use super::model::Unit;
    let (color, weight, kind) = part.unwrap_or((&None, None, &None));
    let weight = weight.filter(|w| *w > 0.0 && !hairlines);
    let (width, unit) = match weight {
        Some(w) => to_drawn(w, Unit::Mm, env),
        None => (0.0, PrimUnit::Px),
    };
    let dash = kind.as_deref().and_then(line_type_dash).map(|d| {
        d.iter()
            .map(|mm| match unit {
                PrimUnit::Px => to_px(*mm, Unit::Mm),
                PrimUnit::World => to_drawn(*mm, Unit::Mm, env).0,
            })
            .collect()
    });
    StrokeStyle {
        color: color.clone().unwrap_or_else(|| ink.to_owned()),
        opacity: 1.0,
        width,
        unit,
        dash,
        dash_offset: 0.0,
        cap: "butt".into(),
        join: "miter".into(),
        blur: 0.0,
        level: LEVEL_LINE + 500.0,
    }
}

/// A picture (docs/adr/0192 §3): its bytes painted over the part shown, in
/// the order of the drawing, and its frame as a hairline in the object's
/// colour. The picture's key is its asset's or its file's.
fn draw_image(shape: &Shape, parts: &[Geom], color: i32, program: &Program, sink: &mut BatchSink) {
    let Shape::Image {
        p,
        width,
        height,
        rotation,
        mirror,
        asset,
        file,
        opacity,
        ..
    } = shape
    else {
        return;
    };
    let image = match (asset, file) {
        (Some(a), _) => format!("asset:{a}"),
        (None, Some(f)) => format!("file:{f}"),
        (None, None) => return,
    };
    sink.set_scale(Scale::default());
    for part in parts {
        match part {
            Geom::Fill(rings) => sink.fill(
                &FillPaint::Image {
                    image: image.clone(),
                    corner: [p.x, p.y],
                    size: [*width, *height],
                    angle: *rotation,
                    mirror: *mirror == Some(true),
                    opacity: opacity.unwrap_or(1.0),
                    level: LEVEL_FILL,
                },
                rings,
            ),
            Geom::Line(paths) => {
                let ink = usize::try_from(color)
                    .ok()
                    .and_then(|c| program.colors.get(c))
                    .cloned()
                    .unwrap_or_default();
                let hair = StrokeStyle {
                    color: ink,
                    opacity: 1.0,
                    width: 0.0,
                    unit: PrimUnit::Px,
                    dash: None,
                    dash_offset: 0.0,
                    cap: "butt".into(),
                    join: "miter".into(),
                    blur: 0.0,
                    level: LEVEL_LINE,
                };
                for (pts, closed) in paths {
                    sink.stroke(&hair, pts, *closed);
                }
            }
            _ => {}
        }
    }
}

/// A point cloud (docs/adr/0207 §6): its plan filled with the points paint,
/// which the points pass draws as the cloud's own picture (its points with
/// their depth), in the order of the drawing; its plan as a hairline in the
/// object's colour. The paint names its files (`geom::pointcloud::cloud_key`)
/// and look; equal files share their nodes.
fn draw_pointcloud(
    shape: &Shape,
    parts: &[Geom],
    color: i32,
    program: &Program,
    sink: &mut BatchSink,
) {
    let Shape::PointCloud {
        sources,
        style,
        opacity,
        ..
    } = shape
    else {
        return;
    };
    let cloud = kentos_geometry_core::geom::pointcloud::cloud_key(sources);
    let look = kentos_geometry_core::api::json::to_string(style);
    let ink = usize::try_from(color)
        .ok()
        .and_then(|c| program.colors.get(c))
        .cloned()
        .unwrap_or_default();
    sink.set_scale(Scale::default());
    for part in parts {
        match part {
            Geom::Fill(rings) => sink.fill(
                &FillPaint::PointCloud {
                    cloud: cloud.clone(),
                    look: look.clone(),
                    color: ink.clone(),
                    opacity: opacity.unwrap_or(1.0),
                    level: LEVEL_FILL,
                },
                rings,
            ),
            Geom::Line(paths) => {
                let ink = ink.clone();
                let hair = StrokeStyle {
                    color: ink,
                    opacity: 1.0,
                    width: 0.0,
                    unit: PrimUnit::Px,
                    dash: None,
                    dash_offset: 0.0,
                    cap: "butt".into(),
                    join: "miter".into(),
                    blur: 0.0,
                    level: LEVEL_LINE,
                };
                for (pts, closed) in paths {
                    sink.stroke(&hair, pts, *closed);
                }
            }
            _ => {}
        }
    }
}

/// The name a raster's tiles go by (docs/adr/0204 §5, 0243 §5): its file
/// (`asset:<id>`, `file:<path>` or `url:<address>`) and, for a NetCDF variable,
/// `#` and the JSON of what the host opens: the variable, vector and mesh, the
/// slice shown at the slider's `moment`, a mesh's sanal grid (its affine and
/// size). None without a file, or before a followed time dimension's first step.
pub fn raster_key(shape: &Shape, moment: Option<f64>) -> Option<String> {
    let Shape::Raster {
        affine,
        width,
        height,
        asset,
        file,
        url,
        dataset,
        ..
    } = shape
    else {
        return None;
    };
    let base = match (asset, file, url) {
        (Some(a), _, _) => format!("asset:{a}"),
        (None, Some(f), _) => format!("file:{f}"),
        (None, None, Some(u)) => format!("url:{u}"),
        (None, None, None) => return None,
    };
    match dataset {
        None => Some(base),
        Some(d) => {
            let grid = matches!(d.get("mesh"), Json::Str(_)).then_some((affine, [*width, *height]));
            dataset_key(d, moment, grid).map(|part| format!("{base}#{part}"))
        }
    }
}

/// A raster's dataset as its key's part (docs/adr/0243 §5, §7): the JSON of its
/// variable, vector, mesh and the slice shown at the slider's `moment` (a time
/// dimension followed: its last step at or before the moment; none before the first).
fn dataset_key(
    d: &Json,
    moment: Option<f64>,
    grid: Option<(&[f64; 6], [f64; 2])>,
) -> Option<String> {
    let dims: &[Json] = match d.get("dims") {
        Json::Arr(a) => a,
        _ => &[],
    };
    let follow = matches!(d.get("followTime"), Json::Bool(true));
    let mut slice = Vec::with_capacity(dims.len());
    for dim in dims {
        let mut index = match dim.get("index") {
            Json::Num(n) => *n,
            _ => 0.0,
        };
        if let (true, Json::Bool(true), Some(t)) = (follow, dim.get("time"), moment) {
            let values: &[Json] = match dim.get("values") {
                Json::Arr(a) => a,
                _ => &[],
            };
            let at = values
                .iter()
                .take_while(|v| matches!(v, Json::Num(x) if *x <= t))
                .count();
            index = at.checked_sub(1)? as f64;
        }
        slice.push(Json::Num(index));
    }
    let mut part = vec![("variable".to_owned(), d.get("variable").clone())];
    for key in ["vector", "mesh"] {
        if let s @ Json::Str(_) = d.get(key) {
            part.push((key.to_owned(), s.clone()));
        }
    }
    part.push(("slice".to_owned(), Json::Arr(slice)));
    if let Some((affine, size)) = grid {
        part.push((
            "affine".to_owned(),
            Json::Arr(affine.iter().map(|&v| Json::Num(v)).collect()),
        ));
        part.push((
            "size".to_owned(),
            Json::Arr(size.iter().map(|&v| Json::Num(v)).collect()),
        ));
    }
    Some(kentos_geometry_core::api::json::to_string(&Json::Obj(part)))
}

/// A raster (docs/adr/0204 §5): its frame filled with the raster paint,
/// which the raster pass draws as the tiles in view, in the order of the
/// drawing; its frame as a hairline in the object's colour. The paint names
/// its file (`asset:`, `file:` or `url:`) and look; equal ones share their tiles.
fn draw_raster(
    shape: &Shape,
    parts: &[Geom],
    color: i32,
    program: &Program,
    moment: Option<f64>,
    sink: &mut BatchSink,
) {
    let Shape::Raster {
        affine,
        width,
        height,
        asset,
        file,
        url,
        style,
        opacity,
        ..
    } = shape
    else {
        return;
    };
    if asset.is_none() && file.is_none() && url.is_none() {
        return;
    }
    // Before its first step the slider shows nothing of a dataset but its frame.
    let raster = raster_key(shape, moment);
    let look = kentos_geometry_core::api::json::to_string(style);
    let nearest = matches!(style.get("resampling"), Json::Str(r) if r == "nearest");
    sink.set_scale(Scale::default());
    for part in parts {
        match part {
            Geom::Fill(rings) => {
                if let Some(raster) = &raster {
                    sink.fill(
                        &FillPaint::Raster {
                            raster: raster.clone(),
                            look: look.clone(),
                            affine: *affine,
                            size: [*width, *height],
                            nearest,
                            opacity: opacity.unwrap_or(1.0),
                            level: LEVEL_FILL,
                        },
                        rings,
                    )
                }
            }
            Geom::Line(paths) => {
                let ink = usize::try_from(color)
                    .ok()
                    .and_then(|c| program.colors.get(c))
                    .cloned()
                    .unwrap_or_default();
                let hair = StrokeStyle {
                    color: ink,
                    opacity: 1.0,
                    width: 0.0,
                    unit: PrimUnit::Px,
                    dash: None,
                    dash_offset: 0.0,
                    cap: "butt".into(),
                    join: "miter".into(),
                    blur: 0.0,
                    level: LEVEL_LINE,
                };
                for (pts, closed) in paths {
                    sink.stroke(&hair, pts, *closed);
                }
            }
            _ => {}
        }
    }
}

/// One geometry of an object through its symbols, by its kind of geometry.
fn draw_part(
    geom: &Geom,
    [mode, a, simple]: [i32; 3],
    program: &Program,
    values: &RowValues<'_, '_>,
    env: &Env<'_>,
    sink: &mut BatchSink,
    cx: &mut Cx,
) {
    let cls = class_of(geom);
    let set_at = |k: i32| usize::try_from(k).ok().and_then(|k| program.sets.get(k));
    let own;
    let plain = |symbols| Resolved {
        symbols,
        scale: Scale::default(),
        patch: Patch::None,
    };
    let sets: Vec<Resolved> = match mode {
        MODE_SET => set_at(a).map(plain).into_iter().collect(),
        MODE_OWN => {
            let r = usize::try_from(a)
                .ok()
                .and_then(|k| program.refs.get(k))
                .cloned();
            own = match cls {
                SymbolType::Marker => SymbolSet {
                    marker: r,
                    ..SymbolSet::default()
                },
                SymbolType::Line => SymbolSet {
                    line: r,
                    ..SymbolSet::default()
                },
                SymbolType::Fill => SymbolSet {
                    fill: r,
                    ..SymbolSet::default()
                },
            };
            vec![plain(&own)]
        }
        MODE_RENDERER => match program.renderer.as_ref() {
            // A cluster or a displacement without a renderer of its own: the layer's simple look.
            Some(
                Renderer::Cluster { inner: None, .. } | Renderer::Displacement { inner: None, .. },
            ) => set_at(simple).map(plain).into_iter().collect(),
            Some(r) => resolve_renderer(r, values),
            None => Vec::new(),
        },
        _ => Vec::new(),
    };
    // No matching rule or category: the renderer leaves the object out (as in QGIS).
    let mut drew = false;
    for r in &sets {
        sink.set_scale(r.scale);
        // Orantılı sembol on an area: its fill as it is, its marker symbol sized at its inside point (docs/adr/0213 §2.2).
        if matches!(r.patch, Patch::Size { .. }) && cls == SymbolType::Fill {
            if let Some(fill) = program.symbol(r.symbols.fill.as_ref()) {
                compile_symbol(fill, geom, values, env, sink, LEVEL_FILL);
                drew = true;
            }
            if let Some(mark) = program.symbol(r.symbols.marker.as_ref()) {
                let mark = cx.patches.of(mark, r.patch);
                compile_symbol(mark, geom, values, env, sink, LEVEL_MARKER);
                drew = true;
            }
            continue;
        }
        if let Some(symbol) = program.symbol(slot(r.symbols, cls)) {
            let symbol = cx.patches.of(symbol, r.patch);
            compile_symbol(symbol, geom, values, env, sink, level_base(symbol.kind));
            drew = true;
        } else if cls == SymbolType::Fill {
            // An area without a fill symbol takes the line symbol on its edges.
            let Some(edge) = program.symbol(r.symbols.line.as_ref()) else {
                continue;
            };
            let edge = cx.patches.of(edge, r.patch);
            compile_symbol(edge, geom, values, env, sink, LEVEL_LINE);
            drew = true;
        }
    }
    // Matched, but nothing for this kind of geometry (or the symbol is gone): the simple look, never nothing.
    if let Some(first) = sets.first()
        && !drew
    {
        sink.set_scale(first.scale);
        if let Some(fallback) = set_at(simple).and_then(|s| program.symbol(slot(s, cls))) {
            compile_symbol(fallback, geom, values, env, sink, level_base(cls));
        }
    }
}

// ── Thematic renderers (docs/adr/0213) ─────────────────────────────────

/// A colour as the batches take it: `#RRGGBB` (or with alpha), a theme name, else the drawing's ink.
fn color_of(c: &str) -> String {
    match thematic::parse_rgba(c) {
        Some(rgba) => thematic::hex(rgba),
        None if ["ink", "paper", "fg", "fg-dim"]
            .iter()
            .any(|t| c.eq_ignore_ascii_case(t)) =>
        {
            c.to_ascii_lowercase()
        }
        None => "ink".to_owned(),
    }
}

/// A filled, unframed circle (a dot, the default cluster).
fn disc(
    color: String,
    size: f64,
    unit: PrimUnit,
    level: f64,
    stroke: Option<(String, f64)>,
) -> MarkerStyle {
    let (stroke, stroke_width) = match stroke {
        Some((c, w)) => (Some(c), w),
        None => (None, 0.0),
    };
    MarkerStyle {
        look: Look::Shape {
            shape: "circle".into(),
            size,
            height: size,
            fill: Some(color),
            stroke,
            stroke_width,
            params: [0.0, 12.0, std::f64::consts::PI, 0.2],
        },
        common: Common {
            unit,
            opacity: 1.0,
            offset: [0.0, 0.0],
            anchor: "center".into(),
            rotation: 0.0,
            level,
        },
    }
}

/// An object's areas, each its rings.
fn areas_of(parts: &[Geom]) -> Vec<&[Vec<Vec2>]> {
    let mut out = Vec::new();
    for g in parts {
        match g {
            Geom::Fill(rings) => out.push(rings.as_slice()),
            Geom::Fills(list) => out.extend(list.iter().map(Vec::as_slice)),
            _ => {}
        }
    }
    out
}

/// Nokta yoğunluğu (§2.4): every value's dots inside the object's areas.
#[allow(clippy::too_many_arguments)]
fn draw_dots(
    parts: &[Geom],
    fields: &[Field],
    dot_value: f64,
    dot_size: f64,
    unit: Unit,
    seed: u64,
    values: &RowValues<'_, '_>,
    env: &Env<'_>,
    sink: &mut BatchSink,
    cx: &mut Cx,
) {
    let areas = areas_of(parts);
    if areas.is_empty() {
        return;
    }
    let Some(inside) = dots::Inside::new(&areas) else {
        return;
    };
    let area = dots::rings_hash(&areas);
    let (size, size_unit) = to_drawn(dot_size, unit, env);
    sink.set_scale(Scale::default());
    let mut pts = Vec::new();
    for (k, f) in fields.iter().enumerate() {
        let n = dots::count_of(f.expr.and_then(|e| values.number(e)), dot_value) as u64;
        if n == 0 {
            continue;
        }
        let take = n.min(cx.dots_left);
        if take < n {
            sink.drop_dots(n - take);
        }
        if take == 0 {
            continue;
        }
        cx.dots_left -= take;
        pts.clear();
        dots::dots(
            &inside,
            take as usize,
            dots::seed_of(seed, area, k),
            &mut pts,
        );
        let style = disc(
            color_of(&f.color),
            size,
            size_unit,
            LEVEL_MARKER + 600.0 + k as f64 / 64.0,
            None,
        );
        for p in &pts {
            sink.marker(&style, *p, 0.0);
        }
    }
}

/// A chart renderer's settings (§2.5).
struct Chart<'a> {
    kind: ChartKind,
    fields: &'a [Field],
    size: f64,
    unit: Unit,
    size_by: Option<super::model::SizeBy>,
    max_value: f64,
    bar_width: Option<f64>,
    outline: Option<&'a (String, f64)>,
}

/// Where a chart sits: a point, an area's inside point (its largest part's), a line's middle.
fn chart_place(parts: &[Geom]) -> Option<Vec2> {
    match parts.first()? {
        Geom::Marker(p) => Some(*p),
        Geom::Fill(rings) => interior_point(rings),
        Geom::Fills(list) => {
            let largest = list.iter().max_by(|a, b| {
                let area = |r: &Vec<Vec<Vec2>>| {
                    r.first().map_or(0.0, |o| {
                        kentos_geometry_core::geometry::signed_area(o).abs()
                    })
                };
                area(a).total_cmp(&area(b))
            })?;
            interior_point(largest)
        }
        Geom::Line(paths) => line_middle(paths),
    }
}

/// Grafik (§2.5): the object's pie or bars at its place, over its background.
fn draw_chart(
    parts: &[Geom],
    chart: &Chart<'_>,
    values: &RowValues<'_, '_>,
    env: &Env<'_>,
    sink: &mut BatchSink,
    cx: &mut Cx,
) {
    let Some(place) = chart_place(parts) else {
        return;
    };
    let vals: Vec<f64> = chart
        .fields
        .iter()
        .map(|f| {
            f.expr
                .and_then(|e| values.number(e))
                .filter(|v| v.is_finite())
                .unwrap_or(0.0)
        })
        .collect();
    let world = |v: f64| match chart.unit {
        Unit::Px => cx.px_world(v, env),
        _ => to_world(v, chart.unit, env),
    };
    let pieces = match chart.kind {
        ChartKind::Pie => {
            let total: f64 = vals.iter().filter(|v| **v > 0.0).sum();
            let d = match chart.size_by {
                Some(b) => thematic::size_at(
                    thematic::share(total, b.min_value, b.max_value),
                    b.min_size,
                    b.max_size,
                    0.5,
                ),
                None => chart.size,
            };
            charts::pie(place, world(d), &vals)
        }
        ChartKind::Bar | ChartKind::Stacked => {
            let width = chart.bar_width.unwrap_or(chart.size / 4.0);
            let vals: Vec<f64> = if chart.kind == ChartKind::Stacked {
                vals.iter().map(|v| js_max(*v, 0.0)).collect()
            } else {
                vals
            };
            charts::bars(
                chart.kind,
                place,
                world(chart.size),
                world(width),
                chart.max_value,
                &vals,
            )
        }
    };
    if pieces.is_empty() {
        return;
    }
    sink.set_scale(Scale::default());
    for p in &pieces {
        let color = chart
            .fields
            .get(p.field)
            .map_or_else(|| "ink".to_owned(), |f| color_of(&f.color));
        let paint = FillPaint::Solid {
            color,
            opacity: 1.0,
            level: LEVEL_MARKER + 800.0 + p.field as f64 / 64.0,
        };
        // A pie's slice (or its whole circle) is star-shaped from its first point: fanned from it.
        if chart.kind == ChartKind::Pie {
            sink.fan(&paint, &p.ring);
        } else {
            sink.fill(&paint, std::slice::from_ref(&p.ring));
        }
    }
    if let Some((color, width)) = chart.outline
        && *width > 0.0
    {
        let (w, unit) = to_drawn(*width, chart.unit, env);
        let style = StrokeStyle {
            color: color_of(color),
            opacity: 1.0,
            width: w,
            unit,
            dash: None,
            dash_offset: 0.0,
            cap: "butt".into(),
            join: "round".into(),
            blur: 0.0,
            level: LEVEL_MARKER + 900.0,
        };
        for p in &pieces {
            sink.stroke(&style, &p.ring, true);
        }
    }
}

/// A point a cluster or a displacement takes: where, which object, its set and its colour.
#[derive(Clone, Copy)]
struct Taken1 {
    at: Vec2,
    i: usize,
    a: i32,
    simple: i32,
    color: i32,
}

/// What a whole-layer renderer took from the objects.
#[derive(Default)]
struct Taken {
    points: Vec<Taken1>,
    weights: Vec<f64>,
    areas: Vec<(Vec<Vec<Vec2>>, usize)>,
}

/// An object a whole-layer renderer draws together with the others: its
/// points (a heat map, a cluster, a displacement) or its areas (Ters alan);
/// the rest of it is drawn by the renderer of single objects.
#[allow(clippy::too_many_arguments)]
fn take(
    r: &Renderer,
    shape: &Shape,
    [a, simple, color]: [i32; 3],
    i: usize,
    program: &Program,
    values: &RowValues<'_, '_>,
    env: &Env<'_>,
    clip: Option<&Bounds>,
    buf: &mut Vec<f64>,
    sink: &mut BatchSink,
    cx: &mut Cx,
    taken: &mut Taken,
) {
    let mut one = |g: Geom| {
        match (r, g) {
            (Renderer::Heatmap { weight, .. }, Geom::Marker(at)) => {
                // A weight that is no number counts once; one below zero not at all (§2.6).
                let w = match weight.and_then(|e| values.number(e)) {
                    Some(v) if v >= 0.0 => v,
                    Some(_) => 0.0,
                    None => 1.0,
                };
                taken.points.push(Taken1 {
                    at,
                    i,
                    a,
                    simple,
                    color,
                });
                taken.weights.push(w);
            }
            (Renderer::Cluster { .. } | Renderer::Displacement { .. }, Geom::Marker(at)) => {
                taken.points.push(Taken1 {
                    at,
                    i,
                    a,
                    simple,
                    color,
                });
            }
            (Renderer::Cluster { .. } | Renderer::Displacement { .. }, g) => {
                draw_part(
                    &g,
                    [MODE_RENDERER, a, simple],
                    program,
                    values,
                    env,
                    sink,
                    cx,
                );
            }
            (Renderer::Inverted { .. }, Geom::Fill(rings)) => taken.areas.push((rings, i)),
            (Renderer::Inverted { .. }, Geom::Fills(list)) => {
                taken.areas.extend(list.into_iter().map(|rings| (rings, i)));
            }
            _ => {}
        }
    };
    // A single point (a heat map's, a cluster's, a spread's usual object): its place as its record
    // gives it (`drawn` never clips a point), without writing one.
    if let Shape::Point { p, parts: None, .. } = shape {
        one(Geom::Marker(*p));
        return;
    }
    for g in styled_parts(shape, clip, buf) {
        one(g);
    }
}

/// A marker symbol's size on the screen, px: its largest layer's (paper mm
/// at the plot scale through the view's scale, or screen mm when symbols
/// keep their size on the screen; metres through the view's scale).
fn marker_px(symbol: &Symbol, env: &Env<'_>, cx: &Cx) -> f64 {
    let ppm = cx.px_per_m;
    symbol
        .layers
        .iter()
        .filter_map(|l| match l {
            Layer::Marker(m) => {
                let v = match &m.size {
                    Some(Dd::Fixed(x)) => *x,
                    Some(Dd::Expr { fallback, .. }) => fallback.unwrap_or(2.0),
                    None => 2.0,
                };
                Some(match m.base.unit {
                    Unit::Px => v,
                    Unit::Mm if env.screen || ppm.is_none() => v / MM_PER_PX,
                    Unit::Mm => v / 1000.0 * env.plot_scale * ppm.unwrap_or(0.0),
                    Unit::M => v * ppm.unwrap_or(0.0),
                })
            }
            _ => None,
        })
        .fold(0.0, js_max)
}

/// The marker symbol an object's point draws with: its renderer's (or a
/// wrapper's inner one), else the layer's simple look.
fn marker_of<'p>(
    program: &'p Program,
    values: &RowValues<'_, '_>,
    simple: i32,
) -> Option<&'p Symbol> {
    let from_renderer = match program.renderer.as_ref() {
        Some(
            Renderer::Cluster { inner: Some(r), .. }
            | Renderer::Displacement { inner: Some(r), .. },
        ) => resolve_renderer(r, values)
            .into_iter()
            .find_map(|x| program.symbol(x.symbols.marker.as_ref())),
        _ => None,
    };
    from_renderer.or_else(|| {
        usize::try_from(simple)
            .ok()
            .and_then(|k| program.sets.get(k))
            .and_then(|s| program.symbol(s.marker.as_ref()))
    })
}

/// A circle of `r` metres round `c`, as a closed path of 72 points.
fn circle_path(c: Vec2, r: f64) -> Vec<Vec2> {
    (0..72)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / 72.0;
            Vec2::new(
                c.x + r * kentos_geometry_core::jsmath::cos(a),
                c.y + r * kentos_geometry_core::jsmath::sin(a),
            )
        })
        .collect()
}

/// The whole-layer renderers' drawing once every object was seen (§2.6–§2.9).
#[allow(clippy::too_many_arguments)]
fn draw_whole(
    r: &Renderer,
    taken: &Taken,
    program: &Program,
    table: &Table<'_>,
    env: &Env<'_>,
    clip: Option<&Bounds>,
    frame: Option<ViewFrame<'_>>,
    sink: &mut BatchSink,
    cx: &mut Cx,
) {
    let values_of = |i: usize| RowValues { program, table, i };
    let distance = |v: f64, unit: Unit, cx: &Cx| match unit {
        Unit::M => v,
        _ => cx.px_world(v, env),
    };
    match r {
        Renderer::Heatmap {
            radius,
            unit,
            max,
            ramp,
            quality,
            opacity,
            ..
        } => {
            let (Some(f), Some(b)) = (frame, clip) else {
                return;
            };
            let Some(grid) = heat::Grid::over(b, f.px_per_m, *quality) else {
                return;
            };
            let radius_px = match unit {
                Unit::M => radius * f.px_per_m,
                _ => *radius,
            };
            let cells = kentos_geometry_core::jsmath::js_round(radius_px / f64::from(*quality));
            let r_cells = if cells.is_finite() {
                js_max(cells, 1.0) as i64
            } else {
                1
            };
            let points: Vec<(Vec2, f64)> = taken
                .points
                .iter()
                .zip(&taken.weights)
                .map(|(p, w)| (p.at, *w))
                .collect();
            let v = heat::values(&grid, &points, r_cells);
            let top = max.unwrap_or_else(|| heat::largest(&v));
            let rgba = heat::colors(&v, top, &heat::table(ramp, *opacity));
            let (w, h) = (
                grid.width as f64 * grid.cell,
                grid.height as f64 * grid.cell,
            );
            let (x0, y0) = (grid.min_x, grid.max_y - h);
            sink.picture(Picture {
                key: f.picture.to_owned(),
                width: grid.width as u32,
                height: grid.height as u32,
                rgba,
            });
            sink.set_scale(Scale::default());
            sink.fill(
                &FillPaint::Image {
                    image: f.picture.to_owned(),
                    corner: [x0, y0],
                    size: [w, h],
                    angle: 0.0,
                    mirror: false,
                    opacity: 1.0,
                    level: LEVEL_FILL,
                },
                &[vec![
                    Vec2::new(x0, y0),
                    Vec2::new(x0 + w, y0),
                    Vec2::new(x0 + w, y0 + h),
                    Vec2::new(x0, y0 + h),
                ]],
            );
        }
        Renderer::Cluster {
            distance: d,
            unit,
            symbol,
            count,
            grow,
            ..
        } => {
            let at: Vec<Vec2> = taken.points.iter().map(|p| p.at).collect();
            for g in groups::group(&at, distance(*d, *unit, cx)) {
                let first = taken.points[g.members[0]];
                let values = values_of(first.i);
                if g.members.len() == 1 {
                    draw_part(
                        &Geom::Marker(first.at),
                        [MODE_RENDERER, first.a, first.simple],
                        program,
                        &values,
                        env,
                        sink,
                        cx,
                    );
                    continue;
                }
                let n = g.members.len();
                let c = g.centre();
                let k = if *grow { groups::growth(n) } else { 1.0 };
                sink.set_scale(Scale::default());
                // The count's height: 0.45 of the symbol's size, in its unit.
                let (text_size, text_unit) = match program.symbol(symbol.as_ref()) {
                    Some(sym) => {
                        let px = marker_px(sym, env, cx);
                        let sym = if k == 1.0 {
                            sym
                        } else {
                            cx.patches.of(
                                sym,
                                Patch::Size {
                                    size: px * k,
                                    unit: Unit::Px,
                                    step: kentos_geometry_core::jsmath::js_round(k * 100.0) as u8,
                                },
                            )
                        };
                        compile_symbol(
                            sym,
                            &Geom::Marker(c),
                            &values,
                            env,
                            sink,
                            LEVEL_MARKER + 700.0,
                        );
                        (0.45 * px * k, PrimUnit::Px)
                    }
                    None => {
                        let ink = usize::try_from(first.color)
                            .ok()
                            .and_then(|k| program.colors.get(k))
                            .cloned()
                            .unwrap_or_else(|| "ink".to_owned());
                        let size = 24.0 * k;
                        sink.marker(
                            &disc(
                                ink,
                                size,
                                PrimUnit::Px,
                                LEVEL_MARKER + 700.0,
                                Some(("#FFFFFF".to_owned(), 1.5)),
                            ),
                            c,
                            0.0,
                        );
                        (0.45 * size, PrimUnit::Px)
                    }
                };
                if *count {
                    sink.marker(
                        &MarkerStyle {
                            look: Look::Text {
                                text: n.to_string(),
                                size: text_size,
                                font: "ui".into(),
                                weight: 700.0,
                                italic: false,
                                color: "#FFFFFF".into(),
                                halo: None,
                            },
                            common: Common {
                                unit: text_unit,
                                opacity: 1.0,
                                offset: [0.0, 0.0],
                                anchor: "center".into(),
                                rotation: 0.0,
                                level: LEVEL_MARKER + 701.0,
                            },
                        },
                        c,
                        0.0,
                    );
                }
            }
        }
        Renderer::Displacement {
            tolerance,
            unit,
            placement,
            spacing,
            center,
            circle,
            ..
        } => {
            let at: Vec<Vec2> = taken.points.iter().map(|p| p.at).collect();
            let centre_symbol = program.symbol(center.as_ref());
            for g in groups::group(&at, distance(*tolerance, *unit, cx)) {
                if g.members.len() == 1 {
                    let p = taken.points[g.members[0]];
                    draw_part(
                        &Geom::Marker(p.at),
                        [MODE_RENDERER, p.a, p.simple],
                        program,
                        &values_of(p.i),
                        env,
                        sink,
                        cx,
                    );
                    continue;
                }
                let c = g.centre();
                // The largest symbol's diagonal (at least 4 px) and the centre symbol's.
                let s = g
                    .members
                    .iter()
                    .map(|&m| {
                        let p = taken.points[m];
                        marker_of(program, &values_of(p.i), p.simple)
                            .map_or(0.0, |sym| marker_px(sym, env, cx))
                    })
                    .fold(0.0, js_max)
                    * std::f64::consts::SQRT_2;
                let s = js_max(s, 4.0);
                let csize = centre_symbol.map_or(0.0, |sym| {
                    marker_px(sym, env, cx) * std::f64::consts::SQRT_2
                });
                let (offsets, rings) =
                    groups::displaced(*placement, g.members.len(), s, csize, *spacing);
                sink.set_scale(Scale::default());
                if let Some((color, width)) = circle
                    && *placement != Placement::Grid
                    && *width > 0.0
                {
                    let style = StrokeStyle {
                        color: color_of(color),
                        opacity: 1.0,
                        width: *width,
                        unit: PrimUnit::Px,
                        dash: None,
                        dash_offset: 0.0,
                        cap: "butt".into(),
                        join: "round".into(),
                        blur: 0.0,
                        level: LEVEL_MARKER - 10.0,
                    };
                    for r in rings {
                        sink.stroke(&style, &circle_path(c, cx.px_world(r, env)), true);
                    }
                }
                let first = taken.points[g.members[0]];
                if let Some(sym) = centre_symbol {
                    compile_symbol(
                        sym,
                        &Geom::Marker(c),
                        &values_of(first.i),
                        env,
                        sink,
                        LEVEL_MARKER,
                    );
                }
                for (k, &m) in g.members.iter().enumerate() {
                    let p = taken.points[m];
                    let off = offsets.get(k).copied().unwrap_or(Vec2::new(0.0, 0.0));
                    let at =
                        Vec2::new(c.x + cx.px_world(off.x, env), c.y + cx.px_world(off.y, env));
                    draw_part(
                        &Geom::Marker(at),
                        [MODE_RENDERER, p.a, p.simple],
                        program,
                        &values_of(p.i),
                        env,
                        sink,
                        cx,
                    );
                }
            }
        }
        Renderer::Inverted { symbols, merge } => {
            let Some(fill) = program.symbol(symbols.fill.as_ref()) else {
                return;
            };
            let areas: Vec<&[Vec<Vec2>]> = taken.areas.iter().map(|(r, _)| r.as_slice()).collect();
            // The box: the build's, else the areas' own and a tenth round it (previews).
            let b = match clip {
                Some(b) => *b,
                None => {
                    let mut b = kentos_geometry_core::geometry::empty_bounds();
                    for rings in &areas {
                        for ring in rings.iter() {
                            for p in ring {
                                kentos_geometry_core::geometry::extend_bounds(&mut b, *p, 0.0);
                            }
                        }
                    }
                    if !(b.max_x > b.min_x && b.max_y > b.min_y) {
                        return;
                    }
                    let m = 0.1 * js_max(b.max_x - b.min_x, b.max_y - b.min_y);
                    Bounds {
                        min_x: b.min_x - m,
                        min_y: b.min_y - m,
                        max_x: b.max_x + m,
                        max_y: b.max_y + m,
                    }
                }
            };
            let rule = if *merge {
                inverted::Rule::NonZero
            } else {
                inverted::Rule::EvenOdd
            };
            // The fill's paints on the region (a gradient as its first colour: each piece would
            // have its own), its lines on the areas' rings only.
            let fills = Symbol {
                kind: SymbolType::Fill,
                layers: fill
                    .layers
                    .iter()
                    .filter_map(|l| match l {
                        Layer::SimpleFill { .. }
                        | Layer::HatchFill(_)
                        | Layer::PatternFill(_)
                        | Layer::ImageFill { .. } => Some(l.clone()),
                        Layer::GradientFill(g) => Some(Layer::SimpleFill {
                            base: g.base.clone(),
                            color: g.color.clone(),
                        }),
                        _ => None,
                    })
                    .collect(),
            };
            let edges = Symbol {
                kind: SymbolType::Fill,
                layers: fill
                    .layers
                    .iter()
                    .filter(|l| matches!(l, Layer::SimpleLine(_) | Layer::MarkerLine(_)))
                    .cloned()
                    .collect(),
            };
            let values = values_of(taken.areas.first().map_or(0, |(_, i)| *i));
            sink.set_scale(Scale::default());
            if !fills.layers.is_empty() {
                for t in inverted::region(&b, &areas, rule) {
                    compile_symbol(
                        &fills,
                        &Geom::Fill(vec![t.to_vec()]),
                        &values,
                        env,
                        sink,
                        LEVEL_FILL,
                    );
                }
            }
            if !edges.layers.is_empty() {
                for (rings, i) in &taken.areas {
                    compile_symbol(
                        &edges,
                        &Geom::Fill(rings.clone()),
                        &values_of(*i),
                        env,
                        sink,
                        LEVEL_FILL,
                    );
                }
            }
        }
        _ => {}
    }
}

// ── One symbol on one object (previews, legends, tests) ────────────────

/// An object's values from its own attributes (previews).
struct ObjectScope<'a> {
    expr: &'a Expr,
    attrs: &'a Json,
    label: Option<&'a str>,
    layer: &'a str,
    kind: &'a str,
    id: f64,
    vertices: Option<f64>,
    measured: Measured,
    scale: f64,
}

impl Scope for ObjectScope<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        match self.attrs.get(self.expr.fields.get(i)?) {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    fn measured(&self) -> Measured {
        self.measured
    }

    fn vertices(&self) -> Option<f64> {
        self.vertices
    }

    fn kind(&self) -> &str {
        self.kind
    }

    fn layer(&self) -> &str {
        self.layer
    }

    fn label(&self) -> Option<&str> {
        self.label
    }

    fn index(&self) -> f64 {
        1.0
    }

    fn id(&self) -> f64 {
        self.id
    }

    fn scale(&self) -> Option<f64> {
        Some(self.scale)
    }
}

struct ObjectValues<'a> {
    exprs: &'a Exprs,
    attrs: &'a Json,
    label: Option<&'a str>,
    layer: &'a str,
    kind: &'a str,
    id: f64,
    vertices: Option<f64>,
    measured: Measured,
    scale: f64,
}

impl ObjectValues<'_> {
    fn eval<R>(&self, expr: usize, read: impl FnOnce(Value<'_>) -> Option<R>) -> Option<R> {
        let e = self.exprs.list.get(expr)?.as_ref()?;
        let scope = ObjectScope {
            expr: e,
            attrs: self.attrs,
            label: self.label,
            layer: self.layer,
            kind: self.kind,
            id: self.id,
            vertices: self.vertices,
            measured: self.measured,
            scale: self.scale,
        };
        read(e.evaluate(&scope))
    }
}

impl Values for ObjectValues<'_> {
    fn number(&self, expr: usize) -> Option<f64> {
        self.eval(expr, read_number)
    }

    fn text(&self, expr: usize) -> Option<String> {
        self.eval(expr, read_text)
    }

    fn truth(&self, expr: usize) -> Option<bool> {
        self.eval(expr, read_truth)
    }
}

/// `styleCompile`: one symbol on one object, as primitives (JSON). Input:
/// `{ symbol, entity, layerName, kindLabel, vertices, plotScale, assets }`.
pub fn compile_one(v: &Json) -> Result<String, String> {
    use kentos_geometry_core::api::json::FromJson;
    use kentos_geometry_core::entity::Entity;
    let mut exprs = Exprs::default();
    let Some(symbol) = exprs.symbol(v.get("symbol")) else {
        return Err("Sembol okunamadı.".into());
    };
    let entity = Entity::from_json(v.get("entity"))?;
    let mut buf = Vec::new();
    let geom = match entity.shape {
        // Text and dimensions are drawn elsewhere (a layer gives dimensions their hairlines).
        Shape::Text { .. } | Shape::Dimension { .. } => None,
        _ => styled_geometry(&entity.shape, None, &mut buf),
    };
    let mut out = PrimitiveList::default();
    let Some(geom) = geom else {
        return Ok(out.json());
    };
    let rest = |k: &str| {
        entity
            .rest
            .iter()
            .find(|(name, _)| name == k)
            .map(|(_, v)| v)
    };
    let mut m = Vec::new();
    measure_record(Some(&entity.shape), &mut m);
    let flags = m[0] as u32;
    fn str_of(v: &Json) -> &str {
        match v {
            Json::Str(s) => s.as_str(),
            _ => "",
        }
    }
    let values = ObjectValues {
        exprs: &exprs,
        attrs: rest("attrs").unwrap_or(&Json::Null),
        label: rest("label").and_then(|l| match l {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }),
        layer: str_of(v.get("layerName")),
        kind: str_of(v.get("kindLabel")),
        id: match rest("id") {
            Some(Json::Num(x)) => *x,
            _ => f64::NAN,
        },
        vertices: match v.get("vertices") {
            Json::Num(x) => Some(*x),
            _ => None,
        },
        measured: Measured {
            length: (flags & 1 != 0).then_some(m[1]),
            area: (flags & 2 != 0).then_some(m[2]),
            anchor: (flags & 4 != 0).then_some((m[3], m[4])),
        },
        scale: match v.get("plotScale") {
            Json::Num(x) => *x,
            _ => 1000.0,
        },
    };
    let aspects = aspects_of(v.get("assets"));
    // A single symbol (previews, legend) is drawn at its paper size.
    let env = Env {
        plot_scale: values.scale,
        aspects: &aspects,
        screen: false,
        moment: None,
    };
    let sink: &mut dyn Sink = &mut out;
    compile_symbol(&symbol, &geom, &values, &env, sink, 0.0);
    Ok(out.json())
}

#[cfg(test)]
mod raster_key_tests {
    use super::dataset_key;
    use kentos_geometry_core::api::json::Json;

    /// The key's part as the web writes it too (`apps/web/src/model/rasterRules.ts`
    /// `rasterKey`, its test `rasterKey.test.ts`): the same text for the same slice.
    #[test]
    fn a_datasets_part_is_its_variable_mesh_and_slice() {
        let grid = Json::parse(
            r#"{"variable":"t2m","dims":[{"name":"time","index":1,"values":[0,3600000,7200000],"time":true},{"name":"level","index":0,"values":[850,500]}],"followTime":true}"#,
        )
        .unwrap();
        assert_eq!(
            dataset_key(&grid, None, None).unwrap(),
            r#"{"variable":"t2m","slice":[1,0]}"#
        );
        // Following the slider: the last step at or before the moment; none before the first.
        assert_eq!(
            dataset_key(&grid, Some(7_199_999.0), None).unwrap(),
            r#"{"variable":"t2m","slice":[1,0]}"#
        );
        assert_eq!(
            dataset_key(&grid, Some(7_200_000.0), None).unwrap(),
            r#"{"variable":"t2m","slice":[2,0]}"#
        );
        assert!(dataset_key(&grid, Some(-1.0), None).is_none());
        let mesh = Json::parse(r#"{"variable":"ucx","vector":"ucy","mesh":"mesh","dims":[{"name":"time","index":0,"values":[0,1800000],"time":true}]}"#).unwrap();
        let affine = [500000.0, 0.125, 0.0, 4420040.0, 0.0, -0.125];
        assert_eq!(
            dataset_key(&mesh, None, Some((&affine, [320.0, 320.0]))).unwrap(),
            r#"{"variable":"ucx","vector":"ucy","mesh":"mesh","slice":[0],"affine":[500000,0.125,0,4420040,0,-0.125],"size":[320,320]}"#
        );
    }
}
