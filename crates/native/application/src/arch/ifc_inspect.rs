//! `cad.ifc.inspect` (plan/ifc-uyum.md IFC-12, docs/adr/8073 §6): an IFC file read through `kentos-ifc`'s reading face
//! and described: its schema, records, objects by class, units, storeys, findings, and the object asked for with its
//! sets as the file writes them. Nothing is written; the drawing is not read. The IFC viewer reads files the same way
//! (apps/desktop's ifc_viewer.rs); this is the face Python and the tool server use.

use std::collections::BTreeMap;
use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, IfcInspect, IfcInspected, IfcInspectedFinding, IfcInspectedObject, IfcInspectedSet,
    IfcInspectedStorey, IfcInspectedUnits, IfcInspectedValue, IfcValue,
};
use kentos_ifc::read::{self as ifc, Limits, Model, SetKind, Severity, Unit, ValueKind};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;

/// Reads the file and describes it.
fn inspect(input: &IfcInspect) -> Result<(IfcInspected, Vec<CommandWarning>), Stop> {
    let path = Path::new(input.path.trim());
    let wrong = |why: &str| {
        Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("IFC dosyasının yolu {why}: tam yolunu verin, .ifc ile biten ve var olan bir dosya."),
            Some("path".into()),
        ))
    };
    if input.path.trim().is_empty() {
        return Err(wrong("boş"));
    }
    if !path.is_absolute() {
        return Err(wrong("göreli"));
    }
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("ifc")) {
        return Err(wrong(".ifc ile bitmiyor"));
    }
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(wrong("bir dosya göstermiyor")),
        Err(e) => return Err(wrong(&format!("okunamıyor ({e})"))),
    };
    let never = || false;
    let model = ifc::read(&bytes, &Limits::default(), &never)
        .map_err(|e| Stop::failed(checks::error(codes::IFC_UNREADABLE, e.to_string(), Some("path".into()))))?;
    let object = match input.object.as_deref() {
        Some(gid) => {
            let Some(i) = model.objects.iter().position(|o| o.global_id == gid) else {
                return Err(Stop::failed(checks::error(
                    codes::IFC_OBJECT_NOT_FOUND,
                    format!("Dosyada GlobalId'si “{gid}” olan nesne yok; GlobalId'yi dosyanın nesnelerinden alın."),
                    Some("object".into()),
                )));
            };
            Some(object_of(&model, &bytes, i)?)
        }
        None => None,
    };
    let mut classes: BTreeMap<String, u64> = BTreeMap::new();
    for o in &model.objects {
        *classes.entry(o.class.clone()).or_default() += 1;
    }
    let mut storeys: Vec<(u32, IfcInspectedStorey)> = model
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.class.eq_ignore_ascii_case("IfcBuildingStorey"))
        .map(|(i, o)| {
            let contains = model.objects.iter().filter(|c| c.container == Some(i)).count() as u64;
            (o.step, IfcInspectedStorey { global_id: o.global_id.clone(), name: o.name.clone(), contains })
        })
        .collect();
    storeys.sort_by_key(|(step, _)| *step);
    let storeys = storeys.into_iter().map(|(_, s)| s).collect();
    // In the file's order: by line, a finding without one last.
    let mut found: Vec<&ifc::Finding> = model.findings.iter().collect();
    found.sort_by_key(|f| (f.line.is_none(), f.line, f.step, f.code));
    let findings: Vec<IfcInspectedFinding> = found
        .into_iter()
        .map(|f| IfcInspectedFinding {
            code: f.code.to_owned(),
            severity: match f.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            }
            .to_owned(),
            step: f.step,
            line: f.line.map(|l| l as u64),
            message: f.message.clone(),
        })
        .collect();
    let warnings = if findings.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::IFC_READ_FINDINGS.into(),
            message: format!("Dosyada okumayı durdurmayan {} bozukluk var; bulgular listesi kayıt ve satırlarını verir.", findings.len()),
            path: None,
        }]
    };
    let units = &model.summary.units;
    let out = IfcInspected {
        path: input.path.clone(),
        schema: model.summary.schema.clone(),
        records: model.summary.records as u64,
        objects: model.objects.len() as u64,
        classes,
        units: IfcInspectedUnits {
            length: units.length.as_ref().map(written),
            area: units.area.as_ref().map(written),
            volume: units.volume.as_ref().map(written),
            plane_angle: units.plane_angle.as_ref().map(written),
        },
        storeys,
        findings,
        object,
    };
    Ok((out, warnings))
}

/// A unit as written: its prefix, then its name.
fn written(u: &Unit) -> String {
    format!("{}{}", u.prefix.as_deref().unwrap_or(""), u.name)
}

fn object_of(model: &Model, bytes: &[u8], i: usize) -> Result<IfcInspectedObject, Stop> {
    let o = &model.objects[i];
    let sets = model
        .property_sets(bytes, i)
        .map_err(|e| Stop::failed(checks::error(codes::IFC_UNREADABLE, e.to_string(), Some("object".into()))))?;
    let mut storey = None;
    let mut at = o.parent.or(o.container);
    for _ in 0..model.objects.len() {
        let Some(p) = at.and_then(|p| model.objects.get(p)) else { break };
        if p.class.eq_ignore_ascii_case("IfcBuildingStorey") {
            storey = p.name.clone();
            break;
        }
        at = p.parent.or(p.container);
    }
    Ok(IfcInspectedObject {
        global_id: o.global_id.clone(),
        step: o.step,
        class: o.class.clone(),
        name: o.name.clone(),
        description: o.description.clone(),
        object_type: o.object_type.clone(),
        long_name: o.long_name.clone(),
        tag: o.tag.clone(),
        predefined_type: o.predefined.clone(),
        type_name: o.type_object.and_then(|t| model.objects[t].name.clone()),
        storey,
        sets: sets
            .into_iter()
            .map(|s| IfcInspectedSet {
                name: s.name,
                kind: match s.kind {
                    SetKind::Properties => "properties",
                    SetKind::Quantities => "quantities",
                    SetKind::Other => "other",
                }
                .to_owned(),
                from_type: s.from_type,
                values: s
                    .values
                    .into_iter()
                    .map(|v| IfcInspectedValue { name: v.name, class: v.class, value: value_of(&v.value), unit: v.unit })
                    .collect(),
            })
            .collect(),
    })
}

fn value_of(v: &ValueKind) -> IfcValue {
    match v {
        ValueKind::Text(s) => IfcValue::Text { text: s.clone() },
        ValueKind::Number { value, measure } => IfcValue::Number { number: *value, measure: measure.clone() },
        ValueKind::Integer { value, measure } => IfcValue::Integer { integer: *value, measure: measure.clone() },
        ValueKind::Logical(b) => IfcValue::Logical { logical: *b },
        ValueKind::Enumeration(items) => IfcValue::Enumeration { items: items.clone() },
        ValueKind::List(items) => IfcValue::List { items: items.iter().map(words).collect() },
        ValueKind::Bounded { lower, upper } => {
            let number = |v: &ValueKind| match v {
                ValueKind::Number { value, .. } => Some(*value),
                ValueKind::Integer { value, .. } => Some(*value as f64),
                _ => None,
            };
            let measure = |v: &ValueKind| match v {
                ValueKind::Number { measure, .. } | ValueKind::Integer { measure, .. } => measure.clone(),
                _ => None,
            };
            IfcValue::Bounded { lower: number(lower), upper: number(upper), measure: measure(lower).or_else(|| measure(upper)) }
        }
        ValueKind::Reference(step) => IfcValue::Reference { step: *step },
        ValueKind::Empty => IfcValue::Empty,
    }
}

/// A list item in words: a number as the shortest decimal that reads back to it.
fn words(v: &ValueKind) -> String {
    match v {
        ValueKind::Text(s) => s.clone(),
        ValueKind::Number { value, .. } => value.to_string(),
        ValueKind::Integer { value, .. } => value.to_string(),
        ValueKind::Logical(Some(b)) => b.to_string(),
        ValueKind::Logical(None) => "unknown".to_owned(),
        ValueKind::Enumeration(items) => items.join(", "),
        ValueKind::List(items) => items.iter().map(words).collect::<Vec<_>>().join(", "),
        ValueKind::Bounded { lower, upper } => format!("{}–{}", words(lower), words(upper)),
        ValueKind::Reference(step) => format!("#{step}"),
        ValueKind::Empty => String::new(),
    }
}

/// Checks `input`: the file reads and the object is there; writes nothing.
pub fn validate(_cx: &ExecutionContext<'_>, input: &IfcInspect) -> CommandResult<()> {
    match inspect(input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The file described; writes nothing.
pub fn plan(_cx: &ExecutionContext<'_>, input: &IfcInspect) -> CommandResult<IfcInspected> {
    match inspect(input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The file described; writes nothing, the drawing does not change.
pub fn execute(_cx: &mut ExecutionContext<'_>, input: IfcInspect) -> CommandResult<IfcInspected> {
    match inspect(&input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}
