//! Çerçeveli metin alanı: metin girişi, önündeki ikon ya da kısa etiket,
//! arkasındaki birim, temizleme ve eylem düğmeleri tek çerçevede.
//!
//! ```text
//!  ┌──────────────────────────────────────────┐
//!  │ ⌕  Kadıköy|                       ×   ◎  │   ikon, metin, temizle, eylem
//!  └──────────────────────────────────────────┘
//!  ┌──────────────────────────────────────────┐
//!  │ Y  412.350,25                          m │   etiket, sayı, birim
//!  └──────────────────────────────────────────┘
//! ```
//!
//! Çerçeve bütün alanın durumunu gösterir (DESIGN.md §7.13): üzerine
//! gelince belirgin çizgi, metin girişi odaktayken vurgu çizgisi ve
//! çevresinde hafif hale, geçersiz değerde kırmızı çizgi. Çerçevenin
//! herhangi bir yerine (ikon, birim, boşluk) tıklamak girişi odaklar ve
//! imleci sona koyar.
//!
//! Yükseklik [`metrics`]'ten gelir: varsayılan kontrol yüksekliği yanındaki
//! seçim kutusu ve sayı girişiyle aynıdır; [`InputField::inline`] tablo ve
//! özellik hücresi içindir. [`InputField::cell`] çerçeveyi üzerine gelinene
//! dek gizler: özellik ızgarasında değer düz yazı gibi durur (DESIGN.md
//! §7.5).
//!
//! ```ignore
//! InputField::new("Katman ara", &self.search)
//!     .on_input(Message::SearchChanged)
//!     .icon(Icon::Search)
//!     .clear(Message::SearchChanged(String::new()))
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::alignment::Horizontal;
use iced::widget::text::Wrapping;
use iced::widget::tooltip::Position;
use iced::widget::{TextInput, button, container, text_input};
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Renderer, Size,
    Theme, Vector, mouse,
};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::style::button::radius;
use crate::theme::{Tokens, metrics, typography};
use crate::widget::{Tip, focus_ring, tip};

type Paragraph = <Renderer as iced::advanced::text::Renderer>::Paragraph;

/// Parçalar arasındaki boşluk.
const GAP: f32 = 6.0;

/// Çerçeveli metin alanı.
pub struct InputField<'a, Message> {
    input: TextInput<'a, Message>,
    value_empty: bool,
    editable: bool,
    leading: Option<Element<'a, Message>>,
    suffix: Option<String>,
    trailing: Vec<Element<'a, Message>>,
    /// Son parça bir düğme mi: düğme kenara yakın durur.
    ends_in_button: bool,
    clear: Option<Message>,
    mono: bool,
    inline: bool,
    cell: bool,
    invalid: bool,
    width: Length,
}

impl<'a, Message: Clone + 'a> InputField<'a, Message> {
    pub fn new(placeholder: &str, value: &str) -> Self {
        Self {
            input: text_input(placeholder, value),
            value_empty: value.is_empty(),
            editable: false,
            leading: None,
            suffix: None,
            trailing: Vec::new(),
            ends_in_button: false,
            clear: None,
            mono: false,
            inline: false,
            cell: false,
            invalid: false,
            width: Length::Fill,
        }
    }

    /// Yazılan metin; verilmezse alan salt okunurdur (sönük).
    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.input = self.input.on_input(on_input);
        self.editable = true;
        self
    }

    /// Enter.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.input = self.input.on_submit(message);
        self
    }

    /// Odaklamak ve imleç işlemleri için kimlik.
    pub fn id(mut self, id: impl Into<iced::widget::Id>) -> Self {
        self.input = self.input.id(id);
        self
    }

    /// Yazılanı noktalarla gizler (parola).
    pub fn secure(mut self, secure: bool) -> Self {
        self.input = self.input.secure(secure);
        self
    }

    /// Eş aralıklı yazı: koordinat, ölçü, kod.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// Metin sağa yaslı (sayı sütunları).
    pub fn align_right(mut self) -> Self {
        self.input = self.input.align_x(Horizontal::Right);
        self
    }

    /// Metnin önünde sönük bir ikon (ör. arama).
    pub fn icon(mut self, glyph: Icon) -> Self {
        self.leading = Some(icon(glyph).size(14.0).tone(Tone::Muted).into());
        self
    }

    /// Metnin önünde kısa, yarı kalın bir etiket (ör. eksen adı “Y”);
    /// `color` verilirse o renkte.
    pub fn prefix(mut self, prefix: impl Into<String>, color: Option<Color>) -> Self {
        self.leading = Some(
            iced::widget::text(prefix.into())
                .font(typography::mono_strong())
                .size(typography::caption())
                .style(move |theme: &Theme| iced::widget::text::Style {
                    color: Some(color.unwrap_or(Tokens::of(theme).muted)),
                })
                .into(),
        );
        self
    }

    /// Metnin önünde herhangi bir öğe (ör. renk örneği).
    pub fn leading(mut self, leading: impl Into<Element<'a, Message>>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    /// Değerin birimi; sağda, üçüncül renkte (DESIGN.md §10.3).
    pub fn suffix(mut self, unit: impl Into<String>) -> Self {
        self.suffix = Some(unit.into());
        self
    }

    /// Alan doluyken görünen × düğmesi; `message` metni temizler.
    pub fn clear(mut self, message: Message) -> Self {
        self.clear = Some(message);
        self
    }

    /// Sağda ikon düğmesi (ör. haritadan seç); ipucunda ne yaptığı yazar.
    pub fn action(mut self, glyph: Icon, description: impl Into<String>, message: Message) -> Self {
        self.trailing.push(tip(
            button(container(icon(glyph).size(13.0)).center(Length::Fill))
                .on_press(message)
                .padding(0)
                .width(typography::from_default(20.0))
                .height(typography::from_default(20.0))
                .style(style::button::subtle),
            Tip::new(description.into()),
            Position::Bottom,
        ));
        self.ends_in_button = true;
        self
    }

    /// Sağda herhangi bir öğe (ör. artırma okları).
    pub fn trailing(mut self, trailing: impl Into<Element<'a, Message>>) -> Self {
        self.trailing.push(trailing.into());
        self.ends_in_button = true;
        self
    }

    /// Geçersiz değer: kenar kırmızıdır.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Satır içi yükseklik (tablo, özellik hücresi, araç çubuğu).
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    /// Özellik hücresi: zemin ve kenar üzerine gelinene ya da odaklanana dek
    /// görünmez; satır içi yüksekliktedir.
    pub fn cell(mut self) -> Self {
        self.cell = true;
        self.inline = true;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: Clone + 'a> From<InputField<'a, Message>> for Element<'a, Message> {
    fn from(field: InputField<'a, Message>) -> Self {
        let height = if field.inline {
            metrics::inline()
        } else {
            metrics::control()
        };
        let size = typography::body();
        let input = field
            .input
            .font(if field.mono {
                typography::mono()
            } else {
                typography::ui()
            })
            .size(size)
            .padding(metrics::padding_for(height, size, 0.0))
            .width(Length::Fill)
            .style(|theme: &Theme, status| {
                let mut input = style::field::bare_input(theme, status);

                // Salt okunur değer ikincil renkte: düzenlenmediği belli olsun.
                if matches!(status, text_input::Status::Disabled) {
                    input.value = Tokens::of(theme).muted;
                }

                input
            });

        let mut children: Vec<Element<'a, Message>> = Vec::new();
        let leading = field.leading.is_some();

        if let Some(leading) = field.leading {
            children.push(leading);
        }

        let input_index = children.len();
        children.push(input.into());

        let mut ends_in_button = field.ends_in_button;

        if let Some(unit) = field.suffix {
            children.push(
                label::caption(unit)
                    .wrapping(Wrapping::None)
                    .style(style::text::faint)
                    .into(),
            );
            ends_in_button = false;
        }

        if let (Some(message), false, true) = (field.clear, field.value_empty, field.editable) {
            children.push(
                button(container(icon(Icon::Close).size(10.0)).center(Length::Fill))
                    .on_press(message)
                    .padding(0)
                    .width(typography::from_default(18.0))
                    .height(typography::from_default(18.0))
                    .style(style::button::subtle)
                    .into(),
            );
            ends_in_button = true;
        }

        let trailing = !field.trailing.is_empty();
        children.extend(field.trailing);

        if trailing {
            ends_in_button = true;
        }

        let edge = if field.inline { 6.0 } else { 8.0 };
        let frame = Frame {
            children,
            input: input_index,
            height,
            width: field.width,
            padding: Padding {
                left: if leading { edge - 1.0 } else { edge },
                right: if ends_in_button { 3.0 } else { edge },
                ..Padding::ZERO
            },
            cell: field.cell,
            invalid: field.invalid,
            editable: field.editable,
        };

        if field.cell {
            Element::new(frame)
        } else {
            focus_ring(Element::new(frame)).into()
        }
    }
}

/// Çerçeve: parçaları yan yana dizer, durumu kenarında gösterir.
struct Frame<'a, Message> {
    children: Vec<Element<'a, Message>>,
    input: usize,
    height: f32,
    width: Length,
    padding: Padding,
    cell: bool,
    invalid: bool,
    editable: bool,
}

/// Çerçevenin kendi durumu: imleç üstünde mi (kenar ona göre çizilir).
#[derive(Debug, Clone, Copy, Default)]
struct State {
    hovered: bool,
}

impl<Message> Frame<'_, Message> {
    fn focused(&self, tree: &Tree) -> bool {
        tree.children
            .get(self.input)
            .map(|input| {
                input
                    .state
                    .downcast_ref::<text_input::State<Paragraph>>()
                    .is_focused()
            })
            .unwrap_or(false)
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Frame<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Fixed(self.height))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let height = self.height;
        let size = limits.resolve(self.width, height, Size::new(0.0, height));
        let loose = layout::Limits::new(Size::ZERO, Size::new(size.width, height));

        // Girişin dışındakiler önce, kendi genişlikleriyle; giriş kalanı alır.
        let mut nodes: Vec<Option<Node>> = Vec::with_capacity(self.children.len());
        let mut taken = 0.0;

        for (index, (child, state)) in self.children.iter_mut().zip(&mut tree.children).enumerate()
        {
            if index == self.input {
                nodes.push(None);
                continue;
            }

            let node = child.as_widget_mut().layout(state, renderer, &loose);
            taken += node.size().width;
            nodes.push(Some(node));
        }

        let gaps = GAP * (self.children.len().saturating_sub(1)) as f32;
        let room = (size.width - self.padding.left - self.padding.right - taken - gaps).max(0.0);
        let input = self.children[self.input].as_widget_mut().layout(
            &mut tree.children[self.input],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(room, height)),
        );
        nodes[self.input] = Some(input);

        let mut x = self.padding.left;
        let children = nodes
            .into_iter()
            .flatten()
            .map(|node| {
                let y = ((height - node.size().height) / 2.0).round();
                let placed = node.clone().move_to(Point::new(x, y));
                x += node.size().width + GAP;
                placed
            })
            .collect();

        Node::with_children(size, children)
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
        let was_focused = self.focused(tree);

        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }

        // Çerçevenin boş bir yerine tıklamak girişi odaklar.
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && self.editable
            && !shell.is_event_captured()
            && cursor.is_over(layout.bounds())
        {
            let input = tree.children[self.input]
                .state
                .downcast_mut::<text_input::State<Paragraph>>();
            input.focus();
            input.move_cursor_to_end();
            shell.capture_event();
            shell.request_redraw();
        }

        // Kenar, imlecin üstünde olup olmamasına ve odağa göre çizilir.
        let hovered = cursor.is_over(layout.bounds());
        let state = tree.state.downcast_mut::<State>();

        if state.hovered != hovered || was_focused != self.focused(tree) {
            tree.state.downcast_mut::<State>().hovered = hovered;
            shell.request_redraw();
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
        let interaction = self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default();

        if interaction == mouse::Interaction::None
            && self.editable
            && cursor.is_over(layout.bounds())
        {
            mouse::Interaction::Text
        } else {
            interaction
        }
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
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let focused = self.focused(tree);
        let hovered = self.editable && cursor.is_over(bounds);

        let edge = if self.invalid {
            Some(t.danger)
        } else if focused {
            Some(t.accent_line())
        } else if hovered {
            Some(t.border_strong())
        } else if self.cell {
            None
        } else {
            Some(t.border)
        };

        let background = if self.cell && edge.is_none() {
            Color::TRANSPARENT
        } else if self.editable {
            t.field
        } else {
            t.field.scale_alpha(0.6)
        };

        if edge.is_some() || background.a > 0.0 {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        color: edge.unwrap_or(Color::TRANSPARENT),
                        width: 1.0,
                        radius: radius().into(),
                    },
                    ..Quad::default()
                },
                Background::Color(background),
            );
        }

        let text_style = renderer::Style {
            text_color: if self.editable { t.text } else { t.muted },
        };

        for ((child, state), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child.as_widget().draw(
                state,
                renderer,
                theme,
                if self.editable { style } else { &text_style },
                layout,
                cursor,
                &bounds.intersection(viewport).unwrap_or(bounds),
            );
        }
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
            for ((child, state), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(state, layout, renderer, operation);
            }
        });
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
