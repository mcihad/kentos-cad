//! KentOS Harita's GNSS campaign archive (plan/map-cad.md MAP-03, docs/adr/1340): `map.survey.query` reads a map
//! project's campaigns, sessions, solutions per mark (last measured, accepted, in the adjustment, excluded), accepted
//! points with whether they are up to date and why, and the objects made from them; `map.survey.edit` adds and changes
//! campaigns and sessions (from a RINEX, NMEA or GPX file), solves and records solutions, excludes them with a reason,
//! accepts a mark's point and makes it again, and records what was made from accepted points. The desktop's handlers
//! are `crates/native/application/src/map_survey*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/map.survey.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{
    AcceptMethod, SolutionKind, SurveyAntenna, SurveyFileFormat, SurveyGrid, SurveyReceiver, SurveyResult, TimeSystem,
};

pub const MAP_SURVEY_QUERY: &str = "map.survey.query";
pub const MAP_SURVEY_QUERY_VERSION: u32 = 1;
pub const MAP_SURVEY_EDIT: &str = "map.survey.edit";
pub const MAP_SURVEY_EDIT_VERSION: u32 = 1;

/// Input of `map.survey.query` v1: `deep` reads every session's file (its SHA-256 against its revision); without it a
/// file is only looked for. `uid` also says what that object is in the archive.
///
/// Refusals, in this order: `not_a_map_project`, `invalid_uid`, `entity_not_found`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapSurveyQuery {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub deep: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// How a session's file stands: there (looked for only), the content read, other content, not there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SurveyFileState {
    Found,
    Current,
    Changed,
    Missing,
}

/// A session's file as it stands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyFileRow {
    pub path: String,
    pub revision: String,
    pub format: SurveyFileFormat,
    pub state: SurveyFileState,
}

/// How many of a session's positions had a fix.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FixCount {
    pub fix: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
}

/// A recorded session replayed: its fixes in the order they first come (a position without one is `—`), how many
/// times an RTK fixed position was followed by another fix, how many steps between timed positions were not forward or
/// longer than twice the usual step, and the usual step (s).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyReplay {
    pub fixes: Vec<FixCount>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub float_drops: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub time_jumps: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub nominal: Option<f64>,
}

/// A campaign as it stands: whether the map's work item it names is still there, how many sessions it has.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyCampaignRow {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub work: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub work_missing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub velocity: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub sessions: u32,
}

/// A session as it stands: its antenna and vertical height, its file, how many positions it holds, the replay of
/// them, how many solutions it has.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveySessionRow {
    pub id: String,
    pub campaign: String,
    pub mark: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time_system: Option<TimeSystem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub receiver: Option<SurveyReceiver>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub antenna: Option<SurveyAntenna>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<SurveyFileRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub observations: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub replay: Option<SurveyReplay>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub solutions: u32,
}

/// A solution as it stands for its mark: last measured, accepted (the accepted point's result now uses it), in the
/// adjustment (marked and not excluded), excluded and why; the mark's height by its session's antenna now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveySolutionRow {
    pub id: String,
    pub session: String,
    pub kind: SolutionKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub lat: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub lon: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub antenna_applied: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub mark_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub epochs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: Option<[f64; 3]>,
    pub latest: bool,
    pub accepted: bool,
    pub adjusted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub excluded: Option<String>,
}

/// A mark: its sessions in the archive's order, its solutions, its accepted point.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyMarkRow {
    pub mark: String,
    pub sessions: Vec<String>,
    pub solutions: Vec<SurveySolutionRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub point: Option<String>,
}

/// How an accepted point or a use stands: up to date, out of date (why says it), its object gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SurveyState {
    Current,
    Stale,
    Missing,
}

/// An accepted point as it stands: its state and why, its result now, how many older results it keeps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyPointRow {
    pub id: String,
    pub mark: String,
    pub uid: String,
    pub state: SurveyState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub why: Vec<String>,
    pub result: SurveyResult,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub history: u32,
}

/// A use as it stands: how many of its objects the drawing holds of how many, its state and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyUseRow {
    pub id: String,
    pub operation: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub recorded: u32,
    pub state: SurveyState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub why: Vec<String>,
}

/// What one object is in the archive: the accepted point it is (with its solutions), or the uses it is in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyObject {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub point: Option<SurveyPointRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SurveySolutionRow>>", optional))]
    pub solutions: Vec<SurveySolutionRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub used: Option<SurveyUseRow>,
}

/// Output of `map.survey.query` v1: the archive in its order, the object asked about, how many accepted points and
/// uses are out of date, whether it was deep, and the revision read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapSurveyTree {
    pub campaigns: Vec<SurveyCampaignRow>,
    pub sessions: Vec<SurveySessionRow>,
    pub marks: Vec<SurveyMarkRow>,
    pub points: Vec<SurveyPointRow>,
    pub uses: Vec<SurveyUseRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub object: Option<SurveyObject>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    pub deep: bool,
    pub revision: String,
}

/// What `map.survey.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MapSurveyOperation {
    /// A new campaign `kp-N`: its name, the map's work item it is (a campaign item), its operator, its epoch decision
    /// and the velocity grid that moves solutions to it.
    Campaign {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        work: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        operator: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
        epoch: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        velocity: Option<SurveyGrid>,
    },
    /// A campaign's name, work item, operator, epoch decision or velocity grid.
    SetCampaign {
        campaign: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        work: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        operator: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
        epoch: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        velocity: Option<SurveyGrid>,
    },
    /// A new session `os-N` of a campaign. With a file (an absolute path): a RINEX observation file's header gives the
    /// mark, receiver, antenna, times and time system; an NMEA log or a GPX file its positions and their first and
    /// last times. What is given here wins over the file (warning `rinex_differs` says where they differ).
    Session {
        campaign: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        mark: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        file: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        start: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        end: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        time_system: Option<TimeSystem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        receiver: Option<SurveyReceiver>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        antenna: Option<SurveyAntenna>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        operator: Option<String>,
    },
    /// A session's mark (not while an accepted point uses it), times, receiver, antenna or operator. A new antenna
    /// height puts the accepted points made with the old one out of date.
    SetSession {
        session: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        mark: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        start: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        end: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        time_system: Option<TimeSystem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        receiver: Option<SurveyReceiver>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        antenna: Option<SurveyAntenna>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        operator: Option<String>,
    },
    /// The receiver's solution `cz-N` of a session: the mean of its positions `from`..`to` (both in; all when not
    /// given) whose fix is one of `fixes` (any when empty), each with an ellipsoidal height; its frame and the antenna
    /// height the receiver took off are the user's to say.
    Solve {
        session: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
        srid: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        antenna_applied: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
        epoch: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        from: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        to: Option<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        fixes: Vec<String>,
    },
    /// A solution `cz-N` from elsewhere (a processing report, a PPP service, a controller's record) with its quality,
    /// covariance, report file and recipe.
    Solution {
        session: String,
        kind: SolutionKind,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        lat: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        lon: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        height: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
        srid: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        antenna_applied: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
        epoch: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        fix: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        epochs: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        satellites: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        hdop: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        sigma: Option<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m2")))]
        covariance: Option<[f64; 6]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        correction: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
        age: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        report: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        recipe: Option<String>,
    },
    /// A solution left out, with why; it stays in the record.
    Exclude { solution: String, reason: String },
    /// An excluded solution taken back.
    Include { solution: String },
    /// Whether a solution goes into the adjustment.
    Adjust { solution: String, adjust: bool },
    /// A mark's point accepted: its object on `layer` (the active layer when not given) and `kn-N`, in one step.
    Accept {
        mark: String,
        solutions: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        method: Option<AcceptMethod>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        layer: Option<String>,
    },
    /// An accepted point made again with the antennas and epoch decision now (and, given, other solutions or method):
    /// its older result is kept, its object moves, its revision goes up.
    Regenerate {
        point: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        solutions: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        method: Option<AcceptMethod>,
    },
    /// Objects recorded as made from accepted points by `operation`, at their revisions now.
    Use { uids: Vec<String>, points: Vec<String>, operation: String },
    /// A record taken away: a campaign (with its sessions and solutions), a session (with its solutions), a solution,
    /// an accepted point's record (its object stays) or a use. What an accepted point or a use rests on is not.
    Remove { id: String },
}

/// Input of `map.survey.edit` v1: one operation, one undo step (“Arazi ölçümü ekle”, “Arazi ölçümünü değiştir”,
/// “GNSS oturumu ekle”, “Oturumu değiştir”, “Alıcı çözümü”, “Çözüm ekle”, “Çözümü dışla”, “Dışlamayı kaldır”,
/// “Dengeleme seçimi”, “Noktayı kabul et”, “Noktayı yeniden üret”, “Dayanan kaydı”, “Arşivden çıkar”).
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_map_project`, then by operation:
/// `unknown_survey_record`, `unknown_work_item`, `invalid_survey_text`, `invalid_survey_value`, `invalid_file_path`,
/// `file_not_found`, `unreadable_gnss_file`, `nothing_to_set`, `survey_in_use`, `unknown_frame`, `no_observations`,
/// `no_ellipsoidal_height`, `solution_excluded`, `mark_mismatch`, `method_count`, `sigma_missing`,
/// `solutions_differ`, `antenna_unknown`, `antenna_eccentric`, `already_accepted`, `layer_not_found`, `layer_locked`,
/// `no_entities`, `invalid_uid`, `entity_not_found`, `survey_full`, and the coordinate chain's own
/// (`epoch_unsupported`, `velocity_missing`, `geoid_missing` …) when the point cannot be moved into the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapSurveyEdit {
    pub operation: MapSurveyOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `map.survey.edit` v1 (and its plan): the record it made or changed, the accepted point's object and
/// revision, how many positions a session took from its file, how many records a remove took, how many accepted points
/// and uses are out of date after, and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapSurveyEdited {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub point_revision: Option<u32>,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub observations: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}
