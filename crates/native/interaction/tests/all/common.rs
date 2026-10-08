//! The tools' test bench: the session over a drawing (the traces' empty one
//! unless a test gives another), seen through the traces' view (0.125 m per
//! pixel around (E, N) on an 800 × 600 area), with the geometry store and
//! the selection the desktop keeps beside it. Points are given as east and
//! north differences from (E, N), as in the traces; clicks and moves snap as
//! the desktop snaps them (docs/adr/0029).
#![allow(dead_code)]

use kentos_contracts::{DocumentSnapshotV1, Entity};
use kentos_domain::Document;
use kentos_interaction::object_tracking::ObjectTracking;
use kentos_interaction::{
    Context, Draft, Level, Line, LockState, Memory, Pointer, Selection, Session, Spatial, Vec2,
    View, ViewChange,
};

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");
pub const E: f64 = 487000.0;
pub const N: f64 = 4420000.0;

/// The traces' view: 0.125 m per pixel around (E, N), an 800 × 600 area.
pub struct Camera;

impl View for Camera {
    fn to_screen(&self, p: Vec2) -> [f64; 2] {
        [(p.x - E) * 8.0 + 400.0, 300.0 - (p.y - N) * 8.0]
    }

    fn world_length(&self, px: f64) -> f64 {
        px / 8.0
    }

    fn visible(&self) -> kentos_geometry_core::geometry::Bounds {
        kentos_geometry_core::geometry::Bounds {
            min_x: E - 50.0,
            min_y: N - 37.5,
            max_x: E + 50.0,
            max_y: N + 37.5,
        }
    }
}

pub struct Bench {
    pub doc: Document,
    pub session: Session,
    pub log: Vec<Line>,
    pub draft: Draft,
    pub spatial: Spatial,
    pub selection: Selection,
    /// What the drawing tools remember between runs (the web's static fields).
    pub memory: Memory,
    /// Shift held for the next pointer events.
    pub shift: bool,
    /// The view changes the tools asked for, oldest first (the desktop applies them).
    pub views: Vec<ViewChange>,
    /// Object tracking (docs/adr/0085): off in these tests unless one acquires a point.
    pub tracking: ObjectTracking,
    /// Seçili katmanlarda önle's layers (docs/adr/0162 §1).
    pub overlap_layers: Vec<String>,
    /// The digitizing locks (docs/adr/0166).
    pub locks: LockState,
    /// The object template drawn with (docs/adr/0176 §3): what new objects take.
    pub template: Option<kentos_interaction::templates::Stamp>,
    /// What Katmanı yalıt hid (docs/adr/0177 §1).
    pub isolated_layers: Vec<String>,
}

impl Bench {
    /// `tool` running on the empty drawing, snapping off (the traces' default).
    pub fn new(tool: &str) -> Self {
        let mut b = Self::on(EMPTY);
        b.draft.snap = false;
        b.start(tool);
        b
    }

    /// No command running on `drawing` (a `.kcad` v1 text): the select tool has the pointer.
    pub fn on(drawing: &str) -> Self {
        let snapshot = DocumentSnapshotV1::from_json(drawing).expect("the drawing reads");
        let doc = Document::from_snapshot(snapshot).expect("opens");
        Self {
            spatial: Spatial::of(&doc),
            doc,
            session: Session::new(),
            log: Vec::new(),
            draft: Draft::default(),
            selection: Selection::new(),
            memory: Memory::default(),
            shift: false,
            overlap_layers: Vec::new(),
            views: Vec::new(),
            tracking: ObjectTracking::new(),
            locks: LockState::default(),
            template: None,
            isolated_layers: Vec::new(),
        }
    }

    /// Starts a tool as the desktop does: start, then activate.
    pub fn start(&mut self, tool: &str) {
        assert!(self.session.start(tool), "{tool} is a session tool");
        self.run(|s, cx| s.activate(cx));
    }

    pub fn run<T>(&mut self, act: impl FnOnce(&mut Session, &mut Context<'_>) -> T) -> T {
        self.spatial.sync(&self.doc);
        let mut cx = Context {
            doc: &mut self.doc,
            view: &Camera,
            draft: self.draft,
            log: &mut self.log,
            spatial: &self.spatial,
            selection: &mut self.selection,
            memory: &mut self.memory,
            view_changes: &mut self.views,
            tracking: &self.tracking,
            shift: self.shift,
            overlap_layers: &self.overlap_layers,
            locks: &mut self.locks,
            template: self.template.as_ref(),
            isolated_layers: &mut self.isolated_layers,
        };
        act(&mut self.session, &mut cx)
    }

    /// The pointer at a point, unsnapped.
    pub fn pointer(de: f64, dn: f64) -> Pointer {
        let world = Vec2::new(E + de, N + dn);
        Pointer::new(world, Camera.to_screen(world), false, None)
    }

    /// The pointer at a point as the desktop gives it to the tool: snapped
    /// when the running tool snaps and snapping is on, Shift as held.
    pub fn snapped(&mut self, de: f64, dn: f64) -> Pointer {
        self.spatial.sync(&self.doc);
        let raw = Vec2::new(E + de, N + dn);
        let snap = self
            .session
            .snap(&self.spatial, raw, &Camera, &self.draft, &self.tracking);
        // Tracking and the snap additions' rests follow, as on the desktop.
        self.session.follow(
            &mut self.tracking,
            &self.spatial,
            raw,
            &Camera,
            &self.draft,
            snap.as_ref(),
        );
        Pointer::new(raw, Camera.to_screen(raw), self.shift, snap)
            .tracked(self.tracking.track().map(|t| t.point))
    }

    /// The pointer rests at a point past the dwell (the traces' `rest`):
    /// what is rested on is acquired, or released (docs/adr/0085, 0163 §2).
    pub fn rest(&mut self, de: f64, dn: f64) {
        let p = self.snapped(de, dn);
        self.run(|s, cx| s.pointer_move(&p, cx));
        if let Some(n) = self.tracking.dwell() {
            let spatial = &self.spatial;
            self.tracking
                .dwell_due(n, |id, at| spatial.extensions_at(id, at));
        }
    }

    /// A click: the snap is taken again where the button goes down and up.
    pub fn click(&mut self, de: f64, dn: f64) {
        let p = self.snapped(de, dn);
        self.run(|s, cx| s.pointer_down(&p, cx));
        let p = self.snapped(de, dn);
        self.run(|s, cx| s.pointer_up(&p, cx));
    }

    pub fn move_to(&mut self, de: f64, dn: f64) {
        let p = self.snapped(de, dn);
        self.run(|s, cx| s.pointer_move(&p, cx));
    }

    /// A drag with the left button, from one point to another.
    pub fn drag(&mut self, from: [f64; 2], to: [f64; 2]) {
        let p = self.snapped(from[0], from[1]);
        self.run(|s, cx| s.pointer_down(&p, cx));
        let middle = [(from[0] + to[0]) / 2.0, (from[1] + to[1]) / 2.0];
        for [de, dn] in [middle, to] {
            let p = self.snapped(de, dn);
            self.run(|s, cx| s.pointer_move(&p, cx));
        }
        let p = self.snapped(to[0], to[1]);
        self.run(|s, cx| s.pointer_up(&p, cx));
    }

    pub fn type_text(&mut self, text: &str) -> bool {
        self.run(|s, cx| s.input(text, cx))
    }

    pub fn confirm(&mut self) {
        self.run(|s, cx| s.confirm(cx));
    }

    pub fn undo_step(&mut self) -> bool {
        self.run(|s, cx| s.undo_step(cx))
    }

    pub fn points(&self) -> usize {
        self.session.point_count()
    }

    pub fn options(&self) -> Vec<&'static str> {
        self.session.prompt().keys()
    }

    pub fn last_level(&self) -> Option<Level> {
        self.log.last().map(|l| l.level)
    }

    pub fn last_text(&self) -> Option<&str> {
        self.log.last().map(|l| l.text.as_str())
    }

    /// The messages said since `before` (a log length), with their levels.
    pub fn said(&self, before: usize) -> Vec<(Level, &str)> {
        self.log[before..]
            .iter()
            .map(|l| (l.level, l.text.as_str()))
            .collect()
    }

    /// The newest object (the highest slot).
    pub fn newest(&self) -> &Entity {
        self.doc
            .entities()
            .max_by_key(|e| e.base().id)
            .expect("an object")
    }

    /// The selected objects' slots.
    pub fn selected(&self) -> Vec<u32> {
        self.selection.ids().iter().map(|s| s.0).collect()
    }
}

/// A point as east and north differences from (E, N).
pub fn rel(p: kentos_contracts::Vec2) -> [f64; 2] {
    [p.x - E, p.y - N]
}

/// Objects added to a test drawing (docs/adr/0140): the fields every object has, on `layer`.
pub fn base(layer: &str) -> kentos_contracts::EntityBase {
    kentos_contracts::EntityBase {
        id: 0,
        layer_id: layer.to_owned(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
    }
}

fn wire(p: [f64; 2]) -> kentos_contracts::Vec2 {
    kentos_contracts::Vec2 {
        x: E + p[0],
        y: N + p[1],
    }
}

impl Bench {
    /// A line from `a` to `b` (east and north differences from (E, N)) on `layer`.
    pub fn add_line(&mut self, layer: &str, a: [f64; 2], b: [f64; 2]) -> kentos_domain::Slot {
        let line = Entity::Line(kentos_contracts::LineEntity {
            base: base(layer),
            a: wire(a),
            b: wire(b),
            za: None,
            zb: None,
        });
        self.doc.add(line).expect("a slot")
    }

    /// An open polyline (`closed` false) or a closed area through `pts` on `layer`.
    pub fn add_path(&mut self, layer: &str, pts: &[[f64; 2]], closed: bool) -> kentos_domain::Slot {
        let path = kentos_contracts::PathEntity {
            base: base(layer),
            pts: pts.iter().map(|p| wire(*p)).collect(),
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        };
        let entity = if closed {
            Entity::Polygon(path)
        } else {
            Entity::Polyline(path)
        };
        self.doc.add(entity).expect("a slot")
    }

    /// An open polyline through `pts` with a bulge on each segment (docs/adr/0147: Hızlı ölçü's arcs).
    pub fn add_bulged(
        &mut self,
        layer: &str,
        pts: &[[f64; 2]],
        bulges: &[f64],
    ) -> kentos_domain::Slot {
        let path = kentos_contracts::PathEntity {
            base: base(layer),
            pts: pts.iter().map(|p| wire(*p)).collect(),
            bulges: Some(bulges.to_vec()),
            holes: None,
            zs: None,
            parts: None,
        };
        self.doc.add(Entity::Polyline(path)).expect("a slot")
    }

    /// A point with an elevation at `at` (east and north differences from (E, N)) on `layer`.
    pub fn add_point_z(&mut self, layer: &str, at: [f64; 2], z: f64) -> kentos_domain::Slot {
        let point = Entity::Point(kentos_contracts::PointEntity {
            base: base(layer),
            p: wire(at),
            z: Some(z),
            parts: None,
        });
        self.doc.add(point).expect("a slot")
    }

    /// A circle about `c` (east and north differences from (E, N)) on `layer`.
    pub fn add_circle(&mut self, layer: &str, c: [f64; 2], r: f64) -> kentos_domain::Slot {
        let circle = Entity::Circle(kentos_contracts::CircleEntity {
            base: base(layer),
            c: wire(c),
            r,
        });
        self.doc.add(circle).expect("a slot")
    }

    /// An arc about `c` from `a0` counter-clockwise to `a1` (radians) on `layer`.
    pub fn add_arc(
        &mut self,
        layer: &str,
        c: [f64; 2],
        r: f64,
        a0: f64,
        a1: f64,
    ) -> kentos_domain::Slot {
        let arc = Entity::Arc(kentos_contracts::ArcEntity {
            base: base(layer),
            c: wire(c),
            r,
            a0,
            a1,
        });
        self.doc.add(arc).expect("a slot")
    }

    /// The vertices of a path or an area, east and north differences from (E, N).
    pub fn path_pts(&self, slot: kentos_domain::Slot) -> Vec<[f64; 2]> {
        let Some(Entity::Polyline(p) | Entity::Polygon(p)) = self.doc.get(slot) else {
            panic!("a path at {slot:?}: {:?}", self.doc.get(slot));
        };
        p.pts.iter().map(|q| rel(*q)).collect()
    }
}
