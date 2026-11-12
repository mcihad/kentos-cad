//! The map commands' inputs with the numbers a fixture's `nonFinite` may name (docs/adr/8054 §9).

#[allow(unused_imports)]
use super::*;

impl Input for kentos_commands::PointCloudsDescribe {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PointCloudsRegion {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PointCloudsProfile {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PointCloudsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ParcelsCreate {
    /// `pts[i].x`, `lineWeight`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, axis) = path.strip_prefix("pts[")?.split_once("].")?;
        let p = self.pts.get_mut(i.parse::<usize>().ok()?)?;
        match axis {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::SurveyAdjust {
    /// `work.rows[i].direction`, `work.rows[i].dh`, `work.known[i].at.x` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::AdjustWork as W;
        let (list, rest) = path.strip_prefix("work.")?.split_once('[')?;
        let (i, field) = rest.split_once("].")?;
        let i = i.parse::<usize>().ok()?;
        match (&mut self.work, list, field) {
            (W::Horizontal { rows, .. }, "rows", "direction") => {
                rows.get_mut(i)?.direction.as_mut()
            }
            (W::Horizontal { known, .. }, "known", "at.x") => {
                known.get_mut(i)?.at.as_mut().map(|a| &mut a.x)
            }
            (W::Level { rows, .. }, "rows", "dh") => Some(&mut rows.get_mut(i)?.dh),
            // docs/adr/5300: a leap-frog setup's sight.
            (W::Level { leapfrog, .. }, "leapfrog", f) => {
                let setup = leapfrog.get_mut(i)?;
                let (side, what) = f.split_once('.')?;
                let s = match side {
                    "back" => &mut setup.back,
                    "fore" => &mut setup.fore,
                    _ => return None,
                };
                match what {
                    "distance" => Some(&mut s.distance),
                    "zenith" => Some(&mut s.zenith),
                    "target" => Some(&mut s.target),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::SurveyPoints {
    /// `work.angles[i]`, `work.a.at.x`, `work.heights.zeniths[i]`, `work.heights.back[i].zenith` (the shared cases
    /// spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::{KnownPoint, SurveyWork as S};
        match (&mut self.work, path) {
            (
                S::Forward {
                    a: KnownPoint::At { at },
                    ..
                },
                "work.a.at.x",
            ) => Some(&mut at.x),
            // docs/adr/4100: a leg's zenith angle; docs/adr/4500: a back sight's.
            (S::Traverse { heights: Some(h), .. }, _) if path.starts_with("work.heights.") => {
                if let Some(i) = path.strip_prefix("work.heights.zeniths[") {
                    return h.zeniths.get_mut(i.strip_suffix(']')?.parse::<usize>().ok()?);
                }
                let i = path.strip_prefix("work.heights.back[")?.strip_suffix("].zenith")?;
                h.back.get_mut(i.parse::<usize>().ok()?)?.as_mut().map(|b| &mut b.zenith)
            }
            (S::Traverse { angles, .. }, _) => {
                let i = path.strip_prefix("work.angles[")?.strip_suffix(']')?;
                angles.get_mut(i.parse::<usize>().ok()?)
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::MapWorkRestore {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::MapWorkQuery {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "limit.metres" => self.limit.as_mut()?.metres.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::MapWorkEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CrsFactors {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::FeaturesCode {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SurveyBaselines {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PointCloudsOverlap {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PointCloudsNetwork {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PointCloudsPlace {
    /// `place.yaw`, `place.angle`, `place.matrix[i]` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (&mut self.place, path) {
            (kentos_commands::CloudPlaceBy::Euler { yaw, .. }, "place.yaw") => Some(yaw),
            (kentos_commands::CloudPlaceBy::AxisAngle { angle, .. }, "place.angle") => Some(angle),
            (kentos_commands::CloudPlaceBy::Matrix { matrix }, p) => {
                let i = p.strip_prefix("place.matrix[")?.strip_suffix(']')?.parse::<usize>().ok()?;
                matrix.get_mut(i)
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::PointCloudsRegister {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OrthophotoCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OrthophotoCompare {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OrthophotoSeamlines {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OrthophotoNetwork {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OrthophotoFootprint {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ReadjustmentDistribute {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ReadjustmentScope {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "tolerance" => self.tolerance.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ReadjustmentSheets {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "scope.tolerance" => self.scope.tolerance.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ZoningCheck {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "tolerance" => self.tolerance.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ParcelsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ParcelsBoundaries {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ParcelsRecord {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ZoningSymbolize {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ParcelsApprove {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AsbuiltReview {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ParcelsWithdraw {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ZoningDraft {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ParcelsRecords {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PipesCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PipesProfile {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "exaggeration" => self.exaggeration.as_mut(),
            "height" => self.height.as_mut(),
            "datum" => self.datum.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::FieldPackage {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "horizontal" => self.horizontal.as_mut(),
            "vertical" => self.vertical.as_mut(),
            "quality" => self.quality.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::AsbuiltCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::MapSurveyQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::MapSurveyEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::MapSurveyNetwork {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::FeaturesReconcile {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "radius" => Some(&mut self.radius),
            _ => None,
        }
    }
}
