//! Görünümü böl (docs/adr/0285 §3): the drawing area split into two model
//! views side by side, each with a camera of its own. One is active: it has
//! the pointer, the running tool, the snap, the marks and the cards, and the
//! commands that move the view move it; the other shows the drawing and its
//! text. A press on the other one makes it the active one first (the press
//! then goes on to it), so a tool runs on in the view clicked; the wheel and
//! the middle button move the other view without making it active.
//!
//! The active view is always `App::viewport` (every part of the app that
//! reads the camera reads it); the other waits in `Split`, and a press swaps
//! them. Each keeps its id, so the renderer keeps each one's pictures.
//! Opening another drawing goes back to one view.

use iced::widget::{container, row, stack};
use iced::{Element, Fill, Task};
use kentos_interaction::{Cursor, Level};

use crate::app::{App, Message};
use crate::viewport::{Event, Viewport};

/// The second view and where it stands.
pub(crate) struct Split {
    /// The view that is not active now.
    pub(crate) viewport: Viewport,
    /// Its drawing's text as last placed.
    spots: crate::labels::Spots,
    /// Whether it stands on the left (the active view on the right).
    pub(crate) left: bool,
    /// The drawing it was split for: another drawing goes back to one view.
    session: u64,
}

impl App {
    /// Görünümü böl (`view.split`): two views side by side, or one again.
    pub(crate) fn toggle_split(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if self.split.take().is_some() {
            self.say(Level::Info, "Çizim alanı tek görünüm.");
            return;
        }
        let mut viewport = Viewport::new();
        viewport.camera = self.viewport.camera;
        viewport.grid_shown = self.viewport.grid_shown;
        viewport.other = true;
        self.split = Some(Split {
            viewport,
            spots: crate::labels::Spots::default(),
            left: false,
            session: doc.session,
        });
        self.say(
            Level::Info,
            "Çizim alanı iki görünüme bölündü: her birinin kendi kamerası var, tıklanan etkin olur.",
        );
    }

    /// Another drawing goes back to one view (after every message).
    pub(crate) fn follow_split(&mut self) {
        let session = self.document.as_ref().map(|d| d.session);
        if self
            .split
            .as_ref()
            .is_some_and(|s| Some(s.session) != session)
        {
            self.split = None;
        }
    }

    /// What the other view reports: its size and the wheel and middle button move it; a press makes it the
    /// active view and goes on to it.
    pub(crate) fn other_viewport(&mut self, event: Event) -> Task<Message> {
        let Some(split) = self.split.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Resized(_)
            | Event::Zoomed { .. }
            | Event::Panned { .. }
            | Event::Extents
            | Event::Settled => {
                split.viewport.update(event, self.document.as_ref());
                Task::none()
            }
            Event::Pressed(_) | Event::RightPressed(_) => {
                self.activate_other();
                self.pointer(event)
            }
            _ => Task::none(),
        }
    }

    /// The other view becomes the active one: the two swap, each with its camera, its pictures and its text.
    pub(crate) fn activate_other(&mut self) {
        let Some(split) = self.split.as_mut() else {
            return;
        };
        std::mem::swap(&mut self.viewport, &mut split.viewport);
        std::mem::swap(&mut self.label_spots, &mut split.spots);
        self.viewport.other = false;
        split.viewport.other = true;
        split.viewport.cursor = None;
        split.left = !split.left;
        // The marks of the pointer were the other view's.
        self.snap = None;
        self.panning = false;
    }

    /// The drawing area with the other view beside the active one, the active one outlined; the active one alone
    /// when the area is not split.
    pub(crate) fn with_split<'a>(&'a self, active: Element<'a, Message>) -> Element<'a, Message> {
        let (Some(split), Some(doc)) = (&self.split, &self.document) else {
            return active;
        };
        let format = kentos_interaction::Format::of(doc.settings());
        let accent = crate::viewport::highlight_color(
            &self.settings.text("graphics.highlightColor"),
            crate::view::rgba8(kentos_ui::theme::Tokens::of(&self.theme()).accent),
        );
        let spots = split.spots.last();
        let area = split.viewport.view(
            doc,
            self.canvas(),
            self.graphics(),
            &self.selection,
            accent,
            Cursor::Pick,
            Some(crate::viewport::Styling {
                labels: spots.as_deref().map(Vec::as_slice),
                ..self.styles.styling(&self.spatial, &self.settings)
            }),
        );
        let labels = crate::labels::layer(
            &doc.model,
            doc.session,
            &self.spatial,
            &split.spots,
            &split.viewport.camera,
            self.canvas(),
            &crate::viewport::palette(self.canvas()),
            &format,
            None,
            None,
            self.color_mode(),
            self.label_size(),
            0.0,
            crate::labels::Engine::default(),
        );
        let other: Element<'a, Message> = container(stack![area, labels])
            .padding(1)
            .style(|theme: &iced::Theme| container::Style {
                border: iced::Border {
                    color: kentos_ui::theme::Tokens::of(theme).border,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            })
            .width(Fill)
            .height(Fill)
            .into();
        let active: Element<'a, Message> = container(active)
            .padding(1)
            .style(|theme: &iced::Theme| container::Style {
                border: iced::Border {
                    color: kentos_ui::theme::Tokens::of(theme).accent,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            })
            .width(Fill)
            .height(Fill)
            .into();
        if split.left {
            row![other, active].spacing(2).into()
        } else {
            row![active, other].spacing(2).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Rectangle, Size};

    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;
    use crate::viewport::Event;

    fn open() -> App {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(
            Point::ORIGIN,
            Size::new(500.0, 800.0),
        ))));
        app
    }

    /// Two views with cameras of their own; a press on the other makes it active and the tool runs on in it.
    #[test]
    fn the_other_view_becomes_active_on_a_press_and_the_tool_runs_on_in_it() {
        let mut app = open();
        let _ = app.update(Message::Run("view.split"));
        assert!(app.split.is_some());
        let _ = app.update(Message::OtherViewport(Event::Resized(Rectangle::new(
            Point::new(502.0, 0.0),
            Size::new(500.0, 800.0),
        ))));
        // The wheel over the other view zooms it alone.
        let main = app.viewport.camera;
        let _ = app.update(Message::OtherViewport(Event::Zoomed {
            factor: 2.0,
            at: Point::new(250.0, 400.0),
        }));
        assert_eq!(app.viewport.camera, main, "the active view stays");
        let other = app.split.as_ref().expect("split").viewport.camera;
        assert!((other.scale / main.scale - 2.0).abs() < 1e-9);
        // A polyline with a corner in the active view, the second corner in the other.
        let _ = app.update(Message::Run("tool.polyline"));
        let _ = app.update(Message::Viewport(Event::Moved(Point::new(100.0, 100.0))));
        let _ = app.update(Message::Viewport(Event::Pressed(Point::new(100.0, 100.0))));
        let _ = app.update(Message::Viewport(Event::Released(Point::new(100.0, 100.0))));
        assert_eq!(app.session.point_count(), 1);
        let at = Point::new(300.0, 300.0);
        let world = app.split.as_ref().expect("split").viewport.world(at);
        let _ = app.update(Message::OtherViewport(Event::Pressed(at)));
        let _ = app.update(Message::Viewport(Event::Released(at)));
        // The other view is the active one now, with its own camera; the corner is where it was clicked in it.
        assert!((app.viewport.camera.scale / main.scale - 2.0).abs() < 1e-9);
        assert!(app.split.as_ref().is_some_and(|s| s.left));
        assert_eq!(app.session.point_count(), 2, "the tool ran on");
        assert_eq!(app.session.tool_id(), "polyline");
        let preview = app.session.preview(&app.format()).expect("a preview");
        let corner = preview.path[1];
        assert!((corner.x - world.x).abs() < 1e-6 && (corner.y - world.y).abs() < 1e-6);
        // One view again.
        let _ = app.update(Message::Run("view.split"));
        assert!(app.split.is_none());
    }
}
