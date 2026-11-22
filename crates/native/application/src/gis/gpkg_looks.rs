//! A project's data layers' looks written into the GeoPackage that keeps the project and their tables (docs/adr/9117
//! §4), so another GIS that opens a table there draws it as KentOS does. Each data layer of that file whose look is not
//! the one its table's default style (`layer_styles`) gives is written there as the table's new default style: its QML
//! (which keeps a categorized or graduated renderer as it is) with an SLD beside it, as the leading desktop GIS's
//! “Save as Default” does. The earlier styles stay in the table, no longer the default. A layer that keeps the drawing's
//! default look (no renderer, the theme's colour) and a table without geometry are passed over. Saving a project into a
//! GeoPackage (the desktop's Kaydet, the headless host's save) writes them after the project.

use std::collections::BTreeMap;
use std::path::Path;

use kentos_commands::LayersRenderer;
use kentos_domain::Document;
use kentos_geopackage::GeoPackage;
use kentos_geopackage::styles::NewStyle;

/// What a write of the looks did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Looks {
    /// The tables whose default style is now KentOS's look, in the layers' order.
    pub written: Vec<String>,
    /// What could not be written, and why.
    pub left: Vec<String>,
}

/// Whether two paths name one file: the same canonical path, or the same text when one cannot be read.
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Whether the layer's look is the one another program's default style `qml` gives it: that look read the way opening
/// reads it (docs/adr/9116 §2) and compared whole. KentOS's own QML does not come back the same way (it is written for
/// that program, not read back losslessly), so it is compared by its text.
fn unchanged(doc: &Document, layer: &str, qml: &str) -> bool {
    let Some(node) = doc.layers().get(layer) else {
        return false;
    };
    let Ok(read) = kentos_qml::read(qml) else {
        return false;
    };
    let input = LayersRenderer { layer: layer.to_owned(), renderer: Some(read.renderer), expected_revision: None };
    crate::layers_renderer::style_of(doc, &input).is_ok_and(|look| look == node.style)
}

/// Writes the looks of the project's data layers whose table is in the GeoPackage `file` as those tables' default
/// styles, in one transaction; nothing when none changed.
pub fn write(doc: &Document, file: &Path) -> Result<Looks, String> {
    let mut out = Looks::default();
    let mut rows: Vec<NewStyle> = Vec::new();
    let mut gp: Option<GeoPackage> = None;
    let mut defaults: BTreeMap<String, String> = BTreeMap::new();
    for node in doc.layers().leaves() {
        let Some(data) = node.data.as_ref() else {
            continue;
        };
        let address = data.address.strip_prefix("gpkg:").unwrap_or(&data.address);
        if data.driver != "gpkg" || !same_file(Path::new(address), file) {
            continue;
        }
        let Some(geometry) = &data.geometry else {
            continue;
        };
        if node.style.renderer.is_none() && !node.style.color.starts_with('#') {
            continue;
        }
        if gp.is_none() {
            let opened = GeoPackage::open(file, true).map_err(|e| e.0)?;
            defaults = opened.default_styles().map_err(|e| e.0)?.into_iter().collect();
            gp = Some(opened);
        }
        let (name, qml) = match crate::qml::export(doc, &node.id) {
            Ok(written) => written,
            Err(e) => {
                out.left.push(format!(
                    "“{}” katmanının görünüşü “{}” tablosunun stili olarak yazılmadı: {}",
                    node.name, data.table, e.message
                ));
                continue;
            }
        };
        // The table's default is this look already: KentOS's own QML written before (the same text), or the style
        // another program wrote, which gives this very look read.
        if defaults.get(&data.table).is_some_and(|kept| *kept == qml.xml || unchanged(doc, &node.id, kept)) {
            continue;
        }
        rows.push(NewStyle {
            table: data.table.clone(),
            geometry: geometry.column.clone(),
            name,
            qml: qml.xml,
            sld: crate::sld::export(doc, &node.id).ok().map(|(_, s)| s.xml),
        });
    }
    if let Some(gp) = gp.as_mut()
        && !rows.is_empty()
    {
        gp.add_default_styles(&rows).map_err(|e| e.0)?;
        out.written = rows.into_iter().map(|r| r.table).collect();
    }
    Ok(out)
}
