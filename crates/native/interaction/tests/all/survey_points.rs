//! Nokta (docs/adr/0152 §2–§4) through the session, over the native
//! document: Ad, Kod and Kot kept for as long as the app lives, the name
//! moving on by Artır, the question where a point already is, Ctrl+Z giving
//! the name back, and `#ad` giving a named point's place to the running
//! tool; as the web's `apps/web/src/tools/surveyPointTool.test.ts` walks it,
//! the same cases moved to (E, N). Values worked out by hand from the ADR's
//! rules.

use crate::common;

use common::{Bench, E, N, rel};
use kentos_contracts::{Entity, EntityBase, PointEntity, TextEntity};
use kentos_domain::Slot;
use kentos_interaction::{Level, ViewChange};

fn bench() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    b.draft.snap = false;
    b
}

fn points(b: &Bench) -> Vec<PointEntity> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Point(p) => Some(p.clone()),
            _ => None,
        })
        .collect()
}

fn add_point(b: &mut Bench, at: [f64; 2], label: Option<&str>) -> Slot {
    let point = PointEntity {
        base: EntityBase {
            label: label.map(str::to_owned),
            ..common::base("cizim")
        },
        p: kentos_contracts::Vec2 {
            x: E + at[0],
            y: N + at[1],
        },
        z: None,
        parts: None,
    };
    b.doc.add(Entity::Point(point)).expect("a slot")
}

/// A point as the tests compare it: where (from (E, N)), its name, its `Kod`, its elevation.
fn seen(p: &PointEntity) -> ([f64; 2], Option<String>, Option<String>, Option<f64>) {
    (
        rel(p.p),
        p.base.label.clone(),
        p.base.attrs.get("Kod").cloned(),
        p.z,
    )
}

fn labels(b: &Bench) -> Vec<String> {
    points(b)
        .iter()
        .map(|p| p.base.label.clone().unwrap_or_default())
        .collect()
}

fn prompt(name: &str, code: &str, z: &str) -> String {
    format!("Nokta: nokta konumunu belirtin [Ad (A): {name} / Kod (K): {code} / Kot (Z): {z}]")
}

/// Answers the text field the key asked for: a value, an empty Enter (`Some("")`) or Esc (`None`).
fn answer(b: &mut Bench, key: &str, value: Option<&str>) {
    assert!(b.type_text(key));
    assert_eq!(b.session.prompt().text(), "Nokta: değeri yazın");
    assert!(matches!(b.views.last(), Some(ViewChange::Text(_))));
    b.run(|s, cx| s.text_typed(value, cx));
}

fn typed_at(de: f64, dn: f64) -> String {
    format!("{},{}", E + de, N + dn)
}

#[test]
fn without_a_name_a_code_or_an_elevation_writes_plain_points() {
    let mut b = bench();
    b.start("point");
    assert_eq!(b.session.prompt().text(), prompt("—", "—", "yok"));
    assert_eq!(b.options(), ["A", "K", "Z"]);
    b.click(10.0, 5.0);
    let p = &points(&b)[0];
    assert_eq!(seen(p), ([10.0, 5.0], None, None, None));
    assert!(p.base.attrs.is_empty());
    assert_eq!(p.base.layer_id, "cizim");
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    // A confirm leaves: it holds no point.
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn gives_the_points_their_name_code_and_elevation_the_name_moving_on() {
    let mut b = bench();
    b.start("point");
    b.move_to(4.0, 4.0);
    answer(&mut b, "A", Some(" 101 "));
    // The field opens by the cursor, the old value in it; an empty Enter answers.
    let Some(ViewChange::Text(field)) = b.views.first() else {
        panic!("a text field");
    };
    assert_eq!(
        (
            rel(kentos_contracts::Vec2 {
                x: field.at.x,
                y: field.at.y
            }),
            field.initial.as_deref(),
            field.placeholder,
            field.empty
        ),
        ([4.0, 4.0], Some(""), Some("Noktanın adı"), true)
    );
    assert_eq!(b.session.prompt().text(), prompt("101", "—", "yok"));
    answer(&mut b, "K", Some("SN"));
    assert!(b.type_text("z"));
    assert_eq!(
        b.session.prompt().text(),
        "Nokta: noktaların kotunu yazın, metre (boş Enter: kotsuz)"
    );
    // Clicks wait while the elevation is asked.
    b.click(1.0, 1.0);
    assert!(points(&b).is_empty());
    assert!(!b.type_text("abc"));
    assert!(b.type_text("102.35"));
    assert_eq!(b.session.prompt().text(), prompt("101", "SN", "102.350 m"));
    b.click(0.0, 0.0);
    assert!(b.type_text(&typed_at(10.0, 20.0)));
    let sn = Some("SN".to_owned());
    assert_eq!(
        points(&b).iter().map(seen).collect::<Vec<_>>(),
        [
            ([0.0, 0.0], Some("101".to_owned()), sn.clone(), Some(102.35)),
            (
                [10.0, 20.0],
                Some("102".to_owned()),
                sn.clone(),
                Some(102.35)
            ),
        ]
    );
    assert_eq!(b.session.prompt().text(), prompt("103", "SN", "102.350 m"));
    // The name the next point takes, beside the cursor.
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    assert_eq!(preview.tag.map(|t| t.lines), Some(vec!["103".to_owned()]));
    // An empty Enter clears the elevation; an empty field the code; Esc keeps the name.
    b.type_text("Z");
    b.confirm();
    answer(&mut b, "K", Some(""));
    answer(&mut b, "A", None);
    assert_eq!(b.session.prompt().text(), prompt("103", "—", "yok"));
    b.click(30.0, 0.0);
    let last = points(&b).pop().expect("a point");
    assert_eq!(
        seen(&last),
        ([30.0, 0.0], Some("103".to_owned()), None, None)
    );
    assert!(last.base.attrs.is_empty());
    // Kept for as long as the app lives.
    b.start("point");
    assert_eq!(b.session.prompt().text(), prompt("104", "—", "yok"));
}

#[test]
fn moves_a_name_on_as_artir_does_a_name_not_ending_in_a_number_stays() {
    let mut b = bench();
    b.start("point");
    for (first, x) in [
        ("101/12", 0.0),
        ("P9", 10.0),
        ("A-009", 20.0),
        ("Köşe", 30.0),
    ] {
        answer(&mut b, "A", Some(first));
        b.click(x, 0.0);
        b.click(x, 5.0);
    }
    assert_eq!(
        labels(&b),
        [
            "101/12", "101/13", "P9", "P10", "A-009", "A-010", "Köşe", "Köşe"
        ]
    );
}

#[test]
fn asks_where_a_point_already_is_duzelt_ekle_and_atla() {
    let mut b = bench();
    add_point(&mut b, [5.0, 5.0], Some("7"));
    b.start("point");
    // Without a value Düzelt has nothing to give.
    b.click(5.0, 5.0000005);
    assert_eq!(
        b.session.prompt().text(),
        "Nokta: bu yerde “7” noktası var [Düzelt (D) / Ekle (E) / Atla (Esc)]"
    );
    assert_eq!(b.options(), ["D", "E", "Esc"]);
    assert!(b.type_text("d"));
    assert_eq!(
        b.last_text(),
        Some("Nokta: düzeltilecek değer yok; Ad (A), Kod (K) ya da Kot (Z) verin.")
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.last_text(), Some("Nokta: atlandı."));
    assert_eq!(points(&b).len(), 1);
    answer(&mut b, "A", Some("201"));
    answer(&mut b, "K", Some("SN"));
    b.type_text("Z");
    b.type_text("50");
    // Düzelt: the point there takes them, in one step; the name moves on.
    b.click(5.0, 5.0);
    // Clicks and Enter wait for the answer.
    b.click(9.0, 9.0);
    b.confirm();
    assert_eq!(b.session.tool_id(), "point");
    assert!(b.type_text("D"));
    assert_eq!(
        points(&b).iter().map(seen).collect::<Vec<_>>(),
        [(
            [5.0, 5.0],
            Some("201".to_owned()),
            Some("SN".to_owned()),
            Some(50.0)
        )]
    );
    assert_eq!(b.last_text(), Some("Nokta: “201” noktası düzeltildi."));
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.session.prompt().text(), prompt("202", "SN", "50.000 m"));
    // Ekle: the new one is written too.
    b.click(5.0, 5.0);
    assert_eq!(
        b.session.prompt().text(),
        "Nokta: bu yerde “201” noktası var [Düzelt (D) / Ekle (E) / Atla (Esc)]"
    );
    b.type_text("E");
    assert_eq!(labels(&b), ["201", "202"]);
    // Atla: nothing, and the name stays.
    b.click(5.0, 5.0);
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(points(&b).len(), 2);
    assert_eq!(b.session.prompt().text(), prompt("203", "SN", "50.000 m"));
    // Undo takes Düzelt back whole.
    b.doc.undo();
    assert_eq!(b.doc.undo().as_deref(), Some("Nokta düzelt"));
    assert_eq!(
        points(&b).iter().map(seen).collect::<Vec<_>>(),
        [([5.0, 5.0], Some("7".to_owned()), None, None)]
    );
}

#[test]
fn says_adsiz_bir_nokta_for_an_unnamed_point_there() {
    let mut b = bench();
    add_point(&mut b, [0.0, 0.0], None);
    b.start("point");
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Nokta: bu yerde adsız bir nokta var [Düzelt (D) / Ekle (E) / Atla (Esc)]"
    );
}

#[test]
fn takes_the_newest_point_or_duzelt_back_with_ctrl_z_its_name_given_back() {
    let mut b = bench();
    b.start("point");
    answer(&mut b, "A", Some("105"));
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert_eq!(b.session.prompt().text(), prompt("107", "—", "yok"));
    assert!(b.undo_step());
    assert_eq!(labels(&b), ["105"]);
    assert_eq!(b.session.prompt().text(), prompt("106", "—", "yok"));
    // Once: the older point is the drawing's to undo.
    assert!(!b.undo_step());
    b.click(0.0, 0.0);
    b.type_text("D");
    assert_eq!(labels(&b), ["106"]);
    assert!(b.undo_step());
    assert_eq!(labels(&b), ["105"]);
    assert_eq!(b.session.prompt().text(), prompt("106", "—", "yok"));
    // Out of the question first, as Esc.
    b.click(0.0, 0.0);
    assert!(b.undo_step());
    assert_eq!(b.session.prompt().text(), prompt("106", "—", "yok"));
    // A name given anew stays.
    b.click(20.0, 0.0);
    answer(&mut b, "A", Some("500"));
    b.undo_step();
    assert_eq!(b.session.prompt().text(), prompt("500", "—", "yok"));
}

#[test]
fn named_point_gives_the_running_tool_its_place_as_if_clicked() {
    let mut b = bench();
    add_point(&mut b, [3.0, 4.0], Some(" 101 "));
    add_point(&mut b, [9.0, 9.0], Some("102"));
    add_point(&mut b, [8.0, 8.0], Some("102"));
    b.doc
        .add(Entity::Text(TextEntity {
            base: common::base("cizim"),
            p: kentos_contracts::Vec2 {
                x: E + 1.0,
                y: N + 1.0,
            },
            text: "103".into(),
            height: 1.0,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
            path: None,
        }))
        .expect("a slot");
    b.start("line");
    assert!(b.type_text("#101"));
    assert!(b.type_text("  # 102 "));
    assert_eq!(
        b.last_text(),
        Some("#102: bu adda 2 nokta var; koordinatı yazın.")
    );
    // A text is no point.
    assert!(b.type_text("#103"));
    assert_eq!(b.last_text(), Some("#103: bu adda nokta yok."));
    assert_eq!(b.last_level(), Some(Level::Warn));
    // “#” alone is no name: the tool's (and not understood).
    assert!(!b.type_text("#"));
    assert!(b.type_text(&typed_at(7.0, 7.0)));
    let line = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Line(l) => Some(l.clone()),
            _ => None,
        })
        .expect("a line");
    assert_eq!((rel(line.a), rel(line.b)), ([3.0, 4.0], [7.0, 7.0]));
}

#[test]
fn named_point_says_so_where_no_point_is_asked() {
    let mut b = bench();
    add_point(&mut b, [3.0, 4.0], Some("101"));
    b.start("move");
    assert!(b.type_text("#101"));
    assert_eq!(b.last_text(), Some("#101: bu adımda nokta istenmiyor."));
}
