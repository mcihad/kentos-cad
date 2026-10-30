//! Özellik hücresi: ızgaradaki metin ve sayı hücresi. Hücreyi çizgiden çizgiye
//! doldurur; durumlarını [`Cell`](crate::widget::cell::Cell) çizer, yazı
//! düzenlenirken de aynı yerde kalır (docs/adr/0804).
//!
//! ```text
//!   Yükseklik │▒2.500▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ m    ← gömük zeminde yazılabilir değer
//!   Yükseklik ║ 3,5|                m ║   ← düzenlenirken alan zemini ve vurgu kenarı
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
//! - Sığmayan değer düzenlenmezken “…” ile kısalır, tamamı ipucundadır; seçili
//!   nesnelerin değerleri farklıysa (“Çeşitli”) soluk yazılır, düzenleme boş
//!   başlar ve yalnız yazılan gönderilir (docs/adr/0748).
//!
//! ```ignore
//! EditCell::new(format.length_bare(height), move |text| Message::Height(slot, text))
//!     .numeric(true)
//!     .unit("m")
//! ```

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, key};
use iced::widget::text::Wrapping;
use iced::widget::{Text, TextInput, text, text_input};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::style;
use crate::theme::{Tokens, metrics, typography};
use crate::widget::cell::Cell;
use crate::widget::dropdown::propagate;
use crate::widget::elided::{self, Cut};
use crate::widget::property_grid::varies;

/// Birimle alan arasındaki boşluk.
const GAP: f32 = 4.0;
/// The field's text's distance to its right edge.
const FIELD_END: f32 = 5.0;

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
            parts: [
                field(&value, "—", false).into(),
                unit_text(String::new()).into(),
            ],
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

    /// Parçaları duruma göre kurar: düzenlenirken yazılan, değilse değer (sığmıyorsa kısalmışı). Değeri olmayan
    /// hücre soluk “—”, değerleri farklı olan soluk “Çeşitli” gösterir (docs/adr/0747, 0748).
    fn build(&mut self, state: &State) {
        let mixed = varies(&self.value);
        let (content, placeholder) = if state.editing {
            (
                state.buffer.as_str(),
                if mixed { self.value.as_str() } else { "" },
            )
        } else if mixed {
            ("", state.rest.as_deref().unwrap_or(&self.value))
        } else {
            (state.rest.as_deref().unwrap_or(&self.value), "—")
        };
        self.parts = [
            field(content, placeholder, self.numeric && !mixed).into(),
            unit_text(self.unit.clone()).into(),
        ];
    }

    /// At rest, what the field shows as it fits its text's room: the value (a mixed value's “Çeşitli”) itself, or
    /// cut with “…” (docs/adr/0748). Measured again only when the value, the room or the type changes.
    fn fit_rest(&self, state: &mut State, field_width: f32) {
        let mixed = varies(&self.value);
        let size = typography::body();
        let font = if self.numeric && !mixed {
            typography::mono()
        } else {
            typography::ui()
        };
        let room = (field_width - metrics::cell_pad() - FIELD_END).max(0.0);
        let key = (self.value.clone(), room.to_bits(), size.to_bits(), font);
        if state.measured.as_ref() != Some(&key) {
            let (_, cut) = elided::fit(
                &self.value,
                room,
                size,
                font,
                iced::widget::text::LineHeight::default(),
            );
            state.rest = cut;
            state.measured = Some(key);
        }
    }

    /// What an edit starts from: the value, or nothing for a mixed one (what is typed goes to them all).
    fn start(&self, state: &mut State) {
        state.editing = true;
        state.buffer = if varies(&self.value) {
            String::new()
        } else {
            self.value.clone()
        };
    }
}

/// The cell's text input: `content`, or `placeholder` faint while it is empty (“—” of a value-less field at rest, as
/// an empty list's; “Çeşitli” of a mixed one; docs/adr/0747, 0748).
fn field<'a>(content: &str, placeholder: &str, numeric: bool) -> Field<'a> {
    text_input(placeholder, content)
        .on_input(Edit::Input)
        .on_submit(Edit::Submit)
        .font(if numeric {
            typography::mono()
        } else {
            typography::ui()
        })
        .size(typography::body())
        // Its text where a read-only value's is, the field filling the cell (docs/adr/0804).
        .padding(iced::Padding {
            top: 0.0,
            bottom: 0.0,
            left: metrics::cell_pad(),
            right: FIELD_END,
        })
        // The input is made again at each layout, so it has seen no event yet: iced draws such an input as disabled
        // (its value in the muted tone). The cell's value is the drawing's, in the text tone (docs/adr/0747).
        .style(|theme: &Theme, status| {
            style::field::bare_input(
                theme,
                match status {
                    text_input::Status::Disabled => text_input::Status::Active,
                    status => status,
                },
            )
        })
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
    /// At rest, the value cut with “…” to fit, when it does not (docs/adr/0748).
    rest: Option<String>,
    /// What `rest` was measured for: the value, the room, the size and the type.
    measured: Option<(String, u32, u32, iced::Font)>,
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
        Size::new(Length::Fill, Length::Fixed(metrics::row()))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let height = metrics::row();
        let size = limits.resolve(Length::Fill, height, Size::new(0.0, height));
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        // The unit first: the field takes what it leaves.
        self.parts[1] = unit_text(self.unit.clone()).into();

        let unbounded = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, height));
        let unit = self.parts[1]
            .as_widget_mut()
            .layout(&mut children[1], renderer, &unbounded);
        // The unit keeps its distance to the cell's right edge, as a read-only value's.
        let unit_pad = metrics::cell_end();
        let suffix = if unit.size().width > 0.0 {
            unit.size().width + GAP + unit_pad
        } else {
            0.0
        };
        let field_width = (size.width - suffix).max(0.0);
        if !state.editing {
            self.fit_rest(state, field_width);
        }
        self.build(state);
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
                unit.clone().move_to(Point::new(
                    size.width - unit.size().width - unit_pad,
                    middle(&unit),
                )),
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

        if !state.editing {
            // A press on the cell, or the focus coming with Tab, starts editing.
            let pressed = matches!(
                event,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            ) && cursor.is_over(bounds);
            if !pressed && !input_state(&children[0]).is_focused() {
                return;
            }
            if pressed {
                // The press goes to the field as it shows: the caret lands where the shown text was pressed, a
                // shortened one's too, before the field takes the whole value (docs/adr/0748).
                let mut pressed_edits = Vec::<Edit>::new();
                let mut local = Shell::new(&mut pressed_edits);
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
                // On a shortened value's “…”, or at its last shown letter, the caret goes to the whole value's end.
                if let Some(shown) = &state.rest {
                    let field = input_state_mut(&mut children[0]);
                    let visible = shown.chars().count().saturating_sub(1);
                    let at = field.cursor().state(&text_input::Value::new(shown));
                    if matches!(at, text_input::cursor::State::Index(i) if i >= visible) {
                        field.move_cursor_to_end();
                    }
                }
            }
            self.start(state);
            self.build(state);
            shell.invalidate_layout();
            return;
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

        // A host save chord must neither type a letter nor commit this edit buffer.
        if super::key_focus::host_character_chord(event) {
            return;
        }

        let mut edits = Vec::new();
        let mut local = Shell::new(&mut edits);
        let modifiers = super::key_focus::text_modifiers(event);
        for event in modifiers.iter().chain(std::iter::once(event)) {
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
        }
        propagate(&local, shell);
        drop(local);

        let mut submitted = false;
        let mut changed = false;
        for edit in edits {
            match edit {
                Edit::Input(buffer) => {
                    changed |= state.buffer != buffer;
                    state.buffer = buffer;
                }
                Edit::Submit => submitted = true,
            }
        }

        // Enter or leaving the cell takes what was typed.
        let blurred = !input_state(&children[0]).is_focused();
        if submitted || blurred {
            // A mixed value left empty is kept as it is (nothing typed for them all).
            let kept = varies(&self.value) && state.buffer.is_empty();
            if state.buffer != self.value && !kept {
                shell.publish((self.on_commit)(state.buffer.clone()));
            }
            finish(state, &mut children[0]);
            changed = true;
        }
        // Cursor motion, selection and caret blinking only need the child's redraw.
        // Invalidating on every event rebuilds the entire CAD window even while idle.
        if changed {
            self.build(state);
            shell.invalidate_layout();
        }
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
        let t = Tokens::of(theme);
        let mut parts = layout.children();
        let Some(field) = parts.next() else {
            return;
        };
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
        // Whether the value at rest was cut to fit: its tip shows the whole of it (docs/adr/0748).
        let state = tree.state.downcast_ref::<State>();
        let mut cut = Cut(!state.editing && state.rest.is_some());
        operation.custom(None, layout.bounds(), &mut cut);
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

/// The cell, its whole value in a tip when it was cut to fit (docs/adr/0748).
impl<'a, Message: Clone + 'a> From<EditCell<'a, Message>> for Element<'a, Message> {
    fn from(cell: EditCell<'a, Message>) -> Self {
        let value = cell.value.clone();
        Cell::new(crate::widget::tip(
            Element::new(cell),
            crate::widget::Tip::new(value).when_cut(),
            iced::widget::tooltip::Position::Bottom,
        ))
        .into()
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

    #[test]
    fn idle_caret_and_pointer_events_do_not_relayout_the_window() {
        use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, widget::Tree};
        use iced::{Event, Rectangle, mouse, window};
        let snapshot = Snapshot::new(Size::new(240.0, 60.0)).expect("renderer");
        let renderer = snapshot.renderer();
        let mut cell = EditCell::new("2.500", |text| text);
        let mut tree = Tree::new(&cell as &dyn Widget<String, iced::Theme, iced::Renderer>);
        let limits = layout::Limits::new(Size::ZERO, Size::new(240.0, 60.0));
        let mut node = cell.layout(&mut tree, renderer, &limits);
        let viewport = Rectangle::with_size(Size::new(240.0, 60.0));
        let cursor = mouse::Cursor::Available(Point::new(150.0, 12.0));
        let mut messages = Vec::new();
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut shell = Shell::new(&mut messages);
        cell.update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            cursor,
            renderer,
            &mut clipboard as &mut dyn Clipboard,
            &mut shell,
            &viewport,
        );
        assert!(tree.state.downcast_ref::<super::State>().editing);
        node = cell.layout(&mut tree, renderer, &limits);
        for i in 0..120 {
            let event = if i % 2 == 0 {
                Event::Window(window::Event::RedrawRequested(std::time::Instant::now()))
            } else {
                Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(151.0, 12.0),
                })
            };
            let mut shell = Shell::new(&mut messages);
            cell.update(
                &mut tree,
                &event,
                Layout::new(&node),
                cursor,
                renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(
                !shell.is_layout_invalid(),
                "idle event {i} caused a full relayout"
            );
        }
        assert!(messages.is_empty(), "no document edit before commit");
    }

    /// A value longer than the cell: it rests cut with “…” (docs/adr/0748).
    const LONG: &str = "Kuzey cephesindeki çatlakların bir kısmı onarıldı";

    #[test]
    fn a_long_value_rests_shortened_to_its_field_and_whole_when_it_fits() {
        use iced::advanced::{Widget, layout, widget::Tree};
        let snapshot = Snapshot::new(Size::new(240.0, 60.0)).expect("renderer");
        let renderer = snapshot.renderer();
        let mut cell = EditCell::new(LONG, |text| text).unit("m");
        let mut tree = Tree::new(&cell as &dyn Widget<String, iced::Theme, iced::Renderer>);
        let narrow = layout::Limits::new(Size::ZERO, Size::new(220.0, 60.0));
        let _ = cell.layout(&mut tree, renderer, &narrow);
        let rest = tree.state.downcast_ref::<super::State>().rest.clone();
        assert!(
            rest.as_deref().is_some_and(|shown| shown.ends_with('…')
                && LONG.starts_with(shown.trim_end_matches('…'))
                && shown.chars().count() < LONG.chars().count()),
            "{rest:?}"
        );
        let wide = layout::Limits::new(Size::ZERO, Size::new(800.0, 60.0));
        let _ = cell.layout(&mut tree, renderer, &wide);
        assert_eq!(tree.state.downcast_ref::<super::State>().rest, None);
    }

    #[test]
    fn a_press_on_a_shortened_value_edits_the_whole_value() {
        // The “…” at the field's right end, left of the unit: the caret goes to the whole value's end.
        let commits = play(
            LONG,
            vec![
                Input::Click(Point::new(203.0, 22.0)),
                Input::Type("!".into()),
                Input::Key(Named::Enter),
            ],
        );
        assert_eq!(commits, [format!("{LONG}!")]);
        // Its first letters: the caret there, in the whole value.
        let commits = play(
            LONG,
            vec![
                Input::Click(Point::new(22.0, 22.0)),
                Input::Type("X".into()),
                Input::Key(Named::Enter),
            ],
        );
        assert!(
            commits.len() == 1
                && commits[0].ends_with("onarıldı")
                && commits[0].chars().count() == LONG.chars().count() + 1,
            "{commits:?}"
        );
    }

    #[test]
    fn a_mixed_value_starts_empty_and_commits_only_what_is_typed() {
        // Pressed and left: the objects keep their values.
        assert!(play("Çeşitli", vec![Input::Click(END), Input::Key(Named::Enter)]).is_empty());
        assert!(
            play(
                "Çeşitli (2/3 nesnede)",
                vec![Input::Click(END), Input::Click(Point::new(5.0, 55.0))]
            )
            .is_empty()
        );
        // What is typed goes to them all, not after “Çeşitli”.
        assert_eq!(
            play(
                "Çeşitli",
                vec![
                    Input::Click(END),
                    Input::Type("12".into()),
                    Input::Key(Named::Enter)
                ]
            ),
            ["12"]
        );
    }

    #[test]
    fn rapid_typing_keeps_every_character_until_one_commit() {
        let commits = play(
            "",
            vec![
                Input::Click(END),
                Input::Type("12345,6789".into()),
                Input::Key(Named::Enter),
            ],
        );
        assert_eq!(commits, ["12345,6789"]);
    }
}
