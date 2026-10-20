//! Katman işlemleri (docs/adr/0217; the web's `builtin/layers/`): objects
//! split into layers by their values, layers merged with a field map,
//! objects extracted into a layer, duplicates deleted, ordered copies. What
//! they share is here: a copy, an object's value of some fields, the copy
//! or move choice.

pub mod dedupe;
pub mod extract;
pub mod merge;
pub mod order_by;
pub mod split;

use std::collections::BTreeMap;

use kentos_commands::Entity;
use kentos_native_application::geometry::unlinked;

use crate::text::js_trim;
use crate::types::{EnumOption, ParamDef, ParamKind};

use super::queries::attr;

/// A copy of an object under a layer (docs/adr/0217 §2): every field but its
/// id; a linked text's copy is a text of its own and an associative hatch's
/// copy follows nothing (docs/adr/0175 §4, 0186 §6). `attrs`: the copy's own.
pub fn copy_of(e: &Entity, layer: &str, attrs: Option<BTreeMap<String, String>>) -> Entity {
    let mut c = unlinked(e.clone());
    let base = c.base_mut();
    base.id = 0;
    layer.clone_into(&mut base.layer_id);
    if let Some(attrs) = attrs {
        base.attrs = attrs;
    }
    c
}

/// An object's value of these fields (docs/adr/0217 §2): each trimmed, “(boş)” for none, joined by “ – ”.
pub fn fields_value(e: &Entity, fields: &[String]) -> String {
    fields
        .iter()
        .map(|f| match js_trim(attr(e, f).unwrap_or("")) {
            "" => "(boş)",
            v => v,
        })
        .collect::<Vec<_>>()
        .join(" – ")
}

/// Kopyala or Taşı.
pub fn mode_param() -> ParamDef {
    ParamDef::new(
        "mode",
        "İşlem",
        ParamKind::Choice {
            options: vec![
                EnumOption::new("copy", "Kopyala")
                    .hint("Nesneler yerinde kalır, kopyaları yazılır"),
                EnumOption::new("move", "Taşı").hint("Nesneler katman değiştirir"),
            ],
        },
    )
    .default_value(serde_json::json!("copy"))
}

/// The result layer: a new one of this name and colour by default; `above`:
/// the features parameter whose first object's layer a new one goes right above.
pub fn output_param(new_name: &str, color: &str, above: Option<&str>) -> ParamDef {
    ParamDef::new(
        "output",
        "Sonuç",
        ParamKind::Layer {
            new_layer_style: crate::types::NewLayerStyle {
                color: Some(color.to_owned()),
                ..crate::types::NewLayerStyle::default()
            },
            above: above.map(str::to_owned),
            below: None,
        },
    )
    .default_value(serde_json::json!({ "newName": new_name }))
}

/// The objects a tool takes, as Katman tools name them.
pub fn input_param(scopes: Vec<crate::types::ScopeKind>, description: &str) -> ParamDef {
    ParamDef::new(
        "input",
        "Nesneler",
        ParamKind::Features {
            kinds: None,
            scopes: Some(scopes),
            writes: true,
        },
    )
    .describe(description)
}
