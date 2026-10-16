//! The session's digitizing locks (docs/adr/0166 §1): what holds the next
//! point, when the locks go, and how they read. The arithmetic is the
//! geometry core's (`tools::locks`); the web keeps the same state in
//! `apps/web/src/tools/locks.ts`.

use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::edges::nearest_edge;
use kentos_geometry_core::tools::locks::{
    Direction, Locks, coordinate_point, deflected, direction_of, edge_direction, lock_point,
    perpendicular,
};
use kentos_geometry_core::tools::point_input::Angles;
use kentos_geometry_core::tools::ucs::Frame;

use crate::Vec2;
use crate::format::Format;
use crate::spatial::record;
use crate::tool::Context;

/// Where a locked direction comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Toward {
    /// An angle typed in the project's way and unit: Açı, a CBS project's Semt.
    Angle(f64),
    /// Turned from the previous edge's direction, the project's way: Sapma.
    Deflection(f64),
    /// Along a picked edge, either way: Nesneye paralel.
    Parallel(Vec2),
    /// Square to a picked edge, either way: Nesneye dik.
    Perpendicular(Vec2),
}

/// Nesneye paralel or dik waiting for its edge (docs/adr/0166 §3): the next
/// press on the drawing picks it instead of reaching the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockPick {
    Parallel,
    Perpendicular,
}

impl LockPick {
    /// The step the prompt shows while the edge is awaited.
    pub fn step(self) -> &'static str {
        match self {
            LockPick::Parallel => "paralel kilidi için kenarı ya da yayı seçin",
            LockPick::Perpendicular => "dik kilidi için kenarı ya da yayı seçin",
        }
    }

    /// The direction lock the picked edge's direction `u` gives.
    pub fn toward(self, u: Vec2) -> Toward {
        match self {
            LockPick::Parallel => Toward::Parallel(u),
            LockPick::Perpendicular => Toward::Perpendicular(u),
        }
    }
}

/// What a press finds no edge under (§3).
pub const NO_LOCK_EDGE: &str = "Tıklanan yerde düz kenar ya da yay yok.";

/// The kinds whose edges lend a lock their direction: those a tangent circle
/// touches (the web's `LOCK_EDGE_KINDS`).
const EDGE_KINDS: [&str; 7] = [
    "line", "polyline", "polygon", "arc", "circle", "xline", "ray",
];

/// The edge under a press at `p` and its direction there (§3): the nearest
/// edge of the nearest visible line, polyline, area, arc, circle,
/// construction line or ray within the pick reach; none elsewhere.
pub fn picked_edge(p: Vec2, cx: &Context<'_>) -> Option<(Edge, Vec2)> {
    let slot = cx.spatial.pick_edge(p, cx.pick_tolerance(), |s| {
        cx.doc
            .get(s)
            .is_some_and(|e| EDGE_KINDS.contains(&e.kind()))
    })?;
    let edge = nearest_edge(&record(cx.doc.get(slot)?).3, p)?;
    let u = edge_direction(&edge, p)?;
    Some((edge, u))
}

/// What the value card asks for when a lock is chosen from the menu
/// (Uzunluk…, Açı…, Sapma…): its typed number locks that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockAsk {
    Length,
    Angle,
    Deflection,
    /// The east coordinate, in the locks' frame (docs/adr/0267 §1): a CAD project's X, a CBS project's Y.
    East,
    /// The north coordinate: a CAD project's Y, a CBS project's X.
    North,
}

/// What holds the next point: a length from the reference and a direction.
/// The session's, never saved; a new command starts with none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LockState {
    /// Metres from the reference.
    pub length: Option<f64>,
    pub toward: Option<Toward>,
    /// Kalıcı: the locks stay for the points after the next, until the command ends.
    pub keep: bool,
    /// The reference the locks were made at: when it moves (a point was
    /// placed), one-shot locks go and kept ones follow it.
    at: Option<Vec2>,
    /// The direction the running tool travels at its reference (its last
    /// edge's, a path's tangent): Sapma turns from it. The session fills it
    /// from the tool before each event.
    pub travel: Option<Vec2>,
    /// The edge a Paralel or Dik lock was picked on: drawn while it holds.
    pub edge: Option<Edge>,
    /// Referans noktası's or Yapım kipi's point (docs/adr/0166 §5): the
    /// cursor rule and typed relative points are measured from it instead
    /// of the tool's last corner. The session sets it before each event.
    pub reference: Option<Vec2>,
    /// The east and north coordinates the next point keeps (docs/adr/0267 §1), metres in [`Self::frame`].
    pub east: Option<f64>,
    pub north: Option<f64>,
    /// The frame the locks are in: the drawing's UCS, else the world (docs/adr/0261, 0267 §2). The session
    /// sets it before each event.
    pub frame: Frame,
}

impl LockState {
    /// Whether anything holds the next point.
    pub fn any(&self) -> bool {
        self.length.is_some() || self.toward.is_some() || self.coordinates()
    }

    /// Whether a coordinate is locked (docs/adr/0267 §1).
    pub fn coordinates(&self) -> bool {
        self.east.is_some() || self.north.is_some()
    }

    /// Locks the east or north coordinate (metres in the frame) at the reference `at`; a direction lock goes
    /// (the two are never held together, §1).
    pub fn lock_coordinate(&mut self, north: bool, metres: f64, at: Vec2) {
        if north {
            self.north = Some(metres);
        } else {
            self.east = Some(metres);
        }
        self.toward = None;
        self.edge = None;
        self.at = Some(at);
    }

    /// The point the locks hold from the reference `from` with the cursor at `c`, in the locks' frame
    /// (docs/adr/0267): the coordinates (with the length), else the direction (a typed angle from the frame's
    /// axes) and the length; none when they hold no point (a length the coordinate's line is beyond, the
    /// cursor on the reference with a length alone).
    pub fn point(&self, from: Vec2, c: Vec2, angles: Angles) -> Option<Vec2> {
        if self.coordinates() {
            let f = self.frame;
            let p = coordinate_point(
                f.to_frame(from),
                f.to_frame(c),
                self.east,
                self.north,
                self.length,
            )?;
            return Some(f.to_world(p));
        }
        let held = self.locks(angles);
        lock_point(from, c, held.length, held.direction)
    }

    /// The locked coordinates' lines in the world, as a point on each and its direction (docs/adr/0267 §1): the
    /// east's runs along the frame's y axis, the north's along its x axis; the guides the drawing dashes.
    pub fn coordinate_lines(&self, from: Vec2) -> Vec<(Vec2, Vec2)> {
        let f = self.frame;
        let o = f.to_frame(from);
        let mut out = Vec::new();
        if let Some(e) = self.east {
            out.push((
                f.to_world(Vec2::new(e, o.y)),
                f.dir_to_world(Vec2::new(0.0, 1.0)),
            ));
        }
        if let Some(n) = self.north {
            out.push((
                f.to_world(Vec2::new(o.x, n)),
                f.dir_to_world(Vec2::new(1.0, 0.0)),
            ));
        }
        out
    }

    /// Every lock goes; Kalıcı stays as it was (it is the command's).
    pub fn clear(&mut self) {
        self.length = None;
        self.toward = None;
        self.east = None;
        self.north = None;
        self.at = None;
        self.edge = None;
    }

    /// A new command: nothing locked, nothing kept.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Locks the length (metres) at the reference `at`.
    pub fn lock_length(&mut self, metres: f64, at: Vec2) {
        self.length = Some(metres);
        self.at = Some(at);
    }

    /// Locks the direction at the reference `at`; it takes the place of the
    /// one before, and of its picked edge.
    pub fn lock_toward(&mut self, toward: Toward, at: Vec2) {
        self.toward = Some(toward);
        self.at = Some(at);
        self.edge = None;
        // A direction and a coordinate are never held together (docs/adr/0267 §1).
        self.east = None;
        self.north = None;
    }

    /// The direction lock alone goes (its chip's ×), its picked edge with it.
    pub fn drop_toward(&mut self) {
        self.toward = None;
        self.edge = None;
    }

    /// The reference is now `from` (after a point was placed, a step taken
    /// back, a command ended): one-shot locks go, kept ones follow it.
    pub fn follow(&mut self, from: Option<Vec2>) {
        if !self.any() || from == self.at {
            return;
        }
        match from {
            Some(p) if self.keep => self.at = Some(p),
            _ => self.clear(),
        }
    }

    /// The direction locked for the next point, as the core takes it: an
    /// angle's, a deflection's from the travel direction (none without one),
    /// a picked edge's.
    pub fn direction(&self, angles: Angles) -> Option<Direction> {
        Some(match self.toward? {
            // A typed angle is from the frame's axes (docs/adr/0261 §4).
            Toward::Angle(a) => Direction {
                u: self.frame.dir_to_world(direction_of(a, angles)),
                both: false,
            },
            Toward::Deflection(a) => Direction {
                u: deflected(Vec2::new(0.0, 0.0), self.travel?, a, angles)?,
                both: false,
            },
            Toward::Parallel(u) => Direction { u, both: true },
            Toward::Perpendicular(u) => Direction {
                u: perpendicular(u),
                both: true,
            },
        })
    }

    /// The locks for the core's cursor rule.
    pub fn locks(&self, angles: Angles) -> Locks {
        Locks {
            length: self.length,
            direction: self.direction(angles),
        }
    }

    /// The locks as the value card's chips and the cursor's tag say them
    /// (“Uzunluk 12.500 m”, “Açı 45.0000°”, “Sapma 100.0000 g”, “Paralel”).
    pub fn words(&self, format: &Format) -> Vec<String> {
        let mut out = Vec::new();
        // What is measured in the frame says so when the frame is a UCS (docs/adr/0267 §2).
        let ucs = if self.frame.is_world() { "" } else { " (UCS)" };
        if let Some(l) = self.length {
            out.push(format!("Uzunluk {}", format.length(l)));
        }
        let (east, north) = (format.east_label(), format.north_label());
        if let Some(x) = self.east {
            out.push(format!("{east} {}{ucs}", format.length(x)));
        }
        if let Some(y) = self.north {
            out.push(format!("{north} {}{ucs}", format.length(y)));
        }
        match self.toward {
            Some(Toward::Angle(a)) => out.push(format!(
                "{} {}{ucs}",
                format.direction_name(),
                format.angle(format.angle_from_typed(a))
            )),
            Some(Toward::Deflection(a)) => out.push(format!(
                "Sapma {}",
                format.angle(format.angle_from_typed(a))
            )),
            Some(Toward::Parallel(_)) => out.push("Paralel".to_owned()),
            Some(Toward::Perpendicular(_)) => out.push("Dik".to_owned()),
            None => {}
        }
        if self.keep && self.any() {
            out.push("Kalıcı".to_owned());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A CBS project in grads.
    fn cbs() -> Format {
        Format::of(&kentos_contracts::ProjectSettings {
            line_type_scale: None,
            line_types: Vec::new(),
            length_style: None,
            named_ucs: Vec::new(),
            named_views: Vec::new(),
            forms: Vec::new(),
            ucs: None,
            srid: 5256,
            length_decimals: 3,
            area_decimals: 2,
            area_unit: kentos_contracts::AreaUnit::M2,
            angle_unit: kentos_contracts::AngleUnit::Grad,
            plot_scale: 1000.0,
            workspace: Some(kentos_contracts::Workspace::Gis),
            drawing_font: None,
            drawing_unit: None,
            second_srid: None,
            custom_crs: None,
            second_custom_crs: None,
            datum_transforms: Vec::new(),
            layer_states: Vec::new(),
            survey: None,
            dimension_styles: Vec::new(),
            topology: None,
            annotation: None,
            text_styles: Vec::new(),
            connections: Vec::new(),
            networks: Vec::new(),
            variables: Vec::new(),
            address_locators: Vec::new(),
        })
    }

    #[test]
    fn one_shot_locks_go_when_the_reference_moves_and_kept_ones_follow() {
        let mut s = LockState::default();
        let a = Vec2::new(1.0, 2.0);
        s.lock_length(12.0, a);
        s.follow(Some(a));
        assert_eq!(s.length, Some(12.0), "the same reference keeps them");
        s.follow(Some(Vec2::new(5.0, 2.0)));
        assert!(!s.any(), "a placed point lets them go");
        s.keep = true;
        s.lock_toward(Toward::Angle(100.0), a);
        s.follow(Some(Vec2::new(5.0, 2.0)));
        assert_eq!(s.toward, Some(Toward::Angle(100.0)), "kept ones follow");
        s.follow(None);
        assert!(!s.any(), "no reference: none");
        assert!(s.keep, "Kalıcı is the command's");
        s.reset();
        assert!(!s.keep);
    }

    #[test]
    fn a_deflection_needs_the_travel_direction() {
        let mut s = LockState::default();
        s.lock_toward(Toward::Deflection(100.0), Vec2::new(0.0, 0.0));
        let angles = cbs().angles();
        assert_eq!(s.direction(angles), None);
        s.travel = Some(Vec2::new(1.0, 0.0));
        let d = s.direction(angles).expect("a direction");
        // A CBS project turns clockwise: from east, 100 grads to the south.
        assert!(d.u.x.abs() < 1e-15 && (d.u.y + 1.0).abs() < 1e-15, "{d:?}");
        assert!(!d.both);
    }

    #[test]
    fn the_words_read_as_the_chips() {
        let mut s = LockState::default();
        s.lock_length(12.5, Vec2::new(0.0, 0.0));
        s.lock_toward(Toward::Angle(100.0), Vec2::new(0.0, 0.0));
        let f = cbs();
        let words = s.words(&f);
        assert_eq!(words[0], format!("Uzunluk {}", f.length(12.5)));
        assert_eq!(
            words[1],
            format!("Semt {}", f.angle(f.angle_from_typed(100.0)))
        );
    }
}
