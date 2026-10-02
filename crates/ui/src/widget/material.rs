//! Malzeme: adı, kategorisi, görünüşü (temel renk, pürüzlülük, metaliklik,
//! desen), gerçek boyutu ve kaynağı; tuvale çizilen önizlemesi.
//!
//! Önizleme iki biçimdedir ([`Shape`]): ışıklı küre (pürüzlülük parlamanın
//! boyunu ve keskinliğini, metaliklik rengini belirler; desen enlem ve
//! boylam çizgileriyle küreye oturur) ve düz karo (desenin ölçeği okunur).
//! Görüntü dosyası gerekmez: malzemeler buluttan gelmeden de, yalnız
//! görünüş değerleriyle önizlenir. Çizim önbelleğe alınır; görünüş, boyut ya
//! da tema değişmedikçe yeniden kurulmaz.
//!
//! ```ignore
//! let brick = Material::new("tugla-kirmizi", "Kırmızı tuğla", "Tuğla",
//!     Look::new(Color::from_rgb8(0xa4, 0x55, 0x3f))
//!         .roughness(0.85)
//!         .pattern(Pattern::Brick)
//!         .accent(Color::from_rgb8(0xc9, 0xbf, 0xae)))
//!     .size(0.24, 0.07)
//!     .source(Source::Cloud);
//!
//! material::preview(brick.look, Shape::Sphere).width(64).height(64)
//! ```

use std::cell::RefCell;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::fmt;

use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Path, Stroke, gradient};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::theme::Tokens;

/// Malzemenin yüzey deseni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// Desensiz (boya, sıva).
    Plain,
    /// Şaşırtmalı tuğla dizisi ve derz.
    Brick,
    /// Kare karo ve derz.
    Tile,
    /// Ahşap döşeme tahtaları ve damar.
    Planks,
    /// Kesme ya da moloz taş.
    Stone,
    /// İnce benekli yüzey (beton, asfalt).
    Speckle,
    /// Çakıl.
    Gravel,
    /// Çim.
    Grass,
    /// Su yüzeyi.
    Water,
    /// Cam: yarı saydam.
    Glass,
    /// Kiremit sıraları.
    Shingle,
    /// Fırçalanmış metal.
    Brushed,
}

impl Pattern {
    pub const ALL: [Pattern; 12] = [
        Pattern::Plain,
        Pattern::Brick,
        Pattern::Tile,
        Pattern::Planks,
        Pattern::Stone,
        Pattern::Speckle,
        Pattern::Gravel,
        Pattern::Grass,
        Pattern::Water,
        Pattern::Glass,
        Pattern::Shingle,
        Pattern::Brushed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Pattern::Plain => "Düz",
            Pattern::Brick => "Tuğla örgü",
            Pattern::Tile => "Karo",
            Pattern::Planks => "Tahta",
            Pattern::Stone => "Taş",
            Pattern::Speckle => "Benekli",
            Pattern::Gravel => "Çakıl",
            Pattern::Grass => "Çim",
            Pattern::Water => "Su",
            Pattern::Glass => "Cam",
            Pattern::Shingle => "Kiremit",
            Pattern::Brushed => "Fırçalanmış",
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Malzemenin görünüşü: önizlemenin bütün girdisi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Temel renk (albedo).
    pub color: Color,
    /// 0 ayna gibi parlak, 1 tamamen mat.
    pub roughness: f32,
    /// 0 yalıtkan (taş, ahşap), 1 metal.
    pub metallic: f32,
    pub pattern: Pattern,
    /// Desenin ikinci rengi: derz, damar, benek (verilmezse temel renkten).
    pub accent: Option<Color>,
}

impl Look {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            roughness: 0.6,
            metallic: 0.0,
            pattern: Pattern::Plain,
            accent: None,
        }
    }

    pub fn roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    pub fn metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    pub fn pattern(mut self, pattern: Pattern) -> Self {
        self.pattern = pattern;
        self
    }

    pub fn accent(mut self, accent: Color) -> Self {
        self.accent = Some(accent);
        self
    }

    /// Desenin ikinci rengi: verilmemişse temel rengin koyu ya da açık hâli.
    fn second(&self) -> Color {
        self.accent.unwrap_or_else(|| {
            if luminance(self.color) > 0.45 {
                scale(self.color, 0.72)
            } else {
                mix(self.color, Color::WHITE, 0.22)
            }
        })
    }
}

/// Malzemenin nereden geldiği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Source {
    /// Uygulamayla gelen kitaplık.
    #[default]
    Library,
    /// Açık projenin kendi malzemesi.
    Project,
    /// Kurumun ya da kullanıcının bulut kitaplığı.
    Cloud,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Library => "Kitaplık",
            Source::Project => "Proje",
            Source::Cloud => "Bulut",
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Kitaplıktaki bir malzeme.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// Kalıcı kimlik (ör. bulut kitaplığındaki anahtar).
    pub id: String,
    pub name: String,
    pub category: String,
    pub look: Look,
    /// Desenin gerçek boyutu (metre): genişlik ve yükseklik.
    pub size: Option<[f32; 2]>,
    pub source: Source,
    pub tags: Vec<String>,
}

impl Material {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
        look: Look,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: category.into(),
            look,
            size: None,
            source: Source::default(),
            tags: Vec::new(),
        }
    }

    /// Desenin gerçek boyutu (metre).
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Some([width, height]);
        self
    }

    pub fn source(mut self, source: Source) -> Self {
        self.source = source;
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Arama: ad, kategori, kimlik ya da etiketlerden biri yazılanı içeriyor mu
    /// (Türkçe harf ve büyük/küçük harf ayırmadan).
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim();

        query.is_empty()
            || crate::attribute::text::contains(&self.name, query)
            || crate::attribute::text::contains(&self.category, query)
            || crate::attribute::text::contains(&self.id, query)
            || self
                .tags
                .iter()
                .any(|tag| crate::attribute::text::contains(tag, query))
    }
}

/// Önizlemenin biçimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Shape {
    /// Işıklı küre: parlaklık ve metaliklik okunur.
    #[default]
    Sphere,
    /// Düz karo: desen ve ölçeği okunur.
    Swatch,
}

impl Shape {
    pub const ALL: [Shape; 2] = [Shape::Sphere, Shape::Swatch];
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Shape::Sphere => "Küre",
            Shape::Swatch => "Karo",
        })
    }
}

/// Malzeme önizlemesi; boyutu `width` ve `height` ile verilir.
pub fn preview<'a, Message: 'a>(look: Look, shape: Shape) -> Canvas<Preview, Message> {
    Canvas::new(Preview { look, shape })
        .width(Length::Fill)
        .height(Length::Fill)
}

/// Malzeme önizlemesinin tuval programı.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preview {
    look: Look,
    shape: Shape,
}

/// Önbellek ve neyle çizildiği: öğeler süzülüp yer değiştirince başka
/// malzemenin çizimi gösterilmesin diye görünüşle eşlenir.
#[derive(Default)]
pub struct PreviewState {
    cache: Cache,
    key: RefCell<Option<(Preview, bool)>>,
}

impl<Message> canvas::Program<Message> for Preview {
    type State = PreviewState;

    fn draw(
        &self,
        state: &PreviewState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::of(theme);
        let key = (*self, t.is_dark);

        if state.key.borrow().as_ref() != Some(&key) {
            state.cache.clear();
            *state.key.borrow_mut() = Some(key);
        }

        let geometry = state
            .cache
            .draw(renderer, bounds.size(), |frame| match self.shape {
                Shape::Sphere => sphere(frame, &self.look, &t),
                Shape::Swatch => swatch(frame, &self.look),
            });

        vec![geometry]
    }
}

// Işık sol üstten gelir (ekranda).
const LIGHT: Vector = Vector::new(-0.6, -0.8);

/// Işıklı küre: temel renk ve desen düz çizilir, gölge ve parlama üstüne
/// yarı saydam katmanlarla eklenir.
fn sphere(frame: &mut Frame, look: &Look, t: &Tokens) {
    let size = frame.size();
    let r = (size.width.min(size.height) / 2.0 - 2.0).max(4.0) * 0.9;
    let c = Point::new(size.width / 2.0, size.height / 2.0 - r * 0.04);
    let light = Vector::new(LIGHT.x, LIGHT.y);

    // Zemindeki temas gölgesi.
    for step in 0..6 {
        let k = step as f32 / 6.0;
        let path = ellipse(
            Point::new(c.x + r * 0.05, c.y + r * 0.96),
            r * (0.85 - k * 0.45),
            r * (0.16 - k * 0.08),
        );
        frame.fill(
            &path,
            Color::from_rgba(0.0, 0.0, 0.0, if t.is_dark { 0.07 } else { 0.035 }),
        );
    }

    let disc = Path::circle(c, r);
    let glass = look.pattern == Pattern::Glass;

    // Cam yarı saydamdır: arkasında ince bir dama deseni görünür.
    if glass {
        checker(frame, c, r, t);
        frame.fill(
            &disc,
            Color {
                a: 0.42,
                ..look.color
            },
        );
    } else {
        frame.fill(&disc, look.color);
    }

    sphere_pattern(frame, look, c, r);

    // Gölge: ışıktan uzaklaştıkça koyulaşan geçiş (Lambert'in kaba karşılığı).
    let metal = look.metallic;
    let deep = 0.62 + metal * 0.22;
    let shade = gradient::Linear::new(c + light * r, c - light * r)
        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.0))
        .add_stop(0.42, Color::from_rgba(0.0, 0.0, 0.0, 0.12 + metal * 0.1))
        .add_stop(0.75, Color::from_rgba(0.0, 0.0, 0.0, 0.42 + metal * 0.12))
        .add_stop(1.0, Color::from_rgba(0.0, 0.0, 0.0, deep));
    frame.fill(&disc, canvas::Gradient::Linear(shade));

    // Gök yansıması: parlak yüzeyde ufkun üstü açık görünür; pürüzlülük
    // arttıkça söner, metalde daha belirgindir.
    let rough = look.roughness;
    let gloss = (1.0 - rough).powf(2.2) * (0.14 + 0.2 * look.metallic);

    if gloss > 0.01 {
        let horizon = c.y - r * 0.06;
        let rise = ((c.y - horizon) / r).clamp(-1.0, 1.0).asin();
        let sky = Path::new(|path| {
            path.arc(canvas::path::Arc {
                center: c,
                radius: r * 0.985,
                start_angle: iced::Radians(PI + rise),
                end_angle: iced::Radians(TAU - rise),
            });
            path.quadratic_curve_to(
                Point::new(c.x, horizon + r * 0.16),
                Point::new(
                    c.x - (r * r - (c.y - horizon).powi(2)).sqrt() * 0.985,
                    horizon,
                ),
            );
            path.close();
        });
        let fade =
            gradient::Linear::new(Point::new(c.x, c.y - r), Point::new(c.x, horizon + r * 0.1))
                .add_stop(0.0, Color::from_rgba(1.0, 1.0, 1.0, gloss * 0.35))
                .add_stop(0.8, Color::from_rgba(1.0, 1.0, 1.0, gloss))
                .add_stop(1.0, Color::from_rgba(1.0, 1.0, 1.0, gloss * 0.25));
        frame.fill(&sky, canvas::Gradient::Linear(fade));
    }

    // Kubbe: ortaya doğru hafif aydınlanma kürenin yuvarlaklığını verir.
    for step in 0..10 {
        let k = step as f32 / 10.0;
        let radius = r * (1.0 - k).powf(0.75);
        let center = c + light * (r * 0.28 * k);
        frame.fill(
            &Path::circle(center, radius),
            Color::from_rgba(1.0, 1.0, 1.0, 0.022 * (1.0 - metal * 0.5)),
        );
    }

    // Arka kenardaki yansıma (zeminden gelen ışık), metal ve camda belirgin.
    let bounce = 0.05 + metal * 0.12 + if glass { 0.1 } else { 0.0 };
    frame.stroke(
        &Path::new(|path| {
            path.arc(canvas::path::Arc {
                center: c,
                radius: r * 0.9,
                start_angle: iced::Radians(0.15 * PI),
                end_angle: iced::Radians(0.62 * PI),
            });
        }),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, bounce))
            .with_width((r * 0.06).max(1.0)),
    );

    // Parlama: pürüzlülük azaldıkça küçülür ve keskinleşir; metalde
    // malzemenin rengini alır.
    let spot = c + light * (r * 0.48);
    let size = r * (0.07 + rough * 0.5);
    let power = (1.0 - rough).powf(1.6) * 0.85 + 0.06;
    let tint = mix(Color::WHITE, look.color, metal * 0.65);
    let layers = 9;

    for step in 0..layers {
        let k = step as f32 / layers as f32;
        frame.fill(
            &Path::circle(spot, size * (1.0 - k)),
            Color {
                a: power / layers as f32 * (0.6 + k),
                ..tint
            },
        );
    }

    // Dış çizgi: küre her zeminde ayrılsın.
    frame.stroke(
        &disc,
        Stroke::default()
            .with_color(Color::from_rgba(
                0.0,
                0.0,
                0.0,
                if t.is_dark { 0.45 } else { 0.18 },
            ))
            .with_width(1.0),
    );
}

/// Desen kürenin üstünde: enlemler yatay çizgi, boylamlar eğri olarak düşer
/// (dikgen izdüşüm, ekvatordan bakış).
fn sphere_pattern(frame: &mut Frame, look: &Look, c: Point, r: f32) {
    let second = look.second();
    let line = (r * 0.035).max(0.8);
    // Enlemin ekrandaki yüksekliği ve o enlemdeki yarım genişlik.
    let at = |latitude: f32, longitude: f32| {
        Point::new(
            c.x + r * latitude.cos() * longitude.sin(),
            c.y - r * latitude.sin(),
        )
    };

    match look.pattern {
        Pattern::Plain | Pattern::Glass => {}
        Pattern::Brick | Pattern::Tile | Pattern::Shingle => {
            let (courses, around) = match look.pattern {
                Pattern::Brick => (11, 18),
                Pattern::Tile => (9, 18),
                _ => (8, 16),
            };
            let band = PI / courses as f32;

            for row in 0..courses {
                let bottom = -FRAC_PI_2 + band * row as f32;
                let top = bottom + band;

                // Kiremidin alt kenarı gölgelidir: bindirme.
                if look.pattern == Pattern::Shingle && row > 0 {
                    let shadow = Path::new(|path| {
                        path.move_to(at(bottom, -FRAC_PI_2));
                        for step in 0..=24 {
                            let longitude = -FRAC_PI_2 + PI * step as f32 / 24.0;
                            path.line_to(at(bottom + band * 0.35, longitude));
                        }
                        path.line_to(at(bottom, FRAC_PI_2));
                        path.close();
                    });
                    frame.fill(
                        &shadow,
                        Color {
                            a: 0.35,
                            ..scale(look.color, 0.55)
                        },
                    );
                }

                if row > 0 {
                    let y = c.y - r * bottom.sin();
                    let half = r * bottom.cos();
                    frame.stroke(
                        &Path::line(Point::new(c.x - half, y), Point::new(c.x + half, y)),
                        Stroke::default()
                            .with_color(second)
                            .with_width(line * (0.5 + 0.5 * bottom.cos())),
                    );
                }

                let offset = if look.pattern == Pattern::Tile || row % 2 == 0 {
                    0.0
                } else {
                    0.5
                };

                for joint in 0..around {
                    let longitude = -PI + TAU * (joint as f32 + offset) / around as f32;

                    if longitude.cos() <= 0.05 {
                        continue;
                    }

                    frame.stroke(
                        &Path::line(at(bottom, longitude), at(top, longitude)),
                        Stroke::default()
                            .with_color(second)
                            .with_width(line * longitude.cos()),
                    );
                }
            }
        }
        Pattern::Planks => {
            let boards = 16;

            for board in 0..boards {
                let longitude = -PI + TAU * board as f32 / boards as f32;

                if longitude.cos() <= 0.05 {
                    continue;
                }

                let seam = Path::new(|path| {
                    for step in 0..=20 {
                        let latitude = -FRAC_PI_2 + PI * step as f32 / 20.0;
                        let point = at(latitude, longitude);
                        if step == 0 {
                            path.move_to(point);
                        } else {
                            path.line_to(point);
                        }
                    }
                });
                frame.stroke(
                    &seam,
                    Stroke::default()
                        .with_color(scale(look.color, 0.6))
                        .with_width(line * longitude.cos()),
                );

                // Damar: tahtanın ortasında ince, dalgalı çizgi.
                let middle = longitude + TAU / boards as f32 / 2.0;
                if middle.cos() > 0.1 {
                    let grain = Path::new(|path| {
                        for step in 0..=24 {
                            let latitude = -FRAC_PI_2 + PI * step as f32 / 24.0;
                            let wobble = (latitude * 7.0 + board as f32).sin() * 0.04;
                            let point = at(latitude, middle + wobble);
                            if step == 0 {
                                path.move_to(point);
                            } else {
                                path.line_to(point);
                            }
                        }
                    });
                    frame.stroke(
                        &grain,
                        Stroke::default()
                            .with_color(Color { a: 0.45, ..second })
                            .with_width((line * 0.6).max(0.6)),
                    );
                }
            }
        }
        Pattern::Brushed | Pattern::Water => {
            let lines = if look.pattern == Pattern::Brushed {
                34
            } else {
                12
            };

            for index in 1..lines {
                let latitude = -FRAC_PI_2 + PI * index as f32 / lines as f32;
                let light = index % 2 == 0;
                let color = if light {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.1)
                } else {
                    Color::from_rgba(0.0, 0.0, 0.0, 0.1)
                };
                let path = Path::new(|path| {
                    for step in 0..=24 {
                        let longitude = -FRAC_PI_2 + PI * step as f32 / 24.0;
                        let wave = if look.pattern == Pattern::Water {
                            (longitude * 5.0 + index as f32).sin() * 0.03
                        } else {
                            0.0
                        };
                        let point = at(latitude + wave, longitude);
                        if step == 0 {
                            path.move_to(point);
                        } else {
                            path.line_to(point);
                        }
                    }
                });
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(if look.pattern == Pattern::Water {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.22)
                        } else {
                            color
                        })
                        .with_width((line * 0.7).max(0.6)),
                );
            }
        }
        Pattern::Speckle | Pattern::Gravel | Pattern::Stone | Pattern::Grass => {
            let (count, dot) = match look.pattern {
                Pattern::Speckle => (220, 0.018),
                Pattern::Gravel => (170, 0.05),
                Pattern::Stone => (46, 0.15),
                _ => (260, 0.03),
            };

            for index in 0..count {
                // Fibonacci küresi: noktalar ön yarıküreye eşit dağılır.
                let z = 1.0 - (index as f32 + 0.5) / count as f32;
                let ring = (1.0 - z * z).sqrt();
                let angle = index as f32 * 2.399_963;
                let (x, y) = (ring * angle.cos(), ring * angle.sin());
                let point = Point::new(c.x + x * r, c.y + y * r);
                let tilt = z.max(0.0).sqrt();
                let jitter = noise(index as u32, 7);
                let color = speckle(look, second, jitter);
                // Kürenin kenarına kalan yer: taneler dış çizgiyi aşmaz.
                let room = (1.0 - ring) * r * 1.1;

                match look.pattern {
                    Pattern::Grass => {
                        let length = r * 0.07 * (0.6 + jitter);
                        let end = Point::new(point.x + length * 0.3, point.y - length * tilt);

                        if end.distance(c) > r * 0.98 {
                            continue;
                        }

                        frame.stroke(
                            &Path::line(point, end),
                            Stroke::default()
                                .with_color(color)
                                .with_width((r * 0.018).max(0.7)),
                        );
                    }
                    Pattern::Stone => {
                        let size = (r * dot * (0.7 + 0.5 * jitter)).min(room);

                        if size < 1.0 {
                            continue;
                        }

                        frame.fill(&ellipse(point, size, size * tilt.max(0.25) * 0.85), color);
                        frame.stroke(
                            &ellipse(point, size, size * tilt.max(0.25) * 0.85),
                            Stroke::default()
                                .with_color(Color { a: 0.5, ..second })
                                .with_width(line * 0.8),
                        );
                    }
                    _ => {
                        let size = (r * dot * (0.6 + 0.8 * jitter)).max(0.5).min(room);

                        if size >= 0.4 {
                            frame.fill(&ellipse(point, size, size * tilt.max(0.3)), color);
                        }
                    }
                }
            }
        }
    }
}

/// Düz karo: desen, üstünde sol üstten gelen ışığın hafif geçişi.
fn swatch(frame: &mut Frame, look: &Look) {
    let size = frame.size();
    let side = size.width.min(size.height) - 2.0;
    let origin = Point::new((size.width - side) / 2.0, (size.height - side) / 2.0);
    let area = Rectangle::new(origin, Size::new(side, side));
    let square = Path::rectangle(area.position(), area.size());
    let second = look.second();
    let line = (side * 0.022).max(0.8);

    if look.pattern == Pattern::Glass {
        // Saydamlığı göstermek için arkada dama.
        let cell = side / 6.0;
        for row in 0..6 {
            for column in 0..6 {
                let dark = (row + column) % 2 == 0;
                frame.fill_rectangle(
                    Point::new(
                        origin.x + column as f32 * cell,
                        origin.y + row as f32 * cell,
                    ),
                    Size::new(cell, cell),
                    if dark {
                        Color::from_rgb(0.55, 0.57, 0.6)
                    } else {
                        Color::from_rgb(0.82, 0.84, 0.86)
                    },
                );
            }
        }
        frame.fill(
            &square,
            Color {
                a: 0.45,
                ..look.color
            },
        );
    } else {
        frame.fill(&square, look.color);
    }

    // Desen karonun içinde kalır. iced'in kırpılmış çizimi (`with_clip`)
    // önbellekli tuvalde içeriği düşürdüğü için kırpma elle yapılır:
    // dikdörtgenler kesişimleriyle, çizgiler uçları kırpılarak, taneler
    // kenara sığacak kadar küçültülerek çizilir.
    let side = area.width;
    let at = |x: f32, y: f32| Point::new(area.x + x.clamp(0.0, side), area.y + y.clamp(0.0, side));
    let rect = |frame: &mut Frame, x: f32, y: f32, width: f32, height: f32, color: Color| {
        let (left, top) = (x.max(0.0), y.max(0.0));
        let (right, bottom) = ((x + width).min(side), (y + height).min(side));

        if right > left && bottom > top {
            frame.fill_rectangle(
                Point::new(area.x + left, area.y + top),
                Size::new(right - left, bottom - top),
                color,
            );
        }
    };
    let inside = |x: f32| (0.0..=side).contains(&x);
    // Tanenin kenara sığacak yarıçapı.
    let fit = |x: f32, y: f32, radius: f32| radius.min(x).min(side - x).min(y).min(side - y);

    match look.pattern {
        Pattern::Plain | Pattern::Glass => {}
        Pattern::Brick | Pattern::Shingle | Pattern::Tile => {
            let (rows, columns) = match look.pattern {
                Pattern::Brick => (7, 3),
                Pattern::Tile => (4, 4),
                _ => (5, 4),
            };
            let height = side / rows as f32;
            let width = side / columns as f32;

            for row in 0..=rows {
                let y = height * row as f32;
                let offset = if look.pattern == Pattern::Tile || row % 2 == 0 {
                    0.0
                } else {
                    width / 2.0
                };

                // Tuğla tuğla renk farkı.
                if look.pattern != Pattern::Tile {
                    for column in -1..=columns {
                        let x = offset + width * column as f32;
                        let tone = noise((row * 31 + column + 40) as u32, 3) - 0.5;
                        rect(
                            frame,
                            x,
                            y,
                            width,
                            height,
                            scale(look.color, 1.0 + tone * 0.16),
                        );
                    }
                }

                // Kiremidin alt kenarında bindirmenin gölgesi.
                if look.pattern == Pattern::Shingle {
                    rect(
                        frame,
                        0.0,
                        y + height * 0.72,
                        side,
                        height * 0.28,
                        Color {
                            a: 0.3,
                            ..scale(look.color, 0.5)
                        },
                    );
                }

                if inside(y) {
                    frame.stroke(
                        &Path::line(at(0.0, y), at(side, y)),
                        Stroke::default().with_color(second).with_width(line),
                    );
                }

                for column in -1..=columns {
                    let x = offset + width * column as f32;

                    if inside(x) && y < side {
                        frame.stroke(
                            &Path::line(at(x, y), at(x, y + height)),
                            Stroke::default().with_color(second).with_width(line),
                        );
                    }
                }
            }
        }
        Pattern::Planks => {
            let boards = 5;
            let width = side / boards as f32;

            for board in 0..boards {
                let x = width * board as f32;
                let tone = noise(board as u32, 11) - 0.5;
                rect(
                    frame,
                    x,
                    0.0,
                    width,
                    side,
                    scale(look.color, 1.0 + tone * 0.14),
                );

                for grain in 0..3 {
                    let base = x + width * (0.25 + 0.25 * grain as f32);
                    let path = Path::new(|path| {
                        for step in 0..=16 {
                            let y = side * step as f32 / 16.0;
                            let wobble = (y / side * 9.0 + board as f32 * 1.7 + grain as f32).sin()
                                * width
                                * 0.06;
                            if step == 0 {
                                path.move_to(at(base + wobble, y));
                            } else {
                                path.line_to(at(base + wobble, y));
                            }
                        }
                    });
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_color(Color { a: 0.4, ..second })
                            .with_width((line * 0.6).max(0.6)),
                    );
                }

                if board > 0 {
                    frame.stroke(
                        &Path::line(at(x, 0.0), at(x, side)),
                        Stroke::default()
                            .with_color(scale(look.color, 0.55))
                            .with_width(line),
                    );
                }

                // Tahtaların uçları şaşırtmalı.
                let end = side * (0.3 + 0.4 * noise(board as u32, 5));
                frame.stroke(
                    &Path::line(at(x, end), at(x + width, end)),
                    Stroke::default()
                        .with_color(scale(look.color, 0.6))
                        .with_width(line * 0.8),
                );
            }
        }
        Pattern::Brushed | Pattern::Water => {
            let lines = if look.pattern == Pattern::Brushed {
                30
            } else {
                9
            };

            for index in 0..lines {
                let y = side * (index as f32 + 0.5) / lines as f32;
                let path = Path::new(|path| {
                    for step in 0..=20 {
                        let x = side * step as f32 / 20.0;
                        let wave = if look.pattern == Pattern::Water {
                            (x / side * TAU * 2.0 + index as f32).sin() * side * 0.012
                        } else {
                            0.0
                        };
                        if step == 0 {
                            path.move_to(at(x, y + wave));
                        } else {
                            path.line_to(at(x, y + wave));
                        }
                    }
                });
                let color = match (look.pattern, index % 2) {
                    (Pattern::Water, _) => Color::from_rgba(1.0, 1.0, 1.0, 0.24),
                    (_, 0) => Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                    _ => Color::from_rgba(0.0, 0.0, 0.0, 0.1),
                };
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(color)
                        .with_width((line * 0.6).max(0.6)),
                );
            }
        }
        Pattern::Speckle | Pattern::Gravel | Pattern::Stone | Pattern::Grass => {
            let (count, dot) = match look.pattern {
                Pattern::Speckle => (240, 0.012),
                Pattern::Gravel => (150, 0.035),
                Pattern::Stone => (30, 0.1),
                _ => (300, 0.02),
            };

            for index in 0..count {
                let x = side * noise(index, 1);
                let y = side * noise(index, 2);
                let jitter = noise(index, 3);
                let color = speckle(look, second, jitter);

                match look.pattern {
                    Pattern::Grass => {
                        let length = side * 0.06 * (0.6 + jitter);
                        frame.stroke(
                            &Path::line(at(x, y), at(x + length * 0.3, y - length)),
                            Stroke::default()
                                .with_color(color)
                                .with_width((side * 0.012).max(0.7)),
                        );
                    }
                    Pattern::Stone => {
                        let width = fit(x, y, side * dot * (0.8 + 0.6 * jitter));

                        if width > line * 2.0 {
                            let stone = ellipse(at(x, y), width, width * 0.8);
                            frame.fill(&stone, color);
                            frame.stroke(
                                &stone,
                                Stroke::default()
                                    .with_color(Color { a: 0.6, ..second })
                                    .with_width(line),
                            );
                        }
                    }
                    _ => {
                        let radius = fit(x, y, (side * dot * (0.5 + jitter)).max(0.5));

                        if radius > 0.3 {
                            frame.fill(&Path::circle(at(x, y), radius), color);
                        }
                    }
                }
            }
        }
    }

    // Işık: sol üstte hafif aydınlık, sağ altta hafif gölge; parlak
    // malzemede çapraz bir parıltı.
    let light = gradient::Linear::new(area.position(), area.position() + Vector::new(side, side))
        .add_stop(0.0, Color::from_rgba(1.0, 1.0, 1.0, 0.12))
        .add_stop(0.5, Color::from_rgba(1.0, 1.0, 1.0, 0.0))
        .add_stop(
            1.0,
            Color::from_rgba(0.0, 0.0, 0.0, 0.18 + look.metallic * 0.15),
        );
    frame.fill(&square, canvas::Gradient::Linear(light));

    let gloss = (1.0 - look.roughness).powf(2.0) * 0.32;
    if gloss > 0.02 {
        let streak = gradient::Linear::new(
            area.position() + Vector::new(0.0, side * 0.2),
            area.position() + Vector::new(side * 0.8, side),
        )
        .add_stop(0.35, Color::from_rgba(1.0, 1.0, 1.0, 0.0))
        .add_stop(0.5, Color::from_rgba(1.0, 1.0, 1.0, gloss))
        .add_stop(0.65, Color::from_rgba(1.0, 1.0, 1.0, 0.0));
        frame.fill(&square, canvas::Gradient::Linear(streak));
    }

    frame.stroke(
        &square,
        Stroke::default()
            .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.22))
            .with_width(1.0),
    );
}

/// Camın arkasındaki dama deseni, kürenin içinde.
fn checker(frame: &mut Frame, c: Point, r: f32, t: &Tokens) {
    let cell = r / 3.0;
    let (light, dark) = if t.is_dark {
        (
            Color::from_rgb(0.32, 0.35, 0.4),
            Color::from_rgb(0.2, 0.22, 0.26),
        )
    } else {
        (
            Color::from_rgb(0.86, 0.88, 0.9),
            Color::from_rgb(0.7, 0.73, 0.76),
        )
    };

    for row in -3_i32..3 {
        for column in -3_i32..3 {
            let x = c.x + column as f32 * cell;
            let y = c.y + row as f32 * cell;
            let middle = Point::new(x + cell / 2.0, y + cell / 2.0);

            if middle.distance(c) < r - cell * 0.35 {
                frame.fill_rectangle(
                    Point::new(x, y),
                    Size::new(cell, cell),
                    if (row + column).rem_euclid(2) == 0 {
                        dark
                    } else {
                        light
                    },
                );
            }
        }
    }
}

/// Benek, çakıl, taş ve çim tanelerinin rengi: temel renkle ikinci renk
/// arasında, tane tane değişen.
fn speckle(look: &Look, second: Color, jitter: f32) -> Color {
    match look.pattern {
        Pattern::Grass => scale(mix(look.color, second, jitter * 0.6), 0.8 + jitter * 0.45),
        Pattern::Stone => scale(look.color, 0.85 + jitter * 0.3),
        _ => Color {
            a: 0.75,
            ..mix(look.color, second, 0.4 + jitter * 0.6)
        },
    }
}

/// Elips yolu.
fn ellipse(center: Point, rx: f32, ry: f32) -> Path {
    Path::new(|path| {
        path.ellipse(canvas::path::arc::Elliptical {
            center,
            radii: Vector::new(rx.max(0.1), ry.max(0.1)),
            rotation: iced::Radians(0.0),
            start_angle: iced::Radians(0.0),
            end_angle: iced::Radians(TAU),
        });
    })
}

/// 0..1 arasında, girdilerine bağlı, her çizimde aynı sözde rastgele sayı.
fn noise(index: u32, seed: u32) -> f32 {
    let mut x = index
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(seed.wrapping_mul(0x85EB_CA6B));
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;

    (x & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

fn mix(a: Color, b: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);

    Color {
        r: a.r + (b.r - a.r) * amount,
        g: a.g + (b.g - a.g) * amount,
        b: a.b + (b.b - a.b) * amount,
        a: a.a + (b.a - a.a) * amount,
    }
}

fn scale(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r * amount).clamp(0.0, 1.0),
        g: (color.g * amount).clamp(0.0, 1.0),
        b: (color.b * amount).clamp(0.0, 1.0),
        a: color.a,
    }
}

fn luminance(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

/// Rengin onaltılık yazımı (#rrggbb).
pub fn hex(color: Color) -> String {
    let [red, green, blue, _] = color.into_rgba8();

    format!("#{red:02x}{green:02x}{blue:02x}")
}

/// Önizlemeyi Element'e çeviren kısayol: verilen kenar uzunluğunda kare.
pub fn tile<'a, Message: 'a>(look: Look, shape: Shape, side: f32) -> Element<'a, Message> {
    preview(look, shape).width(side).height(side).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_stable_and_in_range() {
        for index in 0..500 {
            let value = noise(index, 3);
            assert!((0.0..1.0).contains(&value));
            assert_eq!(value, noise(index, 3));
        }
        assert_ne!(noise(1, 1), noise(1, 2));
    }

    #[test]
    fn materials_match_by_name_category_id_and_tag() {
        let brick = Material::new(
            "tugla-kirmizi",
            "Kırmızı tuğla",
            "Tuğla",
            Look::new(Color::from_rgb8(0xa4, 0x55, 0x3f)).pattern(Pattern::Brick),
        )
        .tags(["cephe", "dış duvar"]);

        assert!(brick.matches("KIRMIZI"));
        assert!(brick.matches("tuğ"));
        assert!(brick.matches("kirmizi"));
        assert!(brick.matches("Cephe"));
        assert!(brick.matches(""));
        assert!(!brick.matches("cam"));
    }

    #[test]
    fn the_second_colour_contrasts_with_the_base() {
        let light = Look::new(Color::from_rgb(0.9, 0.9, 0.9));
        let dark = Look::new(Color::from_rgb(0.1, 0.1, 0.1));

        assert!(luminance(light.second()) < luminance(light.color));
        assert!(luminance(dark.second()) > luminance(dark.color));
        assert_eq!(
            Look::new(Color::BLACK).accent(Color::WHITE).second(),
            Color::WHITE
        );
    }

    #[test]
    fn look_values_stay_in_range() {
        let look = Look::new(Color::WHITE).roughness(4.0).metallic(-1.0);

        assert_eq!(look.roughness, 1.0);
        assert_eq!(look.metallic, 0.0);
    }
}
