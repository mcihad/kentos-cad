//! A dwelling on two storeys (BYKHY 48/2's second and third sentences): its only door is not upstairs; an upper storey
//! without its own door is 70 m² at most.

use super::*;

const RULE: &str = "ara-katli-daire";

/// The first flat on two storeys, its upper one (the first storey) of `net` and `gross` square metres, in a building
/// of five storeys.
fn two_storeys(a: &mut Value, net: f64, gross: f64) {
    five_storeys(a);
    for u in a["units"].as_array_mut().unwrap() {
        if u["id"] == "d1" {
            u["levels"]
                .as_array_mut()
                .unwrap()
                .push(json!({ "level": "k1", "gross": gross, "net": net }));
        }
    }
}

#[test]
fn a_flat_on_two_storeys_keeps_its_only_door_downstairs_and_its_upper_storey_small() {
    let python = need_python!();
    assert!(findings(&python, RULE, facts(|_| {})).is_empty());
    let spared = facts(|a| {
        two_storeys(a, 75.0, 80.0);
        a["levels"].as_array_mut().unwrap().truncate(2);
    });
    assert!(findings(&python, RULE, spared).is_empty(), "48/1");
    assert!(findings(&python, RULE, facts(|a| two_storeys(a, 60.0, 68.0))).is_empty());
    let found = findings(&python, RULE, facts(|a| two_storeys(a, 75.0, 80.0)));
    assert_eq!(said(&found), [(Severity::Error, None)]);
    assert!(
        says(
            &found[0],
            &[
                "üst katta kapı yok",
                "75.00 m²",
                "70.00 m²'yi aşamaz",
                "BYKHY md. 48/2"
            ]
        ),
        "{}",
        found[0].message
    );
    let found = findings(&python, RULE, facts(|a| two_storeys(a, 66.0, 74.0)));
    assert_eq!(said(&found), [(Severity::Info, None)]);
    assert!(
        says(&found[0], &["net mi brüt mü", "karar verilemedi"]),
        "{}",
        found[0].message
    );
    // Its only door upstairs.
    let entrance = uid(|a| door(a, "d1"));
    let upstairs = facts(|a| {
        two_storeys(a, 60.0, 68.0);
        door(a, "d1")["level"] = json!("k1");
    });
    let found = findings(&python, RULE, upstairs);
    assert_eq!(said(&found), [(Severity::Error, Some(entrance.as_str()))]);
    assert!(
        says(&found[0], &["tek kapısı üst katta"]),
        "{}",
        found[0].message
    );
    // A door on each storey: the upper one has its own exit, its area is free.
    let two_doors = facts(|a| {
        two_storeys(a, 90.0, 95.0);
        let mut second = door(a, "d1").clone();
        second["uid"] = json!("sinama-kapi");
        second["level"] = json!("k1");
        a["doors"].as_array_mut().unwrap().push(second);
    });
    assert!(findings(&python, RULE, two_doors).is_empty());
    // Three storeys: which is "the upper one" the regulation does not say.
    let three = facts(|a| {
        two_storeys(a, 90.0, 95.0);
        for u in a["units"].as_array_mut().unwrap() {
            if u["id"] == "d1" {
                u["levels"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({ "level": "k2", "gross": 50.0, "net": 45.0 }));
            }
        }
    });
    assert!(findings(&python, RULE, three).is_empty());
}
