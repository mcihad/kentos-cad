//! The paper on its desk (design §11): millimetre rulers on the top and the
//! left with their guides (KentOS UI `Rulers`), the sheet's layers and maps
//! stacked in its drawing order, and on top of them the pointer's layer:
//! the chosen items' frames and handles, the snapping lines and distances,
//! the box being drawn. The paper is white in every theme; the desk is the
//! theme's surface, and the chosen items and guides are drawn in its one
//! accent colour (“bir vurgu, bir anlam”).

use std::hash::{DefaultHasher, Hash, Hasher};

use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::widget::{Stack, canvas as canvas_widget, container, pin, responsive};
use iced::{
    Color, Element, Event, Fill, Point, Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse,
};
use kentos_sheet::display::{DisplayList, MapPrim, Prim};
use kentos_sheet::hit::{HandleHit, handle_points};
use kentos_sheet::model::{Axis, Sheet};
use kentos_sheet::snap::SnapKind;
use kentos_sheet::units::{RectUm, corners, rotated_bounds};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::Rulers;
use kentos_ui::widget::rulers::{self, Transform};

use crate::designer::{Designer, Drag, MapCache};
use crate::interact::ROTATE_OFFSET_PX;
use crate::message::{Message, StageEvent, Tool};
use crate::paint::{self, Entry, Options, Xf};
use crate::painter::{MapRequest, Painter};

/// The paper's shadow on the desk.
fn shadow(frame: &mut Frame, r: Rectangle, dark: bool) {
    let a = if dark { 0.45 } else { 0.16 };
    for (grow, alpha) in [(6.0, a * 0.18), (3.0, a * 0.35), (1.0, a * 0.6)] {
        frame.fill_rectangle(
            Point::new(r.x - grow * 0.5, r.y - grow * 0.2 + 1.5),
            Size::new(r.width + grow, r.height + grow),
            Color::from_rgba(0.0, 0.0, 0.0, alpha),
        );
    }
}

/// A run of the paper's primitives between two maps; the first also lays the paper down.
struct PaperLayer<'a> {
    d: &'a Designer,
    list: &'a DisplayList,
    entries: &'a [Entry],
    first: bool,
    xf: Xf,
    cache: Option<&'a canvas::Cache>,
    sheet: &'a Sheet,
    /// The design's aids (the desk's shadow, the margins, the snap grid); not in an export.
    aids: bool,
}

impl<'a> PaperLayer<'a> {
    fn paint(&self, frame: &mut Frame, theme: &Theme) {
        let pictures = |sha: &str| self.d.picture(sha);
        if self.first && !self.aids {
            paint::paper(frame, self.list, &self.xf);
        }
        if self.first && self.aids {
            let page = RectUm::new(0, 0, self.list.size.width, self.list.size.height);
            shadow(frame, self.xf.rect(&page), Tokens::of(theme).is_dark);
            paint::paper(frame, self.list, &self.xf);
            // The margins and the snap grid: aids of the design, never printed.
            let m = &self.sheet.page.margins;
            let inner = page.inset_sides(m.top, m.right, m.bottom, m.left);
            let r = self.xf.rect(&inner);
            let aid = Color::from_rgba8(0x5b, 0x7c, 0xb0, 0.45);
            frame.stroke(
                &Path::rectangle(r.position(), r.size()),
                Stroke {
                    line_dash: canvas::LineDash {
                        segments: &[4.0, 3.0],
                        offset: 0,
                    },
                    ..Stroke::default().with_color(aid).with_width(1.0)
                },
            );
            let g = &self.sheet.snap_grid;
            if g.visible && g.spacing > 0 {
                let step = self.xf.len(f64::from(g.spacing));
                if step >= 6.0 {
                    let dots = Path::new(|b| {
                        let mut y = 0;
                        while y <= self.list.size.height {
                            let mut x = 0;
                            while x <= self.list.size.width {
                                let p = self.xf.pt([x, y]);
                                b.rectangle(Point::new(p.x - 0.5, p.y - 0.5), Size::new(1.0, 1.0));
                                x += g.spacing;
                            }
                            y += g.spacing;
                        }
                    });
                    frame.fill(&dots, Color::from_rgba8(0x5b, 0x7c, 0xb0, 0.5));
                }
            }
        }
        let opts = if self.aids {
            Options::SCREEN
        } else {
            Options::EXPORT
        };
        paint::paint_entries(frame, self.list, self.entries, &self.xf, &pictures, opts);
    }
}

impl<'a, M> canvas::Program<M> for PaperLayer<'a> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        match self.cache {
            Some(c) => vec![c.draw(renderer, bounds.size(), |f| self.paint(f, theme))],
            None => {
                let mut f = Frame::new(renderer, bounds.size());
                self.paint(&mut f, theme);
                vec![f.into_geometry()]
            }
        }
    }
}

/// A map's content on a canvas of its own, as large as the map's box on the screen.
struct MapLayer<'a> {
    prim: &'a MapPrim,
    painter: Painter<'a>,
    cache: Option<&'a MapCache>,
    /// Pixels per micrometre.
    k: f64,
    /// The content's size unturned, pixels.
    size: Size,
    /// Where the content's middle is in this canvas.
    middle: Point,
    export: bool,
}

impl<'a> MapLayer<'a> {
    fn key(&self) -> u64 {
        let mut h = DefaultHasher::new();
        serde_json::to_string(&self.prim.view)
            .unwrap_or_default()
            .hash(&mut h);
        serde_json::to_string(&self.prim.layers)
            .unwrap_or_default()
            .hash(&mut h);
        self.prim.crs.hash(&mut h);
        self.prim.rotation.hash(&mut h);
        (
            (self.size.width * 4.0).round() as i64,
            (self.size.height * 4.0).round() as i64,
        )
            .hash(&mut h);
        self.painter.revision().hash(&mut h);
        h.finish()
    }

    fn paint(&self, frame: &mut Frame) {
        let request = MapRequest::of(self.prim, self.size, self.k, self.export);
        frame.translate(Vector::new(self.middle.x, self.middle.y));
        if self.prim.rotation != 0 {
            frame.rotate(iced::Radians(paint::rad(self.prim.rotation) as f32));
        }
        frame.translate(Vector::new(-self.size.width / 2.0, -self.size.height / 2.0));
        let content = Rectangle::new(Point::ORIGIN, self.size);
        if self.prim.view.center.is_none() || !self.painter.paint(&request, frame) {
            paint::placeholder(frame, content, 1.0);
        }
    }
}

impl<'a, M> canvas::Program<M> for MapLayer<'a> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        match self.cache {
            Some(c) => {
                let key = self.key();
                if c.key.get() != key {
                    c.cache.clear();
                    c.key.set(key);
                }
                vec![c.cache.draw(renderer, bounds.size(), |f| self.paint(f))]
            }
            None => {
                let mut f = Frame::new(renderer, bounds.size());
                self.paint(&mut f);
                vec![f.into_geometry()]
            }
        }
    }
}

/// The pointer's layer: what is chosen and what is being done, and the events of the paper.
struct Overlay<'a> {
    d: &'a Designer,
    xf: Xf,
    size: Size,
}

#[derive(Default)]
struct OverlayState {
    modifiers: keyboard::Modifiers,
    pressed: bool,
    panning: Option<Point>,
    last_click: Option<iced::advanced::mouse::Click>,
    inside: bool,
}

fn rotated_poly(xf: &Xf, r: &RectUm, rotation: i32) -> Vec<Point> {
    corners(r, rotation)
        .iter()
        .map(|c| xf.p(c[0], c[1]))
        .collect()
}

fn closed(pts: &[Point]) -> Path {
    Path::new(|b| {
        if let Some(first) = pts.first() {
            b.move_to(*first);
            for p in &pts[1..] {
                b.line_to(*p);
            }
            b.close();
        }
    })
}

impl<'a> Overlay<'a> {
    fn draw_into(&self, frame: &mut Frame, theme: &Theme) {
        let t = Tokens::of(theme);
        let accent = t.accent;
        let d = self.d;
        let xf = &self.xf;
        // The items where they are, or where a drag is taking them.
        let book = d
            .preview
            .as_ref()
            .and_then(|p| p.book.as_ref())
            .unwrap_or(&d.book);
        let item_frame =
            |id: &str| -> Option<(RectUm, i32)> { book.item(id).map(|i| (i.frame, i.rotation)) };
        // The hovered item, faint.
        if d.drag.is_none()
            && let Some(id) = &d.hovered_item
            && !d.selection.contains(id)
            && let Some((r, rot)) = item_frame(id)
        {
            frame.stroke(
                &closed(&rotated_poly(xf, &r, rot)),
                Stroke::default()
                    .with_color(Color { a: 0.55, ..accent })
                    .with_width(1.0),
            );
        }
        // The chosen items.
        for id in &d.selection {
            let Some((r, rot)) = item_frame(id) else {
                continue;
            };
            let locked = d.book.item(id).is_some_and(|i| i.locked);
            frame.stroke(
                &closed(&rotated_poly(xf, &r, rot)),
                Stroke {
                    line_dash: canvas::LineDash {
                        segments: if locked { &[3.0, 3.0] } else { &[] },
                        offset: 0,
                    },
                    ..Stroke::default().with_color(accent).with_width(1.25)
                },
            );
            // One unlocked item: its handles.
            if d.selection.len() == 1 && !locked && d.drag.is_none() {
                let off = (ROTATE_OFFSET_PX / xf.k).round() as i32;
                for (h, p) in handle_points(&r, rot, off) {
                    let c = xf.p(p[0], p[1]);
                    if h == HandleHit::Rotate {
                        let top = handle_points(&r, rot, 0)
                            .into_iter()
                            .find(|(x, _)| *x == HandleHit::N)
                            .map(|(_, q)| xf.p(q[0], q[1]));
                        if let Some(top) = top {
                            frame.stroke(
                                &Path::line(top, c),
                                Stroke::default().with_color(accent).with_width(1.0),
                            );
                        }
                        let circle = Path::circle(c, 4.5);
                        frame.fill(&circle, t.surface);
                        frame.stroke(
                            &circle,
                            Stroke::default().with_color(accent).with_width(1.5),
                        );
                    } else {
                        let s = 7.0;
                        let sq = Path::rectangle(
                            Point::new(c.x - s / 2.0, c.y - s / 2.0),
                            Size::new(s, s),
                        );
                        frame.fill(&sq, Color::WHITE);
                        frame.stroke(&sq, Stroke::default().with_color(accent).with_width(1.25));
                    }
                }
            }
        }
        // Several chosen: their box.
        if d.selection.len() > 1
            && d.drag.is_none()
            && let Some(b) = d.chosen_bounds()
        {
            let r = xf.rect(&b);
            frame.stroke(
                &Path::rectangle(r.position(), r.size()),
                Stroke {
                    line_dash: canvas::LineDash {
                        segments: &[5.0, 3.0],
                        offset: 0,
                    },
                    ..Stroke::default().with_color(accent).with_width(1.0)
                },
            );
        }
        // The snapping: lines, distances, equal gaps.
        if let Some(p) = &d.preview {
            let snap = Color { a: 0.9, ..t.snap() };
            for l in &p.lines {
                let (a, b) = match l.axis {
                    Axis::X => (xf.pt([l.at, l.from]), xf.pt([l.at, l.to])),
                    Axis::Y => (xf.pt([l.from, l.at]), xf.pt([l.to, l.at])),
                };
                let dashed = matches!(l.kind, SnapKind::Grid | SnapKind::Margin);
                frame.stroke(
                    &Path::line(a, b),
                    Stroke {
                        line_dash: canvas::LineDash {
                            segments: if dashed { &[4.0, 3.0] } else { &[] },
                            offset: 0,
                        },
                        ..Stroke::default().with_color(snap).with_width(1.0)
                    },
                );
            }
            for g in &p.gaps {
                let (a, b) = (xf.pt(g.from), xf.pt(g.to));
                frame.stroke(
                    &Path::line(a, b),
                    Stroke::default().with_color(snap).with_width(1.0),
                );
                let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                badge(frame, mid, &g.text, snap);
            }
            for s in &p.spacing {
                for seg in &s.segments {
                    let (a, b) = (xf.pt(seg[0]), xf.pt(seg[1]));
                    frame.stroke(
                        &Path::line(a, b),
                        Stroke {
                            line_dash: canvas::LineDash {
                                segments: &[2.0, 2.0],
                                offset: 0,
                            },
                            ..Stroke::default().with_color(snap).with_width(1.0)
                        },
                    );
                }
            }
        }
        // A box being drawn: to choose, or a new item's.
        match &d.drag {
            Some(Drag::Band { start, end, .. }) => {
                let (a, b) = (xf.p(start[0], start[1]), xf.p(end[0], end[1]));
                let r = Rectangle::new(
                    Point::new(a.x.min(b.x), a.y.min(b.y)),
                    Size::new((a.x - b.x).abs(), (a.y - b.y).abs()),
                );
                let touching = end[0] < start[0];
                frame.fill_rectangle(r.position(), r.size(), Color { a: 0.08, ..accent });
                frame.stroke(
                    &Path::rectangle(r.position(), r.size()),
                    Stroke {
                        line_dash: canvas::LineDash {
                            segments: if touching { &[4.0, 3.0] } else { &[] },
                            offset: 0,
                        },
                        ..Stroke::default().with_color(accent).with_width(1.0)
                    },
                );
            }
            Some(Drag::Create { start, end, .. }) => {
                let (a, b) = (xf.p(start[0], start[1]), xf.p(end[0], end[1]));
                let r = Rectangle::new(
                    Point::new(a.x.min(b.x), a.y.min(b.y)),
                    Size::new((a.x - b.x).abs(), (a.y - b.y).abs()),
                );
                frame.fill_rectangle(r.position(), r.size(), Color { a: 0.06, ..accent });
                frame.stroke(
                    &Path::rectangle(r.position(), r.size()),
                    Stroke::default().with_color(accent).with_width(1.25),
                );
                let text = format!(
                    "{} × {} mm",
                    kentos_geometry_core::display::fixed((end[0] - start[0]).abs() / 1000.0, 1),
                    kentos_geometry_core::display::fixed((end[1] - start[1]).abs() / 1000.0, 1)
                );
                badge(
                    frame,
                    Point::new(r.x + r.width / 2.0, r.y + r.height + 12.0),
                    &text,
                    accent,
                );
            }
            _ => {}
        }
    }
}

/// A distance's label: white on the snapping colour.
fn badge(frame: &mut Frame, at: Point, text: &str, color: Color) {
    let w = text.chars().count() as f32 * 6.2 + 10.0;
    let r = Rectangle::new(Point::new(at.x - w / 2.0, at.y - 8.0), Size::new(w, 16.0));
    frame.fill(
        &Path::rounded_rectangle(r.position(), r.size(), 3.0.into()),
        color,
    );
    frame.fill_text(Text {
        content: text.to_owned(),
        position: at,
        color: Color::WHITE,
        size: iced::Pixels(11.0),
        font: kentos_ui::theme::typography::mono(),
        align_x: iced::widget::text::Alignment::Center,
        align_y: iced::alignment::Vertical::Center,
        ..Text::default()
    });
}

impl<'a> canvas::Program<Message> for Overlay<'a> {
    type State = OverlayState;

    fn update(
        &self,
        state: &mut OverlayState,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let size = self.size;
        let paper = |p: Point| self.xf.paper(p);
        let m = state.modifiers;
        // A redraw at another size than the mode knows: it is told (once; then they agree).
        if matches!(event, Event::Window(_)) && size != self.d.stage {
            return Some(canvas::Action::publish(Message::Stage(
                StageEvent::Resize { size },
            )));
        }
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) => {
                state.modifiers = *mods;
                None
            }
            // Space held with the pointer over the paper: the hand for a moment (design §11).
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Space),
                repeat: false,
                ..
            }) if state.inside && !self.d.space => {
                Some(canvas::Action::publish(Message::Space(true)).and_capture())
            }
            Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(keyboard::key::Named::Space),
                ..
            }) if self.d.space => Some(canvas::Action::publish(Message::Space(false))),
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(p) = cursor.position_in(bounds) else {
                    if state.inside && !state.pressed {
                        state.inside = false;
                        return Some(canvas::Action::publish(Message::Stage(StageEvent::Leave)));
                    }
                    return None;
                };
                state.inside = true;
                if let Some(last) = state.panning {
                    state.panning = Some(p);
                    return Some(canvas::Action::publish(Message::Stage(StageEvent::Pan {
                        dx: p.x - last.x,
                        dy: p.y - last.y,
                        size,
                    })));
                }
                // The hand tool drags the paper with the left button.
                if state.pressed && matches!(self.d.drag, Some(Drag::Pan)) {
                    let last = state.last_click.map_or(p, |c| c.position());
                    state.last_click = Some(iced::advanced::mouse::Click::new(
                        p,
                        mouse::Button::Left,
                        None,
                    ));
                    return Some(canvas::Action::publish(Message::Stage(StageEvent::Pan {
                        dx: p.x - last.x,
                        dy: p.y - last.y,
                        size,
                    })));
                }
                Some(canvas::Action::publish(Message::Stage(StageEvent::Move {
                    at: paper(p),
                    size,
                    shift: m.shift(),
                    ctrl: m.command(),
                    alt: m.alt(),
                })))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                state.pressed = true;
                let click =
                    iced::advanced::mouse::Click::new(p, mouse::Button::Left, state.last_click);
                state.last_click = Some(click);
                let message = if click.kind() == iced::advanced::mouse::click::Kind::Double {
                    Message::Stage(StageEvent::DoubleClick { at: paper(p) })
                } else {
                    Message::Stage(StageEvent::Press {
                        at: paper(p),
                        px: p,
                        size,
                        shift: m.shift(),
                        ctrl: m.command(),
                        alt: m.alt(),
                    })
                };
                Some(canvas::Action::publish(message).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.pressed => {
                state.pressed = false;
                let p = cursor.position_in(bounds).or_else(|| {
                    cursor
                        .position()
                        .map(|q| Point::new(q.x - bounds.x, q.y - bounds.y))
                })?;
                Some(canvas::Action::publish(Message::Stage(
                    StageEvent::Release {
                        at: paper(p),
                        shift: m.shift(),
                        ctrl: m.command(),
                        alt: m.alt(),
                    },
                )))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                let p = cursor.position_in(bounds)?;
                state.panning = Some(p);
                Some(canvas::Action::capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                state.panning = None;
                None
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let p = cursor.position_in(bounds)?;
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => f64::from(*y),
                    mouse::ScrollDelta::Pixels { y, .. } => f64::from(*y) / 40.0,
                };
                if lines == 0.0 {
                    return None;
                }
                Some(
                    canvas::Action::publish(Message::Stage(StageEvent::Zoom {
                        factor: 1.15_f64.powf(lines),
                        px: p,
                        size,
                    }))
                    .and_capture(),
                )
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _: &OverlayState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        self.draw_into(&mut f, theme);
        vec![f.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &OverlayState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if !cursor.is_over(bounds) {
            return mouse::Interaction::default();
        }
        if state.panning.is_some() || matches!(self.d.drag, Some(Drag::Pan)) {
            return mouse::Interaction::Grabbing;
        }
        if self.d.space {
            return mouse::Interaction::Grab;
        }
        match (&self.d.tool, &self.d.drag) {
            (Tool::Hand, _) => mouse::Interaction::Grab,
            (Tool::Add { .. }, _) => mouse::Interaction::Crosshair,
            (_, Some(Drag::Move { .. })) => mouse::Interaction::Grabbing,
            (_, Some(Drag::Resize { .. } | Drag::Rotate { .. })) => mouse::Interaction::Crosshair,
            _ if self.d.hovered_item.is_some() => mouse::Interaction::Pointer,
            _ => mouse::Interaction::default(),
        }
    }
}

/// The paper's layers and its maps and pictures between them, in the drawing order: each map on
/// a canvas of its own the size of its box, clipped to it; each picture on a canvas of its own
/// (a renderer draws a canvas's pictures over its shapes).
pub(crate) fn paper_layers<'a, M: 'a>(
    d: &'a Designer,
    list: &'a DisplayList,
    plan: &'a paint::Plan,
    sheet: &'a Sheet,
    xf: Xf,
    painter: &Painter<'a>,
    export: bool,
) -> Vec<Element<'a, M>> {
    let mut layers: Vec<Element<'a, M>> = Vec::new();
    for (i, entries) in plan.layers.iter().enumerate() {
        layers.push(
            canvas_widget(PaperLayer {
                d,
                list,
                entries,
                first: i == 0,
                xf,
                cache: if export { None } else { d.layer_caches.get(i) },
                sheet,
                aids: !export,
            })
            .width(Fill)
            .height(Fill)
            .into(),
        );
        if let Some(e) = plan.maps.get(i)
            && let Some(Prim::Image(_)) = list.prims.get(e.prim)
        {
            layers.push(
                canvas_widget(PaperLayer {
                    d,
                    list,
                    entries: std::slice::from_ref(e),
                    first: false,
                    xf,
                    cache: None,
                    sheet,
                    aids: !export,
                })
                .width(Fill)
                .height(Fill)
                .into(),
            );
        }
        if let Some(Prim::Map(mp)) = plan.maps.get(i).and_then(|e| list.prims.get(e.prim)) {
            let unturned = xf.rect(&mp.clip);
            let around = xf.rect(&rotated_bounds(&mp.clip, mp.rotation));
            if around.width >= 1.0 && around.height >= 1.0 {
                let middle = Point::new(
                    unturned.center_x() - around.x,
                    unturned.center_y() - around.y,
                );
                let map = canvas_widget(MapLayer {
                    prim: mp,
                    painter: painter.clone(),
                    cache: if export {
                        None
                    } else {
                        d.map_caches.get(&mp.item)
                    },
                    k: xf.k,
                    size: unturned.size(),
                    middle,
                    export,
                })
                .width(around.width)
                .height(around.height);
                layers.push(
                    pin(container(map).clip(true))
                        .x(around.x)
                        .y(around.y)
                        .into(),
                );
            }
        }
    }
    layers
}

impl Designer {
    /// The stage: rulers, the paper's layers and maps, the pointer's layer.
    pub fn stage<'a>(&'a self, painter: Painter<'a>) -> Element<'a, Message> {
        let Some(sheet) = self.open_sheet() else {
            return container(kentos_ui::label::muted("Önde bir pafta yok."))
                .center(Fill)
                .into();
        };
        responsive(move |size| {
            let ruler = rulers::thickness();
            let content = Size::new(
                (size.width - ruler).max(1.0),
                (size.height - ruler).max(1.0),
            );
            let xf = self.view.xf(content, sheet.page.size);
            let (list, plan) = self.shown();
            let mut layers: Vec<Element<'a, Message>> = match list {
                Some(list) => paper_layers(self, list, plan, sheet, xf, &painter, false),
                None => Vec::new(),
            };
            layers.push(
                canvas_widget(Overlay {
                    d: self,
                    xf,
                    size: content,
                })
                .width(Fill)
                .height(Fill)
                .into(),
            );
            let desk = container(Stack::with_children(layers).width(Fill).height(Fill))
                .width(Fill)
                .height(Fill)
                .clip(true)
                .style(kentos_ui::style::container::surface_alt);
            Rulers::new(desk, Transform::new(xf.origin, (xf.k * 1000.0) as f32))
                .unit("mm")
                .guides(&self.guides, Message::Guide)
                .into()
        })
        .into()
    }
}
