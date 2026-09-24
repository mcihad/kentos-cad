//! Geri bildirim sayfası: bildirimler.

use iced::widget::{Row, button};
use iced::{Center, Element};

use kentos_rc::label;
use kentos_rc::style;

use super::entry;
use crate::app::Showcase;
use crate::gallery::{Demo, sample_toasts};
use crate::message::Message;

impl Showcase {
    pub(super) fn feedback_page(&self) -> Vec<Element<'_, Message>> {
        let samples = sample_toasts();
        let all = samples.len();

        let buttons = samples
            .into_iter()
            .enumerate()
            .map(|(index, (name, _))| demo_button(name, index))
            .chain(std::iter::once(demo_button("Hepsi birden", all)));

        vec![entry(
            "Bildirimler",
            "kentos_rc::widget::Toaster",
            "Alanın sağ alt köşesinde üst üste dizilen kısa iletiler. İkonun ve alttaki kalan \
             süre çizgisinin rengi önem düzeyidir. Bilgi ve başarı 5, uyarı 8 saniyede kapanır; \
             eylemli bildirim en az 8 saniye durur, hata kendiliğinden kapanmaz. İmleç \
             üzerindeyken süre durur. En fazla üç bildirim görünür, eskiler sayılır; aynı \
             bildirim yinelenirse sayısı artar. Düğmelerle deneyin: bildirimler bu sayfanın \
             sağ alt köşesinde açılır.",
            Row::with_children(buttons).spacing(6).align_y(Center),
            Some(
                "self.toasts.push(\n    \
                 Toast::success(\"Çizim silindi\").action(\"Geri al\", Message::UndoDelete),\n);\n\n\
                 Toaster::new(map, &self.toasts, Message::ToastClosed)\n\n\
                 // update\n\
                 Message::ToastClosed(id) => self.toasts.dismiss(id),",
            ),
        )]
    }
}

fn demo_button<'a>(name: &'a str, index: usize) -> Element<'a, Message> {
    button(label::body(name))
        .on_press(Message::Gallery(Demo::Notify(index)))
        .padding([4, 12])
        .style(style::button::secondary)
        .into()
}
