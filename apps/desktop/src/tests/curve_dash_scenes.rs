//! The pictures of docs/adr/1121 and 1122 on the drawing area (the GPU): a CAD line type's dashes cut on a circle,
//! an arc, an ellipse, a NURBS curve and a bulge path by their own arc length, beside blue ticks at every period of
//! true arc length, a dash-dot arc, and a dotted wide polyline whose dots are the band's cross-sections.
//! `tools_screens` takes them:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=egri-desen-cizim,egri-desen-yakin,noktali-bant cargo test -p kentos-desktop tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::tools_scenes::{E, N, Objects, layer, open_edited};
use crate::tools_screens::Scene;
use serde_json::json;

fn drawing(app: &mut App) {
    let mut o = Objects::new();
    o.circle("p", [-60.0, 20.0], 15.0);
    o.arc("p", [-20.0, 20.0], 15.0, 0.3, 2.9);
    o.ellipse("p", [25.0, 20.0], [20.0, 0.0], 0.45, 0.0, 0.0);
    o.nurbs("p", 2, &[[50.0, 5.0], [70.0, 40.0], [90.0, 5.0]], &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0], json!({}));
    o.bulged("p", &[[-80.0, -25.0], [-50.0, -25.0], [-50.0, -5.0]], &[0.0, 0.6, 0.0]);
    o.arc("d", [-5.0, -30.0], 15.0, 0.2, 3.0);
    let band = o.path("n", &[[25.0, -30.0], [85.0, -30.0]], false);
    o.fields(band, json!({"widths": [[1.5, 1.5], [0.0, 0.0]]}));
    // Blue references: every 4 m of the circle's and the arc's own arc length (r·θ).
    for (cx, a0, sweep) in [(-60.0, 0.0, std::f64::consts::TAU), (-20.0, 0.3, 2.6)] {
        let n = (sweep * 15.0 / 4.0_f64).floor() as i32;
        for k in 0..=n {
            let a = a0 + f64::from(k) * 4.0 / 15.0;
            let (u, v) = (a.cos(), a.sin());
            o.line("ref", [cx + 16.0 * u, 20.0 + 16.0 * v], [cx + 18.0 * u, 20.0 + 18.0 * v]);
        }
    }
    open_edited(app, o, |d| {
        let pattern = |id: &str, name: &str, color: &str, pattern: &str| {
            let mut l = layer(id, name, color, 0.35);
            l["style"]["lineType"] = json!("dashed");
            l["style"]["linePattern"] = json!(pattern);
            l
        };
        d["layers"] = json!([
            pattern("p", "DESEN", "fg", "P"),
            pattern("d", "NOKTALI", "#E5484D", "D"),
            pattern("n", "BANT", "#30A46C", "N"),
            layer("ref", "REFERANS", "#3E63DD", 0.25),
        ]);
        d["activeLayer"] = json!("p");
        d["settings"]["drawingUnit"] = json!("m");
        d["settings"]["lineTypes"] = json!([
            {"name": "P", "unit": "drawing", "elements": [{"length": 3.0}, {"length": -1.0}]},
            {"name": "D", "unit": "drawing", "elements": [{"length": 4.0}, {"length": -1.0}, {"length": 0.0}, {"length": -1.0}]},
            {"name": "N", "unit": "drawing", "elements": [{"length": 0.0}, {"length": -2.0}]}
        ]);
    });
    crate::files_testing::make_cad(app);
}

fn fit(app: &mut App, b: [f64; 4]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + b[0], min_y: N + b[1], max_x: E + b[2], max_y: N + b[3] },
        24.0,
    );
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("egri-desen-cizim", |app| {
            drawing(app);
            fit(app, [-82.0, -40.0, 95.0, 42.0]);
        }),
        ("egri-desen-yakin", |app| {
            drawing(app);
            fit(app, [-80.0, 18.0, -40.0, 40.0]);
        }),
        ("noktali-bant", |app| {
            drawing(app);
            fit(app, [20.0, -36.0, 50.0, -24.0]);
        }),
    ]
}
