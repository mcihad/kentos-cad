//! Planlı Alanlar İmar Yönetmeliği's rules: a dwelling's rooms (29/1 with 29/2, 29/4 and 5/25) and halls (29/3).

use super::*;

const RULE: &str = "piyesler";

/// A room added to `unit`, as its living room's facts with another number, name, kind, net area and narrow side.
fn add_room(
    a: &mut Value,
    unit: &str,
    number: &str,
    name: &str,
    kind: &str,
    net: f64,
    narrow: f64,
) {
    let mut added = rooms(a)
        .iter()
        .find(|r| r["unit"] == unit && r["kind"] == "living")
        .expect("the unit's living room")
        .clone();
    added["uid"] = json!(format!("sinama-{number}"));
    added["number"] = json!(number);
    added["name"] = json!(name);
    added["kind"] = json!(kind);
    added["net"] = json!(net);
    added["narrow"] = json!(narrow);
    rooms(a).push(added);
}

fn drop_room(a: &mut Value, number: &str) {
    rooms(a).retain(|r| r["number"] != number);
}

/// Both flats given a kitchen, a bathroom and a toilet: they then have every room 29/1 asks.
fn complete(a: &mut Value) {
    for (unit, p) in [("d1", "Z"), ("d2", "K")] {
        add_room(a, unit, &format!("{p}91"), "Mutfak", "kitchen", 8.0, 2.2);
        add_room(a, unit, &format!("{p}92"), "Banyo", "bath", 4.5, 1.8);
        add_room(a, unit, &format!("{p}93"), "WC", "toilet", 1.6, 1.1);
    }
}

/// Every room's kind unknown, as names that say none give it.
fn kinds_unknown(a: &mut Value) {
    for r in rooms(a) {
        r["kind"] = Value::Null;
    }
}

/// PAİY 29/1: every room of a known kind meets the table's net area and narrow side, else an error that says both.
#[test]
fn a_room_of_a_known_kind_meets_the_table_or_says_both_its_measures() {
    let python = need_python!();
    assert!(findings(&python, RULE, facts(complete)).is_empty());
    let small = facts(|a| {
        complete(a);
        room(a, "K03")["net"] = json!(8.5);
    });
    let found = findings(&python, RULE, small);
    let bedroom = uid(|a| room(a, "K03"));
    assert_eq!(said(&found), [(Severity::Error, Some(bedroom.as_str()))]);
    let both =
        "net alanı 8.50 m² (en az 9.00 m²), dar kenarı 3.85 m (en az 2.50 m); alanı yetmiyor";
    assert!(
        says(&found[0], &[both, "PAİY md. 29/1"]),
        "{}",
        found[0].message
    );
    let narrow = facts(|a| {
        complete(a);
        room(a, "Z02")["narrow"] = json!(2.8);
    });
    let found = findings(&python, RULE, narrow);
    let living = uid(|a| room(a, "Z02"));
    assert_eq!(said(&found), [(Severity::Error, Some(living.as_str()))]);
    assert!(
        says(
            &found[0],
            &["dar kenarı 2.80 m (en az 3.00 m); dar kenarı yetmiyor"]
        ),
        "{}",
        found[0].message
    );
}

/// A missing room is an error unless an exception the regulation gives may hold, one the facts cannot see (29/2's
/// bathroom holding the toilet, 29/4's kitchen niche, 5/25's bed niche): then it says it cannot decide.
#[test]
fn a_missing_room_is_an_error_unless_the_regulation_s_own_exception_may_hold() {
    let python = need_python!();
    use Severity::{Error, Info};
    // As drawn: each flat lacks a bathroom and a toilet; its living room may hold a kitchen niche.
    let found = findings(&python, RULE, facts(|_| {}));
    let severities: Vec<Severity> = found.iter().map(|f| f.severity).collect();
    assert_eq!(severities, [Info, Error, Error, Info, Error, Error]);
    assert!(says(
        &found[0],
        &["mutfak yok", "karar verilemedi", "PAİY md. 29/1, 29/4"]
    ));
    assert!(says(&found[1], &["banyo yok"]) && says(&found[2], &["tuvalet yok"]));
    // No room holds its own area and a kitchen's 3.30 m² (29/4): the kitchen is missing.
    let no_niche = facts(|a| {
        complete(a);
        drop_room(a, "Z91");
        room(a, "Z02")["net"] = json!(13.0);
        room(a, "Z03")["net"] = json!(10.0);
    });
    let found = findings(&python, RULE, no_niche);
    assert_eq!(said(&found), [(Error, None)]);
    assert!(says(
        &found[0],
        &[
            "mutfak yok",
            "hiçbir oda buna yetmiyor",
            "PAİY md. 29/1, 29/4"
        ]
    ));
    // Two rooms (29/2): the toilet may be in the bathroom, which then holds both rooms' areas, 4.20 m² (29/4).
    let without_toilet = |bath: f64| {
        facts(move |a| {
            complete(a);
            drop_room(a, "Z93");
            room(a, "Z92")["net"] = json!(bath);
        })
    };
    assert!(findings(&python, RULE, without_toilet(4.5)).is_empty());
    let found = findings(&python, RULE, without_toilet(3.5));
    assert_eq!(said(&found), [(Error, None)]);
    assert!(
        says(
            &found[0],
            &[
                "tuvalet yok",
                "en büyük banyo Z92 Banyo 3.50 m²",
                "PAİY md. 29/2, 29/4"
            ]
        ),
        "{}",
        found[0].message
    );
    // Four rooms: the toilet stands apart.
    let four = facts(|a| {
        complete(a);
        drop_room(a, "K93");
        add_room(a, "d2", "K94", "Yatak odası 3", "bedroom", 10.0, 2.8);
    });
    let found = findings(&python, RULE, four);
    assert_eq!(said(&found), [(Error, None)]);
    assert!(
        says(&found[0], &["tuvalet yok", "bu konutta 4 oda var"]),
        "{}",
        found[0].message
    );
    // A bed niche in the living room may stand for the bedroom (5/25): the facts cannot see it.
    let no_bedroom = facts(|a| {
        complete(a);
        drop_room(a, "Z03");
    });
    let found = findings(&python, RULE, no_bedroom);
    assert_eq!(said(&found), [(Info, None)]);
    assert!(says(
        &found[0],
        &["yatak odası yok", "yatak nişi", "karar verilemedi", "5/25"]
    ));
    // The living room has no exception.
    let no_living = facts(|a| {
        complete(a);
        room(a, "Z02")["kind"] = json!("bedroom");
    });
    let found = findings(&python, RULE, no_living);
    assert_eq!(said(&found), [(Error, None)]);
    assert!(
        says(&found[0], &["oturma odası yok"]),
        "{}",
        found[0].message
    );
    // A room whose name gives no kind but whose size fits may be the missing one: its name is to be corrected.
    let unnamed = facts(|a| {
        complete(a);
        room(a, "Z91")["kind"] = Value::Null;
        room(a, "Z91")["name"] = json!("Islak hacim");
    });
    let found = findings(&python, RULE, unnamed);
    assert_eq!(said(&found), [(Info, None)]);
    assert!(
        says(
            &found[0],
            &["Z91 Islak hacim mutfak olabilir", "adını düzeltin"]
        ),
        "{}",
        found[0].message
    );
}

/// Where no room's kind is known (names that give none, or an older KentOS), the rule looks only for the room of
/// 12.0 m² any living room needs.
#[test]
fn where_no_room_s_kind_is_known_only_a_room_of_twelve_square_metres_is_looked_for() {
    let python = need_python!();
    assert!(findings(&python, RULE, facts(kinds_unknown)).is_empty());
    assert!(findings(&python, RULE, facts(older_kentos)).is_empty());
    let small = facts(|a| {
        kinds_unknown(a);
        room(a, "Z02")["net"] = json!(11.5);
        room(a, "Z03")["net"] = json!(10.0);
    });
    let found = findings(&python, RULE, small);
    let living = uid(|a| room(a, "Z02"));
    assert_eq!(said(&found), [(Severity::Error, Some(living.as_str()))]);
    assert!(
        says(
            &found[0],
            &["türü adlarından okunamadı", "11.50 m²", "PAİY md. 29/1"]
        ),
        "{}",
        found[0].message
    );
}

/// PAİY 29/3: a hall or corridor is 1.20 m wide at least, the paragraph quoted; a shop's or an office's own hall is
/// not read.
#[test]
fn a_hall_or_corridor_is_one_metre_twenty_wide_at_least() {
    let python = need_python!();
    assert!(findings(&python, "koridor", facts(|_| {})).is_empty());
    let narrow = facts(|a| named(a, "Koridor", "zemin")["narrow"] = json!(1.1));
    let found = findings(&python, "koridor", narrow);
    let hall = uid(|a| named(a, "Koridor", "zemin"));
    assert_eq!(said(&found), [(Severity::Error, Some(hall.as_str()))]);
    let quoted = "“Hol ve koridor genişlikleri 1.20 metreden az olamaz.”";
    assert!(
        says(
            &found[0],
            &["net genişliği 1.10 m", quoted, "PAİY md. 29/3"]
        ),
        "{}",
        found[0].message
    );
    let office_hall = facts(|a| {
        room(a, "K04")["kind"] = json!("corridor");
        room(a, "K04")["narrow"] = json!(1.0);
    });
    assert!(findings(&python, "koridor", office_hall).is_empty());
}
