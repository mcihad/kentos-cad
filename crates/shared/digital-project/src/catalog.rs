//! The base profile's EK-1…EK-4 and EK-8 rows (data/temel-2026.json, written by scripts/mevzuat/katalog.py from the
//! annexes' transcription; docs/adr/8062): the clauses with their words, the name templates (EK-2 Tablo 2.1, EK-3
//! Tablo 3.1, EK-4 Tablo 4.1–4.6), the code lists (EK-2 Tablo 2.2–2.14, EK-3 Tablo 3.2), EK-4's fonts (Tablo 4.7) and
//! its plan marks' sizes (Şekil 4.7). Each row as printed, with its table, row and page: a code the source spells oddly
//! keeps its spelling, the decisions settle it.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::source::Source;

/// EK-2 Tablo 2.2's disciplines (MM, IM, ST …).
pub const DISCIPLINES: &str = "discipline";
/// EK-2 Tablo 2.12's file kinds (PL, KS, MD, FR …).
pub const FILE_KINDS: &str = "fileKind";
/// EK-2 Tablo 2.13's file types (MMC, MKF, XXX …), each in its discipline's group.
pub const FILE_TYPES: &str = "fileType";
/// EK-2 Tablo 2.14's storeys (B01, K00, K01, NNN …); its rows of dots stand for the series around them.
pub const STOREYS: &str = "storey";
/// EK-3 Tablo 3.2's rooms and their three-letter abbreviations.
pub const ROOMS: &str = "room";

/// A discipline's items or sub-disciplines (EK-2 Tablo 2.3–2.11): `items("MM")` is Tablo 2.3's list.
pub fn items_list(discipline: &str) -> Option<&'static str> {
    Some(match discipline {
        "MM" => "item:MM",
        "IM" => "item:IM",
        "HR" => "item:HR",
        "ZE" => "item:ZE",
        "ST" => "subdiscipline:ST",
        "MK" => "subdiscipline:MK",
        "EE" => "subdiscipline:EE",
        "AL" => "subdiscipline:AL",
        "PY" => "subdiscipline:PY",
        _ => return None,
    })
}

/// The profile's name, version and source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub act: String,
    pub gazette: String,
    pub annexes_pdf: Pdf,
    pub transcription: String,
}

/// The annexes PDF the transcription was read from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pdf {
    pub pages: u32,
    pub bytes: u64,
    pub sha256: String,
}

/// A clause of an annex, with its words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clause {
    pub id: String,
    pub annex: u8,
    pub clause: String,
    pub pages: Vec<u32>,
    pub text: String,
}

/// A name template's field: its name, length, kind and whether the template asks it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub no: u32,
    pub name: String,
    pub length: Option<String>,
    pub kind: Option<String>,
    pub required: Option<bool>,
}

/// A name template (EK-2 Tablo 2.1, EK-3 Tablo 3.1, EK-4 Tablo 4.1–4.6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub table: String,
    pub title: String,
    pub page: u32,
    pub fields: Vec<Field>,
}

/// A code of a list: its name, its code, the code cell's note, the group the table puts it in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Code {
    pub id: String,
    pub list: String,
    pub table: String,
    pub row: u32,
    pub page: u32,
    pub name: String,
    pub code: String,
    pub note: Option<String>,
    pub group: Option<String>,
}

/// A row of dots in a code list: the series goes on (EK-2 Tablo 2.14: … B03, K04 …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Series {
    pub series: bool,
    pub table: String,
    pub page: u32,
    pub after: Option<String>,
}

/// EK-4 Tablo 4.7: a font and the point sizes it may have (for information).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Font {
    pub id: String,
    pub table: String,
    pub row: u32,
    pub page: u32,
    pub font: String,
    pub points: String,
}

/// EK-4 Şekil 4.7: a plan mark's measure and its size at each scale, millimetres (for information).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mark {
    pub id: String,
    pub figure: String,
    pub row: u32,
    pub page: u32,
    pub mark: String,
    pub measure: String,
    pub sizes: BTreeMap<String, String>,
}

/// A figure's caption (EK-4 Şekil 4.1–4.7).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    pub id: String,
    pub figure: String,
    pub page: u32,
    pub title: String,
}

/// EK-1 Tablo 1.1: the project's information the administration keeps for its code.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectInformation {
    pub id: String,
    pub table: String,
    pub title: String,
    pub page: u32,
    pub fields: Vec<String>,
}

/// The base profile's EK-1…EK-4 and EK-8.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub format: u32,
    pub profile: ProfileInfo,
    pub clauses: Vec<Clause>,
    pub project_information: Option<ProjectInformation>,
    pub templates: Vec<Template>,
    pub codes: Vec<Code>,
    pub series: Vec<Series>,
    pub fonts: Vec<Font>,
    pub marks: Vec<Mark>,
    pub figures: Vec<Figure>,
}

/// The base profile's text, as the generator wrote it.
pub const BASE_JSON: &str = include_str!("../data/temel-2026.json");

/// The base profile, read once. The text is KentOS's own (a test reads it whole); were it ever unreadable, the profile
/// is empty and every rule unknown, never guessed.
pub fn base() -> &'static Catalog {
    static BASE: OnceLock<Catalog> = OnceLock::new();
    BASE.get_or_init(|| serde_json::from_str(BASE_JSON).unwrap_or_else(|_| Catalog::empty()))
}

/// Any row of the catalog, as the rules window lists it and a decision names it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Row<'a> {
    Clause(&'a Clause),
    Template(&'a Template),
    Code(&'a Code),
    Font(&'a Font),
    Mark(&'a Mark),
    Figure(&'a Figure),
    ProjectInformation(&'a ProjectInformation),
}

impl Row<'_> {
    pub fn id(&self) -> &str {
        match self {
            Row::Clause(c) => &c.id,
            Row::Template(t) => &t.id,
            Row::Code(c) => &c.id,
            Row::Font(f) => &f.id,
            Row::Mark(m) => &m.id,
            Row::Figure(f) => &f.id,
            Row::ProjectInformation(p) => &p.id,
        }
    }

    /// Where it was read.
    pub fn source(&self) -> Source {
        match self {
            Row::Clause(c) => Source::clause(c.annex, &c.clause, &c.pages),
            Row::Template(t) => Source::table(annex_of(&t.table), &t.table, None, t.page),
            Row::Code(c) => Source::table(annex_of(&c.table), &c.table, Some(c.row), c.page),
            Row::Font(f) => Source::table(4, &f.table, Some(f.row), f.page),
            Row::Mark(m) => Source {
                annex: 4,
                figure: Some(m.figure.clone()),
                row: Some(m.row),
                pages: vec![m.page],
                ..Source::default()
            },
            Row::Figure(f) => Source {
                annex: 4,
                figure: Some(f.figure.clone()),
                pages: vec![f.page],
                ..Source::default()
            },
            Row::ProjectInformation(p) => Source::table(1, &p.table, None, p.page),
        }
    }

    /// Its short name: a code's “MM — Mimari”, a clause's number, a template's title.
    pub fn title(&self) -> String {
        match self {
            Row::Clause(c) => format!("Madde {}", c.clause),
            Row::Template(t) => t.title.clone(),
            Row::Code(c) => format!("{} — {}", c.code, c.name),
            Row::Font(f) => f.font.clone(),
            Row::Mark(m) => format!("{} ({})", m.mark, m.measure),
            Row::Figure(f) => f.title.clone(),
            Row::ProjectInformation(p) => p.title.clone(),
        }
    }

    /// Its words: a clause's text, a template's fields, a code's note and group.
    pub fn text(&self) -> String {
        match self {
            Row::Clause(c) => c.text.clone(),
            Row::Template(t) => t
                .fields
                .iter()
                .map(|f| {
                    let mut s = f.name.clone();
                    if let Some(l) = &f.length {
                        s.push_str(&format!(" ({l}"));
                        if let Some(k) = &f.kind {
                            s.push_str(&format!(", {k}"));
                        }
                        s.push(')');
                    }
                    match f.required {
                        Some(true) => s.push_str(", zorunlu"),
                        Some(false) => s.push_str(", isteğe bağlı"),
                        None => {}
                    }
                    s
                })
                .collect::<Vec<_>>()
                .join(" _ "),
            Row::Code(c) => {
                let mut s = c.name.clone();
                if let Some(n) = &c.note {
                    s.push_str(&format!(" ({n})"));
                }
                if let Some(g) = &c.group {
                    s.push_str(&format!(" · {g}"));
                }
                s
            }
            Row::Font(f) => format!("{} punto", f.points),
            Row::Mark(m) => m
                .sizes
                .iter()
                .map(|(scale, size)| format!("{scale}: {size}"))
                .collect::<Vec<_>>()
                .join(", "),
            Row::Figure(f) => f.title.clone(),
            Row::ProjectInformation(p) => p.fields.join(", "),
        }
    }
}

fn annex_of(table: &str) -> u8 {
    table
        .split('.')
        .next()
        .and_then(|a| a.parse().ok())
        .unwrap_or(0)
}

impl Catalog {
    fn empty() -> Self {
        Self {
            format: 0,
            profile: ProfileInfo {
                id: String::new(),
                name: String::new(),
                version: String::new(),
                act: String::new(),
                gazette: String::new(),
                annexes_pdf: Pdf {
                    pages: 0,
                    bytes: 0,
                    sha256: String::new(),
                },
                transcription: String::new(),
            },
            clauses: Vec::new(),
            project_information: None,
            templates: Vec::new(),
            codes: Vec::new(),
            series: Vec::new(),
            fonts: Vec::new(),
            marks: Vec::new(),
            figures: Vec::new(),
        }
    }

    /// A list's codes in printed order.
    pub fn list<'a>(&'a self, list: &'a str) -> impl Iterator<Item = &'a Code> + 'a {
        self.codes.iter().filter(move |c| c.list == list)
    }

    /// A list's code.
    pub fn code(&self, list: &str, code: &str) -> Option<&Code> {
        self.codes.iter().find(|c| c.list == list && c.code == code)
    }

    pub fn template(&self, table: &str) -> Option<&Template> {
        self.templates.iter().find(|t| t.table == table)
    }

    pub fn clause(&self, no: &str) -> Option<&Clause> {
        self.clauses.iter().find(|c| c.clause == no)
    }

    /// Every row, in the annexes' order: EK-1's, EK-2's … each annex's clauses before its tables.
    pub fn rows(&self) -> Vec<Row<'_>> {
        let mut out: Vec<(u8, u8, Row<'_>)> = Vec::new();
        for c in &self.clauses {
            out.push((c.annex, 0, Row::Clause(c)));
        }
        if let Some(p) = &self.project_information {
            out.push((1, 1, Row::ProjectInformation(p)));
        }
        for t in &self.templates {
            out.push((annex_of(&t.table), 1, Row::Template(t)));
        }
        for c in &self.codes {
            out.push((annex_of(&c.table), 1, Row::Code(c)));
        }
        for f in &self.fonts {
            out.push((4, 1, Row::Font(f)));
        }
        for f in &self.figures {
            out.push((4, 2, Row::Figure(f)));
        }
        for m in &self.marks {
            out.push((4, 2, Row::Mark(m)));
        }
        // Stable: within an annex and a kind, the generator's printed order stays.
        out.sort_by_key(|(annex, kind, _)| (*annex, *kind));
        out.into_iter().map(|(_, _, r)| r).collect()
    }

    /// A row by its id (“T2.2/MM”, “M4.11”, “S4.7/3”).
    pub fn row(&self, id: &str) -> Option<Row<'_>> {
        self.rows().into_iter().find(|r| r.id() == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_profile_reads_whole() {
        let c: Catalog = serde_json::from_str(BASE_JSON).expect("the base profile's text reads");
        assert_eq!(c.format, 1);
        assert_eq!(c.profile.gazette, "5.8.2026/33331");
        assert_eq!(c.profile.annexes_pdf.pages, 241);
        assert_eq!(base().codes.len(), c.codes.len());
    }

    #[test]
    fn every_row_has_its_page_and_a_unique_id() {
        let mut ids = std::collections::HashSet::new();
        for r in base().rows() {
            assert!(ids.insert(r.id().to_owned()), "{} twice", r.id());
            let s = r.source();
            assert!(!s.pages.is_empty(), "{} has no page", r.id());
            assert!(s.pages.iter().all(|p| (1..=241).contains(p)), "{}", r.id());
            assert!((1..=8).contains(&s.annex), "{}", r.id());
        }
        assert!(ids.len() > 500);
    }

    #[test]
    fn ek_2_and_ek_3_lists_are_as_printed() {
        let c = base();
        let disciplines: Vec<_> = c.list(DISCIPLINES).map(|d| d.code.as_str()).collect();
        assert_eq!(
            disciplines,
            ["MM", "IM", "ST", "EE", "MK", "HR", "AL", "PY", "ZE", "YN", "GS", "AK"]
        );
        let mm = c.code(DISCIPLINES, "MM").expect("Mimari");
        assert_eq!((mm.name.as_str(), mm.table.as_str(), mm.page), ("Mimari", "2.2", 3));
        let ks = c.code(FILE_KINDS, "KS").expect("Kesit");
        assert_eq!(ks.note.as_deref(), Some("Tip kesitler de dahil"));
        let mkf = c.code(FILE_TYPES, "MKF").expect("Model Kontrol Formu");
        assert_eq!(mkf.group.as_deref(), Some("TÜM DİSİPLİNLERDE OLABİLECEK DOSYALAR"));
        assert_eq!(c.code(STOREYS, "K00").map(|s| s.name.as_str()), Some("Zemin Kat"));
        assert_eq!(c.code(ROOMS, "SLN").map(|s| s.name.as_str()), Some("Salon"));
        assert!(c.list(ROOMS).count() > 140);
        let t = c.template("2.1").expect("Tablo 2.1");
        let lengths: Vec<_> = t.fields.iter().map(|f| f.length.clone().unwrap_or_default()).collect();
        assert_eq!(lengths, ["6", "2", "2", "4", "2", "3", "3", "2", "3"]);
        assert_eq!(items_list("MM"), Some("item:MM"));
        assert!(c.list("item:MM").any(|i| i.code == "EMSL"));
    }

    #[test]
    fn a_clause_cut_by_a_page_keeps_both_pages() {
        let c = base().clause("4.12.3").expect("EK-4 4.12.3");
        assert_eq!(c.pages, vec![20, 21]);
        assert!(c.text.contains("İlgili idare"));
    }
}
