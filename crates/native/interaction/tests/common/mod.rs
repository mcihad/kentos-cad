//! The tools' test bench: a tool running in the session over the traces'
//! empty drawing, seen through the traces' view (0.125 m per pixel around
//! (E, N) on an 800 × 600 area). Points are given as east and north
//! differences from (E, N), as in the traces.
#![allow(dead_code)]

use kentos_contracts::{DocumentSnapshotV1, Entity};
use kentos_domain::Document;
use kentos_interaction::{Context, Draft, Level, Line, Pointer, Session, Vec2, View};

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
}

pub struct Bench {
    pub doc: Document,
    pub session: Session,
    pub log: Vec<Line>,
    pub draft: Draft,
}

impl Bench {
    /// `tool` running on the empty drawing.
    pub fn new(tool: &str) -> Self {
        let snapshot = DocumentSnapshotV1::from_json(EMPTY).expect("the traces' drawing reads");
        let mut session = Session::new();
        assert!(session.start(tool), "{tool} is a session tool");
        Self {
            doc: Document::from_snapshot(snapshot).expect("opens"),
            session,
            log: Vec::new(),
            draft: Draft::default(),
        }
    }

    pub fn run<T>(&mut self, act: impl FnOnce(&mut Session, &mut Context<'_>) -> T) -> T {
        let mut cx = Context {
            doc: &mut self.doc,
            view: &Camera,
            draft: self.draft,
            log: &mut self.log,
        };
        act(&mut self.session, &mut cx)
    }

    pub fn pointer(de: f64, dn: f64) -> Pointer {
        let world = Vec2::new(E + de, N + dn);
        Pointer {
            world,
            screen: Camera.to_screen(world),
            shift: false,
        }
    }

    pub fn click(&mut self, de: f64, dn: f64) {
        let p = Self::pointer(de, dn);
        self.run(|s, cx| s.pointer_down(&p, cx));
    }

    pub fn move_to(&mut self, de: f64, dn: f64) {
        let p = Self::pointer(de, dn);
        self.run(|s, cx| s.pointer_move(&p, cx));
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
}

/// A point as east and north differences from (E, N).
pub fn rel(p: kentos_contracts::Vec2) -> [f64; 2] {
    [p.x - E, p.y - N]
}
