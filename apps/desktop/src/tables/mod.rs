//! Tablo (docs/adr/0184): what the desktop's table windows share, the web's
//! `app/tables.ts`. The rules are the geometry core's (`ops::table`: the
//! schedules, a file's rows, a new table's sizes); the windows are
//! `insert` (Tablo ekle) and `editor` (Tabloyu düzenle), the update
//! `update` (Tabloyu güncelle).

pub mod editor;
pub mod insert;
pub mod update;

use kentos_commands::{TableEntity, TableSource};

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
        Some(TableSource::Blocks { names, by, columns }) => {
            let what = if names.is_empty() { "bütün bloklar".to_owned() } else { names.join(", ") };
            let with = if columns.is_empty() { String::new() } else { format!("; {}", columns.join(", ")) };
            match by {
                Some(b) => format!("Blok çizelgesi: {what} ({b} değerleriyle{with})"),
                None if columns.is_empty() => format!("Blok çizelgesi: {what}"),
                None => format!("Blok çizelgesi: {what} ({})", columns.join(", ")),
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
            if let Some(kentos_commands::Entity::Insert(i)) = model.get(*s)
                && let Some(b) = model.blocks().iter().find(|b| b.id == i.block)
                && !names.contains(&b.name)
            {
                names.push(b.name.clone());
            }
        }
        let rows = model.blocks_schedule(&names, None, &[]);
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
        let source = kentos_commands::TableSource::Blocks {
            names,
            by: None,
            columns: Vec::new(),
        };
        let geometry = kentos_interaction::table::new_table(
            model,
            &cells,
            &look,
            kentos_interaction::Vec2::new(0.0, 0.0),
            Some(source),
        );
        self.place_table(geometry, "Blok çizelgesi tablosu");
    }

    /// Revizyon listesi (`arch.revisionList`, docs/adr/0721 §4): a row for each revision of the drawing's deltas, its
    /// number (`REV`), date, description and how many clouds mark it, a Blok çizelgesi of “Revizyon” by `REV` with
    /// Tarih and Açıklama placed from the cursor; the document keeps it at every step.
    pub(crate) fn place_revision_list(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        if model.block_named(kentos_commands::REVISION_BLOCK).is_none() {
            self.say(
                kentos_interaction::Level::Info,
                "Çizimde revizyon üçgeni yok: önce Revizyon işaretle ile bir revizyon işaretleyin.",
            );
            return;
        }
        let names = vec![kentos_commands::REVISION_BLOCK.to_owned()];
        let by = Some(kentos_commands::REVISION_TAG.to_owned());
        let columns: Vec<String> = kentos_commands::REVISION_DATA.iter().map(|c| (*c).to_owned()).collect();
        let rows = model.blocks_schedule(&names, by.as_deref(), &columns);
        let scale = model.settings().plot_scale;
        let look = kentos_interaction::table::Look {
            header: true,
            height: 2.5 / 1000.0 * scale,
            ..Default::default()
        };
        let cells = kentos_geometry_core::ops::table::from_rows(rows, true);
        let geometry = kentos_interaction::table::new_table(
            model,
            &cells,
            &look,
            kentos_interaction::Vec2::new(0.0, 0.0),
            Some(kentos_commands::TableSource::Blocks { names, by, columns }),
        );
        self.place_table(geometry, "Revizyon listesi");
    }

    pub(crate) fn place_table(
        &mut self,
        table: kentos_commands::EntityGeometry,
        label: &'static str,
    ) {
        self.session.run(Box::new(
            kentos_interaction::table_place::TablePlace::named(table, label),
        ));
        self.say(kentos_interaction::Level::Command, label);
        self.with_tool(|s, cx| s.activate(cx));
    }
}
