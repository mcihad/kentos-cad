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

impl crate::app::App {
    /// A table a tool made, placed from the cursor by Tablo ekle's tool under
    /// the tool's name (Köşelere koordinat yaz's Çizelge, docs/adr/0185 §1).
    pub(crate) fn place_table(
        &mut self,
        table: kentos_contracts::EntityGeometry,
        label: &'static str,
    ) {
        self.session.run(Box::new(
            kentos_interaction::table_place::TablePlace::named(table, label),
        ));
        self.say(kentos_interaction::Level::Command, label);
        self.with_tool(|s, cx| s.activate(cx));
    }
}
