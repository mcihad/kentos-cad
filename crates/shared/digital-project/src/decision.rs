//! KentOS's decisions on the regulation's open points (docs/adr/8062 §4; docs/research/2026-11-15-bim-ifc-yonetmelik.md
//! §4): each says how it was settled — read from a page, settled by a schema, waiting for the administration or the
//! ministry, or KentOS's own reading until one answers — with its sources, the rows it touches and, where it settles a
//! parameter of the profile, the parameter. An open parameter has no value: nothing fills it with a guess; KentOS's
//! default, where it has one, is said as its own.
//!
//! This crate's decisions are the regulation's general and 2D ones (data/kararlar-2026.json); the IFC ones are
//! kentos-ifc's, of the same type.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::source::Source;

/// How a point was settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// Read from the annexes' page.
    Page,
    /// Settled by the IFC 4.3 schema or its official templates: the schema is above a table's spelling.
    Schema,
    /// The administration or the ministry answers; until then the parameter is open.
    Administration,
    /// KentOS's own reading, said as such, until the administration answers.
    Kentos,
}

impl Kind {
    pub fn named(self) -> &'static str {
        match self {
            Kind::Page => "Sayfadan okundu",
            Kind::Schema => "Şemadan çözüldü",
            Kind::Administration => "İdareye soruldu",
            Kind::Kentos => "KentOS'un yorumu",
        }
    }
}

/// A parameter of the profile a decision settles: its value, or none while it is open, and KentOS's default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Param {
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
}

/// A decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    /// “K01” …: stable, the rules window and the ADR name it.
    pub id: String,
    /// The research note's point (§4, 1…17), when it is one of them.
    #[serde(default)]
    pub point: Option<u8>,
    pub title: String,
    pub kind: Kind,
    pub sources: Vec<Source>,
    pub text: String,
    /// Who answers an open point.
    #[serde(default)]
    pub asked: Option<String>,
    #[serde(default)]
    pub param: Option<Param>,
    /// The catalog rows it touches, by their ids.
    #[serde(default)]
    pub rows: Vec<String>,
    /// The comparison's findings it settles, by their keys (kentos-ifc's schema comparison: “set:Qto_…”); a key ending
    /// in “*” settles every finding it begins.
    #[serde(default)]
    pub settles: Vec<String>,
    /// Names the source prints one way and KentOS uses another (a misspelled set, a class, a predefined type).
    #[serde(default)]
    pub replaces: Vec<Replace>,
}

/// A printed name and the name KentOS uses for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replace {
    pub printed: String,
    pub used: String,
}

impl Decision {
    /// Whether it settles a finding of this key.
    pub fn settles_key(&self, key: &str) -> bool {
        self.settles.iter().any(|s| match s.strip_suffix('*') {
            Some(prefix) => key.starts_with(prefix),
            None => s == key,
        })
    }

    /// Whether it still waits for an answer.
    pub fn open(&self) -> bool {
        self.kind == Kind::Administration
            || self.param.as_ref().is_some_and(|p| p.value.is_none())
    }
}

/// A decisions file: data/kararlar-2026.json here, kentos-ifc's beside its catalog.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decisions {
    pub format: u32,
    pub decisions: Vec<Decision>,
}

impl Decisions {
    /// A decisions file's text read; an unreadable one is no decision, never a guessed one.
    pub fn read(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or(Self {
            format: 0,
            decisions: Vec::new(),
        })
    }
}

/// This crate's decisions file.
pub const BASE_JSON: &str = include_str!("../data/kararlar-2026.json");

/// The regulation's general and 2D decisions, read once.
pub fn base() -> &'static [Decision] {
    static BASE: OnceLock<Decisions> = OnceLock::new();
    &BASE.get_or_init(|| Decisions::read(BASE_JSON)).decisions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    #[test]
    fn the_decisions_read_and_name_rows_that_are_there() {
        let d: Decisions = serde_json::from_str(BASE_JSON).expect("the decisions' text reads");
        assert_eq!(d.format, 1);
        let mut ids = std::collections::HashSet::new();
        for x in base() {
            assert!(ids.insert(x.id.clone()), "{} twice", x.id);
            assert!(!x.sources.is_empty() || x.kind == Kind::Administration, "{} has no source", x.id);
            for r in &x.rows {
                assert!(catalog::base().row(r).is_some(), "{}: no row {r}", x.id);
            }
            if x.kind == Kind::Administration {
                assert!(x.asked.is_some(), "{}: who answers", x.id);
            }
        }
    }

    #[test]
    fn an_open_parameter_has_no_value() {
        let pdfa = base()
            .iter()
            .find(|d| d.param.as_ref().is_some_and(|p| p.key == "pdfa.part"))
            .expect("the PDF/A part's decision");
        assert!(pdfa.open());
        let p = pdfa.param.as_ref().expect("its parameter");
        assert_eq!(p.value, None);
        assert_eq!(p.default.as_deref(), Some("PDF/A-2b"));
    }
}
