//! Every rule of the base profile as one list (docs/adr/8062 §9): the 2D annexes' rows (`kentos-digital-project`: EK-1…
//! EK-4, EK-8) and the IFC annexes' (`crate::catalog`: EK-5…EK-7, EK-9), each with its source, its level, the decisions
//! that touch it and whether an architectural model meets it. The rules window lists it; a filter narrows it by annex,
//! level, the architectural model and words. Nothing here reads a drawing: what a model carries is IFC-08's, from the
//! written file.

use kentos_digital_project::catalog::{self as dp, Row as DpRow};
use kentos_digital_project::decision::Decision;
use kentos_digital_project::{Level, Source};

use crate::catalog::{self, ARCHITECTURE, Row as IfcRow};

/// A rule as the list shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: String,
    pub annex: u8,
    /// Its short name: a requirement's Turkish name, a code's “MM — Mimari”, a clause's number.
    pub title: String,
    /// What it names: an IFC path (“Pset_WallCommon.FireRating”), a class, a code, a template's fields.
    pub path: String,
    /// The class it is asked of, for EK-5's, EK-6's and EK-7's rows, and the class's Turkish name (EK-5 Tablo 5.1–5.2,
    /// or its EK-7 table's).
    pub class: Option<String>,
    pub class_tr: Option<String>,
    pub level: Option<Level>,
    pub source: Source,
    /// Its words: a clause's text, a requirement's description, a question.
    pub text: String,
    /// The decisions that touch it, by their ids.
    pub decisions: Vec<String>,
    /// Whether an architectural model meets it: the 2D rules of Mimari and every discipline, the architectural classes
    /// (ARCHITECTURE), the project's classes, the architectural form.
    pub architecture: bool,
}

/// The lists of other disciplines (EK-2 Tablo 2.4–2.11; EK-4 Tablo 4.2, 4.3, 4.5, 4.6): not the architectural model's.
fn other_discipline(list: &str) -> bool {
    list.starts_with("subdiscipline:") || matches!(list, "item:IM" | "item:HR" | "item:ZE")
}

fn other_template(table: &str) -> bool {
    matches!(table, "4.2" | "4.3" | "4.5" | "4.6")
}

/// Both crates' decisions, the general ones first.
pub fn decisions() -> Vec<&'static Decision> {
    kentos_digital_project::decision::base().iter().chain(catalog::decisions()).collect()
}

/// A class's Turkish name: EK-5's, else its EK-6 or EK-7 table's.
fn turkish_of(class: &str) -> Option<String> {
    let c = catalog::base();
    c.class(class)
        .map(|x| x.tr.clone())
        .or_else(|| c.tables.iter().find(|t| t.class == class).and_then(|t| t.tr.clone()))
}

/// A clause as the list names it: its number and its words' start (“Madde 4.11: Katman ve sembol isimlendirmesinde …”).
fn clause_title(no: &str, text: &str) -> String {
    const MOST: usize = 72;
    let words: String = text.chars().take(MOST).collect();
    if text.chars().count() > MOST {
        format!("Madde {no}: {}…", words.trim_end())
    } else {
        format!("Madde {no}: {words}")
    }
}

/// Every rule, in the annexes' order.
pub fn all() -> Vec<Rule> {
    let decisions = decisions();
    let touching = |id: &str| -> Vec<String> {
        decisions.iter().filter(|d| d.rows.iter().any(|r| r == id)).map(|d| d.id.clone()).collect()
    };
    let mut out = Vec::new();
    for r in dp::base().rows() {
        let (path, architecture) = match r {
            DpRow::Code(c) => (c.code.clone(), !other_discipline(&c.list) && !c.group.as_deref().is_some_and(|g| !g.starts_with("MİMARİ") && !g.starts_with("TÜM"))),
            DpRow::Template(t) => (
                t.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(" _ "),
                !other_template(&t.table),
            ),
            DpRow::Font(f) => (f.points.clone(), true),
            DpRow::Mark(m) => (m.measure.clone(), true),
            _ => (String::new(), true),
        };
        let source = r.source();
        let title = match r {
            DpRow::Clause(c) => clause_title(&c.clause, &c.text),
            _ => r.title(),
        };
        out.push(Rule {
            id: r.id().to_owned(),
            annex: source.annex,
            title,
            path,
            class: None,
            class_tr: None,
            level: None,
            source,
            text: r.text(),
            decisions: touching(r.id()),
            architecture,
        });
    }
    for r in catalog::base().rows() {
        let (path, class, level, architecture) = match r {
            IfcRow::Requirement(q) => (
                q.path(),
                Some(q.class.clone()),
                Some(q.level),
                ARCHITECTURE.contains(&q.class.as_str()) || q.table.starts_with("7."),
            ),
            IfcRow::Class(c) => (c.code.clone(), Some(c.class.clone()), Some(c.level), ARCHITECTURE.contains(&c.class.as_str())),
            IfcRow::Table(t) => (
                String::new(),
                Some(t.class.clone()),
                Some(t.level),
                ARCHITECTURE.contains(&t.class.as_str()) || t.table.starts_with("7."),
            ),
            IfcRow::Question(q) => (q.title.clone(), None, None, q.form == "9.1"),
            IfcRow::Template(t) => (t.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(" - "), None, None, true),
            IfcRow::Clause(_) => (String::new(), None, None, true),
        };
        let source = r.source();
        let class_tr = class.as_deref().and_then(turkish_of);
        // A table's and a class's own row name what the class column does not: the table, the category code.
        let title = match r {
            IfcRow::Clause(c) => clause_title(&c.clause, &c.text),
            IfcRow::Table(t) => format!("Tablo {}: tanım ve ön tanımlı tipler", t.table),
            IfcRow::Class(c) => format!("Kategori kodu {}", c.code),
            _ => r.title(),
        };
        out.push(Rule {
            id: r.id().to_owned(),
            annex: source.annex,
            title,
            path,
            class,
            class_tr,
            level,
            source,
            text: r.text(),
            decisions: touching(r.id()),
            architecture,
        });
    }
    out
}

/// How the list is narrowed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    /// One annex (1…9), or all.
    pub annex: Option<u8>,
    /// Only what an architectural model meets.
    pub architecture: bool,
    /// One level, or all; a row without one (a code, a clause) is kept by none but all.
    pub level: Option<Level>,
    /// Words the row's title, path, class, source or text holds ([`fold`]ed: case and Turkish letters not minded).
    pub words: String,
}

/// Words as the search compares them: lower case, the Turkish letters with their plain ones (i, ı, İ and I alike; ç c,
/// ğ g, ö o, ş s, ü u), so “isik” finds “Işık” and “ifcwall” finds IfcWall.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'ı' | 'i' => 'i',
            'ç' => 'c',
            'ğ' => 'g',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            c => c,
        })
        .filter(|c| *c != '\u{307}')
        .collect()
}

impl Filter {
    pub fn keeps(&self, r: &Rule) -> bool {
        if self.annex.is_some_and(|a| a != r.annex) || (self.architecture && !r.architecture) {
            return false;
        }
        if self.level.is_some() && r.level != self.level {
            return false;
        }
        let words = fold(self.words.trim());
        if words.is_empty() {
            return true;
        }
        let hay = fold(&format!(
            "{} {} {} {} {} {}",
            r.title,
            r.path,
            r.class.as_deref().unwrap_or(""),
            r.class_tr.as_deref().unwrap_or(""),
            r.source.cite(),
            r.text
        ));
        words.split_whitespace().all(|w| hay.contains(w))
    }

    /// The rows it keeps, by their places.
    pub fn apply(&self, rules: &[Rule]) -> Vec<usize> {
        rules.iter().enumerate().filter(|(_, r)| self.keeps(r)).map(|(i, _)| i).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_is_listed_once_with_its_page() {
        let rules = all();
        let mut ids = std::collections::HashSet::new();
        for r in &rules {
            assert!(ids.insert(r.id.clone()), "{} twice", r.id);
            assert!(!r.source.pages.is_empty(), "{}", r.id);
        }
        assert_eq!(rules.len(), dp::base().rows().len() + catalog::base().rows().len());
    }

    #[test]
    fn the_architectural_model_s_wall_rows_are_found_by_their_words() {
        let rules = all();
        let f = Filter {
            annex: Some(6),
            architecture: true,
            level: Some(Level::Mandatory),
            words: "duvar yangın".into(),
        };
        let kept: Vec<&str> = f.apply(&rules).into_iter().map(|i| rules[i].id.as_str()).collect();
        assert_eq!(kept, ["T6.66/Pset_WallCommon.FireRating"]);
        let fire = &rules[f.apply(&rules)[0]];
        assert_eq!(fire.class.as_deref(), Some("IfcWall"));
        assert_eq!(fire.source.cite(), "EK-6 Tablo 6.66 satır 6 · PDF s. 107");
    }

    #[test]
    fn other_disciplines_rows_are_left_out_of_the_architectural_list() {
        let rules = all();
        let arch = Filter { architecture: true, ..Filter::default() };
        let kept: std::collections::HashSet<&str> =
            arch.apply(&rules).into_iter().map(|i| rules[i].id.as_str()).collect();
        assert!(kept.contains("T2.3/DUVA"), "Mimari's item");
        assert!(!kept.contains("T2.5/BETO"), "a structural sub-discipline");
        assert!(kept.contains("T6.58/Name"), "the space's name");
        assert!(!kept.contains("T6.1/Name"), "an air terminal's");
        assert!(kept.contains("T7.6/VerticalDatum"), "the project's system");
        assert!(kept.contains("T9.1/21") && !kept.contains("T9.2/1"));
    }

    #[test]
    fn a_rule_says_its_decisions() {
        let rules = all();
        let tag = rules.iter().find(|r| r.id == "T6.37/Tag").expect("IfcGroup's Tag");
        assert_eq!(tag.decisions, ["K06"]);
        assert_eq!(fold("İÇ MİMARİ Işık"), "ic mimari isik");
        assert_eq!(fold("IfcWall"), fold("ifcwall"));
        assert!(decisions().iter().any(|d| d.id == "K13") && decisions().iter().any(|d| d.id == "K06"));
    }
}
