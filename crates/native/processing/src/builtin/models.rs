//! Models that ship with KentOS (the web's `builtin/models.ts`): ready to
//! run and a starting point to copy in the model designer. They cannot be
//! changed in place (a copy can).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::model::{Model, ModelInput, ModelOutput, ModelStep, ValueSource};

fn input(name: &str) -> ValueSource {
    ValueSource::Input(name.into())
}

/// Parsel ölçü yazıları: corner numbers, edge lengths and the computed area, one undo step.
pub fn parcel_sheet() -> Model {
    Model {
        id: "builtin.parcelSheet".into(),
        label: "Parsel ölçü yazıları".into(),
        category: "cadastre".into(),
        description: "Parsellerin köşelerini numaralar, kenar uzunluklarını yazar ve hesaplanan alanı özniteliğe yazar; hepsi tek adımda geri alınır.".into(),
        // The web's definitions, as its model keeps them.
        inputs: [
            json!({
                "type": "features",
                "name": "parcels",
                "label": "Parseller",
                "description": "Ölçü yazıları hazırlanacak parseller.",
                "kinds": ["polygon"],
                "default": { "scope": "selection" },
            }),
            json!({
                "type": "string",
                "name": "prefix",
                "label": "Nokta öneki",
                "description": "Köşe numaralarının başındaki yazı: P00001.",
                "default": "P",
                "allowEmpty": true,
                "maxLength": 12,
            }),
        ]
        .into_iter()
        .filter_map(|v| match v {
            Value::Object(map) => Some(ModelInput(map)),
            _ => None,
        })
        .collect(),
        steps: vec![
            ModelStep {
                id: "corners".into(),
                tool: "points.numberVertices".into(),
                values: vec![
                    ("input".into(), input("parcels")),
                    ("prefix".into(), input("prefix")),
                ],
                position: Some((320.0, 60.0)),
                caption: None,
            },
            ModelStep {
                id: "edges".into(),
                tool: "annotation.edgeLengths".into(),
                values: vec![("input".into(), input("parcels"))],
                position: Some((320.0, 180.0)),
                caption: None,
            },
            ModelStep {
                id: "area".into(),
                tool: "attributes.calculate".into(),
                values: vec![
                    ("input".into(), input("parcels")),
                    ("field".into(), ValueSource::Value(json!("Hesap alanı"))),
                    ("value".into(), ValueSource::Value(json!("metin($alan, 2)"))),
                    ("label".into(), ValueSource::Value(json!(false))),
                ],
                position: Some((320.0, 300.0)),
                caption: None,
            },
        ],
        outputs: vec![ModelOutput {
            name: "points".into(),
            label: "Köşe noktaları".into(),
            step: "corners".into(),
            output: "points".into(),
        }],
        input_positions: BTreeMap::new(),
    }
}
