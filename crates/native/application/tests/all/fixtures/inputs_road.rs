//! The road commands' inputs with the numbers a fixture's `nonFinite` may name (docs/adr/8054 §9).

#[allow(unused_imports)]
use super::*;

impl Input for kentos_commands::RoadsEdit {
    /// `work.axis[i].x` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let kentos_commands::RoadWork::Plan { axis, .. } = &mut self.work else {
            return None;
        };
        let (i, axis_name) = path.strip_prefix("work.axis[")?.split_once("].")?;
        let p = axis.get_mut(i.parse::<usize>().ok()?)?;
        match axis_name {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_commands::AlignmentsCreate {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AlignmentsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AlignmentsLabel {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AlignmentsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesCreate {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SuperelevationsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SuperelevationsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SuperelevationsCheck {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoadProfilesLengths {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SuperelevationsWiden {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SuperelevationsTransitions {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CorridorsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CorridorsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CorridorsDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::VolumesEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::VolumesQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::VolumesUncertainty {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::VolumesProgress {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::VolumesHistory {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::VolumesStates {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesImpact {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AlignmentsImpact {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::VolumesLedger {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (path, &mut self.by) {
            ("by.step", kentos_commands::LedgerBy::Stations { step }) => Some(step),
            _ => None,
        }
    }
}

impl Input for kentos_commands::VolumesCompare {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::VolumesApprove {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::HaulsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CulvertsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CulvertsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CulvertsDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CulvertsClosure {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "entranceLoss" => Some(&mut self.entrance_loss),
            _ => None,
        }
    }
}

impl Input for kentos_commands::CulvertsHydraulics {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "flow" => Some(&mut self.flow),
            "entranceLoss" => Some(&mut self.entrance_loss),
            _ => None,
        }
    }
}

impl Input for kentos_commands::RetainingWallsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RetainingWallsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RetainingWallsDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::DrainageQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::DrainageDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SweptQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsAnalysis {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsCompare {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsQuality {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsRepair {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsCoverage {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsCoverageDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsDensity {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsStockpile {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsSparseDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SweptDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsFromCloud {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PlatformsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PlatformsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PlatformsDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PayEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::PayQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoutesFind {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoutesDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::JunctionsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::JunctionsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::JunctionsClearance {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::JunctionsTie {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::JunctionsOverlap {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoadProjectQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoadProjectGraphQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoadProjectRebuild {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoadProjectProduce {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsSectionCheck {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "corridor" => self.corridor.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::RoadProjectEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::JunctionsDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::GuardrailsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::GuardrailsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::GuardrailsDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::HaulsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AlignmentsImport {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ExportGltf {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ExportIfc {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ExportLandxml {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AlignmentsFit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesFit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoadSheetsSections {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoadSheetsPlan {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::StakeoutSend {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::StakeoutQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SectionsDraw {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TemplatesEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SectionsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SectionsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AlignmentsCheck {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RoadProfilesCheck {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CorridorsPreflight {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CorridorsProtected {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::CorridorsDaylights {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "sensitive" => self.sensitive.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::CorridorsWater {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::HaulsNetwork {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::HaulsApprove {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::HaulsPlan {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::HaulsTrucks {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesSight {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProfilesPropose {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsCreate {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsEdit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsZeroLine {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TerrainsCutFill {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "interval" => self.interval.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::TerrainsContours {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "interval").then_some(&mut self.interval)
    }
}

impl Input for kentos_commands::TerrainsSheetContours {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "interval" { self.interval.as_mut() } else { None }
    }
}

impl Input for kentos_commands::TerrainsQuery {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}
