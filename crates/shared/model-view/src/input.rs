//! What an adapter gives the viewer: the model's objects and bodies (docs/adr/8073 §2).

use std::sync::Arc;

/// What an object is to the viewer: its part's colour, whether it is a place in the tree (project, site, building,
/// storey), a see-through volume (a space) or a body among bodies. The adapter decides it from the source's class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Project,
    Site,
    Building,
    Storey,
    Space,
    /// A spatial zone (an emsal area, a fire compartment's shape; docs/adr/8123): a see-through volume like a space's,
    /// hidden until asked for.
    Zone,
    Wall,
    Slab,
    Column,
    Beam,
    /// A footing (docs/adr/8172).
    Footing,
    /// A mass (docs/adr/8178): a massing study's prism, beside the drawing's model (the IFC holds none).
    Mass,
    Roof,
    Stair,
    Railing,
    Door,
    Window,
    Furnishing,
    /// An opening that cuts its host: a see-through volume, hidden until asked for.
    Opening,
    Other,
}

impl Category {
    /// Whether it is a place of the spatial structure, not a body among bodies.
    pub fn is_place(self) -> bool {
        matches!(self, Category::Project | Category::Site | Category::Building | Category::Storey)
    }

    /// Whether it is a see-through volume a click goes through (a space, a zone, an opening).
    pub fn is_volume(self) -> bool {
        matches!(self, Category::Space | Category::Zone | Category::Opening)
    }
}

/// An object: a key that stays (a GlobalId), the class as the source names it, what the tree says of it, its category,
/// its parent in the tree (parts, containment) and the storey it is on, as indices into [`Input::objects`].
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectIn {
    pub key: String,
    pub class: String,
    pub label: String,
    pub category: Category,
    pub parent: Option<usize>,
    pub storey: Option<usize>,
}

/// An object's body: its corners in world metres (float64) and its triangles.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyIn {
    pub object: usize,
    pub positions: Arc<Vec<[f64; 3]>>,
    pub triangles: Vec<[u32; 3]>,
}

/// A model to view: its title, its objects, the bodies of those that have one, and the objects with a shape that have
/// none, with why.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Input {
    pub title: String,
    pub objects: Vec<ObjectIn>,
    pub bodies: Vec<BodyIn>,
    pub not_shown: Vec<(usize, String)>,
    /// The first object of another model appended after the source's own ([`Input::append`]: an underlay beside the
    /// drawing's model); none: one model.
    pub apart: Option<usize>,
}

impl Input {
    /// `other` after this input: its objects, bodies and objects not shown, their indices moved past this input's and
    /// their keys under `prefix` (an underlay's model beside the drawing's, docs/adr/8119: two files' GlobalIds stay
    /// apart). Its roots stay roots of their own; the first appended object is where [`Input::apart`] starts.
    pub fn append(&mut self, other: Input, prefix: &str) {
        let base = self.objects.len();
        self.apart.get_or_insert(base);
        self.objects.extend(other.objects.into_iter().map(|mut o| {
            o.key = format!("{prefix}{}", o.key);
            o.parent = o.parent.map(|p| p + base);
            o.storey = o.storey.map(|s| s + base);
            o
        }));
        self.bodies.extend(other.bodies.into_iter().map(|mut b| {
            b.object += base;
            b
        }));
        self.not_shown.extend(other.not_shown.into_iter().map(|(o, why)| (o + base, why)));
    }

    /// Appends `other`'s objects and bodies as the source's own, not apart: what the drawing holds beside its file
    /// (its masses, docs/adr/8178), drawn and lit as its elements are. Before any [`Input::append`]: an appended
    /// model's objects stay apart after it.
    pub fn join(&mut self, other: Input, prefix: &str) {
        debug_assert!(self.apart.is_none(), "joined after an appended model");
        let apart = self.apart.take();
        self.append(other, prefix);
        self.apart = apart;
    }

    /// Whether `object` came from a model appended after the source's own.
    pub fn is_apart(&self, object: usize) -> bool {
        self.apart.is_some_and(|a| object >= a)
    }
}
