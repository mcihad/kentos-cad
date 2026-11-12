//! Real widget regression for nested style-list/body scrolling and fixed actions.
use std::sync::{Arc, Mutex};

use iced::advanced::widget::{
    Id, Operation,
    operation::{Scrollable, TextInput},
};
use iced::{Element, Point, Rectangle, Size, Vector};
use kentos_ui::snapshot::{Input, Snapshot};
use kentos_ui::theme::typography::{self, Typography};

use super::{CANCEL, Event, SAVE};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, make_cad, offscreen};

struct RestoreTypography;
impl Drop for RestoreTypography {
    fn drop(&mut self) {
        typography::set(Typography::DEFAULT);
    }
}

fn open(size: Size, text_size: u32, long: bool) -> App {
    let mut app = app_with_drawing();
    make_cad(&mut app);
    let _ = app
        .settings
        .choose(&[("appearance.textSize", serde_json::json!(text_size))]);
    app.apply_settings();
    app.window_size = size;
    app.open_wall_styles();
    let id = app
        .wall_styles
        .as_ref()
        .unwrap()
        .builtin
        .iter()
        .find(|s| s.name == "Dış duvar 25")
        .expect("builtin fixture")
        .id
        .clone();
    let _ = app.wall_styles_event(Event::Choose(Some(id)));
    let _ = app.wall_styles_event(Event::Take);
    if long {
        // Only the window's draft is expanded: both nested scroll ranges are
        // non-empty without writing a style to the drawing.
        let _ = app.wall_styles_event(Event::Width("1.0".into()));
        for _ in 0..12 {
            let _ = app.wall_styles_event(Event::Add);
        }
        let chosen = app.wall_styles.as_ref().unwrap().chosen.clone();
        for _ in 0..22 {
            let _ = app.wall_styles_event(Event::New);
        }
        let _ = app.wall_styles_event(Event::Choose(chosen));
    }
    app
}

fn view(app: &App) -> Element<'_, Message> {
    assert_typography(app);
    if app.wall_styles.is_some() {
        app.wall_styles_view()
    } else {
        iced::widget::space().into()
    }
}

fn input(shot: &mut Snapshot, app: &mut App, value: Input) {
    assert_typography(app);
    shot.input(
        app,
        view,
        &mut |app, message| {
            assert_typography(app);
            let _ = app.update(message);
            assert_typography(app);
        },
        value,
    );
    assert_typography(app);
}

fn assert_typography(app: &App) {
    assert_eq!(
        typography::current(),
        app.typography,
        "another test changed the global font during the wall-style interaction",
    );
}

#[derive(Clone, Copy, Debug)]
struct Scroll {
    bounds: Rectangle,
    content_height: f32,
    offset: f32,
}

#[derive(Clone, Default)]
struct Layout {
    scrolls: Vec<Scroll>,
    texts: Vec<(String, Rectangle)>,
    inputs: Vec<Rectangle>,
}
impl Layout {
    fn text(&self, caption: &str) -> Rectangle {
        self.texts
            .iter()
            .find_map(|(text, bounds)| (text == caption).then_some(*bounds))
            .unwrap_or_else(|| panic!("text: {caption}"))
    }
}

fn layout(shot: &mut Snapshot, app: &App) -> Layout {
    assert_typography(app);
    struct Read {
        found: Arc<Mutex<Layout>>,
        offset: Vector,
        pending: Option<Vector>,
    }
    impl Operation for Read {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            let saved = self.offset;
            if let Some(offset) = self.pending.take() {
                self.offset += offset;
            }
            visit(self);
            self.offset = saved;
        }
        fn scrollable(
            &mut self,
            _: Option<&Id>,
            bounds: Rectangle,
            content: Rectangle,
            translation: Vector,
            _: &mut dyn Scrollable,
        ) {
            self.found.lock().unwrap().scrolls.push(Scroll {
                bounds: bounds - self.offset,
                content_height: content.height,
                offset: translation.y,
            });
            self.pending = Some(translation);
        }
        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
            self.found
                .lock()
                .unwrap()
                .texts
                .push((text.into(), bounds - self.offset));
        }
        fn text_input(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn TextInput) {
            self.found.lock().unwrap().inputs.push(bounds - self.offset);
        }
    }
    let found = Arc::new(Mutex::new(Layout::default()));
    shot.operate(
        view(app),
        Box::new(Read {
            found: found.clone(),
            offset: Vector::ZERO,
            pending: None,
        }),
    );
    assert_typography(app);
    found.lock().unwrap().clone()
}

fn inside(a: Rectangle, b: Rectangle) -> bool {
    a.x >= b.x - 0.5
        && a.y >= b.y - 0.5
        && a.x + a.width <= b.x + b.width + 0.5
        && a.y + a.height <= b.y + b.height + 0.5
}

#[test]
fn short_large_wall_styles_scrolls_the_right_form_and_left_list_without_hiding_actions() {
    let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
    let _restore = RestoreTypography;
    for height in [650., 682.] {
        let size = Size::new(1100., height);
        let mut app = open(size, 18, true);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let selection = app.selection.ids().to_vec();
        let mut shot = offscreen(size);
        let first = layout(&mut shot, &app);
        assert_eq!(
            first.scrolls.len(),
            2,
            "outer body and existing inner style list"
        );
        let (outer, inner) = (first.scrolls[0], first.scrolls[1]);
        assert!(outer.content_height > outer.bounds.height + 100.);
        assert!(inner.content_height > inner.bounds.height + 100.);
        for caption in [SAVE, CANCEL] {
            assert!(
                inside(first.text(caption), Rectangle::with_size(size)),
                "{caption}"
            );
        }
        input(
            &mut shot,
            &mut app,
            Input::Scroll(inner.bounds.center(), -6.),
        );
        let left = layout(&mut shot, &app);
        assert!(
            left.scrolls[1].offset > 20.,
            "real wheel over the style list"
        );
        assert_eq!(
            left.scrolls[0].offset, 0.,
            "inner list must handle its own wheel"
        );
        let at = Point::new(
            outer.bounds.x + outer.bounds.width - 40.,
            outer.bounds.y + 40.,
        );
        let mut route = Vec::new();
        for event in [
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: at }),
            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Lines { x: 0., y: -100. },
            }),
        ] {
            let (messages, status) = shot.deliver(view(&app), &event);
            assert_typography(&app);
            let count = messages.len();
            for message in messages {
                let _ = app.update(message);
                assert_typography(&app);
            }
            route.push((status, count, layout(&mut shot, &app).scrolls));
        }
        shot.settle(&mut app, view, &mut |app, message| {
            let _ = app.update(message);
        });
        let down = layout(&mut shot, &app);
        assert!(
            down.scrolls[0].offset > 100.,
            "right form wheel at {at:?}, initial outer {outer:?}, initial inner {inner:?}, route {route:?}, final {:?}",
            down.scrolls,
        );
        assert!(inside(
            down.text("Kesit (ayrıntılı)"),
            down.scrolls[0].bounds
        ));
        assert!(inside(
            *down.inputs.last().expect("last component thickness"),
            down.scrolls[0].bounds
        ));
        for caption in [SAVE, CANCEL] {
            assert_eq!(down.text(caption), first.text(caption));
        }
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        input(
            &mut shot,
            &mut app,
            Input::Click(down.text(CANCEL).center()),
        );
        assert!(app.wall_styles.is_none());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        assert_eq!(app.selection.ids(), selection);
    }
}

#[test]
fn wide_standard_wall_styles_keeps_its_natural_size_and_cancel_keeps_the_model() {
    let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
    let _restore = RestoreTypography;
    let size = Size::new(1440., 900.);
    let mut app = open(size, 13, false);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let mut shot = offscreen(size);
    let actual = layout(&mut shot, &app);
    let body = actual.scrolls[0];
    assert_eq!(body.offset, 0.);
    assert!(
        body.content_height <= body.bounds.height + 0.5,
        "no unnecessary body scroll"
    );
    assert!(
        body.bounds.y > 60. && actual.text(CANCEL).y < 830.,
        "natural centred height"
    );
    for caption in [SAVE, CANCEL] {
        assert!(inside(actual.text(caption), Rectangle::with_size(size)));
    }
    input(
        &mut shot,
        &mut app,
        Input::Click(actual.text(CANCEL).center()),
    );
    assert!(app.wall_styles.is_none());
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}
