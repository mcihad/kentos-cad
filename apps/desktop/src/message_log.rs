//! The log in the app, as the web keeps and shows it (docs/adr/0114): the
//! web's `MessageLog` (app/state.ts), the bottom panel's Komut geçmişi and
//! Uyarılar lists (BottomPanel.ts), the Uyarılar badge (warnings.ts), what
//! the command line's ↑ brings back (CommandLine.ts) and the status bar's
//! message for its few seconds (StatusBar.ts). The rules are log_plan.rs's.

use std::time::{Duration, Instant};

use iced::futures::channel::oneshot;
use iced::futures::{Stream, StreamExt, stream};
use iced::widget::scrollable::Viewport;
use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::{Column, container, hover, row, scrollable, space, text};
use iced::{Center, Color, Element, Fill, Padding, Subscription, Task, Theme, window};
use kentos_interaction::Level;
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Elided;
use kentos_ui::widget::command_line::Entry;

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::cloud::local_time::Zone;
use crate::log_plan::{self as plan, Line, Listing};

/// The status bar's message fades in and out in this long (the web's
/// 160 ms opacity transition, `look.flash.fadeMs`).
pub(crate) const FADE: Duration = Duration::from_millis(160);

/// The lists' scroll id: new lines take them to their end.
pub(crate) const LIST: &str = "bottom-log";

/// The icons' size in the lists and in the status bar's message (the web's `ICON_SIZE`).
const ICON_SIZE: f32 = 14.0;

/// The status bar's message: its line, and when it came and goes.
#[derive(Debug, Clone)]
pub(crate) struct Flash {
    pub level: Level,
    pub text: String,
    /// The web's icon name.
    pub icon: &'static str,
    since: Instant,
    until: Instant,
    /// How visible the bar's message was when this one came: one on screen
    /// is replaced at once, as the web's cell keeps its opacity.
    from: f32,
}

impl Flash {
    /// How visible it is at `now`: it fades in, stays, then fades out.
    pub(crate) fn alpha(&self, now: Instant) -> f32 {
        let part = |d: Duration| (d.as_secs_f32() / FADE.as_secs_f32()).min(1.0);
        if now >= self.until {
            return 1.0 - part(now - self.until);
        }
        self.from + (1.0 - self.from) * part(now.saturating_duration_since(self.since))
    }

    /// Whether it is fading now: the window's frames draw it.
    pub(crate) fn fading(&self, now: Instant) -> bool {
        (self.from < 1.0 && now < self.since + FADE)
            || (now >= self.until && now < self.until + FADE)
    }

    /// Whether it has gone.
    fn gone(&self, now: Instant) -> bool {
        now >= self.until + FADE
    }
}

/// What the log's views follow between messages.
#[derive(Debug)]
pub(crate) struct Follow {
    /// The status bar's message.
    pub flash: Option<Flash>,
    /// The newest line the status bar has looked at.
    flashed: u64,
    /// The list on screen and the newest line it showed.
    shown: Option<(Listing, u64)>,
    /// The list was near its end at its last scroll (the web's `FOLLOW_WITHIN`).
    near_end: bool,
}

impl Follow {
    /// The status bar's message as it stays on screen, for pictures.
    #[cfg(test)]
    pub(crate) fn show_fully(&mut self) {
        if let Some(flash) = &mut self.flash {
            flash.from = 1.0;
        }
    }
}

impl Default for Follow {
    fn default() -> Self {
        Self {
            flash: None,
            flashed: 0,
            shown: None,
            near_end: true,
        }
    }
}

/// What the user typed in the command line are kept at most this many.
const TYPED: usize = plan::LIMIT;

impl App {
    /// A line in the log, with the web's level: commands and what the user
    /// typed, information, success, warnings, errors. Said while Uyarılar is
    /// on screen, it is seen (the web's badge).
    pub(crate) fn say(&mut self, level: Level, text: impl Into<String>) {
        self.log.push(level, text, crate::cloud::now_ms());
        self.last_level = Some(level);
        if self.messages_on_screen() {
            self.log.look();
        }
    }

    /// A value or an option given to the running command, echoed as the
    /// history writes it before it is handled (the web's `log.command(echo(text))`).
    pub(crate) fn echo_value(&mut self, text: impl AsRef<str>) {
        self.say(Level::Command, plan::echo(text.as_ref()));
    }

    /// What was typed in the command line, for its ↑ (the web's
    /// `CommandLine.remember`): the same text twice in a row is kept once.
    pub(crate) fn remember(&mut self, text: &str) {
        if text.is_empty() || matches!(self.typed.last(), Some(Entry::Input(last)) if last == text)
        {
            return;
        }
        self.typed.push(Entry::Input(text.to_owned()));
        if self.typed.len() > TYPED {
            self.typed.remove(0);
        }
    }

    pub(crate) fn output(&mut self, text: impl Into<String>) {
        self.say(Level::Info, text);
    }

    pub(crate) fn warn(&mut self, text: impl Into<String>) {
        self.say(Level::Warn, text);
    }

    pub(crate) fn error(&mut self, text: impl Into<String>) {
        self.say(Level::Error, text);
    }

    /// The Uyarılar tab is on screen.
    fn messages_on_screen(&self) -> bool {
        self.command_expanded && self.bottom_tab == BottomTab::Messages
    }

    /// The list the bottom panel shows, if a log list.
    fn listing_on_screen(&self) -> Option<Listing> {
        if !self.command_expanded {
            return None;
        }
        match self.bottom_tab {
            BottomTab::History => Some(Listing::History),
            BottomTab::Messages => Some(Listing::Messages),
            BottomTab::Coords
            | BottomTab::Points
            | BottomTab::Table
            | BottomTab::Search
            | BottomTab::Topology
            | BottomTab::ServiceInfo
            | BottomTab::Python => None,
        }
    }

    /// After every message: the status bar takes the newest line it shows,
    /// and the list on screen follows new lines to its end when it was near
    /// it; a list just shown starts at its end (the web's `appendLog`,
    /// `renderContent`).
    pub(crate) fn follow_log(&mut self, now: Instant) -> Task<Message> {
        let mut tasks = Vec::new();
        let flashed = self.follow.flashed;
        let newest = self
            .log
            .lines()
            .rev()
            .take_while(|l| l.id > flashed)
            .find_map(|l| plan::flash_of(l.level, &l.text).map(|shown| (l, shown)));
        if let Some((line, shown)) = newest {
            let from = self.follow.flash.as_ref().map_or(0.0, |f| f.alpha(now));
            let ms = Duration::from_millis(shown.ms);
            self.follow.flash = Some(Flash {
                level: line.level,
                text: line.text.clone(),
                icon: shown.icon,
                since: now,
                until: now + ms,
                from,
            });
        }
        self.follow.flashed = self.follow.flashed.max(self.log.last_id());
        if self.follow.flash.as_ref().is_some_and(|f| f.gone(now)) {
            self.follow.flash = None;
        }

        let listing = self.listing_on_screen();
        let newest_listed = listing.map_or(0, |listing| {
            self.log
                .lines()
                .rev()
                .find(|l| plan::listed_in(listing, l.level))
                .map_or(0, |l| l.id)
        });
        match (self.follow.shown, listing) {
            (Some((was, _)), Some(now_listing)) if was == now_listing => {
                if newest_listed > self.follow.shown.map_or(0, |s| s.1) && self.follow.near_end {
                    tasks.push(iced::widget::operation::snap_to_end(LIST));
                }
            }
            (_, Some(_)) => {
                self.follow.near_end = true;
                tasks.push(iced::widget::operation::snap_to_end(LIST));
            }
            (_, None) => {}
        }
        self.follow.shown = listing.map(|l| (l, newest_listed));
        Task::batch(tasks)
    }

    /// The list was scrolled (or its lines changed): whether it is near its end.
    pub(crate) fn log_scrolled(&mut self, viewport: Viewport) {
        let end = viewport.content_bounds().height;
        let seen = viewport.absolute_offset().y + viewport.bounds().height;
        self.follow.near_end = end - seen < plan::FOLLOW_WITHIN;
    }

    /// A frame of the status bar's message: the message that has gone is dropped.
    pub(crate) fn log_frame(&mut self, now: Instant) {
        if self.follow.flash.as_ref().is_some_and(|f| f.gone(now)) {
            self.follow.flash = None;
        }
    }

    /// What the status bar's message waits for: the window's frames while it
    /// fades, the moment its fade out starts while it stays.
    pub(crate) fn log_subscription(&self, now: Instant) -> Subscription<Message> {
        match &self.follow.flash {
            Some(flash) if flash.fading(now) => window::frames().map(|_| Message::LogFrame),
            Some(flash) if now < flash.until => Subscription::run_with(flash.until, wake_at),
            _ => Subscription::none(),
        }
    }

    /// A tab's list (the web's `.log`): every line of the tab with its time,
    /// its level's icon and its text; or what the empty list says.
    pub(crate) fn log_list(&self, listing: Listing) -> Element<'_, Message> {
        let mut lines = self
            .log
            .lines()
            .filter(|l| plan::listed_in(listing, l.level))
            .peekable();
        if lines.peek().is_none() {
            let empty = match listing {
                Listing::History => plan::EMPTY_HISTORY,
                Listing::Messages => plan::EMPTY_MESSAGES,
            };
            // The web's `.empty--inline`: third tone, the body's size.
            return container(text(empty).size(typography::body()).style(faint))
                .padding(Padding::new(14.0).bottom(12.0))
                .into();
        }
        let zone = Zone::system();
        let rows = Column::with_children(lines.map(|l| log_row(l, zone))).padding([6, 0]);
        scrollable(rows)
            .id(LIST)
            .on_scroll(Message::LogScrolled)
            .direction(kentos_ui::style::field::body_scrollbar())
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// The status bar's message (the web's `status__flash`): the newest line
    /// the bar shows, with its level's icon, fading in and out; the text is
    /// cut with “…” where the cell ends.
    /// Whether the status bar shows a message now (the second system's
    /// values leave it their room when both do not fit, docs/adr/0167 §2).
    pub(crate) fn flashing(&self) -> bool {
        self.follow
            .flash
            .as_ref()
            .is_some_and(|flash| flash.alpha(Instant::now()) > 0.0)
    }

    pub(crate) fn flash_cell(&self) -> Element<'_, Message> {
        let now = Instant::now();
        let Some(flash) = &self.follow.flash else {
            return space::horizontal().into();
        };
        let alpha = flash.alpha(now);
        if alpha <= 0.0 {
            return space::horizontal().into();
        }
        let level = flash.level;
        let mark = container(icon(crate::icons::from_web(Some(flash.icon))).size(ICON_SIZE)).style(
            move |theme: &Theme| container::Style {
                text_color: Some(icon_color(level, theme).scale_alpha(alpha)),
                ..container::Style::default()
            },
        );
        let words = Elided::new(flash.text.as_str())
            .size(typography::body())
            .style(move |theme: &Theme| text::Style {
                color: Some(Tokens::of(theme).muted.scale_alpha(alpha)),
            });
        container(row![mark, words].spacing(6).align_y(Center))
            .padding([0, 12])
            .width(Fill)
            .clip(true)
            .into()
    }
}

/// One message at `until`, from a thread of its own (as the recovery ticks run).
fn wake_at(until: &Instant) -> impl Stream<Item = Message> + use<> {
    let until = *until;
    let (done, wait) = oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(until.saturating_duration_since(Instant::now()));
        let _ = done.send(());
    });
    stream::once(wait).map(|_| Message::LogFrame)
}

/// The web's `--c-text-3`.
fn faint(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Tokens::of(theme).muted.scale_alpha(0.75)),
    }
}

/// A line's text colour (the web's `.log__row--*`): a command's the text's,
/// a warning's and an error's their own, the rest the second tone.
fn text_color(level: Level, theme: &Theme) -> Color {
    let t = Tokens::of(theme);
    match level {
        Level::Command => t.text,
        Level::Warn => t.warning,
        Level::Error => t.danger,
        Level::Info | Level::Success => t.muted,
    }
}

/// A line's icon colour, in the lists and in the status bar (information
/// keeps the bar's own tone).
fn icon_color(level: Level, theme: &Theme) -> Color {
    let t = Tokens::of(theme);
    match level {
        Level::Success => t.success,
        Level::Warn => t.warning,
        Level::Error => t.danger,
        Level::Command | Level::Info => t.muted,
    }
}

/// A line of a list (the web's `.log__row`): its time (64 px), its level's
/// icon (18 px) and its text, which keeps its spaces and wraps; the row
/// lights up under the pointer.
fn log_row<'a>(line: &'a Line, zone: &Zone) -> Element<'a, Message> {
    let size = typography::caption();
    let height = LineHeight::Relative(1.5);
    let level = line.level;
    let time = text(plan::log_time(line.at, zone))
        .font(typography::mono())
        .size(size)
        .line_height(height)
        .style(faint);
    let mark: Element<'a, Message> = match plan::level_icon(level) {
        Some(name) => container(icon(crate::icons::from_web(Some(name))).size(ICON_SIZE))
            .padding(Padding::ZERO.top(2.0))
            .style(move |theme: &Theme| container::Style {
                text_color: Some(icon_color(level, theme)),
                ..container::Style::default()
            })
            .into(),
        None => space::horizontal().width(0).into(),
    };
    // A command's line is the text's, in the web's 500 (the nearest face kept is the semibold).
    let font = if level == Level::Command {
        typography::mono_strong()
    } else {
        typography::mono()
    };
    let words = text(line.text.as_str())
        .font(font)
        .size(size)
        .line_height(height)
        .wrapping(Wrapping::WordOrGlyph)
        .style(move |theme: &Theme| text::Style {
            color: Some(text_color(level, theme)),
        });
    let line = container(row![
        container(time).width(typography::from_default(64.0)),
        container(mark).width(18),
        container(words).width(Fill),
    ])
    .padding([1, 12])
    .width(Fill);
    let lit = container(space::horizontal().height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(t.layer(if t.is_dark { 0.055 } else { 0.06 }).into()),
                ..container::Style::default()
            }
        });
    hover(line, lit)
}
