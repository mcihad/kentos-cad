//! Çizim taslağı: çizim araçlarının nokta girişlerini geometriye çeviren
//! durum makinesi.
//!
//! | Araç          | Tamamlanma                                            |
//! |---------------|-------------------------------------------------------|
//! | Nokta         | Her tıklama bir nokta                                 |
//! | Çizgi         | Her iki nokta bir parça; sonraki parça buradan başlar |
//! | Dikdörtgen    | İki köşe                                              |
//! | Daire         | Merkez ve yarıçap noktası                             |
//! | Çoklu çizgi   | [`Draft::finish`] ile, en az iki köşe                 |
//! | Alan          | [`Draft::finish`] ile, en az üç köşe                  |

use iced::Vector;

use super::{Geometry, LonLat, Tool, Viewport};

/// Daire çiziminde çemberi oluşturan köşe sayısı.
pub const CIRCLE_SEGMENTS: usize = 72;

/// Tamamlanmamış geometrinin noktaları.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Draft {
    points: Vec<LonLat>,
}

impl Draft {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn points(&self) -> &[LonLat] {
        &self.points
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    /// Son noktayı kaldırır; kaldıracak nokta yoksa `false`.
    pub fn undo(&mut self) -> bool {
        self.points.pop().is_some()
    }

    /// Aracın nokta girişini ekler. Aracın geometrisi tamamlandıysa onu
    /// döndürür.
    ///
    /// Daire, ekran uzayında örneklenir; böylece model alanındaki önizlemeyle
    /// birebir aynı olur. Bu yüzden o anki görünüm gerekir.
    pub fn push(&mut self, tool: Tool, point: LonLat, viewport: &Viewport) -> Option<Geometry> {
        self.points.push(point);

        match tool {
            Tool::Point => {
                self.points.clear();
                Some(Geometry::Point(point))
            }
            Tool::Line if self.points.len() == 2 => {
                // Zincirleme: sonraki parça bu parçanın bitişinden başlar.
                let segment = std::mem::replace(&mut self.points, vec![point]);
                Some(Geometry::Line(segment))
            }
            Tool::Rectangle if self.points.len() == 2 => {
                let (a, b) = (self.points[0], self.points[1]);
                self.points.clear();

                Some(Geometry::Polygon(vec![
                    a,
                    LonLat::new(b.lon, a.lat),
                    b,
                    LonLat::new(a.lon, b.lat),
                ]))
            }
            Tool::Circle if self.points.len() == 2 => {
                let center = viewport.project(self.points[0]);
                let radius = center.distance(viewport.project(self.points[1]));
                self.points.clear();

                let ring = (0..CIRCLE_SEGMENTS)
                    .map(|index| {
                        let angle = index as f32 / CIRCLE_SEGMENTS as f32 * std::f32::consts::TAU;
                        viewport.unproject(center + Vector::new(angle.cos(), angle.sin()) * radius)
                    })
                    .collect();

                Some(Geometry::Polygon(ring))
            }
            _ => None,
        }
    }

    /// Açık uçlu araçlarda (çoklu çizgi, alan) taslağı tamamlar; diğer
    /// araçlarda yarım kalan taslağı atar. Taslak her durumda boşalır.
    pub fn finish(&mut self, tool: Tool) -> Option<Geometry> {
        let points = std::mem::take(&mut self.points);

        match tool {
            Tool::Polyline if points.len() >= 2 => Some(Geometry::Line(points)),
            Tool::Polygon if points.len() >= 3 => Some(Geometry::Polygon(points)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;

    fn viewport() -> Viewport {
        Viewport::new(LonLat::new(32.0, 39.0), 6.0, Size::new(900.0, 600.0))
    }

    const A: LonLat = LonLat::new(30.0, 40.0);
    const B: LonLat = LonLat::new(32.0, 38.0);
    const C: LonLat = LonLat::new(34.0, 39.0);

    #[test]
    fn point_completes_immediately() {
        let mut draft = Draft::new();

        assert_eq!(
            draft.push(Tool::Point, A, &viewport()),
            Some(Geometry::Point(A))
        );
        assert!(draft.is_empty());
    }

    #[test]
    fn lines_chain_from_the_previous_end() {
        let viewport = viewport();
        let mut draft = Draft::new();

        assert_eq!(draft.push(Tool::Line, A, &viewport), None);
        assert_eq!(
            draft.push(Tool::Line, B, &viewport),
            Some(Geometry::Line(vec![A, B]))
        );
        assert_eq!(draft.points(), &[B]);
        assert_eq!(
            draft.push(Tool::Line, C, &viewport),
            Some(Geometry::Line(vec![B, C]))
        );
    }

    #[test]
    fn rectangle_uses_two_opposite_corners() {
        let viewport = viewport();
        let mut draft = Draft::new();

        draft.push(Tool::Rectangle, A, &viewport);
        let rectangle = draft.push(Tool::Rectangle, B, &viewport);

        assert_eq!(
            rectangle,
            Some(Geometry::Polygon(vec![
                A,
                LonLat::new(B.lon, A.lat),
                B,
                LonLat::new(A.lon, B.lat),
            ]))
        );
        assert!(draft.is_empty());
    }

    #[test]
    fn circle_is_sampled_around_the_center() {
        let viewport = viewport();
        let mut draft = Draft::new();

        draft.push(Tool::Circle, A, &viewport);
        let Some(Geometry::Polygon(ring)) = draft.push(Tool::Circle, B, &viewport) else {
            panic!("daire bir alan olmalı");
        };

        assert_eq!(ring.len(), CIRCLE_SEGMENTS);

        let center = viewport.project(A);
        let radius = center.distance(viewport.project(B));

        for vertex in ring {
            let distance = center.distance(viewport.project(vertex));
            assert!((distance - radius).abs() < 0.5, "{distance} ≠ {radius}");
        }
    }

    #[test]
    fn undo_removes_the_last_point() {
        let viewport = viewport();
        let mut draft = Draft::new();

        draft.push(Tool::Polyline, A, &viewport);
        draft.push(Tool::Polyline, B, &viewport);

        assert!(draft.undo());
        assert_eq!(draft.points(), &[A]);
        assert!(draft.undo());
        assert!(!draft.undo());
    }

    #[test]
    fn open_shapes_need_enough_vertices() {
        let viewport = viewport();
        let mut draft = Draft::new();

        draft.push(Tool::Polyline, A, &viewport);
        assert_eq!(draft.finish(Tool::Polyline), None);

        draft.push(Tool::Polygon, A, &viewport);
        draft.push(Tool::Polygon, B, &viewport);
        assert_eq!(draft.finish(Tool::Polygon), None);

        for point in [A, B, C] {
            draft.push(Tool::Polygon, point, &viewport);
        }

        assert_eq!(
            draft.finish(Tool::Polygon),
            Some(Geometry::Polygon(vec![A, B, C]))
        );
        assert!(draft.is_empty());
    }
}
