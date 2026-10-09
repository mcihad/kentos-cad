//! Genel bakış and Büyüteç over the drawing (docs/adr/0181; the web's
//! `viewport/navigationCards.ts`): two cards at the places the core's rule
//! gives them (`kentos_geometry_core::tools::navigation`).
//!
//! - The overview shows the core's picture of the whole drawing
//!   (`Store::overview_picture`), drawn again 250 ms after the drawing, its
//!   layers or the drawing area's ground change, never for a view change,
//!   with the view's frame over it. A press moves the view there (one step
//!   of the view's history), a drag pans, the wheel zooms the view about its
//!   middle, a double click shows everything. The picture's pixels are drawn
//!   as runs of rectangles, kept until the picture changes.
//! - The magnifier is a frame and a title row with its zooms around a window
//!   the drawing area's renderer draws again through a second camera
//!   (`viewport::Viewport::lens`), and the drawing's text and marks through
//!   the same camera (`labels::layer`, `preview::layer`). It looks where the
//!   pointer last was on the drawing and moves to the other side when the
//!   pointer nears it.
//!
//! Both are the layout's (`overview`, `magnifier`, `magnifierZoom` in
//! `yerlesim.json`), not the drawing's.

use std::cell::Cell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{Duration, Instant};

use iced::futures::channel::oneshot;
use iced::futures::{Stream, StreamExt, stream};
use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::widget::{button, column, container, row};
use iced::{
    Background, Border, Center, Color, Element, Fill, Padding, Point, Rectangle, Renderer, Size,
    Subscription, Theme, mouse,
};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::overview::OverviewRequest;
use kentos_geometry_core::tools::navigation::{
    self as rule, Cards, Fit, LENS, OVERVIEW, Side, ZOOMS,
};
use kentos_render_wgpu::{Camera, Vec2};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::shape::{self, Level};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::{label, style};
use serde_json::Value;

use crate::app::{App, Message};

/// How long after a change the overview's picture is drawn again.
const REDRAW: Duration = Duration::from_millis(250);
/// The title row's height, logical pixels at the interface's type scale.
const HEADER: f32 = 24.0;
/// Two presses this close are a double click.
const DOUBLE: Duration = Duration::from_millis(400);

/// The overview's picture as drawn, and what it was drawn at.
pub struct Picture {
    width: usize,
    height: usize,
    /// RGBA, straight alpha, rows top down.
    pixels: Vec<u8>,
    dpr: f64,
    fit: Fit,
    extent: Bounds,
    /// Bumped with every new picture: the canvas draws its runs again.
    number: u64,
}

/// The cards' session state.
pub struct Navigation {
    side: Side,
    /// Where the magnifier looks: the pointer's last place on the drawing.
    at: Option<Vec2>,
    picture: Option<Picture>,
    /// What the picture was drawn for (`App::overview_key`).
    drawn_for: Option<u64>,
    /// When the drawing first changed after the picture was drawn.
    changed: Option<Instant>,
    /// The overview's left button is down (a drag pans).
    pressed: bool,
    pictures: u64,
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            side: Side::Right,
            at: None,
            picture: None,
            drawn_for: None,
            changed: None,
            pressed: false,
            pictures: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The overview's picture pressed at this point of it (logical pixels), or dragged there.
    Press(Point),
    Drag(Point),
    Release,
    DoubleClick,
    /// The magnifier's zoom button.
    Zoom(u32),
    CloseOverview,
    CloseLens,
    /// The overview's wait after a change is over.
    Due,
}

fn msg(event: Event) -> Message {
    Message::Navigation(event)
}

impl App {
    pub(crate) fn overview_shown(&self) -> bool {
        self.layout.flag("overview")
    }

    pub(crate) fn magnifier_shown(&self) -> bool {
        self.layout.flag("magnifier")
    }

    /// The magnifier's zoom: the kept one, at the nearest of its steps.
    pub(crate) fn magnifier_zoom(&self) -> u32 {
        let kept = self.layout.number("magnifierZoom");
        let off = |z: u32| (f64::from(z) / kept).log2().abs();
        ZOOMS
            .into_iter()
            .reduce(|a, b| if off(b) < off(a) { b } else { a })
            .unwrap_or(4)
    }

    /// Genel bakış (`view.overview`): shown or put away; shown, its picture at once.
    pub(crate) fn toggle_overview(&mut self) {
        let on = !self.overview_shown();
        self.layout
            .keep("overview", Value::from(on), Instant::now());
        self.navigation.drawn_for = None;
        self.navigation.changed = None;
        if on {
            self.draw_overview();
        }
    }

    /// Büyüteç (`view.magnifier`): shown or put away.
    pub(crate) fn toggle_magnifier(&mut self) {
        let on = !self.magnifier_shown();
        self.layout
            .keep("magnifier", Value::from(on), Instant::now());
    }

    /// Where the cards stand in the drawing area now (§4).
    fn cards(&self) -> Cards {
        let camera = &self.viewport.camera;
        rule::cards(
            (camera.width, camera.height),
            f64::from(header()),
            self.work_mode() != kentos_contracts::Workspace::Cad,
            self.overview_shown(),
        )
    }

    /// What the overview's picture depends on: the drawing and its layers,
    /// the drawing area's ground, the screen's pixel ratio.
    fn overview_key(&self) -> Option<u64> {
        let doc = self.document.as_ref()?;
        let mut key = DefaultHasher::new();
        (doc.model.generation(), doc.session).hash(&mut key);
        format!("{:?}", self.canvas()).hash(&mut key);
        // The time slider's window leaves its temporal layers' objects out (docs/adr/0210 §6).
        format!("{:?}", self.spatial.store().time_window()).hash(&mut key);
        self.viewport.scale_factor().to_bits().hash(&mut key);
        Some(key.finish())
    }

    /// The overview's picture from the store, each layer in its colour on the drawing's ground.
    fn draw_overview(&mut self) {
        let Some(doc) = &self.document else {
            self.navigation.picture = None;
            return;
        };
        self.spatial.sync(&doc.model);
        let palette = crate::viewport::palette(self.canvas());
        let colors: HashMap<String, [u8; 3]> = doc
            .model
            .layers()
            .leaves()
            .into_iter()
            .filter_map(|l| {
                let c = palette.resolve(&l.style.color)?;
                Some((l.id.clone(), [c.0[0], c.0[1], c.0[2]]))
            })
            .collect();
        let dpr = self.viewport.scale_factor();
        let store = self.spatial.store();
        let picture = store
            .overview_picture(&OverviewRequest {
                width: OVERVIEW.0,
                height: OVERVIEW.1,
                dpr,
                colors: &colors,
            })
            .zip(store.overview_extent());
        self.navigation.pictures += 1;
        self.navigation.picture = picture.map(|(p, extent)| Picture {
            width: p.width,
            height: p.height,
            pixels: p.pixels,
            dpr,
            fit: p.fit,
            extent,
            number: self.navigation.pictures,
        });
        self.navigation.drawn_for = self.overview_key();
        self.navigation.changed = None;
    }

    /// After every message: the magnifier follows the pointer and keeps away
    /// from it; the overview's picture is drawn again a moment after a change.
    pub(crate) fn follow_navigation(&mut self, now: Instant) {
        if self.magnifier_shown()
            && let Some(at) = self.viewport.cursor
        {
            self.navigation.at = Some(at);
            let [x, y] = self.viewport.camera.world_to_screen(at);
            let cards = self.cards();
            self.navigation.side = rule::next_side(&cards, self.navigation.side, (x, y));
        }
        if !self.overview_shown() {
            return;
        }
        let key = self.overview_key();
        if key == self.navigation.drawn_for {
            return;
        }
        match self.navigation.changed {
            _ if self.navigation.picture.is_none() && self.navigation.drawn_for.is_none() => {
                self.draw_overview();
            }
            None => self.navigation.changed = Some(now),
            Some(since) if now.saturating_duration_since(since) >= REDRAW => self.draw_overview(),
            Some(_) => {}
        }
    }

    /// When the overview's wait after a change ends.
    pub(crate) fn navigation_subscription(&self) -> Subscription<Message> {
        match self.navigation.changed {
            Some(since) if self.overview_shown() => Subscription::run_with(since + REDRAW, wake),
            _ => Subscription::none(),
        }
    }

    pub(crate) fn navigation_event(&mut self, event: Event) {
        match event {
            Event::Press(p) => {
                if let Some(world) = self.overview_world(p) {
                    self.navigation.pressed = true;
                    self.navigating(|app| app.viewport.camera.center_on(world));
                }
            }
            Event::Drag(p) => {
                if self.navigation.pressed
                    && let Some(world) = self.overview_world(p)
                {
                    self.viewport.camera.center_on(world);
                }
            }
            Event::Release => self.navigation.pressed = false,
            Event::DoubleClick => {
                self.navigation.pressed = false;
                let _ = self.update(Message::Run("view.zoomExtents"));
            }
            Event::Zoom(z) => {
                self.layout
                    .keep("magnifierZoom", Value::from(z), Instant::now());
            }
            Event::CloseOverview => {
                if self.overview_shown() {
                    self.toggle_overview();
                }
            }
            Event::CloseLens => {
                if self.magnifier_shown() {
                    self.toggle_magnifier();
                }
            }
            Event::Due => {}
        }
    }

    /// A point of the overview's picture in the drawing.
    fn overview_world(&self, p: Point) -> Option<Vec2> {
        let picture = self.navigation.picture.as_ref()?;
        let (x, y) = picture
            .fit
            .to_world(OVERVIEW, f64::from(p.x), f64::from(p.y));
        Some(Vec2::new(x, y))
    }

    /// A drawing point on the overview's picture (the traces press there); none when it shows nothing.
    pub(crate) fn overview_point(&self, world: Vec2) -> Option<Point> {
        let picture = self.navigation.picture.as_ref()?;
        let (u, v) = picture.fit.to_card(OVERVIEW, world.x, world.y);
        Some(Point::new(u as f32, v as f32))
    }

    /// Genel bakış's extent while it shows (the traces read it).
    pub(crate) fn overview_extent(&self) -> Option<Option<[f64; 4]>> {
        self.overview_shown().then(|| {
            self.navigation.picture.as_ref().map(|p| {
                [
                    p.extent.min_x,
                    p.extent.min_y,
                    p.extent.max_x,
                    p.extent.max_y,
                ]
            })
        })
    }

    /// Büyüteç's zoom, side and centre while it shows (the traces read it).
    pub(crate) fn magnifier_state(&self) -> Option<(u32, &'static str, Option<[f64; 2]>)> {
        self.magnifier_shown().then(|| {
            (
                self.magnifier_zoom(),
                self.navigation.side.word(),
                self.navigation.at.map(|p| [p.x, p.y]),
            )
        })
    }

    /// The cards over the drawing, each at its place.
    pub(crate) fn navigation_view(&self) -> Vec<Element<'_, Message>> {
        let mut out = Vec::new();
        let cards = self.cards();
        let head = header();
        let tokens = Tokens::of(&self.theme());
        if let Some(card) = cards.overview {
            let palette = crate::viewport::palette(self.canvas());
            let ground = palette.background.0;
            let camera = &self.viewport.camera;
            let picture = self.navigation.picture.as_ref();
            let frame = picture.map(|p| {
                p.fit.view_frame(
                    OVERVIEW,
                    (camera.center.x, camera.center.y),
                    1.0 / camera.scale,
                    (camera.width, camera.height),
                )
            });
            let body = canvas::Canvas::new(OverviewCanvas {
                picture,
                frame,
                ground: Color::from_rgb8(ground[0], ground[1], ground[2]),
                accent: tokens.accent,
                muted: tokens.muted,
                middle: Point::new((camera.width / 2.0) as f32, (camera.height / 2.0) as f32),
            })
            .width(OVERVIEW.0 as f32)
            .height(OVERVIEW.1 as f32);
            let title = title_row("Genel bakış", Vec::new(), msg(Event::CloseOverview), head);
            out.push(placed(card.card, column![title, body].into()));
        }
        if self.magnifier_shown() {
            let card = cards.lens(self.navigation.side);
            let [r, g, b, _] = crate::viewport::palette(self.canvas()).background.0;
            let ground = Color::from_rgb8(r, g, b);
            let zoom = self.magnifier_zoom();
            let zooms: Vec<Element<'_, Message>> = ZOOMS
                .into_iter()
                .map(|z| {
                    button(label::caption(format!("{z}×")))
                        .on_press(msg(Event::Zoom(z)))
                        .padding([0, 4])
                        .height(head - 6.0)
                        .style(style::button::toggle(z == zoom))
                        .into()
                })
                .collect();
            let title = title_row("Büyüteç", zooms, msg(Event::CloseLens), head);
            let body: Element<'_, Message> = match (self.navigation.at, &self.document) {
                (Some(at), Some(doc)) => {
                    let mut camera = Camera {
                        center: at,
                        scale: self.viewport.camera.scale * f64::from(zoom),
                        width: LENS.0,
                        height: LENS.1,
                    };
                    camera.center = at;
                    self.lens_view(doc, camera)
                }
                // Before the pointer has been on the drawing: where it will look.
                _ => container(label::caption("İmleci çizimin üstüne getirin."))
                    .center(Fill)
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(ground)),
                        ..container::Style::default()
                    })
                    .into(),
            };
            let window = container(body)
                .width(LENS.0 as f32)
                .height(LENS.1 as f32)
                .clip(true);
            out.push(placed(card.card, column![title, window].into()));
        }
        out
    }
}

/// The title row's height at the interface's type scale, whole pixels.
fn header() -> f32 {
    typography::scaled(HEADER).round()
}

/// A card's title row: its name, its own buttons and the close button.
fn title_row<'a>(
    name: &'a str,
    extra: Vec<Element<'a, Message>>,
    close: Message,
    head: f32,
) -> Element<'a, Message> {
    let mut line = row![container(label::caption(name)).width(Fill)]
        .spacing(1)
        .align_y(Center);
    for e in extra {
        line = line.push(e);
    }
    line = line.push(
        button(icon(Icon::Close).size(10.0).tone(Tone::Muted))
            .on_press(close)
            .padding(4)
            .style(style::button::ghost),
    );
    container(line)
        .height(head)
        .width(Fill)
        .padding(Padding {
            top: 0.0,
            right: 3.0,
            bottom: 0.0,
            left: 8.0,
        })
        .align_y(Center)
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            let r = shape::md();
            container::Style {
                background: Some(Background::Color(t.header)),
                border: Border {
                    radius: iced::border::Radius {
                        top_left: r,
                        top_right: r,
                        bottom_right: 0.0,
                        bottom_left: 0.0,
                    },
                    ..Border::default()
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// A card at its place in the drawing area: its frame, a shadow and its
/// content, the rest of the area free for the drawing's own events.
fn placed(rect: [f64; 4], content: Element<'_, Message>) -> Element<'_, Message> {
    let card = container(content)
        .width(rect[2] as f32)
        .height(rect[3] as f32)
        .padding(1)
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            let r = shape::md();
            container::Style {
                background: Some(Background::Color(t.surface)),
                border: Border {
                    color: t.border_strong(),
                    width: 1.0,
                    radius: iced::border::Radius {
                        top_left: r,
                        top_right: r,
                        bottom_right: 0.0,
                        bottom_left: 0.0,
                    },
                },
                shadow: shape::shadow(Level::Float, &t),
                ..container::Style::default()
            }
        });
    container(card)
        .padding(Padding {
            top: rect[1] as f32,
            right: 0.0,
            bottom: 0.0,
            left: rect[0] as f32,
        })
        .width(Fill)
        .height(Fill)
        .into()
}

fn wake(due: &Instant) -> impl Stream<Item = Message> + use<> {
    let due = *due;
    let (done, wait) = oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        let _ = done.send(());
    });
    stream::once(wait).map(|_| msg(Event::Due))
}

/// The overview's picture and the view's frame over it, taking the mouse.
struct OverviewCanvas<'a> {
    picture: Option<&'a Picture>,
    frame: Option<([f64; 4], bool)>,
    ground: Color,
    accent: Color,
    muted: Color,
    /// The drawing area's middle, where the wheel zooms the view about.
    middle: Point,
}

/// The pointer's state over the overview.
#[derive(Default)]
struct Gesture {
    pressed: bool,
    last_press: Option<Instant>,
    cache: canvas::Cache,
    drawn: Cell<u64>,
}

impl canvas::Program<Message> for OverviewCanvas<'_> {
    type State = Gesture;

    fn update(
        &self,
        state: &mut Gesture,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        self.picture?;
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                let now = Instant::now();
                let double = state
                    .last_press
                    .is_some_and(|t| now.saturating_duration_since(t) < DOUBLE);
                state.last_press = (!double).then_some(now);
                state.pressed = !double;
                Some(
                    canvas::Action::publish(msg(if double {
                        Event::DoubleClick
                    } else {
                        Event::Press(p)
                    }))
                    .and_capture(),
                )
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) if state.pressed => {
                let p = Point::new(position.x - bounds.x, position.y - bounds.y);
                Some(canvas::Action::publish(msg(Event::Drag(p))).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.pressed =>
            {
                state.pressed = false;
                Some(canvas::Action::publish(msg(Event::Release)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let steps = crate::viewport::wheel_steps(delta);
                let factor = steps.exp();
                (steps != 0.0 && factor.is_finite()).then(|| {
                    canvas::Action::publish(Message::Viewport(crate::viewport::Event::Zoomed {
                        factor,
                        at: self.middle,
                    }))
                    .and_capture()
                })
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Gesture,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let number = self.picture.map_or(0, |p| p.number);
        if state.drawn.get() != number {
            state.cache.clear();
            state.drawn.set(number);
        }
        let picture = state.cache.draw(renderer, bounds.size(), |frame| {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.ground);
            match self.picture {
                Some(p) => runs(frame, p),
                None => frame.fill_text(Text {
                    content: "Çizimde gösterilecek nesne yok.".to_owned(),
                    position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                    color: self.muted,
                    size: typography::scaled(12.0).into(),
                    align_x: iced::widget::text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
                    ..Text::default()
                }),
            }
        });
        let mut over = Frame::new(renderer, bounds.size());
        if let Some(([u0, v0, u1, v1], cross)) = self.frame {
            let stroke = Stroke::default().with_color(self.accent).with_width(1.5);
            if cross {
                let (x, y) = (((u0 + u1) / 2.0) as f32, ((v0 + v1) / 2.0) as f32);
                over.stroke(
                    &Path::new(|b| {
                        b.move_to(Point::new(x - 6.0, y));
                        b.line_to(Point::new(x + 6.0, y));
                        b.move_to(Point::new(x, y - 6.0));
                        b.line_to(Point::new(x, y + 6.0));
                    }),
                    stroke,
                );
            } else {
                let at = Point::new(u0 as f32, v0 as f32);
                let size = Size::new((u1 - u0) as f32, (v1 - v0) as f32);
                over.fill_rectangle(at, size, self.accent.scale_alpha(0.14));
                over.stroke(&Path::rectangle(at, size), stroke);
            }
        }
        vec![picture, over.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Gesture,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.pressed {
            mouse::Interaction::Grabbing
        } else if self.picture.is_some() && cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

/// The picture's pixels as runs of one colour along each row.
fn runs(frame: &mut Frame, p: &Picture) {
    let k = 1.0 / p.dpr as f32;
    for j in 0..p.height {
        let row = &p.pixels[j * p.width * 4..(j + 1) * p.width * 4];
        let mut i = 0;
        while i < p.width {
            let px = &row[i * 4..i * 4 + 4];
            if px[3] == 0 {
                i += 1;
                continue;
            }
            let start = i;
            while i < p.width && row[i * 4..i * 4 + 4] == *px {
                i += 1;
            }
            frame.fill_rectangle(
                Point::new(start as f32 * k, j as f32 * k),
                Size::new((i - start) as f32 * k, k),
                Color::from_rgba8(px[0], px[1], px[2], f32::from(px[3]) / 255.0),
            );
        }
    }
}

/// A small cross in the middle of the magnifier's window: the pointer's place.
pub(crate) fn lens_cross<'a>(color: Color) -> Element<'a, Message> {
    canvas::Canvas::new(LensCross(color))
        .width(Fill)
        .height(Fill)
        .into()
}

struct LensCross(Color);

impl canvas::Program<Message> for LensCross {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (x, y) = (
            (bounds.width / 2.0).round() + 0.5,
            (bounds.height / 2.0).round() + 0.5,
        );
        // Arms of 9 px, 3 px clear of the middle, in the drawing's ink (the web's too).
        let arms = Path::new(|b| {
            for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                b.move_to(Point::new(x + 3.0 * dx, y + 3.0 * dy));
                b.line_to(Point::new(x + 12.0 * dx, y + 12.0 * dy));
            }
        });
        frame.stroke(&arms, Stroke::default().with_color(self.0).with_width(1.0));
        vec![frame.into_geometry()]
    }
}
