//! Şehir ve afet sayfası: bir deprem senaryosunun gösterge kartları, hasar
//! sınıfı ve su derinliği ölçekleri, 3B yapının kat seçicisi.

use iced::widget::{column, container, row};
use iced::{Element, Fill};

use kentos_ui::label;
use kentos_ui::theme::typography;
use kentos_ui::widget::mesh::{self, Camera};
use kentos_ui::widget::stat_card::{Better, Delta};
use kentos_ui::widget::{ClassScale, FloorPicker, Severity, StatCard};

use super::entry;
use crate::app::Showcase;
use crate::gallery::Demo;
use crate::message::Message;

impl Showcase {
    pub(super) fn city_page(&self) -> Vec<Element<'_, Message>> {
        vec![
            entry(
                "Gösterge kartı",
                "kentos_ui::widget::StatCard",
                "Bir sonucun değeri, birimi, önceki duruma göre değişimi ve eğilimi. \
                 Değişimin rengi yönünden değil anlamından gelir: etkilenen yapı için artış \
                 kötü, toplanma alanı kapasitesi için iyidir; ok ve işaretli değer her zaman \
                 yazar. Tıklanabilen kart seçilir (ör. haritada ilgili katmanı göster).",
                self.demo_cards(),
                Some(
                    "StatCard::new(\"Etkilenen yapı\", \"1.284\")\n    \
                     .unit(\"yapı\")\n    \
                     .delta(Delta::new(8.2, \"+8,2 %\").better(Better::Down))\n    \
                     .note(\"önceki senaryoya göre\")\n    \
                     .trend(&affected_by_hour)",
                ),
            ),
            entry(
                "Sınıf ölçeği",
                "kentos_ui::widget::ClassScale",
                "Sıralı sınıfların renkli bantları; genişlik sınıfın payıyla orantılıdır. \
                 Sınıfa tıklamak onu seçer, öbür bantlar solar; yeniden tıklamak bırakır. \
                 İşaret bir öğenin sınıfını gösterir. Renkler veridir: hasar sınıfı, su \
                 derinliği ya da sarsıntı şiddeti kendi renkleriyle gelir.",
                self.demo_scales(),
                Some(
                    "ClassScale::new(&damage)\n    \
                     .title(\"Hasar sınıfı\")\n    \
                     .unit(\"yapı\")\n    \
                     .marker(2, \"Seçili yapı\")\n    \
                     .selected(self.class, Message::ClassChosen)",
                ),
            ),
            entry(
                "Kat seçici",
                "kentos_ui::widget::FloorPicker",
                "3B yapı modelinde bir katı ayırmak için. Katlar yapının kesiti gibi üst üste \
                 durur, zemin çizgisinin altında bodrumlar; her katın kotu sağda. Kesit karesi \
                 katın rengini taşıyabilir (burada senaryodaki hasar sınıfı); renk tek başına \
                 anlam taşımasın diye not da yazar. Seçilen kat yandaki önizlemede \
                 vurgulanır; seçili katı yeniden tıklamak ya da Tümü bütün katlara döner.",
                self.demo_floors(),
                Some(
                    "FloorPicker::new(&floors, self.floor, Message::FloorChosen)\n    \
                     .ground(2)       // zemin kat; altındakiler bodrum\n    \
                     .title(\"Katlar\")\n\n\
                     Floor::new(\"Z\", 0.0).note(\"Ağır hasar\").tone(damage[3].color)",
                ),
            ),
        ]
    }

    fn demo_cards(&self) -> Element<'_, Message> {
        let chosen = self.gallery.card;
        let card = |index: usize, card: StatCard<'static, Message>| {
            card.on_press(Message::Gallery(Demo::CardChosen(index)))
                .selected(chosen == index)
                .width(Fill)
        };

        row![
            card(
                0,
                StatCard::new("Etkilenen yapı", "1.284")
                    .unit("yapı")
                    .delta(Delta::new(8.2, "+8,2 %").better(Better::Down))
                    .note("önceki senaryoya göre")
                    .trend(&[
                        310.0, 520.0, 760.0, 940.0, 1_080.0, 1_190.0, 1_250.0, 1_284.0
                    ]),
            ),
            card(
                1,
                StatCard::new("Ağır hasar ve göçme", "153")
                    .unit("yapı")
                    .delta(Delta::new(12.0, "+12").better(Better::Down))
                    .note("M7,0 senaryosuna göre")
                    .trend(&[20.0, 48.0, 81.0, 110.0, 131.0, 144.0, 150.0, 153.0])
                    .status(Severity::Warning, "Kritik yapıların 4'ü ağır hasarlı."),
            ),
            card(
                2,
                StatCard::new("Tahliye süresi", "42")
                    .unit("dk")
                    .delta(Delta::new(-3.0, "−3 dk").better(Better::Down))
                    .note("yeni güzergâhla")
                    .trend(&[58.0, 55.0, 51.0, 49.0, 47.0, 45.0, 43.0, 42.0]),
            ),
            card(
                3,
                StatCard::new("Toplanma alanı doluluğu", "%68")
                    .unit("37 / 41 alan açık")
                    .delta(Delta::new(-5.0, "−%5").better(Better::Down))
                    .note("son bir saatte")
                    .trend(&[41.0, 55.0, 63.0, 71.0, 74.0, 72.0, 70.0, 68.0]),
            ),
        ]
        .spacing(10)
        .into()
    }

    fn demo_scales(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let damage = &gallery.damage;
        let depth = &gallery.depth;

        row![
            container(
                ClassScale::new(damage)
                    .title("Hasar sınıfı")
                    .unit("yapı")
                    .marker(2, "Seçili yapı")
                    .selected(gallery.damage_class, |class| {
                        Message::Gallery(Demo::ClassChosen(class))
                    })
            )
            .width(Fill),
            container(
                ClassScale::new(depth)
                    .title("Su derinliği")
                    .unit("parsel")
                    .equal()
            )
            .width(Fill),
        ]
        .spacing(28)
        .into()
    }

    fn demo_floors(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let floors = &gallery.floors;
        let mesh = gallery
            .building_floors
            .get(gallery.floor.map_or(0, |floor| floor + 1))
            .cloned();

        let preview: Element<'_, Message> = match mesh {
            Some(mesh) => container(mesh::detail(
                mesh,
                Camera {
                    yaw: -32.0,
                    pitch: 22.0,
                },
            ))
            .height(typography::scaled(330.0))
            .style(|theme: &iced::Theme| iced::widget::container::Style {
                background: Some(kentos_ui::theme::Tokens::of(theme).field.into()),
                border: iced::Border {
                    radius: kentos_ui::theme::shape::md().into(),
                    ..iced::Border::default()
                },
                ..iced::widget::container::Style::default()
            })
            .into(),
            None => label::muted("Yapının ağı yok.").into(),
        };

        row![
            container(
                FloorPicker::new(floors, gallery.floor, |floor| {
                    Message::Gallery(Demo::FloorChosen(floor))
                })
                .ground(2)
                .title("Katlar")
            )
            .width(typography::scaled(280.0)),
            column![
                preview,
                label::caption("Seçilen kat önizlemede vurgulanır; sürükleyerek döndürün."),
            ]
            .spacing(6)
            .width(Fill),
        ]
        .spacing(20)
        .into()
    }
}
