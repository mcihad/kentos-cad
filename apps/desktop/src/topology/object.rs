//! One object's findings (docs/adr/3800; plan/map-cad.md MAP-07 “veri formu yolu”): what Öznitelikler's Topoloji
//! section and Formla düzenle say of the selected object, from the Topoloji tab's last check — no check of its own, so
//! nothing slows the selection. A finding pressed there opens the tab on it (`Event::Show`).

use kentos_domain::Slot;

use super::{Filter, plan, review};
use crate::app::App;
use crate::bottom::BottomTab;

/// One finding of the object as its words say it: its problem, its rule, its place in its life cycle (with the
/// reason), its measure, and its lasting key.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ObjectFinding {
    pub problem: String,
    pub rule: String,
    pub state: String,
    pub measure: String,
    pub key: String,
}

/// What can be said of the object: no check of the drawing yet, or the last check's findings naming it (`old` when
/// the drawing changed since).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ObjectFindings {
    NotChecked,
    Checked { findings: Vec<ObjectFinding>, old: bool },
}

impl App {
    /// The object's findings; none when no rule of the project names its layer (the object is not checked at all).
    pub(crate) fn object_findings(&self, slot: Slot) -> Option<ObjectFindings> {
        let doc = self.document.as_ref()?;
        let layer = doc.model.get(slot)?.base().layer_id.clone();
        let named = |id: &str| id == layer;
        if !self.topology_settings()?.rules.iter().any(|r| named(&r.layer) || r.other.as_deref().is_some_and(named)) {
            return None;
        }
        let Some(c) = self.topology_checked() else {
            return Some(ObjectFindings::NotChecked);
        };
        let name = |id: &str| self.topology_layer_name(id);
        let format = self.format();
        let findings = c
            .findings
            .iter()
            .filter(|f| f.objects.iter().any(|&k| c.g.slots.get(k).copied().flatten() == Some(slot)))
            .map(|f| ObjectFinding {
                problem: plan::problem_text(f, c.g.lapsed(f)),
                rule: c.g.rules.get(f.rule).map_or_else(String::new, |r| plan::rule_text(r, name)),
                state: review::state_text(c, f),
                measure: plan::measure_text(f, &format),
                key: c.key(f),
            })
            .collect();
        Some(ObjectFindings::Checked { findings, old: self.topology_stale() })
    }

    /// The Topoloji tab opened on the finding of `key`: every finding listed, its row chosen and shown over the drawing.
    pub(crate) fn topology_open_finding(&mut self, key: &str) {
        let Some(c) = self.topology_checked() else { return };
        let Some(at) = c.findings.iter().position(|f| c.key(f) == key) else {
            self.warn("Bu bulgu son denetimde yok; Topoloji sekmesinde yeniden denetleyin.");
            return;
        };
        self.topology.filter = Filter::All;
        self.topology.rule = None;
        self.show_bottom(BottomTab::Topology);
        if let Some(row) = self.topology_shown().iter().position(|&i| i == at) {
            self.topology_press(row);
        }
    }
}
