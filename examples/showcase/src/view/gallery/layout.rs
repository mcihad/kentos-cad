//! Yerleşim sayfası: belge sekmeleri, sekmeli yuva, görünüm alanları ve
//! cetveller.

use iced::widget::{Column, button, canvas, column, container, responsive, row, space};
use iced::{Center, Element, Fill, Point, Shadow, Size, Theme, Vector};

use kentos_rc::icon::{Icon, Tone, icon};
use kentos_rc::label;
use kentos_rc::style;
use kentos_rc::theme::{Tokens, typography};
use kentos_rc::widget::number::units;
use kentos_rc::widget::progress;
use kentos_rc::widget::rulers::Transform;
use kentos_rc::widget::viewports::{self, View, Viewports};
use kentos_rc::widget::{DockSpace, Menu, NumberInput, Pane, Rulers, Tab, Tabs, swatch};

use super::{entry, pressed};
use crate::app::Showcase;
use crate::gallery::{Demo, DemoPanel};
use crate::message::Message;
use crate::view::gallery::scene::{self, Camera, Scene, Shading};

impl Showcase {
    pub(super) fn layout_page(&self) -> Vec<Element<'_, Message>> {
        vec![
            entry(
                "Sekmeli yuva",
                "kentos_rc::widget::DockSpace",
                "Paneller kenarlardaki alanlara yerleşir, aynı yeri sekmelerle paylaşır. Bir \
                 sekmeyi sürükleyin: başka yığının şeridine ya da gövdesinin ortasına bırakılan \
                 panel o yığına katılır, gövdenin kenarına bırakılan yığını böler, ortanın \
                 kenarlarına bırakılan o kenara yerleşir, ortanın içine bırakılan yüzen pencere \
                 olur. Bırakılacak yer vurgu rengiyle gösterilir; Esc vazgeçer. Alanların ve \
                 yığınların arasındaki çizgiler sürüklenir. ⌃ ya da başlığa çift tık yığını \
                 daraltır; ⋯ menüsü paneli yüzdürür, yuvaya geri koyar ya da kapatır. Gövdeler \
                 yalnızca görünürken kurulur; yerleşim metne yazılıp saklanır.",
                self.dock_sample(),
                Some(
                    "DockSpace::new(map, &self.docks, Message::Dock, |panel| match panel {\n    \
                         Panel::Layers => Pane::new(\"Katmanlar\", || self.layers())\n        \
                             .icon(Icon::Layers)\n        \
                             .actions(add_layer),\n    \
                         Panel::Table => Pane::new(\"Öznitelik tablosu\", || self.table()),\n\
                     })\n\n\
                     // update\n\
                     Message::Dock(event) => self.docks.update(event),\n\n\
                     // saklama\n\
                     settings.dock = self.docks.save(|panel| panel.key().to_owned());\n\
                     Docks::load(&text, Panel::parse)",
                ),
            ),
            entry(
                "Görünüm alanları",
                "kentos_rc::widget::Viewports",
                "Aynı modeli birden çok bakışla gösterir: tek, iki, üç ya da dört görünüm. \
                 Görünüme tıklamak onu etkin yapar; etkin görünüm vurgu renginde çerçevelenir. \
                 Sol üstteki menüler bakış yönünü ve görsel stili seçer; ⤢ ya da başlığa çift tık \
                 görünümü büyütür. Görünümler arasındaki çizgiler sürüklenir. Buradaki sahne \
                 örnek bir tuvaldir; içerik uygulamanındır (model alanı, 3B sahne).",
                self.viewports_sample(),
                Some(
                    "Viewports::new(&self.views, Message::Views, |index| {\n    \
                         View::new(self.scene(index))\n        \
                             .menu(camera.label(), move || camera_menu(index))\n        \
                             .menu(style.label(), move || style_menu(index))\n\
                     })\n\n\
                     viewports::arrangements(self.views.arrangement, |arrangement| {\n    \
                         Message::Views(viewports::Event::Arranged(arrangement))\n\
                     })",
                ),
            ),
            entry(
                "Cetveller ve kılavuzlar",
                "kentos_rc::widget::Rulers",
                "İçeriğin üstünde ve solunda birimli cetveller; yakınlaştıkça aralıklar 1, 2, 5 × \
                 10ⁿ adımlarla sıklaşır, imlecin yeri iki cetvelde de işaretlenir. Üst cetvelden \
                 aşağı sürükleyin: yatay kılavuz; sol cetvelden sağa: dikey kılavuz. Kılavuzu \
                 sürükleyerek taşıyın, cetvele geri bırakarak silin; Shift küçük çizgilere \
                 oturtur, Esc vazgeçer. Dönüşüm uygulamanındır: sıfır noktası ve birim başına \
                 piksel. Giriş sekmesinde düzen (pafta) sekmelerinde de vardır.",
                self.rulers_sample(),
                Some(
                    "Rulers::new(sheet, Transform::new(paper_origin, pixels_per_mm))\n    \
                     .unit(\"mm\")\n    \
                     .guides(&self.guides, Message::Guide)\n\n\
                     // update\n\
                     Message::Guide(event) => self.guides.update(event),",
                ),
            ),
            entry(
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
            ),
        ]
    }

    /// Cetvel örneği: masanın ortasında A5 kâğıt, yakınlık ve kılavuzlar.
    fn rulers_sample(&self) -> Element<'_, Message> {
        /// A5 yatay, milimetre; yüzde yüzde milimetre başına piksel.
        const PAPER: Size = Size::new(210.0, 148.0);
        const PIXELS: f32 = 1.6;

        let gallery = &self.gallery;
        let scale = PIXELS * gallery.ruler_zoom as f32 / 100.0;

        let desk = responsive(move |size| {
            let paper = Size::new(PAPER.width * scale, PAPER.height * scale);
            let origin = Point::new(
                ((size.width - paper.width) / 2.0).round(),
                ((size.height - paper.height) / 2.0).round(),
            );

            let sheet = container(label::caption("A5 yatay, 210 × 148 mm").style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(theme).muted),
                },
            ))
            .padding(8)
            .width(paper.width)
            .height(paper.height)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Color::WHITE.into()),
                shadow: Shadow {
                    color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 10.0,
                },
                ..container::Style::default()
            });

            Rulers::new(
                container(iced::widget::pin(sheet).x(origin.x).y(origin.y))
                    .width(Fill)
                    .height(Fill)
                    .style(style::container::surface),
                Transform::new(origin, scale),
            )
            .unit("mm")
            .guides(&gallery.ruler_guides, |event| {
                Message::Gallery(Demo::RulerGuide(event))
            })
            .into()
        });

        let controls = row![
            label::muted("Yakınlık"),
            NumberInput::new(gallery.ruler_zoom, |zoom| {
                Message::Gallery(Demo::RulerZoom(zoom))
            })
            .units(units::PERCENT)
            .range(25.0..=800.0)
            .step(25.0)
            .width(typography::scaled(110.0)),
            space::horizontal(),
            label::caption(format!("{} kılavuz", gallery.ruler_guides.len())),
            button(label::caption("Kılavuzları sil").style(style::text::default))
                .on_press_maybe(
                    (!gallery.ruler_guides.is_empty())
                        .then_some(Message::Gallery(Demo::RulerGuidesCleared)),
                )
                .padding([2, 8])
                .style(style::button::flat),
        ]
        .spacing(8)
        .align_y(Center);

        column![
            controls,
            container(desk)
                .height(typography::scaled(340.0))
                .padding(1)
                .style(style::container::bordered),
        ]
        .spacing(8)
        .into()
    }

    /// Görünüm alanı örneği: düzen seçici ve evin dört bakışı.
    fn viewports_sample(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let views = Viewports::new(
            &gallery.views,
            |event| Message::Gallery(Demo::Views(event)),
            |index| {
                let camera = gallery.cameras[index];
                let shading = gallery.shadings[index];

                View::new(
                    canvas(Scene {
                        camera,
                        shading,
                        yaw: scene::YAW,
                    })
                    .width(Fill)
                    .height(Fill),
                )
                .menu(camera.label(), move || {
                    Camera::ALL
                        .into_iter()
                        .fold(Menu::new().header("Bakış"), |menu, choice| {
                            menu.check(
                                choice.label(),
                                choice == camera,
                                Message::Gallery(Demo::Camera(index, choice)),
                            )
                        })
                })
                .menu(shading.label(), move || {
                    Shading::ALL.into_iter().fold(
                        Menu::new().header("Görsel stil"),
                        |menu, choice| {
                            menu.check(
                                choice.label(),
                                choice == shading,
                                Message::Gallery(Demo::Shading(index, choice)),
                            )
                        },
                    )
                })
            },
        );

        let active = gallery.views.active;

        column![
            row![
                viewports::arrangements(gallery.views.arrangement, |arrangement| {
                    Message::Gallery(Demo::Views(viewports::Event::Arranged(arrangement)))
                }),
                label::caption(format!(
                    "Etkin görünüm: {} · {}",
                    gallery.cameras[active].label(),
                    gallery.shadings[active].label()
                )),
            ]
            .spacing(16)
            .align_y(Center),
            container(views)
                .height(460)
                .padding(1)
                .style(style::container::bordered),
        ]
        .spacing(10)
        .into()
    }

    /// Sekmeli yuva örneği: ortada çizim alanı, kenarlarda paneller.
    fn dock_sample(&self) -> Element<'_, Message> {
        let center = container(
            column![
                icon(Icon::Layout).size(28.0).tone(Tone::Muted),
                label::muted("Çizim alanı"),
                label::caption("Bir sekmeyi buraya bırakırsanız yüzen pencere olur."),
            ]
            .spacing(6)
            .align_x(Center),
        )
        .center(Fill)
        .style(style::container::field);

        let dock = DockSpace::new(
            center,
            &self.gallery.docks,
            |event| Message::Gallery(Demo::Dock(event)),
            |panel| {
                let pane = Pane::new(panel.title(), move || demo_body(panel)).icon(panel.icon());

                match panel {
                    DemoPanel::Layers => pane.actions(
                        button(icon(Icon::Plus).size(12.0))
                            .on_press(pressed("Katman ekle"))
                            .padding(4)
                            .style(style::button::flat),
                    ),
                    DemoPanel::Properties | DemoPanel::Output => pane.scrollable(),
                    _ => pane,
                }
            },
        );

        column![
            container(dock)
                .height(480)
                .padding(1)
                .style(style::container::bordered),
            row![
                button(label::body("Yerleşimi sıfırla"))
                    .on_press(Message::Gallery(Demo::DockReset))
                    .padding([4, 12])
                    .style(style::button::secondary),
                label::caption(self.gallery.docks.save(|panel| demo_key(panel).to_owned())),
            ]
            .spacing(12)
            .align_y(Center),
        ]
        .spacing(10)
        .into()
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

/// Örnek panelin metindeki adı.
fn demo_key(panel: DemoPanel) -> &'static str {
    match panel {
        DemoPanel::Layers => "katmanlar",
        DemoPanel::Styles => "stiller",
        DemoPanel::Properties => "ozellikler",
        DemoPanel::Tasks => "gorevler",
        DemoPanel::Output => "cikti",
        DemoPanel::History => "gecmis",
    }
}

/// Örnek panellerin gövdesi.
fn demo_body<'a>(panel: DemoPanel) -> Element<'a, Message> {
    let rows = |items: &[(&'a str, &'a str)]| {
        items
            .iter()
            .fold(
                Column::new().spacing(6).padding(10),
                |column, (name, value)| {
                    column.push(
                        row![
                            label::body(*name),
                            space::horizontal(),
                            label::mono_caption(*value)
                        ]
                        .align_y(Center),
                    )
                },
            )
            .into()
    };

    match panel {
        DemoPanel::Layers => [
            ("Yapılar", iced::Color::from_rgb8(0xd9, 0x8c, 0x5f)),
            ("Yollar", iced::Color::from_rgb8(0xe2, 0xa9, 0x3b)),
            ("Parseller", iced::Color::from_rgb8(0x8f, 0xa8, 0x6e)),
            ("Su", iced::Color::from_rgb8(0x4c, 0x9b, 0xe8)),
        ]
        .into_iter()
        .fold(
            Column::new().spacing(8).padding(10),
            |column, (name, color)| {
                column.push(
                    row![
                        swatch(color),
                        label::body(name),
                        space::horizontal(),
                        icon(Icon::Eye).size(14.0).tone(Tone::Muted)
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
            },
        )
        .into(),
        DemoPanel::Styles => rows(&[
            ("Çizgi kalınlığı", "1,5 px"),
            ("Dolgu saydamlığı", "%40"),
            ("Etiket", "Ad"),
        ]),
        DemoPanel::Properties => rows(&[
            ("Ad", "Moda Parkı"),
            ("Tür", "Yeşil alan"),
            ("Alan", "12.480 m²"),
            ("Çevre", "512 m"),
            ("İlçe", "Kadıköy"),
            ("Güncelleme", "24.09.2026"),
        ]),
        DemoPanel::Tasks => column![
            label::body("Dışa aktarma: Kadıköy.dxf"),
            progress::bar(Some(0.6)),
            label::body("Dizin oluşturma"),
            progress::bar(None),
        ]
        .spacing(6)
        .padding(10)
        .into(),
        DemoPanel::Output => [
            "12:04:11  Katman eklendi: Yapılar (1.204 öğe)",
            "12:04:12  Dizin oluşturuldu: 0,4 sn",
            "12:05:40  Dışa aktarma başladı: Kadıköy.dxf",
            "12:06:02  Uyarı: 3 geometri boş, atlandı",
        ]
        .into_iter()
        .fold(Column::new().spacing(4).padding(10), |column, line| {
            column.push(label::mono_caption(line).style(style::text::default))
        })
        .into(),
        DemoPanel::History => rows(&[("CIZGI", "12:03"), ("OLC", "12:04"), ("KATMAN", "12:05")]),
    }
}
