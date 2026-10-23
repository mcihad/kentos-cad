//! Drawing-set editor: draft fields survive row switches; saving uses typed core operations.
use iced::{Element, Fill};
use iced::widget::{button, column, row, scrollable};
use kentos_sheet::{SheetBook, Item, Owner, RectUm, ItemKind, TableSource, TableColumn};
use kentos_sheet::drawing_set::{DrawingInfo, DrawingIndex, SetDrawingInfo, SetDrawingSet};
use kentos_sheet::ops::{Op, AddItems};
use kentos_ui::{
    label, style,
    widget::{Dialog, InputField, Switch},
};
use crate::{Designer, Message, designer::Dialog as Window};
#[derive(Clone, Debug)]
struct Entry {
    id: String,
    name: String,
    info: DrawingInfo,
    subset: String,
}
#[derive(Clone, Debug)]
pub(crate) struct SetDialog {
    entries: Vec<Entry>,
    active: usize,
    subset: String,
    views: bool,
    error: Option<String>,
}
#[derive(Clone, Copy, Debug)]
pub enum Field {
    Number,
    Title,
    Revision,
    Status,
    Subset,
}
#[derive(Clone, Debug)]
pub enum SetMessage {
    Open,
    Select(usize),
    Edit(Field, String),
    Filter(String),
    Views(bool),
    Save,
    InsertIndex,
}
fn msg(m: SetMessage) -> Message {
    Message::DrawingSet(m)
}
fn subset(value: &str) -> Vec<String> {
    if value.trim().is_empty() {
        Vec::new()
    } else {
        value.split('/').map(|s| s.trim().to_owned()).collect()
    }
}
impl SetDialog {
    fn operations(&self, book: &SheetBook) -> Vec<Op> {
        let entries = self
            .entries
            .iter()
            .filter_map(|e| {
                let old = book.sheet(&e.id)?;
                let mut info = e.info.clone();
                info.subset = subset(&e.subset);
                (old.drawing_info.clone().unwrap_or_default() != info).then(|| SetDrawingInfo {
                    id: e.id.clone(),
                    info: Some(info),
                })
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            Vec::new()
        } else {
            vec![Op::SetDrawingSet(SetDrawingSet { entries })]
        }
    }
    fn source(&self) -> DrawingIndex {
        DrawingIndex {
            subset: subset(&self.subset),
            views: self.views,
        }
    }
}
impl Designer {
    pub(crate) fn drawing_set_update(&mut self, m: SetMessage) {
        if matches!(m, SetMessage::Open) {
            let entries: Vec<_> = self
                .book
                .sheets
                .iter()
                .map(|s| Entry {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    subset: s
                        .drawing_info
                        .as_ref()
                        .map_or(String::new(), |i| i.subset.join(" / ")),
                    info: s.drawing_info.clone().unwrap_or_default(),
                })
                .collect();
            if entries.is_empty() {
                return;
            }
            let active = self
                .open
                .as_ref()
                .and_then(|id| self.book.sheet_index(id))
                .unwrap_or(0);
            self.dialog = Some(Window::DrawingSet(Box::new(SetDialog {
                entries,
                active,
                subset: String::new(),
                views: false,
                error: None,
            })));
            return;
        }
        if !matches!(self.dialog, Some(Window::DrawingSet(_))) {
            return;
        }
        let Some(Window::DrawingSet(mut d)) = self.dialog.take() else {
            return;
        };
        d.error = None;
        match m {
            SetMessage::Select(i) if i < d.entries.len() => d.active = i,
            SetMessage::Edit(field, value) => {
                if let Some(e) = d.entries.get_mut(d.active) {
                    match field {
                        Field::Number => e.info.number = value,
                        Field::Title => e.info.title = value,
                        Field::Revision => e.info.revision = value,
                        Field::Status => e.info.status = value,
                        Field::Subset => e.subset = value,
                    }
                }
            }
            SetMessage::Filter(v) => d.subset = v,
            SetMessage::Views(v) => d.views = v,
            SetMessage::Save | SetMessage::InsertIndex => {
                let mut ops = d.operations(&self.book);
                if matches!(m, SetMessage::InsertIndex)
                    && let Some(sheet) = self.open_sheet()
                    && let Some(ItemKind::Table(mut table)) =
                        kentos_sheet::kinds::default_kind("table")
                {
                    let owner = Owner::sheet(&sheet.id);
                    let rect = sheet.page.margin_rect();
                    table.source = TableSource::DrawingSet(d.source());
                    table.title = if d.views {
                        "Görünüm dizini"
                    } else {
                        "Pafta dizini"
                    }
                    .into();
                    table.columns = [
                        "Pafta no",
                        "Başlık",
                        "Revizyon",
                        "Durum",
                        "Alt küme",
                        "Görünüm",
                    ]
                    .into_iter()
                    .map(|heading| TableColumn {
                        heading: heading.into(),
                        value: String::new(),
                        width: 0,
                        align: Default::default(),
                        decimals: None,
                    })
                    .collect();
                    let id = self.new_id("dizin");
                    let title = table.title.clone();
                    let item = Item::new(
                        &id,
                        &title,
                        RectUm {
                            left: rect.left,
                            top: rect.top,
                            width: rect.width,
                            height: rect.height.min(80_000),
                        },
                        ItemKind::Table(table),
                    );
                    ops.push(Op::AddItems(AddItems {
                        to: owner,
                        items: vec![item],
                        index: None,
                    }));
                }
                if ops.is_empty() || self.commit(ops) {
                    return;
                }
                d.error = self.error.clone();
            }
            _ => {}
        }
        self.dialog = Some(Window::DrawingSet(d));
    }
    pub(crate) fn drawing_set_view<'a>(&'a self, d: &'a SetDialog) -> Element<'a, Message> {
        let mut list = column![].spacing(4);
        for (i, e) in d.entries.iter().enumerate() {
            let title = format!(
                "{} · {}",
                if e.info.number.is_empty() {
                    "Numarasız"
                } else {
                    &e.info.number
                },
                e.name
            );
            list = list.push(
                button(label::body(title))
                    .width(Fill)
                    .style(if i == d.active {
                        style::button::primary
                    } else {
                        style::button::secondary
                    })
                    .on_press(msg(SetMessage::Select(i))),
            );
        }
        let e = &d.entries[d.active];
        let field = |title: &'static str, value: &'a str, which: Field| -> Element<'a, Message> {
            row![
                label::caption(title).width(132),
                InputField::new("", value).on_input(move |v| msg(SetMessage::Edit(which, v)))
            ]
            .spacing(8)
            .align_y(iced::Center)
            .into()
        };
        let form = column![
            label::body(&e.name),
            field("Pafta numarası", &e.info.number, Field::Number),
            field("Pafta başlığı", &e.info.title, Field::Title),
            field("Revizyon", &e.info.revision, Field::Revision),
            field("Durum", &e.info.status, Field::Status),
            field("Alt küme", &e.subset, Field::Subset),
            label::caption("Alt küme örneği: Mimari / Kat planları"),
        ]
        .spacing(8);
        let mut body = column![
            label::caption(
                "Pafta bilgileri antetlere ve dizinlere yansır; boş başlık sekme adını izler."
            ),
            row![
                scrollable(
                    iced::widget::container(list).padding(iced::Padding::new(0.).right(12.))
                )
                .width(240)
                .height(260),
                form.width(Fill)
            ]
            .spacing(16),
            row![
                label::caption("Dizin alt kümesi").width(132),
                InputField::new("Boş: bütün set", &d.subset)
                    .on_input(|v| msg(SetMessage::Filter(v)))
            ]
            .spacing(8)
            .align_y(iced::Center),
            Switch::new(d.views, |v| msg(SetMessage::Views(v)))
                .label("Pafta yerine görünüm dizini"),
            label::caption(
                "Dizin, pafta ve görünüm değişiklikleriyle yenilenir. Düzeyler / ile ayrılır."
            ),
        ]
        .spacing(12);
        if let Some(error) = &d.error {
            body = body.push(label::body(error).style(style::text::danger));
        }
        Dialog::new("Pafta seti")
            .scroll(body)
            .width(760.)
            .max_height(640.)
            .aside(
                button(label::body("Vazgeç"))
                    .on_press(Message::CloseDialog)
                    .style(style::button::secondary),
            )
            .action(
                button(label::body("Öndeki paftaya dizin ekle"))
                    .on_press(msg(SetMessage::InsertIndex))
                    .style(style::button::secondary),
            )
            .action(
                button(label::body("Kaydet"))
                    .on_press(msg(SetMessage::Save))
                    .style(style::button::primary),
            )
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn designer() -> Designer {
        let fx: Value =
            serde_json::from_str(include_str!("../../../fixtures/sheet/v1/drawing-set.json"))
                .unwrap();
        let mut d = Designer::default();
        d.load(serde_json::from_value(fx["book"].clone()).unwrap());
        d.update(Message::Tab(1));
        d
    }
    #[test]
    fn drawing_set_drafts_survive_row_switch_and_save_as_one_undo() {
        let mut d = designer();
        let before = d.book().clone();
        for m in [
            SetMessage::Open,
            SetMessage::Edit(Field::Number, "A-101".into()),
            SetMessage::Edit(Field::Subset, "Mimari / Kat planları".into()),
            SetMessage::Select(1),
            SetMessage::Edit(Field::Number, "A-201".into()),
            SetMessage::Select(0),
        ] {
            d.drawing_set_update(m);
        }
        let Some(Window::DrawingSet(state)) = &d.dialog else {
            panic!("dialog")
        };
        assert_eq!(state.entries[0].subset, "Mimari / Kat planları");
        d.drawing_set_update(SetMessage::Save);
        assert_eq!(
            d.book().sheets[0].drawing_info.as_ref().unwrap().number,
            "A-101"
        );
        assert_eq!(
            d.book().sheets[1].drawing_info.as_ref().unwrap().number,
            "A-201"
        );
        d.update(Message::Undo);
        assert_eq!(d.book(), &before);
        d.update(Message::Redo);
        assert_eq!(
            d.book().sheets[0].drawing_info.as_ref().unwrap().subset,
            ["Mimari", "Kat planları"]
        );
    }
    #[test]
    fn drawing_set_rejects_duplicate_numbers_keeps_draft_then_inserts_live_index() {
        let mut d = designer();
        let before = d.book().clone();
        for m in [
            SetMessage::Open,
            SetMessage::Edit(Field::Number, "A".into()),
            SetMessage::Select(1),
            SetMessage::Edit(Field::Number, "A".into()),
            SetMessage::Save,
        ] {
            d.drawing_set_update(m);
        }
        assert_eq!(d.book(), &before);
        assert!(matches!(&d.dialog,Some(Window::DrawingSet(s)) if s.error.is_some()));
        d.drawing_set_update(SetMessage::Edit(Field::Number, "B".into()));
        d.drawing_set_update(SetMessage::Views(true));
        d.drawing_set_update(SetMessage::InsertIndex);
        assert!(
            matches!(&d.book().sheets[0].items.last().unwrap().kind,ItemKind::Table(t) if matches!(&t.source,TableSource::DrawingSet(i) if i.views))
        );
        d.update(Message::Undo);
        assert_eq!(d.book(), &before);
    }
}
