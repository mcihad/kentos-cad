//! How the desktop writes its log, as the web does (the web's
//! ui/bottom/logPlan.ts, warnings.ts and MessageLog; docs/adr/0114): a
//! line's time and level in the bottom panel's Komut geçmişi and Uyarılar,
//! which lines each tab lists, what the user typed as the history echoes
//! it, which lines the status bar shows and for how long, how many lines
//! are kept and what the Uyarılar badge counts. fixtures/shell/v1/log.json
//! holds every answer; both platforms play it (`log_plan_tests.rs`).

use std::collections::VecDeque;

use kentos_interaction::Level;

use crate::cloud::local_time::Zone;

/// The panel's tabs, in order: their names.
pub(crate) const TAB_HISTORY: &str = "Komut geçmişi";
pub(crate) const TAB_COORDS: &str = "Koordinat listesi";
pub(crate) const TAB_MESSAGES: &str = "Uyarılar";

/// The panel's words: its two buttons, the command line's button, the empty lists.
pub(crate) const CLEAR: &str = "Geçmişi temizle";
pub(crate) const CLOSE: &str = "Paneli kapat";
/// The web's button on the command line; the desktop's is the command
/// line's own Geçmiş button (KentOS UI).
#[cfg(test)]
pub(crate) const OPEN: &str = "Komut geçmişini aç";
pub(crate) const EMPTY_HISTORY: &str =
    "Henüz komut çalıştırılmadı. Bir araç seçin ya da komut satırına yazın.";
pub(crate) const EMPTY_MESSAGES: &str = "Uyarı yok.";

/// The log keeps this many lines; older ones go first.
pub(crate) const LIMIT: usize = 500;

/// New lines scroll a list to its end only when it was this close to its end
/// (px): one being read stays put.
pub(crate) const FOLLOW_WITHIN: f32 = 24.0;

/// A line's time: the local wall clock when it was written, “09:05:07”,
/// 24-hour, two digits each; the second the clock shows, never rounded up.
pub(crate) fn log_time(at_ms: i64, zone: &Zone) -> String {
    let seconds = at_ms.div_euclid(1000);
    let local = seconds + i64::from(zone.offset_at(seconds));
    let day = local.rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", day / 3600, day % 3600 / 60, day % 60)
}

/// A level's icon in the lists, as the web names it: none for a command or a plain line.
pub(crate) fn level_icon(level: Level) -> Option<&'static str> {
    match level {
        Level::Command | Level::Info => None,
        Level::Success => Some("success"),
        Level::Warn => Some("warning"),
        Level::Error => Some("error"),
    }
}

/// Which list a tab shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Listing {
    History,
    Messages,
}

/// Whether a tab lists a line of this level: Komut geçmişi every line,
/// Uyarılar the warnings and errors.
pub(crate) fn listed_in(tab: Listing, level: Level) -> bool {
    tab == Listing::History || matches!(level, Level::Warn | Level::Error)
}

/// What the user typed or chose (a value, an option's letter), as the history
/// writes it before it is handled.
pub(crate) fn echo(text: &str) -> String {
    format!("› {text}")
}

/// A line the status bar shows: its icon (the web's name) and how long (ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shown {
    pub icon: &'static str,
    pub ms: u64,
}

/// Whether the status bar shows a line: a command's line never, nor a line
/// that goes on the one before it (written indented, two spaces first: a
/// clicked point, a distance); a warning or an error for 9 s, anything else
/// for 5 s.
pub(crate) fn flash_of(level: Level, text: &str) -> Option<Shown> {
    if level == Level::Command || text.starts_with("  ") {
        return None;
    }
    let icon = match level {
        Level::Warn => "warning",
        Level::Error => "error",
        Level::Success => "success",
        _ => "info",
    };
    let ms = if matches!(level, Level::Warn | Level::Error) {
        9000
    } else {
        5000
    };
    Some(Shown { icon, ms })
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    /// Grows from 1 for the program's run, across Geçmişi temizle too.
    pub id: u64,
    pub level: Level,
    pub text: String,
    /// When it was written (milliseconds since 1970).
    pub at: i64,
}

/// The log: its lines (500 at most) and what the Uyarılar badge has seen.
#[derive(Debug, Default)]
pub(crate) struct Log {
    lines: VecDeque<Line>,
    last: u64,
    /// The newest id seen while the Uyarılar tab was on screen.
    seen: u64,
}

impl Log {
    /// Writes a line; the oldest goes past the limit. Its id.
    pub(crate) fn push(&mut self, level: Level, text: impl Into<String>, at: i64) -> u64 {
        self.last += 1;
        self.lines.push_back(Line {
            id: self.last,
            level,
            text: text.into(),
            at,
        });
        while self.lines.len() > LIMIT {
            self.lines.pop_front();
        }
        self.last
    }

    /// Geçmişi temizle: the lines go; the ids go on.
    pub(crate) fn clear(&mut self) {
        self.lines.clear();
    }

    /// The Uyarılar tab is on screen: every line so far is seen (over an
    /// empty log nothing changes).
    pub(crate) fn look(&mut self) {
        if let Some(line) = self.lines.back() {
            self.seen = line.id;
        }
    }

    /// The badge: the warnings and errors written after the last look.
    pub(crate) fn unseen(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| matches!(l.level, Level::Warn | Level::Error) && l.id > self.seen)
            .count()
    }

    pub(crate) fn lines(&self) -> impl DoubleEndedIterator<Item = &Line> {
        self.lines.iter()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    /// The newest line's id (0 before any).
    pub(crate) fn last_id(&self) -> u64 {
        self.lines.back().map_or(0, |l| l.id)
    }

    #[cfg(test)]
    pub(crate) fn first_id(&self) -> u64 {
        self.lines.front().map_or(0, |l| l.id)
    }

    /// The newest line.
    #[cfg(test)]
    pub(crate) fn last(&self) -> Option<&Line> {
        self.lines.back()
    }

    /// Whether a line of this level says `part`.
    #[cfg(test)]
    pub(crate) fn said(&self, level: Level, part: &str) -> bool {
        self.lines
            .iter()
            .any(|l| l.level == level && l.text.contains(part))
    }
}
