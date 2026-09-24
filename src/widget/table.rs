//! Veri tablosu: sabit başlık satırı ve seçilebilir satırlar.
//!
//! ```ignore
//! Table::new([
//!     table::Column::new("Ad").width(Fill),
//!     table::Column::new("Öğe").width(24).align_right(),
//! ])
//! .push(table::Row::new([name, count]).selected(true).on_press(Message::Select(0)))
//! ```
//!
//! Hücreler sütun genişliğine ve hizasına göre yerleştirilir; başlık ve
//! satırlar aynı aralıkla dizildiği için sütunlar hizalı kalır. Hücrelerin
//! içindeki onay kutusu ve düğmeler satır tıklamasından önce olayı alır.

use iced::alignment::Horizontal;
use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{button, container, scrollable};
use iced::{Center, Element, Fill, Length};

use crate::label;
use crate::style;

const SPACING: f32 = 8.0;

/// Tablo sütunu.
pub struct Column<'a> {
    title: Fragment<'a>,
    width: Length,
    align: Horizontal,
}

impl<'a> Column<'a> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            width: Length::Shrink,
            align: Horizontal::Left,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sayısal sütunlar için sağa hizalama.
    pub fn align_right(mut self) -> Self {
        self.align = Horizontal::Right;
        self
    }
}

/// Tablo satırı.
pub struct Row<'a, Message> {
    cells: Vec<Element<'a, Message>>,
    selected: bool,
    on_press: Option<Message>,
}

impl<'a, Message: 'a> Row<'a, Message> {
    pub fn new(cells: impl IntoIterator<Item = Element<'a, Message>>) -> Self {
        Self {
            cells: cells.into_iter().collect(),
            selected: false,
            on_press: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

/// Veri tablosu.
pub struct Table<'a, Message> {
    columns: Vec<Column<'a>>,
    rows: Vec<Row<'a, Message>>,
    height: Option<Length>,
    empty: Option<Fragment<'a>>,
}

impl<'a, Message: Clone + 'a> Table<'a, Message> {
    pub fn new(columns: impl IntoIterator<Item = Column<'a>>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: Vec::new(),
            height: None,
            empty: None,
        }
    }

    pub fn push(mut self, row: Row<'a, Message>) -> Self {
        self.rows.push(row);
        self
    }

    pub fn extend(mut self, rows: impl IntoIterator<Item = Row<'a, Message>>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// Satırların yüksekliği; verilirse satırlar kayar, başlık sabit kalır.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Hiç satır yokken gösterilecek açıklama.
    pub fn empty(mut self, message: impl IntoFragment<'a>) -> Self {
        self.empty = Some(message.into_fragment());
        self
    }
}

/// Hücreleri sütunların genişliğine ve hizasına yerleştirir.
fn line<'a, Message: 'a>(
    columns: &[Column<'a>],
    cells: impl IntoIterator<Item = Element<'a, Message>>,
) -> iced::widget::Row<'a, Message> {
    iced::widget::Row::with_children(columns.iter().zip(cells).map(|(column, cell)| {
        container(cell)
            .width(column.width)
            .align_x(column.align)
            .into()
    }))
    .spacing(SPACING)
    .align_y(Center)
}

impl<'a, Message: Clone + 'a> From<Table<'a, Message>> for Element<'a, Message> {
    fn from(table: Table<'a, Message>) -> Self {
        let header = container(line(
            &table.columns,
            table
                .columns
                .iter()
                .map(|column| label::caption(column.title.clone()).into())
                .collect::<Vec<_>>(),
        ))
        .padding([3, 10])
        .width(Fill)
        .style(style::container::surface);

        let body: Element<'a, Message> = match (table.rows.is_empty(), table.empty) {
            (true, Some(message)) => container(label::muted(message))
                .padding([8, 10])
                .width(Fill)
                .into(),
            _ => iced::widget::Column::with_children(table.rows.into_iter().map(|row| {
                button(line(&table.columns, row.cells))
                    .on_press_maybe(row.on_press)
                    .width(Fill)
                    .padding([4, 10])
                    .style(style::button::row(row.selected))
                    .into()
            }))
            .width(Fill)
            .into(),
        };

        let body = match table.height {
            Some(height) => scrollable(body)
                .direction(style::field::thin_scrollbar())
                .width(Fill)
                .height(height)
                .into(),
            None => body,
        };

        iced::widget::Column::new()
            .push(header)
            .push(body)
            .width(Fill)
            .into()
    }
}
