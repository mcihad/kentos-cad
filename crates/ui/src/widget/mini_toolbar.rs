//! Mini araç çubuğu: seçimin yanında beliren, sık kullanılan komutların
//! küçük çubuğu.
//!
//! ```text
//!           ╭─────────────────────╮
//!           │ ⌖  ▦  ⧉ │ 🔒 │ ✕   │   imleç uzaklaştıkça soluklaşır
//!           ╰─────────────────────╯
//!              ┌─────────────┐
//!              │    seçim    │
//!              └─────────────┘
//! ```
//!
//! Çubuk içeriğin (ör. model alanı) üstünde, seçimin sınırlarının üst
//! ortasında durur; üstte yer yoksa altına geçer, içeriğin kenarlarından
//! taşmaz. Office'teki mini araç çubuğu gibi imleç uzaklaştıkça soluklaşır
//! ve çizimi kapatmaz; imleç yaklaşınca belirginleşir. Düğmenin adı ve
//! kısayolu, üzerine gelince çubuğun seçimden uzak yanında yazar.
//!
//! Sınırlar içeriğin sol üst köşesine göredir (ör. model alanında seçimin
//! ekrandaki kutusu). Seçim görünür alanın dışındaysa çubuk gizlenir.
//!
//! ```ignore
//! MiniToolbar::new(model_space, self.selection_bounds())
//!     .button(Icon::Target, "Seçime yakınlaştır", Message::FocusSelection)
//!     .button(Icon::Copy, "Kopyala", Message::Copy)
//!     .shortcut("Ctrl+C")
//!     .separator()
//!     .button(Icon::Lock, "Katmanı kilitle", Message::Lock)
//!     .active(layer.locked)
//!     .button(Icon::Close, "Sil", Message::Delete)
//!     .danger()
//! ```

use std::time::Duration;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::time::Instant;
use iced::widget::{container, row};
use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Renderer, Shadow, Size, Theme,
    Vector, mouse, window,
};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, motion, typography};

/// Düğmenin kenarı, çubuğun iç boşluğu ve ayırıcının genişliği (12
/// piksellik gövde metnine göre).
const ITEM: f32 = 28.0;
const PAD: f32 = 3.0;
const SEPARATOR: f32 = 9.0;
/// Çubukla seçim arası, içeriğin kenarlarıyla en az aralık ve ipucunun
/// çubuğa uzaklığı.
const OFFSET: f32 = 10.0;
const GAP: f32 = 8.0;
const TIP_GAP: f32 = 6.0;
/// İmleç çubuğa `NEAR` pikselden yakınken çubuk tam görünür; `FAR` piksel
/// ve ötesinde en soluk hâlindedir (`FAINT`).
const NEAR: f32 = 24.0;
const FAR: f32 = 220.0;
const FAINT: f32 = 0.3;
/// Çubuğun belirmesi ve bu sırada seçimden uzaklaştığı mesafe.
const APPEAR: Duration = Duration::from_millis(140);
const RISE: f32 = 4.0;

/// Mini araç çubuğu.
pub struct MiniToolbar<'a, Message> {
    content: Element<'a, Message>,
    anchor: Option<Rectangle>,
    entries: Vec<Entry<Message>>,
}

enum Entry<Message> {
    Button {
        glyph: Icon,
        label: String,
        shortcut: Option<String>,
        on_press: Option<Message>,
        active: bool,
        danger: bool,
    },
    Separator,
}

impl<'a, Message: Clone + 'a> MiniToolbar<'a, Message> {
    /// `anchor` seçimin içeriğe göre sınırlarıdır; `None` çubuğu gizler.
    pub fn new(content: impl Into<Element<'a, Message>>, anchor: Option<Rectangle>) -> Self {
        Self {
            content: content.into(),
            anchor,
            entries: Vec::new(),
        }
    }

    /// Düğme; `on_press` yoksa devre dışıdır.
    pub fn button(
        mut self,
        glyph: Icon,
        label: impl Into<String>,
        on_press: impl Into<Option<Message>>,
    ) -> Self {
        self.entries.push(Entry::Button {
            glyph,
            label: label.into(),
            shortcut: None,
            on_press: on_press.into(),
            active: false,
            danger: false,
        });
        self
    }

    /// Son düğmenin kısayolu; ipucunda adın yanında yazar.
    pub fn shortcut(mut self, text: impl Into<String>) -> Self {
        if let Some(Entry::Button { shortcut, .. }) = self.entries.last_mut() {
            *shortcut = Some(text.into());
        }
        self
    }

    /// Son düğme açık bir ayarı gösterir (ör. kilitli katman): vurgulu
    /// çizilir.
    pub fn active(mut self, on: bool) -> Self {
        if let Some(Entry::Button { active, .. }) = self.entries.last_mut() {
            *active = on;
        }
        self
    }

    /// Son düğme geri alınamayan bir iştir (ör. silme): üzerine gelince
    /// kırmızıdır.
    pub fn danger(mut self) -> Self {
        if let Some(Entry::Button { danger, .. }) = self.entries.last_mut() {
            *danger = true;
        }
        self
    }

    /// Düğme grupları arasında ince çizgi.
    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }
}

/// Çubuğun öğesi. Düğmelerin ikonları ve ipuçları, düğmelerin sırasıyla
/// içerikten sonra gelir.
enum Item<Message> {
    Button {
        on_press: Option<Message>,
        active: bool,
        danger: bool,
    },
    Separator,
}

struct Bar<'a, Message> {
    /// İçerik; ardından her düğmenin ikonu ve ipucu.
    elements: Vec<Element<'a, Message>>,
    anchor: Option<Rectangle>,
    items: Vec<Item<Message>>,
}

impl<'a, Message: Clone + 'a> From<MiniToolbar<'a, Message>> for Element<'a, Message> {
    fn from(toolbar: MiniToolbar<'a, Message>) -> Self {
        let mut elements = vec![toolbar.content];
        let mut items = Vec::with_capacity(toolbar.entries.len());

        for entry in toolbar.entries {
            match entry {
                Entry::Button {
                    glyph,
                    label: name,
                    shortcut,
                    on_press,
                    active,
                    danger,
                } => {
                    let mut tip = row![label::caption(name).style(style::text::default)].spacing(8);

                    if let Some(shortcut) = shortcut {
                        tip = tip.push(label::mono_caption(shortcut));
                    }

                    elements.push(icon(glyph).into());
                    elements.push(
                        container(tip)
                            .padding([4, 8])
                            .style(style::container::popover)
                            .into(),
                    );
                    items.push(Item::Button {
                        on_press,
                        active,
                        danger,
                    });
                }
                Entry::Separator => items.push(Item::Separator),
            }
        }

        Element::new(Bar {
            elements,
            anchor: toolbar.anchor,
            items,
        })
    }
}

#[derive(Debug)]
struct State {
    /// Çubuk gösteriliyor mu (belirmeyi başlatmak için).
    shown: bool,
    /// Üzerinde durulan ve basılan düğme, öğe sırasıyla.
    hovered: Option<usize>,
    pressed: Option<usize>,
    /// Son çizilen saydamlık; imleç yaklaşıp uzaklaştıkça değişir.
    opacity: f32,
    /// Çubuk seçimin üstünde mi; gizliyse `None`.
    above: Option<bool>,
    /// Çubuğun belirdiği an ve son çizim anı.
    appeared: Option<Instant>,
    now: Instant,
}

/// Çubuğun yeri: seçimin üst ortası; üstte yer yoksa altı, o da yoksa
/// içeriğin üstü. İkinci değer çubuğun seçimin üstünde olup olmadığıdır.
fn place(bounds: Size, anchor: Rectangle, bar: Size) -> (Point, bool) {
    let x = (anchor.center_x() - bar.width / 2.0)
        .min(bounds.width - bar.width - GAP)
        .max(GAP);
    let above = anchor.y - OFFSET - bar.height;
    let below = anchor.y + anchor.height + OFFSET;

    if above >= GAP {
        (Point::new(x, above), true)
    } else if below + bar.height <= bounds.height - GAP {
        (Point::new(x, below), false)
    } else {
        (Point::new(x, GAP), true)
    }
}

/// Seçimin bir kısmı görünür alanda mı.
fn visible(bounds: Size, anchor: Rectangle) -> bool {
    anchor.x <= bounds.width
        && anchor.x + anchor.width >= 0.0
        && anchor.y <= bounds.height
        && anchor.y + anchor.height >= 0.0
}

/// İmlecin çubuğa uzaklığına göre saydamlık.
fn opacity(bar: Rectangle, cursor: Option<Point>) -> f32 {
    let Some(point) = cursor else {
        return FAINT;
    };

    let dx = (bar.x - point.x)
        .max(point.x - (bar.x + bar.width))
        .max(0.0);
    let dy = (bar.y - point.y)
        .max(point.y - (bar.y + bar.height))
        .max(0.0);
    let distance = dx.hypot(dy);
    let t = ((distance - NEAR) / (FAR - NEAR)).clamp(0.0, 1.0);

    1.0 - t * (1.0 - FAINT)
}

impl<'a, Message: Clone + 'a> Bar<'a, Message> {
    /// `index`. düğmenin ikonunun ve ipucunun öğe sırası.
    fn button_elements(&self, item: usize) -> Option<(usize, usize)> {
        let buttons = self.items[..item]
            .iter()
            .filter(|item| matches!(item, Item::Button { .. }))
            .count();

        matches!(self.items.get(item), Some(Item::Button { .. }))
            .then_some((1 + buttons * 2, 2 + buttons * 2))
    }

    /// İmlecin üzerinde olduğu düğme.
    fn hit(&self, bar: Layout<'_>, cursor: mouse::Cursor) -> Option<usize> {
        let point = cursor.position_over(bar.bounds())?;

        bar.children().zip(&self.items).position(|(cell, item)| {
            matches!(item, Item::Button { .. }) && cell.bounds().contains(point)
        })
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Bar<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let now = Instant::now();

        tree::State::new(State {
            shown: self.anchor.is_some(),
            hovered: None,
            pressed: None,
            opacity: 1.0,
            above: None,
            appeared: self.anchor.is_some().then_some(now),
            now,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.elements.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.elements);

        let state = tree.state.downcast_mut::<State>();
        let shown = self.anchor.is_some();

        if shown && !state.shown {
            state.appeared = Some(Instant::now());
        }

        if !shown {
            state.hovered = None;
            state.pressed = None;
        }

        state.shown = shown;
    }

    fn size(&self) -> Size<Length> {
        self.elements[0].as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let content =
            self.elements[0]
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits);
        let size = content.size();
        let item = typography::scaled(ITEM).round();
        let mut nodes = vec![content];
        let mut cells = Vec::with_capacity(self.items.len());
        let mut tips = Vec::new();
        let mut x = PAD;

        for index in 0..self.items.len() {
            match self.button_elements(index) {
                Some((glyph, tip)) => {
                    let glyph_node = self.elements[glyph].as_widget_mut().layout(
                        &mut tree.children[glyph],
                        renderer,
                        &layout::Limits::new(Size::ZERO, Size::new(item, item)),
                    );
                    let glyph_size = glyph_node.size();

                    cells.push(
                        Node::with_children(
                            Size::new(item, item),
                            vec![glyph_node.move_to(Point::new(
                                ((item - glyph_size.width) / 2.0).round(),
                                ((item - glyph_size.height) / 2.0).round(),
                            ))],
                        )
                        .move_to(Point::new(x, PAD)),
                    );
                    tips.push(self.elements[tip].as_widget_mut().layout(
                        &mut tree.children[tip],
                        renderer,
                        &layout::Limits::new(Size::ZERO, size),
                    ));
                    x += item;
                }
                None => {
                    cells.push(Node::new(Size::new(SEPARATOR, item)).move_to(Point::new(x, PAD)));
                    x += SEPARATOR;
                }
            }
        }

        let bar = Size::new(x + PAD, item + PAD * 2.0);
        let state = tree.state.downcast_mut::<State>();

        match self.anchor.filter(|anchor| visible(size, *anchor)) {
            Some(anchor) => {
                let (position, above) = place(size, anchor, bar);

                state.above = Some(above);
                nodes.push(Node::with_children(bar, cells).move_to(position));
            }
            None => {
                state.above = None;
                nodes.push(Node::new(Size::ZERO));
            }
        }

        nodes.extend(tips);

        Node::with_children(size, nodes)
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
        let mut children = layout.children();
        let (Some(content), Some(bar)) = (children.next(), children.next()) else {
            return;
        };
        let shown = tree.state.downcast_ref::<State>().above.is_some();
        let over = shown && cursor.is_over(bar.bounds());
        let hovered = if shown { self.hit(bar, cursor) } else { None };
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                state.now = *now;

                if let Some(appeared) = state.appeared {
                    if motion::running(appeared, *now, APPEAR) {
                        shell.request_redraw();
                    } else {
                        state.appeared = None;
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if shown => {
                let faded = opacity(bar.bounds(), cursor.position());

                if hovered != state.hovered || (faded - state.opacity).abs() > 0.01 {
                    state.hovered = hovered;
                    state.opacity = faded;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(button)) if over => {
                state.pressed = hovered.filter(|_| *button == mouse::Button::Left);
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.pressed.is_some() =>
            {
                let pressed = state.pressed.take();

                if pressed == hovered
                    && let Some(Item::Button {
                        on_press: Some(message),
                        ..
                    }) = pressed.and_then(|index| self.items.get(index))
                {
                    shell.publish(message.clone());
                }

                shell.capture_event();
                shell.request_redraw();
                return;
            }
            Event::Mouse(mouse::Event::WheelScrolled { .. }) if over => {
                shell.capture_event();
                return;
            }
            _ => {}
        }

        self.elements[0].as_widget_mut().update(
            &mut tree.children[0],
            event,
            content,
            if over { cursor.levitate() } else { cursor },
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
        let mut children = layout.children();
        let (Some(content), Some(bar)) = (children.next(), children.next()) else {
            return mouse::Interaction::None;
        };

        if tree.state.downcast_ref::<State>().above.is_some() && cursor.is_over(bar.bounds()) {
            return match self
                .hit(bar, cursor)
                .and_then(|index| self.items.get(index))
            {
                Some(Item::Button {
                    on_press: Some(_), ..
                }) => mouse::Interaction::Pointer,
                _ => mouse::Interaction::Idle,
            };
        }

        self.elements[0].as_widget().mouse_interaction(
            &tree.children[0],
            content,
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
        let state = tree.state.downcast_ref::<State>();
        let mut children = layout.children();
        let (Some(content), Some(bar)) = (children.next(), children.next()) else {
            return;
        };
        let tips: Vec<Layout<'_>> = children.collect();
        let over = state.above.is_some() && cursor.is_over(bar.bounds());

        self.elements[0].as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content,
            if over { cursor.levitate() } else { cursor },
            viewport,
        );

        let Some(above) = state.above else {
            return;
        };

        let t = Tokens::of(theme);
        let appear = state.appeared.map_or(1.0, |appeared| {
            motion::progress(appeared, state.now, APPEAR)
        });
        let alpha = opacity(bar.bounds(), cursor.position()) * appear;
        let hovered = if over { state.hovered } else { None };
        // Belirirken seçimden uzaklaşarak yerine oturur.
        let rise = (1.0 - appear) * RISE * if above { 1.0 } else { -1.0 };
        let clip = layout.bounds();

        renderer.with_layer(clip, |renderer| {
            renderer.with_translation(Vector::new(0.0, rise), |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: bar.bounds(),
                        border: Border {
                            color: t.border.scale_alpha(alpha),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        shadow: Shadow {
                            color: t.shadow().scale_alpha(alpha),
                            offset: Vector::new(0.0, 2.0),
                            blur_radius: 8.0,
                        },
                        ..Quad::default()
                    },
                    Background::Color(t.popover.scale_alpha(alpha)),
                );

                for (index, (cell, item)) in bar.children().zip(&self.items).enumerate() {
                    let bounds = cell.bounds();

                    let Item::Button {
                        on_press,
                        active,
                        danger,
                    } = item
                    else {
                        renderer.fill_quad(
                            Quad {
                                bounds: Rectangle::new(
                                    Point::new(bounds.center_x().floor(), bounds.y + 6.0),
                                    Size::new(1.0, bounds.height - 12.0),
                                ),
                                ..Quad::default()
                            },
                            Background::Color(t.border.scale_alpha(alpha)),
                        );
                        continue;
                    };

                    let enabled = on_press.is_some();
                    let is_hovered = enabled && hovered == Some(index);
                    let pressed = is_hovered && state.pressed == Some(index);
                    let highlight = match (is_hovered, *danger, *active) {
                        (true, true, _) => {
                            Some(t.danger.scale_alpha(if pressed { 0.3 } else { 0.2 }))
                        }
                        (true, false, _) => Some(if pressed {
                            t.layer(0.14)
                        } else {
                            t.surface_hover
                        }),
                        (false, _, true) => Some(t.accent.scale_alpha(0.2)),
                        (false, _, false) => None,
                    };

                    if let Some(color) = highlight {
                        renderer.fill_quad(
                            Quad {
                                bounds,
                                border: Border {
                                    radius: 4.0.into(),
                                    ..Border::default()
                                },
                                ..Quad::default()
                            },
                            Background::Color(color.scale_alpha(alpha)),
                        );
                    }

                    let color = if !enabled {
                        t.disabled()
                    } else if is_hovered && *danger {
                        t.danger
                    } else if *active {
                        t.accent_hover
                    } else {
                        t.text
                    };

                    if let (Some((glyph, _)), Some(glyph_layout)) =
                        (self.button_elements(index), cell.children().next())
                    {
                        self.elements[glyph].as_widget().draw(
                            &tree.children[glyph],
                            renderer,
                            theme,
                            &renderer::Style {
                                text_color: color.scale_alpha(alpha),
                            },
                            glyph_layout,
                            cursor,
                            viewport,
                        );
                    }
                }
            });
        });

        // Üzerinde durulan düğmenin adı: çubuğun seçimden uzak yanında;
        // sığmazsa öbür yanında.
        let Some(index) = hovered else {
            return;
        };
        let (Some((_, tip)), Some(cell)) = (self.button_elements(index), bar.children().nth(index))
        else {
            return;
        };
        let buttons_before = self.items[..index]
            .iter()
            .filter(|item| matches!(item, Item::Button { .. }))
            .count();
        let Some(tip_layout) = tips.get(buttons_before) else {
            return;
        };

        let size = tip_layout.bounds().size();
        let cell = cell.bounds();
        let top = bar.bounds().y - TIP_GAP - size.height;
        let bottom = bar.bounds().y + bar.bounds().height + TIP_GAP;
        let y = match above {
            true if top >= clip.y => top,
            true => bottom,
            false if bottom + size.height <= clip.y + clip.height => bottom,
            false => top,
        };
        let x = (cell.center_x() - size.width / 2.0)
            .min(clip.x + clip.width - size.width - 2.0)
            .max(clip.x + 2.0);
        let origin = tip_layout.bounds().position();

        renderer.with_layer(clip, |renderer| {
            renderer.with_translation(Vector::new(x - origin.x, y - origin.y), |renderer| {
                self.elements[tip].as_widget().draw(
                    &tree.children[tip],
                    renderer,
                    theme,
                    style,
                    *tip_layout,
                    mouse::Cursor::Unavailable,
                    viewport,
                );
            });
        });
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(content) = layout.children().next() {
            self.elements[0].as_widget_mut().operate(
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

        self.elements[0].as_widget_mut().overlay(
            &mut tree.children[0],
            content,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Size = Size::new(800.0, 600.0);
    const BAR: Size = Size::new(120.0, 34.0);

    #[test]
    fn the_bar_sits_above_the_selection_and_stays_inside() {
        let selection = Rectangle::new(Point::new(300.0, 200.0), Size::new(100.0, 60.0));

        assert_eq!(
            place(AREA, selection, BAR),
            (Point::new(290.0, 200.0 - OFFSET - 34.0), true)
        );

        // Üstte yer yok: altına geçer.
        let top = Rectangle::new(Point::new(300.0, 20.0), Size::new(100.0, 60.0));
        assert_eq!(
            place(AREA, top, BAR),
            (Point::new(290.0, 80.0 + OFFSET), false)
        );

        // Kenarlardan taşmaz.
        let edge = Rectangle::new(Point::new(-40.0, 200.0), Size::new(60.0, 20.0));
        assert_eq!(place(AREA, edge, BAR).0.x, GAP);

        let right = Rectangle::new(Point::new(780.0, 200.0), Size::new(60.0, 20.0));
        assert_eq!(place(AREA, right, BAR).0.x, AREA.width - BAR.width - GAP);

        // Seçim bütün yüksekliği kaplıyor: üstte kalır.
        let tall = Rectangle::new(Point::new(300.0, -10.0), Size::new(100.0, 700.0));
        assert_eq!(place(AREA, tall, BAR), (Point::new(290.0, GAP), true));
    }

    #[test]
    fn selections_outside_the_area_hide_the_bar() {
        assert!(visible(
            AREA,
            Rectangle::new(Point::new(10.0, 10.0), Size::ZERO)
        ));
        assert!(!visible(
            AREA,
            Rectangle::new(Point::new(-200.0, 10.0), Size::new(100.0, 50.0))
        ));
        assert!(!visible(
            AREA,
            Rectangle::new(Point::new(10.0, 700.0), Size::new(100.0, 50.0))
        ));
    }

    #[test]
    fn the_bar_fades_as_the_cursor_moves_away() {
        let bar = Rectangle::new(Point::new(100.0, 100.0), BAR);

        assert_eq!(opacity(bar, Some(Point::new(150.0, 110.0))), 1.0);
        assert_eq!(
            opacity(bar, Some(Point::new(150.0, 100.0 + 34.0 + NEAR))),
            1.0
        );
        assert_eq!(opacity(bar, Some(Point::new(700.0, 500.0))), FAINT);
        assert_eq!(opacity(bar, None), FAINT);

        let halfway = opacity(
            bar,
            Some(Point::new(100.0 + 120.0 + (NEAR + FAR) / 2.0, 110.0)),
        );
        assert!((halfway - (1.0 + FAINT) / 2.0).abs() < 1e-3);
    }
}

/// Gerçek olaylarla: düğmeye tıklama, devre dışı düğme ve çubuğun altına
/// tıklamayı engellemesi.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::widget::{button, space};
    use iced::{Element, Fill, Point, Rectangle, Size};

    use super::MiniToolbar;
    use crate::icon::Icon;
    use crate::snapshot::{Input, Snapshot};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Pressed {
        Focus,
        Delete,
        Stage,
    }

    type Presses = Vec<Pressed>;

    fn view(_presses: &Presses) -> Element<'_, Pressed> {
        let stage = button(space::horizontal().width(Fill).height(Fill))
            .on_press(Pressed::Stage)
            .width(Fill)
            .height(Fill);

        MiniToolbar::new(
            stage,
            Some(Rectangle::new(
                Point::new(150.0, 150.0),
                Size::new(100.0, 50.0),
            )),
        )
        .button(Icon::Target, "Yakınlaştır", Pressed::Focus)
        .button(Icon::Copy, "Kopyala", None)
        .separator()
        .button(Icon::Close, "Sil", Pressed::Delete)
        .danger()
        .into()
    }

    #[test]
    fn buttons_publish_and_the_bar_blocks_the_content_below() {
        let mut snapshot = Snapshot::new(Size::new(400.0, 300.0)).expect("çizici kurulamadı");
        let mut presses: Presses = Vec::new();
        let mut update = |presses: &mut Presses, next| presses.push(next);
        let mut input =
            |presses: &mut Presses, input| snapshot.input(presses, view, &mut update, input);

        // Düğmeler 28 piksel; çubuk seçimin 10 piksel üstünde, ortada.
        let item = crate::theme::typography::scaled(super::ITEM).round();
        let bar_width = super::PAD * 2.0 + item * 3.0 + super::SEPARATOR;
        let left = 200.0 - bar_width / 2.0;
        let y = 150.0 - super::OFFSET - super::PAD - item / 2.0;
        let cell = |index: f32, separators: f32| {
            Point::new(
                left + super::PAD + index * item + separators * super::SEPARATOR + item / 2.0,
                y,
            )
        };

        input(&mut presses, Input::Click(cell(0.0, 0.0)));
        assert_eq!(presses, [Pressed::Focus]);

        // Devre dışı düğme bir şey yapmaz, alttaki içeriğe de geçmez.
        input(&mut presses, Input::Click(cell(1.0, 0.0)));
        assert_eq!(presses, [Pressed::Focus]);

        input(&mut presses, Input::Click(cell(2.0, 1.0)));
        assert_eq!(presses, [Pressed::Focus, Pressed::Delete]);

        // Çubuğun dışı içeriğindir.
        input(&mut presses, Input::Click(Point::new(40.0, 260.0)));
        assert_eq!(presses.last(), Some(&Pressed::Stage));
    }
}
