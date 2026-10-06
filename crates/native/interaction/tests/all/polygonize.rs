//! Toplu alan (docs/adr/0151 §8) through the session, over the native
//! document: the line work and labels taken when it starts, the finding
//! shown first, Adalar and the attribute's name, the areas written in one
//! step on the active layer; as the web's
//! `apps/web/src/tools/polygonizeTool.test.ts` walks it, the same cases moved
//! to (E, N). Values worked out by hand from the rules of §2–§5; the core
//! itself is checked against the independent reference in the core's
//! `tests/polygonize.rs`.

use crate::common;

use common::{Bench, E, N};
use kentos_contracts::{Entity, PathEntity, PointEntity, TextEntity, Vec2};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_interaction::polygonize::label_value;

fn bench() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    b.draft.snap = false;
    b
}

fn pt(x: f64, y: f64) -> Vec2 {
    Vec2 { x: E + x, y: N + y }
}

fn add(b: &mut Bench, e: Entity) -> Slot {
    b.doc.add(e).expect("a slot")
}

/// A 40 × 30 block drawn as a frame and a cross: four 20 × 15 parcels.
fn block(b: &mut Bench, zs: Option<Vec<Option<f64>>>) {
    add(
        b,
        Entity::Polyline(PathEntity {
            base: common::base("cizim"),
            pts: vec![
                pt(0.0, 0.0),
                pt(40.0, 0.0),
                pt(40.0, 30.0),
                pt(0.0, 30.0),
                pt(0.0, 0.0),
            ],
            bulges: None,
            holes: None,
            zs,
            parts: None,
        }),
    );
    b.add_line("cizim", [20.0, 0.0], [20.0, 30.0]);
    b.add_line("cizim", [0.0, 15.0], [40.0, 15.0]);
}

fn text(b: &mut Bench, at: [f64; 2], value: &str) -> Slot {
    add(
        b,
        Entity::Text(TextEntity {
            base: common::base("cizim"),
            p: pt(at[0], at[1]),
            text: value.into(),
            height: 1.0,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
        }),
    )
}

/// The new areas.
fn areas(b: &Bench) -> Vec<PathEntity> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Polygon(p) => Some(p.clone()),
            _ => None,
        })
        .collect()
}

/// The area whose box holds `at` (east and north from (E, N)).
fn holding(list: &[PathEntity], at: [f64; 2]) -> Option<&PathEntity> {
    let p = pt(at[0], at[1]);
    list.iter().find(|a| {
        let xs = a.pts.iter().map(|q| q.x);
        let ys = a.pts.iter().map(|q| q.y);
        xs.clone().fold(f64::INFINITY, f64::min) <= p.x
            && xs.fold(f64::NEG_INFINITY, f64::max) >= p.x
            && ys.clone().fold(f64::INFINITY, f64::min) <= p.y
            && ys.fold(f64::NEG_INFINITY, f64::max) >= p.y
    })
}

fn attrs(a: Option<&PathEntity>) -> Vec<(String, String)> {
    a.expect("an area")
        .base
        .attrs
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn one(k: &str, v: &str) -> Vec<(String, String)> {
    vec![(k.to_owned(), v.to_owned())]
}

const OPTIONS: &str = "[Adalar (A): açık / Öznitelik (Ö): Ad / Uygula (Enter)]";

#[test]
fn makes_every_parcel_of_the_block_an_area_with_its_number_in_one_step() {
    let mut b = bench();
    block(&mut b, None);
    text(&mut b, [8.0, 6.0], "101/1");
    text(&mut b, [28.0, 6.0], "101/2");
    text(&mut b, [8.0, 21.0], "101/3");
    text(&mut b, [28.0, 21.0], "101/4");
    let before = b.log.len();
    b.start("polygonize");
    assert_eq!(
        b.said(before),
        [(
            Level::Info,
            "Toplu alan: 4 alan. Enter ile uygulayın (bütün çizim: 3 çizgi, 4 etiket)."
        )]
    );
    assert_eq!(
        b.session.prompt().text(),
        format!("Toplu alan: 4 alan {OPTIONS}")
    );
    assert_eq!(b.options(), ["A", "Ö", "Enter"]);
    b.move_to(50.0, 10.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    assert_eq!(
        preview.tag.map(|t| t.lines),
        Some(vec!["4 alan".to_owned(), "Enter: uygula".to_owned()])
    );
    assert_eq!(preview.areas.len(), 4);
    b.confirm();
    let made = areas(&b);
    assert_eq!(made.len(), 4);
    assert!(made.iter().all(|a| a.base.layer_id == "cizim"));
    assert_eq!(attrs(holding(&made, [10.0, 7.0])), one("Ad", "101/1"));
    assert_eq!(attrs(holding(&made, [30.0, 7.0])), one("Ad", "101/2"));
    assert_eq!(attrs(holding(&made, [10.0, 22.0])), one("Ad", "101/3"));
    assert_eq!(attrs(holding(&made, [30.0, 22.0])), one("Ad", "101/4"));
    assert_eq!(
        b.last_text(),
        Some("Toplu alan: 4 alan oluşturuldu; “Ad” 4 alana yazıldı.")
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.undo().as_deref(), Some("Toplu alan"));
    assert!(areas(&b).is_empty());
}

#[test]
fn says_what_to_look_at_takes_adalar_and_the_name_and_leaves_the_rest_empty() {
    let mut b = bench();
    block(&mut b, None);
    b.add_line("cizim", [5.0, 5.0], [12.0, 9.0]);
    text(&mut b, [2.0, 2.0], "101/1");
    text(&mut b, [13.0, 11.0], "fazla");
    text(&mut b, [28.0, 6.0], "101/2");
    add(
        &mut b,
        Entity::Point(PointEntity {
            base: kentos_contracts::EntityBase {
                label: Some("sınır".into()),
                ..common::base("cizim")
            },
            p: pt(20.0, 7.0),
            z: None,
            parts: None,
        }),
    );
    text(&mut b, [28.0, 21.0], "101/4");
    let before = b.log.len();
    b.start("polygonize");
    assert_eq!(
        b.said(before),
        [
            (
                Level::Info,
                "Toplu alan: 4 alan; 1 etiketsiz, 1 çok etiketli, 1 etiket sınırda, 2 uç boşta. Enter ile uygulayın (bütün çizim: 4 çizgi, 5 etiket)."
            ),
            (
                Level::Warn,
                "Toplu alan: 2 çizgi ucu boşta; kapanmayan bölge alan olmaz. Önce Topolojik temizlik'i deneyin."
            ),
            (
                Level::Warn,
                "Toplu alan: bir bölgede 2 etiket: “101/1”, “fazla”."
            ),
            (
                Level::Warn,
                "Toplu alan: “sınır” etiketi bir sınırın üstünde; hiçbir bölgeye verilmedi."
            ),
        ]
    );
    let said = b.log.len();
    // Ö asks for the name in a text field by the cursor, the old one in it.
    b.move_to(50.0, 10.0);
    assert!(b.type_text("Ö"));
    assert_eq!(
        b.session.prompt().text(),
        "Toplu alan: öznitelik adını yazın (şimdi: Ad)"
    );
    let Some(kentos_interaction::ViewChange::Text(field)) = b.views.last() else {
        panic!("a text field: {:?}", b.views.last());
    };
    assert_eq!(
        (field.at, field.initial.as_deref(), field.placeholder),
        (
            kentos_interaction::Vec2::new(E + 50.0, N + 10.0),
            Some("Ad"),
            Some("Öznitelik adı")
        )
    );
    b.run(|s, cx| s.text_typed(Some("  "), cx));
    assert_eq!(b.last_text(), Some("Öznitelik adı boş olamaz."));
    b.type_text("Ö");
    b.run(|s, cx| s.text_typed(Some("Parsel"), cx));
    assert_eq!(
        b.session.prompt().text(),
        "Toplu alan: 4 alan; 1 etiketsiz, 1 çok etiketli, 1 etiket sınırda, 2 uç boşta [Adalar (A): açık / Öznitelik (Ö): Parsel / Uygula (Enter)]"
    );
    // O answers too, on a keyboard without Ö; Esc keeps the name.
    b.type_text("o");
    b.run(|s, cx| s.text_typed(None, cx));
    assert!(b.session.prompt().text().contains("Öznitelik (Ö): Parsel"));
    b.type_text("a");
    assert!(b.session.prompt().text().contains("Adalar (A): kapalı"));
    // Nothing changed in what is found: nothing more is said.
    assert_eq!(b.log.len(), said + 1);
    b.confirm();
    let made = areas(&b);
    assert_eq!(made.len(), 4);
    assert_eq!(attrs(holding(&made, [30.0, 7.0])), one("Parsel", "101/2"));
    assert_eq!(attrs(holding(&made, [30.0, 22.0])), one("Parsel", "101/4"));
    assert!(attrs(holding(&made, [10.0, 7.0])).is_empty());
    assert!(attrs(holding(&made, [10.0, 22.0])).is_empty());
    assert_eq!(
        b.last_text(),
        Some(
            "Toplu alan: 4 alan oluşturuldu; “Parsel” 2 alana yazıldı. Özniteliği boş kalan: 1 etiketsiz, 1 çok etiketli."
        )
    );
    // Kept for as long as the app lives.
    b.start("polygonize");
    assert!(
        b.session
            .prompt()
            .text()
            .contains("Adalar (A): kapalı / Öznitelik (Ö): Parsel")
    );
}

#[test]
fn does_not_write_an_area_again_and_with_a_selection_reads_only_it() {
    let mut b = bench();
    block(&mut b, None);
    b.add_path(
        "cizim",
        &[[0.0, 0.0], [20.0, 0.0], [20.0, 15.0], [0.0, 15.0]],
        true,
    );
    b.start("polygonize");
    assert_eq!(
        b.session.prompt().text(),
        format!("Toplu alan: 3 alan; 3 etiketsiz, 1 zaten alan {OPTIONS}")
    );
    b.confirm();
    assert_eq!(areas(&b).len(), 4);
    b.doc.undo();
    // The frame and the divider only: two halves.
    let ids: Vec<Slot> = b
        .doc
        .entities()
        .take(2)
        .map(|e| Slot(e.base().id))
        .collect();
    b.selection.set(ids);
    b.start("polygonize");
    assert_eq!(
        b.last_text(),
        Some("Toplu alan: 2 alan; 2 etiketsiz. Enter ile uygulayın (seçili 2 çizgi, 0 etiket).")
    );
}

#[test]
fn with_no_line_work_says_so_and_leaves() {
    let mut b = bench();
    text(&mut b, [0.0, 0.0], "101");
    b.start("polygonize");
    assert_eq!(
        b.last_text(),
        Some("Toplu alan: bölge kapatacak çizgi yok; çizgi, çoklu çizgi, yay ya da alan seçin.")
    );
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn carries_the_corners_elevations_from_the_line_work() {
    let mut b = bench();
    block(
        &mut b,
        Some(vec![
            Some(10.0),
            Some(11.0),
            Some(12.0),
            Some(13.0),
            Some(10.0),
        ]),
    );
    b.start("polygonize");
    b.confirm();
    let made = areas(&b);
    let first = holding(&made, [10.0, 7.0]).expect("the first parcel");
    let z = |x: f64, y: f64| {
        let i = first
            .pts
            .iter()
            .position(|q| q.x == E + x && q.y == N + y)
            .expect("a corner");
        first.zs.as_ref().expect("elevations")[i]
    };
    // On a frame vertex its elevation; on a frame edge the edge's; where the cross lines meet, none.
    assert_eq!(z(0.0, 0.0), Some(10.0));
    assert!((z(20.0, 0.0).unwrap() - 10.5).abs() < 1e-12);
    assert!((z(0.0, 15.0).unwrap() - 11.5).abs() < 1e-12);
    assert_eq!(z(20.0, 15.0), None);
}

#[test]
fn a_label_is_a_texts_value_or_a_points_label_trimmed() {
    let mut b = bench();
    let t = text(&mut b, [0.0, 0.0], " 101/7 ");
    assert_eq!(label_value(b.doc.get(t).unwrap()).as_deref(), Some("101/7"));
    let named = add(
        &mut b,
        Entity::Point(PointEntity {
            base: kentos_contracts::EntityBase {
                label: Some("P12".into()),
                ..common::base("cizim")
            },
            p: pt(0.0, 0.0),
            z: None,
            parts: None,
        }),
    );
    assert_eq!(
        label_value(b.doc.get(named).unwrap()).as_deref(),
        Some("P12")
    );
    let bare = b.add_point_z("cizim", [1.0, 1.0], 3.0);
    assert_eq!(label_value(b.doc.get(bare).unwrap()), None);
    let line = b.add_line("cizim", [0.0, 0.0], [1.0, 0.0]);
    assert_eq!(label_value(b.doc.get(line).unwrap()), None);
}
