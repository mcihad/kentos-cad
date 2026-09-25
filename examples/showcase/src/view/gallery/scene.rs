//! Görünüm alanı örneğinin sahnesi: tuvale çizilen basit bir ev modeli.
//!
//! Galerideki örnek içindir; üst, ön, sağ ve perspektif bakışlarla, tel
//! kafes, gizli çizgi ve gölgeli stillerde çizilir. Yüzler ressam
//! yöntemiyle (uzaktan yakına) sıralanır.

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme, Vector, mouse};

use kentos_rc::theme::Tokens;

/// Bakış yönü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Camera {
    Top,
    Front,
    Right,
    Perspective,
}

impl Camera {
    pub const ALL: [Camera; 4] = [
        Camera::Top,
        Camera::Front,
        Camera::Right,
        Camera::Perspective,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Camera::Top => "Üst",
            Camera::Front => "Ön",
            Camera::Right => "Sağ",
            Camera::Perspective => "Perspektif",
        }
    }
}

/// Görsel stil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shading {
    Wireframe,
    Hidden,
    Shaded,
    ShadedEdges,
}

impl Shading {
    pub const ALL: [Shading; 4] = [
        Shading::Wireframe,
        Shading::Hidden,
        Shading::Shaded,
        Shading::ShadedEdges,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Shading::Wireframe => "Tel kafes",
            Shading::Hidden => "Gizli çizgi",
            Shading::Shaded => "Gölgeli",
            Shading::ShadedEdges => "Kenarlı gölgeli",
        }
    }
}

/// Evin köşeleri (metre): 4 × 3 m taban, 2,5 m duvar, 4 m mahya.
const VERTICES: [[f32; 3]; 10] = [
    [0.0, 0.0, 0.0],
    [4.0, 0.0, 0.0],
    [4.0, 3.0, 0.0],
    [0.0, 3.0, 0.0],
    [0.0, 0.0, 2.5],
    [4.0, 0.0, 2.5],
    [4.0, 3.0, 2.5],
    [0.0, 3.0, 2.5],
    [0.0, 1.5, 4.0],
    [4.0, 1.5, 4.0],
];

/// Yüzler ve gölgelemedeki aydınlıkları (duvarlar, çatı, taban).
const FACES: [(&[usize], f32); 7] = [
    (&[0, 1, 5, 4], 0.78),
    (&[2, 3, 7, 6], 0.55),
    (&[3, 0, 4, 8, 7], 0.66),
    (&[1, 2, 6, 9, 5], 0.9),
    (&[4, 5, 9, 8], 1.0),
    (&[6, 7, 8, 9], 0.7),
    (&[0, 3, 2, 1], 0.4),
];

/// Tuvalde çizilen sahne. Perspektif bakış `yaw` derece döner.
pub struct Scene {
    pub camera: Camera,
    pub shading: Shading,
    pub yaw: f32,
}

/// Perspektif bakışın açılıştaki dönüşü.
pub const YAW: f32 = -35.0;

impl Scene {
    /// Noktanın bakıştaki ekran konumu (yukarı eksi y) ve derinliği
    /// (büyük değer yakın).
    fn project(&self, [x, y, z]: [f32; 3]) -> (Point, f32) {
        match self.camera {
            Camera::Top => (Point::new(x, -y), z),
            Camera::Front => (Point::new(x, -z), -y),
            Camera::Right => (Point::new(y, -z), x),
            Camera::Perspective => {
                // Modelin ortası çevresinde döndürülür, sonra eğilir.
                let (cx, cy, cz) = (x - 2.0, y - 1.5, z - 1.5);
                let yaw = self.yaw.to_radians();
                let pitch = 28.0_f32.to_radians();
                let (sy, cyaw) = yaw.sin_cos();
                let rx = cx * cyaw - cy * sy;
                let ry = cx * sy + cy * cyaw;
                let (sp, cp) = pitch.sin_cos();
                let up = cz * cp + ry * sp;
                let depth = ry * cp - cz * sp;
                let scale = 9.0 / (9.0 + depth);

                (Point::new(rx * scale, -up * scale), -depth)
            }
        }
    }
}

impl<Message> canvas::Program<Message> for Scene {
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

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), t.field);

        let projected: Vec<(Point, f32)> = VERTICES
            .iter()
            .map(|vertex| self.project(*vertex))
            .collect();

        // Modeli alanın ortasına, kenarlardan pay bırakarak sığdır.
        let (min, max) = projected.iter().fold(
            (
                Point::new(f32::MAX, f32::MAX),
                Point::new(f32::MIN, f32::MIN),
            ),
            |(min, max), (point, _)| {
                (
                    Point::new(min.x.min(point.x), min.y.min(point.y)),
                    Point::new(max.x.max(point.x), max.y.max(point.y)),
                )
            },
        );
        let extent = Vector::new((max.x - min.x).max(0.001), (max.y - min.y).max(0.001));
        let margin = 36.0;
        let scale = ((bounds.width - 2.0 * margin) / extent.x)
            .min((bounds.height - 2.0 * margin) / extent.y)
            .max(0.1);
        let offset = Vector::new(
            (bounds.width - extent.x * scale) / 2.0 - min.x * scale,
            (bounds.height - extent.y * scale) / 2.0 - min.y * scale + 8.0,
        );
        let screen = |point: Point| Point::new(point.x * scale, point.y * scale) + offset;

        let base = t.accent;
        let edge = if self.shading == Shading::Wireframe {
            t.accent
        } else {
            t.text.scale_alpha(0.85)
        };

        // Uzaktan yakına sıralı yüzler.
        let mut faces: Vec<&(&[usize], f32)> = FACES.iter().collect();
        faces.sort_by(|a, b| {
            let depth = |face: &[usize]| {
                face.iter().map(|index| projected[*index].1).sum::<f32>() / face.len() as f32
            };

            depth(a.0).total_cmp(&depth(b.0))
        });

        for (face, light) in faces {
            let path = Path::new(|builder| {
                builder.move_to(screen(projected[face[0]].0));

                for index in &face[1..] {
                    builder.line_to(screen(projected[*index].0));
                }

                builder.close();
            });

            match self.shading {
                Shading::Wireframe => {}
                Shading::Hidden => frame.fill(&path, t.field),
                Shading::Shaded | Shading::ShadedEdges => frame.fill(
                    &path,
                    Color {
                        r: base.r * light,
                        g: base.g * light,
                        b: base.b * light,
                        a: 1.0,
                    },
                ),
            }

            if self.shading != Shading::Shaded {
                frame.stroke(&path, Stroke::default().with_color(edge).with_width(1.4));
            }
        }

        vec![frame.into_geometry()]
    }
}
