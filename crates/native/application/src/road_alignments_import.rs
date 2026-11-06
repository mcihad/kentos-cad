//! `road.alignments.import` v1 (plan/road.md Y10.2, docs/adr/0908 §7; ROAD-IO-05): a LandXML file's alignments taken
//! in as KentOS alignments. Each one's PIs come from its elements (`kentos_road::alignment::fit`); KentOS's alignment
//! built from them is set against the file's elements end by end, and one that lies more than 1 mm from them, or whose
//! elements KentOS's PIs cannot hold, is left out and said. Its km equations and (unless `withoutProfiles`) its
//! profiles come too. One undo step “LandXML aks al”. The shared cases are
//! `fixtures/commands/v1/road.alignments.import.json`.

use std::path::Path;

use kentos_commands::{
    AlignmentDef, AlignmentImported, AlignmentPi, AlignmentStationing, AlignmentsImport, AlignmentsImported,
    CommandResult, CommandWarning, ProfileDef, ProfilePvi, StationEquationDef,
};
use kentos_domain::Document;
use kentos_formats::landxml::{self, LxAlignment, LxKind, LxVCurve};
use kentos_road::alignment::fit::{FitElement, FitError, FitKind, pis_of};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "LandXML aks al";

/// How far (m) KentOS's alignment may lie from the file's elements at their ends.
pub const FIT: f64 = 0.001;

fn left(message: String) -> CommandWarning {
    CommandWarning { code: codes::LANDXML_LEFT.into(), message, path: Some("alignments".into()) }
}

fn fit_words(e: FitError) -> String {
    match e {
        FitError::Empty => "elemanı yok".into(),
        FitError::Unplaced { element } => format!("{}. elemanın merkezi ya da PI'sı yok", element + 1),
        FitError::Compound { element } => format!(
            "{}. elemanda aralarında aliyman olmayan bileşik ya da ters kurp var; KentOS'un PI'larında her kurp aliymanlar arasındadır",
            element + 1
        ),
        FitError::Spiral { element } => format!(
            "{}. eleman aliymandan kurba giden bir klotoid değil (ove klotoid ya da yarıçapı kurbunkinden farklı)",
            element + 1
        ),
        FitError::NoMeet { element } => format!("{}. elemandaki kurbun giriş ve çıkış teğetleri buluşmuyor", element + 1),
        FitError::Degenerate { element } => format!("{}. elemanın boyu yok", element + 1),
    }
}

/// The file's elements as the fit takes them; none (with why) where one is not a clothoid.
fn elements(a: &LxAlignment) -> Result<Vec<FitElement>, String> {
    a.elements
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let kind = match &e.kind {
                LxKind::Line => FitKind::Line,
                LxKind::Curve { radius, clockwise, center } => {
                    FitKind::Curve { radius: *radius, clockwise: *clockwise, center: *center }
                }
                LxKind::Spiral { clothoid: false, .. } => return Err(format!("{}. eleman klotoid olmayan bir geçiş eğrisi", i + 1)),
                LxKind::Spiral { radius_start, radius_end, clockwise, .. } => {
                    FitKind::Spiral { radius_start: *radius_start, radius_end: *radius_end, clockwise: *clockwise }
                }
            };
            let length = e.length.unwrap_or_else(|| (e.end[0] - e.start[0]).hypot(e.end[1] - e.start[1]));
            Ok(FitElement { kind, start: e.start, end: e.end, pi: e.pi, length })
        })
        .collect()
}

/// The km: the file's start and its equations, the km back worked out where the file does not say it.
fn stationing(a: &LxAlignment) -> Option<AlignmentStationing> {
    let mut equations = Vec::new();
    let (mut ahead, mut dir, mut at) = (a.sta_start, 1.0, 0.0);
    for q in &a.equations {
        let dist = q.internal - a.sta_start;
        let back = q.back.unwrap_or(ahead + dir * (dist - at));
        equations.push(StationEquationDef { back, ahead: q.ahead, increasing: q.increasing });
        (ahead, dir, at) = (q.ahead, if q.increasing { 1.0 } else { -1.0 }, dist);
    }
    let s = AlignmentStationing { start: a.sta_start, equations };
    (!s.is_empty()).then_some(s)
}

struct Checked {
    alignments: Vec<AlignmentDef>,
    profiles: Vec<ProfileDef>,
    out: Vec<AlignmentImported>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &AlignmentsImport) -> Result<Checked, Stop> {
    alignments::road_project(doc)?;
    let path = Path::new(&input.path);
    if !path.exists() {
        return Err(fail(codes::FILE_NOT_FOUND, format!("{} yok; dosyanın yolunu denetleyin.", input.path), "path"));
    }
    let bytes = std::fs::read(path).map_err(|e| {
        fail(codes::IMPORT_UNREADABLE, format!("{} okunamadı: {e}. Dosyanın izinlerini denetleyin.", input.path), "path")
    })?;
    let file = landxml::read(&bytes).map_err(|e| {
        fail(codes::IMPORT_UNREADABLE, format!("{} LandXML olarak okunamadı: {}.", input.path, e.words()), "path")
    })?;
    if file.alignments.is_empty() {
        return Err(fail(codes::LANDXML_NO_ALIGNMENT, "Dosyada aks yok.".into(), "path"));
    }
    let chosen: Vec<&LxAlignment> = match &input.alignments {
        None => file.alignments.iter().collect(),
        Some(names) => {
            let mut out = Vec::new();
            for (i, n) in names.iter().enumerate() {
                let a = file.alignments.iter().find(|a| a.name == *n).ok_or_else(|| {
                    let all: Vec<String> = file.alignments.iter().map(|a| format!("“{}”", a.name)).collect();
                    fail(
                        codes::LANDXML_NO_ALIGNMENT,
                        format!("“{n}” adlı aks dosyada yok; dosyanın aksları: {}.", all.join(", ")),
                        &format!("alignments[{i}]"),
                    )
                })?;
                if !out.iter().any(|b: &&LxAlignment| std::ptr::eq(*b, a)) {
                    out.push(a);
                }
            }
            out
        }
    };
    let mut list = alignments::alignments(doc).to_vec();
    let mut profiles = doc.road_profiles().to_vec();
    let (mut out, mut warnings, mut first_why) = (Vec::new(), Vec::new(), None);
    for a in chosen {
        let mut skip = |why: String, warnings: &mut Vec<CommandWarning>| {
            let words = format!("“{}” aksı alınmadı: {why}.", a.name);
            first_why.get_or_insert_with(|| words.clone());
            warnings.push(left(words));
        };
        if !a.left.is_empty() {
            skip(format!("KentOS'un almadığı elemanlar var ({})", a.left.join(", ")), &mut warnings);
            continue;
        }
        let pis = match elements(a).and_then(|els| pis_of(&els).map_err(fit_words)) {
            Ok(p) => p,
            Err(why) => {
                skip(why, &mut warnings);
                continue;
            }
        };
        if list.iter().any(|d| d.name.to_lowercase() == a.name.to_lowercase()) {
            return Err(fail(
                codes::INVALID_ALIGNMENT,
                format!("Projede “{}” adlı bir aks var; dosyanınkini almadan önce birini yeniden adlandırın.", a.name),
                "alignments",
            ));
        }
        let id = (1..).map(|n| format!("g-{n}")).find(|id| list.iter().all(|d| &d.id != id)).unwrap_or_default();
        let def = AlignmentDef {
            id: id.clone(),
            name: a.name.clone(),
            revision: 1,
            pis: pis
                .iter()
                .map(|p| AlignmentPi { id: p.id.clone(), x: p.x, y: p.y, radius: p.radius, spiral_in: p.spiral_in, spiral_out: p.spiral_out, ove: false })
                .collect(),
            offset_of: None,
            stationing: stationing(a),
        };
        let mut with = list.clone();
        with.push(def.clone());
        let built = match alignments::build(&with, &def, "") {
            Ok(b) => b,
            Err(stop) => {
                skip(format!("PI'larından aks kurulamıyor: {}", crate::road_corridors::words(stop)), &mut warnings);
                continue;
            }
        };
        let els = &built.core.elements;
        if els.len() != a.elements.len() {
            skip(
                format!("PI'larından kurulan aksın {} elemanı var, dosyanın {}", els.len(), a.elements.len()),
                &mut warnings,
            );
            continue;
        }
        let gap = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).hypot(p[1] - q[1]);
        let deviation = els
            .iter()
            .zip(&a.elements)
            .map(|(e, f)| gap(e.start, f.start).max(gap(e.point(e.length), f.end)))
            .fold(0.0, f64::max);
        if deviation > FIT {
            skip(format!("PI'larından kurulan aks dosyanın elemanlarından {deviation:.4} m ayrılıyor (en çok {FIT} m)"), &mut warnings);
            continue;
        }
        let mut names = Vec::new();
        if !input.without_profiles {
            for p in &a.profiles {
                let n = p.pvis.len();
                let pvis: Vec<ProfilePvi> = p
                    .pvis
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let (mut length, mut length_in, mut length_out, mut radius) = (0.0, 0.0, 0.0, 0.0);
                        match v.curve {
                            LxVCurve::None => {}
                            LxVCurve::Parabola { length: l } => length = l,
                            LxVCurve::Unsymmetric { length_in: a, length_out: b } => (length_in, length_out) = (a, b),
                            LxVCurve::Circle { radius: r, .. } => radius = r,
                        }
                        let id = if i == 0 { "a".into() } else if i + 1 == n { "b".into() } else { format!("v{i}") };
                        ProfilePvi { id, station: v.station - a.sta_start, elevation: v.elevation, length, length_in, length_out, radius }
                    })
                    .collect();
                let name = if p.name.is_empty() { format!("{} kotu", a.name) } else { p.name.clone() };
                if profiles.iter().any(|q| q.name.to_lowercase() == name.to_lowercase()) {
                    warnings.push(left(format!("“{name}” boykesiti alınmadı: projede aynı adlı bir boykesit var.")));
                    continue;
                }
                let pid = (1..).map(|k| format!("b-{k}")).find(|id| profiles.iter().all(|q| &q.id != id)).unwrap_or_default();
                let def = ProfileDef {
                    id: pid,
                    name: name.clone(),
                    alignment: id.clone(),
                    alignment_revision: 1,
                    revision: 1,
                    pvis,
                    surface: None,
                    crossings: Vec::new(),
                };
                if kentos_road::profile::vertical::build(&crate::road_profiles::pvis_of(&def)).is_err() {
                    warnings.push(left(format!("“{name}” boykesiti alınmadı: PVI'larından düşey güzergâh kurulamıyor.")));
                    continue;
                }
                names.push(name);
                profiles.push(def);
            }
        }
        out.push(AlignmentImported {
            id,
            name: a.name.clone(),
            pis: def.pis.clone(),
            elements: a.elements.len() as u32,
            deviation,
            profiles: names,
        });
        list.push(def);
    }
    if out.is_empty() {
        return Err(fail(
            codes::LANDXML_NO_ALIGNMENT,
            first_why.unwrap_or_else(|| "Dosyanın hiçbir aksı alınamadı.".into()),
            "alignments",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { alignments: list, profiles, out, warnings })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<AlignmentsImported> {
    let group = cx.doc.begin_group(STEP);
    let done = cx
        .doc
        .set_road_alignments(c.alignments, STEP)
        .and_then(|_| cx.doc.set_road_profiles(c.profiles, STEP));
    if let Err(r) = done {
        cx.doc.cancel_group(group);
        return fail(codes::INVALID_ALIGNMENT, format!("Aks kurallarına uymuyor: {}.", r.0), "").into();
    }
    cx.doc.end_group(group);
    CommandResult::Completed {
        output: AlignmentsImported { alignments: c.out, revision: cx.doc.revision().to_string() },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document and the file, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AlignmentsImport) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would take in; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &AlignmentsImport) -> CommandResult<AlignmentsImported> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Takes the alignments in: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AlignmentsImport) -> CommandResult<AlignmentsImported> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_ALIGNMENTS_IMPORT,
        kentos_commands::ROAD_ALIGNMENTS_IMPORT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
