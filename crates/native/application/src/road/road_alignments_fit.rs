//! `road.alignments.fit` v1 (plan/road.md Y10.6, docs/adr/0908 §12; ROAD-IO-14): an alignment made of a line or an open
//! polyline of the drawing (`kentos_road::alignment::trace`): element by element (Civil 3D's Create Alignment from
//! Objects) or by best fit (Create Best Fit Alignment). The line is kept; the alignment's deviation from its vertices
//! (and its arcs' middles) is measured on the built alignment (`Alignment::locate`). One undo step “Çizgiden aks”. The
//! shared cases are `fixtures/commands/v1/road.alignments.fit.json`.

use kentos_commands::{
    AlignmentDef, AlignmentFitMethod, AlignmentFitted, AlignmentPi, AlignmentStationing, AlignmentsFit, CommandResult,
    CommandWarning, Entity,
};
use kentos_domain::Document;
use kentos_road::alignment::trace::{TraceError, best_fit, from_elements};
use kentos_road::alignment::{Kind, PiInput};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments::{self as alignments, Built};
use crate::road_terrains::fail;

/// The undo step's name.
pub const STEP: &str = "Çizgiden aks";

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_ALIGNMENT_FIT, words, path)
}

fn num(v: f64) -> String {
    format!("{v}")
}

struct Checked {
    list: Vec<AlignmentDef>,
    out: AlignmentFitted,
    warnings: Vec<CommandWarning>,
    built: Built,
}

/// The line's points and bulges, in the order asked.
fn line_of(doc: &Document, uid: &str, reverse: bool) -> Result<(Vec<[f64; 2]>, Vec<f64>), Stop> {
    let entity = doc.entities_with_uids().find(|(u, _)| u.to_string() == uid).map(|(_, e)| e);
    let (mut pts, mut bulges): (Vec<[f64; 2]>, Vec<f64>) = match entity {
        None => return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), "uid")),
        Some(Entity::Line(l)) => (vec![[l.a.x, l.a.y], [l.b.x, l.b.y]], vec![0.0]),
        Some(Entity::Polyline(p)) if p.closed => {
            return Err(bad("uid", format!("“{uid}” kapalı bir çoklu çizgi; aks açık bir çizgiden kurulur.")));
        }
        Some(Entity::Polyline(p)) if p.parts.as_ref().is_some_and(|q| !q.is_empty()) => {
            return Err(bad("uid", format!("“{uid}” çok parçalı bir çoklu çizgi; aks tek parçalı bir çizgiden kurulur.")));
        }
        Some(Entity::Polyline(p)) => {
            let pts: Vec<[f64; 2]> = p.pts.iter().map(|v| [v.x, v.y]).collect();
            let n = pts.len().saturating_sub(1);
            let b = p.bulges.clone().unwrap_or_default();
            (pts, (0..n).map(|i| b.get(i).copied().unwrap_or(0.0)).collect())
        }
        Some(_) => return Err(bad("uid", format!("“{uid}” bir çizgi ya da çoklu çizgi değil."))),
    };
    if reverse {
        pts.reverse();
        bulges.reverse();
        for b in &mut bulges {
            *b = -*b;
        }
    }
    Ok((pts, bulges))
}

/// The points the deviation is measured at: every vertex and every arc's middle.
fn probes(pts: &[[f64; 2]], bulges: &[f64]) -> Vec<[f64; 2]> {
    let mut out = pts.to_vec();
    for (i, w) in pts.windows(2).enumerate() {
        let b = bulges.get(i).copied().unwrap_or(0.0);
        if b != 0.0 {
            // The arc's middle: its sagitta, `b` times half the chord, off the chord's middle to the right of a left
            // turn (the arc bulges away from its centre), to the left of a right one.
            let (a, c) = (w[0], w[1]);
            let (dx, dy) = (c[0] - a[0], c[1] - a[1]);
            let s = 0.5 * b;
            out.push([0.5 * (a[0] + c[0]) + s * dy, 0.5 * (a[1] + c[1]) - s * dx]);
        }
    }
    out
}

/// The largest distance of `points` from the built alignment, and where.
fn deviation(b: &Built, points: &[[f64; 2]]) -> (f64, [f64; 2]) {
    let mut worst = (0.0, points.first().copied().unwrap_or([0.0, 0.0]));
    for p in points {
        let s = b.core.locate(*p);
        let d = match (&s.nearest, &s.beyond) {
            (Some(f), _) => f.offset.abs(),
            (None, Some((_, f))) => (p[0] - f.at.x).hypot(p[1] - f.at.y),
            _ => 0.0,
        };
        if d > worst.0 {
            worst = (d, *p);
        }
    }
    worst
}

fn trace_words(e: TraceError) -> String {
    use kentos_road::alignment::fit::FitError;
    match e {
        TraceError::TooFew => "çizginin iki ayrı noktası yok".into(),
        TraceError::NoTangent => "her köşe en büyük yarıçaptan dar bir kurpta: hiç aliyman yok; en büyük yarıçapı küçültün".into(),
        TraceError::Parallel { run } => format!("{}. ve {}. aliyman paralel, kesişmiyor", run, run + 1),
        TraceError::Fit(FitError::Compound { element }) => {
            format!("{}. parçada aralarında aliyman olmayan iki yay var (bileşik ya da ters kurp)", element + 1)
        }
        TraceError::Fit(FitError::NoMeet { element }) => format!("{}. parçanın giriş ve çıkış teğetleri kesişmiyor", element + 1),
        TraceError::Fit(FitError::Degenerate { element }) => format!("{}. parçanın boyu yok", element + 1),
        TraceError::Fit(FitError::Spiral { element } | FitError::Unplaced { element }) => {
            format!("{}. parça KentOS'un PI'larına çevrilemiyor", element + 1)
        }
        TraceError::Fit(FitError::Empty) => "çizginin parçası yok".into(),
    }
}

fn check(doc: &Document, input: &AlignmentsFit) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    alignments::road_project(doc)?;
    let positive = |v: Option<f64>, field: &str, what: &str| match v {
        Some(r) if !(r.is_finite() && r > 0.0) => Err(bad(field, format!("{what} 0'dan büyük olmalı."))),
        _ => Ok(()),
    };
    positive(input.radius, "radius", "Teğetler arasındaki kurbun yarıçapı (radius)")?;
    positive(input.max_radius, "maxRadius", "En büyük yarıçap (maxRadius)")?;
    positive(input.tolerance, "tolerance", "Tolerans (tolerance)")?;
    let (pts, bulges) = line_of(doc, &input.uid, input.reverse)?;
    let pis: Vec<PiInput> = match input.method {
        AlignmentFitMethod::Elements => from_elements(&pts, &bulges, input.radius.unwrap_or(0.0)),
        AlignmentFitMethod::BestFit => {
            let Some(most) = input.max_radius else {
                return Err(bad("maxRadius", "En iyi uyumda kurpları ayıran en büyük yarıçap (maxRadius) verilmeli.".into()));
            };
            best_fit(&pts, most)
        }
    }
    .map_err(|e| bad("uid", format!("Çizgiden aks kurulamıyor: {}.", trace_words(e))))?;
    let mut list = alignments::alignments(doc).to_vec();
    let id = (1..).map(|n| format!("g-{n}")).find(|i| list.iter().all(|d| &d.id != i)).unwrap_or_default();
    let name = match input.name.as_deref().map(str::trim) {
        Some(n) if !n.is_empty() => n.to_owned(),
        _ => (1..).map(|k| format!("{k}. aks")).find(|n| list.iter().all(|d| d.name.to_lowercase() != n.to_lowercase())).unwrap_or_default(),
    };
    if list.iter().any(|d| d.name.to_lowercase() == name.to_lowercase()) {
        return Err(bad("name", format!("Projede “{name}” adlı bir aks var.")));
    }
    let start = input.start.unwrap_or(0.0);
    if !start.is_finite() {
        return Err(bad("start", "Başlangıç km'si sonlu bir sayı olmalı.".into()));
    }
    let def = AlignmentDef {
        id: id.clone(),
        name: name.clone(),
        revision: 1,
        pis: pis
            .iter()
            .map(|p| AlignmentPi { id: p.id.clone(), x: p.x, y: p.y, radius: p.radius, spiral_in: 0.0, spiral_out: 0.0, ove: false, speed: None })
            .collect(),
        offset_of: None,
        stationing: (start != 0.0).then(|| AlignmentStationing { start, equations: Vec::new() }),
        design_speed: None,
        start_place: None,
        end_place: None,
    };
    list.push(def.clone());
    let built = alignments::build(&list, &def, "").map_err(|stop| {
        bad("uid", format!("Çizgiden aks kurulamıyor: {}", crate::road_corridors::words(stop)))
    })?;
    let (dev, worst) = deviation(&built, &probes(&pts, &bulges));
    let (mut tangents, mut curves) = (0u32, 0u32);
    for e in &built.core.elements {
        match e.kind {
            Kind::Tangent => tangents += 1,
            Kind::Arc => curves += 1,
            Kind::Spiral => {}
        }
    }
    let mut warnings = Vec::new();
    if let Some(t) = input.tolerance
        && dev > t
    {
        warnings.push(CommandWarning {
            code: codes::ALIGNMENT_FIT_DEVIATION.into(),
            message: format!(
                "“{name}” aksı çizgiden en çok {dev:.3} m ayrılıyor ({}; {}), tolerans {} m.",
                num((worst[0] * 1000.0).round() / 1000.0),
                num((worst[1] * 1000.0).round() / 1000.0),
                num(t)
            ),
            path: Some("tolerance".into()),
        });
    }
    let out = AlignmentFitted {
        id,
        name,
        source: input.uid.clone(),
        method: input.method,
        pis: def.pis.clone(),
        tangents,
        curves,
        length: built.core.length,
        deviation: dev,
        worst,
        revision: doc.revision().to_string(),
    };
    Ok(Checked { list, out, warnings, built })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<AlignmentFitted> {
    let Checked { list, mut out, warnings, .. } = c;
    if let Err(r) = cx.doc.set_road_alignments(list, STEP) {
        return bad("", format!("Aks kurallarına uymuyor: {}.", r.0)).into();
    }
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AlignmentsFit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// The alignment that would be made; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &AlignmentsFit) -> CommandResult<AlignmentFitted> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// Makes the alignment: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AlignmentsFit) -> CommandResult<AlignmentFitted> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_ALIGNMENTS_FIT,
        kentos_commands::ROAD_ALIGNMENTS_FIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}

/// The alignment for a window: the answer, its warnings and its line sampled at `n` + 1 points, or why there is none.
pub type Fitted = (AlignmentFitted, Vec<CommandWarning>, Vec<[f64; 2]>);

/// The alignment for a window: [`Fitted`], or why there is none.
pub fn preview(doc: &Document, input: &AlignmentsFit, n: usize) -> Result<Fitted, String> {
    check(doc, input)
        .map(|c| {
            let core = &c.built.core;
            let n = n.max(1);
            let line = (0..=n).filter_map(|i| core.at(core.length * i as f64 / n as f64)).map(|p| [p.x, p.y]).collect();
            (c.out, c.warnings, line)
        })
        .map_err(crate::road_corridors::words)
}
