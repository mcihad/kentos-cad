//! The orbit camera of the 3D views (docs/adr/0910 §4, docs/adr/8067, docs/adr/8073 §2): about a point kept in the
//! source's heights, at an azimuth (degrees from east, counterclockwise) and an elevation; a drag orbits, a right drag
//! pans along the ground, the wheel dollies; the four views and Kuzeye bak set its angles; Sığdır puts the whole box in
//! view. The vertical exaggeration is the view's: the target is shown through it.

use kentos_scene3d::{Aabb, Camera, Exaggeration, Projection};

/// The view presets: name, azimuth and elevation.
pub const VIEWS: [(&str, f64, f64); 4] =
    [("Kuzeydoğu", 45.0, 30.0), ("Güneybatı", 225.0, 30.0), ("Kuzeybatı", 135.0, 30.0), ("Üst", -90.0, 89.0)];

/// The vertical field of view, degrees.
pub const FOV: f64 = 40.0;

/// The camera about a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    pub target: [f64; 3],
    pub azimuth: f64,
    pub elevation: f64,
    pub distance: f64,
}

impl OrbitCamera {
    /// Looking at `bounds`' middle from `azimuth` and `elevation`, from where all of it shows as `ex` shows it; `room`
    /// below 1 comes nearer (a flat, wide ground), 1 keeps all of a box that stands up in view.
    pub fn fitted(bounds: &Aabb, ex: &Exaggeration, azimuth: f64, elevation: f64, room: f64) -> OrbitCamera {
        let mut c = OrbitCamera { target: [0.0; 3], azimuth, elevation, distance: 1.0 };
        c.fit(bounds, ex, room);
        c
    }

    /// The whole of `bounds` in view at the camera's angles: the target its middle in the source's heights, the
    /// distance the shown box's.
    pub fn fit(&mut self, bounds: &Aabb, ex: &Exaggeration, room: f64) {
        if bounds.is_empty() {
            return;
        }
        self.target = bounds.center();
        self.distance = (ex.shown_box(bounds).diagonal() / 2.0).max(1.0) / (FOV.to_radians() / 2.0).sin() * room;
    }

    /// A drag of `dx`, `dy` logical pixels: about the target.
    pub fn orbit(&mut self, dx: f64, dy: f64) {
        self.azimuth -= dx * 0.4;
        self.elevation = (self.elevation + dy * 0.3).clamp(2.0, 89.0);
    }

    /// A right drag: along the ground, the target's distance a window's width across.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        let k = self.distance * (FOV.to_radians() / 2.0).tan() * 2.0 / 600.0;
        let a = self.azimuth.to_radians();
        let (right, ahead) = ([-a.sin(), a.cos()], [-a.cos(), -a.sin()]);
        self.target[0] -= (right[0] * dx - ahead[0] * dy) * k;
        self.target[1] -= (right[1] * dx - ahead[1] * dy) * k;
    }

    /// The wheel: `steps` notches nearer (negative: farther).
    pub fn dolly(&mut self, steps: f64) {
        self.distance = (self.distance * 0.85f64.powf(steps)).clamp(1.0, 1.0e6);
    }

    /// From the south, looking north, its height and distance kept.
    pub fn north(&mut self) {
        self.azimuth = -90.0;
    }

    /// At the `i`th of [`VIEWS`]' angles, all of `bounds` in view as `ex` shows it.
    pub fn look(&mut self, i: usize, bounds: &Aabb, ex: &Exaggeration, room: f64) {
        let (_, a, e) = VIEWS[i.min(VIEWS.len() - 1)];
        self.azimuth = a;
        self.elevation = e;
        self.fit(bounds, ex, room);
    }

    /// The view the camera is at (to half a degree), by its place in [`VIEWS`]; none once it has been turned.
    pub fn preset(&self) -> Option<usize> {
        VIEWS.iter().position(|&(_, a, e)| {
            let d = (self.azimuth - a).rem_euclid(360.0);
            (d < 0.5 || d > 359.5) && (self.elevation - e).abs() < 0.5
        })
    }

    /// Where north is on the screen, clockwise from up, as the north arrow turns. The camera looks along
    /// −(cos a, sin a) from its azimuth's side; north is (cos a, −sin a) in the screen's right and ahead.
    pub fn north_on_screen(&self) -> f32 {
        let a = self.azimuth.to_radians();
        a.cos().atan2(-a.sin()).to_degrees() as f32
    }

    /// The side the model is seen from, in words (“kuzeydoğudan”).
    pub fn seen_from(&self) -> &'static str {
        const SIDES: [&str; 8] =
            ["doğudan", "kuzeydoğudan", "kuzeyden", "kuzeybatıdan", "batıdan", "güneybatıdan", "güneyden", "güneydoğudan"];
        let k = ((self.azimuth.rem_euclid(360.0) + 22.5) / 45.0).floor() as usize % 8;
        SIDES[k]
    }

    /// Towards the light, over the viewer's left shoulder: the two faces seen are lit unlike each other from whichever
    /// side the model is seen.
    pub fn light(&self) -> [f64; 3] {
        let (a, e) = ((self.azimuth + 35.0).to_radians(), 50f64.to_radians());
        [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()]
    }

    /// The camera in the shown space.
    pub fn camera(&self, ex: &Exaggeration) -> Camera {
        let near = (self.distance * 1e-3).clamp(0.05, 5.0);
        Camera::orbit(ex.shown(self.target), self.azimuth, self.elevation, self.distance, Projection::Perspective {
            fov_y: FOV.to_radians(),
            near,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_orbits_pans_dollies_and_fits() {
        let b = Aabb { min: [0.0, 0.0, 0.0], max: [10.0, 8.0, 6.0] };
        let mut c = OrbitCamera::fitted(&b, &Exaggeration { factor: 1.0, datum: 0.0 }, 45.0, 30.0, 1.0);
        assert_eq!(c.target, [5.0, 4.0, 3.0]);
        let steep = OrbitCamera::fitted(&b, &Exaggeration { factor: 3.0, datum: 0.0 }, 45.0, 30.0, 1.0);
        assert_eq!(steep.target, c.target, "the target stays in the source's heights");
        assert!(steep.distance > c.distance, "the shown box is taller");
        let d = c.distance;
        c.orbit(50.0, 20.0);
        assert!((c.azimuth - 25.0).abs() < 1e-9 && (c.elevation - 36.0).abs() < 1e-9);
        c.orbit(0.0, 1000.0);
        assert_eq!(c.elevation, 89.0, "never over the top");
        c.dolly(2.0);
        assert!(c.distance < d);
        c.north();
        assert_eq!(c.azimuth, -90.0);
        let before = c.target;
        c.pan(100.0, 0.0);
        assert!(c.target != before && c.target[2] == before[2], "along the ground");
        let at = c.camera(&Exaggeration { factor: 1.0, datum: 0.0 });
        assert_eq!(at.target, c.target);
        c.look(1, &b, &Exaggeration { factor: 1.0, datum: 0.0 }, 1.0);
        assert_eq!((c.preset(), c.target), (Some(1), [5.0, 4.0, 3.0]));
        c.orbit(10.0, 0.0);
        assert_eq!(c.preset(), None, "turned: no view");
    }

    #[test]
    fn the_north_arrow_and_the_side_follow_the_camera() {
        let at = |azimuth: f64| OrbitCamera { target: [0.0; 3], azimuth, elevation: 30.0, distance: 10.0 };
        // From the north-east the model is seen with north behind and to the right: down right on the screen.
        assert!((at(45.0).north_on_screen() - 135.0).abs() < 1e-4);
        // From the south, looking north (Üst, Kuzeye bak): north is up.
        assert!(at(-90.0).north_on_screen().abs() < 1e-4);
        // From the south-west, looking north-east: north is up and to the left.
        assert!((at(225.0).north_on_screen() + 45.0).abs() < 1e-4);
        assert_eq!(at(45.0).seen_from(), "kuzeydoğudan");
        assert_eq!(at(225.0).seen_from(), "güneybatıdan");
        assert_eq!(at(-90.0).seen_from(), "güneyden");
        assert_eq!(at(359.0).seen_from(), "doğudan");
    }
}
