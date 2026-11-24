//! KentOS Mimari's profile (docs/adr/9195 §5, §6): a building on its own coordinates. Komut geçmişi, Koordinat
//! listesi, Arama (Veride ara finds the texts of a building too), Denetim, CAD denetimi, Uyarılar and Python;
//! Katmanlar, Bloklar, Şablonlar and Gezgin; the Kat cell; Mimari in Proje ayarları; drawings (KCAD, DWG, DXF, NCZ,
//! PDF) in and out; a drawing's layer menu, no feature data, style files, services or sources; no survey's tool options
//! (Ölçülendirme's Semt, Nokta's Ad, Kod and its question, Çizgi's recorded measurements) and no zoning plan's legends
//! (the library's MPYY). Every surface is placed by a match with no arm for the rest (module_profile.rs).

use kentos_commands::ProjectType;
use kentos_interaction::ToolOption;
use kentos_interaction::point_calc::CalcKind;

use crate::app::Panel;
use crate::bottom::BottomTab;
use crate::layer_entries::LayerEntry;
use crate::module_profile::ModuleProfile;
use crate::module_surfaces::{AppSetting, Cell, Formats, PropertyRows, SourceSection};
use crate::project::settings::Section as ProjectSection;

/// KentOS Mimari.
pub(crate) struct Architecture;

impl ModuleProfile for Architecture {
    fn bottom(&self, tab: BottomTab) -> bool {
        use BottomTab as T;
        match tab {
            // An extension's audit rules' findings are every program's (docs/adr/8057 §17).
            T::History
            | T::Coords
            | T::Search
            | T::Audit
            | T::ExtAudit
            | T::CadInspection
            | T::Works
            | T::Messages
            | T::Python => true,
            T::Points
            | T::Table
            | T::Statistics
            | T::Chart
            | T::Address
            | T::Topology
            | T::ServiceInfo => false,
        }
    }

    fn panel(&self, panel: Panel) -> bool {
        use Panel as P;
        match panel {
            // Asistan and an extension's panel are every program's (docs/adr/8056 §4, 8057 §24).
            P::Layers
            | P::Blocks
            | P::Templates
            | P::Navigator
            | P::Assistant
            | P::Extension
            | P::Properties => true,
            P::Processing | P::Sources | P::RoadProject | P::MapWork | P::MapSurvey => false,
        }
    }

    fn cell(&self, cell: Cell) -> bool {
        match cell {
            Cell::Level | Cell::LineWeights => true,
            Cell::MapCrs | Cell::SecondCrs | Cell::Topology | Cell::Overlap | Cell::Scenario => {
                false
            }
        }
    }

    fn project_section(&self, section: ProjectSection) -> bool {
        use ProjectSection as S;
        match section {
            S::General | S::Units | S::Crs | S::Variables | S::Arch => true,
            S::Heights | S::Survey => false,
        }
    }

    fn setting(&self, setting: AppSetting) -> bool {
        match setting {
            AppSetting::Geographic
            | AppSetting::FormOnCreate
            | AppSetting::NewProjects
            | AppSetting::NewProjectType
            | AppSetting::NewProjectUnit => false,
            // The user's keys and names are the person's, every program's (docs/adr/9308).
            AppSetting::Shortcuts => true,
        }
    }

    fn layer_entry(&self, entry: LayerEntry) -> bool {
        use LayerEntry as E;
        match entry {
            // A drawing's.
            E::Activate
            | E::Visibility
            | E::Lock
            | E::Freeze
            | E::Plot
            // The mobile viewer shows every module's project (docs/adr/9197 §2.2, 9198).
            | E::Mobile
            | E::Order
            | E::SelectObjects
            | E::ZoomTo
            | E::Color
            | E::LineType
            | E::LineWeight
            | E::Style
            | E::Impact
            | E::Snap
            | E::Rename
            | E::Add
            | E::Duplicate
            | E::Merge
            | E::Remove => true,
            // A map's and a CBS's: services, tables, sources, feature data and other programs' style files.
            E::Service
            | E::Refresh
            | E::Export
            | E::Migrate
            | E::Counts
            | E::Labels
            | E::StyleFiles
            | E::Sld
            | E::Qml
            | E::MapLibre
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
            | E::Scenario => false,
        }
    }

    fn source_section(&self, section: SourceSection) -> bool {
        match section {
            SourceSection::Databases | SourceSection::Services => false,
        }
    }

    fn rows(&self, rows: PropertyRows) -> bool {
        match rows {
            PropertyRows::Mask | PropertyRows::Separators => true,
            PropertyRows::Crs
            | PropertyRows::Services
            | PropertyRows::MapWork
            | PropertyRows::Survey
            | PropertyRows::Topology => false,
        }
    }

    fn formats(&self) -> Formats {
        Formats::Drawings
    }

    fn parcels(&self) -> bool {
        false
    }

    fn sheet(&self) -> ProjectType {
        ProjectType::Architecture
    }

    fn cloud(&self) -> Option<ProjectType> {
        Some(ProjectType::Architecture)
    }

    fn upload(&self) -> ProjectType {
        ProjectType::Architecture
    }

    fn hidden_calc(&self) -> &'static [CalcKind] {
        &[CalcKind::Km, CalcKind::Name]
    }

    fn left_out(&self) -> &'static [ToolOption] {
        &[
            ToolOption::Bearing,
            ToolOption::PointNames,
            ToolOption::Recorded,
        ]
    }

    fn thematic(&self) -> bool {
        false
    }

    fn library_hidden(&self) -> &'static [&'static str] {
        &["MPYY"]
    }

    fn project_tree(&self) -> (&'static str, &'static str) {
        ("Yol projesi", "Projede yol kaynağı yok: Arazi sekmesinden yüzey, Güzergâh'tan aks oluşturun.")
    }
}
