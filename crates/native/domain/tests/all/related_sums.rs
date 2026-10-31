//! A form's calculation reading the record's related records (docs/adr/0761): the document works the
//! parent out again in the same step whenever one of its related records is added, changed, linked,
//! unlinked or removed; a grandparent reading the parents' sums follows in the next round; undo takes
//! the whole step back.

use kentos_contracts::{DocumentSnapshotV1, Entity, ProjectForm, ProjectRelation};
use kentos_domain::{Document, Slot};

fn layer(id: &str, fields: &str) -> String {
    format!(
        r#"{{"id":"{id}","name":"{id}","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{{"color":"ink","lineType":"continuous","lineWeight":0.25}},"children":[],"fields":[{fields}]}}"#
    )
}

fn form(id: &str, layer: &str, items: &str) -> ProjectForm {
    let body = format!(r#"{{"revision":1,"items":[{items}]}}"#);
    let digest =
        kentos_contracts::body_digest(&serde_json::from_str(&body).expect("a form's body"));
    serde_json::from_str(&format!(
        r#"{{"id":"{id}","name":"{id}","layer":"{layer}","draft":{body},
        "releases":[{{"version":1,"digest":"{digest}","body":{body}}}],"enforce":true}}"#
    ))
    .expect("a form")
}

fn relation(id: &str, name: &str, parent: &str, child: &str, field: &str) -> ProjectRelation {
    serde_json::from_value(serde_json::json!({
        "id": id, "name": name, "parent": parent, "child": child, "field": field
    }))
    .expect("a relation")
}

/// An island (ada) of buildings (bina), each with owners (malik); the building counts its owners and sums
/// their shares, the island sums its buildings' owners.
fn doc() -> Document {
    let text = |n: &str| format!(r#"{{"name":"{n}","kind":"text"}}"#);
    let layers = [
        layer(
            "ada",
            &[
                text("Ad"),
                r#"{"name":"Toplam","kind":"integer"}"#.to_owned(),
            ]
            .join(","),
        ),
        layer(
            "bina",
            &[
                text("Kod"),
                text("ada"),
                r#"{"name":"Sayı","kind":"integer"}"#.to_owned(),
                r#"{"name":"Hisse","kind":"decimal"}"#.to_owned(),
            ]
            .join(","),
        ),
        layer(
            "malik",
            &[
                text("Ad"),
                text("bina"),
                r#"{"name":"Pay","kind":"decimal"}"#.to_owned(),
            ]
            .join(","),
        ),
    ]
    .join(",");
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Ada","settings":{{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
        "origin":{{"x":0,"y":0}},"layers":[{layers}],"activeLayer":"bina","entities":[],
        "styles":{{"items":[],"categories":[]}}}}"#
    );
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("reads"))
        .expect("opens");
    let mut settings = doc.settings().clone();
    settings.relations = vec![
        relation("ada-bina", "Binalar", "ada", "bina", "ada"),
        relation("bina-malik", "Malikler", "bina", "malik", "bina"),
    ];
    settings.forms = vec![
        form(
            "bina-formu",
            "bina",
            r#"{"type":"field","id":"kod","field":"Kod"},
               {"type":"field","id":"sayi","field":"Sayı","calculate":"ilişkili_toplam('Malikler', 'say')"},
               {"type":"field","id":"hisse","field":"Hisse","calculate":"ilişkili_toplam('Malikler', 'topla', Pay)"}"#,
        ),
        form(
            "ada-formu",
            "ada",
            r#"{"type":"field","id":"ad","field":"Ad"},
               {"type":"field","id":"toplam","field":"Toplam","calculate":"ilişkili_toplam('Binalar', 'topla', Sayı)"}"#,
        ),
    ];
    doc.set_settings(settings);
    assert_eq!(doc.settings().forms.len(), 2, "the forms hold");
    doc
}

fn record(id: u32, layer: &str, attrs: &[(&str, &str)]) -> Entity {
    let attrs: serde_json::Map<String, serde_json::Value> = attrs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), serde_json::Value::from(*v)))
        .collect();
    serde_json::from_value(serde_json::json!({
        "kind": "point", "id": id, "layerId": layer, "attrs": attrs, "p": {"x": f64::from(id), "y": 0}
    }))
    .expect("a point")
}

fn attr(doc: &Document, slot: Slot, name: &str) -> Option<String> {
    doc.get(slot)
        .and_then(|e| e.base().attrs.get(name).cloned())
}

#[test]
fn a_parent_counts_its_related_records_in_the_same_step() {
    let mut doc = doc();
    let ada = doc.add(record(1, "ada", &[("Ad", "101")])).expect("a slot");
    let ada_uid = doc.uid(ada).expect("a uid").to_string();
    let bina = doc
        .add(record(2, "bina", &[("Kod", "B-1"), ("ada", &ada_uid)]))
        .expect("a slot");
    let bina_uid = doc.uid(bina).expect("a uid").to_string();
    assert_eq!(attr(&doc, bina, "Sayı").as_deref(), Some("0"));
    assert_eq!(attr(&doc, ada, "Toplam").as_deref(), Some("0"));

    // An owner added: the building counts it, the island sums the building's count in the next round.
    let m1 = doc
        .add(record(
            3,
            "malik",
            &[("Ad", "Ayşe"), ("bina", &bina_uid), ("Pay", "0.5")],
        ))
        .expect("a slot");
    assert_eq!(attr(&doc, bina, "Sayı").as_deref(), Some("1"));
    assert_eq!(attr(&doc, bina, "Hisse").as_deref(), Some("0.5"));
    assert_eq!(attr(&doc, ada, "Toplam").as_deref(), Some("1"));
    let m2 = doc
        .add(record(
            4,
            "malik",
            &[("Ad", "Ali"), ("bina", &bina_uid), ("Pay", "0.25")],
        ))
        .expect("a slot");
    assert_eq!(attr(&doc, bina, "Sayı").as_deref(), Some("2"));
    assert_eq!(attr(&doc, bina, "Hisse").as_deref(), Some("0.75"));
    assert_eq!(attr(&doc, ada, "Toplam").as_deref(), Some("2"));

    // One undo takes the owner, the count and the island's sum back together.
    doc.undo().expect("an undo");
    assert_eq!(attr(&doc, bina, "Sayı").as_deref(), Some("1"));
    assert_eq!(attr(&doc, ada, "Toplam").as_deref(), Some("1"));
    doc.redo().expect("a redo");
    assert_eq!(attr(&doc, bina, "Sayı").as_deref(), Some("2"));

    // A share changed: the sum follows.
    let mut e = doc.get(m2).expect("the owner").clone();
    e.base_mut().attrs.insert("Pay".into(), "0.5".into());
    assert!(doc.update(m2, e));
    assert_eq!(attr(&doc, bina, "Hisse").as_deref(), Some("1"));

    // Unlinked: the building no longer counts it.
    let mut e = doc.get(m1).expect("the owner").clone();
    e.base_mut().attrs.remove("bina");
    assert!(doc.update(m1, e));
    assert_eq!(attr(&doc, bina, "Sayı").as_deref(), Some("1"));
    assert_eq!(attr(&doc, bina, "Hisse").as_deref(), Some("0.5"));
    assert_eq!(attr(&doc, ada, "Toplam").as_deref(), Some("1"));

    // Removed: the count drops to 0.
    assert_eq!(doc.remove(&[m2]), 1);
    assert_eq!(attr(&doc, bina, "Sayı").as_deref(), Some("0"));
    assert_eq!(attr(&doc, bina, "Hisse"), None);
    assert_eq!(attr(&doc, ada, "Toplam").as_deref(), Some("0"));
}

#[test]
fn an_owner_moved_to_another_building_changes_both() {
    let mut doc = doc();
    let b1 = doc
        .add(record(1, "bina", &[("Kod", "B-1")]))
        .expect("a slot");
    let b2 = doc
        .add(record(2, "bina", &[("Kod", "B-2")]))
        .expect("a slot");
    let (u1, u2) = (
        doc.uid(b1).expect("a uid").to_string(),
        doc.uid(b2).expect("a uid").to_string(),
    );
    let m = doc
        .add(record(
            3,
            "malik",
            &[("Ad", "Ece"), ("bina", &u1), ("Pay", "1")],
        ))
        .expect("a slot");
    assert_eq!(attr(&doc, b1, "Sayı").as_deref(), Some("1"));
    assert_eq!(attr(&doc, b2, "Sayı").as_deref(), Some("0"));
    let mut e = doc.get(m).expect("the owner").clone();
    e.base_mut().attrs.insert("bina".into(), u2.clone());
    assert!(doc.update(m, e));
    assert_eq!(attr(&doc, b1, "Sayı").as_deref(), Some("0"));
    assert_eq!(attr(&doc, b2, "Sayı").as_deref(), Some("1"));
    // A field of several ids counts for each.
    let mut e = doc.get(m).expect("the owner").clone();
    e.base_mut()
        .attrs
        .insert("bina".into(), format!("{u1}; {u2}"));
    assert!(doc.update(m, e));
    assert_eq!(attr(&doc, b1, "Sayı").as_deref(), Some("1"));
    assert_eq!(attr(&doc, b2, "Sayı").as_deref(), Some("1"));
}

/// docs/adr/0761 Doğrulama: one owner added among 10 000 owners of 1 000 buildings, the building and the
/// island worked out again in the same step; a step on a layer the forms do not read.
#[test]
#[ignore]
fn timing() {
    let mut doc = doc();
    let ada = doc.add(record(1, "ada", &[("Ad", "101")])).expect("a slot");
    let ada_uid = doc.uid(ada).expect("a uid").to_string();
    let buildings: Vec<Entity> = (0..1000)
        .map(|i| {
            record(
                2 + i,
                "bina",
                &[("Kod", &format!("B-{i}")), ("ada", &ada_uid)],
            )
        })
        .collect();
    let slots = doc.add_many(buildings, "Binalar").expect("slots");
    let uids: Vec<String> = slots
        .iter()
        .map(|s| doc.uid(*s).expect("a uid").to_string())
        .collect();
    let owners: Vec<Entity> = (0..10_000)
        .map(|i| {
            record(
                2000 + i,
                "malik",
                &[
                    ("Ad", "Malik"),
                    ("bina", &uids[(i % 1000) as usize]),
                    ("Pay", "0.1"),
                ],
            )
        })
        .collect();
    let started = std::time::Instant::now();
    doc.add_many(owners, "Malikler").expect("slots");
    eprintln!(
        "10 000 malik birden: {:.1} ms",
        started.elapsed().as_secs_f64() * 1e3
    );
    assert_eq!(attr(&doc, slots[0], "Sayı").as_deref(), Some("10"));
    assert_eq!(attr(&doc, ada, "Toplam").as_deref(), Some("10000"));
    let mut times = Vec::new();
    for i in 0..20 {
        let started = std::time::Instant::now();
        doc.add(record(
            30_000 + i,
            "malik",
            &[("Ad", "Yeni"), ("bina", &uids[0]), ("Pay", "0.1")],
        ))
        .expect("a slot");
        times.push(started.elapsed().as_secs_f64() * 1e3);
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "bir malik eklemek: ortanca {:.2} ms, en çok {:.2} ms",
        times[10], times[19]
    );
    assert_eq!(attr(&doc, slots[0], "Sayı").as_deref(), Some("30"));
    let mut times = Vec::new();
    for i in 0..20 {
        let started = std::time::Instant::now();
        doc.add(record(40_000 + i, "ada", &[("Ad", "başka")]))
            .expect("a slot");
        times.push(started.elapsed().as_secs_f64() * 1e3);
    }
    times.sort_by(f64::total_cmp);
    eprintln!("ilişkisiz bir ada eklemek: ortanca {:.2} ms", times[10]);
}
