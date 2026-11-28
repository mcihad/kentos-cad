//! Model görüntüleyici sayfası (docs/adr/8073): uygulamanın IFC modeli penceresi, uygulama olmadan; ve onun 3B
//! görünümle paylaştığı parçalar: kuzey oku ve lejant.

use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Fill};

use kentos_model_view::{OrbitCamera, PARTS, VIEWS};
use kentos_model_view_ui::{Host, look};
use kentos_ui::attribute::number;
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::EmptyState;

use super::{entry, pressed};
use crate::app::Showcase;
use crate::gallery::Demo;
use crate::message::Message;

/// Görüntüleyicinin örnekteki yüksekliği, 12 px yazıda.
const VIEWER_HEIGHT: f32 = 640.0;

impl Showcase {
    pub(super) fn model_page(&self) -> Vec<Element<'_, Message>> {
        vec![
            entry(
                "Model görüntüleyici",
                "kentos_model_view_ui::Viewer",
                "Bir IFC dosyasının modeli: solda yapı ve sınıf ağacı (gövdesi çıkarılamayan nesneler kendi sekmesinde), \
                 ortada 3B sahne, sağda seçilen nesnenin öznitelikleri ve setleri dosyada yazdığı gibi. Sürükleyin: \
                 döndürür; sağ tuşla kaydırır; tekerlek yaklaştırır; tıklama nesneyi seçer, ağaçta dalını açar. \
                 Şeritte bakışlar, gizle ve yalnız bırak, mahaller, açıklıklar ve saydam; Kesit menüsü modeli yatay ya \
                 da düşey bir düzlemle ya da seçimin çevresindeki kutuyla kırpar. Bileşen uygulamayı bilmez: modeli, okumanın \
                 evresini ve seçilenin özelliklerini ev sahibi verir; seçimi ve kapanışı ev sahibine söyler. Örnek \
                 KentOS'un kesit binasından yazdığı IFC 4.3 dosyasıdır.",
                self.model_viewer(),
                Some(
                    "let mut viewer = Viewer::new(\"kesit-binasi.ifc\");\n\
                     viewer.load(kentos_ifc::read::view::input(ad, &model, &bodies), facts);\n\
                     viewer.layout(Host { to: Message::Viewer, end: Vec::new(), sky, coordinate, east: \"Y\", north: \"X\" })\n\
                     // Out::Chosen(Some((nesne, _))) → viewer.set_details(nesne, model.details(&bytes, nesne)?)",
                ),
            ),
            entry(
                "Kuzey oku ve lejant",
                "kentos_model_view_ui::look",
                "Her 3B görünümün sahnesinin çevresindekiler: kuzey oku kamerayla döner, ipucu modelin hangi yandan \
                 görüldüğünü yazar, basılınca kamera kuzeye bakar; lejant sahnedeki parçaları renkleriyle sayar, \
                 saydam parça kendi renginde kenarla durur, başlığı daraltır. Binanın ve yolun 3B görünümü de \
                 görüntüleyici de bunları kullanır.",
                self.model_look(),
                Some(
                    "look::compass(&orbit, Message::North)\n\
                     look::legend(&parcalar, acik, Message::Legend)",
                ),
            ),
        ]
    }

    fn model_viewer(&self) -> Element<'_, Message> {
        let Some(demo) = &self.model_demo else {
            return container(EmptyState::new(Icon::Cube, "Model okunuyor"))
                .height(typography::scaled(VIEWER_HEIGHT))
                .center(Fill)
                .into();
        };
        let t = Tokens::of(&self.theme());
        let viewer = demo.viewer.layout(Host {
            to: |m| Message::Gallery(Demo::Model(m)),
            end: Vec::new(),
            sky: look::sky(t.window, Some(t.accent)),
            quality: kentos_model_view_ui::scene::Quality::Balanced,
            coordinate: Box::new(|v| number::real(v, 3)),
            measure: Box::new(|v, _| number::real(v, 3)),
            east: "Y",
            north: "X",
        });
        container(viewer)
            .width(Fill)
            .height(typography::scaled(VIEWER_HEIGHT))
            .into()
    }

    /// The north arrow from the four views and the viewer's legend.
    fn model_look(&self) -> Element<'_, Message> {
        let arrows = VIEWS.iter().map(|(name, azimuth, elevation)| {
            let orbit = OrbitCamera {
                target: [0.0; 3],
                azimuth: *azimuth,
                elevation: *elevation,
                distance: 10.0,
            };
            column![
                look::compass(&orbit, pressed("Kuzeye bak")),
                label::caption(*name)
            ]
            .spacing(6)
            .align_x(Alignment::Center)
            .into()
        });
        let parts: Vec<(&str, [f32; 4])> = PARTS.iter().take(10).copied().collect();
        row![
            row(arrows).spacing(24).align_y(Alignment::Center),
            look::legend(&parts, true, pressed("Lejant")),
            text("").width(Fill),
        ]
        .spacing(32)
        .align_y(Alignment::Start)
        .into()
    }
}
