//! Temel sayfalar: renkler, yazı ve ikonlar.

use iced::widget::{
    button, checkbox, column, container, row, slider, space, text, text_input, themer,
};
use iced::{Bottom, Center, Color, Element, Fill};

use kentos_rc::icon::{Icon, Tone, icon};
use kentos_rc::label;
use kentos_rc::spatial::model_space;
use kentos_rc::style;
use kentos_rc::theme::typography::{self, Family, Mono, Typography};
use kentos_rc::theme::{self, Accent, Mode, Tokens};
use kentos_rc::widget::table::{self, Table};

use super::{chip, entry, hex, pressed};
use crate::app::Showcase;
use crate::gallery::Demo;
use crate::message::Message;
use crate::view::family_note;

/// Türkçe harflerin hepsini içeren pangram.
const PANGRAM: &str = "Pijamalı hasta yağız şoföre çabucak güvendi.";

/// CAD metinlerinde sık geçen rakam ve işaretler.
const FIGURES: &str = "0123456789  41°00'29.5\"K  ±0,05 m  1:25.000  Ø12  ×2";

impl Showcase {
    pub(super) fn colors_page(&self) -> Vec<Element<'_, Message>> {
        let t = self.mode.tokens(self.accent);

        let interface = [
            (
                "window",
                t.window,
                "Sekme şeridi, durum çubuğu ve pencere zemini",
            ),
            ("surface", t.surface, "Şerit, paneller ve menülerin gövdesi"),
            (
                "surface_alt",
                t.surface_alt,
                "Özellik değerleri, menünün ayrıntı bölmesi",
            ),
            (
                "surface_hover",
                t.surface_hover,
                "Üzerine gelinen satır ve düğmeler",
            ),
            ("header", t.header, "Panel başlıkları ve kategori satırları"),
            ("field", t.field, "Metin girişleri ve komut satırı"),
            ("border", t.border, "Bölücüler ve kenarlar"),
            ("text", t.text, "Birincil metin"),
            ("muted", t.muted, "Açıklamalar ve meta bilgisi"),
            ("disabled()", t.disabled(), "Devre dışı metin ve ikonlar"),
            ("accent", t.accent, "Etkin araç, seçim ve odak"),
            (
                "accent_hover",
                t.accent_hover,
                "Etkin aracın ikonu, üzerine gelme",
            ),
            ("on_accent", t.on_accent, "Vurgu zemini üzerindeki metin"),
            (
                "selection()",
                t.selection(),
                "Seçili satır ve etkin araç zemini",
            ),
            ("success", t.success, "Başarılı işlem"),
            ("warning", t.warning, "Uyarı"),
            ("danger", t.danger, "Hata ve silme"),
        ];

        let canvas = model_space::Style::with(self.backdrop, &self.theme());

        let model = [
            ("background", canvas.background, "Model alanı zemini"),
            ("grid", canvas.grid, "Izgara çizgileri"),
            (
                "grid_major",
                canvas.grid_major,
                "Her beşinci ızgara çizgisi",
            ),
            ("grid_label", canvas.grid_label, "Izgara etiketleri"),
            ("label", canvas.label, "Öğe etiketleri ve ölçek çubuğu"),
            ("selection", canvas.selection, "Seçili öğe"),
            ("grip", canvas.grip, "Seçim tutamaçları"),
            ("measure", canvas.measure, "Ölçüm çizgisi ve etiketleri"),
            ("snap", canvas.snap, "Nesne yakalama işaretleri"),
            ("axis_x", canvas.axis_x, "UCS X ekseni"),
            ("axis_y", canvas.axis_y, "UCS Y ekseni"),
            ("crosshair", canvas.crosshair, "Tam ekran artı imleç"),
            (
                "tag_background",
                canvas.tag_background,
                "İmleç bilgi kutuları",
            ),
        ];

        vec![
            entry(
                "Temalar",
                "kentos_rc::theme::Mode",
                "Dört tema: CAD programlarının grafit koyusu, kâğıt zeminli aydınlık, gece \
                 çalışması için çok koyu ve az parlak gece, siyah zemin ve beyaz yazıyla yüksek \
                 karşıtlık. Her tema kendi harita zeminiyle gelir; gecede katman renkleri kısılır. \
                 Önizlemeler gerçek bileşenlerdir: iced'in themer'ı alt ağaca başka tema verir, \
                 belirteçler onu izler.",
                self.theme_cards(),
                Some(
                    "theme::theme(Mode::Night, Accent::Blue)\n\n\
                     // Bir alt ağaca başka tema\n\
                     themer(Some(theme::theme(Mode::HighContrast, accent)), preview)",
                ),
            ),
            entry(
                "Vurgu rengi",
                "kentos_rc::theme::Accent",
                "Etkin araç, seçim, odak, birincil düğmeler ve öndeki pencerenin çizgisi vurgu \
                 rengindedir; haritadaki seçim ve tutamaçlar da. Sekiz hazır rengin koyu ve \
                 aydınlık tema için ayrı tonları var. Kendi renginiz VURGU komutuyla #RRGGBB \
                 olarak yazılır ve zeminde okunur kalacak kadar açılır ya da koyulaştırılır. \
                 Vurgu zeminindeki yazı, rengin açıklığına göre beyaz ya da koyudur. Bir renge \
                 tıklayarak deneyin.",
                self.accent_cards(),
                Some(
                    "iced::application(App::new, App::update, App::view)\n    \
                     .theme(|app: &App| theme::theme(app.mode, app.accent))\n\n\
                     Accent::parse(\"#e8618c\") // Some(Accent::Custom(0xe8618c))",
                ),
            ),
            entry(
                "Arayüz renkleri",
                "kentos_rc::theme::Tokens",
                "Bileşenler renklerini temadan okur; uygulama yalnızca kipi seçer. \
                 Tablo o anki temanın değerlerini gösterir; Arayüz grubundaki tema \
                 düğmesiyle karşılaştırabilirsiniz.",
                color_table(&interface),
                Some(
                    "text(\"Uyarı\").style(|theme: &Theme| text::Style {\n    \
                     color: Some(Tokens::of(theme).warning),\n})",
                ),
            ),
            entry(
                "Model alanı renkleri",
                "kentos_rc::spatial::model_space::Style",
                "Zemin arayüzün temasından bağımsız seçilir: temaya uyan (koyu temada \
                 arduvaz, aydınlıkta kâğıt), arduvaz, klasik AutoCAD siyahı ya da kâğıt. Seçim \
                 ve tutamaçlar vurgu rengindedir; katman renkleri kâğıt zeminde biraz \
                 koyulaştırılır. Tablo o anki zemini gösterir.",
                color_table(&model),
                Some(
                    "ModelSpace::new(viewport, &layers, Message::ModelSpace)\n    \
                     .backdrop(Backdrop::Black)\n\n\
                     let colors = model_space::Style::with(Backdrop::Black, theme);\n\
                     let stroke = colors.layer_color(layer.color);",
                ),
            ),
        ]
    }
}

impl Showcase {
    /// Dört temanın canlı önizlemesi: her kart kendi temasında çizilir;
    /// altındaki düğme temayı uygular.
    fn theme_cards(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;

        let cards = Mode::ALL.map(|mode| {
            let current = self.mode == mode;

            let preview = column![
                row![
                    label::strong(mode.label()).width(Fill),
                    label::mono_caption("1:25.000"),
                ]
                .align_y(Center),
                text_input("Katman adı", &gallery.text)
                    .on_input(|text| Message::Gallery(Demo::TextChanged(text)))
                    .font(typography::ui())
                    .size(typography::body())
                    .padding([3, 6])
                    .style(style::field::input),
                checkbox(gallery.checked)
                    .label("Etiketleri göster")
                    .size(13.0)
                    .font(typography::ui())
                    .text_size(typography::body())
                    .on_toggle(|checked| Message::Gallery(Demo::Checked(checked))),
                slider(0.0..=1.0, gallery.opacity, |opacity| {
                    Message::Gallery(Demo::OpacityChanged(opacity))
                })
                .step(0.05_f32),
                label::muted("Seçili satır ve odak vurgu renginde."),
                row![
                    button(label::body("Vazgeç"))
                        .on_press(pressed("Vazgeç"))
                        .padding([3, 10])
                        .style(style::button::secondary),
                    button(label::body("Uygula"))
                        .on_press(pressed("Uygula"))
                        .padding([3, 10])
                        .style(style::button::primary),
                ]
                .spacing(6),
            ]
            .spacing(10)
            .padding(12);

            let card = themer(
                Some(theme::theme(mode, self.accent)),
                container(preview)
                    .width(Fill)
                    .style(style::container::bordered),
            )
            .text_color(|theme| Tokens::of(theme).text)
            .background(|theme| Tokens::of(theme).window.into());

            let apply = button(
                label::body(if current {
                    "Kullanılıyor"
                } else {
                    "Bu temayı kullan"
                })
                .width(Fill)
                .align_x(Center),
            )
            .on_press_maybe((!current).then_some(Message::ThemeSelected(mode)))
            .width(Fill)
            .padding([3, 10])
            .style(style::button::flat);

            column![container(card).padding(1).width(Fill), apply]
                .spacing(6)
                .width(Fill)
                .into()
        });

        row(cards).spacing(12).into()
    }

    /// Hazır vurgu renkleri: renk, adı, iki temadaki tonu ve vurgu
    /// zeminindeki yazının örneği. Tıklanınca o renk seçilir.
    fn accent_cards(&self) -> Element<'_, Message> {
        let cards = Accent::PRESETS.map(|accent| {
            let tokens = self.mode.tokens(accent);
            let selected = self.accent == accent;

            let sample =
                container(
                    label::caption("Kaydet").style(move |_: &iced::Theme| text::Style {
                        color: Some(tokens.on_accent),
                    }),
                )
                .padding([2, 10])
                .style(move |_: &iced::Theme| container::Style {
                    background: Some(tokens.accent.into()),
                    border: iced::border::rounded(2.0),
                    ..container::Style::default()
                });

            button(
                column![
                    row![
                        container(space::horizontal()).width(18).height(18).style(
                            move |_: &iced::Theme| container::Style {
                                background: Some(tokens.accent.into()),
                                border: iced::border::rounded(9.0),
                                ..container::Style::default()
                            }
                        ),
                        label::strong(accent.name()),
                    ]
                    .spacing(8)
                    .align_y(Center),
                    label::mono_caption(format!(
                        "{}  {}",
                        hex(accent.color(Mode::Dark)),
                        hex(accent.color(Mode::Light))
                    )),
                    sample,
                ]
                .spacing(6),
            )
            .on_press(Message::AccentChanged(accent))
            .width(Fill)
            .padding([8, 10])
            .style(style::button::list_item(selected))
            .into()
        });

        // İki sıra, dörder renk.
        let mut cards = cards.into_iter();
        let rows = (0..2).map(|_| row(cards.by_ref().take(4)).spacing(8).into());

        column(rows).spacing(8).into()
    }
}

fn color_table<'a>(entries: &[(&'static str, Color, &'static str)]) -> Element<'a, Message> {
    Table::new([
        table::Column::new("").width(28),
        table::Column::new("Belirteç").width(130),
        table::Column::new("Değer").width(120),
        table::Column::new("Kullanım").width(Fill),
    ])
    .extend(entries.iter().map(|&(name, color, usage)| {
        table::Row::new([
            chip(color, 24.0, 14.0),
            label::mono(name).into(),
            label::mono_caption(hex(color)).into(),
            label::muted(usage).into(),
        ])
    }))
    .into()
}

impl Showcase {
    pub(super) fn typography_page(&self) -> Vec<Element<'_, Message>> {
        let current = self.typography;
        let family = current.family.name();
        let mono = current.mono.name();
        let strong = format!("{family} SemiBold");

        let styles = [
            (
                "label::figure",
                typography::figure(),
                mono.to_owned(),
                label::figure("346,71 km"),
            ),
            (
                "label::title",
                typography::title(),
                strong.clone(),
                label::title("Kısayollar ve komutlar"),
            ),
            (
                "label::heading",
                typography::heading(),
                strong.clone(),
                label::heading("Dışa aktar"),
            ),
            (
                "label::strong",
                typography::body(),
                strong,
                label::strong("Katmanlar"),
            ),
            (
                "label::body",
                typography::body(),
                family.to_owned(),
                label::body("Seçili öğenin özellikleri"),
            ),
            (
                "label::muted",
                typography::body(),
                family.to_owned(),
                label::muted("Haritada bir öğeye tıklayın"),
            ),
            (
                "label::caption",
                typography::caption(),
                family.to_owned(),
                label::caption("6 katman"),
            ),
            (
                "label::mono",
                typography::body(),
                mono.to_owned(),
                label::mono("41.00820, 28.97840"),
            ),
            (
                "label::mono_caption",
                typography::caption(),
                mono.to_owned(),
                label::mono_caption("EPSG:3857"),
            ),
        ];

        let scale = Table::new([
            table::Column::new("Biçim").width(160),
            table::Column::new("Boyut").width(44).align_right(),
            table::Column::new("Yazı tipi").width(170),
            table::Column::new("Örnek").width(Fill),
        ])
        .extend(styles.into_iter().map(|(name, size, font, sample)| {
            table::Row::new([
                label::mono(name).into(),
                label::mono_caption(format!("{size:.0}")).into(),
                label::muted(font).into(),
                sample.into(),
            ])
        }));

        let families = Family::ALL
            .into_iter()
            .fold(column![].spacing(6), |families, option| {
                let typography = Typography {
                    family: option,
                    ..current
                };

                families.push(typeface(
                    option.name(),
                    family_note(option),
                    text(PANGRAM)
                        .font(typography.ui())
                        .size(typography::scaled(20.0)),
                    current.family == option,
                    Message::TypographyChanged(typography),
                ))
            });

        let monos = Mono::ALL
            .into_iter()
            .fold(column![].spacing(6), |monos, option| {
                let typography = Typography {
                    mono: option,
                    ..current
                };

                monos.push(typeface(
                    option.name(),
                    "Koordinat, ölçü ve komutlar; rakamlar aynı genişlikte.",
                    text(FIGURES)
                        .font(typography.mono())
                        .size(typography::scaled(16.0)),
                    current.mono == option,
                    Message::TypographyChanged(typography),
                ))
            });

        vec![
            entry(
                "Tip ölçeği",
                "kentos_rc::label",
                format!(
                    "Beş boyut, gövde metnine göre: açıklamalar {}, kontroller ve gövde {}, \
                     menü komutları {}, başlıklar {}, öne çıkan değerler {} piksel. Gövde \
                     metni Görünüm sekmesinden, PUNTO komutuyla ya da Ctrl + ve Ctrl − ile \
                     değişir; satır yükseklikleri gibi metni taşıyan ölçüler de birlikte \
                     büyür.",
                    typography::caption(),
                    typography::body(),
                    typography::heading(),
                    typography::title(),
                    typography::figure(),
                ),
                scale,
                Some(
                    "label::title(feature.name.as_str())\n\
                     label::caption(format!(\"{} katman\", layers.len()))\n\
                     label::mono(format::decimal(location))",
                ),
            ),
            entry(
                "Yazı aileleri",
                "kentos_rc::theme::typography",
                "Aileler kütüphaneye gömülüdür ve SIL Open Font License ile dağıtılır; \
                 makinede kurulu olmaları gerekmez. Pangram Türkçe harflerin hepsini \
                 içerir. Bir aileye tıklamak arayüzü o aileye geçirir; seçim saklanır.",
                column![families, monos].spacing(14),
                Some(
                    "typography::load();\n\
                     typography::set(Typography { family: Family::Inter, size: 14.0, ..Typography::DEFAULT });\n\n\
                     iced::application(new, update, view)\n    .default_font(typography::ui())",
                ),
            ),
        ]
    }
}

/// Yazı ailesi örneği: adı, açıklaması ve kendi ailesiyle yazılmış örnek;
/// tıklanınca o aile seçilir.
fn typeface<'a>(
    name: &'static str,
    note: &'static str,
    sample: text::Text<'a>,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let mut heading = row![label::strong(name)].spacing(8).align_y(Center);

    if active {
        heading = heading.push(label::caption("Seçili").style(style::text::accent));
    }

    button(column![heading, sample, label::caption(note)].spacing(4))
        .on_press(on_press)
        .width(Fill)
        .padding([10, 12])
        .style(style::button::list_item(active))
        .into()
}

pub(super) fn icons_page<'a>() -> Vec<Element<'a, Message>> {
    let tiles = Icon::ALL.map(|glyph| {
        container(
            column![
                icon(glyph).size(24.0),
                label::mono_caption(format!("{glyph:?}"))
            ]
            .spacing(8)
            .align_x(Center),
        )
        .width(112)
        .padding([12, 4])
        .align_x(Center)
        .style(style::container::surface_alt)
        .into()
    });

    let set = row(tiles).spacing(8).wrap().vertical_spacing(8);

    let sizes = row([12.0, 16.0, 24.0, 32.0, 48.0].map(|size: f32| {
        column![
            icon(Icon::Globe).size(size),
            label::mono_caption(format!("{size:.0}"))
        ]
        .spacing(6)
        .align_x(Center)
        .into()
    }))
    .spacing(28)
    .align_y(Bottom);

    let tones = row([
        ("Text", Tone::Text),
        ("Muted", Tone::Muted),
        ("Accent", Tone::Accent),
        ("Highlight", Tone::Highlight),
        ("Disabled", Tone::Disabled),
    ]
    .map(|(name, tone)| {
        column![
            icon(Icon::Target).size(24.0).tone(tone),
            label::mono_caption(name)
        ]
        .spacing(6)
        .align_x(Center)
        .width(72)
        .into()
    }))
    .spacing(8);

    let inherit = row![
        button(
            row![icon(Icon::ZoomIn), label::body("Etkin")]
                .spacing(6)
                .align_y(Center)
        )
        .on_press(pressed("Etkin"))
        .padding([3, 8])
        .style(style::button::flat),
        button(
            row![icon(Icon::ZoomIn), label::body("Devre dışı")]
                .spacing(6)
                .align_y(Center)
        )
        .padding([3, 8])
        .style(style::button::flat),
        button(
            row![icon(Icon::Power).size(14.0), label::body("Birincil")]
                .spacing(7)
                .align_y(Center)
        )
        .on_press(pressed("Birincil"))
        .padding([5, 12])
        .style(style::button::primary),
    ]
    .spacing(8)
    .align_y(Center);

    vec![
        entry(
            "İkon seti",
            "kentos_rc::icon::Icon",
            "Her ikon 16×16'lık bir ızgarada tek çizgi kalınlığıyla çizilir ve \
             çizimi önbellekte tutulur. Adlar Rust'taki varyant adlarıdır.",
            set,
            Some("icon(Icon::ZoomIn).size(24.0)"),
        ),
        entry(
            "Boyutlar",
            "kentos_rc::icon::Glyph::size",
            "İkon istenen boyuta ölçeklenir; çizgi kalınlığı ölçekle büyür ama 1,2 \
             pikselin altına inmez.",
            sizes,
            None,
        ),
        entry(
            "Tonlar",
            "kentos_rc::icon::Tone",
            "Varsayılan ton Inherit'tir: ikon içinde bulunduğu düğmenin metin rengini \
             alır, düğme devre dışıyken sönükleşir. Diğer tonlar rengi temadan seçer.",
            column![tones, inherit].spacing(16),
            Some(
                "icon(Icon::Target).tone(Tone::Accent)\n\
                 button(icon(Icon::ZoomIn)).style(style::button::flat) // Inherit",
            ),
        ),
    ]
}
