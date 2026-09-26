//! The tools' test bench: the session over a drawing (the traces' empty one
//! unless a test gives another), seen through the traces' view (0.125 m per
//! pixel around (E, N) on an 800 × 600 area), with the geometry store and
//! the selection the desktop keeps beside it. Points are given as east and
//! north differences from (E, N), as in the traces; clicks and moves snap as
//! the desktop snaps them (docs/adr/0029).
#![allow(dead_code)]

use kentos_contracts::{DocumentSnapshotV1, Entity};
use kentos_domain::Document;
use kentos_interaction::{
    Context, Draft, Level, Line, Memory, Pointer, Selection, Session, Spatial, Vec2, View,
    ViewChange,
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
            views: Vec::new(),
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
        let snap = self.session.snap(&self.spatial, raw, &Camera, &self.draft);
        Pointer::new(raw, Camera.to_screen(raw), self.shift, snap)
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
