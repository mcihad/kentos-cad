//! Veri tablosu: kendi bandında sabit başlık, sıralanabilir ve boyutlanabilir sütunlar, ince çizgili seçilebilir satırlar
//! (DESIGN.md §7.6, docs/adr/0805).
//!
//! ```ignore
//! Table::new([
//!     table::Column::new("Ad").width(Fill),
//!     table::Column::new("Öğe").width(24).align_right(),
//! ])
//! .push(table::Row::new([name, count]).selected(true).on_press(Message::Select(0)))
//! ```
//!
//! Hücreler sütun genişliğine ve hizasına göre yerleştirilir; başlık ve satırlar aynı kuralla dizildiği için sütunlar
//! hizalı kalır. Her hücre sütun sınırından sınırına uzanır ve yazısı içeriden başlar; [`Cell`](super::Cell) ise payı
//! olmadan hücreyi doldurur (düzenlenen değer, koşullu zemin). Hücrelerin içindeki onay kutusu ve düğmeler satır
//! tıklamasından önce olayı alır.
//!
//! Öznitelik tablosu gibi çok sütunlu tablolar [`Table::horizontal`] ile
//! yatay kaydırılır; başlık dikey kaydırmada yerinde kalır.
//! [`Table::min_width`] verilirse dar tablonun başlığı ve satır vurgusu yine
//! de bu genişliğe uzanır. Satırlara [`Row::menu`] ile bağlam menüsü
//! eklenebilir.
//!
//! Çok satırlı tablolar [`Table::virtualized`] ile kurulur: satırlar
//! yalnızca görünürken kurulur ([`VirtualList`]), satırların yüksekliği
//! eşittir ([`row_height`]).

mod head;
mod row;
#[cfg(all(test, feature = "snapshot"))]
mod tests;

use iced::alignment::Horizontal;
use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{container, scrollable};
use iced::{Center, Color, Element, Fill, Length, Padding, Point, Theme};

use crate::label;
use crate::style;
use crate::theme::{Tokens, metrics, typography};
use crate::widget::cell::is_cell;
use crate::widget::context_menu::{ContextMenu, Menu};
use crate::widget::virtual_list::VirtualList;

pub(crate) use row::RowBox;

use std::borrow::Cow;

use crate::widget::elided::Elided;

/// Hücrenin yazısının sütun sınırına uzaklığı.
pub const PAD: f32 = 6.0;
/// Tablonun iki ucundaki hücrenin yazısının kenara uzaklığı.
pub const PADDING_X: f32 = 10.0;
/// Tablonun satır yüksekliği, geçerli yazı boyutunda: bütün satırlarınki
/// ([`metrics::row`](crate::theme::metrics::row)).
pub fn row_height() -> f32 {
    crate::theme::metrics::row()
}

/// Sıralama yönü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn reversed(self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

/// Tablo sütunu.
pub struct Column<'a, Message> {
    title: Fragment<'a>,
    width: Length,
    align: Horizontal,
    sort: Option<(Option<SortOrder>, Message)>,
    resize: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    fit: Option<Message>,
}

impl<'a, Message> Column<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            width: Length::Shrink,
            align: Horizontal::Left,
            sort: None,
            resize: None,
            fit: None,
        }
    }

    /// Sütunun genişliği. Sabit genişlik 12 piksellik gövde metnine göre
    /// verilir ve yazı boyutuyla büyür.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sayısal sütunlar için sağa hizalama.
    pub fn align_right(mut self) -> Self {
        self.align = Horizontal::Right;
        self
    }

    /// Başlığa tıklanınca `on_press` üretilir; `order` sütun sıralıysa
    /// yönüdür ve başlıkta okla gösterilir.
    pub fn sortable(mut self, order: Option<SortOrder>, on_press: Message) -> Self {
        self.sort = Some((order, on_press));
        self
    }

    /// Sütunun sağ kenarı sürüklenir: `on_resize` yeni genişliği [`width`](Self::width)'in birimiyle verir; tablo
    /// durum tutmaz, genişlik çağıranın saklayıp verdiğidir. Sabit genişlikli sütunlar içindir.
    pub fn resizable(mut self, on_resize: impl Fn(f32) -> Message + 'a) -> Self {
        self.resize = Some(Box::new(on_resize));
        self
    }

    /// Sağ kenara çift tık: sütunu içeriğine sığdırma isteği.
    pub fn on_fit(mut self, on_fit: Message) -> Self {
        self.fit = Some(on_fit);
        self
    }
}

/// Sağ tıklanınca açılan menüyü kuran fonksiyon; sağ tıklanan noktanın
/// satır içindeki konumunu alır.
pub(crate) type MenuBuilder<'a, Message> = Box<dyn Fn(Point) -> Menu<Message> + 'a>;

/// Tablo satırı.
pub struct Row<'a, Message> {
    cells: Vec<Element<'a, Message>>,
    selected: bool,
    current: Option<bool>,
    on_press: Option<Message>,
    menu: Option<MenuBuilder<'a, Message>>,
    tint: Option<Color>,
}

impl<'a, Message: 'a> Row<'a, Message> {
    pub fn new(cells: impl IntoIterator<Item = Element<'a, Message>>) -> Self {
        Self {
            cells: cells.into_iter().collect(),
            selected: false,
            current: None,
            on_press: None,
            menu: None,
            tint: None,
        }
    }

    /// Satırın kendi zemini (koşullu biçim gibi): hücrelerin arasıyla bütün satırı boyar; seçim ve
    /// üzerine gelme vurgusu onun yerine geçer.
    pub fn tint(mut self, tint: Option<Color>) -> Self {
        self.tint = tint;
        self
    }

    /// Seçili satır vurgu zeminiyle gösterilir.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Birkaç satır seçiliyken birincil satır ayrıca kenarla gösterilir.
    pub fn current(mut self, current: bool) -> Self {
        self.current = Some(current);
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Satıra sağ tıklanınca açılan bağlam menüsü.
    pub fn menu(mut self, menu: impl Fn(Point) -> Menu<Message> + 'a) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }
}

/// Veri tablosu.
pub struct Table<'a, Message> {
    columns: Vec<Column<'a, Message>>,
    rows: Vec<Row<'a, Message>>,
    /// Sanal satırlar: sayısı, kurucusu ve görünür yapılacak satır.
    lazy: Option<Lazy<'a, Message>>,
    height: Option<Length>,
    empty: Option<Fragment<'a>>,
    horizontal: bool,
    min_width: f32,
    grid: bool,
}

struct Lazy<'a, Message> {
    count: usize,
    view: Box<dyn Fn(usize) -> Row<'a, Message> + 'a>,
    reveal: Option<usize>,
}

impl<'a, Message: Clone + 'a> Table<'a, Message> {
    pub fn new(columns: impl IntoIterator<Item = Column<'a, Message>>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: Vec::new(),
            lazy: None,
            height: None,
            empty: None,
            horizontal: false,
            min_width: 0.0,
            grid: false,
        }
    }

    /// Sanal satırlar: `count` satırdan yalnızca görünenler `view` ile
    /// kurulur. Satırlar eşit yükseklikte ([`row_height`]) ve tablo
    /// verilen yüksekliği (varsayılan `Fill`) doldurur.
    pub fn virtualized(
        mut self,
        count: usize,
        view: impl Fn(usize) -> Row<'a, Message> + 'a,
    ) -> Self {
        self.lazy = Some(Lazy {
            count,
            view: Box::new(view),
            reveal: None,
        });
        self
    }

    /// Sanal tabloda satırı görünür yapar (ör. seçilen öğenin satırı).
    pub fn reveal(mut self, index: Option<usize>) -> Self {
        if let Some(lazy) = &mut self.lazy {
            lazy.reveal = index;
        }
        self
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

    /// Sütunlar sığmadığında tablo yatay kayar. Sütun genişlikleri sabit
    /// olmalıdır; esnek sütunlar 120 piksel sayılır.
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }

    /// Yatay kaydırmada tablonun en az genişliği; genellikle tablonun
    /// yerleştiği alanın genişliği. Sütunlar daha dar kalırsa başlık ve
    /// satırlar boş alana uzanır.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// Elektronik tablo gibi: sütun çizgileri satırların arasından da geçer. Hücre hücre okunan ve düzenlenen
    /// tablolar içindir (Öznitelik tablosu, Noktalar).
    pub fn grid(mut self) -> Self {
        self.grid = true;
        self
    }

    /// Yatay kaydırmada tablonun toplam genişliği.
    fn content_width(&self) -> f32 {
        let count = self.columns.len();
        let natural: f32 = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, column)| {
                let (left, right) = cell_pads(i, count);
                let width = match column.width {
                    Length::Fixed(width) => width,
                    _ => 120.0,
                };
                typography::scaled(width) + left + right
            })
            .sum();

        natural.max(self.min_width)
    }
}

/// `count` sütunun `i`'ncisinin sol ve sağ payı: uçtakiler tablonun kenarından daha çok uzak durur. Hücreyi dolduran
/// [`Cell`](super::Cell)'in yazısı bu paylarla içeriden başlar: düzenlemeye geçince yazı kaymaz.
pub fn cell_pads(i: usize, count: usize) -> (f32, f32) {
    (
        if i == 0 { PADDING_X } else { PAD },
        if i + 1 == count { PADDING_X } else { PAD },
    )
}

/// Hücrenin genişliği, payları dahil: sabit genişlik yazı alanıdır ve yazı boyutuyla büyür.
fn cell_width(width: Length, pads: (f32, f32)) -> Length {
    match width {
        Length::Fixed(width) => Length::Fixed(typography::scaled(width) + pads.0 + pads.1),
        width => width,
    }
}

/// Sütunların genişliği ve hizası.
pub(crate) fn layout<Message>(columns: &[Column<'_, Message>]) -> Vec<(Length, Horizontal)> {
    columns
        .iter()
        .map(|column| (column.width, column.align))
        .collect()
}

/// Hücrenin yazısı: tek satır, sığmazsa “…” ile biter (sütun sürüklenerek açılır).
pub fn text<'a>(value: impl Into<Cow<'a, str>>) -> Elided<'a> {
    Elided::new(value)
        .size(typography::body())
        .font(typography::ui())
}

/// Sütunda hizalanacak sayı: eş aralıklı, tek satır; sığmazsa son basamakları sessizce gitmez, “…” ile biter.
pub fn number<'a>(value: impl Into<Cow<'a, str>>) -> Elided<'a> {
    Elided::new(value)
        .size(typography::body())
        .font(typography::mono())
}

/// Başlığın yazısı, satırlarını kendisi kuran tablolar için ([`band`]'ın içinde).
pub fn title<'a>(content: impl IntoFragment<'a>) -> iced::widget::Text<'a> {
    iced::widget::text(content)
        .size(typography::caption())
        .font(typography::ui_strong())
        .wrapping(iced::widget::text::Wrapping::None)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(Tokens::of(theme).text),
        })
}

/// Satırlarını kendisi kuran tabloların başlığı (düzenlenen alanlardan oluşan satırlar): [`Table`]'ın bandı ve güçlü
/// çizgisi, içinde çağıranın sütunları.
pub fn band<'a, Message: 'a>(columns: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    iced::widget::Column::new()
        .push(
            container(columns)
                .width(Fill)
                .height(metrics::row())
                .align_y(Center)
                .style(|theme: &Theme| container::Style {
                    background: Some(Tokens::of(theme).table_head().into()),
                    snap: true,
                    ..container::Style::default()
                }),
        )
        .push(
            container(iced::widget::space())
                .width(Fill)
                .height(1)
                .style(|theme: &Theme| container::Style {
                    background: Some(Tokens::of(theme).border_strong().into()),
                    snap: true,
                    ..container::Style::default()
                }),
        )
        .width(Fill)
        .into()
}

/// Başlık satırı.
pub(crate) fn header<'a, Message: Clone + 'a>(
    columns: Vec<Column<'a, Message>>,
    layout: &[(Length, Horizontal)],
) -> Element<'a, Message> {
    head::head(columns, layout).into()
}

/// Hücreleri sütunların genişliğine ve hizasına yerleştirir. Hücreler bitişiktir, payı kendilerindedir; bir
/// [`Cell`](super::Cell) payını kendisi bilir ve sütunu sınırdan sınıra doldurur.
pub(crate) fn line<'a, Message: 'a>(
    columns: &[(Length, Horizontal)],
    cells: impl IntoIterator<Item = Element<'a, Message>>,
) -> iced::widget::Row<'a, Message> {
    let count = columns.len();
    iced::widget::Row::with_children(columns.iter().zip(cells).enumerate().map(
        |(i, (&(width, align), cell))| {
            let pads = cell_pads(i, count);
            // A cell fills its column but for the pixel of the column's line.
            let padding = if is_cell(&cell) {
                Padding::ZERO.right(1.0)
            } else {
                Padding::ZERO.left(pads.0).right(pads.1)
            };

            container(cell)
                .width(cell_width(width, pads))
                .align_x(align)
                .padding(padding)
                .clip(true)
                .into()
        },
    ))
    .align_y(Center)
}

/// Satırın öğesi: hücreler sütunlara yerleşir, satır tıklanır ve sağ
/// tıklanınca menüsü açılır. Sanal tabloda yükseklik sabittir.
fn row_element<'a, Message: Clone + 'a>(
    row: Row<'a, Message>,
    layout: &[(Length, Horizontal)],
    height: Option<f32>,
    grid: bool,
) -> Element<'a, Message> {
    let content = RowBox::new(line(layout, row.cells))
        .height(height)
        .on_press_maybe(row.on_press)
        .selected(row.selected)
        .current(row.current.unwrap_or(false))
        .tint(row.tint)
        .ruled(true)
        .grid(grid);

    match row.menu {
        Some(menu) => ContextMenu::new(content, menu).into(),
        None => content.into(),
    }
}

impl<'a, Message: Clone + 'a> From<Table<'a, Message>> for Element<'a, Message> {
    fn from(table: Table<'a, Message>) -> Self {
        let width = table.horizontal.then(|| table.content_width());
        let layout = layout(&table.columns);
        let grid = table.grid;

        let header = header(table.columns, &layout);
        let empty = match &table.lazy {
            Some(lazy) => lazy.count == 0,
            None => table.rows.is_empty(),
        };

        // Sanal liste kendisi kayar; öbür satırlar verilen yükseklikte
        // kaydırılabilir alana konur.
        let (body, scrolls): (Element<'a, Message>, bool) = match (empty, table.empty, table.lazy) {
            (true, Some(message), _) => (
                container(label::muted(message))
                    .padding([8.0, PADDING_X])
                    .width(Fill)
                    .into(),
                false,
            ),
            (_, _, Some(lazy)) => {
                let height = row_height();
                let row_layout = layout.clone();
                let view = lazy.view;

                (
                    VirtualList::new(lazy.count, height, move |index| {
                        row_element(view(index), &row_layout, Some(height), grid)
                    })
                    .reveal(lazy.reveal)
                    .height(table.height.unwrap_or(Length::Fill))
                    .into(),
                    true,
                )
            }
            (_, _, None) => (
                iced::widget::Column::with_children(
                    table
                        .rows
                        .into_iter()
                        .map(|row| row_element(row, &layout, None, grid)),
                )
                .width(Fill)
                .into(),
                false,
            ),
        };

        let body = match table.height {
            Some(height) if !scrolls => scrollable(body)
                .direction(style::field::thin_scrollbar())
                .width(Fill)
                .height(height)
                .into(),
            _ => body,
        };

        let content = iced::widget::Column::new().push(header).push(body);

        match width {
            // The bar takes its own room while it shows: it never lies over the last row.
            Some(width) => scrollable(content.width(width).height(Fill))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new()
                        .width(6)
                        .scroller_width(6)
                        .spacing(0),
                ))
                .width(Fill)
                .height(table.height.unwrap_or(Length::Shrink))
                .into(),
            None => content.width(Fill).into(),
        }
    }
}
