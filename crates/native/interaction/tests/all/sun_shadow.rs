//! Gölge (`tool.sunShadow`, docs/adr/8177 §3.3): the building's shadow previewed as the settings change; Enter draws it
//! on MM-XXXX-GOLGE in one undo step and the tool leaves. Without the project's coordinate system it asks for the
//! latitude (E) and longitude (B), kept for the next run; the day's hours from 09:00 to 15:00 are each outlined with
//! its time.

use crate::common::{Bench, E, N};
use kentos_commands::{CadWork, Entity, Workspace};
use kentos_interaction::{Format, Level};
use serde_json::json;

/// Zemin kat 3 m high with a wall 6 m long east–west; the tool started.
fn bench() -> Bench {
    let mut b = Bench::new("line");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.cad_work = Some(CadWork::Architecture);
    let levels = json!({"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]});
    s.arch = Some(serde_json::from_value(levels).unwrap());
    b.doc.set_settings(s);
    let wall: Entity = serde_json::from_value(json!({"kind": "wall", "id": 0, "layerId": "cizim", "attrs": {},
        "pts": [{"x": E, "y": N}, {"x": E + 6.0, "y": N}], "width": 0.25, "height": 3.0, "arch": {"level": "zemin"}}))
    .unwrap();
    b.doc.add(wall).expect("a slot");
    b.level = Some("zemin".into());
    b.memory.layer_keys = true;
    b.start("sunShadow");
    b
}

/// Sivas: 39.75 N, 37.02 E, typed.
fn in_sivas(b: &mut Bench) {
    for t in ["E", "39,75", "B", "37,02"] {
        assert!(b.type_text(t), "{t}");
    }
}

fn shadow_objects(b: &Bench) -> Vec<&Entity> {
    b.doc.entities().filter(|e| e.base().attrs.contains_key("Gölge")).collect()
}

#[test]
fn without_the_project_s_place_it_asks_for_the_latitude_and_longitude() {
    let mut b = bench();
    assert_eq!(b.session.tool_id(), "sunShadow");
    assert!(b.session.preview(&Format::default()).is_none_or(|p| p.strokes.is_empty()));
    // A letter reaches the tool only as the prompt's option: Enlem and Boylam are offered.
    let options = b.options();
    assert!(["T", "S", "E", "B"].iter().all(|k| options.contains(k)), "{options:?}");
    assert!(b.session.prompt().text().contains("binanın enlemini (E) ve boylamını (B) verin"), "{:?}", b.session.prompt());
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.last_text(), Some("Projenin koordinat sistemi yok: binanın enlemini (E) ve boylamını (B) verin."));
    assert_eq!(b.session.tool_id(), "sunShadow", "it stays for the place");
    assert!(b.type_text("E"));
    assert!(!b.type_text("kuzey") || b.last_level() == Some(Level::Warn), "a word is no latitude");
}

#[test]
fn enter_draws_the_hour_s_shadow_filled_on_its_layer_in_one_step() {
    let mut b = bench();
    in_sivas(&mut b);
    let preview = b.session.preview(&Format::default()).expect("a preview");
    assert!(!preview.strokes.is_empty(), "the shadow's outline previewed");
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Success));
    let said = b.last_text().unwrap();
    assert!(said.starts_with("Gölge çizildi: 21 Aralık 12:00, güneş yüksekliği 26,"), "{said}");
    assert!(said.ends_with("azimut 172° (kuzey oku yok, plan kuzeyi coğrafi kuzey sayıldı)."), "{said}");
    let mut kinds: Vec<&str> = shadow_objects(&b)
        .iter()
        .map(|e| match e {
            Entity::Hatch(_) => "hatch",
            Entity::Polyline(_) => "outline",
            Entity::Text(_) => "time",
            _ => "other",
        })
        .collect();
    kinds.sort_unstable();
    assert_eq!(kinds, ["hatch", "outline", "time"]);
    for e in shadow_objects(&b) {
        let name = b.doc.layers().get(&e.base().layer_id).map(|l| l.name.clone());
        assert_eq!(name.as_deref(), Some("MM-XXXX-GOLGE"));
    }
    assert_ne!(b.session.tool_id(), "sunShadow", "the tool leaves once it is drawn");
    assert_eq!(b.doc.undo().as_deref(), Some("Gölge"));
    assert!(shadow_objects(&b).is_empty(), "one undo step");
}

#[test]
fn the_day_s_hours_are_outlined_each_with_its_time_and_drawn_again_they_replace() {
    let mut b = bench();
    in_sivas(&mut b);
    // 12:00 → 15:00 → 09:00–15:00 saatlik.
    assert!(b.type_text("S") && b.type_text("S"));
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Gölge çizildi: 21 Aralık 09:00–15:00 saatlik, 7 saat (kuzey oku yok, plan kuzeyi coğrafi kuzey sayıldı).")
    );
    let mut times: Vec<String> = shadow_objects(&b)
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    times.sort();
    assert_eq!(times, ["09:00", "10:00", "11:00", "12:00", "13:00", "14:00", "15:00"]);
    assert!(!shadow_objects(&b).iter().any(|e| matches!(e, Entity::Hatch(_))), "several hours are not filled");
    // Drawn again for 21 Haziran at 12:00, half an hour before the sun's noon in Sivas: the old shadows go. The place
    // and the hours typed and chosen are kept from the last run.
    b.start("sunShadow");
    let kept = b.session.prompt();
    assert!(format!("{kept:?}").contains("09:00–15:00 saatlik"), "{kept:?}");
    assert!(b.run(|s, cx| s.configure("T", "haziran", cx)));
    assert!(b.run(|s, cx| s.configure("S", "12", cx)));
    b.confirm();
    let said = b.last_text().unwrap();
    assert!(said.starts_with("Gölge çizildi: 21 Haziran 12:00, güneş yüksekliği 72,3°, azimut 154°"), "{said}");
    assert_eq!(shadow_objects(&b).iter().filter(|e| matches!(e, Entity::Text(_))).count(), 1);
}
