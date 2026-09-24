//! Bileşen sayfaları: düğmeler ve veri.

use iced::widget::{
    Row, button, checkbox, column, container, pick_list, row, slider, text_input, tooltip,
};
use iced::{Center, Element, Fill, Theme};

use kentos_rc::icon::{Icon, icon};
use kentos_rc::label;
use kentos_rc::spatial::Tool;
use kentos_rc::style;
use kentos_rc::widget::ribbon::{self, Field, Group, Stack};
use kentos_rc::widget::table::{self, Table};
use kentos_rc::widget::{Panel, PropertyGrid, Tip, badge, swatch, tip, vertical_divider};

use super::{entry, pressed};
use crate::app::Showcase;
use crate::gallery::{Crs, Demo};
use crate::message::Message;

type ButtonStyle = Box<dyn Fn(&Theme, button::Status) -> button::Style>;

impl Showcase {
    pub(super) fn buttons_page(&self) -> Vec<Element<'_, Message>> {
        let styles: Vec<(&'static str, Vec<Element<'_, Message>>, &'static str)> = vec![
            (
                "button::flat",
                vec![
                    sample_icon(
                        Icon::ZoomIn,
                        "Yakınlaştır",
                        Box::new(style::button::flat),
                        true,
                    ),
                    sample_icon(
                        Icon::ZoomIn,
                        "Yakınlaştır",
                        Box::new(style::button::flat),
                        false,
                    ),
                ],
                "Şerit, gezinme çubuğu ve küçük eylemler",
            ),
            (
                "button::primary",
                vec![
                    sample("Kaydet", Box::new(style::button::primary), true),
                    sample("Kaydet", Box::new(style::button::primary), false),
                ],
                "İletişim kutusunu onaylayan, birincil eylem",
            ),
            (
                "button::secondary",
                vec![
                    sample("Vazgeç", Box::new(style::button::secondary), true),
                    sample("Vazgeç", Box::new(style::button::secondary), false),
                ],
                "Birincil eylemin yanındaki ikincil eylem",
            ),
            (
                "button::subtle",
                vec![
                    button(icon(Icon::Target).size(13.0))
                        .on_press(pressed("subtle"))
                        .padding([2, 4])
                        .style(style::button::subtle)
                        .into(),
                ],
                "Tablo satırlarındaki ikon düğmeleri",
            ),
            (
                "button::tab",
                vec![sample("Ekle", Box::new(style::button::tab), true)],
                "Seçili olmayan şerit sekmesi",
            ),
            (
                "button::tool(bool)",
                vec![
                    sample_icon(
                        Icon::Select,
                        "Seç",
                        Box::new(style::button::tool(true)),
                        true,
                    ),
                    sample_icon(
                        Icon::Pan,
                        "Kaydır",
                        Box::new(style::button::tool(false)),
                        true,
                    ),
                ],
                "Araç düğmesi; etkin araç vurgulanır",
            ),
            (
                "button::toggle(bool)",
                vec![
                    sample("Izgara", Box::new(style::button::toggle(true)), true),
                    sample("Etiketler", Box::new(style::button::toggle(false)), true),
                ],
                "Durum çubuğundaki açık/kapalı anahtarlar",
            ),
            (
                "button::list_item(bool)",
                vec![
                    sample("Dışa aktar", Box::new(style::button::list_item(true)), true),
                    sample("Yazdır", Box::new(style::button::list_item(false)), true),
                ],
                "Menü ve liste satırları; açık alt menü vurgulanır",
            ),
            (
                "button::row(bool)",
                vec![
                    sample("Seçili satır", Box::new(style::button::row(true)), true),
                    sample("Satır", Box::new(style::button::row(false)), true),
                ],
                "Tablo satırları",
            ),
            (
                "button::brand(bool)",
                vec![
                    sample("KentOS CAD", Box::new(style::button::brand(false)), true),
                    sample("KentOS CAD", Box::new(style::button::brand(true)), true),
                ],
                "Uygulama menüsünü açan düğme; menü açıkken koyulaşır",
            ),
        ];

        let style_table = Table::new([
            table::Column::new("Stil").width(190),
            table::Column::new("Örnekler").width(300),
            table::Column::new("Kullanım").width(Fill),
        ])
        .extend(styles.into_iter().map(|(name, samples, usage)| {
            table::Row::new([
                label::mono(name).into(),
                Row::with_children(samples)
                    .spacing(8)
                    .align_y(Center)
                    .into(),
                label::muted(usage).into(),
            ])
        }));

        vec![
            entry(
                "Düğme stilleri",
                "kentos_rc::style::button",
                "iced'in stil imzasını kullanır; kentos-rc bileşenleri dışında da doğrudan \
                 verilebilir. İkinci örnekler devre dışı ya da diğer durumu gösterir.",
                style_table,
                Some("button(\"Kaydet\").style(style::button::primary)"),
            ),
            entry(
                "Şerit düğmeleri ve grupları",
                "kentos_rc::widget::ribbon",
                "Grup içeriği üç satırlık ızgaraya oturur: büyük düğme üç, küçük düğme ve \
                 alan bir satır yüksekliğindedir. Etkin düğme vurgulanır; on_press \
                 verilmeyen düğme ikonuyla birlikte sönükleşir.",
                self.ribbon_sample(),
                Some(
                    "ribbon::Group::new(\"Çizim\")\n    \
                     .push(ribbon::Button::large(Icon::Line, \"Çizgi\").active(true))\n    \
                     .push(ribbon::Stack::new()\n        \
                     .push(ribbon::Button::small(Icon::Rectangle, \"Dikdörtgen\")))",
                ),
            ),
            entry(
                "İpucu",
                "kentos_rc::widget::Tip",
                "Yalnızca başlığı olan ipucu tek satırlık bir etikettir; açıklama ve ayrıntı \
                 satırı eklenince araç ipucuna dönüşür. Görmek için düğmelerin üzerine gelin.",
                row![
                    tip(
                        button(label::body("Sade ipucu"))
                            .on_press(pressed("Sade ipucu"))
                            .padding([3, 10])
                            .style(style::button::secondary),
                        Tip::new("Yakınlaştır"),
                        tooltip::Position::Bottom,
                    ),
                    tip(
                        button(label::body("Araç ipucu"))
                            .on_press(pressed("Araç ipucu"))
                            .padding([3, 10])
                            .style(style::button::secondary),
                        Tip::new(Tool::Line.label())
                            .body(Tool::Line.description())
                            .detail("Komut: CIZGI"),
                        tooltip::Position::Bottom,
                    ),
                ]
                .spacing(8),
                Some(
                    "tip(content, Tip::new(\"Çizgi\")\n    \
                     .body(\"Art arda doğru parçaları çizer.\")\n    \
                     .detail(\"Komut: CIZGI\"), tooltip::Position::Bottom)",
                ),
            ),
        ]
    }

    /// Tek başına çizilmiş şerit grupları.
    fn ribbon_sample(&self) -> Element<'_, Message> {
        let crs = pick_list(Crs::ALL, self.gallery.crs, |crs| {
            Message::Gallery(Demo::CrsSelected(crs))
        })
        .text_size(12.0)
        .padding([2, 8])
        .width(Fill)
        .style(style::field::pick_list)
        .menu_style(style::field::menu);

        let color = self
            .layers
            .get(1)
            .map(|layer| layer.color)
            .unwrap_or_default();

        container(
            row![
                Group::new("Büyük")
                    .push(ribbon::Button::large(Icon::Line, "Çizgi").on_press(pressed("Çizgi")))
                    .push(
                        ribbon::Button::large(Icon::Polyline, "Çoklu\nçizgi")
                            .active(true)
                            .on_press(pressed("Çoklu çizgi")),
                    )
                    .push(ribbon::Button::large(Icon::Circle, "Daire")),
                vertical_divider(),
                Group::new("Küçük").push(
                    Stack::new()
                        .push(
                            ribbon::Button::small(Icon::Rectangle, "Dikdörtgen")
                                .on_press(pressed("Dikdörtgen")),
                        )
                        .push(
                            ribbon::Button::small(Icon::Polygon, "Alan")
                                .active(true)
                                .on_press(pressed("Alan")),
                        )
                        .push(ribbon::Button::small(Icon::Point, "Nokta")),
                ),
                vertical_divider(),
                Group::new("Alan ve satır").push(
                    Stack::new()
                        .width(260)
                        .push(Field::new(swatch(color), crs))
                        .push(
                            ribbon::Row::new()
                                .push(
                                    ribbon::Button::small(Icon::Eye, "Göster")
                                        .on_press(pressed("Göster")),
                                )
                                .push(
                                    ribbon::Button::small(Icon::EyeOff, "Gizle")
                                        .on_press(pressed("Gizle")),
                                )
                                .push(
                                    ribbon::Button::small(Icon::Target, "Sığdır")
                                        .on_press(pressed("Sığdır")),
                                ),
                        ),
                ),
                vertical_divider(),
            ]
            .height(Fill),
        )
        .height(ribbon::PANEL_HEIGHT)
        .style(style::container::surface)
        .into()
    }

    pub(super) fn data_page(&self) -> Vec<Element<'_, Message>> {
        let layers =
            Table::new([
                table::Column::new("").width(11),
                table::Column::new("Ad").width(Fill),
                table::Column::new("Tür").width(50),
                table::Column::new("Öğe").width(30).align_right(),
            ])
            .extend(self.layers.iter().enumerate().skip(1).take(4).map(
                |(index, layer)| {
                    table::Row::new([
                        swatch(layer.color),
                        label::body(layer.name.as_str()).into(),
                        label::caption(layer.kind.label()).into(),
                        label::mono_caption(layer.features.len().to_string()).into(),
                    ])
                    .selected(self.gallery.row == index)
                    .on_press(Message::Gallery(Demo::RowSelected(index)))
                },
            ));

        let properties = PropertyGrid::new()
            .category("Genel")
            .property("Katman", "Şehirler")
            .property("Geometri", "Nokta")
            .figure("Merkez", "41.00820, 28.97840")
            .category("Öznitelikler")
            .property("Bölge", "Marmara")
            .figure("Nüfus (2024)", "15.840.900")
            .figure("Plaka", "34");

        let panels = row![
            container(
                Panel::new("Katmanlar", panel_body("Başlık, meta bilgisi ve gövde."))
                    .meta("6 katman")
            )
            .padding(1)
            .width(300)
            .style(style::container::bordered),
            container(
                Panel::new(
                    "Özellikler",
                    panel_body("Başlığın sağında bir eylem düğmesi.")
                )
                .trailing(
                    button(label::caption("Odakla").style(style::text::default))
                        .on_press(pressed("Odakla"))
                        .padding([1, 8])
                        .style(style::button::flat),
                ),
            )
            .padding(1)
            .width(300)
            .style(style::container::bordered),
        ]
        .spacing(12);

        vec![
            entry(
                "Tablo",
                "kentos_rc::widget::Table",
                "Sütunlar genişlik ve hizayla tanımlanır; başlık ve satırlar aynı aralıkla \
                 dizildiği için hizalı kalır. Satıra tıklayarak seçin. Hücrelerdeki onay \
                 kutusu ve düğmeler satır tıklamasından önce olayı alır.",
                container(layers).width(420),
                Some(
                    "Table::new([\n    \
                     table::Column::new(\"Ad\").width(Fill),\n    \
                     table::Column::new(\"Öğe\").width(30).align_right(),\n])\n\
                     .push(table::Row::new([name, count]).selected(true).on_press(message))",
                ),
            ),
            entry(
                "Özellik ızgarası",
                "kentos_rc::widget::PropertyGrid",
                "CAD programlarındaki Özellikler paleti: anahtar ve değer iki sütunda, \
                 kategoriler başlık satırlarıyla. Sayısal değerler eş aralıklı yazılır.",
                container(properties).width(380),
                Some(
                    "PropertyGrid::new()\n    .category(\"Genel\")\n    \
                     .property(\"Katman\", \"Şehirler\")\n    \
                     .figure(\"Merkez\", \"41.00820, 28.97840\")",
                ),
            ),
            entry(
                "Panel ve yuva",
                "kentos_rc::widget::Panel",
                "Yan paneller başlık çubuğu ve gövdeden oluşur; yuva (Dock) onları bölücü \
                 çizgilerle alt alta dizer. Esnek panellerin gövdesi kaydırılabilir.",
                panels,
                Some(
                    "Dock::new(332.0).push(\n    Panel::new(\"Katmanlar\", table)\n        \
                     .meta(\"6 katman\")\n        .height(FillPortion(4))\n        \
                     .scrollable(),\n)",
                ),
            ),
            entry(
                "Giriş alanları",
                "kentos_rc::style::field",
                "iced'in kendi kontrolleri kentos-rc stilleriyle; onay kutusu ve kaydırıcı \
                 renklerini doğrudan temadan alır.",
                self.fields_sample(),
                Some(
                    "text_input(\"Katman adı\", &value).style(style::field::input)\n\
                     pick_list(Crs::ALL, selected, on_select)\n    \
                     .style(style::field::pick_list)\n    \
                     .menu_style(style::field::menu)",
                ),
            ),
        ]
    }

    fn fields_sample(&self) -> Element<'_, Message> {
        let field = |name: &'static str, control: Element<'static, Message>| {
            row![label::muted(name).width(110), control]
                .spacing(12)
                .align_y(Center)
        };

        let gallery = &self.gallery;

        column![
            row![
                label::muted("Metin").width(110),
                text_input("Katman adı", &gallery.text)
                    .on_input(|text| Message::Gallery(Demo::TextChanged(text)))
                    .size(12.0)
                    .padding([4, 8])
                    .width(280)
                    .style(style::field::input),
            ]
            .spacing(12)
            .align_y(Center),
            row![
                label::muted("Açılır liste").width(110),
                pick_list(Crs::ALL, gallery.crs, |crs| {
                    Message::Gallery(Demo::CrsSelected(crs))
                })
                .placeholder("Koordinat sistemi")
                .text_size(12.0)
                .padding([3, 8])
                .width(280)
                .style(style::field::pick_list)
                .menu_style(style::field::menu),
            ]
            .spacing(12)
            .align_y(Center),
            row![
                label::muted("Onay kutusu").width(110),
                checkbox(gallery.checked)
                    .label("Etiketleri göster")
                    .size(13.0)
                    .text_size(12.0)
                    .on_toggle(|checked| Message::Gallery(Demo::Checked(checked))),
            ]
            .spacing(12)
            .align_y(Center),
            row![
                label::muted("Kaydırıcı").width(110),
                slider(0.0..=1.0, gallery.opacity, |opacity| {
                    Message::Gallery(Demo::OpacityChanged(opacity))
                })
                .step(0.05_f32)
                .width(220),
                label::mono_caption(format!("{:.0}%", gallery.opacity * 100.0))
                    .style(style::text::default),
            ]
            .spacing(12)
            .align_y(Center),
            field(
                "Rozet",
                row![badge("PDF"), badge("DXF"), badge("GeoJSON")]
                    .spacing(6)
                    .into(),
            ),
            field(
                "Renk örneği",
                Row::with_children(
                    self.layers
                        .iter()
                        .map(|layer| swatch(layer.color))
                        .collect::<Vec<_>>(),
                )
                .spacing(6)
                .into(),
            ),
        ]
        .spacing(10)
        .into()
    }
}

/// Metin düğmesi örneği; `enabled` değilse devre dışıdır.
fn sample<'a>(content: &'static str, style: ButtonStyle, enabled: bool) -> Element<'a, Message> {
    button(label::body(content))
        .on_press_maybe(enabled.then(|| pressed(content)))
        .padding([3, 10])
        .style(style)
        .into()
}

/// İkonlu düğme örneği; `enabled` değilse devre dışıdır.
fn sample_icon<'a>(
    glyph: Icon,
    content: &'static str,
    style: ButtonStyle,
    enabled: bool,
) -> Element<'a, Message> {
    button(
        row![icon(glyph), label::body(content)]
            .spacing(6)
            .align_y(Center),
    )
    .on_press_maybe(enabled.then(|| pressed(content)))
    .padding([3, 8])
    .style(style)
    .into()
}

fn panel_body<'a>(content: &'static str) -> Element<'a, Message> {
    container(label::muted(content))
        .padding([12, 10])
        .width(Fill)
        .into()
}
