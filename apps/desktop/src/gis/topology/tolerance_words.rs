//! The rules window's lengths said with what each is compared to (docs/adr/9146): the tolerance as the rules compare it
//! (a line end meets other line work at that distance or nearer; an overlap counts when its width, 2 × area ÷
//! perimeter, is more), and a quality profile's storage precision and acceptance threshold as the repair recipe takes
//! them (docs/adr/1260 §4). The values are the ones typed, in the project's unit.

/// The words under the window's lengths: `tolerance` and, with a profile, its `precision` and `acceptance`, as typed.
pub(crate) fn explained(tolerance: &str, profile: Option<(&str, &str)>, unit: &str) -> String {
    let typed = |s: &str| {
        let s = s.trim();
        if s.is_empty() { "…".to_owned() } else { format!("{s} {unit}") }
    };
    let t = typed(tolerance);
    let mut out = format!(
        "Tolerans {t}: bir çizgi ucu başka bir çizgiye {t} ya da daha yakınsa değer, daha uzaksa sarkan uçtur; iki alanın örtüşen parçası genişliği (2 × alan ÷ çevre) toleransı aşarsa bulgudur."
    );
    match profile {
        Some((precision, acceptance)) => out.push_str(&format!(
            " Depolama hassasiyeti {}: koordinatlar ona yuvarlanmaz; onarımın bundan az taşıdığı köşe de taşınmış sayılır. Kabul eşiği {}: bir onarım adımı kaynak köşeyi bundan uzağa taşıyamaz. Tolerans hassasiyetin en az iki katıdır; her işaret gerekçe ister.",
            typed(precision),
            typed(acceptance)
        )),
        None => out.push_str(" Kalite profili yok: hassasiyet ve kabul eşiği uygulanmaz; işaretler gerekçesiz olabilir."),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::explained;

    #[test]
    fn each_length_is_said_with_its_unit_and_comparison() {
        let words = explained("0,001", Some(("0,0001", "0,05")), "m");
        assert!(words.starts_with("Tolerans 0,001 m: bir çizgi ucu başka bir çizgiye 0,001 m ya da daha yakınsa değer"), "{words}");
        assert!(words.contains("Depolama hassasiyeti 0,0001 m: koordinatlar ona yuvarlanmaz"), "{words}");
        assert!(words.contains("Kabul eşiği 0,05 m: bir onarım adımı kaynak köşeyi bundan uzağa taşıyamaz"), "{words}");
        let none = explained("", None, "m");
        assert!(none.starts_with("Tolerans …:"), "{none}");
        assert!(none.ends_with("işaretler gerekçesiz olabilir."), "{none}");
    }
}
