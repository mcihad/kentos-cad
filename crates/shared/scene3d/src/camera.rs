//! The 3D camera (plan/3d.md SCENE-CAM): its eye and target in f64 in the source's coordinates (x east, y north,
//! z up) as shown, perspective or orthographic, and what the GPU, the frustum test, the level of detail and a pick
//! take from it.
//!
//! - The GPU takes a translation-free matrix (the rotation and the projection: small numbers) and, per tile, the
//!   tile's origin minus the eye computed in f64 and only then made float32 (SCENE-PREC-05, 08): the vertex shader
//!   adds the tile's local position to it, so no large number is ever rounded.
//! - Depth is reversed (plan/3d.md SCENE-CAM-05, 06; Reed, “Depth Precision Visualized”): the near plane at 1, far at
//!   0, compared `Greater`, cleared to 0; perspective's far plane is at infinity, so a road and the mountains behind it
//!   share one float32 depth buffer without a far plane to choose.
//! - The vertical exaggeration is the view's alone (SCENE-CAM-14): heights shown are `datum + factor · (z − datum)`;
//!   the camera lives in that shown space, the sources keep their heights, a pick returns real ones.

use crate::math::{self, M4, V3};
use crate::tile::Aabb;

/// How the camera projects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    /// A perspective of `fov_y` radians from top to bottom, the near plane `near` metres before the eye, no far plane.
    Perspective { fov_y: f64, near: f64 },
    /// An orthographic view `height` metres from top to bottom, `depth` metres deep before the eye.
    Orthographic { height: f64, depth: f64 },
}

/// The viewport in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub width: f64,
    pub height: f64,
}

impl Viewport {
    pub fn aspect(&self) -> f64 {
        self.width.max(1.0) / self.height.max(1.0)
    }
}

/// The view's vertical exaggeration about a datum: a height `z` is shown at `datum + factor · (z − datum)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exaggeration {
    pub factor: f64,
    pub datum: f64,
}

impl Exaggeration {
    pub const NONE: Exaggeration = Exaggeration { factor: 1.0, datum: 0.0 };

    /// A source point as shown.
    pub fn shown(&self, p: V3) -> V3 {
        [p[0], p[1], self.datum + self.factor * (p[2] - self.datum)]
    }

    /// A shown point's real height.
    pub fn real(&self, p: V3) -> V3 {
        [p[0], p[1], self.datum + (p[2] - self.datum) / self.factor]
    }

    /// A source box as shown (the map is monotone in z, so a box stays a box).
    pub fn shown_box(&self, b: &Aabb) -> Aabb {
        let (lo, hi) = (self.shown(b.min), self.shown(b.max));
        Aabb { min: [lo[0], lo[1], lo[2].min(hi[2])], max: [hi[0], hi[1], lo[2].max(hi[2])] }
    }

    /// How much an error grows when shown: the largest scale of the map (3D Tiles 1.1 scales a tile's geometric error
    /// by its transform's largest scale the same way).
    pub fn error_scale(&self) -> f64 {
        self.factor.abs().max(1.0)
    }
}

/// Where a box is against the frustum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Outside,
    Intersecting,
    Inside,
}

/// The frustum as half-spaces `n · (p − eye) + d ≥ 0`, inside when every one holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Frustum {
    pub eye: V3,
    pub planes: Vec<(V3, f64)>,
}

impl Frustum {
    /// Where a box is: outside when one plane leaves all its corners out (the p-vertex test: conservative, a box it
    /// says is outside is outside), inside when every plane keeps all its corners in.
    pub fn test(&self, b: &Aabb) -> Visibility {
        let mut inside = true;
        for (n, d) in &self.planes {
            let far = std::array::from_fn(|a| if n[a] >= 0.0 { b.max[a] } else { b.min[a] });
            let near = std::array::from_fn(|a| if n[a] >= 0.0 { b.min[a] } else { b.max[a] });
            if math::dot(*n, math::sub(far, self.eye)) + d < 0.0 {
                return Visibility::Outside;
            }
            if math::dot(*n, math::sub(near, self.eye)) + d < 0.0 {
                inside = false;
            }
        }
        if inside { Visibility::Inside } else { Visibility::Intersecting }
    }
}

/// The camera: its eye, the point it looks at and its up, in the shown space's source coordinates, and how it
/// projects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub eye: V3,
    pub target: V3,
    pub up: V3,
    pub projection: Projection,
}

impl Camera {
    /// The camera `distance` from `target` at `azimuth` (degrees from east, counterclockwise) and `elevation` (degrees
    /// above the horizon), looking at it, z up.
    pub fn orbit(target: V3, azimuth: f64, elevation: f64, distance: f64, projection: Projection) -> Camera {
        let (a, e) = (azimuth.to_radians(), elevation.to_radians());
        let eye = [
            target[0] + distance * e.cos() * a.cos(),
            target[1] + distance * e.cos() * a.sin(),
            target[2] + distance * e.sin(),
        ];
        Camera { eye, target, up: [0.0, 0.0, 1.0], projection }
    }

    /// The camera's right, up and forward, of length one; looking straight along `up`, the right is east.
    pub fn basis(&self) -> (V3, V3, V3) {
        let f = math::normalize(math::sub(self.target, self.eye));
        let mut s = math::cross(f, self.up);
        if math::length(s) < 1e-9 {
            s = math::cross(f, [0.0, 1.0, 0.0]);
            if math::length(s) < 1e-9 {
                s = [1.0, 0.0, 0.0];
            }
        }
        let s = math::normalize(s);
        let u = math::cross(s, f);
        (s, u, f)
    }

    /// The projection times the rotation, without the eye's translation: what turns a point relative to the eye into
    /// clip space. Reversed depth: perspective's `z/w` is `near / depth` (1 at the near plane, 0 at infinity);
    /// orthographic's `1 − depth / depth_range`.
    pub fn rotation_projection(&self, aspect: f64) -> M4 {
        let (s, u, f) = self.basis();
        let rotation = M4::from_rows([
            [s[0], s[1], s[2], 0.0],
            [u[0], u[1], u[2], 0.0],
            [-f[0], -f[1], -f[2], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let projection = match self.projection {
            Projection::Perspective { fov_y, near } => {
                let g = 1.0 / (fov_y / 2.0).tan();
                M4::from_rows([[g / aspect, 0.0, 0.0, 0.0], [0.0, g, 0.0, 0.0], [0.0, 0.0, 0.0, near], [0.0, 0.0, -1.0, 0.0]])
            }
            Projection::Orthographic { height, depth } => {
                let (hh, hw) = (height / 2.0, height / 2.0 * aspect);
                M4::from_rows([
                    [1.0 / hw, 0.0, 0.0, 0.0],
                    [0.0, 1.0 / hh, 0.0, 0.0],
                    [0.0, 0.0, 1.0 / depth, 1.0],
                    [0.0, 0.0, 0.0, 1.0],
                ])
            }
        };
        projection.mul(&rotation)
    }

    /// A tile's origin relative to the eye, both in f64 in the shown space, then made float32: what the GPU adds a
    /// tile's local (float32) position to (`shown(origin + local) − eye = local · (1, 1, factor) + this`).
    pub fn offset(&self, origin: V3, ex: &Exaggeration) -> [f32; 3] {
        math::sub(ex.shown(origin), self.eye).map(|v| v as f32)
    }

    /// The frustum for `aspect`: the near plane and the four sides; perspective has no far plane, orthographic's is
    /// `depth` before the eye.
    pub fn frustum(&self, aspect: f64) -> Frustum {
        let (s, u, f) = self.basis();
        let planes = match self.projection {
            Projection::Perspective { fov_y, near } => {
                let ty = (fov_y / 2.0).tan();
                let tx = ty * aspect;
                vec![
                    (f, -near),
                    (math::add(s, math::scale(f, tx)), 0.0),
                    (math::add(math::scale(s, -1.0), math::scale(f, tx)), 0.0),
                    (math::add(u, math::scale(f, ty)), 0.0),
                    (math::add(math::scale(u, -1.0), math::scale(f, ty)), 0.0),
                ]
            }
            Projection::Orthographic { height, depth } => {
                let (hh, hw) = (height / 2.0, height / 2.0 * aspect);
                vec![
                    (f, 0.0),
                    (math::scale(f, -1.0), depth),
                    (s, hw),
                    (math::scale(s, -1.0), hw),
                    (u, hh),
                    (math::scale(u, -1.0), hh),
                ]
            }
        };
        Frustum { eye: self.eye, planes }
    }

    /// The ray through a place of the viewport (pixels from its top left, fractional): its origin and direction (of
    /// length one) in the shown space.
    pub fn ray(&self, x: f64, y: f64, viewport: &Viewport) -> (V3, V3) {
        let (s, u, f) = self.basis();
        let nx = 2.0 * x / viewport.width.max(1.0) - 1.0;
        let ny = 1.0 - 2.0 * y / viewport.height.max(1.0);
        match self.projection {
            Projection::Perspective { fov_y, .. } => {
                let ty = (fov_y / 2.0).tan();
                let tx = ty * viewport.aspect();
                let d = math::add(f, math::add(math::scale(s, nx * tx), math::scale(u, ny * ty)));
                (self.eye, math::normalize(d))
            }
            Projection::Orthographic { height, .. } => {
                let (hh, hw) = (height / 2.0, height / 2.0 * viewport.aspect());
                let o = math::add(self.eye, math::add(math::scale(s, nx * hw), math::scale(u, ny * hh)));
                (o, f)
            }
        }
    }

    /// A shown point's place in the viewport (pixels from its top left) and its reversed depth, none behind the eye.
    pub fn project(&self, p: V3, viewport: &Viewport) -> Option<(f64, f64, f64)> {
        let m = self.rotation_projection(viewport.aspect());
        let c = m.apply(math::sub(p, self.eye));
        if c[3] <= 0.0 {
            return None;
        }
        let (x, y, z) = (c[0] / c[3], c[1] / c[3], c[2] / c[3]);
        Some(((x + 1.0) / 2.0 * viewport.width, (1.0 - y) / 2.0 * viewport.height, z))
    }

    /// The distance from the eye to a box (0 inside it).
    pub fn distance_to(&self, b: &Aabb) -> f64 {
        let d: V3 = std::array::from_fn(|a| (b.min[a] - self.eye[a]).max(self.eye[a] - b.max[a]).max(0.0));
        math::length(d)
    }
}
