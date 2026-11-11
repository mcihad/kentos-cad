//! The building's IFC (docs/adr/8047): `cad.arch.ifc` v1 and the adapter from KentOS's document to the IFC
//! core. It gives `kentos_ifc::arch` the section model (`sections::model`), the drawing's objects with their ids, the
//! wall styles' names, the rooms' net areas and the project's architecture settings; every IFC rule (classes, names,
//! properties, geometry, what is left out) is in the kentos-ifc crate. The command writes the file whole or not at
//! all; the drawing does not change. The shared cases are `fixtures/commands/v1/cad.arch.ifc.json`.

use std::path::{Path, PathBuf};

use kentos_commands::{CommandResult, CommandWarning, ExportIfcBuilding, IfcBuildingExported};
use kentos_domain::Document;
use kentos_ifc::arch::{Gap, Input, Why, Written};
use kentos_ifc::building::Building;
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_ifc::write_file;
use crate::road_landxml::{now, stamp_of};
use crate::road_terrains::fail;
use crate::sections::{self, Left};

/// The building and what was written, with the walls and stairs the section model drew as chords.
fn made(doc: &Document, stamp: &str, application: &str) -> (Building, Written, usize) {
    let model = sections::model(doc);
    let curved = model
        .left
        .iter()
        .filter(|(why, owner)| *why == Left::Curved && owner.is_some())
        .count();
    let entities: Vec<(u128, String, &kentos_commands::Entity)> = doc
        .entities_with_uids()
        .map(|(u, e)| (u.as_u128(), u.to_string(), e))
        .collect();
    let gaps: Vec<Gap> = model
        .gaps
        .iter()
        .map(|g| Gap {
            ring: g.ring.clone(),
            sill: g.sill,
            head: g.head,
            form: g.form,
            owner: g.owner.clone(),
        })
        .collect();
    let mut search = kentos_domain::space_follow::SpaceSearch::new(doc);
    let mut net_area = |room: &kentos_commands::SpaceFields| search.net_area(room);
    let wall_style_name = |style: Option<&str>| doc.wall_style_name(style);
    let (b, written) = kentos_ifc::arch::building(Input {
        title: doc.name(),
        stamp,
        application,
        levels: model.levels.clone(),
        solids: &model.solids,
        owners: &model.owners,
        gaps,
        entities,
        wall_style_name: &wall_style_name,
        net_area: &mut net_area,
        arch: doc.settings().arch.as_ref(),
    });
    (b, written, curved)
}

/// The building to write: `stamp` its time (`YYYY-MM-DDThh:mm:ss`), `application` KentOS's name.
pub fn building(doc: &Document, stamp: &str, application: &str) -> (Building, Written) {
    let (b, written, _) = made(doc, stamp, application);
    (b, written)
}

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_IFC, words, path)
}

fn losses(message: String) -> CommandWarning {
    CommandWarning {
        code: codes::EXPORT_LOSSES.into(),
        message,
        path: None,
    }
}

/// What was left out for `why`, its kinds in this order with how many ("2 duvar, 1 kapı"); empty: nothing.
fn counted(written: &Written, why: Why) -> String {
    const ORDER: [&str; 12] = [
        "duvar",
        "döşeme",
        "kolon",
        "kiriş",
        "çatı",
        "korkuluk",
        "merdiven",
        "rampa",
        "kapı",
        "pencere",
        "açıklık",
        "mahal",
    ];
    ORDER
        .iter()
        .filter_map(|what| {
            let n = written
                .left
                .iter()
                .filter(|l| l.what == *what && l.why == why)
                .count();
            (n > 0).then(|| format!("{n} {what}"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

struct Planned {
    path: PathBuf,
    out: IfcBuildingExported,
    text: String,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &ExportIfcBuilding) -> Result<Planned, Stop> {
    let text = input.path.trim();
    if text.is_empty() || !Path::new(text).is_absolute() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            "IFC'nin yolu boş ya da göreli; mutlak bir .ifc yolu verin.".into(),
            "path",
        ));
    }
    if !text.to_ascii_lowercase().ends_with(".ifc") {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!("“{text}” bir .ifc dosyası değil."),
            "path",
        ));
    }
    let path = PathBuf::from(text);
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if !folder.is_dir() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” klasörü yok; dosya var olan bir klasöre yazılır.",
                folder.display()
            ),
            "path",
        ));
    }
    if path.exists() && !input.overwrite {
        return Err(fail(
            codes::FILE_EXISTS,
            format!(
                "{} var; üzerine yazmak için overwrite verin.",
                path.display()
            ),
            "path",
        ));
    }
    let stamp = match input.stamp.as_deref() {
        None => {
            let (d, t) = now();
            format!("{d}T{t}")
        }
        Some(s) => {
            let (d, t) = stamp_of(s).ok_or_else(|| {
                bad(
                    "stamp",
                    format!("“{s}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın."),
                )
            })?;
            format!("{d}T{t}")
        }
    };
    let (b, written, curved) = made(
        doc,
        &stamp,
        &format!("KentOS Mimari {}", env!("CARGO_PKG_VERSION")),
    );
    let held = written.walls
        + written.slabs
        + written.columns
        + written.beams
        + written.roofs
        + written.railings
        + written.stairs
        + written.spaces;
    if held == 0 {
        return Err(bad(
            "",
            "Yazılacak bina yok: çizimde gövdesi olan duvar, döşeme, kolon, kiriş, çatı, korkuluk, merdiven ya da mahal \
             yok."
                .into(),
        ));
    }
    let mut warnings = Vec::new();
    for (why, because) in [
        (Why::NoBody, "kesit modeli gövdesini kuramadı"),
        (Why::NoWall, "duvarı yazılmadı"),
        (Why::NoArea, "alanı yok"),
        (Why::NotYet, "henüz IFC'ye aktarılmıyor"),
    ] {
        let list = counted(&written, why);
        if !list.is_empty() {
            warnings.push(losses(format!("{list} IFC'ye yazılmadı: {because}.")));
        }
    }
    if curved > 0 {
        warnings.push(losses(format!(
            "{curved} eğri duvar ya da merdiven IFC'ye düz parçalarla yazıldı."
        )));
    }
    let text = kentos_ifc::building::write(&b);
    let sha256 = Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let n = |x: usize| x as u32;
    let out = IfcBuildingExported {
        path: path.display().to_string(),
        schema: "IFC4X3_ADD2".into(),
        storeys: n(written.storeys),
        walls: n(written.walls),
        doors: n(written.doors),
        windows: n(written.windows),
        openings: n(written.openings),
        spaces: n(written.spaces),
        slabs: n(written.slabs),
        columns: n(written.columns),
        beams: n(written.beams),
        roofs: n(written.roofs),
        railings: n(written.railings),
        stairs: n(written.stairs),
        bytes: text.len() as u64,
        sha256,
    };
    Ok(Planned {
        path,
        out,
        text,
        warnings,
    })
}

/// Checks `input` against the document; writes nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ExportIfcBuilding) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What the file would hold; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ExportIfcBuilding,
) -> CommandResult<IfcBuildingExported> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: p.out,
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the file; the drawing does not change.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ExportIfcBuilding,
) -> CommandResult<IfcBuildingExported> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if let Err(why) = write_file(&p.path, &p.text) {
        return Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("IFC yazılamadı: {why}. Hiçbir şey değişmedi."),
            Some("path".into()),
        ))
        .into();
    }
    CommandResult::Completed {
        output: p.out,
        warnings: p.warnings,
    }
}
