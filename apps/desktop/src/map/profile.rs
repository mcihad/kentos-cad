//! KentOS Harita's profile (docs/adr/9195 §5, §6): a map on real coordinates. Every tab but Grafik and Denetim; every
//! panel but Gezgin, the map's work and GNSS archive too; the map system, the second system, Topoloji and Çakışma; Kotlar
//! ve epok and Ölçme; GIS files, sources and services; a map's layer menu, its features' data, forms and style files,
//! no time, scenarios, typed layers or counts; its new projects in metres. Every surface is placed by a match with no
//! arm for the rest (module_profile.rs).

use kentos_commands::ProjectType;
use kentos_interaction::ToolOption;
use kentos_interaction::point_calc::CalcKind;

use crate::app::Panel;
use crate::bottom::BottomTab;
use crate::layer_entries::LayerEntry;
use crate::module_profile::ModuleProfile;
use crate::module_surfaces::{AppSetting, Cell, Formats, PropertyRows, SourceSection};
use crate::project::settings::Section as ProjectSection;

/// KentOS Harita.
pub(crate) struct Map;

impl ModuleProfile for Map {
    fn bottom(&self, tab: BottomTab) -> bool {
        use BottomTab as T;
        match tab {
            T::History
            | T::Coords
            | T::Points
            | T::Table
            | T::Statistics
            | T::Search
            | T::Address
            | T::Topology
            | T::ExtAudit
            | T::CadInspection
            | T::ServiceInfo
            | T::Works
            | T::Messages
            | T::Python => true,
            T::Chart | T::Audit => false,
        }
    }

    fn panel(&self, panel: Panel) -> bool {
        use Panel as P;
        match panel {
            // Yol projesi is a map project's tree too (docs/adr/1720): its surfaces, pads and what was drawn from them.
            P::Layers
            | P::Processing
            | P::Blocks
            | P::Templates
            | P::Sources
            | P::RoadProject
            | P::MapWork
            | P::MapSurvey
            | P::Assistant
            | P::Extension
            | P::Properties => true,
            P::Navigator => false,
        }
    }

    fn cell(&self, cell: Cell) -> bool {
        match cell {
            Cell::MapCrs | Cell::SecondCrs | Cell::Topology | Cell::Overlap | Cell::LineWeights => {
                true
            }
            Cell::Scenario | Cell::Level => false,
        }
    }

    fn project_section(&self, section: ProjectSection) -> bool {
        use ProjectSection as S;
        match section {
            S::General | S::Units | S::Crs | S::Heights | S::Survey | S::Variables => true,
            S::Arch => false,
        }
    }

    fn setting(&self, setting: AppSetting) -> bool {
        match setting {
            AppSetting::Geographic | AppSetting::FormOnCreate | AppSetting::NewProjects => true,
            AppSetting::NewProjectType | AppSetting::NewProjectUnit => false,
        }
    }

    fn layer_entry(&self, entry: LayerEntry) -> bool {
        use LayerEntry as E;
        match entry {
            E::Activate
            | E::Visibility
            | E::Lock
            | E::Freeze
            | E::Plot
            | E::Order
            | E::Service
            | E::Refresh
            | E::SelectObjects
            | E::ZoomTo
            | E::Export
            | E::Color
            | E::LineType
            | E::LineWeight
            | E::Style
            | E::Labels
            | E::StyleFiles
            | E::Sld
            | E::Qml
            | E::MapLibre
            | E::Impact
            | E::Fields
            | E::Forms
            | E::Info
            | E::Dataset
            | E::Filter
            | E::Statistics
            | E::Crs
            | E::Source
            | E::Snap
            | E::Rename
            | E::Add
            | E::Duplicate
            | E::Merge
            | E::Remove => true,
            // A CBS's: a table's edit session (a CBS table is a map's read-only underlay, docs/adr/9196 §2), the move
            // into a table, counts, time, charts, typed layers and scenarios.
            E::EditSession
            | E::Migrate
            | E::Counts
            | E::Time
            | E::Chart
            | E::Kind
            | E::Scenario => false,
        }
    }

    fn source_section(&self, section: SourceSection) -> bool {
        match section {
            SourceSection::Databases | SourceSection::Services => true,
        }
    }

    fn rows(&self, rows: PropertyRows) -> bool {
        match rows {
            PropertyRows::Crs
            | PropertyRows::Services
            | PropertyRows::Mask
            | PropertyRows::MapWork
            | PropertyRows::Survey
            | PropertyRows::Topology => true,
            PropertyRows::Separators => false,
        }
    }

    fn formats(&self) -> Formats {
        Formats::All
    }

    fn parcels(&self) -> bool {
        true
    }

    fn sheet(&self) -> ProjectType {
        ProjectType::ZoningPlan
    }

    fn cloud(&self) -> Option<ProjectType> {
        None
    }

    fn upload(&self) -> ProjectType {
        ProjectType::Cad
    }

    fn hidden_calc(&self) -> &'static [CalcKind] {
        &[]
    }

    fn left_out(&self) -> &'static [ToolOption] {
        &[]
    }

    fn thematic(&self) -> bool {
        false
    }

    fn library_hidden(&self) -> &'static [&'static str] {
        &[]
    }
}
