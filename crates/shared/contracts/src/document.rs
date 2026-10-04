//! A whole drawing as stored or sent: project settings, the layer tree, the
//! objects and the project's styles. `DocumentSnapshotV1` is the v1 `.kcad`
//! (JSON); `DocumentSnapshotV2` is what a binary `.kcad` v2 holds
//! (docs/specs/kcad-v2.md), with every object's persistent id.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::crs::{CrsDefinition, DatumTransform, choices_problem};
use crate::entity::{Entity, Vec2};
use crate::identity::{EntityId, ProjectId};
use crate::layer::LayerNode;

pub const DOCUMENT_FORMAT: &str = "kentos.document";
pub const DOCUMENT_VERSION: u32 = 1;
/// The document schema inside a `.kcad` v2 file (docs/specs/kcad-v2.md §6.1).
pub const DOCUMENT_VERSION_2: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AreaUnit {
    M2,
    Donum,
    Ha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AngleUnit {
    Grad,
    Deg,
}

/// A project's type (`app/workspaces.ts`, docs/adr/0165): CAD or CBS, each
/// with its own scene, axes and ribbon; every command still runs in both.
/// `plan3d` and `disaster` are announced ("Yakında") and cannot be chosen yet;
/// a file naming them shows as CBS. There is no hybrid type any more.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Workspace {
    Cad,
    Gis,
    Plan3d,
    Disaster,
    /// The former Hibrit mode, as files written before project types name it
    /// (docs/adr/0165 §1): read and kept as written, so such a file reads and
    /// its v1 objects' ids derive as before (docs/adr/0014). It means the
    /// project's type is not asked yet; nothing chooses it, and the apps hold
    /// it as none (`ProjectSettings::project_type`).
    #[serde(rename = "hybrid")]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    LegacyHybrid,
}

/// The typeface of the text that is part of the drawing (text objects,
/// dimension values, labels; `app/appearance.ts` DRAWING_FONTS), bundled with
/// the app. A project setting: everyone who opens the project sees the same
/// letters. Files written before it have none (Barlow).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DrawingFont {
    Barlow,
    Arimo,
    Overpass,
    Quicksand,
    ArchitectsDaughter,
    CourierPrime,
    PlexMono,
}

/// The unit a local project's lengths are typed and read in (docs/adr/0165 §2).
/// Geometry stays in metres: the unit is where the user meets the numbers
/// (typed and shown lengths, coordinates and areas, DXF's `$INSUNITS`). A
/// project with a coordinate system has the system's unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DrawingUnit {
    Mm,
    Cm,
    #[default]
    M,
}

impl DrawingUnit {
    /// How many of the unit make a metre.
    pub fn per_metre(self) -> f64 {
        match self {
            Self::Mm => 1000.0,
            Self::Cm => 100.0,
            Self::M => 1.0,
        }
    }

    /// The unit's mark: `mm`, `cm`, `m`.
    pub fn mark(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::M => "m",
        }
    }
}

/// Project settings (`ProjectSettingsData`): saved with the drawing, the same for everyone who opens it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectSettings {
    pub srid: u32,
    pub length_decimals: u32,
    pub area_decimals: u32,
    pub area_unit: AreaUnit,
    pub angle_unit: AngleUnit,
    /// Plot scale denominator (1:1000 → 1000).
    pub plot_scale: f64,
    /// The project's type; none while it is not asked (files written before
    /// types). The former Hibrit mode reads as written and means the same
    /// (see [`ProjectSettings::project_type`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub workspace: Option<Workspace>,
    /// Absent in files written before drawing typefaces (read as Barlow).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_font: Option<DrawingFont>,
    /// A local project's unit (docs/adr/0165 §2); absent: metres. Only a
    /// project without a coordinate system (SRID 0) has another.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_unit: Option<DrawingUnit>,
    /// The project's second coordinate system (docs/adr/0167 §1): its
    /// coordinates are shown beside the project's own; absent: none. Never
    /// the project's own system, never a local project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub second_srid: Option<u32>,
    /// The project's own coordinate system when it is a definition
    /// (docs/adr/0168 §1); `srid` is then 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub custom_crs: Option<CrsDefinition>,
    /// The second system when it is a definition (instead of `second_srid`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub second_custom_crs: Option<CrsDefinition>,
    /// The project's datum choices (docs/adr/0168 §3); none: EPSG's ways.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DatumTransform>>", optional))]
    pub datum_transforms: Vec<DatumTransform>,
    /// The project's survey constants and tolerances (docs/adr/0169 §3);
    /// absent: k = [`REFRACTION`] and no tolerance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub survey: Option<SurveySettings>,
}

/// The refraction coefficient of trigonometric heights when a project names
/// none (docs/adr/0169 §3; the owner's choice, 4 October 2026).
pub const REFRACTION: f64 = 0.13;

/// The project's survey constants and tolerances (docs/adr/0169 §3): the
/// refraction coefficient k of trigonometric heights, and the greatest
/// differences a field book's two faces are checked against. Angles are in
/// radians, lengths in metres. An absent tolerance is not checked; the
/// differences are still shown.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveySettings {
    /// k, within [−1, 1]; absent: [`REFRACTION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refraction: Option<f64>,
    /// The two faces' horizontal reading difference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub face_hz: Option<f64>,
    /// The vertical index error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<f64>,
    /// The two faces' slope distance difference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub face_slope: Option<f64>,
}

impl SurveySettings {
    /// k: the project's, or [`REFRACTION`].
    pub fn refraction(&self) -> f64 {
        self.refraction.unwrap_or(REFRACTION)
    }

    /// Whether `k` is one a project may name: finite, within [−1, 1].
    pub fn refraction_holds(k: f64) -> bool {
        k.is_finite() && (-1.0..=1.0).contains(&k)
    }

    /// Whether `t` is a tolerance: finite and above zero.
    pub fn tolerance_holds(t: f64) -> bool {
        t.is_finite() && t > 0.0
    }

    /// What is wrong with the settings as a file holds them: none of them,
    /// k out of [−1, 1], a tolerance not above zero; none when they hold.
    pub fn problem(&self) -> Option<String> {
        if *self == Self::default() {
            return Some("ölçme ayarları boş; ayarı olmayan proje alanı yazmaz".to_owned());
        }
        if let Some(k) = self.refraction.filter(|k| !Self::refraction_holds(*k)) {
            return Some(format!("kırılma katsayısı k {k}; −1 ile 1 arasında olmalı"));
        }
        let tolerances = [
            ("iki durumun yatay açı farkı", self.face_hz),
            ("indeks hatası", self.index),
            ("iki durumun uzunluk farkı", self.face_slope),
        ];
        tolerances.into_iter().find_map(|(what, t)| {
            t.filter(|t| !Self::tolerance_holds(*t))
                .map(|t| format!("{what} toleransı {t}; sıfırdan büyük olmalı"))
        })
    }

    /// The settings as a project keeps them: k where it holds and is not
    /// the default, the tolerances that hold; none when nothing is left.
    pub fn sanitized(self) -> Option<Self> {
        let kept = Self {
            refraction: self
                .refraction
                .filter(|k| Self::refraction_holds(*k) && *k != REFRACTION),
            face_hz: self.face_hz.filter(|t| Self::tolerance_holds(*t)),
            index: self.index.filter(|t| Self::tolerance_holds(*t)),
            face_slope: self.face_slope.filter(|t| Self::tolerance_holds(*t)),
        };
        (kept != Self::default()).then_some(kept)
    }
}

impl ProjectSettings {
    /// The project's type; none while it is not asked: no value, or the
    /// former Hibrit mode's (docs/adr/0165 §1).
    pub fn project_type(&self) -> Option<Workspace> {
        self.workspace.filter(|w| *w != Workspace::LegacyHybrid)
    }

    /// The refraction coefficient k of trigonometric heights: the project's,
    /// or [`REFRACTION`] (docs/adr/0169 §3).
    pub fn refraction(&self) -> f64 {
        self.survey
            .as_ref()
            .map_or(REFRACTION, SurveySettings::refraction)
    }

    /// Whether the project has a coordinate system: the registry's, or its
    /// own definition (docs/adr/0168 §1).
    pub fn has_system(&self) -> bool {
        self.srid != 0 || self.custom_crs.is_some()
    }

    /// The unit lengths are typed and read in: a project without a
    /// coordinate system has its own, any other metres (docs/adr/0165 §2).
    pub fn unit(&self) -> DrawingUnit {
        if self.has_system() {
            DrawingUnit::M
        } else {
            self.drawing_unit.unwrap_or_default()
        }
    }

    /// The second coordinate system's SRID, when the project may have one: a
    /// system other than its own, and the project has one (docs/adr/0167 §1).
    pub fn second(&self) -> Option<u32> {
        self.second_srid
            .filter(|s| *s != 0 && *s != self.srid && self.has_system())
    }

    /// The settings as a project keeps them (docs/adr/0167 §1, 0168 §1–§3):
    /// its own definition only without an EPSG code and where its rules
    /// hold, a second system only where it may be (an EPSG code before a
    /// definition), a unit only without a system, the datum choices only
    /// when they all hold, the survey settings that hold (docs/adr/0169 §3).
    pub fn sanitized(mut self) -> Self {
        if self.srid != 0
            || self
                .custom_crs
                .as_ref()
                .is_some_and(|d| d.problem().is_some())
        {
            self.custom_crs = None;
        }
        self.second_srid = self.second();
        if self.second_srid.is_some()
            || !self.has_system()
            || self
                .second_custom_crs
                .as_ref()
                .is_some_and(|d| d.problem().is_some())
        {
            self.second_custom_crs = None;
        }
        if self.custom_crs.is_some() {
            self.drawing_unit = None;
        }
        if choices_problem(&self.datum_transforms).is_some() {
            self.datum_transforms.clear();
        }
        self.survey = self.survey.and_then(SurveySettings::sanitized);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// The project's own style library (opaque items in v1, see `style`).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectStyles {
    #[cfg_attr(feature = "ts", ts(type = "unknown[]"))]
    pub items: Vec<serde_json::Value>,
    #[cfg_attr(feature = "ts", ts(type = "unknown[]"))]
    pub categories: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DocumentSnapshotV1 {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.document\""))]
    pub format: String,
    #[cfg_attr(feature = "ts", ts(type = "1"))]
    pub version: u32,
    pub name: String,
    pub settings: ProjectSettings,
    /// Local anchor near the data (the GPU works relative to it).
    pub origin: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub home_view: Option<Bounds>,
    pub layers: Vec<LayerNode>,
    pub active_layer: String,
    pub entities: Vec<Entity>,
    pub styles: ProjectStyles,
    /// Block definitions (docs/adr/0144); none in a drawing without blocks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::entity::BlockDefinition>>", optional)
    )]
    pub blocks: Vec<crate::entity::BlockDefinition>,
}

/// Where a drawing kept as v2 was migrated from: the v1 file it was opened
/// from (docs/adr/0014, TODOS.md FILE-05, FILE-21). A v2 file keeps it in
/// every later save; it says where the objects' derived ids came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MigrationSource {
    /// `kentos.document`.
    pub format: String,
    /// 1: the only migration there is.
    pub version: u32,
    /// sha256 of the v1 file's canonical text, 64 lowercase hexadecimal
    /// digits: the namespace of the derived ids comes from it (`V1Identities`).
    pub source_sha256: String,
}

impl MigrationSource {
    /// A v1 `.kcad` whose canonical text has this sha256 (64 lowercase hexadecimal digits).
    pub fn v1(source_sha256: String) -> Self {
        Self {
            format: DOCUMENT_FORMAT.to_owned(),
            version: DOCUMENT_VERSION,
            source_sha256,
        }
    }
}

/// A whole drawing as a binary `.kcad` v2 holds it (docs/specs/kcad-v2.md):
/// what v1 holds, every object's persistent id, the project's id and, for a
/// drawing migrated from v1, where it came from. This is its JSON form, which
/// the browser and the formats WASM module exchange; the file itself is
/// written and read by `kentos-kcad`.
///
/// The objects' `id`s are the open document's slots: a v2 file does not write
/// them, and a reader numbers the objects 1, 2, 3 … in file order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DocumentSnapshotV2 {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.document\""))]
    pub format: String,
    #[cfg_attr(feature = "ts", ts(type = "2"))]
    pub version: u32,
    pub name: String,
    pub settings: ProjectSettings,
    /// Local anchor near the data (the GPU works relative to it).
    pub origin: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub home_view: Option<Bounds>,
    pub layers: Vec<LayerNode>,
    pub active_layer: String,
    /// The objects in document order (drawing order).
    pub entities: Vec<Entity>,
    /// Each object's persistent id, in the order of `entities`.
    pub uids: Vec<EntityId>,
    pub styles: ProjectStyles,
    /// Block definitions (docs/adr/0144); none in a drawing without blocks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::entity::BlockDefinition>>", optional)
    )]
    pub blocks: Vec<crate::entity::BlockDefinition>,
    /// The project's persistent id, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub project_id: Option<ProjectId>,
    /// The v1 file a migrated drawing came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub migrated_from: Option<MigrationSource>,
}

/// The two fields a reader checks before the rest; every other field is skipped unread.
#[derive(Default, Deserialize)]
struct Head {
    #[serde(default)]
    format: Option<serde_json::Value>,
    #[serde(default)]
    version: Option<serde_json::Value>,
}

impl DocumentSnapshotV1 {
    /// Reads a snapshot, refusing other formats and versions instead of guessing.
    /// A leading byte order mark is skipped, as a browser skips it when it
    /// decodes the file. The format and version are read first without building
    /// the rest; the drawing is then read once, straight into the typed
    /// contract (a large file never becomes a `serde_json::Value` tree).
    pub fn from_json(text: &str) -> Result<Self, String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let head = match serde_json::from_str::<Head>(text) {
            Ok(head) => head,
            Err(e) if e.is_syntax() || e.is_eof() => return Err(format!("JSON değil: {e}")),
            // Not an object, or its format or version is not plain JSON: the checks below say so.
            Err(_) => Head::default(),
        };
        if head.format.as_ref().and_then(|f| f.as_str()) != Some(DOCUMENT_FORMAT) {
            return Err("KentOS çizim dosyası değil (format ≠ kentos.document).".into());
        }
        match head.version.as_ref().and_then(|v| v.as_u64()) {
            Some(v) if v == u64::from(DOCUMENT_VERSION) => {}
            Some(v) => {
                return Err(format!(
                    "Çizim dosyası sürümü {v} bu uygulamada okunamıyor (desteklenen: {DOCUMENT_VERSION})."
                ));
            }
            None => return Err("Çizim dosyasında sürüm yok.".into()),
        }
        serde_json::from_str(text).map_err(|e| format!("Çizim dosyası bozuk: {e}"))
    }
}
