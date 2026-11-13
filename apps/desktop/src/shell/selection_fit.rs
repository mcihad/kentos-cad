//! What the selection is for the ribbon (usage review UR-007): the kind its contextual tab is chosen by (an open
//! elliptical arc is a curve, not an area), the tab's name for a selection of one curved kind (Daire, Elips,
//! Eliptik yay) and the selection tools that take only some kinds, off with their reason when nothing selected is
//! one of them.

use kentos_commands::Entity;

use crate::app::App;
use crate::catalog::Tab;

/// Whether an ellipse is an arc of one: its parameters do not go all the way round.
fn open_ellipse(e: &Entity) -> bool {
    matches!(e, Entity::Ellipse(el) if el.t0 != el.t1 && (el.t1 - el.t0).rem_euclid(std::f64::consts::TAU) != 0.0)
}

/// The kind a selected object chooses its contextual tab by.
pub(crate) fn tab_kind(e: &Entity) -> &'static str {
    if open_ellipse(e) { "arc" } else { e.kind() }
}

/// The tab's name for a selection all of one curved kind, where the tab is named for more kinds.
fn curved_title(e: &Entity) -> Option<&'static str> {
    match e {
        Entity::Circle(_) => Some("Daire"),
        Entity::Ellipse(_) if open_ellipse(e) => Some("Eliptik yay"),
        Entity::Ellipse(_) => Some("Elips"),
        _ => None,
    }
}

/// The selection tools that take only some kinds: their id, whether an object is one, what they take.
const PICKY: [(&str, fn(&Entity) -> bool, &str); 2] = [
    (
        "tool.vertex",
        |e| matches!(e, Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)),
        "çizgi, çoklu çizgi ya da kapalı alan",
    ),
    (
        "tool.setElevation",
        kentos_native_application::elevation::takes,
        "çizgi, çoklu çizgi, alan ya da nokta",
    ),
];

impl App {
    /// The selection's tab name: a lone curved kind's own, else the tab's.
    pub(crate) fn selection_tab_title(&self, tab: &Tab) -> &'static str {
        let Some(doc) = &self.document else {
            return tab.label;
        };
        let mut titles = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| doc.model.get(*s))
            .map(curved_title);
        match titles.next() {
            Some(Some(first)) if titles.all(|t| t == Some(first)) => first,
            _ => tab.label,
        }
    }

    /// Why a selection tool is off: something is selected and none of it is a kind the tool takes.
    pub(crate) fn selection_misfit(&self, id: &str) -> Option<String> {
        let (_, fits, takes) = PICKY.iter().find(|(tool, _, _)| *tool == id)?;
        let doc = self.document.as_ref()?;
        let selected: Vec<&Entity> = self.selection.ids().iter().filter_map(|s| doc.model.get(*s)).collect();
        if selected.is_empty() || selected.iter().any(|e| fits(e)) {
            return None;
        }
        let mut kinds: Vec<&str> = Vec::new();
        for e in &selected {
            let title = crate::selecting::kind_title(e.kind());
            if !kinds.contains(&title) {
                kinds.push(title);
            }
        }
        let title = crate::catalog::catalog().get(id).map_or(id, |c| c.title);
        let kinds = crate::layer_tree::lower_tr(&kinds.join(", "));
        Some(format!("{title} seçili {kinds} için çalışmaz; {takes} seçin."))
    }
}
