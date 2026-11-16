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
    Wall,
    Slab,
    Column,
    Beam,
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

    /// Whether it is a see-through volume a click goes through (a space, an opening).
    pub fn is_volume(self) -> bool {
        matches!(self, Category::Space | Category::Opening)
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
}
