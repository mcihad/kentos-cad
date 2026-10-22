//! One model-metre ↔ paper-micrometre ↔ pixel boundary (ADR 0595).
//! Drawing units are an input/output or presentation concern; they do not rescale this camera.
use crate::{
    GroundPoint,
    units::{Mdeg, RectUm, sin_cos},
};

pub const UM_PER_MM: f64 = crate::units::UM_PER_MM as f64;
pub const UM_PER_METRE: f64 = UM_PER_MM * 1000.;
pub fn paper_metres_to_um(metres: f64) -> f64 {
    metres * UM_PER_METRE
}
pub fn paper_um_to_metres(um: f64) -> f64 {
    um / UM_PER_METRE
}
pub fn paper_mm_to_um(mm: f64) -> f64 {
    mm * UM_PER_MM
}
pub fn paper_um_to_mm(um: f64) -> f64 {
    um / UM_PER_MM
}

/// A physical denominator: 50 means 1:50; 0.5 means 2:1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    denominator: f64,
}
impl Scale {
    pub const MIN: f64 = 1e-6;
    pub const MAX: f64 = 1e8;
    pub fn new(denominator: f64) -> Option<Self> {
        (denominator.is_finite() && (Self::MIN..=Self::MAX).contains(&denominator))
            .then_some(Self { denominator })
    }
    /// Render inputs come from a validated book. Retains the renderer's finite lower fallback;
    /// commands always use `new` and reject invalid ratios instead of silently repairing them.
    pub fn for_rendering(denominator: f64) -> Self {
        Self {
            denominator: denominator.max(Self::MIN),
        }
    }
    pub fn from_lengths(model_metres: f64, paper_mm: f64) -> Option<Self> {
        if !model_metres.is_finite()
            || !paper_mm.is_finite()
            || model_metres <= 0.
            || paper_mm <= 0.
        {
            return None;
        }
        Self::new(model_metres * 1000. / paper_mm)
    }
    pub fn denominator(self) -> f64 {
        self.denominator
    }
    pub fn paper_um(self, model_metres: f64) -> f64 {
        model_metres * UM_PER_METRE / self.denominator
    }
    pub fn model_metres(self, paper_um: f64) -> f64 {
        paper_um * self.denominator / UM_PER_METRE
    }
    pub fn paper_mm(self, model_metres: f64) -> f64 {
        model_metres * (UM_PER_METRE / UM_PER_MM) / self.denominator
    }
    pub fn model_metres_from_mm(self, paper_mm: f64) -> f64 {
        paper_mm * self.denominator / (UM_PER_METRE / UM_PER_MM)
    }
    pub fn pixels_per_metre(self, pixels_per_paper_um: f64) -> f64 {
        pixels_per_paper_um * UM_PER_METRE / self.denominator
    }
    pub fn paper_pixels_per_mm(self, pixels_per_model_metre: f64) -> f64 {
        pixels_per_model_metre * self.denominator / 1000.
    }
}

/// The denominator required by an axis-aligned model box in a turned paper camera.
/// The caller chooses exact/standard/rounded fit policy after this one physical calculation.
pub fn fitting_denominator(width_m: f64, height_m: f64, frame: &RectUm, rotation: Mdeg) -> f64 {
    let (s, c) = sin_cos(rotation);
    ((width_m * c.abs() + height_m * s.abs()) * UM_PER_METRE / f64::from(frame.width))
        .max((width_m * s.abs() + height_m * c.abs()) * UM_PER_METRE / f64::from(frame.height))
}

/// A centred planar camera. Output units are paper µm or pixels; north is up, output y is down.
/// World subtraction happens in f64 before rotation/scaling and before any host casts to f32.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    center: [f64; 2],
    anchor: [f64; 2],
    units_per_metre: f64,
    sin: f64,
    cos: f64,
}
impl Transform {
    /// Pixel/render callers supply their already validated positive resolution.
    pub fn at_resolution(
        center: GroundPoint,
        anchor: [f64; 2],
        units_per_metre: f64,
        rotation: Mdeg,
    ) -> Self {
        let (sin, cos) = sin_cos(rotation);
        Self {
            center: [center.x, center.y],
            anchor,
            units_per_metre,
            sin,
            cos,
        }
    }
    pub fn paper(frame: &RectUm, center: GroundPoint, denominator: f64, rotation: Mdeg) -> Self {
        Self::at_resolution(
            center,
            frame.center(),
            Scale::for_rendering(denominator).paper_um(1.),
            rotation,
        )
    }
    pub fn point(&self, world: [f64; 2]) -> [f64; 2] {
        let (a, b) = (
            (world[0] - self.center[0]) * self.units_per_metre,
            (world[1] - self.center[1]) * self.units_per_metre,
        );
        [
            self.anchor[0] + a * self.cos + b * self.sin,
            self.anchor[1] + a * self.sin - b * self.cos,
        ]
    }
    pub fn inverse(&self, plane: [f64; 2]) -> [f64; 2] {
        let (x, y) = (plane[0] - self.anchor[0], plane[1] - self.anchor[1]);
        [
            self.center[0] + (x * self.cos + y * self.sin) / self.units_per_metre,
            self.center[1] + (x * self.sin - y * self.cos) / self.units_per_metre,
        ]
    }
    /// Column-major affine for vertices relative to a host's local model origin.
    pub fn relative_affine(&self, origin: [f64; 2]) -> ([f64; 4], [f64; 2]) {
        (
            [
                self.cos * self.units_per_metre,
                self.sin * self.units_per_metre,
                self.sin * self.units_per_metre,
                -self.cos * self.units_per_metre,
            ],
            self.point(origin),
        )
    }
    pub fn ground_corners(&self, frame: &RectUm) -> [[f64; 2]; 4] {
        [
            [f64::from(frame.left), f64::from(frame.top)],
            [frame.right() as f64, f64::from(frame.top)],
            [frame.right() as f64, frame.bottom() as f64],
            [f64::from(frame.left), frame.bottom() as f64],
        ]
        .map(|p| self.inverse(p))
    }
    pub fn ground_bounds(&self, frame: &RectUm) -> [f64; 4] {
        let corners = self.ground_corners(frame);
        let mut bounds = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for p in corners {
            bounds = [
                bounds[0].min(p[0]),
                bounds[1].min(p[1]),
                bounds[2].max(p[0]),
                bounds[3].max(p[1]),
            ];
        }
        bounds
    }
}
