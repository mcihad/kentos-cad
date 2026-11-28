//! `cad.underlay.check` v1 (“Altlık modeli denetle”, docs/adr/8102; plan/bim-plan.md BIM-17, B19): an underlay model
//! set beside the drawing's own. It reads; it writes nothing.
//!
//! - **Before any geometry:** the file is the revision the drawing names (its SHA-256) and is placed as the drawing says
//!   (`kentos_ifc::underlay::place`): another system or height datum refuses here, nothing else is looked at. Placed by
//!   its map conversion while the file or the project does not say its height datum, the heights are taken as the
//!   same, and a warning says so (`underlay_datum_unsaid`).
//! - **Levels:** the project's levels and the file's storeys (their `Elevation` in the drawing's heights) paired by
//!   elevation, the nearest first; each pair's difference, the unpaired on either side.
//! - **Axes:** the project's grids and the file's by their labels: the distance of the file's from the project's line
//!   and the turn between them.
//! - **Elements:** an element of the file and one of the drawing's model of the same class whose boxes stand within the
//!   tolerance are the same element (matched): the drawing's box as its frame is joined, or as drawn. The section model
//!   joins the frame (docs/adr/8171 §2.1: a column cuts a beam, a slab holds what lies in it); another program joins it
//!   by its own rules, a structural model's beam running over the columns: by the joined box alone, the section
//!   building's 8 beams went unmatched and clashed with its columns and slab. The file's other elements are set against
//!   every element of the drawing's model as joined (`kentos_ifc::clash`); the elements of the file's classes that
//!   neither match nor clash, on either side, are said too.

use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, MAX_UNDERLAY_TOLERANCE, MIN_UNDERLAY_TOLERANCE, ModelUnderlay, UNDERLAY_TOLERANCE,
    UnderlayAxisCheck, UnderlayCheck, UnderlayChecked, UnderlayClash, UnderlayLevelCheck, UnderlayPlacement,
    UnderlayUnmatched,
};
use kentos_domain::{Document, Uuid};
use kentos_ifc::building::Class;
use kentos_ifc::clash::{self, Body};
use kentos_ifc::mesh::{self, Kind};
use kentos_ifc::underlay::{self, MapFrame, Transform};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// A class as the file's objects spell it.
fn class_name(c: Class) -> &'static str {
    match c {
        Class::Wall => "IfcWall",
        Class::Slab => "IfcSlab",
        Class::Column => "IfcColumn",
        Class::Beam => "IfcBeam",
        Class::Roof => "IfcRoof",
        Class::Railing => "IfcRailing",
        Class::Stair => "IfcStair",
        Class::Covering => "IfcCovering",
        Class::Door => "IfcDoor",
        Class::Window => "IfcWindow",
        Class::TransportElement => "IfcTransportElement",
        Class::Furniture => "IfcFurniture",
        Class::SanitaryTerminal => "IfcSanitaryTerminal",
        Class::ElectricAppliance => "IfcElectricAppliance",
        Class::Member => "IfcMember",
        Class::Chimney => "IfcChimney",
        Class::Ramp => "IfcRamp",
        Class::CurtainWall => "IfcCurtainWall",
        Class::Footing => "IfcFooting",
    }
}

/// The underlay's file read and placed.
pub(crate) struct Opened {
    pub model: kentos_ifc::read::Model,
    pub bytes: Vec<u8>,
    /// Into the drawing.
    pub transform: Transform,
    /// Placed by its map conversion while the file or the project does not say its height datum: its heights were
    /// taken as the project's (the warning's words).
    pub datum_unsaid: Option<String>,
}

/// The underlay's file read and placed; refused before any geometry when it is not the revision the drawing names or
/// cannot be placed.
pub(crate) fn opened(doc: &Document, u: &ModelUnderlay) -> Result<Opened, Stop> {
    let path = Path::new(&u.file);
    let bytes = std::fs::read(path)
        .map_err(|_| fail(codes::FILE_NOT_FOUND, format!("{} okunamadı: dosya yok ya da açılamıyor.", u.file), "id"))?;
    if kentos_commands::encoding::sha256_hex(&bytes) != u.sha256 {
        return Err(fail(
            codes::MODEL_UNDERLAY_CHANGED,
            format!(
                "“{}” dosyası eklendiğinden beri değişmiş: çizimin andığı revizyon değil. Önce Yeniden oku ile yeni \
                 revizyonu alın.",
                u.name
            ),
            "id",
        ));
    }
    let model = kentos_ifc::read::read(&bytes, &kentos_ifc::read::Limits::default(), &|| false).map_err(|e| {
        fail(codes::MODEL_UNDERLAY_UNREADABLE, format!("{} bir IFC dosyası olarak okunamadı: {e}", u.name), "id")
    })?;
    let unit = model.summary.units.length.as_ref().map_or(f64::NAN, |x| x.to_si);
    let frame = model.summary.location.as_ref().and_then(kentos_ifc::read::Location::frame);
    let project = crate::arch_ifc::crs(doc).map(|c| MapFrame::from(&c));
    let how = match u.placement {
        UnderlayPlacement::Map => underlay::Placement::Map,
        UnderlayPlacement::Origin => underlay::Placement::Origin,
    };
    let transform = underlay::place(frame.as_ref(), unit, project.as_ref().map_err(String::as_str), how)
        .map_err(|why| fail(codes::MODEL_UNDERLAY_NOT_PLACED, format!("{}: {why}", u.name), "id"))?;
    let datum_unsaid = match (how, frame.as_ref().and_then(|f| f.vertical.as_deref()), project.ok().and_then(|p| p.vertical)) {
        (underlay::Placement::Origin, ..) | (_, Some(_), Some(_)) => None,
        (_, Some(f), None) => Some(format!(
            "Projenin yükseklik sistemi söylenmiyor: dosyanın kotları (“{f}”) projeninkilerle aynı datumda sayıldı; kat \
             kotlarının farkı ve çakışmaların yüksekliği bu varsayıma dayanır. Proje ayarları › Kotlar'da yükseklik \
             sistemini seçin."
        )),
        (_, None, Some(p)) => Some(format!(
            "Dosya yükseklik datumunu söylemiyor: kotları projeninkiyle (“{p}”) aynı datumda sayıldı; kat kotlarının farkı \
             ve çakışmaların yüksekliği bu varsayıma dayanır. Dosyayı yükseklik datumuyla yeniden dışa aktarın."
        )),
        (_, None, None) => Some(
            "Ne proje ne dosya yükseklik datumunu söylüyor: kotlar aynı datumda sayıldı; kat kotlarının farkı ve \
             çakışmaların yüksekliği bu varsayıma dayanır. Proje ayarları › Kotlar'da yükseklik sistemini seçin."
                .to_owned(),
        ),
    };
    Ok(Opened { model, bytes, transform, datum_unsaid })
}

/// Whether two boxes stand within `tolerance` of each other on every side.
fn same_box(a: &[f64; 6], b: &[f64; 6], tolerance: f64) -> bool {
    (0..6).all(|i| (a[i] - b[i]).abs() <= tolerance)
}

fn check(doc: &Document, input: &UnderlayCheck) -> Result<(UnderlayChecked, Vec<CommandWarning>), Stop> {
    let id = input.id.as_str();
    if !Uuid::parse_str(id).is_ok_and(|u| u.to_string() == id) {
        return Err(fail(codes::INVALID_UID, format!("“{id}” küçük harfli, tireli bir UUID değil."), "id"));
    }
    let Some(u) = doc.underlays().iter().find(|u| u.id == id).cloned() else {
        return Err(fail(codes::MODEL_UNDERLAY_NOT_FOUND, format!("“{id}” kimlikli altlık model çizimde yok."), "id"));
    };
    let tolerance = input.tolerance.unwrap_or(UNDERLAY_TOLERANCE);
    if !(MIN_UNDERLAY_TOLERANCE..=MAX_UNDERLAY_TOLERANCE).contains(&tolerance) {
        return Err(fail(
            codes::INVALID_TOLERANCE,
            format!("Tolerans {tolerance} m; {MIN_UNDERLAY_TOLERANCE} ile {MAX_UNDERLAY_TOLERANCE} m arasında olmalı."),
            "tolerance",
        ));
    }
    let Opened { model, bytes, transform: t, datum_unsaid } = opened(doc, &u)?;
    let warnings: Vec<CommandWarning> = datum_unsaid
        .into_iter()
        .map(|message| CommandWarning { code: codes::UNDERLAY_DATUM_UNSAID.into(), message, path: Some("id".into()) })
        .collect();
    let unreadable =
        |e: kentos_ifc::read::ReadError| fail(codes::MODEL_UNDERLAY_UNREADABLE, format!("{}: {e}", u.name), "id");
    let bodies = model.bodies(&bytes, &|| false).map_err(unreadable)?;
    let storeys = model.storeys(&bytes).map_err(unreadable)?;
    let file_axes = model.grid_axes(&bytes).map_err(unreadable)?;

    // Levels: the project's and the file's storeys in the drawing's heights.
    let project_levels: Vec<(String, f64)> = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.iter().map(|l| (l.name.clone(), l.elevation)).collect())
        .unwrap_or_default();
    let file_levels: Vec<(String, f64)> = storeys
        .iter()
        .filter_map(|s| s.elevation.map(|e| (s.name.clone(), t.z_scale * e + t.z_shift)))
        .collect();
    let mut levels: Vec<UnderlayLevelCheck> = underlay::pair_levels(
        &project_levels.iter().map(|l| l.1).collect::<Vec<_>>(),
        &file_levels.iter().map(|l| l.1).collect::<Vec<_>>(),
    )
    .into_iter()
    .map(|(i, j)| {
        let level = i.map(|i| &project_levels[i]);
        let storey = j.map(|j| &file_levels[j]);
        UnderlayLevelCheck {
            level: level.map(|l| l.0.clone()),
            storey: storey.map(|s| s.0.clone()),
            project: level.map(|l| l.1),
            file: storey.map(|s| s.1),
            difference: level.zip(storey).map(|(l, s)| s.1 - l.1),
        }
    })
    .collect();
    // A storey whose file says no elevation: listed, not guessed.
    levels.extend(storeys.iter().filter(|s| s.elevation.is_none()).map(|s| UnderlayLevelCheck {
        level: None,
        storey: Some(s.name.clone()),
        project: None,
        file: None,
        difference: None,
    }));

    // Axes, by their labels.
    let mut project_axes: Vec<(String, [[f64; 2]; 2])> = Vec::new();
    for e in doc.entities() {
        if let Entity::Grid(g) = e
            && let (Some(a), Some(b)) = (g.grid.pts.first(), g.grid.pts.last())
            && g.grid.pts.len() >= 2
        {
            project_axes.push((g.grid.label.trim().to_owned(), [[a.x, a.y], [b.x, b.y]]));
        }
    }
    let placed_axes: Vec<(String, [[f64; 2]; 2])> = file_axes
        .iter()
        .map(|x| {
            let (a, b) = (t.apply(x.a), t.apply(x.b));
            (x.tag.trim().to_owned(), [[a[0], a[1]], [b[0], b[1]]])
        })
        .collect();
    let mut labels: Vec<String> = project_axes.iter().chain(&placed_axes).map(|x| x.0.clone()).collect();
    labels.sort();
    labels.dedup();
    let axes = labels
        .into_iter()
        .map(|label| {
            let p = project_axes.iter().find(|x| x.0 == label);
            let f = placed_axes.iter().find(|x| x.0 == label);
            // An axis of no length has no line to stand from: neither is said.
            let measured =
                p.zip(f).map(|(p, f)| underlay::axis_offset(p.1, f.1)).filter(|m| m.0.is_finite() && m.1.is_finite());
            UnderlayAxisCheck {
                in_project: p.is_some(),
                in_file: f.is_some(),
                offset: measured.map(|m| m.0),
                turn: measured.map(|m| m.1),
                label,
            }
        })
        .collect();

    // The drawing's model: its elements' bodies (no room), as the IFC export makes them.
    let (building, _, _) = crate::arch_ifc::model(doc);
    let own: Vec<mesh::Mesh> = mesh::meshes(&building).into_iter().filter(|m| matches!(m.kind, Kind::Element(_))).collect();
    let own_class = |m: &mesh::Mesh| match m.kind {
        Kind::Element(c) => class_name(c),
        Kind::Space => "IfcSpace",
    };
    let own_bodies: Vec<Body<'_>> =
        own.iter().enumerate().map(|(i, m)| Body { owner: i, positions: &m.vertices, triangles: &m.triangles }).collect();
    let own_boxes: Vec<Option<[f64; 6]>> = own_bodies.iter().map(clash::bounds).collect();
    // The same elements' boxes as drawn, by their ids.
    let unjoined = crate::arch_ifc::unjoined_model(doc);
    let drawn: std::collections::HashMap<u128, [f64; 6]> = mesh::meshes(&unjoined)
        .iter()
        .filter(|m| matches!(m.kind, Kind::Element(_)))
        .filter_map(|m| {
            let body = Body { owner: 0, positions: &m.vertices, triangles: &m.triangles };
            Some((m.id?, clash::bounds(&body)?))
        })
        .collect();

    // The file's bodies placed in the drawing.
    #[allow(clippy::type_complexity)]
    let placed: Vec<(usize, Vec<[f64; 3]>, Vec<[u32; 3]>)> = bodies
        .bodies
        .iter()
        .map(|b| (b.object, b.positions.iter().map(|p| t.apply(*p)).collect(), b.triangles.clone()))
        .collect();
    let file_bodies: Vec<Body<'_>> =
        placed.iter().map(|(o, p, tr)| Body { owner: *o, positions: p, triangles: tr }).collect();
    let file_class = |o: usize| model.objects.get(o).map_or("", |x| x.class.as_str());

    // Matched: the same class, the same box; each element once, the file's in its order taking the drawing's first
    // free one by persistent id.
    let uid_of = |m: &mesh::Mesh| m.id.map(|x| Uuid::from_u128(x).to_string()).unwrap_or_default();
    let mut by_uid: Vec<usize> = (0..own.len()).collect();
    by_uid.sort_by_key(|&i| uid_of(&own[i]));
    let mut own_taken = vec![false; own.len()];
    let mut file_matched = vec![false; file_bodies.len()];
    for (j, fb) in file_bodies.iter().enumerate() {
        let Some(fbox) = clash::bounds(fb) else { continue };
        let class = file_class(fb.owner);
        if let Some(&i) = by_uid.iter().find(|&&i| {
            !own_taken[i]
                && own_class(&own[i]).eq_ignore_ascii_case(class)
                && (own_boxes[i].as_ref().into_iter().chain(own[i].id.and_then(|id| drawn.get(&id))))
                    .any(|b| same_box(b, &fbox, tolerance.max(1e-6)))
        }) {
            own_taken[i] = true;
            file_matched[j] = true;
        }
    }
    let matched = file_matched.iter().filter(|m| **m).count();

    // Clashes: the file's unmatched elements against all the drawing's.
    let loose: Vec<Body<'_>> =
        file_bodies.iter().zip(&file_matched).filter(|(_, m)| !**m).map(|(b, _)| *b).collect();
    // In the file's order, each file object's by the drawing's persistent id.
    let mut found = clash::clashes(&own_bodies, &loose, tolerance);
    found.sort_by(|x, y| x.b.cmp(&y.b).then_with(|| uid_of(&own[x.a]).cmp(&uid_of(&own[y.a]))));
    let clashes: Vec<UnderlayClash> = found
        .iter()
        .map(|c| {
            let m = &own[c.a];
            let o = &model.objects[c.b];
            UnderlayClash {
                uid: uid_of(m),
                class: own_class(m).to_owned(),
                name: m.name.clone(),
                object: o.global_id.clone(),
                object_class: o.class.clone(),
                object_name: o.name.clone(),
                overlap: c.overlap.to_vec(),
                inside: c.inside,
            }
        })
        .collect();

    // What only one model has, of the classes the file has: neither matched nor clashing.
    let file_classes: std::collections::BTreeSet<String> =
        file_bodies.iter().map(|b| file_class(b.owner).to_ascii_lowercase()).collect();
    let clashing: std::collections::BTreeSet<usize> = found.iter().map(|c| c.b).collect();
    let own_clashing: std::collections::BTreeSet<usize> = found.iter().map(|c| c.a).collect();
    let mut unmatched: Vec<UnderlayUnmatched> = own
        .iter()
        .enumerate()
        .zip(&own_taken)
        .filter(|((i, m), taken)| {
            !**taken && !own_clashing.contains(i) && file_classes.contains(&own_class(m).to_ascii_lowercase())
        })
        .map(|((_, m), t)| (m, t))
        .map(|(m, _)| UnderlayUnmatched { uid: Some(uid_of(m)), object: None, class: own_class(m).to_owned(), name: m.name.clone() })
        .collect();
    unmatched.sort_by(|x, y| x.uid.cmp(&y.uid));
    let mut file_loose: Vec<&Body<'_>> =
        file_bodies.iter().zip(&file_matched).filter(|(b, m)| !**m && !clashing.contains(&b.owner)).map(|(b, _)| b).collect();
    file_loose.sort_by_key(|b| b.owner);
    unmatched.extend(file_loose.into_iter().map(
        |b| {
            let o = &model.objects[b.owner];
            UnderlayUnmatched {
                uid: None,
                object: Some(o.global_id.clone()),
                class: o.class.clone(),
                name: o.name.clone().unwrap_or_default(),
            }
        },
    ));

    let checked = UnderlayChecked {
        id: u.id,
        levels,
        axes,
        matched: u32::try_from(matched).unwrap_or(u32::MAX),
        clashes,
        unmatched,
        tolerance,
        revision: doc.revision().to_string(),
    };
    Ok((checked, warnings))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &UnderlayCheck) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &UnderlayCheck) -> CommandResult<UnderlayChecked> {
    match check(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// Reads only: its answer is its plan's.
pub fn execute(cx: &mut ExecutionContext<'_>, input: UnderlayCheck) -> CommandResult<UnderlayChecked> {
    plan(cx, &input)
}
