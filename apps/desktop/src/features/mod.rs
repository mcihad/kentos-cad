//! Öznitelik tablosu, the bottom panel's Tablo tab (docs/adr/0199 §4; the
//! web's `ui/bottom/FeatureTable.ts`): a layer's objects in a table, their
//! attributes by the layer's fields. The cells are read by
//! `kentos_interaction::feature_table`; the shared core's `feature_table`
//! shows and orders them, so both platforms show the same rows.
//!
//! - Katman ▾ picks the layer (at first the first selected object's, else the
//!   active one); Ara looks in the cells as they show (`*` any run); Göster
//!   takes every object, the selected ones or the ones in the view; İfade
//!   süzgeci keeps those an expression holds for (ε opens the İfade
//!   oluşturucu on it).
//! - A header press sorts (ascending, descending, the drawing's order); a row
//!   press selects (Ctrl turns one over, Shift takes the run), the drawing's
//!   selection shows in the rows; a double click on Sıra zooms to the object.
//! - A double click on a value edits it: a value list and yes or no in a
//!   list, anything else in a field; Enter writes it (`cad.entities.set`,
//!   “Değiştir”), Tab writes and goes right, Shift+Tab left, Esc gives up; a
//!   value its field refuses stays in the cell with what was typed.
//! - Alanlar… opens the layer's fields (`layer_fields.rs`).
//!
//! The rows are worked out once for the drawing, the selection, the view
//! (while Göster takes the view's) and the query as they are (docs/adr/0120).

use std::cell::RefCell;

use iced::Task;
use kentos_contracts::{EntitiesSetProperties, Entity, PropertiesOperation, js_trim};
use kentos_domain::Slot;
use kentos_geometry_core::ops::feature_table::{Query, feature_table};
use kentos_interaction::feature_table::{FeatureTable, feature_table_of};

use crate::app::{App, Message};
use crate::document::Document;
use crate::points::{click_pick, focus_field};

#[cfg(test)]
mod tests;
mod view;

/// The tab's words (the web's `FEATURE_TEXTS`).
pub mod texts {
    pub const LAYER: &str = "Katman";
    pub const SEARCH: &str = "Ara";
    pub const SHOW: &str = "Göster";
    pub const ALL: &str = "Tümü";
    pub const SELECTED: &str = "Seçililer";
    pub const IN_VIEW: &str = "Görünümdekiler";
    pub const FILTER: &str = "İfade süzgeci";
    pub const FILTER_HINT: &str =
        "Yalnız koşulu sağlayan nesneler: Kat > 3 ve Kullanım = \u{2019}K\u{2019}. Boşsa hepsi.";
    pub const ZOOM: &str = "Seçime yakınlaş";
    pub const ZOOM_HINT: &str = "Seçili nesnelere yakınlaştırır";
    pub const FIELDS: &str = "Alanlar…";
    pub const FIELDS_HINT: &str =
        "Katmanın alanları: adları, türleri, kuralları, varsayılanları ve değer listeleri";
    pub const NO_LAYERS: &str = "Çizimde katman yok.";
    pub const NONE: &str = "Katmanda nesne yok.";
    pub const NO_MATCH: &str = "Süzgece uyan nesne yok.";
    pub const EMPTY: &str = "—";
}

/// Göster's choices (the core's `show`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Show {
    #[default]
    All,
    Selected,
    InView,
}

impl Show {
    pub const ALL: [Show; 3] = [Show::All, Show::Selected, Show::InView];

    pub fn label(self) -> &'static str {
        match self {
            Show::All => texts::ALL,
            Show::Selected => texts::SELECTED,
            Show::InView => texts::IN_VIEW,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Show::All => "all",
            Show::Selected => "selected",
            Show::InView => "inView",
        }
    }
}

/// Which way an edit ends: Enter, Tab, Shift+Tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Walk {
    Right,
    Left,
}

/// Which way Tab takes the editor (the app's key handler sends it).
pub(crate) fn walk(shift: bool) -> Walk {
    if shift { Walk::Left } else { Walk::Right }
}

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Katman ▾: a layer by id.
    Layer(String),
    Search(String),
    Show(Show),
    Filter(String),
    /// ε: the İfade oluşturucu on the filter.
    Builder,
    /// A header pressed, by the table's column (0: Sıra).
    Sort(usize),
    /// A row pressed, by its place in the order shown.
    Press(usize),
    /// A row's Sıra double-clicked: that object, zoomed to.
    Zoom(usize),
    /// Seçime yakınlaş.
    ZoomSelection,
    /// Alanlar….
    Fields,
    /// A value cell double-clicked: the row and the model's column.
    Edit(usize, usize),
    /// The editor's text typed.
    Input(String),
    /// A list editor's choice: written at once.
    Choose(String),
    /// The edit ends: written and gone a way (Tab, Shift+Tab), or stopped
    /// (a press elsewhere: a refused value is given up).
    Finish(Option<Walk>),
    /// Enter: written, and the editor stops; a refused value stays in it.
    Enter,
    /// Esc: the edit given up.
    Cancel,
}

/// The rows as worked out for a drawing, a selection, the view and a query.
#[derive(Clone, Debug, Default)]
pub(crate) struct Rows {
    /// The layer shown; none without layers.
    pub layer: Option<String>,
    /// The drawing's layers: id, path and object count, in the list's order.
    pub layers: Vec<(String, String, usize)>,
    /// The layer's objects in the drawing's order.
    pub slots: Vec<Slot>,
    pub table: FeatureTable,
    /// The rows shown, by their index in `slots`, in the order shown.
    pub shown: Vec<usize>,
    pub first_selected: Option<usize>,
    /// Why the expression filter cannot be read.
    pub filter_error: Option<String>,
    /// The sorted column in the model's columns.
    pub sorted: Option<usize>,
}

/// The open drawing (its session), its changes, the selection's version, the
/// view (while it counts) and the query's.
type Key = (u64, u64, u64, [u64; 4], u64);

/// The Tür column's sort key (no attribute is named so).
const KIND_SORT: &str = "\u{0}tür";

/// The tab's state, kept for as long as the app lives.
#[derive(Default)]
pub(crate) struct FeaturesPanel {
    pub layer: Option<String>,
    pub search: String,
    pub show: Show,
    pub filter: String,
    sort: Option<String>,
    descending: bool,
    anchor: Option<usize>,
    /// The cell edited (its object and the model's column) and its text.
    editing: Option<(Slot, usize)>,
    text: String,
    version: u64,
    cache: RefCell<Option<(Key, Rows)>>,
}

impl FeaturesPanel {
    fn changed(&mut self) {
        self.version += 1;
    }

    /// Whether a cell is being edited: Tab is its key then.
    pub(crate) fn editing(&self) -> bool {
        self.editing.is_some()
    }

    /// The choices as a new session starts them (the pictures start each scene from them).
    #[cfg(test)]
    pub(crate) fn reset(&mut self, layer: Option<String>) {
        *self = FeaturesPanel {
            layer,
            ..FeaturesPanel::default()
        };
    }
}

/// A header press: ascending, then descending, then the drawing's order.
fn next_sort(sort: &Option<String>, descending: bool, key: &str) -> (Option<String>, bool) {
    match sort {
        Some(k) if k == key && descending => (None, false),
        Some(k) if k == key => (Some(key.to_owned()), true),
        _ => (Some(key.to_owned()), false),
    }
}

impl App {
    /// The layer shown: the one chosen while it is a layer, else the first
    /// selected object's, else the active one, else the first.
    fn feature_layer(&self, doc: &Document) -> Option<String> {
        let layers = doc.model.layers();
        let is_layer = |id: &str| {
            layers
                .get(id)
                .is_some_and(|n| n.kind == kentos_contracts::LayerNodeType::Layer)
        };
        if let Some(l) = self.features.layer.as_deref().filter(|l| is_layer(l)) {
            return Some(l.to_owned());
        }
        let first = self
            .selection
            .ids()
            .iter()
            .find_map(|&s| doc.model.get(s))
            .map(|e| e.base().layer_id.clone())
            .filter(|l| is_layer(l));
        first
            .or_else(|| Some(layers.active().to_owned()).filter(|l| is_layer(l)))
            .or_else(|| layers.leaves().first().map(|n| n.id.clone()))
    }

    /// The rows for the drawing, the selection, the view and the query as
    /// they are, worked out again only when one of them changed.
    pub(crate) fn feature_rows(&self, doc: &Document) -> Rows {
        let panel = &self.features;
        let view = self.viewport.camera.visible_bounds();
        let view_key = if panel.show == Show::InView {
            [
                view.min_x.to_bits(),
                view.min_y.to_bits(),
                view.max_x.to_bits(),
                view.max_y.to_bits(),
            ]
        } else {
            [0; 4]
        };
        let key = (
            doc.session,
            doc.model.generation(),
            self.selection.version(),
            view_key,
            panel.version,
        );
        if let Some((known, rows)) = panel.cache.borrow().as_ref()
            && *known == key
        {
            return rows.clone();
        }
        let model = &doc.model;
        let layer = self.feature_layer(doc);
        let layers = model
            .layers()
            .leaves()
            .iter()
            .map(|n| {
                (
                    n.id.clone(),
                    model.layers().path(&n.id),
                    // A filtered layer counts what passes its filter (docs/adr/0211 §1).
                    self.spatial
                        .filter_counts(&n.id)
                        .map_or_else(|| model.by_layer(&n.id).count(), |(passed, _)| passed),
                )
            })
            .collect();
        let entities: Vec<&Entity> = layer
            .as_deref()
            .map(|l| {
                model
                    .by_layer(l)
                    .filter(|e| self.spatial.filter_shown(Slot(e.base().id)))
                    .collect()
            })
            .unwrap_or_default();
        let slots: Vec<Slot> = entities.iter().map(|e| Slot(e.base().id)).collect();
        let in_view: std::collections::HashSet<Slot> = if panel.show == Show::InView {
            let a = kentos_interaction::Vec2 {
                x: view.min_x,
                y: view.min_y,
            };
            let b = kentos_interaction::Vec2 {
                x: view.max_x,
                y: view.max_y,
            };
            self.spatial.in_rect(a, b, true).into_iter().collect()
        } else {
            Default::default()
        };
        let (passes, filter_error) = filter_passes(&panel.filter, &entities, model, &|| {
            self.expression_variables()
        });
        let fields = layer
            .as_deref()
            .and_then(|l| model.layers().get(l))
            .map(|n| n.fields.clone())
            .unwrap_or_default();
        let table = feature_table_of(
            &fields,
            &entities,
            &|e| self.selection.contains(Slot(e.base().id)),
            &|e| in_view.contains(&Slot(e.base().id)),
            passes.as_deref(),
        );
        let sorted = match panel.sort.as_deref() {
            None => None,
            Some(KIND_SORT) => Some(0),
            Some(k) => table
                .columns
                .iter()
                .position(|c| c.key.as_deref() == Some(k)),
        };
        let columns: Vec<_> = table.columns.iter().map(|c| c.core()).collect();
        let shown: Vec<usize> = feature_table(
            &columns,
            &table.rows,
            &Query {
                search: panel.search.clone(),
                show: panel.show.word().to_owned(),
                sort: sorted,
                descending: panel.descending,
            },
        )
        .into_iter()
        .map(|i| i as usize)
        .collect();
        let first_selected = shown
            .iter()
            .position(|&i| self.selection.contains(slots[i]));
        let rows = Rows {
            layer,
            layers,
            slots,
            table,
            shown,
            first_selected,
            filter_error,
            sorted,
        };
        *panel.cache.borrow_mut() = Some((key, rows.clone()));
        rows
    }

    /// Öznitelik tablosu (`data.featureTable`): the bottom panel on its Tablo
    /// tab, as tall as Arama's at least (the rows want room; a taller panel stays).
    pub(crate) fn open_feature_table(&mut self) {
        self.show_bottom(crate::bottom::BottomTab::Table);
        let least = crate::search::PANEL_HEIGHT;
        let bar = f64::from(kentos_ui::widget::tabs::height());
        if f64::from(self.bottom_log()) + bar < least {
            self.bottom_dragged(Some((least - bar) as f32), std::time::Instant::now());
        }
    }

    /// The objects shown now, in the order shown.
    fn shown_features(&self) -> Vec<Slot> {
        self.document
            .as_ref()
            .map(|doc| {
                let rows = self.feature_rows(doc);
                rows.shown.iter().map(|&i| rows.slots[i]).collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn features_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Layer(id) => {
                self.features.layer = Some(id);
                self.features.sort = None;
                self.features.editing = None;
                self.features.changed();
            }
            Event::Search(text) => {
                self.features.search = text;
                self.features.changed();
            }
            Event::Show(show) => {
                self.features.show = show;
                self.features.changed();
            }
            Event::Filter(text) => {
                self.features.filter = text;
                self.features.changed();
            }
            Event::Builder => return self.open_builder_for_features(),
            Event::Sort(column) => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let rows = self.feature_rows(doc);
                let p = &mut self.features;
                if column == 0 {
                    (p.sort, p.descending) = (None, false);
                } else if let Some(c) = rows.table.columns.get(column - 1) {
                    let key = c.key.clone().unwrap_or_else(|| KIND_SORT.to_owned());
                    (p.sort, p.descending) = next_sort(&p.sort, p.descending, &key);
                }
                p.changed();
            }
            Event::Press(at) => {
                let shown = self.shown_features();
                if at >= shown.len() {
                    return Task::none();
                }
                let (ctrl, shift) = (self.modifiers.control(), self.modifiers.shift());
                let ids = click_pick(
                    self.selection.ids(),
                    &shown,
                    at,
                    self.features.anchor,
                    ctrl,
                    shift,
                );
                if !shift {
                    self.features.anchor = Some(at);
                }
                self.selection.set(ids);
            }
            Event::Zoom(at) => {
                if let Some(&slot) = self.shown_features().get(at) {
                    self.selection.set([slot]);
                    self.navigating(Self::zoom_selection);
                }
            }
            Event::ZoomSelection => self.navigating(Self::zoom_selection),
            Event::Fields => {
                let layer = self
                    .document
                    .as_ref()
                    .and_then(|doc| self.feature_layer(doc));
                if let Some(layer) = layer {
                    return self.open_layer_fields(&layer);
                }
            }
            Event::Edit(at, column) => {
                let Some(&slot) = self.shown_features().get(at) else {
                    return Task::none();
                };
                return self.edit_feature(Some((slot, column)));
            }
            Event::Input(text) => self.features.text = text,
            Event::Choose(text) => {
                self.features.text = text;
                return self.finish_feature(None, true);
            }
            Event::Finish(walk) => return self.finish_feature(walk, walk.is_some()),
            Event::Enter => return self.finish_feature(None, true),
            Event::Cancel => self.features.editing = None,
        }
        Task::none()
    }

    /// The cell edited from now (none: no editor), its text the value as its
    /// field shows it (a refused one as written).
    fn edit_feature(&mut self, to: Option<(Slot, usize)>) -> Task<Message> {
        self.features.editing = to;
        let Some((slot, column)) = to else {
            return Task::none();
        };
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let rows = self.feature_rows(doc);
        let (Some(c), Some(e)) = (rows.table.columns.get(column), doc.model.get(slot)) else {
            self.features.editing = None;
            return Task::none();
        };
        let Some(key) = &c.key else {
            self.features.editing = None;
            return Task::none();
        };
        let raw = e.base().attrs.get(key).cloned().unwrap_or_default();
        self.features.text = match &c.field {
            Some(f) => match kentos_contracts::check_value(f, &raw) {
                Ok(v) if !v.is_empty() => kentos_contracts::display_value(f, &v),
                _ => raw,
            },
            None => raw,
        };
        focus_field()
    }

    /// The edit ends with the editor's text: written (blank takes the
    /// attribute away), then the editor goes `walk`'s way (none: it stops).
    /// A value refused keeps the cell open with what was typed when a key
    /// ended it (`keep`).
    fn finish_feature(&mut self, walk: Option<Walk>, keep: bool) -> Task<Message> {
        let Some((slot, column)) = self.features.editing else {
            return Task::none();
        };
        let text = std::mem::take(&mut self.features.text);
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let rows = self.feature_rows(doc);
        let last = rows.table.columns.len().saturating_sub(1);
        let (Some(key), Some(uid)) = (
            rows.table.columns.get(column).and_then(|c| c.key.clone()),
            doc.model.uid(slot),
        ) else {
            self.features.editing = None;
            return Task::none();
        };
        let value = (!js_trim(&text).is_empty()).then(|| text.clone());
        let input = EntitiesSetProperties {
            uids: vec![uid.to_string()],
            layer_id: None,
            color: None,
            line_weight: None,
            symbol: None,
            attrs: Some([(key, value)].into_iter().collect()),
            label: None,
            unlink: false,
            operation: PropertiesOperation::Attributes,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let written = write_attribute(&mut doc.model, input);
        if let Err(message) = written {
            self.warn(message);
            if keep {
                self.features.editing = Some((slot, column));
                self.features.text = text;
                return focus_field();
            }
            self.features.editing = None;
            return Task::none();
        }
        let next = walk.and_then(|w| {
            let c = match w {
                Walk::Right => column + 1,
                Walk::Left => column.checked_sub(1)?,
            };
            (1..=last).contains(&c).then_some((slot, c))
        });
        self.edit_feature(next)
    }
}

impl App {
    /// The header of the model's column `j` as the table shows it (a
    /// required field's marked).
    fn feature_header(rows: &Rows, j: usize) -> String {
        let c = &rows.table.columns[j];
        match &c.field {
            Some(f) if f.required => format!("{} *", c.label),
            _ => c.label.clone(),
        }
    }

    /// What a trace's `table` expectation reads: the count, the headers after
    /// Sıra and the rows (the cells after Sıra joined by “ | ”), as the page
    /// shows them, while the tab is open.
    pub(crate) fn features_seen(&self) -> Option<(String, Vec<String>, Vec<String>)> {
        if !self.command_expanded || self.bottom_tab != crate::bottom::BottomTab::Table {
            return None;
        }
        let doc = self.document.as_ref()?;
        let rows = self.feature_rows(doc);
        let headers = (0..rows.table.columns.len())
            .map(|j| Self::feature_header(&rows, j))
            .collect();
        let lines = rows
            .shown
            .iter()
            .map(|&i| {
                rows.table.rows[i]
                    .cells
                    .iter()
                    .map(|c| c.shown.as_str())
                    .collect::<Vec<_>>()
                    .join(" | ")
            })
            .collect();
        Some((
            format!("{} / {}", rows.shown.len(), rows.slots.len()),
            headers,
            lines,
        ))
    }

    /// The tab's controls by their words (a trace's `panel` step): Ara and
    /// İfade süzgeci, Katman ▾ and Göster ▾, a header, a row, the buttons, Esc.
    pub(crate) fn features_control(
        &self,
        control: crate::traces::Control<'_>,
    ) -> Result<Option<Message>, String> {
        use crate::traces::Control;
        let msg = |e| Some(Message::Features(e));
        let doc = self.document.as_ref().ok_or("açık çizim yok")?;
        let rows = self.feature_rows(doc);
        Ok(match control {
            Control::Fill(texts::SEARCH, t) => msg(Event::Search(t.to_owned())),
            Control::Fill(texts::FILTER, t) => msg(Event::Filter(t.to_owned())),
            Control::Pick(texts::LAYER, item) => {
                let (id, _, _) = rows
                    .layers
                    .iter()
                    .find(|(_, path, n)| format!("{path} ({n})") == item)
                    .ok_or_else(|| format!("“{item}” öğesi (Katman) yok"))?;
                msg(Event::Layer(id.clone()))
            }
            Control::Pick(texts::SHOW, item) => {
                let show = Show::ALL
                    .into_iter()
                    .find(|s| s.label() == item)
                    .ok_or_else(|| format!("“{item}” öğesi (Göster) yok"))?;
                msg(Event::Show(show))
            }
            Control::Sort("Sıra") => msg(Event::Sort(0)),
            Control::Sort(header) => {
                let j = (0..rows.table.columns.len())
                    .find(|&j| Self::feature_header(&rows, j) == header)
                    .ok_or_else(|| format!("“{header}” başlığı yok"))?;
                msg(Event::Sort(j + 1))
            }
            Control::Row(n) => {
                if n == 0 || n > rows.shown.len() {
                    return Err(format!("{n}. satır yok"));
                }
                msg(Event::Press(n - 1))
            }
            Control::Press(texts::ZOOM) => {
                (!self.selection.is_empty()).then(|| Message::Features(Event::ZoomSelection))
            }
            Control::Press(texts::FIELDS) => rows
                .layer
                .is_some()
                .then(|| Message::Features(Event::Fields)),
            Control::Key("Esc") => self
                .features
                .editing
                .map(|_| Message::Features(Event::Cancel)),
            other => return Err(format!("“Tablo” sekmesinde {other} yok")),
        })
    }

    /// A trace's cell edit: the row's value under `header` double-clicked,
    /// `text` typed over it, Enter.
    pub(crate) fn features_edit(
        &self,
        row: usize,
        header: &str,
        text: &str,
    ) -> Result<Vec<Message>, String> {
        let doc = self.document.as_ref().ok_or("açık çizim yok")?;
        let rows = self.feature_rows(doc);
        if row == 0 || row > rows.shown.len() {
            return Err(format!("{row}. satır yok"));
        }
        let j = (0..rows.table.columns.len())
            .find(|&j| {
                Self::feature_header(&rows, j) == header && rows.table.columns[j].key.is_some()
            })
            .ok_or_else(|| format!("“{header}” sütunu yok"))?;
        let field = rows.table.columns[j].field.as_ref();
        let listed = field.is_some_and(|f| {
            f.kind == kentos_contracts::LayerFieldKind::Boolean
                || f.values.as_ref().is_some_and(|v| !v.is_empty())
        });
        let write = if listed {
            // A list: the item with these words (Evet, Konut; — for none).
            let f = field.ok_or("alan yok")?;
            let code = match &f.values {
                Some(v) if !v.is_empty() => {
                    v.iter().find(|c| c.label == text).map(|c| c.code.clone())
                }
                _ => match text {
                    "Evet" => Some("true".to_owned()),
                    "Hayır" => Some("false".to_owned()),
                    _ => None,
                },
            }
            .or_else(|| (text == texts::EMPTY).then(String::new))
            .ok_or_else(|| format!("“{header}” listesinde “{text}” yok"))?;
            Message::Features(Event::Choose(code))
        } else {
            Message::Features(Event::Enter)
        };
        // A press selects the row's object first, as the double click's first press does.
        let mut out = vec![
            Message::Features(Event::Press(row - 1)),
            Message::Features(Event::Edit(row - 1, j)),
        ];
        if !listed {
            out.push(Message::Features(Event::Input(text.to_owned())));
        }
        out.push(write);
        Ok(out)
    }
}

/// Writes an attribute through `cad.entities.set` (“Değiştir”): its
/// warnings, or why it was refused.
fn write_attribute(
    model: &mut kentos_domain::Document,
    input: EntitiesSetProperties,
) -> Result<(), String> {
    let cx = &mut kentos_native_application::ExecutionContext::new(model);
    match kentos_native_application::set::execute(cx, input) {
        kentos_contracts::CommandResult::Completed { .. } => Ok(()),
        kentos_contracts::CommandResult::Failed { error }
        | kentos_contracts::CommandResult::Conflict { error }
        | kentos_contracts::CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Değer yazılamadı.".to_owned()),
    }
}

/// Which objects the expression filter keeps (none: no filter), and why it
/// cannot be read; a filter that cannot be read keeps none. The filter is
/// evaluated when asked, so it reads the project's `@` values (docs/adr/0214
/// §2.3): `variables` gives them, asked only when the filter has a `@`.
fn filter_passes(
    filter: &str,
    entities: &[&Entity],
    model: &kentos_domain::Document,
    variables: &dyn Fn() -> Vec<kentos_expression::Variable>,
) -> (Option<Vec<bool>>, Option<String>) {
    let source = js_trim(filter);
    if source.is_empty() {
        return (None, None);
    }
    let schema = kentos_expression::Schema {
        variables: if source.contains('@') {
            variables()
        } else {
            Vec::new()
        },
        ..kentos_expression::Schema::default()
    };
    match kentos_expression::compile_with(source, &schema) {
        Err(e) => (Some(vec![false; entities.len()]), Some(e.text())),
        Ok(expr) => {
            let layer_name = |id: &str| {
                model
                    .layers()
                    .get(id)
                    .map_or_else(|| id.to_owned(), |n| n.name.clone())
            };
            let mut measures = || kentos_processing::expression::measures_of(entities);
            let values = kentos_processing::expression::evaluate_all(
                &expr,
                entities,
                &layer_name,
                &mut measures,
                kentos_expression::rows::As::Bool,
            );
            (
                Some(
                    values
                        .iter()
                        .map(|v| *v == kentos_expression::Value::Bool(true))
                        .collect(),
                ),
                None,
            )
        }
    }
}
