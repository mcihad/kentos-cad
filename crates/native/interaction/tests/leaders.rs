//! Leaders through the session (docs/adr/0146 §4): Patlat takes a leader
//! apart into its line on to its landing's end, its arrowhead and its note,
//! one undo step, the pieces on its layer with its colour. The drawing is the
//! leaders' picture scene (fixtures/interaction/README.md); expected values
//! are worked out by hand from the layout (§2).

mod common;

use common::{Bench, E, N, base};
use kentos_contracts::{Entity, HatchPatternType, TextAlign};
use kentos_domain::Slot;

const LEADERS: &str = include_str!("../../../../fixtures/interaction/v1/leaders.kcad");

/// A point from the scene's origin, (500000, 4400000).
fn rel(p: kentos_contracts::Vec2) -> [f64; 2] {
    [p.x - 500_000.0, p.y - 4_400_000.0]
}

fn bench(selected: &[u32]) -> Bench {
    let mut b = Bench::on(LEADERS);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

#[test]
fn explode_takes_a_leader_apart_into_its_line_arrowhead_and_note() {
    // “Mevcut bina”, 2.5 m high, from (0, 0) to (6, 5) with a filled arrow: the landing 5 m east, the note 1.25 m on.
    let mut b = bench(&[1]);
    b.start("explode");
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 3 parça."));
    assert!(b.doc.get(Slot(1)).is_none());
    assert_eq!(b.selected(), [6, 7, 8]);
    let Some(Entity::Polyline(line)) = b.doc.get(Slot(6)) else {
        panic!("its line: {:?}", b.doc.get(Slot(6)));
    };
    let pts: Vec<[f64; 2]> = line.pts.iter().map(|q| rel(*q)).collect();
    assert_eq!(pts, [[0.0, 0.0], [6.0, 5.0], [11.0, 5.0]]);
    assert_eq!(line.base.layer_id, "kilavuz", "the pieces keep the layer");
    let Some(Entity::Hatch(head)) = b.doc.get(Slot(7)) else {
        panic!("its arrowhead: {:?}", b.doc.get(Slot(7)));
    };
    assert_eq!(head.pattern.kind, HatchPatternType::Solid);
    assert_eq!(head.ring.len(), 3);
    assert_eq!(rel(head.ring[0]), [0.0, 0.0], "the tip first");
    let Some(Entity::Text(note)) = b.doc.get(Slot(8)) else {
        panic!("its note: {:?}", b.doc.get(Slot(8)));
    };
    assert_eq!(note.text, "Mevcut bina");
    assert_eq!(rel(note.p), [12.25, 5.0]);
    assert_eq!((note.height, note.rotation), (2.5, 0.0));
    assert_eq!(note.align, Some(TextAlign::MiddleLeft));
    assert_eq!(b.doc.undo().as_deref(), Some("Patlat"));
    assert!(matches!(b.doc.get(Slot(1)), Some(Entity::Leader(_))));
}

#[test]
fn an_open_arrow_comes_apart_as_a_path_and_a_masked_note_keeps_its_mask() {
    // “Ø150 PVC”, 2 m high, its landing to the left from (36, −7): the note's middle right at (31, −7).
    let mut b = bench(&[2]);
    b.start("explode");
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 3 parça."));
    let Some(Entity::Polyline(sides)) = b.doc.get(Slot(7)) else {
        panic!("its arrowhead's sides: {:?}", b.doc.get(Slot(7)));
    };
    assert_eq!(sides.pts.len(), 3);
    assert_eq!(rel(sides.pts[1]), [44.0, -12.0], "through the tip");
    let Some(Entity::Text(note)) = b.doc.get(Slot(8)) else {
        panic!("its note");
    };
    assert_eq!(rel(note.p), [31.0, -7.0]);
    assert_eq!(note.align, Some(TextAlign::MiddleRight));
    assert!(note.mask, "the mask goes with the note");
}

/// A leader added to the grips' drawing (fixtures/interaction/v1/objects.kcad), near (E, N):
/// from (0, 20) up to (6, 25), its note 2 m high.
fn with_leader() -> (Bench, Slot) {
    let mut b = Bench::on(include_str!("../../../../fixtures/interaction/v1/objects.kcad"));
    b.draft.snap = false;
    let at = |x: f64, y: f64| kentos_contracts::Vec2 { x: E + x, y: N + y };
    let leader = Entity::Leader(kentos_contracts::LeaderEntity {
        base: base("cizim"),
        pts: vec![at(0.0, 20.0), at(6.0, 25.0)],
        text: Some("Mevcut bina".into()),
        height: 2.0,
        rotation: 0.0,
        arrow: None,
        mask: false,
    });
    let slot = b.doc.add(leader).expect("a slot");
    b.selection.set([slot]);
    (b, slot)
}

fn leader_pts(b: &Bench, slot: Slot) -> Vec<[f64; 2]> {
    let Some(Entity::Leader(l)) = b.doc.get(slot) else {
        panic!("a leader: {:?}", b.doc.get(slot));
    };
    l.pts.iter().map(|q| [q.x - E, q.y - N]).collect()
}

#[test]
fn a_dragged_grip_moves_a_leader_s_vertex_and_a_mid_grip_adds_one() {
    let (mut b, slot) = with_leader();
    // Its last vertex, where the landing starts: the note goes with it.
    b.drag([6.0, 25.0], [8.0, 27.0]);
    assert_eq!(leader_pts(&b, slot), [[0.0, 20.0], [8.0, 27.0]]);
    assert_eq!(b.doc.undo().as_deref(), Some("Tutamaçla düzenle"));
    // The segment's middle grip adds a vertex there.
    b.drag([3.0, 22.5], [2.0, 24.0]);
    assert_eq!(leader_pts(&b, slot), [[0.0, 20.0], [2.0, 24.0], [6.0, 25.0]]);
    let Some(Entity::Leader(l)) = b.doc.get(slot) else {
        panic!("a leader");
    };
    assert_eq!(l.text.as_deref(), Some("Mevcut bina"), "the note stays");
}

// ── The tool (docs/adr/0146 §7): the web's tests in apps/web/src/tools/leaderTool.test.ts, the same words ──

/// The tool on the empty drawing, snapping off (the traces' default).
fn tool() -> Bench {
    let mut b = Bench::new("leader");
    b.draft.snap = false;
    b
}

/// The text field the tool asked for last.
fn field(b: &Bench) -> kentos_interaction::TextField {
    let Some(kentos_interaction::ViewChange::Text(field)) = b.views.last().cloned() else {
        panic!("a text field asked for: {:?}", b.views);
    };
    field
}

fn leaders_drawn(b: &Bench) -> Vec<kentos_contracts::LeaderEntity> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Leader(l) => Some(l.clone()),
            _ => None,
        })
        .collect()
}

fn at(p: kentos_contracts::Vec2) -> [f64; 2] {
    [p.x - E, p.y - N]
}

#[test]
fn the_tool_opens_the_note_past_the_landing_and_writes_the_leader_in_one_step() {
    let mut b = tool();
    assert_eq!(
        b.session.prompt().text(),
        "Kılavuz: okun ucuna tıklayın [Ok (O): dolu / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı]"
    );
    b.click(0.0, 0.0);
    b.click(6.0, 5.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kılavuz: sonraki köşeye tıklayın; Enter ya da sağ tık notu yazdırır [Ok (O): dolu / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı / Geri (G)]"
    );
    b.confirm();
    // The landing 5 m east of (6, 5), the note 1.25 m on: its middle left at (12.25, 5).
    let f = field(&b);
    assert_eq!(at(kentos_contracts::Vec2 { x: f.at.x, y: f.at.y }), [12.25, 5.0]);
    assert_eq!((f.height, f.rotation), (2.5, 0.0));
    assert_eq!(f.align, Some(kentos_contracts::TextAlign::MiddleLeft));
    assert!(f.empty, "an empty field's Enter answers");
    assert_eq!(
        b.session.prompt().text(),
        "Kılavuz: notu kolun ucuna yazın; Enter ekler, boş Enter notsuz ekler, Esc köşelere döner"
    );
    b.run(|s, cx| s.text_typed(Some("Mevcut bina"), cx));
    let [l] = &leaders_drawn(&b)[..] else {
        panic!("one leader");
    };
    assert_eq!(
        l.pts.iter().map(|p| at(*p)).collect::<Vec<_>>(),
        [[0.0, 0.0], [6.0, 5.0]]
    );
    assert_eq!(
        (l.text.as_deref(), l.height, l.rotation, l.arrow, l.mask),
        (Some("Mevcut bina"), 2.5, 0.0, None, false)
    );
    assert_eq!(b.last_text(), Some("Kılavuz eklendi: “Mevcut bina”"));
    assert_eq!(
        b.session.prompt().text(),
        "Kılavuz: okun ucuna tıklayın [Ok (O): dolu / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı]"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Kılavuz"));
}

#[test]
fn a_note_to_the_left_opens_on_its_middle_right_and_the_empty_field_writes_the_arrow_alone() {
    let mut b = tool();
    assert!(b.type_text("Z"));
    b.click(30.0, 0.0);
    b.click(26.0, 4.0);
    b.click(20.0, 6.0);
    b.confirm();
    // The landing 5 m west of (20, 6), the note 1.25 m on: its middle right at (13.75, 6).
    let f = field(&b);
    assert_eq!(at(kentos_contracts::Vec2 { x: f.at.x, y: f.at.y }), [13.75, 6.0]);
    assert_eq!(f.align, Some(kentos_contracts::TextAlign::MiddleRight));
    b.run(|s, cx| s.text_typed(Some(""), cx));
    let [l] = &leaders_drawn(&b)[..] else {
        panic!("one leader");
    };
    assert_eq!((l.pts.len(), l.text.as_deref(), l.mask), (3, None, false));
    assert_eq!(b.last_text(), Some("Kılavuz eklendi (notsuz)."));
}

#[test]
fn ok_chooses_the_arrowhead_zemin_masks_the_note_geri_takes_the_last_vertex_back() {
    let mut b = tool();
    let menu = b.session.option_choices("O");
    assert_eq!(
        menu.iter()
            .map(|c| (c.label, c.icon, c.checked))
            .collect::<Vec<_>>(),
        [
            ("Dolu", "leaderArrowFilled", true),
            ("Açık", "leaderArrowOpen", false),
            ("Nokta", "leaderArrowDot", false),
            ("Yok", "leaderArrowNone", false),
        ]
    );
    assert!(b.run(|s, cx| s.choose_option("O", "nokta", cx)));
    assert!(b.type_text("O"));
    assert_eq!(
        b.session.prompt().text(),
        "Kılavuz: ok başını seçin ya da adını yazın: dolu, açık, nokta, yok [Ok (O): nokta]"
    );
    assert!(b.type_text("Acik"));
    assert!(b.type_text("Z"));
    b.click(0.0, 0.0);
    b.click(3.0, 3.0);
    b.click(9.0, 3.0);
    assert!(b.type_text("G"));
    b.click(8.0, 6.0);
    b.confirm();
    b.run(|s, cx| s.text_typed(Some("Ø150 PVC"), cx));
    let [l] = &leaders_drawn(&b)[..] else {
        panic!("one leader");
    };
    assert_eq!(
        l.pts.iter().map(|p| at(*p)).collect::<Vec<_>>(),
        [[0.0, 0.0], [3.0, 3.0], [8.0, 6.0]]
    );
    assert_eq!(
        (l.arrow, l.mask),
        (Some(kentos_contracts::LeaderArrow::Open), true)
    );
    // A name that names none is said, and the tool waits on.
    assert!(b.type_text("O"));
    assert!(b.type_text("üçgen"));
    assert_eq!(
        b.last_text(),
        Some("“üçgen” bir ok başı adı değil. Ok başını menüden seçin ya da adını yazın: dolu, açık, nokta, yok.")
    );
}

#[test]
fn yukseklik_is_yazis_and_esc_steps_back() {
    let mut b = tool();
    assert!(b.type_text("Y"));
    assert_eq!(
        b.session.prompt().text(),
        "Kılavuz: kâğıt üzerindeki not yüksekliğini mm olarak yazın (Yazı ile ortak)"
    );
    assert!(b.type_text("4"));
    assert_eq!(b.memory.text_height_mm, 4.0, "Yazı's own");
    b.click(0.0, 0.0);
    b.click(4.0, 3.0);
    b.confirm();
    assert_eq!(field(&b).height, 4.0);
    // Esc in the field: back to the vertices; then the leader drawn; then out of the tool.
    b.run(|s, cx| s.text_typed(None, cx));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Kılavuz: sonraki köşeye tıklayın; Enter ya da sağ tık notu yazdırır")
    );
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.points(), 0);
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert!(leaders_drawn(&b).is_empty());
}

#[test]
fn with_the_tip_alone_enter_says_what_is_missing_and_a_locked_layer_is_said_at_the_tip() {
    let mut b = tool();
    b.click(0.0, 0.0);
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Kılavuzun en az 2 köşesi olur: okun ucundan sonra bir köşeye daha tıklayın.")
    );
    assert!(b.views.is_empty());
    // The tools' drawing has a locked layer.
    let mut b = Bench::on(include_str!("../../../../fixtures/interaction/v1/tools.kcad"));
    b.draft.snap = false;
    assert!(b.doc.set_active_layer("kilitli"), "the locked layer");
    b.start("leader");
    b.click(0.0, 0.0);
    assert_eq!(b.points(), 0);
    assert_eq!(
        b.last_text(),
        Some("“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.")
    );
}
