//! The base profile's IFC annexes (Dijital Hazırlama Yönetmeliği EK-5, EK-6, EK-7, EK-9; plan/ifc-uyum.md IFC-01,
//! docs/adr/8062): data/temel-2026.json, written by scripts/mevzuat/katalog.py from the annexes' transcription. Every
//! row with its table, its row and its PDF page, its words as printed:
//!
//! - EK-5 Tablo 5.1–5.2: the 69 mandatory and 67 optional classes, their Turkish names and category codes; Tablo 5.3
//!   the entity name template; the clauses 5.1–5.5.4.
//! - EK-6 Tablo 6.1–6.135 and EK-7 Tablo 7.1–7.12: every class's definition, predefined types, attributes, properties,
//!   quantities, material and relations, mandatory or optional; the clauses 6.1–6.6, 7.1–7.4.
//! - EK-9 Tablo 9.1–9.2: the model quality control forms' questions.
//!
//! A row the source spells otherwise than the IFC 4.3 schema keeps the source's spelling; the schema comparison
//! ([`crate::catalog_schema`]) finds it and a decision (data/kararlar-2026.json) settles it. A class is not asked to be in
//! every building: its rows are asked of the objects of it a model has (EK-5 5.1).

use std::sync::OnceLock;

use kentos_digital_project::catalog::{Clause, ProfileInfo, Template};
use kentos_digital_project::decision::{Decision, Decisions};
use kentos_digital_project::{Level, Source};
use serde::{Deserialize, Serialize};

/// The classes an architectural model meets (EK-5 Tablo 5.1; plan/ifc-uyum.md §4): the spatial structure, the
/// architectural elements, the zones and the blocks a building's drawing holds. IfcFooting (the structural model's) and
/// IfcSign are among them as the table has them.
pub const ARCHITECTURE: [&str; 30] = [
    "IfcProject",
    "IfcSite",
    "IfcBuilding",
    "IfcBuildingStorey",
    "IfcSpace",
    "IfcSpatialZone",
    "IfcZone",
    "IfcWall",
    "IfcCurtainWall",
    "IfcSlab",
    "IfcRoof",
    "IfcColumn",
    "IfcBeam",
    "IfcMember",
    "IfcDoor",
    "IfcWindow",
    "IfcOpeningElement",
    "IfcStair",
    "IfcRamp",
    "IfcRailing",
    "IfcCovering",
    "IfcChimney",
    "IfcGridAxis",
    "IfcGroup",
    "IfcFurniture",
    "IfcSanitaryTerminal",
    "IfcElectricAppliance",
    "IfcTransportElement",
    "IfcFooting",
    "IfcSign",
];

/// An EK-5 class: its table (5.1 mandatory, 5.2 optional), row, page, Turkish name and category code.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Class {
    pub id: String,
    pub table: String,
    pub row: u32,
    pub page: u32,
    pub class: String,
    pub tr: String,
    pub code: String,
    pub level: Level,
}

/// An EK-6 or EK-7 table: its class, the class's Turkish name, definition and predefined types as printed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub id: String,
    pub table: String,
    pub class: String,
    pub tr: Option<String>,
    pub level: Level,
    pub pages: Vec<u32>,
    pub title: String,
    pub definition: Option<String>,
    pub predefined: Vec<String>,
}

/// What a requirement row asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// An attribute of the class (“Attribute: Name”, PredefinedType).
    Attribute,
    /// A property of a property set (Pset_…, TREpys_…).
    Property,
    /// A quantity of a quantity set (Qto_…).
    Quantity,
    /// The material's name, through `IfcRelAssociatesMaterial` (“Malzeme İsmi_Dayanım”).
    Material,
    /// A relation the object takes part in (`IfcRelAssignsToGroup>IfcSystem`).
    Relation,
}

impl Kind {
    pub fn named(self) -> &'static str {
        match self {
            Kind::Attribute => "Öznitelik",
            Kind::Property => "Özellik",
            Kind::Quantity => "Miktar",
            Kind::Material => "Malzeme",
            Kind::Relation => "İlişki",
        }
    }
}

/// An EK-6 or EK-7 row: what it asks of the class's objects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub table: String,
    pub row: u32,
    pub page: u32,
    pub class: String,
    pub level: Level,
    pub kind: Kind,
    pub set: Option<String>,
    pub path: Option<String>,
    pub name: String,
    pub tr: String,
    /// The table's data type, for information (Metin, Sayı, Mantıksal, Sayı veya Metin).
    pub datatype: String,
    pub relation: Option<String>,
    pub description: Option<String>,
}

impl Requirement {
    /// Its IFC path: “Pset_WallCommon.FireRating”, “Name”, “IfcMaterial.Name”, “IfcRelAssignsToGroup>IfcSystem”.
    pub fn path(&self) -> String {
        match self.kind {
            Kind::Property | Kind::Quantity => {
                format!("{}.{}", self.set.as_deref().unwrap_or("?"), self.name)
            }
            Kind::Material => "IfcMaterial.Name".to_owned(),
            Kind::Relation => self.path.clone().unwrap_or_else(|| self.name.clone()),
            Kind::Attribute => self.name.clone(),
        }
    }

    pub fn source(&self) -> Source {
        Source::table(annex_of(&self.table), &self.table, Some(self.row), self.page)
    }
}

/// A question of the model quality control forms (EK-9 Tablo 9.1 Mimari, 9.2 the other disciplines).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub form: String,
    pub no: u32,
    pub pages: Vec<u32>,
    pub group: Option<String>,
    pub discipline: String,
    pub title: String,
    pub question: String,
}

/// The base profile's IFC annexes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    pub format: u32,
    pub profile: ProfileInfo,
    pub clauses: Vec<Clause>,
    pub template: Option<Template>,
    pub classes: Vec<Class>,
    pub tables: Vec<Table>,
    pub requirements: Vec<Requirement>,
    pub questions: Vec<Question>,
}

/// The base profile's text, as the generator wrote it.
pub const BASE_JSON: &str = include_str!("../data/temel-2026.json");
/// KentOS's IFC decisions' text.
pub const DECISIONS_JSON: &str = include_str!("../data/kararlar-2026.json");

/// The base profile's IFC annexes, read once. The text is KentOS's own and a test reads it whole; were it unreadable,
/// the catalog is empty and every rule unknown, never guessed.
pub fn base() -> &'static Catalog {
    static BASE: OnceLock<Option<Catalog>> = OnceLock::new();
    static EMPTY: OnceLock<Catalog> = OnceLock::new();
    match BASE.get_or_init(|| serde_json::from_str(BASE_JSON).ok()) {
        Some(c) => c,
        None => EMPTY.get_or_init(|| Catalog {
            format: 0,
            profile: kentos_digital_project::catalog::base().profile.clone(),
            clauses: Vec::new(),
            template: None,
            classes: Vec::new(),
            tables: Vec::new(),
            requirements: Vec::new(),
            questions: Vec::new(),
        }),
    }
}

/// KentOS's IFC decisions (the research note's IFC points and the schema comparison's findings), read once.
pub fn decisions() -> &'static [Decision] {
    static D: OnceLock<Decisions> = OnceLock::new();
    &D.get_or_init(|| Decisions::read(DECISIONS_JSON)).decisions
}

/// Any row, as the rules window lists it and a decision names it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Row<'a> {
    Clause(&'a Clause),
    Template(&'a Template),
    Class(&'a Class),
    Table(&'a Table),
    Requirement(&'a Requirement),
    Question(&'a Question),
}

impl Row<'_> {
    pub fn id(&self) -> &str {
        match self {
            Row::Clause(c) => &c.id,
            Row::Template(t) => &t.id,
            Row::Class(c) => &c.id,
            Row::Table(t) => &t.id,
            Row::Requirement(r) => &r.id,
            Row::Question(q) => &q.id,
        }
    }

    pub fn source(&self) -> Source {
        match self {
            Row::Clause(c) => Source::clause(c.annex, &c.clause, &c.pages),
            Row::Template(t) => Source::table(5, &t.table, None, t.page),
            Row::Class(c) => Source::table(5, &c.table, Some(c.row), c.page),
            Row::Table(t) => Source {
                annex: annex_of(&t.table),
                table: Some(t.table.clone()),
                pages: t.pages.clone(),
                ..Source::default()
            },
            Row::Requirement(r) => r.source(),
            Row::Question(q) => Source {
                annex: 9,
                table: Some(q.form.clone()),
                row: Some(q.no),
                pages: q.pages.clone(),
                ..Source::default()
            },
        }
    }

    /// Its short name.
    pub fn title(&self) -> String {
        match self {
            Row::Clause(c) => format!("Madde {}", c.clause),
            Row::Template(t) => t.title.clone(),
            Row::Class(c) => format!("{} — {} ({})", c.code, c.tr, c.class),
            Row::Table(t) => format!("{} ({})", t.tr.as_deref().unwrap_or(&t.class), t.class),
            Row::Requirement(r) => {
                if r.tr.is_empty() {
                    r.name.clone()
                } else {
                    r.tr.clone()
                }
            }
            Row::Question(q) => format!("{}. {}", q.no, q.title),
        }
    }

    /// Its words.
    pub fn text(&self) -> String {
        match self {
            Row::Clause(c) => c.text.clone(),
            Row::Template(t) => t.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(" - "),
            Row::Class(c) => format!("{} · {}", c.class, c.level.named()),
            Row::Table(t) => {
                let mut s = t.definition.clone().unwrap_or_default();
                if !t.predefined.is_empty() {
                    s.push_str(&format!(" Ön tanımlı tipler: {}.", t.predefined.join(", ")));
                }
                s.trim().to_owned()
            }
            Row::Requirement(r) => r.description.clone().unwrap_or_default(),
            Row::Question(q) => q.question.clone(),
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
    /// An EK-5 class by its IFC name.
    pub fn class(&self, name: &str) -> Option<&Class> {
        self.classes.iter().find(|c| c.class == name)
    }

    /// A class's EK-6 or EK-7 table of a level (mandatory: 6.1–6.68, 7.1–7.9; optional: 6.69–6.135, 7.10–7.12).
    pub fn table_of(&self, class: &str, level: Level) -> Option<&Table> {
        self.tables.iter().find(|t| t.class == class && t.level == level)
    }

    /// A class's rows, mandatory ones first.
    pub fn requirements_of<'a>(&'a self, class: &'a str) -> impl Iterator<Item = &'a Requirement> + 'a {
        let mandatory = self
            .requirements
            .iter()
            .filter(move |r| r.class == class && r.level == Level::Mandatory);
        let optional = self
            .requirements
            .iter()
            .filter(move |r| r.class == class && r.level != Level::Mandatory);
        mandatory.chain(optional)
    }

    /// Every row in the annexes' order: EK-5's clauses, classes and template, EK-6's and EK-7's tables each followed by
    /// its rows, EK-9's questions.
    pub fn rows(&self) -> Vec<Row<'_>> {
        let mut out = Vec::new();
        for annex in [5u8, 6, 7, 9] {
            out.extend(self.clauses.iter().filter(|c| c.annex == annex).map(Row::Clause));
            if annex == 5 {
                out.extend(self.classes.iter().map(Row::Class));
                out.extend(self.template.iter().map(Row::Template));
            }
            for t in self.tables.iter().filter(|t| annex_of(&t.table) == annex) {
                out.push(Row::Table(t));
                out.extend(self.requirements.iter().filter(|r| r.table == t.table).map(Row::Requirement));
            }
            if annex == 9 {
                out.extend(self.questions.iter().map(Row::Question));
            }
        }
        out
    }

    /// A row by its id (“T6.66/Pset_WallCommon.FireRating”, “T5.1/IfcWall”, “M5.5.3”, “T9.1/21”).
    pub fn row(&self, id: &str) -> Option<Row<'_>> {
        self.rows().into_iter().find(|r| r.id() == id)
    }

    /// A requirement row's level (for the profile's layers: an administration may ask an optional row, Md. 12/2).
    pub fn level_of(&self, id: &str) -> Option<Level> {
        self.requirements.iter().find(|r| r.id == id).map(|r| r.level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building;

    #[test]
    fn the_ifc_annexes_read_whole() {
        let c: Catalog = serde_json::from_str(BASE_JSON).expect("the IFC annexes' text reads");
        assert_eq!(c.format, 1);
        assert_eq!(c.classes.len(), 136, "EK-5: 69 mandatory and 67 optional classes");
        assert_eq!(c.classes.iter().filter(|x| x.level == Level::Mandatory).count(), 69);
        assert_eq!(c.tables.len(), 147, "EK-6 6.1–6.135 and EK-7 7.1–7.12");
        assert_eq!(c.questions.iter().filter(|q| q.form == "9.1").count(), 21);
        assert_eq!(c.questions.iter().filter(|q| q.form == "9.2").count(), 16);
        let d: Decisions = serde_json::from_str(DECISIONS_JSON).expect("the IFC decisions read");
        assert_eq!(d.format, 1);
    }

    #[test]
    fn every_row_has_its_page_and_a_unique_id() {
        let mut ids = std::collections::HashSet::new();
        for r in base().rows() {
            assert!(ids.insert(r.id().to_owned()), "{} twice", r.id());
            let s = r.source();
            assert!(!s.pages.is_empty(), "{}", r.id());
            assert!(s.pages.iter().all(|p| (29..=241).contains(p)), "{}: {:?}", r.id(), s.pages);
        }
    }

    #[test]
    fn the_wall_s_rows_are_ek_6_s() {
        let c = base();
        let wall = c.class("IfcWall").expect("IfcWall");
        assert_eq!((wall.code.as_str(), wall.table.as_str(), wall.level), ("DVR", "5.1", Level::Mandatory));
        let t = c.table_of("IfcWall", Level::Mandatory).expect("Tablo 6.66");
        assert_eq!(t.table, "6.66");
        assert!(t.predefined.iter().any(|p| p == "ELEMENTED WALL"), "printed with its space");
        let rows: Vec<String> = c
            .requirements_of("IfcWall")
            .filter(|r| r.level == Level::Mandatory)
            .map(Requirement::path)
            .collect();
        assert_eq!(
            rows,
            ["PredefinedType", "Name", "Tag", "IfcMaterial.Name", "Pset_WallCommon.IsExternal", "Pset_WallCommon.FireRating"]
        );
        let fire = c.row("T6.66/Pset_WallCommon.FireRating").expect("the wall's fire rating");
        assert_eq!(fire.source().cite(), "EK-6 Tablo 6.66 satır 6 · PDF s. 107");
    }

    #[test]
    fn every_class_kentos_writes_has_ek_5_s_code() {
        for class in [
            building::Class::Wall,
            building::Class::Slab,
            building::Class::Column,
            building::Class::Beam,
            building::Class::Roof,
            building::Class::Railing,
            building::Class::Stair,
            building::Class::Covering,
            building::Class::Door,
            building::Class::Window,
            building::Class::TransportElement,
            building::Class::Furniture,
            building::Class::SanitaryTerminal,
            building::Class::ElectricAppliance,
            building::Class::Member,
            building::Class::Chimney,
            building::Class::Ramp,
        ] {
            let name = ifc_name(class.entity());
            let row = base().class(&name).unwrap_or_else(|| panic!("{name} is not EK-5's"));
            assert_eq!(row.code, class.code(), "{name}");
        }
        for name in ARCHITECTURE {
            assert!(base().class(name).is_some(), "{name}");
        }
    }

    /// “IFCWALL” as the schema spells it, through the catalog's own names.
    fn ifc_name(step: &str) -> String {
        base()
            .classes
            .iter()
            .find(|c| c.class.to_ascii_uppercase() == step)
            .map(|c| c.class.clone())
            .unwrap_or_else(|| step.to_owned())
    }

    #[test]
    fn a_form_question_cut_by_a_page_is_one_question() {
        let q = base().row("T9.1/9").expect("question 9");
        assert_eq!(q.source().pages, vec![238, 239]);
        assert!(q.text().contains("zorunlu öznitelikler ve özellikler"));
    }
}
