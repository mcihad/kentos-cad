//! The drawing area's text after random changes (docs/adr/0786): the parts the main view takes as kept are the
//! parts a fresh read gives, and the bands its painting takes as kept are the bands a fresh painting draws, to the
//! bit; the flat of the acceptance scenario, its schedules copied with it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use iced::Point;
use kentos_interaction::spatial::LabelParts;
use kentos_interaction::{Format, Vec2};
use kentos_native_style::color::ColorMode;
use serde_json::{Value, json};

use super::{BandMemo, Collect, Labels, Shown, cells_near, colors};
use crate::app::App;
use crate::viewport::Canvas;

fn ask(app: &mut App, params: Value) -> Value {
    app.agents_answer(&json!({ "id": 1, "method": "run", "params": params }))
        .unwrap_or_else(|(code, message)| panic!("{code}: {message}"))
}

/// Knuth's MMIX generator: the same draws everywhere.
struct Dice(u64);

impl Dice {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) % n.max(1)
    }
}

/// The view's labels as the drawing area asks them (true sizes, a table's cells near the view's box): the main
/// view's (`keep`, through its memo) or read afresh.
fn parts(app: &App, keep: bool) -> LabelParts {
    let v = app.viewport.camera.visible_bounds();
    let (w, h) = (v.max_x - v.min_x, v.max_y - v.min_y);
    let wide = kentos_geometry_core::geometry::Bounds {
        min_x: v.min_x - w,
        min_y: v.min_y - h,
        max_x: v.max_x + w,
        max_y: v.max_y + h,
    };
    app.spatial.label_parts(
        Vec2::new(wide.min_x, wide.min_y),
        Vec2::new(wide.max_x, wide.max_y),
        app.viewport.camera.scale,
        kentos_geometry_core::store::legible::LabelSize::Print,
        Default::default(),
        keep,
        None,
        Some(cells_near(&wide)),
    )
}

/// The bands `spots` paint as in the drawing area, through `memo` or afresh.
fn bands(
    app: &App,
    spots: Shown,
    memo: Option<Rc<RefCell<BandMemo>>>,
) -> Vec<(iced::Color, f32, Vec<[Point; 2]>)> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let camera = app.viewport.camera;
    let canvas = Canvas::Slate;
    let labels = Labels {
        doc,
        map_layer_colors: None,
        spots,
        view: camera,
        camera: Cell::new(camera),
        colors: colors(canvas, &crate::viewport::palette(canvas)).in_mode(ColorMode::Color),
        font: kentos_commands::DrawingFont::Barlow,
        format: Format::of(doc.settings()),
        key: 0,
        fence: None,
        current: Cell::new(None),
        faded: Rc::default(),
        map_marks: None,
        preview: None,
        pinned: false,
        turned: None,
        true_size: true,
        bands: RefCell::new(Vec::new()),
        margin: Cell::new(0.0),
        band_memo: memo,
        band_key: 7,
        bands_out: Some(()),
    };
    let mut ink = Collect {
        doc,
        camera,
        labels: Vec::new(),
    };
    labels.paint_texts(&mut ink, 120.0);
    labels.band_looks()
}

/// Random changes to four flats and their eight schedules (a wall moved, a room's name, a schedule's cell, a door
/// taken away, undone), the view now and then elsewhere: after each, the parts taken as kept are a fresh read's, the
/// bands taken as kept a fresh painting's; most parts and bands were taken as kept.
#[cfg(unix)]
#[test]
fn the_text_kept_after_random_changes_is_a_fresh_reads() {
    let (mut app, _dir) = crate::acceptance_flat_tests::opened("etiket-parcalari");
    let names = crate::acceptance_flat_tests::drawn(&mut app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let one: Vec<String> = doc
        .entities_with_uids()
        .map(|(u, _)| u.to_string())
        .collect();
    for c in 1..4 {
        let answer = ask(
            &mut app,
            json!({ "command": "cad.entities.transform", "op": "execute", "input": {
            "uids": one, "transform": { "kind": "move", "dx": 30.0 * f64::from(c), "dy": 0.0 }, "copy": true } }),
        );
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    app.follow_document();
    app.viewport.camera.set_size(1440.0, 900.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -3.0,
            min_y: -40.0,
            max_x: 120.0,
            max_y: 12.0,
        },
        24.0,
    );
    let memo: Rc<RefCell<BandMemo>> = Rc::default();
    let mut d = Dice(0x0786_2026);
    let (mut taken, mut kept_bands, mut changes) = (0, 0, 0);
    let walls: Vec<&str> = ["p1", "p2", "p3"]
        .into_iter()
        .filter(|n| names.contains_key(*n))
        .collect();
    for step in 0..40 {
        let wall = names[walls[d.below(walls.len() as u64) as usize]].clone();
        let done = match d.below(6) {
            0 | 1 => ask(
                &mut app,
                json!({ "command": "cad.walls.edit", "op": "execute", "input": {
                "uids": [wall], "operation": { "kind": "shift", "by": 0.01 * (1 + d.below(5)) as f64 } } }),
            ),
            2 => ask(
                &mut app,
                json!({ "command": "cad.entities.set", "op": "execute", "input": {
                "uids": [wall], "operation": "attributes", "attrs": { "Not": format!("{step}") } } }),
            ),
            3 => {
                app.undo();
                Value::Null
            }
            4 => {
                app.viewport
                    .camera
                    .pan_by(f64::from(d.below(300) as u32) - 150.0, 40.0);
                Value::Null
            }
            _ => {
                app.viewport.camera.zoom_at(1.25, 720.0, 450.0);
                Value::Null
            }
        };
        if !done.is_null() {
            assert_eq!(done["status"], "completed", "{done}");
            changes += 1;
        }
        app.follow_document();
        let kept = Rc::new(parts(&app, true));
        taken += app.spatial.labels_taken().0;
        let fresh = parts(&app, false);
        assert_eq!(kept.parts.len(), fresh.parts.len(), "step {step}");
        for (a, b) in kept.parts.iter().zip(&fresh.parts) {
            assert_eq!(a[..], b[..], "step {step}");
        }
        let with = bands(&app, kept.clone(), Some(memo.clone()));
        kept_bands += memo.borrow().taken().0;
        let without = bands(&app, kept, None);
        assert_eq!(with, without, "step {step}");
    }
    assert!(changes > 10, "{changes}");
    assert!(taken > 100, "{taken}");
    assert!(kept_bands > 50, "{kept_bands}");
}
