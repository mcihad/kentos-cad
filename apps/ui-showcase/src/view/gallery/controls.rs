//! Bileşen sayfaları: düğmeler ve veri.

use iced::widget::{
    Row, button, checkbox, column, container, pick_list, row, slider, space, text_input, tooltip,
};
use iced::{Center, Element, Fill, Theme};

use kentos_ui::attribute::number;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::spatial::Tool;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::assets::Asset;
use kentos_ui::widget::color::Ramp;
use kentos_ui::widget::legend::{self, Symbol};
use kentos_ui::widget::ribbon::{self, AppButton, Field, Group, Preview, Ribbon, Stack, Tile};
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::tree_view::{self, Check, Node, Toggle, TreeView};
use kentos_ui::widget::{
    AssetBrowser, Legend, Menu, NumberInput, Panel, PropertyGrid, Tip, badge, swatch, tip,
    vertical_divider,
};

use super::{entry, pressed};
use crate::app::Showcase;
use crate::gallery::{self, Crs, Demo, Gallery, PARCELS, PROJECT_FILES, ProjectRow};
use crate::message::Message;
use crate::view::library;

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
                "kentos_ui::style::button",
                "iced'in stil imzasını kullanır; kentos-ui bileşenleri dışında da doğrudan \
                 verilebilir. İkinci örnekler devre dışı ya da diğer durumu gösterir.",
                style_table,
                Some("button(\"Kaydet\").style(style::button::primary)"),
            ),
            entry(
                "Şerit düğmeleri ve grupları",
                "kentos_ui::widget::ribbon",
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
                "Menülü düğme, galeri, hızlı erişim",
                "kentos_ui::widget::ribbon::{Button::menu, Gallery}",
                "Eylemi olan menülü düğme bölünür: üst (küçükte sol) kısım eylemi yapar, ok \
                 menüyü açar; eylemsiz düğmenin tamamı menüdür. Galeri seçeneklerin \
                 önizlemelerini (renk, rampa, ikon, çizgi) dizer; satır seçili karoyu içerecek \
                 kadar kayar, ⌄ hepsini ızgarada açar. Sekme şeridinde uygulama düğmesinin \
                 yanında hızlı erişim düğmeleri durur; sağ uçtaki ok şeridi daraltır.",
                self.ribbon_extras(),
                Some(
                    "ribbon::Button::large(Icon::Export, \"Dışa aktar\")\n    \
                         .on_press(Message::Export(Format::GeoJson))\n    \
                         .menu(|| formats_menu())\n\n\
                     ribbon::Gallery::new(tiles, Some(selected), Message::RampSelected)\n\n\
                     Ribbon::new()\n    \
                         .quick(Icon::Undo, \"Geri al\", Some(Message::Undo))\n    \
                         .collapsible(self.collapsed, Message::RibbonToggled)",
                ),
            ),
            entry(
                "İpucu",
                "kentos_ui::widget::Tip",
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

    /// Hızlı erişimli, daraltılabilen küçük bir şerit: menülü düğmeler ve
    /// rampa galerisi.
    fn ribbon_extras(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let menu = || {
            Menu::new()
                .item("Yapıştır", pressed("Yapıştır"))
                .shortcut("Ctrl+V")
                .item("Yerinde yapıştır", pressed("Yerinde yapıştır"))
                .item("Blok olarak yapıştır", pressed("Blok olarak yapıştır"))
        };
        let ramps = Ramp::presets();

        let ribbon = Ribbon::new()
            .application(AppButton::new("Örnek").on_press(pressed("Uygulama menüsü")))
            .quick(Icon::Save, "Kaydet", Some(pressed("Kaydet")))
            .quick(Icon::Undo, "Geri al", Some(pressed("Geri al")))
            .quick(Icon::Redo, "Yinele", None)
            .quick_menu(|| {
                Menu::new()
                    .header("Hızlı erişim")
                    .item("Özelleştir…", pressed("Özelleştir"))
            })
            .tab("Giriş", true, pressed("Giriş"))
            .tab("Görünüm", false, pressed("Görünüm"))
            .collapsible(
                gallery.ribbon_collapsed,
                Message::Gallery(Demo::RibbonCollapsed),
            )
            .group(
                Group::new("Pano")
                    .push(
                        ribbon::Button::large(Icon::Copy, "Yapıştır")
                            .on_press(pressed("Yapıştır"))
                            .menu(menu),
                    )
                    .push(ribbon::Button::large(Icon::DocumentNew, "Yeni").menu(|| {
                        Menu::new()
                            .item("Boş çizim", pressed("Boş çizim"))
                            .item("Şablondan…", pressed("Şablondan"))
                    }))
                    .push(
                        Stack::new()
                            .push(
                                ribbon::Button::small(Icon::Export, "Dışa aktar")
                                    .on_press(pressed("Dışa aktar"))
                                    .menu(|| {
                                        Menu::new()
                                            .item("PDF", pressed("PDF"))
                                            .item("DXF", pressed("DXF"))
                                    }),
                            )
                            .push(ribbon::Button::small(Icon::Print, "Yazdır").menu(|| {
                                Menu::new()
                                    .item("Yazdır…", pressed("Yazdır"))
                                    .item("Önizleme", pressed("Önizleme"))
                            })),
                    ),
            )
            .group(
                Group::new("Renk rampası").push(ribbon::Gallery::new(
                    ramps
                        .iter()
                        .map(|(name, ramp)| Tile::new(*name, Preview::Ramp(ramp.clone()))),
                    Some(gallery.ribbon_ramp),
                    |ramp| Message::Gallery(Demo::RibbonRamp(ramp)),
                )),
            );

        container(ribbon)
            .padding(1)
            .width(Fill)
            .style(style::container::bordered)
            .into()
    }

    /// Tek başına çizilmiş şerit grupları.
    fn ribbon_sample(&self) -> Element<'_, Message> {
        let crs = pick_list(Crs::ALL, self.gallery.crs, |crs| {
            Message::Gallery(Demo::CrsSelected(crs))
        })
        .font(typography::ui())
        .text_size(typography::body())
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
        .height(ribbon::panel_height())
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
                    .collapsible(
                        self.gallery.panels_collapsed[0],
                        Message::Gallery(Demo::PanelToggled(0)),
                    )
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
                )
                .collapsible(
                    self.gallery.panels_collapsed[1],
                    Message::Gallery(Demo::PanelToggled(1)),
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
                "kentos_ui::widget::Table",
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
                "Ağaç görünümü",
                "kentos_ui::widget::TreeView",
                "İç içe klasörler ve öğeler; derinlik sınırsızdır. İlk sütun girinti \
                 çizgilerini, açma okunu, onay kutusunu ve ikonu taşır; diğer sütunlar \
                 tabloyla aynı hizadadır. Klasörün kutusu içindekilerden hesaplanır: \
                 bazıları işaretliyse karışıktır, tıklamak hepsini işaretler. Dosyalara \
                 sağ tıklayın.",
                container(self.project_tree()).width(520),
                Some(
                    "TreeView::new([\n    \
                     tree_view::Column::new(\"Ad\").width(Fill),\n    \
                     tree_view::Column::new(\"Boyut\").width(72).align_right(),\n])\n\
                     .push(\n    Node::new(\"Dış referanslar\")\n        \
                     .check(Check::Mixed, Message::FolderChecked(2))\n        \
                     .expanded(open, Message::FolderToggled(2))\n        \
                     .push(Node::new(\"Ortofoto 2025.tif\").cells([size]))\n        \
                     .menu(|_| Menu::new().item(\"Aç\", Message::Open(3))),\n)",
                ),
            ),
            entry(
                "Sanal tablo",
                "kentos_ui::widget::Table::virtualized",
                "100.000 kayıtlık tablo: satırlar sıra numarasından istenir ve yalnızca \
                 görünenler kurulur, kaydırma hep akıcıdır. Satır numarası yazıp Enter'a \
                 basın; seçilen satır görünür yapılır (reveal). Aynı yapı TreeView::virtualized \
                 ve VirtualList ile ağaçlarda ve listelerde de kullanılır.",
                self.parcel_table(),
                Some(
                    "Table::new(columns)\n    \
                     .virtualized(records.len(), |index| {\n        \
                     table::Row::new(cells(&records[index]))\n            \
                     .selected(index == selected)\n            \
                     .on_press(Message::Selected(index))\n    })\n    \
                     .reveal(Some(selected))\n    .height(300)",
                ),
            ),
            entry(
                "Ağaçta taşıma, adlandırma, satır düğmeleri",
                "kentos_ui::widget::tree_view",
                "Kimlikli düğümleri sürükleyin: satırın üst yarısı önüne, alt yarısı ardına, \
                 klasörün ortası içine bırakır; altındakiler de taşınır, Esc vazgeçer. Satırı \
                 seçip F2'ye basın ya da sağ tıklayıp Yeniden adlandır'ı seçin: Enter ve \
                 dışarı tıklamak kaydeder, Esc vazgeçer. Göz ve kilit düğmeleri adın \
                 sağındadır; gizli klasörün altındakiler sönük görünür.",
                container(self.outline_tree()).width(340),
                Some(
                    "TreeView::new([tree_view::Column::new(\"Katman\").width(Fill)])\n    \
                     .on_move(Message::Moved)\n    \
                     .push(\n        Node::new(\"Mimari\")\n            .id(0)\n            \
                     .folder()\n            .expanded(open, Message::Opened(0))\n            \
                     .push(\n                Node::new(\"Kapılar\")\n                    \
                     .id(2)\n                    \
                     .toggle(Toggle::visible(true, Message::Shown(2)))\n                    \
                     .toggle(Toggle::locked(false, Message::Locked(2))),\n            ),\n    )\n\
                     // F2: .editor(tree_view::rename(&name, Input, Renamed, Cancelled))",
                ),
            ),
            entry(
                "Lejant",
                "kentos_ui::widget::Legend",
                "Harita katmanlarının simgeleri: nokta, çizgi, alan, renk kutusu ya da ikon; \
                 bölüm başlıkları, girintili alt satırlar ve sürekli renk ölçeği. Satıra \
                 tıklamak gizler ya da gösterir, gizli satırlar sönüktür; başlığa tıklamak \
                 lejantı daraltır. Giriş sekmesinde Görünüm → Pencereler → Lejant.",
                self.legend_sample(),
                Some(
                    "Legend::new()\n    \
                     .title(\"Lejant\")\n    \
                     .section(\"Ulaşım\")\n    \
                     .item(Symbol::line(road_color, 2.0), \"Karayolları\")\n    \
                     .detail(\"7\")\n    \
                     .on_press(Message::LayerToggled(3))\n    \
                     .sub(Symbol::line(highway, 2.0), \"Otoyol\")\n    \
                     .ramp(\"Nüfus\", &ramp, \"0\", \"16 M\")",
                ),
            ),
            entry(
                "Varlık tarayıcısı",
                "kentos_ui::widget::AssetBrowser",
                "Sembol, blok ve malzeme kitaplıkları için aranabilir, kategorili ızgara. \
                 Arama Türkçe harf ayırmaz, kategoriler öğelerden çıkarılır; tıklamak seçer, \
                 çift tıklamak kullanır. Izgara ile liste arasında geçin. Önizlemeler \
                 uygulamanındır; buradakiler tuvale çizilen plan sembolleri.",
                container(self.assets_sample()).width(460),
                Some(
                    "AssetBrowser::new(assets, self.selected, Message::AssetSelected)\n    \
                     .on_activate(Message::AssetInserted)\n    \
                     .search(&self.query, Message::AssetSearch)\n    \
                     .category(self.category.as_deref(), Message::AssetCategory)\n    \
                     .view(self.view, Message::AssetView)",
                ),
            ),
            entry(
                "Özellik ızgarası",
                "kentos_ui::widget::PropertyGrid",
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
                "kentos_ui::widget::Panel",
                "Yan paneller başlık çubuğu ve gövdeden oluşur; başlığa tıklayınca açılıp \
                 kapanır, kapalı panelin yerini açık olanlar doldurur. Yuva (Dock) panelleri \
                 bölücü çizgilerle alt alta dizer; sol kenarı sürüklenerek genişletilir, \
                 kenara çift tıklamak varsayılan genişliğe döndürür. Başlıklara tıklayın.",
                panels,
                Some(
                    "Dock::new(width)\n    .resizable(332.0, Message::DockResized)\n    \
                     .on_resize_end(Message::DockResizeEnded)\n    \
                     .push(\n        Panel::new(\"Katmanlar\", table)\n            \
                     .meta(\"6 katman\")\n            \
                     .collapsible(collapsed, Message::LayersToggled)\n            \
                     .height(FillPortion(4))\n            .scrollable(),\n    )",
                ),
            ),
            entry(
                "Giriş alanları",
                "kentos_ui::style::field",
                "iced'in kendi kontrolleri kentos-ui stilleriyle; onay kutusu ve kaydırıcı \
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

    /// Lejant örneği: bölümler, alt satırlar, ikon ve renk ölçeği.
    fn legend_sample(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let hidden = gallery.legend_hidden;
        let rgb = |value: u32| {
            iced::Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8)
        };
        let toggle = |index: usize| Message::Gallery(Demo::LegendItem(index));

        let legend = Legend::new()
            .title("Lejant")
            .collapsible(
                gallery.legend_collapsed,
                Message::Gallery(Demo::LegendCollapsed),
            )
            .section("Yerleşim")
            .item(Symbol::point(rgb(0xe8_59_4c)), "Şehirler")
            .detail("16")
            .muted(hidden[0])
            .on_press(toggle(0))
            .item(Symbol::Icon(Icon::Target, rgb(0x9b_7b_e8)), "Önemli yerler")
            .detail("10")
            .muted(hidden[1])
            .on_press(toggle(1))
            .section("Ulaşım")
            .item(Symbol::line(rgb(0xe2_a9_3b), 2.5), "Karayolları")
            .detail("7")
            .muted(hidden[2])
            .on_press(toggle(2))
            .sub(Symbol::line(rgb(0xf0_8a_3c), 2.5), "Otoyol")
            .sub(Symbol::line(rgb(0xe2_c1_3b), 2.0), "Devlet yolu")
            .item(Symbol::line(rgb(0x4c_9b_e8), 1.5), "Nehirler")
            .detail("12")
            .muted(hidden[3])
            .on_press(toggle(3))
            .section("İdari")
            .item(Symbol::area(rgb(0x8f_c9_5a)), "İlçeler")
            .detail("24")
            .muted(hidden[4])
            .on_press(toggle(4))
            .item(Symbol::Swatch(rgb(0xa0_a4_ab)), "Kıyı şeridi")
            .muted(hidden[5])
            .on_press(toggle(5))
            .ramp("Nüfus yoğunluğu (kişi/km²)", &gallery.ramp, "0", "3.000")
            .width(260);

        row![
            legend::frame(legend),
            label::muted(
                "Harita üstünde lejant::frame ile yarı saydam kutuda durur; kâğıt \
                 düzeninde çerçevesiz kullanılır."
            )
            .width(260),
        ]
        .spacing(24)
        .into()
    }

    /// Varlık tarayıcısı örneği: blok kitaplığı.
    fn assets_sample(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let size = typography::scaled(44.0).round();
        let assets = library::BLOCKS
            .iter()
            .enumerate()
            .map(|(index, (name, category, lines))| {
                Asset::new(*name, *category, library::preview(index, size))
                    .detail(format!("{lines} çizgi"))
            });

        AssetBrowser::new(assets, gallery.asset, |asset| {
            Message::Gallery(Demo::AssetSelected(asset))
        })
        .on_activate(|asset| Message::Gallery(Demo::AssetActivated(asset)))
        .search(&gallery.asset_search, |search| {
            Message::Gallery(Demo::AssetSearch(search))
        })
        .category(gallery.asset_category.as_deref(), |category| {
            Message::Gallery(Demo::AssetCategory(category))
        })
        .view(gallery.asset_view, |view| {
            Message::Gallery(Demo::AssetView(view))
        })
        .height(typography::scaled(300.0))
        .into()
    }

    /// Sanal tablo örneği: 100.000 parsel ve satır numarasıyla gitme.
    fn parcel_table(&self) -> Element<'_, Message> {
        let selected = self.gallery.parcel;

        let table = Table::new([
            table::Column::new("No").width(56).align_right(),
            table::Column::new("Ada/parsel").width(76),
            table::Column::new("Mahalle").width(Fill),
            table::Column::new("Kullanım").width(72),
            table::Column::new("Alan").width(84).align_right(),
        ])
        .virtualized(PARCELS, move |index| {
            let parcel = gallery::parcel(index);

            table::Row::new([
                label::mono_caption(number::integer(index as i64 + 1)).into(),
                label::body(format!("{}/{}", parcel.block, parcel.lot)).into(),
                label::body(parcel.district).into(),
                label::caption(parcel.usage).into(),
                label::mono_caption(format!("{} m²", number::real(parcel.area, 2))).into(),
            ])
            .selected(index == selected)
            .on_press(Message::Gallery(Demo::ParcelSelected(index)))
        })
        .reveal(Some(selected))
        .height(typography::scaled(300.0));

        let jump = row![
            label::muted("Satıra git"),
            NumberInput::new((selected + 1) as f64, |value| {
                Message::Gallery(Demo::ParcelSelected(
                    (value.round().max(1.0) as usize).saturating_sub(1),
                ))
            })
            .range(1.0..=PARCELS as f64)
            .step(1.0)
            .decimals(0)
            .width(typography::scaled(110.0)),
            space::horizontal(),
            label::caption(format!(
                "{} kayıt, seçili: {}",
                number::integer(PARCELS as i64),
                number::integer(selected as i64 + 1)
            )),
        ]
        .spacing(8)
        .align_y(Center);

        container(column![jump, container(table).style(style::container::bordered)].spacing(8))
            .width(560)
            .into()
    }

    /// Taşınabilir ağaç örneği: bir yapı projesinin katman grupları.
    fn outline_tree(&self) -> Element<'_, Message> {
        TreeView::new([tree_view::Column::new("Katman").width(Fill)])
            .on_move(|source, target, place| {
                Message::Gallery(Demo::OutlineMoved(source, target, place))
            })
            .extend(outline_branch(&self.gallery, &mut 0, 0, false))
            .into()
    }

    /// Ağaç örneği: bir imar planı projesinin klasörleri ve dosyaları.
    fn project_tree(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;

        let file = |index: usize, name: &'static str, glyph: Icon, kind: &'static str, size| {
            let row = ProjectRow::File(index);
            let checked = gallery.project_checked[index];

            Node::new(name)
                .icon(icon(glyph).size(14.0).tone(Tone::Muted))
                .check(checked, Message::Gallery(Demo::ProjectFileChecked(index)))
                .cells([
                    label::caption(kind).into(),
                    label::mono_caption(size).into(),
                ])
                .selected(gallery.project_selected == Some(row))
                .muted(!checked)
                .on_press(Message::Gallery(Demo::ProjectSelected(row)))
                .menu(move |_| {
                    Menu::new()
                        .header(name)
                        .item("Aç", pressed("Aç"))
                        .icon(Icon::Folder)
                        .item("Kopyala", pressed("Kopyala"))
                        .icon(Icon::Copy)
                        .shortcut("Ctrl+C")
                        .item("Yeniden adlandır", None)
                        .shortcut("F2")
                        .separator()
                        .item("Projeden kaldır", pressed("Projeden kaldır"))
                        .icon(Icon::Close)
                        .danger()
                })
        };

        let folder = |index: usize, name: &'static str, children: Vec<Node<'static, Message>>| {
            let files = PROJECT_FILES[index];
            let checked = files
                .iter()
                .filter(|&&file| gallery.project_checked[file])
                .count();
            let check = match checked {
                0 => Check::Unchecked,
                count if count == files.len() => Check::Checked,
                _ => Check::Mixed,
            };
            let open = gallery.project_open[index];
            let row = ProjectRow::Folder(index);

            let node = Node::new(name)
                .icon(icon(Icon::Folder).size(14.0).tone(Tone::Muted))
                .check(check, Message::Gallery(Demo::ProjectFolderChecked(index)))
                .expanded(open, Message::Gallery(Demo::ProjectToggled(index)))
                .cells([
                    label::caption("Klasör").into(),
                    label::mono_caption(format!("{} dosya", files.len())).into(),
                ])
                .selected(gallery.project_selected == Some(row))
                .on_press(Message::Gallery(Demo::ProjectSelected(row)));

            if open { node.extend(children) } else { node }
        };

        TreeView::new([
            tree_view::Column::new("Ad").width(Fill),
            tree_view::Column::new("Tür").width(64),
            tree_view::Column::new("Boyut").width(72).align_right(),
        ])
        .push(folder(
            0,
            "Kadıköy imar planı",
            vec![
                folder(
                    1,
                    "Paftalar",
                    vec![
                        file(0, "G22-b-18-c", Icon::Rectangle, "Pafta", "1:1000"),
                        file(1, "G22-b-18-d", Icon::Rectangle, "Pafta", "1:1000"),
                    ],
                ),
                folder(
                    2,
                    "Dış referanslar",
                    vec![
                        file(2, "Halihazır harita.dxf", Icon::Polyline, "DXF", "4,2 MB"),
                        file(3, "Ortofoto 2025.tif", Icon::Grid, "TIFF", "182 MB"),
                    ],
                ),
                folder(
                    3,
                    "Plan kararları",
                    vec![file(4, "Plan notları.pdf", Icon::Document, "PDF", "860 KB")],
                ),
            ],
        ))
        .into()
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
                    .font(typography::ui())
                    .size(typography::body())
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
                .font(typography::ui())
                .text_size(typography::body())
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
                    .font(typography::ui())
                    .text_size(typography::body())
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

/// Taşınabilir ağaç örneğinin `depth` derinliğindeki düğümleri, `next`
/// satırından başlayarak; derinlik sırasındaki satırlardan iç içe düğümler
/// kurar. Gizli bir klasörün altındakiler sönük görünür.
fn outline_branch<'a>(
    gallery: &'a Gallery,
    next: &mut usize,
    depth: usize,
    hidden: bool,
) -> Vec<Node<'a, Message>> {
    let mut nodes = Vec::new();

    while let Some(row) = gallery.outline.get(*next).filter(|row| row.depth == depth) {
        let index = *next;
        let name = row.name.as_str();
        *next += 1;

        let children = outline_branch(gallery, next, depth + 1, hidden || !row.visible);
        let glyph: Element<'a, Message> = if row.folder {
            icon(Icon::Folder).size(14.0).tone(Tone::Muted).into()
        } else {
            swatch(row.color)
        };

        let mut node = Node::new(name)
            .id(index)
            .icon(glyph)
            .toggle(Toggle::visible(
                row.visible,
                Message::Gallery(Demo::OutlineShown(index)),
            ))
            .toggle(Toggle::locked(
                row.locked,
                Message::Gallery(Demo::OutlineLocked(index)),
            ))
            .selected(gallery.outline_selected == Some(index))
            .muted(hidden || !row.visible)
            .on_press(Message::Gallery(Demo::OutlineSelected(index)))
            .menu(move |_| {
                Menu::new()
                    .header(name)
                    .item(
                        "Yeniden adlandır",
                        Message::Gallery(Demo::OutlineRename(index)),
                    )
                    .shortcut("F2")
            });

        if let Some((_, text)) = gallery
            .outline_renaming
            .as_ref()
            .filter(|(renaming, _)| *renaming == index)
        {
            node = node.editor(tree_view::rename(
                text,
                |text| Message::Gallery(Demo::OutlineInput(text)),
                Message::Gallery(Demo::OutlineRenamed),
                Message::Gallery(Demo::OutlineCancelled),
            ));
        }

        if row.folder {
            node = node
                .folder()
                .expanded(row.open, Message::Gallery(Demo::OutlineOpened(index)));
        }

        if !row.folder || row.open {
            node = node.extend(children);
        }

        nodes.push(node);
    }

    nodes
}
