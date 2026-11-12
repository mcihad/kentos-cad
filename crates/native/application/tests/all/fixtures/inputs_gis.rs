//! The gis commands' inputs with the numbers a fixture's `nonFinite` may name (docs/adr/8054 §9).

#[allow(unused_imports)]
use super::*;

impl Input for kentos_commands::TopologyRepair {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TopologyCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::TopologyExceptions {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsValidate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsSubmit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsFill {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsAudit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersContingents {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersRules {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersIndexes {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersSubtypes {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsRecompute {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsTransition {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for DomainsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for DomainsExport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FieldsMigrate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for AttachmentsInspect {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for SchemaDescribe {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsExport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsImport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsCompare {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsSources {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsTest {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for RecordsDuplicate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for RecordsCreate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for RelationsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for JoinsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for JoinsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for DatasetsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for DatasetsDescribe {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImagesEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RastersMosaic {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RastersReproject {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RastersGpkgRead {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RastersGpkgWrite {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::StacSearch {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::StacPrepare {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::MeshesDescribe {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::MeshesQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::MeshesRegion {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "time" => self.time.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::MeshesProfile {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "step" => Some(&mut self.step),
            _ => None,
        }
    }
}

impl Input for kentos_commands::TimeQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SensorsIngest {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SensorsLatest {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "staleAfter" => self.stale_after.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_commands::RastersGeoref {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersService {
    /// `service.opacity` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "service.opacity" => self.service.as_mut()?.opacity.as_mut(),
            _ => None,
        }
    }
}

impl Input for LayersTime {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ScenariosEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersFilter {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersInfo {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersTable {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersCompose {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersLabels {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImagesClip {
    /// `clip.a.x`, `clip.b.y`, `clip.points[i].x` (the shared cases spoil only these).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match &mut self.clip {
            kentos_commands::ImageClipBy::Rectangle { a, b } => match path {
                "clip.a.x" => Some(&mut a.x),
                "clip.a.y" => Some(&mut a.y),
                "clip.b.x" => Some(&mut b.x),
                "clip.b.y" => Some(&mut b.y),
                _ => None,
            },
            kentos_commands::ImageClipBy::Polygon { points } => {
                let (i, axis) = path.strip_prefix("clip.points[")?.split_once("].")?;
                let p = points.get_mut(i.parse::<usize>().ok()?)?;
                match axis {
                    "x" => Some(&mut p.x),
                    "y" => Some(&mut p.y),
                    _ => None,
                }
            }
            kentos_commands::ImageClipBy::Remove => None,
        }
    }
}

impl Input for kentos_commands::TimeVersion {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::FieldsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersRenderer {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersEdit {
    /// `lineWeight` (which the step's input must give, docs/adr/0248 §1).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "lineWeight" => self.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for LayersSet {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersStates {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FieldsUsage {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersFind {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "lineWeight").then_some(())?;
        self.line_weight.as_mut()
    }
}

impl Input for LayersTranslate {
    /// `rules[2].lineWeight`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let rest = path.strip_prefix("rules[")?;
        let (i, field) = rest.split_once("].")?;
        let rule = self.rules.get_mut(i.parse::<usize>().ok()?)?;
        (field == "lineWeight").then_some(())?;
        rule.line_weight.as_mut()
    }
}

impl Input for kentos_commands::GisTakeback {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::GisDetach {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::GisDerivation {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportSld {
    /// No number: a path or a text, a layer.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportMaplibre {
    /// No number: a path or a text, a layer.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ImportQml {
    /// No number: a path or a text, a layer.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SourcesDescribe {
    /// No number: a path.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SourcesEdit {
    /// No number: paths, a table, a layer and ids.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProcessingTools {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProcessingRun {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ProcessingBatch {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::FieldsWrite {
    /// No number: a layer, fields and values as text.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SourcesSync {
    /// No number: a layer and a direction.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SourcesReconcile {
    /// No number: a layer and the sides taken.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SourcesRelate {
    /// No number: a relation, an operation and a table's name.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::RecordsPage {
    /// No number to move: a layer or a relation, words, a cursor.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::LayersCrs {
    /// No number to move: a layer and a code.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::LayersReproject {
    /// No number to move: a layer and a code.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SourcesQuery {
    /// The box's and the place's numbers.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let at = |rest: &str| rest.parse::<usize>().ok();
        if let Some(i) = path.strip_prefix("bbox.").and_then(at) {
            return self.bbox.as_mut()?.get_mut(i);
        }
        if let Some(i) = path.strip_prefix("near.at.").and_then(at) {
            return self.near.as_mut()?.at.get_mut(i);
        }
        if path == "near.tolerance" {
            return Some(&mut self.near.as_mut()?.tolerance);
        }
        None
    }
}

impl Input for kentos_commands::SubnetworksFind {
    /// No number: a network's id.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::SubnetworksUpdate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for NetworkDefine {
    /// `network.tolerance` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "network.tolerance" => self.network.as_mut().map(|n| &mut n.tolerance),
            _ => None,
        }
    }
}

impl Input for kentos_commands::LinearRefDefine {
    /// `link.tolerance` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "link.tolerance" => self.link.as_mut().and_then(|l| l.tolerance.as_mut()),
            _ => None,
        }
    }
}

impl Input for kentos_commands::TopologyClean {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "tolerance" => Some(&mut self.tolerance),
            _ => None,
        }
    }
}

impl Input for kentos_commands::ScenariosImpact {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::ScenariosRecord {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AddressesNormalize {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AddressesCheck {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_commands::AddressesCompare {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for AddressesDefine {
    /// `locator.reach` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "locator.reach" => self.locator.as_mut().map(|l| &mut l.reach),
            _ => None,
        }
    }
}
