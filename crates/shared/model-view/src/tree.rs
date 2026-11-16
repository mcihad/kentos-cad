//! The tree panel's rows (docs/adr/8061 §5 "Yapı ağacı"): by structure (project, site, building, storey, then what each
//! holds) or by class (a row a class with its count, then its objects). Only open branches are walked, so a model of
//! many thousand objects gives as many rows as the panel shows open. A node is open by its id in the host's set: an
//! object's key, or `sınıf:` and the class.

use std::collections::{BTreeMap, BTreeSet};

use crate::view::View;

/// How the tree is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeMode {
    #[default]
    Structure,
    Class,
}

/// A row: its object (none: a class's row), its node id, how deep it is, its words, how many it holds, whether it is
/// open, shows and is chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub object: Option<usize>,
    pub id: String,
    pub depth: usize,
    pub label: String,
    pub count: usize,
    pub open: bool,
    pub shows: bool,
    pub chosen: bool,
}

/// A class's node id.
pub fn class_id(class: &str) -> String {
    format!("sınıf:{class}")
}

impl View {
    /// The tree's rows in `mode`, with the nodes in `open` open.
    pub fn rows(&self, mode: TreeMode, open: &BTreeSet<String>) -> Vec<Row> {
        let mut out = Vec::new();
        match mode {
            TreeMode::Structure => {
                let mut seen = BTreeSet::new();
                for r in self.roots() {
                    self.walk(r, 0, open, &mut seen, &mut out);
                }
            }
            TreeMode::Class => {
                let mut by: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
                for (i, o) in self.input().objects.iter().enumerate() {
                    by.entry(o.class.as_str()).or_default().push(i);
                }
                for (class, mut members) in by {
                    let id = class_id(class);
                    let is_open = open.contains(&id);
                    out.push(Row {
                        object: None,
                        id,
                        depth: 0,
                        label: class.to_owned(),
                        count: members.len(),
                        open: is_open,
                        shows: members.iter().any(|&i| self.shows(i)),
                        chosen: false,
                    });
                    if is_open {
                        members.sort_by(|a, b| self.input().objects[*a].label.cmp(&self.input().objects[*b].label));
                        out.extend(members.into_iter().map(|i| self.row(i, 1, open)));
                    }
                }
            }
        }
        out
    }

    fn row(&self, i: usize, depth: usize, open: &BTreeSet<String>) -> Row {
        let o = &self.input().objects[i];
        Row {
            object: Some(i),
            id: o.key.clone(),
            depth,
            label: o.label.clone(),
            count: self.children(i).len(),
            open: open.contains(&o.key),
            shows: self.shows(i),
            chosen: self.selected() == Some(i),
        }
    }

    fn walk(&self, i: usize, depth: usize, open: &BTreeSet<String>, seen: &mut BTreeSet<usize>, out: &mut Vec<Row>) {
        if !seen.insert(i) {
            return;
        }
        let row = self.row(i, depth, open);
        let go_in = row.open;
        out.push(row);
        if go_in {
            for &c in self.children(i) {
                self.walk(c, depth + 1, open, seen, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::View;

    fn view() -> View {
        View::new(crate::view::tests::small())
    }

    #[test]
    fn the_structure_opens_branch_by_branch() {
        let v = view();
        let closed: Vec<String> = v.rows(TreeMode::Structure, &BTreeSet::new()).into_iter().map(|r| r.id).collect();
        assert_eq!(closed, ["proje"]);
        let open: BTreeSet<String> = ["proje", "kat"].map(String::from).into();
        let rows = v.rows(TreeMode::Structure, &open);
        let ids: Vec<(&str, usize)> = rows.iter().map(|r| (r.id.as_str(), r.depth)).collect();
        assert_eq!(ids, [("proje", 0), ("kat", 1), ("duvar-1", 2), ("duvar-2", 2), ("oda", 2)]);
        assert_eq!(rows[1].count, 3);
    }

    #[test]
    fn by_class_a_row_a_class_with_its_count() {
        let mut v = view();
        v.select(Some(3));
        let open: BTreeSet<String> = [class_id("Wall")].into();
        let rows = v.rows(TreeMode::Class, &open);
        let words: Vec<(&str, usize, bool)> = rows.iter().map(|r| (r.label.as_str(), r.count, r.chosen)).collect();
        assert_eq!(
            words,
            [("Project", 1, false), ("Space", 1, false), ("Storey", 1, false), ("Wall", 2, false), ("duvar-1", 0, false), ("duvar-2", 0, true)]
        );
    }
}
