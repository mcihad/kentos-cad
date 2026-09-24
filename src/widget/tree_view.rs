//! Ağaç görünümü: iç içe gruplar ve öğeler; isteğe bağlı sütunlarla ağaç
//! tablo.
//!
//! ```text
//! Ad                                  Öğe  Opak.
//! ▾ ☑ ▭ Türkiye                        50
//! │ ▾ ☑ ▭ Yerleşim                     26
//! │ │ ▾ ☑ ● Şehirler                   16  100%
//! │ │ │   ☑ ■ Marmara                   2
//! │ │ │   ☐ ■ Ege                       1
//! │ ▸ ☑ ▭ Ulaşım                        7
//! ```
//!
//! İlk sütun ağacı taşır: girinti çizgileri, açma/kapama oku, isteğe bağlı
//! onay kutusu (işaretli, işaretsiz ya da karışık), ikon ve ad. Diğer
//! sütunlar [`Table`](super::Table) ile aynı düzende hizalanır.
//!
//! ```ignore
//! TreeView::new([
//!     tree_view::Column::new("Ad").width(Fill),
//!     tree_view::Column::new("Öğe").width(28).align_right(),
//! ])
//! .push(
//!     Node::new("Yerleşim")
//!         .icon(icon(Icon::Folder))
//!         .check(group.visible, Message::GroupChecked(id))
//!         .expanded(group.expanded, Message::GroupToggled(id))
//!         .push(Node::new("Şehirler").cells([count.into()])),
//! )
//! ```
//!
//! Düğümler her görünümde uygulamanın verisinden kurulur; açık/kapalı,
//! işaret ve seçim durumları uygulamanındır ve her değişiklik bir mesajla
//! bildirilir. Derinlik sınırsızdır. Kapalı düğümün çocukları hiç
//! kurulmayabilir: [`Node::expanded`] oku gösterir, çocuk beklemez.
//! Düğümlere [`Node::menu`] ile bağlam menüsü eklenebilir.

use iced::alignment::Horizontal;
use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{Column as Rows, Row, button, container, rule, scrollable, space};
use iced::{Center, Element, Fill, Length, Point};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::widget::context_menu::{ContextMenu, Menu};
use crate::widget::table::{self, MenuBuilder};

pub use crate::widget::table::Column;

/// Satır yüksekliği; girinti çizgileri satırlar boyunca kesintisiz uzanır.
pub const ROW_HEIGHT: f32 = 24.0;
/// Her derinlik düzeyinin girintisi; açma/kapama oku da bu genişliktedir.
pub const INDENT: f32 = 16.0;
/// Onay kutusunun kenarı.
const CHECK_SIZE: f32 = 13.0;

/// Onay kutusunun durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    Checked,
    Unchecked,
    /// Çocuklarından bazıları işaretli (ör. gruptaki katmanların bir kısmı
    /// görünür).
    Mixed,
}

impl From<bool> for Check {
    fn from(checked: bool) -> Self {
        if checked {
            Check::Checked
        } else {
            Check::Unchecked
        }
    }
}

/// Ağacın bir düğümü.
pub struct Node<'a, Message> {
    label: Fragment<'a>,
    icon: Option<Element<'a, Message>>,
    check: Option<(Check, Message)>,
    cells: Vec<Element<'a, Message>>,
    children: Vec<Node<'a, Message>>,
    expanded: Option<(bool, Message)>,
    on_press: Option<Message>,
    selected: bool,
    muted: bool,
    menu: Option<MenuBuilder<'a, Message>>,
}

impl<'a, Message: 'a> Node<'a, Message> {
    pub fn new(label: impl IntoFragment<'a>) -> Self {
        Self {
            label: label.into_fragment(),
            icon: None,
            check: None,
            cells: Vec::new(),
            children: Vec::new(),
            expanded: None,
            on_press: None,
            selected: false,
            muted: false,
            menu: None,
        }
    }

    /// Adın solundaki ikon (ör. klasör, katman rengi).
    pub fn icon(mut self, icon: impl Into<Element<'a, Message>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Onay kutusu; tıklanınca `on_toggle` üretilir.
    pub fn check(mut self, check: impl Into<Check>, on_toggle: Message) -> Self {
        self.check = Some((check.into(), on_toggle));
        self
    }

    /// İlk sütundan sonraki sütunların hücreleri.
    pub fn cells(mut self, cells: impl IntoIterator<Item = Element<'a, Message>>) -> Self {
        self.cells = cells.into_iter().collect();
        self
    }

    pub fn push(mut self, child: Node<'a, Message>) -> Self {
        self.children.push(child);
        self
    }

    pub fn extend(mut self, children: impl IntoIterator<Item = Node<'a, Message>>) -> Self {
        self.children.extend(children);
        self
    }

    /// Açılıp kapanabilen düğüm: ok gösterilir ve tıklanınca `on_toggle`
    /// üretilir. Kapalı düğümün çocukları gösterilmez. Verilmezse çocuklar
    /// her zaman açıktır.
    pub fn expanded(mut self, expanded: bool, on_toggle: Message) -> Self {
        self.expanded = Some((expanded, on_toggle));
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Seçili düğüm vurgu zeminiyle gösterilir.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sönük ad; ör. üst grubu gizli olduğu için çizilmeyen katman.
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// Düğüme sağ tıklanınca açılan bağlam menüsü.
    pub fn menu(mut self, menu: impl Fn(Point) -> Menu<Message> + 'a) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }
}

/// Ağaç görünümü.
pub struct TreeView<'a, Message> {
    columns: Vec<Column<'a, Message>>,
    nodes: Vec<Node<'a, Message>>,
    header: bool,
    height: Option<Length>,
    empty: Option<Fragment<'a>>,
}

impl<'a, Message: Clone + 'a> TreeView<'a, Message> {
    /// İlk sütun ağacı taşır; genellikle esnek genişliklidir.
    pub fn new(columns: impl IntoIterator<Item = Column<'a, Message>>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            nodes: Vec::new(),
            header: true,
            height: None,
            empty: None,
        }
    }

    pub fn push(mut self, node: Node<'a, Message>) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn extend(mut self, nodes: impl IntoIterator<Item = Node<'a, Message>>) -> Self {
        self.nodes.extend(nodes);
        self
    }

    /// Başlık satırını gösterir ya da gizler (varsayılan: gösterir).
    pub fn header(mut self, header: bool) -> Self {
        self.header = header;
        self
    }

    /// Satırların yüksekliği; verilirse satırlar kayar, başlık sabit kalır.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Hiç düğüm yokken gösterilecek açıklama.
    pub fn empty(mut self, message: impl IntoFragment<'a>) -> Self {
        self.empty = Some(message.into_fragment());
        self
    }
}

/// Düğümleri derinlik sırasıyla satırlara açar; kapalı düğümlerin çocukları
/// atlanır.
fn flatten<'a, Message: Clone + 'a>(
    nodes: Vec<Node<'a, Message>>,
    depth: usize,
    layout: &[(Length, Horizontal)],
    rows: &mut Vec<Element<'a, Message>>,
) {
    for node in nodes {
        let Node {
            label: name,
            icon: glyph,
            check,
            cells,
            children,
            expanded,
            on_press,
            selected,
            muted,
            menu,
        } = node;

        let open = expanded.as_ref().is_none_or(|(open, _)| *open);

        let mut tree = Row::with_children((0..depth).map(|_| guide()))
            .push(toggle(expanded))
            .height(ROW_HEIGHT)
            .align_y(Center);

        if let Some((check, on_toggle)) = check {
            tree = tree
                .push(check_box(check, on_toggle))
                .push(space::horizontal().width(6));
        }

        if let Some(glyph) = glyph {
            tree = tree.push(glyph).push(space::horizontal().width(6));
        }

        let name = label::body(name).wrapping(Wrapping::None);
        let name = if muted {
            name.style(style::text::muted)
        } else {
            name
        };

        tree = tree.push(container(name).width(Fill).clip(true));

        let content = button(table::line(
            layout,
            std::iter::once(tree.into()).chain(cells),
        ))
        .on_press_maybe(on_press)
        .width(Fill)
        .height(ROW_HEIGHT)
        .padding([0.0, table::PADDING_X])
        .style(style::button::table_row(selected, selected));

        rows.push(match menu {
            Some(menu) => ContextMenu::new(content, menu).into(),
            None => content.into(),
        });

        if open {
            flatten(children, depth + 1, layout, rows);
        }
    }
}

/// Bir derinlik düzeyinin girintisi ve ortasından geçen çizgi.
fn guide<'a, Message: 'a>() -> Element<'a, Message> {
    container(rule::vertical(1).style(style::field::guide))
        .width(INDENT)
        .height(ROW_HEIGHT)
        .center_x(INDENT)
        .into()
}

/// Açma/kapama oku; açılamayan düğümde boşluk.
fn toggle<'a, Message: Clone + 'a>(expanded: Option<(bool, Message)>) -> Element<'a, Message> {
    match expanded {
        Some((open, on_toggle)) => button(
            container(
                icon(if open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                })
                .size(12.0),
            )
            .center(INDENT),
        )
        .on_press(on_toggle)
        .padding(0)
        .width(INDENT)
        .height(ROW_HEIGHT)
        .style(style::button::subtle)
        .into(),
        None => space::horizontal().width(INDENT).into(),
    }
}

/// Üç durumlu onay kutusu.
fn check_box<'a, Message: Clone + 'a>(check: Check, on_toggle: Message) -> Element<'a, Message> {
    let mark: Element<'a, Message> = match check {
        Check::Checked => icon(Icon::Check).size(11.0).tone(Tone::OnAccent).into(),
        Check::Mixed => icon(Icon::Minus).size(11.0).tone(Tone::OnAccent).into(),
        Check::Unchecked => space::horizontal().width(0).into(),
    };

    button(container(mark).center(CHECK_SIZE))
        .on_press(on_toggle)
        .padding(0)
        .width(CHECK_SIZE)
        .height(CHECK_SIZE)
        .style(style::button::check(check != Check::Unchecked))
        .into()
}

impl<'a, Message: Clone + 'a> From<TreeView<'a, Message>> for Element<'a, Message> {
    fn from(tree: TreeView<'a, Message>) -> Self {
        let layout = table::layout(&tree.columns);

        let mut rows = Vec::new();
        flatten(tree.nodes, 0, &layout, &mut rows);

        let body: Element<'a, Message> = match (rows.is_empty(), tree.empty) {
            (true, Some(message)) => container(label::muted(message))
                .padding([8.0, table::PADDING_X])
                .width(Fill)
                .into(),
            _ => Rows::with_children(rows).width(Fill).into(),
        };

        let body = match tree.height {
            Some(height) => scrollable(body)
                .direction(style::field::thin_scrollbar())
                .width(Fill)
                .height(height)
                .into(),
            None => body,
        };

        let mut content = Rows::new().width(Fill);

        if tree.header {
            content = content.push(table::header(tree.columns, &layout));
        }

        content.push(body).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_come_from_booleans() {
        assert_eq!(Check::from(true), Check::Checked);
        assert_eq!(Check::from(false), Check::Unchecked);
    }

    #[test]
    fn collapsed_nodes_hide_their_children() {
        let layout = [(Length::Fill, Horizontal::Left)];
        let tree = vec![
            Node::<()>::new("Açık grup").expanded(true, ()).extend([
                Node::new("Katman"),
                Node::new("Kapalı grup")
                    .expanded(false, ())
                    .push(Node::new("Görünmez")),
            ]),
            Node::new("Sabit grup").push(Node::new("Her zaman görünür")),
        ];

        let mut rows = Vec::new();
        flatten(tree, 0, &layout, &mut rows);

        assert_eq!(rows.len(), 5);
    }
}
