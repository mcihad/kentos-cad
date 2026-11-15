//! Dead-end corridors (BYKHY 4/i, Ek-5/B): a corridor's dead end, its walk's one-way part, against 15 m or, when the
//! project says the building has sprinklers, 20 m.

use super::*;

const RULE: &str = "cikmaz-koridor";

/// The first storey's corridor, whose only exit makes its whole walk a dead end (21.82 m as drawn).
fn upstairs(a: &mut Value) -> &mut Value {
    named(a, "Koridor", "k1")
}

fn dead_end(a: &mut Value, metres: f64) {
    let walk = &mut upstairs(a)["escape"]["exit"];
    walk["length"] = json!(metres);
    walk["oneWay"] = json!(metres);
}

/// Every independent unit a dwelling: the corridor is an apartment building's.
fn residential(a: &mut Value) {
    for u in a["units"].as_array_mut().unwrap() {
        if u["kind"] == "unit" {
            u["dwelling"] = json!(true);
        }
    }
}

/// The building as drawn has a shop and an office: no occupancy class in the facts, the limit every class but a high
/// hazard's shares. 21.82 m breaks 15 m without sprinklers and 20 m with them; 12 m and, with sprinklers, 19 m do not.
#[test]
fn a_corridor_s_dead_end_is_as_long_as_the_sprinkler_setting_allows() {
    let python = need_python!();
    let hall = uid(upstairs);
    let found = findings(&python, RULE, facts(|_| {}));
    assert_eq!(said(&found), [(Severity::Error, Some(hall.as_str()))]);
    assert!(
        says(
            &found[0],
            &[
                "çıkmaz koridor 21.82 m",
                "yağmurlama sistemi işaretli değil, sınır 15.00 m",
                "yüksek tehlikeli yerler dışında her sınıfın sınırı",
                "BYKHY md. 4/i ve Ek-5/B"
            ]
        ),
        "{}",
        found[0].message
    );
    assert!(findings(&python, RULE, facts(|a| dead_end(a, 12.0))).is_empty());
    let wet = |metres: f64| {
        facts(move |a| {
            a["sprinklers"] = json!(true);
            dead_end(a, metres);
        })
    };
    assert!(findings(&python, RULE, wet(19.0)).is_empty());
    let found = findings(&python, RULE, wet(21.819));
    assert_eq!(said(&found), [(Severity::Error, Some(hall.as_str()))]);
    assert!(
        says(&found[0], &["yağmurlama sistemi var, sınır 20.00 m"]),
        "{}",
        found[0].message
    );
}

/// An apartment building's corridor is read with the apartments' row (15/20 m), and 48/1 spares one of four storeys or
/// fewer; a dwelling's own hall is 48/2's, not read; an office's corridor is.
#[test]
fn an_apartment_building_s_corridor_takes_its_row_and_its_exemption() {
    let python = need_python!();
    assert!(
        findings(&python, RULE, facts(residential)).is_empty(),
        "48/1"
    );
    let tall = facts(|a| {
        residential(a);
        five_storeys(a);
    });
    let found = findings(&python, RULE, tall);
    let hall = uid(upstairs);
    assert_eq!(said(&found), [(Severity::Error, Some(hall.as_str()))]);
    assert!(
        says(&found[0], &["apartmanlar satırı", "sınır 15.00 m"]),
        "{}",
        found[0].message
    );
    let own_hall = facts(|a| {
        dead_end(a, 12.0);
        let bedroom = room(a, "K02");
        bedroom["kind"] = json!("corridor");
        bedroom["escape"]["exit"]["oneWay"] = json!(30.0);
    });
    assert!(findings(&python, RULE, own_hall).is_empty());
    let office_corridor = facts(|a| {
        dead_end(a, 12.0);
        room(a, "K04")["kind"] = json!("corridor");
    });
    let found = findings(&python, RULE, office_corridor);
    let archive = uid(|a| room(a, "K04"));
    assert_eq!(said(&found), [(Severity::Error, Some(archive.as_str()))]);
    assert!(
        says(&found[0], &["çıkmaz koridor 22.52 m"]),
        "{}",
        found[0].message
    );
    assert!(
        findings(&python, RULE, facts(older_kentos)).is_empty(),
        "an older KentOS gives no kinds"
    );
}
