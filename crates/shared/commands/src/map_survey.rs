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
    AcceptMethod, BaselineSolution, SolutionKind, SurveyAdjustment, SurveyAntenna, SurveyFileFormat, SurveyGrid,
    SurveyKnown, SurveyReceiver, SurveyResult, TimeSystem,
};

pub const MAP_SURVEY_QUERY: &str = "map.survey.query";
pub const MAP_SURVEY_QUERY_VERSION: u32 = 1;
pub const MAP_SURVEY_EDIT: &str = "map.survey.edit";
pub const MAP_SURVEY_EDIT_VERSION: u32 = 1;
pub const MAP_SURVEY_NETWORK: &str = "map.survey.network";
pub const MAP_SURVEY_NETWORK_VERSION: u32 = 1;

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

/// A baseline vector as it stands (docs/adr/1740): its record's marks, its length (m) and processed standard
/// deviations σX σY σZ (m), whether it is excluded and why, whether it goes into the adjustment, its file and its id
/// there, and the network adjustments (`da-N`) made with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyBaselineRow {
    pub id: String,
    pub campaign: String,
    pub from: String,
    pub to: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub excluded: Option<String>,
    pub adjust: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<SurveyFileRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub adjustments: Vec<String>,
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

/// Output of `map.survey.query` v1: the archive in its order (with its baselines, known points and network
/// adjustments, docs/adr/1740), the object asked about, how many accepted points and uses are out of date, whether it
/// was deep, and the revision read.
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyBaselineRow>>"))]
    pub baselines: Vec<SurveyBaselineRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyKnown>>"))]
    pub known: Vec<SurveyKnown>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyAdjustment>>"))]
    pub adjustments: Vec<SurveyAdjustment>,
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
    /// A solution `cz-N`, a baseline `bz-N` or a known point `bn-N` left out, with why; it stays in the record.
    Exclude { solution: String, reason: String },
    /// An excluded solution, baseline or known point taken back.
    Include { solution: String },
    /// Whether a solution or a baseline goes into the adjustment.
    Adjust { solution: String, adjust: bool },
    /// Baseline vectors `bz-N` of a campaign read from a file (an absolute path): a GVX 1.0 file (NGS) or KentOS's
    /// baseline text (`B;from;to;dX;dY;dZ;xx;xy;xz;yy;yz;zz`); each goes into the adjustment.
    Baselines { campaign: String, file: String },
    /// One baseline vector `bz-N` typed in: from mark to mark, ΔX ΔY ΔZ (m; ECEF, the antenna heights applied) and its
    /// covariance xx, xy, xz, yy, yz, zz (m²).
    Baseline {
        campaign: String,
        from: String,
        to: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        d: [f64; 3],
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m2")))]
        covariance: [f64; 6],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        start: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        end: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        solution: Option<BaselineSolution>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        recipe: Option<String>,
    },
    /// A known point `bn-N`: a mark's given coordinates in a geographic frame at an epoch (the measurement's); held
    /// fixed, or weighted by `sigma` north, east, up (m); with `velocity` (the frame's Earth-centred X, Y, Z, m/yr; a
    /// station's published velocity) it is carried from its epoch to the measurement epoch by it (docs/adr/2520).
    Known {
        mark: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        lat: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        lon: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        height: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
        srid: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
        epoch: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        sigma: Option<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
        velocity: Option<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        source: Option<String>,
    },
    /// The network adjusted as `map.survey.network` adjusts it and accepted (docs/adr/1740): `da-N`, and each mark that
    /// is not a known point accepted (its object on `layer`, the active layer when not given) or made again, in one
    /// step. Refused while a baseline is a blunder or the model test fails.
    Network {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        campaign: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        scale: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        class: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        rules: Option<GnssRules>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        hold: Option<String>,
        /// The measurement epoch the known points are moved to (docs/adr/2120), as `map.survey.network` takes it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
        epoch: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        layer: Option<String>,
        /// Each point's error ellipse drawn in the same step (docs/adr/2960).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        ellipses: Option<NetworkEllipses>,
    },
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
/// “Dengeleme seçimi”, “Noktayı kabul et”, “Noktayı yeniden üret”, “Dayanan kaydı”, “Arşivden çıkar”, “Baz ekle”,
/// “Bilinen nokta ekle”, “Ağ dengelemesini kabul et”).
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_map_project`, then by operation:
/// `unknown_survey_record`, `unknown_work_item`, `invalid_survey_text`, `invalid_survey_value`, `invalid_file_path`,
/// `file_not_found`, `unreadable_gnss_file`, `nothing_to_set`, `survey_in_use`, `unknown_frame`, `no_observations`,
/// `no_ellipsoidal_height`, `solution_excluded`, `mark_mismatch`, `method_count`, `sigma_missing`,
/// `solutions_differ`, `antenna_unknown`, `antenna_eccentric`, `already_accepted`, `layer_not_found`, `layer_locked`,
/// `no_entities`, `invalid_uid`, `entity_not_found`, `survey_full`, the network's (`no_baselines`, `datum_missing`,
/// `known_differ`, `unknown_rule_class`, `network_too_large`, `network_disconnected`, `network_blunder`,
/// `network_test_failed`) and the coordinate chain's own (`epoch_unsupported`, `velocity_missing`, `geoid_missing` …)
/// when a point cannot be moved into the project.
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
    /// The error ellipses a network drew (docs/adr/2960).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ellipses: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    /// The records a file brought (`baselines`) or the accepted points a network made or made again (`network`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub records: Vec<String>,
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Whose standard deviations a class's baseline limit is checked on: the processor's (the vector's covariance by the
/// variance factor) or the free adjustment's (the adjusted vector's, by its m0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BaselineSigmaOf {
    Processed,
    Adjusted,
}

/// A class of a GNSS network rule package (docs/adr/1740): a baseline component's standard deviation's limit, a
/// constant (m) and parts per million of its length, and whose; a station's limits after the adjustment held by the
/// known points, σφ and σλ (as north and east, m) and σh (m); and, when the class says one, the tests' confidence
/// 1 − α (otherwise Baarda's α₀ = 0.001).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GnssRuleClass {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub constant: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "ppm")))]
    pub ppm: f64,
    pub baseline_of: BaselineSigmaOf,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub confidence: Option<f64>,
    /// The largest |1 − λ| (ppm) of the 3D similarity between the known points' given coordinates and the free stage's
    /// (docs/adr/2160; BÖHHBÜY Madde 15(1)(c): C1's 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "ppm")))]
    pub similarity: Option<f64>,
    /// The articles the class's numbers come from.
    pub articles: String,
}

/// A GNSS network rule package: who issues it, the document, when it holds, its release and its classes. KentOS's is
/// BÖHHBÜY 2018's (`catalog/v1/rules/gnss-ag-bohhbuy-2018.json`); none of its numbers is a constant in code.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GnssRules {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub release: u32,
    pub jurisdiction: String,
    pub source: String,
    pub effective: String,
    pub classes: Vec<GnssRuleClass>,
}

/// KentOS's GNSS network rule package (BÖHHBÜY 2018, Articles 15, 20 and 22).
pub fn gnss_rules_default() -> Option<GnssRules> {
    serde_json::from_str(include_str!("../../../../catalog/v1/rules/gnss-ag-bohhbuy-2018.json")).ok()
}

/// Input of `map.survey.network` v1 (docs/adr/1740): the project's GNSS network adjusted from its archive, reading
/// only. The baselines that go into the adjustment (of `campaign` alone when given), the variance factor their
/// covariances are multiplied by (`scale`, 1 when not given), the known point the free stage holds (`hold`, the first
/// otherwise) and the rule class checked (`class` of `rules`, KentOS's BÖHHBÜY 2018 when not given).
///
/// The measurement epoch (docs/adr/2120): `epoch` when given, else the mean of the baselines' observation mid-times
/// when every one has its start and end, else none. With one, every known point needs its epoch and one at another
/// epoch is moved to it (BÖHHBÜY Madde 15(1)(a)): by its own velocity when it has one (docs/adr/2520), else along the
/// campaign's velocity grid; without one the known points are in one frame and epoch, the network's.
///
/// Refusals, in this order: `not_a_map_project`, `unknown_survey_record`, `invalid_survey_value`,
/// `unknown_rule_class`, `invalid_epoch`, `no_baselines`, `datum_missing`, `known_differ`, `unknown_frame`,
/// `epoch_unsupported` (a known point at another epoch without its own velocity, the campaign without a velocity
/// grid) and the grid's own refusals, `network_too_large`, `network_disconnected`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapSurveyNetwork {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub campaign: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<GnssRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hold: Option<String>,
    /// The measurement epoch (decimal year, 1900–2100) the known points are moved to (docs/adr/2120).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
}

/// Where the network's epoch comes from (docs/adr/2120): given, the baselines' observation times, the known points'.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NetworkEpochFrom {
    Given,
    Baselines,
    Known,
}

/// What carried a known point to the measurement epoch (docs/adr/2520): its own velocity, or the campaign's
/// velocity grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum KnownMovedBy {
    Velocity,
    Grid,
}

/// A known point moved to the measurement epoch (BÖHHBÜY Madde 15(1)(a), docs/adr/2120): by its own velocity or along
/// the campaign's velocity grid (docs/adr/2520), its epoch, the years it moved and how far east, north and up (m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KnownMoved {
    pub id: String,
    pub mark: String,
    pub by: KnownMovedBy,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub years: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub shift: [f64; 3],
}

/// One stage of the adjustment: the marks held fixed, m0 (none without degrees of freedom), the degrees of freedom,
/// vᵀPv, the χ² quantile (95 %) and whether the model test passed, the test value above which a baseline is a
/// blunder, how many are and the worst one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkStage {
    pub held: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub m0: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub dof: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub omega: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub chi2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed: Option<bool>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub limit: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub blunders: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub worst: Option<String>,
}

/// A baseline's test in a stage: its residual (computed − observed) in the local east, north, up of its start (m),
/// its largest component test value (none: not checked) and what it says (`ok`, `blunder`, `uncontrolled`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BaselineTest {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub v: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub w: Option<f64>,
    pub flag: String,
}

/// A baseline of the network: its marks and length (m), its processed standard deviations σX σY σZ (by the variance
/// factor), the free stage's of the adjusted vector, its test in each stage, and the class's limit and whether it
/// holds (when a class is checked).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkBaseline {
    pub id: String,
    pub from: String,
    pub to: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub adjusted: [f64; 3],
    pub free: BaselineTest,
    pub constrained: BaselineTest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<bool>,
}

/// A mark of the network after the stage held by the known points: its known point (`bn-N`) and whether it is held
/// fixed, its latitude, longitude (degrees) and ellipsoidal height (m) in the network's frame and epoch, its standard
/// deviations north, east, up (m), its accepted point (`kn-N`), where it goes in the project (a mark that is not a
/// known point, when the coordinate chain reaches the project), how far that is from where its accepted point is now
/// (project units), whether the class's limits hold for it, and its confidence region when it is not held fixed
/// (docs/adr/2560).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkStation {
    pub mark: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub known: Option<String>,
    pub held: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub lat: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub lon: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub point: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub dx: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub dy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub dz: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ellipse: Option<ErrorEllipse>,
}

/// The error ellipses an accepted network draws (docs/adr/2960): on `layer`, each the point's horizontal confidence
/// region magnified `scale` times (1 to 100 000; 1000 draws a centimetre as ten metres), its major axis along the
/// map's direction of its azimuth (the meridian's convergence taken off), recorded as made from the points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkEllipses {
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: f64,
}

/// The magnifications an ellipse may be drawn at.
pub const ELLIPSE_SCALES: (f64, f64) = (1.0, 100_000.0);

/// A mark's confidence region (docs/adr/2560) from its east, north, up covariance as the stage held by the known points
/// gives it (by m0²), at a confidence (the class's, else Madde 15(1)(d)'s 0.95): the horizontal error ellipse's semi-
/// major and semi-minor axes (m, √χ²₂ times the 2×2 covariance's eigenvalues' roots) and the major axis's direction
/// (degrees clockwise from north, 0 to 180; none for a circle, the axes within a millionth of the major one), and the
/// height's half-interval (m, √χ²₁ times σ up).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ErrorEllipse {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub confidence: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub major: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub minor: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub azimuth: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub up: f64,
}

/// What a rule class said: the package and its release, the class, the tests' confidence when the class gives one,
/// whose baseline standard deviations were checked, whether all holds, how many baselines and stations do not, and
/// whether the similarity's scale holds when the class limits it (docs/adr/2160).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkRules {
    pub package: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub release: u32,
    pub class: String,
    pub articles: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub confidence: Option<f64>,
    pub baseline_of: BaselineSigmaOf,
    pub passed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub baselines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stations: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub similarity: Option<bool>,
}

/// A known point's residual in the similarity (docs/adr/2160): east, north, up (m) at its given place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SimilarityResidual {
    pub id: String,
    pub mark: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub enu: [f64; 3],
}

/// The 3D similarity between the known points' given coordinates (at the measurement epoch) and the free stage's
/// (BÖHHBÜY Madde 15(1)(c), docs/adr/2160): given = T + λ·R·free, R the small rotations in the position vector
/// convention. The translation (m), the rotations (arc-seconds), the scale's difference λ − 1 and its standard deviation
/// (ppm), the degrees of freedom 3n − 7, the scale's test value |λ − 1|/σ against Student's t at the confidence, whether
/// the scale agrees, and the residuals.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NetworkSimilarity {
    pub known: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub translation: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "arcsec")))]
    pub rotation: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "ppm")))]
    pub scale: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "ppm")))]
    pub sigma: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub dof: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub t: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub quantile: f64,
    pub agrees: bool,
    pub residuals: Vec<SimilarityResidual>,
}

/// Output of `map.survey.network` v1: the network's frame and epoch (the measurement epoch, or its known points'), where
/// the epoch comes from and the known points moved to it (docs/adr/2120), the known points it rests on,
/// the baselines in the archive's order, the marks in the order the baselines name them, both stages, the 3D
/// similarity of the known points (three or more, more than seven coordinates; docs/adr/2160), the class's result, the
/// adjustment (`da-N`) this same network was accepted with when its points still carry it, whether it can be accepted
/// and, if not, why, and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapSurveyNetworked {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    pub epoch_from: NetworkEpochFrom,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<KnownMoved>>"))]
    pub moved: Vec<KnownMoved>,
    pub known: Vec<String>,
    pub baselines: Vec<NetworkBaseline>,
    pub stations: Vec<NetworkStation>,
    pub free: NetworkStage,
    pub constrained: NetworkStage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub similarity: Option<NetworkSimilarity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<NetworkRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub current: Option<String>,
    pub acceptable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub why: Vec<String>,
    pub revision: String,
}
