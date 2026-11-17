//! A panel's tabs (docs/adr/8057 §43): a component's `tab` puts it on a page; the bar lists the tabs in the order they
//! first appear and stands where the first component with a tab is; a component without a tab is drawn on every page.
//! A tab is a view, not a rule: what another page holds is still shown to the rules and asked of the user
//! ([`OpenPanel::shown`]), only not drawn; a field of it left wrong brings its page up. A tab all of whose components
//! are hidden is not listed, and a chosen tab hidden so gives the first one its place.

use iced::Element;
use kentos_ui::widget::tabs::{Tab, Tabs};

use super::panels::{OpenPanel, PanelEvent};
use crate::app::Message;

impl OpenPanel {
    /// Component `at`'s tab as written; none for one drawn on every page.
    pub(crate) fn tab_at(&self, at: usize) -> Option<&str> {
        self.definition.rules.get(at).and_then(|r| r.tab.as_deref())
    }

    /// The tabs the bar lists: in the order they first appear in the body, each while one of its components is shown.
    pub(crate) fn tabs(&self) -> Vec<&str> {
        let mut all: Vec<&str> = Vec::new();
        for at in 0..self.definition.body.len() {
            if let Some(tab) = self.tab_at(at)
                && !all.contains(&tab)
            {
                all.push(tab);
            }
        }
        all.retain(|tab| {
            self.definition
                .body
                .iter()
                .enumerate()
                .any(|(at, c)| self.tab_at(at) == Some(*tab) && self.shown(at, c))
        });
        all
    }

    /// The tab whose page is drawn, of `tabs` (the listed ones): the one the user chose while it is listed, else the
    /// first; none for a panel without tabs.
    pub(crate) fn active_tab<'a>(&'a self, tabs: &[&'a str]) -> Option<&'a str> {
        self.tab
            .as_deref()
            .filter(|t| tabs.contains(t))
            .or_else(|| tabs.first().copied())
    }

    /// Whether component `at` is on page `page` (the drawn tab): it has no tab, or that one.
    pub(crate) fn on_page(&self, at: usize, page: Option<&str>) -> bool {
        self.tab_at(at).is_none_or(|t| Some(t) == page)
    }

    /// Tab `name`'s page drawn: the user pressed it (`PanelEvent::Tab`), or a field of it was left wrong.
    pub(crate) fn show_tab(&mut self, name: &str) {
        self.tab = Some(name.to_owned());
    }
}

/// The bar of `tabs`, `page` drawn: KentOS's tab strip (it scrolls and lists them when it does not fit); a tab pressed
/// is `event`'s.
pub(super) fn tab_bar<'a>(
    tabs: &[&str],
    page: Option<&str>,
    event: fn(PanelEvent) -> Message,
) -> Element<'a, Message> {
    let names: Vec<String> = tabs.iter().map(|t| (*t).to_owned()).collect();
    let active = page
        .and_then(|p| tabs.iter().position(|t| *t == p))
        .unwrap_or(0);
    let strip: Vec<Tab<'a>> = names
        .iter()
        .map(|t| Tab::new(t.clone()).closable(false))
        .collect();
    Tabs::new(strip, active, move |i| {
        event(PanelEvent::Tab(names[i].clone()))
    })
    .into()
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;
    use serde_json::json;

    fn panel() -> super::OpenPanel {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Sekmeler",
            "body": [
                { "type": "text", "text": "Her sayfada." },
                { "type": "textField", "id": "ad", "label": "Ad", "tab": "Genel" },
                { "type": "checkbox", "id": "ayrinti", "label": "Ayrıntı", "tab": "Genel" },
                { "type": "numberField", "id": "adet", "label": "Adet", "required": true, "tab": "Seçenekler" },
                { "type": "section", "label": "Gelişmiş", "tab": "Seçenekler" },
                { "type": "textField", "id": "not", "label": "Not", "tab": "Seçenekler" },
                { "type": "text", "text": "Ayrıntılar", "tab": "Ayrıntı", "visibleWhen": "ayrinti" },
                { "type": "button", "id": "calistir", "label": "Çalıştır", "command": "x" }
            ] });
        super::OpenPanel::new(
            "tr.ornek.deneme",
            "sekmeler",
            ui_panel::read(&value).expect("a panel"),
        )
    }

    /// The tabs in their first order, a hidden one not listed; the first drawn until the user chooses; a component
    /// without a tab on every page; a required field on another page asked all the same and its page brought up; a
    /// section stops gathering at a component of another tab, so the trailing button is not folded with it.
    #[test]
    fn a_tab_draws_its_page_and_a_field_left_wrong_brings_it_up() {
        let mut p = panel();
        assert_eq!(
            p.tabs(),
            ["Genel", "Seçenekler"],
            "Ayrıntı hidden by its rule"
        );
        assert_eq!(p.active_tab(&p.tabs()), Some("Genel"));
        let page = Some("Genel");
        assert!(p.on_page(0, page) && p.on_page(1, page) && !p.on_page(3, page));
        assert!(p.on_page(7, page), "the button without a tab on every page");
        // The number on Seçenekler is asked; its page comes up.
        let (id, _) = p.check_params().unwrap_err();
        assert_eq!(id, "adet");
        p.reveal(&id);
        assert_eq!(p.active_tab(&p.tabs()), Some("Seçenekler"));
        // The section gathers Not, not the button after it: folding it keeps the button drawn.
        assert_eq!(p.section_of(5), Some(4));
        assert_eq!(p.section_of(7), None, "another tab ends the section");
        // Shown by its rule, the third tab is listed; chosen, then hidden again, the first takes its place.
        p.checks.insert("ayrinti".into(), true);
        assert_eq!(p.tabs(), ["Genel", "Seçenekler", "Ayrıntı"]);
        p.show_tab("Ayrıntı");
        p.checks.insert("ayrinti".into(), false);
        assert_eq!(p.active_tab(&p.tabs()), Some("Genel"));
    }
}
