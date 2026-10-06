//! Tablo (docs/adr/0184): what the desktop's table windows share, the web's
//! `app/tables.ts`. The rules are the geometry core's (`ops::table`: the
//! schedules, a file's rows, a new table's sizes); the windows are
//! `insert` (Tablo ekle) and `editor` (Tabloyu düzenle), the update
//! `update` (Tabloyu güncelle).

pub mod editor;
pub mod insert;
pub mod update;

use kentos_contracts::{TableEntity, TableSource};

/// Where a table's rows came from, as Öznitelikler says it.
pub fn source_words(t: &TableEntity) -> String {
    match &t.source {
        None => "Elle yazıldı".to_owned(),
        Some(TableSource::File { name, sheet: None }) => name.clone(),
        Some(TableSource::File {
            name,
            sheet: Some(sheet),
        }) => format!("{name} › {sheet}"),
        Some(s) => {
            let what = match s {
                TableSource::Coordinates { .. } => "Koordinat çizelgesi",
                TableSource::Areas { .. } => "Alan çizelgesi",
                _ => "Öznitelik tablosu",
            };
            format!("{what} ({} nesne)", s.objects().len())
        }
    }
}
