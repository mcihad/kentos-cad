use super::*;
use iced::Size;

const AREA: Rectangle = Rectangle {
    x: 100.0,
    y: 50.0,
    width: 800.0,
    height: 600.0,
};

fn over(x: f32, y: f32) -> mouse::Cursor {
    mouse::Cursor::Available(Point::new(x, y))
}

fn sized() -> Gesture {
    let mut state = Gesture::default();
    let first = gesture(
        &mut state,
        &iced::Event::Mouse(mouse::Event::CursorLeft),
        AREA,
        mouse::Cursor::Unavailable,
        Instant::now(),
    );
    assert_eq!(first, Some((Some(Event::Resized(AREA)), false)));
    state
}

#[test]
fn focus_loss_drops_every_button_and_never_synthesizes_a_release() {
    let mut state = sized();
    let now = Instant::now();
    let at = Point::new(300.0, 200.0);
    state.pan = Some(at);
    state.right = Some((now, at, false));
    state.left = true;
    state.last_middle = Some((now, at));
    let lost = gesture(
        &mut state,
        &iced::Event::Window(iced::window::Event::Unfocused),
        AREA,
        over(300.0, 200.0),
        now,
    );
    assert_eq!(lost, None);
    assert_eq!(right_hold(&mut state, now + RIGHT_HOLD * 2), None);
    for button in [
        mouse::Button::Left,
        mouse::Button::Right,
        mouse::Button::Middle,
    ] {
        assert_eq!(
            mouse_event(
                &mut state,
                mouse::Event::ButtonReleased(button),
                over(300.0, 200.0)
            ),
            None
        );
    }
    assert!(state.last_middle.is_none());
    let moved = mouse_event(
        &mut state,
        mouse::Event::CursorMoved { position: at },
        over(300.0, 200.0),
    );
    assert_eq!(
        moved,
        Some((Some(Event::Moved(Point::new(200.0, 150.0))), false))
    );
}

fn mouse_event(
    state: &mut Gesture,
    event: mouse::Event,
    cursor: mouse::Cursor,
) -> Option<(Option<Event>, bool)> {
    gesture(
        state,
        &iced::Event::Mouse(event),
        AREA,
        cursor,
        Instant::now(),
    )
}

#[test]
fn batched_pointer_events_press_where_the_preceding_move_happened() {
    let mut state = sized();
    // Iced can deliver several native events with the final cursor for
    // the whole batch. A press must still use its chronological position.
    let final_cursor = over(350.0, 170.0);
    let _ = mouse_event(
        &mut state,
        mouse::Event::CursorMoved {
            position: Point::new(150.0, 70.0),
        },
        final_cursor,
    );
    assert_eq!(
        mouse_event(
            &mut state,
            mouse::Event::ButtonPressed(mouse::Button::Left),
            final_cursor
        ),
        Some((Some(Event::Pressed(Point::new(50.0, 20.0))), true))
    );
}

#[test]
fn batched_middle_right_and_wheel_use_the_chronological_position() {
    let p = Point::new(150.0, 70.0);
    let end = over(350.0, 170.0);
    for button in [mouse::Button::Middle, mouse::Button::Right] {
        let mut state = sized();
        let _ = mouse_event(&mut state, mouse::Event::CursorMoved { position: p }, end);
        let press = mouse_event(&mut state, mouse::Event::ButtonPressed(button), end);
        match button {
            mouse::Button::Middle => {
                assert_eq!(press, Some((None, true)));
                assert_eq!(state.pan, Some(Point::new(50.0, 20.0)));
            }
            _ => {
                assert_eq!(
                    press,
                    Some((Some(Event::RightPressed(Point::new(50.0, 20.0))), true))
                );
                assert_eq!(state.right.unwrap().1, Point::new(50.0, 20.0));
            }
        }
        let _ = mouse_event(
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(350.0, 170.0),
            },
            end,
        );
        let release = mouse_event(&mut state, mouse::Event::ButtonReleased(button), end);
        if button == mouse::Button::Right {
            assert_eq!(
                release,
                Some((Some(Event::RightClick(Point::new(250.0, 120.0))), true))
            );
        }
    }
    let mut state = sized();
    let _ = mouse_event(&mut state, mouse::Event::CursorMoved { position: p }, end);
    let Some((Some(Event::Zoomed { at, .. }), true)) = mouse_event(
        &mut state,
        mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        },
        end,
    ) else {
        panic!("wheel")
    };
    assert_eq!(at, Point::new(50.0, 20.0));
}

#[test]
fn a_batch_ending_outside_keeps_the_press_inside_and_the_release_outside() {
    let mut state = sized();
    let end = over(1000.0, 20.0);
    assert_eq!(
        mouse_event(
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(150.0, 70.0)
            },
            end
        ),
        Some((Some(Event::Moved(Point::new(50.0, 20.0))), false))
    );
    assert_eq!(
        mouse_event(
            &mut state,
            mouse::Event::ButtonPressed(mouse::Button::Left),
            end
        ),
        Some((Some(Event::Pressed(Point::new(50.0, 20.0))), true))
    );
    let _ = mouse_event(
        &mut state,
        mouse::Event::CursorMoved {
            position: Point::new(1000.0, 20.0),
        },
        end,
    );
    assert_eq!(
        mouse_event(
            &mut state,
            mouse::Event::ButtonReleased(mouse::Button::Left),
            end
        ),
        Some((Some(Event::Released(Point::new(900.0, -30.0))), true))
    );
    assert!(!state.left);
}

#[test]
fn cached_event_positions_cannot_press_through_an_overlay_or_unavailable_cursor() {
    for cursor in [
        mouse::Cursor::Levitating(Point::new(350.0, 170.0)),
        mouse::Cursor::Unavailable,
    ] {
        let mut state = sized();
        let _ = mouse_event(
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(150.0, 70.0),
            },
            cursor,
        );
        for button in [
            mouse::Button::Left,
            mouse::Button::Middle,
            mouse::Button::Right,
        ] {
            assert_eq!(
                mouse_event(&mut state, mouse::Event::ButtonPressed(button), cursor),
                None
            );
        }
        assert_eq!(
            mouse_event(
                &mut state,
                mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 }
                },
                cursor
            ),
            None
        );
        assert!(!state.left && state.pan.is_none() && state.right.is_none());
    }
}

#[test]
fn a_fresh_cursor_or_leaving_the_window_supersedes_a_cached_event_position() {
    let mut state = sized();
    let _ = mouse_event(
        &mut state,
        mouse::Event::CursorMoved {
            position: Point::new(150.0, 70.0),
        },
        over(350.0, 170.0),
    );
    assert_eq!(
        mouse_event(
            &mut state,
            mouse::Event::ButtonPressed(mouse::Button::Left),
            over(400.0, 200.0)
        ),
        Some((Some(Event::Pressed(Point::new(300.0, 150.0))), true))
    );
    let _ = mouse_event(
        &mut state,
        mouse::Event::CursorLeft,
        mouse::Cursor::Unavailable,
    );
    assert!(state.pointer.is_none());
}

#[test]
fn the_area_reports_its_size_first_and_the_pointer_in_its_own_pixels() {
    let mut state = sized();
    let moved = mouse_event(
        &mut state,
        mouse::Event::CursorMoved {
            position: Point::new(150.0, 70.0),
        },
        over(150.0, 70.0),
    );
    assert_eq!(
        moved,
        Some((Some(Event::Moved(Point::new(50.0, 20.0))), false))
    );
    let left = mouse_event(
        &mut state,
        mouse::Event::CursorMoved {
            position: Point::new(10.0, 10.0),
        },
        over(10.0, 10.0),
    );
    assert_eq!(left, Some((Some(Event::Left), false)));
}

/// The left button's release reaches the app wherever it happens, once
/// it went down over the area (a selection box ends off the area too);
/// a release that began elsewhere is not the area's.
#[test]
fn the_left_release_follows_a_press_over_the_area() {
    let mut state = sized();
    let release = mouse::Event::ButtonReleased(mouse::Button::Left);
    assert_eq!(mouse_event(&mut state, release, over(150.0, 70.0)), None);
    let press = mouse_event(
        &mut state,
        mouse::Event::ButtonPressed(mouse::Button::Left),
        over(150.0, 70.0),
    );
    assert_eq!(
        press,
        Some((Some(Event::Pressed(Point::new(50.0, 20.0))), true))
    );
    assert_eq!(
        mouse_event(&mut state, release, over(1000.0, 20.0)),
        Some((Some(Event::Released(Point::new(900.0, -30.0))), true))
    );
    assert_eq!(mouse_event(&mut state, release, over(150.0, 70.0)), None);
}

#[test]
fn the_middle_button_drags_the_view_even_past_the_edge() {
    let mut state = sized();
    let press = mouse_event(
        &mut state,
        mouse::Event::ButtonPressed(mouse::Button::Middle),
        over(200.0, 200.0),
    );
    assert_eq!(press, Some((None, true)));
    let drag = mouse_event(
        &mut state,
        mouse::Event::CursorMoved {
            position: Point::new(1000.0, 180.0),
        },
        over(1000.0, 180.0),
    );
    assert_eq!(
        drag,
        Some((
            Some(Event::Panned {
                by: Vector::new(800.0, -20.0),
                at: Point::new(900.0, 130.0)
            }),
            true
        ))
    );
    let release = mouse_event(
        &mut state,
        mouse::Event::ButtonReleased(mouse::Button::Middle),
        over(1000.0, 180.0),
    );
    assert_eq!(release, Some((None, true)));
    assert!(state.pan.is_none());
}

/// The command strip over the area (command_bar.rs) levitates the
/// pointer: a press that began over the area still moves and ends there,
/// as the web's captured pointer does; with no press, the pointer has
/// left the area.
#[test]
fn a_press_goes_on_and_ends_over_the_strip_above_the_area() {
    let mut state = sized();
    let press = mouse_event(
        &mut state,
        mouse::Event::ButtonPressed(mouse::Button::Left),
        over(500.0, 300.0),
    );
    assert_eq!(
        press,
        Some((Some(Event::Pressed(Point::new(400.0, 250.0))), true))
    );
    let strip = mouse::Cursor::Levitating(Point::new(500.0, 70.0));
    let moved = mouse_event(
        &mut state,
        mouse::Event::CursorMoved {
            position: Point::new(500.0, 70.0),
        },
        strip,
    );
    assert_eq!(
        moved,
        Some((Some(Event::Moved(Point::new(400.0, 20.0))), false))
    );
    let released = mouse_event(
        &mut state,
        mouse::Event::ButtonReleased(mouse::Button::Left),
        strip,
    );
    assert_eq!(
        released,
        Some((Some(Event::Released(Point::new(400.0, 20.0))), true))
    );
    // A quick right press in the area let go over the strip is still Enter.
    let _ = mouse_event(
        &mut state,
        mouse::Event::ButtonPressed(mouse::Button::Right),
        over(500.0, 300.0),
    );
    let enter = mouse_event(
        &mut state,
        mouse::Event::ButtonReleased(mouse::Button::Right),
        strip,
    );
    assert_eq!(
        enter,
        Some((Some(Event::RightClick(Point::new(400.0, 20.0))), true))
    );
    // With no button down, over the strip is off the area.
    let left = mouse_event(
        &mut state,
        mouse::Event::CursorMoved {
            position: Point::new(500.0, 72.0),
        },
        strip,
    );
    assert_eq!(left, Some((Some(Event::Left), false)));
}

#[test]
fn a_held_right_button_is_told_once_and_its_release_is_no_click() {
    let mut state = sized();
    let now = Instant::now();
    let at = Point::new(300.0, 200.0);
    state.right = Some((now, at, false));
    // Before its time: a frame then.
    assert_eq!(
        right_hold(&mut state, now),
        Some(Hold::Wait(now + RIGHT_HOLD))
    );
    // Its time up: told once.
    assert_eq!(
        right_hold(&mut state, now + RIGHT_HOLD),
        Some(Hold::Held(at))
    );
    assert_eq!(right_hold(&mut state, now + RIGHT_HOLD * 2), None);
    let released = gesture(
        &mut state,
        &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
        AREA,
        mouse::Cursor::Available(Point::new(300.0, 200.0)),
        now + RIGHT_HOLD * 2,
    );
    assert_eq!(released, Some((None, true)), "no Enter after a menu");
    // Held long enough with no frame to tell it: the release opens the menu.
    state.right = Some((now, at, false));
    let released = gesture(
        &mut state,
        &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
        AREA,
        mouse::Cursor::Available(Point::new(310.0, 205.0)),
        now + RIGHT_HOLD * 2,
    );
    assert_eq!(released, Some((Some(Event::RightHeld(at)), true)));
}

#[test]
fn a_middle_double_click_shows_everything() {
    let mut state = sized();
    let now = Instant::now();
    let press = |state: &mut Gesture, at: Instant| {
        gesture(
            state,
            &iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)),
            AREA,
            over(300.0, 300.0),
            at,
        )
    };
    assert_eq!(press(&mut state, now), Some((None, true)));
    let _ = gesture(
        &mut state,
        &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)),
        AREA,
        over(300.0, 300.0),
        now,
    );
    assert_eq!(
        press(&mut state, now + Duration::from_millis(200)),
        Some((Some(Event::Extents), true))
    );
    // Slow presses are two drags.
    assert_eq!(
        press(&mut state, now + Duration::from_secs(3)),
        Some((None, true))
    );
}

#[test]
fn the_wheel_zooms_at_the_pointer_and_only_over_the_area() {
    let mut state = sized();
    let notch = mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
    };
    let Some((Some(Event::Zoomed { factor, at }), true)) =
        mouse_event(&mut state, notch, over(500.0, 350.0))
    else {
        panic!("a notch zooms");
    };
    assert!((factor - 0.15f64.exp()).abs() < 1e-12);
    assert_eq!(at, Point::new(400.0, 300.0));
    assert_eq!(mouse_event(&mut state, notch, over(20.0, 20.0)), None);
    let back = mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -100.0 },
    };
    let Some((Some(Event::Zoomed { factor, .. }), true)) =
        mouse_event(&mut state, back, over(500.0, 350.0))
    else {
        panic!("a touchpad zooms");
    };
    assert!((factor - (-0.15f64).exp()).abs() < 1e-12);
}

fn sample() -> Document {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/document/v1/sample.json");
    Document::read(&path).expect("the sample reads")
}

#[test]
fn opening_fits_the_start_view_once_the_area_has_a_size() {
    let doc = sample();
    let mut viewport = Viewport::new();
    viewport.opened(&doc, None);
    assert_eq!(viewport.camera, Camera::default(), "no size yet: no fit");
    viewport.update(
        Event::Resized(Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0))),
        Some(&doc),
    );
    // The sample's start view: 70 × 80 m around the parcel.
    let home = start_view(&doc).expect("the sample has a start view");
    let visible = viewport.camera.visible_bounds();
    assert!(visible.min_x <= home.min_x && visible.max_x >= home.max_x);
    assert!(visible.min_y <= home.min_y && visible.max_y >= home.max_y);
    assert!((viewport.camera.center.x - (home.min_x + home.max_x) / 2.0).abs() < 1e-9);

    viewport.update(Event::Extents, Some(&doc));
    let extents = scene::extents(&doc).expect("extents");
    assert!((viewport.camera.center.y - (extents.min_y + extents.max_y) / 2.0).abs() < 1e-9);

    // The pointer's world point follows pan and zoom.
    viewport.update(Event::Moved(Point::new(500.0, 400.0)), Some(&doc));
    let middle = viewport.cursor.expect("over the area");
    assert!((middle.x - viewport.camera.center.x).abs() < 1e-9);
    viewport.update(
        Event::Zoomed {
            factor: 2.0,
            at: Point::new(500.0, 400.0),
        },
        Some(&doc),
    );
    let after = viewport.cursor.expect("still over the area");
    assert!((after.x - middle.x).abs() < 1e-9 && (after.y - middle.y).abs() < 1e-9);
    viewport.update(Event::Left, Some(&doc));
    assert_eq!(viewport.cursor, None);
}

#[test]
fn the_scene_is_built_again_only_when_its_inputs_change() {
    use crate::app::App;
    // Through the app's own messages, as the window drives it.
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(sample())))));
    let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(
        Point::ORIGIN,
        Size::new(1000.0, 800.0),
    ))));
    let palette = palette(Mode::Dark);
    let settings = RenderSettings::new(palette.background);
    let scene = |app: &App, mode: Mode, palette: &Palette| {
        let doc = app.document.as_ref().expect("the sample is open");
        let (_, fixed, curves, _, _) = app.viewport.scene(doc, mode, palette, &settings);
        (fixed, curves)
    };
    let (fixed, curves) = scene(&app, Mode::Dark, &palette);
    // A pan leaves the scene as it is.
    let _ = app.update(Message::Viewport(Event::Panned {
        by: Vector::new(30.0, 10.0),
        at: Point::new(1.0, 1.0),
    }));
    let (same_fixed, same_curves) = scene(&app, Mode::Dark, &palette);
    assert_eq!((fixed.id, curves.id), (same_fixed.id, same_curves.id));
    // Zooming in 8× leaves the curves' band: only the curves are built again.
    let _ = app.update(Message::Viewport(Event::Zoomed {
        factor: 8.0,
        at: Point::new(500.0, 400.0),
    }));
    let (zoomed_fixed, zoomed_curves) = scene(&app, Mode::Dark, &palette);
    assert_eq!(zoomed_fixed.id, fixed.id);
    assert_ne!(zoomed_curves.id, curves.id);
    assert!(zoomed_curves.segments.len() > curves.segments.len());
    // A change to the drawing (the Kadastro group hidden) builds both.
    let _ = app.update(Message::LayerVisible("layer-g".into()));
    let (changed, _) = scene(&app, Mode::Dark, &palette);
    assert_ne!(changed.id, fixed.id);
    assert!(
        changed.segments.is_empty(),
        "every shown layer is in the hidden group"
    );
    // So does the theme: token colours resolve anew.
    let light = super::palette(Mode::Light);
    let (relit, _) = scene(&app, Mode::Light, &light);
    assert_ne!(relit.id, changed.id);
}

/// The wheel turned over the area through the real widgets (docs/adr/9205): the area's live camera moves at once, the
/// app's waits, nothing of the app runs; once the view has been still for the settle, the app takes the camera the
/// gesture stopped at, in one message, and keeps the view it left in its history.
#[test]
fn a_wheel_turn_moves_the_live_camera_and_reaches_the_app_once_settled() {
    use crate::app::App;
    use kentos_ui::snapshot::{Input, Snapshot};
    let mut app = crate::files_testing::app_with_drawing();
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let before = app.viewport.camera;
    assert_eq!(
        app.viewport.live.camera(),
        before,
        "the area shows the app's camera"
    );
    let at = app.viewport.bounds.center();
    snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 1.0));
    snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 1.0));
    assert_eq!(
        app.viewport.camera, before,
        "the app's camera waits for the view to settle"
    );
    let live = app.viewport.live.camera();
    assert!(
        live.scale > before.scale,
        "the area's live camera zoomed in: {} → {}",
        before.scale,
        live.scale
    );
    assert!(!app.view_history.can_back(), "nothing recorded yet");
    std::thread::sleep(live::SETTLE + Duration::from_millis(40));
    snapshot.settle(&mut app, App::view, &mut update);
    assert_eq!(
        app.viewport.camera, live,
        "the app took the camera the gesture stopped at"
    );
    assert!(!app.viewport.live.moving(), "told once");
    assert!(app.view_history.can_back(), "the view left is kept");
}

/// A pan with the middle button drags the pointer with the drawing and tells the app nothing on the way; once it
/// settles, the pointer is where the drag left it and its world point is the one the drag held under it, so a running
/// tool's preview still ends at the pointer (docs/adr/9251). A drag let go past the area's edge leaves no point.
#[test]
fn a_settled_pan_puts_the_pointer_where_the_drag_left_it() {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Viewport(Event::Resized(AREA)));
    let _ = app.view();
    let start = Point::new(300.0, 200.0);
    let _ = app.update(Message::Viewport(Event::Moved(start)));
    let held = app.viewport.cursor.expect("a point");
    let end = Point::new(340.0, 175.0);
    app.viewport
        .live
        .pan(end - start, end, iced::time::Instant::now());
    let (from, to, at) = app.viewport.take_live().expect("a gesture");
    assert_eq!(at, Some(end));
    let _ = app.update(Message::Viewport(Event::ViewChanged { from, to, at }));
    let cursor = app.viewport.cursor.expect("a point");
    assert!(
        (cursor.x - held.x).abs() < 1e-9 && (cursor.y - held.y).abs() < 1e-9,
        "{cursor:?} against {held:?}"
    );
    let _ = app.view();
    let past = Point::new(-30.0, 175.0);
    app.viewport
        .live
        .pan(past - end, past, iced::time::Instant::now());
    let (from, to, at) = app.viewport.take_live().expect("a gesture");
    let _ = app.update(Message::Viewport(Event::ViewChanged { from, to, at }));
    assert!(app.viewport.cursor.is_none(), "off the area");
}

/// Yakınlaştır and Uzaklaştır zoom about the middle: the pointer stays where it is on the screen and its world point is
/// the one under it now, not the middle's (a running tool's preview follows it); off the area there is none
/// (docs/adr/9251).
#[test]
fn a_zoom_about_the_middle_leaves_the_pointer_where_it_is_on_the_screen() {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Viewport(Event::Resized(AREA)));
    let _ = app.view();
    let _ = app.update(Message::Viewport(Event::Moved(Point::new(120.0, 80.0))));
    app.zoom_in();
    let cursor = app.viewport.cursor.expect("a point");
    let under = app.viewport.camera.screen_to_world(120.0, 80.0);
    assert!(
        (cursor.x - under.x).abs() < 1e-9 && (cursor.y - under.y).abs() < 1e-9,
        "{cursor:?} against {under:?}"
    );
    let _ = app.update(Message::Viewport(Event::Left));
    app.zoom_out();
    assert!(app.viewport.cursor.is_none(), "off the area");
}

/// While a gesture has not yet settled, a move reads the pointer through the camera the screen shows without taking
/// the gesture (the app would build the view's labels and curves again on every move); a press takes it first
/// (docs/adr/9205).
#[test]
fn a_move_reads_the_live_camera_and_a_press_takes_it() {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Viewport(Event::Resized(AREA)));
    let _ = app.view();
    let before = app.viewport.camera;
    assert_eq!(app.viewport.live.camera(), before);
    app.viewport
        .live
        .zoom(1.5, Point::new(400.0, 300.0), iced::time::Instant::now());
    let live = app.viewport.live.camera();
    assert_ne!(live, before);
    let near = |a: kentos_render_wgpu::Vec2, b: kentos_render_wgpu::Vec2| {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    };
    let _ = app.update(Message::Viewport(Event::Moved(Point::new(120.0, 80.0))));
    assert_eq!(
        app.viewport.camera, before,
        "a move does not take the gesture"
    );
    assert!(
        near(
            app.viewport.cursor.expect("a point"),
            live.screen_to_world(120.0, 80.0)
        ),
        "read through the live camera"
    );
    let _ = app.update(Message::Viewport(Event::Pressed(Point::new(130.0, 90.0))));
    assert_eq!(app.viewport.camera, live, "a press takes the gesture first");
    assert!(!app.viewport.live.moving());
    assert!(near(
        app.viewport.cursor.expect("a point"),
        live.screen_to_world(130.0, 90.0)
    ));
}
