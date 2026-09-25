//! Koordinat, mesafe ve sayıların Türkçe yazımı.
//!
//! Sayılar binlik ayraç olarak nokta, ondalık ayırıcı olarak virgül kullanır
//! (15.840.900, 346,71 km). Ondalık koordinatlar ise alışılmış biçimde
//! noktayla yazılır ve virgülle ayrılır (41.00820, 28.97840).

use super::LonLat;
use crate::attribute::number;

/// Sayıyı yuvarlayıp binlik ayraçlı yazar (15.840.900).
pub fn integer(value: f64) -> String {
    number::integer(value.round() as i64)
}

/// Mesafeyi okunaklı yazar: 1 km altında metre, üstünde iki ondalıklı km.
pub fn distance(meters: f64) -> String {
    if meters >= 1_000.0 {
        format!("{} km", number::real(meters / 1_000.0, 2))
    } else {
        format!("{} m", number::integer(meters.round() as i64))
    }
}

/// Dereceyi derece-dakika-saniye biçiminde yazar (41°00'29.5"K).
pub fn dms(value: f64, positive: &str, negative: &str) -> String {
    let hemisphere = if value >= 0.0 { positive } else { negative };
    let absolute = value.abs();

    let degrees = absolute.floor();
    let minutes_total = (absolute - degrees) * 60.0;
    let minutes = minutes_total.floor();
    let seconds = (minutes_total - minutes) * 60.0;

    format!("{degrees:.0}\u{b0}{minutes:02.0}'{seconds:04.1}\"{hemisphere}")
}

/// Koordinatı enlem-boylam sırasıyla DMS biçiminde yazar
/// (41°00'29.5"K 28°58'42.2"D).
pub fn coordinates(location: LonLat) -> String {
    format!(
        "{} {}",
        dms(location.lat, "K", "G"),
        dms(location.lon, "D", "B")
    )
}

/// Koordinatı enlem-boylam sırasıyla ondalık derece olarak yazar.
pub fn decimal(location: LonLat) -> String {
    format!("{:.5}, {:.5}", location.lat, location.lon)
}

/// Ondalık sayıyı gereksiz sıfırlar olmadan yazar (5, 2,5).
pub fn pretty(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        number::integer(value.round() as i64)
    } else {
        number::real(value, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_are_grouped_by_thousands() {
        assert_eq!(integer(15_840_900.0), "15.840.900");
        assert_eq!(integer(999.0), "999");
        assert_eq!(integer(1_000.0), "1.000");
        assert_eq!(integer(-1_234_567.0), "-1.234.567");
    }

    #[test]
    fn distances_switch_to_kilometers() {
        assert_eq!(distance(950.0), "950 m");
        assert_eq!(distance(1_500.0), "1,50 km");
        assert_eq!(distance(346_710.0), "346,71 km");
        assert_eq!(pretty(2.5), "2,5");
        assert_eq!(pretty(-30.0), "-30");
    }

    #[test]
    fn dms_uses_turkish_hemispheres() {
        assert_eq!(dms(41.0082, "K", "G"), "41\u{b0}00'29.5\"K");
        assert_eq!(dms(-12.5, "D", "B"), "12\u{b0}30'00.0\"B");
    }
}
