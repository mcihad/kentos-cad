//! Yangın, performans ve malzeme bilgisi (docs/adr/8068; plan/bim-plan.md BIM-07; Dijital Proje Yön. EK-6 Tablo 6.3,
//! 6.7, 6.16, 6.25, 6.52, 6.56, 6.61, 6.66): what the author declares and the IFC model carries. A value is never a
//! default: what is not declared is absent and counted missing (a file never says an empty word for it).
//!
//! - **A declared word** ([`Declared`]): a fire resistance class (`FireRating`: as the national fire regulation reads it,
//!   TS EN 13501-2's "REI 120", "EI 60"), a building's fire protection class (`FireProtectionClass`). Its words are the
//!   author's; whether they read as the standard's is the standards package's rule (bim-plan K10), not this check.
//! - **An exit role** ([`DeclaredExit`]): whether a door or a stair is an emergency exit (`FireExit`); unknown is absent.
//! - **Its basis** ([`DeclaredBasis`]): the author's statement, or a document (its name and date). What KentOS derives (outside,
//!   load bearing, the construction action from the künye) is never stored: it is worked out where it is written.
//! - **Where:** an object's own value on it (`arch.fireRating`, an opening's `fireRating`), its type's on the type (a
//!   wall style's, a door or window type's); the object's own wins. A material's strength (`Beton_C25`'s `C25`) on the
//!   material; the building's on the project's architecture settings (`performance`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The longest declared word, in characters (an IFC label is 255; a class is a few).
pub const MAX_DECLARED: usize = 64;
/// The longest document name a basis gives.
pub const MAX_BASIS_NAME: usize = 128;

/// Where a declared value comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum DeclaredBasis {
    /// Müellif beyanı: the author's own statement.
    Author,
    /// A document: its name (a test report, a product's declaration of performance) and, when given, its date.
    Document {
        name: String,
        /// `YYYY-MM-DD`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(regex(pattern = r"^\d{4}-\d{2}-\d{2}$")))]
        date: Option<String>,
    },
}

impl DeclaredBasis {
    /// Its words: “müellif beyanı”, or the document's name and date.
    pub fn words(&self) -> String {
        match self {
            DeclaredBasis::Author => "müellif beyanı".to_owned(),
            DeclaredBasis::Document {
                name,
                date: Some(d),
            } => format!("{name} ({d})"),
            DeclaredBasis::Document { name, date: None } => name.clone(),
        }
    }

    /// A basis as one line gives it (docs/adr/8124: Öznitelikler, the type and the settings windows), the way
    /// [`DeclaredBasis::words`] writes it: empty or “müellif beyanı” the author's statement; else a document's name, its
    /// day after it in brackets when given (`Test raporu 41 (2026-02-28)`). What is wrong with it, refused.
    pub fn read(text: &str) -> Result<DeclaredBasis, String> {
        let t = text.trim();
        if t.is_empty() || t.to_lowercase() == "müellif beyanı" {
            return Ok(DeclaredBasis::Author);
        }
        // A day in brackets at its end: `(YYYY-AA-GG)`; brackets holding other words are the name's.
        let dated = t
            .strip_suffix(')')
            .and_then(|r| r.rsplit_once('('))
            .filter(|(_, d)| {
                let b = d.trim().as_bytes();
                b.len() == 10 && b[4] == b'-' && b[7] == b'-'
            });
        let basis = match dated {
            Some((name, day)) => DeclaredBasis::Document {
                name: name.trim().to_owned(),
                date: Some(day.trim().to_owned()),
            },
            None => DeclaredBasis::Document {
                name: t.to_owned(),
                date: None,
            },
        };
        match basis.problem() {
            Some(why) => Err(why),
            None => Ok(basis),
        }
    }

    /// Its line as a field shows it: empty for the author's statement, else [`DeclaredBasis::words`].
    pub fn field(&self) -> String {
        match self {
            DeclaredBasis::Author => String::new(),
            _ => self.words(),
        }
    }

    /// What is wrong with it: a document without a name, a name too long, a date that is no day.
    pub fn problem(&self) -> Option<String> {
        let DeclaredBasis::Document { name, date } = self else {
            return None;
        };
        if name.trim().is_empty() {
            return Some(
                "Dayanak belgesinin adı boş: belgenin adını yazın ya da müellif beyanı seçin."
                    .into(),
            );
        }
        if name.chars().count() > MAX_BASIS_NAME || name.chars().any(char::is_control) {
            return Some(format!(
                "Dayanak belgesinin adı en çok {MAX_BASIS_NAME} karakter olur ve satır sonu içermez."
            ));
        }
        if let Some(d) = date
            && !is_day(d)
        {
            return Some(format!(
                "“{d}” bir gün değil: tarihi YYYY-AA-GG biçiminde yazın (2026-11-16)."
            ));
        }
        None
    }
}

/// Whether `s` is a day `YYYY-MM-DD` of the Gregorian calendar.
fn is_day(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| {
        s.get(r)
            .filter(|t| t.bytes().all(|c| c.is_ascii_digit()))?
            .parse::<u32>()
            .ok()
    };
    let (Some(y), Some(m), Some(d)) = (num(0..4), num(5..7), num(8..10)) else {
        return false;
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&d)
}

/// A declared word and its basis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Declared {
    pub value: String,
    pub basis: DeclaredBasis,
}

impl Declared {
    /// The author's word.
    pub fn author(value: &str) -> Declared {
        Declared {
            value: value.to_owned(),
            basis: DeclaredBasis::Author,
        }
    }

    /// What is wrong with it, `what` naming it (“yangın dayanım sınıfı”): an empty word (a value never says nothing),
    /// one too long or with a line break, a bad basis.
    pub fn problem(&self, what: &str) -> Option<String> {
        let v = self.value.trim();
        if v.is_empty() {
            return Some(format!(
                "{} boş olamaz: bilinmiyorsa silin (null); boş değer dosyaya yazılmaz, eksik sayılır.",
                capital(what)
            ));
        }
        if v != self.value {
            return Some(format!(
                "{} başında ya da sonunda boşluk var: “{}” yazın.",
                capital(what),
                v
            ));
        }
        if self.value.chars().count() > MAX_DECLARED || self.value.chars().any(char::is_control) {
            return Some(format!(
                "{} en çok {MAX_DECLARED} karakter olur ve satır sonu içermez.",
                capital(what)
            ));
        }
        self.basis.problem()
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// A declared emergency exit role and its basis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredExit {
    /// Evet: it serves as an exit in a fire; hayır: it does not.
    pub exit: bool,
    pub basis: DeclaredBasis,
}

/// The largest area a lift shaft's vent is declared with, m².
pub const MAX_SHAFT_VENT: f64 = 100.0;

/// A lift shaft's declared ventilation and smoke vent, or its pressurisation, and its basis (docs/adr/8135; BYKHY 62/4:
/// "kuyu alanının 0.025 katı kadar bir havalandırma ve dumandan arındırma bacası bulundurulur veya kuyular
/// basınçlandırılır"): one of the shaft's rooms says it for the whole shaft.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredShaftVent {
    /// The vent's free area, m²; absent when the shaft is pressurised instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: Option<f64>,
    /// The shaft is pressurised (62/4's other way).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub pressurized: bool,
    pub basis: DeclaredBasis,
}

impl DeclaredShaftVent {
    /// What is wrong with it, its words and the field: neither an area nor pressurised; an area not finite, not over 0
    /// or over [`MAX_SHAFT_VENT`]; its basis.
    pub fn problem(&self) -> Option<(String, &'static str)> {
        match self.area {
            None if !self.pressurized => {
                return Some((
                    "Kuyunun havalandırma bacasının alanı ya da basınçlandırıldığı verilmedi: area (m²) ya da \
                     pressurized: true verin."
                        .into(),
                    "area",
                ));
            }
            Some(a) if !(a.is_finite() && a > 0.0 && a <= MAX_SHAFT_VENT) => {
                return Some((
                    format!(
                        "Havalandırma bacasının alanı 0'dan büyük ve en çok {MAX_SHAFT_VENT} m² olur."
                    ),
                    "area",
                ));
            }
            _ => {}
        }
        self.basis.problem().map(|why| (why, "basis"))
    }
}

/// A declared load-bearing role and its basis (docs/adr/8090; EK-6 `Pset_WallCommon.LoadBearing`, BYKHY Ek-3/B 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredBearing {
    /// Evet: it carries the structure's loads (a load-bearing wall); hayır: it does not (a partition).
    pub bearing: bool,
    pub basis: DeclaredBasis,
}

/// A declared exterior role and its basis (docs/adr/8124; EK-6 `IsExternal` of a wall, a slab): what changes the
/// drawing's own reading, a wall on its storey's outline or a balcony's or a terrace's slab being outside.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredExternal {
    /// Evet: it faces the outside (an outer wall, a slab over open air); hayır: it does not.
    pub external: bool,
    pub basis: DeclaredBasis,
}

/// A shaft room's declared protected (korunumlu) role and its basis (docs/adr/8136; BYKHY Ek-3/B 8: "Korunumlu Şaftlar"
/// REI 120, each side): the regulation does not define a protected shaft, the designer says which is. One of the
/// shaft's rooms says it for the whole shaft through the storeys.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredProtected {
    /// Evet: the shaft is protected; hayır: it is not.
    pub protected: bool,
    pub basis: DeclaredBasis,
}

/// A wall's or a slab's declared fire stop (yangın kesici) role and its basis (docs/adr/8136; BYKHY 4/pp: an element,
/// horizontal or vertical, stopping fire and smoke spreading for a set time; Ek-3/B 10: EI 30, each side).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredFireStop {
    /// Evet: it is a fire stop; hayır: it is not.
    pub fire_stop: bool,
    pub basis: DeclaredBasis,
}

/// The building's declared particulars for the fire regulation (EK-6 Tablo 6.7): its fire protection class. The
/// construction action (`ConstructionMethod`: new construction, addition, renovation) is the künye's building kind,
/// worked out where it is written.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildingPerformance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_protection_class: Option<Declared>,
}

impl BuildingPerformance {
    pub fn is_empty(&self) -> bool {
        self.fire_protection_class.is_none()
    }
}

/// What is wrong with a reaction to fire class (docs/adr/8091; TS EN 13501-1 as BYKHY Ek-2/Ç writes it): its words, or
/// none. A class letter (A1, A2, B, C, D, E, F), `fl` for a floor covering, then its smoke (`-s1` … `-s3`) and its
/// droplets (`,d0` … `,d2`, or `-d2` with no smoke), as written: “A1”, “A2-s1,d0”, “E-d2”, “Bfl-s1”.
pub fn reaction_problem(s: &str) -> Option<String> {
    let bad = || {
        Some(format!(
            "“{s}” bir TS EN 13501-1 yanıcılık sınıfı değil: A1, A2-s1,d0, B-s2,d0, E, E-d2, Bfl-s1 gibi yazın (Ek-2/Ç)."
        ))
    };
    let t = s.trim();
    if t != s || t.is_empty() {
        return bad();
    }
    let (class, rest) = if let Some(r) = t.strip_prefix("A1").or_else(|| t.strip_prefix("A2")) {
        (&t[..2], r)
    } else {
        match t.chars().next() {
            Some(c @ ('B' | 'C' | 'D' | 'E' | 'F')) => (&t[..c.len_utf8()], &t[c.len_utf8()..]),
            _ => return bad(),
        }
    };
    let (floor, rest) = match rest.strip_prefix("fl") {
        Some(r) => (true, r),
        None => (false, rest),
    };
    // Smoke, then droplets: a floor's class has no droplets; A1 and F have neither.
    let (smoke, rest) = match rest.strip_prefix("-s") {
        Some(r) if matches!(r.chars().next(), Some('1'..='3')) => (true, &r[1..]),
        Some(_) => return bad(),
        None => (false, rest),
    };
    // Droplets after the smoke with a comma (“B-s1,d0”), alone with a dash (“E-d2”); a floor's class has none.
    let droplets = match rest {
        "" => false,
        r if !floor
            && (if smoke {
                r.strip_prefix(",d")
            } else {
                r.strip_prefix("-d")
            })
            .is_some_and(|d| matches!(d, "0" | "1" | "2")) =>
        {
            true
        }
        _ => return bad(),
    };
    if matches!(class, "A1" | "F") && (smoke || droplets) {
        return bad();
    }
    None
}

/// What is wrong with a roof covering's external fire performance class (docs/adr/8134; TS EN 13501-5 as BYKHY 28/2 and
/// Ek-2/Ç write it): its words, or none. A class letter (B, C, D, E, F), `ROOF`, then the test method in parentheses
/// (`(t1)` … `(t4)`) or none, as Ek-2/Ç writes BROOF alone: “BROOF”, “BROOF(t4)”, “EROOF(t3)”. Which letters each method
/// gives is the standard's; it is not checked here.
pub fn roof_class_problem(s: &str) -> Option<String> {
    let ok = s.len() >= 5
        && matches!(s.as_bytes()[0], b'B' | b'C' | b'D' | b'E' | b'F')
        && s[1..].starts_with("ROOF")
        && matches!(&s[5..], "" | "(t1)" | "(t2)" | "(t3)" | "(t4)");
    (!ok).then(|| {
        format!("“{s}” bir TS EN 13501-5 dış yangın sınıfı değil: BROOF, BROOF(t1) … BROOF(t4), CROOF(t3), FROOF(t4) gibi yazın (Ek-2/Ç).")
    })
}

/// What is wrong with a material's strength (EK-6: `Malzeme İsmi_Dayanım`, “Beton_C25”, “Çelik_S355”): its words, or
/// none. Letters, digits and `/ . , -`, at most 16 (C25/30, S355, B420C); no `_`, which joins it to the name.
pub fn strength_problem(s: &str) -> Option<String> {
    if s.is_empty() {
        return Some("Malzemenin dayanımı boş olamaz: bilinmiyorsa silin (null).".into());
    }
    if s.chars().count() > 16
        || !s
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | ',' | '-'))
    {
        return Some(format!(
            "“{s}” bir dayanım sınıfı gibi yazılmamış: harf, rakam ve / . , - ile en çok 16 karakter yazın (C25, C30/37, S355)."
        ));
    }
    None
}

/// A material's name as EK-6 asks it written: `Ad_Dayanım` (“Beton_C25”), its name alone without a strength.
pub fn material_label(name: &str, strength: Option<&str>) -> String {
    match strength {
        Some(s) if !s.is_empty() => format!("{name}_{s}"),
        _ => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declared_word_is_never_empty_and_its_basis_reads() {
        assert!(
            Declared::author("REI 120")
                .problem("yangın dayanım sınıfı")
                .is_none()
        );
        assert!(
            Declared::author("")
                .problem("yangın dayanım sınıfı")
                .unwrap()
                .starts_with("Yangın dayanım sınıfı boş")
        );
        assert!(
            Declared::author(" EI 60")
                .problem("x")
                .unwrap()
                .contains("“EI 60”")
        );
        assert!(Declared::author("a\nb").problem("x").is_some());
        let doc = |name: &str, date: Option<&str>| Declared {
            value: "EI 60".into(),
            basis: DeclaredBasis::Document {
                name: name.into(),
                date: date.map(str::to_owned),
            },
        };
        assert!(
            doc("Test raporu 41", Some("2026-02-28"))
                .problem("x")
                .is_none()
        );
        assert!(
            doc("Test raporu 41", Some("2025-02-29"))
                .problem("x")
                .unwrap()
                .contains("bir gün değil")
        );
        assert!(
            doc("Test raporu 41", Some("2024-02-29"))
                .problem("x")
                .is_none()
        );
        assert!(doc("  ", None).problem("x").unwrap().contains("adı boş"));
        assert_eq!(
            doc("Rapor", Some("2026-01-05")).basis.words(),
            "Rapor (2026-01-05)"
        );
        assert_eq!(DeclaredBasis::Author.words(), "müellif beyanı");
    }

    /// One line reads back as its words write it (docs/adr/8124); a bad day or name is refused.
    #[test]
    fn a_basis_reads_from_one_line() {
        assert_eq!(DeclaredBasis::read("").unwrap(), DeclaredBasis::Author);
        assert_eq!(
            DeclaredBasis::read("  Müellif beyanı ").unwrap(),
            DeclaredBasis::Author
        );
        let doc = DeclaredBasis::read("Test raporu 41 (2026-02-28)").unwrap();
        assert_eq!(
            doc,
            DeclaredBasis::Document {
                name: "Test raporu 41".into(),
                date: Some("2026-02-28".into())
            }
        );
        assert_eq!(DeclaredBasis::read(&doc.words()).unwrap(), doc);
        assert_eq!(doc.field(), "Test raporu 41 (2026-02-28)");
        assert_eq!(DeclaredBasis::Author.field(), "");
        // Brackets of other words are the name's.
        assert_eq!(
            DeclaredBasis::read("Ürün beyanı (ek A)").unwrap(),
            DeclaredBasis::Document {
                name: "Ürün beyanı (ek A)".into(),
                date: None
            }
        );
        assert!(
            DeclaredBasis::read("Rapor (2025-02-29)")
                .unwrap_err()
                .contains("bir gün değil")
        );
        assert!(
            DeclaredBasis::read("(2026-01-05)")
                .unwrap_err()
                .contains("adı boş")
        );
    }

    #[test]
    fn a_strength_joins_its_material_s_name() {
        for ok in ["C25", "C30/37", "S355", "B420C", "C25,5"] {
            assert!(strength_problem(ok).is_none(), "{ok}");
        }
        for bad in ["", "C 25", "C_25", "çok-uzun-bir-dayanım-sınıfı"] {
            assert!(strength_problem(bad).is_some(), "{bad}");
        }
        assert_eq!(material_label("Beton", Some("C25")), "Beton_C25");
        assert_eq!(material_label("Tuğla", None), "Tuğla");
    }

    /// Every class of BYKHY Ek-2/Ç's table reads (docs/adr/8091); others do not.
    #[test]
    fn a_reaction_class_is_written_as_ek_2_c_writes_it() {
        for ok in [
            "A1", "A2-s1,d0", "B-s1,d0", "C-s1,d0", "A2-s2,d0", "C-s3,d0", "B-s1,d1", "A2-s1,d2",
            "C-s3,d2", "D-s1,d0", "D-s3,d2", "E", "E-d2", "F", "A1fl", "A2fl-s1", "Bfl-s1",
            "Cfl-s1", "Cfl-s2", "Dfl-s2", "Efl", "Ffl",
        ] {
            assert!(reaction_problem(ok).is_none(), "{ok}");
        }
        for bad in [
            "",
            " A1",
            "A3",
            "G",
            "B-s4,d0",
            "B-s1,d3",
            "A1-s1,d0",
            "F-d2",
            "Bfl-s1,d0",
            "B s1 d0",
            "b-s1,d0",
            "Bfl-d2",
            "B,d0",
            "B-s1-d0",
        ] {
            assert!(reaction_problem(bad).is_some(), "{bad}");
        }
    }

    /// TS EN 13501-5 as BYKHY 28/2 and Ek-2/Ç write it (docs/adr/8134): a letter B–F, ROOF, the test method or none.
    #[test]
    fn a_roof_class_is_a_letter_roof_and_its_test_method() {
        for ok in [
            "BROOF",
            "BROOF(t1)",
            "BROOF(t2)",
            "BROOF(t3)",
            "BROOF(t4)",
            "CROOF(t3)",
            "DROOF(t4)",
            "EROOF(t3)",
            "FROOF(t4)",
        ] {
            assert!(roof_class_problem(ok).is_none(), "{ok}");
        }
        for bad in [
            "",
            "B",
            "AROOF",
            "GROOF",
            "Broof",
            "BROOF(t5)",
            "BROOF(T1)",
            "BROOF (t1)",
            " BROOF",
            "BROOF(t1",
            "B-s1,d0",
        ] {
            assert!(roof_class_problem(bad).is_some(), "{bad}");
        }
    }

    #[test]
    fn the_basis_is_tagged_by_its_kind() {
        let d = Declared {
            value: "REI 90".into(),
            basis: DeclaredBasis::Document {
                name: "Ürün performans beyanı".into(),
                date: None,
            },
        };
        let j = serde_json::to_value(&d).unwrap();
        assert_eq!(
            j,
            serde_json::json!({"value": "REI 90", "basis": {"kind": "document", "name": "Ürün performans beyanı"}})
        );
        assert_eq!(serde_json::from_value::<Declared>(j).unwrap(), d);
        let a: Declared = serde_json::from_value(
            serde_json::json!({"value": "EI 30", "basis": {"kind": "author"}}),
        )
        .unwrap();
        assert_eq!(a.basis, DeclaredBasis::Author);
    }
}
