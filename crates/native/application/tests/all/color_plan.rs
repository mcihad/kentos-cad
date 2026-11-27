//! Mahal renk planı (docs/adr/8176): each room's colour by the project's scheme, the same in every storey and in the
//! legend, and the legend drawn and drawn again. Kullanım reads a room's group from its name or use (oturma, yatma,
//! mutfak, ıslak hacim, dolaşım, teknik, açık alan, şaft); a use no group reads is its own row; Bağımsız bölüm the
//! program's units in their order, the common and outdoor areas one row each, a room in no unit unfilled.

use kentos_commands::{ColorPlan, ColorScheme, Entity, Vec2};
use kentos_domain::Document;
use kentos_native_application::color_legend;
use serde_json::{Value, json};

/// A room 4 × 3 m at (x, 0) on `level`, named `name`, its use and unit given or not.
fn room(x: f64, level: &str, name: Option<&str>, usage: Option<&str>, unit: Option<&str>) -> Value {
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let mut v = json!({"kind": "space", "parts": [{"outer": {"pts": [p(x, 0.0), p(x + 4.0, 0.0), p(x + 4.0, 3.0), p(x, 3.0)]}}],
                       "arch": {"level": level}});
    for (key, value) in [("name", name), ("usage", usage), ("unit", unit)] {
        if let Some(value) = value {
            v[key] = json!(value);
        }
    }
    v
}

/// The drawing: Zemin kat and 1. kat, the program's units D1, D2, Ortak (common), Bahçe (outdoor) and D1's annex.
fn drawing(rooms: Vec<Value>, scheme: Option<&str>) -> Document {
    let mut snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "Renk planı", "origin": {"x": 0.0, "y": 0.0},
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 50.0, "workspace": "cad", "cadWork": "architecture",
            "arch": {"levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
            ], "program": {"units": [
                {"id": "d1", "number": "D1", "name": "Daire 1", "kind": "unit"},
                {"id": "d2", "number": "D2", "kind": "unit"},
                {"id": "ortak", "number": "Ortak", "kind": "common"},
                {"id": "bahce", "number": "Bahçe", "kind": "outdoor"},
                {"id": "depo1", "number": "D1-E", "kind": "annex", "annexOf": "d1"}
            ]}}},
        "layers": [{"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "0", "styles": {"items": [], "categories": []}, "uids": [], "entities": []
    });
    if let Some(s) = scheme {
        snapshot["settings"]["arch"]["colorPlan"] = json!({"scheme": s});
    }
    for (i, mut e) in rooms.into_iter().enumerate() {
        e["id"] = json!(i + 1);
        e["layerId"] = json!("0");
        e["attrs"] = json!({});
        snapshot["entities"].as_array_mut().unwrap().push(e);
        snapshot["uids"].as_array_mut().unwrap().push(json!(format!("0192f5a1-7242-7000-8000-{:012x}", i + 1)));
    }
    Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap()
}

/// Each room's fill by its id.
fn fills(doc: &Document) -> Vec<(u32, &'static str)> {
    let mut f = doc.space_fills();
    f.sort_unstable();
    f
}

#[test]
fn kullanim_colours_a_room_by_the_group_its_name_reads_as_and_a_use_by_its_own() {
    let rooms = vec![
        room(0.0, "zemin", Some("Salon"), None, None),
        room(4.0, "zemin", Some("Ebeveyn yatak odası"), None, None),
        room(8.0, "zemin", Some("Banyo"), None, None),
        room(12.0, "zemin", Some("Mutfak"), None, None),
        room(16.0, "zemin", Some("Giriş holü"), None, None),
        room(20.0, "zemin", Some("Balkon"), None, None),
        room(24.0, "zemin", Some("Kazan dairesi"), None, None),
        room(28.0, "zemin", Some("Oda 7"), Some("Arşiv"), None),
        room(32.0, "zemin", Some("Oda 8"), Some("arşiv "), None),
        room(36.0, "zemin", Some("Oda 9"), Some("Ofis"), None),
        room(40.0, "zemin", Some("Kiler"), None, None),
        room(44.0, "k1", Some("Salon"), None, None),
    ];
    assert!(drawing(rooms.clone(), None).space_fills().is_empty(), "no colour plan, no fill");
    let doc = drawing(rooms, Some("usage"));
    assert_eq!(doc.color_plan(), Some(ColorScheme::Usage));
    let f = fills(&doc);
    assert_eq!(f.len(), 12, "every room filled");
    // The groups' own colours; both salons alike on both storeys; a use as written its own, folded (Arşiv, arşiv).
    assert_eq!((f[0].1, f[11].1), ("#FDB462", "#FDB462"));
    assert_eq!(f[2].1, "#80B1D3", "Banyo: ıslak hacim");
    assert_eq!(f[7].1, f[8].1, "one use, one colour");
    assert_ne!(f[7].1, f[9].1, "two uses, two colours");
    let legend = doc.color_legend(Some("zemin"));
    let rows: Vec<(&str, usize)> = legend.iter().map(|r| (r.label.as_str(), r.rooms)).collect();
    assert_eq!(
        rows,
        [
            ("Oturma", 1),
            ("Yatma", 1),
            ("Mutfak", 1),
            ("Islak hacim", 1),
            ("Dolaşım", 1),
            ("Teknik hacim", 1),
            ("Açık alan", 1),
            ("Arşiv", 2),
            ("Ofis", 1),
            ("Diğer", 1),
        ]
    );
    assert!(legend.iter().all(|r| (r.area - 12.0 * r.rooms as f64).abs() < 1e-9), "each room 12 m²");
    assert_eq!(doc.color_legend(None).iter().map(|r| r.rooms).sum::<usize>(), 12, "every storey's");
}

#[test]
fn bagimsiz_bolum_colours_the_units_in_their_order_and_leaves_a_room_in_none_unfilled() {
    let doc = drawing(
        vec![
            room(0.0, "zemin", Some("Salon"), None, Some("d1")),
            room(4.0, "zemin", Some("Salon"), None, Some("d2")),
            room(8.0, "zemin", Some("Merdiven"), None, Some("ortak")),
            room(12.0, "zemin", Some("Bahçe"), None, Some("bahce")),
            room(16.0, "zemin", Some("Depo"), None, Some("depo1")),
            room(20.0, "zemin", Some("Hol"), None, None),
        ],
        Some("unit"),
    );
    let f = fills(&doc);
    assert_eq!(f.len(), 5, "the room in no unit is not filled");
    assert_eq!(f[0].1, f[4].1, "an annex its unit's colour");
    assert_ne!(f[0].1, f[1].1, "two units, two colours");
    let rows: Vec<(String, Option<&str>, usize)> =
        doc.color_legend(None).into_iter().map(|r| (r.label, r.color, r.rooms)).collect();
    assert_eq!(
        rows,
        [
            ("D1 — Daire 1".to_owned(), Some(f[0].1), 2),
            ("D2".to_owned(), Some(f[1].1), 1),
            ("Ortak alanlar".to_owned(), Some("#D9D9D9"), 1),
            ("Dış alanlar".to_owned(), Some("#B3DE69"), 1),
            ("Bölüme atanmamış".to_owned(), None, 1),
        ]
    );
}

#[test]
fn the_legend_is_drawn_and_drawn_again_in_one_place() {
    let mut doc = drawing(
        vec![room(0.0, "zemin", Some("Salon"), None, None), room(4.0, "zemin", Some("Banyo"), None, None)],
        Some("usage"),
    );
    let marked = |doc: &Document| {
        doc.entities().filter(|e| e.base().attrs.contains_key(color_legend::MARK_ATTR)).count()
    };
    let rows = color_legend::draw(&mut doc, Some("zemin"), Vec2 { x: 0.0, y: -2.0 }, "0").expect("drawn");
    assert_eq!(rows, 2);
    // The title, each row's swatch, outline and words, the total.
    assert_eq!(marked(&doc), 1 + 2 * 3 + 1);
    let texts: Vec<String> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Text(t) if t.base.attrs.contains_key(color_legend::MARK_ATTR) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        [
            "Mahal renk planı: Kullanım (Zemin kat)",
            "Oturma — 1 mahal, 12,00 m²",
            "Islak hacim — 1 mahal, 12,00 m²",
            "Toplam — 2 mahal, 24,00 m²",
        ]
    );
    // Again elsewhere: the old legend goes.
    color_legend::draw(&mut doc, Some("zemin"), Vec2 { x: 20.0, y: -2.0 }, "0").expect("drawn again");
    assert_eq!(marked(&doc), 8);
    // Without a colour plan it says where to choose one, and writes nothing.
    let mut s = doc.settings().clone();
    s.arch.as_mut().unwrap().color_plan = None;
    doc.set_settings(s);
    let why = color_legend::draw(&mut doc, Some("zemin"), Vec2 { x: 0.0, y: 0.0 }, "0").expect_err("no plan");
    assert!(why.contains("Renk planı kapalı"), "{why}");
}

#[test]
fn a_colour_plan_round_trips_under_arch_data_44() {
    let mut doc = drawing(vec![room(0.0, "zemin", Some("Salon"), None, None)], None);
    let mut s = doc.settings().clone();
    s.arch.as_mut().unwrap().color_plan = Some(ColorPlan { scheme: ColorScheme::Unit });
    doc.set_settings(s);
    let snapshot = doc.to_snapshot_v2();
    assert!(kentos_kcad::uses_color_plan(&snapshot));
    assert_eq!(kentos_kcad::arch_version(&snapshot), kentos_kcad::ARCH_WITH_COLOR_PLAN);
    let back = kentos_kcad::decode(&kentos_kcad::encode(&snapshot).expect("written")).expect("read");
    assert_eq!(back.settings.arch.and_then(|a| a.color_plan), Some(ColorPlan { scheme: ColorScheme::Unit }));
}

/// A fill is drawn where its rooms' layer is (docs/adr/8176 §3.4): a layer of rooms over a layer of doors covers it;
/// rooms at the bottom cover nothing; a hidden layer is none.
#[test]
fn the_layers_a_fill_covers_are_said() {
    use kentos_domain::NewLayer;
    let mut doc = drawing(vec![room(0.0, "zemin", Some("Salon"), None, None)], Some("usage"));
    let doors = doc.add_layer(NewLayer::layer("Kapılar"), None, false).unwrap();
    let e: Entity = serde_json::from_value(json!({"kind": "line", "id": 0, "layerId": doors, "attrs": {},
        "a": {"x": 1.0, "y": 1.0}, "b": {"x": 2.0, "y": 1.0}}))
    .unwrap();
    doc.add(e).unwrap();
    // "0" (the rooms') is first, over "Kapılar".
    assert_eq!(doc.covered_layers(), [("0".to_owned(), vec!["Kapılar".to_owned()])]);
    assert_eq!(doc.move_layer(&doors, None, Some(0)), Ok(true));
    assert!(doc.covered_layers().is_empty(), "the doors over the rooms: nothing covered");
}
