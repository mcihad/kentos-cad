//! Layer names of the 2026 Dijital Hazırlama Yönetmeliği (RG 05.08.2026/33331; docs/adr/8008).
//!
//! - EK-4 Tablo 4.1, architecture: disiplin (2 letters), öğe (4 letters), malzeme/tip/gösterim (at most 15 letters or
//!   digits), all three required.
//! - EK-2 Tablo 2.2 gives the disiplin (Mimari: MM), Tablo 2.3 the öğe codes.
//! - EK-4 4.11: upper case, no Ç Ğ İ Ö Ş Ü, the fields joined by hyphens.
//! - EK-4 4.14.3: what is cut in a plan or section is named by its material.
//! - EK-4 4.14.5: a hatch goes on its own layer, the name with "-T".

/// EK-2 Tablo 2.2: the architecture discipline.
pub const EK4_ARCHITECTURE: &str = "MM";

/// EK-2 Tablo 2.3's "Genel (Öğe kullanılmaması durumunda)": an annotation of no element (dimensions, labels).
pub const EK4_GENERAL: &str = "XXXX";

/// The third field when an object has no material, type or showing of its own to name.
pub const EK4_NO_DETAIL: &str = "GENEL";

/// EK-2 Tablo 2.3: the architecture discipline's elements and their codes, in the table's order.
pub const EK4_ELEMENTS: &[(&str, &str)] = &[
    ("Aks", "AKSS"),
    ("Aksesuar", "AKSR"),
    ("Asansör", "ASAN"),
    ("Asma Tavan", "ASTV"),
    ("Baca", "BACA"),
    ("Bitki", "BITK"),
    ("Bordür", "BORD"),
    ("Cam", "CAMM"),
    ("Cephe", "CEPH"),
    ("Çatı", "CATI"),
    ("Çatı Kaplama", "CKAP"),
    ("Çatı Kaplama Tahtası", "CKTA"),
    ("Çatı Taşıyıcı", "CTAS"),
    ("Çevre", "CVRE"),
    ("Çörten", "CORT"),
    ("Denizlik", "DENZ"),
    ("Doğrama", "DOGR"),
    ("Döşeme", "DOSM"),
    ("Duvar", "DUVA"),
    ("Duvar Kaplama", "DUVK"),
    ("Emsal", "EMSL"),
    ("Giydirme Cephe", "GIYC"),
    ("Güneş Kırıcı", "GNKR"),
    ("Harpuşta", "HARP"),
    ("Hatıl", "HATL"),
    ("Isı Yalıtımı", "ISYA"),
    ("Islak Hacim", "ISLH"),
    ("Kaldırım", "KALD"),
    ("Kapı", "KAPI"),
    ("Kartonpiyer", "KARP"),
    ("Kolon", "KOLN"),
    ("Korkuluk", "KORK"),
    ("Lento", "LENT"),
    ("Merdiven", "MERD"),
    ("Oluk", "OLUK"),
    ("Pencere", "PENC"),
    ("Rampa", "RAMP"),
    ("Şap", "SAPP"),
    ("Ses Yalıtımı", "SESY"),
    ("Sıva", "SIVA"),
    ("Su Yalıtımı", "SUYA"),
    ("Süpürgelik", "SUPR"),
    ("Taşıyıcı", "TASI"),
    ("Tavan", "TAVN"),
    ("Temel", "TEML"),
    ("Tefriş", "TEFR"),
    ("Tesviye Betonu", "TSVY"),
    ("Tretuvar", "TRET"),
    ("Yağmur İniş Borusu", "YIBO"),
    ("Yol", "YOLL"),
    ("Zemin Kaplama", "ZMNK"),
    ("Genel", EK4_GENERAL),
    ("Diğer", "DIGR"),
];

/// The longest third field (EK-4 Tablo 4.1).
pub const EK4_DETAIL_MAX: usize = 15;

/// A field of an EK-4 name from Turkish words: upper case, Ç Ğ İ Ö Ş Ü as C G I O S U (EK-4 4.11), letters and
/// digits only, at most `max` characters. "Tuğla" is TUGLA, "Alçı levha" ALCILEVHA.
pub fn ek4_word(text: &str, max: usize) -> String {
    text.chars()
        .filter_map(|c| {
            let plain = match c {
                'ç' | 'Ç' => 'C',
                'ğ' | 'Ğ' => 'G',
                'ı' | 'I' | 'i' | 'İ' => 'I',
                'ö' | 'Ö' => 'O',
                'ş' | 'Ş' => 'S',
                'ü' | 'Ü' => 'U',
                c if c.is_ascii_alphanumeric() => c.to_ascii_uppercase(),
                _ => return None,
            };
            Some(plain)
        })
        .take(max)
        .collect()
}

/// An architecture layer's EK-4 name: `MM-DUVA-TUGLA`. The detail is made a word ([`ek4_word`]); none left is
/// [`EK4_NO_DETAIL`].
pub fn ek4_layer(element: &str, detail: &str) -> String {
    let word = ek4_word(detail, EK4_DETAIL_MAX);
    let detail = if word.is_empty() {
        EK4_NO_DETAIL
    } else {
        &word
    };
    format!("{EK4_ARCHITECTURE}-{element}-{detail}")
}

/// Whether `name` is an EK-4 layer name: a two-letter disiplin, a four-letter öğe, then one to fifteen upper-case
/// letters or digits; a tadilat durumu's word after them ([`ek4_state_layer`]), then a hatch layer's "-T".
pub fn is_ek4_layer(name: &str) -> bool {
    let name = name.strip_suffix("-T").unwrap_or(name);
    let mut fields = name.split('-');
    let letters = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_uppercase());
    let (Some(discipline), Some(element), Some(detail), state, None) = (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) else {
        return false;
    };
    if state.is_some_and(|s| !EK4_STATES.contains(&s)) {
        return false;
    }
    letters(discipline, 2)
        && letters(element, 4)
        && (1..=EK4_DETAIL_MAX).contains(&detail.len())
        && detail
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// The layer a hatch of an object on `layer` goes on (EK-4 4.14.5): its name and "-T".
pub fn ek4_hatch_layer(layer: &str) -> String {
    format!("{layer}-T")
}

/// The tadilat durumu's words of a layer name (docs/adr/8039 §4): Mimarlar Odası's mevcut, iptal and ilave and geçici,
/// as EK-4 4.11 writes a field (upper case, no Turkish letters), in `ArchPhase::ALL`'s order.
pub const EK4_STATES: [&str; 4] = ["MEVCUT", "IPTAL", "ILAVE", "GECICI"];

/// The layer an object of tadilat durumu `phase` on `layer` is drawn on in a DXF (docs/adr/8039 §4): its name and the
/// state's word, as EK-4 4.14.3 adds a depth and 4.14.5 a "-T" after the third field (which holds the material):
/// MM-DUVA-TUGLA-IPTAL. A hatch of it is on that layer's "-T" (MM-DUVA-TUGLA-IPTAL-T).
pub fn ek4_state_layer(layer: &str, phase: crate::ArchPhase) -> String {
    let word = EK4_STATES[crate::ArchPhase::ALL
        .iter()
        .position(|p| *p == phase)
        .unwrap_or(0)];
    format!("{layer}-{word}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tadilat durumu's layer (docs/adr/8039 §4): the state's word after the material, its hatch's "-T" after that;
    /// still an EK-4 name. Another fourth field is not.
    #[test]
    fn a_state_layer_is_the_layer_and_its_state() {
        use crate::ArchPhase;
        assert_eq!(
            ek4_state_layer("MM-DUVA-TUGLA", ArchPhase::Demolish),
            "MM-DUVA-TUGLA-IPTAL"
        );
        assert_eq!(
            ek4_state_layer("MM-KAPI-GENEL", ArchPhase::New),
            "MM-KAPI-GENEL-ILAVE"
        );
        assert_eq!(
            ek4_state_layer("MM-DUVA-TUGLA", ArchPhase::Existing),
            "MM-DUVA-TUGLA-MEVCUT"
        );
        assert_eq!(
            ek4_state_layer("MM-DUVA-TUGLA", ArchPhase::Temporary),
            "MM-DUVA-TUGLA-GECICI"
        );
        assert!(is_ek4_layer("MM-DUVA-TUGLA-IPTAL"));
        assert!(is_ek4_layer("MM-DUVA-TUGLA-IPTAL-T"));
        assert!(is_ek4_layer("MM-DUVA-TUGLA-T"));
        assert!(!is_ek4_layer("MM-DUVA-TUGLA-YIKIM"));
        assert!(!is_ek4_layer("MM-DUVA-TUGLA-IPTAL-ILAVE"));
    }

    #[test]
    fn words_are_upper_case_without_turkish_letters() {
        assert_eq!(ek4_word("Tuğla", 15), "TUGLA");
        assert_eq!(ek4_word("Alçı levha", 15), "ALCILEVHA");
        assert_eq!(ek4_word("İç sıva", 15), "ICSIVA");
        assert_eq!(ek4_word("Çörtenşüğö", 15), "CORTENSUGO");
        assert_eq!(ek4_word("Betonarme perde 25 cm", 15), "BETONARMEPERDE2");
    }

    #[test]
    fn layer_names_have_three_fields_and_a_hatch_layer_its_t() {
        assert_eq!(ek4_layer("DUVA", "Tuğla"), "MM-DUVA-TUGLA");
        assert_eq!(ek4_layer("KAPI", ""), "MM-KAPI-GENEL");
        assert_eq!(ek4_hatch_layer("MM-DUVA-TUGLA"), "MM-DUVA-TUGLA-T");
        for name in [
            "MM-DUVA-TUGLA",
            "MM-XXXX-OLCU",
            "MM-DUVA-TUGLA-T",
            "MM-KOLN-BETONARME",
        ] {
            assert!(is_ek4_layer(name), "{name}");
        }
        for name in [
            "Çizim",
            "A-DUVAR",
            "MM-DUVA",
            "MM-DUVAR-TUGLA",
            "MM-DUVA-TUĞLA",
            "mm-duva-tugla",
            "MM-DUVA-X-Y",
        ] {
            assert!(!is_ek4_layer(name), "{name}");
        }
    }

    #[test]
    fn the_element_codes_are_four_capital_letters_each_once() {
        let mut seen = std::collections::BTreeSet::new();
        for (name, code) in EK4_ELEMENTS {
            assert!(
                code.len() == 4 && code.bytes().all(|b| b.is_ascii_uppercase()),
                "{name}: {code}"
            );
            assert!(seen.insert(*code), "{code} twice");
        }
        assert_eq!(EK4_ELEMENTS.len(), 53);
    }
}
