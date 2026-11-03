//! Tiles as 3D Tiles 1.1 has them (<https://github.com/CesiumGS/3d-tiles/blob/1.1/specification/README.adoc>):
//! a tile's bounding volume encloses its content and its children's; its geometric error is “the error, in meters,
//! of the tile's simplified representation” (0: the source itself); replacement refinement draws the children in
//! the tile's place, additive draws them with it. Here bounds are axis-aligned boxes in the source's coordinates
//! (f64), a content is referred to by an id the host and the renderer share, and its size in bytes is what the
//! residency budgets count (plan/3d.md SCENE-STREAM-02, 11).

use crate::math::{self, V3};

/// A tile's key: stable for the same source and the same cut (plan/3d.md §8 TileDescriptor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileKey(pub u64);

/// An axis-aligned box in the source's coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: V3,
    pub max: V3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb { min: [f64::INFINITY; 3], max: [f64::NEG_INFINITY; 3] };

    pub fn is_empty(&self) -> bool {
        (0..3).any(|a| self.min[a] > self.max[a])
    }

    pub fn grow(&mut self, p: V3) {
        self.min = std::array::from_fn(|a| self.min[a].min(p[a]));
        self.max = std::array::from_fn(|a| self.max[a].max(p[a]));
    }

    pub fn union(&self, o: &Aabb) -> Aabb {
        Aabb { min: std::array::from_fn(|a| self.min[a].min(o.min[a])), max: std::array::from_fn(|a| self.max[a].max(o.max[a])) }
    }

    pub fn center(&self) -> V3 {
        std::array::from_fn(|a| (self.min[a] + self.max[a]) / 2.0)
    }

    pub fn diagonal(&self) -> f64 {
        if self.is_empty() { 0.0 } else { math::length(math::sub(self.max, self.min)) }
    }

    /// The largest side.
    pub fn extent(&self) -> f64 {
        if self.is_empty() { 0.0 } else { (0..3).map(|a| self.max[a] - self.min[a]).fold(0.0, f64::max) }
    }

    /// Whether `o` lies inside, `tolerance` metres allowed.
    pub fn contains(&self, o: &Aabb, tolerance: f64) -> bool {
        (0..3).all(|a| o.min[a] >= self.min[a] - tolerance && o.max[a] <= self.max[a] + tolerance)
    }
}

/// How a tile's children refine it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refine {
    /// The children are drawn in the tile's place.
    Replace,
    /// The children are drawn with it.
    Add,
}

/// A tile's content: its id (the host's and the renderer's) and its size on the GPU, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContentRef {
    pub id: u64,
    pub bytes: u64,
}

/// A tile.
#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub key: TileKey,
    pub bounds: Aabb,
    pub geometric_error: f64,
    pub refine: Refine,
    pub content: Option<ContentRef>,
    /// Its children, as indices into the hierarchy's tiles.
    pub children: Vec<u32>,
}

/// Tiles and their roots.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hierarchy {
    pub tiles: Vec<Tile>,
    pub roots: Vec<u32>,
}

impl Hierarchy {
    /// What breaks the rules, the first: a child index out of the tiles, a tile with two parents or a cycle, a
    /// tile no root reaches, a negative or not finite geometric error, a child's error above its parent's, a child's
    /// bounds outside its parent's (1 µm allowed).
    pub fn problem(&self) -> Option<String> {
        let n = self.tiles.len();
        let mut parent: Vec<Option<u32>> = vec![None; n];
        for (i, t) in self.tiles.iter().enumerate() {
            if !(t.geometric_error >= 0.0 && t.geometric_error.is_finite()) {
                return Some(format!("{}. karonun geometrik hatası geçersiz", i));
            }
            for &c in &t.children {
                let c = c as usize;
                if c >= n {
                    return Some(format!("{}. karonun çocuğu {} yok", i, c));
                }
                if parent[c].is_some() || c == i {
                    return Some(format!("{}. karonun iki ebeveyni var", c));
                }
                parent[c] = Some(i as u32);
                let child = &self.tiles[c];
                if child.geometric_error > t.geometric_error {
                    return Some(format!("{}. karonun hatası ebeveyninkinden büyük", c));
                }
                if !t.bounds.contains(&child.bounds, 1e-6) {
                    return Some(format!("{}. karonun sınırları ebeveyninin dışında", c));
                }
            }
        }
        let mut seen = vec![false; n];
        let mut stack: Vec<u32> = self.roots.clone();
        for &r in &self.roots {
            if r as usize >= n || parent[r as usize].is_some() {
                return Some(format!("{} kök olamaz", r));
            }
        }
        while let Some(i) = stack.pop() {
            if std::mem::replace(&mut seen[i as usize], true) {
                return Some(format!("{}. karo iki kez ulaşılıyor", i));
            }
            stack.extend(&self.tiles[i as usize].children);
        }
        seen.iter().position(|s| !s).map(|i| format!("{}. karoya kökten ulaşılmıyor", i))
    }
}
