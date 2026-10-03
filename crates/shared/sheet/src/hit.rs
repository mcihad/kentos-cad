//! What is under the pointer or inside a selection rectangle (design §5):
//! turned frames by their turned shape, lines by their path. The master
//! page's items are never hit (they are not selectable); hidden ones neither.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::error::Result;
use crate::kinds::ItemKind;
use crate::model::{Item, ItemId, SheetBook};
use crate::ops::Handle;
use crate::scene::Scene;
use crate::units::{Mdeg, PointUm, RectUm, Um, corners, rotate};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum RectMode {
    /// Wholly inside the rectangle (a drag to the right).
    Contain,
    /// Touching it (a drag to the left).
    Intersect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PointQuery {
    pub at: PointUm,
    pub tolerance: Um,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RectQuery {
    pub rect: RectUm,
    pub mode: RectMode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct HandleQuery {
    /// The selected item whose handles are asked.
    pub item: ItemId,
    pub at: PointUm,
    pub tolerance: Um,
    /// How far above the top edge the rotation handle stands.
    #[serde(default)]
    pub rotate_offset: Um,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum HitQuery {
    Point(PointQuery),
    Rect(RectQuery),
    Handle(HandleQuery),
}

/// An item hit, and the outermost group it is in (what a click selects).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Hit {
    pub item: ItemId,
    pub top: ItemId,
}

/// What a handle query found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum HandleHit {
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
    Rotate,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Hits {
    /// Topmost first.
    pub hits: Vec<Hit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub handle: Option<HandleHit>,
}

/// The handles of a frame turned by `rotation`, on the paper: eight resize handles and the rotation handle above the top edge.
pub fn handle_points(
    frame: &RectUm,
    rotation: Mdeg,
    rotate_offset: Um,
) -> Vec<(HandleHit, [f64; 2])> {
    let c = frame.center();
    let (l, t) = (f64::from(frame.left), f64::from(frame.top));
    let (r, b) = (frame.right() as f64, frame.bottom() as f64);
    let (mx, my) = (c[0], c[1]);
    let local = [
        (HandleHit::Nw, [l, t]),
        (HandleHit::N, [mx, t]),
        (HandleHit::Ne, [r, t]),
        (HandleHit::E, [r, my]),
        (HandleHit::Se, [r, b]),
        (HandleHit::S, [mx, b]),
        (HandleHit::Sw, [l, b]),
        (HandleHit::W, [l, my]),
        (HandleHit::Rotate, [mx, t - f64::from(rotate_offset)]),
    ];
    local
        .into_iter()
        .map(|(h, p)| (h, rotate(p, c, rotation)))
        .collect()
}

impl HandleHit {
    /// The resize handle, or none for the rotation handle.
    pub fn resize(self) -> Option<Handle> {
        Some(match self {
            HandleHit::N => Handle::N,
            HandleHit::Ne => Handle::Ne,
            HandleHit::E => Handle::E,
            HandleHit::Se => Handle::Se,
            HandleHit::S => Handle::S,
            HandleHit::Sw => Handle::Sw,
            HandleHit::W => Handle::W,
            HandleHit::Nw => Handle::Nw,
            HandleHit::Rotate => return None,
        })
    }
}

fn dist_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (x, y) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (x * x + y * y).sqrt()
}

/// Whether `p` is on the item: inside its turned frame (grown by `tol`), or near a line item's path.
pub fn point_on(item: &Item, p: [f64; 2], tol: f64) -> bool {
    let c = item.frame.center();
    let q = rotate(p, c, -item.rotation);
    if let ItemKind::Line(l) = &item.kind {
        let pts: Vec<[f64; 2]> = l
            .points
            .iter()
            .map(|pt| crate::kinds::frame_point(&item.frame, *pt))
            .collect();
        let reach = tol + f64::from(l.stroke.width) / 2.0;
        return pts
            .windows(2)
            .any(|w| dist_to_segment(q, w[0], w[1]) <= reach);
    }
    let f = &item.frame;
    q[0] >= f64::from(f.left) - tol
        && q[0] <= f.right() as f64 + tol
        && q[1] >= f64::from(f.top) - tol
        && q[1] <= f.bottom() as f64 + tol
}

/// Whether a convex polygon and an upright rectangle meet (separating axes).
fn polygon_meets_rect(poly: &[[f64; 2]; 4], r: &RectUm) -> bool {
    let rect = [
        [f64::from(r.left), f64::from(r.top)],
        [r.right() as f64, f64::from(r.top)],
        [r.right() as f64, r.bottom() as f64],
        [f64::from(r.left), r.bottom() as f64],
    ];
    let axes = [
        [1.0, 0.0],
        [0.0, 1.0],
        [poly[1][0] - poly[0][0], poly[1][1] - poly[0][1]],
        [poly[3][0] - poly[0][0], poly[3][1] - poly[0][1]],
    ];
    for ax in axes {
        let proj = |pts: &[[f64; 2]; 4]| {
            let mut lo = f64::MAX;
            let mut hi = f64::MIN;
            for p in pts {
                let v = p[0] * ax[0] + p[1] * ax[1];
                lo = lo.min(v);
                hi = hi.max(v);
            }
            (lo, hi)
        };
        let (a0, a1) = proj(poly);
        let (b0, b1) = proj(&rect);
        if a1 < b0 || b1 < a0 {
            return false;
        }
    }
    true
}

/// The outermost group above an item (or the item itself).
fn top_of(items: &[Item], item: &Item) -> ItemId {
    let mut top = item.id.clone();
    let mut up = item.group.clone();
    let mut n = 0;
    while let Some(g) = up {
        top = g.clone();
        n += 1;
        if n > items.len() {
            break;
        }
        up = items
            .iter()
            .find(|i| i.id == g)
            .and_then(|i| i.group.clone());
    }
    top
}

/// The items under a point, inside a rectangle, or the handle of a selected item under the pointer.
pub fn hit_test(book: &SheetBook, sheet_id: &str, query: &HitQuery) -> Result<Hits> {
    let scene = Scene::new(book, sheet_id)?;
    let items = &scene.sheet.items;
    let mut out = Hits::default();
    match query {
        HitQuery::Point(q) => {
            let p = [f64::from(q.at[0]), f64::from(q.at[1])];
            for it in items.iter().rev() {
                if it.is_group() || !scene.shown(it) {
                    continue;
                }
                if point_on(it, p, f64::from(q.tolerance.max(0))) {
                    out.hits.push(Hit {
                        item: it.id.clone(),
                        top: top_of(items, it),
                    });
                }
            }
        }
        HitQuery::Rect(q) => {
            let mut seen = std::collections::BTreeSet::new();
            for it in items.iter().rev() {
                if it.is_group() || !scene.shown(it) {
                    continue;
                }
                let cs = corners(&it.frame, it.rotation);
                let inside = |p: &[f64; 2]| q.rect.contains(*p);
                let ok = match q.mode {
                    RectMode::Contain => cs.iter().all(inside),
                    RectMode::Intersect => polygon_meets_rect(&cs, &q.rect),
                };
                if ok {
                    let top = top_of(items, it);
                    if seen.insert(top.clone()) {
                        out.hits.push(Hit {
                            item: it.id.clone(),
                            top,
                        });
                    }
                }
            }
        }
        HitQuery::Handle(q) => {
            if let Some(it) = items.iter().find(|i| i.id == q.item) {
                let p = [f64::from(q.at[0]), f64::from(q.at[1])];
                let tol = f64::from(q.tolerance.max(0));
                let best = handle_points(&it.frame, it.rotation, q.rotate_offset)
                    .into_iter()
                    .filter(|(h, _)| *h != HandleHit::Rotate || q.rotate_offset > 0)
                    .map(|(h, hp)| {
                        let (dx, dy) = (hp[0] - p[0], hp[1] - p[1]);
                        (h, (dx * dx + dy * dy).sqrt())
                    })
                    .filter(|(_, d)| *d <= tol)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                out.handle = best.map(|(h, _)| h);
            }
        }
    }
    Ok(out)
}
