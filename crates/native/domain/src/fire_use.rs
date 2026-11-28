//! A building's use as BYKHY Ek-5/B's rows name it, read from a unit's nature or a room's name or use (docs/adr/8035,
//! 8057 §79): its escape distances, dead-end corridor, unit widths and default person load. One reading for Mimari
//! denetim's fire rules (`kentos_native_application::arch::fire_audit`) and the audit facts a rule of an extension
//! reads (`UnitFacts::occupancy`).

use crate::room_names::{has, is, words};

/// A building's use as Ek-5/B's rows name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    HighHazard,
    Industry,
    Dormitory,
    Shop,
    Office,
    Parking,
    School,
    Assembly,
    Hospital,
    Hotel,
    Dwelling,
}

impl Use {
    /// Its row's words in a finding.
    pub fn label(self) -> &'static str {
        match self {
            Use::HighHazard => "yüksek tehlikeli yer",
            Use::Industry => "endüstri yapısı",
            Use::Dormitory => "yurt",
            Use::Shop => "mağaza",
            Use::Office => "büro",
            Use::Parking => "otopark ve depo",
            Use::School => "okul",
            Use::Assembly => "toplanma amaçlı bina",
            Use::Hospital => "hastane",
            Use::Hotel => "otel",
            Use::Dwelling => "apartman",
        }
    }

    /// Ek-5/B: the farthest one-way and two-way walks, m, without a sprinkler system and with one.
    pub fn walks(self) -> [[f64; 2]; 2] {
        match self {
            Use::HighHazard => [[10.0, 20.0], [20.0, 35.0]],
            Use::Industry => [[15.0, 25.0], [30.0, 60.0]],
            Use::Dormitory | Use::Office | Use::School => [[15.0, 30.0], [45.0, 75.0]],
            Use::Shop | Use::Parking | Use::Assembly => [[15.0, 25.0], [45.0, 60.0]],
            Use::Hospital => [[15.0, 25.0], [30.0, 45.0]],
            Use::Hotel => [[15.0, 20.0], [30.0, 45.0]],
            Use::Dwelling => [[15.0, 30.0], [30.0, 75.0]],
        }
    }

    /// Ek-5/B: a dead-end corridor's farthest walk (çıkmaz koridor, 4/i), m, without a sprinkler system and with one.
    pub fn dead_end(self) -> [f64; 2] {
        match self {
            Use::HighHazard => [10.0, 20.0],
            _ => [15.0, 20.0],
        }
    }

    /// Ek-5/B: the people a 50 cm unit width passes: a door to the outside, other doors, escape stairs, ramps and
    /// corridors.
    pub fn per_unit(self) -> [f64; 4] {
        match self {
            Use::HighHazard | Use::Dormitory | Use::Hotel | Use::Dwelling => {
                [50.0, 40.0, 30.0, 50.0]
            }
            Use::Hospital => [30.0, 30.0, 30.0, 30.0],
            _ => [100.0, 80.0, 60.0, 100.0],
        }
    }

    /// The square metres a person of a room nothing names more plainly (Ek-5/A's row of the use's own rooms).
    pub fn default_load(self) -> f64 {
        match self {
            Use::Shop => 5.0,
            Use::Assembly => 1.5,
            Use::Hospital | Use::Hotel => 20.0,
            Use::Parking => 30.0,
            _ => 10.0,
        }
    }

    /// The stricter of two uses: the shorter two-way walk, then the shorter one-way walk (31/5).
    pub fn stricter(self, other: Use) -> Use {
        let key = |u: Use| (u.walks()[1][0], u.walks()[0][0], u.per_unit()[2]);
        if key(other) < key(self) { other } else { self }
    }
}

/// A use by its words (a unit's nature, a room's name or use); none for words that name no use. A high-hazard place
/// first: an LPG cylinder store is not a store's row.
pub fn use_of(name: Option<&str>, usage: Option<&str>) -> Option<Use> {
    let w = words(name, usage);
    let any = |list: &[&str]| list.iter().any(|p| has(&w, p));
    if high_hazard(&w) {
        Some(Use::HighHazard)
    } else if any(&["DAIRE", "MESKEN", "KONUT", "APARTMAN"]) || is(&w, "EV") {
        Some(Use::Dwelling)
    } else if any(&[
        "HASTANE",
        "HUZUREVI",
        "KLINIK",
        "POLIKLINIK",
        "BAKIMEVI",
        "SAGLIK",
    ]) {
        Some(Use::Hospital)
    } else if any(&["OTEL", "MOTEL", "PANSIYON"]) {
        Some(Use::Hotel)
    } else if any(&["YURT", "YATAKHANE"]) {
        Some(Use::Dormitory)
    } else if any(&["OKUL", "EGITIM", "DERSLIK", "SINIF", "KRES", "ANAOKUL"]) {
        Some(Use::School)
    } else if any(&["DUKKAN", "MAGAZA", "MARKET", "SATIS"]) {
        Some(Use::Shop)
    } else if any(&["OTOPARK", "GARAJ", "DEPO", "AMBAR"]) {
        Some(Use::Parking)
    } else if any(&["FABRIKA", "IMALAT", "ATOLYE", "URETIM", "SANAYI"]) {
        Some(Use::Industry)
    } else if any(&[
        "LOKANTA",
        "RESTORAN",
        "KAFE",
        "SINEMA",
        "TIYATRO",
        "KONFERANS",
        "DUGUN",
        "CAMI",
        "MESCIT",
        "KONSER",
    ]) {
        Some(Use::Assembly)
    } else if any(&["BURO", "OFIS", "IDARI", "DERNEK"]) {
        Some(Use::Office)
    } else {
        None
    }
}

/// The words that make a building one of BYKHY 50's (oteller, moteller ve yatakhaneler; the lodging buildings of 10).
const LODGING: [&str; 8] = [
    "OTEL",
    "MOTEL",
    "PANSIYON",
    "KONAKLAMA",
    "HOSTEL",
    "YURT",
    "YURD",
    "YATAKHANE",
];
/// Those of them that are a hotel's: a use or a name with none of them is a dormitory's.
const HOTEL: [&str; 5] = ["OTEL", "MOTEL", "PANSIYON", "KONAKLAMA", "HOSTEL"];
/// Words that make a room no bedroom: a corridor, a hall, a wet room (Süit banyosu, Kat holü).
const NO_BEDROOM: [&str; 8] = [
    "KORIDOR", "HOL", "WC", "TUVALET", "BANYO", "DUS", "LAVABO", "ANTRE",
];

/// A lodging building's row of Ek-5/B as the standards package's rules read BYKHY 50's building (docs/adr/8107,
/// 8156): from the program's buildings' uses, else from its rooms' names (none when all its units are dwellings). A
/// word of Otel, Motel, Pansiyon, Konaklama, Hostel, Yurt or Yatakhane makes it one: a hotel's row when one of those
/// uses or names has a hotel's word, a dormitory's otherwise. The fire rules leave such a building's bedrooms
/// ([`lodging_bedroom`]) to the package, which measures them from their doors (50/4).
pub fn lodging_of<'a>(
    uses: impl IntoIterator<Item = &'a str>,
    units: usize,
    dwellings: usize,
    rooms: impl IntoIterator<Item = &'a str>,
) -> Option<Use> {
    let uses: Vec<Vec<String>> = uses
        .into_iter()
        .filter(|u| !u.trim().is_empty())
        .map(|u| words(Some(u), None))
        .collect();
    let texts = if !uses.is_empty() {
        uses
    } else if dwellings > 0 && dwellings == units {
        return None;
    } else {
        rooms.into_iter().map(|r| words(Some(r), None)).collect()
    };
    let lodging: Vec<&Vec<String>> = texts
        .iter()
        .filter(|w| LODGING.iter().any(|p| has(w, p)))
        .collect();
    if lodging.is_empty() {
        return None;
    }
    Some(if lodging.iter().any(|w| HOTEL.iter().any(|p| has(w, p))) {
        Use::Hotel
    } else {
        Use::Dormitory
    })
}

/// A lodging building's bedroom by its name (its name, else its use; docs/adr/8111, 8156; BYKHY 50/3, 50/4) as the
/// standards package reads it: a word beginning Yatak, Süit, Konuk or Misafir, an Öğrenci odası, or a name beginning
/// Oda or Otel odası; never a corridor, a hall or a wet room.
pub fn lodging_bedroom(name: &str) -> bool {
    let w = words(Some(name), None);
    let Some(first) = w.first() else {
        return false;
    };
    if NO_BEDROOM.iter().any(|p| has(&w, p)) {
        return false;
    }
    ["YATAK", "SUIT", "KONUK", "MISAFIR"]
        .iter()
        .any(|p| has(&w, p))
        || (has(&w, "OGRENCI") && has(&w, "ODA"))
        || first == "ODA"
        || (first == "OTEL" && w.get(1).is_some_and(|x| x.starts_with("ODA")))
}

/// BYKHY 17 (yüksek tehlikeli yerler, 8/1-ğ's use): “Parlayıcı ve patlayıcı maddeler ile akaryakıtların imal edildiği,
/// depolandığı, doldurma-boşaltma ve satış işlerinin yapıldığı yerler”: LPG and natural gas stored, filled or sold
/// (a), explosives, bullets, gunpowder, dynamite (b), flammable liquids (c). Only words that name such a place plainly:
/// a building's own fuel tank room (“Yakıt deposu”, “Akaryakıt deposu”) is article 56's, the boiler's store, not this
/// use; a gas meter room stores nothing.
fn high_hazard(w: &[String]) -> bool {
    let any = |list: &[&str]| list.iter().any(|p| has(w, p));
    let trade = any(&["ISTASYON", "DOLUM", "DOLDURMA", "SATIS", "BAYI"]);
    let store = any(&["DEPO", "AMBAR"]);
    let natural_gas = has(w, "DOGALGAZ") || (is(w, "DOGAL") && has(w, "GAZ")) || is(w, "CNG");
    any(&["PATLAYICI", "PARLAYICI", "BARUT", "DINAMIT", "LPG", "TUPCU"])
        || (has(w, "AKARYAKIT") && (trade || has(w, "DEPOLAMA")))
        || (natural_gas && (trade || store))
        || (is(w, "TUP") && (trade || store))
        || (is(w, "MERMI") && (trade || store || has(w, "IMAL")))
        || (has(w, "YANICI") && has(w, "SIVI"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BYKHY 17's places by their names, before a store's or a shop's row; a building's fuel tank room, a gas meter
    /// room and the words alone are not.
    #[test]
    fn a_high_hazard_place_is_read_from_its_name() {
        for name in [
            "LPG tüp deposu",
            "Tüp bayii",
            "Tüpçü",
            "Akaryakıt istasyonu",
            "Akaryakıt depolama tesisi",
            "Doğalgaz dolum istasyonu",
            "Doğal gaz deposu",
            "CNG dolum",
            "Parlayıcı madde deposu",
            "Patlayıcı madde ambarı",
            "Barut deposu",
            "Mermi deposu",
            "Yanıcı sıvı deposu",
        ] {
            assert_eq!(use_of(Some(name), None), Some(Use::HighHazard), "{name}");
        }
        for (name, plain) in [
            ("Yakıt deposu", Some(Use::Parking)),
            ("Akaryakıt deposu", Some(Use::Parking)),
            ("Doğalgaz sayaç odası", None),
            ("Depo", Some(Use::Parking)),
            ("Market", Some(Use::Shop)),
            ("Tüp", None),
            ("Mermer atölyesi", Some(Use::Industry)),
        ] {
            assert_eq!(use_of(Some(name), None), plain, "{name}");
        }
        assert_eq!(Use::HighHazard.dead_end(), [10.0, 20.0]);
        assert_eq!(Use::Office.stricter(Use::HighHazard), Use::HighHazard);
    }

    /// BYKHY 50's building and its bedrooms as the standards package reads them (docs/adr/8156): the program's uses
    /// first, the rooms' names without them; a dormitory's row only when no word is a hotel's.
    #[test]
    fn a_lodging_building_and_its_bedrooms_are_read_from_their_words() {
        let none: [&str; 0] = [];
        assert_eq!(lodging_of(["Otel"], 0, 0, none), Some(Use::Hotel));
        assert_eq!(
            lodging_of(["Öğrenci yurdu"], 0, 0, none),
            Some(Use::Dormitory)
        );
        assert_eq!(lodging_of(["Otel ve yurt"], 0, 0, none), Some(Use::Hotel));
        assert_eq!(
            lodging_of(["Yurt", "Pansiyon"], 0, 0, none),
            Some(Use::Hotel)
        );
        assert_eq!(lodging_of(["Büro"], 0, 0, ["Otel odası 5"]), None);
        assert_eq!(lodging_of([" "], 0, 0, ["Otel odası 5"]), Some(Use::Hotel));
        assert_eq!(lodging_of(none, 2, 2, ["Yurt odası"]), None);
        assert_eq!(lodging_of(none, 0, 0, ["Yatakhane"]), Some(Use::Dormitory));
        for name in [
            "Oda 101",
            "Otel odası 12",
            "Süit 301",
            "Yatak odası",
            "Konuk odası",
            "Öğrenci odası (4)",
            "Yatakhane",
        ] {
            assert!(lodging_bedroom(name), "{name}");
        }
        for name in [
            "Süit 301 banyosu",
            "Kat holü",
            "Koridor",
            "Toplantı odası",
            "Lobi",
            "Etüt odası",
            "",
        ] {
            assert!(!lodging_bedroom(name), "{name}");
        }
    }
}
