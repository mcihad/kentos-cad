//! Satır ekle's layer (the CBS review's 11th item, docs/adr/9320). The layer a new row is written to is found when the
//! row opens and is shown above it, not learnt from a refusal after every value has been typed.
//!
//! - A CAD project (Harita, Yol) keeps its points in the project: a drawing layer that takes points (of the point
//!   kind or of any kind), not locked.
//! - A CBS project keeps them in tables (CLAUDE.md §25 rule 5): a writable data layer of points, not locked. Its edit
//!   session may still be closed: the row says so and offers Düzenlemeyi başlat.
//! - The row's layer is the one chosen in its list while that is still one of these. Else it is the active layer when
//!   it is open for writing, else the first open one, else the active, else the first. With none, the row offers Nokta
//!   katmanı oluştur; what was typed stays in the row.
//! - A data layer's table keeps the row's name and code in its columns of those names (`ad`, `adı`, `adi`, `name`;
//!   `kod`, `code`; any case), and the elevation in the geometry's Z.

use iced::Task;
use kentos_commands::{LayerField, LayerGeometry, Workspace};
use kentos_domain::Document;
use kentos_native_application::layer_kind::{self, LayerKind};

use super::Target;
use super::edit::EditColumn;
use crate::app::{App, Message};

/// A layer a point row may be written to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Choice {
    pub id: String,
    pub name: String,
    /// A data layer: its rows go to its table when the edits are saved.
    pub data: bool,
    /// A data layer whose edit session is closed: Düzenlemeyi başlat opens it.
    pub closed: bool,
}

/// The column names a table's name and code are found under, folded.
const NAME_COLUMNS: [&str; 4] = ["ad", "adı", "adi", "name"];
const CODE_COLUMNS: [&str; 2] = ["kod", "code"];

fn cbs(doc: &Document) -> bool {
    doc.settings().project_type() == Some(Workspace::Gis)
}

/// The layers a point row may be written to, in the layer list's order.
pub(crate) fn choices(doc: &Document) -> Vec<Choice> {
    let layers = doc.layers();
    let cbs = cbs(doc);
    layers
        .leaves()
        .into_iter()
        .filter(|n| !layers.is_locked(&n.id))
        .filter(|n| match layer_kind::of(doc, n) {
            // A table of points (or of any geometry) whose rows are not texts, written where the source allows it.
            LayerKind::Data => {
                cbs && n.data.as_ref().is_some_and(|d| {
                    !d.read_only
                        && d.text.is_none()
                        && d.geometry.as_ref().is_some_and(|g| {
                            crate::layer_types::data_geometry(&g.type_name).is_none_or(|g| g.admits("point"))
                        })
                })
            }
            // A CBS project's drawing layers keep no objects of their own (docs/adr/9196 §2).
            LayerKind::Drawing => !cbs && n.geometry.is_none_or(|g: LayerGeometry| g.admits("point")),
            _ => false,
        })
        .map(|n| Choice {
            id: n.id.clone(),
            name: n.name.clone(),
            data: n.data.is_some(),
            closed: n.data.is_some() && !doc.editing(&n.id),
        })
        .collect()
}

/// The row's layer: `chosen` while it is one of the [`choices`], else the active layer when it is open for writing,
/// else the first open one, else the active, else the first; none when the project has no such layer.
pub(crate) fn resolve(doc: &Document, chosen: Option<&str>) -> Option<Choice> {
    let all = choices(doc);
    let find = |id: &str| all.iter().find(|c| c.id == id).cloned();
    let active = find(doc.layers().active());
    chosen
        .and_then(find)
        .or_else(|| active.clone().filter(|c| !c.closed))
        .or_else(|| all.iter().find(|c| !c.closed).cloned())
        .or(active)
        .or_else(|| all.first().cloned())
}

/// What a refusal at Enter adds: nothing typed is lost.
pub(crate) const KEPT: &str = " Yazdıklarınız satırda duruyor.";

/// Why the row is not written to `choice` now, with the way out; none when it is.
pub(crate) fn refusal(doc: &Document, choice: Option<&Choice>) -> Option<String> {
    let prefix = super::edit::PREFIX;
    match choice {
        None if cbs(doc) => Some(format!(
            "{prefix}Satırın yazılacağı nokta katmanı yok. Tablonun üstündeki Nokta katmanı oluştur ile bir nokta tablosu \
             yapın ya da Harita ▸ Veri ekle ile bir nokta tablosu bağlayın."
        )),
        None => Some(format!(
            "{prefix}Satırın yazılacağı nokta katmanı yok. Tablonun üstündeki Nokta katmanı oluştur ile bir nokta \
             katmanı ekleyin."
        )),
        Some(c) if c.closed => Some(format!(
            "{prefix}“{}” düzenlemeye kapalı. Tablonun üstündeki Düzenlemeyi başlat ile oturumunu açın.",
            c.name
        )),
        Some(_) => None,
    }
}

/// A table's columns for the row's name and code: the first field whose folded name is one of theirs.
pub(crate) fn columns(fields: &[LayerField]) -> (Option<String>, Option<String>) {
    let find = |names: &[&str]| {
        fields.iter().find(|f| names.contains(&kentos_commands::fold(&f.name).as_str())).map(|f| f.name.clone())
    };
    (find(&NAME_COLUMNS), find(&CODE_COLUMNS))
}

impl App {
    /// The draft row's layer now: the one chosen while it takes points, else found as [`resolve`] says.
    pub(crate) fn draft_target(&self) -> Option<Choice> {
        let doc = &self.document.as_ref()?.model;
        let chosen = self.gis.points.draft.as_ref()?.layer.clone();
        resolve(doc, chosen.as_deref())
    }

    /// What is typed in the row's open cell kept in the row, as a press beside it keeps it: the actions above the
    /// table take the row as typed.
    pub(crate) fn keep_typed(&mut self) {
        if matches!(self.gis.points.editing, Some((Target::Draft, _))) {
            let _ = self.finish_cell(None);
        }
    }

    /// The keyboard back in the draft row, at its first empty cell of Ad, Y and X, else at Kod: Enter writes the row
    /// from any of its cells.
    pub(crate) fn focus_draft(&mut self) -> Task<Message> {
        let Some(d) = self.gis.points.draft.as_ref() else {
            return Task::none();
        };
        let empty = |v: &str| v.trim().is_empty();
        let col = [(EditColumn::Name, &d.name), (EditColumn::East, &d.east), (EditColumn::North, &d.north)]
            .into_iter()
            .find_map(|(col, v)| empty(v).then_some(col))
            .unwrap_or(EditColumn::Code);
        self.edit_cell(Some((Target::Draft, col)))
    }

    /// Nokta katmanı oluştur: in a CBS project the window for a table of points with `ad`, `kod` and Z, which
    /// becomes the row's layer with its edit session open when it is made; in a CAD project a point layer at once,
    /// the row's layer. What was typed stays in the row.
    pub(crate) fn create_point_layer_for_row(&mut self) -> Task<Message> {
        self.keep_typed();
        let Some(doc) = &self.document else {
            return Task::none();
        };
        if cbs(&doc.model) {
            self.open_point_table_for_rows();
            return Task::none();
        }
        self.new_typed_layer(LayerGeometry::Point);
        let made = self.document.as_ref().map(|d| d.model.layers().active().to_owned());
        if let (Some(d), Some(id)) = (self.gis.points.draft.as_mut(), made) {
            d.layer = Some(id);
        }
        self.focus_draft()
    }

    /// The table made for the row by Nokta katmanı oluştur: opened for editing and made the row's layer.
    pub(crate) fn point_table_made(&mut self, id: &str) -> Task<Message> {
        let _ = self.data_edits_event(crate::data_edits::Event::Start(id.to_owned()));
        if let Some(d) = self.gis.points.draft.as_mut() {
            d.layer = Some(id.to_owned());
        }
        self.focus_draft()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::LayerFieldKind;

    #[test]
    fn a_tables_name_and_code_are_its_columns_of_those_names_in_any_case() {
        let fields = |names: &[&str]| -> Vec<LayerField> {
            names.iter().map(|n| LayerField::new(*n, LayerFieldKind::Text)).collect()
        };
        assert_eq!(columns(&fields(&["fid_eski", "AD", "Kod"])), (Some("AD".into()), Some("Kod".into())));
        assert_eq!(columns(&fields(&["ADI", "CODE"])), (Some("ADI".into()), Some("CODE".into())));
        assert_eq!(columns(&fields(&["Name", "kodu"])), (Some("Name".into()), None));
        assert_eq!(columns(&fields(&["adres", "durum"])), (None, None), "a column only like a name is not one");
    }
}
