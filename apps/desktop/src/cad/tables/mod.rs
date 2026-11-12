//! Tablo (docs/adr/0184): what the desktop's table windows share, the web's
//! `app/tables.ts`. The rules are the geometry core's (`ops::table`: the
//! schedules, a file's rows, a new table's sizes); the windows are
//! `insert` (Tablo ekle) and `editor` (Tabloyu düzenle), the update
//! `update` (Tabloyu güncelle).

pub mod editor;
pub mod export;
pub mod extract;
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
        // A units schedule's rows are the building program's groups (docs/adr/0795).
        Some(TableSource::Schedule { of: kentos_commands::ScheduleOf::Units, units, .. }) => {
            format!("Bölüm çizelgesi: {} satır, bina programından", units.len())
        }
        Some(TableSource::Schedule { of, members, .. }) => {
            format!("{}: {} satır, nesnelerinden", of.label(), members.len())
        }
        // Emsal ve TAKS tablosu (docs/adr/8030).
        Some(TableSource::Zoning {}) => {
            "Emsal ve TAKS tablosu: imar durumundan ve mahallerden".to_owned()
        }
        // Otopark and Sığınak hesabı (docs/adr/8036).
        Some(TableSource::Parking {}) => "Otopark hesabı: bina programından ve otopark mahallerinden".to_owned(),
        Some(TableSource::Shelter {}) => "Sığınak hesabı: bina programından ve sığınak mahallerinden".to_owned(),
        // Veri çıkarımı (docs/adr/1030 §6): its recipe, the drawings it reads besides this one.
        Some(TableSource::Extraction(x)) => {
            let others = x.recipe.drawings.len();
            let from = if others == 0 { "bu çizimden".to_owned() } else { format!("bu ve {others} başka çizimden") };
            format!("Veri çıkarımı “{}”: {} satır, {from}", x.recipe.name, x.keys.len())
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

    /// Emsal ve TAKS tablosu (`arch.zoningTable`, docs/adr/8030): the parcel's zoning against the drawing as PAİY 57/3
    /// asks the approved project to show it, placed from the cursor; the document keeps it at every step and when
    /// Proje ayarları change. Without a zoning, where it is typed, and Proje ayarları open on Mimari.
    pub(crate) fn place_zoning_table(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        if model
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.zoning)
            .is_none()
        {
            self.say(
                kentos_interaction::Level::Info,
                "Projede imar durumu yok: Proje ayarları › Mimari › İmar durumu'na parsel alanını, emsali ve TAKS'ı \
                 yazın; tablo onlarla yazılır.",
            );
            let _ = self.project_command("arch.settings");
            return;
        }
        let written = model.zoning_table();
        let scale = model.settings().plot_scale;
        let look = kentos_interaction::table::Look {
            header: true,
            height: 2.5 / 1000.0 * scale,
            ..Default::default()
        };
        let cells = kentos_geometry_core::ops::table::from_rows(written.rows, true);
        let geometry = kentos_interaction::table::new_table(
            model,
            &cells,
            &look,
            kentos_interaction::Vec2::new(0.0, 0.0),
            Some(kentos_commands::TableSource::Zoning {}),
        );
        self.place_table(geometry, "Emsal ve TAKS tablosu");
    }

    /// Model kalite kontrol formu (`arch.qualityForm`, docs/adr/8047 §10): EK-9's model quality control form, its answers
    /// the IFC model's now (`kentos_ifc::quality`, through the application), placed from the cursor. Ruhsat takımı
    /// writes its answers again and gives it a sheet of its own (EK-2 FR/MKF).
    pub(crate) fn place_quality_form(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        let rows = kentos_native_application::arch_ifc::form_rows(model);
        let look = kentos_interaction::table::Look {
            header: true,
            height: 2.5 / 1000.0 * model.settings().plot_scale,
            ..Default::default()
        };
        let cells = kentos_geometry_core::ops::table::from_rows(rows, true);
        let geometry = kentos_interaction::table::new_table(
            model,
            &cells,
            &look,
            kentos_interaction::Vec2::new(0.0, 0.0),
            None,
        );
        self.place_table(geometry, "Model kalite kontrol formu");
    }

    /// Otopark hesabı and Sığınak hesabı (docs/adr/8036): a live table from the building program's units, placed at the
    /// cursor; a project without units is told where they are given.
    pub(crate) fn place_provision_table(&mut self, source: kentos_commands::TableSource) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        let (written, label) = match source {
            kentos_commands::TableSource::Shelter {} => (model.shelter_table(), "Sığınak hesabı"),
            _ => (model.parking_table(), "Otopark hesabı"),
        };
        let has_units = model
            .program()
            .units
            .iter()
            .any(|u| u.kind == kentos_commands::UnitKind::Unit);
        let scale = model.settings().plot_scale;
        let look = kentos_interaction::table::Look {
            header: true,
            height: 2.5 / 1000.0 * scale,
            ..Default::default()
        };
        let cells = kentos_geometry_core::ops::table::from_rows(written.rows, true);
        let geometry = kentos_interaction::table::new_table(
            model,
            &cells,
            &look,
            kentos_interaction::Vec2::new(0.0, 0.0),
            Some(source),
        );
        if !has_units {
            self.say(
                kentos_interaction::Level::Info,
                "Bina programında bağımsız bölüm yok: tablo bölümlerden hesaplanır (Mimari › Proje ▾ › Bina programı; \
                 mahallere bölüm verin).",
            );
        }
        self.place_table(geometry, label);
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
