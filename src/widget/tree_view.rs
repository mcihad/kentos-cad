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
//!
//! - **Sürükleyerek taşıma.** Kimliği ([`Node::id`]) olan düğümler
//!   sürüklenir; bırakılınca [`TreeView::on_move`] kaynağı, hedefi ve yeri
//!   ([`Place`]: önüne, ardına, içine) bildirir. Bir düğüm kendi altına
//!   taşınamaz. Esc sürüklemeyi bırakır.
//! - **Yerinde adlandırma.** [`Node::editor`] adın yerine düzenleyiciyi
//!   koyar (ör. F2'ye basınca [`rename`] ile kurulan metin kutusu). Enter
//!   ve kutunun dışına tıklamak adı kaydeder, Esc vazgeçer; düzenlenen
//!   satır sürüklenmez, kutuda fareyle metin seçilebilir.
//! - **Satır düğmeleri.** [`Node::toggle`] adın sağına göz, kilit ya da
//!   seçilebilirlik düğmesi ekler ([`Toggle`]).
//! - **Sanal ağaç.** Çok düğümlü ağaçlar [`TreeView::virtualized`] ile
//!   düzleştirilmiş satırlardan kurulur; yalnızca görünen satırlar kurulur.

use iced::advanced::layout::{self, Layout, Node as LayoutNode};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Horizontal;
use iced::keyboard::{self, key};
use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{
    Column as Rows, Row, button, container, rule, scrollable, space, text_input, tooltip,
};
use iced::{
    Background, Border, Center, Element, Event, Fill, Length, Point, Rectangle, Renderer, Size,
    Theme, Vector, mouse,
};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{ContextMenu, Menu};
use crate::widget::table::{self, MenuBuilder};
use crate::widget::virtual_list::VirtualList;
use crate::widget::{Tip, tip};

pub use crate::widget::table::Column;

/// Satır yüksekliği, 12 piksellik gövde metninde; girinti çizgileri satırlar
/// boyunca kesintisiz uzanır. Yazı boyutuyla büyür ([`row_height`]).
pub const ROW_HEIGHT: f32 = 24.0;
/// Her derinlik düzeyinin girintisi; açma/kapama oku da bu genişliktedir.
pub const INDENT: f32 = 16.0;
/// Onay kutusunun kenarı.
const CHECK_SIZE: f32 = 13.0;

/// Geçerli yazı boyutundaki satır yüksekliği.
pub fn row_height() -> f32 {
    typography::scaled(ROW_HEIGHT)
}

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

/// Düğümün adının sağındaki açık/kapalı düğme: görünürlük, kilit ya da
/// seçilebilirlik.
pub struct Toggle<Message> {
    on: bool,
    glyphs: (Icon, Icon),
    tips: (&'static str, &'static str),
    message: Message,
}

impl<Message> Toggle<Message> {
    /// Kendi ikonlarıyla düğme; kapalıyken ikon sönüktür.
    pub fn new(
        on: bool,
        glyphs: (Icon, Icon),
        tips: (&'static str, &'static str),
        message: Message,
    ) -> Self {
        Self {
            on,
            glyphs,
            tips,
            message,
        }
    }

    /// Görünürlük: göz.
    pub fn visible(visible: bool, message: Message) -> Self {
        Self::new(
            visible,
            (Icon::Eye, Icon::EyeOff),
            (
                "Görünür: gizlemek için tıklayın",
                "Gizli: göstermek için tıklayın",
            ),
            message,
        )
    }

    /// Kilit: kilitli öğeler düzenlenmez.
    pub fn locked(locked: bool, message: Message) -> Self {
        Self::new(
            locked,
            (Icon::Lock, Icon::Unlock),
            ("Kilitli: açmak için tıklayın", "Kilitlemek için tıklayın"),
            message,
        )
    }

    /// Seçilebilirlik: seçilemeyen öğelere tıklanınca seçilmez.
    pub fn selectable(selectable: bool, message: Message) -> Self {
        Self::new(
            selectable,
            (Icon::Select, Icon::NoSelect),
            ("Seçilebilir", "Seçilemez: haritada tıklanınca seçilmez"),
            message,
        )
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
    id: Option<usize>,
    folder: bool,
    editor: Option<Element<'a, Message>>,
    toggles: Vec<Toggle<Message>>,
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
            id: None,
            folder: false,
            editor: None,
            toggles: Vec::new(),
        }
    }

    /// Klasör: sürüklenen düğümler içine bırakılabilir ([`Place::Into`]).
    pub fn folder(mut self) -> Self {
        self.folder = true;
        self
    }

    /// Sürükleyerek taşıma kimliği; [`TreeView::on_move`] bununla bildirir.
    pub fn id(mut self, id: usize) -> Self {
        self.id = Some(id);
        self
    }

    /// Adın yerine düzenleyici (ör. [`rename`] ile kurulan metin kutusu).
    pub fn editor(mut self, editor: impl Into<Element<'a, Message>>) -> Self {
        self.editor = Some(editor.into());
        self
    }

    /// Adın sağına açık/kapalı düğmesi ekler.
    pub fn toggle(mut self, toggle: Toggle<Message>) -> Self {
        self.toggles.push(toggle);
        self
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

/// Sürüklenen düğümün bırakıldığı yer, hedef düğüme göre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// Hedefin önüne, kardeşi olarak.
    Before,
    /// Hedefin ardına, kardeşi olarak.
    After,
    /// Hedefin (grubun) içine, son çocuğu olarak.
    Into,
}

/// Düğümü taşıma mesajı: kaynağın ve hedefin kimliği, yer.
type OnMove<'a, Message> = Box<dyn Fn(usize, usize, Place) -> Message + 'a>;

/// Sanal ağacın satırı: derinliği ve düğümü (çocukları yok sayılır).
type LazyRow<'a, Message> = Box<dyn Fn(usize) -> (usize, Node<'a, Message>) + 'a>;

/// Ağaç görünümü.
pub struct TreeView<'a, Message> {
    columns: Vec<Column<'a, Message>>,
    nodes: Vec<Node<'a, Message>>,
    header: bool,
    height: Option<Length>,
    empty: Option<Fragment<'a>>,
    on_move: Option<OnMove<'a, Message>>,
    lazy: Option<(usize, LazyRow<'a, Message>, Option<usize>)>,
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
            on_move: None,
            lazy: None,
        }
    }

    /// Kimlikli düğümler sürüklenerek taşınır; bırakılınca kaynağın ve
    /// hedefin kimliği ve yer bildirilir. Uygulama taşımayı reddedebilir
    /// (ör. grupsuz katmanı başka türden gruba).
    pub fn on_move(mut self, on_move: impl Fn(usize, usize, Place) -> Message + 'a) -> Self {
        self.on_move = Some(Box::new(on_move));
        self
    }

    /// Sanal ağaç: `count` görünür satırdan (açık düğümlerin düzleştirilmiş
    /// sırası) yalnızca görünenler `view` ile kurulur; `view` satırın
    /// derinliğini ve düğümünü verir. Sürükleyerek taşıma sanal ağaçta
    /// yoktur.
    pub fn virtualized(
        mut self,
        count: usize,
        view: impl Fn(usize) -> (usize, Node<'a, Message>) + 'a,
    ) -> Self {
        self.lazy = Some((count, Box::new(view), None));
        self
    }

    /// Sanal ağaçta satırı görünür yapar.
    pub fn reveal(mut self, index: Option<usize>) -> Self {
        if let Some((_, _, reveal)) = &mut self.lazy {
            *reveal = index;
        }
        self
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

/// Taşımada satırın bilgisi: kimliği, derinliği, grup olup olmadığı ve
/// açıklığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    id: Option<usize>,
    depth: usize,
    group: bool,
    open: bool,
    /// Satır yerinde adlandırılıyor: basılınca sürükleme başlamaz.
    editing: bool,
}

/// Düğümleri derinlik sırasıyla satırlara açar; kapalı düğümlerin çocukları
/// atlanır.
fn flatten<'a, Message: Clone + 'a>(
    nodes: Vec<Node<'a, Message>>,
    depth: usize,
    layout: &[(Length, Horizontal)],
    rows: &mut Vec<Element<'a, Message>>,
    slots: &mut Vec<Slot>,
) {
    for mut node in nodes {
        let children = std::mem::take(&mut node.children);
        let open = node.expanded.as_ref().is_none_or(|(open, _)| *open);

        slots.push(Slot {
            id: node.id,
            depth,
            group: node.folder,
            open,
            editing: node.editor.is_some(),
        });
        rows.push(node_row(node, depth, layout));

        if open {
            flatten(children, depth + 1, layout, rows, slots);
        }
    }
}

/// Düğümün satırı: girinti, ok, onay kutusu, ikon, ad, düğmeler ve
/// hücreler.
fn node_row<'a, Message: Clone + 'a>(
    node: Node<'a, Message>,
    depth: usize,
    layout: &[(Length, Horizontal)],
) -> Element<'a, Message> {
    let Node {
        label: name,
        icon: glyph,
        check,
        cells,
        children: _,
        expanded,
        on_press,
        selected,
        muted,
        menu,
        id: _,
        folder: _,
        editor,
        toggles,
    } = node;

    let mut tree = Row::with_children((0..depth).map(|_| guide()))
        .push(toggle(expanded))
        .height(row_height())
        .align_y(Center);

    if let Some((check, on_toggle)) = check {
        tree = tree
            .push(check_box(check, on_toggle))
            .push(space::horizontal().width(6));
    }

    if let Some(glyph) = glyph {
        tree = tree.push(glyph).push(space::horizontal().width(6));
    }

    tree = match editor {
        Some(editor) => tree.push(container(editor).width(Fill)),
        None => {
            let name = label::body(name).wrapping(Wrapping::None);
            let name = if muted {
                name.style(style::text::muted)
            } else {
                name
            };

            tree.push(container(name).width(Fill).clip(true))
        }
    };

    for toggle in toggles {
        tree = tree.push(row_toggle(toggle));
    }

    let content = button(table::line(
        layout,
        std::iter::once(tree.into()).chain(cells),
    ))
    .on_press_maybe(on_press)
    .width(Fill)
    .height(row_height())
    .padding([0.0, table::PADDING_X])
    .style(style::button::table_row(selected, selected));

    match menu {
        Some(menu) => ContextMenu::new(content, menu).into(),
        None => content.into(),
    }
}

/// Satırdaki açık/kapalı düğme; kapalıyken ikon sönüktür.
fn row_toggle<'a, Message: Clone + 'a>(toggle: Toggle<Message>) -> Element<'a, Message> {
    let (glyph, description) = if toggle.on {
        (toggle.glyphs.0, toggle.tips.0)
    } else {
        (toggle.glyphs.1, toggle.tips.1)
    };

    tip(
        button(icon(glyph).size(13.0).tone(if toggle.on {
            Tone::Inherit
        } else {
            Tone::Muted
        }))
        .on_press(toggle.message)
        .padding([2, 3])
        .style(style::button::subtle),
        Tip::new(description),
        tooltip::Position::Left,
    )
}

/// Yerinde adlandırma kutusu: satıra oturur, odaklanınca vurgu kenarı alır.
/// Enter ve kutunun dışına tıklamak adı kaydeder (`on_submit`); Esc
/// vazgeçer (`on_cancel`). Uygulama kutuyu [`RENAME`] kimliğiyle odaklar
/// (`iced::widget::operation::focus`).
pub fn rename<'a, Message: Clone + 'a>(
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
    on_submit: Message,
    on_cancel: Message,
) -> Element<'a, Message> {
    Element::new(Rename {
        input: text_input("", value)
            .id(RENAME)
            .on_input(on_input)
            .on_submit(on_submit.clone())
            .font(typography::ui())
            .size(typography::body())
            .padding([1, 4])
            .style(style::field::input)
            .into(),
        on_submit,
        on_cancel,
    })
}

/// Yerinde adlandırma kutusunun kimliği.
pub const RENAME: &str = "kentos-tree-rename";

type Paragraph = <Renderer as iced::advanced::text::Renderer>::Paragraph;

/// Adlandırma kutusunu saran bileşen. Metin kutusu Esc'yi kendisi yutar ve
/// odağı bırakırken haber vermez; bu yüzden Esc ve dışarı tıklama kutuya
/// ulaşmadan burada yakalanır.
struct Rename<'a, Message> {
    input: Element<'a, Message>,
    on_submit: Message,
    on_cancel: Message,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Rename<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.input)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.input));
    }

    fn size(&self) -> Size<Length> {
        self.input.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> LayoutNode {
        self.input
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let focused = tree.children[0]
            .state
            .downcast_ref::<text_input::State<Paragraph>>()
            .is_focused();

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) if focused => {
                shell.publish(self.on_cancel.clone());
                shell.capture_event();
                return;
            }
            // Başka bir yere tıklamak adı kaydeder; tıklama yerine ulaşır.
            Event::Mouse(mouse::Event::ButtonPressed(_)) if !cursor.is_over(layout.bounds()) => {
                shell.publish(self.on_submit.clone());
            }
            _ => {}
        }

        self.input.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.input.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.input.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.input
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }
}

/// Bir derinlik düzeyinin girintisi ve ortasından geçen çizgi.
fn guide<'a, Message: 'a>() -> Element<'a, Message> {
    container(rule::vertical(1).style(style::field::guide))
        .width(INDENT)
        .height(row_height())
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
        .height(row_height())
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

        // Sanal ağaç: yalnızca görünen satırlar kurulur, liste kendisi kayar.
        if let Some((count, view, reveal)) = tree.lazy {
            let row_layout = layout.clone();
            let body: Element<'a, Message> = match (count, tree.empty) {
                (0, Some(message)) => container(label::muted(message))
                    .padding([8.0, table::PADDING_X])
                    .width(Fill)
                    .into(),
                _ => VirtualList::new(count, row_height(), move |index| {
                    let (depth, node) = view(index);
                    node_row(node, depth, &row_layout)
                })
                .reveal(reveal)
                .height(tree.height.unwrap_or(Length::Fill))
                .into(),
            };

            let mut content = Rows::new().width(Fill);

            if tree.header {
                content = content.push(table::header(tree.columns, &layout));
            }

            return content.push(body).into();
        }

        let mut rows = Vec::new();
        let mut slots = Vec::new();
        flatten(tree.nodes, 0, &layout, &mut rows, &mut slots);

        let body: Element<'a, Message> = match (rows.is_empty(), tree.empty) {
            (true, Some(message)) => container(label::muted(message))
                .padding([8.0, table::PADDING_X])
                .width(Fill)
                .into(),
            _ => {
                let rows = Rows::with_children(rows).width(Fill);

                match tree.on_move {
                    Some(on_move) => Reorder {
                        content: rows.into(),
                        slots,
                        on_move,
                    }
                    .into(),
                    None => rows.into(),
                }
            }
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

/// Sürüklemenin başladığı uzaklık.
const DRAG: f32 = 4.0;

/// Sürükleyerek taşınabilen satırlar: satırların sütunu, her satırın
/// bilgisi ve taşıma mesajı.
struct Reorder<'a, Message> {
    content: Element<'a, Message>,
    slots: Vec<Slot>,
    on_move: OnMove<'a, Message>,
}

/// Sürüklenen `source` satırı, satır yüksekliği `height` olan listede `y`
/// (listenin üstüne göre) yüksekliğine bırakılırsa hedef satır ve yer.
/// Kimliksiz satıra, kaynağın kendisine ve altına bırakılamaz.
fn target(slots: &[Slot], source: usize, height: f32, y: f32) -> Option<(usize, Place)> {
    let row = (y / height).floor();

    if row < 0.0 {
        return None;
    }

    let row = row as usize;
    let slot = slots.get(row)?;
    let origin = slots.get(source)?;

    slot.id?;

    // Kaynağın altındaki satırlar: derinliği kaynaktan büyük olanlar.
    let below = slots[source + 1..]
        .iter()
        .take_while(|other| other.depth > origin.depth)
        .count();

    if row >= source && row <= source + below {
        return None;
    }

    let within = (y - row as f32 * height) / height;

    let place = if slot.group {
        match within {
            within if within < 0.3 => Place::Before,
            within if within > 0.7 && !slot.open => Place::After,
            _ => Place::Into,
        }
    } else if within < 0.5 {
        Place::Before
    } else {
        Place::After
    };

    Some((row, place))
}

#[derive(Debug, Default)]
struct Drag {
    /// Basılan satır ve basıldığı yer.
    press: Option<(usize, Point)>,
    /// Sürüklenen satır ve bırakılacağı yer.
    dragging: Option<usize>,
    target: Option<(usize, Place)>,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Reorder<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Drag>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Drag::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> LayoutNode {
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);

        LayoutNode::with_children(content.size(), vec![content])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let height = row_height();
        let state = tree.state.downcast_mut::<Drag>();
        let Some(content) = layout.children().next() else {
            return;
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(point) = cursor.position_over(bounds) {
                    let row = ((point.y - bounds.y) / height).floor() as usize;

                    state.press = self
                        .slots
                        .get(row)
                        .filter(|slot| slot.id.is_some() && !slot.editing)
                        .map(|_| (row, point));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some((row, origin)) = state.press
                    && state.dragging.is_none()
                    && position.distance(origin) > DRAG
                {
                    state.dragging = Some(row);
                }

                if let Some(source) = state.dragging {
                    state.target = target(&self.slots, source, height, position.y - bounds.y);
                    shell.request_redraw();
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.press = None;

                if let Some(source) = state.dragging.take() {
                    if let (Some((row, place)), Some(from)) =
                        (state.target.take(), self.slots[source].id)
                        && let Some(to) = self.slots[row].id
                    {
                        shell.publish((self.on_move)(from, to, place));
                    }

                    // Satır düğmeleri basılı kalmasın; tıklama sayılmaz.
                    self.content.as_widget_mut().update(
                        &mut tree.children[0],
                        event,
                        content,
                        cursor.levitate(),
                        renderer,
                        clipboard,
                        shell,
                        viewport,
                    );
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) if state.dragging.is_some() => {
                *state = Drag::default();
                shell.capture_event();
                shell.request_redraw();
                return;
            }
            _ => {}
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<Drag>().dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }

        layout
            .children()
            .next()
            .map_or(mouse::Interaction::None, |content| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    content,
                    cursor,
                    viewport,
                    renderer,
                )
            })
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<Drag>();
        let bounds = layout.bounds();
        let height = row_height();

        if let Some(content) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                content,
                if state.dragging.is_some() {
                    cursor.levitate()
                } else {
                    cursor
                },
                viewport,
            );
        }

        let Some(source) = state.dragging else {
            return;
        };
        let t = Tokens::of(theme);
        let row_rect = |row: usize| {
            Rectangle::new(
                Point::new(bounds.x, bounds.y + row as f32 * height),
                Size::new(bounds.width, height),
            )
        };

        // Sürüklenen satır hafifçe işaretlenir; hedef çizgiyle ya da
        // çerçeveyle gösterilir.
        renderer.fill_quad(
            Quad {
                bounds: row_rect(source),
                ..Quad::default()
            },
            Background::Color(t.accent.scale_alpha(0.08)),
        );

        let Some((row, place)) = state.target else {
            return;
        };
        let rect = row_rect(row);
        let depth = self.slots.get(row).map_or(0, |slot| slot.depth);
        let indent = table::PADDING_X + depth as f32 * INDENT;

        match place {
            Place::Into => renderer.fill_quad(
                Quad {
                    bounds: rect,
                    border: Border {
                        color: t.accent,
                        width: 1.0,
                        radius: 2.0.into(),
                    },
                    ..Quad::default()
                },
                Background::Color(t.accent.scale_alpha(0.14)),
            ),
            Place::Before | Place::After => {
                let y = if place == Place::Before {
                    rect.y
                } else {
                    rect.y + rect.height
                };

                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(rect.x + indent, y - 1.0),
                            Size::new(rect.width - indent, 2.0),
                        ),
                        border: Border {
                            radius: 1.0.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    Background::Color(t.accent),
                );
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(rect.x + indent - 3.0, y - 3.0),
                            Size::new(6.0, 6.0),
                        ),
                        border: Border {
                            color: t.accent,
                            width: 2.0,
                            radius: 3.0.into(),
                        },
                        ..Quad::default()
                    },
                    Background::Color(t.surface),
                );
            }
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(content) = layout.children().next() {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                content,
                renderer,
                operation,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let content = layout.children().next()?;

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            content,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<Reorder<'a, Message>> for Element<'a, Message> {
    fn from(reorder: Reorder<'a, Message>) -> Self {
        Element::new(reorder)
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
                    .folder()
                    .expanded(false, ())
                    .push(Node::new("Görünmez")),
            ]),
            Node::new("Sabit grup").push(Node::new("Her zaman görünür")),
        ];

        let mut rows = Vec::new();
        let mut slots = Vec::new();
        flatten(tree, 0, &layout, &mut rows, &mut slots);

        assert_eq!(rows.len(), 5);
        assert_eq!(
            slots.iter().map(|slot| slot.depth).collect::<Vec<_>>(),
            [0, 1, 1, 0, 1]
        );
        assert!(slots[2].group && !slots[2].open);
    }

    fn slot(id: usize, depth: usize, group: bool) -> Slot {
        Slot {
            id: Some(id),
            depth,
            group,
            open: true,
            editing: false,
        }
    }

    #[test]
    fn drops_land_before_after_or_into_rows() {
        // 0 grup, 1 onun çocuğu, 2 grup, 3 onun çocuğu; satırlar 20 piksel.
        let slots = [
            slot(10, 0, true),
            slot(11, 1, false),
            slot(12, 0, true),
            slot(13, 1, false),
        ];

        // Yaprak satır: üst yarısı önüne, alt yarısı ardına.
        assert_eq!(target(&slots, 3, 20.0, 22.0), Some((1, Place::Before)));
        assert_eq!(target(&slots, 3, 20.0, 38.0), Some((1, Place::After)));

        // Grup satırı: kenarları önüne/ardına, ortası içine.
        assert_eq!(target(&slots, 1, 20.0, 42.0), Some((2, Place::Before)));
        assert_eq!(target(&slots, 1, 20.0, 50.0), Some((2, Place::Into)));

        // Açık grubun alt kenarı da içine.
        assert_eq!(target(&slots, 1, 20.0, 58.0), Some((2, Place::Into)));

        // Düğüm kendine ya da altına taşınamaz.
        assert_eq!(target(&slots, 2, 20.0, 50.0), None);
        assert_eq!(target(&slots, 2, 20.0, 70.0), None);
        assert_eq!(target(&slots, 0, 20.0, 30.0), None);
    }
}

/// Gerçek olaylarla: düğümü başka düğümün ardına ve grubun içine taşıma,
/// Esc ile vazgeçme.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::keyboard::key::Named;
    use iced::{Element, Fill, Point, Size};

    use super::{Column, Node, Place, TreeView, rename, row_height};
    use crate::snapshot::{Input, Snapshot};

    type Moves = Vec<(usize, usize, Place)>;

    fn view(_moves: &Moves) -> Element<'_, (usize, usize, Place)> {
        TreeView::new([Column::new("Ad").width(Fill)])
            .header(false)
            .extend([
                Node::new("Çizimler").id(0),
                Node::new("Yollar").id(1),
                Node::new("Ulaşım")
                    .id(2)
                    .folder()
                    .expanded(true, (9, 9, Place::Into)),
            ])
            .on_move(|from, to, place| (from, to, place))
            .into()
    }

    #[test]
    fn nodes_are_dragged_after_and_into_others() {
        let mut snapshot = Snapshot::new(Size::new(300.0, 200.0)).expect("çizici kurulamadı");
        let mut moves: Moves = Vec::new();
        let mut update = |moves: &mut Moves, next| moves.push(next);
        let mut input = |moves: &mut Moves, input| snapshot.input(moves, view, &mut update, input);

        let row = row_height();
        let at = |index: f32, within: f32| Point::new(120.0, (index + within) * row);

        // Çizimler, Yollar'ın alt yarısına: ardına.
        input(&mut moves, Input::Drag(at(0.0, 0.5), at(1.0, 0.8)));
        assert_eq!(moves, [(0, 1, Place::After)]);

        // Yollar, Ulaşım grubunun ortasına: içine.
        input(&mut moves, Input::Drag(at(1.0, 0.5), at(2.0, 0.5)));
        assert_eq!(moves[1], (1, 2, Place::Into));

        // Esc sürüklemeyi bırakır.
        input(&mut moves, Input::Press(at(0.0, 0.5)));
        input(&mut moves, Input::Move(at(2.0, 0.5)));
        input(&mut moves, Input::Key(Named::Escape));
        input(&mut moves, Input::Release(at(2.0, 0.5)));
        assert_eq!(moves.len(), 2);
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Edit {
        Input(String),
        Submitted,
        Cancelled,
        Moved,
    }

    type Edits = Vec<Edit>;

    fn renaming(_edits: &Edits) -> Element<'_, Edit> {
        TreeView::new([Column::new("Ad").width(Fill)])
            .header(false)
            .extend([
                Node::new("Çizimler").id(0).editor(rename(
                    "Çizimler",
                    Edit::Input,
                    Edit::Submitted,
                    Edit::Cancelled,
                )),
                Node::new("Yollar").id(1),
            ])
            .on_move(|_, _, _| Edit::Moved)
            .into()
    }

    #[test]
    fn the_rename_box_cancels_on_escape_and_submits_on_outside_clicks() {
        let mut snapshot = Snapshot::new(Size::new(300.0, 200.0)).expect("çizici kurulamadı");
        let mut edits: Edits = Vec::new();
        let mut update = |edits: &mut Edits, next| edits.push(next);
        let mut input =
            |edits: &mut Edits, input| snapshot.input(edits, renaming, &mut update, input);

        let row = row_height();

        // Kutuda sürüklemek metin seçer; satır taşınmaz.
        input(
            &mut edits,
            Input::Drag(Point::new(120.0, row * 0.5), Point::new(160.0, row * 1.8)),
        );
        assert!(edits.is_empty());

        input(&mut edits, Input::Type("x".to_owned()));
        assert!(matches!(edits.last(), Some(Edit::Input(_))));

        // Metin kutusu Esc'yi yutardı; kutu vazgeçmeyi bildirir.
        input(&mut edits, Input::Key(Named::Escape));
        assert_eq!(edits.last(), Some(&Edit::Cancelled));

        // Başka bir satıra tıklamak adı kaydeder.
        input(&mut edits, Input::Click(Point::new(120.0, row * 1.5)));
        assert_eq!(edits.last(), Some(&Edit::Submitted));
        assert!(!edits.contains(&Edit::Moved));
    }
}
