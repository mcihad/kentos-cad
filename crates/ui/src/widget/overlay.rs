//! Pencerenin üstüne açılan katmanlar: açılır paneller ve iletişim kutuları.
//!
//! Buradaki fonksiyonlar tüm pencereyi kaplayan bir katman döndürür;
//! uygulama bunu ana görünümün üstüne yığar:
//!
//! ```ignore
//! stack![base, overlay::modal(dialog, Message::CloseDialog)]
//! ```
//!
//! Katmanın boş alanı imleç türü bildirir; böylece alttaki bileşenler (ör.
//! model alanının artı imleci) katman açıkken imleci görmez. Kutunun kendisi
//! tıklamaları yutar (`opaque`): içindeki yazıya ya da boşluğa tıklamak
//! arkadaki kapatma alanına geçmez, kutuyu kapatmaz.

use iced::widget::{container, mouse_area, opaque, space, stack};
use iced::{Center, Element, Fill, Left, Padding, Point, Top, mouse};

use crate::style;

/// `content`'i pencerenin `anchor` noktasına yerleştirir. İçeriğin dışına
/// tıklamak `on_dismiss` mesajını üretir.
pub fn popover<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    anchor: Point,
    on_dismiss: Message,
) -> Element<'a, Message> {
    let positioned = container(opaque(content))
        .width(Fill)
        .height(Fill)
        .padding(Padding {
            top: anchor.y,
            right: 0.0,
            bottom: 0.0,
            left: anchor.x,
        })
        .align_x(Left)
        .align_y(Top);

    stack![dismiss_area(on_dismiss, false), positioned].into()
}

/// `content`'i pencerenin ortasına, arkasını karartarak yerleştirir.
/// Karartılmış alana tıklamak `on_dismiss` mesajını üretir.
pub fn modal<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_dismiss: Message,
) -> Element<'a, Message> {
    let centered = container(opaque(content))
        .width(Fill)
        .height(Fill)
        .align_x(Center)
        .align_y(Center);

    stack![dismiss_area(on_dismiss, true), centered].into()
}

/// `content`'i ortalar ve arkasını karartır; karartılmış alana tıklamak
/// kutuyu kapatmaz. İlerlemesi ya da taslağı olan kutular (sihirbaz,
/// özellikler penceresi) yanlışlıkla kapanmasın diye: kutu yalnızca kendi
/// düğmeleriyle ya da Esc ile kapanır.
pub fn blocking<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let centered = container(opaque(content))
        .width(Fill)
        .height(Fill)
        .align_x(Center)
        .align_y(Center);

    let scrim = mouse_area(
        container(space::vertical())
            .width(Fill)
            .height(Fill)
            .style(style::container::scrim),
    )
    .interaction(mouse::Interaction::Idle);

    stack![scrim, centered].into()
}

fn dismiss_area<'a, Message: Clone + 'a>(on_dismiss: Message, scrim: bool) -> Element<'a, Message> {
    let area = container(space::vertical()).width(Fill).height(Fill);
    let area = if scrim {
        area.style(style::container::scrim)
    } else {
        area
    };

    mouse_area(area)
        .interaction(mouse::Interaction::Idle)
        .on_press(on_dismiss)
        .into()
}

/// Gerçek tıklamalarla: kutunun içi kutuyu kapatmaz, karartılmış alan kapatır.
#[cfg(all(test, feature = "snapshot"))]
mod clicks {
    use iced::widget::{button, column, container, text};
    use iced::{Element, Point, Size};

    use super::{modal, popover};
    use crate::snapshot::{Input, Snapshot};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Dismissed,
        Pressed,
    }

    struct Open {
        popover: bool,
        messages: Vec<Message>,
    }

    impl Open {
        fn view(&self) -> Element<'_, Message> {
            // 300×200 box with a line of text and a button, centred in 800×600
            // (or at 100, 100 for the popover).
            let content = container(
                column![
                    button(text("Düğme")).on_press(Message::Pressed),
                    text("Yalnız yazı: tıklamak kutuyu kapatmamalı."),
                ]
                .spacing(20),
            )
            .width(300)
            .height(200)
            .padding(10);
            if self.popover {
                popover(content, Point::new(100.0, 100.0), Message::Dismissed)
            } else {
                modal(content, Message::Dismissed)
            }
        }
    }

    fn clicks(popover: bool, at: Point) -> Vec<Message> {
        let mut snapshot = Snapshot::new(Size::new(800.0, 600.0)).expect("çizici kurulamadı");
        let mut open = Open {
            popover,
            messages: Vec::new(),
        };
        let mut update = |open: &mut Open, message| open.messages.push(message);
        snapshot.settle(&mut open, Open::view, &mut update);
        snapshot.input(&mut open, Open::view, &mut update, Input::Click(at));
        open.messages
    }

    #[test]
    fn a_click_inside_the_box_does_not_dismiss_it() {
        // The modal's box spans 250–550 × 200–400: its button at the top
        // left, its text under it, an empty corner.
        assert_eq!(clicks(false, Point::new(275.0, 222.0)), [Message::Pressed]);
        assert_eq!(clicks(false, Point::new(300.0, 265.0)), []);
        assert_eq!(clicks(false, Point::new(540.0, 390.0)), []);
        // The popover's box spans 100–400 × 100–300.
        assert_eq!(clicks(true, Point::new(150.0, 165.0)), []);
        assert_eq!(clicks(true, Point::new(390.0, 290.0)), []);
    }

    #[test]
    fn a_click_outside_the_box_dismisses_it() {
        assert_eq!(clicks(false, Point::new(50.0, 50.0)), [Message::Dismissed]);
        assert_eq!(clicks(true, Point::new(600.0, 500.0)), [Message::Dismissed]);
    }
}
