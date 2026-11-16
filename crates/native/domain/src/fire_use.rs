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

/// A use by its words (a unit's nature, a room's name or use); none for words that name no use.
pub fn use_of(name: Option<&str>, usage: Option<&str>) -> Option<Use> {
    let w = words(name, usage);
    let any = |list: &[&str]| list.iter().any(|p| has(&w, p));
    if any(&["DAIRE", "MESKEN", "KONUT", "APARTMAN"]) || is(&w, "EV") {
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
