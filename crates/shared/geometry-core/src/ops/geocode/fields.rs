//! Alanları tanı (docs/adr/0229 §13): which attributes of a layer say what,
//! guessed from their names. A name's words are its folded words (`KAPI_NO`
//! → `kapi`, `no`), its compact form those words run together. Each role
//! takes the first name, in the layer's order, that its rule takes and no
//! role before it took; the roles are tried in this order: code, door,
//! postcode, street, neighbourhood, district, province, building. A name
//! with a word of a code or a kind (`kod`, `kodu`, `id`, `kimlik`, `tip`,
//! `tipi`, `tur`, `turu`, `no`) is no name of a street, a neighbourhood, a
//! district, a province or a building.

use super::fold::{fold, tokens};
use super::locator::Level;

/// A field's words and its compact form.
struct Name {
    words: Vec<String>,
    compact: String,
}

fn name_of(field: &str) -> Name {
    let words: Vec<String> = tokens(&fold(field))
        .into_iter()
        .filter_map(|t| t.word().map(str::to_owned))
        .collect();
    let compact = words.concat();
    Name { words, compact }
}

impl Name {
    fn has(&self, word: &str) -> bool {
        self.words.iter().any(|w| w == word)
    }

    fn coded(&self) -> bool {
        [
            "kod", "kodu", "id", "kimlik", "tip", "tipi", "tur", "turu", "no",
        ]
        .iter()
        .any(|w| self.has(w))
    }
}

fn code(n: &Name) -> bool {
    n.compact.contains("uavt") || (n.has("adres") && (n.has("kod") || n.has("kodu")))
}

fn door(n: &Name) -> bool {
    !n.has("ic")
        && (n.has("kapi")
            || n.compact.starts_with("kapino")
            || n.has("numara")
            || n.words == ["no"])
}

fn postcode(n: &Name) -> bool {
    n.has("posta") || n.compact == "pk" || n.compact == "postakodu"
}

fn street(n: &Name) -> bool {
    !n.coded()
        && ["yol", "sokak", "cadde", "csbm", "bulvar"]
            .iter()
            .any(|w| n.compact.contains(w))
}

fn hood(n: &Name) -> bool {
    !n.coded() && (n.compact.contains("mahalle") || n.has("mah") || n.compact.contains("koy"))
}

fn district(n: &Name) -> bool {
    !n.coded() && n.compact.contains("ilce")
}

fn province(n: &Name) -> bool {
    !n.coded() && (n.has("il") || n.compact.contains("sehir") || n.compact == "iladi")
}

fn building(n: &Name) -> bool {
    !n.coded()
        && ["bina", "apartman", "site"]
            .iter()
            .any(|w| n.compact.contains(w))
}

fn plain_name(n: &Name) -> bool {
    !n.coded() && ["ad", "adi", "isim", "name"].iter().any(|w| n.has(w))
}

/// Takes, for each role in order, the first untaken field its rule takes.
fn take(fields: &[String], roles: &[&dyn Fn(&Name) -> bool]) -> Vec<Option<String>> {
    let names: Vec<Name> = fields.iter().map(|f| name_of(f)).collect();
    let mut taken = vec![false; fields.len()];
    roles
        .iter()
        .map(|rule| {
            let i = (0..fields.len()).find(|&i| !taken[i] && rule(&names[i]))?;
            taken[i] = true;
            Some(fields[i].clone())
        })
        .collect()
}

/// A door layer's fields guessed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoorFields {
    pub street: Option<String>,
    pub door: Option<String>,
    pub neighbourhood: Option<String>,
    pub district: Option<String>,
    pub province: Option<String>,
    pub postcode: Option<String>,
    pub building: Option<String>,
    pub code: Option<String>,
}

pub fn doors(fields: &[String]) -> DoorFields {
    let mut t = take(
        fields,
        &[
            &code, &door, &postcode, &street, &hood, &district, &province, &building,
        ],
    )
    .into_iter();
    let mut next = || t.next().flatten();
    let (code, door, postcode, street, neighbourhood, district, province, building) = (
        next(),
        next(),
        next(),
        next(),
        next(),
        next(),
        next(),
        next(),
    );
    DoorFields {
        street,
        door,
        neighbourhood,
        district,
        province,
        postcode,
        building,
        code,
    }
}

/// A street layer's fields guessed: its name (a street's, else a plain name), neighbourhood and district.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreetFields {
    pub name: Option<String>,
    pub neighbourhood: Option<String>,
    pub district: Option<String>,
}

pub fn streets(fields: &[String]) -> StreetFields {
    let named = |n: &Name| street(n) || plain_name(n);
    let mut t = take(fields, &[&named, &hood, &district]).into_iter();
    let mut next = || t.next().flatten();
    let (name, neighbourhood, district) = (next(), next(), next());
    StreetFields {
        name,
        neighbourhood,
        district,
    }
}

/// An area layer's fields guessed: its name (a plain name, else its level's) and a neighbourhood's district.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AreaFields {
    pub name: Option<String>,
    pub district: Option<String>,
}

pub fn areas(level: Level, fields: &[String]) -> AreaFields {
    let own: &dyn Fn(&Name) -> bool = match level {
        Level::Neighbourhood => &hood,
        Level::District => &district,
        Level::Province => &province,
    };
    let named = |n: &Name| plain_name(n) || own(n);
    match level {
        Level::Neighbourhood => {
            let mut t = take(fields, &[&named, &district]).into_iter();
            let mut next = || t.next().flatten();
            let (name, district) = (next(), next());
            AreaFields { name, district }
        }
        _ => AreaFields {
            name: take(fields, &[&named]).into_iter().next().flatten(),
            district: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn a_numbering_layers_fields_are_found_by_their_names() {
        let f = doors(&list(&[
            "OBJECTID",
            "MAHALLE_KODU",
            "MAHALLE_ADI",
            "YOL_ID",
            "YOL_ADI",
            "DIS_KAPI_NO",
            "IC_KAPI_NO",
            "ILCE",
            "IL_ADI",
            "POSTA_KODU",
            "BINA_ADI",
            "UAVT_KODU",
        ]));
        assert_eq!(f.code.as_deref(), Some("UAVT_KODU"));
        assert_eq!(f.door.as_deref(), Some("DIS_KAPI_NO"));
        assert_eq!(f.postcode.as_deref(), Some("POSTA_KODU"));
        assert_eq!(f.street.as_deref(), Some("YOL_ADI"));
        assert_eq!(f.neighbourhood.as_deref(), Some("MAHALLE_ADI"));
        assert_eq!(f.district.as_deref(), Some("ILCE"));
        assert_eq!(f.province.as_deref(), Some("IL_ADI"));
        assert_eq!(f.building.as_deref(), Some("BINA_ADI"));
    }
}
