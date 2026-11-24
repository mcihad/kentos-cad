//! The interface without a window, driven as Iced's runtime drives it (the core's test kit, docs/adr/9199 §4.11): the
//! app's view built, laid out and drawn by a headless renderer, wgpu when there is a GPU and tiny-skia otherwise, and the
//! window's picture read back. The measurements of `perf` (TODOS.md PERF-03, PERF-08) and the drawing area's visual
//! conformance (plan/render.md RM-16, docs/adr/9224) drive the app through it; a module package's tests use it too.
//!
//! Test code only.
#![cfg_attr(not(test), allow(dead_code))]

use std::time::Instant;

use iced::advanced::clipboard;
use iced::advanced::renderer::{self, Headless};
use iced::keyboard::{self, key};
use iced::{Event, Pixels, Point, Renderer, Size, mouse, window};
use iced_runtime::user_interface::{self, UserInterface};
use kentos_ui::theme::typography;

use crate::app::App;

/// The window measured: the owner's reference size.
pub const WINDOW: Size = Size::new(1440.0, 900.0);

/// One frame's parts, milliseconds.
#[derive(Debug, Clone, Copy, Default)]
pub struct Parts {
    pub event: f64,
    pub app: f64,
    pub view: f64,
    pub layout: f64,
    pub draw: f64,
    pub gpu: Option<f64>,
    /// The app's view was built for this frame: a message reached the app since the last one (docs/adr/9205). A frame
    /// a widget asked for without a message (a gesture of the area's live camera) redraws the interface as it is,
    /// as the runtime does; its view and layout are not counted.
    pub rebuilt: bool,
}

impl Parts {
    /// The UI thread's share: everything but the GPU's.
    pub fn cpu(&self) -> f64 {
        self.event + self.app + self.view + self.layout + self.draw
    }

    /// The UI thread's parts: event, app, view, layout, draw (milliseconds).
    pub fn split(&self) -> [f64; 5] {
        [self.event, self.app, self.view, self.layout, self.draw]
    }

    /// `App::view`'s share, milliseconds (the drawing's scene is built there when it is out of date).
    pub fn view_ms(&self) -> f64 {
        self.view
    }

    /// The headless GPU's frame and read back, milliseconds (none without a GPU).
    pub fn gpu_ms(&self) -> f64 {
        self.gpu.unwrap_or(0.0)
    }

    /// Whether the app's view was built for this frame.
    pub fn rebuilt(&self) -> bool {
        self.rebuilt
    }
}

/// The interface without a window, driven as Iced's runtime drives it.
pub struct Harness {
    pub renderer: Renderer,
    cache: user_interface::Cache,
    cursor: mouse::Cursor,
    gpu: bool,
    /// A message reached the app after the last frame was drawn (what its redraw asked for): the next frame builds
    /// the view.
    dirty: bool,
}

fn redraw() -> Event {
    Event::Window(window::Event::RedrawRequested(iced::time::Instant::now()))
}

pub fn moved(position: Point) -> Event {
    Event::Mouse(mouse::Event::CursorMoved { position })
}

/// A left click at a place: the pointer there, the button pressed and let go.
pub fn click(position: Point) -> [Event; 3] {
    [
        moved(position),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]
}

impl Harness {
    pub fn new() -> Self {
        typography::load();
        let asked = std::env::var("KENTOS_SNAPSHOT_BACKEND").unwrap_or_else(|_| "wgpu".into());
        let make = |backend: &str| {
            iced::futures::executor::block_on(<Renderer as Headless>::new(
                typography::ui(),
                Pixels(16.0),
                Some(backend),
            ))
        };
        let renderer = make(&asked)
            .or_else(|| make("tiny-skia"))
            .expect("a renderer, wgpu or tiny-skia");
        let gpu = renderer.name().contains("wgpu");
        Self {
            renderer,
            cache: user_interface::Cache::default(),
            cursor: mouse::Cursor::Unavailable,
            gpu,
            dirty: true,
        }
    }

    /// One input: `act` is the app taking it (by default, the widgets'
    /// messages for `events`), then the interface built, laid out and drawn.
    pub fn frame_with(
        &mut self,
        app: &mut App,
        events: &[Event],
        act: impl FnOnce(&mut App),
    ) -> Parts {
        self.frame_acting(app, events, Some(act))
    }

    /// One input; `act`, when given, is the app taking something besides the widgets' messages.
    fn frame_acting(
        &mut self,
        app: &mut App,
        events: &[Event],
        act: Option<impl FnOnce(&mut App)>,
    ) -> Parts {
        for event in events {
            if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
                self.cursor = mouse::Cursor::Available(*position);
            }
        }
        let mut parts = Parts::default();
        // The interface the last frame left: the runtime keeps it between
        // inputs, so building it again here is not measured.
        let mut messages = Vec::new();
        if !events.is_empty() {
            let mut ui = UserInterface::build(
                app.view(),
                WINDOW,
                std::mem::take(&mut self.cache),
                &mut self.renderer,
            );
            let t = Instant::now();
            let (_, statuses) = ui.update(
                events,
                self.cursor,
                &mut self.renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            // Keys reach the app through its subscription, after the widgets saw them.
            for (event, status) in events.iter().zip(statuses) {
                if let Event::Keyboard(_) = event
                    && let Some(message) =
                        crate::keys::key_event(event.clone(), status, window::Id::unique())
                {
                    messages.push(message);
                }
            }
            parts.event = ms(t);
            self.cache = ui.into_cache();
        }
        let t = Instant::now();
        // The runtime builds the app's view only after a message (the runtime's `update` then `view`): a frame a
        // widget asked for without one draws the interface it has (docs/adr/9205).
        parts.rebuilt = self.dirty || !messages.is_empty() || act.is_some();
        for message in messages {
            let _ = app.update(message);
        }
        if let Some(act) = act {
            act(app);
        }
        parts.app = ms(t);
        let theme = app.theme();
        let t = Instant::now();
        let element = app.view();
        parts.view = if parts.rebuilt { ms(t) } else { 0.0 };
        let t = Instant::now();
        let mut ui = UserInterface::build(
            element,
            WINDOW,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );
        parts.layout = if parts.rebuilt { ms(t) } else { 0.0 };
        let t = Instant::now();
        let mut later = Vec::new();
        let _ = ui.update(
            &[redraw()],
            self.cursor,
            &mut self.renderer,
            &mut clipboard::Null,
            &mut later,
        );
        ui.draw(
            &mut self.renderer,
            &theme,
            &renderer::Style {
                text_color: theme.palette().text,
            },
            self.cursor,
        );
        parts.draw = ms(t);
        self.cache = ui.into_cache();
        if self.gpu {
            let t = Instant::now();
            let _ = self.renderer.screenshot(
                Size::new(WINDOW.width as u32, WINDOW.height as u32),
                1.0,
                theme.palette().background,
            );
            parts.gpu = Some(ms(t));
        }
        // What the redraw asked for goes to the app before the next input, as in the runtime.
        self.dirty = !later.is_empty();
        for message in later {
            let _ = app.update(message);
        }
        parts
    }

    pub fn frame(&mut self, app: &mut App, events: &[Event]) -> Parts {
        self.frame_acting(app, events, None::<fn(&mut App)>)
    }

    /// The last frame drawn again and read back: RGBA rows of the window at one device pixel per logical pixel
    /// (docs/adr/1062's pixel comparisons). Empty without a GPU.
    pub fn picture(&mut self, app: &App) -> Vec<u8> {
        if !self.gpu {
            return Vec::new();
        }
        self.renderer.screenshot(
            Size::new(WINDOW.width as u32, WINDOW.height as u32),
            1.0,
            app.theme().palette().background,
        )
    }

    /// Whether frames are drawn on a GPU (wgpu), not in software.
    pub fn on_gpu(&self) -> bool {
        self.gpu
    }

    /// Frames until the interface asks for nothing more (the area's size, the first fit).
    pub fn settle(&mut self, app: &mut App) {
        self.dirty = true;
        for _ in 0..12 {
            let mut messages = Vec::new();
            let mut ui = UserInterface::build(
                app.view(),
                WINDOW,
                std::mem::take(&mut self.cache),
                &mut self.renderer,
            );
            let _ = ui.update(
                &[redraw()],
                self.cursor,
                &mut self.renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            self.cache = ui.into_cache();
            if messages.is_empty() {
                break;
            }
            for message in messages {
                let _ = app.update(message);
            }
        }
    }
}

/// A key pressed and let go, as the window gives it.
pub fn key(named: key::Named) -> Vec<Event> {
    let physical = key::Physical::Unidentified(key::NativeCode::Unidentified);
    let key = keyboard::Key::Named(named);
    vec![
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key.clone(),
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }),
        Event::Keyboard(keyboard::Event::KeyReleased {
            key: key.clone(),
            modified_key: key,
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
        }),
    ]
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}
