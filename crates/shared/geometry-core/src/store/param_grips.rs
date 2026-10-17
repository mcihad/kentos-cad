//! A dynamic block's insert's parameter grips in the store (docs/adr/0319 §1): the definition's grips
//! (`Dynamic::grips`) placed where the insert puts them, the value a grip dragged to a drawing point gives, and the
//! insert's outline with other values (a grip's preview). Worked by hand on a 2×1 block with a width.

use super::Store;
use super::tools::outline_paths;
use crate::block::{insert_local_xy, insert_matrix_xy, shows_nothing};
use crate::block_dynamic::{GripKind, ParamGrip};
use crate::entity::{ParamValue, Params, Shape};
use crate::geom::affine::{apply, apply_linear};
use crate::jsmath::{js_hypot, js_max};
use crate::vec2::Vec2;

impl Store {
    /// Insert `id`'s parameter grips on the drawing; none for an object that is not a dynamic block's insert. An
    /// angle's grip stands half the insert's larger side from its centre.
    pub fn param_grips(&self, id: f64) -> Vec<ParamGrip> {
        let Some(it) = self.get(id) else {
            return Vec::new();
        };
        let Shape::Insert {
            block,
            p,
            scale,
            y_scale,
            rotation,
            mirror,
            params,
            ..
        } = &it.shape
        else {
            return Vec::new();
        };
        let (Some(dynamic), Some(flat)) =
            (self.blocks().dynamic_of(block), self.blocks().get(block))
        else {
            return Vec::new();
        };
        let factors = (*scale, y_scale.unwrap_or(*scale));
        let m = insert_matrix_xy(flat.base, *p, factors, *rotation, mirror.unwrap_or(false));
        let reach = self.extent(Some(&[id])).map_or(1.0, |b| {
            js_max(b.max_x - b.min_x, b.max_y - b.min_y)
                / 2.0
                / js_max(factors.0.abs(), factors.1.abs())
        });
        dynamic
            .grips(params.as_ref(), reach)
            .into_iter()
            .map(|g| ParamGrip {
                at: apply(&m, g.at),
                from: g.from.map(|f| apply(&m, f)),
                along: g.along.and_then(|u| {
                    let w = apply_linear(&m, u);
                    let n = js_hypot(w.x, w.y);
                    (n > 0.0).then(|| Vec2::new(w.x / n, w.y / n))
                }),
                ..g
            })
            .collect()
    }

    /// The value insert `id`'s parameter `name` takes with its grip at drawing point `at`, its rule kept
    /// (`Dynamic::value_at`); none for another object, a grip it has not, or an insert without an inverse.
    pub fn param_value(&self, id: f64, name: &str, at: Vec2) -> Option<ParamValue> {
        let it = self.get(id)?;
        let Shape::Insert {
            block,
            p,
            scale,
            y_scale,
            rotation,
            mirror,
            params,
            ..
        } = &it.shape
        else {
            return None;
        };
        let dynamic = self.blocks().dynamic_of(block)?;
        let base = self.blocks().get(block)?.base;
        let factors = (*scale, y_scale.unwrap_or(*scale));
        let local = insert_local_xy(base, *p, factors, *rotation, mirror.unwrap_or(false), at)?;
        dynamic.value_at(name, local, params.as_ref())
    }

    /// Whether a flip parameter's grip: it is clicked, not dragged.
    pub fn param_flips(&self, id: f64, name: &str) -> bool {
        self.param_grips(id)
            .iter()
            .any(|g| g.name == name && g.kind == GripKind::Flip)
    }

    /// Insert `id` as `values` would draw it: its pieces' outlines (see `outline_paths` for the layout); nothing for
    /// another object.
    pub fn varied_outlines(&self, id: f64, values: &Params) -> Vec<f64> {
        let mut out = Vec::new();
        let Some(it) = self.get(id) else {
            return out;
        };
        let Shape::Insert { .. } = &it.shape else {
            return out;
        };
        let mut shape = it.shape.clone();
        if let Shape::Insert { params, .. } = &mut shape {
            *params = (!values.0.is_empty()).then(|| values.clone());
        }
        for piece in self.blocks().expand(&shape) {
            if !shows_nothing(&piece.shape) {
                outline_paths(&piece.shape, &mut out);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2×1 frame whose right side moves with “Genişlik” (0.5 … 3 on 0.25 steps), a flip about its middle, a
    /// visibility state hiding its diagonal, a turn of its diagonal about (1, 0) and a point moving its tick.
    const DOOR: &str = r#"[{"id":"k","name":"Kapı","base":{"x":0,"y":0},"entities":[
        {"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":0,"y":1}},
        {"kind":"line","id":2,"layerId":"","attrs":{},"a":{"x":2,"y":0},"b":{"x":2,"y":1}},
        {"kind":"line","id":3,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":2,"y":1}},
        {"kind":"point","id":4,"layerId":"","attrs":{},"p":{"x":1,"y":2}}],
      "dynamic":{"parameters":[
        {"kind":"linear","name":"Genişlik","from":{"x":0,"y":0},"to":{"x":2,"y":0},"rule":{"min":0.5,"max":3,"step":0.25}},
        {"kind":"flip","name":"Ters","a":{"x":1,"y":0},"b":{"x":1,"y":1}},
        {"kind":"visibility","name":"Görünüş","states":[{"name":"Tam"},{"name":"Sade","hidden":[3]}]},
        {"kind":"angle","name":"Açı","center":{"x":1,"y":0},"base":0},
        {"kind":"point","name":"Uç","at":{"x":1,"y":2}},
        {"kind":"linear","name":"Yarı","from":{"x":0,"y":0},"to":{"x":1,"y":0},"rule":{"expression":"Genişlik / 2"}}],
       "actions":[{"kind":"stretch","parameter":"Genişlik","min":{"x":1.5,"y":-1},"max":{"x":3,"y":2},"pieces":[2,3]},
                  {"kind":"flip","parameter":"Ters","pieces":[1,2,3]},
                  {"kind":"rotate","parameter":"Açı","pieces":[3]},
                  {"kind":"move","parameter":"Uç","pieces":[4]}]}}]"#;

    /// The door at (10, 20), twice its size, a quarter turn; the first plain, the second 3 wide and sparse.
    fn store() -> Store {
        let mut s = Store::new();
        s.set_blocks_json(DOOR).unwrap();
        s.put_json(
            r#"[{"id":1,"layerId":"a","kind":"insert","block":"k","p":{"x":10,"y":20},"scale":2,"rotation":1.5707963267948966},
                {"id":2,"layerId":"a","kind":"insert","block":"k","p":{"x":0,"y":0},"scale":1,"rotation":0,
                 "parameters":{"Genişlik":3,"Görünüş":"Sade"}},
                {"id":3,"layerId":"a","kind":"point","p":{"x":90,"y":190}}]"#,
        )
        .unwrap();
        s
    }

    #[test]
    fn grips_stand_where_the_values_put_them() {
        let s = store();
        let g = s.param_grips(2.0);
        let names: Vec<(&str, GripKind)> = g.iter().map(|g| (g.name.as_str(), g.kind)).collect();
        // The derived one, the visibility state and nothing else: no grip.
        assert_eq!(
            names,
            [
                ("Genişlik", GripKind::Linear),
                ("Ters", GripKind::Flip),
                ("Açı", GripKind::Angle),
                ("Uç", GripKind::Point)
            ]
        );
        assert_eq!(g[0].at, Vec2::new(3.0, 0.0));
        assert_eq!(g[0].from, Some(Vec2::new(0.0, 0.0)));
        assert_eq!(g[0].along, Some(Vec2::new(1.0, 0.0)));
        assert_eq!(g[1].at, Vec2::new(1.0, 0.5));
        assert_eq!(g[3].at, Vec2::new(1.0, 2.0));
        // Turned a quarter and doubled: the width's grip 4 above (10, 20), sliding north.
        let g = s.param_grips(1.0);
        assert!((g[0].at.x - 10.0).abs() < 1e-12 && (g[0].at.y - 24.0).abs() < 1e-12);
        let u = g[0].along.unwrap();
        assert!(u.x.abs() < 1e-12 && (u.y - 1.0).abs() < 1e-12);
        assert!(s.param_grips(3.0).is_empty());
    }

    #[test]
    fn a_grip_dragged_gives_a_value_its_rule_keeps() {
        let s = store();
        // 2.6 along the line: the nearest step 2.5; beyond the end 3; behind the start 0.5; on the line's side the same.
        let at = |x, y| s.param_value(2.0, "Genişlik", Vec2::new(x, y));
        assert_eq!(at(2.6, 0.0), Some(ParamValue::Number(2.5)));
        assert_eq!(at(2.6, 7.0), Some(ParamValue::Number(2.5)));
        assert_eq!(at(9.0, 0.0), Some(ParamValue::Number(3.0)));
        assert_eq!(at(-4.0, 0.0), Some(ParamValue::Number(0.5)));
        // On the turned insert the drawing's 2.6 north of (10, 20) is 1.3 of the block.
        assert_eq!(
            s.param_value(1.0, "Genişlik", Vec2::new(10.0, 22.6)),
            Some(ParamValue::Number(1.25))
        );
        // An angle toward the point, a point where it is; none for a flip, a derived one, a state.
        let a = s.param_value(2.0, "Açı", Vec2::new(1.0, 5.0)).unwrap();
        assert_eq!(a, ParamValue::Number(std::f64::consts::FRAC_PI_2));
        assert_eq!(
            s.param_value(2.0, "Uç", Vec2::new(4.0, 4.0)),
            Some(ParamValue::Point(Vec2::new(4.0, 4.0)))
        );
        assert_eq!(s.param_value(2.0, "Ters", Vec2::new(4.0, 4.0)), None);
        assert_eq!(s.param_value(2.0, "Yarı", Vec2::new(4.0, 0.0)), None);
        assert_eq!(s.param_value(2.0, "Görünüş", Vec2::new(4.0, 0.0)), None);
        assert!(s.param_flips(2.0, "Ters") && !s.param_flips(2.0, "Uç"));
    }

    #[test]
    fn a_preview_draws_the_values() {
        let s = store();
        // Plain: four lines' worth (the point draws none); flipped and 1 wide the right side stands at x = 1.
        let plain = s.varied_outlines(1.0, &Params(Vec::new()));
        assert!(!plain.is_empty());
        let narrow = Params(vec![("Genişlik".into(), ParamValue::Number(1.0))]);
        let flipped = narrow.with("Ters", ParamValue::Flag(true));
        assert_eq!(flipped.0[0].0, "Genişlik");
        let a = s.varied_outlines(2.0, &narrow);
        let b = s.varied_outlines(2.0, &flipped);
        assert_ne!(a, b);
        assert!(s.varied_outlines(3.0, &narrow).is_empty());
    }

    /// A visibility state hides its objects alike from the drawing, the click, the snap and the plot, which all draw
    /// the store's pieces (docs/adr/0319 §2): the sparse insert's diagonal is not there.
    #[test]
    fn a_hidden_piece_is_hidden_everywhere() {
        use crate::store::snap::SnapKind;
        let s = store();
        // The plain insert draws its three lines and its point; the sparse one its two sides and its point.
        assert_eq!(s.get(1.0).unwrap().shapes().count(), 4);
        assert_eq!(s.get(2.0).unwrap().shapes().count(), 3);
        // The plain insert's diagonal at (9, 22) is clicked; the sparse one's at (1.5, 0.5) is not.
        assert_eq!(s.hit(Vec2::new(9.0, 22.0), 0.01), Some(1.0));
        assert_eq!(s.hit(Vec2::new(1.5, 0.5), 0.01), None);
        // Nor does its middle snap: only the sides' middles do.
        let mid = s.snap(Vec2::new(1.5, 0.5), 0.2, SnapKind::Midpoint.bit(), None);
        assert!(mid.is_none(), "{mid:?}");
        let side = s.snap(Vec2::new(3.0, 0.5), 0.2, SnapKind::Midpoint.bit(), None);
        assert_eq!(side.map(|h| (h.point.x, h.point.y)), Some((3.0, 0.5)));
    }

    /// A flip that is on mirrors the other grips (docs/adr/0319 §1): the width's grip stands at the flipped side's
    /// end, and a point dragged there gives the width it shows.
    #[test]
    fn a_flipped_insert_has_its_grips_mirrored() {
        let mut s = store();
        s.put_json(
            r#"[{"id":4,"layerId":"a","kind":"insert","block":"k","p":{"x":0,"y":0},"scale":1,"rotation":0,
                 "parameters":{"Genişlik":1.5,"Ters":true}}]"#,
        )
        .unwrap();
        let g = s.param_grips(4.0);
        // Mirrored in x = 1: (1.5, 0) to (0.5, 0), sliding west; the flip's own grip stays.
        let near = |p: Vec2, x: f64, y: f64| (p.x - x).abs() < 1e-12 && (p.y - y).abs() < 1e-12;
        assert!(near(g[0].at, 0.5, 0.0), "{:?}", g[0].at);
        assert!(near(g[0].from.unwrap(), 2.0, 0.0));
        let u = g[0].along.unwrap();
        assert!((u.x + 1.0).abs() < 1e-12 && u.y.abs() < 1e-12);
        assert_eq!(g[1].at, Vec2::new(1.0, 0.5));
        // Dragged 2.25 west of the mirrored start: a width of 2.25.
        assert_eq!(
            s.param_value(4.0, "Genişlik", Vec2::new(-0.25, 0.3)),
            Some(ParamValue::Number(2.25))
        );
    }
}
