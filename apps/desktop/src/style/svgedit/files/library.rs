//! Kitaplıktan aç (the web opens Stil yöneticisi to pick): the library's SVG
//! drawings as cards with their pictures, searched by name or category, in
//! a window over the editor (the editor may itself stand over Stil
//! yöneticisi, so it keeps its own picker). A system drawing opens as the
//! user's copy.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space, text_input};
use iced::{Center, Element, Fill, Length};
use kentos_native_style::library::{ItemKind, Source};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog;

use super::super::{Event as EdEvent, change, ev};
use super::{Event, FileDialog};
use crate::app::{App, Message};
use crate::style::manager::details::symbol_of_item;
use crate::style::thumbs::Look;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picker {
    pub query: String,
}

/// Cards on a row, and the picture's side.
const PER_ROW: usize = 5;
const PICTURE: (f32, f32) = (72.0, 72.0);

impl App {
    pub(crate) fn svgedit_library_view(&self, picker: &Picker, width: f32) -> Element<'_, Message> {
        let lib = &self.styles.library;
        let palette = self.style_palette();
        let look = Look {
            palette: &palette,
            library: lib,
            images: &self.styles.images,
        };
        let q = kentos_interaction::upper_tr(kentos_processing::text::js_trim(&picker.query)).to_lowercase();
        let current = self
            .styles
            .svg_editor
            .as_ref()
            .and_then(|e| e.original.as_ref().map(|o| o.id.clone()));
        let items: Vec<_> = lib
            .items(None)
            .into_iter()
            .filter(|(item, _)| item.kind() == ItemKind::Asset && item.format() == Some("svg"))
            .filter(|(item, _)| {
                q.is_empty()
                    || kentos_interaction::upper_tr(item.name()).to_lowercase().contains(&q)
                    || item
                        .path()
                        .iter()
                        .any(|p| kentos_interaction::upper_tr(p).to_lowercase().contains(&q))
            })
            .collect();
        let count = items.len();
        let mut grid = Column::new().spacing(10);
        for chunk in items.chunks(PER_ROW) {
            let mut line = Row::new().spacing(10);
            for (item, source) in chunk {
                let id = item.id().to_owned();
                let chosen = current.as_deref() == Some(item.id());
                let tag = match source {
                    Source::System => "Sistem",
                    Source::User => "Kitaplığım",
                    Source::Project => "Proje",
                };
                let card = column![
                    self.styles.thumbs.picture(&symbol_of_item(item), None, PICTURE, None, &look),
                    container(
                        label::caption(item.name().to_owned())
                            .align_x(iced::alignment::Horizontal::Center)
                            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                    )
                    .center_x(Fill)
                    .height(Length::Fixed(typography::scaled(32.0)))
                    .clip(true),
                    label::caption(tag).style(ui_style::text::muted),
                ]
                .spacing(4)
                .align_x(Center);
                line = line.push(
                    button(container(card).padding(6).width(Fill))
                        .width(Length::FillPortion(1))
                        .style(ui_style::button::tool(chosen))
                        .on_press(ev(EdEvent::File(Event::OpenAsset(id)))),
                );
            }
            for _ in chunk.len()..PER_ROW {
                line = line.push(space().width(Length::FillPortion(1)));
            }
            grid = grid.push(line);
        }
        let list: Element<'_, Message> = if count == 0 {
            container(label::body("Aranan adda SVG çizimi yok.").style(ui_style::text::muted))
                .center_x(Fill)
                .padding(30)
                .into()
        } else {
            scrollable(grid)
                .height(Length::Fixed(typography::scaled(420.0)))
                .direction(ui_style::field::body_scrollbar())
                .into()
        };
        let search = text_input("Ad ya da kategori ara", &picker.query)
            .id(iced::widget::Id::from("svge:libsearch"))
            .on_input(|t| {
                change(move |ed| {
                    if let Some(FileDialog::Library(p)) = &mut ed.files.dialog {
                        p.query = t.clone();
                    }
                })
            })
            .padding([4, 7])
            .size(typography::body())
            .style(ui_style::field::validated(false));
        Dialog::new("Kitaplıktan SVG çizimi aç")
            .push(
                row![
                    container(search).width(Fill),
                    label::caption(format!("{count} çizim")).style(ui_style::text::muted)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .push(list)
            .push(
                row![
                    label::caption("Sistem çizimi açılınca kopyası Kitaplığım’a alınır.").style(ui_style::text::muted),
                    space::horizontal(),
                    button(label::body("Vazgeç"))
                        .padding([5, 14])
                        .style(ui_style::button::secondary)
                        .on_press(change(|ed| ed.files.dialog = None)),
                ]
                .align_y(Center),
            )
            .width(typography::unscaled(width))
            .into()
    }
}
