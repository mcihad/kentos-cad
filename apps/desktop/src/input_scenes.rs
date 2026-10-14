//! The pictures of docs/adr/0266 (lengths with their own unit and sums in brackets): a millimetre project, Çizgi
//! with `@2.5m<0`, `@(1.2+0.3)m<90` and `@-50cm,(100*2)` typed, the cursor's value field holding `(1000/4)` while
//! the next segment waits; and docs/adr/0268's: Shift held over a polyline, ortho ruling for the moment and polar
//! tracking (on) shown out of use in the status bar; docs/adr/0269's: the one-shot Merkez through an arc's curve,
//! and Ekleme noktası on a text; docs/adr/0271's: Tab on the second candidate where two lines meet a parcel;
//! docs/adr/0272's: Koordinat oku on a parcel's corner, the log saying the snap's object and place; docs/adr/0274's:
//! inside a room block turned 30°, the midpoint of a nested, mirrored door's swing; docs/adr/0276's: a Ctrl click
//! on a parcel's corner where a wall crosses it (the corner ringed, the log saying the parcel's id and corner, the
//! chip for the wall), and the shortcuts window with Ctrl + tık and the drawing keys; docs/adr/0279's: the grip menu of a parcel's corner; docs/adr/0280's: two hot corners turning the parcel; docs/adr/0281's: an ambiguous start and a start of one command typed; docs/adr/0283's: Pencere yakınlaştır run over a polyline being drawn, and the polyline back after it with its preview in the new view; docs/adr/0286's: Öznitelikler of a mixed selection with a locked wall and shared attributes. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=giris-birim,giris-oncelik,kenet-merkez,kenet-ekleme,kenet-dolasma,kenet-oku,kenet-blok-ici,secim-alt-nesne,kisayollar,tutamac-menusu,tutamac-kipleri,komut-adlari,seffaf-pencere,seffaf-donus,oznitelik-coklu,oznitelik-ortak,dikdortgen-komut cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::DrawingUnit;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, open_with_blocks, run, typed};
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

/// Koordinat oku clicked on the parcel's third corner: the log says “Kenet: Uç nokta · Kapalı alan … · 3. köşe”.
fn reading(app: &mut App) {
    let mut o = Objects::new();
    o.path(
        "cizim",
        &[[0.0, 0.0], [12.0, 0.0], [12.0, 9.0], [0.0, 9.0]],
        true,
    );
    open(app, o);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 6.0,
            min_y: N - 4.0,
            max_x: E + 18.0,
            max_y: N + 12.0,
        },
        24.0,
    );
    run(app, "crs.query");
    click(app, [12.1, 9.05]);
    hover(app, [12.1, 9.05]);
}

/// A door (a leaf and its swing) inside a room (walls, a column), the door twice: as it is and turned a quarter,
/// mirrored. The room put turned 30° and one and a half times as large, and once more mirrored.
fn room_blocks() -> serde_json::Value {
    let piece = |id: u32, more: serde_json::Value| {
        let mut v = serde_json::json!({ "id": id, "layerId": "cizim", "attrs": {} });
        v.as_object_mut()
            .expect("an object")
            .extend(more.as_object().expect("an object").clone());
        v
    };
    let p = |x: f64, y: f64| serde_json::json!({ "x": x, "y": y });
    serde_json::json!([
        { "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4201", "name": "Kapı", "base": p(0.0, 0.0), "entities": [
            piece(1, serde_json::json!({ "kind": "line", "a": p(0.0, 0.0), "b": p(0.0, 1.0) })),
            piece(2, serde_json::json!({ "kind": "arc", "c": p(0.0, 0.0), "r": 1.0, "a0": 0.0,
                                         "a1": std::f64::consts::FRAC_PI_2 })),
        ]},
        { "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4202", "name": "Oda", "base": p(0.0, 0.0), "entities": [
            piece(1, serde_json::json!({ "kind": "polygon",
                                         "pts": [p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)] })),
            piece(2, serde_json::json!({ "kind": "circle", "c": p(4.5, 2.0), "r": 0.5 })),
            piece(3, serde_json::json!({ "kind": "insert", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4201",
                                         "p": p(1.5, 0.0), "scale": 1.0, "rotation": 0.0 })),
            piece(4, serde_json::json!({ "kind": "insert", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4201",
                                         "p": p(6.0, 1.0), "scale": 1.0,
                                         "rotation": std::f64::consts::FRAC_PI_2, "mirror": true })),
        ]},
    ])
}

/// Where a room point is in the first room (turned 30°, 1.5 times as large, at the origin).
fn in_room(q: [f64; 2]) -> [f64; 2] {
    let (s, c) = 30f64.to_radians().sin_cos();
    [1.5 * (c * q[0] - s * q[1]), 1.5 * (s * q[0] + c * q[1])]
}

/// Çizgi from outside the rooms, the pointer near the midpoint of the turned room's mirrored door's swing.
fn in_blocks(app: &mut App) {
    let mut o = Objects::new();
    o.insert(
        "cizim",
        "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4202",
        [0.0, 0.0],
        1.5,
        30f64.to_radians(),
        false,
    );
    o.insert(
        "cizim",
        "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4202",
        [16.0, 0.0],
        1.5,
        0.0,
        true,
    );
    open_with_blocks(app, o, room_blocks());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 6.0,
            min_y: N - 8.0,
            max_x: E + 26.0,
            max_y: N + 10.0,
        },
        24.0,
    );
    run(app, "tool.line");
    click(app, [12.0, 8.0]);
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let m = in_room([6.0 + h, 1.0 + h]);
    hover(app, [m[0] + 0.08, m[1] + 0.05]);
}

/// A parcel and a wall line through its corner's neighbourhood; the select tool, Ctrl held, a click near the
/// corner: the parcel's corner is the sub-object.
fn sub_object(app: &mut App) {
    sub_ground(app);
    ctrl_click(app);
}

/// The parcel and the wall, in view.
fn sub_ground(app: &mut App) {
    let mut o = Objects::new();
    o.path(
        "cizim",
        &[[0.0, 0.0], [12.0, 0.0], [12.0, 9.0], [0.0, 9.0]],
        true,
    );
    o.line("cizim", [8.0, 9.1], [16.0, 9.1]);
    open(app, o);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 6.0,
            min_y: N - 4.0,
            max_x: E + 22.0,
            max_y: N + 14.0,
        },
        24.0,
    );
}

/// Ctrl held, a click near the parcel's corner.
fn ctrl_click(app: &mut App) {
    let _ = app.update(crate::app::Message::Modifiers(
        iced::keyboard::Modifiers::COMMAND,
    ));
    click(app, [11.95, 9.04]);
    let _ = app.update(crate::app::Message::Modifiers(
        iced::keyboard::Modifiers::empty(),
    ));
}

/// docs/adr/0279: the parcel selected, a right click on its corner's grip: Köşe 3 with Köşeyi sil, then Esnet,
/// Taşı, Döndür, Ölçekle and Aynala.
fn grip_menu(app: &mut App) {
    sub_ground(app);
    click(app, [6.0, 0.0]);
    let corner = crate::tools_scenes::at(app, [12.0, 9.0]);
    let _ = app.update(crate::app::Message::Viewport(
        crate::viewport::Event::Moved(corner),
    ));
    let _ = app.update(crate::app::Message::Viewport(
        crate::viewport::Event::RightPressed(corner),
    ));
    let _ = app.update(crate::app::Message::Viewport(
        crate::viewport::Event::RightClick(corner),
    ));
}

/// docs/adr/0280: the parcel selected, two of its corners made hot with Shift, the edit started on one, Döndür (D)
/// chosen and the pointer up to the north-west: the selection's ghost turned about the grip, its angle beside it.
fn grip_modes(app: &mut App) {
    sub_ground(app);
    click(app, [6.0, 0.0]);
    grip_mode_keys(app);
    hover(app, [4.0, 14.0]);
}

/// Shift and a click on two corners, a click on the first, then D.
fn grip_mode_keys(app: &mut App) {
    use crate::app::Message;
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::SHIFT));
    click(app, [12.0, 9.0]);
    click(app, [12.0, 0.0]);
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::empty()));
    click(app, [12.0, 9.0]);
    let key = iced::keyboard::Key::Character("d".into());
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key,
        physical: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        modifiers: iced::keyboard::Modifiers::default(),
        text: Some("d".into()),
        repeat: false,
    }));
}

/// docs/adr/0281: names typed in the command line: “ci” several commands start with (said with them), “orta ha”
/// the start of Orta hat only (run, said), and in it “Adım” by its name.
fn typed_names(app: &mut App) {
    sub_ground(app);
    let _ = app.run_typed("ci");
    let _ = app.run_typed("orta ha");
}

/// docs/adr/0286: three walls and a parcel selected, one wall on the locked Yol layer: Öznitelikler says the
/// locked one is skipped, Ortak öznitelikler shows Tür shared and Kat Çeşitli.
fn many_selected(app: &mut App) {
    let mut o = Objects::new();
    let a = o.line("cizim", [0.0, 0.0], [12.0, 0.0]);
    let b = o.line("cizim", [0.0, 3.0], [12.0, 3.0]);
    let c = o.line("yol", [0.0, 6.0], [12.0, 6.0]);
    let d = o.path(
        "parsel",
        &[[0.0, 9.0], [12.0, 9.0], [12.0, 18.0], [0.0, 18.0]],
        true,
    );
    o.data(a, &[("Tür", "Duvar"), ("Kat", "1")], None);
    o.data(b, &[("Tür", "Duvar"), ("Kat", "2")], None);
    o.data(c, &[("Tür", "Duvar")], None);
    o.data(d, &[("Tür", "Duvar"), ("Kat", "1")], None);
    open(app, o);
    make_cad(app);
    if let Some(doc) = app.document.as_mut() {
        doc.model.toggle_layer_locked("yol");
    }
    app.selection.set(
        [a, b, c, d]
            .into_iter()
            .map(kentos_domain::Slot)
            .collect::<Vec<_>>(),
    );
}

/// The same with Ortak özellikler closed: Ortak öznitelikler in view.
fn many_attributes(app: &mut App) {
    many_selected(app);
    app.props_closed.insert("general");
}

/// docs/adr/0290: Dikdörtgen with 1.5 m rounded corners drawn through `cad.rectangle.create`; Komut geçmişi shows
/// the call with its Python open.
fn rectangle_command(app: &mut App) {
    open(app, Objects::new());
    make_cad(app);
    app.command_expanded = true;
    app.bottom_log = Some(300.0);
    app.history = Default::default();
    run(app, "tool.rectangle");
    typed(app, "Y");
    typed(app, "1.5");
    click(app, [0.0, 0.0]);
    click(app, [20.0, 12.5]);
    let _ = app.update(crate::app::Message::Run("tool.select"));
    if let Some(entry) = app.history.entries.last() {
        let id = entry.id;
        let _ = app.update(crate::app::Message::History(crate::history::Event::Toggle(id)));
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 6.0,
            min_y: N - 4.0,
            max_x: E + 26.0,
            max_y: N + 16.0,
        },
        24.0,
    );
}

/// The shortcuts window, opened from Yardım.
fn shortcut_list(app: &mut App) {
    open(app, Objects::new());
    make_cad(app);
    let _ = app.run("help.shortcuts");
}

/// Two parcels; Çoklu çizgi along their south with three corners; Pencere yakınlaştır run over it (docs/adr/0283),
/// its first corner given and the box dragged to the east parcel.
fn transparent_window(app: &mut App) {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[0.0, 0.0], [20.0, 0.0], [20.0, 14.0], [0.0, 14.0]],
        true,
    );
    o.path(
        "parsel",
        &[[20.0, 0.0], [38.0, 0.0], [38.0, 14.0], [20.0, 14.0]],
        true,
    );
    open(app, o);
    make_cad(app);
    let _ = app
        .settings
        .choose(&[("drafting.snap", serde_json::Value::Bool(false))]);
    app.apply_settings();
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 6.0,
            min_y: N - 10.0,
            max_x: E + 44.0,
            max_y: N + 20.0,
        },
        24.0,
    );
    run(app, "tool.polyline");
    click(app, [0.0, -4.0]);
    click(app, [20.0, -4.0]);
    click(app, [24.0, -2.0]);
    run(app, "tool.zoomWindow");
    click(app, [16.0, -7.0]);
    hover(app, [40.0, 16.0]);
}

/// The window's second corner: the view shows the box, Çoklu çizgi is back with its three corners, its preview to
/// the pointer where the corner was clicked.
fn transparent_back(app: &mut App) {
    transparent_window(app);
    click(app, [40.0, 16.0]);
    hover(app, [32.0, 6.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("seffaf-pencere", transparent_window),
        ("seffaf-donus", transparent_back),
        ("secim-alt-nesne", sub_object),
        ("kisayollar", shortcut_list),
        ("tutamac-menusu", grip_menu),
        ("tutamac-kipleri", grip_modes),
        ("komut-adlari", typed_names),
        ("oznitelik-coklu", many_selected),
        ("oznitelik-ortak", many_attributes),
        ("dikdortgen-komut", rectangle_command),
        ("kenet-blok-ici", in_blocks),
        ("kenet-oku", reading),
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

    use super::{
        centre_through, ctrl_click, grip_mode_keys, in_blocks, in_room, insertion, sub_ground,
    };
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

    /// docs/adr/0274 on the desktop: inside the turned room the nested, mirrored door's swing gives its midpoint,
    /// the room's insert its object.
    #[test]
    fn a_nested_mirrored_door_s_swing_snaps_inside_the_turned_room() {
        let mut app = crate::files_testing::app_with_drawing();
        in_blocks(&mut app);
        let hit = app.snap.expect("a snap");
        assert_eq!(hit.kind, SnapKind::Midpoint);
        let h = std::f64::consts::FRAC_1_SQRT_2;
        let m = in_room([6.0 + h, 1.0 + h]);
        assert!(
            (hit.point.x - (E + m[0])).abs() < 1e-9 && (hit.point.y - (N + m[1])).abs() < 1e-9,
            "{hit:?}"
        );
    }

    /// docs/adr/0276 on the desktop: a Ctrl click near the parcel's corner selects the parcel with its corner as the
    /// sub-object, the wall line the chip's second candidate; Sıradakini seç moves to the wall with its own place.
    #[test]
    fn a_ctrl_click_selects_the_corner_and_cycling_keeps_each_sub_object() {
        let mut app = crate::files_testing::app_with_drawing();
        sub_ground(&mut app);
        // A view 1100 × 600 px at 40 px a metre: the pick tolerance is a fraction of a metre (no window here).
        let c = &mut app.viewport.camera;
        (c.width, c.height, c.scale) = (1100.0, 600.0, 40.0);
        c.center = kentos_interaction::Vec2::new(E + 8.0, N + 5.0);
        ctrl_click(&mut app);
        let (slot, sub) = app.selection.sub().expect("a sub-object");
        assert_eq!(app.selection.ids(), [slot]);
        assert_eq!(
            sub.place,
            kentos_contracts::SubPlace::Vertex {
                part: 0,
                ring: 0,
                index: 2
            }
        );
        let p = app.sub_object_point().expect("its point");
        assert!((p.x - (E + 12.0)).abs() < 1e-9 && (p.y - (N + 9.0)).abs() < 1e-9);
        assert!(
            app.log
                .lines()
                .any(|l| l.text.starts_with("Alt nesne: Kapalı alan")),
            "{:?}",
            app.log.lines().map(|l| &l.text).collect::<Vec<_>>()
        );
        assert_eq!(app.selection.cycle().map(|c| c.candidates.len()), Some(2));
        app.cycle_selection();
        let (wall, sub) = app.selection.sub().expect("the wall's sub-object");
        assert_ne!(wall, slot);
        assert!(matches!(
            sub.place,
            kentos_contracts::SubPlace::Along { .. }
        ));
    }

    /// docs/adr/0280 on the desktop: Shift heats two corners, a click starts the edit, the letter D is the grip's
    /// Döndür (not a shortcut), a typed angle (100g, the project's unit) turns the parcel a quarter about the grip in
    /// one step.
    #[test]
    fn two_hot_corners_take_the_rotate_letter_and_a_typed_angle() {
        use kentos_domain::Slot;
        let mut app = crate::files_testing::app_with_drawing();
        sub_ground(&mut app);
        crate::tools_scenes::click(&mut app, [6.0, 0.0]);
        grip_mode_keys(&mut app);
        assert_eq!(app.session.hot_grips().len(), 2);
        assert!(app.session.prompt().text().starts_with("Tutamaç · Döndür"));
        let _ = app.update(crate::app::Message::PromptOption("K"));
        let _ = app.update(crate::app::Message::PromptOption("K"));
        // A CAD project whose angles are grads: a typed turn is counter-clockwise in grads (docs/adr/0291, 0294).
        app.with_tool(|s, cx| s.input("100", cx));
        let doc = app.document.as_ref().expect("open");
        let slot = app.selection.ids().first().copied().unwrap_or(Slot(0));
        let Some(kentos_contracts::Entity::Polygon(p)) = doc.model.get(slot) else {
            panic!("the parcel");
        };
        // A quarter turn about the corner (12, 9): (0, 0) goes to (21, −3).
        let first = [p.pts[0].x - E, p.pts[0].y - N];
        assert!(
            (first[0] - 21.0).abs() < 1e-9 && (first[1] + 3.0).abs() < 1e-9,
            "{first:?}"
        );
        assert!(!app.session.grip_active());
    }
}
