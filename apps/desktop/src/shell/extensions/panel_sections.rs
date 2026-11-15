//! A panel's sections (docs/adr/8057 §38): a `section` heading gathers the components after it, up to the next one,
//! and the user folds it. Folding is a view, not a rule: what a folded section gathers is still shown to the rules and
//! asked of the user ([`OpenPanel::shown`]), only not drawn; a field of it left wrong opens it, so the words under the
//! field are seen. A section hidden (its rule, its code) hides what it gathers.

use kentos_extensions::ui_panel::Component;

use super::panels::OpenPanel;

impl OpenPanel {
    /// Whether component `at` is drawn: shown, and not in a folded section.
    pub(crate) fn drawn(&self, at: usize, c: &Component) -> bool {
        self.shown(at, c)
            && self
                .section_of(at)
                .is_none_or(|s| !self.closed.contains(&s))
    }

    /// The section component `at` is in: the nearest one above it; none above the first, and none for a section.
    pub(crate) fn section_of(&self, at: usize) -> Option<usize> {
        if matches!(
            self.definition.body.get(at),
            Some(Component::Section { .. })
        ) {
            return None;
        }
        (0..at)
            .rev()
            .find(|i| matches!(self.definition.body[*i], Component::Section { .. }))
    }

    /// Section `at` folded when open, opened when folded: the user pressed its heading (`PanelEvent::Section`).
    pub(crate) fn toggle_section(&mut self, at: usize) {
        if !self.closed.remove(&at) {
            self.closed.insert(at);
        }
    }

    /// Input `id`'s section opened, when it is folded: a field left wrong is said where the user sees it.
    pub(crate) fn reveal(&mut self, id: &str) {
        if let Some(s) = self
            .definition
            .body
            .iter()
            .position(|c| c.id() == Some(id))
            .and_then(|at| self.section_of(at))
        {
            self.closed.remove(&s);
        }
    }
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;
    use serde_json::json;

    use super::*;

    fn panel() -> OpenPanel {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Bölümler",
            "body": [
                { "type": "textField", "id": "ad", "label": "Ad" },
                { "type": "section", "id": "gelismis", "label": "Gelişmiş", "collapsed": true },
                { "type": "numberField", "id": "adet", "label": "Adet", "required": true },
                { "type": "section", "label": "Yardım", "visibleWhen": "ad = 'göster'" },
                { "type": "text", "text": "Bir yardım satırı." },
                { "type": "button", "id": "kaydet", "label": "Kaydet", "command": "kaydet" }
            ] });
        OpenPanel::new(
            "tr.ornek.deneme",
            "bolumler",
            ui_panel::read(&value).expect("a panel"),
        )
    }

    /// A folded section's components are shown (asked, read by the rules) but not drawn; one left wrong opens its
    /// section; a section hidden by its rule hides what it gathers; the user folds and opens it.
    #[test]
    fn a_section_folds_what_it_gathers_and_opens_for_a_field_left_wrong() {
        let mut p = panel();
        let body = p.definition.body.clone();
        assert_eq!(p.section_of(2), Some(1));
        assert_eq!(p.section_of(1), None, "a section is in no section");
        assert_eq!(p.section_of(0), None, "above the first");
        assert!(
            p.shown(2, &body[2]) && !p.drawn(2, &body[2]),
            "folded: shown, not drawn"
        );
        assert!(p.drawn(1, &body[1]), "its heading is drawn");
        // The required number in the folded section is asked all the same; its section opens to say so.
        let (id, _) = p.check_params().unwrap_err();
        assert_eq!(id, "adet");
        p.reveal(&id);
        assert!(p.drawn(2, &body[2]), "opened for the field left wrong");
        // Hidden by its rule: its heading and what it gathers.
        assert!(!p.shown(3, &body[3]) && !p.shown(4, &body[4]) && !p.drawn(5, &body[5]));
        p.texts.insert("ad".into(), "göster".into());
        assert!(p.drawn(3, &body[3]) && p.drawn(4, &body[4]));
        // The user folds it: its heading stays, what it gathers goes; pressed again, it opens.
        p.toggle_section(3);
        assert!(p.drawn(3, &body[3]) && !p.drawn(4, &body[4]));
        p.toggle_section(3);
        assert!(p.drawn(4, &body[4]));
    }
}
