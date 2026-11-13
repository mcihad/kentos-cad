//! Komut ara finds the extensions' buttons (docs/adr/8057 §3; plan/eklenti.md ADIM 18): a data source, a form or a
//! Python item of an extension's tab this program shows, by its label, its tab's or its extension's name. Each row
//! says where it lives (“Kent Rehberi › Altlık · Kent Rehberi eklentisi”) or why it does not run yet; Enter runs it,
//! Alt+Enter opens its tab and outlines it. A button that runs a KentOS command is not listed again: the command itself
//! is found.

use iced::Task;
use kentos_ui::widget::search_box::Found;

use super::ribbon::Action;
use super::{Event, msg};
use crate::app::{App, Message};
use crate::exchange::apply::fold_turkish;

/// A button of an extension's tab: the tab, its panel and its place there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExtHit {
    pub(crate) tab: &'static str,
    pub(crate) panel: usize,
    pub(crate) item: usize,
}

impl App {
    /// The buttons the folded text finds, with their rank as Komut ara ranks commands (a label equal, beginning with
    /// it, holding it; then the tab's or the extension's name holding it).
    pub(crate) fn extension_hits(&self, q: &str) -> Vec<(f64, ExtHit)> {
        if q.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for tab in self.extension_tabs() {
            let around = fold_turkish(&format!(
                "{} {}",
                tab.label,
                self.extension_name(&tab.extension)
            ));
            for (p, panel) in tab.panels.iter().enumerate() {
                for (i, item) in panel.items.iter().enumerate() {
                    if matches!(item.action, Action::Builtin(_)) {
                        continue;
                    }
                    let label = fold_turkish(&item.label);
                    let score = if label == q {
                        0.5
                    } else if label.starts_with(q) {
                        2.0
                    } else if label.contains(q) {
                        3.0
                    } else if around.contains(q) {
                        3.5
                    } else {
                        continue;
                    };
                    out.push((
                        score,
                        ExtHit {
                            tab: tab.id,
                            panel: p,
                            item: i,
                        },
                    ));
                }
            }
        }
        out
    }

    /// A found button's row: its icon and label, where it lives and whose it is, or why it does not run yet.
    pub(crate) fn extension_found(&self, hit: ExtHit) -> Found<'static> {
        let Some((tab, panel, item)) = self.extension_item(hit) else {
            return Found {
                icon: crate::icons::from_web(Some("extensions")),
                title: "?".into(),
                detail: "Eklenti artık çalışmıyor.".into(),
                shortcut: None,
                enabled: false,
                revealable: false,
            };
        };
        let whose = format!("{} eklentisi", self.extension_name(&tab.extension));
        let (detail, enabled) = match &item.action {
            Action::Unavailable(why) => (format!("{why} · {whose}"), false),
            _ => (format!("{} › {} · {whose}", tab.label, panel.label), true),
        };
        Found {
            icon: item.icon,
            title: item.label.clone().into(),
            detail: detail.into(),
            shortcut: None,
            enabled,
            revealable: true,
        }
    }

    /// Enter or a click: the button's work, as pressing it does.
    pub(crate) fn extension_hit_run(&mut self, hit: ExtHit) -> Task<Message> {
        let Some((_, _, item)) = self.extension_item(hit) else {
            return Task::none();
        };
        match item.action.clone() {
            Action::Source(ext, local) => self.extensions_event(Event::OpenSource(ext, local)),
            Action::Form(ext, local) => self.extensions_event(Event::OpenForm(ext, local)),
            Action::Builtin(id) => self.run(id),
            Action::Unavailable(_) => Task::none(),
        }
    }

    /// Alt+Enter or the row's pin: the extension's tab opens and the button is outlined for a moment.
    pub(crate) fn extension_hit_reveal(&mut self, hit: ExtHit) -> Task<Message> {
        if self.extension_item(hit).is_none() {
            return Task::none();
        }
        self.choose_tab(hit.tab);
        self.ribbon_context = false;
        self.ribbon_collapsed = false;
        self.extensions.flash = Some(hit);
        crate::hover_card::after(
            crate::ribbon_search::FLASH,
            msg(Event::FlashEnd(hit.tab, hit.panel, hit.item)),
        )
    }

    /// The outline goes, unless another reveal took its place.
    pub(crate) fn extension_flash_end(&mut self, hit: ExtHit) {
        if self.extensions.flash == Some(hit) {
            self.extensions.flash = None;
        }
    }

    /// The tab, the panel and the button a hit names, while this program shows them.
    fn extension_item(
        &self,
        hit: ExtHit,
    ) -> Option<(
        &super::ribbon::ExtTab,
        &super::ribbon::ExtPanel,
        &super::ribbon::ExtItem,
    )> {
        let tab = self.extension_tab(hit.tab)?;
        let panel = tab.panels.get(hit.panel)?;
        let item = panel.items.get(hit.item)?;
        Some((tab, panel, item))
    }
}
