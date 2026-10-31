//! An opening's fields beside its wall, place along it, width and jambs (docs/adr/0341 §1), as schema 77 writes
//! them: one map, each field only when it is not its default, so a file has one way to say an opening. Schema 119
//! (docs/adr/0719) adds a niche's and flue's form, `depth` and `inset` and the break `tooDeep`.

use kentos_contracts::{
    OpeningAnchor, OpeningBreak, OpeningFields, OpeningForm, OpeningHand, OpeningOperation, OpeningSide,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OpeningRest {
    #[serde(default, skip_serializing_if = "is_center")]
    pub anchor: OpeningAnchor,
    pub form: OpeningForm,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub leaves: u8,
    #[serde(default, skip_serializing_if = "is_swing")]
    pub operation: OpeningOperation,
    #[serde(default, skip_serializing_if = "is_start")]
    pub hand: OpeningHand,
    #[serde(default, skip_serializing_if = "is_left")]
    pub side: OpeningSide,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swing: Option<f64>,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub show_swing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broken: Option<OpeningBreak>,
    /// Schema 137 (docs/adr/0772): the version of the project type it follows, its exceptions, its construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<kentos_contracts::OpeningTypeField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<kentos_contracts::OpeningBuild>,
}

fn one() -> u8 {
    1
}
fn is_one(v: &u8) -> bool {
    *v == 1
}
fn yes() -> bool {
    true
}
fn is_yes(v: &bool) -> bool {
    *v
}
fn is_center(a: &OpeningAnchor) -> bool {
    *a == OpeningAnchor::Center
}
fn is_swing(o: &OpeningOperation) -> bool {
    *o == OpeningOperation::Swing
}
fn is_start(h: &OpeningHand) -> bool {
    *h == OpeningHand::Start
}
fn is_left(s: &OpeningSide) -> bool {
    *s == OpeningSide::Left
}

impl OpeningRest {
    pub(crate) fn of(f: &OpeningFields) -> OpeningRest {
        OpeningRest {
            anchor: f.anchor,
            form: f.form,
            leaves: f.leaves,
            operation: f.operation,
            hand: f.hand,
            side: f.side,
            swing: f.swing,
            show_swing: f.show_swing,
            frame: f.frame,
            sill: f.sill,
            head: f.head,
            depth: f.depth,
            inset: f.inset,
            style: f.style.clone(),
            number: f.number.clone(),
            broken: f.broken,
            type_version: f.type_version,
            overrides: f.overrides.clone(),
            build: f.build.clone(),
        }
    }

    pub(crate) fn with(self, host: String, at: f64, width: f64, place: Vec<kentos_contracts::Vec2>) -> OpeningFields {
        OpeningFields {
            host,
            at,
            anchor: self.anchor,
            width,
            form: self.form,
            leaves: self.leaves,
            operation: self.operation,
            hand: self.hand,
            side: self.side,
            swing: self.swing,
            show_swing: self.show_swing,
            frame: self.frame,
            sill: self.sill,
            head: self.head,
            depth: self.depth,
            inset: self.inset,
            style: self.style,
            number: self.number,
            type_version: self.type_version,
            overrides: self.overrides,
            build: self.build,
            place,
            broken: self.broken,
        }
    }
}
