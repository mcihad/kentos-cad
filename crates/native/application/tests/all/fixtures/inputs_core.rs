//! The core commands' inputs with the numbers a fixture's `nonFinite` may name (docs/adr/8054 §9).

#[allow(unused_imports)]
use super::*;

impl Input for kentos_commands::StandardsPalettes {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::StandardsResolve {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::StandardsConflicts {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::StandardsInstall {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::StandardsExceptions {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::DrawingRepair {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "maxGap" => self.max_gap.as_mut(),
            "maxAngle" => self.max_angle.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::DrawingTake {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::DrawingInspect {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path { "maxGap" => self.max_gap.as_mut(), _ => None }
    }
}

impl Input for kentos_commands::ArraysFind {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path { "tolerance" => self.tolerance.as_mut(), _ => None }
    }
}

impl Input for kentos_commands::ArraysAdopt {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::ArrayLayout as L;
        match (&mut self.layout, path) {
            (_, "tolerance") => self.tolerance.as_mut(),
            (L::Grid { dx, .. } | L::Line { dx, .. }, "layout.dx") => Some(dx),
            (L::Grid { dy, .. } | L::Line { dy, .. }, "layout.dy") => Some(dy),
            (L::Polar { center, .. }, "layout.center.x") => Some(&mut center.x),
            (L::Polar { center, .. }, "layout.center.y") => Some(&mut center.y),
            (L::Polar { fill, .. }, "layout.fill") => Some(fill),
            (L::Path { spacing, .. }, "layout.spacing") => spacing.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::DimensionSetsCreateInput {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "gapMm" => Some(&mut self.gap_mm),
            _ => None,
        }
    }
}

impl Input for kentos_commands::DimensionSetsEditInput {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match &mut self.operation {
            kentos_commands::DimensionSetEdit::Move { at, .. } => match path {
                "operation.at.x" => Some(&mut at.x),
                "operation.at.y" => Some(&mut at.y),
                _ => None,
            },
            kentos_commands::DimensionSetEdit::RowOffset { offset, .. }
                if path == "operation.offset" =>
            {
                Some(offset)
            }
            kentos_commands::DimensionSetEdit::Set { gap_mm, .. } if path == "operation.gapMm" => {
                gap_mm.as_mut()
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::DimensionSetsQueryInput {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ConstraintsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ParametersEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ConstraintsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for PolygonCreate {
    /// `pts[1].y`, `bulges[3]`, `holes[0].pts[2].x`, `holes[1].bulges[0]`,
    /// `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (pts, bulges, rest) = match path.strip_prefix("holes[") {
            Some(rest) => {
                let (h, rest) = rest.split_once("].")?;
                let ring = self.holes.as_mut()?.get_mut(h.parse::<usize>().ok()?)?;
                (&mut ring.pts, &mut ring.bulges, rest)
            }
            None => (&mut self.pts, &mut self.bulges, path),
        };
        if rest.starts_with("pts[") {
            point_number(pts, rest)
        } else {
            bulge_number(bulges, rest)
        }
    }
}

impl Input for LineCreate {
    /// `a.x`, `b.y`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (end, axis) = path.split_once('.')?;
        let p = match end {
            "a" => &mut self.a,
            "b" => &mut self.b,
            _ => return None,
        };
        match axis {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for PolylineCreate {
    /// `pts[1].y`, `bulges[0]`, `widths[1][0]` (docs/adr/0258), `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            self.line_weight.as_mut()
        } else if let Some(rest) = path.strip_prefix("widths[") {
            let (i, rest) = rest.split_once("][")?;
            let k: usize = rest.strip_suffix(']')?.parse().ok()?;
            self.widths
                .as_mut()?
                .get_mut(i.parse::<usize>().ok()?)?
                .get_mut(k)
        } else if path.starts_with("pts[") {
            point_number(&mut self.pts, path)
        } else {
            bulge_number(&mut self.bulges, path)
        }
    }
}

impl Input for kentos_commands::NurbsCreate {
    /// `by.ctrl[1].y`, `by.pts[0].x`, `by.weights[2]`, `by.knots[3]`, `lineWeight` (docs/adr/0259).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::NurbsBy;
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let rest = path.strip_prefix("by.")?;
        match &mut self.by {
            NurbsBy::Control {
                ctrl,
                weights,
                knots,
                ..
            } => named_point(ctrl, "ctrl", rest)
                .or_else(|| list_number(weights.as_deref_mut()?, "weights", rest))
                .or_else(|| list_number(knots.as_deref_mut()?, "knots", rest)),
            NurbsBy::Fit { pts, .. } => named_point(pts, "pts", rest),
        }
    }
}

impl Input for kentos_commands::GridsCreate {
    /// `grids[0].pts[k].x` and `.y`, `series.spacings[k]`, `series.ends`, `lineWeight` (docs/adr/0343); the curved
    /// system's `radial.center.x` and `.y`, `.radius`, `.turns[k]`, `.rings[k]` (docs/adr/0725 §1).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        if path == "series.ends" {
            return self.series.as_mut()?.ends.as_mut();
        }
        if let Some(rest) = path.strip_prefix("series.spacings[") {
            let k = rest.strip_suffix(']')?.parse::<usize>().ok()?;
            return self.series.as_mut()?.spacings.get_mut(k);
        }
        if let Some(rest) = path.strip_prefix("radial.") {
            let r = self.radial.as_mut()?;
            return match rest {
                "center.x" => Some(&mut r.center.x),
                "center.y" => Some(&mut r.center.y),
                "radius" => Some(&mut r.radius),
                "ends" => r.ends.as_mut(),
                _ => {
                    let (list, k) = rest.split_once('[')?;
                    let k = k.strip_suffix(']')?.parse::<usize>().ok()?;
                    match list {
                        "turns" => r.turns.get_mut(k),
                        "rings" => r.rings.get_mut(k),
                        _ => None,
                    }
                }
            };
        }
        if let Some(rest) = path.strip_prefix("system.") {
            let s = self.system.as_mut()?;
            return match rest {
                "origin.x" => Some(&mut s.origin.x),
                "origin.y" => Some(&mut s.origin.y),
                "angle" => s.angle.as_mut(),
                "extension" => s.extension.as_mut(),
                "columns.width" => Some(&mut s.columns.as_mut()?.width),
                _ => {
                    let (axis, k) = rest.split_once('[')?;
                    let k = k.strip_suffix(']')?.parse::<usize>().ok()?;
                    match axis {
                        "x" => s.x.get_mut(k),
                        "y" => s.y.get_mut(k),
                        _ => None,
                    }
                }
            };
        }
        let (i, rest) = path.strip_prefix("grids[")?.split_once("].")?;
        let g = self.grids.get_mut(i.parse::<usize>().ok()?)?;
        named_point(&mut g.pts, "pts", rest)
    }
}

impl Input for kentos_commands::ColumnsCreate {
    /// `columns[0].width`, `.at.x` and `.y`, `lineWeight` (docs/adr/0343).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("columns[")?.split_once("].")?;
        let c = self.columns.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "width" => Some(&mut c.width),
            "at.x" => c.at.as_mut().map(|p| &mut p.x),
            "at.y" => c.at.as_mut().map(|p| &mut p.y),
            // An L's legs (docs/adr/0725 §3).
            "legs[0]" => c.legs.as_mut().map(|l| &mut l[0]),
            "legs[1]" => c.legs.as_mut().map(|l| &mut l[1]),
            _ => None,
        }
    }
}

impl Input for kentos_commands::StairsCreate {
    /// `stairs[0].tread`, `.width`, `.height`, `.cutHeight`, `.post`, `.pts[k].x` and `.y`, `.bulges[k]`, `lineWeight`
    /// (docs/adr/0343, docs/adr/0704, docs/adr/0709, docs/adr/0724).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("stairs[")?.split_once("].")?;
        let s = self.stairs.get_mut(i.parse::<usize>().ok()?)?;
        if let Some(k) = rest.strip_prefix("bulges[").and_then(|k| k.strip_suffix(']')) {
            return s.bulges.as_mut()?.get_mut(k.parse::<usize>().ok()?);
        }
        match rest {
            "tread" => Some(&mut s.tread),
            "width" => Some(&mut s.width),
            "height" => s.height.as_mut(),
            "cutHeight" => s.cut_height.as_mut(),
            "post" => s.post.as_mut(),
            _ => named_point(&mut s.pts, "pts", rest),
        }
    }
}

impl Input for kentos_commands::RampsCreate {
    /// `ramps[0].rise`, `.width`, `.baseOffset` (docs/adr/0723 §2), `.pts[k].x` and `.y`, `lineWeight` (docs/adr/0343).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("ramps[")?.split_once("].")?;
        let r = self.ramps.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "rise" => Some(&mut r.rise),
            "width" => Some(&mut r.width),
            "baseOffset" => r.base_offset.as_mut(),
            _ => named_point(&mut r.pts, "pts", rest),
        }
    }
}

impl Input for kentos_commands::BeamsCreate {
    /// `beams[0].width`, `.height`, `.offset`, `.baseOffset` (docs/adr/8041 §3), `.pts[k].x` and `.y`, `lineWeight`
    /// (docs/adr/8041 §1).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("beams[")?.split_once("].")?;
        let b = self.beams.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "width" => Some(&mut b.width),
            "height" => Some(&mut b.height),
            "offset" => b.offset.as_mut(),
            "baseOffset" => b.base_offset.as_mut(),
            _ => named_point(&mut b.pts, "pts", rest),
        }
    }
}

impl Input for kentos_commands::RailingsCreate {
    /// `railings[0].height`, `.width`, `.gap`, `.offset`, `.pts[k].x` and `.y`, `lineWeight` (docs/adr/8045 §3).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("railings[")?.split_once("].")?;
        let r = self.railings.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "height" => r.height.as_mut(),
            "width" => r.width.as_mut(),
            "gap" => r.gap.as_mut(),
            "offset" => r.offset.as_mut(),
            _ => named_point(&mut r.pts, "pts", rest),
        }
    }
}

impl Input for kentos_commands::XrefsEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::XrefsOperation as O;
        match (&mut self.operation, path) {
            (O::Attach { at: Some(p), .. }, "operation.at.x") => Some(&mut p.x),
            (O::Attach { at: Some(p), .. }, "operation.at.y") => Some(&mut p.y),
            (O::Attach { scale: Some(s), .. }, "operation.scale") => Some(s),
            (
                O::Attach {
                    rotation: Some(r), ..
                },
                "operation.rotation",
            ) => Some(r),
            _ => None,
        }
    }
}

impl Input for kentos_commands::XrefsList {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AssetsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::InsertsClip {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SheetsSet {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::NavigatorRead {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ExtractRun {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ExtractTable {
    /// `at.x` and `.y`, `height` (docs/adr/1030).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "height" => self.height.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::DimensionsInterior {
    /// `from.x`, `to.y`, `at.x`, `height`, `lineWeight`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "from.x" => Some(&mut self.from.x),
            "from.y" => Some(&mut self.from.y),
            "to.x" => Some(&mut self.to.x),
            "to.y" => Some(&mut self.to.y),
            "at.x" => self.at.as_mut().map(|p| &mut p.x),
            "at.y" => self.at.as_mut().map(|p| &mut p.y),
            "height" => self.height.as_mut(),
            "lineWeight" => self.look.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ColumnsDistancesInput {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SheetsDetails {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PlotStylesImport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SheetsLevels {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for AnnotationsScales {
    /// `scale`, `rotation`, `offset`, `scales[i]`, `point.x` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "scale" => self.scale.as_mut(),
            "rotation" => self.rotation.as_mut(),
            "offset" => self.offset.as_mut(),
            "point.x" => self.point.as_mut().map(|p| &mut p.x),
            "point.y" => self.point.as_mut().map(|p| &mut p.y),
            _ => {
                let i: usize = path
                    .strip_prefix("scales[")?
                    .strip_suffix(']')?
                    .parse()
                    .ok()?;
                self.scales.get_mut(i)
            }
        }
    }
}

impl Input for kentos_commands::EntitiesFlatten {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "plane" => self.plane.as_mut(),
            _ => None,
        }
    }
}

impl Input for EntitiesOrder {
    /// No number: the input is ids.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for EntitiesDelete {
    /// No number: the input is ids.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for BlocksDefine {
    /// `base.x`, `base.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        coordinate(&mut self.base, "base", path)
    }
}

impl Input for EntitiesTrim {
    /// `picks[0].at.x`, `boundaries.max.y` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if let Some(rest) = path.strip_prefix("boundaries.") {
            let Some(kentos_commands::TrimBoundaries::Area { min, max }) = &mut self.boundaries
            else {
                return None;
            };
            return match rest {
                "min.x" => Some(&mut min.x),
                "min.y" => Some(&mut min.y),
                "max.x" => Some(&mut max.x),
                "max.y" => Some(&mut max.y),
                _ => None,
            };
        }
        if let Some(rest) = path.strip_prefix("crossing.") {
            let w = self.crossing.as_mut()?;
            return match rest {
                "min.x" => Some(&mut w.min.x),
                "min.y" => Some(&mut w.min.y),
                "max.x" => Some(&mut w.max.x),
                "max.y" => Some(&mut w.max.y),
                _ => None,
            };
        }
        if let Some(rest) = path.strip_prefix("fence[") {
            let (i, rest) = rest.split_once(']')?;
            let p = self.fence.as_mut()?.get_mut(i.parse::<usize>().ok()?)?;
            return match rest {
                ".x" => Some(&mut p.x),
                ".y" => Some(&mut p.y),
                _ => None,
            };
        }
        let rest = path.strip_prefix("picks[")?;
        let (i, rest) = rest.split_once(']')?;
        let p = &mut self.picks.get_mut(i.parse::<usize>().ok()?)?.at;
        match rest {
            ".at.x" => Some(&mut p.x),
            ".at.y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::SnapFind {
    /// `at.x`, `at.y`, `aperture`, `from.x`, `from.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "aperture" => Some(&mut self.aperture),
            "from.x" => self.from.as_mut().map(|f| &mut f.x),
            "from.y" => self.from.as_mut().map(|f| &mut f.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::DonutCreate {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "inner" => Some(&mut self.inner),
            "outer" => Some(&mut self.outer),
            "lineWeight" => self.line_weight.as_mut(),
            _ => coordinate(&mut self.center, "center", path),
        }
    }
}

impl Input for kentos_commands::RectangleCreate {
    /// `frame.a.x` … `frame.length`, `frame.width`, `frame.area`, `rotation`, `corners.radius`, `corners.distance`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::{RectangleCorners as C, RectangleFrame as F};
        if path == "rotation" {
            return Some(self.rotation.get_or_insert(0.0));
        }
        let (head, field) = path.split_once('.')?;
        match (head, field) {
            ("frame", field) => match (&mut self.frame, field) {
                (F::Corners { a, .. }, "a.x") | (F::Edge { a, .. }, "a.x") => Some(&mut a.x),
                (F::Corners { a, .. }, "a.y") | (F::Edge { a, .. }, "a.y") => Some(&mut a.y),
                (F::Corners { b, .. }, "b.x") | (F::Edge { b, .. }, "b.x") => Some(&mut b.x),
                (F::Corners { b, .. }, "b.y") | (F::Edge { b, .. }, "b.y") => Some(&mut b.y),
                (F::Size { corner, .. }, "corner.x") | (F::Area { corner, .. }, "corner.x") => {
                    Some(&mut corner.x)
                }
                (F::Size { length, .. }, "length") | (F::Area { length, .. }, "length") => {
                    Some(length)
                }
                (F::Size { width, .. }, "width") | (F::Edge { width, .. }, "width") => Some(width),
                (F::Area { area, .. }, "area") => Some(area),
                _ => None,
            },
            ("corners", "radius") => match &mut self.corners {
                Some(C::Fillet { radius }) => Some(radius),
                _ => None,
            },
            ("corners", "distance") => match &mut self.corners {
                Some(C::Chamfer { distance }) => Some(distance),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Input for kentos_commands::SectorCreate {
    /// `c.x`, `c.y`, `r`, `a0`, `a1`, `lineWeight`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            "a0" => Some(&mut self.a0),
            "a1" => Some(&mut self.a1),
            "lineWeight" => self.line_weight.as_mut(),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

impl Input for kentos_commands::RevcloudCreate {
    /// `arc`, `lineWeight`, `frame.a.x` … `frame.b.y`, `frame.pts[i].x`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::RevcloudFrame as F;
        match (&mut self.frame, path) {
            (_, "arc") => Some(&mut self.arc),
            (_, "lineWeight") => self.line_weight.as_mut(),
            (F::Rectangle { a, .. }, path) if path.starts_with("frame.a") => {
                coordinate(a, "frame.a", path)
            }
            (F::Rectangle { b, .. }, path) if path.starts_with("frame.b") => {
                coordinate(b, "frame.b", path)
            }
            (F::Polygon { pts }, path) => {
                let rest = path.strip_prefix("frame.pts[")?;
                let (i, axis) = rest.split_once("].")?;
                let p = pts.get_mut(i.parse::<usize>().ok()?)?;
                match axis {
                    "x" => Some(&mut p.x),
                    "y" => Some(&mut p.y),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::ShapesEdit {
    /// `work.line[i].y`, `work.elevation.z`, `work.length` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::{ShapeElevation as E, ShapeWork as S};
        match (&mut self.work, path) {
            (
                S::Elevation {
                    elevation: E::Set { z },
                    ..
                },
                "work.elevation.z",
            ) => Some(z),
            (
                S::Split {
                    length: Some(l), ..
                },
                "work.length",
            ) => Some(l),
            (S::Reshape { line, .. }, _) => {
                let (i, axis) = path.strip_prefix("work.line[")?.split_once("].")?;
                let p = line.get_mut(i.parse::<usize>().ok()?)?;
                match axis {
                    "x" => Some(&mut p.x),
                    "y" => Some(&mut p.y),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::ObjectsDerive {
    /// `work.step`, `lineWeight` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (&mut self.work, path) {
            (kentos_commands::DeriveWork::Centerline { step, .. }, "work.step") => Some(step),
            (_, "lineWeight") => self.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::AnnotationsWrite {
    /// `work.at[i].x`, `work.at.x`, `work.interval` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::AnnotationWork as A;
        match (&mut self.work, path) {
            (A::Stations { interval, .. }, "work.interval") => Some(interval),
            (A::Along { at, .. } | A::Dimensions { at, .. }, "work.at.x") => Some(&mut at.x),
            (A::Coordinates { at: Some(at), .. }, _) => {
                let (i, axis) = path.strip_prefix("work.at[")?.split_once("].")?;
                let p = at.get_mut(i.parse::<usize>().ok()?)?;
                match axis {
                    "x" => Some(&mut p.x),
                    "y" => Some(&mut p.y),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::AreasEdit {
    /// `work.line[i].x`, `work.ring[i].y`, `work.at.x` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::AreaWork as A;
        let rest = path.strip_prefix("work.")?;
        let (list, place) = match &mut self.work {
            A::Split {
                line: Some(pts), ..
            } => (pts, rest.strip_prefix("line[")?),
            A::HoleAdd { ring, .. } => (ring, rest.strip_prefix("ring[")?),
            A::HoleRemove { at, .. } | A::HoleFill { at, .. } => {
                return match rest {
                    "at.x" => Some(&mut at.x),
                    "at.y" => Some(&mut at.y),
                    _ => None,
                };
            }
            _ => return None,
        };
        let (i, axis) = place.split_once("].")?;
        let p = list.get_mut(i.parse::<usize>().ok()?)?;
        match axis {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ConstructionsCreate {
    /// `lineWeight`, `construction.axis[i].x` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::Construction as C;
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let C::Parallel { axis, .. } = &mut self.construction else {
            return None;
        };
        let (i, axis_name) = path.strip_prefix("construction.axis[")?.split_once("].")?;
        let p = axis.get_mut(i.parse::<usize>().ok()?)?;
        match axis_name {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesAdjoin {
    /// `pts[i].x`, `bulges[i]`, `within.min.x`, `lineWeight`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        if let Some(rest) = path.strip_prefix("pts[") {
            let (i, axis) = rest.split_once("].")?;
            let p = self.pts.get_mut(i.parse::<usize>().ok()?)?;
            return match axis {
                "x" => Some(&mut p.x),
                "y" => Some(&mut p.y),
                _ => None,
            };
        }
        if let Some(rest) = path.strip_prefix("bulges[") {
            let i = rest.strip_suffix(']')?.parse::<usize>().ok()?;
            return self.bulges.as_mut()?.get_mut(i);
        }
        let w = self.within.as_mut()?;
        coordinate(&mut w.min, "within.min", path)
    }
}

impl Input for kentos_commands::RegularPolygonCreate {
    /// `frame.center.x`, `frame.a.y` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::RegularFrame as F;
        match (&mut self.frame, path) {
            (F::Inscribed { center, .. } | F::Circumscribed { center, .. }, "frame.center.x") => {
                Some(&mut center.x)
            }
            (F::Inscribed { corner, .. }, "frame.corner.y") => Some(&mut corner.y),
            (F::Edge { a, .. }, "frame.a.x") => Some(&mut a.x),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesVertex {
    /// `radius`, `through.x`, `through.y`, `widths[0]`, `widths[1]` (docs/adr/0288), `joinArcs` (docs/adr/5000).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "radius" => self.radius.as_mut(),
            "through.x" => self.through.as_mut().map(|p| &mut p.x),
            "through.y" => self.through.as_mut().map(|p| &mut p.y),
            "widths[0]" => self.widths.as_mut().map(|w| &mut w[0]),
            "widths[1]" => self.widths.as_mut().map(|w| &mut w[1]),
            "joinArcs" => self.join_arcs.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesPick {
    /// `at.x`, `at.y`, `tolerance`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "tolerance" => Some(&mut self.tolerance),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesRelate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::EntitiesIsolate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> { None }
}

impl Input for kentos_commands::EntitiesSelect {
    /// The region's `region.a.x` … `region.radius`, `region.tolerance`, `region.points[i].x`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::SelectRegion as R;
        let rest = path.strip_prefix("region.")?;
        let (head, x) = rest.split_once('.').unwrap_or((rest, ""));
        match (&mut self.region, head) {
            (R::Window { a, .. } | R::Crossing { a, .. }, "a") => axis(a, x),
            (R::Window { b, .. } | R::Crossing { b, .. }, "b") => axis(b, x),
            (R::Circle { center, .. }, "center") => axis(center, x),
            (R::Containing { at } | R::Point { at, .. }, "at") => axis(at, x),
            (R::Circle { radius, .. }, "radius") => Some(radius),
            (R::Fence { tolerance, .. } | R::Point { tolerance, .. }, "tolerance") => {
                Some(tolerance)
            }
            (R::Fence { points, .. } | R::Polygon { points, .. }, head) => {
                let i: usize = head
                    .strip_prefix("points[")?
                    .strip_suffix(']')?
                    .parse()
                    .ok()?;
                axis(points.get_mut(i)?, x)
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::CrsTransform {
    /// `points[i].x`, `.y`, `.z`, `uncertainty.horizontal`, `.vertical` (docs/adr/0519).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if let Some(rest) = path.strip_prefix("points[") {
            let (i, axis) = rest.split_once("].")?;
            let p = self.points.get_mut(i.parse::<usize>().ok()?)?;
            return match axis {
                "x" => Some(&mut p.x),
                "y" => Some(&mut p.y),
                "z" => p.z.as_mut(),
                _ => None,
            };
        }
        let u = self.uncertainty.as_mut()?;
        match path {
            "uncertainty.horizontal" => u.horizontal.as_mut(),
            "uncertainty.vertical" => u.vertical.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::PointsConstruct {
    /// `constructions[i].distance`, `.angle`, `.a.at.x` … `.edge.near.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::PointConstruction as C;
        let rest = path.strip_prefix("constructions[")?;
        let (i, rest) = rest.split_once("].")?;
        let c = self.constructions.get_mut(i.parse::<usize>().ok()?)?;
        let (head, axis) = rest.rsplit_once(".at.").unwrap_or((rest, ""));
        match (c, head) {
            (C::Mid { a, .. } | C::Along { a, .. }, "a") => measured_number(a, axis),
            (C::Mid { b, .. } | C::Along { b, .. }, "b") => measured_number(b, axis),
            (C::Track { x, .. }, "x") => measured_number(x, axis),
            (C::Track { y, .. }, "y") => measured_number(y, axis),
            (C::Polar { from, .. }, "from") => measured_number(from, axis),
            (C::Parallel { through, .. }, "through") => measured_number(through, axis),
            (C::Parallel { edge, .. }, "edge.near.x") => Some(&mut edge.near.x),
            (C::Parallel { edge, .. }, "edge.near.y") => Some(&mut edge.near.y),
            (
                C::Extension { distance, .. }
                | C::Parallel { distance, .. }
                | C::Polar { distance, .. }
                | C::Along { distance, .. },
                "distance",
            ) => Some(distance),
            (C::Polar { angle, .. }, "angle") => Some(angle),
            _ => None,
        }
    }
}

impl Input for kentos_commands::CurvesConstruct {
    /// `constructions[i].a.at.x` … (any point), `.first.near.x` …, `.r`, `.angle`, `.chord`, `.direction`,
    /// `.other`, `.rotation`, `.startAngle`, `.endAngle`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::CurveConstruction as C;
        let rest = path.strip_prefix("constructions[")?;
        let (i, rest) = rest.split_once("].")?;
        let c = self.constructions.get_mut(i.parse::<usize>().ok()?)?;
        if let Some((head, axis)) = rest.rsplit_once(".at.") {
            let p = match (c, head) {
                (
                    C::CircleTwoPoints { a, .. }
                    | C::CircleThreePoints { a, .. }
                    | C::EllipseAxis { a, .. },
                    "a",
                ) => a,
                (
                    C::CircleTwoPoints { b, .. }
                    | C::CircleThreePoints { b, .. }
                    | C::EllipseAxis { b, .. },
                    "b",
                ) => b,
                (C::CircleThreePoints { c, .. }, "c") => c,
                (
                    C::ArcThreePoints { start, .. }
                    | C::ArcStartCentreEnd { start, .. }
                    | C::ArcStartCentreAngle { start, .. }
                    | C::ArcStartCentreChord { start, .. }
                    | C::ArcStartEndAngle { start, .. }
                    | C::ArcStartEndDirection { start, .. }
                    | C::ArcStartEndRadius { start, .. },
                    "start",
                ) => start,
                (C::ArcThreePoints { mid, .. }, "mid") => mid,
                (
                    C::ArcThreePoints { end, .. }
                    | C::ArcStartCentreEnd { end, .. }
                    | C::ArcStartEndAngle { end, .. }
                    | C::ArcStartEndDirection { end, .. }
                    | C::ArcStartEndRadius { end, .. }
                    | C::ArcContinue { end, .. },
                    "end",
                ) => end,
                (
                    C::ArcStartCentreEnd { centre, .. }
                    | C::ArcStartCentreAngle { centre, .. }
                    | C::ArcStartCentreChord { centre, .. }
                    | C::EllipseCentre { centre, .. },
                    "centre",
                ) => centre,
                (C::EllipseCentre { axis_end, .. }, "axisEnd") => axis_end,
                _ => return None,
            };
            return measured_number(p, axis);
        }
        match (c, rest) {
            (
                C::CircleTangentTangentRadius { first, .. } | C::CircleTangents { first, .. },
                "first.near.x",
            ) => Some(&mut first.near.x),
            (
                C::CircleTangentTangentRadius { first, .. } | C::CircleTangents { first, .. },
                "first.near.y",
            ) => Some(&mut first.near.y),
            (C::CircleTangentTangentRadius { r, .. } | C::ArcStartEndRadius { r, .. }, "r") => {
                Some(r)
            }
            (C::ArcStartCentreAngle { angle, .. } | C::ArcStartEndAngle { angle, .. }, "angle") => {
                Some(angle)
            }
            (C::ArcStartCentreChord { chord, .. }, "chord") => Some(chord),
            (C::ArcStartEndDirection { direction, .. }, "direction") => Some(direction),
            (C::EllipseAxis { other, .. } | C::EllipseCentre { other, .. }, "other") => {
                other.as_mut()
            }
            (C::EllipseAxis { rotation, .. } | C::EllipseCentre { rotation, .. }, "rotation") => {
                rotation.as_mut()
            }
            (
                C::EllipseAxis { start_angle, .. } | C::EllipseCentre { start_angle, .. },
                "startAngle",
            ) => start_angle.as_mut(),
            (C::EllipseAxis { end_angle, .. } | C::EllipseCentre { end_angle, .. }, "endAngle") => {
                end_angle.as_mut()
            }
            _ => None,
        }
    }
}

impl Input for EntitiesStretch {
    /// `window.min.x` … `window.max.y`, `dx`, `dy`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "dx" => Some(&mut self.dx),
            "dy" => Some(&mut self.dy),
            "window.min.x" => Some(&mut self.window.min.x),
            "window.min.y" => Some(&mut self.window.min.y),
            "window.max.x" => Some(&mut self.window.max.x),
            "window.max.y" => Some(&mut self.window.max.y),
            _ => None,
        }
    }
}

impl Input for EntitiesBreak {
    /// `first.x` … `second.y` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "first.x" => Some(&mut self.first.x),
            "first.y" => Some(&mut self.first.y),
            "second.x" => self.second.as_mut().map(|p| &mut p.x),
            "second.y" => self.second.as_mut().map(|p| &mut p.y),
            _ => None,
        }
    }
}

impl Input for EntitiesJoin {
    /// `tolerance`, `chain.min.x` … (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "tolerance" {
            return self.tolerance.as_mut();
        }
        area_number(&mut self.chain, path.strip_prefix("chain.")?)
    }
}

impl Input for kentos_commands::EntitiesLengthen {
    /// `change.length`, `change.angle`, `change.percent`, `change.to.x` and `.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::LengthenChange as C;
        match (&mut self.change, path) {
            (C::Delta { length, .. } | C::Total { length, .. }, "change.length") => length.as_mut(),
            (C::Delta { angle, .. } | C::Total { angle, .. }, "change.angle") => angle.as_mut(),
            (C::Percent { percent }, "change.percent") => Some(percent),
            (C::Dynamic { to }, "change.to.x") => Some(&mut to.x),
            (C::Dynamic { to }, "change.to.y") => Some(&mut to.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesReverse {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::EntitiesCopy {
    /// `base.x`, `base.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let b = self.base.as_mut()?;
        match path {
            "base.x" => Some(&mut b.x),
            "base.y" => Some(&mut b.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesPaste {
    /// `at.x`, `at.y`, `clip.base.x`, `clip.base.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => self.at.as_mut().map(|a| &mut a.x),
            "at.y" => self.at.as_mut().map(|a| &mut a.y),
            "clip.base.x" => Some(&mut self.clip.base.x),
            "clip.base.y" => Some(&mut self.clip.base.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::DrawingAudit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::BlocksConvert {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path { "tolerance" => self.tolerance.as_mut(), _ => None }
    }
}

impl Input for kentos_commands::BlocksFind {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path { "tolerance" => self.tolerance.as_mut(), _ => None }
    }
}

impl Input for kentos_commands::DrawingCompare {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "tolerance" => self.tolerance.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesCleanup {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "tolerance" => self.tolerance.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesMatch {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CurvesEdit {
    /// `edit.at.x`, `edit.at.y`, `edit.weight`, `edit.tolerance`, a knot's `edit.at`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::CurveEdit as E;
        match (&mut self.edit, path) {
            (
                E::AddFit { at, .. } | E::MoveFit { at, .. } | E::MoveControl { at, .. },
                "edit.at.x",
            ) => Some(&mut at.x),
            (
                E::AddFit { at, .. } | E::MoveFit { at, .. } | E::MoveControl { at, .. },
                "edit.at.y",
            ) => Some(&mut at.y),
            (E::Weight { weight, .. }, "edit.weight") => Some(weight),
            (E::ToPolyline { tolerance }, "edit.tolerance") => Some(tolerance),
            (E::InsertKnot { at }, "edit.at") => Some(at),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesExplode {
    /// `density.length`, `density.tolerance`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::ExplodeDensity as D;
        match (self.density.as_mut()?, path) {
            (D::Step { length }, "density.length") => Some(length),
            (D::Deviation { tolerance }, "density.tolerance") => Some(tolerance),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ArraysEdit {
    /// `edit.scale`, `edit.rotation`, `edit.p.x`, `edit.p.y`, a layout's `edit.layout.dx` and `edit.layout.dy`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::{ArrayEdit as E, ArrayLayout as L};
        match (&mut self.edit, path) {
            (E::Placement { scale, .. }, "edit.scale") => scale.as_mut(),
            (E::Placement { rotation, .. }, "edit.rotation") => rotation.as_mut(),
            (E::Placement { p, .. }, "edit.p.x") => p.as_mut().map(|p| &mut p.x),
            (E::Placement { p, .. }, "edit.p.y") => p.as_mut().map(|p| &mut p.y),
            (
                E::Layout {
                    layout: L::Grid { dx, .. } | L::Line { dx, .. },
                },
                "edit.layout.dx",
            ) => Some(dx),
            (
                E::Layout {
                    layout: L::Grid { dy, .. } | L::Line { dy, .. },
                },
                "edit.layout.dy",
            ) => Some(dy),
            _ => None,
        }
    }
}

impl Input for EntitiesDivide {
    /// `division.length`, `start`, `block.rotation`, `block.scale`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (path, &mut self.division) {
            ("division.length", Division::Step { length }) => Some(length),
            ("start", _) => self.start.as_mut(),
            ("block.rotation", _) => self.block.as_mut().map(|b| &mut b.rotation),
            ("block.scale", _) => self.block.as_mut().map(|b| &mut b.scale),
            ("lineWeight", _) => self.look.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for DimensionsCreate {
    /// `at.x`, `offset`, `height`, `lineWeight`, `measure.a.at.x`, `measure.angle`,
    /// `measure.za`, `measure.of.near.y`, `measure.between[1].x`, `measure.shownCentre.x`, `measure.on.y` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        fn xy<'a>(p: &'a mut kentos_commands::Vec2, rest: &str) -> Option<&'a mut f64> {
            match rest {
                "x" => Some(&mut p.x),
                "y" => Some(&mut p.y),
                _ => None,
            }
        }
        match path {
            "offset" => return self.offset.as_mut(),
            "height" => return self.height.as_mut(),
            "lineWeight" => return self.look.line_weight.as_mut(),
            _ => {}
        }
        if let Some(rest) = path.strip_prefix("at.") {
            return xy(&mut self.at, rest);
        }
        let rest = path.strip_prefix("measure.")?;
        let (field, rest) = rest.split_once('.').unwrap_or((rest, ""));
        fn point(p: &mut MeasuredPoint) -> Option<&mut kentos_commands::Vec2> {
            match p {
                MeasuredPoint::At { at } => Some(at),
                MeasuredPoint::Place { .. } => None,
            }
        }
        match (&mut self.measure, field) {
            (DimensionMeasure::Aligned { a, .. } | DimensionMeasure::Azimuth { a, .. }, "a")
            | (DimensionMeasure::Linear { a, .. } | DimensionMeasure::Slope { a, .. }, "a")
            | (
                DimensionMeasure::Ordinate { a, .. } | DimensionMeasure::AngleVertex { a, .. },
                "a",
            ) => xy(point(a)?, rest.strip_prefix("at.")?),
            (DimensionMeasure::Aligned { b, .. } | DimensionMeasure::Azimuth { b, .. }, "b")
            | (DimensionMeasure::Linear { b, .. } | DimensionMeasure::Slope { b, .. }, "b")
            | (DimensionMeasure::AngleVertex { b, .. }, "b") => {
                xy(point(b)?, rest.strip_prefix("at.")?)
            }
            (DimensionMeasure::AngleVertex { c, .. }, "c") => {
                xy(point(c)?, rest.strip_prefix("at.")?)
            }
            (
                DimensionMeasure::Linear { angle, .. } | DimensionMeasure::Ordinate { angle, .. },
                "angle",
            ) => angle.as_mut(),
            (DimensionMeasure::Slope { za, .. }, "za") => za.as_mut(),
            (DimensionMeasure::Slope { zb, .. }, "zb") => zb.as_mut(),
            (DimensionMeasure::Elevation { z, .. }, "z") => z.as_mut(),
            (
                DimensionMeasure::Elevation {
                    level: Some(level), ..
                },
                "level",
            ) => match rest {
                "offset" => level.offset.as_mut(),
                _ => None,
            },
            (DimensionMeasure::Elevation { a, .. }, "a") => {
                xy(point(a)?, rest.strip_prefix("at.")?)
            }
            (DimensionMeasure::Angle { first, .. }, "first") => {
                xy(&mut first.near, rest.strip_prefix("near.")?)
            }
            (DimensionMeasure::Angle { second, .. }, "second") => {
                xy(&mut second.near, rest.strip_prefix("near.")?)
            }
            (
                DimensionMeasure::AngleArc { of }
                | DimensionMeasure::Radius { of }
                | DimensionMeasure::Diameter { of }
                | DimensionMeasure::ArcLength { of, .. }
                | DimensionMeasure::Jogged { of, .. },
                "of",
            ) => xy(&mut of.near, rest.strip_prefix("near.")?),
            (
                DimensionMeasure::ArcLength {
                    between: Some([p, q]),
                    ..
                },
                f,
            ) if f.starts_with("between[") => match f {
                "between[0]" => xy(p, rest),
                "between[1]" => xy(q, rest),
                _ => None,
            },
            (DimensionMeasure::Jogged { shown_centre, .. }, "shownCentre") => {
                xy(shown_centre, rest)
            }
            (DimensionMeasure::Jogged { on, .. }, "on") => xy(on, rest),
            _ => None,
        }
    }
}

impl Input for EntitiesBoundary {
    /// `at.x`, `at.y`, `boundaries.min.x` …, `lineWeight`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "lineWeight" => self.look.line_weight.as_mut(),
            _ => area_number(&mut self.boundaries, path.strip_prefix("boundaries.")?),
        }
    }
}

impl Input for EntitiesHatch {
    /// `pattern.scale`, `pattern.angle`, `region.at.x`, `region.boundaries.min.x` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "pattern.scale" => return self.pattern.scale.as_mut(),
            "pattern.angle" => return self.pattern.angle.as_mut(),
            "lineWeight" => return self.look.line_weight.as_mut(),
            // docs/adr/0331 §2.
            "pattern.origin.at.x" | "pattern.origin.at.y" => {
                let Some(kentos_commands::HatchOrigin::Point { at }) = &mut self.pattern.origin
                else {
                    return None;
                };
                return Some(if path.ends_with('x') {
                    &mut at.x
                } else {
                    &mut at.y
                });
            }
            _ => {}
        }
        let HatchRegion::Point {
            at,
            boundaries,
            also_at,
            gap,
            ..
        } = &mut self.region
        else {
            return None;
        };
        match path {
            "region.at.x" => Some(&mut at.x),
            "region.at.y" => Some(&mut at.y),
            "region.gap" => gap.as_mut(),
            _ if path.starts_with("region.alsoAt[") => point_number(
                also_at,
                path.strip_prefix("region.alsoAt")?
                    .replacen('[', "pts[", 1)
                    .as_str(),
            ),
            _ => area_number(boundaries, path.strip_prefix("region.boundaries.")?),
        }
    }
}

impl Input for EntitiesFillet {
    /// `radius` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "radius").then_some(&mut self.radius)
    }
}

impl Input for EntitiesChamfer {
    /// `distance` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "distance").then_some(&mut self.distance)
    }
}

impl Input for DimensionsReassociate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::DimensionsEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::DimensionEdit as E;
        match (&mut self.edit, path) {
            (
                E::Space {
                    spacing: Some(s), ..
                },
                "edit.spacing",
            ) => Some(s),
            (E::MoveText { along, .. }, "edit.along") => Some(along),
            (E::MoveText { across, .. }, "edit.across") => Some(across),
            (E::Level { level: Some(l), .. }, "edit.level.offset") => l.offset.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::LeadersEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::LeaderEdit;
        match (&mut self.edit, path) {
            (LeaderEdit::Align { angle, .. }, "edit.angle") => Some(angle),
            (
                LeaderEdit::Align {
                    spacing: Some(s), ..
                },
                "edit.spacing",
            ) => Some(s),
            _ => None,
        }
    }
}

impl Input for kentos_commands::HatchesEdit {
    /// `edit.region.at.x`, `edit.region.at.y` (Sınırı yeniden seç).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::{HatchEdit, HatchRegion};
        let HatchEdit::Reselect {
            region: HatchRegion::Point { at, .. },
            ..
        } = &mut self.edit
        else {
            return None;
        };
        match path {
            "edit.region.at.x" => Some(&mut at.x),
            "edit.region.at.y" => Some(&mut at.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::TablesEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::{TableEdit, TablesOperation};
        let TablesOperation::Edit { edits, .. } = &mut self.operation else {
            return None;
        };
        // `operation.edits[i].height` or `.width`.
        let rest = path.strip_prefix("operation.edits[")?;
        let (i, field) = rest.split_once("].")?;
        match (edits.get_mut(i.parse::<usize>().ok()?)?, field) {
            (TableEdit::RowHeight { height, .. }, "height") => Some(height),
            (TableEdit::ColumnWidth { width, .. }, "width") => Some(width),
            (TableEdit::Frame { width: Some(w) }, "width") => Some(w),
            _ => None,
        }
    }
}

impl Input for kentos_commands::TextsEdit {
    /// `edit.at.x` (Doğrultuya döndür's click; the shared cases spoil only it).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (&mut self.edit, path) {
            (kentos_commands::TextEdit::Turn { at, .. }, "edit.at.x") => Some(&mut at.x),
            _ => None,
        }
    }
}

impl Input for kentos_commands::TextsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::FontsReport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::DimensionsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LabelsPin {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::LabelsReport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LibraryEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for StylesImpact {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LineTypesEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "scale").then_some(())?;
        self.scale.as_mut()
    }
}

impl Input for HatchPatternsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for StandardsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for StandardsApply {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for StylesEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for SheetsCreate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for SheetsFit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for SheetsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ChangesetApply {
    /// A step's numbers cross as JSON, which carries no NaN: none here.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for SettingsSet {
    /// `plotScale` (which the step's input must give, docs/adr/0248 §5).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "plotScale" => self.plot_scale.as_mut(),
            _ => None,
        }
    }
}

impl Input for UcsSet {
    /// `to.angle`; a point's `x` or `y` (`to.origin.x`, `to.point.y`, `to.xPoint.x`, `to.at.y`);
    /// the command's errors name the point (docs/adr/0261 §2).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "to.angle" {
            return match &mut self.to {
                UcsTo::Frame { angle, .. } | UcsTo::Rotate { angle } => Some(angle),
                _ => None,
            };
        }
        let (point, axis) = path.rsplit_once('.')?;
        let p = match (&mut self.to, point) {
            (UcsTo::Frame { origin: p, .. }, "to.origin")
            | (UcsTo::Origin { point: p }, "to.point")
            | (UcsTo::TwoPoints { origin: p, .. }, "to.origin")
            | (UcsTo::TwoPoints { x_point: p, .. }, "to.xPoint")
            | (UcsTo::Object { at: p, .. }, "to.at") => p,
            _ => return None,
        };
        match axis {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for UnitsChange {
    /// No numbers: two units and a choice (docs/adr/0256 §1).
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ImportDxf {
    /// No number: a path, names and choices.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportCsv {
    /// No number: a path, a layer and column names.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportGml {
    /// No number: a path, feature types and layers.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportShp {
    /// No number: a path, an entry, layers and a system.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportGeojson {
    /// No number: a path, layers and a system.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportKml {
    /// No number: a path, layers and a system.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportFgb {
    /// No number: a path and a layer.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportParquet {
    /// No number: a path, a layer and the merge's words.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ImportPdf {
    /// The numbers a case may give through a placeholder: none, the file's are literal.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ImportGpkg {
    /// No number: a path, table names and a choice.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesApply {
    /// No number to move: a profile.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesImport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesList {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesMigrate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesRemove {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::EntitiesGrade {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "start" => self.start.as_mut(),
            "end" => self.end.as_mut(),
            "grade" => self.grade.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesDrape {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "offset" => self.offset.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::EntitiesRaise {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "grade" => self.grade.as_mut(),
            "difference" => self.difference.as_mut(),
            "reference.z" => Some(&mut self.reference.z),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ProfilesCompare {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesCarry {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesNames {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportLandxml {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for EntitiesOffset {
    /// `by.distance`, `by.side.x`, `by.point.y` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "approximation.tolerance" {
            return self.approximation.as_mut().map(|a| &mut a.tolerance);
        }
        match (&mut self.by, path) {
            (OffsetBy::Distance { distance, .. }, "by.distance") => Some(distance),
            (OffsetBy::Distance { side: p, .. }, _) | (OffsetBy::Through { point: p }, _) => {
                match path
                    .strip_prefix("by.side.")
                    .or(path.strip_prefix("by.point."))?
                {
                    "x" => Some(&mut p.x),
                    "y" => Some(&mut p.y),
                    _ => None,
                }
            }
        }
    }
}

impl Input for BlocksEdit {
    /// `base.x`, `base.y`; `attributes[0].p.x`, `attributes[1].rotation`,
    /// `attributes[2].height` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let Some(rest) = path.strip_prefix("attributes[") else {
            return coordinate(self.base.as_mut()?, "base", path);
        };
        let (i, field) = rest.split_once("].")?;
        let a = self
            .attributes
            .as_mut()?
            .get_mut(i.parse::<usize>().ok()?)?;
        match field {
            "rotation" => Some(&mut a.rotation),
            "height" => Some(&mut a.height),
            _ => coordinate(&mut a.p, "p", field),
        }
    }
}

impl Input for kentos_commands::BlocksReplace {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::BlocksQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::BlocksParameters {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::BlocksProxy {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::BlocksFreeze {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportBlock {
    /// No number to make not finite.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::BlocksInsert {
    /// `at.x`, `at.y`, `rotation`, `scale`, `yScale` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "rotation" => self.rotation.as_mut(),
            "scale" => self.scale.as_mut(),
            "yScale" => self.y_scale.as_mut(),
            _ => coordinate(&mut self.at, "at", path),
        }
    }
}

impl Input for EntitiesSetProperties {
    /// `lineWeight` (which the step's input must give); the rest is ids and texts.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "lineWeight" => self.line_weight.as_mut()?.as_mut(),
            "lineTypeScale" => self.line_type_scale.as_mut()?.as_mut(),
            "transparency" => self.transparency.as_mut()?.as_mut(),
            _ => None,
        }
    }
}

impl Input for PointCreate {
    /// `p.x`, `p.y`, `z` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "z" => self.z.as_mut(),
            _ => coordinate(&mut self.p, "p", path),
        }
    }
}

impl Input for CircleCreate {
    /// `c.x`, `c.y`, `r`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            "lineWeight" => self.line_weight.as_mut(),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

impl Input for ArcCreate {
    /// `c.x`, `c.y`, `r`, `a0`, `a1`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            "lineWeight" => self.line_weight.as_mut(),
            "a0" => Some(&mut self.a0),
            "a1" => Some(&mut self.a1),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

impl Input for EntitiesTransform {
    /// `transform.dx`, `transform.center.x`, `transform.angle`, `transform.factor`, `transform.a.y` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let rest = path.strip_prefix("transform.")?;
        match &mut self.transform {
            Transform::Move { dx, dy } => match rest {
                "dx" => Some(dx),
                "dy" => Some(dy),
                _ => None,
            },
            Transform::Rotate { center, angle } => match rest {
                "angle" => Some(angle),
                _ => coordinate(center, "center", rest),
            },
            Transform::Scale { center, factor } => match rest {
                "factor" => Some(factor),
                _ => coordinate(center, "center", rest),
            },
            Transform::Mirror { a, b, .. } => {
                coordinate(a, "a", rest).or_else(|| coordinate(b, "b", rest))
            }
            Transform::Align {
                source,
                target,
                source2,
                target2,
                ..
            } => coordinate(source, "source", rest)
                .or_else(|| coordinate(target, "target", rest))
                .or_else(|| coordinate(source2.as_mut()?, "source2", rest))
                .or_else(|| coordinate(target2.as_mut()?, "target2", rest)),
            // Oturt (docs/adr/0156): `transform.from.x`, `transform.a`, `transform.m[2]`, `transform.h[6]` …
            Transform::Similarity { from, to, a, b } => match rest {
                "a" => Some(a),
                "b" => Some(b),
                _ => coordinate(from, "from", rest).or_else(|| coordinate(to, "to", rest)),
            },
            Transform::Affine { from, to, m } => indexed(m, "m", rest)
                .or_else(|| coordinate(from, "from", rest))
                .or_else(|| coordinate(to, "to", rest)),
            Transform::Projective { from, to, h } => indexed(h, "h", rest)
                .or_else(|| coordinate(from, "from", rest))
                .or_else(|| coordinate(to, "to", rest)),
            // Kauçuk levha (docs/adr/0158): `transform.links[1].to.x` …
            Transform::Rubbersheet { links } => {
                let (at, field) = rest.strip_prefix("links[")?.split_once("].")?;
                let link = links.get_mut(at.parse::<usize>().ok()?)?;
                coordinate(&mut link.from, "from", field)
                    .or_else(|| coordinate(&mut link.to, "to", field))
            }
            // Hizala ve dağıt (docs/adr/0194): `transform.at`.
            Transform::Arrange { at, .. } => match rest {
                "at" => at.as_mut(),
                _ => None,
            },
        }
    }
}

impl Input for EntitiesArray {
    /// `layout.dx`, `layout.dy`, `layout.center.x`, `layout.fill`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let rest = path.strip_prefix("layout.")?;
        match &mut self.layout {
            ArrayLayout::Grid { dx, dy, .. } | ArrayLayout::Line { dx, dy, .. } => match rest {
                "dx" => Some(dx),
                "dy" => Some(dy),
                _ => None,
            },
            ArrayLayout::Lattice {dx,dy,column_angle,row_angle,..} => match rest {
                "dx"=>Some(dx),"dy"=>Some(dy),"columnAngle"=>Some(column_angle),"rowAngle"=>Some(row_angle),_=>None,
            },
            ArrayLayout::Polar { center, fill, .. } => match rest {
                "fill" => Some(fill),
                _ => coordinate(center, "center", rest),
            },
            ArrayLayout::Path { spacing, .. } => match rest {
                "spacing" => spacing.as_mut(),
                _ => None,
            },
        }
    }
}

impl Input for EntitiesEdit {
    /// `changes[0].geometry.a.x`, `changes[1].geometry.r`, `changes[0].geometry.pts[1].y`,
    /// `changes[2].geometry.bulges[0]` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        // An add's own line weight (docs/adr/0144): `changes[0].lineWeight`.
        if let Some(i) = path
            .strip_prefix("changes[")
            .and_then(|rest| rest.strip_suffix("].lineWeight"))
        {
            return match self.changes.get_mut(i.parse::<usize>().ok()?)? {
                EntityEdit::Add { line_weight, .. } => line_weight.as_mut(),
                _ => None,
            };
        }
        let (i, rest) = path.strip_prefix("changes[")?.split_once("].geometry.")?;
        let geometry = match self.changes.get_mut(i.parse::<usize>().ok()?)? {
            EntityEdit::Update { geometry, .. }
            | EntityEdit::Replace { geometry, .. }
            | EntityEdit::Add { geometry, .. } => geometry,
            EntityEdit::Remove { .. } | EntityEdit::EllipseAxis { .. } => return None,
        };
        geometry_number(geometry, rest)
    }
}

impl Input for EntitiesCreate {
    /// `objects[0].geometry.major.x`, `objects[1].geometry.pts[2].y`, `objects[0].geometry.ratio`,
    /// `objects[1].lineWeight`, `objects[0].labelScale` (which the step's input must give) …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let (i, rest) = path.strip_prefix("objects[")?.split_once("].")?;
        let object = self.objects.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "lineWeight" => object.line_weight.as_mut(),
            "labelScale" => object.label_scale.as_mut(),
            _ => geometry_number(&mut object.geometry, rest.strip_prefix("geometry.")?),
        }
    }
}
