//! Form düzeni: etiket sütunu, alanlar, bölüm başlıkları, yardım ve hata
//! satırları.
//!
//! ```text
//! Genel ─────────────────────────────────────────
//!   Ad *         [Kadıköy imar planı          ]
//!                Paftanın başlığında görünür.
//!   Ölçek        [1:1000                     ⌄]
//! Konum ─────────────────────────────────────────
//!   Enlem        [91                          ]
//!                ⚠ −90 ile 90 arasında olmalı.
//! ```
//!
//! Etiketler aynı genişlikte bir sütunda, alanın ilk satırına hizalı durur.
//! Zorunlu alanın adının yanında yıldız, alanın altında yardım, uyarı ya da
//! hata yazar; hata uyarının, uyarı yardımın yerini alır.
//!
//! Dar yerde (dar bir yan panel, büyük yazı) alanın yanında en az [`LEAST`]
//! genişlik kalmıyorsa etiket alanın üstüne geçer ve alan bütün genişliği
//! alır; adsız satır (onay kutusu, düğmeler) girintisiz durur. Geniş yerde
//! görünüm aynıdır.
//!
//! ```ignore
//! Form::new()
//!     .section("Genel")
//!     .field("Ad", text_input("", &self.name).on_input(Message::Named))
//!     .required()
//!     .help("Paftanın başlığında görünür.")
//!     .field("Enlem", latitude)
//!     .error(self.latitude_error.as_deref())
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, column, container, row, rule, space};
use iced::{
    Alignment, Element, Event, Fill, Length, Padding, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;

/// Etiket sütununun varsayılan genişliği (12 piksellik gövde metnine göre).
const LABEL: f32 = 110.0;

/// Etiketin yanında alana kalması gereken en az genişlik (12 piksellik gövde
/// metnine göre); kalmıyorsa etiket alanın üstüne geçer.
const LEAST: f32 = 120.0;

/// Etiket sütunuyla alan arası (piksel; yazı boyuyla büyümez, eskisi gibi).
const GAP: f32 = 12.0;

/// Form.
pub struct Form<'a, Message> {
    rows: Vec<Line<'a, Message>>,
    label_width: f32,
}

enum Line<'a, Message> {
    Section(Fragment<'a>),
    /// A heading the user folds, made by [`Form::foldable_section`]: a button the whole line wide.
    Fold(Element<'a, Message>),
    Field {
        label: Fragment<'a>,
        control: Element<'a, Message>,
        required: bool,
        help: Option<Fragment<'a>>,
        warning: Option<Fragment<'a>>,
        error: Option<Fragment<'a>>,
    },
    Full(Element<'a, Message>),
    /// The whole width, the label column included: a table of several columns.
    Wide(Element<'a, Message>),
}

impl<'a, Message: 'a> Default for Form<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> Form<'a, Message> {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            label_width: LABEL,
        }
    }

    /// Etiket sütununun genişliği (12 piksellik gövde metnine göre).
    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }

    /// Bölüm başlığı: adı ve satırın sonuna kadar uzanan çizgi.
    pub fn section(mut self, title: impl IntoFragment<'a>) -> Self {
        self.rows.push(Line::Section(title.into_fragment()));
        self
    }

    /// Adlı alan.
    pub fn field(
        mut self,
        label: impl IntoFragment<'a>,
        control: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.rows.push(Line::Field {
            label: label.into_fragment(),
            control: control.into(),
            required: false,
            help: None,
            warning: None,
            error: None,
        });
        self
    }

    /// Etiket sütununa hizalı, adsız satır (ör. onay kutusu, düğmeler).
    pub fn row(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.rows.push(Line::Full(content.into()));
        self
    }

    /// Etiket sütunu dahil bütün genişlikte satır: birkaç sütunlu bir tablo (bileşenler, tabakalar, katsayılar).
    pub fn wide(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.rows.push(Line::Wide(content.into()));
        self
    }

    /// Son alan zorunludur: adının yanında yıldız.
    pub fn required(mut self) -> Self {
        if let Some(Line::Field { required, .. }) = self.rows.last_mut() {
            *required = true;
        }
        self
    }

    /// Son alanın altındaki yardım metni.
    pub fn help(mut self, text: impl IntoFragment<'a>) -> Self {
        if let Some(Line::Field { help, .. }) = self.rows.last_mut() {
            *help = Some(text.into_fragment());
        }
        self
    }

    /// Son alanın uyarısı: kaydı durdurmayan bir dikkat; varsa yardımın yerinde
    /// sarı yazar, hata onun yerini alır.
    pub fn warning(mut self, text: Option<impl IntoFragment<'a>>) -> Self {
        if let Some(Line::Field { warning, .. }) = self.rows.last_mut() {
            *warning = text.map(IntoFragment::into_fragment);
        }
        self
    }

    /// Son alanın hatası; varsa yardımın yerinde kırmızı yazar.
    pub fn error(mut self, text: Option<impl IntoFragment<'a>>) -> Self {
        if let Some(Line::Field { error, .. }) = self.rows.last_mut() {
            *error = text.map(IntoFragment::into_fragment);
        }
        self
    }
}

impl<'a, Message: Clone + 'a> Form<'a, Message> {
    /// Katlanır bölüm başlığı: adı, açıkken aşağı, katlıyken sağa bakan ok ve satırın sonuna kadar çizgi; satırın
    /// tamamı basılır, basınca `toggle` gider. Katlıyken bölümün satırlarını koymamak çağıranın işidir.
    pub fn foldable_section(
        mut self,
        title: impl IntoFragment<'a>,
        open: bool,
        toggle: Message,
    ) -> Self {
        let heading = iced::widget::button(
            row![
                icon(if open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                })
                .size(12.0)
                .tone(Tone::Muted),
                label::strong(title),
                container(rule::horizontal(1).style(style::field::hairline)).width(Fill)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(Padding::ZERO)
        .width(Fill)
        .style(style::button::ghost)
        .on_press(toggle);
        self.rows.push(Line::Fold(heading.into()));
        self
    }
}

impl<'a, Message: 'a> From<Form<'a, Message>> for Element<'a, Message> {
    fn from(form: Form<'a, Message>) -> Self {
        let label_width = typography::scaled(form.label_width);
        let mut column = Column::new().spacing(10);

        for (index, line) in form.rows.into_iter().enumerate() {
            let line: Element<'a, Message> = match line {
                Line::Section(title) => container(
                    row![
                        label::strong(title),
                        container(rule::horizontal(1).style(style::field::hairline)).width(Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding(Padding {
                    top: if index == 0 { 0.0 } else { 8.0 },
                    ..Padding::ZERO
                })
                .into(),
                Line::Fold(heading) => container(heading)
                    .padding(Padding {
                        top: if index == 0 { 0.0 } else { 8.0 },
                        ..Padding::ZERO
                    })
                    .into(),
                Line::Field {
                    label: name,
                    control,
                    required,
                    help,
                    warning,
                    error,
                } => {
                    let mut name_row = row![label::body(name)].spacing(3);

                    if required {
                        name_row = name_row.push(label::body("*").style(style::text::danger));
                    }

                    let mut body = column![control].spacing(4);

                    body = match (error, warning, help) {
                        (Some(error), _, _) => body.push(
                            row![
                                icon(Icon::Warning).size(12.0).tone(Tone::Danger),
                                label::caption(error).style(style::text::danger),
                            ]
                            .spacing(5)
                            .align_y(Alignment::Center),
                        ),
                        (None, Some(warning), _) => body.push(
                            row![
                                icon(Icon::Warning).size(12.0).tone(Tone::Warning),
                                label::caption(warning).style(style::text::warning),
                            ]
                            .spacing(5)
                            .align_y(Alignment::Center),
                        ),
                        (None, None, Some(help)) => body.push(label::caption(help)),
                        (None, None, None) => body,
                    };

                    // Ad, alanın ilk satırıyla aynı yükseklikte başlar.
                    Pair::new(
                        container(name_row).padding(Padding {
                            top: 4.0,
                            ..Padding::ZERO
                        }),
                        body.width(Fill),
                        label_width,
                    )
                    .into()
                }
                Line::Full(content) => Pair::new(
                    space::horizontal().width(0),
                    container(content).width(Fill),
                    label_width,
                )
                .into(),
                Line::Wide(content) => container(content).width(Fill).into(),
            };

            column = column.push(line);
        }

        column.into()
    }
}

/// A row's name and its control (with its help): side by side when the control has [`LEAST`] beside the name column,
/// else the name above and the control the whole width below. A form in a narrow dock with large text kept its name
/// column and left the controls a third of the width; a check box's words ran past the edge (real window, 15 November
/// 2026: 1100×650, 16 px text). An unnamed row's name is empty: indented beside, flush below.
struct Pair<'a, Message> {
    /// The name, the control.
    children: [Element<'a, Message>; 2],
    label_width: f32,
}

impl<'a, Message: 'a> Pair<'a, Message> {
    fn new(
        name: impl Into<Element<'a, Message>>,
        control: impl Into<Element<'a, Message>>,
        label_width: f32,
    ) -> Self {
        Self {
            children: [name.into(), control.into()],
            label_width,
        }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Pair<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let limits = limits.width(Length::Fill).height(Length::Shrink);
        let width = limits.max().width;
        let gap = GAP;
        let beside = width >= self.label_width + gap + typography::scaled(LEAST);
        let room = |w: f32| layout::Limits::new(Size::ZERO, Size::new(w.max(0.0), f32::INFINITY));
        let [name, control] = &mut self.children;
        let (name_tree, control_tree) = tree.children.split_at_mut(1);
        let (name_width, control_x) = if beside {
            (self.label_width, self.label_width + gap)
        } else {
            (width, 0.0)
        };
        let name_node = name
            .as_widget_mut()
            .layout(&mut name_tree[0], renderer, &room(name_width));
        let mut control_node = control.as_widget_mut().layout(
            &mut control_tree[0],
            renderer,
            &room(width - control_x),
        );
        let named = name_node.size().height > 0.0 && name_node.size().width > 0.0;
        let below = if beside || !named {
            0.0
        } else {
            name_node.size().height + 4.0
        };
        control_node.move_to_mut((control_x, below));
        let height = if beside {
            name_node.size().height.max(control_node.size().height)
        } else {
            below + control_node.size().height
        };
        let size = limits.resolve(Length::Fill, Length::Shrink, Size::new(width, height));
        Node::with_children(size, vec![name_node, control_node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
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
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        for ((child, tree), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
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
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<Pair<'a, Message>> for Element<'a, Message> {
    fn from(pair: Pair<'a, Message>) -> Self {
        Element::new(pair)
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use iced::advanced::layout::Limits;
    use iced::advanced::widget::Tree;
    use iced::widget::space;
    use iced::{Element, Length, Size};

    use super::Form;
    use crate::snapshot::Snapshot;

    /// A named field and an unnamed row laid out in `width`: each row's name and control, (x, y) in the row.
    fn rows(width: f32) -> Vec<Vec<(f32, f32)>> {
        let snapshot = Snapshot::new(Size::new(width, 400.0)).expect("çizici kurulamadı");
        let control = || {
            space::horizontal()
                .width(Length::Fill)
                .height(Length::Fixed(28.0))
        };
        let form: Form<'_, ()> = Form::new().field("Katman", control()).row(control());
        let mut element: Element<'_, ()> = form.into();
        let mut tree = Tree::new(&element);
        let limits = Limits::new(Size::ZERO, Size::new(width, 400.0));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, snapshot.renderer(), &limits);
        node.children()
            .iter()
            .map(|row| {
                row.children()
                    .iter()
                    .map(|c| (c.bounds().x, c.bounds().y))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_wide_form_keeps_its_name_column() {
        let rows = rows(400.0);
        let beside = crate::theme::typography::scaled(super::LABEL) + super::GAP;
        assert_eq!(rows[0][1], (beside, 0.0), "the control beside the name");
        assert_eq!(
            rows[1][1],
            (beside, 0.0),
            "an unnamed row indented as the controls"
        );
    }

    #[test]
    fn a_narrow_form_puts_the_name_above_and_gives_the_control_the_width() {
        // 1100×650 at 16 px text left a dock's controls a third of the width (real window, 15 November 2026).
        let rows = rows(200.0);
        let (x, y) = rows[0][1];
        assert_eq!(x, 0.0, "the control the whole width");
        assert!(y > 4.0, "below the name: {y}");
        assert_eq!(
            rows[1][1],
            (0.0, 0.0),
            "an unnamed row flush, no gap above it"
        );
    }
}
