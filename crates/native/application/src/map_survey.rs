//! KentOS Harita's GNSS campaign archive (plan/map-cad.md MAP-03, docs/adr/1340): how sessions, solutions, accepted
//! points and uses stand (last measured, accepted, in the adjustment, excluded; out of date and why), the replay of a
//! recorded session, the computing of an accepted point (the mark's height by the antenna, the mean, the coordinate
//! chain into the project), and `map.survey.query`'s answer. Reads only; `map_survey_edit.rs` writes. The shared cases
//! are `fixtures/commands/v1/map.survey.*.json`.

use std::collections::HashMap;
use std::path::Path;

use kentos_commands::{
    AcceptMethod, AntennaUsed, ChainStatus, CommandResult, CrsPoint, CrsSide, CrsTransform, FixCount, HeightKind,
    HeightReference, MapSurvey, MapSurveyQuery, MapSurveyTree, SurveyBaseline, SurveyBaselineRow, SurveyCampaign,
    SurveyCampaignRow, SurveyFileRow, SurveyFileState as F, SurveyMarkRow, SurveyObject, SurveyPoint, SurveyPointRow,
    SurveyReplay, SurveyResult, SurveySession, SurveySessionRow, SurveySolution, SurveySolutionRow, SurveyState as S,
    SurveyUse, SurveyUseRow, VelocityModel, survey_seconds, survey_time,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// Refuses a drawing that is not a map project's.
pub(crate) fn map_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work != Some(kentos_commands::CadWork::Map) {
        return Err(Stop::failed(error(
            codes::NOT_A_MAP_PROJECT,
            "GNSS arşivi yalnız harita projesinde (KentOS Harita) olur; bu çizim harita projesi değil.".into(),
            None,
        )));
    }
    Ok(())
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A length in sentences: metres to the millimetre.
pub(crate) fn m3(v: f64) -> String {
    format!("{v:.3}")
}

/// An epoch in sentences, or “yok”.
pub(crate) fn epoch_text(e: Option<f64>) -> String {
    e.map_or_else(|| "yok".to_owned(), |e| format!("{e:.4}"))
}

/// The session's vertical antenna height now.
pub(crate) fn vertical(s: &SurveySession) -> Option<f64> {
    s.antenna.as_ref().map(kentos_commands::SurveyAntenna::vertical)
}

/// The campaign a solution's session belongs to.
pub(crate) fn campaign_of<'a>(survey: &'a MapSurvey, solution: &SurveySolution) -> Option<&'a SurveyCampaign> {
    survey.session(&solution.session).and_then(|s| survey.campaign(&s.campaign))
}

/// The epoch decision a campaign's points are given in: its own, else the project's.
pub(crate) fn decision(doc: &Document, campaign: Option<&SurveyCampaign>) -> Option<f64> {
    campaign.and_then(|c| c.epoch).or(doc.settings().coordinate_epoch)
}

/// Whether an object with this persistent id is in the drawing.
pub(crate) fn present(doc: &Document, uid: &kentos_commands::EntityId) -> bool {
    doc.slot_of(Uuid::from_bytes(uid.0)).is_some()
}

/// The campaign a result's epoch decision is of: its first solution's, or its network adjustment's first baseline's.
pub(crate) fn campaign_of_result<'a>(survey: &'a MapSurvey, r: &SurveyResult) -> Option<&'a SurveyCampaign> {
    match r.adjustment.as_deref() {
        Some(a) => survey
            .adjustment(a)
            .and_then(|a| a.baselines.first())
            .and_then(|b| survey.baseline(b))
            .and_then(|b| survey.campaign(&b.campaign)),
        None => r.solutions.first().and_then(|s| survey.solution(s)).and_then(|s| campaign_of(survey, s)),
    }
}

/// Why a network adjustment's result is out of date: the adjustment gone, a baseline of it excluded, out of the
/// adjustment or gone, a known point of it excluded or gone (docs/adr/1740).
fn network_why(survey: &MapSurvey, adjustment: &str) -> Vec<String> {
    let Some(a) = survey.adjustment(adjustment) else {
        return vec![format!("“{adjustment}” ağ dengelemesi arşivde yok.")];
    };
    let mut why = Vec::new();
    for id in &a.baselines {
        match survey.baseline(id) {
            None => why.push(format!("“{id}” bazı arşivde yok.")),
            Some(b) => {
                if let Some(reason) = &b.excluded {
                    why.push(format!("“{id}” bazı dışlandı: {reason}"));
                } else if !b.adjust {
                    why.push(format!("“{id}” bazı dengelemeden çıkarıldı."));
                }
            }
        }
    }
    for id in &a.known {
        match survey.known_point(id) {
            None => why.push(format!("“{id}” bilinen noktası arşivde yok.")),
            Some(k) => {
                if let Some(reason) = &k.excluded {
                    why.push(format!("“{id}” bilinen noktası dışlandı: {reason}"));
                }
            }
        }
    }
    why
}

/// Why an accepted point is out of date now: a session's antenna, the epoch decision, an excluded solution; for a
/// network's point, what its adjustment rests on.
pub(crate) fn point_why(doc: &Document, survey: &MapSurvey, p: &SurveyPoint) -> Vec<String> {
    let mut why = Vec::new();
    if let Some(a) = p.result.adjustment.as_deref() {
        why.extend(network_why(survey, a));
    }
    for used in &p.result.antenna {
        let now = survey.session(&used.session).and_then(vertical);
        if now != Some(used.height) {
            match now {
                Some(h) => why.push(format!(
                    "“{}” oturumunun anten yüksekliği {} m iken {} m oldu.",
                    used.session,
                    m3(used.height),
                    m3(h)
                )),
                None => why.push(format!("“{}” oturumunun anteni yok.", used.session)),
            }
        }
    }
    let campaign = campaign_of_result(survey, &p.result);
    let now = decision(doc, campaign);
    if now != p.result.target_epoch {
        why.push(format!("Epok kararı {} iken {} oldu.", epoch_text(p.result.target_epoch), epoch_text(now)));
    }
    for s in &p.result.solutions {
        if let Some(reason) = survey.solution(s).and_then(|s| s.excluded.as_ref()) {
            why.push(format!("“{s}” çözümü dışlandı: {reason}"));
        }
    }
    why
}

/// An accepted point's row: its state and why.
pub(crate) fn point_row(doc: &Document, survey: &MapSurvey, p: &SurveyPoint) -> SurveyPointRow {
    let mut why = point_why(doc, survey, p);
    let stale = !why.is_empty();
    let gone = !present(doc, &p.uid);
    if gone {
        why.push("Noktanın nesnesi çizimde yok.".into());
    }
    SurveyPointRow {
        id: p.id.clone(),
        mark: p.mark.clone(),
        uid: p.uid.to_text(),
        state: if stale {
            S::Stale
        } else if gone {
            S::Missing
        } else {
            S::Current
        },
        why,
        result: p.result.clone(),
        history: count(p.history.len()),
    }
}

/// A use's row: its points that moved on, its objects gone.
pub(crate) fn use_row(doc: &Document, survey: &MapSurvey, u: &SurveyUse) -> SurveyUseRow {
    let mut why = Vec::new();
    for pr in &u.points {
        if let Some(p) = survey.point(&pr.point)
            && p.result.revision != pr.revision
        {
            why.push(format!(
                "“{}” noktası {}. revizyondan {}. revizyona geçti.",
                pr.point, pr.revision, p.result.revision
            ));
        }
    }
    let stale = !why.is_empty();
    let held = u.uids.iter().filter(|x| present(doc, x)).count();
    let gone = u.uids.len() - held;
    if gone > 0 {
        why.push(format!("{gone} nesnesi çizimde yok."));
    }
    SurveyUseRow {
        id: u.id.clone(),
        operation: u.operation.clone(),
        objects: count(held),
        recorded: count(u.uids.len()),
        state: if stale {
            S::Stale
        } else if gone > 0 {
            S::Missing
        } else {
            S::Current
        },
        why,
    }
}

/// How many accepted points and uses are out of date.
pub(crate) fn stale_count(doc: &Document, survey: &MapSurvey) -> u32 {
    let points = survey.points.iter().filter(|p| !point_why(doc, survey, p).is_empty()).count();
    let uses = survey.uses.iter().filter(|u| use_row(doc, survey, u).state == S::Stale).count();
    count(points + uses)
}

/// The replay of a session's positions (docs/adr/1340 §3).
pub(crate) fn replay(s: &SurveySession) -> Option<SurveyReplay> {
    if s.observations.is_empty() {
        return None;
    }
    let mut fixes: Vec<FixCount> = Vec::new();
    for o in &s.observations {
        let fix = o.fix.clone().unwrap_or_else(|| "—".into());
        match fixes.iter_mut().find(|f| f.fix == fix) {
            Some(f) => f.count += 1,
            None => fixes.push(FixCount { fix, count: 1 }),
        }
    }
    let fixed = |o: &kentos_commands::GnssPoint| o.fix.as_deref() == Some("RTK sabit");
    let float_drops = s.observations.windows(2).filter(|w| fixed(&w[0]) && !fixed(&w[1])).count();
    let timed: Vec<f64> = s.observations.iter().filter_map(|o| o.time.as_deref().and_then(survey_seconds)).collect();
    let steps: Vec<i64> = timed.windows(2).map(|w| ((w[1] - w[0]) * 1000.0 + 0.5).floor() as i64).collect();
    let mut tally: Vec<(i64, usize)> = Vec::new();
    for &st in steps.iter().filter(|&&st| st > 0) {
        match tally.iter_mut().find(|t| t.0 == st) {
            Some(t) => t.1 += 1,
            None => tally.push((st, 1)),
        }
    }
    let nominal = tally.iter().copied().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|t| t.0);
    let time_jumps = steps.iter().filter(|&&st| st <= 0 || nominal.is_some_and(|n| st > 2 * n)).count();
    Some(SurveyReplay {
        fixes,
        float_drops: count(float_drops),
        time_jumps: count(time_jumps),
        nominal: nominal.map(|n| n as f64 / 1000.0),
    })
}

/// When a solution was measured: its session's end, else its start (sortable), none when neither is given.
fn measured(survey: &MapSurvey, s: &SurveySolution) -> Option<(u32, u32, u32, u32, u32, u32, String)> {
    let session = survey.session(&s.session)?;
    session.end.as_deref().or(session.start.as_deref()).and_then(survey_time)
}

fn number_of(id: &str) -> u64 {
    id.split_once('-').and_then(|(_, n)| n.parse().ok()).unwrap_or(0)
}

/// A solution's row for its mark.
pub(crate) fn solution_row(survey: &MapSurvey, s: &SurveySolution, latest: Option<&str>) -> SurveySolutionRow {
    let session = survey.session(&s.session);
    SurveySolutionRow {
        id: s.id.clone(),
        session: s.session.clone(),
        kind: s.kind,
        lat: s.lat,
        lon: s.lon,
        height: s.height,
        srid: s.srid,
        epoch: s.epoch,
        antenna_applied: s.antenna_applied,
        mark_height: session.and_then(vertical).map(|h| s.height + s.antenna_applied - h),
        fix: s.fix.clone(),
        epochs: s.epochs,
        sigma: s.sigma,
        latest: latest == Some(s.id.as_str()),
        accepted: survey.point_using(&s.id).is_some(),
        adjusted: s.adjust && s.excluded.is_none(),
        excluded: s.excluded.clone(),
    }
}

/// The last measured solution of each mark.
fn latest_of(survey: &MapSurvey) -> HashMap<String, String> {
    let mut best: HashMap<String, (&SurveySolution, (u32, u32, u32, u32, u32, u32, String))> = HashMap::new();
    for s in &survey.solutions {
        let (Some(mark), Some(when)) = (survey.mark_of(s), measured(survey, s)) else { continue };
        let take = match best.get(mark) {
            None => true,
            Some((b, bw)) => when > *bw || (when == *bw && number_of(&s.id) > number_of(&b.id)),
        };
        if take {
            best.insert(mark.to_owned(), (s, when));
        }
    }
    best.into_iter().map(|(m, (s, _))| (m, s.id.clone())).collect()
}

/// The rows of the archive as they stand now.
pub(crate) fn tree(doc: &Document, deep: bool, uid: Option<&str>, places: bool) -> Result<MapSurveyTree, Stop> {
    map_project(doc)?;
    let target = match uid {
        Some(u) => Some(target(doc, u)?),
        None => None,
    };
    let empty = MapSurvey::default();
    let survey = doc.map_survey().unwrap_or(&empty);
    let work = doc.map_work();
    let campaigns = survey
        .campaigns
        .iter()
        .map(|c| SurveyCampaignRow {
            id: c.id.clone(),
            name: c.name.clone(),
            work: c.work.clone(),
            work_missing: c.work.as_deref().is_some_and(|w| work.and_then(|mw| mw.item(w)).is_none()),
            operator: c.operator.clone(),
            epoch: c.epoch,
            velocity: c.velocity.as_ref().map(|g| g.name.clone()),
            sessions: count(survey.sessions.iter().filter(|s| s.campaign == c.id).count()),
        })
        .collect();
    let sessions = survey
        .sessions
        .iter()
        .map(|s| SurveySessionRow {
            id: s.id.clone(),
            campaign: s.campaign.clone(),
            mark: s.mark.clone(),
            start: s.start.clone(),
            end: s.end.clone(),
            time_system: s.time_system,
            receiver: s.receiver.clone(),
            antenna: s.antenna.clone(),
            vertical: vertical(s),
            file: s.file.as_ref().map(|f| SurveyFileRow {
                path: f.path.clone(),
                revision: f.revision.clone(),
                format: f.format,
                state: file_state(&f.path, &f.revision, deep),
            }),
            observations: count(s.observations.len()),
            replay: replay(s),
            solutions: count(survey.solutions.iter().filter(|x| x.session == s.id).count()),
        })
        .collect();
    let latest = latest_of(survey);
    let mut marks: Vec<SurveyMarkRow> = Vec::new();
    for s in &survey.sessions {
        match marks.iter_mut().find(|m| m.mark == s.mark) {
            Some(m) => m.sessions.push(s.id.clone()),
            None => marks.push(SurveyMarkRow {
                mark: s.mark.clone(),
                sessions: vec![s.id.clone()],
                solutions: Vec::new(),
                point: survey.points.iter().find(|p| p.mark == s.mark).map(|p| p.id.clone()),
            }),
        }
    }
    for s in &survey.solutions {
        let Some(mark) = survey.mark_of(s) else { continue };
        if let Some(m) = marks.iter_mut().find(|m| m.mark == mark) {
            m.solutions.push(solution_row(survey, s, latest.get(mark).map(String::as_str)));
        }
    }
    let points: Vec<SurveyPointRow> = survey.points.iter().map(|p| point_row(doc, survey, p)).collect();
    let uses: Vec<SurveyUseRow> = survey.uses.iter().map(|u| use_row(doc, survey, u)).collect();
    let stale = count(
        points.iter().filter(|p| p.state == S::Stale).count() + uses.iter().filter(|u| u.state == S::Stale).count(),
    );
    let object = target.map(|u| object(doc, survey, &u, &latest));
    let baselines = survey.baselines.iter().map(|b| baseline_row(survey, b, deep)).collect();
    Ok(MapSurveyTree {
        campaigns,
        sessions,
        marks,
        points,
        uses,
        baselines,
        known: survey.known.clone(),
        adjustments: survey.adjustments.clone(),
        // Docs/adr/4120: the known points' own places, when asked.
        known_places: if places { crate::map_survey_places::known_places(doc, survey) } else { Vec::new() },
        object,
        stale,
        deep,
        revision: doc.revision().to_string(),
    })
}

/// A baseline's length (m).
pub(crate) fn baseline_length(b: &SurveyBaseline) -> f64 {
    (b.d[0] * b.d[0] + b.d[1] * b.d[1] + b.d[2] * b.d[2]).sqrt()
}

/// A baseline's row: its length, its processed standard deviations, its file as it stands, the adjustments made with
/// it (docs/adr/1740).
fn baseline_row(survey: &MapSurvey, b: &SurveyBaseline, deep: bool) -> SurveyBaselineRow {
    let c = b.covariance;
    SurveyBaselineRow {
        id: b.id.clone(),
        campaign: b.campaign.clone(),
        from: b.from.clone(),
        to: b.to.clone(),
        length: baseline_length(b),
        sigma: [c[0].sqrt(), c[3].sqrt(), c[5].sqrt()],
        excluded: b.excluded.clone(),
        adjust: b.adjust,
        file: b.file.as_ref().map(|f| SurveyFileRow {
            path: f.path.clone(),
            revision: f.revision.clone(),
            format: f.format,
            state: file_state(&f.path, &f.revision, deep),
        }),
        source: b.source.clone(),
        adjustments: survey.adjustments.iter().filter(|a| a.baselines.contains(&b.id)).map(|a| a.id.clone()).collect(),
    }
}

fn file_state(path: &str, revision: &str, deep: bool) -> F {
    if !deep {
        return if Path::new(path).is_file() { F::Found } else { F::Missing };
    }
    match crate::map_work::file_sha(path) {
        Some(sha) if sha == revision => F::Current,
        Some(_) => F::Changed,
        None => F::Missing,
    }
}

/// The object asked about, by its checked id text.
fn target(doc: &Document, uid: &str) -> Result<String, Stop> {
    if !crate::checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
            "uid",
        ));
    }
    if Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).is_none() {
        return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), "uid"));
    }
    Ok(uid.to_owned())
}

fn object(doc: &Document, survey: &MapSurvey, uid: &str, latest: &HashMap<String, String>) -> SurveyObject {
    let point = survey.points.iter().find(|p| p.uid.to_text() == uid);
    let solutions = point
        .map(|p| {
            p.result
                .solutions
                .iter()
                .filter_map(|s| survey.solution(s))
                .map(|s| solution_row(survey, s, latest.get(&p.mark).map(String::as_str)))
                .collect()
        })
        .unwrap_or_default();
    SurveyObject {
        uid: uid.to_owned(),
        point: point.map(|p| point_row(doc, survey, p)),
        solutions,
        used: survey.uses.iter().find(|u| u.uids.iter().any(|x| x.to_text() == uid)).map(|u| use_row(doc, survey, u)),
    }
}

/// An accepted point's result made of `ids` by `method` (docs/adr/1340 §3): checked, the marks' heights by the
/// sessions' antennas now, the mean, and the coordinate chain into the project. `at` is the input's path of the ids.
pub(crate) fn compute(
    doc: &Document,
    survey: &MapSurvey,
    mark: &str,
    ids: &[String],
    method: AcceptMethod,
    at: &str,
) -> Result<SurveyResult, Stop> {
    let mut sols: Vec<&SurveySolution> = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let path = format!("{at}[{i}]");
        let Some(s) = survey.solution(id) else {
            return Err(fail(codes::UNKNOWN_SURVEY_RECORD, format!("GNSS arşivinde “{id}” kimlikli çözüm yok."), &path));
        };
        if ids[..i].contains(id) {
            return Err(fail(codes::INVALID_SURVEY_VALUE, format!("“{id}” çözümü iki kez verilmiş."), &path));
        }
        sols.push(s);
    }
    for (i, s) in sols.iter().enumerate() {
        let path = format!("{at}[{i}]");
        if let Some(reason) = &s.excluded {
            return Err(fail(
                codes::SOLUTION_EXCLUDED,
                format!("“{}” çözümü dışlanmış ({reason}); önce dışlamayı kaldırın ya da başka çözüm seçin.", s.id),
                &path,
            ));
        }
        if survey.mark_of(s) != Some(mark) {
            return Err(fail(
                codes::MARK_MISMATCH,
                format!("“{}” çözümü “{}” işaretinin, “{mark}” işaretinin değil.", s.id, survey.mark_of(s).unwrap_or("")),
                &path,
            ));
        }
    }
    let n = sols.len();
    if method == AcceptMethod::Network {
        return Err(fail(
            codes::METHOD_COUNT,
            "Ağ dengelemesi çözümlerle seçilmez; noktayı ağ dengelemesiyle (network) kabul edin.".into(),
            at,
        ));
    }
    let fits = match method {
        AcceptMethod::Single | AcceptMethod::Network => n == 1,
        AcceptMethod::Mean | AcceptMethod::Weighted => n >= 2,
    };
    if !fits {
        return Err(fail(
            codes::METHOD_COUNT,
            format!("{} yöntemi {} ister; {n} çözüm verildi.", method.title(), if method == AcceptMethod::Single { "tek çözüm" } else { "en az iki çözüm" }),
            at,
        ));
    }
    if method == AcceptMethod::Weighted
        && let Some((i, s)) = sols.iter().enumerate().find(|(_, s)| s.sigma.is_none())
    {
        return Err(fail(
            codes::SIGMA_MISSING,
            format!("Ağırlıklı ortalama her çözümün σ'sını ister; “{}” çözümünün σ'sı yok.", s.id),
            &format!("{at}[{i}]"),
        ));
    }
    let first = sols[0];
    let campaign = campaign_of(survey, first);
    for (i, s) in sols.iter().enumerate().skip(1) {
        let c = campaign_of(survey, s);
        let same_frame = s.srid == first.srid && s.epoch == first.epoch;
        let same_decision = c.map(|c| (c.epoch, &c.velocity)) == campaign.map(|c| (c.epoch, &c.velocity));
        if !same_frame || !same_decision {
            return Err(fail(
                codes::SOLUTIONS_DIFFER,
                format!(
                    "“{}” ile “{}” aynı çerçeve ve epokta değil ya da arazi ölçümlerinin epok kararı farklı; önce aynı çerçeveye taşıyın.",
                    first.id, s.id
                ),
                &format!("{at}[{i}]"),
            ));
        }
    }
    let mut antenna: Vec<AntennaUsed> = Vec::new();
    let mut heights = Vec::with_capacity(n);
    for (i, s) in sols.iter().enumerate() {
        let path = format!("{at}[{i}]");
        let session = survey.session(&s.session);
        let Some(a) = session.and_then(|x| x.antenna.as_ref()) else {
            return Err(fail(
                codes::ANTENNA_UNKNOWN,
                format!("“{}” oturumunun anten yüksekliği yok; anteni ve yüksekliğin ölçüldüğü yeri girin.", s.session),
                &path,
            ));
        };
        if a.eccentric() {
            return Err(fail(
                codes::ANTENNA_ECCENTRIC,
                format!("“{}” oturumunun anteni işaretin dışında (doğu/kuzey dış merkezlik); bu sürümde indirgenmez.", s.session),
                &path,
            ));
        }
        let h = a.vertical();
        if !antenna.iter().any(|u| u.session == s.session) {
            antenna.push(AntennaUsed { session: s.session.clone(), height: h });
        }
        heights.push(s.height + s.antenna_applied - h);
    }
    let (lat, lon, height, sigma) = match method {
        AcceptMethod::Single | AcceptMethod::Network => (first.lat, first.lon, heights[0], first.sigma),
        AcceptMethod::Mean => {
            let (mut a, mut b, mut c) = (0.0, 0.0, 0.0);
            for (s, h) in sols.iter().zip(&heights) {
                a += s.lat;
                b += s.lon;
                c += h;
            }
            let k = n as f64;
            (a / k, b / k, c / k, None)
        }
        AcceptMethod::Weighted => {
            let mut sums = [(0.0f64, 0.0f64); 3];
            for (s, h) in sols.iter().zip(&heights) {
                let g = s.sigma.unwrap_or([1.0; 3]);
                for (k, v) in [s.lat, s.lon, *h].into_iter().enumerate() {
                    let w = 1.0 / (g[k] * g[k]);
                    sums[k].0 += w * v;
                    sums[k].1 += w;
                }
            }
            let v = |k: usize| sums[k].0 / sums[k].1;
            let g = |k: usize| (1.0 / sums[k].1).sqrt();
            (v(0), v(1), v(2), Some([g(0), g(1), g(2)]))
        }
    };
    let placed = place(doc, campaign, (lat, lon, height), first.srid, first.epoch, at)?;
    Ok(SurveyResult {
        revision: 1,
        solutions: ids.to_vec(),
        method,
        lat,
        lon,
        height,
        srid: first.srid,
        epoch: first.epoch,
        target_epoch: placed.target,
        antenna,
        sigma,
        x: placed.x,
        y: placed.y,
        z: placed.z,
        chain: placed.chain,
        reasons: Vec::new(),
        adjustment: None,
    })
}

/// A mark's place in the project: x, y, z there, whether the chain was complete, the epoch decision it went to.
pub(crate) struct Placed {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    pub chain: String,
    pub target: Option<f64>,
}

/// A mark's latitude, longitude (degrees) and ellipsoidal height (m) in `srid` at `epoch` moved into the project by the
/// coordinate chain itself (docs/adr/1340 §3): to the campaign's epoch decision with its velocity grid; the chain's own
/// refusal kept, its path the operation's. `at` is the input's path for the other refusals.
pub(crate) fn place(
    doc: &Document,
    campaign: Option<&SurveyCampaign>,
    (lat, lon, height): (f64, f64, f64),
    srid: u32,
    epoch: Option<f64>,
    at: &str,
) -> Result<Placed, Stop> {
    let target = decision(doc, campaign);
    let with_z = doc.settings().heights.is_some();
    let ellipsoidal = HeightReference { kind: HeightKind::Ellipsoidal, vertical_srid: None, geoid: None };
    let input = CrsTransform {
        points: vec![CrsPoint { x: lon, y: lat, z: with_z.then_some(height) }],
        from: Some(CrsSide { srid: Some(srid), heights: with_z.then_some(ellipsoidal), epoch }),
        to: Some(CrsSide { srid: None, heights: None, epoch: target }),
        uncertainty: None,
        velocity: campaign
            .and_then(|c| c.velocity.as_ref())
            .map(|g| VelocityModel { name: g.name.clone(), sha256: g.sha256.clone() }),
    };
    let out = match crate::crs_transform::answer(doc, &input) {
        CommandResult::Completed { output, .. } => output,
        // The chain's own refusal (two epochs without a velocity grid, a grid not on this device …), its words kept;
        // its path is the call's, not this input's: the operation's.
        CommandResult::Failed { mut error } => {
            error.path = Some("operation".into());
            return Err(Stop::failed(error));
        }
        other => {
            let message = format!("{other:?}");
            return Err(fail(codes::INVALID_SURVEY_VALUE, message, at));
        }
    };
    let Some(Some(p)) = out.points.first() else {
        let why = out.unreached.first().map_or("", |u| crate::crs_transform::reason_text(&u.reason));
        return Err(fail(
            codes::INVALID_SURVEY_VALUE,
            format!("İşaretin konumu projenin sistemine taşınamadı: {why}"),
            at,
        ));
    };
    Ok(Placed {
        x: p.x,
        y: p.y,
        z: p.z,
        chain: match out.chain.status {
            ChainStatus::Complete => "complete".into(),
            ChainStatus::Limited => "limited".into(),
        },
        target,
    })
}

/// Docs/adr/4520: the tolerance's own rules: with `places`, one limit at least, each a length above 0.
fn tolerance_checked(input: &MapSurveyQuery) -> Result<(), Stop> {
    let Some(t) = &input.tolerance else { return Ok(()) };
    let bad = |message: &str, path: &str| Err(Stop::failed(error(codes::INVALID_INPUT, message.to_owned(), Some(path.to_owned()))));
    if !input.places {
        return bad("Kabul farkı sınırı bilinen noktaların yerleriyle okunur: places'i de verin.", "tolerance");
    }
    if t.horizontal.is_none() && t.vertical.is_none() {
        return bad("Kabul farkı sınırı boş: yatay ya da düşey sınırı verin (m).", "tolerance");
    }
    for (v, what, path) in [(t.horizontal, "Yatay", "tolerance.horizontal"), (t.vertical, "Düşey", "tolerance.vertical")] {
        if let Some(v) = v
            && !(v.is_finite() && v > 0.0)
        {
            return bad(&format!("{what} sınır 0'dan büyük bir uzunluk olmalı (m)."), path);
        }
    }
    Ok(())
}

/// The archive as asked: the tolerance checked first, the known places judged against it (docs/adr/4520).
fn asked_tree(doc: &Document, input: &MapSurveyQuery) -> Result<MapSurveyTree, Stop> {
    tolerance_checked(input)?;
    let mut out = tree(doc, input.deep, input.uid.as_deref(), input.places)?;
    if let Some(t) = &input.tolerance {
        crate::map_survey_places::judged(&mut out.known_places, t);
    }
    Ok(out)
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &MapSurveyQuery) -> CommandResult<()> {
    match asked_tree(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &MapSurveyQuery) -> CommandResult<MapSurveyTree> {
    match asked_tree(cx.doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: MapSurveyQuery) -> CommandResult<MapSurveyTree> {
    plan(cx, &input)
}

/// The archive for a panel (and, with `uid`, what that object is; with `places` the known points' own places,
/// docs/adr/4120), or why it does not read.
pub fn preview(doc: &Document, deep: bool, uid: Option<&str>, places: bool) -> Result<MapSurveyTree, String> {
    tree(doc, deep, uid, places).map_err(|stop| stop.into_error().message)
}

/// The archive for a panel with the known points' places judged against the user's `tolerance` (docs/adr/4520), as
/// `map.survey.query` with `places` gives it; or why it does not read (a tolerance that does not hold, too).
pub fn preview_judged(
    doc: &Document,
    deep: bool,
    tolerance: Option<kentos_commands::SurveyPlaceTolerance>,
) -> Result<MapSurveyTree, String> {
    let input = MapSurveyQuery { deep, uid: None, places: true, tolerance };
    asked_tree(doc, &input).map_err(|stop| stop.into_error().message)
}

/// What one object is in the archive, read quickly, for the properties; none outside a map project with an archive,
/// for an object not in the drawing, or for one the archive does not name.
pub fn object_of(doc: &Document, uid: &str) -> Option<SurveyObject> {
    if doc.settings().cad_work != Some(kentos_commands::CadWork::Map) {
        return None;
    }
    let survey = doc.map_survey()?;
    let o = object(doc, survey, uid, &latest_of(survey));
    (o.point.is_some() || o.used.is_some()).then_some(o)
}
