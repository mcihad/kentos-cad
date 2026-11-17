//! Yüksek tehlikeli yerler (BYKHY 8/1-ğ, 17): a place its unit's nature names as one (an LPG cylinder seller) is read
//! with Ek-5/B's high-hazard row, and two exits are asked above 25 people (39/2); the same room as a shop asks neither.

use serde_json::json;

use crate::fire_escape_tests::{blank, doors, fire, rooms, run, walls};

/// A one-storey 15 × 10 m sales room of 30 people (5 m² a person, its axis outline) with one door in its south wall,
/// in a unit whose nature is `nature`: its fire findings' messages.
fn sales_room(nature: &str) -> Vec<String> {
    let mut app = blank();
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}
        ] } }),
    );
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [{"id": "t1", "number": "1", "nature": nature}]}}),
    );
    let w = walls(
        &mut app,
        "zemin",
        &[
            ((0.0, 0.0), (15.0, 0.0), 0.2),
            ((15.0, 0.0), (15.0, 10.0), 0.2),
            ((15.0, 10.0), (0.0, 10.0), 0.2),
            ((0.0, 10.0), (0.0, 0.0), 0.2),
        ],
    );
    rooms(&mut app, "zemin", &[((7.5, 5.0), "Satış alanı", "t1")]);
    doors(&mut app, &[(&w[0], 7.5, 1.0)]);
    fire(&mut app).into_iter().map(|(_, m)| m).collect()
}

#[test]
fn a_high_hazard_place_asks_two_exits_above_25_people_and_its_own_distances() {
    let lpg = sales_room("Tüp bayii");
    assert!(
        lpg.iter().any(|m| m
            == "Satış alanı kişi yükü 30: en az 2 çıkış gerekir, 1 var — BYKHY 39/2 (9.9.2009)"),
        "{lpg:#?}"
    );
    assert!(
        lpg.iter()
            .any(|m| m.starts_with("“Zemin kat” kişi yükü 30: en az 2 çıkış gerekir, 1 var")),
        "{lpg:#?}"
    );
    // Ek-5/B's high-hazard row: 10 m one way without a sprinkler system.
    assert!(
        lpg.iter().any(
            |m| m.starts_with("Satış alanı en uzak noktasından kat çıkışına ")
                && m.ends_with(
                    "> 10.00 m (yüksek tehlikeli yer, tek yön) — BYKHY 32/4, Ek-5/B (9.7.2015)"
                )
        ),
        "{lpg:#?}"
    );
    // The same room as a shop: 30 people need one exit, the walk is within 15 m.
    let shop = sales_room("Dükkân");
    assert!(
        !shop
            .iter()
            .any(|m| m.contains("en az 2 çıkış") || m.contains("yüksek tehlikeli")),
        "{shop:#?}"
    );
}
