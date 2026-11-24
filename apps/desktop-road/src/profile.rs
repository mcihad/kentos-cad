//! KentOS Yol's profile (plan/road.md ROAD-MOD; docs/adr/9195 §5, §6): a road on real coordinates. The map's survey and
//! coordinate surfaces (Koordinat listesi, Noktalar, Harita sistemi, İkinci sistem, Kotlar ve epok, Ölçme) and CAD
//! denetimi; no parcel rows, zoning legends, addresses, topology or overlap lamps; a road's layer menu, a map's layers
//! and services without forms, cards, statistics and style files; its sheets and its cloud projects are a road's. Every
//! surface is placed by a match with no arm for the rest (module_profile.rs).

use kentos_commands::ProjectType;
use kentos_interaction::ToolOption;
use kentos_interaction::point_calc::CalcKind;

use crate::app::Panel;
use crate::bottom::BottomTab;
use crate::layer_entries::LayerEntry;
use crate::module_profile::ModuleProfile;
use crate::module_surfaces::{AppSetting, Cell, Formats, PropertyRows, SourceSection};
use crate::project::settings::Section as ProjectSection;

/// KentOS Yol.
pub(crate) struct Road;

impl ModuleProfile for Road {
    fn bottom(&self, tab: BottomTab) -> bool {
        use BottomTab as T;
        match tab {
            T::History
            | T::Coords
            | T::Points
            | T::Table
            | T::Search
            | T::ExtAudit
            | T::CadInspection
            | T::Works
            | T::Messages
            | T::Python => true,
            T::Statistics | T::Chart | T::Address | T::Topology | T::Audit | T::ServiceInfo => {
                false
            }
        }
    }

    fn panel(&self, panel: Panel) -> bool {
        use Panel as P;
        match panel {
            P::Layers
            | P::Processing
            | P::Blocks
            | P::Templates
            | P::Sources
            | P::RoadProject
            | P::Assistant
            | P::Extension
            | P::Properties => true,
            P::Navigator | P::MapWork | P::MapSurvey => false,
        }
    }

    fn cell(&self, cell: Cell) -> bool {
        match cell {
            // Harita sistemi as a map's: its commands are a road's too (catalog/reading.rs).
            Cell::MapCrs | Cell::SecondCrs | Cell::LineWeights => true,
            Cell::Topology | Cell::Overlap | Cell::Scenario | Cell::Level => false,
        }
    }

    fn project_section(&self, section: ProjectSection) -> bool {
        use ProjectSection as S;
        match section {
            S::General | S::Units | S::Crs | S::Heights | S::Survey => true,
            S::Variables | S::Arch => false,
        }
    }

    fn setting(&self, setting: AppSetting) -> bool {
        match setting {
            AppSetting::Geographic | AppSetting::NewProjects => true,
            AppSetting::FormOnCreate | AppSetting::NewProjectType | AppSetting::NewProjectUnit => {
                false
            }
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
            // The mobile viewer shows every module's project (docs/adr/9197 §2.2, 9198).
            | E::Mobile
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
            | E::Impact
            | E::Dataset
            | E::Filter
            | E::Crs
            | E::Source
            | E::Snap
            | E::Rename
            | E::Add
            | E::Duplicate
            | E::Merge
            | E::Remove => true,
            // A CBS table is a road's read-only underlay (docs/adr/9196 §2); no forms, cards, statistics or other
            // programs' style files.
            E::EditSession
            | E::Migrate
            | E::Counts
            | E::StyleFiles
            | E::Sld
            | E::Qml
            | E::MapLibre
            | E::Fields
            | E::Forms
            | E::Info
            | E::Time
            | E::Statistics
            | E::Chart
            | E::Kind
            | E::Scenario => false,
        }
    }

    fn source_section(&self, section: SourceSection) -> bool {
        match section {
            SourceSection::Services => true,
            SourceSection::Databases => false,
        }
    }

    fn rows(&self, rows: PropertyRows) -> bool {
        match rows {
            PropertyRows::Crs | PropertyRows::Services => true,
            PropertyRows::Mask
            | PropertyRows::Separators
            | PropertyRows::MapWork
            | PropertyRows::Survey
            | PropertyRows::Topology => false,
        }
    }

    fn formats(&self) -> Formats {
        Formats::All
    }

    fn parcels(&self) -> bool {
        false
    }

    fn sheet(&self) -> ProjectType {
        ProjectType::Road
    }

    fn cloud(&self) -> Option<ProjectType> {
        Some(ProjectType::Road)
    }

    fn upload(&self) -> ProjectType {
        ProjectType::Road
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

    fn project_tree(&self) -> (&'static str, &'static str) {
        ("Yol projesi", "Projede yol kaynağı yok: Arazi sekmesinden yüzey, Güzergâh'tan aks oluşturun.")
    }
}
