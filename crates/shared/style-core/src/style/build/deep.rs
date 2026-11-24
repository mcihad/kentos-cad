//! Deep zoom (plan/render.md RM-24): an object's drawn geometry with its paths and rings split at the edge of the
//! view's box (`View::deep`, `kentos_geometry_core::geom::deep`). The pieces in the view have their ends near it,
//! so the GPU's float32 offsets from the camera place them to a fraction of a pixel however deep the view.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::deep::split_at_box;
use kentos_geometry_core::geometry::Bounds;

use crate::style::compile::Geom;

/// `g` with every path and ring split where it crosses the edge of `b`; a mark as it is.
pub(super) fn split(g: Geom, b: &Bounds) -> Geom {
    let ring = |r: Vec<Vec2>| split_at_box(&r, true, b).unwrap_or(r);
    match g {
        Geom::Line(paths) => Geom::Line(
            paths
                .into_iter()
                .map(|(p, closed)| (split_at_box(&p, closed, b).unwrap_or(p), closed))
                .collect(),
        ),
        Geom::Fill(rings) => Geom::Fill(rings.into_iter().map(ring).collect()),
        Geom::Fills(parts) => Geom::Fills(
            parts
                .into_iter()
                .map(|rings| rings.into_iter().map(ring).collect())
                .collect(),
        ),
        mark @ Geom::Marker(_) => mark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::build::{LayerObjects, MODE_SET, Program, View, build_layer_with};
    use kentos_geometry_core::api::json::Json;
    use kentos_geometry_core::store::Store;

    const BOX: Bounds = Bounds {
        min_x: -1.0,
        min_y: -1.0,
        max_x: 1.0,
        max_y: 1.0,
    };

    #[test]
    fn paths_and_rings_are_split_and_marks_kept() {
        let v = Vec2::new;
        let Geom::Line(paths) = split(Geom::Line(vec![(vec![v(-50.0, 0.0), v(50.0, 0.0)], false)]), &BOX) else {
            panic!("a line");
        };
        assert_eq!(paths[0].0, [v(-50.0, 0.0), v(-1.0, 0.0), v(1.0, 0.0), v(50.0, 0.0)]);
        let square = vec![v(-50.0, -50.0), v(50.0, -50.0), v(50.0, 0.5), v(-50.0, 0.5)];
        let Geom::Fill(rings) = split(Geom::Fill(vec![square.clone()]), &BOX) else {
            panic!("an area");
        };
        // The top edge crosses the box: its two points put in, and nothing else.
        assert_eq!(rings[0].len(), square.len() + 2);
        let Geom::Fills(parts) = split(Geom::Fills(vec![vec![square]]), &BOX) else {
            panic!("areas");
        };
        assert_eq!(parts[0][0], rings[0]);
        let mark = split(Geom::Marker(v(100.0, 100.0)), &BOX);
        assert!(matches!(mark, Geom::Marker(p) if p == v(100.0, 100.0)));
    }

    /// The segments a layer of one long line draws, built with `view`.
    fn segments(view: View) -> f64 {
        let mut store = Store::new();
        store
            .put_json(r#"[{"id":1,"layerId":"a","kind":"line","attrs":{},"a":{"x":-5000,"y":0.25},"b":{"x":5000,"y":0.25}}]"#)
            .expect("line");
        let program = Program::read(
            r##"{"symbols":{},"renderer":null,"sets":[{"line":{"type":"line","layers":[{"id":"r","type":"simpleLine","color":"#FF0000","width":0.3}]}}],"refs":[],"colors":["#FF0000"],"assets":{}}"##,
        )
        .expect("program");
        let ids = [1.0];
        let how = [MODE_SET, 0, 0, 0];
        let lens = vec![-1; program.fields.len()];
        let o = LayerObjects {
            ids: &ids,
            objects: &how,
            texts: "",
            text_lens: &lens,
            numbers: &[],
            pieces: &[0; 8],
            bands: &[],
        };
        let out = build_layer_with(&store, &program, &o, None, Vec2::new(0.0, 0.0), 1000.0, false, view)
            .expect("build");
        let Json::Arr(batches) = Json::parse(&out.json).expect("json") else {
            panic!("{}", out.json);
        };
        batches
            .iter()
            .map(|b| match b.get("len") {
                Json::Num(n) => *n,
                _ => 0.0,
            })
            .sum()
    }

    #[test]
    fn a_deep_view_draws_a_long_line_through_it_in_three_pieces() {
        let plain = segments(View::default());
        let deep = segments(View {
            deep: Some(BOX),
            ..View::default()
        });
        assert!(plain > 0.0, "the line is drawn");
        assert!(deep > plain, "the line's piece in the box is drawn on its own: {plain} → {deep}");
        // A box the line misses: drawn as it is.
        let beside = segments(View {
            deep: Some(Bounds {
                min_x: -1.0,
                min_y: 10.0,
                max_x: 1.0,
                max_y: 12.0,
            }),
            ..View::default()
        });
        assert_eq!(beside, plain);
    }
}
