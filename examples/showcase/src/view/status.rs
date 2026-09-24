//! Durum çubuğu: imleç koordinatı, görüntüleme anahtarları ve ölçek.

use iced::Element;

use kentos_rc::label;
use kentos_rc::spatial::format;
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::widget::StatusBar;
use kentos_rc::widget::status_bar::Toggle;

use crate::app::Showcase;
use crate::message::{Message, Setting};

impl Showcase {
    pub(super) fn status_bar(&self) -> Element<'_, Message> {
        let (decimal, dms) = match self.cursor {
            Some(location) => (
                format!("{:>9.5}, {:>9.5}", location.lat, location.lon),
                format::coordinates(location),
            ),
            None => ("Model alanı dışında".to_owned(), String::new()),
        };

        let bar = StatusBar::new()
            .push(label::mono(decimal).width(156))
            .push(label::mono_caption(dms).width(196))
            .separator()
            .push(label::caption("MODEL").font(typography::UI_STRONG))
            .separator();

        let bar = Setting::ALL.into_iter().fold(bar, |bar, setting| {
            bar.push(
                Toggle::new(setting.label(), self.setting(setting))
                    .shortcut(setting.shortcut())
                    .on_press(Message::Toggle(setting)),
            )
        });

        bar.spacer()
            .push(
                label::mono_caption(format!(
                    "1:{}",
                    format::integer(self.viewport.scale_denominator())
                ))
                .style(style::text::default),
            )
            .separator()
            .push(label::mono_caption(format!("z {:.2}", self.viewport.zoom)))
            .separator()
            .push(label::mono_caption("EPSG:3857"))
            .into()
    }
}
