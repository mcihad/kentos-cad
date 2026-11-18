//! Municipal fees and charges (docs/adr/9003): a municipality's yearly tariff as data and what it makes of a dossier.
//!
//! A tariff (`FeeSchedule`) is a municipality's fees and charges for a year (the law's fees at the amounts decided for
//! its group, 2464 m.95 and m.96/A, revalued; its council's charges, m.97): the inputs its items need
//! (`FeeParameter`: an area, a building's use, an exemption…), rate tables (`FeeTable`: a rate per unit by a choice and
//! a quantity's bracket, as 2464 sayılı Kanun Ek Madde 6 tabulates the building fee) and the items (`FeeItem`): a fixed
//! amount, a quantity times a rate, or a free formula over the inputs. Nothing about a fee is code: a new fee is a new
//! item, a new year is a copy revalued by a factor, a new kind of item is a new variant here.
//!
//! An assessment (`FeeAssessment`) is the tariff applied to one dossier with the values its staff gave: its lines say
//! what each amount rests on, its total is what the professional pays. Amounts are kuruş (1/100 TL), whole numbers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Where a tariff is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeeScheduleState {
    /// Being written; not applied to dossiers.
    Draft,
    /// The one applied to new assessments while its dates hold.
    Active,
    /// Replaced; its assessments keep it.
    Retired,
}

/// What an input is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeeParameterKind {
    /// A number (an area, a volume, a count), in its `unit`.
    Number,
    /// One of `choices` (a building's use, a zone).
    Choice,
    /// Yes or no (an exemption).
    Flag,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeChoice {
    pub value: String,
    pub label: String,
}

/// An input the tariff's items read: in a formula and a condition as `@key`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeParameter {
    /// Lower-case letters, digits and `_`, starting with a letter (`insaat_alani`).
    pub key: String,
    pub label: String,
    pub kind: FeeParameterKind,
    /// `m²`, `m³`, `sayfa`… for a number.
    #[serde(default)]
    pub unit: Option<String>,
    /// For a choice.
    #[serde(default)]
    pub choices: Vec<FeeChoice>,
    /// Asked of the staff on every assessment; one not given makes the assessment refused.
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub help: Option<String>,
}

/// A rate for a choice's value and a quantity's bracket; an empty `when` or bound matches every value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeTableRow {
    #[serde(default)]
    pub when: Option<String>,
    /// Inclusive bounds of the quantity (`101`–`120` m²).
    #[serde(default)]
    pub from: Option<f64>,
    #[serde(default)]
    pub to: Option<f64>,
    /// TL per unit of the quantity.
    pub rate: f64,
    /// The bounds the rate must stay within, when given: the law's “en az – en çok” (2464 Ek Madde 6, m.84).
    #[serde(default)]
    pub min_rate: Option<f64>,
    #[serde(default)]
    pub max_rate: Option<f64>,
}

/// A rate table: the first row whose choice and bracket match gives the rate; in a formula as `@<key>`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeTable {
    pub key: String,
    pub label: String,
    /// The choice parameter the rows' `when` is matched with, if any.
    #[serde(default)]
    pub by: Option<String>,
    /// The number parameter the rows' brackets are matched with, if any.
    #[serde(default)]
    pub quantity: Option<String>,
    pub rows: Vec<FeeTableRow>,
}

/// How an item's amount is found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeeRule {
    /// A fixed amount (maktu), TL.
    Fixed { amount: f64 },
    /// A number input times a rate: the rate a table gives, or a single rate.
    PerUnit {
        quantity: String,
        #[serde(default)]
        table: Option<String>,
        #[serde(default)]
        rate: Option<f64>,
    },
    /// Any formula over `@inputs` and `@tables`, in the expression language (`@alan * @birim * 1.5`).
    Formula { expression: String },
}

/// One fee or charge of the tariff.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeItem {
    /// Unique in the tariff (`bina_insaat_harci`).
    pub code: String,
    pub label: String,
    /// What it rests on (“2464 sayılı Kanun Ek Madde 1 ve 6”).
    #[serde(default)]
    pub legal_basis: String,
    /// `harç` (a fee of the law) or `ücret` (a charge of the council's tariff, 2464 m.97); shown as the line's group.
    #[serde(default)]
    pub group: String,
    pub rule: FeeRule,
    /// The dossier kinds it applies to; none: every kind.
    #[serde(default)]
    pub applies_to: Vec<String>,
    /// Applies only when this is true (`değil @kamu_binasi`); none: always.
    #[serde(default)]
    pub condition: Option<String>,
    /// Bounds of the amount, TL.
    #[serde(default)]
    pub minimum: Option<f64>,
    #[serde(default)]
    pub maximum: Option<f64>,
    /// VAT on top, percent (a fee of the law carries none).
    #[serde(default)]
    pub vat_percent: f64,
    /// Paid before the dossier's decision (2464 Ek Madde 5: before the permit).
    #[serde(default)]
    pub before_decision: bool,
}

/// How amounts are rounded, line by line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeeRounding {
    /// To the kuruş, half up.
    #[default]
    Kurus,
    /// Down to whole lira.
    LiraDown,
    /// To the nearest lira, half up.
    Lira,
}

/// A tariff's content: what it asks, its tables and its items.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeDefinition {
    #[serde(default)]
    pub parameters: Vec<FeeParameter>,
    #[serde(default)]
    pub tables: Vec<FeeTable>,
    #[serde(default)]
    pub items: Vec<FeeItem>,
    #[serde(default)]
    pub rounding: FeeRounding,
    /// How to pay, shown to the professional (bank, IBAN, the cash office).
    #[serde(default)]
    pub payment_note: Option<String>,
}

/// A municipality's tariff.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeSchedule {
    pub id: String,
    pub name: String,
    pub year: i32,
    /// `YYYY-MM-DD`.
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub state: FeeScheduleState,
    /// The decision it carries out (“Meclis kararı 2025/245, 12.12.2025”): the council's tariff for the charges (2464
    /// m.97); the fees' amounts are the law's and the decision for the municipality's group (m.95, m.96/A), revalued.
    pub decision: Option<String>,
    pub definition: FeeDefinition,
    pub updated_at: String,
    pub updated_by_name: String,
}

/// `POST`/`PUT …/fees/schedules`: a tariff as it is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeScheduleEdit {
    pub name: String,
    pub year: i32,
    pub valid_from: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub valid_to: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub decision: Option<String>,
    pub definition: FeeDefinition,
}

/// `POST …/fees/schedules/{id}/copy`: next year's tariff from this one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeScheduleCopy {
    pub year: i32,
    pub valid_from: String,
    /// Every rate, fixed amount and bound times this (the year's revaluation: `1.25` for a 25 % rise).
    pub factor: f64,
}

/// A value given for an input: a number, a choice's value or a flag.
pub type FeeInputs = BTreeMap<String, serde_json::Value>;

/// `POST /v1/dossiers/{id}/fees/preview` and `…/fees`: the values for the active tariff's inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeAssess {
    #[cfg_attr(feature = "ts", ts(type = "Record<string, number | string | boolean>"))]
    pub inputs: FeeInputs,
    /// When it is to be paid by, `YYYY-MM-DD`.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub due_on: Option<String>,
}

/// `POST /v1/tenants/{tenant}/fees/try`: what a tariff (saved or not, e.g. a draft being edited) makes of a dossier
/// kind's inputs; nothing is saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeTry {
    pub definition: FeeDefinition,
    /// The dossier kind (`building_permit`, …): the items that apply to it.
    pub kind: String,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, number | string | boolean>"))]
    pub inputs: FeeInputs,
}

/// One line of an assessment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeLine {
    pub code: String,
    pub label: String,
    pub group: String,
    pub legal_basis: String,
    /// How it was found, in words: “152 m² × 3,75 TL (Konut, 151–200 m²)”.
    pub basis: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub amount: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub vat: i64,
    pub before_decision: bool,
}

/// Where an assessment is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeeAssessmentState {
    /// Issued (tahakkuk): to be paid.
    Issued,
    Paid,
    Cancelled,
}

/// A tariff applied to a dossier.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeAssessment {
    pub id: String,
    pub schedule_id: String,
    pub schedule_name: String,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, number | string | boolean>"))]
    pub inputs: FeeInputs,
    pub lines: Vec<FeeLine>,
    /// Kuruş, VAT included.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub total: i64,
    pub state: FeeAssessmentState,
    pub due_on: Option<String>,
    pub issued_at: String,
    pub issued_by_name: String,
    pub paid_at: Option<String>,
    pub receipt_no: Option<String>,
    pub paid_recorded_by_name: Option<String>,
    pub cancel_reason: Option<String>,
    /// How to pay, from the tariff.
    pub payment_note: Option<String>,
}

/// What a preview gives: the lines and why some inputs are missing, nothing saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeePreview {
    pub schedule_id: String,
    pub schedule_name: String,
    /// The inputs the dossier's kind needs, to ask.
    pub parameters: Vec<FeeParameter>,
    pub lines: Vec<FeeLine>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub total: i64,
    /// Inputs not given or not valid, by key, with why.
    pub missing: BTreeMap<String, String>,
}

/// `POST /v1/dossiers/{id}/fees/{fee}/paid`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeePaid {
    /// The receipt (makbuz) number.
    pub receipt_no: String,
    /// `YYYY-MM-DD`; absent: today.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub paid_on: Option<String>,
}

/// `POST /v1/dossiers/{id}/fees/{fee}/cancel`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeeCancel {
    pub reason: String,
}
