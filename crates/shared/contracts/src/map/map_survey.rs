//! A map project's GNSS campaign archive (plan/map-cad.md MAP-03, docs/adr/1340): its survey campaigns, the sessions
//! measured on each mark (the receiver, the antenna and its height, the original file and the positions read from it),
//! the providers' solutions (a receiver's, a processed baseline's, a PPP service's), the points the project accepted
//! from them, and the objects made from accepted points. A raw observation, a provider's solution and an accepted point
//! carry ids of their own; a solution is excluded with a reason, never deleted; an accepted point keeps its older
//! results when it is made again. A project setting (`.kcad` schema 194, `mapSurvey`), a map project's only; a campaign
//! names the map's work item (docs/adr/1200) by its id alone.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::sheets::is_sha256_text;
use crate::{EntityId, GnssPoint};

/// At most this many campaigns, sessions, solutions, accepted points and uses in a project.
pub const MAX_SURVEY_CAMPAIGNS: usize = 1000;
pub const MAX_SURVEY_SESSIONS: usize = 10_000;
pub const MAX_SURVEY_SOLUTIONS: usize = 100_000;
pub const MAX_SURVEY_POINTS: usize = 10_000;
pub const MAX_SURVEY_USES: usize = 10_000;
/// At most this many baseline vectors, known points and network adjustments (docs/adr/1740).
pub const MAX_SURVEY_BASELINES: usize = 100_000;
pub const MAX_SURVEY_KNOWN: usize = 10_000;
pub const MAX_SURVEY_ADJUSTMENTS: usize = 1000;
/// At most this many baseline sessions, and baselines in one (docs/adr/4920): a session's full covariance is factored
/// whole, 3n × 3n.
pub const MAX_BASELINE_SESSIONS: usize = 10_000;
pub const MAX_SESSION_BASELINES: usize = 100;
/// The fastest velocity a point may be given (m/yr): plates move centimetres a year.
pub const MOST_POINT_VELOCITY: f64 = 1.0;
/// The longest baseline component (m): more than the Earth's diameter is not a baseline.
pub const MOST_BASELINE_COMPONENT: f64 = 2.0e7;
/// At most this many positions in one session: a day at one a second.
pub const MAX_SESSION_OBSERVATIONS: usize = 86_400;
/// At most this many older results an accepted point keeps.
pub const MAX_POINT_HISTORY: usize = 100;
/// At most this many objects in one use.
pub const MAX_USE_OBJECTS: usize = 100_000;
/// The longest name, mark or other short text; a recipe; an exclusion's reason (characters).
pub const MOST_SURVEY_TEXT: usize = 200;
pub const MOST_RECIPE_TEXT: usize = 2000;
pub const MOST_REASON_TEXT: usize = 500;
/// The highest antenna and the largest eccentricity (m).
pub const MOST_ANTENNA_HEIGHT: f64 = 100.0;

/// A time system a session's times are in (RINEX 3.05's codes, and UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TimeSystem {
    Gps,
    Utc,
    Glo,
    Gal,
    Bdt,
    Qzs,
    Irn,
}

impl TimeSystem {
    /// RINEX's three letters (`GPS`, `GLO` …); UTC is NMEA's.
    pub fn from_rinex(code: &str) -> Option<TimeSystem> {
        Some(match code {
            "GPS" => TimeSystem::Gps,
            "UTC" => TimeSystem::Utc,
            "GLO" => TimeSystem::Glo,
            "GAL" => TimeSystem::Gal,
            "BDT" => TimeSystem::Bdt,
            "QZS" => TimeSystem::Qzs,
            "IRN" => TimeSystem::Irn,
            _ => return None,
        })
    }
}

/// Where an antenna's height was measured to: the antenna reference point straight up (RINEX's `DELTA H`), or slant to
/// the antenna's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AntennaReference {
    #[default]
    Arp,
    Slant,
}

impl AntennaReference {
    fn is_arp(&self) -> bool {
        *self == AntennaReference::Arp
    }
}

/// A session's antenna: its type and serial, the height measured and to where; a slant height with the maker's
/// radius and the measuring point's height above the ARP; the eccentricity east and north (m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyAntenna {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub serial: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    #[serde(default, skip_serializing_if = "AntennaReference::is_arp")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<AntennaReference>"))]
    pub reference: AntennaReference,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub east: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub north: Option<f64>,
}

impl SurveyAntenna {
    /// The ARP's height straight above the mark (m): the measured one, or √(S² − R²) − offset of a slant one.
    pub fn vertical(&self) -> f64 {
        match self.reference {
            AntennaReference::Arp => self.height,
            AntennaReference::Slant => {
                let r = self.radius.unwrap_or(0.0);
                (self.height * self.height - r * r).sqrt() - self.offset.unwrap_or(0.0)
            }
        }
    }

    /// Whether its ARP is off the mark sideways.
    pub fn eccentric(&self) -> bool {
        self.east.is_some_and(|e| e != 0.0) || self.north.is_some_and(|n| n != 0.0)
    }

    /// Why it cannot be a session's: the field and the words.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        for (field, t) in [("type", &self.kind), ("serial", &self.serial)] {
            if t.as_deref().is_some_and(|t| !survey_text_ok(t, MOST_SURVEY_TEXT)) {
                return Some((field, format!("boşlukla başlayıp bitmeyen 1–{MOST_SURVEY_TEXT} karakter olmalı")));
            }
        }
        if !(self.height.is_finite() && (0.0..=MOST_ANTENNA_HEIGHT).contains(&self.height)) {
            return Some(("height", format!("anten yüksekliği 0 ile {MOST_ANTENNA_HEIGHT} m arasında olmalı")));
        }
        match (self.reference, self.radius) {
            (AntennaReference::Slant, Some(r)) if r.is_finite() && r > 0.0 && r < self.height => {}
            (AntennaReference::Slant, _) => {
                return Some(("radius", "eğik yükseklikte antenin yarıçapı 0'dan büyük, yükseklikten küçük olmalı".into()));
            }
            (AntennaReference::Arp, Some(_)) => {
                return Some(("radius", "yarıçap yalnız eğik ölçülen yükseklikte verilir".into()));
            }
            (AntennaReference::Arp, None) => {}
        }
        if self.reference == AntennaReference::Arp && self.offset.is_some() {
            return Some(("offset", "ölçü noktasının yüksekliği yalnız eğik ölçülen yükseklikte verilir".into()));
        }
        for (field, v) in [("offset", self.offset), ("east", self.east), ("north", self.north)] {
            if v.is_some_and(|v| !(v.is_finite() && v.abs() <= MOST_ANTENNA_HEIGHT)) {
                return Some((field, format!("en çok {MOST_ANTENNA_HEIGHT} m olan sonlu bir sayı olmalı")));
            }
        }
        if self.reference == AntennaReference::Slant && (self.vertical().is_nan() || self.vertical() < 0.0) {
            return Some(("offset", "düşey anten yüksekliği eksi çıkıyor".into()));
        }
        None
    }
}

/// A session's receiver: its type, serial and firmware.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyReceiver {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub serial: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub firmware: Option<String>,
}

/// What a session's original file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SurveyFileFormat {
    Rinex,
    Nmea,
    Gpx,
    /// A processing report (a solution's).
    Report,
    /// NGS's GNSS Vector Exchange 1.0 (baseline vectors; docs/adr/1740).
    Gvx,
    /// KentOS's baseline text: `B;from;to;dX;dY;dZ;xx;xy;xz;yy;yz;zz` (docs/adr/1380 §5).
    Text,
}

/// A file the archive rests on: an absolute path and the SHA-256 of the content read (its revision).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyFile {
    pub path: String,
    pub revision: String,
    pub format: SurveyFileFormat,
}

/// What a RINEX observation file's header said that is kept nowhere else: its version (as written), the marker's
/// number, the interval (s) and the approximate position (ECEF XYZ, m).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RinexFacts {
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marker_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub interval: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub approximate: Option<[f64; 3]>,
}

/// A RINEX observation file's header as read (docs/adr/1340 §4): what fills a session.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RinexHeader {
    pub facts: RinexFacts,
    pub marker: Option<String>,
    pub receiver: Option<SurveyReceiver>,
    pub antenna: Option<SurveyAntenna>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub time_system: Option<TimeSystem>,
}

/// A survey campaign.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyCampaign {
    /// `kp-N`.
    pub id: String,
    pub name: String,
    /// The map's work item it is (`is-N`, docs/adr/1200), by id alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub work: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operator: Option<String>,
    /// The epoch its accepted points are given in (decimal year); none: the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    /// The velocity grid that moves its solutions to that epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub velocity: Option<SurveyGrid>,
}

/// A grid by its name and SHA-256 (lower-case hex), as the device's grid library keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyGrid {
    pub name: String,
    pub sha256: String,
}

/// A session: one occupation of one mark.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveySession {
    /// `os-N`.
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
    pub operator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<SurveyFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rinex: Option<RinexFacts>,
    /// The positions an NMEA log or a GPX file gave, as the reader read them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<GnssPoint>>"))]
    pub observations: Vec<GnssPoint>,
}

/// What made a solution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SolutionKind {
    /// The receiver's own real-time solution, from its log.
    Receiver,
    /// A baseline, PPK or static solution processed in office software.
    Processed,
    /// A precise point positioning service's.
    Ppp,
}

/// A provider's solution of a session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveySolution {
    /// `cz-N`.
    pub id: String,
    pub session: String,
    pub kind: SolutionKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub lat: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub lon: f64,
    /// The ellipsoidal height (m).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    /// The geographic system its coordinates are in (an EPSG code of the registry).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    /// The vertical ARP height its maker took off (m); 0: the position is the antenna's.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub antenna_applied: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub epochs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub satellites: Option<u32>,
    /// The largest HDOP (no unit: geometry, not metres).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub hdop: Option<f64>,
    /// σ north, east, up (m).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: Option<[f64; 3]>,
    /// The full covariance NN, NE, NU, EE, EU, UU (m²).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m2")))]
    pub covariance: Option<[f64; 6]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub correction: Option<String>,
    /// The largest correction age (s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub age: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub report: Option<SurveyFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recipe: Option<String>,
    /// A receiver solution's positions: the first and last place in the session's (both in) and the fixes taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub from: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub to: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub fixes: Vec<String>,
    /// Why it is left out; present: excluded (it stays in the record).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub excluded: Option<String>,
    /// It goes into the adjustment (when not excluded).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub adjust: bool,
}

/// How an accepted point is made of its solutions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AcceptMethod {
    /// One solution.
    #[default]
    Single,
    /// The arithmetic mean of two or more.
    Mean,
    /// Each component weighted by 1/σ².
    Weighted,
    /// A network adjustment of baseline vectors held by known points (docs/adr/1740): no solutions, its `da-N`.
    Network,
}

impl AcceptMethod {
    /// Its name in the window and the attributes.
    pub fn title(self) -> &'static str {
        match self {
            AcceptMethod::Single => "Tek çözüm",
            AcceptMethod::Mean => "Ortalama",
            AcceptMethod::Weighted => "Ağırlıklı ortalama",
            AcceptMethod::Network => "Ağ dengelemesi",
        }
    }
}

/// A session's vertical antenna height an accepted point's result was made with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AntennaUsed {
    pub session: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
}

/// What an accepted point is, once made: its solutions and method, the mark's position in the solutions' frame and
/// epoch, the epoch decision it was moved to, each session's vertical antenna height, σ when known, the project's x,
/// y and z, whether the coordinate chain was complete, and why it was made again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyResult {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub solutions: Vec<String>,
    pub method: AcceptMethod,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub target_epoch: Option<f64>,
    pub antenna: Vec<AntennaUsed>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: Option<[f64; 3]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
    /// `complete` or `limited` (docs/adr/1230).
    pub chain: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub reasons: Vec<String>,
    /// The network adjustment `da-N` it was made by (method `network`; docs/adr/1740).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub adjustment: Option<String>,
    /// Its velocity, read from the velocity grid the user gave (docs/adr/4920), never made up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub velocity: Option<PointVelocity>,
}

/// A point's velocity east, north, up (m/yr) and the velocity grid it was read from, by its name and SHA-256
/// (docs/adr/4920): the grid's displacement over one year at the point, by the coordinate chain itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointVelocity {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub enu: [f64; 3],
    pub grid: SurveyGrid,
}

/// A point accepted into the project: its mark, its object, its result now and its older ones (oldest first).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyPoint {
    /// `kn-N`.
    pub id: String,
    pub mark: String,
    pub uid: EntityId,
    pub result: SurveyResult,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyResult>>"))]
    pub history: Vec<SurveyResult>,
}

/// An accepted point's revision a use was made from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointRevision {
    pub point: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// How a baseline vector was made (GVX 1.0's SOLUTION_TYPE).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BaselineSolution {
    /// Processed in office software from both receivers' observations.
    PostProcessed,
    /// A real-time vector from one base station.
    SingleBaseRtk,
    /// A real-time vector from a network's correctors.
    NetworkRtk,
}

/// A baseline vector `bz-N` (docs/adr/1740): from mark to mark, ΔX, ΔY, ΔZ in Earth-centred, Earth-fixed axes (m)
/// with the antenna heights applied by its processor, and its covariance xx, xy, xz, yy, yz, zz (m²); its campaign,
/// when it was observed (GPS time), how and with what it was made, the file it was read from and its id there; left
/// out with a reason, never deleted; whether it goes into the network adjustment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyBaseline {
    /// `bz-N`.
    pub id: String,
    pub campaign: String,
    pub from: String,
    pub to: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub d: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m2")))]
    pub covariance: [f64; 6],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub solution: Option<BaselineSolution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub software: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub epochs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub satellites: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<SurveyFile>,
    /// Its id in the file (GVX's `ID`, the text's line).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recipe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub excluded: Option<String>,
    pub adjust: bool,
}

/// A known point `bn-N` (docs/adr/1740): a mark's given coordinates (a TUSAGA-Aktif or TUTGA station, an older C1
/// point) in a geographic frame at an epoch, the network's datum: held fixed, or with standard deviations north,
/// east, up (m) weighted; its own velocity in the frame's Earth-centred X, Y, Z (m/yr, from its epoch; a station's
/// published velocity, docs/adr/2520); where they come from; left out with a reason, never deleted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyKnown {
    /// `bn-N`.
    pub id: String,
    pub mark: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub velocity: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub excluded: Option<String>,
}

/// A network adjustment the project accepted, `da-N` (docs/adr/1740): what it was made of (its baselines, its known
/// points, the one the free stage held), with what (the variance factor, the rule class and package checked), what
/// it said (both stages' m0, degrees of freedom and model test; whether the rules held) and in which frame and epoch.
/// The accepted points it made name it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyAdjustment {
    /// `da-N`.
    pub id: String,
    pub baselines: Vec<String>,
    pub known: Vec<String>,
    pub hold: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules_passed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub free_m0: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub free_dof: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub m0: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub dof: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    /// The baseline sessions `bo-N` whose full covariance it was adjusted with (docs/adr/4920); none: each baseline's
    /// own 3×3 alone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub sessions: Vec<String>,
}

/// A baseline session `bo-N` (docs/adr/4920): baselines processed together from simultaneous observations of common
/// satellites, as GVX 1.0 keeps a `SESSION`: its id in the file, its time, its baselines and the cross-correlation
/// blocks between them. A block's nine numbers go row by row, the rows the `row` baseline's X, Y, Z and the columns the
/// `col` baseline's; a complete session of n baselines has n(n − 1)/2 blocks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyBaselineSession {
    /// `bo-N`.
    pub id: String,
    pub campaign: String,
    /// Its id in the file (GVX's `SESSION ID`).
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end: Option<String>,
    pub baselines: Vec<String>,
    pub blocks: Vec<CorrelationBlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<SurveyFile>,
}

/// The correlations between two baselines of a session, X, Y, Z by X, Y, Z, row by row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorrelationBlock {
    pub row: String,
    pub col: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub r: [f64; 9],
}

/// The limits a known point's place may lie from its mark's accepted point (docs/adr/4520), kept in the project with
/// where they come from (docs/adr/4920): a regulation's article, an administration's stricter specification
/// (BÖHHBÜY Madde 103(1)(c)).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyTolerance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub vertical: Option<f64>,
    pub source: String,
}

impl SurveyTolerance {
    /// Why the limits cannot be kept: the field and the words.
    pub fn problem(&self) -> Option<(&'static str, &'static str)> {
        if self.horizontal.is_none() && self.vertical.is_none() {
            return Some(("horizontal", "yatay ya da düşey sınırdan en az biri verilmeli"));
        }
        for (field, v) in [("horizontal", self.horizontal), ("vertical", self.vertical)] {
            if v.is_some_and(|v| !(v.is_finite() && v > 0.0)) {
                return Some((field, "sınır 0'dan büyük sonlu bir uzunluk olmalı"));
            }
        }
        if !survey_text_ok(&self.source, MOST_REASON_TEXT) {
            return Some(("source", "kaynak boşlukla başlayıp bitmeyen 1–500 karakter olmalı"));
        }
        None
    }
}

/// Objects made from accepted points (a surface, a contour): the operation, the objects and each point's revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyUse {
    /// `kb-N`.
    pub id: String,
    pub operation: String,
    pub uids: Vec<EntityId>,
    pub points: Vec<PointRevision>,
}

/// A map project's GNSS campaign archive.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapSurvey {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyCampaign>>"))]
    pub campaigns: Vec<SurveyCampaign>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveySession>>"))]
    pub sessions: Vec<SurveySession>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveySolution>>"))]
    pub solutions: Vec<SurveySolution>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyPoint>>"))]
    pub points: Vec<SurveyPoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyUse>>"))]
    pub uses: Vec<SurveyUse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyBaseline>>"))]
    pub baselines: Vec<SurveyBaseline>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyKnown>>"))]
    pub known: Vec<SurveyKnown>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyAdjustment>>"))]
    pub adjustments: Vec<SurveyAdjustment>,
    /// Baseline sessions with their cross-correlations (docs/adr/4920).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SurveyBaselineSession>>"))]
    pub baseline_sessions: Vec<SurveyBaselineSession>,
    /// The known points' place limits kept in the project (docs/adr/4920).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tolerance: Option<SurveyTolerance>,
}

impl MapSurvey {
    /// Whether it says nothing (and is not written).
    pub fn is_empty(&self) -> bool {
        self.campaigns.is_empty()
            && self.sessions.is_empty()
            && self.solutions.is_empty()
            && self.points.is_empty()
            && self.uses.is_empty()
            && self.baselines.is_empty()
            && self.known.is_empty()
            && self.adjustments.is_empty()
            && self.baseline_sessions.is_empty()
            && self.tolerance.is_none()
    }

    pub fn baseline_session(&self, id: &str) -> Option<&SurveyBaselineSession> {
        self.baseline_sessions.iter().find(|s| s.id == id)
    }

    /// The baseline session a baseline is in.
    pub fn session_of(&self, baseline: &str) -> Option<&SurveyBaselineSession> {
        self.baseline_sessions.iter().find(|s| s.baselines.iter().any(|b| b == baseline))
    }

    /// Whether it holds what schema 248 brought (docs/adr/4920): baseline sessions, an adjustment made with them, the
    /// place limits kept, a point's velocity.
    pub fn has_sessions(&self) -> bool {
        !self.baseline_sessions.is_empty()
            || self.tolerance.is_some()
            || self.adjustments.iter().any(|a| !a.sessions.is_empty())
            || self
                .points
                .iter()
                .any(|p| std::iter::once(&p.result).chain(&p.history).any(|r| r.velocity.is_some()))
    }

    pub fn campaign(&self, id: &str) -> Option<&SurveyCampaign> {
        self.campaigns.iter().find(|c| c.id == id)
    }

    pub fn session(&self, id: &str) -> Option<&SurveySession> {
        self.sessions.iter().find(|s| s.id == id)
    }

    pub fn solution(&self, id: &str) -> Option<&SurveySolution> {
        self.solutions.iter().find(|s| s.id == id)
    }

    pub fn point(&self, id: &str) -> Option<&SurveyPoint> {
        self.points.iter().find(|p| p.id == id)
    }

    /// The mark a solution is of (its session's).
    pub fn mark_of(&self, solution: &SurveySolution) -> Option<&str> {
        self.session(&solution.session).map(|s| s.mark.as_str())
    }

    /// The accepted point whose result now uses `solution`.
    pub fn point_using(&self, solution: &str) -> Option<&SurveyPoint> {
        self.points.iter().find(|p| p.result.solutions.iter().any(|s| s == solution))
    }

    pub fn baseline(&self, id: &str) -> Option<&SurveyBaseline> {
        self.baselines.iter().find(|b| b.id == id)
    }

    pub fn known_point(&self, id: &str) -> Option<&SurveyKnown> {
        self.known.iter().find(|k| k.id == id)
    }

    pub fn adjustment(&self, id: &str) -> Option<&SurveyAdjustment> {
        self.adjustments.iter().find(|a| a.id == id)
    }

    /// Whether it holds what schema 207 brought (docs/adr/2520): a known point's own velocity.
    pub fn has_velocities(&self) -> bool {
        self.known.iter().any(|k| k.velocity.is_some())
    }

    /// Whether it holds what schema 199 brought (docs/adr/1740): baselines, known points, network adjustments, a point
    /// made by a network.
    pub fn has_network(&self) -> bool {
        !self.baselines.is_empty()
            || !self.known.is_empty()
            || !self.adjustments.is_empty()
            || self.points.iter().any(|p| {
                std::iter::once(&p.result)
                    .chain(&p.history)
                    .any(|r| r.method == AcceptMethod::Network || r.adjustment.is_some())
            })
    }

    /// The accepted point whose result now was made by the adjustment `adjustment`.
    pub fn point_by(&self, adjustment: &str) -> Option<&SurveyPoint> {
        self.points.iter().find(|p| p.result.adjustment.as_deref() == Some(adjustment))
    }

    /// What is wrong with it, by the field's path and in words; none when it is right.
    pub fn problem(&self) -> Option<(String, String)> {
        map_survey_problem(self)
    }
}

/// `prefix-N` with N a positive number written without leading zeros.
pub fn survey_numbered(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|n| {
        !n.is_empty() && !n.starts_with('0') && n.len() <= 9 && n.bytes().all(|b| b.is_ascii_digit())
    })
}

/// 1–`most` characters, no space at either end.
pub fn survey_text_ok(s: &str, most: usize) -> bool {
    let n = s.chars().count();
    n > 0 && n <= most && s.trim() == s
}

/// A time `YYYY-MM-DDThh:mm:ss[.fraction]Z` as its parts: the date and the seconds of the day as text, sortable.
pub fn survey_time(text: &str) -> Option<(u32, u32, u32, u32, u32, u32, String)> {
    let b = text.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let body = text.strip_suffix('Z')?;
    let num = |s: &str| (s.bytes().all(|c| c.is_ascii_digit())).then(|| s.parse::<u32>().ok()).flatten();
    let (y, mo, d) = (num(&body[0..4])?, num(&body[5..7])?, num(&body[8..10])?);
    let (h, mi, s) = (num(&body[11..13])?, num(&body[14..16])?, num(&body[17..19])?);
    let fraction = match &body[19..] {
        "" => String::new(),
        f => {
            let digits = f.strip_prefix('.')?;
            if digits.is_empty() || digits.len() > 9 || !digits.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            digits.trim_end_matches('0').to_owned()
        }
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1900..=2100).contains(&y) || mo == 0 || mo > 12 || d == 0 || d > days[mo as usize - 1] || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    Some((y, mo, d, h, mi, s, fraction))
}

/// Seconds since 1900-01-01T00:00:00 of a time [`survey_time`] reads (for the replay's steps).
pub fn survey_seconds(text: &str) -> Option<f64> {
    let (y, mo, d, h, mi, s, fraction) = survey_time(text)?;
    let mut days = 0i64;
    for year in 1900..y {
        days += if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 366 } else { 365 };
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let months = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    days += months[..mo as usize - 1].iter().map(|&m| i64::from(m)).sum::<i64>();
    days += i64::from(d) - 1;
    let whole = days * 86_400 + i64::from(h) * 3600 + i64::from(mi) * 60 + i64::from(s);
    let frac = if fraction.is_empty() { 0.0 } else { format!("0.{fraction}").parse::<f64>().ok()? };
    Some(whole as f64 + frac)
}

fn fail(path: String, words: &str) -> Option<(String, String)> {
    Some((path, words.to_owned()))
}

fn opt_text(v: &Option<String>, most: usize) -> bool {
    v.as_deref().is_none_or(|t| survey_text_ok(t, most))
}

fn file_problem(f: &SurveyFile) -> Option<&'static str> {
    if f.path.trim().is_empty() || f.path.trim() != f.path {
        return Some("dosyanın yolu boş olamaz, başında ve sonunda boşluk olmaz");
    }
    if !is_sha256_text(&f.revision) {
        return Some("dosyanın revizyonu 64 küçük onaltılık haneli SHA-256 olmalı");
    }
    None
}

/// Whether a covariance NN, NE, NU, EE, EU, UU is positive definite (its leading minors above zero) and finite.
pub fn covariance_ok(c: &[f64; 6]) -> bool {
    let [nn, ne, nu, ee, eu, uu] = *c;
    if !c.iter().all(|v| v.is_finite()) {
        return false;
    }
    let m2 = nn * ee - ne * ne;
    let m3 = nn * (ee * uu - eu * eu) - ne * (ne * uu - eu * nu) + nu * (ne * eu - ee * nu);
    nn > 0.0 && m2 > 0.0 && m3 > 0.0
}

/// Why a solution's numbers cannot be kept: the field and the words.
pub fn solution_numbers_problem(s: &SurveySolution) -> Option<(&'static str, String)> {
    if !(s.lat.is_finite() && (-90.0..=90.0).contains(&s.lat)) {
        return Some(("lat", "enlem −90 ile 90 derece arasında olmalı".into()));
    }
    if !(s.lon.is_finite() && (-180.0..=180.0).contains(&s.lon)) {
        return Some(("lon", "boylam −180 ile 180 derece arasında olmalı".into()));
    }
    if !(s.height.is_finite() && s.height.abs() <= 100_000.0) {
        return Some(("height", "elipsoit yüksekliği en çok 100 km olan sonlu bir sayı olmalı".into()));
    }
    if s.srid == 0 {
        return Some(("srid", "çözümün çerçevesi bir EPSG kodu olmalı".into()));
    }
    if let Some(why) = s.epoch.and_then(crate::epoch_problem) {
        return Some(("epoch", why));
    }
    if !(s.antenna_applied.is_finite() && (0.0..=MOST_ANTENNA_HEIGHT).contains(&s.antenna_applied)) {
        return Some(("antennaApplied", format!("uygulanmış anten yüksekliği 0 ile {MOST_ANTENNA_HEIGHT} m arasında olmalı")));
    }
    if s.hdop.is_some_and(|h| !(h.is_finite() && h >= 0.0)) {
        return Some(("hdop", "HDOP eksi olmayan sonlu bir sayı olmalı".into()));
    }
    if s.sigma.is_some_and(|g| !g.iter().all(|v| v.is_finite() && *v > 0.0)) {
        return Some(("sigma", "σ değerleri 0'dan büyük sonlu sayılar olmalı (m)".into()));
    }
    if s.covariance.is_some_and(|c| !covariance_ok(&c)) {
        return Some(("covariance", "kovaryans pozitif tanımlı olmalı (köşegen, 2×2 ve 3×3 öncü küçükler 0'dan büyük)".into()));
    }
    if s.age.is_some_and(|a| !(a.is_finite() && a >= 0.0)) {
        return Some(("age", "düzeltme yaşı eksi olmayan sonlu bir sayı olmalı (s)".into()));
    }
    None
}

/// Why `survey` cannot be a project's, by the field's path and in words; none when it can.
pub fn map_survey_problem(survey: &MapSurvey) -> Option<(String, String)> {
    let limits = [
        ("campaigns", survey.campaigns.len(), MAX_SURVEY_CAMPAIGNS),
        ("sessions", survey.sessions.len(), MAX_SURVEY_SESSIONS),
        ("solutions", survey.solutions.len(), MAX_SURVEY_SOLUTIONS),
        ("points", survey.points.len(), MAX_SURVEY_POINTS),
        ("uses", survey.uses.len(), MAX_SURVEY_USES),
        ("baselines", survey.baselines.len(), MAX_SURVEY_BASELINES),
        ("known", survey.known.len(), MAX_SURVEY_KNOWN),
        ("adjustments", survey.adjustments.len(), MAX_SURVEY_ADJUSTMENTS),
        ("baselineSessions", survey.baseline_sessions.len(), MAX_BASELINE_SESSIONS),
    ];
    for (field, n, most) in limits {
        if n > most {
            return fail(field.into(), &format!("en çok {most} kayıt olur"));
        }
    }
    let mut ids = BTreeSet::new();
    for (i, c) in survey.campaigns.iter().enumerate() {
        let at = |f: &str| format!("campaigns[{i}].{f}");
        if !survey_numbered(&c.id, "kp-") || !ids.insert(c.id.as_str()) {
            return fail(at("id"), "arazi ölçümünün kimliği kp-1, kp-2 … biçiminde ve tek olmalı");
        }
        if !survey_text_ok(&c.name, MOST_SURVEY_TEXT) || !opt_text(&c.operator, MOST_SURVEY_TEXT) {
            return fail(at("name"), "ad ve operatör boşlukla başlayıp bitmeyen 1–200 karakter olmalı");
        }
        if c.work.as_deref().is_some_and(|w| !survey_numbered(w, "is-")) {
            return fail(at("work"), "bağlı harita işi öğesi is-N kimliğiyle verilir");
        }
        if let Some(why) = c.epoch.and_then(crate::epoch_problem) {
            return fail(at("epoch"), &why);
        }
        if let Some(g) = &c.velocity
            && (!survey_text_ok(&g.name, MOST_SURVEY_TEXT) || !is_sha256_text(&g.sha256))
        {
            return fail(at("velocity"), "hız ızgarasının adı boş olmamalı, SHA-256'sı 64 küçük onaltılık hane olmalı");
        }
    }
    for (i, s) in survey.sessions.iter().enumerate() {
        let at = |f: &str| format!("sessions[{i}].{f}");
        if !survey_numbered(&s.id, "os-") || !ids.insert(s.id.as_str()) {
            return fail(at("id"), "oturumun kimliği os-1, os-2 … biçiminde ve tek olmalı");
        }
        if survey.campaign(&s.campaign).is_none() {
            return fail(at("campaign"), "oturumun arazi ölçümü arşivde olmalı");
        }
        if !survey_text_ok(&s.mark, MOST_SURVEY_TEXT) || !opt_text(&s.operator, MOST_SURVEY_TEXT) {
            return fail(at("mark"), "işaret ve operatör boşlukla başlayıp bitmeyen 1–200 karakter olmalı");
        }
        let start = s.start.as_deref().map(survey_time);
        let end = s.end.as_deref().map(survey_time);
        if matches!(start, Some(None)) || matches!(end, Some(None)) {
            return fail(at("start"), "zaman YYYY-AA-GGTss:dd:sn[.kesir]Z biçiminde geçerli bir an olmalı");
        }
        if let (Some(Some(a)), Some(Some(b))) = (&start, &end)
            && b < a
        {
            return fail(at("end"), "bitiş başlangıçtan önce olamaz");
        }
        if let Some(r) = &s.receiver
            && (!survey_text_ok(&r.kind, MOST_SURVEY_TEXT) || !opt_text(&r.serial, MOST_SURVEY_TEXT) || !opt_text(&r.firmware, MOST_SURVEY_TEXT))
        {
            return fail(at("receiver"), "alıcının türü, seri no'su ve sürümü boşlukla başlayıp bitmeyen 1–200 karakter olmalı");
        }
        if let Some((field, why)) = s.antenna.as_ref().and_then(SurveyAntenna::problem) {
            return fail(at(&format!("antenna.{field}")), &why);
        }
        if let Some(f) = &s.file {
            if let Some(why) = file_problem(f) {
                return fail(at("file"), why);
            }
            if !matches!(f.format, SurveyFileFormat::Rinex | SurveyFileFormat::Nmea | SurveyFileFormat::Gpx) {
                return fail(at("file.format"), "oturumun dosyası RINEX, NMEA ya da GPX olur");
            }
        }
        if s.observations.len() > MAX_SESSION_OBSERVATIONS {
            return fail(at("observations"), &format!("bir oturumda en çok {MAX_SESSION_OBSERVATIONS} konum olur"));
        }
        if !s.observations.is_empty() && !s.file.as_ref().is_some_and(|f| f.format != SurveyFileFormat::Rinex) {
            return fail(at("observations"), "ham ölçümler yalnız NMEA ya da GPX dosyasından okunur");
        }
        if s.observations.iter().any(|o| !(o.lat.is_finite() && o.lon.is_finite())) {
            return fail(at("observations"), "ham ölçümün enlemi ve boylamı sonlu olmalı");
        }
    }
    for (i, s) in survey.solutions.iter().enumerate() {
        let at = |f: &str| format!("solutions[{i}].{f}");
        if !survey_numbered(&s.id, "cz-") || !ids.insert(s.id.as_str()) {
            return fail(at("id"), "çözümün kimliği cz-1, cz-2 … biçiminde ve tek olmalı");
        }
        let Some(session) = survey.session(&s.session) else {
            return fail(at("session"), "çözümün oturumu arşivde olmalı");
        };
        if let Some((field, why)) = solution_numbers_problem(s) {
            return fail(at(field), &why);
        }
        if !opt_text(&s.fix, MOST_SURVEY_TEXT) || !opt_text(&s.correction, MOST_SURVEY_TEXT) {
            return fail(at("fix"), "fix ve düzeltme kaynağı boşlukla başlayıp bitmeyen 1–200 karakter olmalı");
        }
        if !opt_text(&s.recipe, MOST_RECIPE_TEXT) {
            return fail(at("recipe"), "yeniden işleme tarifi boşlukla başlayıp bitmeyen 1–2000 karakter olmalı");
        }
        if !opt_text(&s.excluded, MOST_REASON_TEXT) {
            return fail(at("excluded"), "dışlama nedeni boşlukla başlayıp bitmeyen 1–500 karakter olmalı");
        }
        if let Some(r) = &s.report
            && (file_problem(r).is_some() || r.format != SurveyFileFormat::Report)
        {
            return fail(at("report"), "işleme raporu mutlak yolu ve SHA-256'sıyla verilir");
        }
        let n = session.observations.len() as u64;
        match (s.from, s.to) {
            (None, None) => {}
            (Some(a), Some(b)) if a <= b && u64::from(b) < n => {}
            _ => return fail(at("from"), "ham ölçüm aralığı oturumun konumları içinde olmalı"),
        }
    }
    if let Some(found) = network_problem(survey, &mut ids) {
        return Some(found);
    }
    let mut marks = BTreeSet::new();
    let mut uids = std::collections::HashSet::new();
    for (i, p) in survey.points.iter().enumerate() {
        let at = |f: &str| format!("points[{i}].{f}");
        if !survey_numbered(&p.id, "kn-") || !ids.insert(p.id.as_str()) {
            return fail(at("id"), "kabul edilen noktanın kimliği kn-1, kn-2 … biçiminde ve tek olmalı");
        }
        if !survey_text_ok(&p.mark, MOST_SURVEY_TEXT) || !marks.insert(p.mark.as_str()) {
            return fail(at("mark"), "bir işaretin tek kabul edilen noktası olur");
        }
        if p.uid.is_nil() || !uids.insert(p.uid) {
            return fail(at("uid"), "noktanın nesnesi boş olamaz ve iki noktanın olamaz");
        }
        if p.history.len() > MAX_POINT_HISTORY {
            return fail(at("history"), &format!("en çok {MAX_POINT_HISTORY} eski sonuç tutulur"));
        }
        if p.result.revision as usize != p.history.len() + 1
            || p.history.iter().enumerate().any(|(k, r)| r.revision as usize != k + 1)
        {
            return fail(at("result.revision"), "revizyonlar 1'den başlayıp sırayla artar; bugünkü sonuç sonuncudur");
        }
        if p.result.method == AcceptMethod::Network {
            if !p.result.solutions.is_empty() || p.result.adjustment.as_deref().and_then(|a| survey.adjustment(a)).is_none() {
                return fail(at("result.adjustment"), "ağ dengelemesiyle kabul edilen noktanın çözümü olmaz, arşivdeki dengelemesi olur");
            }
        } else if p.result.adjustment.is_some() {
            return fail(at("result.adjustment"), "yalnız ağ dengelemesiyle kabul edilen noktanın dengelemesi olur");
        } else if p.result.solutions.is_empty() {
            return fail(at("result.solutions"), "kabul edilen noktanın en az bir çözümü olur");
        }
        for s in &p.result.solutions {
            match survey.solution(s) {
                None => return fail(at("result.solutions"), "noktanın çözümleri arşivde olmalı"),
                Some(sol) if survey.mark_of(sol) != Some(p.mark.as_str()) => {
                    return fail(at("result.solutions"), "noktanın çözümleri noktanın işaretinin olmalı");
                }
                Some(_) => {}
            }
        }
        for r in std::iter::once(&p.result).chain(&p.history) {
            let nums = [r.lat, r.lon, r.height, r.x, r.y];
            if !nums.iter().all(|v| v.is_finite()) || r.z.is_some_and(|z| !z.is_finite()) {
                return fail(at("result"), "sonucun sayıları sonlu olmalı");
            }
            if r.chain != "complete" && r.chain != "limited" {
                return fail(at("result.chain"), "zincirin kararı complete ya da limited olur");
            }
        }
    }
    let mut objects = std::collections::HashSet::new();
    for (i, u) in survey.uses.iter().enumerate() {
        let at = |f: &str| format!("uses[{i}].{f}");
        if !survey_numbered(&u.id, "kb-") || !ids.insert(u.id.as_str()) {
            return fail(at("id"), "dayananın kimliği kb-1, kb-2 … biçiminde ve tek olmalı");
        }
        if !survey_text_ok(&u.operation, MOST_SURVEY_TEXT) {
            return fail(at("operation"), "işlem boşlukla başlayıp bitmeyen 1–200 karakter olmalı");
        }
        if u.uids.is_empty() || u.uids.len() > MAX_USE_OBJECTS || u.uids.iter().any(|x| x.is_nil() || !objects.insert(*x)) {
            return fail(at("uids"), "dayananın nesneleri boş olmayan, tek ve başka dayananda olmayan kimliklerdir");
        }
        if u.points.is_empty() {
            return fail(at("points"), "dayanan en az bir kabul edilen noktaya dayanır");
        }
        for pr in &u.points {
            match survey.point(&pr.point) {
                Some(p) if pr.revision >= 1 && pr.revision <= p.result.revision => {}
                _ => return fail(at("points"), "dayananın noktaları arşivde ve revizyonları geçerli olmalı"),
            }
        }
    }
    None
}

/// Why a baseline's numbers cannot be kept: the field and the words.
pub fn baseline_numbers_problem(b: &SurveyBaseline) -> Option<(&'static str, String)> {
    if !b.d.iter().all(|v| v.is_finite() && v.abs() <= MOST_BASELINE_COMPONENT) {
        return Some(("d", "bazın bileşenleri en çok 20 000 km olan sonlu sayılar olmalı (m)".into()));
    }
    if b.d.iter().all(|v| *v == 0.0) {
        return Some(("d", "bazın uzunluğu sıfır olamaz".into()));
    }
    if !covariance_ok(&b.covariance) {
        return Some(("covariance", "bazın kovaryansı pozitif tanımlı olmalı (köşegen, 2×2 ve 3×3 öncü küçükler 0'dan büyük)".into()));
    }
    None
}

/// Why a known point's numbers cannot be kept: the field and the words.
pub fn known_numbers_problem(k: &SurveyKnown) -> Option<(&'static str, String)> {
    if !(k.lat.is_finite() && (-90.0..=90.0).contains(&k.lat)) {
        return Some(("lat", "enlem −90 ile 90 derece arasında olmalı".into()));
    }
    if !(k.lon.is_finite() && (-180.0..=180.0).contains(&k.lon)) {
        return Some(("lon", "boylam −180 ile 180 derece arasında olmalı".into()));
    }
    if !(k.height.is_finite() && k.height.abs() <= 100_000.0) {
        return Some(("height", "elipsoit yüksekliği en çok 100 km olan sonlu bir sayı olmalı".into()));
    }
    if k.srid == 0 {
        return Some(("srid", "bilinen noktanın çerçevesi bir EPSG kodu olmalı".into()));
    }
    if let Some(why) = k.epoch.and_then(crate::epoch_problem) {
        return Some(("epoch", why));
    }
    if k.sigma.is_some_and(|g| !g.iter().all(|v| v.is_finite() && *v > 0.0)) {
        return Some(("sigma", "σ değerleri 0'dan büyük sonlu sayılar olmalı (m)".into()));
    }
    // docs/adr/2520 §1: a velocity carries the point from its epoch; stations move centimetres a year, so a component
    // above a metre a year is a unit's mistake (mm/yr written as m/yr), not a velocity.
    if k.velocity.is_some_and(|v| !v.iter().all(|c| c.is_finite() && c.abs() <= MAX_KNOWN_VELOCITY)) {
        return Some((
            "velocity",
            "hız yer merkezli VX, VY, VZ olarak yılda metredir; her bileşen en çok 1 m/yıl olan sonlu bir sayı olmalı".into(),
        ));
    }
    if k.velocity.is_some() && k.epoch.is_none() {
        return Some(("velocity", "hızı olan noktanın epoku verilmeli: hız noktayı o epoktan taşır".into()));
    }
    None
}

/// The largest velocity component a known point takes (m/yr, docs/adr/2520 §1): KentOS's bound, not a rule's.
pub const MAX_KNOWN_VELOCITY: f64 = 1.0;

/// The baselines', known points' and network adjustments' rules (docs/adr/1740); `ids` are the archive's ids so far.
fn network_problem<'a>(survey: &'a MapSurvey, ids: &mut BTreeSet<&'a str>) -> Option<(String, String)> {
    for (i, b) in survey.baselines.iter().enumerate() {
        let at = |f: &str| format!("baselines[{i}].{f}");
        if !survey_numbered(&b.id, "bz-") || !ids.insert(b.id.as_str()) {
            return fail(at("id"), "bazın kimliği bz-1, bz-2 … biçiminde ve tek olmalı");
        }
        if survey.campaign(&b.campaign).is_none() {
            return fail(at("campaign"), "bazın arazi ölçümü arşivde olmalı");
        }
        if !survey_text_ok(&b.from, MOST_SURVEY_TEXT) || !survey_text_ok(&b.to, MOST_SURVEY_TEXT) || b.from == b.to {
            return fail(at("to"), "bazın iki ucu boşlukla başlayıp bitmeyen 1–200 karakterlik iki ayrı işarettir");
        }
        if let Some((field, why)) = baseline_numbers_problem(b) {
            return fail(at(field), &why);
        }
        let start = b.start.as_deref().map(survey_time);
        let end = b.end.as_deref().map(survey_time);
        if matches!(start, Some(None)) || matches!(end, Some(None)) {
            return fail(at("start"), "zaman YYYY-AA-GGTss:dd:sn[.kesir]Z biçiminde geçerli bir an olmalı");
        }
        if let (Some(Some(a)), Some(Some(z))) = (&start, &end)
            && z < a
        {
            return fail(at("end"), "bitiş başlangıçtan önce olamaz");
        }
        if !opt_text(&b.software, MOST_SURVEY_TEXT) || !opt_text(&b.source, MOST_SURVEY_TEXT) {
            return fail(at("software"), "yazılım ve dosyadaki kimlik boşlukla başlayıp bitmeyen 1–200 karakter olmalı");
        }
        if !opt_text(&b.recipe, MOST_RECIPE_TEXT) {
            return fail(at("recipe"), "yeniden işleme tarifi boşlukla başlayıp bitmeyen 1–2000 karakter olmalı");
        }
        if !opt_text(&b.excluded, MOST_REASON_TEXT) {
            return fail(at("excluded"), "dışlama nedeni boşlukla başlayıp bitmeyen 1–500 karakter olmalı");
        }
        if let Some(f) = &b.file
            && (file_problem(f).is_some() || !matches!(f.format, SurveyFileFormat::Gvx | SurveyFileFormat::Text))
        {
            return fail(at("file"), "bazın dosyası mutlak yolu ve SHA-256'sıyla verilen bir GVX ya da baz metnidir");
        }
    }
    let mut held = BTreeSet::new();
    for (i, k) in survey.known.iter().enumerate() {
        let at = |f: &str| format!("known[{i}].{f}");
        if !survey_numbered(&k.id, "bn-") || !ids.insert(k.id.as_str()) {
            return fail(at("id"), "bilinen noktanın kimliği bn-1, bn-2 … biçiminde ve tek olmalı");
        }
        if !survey_text_ok(&k.mark, MOST_SURVEY_TEXT) {
            return fail(at("mark"), "işaret boşlukla başlayıp bitmeyen 1–200 karakter olmalı");
        }
        if k.excluded.is_none() && !held.insert(k.mark.as_str()) {
            return fail(at("mark"), "bir işaretin dışlanmamış tek bilinen noktası olur");
        }
        if let Some((field, why)) = known_numbers_problem(k) {
            return fail(at(field), &why);
        }
        if !opt_text(&k.source, MOST_REASON_TEXT) {
            return fail(at("source"), "kaynak boşlukla başlayıp bitmeyen 1–500 karakter olmalı");
        }
        if !opt_text(&k.excluded, MOST_REASON_TEXT) {
            return fail(at("excluded"), "dışlama nedeni boşlukla başlayıp bitmeyen 1–500 karakter olmalı");
        }
    }
    for (i, a) in survey.adjustments.iter().enumerate() {
        let at = |f: &str| format!("adjustments[{i}].{f}");
        if !survey_numbered(&a.id, "da-") || !ids.insert(a.id.as_str()) {
            return fail(at("id"), "dengelemenin kimliği da-1, da-2 … biçiminde ve tek olmalı");
        }
        let mut seen = BTreeSet::new();
        if a.baselines.is_empty() || a.baselines.iter().any(|b| survey.baseline(b).is_none() || !seen.insert(b.as_str())) {
            return fail(at("baselines"), "dengelemenin bazları arşivde, tek tek ve en az bir tane olmalı");
        }
        if a.known.is_empty() || a.known.iter().any(|k| survey.known_point(k).is_none() || !seen.insert(k.as_str())) {
            return fail(at("known"), "dengelemenin bilinen noktaları arşivde, tek tek ve en az bir tane olmalı");
        }
        if !a.known.contains(&a.hold) {
            return fail(at("hold"), "serbest dengelemede tutulan nokta dengelemenin bilinen noktalarından olmalı");
        }
        if !(a.scale.is_finite() && a.scale > 0.0) {
            return fail(at("scale"), "varyans çarpanı 0'dan büyük sonlu bir sayı olmalı");
        }
        if !opt_text(&a.class, MOST_SURVEY_TEXT) || !opt_text(&a.rules, MOST_SURVEY_TEXT) || a.class.is_some() != a.rules.is_some() {
            return fail(at("class"), "sınıf ve kural paketi birlikte, boşlukla başlayıp bitmeyen 1–200 karakter olur");
        }
        if a.rules_passed.is_some() != a.class.is_some() {
            return fail(at("rulesPassed"), "kuralların sonucu yalnız sınıf denetlendiyse yazılır");
        }
        if [a.free_m0, a.m0].iter().flatten().any(|m| !(m.is_finite() && *m >= 0.0)) {
            return fail(at("m0"), "m0 eksi olmayan sonlu bir sayı olmalı");
        }
        if a.srid == 0 {
            return fail(at("srid"), "dengelemenin çerçevesi bir EPSG kodu olmalı");
        }
        if let Some(why) = a.epoch.and_then(crate::epoch_problem) {
            return fail(at("epoch"), &why);
        }
    }
    sessions_problem(survey, ids)
}

/// docs/adr/4920: the baseline sessions, the kept place limits, the adjustments' sessions, the points' velocities.
fn sessions_problem<'a>(survey: &'a MapSurvey, ids: &mut BTreeSet<&'a str>) -> Option<(String, String)> {
    let mut taken = BTreeSet::new();
    for (i, s) in survey.baseline_sessions.iter().enumerate() {
        let at = |f: &str| format!("baselineSessions[{i}].{f}");
        if !survey_numbered(&s.id, "bo-") || !ids.insert(s.id.as_str()) {
            return fail(at("id"), "baz oturumunun kimliği bo-1, bo-2 … biçiminde ve tek olmalı");
        }
        if survey.campaign(&s.campaign).is_none() {
            return fail(at("campaign"), "baz oturumunun arazi ölçümü arşivde olmalı");
        }
        if !survey_text_ok(&s.name, MOST_SURVEY_TEXT) {
            return fail(at("name"), "oturumun dosyadaki kimliği boşlukla başlayıp bitmeyen 1–200 karakter olmalı");
        }
        let start = s.start.as_deref().map(survey_time);
        let end = s.end.as_deref().map(survey_time);
        if matches!(start, Some(None)) || matches!(end, Some(None)) {
            return fail(at("start"), "zaman YYYY-AA-GGTss:dd:sn[.kesir]Z biçiminde geçerli bir an olmalı");
        }
        if let (Some(Some(a)), Some(Some(z))) = (&start, &end)
            && z < a
        {
            return fail(at("end"), "bitiş başlangıçtan önce olamaz");
        }
        if s.baselines.len() < 2 || s.baselines.len() > MAX_SESSION_BASELINES {
            return fail(at("baselines"), &format!("bir baz oturumunda 2 ile {MAX_SESSION_BASELINES} arasında baz olur"));
        }
        for b in &s.baselines {
            if survey.baseline(b).is_none_or(|x| x.campaign != s.campaign) {
                return fail(at("baselines"), "oturumun bazları arşivde ve oturumun arazi ölçümünde olmalı");
            }
            if !taken.insert(b.as_str()) {
                return fail(at("baselines"), "bir baz en çok bir oturumda olur");
            }
        }
        let n = s.baselines.len();
        if s.blocks.len() > n * (n - 1) / 2 {
            return fail(at("blocks"), "n bazlı oturumda en çok n(n − 1)/2 korelasyon bloğu olur");
        }
        let mut pairs = BTreeSet::new();
        for (j, b) in s.blocks.iter().enumerate() {
            let at = |f: &str| format!("baselineSessions[{i}].blocks[{j}].{f}");
            if b.row == b.col || !s.baselines.contains(&b.row) || !s.baselines.contains(&b.col) {
                return fail(at("col"), "bloğun iki bazı oturumun iki ayrı bazı olmalı");
            }
            let pair = if b.row < b.col { (b.row.as_str(), b.col.as_str()) } else { (b.col.as_str(), b.row.as_str()) };
            if !pairs.insert(pair) {
                return fail(at("col"), "iki baz arasında tek korelasyon bloğu olur");
            }
            if !b.r.iter().all(|r| r.is_finite() && (-1.0..=1.0).contains(r)) {
                return fail(at("r"), "korelasyonlar −1 ile 1 arasında olmalı");
            }
        }
        if let Some(f) = &s.file
            && (file_problem(f).is_some() || f.format != SurveyFileFormat::Gvx)
        {
            return fail(at("file"), "oturumun dosyası mutlak yolu ve SHA-256'sıyla verilen bir GVX dosyasıdır");
        }
    }
    for (i, a) in survey.adjustments.iter().enumerate() {
        if a.sessions.iter().any(|s| survey.baseline_session(s).is_none()) {
            return fail(format!("adjustments[{i}].sessions"), "dengelemenin baz oturumları arşivde olmalı");
        }
    }
    if let Some((field, why)) = survey.tolerance.as_ref().and_then(SurveyTolerance::problem) {
        return fail(format!("tolerance.{field}"), why);
    }
    for (i, p) in survey.points.iter().enumerate() {
        for r in std::iter::once(&p.result).chain(&p.history) {
            if let Some(v) = &r.velocity
                && (!v.enu.iter().all(|x| x.is_finite() && x.abs() <= MOST_POINT_VELOCITY)
                    || !survey_text_ok(&v.grid.name, MOST_SURVEY_TEXT)
                    || !is_sha256_text(&v.grid.sha256))
            {
                return fail(
                    format!("points[{i}].result.velocity"),
                    "hız en çok 1 m/yıl olan sonlu sayılar, ızgaranın adı ve SHA-256'sıyla olmalı",
                );
            }
        }
    }
    None
}

/// The next free `prefix-N` among `ids`.
pub fn next_survey_id<'a>(ids: impl Iterator<Item = &'a str>, prefix: &str) -> String {
    let most = ids
        .filter_map(|id| id.strip_prefix(prefix).and_then(|n| n.parse::<u64>().ok()))
        .max()
        .unwrap_or(0);
    format!("{prefix}{}", most + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn antenna(height: f64) -> SurveyAntenna {
        SurveyAntenna {
            kind: Some("TRM59800.00 NONE".into()),
            serial: None,
            height,
            reference: AntennaReference::Arp,
            radius: None,
            offset: None,
            east: None,
            north: None,
        }
    }

    #[test]
    fn a_slant_height_is_made_vertical() {
        let mut a = antenna(1.6);
        a.reference = AntennaReference::Slant;
        a.radius = Some(0.12);
        a.offset = Some(0.05);
        assert_eq!(a.problem(), None);
        assert_eq!(a.vertical(), (1.6f64 * 1.6 - 0.12 * 0.12).sqrt() - 0.05);
        a.radius = None;
        assert_eq!(a.problem().map(|p| p.0), Some("radius"));
        let mut b = antenna(1.5);
        b.offset = Some(0.1);
        assert_eq!(b.problem().map(|p| p.0), Some("offset"));
        assert_eq!(antenna(120.0).problem().map(|p| p.0), Some("height"));
    }

    #[test]
    fn times_are_read_and_ordered() {
        assert!(survey_time("2026-10-04T09:31:10.00Z").is_some());
        assert_eq!(survey_time("2026-10-04T09:31:10.00Z"), survey_time("2026-10-04T09:31:10Z"));
        assert!(survey_time("2026-02-30T00:00:00Z").is_none());
        assert!(survey_time("2026-10-04 09:31:10Z").is_none());
        assert_eq!(survey_seconds("1900-01-02T00:00:01.5Z"), Some(86_401.5));
    }

    #[test]
    fn a_covariance_must_be_positive_definite() {
        assert!(covariance_ok(&[1e-4, 0.0, 0.0, 1e-4, 0.0, 4e-4]));
        assert!(!covariance_ok(&[1e-4, 2e-4, 0.0, 1e-4, 0.0, 4e-4]));
        assert!(!covariance_ok(&[-1e-4, 0.0, 0.0, 1e-4, 0.0, 4e-4]));
    }

    #[test]
    fn the_archive_keeps_its_rules() {
        let survey = MapSurvey {
            campaigns: vec![SurveyCampaign {
                id: "kp-1".into(),
                name: "GNSS 2026-11".into(),
                work: Some("is-1".into()),
                operator: None,
                epoch: Some(2026.84),
                velocity: None,
            }],
            sessions: vec![SurveySession {
                id: "os-1".into(),
                campaign: "kp-1".into(),
                mark: "N1".into(),
                start: Some("2026-11-03T09:00:00Z".into()),
                end: Some("2026-11-03T08:00:00Z".into()),
                time_system: Some(TimeSystem::Gps),
                receiver: None,
                antenna: Some(antenna(1.5)),
                operator: None,
                file: None,
                rinex: None,
                observations: Vec::new(),
            }],
            ..MapSurvey::default()
        };
        assert_eq!(survey.problem().map(|p| p.0), Some("sessions[0].end".into()));
        let mut ok = survey.clone();
        ok.sessions[0].end = Some("2026-11-03T10:00:00Z".into());
        assert_eq!(ok.problem(), None);
        let mut twice = ok.clone();
        twice.sessions[0].id = "kp-1".into();
        assert_eq!(twice.problem().map(|p| p.0), Some("sessions[0].id".into()));
        assert_eq!(next_survey_id(["kp-3", "kp-1"].into_iter(), "kp-"), "kp-4");
    }
}
