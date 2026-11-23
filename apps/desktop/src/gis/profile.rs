//! KentOS GIS's profile (docs/adr/9195 §5, §6): every tab but Denetim and CAD denetimi; every panel but Bloklar,
//! Gezgin and the map's and road's own; Senaryo too; time, scenarios and typed layers; Katman stili's thematic
//! renderers; every layer menu entry, the data's edit session, the move into a table and the counts too; no blocks and
//! no polyline widths in the tools (Böl's Blok, Çoklu çizgi's Genişlik, the grip menu's Kenar genişliği…), nor a
//! polyline's Genişlik and an area's Maske in Öznitelikler; its new projects in metres. Every surface is placed by a
//! match with no arm for the rest (module_profile.rs).

use kentos_commands::ProjectType;
use kentos_interaction::ToolOption;
use kentos_interaction::point_calc::CalcKind;

use crate::app::Panel;
use crate::bottom::BottomTab;
use crate::layer_entries::LayerEntry;
use crate::module_profile::ModuleProfile;
use crate::module_surfaces::{AppSetting, Cell, Formats, PropertyRows, SourceSection};
use crate::project::settings::Section as ProjectSection;

/// KentOS GIS.
pub(crate) struct Gis;

impl ModuleProfile for Gis {
    fn bottom(&self, tab: BottomTab) -> bool {
        use BottomTab as T;
        match tab {
            T::History
            | T::Coords
            | T::Points
            | T::Table
            | T::Statistics
            | T::Chart
            | T::Search
            | T::Address
            | T::Topology
            | T::ExtAudit
            | T::ServiceInfo
            | T::Works
            | T::Messages
            | T::Python => true,
            // A building's audit and a drawing's quality are the CAD programs'.
            T::Audit | T::CadInspection => false,
        }
    }

    fn panel(&self, panel: Panel) -> bool {
        use Panel as P;
        match panel {
            // Şablonlar' object templates are a CBS's feature templates too (docs/adr/0176 §4, 9195 §5).
            P::Layers
            | P::Processing
            | P::Templates
            | P::Sources
            | P::Assistant
            | P::Extension
            | P::Properties => true,
            P::Blocks | P::Navigator | P::RoadProject | P::MapWork | P::MapSurvey => false,
        }
    }

    fn cell(&self, cell: Cell) -> bool {
        match cell {
            Cell::MapCrs | Cell::SecondCrs | Cell::Topology | Cell::Overlap | Cell::Scenario => {
                true
            }
            // A CBS project's Kalınlık stays in Görünüm (docs/adr/9185).
            Cell::Level | Cell::LineWeights => false,
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
            | E::EditSession
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
            | E::Migrate
            | E::Counts
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
            | E::Time
            | E::Statistics
            | E::Chart
            | E::Kind
            | E::Crs
            | E::Source
            | E::Snap
            | E::Scenario
            | E::Rename
            | E::Add
            | E::Duplicate
            | E::Merge
            | E::Remove => true,
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
            | PropertyRows::Survey
            | PropertyRows::Topology => true,
            PropertyRows::Mask | PropertyRows::Separators | PropertyRows::MapWork => false,
        }
    }

    fn formats(&self) -> Formats {
        Formats::All
    }

    fn parcels(&self) -> bool {
        true
    }

    fn sheet(&self) -> ProjectType {
        ProjectType::Gis
    }

    fn cloud(&self) -> Option<ProjectType> {
        Some(ProjectType::Gis)
    }

    fn upload(&self) -> ProjectType {
        ProjectType::Gis
    }

    fn hidden_calc(&self) -> &'static [CalcKind] {
        &[]
    }

    fn left_out(&self) -> &'static [ToolOption] {
        &[ToolOption::Blocks, ToolOption::Widths]
    }

    fn thematic(&self) -> bool {
        true
    }

    fn library_hidden(&self) -> &'static [&'static str] {
        &[]
    }
}
