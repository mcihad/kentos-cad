//! Adres verisi's form (docs/adr/0229 §2, §13): a locator's definition as
//! the window edits it: its three parts (Kapı noktaları, Yollar, Bölgeler),
//! each on or off with its layer and the attributes that say what, and its
//! numbers as typed (Konumdan adres's reach in the project's unit). The
//! definition it writes is the contract's (`AddressLocator`, whose rules say
//! what is wrong); Alanları tanı fills a part's fields from its layer's
//! names by the core's rules (`ops::geocode::fields`, §13.1).

use kentos_commands::{
    ADDRESS_MIN_SCORE, ADDRESS_MIN_SCORES, ADDRESS_REACH, ADDRESS_REACH_LIMIT, AddressLocator,
    AreaLevel, AreaSource, DoorSource, StreetSource,
};
use kentos_geometry_core::ops::geocode::Level;
use kentos_geometry_core::ops::geocode::fields;

/// The doors' fields, in the form's order: their names and whether one must be chosen.
pub const DOOR_FIELDS: [(&str, bool); 8] = [
    ("Yol", true),
    ("Kapı no", true),
    ("Mahalle", false),
    ("İlçe", false),
    ("İl", false),
    ("Posta kodu", false),
    ("Bina adı", false),
    ("Adres kodu", false),
];

/// The streets' fields.
pub const STREET_FIELDS: [(&str, bool); 3] = [("Ad", true), ("Mahalle", false), ("İlçe", false)];

/// The areas' levels, in the form's order.
pub const LEVELS: [(AreaLevel, &str); 3] = [
    (AreaLevel::Neighbourhood, "Mahalle"),
    (AreaLevel::District, "İlçe"),
    (AreaLevel::Province, "İl"),
];

/// Kapı noktaları: on or off, the layer, the fields (empty: none) in [`DOOR_FIELDS`]' order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DoorForm {
    pub on: bool,
    pub layer: String,
    pub fields: [String; 8],
}

/// Yollar: on or off, the layer, the fields in [`STREET_FIELDS`]' order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StreetForm {
    pub on: bool,
    pub layer: String,
    pub fields: [String; 3],
}

/// A level of Bölgeler: on or off, the layer, its name's field and (a neighbourhood's) its district's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AreaForm {
    pub on: bool,
    pub layer: String,
    pub name: String,
    pub district: String,
}

/// A locator as the window edits it.
#[derive(Clone, Debug, PartialEq)]
pub struct AddressForm {
    pub id: String,
    pub name: String,
    pub doors: DoorForm,
    pub streets: StreetForm,
    /// Mahalle, İlçe and İl, in [`LEVELS`]' order.
    pub areas: [AreaForm; 3],
    pub min_score: String,
    /// Konumdan adres's reach, in the project's unit.
    pub reach: String,
}

fn opt(v: &Option<String>) -> String {
    v.clone().unwrap_or_default()
}

fn some(v: &str) -> Option<String> {
    let v = v.trim();
    (!v.is_empty()).then(|| v.to_owned())
}

/// A number as the form shows it: no trailing zeros.
fn number(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A definition as the form shows it.
pub fn form_of(def: &AddressLocator, from_metres: impl Fn(f64) -> f64) -> AddressForm {
    let doors = def
        .doors
        .as_ref()
        .map_or_else(DoorForm::default, |d| DoorForm {
            on: true,
            layer: d.layer.clone(),
            fields: [
                d.street.clone(),
                d.door.clone(),
                opt(&d.neighbourhood),
                opt(&d.district),
                opt(&d.province),
                opt(&d.postcode),
                opt(&d.building),
                opt(&d.code),
            ],
        });
    let streets = def
        .streets
        .as_ref()
        .map_or_else(StreetForm::default, |s| StreetForm {
            on: true,
            layer: s.layer.clone(),
            fields: [s.name.clone(), opt(&s.neighbourhood), opt(&s.district)],
        });
    let areas = LEVELS.map(|(level, _)| {
        def.area(level)
            .map_or_else(AreaForm::default, |a| AreaForm {
                on: true,
                layer: a.layer.clone(),
                name: a.name.clone(),
                district: opt(&a.district),
            })
    });
    AddressForm {
        id: def.id.clone(),
        name: def.name.clone(),
        doors,
        streets,
        areas,
        min_score: def.min_score.to_string(),
        reach: number(from_metres(def.reach)),
    }
}

/// A new definition's form: its parts off, the scores and the reach the ADR's.
pub fn new_form(id: &str, name: &str, from_metres: impl Fn(f64) -> f64) -> AddressForm {
    AddressForm {
        id: id.to_owned(),
        name: name.to_owned(),
        doors: DoorForm::default(),
        streets: StreetForm::default(),
        areas: Default::default(),
        min_score: ADDRESS_MIN_SCORE.to_string(),
        reach: number(from_metres(ADDRESS_REACH)),
    }
}

/// The definition the form writes, or the first thing wrong in the window's words.
pub fn def_of(
    form: &AddressForm,
    to_metres: impl Fn(f64) -> f64,
) -> Result<AddressLocator, String> {
    let name = form.name.trim();
    let what = if name.is_empty() {
        "Adsız adres verisi".to_owned()
    } else {
        format!("“{name}”")
    };
    let (low, high) = ADDRESS_MIN_SCORES;
    let min_score = form
        .min_score
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|s| (low..=high).contains(s))
        .ok_or_else(|| {
            format!("{what}: en düşük puan {low} ile {high} arasında bir tam sayı olmalı.")
        })?;
    let reach = form
        .reach
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .map(&to_metres)
        .filter(|r| r.is_finite() && *r > 0.0 && *r <= ADDRESS_REACH_LIMIT)
        .ok_or_else(|| {
            format!(
                "{what}: Konumdan adres uzaklığı sıfırdan büyük ve en çok {} m olmalı.",
                number(ADDRESS_REACH_LIMIT)
            )
        })?;
    let doors = if form.doors.on {
        let d = &form.doors;
        if d.layer.trim().is_empty() {
            return Err(format!("{what}: kapı noktalarının katmanını seçin."));
        }
        let (Some(street), Some(door)) = (some(&d.fields[0]), some(&d.fields[1])) else {
            return Err(format!(
                "{what}: kapı noktalarının yol ve kapı no alanlarını seçin."
            ));
        };
        Some(DoorSource {
            layer: d.layer.clone(),
            street,
            door,
            neighbourhood: some(&d.fields[2]),
            district: some(&d.fields[3]),
            province: some(&d.fields[4]),
            postcode: some(&d.fields[5]),
            building: some(&d.fields[6]),
            code: some(&d.fields[7]),
        })
    } else {
        None
    };
    let streets = if form.streets.on {
        let s = &form.streets;
        if s.layer.trim().is_empty() {
            return Err(format!("{what}: yolların katmanını seçin."));
        }
        let Some(street) = some(&s.fields[0]) else {
            return Err(format!("{what}: yolların ad alanını seçin."));
        };
        Some(StreetSource {
            layer: s.layer.clone(),
            name: street,
            neighbourhood: some(&s.fields[1]),
            district: some(&s.fields[2]),
        })
    } else {
        None
    };
    let mut areas = Vec::new();
    for ((level, word), a) in LEVELS.iter().zip(&form.areas) {
        if !a.on {
            continue;
        }
        let lower = crate::lower_tr(word);
        if a.layer.trim().is_empty() {
            return Err(format!("{what}: {lower} alanlarının katmanını seçin."));
        }
        let Some(area) = some(&a.name) else {
            return Err(format!("{what}: {lower} alanlarının ad alanını seçin."));
        };
        areas.push(AreaSource {
            layer: a.layer.clone(),
            level: *level,
            name: area,
            district: (*level == AreaLevel::Neighbourhood)
                .then(|| some(&a.district))
                .flatten(),
        });
    }
    let def = AddressLocator {
        id: form.id.clone(),
        name: name.to_owned(),
        doors,
        streets,
        areas,
        min_score,
        reach,
    };
    match def.problem() {
        Some(p) => Err(format!("{what}: {p}.")),
        None => Ok(def),
    }
}

/// Alanları tanı for Kapı noktaları: each field the layer's names give, by the core's rules.
pub fn guess_doors(form: &mut DoorForm, names: &[String]) {
    let g = fields::doors(names);
    form.fields = [
        opt(&g.street),
        opt(&g.door),
        opt(&g.neighbourhood),
        opt(&g.district),
        opt(&g.province),
        opt(&g.postcode),
        opt(&g.building),
        opt(&g.code),
    ];
}

/// Alanları tanı for Yollar.
pub fn guess_streets(form: &mut StreetForm, names: &[String]) {
    let g = fields::streets(names);
    form.fields = [opt(&g.name), opt(&g.neighbourhood), opt(&g.district)];
}

/// Alanları tanı for a level of Bölgeler.
pub fn guess_area(form: &mut AreaForm, level: AreaLevel, names: &[String]) {
    let level = match level {
        AreaLevel::Neighbourhood => Level::Neighbourhood,
        AreaLevel::District => Level::District,
        AreaLevel::Province => Level::Province,
    };
    let g = fields::areas(level, names);
    form.name = opt(&g.name);
    form.district = if level == Level::Neighbourhood {
        opt(&g.district)
    } else {
        String::new()
    };
}

/// Alanları tanı for every part that has a layer: `names` gives a layer's names.
pub fn guess_all(form: &mut AddressForm, names: impl Fn(&str) -> Vec<String>) {
    if form.doors.on && !form.doors.layer.is_empty() {
        let n = names(&form.doors.layer);
        guess_doors(&mut form.doors, &n);
    }
    if form.streets.on && !form.streets.layer.is_empty() {
        let n = names(&form.streets.layer);
        guess_streets(&mut form.streets, &n);
    }
    for ((level, _), a) in LEVELS.iter().zip(form.areas.iter_mut()) {
        if a.on && !a.layer.is_empty() {
            let n = names(&a.layer);
            guess_area(a, *level, &n);
        }
    }
}

/// What the list says of a definition under its name: its parts.
pub fn parts_line(form: &AddressForm) -> String {
    let mut parts: Vec<String> = Vec::new();
    if form.doors.on {
        parts.push("kapılar".into());
    }
    if form.streets.on {
        parts.push("yollar".into());
    }
    let areas = form.areas.iter().filter(|a| a.on).count();
    if areas > 0 {
        parts.push(format!("{areas} bölge düzeyi"));
    }
    if parts.is_empty() {
        "bölüm seçilmedi".into()
    } else {
        let line = parts.join(", ");
        let mut c = line.chars();
        c.next()
            .map(|f| crate::upper_tr(&f.to_string()) + c.as_str())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def() -> AddressLocator {
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
                postcode: Some("POSTA_KODU".into()),
                building: None,
                code: Some("UAVT".into()),
            }),
            streets: None,
            areas: vec![AreaSource {
                layer: "mahalle".into(),
                level: AreaLevel::Neighbourhood,
                name: "AD".into(),
                district: Some("ILCE".into()),
            }],
            min_score: 85,
            reach: 100.0,
        }
    }

    #[test]
    fn a_definition_comes_back_from_its_form() {
        let d = def();
        let f = form_of(&d, |m| m * 1000.0);
        assert_eq!(f.reach, "100000");
        assert_eq!(f.doors.fields[0], "YOL_ADI");
        assert!(!f.streets.on && f.areas[0].on && !f.areas[1].on);
        assert_eq!(def_of(&f, |v| v / 1000.0), Ok(d));
        assert_eq!(parts_line(&f), "Kapılar, 1 bölge düzeyi");
    }

    #[test]
    fn the_form_says_what_is_wrong() {
        let mut f = form_of(&def(), |m| m);
        f.min_score = "40".into();
        assert!(
            def_of(&f, |v| v)
                .unwrap_err()
                .contains("en düşük puan 50 ile 100")
        );
        let mut f = form_of(&def(), |m| m);
        f.reach = "0".into();
        assert!(def_of(&f, |v| v).unwrap_err().contains("uzaklığı"));
        let mut f = form_of(&def(), |m| m);
        f.doors.fields[1].clear();
        assert!(def_of(&f, |v| v).unwrap_err().contains("yol ve kapı no"));
        let mut f = form_of(&def(), |m| m);
        f.doors.on = false;
        f.areas[0].on = false;
        let why = def_of(&f, |v| v).unwrap_err();
        assert!(why.starts_with("“Numarataj”: "), "{why}");
        let mut f = new_form("adres-2", "Yeni", |m| m);
        f.streets.on = true;
        f.streets.layer = "yol".into();
        assert!(
            def_of(&f, |v| v)
                .unwrap_err()
                .contains("yolların ad alanını")
        );
    }

    #[test]
    fn fields_are_guessed_from_the_layers_names() {
        let mut f = new_form("adres-1", "Numarataj", |m| m);
        f.doors.on = true;
        f.doors.layer = "kapi".into();
        f.areas[0].on = true;
        f.areas[0].layer = "mahalle".into();
        guess_all(&mut f, |l| match l {
            "kapi" => ["UAVT", "YOL_ADI", "KAPI_NO", "MAHALLE"]
                .map(String::from)
                .to_vec(),
            _ => ["AD", "ILCE"].map(String::from).to_vec(),
        });
        assert_eq!(
            f.doors.fields[..2],
            ["YOL_ADI".to_owned(), "KAPI_NO".to_owned()]
        );
        assert_eq!(f.doors.fields[7], "UAVT");
        assert_eq!(
            (f.areas[0].name.as_str(), f.areas[0].district.as_str()),
            ("AD", "ILCE")
        );
    }
}
