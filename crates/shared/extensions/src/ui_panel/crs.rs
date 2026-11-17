//! A panel's coordinate system picker (docs/adr/8057 §45): its value `EPSG:<code>` of the systems KentOS knows, the
//! registry the web and the desktop read (`fixtures/crs/v1/registry.json`), local (0) left out: a picker asks for a
//! system, not for none.

use std::sync::OnceLock;

use serde_json::{Map, Value};

/// The codes of KentOS's coordinate systems, in the registry's order; the local system (0) left out.
pub fn codes() -> &'static [u32] {
    static CODES: OnceLock<Vec<u32>> = OnceLock::new();
    CODES.get_or_init(|| {
        let registry: Value =
            serde_json::from_str(include_str!("../../../../../fixtures/crs/v1/registry.json"))
                .unwrap_or_default();
        registry["systems"]
            .as_array()
            .map(|s| {
                s.iter()
                    .filter_map(|s| s["srid"].as_u64().and_then(|c| u32::try_from(c).ok()))
                    .filter(|c| *c != 0)
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// The code of a picker's value (`EPSG:5256` → 5256) when it is one of KentOS's systems; none otherwise.
pub fn code_of(value: &str) -> Option<u32> {
    let code: u32 = value.trim().strip_prefix("EPSG:")?.parse().ok()?;
    codes().contains(&code).then_some(code)
}

/// Picker `c`'s problems, each said at its place under `at` by `add`: a value that is not one of KentOS's systems.
pub(super) fn check(c: &Map<String, Value>, at: &str, add: &mut impl FnMut(String, String)) {
    match c.get("value") {
        None => {}
        Some(Value::String(v)) if code_of(v).is_some() => {}
        Some(_) => add(
            format!("{at}/value"),
            "value KentOS'un tanıdığı bir koordinat sistemi olmalı, ör. \"EPSG:5256\" (TUREF / TM36); \
             yerel sistem (EPSG:0) seçilmez."
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The registry's systems without the local one; a value is one of them, written `EPSG:<code>`.
    #[test]
    fn a_pickers_value_is_one_of_kentos_systems() {
        assert!(codes().contains(&5256) && codes().contains(&4326) && !codes().contains(&0));
        assert_eq!(code_of("EPSG:5256"), Some(5256));
        assert_eq!(code_of("EPSG:0"), None, "the local system is no choice");
        assert_eq!(code_of("EPSG:9999"), None);
        assert_eq!(code_of("5256"), None, "written EPSG:<code>");
        let said = |c: serde_json::Value| {
            let mut out: Vec<String> = Vec::new();
            check(c.as_object().unwrap(), "/body/0", &mut |at, _| out.push(at));
            out
        };
        let picker = json!({ "type": "crsPicker", "id": "crs", "label": "Sistem" });
        assert!(
            said(picker.clone()).is_empty(),
            "no value: the drawing's own"
        );
        let mut wrong = picker;
        wrong["value"] = json!("EPSG:9999");
        assert_eq!(said(wrong), ["/body/0/value"]);
    }
}
