//! The system library: everything that ships with KentOS (the basic symbols
//! and MPYY's 695 symbols and 81 drawings, docs/STYLE.md §8). Its source is the
//! web's TypeScript (`apps/web/src/style/system`); the desktop embeds the
//! same items as a `.kstil` file written from it
//! (`apps/web/scripts/style/system-library.test.ts`, which also fails when
//! the file falls behind the TypeScript). Read once, on first use.

use std::sync::OnceLock;

use serde_json::Value;

use crate::library::{Item, StyleLibrary};

/// The file as the web writes it.
const FILE: &str = include_str!("../assets/system-library.kstil");

struct System {
    items: Vec<Item>,
    categories: Vec<Value>,
}

fn read() -> System {
    // The file ships with the program and a test reads it: it is well formed.
    let value: Value = serde_json::from_str(FILE).unwrap_or(Value::Null);
    let list = |k: &str| {
        value
            .get(k)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    System {
        items: list("items")
            .into_iter()
            .filter_map(Item::from_value)
            .collect(),
        categories: list("categories"),
    }
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
        assert!(symbols >= 695, "symbols: {symbols}");
        assert!(assets >= 81, "assets: {assets}");
        assert!(lib.symbol("temel.cizgi.surekli").is_some());
    }
}
