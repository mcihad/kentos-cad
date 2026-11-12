//! `road.profiles.propose` v1 (plan/road.md Y25, docs/adr/0926; ROAD-PROFILE-14): new elevations for a profile's PVIs
//! that follow its surface's terrain profile as closely as the user's rules allow (`kentos_road::profile::propose`).
//! Reads only; a window writes the proposal as a new profile with `road.profiles.create`. The shared cases are
//! `fixtures/commands/v1/road.profiles.propose.json`.

use kentos_commands::{
    CommandResult, CommandWarning, GradeLimit, ProfileProposed, ProfilesPropose, ProposedGrade, ProposedPvi,
};
use kentos_domain::Document;
use kentos_road::profile::propose::{ProposeError, Rules, propose};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_profiles as common;
use crate::road_terrains::fail;

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_PROFILE_PROPOSAL, message, path)
}

/// A proposal, its warnings and the terrain profile it read (station, black).
pub type Proposed = (ProfileProposed, Vec<CommandWarning>, Vec<(f64, Option<f64>)>);

/// The proposal now, its warnings and the terrain profile it read (station, black), for a window.
pub(crate) fn answer(
    doc: &Document,
    input: &ProfilesPropose,
) -> Result<Proposed, Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(common::profiles(doc), &input.profile, "profile")?;
    for (j, id) in input.hold.iter().enumerate() {
        if !def.pvis.iter().any(|p| &p.id == id) {
            return Err(invalid(format!("“{}” boykesitinde “{id}” PVI'si yok.", def.name), &format!("hold[{j}]")));
        }
    }
    if input.most_grade.is_some_and(|g| !(g.is_finite() && g > 0.0)) {
        return Err(invalid("En büyük eğim (mostGrade) 0'dan büyük bir oran olmalı.".into(), "mostGrade"));
    }
    let mut warnings = Vec::new();
    let surface = match common::surface(doc, def) {
        None => {
            return Err(invalid(
                format!("“{}” boykesitinin yüzeyi yok; öneri arazi boykesitine göre kurulur.", def.name),
                "profile",
            ));
        }
        Some(Err(_)) => {
            return Err(invalid(format!("“{}” boykesitinin yüzeyi kurulamıyor; öneri arazi boykesitine göre kurulur.", def.name), "profile"));
        }
        Some(Ok(s)) => s,
    };
    let built = common::build(doc, def)?;
    let (stale, now) = common::stale(doc, def);
    if stale {
        warnings.push(CommandWarning {
            code: codes::PROFILE_STALE.into(),
            message: format!(
                "“{}” boykesitinin aksı değişti (boykesit {}. revizyona göre kurulu, aks şimdi {}.); PVI'ları denetleyip aksa yeniden bağlayın.",
                def.name, def.alignment_revision, now
            ),
            path: Some("profile".into()),
        });
    }
    let ground = kentos_road::profile::terrain(&surface, &built.alignment.core);
    let pvis = common::pvis_of(def);
    let rules = Rules {
        held: def.pvis.iter().map(|p| input.hold.contains(&p.id)).collect(),
        most_grade: input.most_grade,
        balanced: input.balanced,
        bounds: common::crossings_of(def).iter().map(|c| c.bound()).collect(),
    };
    let p = propose(&pvis, &ground, &rules).map_err(|e| {
        invalid(
            match e {
                ProposeError::Circle { pvi } => format!(
                    "“{}” boykesitinin {pvi} PVI'sinde dairesel kurp var; öneri yalnız parabolik kurplarla kurulur.",
                    def.name
                ),
                ProposeError::NoGround => format!("“{}” boykesitinin hiçbir yeri yüzeyin üstünde değil.", def.name),
                ProposeError::Unfixed => format!(
                    "“{}” boykesitinde arazinin belirlemediği PVI kotları var (çevreleri yüzeyin dışında); onları kilitleyin.",
                    def.name
                ),
                ProposeError::Infeasible => format!(
                    "“{}” boykesitinin kuralları birlikte tutmuyor: kilitli PVI'lar arasındaki eğim en büyük eğimi aşıyor, denge kurulamıyor ya da kesişmelerin açıklık ve örtüsü birlikte sağlanamıyor.",
                    def.name
                ),
            },
            "profile",
        )
    })?;
    // The crossings against the proposed line (docs/adr/0946).
    let proposed: Vec<kentos_road::profile::vertical::Pvi> =
        pvis.iter().zip(&p.elevations).map(|(q, z)| kentos_road::profile::vertical::Pvi { elevation: *z, ..q.clone() }).collect();
    let crossings = match kentos_road::profile::vertical::build(&proposed) {
        Ok(line) => common::crossing_margins(def, &line, &p.bound),
        Err(_) => Vec::new(),
    };
    let out = ProfileProposed {
        profile: def.id.clone(),
        pvis: def
            .pvis
            .iter()
            .zip(&p.elevations)
            .zip(&rules.held)
            .map(|((d, z), held)| ProposedPvi { id: d.id.clone(), station: d.station, elevation: *z, was: d.elevation, held: *held })
            .collect(),
        grades: def
            .pvis
            .windows(2)
            .zip(p.grades.iter().zip(&p.limited))
            .map(|(w, (g, l))| ProposedGrade {
                between: [w[0].id.clone(), w[1].id.clone()],
                grade: *g,
                limit: match l {
                    1 => Some(GradeLimit::Rise),
                    -1 => Some(GradeLimit::Fall),
                    _ => None,
                },
            })
            .collect(),
        squares: p.squares,
        covered: p.covered,
        rms: (p.squares / p.covered).sqrt(),
        cut: p.cut,
        fill: p.fill,
        crossings,
    };
    Ok((out, warnings, ground.iter().map(|q| (q.station, q.z)).collect()))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesPropose) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings, _)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesPropose) -> CommandResult<ProfileProposed> {
    match answer(cx.doc, input) {
        Ok((output, warnings, _)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesPropose) -> CommandResult<ProfileProposed> {
    plan(cx, &input)
}

/// The proposal and the terrain profile for a window, or why it does not read.
pub fn preview(doc: &Document, input: &ProfilesPropose) -> Result<Proposed, String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
