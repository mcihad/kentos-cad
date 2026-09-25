//! Girdiler sayfası: birimli sayı, vektör ve açı girişleri; renk seçici ve
//! renk rampası.

use iced::widget::{Column, Row, column, container, row, space, text, text_input};
use iced::{Center, Element, Fill};

use kentos_rc::attribute::number::real;
use kentos_rc::label;
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::widget::color::{self, ColorPicker};
use kentos_rc::widget::number::{self, Dial, units};
use kentos_rc::widget::range::histogram;
use kentos_rc::widget::timeline::{self, Marker, Scale};
use kentos_rc::widget::{ChipInput, Form, NumberInput, RadioGroup, RangeSlider, Switch, Timeline};

use super::entry;
use crate::app::Showcase;
use crate::gallery::{self, Demo, KEYFRAMES, MILESTONES};
use crate::message::Message;
use crate::sample;
use crate::view::gallery::scene::{self, Camera, Scene, Shading};

/// Nüfus aralığı örneğinin sınırları ve histogramın aralık sayısı.
const POPULATION: std::ops::RangeInclusive<f64> = 0.0..=16_000_000.0;
const POPULATION_BINS: usize = 32;

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

        let demo = |message| Message::Gallery(message);

        let switches = row![
            Switch::new(gallery.switches[0], move |on| demo(Demo::Switched(0, on)))
                .label("Nesne yakalama"),
            Switch::new(gallery.switches[1], move |on| demo(Demo::Switched(1, on))).label("Izgara"),
            Switch::<Message>::disabled(true).label("Salt okunur katman"),
        ]
        .spacing(28)
        .align_y(Center);

        let radios = row![
            RadioGroup::new(gallery.method, move |method| demo(Demo::Method(method)))
                .option(0, "Pencere", "Tamamen içinde kalan öğeler seçilir.")
                .option(1, "Kesişen", "Pencereye değen öğeler de seçilir.")
                .disabled(2, "Çokgenle (bu katmanda yok)"),
            RadioGroup::new(gallery.unit_system, move |system| demo(Demo::UnitSystem(
                system
            )))
            .option(0, "Metrik", "")
            .option(1, "İmparatorluk", "")
            .horizontal(),
        ]
        .spacing(48);

        // Şehirlerin nüfus dağılımı; seçili aralıktaki şehirler sayılır.
        let populations: Vec<f64> = self
            .layers
            .iter()
            .find(|layer| layer.name == sample::CITIES)
            .map(|layer| {
                layer
                    .features
                    .iter()
                    .filter_map(|feature| feature.value(2).as_f64())
                    .collect()
            })
            .unwrap_or_default();
        let (low, high) = gallery.population;
        let inside = populations
            .iter()
            .filter(|value| **value >= low && **value <= high)
            .count();

        let range = column![
            RangeSlider::new(POPULATION, gallery.population, move |range| {
                demo(Demo::Population(range))
            })
            .step(100_000.0)
            .histogram(&histogram(
                populations.iter().copied(),
                POPULATION,
                POPULATION_BINS
            )),
            row![
                label::mono_caption(real(low, 0)),
                space::horizontal(),
                label::caption(format!(
                    "{inside} / {} şehir bu aralıkta",
                    populations.len()
                )),
                space::horizontal(),
                label::mono_caption(real(high, 0)),
            ],
        ]
        .spacing(6)
        .max_width(typography::scaled(560.0));

        let tags = column![
            ChipInput::new(&gallery.tags, move |tags| demo(Demo::Tags(tags)))
                .suggestions([
                    "park",
                    "yeşil alan",
                    "kentsel dönüşüm",
                    "okul",
                    "sağlık tesisi",
                    "kıyı",
                ])
                .placeholder("Etiket yazın"),
            label::caption(
                "Enter ya da virgül ekler; boş alanda Backspace son etiketi siler; Tab öneriyi \
                 tamamlar."
            ),
        ]
        .spacing(6)
        .max_width(typography::scaled(560.0));

        let name_error = gallery
            .sheet_name
            .trim()
            .is_empty()
            .then_some("Pafta adı boş olamaz.");

        let form = Form::new()
            .section("Pafta")
            .field(
                "Ad",
                text_input("Pafta adı", &gallery.sheet_name)
                    .on_input(move |name| demo(Demo::SheetName(name)))
                    .font(typography::ui())
                    .size(typography::body())
                    .padding([4, 8])
                    .style(style::field::validated(name_error.is_some())),
            )
            .required()
            .help("Antet kutusunda ve sekmede görünür.")
            .error(name_error)
            .field(
                "Kâğıt",
                RadioGroup::new(gallery.paper, move |paper| demo(Demo::Paper(paper)))
                    .option(0, "A4", "")
                    .option(1, "A3", "")
                    .option(2, "A1", "")
                    .horizontal(),
            )
            .section("Konum")
            .field(
                "Merkez enlemi",
                NumberInput::new(gallery.latitude, move |value| demo(Demo::Latitude(value)))
                    .units(units::ANGLE)
                    .range(-90.0..=90.0)
                    .step(0.01)
                    .decimals(5)
                    .width(typography::scaled(200.0)),
            )
            .help("Derece, dakika ve saniye de yazılabilir: 40°59'24\"")
            .section("Seçenekler")
            .row(
                Switch::new(gallery.title_block, move |on| demo(Demo::TitleBlock(on)))
                    .label("Antet kutusunu göster"),
            )
            .field(
                "Etiketler",
                ChipInput::new(&gallery.tags, move |tags| demo(Demo::Tags(tags)))
                    .placeholder("Etiket yazın"),
            );

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
            entry(
                "Anahtar",
                "kentos_rc::widget::Switch",
                "Hemen uygulanan açık/kapalı ayarlar için (ör. ızgarayı açmak); onay kutusu ise \
                 bir formla birlikte onaylanan seçimler içindir. Düğme yeni konumuna kayar; \
                 etiket de tıklanabilir. Devre dışı anahtar durumunu gösterir.",
                switches,
                Some("Switch::new(self.snap, Message::SnapToggled).label(\"Nesne yakalama\")"),
            ),
            entry(
                "Radyo grubu",
                "kentos_rc::widget::RadioGroup",
                "Birbirini dışlayan seçenekler; her seçeneğin açıklaması olabilir, seçilemeyen \
                 seçenek sönük görünür. Kısa seçenekler yan yana dizilir.",
                radios,
                Some(
                    "RadioGroup::new(self.mode, Message::ModeSelected)\n    \
                         .option(Mode::Window, \"Pencere\", \"Tamamen içinde kalan öğeler seçilir.\")\n    \
                         .option(Mode::Crossing, \"Kesişen\", \"Pencereye değen öğeler de seçilir.\")",
                ),
            ),
            entry(
                "Aralık kaydırıcısı ve histogram",
                "kentos_rc::widget::RangeSlider",
                "İki tutamakla alt ve üst sınır; aradaki parça sürüklenince aralık bütün olarak \
                 kayar, rayın boş yerine basmak en yakın tutamağı taşır. Histogram verinin \
                 dağılımını gösterir, seçili aralıktaki çubuklar vurgu rengindedir. Burada örnek \
                 verideki illerin nüfusu.",
                range,
                Some(
                    "RangeSlider::new(0.0..=16e6, self.population, Message::PopulationRange)\n    \
                         .step(100_000.0)\n    \
                         .histogram(&range::histogram(values, 0.0..=16e6, 32))",
                ),
            ),
            entry(
                "Etiket girişi",
                "kentos_rc::widget::ChipInput",
                "Yazılan değerler kaldırılabilir etiketlere dönüşür. Aynı etiket büyük/küçük harf \
                 ve Türkçe harf ayırmadan ikinci kez eklenmez. Öneriler verilmişse yazılanla \
                 başlayan ilki sağda görünür, Tab tamamlar. Etiketler sığmayınca alt satıra geçer.",
                tags,
                Some(
                    "ChipInput::new(&self.tags, Message::TagsChanged)\n    \
                         .suggestions([\"park\", \"okul\", \"kentsel dönüşüm\"])",
                ),
            ),
            entry(
                "Zaman çizelgesi",
                "kentos_rc::widget::Timeline",
                "Zamanlı verinin oynatma başı, aralığı ve olayları: burada bir kentsel dönüşüm \
                 projesinin takvimi. Eksene tıklayın ya da sürükleyin; tekerlek imlecin altındaki \
                 ana göre yakınlaştırır, Shift ile kaydırır, ⤢ tümünü gösterir. Aralık şeridinde \
                 sürükleyerek aralık seçin; oynatma aralıkta döner. Yakınlaştıkça etiketler \
                 yıldan aya, güne ve saate iner. ◆ işaretlerine tıklamak o aşamaya gider.",
                self.project_timeline(),
                Some(
                    "Timeline::new(&self.playback, Message::Timeline)\n    \
                         .scale(Scale::Calendar { offset: 180 })\n    \
                         .marker(Marker::new(permit, \"Yapı ruhsatı\"))\n\n\
                     // update\n\
                     Message::Timeline(event) => self.playback.update(event),\n\
                     Message::Tick(elapsed) => self.playback.advance(elapsed),",
                ),
            ),
            entry(
                "Animasyon",
                "kentos_rc::widget::timeline::Scale::Number",
                "Aynı çizelge kare sayısıyla: 24 kare/saniye, 10 saniye. Oynatın ya da oynatma \
                 başını sürükleyin; ev modeli kareye göre döner. ◆ anahtar karelerdir; hız \
                 menüsü ve döngü düğmesi oynatmayı değiştirir.",
                self.animation_timeline(),
                Some(
                    "Timeline::new(&self.animation, Message::Animation)\n    \
                         .markers(KEYFRAMES.map(|frame| Marker::new(frame, \"Anahtar kare\")))",
                ),
            ),
            entry(
                "Form düzeni",
                "kentos_rc::widget::Form",
                "Etiketler aynı genişlikte bir sütunda, alanın ilk satırına hizalı durur. Bölüm \
                 başlıkları formu böler; zorunlu alanın adının yanında yıldız, altında yardım ya \
                 da hata yazar. Adı silerek hatayı görün.",
                container(form).max_width(typography::scaled(620.0)),
                Some(
                    "Form::new()\n    \
                         .section(\"Pafta\")\n    \
                         .field(\"Ad\", name_input).required()\n    \
                         .help(\"Antet kutusunda ve sekmede görünür.\")\n    \
                         .error(self.name_error())\n    \
                         .row(Switch::new(self.title_block, Message::TitleBlock).label(\"Antet\"))",
                ),
            ),
        ]
    }
}

impl Showcase {
    /// Proje takvimi: aşamalar işaret, oynatma saniyede bir ay.
    fn project_timeline(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let phase = MILESTONES
            .iter()
            .rev()
            .find(|(day, month, year, _)| {
                gallery::unix(*day, *month, *year) <= gallery.project.current
            })
            .map_or("Hazırlık", |(_, _, _, name)| name);

        column![
            Timeline::new(&gallery.project, |event| {
                Message::Gallery(Demo::Project(event))
            })
            .scale(Scale::Calendar { offset: 180 })
            .markers(MILESTONES.iter().map(|(day, month, year, name)| {
                Marker::new(gallery::unix(*day, *month, *year), *name)
            })),
            label::caption(format!("Aşama: {phase}")),
        ]
        .spacing(8)
        .into()
    }

    /// Animasyon: kare sayılı çizelge ve kareye göre dönen ev.
    fn animation_timeline(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let frame = gallery.animation.current;
        let yaw = scene::YAW + (frame / f64::from(gallery::ANIMATION_FRAMES) * 360.0) as f32;

        column![
            container(
                iced::widget::canvas(Scene {
                    camera: Camera::Perspective,
                    shading: Shading::ShadedEdges,
                    yaw,
                })
                .width(Fill)
                .height(typography::scaled(180.0)),
            )
            .style(style::container::field),
            Timeline::new(&gallery.animation, |event| {
                Message::Gallery(Demo::Animation(event))
            })
            .markers(KEYFRAMES.map(|frame| Marker::new(f64::from(frame), "Anahtar kare")),),
            label::caption(format!(
                "Kare {} / {}, {} saniye",
                timeline::format(frame, Scale::Number, 1.0),
                gallery::ANIMATION_FRAMES,
                real(frame / 24.0, 2)
            )),
        ]
        .spacing(8)
        .into()
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
