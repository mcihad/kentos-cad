//! The sheet mode put together (design §11): the sheets and items on the
//! left, the paper in the middle, the inspector on the right; the windows
//! over them. The host places these where its drawing area and docks are,
//! the tabs under them and the cells in its status bar.

use iced::widget::{button, column, container, row};
use iced::{Element, Fill};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::{Dialog as Window, Segmented, overlay, vertical_divider};

use crate::designer::{Designer, Dialog};
use crate::message::{ExportKind, Message};
use crate::painter::Painter;

/// The PNG resolutions offered.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Dpi(u16);

impl std::fmt::Display for Dpi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} dpi", self.0)
    }
}

impl Designer {
    /// The sheets and items, the paper, the inspector.
    pub fn body<'a>(&'a self, painter: Painter<'a>) -> Element<'a, Message> {
        row![
            container(self.left_panel())
                .height(Fill)
                .style(style::container::surface),
            vertical_divider(),
            container(self.stage(painter)).width(Fill).height(Fill),
            vertical_divider(),
            container(self.inspector_panel())
                .height(Fill)
                .style(style::container::surface),
        ]
        .height(Fill)
        .into()
    }

    /// The window over the mode, if one is open (the gallery, saving a
    /// template, an export's questions). The gallery's pictures show the
    /// project's drawing through `painter`, as a sheet made from them would.
    pub fn window<'a>(&'a self, painter: Painter<'a>) -> Option<Element<'a, Message>> {
        if let Some(g) = &self.gallery {
            let gallery = self.gallery_window(g, painter);
            // Kullan's questions over the gallery: leaving them goes back to it.
            if let Some(Dialog::Questions(d)) = &self.dialog {
                return Some(
                    iced::widget::stack![
                        gallery,
                        overlay::modal(self.questions_view(d), Message::CloseDialog)
                    ]
                    .into(),
                );
            }
            return Some(gallery);
        }
        let inner: Element<'_, Message> = match self.dialog.as_ref()? {
            Dialog::SaveTemplate(d) => self.save_template_view(d),
            Dialog::Questions(d) => self.questions_view(d),
            Dialog::Variables(d) => self.variables_view(d),
            Dialog::Binding(d) => self.binding_view(d),
            Dialog::Import {
                label,
                sheets,
                have,
                ..
            } => self.import_view(label, *sheets, *have),
            Dialog::Export {
                kind: ExportKind::Pdf,
                dpi,
                errors,
                print,
            } => {
                let ways = self.pdf_ways(&*painter);
                self.pdf_view(*dpi, *errors, *print, &ways)
            }
            Dialog::Export {
                kind, dpi, errors, ..
            } => self.export_view(*kind, *dpi, *errors),
        };
        Some(overlay::modal(inner, Message::CloseDialog))
    }

    fn import_view(&self, file: &str, sheets: usize, have: usize) -> Element<'_, Message> {
        Window::new(".kpafta dosyasından")
            .push(
                column![
                    label::body(format!("“{file}” dosyasında {sheets} pafta var. Bu projenin {have} paftasının yanına mı eklensin, yerine mi?")),
                    label::caption("Yerine konursa projenin paftaları kalkar; Geri al (Ctrl+Z) onları geri getirir.").style(style::text::muted),
                ]
                .spacing(8),
            )
            .aside(button(label::body("Vazgeç")).on_press(Message::CloseDialog).padding([6, 14]).style(style::button::secondary))
            .action(button(label::body("Yerine koy")).on_press(Message::ImportChoose(true)).padding([6, 14]).style(style::button::secondary))
            .action(button(label::body("Yanına ekle")).on_press(Message::ImportChoose(false)).padding([6, 16]).style(style::button::primary))
            .width(460.0)
            .into()
    }

    fn export_view(&self, kind: ExportKind, dpi: u16, errors: usize) -> Element<'_, Message> {
        let title = match kind {
            ExportKind::Pdf => "PDF olarak dışa aktar",
            ExportKind::Svg => "SVG olarak dışa aktar",
            ExportKind::Png => "PNG olarak dışa aktar",
            ExportKind::Kpafta => "Pafta dosyası olarak kaydet",
        };
        let mut body = column![].spacing(10);
        if kind == ExportKind::Png {
            let size = self
                .png_size(dpi)
                .map_or_else(String::new, |(w, h)| format!("{w} × {h} piksel"));
            body = body
                .push(label::caption("Çözünürlük").style(style::text::muted))
                .push(
                    Segmented::new(
                        [Dpi(96), Dpi(150), Dpi(200), Dpi(300)],
                        Dpi(dpi),
                        |d: Dpi| Message::ExportDpi(d.0),
                    )
                    .width(Fill),
                )
                .push(label::caption(size).style(style::text::muted));
        }
        if errors > 0 {
            body = body.push(
                container(label::body(format!(
                    "Ön denetim {errors} hata buldu. Denetçinin Ön denetim sekmesinde çözümleriyle görebilirsiniz; yine de aktarabilirsiniz."
                )))
                .padding(8)
                .style(style::container::bordered),
            );
        }
        let go = if errors > 0 {
            "Yine de aktar"
        } else {
            "Aktar…"
        };
        Window::new(title)
            .push(body)
            .action(
                button(label::body("Vazgeç"))
                    .on_press(Message::CloseDialog)
                    .padding([6, 14])
                    .style(style::button::secondary),
            )
            .action(
                button(label::body(go))
                    .on_press(Message::ExportAnyway)
                    .padding([6, 16])
                    .style(if errors > 0 {
                        style::button::danger
                    } else {
                        style::button::primary
                    }),
            )
            .width(440.0)
            .into()
    }
}
