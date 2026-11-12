//! What `map.survey.network` reads beyond each baseline's own covariance (docs/adr/4920): the baseline sessions whose
//! full covariance the adjustment takes when asked (`correlations: session`), and the new marks' velocities read from
//! the campaign's velocity grid (`velocities`), the user's own grid, by the coordinate chain itself; never a velocity
//! KentOS makes up. The shared cases are `fixtures/commands/v1/map.survey.network.json`.

use kentos_commands::{MapSurvey, SurveyBaseline, VelocityModel};
use kentos_domain::Document;
use kentos_geometry_core::survey::adjust::gnss::{self, GnssSession};

use crate::checks::Stop;
use crate::codes;
use crate::map_survey::fail;
use crate::map_survey_network::{RAD, ecef, moved_to};

/// The used baselines' sessions in the archive's order: each `bo-N` naming two or more of them, with their places in
/// `used` in the session's order.
pub(crate) fn sessions_of(survey: &MapSurvey, used: &[&SurveyBaseline]) -> Vec<(String, Vec<usize>)> {
    let at: std::collections::HashMap<&str, usize> = used.iter().enumerate().map(|(k, b)| (b.id.as_str(), k)).collect();
    survey
        .baseline_sessions
        .iter()
        .filter_map(|s| {
            let members: Vec<usize> = s.baselines.iter().filter_map(|b| at.get(b.as_str()).copied()).collect();
            (members.len() >= 2).then(|| (s.id.clone(), members))
        })
        .collect()
}

/// The core's sessions of `relevant`: each block's correlations made covariances by the two baselines' standard
/// deviations (Σᵢⱼ = ρᵢⱼ·σᵢ·σⱼ), a block of a baseline left out of the adjustment dropped with it (the rest of the
/// session's covariance is its marginal); refused when a pair of used baselines has no block.
pub(crate) fn core_sessions(
    survey: &MapSurvey,
    used: &[&SurveyBaseline],
    relevant: &[(String, Vec<usize>)],
    refuse: &dyn Fn(&'static str, String) -> Stop,
) -> Result<Vec<GnssSession>, Stop> {
    let sd = |b: &SurveyBaseline| [b.covariance[0].sqrt(), b.covariance[3].sqrt(), b.covariance[5].sqrt()];
    let mut out = Vec::with_capacity(relevant.len());
    for (id, members) in relevant {
        let Some(s) = survey.baseline_session(id) else { continue };
        let mut cross = Vec::new();
        for (p, &a) in members.iter().enumerate() {
            for &b in &members[p + 1..] {
                let (ia, ib) = (used[a].id.as_str(), used[b].id.as_str());
                let block = s.blocks.iter().find_map(|x| {
                    if x.row == ia && x.col == ib {
                        Some((a, b, x.r))
                    } else if x.row == ib && x.col == ia {
                        Some((b, a, x.r))
                    } else {
                        None
                    }
                });
                let Some((row, col, r)) = block else {
                    return Err(refuse(
                        codes::SESSION_BLOCKS_MISSING,
                        format!(
                            "“{id}” baz oturumunda “{ia}” ile “{ib}” bazları arasındaki korelasyon bloğu yok; oturum korelasyonsuz dengelenebilir (correlations none) ya da dosya yeniden alınmalı."
                        ),
                    ));
                };
                let (sr, sc) = (sd(used[row]), sd(used[col]));
                let mut cov = [0.0; 9];
                for i in 0..3 {
                    for j in 0..3 {
                        cov[3 * i + j] = r[3 * i + j] * sr[i] * sc[j];
                    }
                }
                cross.push((row, col, cov));
            }
        }
        out.push(GnssSession { name: id.clone(), baselines: members.clone(), cross });
    }
    Ok(out)
}

/// A new mark's velocity east, north, up (m/yr) from the campaign's velocity grid: the chain carries it one year
/// (`epoch` to `epoch` + 1) and the move is read in the mark's own east, north, up.
pub(crate) fn velocity_at(
    doc: &Document,
    grid: &VelocityModel,
    at: (f64, f64, f64),
    srid: u32,
    shape: (f64, f64),
    epoch: f64,
) -> Result<[f64; 3], Stop> {
    let later = moved_to(doc, grid, at, srid, epoch, epoch + 1.0)?;
    let (a, b) = (ecef(at, shape), ecef(later, shape));
    let r = gnss::enu_rows(at.0 / RAD, at.1 / RAD);
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    Ok([0, 1, 2].map(|i| r[i][0] * d[0] + r[i][1] * d[1] + r[i][2] * d[2]))
}

/// Refused: velocities asked for without the campaign's velocity grid.
pub(crate) fn no_grid(path: &str) -> Stop {
    fail(
        codes::NO_VELOCITY_GRID,
        "Yeni noktaların hızı arazi ölçümünün hız ızgarasından okunur; KentOS hız uydurmaz. Arazi ölçümüne hız ızgarasını verin (Proje ayarları › Izgaralar).".into(),
        path,
    )
}

/// docs/adr/5320: a baseline session's independent and dependent baselines: of its baselines in the archive and not
/// excluded, by length (the lower id number of equals), each one that joins two marks not yet joined is independent
/// (Kruskal's shortest spanning tree: n − 1 for n marks tied together), the rest dependent; mark names compared as
/// written.
pub(crate) fn independent(survey: &MapSurvey, session: &kentos_commands::SurveyBaselineSession) -> (Vec<String>, Vec<String>) {
    let number = |id: &str| id.rsplit('-').next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(u64::MAX);
    let mut members: Vec<&SurveyBaseline> = session
        .baselines
        .iter()
        .filter_map(|id| survey.baselines.iter().find(|b| b.id == *id))
        .filter(|b| b.excluded.is_none())
        .collect();
    members.sort_by(|a, b| {
        crate::map_survey::baseline_length(a)
            .total_cmp(&crate::map_survey::baseline_length(b))
            .then(number(&a.id).cmp(&number(&b.id)))
    });
    let mut parent: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    fn root<'a>(parent: &mut std::collections::HashMap<&'a str, &'a str>, x: &'a str) -> &'a str {
        let mut r = x;
        while let Some(&p) = parent.get(r) {
            if p == r {
                break;
            }
            r = p;
        }
        parent.insert(x, r);
        r
    }
    let (mut free, mut tied) = (Vec::new(), Vec::new());
    for b in members {
        parent.entry(&b.from).or_insert(&b.from);
        parent.entry(&b.to).or_insert(&b.to);
        let (a, c) = (root(&mut parent, &b.from), root(&mut parent, &b.to));
        if a == c {
            tied.push(b.id.clone());
        } else {
            parent.insert(a, c);
            free.push(b.id.clone());
        }
    }
    (free, tied)
}
