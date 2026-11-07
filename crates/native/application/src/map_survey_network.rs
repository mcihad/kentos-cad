//! `map.survey.network` v1 (plan/map-cad.md MAP-03, MAP-05; docs/adr/1740): a map project's GNSS network adjusted from
//! its archive, reading only. The baselines that go into the adjustment and the known points on their marks make the
//! network. The shared core (`survey::adjust::gnss`) adjusts it twice: free, held by one known point alone, for the
//! blunders and the network's own precision (BÖHHBÜY Madde 15(1)(b)); then held by all its known points, fixed or
//! weighted, for the marks' coordinates (15(1)(ç)). The marks that are not known points are moved into the project by
//! the coordinate chain and compared with their accepted points. A rule class (KentOS's package: BÖHHBÜY 2018's C1,
//! C2, C3) checks the baselines' and the stations' standard deviations at its confidence. `map.survey.edit`'s
//! `network` writes what this reads. The shared cases are `fixtures/commands/v1/map.survey.network.json`.

use std::collections::HashSet;

use kentos_commands::{
    BaselineSigmaOf, BaselineTest, CommandError, CommandResult, CommandWarning, GnssRuleClass, GnssRules, MapSurvey,
    MapSurveyNetwork, MapSurveyNetworked, NetworkBaseline, NetworkRules, NetworkStage, NetworkStation, SurveyBaseline,
    SurveyKnown, gnss_rules_default,
};
use kentos_domain::Document;
use kentos_geometry_core::survey::adjust::gnss::{self, Baseline, Datum, GnssInput, GnssResult, GnssStation};
use kentos_geometry_core::survey::adjust::{W_LIMIT, chi2_quantile};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::map_survey::{Placed, baseline_length, fail, map_project, place};

/// A mark that is not a known point, as the stage held by the known points leaves it: its latitude, longitude
/// (degrees), ellipsoidal height (m), standard deviations north, east, up (m), and its place in the project or why the
/// chain did not reach it.
pub(crate) struct NewMark {
    pub mark: String,
    pub lat: f64,
    pub lon: f64,
    pub height: f64,
    pub sigma: [f64; 3],
    pub placed: Result<Placed, CommandError>,
}

/// What the adjustment read and said, for the answer and for the write: the baselines and known points it rests on,
/// the one the free stage held, the variance factor, the class and its package, and the new marks.
pub(crate) struct Analysis {
    pub out: MapSurveyNetworked,
    pub baselines: Vec<String>,
    pub known: Vec<String>,
    pub hold: String,
    pub scale: f64,
    pub class: Option<(String, String)>,
    pub new: Vec<NewMark>,
}

const RAD: f64 = 180.0 / std::f64::consts::PI;

/// The ellipsoid (semi-major axis, inverse flattening) of a geographic frame of the registry.
fn ellipsoid_of(srid: u32) -> Option<(f64, f64)> {
    let system = kentos_project::crs::system(srid).filter(|s| s.kind == "geographic")?;
    match system.transform_system()? {
        kentos_geometry_core::crs::System::Geographic { datum } => Some(datum.ellipsoid()),
        _ => None,
    }
}

/// Why a rule package's numbers cannot be used.
fn rules_problem(r: &GnssRules) -> Option<&'static str> {
    if r.id.trim().is_empty() || r.source.trim().is_empty() || r.classes.is_empty() {
        return Some("Kural paketinin kimliği, kaynağı ve en az bir sınıfı olmalı.");
    }
    let mut ids = HashSet::new();
    for c in &r.classes {
        if c.id.trim().is_empty() || !ids.insert(c.id.as_str()) {
            return Some("Kural paketinin sınıflarının kimlikleri boş olmayan ve tek olmalı.");
        }
        let limit = [c.constant, c.ppm];
        if !limit.iter().all(|v| v.is_finite() && *v >= 0.0) || limit.iter().all(|v| *v == 0.0) {
            return Some("Sınıfın baz sınırı eksi olmayan sonlu iki sayıdır (m ve ppm), ikisi birden sıfır olamaz.");
        }
        if !(c.horizontal.is_finite() && c.horizontal > 0.0 && c.height.is_finite() && c.height > 0.0) {
            return Some("Sınıfın nokta sınırları 0'dan büyük sonlu sayılar olmalı (m).");
        }
        if c.confidence.is_some_and(|p| !(p.is_finite() && p > 0.0 && p < 1.0)) {
            return Some("Sınıfın güveni 0 ile 1 arasında olmalı.");
        }
    }
    None
}

/// The test value above which a component is a blunder: two-sided at the class's confidence (√χ²₁), else Baarda's.
fn w_limit(class: Option<&GnssRuleClass>) -> f64 {
    match class.and_then(|c| c.confidence) {
        Some(p) => chi2_quantile(1.0, p).sqrt(),
        None => W_LIMIT,
    }
}

/// A refusal of what the archive holds (no input field is wrong): without a path in the query, the operation's in an
/// edit (`prefix` “operation.”).
fn held_wrong(code: &str, message: String, prefix: &str) -> Stop {
    if prefix.is_empty() {
        Stop::failed(crate::checks::error(code, message, None))
    } else {
        fail(code, message, "operation")
    }
}

/// A known point's X, Y, Z (m) on its frame's ellipsoid.
fn ecef(k: &SurveyKnown, (a, inv_f): (f64, f64)) -> [f64; 3] {
    gnss::to_ecef(a, inv_f, k.lat / RAD, k.lon / RAD, k.height)
}

/// A weighted known point's north, east, up deviations as an X, Y, Z covariance: Rᵀ·diag(σe², σn², σu²)·R, R the
/// rows east, north, up at its place.
fn weight(k: &SurveyKnown, sigma: [f64; 3]) -> [f64; 6] {
    let r = gnss::enu_rows(k.lat / RAD, k.lon / RAD);
    let var = [sigma[1] * sigma[1], sigma[0] * sigma[0], sigma[2] * sigma[2]];
    let c = |i: usize, j: usize| (0..3).map(|k| r[k][i] * var[k] * r[k][j]).sum::<f64>();
    [c(0, 0), c(0, 1), c(0, 2), c(1, 1), c(1, 2), c(2, 2)]
}

/// A residual vector (ECEF, m) in the local east, north, up of a place.
fn enu(at: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    let (lat, lon) = gnss::geodetic(at);
    let r = gnss::enu_rows(lat, lon);
    [0, 1, 2].map(|k| r[k][0] * v[0] + r[k][1] * v[1] + r[k][2] * v[2])
}

/// A stage's summary and each baseline's test at `limit`; `froms` is each baseline's start among the stations,
/// `known_ids` names the stations that are known points.
fn stage(
    r: &GnssResult,
    ids: &[String],
    froms: &[usize],
    known_ids: &[Option<String>],
    limit: f64,
    held: Vec<String>,
) -> (NetworkStage, Vec<BaselineTest>) {
    let obs = &r.stats.observations;
    let mut tests = Vec::with_capacity(ids.len());
    let mut worst: Option<(f64, String)> = None;
    let mut blunders = 0u32;
    let mut note = |w: f64, id: &str, worst: &mut Option<(f64, String)>| {
        blunders += 1;
        if worst.as_ref().is_none_or(|(m, _)| w > *m) {
            *worst = Some((w, id.to_owned()));
        }
    };
    for (k, id) in ids.iter().enumerate() {
        let parts = &obs[3 * k..3 * k + 3];
        let w = parts.iter().filter_map(|o| o.w).fold(None, |m: Option<f64>, w| Some(m.map_or(w, |m| if w > m { w } else { m })));
        let flag = if parts.iter().all(|o| o.flag == "uncontrolled") {
            "uncontrolled"
        } else if w.is_some_and(|w| w > limit) {
            "blunder"
        } else {
            "ok"
        };
        if flag == "blunder" {
            note(w.unwrap_or(0.0), id, &mut worst);
        }
        let at = r.stations[froms[k]].xyz;
        tests.push(BaselineTest { v: enu(at, r.baselines[k].v), w, flag: flag.into() });
    }
    // A weighted known point's pseudo-observations: a known point the network does not agree with.
    let mut i = 3 * ids.len();
    while i < obs.len() {
        let row = obs[i].row;
        let mut w: Option<f64> = None;
        while i < obs.len() && obs[i].row == row {
            if let Some(x) = obs[i].w {
                w = Some(w.map_or(x, |m| if x > m { x } else { m }));
            }
            i += 1;
        }
        if let Some(w) = w.filter(|w| *w > limit) {
            let id = known_ids.get(row).cloned().flatten().unwrap_or_default();
            note(w, &id, &mut worst);
        }
    }
    let s = &r.stats;
    (
        NetworkStage {
            held,
            m0: s.m0,
            dof: u32::try_from(s.f).unwrap_or(u32::MAX),
            omega: s.omega,
            chi2: s.chi2,
            passed: s.passed,
            limit,
            blunders,
            worst: worst.map(|(_, id)| id),
        },
        tests,
    )
}

/// The network of `input` adjusted, or why it cannot be. `prefix` is the input's path (“operation.” in an edit).
#[allow(clippy::too_many_lines)]
pub(crate) fn analyse(doc: &Document, input: &MapSurveyNetwork, prefix: &str) -> Result<Analysis, Stop> {
    map_project(doc)?;
    let empty = MapSurvey::default();
    let survey = doc.map_survey().unwrap_or(&empty);
    let path = |f: &str| format!("{prefix}{f}");
    if let Some(c) = &input.campaign
        && survey.campaign(c).is_none()
    {
        return Err(fail(codes::UNKNOWN_SURVEY_RECORD, format!("GNSS arşivinde “{c}” kimlikli arazi ölçümü yok."), &path("campaign")));
    }
    let scale = input.scale.unwrap_or(1.0);
    if !(scale.is_finite() && scale > 0.0) {
        return Err(fail(codes::INVALID_SURVEY_VALUE, "Varyans çarpanı 0'dan büyük sonlu bir sayı olmalı.".into(), &path("scale")));
    }
    let package = match &input.rules {
        Some(r) => r.clone(),
        None => gnss_rules_default().ok_or_else(|| {
            fail(codes::INVALID_SURVEY_VALUE, "KentOS'un GNSS ağı kural paketi okunamadı.".into(), &path("rules"))
        })?,
    };
    if let Some(why) = rules_problem(&package) {
        return Err(fail(codes::INVALID_SURVEY_VALUE, why.into(), &path("rules")));
    }
    let class = match &input.class {
        Some(id) => Some(package.classes.iter().find(|c| c.id == *id).ok_or_else(|| {
            let names: Vec<&str> = package.classes.iter().map(|c| c.id.as_str()).collect();
            fail(
                codes::UNKNOWN_RULE_CLASS,
                format!("“{}” kural paketinde “{id}” sınıfı yok; sınıflar: {}.", package.id, names.join(", ")),
                &path("class"),
            )
        })?),
        None => None,
    };
    // The baselines that go in.
    let used: Vec<&SurveyBaseline> = survey
        .baselines
        .iter()
        .filter(|b| b.adjust && b.excluded.is_none())
        .filter(|b| input.campaign.as_ref().is_none_or(|c| b.campaign == *c))
        .collect();
    if used.is_empty() {
        return Err(held_wrong(
            codes::NO_BASELINES,
            "Dengelemeye girecek baz yok: bazları içe aktarın, dışlananları geri alın ya da dengeleme seçimini açın.".into(),
            prefix,
        ));
    }
    let campaign = survey.campaign(&used[0].campaign).cloned();
    for b in &used[1..] {
        let c = survey.campaign(&b.campaign);
        if c.map(|c| (c.epoch, &c.velocity)) != campaign.as_ref().map(|c| (c.epoch, &c.velocity)) {
            return Err(held_wrong(
                codes::SOLUTIONS_DIFFER,
                format!(
                    "“{}” ile “{}” bazlarının arazi ölçümlerinin epok kararı ya da hız ızgarası farklı; ağ tek epok kararıyla projeye taşınır.",
                    used[0].id, b.id
                ),
                prefix,
            ));
        }
    }
    // The marks in the order the baselines name them.
    let mut marks: Vec<String> = Vec::new();
    for b in &used {
        for m in [&b.from, &b.to] {
            if !marks.iter().any(|x| x == m) {
                marks.push(m.clone());
            }
        }
    }
    let known: Vec<&SurveyKnown> =
        survey.known.iter().filter(|k| k.excluded.is_none() && marks.contains(&k.mark)).collect();
    if known.is_empty() {
        return Err(held_wrong(
            codes::DATUM_MISSING,
            "Ağın bilinen noktası yok: bazların uçlarından en az birine bilinen nokta (TUSAGA-Aktif, C1 …) ekleyin.".into(),
            prefix,
        ));
    }
    let (srid, epoch) = (known[0].srid, known[0].epoch);
    if let Some(k) = known.iter().find(|k| k.srid != srid || k.epoch != epoch) {
        return Err(held_wrong(
            codes::KNOWN_DIFFER,
            format!(
                "“{}” ile “{}” bilinen noktaları aynı çerçeve ve epokta değil; hepsini ölçme epokunda aynı çerçevede girin.",
                known[0].id, k.id
            ),
            prefix,
        ));
    }
    let Some(shape) = ellipsoid_of(srid) else {
        return Err(held_wrong(
            codes::UNKNOWN_FRAME,
            format!("EPSG:{srid} kayıttaki coğrafi bir sistem değil; bilinen noktaların çerçevesini (4326 WGS 84, 5252 TUREF) verin."),
            prefix,
        ));
    };
    let hold = match &input.hold {
        Some(h) => known.iter().find(|k| k.id == *h).copied().ok_or_else(|| {
            fail(codes::UNKNOWN_SURVEY_RECORD, format!("“{h}” ağın bilinen noktalarından biri değil."), &path("hold"))
        })?,
        None => known[0],
    };
    let known_of = |m: &str| known.iter().find(|k| k.mark == m).copied();
    let baselines: Vec<Baseline> =
        used.iter().map(|b| Baseline { from: b.from.clone(), to: b.to.clone(), d: b.d, cov: b.covariance }).collect();
    let free_stations: Vec<GnssStation> = marks
        .iter()
        .map(|m| match known_of(m) {
            Some(k) => GnssStation { name: m.clone(), xyz: Some(ecef(k, shape)), fixed: k.id == hold.id, sigma: None, cov: None },
            None => GnssStation { name: m.clone(), xyz: None, fixed: false, sigma: None, cov: None },
        })
        .collect();
    let held_stations: Vec<GnssStation> = marks
        .iter()
        .map(|m| match known_of(m) {
            Some(k) => GnssStation {
                name: m.clone(),
                xyz: Some(ecef(k, shape)),
                fixed: k.sigma.is_none(),
                sigma: None,
                cov: k.sigma.map(|s| weight(k, s)),
            },
            None => GnssStation { name: m.clone(), xyz: None, fixed: false, sigma: None, cov: None },
        })
        .collect();
    let refused = |r: gnss::Refused| held_wrong(r.code, r.message, prefix);
    let free = gnss::adjust(&GnssInput { stations: free_stations, baselines: baselines.clone(), scale, datum: Datum::Given })
        .map_err(refused)?;
    let held = gnss::adjust(&GnssInput { stations: held_stations, baselines, scale, datum: Datum::Given }).map_err(refused)?;
    let limit = w_limit(class);
    let ids: Vec<String> = used.iter().map(|b| b.id.clone()).collect();
    let known_ids: Vec<Option<String>> = marks.iter().map(|m| known_of(m).map(|k| k.id.clone())).collect();
    let froms: Vec<usize> = used.iter().map(|b| marks.iter().position(|m| *m == b.from).unwrap_or(0)).collect();
    let (free_stage, free_tests) = stage(&free, &ids, &froms, &known_ids, limit, vec![hold.mark.clone()]);
    let fixed: Vec<String> = known.iter().filter(|k| k.sigma.is_none()).map(|k| k.mark.clone()).collect();
    let (held_stage, held_tests) = stage(&held, &ids, &froms, &known_ids, limit, fixed);
    let mut failed_baselines = 0u32;
    let rows: Vec<NetworkBaseline> = used
        .iter()
        .enumerate()
        .map(|(k, b)| {
            let length = baseline_length(b);
            let c = b.covariance;
            let sigma = [(c[0] * scale).sqrt(), (c[3] * scale).sqrt(), (c[5] * scale).sqrt()];
            let adjusted = free.baselines[k].adjusted;
            let limit = class.map(|c| c.constant + c.ppm * 1e-6 * length);
            let rules = class.zip(limit).map(|(c, l)| {
                let of = if c.baseline_of == BaselineSigmaOf::Adjusted { adjusted } else { sigma };
                of.iter().all(|v| *v <= l)
            });
            if rules == Some(false) {
                failed_baselines += 1;
            }
            NetworkBaseline {
                id: b.id.clone(),
                from: b.from.clone(),
                to: b.to.clone(),
                length,
                sigma,
                adjusted,
                free: free_tests[k].clone(),
                constrained: held_tests[k].clone(),
                limit,
                rules,
            }
        })
        .collect();
    let mut failed_stations = 0u32;
    let mut stations = Vec::with_capacity(marks.len());
    let mut new = Vec::new();
    for (i, m) in marks.iter().enumerate() {
        let s = &held.stations[i];
        let (lat, lon, h) = gnss::to_geodetic(shape.0, shape.1, s.xyz);
        let (lat, lon) = (lat * RAD, lon * RAD);
        let sigma = [s.senu[1], s.senu[0], s.senu[2]];
        let k = known_of(m);
        let point = survey.points.iter().find(|p| p.mark == *m);
        let mut row = NetworkStation {
            mark: m.clone(),
            known: k.map(|k| k.id.clone()),
            held: s.held,
            lat,
            lon,
            height: h,
            sigma,
            point: point.map(|p| p.id.clone()),
            x: None,
            y: None,
            z: None,
            dx: None,
            dy: None,
            dz: None,
            rules: None,
        };
        if k.is_none() {
            let placed = place(doc, campaign.as_ref(), (lat, lon, h), srid, epoch, "operation").map_err(Stop::into_error);
            if let Ok(p) = &placed {
                row.x = Some(p.x);
                row.y = Some(p.y);
                row.z = p.z;
                if let Some(pt) = point {
                    row.dx = Some(p.x - pt.result.x);
                    row.dy = Some(p.y - pt.result.y);
                    row.dz = p.z.zip(pt.result.z).map(|(a, b)| a - b);
                }
            }
            if let Some(c) = class {
                let ok = sigma[0] <= c.horizontal && sigma[1] <= c.horizontal && sigma[2] <= c.height;
                row.rules = Some(ok);
                if !ok {
                    failed_stations += 1;
                }
            }
            new.push(NewMark { mark: m.clone(), lat, lon, height: h, sigma, placed });
        }
        stations.push(row);
    }
    let rules = class.map(|c| NetworkRules {
        package: package.id.clone(),
        release: package.release,
        class: c.id.clone(),
        articles: c.articles.clone(),
        confidence: c.confidence,
        baseline_of: c.baseline_of,
        passed: failed_baselines == 0 && failed_stations == 0,
        baselines: failed_baselines,
        stations: failed_stations,
    });
    // The same network already accepted: the same baselines, known points, held point, factor and class, and every
    // new mark's point carrying that adjustment now.
    let class_of = class.map(|c| (c.id.clone(), format!("{}@{}", package.id, package.release)));
    let known_ids_used: Vec<String> = known.iter().map(|k| k.id.clone()).collect();
    let current = survey
        .adjustments
        .iter()
        .rev()
        .find(|a| {
            a.baselines == ids
                && a.known == known_ids_used
                && a.hold == hold.id
                && a.scale == scale
                && a.class.clone().zip(a.rules.clone()) == class_of
                && !new.is_empty()
                && new.iter().all(|n| {
                    survey.points.iter().any(|p| p.mark == n.mark && p.result.adjustment.as_deref() == Some(a.id.as_str()))
                })
        })
        .map(|a| a.id.clone());
    let mut why = Vec::new();
    if let Some(a) = &current {
        why.push(format!("Bu ağ “{a}” dengelemesiyle kabul edildi; noktaları güncel."));
    }
    for (name, st) in [("Serbest dengelemede", &free_stage), ("Bilinen noktalara dayalı dengelemede", &held_stage)] {
        if st.blunders > 0 {
            why.push(format!(
                "{name} {} uyuşumsuz ölçü var (test değeri {:.2}'yi aşıyor); en kötüsü “{}”.",
                st.blunders,
                st.limit,
                st.worst.clone().unwrap_or_default()
            ));
        }
        if st.passed == Some(false) {
            why.push(format!(
                "{name} model testi geçmedi: vᵀPv = {:.2}, χ² sınırı {:.2} (m0 = {:.2}).",
                st.omega,
                st.chi2.unwrap_or(0.0),
                st.m0.unwrap_or(0.0)
            ));
        }
    }
    for n in &new {
        if let Err(e) = &n.placed {
            why.push(format!("“{}” projeye taşınamadı: {}", n.mark, e.message));
        }
    }
    if new.is_empty() {
        why.push("Ağda bilinen nokta olmayan işaret yok; dengelenecek nokta yok.".into());
    }
    let out = MapSurveyNetworked {
        srid,
        epoch,
        known: known.iter().map(|k| k.id.clone()).collect(),
        baselines: rows,
        stations,
        free: free_stage,
        constrained: held_stage,
        rules,
        current,
        acceptable: why.is_empty(),
        why,
        revision: doc.revision().to_string(),
    };
    Ok(Analysis {
        out,
        baselines: ids,
        known: known.iter().map(|k| k.id.clone()).collect(),
        hold: hold.id.clone(),
        scale,
        class: class.map(|c| (c.id.clone(), format!("{}@{}", package.id, package.release))),
        new,
    })
}

/// The answer's warnings: a class that does not hold, a network without degrees of freedom; `prefix` is the input's
/// path (“operation.” in an edit).
pub(crate) fn warnings_of(out: &MapSurveyNetworked, prefix: &str) -> Vec<CommandWarning> {
    let mut w = Vec::new();
    if let Some(r) = out.rules.as_ref().filter(|r| !r.passed) {
        w.push(CommandWarning {
            code: codes::NETWORK_RULES_FAILED.into(),
            message: format!(
                "{} sınıfı ({} {}) tutmuyor: {} baz ve {} nokta sınırı aşıyor.",
                r.class, r.package, r.articles, r.baselines, r.stations
            ),
            path: Some(format!("{prefix}class")),
        });
    }
    if out.free.dof == 0 {
        w.push(CommandWarning {
            code: codes::NETWORK_UNCHECKED.into(),
            message: "Ağda fazla ölçü yok; uyuşumsuz baz ve model sınanamaz.".into(),
            path: None,
        });
    }
    w
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &MapSurveyNetwork) -> CommandResult<()> {
    match analyse(cx.doc, input, "") {
        Ok(a) => CommandResult::Completed { output: (), warnings: warnings_of(&a.out, "") },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &MapSurveyNetwork) -> CommandResult<MapSurveyNetworked> {
    match analyse(cx.doc, input, "") {
        Ok(a) => {
            let warnings = warnings_of(&a.out, "");
            CommandResult::Completed { output: a.out, warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: MapSurveyNetwork) -> CommandResult<MapSurveyNetworked> {
    plan(cx, &input)
}

/// The network for a window, or why it does not adjust.
pub fn preview(doc: &Document, input: &MapSurveyNetwork) -> Result<MapSurveyNetworked, String> {
    analyse(doc, input, "").map(|a| a.out).map_err(|stop| stop.into_error().message)
}
