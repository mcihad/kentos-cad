//! The closed-area tool through the session, over the native document: the
//! behaviour of the web's `PathTool` (closed) that the interaction traces and
//! ADR 0018 fix, one rule per test. The traces themselves are played by the
//! desktop (apps/desktop/src/traces.rs).

use kentos_contracts::{DocumentSnapshotV1, Entity};
use kentos_domain::Document;
use kentos_interaction::{Context, Draft, Level, Line, Pointer, Session, Vec2, View};

const EMPTY: &str = include_str!("../../../../fixtures/interaction/v1/empty.kcad");
const E: f64 = 487000.0;
const N: f64 = 4420000.0;

/// The traces' view: 0.125 m per pixel around (E, N), an 800 × 600 area.
struct Camera;

impl View for Camera {
    fn to_screen(&self, p: Vec2) -> [f64; 2] {
        [(p.x - E) * 8.0 + 400.0, 300.0 - (p.y - N) * 8.0]
    }

    fn world_length(&self, px: f64) -> f64 {
        px / 8.0
    }
}

struct Bench {
    doc: Document,
    session: Session,
    log: Vec<Line>,
    draft: Draft,
}

impl Bench {
    fn new() -> Self {
        let snapshot = DocumentSnapshotV1::from_json(EMPTY).expect("the traces' drawing reads");
        let mut session = Session::new();
        assert!(session.start("polygon"));
        Self {
            doc: Document::from_snapshot(snapshot).expect("opens"),
            session,
            log: Vec::new(),
            draft: Draft::default(),
        }
    }

    fn run<T>(&mut self, act: impl FnOnce(&mut Session, &mut Context<'_>) -> T) -> T {
        let mut cx = Context {
            doc: &mut self.doc,
            view: &Camera,
            draft: self.draft,
            log: &mut self.log,
        };
        act(&mut self.session, &mut cx)
    }

    fn pointer(de: f64, dn: f64) -> Pointer {
        let world = Vec2::new(E + de, N + dn);
        Pointer {
            world,
            screen: Camera.to_screen(world),
            shift: false,
        }
    }

    fn click(&mut self, de: f64, dn: f64) {
        let p = Self::pointer(de, dn);
        self.run(|s, cx| s.pointer_down(&p, cx));
    }

    fn move_to(&mut self, de: f64, dn: f64) {
        let p = Self::pointer(de, dn);
        self.run(|s, cx| s.pointer_move(&p, cx));
    }

    fn type_text(&mut self, text: &str) -> bool {
        self.run(|s, cx| s.input(text, cx))
    }

    fn confirm(&mut self) {
        self.run(|s, cx| s.confirm(cx));
    }

    fn undo_step(&mut self) -> bool {
        self.run(|s, cx| s.undo_step(cx))
    }

    fn points(&self) -> usize {
        self.session.point_count()
    }

    fn options(&self) -> Vec<&'static str> {
        self.session.prompt().keys()
    }

    fn last_level(&self) -> Option<Level> {
        self.log.last().map(|l| l.level)
    }

    /// The newest object's corners relative to (E, N), and its bulges.
    fn newest(&self) -> (Vec<[f64; 2]>, Option<Vec<f64>>) {
        let newest = self
            .doc
            .entities()
            .max_by_key(|e| e.base().id)
            .expect("an object");
        let Entity::Polygon(path) = newest else {
            panic!("a polygon, not {newest:?}");
        };
        let pts = path.pts.iter().map(|p| [p.x - E, p.y - N]).collect();
        (pts, path.bulges.clone())
    }
}

#[test]
fn prompts_are_the_web_s_text_and_options() {
    let mut b = Bench::new();
    assert_eq!(
        b.session.prompt().text(),
        "Kapalı alan: ilk noktayı belirtin"
    );
    assert!(b.options().is_empty());
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kapalı alan: sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / Geri (G)]"
    );
    b.click(10.0, 0.0);
    b.click(10.0, 10.0);
    assert_eq!(b.options(), ["Y", "U", "G", "Enter"]);
    assert!(b.type_text("y"));
    assert_eq!(
        b.session.prompt().text(),
        "Kapalı alan: yayın bitiş noktasını belirtin [Düz (D) / Açı (A) / Merkez (M) / Yarıçap (R) / İkinci nokta (İ) / Doğrultu (T) / Geri (G) / Bitir (Enter)]"
    );
    assert!(b.type_text("A"));
    assert_eq!(
        b.session.prompt().text(),
        "Kapalı alan: yayın iç açısını derece olarak yazın (artı saat yönünün tersine)"
    );
    assert!(b.type_text("i"), "i is İ, the second point");
    assert_eq!(
        b.session.prompt().step,
        "yayın üzerinden geçeceği bir nokta belirtin"
    );
    assert!(b.type_text("D"));
    assert!(b.type_text("U"));
    assert_eq!(
        b.session.prompt().text(),
        "Kapalı alan: son doğrultuda devam edilecek uzunluğu yazın"
    );
    assert!(!b.type_text("K"), "no such option");
}

#[test]
fn clicks_then_confirm_write_one_polygon_in_one_undo_step() {
    let mut b = Bench::new();
    for (de, dn) in [(0.0, 0.0), (12.0, 0.0), (12.0, 8.0)] {
        b.click(de, dn);
    }
    assert_eq!(b.points(), 3);
    assert_eq!(b.doc.len(), 0, "the preview does not touch the drawing");
    b.confirm();
    assert_eq!(b.doc.len(), 1);
    assert_eq!(
        b.newest(),
        (vec![[0.0, 0.0], [12.0, 0.0], [12.0, 8.0]], None)
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(
        b.log.last().map(|l| l.text.as_str()),
        Some("Kapalı alan eklendi: 48.00 m²")
    );
    assert_eq!(
        (b.session.tool_id(), b.points()),
        ("polygon", 0),
        "ready for the next area"
    );
    assert!(b.doc.can_undo() && b.doc.is_dirty());
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert_eq!(b.doc.len(), 0);
    assert!(!b.doc.can_undo());
}

#[test]
fn typed_values_are_exact() {
    let mut b = Bench::new();
    b.click(0.0, 0.0);
    b.move_to(20.0, 0.0);
    assert!(b.type_text("12"), "a distance towards the cursor");
    assert!(b.type_text("@0,8"));
    b.confirm();
    let (pts, _) = b.newest();
    assert_eq!(pts, [[0.0, 0.0], [12.0, 0.0], [12.0, 8.0]]);
    // Minus goes the other way (UX-04), plus is plain.
    b.click(0.0, -15.0);
    b.move_to(20.0, -15.0);
    assert!(b.type_text("-4"));
    assert!(b.type_text("+7"));
    assert!(b.type_text("487010,4419990"));
    b.confirm();
    let (pts, _) = b.newest();
    assert_eq!(
        pts,
        [[0.0, -15.0], [-4.0, -15.0], [3.0, -15.0], [10.0, -10.0]]
    );
    assert!(!b.type_text("abc"));
}

#[test]
fn fewer_than_three_corners_warn_and_write_nothing() {
    let mut b = Bench::new();
    b.click(-25.0, -10.0);
    b.click(-25.0, -10.0);
    assert_eq!(b.points(), 1, "a double click adds no second point");
    b.click(-15.0, -10.0);
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.log.last().map(|l| l.text.as_str()),
        Some("Kapalı alan için en az 3 nokta gerekir.")
    );
    assert_eq!(
        (b.doc.len(), b.points(), b.session.tool_id()),
        (0, 0, "polygon")
    );
    // With nothing drawn, a confirm leaves the tool.
    b.confirm();
    assert!(!b.session.is_running());
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.session.last(), Some("polygon"), "Enter repeats it");
}

#[test]
fn undo_takes_back_the_draft_first_then_hands_over() {
    let mut b = Bench::new();
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(b.undo_step());
    assert_eq!(b.points(), 1);
    assert!(b.type_text("g"), "G is Geri");
    assert_eq!(b.points(), 0);
    assert!(
        !b.undo_step(),
        "nothing pending: the drawing is undone instead"
    );
    assert!(!b.doc.can_undo());
}

#[test]
fn giving_the_first_corner_again_closes_the_area() {
    let mut b = Bench::new();
    b.click(-30.0, -15.0);
    b.click(-20.0, -15.0);
    b.click(-20.0, -5.0);
    b.click(-30.0, -15.0);
    assert_eq!((b.doc.len(), b.points()), (1, 0));
    assert_eq!(
        b.newest().0,
        [[-30.0, -15.0], [-20.0, -15.0], [-20.0, -5.0]]
    );
    // Within the snap aperture: 5 px off the first corner.
    b.click(-10.0, -15.0);
    b.click(0.0, -15.0);
    b.click(0.0, -5.0);
    b.click(-9.5, -14.625);
    assert_eq!(b.doc.len(), 2);
    assert_eq!(
        b.newest().0.len(),
        3,
        "the first corner is not written twice"
    );
    // Typed exactly.
    for text in ["487010,4419985", "@10,0", "@0,10", "487010,4419985"] {
        assert!(b.type_text(text));
    }
    assert_eq!(b.doc.len(), 3);
    // With fewer than three corners the first one is not added again.
    assert!(b.type_text("486975,4420005"));
    assert!(b.type_text("@5,0"));
    assert!(b.type_text("486975,4420005"));
    assert_eq!((b.doc.len(), b.points()), (3, 2));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.log.last().map(|l| l.text.as_str()),
        Some("Kapalı alan için en az 3 köşe gerekir; ilk köşe ikinci kez eklenmedi.")
    );
    // Beyond the aperture a click is a new corner.
    let mut far = Bench::new();
    far.click(0.0, 0.0);
    far.click(10.0, 0.0);
    far.click(10.0, 10.0);
    far.click(2.0, 0.0);
    assert_eq!((far.doc.len(), far.points()), (0, 4));
}

#[test]
fn in_arc_mode_the_closing_edge_is_the_arc() {
    let mut b = Bench::new();
    b.click(0.0, 5.0);
    b.click(10.0, 5.0);
    b.click(10.0, 15.0);
    assert!(b.type_text("Y"));
    b.click(0.0, 5.0);
    assert_eq!(b.doc.len(), 1);
    let (pts, bulges) = b.newest();
    assert_eq!(pts.len(), 3);
    let bulges = bulges.expect("an arc edge");
    assert_eq!(bulges.len(), 3);
    assert_eq!(bulges.iter().filter(|b| **b != 0.0).count(), 1);
    assert_ne!(bulges[2], 0.0, "the closing edge");
}

#[test]
fn arc_options_shape_one_segment() {
    // A typed included angle, then the end: a quarter circle bulges tan(90°/4).
    let mut b = Bench::new();
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(b.type_text("Y"));
    assert!(b.type_text("A"));
    assert!(b.type_text("400"), "out of range warns and keeps waiting");
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.type_text("90"));
    b.click(10.0, 10.0);
    assert!(b.type_text("D"));
    b.click(0.0, 10.0);
    b.confirm();
    let (_, bulges) = b.newest();
    let bulges = bulges.expect("one arc");
    assert!((bulges[1] - (std::f64::consts::PI / 8.0).tan()).abs() < 1e-12);
    assert_eq!([bulges[0], bulges[2], bulges[3]], [0.0, 0.0, 0.0]);

    // A radius too short for the chord warns with the chord limit.
    let mut r = Bench::new();
    r.click(0.0, 0.0);
    assert!(r.type_text("Y"));
    assert!(r.type_text("R"));
    assert!(r.type_text("-1"));
    assert_eq!(r.last_level(), Some(Level::Warn));
    assert!(r.type_text("2"));
    r.click(10.0, 0.0);
    assert_eq!(
        r.log.last().map(|l| l.text.as_str()),
        Some("Kiriş yarıçapın iki katından (4.000 m) uzun; daha yakın bir nokta seçin.")
    );
    assert_eq!(r.points(), 1);

    // Centre, second point and direction take their point first.
    let mut c = Bench::new();
    c.click(10.0, 0.0);
    assert!(c.type_text("Y"));
    assert!(c.type_text("M"));
    c.click(0.0, 0.0);
    assert_eq!(c.session.prompt().step, "yayın bitiş doğrultusunu gösterin");
    c.click(0.0, 20.0);
    assert_eq!(c.points(), 2, "the end goes on the circle");
    assert!(c.type_text("İ"));
    c.click(-8.0, 6.0);
    assert_eq!(c.points(), 2);
    c.click(-10.0, 0.0);
    assert!(c.type_text("T"));
    c.click(-10.0, -5.0);
    c.click(0.0, -10.0);
    assert_eq!(c.points(), 4);
    c.confirm();
    let (pts, bulges) = c.newest();
    assert!((pts[1][0] - 0.0).abs() < 1e-9 && (pts[1][1] - 10.0).abs() < 1e-9);
    let bulges = bulges.expect("arcs");
    assert!(bulges[..3].iter().all(|b| *b != 0.0));
}

#[test]
fn length_continues_the_last_direction() {
    let mut b = Bench::new();
    b.click(0.0, 0.0);
    assert!(b.type_text("U"));
    assert_eq!(b.last_level(), Some(Level::Warn), "no direction yet");
    b.click(3.0, 4.0);
    assert!(b.type_text("U"));
    assert!(b.type_text("0"));
    assert_eq!(
        b.log.last().map(|l| l.text.as_str()),
        Some("Uzunluk sıfırdan büyük olmalı.")
    );
    assert!(b.type_text("5"));
    assert_eq!(b.points(), 3);
    b.move_to(0.0, 20.0);
    b.confirm();
    let (pts, _) = b.newest();
    assert!((pts[2][0] - 6.0).abs() < 1e-9 && (pts[2][1] - 8.0).abs() < 1e-9);
}

#[test]
fn a_locked_layer_takes_nothing_and_says_how_to_fix_it() {
    let mut b = Bench::new();
    b.doc.toggle_layer_locked("cizim");
    for (de, dn) in [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)] {
        b.click(de, dn);
    }
    b.confirm();
    assert_eq!(b.doc.len(), 0);
    assert_eq!(
        b.log.last().map(|l| l.text.as_str()),
        Some(
            "“Çizim” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
        )
    );
}

#[test]
fn shift_turns_ortho_on_for_a_point() {
    let mut b = Bench::new();
    b.click(0.0, 0.0);
    let mut p = Bench::pointer(10.0, 3.0);
    p.shift = true;
    b.run(|s, cx| s.pointer_down(&p, cx));
    b.click(0.0, 10.0);
    b.confirm();
    assert_eq!(b.newest().0[1], [10.0, 0.0]);
}

#[test]
fn the_preview_follows_the_cursor_and_measures() {
    let mut b = Bench::new();
    b.click(0.0, 0.0);
    b.move_to(12.0, 0.0);
    let format = kentos_interaction::Format::default();
    let preview = b.session.preview(&format).expect("a tool runs");
    assert_eq!(preview.path.len(), 2);
    assert!(preview.ring.is_none());
    let tag = preview.tag.expect("a measurement beside the cursor");
    assert_eq!(tag.lines, ["12.000 m", "Semt 100.0000 g"]);
    b.click(12.0, 0.0);
    b.move_to(12.0, 8.0);
    let preview = b.session.preview(&format).expect("a tool runs");
    assert_eq!(preview.ring.as_ref().map(Vec::len), Some(3));
    assert_eq!(
        preview.tag.map(|t| t.lines.last().cloned()),
        Some(Some("Alan 48.00 m²".to_owned()))
    );
    assert_eq!(b.doc.len(), 0);
}
