//! A layer field's stable id (docs/adr/0401 §2): a field a form binds keeps
//! its key when it is renamed; an unbound field takes no id, so a drawing
//! without forms is written as it was.

use kentos_domain::Document;
use kentos_domain::contracts::{
    DocumentSnapshotV1, FormBody, FormField, FormItem, LayerField, LayerFieldKind, ProjectForm,
};

fn drawing() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Deneme","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads"))
        .expect("opens");
    let fields = vec![
        LayerField::new("Durum", LayerFieldKind::Text),
        LayerField::new("Kat", LayerFieldKind::Integer),
    ];
    assert_eq!(doc.set_layer_fields("a", fields, &[]), Ok(true));
    doc
}

fn bind(doc: &mut Document, key: &str) {
    let mut settings = doc.settings().clone();
    settings.forms = vec![ProjectForm {
        id: "f".into(),
        name: "Form".into(),
        layer: "a".into(),
        draft: FormBody {
            revision: 1,
            items: vec![FormItem::Field(FormField::new("k", key))],
            ..Default::default()
        },
        releases: Vec::new(),
        enforce: false,
    }];
    doc.set_settings(settings);
}

fn fields(doc: &Document) -> Vec<(Option<String>, String)> {
    let node = doc.layers().get("a").expect("the layer");
    node.fields
        .iter()
        .map(|f| (f.id.clone(), f.name.clone()))
        .collect()
}

fn renamed(doc: &mut Document, from: &str, to: &str, extra: Option<&str>) {
    let mut next: Vec<LayerField> = doc.layers().get("a").expect("layer").fields.clone();
    for f in &mut next {
        if f.name == from {
            f.name = to.into();
        }
    }
    if let Some(name) = extra {
        next.push(LayerField::new(name, LayerFieldKind::Text));
    }
    let renames = [(from.to_owned(), to.to_owned())];
    assert_eq!(doc.set_layer_fields("a", next, &renames), Ok(true));
}

#[test]
fn an_unbound_field_takes_no_id_when_renamed() {
    let mut doc = drawing();
    renamed(&mut doc, "Kat", "Kat sayısı", None);
    assert_eq!(
        fields(&doc),
        [(None, "Durum".into()), (None, "Kat sayısı".into())]
    );
}

#[test]
fn a_bound_field_keeps_its_key_when_renamed_and_after() {
    let mut doc = drawing();
    bind(&mut doc, "Kat");
    renamed(&mut doc, "Kat", "Kat sayısı", None);
    assert_eq!(
        fields(&doc),
        [
            (None, "Durum".into()),
            (Some("Kat".into()), "Kat sayısı".into())
        ]
    );
    // Its id stays through the next rename and through an edit that does not name it.
    renamed(&mut doc, "Kat sayısı", "Katlar", None);
    assert_eq!(fields(&doc)[1], (Some("Kat".into()), "Katlar".into()));
    let mut next = doc.layers().get("a").expect("layer").fields.clone();
    next[1].id = None;
    next[1].alias = Some("Kat adedi".into());
    assert_eq!(doc.set_layer_fields("a", next, &[]), Ok(true));
    assert_eq!(fields(&doc)[1], (Some("Kat".into()), "Katlar".into()));
}

#[test]
fn a_new_field_with_the_old_name_gets_a_fresh_id() {
    let mut doc = drawing();
    bind(&mut doc, "Kat");
    renamed(&mut doc, "Kat", "Kat sayısı", Some("Kat"));
    assert_eq!(
        fields(&doc),
        [
            (None, "Durum".into()),
            (Some("Kat".into()), "Kat sayısı".into()),
            (Some("Kat-2".into()), "Kat".into()),
        ]
    );
}

#[test]
fn undo_takes_the_id_back() {
    let mut doc = drawing();
    bind(&mut doc, "Kat");
    renamed(&mut doc, "Kat", "Kat sayısı", None);
    assert!(doc.undo().is_some());
    assert_eq!(fields(&doc), [(None, "Durum".into()), (None, "Kat".into())]);
}
