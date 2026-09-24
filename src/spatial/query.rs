//! Ekrana göre sorgular: tıklanan öğe (hit test), seçim penceresindeki
//! öğeler ve nesne yakalama.

use iced::{Point, Rectangle};

use super::{Bounds, Feature, FeatureRef, Geometry, Layer, LonLat, Viewport};

/// Nokta öğelerin seçim yarıçapı (piksel).
const POINT_TOLERANCE: f32 = 9.0;
/// Çizgilerin seçim mesafesi (piksel).
const LINE_TOLERANCE: f32 = 6.0;

/// Varsayılan nesne yakalama mesafesi (piksel).
pub const SNAP_TOLERANCE: f32 = 12.0;

/// Ekran noktasının altındaki öğe. Listede önce gelen katman ve katman
/// içinde sonra çizilen öğe önceliklidir.
pub fn hit_test(layers: &[Layer], viewport: &Viewport, point: Point) -> Option<FeatureRef> {
    layers
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.is_interactive())
        .find_map(|(index, layer)| hit_layer(index, layer, viewport, point))
}

/// Yalnızca verilen katmanda, ekran noktasının altındaki öğe (ör. haritadan
/// varlık seçerken hedef katman).
pub fn hit_test_in(
    layers: &[Layer],
    layer: usize,
    viewport: &Viewport,
    point: Point,
) -> Option<FeatureRef> {
    layers
        .get(layer)
        .filter(|candidate| candidate.is_interactive())
        .and_then(|candidate| hit_layer(layer, candidate, viewport, point))
}

fn hit_layer(index: usize, layer: &Layer, viewport: &Viewport, point: Point) -> Option<FeatureRef> {
    layer
        .features
        .iter()
        .rev()
        .filter(|feature| layer.shows(feature))
        .find(|feature| is_hit(&feature.geometry, viewport, point))
        .map(|feature| FeatureRef::new(index, feature.id))
}

/// Seçim penceresindeki öğeler. Pencere seçiminde (`crossing` yanlış)
/// yalnızca tamamı içeride kalan öğeler, kesişen seçimde pencereye değen
/// ya da pencereyi içine alan öğeler de seçilir; AutoCAD'deki gibi.
pub fn in_bounds(
    layers: &[Layer],
    viewport: &Viewport,
    bounds: Bounds,
    crossing: bool,
) -> Vec<FeatureRef> {
    let corner_a = viewport.project(bounds.south_west);
    let corner_b = viewport.project(bounds.north_east);
    let window = Rectangle {
        x: corner_a.x.min(corner_b.x),
        y: corner_a.y.min(corner_b.y),
        width: (corner_a.x - corner_b.x).abs(),
        height: (corner_a.y - corner_b.y).abs(),
    };

    layers
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.is_interactive())
        .flat_map(|(index, layer)| {
            layer
                .features
                .iter()
                .filter(move |feature| layer.shows(feature))
                .filter(move |feature| is_in_window(feature, viewport, window, crossing))
                .map(move |feature| FeatureRef::new(index, feature.id))
        })
        .collect()
}

fn is_in_window(feature: &Feature, viewport: &Viewport, window: Rectangle, crossing: bool) -> bool {
    let points: Vec<Point> = feature
        .geometry
        .vertices()
        .iter()
        .map(|location| viewport.project(*location))
        .collect();

    if points.is_empty() {
        return false;
    }

    if !crossing {
        return points.iter().all(|point| window.contains(*point));
    }

    if points.iter().any(|point| window.contains(*point)) {
        return true;
    }

    let closed = matches!(feature.geometry, Geometry::Polygon(_));
    let closing = closed.then(|| (points[points.len() - 1], points[0]));

    let crosses_edge = points
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .chain(closing)
        .any(|(start, end)| segment_crosses_rectangle(start, end, window));

    // Pencere alanın tamamen içinde kalıyorsa kenarlar kesişmez.
    crosses_edge || (closed && point_in_polygon(window.center(), &points))
}

/// Doğru parçası dikdörtgenin kenarlarından birini kesiyor mu.
fn segment_crosses_rectangle(start: Point, end: Point, rectangle: Rectangle) -> bool {
    let top_left = Point::new(rectangle.x, rectangle.y);
    let top_right = Point::new(rectangle.x + rectangle.width, rectangle.y);
    let bottom_left = Point::new(rectangle.x, rectangle.y + rectangle.height);
    let bottom_right = Point::new(
        rectangle.x + rectangle.width,
        rectangle.y + rectangle.height,
    );

    [
        (top_left, top_right),
        (top_right, bottom_right),
        (bottom_right, bottom_left),
        (bottom_left, top_left),
    ]
    .into_iter()
    .any(|(a, b)| segments_intersect(start, end, a, b))
}

fn segments_intersect(p1: Point, p2: Point, q1: Point, q2: Point) -> bool {
    let cross =
        |a: Point, b: Point, c: Point| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);

    let d1 = cross(q1, q2, p1);
    let d2 = cross(q1, q2, p2);
    let d3 = cross(p1, p2, q1);
    let d4 = cross(p1, p2, q2);

    ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0))
}

/// Yakalanan noktanın türü; AutoCAD nesne yakalama işaretlerini izler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    /// Nokta öğe.
    Node,
    /// Çizginin başı ya da sonu.
    Endpoint,
    /// Çizgi ara köşesi veya alan köşesi.
    Vertex,
}

impl SnapKind {
    pub fn label(self) -> &'static str {
        match self {
            SnapKind::Node => "Düğüm",
            SnapKind::Endpoint => "Uç nokta",
            SnapKind::Vertex => "Köşe",
        }
    }
}

/// Yakalanan nokta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snap {
    pub location: LonLat,
    pub kind: SnapKind,
}

/// İmlece `tolerance` piksel içindeki en yakın köşe (nokta öğe, çizgi ucu,
/// çizgi veya alan köşesi).
pub fn snap(layers: &[Layer], viewport: &Viewport, point: Point, tolerance: f32) -> Option<Snap> {
    let mut best: Option<(f32, Snap)> = None;
    let mut consider = |location: LonLat, kind: SnapKind| {
        let distance = viewport.project(location).distance(point);

        if distance <= tolerance && best.is_none_or(|(closest, _)| distance < closest) {
            best = Some((distance, Snap { location, kind }));
        }
    };

    for layer in layers.iter().filter(|layer| layer.is_interactive()) {
        for feature in layer.features.iter().filter(|feature| layer.shows(feature)) {
            match &feature.geometry {
                Geometry::Point(location) => consider(*location, SnapKind::Node),
                Geometry::Line(points) => {
                    let last = points.len().saturating_sub(1);

                    for (index, location) in points.iter().enumerate() {
                        let kind = if index == 0 || index == last {
                            SnapKind::Endpoint
                        } else {
                            SnapKind::Vertex
                        };

                        consider(*location, kind);
                    }
                }
                Geometry::Polygon(points) => {
                    for location in points {
                        consider(*location, SnapKind::Vertex);
                    }
                }
            }
        }
    }

    best.map(|(_, snap)| snap)
}

fn is_hit(geometry: &Geometry, viewport: &Viewport, point: Point) -> bool {
    match geometry {
        Geometry::Point(location) => viewport.project(*location).distance(point) <= POINT_TOLERANCE,
        Geometry::Line(points) => points.windows(2).any(|pair| {
            distance_to_segment(point, viewport.project(pair[0]), viewport.project(pair[1]))
                <= LINE_TOLERANCE
        }),
        Geometry::Polygon(points) => {
            let screen: Vec<Point> = points
                .iter()
                .map(|location| viewport.project(*location))
                .collect();

            point_in_polygon(point, &screen)
        }
    }
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f32 {
    let segment = end - start;
    let length_squared = segment.x * segment.x + segment.y * segment.y;

    if length_squared <= f32::EPSILON {
        return point.distance(start);
    }

    let to_point = point - start;
    let projection =
        ((to_point.x * segment.x + to_point.y * segment.y) / length_squared).clamp(0.0, 1.0);

    point.distance(start + segment * projection)
}

fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut previous = polygon.len() - 1;

    for current in 0..polygon.len() {
        let a = polygon[current];
        let b = polygon[previous];

        let crosses = (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y + f32::EPSILON) + a.x;

        if crosses {
            inside = !inside;
        }

        previous = current;
    }

    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::ObjectId;
    use iced::{Color, Size};

    fn viewport() -> Viewport {
        Viewport::new(LonLat::new(29.0, 41.0), 10.0, Size::new(800.0, 600.0))
    }

    fn layer(geometry: Geometry) -> Layer {
        Layer::lines("Test", Color::BLACK).with_features([Feature::new(geometry)])
    }

    /// Katmandaki ilk (tek) öğenin adresi.
    const FIRST: FeatureRef = FeatureRef::new(0, ObjectId(1));

    #[test]
    fn point_hit_is_within_tolerance() {
        let viewport = viewport();
        let location = LonLat::new(28.98, 41.01);
        let layers = [layer(Geometry::Point(location))];

        assert_eq!(
            hit_test(&layers, &viewport, viewport.project(location)),
            Some(FIRST)
        );

        let somewhere_else = viewport.project(LonLat::new(33.0, 39.0));
        assert_eq!(hit_test(&layers, &viewport, somewhere_else), None);
    }

    #[test]
    fn polygon_hit_detects_inside_and_outside() {
        let viewport = viewport();
        let layers = [layer(Geometry::Polygon(vec![
            LonLat::new(28.9, 40.9),
            LonLat::new(29.1, 40.9),
            LonLat::new(29.1, 41.1),
            LonLat::new(28.9, 41.1),
        ]))];

        let inside = viewport.project(LonLat::new(29.0, 41.0));
        let outside = viewport.project(LonLat::new(29.5, 41.0));

        assert_eq!(hit_test(&layers, &viewport, inside), Some(FIRST));
        assert_eq!(hit_test(&layers, &viewport, outside), None);
    }

    #[test]
    fn line_hit_uses_distance_to_segments() {
        let viewport = viewport();
        let layers = [layer(Geometry::Line(vec![
            LonLat::new(28.9, 41.0),
            LonLat::new(29.1, 41.0),
        ]))];

        let on_line = viewport.project(LonLat::new(29.0, 41.0));
        let off_line = viewport.project(LonLat::new(29.0, 40.7));

        assert_eq!(hit_test(&layers, &viewport, on_line), Some(FIRST));
        assert_eq!(hit_test(&layers, &viewport, off_line), None);
    }

    #[test]
    fn invisible_layers_are_ignored() {
        let viewport = viewport();
        let location = LonLat::new(28.98, 41.01);

        let mut hidden = layer(Geometry::Point(location));
        hidden.visible = false;

        let point = viewport.project(location);

        assert_eq!(hit_test(&[hidden.clone()], &viewport, point), None);
        assert_eq!(snap(&[hidden], &viewport, point, SNAP_TOLERANCE), None);
    }

    #[test]
    fn snap_reports_endpoints_and_vertices() {
        let viewport = viewport();
        let start = LonLat::new(28.9, 41.0);
        let middle = LonLat::new(29.0, 41.05);
        let end = LonLat::new(29.1, 41.0);
        let layers = [layer(Geometry::Line(vec![start, middle, end]))];

        let near = |location: LonLat| viewport.project(location) + iced::Vector::new(3.0, -2.0);

        let snapped_start = snap(&layers, &viewport, near(start), SNAP_TOLERANCE).expect("uç");
        let snapped_middle = snap(&layers, &viewport, near(middle), SNAP_TOLERANCE).expect("köşe");

        assert_eq!(snapped_start.kind, SnapKind::Endpoint);
        assert_eq!(snapped_start.location, start);
        assert_eq!(snapped_middle.kind, SnapKind::Vertex);

        let far = viewport.project(start) + iced::Vector::new(40.0, 40.0);
        assert_eq!(snap(&layers, &viewport, far, SNAP_TOLERANCE), None);
    }

    #[test]
    fn window_selects_whole_features_and_crossing_touching_ones() {
        let viewport = viewport();
        let line = layer(Geometry::Line(vec![
            LonLat::new(28.95, 41.0),
            LonLat::new(29.05, 41.0),
        ]));
        let layers = [line];

        let around = Bounds {
            south_west: LonLat::new(28.9, 40.95),
            north_east: LonLat::new(29.1, 41.05),
        };
        let half = Bounds {
            south_west: LonLat::new(29.0, 40.95),
            north_east: LonLat::new(29.1, 41.05),
        };
        let beside = Bounds {
            south_west: LonLat::new(29.2, 40.95),
            north_east: LonLat::new(29.3, 41.05),
        };

        assert_eq!(in_bounds(&layers, &viewport, around, false), [FIRST]);
        assert!(in_bounds(&layers, &viewport, half, false).is_empty());
        assert_eq!(in_bounds(&layers, &viewport, half, true), [FIRST]);
        assert!(in_bounds(&layers, &viewport, beside, true).is_empty());
    }

    #[test]
    fn crossing_window_inside_a_polygon_selects_it() {
        let viewport = viewport();
        let layers = [layer(Geometry::Polygon(vec![
            LonLat::new(28.8, 40.8),
            LonLat::new(29.2, 40.8),
            LonLat::new(29.2, 41.2),
            LonLat::new(28.8, 41.2),
        ]))];

        let inside = Bounds {
            south_west: LonLat::new(28.99, 40.99),
            north_east: LonLat::new(29.01, 41.01),
        };

        assert_eq!(in_bounds(&layers, &viewport, inside, true), [FIRST]);
        assert!(in_bounds(&layers, &viewport, inside, false).is_empty());
    }

    #[test]
    fn hit_test_can_be_limited_to_a_layer() {
        let viewport = viewport();
        let location = LonLat::new(28.98, 41.01);
        let layers = [
            layer(Geometry::Point(location)),
            layer(Geometry::Point(location)),
        ];
        let point = viewport.project(location);

        assert_eq!(hit_test(&layers, &viewport, point), Some(FIRST));
        assert_eq!(
            hit_test_in(&layers, 1, &viewport, point),
            Some(FeatureRef::new(1, ObjectId(1)))
        );
    }

    #[test]
    fn hidden_sublayers_cannot_be_picked() {
        use crate::attribute::{Field, Value};
        use crate::spatial::Sublayer;

        let viewport = viewport();
        let location = LonLat::new(28.98, 41.01);
        let mut layers = [Layer::points("Test", Color::BLACK)
            .with_schema([Field::text("Tür")])
            .with_features(
                [Feature::new(Geometry::Point(location)).with_values([Value::from("A")])],
            )
            .with_sublayers("Tür", [Sublayer::new("A", Color::WHITE)])];
        let point = viewport.project(location);
        let everything = viewport.visible_bounds();

        assert_eq!(hit_test(&layers, &viewport, point), Some(FIRST));

        layers[0].sublayers[0].visible = false;

        assert_eq!(hit_test(&layers, &viewport, point), None);
        assert!(in_bounds(&layers, &viewport, everything, true).is_empty());
        assert!(snap(&layers, &viewport, point, SNAP_TOLERANCE).is_none());
    }
}
