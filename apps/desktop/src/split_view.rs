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
//!
//! Bağlı görünümler and Kaydırmalı karşılaştır (docs/adr/0523): each view may
//! show the drawing with a theme, one of the project's layer states, the
//! drawing untouched (its visibility, freeze and styles on a copy of the tree;
//! the active view's theme is also what is picked and snapped to). Bağla makes
//! the other view follow the active one's centre, scale and twist; Kaydırmalı
//! karşılaştır draws the other view over the active one right of a divider
//! dragged across, at the same place, the pointer going through it.
//!
//! Görünüm başına zaman, bağ ve revizyon (docs/adr/0576): each view may show its own time (a moment of the slider's
//! grid, or the slider's own window), Bağla follows the centre alone or the centre, scale and twist (or nothing), each
//! view says its screen scale, and the other view may show another state of the drawing (`split_revision.rs`).

use iced::widget::{Space, button, column, container, mouse_area, row, stack};
use iced::{Center, Element, Fill, Length, Point, Task};
use kentos_geometry_core::time::{self, Window};
use kentos_geometry_core::tools::view_link::{self, Frame, Link};
use kentos_geometry_core::vec2::Vec2 as Vec2d;
use kentos_interaction::{Cursor, Level};
use kentos_ui::theme::Tokens;
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::split_revision::Revision;
use crate::viewport::{Event as ViewEvent, Viewport};

/// The time a view shows (docs/adr/0576 §1): the time slider's window, or a moment of its own: the slider's grid
/// position nearest it (the slider's step and its Anlık or Aralık), while the slider is open.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum ViewTime {
    #[default]
    Follow,
    Own(f64),
}

/// What a view's time control asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeAsk {
    /// Kendi zamanı: the slider's moment now, then the view goes its own way.
    Own,
    /// ◀ and ▶: a step of the slider's.
    Step(i64),
    /// Sürgüyü izle.
    Follow,
}

/// The source the other view shows (docs/adr/0576 §3), by its place in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The open drawing.
    Drawing,
    /// The open drawing this many undo steps back.
    History(usize),
    /// A drawing file, chosen.
    File,
}

/// The width of a view's choosers, logical pixels.
const CHOOSER: f32 = 200.0;

/// A small pill over a view: a control or a fact, in the surface's colour.
fn chip<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .padding([2, 8])
        .style(|theme: &iced::Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(iced::Background::Color(t.surface)),
                text_color: Some(t.text),
                border: iced::Border {
                    color: t.border,
                    width: 1.0,
                    radius: 999.0.into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// A camera as the link's rules take it.
fn frame_of(c: &kentos_render_wgpu::Camera) -> Frame {
    Frame {
        centre: Vec2d::new(c.center.x, c.center.y),
        scale: c.scale,
        twist: c.twist,
    }
}

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
    /// Bağla: what the other view follows of the active one (docs/adr/0523 §3, 0576 §2).
    pub(crate) link: Link,
    /// The other state of the drawing the other view shows, read only (docs/adr/0576 §3); none, the open drawing.
    pub(crate) revision: Option<Revision>,
    /// Kaydırmalı karşılaştır: where the divider stands, a share of the active view's width from its left;
    /// none, side by side.
    pub(crate) swipe: Option<f32>,
    /// The divider is being dragged.
    pub(crate) dragging: bool,
}

impl Split {
    /// The other view's text is placed again (what it shows changed).
    pub(crate) fn reset_text(&mut self) {
        self.spots = crate::labels::Spots::default();
    }
}

/// What the views' own controls ask (docs/adr/0523).
#[derive(Debug, Clone)]
pub enum Event {
    /// A view's time chosen (docs/adr/0576 §1): the active view's or the other's.
    Time { active: bool, ask: TimeAsk },
    /// What the other view follows (docs/adr/0576 §2).
    Link(Link),
    /// What the other view shows (docs/adr/0576 §3); a file is asked for, then `RevisionPicked`.
    Source(Source),
    RevisionPicked(Option<std::path::PathBuf>),
    /// A view's theme chosen: the active view's or the other's; the layer state's place in the project's list,
    /// none: as the drawing is.
    Theme {
        active: bool,
        state: Option<usize>,
    },
    /// The divider taken, moved to a share of the width, let go.
    Grab,
    Drag(Point),
    Release,
}

impl App {
    /// Görünümü böl (`view.split`): two views side by side, or one again.
    pub(crate) fn toggle_split(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if self.split.take().is_some() {
            self.viewport.theme = None;
            self.viewport.passive = false;
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
            link: Link::Off,
            revision: None,
            swipe: None,
            dragging: false,
        });
        self.say(
            Level::Info,
            "Çizim alanı iki görünüme bölündü: her birinin kendi kamerası var, tıklanan etkin olur.",
        );
    }

    /// Another drawing goes back to one view; a linked or compared view follows the active one; what is picked
    /// follows the active view's theme (after every message).
    pub(crate) fn follow_split(&mut self) {
        let session = self.document.as_ref().map(|d| d.session);
        if self
            .split
            .as_ref()
            .is_some_and(|s| Some(s.session) != session)
        {
            self.split = None;
            self.viewport.theme = None;
            self.viewport.passive = false;
        }
        // A theme the project no longer has is let go.
        if let Some(doc) = &self.document {
            let states = &doc.settings().layer_states;
            let gone = |t: &Option<kentos_commands::LayerState>| {
                t.as_ref()
                    .is_some_and(|t| !states.iter().any(|s| s.id == t.id))
            };
            if gone(&self.viewport.theme) {
                self.viewport.theme = None;
            }
            if let Some(split) = self.split.as_mut()
                && gone(&split.viewport.theme)
            {
                split.viewport.theme = None;
            }
        }
        if let Some(split) = self.split.as_mut() {
            let active = self.viewport.camera;
            let other = &mut split.viewport;
            if split.swipe.is_some() {
                // The same world under the same place of the window: the other view's middle as the active one sees it.
                other.camera.scale = active.scale;
                other.camera.twist = active.twist;
                let b = other.bounds;
                let a = self.viewport.bounds;
                other.camera.center = self.viewport.world(Point::new(
                    b.x + b.width / 2.0 - a.x,
                    b.y + b.height / 2.0 - a.y,
                ));
            } else if split.link != Link::Off {
                let got = view_link::follow(split.link, frame_of(&active), frame_of(&other.camera));
                other.camera.center = kentos_render_wgpu::Vec2::new(got.centre.x, got.centre.y);
                other.camera.scale = got.scale;
                other.camera.twist = got.twist;
            }
        }
        // A time of its own needs the slider: without it every view follows (the slider's window is none).
        if !self.time.slider.open {
            self.viewport.time = ViewTime::Follow;
            if let Some(split) = self.split.as_mut() {
                split.viewport.time = ViewTime::Follow;
            }
        } else if self.split.is_none() {
            self.viewport.time = ViewTime::Follow;
        }
        // The marks of a revision follow the open drawing's edits (split_revision.rs).
        self.follow_revision();
        // What is picked and snapped to is what the active view shows, at its time too (docs/adr/0576 §1).
        let want = self.view_window(self.viewport.time);
        if self.spatial.store().time_window() != want {
            self.spatial.set_time_window(want);
        }
        if let Some(doc) = &self.document
            && self.spatial.theme() != self.viewport.theme.as_ref()
        {
            self.spatial
                .set_theme(&doc.model, self.viewport.theme.clone());
        }
    }

    /// Görünümleri bağla (`view.splitLink`): the other view follows the active one, or goes its own way again.
    pub(crate) fn toggle_split_link(&mut self) {
        if self.split.is_none() {
            self.toggle_split();
        }
        let Some(split) = self.split.as_mut() else {
            return;
        };
        split.link = if split.link == Link::Off {
            Link::CentreScale
        } else {
            Link::Off
        };
        let said = match split.link {
            Link::Off => "Görünümler bağımsız: her birinin kendi kamerası var.",
            Link::Centre => "Görünümler bağlı: öbür görünüm etkin görünümün merkezini izler, ölçeğini korur.",
            Link::CentreScale => {
                "Görünümler bağlı: öbür görünüm etkin görünümün merkezini, ölçeğini ve dönüklüğünü izler."
            }
        };
        self.say(Level::Info, said);
    }

    /// Kaydırmalı karşılaştır (`view.swipe`): the other view over the active one right of a divider, or side by
    /// side again.
    pub(crate) fn toggle_swipe(&mut self) {
        if self.split.is_none() {
            self.toggle_split();
        }
        let Some(split) = self.split.as_mut() else {
            return;
        };
        if split.swipe.take().is_some() {
            split.viewport.passive = false;
            split.dragging = false;
            self.say(Level::Info, "Görünümler yan yana.");
            return;
        }
        split.swipe = Some(0.5);
        split.viewport.passive = true;
        self.say(
            Level::Info,
            "Kaydırmalı karşılaştır: ayırıcının solunda etkin görünüm, sağında öbür görünüm aynı yerde. Ayırıcıyı sürükleyin.",
        );
    }

    /// What the views' controls ask.
    pub(crate) fn split_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Time { active, ask } => {
                let s = &self.time.slider;
                let (anchor, step, last, moment) = (s.anchor, s.step, s.last, s.moment());
                let view = if active {
                    Some(&mut self.viewport)
                } else {
                    self.split.as_mut().map(|s| &mut s.viewport)
                };
                if let Some(view) = view {
                    view.time = match (ask, view.time) {
                        (TimeAsk::Own, _) => ViewTime::Own(moment),
                        (TimeAsk::Step(by), ViewTime::Own(m)) => {
                            ViewTime::Own(view_link::own_step(anchor, step, last, m, by))
                        }
                        (TimeAsk::Step(_), ViewTime::Follow) | (TimeAsk::Follow, _) => ViewTime::Follow,
                    };
                }
                self.follow_split();
            }
            Event::Link(link) => {
                if let Some(split) = self.split.as_mut() {
                    split.link = link;
                }
                self.follow_split();
            }
            Event::Source(Source::Drawing) => self.hide_revision(),
            Event::Source(Source::History(n)) => self.revision_from_history(n),
            Event::Source(Source::File) => return self.pick_revision_file(),
            Event::RevisionPicked(path) => {
                if let Some(path) = path {
                    self.revision_from_file(&path);
                }
            }
            Event::Theme { active, state } => {
                let theme = state.and_then(|i| {
                    self.document
                        .as_ref()
                        .and_then(|d| d.settings().layer_states.get(i).cloned())
                });
                if active {
                    self.viewport.theme = theme;
                } else if let Some(split) = self.split.as_mut() {
                    split.viewport.theme = theme;
                }
                self.follow_split();
            }
            Event::Grab => {
                if let Some(split) = self.split.as_mut() {
                    split.dragging = true;
                }
            }
            Event::Drag(at) => {
                let width = self.viewport.bounds.width;
                if let Some(split) = self.split.as_mut()
                    && split.dragging
                    && width > 0.0
                {
                    split.swipe = Some((at.x / width).clamp(0.05, 0.95));
                }
            }
            Event::Release => {
                if let Some(split) = self.split.as_mut() {
                    split.dragging = false;
                }
            }
        }
        Task::none()
    }

    /// The theme chooser of a view (docs/adr/0523 §1): Çizimdeki gibi, then the project's layer states.
    fn theme_chooser<'a>(&'a self, active: bool) -> Element<'a, Message> {
        use kentos_ui::widget::select::{Choice, Select};
        let Some(doc) = &self.document else {
            return Space::new().into();
        };
        let states = &doc.settings().layer_states;
        let theme = if active {
            self.viewport.theme.as_ref()
        } else {
            self.split.as_ref().and_then(|s| s.viewport.theme.as_ref())
        };
        let at = theme
            .and_then(|t| states.iter().position(|s| s.id == t.id))
            .map_or(0, |i| i + 1);
        let choices = std::iter::once(Choice::new("Çizimdeki gibi".to_owned())).chain(
            states
                .iter()
                .map(|s| Choice::new(format!("Tema: {}", s.name))),
        );
        let pick = Select::new(choices, Some(at), move |i| {
            Message::Split(Event::Theme {
                active,
                state: i.checked_sub(1),
            })
        });
        container(pick).width(Length::Fixed(CHOOSER)).into()
    }

    /// What a view with this time shows now (docs/adr/0576 §1): its own moment's position of the slider's grid, or the
    /// slider's window (none while the slider is closed).
    pub(crate) fn view_window(&self, time: ViewTime) -> Option<Window> {
        let s = &self.time.slider;
        match time {
            ViewTime::Own(moment) if s.open => Some(s.window_near(moment)),
            _ => s.window(),
        }
    }

    /// A view's time control (docs/adr/0576 §1): while the slider is open, Kendi zamanı, or ◀ the moment ▶ and
    /// Sürgüyü izle.
    fn time_chip<'a>(&'a self, active: bool) -> Option<Element<'a, Message>> {
        let slider = &self.time.slider;
        if !slider.open {
            return None;
        }
        let time = if active {
            self.viewport.time
        } else {
            self.split.as_ref()?.viewport.time
        };
        let ask = move |ask: TimeAsk| Message::Split(Event::Time { active, ask });
        let small = move |words: &'static str, a: TimeAsk| {
            button(label::caption(words))
                .on_press(ask(a))
                .padding([2, 6])
                .style(style::button::flat)
        };
        let content: Element<'a, Message> = match time {
            ViewTime::Follow => row![
                label::caption("Zaman: sürgü"),
                small("Kendi zamanı", TimeAsk::Own),
            ]
            .spacing(4)
            .align_y(Center)
            .into(),
            ViewTime::Own(moment) => {
                let w = slider.window_near(moment);
                row![
                    small("◀", TimeAsk::Step(-1)),
                    label::caption(time::show_window(&w, slider.step.unit)),
                    small("▶", TimeAsk::Step(1)),
                    small("Sürgüyü izle", TimeAsk::Follow),
                ]
                .spacing(4)
                .align_y(Center)
                .into()
            }
        };
        Some(chip(content))
    }

    /// A view's screen scale, 1:N on a 96 dpi screen (docs/adr/0576 §2).
    fn scale_chip<'a>(&'a self, active: bool) -> Element<'a, Message> {
        let camera = if active {
            self.viewport.camera
        } else {
            match &self.split {
                Some(s) => s.viewport.camera,
                None => self.viewport.camera,
            }
        };
        let n = view_link::screen_scale(camera.scale).round();
        chip(label::caption(format!("Ölçek {}", kentos_project::wizard::scale_text(n))).into())
    }

    /// What the other view follows of the active one (docs/adr/0576 §2).
    fn link_chooser<'a>(&'a self) -> Element<'a, Message> {
        use kentos_ui::widget::select::{Choice, Select};
        let links = [Link::Off, Link::Centre, Link::CentreScale];
        let at = self
            .split
            .as_ref()
            .and_then(|s| links.iter().position(|l| *l == s.link));
        let choices = [
            Choice::new("Bağ: yok".to_owned()),
            Choice::new("Bağ: merkez".to_owned()),
            Choice::new("Bağ: merkez ve ölçek".to_owned()),
        ];
        let pick = Select::new(choices, at, move |i| Message::Split(Event::Link(links[i])));
        container(pick).width(Length::Fixed(CHOOSER)).into()
    }

    /// What the other view shows (docs/adr/0576 §3): the open drawing, its history some steps back, a file.
    fn source_chooser<'a>(&'a self) -> Element<'a, Message> {
        use kentos_ui::widget::select::{Choice, Select};
        let Some(doc) = &self.document else {
            return Space::new().into();
        };
        let held = doc.model.undo_labels();
        let steps = held.len().min(crate::split_revision::MOST_STEPS);
        let mut sources = vec![Source::Drawing];
        let mut choices = vec![Choice::new("Açık çizim".to_owned())];
        for n in 1..=steps {
            sources.push(Source::History(n));
            choices.push(Choice::new(format!(
                "Geçmiş: {n} adım geri, “{}” öncesi",
                held[n - 1]
            )));
        }
        sources.push(Source::File);
        choices.push(Choice::new("Dosyadan…".to_owned()));
        let at = match self.split.as_ref().and_then(|s| s.revision.as_ref()) {
            None => Some(0),
            Some(r) => sources.iter().position(|x| *x == r.source),
        };
        let pick = Select::new(choices, at, move |i| Message::Split(Event::Source(sources[i])));
        container(pick).width(Length::Fixed(CHOOSER)).into()
    }

    /// A view's controls at its top left (docs/adr/0523 §1, 0576): the theme and the scale, the time, and for the
    /// other view what it shows, what it follows and what it differs in.
    fn strip<'a>(&'a self, active: bool) -> Element<'a, Message> {
        let revision = if active {
            None
        } else {
            self.split.as_ref().and_then(|s| s.revision.as_ref())
        };
        let mut col = column![].spacing(4).padding(8);
        if active {
            col = col.push(
                row![self.theme_chooser(true), self.scale_chip(true)]
                    .spacing(4)
                    .align_y(Center),
            );
        } else {
            col = col.push(
                row![self.source_chooser(), self.scale_chip(false)]
                    .spacing(4)
                    .align_y(Center),
            );
            if revision.is_none() {
                col = col.push(self.theme_chooser(false));
            }
        }
        if let Some(time) = self.time_chip(active) {
            col = col.push(time);
        }
        if !active {
            col = col.push(self.link_chooser());
        }
        if let Some(r) = revision {
            let c = r.counts;
            col = col.push(chip(
                label::caption(format!(
                    "{} · {} eklendi · {} silindi · {} değişti",
                    r.name, c.added, c.removed, c.changed
                ))
                .into(),
            ));
        }
        col.into()
    }

    /// Kaydırmalı karşılaştır's part of the active area (view.rs puts it over the active view's text, under its
    /// marks): the other view right of the divider, its text, the divider; nothing when not compared.
    pub(crate) fn swipe_layer(&self) -> Option<Element<'_, Message>> {
        let split = self.split.as_ref()?;
        let share = split.swipe?;
        let doc = self.document.as_ref()?;
        let left = (share * 1000.0).round().max(1.0) as u16;
        let right = (1000 - left).max(1);
        let other = self.other_view(split, doc);
        let divider = mouse_area(
            container(Space::new().width(Length::Fixed(4.0)).height(Fill)).style(
                |theme: &iced::Theme| container::Style {
                    background: Some(kentos_ui::theme::Tokens::of(theme).accent.into()),
                    ..container::Style::default()
                },
            ),
        )
        .interaction(iced::mouse::Interaction::ResizingHorizontally)
        .on_press(Message::Split(Event::Grab))
        .on_release(Message::Split(Event::Release));
        let mut parts = stack![
            row![
                Space::new().width(Length::FillPortion(left)),
                divider,
                container(other)
                    .width(Length::FillPortion(right))
                    .height(Fill),
            ]
            .height(Fill)
        ];
        // While the divider is held the whole area follows the pointer.
        if split.dragging {
            parts = parts.push(
                mouse_area(Space::new().width(Fill).height(Fill))
                    .interaction(iced::mouse::Interaction::ResizingHorizontally)
                    .on_move(|p| Message::Split(Event::Drag(p)))
                    .on_release(Message::Split(Event::Release)),
            );
        }
        Some(parts.into())
    }

    /// The other view's drawing and text as it shows them (its camera, its theme, its time), or the revision it
    /// shows: its own drawing and store, its changed objects marked.
    fn other_view<'a>(
        &'a self,
        split: &'a Split,
        doc: &'a crate::document::Document,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let (shown, spatial, selection, mark) = match &split.revision {
            Some(r) => (&r.doc, &r.spatial, &r.marked, Tokens::of(&theme).warning),
            None => (doc, &self.spatial, &self.selection, Tokens::of(&theme).accent),
        };
        let accent = crate::viewport::highlight_color(
            &self.settings.text("graphics.highlightColor"),
            crate::view::rgba8(mark),
        );
        let accent = if split.revision.is_some() {
            crate::view::rgba8(mark)
        } else {
            accent
        };
        let format = kentos_interaction::Format::of(shown.settings());
        let window = self.view_window(split.viewport.time);
        spatial.store().with_window(window, || {
            let spots = split.spots.last();
            let area = split.viewport.view(
                shown,
                self.canvas(),
                self.graphics(),
                selection,
                accent,
                Cursor::Pick,
                Some(crate::viewport::Styling {
                    labels: spots.as_deref().map(Vec::as_slice),
                    ..self.styles.styling(spatial, &self.settings)
                }),
            );
            let tree = split.viewport.themed_tree(shown);
            let labels = crate::labels::layer(
                &shown.model,
                shown.session,
                spatial,
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
                tree.as_deref(),
            );
            stack![area, labels].into()
        })
    }

    /// What the other view reports: its size and the wheel and middle button move it; a press makes it the
    /// active view and goes on to it.
    pub(crate) fn other_viewport(&mut self, event: ViewEvent) -> Task<Message> {
        let Some(split) = self.split.as_mut() else {
            return Task::none();
        };
        match event {
            ViewEvent::Resized(_)
            | ViewEvent::Zoomed { .. }
            | ViewEvent::Panned { .. }
            | ViewEvent::Extents
            | ViewEvent::Settled => {
                split.viewport.update(event, self.document.as_ref());
                Task::none()
            }
            ViewEvent::Pressed(_) | ViewEvent::RightPressed(_) => {
                // A revision is read only: it never takes the pointer, the tool or the snap (docs/adr/0576 §3).
                if split.revision.is_some() {
                    return Task::none();
                }
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
        if split.revision.is_some() {
            return;
        }
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
    /// when the area is not split. Compared (Kaydırmalı karşılaştır) the active area holds the other view itself
    /// (`swipe_layer`); each view's theme chooser at its top.
    pub(crate) fn with_split<'a>(&'a self, active: Element<'a, Message>) -> Element<'a, Message> {
        let (Some(split), Some(doc)) = (&self.split, &self.document) else {
            return active;
        };
        if split.swipe.is_some() {
            return stack![
                active,
                row![self.strip(true), Space::new().width(Fill), self.strip(false)]
            ]
            .into();
        }
        let other: Element<'a, Message> = container(stack![self.other_view(split, doc), self.strip(false)])
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
        let active: Element<'a, Message> = container(stack![active, self.strip(true)])
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
    use kentos_geometry_core::tools::view_link::Link;

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

    /// A theme is a look (docs/adr/0523 §1): the other view shows the drawing with its layer state, the drawing
    /// untouched; the active view's theme is what is picked; Bağla keeps the other view on the active one's place.
    #[test]
    fn a_theme_shows_the_drawing_otherwise_and_leaves_it_as_it_is() {
        use crate::tools_scenes::{E, N, Objects, open as open_drawing};
        let mut app = open();
        let mut o = Objects::new();
        o.path(
            "parsel",
            &[[0.0, 0.0], [20.0, 0.0], [20.0, 16.0], [0.0, 16.0]],
            true,
        );
        o.line("yol", [-8.0, -6.0], [48.0, -6.0]);
        open_drawing(&mut app, o);
        crate::view_scenes::themes(&mut app);
        let before = app
            .document
            .as_ref()
            .expect("open")
            .model
            .layers()
            .nodes()
            .to_vec();
        let revision = app.document.as_ref().expect("open").model.revision();
        let _ = app.update(Message::Run("view.splitLink"));
        assert!(
            app.split.as_ref().is_some_and(|s| s.link == Link::CentreScale),
            "Bağla splits first"
        );
        let _ = app.update(Message::Split(super::Event::Theme {
            active: false,
            state: Some(1),
        }));
        let doc = app.document.as_ref().expect("open");
        let other = app
            .split
            .as_ref()
            .expect("split")
            .viewport
            .themed_tree(doc)
            .expect("themed");
        assert!(
            !other.is_visible("parsel"),
            "Yol hides the parcels in the other view"
        );
        assert!(other.is_visible("yol"));
        assert_eq!(
            doc.model.layers().nodes(),
            before.as_slice(),
            "the drawing untouched"
        );
        assert_eq!(doc.model.revision(), revision, "a theme is no edit");
        // The parcel is picked in the active view, which shows it; with Yol as the active view's theme, it is not.
        let inside = kentos_render_wgpu::Vec2::new(E + 0.0, N + 8.0);
        assert!(app.spatial.pick(inside, 0.5).is_some());
        let _ = app.update(Message::Split(super::Event::Theme {
            active: true,
            state: Some(1),
        }));
        assert!(
            app.spatial.pick(inside, 0.5).is_none(),
            "picked as the active view shows"
        );
        // Bağla: the other view at the active one's centre and scale.
        app.viewport.camera.zoom_at(2.0, 100.0, 100.0);
        let _ = app.update(Message::DialogClosed);
        let (a, b) = (
            app.viewport.camera,
            app.split.as_ref().expect("split").viewport.camera,
        );
        assert_eq!((a.center, a.scale), (b.center, b.scale));
        // One view again: no theme left behind.
        let _ = app.update(Message::Run("view.split"));
        assert!(app.viewport.theme.is_none());
        let _ = app.update(Message::DialogClosed);
        assert!(app.spatial.pick(inside, 0.5).is_some());
    }

    /// Kaydırmalı karşılaştır (docs/adr/0523 §3): the other view lets the pointer through, stands where the
    /// active view shows the same place, and the divider follows the pointer.
    #[test]
    fn the_compared_view_stands_where_the_active_one_shows_the_same_place() {
        let mut app = open();
        let _ = app.update(Message::Run("view.swipe"));
        let split = app.split.as_ref().expect("split");
        assert!(split.viewport.passive && split.swipe == Some(0.5));
        // The other view right of the divider: x 252..500 of the active area's 0..500.
        let _ = app.update(Message::OtherViewport(Event::Resized(Rectangle::new(
            Point::new(252.0, 0.0),
            Size::new(248.0, 800.0),
        ))));
        let other = &app.split.as_ref().expect("split").viewport;
        let here = Point::new(400.0, 300.0);
        let mine = app.viewport.world(here);
        let theirs = other.world(Point::new(here.x - 252.0, here.y));
        assert!((mine.x - theirs.x).abs() < 1e-6 && (mine.y - theirs.y).abs() < 1e-6);
        let _ = app.update(Message::Split(super::Event::Grab));
        let _ = app.update(Message::Split(super::Event::Drag(Point::new(150.0, 10.0))));
        let _ = app.update(Message::Split(super::Event::Release));
        assert_eq!(app.split.as_ref().and_then(|s| s.swipe), Some(0.3));
        let _ = app.update(Message::Run("view.swipe"));
        assert!(
            app.split
                .as_ref()
                .is_some_and(|s| s.swipe.is_none() && !s.viewport.passive)
        );
    }
}
