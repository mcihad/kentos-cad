//! The Özel koordinat sistemi window's rules (docs/adr/0168 §1–§2, §6; the
//! web's `model/definitionForm.ts`): what is typed turned into the project's
//! definition (`CrsDefinition`) or, field by field, what is wrong; a
//! definition the registry has already is said. The shared cases are
//! fixtures/crs/v1/definition-form.json
//! (scripts/fixtures/crs_definition_form_cases.py, from the ADR's rules).

use std::collections::BTreeMap;

use kentos_contracts::{
    Convention, CrsBase, CrsDefinition, CrsPlane, CrsSystem, CustomDatum, Ellipsoid,
    GeographicDefinition, Helmert, LocalDefinition, RegistryDatum, TmDefinition,
};

use crate::crs::{system, systems};
use crate::systems::definition_system;

/// What the definition is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Tm,
    Geographic,
    Local,
}

/// A transverse Mercator's or a geographic system's datum: one of the
/// registry's by name, or the project's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatumPick {
    Registry(RegistryDatum),
    Custom,
}

/// A local system's plane transform.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaneKind {
    #[default]
    Similarity,
    Affine,
}

/// The classic ellipsoids by name: semi-major axis (m), inverse flattening
/// (the core's `crs::text` table).
pub const ELLIPSOIDS: [(&str, f64, f64); 6] = [
    ("GRS 1980", 6_378_137.0, 298.257_222_101),
    ("WGS 84", 6_378_137.0, 298.257_223_563),
    ("International 1924", 6_378_388.0, 297.0),
    ("Bessel 1841", 6_377_397.155, 299.152_812_8),
    ("Krasovski 1940", 6_378_245.0, 298.3),
    ("Clarke 1880 (RGS)", 6_378_249.145, 293.465),
];

/// The seven parameters' fields, in their order.
pub const PARAMETERS: [&str; 7] = ["tx", "ty", "tz", "rx", "ry", "rz", "ds"];

/// The affine's fields, in their order.
pub const AFFINE: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

/// What the window holds, as typed.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub name: String,
    pub kind: Kind,
    pub latitude_of_origin: String,
    pub central_meridian: String,
    pub scale_factor: String,
    pub false_easting: String,
    pub false_northing: String,
    pub datum: DatumPick,
    pub datum_name: String,
    /// One of `ELLIPSOIDS` by its place; none: typed.
    pub ellipsoid: Option<usize>,
    pub semi_major: String,
    pub inverse_flattening: String,
    /// The project's datum is bound to WGS 84 by seven parameters.
    pub linked: bool,
    pub parameters: [String; 7],
    pub convention: Convention,
    pub accuracy: String,
    /// The local system's base: a projected system of the registry by its code.
    pub base: String,
    pub plane: PlaneKind,
    pub east: String,
    pub north: String,
    pub rotation: String,
    pub scale: String,
    pub affine: [String; 6],
}

impl Default for Form {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: Kind::Tm,
            latitude_of_origin: String::new(),
            central_meridian: String::new(),
            scale_factor: String::new(),
            false_easting: String::new(),
            false_northing: String::new(),
            datum: DatumPick::Registry(RegistryDatum::Turef),
            datum_name: String::new(),
            ellipsoid: Some(0),
            semi_major: String::new(),
            inverse_flattening: String::new(),
            linked: true,
            parameters: Default::default(),
            convention: Convention::PositionVector,
            accuracy: String::new(),
            base: String::new(),
            plane: PlaneKind::Similarity,
            east: String::new(),
            north: String::new(),
            rotation: String::new(),
            scale: String::new(),
            affine: Default::default(),
        }
    }
}

/// What is wrong, by field.
pub type Problems = BTreeMap<&'static str, &'static str>;

pub const NAME: &str = "Adını yazın: sistem bu adla görünür.";
pub const NUMBER: &str = "Sayı yazın.";
pub const MERIDIAN: &str = "−180 ile 180 arasında bir derece yazın.";
pub const LATITUDE: &str = "−90 ile 90 arasında bir derece yazın.";
pub const POSITIVE: &str = "0'dan büyük bir sayı yazın.";
pub const DATUM_NAME: &str = "Datumun adını yazın.";
pub const SEMI_MAJOR: &str = "Büyük yarı ekseni metre olarak, 0'dan büyük yazın.";
pub const INVERSE_FLATTENING: &str = "Ters basıklığı 1'den büyük yazın.";
pub const ACCURACY: &str = "0 ya da büyük bir sayı yazın; bilinmiyorsa boş bırakın.";
pub const BASE: &str = "Kayıttaki projeksiyonlu bir sistem seçin.";
pub const FOLDS: &str = "Bu katsayılar düzlemi katlıyor (a·e − b·d = 0).";

/// A number as the Hesap windows read one: trimmed, its first comma a
/// point, `^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$`; none for anything else.
pub fn number(text: &str) -> Option<f64> {
    let t = text.trim().replacen(',', ".", 1);
    let b = t.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let whole = digits(i);
    i += whole;
    let mut fraction = 0;
    if b.get(i) == Some(&b'.') {
        fraction = digits(i + 1);
        i += 1 + fraction;
    }
    if whole == 0 && fraction == 0 {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        i += usize::from(matches!(b.get(i), Some(b'-' | b'+')));
        let exponent = digits(i);
        if exponent == 0 {
            return None;
        }
        i += exponent;
    }
    (i == b.len()).then(|| t.parse().ok()).flatten()
}

/// A number's rule and what is said when it fails it.
type Check = Option<(fn(f64) -> bool, &'static str)>;

/// Reads the form's numbers, keeping the problems by field.
struct Reader {
    problems: Problems,
}

impl Reader {
    /// The number typed in `key`: `default` when empty (none: required), and
    /// `check`'s `problem` when it fails it. Zero when wrong: the problems say so.
    fn number(&mut self, key: &'static str, text: &str, default: Option<f64>, check: Check) -> f64 {
        let t = text.trim();
        if t.is_empty()
            && let Some(d) = default
        {
            return d;
        }
        let Some(v) = number(t) else {
            self.problems.insert(key, NUMBER);
            return 0.0;
        };
        if let Some((ok, problem)) = check
            && !ok(v)
        {
            self.problems.insert(key, problem);
            return 0.0;
        }
        v
    }
}

fn custom_datum(form: &Form, r: &mut Reader) -> CustomDatum {
    let name = form.datum_name.trim();
    if name.is_empty() {
        r.problems.insert("datumName", DATUM_NAME);
    }
    let ellipsoid = match form.ellipsoid.and_then(|i| ELLIPSOIDS.get(i)) {
        Some(&(name, a, rf)) => Ellipsoid {
            name: name.to_owned(),
            semi_major: a,
            inverse_flattening: rf,
        },
        None => Ellipsoid {
            name: format!(
                "a={}, 1/f={}",
                form.semi_major.trim(),
                form.inverse_flattening.trim()
            ),
            semi_major: r.number(
                "semiMajor",
                &form.semi_major,
                None,
                Some((|v| v > 0.0, SEMI_MAJOR)),
            ),
            inverse_flattening: r.number(
                "inverseFlattening",
                &form.inverse_flattening,
                None,
                Some((|v| v > 1.0, INVERSE_FLATTENING)),
            ),
        },
    };
    let to_wgs84 = form.linked.then(|| {
        let mut v = [0.0; 7];
        for (i, key) in PARAMETERS.iter().enumerate() {
            // An empty rotation or scale difference is 0: three parameters.
            let default = (i >= 3).then_some(0.0);
            v[i] = r.number(key, &form.parameters[i], default, None);
        }
        let accuracy = match form.accuracy.trim() {
            "" => None,
            t => match number(t) {
                Some(a) if a >= 0.0 => Some(a),
                _ => {
                    r.problems.insert("accuracy", ACCURACY);
                    None
                }
            },
        };
        Helmert {
            translation: [v[0], v[1], v[2]],
            rotation: [v[3], v[4], v[5]],
            scale: v[6],
            convention: form.convention,
            accuracy,
        }
    });
    CustomDatum {
        name: name.to_owned(),
        ellipsoid,
        to_wgs84,
    }
}

/// A definition's datum fields: the registry's name or the project's own.
fn datum_fields(form: &Form, r: &mut Reader) -> (Option<RegistryDatum>, Option<Box<CustomDatum>>) {
    match form.datum {
        DatumPick::Registry(d) => (Some(d), None),
        DatumPick::Custom => (None, Some(Box::new(custom_datum(form, r)))),
    }
}

/// The definition the window gives, with what to say of it (the registry's
/// system it is); or what is wrong.
pub fn build(form: &Form) -> Result<(CrsDefinition, Option<String>), Problems> {
    let mut r = Reader {
        problems: Problems::new(),
    };
    let name = form.name.trim();
    if name.is_empty() {
        r.problems.insert("name", NAME);
    }
    let system = match form.kind {
        Kind::Tm => {
            let (datum, custom_datum) = datum_fields(form, &mut r);
            let lat0 = r.number(
                "latitudeOfOrigin",
                &form.latitude_of_origin,
                Some(0.0),
                Some((|v| (-90.0..=90.0).contains(&v), LATITUDE)),
            );
            CrsSystem::Tm(TmDefinition {
                datum,
                custom_datum,
                latitude_of_origin: (lat0 != 0.0).then_some(lat0),
                central_meridian: r.number(
                    "centralMeridian",
                    &form.central_meridian,
                    None,
                    Some((|v| (-180.0..=180.0).contains(&v), MERIDIAN)),
                ),
                scale_factor: r.number(
                    "scaleFactor",
                    &form.scale_factor,
                    Some(1.0),
                    Some((|v| v > 0.0, POSITIVE)),
                ),
                false_easting: r.number("falseEasting", &form.false_easting, None, None),
                false_northing: r.number("falseNorthing", &form.false_northing, None, None),
            })
        }
        Kind::Geographic => {
            let (datum, custom_datum) = datum_fields(form, &mut r);
            CrsSystem::Geographic(GeographicDefinition {
                datum,
                custom_datum,
            })
        }
        Kind::Local => {
            let base = form
                .base
                .trim()
                .parse::<u32>()
                .ok()
                .and_then(system)
                .filter(|s| s.kind == "projected");
            if base.is_none() {
                r.problems.insert("base", BASE);
            }
            let plane = match form.plane {
                PlaneKind::Similarity => CrsPlane::Similarity {
                    east: r.number("east", &form.east, None, None),
                    north: r.number("north", &form.north, None, None),
                    rotation: r.number("rotation", &form.rotation, Some(0.0), None),
                    scale: r.number(
                        "scale",
                        &form.scale,
                        Some(1.0),
                        Some((|v| v > 0.0, POSITIVE)),
                    ),
                },
                PlaneKind::Affine => {
                    let wrong = |r: &Reader, k: &str| r.problems.contains_key(k);
                    let mut v = [0.0; 6];
                    for (i, key) in AFFINE.iter().enumerate() {
                        v[i] = r.number(key, &form.affine[i], None, None);
                    }
                    let read = ["a", "b", "d", "e"].iter().all(|k| !wrong(&r, k));
                    if read && v[0] * v[4] - v[1] * v[3] == 0.0 {
                        r.problems.insert("a", FOLDS);
                    }
                    CrsPlane::Affine {
                        a: v[0],
                        b: v[1],
                        c: v[2],
                        d: v[3],
                        e: v[4],
                        f: v[5],
                    }
                }
            };
            CrsSystem::Local(LocalDefinition {
                base: CrsBase {
                    srid: Some(base.map_or(0, |s| s.srid)),
                    definition: None,
                },
                plane,
            })
        }
    };
    if !r.problems.is_empty() {
        return Err(r.problems);
    }
    let definition = CrsDefinition {
        name: name.to_owned(),
        system,
    };
    let note = same_as_registry(&definition);
    Ok((definition, note))
}

/// “EPSG:5254 (TUREF / TM30) ile aynı; kayıttakini seçin.”: the registry's
/// system a definition on one of its datums is, every value the same.
fn same_as_registry(d: &CrsDefinition) -> Option<String> {
    let registry_datum = match &d.system {
        CrsSystem::Tm(t) => t.datum.is_some(),
        CrsSystem::Geographic(g) => g.datum.is_some(),
        CrsSystem::Local(_) => false,
    };
    if !registry_datum {
        return None;
    }
    let it = definition_system(d)?;
    systems()
        .iter()
        .filter(|s| !s.is_local())
        .find(|s| s.transform_system().as_ref() == Some(&it))
        .map(|s| format!("EPSG:{} ({}) ile aynı; kayıttakini seçin.", s.srid, s.name))
}

/// The form of a definition (a new one: an empty TM on TUREF).
pub fn form_of(d: &CrsDefinition) -> Form {
    let text = |v: f64| format!("{v}");
    let mut form = Form {
        name: d.name.clone(),
        ..Form::default()
    };
    let take_datum = |form: &mut Form,
                      datum: Option<RegistryDatum>,
                      custom: Option<&CustomDatum>| match (datum, custom) {
        (Some(r), _) => form.datum = DatumPick::Registry(r),
        (None, Some(c)) => {
            form.datum = DatumPick::Custom;
            form.datum_name = c.name.clone();
            // A classic ellipsoid by its name and values; any other is typed.
            form.ellipsoid = ELLIPSOIDS.iter().position(|(name, a, rf)| {
                *name == c.ellipsoid.name
                    && *a == c.ellipsoid.semi_major
                    && *rf == c.ellipsoid.inverse_flattening
            });
            form.semi_major = text(c.ellipsoid.semi_major);
            form.inverse_flattening = text(c.ellipsoid.inverse_flattening);
            form.linked = c.to_wgs84.is_some();
            if let Some(h) = &c.to_wgs84 {
                form.parameters = [
                    h.translation[0],
                    h.translation[1],
                    h.translation[2],
                    h.rotation[0],
                    h.rotation[1],
                    h.rotation[2],
                    h.scale,
                ]
                .map(text);
                form.convention = h.convention;
                form.accuracy = h.accuracy.map(text).unwrap_or_default();
            }
        }
        (None, None) => {}
    };
    match &d.system {
        CrsSystem::Tm(t) => {
            form.kind = Kind::Tm;
            take_datum(&mut form, t.datum, t.custom_datum.as_deref());
            form.latitude_of_origin = t.latitude_of_origin.map(text).unwrap_or_default();
            form.central_meridian = text(t.central_meridian);
            form.scale_factor = text(t.scale_factor);
            form.false_easting = text(t.false_easting);
            form.false_northing = text(t.false_northing);
        }
        CrsSystem::Geographic(g) => {
            form.kind = Kind::Geographic;
            take_datum(&mut form, g.datum, g.custom_datum.as_deref());
        }
        CrsSystem::Local(l) => {
            form.kind = Kind::Local;
            form.base = l.base.srid.map(|s| s.to_string()).unwrap_or_default();
            match l.plane {
                CrsPlane::Similarity {
                    east,
                    north,
                    rotation,
                    scale,
                } => {
                    form.plane = PlaneKind::Similarity;
                    form.east = text(east);
                    form.north = text(north);
                    form.rotation = text(rotation);
                    form.scale = text(scale);
                }
                CrsPlane::Affine { a, b, c, d, e, f } => {
                    form.plane = PlaneKind::Affine;
                    form.affine = [a, b, c, d, e, f].map(text);
                }
            }
        }
    }
    form
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn form(v: &Value) -> Form {
        let text = |k: &str| v[k].as_str().expect("a text").to_owned();
        Form {
            name: text("name"),
            kind: match v["kind"].as_str() {
                Some("geographic") => Kind::Geographic,
                Some("local") => Kind::Local,
                _ => Kind::Tm,
            },
            latitude_of_origin: text("latitudeOfOrigin"),
            central_meridian: text("centralMeridian"),
            scale_factor: text("scaleFactor"),
            false_easting: text("falseEasting"),
            false_northing: text("falseNorthing"),
            datum: match v["datum"].as_str() {
                Some("custom") => DatumPick::Custom,
                _ => DatumPick::Registry(
                    serde_json::from_value(v["datum"].clone()).expect("a datum"),
                ),
            },
            datum_name: text("datumName"),
            ellipsoid: ELLIPSOIDS
                .iter()
                .position(|(name, ..)| v["ellipsoid"] == *name),
            semi_major: text("semiMajor"),
            inverse_flattening: text("inverseFlattening"),
            linked: v["linked"].as_bool().expect("linked"),
            parameters: PARAMETERS.map(|k| v["parameters"][k].as_str().expect("a text").to_owned()),
            convention: serde_json::from_value(v["convention"].clone()).expect("a convention"),
            accuracy: text("accuracy"),
            base: text("base"),
            plane: match v["plane"].as_str() {
                Some("affine") => PlaneKind::Affine,
                _ => PlaneKind::Similarity,
            },
            east: text("east"),
            north: text("north"),
            rotation: text("rotation"),
            scale: text("scale"),
            affine: AFFINE.map(text),
        }
    }

    /// What is typed turns into the definition and its note, or the
    /// problems by field, as the shared cases say (the web reads the same
    /// file, `model/definitionForm.test.ts`); a definition's form gives it back.
    #[test]
    fn the_forms_are_the_shared_cases() {
        let file: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/crs/v1/definition-form.json"
        ))
        .expect("the cases read");
        assert_eq!(file["format"], "kentos.crs-definition-form");
        let texts = [
            ("name", NAME),
            ("number", NUMBER),
            ("meridian", MERIDIAN),
            ("latitude", LATITUDE),
            ("positive", POSITIVE),
            ("datumName", DATUM_NAME),
            ("semiMajor", SEMI_MAJOR),
            ("inverseFlattening", INVERSE_FLATTENING),
            ("accuracy", ACCURACY),
            ("base", BASE),
            ("folds", FOLDS),
        ];
        for (key, text) in texts {
            assert_eq!(file["texts"][key], text, "{key}");
        }
        for (i, e) in file["ellipsoids"]
            .as_array()
            .expect("ellipsoids")
            .iter()
            .enumerate()
        {
            let (name, a, rf) = ELLIPSOIDS[i];
            assert_eq!(
                (
                    e["name"].as_str(),
                    e["semiMajor"].as_f64(),
                    e["inverseFlattening"].as_f64()
                ),
                (Some(name), Some(a), Some(rf))
            );
        }
        let cases = file["cases"].as_array().expect("cases");
        assert!(cases.len() >= 13);
        for case in cases {
            let name = case["name"].as_str().expect("a name");
            let typed = form(&case["form"]);
            let got = build(&typed);
            match case.get("problems") {
                Some(p) => {
                    let got = got.expect_err(name);
                    let want: BTreeMap<String, String> =
                        serde_json::from_value(p.clone()).expect("problems");
                    let got: BTreeMap<String, String> = got
                        .into_iter()
                        .map(|(k, v)| (k.to_owned(), v.to_owned()))
                        .collect();
                    assert_eq!(got, want, "{name}");
                }
                None => {
                    let (definition, note) = got.unwrap_or_else(|p| panic!("{name}: {p:?}"));
                    let want: CrsDefinition =
                        serde_json::from_value(case["definition"].clone()).expect("a definition");
                    assert_eq!(definition, want, "{name}");
                    assert_eq!(note.as_deref(), case["note"].as_str(), "{name}");
                    // Its form gives it back.
                    let again = build(&form_of(&definition)).map(|(d, _)| d);
                    assert_eq!(again, Ok(want), "{name}");
                }
            }
        }
    }
}
