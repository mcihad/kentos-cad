//! The KentOS modules (docs/adr/0801): KentOS Mimari, KentOS Harita and
//! KentOS GIS, each a program of its own ([`Module::program`]) built from the
//! one desktop library; `kentos-cad` is all of them at once (`--modul` picks
//! one). A project belongs to a module by its type and its CAD work
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
}

impl Module {
    /// Every module, in the launcher's order.
    pub const ALL: [Module; 3] = [Module::Architecture, Module::Map, Module::Gis];

    /// The id the command line and the recent files write.
    pub const fn id(self) -> &'static str {
        match self {
            Module::Architecture => "mimari",
            Module::Map => "harita",
            Module::Gis => "cbs",
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
        }
    }

    /// The module's own program (docs/adr/0801): a binary of its own, built from
    /// the same desktop library as `kentos-cad`, that works only as this module.
    pub const fn program(self) -> &'static str {
        match self {
            Module::Architecture => "kentos-mimari",
            Module::Map => "kentos-harita",
            Module::Gis => "kentos-gis",
        }
    }

    /// One line on what it is for.
    pub const fn note(self) -> &'static str {
        match self {
            Module::Architecture => "Mimari CAD: plan, kesit ve görünüş",
            Module::Map => "Harita CAD: vaziyet, imar ve halihazır",
            Module::Gis => "Coğrafi bilgi sistemi",
        }
    }

    /// The type its projects have, as a project file writes it (`ProjectSettings.workspace`): `cad` or `gis`.
    pub const fn project_type(self) -> &'static str {
        match self {
            Module::Architecture | Module::Map => "cad",
            Module::Gis => "gis",
        }
    }

    /// The CAD works its projects are drawn for, as a project file writes them (`ProjectSettings.cadWork`), the
    /// one a new project starts with first; a CBS module has none.
    pub const fn cad_works(self) -> &'static [&'static str] {
        match self {
            Module::Architecture => &["architecture"],
            Module::Map => &["map"],
            Module::Gis => &[],
        }
    }

    /// The cloud catalog's project types its projects have (docs/adr/0802), as the server names them (`type`,
    /// `ProjectType::name`), the one a new cloud project starts with first: KentOS Mimari's a building's; KentOS
    /// Harita's every CAD type that is not a building's (general CAD, land readjustment, zoning plan, subdivision,
    /// road); KentOS GIS's CBS. No type is two modules'.
    pub const fn project_types(self) -> &'static [&'static str] {
        match self {
            Module::Architecture => &["architecture"],
            Module::Map => &[
                "cad",
                "landReadjustment",
                "zoningPlan",
                "subdivision",
                "road",
            ],
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
        assert_eq!(programs, ["kentos-mimari", "kentos-harita", "kentos-gis"]);
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
        assert_eq!(of(Some("cad"), Some("detail")), None, "two CAD works only");
        assert_eq!(of(Some("cad"), Some("map")), Some(Module::Map));
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
                "subdivision",
                "road"
            ]
        );
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
