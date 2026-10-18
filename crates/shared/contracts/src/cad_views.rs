//! The named views' command of the product command catalog (docs/adr/0285):
//! `cad.views.edit` keeps, renames and removes the project's named views. A
//! named view is where a camera looks: the world point in the middle of the
//! area, the world width and height it shows and its twist (the view turned
//! so a direction of the world is horizontal on the screen). Named views are
//! a project setting (an edit, no undo step); going to one is the host's
//! camera, not the drawing's: no command writes it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Keeps, renames and removes the project's named views.
pub const CAD_VIEWS_EDIT: &str = "cad.views.edit";
pub const CAD_VIEWS_EDIT_VERSION: u32 = 1;

/// The most named views a project keeps.
pub const MOST_NAMED_VIEWS: usize = 100;
/// The longest name of a named view, in characters.
pub const MOST_VIEW_NAME: usize = 64;
/// How far from the world's origin a view's middle may be, metres (10 000 km).
pub const VIEW_REACH: f64 = 1.0e7;
/// The widest and tallest a view may show, metres (20 000 km).
pub const MOST_VIEW_SIZE: f64 = 2.0e7;

/// A named view of the project: its middle, the world size it shows (width
/// along the screen's x, height along its y) and its twist: the angle of the
/// screen's x axis from the world's x (east), counter-clockwise, in radians,
/// in [0, 2π). A view with twist 0 shows the north up.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NamedView {
    /// Its stable id, which a view mark points at (docs/adr/0345 §2); empty: the one its name gives (`view_key`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub id: String,
    /// Trimmed, 1–64 characters, no control characters; unique.
    pub name: String,
    /// The world point in the middle of the area.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub center: Vec2,
    /// The world width the area shows, along the screen's x.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// The world height the area shows, along the screen's y.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// The screen's x axis from east, counter-clockwise; 0 shows the north up.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub twist: f64,
}

/// The id a view's name gives: `v-` and the name's FNV-1a (32 bits) in eight lowercase hexadecimal digits.
pub fn view_id_of_name(name: &str) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in name.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("v-{h:08x}")
}

/// A view's stable id: its own, or the one its name gives (docs/adr/0345 §2).
pub fn view_key(v: &NamedView) -> String {
    if v.id.is_empty() {
        view_id_of_name(&v.name)
    } else {
        v.id.clone()
    }
}

/// Whether `name` (trimmed) can name a view.
pub fn view_name_ok(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name.chars().count() <= MOST_VIEW_NAME
        && !name.chars().any(char::is_control)
}

/// Whether a view's numbers hold: a middle within reach, a width and a
/// height above 0 and at most 20 000 km, a twist in [0, 2π).
pub fn view_holds(v: &NamedView) -> bool {
    let size = |s: f64| s.is_finite() && s > 0.0 && s <= MOST_VIEW_SIZE;
    v.center.x.is_finite()
        && v.center.y.is_finite()
        && v.center.x.abs() <= VIEW_REACH
        && v.center.y.abs() <= VIEW_REACH
        && size(v.width)
        && size(v.height)
        && v.twist.is_finite()
        && v.twist >= 0.0
        && v.twist < 2.0 * std::f64::consts::PI
}

/// What is wrong with a project's named views, for a reader (KCAD, the
/// server); none when they hold: at most 100, names that hold and are
/// unique, numbers that hold.
pub fn views_problem(views: &[NamedView]) -> Option<String> {
    if views.len() > MOST_NAMED_VIEWS {
        return Some(format!(
            "En çok {MOST_NAMED_VIEWS} adlı görünüm olur, {} var.",
            views.len()
        ));
    }
    for (i, v) in views.iter().enumerate() {
        if !view_name_ok(&v.name) || v.name.trim() != v.name {
            return Some(format!("“{}” bir görünüm adı olamaz.", v.name));
        }
        if views[..i].iter().any(|w| w.name == v.name) {
            return Some(format!("“{}” adlı iki görünüm var.", v.name));
        }
        if v.id.chars().count() > MOST_VIEW_NAME || v.id.chars().any(char::is_control) {
            return Some(format!("“{}” görünümünün kimliği tutmuyor.", v.name));
        }
        if views[..i].iter().any(|w| view_key(w) == view_key(v)) {
            return Some(format!("“{}” görünümünün kimliği başka bir görünümünkiyle aynı.", v.name));
        }
        if !view_holds(v) {
            return Some(format!(
                "“{}” görünümünün ortası 10 000 km içinde, eni ve boyu 0'dan büyük ve en çok 20 000 km, dönüşü [0, 2π) aralığında olmalı.",
                v.name
            ));
        }
    }
    None
}

/// The named views a project keeps: those that hold, the first of a name, at most 100.
pub fn sanitized_named_views(views: Vec<NamedView>) -> Vec<NamedView> {
    let mut kept: Vec<NamedView> = Vec::with_capacity(views.len().min(MOST_NAMED_VIEWS));
    for v in views {
        if kept.len() == MOST_NAMED_VIEWS {
            break;
        }
        if views_problem(std::slice::from_ref(&v)).is_none()
            && !kept
                .iter()
                .any(|k| k.name == v.name || view_key(k) == view_key(&v))
        {
            kept.push(v);
        }
    }
    kept
}

/// What `cad.views.edit` does (docs/adr/0285 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ViewsOp {
    /// Kaydet: a view kept under a name; one of that name is replaced.
    Save {
        name: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        center: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        width: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        height: f64,
        /// Radians, any turn; kept in [0, 2π). None: 0, the north up.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        twist: Option<f64>,
    },
    /// Yeniden adlandır: a named view gets another name, keeping its place in the list.
    Rename { name: String, to: String },
    /// Sil: a named view removed.
    Delete { name: String },
}

/// Input of `cad.views.edit` v1.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite`,
/// `far_point`, `invalid_view_size`, `invalid_view_name`;
/// `invalid_revision`, `revision_conflict` (status `conflict`);
/// `unknown_view`, `view_exists`, `too_many_views`. Warning: `view_replaced`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewsEdit {
    pub edit: ViewsOp,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.views.edit` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewsEdited {
    /// The project's named views afterwards, in order.
    pub views: Vec<NamedView>,
    /// The view saved or renamed, as kept; none for a delete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<NamedView>,
    /// Whether the settings changed.
    pub changed: bool,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
