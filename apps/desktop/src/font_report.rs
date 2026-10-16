//! Yazı tipleri (docs/adr/0324 §5; `text.fonts`): the drawing's text styles, the typeface each names, the face it is
//! drawn in and how, and the faces a sheet's PDF embeds, as `cad.fonts.report` answers them. A style drawn in a
//! substitute is said on a warning line: the change is never silent.

use iced::widget::{Column, container};
use iced::{Element, Fill, Task};
use kentos_contracts::{FontMatch, FontsReport};
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Yazı tipleri";
const CLOSE: &str = "Kapat";

/// How a match reads.
pub fn how_text(how: FontMatch) -> &'static str {
    match how {
        FontMatch::Own => "Kendisi",
        FontMatch::Equivalent => "Eşdeğeri",
        FontMatch::Shx => "SHX, yerine",
        FontMatch::Missing => "Yok, yerine",
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Close,
}

fn msg(event: Event) -> Message {
    Message::FontReport(event)
}

impl App {
    /// Yazı tipleri (`text.fonts`): the window.
    pub(crate) fn open_font_report(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.dialog = Some(Dialog::FontReport);
    }

    pub(crate) fn font_report_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Close => self.dialog = None,
        }
        Task::none()
    }

    pub(crate) fn font_report_view(&self) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::caption("Açık çizim yok.")).into();
        };
        let (report, warnings) =
            kentos_native_application::fonts_report::report(&doc.model, &FontsReport::default());
        let header = [
            "Stil",
            "Adlandırdığı dosya",
            "Çizildiği yazı tipi",
            "Nasıl",
            "Yazı",
        ];
        let widths: [f32; 5] = [150.0, 170.0, 150.0, 100.0, 56.0];
        let columns = header.iter().zip(widths).enumerate().map(|(i, (h, w))| {
            let c = table::Column::new(*h).width(w);
            if i == 4 { c.align_right() } else { c }
        });
        let body: Vec<table::Row<'_, Message>> = report
            .styles
            .iter()
            .map(|s| {
                table::Row::new(
                    [
                        s.name.clone(),
                        s.file.clone().unwrap_or_else(|| "—".to_owned()),
                        s.face.label().to_owned(),
                        how_text(s.how).to_owned(),
                        s.texts.to_string(),
                    ]
                    .into_iter()
                    .map(|c| label::caption(c).into()),
                )
            })
            .collect();
        let list = container(
            Table::new(columns)
                .extend(body)
                .min_width(widths.iter().sum::<f32>() + 8.0 * 5.0 + 20.0)
                .height(220.0),
        )
        .style(style::container::field_box)
        .width(Fill);
        let faces: Vec<&str> = report.faces.iter().map(|f| f.label()).collect();
        let mut lines = vec![
            words::text_line(
                Kind::Info,
                format!(
                    "Projenin yazı tipi {}. KentOS yalnız kendi yedi yazı tipiyle çizer; Arial ve Helvetica Arimo'yla, Courier Courier Prime'la çizilir.",
                    report.project.label()
                ),
            ),
            words::text_line(
                Kind::Info,
                if faces.is_empty() {
                    "Çizimde yazı yok: paftanın PDF'i yazı tipi gömmez.".to_owned()
                } else {
                    format!(
                        "Paftanın PDF'i bu yazı tiplerinin kullanılan harflerini gömer: {}.",
                        faces.join(", ")
                    )
                },
            ),
        ];
        lines.extend(
            warnings
                .into_iter()
                .map(|w| words::text_line(Kind::Warn, w.message)),
        );
        if report.styles.is_empty() {
            lines.push(words::text_line(Kind::Info, "Çizimde yazı stili yok."));
        }
        overlay::modal(
            Frame::new(TITLE)
                .push(
                    Column::new()
                        .spacing(12)
                        .push(words::summary(lines))
                        .push(list),
                )
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(720.0),
            msg(Event::Close),
        )
    }
}
