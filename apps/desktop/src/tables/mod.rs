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
        Some(TableSource::File {
            name, sheet, path, ..
        }) => {
            let base = match sheet {
                None => name.clone(),
                Some(sheet) => format!("{name} › {sheet}"),
            };
            if path.is_some() { format!("{base} (bağlı)") } else { base }
        }
        Some(TableSource::Blocks { names, by }) => {
            let what = if names.is_empty() { "bütün bloklar".to_owned() } else { names.join(", ") };
            match by {
                Some(b) => format!("Blok çizelgesi: {what} ({b} değerleriyle)"),
                None => format!("Blok çizelgesi: {what}"),
            }
        }
        Some(TableSource::Schedule { of, members, .. }) => {
            format!("{}: {} satır, nesnelerinden", of.label(), members.len())
        }
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
    /// Blok çizelgesi tablosu (`table.blockSchedule`, docs/adr/0327 §4): the blocks of the selected inserts (every
    /// block placed with none selected) counted in a table placed from the cursor; the document keeps it at every
    /// step.
    pub(crate) fn place_block_schedule(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        let mut names: Vec<String> = Vec::new();
        for s in self.selection.ids() {
            if let Some(kentos_contracts::Entity::Insert(i)) = model.get(*s)
                && let Some(b) = model.blocks().iter().find(|b| b.id == i.block)
                && !names.contains(&b.name)
            {
                names.push(b.name.clone());
            }
        }
        let rows = model.blocks_schedule(&names, None);
        if rows.len() <= 2 && names.is_empty() {
            self.say(kentos_interaction::Level::Info, "Çizimde blok yerleştirmesi yok: çizelgelenecek bir şey yok.");
            return;
        }
        let scale = model.settings().plot_scale;
        let look = kentos_interaction::table::Look {
            header: true,
            height: 2.5 / 1000.0 * scale,
            ..Default::default()
        };
        let cells = kentos_geometry_core::ops::table::from_rows(rows, true);
        let source = kentos_contracts::TableSource::Blocks { names, by: None };
        let geometry = kentos_interaction::table::new_table(
            model,
            &cells,
            &look,
            kentos_interaction::Vec2::new(0.0, 0.0),
            Some(source),
        );
        self.place_table(geometry, "Blok çizelgesi tablosu");
    }

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
