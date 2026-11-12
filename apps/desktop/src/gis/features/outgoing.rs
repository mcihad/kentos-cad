//! What Öznitelik tablosu shows and what it lets out of itself (docs/adr/0546 §1, §2): Sütunlar ▾ hides a column from
//! the table (kept per layer, for the session), Kopyala and Dışa aktar ▾ write the rows as the table shows them (its
//! visible columns, filter, sort and Göster) and leave out the fields a form marks `redact`, and İstatistik ve grafik ▾
//! opens the two tabs on the table's own rows. The rules are the interaction crate's (`outgoing_columns`,
//! `copy_text`, `workbook_sheet_of`); this file wires them to the tab.

use std::collections::BTreeSet;

use iced::Task;
use kentos_commands::{WorkbookFormat, WorkbookSheet};
use kentos_interaction::Level;
use kentos_interaction::feature_table::{Outgoing, copy_text, outgoing_columns, workbook_sheet_of};

use super::{Rows, Show};
use crate::app::{App, Message};

/// The words of what the table lets out.
pub mod words {
    pub const COLUMNS: &str = "Sütunlar";
    pub const COLUMNS_HEADER: &str = "Tabloda gösterilen sütunlar";
    pub const COLUMNS_HINT: &str = "Tabloda hangi sütunların görüneceğini seçer. Gizlenen sütun kopyada ve dışa aktarılan dosyada da yoktur.";
    pub const SHOW_ALL: &str = "Hepsini göster";
    pub const COPY: &str = "Kopyala";
    pub const COPY_HINT: &str = "Tablonun gösterilen satırlarını, gösterilen sütunlarıyla (alan adları başlıkta) sekmeyle ayrılmış olarak panoya kopyalar; Yapıştır'la geri yapıştırılabilir. Formda gizli işaretli alan kopyalanmaz.";
    pub const ANALYSE: &str = "İncele";
    pub const ANALYSE_HINT: &str = "Tablonun süzgeciyle kalan satırların istatistiğini ya da grafiğini açar: Ara, Göster ve İfade süzgeci neyi gösteriyorsa onu.";
    pub const STATISTICS: &str = "İstatistik (tablonun süzgeciyle)";
    pub const CHART: &str = "Grafik (tablonun süzgeciyle)";
    pub const NO_ROWS: &str = "Kopyalanacak satır yok.";

    pub fn copied(rows: usize) -> String {
        format!("{rows} satır panoya kopyalandı.")
    }

    /// The sentence that says fields were left out of a copy or a file.
    pub fn withheld(headers: &[String]) -> String {
        format!(
            "Formda gizli işaretli {} alan dışarı yazılmadı: {}.",
            headers.len(),
            headers.join(", ")
        )
    }
}

/// A change of the columns shown.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnsEvent {
    /// A column hidden or shown again, by its attribute key.
    Toggle(String),
    /// Every column shown again.
    ShowAll,
}

/// Where İncele ▾ goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Analyse {
    Statistics,
    Chart,
}

impl App {
    /// The keys of the columns the reader hid on `layer`.
    pub(crate) fn hidden_columns(&self, layer: Option<&str>) -> BTreeSet<String> {
        layer
            .and_then(|l| self.features.hidden.get(l))
            .cloned()
            .unwrap_or_default()
    }

    /// The layer fields' keys and names the project's forms keep out of anything that leaves the table.
    pub(crate) fn redacted_fields(&self) -> Vec<String> {
        self.document
            .as_ref()
            .map(|d| kentos_native_application::python_call::redacted_names(&d.model))
            .unwrap_or_default()
    }

    /// What the table lets out of itself now.
    pub(crate) fn outgoing(&self, rows: &Rows) -> Outgoing {
        outgoing_columns(
            &rows.table,
            &self.hidden_columns(rows.layer.as_deref()),
            &self.redacted_fields(),
        )
    }

    pub(crate) fn features_columns_event(&mut self, event: ColumnsEvent) {
        let Some(layer) = self
            .document
            .as_ref()
            .and_then(|doc| self.feature_layer(doc))
        else {
            return;
        };
        let hidden = self.features.hidden.entry(layer.clone()).or_default();
        match event {
            ColumnsEvent::Toggle(key) => {
                if !hidden.remove(&key) {
                    hidden.insert(key);
                }
            }
            ColumnsEvent::ShowAll => hidden.clear(),
        }
        if hidden.is_empty() {
            self.features.hidden.remove(&layer);
        }
        // A cell edited in a column that went is not edited any more.
        self.features.editing = None;
        self.features.changed();
    }

    /// What Kopyala writes: the rows shown as tab-separated lines, how many rows, and the headers of the fields a
    /// form keeps out; none when the table shows no row.
    pub(crate) fn features_copy_text(&self) -> Option<(String, usize, Vec<String>)> {
        let doc = self.document.as_ref()?;
        let rows = self.feature_rows(doc);
        if rows.shown.is_empty() {
            return None;
        }
        let out = self.outgoing(&rows);
        Some((
            copy_text(&rows.table, &rows.shown, &out.columns),
            rows.shown.len(),
            out.withheld,
        ))
    }

    /// What Dışa aktar ▾ writes: the sheet and its name, and the headers of the fields a form keeps out; none when
    /// the table shows no row.
    pub(crate) fn features_sheet(&self) -> Option<(WorkbookSheet, Vec<String>)> {
        let doc = self.document.as_ref()?;
        let rows = self.feature_rows(doc);
        if rows.shown.is_empty() {
            return None;
        }
        let name = rows
            .layer
            .as_deref()
            .and_then(|id| doc.model.layers().get(id))
            .map_or_else(|| "Tablo".to_owned(), |n| n.name.clone());
        let out = self.outgoing(&rows);
        Some((
            workbook_sheet_of(&rows.table, &rows.shown, &name, &out.columns),
            out.withheld,
        ))
    }

    /// Kopyala: the rows shown on the clipboard.
    pub(crate) fn features_copy(&mut self) -> Task<Message> {
        let Some((text, count, withheld)) = self.features_copy_text() else {
            self.say(Level::Info, words::NO_ROWS);
            return Task::none();
        };
        let mut said = words::copied(count);
        if !withheld.is_empty() {
            said = format!("{said} {}", words::withheld(&withheld));
        }
        self.say(Level::Success, said);
        iced::clipboard::write(text)
    }

    /// Dışa aktar ▾: the rows shown written in `format`.
    pub(crate) fn features_export(&mut self, format: WorkbookFormat) -> Task<Message> {
        let Some((sheet, withheld)) = self.features_sheet() else {
            self.say(Level::Info, "Dışa aktarılacak satır yok.");
            return Task::none();
        };
        if !withheld.is_empty() {
            self.say(Level::Info, words::withheld(&withheld));
        }
        let name = sheet.name.clone();
        crate::table_write::save(vec![sheet], format, name)
    }

    /// İncele ▾: İstatistik or Grafik for the table's layer, over the rows the table shows.
    pub(crate) fn features_analyse(&mut self, to: Analyse) {
        let Some(layer) = self
            .document
            .as_ref()
            .and_then(|doc| self.feature_layer(doc))
        else {
            return;
        };
        // The tabs read the table's layer over the rows the table shows.
        self.charts.show = Show::Table;
        match to {
            Analyse::Statistics => self.open_statistics(Some(layer)),
            Analyse::Chart => self.open_chart(Some(layer), None, None),
        }
    }

    /// The rows the table shows, as the places of its layer's objects (`slots`), for a tab that charts the same
    /// layer; none when the table is on another layer.
    pub(crate) fn table_rows_of(&self, layer: &str) -> Option<Vec<kentos_domain::Slot>> {
        let doc = self.document.as_ref()?;
        let rows = self.feature_rows(doc);
        (rows.layer.as_deref() == Some(layer))
            .then(|| rows.shown.iter().map(|&i| rows.slots[i]).collect())
    }

    /// What Göster ▾ = Tablonun süzgeciyle depends on: the table's choices, and the selection and the view while
    /// the table's own Göster takes them.
    pub(crate) fn table_rows_stamp(&self) -> (u64, [u64; 4]) {
        let p = &self.features;
        let selection = if p.show == Show::Selected {
            self.selection.version()
        } else {
            0
        };
        let view = if p.show == Show::InView {
            let v = self.viewport.camera.visible_bounds();
            [
                v.min_x.to_bits(),
                v.min_y.to_bits(),
                v.max_x.to_bits(),
                v.max_y.to_bits(),
            ]
        } else {
            [0; 4]
        };
        (
            p.version.wrapping_mul(0x9E37_79B9).wrapping_add(selection),
            view,
        )
    }
}
