//! A municipality's figures for a period (docs/adr/9000 §22; plan BUL-20): what came in, what was decided and how
//! fast, what waits and what is late, by kind, month and person in charge; its fees. Also as a workbook
//! (`…/statistics/export`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// `GET /v1/tenants/{tenant}/portal/statistics?from=&to=`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PortalStatistics {
    /// The period, both days in it (YYYY-MM-DD, Turkey's calendar).
    pub from: String,
    pub to: String,
    /// Applications sent for the first time in the period.
    pub received: u32,
    /// Applications approved or rejected in the period.
    pub decided: u32,
    pub approved: u32,
    pub rejected: u32,
    pub withdrawn: u32,
    /// Applications open now (sent, in review, waiting for corrections).
    pub open_now: u32,
    /// Of those waiting for the municipality, the ones past their legal deadline now.
    pub overdue_now: u32,
    /// Days from the first sending to the decision, of the ones decided in the period.
    pub average_decision_days: Option<f64>,
    pub median_decision_days: Option<f64>,
    /// Rounds (packages sent) per decided application.
    pub average_rounds: Option<f64>,
    pub transfers_sent: u32,
    pub transfers_completed: u32,
    /// Kuruş assessed in the period (not cancelled), and paid in it.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub fees_assessed: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub fees_paid: i64,
    pub by_kind: Vec<StatisticsKindRow>,
    pub by_month: Vec<StatisticsMonthRow>,
    pub by_assignee: Vec<StatisticsAssigneeRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatisticsKindRow {
    pub kind: String,
    pub label: String,
    pub received: u32,
    pub approved: u32,
    pub rejected: u32,
    pub average_decision_days: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatisticsMonthRow {
    /// YYYY-MM.
    pub month: String,
    pub received: u32,
    pub decided: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatisticsAssigneeRow {
    /// The person in charge; "Atanmamış" for none.
    pub name: String,
    pub open: u32,
    pub decided: u32,
}
