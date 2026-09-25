//! Model alanının canvas programı: girdi işleme ve çizim sırası.

use iced::keyboard::{self, Modifiers};
use iced::widget::canvas::{self, Frame, Geometry};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use super::{Backdrop, CHROME_GAP, Event, OVERLAY_PADDING, Options, Style};
use crate::spatial::{Bounds, FeatureRef, Layer, LonLat, Selection, Tool, Viewport, query};
use crate::widget::navigation_bar;

/// Basılan fare tuşunun sürükleme sayılması için gereken mesafe (piksel).
const DRAG_THRESHOLD: f32 = 4.0;

/// Tekerleğin bir adımındaki yakınlaştırma.
const WHEEL_ZOOM: f64 = 0.35;

/// Sağ üstteki bileşenlerin kapladığı alan. Bu bölgede artı imleç çizilmez
/// ve sistem imleci gösterilir; ViewCube bölgesinde olaylar alta geçmez.
#[derive(Debug, Clone, Copy)]
pub(super) struct Chrome {
    /// ViewCube'un kenar uzunluğu.
    pub view_cube: Option<f32>,
    /// Gezinme çubuğunun yüksekliği.
    pub navigation: Option<f32>,
}

impl Chrome {
    pub fn column_width(&self) -> f32 {
        self.view_cube
            .unwrap_or(0.0)
            .max(if self.navigation.is_some() {
                navigation_bar::WIDTH
            } else {
                0.0
            })
    }

    fn view_cube(&self, size: Size) -> Option<Rectangle> {
        let side = self.view_cube?;
        let center = size.width - OVERLAY_PADDING - self.column_width() / 2.0;

        Some(Rectangle {
            x: center - side / 2.0,
            y: OVERLAY_PADDING,
            width: side,
            height: side,
        })
    }

    fn navigation(&self, size: Size) -> Option<Rectangle> {
        let height = self.navigation?;
        let center = size.width - OVERLAY_PADDING - self.column_width() / 2.0;
        let top = match self.view_cube {
            Some(side) => OVERLAY_PADDING + side + CHROME_GAP,
            None => OVERLAY_PADDING,
        };

        Some(Rectangle {
            x: center - navigation_bar::WIDTH / 2.0,
            y: top,
            width: navigation_bar::WIDTH,
            height,
        })
    }

    /// ViewCube shader'ı olayları yakalamaz; altındaki model alanı bu
    /// bölgedeki tıklamaları yok saymalıdır.
    pub fn is_over_view_cube(&self, size: Size, point: Point) -> bool {
        self.view_cube(size)
            .is_some_and(|rect| rect.contains(point))
    }

    pub fn contains(&self, size: Size, point: Point) -> bool {
        self.is_over_view_cube(size, point)
            || self
                .navigation(size)
                .is_some_and(|rect| rect.contains(point))
    }
}

/// Canvas programı.
pub(super) struct Program<'a, Message> {
    pub viewport: Viewport,
    pub layers: &'a [Layer],
    pub tool: Tool,
    pub selection: Option<&'a Selection>,
    pub hover: Option<FeatureRef>,
    pub measurement: &'a [LonLat],
    pub draft: &'a [LonLat],
    pub draft_color: Option<Color>,
    pub options: Options,
    pub prompt: Option<&'a str>,
    pub chrome: Chrome,
    pub backdrop: Backdrop,
    pub on_event: Box<dyn Fn(Event) -> Message + 'a>,
}

/// Programın çizimler arasında koruduğu etkileşim durumu.
#[derive(Debug, Default)]
pub(super) struct State {
    size: Option<Size>,
    /// Sürükleme sürüyorsa son imleç konumu.
    drag: Option<Point>,
    /// Sol tuşun basıldığı konum.
    press: Option<Point>,
    /// Basılı tuş sürükleme eşiğini aştı mı.
    moved: bool,
    /// İmleç en son alanın üzerinde miydi; dışarı çıkışta bir kez
    /// `CursorLeft` bildirilir.
    inside: bool,
    /// Sürüklenen seçim penceresi: başlangıç ve o anki köşe.
    window: Option<(Point, Point)>,
    /// Basılı değiştirici tuşlar (Shift, Ctrl).
    modifiers: Modifiers,
}

impl State {
    /// Sürüklenen seçim penceresi ve kesişen seçim olup olmadığı.
    pub fn selection_window(&self) -> Option<(Rectangle, bool)> {
        let (start, end) = self.window?;

        Some((
            Rectangle {
                x: start.x.min(end.x),
                y: start.y.min(end.y),
                width: (start.x - end.x).abs(),
                height: (start.y - end.y).abs(),
            },
            end.x < start.x,
        ))
    }
}

impl<Message> Program<'_, Message> {
    fn publish(&self, event: Event) -> canvas::Action<Message> {
        canvas::Action::publish((self.on_event)(event))
    }

    /// Nokta girişi alan araçlarda yakalama açıksa en yakın köşe.
    pub(super) fn snap_at(&self, position: Point) -> Option<query::Snap> {
        (self.tool.takes_points() && self.options.snap)
            .then(|| query::snap(self.layers, &self.viewport, position, query::SNAP_TOLERANCE))
            .flatten()
    }
}

impl<Message> canvas::Program<Message> for Program<'_, Message> {
    type State = State;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let size = bounds.size();

        if state.size != Some(size) {
            state.size = Some(size);

            return Some(self.publish(Event::Resized(size)));
        }

        let event = match event {
            canvas::Event::Mouse(event) => event,
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                return None;
            }
            _ => return None,
        };

        match event {
            mouse::Event::CursorMoved { position } => {
                // Sürükleme sürerken imleç alanın dışına çıksa da gezinme
                // devam eder. Aksi hâlde yalnızca alanın üzerindeki ve üstünde
                // başka bir katman olmayan konumlar dikkate alınır.
                let dragging = state.drag.is_some() || state.window.is_some();

                let position = match (dragging, cursor.position_in(bounds)) {
                    (true, _) => *position - Vector::new(bounds.x, bounds.y),
                    (false, Some(position)) => position,
                    (false, None) => {
                        return state.inside.then(|| {
                            state.inside = false;
                            self.publish(Event::CursorLeft)
                        });
                    }
                };

                state.inside = true;

                if !dragging && self.chrome.is_over_view_cube(size, position) {
                    return None;
                }

                let location = self.viewport.unproject(position);

                if let Some(last) = state.drag {
                    state.drag = Some(position);

                    return Some(self.publish(Event::Panned {
                        delta: position - last,
                        cursor: location,
                    }));
                }

                if let Some((start, _)) = state.window {
                    state.window = Some((start, position));
                } else if let Some(pressed) = state.press
                    && position.distance(pressed) > DRAG_THRESHOLD
                {
                    state.moved = true;

                    // Seç aracında sol tuşla sürüklemek seçim penceresi açar;
                    // diğer araçlarda görünümü kaydırır.
                    if self.tool == Tool::Select {
                        state.window = Some((pressed, position));
                    } else {
                        state.drag = Some(position);
                    }
                }

                Some(self.publish(Event::CursorMoved(location)))
            }
            mouse::Event::ButtonPressed(button) => {
                let position = cursor.position_in(bounds)?;

                if self.chrome.is_over_view_cube(size, position) {
                    return None;
                }

                match button {
                    mouse::Button::Left => {
                        state.press = Some(position);
                        state.moved = self.tool == Tool::Pan;
                        state.drag = (self.tool == Tool::Pan).then_some(position);

                        Some(canvas::Action::capture())
                    }
                    mouse::Button::Middle => {
                        state.press = Some(position);
                        state.drag = Some(position);
                        state.moved = true;

                        Some(canvas::Action::capture())
                    }
                    mouse::Button::Right if self.tool.takes_points() => {
                        Some(self.publish(Event::Finished).and_capture())
                    }
                    _ => None,
                }
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let position = cursor.position_in(bounds).unwrap_or_default();
                let was_dragging = state.moved;
                let pressed = state.press.take();

                state.drag = None;
                state.moved = false;

                if let Some((start, end)) = state.window.take() {
                    let corners = [self.viewport.unproject(start), self.viewport.unproject(end)];

                    return Bounds::from_points(corners).map(|bounds| {
                        self.publish(Event::BoxSelected {
                            bounds,
                            crossing: end.x < start.x,
                            modifiers: state.modifiers,
                        })
                    });
                }

                if was_dragging {
                    return None;
                }

                // Tuş basıldığı yerde bırakılmadıysa tıklama sayılmaz.
                pressed.filter(|pressed| pressed.distance(position) <= DRAG_THRESHOLD)?;

                let location = self
                    .snap_at(position)
                    .map_or_else(|| self.viewport.unproject(position), |snap| snap.location);

                Some(self.publish(if self.tool.takes_points() {
                    Event::PointPicked(location)
                } else {
                    Event::Clicked {
                        location,
                        modifiers: state.modifiers,
                    }
                }))
            }
            mouse::Event::ButtonReleased(mouse::Button::Middle) => {
                state.drag = None;
                state.press = None;
                state.moved = false;

                None
            }
            mouse::Event::WheelScrolled { delta } => {
                // Tekerlek yalnızca imleç model alanının üzerindeyken
                // yakınlaştırır; şerit, paneller ve tablolar kendi içlerinde
                // kaydırılır.
                let position = cursor.position_in(bounds)?;

                if self.chrome.is_over_view_cube(size, position) {
                    return None;
                }

                let steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };

                if steps.abs() < f32::EPSILON {
                    return None;
                }

                Some(
                    self.publish(Event::Zoomed {
                        delta: f64::from(steps) * WHEEL_ZOOM,
                        anchor: position,
                    })
                    .and_capture(),
                )
            }
            mouse::Event::CursorLeft => {
                state.inside = false;
                Some(self.publish(Event::CursorLeft))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let style = Style::with(self.backdrop, theme);
        let mut frame = Frame::new(renderer, bounds.size());

        // Artı imleç ve önizlemeler yalnızca imleç alanın üzerindeyken ve
        // sağ üstteki bileşenlerin dışındayken çizilir.
        let pointer = cursor
            .position_in(bounds)
            .filter(|position| !self.chrome.contains(bounds.size(), *position));

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), style.background);

        if self.options.grid {
            self.draw_grid(&mut frame, &style);
        }

        self.draw_layers(&mut frame, &style);

        if self.options.labels {
            self.draw_labels(&mut frame, &style);
        }

        self.draw_measurement(&mut frame, &style, pointer);
        self.draw_draft(&mut frame, &style, pointer);

        if let Some((window, crossing)) = state.selection_window() {
            self.draw_selection_window(&mut frame, &style, window, crossing);
        }

        if let Some(pointer) = pointer {
            self.draw_crosshair(&mut frame, &style, pointer);
        }

        self.draw_ucs(&mut frame, &style);
        self.draw_scale_bar(&mut frame, &style);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }

        let Some(position) = cursor.position_in(bounds) else {
            return mouse::Interaction::None;
        };

        if self.chrome.contains(bounds.size(), position) {
            return mouse::Interaction::Idle;
        }

        // Sistem imleci gizlenir; yerine artı imleç çizilir. Kaydır aracı
        // el gösterir.
        match self.tool {
            Tool::Pan => mouse::Interaction::Grab,
            _ => mouse::Interaction::Hidden,
        }
    }
}
