//! The modules' profiles (docs/adr/9195 §2, §6; CLAUDE.md §25 rules 2–3): what each module shows of the window. Each
//! module's profile is in its own folder (arch/profile.rs, map/profile.rs, road/profile.rs, gis/profile.rs) and places
//! every surface by a match with no arm for the rest: a new tab, panel, cell, settings section or layer menu entry does
//! not build until each module says yes or no. [`profile`] is the shell's one branch by module for its surfaces
//! (scripts/dev/module_deps.py allows it here alone); kentos-cad with nothing open has the shell's own
//! [`NoProject`].

use kentos_commands::ProjectType;
use kentos_interaction::ToolOption;
use kentos_interaction::point_calc::CalcKind;
use kentos_launch::Module;

use crate::app::Panel;
use crate::bottom::BottomTab;
use crate::layer_entries::LayerEntry;
use crate::module_surfaces::{AppSetting, Cell, Formats, PropertyRows, SourceSection};
use crate::project::settings::Section as ProjectSection;

/// What a module shows of the window: each surface yes or no, and the values its windows start with.
pub trait ModuleProfile: Sync {
    /// Whether the bottom panel has `tab`.
    fn bottom(&self, tab: BottomTab) -> bool;
    /// Whether the dock has `panel`.
    fn panel(&self, panel: Panel) -> bool;
    /// Whether the status bar shows `cell` (and what goes with it, see [`Cell`]).
    fn cell(&self, cell: Cell) -> bool;
    /// Whether Proje ayarları has `section`.
    fn project_section(&self, section: ProjectSection) -> bool;
    /// Whether Uygulama ayarları shows `setting`.
    fn setting(&self, setting: AppSetting) -> bool;
    /// Whether the layer menu has `entry`; the kinds of layer it shows on are layer_entries.rs's.
    fn layer_entry(&self, entry: LayerEntry) -> bool;
    /// Whether Kaynaklar has `section`.
    fn source_section(&self, section: SourceSection) -> bool;
    /// Whether Öznitelikler shows `rows`.
    fn rows(&self, rows: PropertyRows) -> bool;
    /// The application menu's İçe aktar and Dışa aktar formats.
    fn formats(&self) -> Formats;
    /// Whether the hover card shows an object's parcel rows: Ada, Mahalle, Nitelik, the deed's area and Parsel's title.
    fn parcels(&self) -> bool;
    /// The project type the sheet mode ranks its templates by (docs/sheet/design.md §4).
    fn sheet(&self) -> ProjectType;
    /// The type Bulut projeleri lists at first; none: every type (a map's projects carry five).
    fn cloud(&self) -> Option<ProjectType>;
    /// The type Buluta yükle offers first.
    fn upload(&self) -> ProjectType;
    /// Nokta hesapla's constructions it has not.
    fn hidden_calc(&self) -> &'static [CalcKind];
    /// The tools' options it leaves out (Draft::offers).
    fn left_out(&self) -> &'static [ToolOption];
    /// Whether Katman stili lists the thematic renderers (docs/adr/0213): a layer drawn with one keeps it everywhere.
    fn thematic(&self) -> bool;
    /// The system style library's branches Stil yöneticisi and the symbol pickers leave out.
    fn library_hidden(&self) -> &'static [&'static str];
    /// The project tree's title and its words while it is empty (road_project_panel.rs, docs/adr/1720): Harita's tree of
    /// its surfaces, Yol's of its sources; a module without the panel says Yol's.
    fn project_tree(&self) -> (&'static str, &'static str);
}

/// `module`'s profile, from its own folder; a module that is a package of its own gives it with its parts
/// (module_parts.rs), and one not in this program shows the shell's own ([`NoProject`]).
pub fn profile(module: Module) -> &'static dyn ModuleProfile {
    match module {
        Module::Gis => &crate::gis_profile::Gis,
        Module::Architecture | Module::Map | Module::Road => {
            crate::module_parts::parts(module).map_or(&NoProject as &dyn ModuleProfile, |p| p.profile())
        }
    }
}

/// kentos-cad with nothing open (docs/adr/9195 §2): the surfaces every module shares, no module's own. A project opened
/// brings its module's.
pub(crate) struct NoProject;

impl ModuleProfile for NoProject {
    fn bottom(&self, tab: BottomTab) -> bool {
        use BottomTab as T;
        match tab {
            T::History | T::Works | T::Messages | T::Python => true,
            T::Coords
            | T::Points
            | T::Table
            | T::Statistics
            | T::Chart
            | T::Search
            | T::Address
            | T::Topology
            | T::Audit
            | T::ExtAudit
            | T::CadInspection
            | T::ServiceInfo => false,
        }
    }

    fn panel(&self, panel: Panel) -> bool {
        use Panel as P;
        match panel {
            // Asistan, the user's own AI agent, and an extension's panel are every program's (docs/adr/8056 §4,
            // 8057 §24).
            P::Layers | P::Assistant | P::Extension | P::Properties => true,
            P::Processing
            | P::Blocks
            | P::Templates
            | P::Sources
            | P::Navigator
            | P::RoadProject
            | P::MapWork
            | P::MapSurvey => false,
        }
    }

    fn cell(&self, cell: Cell) -> bool {
        match cell {
            Cell::MapCrs
            | Cell::SecondCrs
            | Cell::Topology
            | Cell::Overlap
            | Cell::Scenario
            | Cell::Level
            | Cell::LineWeights => false,
        }
    }

    fn project_section(&self, section: ProjectSection) -> bool {
        use ProjectSection as S;
        match section {
            S::General | S::Units | S::Crs | S::Heights | S::Survey | S::Variables | S::Arch => {
                false
            }
        }
    }

    fn setting(&self, setting: AppSetting) -> bool {
        // A new project's type and unit are asked here alone: a module's program and an open project fix them.
        match setting {
            AppSetting::Geographic
            | AppSetting::FormOnCreate
            | AppSetting::NewProjects
            | AppSetting::NewProjectType
            | AppSetting::NewProjectUnit => true,
        }
    }

    fn layer_entry(&self, _entry: LayerEntry) -> bool {
        // No project, no layer.
        false
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
        ProjectType::Cad
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

    fn project_tree(&self) -> (&'static str, &'static str) {
        ("Yol projesi", "Projede yol kaynağı yok: Arazi sekmesinden yüzey, Güzergâh'tan aks oluşturun.")
    }
}

/// The modules whose parts are still in the core's package (docs/adr/9199 §4.5): the shell takes them with the ones a
/// program registers.
pub(crate) const IN_CORE: &[&'static dyn crate::module_parts::ModuleParts] = &[&crate::arch_parts::Architecture];
