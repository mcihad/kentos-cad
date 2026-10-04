//! The Özel koordinat sistemi window's rules (docs/adr/0168 §1–§2, §5–§6;
//! the web's `model/definitionForm.ts`): what is typed turned into the
//! project's definition (`CrsDefinition`) or, field by field, what is wrong;
//! a definition the registry has already is said; a WKT or PROJ text read
//! into one, or why not; a definition written as WKT and PROJ; a local
//! system's plane from points known in both systems. The shared cases are
//! fixtures/crs/v1/definition-form.json, definition-text.json and
//! definition-fit.json (scripts/fixtures/crs_definition_form_cases.py,
//! crs_definition_text_cases.py and crs_definition_fit_cases.py, from the
//! ADR's rules).

use std::collections::BTreeMap;

use kentos_contracts::{
    Convention, CrsBase, CrsDefinition, CrsPlane, CrsSystem, CustomDatum, Ellipsoid,
    GeographicDefinition, Helmert, LocalDefinition, RegistryDatum, TmDefinition,
};

use kentos_geometry_core::crs::text::{self as core_text, Refusal};
use kentos_geometry_core::jsmath::{PI, atan2, js_hypot};
use kentos_geometry_core::ops::fit::{self as solver, FitError, FitKind, FitPair};
use kentos_geometry_core::vec2::Vec2;

use crate::crs::{system, systems};
use crate::systems::{definition_from, definition_system};

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

/// The classic ellipsoids by name (EPSG's): semi-major axis (m), inverse
/// flattening.
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

/// What reading a text says when it gives no definition, and of a grid the
/// registry has on the text's own datum; `{detail}`, `{srid}` and `{name}`
/// are filled in.
pub const READ_SYNTAX: &str = "Metin okunamadı: WKT (PROJCS[…], GEOGCS[…], PROJCRS[…] …) ya da +proj= ile başlayan bir PROJ dizesi yapıştırın.";
pub const READ_UNSUPPORTED: &str = "“{detail}” okunmuyor: yalnız Transverse Mercator (UTM dahil), coğrafi sistem ve afinle türetilmiş yerel sistem tanımlanabilir.";
pub const READ_UNIT: &str = "Birim “{detail}”: yalnız metre ve derece okunur.";
pub const READ_MERIDIAN: &str = "Başlangıç meridyeni “{detail}”: yalnız Greenwich okunur.";
pub const READ_GRID: &str = "Izgarası EPSG:{srid} ({name}) ile aynı; datumu metnin kendi datumu.";

/// What Ortak noktalardan hesapla says: rows left out, and why there is no
/// plane; `{rows}`, `{kind}`, `{need}` and `{n}` are filled in.
pub const FIT_SKIPPED: &str = "Satır {rows} hesaba katılmadı: dört değer de sayı olmalı (bu sistemde ve tabanda sağa ve yukarı).";
pub const FIT_TOO_FEW: &str = "{kind} için en az {need} kullanılan ortak nokta gerekir; şimdi {n}.";
pub const FIT_COINCIDENT: &str = "Bu sistemdeki noktaların hepsi aynı yerde; düzlem bulunamaz.";
pub const FIT_COLLINEAR: &str =
    "Bu sistemdeki noktalar bir doğru üstünde; afin bulunamaz. Doğrunun dışında bir nokta ekleyin.";

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

/// The registry's system a definition on one of its datums is, every value
/// the same (the window's Kayıttakini seç).
pub fn same_srid(d: &CrsDefinition) -> Option<u32> {
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
        .map(|s| s.srid)
}

/// “EPSG:5254 (TUREF / TM30) ile aynı; kayıttakini seçin.”
fn same_as_registry(d: &CrsDefinition) -> Option<String> {
    let s = system(same_srid(d)?)?;
    Some(format!(
        "EPSG:{} ({}) ile aynı; kayıttakini seçin.",
        s.srid, s.name
    ))
}

/// A WKT or PROJ text read (docs/adr/0168 §5): the definition it is, the
/// registry's system it is, what its datum shares with the registry.
#[derive(Clone, Debug, PartialEq)]
pub struct Imported {
    pub definition: CrsDefinition,
    pub same: Option<u32>,
    pub note: Option<String>,
}

/// A text pasted or a `.prj` file read as a definition, or why not.
pub fn read(text: &str) -> Result<Imported, String> {
    let r = core_text::read_text(text).map_err(|why| match &why {
        Refusal::Syntax => READ_SYNTAX.to_owned(),
        Refusal::Unsupported(d) => READ_UNSUPPORTED.replace("{detail}", d),
        Refusal::Unit(d) => READ_UNIT.replace("{detail}", d),
        Refusal::Meridian(d) => READ_MERIDIAN.replace("{detail}", d),
    })?;
    let definition = definition_from(&r.name, &r.system)
        .ok_or_else(|| READ_UNSUPPORTED.replace("{detail}", "Pseudo-Mercator"))?;
    let note = match r.registry {
        Some((srid, false)) => system(srid).map(|s| {
            READ_GRID
                .replace("{srid}", &srid.to_string())
                .replace("{name}", &s.name)
        }),
        _ => None,
    };
    Ok(Imported {
        same: same_srid(&definition),
        definition,
        note,
    })
}

/// A row of Ortak noktalardan hesapla as typed: this system's east and
/// north, the base's east and north, and Kullan ("0" leaves it out).
pub type FitRow = [String; 5];

/// The plane the common points give (this system → its base), the rows its
/// pairs came from, each pair's residual (transformed point less the base's:
/// east, north, length; metres) and m0 (none without redundancy).
#[derive(Clone, Debug, PartialEq)]
pub struct PlaneFit {
    pub plane: CrsPlane,
    pub rows: Vec<usize>,
    pub residuals: Vec<[f64; 3]>,
    pub m0: Option<f64>,
}

/// A row's four values when all read as numbers; none for an empty row or
/// one being typed.
fn pair_values(r: &FitRow) -> Option<[f64; 4]> {
    let v: Option<Vec<f64>> = r[..4].iter().map(|v| number(v)).collect();
    v.and_then(|v| v.try_into().ok())
}

/// The rows with something typed that are not pairs: left out of the
/// solution, as Vektör oturtma leaves a row being typed, and named.
pub fn unread_rows(rows: &[FitRow]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_, r)| r[..4].iter().any(|v| !v.trim().is_empty()) && pair_values(r).is_none())
        .map(|(i, _)| i)
        .collect()
}

/// “Satır 3, 5 hesaba katılmadı: …”; none when every row is read.
pub fn skipped_text(rows: &[usize]) -> Option<String> {
    (!rows.is_empty()).then(|| {
        let named: Vec<String> = rows.iter().map(|r| (r + 1).to_string()).collect();
        FIT_SKIPPED.replace("{rows}", &named.join(", "))
    })
}

/// Ortak noktalardan hesapla (docs/adr/0168 §1, §6): the least-squares
/// similarity or affine through the used pairs, as Vektör oturtma solves
/// them (`ops::fit`, its pairs' centred frames), written as the window's
/// plane; or what stops it. Rows that are not pairs are left out.
pub fn fit_plane(rows: &[FitRow], kind: PlaneKind) -> Result<PlaneFit, String> {
    let mut pairs = Vec::new();
    let mut at = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let Some(v) = pair_values(r) else {
            continue;
        };
        pairs.push(FitPair {
            source: Vec2::new(v[0], v[1]),
            target: Vec2::new(v[2], v[3]),
            used: r[4].trim() != "0",
        });
        at.push(i);
    }
    let (solved_by, word) = match kind {
        PlaneKind::Similarity => (FitKind::Helmert, "Benzerlik"),
        PlaneKind::Affine => (FitKind::Affine, "Afin"),
    };
    let fit = solver::fit(&pairs, solved_by).map_err(|why| match why {
        FitError::TooFew(need) => FIT_TOO_FEW
            .replace("{kind}", word)
            .replace("{need}", &need.to_string())
            .replace("{n}", &pairs.iter().filter(|p| p.used).count().to_string()),
        FitError::Coincident => FIT_COINCIDENT.to_owned(),
        FitError::Collinear | FitError::Singular => FIT_COLLINEAR.to_owned(),
    })?;
    // The centred solution un-centred: base = to + M·(p − from).
    let (o, t) = (fit.from, fit.to);
    let plane = match *fit.params.as_slice() {
        [a, b] => CrsPlane::Similarity {
            east: t.x - (a * o.x - b * o.y),
            north: t.y - (b * o.x + a * o.y),
            rotation: atan2(b, a) * (180.0 / PI),
            scale: js_hypot(a, b),
        },
        [a, b, c, d] => CrsPlane::Affine {
            a,
            b: c,
            c: t.x - a * o.x - c * o.y,
            d: b,
            e: d,
            f: t.y - b * o.x - d * o.y,
        },
        _ => return Err(FIT_COLLINEAR.to_owned()),
    };
    Ok(PlaneFit {
        plane,
        rows: at,
        residuals: fit.residuals,
        m0: fit.m0,
    })
}

/// A plane's values as the form's fields write them.
pub fn plane_texts(form: &mut Form, plane: &CrsPlane) {
    let text = |v: f64| format!("{v}");
    match *plane {
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

/// A definition as WKT: WKT 1, a local system WKT 2 over its base, named as
/// the registry or the base's definition names it (docs/adr/0168 §5).
pub fn wkt(d: &CrsDefinition) -> Option<String> {
    let s = definition_system(d)?;
    let base = match &d.system {
        CrsSystem::Local(l) => Some(match (l.base.srid, &l.base.definition) {
            (_, Some(b)) => b.name.clone(),
            (Some(srid), None) => system(srid)?.name.clone(),
            (None, None) => return None,
        }),
        _ => None,
    };
    core_text::write_wkt(&d.name, &s, base.as_deref())
}

/// A definition as a PROJ string; none for a local system (PROJ cannot
/// write one derived from another).
pub fn proj(d: &CrsDefinition) -> Option<String> {
    core_text::write_proj(&definition_system(d)?)
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

    /// A text read as the shared cases say (the web reads the same file,
    /// `wasm/definitionText.wasm.test.ts`): the definition, the registry's
    /// system it is and what its datum shares with the registry, or why not.
    #[test]
    fn texts_read_as_the_shared_cases() {
        let file: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/crs/v1/definition-text.json"
        ))
        .expect("the cases read");
        assert_eq!(file["format"], "kentos.crs-definition-text");
        for (key, text) in [
            ("syntax", READ_SYNTAX),
            ("unsupported", READ_UNSUPPORTED),
            ("unit", READ_UNIT),
            ("meridian", READ_MERIDIAN),
            ("grid", READ_GRID),
            ("base", "{name} tabanı"),
        ] {
            assert_eq!(file["texts"][key], text, "{key}");
        }
        let reads = file["reads"].as_array().expect("reads");
        assert!(reads.len() >= 27);
        for case in reads {
            let name = case["name"].as_str().expect("a name");
            let got = read(case["text"].as_str().expect("a text"));
            match case.get("problem") {
                Some(p) => assert_eq!(got.as_ref().err().map(String::as_str), p.as_str(), "{name}"),
                None => {
                    let got = got.unwrap_or_else(|e| panic!("{name}: {e}"));
                    let want: CrsDefinition =
                        serde_json::from_value(case["definition"].clone()).expect("a definition");
                    assert_eq!(got.definition, want, "{name}");
                    assert_eq!(got.same, case["same"].as_u64().map(|s| s as u32), "{name}");
                    assert_eq!(got.note.as_deref(), case["note"].as_str(), "{name}");
                }
            }
        }
    }

    /// The common points' plane as the shared cases say (the web reads the
    /// same file, `wasm/definitionFit.wasm.test.ts`): within the file's
    /// tolerances, or the same words.
    #[test]
    fn the_common_points_give_the_shared_cases_plane() {
        let file: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/crs/v1/definition-fit.json"
        ))
        .expect("the cases read");
        assert_eq!(file["format"], "kentos.crs-definition-fit");
        for (key, text) in [
            ("skipped", FIT_SKIPPED),
            ("tooFew", FIT_TOO_FEW),
            ("coincident", FIT_COINCIDENT),
            ("collinear", FIT_COLLINEAR),
        ] {
            assert_eq!(file["texts"][key], text, "{key}");
        }
        let tol = &file["tolerance"];
        let (metres, relative, degrees) = (
            tol["metres"].as_f64().expect("metres"),
            tol["relative"].as_f64().expect("relative"),
            tol["degrees"].as_f64().expect("degrees"),
        );
        let near = |got: f64, want: &Value, abs: f64, what: &str| {
            let want = want.as_f64().unwrap_or_else(|| panic!("{what}"));
            assert!(
                (got - want).abs() <= abs.max(relative * want.abs()),
                "{what}: {got} ≠ {want}"
            );
        };
        let cases = file["cases"].as_array().expect("cases");
        assert!(cases.len() >= 13);
        for case in cases {
            let name = case["name"].as_str().expect("a name");
            let kind = match case["plane"].as_str() {
                Some("affine") => PlaneKind::Affine,
                _ => PlaneKind::Similarity,
            };
            let rows: Vec<FitRow> = case["typed"]
                .as_array()
                .expect("rows")
                .iter()
                .map(|r| std::array::from_fn(|i| r[i].as_str().expect("a text").to_owned()))
                .collect();
            let got = fit_plane(&rows, kind);
            if let Some(p) = case.get("problem") {
                assert_eq!(got.err().as_deref(), p.as_str(), "{name}");
                continue;
            }
            let got = got.unwrap_or_else(|e| panic!("{name}: {e}"));
            let want = &case["expect"];
            let p = &want["plane"];
            match got.plane {
                CrsPlane::Similarity {
                    east,
                    north,
                    rotation,
                    scale,
                } => {
                    assert_eq!(p["kind"], "similarity", "{name}");
                    near(east, &p["east"], metres, name);
                    near(north, &p["north"], metres, name);
                    near(rotation, &p["rotation"], degrees, name);
                    near(scale, &p["scale"], 0.0, name);
                }
                CrsPlane::Affine { a, b, c, d, e, f } => {
                    assert_eq!(p["kind"], "affine", "{name}");
                    for (k, v, abs) in [
                        ("a", a, 0.0),
                        ("b", b, 0.0),
                        ("c", c, metres),
                        ("d", d, 0.0),
                        ("e", e, 0.0),
                        ("f", f, metres),
                    ] {
                        near(v, &p[k], abs.max(1e-15), &format!("{name}: {k}"));
                    }
                }
            }
            let rows_want: Vec<usize> = want["rows"]
                .as_array()
                .expect("rows")
                .iter()
                .map(|v| v.as_u64().expect("a row") as usize)
                .collect();
            assert_eq!(got.rows, rows_want, "{name}");
            let skipped: Vec<usize> = want["skipped"]
                .as_array()
                .expect("skipped")
                .iter()
                .map(|v| v.as_u64().expect("a row") as usize)
                .collect();
            assert_eq!(unread_rows(&rows), skipped, "{name}");
            for (g, w) in got
                .residuals
                .iter()
                .zip(want["residuals"].as_array().expect("residuals"))
            {
                for k in 0..3 {
                    near(g[k], &w[k], metres, name);
                }
            }
            match (got.m0, want["m0"].as_f64()) {
                (Some(g), Some(_)) => near(g, &want["m0"], metres, name),
                (g, w) => assert_eq!(g, w, "{name}: m0"),
            }
            // The plane goes into the fields and builds again from them.
            let mut form = Form {
                name: "Ortak".to_owned(),
                kind: Kind::Local,
                base: "5254".to_owned(),
                ..Form::default()
            };
            plane_texts(&mut form, &got.plane);
            let (built, _) = build(&form).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let CrsSystem::Local(l) = built.system else {
                panic!("{name}: a local system");
            };
            assert_eq!(l.plane, got.plane, "{name}: the fields give it back");
        }
    }

    /// A definition copied is the core's text of its system (the cases of
    /// fixtures/geodesy/v1/text.json, which PROJ reads back): WKT 1, a local
    /// system WKT 2 over its base by name; PROJ but for a local system.
    #[test]
    fn a_definition_is_copied_as_the_cores_texts() {
        use kentos_geometry_core::api::json::{FromJson, Json};
        let file: Value =
            serde_json::from_str(include_str!("../../../../fixtures/geodesy/v1/text.json"))
                .expect("the cases read");
        for case in file["writes"].as_array().expect("writes") {
            let w = &case["definition"];
            let system = kentos_geometry_core::crs::System::from_json(
                &Json::parse(&w["system"].to_string()).expect("JSON"),
            )
            .expect("the core reads it");
            let name = w["name"].as_str().expect("a name");
            let mut d = definition_from(name, &system).expect("a definition");
            let want_wkt = case["wkt"].as_str();
            // A base the registry does not have is named as the case names it.
            if let CrsSystem::Local(l) = &mut d.system
                && let Some(b) = l.base.definition.as_mut()
            {
                b.name = want_wkt
                    .and_then(|t| t.split("BASEPROJCRS[\"").nth(1))
                    .and_then(|t| t.split('"').next())
                    .expect("the base's name")
                    .to_owned();
            }
            assert_eq!(wkt(&d).as_deref(), want_wkt, "{}: WKT", case["name"]);
            assert_eq!(
                proj(&d).as_deref(),
                case["proj"].as_str(),
                "{}: PROJ",
                case["name"]
            );
        }
    }
}
