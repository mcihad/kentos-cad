//! What shows and what is chosen (docs/adr/8061 §5 "Görünürlük", docs/adr/8073 §2): objects hidden, the objects the
//! view is isolated to, the spaces on or off, the chosen object and the found ones; and from them the scene's source
//! meshes, its box, its legend and its counts. An object hides with its parent and shows within an isolated one; a
//! walk up the tree stops after as many steps as there are objects, so a cycle the source left cannot hang it.

use std::collections::BTreeSet;
use std::sync::Arc;

use kentos_scene3d::Aabb;

use crate::input::{Category, Input};
use crate::parts::{self, PARTS, Part};

/// A source mesh for the scene: its part, what a click on it says, its object, its corners and triangles, and whether a
/// click goes through it (a space, an opening: met only when nothing behind it is).
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub part: Part,
    pub name: String,
    pub object: usize,
    pub vertices: Arc<Vec<[f64; 3]>>,
    pub triangles: Vec<[u32; 3]>,
    pub through: bool,
}

/// The viewer's state over a model.
#[derive(Debug, Clone)]
pub struct View {
    input: Arc<Input>,
    children: Vec<Vec<usize>>,
    hidden: BTreeSet<usize>,
    isolated: Option<BTreeSet<usize>>,
    /// Whether the spaces show.
    pub spaces: bool,
    /// Whether the openings show (see-through volumes where they cut their hosts).
    pub openings: bool,
    selected: Option<usize>,
    found: BTreeSet<usize>,
}

impl View {
    /// A view of `input`, everything showing but the openings, the spaces too, nothing chosen.
    pub fn new(input: Input) -> View {
        let mut children = vec![Vec::new(); input.objects.len()];
        for (i, o) in input.objects.iter().enumerate() {
            if let Some(p) = o.parent.filter(|&p| p < children.len() && p != i) {
                children[p].push(i);
            }
        }
        View {
            input: Arc::new(input),
            children,
            hidden: BTreeSet::new(),
            isolated: None,
            spaces: true,
            openings: false,
            selected: None,
            found: BTreeSet::new(),
        }
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    /// An object's children in the tree, in the source's order.
    pub fn children(&self, object: usize) -> &[usize] {
        self.children.get(object).map_or(&[], Vec::as_slice)
    }

    /// The tree's roots: objects with no parent (a parent out of range counts as none).
    pub fn roots(&self) -> Vec<usize> {
        let n = self.input.objects.len();
        (0..n).filter(|&i| self.input.objects[i].parent.is_none_or(|p| p >= n || p == i)).collect()
    }

    /// `object` and its ancestors, nearest first, at most as many as there are objects.
    fn line(&self, object: usize) -> impl Iterator<Item = usize> + '_ {
        let n = self.input.objects.len();
        std::iter::successors(Some(object), move |&i| self.input.objects.get(i).and_then(|o| o.parent).filter(|&p| p < n))
            .take(n.max(1))
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Chooses an object (none: nothing chosen).
    pub fn select(&mut self, object: Option<usize>) {
        self.selected = object.filter(|&i| i < self.input.objects.len());
    }

    /// The objects a check found something about, painted over their category's colour.
    pub fn set_found(&mut self, found: impl IntoIterator<Item = usize>) {
        self.found = found.into_iter().collect();
    }

    /// The objects painted as found.
    pub fn found(&self) -> &BTreeSet<usize> {
        &self.found
    }

    /// Whether `object` itself is hidden (not through its parent).
    pub fn is_hidden(&self, object: usize) -> bool {
        self.hidden.contains(&object)
    }

    /// Whether the view is isolated to some objects.
    pub fn is_isolated(&self) -> bool {
        self.isolated.is_some()
    }

    /// Whether `object` shows: neither it nor an ancestor hidden, it or an ancestor among the isolated when the view is
    /// isolated, a space only while the spaces show and an opening only while the openings do.
    pub fn shows(&self, object: usize) -> bool {
        let Some(o) = self.input.objects.get(object) else { return false };
        if (o.category == Category::Space && !self.spaces) || (o.category == Category::Opening && !self.openings) {
            return false;
        }
        if self.line(object).any(|i| self.hidden.contains(&i)) {
            return false;
        }
        match &self.isolated {
            Some(only) => self.line(object).any(|i| only.contains(&i)) || self.below(object, only),
            None => true,
        }
    }

    /// Whether an isolated object is below `object` (a storey shows while one of its walls is isolated, so the tree
    /// can reach it).
    fn below(&self, object: usize, only: &BTreeSet<usize>) -> bool {
        only.iter().any(|&i| i != object && self.line(i).skip(1).any(|a| a == object))
    }

    /// Hides `objects` (and what is under them).
    pub fn hide(&mut self, objects: impl IntoIterator<Item = usize>) {
        self.hidden.extend(objects);
    }

    /// Shows `object` again when it was hidden itself (one hidden with its parent stays hidden with it).
    pub fn unhide(&mut self, object: usize) {
        self.hidden.remove(&object);
    }

    /// The objects above `object` in the tree, nearest first.
    pub fn ancestors(&self, object: usize) -> Vec<usize> {
        self.line(object).skip(1).collect()
    }

    /// The box of the bodies of `object` and of what is under it, whether they show or not; empty when none has one.
    pub fn bounds_under(&self, object: usize) -> Aabb {
        let mut out = Aabb::EMPTY;
        for b in self.input.bodies.iter().filter(|b| self.line(b.object).any(|a| a == object)) {
            for p in b.positions.iter() {
                for k in 0..3 {
                    out.min[k] = out.min[k].min(p[k]);
                    out.max[k] = out.max[k].max(p[k]);
                }
            }
        }
        out
    }

    /// Shows only `objects` and what is under them; nothing given: the view is not isolated any more.
    pub fn isolate(&mut self, objects: impl IntoIterator<Item = usize>) {
        let only: BTreeSet<usize> = objects.into_iter().collect();
        self.isolated = (!only.is_empty()).then_some(only);
    }

    /// Everything shows again (the spaces as they were).
    pub fn show_all(&mut self) {
        self.hidden.clear();
        self.isolated = None;
    }

    /// An object's part: found red, else chosen amber, else its category's; a see-through volume stays see-through in
    /// each.
    fn part_of(&self, object: usize) -> Part {
        let space = self.input.objects[object].category.is_volume();
        if self.found.contains(&object) {
            if space { parts::FOUND_SPACE } else { parts::FOUND }
        } else if self.selected == Some(object) {
            if space { parts::SELECTED_SPACE } else { parts::SELECTED }
        } else {
            parts::of(self.input.objects[object].category)
        }
    }

    /// The bodies that show, as the scene's source meshes.
    pub fn sources(&self) -> Vec<Source> {
        self.input
            .bodies
            .iter()
            .filter(|b| self.shows(b.object))
            .map(|b| Source {
                part: self.part_of(b.object),
                name: self.input.objects[b.object].label.clone(),
                object: b.object,
                vertices: b.positions.clone(),
                triangles: b.triangles.clone(),
                through: self.input.objects[b.object].category.is_volume(),
            })
            .collect()
    }

    /// The box of the bodies that show; empty when none does.
    pub fn bounds(&self) -> Aabb {
        let mut out = Aabb::EMPTY;
        for b in self.input.bodies.iter().filter(|b| self.shows(b.object)) {
            for p in b.positions.iter() {
                for k in 0..3 {
                    out.min[k] = out.min[k].min(p[k]);
                    out.max[k] = out.max[k].max(p[k]);
                }
            }
        }
        out
    }

    /// The bodies that show, and all of them; the openings' volumes are not counted (they are the holes in the
    /// bodies, not bodies of their own).
    pub fn counts(&self) -> (usize, usize) {
        let bodies = || self.input.bodies.iter().filter(|b| self.input.objects[b.object].category != Category::Opening);
        (bodies().filter(|b| self.shows(b.object)).count(), bodies().count())
    }

    /// The legend: the parts the scene shows, in the parts' order.
    pub fn legend(&self) -> Vec<(&'static str, [f32; 4])> {
        let used: BTreeSet<Part> =
            self.input.bodies.iter().filter(|b| self.shows(b.object)).map(|b| self.part_of(b.object)).collect();
        used.into_iter().map(|p| PARTS[p as usize]).collect()
    }

    /// The object whose key is `key`.
    pub fn find(&self, key: &str) -> Option<usize> {
        self.input.objects.iter().position(|o| o.key == key)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::input::{BodyIn, ObjectIn};

    /// A project, a storey, two walls on it and a space; the walls and the space with bodies.
    pub(crate) fn small() -> Input {
        let object = |key: &str, category, parent| ObjectIn {
            key: key.into(),
            class: format!("{category:?}"),
            label: key.into(),
            category,
            parent,
            storey: None,
        };
        let cube = |at: f64| Arc::new(vec![[at, 0.0, 0.0], [at + 1.0, 0.0, 0.0], [at, 1.0, 0.0], [at, 0.0, 1.0]]);
        let tri = vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]];
        Input {
            title: "küçük".into(),
            objects: vec![
                object("proje", Category::Project, None),
                object("kat", Category::Storey, Some(0)),
                object("duvar-1", Category::Wall, Some(1)),
                object("duvar-2", Category::Wall, Some(1)),
                object("oda", Category::Space, Some(1)),
            ],
            bodies: vec![
                BodyIn { object: 2, positions: cube(0.0), triangles: tri.clone() },
                BodyIn { object: 3, positions: cube(5.0), triangles: tri.clone() },
                BodyIn { object: 4, positions: cube(10.0), triangles: tri },
            ],
            not_shown: Vec::new(),
        }
    }

    #[test]
    fn hiding_isolating_and_the_spaces_decide_what_shows() {
        let mut v = View::new(small());
        assert_eq!(v.counts(), (3, 3));
        assert_eq!(v.roots(), [0]);
        assert_eq!(v.children(1), [2, 3, 4]);
        v.hide([2]);
        assert!(!v.shows(2) && v.shows(3));
        v.hide([1]);
        assert_eq!(v.counts().0, 0, "hidden with their storey");
        v.show_all();
        v.isolate([3]);
        assert!(v.shows(3) && !v.shows(2) && !v.shows(4));
        assert!(v.shows(1) && v.shows(0), "the way to it in the tree shows");
        v.isolate([]);
        assert!(!v.is_isolated());
        v.hide([2]);
        v.unhide(2);
        assert!(v.shows(2));
        assert_eq!(v.ancestors(2), [1, 0]);
        assert_eq!(v.bounds_under(1).max[0], 11.0, "the storey's box holds its walls and its space");
        assert_eq!(v.bounds_under(3).min[0], 5.0);
        v.spaces = false;
        assert_eq!(v.counts(), (2, 3));
        assert_eq!(v.bounds().max[0], 6.0, "the spaces off, the box is the walls'");
    }

    #[test]
    fn the_chosen_and_the_found_are_painted_a_space_see_through() {
        let mut v = View::new(small());
        v.select(Some(2));
        v.set_found([4]);
        let parts: Vec<(usize, Part)> = v.sources().iter().map(|s| (s.object, s.part)).collect();
        assert_eq!(parts, [(2, parts::SELECTED), (3, 0), (4, parts::FOUND_SPACE)]);
        assert!(v.sources()[2].through);
        let names: Vec<&str> = v.legend().iter().map(|l| l.0).collect();
        assert_eq!(names, ["Duvar", "Seçili", "Bulgulu mahal"]);
        assert_eq!(v.find("duvar-2"), Some(3));
    }

    #[test]
    fn a_cycle_the_source_left_cannot_hang_the_view() {
        let mut input = small();
        input.objects[0].parent = Some(2);
        let mut v = View::new(input);
        v.hide([0]);
        assert!(!v.shows(3));
        assert!(v.roots().is_empty(), "no root in a cycle");
    }
}
