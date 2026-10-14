//! The pictures of docs/adr/0266 (lengths with their own unit and sums in brackets): a millimetre project, Çizgi
//! with `@2.5m<0`, `@(1.2+0.3)m<90` and `@-50cm,(100*2)` typed, the cursor's value field holding `(1000/4)` while
//! the next segment waits; and docs/adr/0268's: Shift held over a polyline, ortho ruling for the moment and polar
//! tracking (on) shown out of use in the status bar; docs/adr/0269's: the one-shot Merkez through an arc's curve,
//! and Ekleme noktası on a text; docs/adr/0271's: Tab on the second candidate where two lines meet a parcel. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=giris-birim,giris-oncelik,kenet-merkez,kenet-ekleme,kenet-dolasma cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::DrawingUnit;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// A millimetre drawing; Çizgi from the origin with three typed segments; the cursor up to the north-west.
fn units(app: &mut App) {
    open(app, Objects::new());
    make_cad(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.srid = 0;
    s.second_srid = None;
    s.drawing_unit = Some(DrawingUnit::Mm);
    s.angle_unit = kentos_contracts::AngleUnit::Deg;
    s.plot_scale = 20.0;
    doc.model.set_settings(s);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.8,
            min_y: N - 0.6,
            max_x: E + 3.4,
            max_y: N + 2.4,
        },
        24.0,
    );
    run(app, "tool.line");
    click(app, [0.0, 0.0]);
    for text in ["@2.5m<0", "@(1.2+0.3)m<90", "@-50cm,(100*2)"] {
        typed(app, text);
    }
    hover(app, [1.0, 2.2]);
}

/// Polar tracking on, ortho off; Shift held: Orto lit for the moment, Kutupsal out of use, the preview square.
fn priority(app: &mut App) {
    open(app, Objects::new());
    make_cad(app);
    let _ = app.settings.choose(&[
        ("drafting.polar", serde_json::Value::Bool(true)),
        ("drafting.snap", serde_json::Value::Bool(false)),
    ]);
    app.apply_settings();
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 4.0,
            max_x: E + 30.0,
            max_y: N + 18.0,
        },
        24.0,
    );
    run(app, "tool.polyline");
    click(app, [0.0, 0.0]);
    click(app, [12.0, 0.0]);
    hover(app, [22.0, 8.5]);
    let _ = app.update(crate::app::Message::Modifiers(
        iced::keyboard::Modifiers::SHIFT,
    ));
}

/// A short arc (its centre outside its box) and a text, Çizgi from the south-west.
fn snap_ground(app: &mut App) {
    let mut o = Objects::new();
    o.arc("cizim", [10.0, 4.0], 8.0, 0.35, 1.2);
    o.text("cizim", [24.0, 8.0], "Bina A", 1.5, 0.0);
    open(app, o);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 4.0,
            max_x: E + 34.0,
            max_y: N + 16.0,
        },
        24.0,
    );
    run(app, "tool.line");
    click(app, [-2.0, -2.0]);
}

/// The one-shot Merkez: the cursor on the arc, the snap at its centre.
fn centre_through(app: &mut App) {
    snap_ground(app);
    let _ = app.update(crate::app::Message::DrawingMenu(
        crate::drawing_menus::Event::SnapOnce(Some(kentos_interaction::SnapKind::Center)),
    ));
    let at = 0.8_f64;
    hover(app, [10.0 + 8.0 * at.cos(), 4.0 + 8.0 * at.sin()]);
}

/// Ekleme noktası: the cursor near the text's insertion point.
fn insertion(app: &mut App) {
    snap_ground(app);
    hover(app, [24.3, 8.2]);
}

/// A parcel, a wall line through its corner's neighbourhood; Çizgi from the north, the pointer near the corner,
/// Tab once: from the crossing to the corner, “Uç nokta · kapalı alan · 2/2 (Tab)”.
fn cycling(app: &mut App) {
    let mut o = Objects::new();
    o.path(
        "cizim",
        &[[0.0, 0.0], [12.0, 0.0], [12.0, 9.0], [0.0, 9.0]],
        true,
    );
    o.line("cizim", [-3.0, 0.5], [8.0, 0.5]);
    open(app, o);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 14.0,
            min_y: N - 8.0,
            max_x: E + 20.0,
            max_y: N + 14.0,
        },
        24.0,
    );
    run(app, "tool.line");
    click(app, [6.0, 6.0]);
    hover(app, [0.15, 0.3]);
    app.cycle_snap(false);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kenet-dolasma", cycling),
        ("giris-birim", units),
        ("giris-oncelik", priority),
        ("kenet-merkez", centre_through),
        ("kenet-ekleme", insertion),
    ]
}

#[cfg(test)]
mod tests {
    use kentos_interaction::SnapKind;

    use super::{centre_through, insertion};
    use crate::tools_scenes::{E, N};

    /// docs/adr/0269 §1–§2 on the desktop: the one-shot Merkez on an arc's curve snaps to its centre; a text's
    /// point is Ekleme noktası.
    #[test]
    fn the_one_shot_centre_comes_through_the_arc_and_a_text_gives_its_insertion_point() {
        let mut app = crate::files_testing::app_with_drawing();
        centre_through(&mut app);
        let hit = app.snap.expect("a snap");
        assert_eq!(hit.kind, SnapKind::Center);
        assert!((hit.point.x - (E + 10.0)).abs() < 1e-9 && (hit.point.y - (N + 4.0)).abs() < 1e-9);
        let mut app = crate::files_testing::app_with_drawing();
        insertion(&mut app);
        let hit = app.snap.expect("a snap");
        assert_eq!(hit.kind, SnapKind::Insertion);
        assert!((hit.point.x - (E + 24.0)).abs() < 1e-9 && (hit.point.y - (N + 8.0)).abs() < 1e-9);
    }
}
