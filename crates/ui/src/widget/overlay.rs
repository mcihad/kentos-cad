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
//! model alanının artı imleci) katman açıkken imleci görmez.

use iced::widget::{container, mouse_area, space, stack};
use iced::{Center, Element, Fill, Left, Padding, Point, Top, mouse};

use crate::style;

/// `content`'i pencerenin `anchor` noktasına yerleştirir. İçeriğin dışına
/// tıklamak `on_dismiss` mesajını üretir.
pub fn popover<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    anchor: Point,
    on_dismiss: Message,
) -> Element<'a, Message> {
    let positioned = container(content)
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
    let centered = container(content)
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
    let centered = container(content)
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
