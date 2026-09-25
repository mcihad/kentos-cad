//! Eksen adımları: cetvellerin ve zaman çizelgesinin ortak hesabı.

/// Etiketli çizgilerin aralığı (birim) ve iki etiket arasındaki bölüm
/// sayısı. `scale` birim başına pikseldir; etiketler en az `label` piksel,
/// küçük çizgiler en az `tick` piksel aralıklıdır. Aralık 1, 2 ya da
/// 5 × 10ⁿ birimdir.
pub(crate) fn nice(scale: f64, label: f64, tick: f64) -> (f64, u32) {
    let raw = label / scale.max(f64::EPSILON);
    let base = 10f64.powf(raw.log10().floor());
    let (nice, parts): (f64, &[u32]) = match raw / base {
        mantissa if mantissa <= 1.0 => (1.0, &[10, 5, 2]),
        mantissa if mantissa <= 2.0 => (2.0, &[4, 2]),
        mantissa if mantissa <= 5.0 => (5.0, &[5]),
        _ => (10.0, &[10, 5, 2]),
    };
    let major = nice * base;
    let parts = parts
        .iter()
        .copied()
        .find(|parts| major * scale / f64::from(*parts) >= tick)
        .unwrap_or(1);

    (major, parts)
}

/// Etiketteki ondalık basamak sayısı: aralık 1'den küçükse gereken kadar.
pub(crate) fn decimals(major: f64) -> usize {
    if major >= 1.0 {
        0
    } else {
        (-major.log10() - 1e-3).ceil().max(0.0) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_one_two_or_five_times_a_power_of_ten() {
        // Birim başına 2 piksel: 56 piksel 28 birim eder, aralık 50.
        assert_eq!(nice(2.0, 56.0, 5.0), (50.0, 5));
        // Birim başına 6 piksel: 9,3 → 10, onda birer.
        assert_eq!(nice(6.0, 56.0, 5.0), (10.0, 10));
        // Birim başına 30 piksel: 1,87 → 2, dörtte birer.
        assert_eq!(nice(30.0, 56.0, 5.0), (2.0, 4));
        // Çok uzakta: 5.600 → 10.000; küçük çizgiler sığdığı kadar.
        let (major, parts) = nice(0.01, 56.0, 5.0);
        assert_eq!(major, 10_000.0);
        assert!(major * 0.01 / f64::from(parts) >= 5.0);
    }

    #[test]
    fn labels_show_as_many_decimals_as_the_step_needs() {
        assert_eq!(decimals(50.0), 0);
        assert_eq!(decimals(1.0), 0);
        assert_eq!(decimals(0.5), 1);
        assert_eq!(decimals(0.2), 1);
        assert_eq!(decimals(0.05), 2);
    }
}
