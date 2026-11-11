//! The building's IFC (docs/adr/8047): `cad.arch.ifc` v1 and the adapter from KentOS's document to the IFC
//! core. It gives `kentos_ifc::arch` the section model (`sections::model`), the drawing's objects with their ids, the
//! wall styles' names, the rooms' net areas and the project's architecture settings; every IFC rule (classes, names,
//! properties, geometry, what is left out) is in the kentos-ifc crate. The command writes the file whole or not at
//! all; the drawing does not change. The shared cases are `fixtures/commands/v1/cad.arch.ifc.json`.

use std::path::{Path, PathBuf};

use kentos_commands::{CommandResult, CommandWarning, ExportIfcBuilding, IfcBuildingExported};
use kentos_commands::{SiteEdgeKind, VariableValue};
use kentos_domain::Document;
use kentos_ifc::arch::{Gap, Input, Particulars, Why, Written};
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
        particulars: particulars(doc),
        crs: crs(doc),
    });
    (b, written, curved)
}

/// The drawing's künye (the permit set's particulars, docs/adr/8037) and the measures its title block reads, for the
/// IFC's EK-6 and EK-7 words: a value not given, or given as "—" (the künye's default), is none. The areas are İmar
/// durumu's (yapı inşaat alanı: all the storeys'; taban alanı), the use the building program's, the gardens' distances
/// the Vaziyet planı's, each kind's least edge (a public area's edge an ön bahçe, PAİY 23/1-a).
pub fn particulars(doc: &Document) -> Particulars {
    let fold = |s: &str| kentos_sheet::expr::fold(s);
    let vars = &doc.settings().variables;
    let get = |name: &str| -> Option<String> {
        let v = vars.iter().find(|v| fold(&v.name) == fold(name))?;
        let t = match &v.value {
            VariableValue::Text(t) => t.trim().to_owned(),
            VariableValue::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => {
                format!("{}", *n as i64)
            }
            VariableValue::Number(n) => format!("{n}"),
            VariableValue::Null | VariableValue::Bool(_) => return None,
        };
        (!t.is_empty() && t != "—" && t != "-").then_some(t)
    };
    let facts = doc.title_facts();
    let fact = |name: &str| {
        facts
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.value.clone())
    };
    let zoning = doc.zoning_report();
    let mut setbacks: [Option<f64>; 3] = [None; 3];
    if let Some(site) = doc.site_report() {
        for e in &site.edges {
            let k = match e.kind {
                SiteEdgeKind::Front | SiteEdgeKind::Public => 0,
                SiteEdgeKind::Side => 1,
                SiteEdgeKind::Back => 2,
            };
            setbacks[k] = Some(setbacks[k].map_or(e.required, |v: f64| v.min(e.required)));
        }
    }
    Particulars {
        work: get("proje_adi")
            .or_else(|| Some(doc.name().trim().to_owned()).filter(|n| !n.is_empty())),
        project_code: get("proje_kodu"),
        building_code: get("bina_kodu"),
        revision: get("revizyon_no"),
        province: get("il"),
        district: get("ilce"),
        neighbourhood: get("mahalle"),
        block: get("ada"),
        parcel: get("parsel"),
        title_deed: get("tapu_kaydi"),
        architect: get("mimar"),
        architect_title: get("mimar_unvan"),
        registration: get("sicil_no"),
        office: get("buro_adi"),
        office_tax_number: get("vergi_no"),
        office_address: get("buro_adres"),
        occupancy: fact("kullanim_amaci"),
        gross_area: zoning
            .as_ref()
            .map(|r| r.levels.iter().map(|l| l.all()).sum::<f64>()),
        footprint: zoning.as_ref().map(|r| r.footprint),
        setbacks,
    }
}

/// The project's system as the IFC says it (EK-7 Tablo 7.6–7.7, `kentos_ifc::crs`): the drawing's coordinates are the
/// system's, so the map conversion moves them by nothing in plan and by ±0.00's absolute elevation (Vaziyet planı) in
/// height; its heights' datum the project's (an orthometric system by its EPSG name, ellipsoidal heights ITRF96's, EK-7's
/// note). None for a drawing in local coordinates.
pub fn crs(doc: &Document) -> Option<kentos_ifc::crs::Crs> {
    let settings = doc.settings();
    let own = kentos_project::systems::own(settings)?;
    let name = if own.code.starts_with("EPSG:") {
        own.code.clone()
    } else {
        own.name.clone()
    };
    let vertical = settings.heights.as_ref().and_then(|h| match h.kind {
        kentos_commands::HeightKind::Orthometric => h
            .vertical()
            .map(|v| format!("{} (EPSG:{})", v.name, v.srid)),
        kentos_commands::HeightKind::Ellipsoidal => Some("ITRF96".to_owned()),
        kentos_commands::HeightKind::Unknown => None,
    });
    let datum = settings
        .arch
        .as_ref()
        .and_then(|a| a.site.as_ref())
        .and_then(|s| s.datum)
        .unwrap_or(0.0);
    Some(kentos_ifc::crs::projected(
        &name,
        Some(own.title.clone()),
        vertical,
        [0.0, 0.0, datum],
    ))
}

/// The model file's name the save dialog offers (EK-2 Tablo 2.1): `260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc`, or the
/// drawing's name while the künye has no project code.
pub fn file_name(doc: &Document) -> String {
    format!("{}.ifc", particulars(doc).file_stem(doc.name()))
}

/// The model as `cad.arch.ifc` writes it, what it left out, and the künye it read: for Mimari denetim's Dijital proje
/// findings (`digital_audit`, EK-9).
pub fn model(doc: &Document) -> (Building, Written, Particulars) {
    let (b, written, _) = made(doc, "", "KentOS Mimari");
    (b, written, particulars(doc))
}

/// EK-9's model quality control form as a table's rows (`kentos_ifc::quality::form_rows`): its header, each group's
/// title and its questions answered from the model `cad.arch.ifc` writes now.
pub fn form_rows(doc: &Document) -> Vec<Vec<String>> {
    let (b, written, k) = model(doc);
    kentos_ifc::quality::form_rows(&kentos_ifc::quality::ek9(&b, &written, &k))
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
