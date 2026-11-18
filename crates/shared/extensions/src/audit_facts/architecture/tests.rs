use super::super::{LayerFacts, ObjectCounts, ProjectAuditFacts, Totals};
use super::*;

fn walk(length: f64, one_way: Option<f64>, path: &[[f64; 2]]) -> EscapeWalk {
    EscapeWalk {
        length,
        one_way,
        exits: one_way.map(|_| 1),
        path: path.to_vec(),
    }
}

fn room(uid: &str, net: f64, escape: Option<RoomEscape>) -> RoomFacts {
    RoomFacts {
        uid: uid.into(),
        number: None,
        name: Some("Ofis".into()),
        usage: None,
        kind: None,
        occupancy: Some(Occupancy::Office),
        level: Some("zemin".into()),
        unit: Some("of".into()),
        void: false,
        outdoor: false,
        broken: false,
        stair: false,
        net,
        axis: net + 1.0,
        narrow: Some(2.400_04),
        escape,
    }
}

fn unit(id: &str) -> UnitFacts {
    UnitFacts {
        id: id.into(),
        number: "1".into(),
        name: None,
        kind: GroupKind::Unit,
        nature: Some("Daire".into()),
        annex_of: None,
        dwelling: true,
        occupancy: Some(super::Occupancy::Dwelling),
        gross: 80.000_04,
        net: 65.123_449,
        annex_gross: 0.0,
        annex_net: 0.0,
        total: 80.000_04,
        share: Some(12.345_66),
        general: Some(92.345_69),
        levels: vec![
            UnitLevel {
                level: Some("k1".into()),
                gross: 40.0,
                net: 30.0,
            },
            UnitLevel {
                level: Some("a0".into()),
                gross: 40.000_04,
                net: 35.123_449,
            },
        ],
    }
}

fn facts() -> ArchitectureFacts {
    ArchitectureFacts {
        levels: vec![
            Storey {
                id: "k1".into(),
                name: "1. kat".into(),
                elevation: 3.0,
                height: Some(3.000_4),
            },
            Storey {
                id: "zemin".into(),
                name: "Zemin kat".into(),
                elevation: -0.0,
                height: None,
            },
        ],
        exits: vec![
            StoreyExits {
                level: Some("zemin".into()),
                count: 2,
            },
            StoreyExits {
                level: None,
                count: 0,
            },
        ],
        rooms: vec![
            room(
                "u2",
                12.345_678,
                Some(RoomEscape {
                    exit: Some(walk(
                        22.519_6,
                        Some(22.519_6),
                        &[[16.000_4, 8.6], [12.0, 4.75]],
                    )),
                    storey_exits: true,
                    entrance: None,
                    has_entrance: false,
                    passages: 1,
                }),
            ),
            room("u1", 3.0, None),
        ],
        units: vec![unit("d1")],
        doors: vec![DoorEscape {
            uid: "door".into(),
            unit: "d1".into(),
            level: Some("zemin".into()),
            at: [15.000_4, 4.0],
            exit: Some(walk(12.0, Some(12.0), &[[15.0, 4.3]])),
            into_stair: false,
        }],
        sprinklers: true,
        zoning: Some(ZoningFacts {
            parcel: 300.0,
            far: Some(1.5),
            coverage: Some(0.4),
            floors: 486.123_456,
            included: 450.0,
            exempt: 171.123_456,
            outside: 12.0,
            cap: Some(135.0),
            over: 36.123_456,
            footprint: 125.000_04,
            far_source: Some("UIP-1 Merkez uygulama imar planı (onay 2024-05-14), karar K1".into()),
            coverage_source: None,
        }),
    }
}

/// The section's rules: storeys by elevation, rooms and doors by id, a group's storeys by id; areas to 0.0001 m², walks
/// and places to 1 mm, −0 as 0; doing it again changes nothing.
#[test]
fn the_architecture_is_rounded_and_ordered_as_written_down() {
    let (a, cut) = facts().normalized();
    assert!(!cut);
    let levels: Vec<(&str, f64, Option<f64>)> = a
        .levels
        .iter()
        .map(|l| (l.id.as_str(), l.elevation, l.height))
        .collect();
    assert_eq!(levels, [("zemin", 0.0, None), ("k1", 3.0, Some(3.0))]);
    assert!(a.levels[0].elevation.is_sign_positive(), "−0 is 0");
    assert_eq!(
        a.exits.iter().map(|e| e.level.clone()).collect::<Vec<_>>(),
        [None, Some("zemin".to_owned())]
    );
    let rooms: Vec<(&str, f64, f64)> = a
        .rooms
        .iter()
        .map(|r| (r.uid.as_str(), r.net, r.axis))
        .collect();
    assert_eq!(rooms, [("u1", 3.0, 4.0), ("u2", 12.3457, 13.3457)]);
    assert_eq!(a.rooms[0].narrow, Some(2.4), "a width to 1 mm");
    let exit = a.rooms[1]
        .escape
        .as_ref()
        .and_then(|e| e.exit.as_ref())
        .expect("a walk");
    assert_eq!(
        (exit.length, exit.one_way, exit.path.clone()),
        (22.52, Some(22.52), vec![[16.0, 8.6], [12.0, 4.75]])
    );
    let u = &a.units[0];
    assert_eq!(
        (u.gross, u.net, u.share, u.general),
        (80.0, 65.1234, Some(12.3457), Some(92.3457))
    );
    assert_eq!(
        u.levels
            .iter()
            .map(|l| (l.level.clone(), l.gross))
            .collect::<Vec<_>>(),
        [(Some("k1".to_owned()), 40.0), (Some("a0".to_owned()), 40.0)],
        "the project's storeys in their order, one it does not name last"
    );
    assert_eq!(a.doors[0].at, [15.0, 4.0]);
    let z = a.zoning.as_ref().expect("the zoning");
    assert_eq!(
        (z.floors, z.over, z.footprint, z.far, z.cap),
        (486.1235, 36.1235, 125.0, Some(1.5), Some(135.0))
    );
    // A project without İmar durumu writes no zoning: its facts are what they were.
    let mut bare = facts();
    bare.zoning = None;
    assert!(!serde_json::to_string(&bare).unwrap().contains("zoning"));
    let (again, _) = a.clone().normalized();
    assert_eq!(again, a);
}

/// A list holds at most `MOST_ITEMS` rooms, the first by id; the facts then say they were cut.
#[test]
fn too_many_rooms_cut_the_facts() {
    let mut a = facts();
    a.rooms = (0..MOST_ITEMS + 2)
        .rev()
        .map(|i| room(&format!("u{i:06}"), 1.0, None))
        .collect();
    let made = ProjectAuditFacts::new(0.001, Vec::new(), false).with_architecture(a);
    assert!(made.cut);
    let rooms = &made.architecture.as_ref().expect("the section").rooms;
    assert_eq!(rooms.len(), MOST_ITEMS);
    assert_eq!(rooms[0].uid, "u000000");
}

fn layer() -> LayerFacts {
    LayerFacts {
        id: "duvar".into(),
        name: "Duvar".into(),
        objects: ObjectCounts::default(),
        area: 0.0,
        length: 0.0,
        totals: Totals::default(),
        invalid: Vec::new(),
        overlaps: Vec::new(),
        gaps: Vec::new(),
    }
}

/// Without architecture the facts are the layers' alone, the same bytes as before the section; with it, its keys are
/// camelCase and it reads back as written.
#[test]
fn the_section_is_absent_without_rooms_and_reads_back_with_them() {
    let bare = ProjectAuditFacts::new(0.001, vec![layer()], false);
    assert!(
        !bare.to_json().contains("architecture"),
        "{}",
        bare.to_json()
    );
    let mut with_kind = facts();
    with_kind.rooms[0].kind = Some(RoomKind::Bedroom);
    let made = bare.clone().with_architecture(with_kind);
    let text = made.to_json();
    for key in [
        r#""architecture":{"levels":"#,
        r#""storeyExits":true"#,
        r#""hasEntrance":false"#,
        r#""oneWay":22.52"#,
        r#""annexGross":0.0"#,
        r#""annexOf":null"#,
        r#""kind":"unit""#,
        r#""kind":"bedroom""#,
        r#""narrow":2.4"#,
        r#""intoStair":false"#,
        r#""sprinklers":true"#,
    ] {
        assert!(text.contains(key), "{key}: {text}");
    }
    assert!(text.ends_with(r#""cut":false}"#), "{text}");
    let read: ProjectAuditFacts = serde_json::from_str(&text).unwrap();
    assert_eq!(read, made);
    let old: ProjectAuditFacts = serde_json::from_str(&bare.to_json()).unwrap();
    assert_eq!(old.architecture, None, "a reader of the layers alone");
}

/// The facts with architecture meet the committed schema; a group of another kind does not.
#[cfg(feature = "schema")]
#[test]
fn the_section_meets_the_schema() {
    let schema = crate::audit_facts::json_schema();
    let value = serde_json::to_value(
        ProjectAuditFacts::new(0.001, vec![layer()], false).with_architecture(facts()),
    )
    .unwrap();
    let errors = crate::schema_check::errors(&schema, &value);
    assert!(errors.is_empty(), "{errors:#?}");
    let mut odd = value;
    odd["architecture"]["units"][0]["kind"] = serde_json::json!("apartment");
    assert!(!crate::schema_check::errors(&schema, &odd).is_empty());
}
