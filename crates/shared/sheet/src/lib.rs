//! Sheet layouts, pafta düzeni (docs/sheet/design.md): everything about a
//! sheet that is computed is computed here, once, for the web (through
//! `kentos-sheet-wasm`) and the desktop (directly); the platforms only paint.
#![forbid(unsafe_code)]
// User data must never crash the core (a panic traps the WASM module).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod atlas;
pub mod bind;
pub mod cloud;
pub mod display;
pub mod error;
pub mod expr;
pub mod geodesy;
pub mod hit;
pub mod info;
pub mod json;
pub mod kinds;
pub mod kpafta;
pub mod layout;
pub mod model;
pub mod ops;
pub mod paper;
pub mod pdf;
pub mod preflight;
pub mod profile;
pub mod scene;
pub mod snap;
pub mod style;
pub mod svg;
pub mod sync;
pub mod template;
pub mod text;
pub mod units;
pub mod validate;
pub mod variants;
pub mod wmm;

pub use error::{Result, SheetError};
pub use kinds::*;
pub use model::*;
pub use style::*;
pub use units::*;

#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn an_item_reads_its_kind_and_refuses_unknown_fields() {
        let ok = r#"{"id":"a","name":"Harita","frame":{"left":0,"top":0,"width":100,"height":100},
            "kind":{"type":"map","view":{"type":"fixed","scale":1000}}}"#;
        let item: Item = serde_json::from_str(ok).unwrap();
        assert!(matches!(item.kind, ItemKind::Map(_)));
        let bad_kind_field = ok.replace(r#""scale":1000"#, r#""scale":1000,"zoom":2"#);
        assert!(serde_json::from_str::<Item>(&bad_kind_field).is_err());
        let bad_map_field = ok.replace(r#""type":"map","#, r#""type":"map","extra":1,"#);
        assert!(serde_json::from_str::<Item>(&bad_map_field).is_err());
        let bad_item_field = ok.replace(r#""id":"a","#, r#""id":"a","x":1,"#);
        assert!(serde_json::from_str::<Item>(&bad_item_field).is_err());
        let group = r#"{"id":"g","name":"G","frame":{"left":0,"top":0,"width":1,"height":1},"kind":{"type":"group"}}"#;
        assert!(serde_json::from_str::<Item>(group).is_ok());
        let group_extra = group.replace(r#"{"type":"group"}"#, r#"{"type":"group","x":1}"#);
        assert!(serde_json::from_str::<Item>(&group_extra).is_err());
        let layers_extra = ok.replace(
            r#""scale":1000}"#,
            r#""scale":1000},"layers":{"type":"all","x":1}"#,
        );
        assert!(serde_json::from_str::<Item>(&layers_extra).is_err());
        let layers_ok = ok.replace(
            r#""scale":1000}"#,
            r#""scale":1000},"layers":{"type":"all"}"#,
        );
        assert!(serde_json::from_str::<Item>(&layers_ok).is_ok());
    }
}
