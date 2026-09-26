//! The writer's input from JSON: the `DxfWriteInput` contract, read by one
//! hand-written serde visitor per object instead of the contract's derived
//! code. The derive reads a tagged enum of flattened structs by buffering
//! every object and weighs about 100 KB in the formats module, which the
//! browser downloads for an export (CLAUDE.md §20); this reads the same
//! JSON field by field, straight into the objects (no tree in between, so
//! a large export is not held twice). The tests check it against the
//! contract's own deserializer.

use std::collections::BTreeMap;
use std::fmt;

use kentos_contracts::{
    ArcEntity, CircleEntity, ConstructionEntity, DimensionEntity, DimensionStyle, DxfWriteInput,
    DxfWriteLayer, EllipseEntity, Entity, EntityBase, HatchEntity, HatchPattern, LineEntity,
    PathEntity, PointEntity, RingGeometry, SplineEntity, TextEntity, Vec2,
};
use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};
use serde::de::{self, Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

/// One object of the list.
struct Wire(Entity);

/// A polygon's hole (a ring with bulges) or a hatch's island (points): `holes` holds either.
enum Hole {
    Ring(RingGeometry),
    Points(Vec<Vec2>),
}

impl<'de> Deserialize<'de> for Hole {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Hole, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Hole;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a ring or a list of points")
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Hole, A::Error> {
                RingGeometry::deserialize(MapAccessDeserializer::new(map)).map(Hole::Ring)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Hole, A::Error> {
                Vec::<Vec2>::deserialize(SeqAccessDeserializer::new(seq)).map(Hole::Points)
            }
        }
        d.deserialize_any(V)
    }
}

/// Every field an object of any kind may have.
#[derive(Default)]
struct Fields {
    kind: Option<String>,
    id: Option<u32>,
    layer_id: Option<String>,
    color: Option<String>,
    attrs: Option<BTreeMap<String, String>>,
    label: Option<String>,
    symbol: Option<String>,
    p: Option<Vec2>,
    z: Option<f64>,
    a: Option<Vec2>,
    b: Option<Vec2>,
    c: Option<Vec2>,
    pts: Option<Vec<Vec2>>,
    bulges: Option<Vec<f64>>,
    holes: Option<Vec<Hole>>,
    r: Option<f64>,
    a0: Option<f64>,
    a1: Option<f64>,
    major: Option<Vec2>,
    ratio: Option<f64>,
    t0: Option<f64>,
    t1: Option<f64>,
    dir: Option<Vec2>,
    closed: Option<bool>,
    text: Option<String>,
    height: Option<f64>,
    rotation: Option<f64>,
    offset: Option<f64>,
    style: Option<DimensionStyle>,
    angle: Option<f64>,
    ring: Option<Vec<Vec2>>,
    pattern: Option<HatchPattern>,
}

fn need<T, E: de::Error>(v: Option<T>, name: &'static str) -> Result<T, E> {
    v.ok_or_else(|| E::missing_field(name))
}

impl Fields {
    fn entity<E: de::Error>(self) -> Result<Entity, E> {
        let kind = need(self.kind, "kind")?;
        let base = EntityBase {
            id: need(self.id, "id")?,
            layer_id: need(self.layer_id, "layerId")?,
            color: self.color,
            attrs: need(self.attrs, "attrs")?,
            label: self.label,
            symbol: self.symbol,
        };
        let rings = |holes: Option<Vec<Hole>>| -> Result<Option<Vec<RingGeometry>>, E> {
            holes
                .map(|h| {
                    h.into_iter()
                        .map(|x| match x {
                            Hole::Ring(r) => Ok(r),
                            Hole::Points(_) => {
                                Err(E::custom("a polygon's hole is a ring (pts, bulges)"))
                            }
                        })
                        .collect()
                })
                .transpose()
        };
        let islands = |holes: Option<Vec<Hole>>| -> Result<Option<Vec<Vec<Vec2>>>, E> {
            holes
                .map(|h| {
                    h.into_iter()
                        .map(|x| match x {
                            Hole::Points(p) => Ok(p),
                            Hole::Ring(_) => Err(E::custom("a hatch's island is a list of points")),
                        })
                        .collect()
                })
                .transpose()
        };
        let path =
            |base: EntityBase, pts: Option<Vec<Vec2>>, bulges, holes| -> Result<PathEntity, E> {
                Ok(PathEntity {
                    base,
                    pts: need(pts, "pts")?,
                    bulges,
                    holes: rings(holes)?,
                })
            };
        let construction = |base: EntityBase,
                            p: Option<Vec2>,
                            dir: Option<Vec2>|
         -> Result<ConstructionEntity, E> {
            Ok(ConstructionEntity {
                base,
                p: need(p, "p")?,
                dir: need(dir, "dir")?,
            })
        };
        Ok(match kind.as_str() {
            "point" => Entity::Point(PointEntity {
                base,
                p: need(self.p, "p")?,
                z: self.z,
            }),
            "line" => Entity::Line(LineEntity {
                base,
                a: need(self.a, "a")?,
                b: need(self.b, "b")?,
            }),
            "polyline" => Entity::Polyline(path(base, self.pts, self.bulges, self.holes)?),
            "polygon" => Entity::Polygon(path(base, self.pts, self.bulges, self.holes)?),
            "circle" => Entity::Circle(CircleEntity {
                base,
                c: need(self.c, "c")?,
                r: need(self.r, "r")?,
            }),
            "arc" => Entity::Arc(ArcEntity {
                base,
                c: need(self.c, "c")?,
                r: need(self.r, "r")?,
                a0: need(self.a0, "a0")?,
                a1: need(self.a1, "a1")?,
            }),
            "ellipse" => Entity::Ellipse(EllipseEntity {
                base,
                c: need(self.c, "c")?,
                major: need(self.major, "major")?,
                ratio: need(self.ratio, "ratio")?,
                t0: need(self.t0, "t0")?,
                t1: need(self.t1, "t1")?,
            }),
            "spline" => Entity::Spline(SplineEntity {
                base,
                pts: need(self.pts, "pts")?,
                closed: need(self.closed, "closed")?,
            }),
            "xline" => Entity::Xline(construction(base, self.p, self.dir)?),
            "ray" => Entity::Ray(construction(base, self.p, self.dir)?),
            "text" => Entity::Text(TextEntity {
                base,
                p: need(self.p, "p")?,
                text: need(self.text, "text")?,
                height: need(self.height, "height")?,
                rotation: need(self.rotation, "rotation")?,
            }),
            "dimension" => Entity::Dimension(DimensionEntity {
                base,
                a: need(self.a, "a")?,
                b: need(self.b, "b")?,
                offset: need(self.offset, "offset")?,
                height: need(self.height, "height")?,
                text: self.text,
                style: self.style,
                angle: self.angle,
                c: self.c,
            }),
            "hatch" => Entity::Hatch(HatchEntity {
                base,
                ring: need(self.ring, "ring")?,
                holes: islands(self.holes)?,
                pattern: need(self.pattern, "pattern")?,
            }),
            other => {
                return Err(E::unknown_variant(
                    other,
                    &[
                        "point",
                        "line",
                        "polyline",
                        "polygon",
                        "circle",
                        "arc",
                        "ellipse",
                        "spline",
                        "xline",
                        "ray",
                        "text",
                        "dimension",
                        "hatch",
                    ],
                ));
            }
        })
    }
}

impl<'de> Deserialize<'de> for Wire {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Wire, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Wire;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a drawing object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Wire, A::Error> {
                let mut f = Fields::default();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "kind" => f.kind = Some(map.next_value()?),
                        "id" => f.id = Some(map.next_value()?),
                        "layerId" => f.layer_id = Some(map.next_value()?),
                        "color" => f.color = map.next_value()?,
                        "attrs" => f.attrs = Some(map.next_value()?),
                        "label" => f.label = map.next_value()?,
                        "symbol" => f.symbol = map.next_value()?,
                        "p" => f.p = Some(map.next_value()?),
                        "z" => f.z = map.next_value()?,
                        "a" => f.a = Some(map.next_value()?),
                        "b" => f.b = Some(map.next_value()?),
                        "c" => f.c = map.next_value()?,
                        "pts" => f.pts = Some(map.next_value()?),
                        "bulges" => f.bulges = map.next_value()?,
                        "holes" => f.holes = map.next_value()?,
                        "r" => f.r = Some(map.next_value()?),
                        "a0" => f.a0 = Some(map.next_value()?),
                        "a1" => f.a1 = Some(map.next_value()?),
                        "major" => f.major = Some(map.next_value()?),
                        "ratio" => f.ratio = Some(map.next_value()?),
                        "t0" => f.t0 = Some(map.next_value()?),
                        "t1" => f.t1 = Some(map.next_value()?),
                        "dir" => f.dir = Some(map.next_value()?),
                        "closed" => f.closed = Some(map.next_value()?),
                        "text" => f.text = map.next_value()?,
                        "height" => f.height = Some(map.next_value()?),
                        "rotation" => f.rotation = Some(map.next_value()?),
                        "offset" => f.offset = Some(map.next_value()?),
                        "style" => f.style = map.next_value()?,
                        "angle" => f.angle = map.next_value()?,
                        "ring" => f.ring = Some(map.next_value()?),
                        "pattern" => f.pattern = Some(map.next_value()?),
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                f.entity().map(Wire)
            }
        }
        d.deserialize_map(V)
    }
}

/// The objects of the list, each read by `Wire` (the GeoJSON writer reads its objects with it too).
pub(crate) struct Objects(pub(crate) Vec<Entity>);

impl<'de> Deserialize<'de> for Objects {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Objects, D::Error> {
        let wires = Vec::<Wire>::deserialize(d)?;
        Ok(Objects(wires.into_iter().map(|w| w.0).collect()))
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    entities: Objects,
    layers: Vec<DxfWriteLayer>,
    scale: f64,
    length_decimals: u32,
    grads: bool,
    #[serde(default)]
    dimension_values: BTreeMap<u32, String>,
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
            entities: i.entities.0,
            layers: i.layers,
            scale: i.scale,
            length_decimals: i.length_decimals,
            grads: i.grads,
            dimension_values: i.dimension_values,
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
      {"kind":"text","id":12,"layerId":"a","attrs":{},"p":{"x":1,"y":1},"text":"Ağaç ^ %%d","height":2.5,"rotation":33.3},
      {"kind":"dimension","id":13,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":2,"height":0.5,"text":"10.00","style":"linear","angle":0},
      {"kind":"dimension","id":14,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":6,"height":1,"style":"angular","c":{"x":-1,"y":-1}},
      {"kind":"hatch","id":15,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],
       "holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"cross","angle":45,"spacing":0.75}}
    ]"##;

    fn doc(objects: &str) -> String {
        format!(
            r##"{{"entities":{objects},"layers":[{{"id":"a","name":"A","path":["G"],"color":"ink","visible":true,"locked":false,"lineType":"dashdot","lineWeight":0.25}}],"scale":1000,"lengthDecimals":3,"grads":true,"dimensionValues":{{"14":"45.0000g"}}}}"##
        )
    }

    #[test]
    fn reads_what_the_contracts_own_deserializer_reads() {
        let text = doc(OBJECTS);
        let ours = input_from_json(&text).expect("input");
        let derived: DxfWriteInput = serde_json::from_str(&text).expect("contract");
        assert_eq!(ours, derived);
        assert_eq!(ours.entities.len(), 15);
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
