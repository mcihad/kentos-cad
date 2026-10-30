//! The field of a cell edited in Noktalar (docs/adr/0153 §3; the web's
//! `PointTable.editor`): the cell itself from line to line, the accent on its edge (KentOS UI's `Cell`,
//! docs/adr/0805); its text starts where the cell's text stands, so nothing moves when an edit starts.
//! Enter sends `on_enter` (the edit is written and goes down the column), a
//! press outside it `on_leave` (written, the edit stops; the press goes on
//! where it was made), Esc `on_cancel` (given up). Tab is the app's key
//! (`points::Event::Tab`). The text box swallows Esc and tells nothing when it
//! loses the keyboard, so both are caught here before it, as KentOS UI's
//! rename box does; it tells Enter from a press elsewhere, which that one does
//! not.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, key};
use iced::widget::text_input;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse};
use kentos_ui::theme::typography;

/// The field's id: the app gives it the keyboard when an edit starts.
pub const FIELD: &str = "kentos-point-cell";

type Paragraph = <Renderer as iced::advanced::text::Renderer>::Paragraph;

struct EditBox<'a, Message> {
    input: Element<'a, Message>,
    on_leave: Message,
    on_cancel: Message,
}

/// The cell's field holding `value`, right-aligned for a number; `pads` are the column's (`table::cell_pads`).
pub fn edit_box<'a, Message: Clone + 'a>(
    value: &str,
    numeric: bool,
    pads: (f32, f32),
    on_input: impl Fn(String) -> Message + 'a,
    on_enter: Message,
    on_leave: Message,
    on_cancel: Message,
) -> Element<'a, Message> {
    let input = kentos_ui::widget::bare_field("", value)
        .padding(
            kentos_ui::theme::metrics::padding(kentos_ui::widget::table::row_height(), 0.0)
                .left(pads.0)
                .right(pads.1),
        )
        .id(FIELD)
        .on_input(on_input)
        .on_submit(on_enter)
        .font(if numeric {
            typography::mono()
        } else {
            typography::ui()
        })
        .align_x(if numeric {
            iced::alignment::Horizontal::Right
        } else {
            iced::alignment::Horizontal::Left
        });
    kentos_ui::widget::Cell::new(Element::new(EditBox {
        input: input.into(),
        on_leave,
        on_cancel,
    }))
    .into()
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for EditBox<'a, Message> {
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
    ) -> layout::Node {
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
            // A press elsewhere writes; the press goes on to where it was made.
            Event::Mouse(mouse::Event::ButtonPressed(_))
                if focused && !cursor.is_over(layout.bounds()) =>
            {
                shell.publish(self.on_leave.clone());
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
