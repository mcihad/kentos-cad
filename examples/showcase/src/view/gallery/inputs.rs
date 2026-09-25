//! Girdiler sayfası: birimli sayı, vektör ve açı girişleri; renk seçici ve
//! renk rampası.

use iced::widget::{Column, Row, column, container, row, space, text};
use iced::{Center, Element, Fill};

use kentos_rc::attribute::number::real;
use kentos_rc::label;
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::widget::NumberInput;
use kentos_rc::widget::color::{self, ColorPicker};
use kentos_rc::widget::number::{self, Dial, units};

use super::entry;
use crate::app::Showcase;
use crate::gallery::Demo;
use crate::message::Message;

/// Form satırlarındaki etiket sütunu (12 piksellik gövde metnine göre).
const LABEL: f32 = 96.0;

impl Showcase {
    pub(super) fn inputs_page(&self) -> Vec<Element<'_, Message>> {
        let gallery = &self.gallery;
        let theme = self.theme();

        let numbers = column![
            field(
                "Genişlik",
                NumberInput::new(gallery.length, |value| Message::Gallery(Demo::Length(
                    value
                )))
                .label("G")
                .units(units::LENGTH)
                .range(0.0..=10_000.0)
                .step(0.1),
                format!("{} m", real(gallery.length, 3)),
            ),
            field(
                "Kalınlık",
                NumberInput::new(gallery.thickness, |value| {
                    Message::Gallery(Demo::Thickness(value))
                })
                .label("T")
                .units(units::MILLIMETRE)
                .range(0.0..=1.0)
                .step(0.5),
                format!("{} m", real(gallery.thickness, 4)),
            ),
            field(
                "Ölçek",
                NumberInput::new(gallery.scale, |value| Message::Gallery(Demo::Scale(value)))
                    .units(units::PERCENT)
                    .range(1.0..=400.0)
                    .step(1.0),
                format!("{} kat", real(gallery.scale / 100.0, 2)),
            ),
            label::caption(
                "Deneyin: 1 m + 20 cm · (3+4)/2 · 250mm · 1,5 km. Etiketi sürükleyin; Shift \
                 ince, Ctrl kaba ayar.",
            ),
        ]
        .spacing(8)
        .max_width(typography::scaled(520.0));

        let position = column![
            number::vector(
                gallery.position,
                |value| Message::Gallery(Demo::Position(value)),
                units::LENGTH,
                0.01,
                &theme,
            ),
            label::caption("Eksen harfleri sürüklenir; her bileşen ayrı ayrı ifade kabul eder."),
        ]
        .spacing(8)
        .max_width(typography::scaled(620.0));

        let angles = row![
            column![
                label::caption("Döndürme (CAD: 0° doğu, saat yönünün tersine)"),
                number::angle(gallery.rotation, |value| {
                    Message::Gallery(Demo::Rotation(value))
                })
                .width(typography::scaled(220.0)),
            ]
            .spacing(6),
            column![
                label::caption("Yön açısı (pusula: 0° kuzey, saat yönünde)"),
                row![
                    Dial::new(gallery.bearing, |value| Message::Gallery(Demo::Bearing(
                        value
                    )))
                    .bearing()
                    .size(40.0),
                    NumberInput::new(gallery.bearing, |value| {
                        Message::Gallery(Demo::Bearing(value.rem_euclid(360.0)))
                    })
                    .label("Y")
                    .units(units::ANGLE)
                    .step(1.0)
                    .width(typography::scaled(150.0)),
                ]
                .spacing(8)
                .align_y(Center),
            ]
            .spacing(6),
        ]
        .spacing(32);

        let recent = [0x8fbf4f, 0xd98c5f, 0x5b626b].map(|value| {
            iced::Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8)
        });

        let colors = column![
            field(
                "Çizgi",
                ColorPicker::new(gallery.stroke, |color| Message::Gallery(Demo::Stroke(
                    color
                )))
                .recent(recent),
                color::to_hex(gallery.stroke),
            ),
            field(
                "Dolgu",
                ColorPicker::new(gallery.fill, |color| Message::Gallery(Demo::Fill(color))).alpha(),
                color::to_hex(gallery.fill),
            ),
            row![
                space::horizontal().width(typography::scaled(LABEL)),
                sample(gallery.stroke, gallery.fill),
            ]
            .spacing(12),
        ]
        .spacing(8)
        .max_width(typography::scaled(520.0));

        // Rampadan eşit aralıklı beş sınıf: derecelendirilmiş sembolojide
        // olduğu gibi.
        let classes = (0..5).fold(Row::new().spacing(4), |row, class| {
            let t = class as f32 / 4.0;

            row.push(
                column![
                    container(space::horizontal())
                        .width(typography::scaled(44.0))
                        .height(typography::scaled(18.0))
                        .style(style::container::swatch(gallery.ramp.color_at(t))),
                    label::caption(format!("%{:.0}", t * 100.0)),
                ]
                .spacing(2)
                .align_x(Center),
            )
        });

        let ramp = column![
            color::ramp(&gallery.ramp, gallery.stop, |ramp, stop| {
                Message::Gallery(Demo::RampChanged(ramp, stop))
            }),
            row![label::caption("Beş sınıf:"), classes]
                .spacing(12)
                .align_y(Center),
        ]
        .spacing(12)
        .max_width(typography::scaled(720.0));

        vec![
            entry(
                "Sayı girişi",
                "kentos_rc::widget::NumberInput",
                "Birimli değer girişi. Alana tıklayınca değer seçili düzenlenir; Enter ya da \
                 alandan çıkmak onaylar, Esc vazgeçer, ↑ ↓ adım kadar değiştirir. Dört işlem ve \
                 parantezli ifadeler, başka birimde yazılan sayılar (alanın birimine çevrilir) ve \
                 Türkçe sayı yazımı kabul edilir; hatalı ifadede kenar kırmızıdır. Öndeki etiket \
                 basılıp yana sürüklenince değer adımıyla değişir. Değer uygulamada temel birimde \
                 durur; sağdaki sütun onu gösterir.",
                numbers,
                Some(
                    "NumberInput::new(self.width, Message::WidthChanged)\n    \
                         .label(\"G\")\n    \
                         .units(units::LENGTH)      // m; cm, mm, km yazılabilir\n    \
                         .range(0.0..=10_000.0)\n    \
                         .step(0.1)                 // ok ve sürükleme adımı\n    \
                         .on_release(Message::WidthSettled)\n\n\
                     number::evaluate(\"1 m + 20 cm\", units::LENGTH) // Ok(1.2)",
                ),
            ),
            entry(
                "Vektör girişi",
                "kentos_rc::widget::number::vector",
                "Konum ve boyut gibi iki ya da üç bileşenli değerler. Etiketler eksen \
                 renklerindedir (X kırmızı, Y yeşil, Z mavi), model alanındaki eksen \
                 göstergesiyle aynı.",
                position,
                Some("number::vector(self.position, Message::Moved, units::LENGTH, 0.01, &theme)"),
            ),
            entry(
                "Açı girişi",
                "kentos_rc::widget::number::angle",
                "Kadran ve derece alanı. Kadranı sürüklemek açıyı değiştirir; Shift 15°'lik \
                 adımlara oturtur. Derece, dakika ve saniye bitişik yazılır (30°15'20\"); \
                 radyan ve grad da kabul edilir. Kadran CAD'deki gibi doğudan saat yönünün \
                 tersine ya da pusuladaki gibi kuzeyden saat yönüne ölçer.",
                angles,
                Some(
                    "number::angle(self.rotation, Message::Rotated)\n\
                     Dial::new(self.bearing, Message::BearingChanged).bearing()",
                ),
            ),
            entry(
                "Renk seçici",
                "kentos_rc::widget::ColorPicker",
                "Alana tıklayınca açılır: doygunluk ve parlaklık düzlemi, ton şeridi, istenirse \
                 saydamlık şeridi, onaltılık giriş (#RGB, #RRGGBB, #RRGGBBAA), hazır renkler ve \
                 uygulamanın verdiği son kullanılanlar. Soldaki kutu panel açıldığındaki renktir; \
                 tıklamak geri döndürür. Renk sürüklerken sürekli bildirilir; Enter ya da \
                 dışarı tıklamak paneli kapatır.",
                colors,
                Some(
                    "ColorPicker::new(layer.color, Message::ColorChanged)\n    \
                         .alpha()\n    \
                         .recent(self.recent.iter().copied())",
                ),
            ),
            entry(
                "Renk rampası",
                "kentos_rc::widget::color::ramp",
                "Sürekli verilerin ve sınıfların renkleri için. Çubuğa tıklamak o noktanın \
                 rengiyle durak ekler; durak sürüklenerek taşınır, çubuğun altına uzağa \
                 sürüklenip bırakılınca silinir. Seçili durağın rengi ve yeri alttaki satırda \
                 düzenlenir; rampa ters çevrilir ya da hazır rampalardan biri seçilir. \
                 Ramp::color_at rampanın herhangi bir noktasındaki rengi verir.",
                ramp,
                Some(
                    "color::ramp(&self.ramp, self.stop, Message::RampChanged)\n\
                     self.ramp.color_at(0.25)",
                ),
            ),
        ]
    }
}

/// Çizgi ve dolgu renklerinin örnek poligonu: saydam dolgu ızgaranın
/// üstünde.
fn sample<'a>(stroke: iced::Color, fill: iced::Color) -> Element<'a, Message> {
    container(
        container(space::horizontal())
            .width(typography::scaled(120.0))
            .height(typography::scaled(44.0))
            .style(move |_theme| container::Style {
                background: Some(fill.into()),
                border: iced::Border {
                    color: stroke,
                    width: 2.0,
                    radius: 2.0.into(),
                },
                ..container::Style::default()
            }),
    )
    .padding(8)
    .style(style::container::field_box)
    .into()
}

/// Form satırı: solda ad, ortada giriş, sağda uygulamadaki değer.
fn field<'a>(
    name: &'a str,
    input: impl Into<Element<'a, Message>>,
    value: String,
) -> Element<'a, Message> {
    row![
        label::body(name).width(typography::scaled(LABEL)),
        Column::new().push(input.into()).width(Fill),
        text(value)
            .font(typography::mono())
            .size(typography::caption())
            .width(typography::scaled(120.0)),
    ]
    .spacing(12)
    .align_y(Center)
    .into()
}
