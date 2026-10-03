//! The export window's PDF part (design §9a), in the web's words
//! (`ui/sheet/exportPdf.ts`): which sheets go (this one, those chosen, all;
//! a page each), GeoPDF (on where the project has a transverse Mercator
//! system, else off and why), the drawing's layers as the viewer's layer
//! list, how the maps go, the file's name; Kaydet or Yazdır.

use std::collections::BTreeSet;

use iced::widget::{button, column, container, row};
use iced::{Center, Element, Fill, Size};
use kentos_sheet::display::{Prim, display_list};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::{Dialog as Window, Segmented, Switch};

use crate::designer::Designer;
use crate::message::{Message, PdfMessage, PdfScope};
use crate::painter::{MapPainter, MapRequest};

/// The web's sentences (`PDF_TEXTS`).
pub mod texts {
    pub const NO_CRS: &str =
        "Projenin koordinat sistemi yok: GeoPDF yazılamaz. Proje ayarlarından sistemi seçin.";
    pub fn not_tm(name: &str) -> String {
        format!(
            "Projenin sistemi ({name}) enine Merkatör değil: GeoPDF bu sürümde TM ve UTM sistemleri için yazılır."
        )
    }
    pub fn geo(name: &str) -> String {
        format!(
            "Konumlu her harita koordinat taşır ({name}): GDAL, QGIS ve Acrobat haritadan koordinat okur."
        )
    }
    pub const LAYERS: &str =
        "Görüntüleyicinin katman listesinden her çizim katmanı açılıp kapatılır.";
    pub fn all_vector(n: usize) -> String {
        format!(
            "{} vektör olarak gider: çizgiler ve yazılar keskin, yazılar seçilebilir.",
            if n == 1 {
                "Harita".to_owned()
            } else {
                format!("{n} harita")
            }
        )
    }
    pub fn picture(name: &str, dpi: u16, why: &str) -> String {
        format!("“{name}” {dpi} dpi resim olarak gider. Vektörle yazılamayanlar: {why}.")
    }
    pub const UNPLACED: &str = "haritanın yeri seçilmedi";
    pub fn pictures(n: usize, dpi: u16) -> String {
        format!(
            "{} {dpi} dpi resim olarak gider.",
            if n == 1 {
                "Harita".to_owned()
            } else {
                format!("{n} harita")
            }
        )
    }
    pub const NO_MAPS: &str = "Bu paftalarda harita çerçevesi yok.";
    pub const PAGES: &str = "Her pafta bir sayfa; kâğıt boyu ve ölçek aynen.";
    pub const NONE_CHOSEN: &str = "Yazılacak bir pafta seçin.";
}

/// A choice of the “Paftalar” row: its scope and the book's sheet count (“Hepsi (3)”).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Scope(PdfScope, usize);

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            PdfScope::This => write!(f, "Bu pafta"),
            PdfScope::Chosen => write!(f, "Seçtiklerim"),
            PdfScope::All => write!(f, "Hepsi ({})", self.1),
        }
    }
}

/// How a map frame goes into the PDF (the web's `MapWayView`).
#[derive(Clone, Debug, PartialEq)]
pub struct MapWay {
    pub name: String,
    pub vector: bool,
    /// Why not as vectors (empty for a painter that writes none).
    pub why: String,
}

/// The PNG resolutions a map's picture may take.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Dpi(u16);

impl std::fmt::Display for Dpi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} dpi", self.0)
    }
}

impl Designer {
    /// How each map frame of the sheets going goes into the PDF (the web's `mapWays`), each once
    /// (a master page's on every sheet that uses it): as vectors, or as a picture and why.
    pub fn pdf_ways(&self, painter: &dyn MapPainter) -> Vec<MapWay> {
        let inputs = self.export_inputs();
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for id in self.pdf_sheets() {
            let Ok(list) = display_list(&self.book, &id, &inputs) else {
                continue;
            };
            for p in &list.prims {
                let Prim::Map(m) = p else {
                    continue;
                };
                if !seen.insert(m.item.clone()) {
                    continue;
                }
                let name = self
                    .book
                    .item(&m.item)
                    .map_or_else(|| "Harita".to_owned(), |i| i.name.clone());
                // The paper's CSS pixels: what a vector map's text is laid out at.
                let k = 96.0 / 25_400.0;
                let size = Size::new(
                    (f64::from(m.clip.width) * k) as f32,
                    (f64::from(m.clip.height) * k) as f32,
                );
                let (vector, why) = if !painter.vectors() {
                    (false, String::new())
                } else if m.view.center.is_none() {
                    (false, texts::UNPLACED.to_owned())
                } else {
                    match painter.no_vectors(&MapRequest::of(m, size, k, true)) {
                        None => (true, String::new()),
                        Some(why) => (false, why),
                    }
                };
                out.push(MapWay { name, vector, why });
            }
        }
        out
    }

    /// The window: the sheets, GeoPDF, layers, the maps' way, the file; Kaydet and Yazdır.
    pub(crate) fn pdf_view(
        &self,
        dpi: u16,
        errors: usize,
        print: bool,
        ways: &[MapWay],
    ) -> Element<'_, Message> {
        let pm = Message::Pdf;
        let count = self.book.sheets.len();
        let mut body = column![].spacing(10);
        body = body
            .push(label::caption("Paftalar").style(style::text::muted))
            .push(
                Segmented::new(
                    [
                        Scope(PdfScope::This, count),
                        Scope(PdfScope::Chosen, count),
                        Scope(PdfScope::All, count),
                    ],
                    Scope(self.pdf.scope, count),
                    move |s: Scope| pm(PdfMessage::Scope(s.0)),
                )
                .width(Fill),
            )
            .push(label::caption(texts::PAGES).style(style::text::muted));
        if self.pdf.scope == PdfScope::Chosen {
            let mut list = column![].spacing(4);
            for s in &self.book.sheets {
                let id = s.id.clone();
                list = list.push(
                    Switch::new(self.pdf.chosen.contains(&s.id), move |on| {
                        pm(PdfMessage::Sheet(id.clone(), on))
                    })
                    .label(s.name.clone()),
                );
            }
            body = body.push(container(list).padding([2, 4]));
        }
        // GeoPDF: on where the project's system is a transverse Mercator one.
        let crs = self.ctx.crs.as_ref();
        let geo_why = match crs {
            None => Some(texts::NO_CRS.to_owned()),
            Some(c) if c.tm.is_none() => Some(texts::not_tm(&c.name)),
            Some(_) => None,
        };
        let geo: Element<'_, Message> = if geo_why.is_some() {
            Switch::disabled(false).label("GeoPDF").into()
        } else {
            Switch::new(self.pdf.geo, move |on| pm(PdfMessage::Geo(on)))
                .label("GeoPDF")
                .into()
        };
        body = body.push(geo).push(
            label::caption(
                geo_why.unwrap_or_else(|| texts::geo(crs.map_or("", |c| c.name.as_str()))),
            )
            .style(style::text::muted),
        );
        body = body
            .push(
                Switch::new(self.pdf.layers, move |on| pm(PdfMessage::Layers(on)))
                    .label("Çizim katmanları PDF katmanı olsun"),
            )
            .push(label::caption(texts::LAYERS).style(style::text::muted));
        // How the maps go: vectors where the host writes them; each one that goes as a picture
        // at the resolution, with why.
        let pictures: Vec<&MapWay> = ways.iter().filter(|w| !w.vector).collect();
        let mut way = column![].spacing(4);
        if ways.is_empty() {
            way = way.push(label::body(texts::NO_MAPS));
        } else if pictures.is_empty() {
            way = way.push(label::body(texts::all_vector(ways.len())));
        } else if pictures.iter().all(|w| w.why.is_empty()) {
            way = way.push(label::body(texts::pictures(pictures.len(), dpi)));
        } else {
            for w in &pictures {
                way = way.push(label::body(texts::picture(&w.name, dpi, &w.why)));
            }
        }
        body = body.push(
            container(way)
                .padding(8)
                .width(Fill)
                .style(style::container::bordered),
        );
        if !pictures.is_empty() {
            body = body.push(
                Segmented::new(
                    [Dpi(150), Dpi(200), Dpi(300), Dpi(400)],
                    Dpi(dpi),
                    |d: Dpi| Message::ExportDpi(d.0),
                )
                .width(Fill),
            );
        }
        let going = self.pdf_sheets();
        if going.is_empty() {
            body = body.push(label::caption(texts::NONE_CHOSEN).style(style::text::muted));
        }
        let file = match self.ask_path_name(crate::message::ExportKind::Pdf) {
            name if name.is_empty() => String::new(),
            name => format!("Dosya: {name}"),
        };
        body = body.push(
            row![
                icon(Icon::Export).size(13.0),
                label::caption(file).style(style::text::muted)
            ]
            .spacing(6)
            .align_y(Center),
        );
        if errors > 0 {
            body = body.push(
                container(label::body(format!(
                    "Ön denetim {errors} hata buldu. Denetçinin Ön denetim sekmesinde çözümleriyle görebilirsiniz; yine de aktarabilirsiniz."
                )))
                .padding(8)
                .style(style::container::bordered),
            );
        }
        let ready = !going.is_empty();
        let anyway = errors > 0;
        let save = button(label::body(if anyway {
            "Yine de kaydet"
        } else {
            "Kaydet"
        }))
        .on_press_maybe(ready.then_some(pm(PdfMessage::Save)))
        .padding([6, 16])
        .style(match (print, anyway) {
            (false, true) => style::button::danger,
            (false, false) => style::button::primary,
            _ => style::button::secondary,
        });
        let printing = button(
            row![
                icon(crate::icons::PRINT).size(15.0),
                label::body(if anyway { "Yine de yazdır" } else { "Yazdır" })
            ]
            .spacing(6)
            .align_y(Center),
        )
        .on_press_maybe(ready.then_some(pm(PdfMessage::Print)))
        .padding([6, 16])
        .style(match (print, anyway) {
            (true, true) => style::button::danger,
            (true, false) => style::button::primary,
            _ => style::button::secondary,
        });
        let window = Window::new(if print {
            "Yazdır"
        } else {
            "PDF olarak dışa aktar"
        })
        .push(body)
        .aside(
            button(label::body("Vazgeç"))
                .on_press(Message::CloseDialog)
                .padding([6, 14])
                .style(style::button::secondary),
        );
        // One primary: Yazdır when the window was opened to print, else Kaydet.
        let window = if print {
            window.action(save).action(printing)
        } else {
            window.action(printing).action(save)
        };
        window.width(480.0).into()
    }
}
