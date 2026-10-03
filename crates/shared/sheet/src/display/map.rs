//! A map frame: its view (centre, scale, the content's turn), the content
//! the host paints, coordinate grids and their labels, and an overview's
//! frame of another map. Turkish surveying conventions: Y is east, X north;
//! labels are whole metres, Y on the top and bottom edges, X on the left and
//! right ones along the edge, and the outside labels stay in the frame's
//! label band, never in the next item (QGIS Q2).

use super::{Ctx, MapPrim, MapViewPrim, Note, Pen, Prim};
use crate::display::RenderInputs;
use crate::expr::{Eval, Scope};
use crate::kinds::*;
use crate::model::{Item, VarValue};
use crate::style::Stroke;
use crate::text;
use crate::units::*;

/// A map as it is drawn: where its content is and what it shows.
#[derive(Clone, Debug)]
pub(crate) struct MapFrame {
    /// The content's rectangle in the item's own (unturned) coordinates.
    pub content: RectUm,
    pub center: Option<GroundPoint>,
    pub scale: u32,
    /// The content's turn in the frame.
    pub view_rot: Mdeg,
    /// The frame's turn on the paper.
    pub item_rot: Mdeg,
}

impl MapFrame {
    fn k(&self) -> f64 {
        // Micrometres on the paper per metre on the ground.
        1_000_000.0 / f64::from(self.scale.max(1))
    }

    /// A ground point (east, north) in the item's own coordinates.
    pub fn to_local(&self, g: [f64; 2]) -> Option<[f64; 2]> {
        let c = self.center?;
        let (a, b) = ((g[0] - c.x) * self.k(), (g[1] - c.y) * self.k());
        let (s, co) = sin_cos(self.view_rot);
        let rc = self.content.center();
        Some([rc[0] + a * co + b * s, rc[1] + a * s - b * co])
    }

    /// A point of the item's own coordinates on the ground.
    pub fn to_ground(&self, p: [f64; 2]) -> Option<[f64; 2]> {
        let c = self.center?;
        let rc = self.content.center();
        let (x, y) = (p[0] - rc[0], p[1] - rc[1]);
        let (s, co) = sin_cos(self.view_rot);
        let a = x * co + y * s;
        let b = x * s - y * co;
        Some([c.x + a / self.k(), c.y + b / self.k()])
    }

    /// The ground the content covers.
    pub fn extent(&self) -> Option<[f64; 4]> {
        let r = &self.content;
        let corners = [
            [f64::from(r.left), f64::from(r.top)],
            [r.right() as f64, f64::from(r.top)],
            [r.right() as f64, r.bottom() as f64],
            [f64::from(r.left), r.bottom() as f64],
        ];
        let mut e = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for c in corners {
            let g = self.to_ground(c)?;
            e = [
                e[0].min(g[0]),
                e[1].min(g[1]),
                e[2].max(g[0]),
                e[3].max(g[1]),
            ];
        }
        Some(e)
    }

    /// Where grid north points on the paper (clockwise from up).
    pub fn grid_north(&self) -> Mdeg {
        norm_mdeg(i64::from(self.item_rot) + i64::from(self.view_rot))
    }

    /// The content's corners on the ground, in order (an overview draws them).
    pub fn ground_corners(&self) -> Option<[[f64; 2]; 4]> {
        let r = &self.content;
        let pts = [
            [f64::from(r.left), f64::from(r.top)],
            [r.right() as f64, f64::from(r.top)],
            [r.right() as f64, r.bottom() as f64],
            [f64::from(r.left), r.bottom() as f64],
        ];
        Some([
            self.to_ground(pts[0])?,
            self.to_ground(pts[1])?,
            self.to_ground(pts[2])?,
            self.to_ground(pts[3])?,
        ])
    }
}

/// The content's rectangle: the frame less its padding and its label band.
pub(crate) fn content_rect(item: &Item, m: &MapItem) -> RectUm {
    let r = item.content_rect().inset(m.label_band);
    RectUm::new(r.left, r.top, r.width.max(1), r.height.max(1))
}

/// A map's frame as drawn: its view, or an atlas page's.
pub(crate) fn frame_of(item: &Item, m: &MapItem, inputs: &RenderInputs) -> Option<MapFrame> {
    let (center, scale, view_rot) = match &m.view {
        MapView::Fixed(f) => (f.center, f.scale, f.rotation),
        MapView::Atlas(a) => {
            let page = inputs
                .atlas
                .as_ref()
                .and_then(|p| p.maps.iter().find(|v| v.item == item.id));
            match page {
                Some(v) => (Some(v.center), v.scale.max(1), v.rotation),
                None => {
                    let s = match &a.policy {
                        AtlasScale::Fixed(f) => f.scale,
                        AtlasScale::Predefined(p) => p.scales.first().copied().unwrap_or(1000),
                        AtlasScale::Fit(_) => 1000,
                    };
                    (None, s, a.rotation)
                }
            }
        }
    };
    Some(MapFrame {
        content: content_rect(item, m),
        center,
        scale: scale.max(1),
        view_rot,
        item_rot: item.rotation,
    })
}

/// A grid interval for a scale: 1, 2, 2.5 or 5 times a power of ten nearest to 10 cm on the paper.
pub fn auto_interval(scale: u32) -> f64 {
    let target = f64::from(scale.max(1)) * 0.1;
    let k = libm::floor(libm::log10(target)) as i32;
    let mut best = (f64::MAX, target);
    for e in (k - 1)..=(k + 1) {
        let p = pow10(e);
        for m in [1.0, 2.0, 2.5, 5.0] {
            let c = m * p;
            let d = libm::fabs(libm::log(c / target));
            if d < best.0 - 1e-12 || ((d - best.0).abs() <= 1e-12 && c > best.1) {
                best = (d, c);
            }
        }
    }
    best.1
}

fn pow10(e: i32) -> f64 {
    let mut p = 1.0;
    if e >= 0 {
        for _ in 0..e {
            p *= 10.0;
        }
    } else {
        for _ in 0..(-e) {
            p /= 10.0;
        }
    }
    p
}

/// The part of segment `a`–`b` inside `r` (Liang–Barsky).
pub(crate) fn clip_segment(a: [f64; 2], b: [f64; 2], r: &RectUm) -> Option<([f64; 2], [f64; 2])> {
    let (x0, y0, x1, y1) = (
        f64::from(r.left),
        f64::from(r.top),
        r.right() as f64,
        r.bottom() as f64,
    );
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let mut t0 = 0.0f64;
    let mut t1 = 1.0f64;
    for (p, q) in [
        (-dx, a[0] - x0),
        (dx, x1 - a[0]),
        (-dy, a[1] - y0),
        (dy, y1 - a[1]),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 <= t1).then(|| {
        (
            [a[0] + t0 * dx, a[1] + t0 * dy],
            [a[0] + t1 * dx, a[1] + t1 * dy],
        )
    })
}

/// Drawing left for after the map's clip is closed: the labels outside the content.
type Deferred = Vec<Box<dyn FnOnce(&mut Pen)>>;

/// Grid line values from `lo` to `hi` (with `offset`), at most `limit` of them.
fn values(lo: f64, hi: f64, step: f64, offset: f64, limit: usize) -> Option<Vec<f64>> {
    if step.is_nan() || step <= 0.0 || !lo.is_finite() || !hi.is_finite() {
        return Some(Vec::new());
    }
    let first = libm::ceil((lo - offset) / step);
    let last = libm::floor((hi - offset) / step);
    if last - first > limit as f64 {
        return None;
    }
    let mut out = Vec::new();
    let mut k = first;
    while k <= last {
        out.push(k * step + offset);
        k += 1.0;
    }
    Some(out)
}

/// A grid label's text.
fn label_text(
    v: f64,
    axis: &str,
    labels: &GridLabels,
    scope: &Scope,
    notes: &mut Vec<Note>,
    item: &str,
) -> String {
    let metres = |f: &MetresFormat| -> String {
        let s = kentos_geometry_core::display::fixed(v, usize::from(f.decimals));
        let s = if f.group_thousands { group(&s) } else { s };
        if labels.axis_prefix {
            format!("{axis}={s}")
        } else {
            s
        }
    };
    match &labels.format {
        GridLabelFormat::Metres(f) => metres(f),
        GridLabelFormat::Dms(_) => {
            notes.push(Note {
                item: item.to_owned(),
                code: "grid_format_unsupported",
                detail: "dms".to_owned(),
            });
            metres(&MetresFormat::default())
        }
        GridLabelFormat::Expression(e) => {
            let mut s = scope.clone();
            s.extra = vec![
                ("DEGER".to_owned(), VarValue::Number(v)),
                ("EKSEN".to_owned(), VarValue::Text(axis.to_owned())),
            ];
            match crate::expr::evaluate_text(&e.expression, &s) {
                Eval::Value(VarValue::Text(t)) => t,
                other => {
                    notes.push(super::eval_note(item, &e.expression, &other));
                    metres(&MetresFormat::default())
                }
            }
        }
    }
}

/// Thousands of the whole part apart by spaces: “4512300.5” → “4 512 300.5”.
fn group(s: &str) -> String {
    let (sign, rest) = s.strip_prefix('-').map_or(("", s), |r| ("-", r));
    let (int, frac) = rest
        .split_once('.')
        .map_or((rest, None), |(a, b)| (a, Some(b)));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    match frac {
        Some(f) => format!("{sign}{out}.{f}"),
        None => format!("{sign}{out}"),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// Where a line through `p` along `d` crosses a side of `r`, as the position along that side.
fn crossing(p: [f64; 2], d: [f64; 2], r: &RectUm, side: Side) -> Option<f64> {
    let (x0, y0, x1, y1) = (
        f64::from(r.left),
        f64::from(r.top),
        r.right() as f64,
        r.bottom() as f64,
    );
    match side {
        Side::Top | Side::Bottom => {
            let y = if side == Side::Top { y0 } else { y1 };
            if d[1].abs() < 1e-12 {
                return None;
            }
            let x = p[0] + (y - p[1]) / d[1] * d[0];
            (x >= x0 && x <= x1).then_some(x)
        }
        Side::Left | Side::Right => {
            let x = if side == Side::Left { x0 } else { x1 };
            if d[0].abs() < 1e-12 {
                return None;
            }
            let y = p[1] + (x - p[0]) / d[0] * d[1];
            (y >= y0 && y <= y1).then_some(y)
        }
    }
}

pub(crate) fn draw(ctx: &Ctx, it: &Item, m: &MapItem, pen: &mut Pen, notes: &mut Vec<Note>) {
    let Some(mf) = ctx.maps.get(&it.id) else {
        return;
    };
    let r = mf.content;
    if let Some(f) = &it.fill {
        pen.rect(r, Some(f), None, 0);
    }
    if mf.center.is_none() {
        notes.push(Note {
            item: it.id.clone(),
            code: "map_unplaced",
            detail: String::new(),
        });
    }
    let clip_feature = (m.clip_to_atlas_feature)
        .then(|| ctx.inputs.atlas.as_ref().map(|a| a.feature.id.clone()))
        .flatten();
    let clip = {
        let c = rotate(r.center(), pen.center, pen.rot);
        if norm_mdeg(i64::from(pen.rot)) == 0 {
            r
        } else {
            RectUm::new(
                round_um(c[0] - f64::from(r.width) / 2.0),
                round_um(c[1] - f64::from(r.height) / 2.0),
                r.width,
                r.height,
            )
        }
    };
    pen.prims.push(Prim::Map(MapPrim {
        item: it.id.clone(),
        clip,
        rotation: pen.rot,
        view: MapViewPrim {
            center: mf.center,
            scale: mf.scale,
            rotation: mf.view_rot,
        },
        layers: m.layers.clone(),
        crs: m.crs.clone(),
        extent: mf.extent(),
        clip_feature,
    }));
    pen.push_clip(r);
    if let Some(other) = &m.overview_of {
        overview(ctx, it, mf, other, &m.overview_frame, pen, notes);
    }
    let scope = ctx.scope_for(it);
    let mut outside: Deferred = Vec::new();
    for grid in m.grids.iter().filter(|g| g.enabled) {
        if grid.crs.is_some() {
            notes.push(Note {
                item: it.id.clone(),
                code: "grid_crs_unsupported",
                detail: grid.crs.clone().unwrap_or_default(),
            });
            continue;
        }
        if mf.center.is_none() {
            continue;
        }
        draw_grid(it, m, mf, grid, &scope, pen, notes, &mut outside);
    }
    pen.pop_clip();
    for f in outside {
        f(pen);
    }
    if let Some(b) = &it.border {
        pen.rect(r, None, Some(b), 0);
    } else if m
        .grids
        .iter()
        .any(|g| g.enabled && g.frame == GridFrame::Line)
    {
        pen.rect(r, None, Some(&Stroke::solid("#000000", 350)), 0);
    }
}

fn overview(
    ctx: &Ctx,
    it: &Item,
    mf: &MapFrame,
    other: &str,
    stroke: &Stroke,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
) {
    let Some(of) = ctx.maps.get(other) else {
        notes.push(Note {
            item: it.id.clone(),
            code: "broken_link",
            detail: other.to_owned(),
        });
        return;
    };
    let Some(corners) = of.ground_corners() else {
        return;
    };
    let pts: Option<Vec<[f64; 2]>> = corners.iter().map(|g| mf.to_local(*g)).collect();
    if let Some(pts) = pts {
        pen.path(&pts, true, None, Some(stroke));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_grid(
    it: &Item,
    m: &MapItem,
    mf: &MapFrame,
    grid: &MapGrid,
    scope: &Scope,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
    outside: &mut Deferred,
) {
    let r = mf.content;
    let Some(ext) = mf.extent() else {
        return;
    };
    let auto = auto_interval(mf.scale);
    let ix = if grid.interval[0] > 0.0 {
        grid.interval[0]
    } else {
        auto
    };
    let iy = if grid.interval[1] > 0.0 {
        grid.interval[1]
    } else {
        ix
    };
    let (Some(es), Some(ns)) = (
        values(ext[0], ext[2], ix, grid.offset[0], 1000),
        values(ext[1], ext[3], iy, grid.offset[1], 1000),
    ) else {
        notes.push(Note {
            item: it.id.clone(),
            code: "grid_too_dense",
            detail: format!("{ix} m"),
        });
        return;
    };
    let (s, c) = sin_cos(mf.view_rot);
    let east = [c, s];
    let north = [s, -c];
    let far = (f64::from(r.width) + f64::from(r.height)) * 2.0;
    let line_through = |p: [f64; 2], d: [f64; 2]| {
        (
            [p[0] - d[0] * far, p[1] - d[1] * far],
            [p[0] + d[0] * far, p[1] + d[1] * far],
        )
    };
    // Points on each line (local) and its direction.
    let mut lines_e: Vec<(f64, [f64; 2])> = Vec::new();
    for e in &es {
        if let Some(p) = mf.to_local([*e, ext[1]]) {
            lines_e.push((*e, p));
        }
    }
    let mut lines_n: Vec<(f64, [f64; 2])> = Vec::new();
    for n in &ns {
        if let Some(p) = mf.to_local([ext[0], *n]) {
            lines_n.push((*n, p));
        }
    }
    let mut segs: Vec<[[f64; 2]; 2]> = Vec::new();
    match grid.kind {
        GridKind::Lines => {
            for (_, p) in &lines_e {
                let (a, b) = line_through(*p, north);
                if let Some((a, b)) = clip_segment(a, b, &r) {
                    segs.push([a, b]);
                }
            }
            for (_, p) in &lines_n {
                let (a, b) = line_through(*p, east);
                if let Some((a, b)) = clip_segment(a, b, &r) {
                    segs.push([a, b]);
                }
            }
        }
        GridKind::Cross => {
            let arm = f64::from(grid.cross_size);
            for e in &es {
                for n in &ns {
                    let Some(p) = mf.to_local([*e, *n]) else {
                        continue;
                    };
                    if !r.contains(p) {
                        continue;
                    }
                    for d in [east, north] {
                        let a = [p[0] - d[0] * arm, p[1] - d[1] * arm];
                        let b = [p[0] + d[0] * arm, p[1] + d[1] * arm];
                        if let Some((a, b)) = clip_segment(a, b, &r) {
                            segs.push([a, b]);
                        }
                    }
                }
            }
        }
        GridKind::Ticks => {
            let len = f64::from(grid.cross_size);
            for (list, d) in [(&lines_e, north), (&lines_n, east)] {
                for (_, p) in list.iter() {
                    let (a, b) = line_through(*p, d);
                    if let Some((a, b)) = clip_segment(a, b, &r) {
                        let l =
                            ((b[0] - a[0]) * (b[0] - a[0]) + (b[1] - a[1]) * (b[1] - a[1])).sqrt();
                        if l <= 0.0 {
                            continue;
                        }
                        let u = [
                            (b[0] - a[0]) / l * len.min(l / 2.0),
                            (b[1] - a[1]) / l * len.min(l / 2.0),
                        ];
                        segs.push([a, [a[0] + u[0], a[1] + u[1]]]);
                        segs.push([b, [b[0] - u[0], b[1] - u[1]]]);
                    }
                }
            }
        }
        GridKind::FrameOnly => {}
    }
    pen.lines(&segs, &grid.stroke);

    // What stands outside the content: the frame's zebra or ticks and the labels, drawn after the clip ends.
    let fw = f64::from(grid.frame_width);
    let band_used = match grid.frame {
        GridFrame::Zebra | GridFrame::Ticks => fw,
        GridFrame::None | GridFrame::Line => 0.0,
    };
    // Crossings on each side: positions along the side with the line's value and axis.
    let mut crossings: Vec<(Side, f64, f64, &'static str)> = Vec::new();
    for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
        for (v, p) in &lines_e {
            if let Some(x) = crossing(*p, north, &r, side) {
                crossings.push((side, x, *v, "Y"));
            }
        }
        for (v, p) in &lines_n {
            if let Some(x) = crossing(*p, east, &r, side) {
                crossings.push((side, x, *v, "X"));
            }
        }
    }
    let frame_kind = grid.frame;
    let frame_stroke = Stroke::solid(&grid.stroke.color, grid.stroke.width.max(180));
    let cross_frame = crossings.clone();
    outside.push(Box::new(move |pen: &mut Pen| {
        zebra_or_ticks(pen, &r, frame_kind, fw, &frame_stroke, &cross_frame)
    }));

    if !grid.labels.show {
        return;
    }
    let labels = grid.labels.clone();
    let style = labels.style.clone();
    let gap = f64::from(labels.gap);
    let offset = band_used + gap;
    let asc = text::ascent(&style) as f64;
    let desc = text::descent(&style) as f64;
    let cap = text::cap_height(&style) as f64;
    let band = f64::from(m.label_band);
    let inside = labels.position == LabelPosition::Inside;
    let along = labels.direction == LabelDirection::AlongEdge;
    let mut placed: Vec<(Side, f64, f64, String, i64)> = Vec::new();
    let mut band_short = false;
    for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
        let wanted = match side {
            Side::Top => labels.sides.top,
            Side::Right => labels.sides.right,
            Side::Bottom => labels.sides.bottom,
            Side::Left => labels.sides.left,
        };
        if !wanted {
            continue;
        }
        let axis_of_side = if matches!(side, Side::Top | Side::Bottom) {
            "Y"
        } else {
            "X"
        };
        let mut on_side: Vec<(f64, f64)> = crossings
            .iter()
            .filter(|(s, _, _, a)| *s == side && *a == axis_of_side)
            .map(|(_, pos, v, _)| (*pos, *v))
            .collect();
        on_side.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (lo, hi) = match side {
            Side::Top | Side::Bottom => (f64::from(r.left), r.right() as f64),
            Side::Left | Side::Right => (f64::from(r.top), r.bottom() as f64),
        };
        let mut last_end = f64::MIN;
        for (pos, v) in on_side {
            let t = label_text(v, axis_of_side, &labels, scope, notes, &it.id);
            let w = text::text_width(&t, &style) as f64;
            // Along the side the label takes its width (level text on top and bottom, or turned along the side) or its height.
            let extent = if matches!(side, Side::Top | Side::Bottom) || along {
                w
            } else {
                asc + desc
            };
            let (a, b) = (pos - extent / 2.0, pos + extent / 2.0);
            if a < lo || b > hi || a < last_end + 1_000.0 {
                continue;
            }
            last_end = b;
            // Out from the content the label takes its height, or its width when level on a side edge.
            let depth = if matches!(side, Side::Top | Side::Bottom) || along {
                asc + desc
            } else {
                w
            };
            if !inside && offset + depth > band + 0.5 {
                band_short = true;
            }
            placed.push((side, pos, v, t, w as i64));
        }
    }
    if band_short {
        notes.push(Note {
            item: it.id.clone(),
            code: "grid_label_band",
            detail: format!("{}", round_i64(offset + asc + desc)),
        });
    }
    outside.push(Box::new(move |pen: &mut Pen| {
        for (side, pos, _v, t, w) in placed {
            let wf = w as f64;
            let (x, y, rot) = match (side, inside, along) {
                (Side::Top, false, _) => (pos - wf / 2.0, f64::from(r.top) - offset - desc, 0),
                (Side::Bottom, false, _) => (pos - wf / 2.0, r.bottom() as f64 + offset + cap, 0),
                (Side::Top, true, _) => (pos - wf / 2.0, f64::from(r.top) + offset + cap, 0),
                (Side::Bottom, true, _) => (pos - wf / 2.0, r.bottom() as f64 - offset - desc, 0),
                // Along the edge, reading upwards: the letters' tops point left.
                (Side::Left, false, true) => {
                    (f64::from(r.left) - offset - desc, pos + wf / 2.0, -90_000)
                }
                (Side::Right, false, true) => {
                    (r.right() as f64 + offset + cap, pos + wf / 2.0, -90_000)
                }
                (Side::Left, true, true) => {
                    (f64::from(r.left) + offset + cap, pos + wf / 2.0, -90_000)
                }
                (Side::Right, true, true) => {
                    (r.right() as f64 - offset - desc, pos + wf / 2.0, -90_000)
                }
                (Side::Left, false, false) => (f64::from(r.left) - offset - wf, pos + cap / 2.0, 0),
                (Side::Right, false, false) => (r.right() as f64 + offset, pos + cap / 2.0, 0),
                (Side::Left, true, false) => (f64::from(r.left) + offset, pos + cap / 2.0, 0),
                (Side::Right, true, false) => (r.right() as f64 - offset - wf, pos + cap / 2.0, 0),
            };
            let before = pen.prims.len();
            pen.text(&t, [x, y], &style, style.size, w, rot);
            if inside && let Some(Prim::Text(tp)) = pen.prims.get_mut(before) {
                tp.halo = Some("#ffffff".to_owned());
            }
        }
    }));
}

fn zebra_or_ticks(
    pen: &mut Pen,
    r: &RectUm,
    kind: GridFrame,
    fw: f64,
    stroke: &Stroke,
    crossings: &[(Side, f64, f64, &'static str)],
) {
    if fw <= 0.0 {
        return;
    }
    let (x0, y0, x1, y1) = (
        f64::from(r.left),
        f64::from(r.top),
        r.right() as f64,
        r.bottom() as f64,
    );
    match kind {
        GridFrame::Zebra => {
            let outer = RectUm::from_edges(
                round_i64(x0 - fw),
                round_i64(y0 - fw),
                round_i64(x1 + fw),
                round_i64(y1 + fw),
            );
            let mut blocks: Vec<[f64; 4]> = Vec::new();
            for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
                let (lo, hi) = match side {
                    Side::Top | Side::Bottom => (x0, x1),
                    Side::Left | Side::Right => (y0, y1),
                };
                let mut cuts: Vec<f64> = crossings
                    .iter()
                    .filter(|c| c.0 == side)
                    .map(|c| c.1)
                    .collect();
                cuts.push(lo);
                cuts.push(hi);
                cuts.sort_by(f64::total_cmp);
                cuts.dedup_by(|a, b| (*a - *b).abs() < 1.0);
                for (i, w) in cuts.windows(2).enumerate() {
                    if i % 2 != 0 {
                        continue;
                    }
                    let (a, b) = (w[0], w[1]);
                    blocks.push(match side {
                        Side::Top => [a, y0 - fw, b, y0],
                        Side::Bottom => [a, y1, b, y1 + fw],
                        Side::Left => [x0 - fw, a, x0, b],
                        Side::Right => [x1, a, x1 + fw, b],
                    });
                }
            }
            // The corners are solid.
            for (cx, cy) in [(x0 - fw, y0 - fw), (x1, y0 - fw), (x1, y1), (x0 - fw, y1)] {
                blocks.push([cx, cy, cx + fw, cy + fw]);
            }
            let black = stroke.color.clone();
            let mut segs = Vec::new();
            for b in blocks {
                segs.push(super::Seg::M(pen.pt(b[0], b[1])));
                segs.push(super::Seg::L(pen.pt(b[2], b[1])));
                segs.push(super::Seg::L(pen.pt(b[2], b[3])));
                segs.push(super::Seg::L(pen.pt(b[0], b[3])));
                segs.push(super::Seg::Z);
            }
            pen.segments(segs, Some(&black), None, false);
            let thin = Stroke::solid(&stroke.color, stroke.width.min(250));
            pen.rect(outer, None, Some(&thin), 0);
            pen.rect(*r, None, Some(&thin), 0);
        }
        GridFrame::Ticks => {
            let mut segs = Vec::new();
            for (side, pos, _, _) in crossings {
                segs.push(match side {
                    Side::Top => [[*pos, y0], [*pos, y0 - fw]],
                    Side::Bottom => [[*pos, y1], [*pos, y1 + fw]],
                    Side::Left => [[x0, *pos], [x0 - fw, *pos]],
                    Side::Right => [[x1, *pos], [x1 + fw, *pos]],
                });
            }
            pen.lines(&segs, stroke);
        }
        GridFrame::None | GridFrame::Line => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_intervals_are_ten_centimetres_on_paper() {
        assert_eq!(auto_interval(500), 50.0);
        assert_eq!(auto_interval(1000), 100.0);
        assert_eq!(auto_interval(2000), 200.0);
        assert_eq!(auto_interval(2500), 250.0);
        assert_eq!(auto_interval(5000), 500.0);
        assert_eq!(auto_interval(25_000), 2500.0);
    }

    #[test]
    fn thousands_are_grouped() {
        assert_eq!(group("4512300"), "4 512 300");
        assert_eq!(group("-485200.25"), "-485 200.25");
        assert_eq!(group("100"), "100");
    }

    #[test]
    fn the_view_goes_both_ways() {
        let mf = MapFrame {
            content: RectUm::new(0, 0, 200_000, 100_000),
            center: Some(GroundPoint {
                x: 485_000.0,
                y: 4_512_000.0,
            }),
            scale: 1000,
            view_rot: 30_000,
            item_rot: 0,
        };
        let p = mf.to_local([485_050.0, 4_512_020.0]).unwrap();
        let g = mf.to_ground(p).unwrap();
        assert!((g[0] - 485_050.0).abs() < 1e-6 && (g[1] - 4_512_020.0).abs() < 1e-6);
        // Unturned: 1 m east is 1 mm to the right at 1/1000; north is up.
        let flat = MapFrame { view_rot: 0, ..mf };
        assert_eq!(
            flat.to_local([485_001.0, 4_512_000.0]).unwrap(),
            [101_000.0, 50_000.0]
        );
        assert_eq!(
            flat.to_local([485_000.0, 4_512_001.0]).unwrap(),
            [100_000.0, 49_000.0]
        );
    }
}
