//! Where an object template draws (docs/adr/0176 §3): the shared cases of
//! fixtures/style/v1/template-layers.json against `templates::find_layer`
//! over the document's own tree and lock rule, and opening a missing layer
//! inside its groups as one undo step. The web's are `model/objectTemplate.test.ts`.

use kentos_contracts::{DocumentSnapshotV1, LineType};
use kentos_domain::Document;
use kentos_interaction::templates::{
    LayerAnswer, Stamp, TemplateLayer, find_layer, locked_text, open_layer,
};
use serde_json::{Value, json};

use crate::common::Bench;

const CASES: &str = include_str!("../../../../../fixtures/style/v1/template-layers.json");

/// How a test draws one object with a tool.
type Draw<'a> = &'a dyn Fn(&mut Bench);

/// A case's tree as a layer node of the document's file: shown, open, plain.
fn node(n: &Value) -> Value {
    json!({
        "id": n["id"],
        "name": n["name"],
        "type": n["type"],
        "visible": true,
        "locked": n.get("locked").and_then(Value::as_bool).unwrap_or(false),
        "expanded": true,
        "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
        "children": n.get("children").and_then(Value::as_array).map_or(Vec::new(), |c| c.iter().map(node).collect()),
    })
}

/// A drawing of a case's tree; a layer of its own last, active, since a
/// drawing has one (no case's template names it).
fn drawing(tree: &[Value]) -> Document {
    let mut layers: Vec<Value> = tree.iter().map(node).collect();
    layers.push(node(
        &json!({ "id": "zz-test", "name": "Sınama", "type": "layer" }),
    ));
    let file = json!({
        "format": "kentos.document",
        "version": 1,
        "name": "Şablon katmanı",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow" },
        "origin": { "x": 487000, "y": 4420000 },
        "layers": layers,
        "activeLayer": "zz-test",
        "entities": [],
        "styles": { "items": [], "categories": [] },
    });
    let snapshot = DocumentSnapshotV1::from_json(&file.to_string()).expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("opens")
}

fn template_layer(v: &Value) -> TemplateLayer {
    TemplateLayer {
        path: v["path"]
            .as_array()
            .expect("a path")
            .iter()
            .map(|g| g.as_str().expect("a name").to_owned())
            .collect(),
        name: v["name"].as_str().expect("a name").to_owned(),
        ..TemplateLayer::default()
    }
}

#[test]
fn the_shared_cases_find_the_layer_the_template_draws_on() {
    let file: Value = serde_json::from_str(CASES).expect("the cases read");
    assert_eq!(file["format"], "kentos.template-layer-cases");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20, "{} cases", cases.len());
    for case in cases {
        let name = case["name"].as_str().unwrap_or_default();
        let doc = drawing(case["tree"].as_array().expect("a tree"));
        let layers = doc.layers();
        let got = find_layer(layers.nodes(), &template_layer(&case["layer"]), |id| {
            layers.is_locked(id)
        });
        let want = &case["result"];
        let expected = if let Some(id) = want.get("found") {
            LayerAnswer::Found(id.as_str().expect("an id").to_owned())
        } else if let Some(id) = want.get("locked") {
            LayerAnswer::Locked(id.as_str().expect("an id").to_owned())
        } else {
            let open = &want["open"];
            LayerAnswer::Open {
                parent: open["parent"].as_str().map(str::to_owned),
                create: open["create"]
                    .as_array()
                    .expect("groups")
                    .iter()
                    .map(|g| g.as_str().expect("a name").to_owned())
                    .collect(),
            }
        };
        assert_eq!(got, expected, "{name}");
    }
}

#[test]
fn a_missing_layer_opens_inside_its_groups_as_one_step_and_becomes_active() {
    let mut doc = drawing(&[
        json!({ "id": "kadastro", "name": "Kadastro", "type": "group", "children": [] }),
    ]);
    let layer = TemplateLayer {
        path: vec!["Kadastro".into(), "Ulaşım".into()],
        name: "Yol".into(),
        color: Some("#E5484D".into()),
        line_type: Some(LineType::Dashed),
        line_weight: Some(0.35),
    };
    let LayerAnswer::Open { parent, create } = find_layer(doc.layers().nodes(), &layer, |_| false)
    else {
        panic!("the drawing lacks it");
    };
    assert_eq!(
        (parent.as_deref(), create.as_slice()),
        (Some("kadastro"), &["Ulaşım".to_owned()][..])
    );
    let id = open_layer(&mut doc, &layer, parent.as_deref(), &create).expect("opens");
    let layers = doc.layers();
    assert_eq!(layers.active(), id);
    let node = layers.get(&id).expect("the layer");
    assert_eq!(
        (
            node.name.as_str(),
            node.style.color.as_str(),
            node.style.line_type,
            node.style.line_weight
        ),
        ("Yol", "#E5484D", LineType::Dashed, 0.35)
    );
    let group = layers.parent(&id).expect("its group");
    assert_eq!(group.name, "Ulaşım");
    assert_eq!(
        layers.parent(&group.id).map(|g| g.id.as_str()),
        Some("kadastro")
    );
    // Found by its name now.
    assert_eq!(
        find_layer(layers.nodes(), &layer, |_| false),
        LayerAnswer::Found(id.clone())
    );
    // One undo step, “Katman ekle”: the layer and its group go, the active layer comes back.
    assert_eq!(doc.undo().as_deref(), Some("Katman ekle"));
    assert!(!doc.can_undo());
    let layers = doc.layers();
    assert!(layers.get(&id).is_none());
    assert_eq!(layers.get("kadastro").map(|g| g.children.len()), Some(0));
    assert_eq!(layers.active(), "zz-test");
}

#[test]
fn a_locked_layer_or_group_is_said_by_its_name_and_the_templates() {
    let doc = drawing(&[
        json!({ "id": "kadastro", "name": "Kadastro", "type": "group", "locked": true, "children": [ { "id": "parsel", "name": "Parsel", "type": "layer" } ] }),
    ]);
    let layers = doc.layers();
    assert_eq!(
        locked_text(layers.get("parsel").expect("a layer"), "Parsel sınırı"),
        "“Parsel” katmanı kilitli; “Parsel sınırı” şablonu bu katmana çizer. Kilidi Katmanlar panelinden açın."
    );
    assert_eq!(
        locked_text(layers.get("kadastro").expect("a group"), "Parsel sınırı"),
        "“Kadastro” grubu kilitli; “Parsel sınırı” şablonu bu katmana çizer. Kilidi Katmanlar panelinden açın."
    );
}

/// Every object a template's tool writes takes its symbol, attributes and
/// label; the tool's own attributes go over the template's.
#[test]
fn the_tools_write_a_templates_symbol_attributes_and_label() {
    let stamp = Stamp {
        symbol: Some("temel.alan.kenar-ici".into()),
        attrs: [("Tür".to_owned(), "Parsel".to_owned())].into(),
        label: Some("P".into()),
    };
    let area = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(10.0, 0.0);
        b.click(10.0, 10.0);
        b.confirm();
    };
    let line = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(10.0, 0.0);
    };
    let circle = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(3.0, 4.0);
    };
    let rectangle = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(10.0, 6.0);
    };
    let ellipse = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(10.0, 0.0);
        b.click(0.0, 4.0);
    };
    let draws: [(&str, Draw<'_>); 6] = [
        ("polygon", &area),
        ("polyline", &area),
        ("line", &line),
        ("circle", &circle),
        ("rectangle", &rectangle),
        ("ellipse", &ellipse),
    ];
    for (tool, draw) in draws {
        let mut b = Bench::new(tool);
        b.template = Some(stamp.clone());
        let before = b.doc.len();
        draw(&mut b);
        assert!(b.doc.len() > before, "{tool} wrote an object");
        let base = b.newest().base().clone();
        assert_eq!(
            (
                base.symbol.as_deref(),
                base.label.as_deref(),
                base.attrs.get("Tür").map(String::as_str)
            ),
            (Some("temel.alan.kenar-ici"), Some("P"), Some("Parsel")),
            "{tool}"
        );
        // Without a template, none of them.
        let mut b = Bench::new(tool);
        draw(&mut b);
        let base = b.newest().base().clone();
        assert_eq!(
            (base.symbol, base.label, base.attrs.len()),
            (None, None, 0),
            "{tool}"
        );
    }
}
