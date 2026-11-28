//! The dated scope decision's data (Dijital Hazırlama Yönetmeliği Md. 1, 16, Geçici 1–2; e-PYS Yönetmeliği Md. 1, 4/4,
//! 8; plan/bim-plan.md BIM-02, docs/adr/8063). The decision itself is a Python rule (bim-plan K10;
//! `catalog/standards/dijital-proje`, `dijital_proje.kapsam`); this module holds what it reads:
//!
//! - [`inputs`]: the project's particulars it asks, as the drawing's variables hold them: the contract between the
//!   drawing, the facts KentOS gives the rule (`facts.project.permit`) and the window that asks them. A value is text as
//!   the user typed it; the rule reads dates as `GG.AA.YYYY` or `YYYY-AA-GG`, numbers with or without the thousands'
//!   dots, a choice by its words. An empty value is no value: the rule says what it needs, it does not guess
//!   (bim-plan K3). An empty application date is an application still to be made.
//! - [`calendar`]: the inputs, the dates and thresholds (data/kapsam-2026.json): the acts' coming into force, the regulation's
//!   scope and exceptions, each obligation's stages with their population and area thresholds, every one with the
//!   article it was read from. The rule gets this file as data; the standards package carries a copy a test holds to
//!   it.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::source::Source;

/// What an input's value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Date,
    Number,
    Text,
    /// One of its `choices`.
    Choice,
}

/// One input: the drawing variable's name, the words a window asks it with, what it is and, for a choice, its words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub name: String,
    pub label: String,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

/// The inputs, in the order a window asks them; none when the calendar does not read.
pub fn inputs() -> &'static [Input] {
    calendar().map_or(&[], |c| c.inputs.as_slice())
}

/// An input by its variable's name.
pub fn input(name: &str) -> Option<&'static Input> {
    inputs().iter().find(|i| i.name == name)
}

/// The dates and thresholds file (`kentos.digitalScope` 1).
pub const CALENDAR_JSON: &str = include_str!("../data/kapsam-2026.json");

/// The dates and thresholds, as data/kapsam-2026.json writes them. Dates are `YYYY-AA-GG`; a population or an area
/// threshold is passed only by a value greater than it (“geçen”, “fazla olan”: docs/adr/8063 K23).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub format: String,
    pub version: u32,
    /// The particulars the decision reads.
    pub inputs: Vec<Input>,
    /// The two regulations: the preparation one and e-PYS, each with the day it comes into force.
    pub acts: Vec<Act>,
    pub scope: ScopeRules,
    /// Geçici 1/2: the uses the regulation starts with; the others wait until `from`.
    pub uses: Uses,
    /// Geçici 2: a provision starting on a date does not reach a building applied for (or tendered) before it.
    pub earlier: Cited,
    /// PDF/A, IFC, PDF/A from the model, e-PYS: in the order a window lists them.
    pub obligations: Vec<Obligation>,
}

/// A regulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Act {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    pub gazette: String,
    pub in_force: String,
    pub source: Source,
}

/// A rule known by its article.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cited {
    pub source: Source,
}

/// What the regulation covers (Md. 1) and what e-PYS leaves out besides (e-PYS Md. 1/2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRules {
    /// Areas with an uygulama imar planı.
    pub plan: Cited,
    /// New buildings.
    pub new_building: Cited,
    pub detached: Detached,
    /// Farm and livestock buildings that are not an integrated plant.
    pub farm: Cited,
    /// The applications to be made (Md. 1/1): one made before the regulation comes into force is not.
    pub applications: Cited,
    pub law_articles: LawArticles,
}

/// A detached building of at most `most_storeys` storeys besides its basements and at most `most_area` m² in all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detached {
    pub most_storeys: u32,
    pub most_area: f64,
    pub source: Source,
}

/// The building law's articles whose buildings e-PYS leaves out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LawArticles {
    pub articles: Vec<String>,
    pub source: Source,
}

/// The uses the regulation starts with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Uses {
    pub residential: Vec<String>,
    pub from: String,
    pub source: Source,
}

/// An obligation and when it starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obligation {
    pub id: String,
    pub title: String,
    /// The articles that ask it.
    pub sources: Vec<Source>,
    /// When it starts: the first stage whose thresholds the building passes; the last has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<Stage>,
    /// It stops for a building once this other obligation starts for it (Geçici 1/1-b's last sentence).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_with: Option<EndsWith>,
    /// It starts with this other obligation (Md. 9/1 with the model's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follows: Option<String>,
    /// The act it is of, when not the preparation regulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<String>,
    /// The ministry's decision it waits for (e-PYS Md. 4/4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Cited>,
}

/// A stage: from a day, for the buildings in a province of more than `province` people, a municipality of more than
/// `municipality` and of more than `area` m²; a stage without thresholds is every other building.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stage {
    pub from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub municipality: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<f64>,
    pub source: Source,
}

/// An obligation that ends another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndsWith {
    pub obligation: String,
    pub source: Source,
}

impl Calendar {
    /// A calendar's text read; an unreadable one is none, never a guessed one.
    pub fn read(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }

    /// An obligation by its id.
    pub fn obligation(&self, id: &str) -> Option<&Obligation> {
        self.obligations.iter().find(|o| o.id == id)
    }
}

/// The 2026 calendar, read once.
pub fn calendar() -> Option<&'static Calendar> {
    static CALENDAR: OnceLock<Option<Calendar>> = OnceLock::new();
    CALENDAR
        .get_or_init(|| Calendar::read(CALENDAR_JSON))
        .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_input_has_a_name_of_its_own_and_a_choice_its_words() {
        let mut names = std::collections::HashSet::new();
        assert_eq!(inputs().len(), 15);
        for i in inputs() {
            assert!(names.insert(i.name.as_str()), "{} twice", i.name);
            assert!(
                i.name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "{}",
                i.name
            );
            assert!(!i.label.is_empty());
            assert_eq!(i.kind == Kind::Choice, !i.choices.is_empty(), "{}", i.name);
        }
        let use_ = input("kullanim").map(|i| (i.kind, i.choices.clone()));
        assert_eq!(
            use_,
            Some((
                Kind::Choice,
                vec!["konut".into(), "ticaret+konut".into(), "diğer".into()]
            ))
        );
        assert!(input("yok").is_none());
    }

    /// `YYYY-AA-GG` with a month and a day that exist.
    fn day(text: &str) -> Option<(u32, u32, u32)> {
        let mut parts = text.split('-');
        let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
            return None;
        }
        let (y, m, d): (u32, u32, u32) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let days = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        (1..=12).contains(&m).then_some(())?;
        (1..=days[m as usize - 1]).contains(&d).then_some((y, m, d))
    }

    #[test]
    fn the_calendar_reads_with_its_dates_in_order_and_every_rule_cited() {
        let c = calendar().expect("data/kapsam-2026.json reads");
        assert_eq!((c.format.as_str(), c.version), ("kentos.digitalScope", 1));
        let force: Vec<_> = c
            .acts
            .iter()
            .map(|a| (a.id.as_str(), a.in_force.as_str()))
            .collect();
        assert_eq!(force, [("hazirlama", "2027-09-01"), ("epys", "2028-09-01")]);
        assert_eq!(c.acts[0].source.cite(), "Md. 16");
        assert_eq!(c.acts[1].source.cite(), "e-PYS Md. 8");
        assert_eq!(
            (c.scope.detached.most_storeys, c.scope.detached.most_area),
            (2, 200.0)
        );
        assert_eq!(c.scope.law_articles.articles, ["4", "26", "27"]);
        assert_eq!(c.uses.residential, ["konut", "ticaret+konut"]);
        assert!(day(&c.uses.from).is_some());
        assert_eq!(c.earlier.source.cite(), "Geçici Md. 2");
        let ids: Vec<_> = c.obligations.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, ["pdfa", "ifc", "modelden-pdfa", "epys"]);
        for o in &c.obligations {
            assert!(
                !o.sources.is_empty() && o.sources.iter().all(|s| !s.cite().is_empty()),
                "{}",
                o.id
            );
            let mut last = None;
            for (i, st) in o.stages.iter().enumerate() {
                let at = day(&st.from).unwrap_or_else(|| panic!("{}: {}", o.id, st.from));
                assert!(last < Some(at), "{}: stages in order", o.id);
                last = Some(at);
                let open = st.province.is_none() && st.municipality.is_none() && st.area.is_none();
                assert_eq!(
                    open,
                    i + 1 == o.stages.len(),
                    "{}: only the last stage is every building",
                    o.id
                );
                assert!(st.source.cite().starts_with("Geçici Md. 1/1-"), "{}", o.id);
            }
            for other in o.ends_with.iter().map(|e| &e.obligation).chain(&o.follows) {
                assert!(c.obligation(other).is_some(), "{}: no {other}", o.id);
            }
        }
        let pdfa = c
            .obligation("pdfa")
            .map(|o| o.stages.iter().map(|s| s.from.as_str()).collect::<Vec<_>>());
        assert_eq!(pdfa, Some(vec!["2027-09-01", "2029-03-01"]));
        let ifc = c.obligation("ifc").map(|o| {
            o.stages
                .iter()
                .map(|s| (s.from.as_str(), s.province, s.municipality, s.area))
                .collect::<Vec<_>>()
        });
        assert_eq!(
            ifc,
            Some(vec![
                ("2029-09-01", Some(2_000_000), Some(600_000), Some(10_000.0)),
                ("2030-09-01", Some(2_000_000), Some(400_000), Some(10_000.0)),
                ("2031-09-01", Some(1_000_000), Some(300_000), Some(10_000.0)),
                ("2032-09-01", None, None, None),
            ])
        );
        assert_eq!(
            c.obligation("epys")
                .and_then(|o| o.decision.as_ref())
                .map(|d| d.source.cite())
                .as_deref(),
            Some("e-PYS Md. 4/4")
        );
    }

    /// The standards package carries a copy of the calendar, so that its rule runs where KentOS does not hand it the
    /// data (a cloud job, a test): the copy is this file, byte for byte.
    #[test]
    fn the_standards_package_carries_this_calendar() {
        let copy = include_str!(
            "../../../../catalog/standards/dijital-proje/src/dijital_proje/data/kapsam-2026.json"
        );
        assert_eq!(
            copy, CALENDAR_JSON,
            "copy data/kapsam-2026.json to the package"
        );
    }
}
