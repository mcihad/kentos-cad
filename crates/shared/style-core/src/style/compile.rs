//! Symbol × geometry × object → drawing primitives (formerly the TypeScript
//! `style/compile.ts`, now its facade): units are converted (mm on paper → metres at the
//! plot scale), data-defined values are evaluated, parallel offsets and
//! marker places along lines are computed in float64.

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::offset::offset_path;
use kentos_geometry_core::jsmath::{PI, cos, js_max, js_min, js_round, sin};

use super::model::{
    Base, Dd, Layer, MarkerKind, MarkerLayer, MarkerLine, SimpleLine, Symbol, SymbolType, Unit,
};
use super::place::{
    PlaceGroup, WaveSpec, centroid_of, interior_point, line_middle, place_along, positive,
    wave_paths,
};
use super::prim::{Common, FillPaint, Look, MarkerStyle, PrimUnit, Sink, StrokeStyle, Tile};
use crate::expr::Value;
use crate::expr::value::{into_text, to_number, truthy};
use crate::js::text::{trim, utf16_len};

/// A CSS pixel on paper (96 dpi), where a length in px must become geometry.
pub const MM_PER_PX: f64 = 25.4 / 96.0;

const DEG: f64 = PI / 180.0;

/// What a symbol is drawn on: a point, lines, or an area (outer ring
/// counter-clockwise, holes clockwise: left of a ring is inside).
#[derive(Clone, Debug, PartialEq)]
pub enum Geom {
    Marker(Vec2),
    /// Paths and whether each is closed.
    Line(Vec<(Vec<Vec2>, bool)>),
    Fill(Vec<Vec<Vec2>>),
}

/// The object's values for its symbol's expressions, as each use reads them.
pub trait Values {
    /// As a number (None: empty, or not a number).
    fn number(&self, expr: usize) -> Option<f64>;
    /// As text (None: empty).
    fn text(&self, expr: usize) -> Option<String>;
    /// As true or false (None: empty).
    fn truth(&self, expr: usize) -> Option<bool>;
}

/// The three readings of an expression's value (an expression that does not compile is empty).
pub fn read_number(v: Value<'_>) -> Option<f64> {
    to_number(&v)
}

pub fn read_text(v: Value<'_>) -> Option<String> {
    match v {
        Value::Null => None,
        v => Some(into_text(v).into_owned()),
    }
}

pub fn read_truth(v: Value<'_>) -> Option<bool> {
    match v {
        Value::Null => None,
        v => Some(truthy(&v)),
    }
}

pub struct Env<'a> {
    /// Denominator of the project's plot scale (1000 for 1:1000).
    pub plot_scale: f64,
    /// Height/width of image assets (tiles keep their proportions); 1 when unknown.
    pub aspects: &'a HashMap<String, f64>,
    /// Symbol sizes on the screen (Uygulama ayarları → Sembol boyutu → Ekranda sabit): paper millimetres
    /// of drawn sizes (widths, dashes, marker and hatch sizes) become CSS px for the shader, so they stay
    /// the same while the view zooms; lengths that place geometry still follow `plot_scale` (the view's).
    pub screen: bool,
}

// ── Data-defined values ────────────────────────────────────────────────

fn dd_number(v: &Option<Dd<f64>>, t: &dyn Values, fallback: f64) -> f64 {
    match v {
        None => fallback,
        Some(Dd::Fixed(x)) => *x,
        Some(Dd::Expr { expr, fallback: f }) => {
            expr.and_then(|e| t.number(e)).or(*f).unwrap_or(fallback)
        }
    }
}

/// The value as text (None: empty).
fn text_of(t: &dyn Values, expr: Option<usize>) -> Option<String> {
    t.text(expr?)
}

fn dd_text(v: &Option<Dd<String>>, t: &dyn Values) -> String {
    match v {
        None => String::new(),
        Some(Dd::Fixed(s)) => s.clone(),
        Some(Dd::Expr { expr, fallback }) => text_of(t, *expr)
            .or_else(|| fallback.clone())
            .unwrap_or_default(),
    }
}

fn dd_bool(v: &Option<Dd<bool>>, t: &dyn Values) -> bool {
    match v {
        None => true,
        Some(Dd::Fixed(b)) => *b,
        Some(Dd::Expr { expr, fallback }) => match expr.and_then(|e| t.truth(e)) {
            None => fallback.unwrap_or(true),
            Some(b) => b,
        },
    }
}

/// `/^(#[0-9a-f]{6}([0-9a-f]{2})?|ink|paper|fg|fg-dim)$/i`.
fn is_color(s: &str) -> bool {
    match s.strip_prefix('#') {
        Some(hex) => {
            (hex.len() == 6 || hex.len() == 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
        }
        None => ["ink", "paper", "fg", "fg-dim"]
            .iter()
            .any(|c| s.eq_ignore_ascii_case(c)),
    }
}

fn dd_color(v: &Option<Dd<String>>, t: &dyn Values) -> Option<String> {
    match v {
        None => None,
        Some(Dd::Fixed(s)) => Some(s.clone()),
        Some(Dd::Expr { expr, fallback }) => {
            let r = text_of(t, *expr).unwrap_or_default();
            let r = trim(&r);
            if is_color(r) {
                Some(r.to_string())
            } else {
                fallback.clone()
            }
        }
    }
}

// ── Units ──────────────────────────────────────────────────────────────

/// A length that changes geometry (offset, interval, spacing) in metres.
pub fn to_world(v: f64, unit: Unit, env: &Env) -> f64 {
    if unit == Unit::M {
        return v;
    }
    let mm = if unit == Unit::Px { v * MM_PER_PX } else { v };
    (mm * env.plot_scale) / 1000.0
}

/// Whether drawn sizes in `unit` go to the shader in px: px always, paper mm with screen-sized symbols.
pub fn drawn_in_px(unit: Unit, env: &Env) -> bool {
    unit == Unit::Px || (env.screen && unit == Unit::Mm)
}

/// A drawn size (width, marker size, dash): px (and with screen-sized symbols paper mm) goes to the shader
/// in px, the rest becomes metres.
pub fn to_drawn(v: f64, unit: Unit, env: &Env) -> (f64, PrimUnit) {
    if drawn_in_px(unit, env) {
        (to_px(v, unit), PrimUnit::Px)
    } else {
        (to_world(v, unit, env), PrimUnit::World)
    }
}

/// A px or paper-mm length in CSS px (96 per inch).
pub fn to_px(v: f64, unit: Unit) -> f64 {
    if unit == Unit::Mm { v / MM_PER_PX } else { v }
}

/// JavaScript's truthiness of a number: not 0 and not NaN.
fn truthy_num(x: f64) -> bool {
    x != 0.0 && !x.is_nan()
}

// ── Markers ────────────────────────────────────────────────────────────

pub fn marker_style(
    layer: &MarkerLayer,
    level: f64,
    t: &dyn Values,
    env: &Env,
) -> Option<MarkerStyle> {
    if !dd_bool(&layer.base.enabled, t) {
        return None;
    }
    let unit = layer.base.unit;
    let (size, size_unit) = to_drawn(dd_number(&layer.size, t, 2.0), unit, env);
    let off = layer.offset.unwrap_or([0.0, 0.0]);
    let conv = |v: f64| {
        if size_unit == PrimUnit::Px {
            to_px(v, unit)
        } else {
            to_world(v, unit, env)
        }
    };
    let common = Common {
        unit: size_unit,
        opacity: layer.base.opacity.unwrap_or(1.0),
        offset: [conv(off[0]), conv(off[1])],
        anchor: layer.anchor.clone().unwrap_or_else(|| "center".into()),
        rotation: dd_number(&layer.rotation, t, 0.0) * DEG,
        level,
    };
    let look = match &layer.kind {
        MarkerKind::Shape {
            shape,
            height,
            fill,
            stroke,
            stroke_width,
            hole,
            teeth,
            teeth_depth,
            sweep,
        } => Look::Shape {
            shape: shape.clone(),
            size,
            height: height.map_or(size, conv),
            fill: dd_color(fill, t),
            stroke: dd_color(stroke, t),
            stroke_width: stroke_width.map_or(0.0, conv),
            params: [
                js_min(0.95, js_max(0.0, hole.unwrap_or(0.0))),
                js_round(js_min(64.0, js_max(3.0, teeth.unwrap_or(12.0)))),
                js_min(360.0, js_max(1.0, sweep.unwrap_or(180.0))) * DEG,
                js_min(0.6, js_max(0.02, teeth_depth.unwrap_or(0.2))),
            ],
        },
        MarkerKind::Svg {
            asset,
            fill,
            stroke,
        } => Look::Svg {
            asset: asset.clone(),
            size,
            fill: dd_color(fill, t),
            stroke: dd_color(stroke, t),
        },
        MarkerKind::Raster { asset } => Look::Raster {
            asset: asset.clone(),
            size,
        },
        MarkerKind::Text {
            text,
            font,
            weight,
            italic,
            color,
            halo,
        } => {
            let text = dd_text(text, t);
            if text.is_empty() {
                return None;
            }
            Look::Text {
                text,
                size,
                font: font.clone().unwrap_or_else(|| "ui".into()),
                weight: weight.unwrap_or(400.0),
                italic: italic.unwrap_or(false),
                color: dd_color(color, t).unwrap_or_else(|| "ink".into()),
                halo: halo.as_ref().map(|(c, w)| (c.clone(), conv(*w))),
            }
        }
    };
    Some(MarkerStyle { look, common })
}

fn emit_marker(
    layer: &MarkerLayer,
    at: Vec2,
    angle: f64,
    level: f64,
    t: &dyn Values,
    env: &Env,
    sink: &mut dyn Sink,
) {
    if let Some(style) = marker_style(layer, level, t, env) {
        sink.marker(&style, at, angle);
    }
}

/// The drawing level of layer `j` of a marker symbol nested in a line or
/// fill layer: the sink merges look-alike marks of every object on a CAD
/// layer, so without its own level a paper-filled frame of one object could
/// cover the mark another object draws over its frame.
fn sub_level(level: f64, j: usize) -> f64 {
    level + (j.min(255) as f64) / 256.0
}

fn emit_marker_symbol(
    layers: &[Option<MarkerLayer>],
    at: Vec2,
    angle: f64,
    level: f64,
    t: &dyn Values,
    env: &Env,
    sink: &mut dyn Sink,
) {
    for (j, l) in layers.iter().enumerate() {
        if let Some(l) = l {
            emit_marker(l, at, angle, sub_level(level, j), t, env, sink);
        }
    }
}

/// Text following a line turns half a turn where it would read upside down (leftwards, or downwards).
fn reads_backwards(angle: f64) -> bool {
    let c = cos(angle);
    c < -1e-9 || (c.abs() <= 1e-9 && sin(angle) < 0.0)
}

/// About half a text mark's length along its line, in world units (px-sized text is left alone).
fn text_half_length(st: &MarkerStyle) -> f64 {
    match &st.look {
        Look::Text { text, size, .. } if st.common.unit == PrimUnit::World => {
            (utf16_len(text) as f64 * 0.3 + 0.3) * size + st.common.offset[0].abs()
        }
        _ => 0.0,
    }
}

fn mirrored_anchor(a: &str) -> String {
    match a {
        "top" => "bottom",
        "bottom" => "top",
        "left" => "right",
        "right" => "left",
        "top-left" => "bottom-right",
        "top-right" => "bottom-left",
        "bottom-left" => "top-right",
        "bottom-right" => "top-left",
        a => a,
    }
    .to_string()
}

/// A text style turned half a turn in place: the offset and the anchor are
/// mirrored, so the text keeps the same box on the same side of the line
/// and only its letters turn.
fn turned_text(st: &MarkerStyle) -> MarkerStyle {
    let c = &st.common;
    MarkerStyle {
        look: st.look.clone(),
        common: Common {
            offset: [-c.offset[0], -c.offset[1]],
            anchor: mirrored_anchor(&c.anchor),
            ..c.clone()
        },
    }
}

// ── Lines ──────────────────────────────────────────────────────────────

fn stroke_style(layer: &SimpleLine, level: f64, t: &dyn Values, env: &Env) -> Option<StrokeStyle> {
    let color = dd_color(&layer.color, t).filter(|c| !c.is_empty())?;
    let unit = layer.base.unit;
    let (width, width_unit) = to_drawn(dd_number(&layer.width, t, 0.0), unit, env);
    let len = |v: f64| {
        if width_unit == PrimUnit::Px {
            to_px(v, unit)
        } else {
            to_world(v, unit, env)
        }
    };
    Some(StrokeStyle {
        color,
        opacity: layer.base.opacity.unwrap_or(1.0),
        width,
        unit: width_unit,
        dash: layer
            .dash
            .as_ref()
            .filter(|d| !d.is_empty())
            .map(|d| d.iter().map(|&x| len(x)).collect()),
        dash_offset: layer
            .dash_offset
            .filter(|&x| truthy_num(x))
            .map_or(0.0, len),
        cap: layer.cap.clone().unwrap_or_else(|| "butt".into()),
        join: layer.join.clone().unwrap_or_else(|| "miter".into()),
        blur: layer.blur.filter(|&x| truthy_num(x)).map_or(0.0, len),
        level,
    })
}

/// A line layer on one path (a line, or an area ring).
fn emit_line_layer(
    layer: &Layer,
    pts: &[Vec2],
    closed: bool,
    level: f64,
    t: &dyn Values,
    env: &Env,
    sink: &mut dyn Sink,
) {
    let (base, offset) = match layer {
        Layer::SimpleLine(l) => (&l.base, &l.offset),
        Layer::MarkerLine(l) => (&l.base, &l.offset),
        _ => return,
    };
    if !dd_bool(&base.enabled, t) {
        return;
    }
    let off = dd_number(offset, t, 0.0);
    let d = if truthy_num(off) {
        to_world(off, base.unit, env)
    } else {
        0.0
    };
    let mut path = if truthy_num(d) {
        offset_path(pts, d, closed)
    } else {
        pts.to_vec()
    };
    // A page shift moves the whole line the same way (north-up view: the page's right is east).
    if let Layer::SimpleLine(SimpleLine {
        shift: Some(shift), ..
    }) = layer
        && (truthy_num(shift[0]) || truthy_num(shift[1]))
    {
        let dx = to_world(shift[0], base.unit, env);
        let dy = to_world(shift[1], base.unit, env);
        for p in &mut path {
            *p = Vec2::new(p.x + dx, p.y + dy);
        }
    }
    if path.len() < 2 {
        return;
    }
    match layer {
        Layer::SimpleLine(l) => {
            let Some(style) = stroke_style(l, level, t, env) else {
                return;
            };
            match &l.wave {
                Some(w) if w.length > 0.0 => {
                    let len = |v: f64| to_world(v, base.unit, env);
                    let spec = WaveSpec {
                        shape: w.shape.clone(),
                        length: len(w.length),
                        amplitude: len(w.amplitude),
                        spacing: len(w.spacing.unwrap_or(w.length)),
                        connect: w.connect != Some(false),
                        offset_along: w.offset_along.map(len),
                    };
                    for piece in wave_paths(&path, closed, &spec) {
                        sink.stroke(&style, &piece, false);
                    }
                }
                _ => sink.stroke(&style, &path, closed),
            }
        }
        Layer::MarkerLine(l) => emit_markers_along(l, &path, closed, level, t, env, sink),
        _ => {}
    }
}

fn emit_markers_along(
    layer: &MarkerLine,
    path: &[Vec2],
    closed: bool,
    level: f64,
    t: &dyn Values,
    env: &Env,
    sink: &mut dyn Sink,
) {
    let unit = layer.base.unit;
    let interval = layer.interval.map_or(0.0, |v| to_world(v, unit, env));
    let along = layer.offset_along.map_or(0.0, |v| to_world(v, unit, env));
    let group = layer
        .group
        .filter(|g| g.0 > 1.0)
        .map(|(count, spacing)| PlaceGroup {
            count,
            spacing: to_world(spacing, unit, env),
        });
    // One style per marker layer for the whole path: the sink merges them into one batch.
    let styles: Vec<MarkerStyle> = layer
        .marker
        .iter()
        .enumerate()
        .filter_map(|(j, m)| {
            m.as_ref()
                .and_then(|m| marker_style(m, sub_level(level, j), t, env))
        })
        .collect();
    if styles.is_empty() {
        return;
    }
    let follow = layer.rotate != Some(false);
    let turned: Vec<Option<MarkerStyle>> = styles
        .iter()
        .map(|st| (follow && st.is_text()).then(|| turned_text(st)))
        .collect();
    // Text that follows the line keeps half its length clear of sharp corners instead of bending round them.
    let clear = if follow {
        styles.iter().map(text_half_length).fold(0.0, js_max)
    } else {
        0.0
    };
    for p in place_along(
        path,
        closed,
        &layer.placement,
        interval,
        along,
        group,
        clear,
    ) {
        for (st, alt) in styles.iter().zip(&turned) {
            if !follow {
                sink.marker(st, p.at, 0.0);
                continue;
            }
            match alt {
                Some(alt) if reads_backwards(p.angle + st.common.rotation) => {
                    sink.marker(alt, p.at, p.angle + PI);
                }
                _ => sink.marker(st, p.at, p.angle),
            }
        }
    }
}

// ── Fills ──────────────────────────────────────────────────────────────

fn base_of(layer: &Layer) -> Option<&Base> {
    match layer {
        Layer::Marker(m) => Some(&m.base),
        Layer::SimpleLine(l) => Some(&l.base),
        Layer::MarkerLine(l) => Some(&l.base),
        Layer::SimpleFill { base, .. }
        | Layer::ImageFill { base, .. }
        | Layer::CentroidMarker { base, .. } => Some(base),
        Layer::HatchFill(h) => Some(&h.base),
        Layer::PatternFill(p) => Some(&p.base),
        Layer::Unknown => None,
    }
}

fn emit_fill_layer(
    layer: &Layer,
    rings: &[Vec<Vec2>],
    level: f64,
    t: &dyn Values,
    env: &Env,
    sink: &mut dyn Sink,
) {
    if let Layer::SimpleLine(SimpleLine { rings: which, .. })
    | Layer::MarkerLine(MarkerLine { rings: which, .. }) = layer
    {
        let which = which.as_deref().unwrap_or("all");
        for (i, r) in rings.iter().enumerate() {
            if (which == "exterior" && i > 0) || (which == "interior" && i == 0) {
                continue;
            }
            emit_line_layer(layer, r, true, level, t, env, sink);
        }
        return;
    }
    let Some(base) = base_of(layer) else {
        return;
    };
    if !dd_bool(&base.enabled, t) {
        return;
    }
    let opacity = base.opacity.unwrap_or(1.0);
    let px = drawn_in_px(base.unit, env);
    let len = |v: f64| {
        if px {
            to_px(v, base.unit)
        } else {
            to_world(v, base.unit, env)
        }
    };
    let unit = if px { PrimUnit::Px } else { PrimUnit::World };
    match layer {
        Layer::SimpleFill { color, .. } => {
            if let Some(color) = dd_color(color, t).filter(|c| !c.is_empty()) {
                sink.fill(
                    &FillPaint::Solid {
                        color,
                        opacity,
                        level,
                    },
                    rings,
                );
            }
        }
        Layer::HatchFill(h) => {
            let Some(color) = dd_color(&h.color, t).filter(|c| !c.is_empty()) else {
                return;
            };
            if !positive(h.spacing) {
                return;
            }
            let paint = FillPaint::Hatch {
                color,
                opacity,
                angle: h.angle * DEG,
                spacing: len(h.spacing),
                width: len(h.width),
                offset: len(h.offset.unwrap_or(0.0)),
                dash: h
                    .dash
                    .as_ref()
                    .filter(|d| !d.is_empty())
                    .map(|d| d.iter().map(|&x| len(x)).collect()),
                dash_offset: len(h.dash_offset.unwrap_or(0.0)),
                unit,
                level,
            };
            sink.fill(&paint, rings);
        }
        Layer::PatternFill(p) => {
            if !(positive(p.spacing_x) && positive(p.spacing_y)) {
                return;
            }
            let markers: Vec<MarkerStyle> = p
                .marker
                .iter()
                .filter_map(|m| m.as_ref().and_then(|m| marker_style(m, level, t, env)))
                .collect();
            if markers.is_empty() {
                return;
            }
            let off = p.offset.unwrap_or([0.0, 0.0]);
            let size = [len(p.spacing_x), len(p.spacing_y)];
            let angle = p.angle.unwrap_or(0.0) * DEG;
            let offset = [len(off[0]), len(off[1])];
            let stagger = p.stagger.unwrap_or(false);
            // Shapes are drawn by the shader, one paint per marker layer, in the pattern's unit;
            // text and images go into one atlas tile after them.
            let world_per_px = (MM_PER_PX * env.plot_scale) / 1000.0;
            let in_unit = |v: f64, u: PrimUnit| {
                if u == unit {
                    v
                } else if u == PrimUnit::Px {
                    v * world_per_px
                } else {
                    v / world_per_px
                }
            };
            for m in &markers {
                let Look::Shape {
                    shape,
                    size: ms,
                    height,
                    fill,
                    stroke,
                    stroke_width,
                    params,
                } = &m.look
                else {
                    continue;
                };
                let u = m.common.unit;
                let mark = MarkerStyle {
                    look: Look::Shape {
                        shape: shape.clone(),
                        size: in_unit(*ms, u),
                        height: in_unit(*height, u),
                        fill: fill.clone(),
                        stroke: stroke.clone(),
                        stroke_width: in_unit(*stroke_width, u),
                        params: *params,
                    },
                    common: Common {
                        unit,
                        offset: [
                            in_unit(m.common.offset[0], u),
                            in_unit(m.common.offset[1], u),
                        ],
                        ..m.common.clone()
                    },
                };
                sink.fill(
                    &FillPaint::Pattern {
                        mark,
                        size,
                        stagger,
                        angle,
                        offset,
                        jitter: js_min(1.0, js_max(0.0, p.jitter.unwrap_or(0.0))),
                        coverage: js_min(1.0, js_max(0.0, p.coverage.unwrap_or(1.0))),
                        seed: p.seed.unwrap_or(0.0),
                        opacity,
                        unit,
                        level,
                    },
                    rings,
                );
            }
            let others: Vec<MarkerStyle> = markers.into_iter().filter(|m| !m.is_shape()).collect();
            if !others.is_empty() {
                sink.fill(
                    &FillPaint::Tile {
                        tile: Tile::Markers {
                            markers: others,
                            stagger,
                        },
                        size,
                        angle,
                        offset,
                        opacity,
                        unit,
                        level,
                    },
                    rings,
                );
            }
        }
        Layer::ImageFill {
            asset,
            tile_size,
            angle,
            ..
        } => {
            if !positive(*tile_size) {
                return;
            }
            let w = len(*tile_size);
            let aspect = env.aspects.get(asset).copied().unwrap_or(1.0);
            sink.fill(
                &FillPaint::Tile {
                    tile: Tile::Asset(asset.clone()),
                    size: [w, w * aspect],
                    angle: angle.unwrap_or(0.0) * DEG,
                    offset: [0.0, 0.0],
                    opacity,
                    unit,
                    level,
                },
                rings,
            );
        }
        Layer::CentroidMarker {
            marker, position, ..
        } => {
            let at = if position.as_deref() == Some("centroid") {
                rings.first().and_then(|r| centroid_of(r))
            } else {
                interior_point(rings)
            };
            if let Some(at) = at {
                emit_marker_symbol(marker, at, 0.0, level, t, env, sink);
            }
        }
        _ => {}
    }
}

// ── Entry point ────────────────────────────────────────────────────────

/// Draws a symbol on a geometry. A symbol of another class adapts, so any
/// symbol can be given to any object: a line symbol on an area draws its
/// edges (the area's rings as closed lines, left = inside), a marker symbol
/// sits at an area's inside point or a line's middle, a fill symbol fills
/// a closed line. A fill symbol on an open line or a point, and a line
/// symbol on a point, draw nothing. `level_base` orders symbol layers
/// across a layer (fills first, then lines, then markers).
pub fn compile_symbol(
    symbol: &Symbol,
    geom: &Geom,
    t: &dyn Values,
    env: &Env,
    sink: &mut dyn Sink,
    level_base: f64,
) {
    match symbol.kind {
        SymbolType::Marker => {
            let at = match geom {
                Geom::Marker(p) => Some(*p),
                Geom::Fill(rings) => interior_point(rings),
                Geom::Line(paths) => line_middle(paths),
            };
            let Some(at) = at else {
                return;
            };
            for (i, l) in symbol.layers.iter().enumerate() {
                if let Layer::Marker(m) = l {
                    emit_marker(m, at, 0.0, level_base + i as f64, t, env, sink);
                }
            }
        }
        SymbolType::Line => {
            let rings;
            let paths: &[(Vec<Vec2>, bool)] = match geom {
                Geom::Line(paths) => paths,
                Geom::Fill(r) => {
                    rings = r.iter().map(|pts| (pts.clone(), true)).collect::<Vec<_>>();
                    &rings
                }
                Geom::Marker(_) => &[],
            };
            for (i, l) in symbol.layers.iter().enumerate() {
                for (pts, closed) in paths {
                    emit_line_layer(l, pts, *closed, level_base + i as f64, t, env, sink);
                }
            }
        }
        SymbolType::Fill => {
            let closed;
            let rings: &[Vec<Vec2>] = match geom {
                Geom::Fill(r) => r,
                Geom::Line(paths) => {
                    closed = paths
                        .iter()
                        .filter(|(pts, c)| *c && pts.len() > 2)
                        .map(|(pts, _)| pts.clone())
                        .collect::<Vec<_>>();
                    &closed
                }
                Geom::Marker(_) => &[],
            };
            if rings.is_empty() {
                return;
            }
            for (i, l) in symbol.layers.iter().enumerate() {
                emit_fill_layer(l, rings, level_base + i as f64, t, env, sink);
            }
        }
    }
}
