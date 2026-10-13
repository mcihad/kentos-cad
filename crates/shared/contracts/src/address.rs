//! Adres arama (docs/adr/0229 §2): a project's address data as the project
//! keeps it (`ProjectSettings::address_locators`): which point layer holds
//! the doors and which of its attributes are the street, the door number and
//! the rest; which line layer holds the streets; which area layers are the
//! neighbourhoods, districts and provinces; how good a match must be, and how
//! far Konumdan adres looks. A locator is an index the shared core builds
//! from the drawing each time it is asked (`kentos_geocode::locator`), never
//! a copy of the objects. Here are the kinds, the limits and the rules a
//! file, a command and the server check alike.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most address locators a project holds.
pub const ADDRESS_LOCATOR_LIMIT: usize = 16;

/// The least and the greatest score a locator may ask a match for; a new locator's.
pub const ADDRESS_MIN_SCORES: (u32, u32) = (50, 100);
pub const ADDRESS_MIN_SCORE: u32 = 85;

/// How far Konumdan adres looks by default, and the farthest it may (metres).
pub const ADDRESS_REACH: f64 = 100.0;
pub const ADDRESS_REACH_LIMIT: f64 = 10_000.0;

/// The longest id, name and field name (characters).
const ID_LENGTH: usize = 40;
const NAME_LENGTH: usize = 80;
const FIELD_LENGTH: usize = 64;

/// A project's address data (docs/adr/0229 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressLocator {
    /// Lower-case letters, digits and hyphens; one of its kind in the project.
    pub id: String,
    /// Trimmed; one of its kind in the project.
    pub name: String,
    /// Kapı noktaları: a point per door.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub doors: Option<DoorSource>,
    /// Yollar: the streets' centre lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub streets: Option<StreetSource>,
    /// Bölgeler: at most one area layer of each level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AreaSource>>", optional))]
    pub areas: Vec<AreaSource>,
    /// The least score a match takes, within [`ADDRESS_MIN_SCORES`].
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub min_score: u32,
    /// How far Konumdan adres looks, metres: more than 0, at most [`ADDRESS_REACH_LIMIT`].
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub reach: f64,
}

/// The doors' layer and which of its attributes say what.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DoorSource {
    /// The layer's id. A layer the project no longer has is said when the locator is built.
    pub layer: String,
    /// Yol: the street's name, with its kind as written (“Atatürk Bulvarı”).
    pub street: String,
    /// Kapı no: the outer door number (“12”, “12/A”).
    pub door: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub neighbourhood: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub district: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub province: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub postcode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub building: Option<String>,
    /// Adres kodu (UAVT): a query that is one finds its door.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub code: Option<String>,
}

/// The streets' layer and which of its attributes say what.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StreetSource {
    pub layer: String,
    /// Yol adı.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub neighbourhood: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub district: Option<String>,
}

/// An area layer: its level and which attribute names its areas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AreaSource {
    pub layer: String,
    pub level: AreaLevel,
    pub name: String,
    /// A neighbourhood's district; only at that level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub district: Option<String>,
}

/// What an area layer's areas are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AreaLevel {
    /// Mahalle (or köy).
    Neighbourhood,
    /// İlçe.
    District,
    /// İl.
    Province,
}

impl AreaLevel {
    /// The level's name in the rules' words.
    pub fn word(self) -> &'static str {
        match self {
            Self::Neighbourhood => "mahalle",
            Self::District => "ilçe",
            Self::Province => "il",
        }
    }
}

/// Whether `id` is a locator id: 1–40 of `a`–`z`, `0`–`9` and `-`.
pub fn address_locator_id_holds(id: &str) -> bool {
    (1..=ID_LENGTH).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Whether `r` is a reach a locator may name.
pub fn address_reach_holds(r: f64) -> bool {
    r.is_finite() && r > 0.0 && r <= ADDRESS_REACH_LIMIT
}

fn chars(s: &str) -> usize {
    s.chars().count()
}

/// What is wrong with a field's name (`what`: its place), if anything.
fn field_problem(name: &str, what: &str) -> Option<String> {
    if name.trim().is_empty() {
        return Some(format!("{what} alanı boş"));
    }
    if name.trim() != name || chars(name) > FIELD_LENGTH || name.chars().any(char::is_control) {
        return Some(format!(
            "{what} alanı kırpılmamış, {FIELD_LENGTH} karakterden uzun ya da denetim karakterli"
        ));
    }
    None
}

/// An optional field's problem.
fn optional_problem(name: &Option<String>, what: &str) -> Option<String> {
    name.as_deref().and_then(|n| field_problem(n, what))
}

impl AddressLocator {
    /// What is wrong with the locator, in the words a file, a command and the server say it; none when it holds.
    pub fn problem(&self) -> Option<String> {
        if !address_locator_id_holds(&self.id) {
            return Some(format!(
                "adres verisinin kimliği “{}”; 1–{ID_LENGTH} küçük harf, rakam ya da tire olmalı",
                self.id
            ));
        }
        let name = self.name.trim();
        if name.is_empty() || name != self.name || chars(name) > NAME_LENGTH {
            return Some(format!(
                "“{}” adres verisinin adı boş, kırpılmamış ya da {NAME_LENGTH} karakterden uzun",
                self.id
            ));
        }
        let at = |what: &str| format!("“{name}” adres verisinin {what}");
        if self.doors.is_none() && self.streets.is_none() && self.areas.is_empty() {
            return Some(at(
                "kaynağı yok; kapı noktaları, yollar ya da bölgelerden en az biri olmalı",
            ));
        }
        if let Some(d) = &self.doors {
            if d.layer.is_empty() {
                return Some(at("kapı katmanının kimliği boş"));
            }
            let problem = field_problem(&d.street, "kapılarının yol")
                .or_else(|| field_problem(&d.door, "kapılarının kapı no"))
                .or_else(|| optional_problem(&d.neighbourhood, "kapılarının mahalle"))
                .or_else(|| optional_problem(&d.district, "kapılarının ilçe"))
                .or_else(|| optional_problem(&d.province, "kapılarının il"))
                .or_else(|| optional_problem(&d.postcode, "kapılarının posta kodu"))
                .or_else(|| optional_problem(&d.building, "kapılarının bina adı"))
                .or_else(|| optional_problem(&d.code, "kapılarının adres kodu"));
            if let Some(p) = problem {
                return Some(at(&p));
            }
        }
        if let Some(s) = &self.streets {
            if s.layer.is_empty() {
                return Some(at("yol katmanının kimliği boş"));
            }
            let problem = field_problem(&s.name, "yollarının ad")
                .or_else(|| optional_problem(&s.neighbourhood, "yollarının mahalle"))
                .or_else(|| optional_problem(&s.district, "yollarının ilçe"));
            if let Some(p) = problem {
                return Some(at(&p));
            }
        }
        if self.areas.len() > 3 {
            return Some(at("bölge katmanları çok; her düzeyden en çok bir"));
        }
        for (i, a) in self.areas.iter().enumerate() {
            let level = a.level.word();
            if a.layer.is_empty() {
                return Some(at(&format!("{level} katmanının kimliği boş")));
            }
            if let Some(p) = field_problem(&a.name, &format!("{level} alanlarının ad")) {
                return Some(at(&p));
            }
            match (&a.district, a.level) {
                (Some(d), AreaLevel::Neighbourhood) => {
                    if let Some(p) = field_problem(d, "mahalle alanlarının ilçe") {
                        return Some(at(&p));
                    }
                }
                (Some(_), _) => {
                    return Some(at(&format!(
                        "{level} alanlarında ilçe alanı olmaz; yalnız mahalle alanlarının ilçesi yazılır"
                    )));
                }
                (None, _) => {}
            }
            if self.areas[..i].iter().any(|o| o.level == a.level) {
                return Some(at(&format!("{level} düzeyi iki kez var")));
            }
        }
        if !(ADDRESS_MIN_SCORES.0..=ADDRESS_MIN_SCORES.1).contains(&self.min_score) {
            return Some(at(&format!(
                "en düşük puanı {}; {} ile {} arasında olmalı",
                self.min_score, ADDRESS_MIN_SCORES.0, ADDRESS_MIN_SCORES.1
            )));
        }
        if !address_reach_holds(self.reach) {
            return Some(at(&format!(
                "uzaklığı {} m; 0'dan büyük, en çok 10 000 m olmalı",
                self.reach
            )));
        }
        None
    }

    /// The layers it reads (doors, streets, then the areas), each once in its first place.
    pub fn layers(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for l in self
            .doors
            .iter()
            .map(|d| d.layer.as_str())
            .chain(self.streets.iter().map(|s| s.layer.as_str()))
            .chain(self.areas.iter().map(|a| a.layer.as_str()))
        {
            if !out.contains(&l) {
                out.push(l);
            }
        }
        out
    }

    /// Its area layer of a level.
    pub fn area(&self, level: AreaLevel) -> Option<&AreaSource> {
        self.areas.iter().find(|a| a.level == level)
    }
}

/// What is wrong with a project's locators: too many, an id or a name twice, or one that does not hold.
pub fn address_locators_problem(locators: &[AddressLocator]) -> Option<String> {
    if locators.len() > ADDRESS_LOCATOR_LIMIT {
        return Some(format!(
            "projede {} adres verisi var; en çok {ADDRESS_LOCATOR_LIMIT}",
            locators.len()
        ));
    }
    for (i, l) in locators.iter().enumerate() {
        if let Some(p) = l.problem() {
            return Some(p);
        }
        if locators[..i].iter().any(|o| o.id == l.id) {
            return Some(format!("“{}” kimlikli adres verisi iki kez var", l.id));
        }
        let key = crate::network_caseless(&l.name);
        if locators[..i]
            .iter()
            .any(|o| crate::network_caseless(&o.name) == key)
        {
            return Some(format!("“{}” adlı adres verisi iki kez var", l.name));
        }
    }
    None
}

/// The locators as a project keeps them: those that hold, the first of an id or a name, at most [`ADDRESS_LOCATOR_LIMIT`].
pub fn sanitized_address_locators(locators: Vec<AddressLocator>) -> Vec<AddressLocator> {
    let mut kept: Vec<AddressLocator> =
        Vec::with_capacity(locators.len().min(ADDRESS_LOCATOR_LIMIT));
    for l in locators {
        if kept.len() == ADDRESS_LOCATOR_LIMIT {
            break;
        }
        let key = crate::network_caseless(&l.name);
        if l.problem().is_none()
            && !kept
                .iter()
                .any(|o| o.id == l.id || crate::network_caseless(&o.name) == key)
        {
            kept.push(l);
        }
    }
    kept
}

/// The id a new locator takes: the first free `adres-N`.
pub fn next_address_locator_id(locators: &[AddressLocator]) -> String {
    (1..)
        .map(|n| format!("adres-{n}"))
        .find(|id| !locators.iter().any(|o| o.id == *id))
        .unwrap_or_else(|| "adres".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numarataj() -> AddressLocator {
        AddressLocator {
            id: "adres-1".into(),
            name: "Numarataj".into(),
            doors: Some(DoorSource {
                layer: "kapi".into(),
                street: "YOL_ADI".into(),
                door: "KAPI_NO".into(),
                neighbourhood: Some("MAHALLE".into()),
                district: None,
                province: None,
                postcode: None,
                building: None,
                code: Some("UAVT".into()),
            }),
            streets: Some(StreetSource {
                layer: "yol".into(),
                name: "AD".into(),
                neighbourhood: None,
                district: None,
            }),
            areas: vec![AreaSource {
                layer: "mahalle".into(),
                level: AreaLevel::Neighbourhood,
                name: "AD".into(),
                district: None,
            }],
            min_score: ADDRESS_MIN_SCORE,
            reach: ADDRESS_REACH,
        }
    }

    #[test]
    fn a_locator_holds_and_names_its_layers_once() {
        let l = numarataj();
        assert_eq!(l.problem(), None);
        assert_eq!(l.layers(), vec!["kapi", "yol", "mahalle"]);
        assert_eq!(address_locators_problem(std::slice::from_ref(&l)), None);
        assert_eq!(next_address_locator_id(&[l]), "adres-2");
    }

    #[test]
    fn the_rules_say_what_is_wrong() {
        let mut l = numarataj();
        l.id = "Adres 1".into();
        assert!(l.problem().unwrap().contains("kimliği"));
        let mut l = numarataj();
        l.doors = None;
        l.streets = None;
        l.areas.clear();
        assert!(l.problem().unwrap().contains("kaynağı yok"));
        let mut l = numarataj();
        l.doors.as_mut().unwrap().door = " KAPI".into();
        assert!(l.problem().unwrap().contains("kapı no"));
        let mut l = numarataj();
        l.areas.push(l.areas[0].clone());
        assert!(l.problem().unwrap().contains("iki kez"));
        let mut l = numarataj();
        l.areas[0].level = AreaLevel::District;
        l.areas[0].district = Some("ILCE".into());
        assert!(l.problem().unwrap().contains("ilçe alanı olmaz"));
        let mut l = numarataj();
        l.min_score = 40;
        assert!(l.problem().unwrap().contains("en düşük puanı"));
        let mut l = numarataj();
        l.reach = 0.0;
        assert!(l.problem().unwrap().contains("uzaklığı"));
        let mut two = numarataj();
        two.id = "adres-2".into();
        two.name = "NUMARATAJ".into();
        assert!(
            address_locators_problem(&[numarataj(), two.clone()])
                .unwrap()
                .contains("iki kez")
        );
        assert_eq!(sanitized_address_locators(vec![numarataj(), two]).len(), 1);
    }
}
