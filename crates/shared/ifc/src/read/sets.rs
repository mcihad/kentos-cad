//! Property and quantity sets, read when asked (docs/adr/8061 §5 "Özellikler"; plan IFC-07 "istenince özellik ve
//! miktar setleri"): an object's own sets (`IfcRelDefinesByProperties`, a set or a set of sets), then its type's
//! (`HasPropertySets`). Values are as the file writes them, with the measure they are typed with; nothing is
//! converted (the units are the project's, `Summary::units`).

use ifc_lite_core::{AttributeValue, DecodedEntity, EntityDecoder, SchemaVersion};

use super::graph::named;
use super::{Model, ReadError, guarded, header};

/// What a set holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetKind {
    /// `IfcPropertySet`.
    Properties,
    /// `IfcElementQuantity`.
    Quantities,
    /// Another property definition (`IfcDoorLiningProperties` and the like): its name only.
    Other,
}

/// A value as the file writes it.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueKind {
    Text(String),
    /// A real, with the measure type it is written with (`IFCLENGTHMEASURE`), when it has one.
    Number { value: f64, measure: Option<String> },
    Integer { value: i64, measure: Option<String> },
    /// `.T.`, `.F.`, or unknown (`.U.`).
    Logical(Option<bool>),
    Enumeration(Vec<String>),
    List(Vec<ValueKind>),
    Bounded { lower: Box<ValueKind>, upper: Box<ValueKind> },
    /// A record the value refers to (`IfcPropertyReferenceValue`).
    Reference(u32),
    /// Unset (`$`).
    Empty,
}

/// One property or quantity: its record, class, name, value, and its own unit when it has one (`MILLIMETRE`).
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub step: u32,
    pub class: String,
    pub name: String,
    pub value: ValueKind,
    pub unit: Option<String>,
}

/// A set: its record, GlobalId, name, kind, whether it came from the object's type, and its values in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertySet {
    pub step: u32,
    pub global_id: Option<String>,
    pub name: String,
    pub kind: SetKind,
    pub from_type: bool,
    pub values: Vec<Value>,
}

impl Model {
    /// The sets of object `object` (an index into [`Model::objects`]): its own, then its type's. `bytes` are the
    /// bytes the model was read from.
    pub fn property_sets(&self, bytes: &[u8], object: usize) -> Result<Vec<PropertySet>, ReadError> {
        let Some(o) = self.objects.get(object) else { return Ok(Vec::new()) };
        let schema = header::version(self.summary.family);
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            let mut out = Vec::new();
            for &r in self.relations_of(o.step) {
                let rel = &self.relations[r];
                if !schema.is_subtype_of(&rel.class, "IfcRelDefinesByProperties") || !rel.related.contains(&o.step) {
                    continue;
                }
                let Ok(e) = decoder.decode_by_id(rel.step) else { continue };
                for set in refs(named(schema, &e, "RelatingPropertyDefinition")) {
                    out.extend(set_of(&mut decoder, schema, set, false));
                }
            }
            if let Some(t) = o.type_object.map(|t| self.objects[t].step)
                && let Ok(e) = decoder.decode_by_id(t)
            {
                for set in refs(named(schema, &e, "HasPropertySets")) {
                    out.extend(set_of(&mut decoder, schema, set, true));
                }
            }
            Ok(out)
        })
    }
}

/// The records an attribute refers to: one, or a list of them.
fn refs(v: Option<&AttributeValue>) -> Vec<u32> {
    match v {
        Some(v) if v.as_entity_ref().is_some() => v.as_entity_ref().into_iter().collect(),
        Some(v) => v.as_list().unwrap_or(&[]).iter().filter_map(AttributeValue::as_entity_ref).collect(),
        None => Vec::new(),
    }
}

fn set_of(decoder: &mut EntityDecoder, schema: SchemaVersion, step: u32, from_type: bool) -> Option<PropertySet> {
    let e = decoder.decode_by_id(step).ok()?;
    let class = e.ifc_type.name().to_owned();
    let text = |name: &str| named(schema, &e, name).and_then(AttributeValue::as_string).map(str::to_owned);
    let (kind, members) = if schema.is_subtype_of(&class, "IfcPropertySet") {
        (SetKind::Properties, refs(named(schema, &e, "HasProperties")))
    } else if schema.is_subtype_of(&class, "IfcElementQuantity") {
        (SetKind::Quantities, refs(named(schema, &e, "Quantities")))
    } else {
        (SetKind::Other, Vec::new())
    };
    let mut values = Vec::new();
    for m in members {
        if let Ok(p) = decoder.decode_by_id(m) {
            values.push(value_of(decoder, schema, &p));
        }
    }
    Some(PropertySet {
        step,
        global_id: text("GlobalId"),
        name: text("Name").unwrap_or_default(),
        kind,
        from_type,
        values,
    })
}

/// A property or a quantity.
fn value_of(decoder: &mut EntityDecoder, schema: SchemaVersion, p: &DecodedEntity) -> Value {
    let class = p.ifc_type.name().to_owned();
    let at = |name: &str| named(schema, p, name);
    let value = if let Some(v) = at("NominalValue") {
        kind(v)
    } else if let Some(v) = at("EnumerationValues") {
        ValueKind::Enumeration(v.as_list().unwrap_or(&[]).iter().map(|x| text_of(&kind(x))).collect())
    } else if let Some(v) = at("ListValues") {
        ValueKind::List(v.as_list().unwrap_or(&[]).iter().map(kind).collect())
    } else if at("UpperBoundValue").is_some() || at("LowerBoundValue").is_some() {
        ValueKind::Bounded {
            lower: Box::new(at("LowerBoundValue").map_or(ValueKind::Empty, kind)),
            upper: Box::new(at("UpperBoundValue").map_or(ValueKind::Empty, kind)),
        }
    } else if let Some(r) = at("PropertyReference").and_then(AttributeValue::as_entity_ref) {
        ValueKind::Reference(r)
    } else if let Some((v, measure)) = ["LengthValue", "AreaValue", "VolumeValue", "CountValue", "WeightValue", "TimeValue", "NumberValue"]
        .iter()
        .find_map(|n| at(n).map(|v| (v, quantity_measure(n))))
    {
        match kind(v) {
            ValueKind::Number { value, .. } => ValueKind::Number { value, measure: Some(measure.to_owned()) },
            ValueKind::Integer { value, .. } => ValueKind::Number { value: value as f64, measure: Some(measure.to_owned()) },
            other => other,
        }
    } else {
        ValueKind::Empty
    };
    let unit = at("Unit").and_then(AttributeValue::as_entity_ref).and_then(|u| {
        let u = decoder.decode_by_id(u).ok()?;
        let name = named(schema, &u, "Name").and_then(|v| v.as_enum().or_else(|| v.as_string()))?;
        let prefix = named(schema, &u, "Prefix").and_then(AttributeValue::as_enum).unwrap_or("");
        Some(format!("{prefix}{name}"))
    });
    Value {
        step: p.id,
        class,
        name: at("Name").and_then(AttributeValue::as_string).unwrap_or_default().to_owned(),
        value,
        unit,
    }
}

/// The measure a quantity's value attribute stands for.
fn quantity_measure(attribute: &str) -> &'static str {
    match attribute {
        "LengthValue" => "IFCLENGTHMEASURE",
        "AreaValue" => "IFCAREAMEASURE",
        "VolumeValue" => "IFCVOLUMEMEASURE",
        "CountValue" => "IFCCOUNTMEASURE",
        "WeightValue" => "IFCMASSMEASURE",
        "TimeValue" => "IFCTIMEMEASURE",
        _ => "IFCREAL",
    }
}

/// A value's kind, a typed value's measure kept.
fn kind(v: &AttributeValue) -> ValueKind {
    match v {
        AttributeValue::Typed { name, value } => match value.as_ref() {
            AttributeValue::Float(f) => ValueKind::Number { value: *f, measure: Some(name.to_string()) },
            AttributeValue::Integer(i) => ValueKind::Integer { value: *i, measure: Some(name.to_string()) },
            other => kind(other),
        },
        AttributeValue::String(s) | AttributeValue::Binary(s) => ValueKind::Text(s.clone()),
        AttributeValue::Integer(i) => ValueKind::Integer { value: *i, measure: None },
        AttributeValue::Float(f) => ValueKind::Number { value: *f, measure: None },
        AttributeValue::Enum(e) => match e.trim_matches('.') {
            "T" | "TRUE" => ValueKind::Logical(Some(true)),
            "F" | "FALSE" => ValueKind::Logical(Some(false)),
            "U" | "UNKNOWN" => ValueKind::Logical(None),
            other => ValueKind::Text(other.to_owned()),
        },
        AttributeValue::List(l) => ValueKind::List(l.iter().map(kind).collect()),
        AttributeValue::EntityRef(r) => ValueKind::Reference(*r),
        AttributeValue::Null | AttributeValue::Derived => ValueKind::Empty,
    }
}

/// A value as words, for an enumeration's items.
fn text_of(v: &ValueKind) -> String {
    match v {
        ValueKind::Text(s) => s.clone(),
        ValueKind::Number { value, .. } => value.to_string(),
        ValueKind::Integer { value, .. } => value.to_string(),
        ValueKind::Logical(Some(b)) => b.to_string(),
        _ => String::new(),
    }
}
