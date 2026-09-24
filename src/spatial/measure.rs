//! Jeodezik mesafeler ve ölçüm.

use super::LonLat;

/// Ortalama Dünya yarıçapı (metre).
const EARTH_RADIUS: f64 = 6_371_008.8;

/// İki koordinat arasındaki büyük daire mesafesi (metre).
pub fn haversine_meters(a: LonLat, b: LonLat) -> f64 {
    let d_lat = (b.lat - a.lat).to_radians();
    let d_lon = (b.lon - a.lon).to_radians();
    let lat_a = a.lat.to_radians();
    let lat_b = b.lat.to_radians();

    let h = (d_lat / 2.0).sin().powi(2) + lat_a.cos() * lat_b.cos() * (d_lon / 2.0).sin().powi(2);

    2.0 * EARTH_RADIUS * h.sqrt().asin()
}

/// Bir çizgi boyunca toplam mesafe (metre).
pub fn polyline_length_meters(points: &[LonLat]) -> f64 {
    points
        .windows(2)
        .map(|pair| haversine_meters(pair[0], pair[1]))
        .sum()
}

/// Kapalı bir halkanın çevresi (metre).
pub fn ring_perimeter_meters(points: &[LonLat]) -> f64 {
    let closing = match (points.first(), points.last()) {
        (Some(first), Some(last)) if points.len() > 2 => haversine_meters(*last, *first),
        _ => 0.0,
    };

    polyline_length_meters(points) + closing
}

/// Ölç aracının topladığı noktalar.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Measurement {
    points: Vec<LonLat>,
}

impl Measurement {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, point: LonLat) {
        self.points.push(point);
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn points(&self) -> &[LonLat] {
        &self.points
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Kenar sayısı.
    pub fn segment_count(&self) -> usize {
        self.points.len().saturating_sub(1)
    }

    /// Kenarların uzunlukları (metre), sırasıyla.
    pub fn segments(&self) -> impl Iterator<Item = f64> + '_ {
        self.points
            .windows(2)
            .map(|pair| haversine_meters(pair[0], pair[1]))
    }

    /// Toplam uzunluk (metre).
    pub fn total_meters(&self) -> f64 {
        polyline_length_meters(&self.points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn istanbul_ankara_distance() {
        let istanbul = LonLat::new(28.9784, 41.0082);
        let ankara = LonLat::new(32.8597, 39.9334);

        let distance = haversine_meters(istanbul, ankara) / 1_000.0;

        assert!((distance - 350.0).abs() < 15.0, "{distance} km");
    }

    #[test]
    fn measurement_sums_segments() {
        let mut measurement = Measurement::new();
        measurement.push(LonLat::new(29.0, 41.0));
        measurement.push(LonLat::new(30.0, 41.0));
        measurement.push(LonLat::new(30.0, 40.0));

        let segments: f64 = measurement.segments().sum();

        assert_eq!(measurement.segment_count(), 2);
        assert!((segments - measurement.total_meters()).abs() < 1e-6);
    }

    #[test]
    fn ring_perimeter_closes_the_ring() {
        let ring = [
            LonLat::new(29.0, 41.0),
            LonLat::new(29.1, 41.0),
            LonLat::new(29.1, 41.1),
        ];

        let open = polyline_length_meters(&ring);
        let closed = ring_perimeter_meters(&ring);

        assert!(closed > open);
        assert!((closed - open - haversine_meters(ring[2], ring[0])).abs() < 1e-6);
    }
}
