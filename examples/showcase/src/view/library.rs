//! Blok kitaplığı: kentsel donatı, bitki, altyapı ve trafik sembolleri.
//!
//! Kitaplık panelinde ve galerideki varlık tarayıcısı örneğinde kullanılır.
//! Önizlemeler tuvale plan görünüşüyle çizilir; çift tıklanan blok
//! haritaya nokta olarak yerleştirilir.

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{Canvas, canvas as canvas_widget};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use kentos_rc::theme::{Tokens, typography};
use kentos_rc::widget::assets::{Asset, AssetBrowser};

use crate::app::Showcase;
use crate::message::Message;

/// Bloklar: adı, kategorisi ve kaç çizgiden oluştuğu (liste görünümündeki
/// ayrıntı).
pub const BLOCKS: [(&str, &str, u8); 12] = [
    ("Ağaç", "Bitki", 10),
    ("Çalı", "Bitki", 3),
    ("Çim alan", "Bitki", 7),
    ("Bank", "Donatı", 3),
    ("Çöp kutusu", "Donatı", 2),
    ("Aydınlatma direği", "Donatı", 9),
    ("Rögar", "Altyapı", 3),
    ("Yangın musluğu", "Altyapı", 3),
    ("Elektrik direği", "Altyapı", 3),
    ("Trafik lambası", "Trafik", 4),
    ("Yaya geçidi", "Trafik", 5),
    ("Otobüs durağı", "Trafik", 3),
];

/// Bloğun tuvaldeki önizlemesi.
struct Preview(usize);

impl<Message> canvas::Program<Message> for Preview {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::of(theme);
        let mut frame = Frame::new(renderer, bounds.size());
        // 48 birimlik kare, ortası sıfır.
        let unit = bounds.width.min(bounds.height) / 48.0;
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let at = |x: f32, y: f32| center + Vector::new(x * unit, y * unit);
        let pen = Stroke::default()
            .with_color(t.text)
            .with_width((1.4 * unit).max(1.0));
        let circle = |frame: &mut Frame, x: f32, y: f32, radius: f32| {
            frame.stroke(&Path::circle(at(x, y), radius * unit), pen);
        };
        let rect = |frame: &mut Frame, x: f32, y: f32, width: f32, height: f32| {
            frame.stroke(
                &Path::rectangle(at(x, y), Size::new(width * unit, height * unit)),
                pen,
            );
        };
        let line = |frame: &mut Frame, from: (f32, f32), to: (f32, f32)| {
            frame.stroke(&Path::line(at(from.0, from.1), at(to.0, to.1)), pen);
        };
        let green = Color::from_rgb8(0x6f, 0xb3, 0x5a);

        match self.0 {
            // Ağaç: taç, gövde noktası ve taç kenarındaki dallar.
            0 => {
                frame.fill(
                    &Path::circle(at(0.0, 0.0), 15.0 * unit),
                    green.scale_alpha(0.25),
                );
                circle(&mut frame, 0.0, 0.0, 15.0);
                frame.fill(&Path::circle(at(0.0, 0.0), 2.0 * unit), t.text);

                for index in 0..8 {
                    let angle = index as f32 * std::f32::consts::TAU / 8.0;
                    let (sin, cos) = angle.sin_cos();

                    line(&mut frame, (cos * 6.0, sin * 6.0), (cos * 15.0, sin * 15.0));
                }
            }
            // Çalı: üç küçük taç.
            1 => {
                for (x, y) in [(-7.0, 4.0), (7.0, 4.0), (0.0, -7.0)] {
                    frame.fill(&Path::circle(at(x, y), 8.0 * unit), green.scale_alpha(0.25));
                    circle(&mut frame, x, y, 8.0);
                }
            }
            // Çim alan: çerçeve ve çim işaretleri.
            2 => {
                frame.fill(
                    &Path::rectangle(at(-17.0, -12.0), Size::new(34.0 * unit, 24.0 * unit)),
                    green.scale_alpha(0.2),
                );
                rect(&mut frame, -17.0, -12.0, 34.0, 24.0);

                for (x, y) in [
                    (-9.0, -4.0),
                    (0.0, -4.0),
                    (9.0, -4.0),
                    (-4.5, 5.0),
                    (4.5, 5.0),
                ] {
                    line(&mut frame, (x - 2.0, y - 2.0), (x, y + 1.0));
                    line(&mut frame, (x, y + 1.0), (x + 2.0, y - 2.0));
                }
            }
            // Bank: oturak ve çıtaları.
            3 => {
                rect(&mut frame, -16.0, -7.0, 32.0, 14.0);
                line(&mut frame, (-16.0, -2.5), (16.0, -2.5));
                line(&mut frame, (-16.0, 2.5), (16.0, 2.5));
            }
            // Çöp kutusu: iç içe iki daire.
            4 => {
                circle(&mut frame, 0.0, 0.0, 10.0);
                circle(&mut frame, 0.0, 0.0, 5.5);
            }
            // Aydınlatma direği: direk ve ışık halkası.
            5 => {
                frame.fill(&Path::circle(at(0.0, 0.0), 3.0 * unit), t.warning);

                for index in 0..8 {
                    let angle = index as f32 * std::f32::consts::TAU / 8.0;
                    let (sin, cos) = angle.sin_cos();

                    line(&mut frame, (cos * 7.0, sin * 7.0), (cos * 13.0, sin * 13.0));
                }
            }
            // Rögar: kapak ve ızgarası.
            6 => {
                circle(&mut frame, 0.0, 0.0, 13.0);
                line(&mut frame, (-13.0, 0.0), (13.0, 0.0));
                line(&mut frame, (0.0, -13.0), (0.0, 13.0));
            }
            // Yangın musluğu: kırmızı gövde ve çıkışlar.
            7 => {
                frame.fill(&Path::circle(at(0.0, 0.0), 8.0 * unit), t.danger);
                circle(&mut frame, 0.0, 0.0, 8.0);
                line(&mut frame, (-14.0, 0.0), (-8.0, 0.0));
                line(&mut frame, (8.0, 0.0), (14.0, 0.0));
            }
            // Elektrik direği: kare ve köşegenleri.
            8 => {
                rect(&mut frame, -8.0, -8.0, 16.0, 16.0);
                line(&mut frame, (-8.0, -8.0), (8.0, 8.0));
                line(&mut frame, (8.0, -8.0), (-8.0, 8.0));
            }
            // Trafik lambası: kutu ve üç ışık.
            9 => {
                frame.stroke(
                    &Path::rounded_rectangle(
                        at(-7.0, -16.0),
                        Size::new(14.0 * unit, 32.0 * unit),
                        (3.0 * unit).into(),
                    ),
                    pen,
                );

                for (y, color) in [(-9.0, t.danger), (0.0, t.warning), (9.0, t.success)] {
                    frame.fill(&Path::circle(at(0.0, y), 3.2 * unit), color);
                }
            }
            // Yaya geçidi: şeritler.
            10 => {
                for index in 0..5 {
                    let x = -18.0 + index as f32 * 8.0;

                    frame.fill(
                        &Path::rectangle(at(x, -12.0), Size::new(4.0 * unit, 24.0 * unit)),
                        t.text,
                    );
                }
            }
            // Otobüs durağı: sığınak, bordür çizgisi ve tabela.
            _ => {
                rect(&mut frame, -16.0, -8.0, 32.0, 12.0);
                line(&mut frame, (-20.0, 9.0), (20.0, 9.0));
                frame.fill(&Path::circle(at(12.0, -14.0), 4.0 * unit), t.accent);
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Bloğun önizlemesi, verilen boyutta.
pub fn preview<'a>(index: usize, size: f32) -> Element<'a, Message> {
    let preview: Canvas<Preview, Message> = canvas_widget(Preview(index)).width(size).height(size);

    preview.into()
}

impl Showcase {
    /// Kitaplık paneli: blokları arar, süzer; çift tıklanan blok Nokta
    /// aracıyla haritaya yerleştirilir.
    pub(super) fn library_panel(&self) -> Element<'_, Message> {
        let size = typography::scaled(44.0).round();
        let assets = BLOCKS
            .iter()
            .enumerate()
            .map(|(index, (name, category, lines))| {
                Asset::new(*name, *category, preview(index, size)).detail(format!("{lines} çizgi"))
            });

        iced::widget::container(
            AssetBrowser::new(assets, self.library.selected, Message::LibrarySelected)
                .on_activate(Message::BlockInserted)
                .search(&self.library.search, Message::LibrarySearch)
                .category(self.library.category.as_deref(), Message::LibraryCategory)
                .view(self.library.view, Message::LibraryView)
                .height(iced::Fill),
        )
        .padding([8, 8])
        .into()
    }
}
