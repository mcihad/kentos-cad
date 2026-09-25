//! Yerleşim sayfası: belge sekmeleri.

use iced::widget::{button, column, container, row};
use iced::{Center, Element, Fill};

use kentos_rc::icon::{Icon, Tone, icon};
use kentos_rc::label;
use kentos_rc::style;
use kentos_rc::widget::{Tab, Tabs};

use super::entry;
use crate::app::Showcase;
use crate::gallery::Demo;
use crate::message::Message;

impl Showcase {
    pub(super) fn layout_page(&self) -> Vec<Element<'_, Message>> {
        vec![entry(
            "Belge sekmeleri",
            "kentos_rc::widget::Tabs",
            "Açık çizimler ya da model ve düzen görünümleri gibi aynı alanı paylaşan içerikler. \
             Etkin sekme içeriğe bağlanır. Kaydedilmemiş çizimde kapatma düğmesinin yerinde nokta \
             durur; üzerine gelince × olur, orta tık da kapatır. Sekmeleri sürükleyerek sıralayın. \
             Sığmayan sekmeler sağdaki listeden seçilir, uzun adların sonu solar; alttaki dar \
             şeritte deneyin. Şerit içeriğin altına da asılır: Giriş sekmesinde haritanın altındaki \
             Model ve Düzen sekmeleri.",
            self.documents_sample(),
            Some(
                "Tabs::new(\n    \
                     drawings.iter().map(|d| Tab::new(&d.name).dirty(d.dirty)),\n    \
                     current,\n    \
                     Message::DrawingSelected,\n\
                 )\n\
                 .on_close(Message::DrawingClosed)\n\
                 .on_reorder(Message::DrawingMoved) // tabs.insert(to, tabs.remove(from))\n\
                 .on_new(Message::DrawingAdded)\n\n\
                 // Model / Düzen: şerit içeriğin altında\n\
                 Tabs::new(sheets, current, Message::SheetSelected).bottom()",
            ),
        )]
    }

    /// Açık çizimler: geniş şerit içeriğiyle, dar şerit taşmayı gösterir.
    fn documents_sample(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;

        let tabs = || {
            Tabs::new(
                gallery.documents.iter().map(|document| {
                    Tab::new(document.name.as_str())
                        .icon(Icon::Document)
                        .dirty(document.dirty)
                }),
                gallery.document,
                |document| Message::Gallery(Demo::DocumentSelected(document)),
            )
            .on_close(|document| Message::Gallery(Demo::DocumentClosed(document)))
            .on_reorder(|from, to| Message::Gallery(Demo::DocumentMoved(from, to)))
            .on_new(Message::Gallery(Demo::DocumentAdded))
        };

        let body: Element<'_, Message> = match gallery.documents.get(gallery.document) {
            Some(document) => {
                let (status, glyph) = if document.dirty {
                    ("Kaydedilmemiş değişiklikler var.", Tone::Warning)
                } else {
                    ("Bütün değişiklikler kaydedildi.", Tone::Success)
                };

                column![
                    label::title(document.name.as_str()),
                    row![
                        icon(if document.dirty {
                            Icon::Warning
                        } else {
                            Icon::Success
                        })
                        .size(14.0)
                        .tone(glyph),
                        label::muted(status),
                    ]
                    .spacing(6)
                    .align_y(Center),
                    button(label::body("Kaydet"))
                        .on_press_maybe(
                            document
                                .dirty
                                .then_some(Message::Gallery(Demo::DocumentSaved)),
                        )
                        .padding([4, 12])
                        .style(style::button::secondary),
                ]
                .spacing(10)
                .into()
            }
            None => label::muted("Açık çizim yok.").into(),
        };

        let document = container(column![
            tabs(),
            container(body).padding([16, 18]).width(Fill).height(150),
        ])
        .padding(1)
        .width(Fill)
        .style(style::container::bordered);

        column![
            document,
            label::caption("Dar şerit: sığmayan sekmeler sağdaki listede."),
            container(tabs())
                .padding(1)
                .width(400)
                .style(style::container::bordered),
        ]
        .spacing(10)
        .into()
    }
}
