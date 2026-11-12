//! The building material command (docs/adr/0785): the project's materials made, changed and removed, wall style
//! components, wall components and slabs bound to one (or left), and the text materials taken into the table
//! (`cad.materials.edit`). A material's change goes to every style, wall and slab bound to it in the same undo step;
//! one on a locked layer keeps its older values and is reported. An office material library's materials are taken in
//! and the project's written out to one (docs/adr/0791).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{ArchMaterial, ComponentEdge, ComponentHatch, MaterialCategory, MaterialQuantity, MaterialsFile};

pub const CAD_MATERIALS_EDIT: &str = "cad.materials.edit";
pub const CAD_MATERIALS_EDIT_VERSION: u32 = 1;

/// A material's fields to change; absent ones stay. A nullable field's null takes it away.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaterialChange {
    /// Its new name (Yeniden adlandır); its id stays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub category: Option<MaterialCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quantity: Option<MaterialQuantity>,
    /// Its cut's hatch; null: none.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<ComponentHatch>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ComponentHatch>"))]
    pub hatch: Option<Option<ComponentHatch>>,
    /// Its cut pen; null: the wall's.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<ComponentEdge>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ComponentEdge>"))]
    pub cut: Option<Option<ComponentEdge>>,
}

impl MaterialChange {
    pub fn is_empty(&self) -> bool {
        self == &MaterialChange::default()
    }
}

/// What a material is bound to: a wall style's component, a wall's component, a slab.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MaterialTarget {
    /// A project wall style's component (from its left face, 0 first); the walls of the style follow it.
    WallStyle {
        /// The style's id or name.
        style: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        component: u32,
    },
    /// A wall's own component (from its left face, 0 first).
    Wall {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        component: u32,
    },
    Slab {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
    },
}

/// What a library's material does when the project has one of its id, or another of its name (docs/adr/0791).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MaterialConflict {
    /// Refused (`material_exists`): nothing is taken.
    #[default]
    Refuse,
    /// The project's stays; the library's is not taken (`material_kept`).
    Keep,
    /// One of the same id becomes the library's (its name, category, way of counting, hatch and pen); everything bound
    /// to it follows. Another material of its name is still refused.
    Replace,
}

/// What `cad.materials.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MaterialsOperation {
    /// Yeni malzeme: a material `name` (its id `id`, else from its name), its other fields from `change` (`name`
    /// there is refused: the name is `name`); category Diğer and volume when not given.
    Define {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
        #[serde(default)]
        change: MaterialChange,
    },
    /// Malzemeyi değiştir: the material (its id or name) changed by `change`; everything bound to it follows.
    Update { material: String, change: MaterialChange },
    /// Sil: the material; refused while anything is bound to it (`material_in_use`, with the walls' and slabs' ids).
    Remove { material: String },
    /// Bağla: the targets bound to the material (its id or name; absent or null: left, each keeping its text and look).
    Assign {
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        material: Option<String>,
        targets: Vec<MaterialTarget>,
    },
    /// Metinlerden al: every text material of the wall styles, the walls and the slabs not bound to one becomes a
    /// project material (its id from its text) and they are bound to it; a text whose components' hatch or pen
    /// differ is bound where they are the first's and left elsewhere (`material_adopt_left`), so no look changes.
    Adopt {},
    /// Kitaplıktan al: an office library's materials (`library`, a `kentos.materials` file as its host read it) or
    /// KentOS's library `name` (`kentos-temel`), those named in `materials` (ids or names; none: all), taken into the
    /// project; `onConflict` for one the project has.
    Import {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        library: Option<MaterialsFile>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        materials: Vec<String>,
        #[serde(default)]
        on_conflict: MaterialConflict,
    },
    /// Kitaplığa yaz: the project's materials named (ids or names; none: all) as an office library `library`, given
    /// back as the file's content (its host writes it). The drawing does not change.
    Export {
        library: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        materials: Vec<String>,
    },
}

/// Input of `cad.materials.edit` v1: the project's materials changed as one undo step “Malzeme”, with the wall styles,
/// walls and slabs bound to them. Its plan is the preview: the table and what follows, nothing written.
///
/// Refusals: `invalid_material`, `material_not_found`, `material_exists`, `material_in_use`, `nothing_to_set`,
/// `style_not_found`, `invalid_uid`, `entity_not_found`, `not_a_wall`, `not_a_slab`, `component_not_found`,
/// `layer_locked`, `invalid_library`, `invalid_revision`, `revision_conflict`. Warnings: `material_locked`,
/// `material_adopt_left`, `material_kept`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaterialsEdit {
    pub operation: MaterialsOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What a material's write takes with it: the wall styles (ids) and the walls and slabs (ids) rewritten, those left on
/// locked layers, the texts not taken in.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaterialReport {
    pub styles: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub walls: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub slabs: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub locked: Vec<String>,
    /// Adopt: the texts some of whose components were left unbound (their look differs from the first's).
    pub left: Vec<String>,
    /// Import: the library's materials taken in as new (their ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub taken: Vec<String>,
    /// Import: the project's materials that became the library's (`replace`; their ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub replaced: Vec<String>,
    /// Import: the library's materials not taken, the project's kept (`keep`; the library's ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub kept: Vec<String>,
}

/// Output of `cad.materials.edit` v1 (its plan's too: the preview, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaterialsEdited {
    /// The project's materials after it.
    pub materials: Vec<ArchMaterial>,
    /// The material made, changed, removed or bound (its id); absent for Adopt and for leaving.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    pub report: MaterialReport,
    /// The office library (Export): the file's content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub library: Option<MaterialsFile>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
