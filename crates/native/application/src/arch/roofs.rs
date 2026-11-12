//! What the roof tool asks of the document (docs/adr/8044 §4): the eaves round a building's walls, and each edge's slope
//! for the kind of roof chosen. `cad.roofs.create` takes the eaves and the slopes as given; these only find them.

use kentos_commands::{ArchLevel, ProjectSettings, SpacePart, Vec2};
use kentos_domain::Document;

/// A roof's kind as the tool's Tür offers it (docs/adr/8044 §4): which of its edges slope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoofForm {
    /// Kırma: every edge slopes.
    #[default]
    Hip,
    /// Beşik: the front edge and the one facing it; the others are gable ends.
    Gable,
    /// Tek eğimli: the front edge alone.
    Shed,
    /// Düz: none.
    Flat,
}

impl RoofForm {
    pub const ALL: [RoofForm; 4] = [
        RoofForm::Hip,
        RoofForm::Gable,
        RoofForm::Shed,
        RoofForm::Flat,
    ];

    /// Its name in the tool's menu.
    pub fn label(self) -> &'static str {
        match self {
            RoofForm::Hip => "Kırma",
            RoofForm::Gable => "Beşik",
            RoofForm::Shed => "Tek eğimli",
            RoofForm::Flat => "Düz",
        }
    }

    /// Its name on the command line.
    pub fn name(self) -> &'static str {
        match self {
            RoofForm::Hip => "hip",
            RoofForm::Gable => "gable",
            RoofForm::Shed => "shed",
            RoofForm::Flat => "flat",
        }
    }
}

/// Each edge's slope for a roof of `form` over the eaves `pts` at `slope` per cent; `front` is the edge whose eaves a
/// shed's water runs off and a gable roof's ridge runs along. A gable roof slopes `front` and the edge facing it most
/// nearly (the most nearly opposite direction: a rectangle's other long side).
pub fn slopes_for(pts: &[Vec2], form: RoofForm, slope: f64, front: usize) -> Vec<Option<f64>> {
    let n = pts.len();
    let front = front.min(n.saturating_sub(1));
    match form {
        RoofForm::Hip => vec![Some(slope); n],
        RoofForm::Flat => vec![None; n],
        RoofForm::Shed => (0..n).map(|i| (i == front).then_some(slope)).collect(),
        RoofForm::Gable => {
            let dir = |i: usize| {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                let len = (b.x - a.x).hypot(b.y - a.y).max(f64::MIN_POSITIVE);
                ((b.x - a.x) / len, (b.y - a.y) / len)
            };
            let f = dir(front);
            let facing = |i: usize| {
                let d = dir(i);
                d.0 * f.0 + d.1 * f.1
            };
            let back = (0..n)
                .filter(|i| *i != front)
                .min_by(|i, j| facing(*i).total_cmp(&facing(*j)));
            (0..n)
                .map(|i| (i == front || Some(i) == back).then_some(slope))
                .collect()
        }
    }
}

/// The edge of the closed line `pts` nearest `at`: the edge from `pts[i]` to the next point.
pub fn nearest_edge(pts: &[Vec2], at: Vec2) -> usize {
    let n = pts.len();
    let distance = |i: usize| {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 {
            (((at.x - a.x) * dx + (at.y - a.y) * dy) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a.x + dx * t - at.x).hypot(a.y + dy * t - at.y)
    };
    (0..n)
        .min_by(|i, j| distance(*i).total_cmp(&distance(*j)))
        .unwrap_or(0)
}

/// The corners of a closed line that turn: a corner whose edges run on in one line is left out (a partition meeting an
/// outer wall leaves one on the walls' outline; two edges in one line with one slope would be one roof face twice).
fn turning(pts: Vec<Vec2>) -> Vec<Vec2> {
    let mut pts = pts;
    loop {
        let n = pts.len();
        if n <= 3 {
            return pts;
        }
        let straight = (0..n).find(|&i| {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let (u, v) = ((b.x - a.x, b.y - a.y), (c.x - b.x, c.y - b.y));
            let cross = u.0 * v.1 - u.1 * v.0;
            let dot = u.0 * v.0 + u.1 * v.1;
            dot > 0.0 && cross.abs() <= 1e-9 * u.0.hypot(u.1) * v.0.hypot(v.1)
        });
        match straight {
            Some(i) => {
                pts.remove(i);
            }
            None => return pts,
        }
    }
}

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

/// The storey under `level`: the highest whose floor is below its own (a roof's level may be a storey of no walls, the
/// roof's own, standing on the walls of the one under it).
fn storey_under(doc: &Document, level: &str) -> Option<String> {
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.as_slice())
        .unwrap_or_default();
    let own = levels.iter().find(|l| l.id == level)?;
    levels
        .iter()
        .filter(|l| l.elevation < own.elevation)
        .max_by(|a, b| a.elevation.total_cmp(&b.elevation))
        .map(|l| l.id.clone())
}

/// The eaves round the walls enclosing `at`: their outer faces, as Döşeme's Dış yüzler finds them, among the walls on
/// `level`, else on the storey under it (a roof's own storey has none), moved `overhang` out (saçak), with only the
/// corners that turn. Why not, in the words the tool says: no walls round the point, a curved outline or a courtyard
/// (not drawn yet).
pub fn eaves_from_walls(
    doc: &Document,
    at: Vec2,
    level: Option<&str>,
    overhang: f64,
) -> Result<Vec<Vec2>, String> {
    let found = doc.slab_from_walls(at, level, false).or_else(|| {
        let under = storey_under(doc, level?)?;
        doc.slab_from_walls(at, Some(&under), false)
    });
    let Some(part) = found else {
        return Err(
            "Nokta hiçbir duvar konturunun içinde değil; saçak bulunamadı. Binanın içine tıklayın ya da saçağın \
             köşelerini çizin."
                .into(),
        );
    };
    if part
        .outer
        .bulges
        .as_ref()
        .is_some_and(|b| b.iter().any(|x| *x != 0.0))
    {
        return Err("Binanın dış konturu eğri; eğri saçaklı çatı henüz çizilmiyor. Saçağın köşelerini çizin.".into());
    }
    if !part.holes.is_empty() {
        return Err(
            "Binanın ortasında avlu ya da ışıklık var; iç boşluklu çatı henüz çizilmiyor. Saçağın köşelerini çizin."
                .into(),
        );
    }
    let moved: Vec<SpacePart> = crate::slabs::moved(&[part], overhang, "overhang")
        .map_err(|stop| stop.into_error().message)?;
    let Some(outer) = moved.into_iter().next().map(|p| p.outer) else {
        return Err("Saçak bulunamadı.".into());
    };
    Ok(turning(outer.pts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    #[test]
    fn a_kind_slopes_its_edges_from_the_front_one() {
        let rect = [p(0.0, 0.0), p(12.0, 0.0), p(12.0, 9.0), p(0.0, 9.0)];
        assert_eq!(
            slopes_for(&rect, RoofForm::Hip, 33.0, 0),
            vec![Some(33.0); 4]
        );
        assert_eq!(slopes_for(&rect, RoofForm::Flat, 33.0, 0), vec![None; 4]);
        // Beşik: the front edge and the one facing it; the ridge runs along them.
        assert_eq!(
            slopes_for(&rect, RoofForm::Gable, 40.0, 1),
            vec![None, Some(40.0), None, Some(40.0)]
        );
        assert_eq!(
            slopes_for(&rect, RoofForm::Shed, 10.0, 2),
            vec![None, None, Some(10.0), None]
        );
        // The edge nearest a click inside: near the north wall, the north edge (from (12,9) to (0,9)).
        assert_eq!(nearest_edge(&rect, p(6.0, 8.0)), 2);
        assert_eq!(nearest_edge(&rect, p(1.0, 4.0)), 3);
    }

    #[test]
    fn corners_that_do_not_turn_are_left_out() {
        let pts = vec![
            p(0.0, 0.0),
            p(5.0, 0.0),
            p(12.0, 0.0),
            p(12.0, 9.0),
            p(0.0, 9.0),
            p(0.0, 4.0),
        ];
        assert_eq!(
            turning(pts),
            vec![p(0.0, 0.0), p(12.0, 0.0), p(12.0, 9.0), p(0.0, 9.0)]
        );
    }
}
