//! What the project's relations tie to some objects (docs/adr/0525 §1): the children whose field
//! holds a reached parent's persistent id, and the parents a reached child names, one field of
//! many ids too; relations that tie none are left out.

use kentos_contracts::{DocumentSnapshotV1, EntityId, ProjectRelation, RelationDelete};
use kentos_domain::{Document, Slot, Tie};

fn layer(id: &str, fields: &str) -> String {
    format!(
        r#"{{"id":"{id}","name":"{id}","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{{"color":"ink","lineType":"continuous","lineWeight":0.25}},"children":[],"fields":[{fields}]}}"#
    )
}

/// Two pipes, three subscribers (two on pipe 1, one on pipe 2), a chamber holding a valve, an owner of
/// both pipes (a field of many ids).
fn doc() -> Document {
    let text = |n: &str| format!(r#"{{"name":"{n}","kind":"text"}}"#);
    let layers = [
        layer("hat", &text("Ad")),
        layer("abone", &[text("ad"), text("hat")].join(",")),
        layer("hazne", &text("no")),
        layer("vana", &[text("no"), text("hazne")].join(",")),
        layer(
            "malik",
            &[
                text("ad"),
                r#"{"name":"hatlar","kind":"text","multiple":true}"#.to_owned(),
            ]
            .join(","),
        ),
    ]
    .join(",");
    let point = |id: u32, layer: &str, x: f64| {
        format!(
            r#"{{"kind":"point","id":{id},"layerId":"{layer}","attrs":{{}},"p":{{"x":{x},"y":0}}}}"#
        )
    };
    let entities = [
        r#"{"kind":"line","id":1,"layerId":"hat","attrs":{},"a":{"x":0,"y":0},"b":{"x":100,"y":0}}"#.to_owned(),
        r#"{"kind":"line","id":2,"layerId":"hat","attrs":{},"a":{"x":100,"y":0},"b":{"x":200,"y":0}}"#.to_owned(),
        point(3, "abone", 10.0),
        point(4, "abone", 20.0),
        point(5, "abone", 150.0),
        point(6, "hazne", 100.0),
        point(7, "vana", 100.0),
        point(8, "malik", 300.0),
    ]
    .join(",");
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Şebeke","settings":{{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
        "origin":{{"x":0,"y":0}},"layers":[{layers}],"activeLayer":"hat","entities":[{entities}],
        "styles":{{"items":[],"categories":[]}}}}"#
    );
    let first = Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("reads"))
        .expect("opens");
    let mut v2 = first.to_snapshot_v2();
    let uid = |i: usize| {
        let mut id = [0x51u8; 16];
        id[8..].copy_from_slice(&(i as u64 + 1).to_be_bytes());
        EntityId(id)
    };
    for (i, u) in v2.uids.iter_mut().enumerate() {
        *u = uid(i);
    }
    let text_of = |i: usize| uuid::Uuid::from_bytes(uid(i).0).to_string();
    let set =
        |v2: &mut kentos_contracts::DocumentSnapshotV2, at: usize, field: &str, value: String| {
            v2.entities[at]
                .base_mut()
                .attrs
                .insert(field.to_owned(), value);
        };
    set(&mut v2, 2, "hat", text_of(0));
    set(&mut v2, 3, "hat", text_of(0));
    set(&mut v2, 4, "hat", text_of(1));
    set(&mut v2, 6, "hazne", text_of(5));
    set(
        &mut v2,
        7,
        "hatlar",
        format!("{}; {}", text_of(0), text_of(1)),
    );
    let relation = |id: &str, parent: &str, child: &str, field: &str| ProjectRelation {
        id: id.to_owned(),
        name: id.to_owned(),
        parent: parent.to_owned(),
        child: child.to_owned(),
        field: field.to_owned(),
        on_delete: RelationDelete::default(),
    };
    v2.settings.relations = vec![
        relation("Aboneler", "hat", "abone", "hat"),
        relation("Hazne", "hazne", "vana", "hazne"),
        relation("Malikler", "hat", "malik", "hatlar"),
        relation("Bos", "hazne", "abone", "ad"),
    ];
    Document::from_snapshot_v2(v2).expect("opens")
}

#[test]
fn a_reached_pipe_gives_its_subscribers_and_a_reached_valve_its_chamber() {
    let d = doc();
    let slot = |i: u32| Slot(i);
    // Pipe 1 and the valve reached: its two subscribers, its owner (one of many ids), the valve's chamber.
    let got = d.related_of(&[slot(1), slot(7)]);
    let groups: Vec<(&str, Tie, Vec<u32>)> = got
        .iter()
        .map(|r| {
            (
                r.relation.as_str(),
                r.tie,
                r.slots.iter().map(|s| s.0).collect(),
            )
        })
        .collect();
    assert_eq!(
        groups,
        vec![
            ("Aboneler", Tie::Attached, vec![3, 4]),
            ("Hazne", Tie::Container, vec![6]),
            ("Malikler", Tie::Attached, vec![8]),
        ]
    );
    // Pipe 2 alone: its one subscriber and the owner; nothing reached: nothing.
    let two = d.related_of(&[slot(2)]);
    assert_eq!(two.len(), 2);
    assert_eq!(two[0].slots, vec![slot(5)]);
    assert!(d.related_of(&[]).is_empty());
}
