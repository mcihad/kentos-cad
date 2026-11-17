//! Binaların Yangından Korunması Hakkında Yönetmelik's rules: inside a dwelling (48/2), from its door (48/3, 48/4,
//! Ek-5/B), the storey's exits (31/3, 33/6-b); the limit the project's sprinkler setting chooses.

use super::*;

/// BYKHY 48/2: inside a dwelling its door is 20 m away at most, 30 m when the project says the building has sprinklers;
/// a dwelling building of four storeys or fewer is spared (48/1); a room that cannot reach its unit's door is an error.
/// From an older KentOS, whose facts do not carry the setting, both limits are said.
#[test]
fn inside_a_dwelling_its_door_is_as_far_as_the_sprinkler_setting_allows() {
    let python = need_python!();
    let rule = "daire-ici";
    let far = |length: f64, sprinklers: bool| {
        facts(move |a| {
            five_storeys(a);
            a["sprinklers"] = json!(sprinklers);
            room(a, "K03")["escape"]["entrance"]["length"] = json!(length);
        })
    };
    let spared = facts(|a| room(a, "K03")["escape"]["entrance"]["length"] = json!(24.0));
    assert!(
        findings(&python, rule, spared).is_empty(),
        "two storeys: 48/1"
    );
    assert!(findings(&python, rule, facts(five_storeys)).is_empty());
    let bedroom = uid(|a| room(a, "K03"));
    let found = findings(&python, rule, far(24.0, false));
    assert_eq!(said(&found), [(Severity::Error, Some(bedroom.as_str()))]);
    assert!(
        says(
            &found[0],
            &[
                "24.00 m",
                "yağmurlama sistemi işaretli değil, sınır 20.00 m",
                "BYKHY md. 48/2"
            ]
        ),
        "{}",
        found[0].message
    );
    assert!(findings(&python, rule, far(24.0, true)).is_empty());
    let found = findings(&python, rule, far(31.0, true));
    assert_eq!(said(&found), [(Severity::Error, Some(bedroom.as_str()))]);
    assert!(
        says(&found[0], &["yağmurlama sistemi var, sınır 30.00 m"]),
        "{}",
        found[0].message
    );
    let older = facts(|a| {
        five_storeys(a);
        older_kentos(a);
        room(a, "K03")["escape"]["entrance"]["length"] = json!(24.0);
    });
    let found = findings(&python, rule, older);
    assert_eq!(said(&found), [(Severity::Warning, Some(bedroom.as_str()))]);
    assert!(
        says(
            &found[0],
            &["eski KentOS", "binada yağmurlama sistemi yoksa"]
        ),
        "{}",
        found[0].message
    );
    let cut_off = facts(|a| {
        five_storeys(a);
        room(a, "K03")["escape"]["entrance"] = Value::Null;
    });
    let found = findings(&python, rule, cut_off);
    assert_eq!(said(&found), [(Severity::Error, Some(bedroom.as_str()))]);
    assert!(
        says(&found[0], &["yürüyüş yolu yok"]),
        "{}",
        found[0].message
    );
}

/// BYKHY 32/3, 48/4, Ek-5/B: from a dwelling's door to the nearest storey exit, the apartments' row in the column the
/// sprinkler setting chooses: one way 15 m (30 m), two ways 30 m (75 m).
#[test]
fn from_a_dwelling_s_door_the_exit_is_as_ek_5b_says_for_apartments() {
    let python = need_python!();
    let rule = "daire-kapisi";
    assert!(findings(&python, rule, facts(five_storeys)).is_empty());
    let walk = |unit: &'static str, length: f64, one_way: f64, sprinklers: bool| {
        facts(move |a| {
            five_storeys(a);
            a["sprinklers"] = json!(sprinklers);
            let walk = &mut door(a, unit)["exit"];
            walk["length"] = json!(length);
            walk["oneWay"] = json!(one_way);
        })
    };
    let second = uid(|a| door(a, "d2"));
    let found = findings(&python, rule, walk("d2", 18.0, 18.0, false));
    assert_eq!(said(&found), [(Severity::Error, Some(second.as_str()))]);
    assert!(
        says(
            &found[0],
            &[
                "tek yönde 18.00 m",
                "sınır 15.00 m",
                "BYKHY md. 32/3, 48/4 ve Ek-5/B"
            ]
        ),
        "{}",
        found[0].message
    );
    assert!(findings(&python, rule, walk("d2", 18.0, 18.0, true)).is_empty());
    let found = findings(&python, rule, walk("d2", 32.0, 32.0, true));
    assert_eq!(said(&found), [(Severity::Error, Some(second.as_str()))]);
    assert!(says(&found[0], &["sınır 30.00 m"]), "{}", found[0].message);
    // Two exits: a short shared part, a long whole walk.
    let first = uid(|a| door(a, "d1"));
    let found = findings(&python, rule, walk("d1", 40.0, 5.0, false));
    assert_eq!(said(&found), [(Severity::Error, Some(first.as_str()))]);
    assert!(
        says(&found[0], &["iki yönde 40.00 m", "sınır 30.00 m"]),
        "{}",
        found[0].message
    );
    assert!(findings(&python, rule, walk("d1", 40.0, 5.0, true)).is_empty());
    let older = facts(|a| {
        five_storeys(a);
        older_kentos(a);
        let walk = &mut door(a, "d2")["exit"];
        walk["length"] = json!(18.0);
        walk["oneWay"] = json!(18.0);
    });
    let found = findings(&python, rule, older);
    assert_eq!(said(&found), [(Severity::Warning, Some(second.as_str()))]);
}

/// BYKHY 48/3 and 33/6-b: a dwelling's door opening into an escape stair's room stands at the storey's exit; one that
/// reaches no exit is an error in any building (48/1 spares distances, not the exit). From an older KentOS the two
/// cannot be told apart, and nothing is said.
#[test]
fn a_door_into_the_stair_is_at_the_exit_and_one_reaching_none_is_an_error() {
    let python = need_python!();
    let rule = "daire-kapisi";
    let without_walk = |into_stair: bool| {
        facts(move |a| {
            let d = door(a, "d2");
            d["exit"] = Value::Null;
            d["intoStair"] = json!(into_stair);
        })
    };
    assert!(findings(&python, rule, without_walk(true)).is_empty());
    let found = findings(&python, rule, without_walk(false));
    let second = uid(|a| door(a, "d2"));
    assert_eq!(said(&found), [(Severity::Error, Some(second.as_str()))]);
    assert!(
        says(
            &found[0],
            &[
                "hiçbir kat çıkışına ulaşılmıyor",
                "BYKHY md. 48/3 ve 33/6-b"
            ]
        ),
        "{}",
        found[0].message
    );
    let older = facts(|a| {
        older_kentos(a);
        door(a, "d2")["exit"] = Value::Null;
    });
    assert!(findings(&python, rule, older).is_empty());
}

/// BYKHY 31/3 and 33/6-b: every storey with rooms has a storey exit; every room reaches one through its own unit's
/// rooms and the common ones.
#[test]
fn every_storey_has_an_exit_and_every_room_reaches_one() {
    let python = need_python!();
    let rule = "cikis-erisimi";
    assert!(findings(&python, rule, facts(|_| {})).is_empty());
    let archive = uid(|a| room(a, "K04"));
    let cut_off = facts(|a| room(a, "K04")["escape"]["exit"] = Value::Null);
    let found = findings(&python, rule, cut_off);
    assert_eq!(said(&found), [(Severity::Error, Some(archive.as_str()))]);
    assert!(
        says(&found[0], &["BYKHY md. 33/6-b"]),
        "{}",
        found[0].message
    );
    let no_exit = facts(|a| {
        for e in a["exits"].as_array_mut().unwrap() {
            if e["level"] == "k1" {
                e["count"] = json!(0);
            }
        }
    });
    let found = findings(&python, rule, no_exit);
    assert_eq!(
        said(&found),
        [(Severity::Error, None)],
        "one for the storey"
    );
    assert!(
        found[0].message.starts_with("1. kat: katta kat çıkışı yok"),
        "{}",
        found[0].message
    );
    assert!(says(&found[0], &["BYKHY md. 31/3"]));
}
