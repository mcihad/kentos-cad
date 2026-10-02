//! Kitaplık sayfası: malzeme tarayıcısı, 3B nesne tarayıcısı ve malzeme
//! önizlemesinin ışık modeli.

use iced::widget::{Column, Row, column, container};
use iced::{Center, Element, Fill};

use kentos_ui::label;
use kentos_ui::theme::typography;
use kentos_ui::widget::material::{self, Look, Pattern, Shape};
use kentos_ui::widget::{MaterialBrowser, ObjectBrowser};

use super::entry;
use crate::app::Showcase;
use crate::gallery::Demo;
use crate::message::Message;

impl Showcase {
    pub(super) fn library_page(&self) -> Vec<Element<'_, Message>> {
        vec![
            entry(
                "Malzeme tarayıcısı",
                "kentos_ui::widget::MaterialBrowser",
                "Kitaplığın, projenin ve bulutun malzemeleri aranabilir, kategorili bir \
                 ızgarada. Önizlemeler görüntü dosyası gerektirmez: temel renk, pürüzlülük, \
                 metaliklik ve desenden çizilir; küre ya da karo olarak gösterilir. Arama ad, \
                 kategori, kimlik ve etiketlerde Türkçe harf ayırmadan arar. Seçilen \
                 malzemenin özellikleri yanda; çift tık ya da Uygula seçili nesnelere uygular. \
                 Bileşen yalnız görüntüleyicidir: malzemeleri uygulama verir, bulut kitaplığı \
                 yüklenirken iskelet karolar gösterilir.",
                container(self.demo_materials()).height(typography::scaled(560.0)),
                Some(
                    "MaterialBrowser::new(&self.materials, self.selected, Message::Selected)\n    \
                     .search(&self.query, Message::Search)\n    \
                     .category(self.category.as_deref(), Message::Category)\n    \
                     .shape(self.shape, Message::Shape)       // küre ya da karo\n    \
                     .action(\"Uygula\", Message::Applied)\n    \
                     .status(\"Bulut kitaplığı: 12 malzeme, son eşitleme 14:20\")",
                ),
            ),
            entry(
                "3B nesne tarayıcısı",
                "kentos_ui::widget::ObjectBrowser",
                "Yapılar, kent mobilyası, bitkiler, altyapı ve afet donatısı. Önizleme \
                 nesnenin ağından çizilir; seçilen nesne yanda sürükleyerek döner, çift tık \
                 ilk bakışa döndürür. Zeminde ölçü ızgarası, kenarlarda genişlik, derinlik ve \
                 yükseklik vardır. Ayrıntı düzeyi CityGML'in LOD kademeleridir; süzgeç yalnız \
                 kitaplıkta bulunan kademeleri gösterir. Çift tık ya da Yerleştir nesneyi \
                 çizime yerleştirir; bileşen yalnız görüntüleyicidir.",
                container(self.demo_objects()).height(typography::scaled(600.0)),
                Some(
                    "ObjectBrowser::new(&self.objects, self.selected, Message::Selected)\n    \
                     .search(&self.query, Message::Search)\n    \
                     .category(self.category.as_deref(), Message::Category)\n    \
                     .lod(self.lod, Message::Lod)\n    \
                     .action(\"Yerleştir\", Message::Placed)\n\n\
                     Object3d::new(\"cami\", \"Cami\", \"Yapı\", Some(Rc::new(mesh)))\n    \
                     .lod(Lod::Lod3)\n    \
                     .format(\"glTF\")",
                ),
            ),
            entry(
                "Malzeme önizlemesi",
                "kentos_ui::widget::material::{preview, Look}",
                "Işık sol üstten gelir. Pürüzlülük arttıkça parlama büyür ve söner; \
                 metaliklik parlamayı malzemenin rengine boyar, gölgeyi derinleştirir. Desen \
                 küreye enlem ve boylam çizgileriyle oturur, karoda gerçek ölçüsü okunur.",
                lighting_chart(),
                Some(
                    "material::preview(\n    \
                     Look::new(color).roughness(0.3).metallic(1.0).pattern(Pattern::Brushed),\n    \
                     Shape::Sphere,\n)\n.width(64)\n.height(64)",
                ),
            ),
        ]
    }

    fn demo_materials(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;

        MaterialBrowser::new(&gallery.materials, gallery.material, |index| {
            Message::Gallery(Demo::MaterialSelected(index))
        })
        .on_activate(|index| Message::Gallery(Demo::MaterialActivated(index)))
        .search(&gallery.material_query, |query| {
            Message::Gallery(Demo::MaterialSearch(query))
        })
        .category(gallery.material_category.as_deref(), |category| {
            Message::Gallery(Demo::MaterialCategory(category))
        })
        .shape(gallery.material_shape, |shape| {
            Message::Gallery(Demo::MaterialShape(shape))
        })
        .action("Uygula", |index| {
            Message::Gallery(Demo::MaterialApplied(index))
        })
        .status("Bulut kitaplığına bağlı değil; kitaplık ve proje malzemeleri gösteriliyor.")
        .into()
    }
}

impl Showcase {
    fn demo_objects(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;

        ObjectBrowser::new(&gallery.objects, gallery.object, |index| {
            Message::Gallery(Demo::ObjectSelected(index))
        })
        .on_activate(|index| Message::Gallery(Demo::ObjectPlaced(index)))
        .search(&gallery.object_query, |query| {
            Message::Gallery(Demo::ObjectSearch(query))
        })
        .category(gallery.object_category.as_deref(), |category| {
            Message::Gallery(Demo::ObjectCategory(category))
        })
        .lod(gallery.object_lod, |lod| {
            Message::Gallery(Demo::ObjectLod(lod))
        })
        .action("Yerleştir", |index| {
            Message::Gallery(Demo::ObjectPlaced(index))
        })
        .status("Önizlemeler ağdan çizilir; bulut modelleri bağlanınca aynı biçimde gelir.")
        .into()
    }
}

/// Işık modelinin çizelgesi: satırlarda yalıtkan ve metal, sütunlarda
/// pürüzlülük; altta desenler.
fn lighting_chart<'a>() -> Element<'a, Message> {
    let side = typography::scaled(64.0).round();
    let steps = [0.05_f32, 0.25, 0.5, 0.75, 1.0];
    let base = iced::Color::from_rgb8(0x4c, 0x7f, 0xe0);
    let gold = iced::Color::from_rgb8(0xc8, 0x9b, 0x4a);

    let header = Row::with_children(
        std::iter::once(
            container(label::caption(""))
                .width(typography::scaled(96.0))
                .into(),
        )
        .chain(steps.iter().map(|roughness| {
            container(label::caption(format!(
                "Pürüzlülük {}",
                kentos_ui::attribute::number::real(f64::from(*roughness), 2)
            )))
            .width(side + 24.0)
            .center_x(side + 24.0)
            .into()
        })),
    );

    let line = |title: &'static str, color: iced::Color, metallic: f32| {
        Row::with_children(
            std::iter::once(
                container(label::body(title))
                    .width(typography::scaled(96.0))
                    .center_y(side)
                    .into(),
            )
            .chain(steps.iter().map(move |roughness| {
                container(material::tile(
                    Look::new(color).roughness(*roughness).metallic(metallic),
                    Shape::Sphere,
                    side,
                ))
                .width(side + 24.0)
                .center_x(side + 24.0)
                .into()
            })),
        )
        .align_y(Center)
    };

    let patterns = Row::with_children(Pattern::ALL.iter().map(|pattern| {
        let look = pattern_look(*pattern);

        column![
            material::tile(look, Shape::Sphere, side * 0.8),
            material::tile(look, Shape::Swatch, side * 0.8),
            label::caption(pattern.label()),
        ]
        .spacing(4)
        .align_x(Center)
        .width(side + 8.0)
        .into()
    }))
    .spacing(6)
    .wrap();

    Column::new()
        .spacing(10)
        .push(header)
        .push(line("Yalıtkan", base, 0.0))
        .push(line("Metal", gold, 1.0))
        .push(container(patterns).padding(iced::Padding {
            top: 8.0,
            ..iced::Padding::ZERO
        }))
        .width(Fill)
        .into()
}

/// Desen çizelgesinin her deseni için uygun bir görünüş.
fn pattern_look(pattern: Pattern) -> Look {
    let rgb = iced::Color::from_rgb8;
    let look = |color, roughness| Look::new(color).roughness(roughness).pattern(pattern);

    match pattern {
        Pattern::Plain => look(rgb(0xd9, 0xd2, 0xc4), 0.6),
        Pattern::Brick => look(rgb(0xa4, 0x55, 0x3f), 0.85).accent(rgb(0xcf, 0xc6, 0xb6)),
        Pattern::Tile => look(rgb(0xe4, 0xe2, 0xdc), 0.3).accent(rgb(0x9a, 0x97, 0x90)),
        Pattern::Planks => look(rgb(0xa8, 0x7a, 0x4f), 0.55).accent(rgb(0x7a, 0x52, 0x31)),
        Pattern::Stone => look(rgb(0xc2, 0x8f, 0x6a), 0.9).accent(rgb(0x8a, 0x6a, 0x52)),
        Pattern::Speckle => look(rgb(0x9a, 0x9a, 0x94), 0.9),
        Pattern::Gravel => look(rgb(0xa5, 0x9f, 0x92), 0.85).accent(rgb(0x6f, 0x69, 0x5d)),
        Pattern::Grass => look(rgb(0x5d, 0x8f, 0x3a), 0.9).accent(rgb(0x8f, 0xb9, 0x4f)),
        Pattern::Water => look(rgb(0x2f, 0x6f, 0x8f), 0.06),
        Pattern::Glass => look(rgb(0xcf, 0xe6, 0xee), 0.04),
        Pattern::Shingle => look(rgb(0xb5, 0x53, 0x2f), 0.75).accent(rgb(0x7d, 0x35, 0x1c)),
        Pattern::Brushed => look(rgb(0xb8, 0xbc, 0xc0), 0.3).metallic(1.0),
    }
}
