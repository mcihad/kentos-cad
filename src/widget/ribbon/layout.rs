//! Şerit grupları ve grup içi yerleşim yardımcıları.

use std::rc::Rc;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, column, container, row, space, text};
use iced::{Center, Color, Element, Fill, Length, Padding, Size, Top};

use super::{ICON, ROW_GAP, caption_height, content_height, row_height};
use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::color::{self, Ramp};
use crate::widget::dropdown::{Dropdown, Reaction};

/// Başlıklı araç grubu. Öğeler yan yana dizilir; altta ortalanmış grup
/// adı bulunur.
pub struct Group<'a, Message> {
    title: Fragment<'a>,
    items: Vec<Element<'a, Message>>,
}

impl<'a, Message: 'a> Group<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            items: Vec::new(),
        }
    }

    /// Büyük düğme, [`Stack`] veya [`Row`] ekler.
    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }
}

impl<'a, Message: 'a> From<Group<'a, Message>> for Element<'a, Message> {
    fn from(group: Group<'a, Message>) -> Self {
        column![
            container(iced::widget::Row::with_children(group.items).spacing(2))
                .height(content_height())
                .align_y(Top),
            container(label::caption(group.title))
                .height(caption_height())
                .align_y(Center),
        ]
        .padding(Padding {
            top: super::PANEL_PADDING_TOP,
            right: 8.0,
            bottom: super::PANEL_PADDING_BOTTOM,
            left: 8.0,
        })
        .height(Fill)
        .align_x(Center)
        .into()
    }
}

/// Öğeleri ızgara satırlarına üstten hizalı, alt alta dizer. Üçten az
/// öğeli yığınlar da üst satırlara oturur; böylece gruplar arasında satırlar
/// hizalı kalır.
pub struct Stack<'a, Message> {
    items: Vec<Element<'a, Message>>,
    width: Length,
}

impl<'a, Message: 'a> Stack<'a, Message> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            width: Length::Shrink,
        }
    }

    /// Küçük düğme, [`Field`] veya [`Row`] ekler.
    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: 'a> Default for Stack<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<Stack<'a, Message>> for Element<'a, Message> {
    fn from(stack: Stack<'a, Message>) -> Self {
        Column::with_children(stack.items)
            .spacing(ROW_GAP)
            .width(stack.width)
            .into()
    }
}

/// Bir ızgara satırında eşit genişlikli hücreler (ör. Göster, Gizle,
/// Sığdır).
pub struct Row<'a, Message> {
    items: Vec<Element<'a, Message>>,
}

impl<'a, Message: 'a> Row<'a, Message> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        self.items.push(item.into());
        self
    }
}

impl<'a, Message: 'a> Default for Row<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<Row<'a, Message>> for Element<'a, Message> {
    fn from(cells: Row<'a, Message>) -> Self {
        iced::widget::Row::with_children(
            cells
                .items
                .into_iter()
                .map(|item| container(item).width(Fill).into()),
        )
        .height(row_height())
        .into()
    }
}

/// Bir ızgara satırında önde küçük bir işaret (ör. renk örneği) ve bir
/// kontrol (ör. açılır liste). İşaret küçük düğmelerin ikon sütununa,
/// kontrol ise etiket sütununa hizalanır.
pub struct Field<'a, Message> {
    leading: Element<'a, Message>,
    control: Element<'a, Message>,
}

impl<'a, Message: 'a> Field<'a, Message> {
    pub fn new(
        leading: impl Into<Element<'a, Message>>,
        control: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            leading: leading.into(),
            control: control.into(),
        }
    }
}

impl<'a, Message: 'a> From<Field<'a, Message>> for Element<'a, Message> {
    fn from(field: Field<'a, Message>) -> Self {
        row![
            container(field.leading).width(ICON).align_x(Center),
            field.control,
        ]
        .spacing(6)
        .padding([0, 6])
        .height(row_height())
        .align_y(Center)
        .into()
    }
}

/// Galeri karosunun önizlemesi.
#[derive(Debug, Clone, PartialEq)]
pub enum Preview {
    /// Renk örneği.
    Color(Color),
    /// Renk rampası.
    Ramp(Ramp),
    /// İkon.
    Icon(Icon),
    /// Çizgi örneği: rengi ve kalınlığı (piksel).
    Line(Color, f32),
}

/// Galerideki seçenek.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub label: String,
    pub preview: Preview,
}

impl Tile {
    pub fn new(label: impl Into<String>, preview: Preview) -> Self {
        Self {
            label: label.into(),
            preview,
        }
    }
}

/// Şerit galerisi: seçeneklerin önizlemeleri bir satırda; ⌄ düğmesi
/// hepsini ızgarada açar. Satırdaki karolar seçili karoyu içerecek biçimde
/// kayar.
pub struct Gallery<'a, Message> {
    tiles: Vec<Tile>,
    selected: Option<usize>,
    on_select: Rc<dyn Fn(usize) -> Message + 'a>,
    columns: usize,
}

impl<'a, Message: Clone + 'a> Gallery<'a, Message> {
    pub fn new(
        tiles: impl IntoIterator<Item = Tile>,
        selected: Option<usize>,
        on_select: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        Self {
            tiles: tiles.into_iter().collect(),
            selected,
            on_select: Rc::new(on_select),
            columns: 4,
        }
    }

    /// Satırda görünen karo sayısı (varsayılan 4).
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }
}

/// Karonun genişliği ve önizlemenin boyutu.
const TILE: f32 = 50.0;
const PREVIEW: Size = Size::new(34.0, 20.0);
/// Açılan ızgarada bir satırdaki karo sayısı.
const GRID_COLUMNS: usize = 5;

impl<'a, Message: Clone + 'a> From<Gallery<'a, Message>> for Element<'a, Message> {
    fn from(gallery: Gallery<'a, Message>) -> Self {
        let Gallery {
            tiles,
            selected,
            on_select,
            columns,
        } = gallery;
        let width = typography::scaled(TILE);

        // Seçili karo satırda kalır: satır gerekirse onu son karo yapacak
        // kadar kayar.
        let first = selected.map_or(0, |selected| (selected + 1).saturating_sub(columns));
        let inline = tiles.iter().enumerate().skip(first).take(columns).fold(
            iced::widget::Row::new().spacing(1),
            |row, (index, tile)| {
                row.push(
                    button(tile_content(tile))
                        .on_press((on_select)(index))
                        .width(width)
                        .height(content_height())
                        .padding([4, 2])
                        .style(style::button::tool(selected == Some(index))),
                )
            },
        );

        let grid_tiles = tiles.clone();
        let reduce_select = on_select.clone();

        let more = Dropdown::new(
            container(icon(Icon::ChevronDown).size(10.0))
                .center_x(typography::scaled(16.0))
                .center_y(content_height())
                .style(style::container::field_box),
            || (),
            move |_: &()| grid(&grid_tiles, selected),
            move |_: &mut (), index: usize| Reaction::commit((reduce_select)(index)),
        );

        row![inline, more].spacing(2).into()
    }
}

/// Karonun içeriği: önizleme ve adı.
fn tile_content<'a, Message: 'a>(tile: &Tile) -> Element<'a, Message> {
    // Karolar eşit genişliktedir; uzun ad kırpılır.
    column![
        preview(&tile.preview),
        container(
            text(tile.label.clone())
                .font(typography::ui())
                .size(typography::caption())
                .wrapping(iced::widget::text::Wrapping::None),
        )
        .center_x(Fill)
        .clip(true),
    ]
    .spacing(4)
    .width(Fill)
    .align_x(Center)
    .into()
}

fn preview<'a, Message: 'a>(preview: &Preview) -> Element<'a, Message> {
    let size = Size::new(
        typography::scaled(PREVIEW.width),
        typography::scaled(PREVIEW.height),
    );

    match preview {
        Preview::Color(color) => container(space::horizontal())
            .width(size.width)
            .height(size.height)
            .style(style::container::swatch(*color))
            .into(),
        Preview::Ramp(ramp) => color::preview(ramp, size.width, size.height),
        Preview::Icon(glyph) => container(icon(*glyph).size(20.0))
            .center_x(size.width)
            .center_y(size.height)
            .into(),
        Preview::Line(color, weight) => {
            let (color, weight) = (*color, *weight);

            container(
                container(space::horizontal())
                    .width(Fill)
                    .height(weight.max(1.0))
                    .style(move |_theme| container::Style {
                        background: Some(color.into()),
                        ..container::Style::default()
                    }),
            )
            .width(size.width)
            .center_y(size.height)
            .into()
        }
    }
}

/// Bütün karoların ızgarası (açılan panel).
fn grid<'a>(tiles: &[Tile], selected: Option<usize>) -> Element<'a, usize> {
    let width = typography::scaled(TILE + 8.0);

    let rows = tiles
        .iter()
        .enumerate()
        .collect::<Vec<_>>()
        .chunks(GRID_COLUMNS)
        .fold(Column::new().spacing(2), |column, chunk| {
            column.push(chunk.iter().fold(
                iced::widget::Row::new().spacing(2),
                |row, (index, tile)| {
                    row.push(
                        button(tile_content(tile))
                            .on_press(*index)
                            .width(width)
                            .padding([6, 2])
                            .style(style::button::tool(selected == Some(*index))),
                    )
                },
            ))
        });

    container(rows)
        .padding(6)
        .style(style::container::popover)
        .into()
}
