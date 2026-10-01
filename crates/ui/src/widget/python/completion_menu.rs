//! Cursor-anchored IntelliSense overlay with semantic vector icons and docs.

use super::{CompletionEvent, CompletionState, SymbolKind, font, strong_font};
use crate::icon::{Icon, Tone, icon};
use crate::style;
use crate::theme::{Tokens, shape, typography};
use iced::advanced::{
    Clipboard, Shell,
    layout::{self, Layout},
    overlay, renderer,
    widget::Tree,
};
use iced::widget::{Column, button, column, container, mouse_area, row, space, text};
use iced::{Border, Element, Event, Fill, Point, Rectangle, Renderer, Size, Theme, mouse};
use std::rc::Rc;

pub(super) type Events<'a, Message> = Rc<dyn Fn(CompletionEvent) -> Message + 'a>;

pub(super) fn glyph(kind: SymbolKind) -> (Icon, Tone) {
    match kind {
        SymbolKind::Function => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="M13 3h-2c-2 0-3 1-3 3v8c0 2-1 3-3 3H3M5 8h8M13 12l4 5m0-5-4 5"/></svg>"#,
            ),
            Tone::Accent,
        ),
        SymbolKind::Method => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="m10 2 7 4v9l-7 4-7-4V6l7-4m-7 4 7 4 7-4m-7 4v9M7 13l3-2 3 2"/></svg>"#,
            ),
            Tone::Accent,
        ),
        SymbolKind::Parameter => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="M6 3H3v14h3M14 3h3v14h-3M7 10h6m-3-3 3 3-3 3"/></svg>"#,
            ),
            Tone::Success,
        ),
        SymbolKind::Class => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="m10 2 7 4-7 4-7-4 7-4M3 6v8l7 4 7-4V6M10 10v8M6 12l4 2 4-2"/></svg>"#,
            ),
            Tone::Warning,
        ),
        SymbolKind::Package => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="M2 6h16v11H2V6m0 0 3-4h10l3 4M7 6v5h6V6M6 14h3"/></svg>"#,
            ),
            Tone::Warning,
        ),
        SymbolKind::Module => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="M3 3h6v6H3V3m8 0h6v6h-6V3M3 11h6v6H3v-6m8 0h6v6h-6v-6"/></svg>"#,
            ),
            Tone::Warning,
        ),
        SymbolKind::Property => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="m10 2 8 8-8 8-8-8 8-8m-3 8h6m-3-3v6"/></svg>"#,
            ),
            Tone::Success,
        ),
        SymbolKind::Variable => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="M6 3H4c-1 0-2 1-2 2v3l-1 2 1 2v3c0 1 1 2 2 2h2m8-14h2c1 0 2 1 2 2v3l1 2-1 2v3c0 1-1 2-2 2h-2M8 8l4 4m0-4-4 4"/></svg>"#,
            ),
            Tone::Highlight,
        ),
        SymbolKind::Keyword => (
            Icon::Svg(
                r#"<svg viewBox="0 0 20 20"><path d="m7 5-5 5 5 5m6-10 5 5-5 5M11 3 9 17"/></svg>"#,
            ),
            Tone::Muted,
        ),
        SymbolKind::Constant => (Icon::Hash, Tone::Highlight),
    }
}


pub(super) fn view<'a, Message: Clone + 'a>(
    state: &'a CompletionState,
    events: Events<'a, Message>,
) -> Element<'a, Message> {
    let first = state
        .selected
        .saturating_sub(5)
        .min(state.items.len().saturating_sub(7));
    let mut rows = Column::new().spacing(2);
    for (index, item) in state.items.iter().enumerate().skip(first).take(7) {
        let selected = index == state.selected;
        let (glyph, tone) = glyph(item.kind);
        let prefix = item.name.get(..state.prefix.len()).unwrap_or("");
        let suffix = item.name.get(state.prefix.len()..).unwrap_or(&item.name);
        let label = row![
            text(prefix)
                .font(strong_font())
                .size(typography::body())
                .style(style::text::default),
            text(suffix).font(font()).size(typography::body())
        ];
        rows = rows.push(
            mouse_area(
                button(
                    row![
                        icon(glyph).size(18.0).tone(tone),
                        container(label).width(Fill),
                        text(item.kind.label())
                            .size(typography::caption())
                            .style(style::text::muted)
                    ]
                    .spacing(10)
                    .align_y(iced::Center),
                )
                .on_press(events(CompletionEvent::Pick(index)))
                .width(Fill)
                .padding([7, 9])
                .style(style::button::segment(selected)),
            )
            .on_enter(events(CompletionEvent::Select(index))),
        );
    }
    if state.items.is_empty() {
        rows = rows.push(
            container(
                text("Python önerileri hazırlanıyor…")
                    .size(typography::body())
                    .style(style::text::muted),
            )
            .padding([16, 10]),
        );
    }
    let mut content = column![
        container(
            row![
                icon(Icon::Terminal).size(13.0).tone(Tone::Accent),
                text("IntelliSense")
                    .font(typography::ui_strong())
                    .size(typography::caption()),
                space::horizontal(),
                text(if state.loading {
                    "Python".into()
                } else {
                    format!("{} öneri", state.items.len())
                })
                .size(typography::caption())
                .style(style::text::muted)
            ]
            .spacing(7)
            .align_y(iced::Center)
        )
        .padding([8, 11]),
        crate::widget::horizontal_divider(),
        container(rows).padding(5).width(Fill),
    ]
    .spacing(0);
    if let Some(item) = state.selected() {
        let mut details = Column::new().spacing(5);
        if !item.detail.is_empty() {
            details = details.push(text(&item.detail).font(font()).size(typography::body()));
        }
        if !item.documentation.is_empty() {
            details = details.push(
                text(&item.documentation)
                    .size(typography::caption())
                    .style(style::text::muted),
            );
        }
        if item.detail.is_empty() {
            details = details.push(
                text(item.kind.label())
                    .size(typography::caption())
                    .style(style::text::muted),
            );
        }
        content = content.push(crate::widget::horizontal_divider()).push(
            container(details)
                .padding([9, 12])
                .width(Fill)
                .height(typography::scaled(72.0))
                .clip(true),
        );
    }
    content = content.push(
        container(
            text("↑↓ seç · Tab / Enter tamamla · Esc kapat")
                .size(typography::caption())
                .style(style::text::muted),
        )
        .padding([7, 12]),
    );
    super::surface::Surface::new(
        container(content)
            .width(Fill)
            .padding(1)
            .style(|theme: &Theme| {
                let t = Tokens::of(theme);
                container::Style {
                    background: Some(t.field.into()),
                    border: Border {
                        color: t.border,
                        width: 1.0,
                        radius: shape::lg().into(),
                    },
                    shadow: shape::shadow(shape::Level::Pop, &t),
                    ..container::Style::default()
                }
            }),
        if state.loading {
            super::RunStatus::Running
        } else {
            super::RunStatus::Ready
        },
        state.selected as u64,
    )
    .into()
}

pub(super) struct Menu<'a, 'b, Message> {
    pub content: &'b mut Element<'a, Message>,
    pub tree: &'b mut Tree,
    pub anchor: Rectangle,
    pub events: Events<'a, Message>,
}
impl<Message> overlay::Overlay<Message, Theme, Renderer> for Menu<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let width = typography::scaled(420.0).min((bounds.width - 16.0).max(0.0));
        let node = self.content.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, bounds.height - 16.0)),
        );
        let x = self
            .anchor
            .x
            .min(bounds.width - node.size().width - 8.0)
            .max(8.0);
        let below = self.anchor.y + self.anchor.height + 5.0;
        let y = if below + node.size().height <= bounds.height - 8.0 {
            below
        } else {
            (self.anchor.y - node.size().height - 5.0).max(8.0)
        };
        layout::Node::with_children(bounds, vec![node.move_to(Point::new(x, y))])
    }
    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            self.tree,
            event,
            child,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) && !cursor.is_over(child.bounds())
        {
            shell.publish((self.events)(CompletionEvent::Dismiss));
        }
    }
    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        if let Some(child) = layout.children().next() {
            self.content.as_widget().draw(
                self.tree,
                renderer,
                theme,
                style,
                child,
                cursor,
                &layout.bounds(),
            );
        }
    }
    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::default(), |child| {
                self.content.as_widget().mouse_interaction(
                    self.tree,
                    child,
                    cursor,
                    &layout.bounds(),
                    renderer,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semantic_vector_icons_contain_visible_outlines() {
        for kind in [
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Parameter,
            SymbolKind::Class,
            SymbolKind::Module,
            SymbolKind::Package,
            SymbolKind::Variable,
            SymbolKind::Property,
            SymbolKind::Keyword,
        ] {
            let (Icon::Svg(svg), _) = glyph(kind) else {
                panic!("vector icon")
            };
            assert!(crate::icon::svg_elements(svg) > 0, "{kind:?}");
        }
    }
}
