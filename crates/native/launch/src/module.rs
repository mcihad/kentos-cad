//! The KentOS modules (docs/adr/0801, 0802): KentOS Mimari, KentOS Harita,
//! KentOS GIS and KentOS Yol (plan/road.md), each an application of its own under its own name
//! ([`Module::program`]): one program file, `kentos-cad`, is all of them at
//! once and each under its name or `--modul`. A project belongs to a module by its type and its CAD work
//! (`ProjectSettings.workspace`, `cadWork`).

use serde::{Deserialize, Serialize};

/// A KentOS module. Its id is what the command line and the recent files
/// write: short, lowercase Turkish, never translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Module {
    /// Mimari CAD: buildings, floor plans, sections and elevations (a CAD
    /// project's `architecture` work).
    #[serde(rename = "mimari")]
    Architecture,
    /// Harita CAD: site plans, zoning, surveys (a CAD project's `map` work).
    #[serde(rename = "harita")]
    Map,
    /// GIS: a CBS project.
    #[serde(rename = "cbs")]
    Gis,
    /// Yol: road design, terrain, alignments, sections and earthworks (a CAD project's `road` work, plan/road.md).
    #[serde(rename = "yol")]
    Road,
}

impl Module {
    /// Every module, in the launcher's order.
    pub const ALL: [Module; 4] = [
        Module::Architecture,
        Module::Map,
        Module::Gis,
        Module::Road,
    ];

    /// The id the command line and the recent files write.
    pub const fn id(self) -> &'static str {
        match self {
            Module::Architecture => "mimari",
            Module::Map => "harita",
            Module::Gis => "cbs",
            Module::Road => "yol",
        }
    }

    /// The module by its id; `None` for one KentOS does not know.
    pub fn from_id(id: &str) -> Option<Module> {
        Module::ALL.into_iter().find(|m| m.id() == id)
    }

    /// The program's name as the window and the launcher show it.
    pub const fn title(self) -> &'static str {
        match self {
            Module::Architecture => "KentOS Mimari",
            Module::Map => "KentOS Harita",
            Module::Gis => "KentOS GIS",
            Module::Road => "KentOS Yol",
        }
    }

    /// The module's own program's name (docs/adr/0801, 0802 §1): the one KentOS program file under this
    /// name (a link to `kentos-cad`, or `kentos-cad --modul <id>`) works only as this module.
    pub const fn program(self) -> &'static str {
        match self {
            Module::Architecture => "kentos-mimari",
            Module::Map => "kentos-harita",
            Module::Gis => "kentos-gis",
            Module::Road => "kentos-yol",
        }
    }

    /// The module a program was started as, by the name it was started under (`kentos-mimari`, a path to it,
    /// `KENTOS-HARITA.EXE`): one program file is every module under its name (docs/adr/0802 §1); none for
    /// `kentos-cad` and any other name.
    pub fn of_program_name(name: &std::ffi::OsStr) -> Option<Module> {
        let stem = std::path::Path::new(name)
            .file_stem()?
            .to_str()?
            .to_ascii_lowercase();
        Module::ALL.into_iter().find(|m| m.program() == stem)
    }

    /// One line on what it is for.
    pub const fn note(self) -> &'static str {
        match self {
            Module::Architecture => "Mimari CAD: plan, kesit ve görünüş",
            Module::Map => "Harita CAD: vaziyet, imar ve halihazır",
            Module::Gis => "Coğrafi bilgi sistemi",
            Module::Road => "Yol projelendirme: arazi, güzergâh, kesit ve kübaj",
        }
    }

    /// The type its projects have, as a project file writes it (`ProjectSettings.workspace`): `cad` or `gis`.
    pub const fn project_type(self) -> &'static str {
        match self {
            Module::Architecture | Module::Map | Module::Road => "cad",
            Module::Gis => "gis",
        }
    }

    /// The CAD works its projects are drawn for, as a project file writes them (`ProjectSettings.cadWork`), the
    /// one a new project starts with first; a CBS module has none.
    pub const fn cad_works(self) -> &'static [&'static str] {
        match self {
            Module::Architecture => &["architecture"],
            Module::Map => &["map"],
            Module::Road => &["road"],
            Module::Gis => &[],
        }
    }

    /// The cloud catalog's project types its projects have (docs/adr/0802), as the server names them (`type`,
    /// `ProjectType::name`), the one a new cloud project starts with first: KentOS Mimari's a building's; KentOS
    /// Harita's every CAD type that is neither a building's nor a road's (general CAD, land readjustment, zoning plan,
    /// subdivision); KentOS Yol's a road's (plan/road.md); KentOS GIS's CBS. No type is two modules'.
    pub const fn project_types(self) -> &'static [&'static str] {
        match self {
            Module::Architecture => &["architecture"],
            Module::Map => &[
                "cad",
                "landReadjustment",
                "zoningPlan",
                "subdivision",
            ],
            Module::Road => &["road"],
            Module::Gis => &["gis"],
        }
    }

    /// The module a cloud project of this catalog type belongs to (`ProjectType::name`); none for a type no
    /// module has.
    pub fn of_project_type(name: &str) -> Option<Module> {
        Module::ALL
            .into_iter()
            .find(|m| m.project_types().contains(&name))
    }

    /// The module a project belongs to (docs/adr/0801), by its type and CAD work as the project file writes them:
    /// a CBS project (`gis`) KentOS GIS's, a CAD project (`cad`) its work's module; none for a CAD project not asked
    /// its work yet, a project not asked its type and any other.
    pub fn of_project(project_type: Option<&str>, cad_work: Option<&str>) -> Option<Module> {
        match project_type? {
            "gis" => Some(Module::Gis),
            "cad" => {
                let work = cad_work?;
                Module::ALL
                    .into_iter()
                    .find(|m| m.cad_works().contains(&work))
            }
            _ => None,
        }
    }

    /// Where its projects keep their data (docs/adr/9196, CLAUDE.md §25 rule 5): a CBS project's layers in a database,
    /// a CAD project whole in its project file.
    pub const fn storage(self) -> Storage {
        match self {
            Module::Gis => Storage::Database,
            Module::Architecture | Module::Map | Module::Road => Storage::File,
        }
    }

    /// The kinds of layer its projects hold (docs/adr/9195 §5, 9196 §2), in [`LayerKind::ALL`]'s order:
    /// - a CBS project's data is a database's tables; the project file keeps the annotation alone, no drawing or bound
    ///   layer;
    /// - KentOS Mimari's a building's drawing on its own coordinates, no map service, table or bound layer;
    /// - KentOS Harita's and Yol's every kind a drawing has on real coordinates, a CBS table as a read-only underlay.
    pub const fn layer_kinds(self) -> &'static [LayerKind] {
        use LayerKind::{Annotation, Bound, Data, Drawing, Group, PointCloud, Raster, Service};
        match self {
            Module::Gis => &[Annotation, Data, Service, Raster, PointCloud, Group],
            Module::Architecture => &[Drawing, Raster, PointCloud, Group],
            Module::Map | Module::Road => {
                &[Drawing, Annotation, Data, Bound, Service, Raster, PointCloud, Group]
            }
        }
    }
}

/// Where a module's projects keep their data (docs/adr/9196 §2): the two kinds a module chooses from; a third needs an
/// ADR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Storage {
    /// Every layer's objects and attributes are a database's table (GeoPackage, PostgreSQL, KentOS Bulut); the project
    /// file keeps the structure, the styles, the settings and the annotation, and is saved to a file, a database or the
    /// cloud. In the cloud, a database project only.
    Database,
    /// The whole project is its project file, saved to a file or to the cloud as the file's revisions.
    File,
}

impl Storage {
    /// Its name as the module catalog writes it (`catalog/v1/modules.json`).
    pub const fn id(self) -> &'static str {
        match self {
            Storage::Database => "database",
            Storage::File => "file",
        }
    }
}

/// What a layer is (docs/adr/9195 §5): the kinds a layer's menu is cut by, and a module's projects hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayerKind {
    /// Objects kept in the project file: a CAD drawing's layer.
    Drawing,
    /// Texts, dimensions and sheets kept in the project file: a layer typed Açıklama.
    Annotation,
    /// A table of a GeoPackage or a database drawn as it is now, its rows in their source.
    Data,
    /// Objects in the project bound to a table they are written to.
    Bound,
    /// Drawn from a map service, holding no objects.
    Service,
    /// Rasters alone.
    Raster,
    /// Point clouds alone.
    PointCloud,
    /// A group of layers.
    Group,
}

impl LayerKind {
    /// Every kind, in the order the tables list them.
    pub const ALL: [LayerKind; 8] = [
        LayerKind::Drawing,
        LayerKind::Annotation,
        LayerKind::Data,
        LayerKind::Bound,
        LayerKind::Service,
        LayerKind::Raster,
        LayerKind::PointCloud,
        LayerKind::Group,
    ];

    /// Its name as the module catalog writes it.
    pub const fn id(self) -> &'static str {
        match self {
            LayerKind::Drawing => "drawing",
            LayerKind::Annotation => "annotation",
            LayerKind::Data => "data",
            LayerKind::Bound => "bound",
            LayerKind::Service => "service",
            LayerKind::Raster => "raster",
            LayerKind::PointCloud => "pointCloud",
            LayerKind::Group => "group",
        }
    }

    /// Where its content is kept (docs/adr/9197 §2.4): a drawing's, an annotation's, a bound layer's, rasters' and point
    /// clouds' objects in the project file (a bound layer's written to its table too, the pixels and points in files
    /// beside); a data layer's rows in their source; a service's on its server; a group holds nothing of its own.
    pub const fn kept(self) -> Kept {
        match self {
            LayerKind::Drawing
            | LayerKind::Annotation
            | LayerKind::Bound
            | LayerKind::Raster
            | LayerKind::PointCloud => Kept::ProjectFile,
            LayerKind::Data => Kept::Source,
            LayerKind::Service => Kept::Service,
            LayerKind::Group => Kept::Nothing,
        }
    }

    /// What a client writes when it writes on a layer of this kind (docs/adr/9197 §2.4): the project file's objects, or
    /// a source's rows; nothing for a service's layer and a group.
    pub const fn written_as(self) -> Option<Writes> {
        match self {
            LayerKind::Drawing
            | LayerKind::Annotation
            | LayerKind::Bound
            | LayerKind::Raster
            | LayerKind::PointCloud => Some(Writes::ProjectFile),
            LayerKind::Data => Some(Writes::DataRows),
            LayerKind::Service | LayerKind::Group => None,
        }
    }

    /// Whether `client` writes on a layer of this kind in a project kept as `storage`.
    pub fn written_by(self, client: Client, storage: Storage) -> bool {
        self.written_as().is_some_and(|w| client.writes(storage).contains(&w))
    }

    /// The mobile behaviours a layer of this kind may take in a project kept as `storage` (docs/adr/9198 §2.2): to edit
    /// only where the mobile app writes the kind (a CBS project's data table); to show or not, every kind.
    pub fn mobile_choices(self, storage: Storage) -> &'static [Mobile] {
        if self.written_by(Client::MobileApp, storage) {
            &Mobile::ALL
        } else {
            &[Mobile::Show, Mobile::Hide]
        }
    }

    /// Its name in a sentence.
    pub const fn title(self) -> &'static str {
        match self {
            LayerKind::Drawing => "çizim",
            LayerKind::Annotation => "açıklama",
            LayerKind::Data => "veri tablosu",
            LayerKind::Bound => "bağlı",
            LayerKind::Service => "servis",
            LayerKind::Raster => "raster",
            LayerKind::PointCloud => "nokta bulutu",
            LayerKind::Group => "grup",
        }
    }
}

/// What a layer is on the mobile clients (docs/adr/9198): the desktop's project says it, as a layer's field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mobile {
    /// Shown, read only: what a layer is when its project says nothing.
    Show,
    /// Shown, its rows written by the mobile app.
    Edit,
    /// Not on the mobile clients.
    Hide,
}

impl Mobile {
    /// Every behaviour, in the order the menus list them.
    pub const ALL: [Mobile; 3] = [Mobile::Show, Mobile::Edit, Mobile::Hide];

    /// Its name as the module catalog and the layer's field write it.
    pub const fn id(self) -> &'static str {
        match self {
            Mobile::Show => "show",
            Mobile::Edit => "edit",
            Mobile::Hide => "hide",
        }
    }

    /// Its name in the interface.
    pub const fn title(self) -> &'static str {
        match self {
            Mobile::Show => "Göster",
            Mobile::Edit => "Düzenle",
            Mobile::Hide => "Gösterme",
        }
    }
}

/// Where a layer kind's content is kept (docs/adr/9197 §2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kept {
    /// In the project file.
    ProjectFile,
    /// In a table of a source: a GeoPackage, PostgreSQL or KentOS Bulut.
    Source,
    /// On a map service's server.
    Service,
    /// Nothing of its own: a group.
    Nothing,
}

impl Kept {
    /// Its name as the module catalog writes it.
    pub const fn id(self) -> &'static str {
        match self {
            Kept::ProjectFile => "projectFile",
            Kept::Source => "source",
            Kept::Service => "service",
            Kept::Nothing => "nothing",
        }
    }
}

/// What a client writes of a project (docs/adr/9197 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Writes {
    /// The project file: its structure, styles, settings and annotation; a CAD project whole.
    ProjectFile,
    /// The data layers' rows in their source.
    DataRows,
}

impl Writes {
    /// Its name as the module catalog writes it.
    pub const fn id(self) -> &'static str {
        match self {
            Writes::ProjectFile => "projectFile",
            Writes::DataRows => "dataRows",
        }
    }
}

/// A KentOS client (docs/adr/9197, CLAUDE.md §25 rules 6–7): what it writes of a module's project, by the project's
/// storage. Every client reads the module catalog and decides its screens by the project's module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Client {
    /// The desktop programs.
    Desktop,
    /// The web application (paused, docs/adr/0228): the desktop's rules when it returns.
    Web,
    /// A mobile viewer: reads every module's project, writes nothing; in the cloud with a read-only key.
    MobileViewer,
    /// The mobile application: writes a CBS project's data rows by server commands under the forms' rules and the
    /// roles; views a CAD project.
    MobileApp,
}

impl Client {
    /// Every client, in the catalog's order.
    pub const ALL: [Client; 4] = [
        Client::Desktop,
        Client::Web,
        Client::MobileViewer,
        Client::MobileApp,
    ];

    /// Its name as the module catalog writes it.
    pub const fn id(self) -> &'static str {
        match self {
            Client::Desktop => "desktop",
            Client::Web => "web",
            Client::MobileViewer => "mobileViewer",
            Client::MobileApp => "mobileApp",
        }
    }

    /// Its name in a sentence.
    pub const fn title(self) -> &'static str {
        match self {
            Client::Desktop => "Masaüstü",
            Client::Web => "Web",
            Client::MobileViewer => "Mobil görüntüleyici",
            Client::MobileApp => "Mobil uygulama",
        }
    }

    /// What it writes of a project kept as `storage` (docs/adr/9197 §2.1): a viewer nothing; the mobile application a
    /// CBS project's data rows alone, never a project file; the desktop (and the web, when it returns) the project file,
    /// and a CBS project's data rows too.
    pub const fn writes(self, storage: Storage) -> &'static [Writes] {
        match (self, storage) {
            (Client::Desktop | Client::Web, Storage::Database) => &[Writes::ProjectFile, Writes::DataRows],
            (Client::Desktop | Client::Web, Storage::File) => &[Writes::ProjectFile],
            (Client::MobileApp, Storage::Database) => &[Writes::DataRows],
            (Client::MobileApp, Storage::File) | (Client::MobileViewer, _) => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_go_both_ways_and_are_the_serde_names() {
        for m in Module::ALL {
            assert_eq!(Module::from_id(m.id()), Some(m));
            assert_eq!(
                serde_json::to_string(&m).expect("json"),
                format!("\"{}\"", m.id())
            );
        }
        assert_eq!(Module::from_id("Mimari"), None, "ids are exact");
        let programs: Vec<&str> = Module::ALL.iter().map(|m| m.program()).collect();
        assert_eq!(
            programs,
            ["kentos-mimari", "kentos-harita", "kentos-gis", "kentos-yol"]
        );
        assert_eq!(Module::from_id(""), None);
    }

    #[test]
    fn a_project_belongs_by_its_type_and_work() {
        let of = Module::of_project;
        assert_eq!(of(Some("gis"), None), Some(Module::Gis));
        // A CBS project's work is no work: the file's rules refuse one; here it is not looked at.
        assert_eq!(of(Some("gis"), Some("map")), Some(Module::Gis));
        assert_eq!(
            of(Some("cad"), Some("architecture")),
            Some(Module::Architecture)
        );
        assert_eq!(of(Some("cad"), Some("detail")), None, "three CAD works only");
        assert_eq!(of(Some("cad"), Some("map")), Some(Module::Map));
        assert_eq!(of(Some("cad"), Some("road")), Some(Module::Road));
        // Not asked yet, or not known: no module.
        assert_eq!(of(Some("cad"), None), None);
        assert_eq!(of(Some("cad"), Some("survey")), None);
        assert_eq!(of(None, None), None);
        assert_eq!(of(None, Some("map")), None);
        assert_eq!(of(Some("hybrid"), None), None);
        assert_eq!(of(Some("plan3d"), None), None);
        // Every work a module names is its own and none is two modules'.
        for m in Module::ALL {
            assert_eq!(m.cad_works().is_empty(), m.project_type() == "gis");
            for w in m.cad_works() {
                assert_eq!(of(Some(m.project_type()), Some(w)), Some(m));
            }
        }
    }

    #[test]
    fn a_cloud_projects_type_is_one_modules() {
        assert_eq!(Module::Architecture.project_types(), ["architecture"]);
        assert_eq!(
            Module::Map.project_types(),
            [
                "cad",
                "landReadjustment",
                "zoningPlan",
                "subdivision"
            ]
        );
        assert_eq!(Module::Road.project_types(), ["road"]);
        assert_eq!(Module::Gis.project_types(), ["gis"]);
        // The seven catalog types the server knows, each one module's; a type it does not know, none's.
        let every = [
            "cad",
            "gis",
            "landReadjustment",
            "zoningPlan",
            "subdivision",
            "road",
            "architecture",
        ];
        for t in every {
            let owners: Vec<Module> = Module::ALL
                .into_iter()
                .filter(|m| m.project_types().contains(&t))
                .collect();
            assert_eq!(owners.len(), 1, "{t}: {owners:?}");
            assert_eq!(Module::of_project_type(t), Some(owners[0]));
        }
        let listed: usize = Module::ALL.iter().map(|m| m.project_types().len()).sum();
        assert_eq!(listed, every.len(), "no type is listed twice");
        assert_eq!(
            Module::of_project_type("Architecture"),
            None,
            "names are exact"
        );
        assert_eq!(Module::of_project_type("hybrid"), None);
        // A module's projects are of its own kind: a CBS module's of the CBS type, a CAD module's never.
        for m in Module::ALL {
            assert_eq!(
                m.project_types().contains(&"gis"),
                m.project_type() == "gis"
            );
        }
    }

    #[test]
    fn a_program_is_the_module_it_is_named_for() {
        use std::ffi::OsStr;
        for m in Module::ALL {
            assert_eq!(Module::of_program_name(OsStr::new(m.program())), Some(m));
        }
        let named = |name: &str| Module::of_program_name(OsStr::new(name));
        assert_eq!(named("/opt/kentos/bin/kentos-harita"), Some(Module::Map));
        assert_eq!(named("KENTOS-GIS.EXE"), Some(Module::Gis));
        assert_eq!(named("./kentos-mimari"), Some(Module::Architecture));
        assert_eq!(named("KentOS-Yol"), Some(Module::Road));
        assert_eq!(named("kentos-cad"), None, "every module at once");
        assert_eq!(named("kentos-launcher"), None);
        assert_eq!(named(""), None);
    }

    #[test]
    fn a_modules_storage_and_layers_follow_its_project_type() {
        for m in Module::ALL {
            // CBS keeps its data in a database, CAD its whole project in the file (docs/adr/9196).
            let cbs = m.project_type() == "gis";
            assert_eq!(m.storage() == Storage::Database, cbs, "{m:?}");
            let kinds = m.layer_kinds();
            // A database project holds no layer whose objects are in the file but its annotation; a file project
            // always draws.
            assert_eq!(kinds.contains(&LayerKind::Drawing), !cbs, "{m:?}");
            assert_eq!(kinds.contains(&LayerKind::Bound), !cbs && m != Module::Architecture, "{m:?}");
            assert!(kinds.contains(&LayerKind::Group));
            // In the tables' order, each once.
            let order: Vec<usize> = kinds
                .iter()
                .map(|k| LayerKind::ALL.iter().position(|a| a == k).expect("known"))
                .collect();
            assert!(order.windows(2).all(|w| w[0] < w[1]), "{m:?}");
        }
        assert_eq!(serde_json::to_string(&Storage::Database).expect("json"), "\"database\"");
        for k in LayerKind::ALL {
            assert_eq!(serde_json::to_string(&k).expect("json"), format!("\"{}\"", k.id()));
        }
    }

    #[test]
    fn a_client_writes_what_its_role_and_the_storage_allow() {
        for storage in [Storage::Database, Storage::File] {
            // A viewer writes nothing.
            assert!(Client::MobileViewer.writes(storage).is_empty());
            // The desktop always writes the project file.
            assert!(Client::Desktop.writes(storage).contains(&Writes::ProjectFile));
            // The mobile application never does.
            assert!(!Client::MobileApp.writes(storage).contains(&Writes::ProjectFile));
            // The web, when it returns, keeps the desktop's rules.
            assert_eq!(Client::Web.writes(storage), Client::Desktop.writes(storage));
        }
        assert_eq!(Client::MobileApp.writes(Storage::Database), [Writes::DataRows]);
        assert!(Client::MobileApp.writes(Storage::File).is_empty(), "a CAD project is viewed");
        // On a module's layers: the mobile application writes a CBS project's data tables alone; nothing of a CAD
        // project; a viewer nothing anywhere; services and groups no client writes.
        for m in Module::ALL {
            for k in m.layer_kinds() {
                let app = k.written_by(Client::MobileApp, m.storage());
                assert_eq!(app, m.storage() == Storage::Database && *k == LayerKind::Data, "{m:?} {k:?}");
                assert!(!k.written_by(Client::MobileViewer, m.storage()), "{m:?} {k:?}");
                if matches!(k, LayerKind::Service | LayerKind::Group) {
                    assert!(!k.written_by(Client::Desktop, m.storage()), "{m:?} {k:?}");
                }
            }
        }
        // A data table is in its source, the rest of what draws in the project file.
        assert_eq!(LayerKind::Data.kept(), Kept::Source);
        assert_eq!(LayerKind::Annotation.kept(), Kept::ProjectFile);
        for c in Client::ALL {
            assert_eq!(serde_json::to_string(&c).expect("json"), format!("\"{}\"", c.id()));
        }
    }

    /// To edit on the phone is the mobile app's alone, on a CBS project's data table; every layer may be shown or not
    /// (docs/adr/9198 §2.2).
    #[test]
    fn only_a_cbs_data_table_is_edited_on_the_phone() {
        assert_eq!(LayerKind::Data.mobile_choices(Storage::Database), Mobile::ALL);
        for kind in LayerKind::ALL {
            assert_eq!(kind.mobile_choices(Storage::File), [Mobile::Show, Mobile::Hide], "{kind:?}");
            if kind != LayerKind::Data {
                assert_eq!(kind.mobile_choices(Storage::Database), [Mobile::Show, Mobile::Hide], "{kind:?}");
            }
        }
        for m in Mobile::ALL {
            assert_eq!(serde_json::to_string(&m).expect("json"), format!("\"{}\"", m.id()));
        }
    }

    #[test]
    fn the_notes_say_what_each_module_draws() {
        assert_eq!(
            Module::Architecture.note(),
            "Mimari CAD: plan, kesit ve görünüş"
        );
        // The detail work is gone (docs/adr/0802 §2): no note promises it.
        for m in Module::ALL {
            assert!(!m.note().contains("detay"), "{m:?}: {}", m.note());
        }
    }
}
