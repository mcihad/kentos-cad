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
    /// Mimari CAD: buildings, floor plans, details (a CAD project's
    /// `architecture` and `detail` works).
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
            Module::Architecture => "Mimari CAD: bina, kat planı ve detay",
            Module::Map => "Harita CAD: vaziyet, imar ve halihazır",
            Module::Gis => "Coğrafi bilgi sistemi",
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
}
