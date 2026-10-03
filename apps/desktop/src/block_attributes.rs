//! Blok öznitelikleri (the web's `ui/blocks/BlockAttributesDialog.ts`,
//! docs/adr/0144 §7): the Bloklar panel's window for a block's attribute
//! definitions, a table as the Hesap windows' (calc/grid.rs): tag, prompt,
//! default, text height, turn and place (east Y and north X of the base
//! point, in the block's own units), one row each (attribute_table.rs). A
//! row's place is typed, or shown on an insert of the block (the selected
//! one, or its only one) and taken back into the definition, as the base
//! point is: the window goes for the point and comes back as it was left.
//! Every change asks `cad.blocks.edit` (`attributes`) and shows its refusal
//! under the table; Kaydet waits until it passes and writes the whole list,
//! one undo step “Blok değiştir”.

use iced::widget::tooltip::Position;
use iced::widget::{Column, button, text};
use iced::{Element, Task};
use kentos_contracts::{BlockId, Vec2};
use kentos_domain::Slot;
use kentos_interaction::blocks as actions;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::style;
use kentos_ui::widget::{Dialog as Frame, Tip, overlay, tip};

use crate::app::{App, Dialog, Message};
use crate::attribute_table::{self as table, COLUMNS, Extent, Row};
use crate::calc::grid::{self, Owner, Table};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const ATTRIBUTES_TITLE: &str = "Blok öznitelikleri";
const ADD: &str = "Öznitelik ekle";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";

/// The window: the block, its rows, and what the command answers for them.
#[derive(Debug, Clone)]
pub struct Window {
    block: BlockId,
    name: String,
    base: Vec2,
    rows: Vec<Row>,
    /// The block's drawing east and north of its base point, for a first row's place.
    extent: Option<Extent>,
    /// A first row's text height: a Yazı's, 2.5 mm on paper.
    height: f64,
    /// The insert a place is shown on, or why none.
    via: Result<Slot, String>,
    /// The command's answer for the rows as they are: its refusal's words.
    answer: Result<(), String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Cell(usize, usize, String),
    Paste(usize, usize, String),
    Pasted(usize, usize, Option<String>),
    Submit(usize, usize),
    Arrow(bool, iced::widget::Id),
    AddRow,
    RemoveRow(usize),
    /// The row's place, shown on the drawing.
    Pick(usize),
    Save,
    Close,
}

fn msg(event: Event) -> Message {
    Message::BlockAttributes(event)
}

/// The table's cells and messages (calc/grid.rs).
#[derive(Clone, Copy, Debug)]
struct Sheet;

impl Owner for Sheet {
    fn cell_id(self, row: usize, col: usize) -> iced::widget::Id {
        iced::widget::Id::from(format!("block-attributes-{row}-{col}"))
    }
    fn cell(self, row: usize, col: usize, text: String) -> Message {
        msg(Event::Cell(row, col, text))
    }
    fn paste(self, row: usize, col: usize, text: String) -> Message {
        msg(Event::Paste(row, col, text))
    }
    fn submit(self, row: usize, col: usize) -> Message {
        msg(Event::Submit(row, col))
    }
    fn remove_row(self, row: usize) -> Message {
        msg(Event::RemoveRow(row))
    }
    fn add_row(self) -> Message {
        msg(Event::AddRow)
    }
    fn add_label(self) -> &'static str {
        ADD
    }
}

impl Table for Window {
    fn columns(&self) -> usize {
        COLUMNS.len()
    }
    fn rows(&self) -> usize {
        self.rows.len()
    }
    fn get(&self, row: usize, col: usize) -> &str {
        &self.rows[row].cells[col]
    }
    fn set(&mut self, row: usize, col: usize, text: String) {
        if let Some(r) = self.rows.get_mut(row) {
            r.cells[col] = text;
        }
    }
    fn insert_after(&mut self, row: usize) {
        let new = table::new_row(&self.rows[..=row], self.base, self.extent, self.height);
        self.rows.insert(row + 1, new);
    }
    fn can_insert_first(&self) -> bool {
        true
    }
    fn insert_first(&mut self) {
        let new = table::new_row(&[], self.base, self.extent, self.height);
        self.rows.insert(0, new);
    }
    fn can_remove(&self, _row: usize) -> bool {
        true
    }
    fn remove(&mut self, row: usize) {
        if row < self.rows.len() {
            self.rows.remove(row);
        }
    }
}

impl Window {
    fn definitions(&self) -> Vec<kentos_contracts::AttributeDefinition> {
        self.rows
            .iter()
            .map(|r| table::definition_of(r, self.base))
            .collect()
    }
}

/// ↑ (`up`) or ↓ with the window open: the focused field is found, then the
/// table moves the keyboard (`grid::arrow`).
pub(crate) fn arrow(up: bool) -> Task<Message> {
    use iced::advanced::widget::{operate, operation::focusable::find_focused};
    operate(find_focused()).map(move |from| msg(Event::Arrow(up, from)))
}

impl App {
    /// Opens the window for the block `id` (the Bloklar panel's menu, `block.attributes`).
    pub(crate) fn open_block_attributes(&mut self, id: BlockId) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        let Some(block) = model.block(id) else {
            return;
        };
        self.spatial.sync(model);
        let outlines = self.spatial.store().insert_outlines(
            &id.to_text(),
            kentos_interaction::Vec2::new(0.0, 0.0),
            1.0,
            0.0,
            false,
        );
        self.block_attributes = Some(Window {
            block: id,
            name: block.name.clone(),
            base: block.base,
            rows: block
                .attributes
                .iter()
                .map(|a| table::row_of(a, block.base))
                .collect(),
            extent: table::outline_extent(&outlines),
            height: 2.5 / 1000.0 * model.settings().plot_scale,
            via: actions::attributes_insert(model, &self.selection, id),
            answer: Ok(()),
        });
        self.dialog = Some(Dialog::BlockAttributes);
        self.block_attributes_check();
    }

    /// `block.attributes`: the window for the block the selected inserts place.
    pub(crate) fn open_selected_block_attributes(&mut self) {
        let block = self
            .document
            .as_ref()
            .and_then(|d| actions::selected_block(&d.model, &self.selection));
        match block {
            Some(id) => self.open_block_attributes(id),
            None => self.warn(
                "Önce bir bloğun yerleştirmesini seçin (Bloklar panelinde bloğun menüsünden de açılır).",
            ),
        }
    }

    /// What the command answers for the rows as they are now.
    fn block_attributes_check(&mut self) {
        let (Some(w), Some(doc)) = (self.block_attributes.as_mut(), self.document.as_mut()) else {
            return;
        };
        w.answer = actions::check_attributes(&mut doc.model, w.block, w.definitions());
    }

    pub(crate) fn block_attributes_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.block_attributes.as_mut() else {
            return Task::none();
        };
        let task = match event {
            Event::Cell(row, col, text) => {
                w.set(row, col, text);
                Task::none()
            }
            Event::Paste(row, col, contents) => {
                // The field's own paste stands until the clipboard says it held a table.
                w.set(row, col, contents);
                iced::clipboard::read().map(move |t| msg(Event::Pasted(row, col, t)))
            }
            Event::Pasted(row, col, raw) => {
                match raw.and_then(|raw| grid::paste(w, row, col, &raw)) {
                    Some(at) => iced::widget::operation::focus(Sheet.cell_id(at, col)),
                    None => Task::none(),
                }
            }
            Event::Submit(row, col) => grid::submit(w, Sheet, row, col),
            Event::Arrow(up, from) => grid::arrow(w, Sheet, &from, up),
            Event::AddRow => grid::add(w, Sheet),
            Event::RemoveRow(row) => {
                w.remove(row);
                Task::none()
            }
            Event::Pick(row) => {
                self.block_attributes_pick(row);
                return Task::none();
            }
            Event::Save => {
                self.block_attributes_save();
                return Task::none();
            }
            Event::Close => {
                self.dialog = None;
                self.block_attributes = None;
                return Task::none();
            }
        };
        self.block_attributes_check();
        task
    }

    /// Sahneden seç: the window goes for a point on the insert, and comes back (blocks_panel.rs).
    fn block_attributes_pick(&mut self, row: usize) {
        let Some(w) = &self.block_attributes else {
            return;
        };
        let Ok(insert) = w.via.clone() else {
            return;
        };
        let name = w.name.clone();
        self.dialog = None;
        self.blocks_pick_place(insert, row, name);
    }

    /// The point shown for a row's place, in the definition's coordinates
    /// (none when the pick was left); the window comes back either way.
    pub(crate) fn block_attributes_picked(&mut self, row: usize, local: Option<Vec2>) {
        let Some(w) = self.block_attributes.as_mut() else {
            return;
        };
        if let (Some(p), Some(r)) = (local, w.rows.get(row)) {
            w.rows[row] = table::placed(r, p, w.base);
        }
        self.dialog = Some(Dialog::BlockAttributes);
        self.block_attributes_check();
    }

    /// Kaydet: the whole list through `cad.blocks.edit`, when the command lets it.
    fn block_attributes_save(&mut self) {
        let (Some(w), Some(doc)) = (self.block_attributes.as_mut(), self.document.as_mut()) else {
            return;
        };
        if w.answer.is_err() {
            return;
        }
        let (said, done) = actions::set_attributes(&mut doc.model, w.block, w.definitions());
        if !done {
            w.answer = Err(said
                .last()
                .map(|(_, line)| line.clone())
                .unwrap_or_default());
            return;
        }
        self.block_attributes = None;
        self.dialog = None;
        for (level, line) in said {
            self.say(level, line);
        }
    }

    /// Blok öznitelikleri (the web's `openBlockAttributesDialog`).
    pub(crate) fn block_attributes_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.block_attributes else {
            return text("").into();
        };
        let summary = words::summary(vec![
            words::text_line(
                Kind::Info,
                format!(
                    "“{}” bloğu: yerleştirmeler özniteliklerin kendi değerlerini, değeri olmayanlar varsayılanı yazar.",
                    w.name
                ),
            ),
            words::text_line(
                Kind::Info,
                "Yer taban noktasına göredir: Y doğuya, X kuzeye, bloğun kendi ölçüsünde."
                    .to_owned(),
            ),
        ]);
        let pick_tip = match &w.via {
            Ok(_) => format!("Yerini “{}” bloğunun yerleştirmesinde gösterin.", w.name),
            Err(why) => why.clone(),
        };
        let can_pick = w.via.is_ok();
        let sheet = grid::view_with(
            Sheet,
            table::columns(&self.format()),
            w,
            |_, _| String::new(),
            1,
            move |r| {
                let pick = button(icon(Icon::Target).size(14.0))
                    .on_press_maybe(can_pick.then(|| msg(Event::Pick(r))))
                    .padding(6)
                    .style(style::button::ghost);
                vec![tip(
                    pick,
                    Tip::new(format!("{}. satırın yerini seç", r + 1)).body(pick_tip.clone()),
                    Position::Top,
                )]
            },
        );
        let mut body = Column::new().spacing(12).push(summary).push(sheet);
        if let Err(why) = &w.answer {
            body = body.push(words::text_line(Kind::Error, why.clone()));
        }
        overlay::modal(
            Frame::new(ATTRIBUTES_TITLE)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Close))))
                .action(words::primary(
                    SAVE,
                    w.answer.is_ok().then(|| msg(Event::Save)),
                ))
                .width(860.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): the
    /// cells as the web names them (`1. satır Etiket`), the rows' buttons
    /// (`1. satırın yerini seç`, `1. satırı sil`) and the window's.
    pub(crate) fn block_attributes_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.block_attributes else {
            return Err("Blok öznitelikleri penceresi açık değil".to_owned());
        };
        /// `N. <rest>`: the row N−1 and the rest.
        fn numbered(words: &str) -> Option<(usize, &str)> {
            let (n, rest) = words.split_once(". ")?;
            let n: usize = n.parse().ok()?;
            Some((n.checked_sub(1)?, rest))
        }
        let missing = |what: String| format!("“{ATTRIBUTES_TITLE}” penceresinde {what} yok");
        Ok(match control {
            Control::Fill(label, t) => {
                let found = numbered(label).and_then(|(r, rest)| {
                    let col = COLUMNS
                        .iter()
                        .position(|c| rest.strip_prefix("satır ") == Some(c.label))?;
                    (r < w.rows.len()).then_some((r, col))
                });
                let (r, col) = found.ok_or_else(|| missing(control.to_string()))?;
                Some(msg(Event::Cell(r, col, t.to_owned())))
            }
            Control::Press(ADD) => Some(msg(Event::AddRow)),
            Control::Press(SAVE) => w.answer.is_ok().then(|| msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            Control::Press(words) => {
                let (r, rest) = numbered(words)
                    .filter(|(r, _)| *r < w.rows.len())
                    .ok_or_else(|| missing(control.to_string()))?;
                match rest {
                    "satırın yerini seç" => w.via.is_ok().then(|| msg(Event::Pick(r))),
                    "satırı sil" => Some(msg(Event::RemoveRow(r))),
                    _ => return Err(missing(control.to_string())),
                }
            }
            other => return Err(missing(other.to_string())),
        })
    }
}

#[cfg(test)]
mod tests {
    use kentos_contracts::{BlockId, DocumentSnapshotV1};
    use kentos_domain::Slot;

    use super::{ATTRIBUTES_TITLE, Event};
    use crate::app::{App, Dialog, Message};
    use crate::document::Document;
    use crate::traces::Control;

    /// The traces' blocks drawing (fixtures/interaction/v1/blocks.kcad): Rögar
    /// has no attributes; its inserts 4, 5 and 6 hold NO = R-1, R-2, R-3.
    fn blocks() -> (App, BlockId) {
        let (mut app, _) = App::boot(None);
        let snapshot = DocumentSnapshotV1::from_json(include_str!(
            "../../../fixtures/interaction/v1/blocks.kcad"
        ))
        .expect("the drawing reads");
        let doc = Document::new(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let model = &app.document.as_ref().expect("open").model;
        let id = model
            .blocks()
            .iter()
            .find(|b| b.name == "Rögar")
            .expect("Rögar")
            .id;
        (app, id)
    }

    fn send(app: &mut App, event: Event) {
        let _ = app.update(Message::BlockAttributes(event));
    }

    fn answer(app: &App) -> Option<Result<(), String>> {
        app.block_attributes.as_ref().map(|w| w.answer.clone())
    }

    fn attributes(app: &App, id: BlockId) -> Vec<kentos_contracts::AttributeDefinition> {
        let model = &app.document.as_ref().expect("open").model;
        model.block(id).expect("the block").attributes.clone()
    }

    #[test]
    fn a_row_added_filled_and_saved_is_the_blocks_attribute_in_one_step() {
        let (mut app, id) = blocks();
        // The command asks for a selected insert of one block.
        assert!(!app.available("block.attributes"));
        app.selection.set([Slot(5)]);
        assert!(app.available("block.attributes"));
        let _ = app.update(Message::Run("block.attributes"));
        assert_eq!(app.dialog, Some(Dialog::BlockAttributes));
        assert_eq!(app.dialog_title().as_deref(), Some(ATTRIBUTES_TITLE));
        // Öznitelik ekle: the first row right of the drawing, a Yazı's height (2.5 mm at 1:500).
        send(&mut app, Event::AddRow);
        let w = app.block_attributes.as_ref().expect("open");
        assert_eq!(w.rows.len(), 1);
        assert_eq!(w.rows[0].cells[super::table::HEIGHT], "1.25");
        // An empty tag is refused under the table; Kaydet does nothing.
        assert!(matches!(answer(&app), Some(Err(why)) if why.contains("etiket")));
        assert!(matches!(
            app.dialog_control(Control::Press("Kaydet")),
            Ok(None)
        ));
        // The cells by the web's names.
        for (label, text) in [
            ("1. satır Etiket", "NO"),
            ("1. satır Yükseklik", "0,5"),
            ("1. satır Y", "0.9"),
            ("1. satır X", "0.15"),
        ] {
            let m = app
                .dialog_control(Control::Fill(label, text))
                .expect("a cell")
                .expect("a message");
            let _ = app.update(m);
        }
        assert_eq!(answer(&app), Some(Ok(())));
        let before = app.document.as_ref().map(|d| d.model.revision());
        let _ = app.update(
            app.dialog_control(Control::Press("Kaydet"))
                .expect("Kaydet")
                .expect("on"),
        );
        assert_eq!(app.dialog, None);
        assert!(app.block_attributes.is_none());
        let a = attributes(&app, id);
        assert_eq!(a.len(), 1);
        assert_eq!(
            (a[0].tag.as_str(), a[0].height, a[0].p.x, a[0].p.y),
            ("NO", 0.5, 0.9, 0.15)
        );
        assert!(app.document.as_ref().map(|d| d.model.revision()) != before);
        let label = app.document.as_mut().and_then(|d| d.model.undo());
        assert_eq!(label.as_deref(), Some("Blok değiştir"));
        assert!(attributes(&app, id).is_empty());
    }

    #[test]
    fn a_place_shown_on_the_insert_comes_back_into_the_row_and_vazgec_writes_nothing() {
        let (mut app, id) = blocks();
        app.selection.set([Slot(6)]);
        app.open_block_attributes(id);
        send(&mut app, Event::AddRow);
        // Sahneden seç: the window goes, the point comes back on the insert at (487030, 4420008.5).
        send(&mut app, Event::Pick(0));
        assert_eq!(app.dialog, None);
        assert!(app.blocks_picked(Some(kentos_interaction::Vec2::new(487031.25, 4420008.0))));
        assert_eq!(app.dialog, Some(Dialog::BlockAttributes));
        let w = app.block_attributes.as_ref().expect("back");
        let cells = &w.rows[0].cells;
        assert_eq!(
            (
                cells[super::table::EAST].as_str(),
                cells[super::table::NORTH].as_str()
            ),
            ("1.25", "-0.5")
        );
        // A pick left with Esc brings the window back as it was.
        send(&mut app, Event::Pick(0));
        assert!(app.blocks_picked(None));
        assert_eq!(app.dialog, Some(Dialog::BlockAttributes));
        // Vazgeç: nothing written.
        let _ = app.update(
            app.dialog_control(Control::Press("Vazgeç"))
                .expect("Vazgeç")
                .expect("on"),
        );
        assert_eq!(app.dialog, None);
        assert!(attributes(&app, id).is_empty());
    }
}
