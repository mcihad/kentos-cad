//! `road.profiles.fit` v1 (plan/road.md Y10.6b, docs/adr/0908 §13; ROAD-IO-14): a profile read off a line or an open
//! polyline drawn in a profile view (`kentos_road::profile::trace`): grades, PVIs and vertical curves. The line is kept;
//! the profile's vertical distance from its vertices (and its arcs' middles) is measured on the built profile. One undo
//! step “Çizimden boykesit”. The shared cases are `fixtures/commands/v1/road.profiles.fit.json`.

use kentos_commands::{CommandResult, CommandWarning, Entity, ProfileDef, ProfileFitted, ProfilePvi, ProfilesFit};
use kentos_domain::Document;
use kentos_road::profile::VCurve;
use kentos_road::profile::trace::{TraceError, View, from_drawing};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_profiles as common;
use crate::road_terrains::fail;

/// The undo step's name.
pub const STEP: &str = "Çizimden boykesit";

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_PROFILE_FIT, words, path)
}

struct Checked {
    list: Vec<ProfileDef>,
    out: ProfileFitted,
    warnings: Vec<CommandWarning>,
    /// The profile's points (station, elevation), for a window.
    line: Vec<[f64; 2]>,
}

/// The line's points and bulges.
fn line_of(doc: &Document, uid: &str) -> Result<(Vec<[f64; 2]>, Vec<f64>), Stop> {
    let entity = doc.entities_with_uids().find(|(u, _)| u.to_string() == uid).map(|(_, e)| e);
    match entity {
        None => Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), "uid")),
        Some(Entity::Line(l)) => Ok((vec![[l.a.x, l.a.y], [l.b.x, l.b.y]], vec![0.0])),
        Some(Entity::Polyline(p)) if p.closed => {
            Err(bad("uid", format!("“{uid}” kapalı bir çoklu çizgi; boykesit açık bir çizgiden kurulur.")))
        }
        Some(Entity::Polyline(p)) if p.parts.as_ref().is_some_and(|q| !q.is_empty()) => {
            Err(bad("uid", format!("“{uid}” çok parçalı bir çoklu çizgi; boykesit tek parçalı bir çizgiden kurulur.")))
        }
        Some(Entity::Polyline(p)) => {
            let pts: Vec<[f64; 2]> = p.pts.iter().map(|v| [v.x, v.y]).collect();
            let n = pts.len().saturating_sub(1);
            let b = p.bulges.clone().unwrap_or_default();
            Ok((pts, (0..n).map(|i| b.get(i).copied().unwrap_or(0.0)).collect()))
        }
        Some(_) => Err(bad("uid", format!("“{uid}” bir çizgi ya da çoklu çizgi değil."))),
    }
}

/// The drawing points the deviation is measured at: every vertex and every arc's middle.
fn probes(pts: &[[f64; 2]], bulges: &[f64]) -> Vec<[f64; 2]> {
    let mut out = pts.to_vec();
    for (i, w) in pts.windows(2).enumerate() {
        let b = bulges.get(i).copied().unwrap_or(0.0);
        if b != 0.0 {
            let (a, c) = (w[0], w[1]);
            let (dx, dy) = (c[0] - a[0], c[1] - a[1]);
            out.push([0.5 * (a[0] + c[0]) + 0.5 * b * dy, 0.5 * (a[1] + c[1]) - 0.5 * b * dx]);
        }
    }
    out
}

fn trace_words(e: TraceError) -> String {
    match e {
        TraceError::TooFew => "çizginin iki ayrı km'si yok".into(),
        TraceError::NotIncreasing { vertex } => format!("{}. köşede km geri gidiyor; boykesit soldan sağa artar", vertex + 1),
        TraceError::Compound { segment } => {
            format!("{}. parçada aralarında eğim olmayan iki yay var", segment + 1)
        }
        TraceError::NoMeet { segment } => format!("{}. parçanın yayının uç teğetleri iki ucu arasında kesişmiyor", segment + 1),
    }
}

fn check(doc: &Document, input: &ProfilesFit) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    common::road_project(doc)?;
    let (_, alignment) = alignments::find(alignments::alignments(doc), &input.alignment, "alignment")?;
    let alignment = alignment.clone();
    let surface = match input.surface.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => Some(common::surface_id(doc, s, "surface")?),
        None => None,
    };
    let (pts, bulges) = line_of(doc, &input.uid)?;
    if !input.datum.is_finite() {
        return Err(bad("datum", "Kıyas kotu (datum) sonlu bir sayı olmalı.".into()));
    }
    if !(input.at[0].is_finite() && input.at[1].is_finite()) {
        return Err(bad("at", "Görünümün yeri (at) sonlu sayılar olmalı.".into()));
    }
    let station = input.station.unwrap_or(0.0);
    if !station.is_finite() {
        return Err(bad("station", "Görünümün km'si (station) sonlu bir sayı olmalı.".into()));
    }
    let positive = |v: Option<f64>, field: &str, what: &str| match v {
        Some(r) if !(r.is_finite() && r > 0.0) => Err(bad(field, format!("{what} 0'dan büyük olmalı."))),
        _ => Ok(()),
    };
    positive(input.exaggeration, "exaggeration", "Abartma (exaggeration)")?;
    positive(input.length, "length", "Düşey kurbun boyu (length)")?;
    positive(input.tolerance, "tolerance", "Tolerans (tolerance)")?;
    let view = View { at: input.at, station, datum: input.datum, exaggeration: input.exaggeration.unwrap_or(1.0) };
    let pvis = from_drawing(&pts, &bulges, view, input.length.unwrap_or(0.0))
        .map_err(|e| bad("uid", format!("Çizimden boykesit kurulamıyor: {}.", trace_words(e))))?;
    let own = common::profiles(doc);
    let id = (1..).map(|n| format!("b-{n}")).find(|i| own.iter().all(|p| &p.id != i)).unwrap_or_default();
    let name = match input.name.as_deref().map(str::trim) {
        Some(n) if !n.is_empty() => n.to_owned(),
        _ => (1..)
            .map(|k| format!("{k}. boykesit"))
            .find(|n| own.iter().all(|p| p.name.to_lowercase() != n.to_lowercase()))
            .unwrap_or_default(),
    };
    if own.iter().any(|p| p.name.to_lowercase() == name.to_lowercase()) {
        return Err(bad("name", format!("Projede “{name}” adlı bir boykesit var.")));
    }
    let def = ProfileDef {
        id: id.clone(),
        name: name.clone(),
        alignment: alignment.id.clone(),
        alignment_revision: alignment.revision,
        revision: 1,
        pvis: pvis
            .iter()
            .map(|p| {
                let (length, length_in, length_out) = match p.curve {
                    VCurve::Parabola { length } => (length, 0.0, 0.0),
                    VCurve::Asymmetric { length_in, length_out } => (0.0, length_in, length_out),
                    VCurve::None | VCurve::Circle { .. } => (0.0, 0.0, 0.0),
                };
                ProfilePvi {
                    id: p.id.clone(),
                    station: p.station,
                    elevation: p.elevation,
                    length,
                    length_in,
                    length_out,
                    radius: 0.0,
                }
            })
            .collect(),
        surface,
        crossings: Vec::new(),
    };
    let mut list = own.to_vec();
    list.push(def.clone());
    let built = common::build(doc, &def).map_err(|stop| {
        bad("uid", format!("Çizimden boykesit kurulamıyor: {}", crate::road_corridors::words(stop)))
    })?;
    common::judged(doc, &list).map_err(|stop| bad("uid", crate::road_corridors::words(stop)))?;
    // The deviation: each probe's vertical distance from the profile at its station.
    let mut worst = (0.0_f64, view.read(pts[0]));
    for p in probes(&pts, &bulges) {
        let q = view.read(p);
        let d = built.vertical.at(q[0]).map_or(0.0, |z| (z.0 - q[1]).abs());
        if d > worst.0 {
            worst = (d, q);
        }
    }
    let mut warnings = Vec::new();
    if let Some(t) = input.tolerance
        && worst.0 > t
    {
        let round = |v: f64| (v * 1000.0).round() / 1000.0;
        warnings.push(CommandWarning {
            code: codes::PROFILE_FIT_DEVIATION.into(),
            message: format!(
                "“{name}” boykesiti çizgiden en çok {:.3} m ayrılıyor (km {} m, kot {}), tolerans {t} m.",
                worst.0,
                round(worst.1[0]),
                round(worst.1[1])
            ),
            path: Some("tolerance".into()),
        });
    }
    let first = def.pvis.first().map_or(0.0, |p| p.station);
    let last = def.pvis.last().map_or(0.0, |p| p.station);
    let n = 400;
    let line = (0..=n)
        .filter_map(|i| {
            let s = first + (last - first) * f64::from(i) / f64::from(n);
            built.vertical.at(s).map(|z| [s, z.0])
        })
        .collect();
    let out = ProfileFitted {
        id,
        name,
        alignment: alignment.id.clone(),
        source: input.uid.clone(),
        grades: def.pvis.len().saturating_sub(1) as u32,
        curves: built.vertical.curves.len() as u32,
        pvis: def.pvis,
        stations: [first, last],
        deviation: worst.0,
        worst: worst.1,
        revision: doc.revision().to_string(),
    };
    Ok(Checked { list, out, warnings, line })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<ProfileFitted> {
    let Checked { list, mut out, warnings, .. } = c;
    if let Err(r) = cx.doc.set_road_profiles(list, STEP) {
        return bad("", format!("Boykesit kurallarına uymuyor: {}.", r.0)).into();
    }
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesFit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// The profile that would be made; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesFit) -> CommandResult<ProfileFitted> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// Makes the profile: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesFit) -> CommandResult<ProfileFitted> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_PROFILES_FIT,
        kentos_commands::ROAD_PROFILES_FIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}

/// The profile for a window: the answer, its warnings and its line as (station, elevation), or why there is none.
pub type Fitted = (ProfileFitted, Vec<CommandWarning>, Vec<[f64; 2]>);

/// The profile for a window: [`Fitted`], or why there is none.
pub fn preview(doc: &Document, input: &ProfilesFit) -> Result<Fitted, String> {
    check(doc, input).map(|c| (c.out, c.warnings, c.line)).map_err(crate::road_corridors::words)
}
