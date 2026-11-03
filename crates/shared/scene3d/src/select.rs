//! Which tiles to draw and which contents to ask for (plan/3d.md SCENE-STREAM-03, 04, 05, 08).
//!
//! The screen-space error of a tile is CesiumJS's (`Cesium3DTile.getScreenSpaceError`): in perspective the geometric
//! error times the viewport's height in pixels over the distance from the eye to the tile's bounds times
//! `2·tan(fov_y/2)` (the distance at least 1e-7 m, so a tile around the eye refines); in orthographic the geometric
//! error over a pixel's size. The error is the shown one: the vertical exaggeration scales it by its largest scale.
//!
//! The walk, from each root, skipping tiles outside the frustum:
//! - A tile without content of its own goes down to its children whatever its error.
//! - A tile whose error is within the limit, or that has no children, is drawn when its content is ready and asked
//!   for when it is not (then the place stays empty this frame: nothing coarser is there to stand in).
//! - A tile to refine by replacement goes down only when every child in view can cover its part now (its content
//!   ready, or no content and every child of it in view can cover); otherwise the tile is drawn in their place, the
//!   children not ready are asked for, and they replace it on a later frame (SCENE-STREAM-05: no hole while they come;
//!   SCENE-STREAM-04: never the parent and the children together).
//! - A tile refined additively is drawn with its children.
//! - The asks are ordered by their error, largest first, then by key: what the view misses most comes first.

use crate::camera::{Camera, Exaggeration, Projection, Viewport, Visibility};
use crate::tile::{ContentRef, Hierarchy, Refine};

/// The limit a tile's error is refined above, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub max_screen_error: f64,
}

/// What a frame draws and asks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selection {
    /// The tiles to draw (indices), in the walk's order.
    pub draw: Vec<u32>,
    /// The tiles whose contents to bring, the most needed first.
    pub wanted: Vec<u32>,
    /// How many tiles were looked at and how many of them were outside the frustum.
    pub visited: usize,
    pub culled: usize,
}

/// A tile's error on the screen, in pixels.
pub fn screen_error(geometric_error: f64, distance: f64, camera: &Camera, viewport: &Viewport) -> f64 {
    if geometric_error <= 0.0 {
        return 0.0;
    }
    match camera.projection {
        Projection::Perspective { fov_y, .. } => {
            geometric_error * viewport.height / (distance.max(1e-7) * 2.0 * (fov_y / 2.0).tan())
        }
        Projection::Orthographic { height, .. } => geometric_error / (height / viewport.height.max(1.0)),
    }
}

struct Walk<'a, F: Fn(&ContentRef) -> bool> {
    h: &'a Hierarchy,
    camera: &'a Camera,
    viewport: &'a Viewport,
    ex: &'a Exaggeration,
    frustum: crate::camera::Frustum,
    settings: Settings,
    ready: F,
    out: Selection,
    asks: Vec<(f64, u64, u32)>,
}

impl<F: Fn(&ContentRef) -> bool> Walk<'_, F> {
    fn shown_box(&self, i: u32) -> crate::tile::Aabb {
        self.ex.shown_box(&self.h.tiles[i as usize].bounds)
    }

    fn in_view(&self, i: u32) -> bool {
        self.frustum.test(&self.shown_box(i)) != Visibility::Outside
    }

    fn error(&self, i: u32) -> f64 {
        let t = &self.h.tiles[i as usize];
        let d = self.camera.distance_to(&self.shown_box(i));
        screen_error(t.geometric_error * self.ex.error_scale(), d, self.camera, self.viewport)
    }

    fn ask(&mut self, i: u32, error: f64) {
        self.asks.push((error, self.h.tiles[i as usize].key.0, i));
    }

    /// Whether a tile in view can cover its part now: its content ready, or none and every child in view can.
    fn can_cover(&self, i: u32) -> bool {
        let t = &self.h.tiles[i as usize];
        match &t.content {
            Some(c) => (self.ready)(c),
            None => t.children.iter().all(|&c| !self.in_view(c) || self.can_cover(c)),
        }
    }

    /// Asks for what would cover a tile's part: its content, or, without one, its children's in view.
    fn ask_cover(&mut self, i: u32) {
        let h = self.h;
        let t = &h.tiles[i as usize];
        match t.content {
            Some(c) => {
                if !(self.ready)(&c) {
                    let e = self.error(i);
                    self.ask(i, e);
                }
            }
            None => {
                for &c in &t.children {
                    if self.in_view(c) {
                        self.ask_cover(c);
                    }
                }
            }
        }
    }

    fn visit(&mut self, i: u32) {
        self.out.visited += 1;
        if !self.in_view(i) {
            self.out.culled += 1;
            return;
        }
        let h = self.h;
        let t = &h.tiles[i as usize];
        let error = self.error(i);
        let children = &t.children;
        let content = t.content;
        // A tile without content of its own has nothing to stand in for its children: it always goes down.
        let refine = !children.is_empty() && (error > self.settings.max_screen_error || content.is_none());
        let draw_own = |w: &mut Self| match content {
            Some(c) if (w.ready)(&c) => {
                w.out.draw.push(i);
                true
            }
            Some(_) => {
                w.ask(i, error);
                false
            }
            None => false,
        };
        if !refine {
            draw_own(self);
            return;
        }
        match t.refine {
            Refine::Add => {
                draw_own(self);
                for &c in children {
                    self.visit(c);
                }
            }
            Refine::Replace => {
                let all = children.iter().all(|&c| !self.in_view(c) || self.can_cover(c));
                if all || content.is_none() {
                    for &c in children {
                        self.visit(c);
                    }
                } else {
                    draw_own(self);
                    for &c in children {
                        if self.in_view(c) {
                            self.ask_cover(c);
                        }
                    }
                }
            }
        }
    }
}

/// The tiles to draw and the contents to ask for, for `camera` over `viewport`; `ready` says whether a content is on
/// the GPU now.
pub fn select<F: Fn(&ContentRef) -> bool>(
    h: &Hierarchy,
    camera: &Camera,
    viewport: &Viewport,
    ex: &Exaggeration,
    settings: Settings,
    ready: F,
) -> Selection {
    let mut w = Walk {
        h,
        camera,
        viewport,
        ex,
        frustum: camera.frustum(viewport.aspect()),
        settings,
        ready,
        out: Selection::default(),
        asks: Vec::new(),
    };
    for r in h.roots.clone() {
        w.visit(r);
    }
    w.asks.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    w.asks.dedup_by_key(|a| a.2);
    w.out.wanted = w.asks.iter().map(|a| a.2).collect();
    w.out
}
