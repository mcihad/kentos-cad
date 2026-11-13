//! The manifest v1, `manifest.json` (plan/eklenti.md §3.2, docs/adr/8057 §1): who the extension is, what it adds and
//! what it asks to do. Read strictly: an unknown field is refused, since an agent that invents a key must hear so at the
//! place it wrote it; that is why discovery, links, dependencies, the preview channel and retirement are optional fields
//! of v1 from the first day (D9). Every broken rule is a [`Problem`] with a stable code, the JSON pointer and the fix in
//! Turkish. The rules here need nothing but the manifest; what it names in the package is the inspector's
//! ([`crate::archive`]).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::compat::Os;
use crate::paths;
use crate::pointer::{self, segment};
use crate::problem::Problem;
use crate::version::{Version, VersionReq};

/// The manifest's name in a package.
pub const FILE: &str = "manifest.json";
/// Its `format`.
pub const FORMAT: &str = "kentos.extension";
/// The `schemaVersion` this build reads.
pub const SCHEMA_VERSION: u32 = 1;

/// The manifest v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    /// `kentos.extension`.
    pub format: String,
    pub schema_version: u32,
    /// Reverse domain, lower case, two parts at least: `tr.ornek.mebis`.
    pub id: String,
    /// A semantic version; a pre-release (`1.2.0-beta.1`) puts it on the preview channel.
    pub version: String,
    pub name: String,
    /// One line.
    pub summary: String,
    /// Plain text (no HTML), shown before installing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Display names; no e-mail (personal data, the catalog account vouches for contact).
    pub authors: Vec<String>,
    /// A claim in the package; the catalog verifies the ownership.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_id: Option<String>,
    /// An SPDX expression.
    pub license: String,
    pub license_file: String,
    #[serde(default, skip_serializing_if = "Links::is_empty")]
    pub links: Links,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog_file: Option<String>,
    /// A 20×20 SVG in the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub kind: ExtensionKind,
    pub modules: Vec<Module>,
    /// The catalog's summary of where it runs; each tool says its own.
    pub runtimes: Vec<Runtime>,
    pub compatibility: Compatibility,
    #[serde(default, skip_serializing_if = "Dependencies::is_empty")]
    pub dependencies: Dependencies,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonProject>,
    /// The vertical extension's layers, domains and relations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l10n: Option<L10n>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<Deprecation>,
    #[serde(default, skip_serializing_if = "Entrypoints::is_empty")]
    pub entrypoints: Entrypoints,
    #[serde(default, skip_serializing_if = "Contributions::is_empty")]
    pub contributions: Contributions,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_capabilities: Vec<RequestedCapability>,
}

/// A vertical extension is a system of its own (a ribbon tab, layers, forms); a utility adds tools to the existing ones
/// and opens no tab (EKL-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionKind {
    Vertical,
    Utility,
}

/// A KentOS program an extension adds to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Module {
    Mimari,
    Harita,
    Cbs,
    Yol,
}

impl Module {
    pub fn id(self) -> &'static str {
        match self {
            Module::Mimari => "mimari",
            Module::Harita => "harita",
            Module::Cbs => "cbs",
            Module::Yol => "yol",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Runtime {
    Desktop,
    Headless,
    Cloud,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Links {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issues: Option<String>,
}

impl Links {
    pub fn is_empty(&self) -> bool {
        self.homepage.is_none() && self.source.is_none() && self.issues.is_none()
    }
}

/// Where a package runs: [`crate::compatible`] reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Compatibility {
    pub sdk_protocol: ProtocolRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kentos: Option<KentosRange>,
    /// None written: every system.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operating_systems: Vec<Os>,
}

/// The extension protocol's versions a package was written for, both ends inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtocolRange {
    pub min: u32,
    pub max: u32,
}

/// KentOS's versions: `min` inclusive, `maxExclusive` exclusive; an end not written is open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KentosRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_exclusive: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Dependencies {
    /// Extensions that must be installed and enabled; no code is imported across (each runs in its own process).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<ExtensionDependency>,
}

impl Dependencies {
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionDependency {
    pub id: String,
    /// A range: `>=1.2, <2`.
    pub version: String,
}

/// The Python project: `requires-python` and the packages are in its files, never repeated here (PY-UV-13).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PythonProject {
    pub project_file: String,
    pub lock_file: String,
    /// What a user may turn on: the key is local, `extra` the project's optional dependency group.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub optional_features: BTreeMap<String, OptionalFeature>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OptionalFeature {
    pub extra: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct L10n {
    /// The language the manifest is written in.
    pub default: String,
    /// The translations: language → file.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Deprecation {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
}

/// What the extension's code answers. Ids are the package's own (`init-schema`); the host names them
/// `ext.<extension>.<id>`. The process starts when one is first called; v1 has no "run at start-up" (D6).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entrypoints {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<CommandEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub map_tools: Vec<MapToolEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processing: Vec<ProcessingEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audits: Vec<AuditEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expressions: Vec<ExpressionEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panels: Vec<PanelEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<FormEntry>,
}

impl Entrypoints {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
            && self.map_tools.is_empty()
            && self.processing.is_empty()
            && self.audits.is_empty()
            && self.expressions.is_empty()
            && self.panels.is_empty()
            && self.forms.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEntry {
    pub id: String,
    pub title: String,
    /// A built-in icon's name or a 20×20 SVG in the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// `module.path:function`, known without importing it.
    pub handler: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapToolEntry {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub capture: Capture,
    /// A rectangle's template, in metres: both or neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_width_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_length_m: Option<f64>,
    /// The layers (by schema name) the tool snaps to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snap_layers: Vec<String>,
    pub handler: String,
}

/// What a map tool asks the user to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Capture {
    Point,
    Line,
    Polygon,
    Rectangle,
    OrientedRectangle,
    Circle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessingEntry {
    pub id: String,
    /// `tools/<id>.tool.json`, a `kentos.processingTool` v1.
    pub descriptor: String,
    pub handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditEntry {
    pub id: String,
    pub title: String,
    /// The program whose audit it joins.
    pub domain: Module,
    pub handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpressionEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PanelEntry {
    pub id: String,
    pub title: String,
    pub side: PanelSide,
    /// `ui/<id>.panel.json`, a `kentos.ui` panel.
    pub definition: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PanelSide {
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormEntry {
    pub id: String,
    /// `forms/<id>.kform`, a `kentos.form-package` v1.
    pub definition: String,
}

/// What the extension adds to the window and the sources, read without its code (D6, D10).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contributions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ribbon_tabs: Vec<RibbonTab>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_menus: Vec<ContextMenuItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_sources: Vec<DataSource>,
}

impl Contributions {
    pub fn is_empty(&self) -> bool {
        self.ribbon_tabs.is_empty() && self.context_menus.is_empty() && self.data_sources.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RibbonTab {
    pub id: String,
    pub label: String,
    /// The program whose ribbon shows it.
    pub module: Module,
    pub panels: Vec<RibbonPanel>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RibbonPanel {
    pub label: String,
    pub items: Vec<RibbonItem>,
}

/// A ribbon button: exactly one of the targets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RibbonItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
    /// A KentOS command by its id (`cad.layers.service`): a button with no code of the extension's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin: Option<String>,
    #[serde(default)]
    pub size: ItemSize,
    /// The target's own title and icon unless given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemSize {
    #[default]
    Small,
    Large,
}

/// A right-click menu's item: exactly one of the targets, and where.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextMenuItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin: Option<String>,
    pub target: MenuTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}

/// Which right-click menu: the drawing's (what is going on there, the selection's edits) or a layer's in Katmanlar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MenuTarget {
    Drawing,
    Layer,
}

/// When an item shows: a closed vocabulary, judged without loading the extension's code (D6), so the ribbon is right
/// for a user without Python too. Every condition given holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct When {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<Module>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_open: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<SelectionWhen>,
    /// A layer of this schema name is in the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_granted: Option<Capability>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionWhen {
    None,
    Feature,
    Layer,
    Many,
}

/// A source the extension brings without code: a service, tiles or a GeoPackage in the package (D10). It opens
/// through KentOS's own service and base-map path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataSource {
    pub id: String,
    pub title: String,
    pub kind: DataSourceKind,
    /// A service's address; its server is in the `network` capability's `hosts`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// A GeoPackage in the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<String>,
    /// `EPSG:5254`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_note: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataSourceKind {
    Wms,
    Wmts,
    Xyz,
    Wfs,
    OgcapiFeatures,
    VectorTiles,
    Geopackage,
}

impl DataSourceKind {
    /// Its layers must be named (a service's layers or collections).
    fn names_layers(self) -> bool {
        matches!(
            self,
            DataSourceKind::Wms
                | DataSourceKind::Wmts
                | DataSourceKind::Wfs
                | DataSourceKind::OgcapiFeatures
        )
    }
}

/// The capability vocabulary v1, fixed in ADIM 16; widening it is a new version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Capability {
    #[serde(rename = "document.read")]
    DocumentRead,
    #[serde(rename = "document.write")]
    DocumentWrite,
    #[serde(rename = "datasets.read")]
    DatasetsRead,
    #[serde(rename = "datasets.create")]
    DatasetsCreate,
    #[serde(rename = "datasets.manage")]
    DatasetsManage,
    #[serde(rename = "ui.ribbonTab")]
    UiRibbonTab,
    #[serde(rename = "ui.panel")]
    UiPanel,
    #[serde(rename = "ui.contextMenu")]
    UiContextMenu,
    #[serde(rename = "processing.provider")]
    ProcessingProvider,
    #[serde(rename = "audit.provider")]
    AuditProvider,
    #[serde(rename = "expression.provider")]
    ExpressionProvider,
    #[serde(rename = "mapTool")]
    MapTool,
    #[serde(rename = "dataSource")]
    DataSource,
    #[serde(rename = "files.userSelected")]
    FilesUserSelected,
    #[serde(rename = "network")]
    Network,
}

impl Capability {
    pub const ALL: [Capability; 15] = [
        Capability::DocumentRead,
        Capability::DocumentWrite,
        Capability::DatasetsRead,
        Capability::DatasetsCreate,
        Capability::DatasetsManage,
        Capability::UiRibbonTab,
        Capability::UiPanel,
        Capability::UiContextMenu,
        Capability::ProcessingProvider,
        Capability::AuditProvider,
        Capability::ExpressionProvider,
        Capability::MapTool,
        Capability::DataSource,
        Capability::FilesUserSelected,
        Capability::Network,
    ];

    /// Its id in the manifest.
    pub fn id(self) -> &'static str {
        match self {
            Capability::DocumentRead => "document.read",
            Capability::DocumentWrite => "document.write",
            Capability::DatasetsRead => "datasets.read",
            Capability::DatasetsCreate => "datasets.create",
            Capability::DatasetsManage => "datasets.manage",
            Capability::UiRibbonTab => "ui.ribbonTab",
            Capability::UiPanel => "ui.panel",
            Capability::UiContextMenu => "ui.contextMenu",
            Capability::ProcessingProvider => "processing.provider",
            Capability::AuditProvider => "audit.provider",
            Capability::ExpressionProvider => "expression.provider",
            Capability::MapTool => "mapTool",
            Capability::DataSource => "dataSource",
            Capability::FilesUserSelected => "files.userSelected",
            Capability::Network => "network",
        }
    }

    /// What it lets the extension do, as the install dialog says it.
    pub fn title(self) -> &'static str {
        match self {
            Capability::DocumentRead => "Açık projeyi okur",
            Capability::DocumentWrite => "Açık projeye yazar",
            Capability::DatasetsRead => "Veri kümelerini okur",
            Capability::DatasetsCreate => "Yeni veri kümesi oluşturur",
            Capability::DatasetsManage => "Veri kümelerinin şemasını yönetir",
            Capability::UiRibbonTab => "Şeride sekme ekler",
            Capability::UiPanel => "Panel ekler",
            Capability::UiContextMenu => "Sağ tık menüsüne öğe ekler",
            Capability::ProcessingProvider => "İşlemler'e araç ekler",
            Capability::AuditProvider => "Denetim ekler",
            Capability::ExpressionProvider => "İfadelere işlev ekler",
            Capability::MapTool => "Harita aracı ekler",
            Capability::DataSource => "Veri kaynağı ekler",
            Capability::FilesUserSelected => "Sizin seçtiğiniz dosyaları okur",
            Capability::Network => "İnternete bağlanır",
        }
    }

    /// The scopes it may be asked in: the window's and the sources' only for what the extension owns, a file only
    /// the user's choice, the project's data the open project or what the user picks.
    pub fn scopes(self) -> &'static [Scope] {
        match self {
            Capability::DocumentRead | Capability::DocumentWrite => {
                &[Scope::CurrentProject, Scope::UserSelected]
            }
            Capability::DatasetsRead | Capability::DatasetsCreate | Capability::DatasetsManage => {
                &[
                    Scope::CurrentProject,
                    Scope::UserSelected,
                    Scope::PluginOwned,
                ]
            }
            Capability::FilesUserSelected => &[Scope::UserSelected],
            Capability::UiRibbonTab
            | Capability::UiPanel
            | Capability::UiContextMenu
            | Capability::ProcessingProvider
            | Capability::AuditProvider
            | Capability::ExpressionProvider
            | Capability::MapTool
            | Capability::DataSource
            | Capability::Network => &[Scope::PluginOwned],
        }
    }

    /// Asked for by what the manifest adds (a ribbon tab, a panel …), not by what the code may do.
    fn is_contribution(self) -> bool {
        matches!(
            self,
            Capability::UiRibbonTab
                | Capability::UiPanel
                | Capability::UiContextMenu
                | Capability::ProcessingProvider
                | Capability::AuditProvider
                | Capability::ExpressionProvider
                | Capability::MapTool
                | Capability::DataSource
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    UserSelected,
    CurrentProject,
    PluginOwned,
}

impl Scope {
    pub fn id(self) -> &'static str {
        match self {
            Scope::UserSelected => "user-selected",
            Scope::CurrentProject => "current-project",
            Scope::PluginOwned => "plugin-owned",
        }
    }
}

/// A capability the extension asks for: not a permission granted; the organisation's policy and the user decide, the
/// broker enforces (D5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestedCapability {
    pub id: Capability,
    pub scope: Scope,
    /// Why, in a short Turkish sentence (120 characters at most): shown at install and, changed, at every update.
    pub reason: String,
    /// `network` only: the servers it talks to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<String>,
    /// `network` only: whether it sends data to an outside AI service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sends_data_to_ai: Option<bool>,
}

/// What a file the manifest names is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileRole {
    License,
    Changelog,
    Icon,
    Schema,
    Translation,
    PythonProject,
    PythonLock,
    ToolDescriptor,
    PanelDefinition,
    Form,
    GeoPackage,
}

/// A file the manifest names: where, which and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    /// The JSON pointer to the field.
    pub at: String,
    pub path: String,
    pub role: FileRole,
}

/// The kind of thing a local id names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Kind {
    Command,
    MapTool,
    Processing,
    Audit,
    Expression,
    Panel,
    Form,
    DataSource,
}

impl Kind {
    fn field(self) -> &'static str {
        match self {
            Kind::Command => "command",
            Kind::MapTool => "mapTool",
            Kind::Processing => "processing",
            Kind::Audit => "audit",
            Kind::Expression => "expression",
            Kind::Panel => "panel",
            Kind::Form => "form",
            Kind::DataSource => "dataSource",
        }
    }

    fn words(self) -> &'static str {
        match self {
            Kind::Command => "komut",
            Kind::MapTool => "harita aracı",
            Kind::Processing => "İşlemler aracı",
            Kind::Audit => "denetim",
            Kind::Expression => "ifade işlevi",
            Kind::Panel => "panel",
            Kind::Form => "form",
            Kind::DataSource => "veri kaynağı",
        }
    }
}

/// The targets a ribbon or menu item may point at, as (field, value).
fn targets<'a>(
    command: &'a Option<String>,
    map_tool: &'a Option<String>,
    processing: &'a Option<String>,
    audit: &'a Option<String>,
    panel: &'a Option<String>,
    form: &'a Option<String>,
    data_source: &'a Option<String>,
) -> [(Kind, Option<&'a str>); 7] {
    [
        (Kind::Command, command.as_deref()),
        (Kind::MapTool, map_tool.as_deref()),
        (Kind::Processing, processing.as_deref()),
        (Kind::Audit, audit.as_deref()),
        (Kind::Panel, panel.as_deref()),
        (Kind::Form, form.as_deref()),
        (Kind::DataSource, data_source.as_deref()),
    ]
}

/// What serde said, in Turkish: the stable code, the message, and the field it is about when the path stops above it.
fn serde_words(message: &str) -> (&'static str, String, Option<String>) {
    let message = message.split(" at line ").next().unwrap_or(message);
    let quoted = |text: &str| -> Vec<String> {
        text.split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect()
    };
    if let Some(rest) = message.strip_prefix("unknown field ") {
        let names = quoted(rest);
        let field = names.first().cloned().unwrap_or_default();
        let known = names.get(1..).map(|n| n.join(", ")).unwrap_or_default();
        let text = if known.is_empty() {
            format!("Bilinmeyen alan “{field}”; burada alan yok. Alanı silin.")
        } else {
            format!(
                "Bilinmeyen alan “{field}”. Buradaki alanlar: {known}. Alanı silin ya da adını düzeltin."
            )
        };
        return ("manifest_unknown_field", text, Some(field));
    }
    if let Some(rest) = message.strip_prefix("missing field ") {
        let field = quoted(rest).into_iter().next().unwrap_or_default();
        return (
            "manifest_missing_field",
            format!("“{field}” alanı zorunlu; ekleyin."),
            Some(field),
        );
    }
    if let Some(rest) = message.strip_prefix("duplicate field ") {
        let field = quoted(rest).into_iter().next().unwrap_or_default();
        return (
            "manifest_duplicate_field",
            format!("“{field}” alanı iki kez yazılmış; birini silin."),
            Some(field),
        );
    }
    if let Some(rest) = message.strip_prefix("unknown variant ") {
        let names = quoted(rest);
        let value = names.first().cloned().unwrap_or_default();
        let known = names.get(1..).map(|n| n.join(", ")).unwrap_or_default();
        return (
            "manifest_bad_value",
            format!("“{value}” geçerli bir değer değil; geçerli değerler: {known}."),
            None,
        );
    }
    if let Some(rest) = message.strip_prefix("invalid type: ") {
        let (got, expected) = rest.split_once(", expected ").unwrap_or((rest, ""));
        return (
            "manifest_bad_type",
            format!(
                "Değerin türü yanlış: {} verilmiş, {} bekleniyor.",
                type_words(got),
                type_words(expected)
            ),
            None,
        );
    }
    if let Some(rest) = message.strip_prefix("invalid value: ") {
        let (got, expected) = rest.split_once(", expected ").unwrap_or((rest, ""));
        return (
            "manifest_bad_value",
            format!(
                "Değer geçersiz: {} verilmiş, {} bekleniyor.",
                type_words(got),
                type_words(expected)
            ),
            None,
        );
    }
    (
        "manifest_bad_value",
        format!("Değer okunamadı: {message}."),
        None,
    )
}

fn type_words(text: &str) -> String {
    let text = text.trim();
    let value = || text.split('`').nth(1).unwrap_or("").to_owned();
    match text {
        "u8" | "u16" | "u32" | "u64" | "usize" => "sıfır ya da pozitif tam sayı".to_owned(),
        "i32" | "i64" => "tam sayı".to_owned(),
        "f32" | "f64" => "sayı".to_owned(),
        "a string" => "metin".to_owned(),
        "a boolean" => "true ya da false".to_owned(),
        "a sequence" => "dizi ([…])".to_owned(),
        "a map" | "map" => "nesne ({…})".to_owned(),
        "sequence" => "dizi ([…])".to_owned(),
        "null" => "null".to_owned(),
        "unit" => "boş değer".to_owned(),
        _ if text.starts_with("struct ") => "nesne ({…})".to_owned(),
        _ if text.starts_with("enum ") || text == "variant identifier" => {
            "seçeneklerden biri".to_owned()
        }
        _ if text.starts_with("string ") => format!("metin {}", &text["string ".len()..]),
        _ if text.starts_with("integer ") => format!("tam sayı {}", value()),
        _ if text.starts_with("floating point ") => format!("ondalık sayı {}", value()),
        _ if text.starts_with("boolean ") => value(),
        _ => text.to_owned(),
    }
}

/// A local id: lower case letters, digits and hyphens, a letter first (`init-schema`).
pub(crate) fn is_local_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 63
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
        && !id.ends_with('-')
}

/// An extension's id: reverse domain, lower case, two parts at least (`tr.ornek.mebis`).
pub fn is_extension_id(id: &str) -> bool {
    id.len() <= 128 && id.split('.').count() >= 2 && id.split('.').all(is_local_id)
}

/// `module.path:function`, Python identifiers in ASCII.
fn is_handler(handler: &str) -> bool {
    let ident = |s: &str| {
        let b = s.as_bytes();
        !b.is_empty()
            && (b[0].is_ascii_alphabetic() || b[0] == b'_')
            && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
    };
    match handler.split_once(':') {
        Some((module, function)) => module.split('.').all(ident) && ident(function),
        None => false,
    }
}

/// A KentOS command's id (`cad.layers.service`).
fn is_command_id(id: &str) -> bool {
    id.split('.').count() >= 2
        && id.split('.').all(|part| {
            let b = part.as_bytes();
            !b.is_empty()
                && b[0].is_ascii_lowercase()
                && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
        })
}

/// A layer's schema name (`mezar_parsel`).
fn is_schema_name(name: &str) -> bool {
    let b = name.as_bytes();
    !b.is_empty()
        && b.len() <= 63
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

/// A server's name as `hosts` gives it: lower case ASCII labels with dots, or an IPv4 address.
pub(crate) fn is_host(host: &str) -> bool {
    host.len() <= 253
        && host.contains('.')
        && host.split('.').all(|label| {
            let b = label.as_bytes();
            !b.is_empty()
                && b.len() <= 63
                && b.iter()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

/// A language code (`tr`, `en`, `pt-BR`).
fn is_language(code: &str) -> bool {
    let (lang, region) = code
        .split_once('-')
        .map_or((code, None), |(l, r)| (l, Some(r)));
    (2..=3).contains(&lang.len())
        && lang.bytes().all(|c| c.is_ascii_lowercase())
        && region.is_none_or(|r| r.len() == 2 && r.bytes().all(|c| c.is_ascii_uppercase()))
}

/// A normalised Python extra's name (PEP 685).
fn is_extra(name: &str) -> bool {
    !name.is_empty()
        && name.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// What is wrong with an SPDX license expression's form (the license list itself is the catalog's).
fn spdx_problem(text: &str) -> Option<String> {
    let spaced = text.replace('(', " ( ").replace(')', " ) ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    if tokens.is_empty() {
        return Some("Lisans boş; SPDX ifadesi yazın (ör. GPL-3.0-or-later, MIT).".to_owned());
    }
    fn id(token: &str) -> bool {
        let token = token.strip_suffix('+').unwrap_or(token);
        !token.is_empty()
            && !matches!(token, "AND" | "OR" | "WITH" | "NONE" | "NOASSERTION")
            && token
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b':'))
    }
    // expression := term (AND|OR term)* ; term := '(' expression ')' | id [WITH id]
    fn expression(t: &[&str], at: &mut usize, depth: usize) -> bool {
        if depth > 16 || !term(t, at, depth) {
            return false;
        }
        while matches!(t.get(*at), Some(&"AND" | &"OR")) {
            *at += 1;
            if !term(t, at, depth) {
                return false;
            }
        }
        true
    }
    fn term(t: &[&str], at: &mut usize, depth: usize) -> bool {
        match t.get(*at) {
            Some(&"(") => {
                *at += 1;
                if !expression(t, at, depth + 1) || t.get(*at) != Some(&")") {
                    return false;
                }
                *at += 1;
                true
            }
            Some(token) if id(token) => {
                *at += 1;
                if t.get(*at) == Some(&"WITH") {
                    *at += 1;
                    if !t.get(*at).is_some_and(|e| id(e)) {
                        return false;
                    }
                    *at += 1;
                }
                true
            }
            _ => false,
        }
    }
    let mut at = 0;
    (!(expression(&tokens, &mut at, 0) && at == tokens.len())).then(|| {
        format!(
            "Lisans “{text}” bir SPDX ifadesi değil; kimlikleri AND, OR ve WITH ile bağlayın (ör. \
             GPL-3.0-or-later, “MIT OR Apache-2.0”). Kurum içi lisans için LicenseRef-… yazın."
        )
    })
}

/// A service address's server, lower case: `https://harita.ornek.bel.tr:8443/wmts?x` → `harita.ornek.bel.tr`. Refused:
/// another scheme, a user name in it, no server.
pub(crate) fn url_host(url: &str) -> Result<(String, bool), String> {
    let (rest, secure) = if let Some(rest) = url.strip_prefix("https://") {
        (rest, true)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (rest, false)
    } else {
        return Err(format!("“{url}” https:// ya da http:// ile başlamıyor."));
    };
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!("“{url}” boşluk ya da denetim karakteri içeriyor."));
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return Err(
            "Adreste kullanıcı adı ya da parola var; parola pakete girmez. Yetkilendirmeyi kurulumdan sonra \
             kullanıcı girer."
                .to_owned(),
        );
    }
    let host = match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => host,
        Some(_) => return Err(format!("“{url}” adresindeki sunucu adı okunamadı.")),
        None => authority,
    };
    let host = host.to_ascii_lowercase();
    if !is_host(&host) {
        return Err(format!(
            "“{url}” adresinin sunucusu “{host}” geçerli bir alan adı değil (ASCII, noktalı; Türkçe karakterli alan \
             adının xn-- biçimi yazılır)."
        ));
    }
    Ok((host, secure))
}

/// The work of [`Manifest::check`].
struct Checker<'a> {
    m: &'a Manifest,
    problems: Vec<Problem>,
    warnings: Vec<Problem>,
}

impl Checker<'_> {
    fn add(&mut self, code: &'static str, at: impl Into<String>, message: impl Into<String>) {
        self.problems
            .push(Problem::new(code, message).file(FILE).at(at));
    }

    fn warn(&mut self, code: &'static str, at: impl Into<String>, message: impl Into<String>) {
        self.warnings
            .push(Problem::new(code, message).file(FILE).at(at));
    }

    /// A one-line text that must be there: not empty, no line break or control character, `max` characters at most.
    fn line(&mut self, at: &str, label: &str, text: &str, max: usize) {
        if text.trim().is_empty() {
            self.add("manifest_text", at, format!("{label} boş olamaz."));
        } else if text.chars().any(char::is_control) {
            self.add(
                "manifest_text",
                at,
                format!("{label} tek satır olmalı; satır sonu ve denetim karakteri olmaz."),
            );
        } else if text.chars().count() > max {
            self.add(
                "manifest_text",
                at,
                format!(
                    "{label} {} karakter; en çok {max} olur. Kısaltın.",
                    text.chars().count()
                ),
            );
        } else if text.trim() != text {
            self.add(
                "manifest_text",
                at,
                format!("{label} boşlukla başlıyor ya da bitiyor; silin."),
            );
        }
    }

    fn local_id(&mut self, at: &str, id: &str) {
        if !is_local_id(id) {
            self.add(
                "manifest_local_id",
                at,
                format!(
                    "Kimlik “{id}” olmaz: küçük İngilizce harf, rakam ve tire, harfle başlar, en çok 63 karakter (ör. \
                     init-schema)."
                ),
            );
        }
    }

    fn handler(&mut self, at: &str, handler: &str) {
        if !is_handler(handler) {
            self.add(
                "manifest_handler",
                at,
                format!("“{handler}” modül.yolu:fonksiyon biçiminde değil (ör. mebis.logic:init_schema)."),
            );
        }
    }

    /// A path to a file in the package: its form and, when given, its ending.
    fn path(&mut self, at: &str, path: &str, ending: Option<&str>) {
        if let Some(problem) = paths::problem(path) {
            self.add("manifest_path", at, format!("“{path}”: {problem}"));
        } else if let Some(ending) = ending
            && !path.ends_with(ending)
        {
            self.add(
                "manifest_path",
                at,
                format!("“{path}” {ending} ile bitmeli."),
            );
        }
    }

    /// A built-in icon's name or an SVG in the package.
    fn icon(&mut self, at: &str, icon: &str) {
        if icon.contains('/') || icon.contains('.') {
            self.path(at, icon, Some(".svg"));
        } else if !is_local_id(icon) {
            self.add(
                "manifest_icon",
                at,
                format!(
                    "İkon “{icon}” ne yerleşik bir ikon adı (küçük harf ve tire) ne de paketteki bir .svg dosyasının yolu."
                ),
            );
        }
    }

    fn link(&mut self, at: &str, url: &str) {
        match url_host(url) {
            Err(problem) => self.add("manifest_link", at, problem),
            Ok((_, false)) => self.warn(
                "manifest_link_insecure",
                at,
                format!("“{url}” şifresiz (http); varsa https adresini yazın."),
            ),
            Ok(_) => {}
        }
    }

    fn when(&mut self, at: &str, when: &When, requested: &BTreeSet<Capability>) {
        if let Some(module) = when.module
            && !self.m.modules.contains(&module)
        {
            self.add(
                "manifest_when",
                format!("{at}/module"),
                format!(
                    "Koşuldaki “{}” programı “modules” listesinde yok.",
                    module.id()
                ),
            );
        }
        if let Some(schema) = &when.layer_schema
            && !is_schema_name(schema)
        {
            self.add(
                "manifest_when",
                format!("{at}/layerSchema"),
                format!("“{schema}” bir katman şeması adı değil: küçük harf, rakam ve alt çizgi (ör. mezar_parsel)."),
            );
        }
        if let Some(capability) = when.capability_granted
            && !requested.contains(&capability)
        {
            self.add(
                "manifest_when",
                format!("{at}/capabilityGranted"),
                format!(
                    "Koşul “{}” yeteneğine bakıyor ama paket bu yeteneği istemiyor; öğe hiç görünmezdi.",
                    capability.id()
                ),
            );
        }
    }

    /// A ribbon or menu item: exactly one target, an existing one of its kind.
    fn item(
        &mut self,
        at: &str,
        found: [(Kind, Option<&str>); 7],
        builtin: Option<&str>,
        ids: &BTreeMap<&str, Kind>,
    ) {
        let given: Vec<(Kind, &str)> = found
            .iter()
            .filter_map(|(k, v)| v.map(|v| (*k, v)))
            .collect();
        let count = given.len() + usize::from(builtin.is_some());
        if count != 1 {
            self.add(
                "manifest_item_target",
                at,
                format!(
                    "Öğe {} hedef gösteriyor; tam bir tanesini yazın: command, mapTool, processing, audit, panel, form, \
                     dataSource ya da builtin.",
                    if count == 0 { "hiçbir".to_owned() } else { count.to_string() }
                ),
            );
            return;
        }
        if let Some(command) = builtin {
            if command.starts_with("ext.") {
                self.add(
                    "manifest_builtin",
                    format!("{at}/builtin"),
                    "Başka bir eklentinin komutu gösterilmez; yalnız KentOS'un kendi komutları ya da bu paketin \
                     katkıları.",
                );
            } else if !is_command_id(command) {
                self.add(
                    "manifest_builtin",
                    format!("{at}/builtin"),
                    format!("“{command}” bir KentOS komut kimliği değil (ör. cad.layers.service)."),
                );
            }
            return;
        }
        let (kind, id) = given[0];
        if ids.get(id) != Some(&kind) {
            self.add(
                "manifest_item_unknown",
                format!("{at}/{}", kind.field()),
                format!(
                    "Öğe “{id}” adlı {}ı gösteriyor ama pakette öyle bir {} yok.",
                    kind.words(),
                    kind.words()
                ),
            );
        }
    }
}

impl Manifest {
    /// Reads `manifest.json` and checks it: every rule's problem at once, or the one that keeps it from being read.
    pub fn parse(text: &str) -> Result<Manifest, Vec<Problem>> {
        let manifest = Manifest::read(text).map_err(|problem| vec![problem])?;
        let (problems, _) = manifest.check();
        if problems.is_empty() {
            Ok(manifest)
        } else {
            Err(problems)
        }
    }

    /// Reads `manifest.json` into its types, its rules not checked ([`Manifest::check`]): the problem that keeps it
    /// from being read. A newer `schemaVersion` asks for a newer KentOS before any field is judged.
    pub fn read(text: &str) -> Result<Manifest, Problem> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| {
            Problem::new(
                "manifest_json",
                format!(
                    "manifest.json geçerli JSON değil: {}. Düzeltin.",
                    json_words(&e)
                ),
            )
            .file(FILE)
            .at(format!("satır {}, sütun {}", e.line(), e.column()))
        })?;
        let one = |code: &'static str, at: &str, message: String| {
            Problem::new(code, message).file(FILE).at(at)
        };
        let Some(object) = value.as_object() else {
            return Err(one(
                "manifest_json",
                "",
                "manifest.json bir JSON nesnesi ({…}) olmalı.".to_owned(),
            ));
        };
        match object.get("format").and_then(|f| f.as_str()) {
            Some(FORMAT) => {}
            Some(other) => {
                return Err(one(
                    "manifest_format",
                    "/format",
                    format!("Biçim “{other}”; bir KentOS eklentisi “{FORMAT}” yazar."),
                ));
            }
            None => {
                return Err(one(
                    "manifest_format",
                    "/format",
                    format!("“format” alanı yok ya da metin değil; “{FORMAT}” yazın."),
                ));
            }
        }
        match object
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
        {
            Some(1) => {}
            Some(newer) if newer > u64::from(SCHEMA_VERSION) => {
                return Err(one(
                    "manifest_newer",
                    "/schemaVersion",
                    format!(
                        "Bu paket manifest sürüm {newer} ile yazılmış; bu KentOS en çok {SCHEMA_VERSION}'i okur. \
                         KentOS'u güncelleyin."
                    ),
                ));
            }
            _ => {
                return Err(one(
                    "manifest_schema_version",
                    "/schemaVersion",
                    format!("“schemaVersion” {SCHEMA_VERSION} olmalı (tam sayı)."),
                ));
            }
        }
        serde_json::from_str::<Manifest>(text).map_err(|e| {
            let mut at = pointer::at_error(text, &e);
            let (code, message, field) = serde_words(&e.to_string());
            if let Some(field) = field
                && !at.ends_with(&format!("/{}", segment(&field)))
            {
                at.push('/');
                at.push_str(&segment(&field));
            }
            one(code, &at, message)
        })
    }

    /// Every rule that needs nothing but the manifest: the problems, then the warnings (what works but should be
    /// mended).
    pub fn check(&self) -> (Vec<Problem>, Vec<Problem>) {
        let mut c = Checker {
            m: self,
            problems: Vec::new(),
            warnings: Vec::new(),
        };
        let m = self;
        if !is_extension_id(&m.id) {
            c.add(
                "manifest_id",
                "/id",
                format!(
                    "Kimlik “{}” ters alan adı biçiminde değil: küçük İngilizce harf, rakam ve tire; noktayla ayrılmış en az \
                     iki parça (ör. tr.ornek.mebis).",
                    m.id
                ),
            );
        }
        match Version::parse(&m.version) {
            None => c.add(
                "manifest_version",
                "/version",
                format!(
                    "Sürüm “{}” anlamsal sürüm değil: MAJOR.MINOR.PATCH (ör. 1.0.0; önizleme için 1.1.0-beta.1).",
                    m.version
                ),
            ),
            Some(_) if m.version.contains('+') => c.add(
                "manifest_version",
                "/version",
                "Sürümdeki yapı eki (+…) sürümü ayırt etmez; iki farklı paket aynı sürüm görünürdü. Eki silin.",
            ),
            Some(_) => {}
        }
        c.line("/name", "Ad", &m.name, 80);
        c.line("/summary", "Özet", &m.summary, 160);
        if let Some(description) = &m.description {
            if description.trim().is_empty() {
                c.add(
                    "manifest_text",
                    "/description",
                    "Açıklama boş; yazın ya da alanı silin.",
                );
            } else if description.chars().count() > 4000 {
                c.add(
                    "manifest_text",
                    "/description",
                    "Açıklama en çok 4000 karakter olur.",
                );
            } else if description.as_bytes().windows(2).any(|w| {
                w[0] == b'<' && (w[1].is_ascii_alphabetic() || w[1] == b'/' || w[1] == b'!')
            }) {
                c.add(
                    "manifest_html",
                    "/description",
                    "Açıklama düz metindir; HTML etiketleri gösterilmez. Etiketleri silin.",
                );
            }
        }
        if m.authors.is_empty() {
            c.add(
                "manifest_authors",
                "/authors",
                "Yazar listesi boş; en az bir ad yazın.",
            );
        }
        for (i, author) in m.authors.iter().enumerate() {
            c.line(&format!("/authors/{i}"), "Yazar", author, 100);
            if author.contains('@') {
                c.add(
                    "manifest_authors",
                    format!("/authors/{i}"),
                    "Yazarda e-posta var; e-posta pakete girmez (kişisel veri). İletişimi katalog hesabı doğrular.",
                );
            }
        }
        if let Some(publisher) = &m.publisher_id
            && (!is_local_id(publisher) || publisher.len() < 2)
        {
            c.add(
                "manifest_publisher",
                "/publisherId",
                format!("Yayımcı “{publisher}” olmaz: küçük İngilizce harf, rakam ve tire (ör. ornek-belediye)."),
            );
        }
        if let Some(problem) = spdx_problem(&m.license) {
            c.add("manifest_license", "/license", problem);
        }
        c.path("/licenseFile", &m.license_file, None);
        if let Some(changelog) = &m.changelog_file {
            c.path("/changelogFile", changelog, None);
        }
        if let Some(icon) = &m.icon {
            c.path("/icon", icon, Some(".svg"));
        }
        for (field, link) in [
            ("homepage", &m.links.homepage),
            ("source", &m.links.source),
            ("issues", &m.links.issues),
        ] {
            if let Some(link) = link {
                c.link(&format!("/links/{field}"), link);
            }
        }
        if m.keywords.len() > 20 {
            c.add(
                "manifest_keyword",
                "/keywords",
                "En çok 20 anahtar sözcük olur.",
            );
        }
        let mut seen = BTreeSet::new();
        for (i, keyword) in m.keywords.iter().enumerate() {
            c.line(&format!("/keywords/{i}"), "Anahtar sözcük", keyword, 40);
            if !seen.insert(keyword.to_lowercase()) {
                c.add(
                    "manifest_keyword",
                    format!("/keywords/{i}"),
                    format!("“{keyword}” iki kez yazılmış."),
                );
            }
        }
        if m.modules.is_empty() {
            c.add(
                "manifest_modules",
                "/modules",
                "Program listesi boş; mimari, harita, cbs ya da yol yazın.",
            );
        }
        if m.modules.iter().collect::<BTreeSet<_>>().len() != m.modules.len() {
            c.add(
                "manifest_modules",
                "/modules",
                "Bir program iki kez yazılmış.",
            );
        }
        if m.runtimes.is_empty() {
            c.add(
                "manifest_runtimes",
                "/runtimes",
                "Çalışma ortamı listesi boş; desktop, headless ya da cloud yazın.",
            );
        }
        if m.runtimes.iter().collect::<BTreeSet<_>>().len() != m.runtimes.len() {
            c.add(
                "manifest_runtimes",
                "/runtimes",
                "Bir çalışma ortamı iki kez yazılmış.",
            );
        }
        self.check_compatibility(&mut c);
        self.check_dependencies(&mut c);
        if let Some(schema) = &m.schema_file {
            c.path("/schemaFile", schema, Some(".json"));
        }
        if let Some(l10n) = &m.l10n {
            if !is_language(&l10n.default) {
                c.add(
                    "manifest_l10n",
                    "/l10n/default",
                    format!("“{}” bir dil kodu değil (ör. tr, en, pt-BR).", l10n.default),
                );
            }
            for (language, file) in &l10n.files {
                let at = format!("/l10n/files/{}", segment(language));
                if !is_language(language) {
                    c.add(
                        "manifest_l10n",
                        &at,
                        format!("“{language}” bir dil kodu değil (ör. tr, en, pt-BR)."),
                    );
                } else if *language == l10n.default {
                    c.add(
                        "manifest_l10n",
                        &at,
                        format!(
                            "“{language}” varsayılan dil; manifest zaten bu dilde, çevirisi olmaz."
                        ),
                    );
                }
                c.path(&at, file, Some(".json"));
            }
        }
        if let Some(deprecation) = &m.deprecation {
            c.line(
                "/deprecation/message",
                "Emeklilik notu",
                &deprecation.message,
                300,
            );
            if let Some(next) = &deprecation.replaced_by
                && (!is_extension_id(next) || *next == m.id)
            {
                c.add(
                    "manifest_deprecation",
                    "/deprecation/replacedBy",
                    format!("“{next}” yerine geçen başka bir eklentinin kimliği olmalı."),
                );
            }
        }
        let ids = self.check_entrypoints(&mut c);
        self.check_python(&mut c);
        let requested: BTreeSet<Capability> =
            m.requested_capabilities.iter().map(|r| r.id).collect();
        self.check_contributions(&mut c, &ids, &requested);
        self.check_capabilities(&mut c);
        if m.entrypoints.is_empty() && m.contributions.is_empty() {
            c.add(
                "manifest_empty",
                "",
                "Paket hiçbir şey eklemiyor: bir komut, form, veri kaynağı ya da yerleşik komuta bağlı bir şerit \
                 düğmesi ekleyin.",
            );
        }
        (c.problems, c.warnings)
    }

    fn check_compatibility(&self, c: &mut Checker<'_>) {
        let compatibility = &self.compatibility;
        let protocol = compatibility.sdk_protocol;
        if protocol.min == 0 || protocol.min > protocol.max {
            c.add(
                "manifest_compat",
                "/compatibility/sdkProtocol",
                format!(
                    "Eklenti arayüzü aralığı {}–{} olmaz: en az 1, “min” “max”tan büyük olmaz.",
                    protocol.min, protocol.max
                ),
            );
        }
        if let Some(range) = &compatibility.kentos {
            let read = |c: &mut Checker<'_>, field: &str, text: &Option<String>| {
                text.as_deref().and_then(|text| {
                    let version = Version::parse(text);
                    if version.is_none() {
                        c.add(
                            "manifest_compat",
                            format!("/compatibility/kentos/{field}"),
                            format!("KentOS sürümü “{text}” anlamsal sürüm değil (ör. 0.2.0)."),
                        );
                    }
                    version
                })
            };
            let min = read(c, "min", &range.min);
            let max = read(c, "maxExclusive", &range.max_exclusive);
            if range.min.is_none() && range.max_exclusive.is_none() {
                c.add(
                    "manifest_compat",
                    "/compatibility/kentos",
                    "KentOS aralığı boş; “min” ya da “maxExclusive” yazın ya da alanı silin.",
                );
            }
            if let (Some(min), Some(max)) = (min, max)
                && min >= max
            {
                c.add(
                    "manifest_compat",
                    "/compatibility/kentos",
                    format!("KentOS aralığı boş: {min} ile {max} öncesi arasında sürüm yok."),
                );
            }
        }
        if compatibility
            .operating_systems
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != compatibility.operating_systems.len()
        {
            c.add(
                "manifest_compat",
                "/compatibility/operatingSystems",
                "Bir işletim sistemi iki kez yazılmış.",
            );
        }
    }

    fn check_dependencies(&self, c: &mut Checker<'_>) {
        let mut seen = BTreeSet::new();
        for (i, dependency) in self.dependencies.extensions.iter().enumerate() {
            let at = format!("/dependencies/extensions/{i}");
            if !is_extension_id(&dependency.id) {
                c.add(
                    "manifest_dependency",
                    format!("{at}/id"),
                    format!(
                        "“{}” bir eklenti kimliği değil (ör. tr.ornek.kent-temel).",
                        dependency.id
                    ),
                );
            } else if dependency.id == self.id {
                c.add(
                    "manifest_dependency_cycle",
                    format!("{at}/id"),
                    "Eklenti kendisine bağımlı olamaz.",
                );
            } else if !seen.insert(dependency.id.as_str()) {
                c.add(
                    "manifest_dependency",
                    format!("{at}/id"),
                    format!(
                        "“{}” iki kez yazılmış; tek kayıtta birleştirin.",
                        dependency.id
                    ),
                );
            }
            if VersionReq::parse(&dependency.version).is_none() {
                c.add(
                    "manifest_dependency",
                    format!("{at}/version"),
                    format!(
                        "Sürüm aralığı “{}” okunamadı: virgülle ayrılmış koşullar (ör. “>=1.2, <2”, “^1.2”, “~1.2.3”).",
                        dependency.version
                    ),
                );
            }
        }
    }

    /// The entrypoints' own rules; their local ids and kinds, ids unique across them and the data sources.
    fn check_entrypoints<'m>(&'m self, c: &mut Checker<'_>) -> BTreeMap<&'m str, Kind> {
        let e = &self.entrypoints;
        let mut ids: BTreeMap<&str, Kind> = BTreeMap::new();
        let mut claim = |c: &mut Checker<'_>, at: String, id: &'m str, kind: Kind| {
            c.local_id(&format!("{at}/id"), id);
            if let Some(&other) = ids.get(id) {
                c.add(
                    "manifest_duplicate_id",
                    format!("{at}/id"),
                    format!(
                        "“{id}” kimliği hem bir {} hem bir {} için kullanılmış; katkı kimlikleri pakette tektir.",
                        other.words(),
                        kind.words()
                    ),
                );
            } else {
                ids.insert(id, kind);
            }
        };
        for (i, entry) in e.commands.iter().enumerate() {
            let at = format!("/entrypoints/commands/{i}");
            claim(c, at.clone(), &entry.id, Kind::Command);
            c.line(&format!("{at}/title"), "Başlık", &entry.title, 80);
            if let Some(icon) = &entry.icon {
                c.icon(&format!("{at}/icon"), icon);
            }
            c.handler(&format!("{at}/handler"), &entry.handler);
        }
        for (i, entry) in e.map_tools.iter().enumerate() {
            let at = format!("/entrypoints/mapTools/{i}");
            claim(c, at.clone(), &entry.id, Kind::MapTool);
            c.line(&format!("{at}/title"), "Başlık", &entry.title, 80);
            if let Some(icon) = &entry.icon {
                c.icon(&format!("{at}/icon"), icon);
            }
            c.handler(&format!("{at}/handler"), &entry.handler);
            match (entry.template_width_m, entry.template_length_m) {
                (None, None) => {}
                (Some(w), Some(l)) => {
                    if !matches!(
                        entry.capture,
                        Capture::Rectangle | Capture::OrientedRectangle
                    ) {
                        c.add(
                            "manifest_map_tool",
                            &at,
                            "Şablon ölçüsü yalnız dikdörtgen yakalamada (rectangle, orientedRectangle) olur.",
                        );
                    }
                    if !(w.is_finite()
                        && l.is_finite()
                        && w > 0.0
                        && l > 0.0
                        && w <= 1000.0
                        && l <= 1000.0)
                    {
                        c.add(
                            "manifest_map_tool",
                            &at,
                            "Şablon ölçüleri metre cinsinden 0'dan büyük ve en çok 1000 olur.",
                        );
                    }
                }
                _ => c.add(
                    "manifest_map_tool",
                    &at,
                    "Şablonun eni ile boyu birlikte yazılır (templateWidthM ve templateLengthM).",
                ),
            }
            for (j, layer) in entry.snap_layers.iter().enumerate() {
                if !is_schema_name(layer) {
                    c.add(
                        "manifest_map_tool",
                        format!("{at}/snapLayers/{j}"),
                        format!(
                            "“{layer}” bir katman şeması adı değil: küçük harf, rakam ve alt çizgi."
                        ),
                    );
                }
            }
        }
        for (i, entry) in e.processing.iter().enumerate() {
            let at = format!("/entrypoints/processing/{i}");
            claim(c, at.clone(), &entry.id, Kind::Processing);
            c.path(
                &format!("{at}/descriptor"),
                &entry.descriptor,
                Some(".tool.json"),
            );
            c.handler(&format!("{at}/handler"), &entry.handler);
        }
        for (i, entry) in e.audits.iter().enumerate() {
            let at = format!("/entrypoints/audits/{i}");
            claim(c, at.clone(), &entry.id, Kind::Audit);
            c.line(&format!("{at}/title"), "Başlık", &entry.title, 80);
            c.handler(&format!("{at}/handler"), &entry.handler);
        }
        for (i, entry) in e.expressions.iter().enumerate() {
            let at = format!("/entrypoints/expressions/{i}");
            claim(c, at.clone(), &entry.id, Kind::Expression);
            if let Some(title) = &entry.title {
                c.line(&format!("{at}/title"), "Başlık", title, 80);
            }
            c.handler(&format!("{at}/handler"), &entry.handler);
        }
        for (i, entry) in e.panels.iter().enumerate() {
            let at = format!("/entrypoints/panels/{i}");
            claim(c, at.clone(), &entry.id, Kind::Panel);
            c.line(&format!("{at}/title"), "Başlık", &entry.title, 80);
            c.path(
                &format!("{at}/definition"),
                &entry.definition,
                Some(".panel.json"),
            );
        }
        for (i, entry) in e.forms.iter().enumerate() {
            let at = format!("/entrypoints/forms/{i}");
            claim(c, at.clone(), &entry.id, Kind::Form);
            c.path(
                &format!("{at}/definition"),
                &entry.definition,
                Some(".kform"),
            );
        }
        for (i, source) in self.contributions.data_sources.iter().enumerate() {
            claim(
                c,
                format!("/contributions/dataSources/{i}"),
                &source.id,
                Kind::DataSource,
            );
        }
        ids
    }

    fn check_python(&self, c: &mut Checker<'_>) {
        let handlers = self.handlers();
        match &self.python {
            None if !handlers.is_empty() => c.add(
                "manifest_python_missing",
                handlers[0].0.clone(),
                "Katkılar Python işlevi çağırıyor ama “python” alanı yok: projectFile (pyproject.toml) ve lockFile \
                 (uv.lock) yazın.",
            ),
            None => {}
            Some(python) => {
                if handlers.is_empty() {
                    c.warn(
                        "manifest_python_unused",
                        "/python",
                        "Python projesi var ama hiçbir katkı onu çağırmıyor; “python” alanını ve dosyalarını silin.",
                    );
                }
                c.path("/python/projectFile", &python.project_file, Some("pyproject.toml"));
                c.path("/python/lockFile", &python.lock_file, Some("uv.lock"));
                let folder = |p: &str| p.rsplit_once('/').map_or("", |(f, _)| f).to_owned();
                if folder(&python.project_file) != folder(&python.lock_file) {
                    c.add(
                        "manifest_python",
                        "/python/lockFile",
                        "uv.lock, pyproject.toml ile aynı klasörde olmalı.",
                    );
                }
                for (key, feature) in &python.optional_features {
                    let at = format!("/python/optionalFeatures/{}", segment(key));
                    c.local_id(&at, key);
                    if !is_extra(&feature.extra) {
                        c.add(
                            "manifest_python",
                            format!("{at}/extra"),
                            format!(
                                "“{}” bir Python ek bağımlılık grubu adı değil: küçük harf, rakam ve tire.",
                                feature.extra
                            ),
                        );
                    }
                    c.line(&format!("{at}/label"), "Seçenek adı", &feature.label, 60);
                }
            }
        }
    }

    fn check_contributions(
        &self,
        c: &mut Checker<'_>,
        ids: &BTreeMap<&str, Kind>,
        requested: &BTreeSet<Capability>,
    ) {
        let contributions = &self.contributions;
        if self.kind == ExtensionKind::Utility && !contributions.ribbon_tabs.is_empty() {
            c.add(
                "manifest_utility_tab",
                "/contributions/ribbonTabs",
                "Yardımcı (utility) eklenti şerit sekmesi açamaz; düğmelerini sağ tık menüsüne koyun ya da “kind”ı \
                 “vertical” yapın.",
            );
        }
        let mut tabs = BTreeSet::new();
        for (i, tab) in contributions.ribbon_tabs.iter().enumerate() {
            let at = format!("/contributions/ribbonTabs/{i}");
            c.local_id(&format!("{at}/id"), &tab.id);
            if !tabs.insert(tab.id.as_str()) {
                c.add(
                    "manifest_duplicate_id",
                    format!("{at}/id"),
                    format!("“{}” sekmesi iki kez tanımlanmış.", tab.id),
                );
            }
            c.line(&format!("{at}/label"), "Sekme adı", &tab.label, 40);
            if !self.modules.contains(&tab.module) {
                c.add(
                    "manifest_tab_module",
                    format!("{at}/module"),
                    format!(
                        "Sekme “{}” programında ama bu program “modules” listesinde yok.",
                        tab.module.id()
                    ),
                );
            }
            if tab.panels.is_empty() {
                c.add(
                    "manifest_tab_empty",
                    format!("{at}/panels"),
                    "Sekmede hiç panel yok.",
                );
            }
            for (j, panel) in tab.panels.iter().enumerate() {
                let at = format!("{at}/panels/{j}");
                c.line(&format!("{at}/label"), "Panel adı", &panel.label, 40);
                if panel.items.is_empty() {
                    c.add(
                        "manifest_tab_empty",
                        format!("{at}/items"),
                        "Panelde hiç düğme yok.",
                    );
                }
                for (k, item) in panel.items.iter().enumerate() {
                    let at = format!("{at}/items/{k}");
                    let found = targets(
                        &item.command,
                        &item.map_tool,
                        &item.processing,
                        &item.audit,
                        &item.panel,
                        &item.form,
                        &item.data_source,
                    );
                    c.item(&at, found, item.builtin.as_deref(), ids);
                    if let Some(label) = &item.label {
                        c.line(&format!("{at}/label"), "Düğme adı", label, 40);
                    }
                    if item.builtin.is_some() && item.label.is_none() {
                        c.add(
                            "manifest_item_label",
                            &at,
                            "Yerleşik komuta bağlı düğmenin adı (label) yazılır; sekmede neyi açtığı görünsün.",
                        );
                    }
                    if let Some(icon) = &item.icon {
                        c.icon(&format!("{at}/icon"), icon);
                    }
                    if let Some(when) = &item.when {
                        c.when(&format!("{at}/when"), when, requested);
                    }
                }
            }
        }
        for (i, item) in contributions.context_menus.iter().enumerate() {
            let at = format!("/contributions/contextMenus/{i}");
            let found = targets(
                &item.command,
                &item.map_tool,
                &item.processing,
                &item.audit,
                &item.panel,
                &item.form,
                &item.data_source,
            );
            c.item(&at, found, item.builtin.as_deref(), ids);
            if let Some(label) = &item.label {
                c.line(&format!("{at}/label"), "Menü adı", label, 60);
            }
            if item.builtin.is_some() && item.label.is_none() {
                c.add(
                    "manifest_item_label",
                    &at,
                    "Yerleşik komuta bağlı menü öğesinin adı (label) yazılır.",
                );
            }
            if let Some(icon) = &item.icon {
                c.icon(&format!("{at}/icon"), icon);
            }
            if let Some(when) = &item.when {
                c.when(&format!("{at}/when"), when, requested);
            }
        }
        let hosts: BTreeSet<&str> = self
            .requested_capabilities
            .iter()
            .filter(|r| r.id == Capability::Network)
            .flat_map(|r| r.hosts.iter().map(String::as_str))
            .collect();
        for (i, source) in contributions.data_sources.iter().enumerate() {
            let at = format!("/contributions/dataSources/{i}");
            c.line(&format!("{at}/title"), "Başlık", &source.title, 80);
            match (source.kind, &source.url, &source.file) {
                (DataSourceKind::Geopackage, None, Some(file)) => c.path(&format!("{at}/file"), file, Some(".gpkg")),
                (DataSourceKind::Geopackage, _, _) => c.add(
                    "manifest_data_source",
                    &at,
                    "GeoPackage kaynağı paketteki bir .gpkg dosyasını gösterir: “file” yazın, “url” yazmayın.",
                ),
                (_, Some(url), None) => match url_host(url) {
                    Err(problem) => c.add("manifest_data_source", format!("{at}/url"), problem),
                    Ok((host, secure)) => {
                        if !hosts.contains(host.as_str()) {
                            c.add(
                                "manifest_data_source_host",
                                format!("{at}/url"),
                                format!(
                                    "Kaynağın sunucusu “{host}” “network” yeteneğinin “hosts” listesinde yok; kullanıcı \
                                     eklentinin nereye bağlandığını kurmadan önce görmeli. Listeye ekleyin."
                                ),
                            );
                        }
                        if !secure {
                            c.warn(
                                "manifest_link_insecure",
                                format!("{at}/url"),
                                format!("“{url}” şifresiz (http); servis https veriyorsa onu yazın."),
                            );
                        }
                        if url.len() > 2000 {
                            c.add("manifest_data_source", format!("{at}/url"), "Adres en çok 2000 karakter olur.");
                        }
                    }
                },
                _ => c.add(
                    "manifest_data_source",
                    &at,
                    "Servis kaynağı adresini “url” ile verir; “file” yalnız GeoPackage içindir.",
                ),
            }
            if source.kind.names_layers() && source.layers.is_empty() {
                c.add(
                    "manifest_data_source",
                    format!("{at}/layers"),
                    "Servisin hangi katmanı açılacak: “layers” listesine en az bir katman adı yazın.",
                );
            }
            for (j, layer) in source.layers.iter().enumerate() {
                c.line(&format!("{at}/layers/{j}"), "Katman adı", layer, 200);
            }
            if let Some(crs) = &source.crs {
                let ok = crs.split_once(':').is_some_and(|(authority, code)| {
                    !authority.is_empty()
                        && authority
                            .bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
                        && !code.is_empty()
                        && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
                });
                if !ok {
                    c.add(
                        "manifest_data_source",
                        format!("{at}/crs"),
                        format!("“{crs}” bir koordinat sistemi kimliği değil (ör. EPSG:5254)."),
                    );
                }
            }
            if let Some(attribution) = &source.attribution {
                c.line(
                    &format!("{at}/attribution"),
                    "Kaynak gösterme",
                    attribution,
                    200,
                );
            }
            if let Some(note) = &source.license_note {
                c.line(&format!("{at}/licenseNote"), "Lisans notu", note, 500);
            }
        }
    }

    fn check_capabilities(&self, c: &mut Checker<'_>) {
        let mut seen = BTreeSet::new();
        for (i, request) in self.requested_capabilities.iter().enumerate() {
            let at = format!("/requestedCapabilities/{i}");
            let id = request.id;
            if !seen.insert(id) {
                c.add(
                    "manifest_capability_duplicate",
                    format!("{at}/id"),
                    format!("“{}” iki kez istenmiş; tek kayıtta birleştirin.", id.id()),
                );
            }
            if !id.scopes().contains(&request.scope) {
                let allowed: Vec<&str> = id.scopes().iter().map(|s| s.id()).collect();
                c.add(
                    "manifest_capability_scope",
                    format!("{at}/scope"),
                    format!(
                        "“{}” “{}” kapsamında istenmez; geçerli kapsamlar: {}.",
                        id.id(),
                        request.scope.id(),
                        allowed.join(", ")
                    ),
                );
            }
            let reason = request.reason.trim();
            if reason.is_empty() {
                c.add(
                    "manifest_capability_reason",
                    format!("{at}/reason"),
                    format!(
                        "“{}” için gerekçe yok; kullanıcıya neden gerektiğini kısa bir Türkçe cümleyle yazın.",
                        id.id()
                    ),
                );
            } else {
                c.line(&format!("{at}/reason"), "Gerekçe", &request.reason, 120);
            }
            if id == Capability::Network {
                if request.hosts.is_empty() {
                    c.add(
                        "manifest_capability_network",
                        format!("{at}/hosts"),
                        "“network” hangi sunuculara bağlanacağını “hosts” ile söyler (ör. harita.ornek.bel.tr).",
                    );
                }
                let mut hosts = BTreeSet::new();
                for (j, host) in request.hosts.iter().enumerate() {
                    if !is_host(host) {
                        c.add(
                            "manifest_capability_network",
                            format!("{at}/hosts/{j}"),
                            format!(
                                "“{host}” bir sunucu adı değil: küçük harfli, noktalı alan adı ya da IPv4 adresi; şema, \
                                 yol ve joker (*) yazılmaz."
                            ),
                        );
                    } else if !hosts.insert(host.as_str()) {
                        c.add(
                            "manifest_capability_network",
                            format!("{at}/hosts/{j}"),
                            format!("“{host}” iki kez yazılmış."),
                        );
                    }
                }
                if request.sends_data_to_ai.is_none() {
                    c.add(
                        "manifest_capability_network",
                        format!("{at}/sendsDataToAi"),
                        "“network” dış bir yapay zekâ servisine veri gönderip göndermediğini söyler: sendsDataToAi \
                         true ya da false.",
                    );
                }
            } else if !request.hosts.is_empty() || request.sends_data_to_ai.is_some() {
                c.add(
                    "manifest_capability_network",
                    &at,
                    "“hosts” ve “sendsDataToAi” yalnız “network” yeteneğinde yazılır.",
                );
            }
        }
        let needed = self.needed_capabilities();
        let missing: Vec<String> = needed
            .iter()
            .filter(|(capability, _)| !seen.contains(capability))
            .map(|(capability, why)| format!("{} ({why})", capability.id()))
            .collect();
        if !missing.is_empty() {
            c.add(
                "manifest_capability_missing",
                "/requestedCapabilities",
                format!(
                    "Katkılar istenmemiş yetenek kullanıyor; kullanıcı kurmadan önce hepsini görmeli. Gerekçesiyle \
                     ekleyin: {}.",
                    missing.join("; ")
                ),
            );
        }
        for (i, request) in self.requested_capabilities.iter().enumerate() {
            if request.id.is_contribution() && !needed.contains_key(&request.id) {
                c.warn(
                    "manifest_capability_unused",
                    format!("/requestedCapabilities/{i}"),
                    format!(
                        "“{}” isteniyor ama onu kullanan katkı yok; silin.",
                        request.id.id()
                    ),
                );
            }
        }
    }

    /// The capabilities what the manifest adds needs, with the reason in words.
    fn needed_capabilities(&self) -> BTreeMap<Capability, &'static str> {
        let e = &self.entrypoints;
        let k = &self.contributions;
        let mut needed = BTreeMap::new();
        for (yes, capability, why) in [
            (
                !k.ribbon_tabs.is_empty(),
                Capability::UiRibbonTab,
                "şerit sekmesi",
            ),
            (
                !k.context_menus.is_empty(),
                Capability::UiContextMenu,
                "sağ tık menüsü",
            ),
            (!e.panels.is_empty(), Capability::UiPanel, "panel"),
            (
                !e.processing.is_empty(),
                Capability::ProcessingProvider,
                "İşlemler aracı",
            ),
            (!e.audits.is_empty(), Capability::AuditProvider, "denetim"),
            (
                !e.expressions.is_empty(),
                Capability::ExpressionProvider,
                "ifade işlevi",
            ),
            (!e.map_tools.is_empty(), Capability::MapTool, "harita aracı"),
            (
                !k.data_sources.is_empty(),
                Capability::DataSource,
                "veri kaynağı",
            ),
            (
                k.data_sources.iter().any(|s| s.url.is_some()),
                Capability::Network,
                "servis adresi olan veri kaynağı",
            ),
        ] {
            if yes {
                needed.insert(capability, why);
            }
        }
        needed
    }

    /// The handlers: (JSON pointer, `module.path:function`).
    pub fn handlers(&self) -> Vec<(String, &str)> {
        let e = &self.entrypoints;
        let mut out = Vec::new();
        out.extend(e.commands.iter().enumerate().map(|(i, x)| {
            (
                format!("/entrypoints/commands/{i}/handler"),
                x.handler.as_str(),
            )
        }));
        out.extend(e.map_tools.iter().enumerate().map(|(i, x)| {
            (
                format!("/entrypoints/mapTools/{i}/handler"),
                x.handler.as_str(),
            )
        }));
        out.extend(e.processing.iter().enumerate().map(|(i, x)| {
            (
                format!("/entrypoints/processing/{i}/handler"),
                x.handler.as_str(),
            )
        }));
        out.extend(e.audits.iter().enumerate().map(|(i, x)| {
            (
                format!("/entrypoints/audits/{i}/handler"),
                x.handler.as_str(),
            )
        }));
        out.extend(e.expressions.iter().enumerate().map(|(i, x)| {
            (
                format!("/entrypoints/expressions/{i}/handler"),
                x.handler.as_str(),
            )
        }));
        out
    }

    /// Every file the manifest names, where and what (icons by name are built in, not files).
    pub fn references(&self) -> Vec<Reference> {
        let mut out = Vec::new();
        let mut add = |at: String, path: &str, role: FileRole| {
            out.push(Reference {
                at,
                path: path.to_owned(),
                role,
            });
        };
        let is_file = |icon: &str| icon.contains('/') || icon.contains('.');
        add(
            "/licenseFile".to_owned(),
            &self.license_file,
            FileRole::License,
        );
        if let Some(path) = &self.changelog_file {
            add("/changelogFile".to_owned(), path, FileRole::Changelog);
        }
        if let Some(path) = &self.icon {
            add("/icon".to_owned(), path, FileRole::Icon);
        }
        if let Some(path) = &self.schema_file {
            add("/schemaFile".to_owned(), path, FileRole::Schema);
        }
        if let Some(l10n) = &self.l10n {
            for (language, path) in &l10n.files {
                add(
                    format!("/l10n/files/{}", segment(language)),
                    path,
                    FileRole::Translation,
                );
            }
        }
        if let Some(python) = &self.python {
            add(
                "/python/projectFile".to_owned(),
                &python.project_file,
                FileRole::PythonProject,
            );
            add(
                "/python/lockFile".to_owned(),
                &python.lock_file,
                FileRole::PythonLock,
            );
        }
        let e = &self.entrypoints;
        for (i, x) in e.commands.iter().enumerate() {
            if let Some(icon) = x.icon.as_deref().filter(|i| is_file(i)) {
                add(
                    format!("/entrypoints/commands/{i}/icon"),
                    icon,
                    FileRole::Icon,
                );
            }
        }
        for (i, x) in e.map_tools.iter().enumerate() {
            if let Some(icon) = x.icon.as_deref().filter(|i| is_file(i)) {
                add(
                    format!("/entrypoints/mapTools/{i}/icon"),
                    icon,
                    FileRole::Icon,
                );
            }
        }
        for (i, x) in e.processing.iter().enumerate() {
            add(
                format!("/entrypoints/processing/{i}/descriptor"),
                &x.descriptor,
                FileRole::ToolDescriptor,
            );
        }
        for (i, x) in e.panels.iter().enumerate() {
            add(
                format!("/entrypoints/panels/{i}/definition"),
                &x.definition,
                FileRole::PanelDefinition,
            );
        }
        for (i, x) in e.forms.iter().enumerate() {
            add(
                format!("/entrypoints/forms/{i}/definition"),
                &x.definition,
                FileRole::Form,
            );
        }
        let k = &self.contributions;
        for (i, tab) in k.ribbon_tabs.iter().enumerate() {
            for (j, panel) in tab.panels.iter().enumerate() {
                for (n, item) in panel.items.iter().enumerate() {
                    if let Some(icon) = item.icon.as_deref().filter(|i| is_file(i)) {
                        add(
                            format!("/contributions/ribbonTabs/{i}/panels/{j}/items/{n}/icon"),
                            icon,
                            FileRole::Icon,
                        );
                    }
                }
            }
        }
        for (i, item) in k.context_menus.iter().enumerate() {
            if let Some(icon) = item.icon.as_deref().filter(|i| is_file(i)) {
                add(
                    format!("/contributions/contextMenus/{i}/icon"),
                    icon,
                    FileRole::Icon,
                );
            }
        }
        for (i, source) in k.data_sources.iter().enumerate() {
            if let Some(file) = &source.file {
                add(
                    format!("/contributions/dataSources/{i}/file"),
                    file,
                    FileRole::GeoPackage,
                );
            }
        }
        out
    }

    /// The version, read (a checked manifest's always is).
    pub fn parsed_version(&self) -> Option<Version> {
        Version::parse(&self.version)
    }

    /// On the preview channel: its version is a pre-release.
    pub fn is_preview(&self) -> bool {
        self.parsed_version().is_some_and(|v| v.is_prerelease())
    }

    /// The capabilities it asks for.
    pub fn capabilities(&self) -> BTreeSet<Capability> {
        self.requested_capabilities.iter().map(|r| r.id).collect()
    }
}

/// serde_json's syntax error, in Turkish.
fn json_words(e: &serde_json::Error) -> String {
    let text = e.to_string();
    let text = text.split(" at line ").next().unwrap_or(&text);
    match text {
        "EOF while parsing an object"
        | "EOF while parsing a list"
        | "EOF while parsing a string"
        | "EOF while parsing a value" => {
            "dosya yarıda bitiyor (kapanmamış parantez ya da tırnak)".to_owned()
        }
        "trailing comma" => "son öğeden sonra virgül var".to_owned(),
        "key must be a string" => "anahtar çift tırnak içinde olmalı".to_owned(),
        "expected value" => "değer bekleniyordu".to_owned(),
        "expected `:`" => "anahtardan sonra “:” bekleniyordu".to_owned(),
        "expected `,` or `}`" => "“,” ya da “}” bekleniyordu".to_owned(),
        "expected `,` or `]`" => "“,” ya da “]” bekleniyordu".to_owned(),
        "trailing characters" => "JSON bittikten sonra fazladan karakter var".to_owned(),
        "control character (\\u0000-\\u001F) found while parsing a string" => {
            "metnin içinde denetim karakteri var (satır sonu \\n yazılır)".to_owned()
        }
        other => other.to_owned(),
    }
}

/// A dependency cycle among packages, given each one's id and the ids it depends on: the ids around it, the first
/// repeated at the end (`a → b → a`). The installer's (docs/adr/8057 §1): one package alone sees only itself.
pub fn dependency_cycle<'a>(packages: &[(&'a str, Vec<&'a str>)]) -> Option<Vec<String>> {
    let edges: BTreeMap<&str, &Vec<&str>> = packages.iter().map(|(id, deps)| (*id, deps)).collect();
    // 0: not seen, 1: on the path, 2: done.
    let mut state: BTreeMap<&str, u8> = BTreeMap::new();
    fn visit<'a>(
        id: &'a str,
        edges: &BTreeMap<&'a str, &Vec<&'a str>>,
        state: &mut BTreeMap<&'a str, u8>,
        path: &mut Vec<&'a str>,
    ) -> Option<Vec<String>> {
        match state.get(id) {
            Some(2) => return None,
            Some(1) => {
                let from = path.iter().position(|p| *p == id).unwrap_or(0);
                let mut cycle: Vec<String> = path[from..].iter().map(|s| (*s).to_owned()).collect();
                cycle.push(id.to_owned());
                return Some(cycle);
            }
            _ => {}
        }
        state.insert(id, 1);
        path.push(id);
        let nexts: Vec<&'a str> = edges.get(id).map(|d| d.to_vec()).unwrap_or_default();
        for next in nexts {
            if let Some(cycle) = visit(next, edges, state, path) {
                return Some(cycle);
            }
        }
        path.pop();
        state.insert(id, 2);
        None
    }
    let mut path = Vec::new();
    for (id, _) in packages {
        if let Some(cycle) = visit(id, &edges, &mut state, &mut path) {
            return Some(cycle);
        }
    }
    None
}

#[cfg(test)]
mod tests;
