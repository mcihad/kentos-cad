//! The storeys as the drawing's views read them: which storey is over another, whether a roof stands over its
//! own storey (docs/adr/8044).

use kentos_commands::{ArchLevel, ProjectSettings};

/// Whether a roof on `level` at `offset` stands over that storey's plan's cut (docs/adr/8044 §3): its eaves at the
/// storey's top and its offset, above the storey's floor. That plan draws its eaves dashed; the plan of the storey over
/// it, or the roof plan, draws it whole. One whose storey has no height and none over it stands on its floor: its own
/// plan is its roof plan.
pub fn over_its_storey(settings: &ProjectSettings, level: &str, offset: f64) -> bool {
    let Some(floor) = levels(settings)
        .iter()
        .find(|l| l.id == level)
        .map(|l| l.elevation)
    else {
        return false;
    };
    crate::geometry::storey_top(settings, Some(level))
        .is_some_and(|top| top + offset > floor + 1e-9)
}

/// The storey over `level`: the lowest whose floor is higher than its; none on the highest.
pub fn storey_over<'a>(settings: &'a ProjectSettings, level: &str) -> Option<&'a ArchLevel> {
    let all = levels(settings);
    let at = all.iter().find(|l| l.id == level)?.elevation;
    all.iter()
        .filter(|l| l.elevation > at)
        .min_by(|a, b| a.elevation.total_cmp(&b.elevation))
}

/// The project's storeys.
fn levels(settings: &ProjectSettings) -> &[ArchLevel] {
    settings.arch.as_ref().map_or(&[], |a| a.levels.as_slice())
}
