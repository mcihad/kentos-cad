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
//!
//! Up to four views (docs/adr/0637 §1): Görünüm ekle adds a view (two, three side by side, four in two rows); each
//! view that is not the active one waits in `Split::others` at a place of its own, a press on it swaps it with the
//! active one (their places swap too, so nothing moves on the screen), Bağla moves every one of them, × closes one.

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

/// What a view's strip leaves free at its right: grid north stands there.
const NORTH_ROOM: f32 = 48.0;

/// A view narrower than this has its strip shortened: the choosers narrower, the scale without its word.
const NARROW: f32 = 470.0;

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

/// The most views the drawing area splits into (docs/adr/0637 §1).
pub(crate) const MOST_VIEWS: usize = 4;

/// A view of the split area that is not the active one.
pub(crate) struct Other {
    pub(crate) viewport: Viewport,
    /// Its drawing's text as last placed.
    spots: crate::labels::Spots,
    /// Where it stands in the area: 0 the first (left, or top left).
    pub(crate) place: usize,
    /// The other state of the drawing it shows, read only (docs/adr/0576 §3); none, the open drawing.
    pub(crate) revision: Option<Revision>,
}

impl Other {
    /// Its text is placed again (what it shows changed).
    pub(crate) fn reset_text(&mut self) {
        self.spots = crate::labels::Spots::default();
    }
}

/// The views that are not active and where they stand.
pub(crate) struct Split {
    /// The views that are not active now, by their index (their viewport's `other`).
    pub(crate) others: Vec<Other>,
    /// Where the active view stands.
    pub(crate) active_at: usize,
    /// The drawing it was split for: another drawing goes back to one view.
    session: u64,
    /// Bağla: what the other views follow of the active one (docs/adr/0523 §3, 0576 §2).
    pub(crate) link: Link,
    /// Kaydırmalı karşılaştır (two views only): where the divider stands, a share of the active view's width from
    /// its left; none, side by side.
    pub(crate) swipe: Option<f32>,
    /// The divider is being dragged.
    pub(crate) dragging: bool,
}

impl Split {
    /// The first view that is not active: the one Kaydırmalı karşılaştır draws and Geçmiş shows a revision in.
    pub(crate) fn first(&self) -> Option<&Other> {
        self.others.first()
    }

    /// Each view tells its place in `others` again (one closed).
    fn renumber(&mut self) {
        for (i, o) in self.others.iter_mut().enumerate() {
            o.viewport.other = Some(i);
        }
    }
}

/// What the views' own controls ask (docs/adr/0523): `view` names one, none the active view, else the index of
/// one that is not.
#[derive(Debug, Clone)]
pub enum Event {
    /// A view's time chosen (docs/adr/0576 §1).
    Time {
        view: Option<usize>,
        ask: TimeAsk,
    },
    /// What the other views follow (docs/adr/0576 §2).
    Link(Link),
    /// What a view that is not active shows (docs/adr/0576 §3); a file is asked for, then `RevisionPicked`.
    Source(usize, Source),
    RevisionPicked(usize, Option<std::path::PathBuf>),
    /// A view's theme chosen; the layer state's place in the project's list, none: as the drawing is.
    Theme {
        view: Option<usize>,
        state: Option<usize>,
    },
    /// × on a view that is not active: it closes (docs/adr/0637 §1).
    Close(usize),
    /// The divider taken, moved to a share of the width, let go.
    Grab,
    Drag(Point),
    Release,
}

impl App {
    /// Görünümü böl (`view.split`): two views side by side, or one again.
    pub(crate) fn toggle_split(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        if self.split.take().is_some() {
            self.viewport.theme = None;
            self.viewport.passive = false;
            self.say(Level::Info, "Çizim alanı tek görünüm.");
            return;
        }
        self.split_into_two();
        self.say(
            Level::Info,
            "Çizim alanı iki görünüme bölündü: her birinin kendi kamerası var, tıklanan etkin olur.",
        );
    }

    /// Two views: the active one and one more at its right, at the same place.
    fn split_into_two(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let session = doc.session;
        self.split = Some(Split {
            others: Vec::new(),
            active_at: 0,
            session,
            link: Link::Off,
            swipe: None,
            dragging: false,
        });
        self.push_view();
    }

    /// One more view at the next free place, at the active view's place and with its grid.
    fn push_view(&mut self) {
        let mut viewport = Viewport::new();
        viewport.camera = self.viewport.camera;
        viewport.grid_shown = self.viewport.grid_shown;
        let Some(split) = self.split.as_mut() else {
            return;
        };
        let taken: Vec<usize> = std::iter::once(split.active_at)
            .chain(split.others.iter().map(|o| o.place))
            .collect();
        let place = (0..MOST_VIEWS).find(|p| !taken.contains(p)).unwrap_or(0);
        viewport.other = Some(split.others.len());
        split.others.push(Other {
            viewport,
            spots: crate::labels::Spots::default(),
            place,
            revision: None,
        });
    }

    /// Görünüm ekle (`view.splitAdd`, docs/adr/0637 §1): one more view, up to four; the area splits first.
    pub(crate) fn add_view(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        if self.split.is_none() {
            self.toggle_split();
            return;
        }
        let views = self.split.as_ref().map_or(1, |s| s.others.len() + 1);
        if views >= MOST_VIEWS {
            self.warn(format!(
                "Çizim alanı en çok {MOST_VIEWS} görünüme bölünür. Bir görünümü üstündeki × ile kapatın."
            ));
            return;
        }
        // Kaydırmalı karşılaştır is of two views: side by side again.
        if let Some(split) = self.split.as_mut()
            && split.swipe.take().is_some()
        {
            split.dragging = false;
            for o in &mut split.others {
                o.viewport.passive = false;
            }
        }
        self.push_view();
        let n = views + 1;
        self.say(
            Level::Info,
            format!(
                "Çizim alanı {n} görünüm: her birinin kendi kamerası, teması ve zamanı var; tıklanan etkin olur."
            ),
        );
    }

    /// × on the view `i` that is not active: it closes; the last one closed leaves one view.
    pub(crate) fn close_view(&mut self, i: usize) {
        let Some(split) = self.split.as_mut() else {
            return;
        };
        if i >= split.others.len() {
            return;
        }
        let gone = split.others.remove(i);
        if split.others.is_empty() {
            self.split = None;
            self.viewport.theme = None;
            self.viewport.passive = false;
            self.say(Level::Info, "Çizim alanı tek görünüm.");
            return;
        }
        // The places close up: the ones after the closed view's move back one.
        let p = gone.place;
        if split.active_at > p {
            split.active_at -= 1;
        }
        for o in &mut split.others {
            if o.place > p {
                o.place -= 1;
            }
        }
        split.renumber();
        let n = split.others.len() + 1;
        self.say(
            Level::Info,
            format!("Görünüm kapandı: çizim alanı {n} görünüm."),
        );
    }

    /// Another drawing goes back to one view; linked or compared views follow the active one; what is picked
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
            if let Some(split) = self.split.as_mut() {
                for o in &mut split.others {
                    if gone(&o.viewport.theme) {
                        o.viewport.theme = None;
                    }
                }
            }
        }
        if let Some(split) = self.split.as_mut() {
            let active = self.viewport.camera;
            let swipe = split.swipe.is_some();
            let link = split.link;
            for o in &mut split.others {
                let other = &mut o.viewport;
                if swipe {
                    // The same world under the same place of the window: the other view's middle as the active one
                    // sees it.
                    other.camera.scale = active.scale;
                    other.camera.twist = active.twist;
                    let b = other.bounds;
                    let a = self.viewport.bounds;
                    let middle = self.viewport.world(Point::new(
                        b.x + b.width / 2.0 - a.x,
                        b.y + b.height / 2.0 - a.y,
                    ));
                    // A world point, on the map when there is one (docs/adr/0583 §3).
                    other.camera.center_on(middle);
                } else if link != Link::Off {
                    let got = view_link::follow(link, frame_of(&active), frame_of(&other.camera));
                    other.camera.center = kentos_render_wgpu::Vec2::new(got.centre.x, got.centre.y);
                    other.camera.scale = got.scale;
                    other.camera.twist = got.twist;
                }
            }
        }
        // A time of its own needs the slider: without it every view follows (the slider's window is none).
        if !self.time.slider.open {
            self.viewport.time = ViewTime::Follow;
            if let Some(split) = self.split.as_mut() {
                for o in &mut split.others {
                    o.viewport.time = ViewTime::Follow;
                }
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

    /// Görünümleri bağla (`view.splitLink`): the other views follow the active one, or go their own way again.
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
            Link::Centre => {
                "Görünümler bağlı: diğer görünümler etkin görünümün merkezini izler, ölçeklerini korur."
            }
            Link::CentreScale => {
                "Görünümler bağlı: diğer görünümler etkin görünümün merkezini, ölçeğini ve dönüklüğünü izler."
            }
        };
        self.say(Level::Info, said);
    }

    /// Kaydırmalı karşılaştır (`view.swipe`): the other view over the active one right of a divider, or side by
    /// side again; two views only.
    pub(crate) fn toggle_swipe(&mut self) {
        if self.split.is_none() {
            self.toggle_split();
        }
        let Some(split) = self.split.as_mut() else {
            return;
        };
        if split.swipe.take().is_some() {
            split.dragging = false;
            for o in &mut split.others {
                o.viewport.passive = false;
            }
            self.say(Level::Info, "Görünümler yan yana.");
            return;
        }
        if split.others.len() > 1 {
            self.warn(
                "Kaydırmalı karşılaştır iki görünümle olur: fazla görünümleri üstlerindeki × ile kapatın.",
            );
            return;
        }
        split.swipe = Some(0.5);
        if let Some(o) = split.others.first_mut() {
            o.viewport.passive = true;
        }
        self.say(
            Level::Info,
            "Kaydırmalı karşılaştır: ayırıcının solunda etkin görünüm, sağında diğer görünüm aynı yerde. Ayırıcıyı sürükleyin.",
        );
    }

    /// The view a control names: the active one, or one that is not.
    fn view_at_mut(&mut self, view: Option<usize>) -> Option<&mut Viewport> {
        match view {
            None => Some(&mut self.viewport),
            Some(i) => self
                .split
                .as_mut()
                .and_then(|s| s.others.get_mut(i))
                .map(|o| &mut o.viewport),
        }
    }

    fn view_at(&self, view: Option<usize>) -> Option<&Viewport> {
        match view {
            None => Some(&self.viewport),
            Some(i) => self
                .split
                .as_ref()
                .and_then(|s| s.others.get(i))
                .map(|o| &o.viewport),
        }
    }

    /// The view's width as it was last told, if it was.
    fn view_width(&self, view: Option<usize>) -> Option<f32> {
        self.view_at(view)
            .map(|v| v.bounds.width)
            .filter(|w| *w > 0.0)
    }

    /// The width of a view's choosers: the usual one, or what a narrow view leaves beside its scale and ×.
    fn chooser_width(&self, view: Option<usize>) -> f32 {
        match self.view_width(view) {
            Some(w) if w < NARROW => (w - 16.0 - NORTH_ROOM - 96.0).clamp(96.0, CHOOSER),
            _ => CHOOSER,
        }
    }

    /// What the views' controls ask.
    pub(crate) fn split_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Time { view, ask } => {
                let s = &self.time.slider;
                let (anchor, step, last, moment) = (s.anchor, s.step, s.last, s.moment());
                if let Some(view) = self.view_at_mut(view) {
                    view.time = match (ask, view.time) {
                        (TimeAsk::Own, _) => ViewTime::Own(moment),
                        (TimeAsk::Step(by), ViewTime::Own(m)) => {
                            ViewTime::Own(view_link::own_step(anchor, step, last, m, by))
                        }
                        (TimeAsk::Step(_), ViewTime::Follow) | (TimeAsk::Follow, _) => {
                            ViewTime::Follow
                        }
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
            Event::Source(i, Source::Drawing) => self.hide_revision(i),
            Event::Source(i, Source::History(n)) => self.revision_from_history(i, n),
            Event::Source(i, Source::File) => return self.pick_revision_file(i),
            Event::RevisionPicked(i, path) => {
                if let Some(path) = path {
                    self.revision_from_file(i, &path);
                }
            }
            Event::Theme { view, state } => {
                let theme = state.and_then(|i| {
                    self.document
                        .as_ref()
                        .and_then(|d| d.settings().layer_states.get(i).cloned())
                });
                if let Some(v) = self.view_at_mut(view) {
                    v.theme = theme;
                }
                self.follow_split();
            }
            Event::Close(i) => self.close_view(i),
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
    fn theme_chooser<'a>(&'a self, view: Option<usize>) -> Element<'a, Message> {
        use kentos_ui::widget::select::{Choice, Select};
        let Some(doc) = &self.document else {
            return Space::new().into();
        };
        let states = &doc.settings().layer_states;
        let theme = self.view_at(view).and_then(|v| v.theme.as_ref());
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
                view,
                state: i.checked_sub(1),
            })
        });
        container(pick)
            .width(Length::Fixed(self.chooser_width(view)))
            .into()
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
    /// Sürgüyü izle. A revision's view has one too (docs/adr/0637 §2): its own drawing at its own time.
    fn time_chip<'a>(&'a self, view: Option<usize>) -> Option<Element<'a, Message>> {
        let slider = &self.time.slider;
        if !slider.open {
            return None;
        }
        let time = self.view_at(view)?.time;
        let ask = move |ask: TimeAsk| Message::Split(Event::Time { view, ask });
        let narrow = self.view_width(view).is_some_and(|w| w < NARROW);
        let small = move |words: &'static str, a: TimeAsk| {
            button(label::caption(words).wrapping(iced::widget::text::Wrapping::None))
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
                    small(
                        if narrow { "İzle" } else { "Sürgüyü izle" },
                        TimeAsk::Follow
                    ),
                ]
                .spacing(4)
                .align_y(Center)
                .into()
            }
        };
        Some(chip(content))
    }

    /// A view's screen scale, 1:N on a 96 dpi screen (docs/adr/0576 §2).
    fn scale_chip<'a>(&'a self, view: Option<usize>) -> Element<'a, Message> {
        let camera = self
            .view_at(view)
            .map_or(self.viewport.camera, |v| v.camera);
        let n = view_link::screen_scale(camera.scale).round();
        let scale = kentos_project::wizard::scale_text(n);
        // A narrow view says the scale alone (docs/adr/0637 §1).
        let words = match self.view_width(view) {
            Some(w) if w < NARROW => scale,
            _ => format!("Ölçek {scale}"),
        };
        chip(
            label::caption(words)
                .wrapping(iced::widget::text::Wrapping::None)
                .into(),
        )
    }

    /// What the other views follow of the active one (docs/adr/0576 §2).
    fn link_chooser<'a>(&'a self, view: Option<usize>) -> Element<'a, Message> {
        use kentos_ui::widget::select::{Choice, Select};
        let links = [Link::Off, Link::Centre, Link::CentreScale];
        let at = self
            .split
            .as_ref()
            .and_then(|s| links.iter().position(|l| *l == s.link));
        let width = self.chooser_width(view);
        let choices = [
            Choice::new("Bağ: yok".to_owned()),
            Choice::new("Bağ: merkez".to_owned()),
            Choice::new("Bağ: merkez ve ölçek".to_owned()),
        ];
        let pick = Select::new(choices, at, move |i| Message::Split(Event::Link(links[i])));
        container(pick).width(Length::Fixed(width)).into()
    }

    /// What the view `i` shows (docs/adr/0576 §3): the open drawing, its history some steps back, a file.
    fn source_chooser<'a>(&'a self, i: usize) -> Element<'a, Message> {
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
        let shown = self
            .split
            .as_ref()
            .and_then(|s| s.others.get(i))
            .and_then(|o| o.revision.as_ref());
        let at = match shown {
            None => Some(0),
            Some(r) => sources.iter().position(|x| *x == r.source),
        };
        let pick = Select::new(choices, at, move |k| {
            Message::Split(Event::Source(i, sources[k]))
        });
        container(pick)
            .width(Length::Fixed(self.chooser_width(Some(i))))
            .into()
    }

    /// × on a view that is not active.
    fn close_button<'a>(&'a self, i: usize) -> Element<'a, Message> {
        kentos_ui::widget::tip(
            button(label::caption("×"))
                .on_press(Message::Split(Event::Close(i)))
                .padding([1, 7])
                .style(style::button::flat),
            kentos_ui::widget::Tip::new("Görünümü kapat"),
            iced::widget::tooltip::Position::Bottom,
        )
    }

    /// A view's controls at its top left (docs/adr/0523 §1, 0576): the theme and the scale, the time, and for a
    /// view that is not active what it shows, what it follows, what it differs in and its ×.
    fn strip<'a>(&'a self, view: Option<usize>) -> Element<'a, Message> {
        let mut col = column![].spacing(4).padding(8);
        let Some(i) = view else {
            col = col.push(
                row![self.theme_chooser(None), self.scale_chip(None)]
                    .spacing(4)
                    .align_y(Center),
            );
            if let Some(time) = self.time_chip(None) {
                col = col.push(time);
            }
            return col.into();
        };
        let revision = self
            .split
            .as_ref()
            .and_then(|s| s.others.get(i))
            .and_then(|o| o.revision.as_ref());
        col = col.push(
            row![
                self.source_chooser(i),
                self.scale_chip(view),
                self.close_button(i)
            ]
            .spacing(4)
            .align_y(Center),
        );
        if revision.is_none() {
            col = col.push(self.theme_chooser(view));
        }
        if let Some(time) = self.time_chip(view) {
            col = col.push(time);
        }
        col = col.push(self.link_chooser(view));
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
        // Grid north keeps its room at the right; a long line wraps short of it.
        match self.view_width(view) {
            Some(w) => container(col).max_width((w - NORTH_ROOM).max(120.0)).into(),
            None => col.into(),
        }
    }

    /// Kaydırmalı karşılaştır's part of the active area (view.rs puts it over the active view's text, under its
    /// marks): the other view right of the divider, its text, the divider; nothing when not compared.
    pub(crate) fn swipe_layer(&self) -> Option<Element<'_, Message>> {
        let split = self.split.as_ref()?;
        let share = split.swipe?;
        let doc = self.document.as_ref()?;
        let first = split.first()?;
        let left = (share * 1000.0).round().max(1.0) as u16;
        let right = (1000 - left).max(1);
        let other = self.other_view(first, doc);
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

    /// A view's drawing and text as it shows them (its camera, its theme, its time), or the revision it shows: its own
    /// drawing and store, its changed objects marked.
    fn other_view<'a>(
        &'a self,
        other: &'a Other,
        doc: &'a crate::document::Document,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let (shown, spatial, selection, mark) = match &other.revision {
            // The open drawing's selection, the same records here, while there is one; else what changed.
            Some(r) if !r.chosen.is_empty() => {
                (&r.doc, &r.spatial, &r.chosen, Tokens::of(&theme).accent)
            }
            Some(r) => (&r.doc, &r.spatial, &r.marked, Tokens::of(&theme).warning),
            None => (
                doc,
                &self.spatial,
                &self.selection,
                Tokens::of(&theme).accent,
            ),
        };
        let accent = crate::viewport::highlight_color(
            &self.settings.text("graphics.highlightColor"),
            crate::view::rgba8(mark),
        );
        let accent = if other.revision.as_ref().is_some_and(|r| r.chosen.is_empty()) {
            crate::view::rgba8(mark)
        } else {
            accent
        };
        let format = kentos_interaction::Format::of(shown.settings());
        let window = self.view_window(other.viewport.time);
        spatial.store().with_window(window, || {
            let spots = other.spots.last();
            let area = other.viewport.view(
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
            let tree = other.viewport.themed_tree(shown);
            let labels = crate::labels::layer(
                &shown.model,
                shown.session,
                spatial,
                &other.spots,
                &other.viewport.camera,
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

    /// What a view that is not active reports: its size, and the wheel and middle button move it; a press makes it
    /// the active view and goes on to it.
    pub(crate) fn other_viewport(&mut self, i: usize, event: ViewEvent) -> Task<Message> {
        let Some(other) = self.split.as_mut().and_then(|s| s.others.get_mut(i)) else {
            return Task::none();
        };
        match event {
            ViewEvent::Resized(_)
            | ViewEvent::Zoomed { .. }
            | ViewEvent::Panned { .. }
            | ViewEvent::Extents
            | ViewEvent::Settled => {
                other.viewport.update(event, self.document.as_ref());
                Task::none()
            }
            ViewEvent::Pressed(_) | ViewEvent::RightPressed(_) => {
                // A revision is read only: it never takes the pointer, the tool or the snap (docs/adr/0576 §3).
                if other.revision.is_some() {
                    return Task::none();
                }
                self.activate_other(i);
                self.pointer(event)
            }
            _ => Task::none(),
        }
    }

    /// The view `i` becomes the active one: the two swap, each with its camera, its pictures and its text, and their
    /// places, so neither moves on the screen.
    pub(crate) fn activate_other(&mut self, i: usize) {
        let Some(split) = self.split.as_mut() else {
            return;
        };
        let Some(other) = split.others.get_mut(i) else {
            return;
        };
        if other.revision.is_some() {
            return;
        }
        std::mem::swap(&mut self.viewport, &mut other.viewport);
        std::mem::swap(&mut self.label_spots, &mut other.spots);
        std::mem::swap(&mut split.active_at, &mut other.place);
        self.viewport.other = None;
        other.viewport.other = Some(i);
        other.viewport.cursor = None;
        // The marks of the pointer were the other view's.
        self.snap = None;
        self.panning = false;
    }

    /// The drawing area with the other views beside the active one, the active one outlined: two or three side by
    /// side, four in two rows; the active one alone when the area is not split. Compared (Kaydırmalı karşılaştır)
    /// the active area holds the other view itself (`swipe_layer`); each view's controls at its top.
    pub(crate) fn with_split<'a>(&'a self, active: Element<'a, Message>) -> Element<'a, Message> {
        let (Some(split), Some(doc)) = (&self.split, &self.document) else {
            return active;
        };
        if split.swipe.is_some() {
            return stack![
                active,
                row![
                    self.strip(None),
                    Space::new().width(Fill),
                    self.strip(Some(0))
                ]
            ]
            .into();
        }
        let framed = |content: Element<'a, Message>, on: bool| -> Element<'a, Message> {
            container(content)
                .padding(1)
                .style(move |theme: &iced::Theme| {
                    let t = kentos_ui::theme::Tokens::of(theme);
                    container::Style {
                        border: iced::Border {
                            color: if on { t.accent } else { t.border },
                            width: 1.0,
                            radius: 0.0.into(),
                        },
                        ..container::Style::default()
                    }
                })
                .width(Fill)
                .height(Fill)
                .into()
        };
        let n = split.others.len() + 1;
        let mut cells: Vec<Option<Element<'a, Message>>> = (0..n).map(|_| None).collect();
        if let Some(cell) = cells.get_mut(split.active_at) {
            *cell = Some(framed(stack![active, self.strip(None)].into(), true));
        }
        for (i, o) in split.others.iter().enumerate() {
            if let Some(cell) = cells.get_mut(o.place) {
                *cell = Some(framed(
                    stack![self.other_view(o, doc), self.strip(Some(i))].into(),
                    false,
                ));
            }
        }
        let mut cells = cells
            .into_iter()
            .map(|c| c.unwrap_or_else(|| Space::new().into()));
        if n == 4 {
            let mut top = row![].spacing(2).height(Fill);
            let mut bottom = row![].spacing(2).height(Fill);
            for _ in 0..2 {
                if let Some(c) = cells.next() {
                    top = top.push(c);
                }
            }
            for c in cells {
                bottom = bottom.push(c);
            }
            column![top, bottom].spacing(2).into()
        } else {
            let mut line = row![].spacing(2);
            for c in cells {
                line = line.push(c);
            }
            line.into()
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
        let _ = app.update(Message::OtherViewport(
            0,
            Event::Resized(Rectangle::new(
                Point::new(502.0, 0.0),
                Size::new(500.0, 800.0),
            )),
        ));
        // The wheel over the other view zooms it alone.
        let main = app.viewport.camera;
        let _ = app.update(Message::OtherViewport(
            0,
            Event::Zoomed {
                factor: 2.0,
                at: Point::new(250.0, 400.0),
            },
        ));
        assert_eq!(app.viewport.camera, main, "the active view stays");
        let other = app.split.as_ref().expect("split").others[0].viewport.camera;
        assert!((other.scale / main.scale - 2.0).abs() < 1e-9);
        // A polyline with a corner in the active view, the second corner in the other.
        let _ = app.update(Message::Run("tool.polyline"));
        let _ = app.update(Message::Viewport(Event::Moved(Point::new(100.0, 100.0))));
        let _ = app.update(Message::Viewport(Event::Pressed(Point::new(100.0, 100.0))));
        let _ = app.update(Message::Viewport(Event::Released(Point::new(100.0, 100.0))));
        assert_eq!(app.session.point_count(), 1);
        let at = Point::new(300.0, 300.0);
        let world = app.split.as_ref().expect("split").others[0]
            .viewport
            .world(at);
        let _ = app.update(Message::OtherViewport(0, Event::Pressed(at)));
        let _ = app.update(Message::Viewport(Event::Released(at)));
        // The other view is the active one now, with its own camera; the corner is where it was clicked in it.
        assert!((app.viewport.camera.scale / main.scale - 2.0).abs() < 1e-9);
        assert!(app.split.as_ref().is_some_and(|s| s.active_at == 1));
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
            app.split
                .as_ref()
                .is_some_and(|s| s.link == Link::CentreScale),
            "Bağla splits first"
        );
        let _ = app.update(Message::Split(super::Event::Theme {
            view: Some(0),
            state: Some(1),
        }));
        let doc = app.document.as_ref().expect("open");
        let other = app.split.as_ref().expect("split").others[0]
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
            view: None,
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
            app.split.as_ref().expect("split").others[0].viewport.camera,
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
        assert!(split.others[0].viewport.passive && split.swipe == Some(0.5));
        // The other view right of the divider: x 252..500 of the active area's 0..500.
        let _ = app.update(Message::OtherViewport(
            0,
            Event::Resized(Rectangle::new(
                Point::new(252.0, 0.0),
                Size::new(248.0, 800.0),
            )),
        ));
        let other = &app.split.as_ref().expect("split").others[0].viewport;
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
                .is_some_and(|s| s.swipe.is_none() && !s.others[0].viewport.passive)
        );
    }
}
