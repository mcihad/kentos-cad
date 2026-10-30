//! The tools' options a program leaves out (docs/adr/0802): KentOS Mimari's Ölçülendirme has no Semt, its Nokta no
//! Ad, Kod nor the question where a point already is, its Çizgi records no measurements; KentOS GIS's Böl has no
//! Blok, its Çoklu çizgi and grip menu no widths. Every option is offered by default (kentos-cad, the traces, the
//! headless server); Kuzey oku's Coğrafi kuzey only in a drawing with a coordinate system, in every program. Expected
//! values are worked out by hand.

use crate::common;

use common::{Bench, E, N, rel};
use kentos_commands::{Entity, PathEntity, PointEntity};
use kentos_domain::Slot;
use kentos_interaction::grip_menu::{self, GripAction};
use kentos_interaction::{DimensionMode, Draft, Name, Offers, ToolOption, Vec2};

const TOOLS: &str = include_str!("../../../../../fixtures/interaction/v1/tools.kcad");

/// The empty drawing (TUREF / TM36), snapping off, the program's options but `left_out`.
fn without(left_out: &[ToolOption]) -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    b.draft.snap = false;
    b.draft.offers = Offers::without(left_out);
    b
}

/// Every option: kentos-cad's.
fn every() -> Bench {
    without(&[])
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

fn newest_path(b: &Bench) -> PathEntity {
    match b.newest() {
        Entity::Polyline(p) => p.clone(),
        other => panic!("a polyline, not {other:?}"),
    }
}

fn on_screen(de: f64, dn: f64) -> [f64; 2] {
    use kentos_interaction::View;
    common::Camera.to_screen(Vec2::new(E + de, N + dn))
}

#[test]
fn every_option_is_offered_unless_left_out() {
    assert_eq!(Offers::default(), Offers::ALL);
    assert_eq!(Draft::default().offers, Offers::ALL);
    for option in ToolOption::ALL {
        assert!(Offers::ALL.has(option), "{option:?}");
        let left = Offers::without(&[option]);
        assert!(!left.has(option), "{option:?}");
        for other in ToolOption::ALL.into_iter().filter(|o| *o != option) {
            assert!(left.has(other), "{option:?} leaves {other:?}");
        }
    }
    let gis = Offers::without(&[ToolOption::Blocks, ToolOption::Widths]);
    assert!(gis.has(ToolOption::Bearing) && gis.has(ToolOption::PointNames));
    assert!(!gis.has(ToolOption::Blocks) && !gis.has(ToolOption::Widths));
}

/// Semt is a survey's: listed and taken with every option, neither in a program without it; a Semt remembered from
/// before goes back to Hizalı there.
#[test]
fn the_dimension_offers_semt_where_a_survey_is_drawn() {
    let mut b = every();
    b.start("dimension");
    assert!(b.options().contains(&"T"), "{:?}", b.options());
    assert!(b.type_text("T"));
    assert_eq!(b.memory.dimension_mode, DimensionMode::Azimuth);
    assert!(b.session.prompt().text().contains("semt"));

    let mut b = without(&[ToolOption::Bearing]);
    b.start("dimension");
    assert!(!b.options().contains(&"T"), "{:?}", b.options());
    assert!(b.options().contains(&"E"), "Eğim stays: {:?}", b.options());
    assert!(!b.type_text("T"));
    assert_eq!(b.memory.dimension_mode, DimensionMode::Aligned);
    assert!(b.type_text("E"), "the other styles are taken");
    assert_eq!(b.memory.dimension_mode, DimensionMode::Slope);

    let mut b = without(&[ToolOption::Bearing]);
    b.memory.dimension_mode = DimensionMode::Azimuth;
    b.start("dimension");
    assert_eq!(b.memory.dimension_mode, DimensionMode::Aligned);
    assert!(b.session.prompt().text().starts_with("Ölçü: hizalı"));
}

/// A building's points have no names: Nokta offers Kot alone, writes no name or code, asks nothing where a point
/// already is and moves no name on.
#[test]
fn the_point_has_no_names_and_no_question_in_a_building() {
    let mut b = every();
    b.start("point");
    assert_eq!(b.options(), ["A", "K", "Z"]);

    let mut b = without(&[ToolOption::PointNames]);
    b.memory.point_name = Name::new("101").expect("a name");
    b.memory.point_code = Name::new("SN").expect("a code");
    b.start("point");
    assert_eq!(b.options(), ["Z"]);
    assert_eq!(
        b.session.prompt().text(),
        "Nokta: nokta konumunu belirtin [Kot (Z): yok]"
    );
    assert!(!b.type_text("A"));
    assert!(!b.type_text("K"));
    b.click(1.0, 1.0);
    b.click(1.0, 1.0);
    let written = points(&b);
    assert_eq!(
        written.len(),
        2,
        "no question: a second point where the first is"
    );
    for p in &written {
        assert_eq!(rel(p.p), [1.0, 1.0]);
        assert_eq!(p.base.label, None, "no name");
        assert!(!p.base.attrs.contains_key("Kod"), "no code");
    }
    assert_eq!(b.memory.point_name.as_str(), "101", "nothing moved on");
    // Kot stays.
    assert!(b.type_text("Z"));
    assert!(b.type_text("12.5"));
    b.click(2.0, 2.0);
    assert_eq!(points(&b).last().and_then(|p| p.z), Some(12.5));
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
}

/// A polar point typed records its length on the line with every option, nothing in a building (docs/adr/0180).
#[test]
fn the_line_records_typed_measurements_where_a_survey_is_drawn() {
    for (mut b, recorded) in [(every(), true), (without(&[ToolOption::Recorded]), false)] {
        b.start("line");
        assert!(b.type_text(&format!("{},{}", E, N)));
        assert!(b.type_text("10<45"));
        let Entity::Line(line) = b.newest().clone() else {
            panic!("a line");
        };
        assert_eq!(
            line.base.attrs.contains_key("Kayıtlı uzunluk"),
            recorded,
            "{:?}",
            line.base.attrs
        );
        if !recorded {
            assert!(line.base.attrs.is_empty(), "{:?}", line.base.attrs);
        }
    }
}

/// The tools' drawing with a block “Direk” (as drawing.rs's).
fn with_a_block() -> String {
    let mut drawing: serde_json::Value = serde_json::from_str(TOOLS).expect("the drawing");
    drawing["blocks"] = serde_json::json!([{
        "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "name": "Direk", "base": { "x": 0, "y": 0 },
        "entities": [{ "kind": "line", "id": 1, "layerId": "0", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 1, "y": 0 } }]
    }]);
    drawing.to_string()
}

/// KentOS GIS has no blocks: Böl lists neither Blok nor its Nokta and Hizala, takes none of their keys and places
/// points even with a block remembered.
#[test]
fn divide_has_no_blocks_in_a_gis() {
    let drawing = with_a_block();
    let mut b = Bench::on(&drawing);
    b.draft.snap = false;
    assert!(b.doc.set_active_layer("cizim"));
    b.start("divide");
    assert!(b.options().contains(&"B"));
    assert!(b.type_text("B"));
    assert!(b.options().contains(&"N") && b.options().contains(&"H"));

    let mut b = Bench::on(&drawing);
    b.draft.snap = false;
    b.draft.offers = Offers::without(&[ToolOption::Blocks]);
    assert!(b.doc.set_active_layer("cizim"));
    b.memory.divide_block = b.doc.block_named("Direk").map(|d| d.id);
    assert!(b.memory.divide_block.is_some());
    b.start("divide");
    for key in ["B", "N", "H"] {
        assert!(!b.options().contains(&key), "{key}: {:?}", b.options());
        assert!(!b.type_text(key), "{key}");
    }
    assert!(!b.session.prompt().text().contains("Blok"));
    // The 20 m line along y = 10: four parts, three points.
    b.click(-18.0, 10.0);
    assert!(b.type_text("4"));
    assert_eq!(b.last_text(), Some("3 nokta kondu."));
}

/// A polyline drawn in `b` from (0, 0) to (10, 0) to (10, 10), the session's width 2 m.
fn polyline(b: &mut Bench) -> PathEntity {
    b.memory.polyline_width = 2.0;
    b.memory.polyline_width_end = 2.0;
    b.start("polyline");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.click(10.0, 10.0);
    b.confirm();
    newest_path(b)
}

/// KentOS GIS has no polyline widths: Çoklu çizgi lists and takes no Genişlik and writes none, and the grip menu of
/// a polyline's edge has no Kenar genişliği….
#[test]
fn the_polyline_and_its_grips_have_no_widths_in_a_gis() {
    let mut b = every();
    b.start("polyline");
    assert!(b.options().contains(&"E"), "{:?}", b.options());
    let path = polyline(&mut b);
    assert!(path.widths.is_some(), "the session's width");
    let slot = Slot(b.newest().base().id);
    b.selection.set([slot]);
    let edge = b
        .run(|_, cx| grip_menu::at(on_screen(5.0, 0.0), cx))
        .expect("the first edge's middle");
    assert!(edge.actions().contains(&GripAction::EdgeWidth));
    assert!(
        b.run(|_, cx| grip_menu::segment_tool(edge, GripAction::EdgeWidth, cx))
            .is_some()
    );

    let mut b = without(&[ToolOption::Widths]);
    b.start("polyline");
    assert!(!b.options().contains(&"E"), "{:?}", b.options());
    assert!(!b.type_text("E"));
    b.click(0.0, 0.0);
    assert!(!b.options().contains(&"E"), "{:?}", b.options());
    assert!(!b.type_text("E"));
    assert!(b.type_text("Y"), "arc mode");
    assert!(!b.options().contains(&"E"), "{:?}", b.options());
    // A new polyline in its place.
    let path = polyline(&mut b);
    assert_eq!(path.widths, None, "no width written");
    let slot = Slot(b.newest().base().id);
    b.selection.set([slot]);
    let edge = b
        .run(|_, cx| grip_menu::at(on_screen(5.0, 0.0), cx))
        .expect("the first edge's middle");
    assert_eq!(edge.header(), "Kenar 1");
    assert!(!edge.actions().contains(&GripAction::EdgeWidth));
    assert!(edge.actions().contains(&GripAction::ArcBy), "Yay… stays");
    assert!(
        b.run(|_, cx| grip_menu::segment_tool(edge, GripAction::EdgeWidth, cx))
            .is_none()
    );
}

/// Kuzey oku offers Coğrafi kuzey in a drawing with a coordinate system alone, in every program: without one Yön
/// steps between Plan kuzeyi and Açı, and Coğrafi kuzey chosen is refused and said.
#[test]
fn the_north_arrow_offers_true_north_with_a_coordinate_system_alone() {
    let ways = |b: &Bench| -> Vec<String> {
        b.session
            .option_choices("Y")
            .into_iter()
            .map(|c| c.typed)
            .collect()
    };
    let mut b = every();
    b.start("northArrow");
    assert_eq!(ways(&b), ["plan", "coğrafi", "açı"]);
    assert!(b.type_text("Y"));
    assert!(
        b.session.prompt().text().contains("Yön (Y): Coğrafi kuzey"),
        "{}",
        b.session.prompt().text()
    );

    for left_out in [&[][..], &ToolOption::ALL[..]] {
        let mut b = without(left_out);
        let mut s = b.doc.settings().clone();
        s.srid = 0;
        b.doc.set_settings(s);
        b.start("northArrow");
        assert_eq!(ways(&b), ["plan", "açı"]);
        let setting = b
            .session
            .settings(&Default::default())
            .into_iter()
            .find(|s| s.key == "Y")
            .expect("Yön");
        assert_eq!(setting.choices.len(), 2);
        assert!(b.type_text("Y"));
        assert!(
            b.session.prompt().text().contains("Yön (Y): 0°"),
            "Plan kuzeyi to Açı: {}",
            b.session.prompt().text()
        );
        assert!(b.type_text("Y"));
        assert!(b.session.prompt().text().contains("Yön (Y): Plan kuzeyi"));
        let before = b.doc.revision();
        assert!(!b.run(|s, cx| s.configure("Y", "coğrafi", cx)));
        assert!(
            b.last_text()
                .is_some_and(|t| t.contains("coğrafi kuzey bulunamaz")),
            "{:?}",
            b.last_text()
        );
        assert!(b.session.prompt().text().contains("Yön (Y): Plan kuzeyi"));
        b.click(5.0, 5.0);
        assert_ne!(b.doc.revision(), before, "the plan's north is placed");
    }
}
