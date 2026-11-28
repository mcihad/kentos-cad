//! Yeniden bağla… for data layers (docs/adr/9155): a data layer's file moved (another computer, another folder) is
//! pointed at its new place once, and every data layer of the project on the same file follows it there. A table in the
//! file picked is taken only when it is the layer's: the same key column, the same geometry column, type and SRID,
//! and every field the layer has with the same kind; a same-named table of another package that differs is refused
//! with each difference said, nothing bound. A database layer is pointed elsewhere from its connection, not here.

use std::path::Path;

use kentos_commands::{CommandResult, DataLayer, LayerDataOperation, LayerNode, LayersData};
use kentos_native_application::ExecutionContext;

use crate::app::App;

/// What the table at the new place lacks to be the layer's, each said; none when it is the layer's.
pub(crate) fn mismatch(
    layer: &LayerNode,
    data: &DataLayer,
    found: &kentos_native_application::data_describe::Described,
) -> Vec<String> {
    let mut out = Vec::new();
    if found.data.key != data.key {
        out.push(format!(
            "anahtar sütunu “{}” değil “{}”",
            data.key, found.data.key
        ));
    }
    match (&data.geometry, &found.data.geometry) {
        (Some(a), Some(b)) => {
            if a.column != b.column {
                out.push(format!(
                    "geometri sütunu “{}” değil “{}”",
                    a.column, b.column
                ));
            }
            if a.type_name != b.type_name {
                out.push(format!(
                    "geometri türü {} değil {}",
                    a.type_name, b.type_name
                ));
            }
            if a.srid != b.srid {
                let s = |v: Option<u32>| v.map_or_else(|| "yok".to_owned(), |n| n.to_string());
                out.push(format!("SRID {} değil {}", s(a.srid), s(b.srid)));
            }
        }
        (Some(_), None) => out.push("tablonun geometrisi yok".to_owned()),
        (None, Some(_)) => out.push("tablo geometrili, katman geometrisiz".to_owned()),
        (None, None) => {}
    }
    for f in &layer.fields {
        match found.fields.iter().find(|g| g.name == f.name) {
            None => out.push(format!("“{}” alanı yok", f.name)),
            Some(g) if g.kind != f.kind => out.push(format!(
                "“{}” alanı {} değil {}",
                f.name,
                f.kind.label(),
                g.kind.label()
            )),
            Some(_) => {}
        }
    }
    out
}

impl App {
    /// The data layers of `layer`'s file pointed at `path` (see the module): the layers relinked, or why none was.
    pub fn relink_data_layers(&mut self, layer: &str, path: &Path) -> Result<Vec<String>, String> {
        let doc = self.document.as_ref().ok_or("Açık çizim yok.")?;
        let node = doc
            .model
            .layers()
            .get(layer)
            .cloned()
            .ok_or("Katman yok.")?;
        let data = node.data.clone().ok_or("Katman bir veri katmanı değil.")?;
        if data.driver == "postgis" {
            return Err("Veritabanı katmanı bağlantısından yeniden yönlendirilir; Yeniden bağla dosyalar içindir.".into());
        }
        let address = path.display().to_string();
        // Every data layer on the same file, the one asked for first.
        let mut same: Vec<LayerNode> = vec![node.clone()];
        same.extend(
            doc.model
                .layers()
                .leaves()
                .into_iter()
                .filter(|n| {
                    n.id != node.id && n.data.as_ref().is_some_and(|d| d.address == data.address)
                })
                .cloned(),
        );
        // Each table checked first: nothing is bound unless all are the layers'.
        let mut planned: Vec<(LayerNode, DataLayer)> = Vec::new();
        for n in &same {
            let d = n.data.clone().unwrap_or_else(|| data.clone());
            let found = crate::data_layers::describe(&address, &d.table).map_err(|why| {
                format!(
                    "“{}”: {} dosyasında “{}” tablosu okunamadı: {why}",
                    n.name,
                    path.display(),
                    d.table
                )
            })?;
            let wrong = mismatch(n, &d, &found);
            if !wrong.is_empty() {
                return Err(format!(
                    "{} dosyasındaki “{}” tablosu “{}” katmanının tablosu değil: {}. Hiçbir katman bağlanmadı.",
                    path.display(),
                    d.table,
                    n.name,
                    wrong.join("; ")
                ));
            }
            let mut next = d.clone();
            next.address = address.clone();
            // The next save writes the new place's relative path.
            next.relative = None;
            planned.push((n.clone(), next));
        }
        let mut done = Vec::new();
        for (n, next) in planned {
            let Some(doc) = self.document.as_mut() else {
                break;
            };
            let input = LayersData {
                operation: LayerDataOperation::Update,
                layer: Some(n.id.clone()),
                name: None,
                parent: None,
                index: None,
                data: Some(next),
                fields: None,
                expected_revision: None,
            };
            match kentos_native_application::layers_data::execute(
                &mut ExecutionContext::new(&mut doc.model),
                input,
            ) {
                CommandResult::Completed { .. } => done.push(n.id.clone()),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    return Err(format!(
                        "“{}” yeniden bağlanmadı: {}",
                        n.name, error.message
                    ));
                }
                _ => {}
            }
        }
        // The file's old address is not asked again.
        kentos_data::registry().forget(&data.address);
        Ok(done)
    }
}

#[cfg(test)]
#[path = "data_relink_tests.rs"]
mod data_relink_tests;
