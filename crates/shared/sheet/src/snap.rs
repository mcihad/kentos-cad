//! Smart guides (design §5). A drag session computes its candidates once
//! (`SnapSession::new`): the page's edges and centre, the margins, the
//! guides, the snapping grid and the left, centre and right of every other
//! shown item (the master page's included; the moving items and their groups
//! left out). Each pointer move asks `query` with the drag's offset: per
//! axis the nearest candidate within the tolerance wins, ties going guide >
//! item > page and margins > grid, then the candidates' order. Equal spacing
//! to the neighbours in the moving box's row (or column) is a candidate too.
//! The lines, spacing marks and distance badges come back to be painted; the
//! platforms compute nothing. Coordinates are doubled inside, so a centre on
//! a half micrometre stays exact.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::error::{Result, SheetError};
use crate::model::{Axis, Item, ItemId, SheetBook, yes};
use crate::ops::Handle;
use crate::scene::Scene;
use crate::units::{Mdeg, PointUm, RectUm, Um, mm1_text, norm_mdeg, rotated_bounds};

/// What a snap line or mark came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum SnapKind {
    Guide,
    Item,
    Page,
    Margin,
    Grid,
    /// Equal spacing.
    Spacing,
    /// The same width or height as another item.
    Size,
}

impl SnapKind {
    /// Lower wins a tie: guide, then item (spacing and size are about items), then page and margins, then the grid.
    fn priority(self) -> u8 {
        match self {
            SnapKind::Guide => 0,
            SnapKind::Item | SnapKind::Spacing | SnapKind::Size => 1,
            SnapKind::Page | SnapKind::Margin => 2,
            SnapKind::Grid => 3,
        }
    }
}

/// A line to paint: `x` is upright at `left = at` from `top = from` to `to`; `y` is level at `top = at`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SnapLine {
    pub axis: Axis,
    pub at: Um,
    pub from: Um,
    pub to: Um,
    pub kind: SnapKind,
}

/// A distance to the nearest neighbour, with its text (“12.5 mm”).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct GapBadge {
    /// `x`: measured across, `y`: down.
    pub axis: Axis,
    pub from: PointUm,
    pub to: PointUm,
    pub distance: Um,
    pub text: String,
    /// To an item, or to the margin when there is none on that side.
    pub kind: SnapKind,
}

/// Gaps that are equal: each segment is one gap, painted with the same mark.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SpacingMark {
    pub axis: Axis,
    pub gap: Um,
    pub segments: Vec<[PointUm; 2]>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SnapResult {
    /// The offset to use: the asked one, snapped.
    pub delta: PointUm,
    pub lines: Vec<SnapLine>,
    pub gaps: Vec<GapBadge>,
    pub spacing: Vec<SpacingMark>,
}

/// Where a dragged resize handle ends up, and what it snapped to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ResizeSnap {
    pub to: PointUm,
    pub lines: Vec<SnapLine>,
    /// Items whose width (`x`) or height (`y`) the new size matches.
    pub sizes: Vec<SizeMatch>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SizeMatch {
    pub axis: Axis,
    pub item: ItemId,
    pub size: Um,
}

/// What snaps; `grid` none: the sheet's own setting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SnapOptions {
    #[serde(default = "yes")]
    pub guides: bool,
    #[serde(default = "yes")]
    pub items: bool,
    /// The page's edges and centre, and the margins.
    #[serde(default = "yes")]
    pub page: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grid: Option<bool>,
    #[serde(default = "yes")]
    pub spacing: bool,
    #[serde(default = "yes")]
    pub badges: bool,
}

impl Default for SnapOptions {
    fn default() -> Self {
        SnapOptions {
            guides: true,
            items: true,
            page: true,
            grid: None,
            spacing: true,
            badges: true,
        }
    }
}

/// Keyboard nudges (design §5): an arrow 1 mm, with Shift 10 mm, with Alt 0.1 mm.
pub const NUDGE: Um = 1_000;
pub const NUDGE_SHIFT: Um = 10_000;
pub const NUDGE_ALT: Um = 100;
/// The default tolerance in screen pixels; the caller turns it into micrometres at its zoom.
pub const TOLERANCE_PX: u32 = 6;

#[derive(Clone, Debug)]
struct Candidate {
    /// Doubled position.
    at2: i64,
    kind: SnapKind,
    /// The extent along the line, for painting it.
    span: Option<(i64, i64)>,
}

#[derive(Clone, Debug)]
struct Other {
    id: ItemId,
    b: RectUm,
}

/// A drag's snapping, computed once at its start.
pub struct SnapSession {
    moving: RectUm,
    single: Option<(ItemId, RectUm, Mdeg)>,
    xs: Vec<Candidate>,
    ys: Vec<Candidate>,
    others: Vec<Other>,
    grid: Option<Um>,
    page: RectUm,
    margins: RectUm,
    options: SnapOptions,
}

fn lo(r: &RectUm, axis: Axis) -> i64 {
    match axis {
        Axis::X => i64::from(r.left),
        Axis::Y => i64::from(r.top),
    }
}

fn hi(r: &RectUm, axis: Axis) -> i64 {
    match axis {
        Axis::X => r.right(),
        Axis::Y => r.bottom(),
    }
}

fn other_axis(axis: Axis) -> Axis {
    match axis {
        Axis::X => Axis::Y,
        Axis::Y => Axis::X,
    }
}

/// Whether two boxes overlap across `axis` (so they stand in one row for an `x` measure).
fn overlap(a: &RectUm, b: &RectUm, axis: Axis) -> bool {
    let o = other_axis(axis);
    lo(a, o) < hi(b, o) && lo(b, o) < hi(a, o)
}

/// A move's best candidate so far: its rank (distance, priority, order, anchor), then the correction, its kind and the gap.
type MoveBest = ((i64, u8, usize, usize), i64, SnapKind, i64);
/// A resize's best candidate so far: its rank (distance, priority, order), then the correction and the item whose size it matches.
type ResizeBest = ((i64, u8, usize), i64, Option<(ItemId, i64)>);

fn half(v2: i64) -> i64 {
    // A half away from zero.
    if v2 >= 0 { (v2 + 1) / 2 } else { (v2 - 1) / 2 }
}

impl SnapSession {
    /// The session for dragging `moving` on a sheet.
    pub fn new(
        book: &SheetBook,
        sheet_id: &str,
        moving: &[ItemId],
        options: SnapOptions,
    ) -> Result<SnapSession> {
        let scene = Scene::new(book, sheet_id)?;
        let items = &scene.sheet.items;
        let mut moving_set: BTreeSet<&str> = BTreeSet::new();
        for id in moving {
            if !items.iter().any(|i| i.id == *id) {
                return Err(SheetError::new(
                    "unknown_item",
                    format!("“{id}” bu paftanın öğesi değil (ana sayfanın öğeleri taşınmaz)."),
                ));
            }
            moving_set.insert(id.as_str());
        }
        // Everything inside a moving group moves with it; the groups around a moving item are not targets.
        let mut changed = true;
        while changed {
            changed = false;
            for it in items {
                if let Some(g) = &it.group
                    && moving_set.contains(g.as_str())
                    && moving_set.insert(it.id.as_str())
                {
                    changed = true;
                }
            }
        }
        let mut ancestors: BTreeSet<&str> = BTreeSet::new();
        for id in &moving_set {
            let mut up = items
                .iter()
                .find(|i| i.id == *id)
                .and_then(|i| i.group.as_deref());
            let mut n = 0;
            while let Some(g) = up {
                ancestors.insert(g);
                n += 1;
                if n > items.len() {
                    break;
                }
                up = items
                    .iter()
                    .find(|i| i.id == g)
                    .and_then(|i| i.group.as_deref());
            }
        }
        let mut mbox: Option<RectUm> = None;
        for it in items.iter().filter(|i| moving_set.contains(i.id.as_str())) {
            let b = rotated_bounds(&it.frame, it.rotation);
            mbox = Some(mbox.map_or(b, |u| u.union(&b)));
        }
        let moving_box = mbox.ok_or_else(|| SheetError::new("no_items", "Taşınacak öğe yok."))?;
        let single = match moving {
            [one] => items
                .iter()
                .find(|i| i.id == *one)
                .filter(|i| !i.is_group())
                .map(|i| (i.id.clone(), i.frame, i.rotation)),
            _ => None,
        };
        let page = scene.sheet.page.rect();
        let margins = scene.sheet.page.margin_rect();
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        if options.guides {
            for g in &scene.sheet.guides {
                let c = Candidate {
                    at2: 2 * i64::from(g.at),
                    kind: SnapKind::Guide,
                    span: None,
                };
                match g.axis {
                    Axis::X => xs.push(c),
                    Axis::Y => ys.push(c),
                }
            }
        }
        let mut others = Vec::new();
        for (it, _) in scene.all() {
            if moving_set.contains(it.id.as_str())
                || ancestors.contains(it.id.as_str())
                || !scene.shown(it)
            {
                continue;
            }
            let b = rotated_bounds(&it.frame, it.rotation);
            if !it.is_group() {
                others.push(Other {
                    id: it.id.clone(),
                    b,
                });
            }
            if options.items {
                let (l2, w) = (2 * i64::from(b.left), i64::from(b.width));
                let (t2, h) = (2 * i64::from(b.top), i64::from(b.height));
                let vspan = Some((i64::from(b.top), b.bottom()));
                let hspan = Some((i64::from(b.left), b.right()));
                for at2 in [l2, l2 + w, l2 + 2 * w] {
                    xs.push(Candidate {
                        at2,
                        kind: SnapKind::Item,
                        span: vspan,
                    });
                }
                for at2 in [t2, t2 + h, t2 + 2 * h] {
                    ys.push(Candidate {
                        at2,
                        kind: SnapKind::Item,
                        span: hspan,
                    });
                }
            }
        }
        if options.page {
            for (r, kind) in [(page, SnapKind::Page), (margins, SnapKind::Margin)] {
                let w = i64::from(r.width);
                let h = i64::from(r.height);
                let l2 = 2 * i64::from(r.left);
                let t2 = 2 * i64::from(r.top);
                let mut xa = vec![l2, l2 + 2 * w];
                let mut ya = vec![t2, t2 + 2 * h];
                if kind == SnapKind::Page {
                    xa.insert(1, l2 + w);
                    ya.insert(1, t2 + h);
                }
                xs.extend(xa.into_iter().map(|at2| Candidate {
                    at2,
                    kind,
                    span: None,
                }));
                ys.extend(ya.into_iter().map(|at2| Candidate {
                    at2,
                    kind,
                    span: None,
                }));
            }
        }
        let grid = match options.grid {
            Some(true) => Some(scene.sheet.snap_grid.spacing),
            Some(false) => None,
            None => scene
                .sheet
                .snap_grid
                .enabled
                .then_some(scene.sheet.snap_grid.spacing),
        }
        .filter(|s| *s > 0);
        Ok(SnapSession {
            moving: moving_box,
            single,
            xs,
            ys,
            others,
            grid,
            page,
            margins,
            options,
        })
    }

    /// The box the moving items make together at the drag's start.
    pub fn moving_box(&self) -> RectUm {
        self.moving
    }

    fn statics(&self, axis: Axis) -> &[Candidate] {
        match axis {
            Axis::X => &self.xs,
            Axis::Y => &self.ys,
        }
    }

    /// Equal-spacing candidates for the moved box `m` on `axis`: (anchor 0 start / 2 end, doubled target, gap).
    fn spacing_candidates(&self, m: &RectUm, axis: Axis) -> Vec<(usize, i64, i64)> {
        if !self.options.spacing {
            return Vec::new();
        }
        let mut row: Vec<&RectUm> = self
            .others
            .iter()
            .map(|o| &o.b)
            .filter(|b| overlap(b, m, axis))
            .collect();
        row.sort_by_key(|b| (lo(b, axis), hi(b, axis)));
        let mut gaps: Vec<i64> = Vec::new();
        for w in row.windows(2) {
            let g = lo(w[1], axis) - hi(w[0], axis);
            if g > 0 && !gaps.contains(&g) {
                gaps.push(g);
            }
        }
        let before = row
            .iter()
            .filter(|b| hi(b, axis) <= lo(m, axis))
            .max_by_key(|b| hi(b, axis));
        let after = row
            .iter()
            .filter(|b| lo(b, axis) >= hi(m, axis))
            .min_by_key(|b| lo(b, axis));
        let size = hi(m, axis) - lo(m, axis);
        let mut out = Vec::new();
        if let Some(b) = before {
            for g in &gaps {
                out.push((0, 2 * (hi(b, axis) + g), *g));
            }
        }
        if let Some(a) = after {
            for g in &gaps {
                out.push((2, 2 * (lo(a, axis) - g), *g));
            }
        }
        if let (Some(b), Some(a)) = (before, after) {
            let free = lo(a, axis) - hi(b, axis) - size;
            if free > 0 {
                out.push((0, hi(b, axis) + lo(a, axis) - size, 0));
            }
        }
        out
    }

    /// The best correction (doubled) on one axis for the moved box, and what it snapped to.
    fn best(&self, m: &RectUm, axis: Axis, tol2: i64) -> Option<(i64, SnapKind, i64)> {
        let s2 = 2 * lo(m, axis);
        let size2 = 2 * (hi(m, axis) - lo(m, axis));
        let anchors = [s2, s2 + size2 / 2, s2 + size2];
        // (distance, priority, order, anchor) → correction, kind, gap.
        let mut best: Option<MoveBest> = None;
        let mut consider = |key: (i64, u8, usize, usize), corr: i64, kind: SnapKind, gap: i64| {
            if key.0 <= tol2 && best.as_ref().is_none_or(|b| key < b.0) {
                best = Some((key, corr, kind, gap));
            }
        };
        for (order, c) in self.statics(axis).iter().enumerate() {
            for (k, a) in anchors.iter().enumerate() {
                let d = c.at2 - a;
                consider((d.abs(), c.kind.priority(), order, k), d, c.kind, 0);
            }
        }
        let base = self.statics(axis).len();
        for (n, (k, at2, gap)) in self.spacing_candidates(m, axis).into_iter().enumerate() {
            let d = at2 - anchors[k];
            consider(
                (d.abs(), SnapKind::Spacing.priority(), base + n, k),
                d,
                SnapKind::Spacing,
                gap,
            );
        }
        if let Some(g) = self.grid {
            let g = i64::from(g);
            for (k, a) in anchors.iter().enumerate() {
                if k == 1 {
                    continue;
                }
                let pos = a / 2;
                let near = (pos + g / 2).div_euclid(g) * g;
                let d = 2 * near - a;
                consider(
                    (d.abs(), SnapKind::Grid.priority(), usize::MAX - 1, k),
                    d,
                    SnapKind::Grid,
                    0,
                );
            }
        }
        best.map(|(_, corr, kind, gap)| (corr, kind, gap))
    }

    /// The lines of every candidate the snapped box now lies exactly on.
    fn lines(&self, m: &RectUm, axis: Axis) -> Vec<SnapLine> {
        let s2 = 2 * lo(m, axis);
        let size2 = 2 * (hi(m, axis) - lo(m, axis));
        let anchors = [s2, s2 + size2 / 2, s2 + size2];
        let o = other_axis(axis);
        let mut out: Vec<SnapLine> = Vec::new();
        for c in self.statics(axis) {
            if !anchors.contains(&c.at2) {
                continue;
            }
            let at = half(c.at2) as Um;
            let (from, to) = match c.span {
                Some((a, b)) => (a.min(lo(m, o)), b.max(hi(m, o))),
                None => (lo(&self.page, o), hi(&self.page, o)),
            };
            if let Some(l) = out.iter_mut().find(|l| l.at == at && l.kind == c.kind) {
                l.from = l.from.min(from as Um);
                l.to = l.to.max(to as Um);
            } else {
                out.push(SnapLine {
                    axis,
                    at,
                    from: from as Um,
                    to: to as Um,
                    kind: c.kind,
                });
            }
        }
        out.sort_by_key(|l| (l.at, l.kind, l.from));
        out
    }

    fn spacing_marks(&self, m: &RectUm, axis: Axis, gap: i64) -> Option<SpacingMark> {
        let o = other_axis(axis);
        let mut row: Vec<RectUm> = self
            .others
            .iter()
            .map(|x| x.b)
            .filter(|b| overlap(b, m, axis))
            .collect();
        row.push(*m);
        row.sort_by_key(|b| (lo(b, axis), hi(b, axis)));
        let mut segs = Vec::new();
        let mut g_used = gap;
        if gap == 0 {
            // Centred between two neighbours: both gaps are the one to mark.
            let before = row
                .iter()
                .filter(|b| hi(b, axis) <= lo(m, axis))
                .map(|b| hi(b, axis))
                .max()?;
            g_used = lo(m, axis) - before;
        }
        for w in row.windows(2) {
            let g = lo(&w[1], axis) - hi(&w[0], axis);
            if g == g_used {
                let c = (lo(&w[0], o).max(lo(&w[1], o)) + hi(&w[0], o).min(hi(&w[1], o))) / 2;
                let (a, b) = (hi(&w[0], axis), lo(&w[1], axis));
                let seg = match axis {
                    Axis::X => [[a as Um, c as Um], [b as Um, c as Um]],
                    Axis::Y => [[c as Um, a as Um], [c as Um, b as Um]],
                };
                segs.push(seg);
            }
        }
        (segs.len() >= 2).then_some(SpacingMark {
            axis,
            gap: g_used as Um,
            segments: segs,
        })
    }

    fn badges(&self, m: &RectUm) -> Vec<GapBadge> {
        let mut out = Vec::new();
        for axis in [Axis::X, Axis::Y] {
            let o = other_axis(axis);
            let row: Vec<&RectUm> = self
                .others
                .iter()
                .map(|x| &x.b)
                .filter(|b| overlap(b, m, axis))
                .collect();
            // Before (left or above) and after.
            let before = row
                .iter()
                .filter(|b| hi(b, axis) <= lo(m, axis))
                .max_by_key(|b| hi(b, axis));
            let after = row
                .iter()
                .filter(|b| lo(b, axis) >= hi(m, axis))
                .min_by_key(|b| lo(b, axis));
            let sides = [
                before
                    .map(|b| (hi(b, axis), lo(m, axis), *b, SnapKind::Item))
                    .or_else(|| {
                        (lo(&self.margins, axis) <= lo(m, axis)).then(|| {
                            (
                                lo(&self.margins, axis),
                                lo(m, axis),
                                &self.margins,
                                SnapKind::Margin,
                            )
                        })
                    }),
                after
                    .map(|a| (hi(m, axis), lo(a, axis), *a, SnapKind::Item))
                    .or_else(|| {
                        (hi(&self.margins, axis) >= hi(m, axis)).then(|| {
                            (
                                hi(m, axis),
                                hi(&self.margins, axis),
                                &self.margins,
                                SnapKind::Margin,
                            )
                        })
                    }),
            ];
            for (from, to, other, kind) in sides.into_iter().flatten() {
                let d = to - from;
                if d <= 0 {
                    continue;
                }
                let c = if kind == SnapKind::Item {
                    (lo(m, o).max(lo(other, o)) + hi(m, o).min(hi(other, o))) / 2
                } else {
                    (lo(m, o) + hi(m, o)) / 2
                };
                let (f, t) = match axis {
                    Axis::X => ([from as Um, c as Um], [to as Um, c as Um]),
                    Axis::Y => ([c as Um, from as Um], [c as Um, to as Um]),
                };
                out.push(GapBadge {
                    axis,
                    from: f,
                    to: t,
                    distance: d as Um,
                    text: mm1_text(d),
                    kind,
                });
            }
        }
        out
    }

    /// The drag's offset `delta` snapped within `tolerance` (micrometres), with what to paint.
    pub fn query(&self, delta: PointUm, tolerance: Um) -> SnapResult {
        let tol2 = 2 * i64::from(tolerance.max(0));
        let m = self
            .moving
            .translate(i64::from(delta[0]), i64::from(delta[1]));
        let bx = self.best(&m, Axis::X, tol2);
        let by = self.best(&m, Axis::Y, tol2);
        let dx = bx.map_or(0, |b| half(b.0));
        let dy = by.map_or(0, |b| half(b.0));
        let snapped = m.translate(dx, dy);
        let mut lines = self.lines(&snapped, Axis::X);
        lines.extend(self.lines(&snapped, Axis::Y));
        let mut spacing = Vec::new();
        if let Some((_, SnapKind::Spacing, g)) = bx
            && let Some(s) = self.spacing_marks(&snapped, Axis::X, g)
        {
            spacing.push(s);
        }
        if let Some((_, SnapKind::Spacing, g)) = by
            && let Some(s) = self.spacing_marks(&snapped, Axis::Y, g)
        {
            spacing.push(s);
        }
        let gaps = if self.options.badges {
            self.badges(&snapped)
        } else {
            Vec::new()
        };
        SnapResult {
            delta: [
                crate::units::sat(i64::from(delta[0]) + dx),
                crate::units::sat(i64::from(delta[1]) + dy),
            ],
            lines,
            gaps,
            spacing,
        }
    }

    /// A resize handle of the one moving item dragged to `to`, snapped: its
    /// moving edges to the candidates, its size to another item's. A turned
    /// item's handle is not snapped.
    pub fn query_resize(
        &self,
        handle: Handle,
        to: PointUm,
        tolerance: Um,
        keep_aspect: bool,
    ) -> ResizeSnap {
        let Some((_, frame, rotation)) = &self.single else {
            return ResizeSnap {
                to,
                ..ResizeSnap::default()
            };
        };
        if norm_mdeg(i64::from(*rotation)) != 0 {
            return ResizeSnap {
                to,
                ..ResizeSnap::default()
            };
        }
        let tol2 = 2 * i64::from(tolerance.max(0));
        let (hx, hy) = handle.dirs();
        let mut out = to;
        let mut sizes = Vec::new();
        for (axis, dir, fixed, p) in [
            (
                Axis::X,
                hx,
                if hx == 1 {
                    i64::from(frame.left)
                } else {
                    frame.right()
                },
                to[0],
            ),
            (
                Axis::Y,
                hy,
                if hy == 1 {
                    i64::from(frame.top)
                } else {
                    frame.bottom()
                },
                to[1],
            ),
        ] {
            if dir == 0 || (keep_aspect && axis == Axis::Y && hx != 0) {
                continue;
            }
            let p2 = 2 * i64::from(p);
            let mut best: Option<ResizeBest> = None;
            for (order, c) in self.statics(axis).iter().enumerate() {
                let d = c.at2 - p2;
                let key = (d.abs(), c.kind.priority(), order);
                if key.0 <= tol2 && best.as_ref().is_none_or(|b| key < b.0) {
                    best = Some((key, d, None));
                }
            }
            for (order, o) in self.others.iter().enumerate() {
                let size = hi(&o.b, axis) - lo(&o.b, axis);
                let edge2 = 2 * (fixed + i64::from(dir) * size);
                let d = edge2 - p2;
                let key = (
                    d.abs(),
                    SnapKind::Size.priority(),
                    self.statics(axis).len() + order,
                );
                if key.0 <= tol2 && best.as_ref().is_none_or(|b| key < b.0) {
                    best = Some((key, d, Some((o.id.clone(), size))));
                }
            }
            if let Some(g) = self.grid {
                let g = i64::from(g);
                let pos = i64::from(p);
                let near = (pos + g / 2).div_euclid(g) * g;
                let d = 2 * near - p2;
                let key = (d.abs(), SnapKind::Grid.priority(), usize::MAX);
                if key.0 <= tol2 && best.as_ref().is_none_or(|b| key < b.0) {
                    best = Some((key, d, None));
                }
            }
            if let Some((_, d, size)) = best {
                let v = crate::units::sat(i64::from(p) + half(d));
                match axis {
                    Axis::X => out[0] = v,
                    Axis::Y => out[1] = v,
                }
                if let Some((item, size)) = size {
                    sizes.push(SizeMatch {
                        axis,
                        item,
                        size: size as Um,
                    });
                }
            }
        }
        // The lines the new edges lie on.
        let nl = if hx == -1 {
            i64::from(out[0])
        } else {
            i64::from(frame.left)
        };
        let nr = if hx == 1 {
            i64::from(out[0])
        } else {
            frame.right()
        };
        let nt = if hy == -1 {
            i64::from(out[1])
        } else {
            i64::from(frame.top)
        };
        let nb = if hy == 1 {
            i64::from(out[1])
        } else {
            frame.bottom()
        };
        let nf = RectUm::from_edges(nl, nt, nr, nb);
        let mut lines = Vec::new();
        if hx != 0 {
            lines.extend(self.lines(&nf, Axis::X).into_iter().filter(|ln| {
                i64::from(ln.at)
                    == if hx == 1 {
                        nf.right()
                    } else {
                        i64::from(nf.left)
                    }
            }));
        }
        if hy != 0 {
            lines.extend(self.lines(&nf, Axis::Y).into_iter().filter(|ln| {
                i64::from(ln.at)
                    == if hy == 1 {
                        nf.bottom()
                    } else {
                        i64::from(nf.top)
                    }
            }));
        }
        ResizeSnap {
            to: out,
            lines,
            sizes,
        }
    }
}

/// A rotation snapped (design §5): with `step` (Shift) to 15°, otherwise to a quarter turn within 2°.
pub fn snap_rotation(angle: Mdeg, step: bool) -> Mdeg {
    let a = norm_mdeg(i64::from(angle));
    if step {
        return norm_mdeg((i64::from(a) + 7_500).div_euclid(15_000) * 15_000);
    }
    for q in [0, 90_000, 180_000, 270_000, 360_000] {
        if (i64::from(a) - q).abs() <= 2_000 {
            return norm_mdeg(q);
        }
    }
    a
}

/// Whether an item takes part in snapping as a target.
pub fn is_target(item: &Item) -> bool {
    !item.hidden
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_snaps_to_steps_and_quarters() {
        assert_eq!(snap_rotation(16_000, true), 15_000);
        assert_eq!(snap_rotation(22_500, true), 30_000);
        assert_eq!(snap_rotation(88_500, false), 90_000);
        assert_eq!(snap_rotation(358_500, false), 0);
        assert_eq!(snap_rotation(45_000, false), 45_000);
        assert_eq!(snap_rotation(-1_000, false), 0);
    }
}
