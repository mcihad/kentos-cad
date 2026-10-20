//! `cad.inserts.clip` v1 (docs/adr/0351 §1; plan/cad.md `CAD-REF-12`): an insert's clipping boundary (Kırp; AutoCAD's
//! XCLIP), for a block and an external reference alike. The boundary is given in the drawing, a rectangle or a
//! polygon, and kept in the definition's coordinates (through the inverse of the insert's placement), so it moves,
//! turns and scales with the insert; `inverted` keeps the outside, `frame` shows its frame on the screen or plots it.
//! The content is cut by the core (`kentos_geometry_core::block_clip`). Inserts on a locked layer are left out and
//! counted (docs/adr/0278); one undo step “Kırp”. The shared cases are `fixtures/commands/v1/cad.inserts.clip.json`.

use kentos_commands::{
    ClipBoundary, ClipFrame, CommandResult, CommandWarning, Entity, InsertClip, InsertClipped, InsertsClip,
    InsertsClipOperation, InsertsClipped, MAX_CLIPPING_CORNERS, Vec2, codes,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::block::placement_xy;
use kentos_geometry_core::predicates::orient2d;
use kentos_geometry_core::vec2::Vec2 as P;

use crate::ExecutionContext;
use crate::checks::{self, Stop};

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

/// The ring's corners in the drawing, its closing corner (one the same as the first) dropped.
fn ring_of(boundary: &ClipBoundary) -> Vec<Vec2> {
    match boundary {
        ClipBoundary::Rectangle { a, b } => {
            let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
            let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
            vec![Vec2 { x: x0, y: y0 }, Vec2 { x: x1, y: y0 }, Vec2 { x: x1, y: y1 }, Vec2 { x: x0, y: y1 }]
        }
        ClipBoundary::Polygon { points } => {
            let mut ring = points.clone();
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            ring
        }
    }
}

fn p(v: Vec2) -> P {
    P::new(v.x, v.y)
}

/// Whether the closed segments `a`–`b` and `c`–`d` meet (exact orientations).
fn meet(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2) = (orient2d(a, b, c), orient2d(a, b, d));
    let (o3, o4) = (orient2d(c, d, a), orient2d(c, d, b));
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    let on = |a: P, b: P, q: P, o: f64| {
        o == 0.0 && q.x >= a.x.min(b.x) && q.x <= a.x.max(b.x) && q.y >= a.y.min(b.y) && q.y <= a.y.max(b.y)
    };
    on(a, b, c, o1) || on(a, b, d, o2) || on(c, d, a, o3) || on(c, d, b, o4)
}

/// The first two edges of the ring that meet other than at their shared corner, by their numbers: the pair with the
/// lowest first number, then the lowest second.
fn crossing(ring: &[Vec2]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    let n = ring.len();
    let edge = |i: usize| (p(ring[i]), p(ring[(i + 1) % n]));
    // Edges sorted by their left end, so each meets only those starting before it ends.
    let mut order: Vec<usize> = (0..n).collect();
    let left = |i: usize| ring[i].x.min(ring[(i + 1) % n].x);
    let right = |i: usize| ring[i].x.max(ring[(i + 1) % n].x);
    order.sort_by(|&a, &b| left(a).total_cmp(&left(b)));
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if left(j) > right(i) {
                break;
            }
            let (lo, hi) = (i.min(j), i.max(j));
            let next = hi == lo + 1 || (lo == 0 && hi == n - 1);
            let (a, b) = edge(lo);
            let (c, d) = edge(hi);
            if next {
                // Neighbours meet at their shared corner only: not folding back along each other.
                let (shared, x, y) = if hi == lo + 1 { (b, a, d) } else { (a, b, c) };
                if orient2d(x, shared, y) == 0.0
                    && (x.x - shared.x) * (y.x - shared.x) + (x.y - shared.y) * (y.y - shared.y) > 0.0
                {
                    best = best.min(Some((lo, hi))).or(Some((lo, hi)));
                }
                continue;
            }
            if meet(a, b, c, d) {
                best = best.min(Some((lo, hi))).or(Some((lo, hi)));
            }
        }
    }
    best
}

/// The ring's twice-signed area.
fn area2(ring: &[Vec2]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| ring[i].x * ring[(i + 1) % n].y - ring[(i + 1) % n].x * ring[i].y).sum()
}

/// The boundary given, checked: its corners in the drawing.
fn boundary(b: &ClipBoundary) -> Result<Vec<Vec2>, Stop> {
    let ring = ring_of(b);
    let at = "operation.boundary";
    let bad = |m: String| refuse(codes::INVALID_CLIPPING, m, at);
    if ring.iter().any(|v| !v.x.is_finite() || !v.y.is_finite()) {
        return Err(bad("Kırpma sınırının bir köşesi sayı değil.".into()));
    }
    if ring.len() < 3 {
        return Err(bad("Kırpma sınırı en az üç köşeli bir çokgendir.".into()));
    }
    if ring.len() > MAX_CLIPPING_CORNERS {
        return Err(bad(format!("Kırpma sınırı en çok {MAX_CLIPPING_CORNERS} köşeli olabilir.")));
    }
    if area2(&ring) == 0.0 {
        return Err(bad("Kırpma sınırının alanı yok: köşeleri bir doğru üzerinde ya da üst üste.".into()));
    }
    if let Some((i, j)) = crossing(&ring) {
        return Err(bad(format!("Kırpma sınırının {}. ve {}. kenarı kesişiyor; kendini kesmeyen bir sınır çizin.", i + 1, j + 1)));
    }
    Ok(ring)
}

/// The ring in the insert's definition: its corners through the inverse of the placement, the base put back. None
/// for a placement without an inverse.
fn in_definition(ring: &[Vec2], insert: &kentos_commands::InsertEntity, base: Vec2) -> Option<Vec<Vec2>> {
    let m = placement_xy(
        P::new(insert.p.x, insert.p.y),
        insert.scale,
        insert.y_scale.unwrap_or(insert.scale),
        insert.rotation,
        insert.mirror,
    );
    let det = m[0] * m[3] - m[1] * m[2];
    if det == 0.0 || !det.is_finite() {
        return None;
    }
    Some(
        ring.iter()
            .map(|v| {
                let (x, y) = (v.x - m[4], v.y - m[5]);
                Vec2 { x: (m[3] * x - m[2] * y) / det + base.x, y: (m[0] * y - m[1] * x) / det + base.y }
            })
            .collect(),
    )
}

struct Planned {
    writes: Vec<(Slot, Entity, InsertClipped)>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &InsertsClip) -> Result<Planned, Stop> {
    checks::uids(&input.uids, "Kırpılacak blok ya da dış başvuru verilmedi.")?;
    let named = checks::named(doc, &input.uids)?;
    for (at, _, e, uid) in &named {
        if !matches!(e, Entity::Insert(_)) {
            return Err(refuse(
                codes::NOT_AN_INSERT,
                format!("“{uid}” kimlikli nesne bir blok ya da dış başvuru değil; yalnız onlar kırpılır."),
                &format!("uids[{at}]"),
            ));
        }
    }
    let (open, warning) = checks::skip_locked(doc, named)?;
    let mut warnings: Vec<CommandWarning> = warning.into_iter().collect();
    let ring = match &input.operation {
        InsertsClipOperation::Set { boundary: b, .. } => Some(boundary(b)?),
        _ => None,
    };
    let mut writes = Vec::new();
    for (at, slot, e, uid) in open {
        let Entity::Insert(ins) = e else { continue };
        let mut next = ins.clone();
        match &input.operation {
            InsertsClipOperation::Set { inverted, frame, .. } => {
                let base = doc.blocks().iter().find(|b| b.id == ins.block).map_or(Vec2 { x: 0.0, y: 0.0 }, |b| b.base);
                let Some(local) = in_definition(ring.as_deref().unwrap_or_default(), ins, base) else {
                    return Err(refuse(
                        codes::INVALID_CLIPPING,
                        format!("“{uid}” kimlikli yerleştirmenin ölçeği sıfır; kırpma sınırı onun tanımına taşınamaz."),
                        &format!("uids[{at}]"),
                    ));
                };
                next.clipping = Some(InsertClip { boundary: local, inverted: *inverted, frame: *frame });
            }
            InsertsClipOperation::Remove => {
                if next.clipping.take().is_none() {
                    continue;
                }
            }
            InsertsClipOperation::Frame { frame } => {
                let Some(c) = next.clipping.as_mut() else { continue };
                c.frame = *frame;
            }
            InsertsClipOperation::Invert => {
                let Some(c) = next.clipping.as_mut() else { continue };
                c.inverted = !c.inverted;
            }
        }
        writes.push((
            slot,
            Entity::Insert(next.clone()),
            InsertClipped { uid: uid.clone(), clipping: next.clipping },
        ));
    }
    if writes.is_empty() {
        return Err(refuse(
            codes::NOT_CLIPPED,
            "Verilen yerleştirmelerin hiçbiri kırpılmış değil; önce bir kırpma sınırı verin.".into(),
            "uids",
        ));
    }
    // A boundary that leaves an insert nothing is said, not refused: the content may still change.
    if matches!(input.operation, InsertsClipOperation::Set { .. } | InsertsClipOperation::Invert) {
        let blocks = crate::blocks::core_blocks(doc.blocks());
        let empty: Vec<&str> = writes
            .iter()
            .filter(|(_, e, _)| {
                let shape = crate::geometry::shape(e);
                blocks.expand(&shape).iter().all(|piece| is_frame(piece, e))
            })
            .map(|(.., c)| c.uid.as_str())
            .collect();
        if !empty.is_empty() {
            warnings.push(CommandWarning {
                code: codes::CLIP_EMPTY.into(),
                message: format!(
                    "Kırpma sınırı {} yerleştirmede çizilecek bir şey bırakmıyor; sınırın içeriğin üstünde olduğundan emin olun.",
                    empty.len()
                ),
                path: Some("operation".into()),
            });
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Planned { writes, warnings })
}

/// Whether a piece is the plotted frame the clipping added (it alone is not content).
fn is_frame(piece: &kentos_geometry_core::block::Piece, e: &Entity) -> bool {
    let Entity::Insert(i) = e else { return false };
    i.clipping.as_ref().is_some_and(|c| c.frame == ClipFrame::Plot)
        && matches!(piece.shape, kentos_geometry_core::entity::Shape::Polyline { closed: true, .. })
        && piece.attribute.is_none()
}

fn in_order(mut writes: Vec<(Slot, Entity, InsertClipped)>) -> Vec<(Slot, Entity, InsertClipped)> {
    writes.sort_by_key(|(slot, ..)| slot.0);
    writes
}

pub fn validate(cx: &ExecutionContext<'_>, input: &InsertsClip) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &InsertsClip) -> CommandResult<InsertsClipped> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: InsertsClipped {
                inserts: in_order(p.writes).into_iter().map(|(.., c)| c).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes it, one undo step “Kırp”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: InsertsClip) -> CommandResult<InsertsClipped> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_INSERTS_CLIP,
        kentos_commands::CAD_INSERTS_CLIP_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: InsertsClip) -> CommandResult<InsertsClipped> {
    let Planned { writes, warnings } = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let mut out = Vec::with_capacity(writes.len());
    let writes: Vec<(Slot, Entity)> = in_order(writes)
        .into_iter()
        .map(|(slot, e, c)| {
            out.push(c);
            (slot, e)
        })
        .collect();
    let _ = cx.doc.transact("Kırp", |doc| -> Result<(), ()> {
        for (slot, e) in writes {
            doc.update(slot, e);
        }
        Ok(())
    });
    CommandResult::Completed {
        output: InsertsClipped { inserts: out, revision: cx.doc.revision().to_string() },
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    #[test]
    fn crossing_rings_are_found() {
        assert_eq!(crossing(&[v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0), v(0.0, 4.0)]), None);
        assert!(crossing(&[v(0.0, 0.0), v(4.0, 4.0), v(4.0, 0.0), v(0.0, 4.0)]).is_some());
        // A spike folding back along its neighbour.
        assert!(crossing(&[v(0.0, 0.0), v(4.0, 0.0), v(2.0, 0.0), v(2.0, 3.0)]).is_some());
        // A ring touching itself at a corner.
        assert!(crossing(&[v(0.0, 0.0), v(4.0, 0.0), v(2.0, 2.0), v(4.0, 4.0), v(0.0, 4.0), v(2.0, 2.0)]).is_some());
    }
}
