//! Özellik hücresi: düzenlenene dek düz yazı gibi görünen değer (ızgaradaki
//! metin ve sayı hücresi).
//!
//! ```text
//!   Yükseklik     2.500            m     ← düz yazı
//!   Yükseklik    ┌2.500──────────┐ m     ← üzerine gelince ince kenar
//!   Yükseklik    ┃3,5│           ┃ m     ← düzenlenirken zemin ve vurgu kenarı
//! ```
//!
//! - Tıklanınca imlecin olduğu yerde düzenlenir; Tab ile gelen odak da
//!   düzenlemeye geçirir.
//! - Enter ya da hücreden çıkmak (başka bir yere tıklamak) onaylar: yazılan
//!   gösterilen değerden farklıysa `on_commit` yazılanla gönderilir. Esc
//!   vazgeçer; Esc uygulamaya ulaşmaz.
//! - Hücre her zaman uygulamanın verdiği değeri gösterir: uygulama yazılanı
//!   almazsa (boş metin, sayı olmayan değer) düzenleme bitince eski değer
//!   görünür.
//!
//! ```ignore
//! EditCell::new(format.length_bare(height), move |text| Message::Height(slot, text))
//!     .numeric(true)
//!     .unit("m")
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, key};
use iced::widget::text::Wrapping;
use iced::widget::{Text, TextInput, text, text_input};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    mouse,
};

use crate::style;
use crate::style::button::RADIUS;
use crate::theme::{Tokens, typography};
use crate::widget::dropdown::propagate;

/// Hücrenin yüksekliği, 12 piksellik gövde metnine göre.
const HEIGHT: f32 = 24.0;
/// Birimle alan arasındaki boşluk.
const GAP: f32 = 4.0;

/// İçteki metin girişinin mesajları; uygulamaya gitmez.
#[derive(Debug, Clone)]
enum Edit {
    Input(String),
    Submit,
}

type Field<'a> = TextInput<'a, Edit, Theme, Renderer>;
type Paragraph = <Renderer as iced::advanced::text::Renderer>::Paragraph;

/// Özellik hücresi.
pub struct EditCell<'a, Message> {
    value: String,
    on_commit: Box<dyn Fn(String) -> Message + 'a>,
    numeric: bool,
    unit: String,
    /// Düzenlenen ya da gösterilen metin ve birim.
    parts: [Element<'a, Edit>; 2],
}

impl<'a, Message: Clone + 'a> EditCell<'a, Message> {
    /// `value` gösterilen değerdir; onaylanan farklı metin `on_commit` ile gelir.
    pub fn new(value: impl Into<String>, on_commit: impl Fn(String) -> Message + 'a) -> Self {
        let value = value.into();
        Self {
            parts: [field(&value, false).into(), unit_text(String::new()).into()],
            value,
            on_commit: Box::new(on_commit),
            numeric: false,
            unit: String::new(),
        }
    }

    /// Sayı: rakamlar eş aralıklı yazılır.
    pub fn numeric(mut self, numeric: bool) -> Self {
        self.numeric = numeric;
        self
    }

    /// Değerin sağında, soluk yazılan birim (m, °, m²).
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Parçaları duruma göre kurar: düzenlenirken yazılan, değilse değer.
    fn build(&mut self, state: &State) {
        let content = if state.editing {
            state.buffer.as_str()
        } else {
            self.value.as_str()
        };
        self.parts = [
            field(content, self.numeric).into(),
            unit_text(self.unit.clone()).into(),
        ];
    }
}

fn field<'a>(content: &str, numeric: bool) -> Field<'a> {
    text_input("", content)
        .on_input(Edit::Input)
        .on_submit(Edit::Submit)
        .font(if numeric {
            typography::mono()
        } else {
            typography::ui()
        })
        .size(typography::body())
        .padding([0, 5])
        .style(style::field::bare_input)
}

fn unit_text<'a>(symbol: String) -> Text<'a> {
    text(symbol)
        .font(typography::ui())
        .size(typography::caption())
        .wrapping(Wrapping::None)
        .style(style::text::muted)
}

#[derive(Debug, Default)]
struct State {
    editing: bool,
    buffer: String,
    hovered: bool,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for EditCell<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.parts.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.parts);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(typography::scaled(HEIGHT)))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        self.build(tree.state.downcast_ref::<State>());

        let height = typography::scaled(HEIGHT);
        let size = limits.resolve(Length::Fill, height, Size::new(0.0, height));
        let children = &mut tree.children;

        let unbounded = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, height));
        let unit = self.parts[1]
            .as_widget_mut()
            .layout(&mut children[1], renderer, &unbounded);
        let suffix = if unit.size().width > 0.0 {
            unit.size().width + GAP
        } else {
            0.0
        };
        let field_width = (size.width - suffix).max(0.0);
        let field = self.parts[0].as_widget_mut().layout(
            &mut children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(field_width, height)),
        );
        let middle = |node: &Node| ((height - node.size().height) / 2.0).round();

        Node::with_children(
            size,
            vec![
                field.clone().move_to(Point::new(0.0, middle(&field))),
                unit.clone()
                    .move_to(Point::new(size.width - unit.size().width, middle(&unit))),
            ],
        )
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
        let Some(field_layout) = layout.children().next() else {
            return;
        };
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();

        if let Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) = event {
            let hovered = cursor.is_over(bounds);
            if hovered != state.hovered {
                state.hovered = hovered;
                shell.request_redraw();
            }
        }

        if !state.editing {
            // A press on the cell, or the focus coming with Tab, starts editing.
            let pressed = matches!(
                event,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            ) && cursor.is_over(bounds);
            if !pressed && !input_state(&children[0]).is_focused() {
                return;
            }
            state.editing = true;
            state.buffer = self.value.clone();
            self.build(state);
            shell.invalidate_layout();
            if !pressed {
                return;
            }
        }

        // Esc: the shown value stays; the key goes no further.
        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            ..
        }) = event
        {
            finish(state, &mut children[0]);
            self.build(state);
            shell.invalidate_layout();
            shell.capture_event();
            return;
        }

        let mut edits = Vec::new();
        let mut local = Shell::new(&mut edits);
        self.parts[0].as_widget_mut().update(
            &mut children[0],
            event,
            field_layout,
            cursor,
            renderer,
            clipboard,
            &mut local,
            viewport,
        );
        propagate(&local, shell);
        drop(local);

        let mut submitted = false;
        for edit in edits {
            match edit {
                Edit::Input(buffer) => state.buffer = buffer,
                Edit::Submit => submitted = true,
            }
        }

        // Enter or leaving the cell takes what was typed.
        let blurred = !input_state(&children[0]).is_focused();
        if submitted || blurred {
            if state.buffer != self.value {
                shell.publish((self.on_commit)(state.buffer.clone()));
            }
            finish(state, &mut children[0]);
        }
        self.build(state);
        shell.invalidate_layout();
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let mut parts = layout.children();
        let Some(field) = parts.next() else {
            return;
        };
        // The box around the field only: the unit stays outside it.
        let frame = field.bounds();
        let (background, edge) = if state.editing {
            (t.field, t.accent)
        } else if state.hovered {
            (Color::TRANSPARENT, t.border)
        } else {
            (Color::TRANSPARENT, Color::TRANSPARENT)
        };
        if edge != Color::TRANSPARENT {
            renderer.fill_quad(
                Quad {
                    bounds: frame,
                    border: Border {
                        color: edge,
                        width: 1.0,
                        radius: RADIUS.into(),
                    },
                    ..Quad::default()
                },
                Background::Color(background),
            );
        }
        self.parts[0].as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style { text_color: t.text },
            field,
            cursor,
            viewport,
        );
        if let Some(unit) = parts.next() {
            self.parts[1].as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                &renderer::Style {
                    text_color: t.muted,
                },
                unit,
                cursor,
                viewport,
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
            if let Some(field) = layout.children().next() {
                self.parts[0].as_widget_mut().operate(
                    &mut tree.children[0],
                    field,
                    renderer,
                    operation,
                );
            }
        });
    }
}

impl<'a, Message: Clone + 'a> From<EditCell<'a, Message>> for Element<'a, Message> {
    fn from(cell: EditCell<'a, Message>) -> Self {
        Element::new(cell)
    }
}

fn input_state(tree: &Tree) -> &text_input::State<Paragraph> {
    tree.state.downcast_ref::<text_input::State<Paragraph>>()
}

fn input_state_mut(tree: &mut Tree) -> &mut text_input::State<Paragraph> {
    tree.state.downcast_mut::<text_input::State<Paragraph>>()
}

/// Düzenlemeyi bitirir; hücre yeniden değeri gösterir.
fn finish(state: &mut State, field: &mut Tree) {
    state.editing = false;
    state.buffer.clear();
    input_state_mut(field).unfocus();
}

/// Gerçek olaylarla: tıklama, yazma, Enter, Esc ve başka yere tıklama.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::keyboard::key::Named;
    use iced::widget::container;
    use iced::{Element, Point, Size};

    use super::EditCell;
    use crate::snapshot::{Input, Snapshot};

    /// The value shown, and what was committed.
    struct Cell {
        value: String,
        commits: Vec<String>,
    }

    fn view(cell: &Cell) -> Element<'_, String> {
        container(EditCell::new(cell.value.clone(), |text| text).unit("m"))
            .padding(10)
            .into()
    }

    fn play(value: &str, inputs: Vec<Input>) -> Vec<String> {
        let mut snapshot = Snapshot::new(Size::new(240.0, 60.0)).expect("a renderer");
        let mut cell = Cell {
            value: value.to_owned(),
            commits: Vec::new(),
        };
        // The app takes nothing: the cell shows the value it is given.
        let mut update = |cell: &mut Cell, text: String| cell.commits.push(text);
        for input in inputs {
            snapshot.input(&mut cell, view, &mut update, input);
        }
        cell.commits
    }

    /// Right of the text, left of the unit: the caret goes to the end.
    const END: Point = Point::new(150.0, 22.0);

    #[test]
    fn enter_commits_what_was_typed() {
        let commits = play(
            "2.500",
            vec![
                Input::Click(END),
                Input::Type("1".to_owned()),
                Input::Key(Named::Enter),
            ],
        );
        assert_eq!(commits, ["2.5001"]);
    }

    #[test]
    fn the_same_value_or_esc_commits_nothing() {
        let commits = play(
            "2.500",
            vec![
                Input::Click(END),
                Input::Key(Named::Enter),
                Input::Click(END),
                Input::Type("9".to_owned()),
                Input::Key(Named::Escape),
            ],
        );
        assert!(commits.is_empty(), "{commits:?}");
    }

    #[test]
    fn a_click_elsewhere_commits() {
        let commits = play(
            "12",
            vec![
                Input::Click(END),
                Input::Type("3".to_owned()),
                Input::Click(Point::new(5.0, 55.0)),
            ],
        );
        assert_eq!(commits, ["123"]);
    }
}
