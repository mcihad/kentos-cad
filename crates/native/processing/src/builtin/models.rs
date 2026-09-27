//! Models that ship with KentOS (the web's `builtin/models.ts`): ready to
//! run and a starting point to copy in the model designer. They cannot be
//! changed in place (a copy can).

use std::collections::BTreeMap;

use serde_json::json;

use crate::model::{Model, ModelOutput, ModelStep, ValueSource};
use crate::types::{ParamDef, ParamKind};

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
        inputs: vec![
            ParamDef::new(
                "parcels",
                "Parseller",
                ParamKind::Features {
                    kinds: Some(vec!["polygon".into()]),
                    scopes: None,
                    writes: false,
                },
            )
            .default_value(json!({ "scope": "selection" }))
            .describe("Ölçü yazıları hazırlanacak parseller."),
            ParamDef::new(
                "prefix",
                "Nokta öneki",
                ParamKind::Text {
                    placeholder: None,
                    max_length: Some(12),
                    allow_empty: true,
                },
            )
            .default_value(json!("P"))
            .describe("Köşe numaralarının başındaki yazı: P00001."),
        ],
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
