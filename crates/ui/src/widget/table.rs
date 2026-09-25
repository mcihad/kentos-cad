//! Veri tablosu: sabit başlık satırı, sıralanabilir sütunlar ve seçilebilir
//! satırlar.
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

use iced::alignment::Horizontal;
use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{button, container, row, scrollable};
use iced::{Center, Element, Fill, Length, Padding, Point};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::context_menu::{ContextMenu, Menu};
use crate::widget::virtual_list::VirtualList;

/// Sütunlar arasındaki boşluk.
pub(crate) const SPACING: f32 = 8.0;
/// Satırların yatay iç boşluğu.
pub(crate) const PADDING_X: f32 = 10.0;
/// Sanal tablonun satır yüksekliği, 12 piksellik gövde metnine göre.
pub const ROW: f32 = 25.0;

/// Sanal tablonun satır yüksekliği, geçerli yazı boyutunda.
pub fn row_height() -> f32 {
    typography::scaled(ROW)
}
/// Sağa hizalı hücrelerin sağındaki boşluk; ardından gelen sola hizalı
/// sütunun metnine yapışmasınlar. Sabit genişlikli sütunlarda genişliğe
/// eklenir, içeriğin alanını daraltmaz.
const RIGHT_INSET: f32 = 4.0;

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
}

impl<'a, Message> Column<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            width: Length::Shrink,
            align: Horizontal::Left,
            sort: None,
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
}

impl<'a, Message: 'a> Row<'a, Message> {
    pub fn new(cells: impl IntoIterator<Item = Element<'a, Message>>) -> Self {
        Self {
            cells: cells.into_iter().collect(),
            selected: false,
            current: None,
            on_press: None,
            menu: None,
        }
    }

    /// Seçili satır vurgu zeminiyle gösterilir.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Çoklu seçimde birincil satır ayrıca kenarla gösterilir. Verilmezse
    /// seçili satır birincil sayılır.
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

    /// Yatay kaydırmada tablonun toplam genişliği.
    fn content_width(&self) -> f32 {
        let columns: f32 = self
            .columns
            .iter()
            .map(|column| match cell_width(column.width, column.align) {
                Length::Fixed(width) => width,
                _ => typography::scaled(120.0),
            })
            .sum();

        let natural =
            columns + SPACING * self.columns.len().saturating_sub(1) as f32 + PADDING_X * 2.0;

        natural.max(self.min_width)
    }
}

/// Hücrenin genişliği: sabit genişlik yazı boyutuyla büyür; sağa hizalı
/// sabit sütunlarda sağ boşluk eklenir.
fn cell_width(width: Length, align: Horizontal) -> Length {
    match (width, align) {
        (Length::Fixed(width), Horizontal::Right) => {
            Length::Fixed(typography::scaled(width) + RIGHT_INSET)
        }
        (Length::Fixed(width), _) => Length::Fixed(typography::scaled(width)),
        (width, _) => width,
    }
}

/// Sütunların genişliği ve hizası.
pub(crate) fn layout<Message>(columns: &[Column<'_, Message>]) -> Vec<(Length, Horizontal)> {
    columns
        .iter()
        .map(|column| (column.width, column.align))
        .collect()
}

/// Başlık satırı: sıralanabilir sütunlarda başlık düğmedir ve sıralama yönü
/// okla gösterilir.
pub(crate) fn header<'a, Message: Clone + 'a>(
    columns: Vec<Column<'a, Message>>,
    layout: &[(Length, Horizontal)],
) -> Element<'a, Message> {
    let titles = columns.into_iter().map(|column| {
        let title = label::caption(column.title);

        match column.sort {
            Some((order, on_press)) => {
                let mut content =
                    row![title.style(|_theme| iced::widget::text::Style { color: None })]
                        .spacing(3)
                        .align_y(Center);

                if let Some(order) = order {
                    content = content.push(
                        icon(match order {
                            SortOrder::Ascending => Icon::ChevronUp,
                            SortOrder::Descending => Icon::ChevronDown,
                        })
                        .size(10.0),
                    );
                }

                button(content)
                    .on_press(on_press)
                    .padding(0)
                    .style(style::button::header(order.is_some()))
                    .into()
            }
            None => title.into(),
        }
    });

    container(line(layout, titles.collect::<Vec<_>>()))
        .padding([3.0, PADDING_X])
        .width(Fill)
        .style(style::container::surface)
        .into()
}

/// Hücreleri sütunların genişliğine ve hizasına yerleştirir.
pub(crate) fn line<'a, Message: 'a>(
    columns: &[(Length, Horizontal)],
    cells: impl IntoIterator<Item = Element<'a, Message>>,
) -> iced::widget::Row<'a, Message> {
    iced::widget::Row::with_children(columns.iter().zip(cells).map(|(&(width, align), cell)| {
        let inset = if align == Horizontal::Right {
            RIGHT_INSET
        } else {
            0.0
        };

        container(cell)
            .width(cell_width(width, align))
            .align_x(align)
            .padding(Padding::ZERO.right(inset))
            .clip(true)
            .into()
    }))
    .spacing(SPACING)
    .align_y(Center)
}

/// Satırın öğesi: hücreler sütunlara yerleşir, satır tıklanır ve sağ
/// tıklanınca menüsü açılır. Sanal tabloda yükseklik sabittir.
fn row_element<'a, Message: Clone + 'a>(
    row: Row<'a, Message>,
    layout: &[(Length, Horizontal)],
    height: Option<f32>,
) -> Element<'a, Message> {
    let current = row.current.unwrap_or(row.selected);

    let line = line(layout, row.cells);

    // Sabit yükseklikli satırda hücreler dikeyde ortalanır.
    let content = match height {
        Some(height) => button(container(line).center_y(Fill))
            .height(height)
            .padding([0.0, PADDING_X]),
        None => button(line).padding([4.0, PADDING_X]),
    }
    .on_press_maybe(row.on_press)
    .width(Fill)
    .style(style::button::table_row(row.selected, current));

    match row.menu {
        Some(menu) => ContextMenu::new(content, menu).into(),
        None => content.into(),
    }
}

impl<'a, Message: Clone + 'a> From<Table<'a, Message>> for Element<'a, Message> {
    fn from(table: Table<'a, Message>) -> Self {
        let width = table.horizontal.then(|| table.content_width());
        let layout = layout(&table.columns);

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
                        row_element(view(index), &row_layout, Some(height))
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
                        .map(|row| row_element(row, &layout, None)),
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
            Some(width) => scrollable(content.width(width).height(Fill))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(6).scroller_width(6),
                ))
                .width(Fill)
                .height(table.height.unwrap_or(Length::Shrink))
                .into(),
            None => content.width(Fill).into(),
        }
    }
}
