//! 3B nesnelerin hafif önizlemesi: çokgen yüzlü, yüz başına renkli ağ
//! ([`Mesh`]), basit yapı taşları ve tuvale çizilen bakış ([`View`]).
//!
//! Koordinatlar metredir; x doğu, y kuzey, z yukarı (CLAUDE.md §5). Yüzler
//! dışarıdan bakınca saat yönünün tersine dizilir; bakışa sırtını dönen
//! yüzler çizilmez, kalanlar uzaktan yakına sıralanır (ressam yöntemi).
//! Gölgeleme yüzün normaliyle ışık arasındaki açıdan gelir.
//!
//! Önizleme iki türlüdür: küçük resim (sabit bakış) ve ayrıntı (sürükleyerek
//! döner, çift tık ilk bakışa döndürür; zeminde ölçü ızgarası, kenarlarında
//! genişlik, derinlik ve yükseklik ölçüleri). Görüntü ya da GPU gerekmez:
//! modeller buluttan gelmeden de ağlarıyla önizlenir.
//!
//! ```ignore
//! let house = Rc::new(Mesh::gable([8.0, 6.0], 3.0, 5.2, wall, roof));
//! mesh::view(house.clone()).width(72).height(72)          // küçük resim
//! mesh::detail(house, Camera::DEFAULT).height(220)          // döndürülebilir
//! ```

use std::cell::{Cell, RefCell};
use std::f32::consts::{PI, TAU};
use std::rc::Rc;

use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Length, Pixels, Point, Rectangle, Renderer, Theme, Vector, mouse};

use crate::theme::{Tokens, typography};

/// Ağın bir yüzü: köşelerinin sırası ve rengi.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    pub indices: Vec<usize>,
    pub color: Color,
}

/// Çokgen yüzlü, yüz başına renkli ağ.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<Face>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    /// Köşeleri verilen yüz ekler (dışarıdan bakınca saat yönünün tersine).
    pub fn face(&mut self, points: &[[f32; 3]], color: Color) {
        let start = self.vertices.len();
        self.vertices.extend_from_slice(points);
        self.faces.push(Face {
            indices: (start..start + points.len()).collect(),
            color,
        });
    }

    /// Öbür ağı bununla birleştirir.
    pub fn merge(mut self, other: Mesh) -> Self {
        let offset = self.vertices.len();
        self.vertices.extend(other.vertices);
        self.faces.extend(other.faces.into_iter().map(|face| {
            Face {
                indices: face
                    .indices
                    .into_iter()
                    .map(|index| index + offset)
                    .collect(),
                color: face.color,
            }
        }));
        self
    }

    /// Ağı öteler (metre).
    pub fn translate(mut self, [dx, dy, dz]: [f32; 3]) -> Self {
        for vertex in &mut self.vertices {
            vertex[0] += dx;
            vertex[1] += dy;
            vertex[2] += dz;
        }
        self
    }

    /// Kutu: iki karşı köşesiyle.
    pub fn cuboid(min: [f32; 3], max: [f32; 3], color: Color) -> Self {
        let footprint = [
            [min[0], min[1]],
            [max[0], min[1]],
            [max[0], max[1]],
            [min[0], max[1]],
        ];

        Self::prism(&footprint, min[2], max[2], color, color)
    }

    /// Tabanı çokgen olan dik prizma (saat yönünün tersine taban): yan
    /// yüzler `wall`, üst ve alt `top` renginde.
    pub fn prism(footprint: &[[f32; 2]], z0: f32, z1: f32, wall: Color, top: Color) -> Self {
        let mut mesh = Self::new();
        let n = footprint.len();

        if n < 3 {
            return mesh;
        }

        for i in 0..n {
            let [ax, ay] = footprint[i];
            let [bx, by] = footprint[(i + 1) % n];
            mesh.face(
                &[[ax, ay, z0], [bx, by, z0], [bx, by, z1], [ax, ay, z1]],
                wall,
            );
        }

        let roof: Vec<[f32; 3]> = footprint.iter().map(|[x, y]| [*x, *y, z1]).collect();
        let floor: Vec<[f32; 3]> = footprint.iter().rev().map(|[x, y]| [*x, *y, z0]).collect();
        mesh.face(&roof, top);
        mesh.face(&floor, scale(top, 0.6));
        mesh
    }

    /// Beşik çatılı yapı: `size` taban (doğu, kuzey), duvar ve mahya
    /// yüksekliği; mahya doğu-batı yönünde.
    pub fn gable(size: [f32; 2], eaves: f32, ridge: f32, wall: Color, roof: Color) -> Self {
        let [w, d] = size;
        let mut mesh = Self::cuboid([0.0, 0.0, 0.0], [w, d, eaves], wall);
        // Kutunun üst yüzü çatının altında kalır; kaldırılır.
        mesh.faces.retain(|face| {
            !face
                .indices
                .iter()
                .all(|index| (mesh.vertices[*index][2] - eaves).abs() < 1e-4)
        });

        let overhang = 0.35;
        let (x0, x1) = (-overhang, w + overhang);
        let (y0, y1) = (-overhang, d + overhang);
        let mid = d / 2.0;
        let low = eaves - overhang * (ridge - eaves) / mid;

        // Alınlıklar (üçgen duvarlar), dışarıdan bakınca saat yönünün tersine.
        mesh.face(
            &[[0.0, d, eaves], [0.0, 0.0, eaves], [0.0, mid, ridge]],
            wall,
        );
        mesh.face(&[[w, 0.0, eaves], [w, d, eaves], [w, mid, ridge]], wall);
        // Çatı yüzleri.
        mesh.face(
            &[
                [x0, y0, low],
                [x1, y0, low],
                [x1, mid, ridge],
                [x0, mid, ridge],
            ],
            roof,
        );
        mesh.face(
            &[
                [x1, y1, low],
                [x0, y1, low],
                [x0, mid, ridge],
                [x1, mid, ridge],
            ],
            roof,
        );
        mesh
    }

    /// Dik silindir: merkezi, yarıçapı, alt ve üst yüksekliği.
    pub fn cylinder(
        center: [f32; 2],
        radius: f32,
        z0: f32,
        z1: f32,
        segments: usize,
        color: Color,
    ) -> Self {
        Self::frustum(center, radius, radius, z0, z1, segments, color)
    }

    /// Koni: üstü sivri.
    pub fn cone(
        center: [f32; 2],
        radius: f32,
        z0: f32,
        z1: f32,
        segments: usize,
        color: Color,
    ) -> Self {
        Self::frustum(center, radius, 0.0, z0, z1, segments, color)
    }

    /// Kesik koni: altta ve üstte yarıçap.
    pub fn frustum(
        [cx, cy]: [f32; 2],
        bottom: f32,
        top: f32,
        z0: f32,
        z1: f32,
        segments: usize,
        color: Color,
    ) -> Self {
        let segments = segments.max(3);
        let mut mesh = Self::new();
        let ring = |radius: f32, z: f32, i: usize| {
            let angle = TAU * i as f32 / segments as f32;
            [cx + radius * angle.cos(), cy + radius * angle.sin(), z]
        };

        for i in 0..segments {
            let next = (i + 1) % segments;

            if top > 0.0 {
                mesh.face(
                    &[
                        ring(bottom, z0, i),
                        ring(bottom, z0, next),
                        ring(top, z1, next),
                        ring(top, z1, i),
                    ],
                    color,
                );
            } else {
                mesh.face(
                    &[ring(bottom, z0, i), ring(bottom, z0, next), [cx, cy, z1]],
                    color,
                );
            }
        }

        if top > 0.0 {
            let lid: Vec<[f32; 3]> = (0..segments).map(|i| ring(top, z1, i)).collect();
            mesh.face(&lid, color);
        }

        let base: Vec<[f32; 3]> = (0..segments).rev().map(|i| ring(bottom, z0, i)).collect();
        mesh.face(&base, scale(color, 0.6));
        mesh
    }

    /// Küre (ağaç tacı, kubbe): enlem ve boylam bölümleriyle; `half` ise
    /// yalnız üst yarısı.
    pub fn sphere(
        center: [f32; 3],
        radius: f32,
        segments: usize,
        half: bool,
        color: Color,
    ) -> Self {
        let around = segments.max(4);
        let rings = (around / 2).max(2);
        let mut mesh = Self::new();
        let first = if half { rings / 2 } else { 0 };
        let point = |ring: usize, i: usize| {
            let latitude = -PI / 2.0 + PI * ring as f32 / rings as f32;
            let longitude = TAU * i as f32 / around as f32;
            [
                center[0] + radius * latitude.cos() * longitude.cos(),
                center[1] + radius * latitude.cos() * longitude.sin(),
                center[2] + radius * latitude.sin(),
            ]
        };

        for ring in first..rings {
            for i in 0..around {
                let next = (i + 1) % around;
                let quad = [
                    point(ring, i),
                    point(ring, next),
                    point(ring + 1, next),
                    point(ring + 1, i),
                ];

                if ring + 1 == rings {
                    mesh.face(&[quad[0], quad[1], quad[2]], color);
                } else if ring == 0 {
                    mesh.face(&[quad[0], quad[2], quad[3]], color);
                } else {
                    mesh.face(&quad, color);
                }
            }
        }

        if half {
            let base: Vec<[f32; 3]> = (0..around).rev().map(|i| point(first, i)).collect();
            mesh.face(&base, scale(color, 0.6));
        }

        mesh
    }

    /// Kapsayan kutu: en küçük ve en büyük köşe.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        self.vertices.iter().fold(
            ([f32::MAX; 3], [f32::MIN; 3]),
            |(mut min, mut max), vertex| {
                for axis in 0..3 {
                    min[axis] = min[axis].min(vertex[axis]);
                    max[axis] = max[axis].max(vertex[axis]);
                }
                (min, max)
            },
        )
    }

    /// Boyutları (metre): doğu, kuzey ve yükseklik doğrultusunda.
    pub fn size(&self) -> [f32; 3] {
        if self.vertices.is_empty() {
            return [0.0; 3];
        }

        let (min, max) = self.bounds();
        [max[0] - min[0], max[1] - min[1], max[2] - min[2]]
    }

    /// Üçgen sayısı: her n köşeli yüz n − 2 üçgendir.
    pub fn triangles(&self) -> usize {
        self.faces
            .iter()
            .map(|face| face.indices.len().saturating_sub(2))
            .sum()
    }
}

/// Bakış açısı: dönüş (kuzeyden, derece) ve eğim (yataydan, derece).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
}

impl Camera {
    /// Küçük resimlerin ve ayrıntının ilk bakışı: güneybatıdan, yukarıdan.
    pub const DEFAULT: Camera = Camera {
        yaw: -38.0,
        pitch: 26.0,
    };
}

impl Default for Camera {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Küçük resim önizlemesi (sabit bakış); boyutu `width` ve `height` ile
/// verilir.
pub fn view<Message>(mesh: Rc<Mesh>) -> Canvas<View, Message> {
    Canvas::new(View {
        mesh,
        interactive: false,
        camera: Camera::DEFAULT,
    })
    .width(Length::Fill)
    .height(Length::Fill)
}

/// Ayrıntı önizlemesi: sürükleyerek döner (çift tık ilk bakışa döndürür);
/// zeminde ölçü ızgarası, kenarlarda genişlik, derinlik ve yükseklik.
pub fn detail<Message>(mesh: Rc<Mesh>, camera: Camera) -> Canvas<View, Message> {
    Canvas::new(View {
        mesh,
        interactive: true,
        camera,
    })
    .width(Length::Fill)
    .height(Length::Fill)
}

/// Önizlemenin tuval programı.
#[derive(Debug, Clone)]
pub struct View {
    mesh: Rc<Mesh>,
    interactive: bool,
    camera: Camera,
}

/// Önizlemenin durumu: önbellek, neyle çizildiği ve sürükleme.
#[derive(Default)]
pub struct ViewState {
    cache: Cache,
    key: RefCell<Option<(usize, bool, Camera)>>,
    /// Kullanıcının döndürdüğü bakış; yoksa programın bakışı.
    camera: Cell<Option<Camera>>,
    drag: Cell<Option<(Point, Camera)>>,
    last_click: Cell<Option<iced::time::Instant>>,
}

impl<Message> canvas::Program<Message> for View {
    type State = ViewState;

    fn update(
        &self,
        state: &mut ViewState,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if !self.interactive {
            return None;
        }

        let canvas::Event::Mouse(event) = event else {
            return None;
        };

        match event {
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let position = cursor.position_over(bounds)?;
                let now = iced::time::Instant::now();
                let double = state
                    .last_click
                    .get()
                    .is_some_and(|last| now.duration_since(last).as_millis() < 400);
                state.last_click.set(Some(now));

                if double {
                    state.camera.set(None);
                    state.drag.set(None);
                    state.cache.clear();
                    return Some(canvas::Action::request_redraw().and_capture());
                }

                let camera = state.camera.get().unwrap_or(self.camera);
                state.drag.set(Some((position, camera)));
                Some(canvas::Action::capture())
            }
            mouse::Event::CursorMoved { position } => {
                let (origin, start) = state.drag.get()?;
                let camera = Camera {
                    yaw: start.yaw + (position.x - origin.x) * 0.5,
                    pitch: (start.pitch + (position.y - origin.y) * 0.35).clamp(5.0, 85.0),
                };
                state.camera.set(Some(camera));
                state.cache.clear();
                Some(canvas::Action::request_redraw().and_capture())
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                state.drag.take()?;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        state: &ViewState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if !self.interactive {
            return mouse::Interaction::None;
        }

        if state.drag.get().is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        state: &ViewState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::of(theme);
        let camera = state.camera.get().unwrap_or(self.camera);
        let key = (Rc::as_ptr(&self.mesh) as usize, t.is_dark, camera);

        if state.key.borrow().as_ref() != Some(&key) {
            state.cache.clear();
            *state.key.borrow_mut() = Some(key);
        }

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            draw(frame, &self.mesh, camera, self.interactive, &t);
        });

        vec![geometry]
    }
}

/// Işığın geldiği yön (dünyada): güneybatıdan, yukarıdan.
const LIGHT: [f32; 3] = [-0.45, -0.6, 0.66];
/// Gölgede kalan yüzün aydınlığı.
const AMBIENT: f32 = 0.38;

/// Noktanın bakıştaki yeri: ekranda sağa ve yukarı, derinlik (büyük değer
/// uzak).
fn project([x, y, z]: [f32; 3], camera: Camera) -> (f32, f32, f32) {
    let yaw = camera.yaw.to_radians();
    let pitch = camera.pitch.to_radians();
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let rx = x * cy - y * sy;
    let ry = x * sy + y * cy;
    let up = z * cp + ry * sp;
    let depth = ry * cp - z * sp;

    (rx, up, depth)
}

/// Bakışın baktığı yön (dünyada): sahneden göze.
fn eye(camera: Camera) -> [f32; 3] {
    let yaw = camera.yaw.to_radians();
    let pitch = camera.pitch.to_radians();
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();

    // `project`'in derinlik ekseninin tersi.
    [-(sy * cp), -(cy * cp), sp]
}

/// Yüzün normali (Newell yöntemi), birim.
fn normal(mesh: &Mesh, face: &Face) -> [f32; 3] {
    let mut n = [0.0_f32; 3];

    for (i, &index) in face.indices.iter().enumerate() {
        let a = mesh.vertices[index];
        let b = mesh.vertices[face.indices[(i + 1) % face.indices.len()]];
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }

    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();

    if length < 1e-9 {
        [0.0, 0.0, 1.0]
    } else {
        [n[0] / length, n[1] / length, n[2] / length]
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt().max(1e-9);
    [v[0] / length, v[1] / length, v[2] / length]
}

/// Sahne: zemin ızgarası ve gölge, yüzler, ayrıntıda ölçüler.
fn draw(frame: &mut Frame, mesh: &Mesh, camera: Camera, detail: bool, t: &Tokens) {
    let size = frame.size();

    if mesh.vertices.is_empty() {
        return;
    }

    let (min, max) = mesh.bounds();
    let center = [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0, min[2]];
    let extent = mesh.size();
    let reach = extent[0].max(extent[1]).max(extent[2]).max(0.5);

    // Zeminin ızgarası ve ölçüler sığsın diye ayrıntıda kutu biraz büyük alınır.
    let pad = if detail { reach * 0.35 } else { reach * 0.08 };
    let corners = [
        [min[0] - pad, min[1] - pad, min[2]],
        [max[0] + pad, min[1] - pad, min[2]],
        [max[0] + pad, max[1] + pad, min[2]],
        [min[0] - pad, max[1] + pad, min[2]],
        [min[0], min[1], max[2]],
        [max[0], min[1], max[2]],
        [max[0], max[1], max[2]],
        [min[0], max[1], max[2]],
    ];
    let shifted = |p: [f32; 3]| [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
    let (mut left, mut right, mut bottom, mut top) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);

    for corner in corners {
        let (x, y, _) = project(shifted(corner), camera);
        left = left.min(x);
        right = right.max(x);
        bottom = bottom.min(y);
        top = top.max(y);
    }

    let margin = if detail { 26.0 } else { 6.0 };
    let width = (right - left).max(0.01);
    let height = (top - bottom).max(0.01);
    let zoom = ((size.width - 2.0 * margin) / width).min((size.height - 2.0 * margin) / height);
    let offset = Vector::new(
        (size.width - width * zoom) / 2.0 - left * zoom,
        (size.height - height * zoom) / 2.0 + top * zoom,
    );
    let screen = |p: [f32; 3]| {
        let (x, y, depth) = project(shifted(p), camera);
        (Point::new(x * zoom + offset.x, -y * zoom + offset.y), depth)
    };

    // Zeminin ölçü ızgarası (ayrıntıda): yuvarlak bir aralıkla, uzaklaştıkça söner.
    if detail {
        let step = grid_step(reach);
        let span = (reach * 0.9 / step).ceil() * step;
        let lines = (span / step) as i32;
        let base = t.text.scale_alpha(if t.is_dark { 0.1 } else { 0.12 });

        for i in -lines..=lines {
            let k = i as f32 * step;
            let alpha = 1.0 - (k.abs() / (span + step)).powf(1.5);
            let color = Color {
                a: base.a * alpha,
                ..base
            };
            let a = screen([center[0] + k, center[1] - span, min[2]]).0;
            let b = screen([center[0] + k, center[1] + span, min[2]]).0;
            frame.stroke(
                &Path::line(a, b),
                Stroke::default().with_color(color).with_width(1.0),
            );
            let a = screen([center[0] - span, center[1] + k, min[2]]).0;
            let b = screen([center[0] + span, center[1] + k, min[2]]).0;
            frame.stroke(
                &Path::line(a, b),
                Stroke::default().with_color(color).with_width(1.0),
            );
        }
    }

    // Temas gölgesi: tabanın izdüşümü çevresinde yumuşak koyuluk.
    let shadow = Path::new(|path| {
        let ring = [
            [min[0], min[1], min[2]],
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
        ];
        path.move_to(screen(ring[0]).0);
        for point in &ring[1..] {
            path.line_to(screen(*point).0);
        }
        path.close();
    });
    for (spread, alpha) in [(1.0_f32, 0.06_f32), (0.0, 0.1)] {
        frame.stroke(
            &shadow,
            Stroke::default()
                .with_color(Color::from_rgba(0.0, 0.0, 0.0, alpha))
                .with_width(2.0 + spread * 6.0),
        );
    }
    frame.fill(
        &shadow,
        Color::from_rgba(0.0, 0.0, 0.0, if t.is_dark { 0.22 } else { 0.1 }),
    );

    // Yüzler: bakışa dönük olanlar, uzaktan yakına.
    let light = unit(LIGHT);
    let toward = eye(camera);
    let mut visible: Vec<(f32, usize, f32)> = mesh
        .faces
        .iter()
        .enumerate()
        .filter_map(|(index, face)| {
            if face.indices.len() < 3 {
                return None;
            }

            let n = normal(mesh, face);

            if dot(n, toward) <= 0.0 {
                return None;
            }

            let depth = face
                .indices
                .iter()
                .map(|i| screen(mesh.vertices[*i]).1)
                .sum::<f32>()
                / face.indices.len() as f32;
            let lit = AMBIENT + (1.0 - AMBIENT) * dot(n, light).max(0.0);

            Some((depth, index, lit))
        })
        .collect();

    visible.sort_by(|a, b| b.0.total_cmp(&a.0));

    let edge = Color::from_rgba(0.0, 0.0, 0.0, if detail { 0.28 } else { 0.18 });

    for (_, index, lit) in visible {
        let face = &mesh.faces[index];
        let path = Path::new(|path| {
            path.move_to(screen(mesh.vertices[face.indices[0]]).0);
            for i in &face.indices[1..] {
                path.line_to(screen(mesh.vertices[*i]).0);
            }
            path.close();
        });

        frame.fill(&path, scale(face.color, lit));
        frame.stroke(&path, Stroke::default().with_color(edge).with_width(0.6));
    }

    if detail {
        dimensions(frame, &screen, min, max, extent, t);
    }
}

/// Ayrıntıdaki ölçüler: önde genişlik ve derinlik, yanda yükseklik; ince
/// çizgi, uçlarında kısa dikler ve değerin yazısı. Yazı çizginin modele
/// göre dış yanında, çizgiye dik doğrultuda durur.
fn dimensions(
    frame: &mut Frame,
    screen: &dyn Fn([f32; 3]) -> (Point, f32),
    min: [f32; 3],
    max: [f32; 3],
    extent: [f32; 3],
    t: &Tokens,
) {
    let gap = extent[0].max(extent[1]).max(0.5) * 0.12;
    let color = t.accent_hover;
    // Yazılar modelin zemindeki izinin ortasından uzağa gider: önde birleşen
    // iki ölçünün yazısı birbirine binmez.
    let middle = screen([(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0, min[2]]).0;

    let mut measure = |a: [f32; 3], b: [f32; 3], value: f32| {
        let (a, b) = (screen(a).0, screen(b).0);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let length = (dx * dx + dy * dy).sqrt();

        if length < 8.0 {
            return;
        }

        let along = Vector::new(dx / length, dy / length);
        let mut across = Vector::new(-along.y, along.x);
        let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);

        // Dik, modelin ortasından uzağa baksın.
        if (mid.x - middle.x) * across.x + (mid.y - middle.y) * across.y < 0.0 {
            across = Vector::new(-across.x, -across.y);
        }

        let stroke = Stroke::default().with_color(color).with_width(1.0);
        frame.stroke(&Path::line(a, b), stroke);

        for end in [a, b] {
            frame.stroke(&Path::line(end - across * 4.0, end + across * 4.0), stroke);
        }

        let text = format!("{} m", crate::attribute::number::real(f64::from(value), 1));
        let size = typography::caption();
        let width = typography::text_width(&text, size);
        // Yatay yazının yarı genişliği kadar daha uzağa: dik doğrultu yatayken.
        let room = 9.0 + across.x.abs() * width * 0.5;
        let at = mid + across * room;

        // Çizimin etiketleri gibi zemin renginde bir plaka: model üstünde de okunur.
        let plate = Path::rounded_rectangle(
            Point::new(at.x - width / 2.0 - 3.0, at.y - size * 0.7),
            iced::Size::new(width + 6.0, size * 1.4),
            iced::border::Radius::from(3.0),
        );
        frame.fill(&plate, t.field.scale_alpha(0.82));

        frame.fill_text(Text {
            content: text,
            position: at,
            color,
            size: Pixels(size),
            font: typography::ui(),
            align_x: iced::advanced::text::Alignment::Center,
            align_y: iced::alignment::Vertical::Center,
            ..Text::default()
        });
    };

    // Genişlik: güney kenarının önünde; derinlik: batı kenarının önünde;
    // yükseklik: güneydoğu köşesinde.
    measure(
        [min[0], min[1] - gap, min[2]],
        [max[0], min[1] - gap, min[2]],
        extent[0],
    );
    measure(
        [min[0] - gap, min[1], min[2]],
        [min[0] - gap, max[1], min[2]],
        extent[1],
    );
    measure(
        [max[0] + gap, min[1], min[2]],
        [max[0] + gap, min[1], max[2]],
        extent[2],
    );
}

/// Izgaranın aralığı: modelin boyuna göre 1-2-5 dizisinden.
fn grid_step(reach: f32) -> f32 {
    let raw = reach / 6.0;
    let power = 10f32.powf(raw.log10().floor());

    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|factor| factor * power)
        .find(|step| *step >= raw)
        .unwrap_or(10.0 * power)
}

fn scale(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r * amount).clamp(0.0, 1.0),
        g: (color.g * amount).clamp(0.0, 1.0),
        b: (color.b * amount).clamp(0.0, 1.0),
        a: color.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cuboid_has_six_outward_faces_and_twelve_triangles() {
        let mesh = Mesh::cuboid([0.0; 3], [2.0, 3.0, 4.0], Color::WHITE);

        assert_eq!(mesh.faces.len(), 6);
        assert_eq!(mesh.triangles(), 12);
        assert_eq!(mesh.size(), [2.0, 3.0, 4.0]);

        // Normaller dışarı bakar: yüzün ortasından kutunun ortasına doğru değil.
        let middle = [1.0, 1.5, 2.0];
        for face in &mesh.faces {
            let n = normal(&mesh, face);
            let p = mesh.vertices[face.indices[0]];
            let out = [p[0] - middle[0], p[1] - middle[1], p[2] - middle[2]];
            assert!(dot(n, out) > 0.0, "{face:?}");
        }
    }

    #[test]
    fn primitives_face_outward() {
        let check = |mesh: &Mesh, middle: [f32; 3]| {
            for face in &mesh.faces {
                let n = normal(mesh, face);
                let p = face
                    .indices
                    .iter()
                    .map(|i| mesh.vertices[*i])
                    .fold([0.0; 3], |s, v| [s[0] + v[0], s[1] + v[1], s[2] + v[2]]);
                let k = face.indices.len() as f32;
                let out = [
                    p[0] / k - middle[0],
                    p[1] / k - middle[1],
                    p[2] / k - middle[2],
                ];
                assert!(dot(n, out) > -1e-4, "{face:?}");
            }
        };

        check(
            &Mesh::cylinder([0.0, 0.0], 1.0, 0.0, 2.0, 12, Color::WHITE),
            [0.0, 0.0, 1.0],
        );
        check(
            &Mesh::cone([0.0, 0.0], 1.0, 0.0, 2.0, 12, Color::WHITE),
            [0.0, 0.0, 0.5],
        );
        check(
            &Mesh::sphere([0.0, 0.0, 0.0], 1.0, 12, false, Color::WHITE),
            [0.0; 3],
        );
        check(
            &Mesh::gable([8.0, 6.0], 3.0, 5.0, Color::WHITE, Color::BLACK),
            [4.0, 3.0, 2.0],
        );
    }

    #[test]
    fn faces_turned_away_from_the_eye_are_not_drawn() {
        let camera = Camera::DEFAULT;
        let toward = eye(camera);
        let mesh = Mesh::cuboid([0.0; 3], [1.0; 3], Color::WHITE);
        let shown = mesh
            .faces
            .iter()
            .filter(|face| dot(normal(&mesh, face), toward) > 0.0)
            .count();

        // Yukarıdan ve köşeden bakılan kutunun üç yüzü görünür.
        assert_eq!(shown, 3);
    }

    #[test]
    fn the_grid_step_follows_one_two_five() {
        assert_eq!(grid_step(6.0), 1.0);
        assert_eq!(grid_step(9.0), 2.0);
        assert_eq!(grid_step(24.0), 5.0);
        assert_eq!(grid_step(60.0), 10.0);
    }
}
