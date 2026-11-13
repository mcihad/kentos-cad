//! When an extension fails (docs/adr/8057 §4; the owner, 13 November: "eklenti hata verdiğinde açıkça bilelim
//! sorunu"): the window says it at once with the extension's name, the part that failed, why and where to look (the
//! status bar and Uyarılar); its log keeps it with its code and the error as it came; the manager shows the problem
//! still standing and copies a report for the publisher or an agent.

use iced::Element;
use iced::widget::text;
use kentos_extension_host::{Record, State};
use kentos_ui::label;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::Tip;
use kentos_ui::widget::status_bar::Readout;

use super::{Event, Extensions, Part, msg};
use crate::app::{App, Message};
use crate::cloud::local_time::{self, Zone};

/// An extension the status bar's cell names: a problem standing, or enabled and kept from running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Trouble {
    pub(crate) id: String,
    pub(crate) name: String,
    /// What is wrong, in a line.
    pub(crate) words: String,
}

/// The extensions in trouble, the ones with an error first: enabled ones only (one the user disabled is known to
/// them). None on a start without extensions: nothing runs on purpose then.
pub(crate) fn troubles(x: &Extensions) -> Vec<Trouble> {
    let Some(registry) = x.registry.as_ref() else {
        return Vec::new();
    };
    if x.safe_mode {
        return Vec::new();
    }
    let mut out: Vec<(bool, Trouble)> = registry
        .extensions()
        .into_iter()
        .filter_map(|e| {
            let words = match (&e.problem, &e.state) {
                // One the user disabled is no trouble of now: they know.
                (Some(p), _) if e.enabled => {
                    format!("{}: {}", p.part.as_deref().unwrap_or("eklenti"), p.message)
                }
                (
                    None,
                    State::Failed { .. }
                    | State::Blocked { .. }
                    | State::Incompatible { .. }
                    | State::Waiting { .. },
                ) if e.enabled => format!("{}: {}", e.state_label, e.help),
                _ => return None,
            };
            let error = e.problem.is_some() || matches!(e.state, State::Failed { .. });
            Some((
                error,
                Trouble {
                    id: e.id,
                    name: e.name,
                    words,
                },
            ))
        })
        .collect();
    out.sort_by_key(|(error, _)| !*error);
    out.into_iter().map(|(_, t)| t).collect()
}

/// A notice's longest error text: the rest is in the log and the report.
const SHORT: usize = 180;

/// Records a report lists.
const REPORTED: usize = 30;

/// A moment of a log in the device's time: “13.11.2026 18:42:07”.
pub(crate) fn when(ms: u64) -> String {
    let zone = Zone::system();
    let secs = i64::try_from(ms / 1000).unwrap_or(0);
    format!(
        "{} {}",
        local_time::date_of(secs, zone),
        local_time::clock(ms, zone)
    )
}

/// The first line of an error, cut to a notice's length.
pub(crate) fn short(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default();
    if line.chars().count() <= SHORT {
        line.to_owned()
    } else {
        format!("{}…", line.chars().take(SHORT).collect::<String>())
    }
}

/// A record's head line: when, the level, the part and the code.
pub(crate) fn head(r: &Record) -> String {
    let part = r
        .part
        .as_ref()
        .map(|p| format!(" · {p}"))
        .unwrap_or_default();
    format!("{} · {}{part} · {}", when(r.at), r.level.label(), r.code)
}

impl App {
    /// An extension's part failed in the window (a form, a data source): said at once in Uyarılar and the status bar
    /// with its name, the part, why and where to look; written in its log with its code and the error as it came. The
    /// manager opens on it next.
    pub(crate) fn extension_failed(
        &mut self,
        ext: &str,
        part: &str,
        code: &str,
        message: &str,
        detail: Option<&str>,
    ) {
        let name = self.extension_name(ext);
        let mut record = Record::error(code, message).of(ext, None).part(part);
        if let Some(detail) = detail {
            record = record.detail(detail);
        }
        let written = match self.extensions.registry.as_mut() {
            Some(registry) => registry.write_log(record),
            None => record,
        };
        let mut words = format!("{name} eklentisi hata verdi — {part}: {}", written.message);
        if let Some(detail) = &written.detail {
            words.push_str(&format!(" [{}]", short(detail)));
        }
        words.push_str(&format!(" Ayrıntı ve rapor: Eklentiler › {name}."));
        self.extensions.part = Part::Installed;
        self.extensions.chosen = Some(ext.to_owned());
        self.extensions.troubles = troubles(&self.extensions);
        self.error(words);
    }

    /// An extension's part worked (a data source connected, a form imported): written in its log, so that an error
    /// of that part no longer stands and the status bar's cell goes when it was the only one.
    pub(crate) fn extension_worked(&mut self, ext: &str, part: &str, code: &str, message: &str) {
        if let Some(registry) = self.extensions.registry.as_mut() {
            registry.write_log(Record::info(code, message).of(ext, None).part(part));
        }
        self.extensions.troubles = troubles(&self.extensions);
    }

    /// The status bar's cell while an extension is in trouble (a problem standing, or enabled and kept from running),
    /// in the danger colour as Eski kesit's: it stays until the problem is mended, so a start's error cannot pass
    /// unseen. Clicked, the manager on the first.
    pub(crate) fn extension_trouble_cell(&self) -> Option<Element<'_, Message>> {
        let list = &self.extensions.troubles;
        let first = list.first()?;
        let words = match list.len() {
            1 => format!("Eklenti hatası: {}", first.name),
            n => format!("{n} eklentide hata"),
        };
        let body = list
            .iter()
            .map(|t| format!("{} — {}", t.name, t.words))
            .collect::<Vec<_>>()
            .join("\n");
        Some(
            Readout::new(
                label::strong(words).style(move |theme: &iced::Theme| text::Style {
                    color: Some(Tokens::of(theme).danger),
                }),
            )
            .icon(crate::icons::from_web(Some("extensions")))
            .tip(Tip::new("Eklenti hatası").body(format!(
                "{body}\nTıklayın: Eklentiler penceresinde nedeni, günlüğü ve raporu."
            )))
            .on_press(msg(Event::ShowTrouble))
            .into(),
        )
    }

    /// The report the manager copies for the extension's publisher or an agent, in Markdown: what it is and where it
    /// came from, its state, the problem still standing with the error as it came, and its newest log.
    pub(crate) fn extension_report(&self, id: &str) -> Option<String> {
        let registry = self.extensions.registry.as_ref()?;
        let e = registry.extensions().into_iter().find(|e| e.id == id)?;
        let mut out = format!("# Eklenti raporu: {} {}\n\n", e.name, e.version);
        out.push_str(&format!("- Kimlik: `{}`\n", e.id));
        out.push_str(&format!("- Durum: {} — {}\n", e.state_label, e.help));
        if let Some(m) = registry.manifest(id) {
            let publisher = m
                .publisher_id
                .as_ref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            out.push_str(&format!("- Yayımcı: {}{publisher}\n", m.authors.join(", ")));
        }
        out.push_str(&format!(
            "- Kaynak: {} — {}\n",
            e.source.kind.words(),
            e.source.location
        ));
        out.push_str(&format!("- Güven: {}\n", e.trust_label));
        out.push_str(&format!(
            "- Bu bilgisayardaki sürümler: {}\n",
            e.versions.join(", ")
        ));
        let program = self.program.map_or("KentOS", |m| m.title());
        out.push_str(&format!(
            "- {program} {} · {} {}\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        out.push_str(&format!(
            "- Günlük dosyası: `{}`\n",
            registry.journal().file(Some(id)).display()
        ));
        out.push_str("\n## Sorun\n\n");
        match &e.problem {
            Some(p) => {
                out.push_str(&format!("{}\n\n{}\n", head(p), p.message));
                if let Some(detail) = &p.detail {
                    out.push_str(&format!("\n```\n{detail}\n```\n"));
                }
            }
            None => out.push_str("Şu an duran bir hata yok.\n"),
        }
        out.push_str("\n## Son kayıtlar (yeniden eskiye)\n\n");
        for r in e.log.iter().rev().take(REPORTED) {
            out.push_str(&format!("- {} — {}\n", head(r), r.message));
            if let Some(detail) = &r.detail {
                for line in detail.lines().take(8) {
                    out.push_str(&format!("  > {line}\n"));
                }
            }
        }
        Some(out)
    }
}
