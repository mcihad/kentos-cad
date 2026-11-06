//! The screen's shortcuts of a layer build (docs/adr/1063), both asked by the view and both drawing what the plain
//! build draws:
//!
//! - **Instanced blocks** (§2): a block's definition drawn once, from its base point, and each of its inserts drawn by
//!   its placement (`BatchSink::instance`) instead of its pieces placed and drawn one by one. Only where a similarity
//!   places every piece exactly as the insert's own pieces are placed: an insert of uniform scale without a clip, all
//!   of its pieces shown, drawn by its simple look (`MODE_SET`), at the layer's own draw order; and only a definition
//!   whose pieces draw continuous strokes and solid areas (`BatchSink::recorder` refuses a dash, a line type, a
//!   pattern, a marker, an effect, a scale range). Anything else is drawn as before.
//! - **Curve detail levels** (§3): a circle's, an arc's and an ellipse's strokes come with coarser chords for far views
//!   (`batch::CURVE_LEVELS`), each within `batch::CURVE_TOLERANCE_PX` of the curve where it is drawn; the fine chords
//!   (as before) are drawn nearer. Snapping, picking, measures and paper use the source geometry, never these.

use std::collections::HashMap;
use std::sync::Arc;

use kentos_geometry_core::entity::{Shape, ellipse_geom, is_closed_outline, tessellate_circle};
use kentos_geometry_core::geom::arc::{ArcGeom, tessellate_arc};
use kentos_geometry_core::geom::ellipse::tessellate_ellipse;
use kentos_geometry_core::jsmath::{TAU, acos, js_hypot, js_max, js_min};
use kentos_geometry_core::store::{Expanded, Item, Store};
use kentos_geometry_core::Vec2;

use super::super::batch::{BatchSink, CURVE_LEVELS, CURVE_TOLERANCE_PX, Def};
use super::super::compile::{Env, Geom};
use super::{Cx, LayerObjects, MODE_SET, Program, RowValues, View, draw_object};

/// Inserts of one look of a definition a layer must have before its definition is drawn once.
const LEAST_ALIKE: u32 = 2;
/// Chords of a full turn at the finest (the drawn outlines' `OUTLINE_SEGMENTS`) and at the coarsest level.
const FINE_CHORDS: f64 = 72.0;
const LEAST_CHORDS: f64 = 4.0;

/// What makes two inserts draw alike: their definition (a dynamic insert's variant), the sets their pieces are drawn
/// with and their colour.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Look {
    flat: usize,
    sets: Vec<i32>,
    color: i32,
}

/// The definitions drawn once in a layer build, by look; none where one could not be.
#[derive(Default)]
pub(super) struct Instancer {
    counts: HashMap<Look, u32>,
    defs: HashMap<Look, Option<Arc<Def>>>,
    made: usize,
    /// Inserts drawn by their placement, and those of a look drawn piece by piece.
    pub(super) instanced: u64,
    pub(super) refused: u64,
}

/// The look of an insert that can be drawn by its placement, its definition and its placement; none for one drawn
/// piece by piece.
fn look_of(
    store: &Store,
    item: &Item,
    x: &Expanded,
    sets: Option<&[i32]>,
    [mode, color, band]: [i32; 3],
) -> Option<(Look, Arc<kentos_geometry_core::block::Flat>, [f64; 6])> {
    if mode != MODE_SET || band != 0 {
        return None;
    }
    let Shape::Insert { clip: None, .. } = &item.shape else {
        return None;
    };
    let (flat, m, similar) = store.blocks().placement_of(&item.shape)?;
    if !similar
        || flat.pieces.len() != x.shapes.len()
        || !(0..x.shapes.len()).all(|k| item.piece_shown(k))
    {
        return None;
    }
    let look = Look {
        flat: Arc::as_ptr(&flat) as usize,
        sets: sets.map(<[i32]>::to_vec).unwrap_or_default(),
        color,
    };
    Some((look, flat, m))
}

impl Instancer {
    /// Counts the layer's inserts by look, as the build will meet them (the pieces' sets one after another).
    pub(super) fn new(store: &Store, o: &LayerObjects<'_>) -> Instancer {
        let mut me = Instancer::default();
        let mut piece_at = 0usize;
        for (i, &id) in o.ids.iter().enumerate() {
            let Some(item) = store.get(id) else {
                continue;
            };
            let Some(x) = item.expanded.as_ref() else {
                continue;
            };
            let sets = o.pieces.get(piece_at..piece_at + x.shapes.len());
            piece_at += x.shapes.len();
            let mode = o.objects[4 * i];
            let color = o.objects[4 * i + 3];
            let band = o.bands.get(i).copied().unwrap_or(0);
            if let Some((look, _, _)) = look_of(store, item, x, sets, [mode, color, band]) {
                *me.counts.entry(look).or_default() += 1;
            }
        }
        me
    }

    /// Draws `item` by its placement when it can be (true), its definition drawn once on its look's first insert.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw(
        &mut self,
        store: &Store,
        item: &Item,
        x: &Expanded,
        sets: Option<&[i32]>,
        [mode, simple, color, band]: [i32; 4],
        program: &Program,
        values: &RowValues<'_, '_>,
        env: &Env<'_>,
        view: View,
        buf: &mut Vec<f64>,
        sink: &mut BatchSink,
        cx: &mut Cx,
    ) -> bool {
        let Some((look, flat, m)) = look_of(store, item, x, sets, [mode, color, band]) else {
            return false;
        };
        if self.counts.get(&look).copied().unwrap_or(0) < LEAST_ALIKE {
            return false;
        }
        let made = &mut self.made;
        let def = self
            .defs
            .entry(look)
            .or_insert_with(|| {
                *made += 1;
                let mut rec = BatchSink::recorder();
                rec.set_line_lod(view.line_lod && !view.plot_precision);
                for (k, piece) in flat.pieces.iter().enumerate() {
                    let shape = &piece.shape;
                    // A construction line is clipped to the view's box, which a definition's base knows nothing of.
                    if matches!(shape, Shape::Xline { .. } | Shape::Ray { .. }) {
                        return None;
                    }
                    let (fills, strokes) = view.hides(shape);
                    rec.hide(fills, strokes);
                    let set = sets.and_then(|s| s.get(k)).copied().unwrap_or(simple);
                    draw_object(shape, [MODE_SET, set, set, color], program, values, env, None, buf, &mut rec, cx);
                }
                rec.into_def(*made).map(Arc::new)
            })
            .clone();
        let Some(def) = def else {
            self.refused += 1;
            return false;
        };
        // The placement's uniform scale: its linear part is k times a turn (and a mirror).
        let k = (m[0] * m[3] - m[1] * m[2]).abs().sqrt();
        sink.hide(false, false);
        if !sink.instance(&def, &m, k) {
            return false;
        }
        self.instanced += 1;
        true
    }
}

/// The radius a curve's chords are counted by (an ellipse's larger semi-axis bounds its chords' distance); none for
/// what has no detail levels.
pub(super) fn level_radius(s: &Shape) -> Option<f64> {
    let r = match s {
        Shape::Circle { r, .. } | Shape::Arc { r, .. } => *r,
        Shape::Ellipse { major, ratio, .. } => {
            let a = js_hypot(major.x, major.y);
            js_max(a, a * ratio.abs())
        }
        _ => return None,
    };
    (r.is_finite() && r > 0.0).then_some(r)
}

/// Chords of a full turn that keep a circle of radius `r` within the tolerance at `px_per_m` logical px a metre:
/// the sagitta `R (1 − cos(π/n))` at most the tolerance, `R = r · px_per_m`; between the coarsest and the fine count.
fn chords(r: f64, px_per_m: f64) -> f64 {
    let px = r * px_per_m;
    if !(px > CURVE_TOLERANCE_PX) {
        return LEAST_CHORDS;
    }
    let n = (std::f64::consts::PI / acos(1.0 - CURVE_TOLERANCE_PX / px)).ceil();
    js_min(FINE_CHORDS, js_max(LEAST_CHORDS, n))
}

/// The curve's outline in `n` chords a full turn, as the fine one is made (its start, its sense, closed or open).
fn coarse(s: &Shape, n: f64) -> Option<Geom> {
    let (pts, closed): (Vec<Vec2>, bool) = match s {
        Shape::Circle { c, r } => (tessellate_circle(*c, *r, n), true),
        Shape::Arc { c, r, a0, a1 } => (
            tessellate_arc(
                &ArcGeom {
                    c: *c,
                    r: *r,
                    a0: *a0,
                    a1: *a1,
                },
                TAU / n,
            ),
            false,
        ),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => (
            tessellate_ellipse(&ellipse_geom(*c, *major, *ratio, *t0, *t1), n),
            is_closed_outline(s),
        ),
        _ => return None,
    };
    Some(Geom::Line(vec![(pts, closed)]))
}

/// The curve's coarser levels, one per `CURVE_LEVELS` ceiling (`r` its [`level_radius`]).
pub(super) fn levels(s: &Shape, r: f64) -> Vec<Geom> {
    CURVE_LEVELS
        .iter()
        .filter_map(|&ceiling| coarse(s, chords(r, ceiling)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_keep_the_tolerance_and_their_bounds() {
        // A sub-pixel circle: the coarsest count.
        assert_eq!(chords(0.3, 0.25), LEAST_CHORDS);
        // 1.2 px: five chords (π / acos(1 − 0.25 / 1.2) = 4.8).
        assert_eq!(chords(0.3, 4.0), 5.0);
        // 24 px: the sagitta within a quarter pixel, one chord fewer not.
        let n = chords(6.0, 4.0);
        let sag = |n: f64| 24.0 * (1.0 - kentos_geometry_core::jsmath::cos(std::f64::consts::PI / n));
        assert!(sag(n) <= CURVE_TOLERANCE_PX && sag(n - 1.0) > CURVE_TOLERANCE_PX, "{n}");
        // A big circle near: never more than the fine count.
        assert_eq!(chords(500.0, 4.0), FINE_CHORDS);
    }

    /// Three inserts of a block of a line and a circle (one turned, one mirrored) and one with a y factor of its own,
    /// built with the shortcuts on or off; the line drawn by `line`.
    fn built(line: &str, on: bool) -> Vec<kentos_geometry_core::api::json::Json> {
        use kentos_geometry_core::api::json::Json;
        let mut store = Store::new();
        store
            .set_blocks_json(
                r#"[{"id":"r","name":"R","base":{"x":0,"y":0},"entities":[
                {"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":1,"y":0}},
                {"kind":"circle","id":2,"layerId":"","attrs":{},"c":{"x":0.5,"y":0.5},"r":0.3}]}]"#,
            )
            .expect("blocks");
        store
            .put_json(
                r#"[{"id":1,"layerId":"a","kind":"insert","block":"r","p":{"x":10,"y":20},"scale":1,"rotation":0},
                {"id":2,"layerId":"a","kind":"insert","block":"r","p":{"x":30,"y":20},"scale":2,"rotation":0.5},
                {"id":3,"layerId":"a","kind":"insert","block":"r","p":{"x":50,"y":20},"scale":1,"rotation":0,"mirror":true},
                {"id":4,"layerId":"a","kind":"insert","block":"r","p":{"x":70,"y":20},"scale":1,"yScale":2,"rotation":0}]"#,
            )
            .expect("inserts");
        let program = Program::read(&format!(
            r##"{{"symbols":{{}},"renderer":null,"sets":[{{"line":{line}}}],"refs":[],"colors":["#FF0000"],"assets":{{}}}}"##
        ))
        .expect("program");
        let ids = [1.0, 2.0, 3.0, 4.0];
        let how = [MODE_SET, 0, 0, 0].repeat(4);
        let lens = vec![-1; 4 * program.fields.len()];
        let o = LayerObjects {
            ids: &ids,
            objects: &how,
            texts: "",
            text_lens: &lens,
            numbers: &[],
            pieces: &[0; 8],
            bands: &[],
        };
        let view = View {
            instancing: on,
            curve_lod: on,
            ..View::default()
        };
        let out = super::super::build_layer_with(&store, &program, &o, None, Vec2::new(0.0, 0.0), 1000.0, false, view)
            .expect("build");
        let Json::Arr(batches) = Json::parse(&out.json).expect("json") else {
            panic!("{}", out.json);
        };
        batches
    }

    #[test]
    fn similar_inserts_are_drawn_from_their_definition_and_the_rest_piece_by_piece() {
        use kentos_geometry_core::api::json::Json;
        const RED: &str = r##"{"type":"line","layers":[{"id":"r","type":"simpleLine","color":"#FF0000","width":0.3}]}"##;
        let count = |b: &Json| match b.get("inst").get("count") {
            Json::Num(n) => *n,
            _ => 0.0,
        };
        let on = built(RED, true);
        // The three similar inserts in the instanced batch; the one with a y factor of its own on its sibling's.
        assert_eq!(on.iter().map(count).sum::<f64>(), 3.0, "{on:?}");
        assert!(on.iter().any(|b| count(b) == 0.0 && matches!(b.get("len"), Json::Num(n) if *n > 0.0)));
        // The definition's circle comes with its coarser levels: main (the line), fine, one per ceiling.
        let lod = on.iter().find(|b| count(b) > 0.0).map(|b| b.get("lod").clone());
        let Some(Json::Arr(lod)) = lod else {
            panic!("levels: {on:?}");
        };
        assert_eq!(lod.len(), 2 + CURVE_LEVELS.len());
        // Off: nothing instanced, no levels.
        let off = built(RED, false);
        assert!(off.iter().all(|b| count(b) == 0.0 && matches!(b.get("lod"), Json::Null)));
        // A dashed line is drawn from the path's start, which a turned definition would move: piece by piece.
        let dashed = r##"{"type":"line","layers":[{"id":"r","type":"simpleLine","color":"#FF0000","width":0.3,"dash":[2,1]}]}"##;
        assert!(built(dashed, true).iter().all(|b| count(b) == 0.0));
    }

    #[test]
    fn a_coarse_outline_starts_and_turns_as_the_fine_one() {
        let c = Shape::Circle {
            c: Vec2::new(10.0, 0.0),
            r: 2.0,
        };
        let Some(Geom::Line(paths)) = coarse(&c, 8.0) else {
            panic!("a line");
        };
        assert_eq!(paths.len(), 1);
        assert!(paths[0].1);
        assert_eq!(paths[0].0.len(), 8);
        assert_eq!(paths[0].0[0], Vec2::new(12.0, 0.0));
        assert!(paths[0].0[2].y > 1.99);
    }
}
