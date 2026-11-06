//! The project's style tables (text, dimension, leader, wall and table styles) as an undo step keeps them: style by
//! style, by id (docs/adr/0699). A `cad.styles.edit` write records the change with the objects that followed
//! the style, so one undo gives both back, even when only a name changed and no object moved.
//!
//! Undo and redo touch only the styles the step changed, and each only while it still is as the step left it:
//! a style another editor changed since, and every style the step never touched, stays as it is. A style that
//! would come back under a name another style took since stays away, so the table is never left invalid.

use crate::{Document, history::Op};
use kentos_contracts::{
    DimensionStyleDef, LeaderStyleDef, ProjectSettings, TableStyleDef, TextStyleDef, WallStyleDef,
    dimension_styles_problem, leader_styles_problem, table_style_defs_problem, text_styles_problem,
    wall_styles_problem,
};

/// A style of one of the tables: its id.
trait Styled: Clone + PartialEq {
    fn id(&self) -> &str;
}

macro_rules! styled {
    ($($t:ty),*) => {$(
        impl Styled for $t {
            fn id(&self) -> &str {
                &self.id
            }
        }
    )*};
}

styled!(TextStyleDef, DimensionStyleDef, LeaderStyleDef, WallStyleDef, TableStyleDef);

/// One style before and after the step, each with its place in the table; none: not there.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Change<T> {
    id: String,
    before: Option<(usize, T)>,
    after: Option<(usize, T)>,
}

impl<T: Clone> Change<T> {
    fn inverse(&self) -> Self {
        Self {
            id: self.id.clone(),
            before: self.after.clone(),
            after: self.before.clone(),
        }
    }
}

/// The styles a step changed, table by table.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StyleTables {
    text: Vec<Change<TextStyleDef>>,
    dimension: Vec<Change<DimensionStyleDef>>,
    leader: Vec<Change<LeaderStyleDef>>,
    wall: Vec<Change<WallStyleDef>>,
    /// The table styles (docs/adr/1126 §2).
    table: Vec<Change<TableStyleDef>>,
}

/// The styles that differ between two tables: the ones before in their order, then the new ones in theirs.
fn changes<T: Styled>(before: &[T], after: &[T]) -> Vec<Change<T>> {
    let at = |table: &[T], id: &str| {
        table
            .iter()
            .position(|s| s.id() == id)
            .map(|i| (i, table[i].clone()))
    };
    let mut ids: Vec<&str> = before.iter().map(Styled::id).collect();
    ids.extend(
        after
            .iter()
            .map(Styled::id)
            .filter(|id| !before.iter().any(|s| s.id() == *id)),
    );
    ids.into_iter()
        .filter_map(|id| {
            let (b, a) = (at(before, id), at(after, id));
            let same = b.as_ref().map(|(_, s)| s) == a.as_ref().map(|(_, s)| s);
            (!same).then(|| Change {
                id: id.to_owned(),
                before: b,
                after: a,
            })
        })
        .collect()
}

/// Brings each style to its state after the step, while it still is as before the step (or already as after).
/// Removals first, then changes in place, then the styles that come back in the order of their places, so each
/// takes its old place. A change that would leave the table invalid (`problem`) is left out.
fn apply<T: Styled>(
    table: &mut Vec<T>,
    changes: &[Change<T>],
    problem: impl Fn(&[T]) -> Option<String>,
) {
    let mut order: Vec<&Change<T>> = changes.iter().collect();
    order.sort_by_key(|c| match (&c.before, &c.after) {
        (_, None) => (0, 0),
        (Some(_), Some(_)) => (1, 0),
        (None, Some((at, _))) => (2, *at),
    });
    for change in order {
        let now = table.iter().position(|s| s.id() == change.id);
        let current = now.map(|i| &table[i]);
        if current == change.after.as_ref().map(|(_, s)| s) {
            continue;
        }
        if current != change.before.as_ref().map(|(_, s)| s) {
            // Changed since (another editor): theirs stays.
            continue;
        }
        let mut next = table.clone();
        match (now, &change.after) {
            (Some(i), None) => {
                next.remove(i);
            }
            (Some(i), Some((_, style))) => next[i] = style.clone(),
            (None, Some((at, style))) => next.insert((*at).min(next.len()), style.clone()),
            (None, None) => {}
        }
        if problem(&next).is_none() {
            *table = next;
        }
    }
}

impl StyleTables {
    /// What changed between two settings' style tables; none: nothing.
    pub(crate) fn between(before: &ProjectSettings, after: &ProjectSettings) -> Option<Self> {
        let tables = Self {
            text: changes(&before.text_styles, &after.text_styles),
            dimension: changes(&before.dimension_styles, &after.dimension_styles),
            leader: changes(&before.leader_styles, &after.leader_styles),
            wall: changes(&before.wall_styles, &after.wall_styles),
            table: changes(&before.table_styles, &after.table_styles),
        };
        (tables != Self::default()).then_some(tables)
    }

    pub(crate) fn inverse(&self) -> Self {
        Self {
            text: self.text.iter().map(Change::inverse).collect(),
            dimension: self.dimension.iter().map(Change::inverse).collect(),
            leader: self.leader.iter().map(Change::inverse).collect(),
            wall: self.wall.iter().map(Change::inverse).collect(),
            table: self.table.iter().map(Change::inverse).collect(),
        }
    }

    /// The tables as the step left them, style by style.
    pub(crate) fn apply_to(&self, settings: &mut ProjectSettings) {
        // Capture only renames that can actually apply. record() can replay a
        // step already materialized by its writer: never rename its references twice.
        let renames: Vec<_> = self
            .text
            .iter()
            .filter_map(|change| {
                let (Some((_, before)), Some((_, after))) = (&change.before, &change.after) else {
                    return None;
                };
                (before.name != after.name && settings.text_styles.iter().any(|s| s == before))
                    .then_some((before.name.as_str(), after))
            })
            .collect();
        // A valid name swap has an invalid intermediate duplicate. Validate the
        // whole eligible text-table change, falling back to the existing conflict
        // policy when concurrent edits make that table invalid.
        let mut next = settings.text_styles.clone();
        apply(&mut next, &self.text, |_| None);
        if text_styles_problem(&next).is_none() {
            settings.text_styles = next;
        } else {
            apply(&mut settings.text_styles, &self.text, text_styles_problem);
        }
        let renames: Vec<_> = renames
            .into_iter()
            .filter(|(_, after)| settings.text_styles.iter().any(|s| s == *after))
            .collect();
        for pattern in &mut settings.line_types {
            for e in &mut pattern.elements {
                if let Some(t) = &mut e.text
                    && let Some((_, after)) = renames
                        .iter()
                        .find(|(before, _)| t.style.eq_ignore_ascii_case(before))
                {
                    t.style.clone_from(&after.name);
                }
            }
        }

        apply(
            &mut settings.dimension_styles,
            &self.dimension,
            dimension_styles_problem,
        );
        apply(&mut settings.leader_styles, &self.leader, leader_styles_problem);
        apply(&mut settings.wall_styles, &self.wall, wall_styles_problem);
        apply(&mut settings.table_styles, &self.table, table_style_defs_problem);
    }
}

impl Document {
    /// Runs `body`, which may rewrite the style tables (through `set_settings`) and the objects that follow them,
    /// and records the tables' change in the step, so undo and redo take the tables with the objects
    /// (docs/adr/0699). Outside a step the change is its own undo step named `label`.
    pub fn with_style_tables<T>(&mut self, label: &str, body: impl FnOnce(&mut Self) -> T) -> T {
        let before = self.settings.clone();
        let result = body(self);
        if let Some(tables) = StyleTables::between(&before, &self.settings) {
            self.record(vec![Op::StyleTables(Box::new(tables))], label);
        }
        result
    }
}
