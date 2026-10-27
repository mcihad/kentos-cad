//! Actual menu widgets: large text must not hide settings below the window.
use std::sync::{Arc, Mutex};

use iced::advanced::widget::{Id, Operation, operation::Scrollable};
use iced::{Element, Rectangle, Size, Task, Vector};
use kentos_ui::snapshot::{Input, Snapshot};
use kentos_ui::theme::typography::{self, Typography};

use super::{Event, Focus};
use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, offscreen};

struct RestoreTypography;
impl Drop for RestoreTypography {
    fn drop(&mut self) {
        typography::set(Typography::DEFAULT);
    }
}

fn open(size: Size, text_size: u32) -> App {
    let mut app = app_with_drawing();
    let _ = app
        .settings
        .choose(&[("appearance.textSize", serde_json::json!(text_size))]);
    app.apply_settings();
    app.window_size = size;
    let _ = app.update(Message::AppMenu(Event::Toggle));
    app
}

fn view(app: &App) -> Element<'_, Message> {
    assert_typography(app);
    app.app_menu_view()
        .unwrap_or_else(|| iced::widget::space().into())
}

fn input(shot: &mut Snapshot, app: &mut App, input: Input) {
    assert_typography(app);
    shot.input(
        app,
        view,
        &mut |app, message| {
            assert_typography(app);
            let _ = app.update(message);
            assert_typography(app);
        },
        input,
    );
    assert_typography(app);
}

fn assert_typography(app: &App) {
    assert_eq!(
        typography::current(),
        app.typography,
        "another test changed the global font during the menu interaction",
    );
}

#[derive(Clone, Copy, Debug)]
struct Scroll {
    bounds: Rectangle,
    content: Rectangle,
    offset: f32,
}

#[derive(Default, Clone)]
struct Layout {
    scrolls: Vec<Scroll>,
    texts: Vec<(String, Rectangle)>,
}

impl Layout {
    fn nav(&self) -> Scroll {
        *self
            .scrolls
            .iter()
            .min_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x))
            .expect("a real navigation scroll viewport")
    }

    fn text(&self, caption: &str) -> Rectangle {
        self.texts
            .iter()
            .find_map(|(text, bounds)| (text == caption).then_some(*bounds))
            .unwrap_or_else(|| panic!("menu text: {caption}"))
    }

    fn nav_text(&self, caption: &str) -> Rectangle {
        self.text(caption) - Vector::new(0., self.nav().offset)
    }
}

fn layout(shot: &mut Snapshot, app: &App) -> Layout {
    assert_typography(app);
    struct Read(Arc<Mutex<Layout>>);
    impl Operation for Read {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
            self.0.lock().unwrap().texts.push((text.to_owned(), bounds));
        }
        fn scrollable(
            &mut self,
            _: Option<&Id>,
            bounds: Rectangle,
            content: Rectangle,
            translation: Vector,
            _: &mut dyn Scrollable,
        ) {
            self.0.lock().unwrap().scrolls.push(Scroll {
                bounds,
                content,
                offset: translation.y,
            });
        }
    }
    let found = Arc::new(Mutex::new(Layout::default()));
    shot.operate(view(app), Box::new(Read(found.clone())));
    assert_typography(app);
    found.lock().unwrap().clone()
}

fn inside(bounds: Rectangle, within: Rectangle) -> bool {
    bounds.x >= within.x - 0.5
        && bounds.y >= within.y - 0.5
        && bounds.x + bounds.width <= within.x + within.width + 0.5
        && bounds.y + bounds.height <= within.y + within.height + 0.5
}

#[test]
fn short_large_text_menu_keeps_footer_visible_and_wheel_reaches_project_settings() {
    let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
    let _restore = RestoreTypography;
    for text_size in [16, 18] {
        let size = Size::new(1100., 650.);
        let window = Rectangle::with_size(size);
        let mut app = open(size, text_size);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let mut shot = offscreen(size);
        let initial = layout(&mut shot, &app);
        assert_eq!(
            initial.scrolls.len(),
            2,
            "independent command and detail panes"
        );
        let nav = initial.nav();
        assert!(nav.content.height > nav.bounds.height + 20.);
        for caption in ["Uygulama ayarları", "Kısayollar", "Hakkında"] {
            assert!(
                inside(initial.text(caption), window),
                "{caption} at {text_size}px"
            );
        }
        // A real wheel event over the command column, not an operation that
        // forces a stored offset: hovering an export row must not steal it.
        let at = nav.bounds.center();
        let mut route = Vec::new();
        for event in [
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: at }),
            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Lines { x: 0., y: -30. },
            }),
        ] {
            let (messages, status) = shot.deliver(view(&app), &event);
            assert_typography(&app);
            let count = messages.len();
            for message in messages {
                let _ = app.update(message);
                assert_typography(&app);
            }
            route.push((status, count, layout(&mut shot, &app).nav()));
        }
        shot.settle(&mut app, view, &mut |app, message| {
            let _ = app.update(message);
        });
        let scrolled = layout(&mut shot, &app);
        assert!(
            scrolled.nav().offset > 20.,
            "navigation wheel at {at:?}, initial {nav:?}, route {route:?}, final {:?}",
            scrolled.nav(),
        );
        assert_eq!(
            scrolled.text("Uygulama ayarları"),
            initial.text("Uygulama ayarları")
        );
        let settings = scrolled.nav_text("Proje ayarları");
        assert!(inside(settings, scrolled.nav().bounds));
        input(&mut shot, &mut app, Input::Click(settings.center()));
        assert!(app.app_menu.is_none());
        assert!(
            app.project.is_some(),
            "the last command actually opens its window"
        );
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );

        // The footer opens settings directly, without scrolling the commands.
        let mut app = open(size, text_size);
        let mut shot = offscreen(size);
        let footer = layout(&mut shot, &app).text("Uygulama ayarları");
        input(&mut shot, &mut app, Input::Click(footer.center()));
        assert_eq!(app.dialog, Some(Dialog::Settings));
        assert!(app.app_menu.is_none());
    }
}

#[test]
fn wide_standard_menu_keeps_natural_height_and_keyboard_reveals_hidden_commands() {
    let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
    let _restore = RestoreTypography;
    let size = Size::new(1440., 900.);
    let app = open(size, 13);
    let mut shot = offscreen(size);
    let wide = layout(&mut shot, &app);
    let nav = wide.nav();
    assert_eq!(nav.offset, 0.);
    assert!((nav.content.height - nav.bounds.height).abs() < 1.);
    assert!(
        wide.text("Uygulama ayarları").y < 750.,
        "do not stretch the old menu to the window"
    );
    assert!(inside(wide.nav_text("Proje ayarları"), nav.bounds));

    let size = Size::new(1100., 650.);
    let mut app = open(size, 18);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let mut shot = offscreen(size);
    let press = |app: &mut App, named| {
        app.update(Message::Key(crate::keys::KeyPress {
            key: iced::keyboard::Key::Named(named),
            physical: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            modifiers: iced::keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }))
    };
    fn drive(shot: &mut Snapshot, app: &App, task: Task<Message>) {
        use iced::futures::StreamExt as _;
        if let Some(mut stream) = iced_runtime::task::into_stream(task) {
            while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                if let iced_runtime::Action::Widget(operation) = action {
                    shot.operate(view(app), operation);
                }
            }
        }
    }
    let task = press(&mut app, iced::keyboard::key::Named::ArrowDown);
    drive(&mut shot, &app, task);
    let task = press(&mut app, iced::keyboard::key::Named::ArrowUp);
    drive(&mut shot, &app, task);
    assert_eq!(app.app_menu.as_ref().unwrap().focus, Some(Focus::Nav(8)));
    let last = layout(&mut shot, &app);
    assert!(last.nav().offset > 20.);
    assert!(inside(last.nav_text("Proje ayarları"), last.nav().bounds));
    let task = press(&mut app, iced::keyboard::key::Named::ArrowDown);
    drive(&mut shot, &app, task);
    let first = layout(&mut shot, &app);
    assert_eq!(first.nav().offset, 0.);
    assert!(inside(first.nav_text("Yeni"), first.nav().bounds));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}
