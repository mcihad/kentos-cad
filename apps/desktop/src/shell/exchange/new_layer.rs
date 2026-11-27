//! A new layer an import window makes (docs/adr/9329): Nokta listesi and GNSS (GPX, NMEA) offer a name no layer of the
//! drawing has, and a name a layer has is refused with the way to that layer. Yeni katman always makes a new layer;
//! the points go onto a layer of the drawing only when Hedef katman names it. Before, a typed name a layer had took
//! that layer silently, while the window said Yeni katman.

use kentos_commands::LayerStyle;
use kentos_domain::Document;

use super::apply::{LayerTarget, layer_named};

/// What the name field says under it while the name is free: `from` names the list the drawing's layers are in, in
/// the ablative (“Hedef katman'dan”).
pub(super) fn help(from: &str) -> String {
    format!("Yeni bir katman açılır. Var olan bir katmana eklemek için {from} onu seçin.")
}

/// A new layer named `name` with `style`, or why there is none: no name, or a name a layer of the drawing has, said
/// with the list that layer is picked in (`from`, in the ablative).
pub(super) fn target(doc: &Document, name: &str, style: LayerStyle, from: &str) -> Result<LayerTarget, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Yeni katmanın adını yazın.".to_owned());
    }
    if let Some(same) = layer_named(doc, name) {
        return Err(format!(
            "“{}” adında bir katman var. Onun üstüne eklemek için {from} seçin; yoksa başka bir ad yazın.",
            same.name
        ));
    }
    Ok(LayerTarget::New {
        frozen: None,
        plot: None,
        name: name.to_owned(),
        style: Box::new(style),
        visible: true,
        locked: false,
        fields: Vec::new(),
    })
}

/// A file's format as a layer's origin names it: its extension, a word of small letters (`gpx`, `nmea`, `ncn`); none
/// for a file without one.
pub(super) fn format_word(file: &str) -> Option<String> {
    let ext = std::path::Path::new(file).extension()?.to_str()?.to_lowercase();
    (!ext.is_empty() && ext.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())).then_some(ext)
}
