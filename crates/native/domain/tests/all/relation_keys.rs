//! A relation by a parent key (docs/adr/9135), the municipal report's neighbourhoods and maintenance works: the
//! neighbourhoods' own codes (M1, M2, M3), the works' `mahalle` holding them (M1, M1, M2, M2, M9, none). M1 and M2 count
//! two works each, M3 none; M9 and the empty one belong to no neighbourhood. Deleting a neighbourhood follows its works
//! by the code (cascade, unlink, restrict), and a code another neighbourhood still holds keeps them.

use kentos_contracts::{DocumentSnapshotV1, Entity, ProjectForm, ProjectRelation, RelationDelete};
use kentos_domain::{Document, Slot, Tie};

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

fn relation(on_delete: RelationDelete) -> ProjectRelation {
    ProjectRelation {
        id: "mahalle-bakim".into(),
        name: "Bakımlar".into(),
        parent: "mahalle".into(),
        child: "bakim".into(),
        field: "mahalle".into(),
        parent_key: Some("kod".into()),
        on_delete,
    }
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

/// The neighbourhoods M1–M3 (each counting its works in Bakım), the works B01–B06; their slots.
fn drawing(on_delete: RelationDelete) -> (Document, Vec<Slot>, Vec<Slot>) {
    let text = |n: &str| format!(r#"{{"name":"{n}","kind":"text"}}"#);
    let layers = [
        layer(
            "mahalle",
            &[
                text("kod"),
                text("ad"),
                r#"{"name":"Bakım","kind":"integer"}"#.to_owned(),
            ]
            .join(","),
        ),
        layer("bakim", &[text("kod"), text("mahalle")].join(",")),
    ]
    .join(",");
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Mahalle","settings":{{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
        "origin":{{"x":0,"y":0}},"layers":[{layers}],"activeLayer":"bakim","entities":[],
        "styles":{{"items":[],"categories":[]}}}}"#
    );
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("reads"))
        .expect("opens");
    let mut settings = doc.settings().clone();
    settings.relations = vec![relation(on_delete)];
    settings.forms = vec![form(
        "mahalle-formu",
        "mahalle",
        r#"{"type":"field","id":"kod","field":"kod"},
           {"type":"field","id":"bakim","field":"Bakım","calculate":"ilişkili_toplam('Bakımlar', 'say')"}"#,
    )];
    doc.set_settings(settings);
    let parents: Vec<Slot> = [("M1", "Çınar"), ("M2", "İstasyon"), ("M3", "Kale")]
        .iter()
        .enumerate()
        .map(|(i, (kod, ad))| {
            doc.add(record(i as u32 + 1, "mahalle", &[("kod", kod), ("ad", ad)]))
                .expect("a slot")
        })
        .collect();
    let codes = [
        Some("M1"),
        Some("M1"),
        Some("M2"),
        Some("M2"),
        Some("M9"),
        None,
    ];
    let children: Vec<Slot> = codes
        .iter()
        .enumerate()
        .map(|(i, code)| {
            let kod = format!("B0{}", i + 1);
            let mut attrs = vec![("kod", kod.as_str())];
            if let Some(c) = code {
                attrs.push(("mahalle", c));
            }
            doc.add(record(i as u32 + 10, "bakim", &attrs))
                .expect("a slot")
        })
        .collect();
    (doc, parents, children)
}

fn attr(doc: &Document, slot: Slot, name: &str) -> Option<String> {
    doc.get(slot)
        .and_then(|e| e.base().attrs.get(name).cloned())
}

#[test]
fn a_parent_keys_children_are_those_holding_its_code() {
    let (doc, parents, children) = drawing(RelationDelete::Unlink);
    // The form's count by the code: two, two and none; M9 and the empty one are no neighbourhood's.
    let counts: Vec<Option<String>> = parents.iter().map(|p| attr(&doc, *p, "Bakım")).collect();
    assert_eq!(
        counts,
        [Some("2".into()), Some("2".into()), Some("0".into())]
    );
    // A trace's related records through it: M1's works, and B03's neighbourhood.
    let related = doc.related_of(&[parents[0]]);
    assert_eq!(related.len(), 1);
    assert_eq!(
        (related[0].tie, related[0].slots.clone()),
        (Tie::Attached, vec![children[0], children[1]])
    );
    let held = doc.related_of(&[children[2]]);
    assert_eq!(
        (held[0].tie, held[0].slots.clone()),
        (Tie::Container, vec![parents[1]])
    );
    assert!(
        doc.related_of(&[children[4], children[5]]).is_empty(),
        "M9 and none belong to no neighbourhood"
    );
}

#[test]
fn deleting_a_keyed_parent_follows_its_children_by_the_code() {
    // Cascade: M1's two works go with it, the others stay.
    let (mut doc, parents, children) = drawing(RelationDelete::Cascade);
    doc.remove(&[parents[0]]);
    assert!(doc.get(children[0]).is_none() && doc.get(children[1]).is_none());
    assert!(children[2..].iter().all(|c| doc.get(*c).is_some()));
    // Unlink: M2's works lose the code.
    let (mut doc, parents, children) = drawing(RelationDelete::Unlink);
    doc.remove(&[parents[1]]);
    assert_eq!(
        (
            attr(&doc, children[2], "mahalle"),
            attr(&doc, children[3], "mahalle")
        ),
        (None, None)
    );
    assert_eq!(attr(&doc, children[0], "mahalle").as_deref(), Some("M1"));
    // Restrict: M1 is kept by its works, M3 has none.
    let (doc, parents, _) = drawing(RelationDelete::Restrict);
    let uid = |s: Slot| doc.uid(s).expect("a uid");
    assert_eq!(doc.restricted_by(&[uid(parents[0])]).len(), 1);
    assert!(doc.restricted_by(&[uid(parents[2])]).is_empty());
    // A code another neighbourhood still holds keeps its works.
    let (mut doc, parents, children) = drawing(RelationDelete::Cascade);
    let twin = doc
        .add(record(99, "mahalle", &[("kod", "M1"), ("ad", "Çınar 2")]))
        .expect("a slot");
    doc.remove(&[parents[0]]);
    assert!(
        doc.get(children[0]).is_some() && doc.get(children[1]).is_some(),
        "the twin keeps them"
    );
    assert_eq!(attr(&doc, twin, "Bakım").as_deref(), Some("2"));
}
