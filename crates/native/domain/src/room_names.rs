//! Mahal adları (docs/adr/3033, 3036): a room's kind by its name or use, as the regulations read rooms — the wet rooms,
//! the kitchen, the passages, the living room and the bedrooms; a unit's nature a dwelling's or not; a stair's room.
//! One reading for Mimari denetim's Mevzuat group, the escape routes and the parking and shelter tables.

/// A room's kind as the regulation names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomKind {
    Living,
    Bedroom,
    Kitchen,
    Bath,
    Toilet,
    Corridor,
}

impl RoomKind {
    /// Its name in a finding.
    pub fn label(self) -> &'static str {
        match self {
            RoomKind::Living => "Oturma odası",
            RoomKind::Bedroom => "Yatak odası",
            RoomKind::Kitchen => "Mutfak",
            RoomKind::Bath => "Banyo",
            RoomKind::Toilet => "Tuvalet",
            RoomKind::Corridor => "Hol",
        }
    }
}

/// Upper-case ASCII, Turkish letters folded (İ, ı → I; Ş → S …).
pub fn fold(t: &str) -> String {
    t.chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'C',
            'ğ' | 'Ğ' => 'G',
            'ı' | 'I' | 'İ' | 'i' => 'I',
            'ö' | 'Ö' => 'O',
            'ş' | 'Ş' => 'S',
            'ü' | 'Ü' | 'û' | 'Û' => 'U',
            'â' | 'Â' => 'A',
            'î' | 'Î' => 'I',
            c => c.to_ascii_uppercase(),
        })
        .collect()
}

/// A name's and a use's words, folded.
pub fn words(name: Option<&str>, usage: Option<&str>) -> Vec<String> {
    [name, usage]
        .into_iter()
        .flatten()
        .flat_map(|t| {
            fold(t)
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| !w.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether a word begins with `w`.
pub fn has(words: &[String], w: &str) -> bool {
    words.iter().any(|x| x.starts_with(w))
}

/// Whether a word is `w`.
pub fn is(words: &[String], w: &str) -> bool {
    words.iter().any(|x| x == w)
}

/// A room's kind by its name or use: the wet rooms first (an Ebeveyn banyosu is a bathroom), then the kitchen, the
/// passages, the living room and the bedrooms; none for another (a store, a study, a balcony …).
pub fn kind_of(name: Option<&str>, usage: Option<&str>) -> Option<RoomKind> {
    let texts: Vec<String> = [name, usage].into_iter().flatten().map(fold).collect();
    let w = words(name, usage);
    let has = |p: &str| has(&w, p);
    if has("WC") || has("TUVALET") {
        Some(RoomKind::Toilet)
    } else if has("BANYO") || has("DUS") || has("YIKANMA") {
        Some(RoomKind::Bath)
    } else if has("MUTFAK") {
        Some(RoomKind::Kitchen)
    } else if has("HOL") || has("KORIDOR") || has("ANTRE") {
        Some(RoomKind::Corridor)
    } else if has("SALON") || has("OTURMA") {
        Some(RoomKind::Living)
    } else if has("YATAK")
        || has("EBEVEYN")
        || has("COCUK")
        || texts.iter().any(|t| t == "ODA" || t.starts_with("ODA "))
    {
        Some(RoomKind::Bedroom)
    } else {
        None
    }
}

/// The words a room people pass through is called by (geçiş mahalleri, docs/adr/3028): a door between one and a room is
/// the room's.
const PASSAGES: [&str; 7] = [
    "KORIDOR", "HOL", "ANTRE", "GIRIS", "SAHANLIK", "MERDIVEN", "GECIT",
];

/// Whether a room's name or use is one people pass through (Koridor, Hol, Antre, Giriş, Sahanlık, Merdiven, Geçit): one
/// of its words is one of them.
pub fn passage(name: Option<&str>, usage: Option<&str>) -> bool {
    words(name, usage)
        .iter()
        .any(|w| PASSAGES.contains(&w.as_str()))
}

/// Whether a unit's nature is a dwelling's (Daire, Mesken, Konut, Ev) or not given (docs/adr/3033).
pub fn dwelling_nature(nature: Option<&str>) -> bool {
    nature.is_none_or(|n| {
        let n = fold(n);
        ["DAIRE", "MESKEN", "KONUT", "EV"]
            .iter()
            .any(|w| n.contains(w))
    })
}

/// Whether a room's name or use is a stair's (Merdiven, Merdiven evi, Yangın merdiveni …); the room under a stair
/// (Merdiven altı, Merdiven altı deposu) is not.
pub fn stair_name(name: Option<&str>, usage: Option<&str>) -> bool {
    [name, usage].into_iter().flatten().any(|t| {
        let t = fold(t);
        let words: Vec<&str> = t
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        words.iter().any(|w| w.starts_with("MERDIVEN"))
            && !words
                .iter()
                .any(|w| w.starts_with("ALTI") || w.starts_with("DEPO"))
    })
}

/// Which of PAİY 28's inside heights a room's name or use asks (docs/adr/3040 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeightClass {
    /// An inhabited room (28/4): 2.60 m.
    Inhabited,
    /// A wet room, a pantry or store, the room under a stair, a passage (28/5): 2.20 m.
    Service,
    /// A garage, a car park, a boiler room, a wood or coal store, a technical room (28/6): 2.20 m, the
    /// administration's.
    Technical,
}

/// A room's 28 class by its name or use; none for one 28 does not read: an outdoor one (balkon, teras, bahçe), a
/// shelter (Sığınak Yönetmeliği's own 2.40), a shaft, an attic (çatı arası), or one with no name and no use.
pub fn height_class(name: Option<&str>, usage: Option<&str>) -> Option<HeightClass> {
    let w = words(name, usage);
    if w.is_empty() {
        return None;
    }
    let has = |p: &str| has(&w, p);
    if [
        "BALKON", "TERAS", "BAHCE", "SIGINAK", "ASANSOR", "ISIKLIK", "AYDINLIK", "BACA", "SAFT",
    ]
    .iter()
    .any(|p| has(p))
        || (has("CATI") && has("ARASI"))
    {
        return None;
    }
    if [
        "GARAJ",
        "OTOPARK",
        "KALORIFER",
        "KAZAN",
        "ODUNLUK",
        "KOMURLUK",
        "TEKNIK",
        "ELEKTRIK",
        "JENERATOR",
        "HIDROFOR",
        "TRAFO",
        "MAKINE",
        "POMPA",
        "IKLIMLENDIRME",
    ]
    .iter()
    .any(|p| has(p))
        || is(&w, "ISI")
    {
        return Some(HeightClass::Technical);
    }
    if matches!(
        kind_of(name, usage),
        Some(RoomKind::Bath | RoomKind::Toilet)
    ) || ["LAVABO", "CAMASIR", "KILER", "DEPO", "ARDIYE"]
        .iter()
        .any(|p| has(p))
        || (has("MERDIVEN") && has("ALTI"))
        || passage(name, usage)
    {
        return Some(HeightClass::Service);
    }
    Some(HeightClass::Inhabited)
}
