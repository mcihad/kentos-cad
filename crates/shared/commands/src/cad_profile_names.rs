//! The form an institution's mapping spells field names in (docs/adr/5180; plan/map-cad.md MAP-17): a mapping's
//! `names` gives every field the mapping does not name itself a name in lowerCamelCase (the national GIS standard's
//! rule for attribute names: no space, every word after the first starts with a capital, words written wholly in
//! capitals are abbreviations and stay) or in capitals joined by `_` (the way the large-scale map regulation's Ek-1
//! attribute codes are written), with the Turkish letters kept or written in ASCII. A name the mapping writes itself
//! always comes first: the regulation's codes cannot be made from a field's name, they are the user's data.
//!
//! `cad.profiles.names` says, without writing anything, each field's name in the institution, where it comes from and
//! what is wrong with it (a name the form cannot spell, two fields falling on one name).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{DataProfile, ProfileClass, ProfileMapping};

pub const CAD_PROFILES_NAMES: &str = "cad.profiles.names";
pub const CAD_PROFILES_NAMES_VERSION: u32 = 1;

/// How the words of a name are joined and cased.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NameCase {
    /// `kapiNo`: the first word small, every later word with a capital first letter; a word of capitals only (two
    /// letters or more) is an abbreviation and stays as it is, except as the first word.
    LowerCamelCase,
    /// `KAPI_NO`: every word in capitals, joined by `_`.
    UpperSnakeCase,
}

/// What becomes of the Turkish letters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NameLetters {
    /// Kept, cased the Turkish way (i ↔ İ, ı ↔ I).
    #[default]
    Keep,
    /// ç, ğ, ı, İ, ö, ş, ü (and â, î, û) written c, g, i, I, o, s, u; any other letter beyond ASCII cannot be spelled.
    Ascii,
}

/// A mapping's form for the names it does not write itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MappingNames {
    pub case: NameCase,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<NameLetters>", optional))]
    pub letters: NameLetters,
}

/// The form's words in the interface and the messages.
pub fn case_words(case: NameCase) -> &'static str {
    match case {
        NameCase::LowerCamelCase => "lowerCamelCase",
        NameCase::UpperSnakeCase => "BÜYÜK_HARF",
    }
}

/// The words of a name: runs of letters and digits; a capital starts a new word after a small letter or a digit, and
/// after a capital when a small letter follows it (`TUCBSNo`: `TUCBS`, `No`); a digit stays with the word before it.
pub fn name_words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut out = Vec::new();
    let mut word = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            continue;
        }
        if !word.is_empty() && c.is_uppercase() {
            let before = chars[i - 1];
            let small_next = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if before.is_lowercase() || before.is_numeric() || (before.is_uppercase() && small_next) {
                out.push(std::mem::take(&mut word));
            }
        }
        word.push(c);
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

fn ascii_letter(c: char) -> Option<char> {
    Some(match c {
        'ç' => 'c',
        'Ç' => 'C',
        'ğ' => 'g',
        'Ğ' => 'G',
        'ı' | 'î' => 'i',
        'İ' | 'Î' => 'I',
        'ö' => 'o',
        'Ö' => 'O',
        'ş' => 's',
        'Ş' => 'S',
        'ü' | 'û' => 'u',
        'Ü' | 'Û' => 'U',
        'â' => 'a',
        'Â' => 'A',
        c if c.is_ascii() => c,
        _ => return None,
    })
}

fn lower(s: &str, turkish: bool) -> String {
    s.chars()
        .flat_map(|c| match (turkish, c) {
            (true, 'I') => vec!['ı'],
            (true, 'İ') => vec!['i'],
            _ => c.to_lowercase().collect(),
        })
        .collect()
}

fn upper(s: &str, turkish: bool) -> String {
    s.chars()
        .flat_map(|c| match (turkish, c) {
            (true, 'i') => vec!['İ'],
            (true, 'ı') => vec!['I'],
            _ => c.to_uppercase().collect(),
        })
        .collect()
}

/// A word of capitals only, two letters or more: an abbreviation.
fn abbreviation(word: &str) -> bool {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase())
}

/// `name` in the form; why not when it cannot be spelled (no letter or digit, a letter with no ASCII form).
pub fn styled_name(name: &str, names: &MappingNames) -> Result<String, String> {
    let mut words = name_words(name);
    if words.is_empty() {
        return Err("adında harf ya da rakam yok".into());
    }
    let turkish = names.letters == NameLetters::Keep;
    if !turkish {
        for w in &mut words {
            let mut out = String::with_capacity(w.len());
            for c in w.chars() {
                match ascii_letter(c) {
                    Some(a) => out.push(a),
                    None => return Err(format!("“{c}” harfinin ASCII karşılığı yok")),
                }
            }
            *w = out;
        }
    }
    Ok(match names.case {
        NameCase::LowerCamelCase => words
            .iter()
            .enumerate()
            .map(|(k, w)| {
                if k == 0 {
                    lower(w, turkish)
                } else if abbreviation(w) {
                    w.clone()
                } else {
                    let mut cs = w.chars();
                    let first = cs.next().map(|c| upper(&c.to_string(), turkish)).unwrap_or_default();
                    first + &lower(cs.as_str(), turkish)
                }
            })
            .collect(),
        NameCase::UpperSnakeCase => words
            .iter()
            .map(|w| upper(w, turkish))
            .collect::<Vec<_>>()
            .join("_"),
    })
}

/// Where a field's name in the institution comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NameSource {
    /// The profile's own name: no mapping names it and no form is given.
    Profile,
    /// The mapping's form spelled it.
    Form,
    /// The mapping writes it itself (`fields`).
    Mapping,
}

/// What is wrong with a field's name in the institution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NameProblem {
    /// The form cannot spell the name: the mapping must write it.
    Unfit,
    /// Another field of the class falls on the same name.
    Clash,
}

/// A profile field and its name in the institution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileNameRow {
    /// The profile's class id and title.
    pub class: String,
    pub class_title: String,
    /// The profile's field name.
    pub field: String,
    /// The field's name in the institution; empty when the form cannot spell it.
    pub name: String,
    pub source: NameSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<NameProblem>,
    /// The problem in words, with what to do.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub message: Option<String>,
}

/// The name of field `field` of class `c` in the institution by mapping `m` (its own `fields` first, then its form);
/// none: the profile's own.
fn named(
    c: &ProfileClass,
    m: Option<&ProfileMapping>,
    names: Option<&MappingNames>,
    field: &str,
) -> (Result<String, String>, NameSource) {
    let written = m
        .and_then(|m| m.classes.iter().find(|x| x.class == c.id))
        .and_then(|mc| mc.fields.iter().find(|x| x.profile == field));
    match (written, names) {
        (Some(x), _) => (Ok(x.name.clone()), NameSource::Mapping),
        (None, Some(n)) => (styled_name(field, n), NameSource::Form),
        (None, None) => (Ok(field.to_owned()), NameSource::Profile),
    }
}

/// Every field of the profile, class by class in the profile's order, with its name in the institution by mapping `m`
/// read with form `names` (the mapping's own form, or another to preview), and its problem. Two names clash when they
/// are the same folded the Turkish way, as a layer's fields are told apart.
pub fn name_rows(
    p: &DataProfile,
    m: Option<&ProfileMapping>,
    names: Option<&MappingNames>,
) -> Vec<ProfileNameRow> {
    let mut rows = Vec::new();
    for c in &p.classes {
        let start = rows.len();
        for f in &c.fields {
            let (name, source) = named(c, m, names, &f.name);
            let (name, problem, message) = match name {
                Ok(n) => (n, None, None),
                Err(why) => (
                    String::new(),
                    Some(NameProblem::Unfit),
                    Some(format!(
                        "“{}” alanının adı {} biçiminde yazılamıyor: {why}; kurumdaki adını eşlemede yazın.",
                        f.name,
                        case_words(names.map_or(NameCase::LowerCamelCase, |n| n.case))
                    )),
                ),
            };
            rows.push(ProfileNameRow {
                class: c.id.clone(),
                class_title: c.title.clone(),
                field: f.name.clone(),
                name,
                source,
                problem,
                message,
            });
        }
        // Two fields of the class on one name, as a layer compares its fields' names (folded the Turkish way): each of
        // them said.
        let class_rows = &rows[start..];
        let folded: Vec<String> = class_rows.iter().map(|r| crate::fold(&r.name)).collect();
        let clashing: Vec<usize> = (0..class_rows.len())
            .filter(|&i| {
                class_rows[i].problem.is_none()
                    && class_rows.iter().enumerate().any(|(j, o)| {
                        j != i && o.problem != Some(NameProblem::Unfit) && folded[j] == folded[i]
                    })
            })
            .collect();
        for i in clashing {
            let formed = rows[start..]
                .iter()
                .enumerate()
                .any(|(j, o)| folded[j] == folded[i] && o.source == NameSource::Form);
            let r = &mut rows[start + i];
            r.problem = Some(NameProblem::Clash);
            r.message = Some(if formed {
                format!(
                    "“{}” adı “{}” sınıfında başka bir alanın adı olurdu; birinin kurumdaki adını eşlemede yazın.",
                    r.name, c.title
                )
            } else {
                format!(
                    "“{}” adı “{}” sınıfında başka bir alanın adı olurdu.",
                    r.name, c.title
                )
            });
        }
    }
    rows
}

/// What is wrong with the names mapping `m` gives `p`'s fields: the first problem in the profile's order; none when
/// every field has a name of its own in its class.
pub fn names_problem(m: &ProfileMapping, p: &DataProfile) -> Option<String> {
    name_rows(p, Some(m), m.names.as_ref())
        .into_iter()
        .find_map(|r| r.message)
}

/// The name of field `field` of class `class` in the institution by mapping `m`, which holds for the profile.
pub fn institution_name(p: &DataProfile, m: &ProfileMapping, class: &str, field: &str) -> String {
    let Some(c) = p.classes.iter().find(|c| c.id == class) else {
        return field.to_owned();
    };
    named(c, Some(m), m.names.as_ref(), field)
        .0
        .unwrap_or_else(|_| field.to_owned())
}

/// Input of `cad.profiles.names` v1 (“Kurumdaki alan adları”): every field of the profile with its name in the
/// institution, where it comes from (the profile, the mapping's form, the mapping itself) and what is wrong with it; it
/// writes nothing. `names` previews another form than the mapping's own (without a mapping, the form alone). Refusals:
/// the profile's (`unknown_profile`, `invalid_profile`) and the mapping's (`unknown_mapping`, `invalid_profile_mapping`
/// for anything but its names: those are said row by row).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesNames {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<DataProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping: Option<ProfileMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping_name: Option<String>,
    /// The form to read the names with instead of the mapping's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub names: Option<MappingNames>,
    /// The open drawing's own file, an absolute path: the project's `profiller` folder is beside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// Output of `cad.profiles.names`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileNames {
    /// The profile's name.
    pub profile: String,
    /// The mapping's name, when one is read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping: Option<String>,
    /// The form the names were read with, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub names: Option<MappingNames>,
    pub rows: Vec<ProfileNameRow>,
    /// Fields whose name in the institution is not the profile's, and that have no problem.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub renamed: u32,
    /// Fields the form cannot spell.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unfit: u32,
    /// Fields that fall on another field's name.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub clashes: u32,
    /// Whether every field has a name of its own: the mapping with this form holds.
    pub holds: bool,
}

/// The output for rows read with `names`.
pub fn profile_names(
    p: &DataProfile,
    mapping: Option<String>,
    names: Option<MappingNames>,
    rows: Vec<ProfileNameRow>,
) -> ProfileNames {
    let count = |want: NameProblem| rows.iter().filter(|r| r.problem == Some(want)).count() as u32;
    let unfit = count(NameProblem::Unfit);
    let clashes = count(NameProblem::Clash);
    let renamed = rows
        .iter()
        .filter(|r| r.problem.is_none() && r.name != r.field)
        .count() as u32;
    ProfileNames {
        profile: p.name.clone(),
        mapping,
        names,
        renamed,
        unfit,
        clashes,
        holds: unfit == 0 && clashes == 0,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(case: NameCase, letters: NameLetters) -> MappingNames {
        MappingNames { case, letters }
    }

    #[test]
    fn the_forms_spell_names_as_the_texts_write_them() {
        let camel = n(NameCase::LowerCamelCase, NameLetters::Ascii);
        let snake = n(NameCase::UpperSnakeCase, NameLetters::Ascii);
        // The standard's theme documents: tucbsNo, araziKullanimiDurumu5000.
        assert_eq!(styled_name("TUCBS No", &camel).unwrap(), "tucbsNo");
        assert_eq!(styled_name("Arazi Kullanımı Durumu 5000", &camel).unwrap(), "araziKullanimiDurumu5000");
        assert_eq!(styled_name("Yasal bağlayıcılık durumu", &camel).unwrap(), "yasalBaglayicilikDurumu");
        assert_eq!(styled_name("sYLKK", &camel).unwrap(), "sYLKK");
        // The regulation's Ek-1 codes: OLCU_YONTEMI, ADRES_1_IL, NITELIK_01.
        assert_eq!(styled_name("Ölçü yöntemi", &snake).unwrap(), "OLCU_YONTEMI");
        assert_eq!(styled_name("Adres 1 il", &snake).unwrap(), "ADRES_1_IL");
        assert_eq!(styled_name("Nitelik 01", &snake).unwrap(), "NITELIK_01");
        assert_eq!(styled_name("KapiNo", &snake).unwrap(), "KAPI_NO");
        // Turkish letters kept, cased the Turkish way.
        let keep = n(NameCase::UpperSnakeCase, NameLetters::Keep);
        assert_eq!(styled_name("KapiNo", &keep).unwrap(), "KAPİ_NO");
        assert_eq!(
            styled_name("Işık şiddeti", &n(NameCase::LowerCamelCase, NameLetters::Keep)).unwrap(),
            "ışıkŞiddeti"
        );
        assert!(styled_name("--", &camel).is_err());
        assert!(styled_name("Café", &camel).is_err());
        assert_eq!(styled_name("Café", &n(NameCase::LowerCamelCase, NameLetters::Keep)).unwrap(), "café");
    }
}
