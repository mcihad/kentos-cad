//! The project catalog (TODOS.md CLOUD-02..05, docs/adr/0028): what
//! describes a project beside its content (a type, a description, tags),
//! where it is in its life (active, archived, in the trash; removed for good
//! after that), the lifecycle commands, and the lists a person browses
//! (their own, an organisation's, shared with them, recently opened,
//! favourites, archived, the trash), searched, sorted and paged on the
//! server after the access check.
//!
//! - A type is a label for finding and ordering work. It does not say that a
//!   project meets a regulation or was approved, it opens no module, and it
//!   does not decide how or where the project is stored (`ProjectStorage`).
//! - The metadata is versioned twice: the commands' input versions (a
//!   change of shape is a new version; an organisation's own fields would
//!   be `project.metadata.update` v2) and each project's `catalogVersion`,
//!   which every change of its name, description, type or tags raises, so a
//!   catalog edit based on an older one is refused instead of overwriting it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::cloud::ProjectSummary;
use crate::document::Bounds;

/// Opening a new project in a workspace (the tenant-level command route).
pub const PROJECT_CREATE: &str = "project.create";
pub const PROJECT_CREATE_VERSION: u32 = 1;
pub const PROJECT_RENAME: &str = "project.rename";
pub const PROJECT_RENAME_VERSION: u32 = 1;
pub const PROJECT_METADATA_UPDATE: &str = "project.metadata.update";
pub const PROJECT_METADATA_UPDATE_VERSION: u32 = 1;
pub const PROJECT_DUPLICATE: &str = "project.duplicate";
pub const PROJECT_DUPLICATE_VERSION: u32 = 1;
pub const PROJECT_ARCHIVE: &str = "project.archive";
pub const PROJECT_ARCHIVE_VERSION: u32 = 1;
pub const PROJECT_UNARCHIVE: &str = "project.unarchive";
pub const PROJECT_UNARCHIVE_VERSION: u32 = 1;
pub const PROJECT_TRASH: &str = "project.trash";
pub const PROJECT_TRASH_VERSION: u32 = 1;
pub const PROJECT_RESTORE: &str = "project.restore";
pub const PROJECT_RESTORE_VERSION: u32 = 1;
pub const PROJECT_PURGE: &str = "project.purge";
pub const PROJECT_PURGE_VERSION: u32 = 1;
pub const PROJECT_FAVORITE: &str = "project.favorite";
pub const PROJECT_FAVORITE_VERSION: u32 = 1;

/// Key of the catalog metadata (name, description, type, tags) in `expectedVersions`.
pub const PROJECT_CATALOG_KEY: &str = "@catalog";

/// Event kinds (`EventRecord.kind`, no objects). Moving to the trash keeps
/// `project.deleted` (`PROJECT_DELETED`), which clients already stop on.
pub const PROJECT_ARCHIVED: &str = "project.archived";
pub const PROJECT_UNARCHIVED: &str = "project.unarchived";
pub const PROJECT_RESTORED: &str = "project.restored";
/// The name, description, type or tags changed (`meta` is true when the name did).
pub const PROJECT_METADATA_CHANGED: &str = "project.metadata";

/// Limits of the catalog metadata, in characters.
pub const PROJECT_DESCRIPTION_MAX: usize = 2000;
pub const PROJECT_TAGS_MAX: usize = 12;
pub const PROJECT_TAG_MAX: usize = 32;

/// What kind of work a project is (TODOS.md CLOUD-02). A label only (see the module notes).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProjectType {
    /// General CAD work; also every project made before types existed.
    #[default]
    Cad,
    /// GIS work.
    Gis,
    /// A land readjustment under article 18 of the Zoning Law (18 uygulaması).
    LandReadjustment,
    /// A zoning plan (imar planı).
    ZoningPlan,
    /// Dividing and merging parcels (ifraz and tevhit).
    Subdivision,
    /// Road design.
    Road,
    /// Architecture.
    Architecture,
}

impl ProjectType {
    pub const ALL: [ProjectType; 7] = [
        Self::Cad,
        Self::Gis,
        Self::LandReadjustment,
        Self::ZoningPlan,
        Self::Subdivision,
        Self::Road,
        Self::Architecture,
    ];

    /// Its name in the API and the database.
    pub fn name(self) -> &'static str {
        match self {
            Self::Cad => "cad",
            Self::Gis => "gis",
            Self::LandReadjustment => "landReadjustment",
            Self::ZoningPlan => "zoningPlan",
            Self::Subdivision => "subdivision",
            Self::Road => "road",
            Self::Architecture => "architecture",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }
}

/// Where a project is in its life. Removed for good, it is gone: every request answers 404.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProjectState {
    Active,
    /// Read-only: it opens, but its content and catalog metadata do not change until it is unarchived.
    Archived,
    /// Soft-deleted (410 to opening and writing); restorable until the retention removes it for good.
    Trashed,
}

// ── Command inputs ───────────────────────────────────────────────────────

/// Input of commands that need nothing but the project the envelope names
/// (`project.archive`, `project.unarchive`, `project.trash`, `project.restore`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EmptyInput {}

/// Input of `project.rename` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectRename {
    /// Not empty, at most 200 bytes; surrounding spaces are dropped.
    pub name: String,
}

/// Input of `project.metadata.update` v1: the fields given replace the
/// project's; absent ones stay.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectMetadataUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// At most 2 000 characters; surrounding spaces are dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub project_type: Option<ProjectType>,
    /// At most 12, each 1–32 characters; spaces inside are collapsed, and a
    /// tag that differs from an earlier one only by case or Turkish letters is dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tags: Option<Vec<String>>,
}

/// Input of `project.duplicate` v1.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectDuplicate {
    /// The copy's name; absent: the source's with " (kopya)".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// The workspace the copy goes to; absent: the source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tenant_id: Option<String>,
}

/// Input of `project.purge` v1: the explicit confirmation of removing a project for good.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectPurge {
    /// The project's name now, exactly: the caller names what goes.
    pub confirm_name: String,
}

/// Input of `project.favorite` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectFavorite {
    /// Add to the caller's favourites (true) or take it out (false).
    pub favorite: bool,
}

// ── Command outputs ──────────────────────────────────────────────────────

/// What a catalog or lifecycle command did (rename, metadata, archive,
/// unarchive, trash, restore, favourite); a retry with the same
/// idempotency key gets the same answer (`replayed`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectCatalogChange {
    /// The project as the caller sees it after the command.
    pub project: ProjectSummary,
    /// Whether anything changed (archiving an archived project does not).
    pub changed: bool,
    /// The event open connections hear, when something changed that they hear of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub event_seq: Option<String>,
    #[serde(default)]
    pub replayed: bool,
}

/// The result of `project.duplicate`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectDuplicated {
    /// The copy, as its owner (the caller) sees it.
    pub project: ProjectSummary,
    pub source_id: String,
    /// How many objects were copied (decimal text).
    pub objects: String,
    #[serde(default)]
    pub replayed: bool,
}

/// The result of `project.purge`. The project is gone: a retry after a lost
/// answer finds nothing (404).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectPurged {
    pub project_id: String,
    pub name: String,
    /// How many objects went with it (decimal text).
    pub objects: String,
}

// ── Lists ────────────────────────────────────────────────────────────────

/// A list of the catalog (`GET /v1/me/catalog?view=`), for the signed-in person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CatalogView {
    /// Active projects they own, in every workspace (“Projelerim”).
    Mine,
    /// An organisation's active projects they may see (`tenant`), whoever owns them (“Kurum projeleri”).
    Organization,
    /// Active projects shared with them, in every workspace (“Benimle paylaşılanlar”).
    Shared,
    /// Projects they opened, latest first; not the ones in the trash (“Son kullanılanlar”).
    Recent,
    /// Their favourites; not the ones in the trash (“Favoriler”).
    Favorites,
    /// Archived projects they may see (“Arşivlenmişler”).
    Archived,
    /// Projects in the trash they may restore or remove for good (“Çöp kutusu”).
    Trash,
}

/// The order of a list; ties go by project id. Times newest first, names A to Z
/// (Turkish letters folded, the same on every server whatever its locale).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CatalogSort {
    /// Last change of content or catalog metadata.
    Updated,
    Name,
    Created,
    /// When the caller last opened it (`recent` only).
    Opened,
    /// When it was moved to the trash (`trash` only).
    Trashed,
}

/// One page of a catalog list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectPage {
    pub projects: Vec<ProjectSummary>,
    /// How many projects the whole list holds with this search, counted after the access check.
    pub total: u32,
    /// The cursor of the next page (`after`); absent on the last page. Opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<String>,
    /// How long a project moved to the trash now stays restorable, in days.
    pub trash_retention_days: u32,
}

/// `GET …/projects/{project}/details`: one project's catalog entry with what
/// is worked out on asking (for the details of a selected project).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectDetails {
    pub project: ProjectSummary,
    /// The extent of its objects' stored GIS geometry in the project's
    /// coordinate system (within a millimetre of curved CAD objects, whose
    /// geometry is their projection); absent when it has none. A catalog
    /// hint, not a measurement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounds: Option<Bounds>,
    /// Decimal text.
    pub feature_count: String,
    /// Leaf layers of its layer tree.
    pub layer_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_and_list_names_on_the_wire_are_the_fixed_ones() {
        for t in ProjectType::ALL {
            assert_eq!(serde_json::to_value(t).unwrap(), t.name());
            assert_eq!(ProjectType::from_name(t.name()), Some(t));
        }
        assert_eq!(ProjectType::from_name("imar"), None);
        assert_eq!(ProjectType::default(), ProjectType::Cad);
        assert_eq!(
            serde_json::to_value(ProjectState::Trashed).unwrap(),
            "trashed"
        );
        assert_eq!(
            serde_json::to_value(CatalogView::Favorites).unwrap(),
            "favorites"
        );
        assert_eq!(
            serde_json::from_value::<CatalogSort>(serde_json::json!("trashed")).unwrap(),
            CatalogSort::Trashed
        );
        // Absent fields of a metadata update stay absent: they are not changes.
        let empty: ProjectMetadataUpdate = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, ProjectMetadataUpdate::default());
        assert_eq!(serde_json::to_string(&empty).unwrap(), "{}");
    }
}
