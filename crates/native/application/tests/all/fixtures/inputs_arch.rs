//! The arch commands' inputs with the numbers a fixture's `nonFinite` may name (docs/adr/8054 §9).

#[allow(unused_imports)]
use super::*;

impl Input for kentos_commands::ReviewNotesList {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ReviewNotesEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (&mut self.change, path) {
            (kentos_commands::ReviewNoteOp::Add { at: Some(p), .. }, "change.at.x") => Some(&mut p.x),
            (kentos_commands::ReviewNoteOp::Add { at: Some(p), .. }, "change.at.y") => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::WallsCreate {
    /// `walls[0].pts[1].y`, `walls[0].bulges[0]`, `walls[0].width`, `.offset`, `.height`, `lineWeight` (docs/adr/0339).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("walls[")?.split_once("].")?;
        let w = self.walls.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "width" => w.width.as_mut(),
            "offset" => w.offset.as_mut(),
            "height" => w.height.as_mut(),
            _ => named_point(&mut w.pts, "pts", rest)
                .or_else(|| list_number(w.bulges.as_deref_mut()?, "bulges", rest)),
        }
    }
}

impl Input for kentos_commands::WallsPartition {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "distance" { return Some(&mut self.distance); }
        if let Some(rest) = path.strip_prefix("wall.") { return self.wall.number(rest); }
        if let kentos_commands::PartitionExtent::Length {anchor,length,..} = &mut self.extent {
            match path { "extent.anchor" => Some(anchor), "extent.length" => Some(length), _ => None }
        } else { None }
    }
}

impl Input for kentos_commands::WallsRectangle {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::RectangleFrame as F;
        match path {
            "width" => return self.width.as_mut(),
            "height" => return self.height.as_mut(),
            "lineWeight" => return self.line_weight.as_mut(),
            "rotation" => return Some(self.rotation.get_or_insert(0.0)),
            _ => {}
        }
        let field = path.strip_prefix("frame.")?;
        match &mut self.frame {
            F::Corners { a, b } => coordinate(a, "a", field).or_else(|| coordinate(b, "b", field)),
            F::Edge { a, b, width } => match field {
                "width" => Some(width),
                _ => coordinate(a, "a", field).or_else(|| coordinate(b, "b", field)),
            },
            F::Size { corner, length, width, toward } => match field {
                "length" => Some(length),
                "width" => Some(width),
                _ => coordinate(corner, "corner", field).or_else(|| coordinate(toward, "toward", field)),
            },
            F::Area { corner, area, length, toward } => match field {
                "area" => Some(area),
                "length" => Some(length),
                _ => coordinate(corner, "corner", field).or_else(|| coordinate(toward, "toward", field)),
            },
        }
    }
}

impl Input for kentos_commands::WallsFromLines {
    /// `width`, `maxWidth`, `angle`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "width" => self.width.as_mut(),
            "maxWidth" => self.max_width.as_mut(),
            "angle" => self.angle.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ArchRecognize {
    /// `minWidth`, `maxWidth`, `maxOpening`, `angle`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "minWidth" => self.min_width.as_mut(),
            "maxWidth" => self.max_width.as_mut(),
            "maxOpening" => self.max_opening.as_mut(),
            "angle" => self.angle.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::SpacesNumber {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let kentos_commands::SpaceNumberOrder::Direction {from,to} = &mut self.order else {return None};
        let path = path.strip_prefix("order.")?;
        coordinate(from,"from",path).or_else(||coordinate(to,"to",path))
    }
}

impl Input for kentos_commands::SpacesDiscover {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        named_point(self.bounds.as_mut()?, "bounds", path)
    }
}

impl Input for kentos_commands::SpacesSeparators {
    fn number(&mut self,path:&str)->Option<&mut f64> {
        let kentos_commands::SpaceSeparatorOperation::Create {paths,line_weight,..}=&mut self.operation else{return None};
        if path=="operation.lineWeight" {return line_weight.as_mut();}
        let (i,rest)=path.strip_prefix("operation.paths[")?.split_once("].")?;
        let p=paths.get_mut(i.parse::<usize>().ok()?)?;
        if rest.starts_with("pts["){return named_point(&mut p.pts,"pts",rest);}
        let i=rest.strip_prefix("bulges[")?.strip_suffix(']')?.parse::<usize>().ok()?;
        p.bulges.as_mut()?.get_mut(i)
    }
}

impl Input for kentos_commands::SpacesCreate {
    /// `spaces[0].seed.x` and `.y`, `.height`, `.floorOffset`, `.suspendedCeiling`, `lineWeight` (docs/adr/0342,
    /// 3040 §3).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("spaces[")?.split_once("].")?;
        let s = self.spaces.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "seed.x" => s.seed.as_mut().map(|p| &mut p.x),
            "seed.y" => s.seed.as_mut().map(|p| &mut p.y),
            "height" => s.height.as_mut(),
            "floorOffset" => s.floor_offset.as_mut(),
            "suspendedCeiling" => s.suspended_ceiling.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::SpacesEdit {
    /// Değiştir's `operation.height`, `.floorOffset`, `.suspendedCeiling`; Böl's `operation.line[k].x` and `.y`;
    /// Duvarlardan bul's `operation.seed.x`, `.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::SpacesOperation as Op;
        match &mut self.operation {
            Op::Set(c)
                if matches!(
                    path,
                    "operation.height" | "operation.floorOffset" | "operation.suspendedCeiling"
                ) =>
            {
                match path {
                    "operation.height" => c.height.as_mut()?.as_mut(),
                    "operation.floorOffset" => c.floor_offset.as_mut()?.as_mut(),
                    _ => c.suspended_ceiling.as_mut()?.as_mut(),
                }
            }
            Op::Split { keep: Some(p), .. } if path.starts_with("operation.keep.") => match path {
                "operation.keep.x" => Some(&mut p.x),
                "operation.keep.y" => Some(&mut p.y),
                _ => None,
            },
            Op::Split { line, .. } => named_point(line, "operation.line", path),
            Op::Rebound { seed } => match path {
                "operation.seed.x" => seed.as_mut().map(|p| &mut p.x),
                "operation.seed.y" => seed.as_mut().map(|p| &mut p.y),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Input for kentos_commands::SlabsCreate {
    /// `slabs[0].seed.x` and `.y`, `.parts[k].outer.pts[j].x` and `.y` (a hole's `.holes[h]`), `.thickness`,
    /// `.offset`, `.edgeOffset`, `.edge.width`, `lineWeight` (docs/adr/0722).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("slabs[")?.split_once("].")?;
        let s = self.slabs.get_mut(i.parse::<usize>().ok()?)?;
        if let Some(rest) = rest.strip_prefix("parts[") {
            let (k, rest) = rest.split_once("].")?;
            let part = s.parts.as_mut()?.get_mut(k.parse::<usize>().ok()?)?;
            let (ring, rest) = match rest.strip_prefix("outer.") {
                Some(rest) => (&mut part.outer, rest),
                None => {
                    let (h, rest) = rest.strip_prefix("holes[")?.split_once("].")?;
                    (part.holes.get_mut(h.parse::<usize>().ok()?)?, rest)
                }
            };
            return named_point(&mut ring.pts, "pts", rest);
        }
        match rest {
            "seed.x" => s.seed.as_mut().map(|p| &mut p.x),
            "seed.y" => s.seed.as_mut().map(|p| &mut p.y),
            "thickness" => s.thickness.as_mut(),
            "offset" => s.offset.as_mut(),
            "edgeOffset" => s.edge_offset.as_mut(),
            "edge.width" => s.edge.as_mut()?.width.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::SlabsEdit {
    /// Değiştir's `operation.thickness`, `.offset`, `.edge.width`; Ötele's `operation.distance`; Kes's and Ekle's
    /// `operation.seed.x`, `.y` (docs/adr/0722).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::SlabsOperation as Op;
        match &mut self.operation {
            Op::Set(c) => match path {
                "operation.thickness" => c.thickness.as_mut(),
                "operation.offset" => c.offset.as_mut(),
                "operation.edge.width" => c.edge.as_mut()?.as_mut()?.width.as_mut(),
                _ => None,
            },
            Op::Offset { distance } => (path == "operation.distance").then_some(distance),
            Op::Subtract { seed, .. } | Op::Add { seed, .. } => match path {
                "operation.seed.x" => seed.as_mut().map(|p| &mut p.x),
                "operation.seed.y" => seed.as_mut().map(|p| &mut p.y),
                _ => None,
            },
            // Kaynağa bağla (docs/adr/0771 §4).
            Op::Bind { seed, offset, .. } => match path {
                "operation.seed.x" => seed.as_mut().map(|p| &mut p.x),
                "operation.seed.y" => seed.as_mut().map(|p| &mut p.y),
                "operation.offset" => offset.as_mut(),
                _ => None,
            },
            Op::Detach => None,
        }
    }
}

impl Input for kentos_commands::RoofsCreate {
    /// `roofs[0].offset`, `.thickness`, `.overhang`, `.slopes[k]`, `.pts[k].x` and `.y`, `lineWeight` (docs/adr/8044 §1);
    /// its parts' (§6).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("roofs[")?.split_once("].")?;
        let r = self.roofs.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "offset" => r.offset.as_mut(),
            "thickness" => r.thickness.as_mut(),
            "overhang" => r.overhang.as_mut(),
            "gutter" => r.gutter.as_mut(),
            "snowGuard" => r.snow_guard.as_mut(),
            _ if rest.starts_with("parts[") => roof_part_number(r.parts.as_mut(), rest),
            _ => match rest
                .strip_prefix("slopes[")
                .and_then(|k| k.strip_suffix(']'))
            {
                Some(k) => r.slopes.get_mut(k.parse::<usize>().ok()?)?.as_mut(),
                None => named_point(&mut r.pts, "pts", rest),
            },
        }
    }
}

impl Input for kentos_commands::TagsCreate {
    /// `tags[0].at.x` and `.y`, `tags[0].size`, `lineWeight` (docs/adr/0344).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("tags[")?.split_once("].")?;
        let t = self.tags.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "at.x" => t.at.as_mut().map(|p| &mut p.x),
            "at.y" => t.at.as_mut().map(|p| &mut p.y),
            "size" => t.size.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::TagsEdit {
    /// `change.size` (docs/adr/0344).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "change.size" => self.change.size.as_mut().and_then(Option::as_mut),
            _ => None,
        }
    }
}

impl Input for kentos_commands::TagsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ViewMarksCreate {
    /// `marks[0].points[k].x` and `.y`, `.at.x`, `.radius`, `.angle`, `.size`, `lineWeight` (docs/adr/0345).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("marks[")?.split_once("].")?;
        let m = self.marks.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "radius" => m.radius.as_mut(),
            "angle" => m.angle.as_mut(),
            "size" => m.size.as_mut(),
            "at.x" => m.at.as_mut().map(|p| &mut p.x),
            "at.y" => m.at.as_mut().map(|p| &mut p.y),
            _ => named_point(&mut m.points, "points", rest),
        }
    }
}

impl Input for kentos_commands::ViewMarksEdit {
    /// `operation.size`, `.angle`, `.radius` (docs/adr/0345).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let kentos_commands::ViewMarksOperation::Set(c) = &mut self.operation else {
            return None;
        };
        match path {
            "operation.size" => c.size.as_mut().and_then(Option::as_mut),
            "operation.angle" => c.angle.as_mut(),
            "operation.radius" => c.radius.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ViewMarksReferences {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TemplatesApply {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SpacesFinish {
    /// `scale`, `grid.dx`, `.dy`, `.angle` (docs/adr/0345).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "scale" => self.scale.as_mut(),
            "grid.dx" => self.grid.as_mut().map(|g| &mut g.dx),
            "grid.dy" => self.grid.as_mut().map(|g| &mut g.dy),
            "grid.angle" => self.grid.as_mut().map(|g| &mut g.angle),
            _ => None,
        }
    }
}

impl Input for kentos_commands::SchedulesCreate {
    /// `at.x` and `.y`, `height`, `lineWeight` (docs/adr/0344).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "height" => self.height.as_mut(),
            "lineWeight" => self.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::SchedulesQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SchedulesEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ArchEdit {
    /// Değiştir's `operation.width`, `.riserHeight`, `.tread`, `.rise` (docs/adr/0343), `.baseOffset` (docs/adr/0723),
    /// `.post` (docs/adr/0724).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::ArchOperation as Op;
        match &mut self.operation {
            Op::Set(c) => match path {
                "operation.width" => c.width.as_mut(),
                "operation.riserHeight" => c.riser_height.as_mut(),
                "operation.tread" => c.tread.as_mut(),
                "operation.rise" => c.rise.as_mut(),
                "operation.baseOffset" => c.base_offset.as_mut().and_then(Option::as_mut),
                "operation.post" => c.post.as_mut().and_then(Option::as_mut),
                // A gapped grid's ends, an L's legs (docs/adr/0725 §3, §5).
                "operation.ends" => c.ends.as_mut().and_then(Option::as_mut),
                "operation.legs[0]" => c.legs.as_mut().map(|l| &mut l[0]),
                "operation.legs[1]" => c.legs.as_mut().map(|l| &mut l[1]),
                // A beam's (docs/adr/8041 §1).
                "operation.height" => c.height.as_mut(),
                "operation.offset" => c.offset.as_mut(),
                // A roof's (docs/adr/8044 §1, §6).
                "operation.thickness" => c.thickness.as_mut().and_then(Option::as_mut),
                "operation.overhang" => c.overhang.as_mut().and_then(Option::as_mut),
                "operation.gutter" => c.gutter.as_mut().and_then(Option::as_mut),
                "operation.snowGuard" => c.snow_guard.as_mut().and_then(Option::as_mut),
                // A railing's (docs/adr/8045 §3).
                "operation.gap" => c.gap.as_mut().and_then(Option::as_mut),
                _ => {
                    let rest = path.strip_prefix("operation.")?;
                    if rest.starts_with("parts[") {
                        return roof_part_number(c.parts.as_mut(), rest);
                    }
                    let k = rest.strip_prefix("slopes[")?.strip_suffix(']')?;
                    c.slopes
                        .as_mut()?
                        .get_mut(k.parse::<usize>().ok()?)?
                        .as_mut()
                }
            },
            // Mesafe's `operation.distance` (docs/adr/0712).
            Op::Distance { distance, .. } if path == "operation.distance" => Some(distance),
            _ => None,
        }
    }
}

impl Input for kentos_commands::SpacesQuantities {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OpeningsCreate {
    /// `openings[0].at`, `.distance`, `.width`, `.swing`, `.sill`, `.head`, `.depth`, `.inset`, `.point.x` and `.y`,
    /// `lineWeight` (docs/adr/0341, 0719).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("openings[")?.split_once("].")?;
        let o = self.openings.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "at" => o.at.as_mut(),
            "distance" => o.distance.as_mut(),
            "width" => o.width.as_mut(),
            "swing" => o.swing.as_mut(),
            "sill" => o.sill.as_mut(),
            "head" => o.head.as_mut(),
            "depth" => o.depth.as_mut(),
            "inset" => o.inset.as_mut(),
            "point.x" => o.point.as_mut().map(|p| &mut p.x),
            "point.y" => o.point.as_mut().map(|p| &mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::OpeningsEdit {
    /// Değiştir's `operation.width`, `.swing`, `.frame`, `.sill`, `.head`, `.depth`, `.inset`; Taşı's `operation.by` and `.at`; Başka
    /// duvara's `operation.point.x` and `.y`; Kopyala's `operation.offsets[k]` (docs/adr/0341).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::OpeningsOperation as Op;
        match &mut self.operation {
            Op::Set(c) => match path {
                "operation.width" => c.width.as_mut(),
                "operation.swing" => c.swing.as_mut()?.as_mut(),
                "operation.frame" => c.frame.as_mut()?.as_mut(),
                "operation.sill" => c.sill.as_mut()?.as_mut(),
                "operation.head" => c.head.as_mut()?.as_mut(),
                "operation.depth" => c.depth.as_mut()?.as_mut(),
                "operation.inset" => c.inset.as_mut()?.as_mut(),
                _ => None,
            },
            Op::Move { by, at } => match path {
                "operation.by" => by.as_mut(),
                "operation.at" => at.as_mut(),
                _ => None,
            },
            Op::Rehost { point, .. } => match path {
                "operation.point.x" => point.as_mut().map(|p| &mut p.x),
                "operation.point.y" => point.as_mut().map(|p| &mut p.y),
                _ => None,
            },
            Op::Copy { offsets } => list_number(offsets, "operation.offsets", path),
            // Mesafeyle taşı's `operation.distance` (docs/adr/0701).
            Op::Distance { distance, .. } => (path == "operation.distance").then_some(distance),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ProgramEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProgramCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::LevelsAssign {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::NorthCreate {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (path, &mut self.north) {
            ("at.x", _) => Some(&mut self.at.x),
            ("at.y", _) => Some(&mut self.at.y),
            ("north.angle", kentos_commands::NorthDirection::Angle { angle }) => Some(angle),
            ("size", _) => self.size.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::RevisionsMark {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => self.at.as_mut().map(|p| &mut p.x),
            "at.y" => self.at.as_mut().map(|p| &mut p.y),
            "size" => self.size.as_mut(),
            "arc" => Some(&mut self.arc),
            _ => None,
        }
    }
}

impl Input for kentos_commands::LegendsMaterials {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => self.at.as_mut().map(|p| &mut p.x),
            "at.y" => self.at.as_mut().map(|p| &mut p.y),
            "height" => self.height.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::WallsDistancesInput {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OpeningsDistancesInput {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ElevationsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ElevationsEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (&mut self.operation, path) {
            (kentos_commands::ElevationsOperation::Derive { gap: Some(g), .. }, "operation.gap") => Some(g),
            _ => None,
        }
    }
}

impl Input for kentos_commands::BuildingSectionsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::BuildingSectionsEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (&mut self.operation, path) {
            (kentos_commands::BuildingSectionsOperation::Derive { at: Some(p), .. }, "operation.at.x") => Some(&mut p.x),
            (kentos_commands::BuildingSectionsOperation::Derive { at: Some(p), .. }, "operation.at.y") => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::OptionsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OptionsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ModulesQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ModulesEdit {
    /// A placement's point and turn, a definition's base (docs/adr/0775).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (&mut self.operation, path) {
            (kentos_commands::ModulesOperation::Place { at, .. }, "operation.at.x") => Some(&mut at.x),
            (kentos_commands::ModulesOperation::Place { at, .. }, "operation.at.y") => Some(&mut at.y),
            (kentos_commands::ModulesOperation::Place { rotation, .. }, "operation.rotation") => rotation.as_mut(),
            (kentos_commands::ModulesOperation::Define { base, .. }, "operation.base.x") => Some(&mut base.x),
            _ => None,
        }
    }
}

impl Input for kentos_commands::OpeningsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::MaterialsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::OpeningTypesEdit {
    /// A define's or an update's change's numbers (docs/adr/0772).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let c = match &mut self.operation {
            kentos_commands::OpeningTypesOperation::Define { change, .. } => change,
            kentos_commands::OpeningTypesOperation::Update { change, .. } => change,
            _ => return None,
        };
        match path.strip_prefix("operation.change.")? {
            "width" => c.width.as_mut(),
            "frame" => c.frame.as_mut()?.as_mut(),
            "head" => c.head.as_mut()?.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::CaseworkRun {
    /// `from.x` and `.y`, `through[i].x` and `.y`, `to.x` and `.y`, `module`, `depth`, `perModule[i].width`,
    /// `lineWeight` (docs/adr/0706, 0727).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if let Some(rest) = path.strip_prefix("through[") {
            let (i, axis) = rest.split_once("].")?;
            let p = self.through.as_mut()?.get_mut(i.parse::<usize>().ok()?)?;
            return match axis {
                "x" => Some(&mut p.x),
                "y" => Some(&mut p.y),
                _ => None,
            };
        }
        if let Some(rest) = path.strip_prefix("perModule[") {
            let i = rest.strip_suffix("].width")?;
            return self.per_module.as_mut()?.get_mut(i.parse::<usize>().ok()?)?.width.as_mut();
        }
        match path {
            "from.x" => Some(&mut self.from.x),
            "from.y" => Some(&mut self.from.y),
            "to.x" => Some(&mut self.to.x),
            "to.y" => Some(&mut self.to.y),
            "module" => self.module.as_mut(),
            "depth" => self.depth.as_mut(),
            "lineWeight" => self.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::CaseworkEdit {
    /// `edit.width` of a module edit; a recipe's numbers after `edit.recipe.` (docs/adr/0727 §6).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match &mut self.edit {
            kentos_commands::CaseworkChange::Module { width, .. } if path == "edit.width" => width.as_mut(),
            kentos_commands::CaseworkChange::Recipe { recipe } => {
                recipe.number(path.strip_prefix("edit.recipe.")?)
            }
            _ => None,
        }
    }
}

impl Input for kentos_commands::CaseworkQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ArchExceptions {
    /// The finding audit's numbers after `audit.` (docs/adr/0732 §3).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        self.audit.as_mut()?.number(path.strip_prefix("audit.")?)
    }
}

impl Input for kentos_commands::ArchAudit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "maxGap" => self.max_gap.as_mut(),
            "minArea" => self.min_area.as_mut(),
            "minWidth" => self.min_width.as_mut(),
            "minLength" => self.min_length.as_mut(),
            "minClearWidth" => self.min_clear_width.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::WallsJoins {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::WallsEdit {
    /// `operation.width`, `operation.offset`, `operation.height`; `operation.at.x` and `.y` of Böl; `operation.by` of
    /// Kaydır, `operation.distance` of Mesafe (docs/adr/0710) and `operation.length` of Boy (docs/adr/0728).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_commands::WallsOperation;
        let c = match &mut self.operation {
            WallsOperation::Split { at } => {
                return match path {
                    "operation.at.x" => Some(&mut at.x),
                    "operation.at.y" => Some(&mut at.y),
                    _ => None,
                };
            }
            WallsOperation::Shift { by, .. } => return (path == "operation.by").then_some(by),
            WallsOperation::Distance { distance, .. } => return (path == "operation.distance").then_some(distance),
            WallsOperation::Length { length, .. } => return (path == "operation.length").then_some(length),
            WallsOperation::Set(c) => c,
            _ => return None,
        };
        match path {
            "operation.width" => c.width.as_mut(),
            "operation.offset" => c.offset.as_mut(),
            "operation.height" => c.height.as_mut()?.as_mut(),
            _ => None,
        }
    }
}

impl Input for ViewsEdit {
    /// A saved view's or a new area's (docs/adr/0789 §1) `op.center.x`, `op.center.y`, `op.width`, `op.height`,
    /// `op.twist` (docs/adr/0285 §1).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let (ViewsOp::Save {
            center,
            width,
            height,
            twist,
            ..
        }
        | ViewsOp::Area {
            center,
            width,
            height,
            twist,
            ..
        }) = &mut self.edit
        else {
            return None;
        };
        match path {
            "edit.center.x" => Some(&mut center.x),
            "edit.center.y" => Some(&mut center.y),
            "edit.width" => Some(width),
            "edit.height" => Some(height),
            "edit.twist" => Some(twist.get_or_insert(0.0)),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ExportIfcBuilding {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}
