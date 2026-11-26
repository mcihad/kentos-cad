//! The system library: everything that ships with KentOS. Three files:
//! - the basic symbols and MPYY's 695 symbols and 81 drawings (docs/STYLE.md §8), whose source is the web's
//!   TypeScript (`apps/web/src/style/system`), written as a `.kstil` file by
//!   `apps/web/scripts/style/system-library.test.ts` (which also fails when the file falls behind the TypeScript);
//! - the general library: Genel, point, line and area symbols for every kind of map by theme (docs/adr/9118), and
//!   Kent, a city's trees, plants, furniture, signs, markings, paving and utility details (docs/adr/9119), written by
//!   `scripts/style/general_library.py` (`--check` fails when the file falls behind the script);
//! - the BÖHHBÜY library: the 277 signs of the large-scale map regulation's Ek-3, sign by sign (docs/adr/9120),
//!   written by `scripts/style/bohhbuy_library.py` (`--check` as above).
//!
//! Read once, on first use.

use std::sync::OnceLock;

use serde_json::Value;

use crate::library::{Item, StyleLibrary};

/// The basic and MPYY items, as the web writes them.
const FILE: &str = include_str!("../assets/system-library.kstil");
/// The general library's items (`genel.*`, `kent.*`).
const GENERAL: &str = include_str!("../assets/general-library.kstil");
/// The BÖHHBÜY library's items (`bohhbuy.*`).
const BOHHBUY: &str = include_str!("../assets/bohhbuy-library.kstil");

struct System {
    items: Vec<Item>,
    categories: Vec<Value>,
}

fn read() -> System {
    let mut system = System { items: Vec::new(), categories: Vec::new() };
    for file in [FILE, GENERAL, BOHHBUY] {
        // The files ship with the program and a test reads them: they are well formed.
        let value: Value = serde_json::from_str(file).unwrap_or(Value::Null);
        let list = |k: &str| value.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
        system.items.extend(list("items").into_iter().filter_map(Item::from_value));
        system.categories.extend(list("categories"));
    }
    system
}

fn system() -> &'static System {
    static SYSTEM: OnceLock<System> = OnceLock::new();
    SYSTEM.get_or_init(read)
}

/// A library holding the system items (and no user or project items yet).
pub fn library() -> StyleLibrary {
    let s = system();
    StyleLibrary::with_system(s.items.clone(), s.categories.clone())
}

#[cfg(test)]
mod tests {
    use crate::library::{ItemKind, Source};

    #[test]
    fn ships_the_web_s_system_library() {
        let lib = super::library();
        let items = lib.items(Some(Source::System));
        let symbols = items
            .iter()
            .filter(|(i, _)| i.kind() == ItemKind::Symbol)
            .count();
        let assets = items.len() - symbols;
        assert!(symbols >= 695 + 340 + 277, "symbols: {symbols}");
        assert!(assets >= 81 + 120 + 30, "assets: {assets}");
        assert!(lib.symbol("temel.cizgi.surekli").is_some());
        assert!(lib.symbol("genel.nokta.ulasim.otobus-duragi").is_some());
        assert!(lib.symbol("kent.nokta.agac.yaprakli").is_some());
        // Harita's Nokta layer draws its points with it (docs/adr/9315).
        assert!(lib.symbol("genel.nokta.olcme.balastro").is_some());
        assert!(lib.symbol("bohhbuy.0201").is_some());
    }
}
