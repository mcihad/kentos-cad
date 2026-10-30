//! What a card says and where it goes (docs/adr/0800 §3), apart from how it
//! is drawn: its state from the manifest, the program found and the
//! processes running; the search; the grid's columns; the summary's counts.

use kentos_ui::attribute::text;

use crate::manifest::Manifest;
use crate::resolve::Found;

/// A card's state, as its dot and word say it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Its program was found: it starts.
    Installed,
    /// Started from here, still running (how many).
    Running(usize),
    /// Not out yet.
    Soon,
    /// Its program was not found.
    Missing,
    /// A web address: the browser opens it.
    Web,
}

impl Status {
    pub fn word(self) -> String {
        match self {
            Status::Installed => "Kurulu".to_owned(),
            Status::Running(1) => "Çalışıyor".to_owned(),
            Status::Running(n) => format!("Çalışıyor ({n})"),
            Status::Soon => "Yakında".to_owned(),
            Status::Missing => "Bulunamadı".to_owned(),
            Status::Web => "Web adresi".to_owned(),
        }
    }

    /// Whether the card's button starts it.
    pub fn starts(self) -> bool {
        matches!(self, Status::Installed | Status::Running(_) | Status::Web)
    }
}

/// The state of an app found as `found`, with `running` of its processes.
pub fn status(found: &Found, running: usize) -> Status {
    match found {
        Found::Program(_) if running > 0 => Status::Running(running),
        Found::Program(_) => Status::Installed,
        Found::Url(_) => Status::Web,
        Found::Soon => Status::Soon,
        Found::Missing(_) => Status::Missing,
    }
}

/// Whether the app answers the search: every word of `query` in its name or
/// note, Turkish letters folded (KentOS UI's `fold`: “GİS” finds “GIS”,
/// “harita” finds “Harita”).
pub fn matches(app: &Manifest, query: &str) -> bool {
    let haystack = text::fold(&format!("{} {}", app.name, app.note));
    text::fold(query)
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

/// The grid's columns for `width`: as many cards of at least `least` as fit,
/// two to four.
pub fn columns(width: f32, least: f32, gap: f32) -> usize {
    let fit = ((width + gap) / (least + gap)).floor();
    (fit.max(0.0) as usize).clamp(2, 4)
}

/// A card's width in `width` with `columns` cards: whole pixels, so that
/// every edge is sharp.
pub fn card_width(width: f32, columns: usize, gap: f32) -> f32 {
    let n = columns.max(1) as f32;
    ((width - gap * (n - 1.0)) / n).floor().max(0.0)
}

/// The summary strip's counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub apps: usize,
    pub installed: usize,
    pub soon: usize,
    pub missing: usize,
    pub running: usize,
}

impl Counts {
    pub fn of(states: impl IntoIterator<Item = Status>) -> Self {
        let mut counts = Counts::default();
        for state in states {
            counts.apps += 1;
            match state {
                Status::Installed | Status::Web => counts.installed += 1,
                Status::Running(_) => {
                    counts.installed += 1;
                    counts.running += 1;
                }
                Status::Soon => counts.soon += 1,
                Status::Missing => counts.missing += 1,
            }
        }
        counts
    }

    /// “5 uygulama”, “3 kurulu”, “2 yakında”: the parts that are not zero
    /// (the apps' count always).
    pub fn parts(self) -> Vec<String> {
        let mut parts = vec![format!("{} uygulama", self.apps)];
        if self.installed > 0 {
            parts.push(format!("{} kurulu", self.installed));
        }
        if self.running > 0 {
            parts.push(format!("{} çalışıyor", self.running));
        }
        if self.soon > 0 {
            parts.push(format!("{} yakında", self.soon));
        }
        if self.missing > 0 {
            parts.push(format!("{} bulunamadı", self.missing));
        }
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use crate::resolve::Spec;

    #[test]
    fn a_card_state_from_what_was_found_and_what_runs() {
        let program = Found::Program(Spec::default());
        assert_eq!(status(&program, 0), Status::Installed);
        assert_eq!(status(&program, 2), Status::Running(2));
        assert_eq!(status(&Found::Soon, 0), Status::Soon);
        assert_eq!(status(&Found::Missing("x".into()), 0), Status::Missing);
        assert_eq!(status(&Found::Url("https://a".into()), 0), Status::Web);
        assert_eq!(Status::Running(1).word(), "Çalışıyor");
        assert_eq!(Status::Running(3).word(), "Çalışıyor (3)");
        assert!(Status::Web.starts() && !Status::Soon.starts() && !Status::Missing.starts());
    }

    #[test]
    fn the_search_folds_turkish_letters_and_reads_every_word() {
        let registry = Registry::builtin();
        let found = |query: &str| -> Vec<String> {
            registry
                .apps
                .iter()
                .filter(|a| matches(a, query))
                .map(|a| a.id.clone())
                .collect()
        };
        assert_eq!(found("GİS"), ["kentos-gis"]);
        assert_eq!(found("gis"), ["kentos-gis"]);
        assert_eq!(found("harita"), ["kentos-harita"]);
        assert_eq!(found("HARİTA"), ["kentos-harita"]);
        assert_eq!(found("kat planı"), ["kentos-mimari"], "the note too");
        assert_eq!(found("kentos yol"), ["kentos-yol"], "every word");
        assert_eq!(found("").len(), 4);
        assert!(found("uzay").is_empty());
    }

    #[test]
    fn the_grid_has_two_to_four_columns_of_whole_pixels() {
        // 1440 × 900 at 13 px: the content is 1172 wide, cards at least 320.
        assert_eq!(columns(1172.0, 320.0, 16.0), 3);
        // 1100 × 650: 832 wide.
        assert_eq!(columns(832.0, 320.0, 16.0), 2);
        assert_eq!(columns(400.0, 320.0, 16.0), 2, "never one");
        assert_eq!(columns(3000.0, 320.0, 16.0), 4, "never five");
        // At 18 px the least card is 443 wide: two columns at 1440.
        assert_eq!(columns(1069.0, 443.0, 22.0), 2);
        assert_eq!(card_width(1172.0, 3, 16.0), 380.0);
        assert_eq!(card_width(833.0, 2, 16.0), 408.0, "floored");
    }

    #[test]
    fn the_summary_counts_what_there_is() {
        let counts = Counts::of([
            Status::Installed,
            Status::Running(2),
            Status::Installed,
            Status::Soon,
            Status::Soon,
        ]);
        assert_eq!(
            counts.parts(),
            ["5 uygulama", "3 kurulu", "1 çalışıyor", "2 yakında"]
        );
        assert_eq!(Counts::of([]).parts(), ["0 uygulama"]);
        assert_eq!(
            Counts::of([Status::Missing, Status::Web]).parts(),
            ["2 uygulama", "1 kurulu", "1 bulunamadı"]
        );
    }
}
