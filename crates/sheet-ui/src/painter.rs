//! A map frame's content is the host's (design §11): the core lays out the
//! frame, its grid and its labels; what the drawing looks like inside is
//! painted by the program that owns the drawing. The stage asks for it on a
//! canvas of its own per map, and keeps what was painted while the map's
//! view, its size on the screen and the painter's [`MapPainter::revision`]
//! stay the same.
//!
//! The desktop paints with its drawing pipeline's scene (kentos-render-wgpu,
//! apps/desktop/src/sheet_maps.rs); the tests and the screens use
//! [`DemoMaps`], a made-up neighbourhood that is the same wherever it is
//! looked at from.

use std::rc::Rc;

use iced::widget::canvas::{self, Frame, Path};
use iced::{Color, Point, Rectangle, Size};
use kentos_sheet::display::{MapPrim, MapViewPrim};
use kentos_sheet::kinds::MapLayers;
use kentos_sheet::units::sin_cos;

/// What a map frame shows and how large it is on the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct MapRequest<'a> {
    /// The map item's id.
    pub item: &'a str,
    /// The content's size, pixels (the frame turned with it, if it is turned).
    pub size: Size,
    /// The view: the ground point at the content's middle, the scale, the content's turn.
    pub view: &'a MapViewPrim,
    pub layers: &'a MapLayers,
    pub crs: Option<&'a str>,
    /// The ground the content covers: min east, min north, max east, max north.
    pub extent: Option<[f64; 4]>,
    /// Pixels per metre on the ground.
    pub pixels_per_metre: f64,
    /// For a file (an export): print as the drawing prints, nothing of the screen's.
    pub export: bool,
}

impl<'a> MapRequest<'a> {
    /// The request of a map primitive drawn `size` pixels large at `k` pixels per micrometre.
    pub fn of(m: &'a MapPrim, size: Size, k: f64, export: bool) -> Self {
        // A metre on the ground is 1 000 000 / scale micrometres on the paper.
        let ppm = k * 1_000_000.0 / f64::from(m.view.scale.max(1));
        MapRequest {
            item: &m.item,
            size,
            view: &m.view,
            layers: &m.layers,
            crs: m.crs.as_deref(),
            extent: m.extent,
            pixels_per_metre: ppm,
            export,
        }
    }

    /// The content's box, pixels from its top-left corner: what a painter
    /// draws keeps inside it (a turned frame turns the box with it, and a
    /// renderer does not cut a canvas's shapes to a turned box).
    pub fn bounds(&self) -> Rectangle {
        Rectangle::new(Point::ORIGIN, self.size)
    }

    /// Where a ground point (east, north) falls in the content, pixels from
    /// its top-left corner: the core's own rule (its grids land on the same
    /// pixels).
    pub fn to_px(&self, east: f64, north: f64) -> Point {
        let c = self
            .view
            .center
            .unwrap_or(kentos_sheet::kinds::GroundPoint { x: 0.0, y: 0.0 });
        let (a, b) = (
            (east - c.x) * self.pixels_per_metre,
            (north - c.y) * self.pixels_per_metre,
        );
        let (s, co) = sin_cos(self.view.rotation);
        Point::new(
            (f64::from(self.size.width) / 2.0 + a * co + b * s) as f32,
            (f64::from(self.size.height) / 2.0 + a * s - b * co) as f32,
        )
    }
}

/// Paints the content of map frames.
pub trait MapPainter {
    /// Paints the content into `frame`, whose origin is the content's top-left
    /// corner; what it draws keeps inside [`MapRequest::bounds`] ([`clip_segment`],
    /// [`clip_polygon`]; a text wholly inside or not at all). False when there
    /// is nothing to show (the stage draws its placeholder then). Called only
    /// for a map that has a place (a centre).
    fn paint(&self, request: &MapRequest<'_>, frame: &mut Frame) -> bool;

    /// Changes whenever what [`paint`](Self::paint) draws would (the drawing
    /// was edited, a layer hidden): the stage paints its maps again.
    fn revision(&self) -> u64 {
        0
    }

    /// The map's content as vectors for a PDF (`MapContent::Vector`: the drawing's paths and
    /// texts on the ground, design §9a); none: the PDF has the map as a picture
    /// ([`paint`](Self::paint) at the export's resolution).
    fn vector(&self, request: &MapRequest<'_>) -> Option<kentos_sheet::pdf::MapContent> {
        let _ = request;
        None
    }

    /// Whether [`vector`](Self::vector) gives the maps' vectors (the export window says how the
    /// maps go before anything is written).
    fn vectors(&self) -> bool {
        false
    }

    /// What keeps a map from going to a PDF as vectors (the web's `mapWays` reason: “Yapı
    /// (desen dolgusu)”); none when it goes as vectors. Asked only of a painter that
    /// [`vectors`](Self::vectors).
    fn no_vectors(&self, _request: &MapRequest<'_>) -> Option<String> {
        None
    }
}

/// A painter as the views keep it: shared by the canvases of the maps they
/// draw (the stage's, the gallery's pictures). The host's may borrow what it
/// paints from (its drawing) for as long as the view lives.
pub type Painter<'a> = Rc<dyn MapPainter + 'a>;

/// A borrowed painter as a shared one (an export paints with what it is lent).
pub fn lend<'a>(painter: &'a dyn MapPainter) -> Painter<'a> {
    Rc::new(Lent(painter))
}

struct Lent<'a>(&'a dyn MapPainter);

impl MapPainter for Lent<'_> {
    fn paint(&self, request: &MapRequest<'_>, frame: &mut Frame) -> bool {
        self.0.paint(request, frame)
    }

    fn revision(&self) -> u64 {
        self.0.revision()
    }

    fn vector(&self, request: &MapRequest<'_>) -> Option<kentos_sheet::pdf::MapContent> {
        self.0.vector(request)
    }

    fn vectors(&self) -> bool {
        self.0.vectors()
    }

    fn no_vectors(&self, request: &MapRequest<'_>) -> Option<String> {
        self.0.no_vectors(request)
    }
}

/// The part of a segment inside the box (Liang–Barsky); none when it misses it.
pub fn clip_segment(a: Point, b: Point, bounds: Rectangle) -> Option<(Point, Point)> {
    crate::paint::clip_line(a, b, Some(bounds))
}

/// A closed polygon cut to the box (Sutherland–Hodgman); empty when it misses it.
pub fn clip_polygon(points: &[Point], bounds: Rectangle) -> Vec<Point> {
    crate::paint::clip_polygon(points, bounds)
}

/// A closed path through points (none for fewer than three).
fn closed(points: &[Point]) -> Option<Path> {
    (points.len() >= 3).then(|| {
        Path::new(|p| {
            p.move_to(points[0]);
            for q in &points[1..] {
                p.line_to(*q);
            }
            p.close();
        })
    })
}

/// No content: every map shows its placeholder.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoMaps;

impl MapPainter for NoMaps {
    fn paint(&self, _: &MapRequest<'_>, _: &mut Frame) -> bool {
        false
    }
}

/// A made-up neighbourhood for the tests and the screens: blocks of parcels
/// along a road grid, a stream and a park, fixed on the ground, so a map
/// shows the same streets wherever and at whatever scale it looks.
#[derive(Clone, Copy, Debug, Default)]
pub struct DemoMaps;

/// A cheap, fixed hash of a block's place: which blocks are parks and how they are split.
fn hash(x: i64, y: i64) -> u64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^ (h >> 29)
}

impl MapPainter for DemoMaps {
    fn paint(&self, r: &MapRequest<'_>, frame: &mut Frame) -> bool {
        let Some(e) = r.extent else {
            return false;
        };
        let block = 120.0_f64;
        let road = 12.0_f64;
        frame.fill_rectangle(Point::ORIGIN, r.size, Color::from_rgb8(0xf4, 0xf1, 0xe8));
        let (x0, x1) = (
            (e[0] / block).floor() as i64 - 1,
            (e[2] / block).ceil() as i64 + 1,
        );
        let (y0, y1) = (
            (e[1] / block).floor() as i64 - 1,
            (e[3] / block).ceil() as i64 + 1,
        );
        if (x1 - x0) * (y1 - y0) > 40_000 {
            // Too far out to tell blocks apart: the town as a tint.
            frame.fill_rectangle(Point::ORIGIN, r.size, Color::from_rgb8(0xe6, 0xdf, 0xd0));
            return true;
        }
        let parcel_fill = Color::from_rgb8(0xfb, 0xf7, 0xee);
        let park = Color::from_rgb8(0xd6, 0xe9, 0xc8);
        let line = Color::from_rgb8(0x7a, 0x6a, 0x58);
        let bounds = r.bounds();
        let corners = |a: [f64; 2], b: [f64; 2]| {
            [
                r.to_px(a[0], a[1]),
                r.to_px(b[0], a[1]),
                r.to_px(b[0], b[1]),
                r.to_px(a[0], b[1]),
            ]
        };
        // A block's or a parcel's fill and outline, kept to the content's box.
        let fill = |frame: &mut Frame, pts: &[Point; 4], color: Color| {
            if let Some(p) = closed(&clip_polygon(pts, bounds)) {
                frame.fill(&p, color);
            }
        };
        let outline = |frame: &mut Frame, pts: &[Point; 4], width: f32| {
            let edges = Path::new(|p| {
                for i in 0..4 {
                    if let Some((a, b)) = clip_segment(pts[i], pts[(i + 1) % 4], bounds) {
                        p.move_to(a);
                        p.line_to(b);
                    }
                }
            });
            frame.stroke(
                &edges,
                canvas::Stroke::default().with_color(line).with_width(width),
            );
        };
        let thin = (r.pixels_per_metre * 0.25).clamp(0.6, 1.4) as f32;
        for bx in x0..x1 {
            for by in y0..y1 {
                let (ex, ny) = (
                    bx as f64 * block + road / 2.0,
                    by as f64 * block + road / 2.0,
                );
                let size = block - road;
                let h = hash(bx, by);
                if h.is_multiple_of(11) {
                    fill(frame, &corners([ex, ny], [ex + size, ny + size]), park);
                    continue;
                }
                // Two rows of parcels, 3 to 5 across.
                let across = 3 + (h % 3) as usize;
                for row in 0..2 {
                    for i in 0..across {
                        let w = size / across as f64;
                        let a = [ex + w * i as f64, ny + size / 2.0 * row as f64];
                        let b = [a[0] + w, a[1] + size / 2.0];
                        let p = corners(a, b);
                        fill(frame, &p, parcel_fill);
                        outline(frame, &p, thin);
                        // A building in most parcels.
                        if (h >> (row * 8 + i)) & 3 != 0 {
                            let m = w * 0.22;
                            let building = corners(
                                [a[0] + m, a[1] + m],
                                [b[0] - m, a[1] + m + (b[1] - a[1]) * 0.45],
                            );
                            fill(frame, &building, Color::from_rgb8(0xd9, 0xcf, 0xc0));
                            outline(frame, &building, thin * 0.8);
                        }
                    }
                }
            }
        }
        // A stream across the town, east to north-east: its pieces inside the box
        // (a wide stroke may reach a few pixels past the edge of an upright frame).
        let stream = Path::new(|p| {
            let mut last: Option<Point> = None;
            let mut x = e[0] - block;
            while x <= e[2] + block {
                let y = x * 0.35 + 40.0 * (x / 90.0).sin();
                let q = r.to_px(x, y);
                if let Some((a, b)) = last.and_then(|l| clip_segment(l, q, bounds)) {
                    p.move_to(a);
                    p.line_to(b);
                }
                last = Some(q);
                x += 8.0;
            }
        });
        frame.stroke(
            &stream,
            canvas::Stroke::default()
                .with_color(Color::from_rgb8(0x6f, 0xa8, 0xd6))
                .with_width((r.pixels_per_metre * 6.0).clamp(1.5, 14.0) as f32),
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_sheet::kinds::GroundPoint;

    /// What a painter draws keeps inside the content's box, the content turned or not: a turned
    /// frame turns that box, so the map never spills out of its frame.
    #[test]
    fn the_demo_town_keeps_inside_the_content_s_box() {
        for rotation in [0, 30_000] {
            let view = MapViewPrim {
                center: Some(GroundPoint {
                    x: 1_000.0,
                    y: 2_000.0,
                }),
                scale: 1000,
                rotation,
            };
            let layers = MapLayers::default();
            let r = MapRequest {
                item: "m",
                size: Size::new(160.0, 90.0),
                view: &view,
                layers: &layers,
                crs: None,
                extent: Some([800.0, 1_800.0, 1_200.0, 2_200.0]),
                pixels_per_metre: 1.0,
                export: false,
            };
            struct One<'a>(MapRequest<'a>);
            impl canvas::Program<()> for One<'_> {
                type State = ();
                fn draw(
                    &self,
                    _: &(),
                    renderer: &iced::Renderer,
                    _: &iced::Theme,
                    b: Rectangle,
                    _: iced::mouse::Cursor,
                ) -> Vec<canvas::Geometry> {
                    let mut f = Frame::new(renderer, b.size());
                    f.fill_rectangle(Point::ORIGIN, b.size(), Color::WHITE);
                    assert!(DemoMaps.paint(&self.0, &mut f));
                    vec![f.into_geometry()]
                }
            }
            let size = Size::new(240.0, 160.0);
            let view: iced::Element<'_, ()> = iced::widget::canvas(One(r))
                .width(iced::Fill)
                .height(iced::Fill)
                .into();
            let image = kentos_ui::snapshot::Snapshot::software(size)
                .expect("a renderer")
                .render(view, &iced::Theme::Light);
            let (mut inside, mut outside) = (0, 0);
            for (i, p) in image.rgba.chunks_exact(4).enumerate() {
                let (x, y) = ((i % 240) as f32, (i / 240) as f32);
                if p[0] < 250 || p[1] < 250 || p[2] < 250 {
                    if x <= 161.0 && y <= 91.0 {
                        inside += 1;
                    } else {
                        outside += 1;
                    }
                }
            }
            assert!(inside > 1000, "the town is drawn: {inside}");
            assert_eq!(outside, 0, "turned {rotation}: nothing past the box");
        }
    }

    #[test]
    fn a_ground_point_lands_where_the_core_puts_it() {
        let view = MapViewPrim {
            center: Some(GroundPoint {
                x: 500_000.0,
                y: 4_400_000.0,
            }),
            scale: 1000,
            rotation: 0,
        };
        let layers = MapLayers::default();
        let r = MapRequest {
            item: "m",
            size: Size::new(200.0, 100.0),
            view: &view,
            layers: &layers,
            crs: None,
            extent: None,
            pixels_per_metre: 2.0,
            export: false,
        };
        assert_eq!(r.to_px(500_000.0, 4_400_000.0), Point::new(100.0, 50.0));
        // East is right, north is up.
        assert_eq!(r.to_px(500_010.0, 4_400_005.0), Point::new(120.0, 40.0));
        // Turned a quarter clockwise: east points down.
        let turned = MapViewPrim {
            rotation: 90_000,
            ..view.clone()
        };
        let r = MapRequest { view: &turned, ..r };
        let p = r.to_px(500_010.0, 4_400_000.0);
        assert!(
            (p.x - 100.0).abs() < 1e-3 && (p.y - 70.0).abs() < 1e-3,
            "{p:?}"
        );
    }
}
