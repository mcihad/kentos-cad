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
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::draw::{
    FILL, LINE, MARKER, REVERSED, SOURCE, drawn, measure_record,
};

use super::batch::{BatchSink, Batches};
use super::compile::{Env, Geom, Values, compile_symbol, read_number, read_text, read_truth};
use super::model::{Exprs, Reader, Renderer, Symbol, SymbolRef, SymbolSet, SymbolType, Symbols};
use super::prim::{PrimUnit, PrimitiveList, Sink, StrokeStyle};
use super::resolve::{Resolved, Scale, resolve_renderer};
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
        Geom::Fill(_) => SymbolType::Fill,
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
}

/// What an object draws for the style engine (`drawn`, oriented): None for
/// text, and for a construction line outside `clip`.
pub fn styled_geometry(s: &Shape, clip: Option<&Bounds>, buf: &mut Vec<f64>) -> Option<Geom> {
    buf.clear();
    drawn(s, true, clip, buf);
    let mut r = Record {
        b: buf,
        at: 0,
        shape: s,
    };
    let kind = r.next();
    if kind == MARKER {
        let x = r.next();
        let y = r.next();
        Some(Geom::Marker(Vec2::new(x, y)))
    } else if kind == LINE {
        let count = r.next() as usize;
        let mut paths = Vec::with_capacity(count);
        for k in 0..count {
            let closed = r.next() == 1.0;
            paths.push((r.points(k), closed));
        }
        Some(Geom::Line(paths))
    } else if kind == FILL {
        let count = r.next() as usize;
        Some(Geom::Fill((0..count).map(|k| r.points(k)).collect()))
    } else {
        None
    }
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
    /// The fields any expression reads, in order of first use.
    pub fields: Vec<String>,
    /// Each expression's fields as slots of `fields`.
    slots: Vec<Vec<usize>>,
    pub needs: Needs,
}

fn union(a: Needs, b: Needs) -> Needs {
    Needs {
        measured: a.measured || b.measured,
        vertices: a.vertices || b.vertices,
        kind: a.kind || b.kind,
        layer: a.layer || b.layer,
        label: a.label || b.label,
        index: a.index || b.index,
        id: a.id || b.id,
        scale: a.scale || b.scale,
    }
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
            fields,
            slots,
            needs,
        })
    }

    fn symbol(&self, r: Option<&SymbolRef>) -> Option<&Symbol> {
        self.symbols.get(r?)
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
    };
    let mut sink = BatchSink::new(origin);
    let mut buf = Vec::new();
    for (i, &id) in o.ids.iter().enumerate() {
        let [mode, a, simple, color] = [
            o.objects[4 * i],
            o.objects[4 * i + 1],
            o.objects[4 * i + 2],
            o.objects[4 * i + 3],
        ];
        if mode == MODE_SKIP {
            continue;
        }
        let Some(item) = store.get(id) else {
            continue;
        };
        let geom = styled_geometry(&item.shape, clip, &mut buf);
        if mode == MODE_DIMENSION {
            // Dimensions keep their own hairline look: their layout lines. Drawn at every scale
            // (the TypeScript kept the previous object's rule range here).
            if let Some(Geom::Line(paths)) = &geom {
                let hair = StrokeStyle {
                    color: usize::try_from(color)
                        .ok()
                        .and_then(|c| program.colors.get(c))
                        .cloned()
                        .unwrap_or_default(),
                    opacity: 1.0,
                    width: 0.0,
                    unit: PrimUnit::Px,
                    dash: None,
                    dash_offset: 0.0,
                    cap: "butt".into(),
                    join: "miter".into(),
                    blur: 0.0,
                    level: LEVEL_LINE + 500.0,
                };
                sink.set_scale(Scale::default());
                for (pts, _) in paths {
                    sink.stroke(&hair, pts, false);
                }
            }
            continue;
        }
        let Some(geom) = geom else {
            continue;
        };
        let cls = class_of(&geom);
        let values = RowValues {
            program,
            table: &table,
            i,
        };
        let set_at = |k: i32| usize::try_from(k).ok().and_then(|k| program.sets.get(k));
        let own;
        let sets: Vec<Resolved> = match mode {
            MODE_SET => set_at(a)
                .map(|symbols| Resolved {
                    symbols,
                    scale: Scale::default(),
                })
                .into_iter()
                .collect(),
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
                vec![Resolved {
                    symbols: &own,
                    scale: Scale::default(),
                }]
            }
            MODE_RENDERER => program
                .renderer
                .as_ref()
                .map_or_else(Vec::new, |r| resolve_renderer(r, &values)),
            _ => Vec::new(),
        };
        // No matching rule or category: the renderer leaves the object out (as in QGIS).
        let mut drew = false;
        for r in &sets {
            sink.set_scale(r.scale);
            if let Some(symbol) = program.symbol(slot(r.symbols, cls)) {
                compile_symbol(
                    symbol,
                    &geom,
                    &values,
                    &env,
                    &mut sink,
                    level_base(symbol.kind),
                );
                drew = true;
            } else if cls == SymbolType::Fill {
                // An area without a fill symbol takes the line symbol on its edges.
                let Some(edge) = program.symbol(r.symbols.line.as_ref()) else {
                    continue;
                };
                compile_symbol(edge, &geom, &values, &env, &mut sink, LEVEL_LINE);
                drew = true;
            }
        }
        // Matched, but nothing for this kind of geometry (or the symbol is gone): the simple look, never nothing.
        if let Some(first) = sets.first()
            && !drew
        {
            sink.set_scale(first.scale);
            if let Some(fallback) = set_at(simple).and_then(|s| program.symbol(slot(s, cls))) {
                compile_symbol(fallback, &geom, &values, &env, &mut sink, level_base(cls));
            }
        }
    }
    Ok(sink.finish())
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
    };
    let sink: &mut dyn Sink = &mut out;
    compile_symbol(&symbol, &geom, &values, &env, sink, 0.0);
    Ok(out.json())
}
