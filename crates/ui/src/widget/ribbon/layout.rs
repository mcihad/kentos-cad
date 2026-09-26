//! Şerit grupları ve grup içi yerleşim yardımcıları.

use std::rc::Rc;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, column, container, row, space, text, tooltip};
use iced::{Center, Color, Element, Fill, Length, Padding, Size, Top};

use super::control::Form;
use super::{Button, ICON, ROW_GAP, caption_height, content_height, row_height};
use crate::icon::{Icon, Tone, icon};
use crate::style;
use crate::theme::typography;
use crate::widget::color::{self, Ramp};
use crate::widget::context_menu::{Menu, MenuButton};
use crate::widget::dropdown::{Dropdown, Reaction};
use crate::widget::{Tip, tip};

/// Başlıklı araç grubu (panel). Altta ortalanmış grup adı bulunur.
///
/// Düğmelerle ([`Group::tool`]) kurulan ya da kendi görünüşünü veren
/// ([`Group::custom`]) grup şeride sığmak için küçülür: panel önce bütün
/// düğmelerini küçük etiketliye, sonra yalnız ikona indirir, en sonda
/// adını taşıyan tek düğmeye katlanır (DESIGN.md §7.3.1). [`Group::push`] ile
/// verilen öğeler (alanlar, galeriler) olduğu gibi kalır.
pub struct Group<'a, Message> {
    title: Fragment<'a>,
    icon: Icon,
    content: Content<'a, Message>,
    keep: bool,
    more: Option<Rc<dyn Fn() -> Menu<Message> + 'a>>,
    launcher: Option<(Message, String)>,
}

enum Content<'a, Message> {
    /// Olduğu gibi çizilen öğeler.
    Fixed(Vec<Element<'a, Message>>),
    /// Şeridin panelin genişliğine göre dizdiği düğmeler.
    Tools(Vec<Button<'a, Message>>),
    /// Kendi görünüşü (tam genişlikte) ve katlanınca açılan menüsü.
    Custom {
        width: f32,
        view: Rc<dyn Fn() -> Element<'a, Message> + 'a>,
        menu: Rc<dyn Fn() -> Menu<Message> + 'a>,
    },
}

/// Panelin sağındaki ve solundaki iç boşluk.
const PANEL_PAD: f32 = 5.0;
/// Paneldeki öğeler arası.
const ITEM_GAP: f32 = 2.0;
/// Başlığın iki yanındaki pay.
const FOOT_PAD: f32 = 14.0;

impl<'a, Message: Clone + 'a> Group<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            icon: Icon::More,
            content: Content::Tools(Vec::new()),
            keep: false,
            more: None,
            launcher: None,
        }
    }

    /// Büyük düğme, [`Stack`] veya [`Row`] ekler; grup olduğu gibi kalır.
    pub fn push(mut self, item: impl Into<Element<'a, Message>>) -> Self {
        match &mut self.content {
            Content::Fixed(items) => items.push(item.into()),
            Content::Tools(tools) => {
                let mut items: Vec<Element<'a, Message>> =
                    tools.drain(..).map(Element::from).collect();
                items.push(item.into());
                self.content = Content::Fixed(items);
            }
            Content::Custom { view, .. } => {
                self.content = Content::Fixed(vec![view(), item.into()]);
            }
        }
        self
    }

    /// Düğme ekler: büyük düğmeler tek başına, küçükler sütunda üçer durur.
    pub fn tool(mut self, button: Button<'a, Message>) -> Self {
        match &mut self.content {
            Content::Tools(tools) => tools.push(button),
            Content::Fixed(items) => items.push(button.into()),
            Content::Custom { .. } => {}
        }
        self
    }

    /// Kendi görünüşü olan grup (ör. tema seçimi): `width` tam genişliği,
    /// `menu` katlanınca açılan menüdür.
    pub fn custom(
        mut self,
        width: f32,
        view: impl Fn() -> Element<'a, Message> + 'a,
        menu: impl Fn() -> Menu<Message> + 'a,
    ) -> Self {
        self.content = Content::Custom {
            width,
            view: Rc::new(view),
            menu: Rc::new(menu),
        };
        self
    }

    /// Katlanmış panelin düğmesindeki ikon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    /// Öbür paneller yalnız ikona inene kadar büyük düğmelerini korur
    /// (Giriş'te Çizim ve Değiştir).
    pub fn keep(mut self, keep: bool) -> Self {
        self.keep = keep;
        self
    }

    /// Seyrek araçlar: başlığın yanındaki ▾ ile açılır.
    pub fn more(mut self, menu: impl Fn() -> Menu<Message> + 'a) -> Self {
        self.more = Some(Rc::new(menu));
        self
    }

    /// Başlığın sağındaki pencere açıcı (↘).
    pub fn launcher(mut self, message: Message, title: impl Into<String>) -> Self {
        self.launcher = Some((message, title.into()));
        self
    }

    /// Şeridin boyutuna göre küçülebilir mi.
    pub(crate) fn adaptive(&self) -> bool {
        !matches!(self.content, Content::Fixed(_))
    }

    /// Öbür paneller yalnız ikona inene kadar büyük düğmelerini korur mu.
    pub fn keeps(&self) -> bool {
        self.keep
    }

    /// Dört seviyedeki genişlikleri (piksel): şeridin [`fit`](super::fit)'i
    /// bunlarla seçer; uygulama sekmelerinin sığdığını sınamak için okur.
    pub fn widths(&self) -> [f32; 4] {
        [0, 1, 2, 3].map(|level| self.width(level))
    }

    /// Seviyedeki genişliği (piksel, sağındaki bölücü dahil).
    pub(crate) fn width(&self, level: Level) -> f32 {
        let body = match (&self.content, level) {
            (_, 3) => return self.folded_width() + PANEL_PAD * 2.0 + 1.0,
            (Content::Tools(tools), level) => tools_width(tools, level),
            (Content::Custom { width, .. }, _) => *width,
            (Content::Fixed(_), _) => 0.0,
        };
        body.max(self.foot_width()) + PANEL_PAD * 2.0 + 1.0
    }

    /// Başlık satırının genişliği.
    fn foot_width(&self) -> f32 {
        let title = typography::text_width(&self.title, foot_size());
        let more = if self.more.is_some() {
            3.0 + 10.0 + 10.0
        } else {
            0.0
        };
        let launcher = if self.launcher.is_some() { 16.0 } else { 0.0 };
        title + more + FOOT_PAD * 2.0 + launcher
    }

    /// Tek düğmeye katlanmış hâlinin genişliği.
    fn folded_width(&self) -> f32 {
        let s = typography::scaled;
        let label = typography::text_width(&self.title, typography::caption()) + 14.0;
        label.ceil().clamp(s(50.0), s(110.0))
    }

    /// Grubu seviyede çizer: 0 tasarlandığı gibi, 1 bütün düğmeler küçük
    /// etiketli, 2 yalnız ikon, 3 adını taşıyan tek düğme.
    pub(crate) fn view(&self, level: Level) -> Element<'a, Message> {
        if level >= 3 {
            return self.folded();
        }
        let body: Element<'a, Message> = match &self.content {
            Content::Tools(tools) => tools_row(tools, level),
            Content::Custom { view, .. } => view(),
            Content::Fixed(_) => space::horizontal().width(0).into(),
        };
        self.frame(body, self.width(level) - 1.0)
    }

    /// Gövde ve altında başlık satırı; genişlik sığdırmanın hesapladığıdır
    /// (bölücü hariç), içerik ona göre ortalanır.
    fn frame(&self, body: impl Into<Element<'a, Message>>, width: f32) -> Element<'a, Message> {
        column![
            container(body).height(content_height()).align_y(Top),
            self.foot(),
        ]
        .width(width)
        .padding(Padding {
            top: super::PANEL_PADDING_TOP,
            right: PANEL_PAD,
            bottom: super::PANEL_PADDING_BOTTOM,
            left: PANEL_PAD,
        })
        .height(Fill)
        .align_x(Center)
        .into()
    }

    /// Başlık satırı: ortada ad (seyrek araçlar varsa ▾ ile menü), sağda
    /// pencere açıcı.
    fn foot(&self) -> Element<'a, Message> {
        let title = || {
            text(self.title.clone())
                .font(typography::ui())
                .size(foot_size())
                .wrapping(iced::widget::text::Wrapping::None)
                .style(style::text::muted)
        };
        let middle: Element<'a, Message> = match &self.more {
            Some(menu) => {
                let menu = menu.clone();
                tip(
                    MenuButton::new(
                        container(
                            row![title(), icon(Icon::ChevronDown).size(8.0).tone(Tone::Muted)]
                                .spacing(3)
                                .align_y(Center),
                        )
                        .padding([0, 5])
                        .center_y(caption_height()),
                        move || menu(),
                    ),
                    Tip::new(format!("{}: diğer araçlar", self.title)),
                    tooltip::Position::Bottom,
                )
            }
            None => title().into(),
        };
        let launcher: Element<'a, Message> = match &self.launcher {
            Some((message, name)) => tip(
                button(icon(Icon::Maximize).size(10.0).tone(Tone::Muted))
                    .on_press(message.clone())
                    .padding(2)
                    .style(style::button::ribbon(style::button::Ribbon::Idle)),
                Tip::new(name.clone()),
                tooltip::Position::Bottom,
            ),
            None => space::horizontal().width(0).into(),
        };
        row![
            space::horizontal().width(Fill),
            middle,
            container(launcher).width(Fill).align_x(iced::Right),
        ]
        .height(caption_height())
        .align_y(Center)
        .into()
    }

    /// Adını taşıyan tek düğme; tıklayınca panelin araçları menüde açılır.
    fn folded(&self) -> Element<'a, Message> {
        let width = self.folded_width();
        let menu = self.folded_menu();
        let face = container(
            column![
                icon(self.icon)
                    .size(super::LARGE_ICON)
                    .weight(super::LARGE_WEIGHT),
                row![
                    text(self.title.clone())
                        .font(typography::ui())
                        .size(typography::caption())
                        .wrapping(iced::widget::text::Wrapping::None),
                    icon(Icon::ChevronDown).size(9.0).tone(Tone::Muted),
                ]
                .spacing(3)
                .align_y(Center),
            ]
            .spacing(typography::scaled(6.0))
            .align_x(Center),
        )
        .center_x(width)
        .height(Fill)
        .padding(Padding {
            top: typography::scaled(10.0),
            ..Padding::ZERO
        });
        container(tip(
            MenuButton::new(face, move || menu()),
            Tip::new(self.title.to_string()).body(
                "Pencere dar olduğu için panel tek düğmeye katlandı; tıklayınca araçları açılır.",
            ),
            tooltip::Position::Bottom,
        ))
        .width(self.width(3) - 1.0)
        .padding(Padding {
            top: super::PANEL_PADDING_TOP,
            right: PANEL_PAD,
            bottom: super::PANEL_PADDING_BOTTOM,
            left: PANEL_PAD,
        })
        .height(Fill)
        .into()
    }

    /// Katlanmış panelin menüsü: düğmeleri (aileler alt menü) ve seyrek araçları.
    fn folded_menu(&self) -> Rc<dyn Fn() -> Menu<Message> + 'a> {
        let title = self.title.to_string();
        let more = self.more.clone();
        match &self.content {
            Content::Custom { menu, .. } => menu.clone(),
            Content::Tools(tools) => {
                let tools = tools.clone();
                Rc::new(move || {
                    let menu = tools
                        .iter()
                        .fold(Menu::new().header(title.clone()), |menu, tool| {
                            tool.menu_entry(menu)
                        });
                    match &more {
                        Some(more) => menu.separator().submenu("Diğer araçlar", more()),
                        None => menu,
                    }
                })
            }
            Content::Fixed(_) => Rc::new(Menu::new),
        }
    }
}

/// Başlık satırının yazı boyutu (DESIGN.md: `--fs-2xs`).
fn foot_size() -> f32 {
    typography::caption() - 1.0
}

/// Panelin seviyesi: 0 tasarlandığı gibi, 1 küçük etiketli, 2 yalnız
/// ikon, 3 tek düğme.
pub type Level = u8;

/// Düğmeleri seviyede dizer: büyükler tek başına, küçükler sütunda üçer.
fn tools_row<'a, Message: Clone + 'a>(
    tools: &[Button<'a, Message>],
    level: Level,
) -> Element<'a, Message> {
    let mut row = iced::widget::Row::new().spacing(ITEM_GAP);
    let mut column: Vec<Element<'a, Message>> = Vec::new();
    let flush = |row: iced::widget::Row<'a, Message>, column: &mut Vec<Element<'a, Message>>| {
        if column.is_empty() {
            return row;
        }
        row.push(Column::with_children(column.drain(..)).spacing(ROW_GAP))
    };
    for tool in tools {
        if level == 0 && tool.is_large() {
            row = flush(row, &mut column);
            row = row.push(tool.render(Form::Large));
            continue;
        }
        column.push(tool.render(if level >= 2 { Form::Icon } else { Form::Small }));
        if column.len() == 3 {
            row = flush(row, &mut column);
        }
    }
    flush(row, &mut column).into()
}

/// [`tools_row`]'un genişliği.
fn tools_width<Message: Clone>(tools: &[Button<'_, Message>], level: Level) -> f32 {
    let (mut total, mut column, mut in_column, mut items) = (0.0_f32, 0.0_f32, 0_u32, 0_u32);
    for tool in tools {
        if level == 0 && tool.is_large() {
            if in_column > 0 {
                total += column;
                items += 1;
                (column, in_column) = (0.0, 0);
            }
            total += tool.width(Form::Large);
            items += 1;
            continue;
        }
        column = column.max(tool.width(if level >= 2 { Form::Icon } else { Form::Small }));
        in_column += 1;
        if in_column == 3 {
            total += column;
            items += 1;
            (column, in_column) = (0.0, 0);
        }
    }
    if in_column > 0 {
        total += column;
        items += 1;
    }
    total + ITEM_GAP * items.saturating_sub(1) as f32
}

impl<'a, Message: Clone + 'a> From<Group<'a, Message>> for Element<'a, Message> {
    fn from(group: Group<'a, Message>) -> Self {
        if group.adaptive() {
            return group.view(0);
        }
        let Content::Fixed(items) = group.content else {
            unreachable!("adaptive groups return above")
        };
        column![
            container(iced::widget::Row::with_children(items).spacing(ITEM_GAP))
                .height(content_height())
                .align_y(Top),
            container(
                text(group.title)
                    .font(typography::ui())
                    .size(foot_size())
                    .style(style::text::muted)
            )
            .height(caption_height())
            .align_y(Center),
        ]
        .padding(Padding {
            top: super::PANEL_PADDING_TOP,
            right: PANEL_PAD + 3.0,
            bottom: super::PANEL_PADDING_BOTTOM,
            left: PANEL_PAD + 3.0,
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
