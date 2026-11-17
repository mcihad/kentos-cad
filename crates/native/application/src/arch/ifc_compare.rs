//! `cad.ifc.compare` v1 (plan/ifc-uyum.md IFC-10, docs/adr/8103): two revisions of an IFC model compared through
//! `kentos-ifc`'s comparing (`kentos_ifc::compare`), by GlobalId alone. Nothing is written; the drawing is not read.

use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, IfcChange, IfcCompare, IfcCompared, IfcComparedChange, IfcComparedFile,
    IfcComparedObject, IfcRepeatedId, MAX_IFC_COMPARE_TOLERANCE, MIN_IFC_COMPARE_TOLERANCE, IFC_COMPARE_TOLERANCE,
};
use kentos_ifc::compare::{self, Change};
use kentos_ifc::read::{self as ifc, Limits, Model};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;

/// The IFC file named at `field` read: its bytes and its model; refused with the field's path.
pub(crate) fn ifc_file(text: &str, field: &str) -> Result<(Vec<u8>, Model), Stop> {
    let path = Path::new(text.trim());
    let wrong = |why: &str| {
        Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("IFC dosyasının yolu {why}: tam yolunu verin, .ifc ile biten ve var olan bir dosya."),
            Some(field.into()),
        ))
    };
    if text.trim().is_empty() {
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
    let model = ifc::read(&bytes, &Limits::default(), &|| false).map_err(|e| {
        Stop::failed(checks::error(codes::IFC_UNREADABLE, format!("{}: {e}", path.display()), Some(field.into())))
    })?;
    Ok((bytes, model))
}

/// An object as the lists name it.
pub(crate) fn named(model: &Model, i: usize) -> IfcComparedObject {
    let o = &model.objects[i];
    IfcComparedObject { global_id: o.global_id.clone(), class: o.class.clone(), name: o.name.clone() }
}

fn changes(older: &Model, newer: &Model, list: &[Change]) -> Vec<IfcChange> {
    list.iter()
        .map(|c| match c {
            Change::Class { old, new } => IfcChange::Class { old: old.clone(), new: new.clone() },
            Change::Attribute { name, old, new } => {
                IfcChange::Attribute { name: (*name).to_owned(), old: old.clone(), new: new.clone() }
            }
            Change::Container { old, new } => IfcChange::Container {
                old: old.map(|i| named(older, i)),
                new: new.map(|i| named(newer, i)),
            },
            Change::Type { old, new } => {
                IfcChange::Type { old: old.map(|i| named(older, i)), new: new.map(|i| named(newer, i)) }
            }
            Change::Body { old, new } => IfcChange::Body { old: *old, new: *new },
            Change::Moved { by } => IfcChange::Moved {
                by: by.to_vec(),
                distance: (by[0] * by[0] + by[1] * by[1] + by[2] * by[2]).sqrt(),
            },
            Change::Resized { old, new } => IfcChange::Resized { old: old.to_vec(), new: new.to_vec() },
            Change::Property { set, from_type, name, old, new } => IfcChange::Property {
                set: set.clone(),
                from_type: *from_type,
                name: name.clone(),
                old: old.as_ref().map(crate::ifc_inspect::value_of),
                new: new.as_ref().map(crate::ifc_inspect::value_of),
            },
        })
        .collect()
}

fn compared(input: &IfcCompare) -> Result<(IfcCompared, Vec<CommandWarning>), Stop> {
    // Both paths first, then the tolerance, then the reading.
    for (field, text) in [("older", &input.older), ("newer", &input.newer)] {
        let p = Path::new(text.trim());
        if text.trim().is_empty() || !p.is_absolute() || !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ifc")) || !p.is_file() {
            // The same words as the reading's refusal.
            ifc_file(text, field)?;
        }
    }
    let tolerance = input.tolerance.unwrap_or(IFC_COMPARE_TOLERANCE);
    if !(MIN_IFC_COMPARE_TOLERANCE..=MAX_IFC_COMPARE_TOLERANCE).contains(&tolerance) {
        return Err(Stop::failed(checks::error(
            codes::INVALID_TOLERANCE,
            format!(
                "Tolerans {tolerance} m; {MIN_IFC_COMPARE_TOLERANCE} ile {MAX_IFC_COMPARE_TOLERANCE} m arasında olmalı."
            ),
            Some("tolerance".into()),
        )));
    }
    let (old_bytes, older) = ifc_file(&input.older, "older")?;
    let (new_bytes, newer) = ifc_file(&input.newer, "newer")?;
    let c = compare::compare(&older, &old_bytes, &newer, &new_bytes, tolerance, &|| false).map_err(|e| {
        Stop::failed(checks::error(codes::IFC_UNREADABLE, e.to_string(), Some("newer".into())))
    })?;
    let file = |path: &str, m: &Model| IfcComparedFile {
        path: path.trim().to_owned(),
        schema: m.summary.schema.clone(),
        objects: m.objects.len() as u64,
    };
    let repeated: Vec<IfcRepeatedId> = c
        .repeated
        .iter()
        .map(|(newer, gid)| IfcRepeatedId { file: if *newer { "newer" } else { "older" }.into(), global_id: gid.clone() })
        .collect();
    let warnings = if repeated.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::IFC_REPEATED_GLOBAL_ID.into(),
            message: format!(
                "{} GlobalId bir dosyada birden çok nesnede ({}): bu nesneler karşılaştırılmadı. Dosyayı yazan programdan \
                 GlobalId'leri tek tek veren bir dışa aktarım isteyin.",
                repeated.len(),
                repeated.iter().take(5).map(|r| r.global_id.as_str()).collect::<Vec<_>>().join(", ")
            ),
            path: None,
        }]
    };
    let out = IfcCompared {
        older: file(&input.older, &older),
        newer: file(&input.newer, &newer),
        added: c.added.iter().map(|&j| named(&newer, j)).collect(),
        removed: c.removed.iter().map(|&i| named(&older, i)).collect(),
        changed: c
            .changed
            .iter()
            .map(|x| {
                let n = named(&newer, x.newer);
                IfcComparedChange { global_id: n.global_id, class: n.class, name: n.name, changes: changes(&older, &newer, &x.changes) }
            })
            .collect(),
        unchanged: c.unchanged as u64,
        repeated,
        tolerance,
    };
    Ok((out, warnings))
}

pub fn validate(_: &ExecutionContext<'_>, input: &IfcCompare) -> CommandResult<()> {
    match compared(input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(_: &ExecutionContext<'_>, input: &IfcCompare) -> CommandResult<IfcCompared> {
    match compared(input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// Reads only: its answer is its plan's.
pub fn execute(cx: &mut ExecutionContext<'_>, input: IfcCompare) -> CommandResult<IfcCompared> {
    plan(cx, &input)
}
