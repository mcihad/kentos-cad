//! The DXF writer's input from JSON: the `DxfWriteInput` contract, its objects and block definitions read by the
//! writers' own visitor (`crate::wire`) instead of the contract's derived code. The tests check it against the
//! contract's own deserializer.

use std::collections::BTreeMap;

use kentos_contracts::{DxfWriteInput, DxfWriteLayer};
use serde::de::{Deserialize, Deserializer};

use crate::wire::{Definitions, Objects};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    #[serde(default)]
    allow_polyline_loss: bool,
    #[serde(default)]
    notes: Vec<kentos_contracts::ReportItem>,
    #[serde(default)]
    uids: BTreeMap<u32, kentos_contracts::EntityId>,
    #[serde(default)]
    wall_detail: Option<kentos_contracts::WallDetail>,
    #[serde(default)]
    cut_height: Option<f64>,
    #[serde(default)]
    layouts: Vec<kentos_contracts::CadLayout>,
    entities: Objects,
    layers: Vec<DxfWriteLayer>,
    scale: f64,
    length_decimals: u32,
    grads: bool,
    #[serde(default)]
    dimension_values: BTreeMap<u32, String>,
    #[serde(default)]
    blocks: Definitions,
    #[serde(default)]
    unit: Option<kentos_contracts::DrawingUnit>,
    #[serde(default)]
    text_styles: Vec<kentos_contracts::TextStyleDef>,
    #[serde(default)]
    dimension_styles: Vec<kentos_contracts::DimensionStyleDef>,
    #[serde(default)]
    table_styles: Vec<kentos_contracts::TableStyleDef>,
    #[serde(default)]
    line_types: Vec<kentos_contracts::LineTypeDef>,
    #[serde(default)]
    line_type_scale: Option<f64>,
    #[serde(default)]
    annotations: kentos_contracts::DxfAnnotations,
}

/// A `DxfWriteInput` read with this module's visitor for the objects. The
/// WASM boundary parses it itself (`serde_json::from_str::<WriteInput>`):
/// serde_json's parser is then built once in that crate, next to the other
/// options it reads, instead of a second copy here (~25 KB in the module).
pub struct WriteInput(pub DxfWriteInput);

impl<'de> Deserialize<'de> for WriteInput {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<WriteInput, D::Error> {
        let i = Input::deserialize(d)?;
        Ok(WriteInput(DxfWriteInput {
            allow_polyline_loss: i.allow_polyline_loss,
            layouts: i.layouts,
            uids: i.uids,
            wall_detail: i.wall_detail,
            cut_height: i.cut_height,
            annotations: i.annotations,
            notes: i.notes,
            entities: i.entities.0,
            layers: i.layers,
            scale: i.scale,
            length_decimals: i.length_decimals,
            grads: i.grads,
            dimension_values: i.dimension_values,
            blocks: i.blocks.0,
            unit: i.unit,
            text_styles: i.text_styles,
            dimension_styles: i.dimension_styles,
            table_styles: i.table_styles,
            line_types: i.line_types,
            line_type_scale: i.line_type_scale,
        }))
    }
}

/// Reads a `DxfWriteInput` from its JSON (as the contract writes it).
pub fn input_from_json(text: &str) -> Result<DxfWriteInput, String> {
    serde_json::from_str::<WriteInput>(text)
        .map(|w| w.0)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::Entity;

    /// Objects of every kind with every optional field, some null, some left out, and a field nobody knows.
    const OBJECTS: &str = r##"[
      {"kind":"point","id":1,"layerId":"kot","attrs":{"Ad":"P1","Z (m)":"105.20"},"label":"P1","p":{"x":452345.123,"y":4412345.678},"z":105.2},
      {"kind":"point","id":2,"layerId":"kot","attrs":{},"p":{"x":0.30000000000000004,"y":-2.5e-7},"z":null,"color":null,"future":{"a":[1,2]}},
      {"id":3,"kind":"line","layerId":"a","color":"#7fb2e5","attrs":{},"a":{"x":1,"y":2},"b":{"x":3,"y":4},"symbol":"s.1"},
      {"kind":"polyline","id":4,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":0}],"bulges":[0.5]},
      {"kind":"polygon","id":5,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":9,"y":0},{"x":9,"y":9}],"bulges":null,
       "holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]},{"pts":[{"x":5,"y":5},{"x":6,"y":5}],"bulges":[1,1]}]},
      {"kind":"circle","id":6,"layerId":"a","attrs":{},"c":{"x":5,"y":5},"r":2.5},
      {"kind":"arc","id":7,"layerId":"a","attrs":{},"c":{"x":5,"y":5},"r":2.5,"a0":0.6435011087932844,"a1":2.5},
      {"kind":"ellipse","id":8,"layerId":"a","attrs":{},"c":{"x":5,"y":5},"major":{"x":3,"y":1},"ratio":0.5,"t0":0,"t1":3.14},
      {"kind":"spline","id":9,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":1},{"x":2,"y":0}],"closed":true},
      {"kind":"xline","id":10,"layerId":"a","attrs":{},"p":{"x":0,"y":0},"dir":{"x":0.6,"y":0.8}},
      {"kind":"ray","id":11,"layerId":"a","attrs":{},"p":{"x":0,"y":0},"dir":{"x":0,"y":-1}},
      {"kind":"text","id":12,"layerId":"a","attrs":{},"p":{"x":1,"y":1},"text":"Ağaç ^ %%d","height":2.5,"rotation":33.3,
       "align":"topRight","widthFactor":0.75,"mask":true},
      {"kind":"dimension","id":13,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":2,"height":0.5,"text":"10.00","style":"linear","angle":0},
      {"kind":"dimension","id":14,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":6,"height":1,"style":"angular","c":{"x":-1,"y":-1}},
      {"kind":"hatch","id":15,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],
       "holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"cross","angle":45,"spacing":0.75}},
      {"kind":"line","id":16,"layerId":"a","attrs":{},"lineWeight":0.35,"a":{"x":0,"y":0},"b":{"x":3,"y":4},"za":10.5,"zb":null},
      {"kind":"polyline","id":17,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":0},{"x":2,"y":0}],"zs":[1.5,null,-2]},
      {"kind":"polygon","id":18,"layerId":"a","attrs":{},"lineWeight":0,"pts":[{"x":0,"y":0},{"x":9,"y":0},{"x":9,"y":9}],"zs":[0,1,2],
       "holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}],"zs":[null,5,6]}]},
      {"kind":"polygon","id":19,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":9,"y":0},{"x":9,"y":9}],
       "parts":[{"pts":[{"x":20,"y":0},{"x":29,"y":0},{"x":29,"y":9}],"bulges":[0,0.5,0],"zs":[1,null,2],
        "holes":[{"pts":[{"x":21,"y":1},{"x":22,"y":1},{"x":22,"y":2}]}]}]},
      {"kind":"insert","id":20,"layerId":"a","attrs":{"No":"7"},"color":"#e5484d","block":"018f3a2b-0000-7000-8000-000000000001",
       "p":{"x":452345.5,"y":4412345.25},"scale":2.5,"rotation":0.1,"mirror":true},
      {"kind":"insert","id":21,"layerId":"a","attrs":{},"block":"018f3a2b-0000-7000-8000-000000000001","p":{"x":1,"y":1},"scale":1,"rotation":0},
      {"kind":"leader","id":22,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":6,"y":5}],"text":"Mevcut bina","height":2.5,"rotation":30,
       "arrow":"dot","mask":true},
      {"kind":"leader","id":23,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":-4,"y":2},{"x":-8,"y":2}],"height":1,"rotation":0},
      {"kind":"dimension","id":24,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":2,"height":0.5,"style":"slope",
       "za":105.25,"zb":104.5,"mask":true},
      {"kind":"dimension","id":25,"layerId":"a","attrs":{},"a":{"x":5,"y":0},"b":{"x":0,"y":5},"offset":1,"height":0.5,"style":"arcLength","c":{"x":0,"y":0}},
      {"kind":"nurbs","id":26,"layerId":"a","attrs":{},"degree":2,"ctrl":[{"x":0,"y":0},{"x":1,"y":1},{"x":2,"y":0}],
       "knots":[0,0,0,1,1,1],"weights":[1,0.7071067811865476,1],"fit":[{"x":0,"y":0},{"x":2,"y":0}],
       "tangents":[{"x":1,"y":0},null],"fitTolerance":1e-10},
      {"kind":"nurbs","id":27,"layerId":"a","attrs":{},"degree":1,"ctrl":[{"x":0,"y":0},{"x":1,"y":1}],"knots":[0,0,1,1],"closed":true,"periodic":true}
    ]"##;

    /// A definition holding a line and a nested insert, and the one it nests.
    const BLOCKS: &str = r##"[
      {"id":"018f3a2b-0000-7000-8000-000000000001","name":"Direk","base":{"x":1,"y":2},"description":"Aydınlatma direği",
       "entities":[{"kind":"line","id":1,"layerId":"0","attrs":{},"a":{"x":0,"y":0},"b":{"x":0,"y":6}},
                   {"kind":"insert","id":2,"layerId":"0","attrs":{},"block":"018f3a2b-0000-7000-8000-000000000002","p":{"x":0,"y":6},"scale":1,"rotation":1.5707963267948966}]},
      {"id":"018f3a2b-0000-7000-8000-000000000002","name":"Lamba","base":{"x":0,"y":0},
       "entities":[{"kind":"circle","id":1,"layerId":"cizim","attrs":{},"color":"#f5d90a","c":{"x":0,"y":0},"r":0.4}],
       "attributes":[{"tag":"GUC","p":{"x":0.5,"y":0},"height":0.25,"rotation":0,"prompt":"Güç","value":"150 W",
                      "align":"middleLeft","widthFactor":1.25}]}
    ]"##;

    fn doc(objects: &str) -> String {
        format!(
            r##"{{"entities":{objects},"layers":[{{"id":"a","name":"A","path":["G"],"color":"ink","visible":true,"locked":false,"lineType":"dashdot","lineWeight":0.25}}],"scale":1000,"lengthDecimals":3,"grads":true,"dimensionValues":{{"14":"45.0000g"}},"blocks":{BLOCKS}}}"##
        )
    }

    #[test]
    fn reads_what_the_contracts_own_deserializer_reads() {
        let text = doc(OBJECTS);
        let ours = input_from_json(&text).expect("input");
        let derived: DxfWriteInput = serde_json::from_str(&text).expect("contract");
        assert_eq!(ours, derived);
        assert_eq!(ours.entities.len(), 27);
        // A leader with every field, and one with none of the optional ones (docs/adr/0146 §8).
        let Entity::Leader(l) = &ours.entities[21] else {
            panic!("{:?}", ours.entities[21])
        };
        assert_eq!(
            (l.text.as_deref(), l.arrow, l.mask),
            (
                Some("Mevcut bina"),
                Some(kentos_contracts::LeaderArrow::Dot),
                true
            )
        );
        // Inserts and the definitions they place, nested ones in them (docs/adr/0144 §5).
        let Entity::Insert(i) = &ours.entities[19] else {
            panic!("{:?}", ours.entities[19])
        };
        assert_eq!((i.scale, i.rotation, i.mirror), (2.5, 0.1, true));
        let names: Vec<(&str, usize, usize)> = ours
            .blocks
            .iter()
            .map(|b| (b.name.as_str(), b.entities.len(), b.attributes.len()))
            .collect();
        assert_eq!(names, [("Direk", 2, 0), ("Lamba", 1, 1)]);
        // What the contract's code reads, this reads too: an object's own weight and its vertices' elevations.
        let Entity::Line(l) = &ours.entities[15] else {
            panic!("{:?}", ours.entities[15])
        };
        assert_eq!(
            (l.base.line_weight, l.za, l.zb),
            (Some(0.35), Some(10.5), None)
        );
        let Entity::Polygon(g) = &ours.entities[17] else {
            panic!("{:?}", ours.entities[17])
        };
        assert_eq!(g.zs, Some(vec![Some(0.0), Some(1.0), Some(2.0)]));
        assert_eq!(
            g.holes.as_ref().map(|h| h[0].zs.clone()),
            Some(Some(vec![None, Some(5.0), Some(6.0)]))
        );
        // A multi-part area's parts (docs/adr/0143).
        let Entity::Polygon(g) = &ours.entities[18] else {
            panic!("{:?}", ours.entities[18])
        };
        let parts = g.parts.as_deref().expect("parts");
        assert_eq!(
            (parts.len(), parts[0].holes.as_ref().map(Vec::len)),
            (1, Some(1))
        );
        // And back from the contract's own JSON.
        let again =
            input_from_json(&serde_json::to_string(&derived).expect("json")).expect("input");
        assert_eq!(again, derived);
    }

    #[test]
    fn refuses_what_the_contract_refuses() {
        for (objects, why) in [
            (
                r#"[{"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":1,"y":2}}]"#,
                "missing field `b`",
            ),
            (
                r#"[{"kind":"blob","id":1,"layerId":"a","attrs":{}}]"#,
                "unknown variant `blob`",
            ),
            (
                r#"[{"kind":"point","layerId":"a","attrs":{},"p":{"x":1,"y":2}}]"#,
                "missing field `id`",
            ),
            (
                r#"[{"kind":"polygon","id":1,"layerId":"a","attrs":{},"pts":[],"holes":[[{"x":1,"y":1}]]}]"#,
                "a polygon's hole is a ring",
            ),
            (
                r#"[{"kind":"hatch","id":1,"layerId":"a","attrs":{},"ring":[],"holes":[{"pts":[]}],"pattern":{"type":"solid","angle":0,"spacing":1}}]"#,
                "a hatch's island is a list of points",
            ),
            (
                r#"[{"kind":"point","id":1,"layerId":"a","attrs":{},"p":{"x":"1","y":2}}]"#,
                "invalid type",
            ),
        ] {
            let e = input_from_json(&doc(objects)).expect_err(objects);
            assert!(e.contains(why), "{objects}: {e}");
            assert!(
                serde_json::from_str::<DxfWriteInput>(&doc(objects)).is_err(),
                "{objects}"
            );
        }
    }
}
