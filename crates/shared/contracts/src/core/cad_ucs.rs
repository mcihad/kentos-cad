//! The user coordinate system's command of the product command catalog
//! (docs/adr/0261): `cad.ucs.set` sets the drawing's 2D UCS, an origin in
//! the world and the angle of its x axis, or keeps and removes named ones.
//! A UCS is a project setting (an edit, no undo step); the objects never
//! move. The wire is always the world: points given and taken are world
//! points, the UCS only says how the user types and sees them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Vec2;

/// Sets the drawing's user coordinate system.
pub const CAD_UCS_SET: &str = "cad.ucs.set";
pub const CAD_UCS_SET_VERSION: u32 = 1;

/// The most named UCSs a project keeps.
pub const MOST_NAMED_UCS: usize = 100;
/// The longest name of a named UCS, in characters.
pub const MOST_UCS_NAME: usize = 64;
/// How far from the world's origin a UCS's point may be, metres (10 000 km).
pub const UCS_REACH: f64 = 1.0e7;
/// Two points nearer than this give no direction, metres (1 µm).
pub const UCS_SAME: f64 = 1.0e-6;

/// A user coordinate system: its origin in the world and the angle of its x
/// axis from the world's x (east), counter-clockwise, in radians, in
/// [0, 2π). Its y axis is square to x, counter-clockwise; it neither scales
/// nor mirrors. The world's is origin (0, 0), angle 0, kept as no setting.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Ucs {
    /// Its origin in the world.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: Vec2,
    /// Its x axis from east, counter-clockwise.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub angle: f64,
}

impl Ucs {
    /// The world UCS.
    pub const WORLD: Ucs = Ucs {
        origin: Vec2 { x: 0.0, y: 0.0 },
        angle: 0.0,
    };

    /// A UCS with its angle brought into [0, 2π).
    pub fn new(origin: Vec2, angle: f64) -> Ucs {
        Ucs {
            origin,
            angle: normal_angle(angle),
        }
    }

    pub fn is_world(&self) -> bool {
        self.origin.x == 0.0 && self.origin.y == 0.0 && self.angle == 0.0
    }
}

/// `a` in [0, 2π): the remainder of a division by 2π, a negative one turned
/// once (docs/adr/0261 §1); −0 is 0.
pub fn normal_angle(a: f64) -> f64 {
    let tau = 2.0 * std::f64::consts::PI;
    let mut r = a % tau;
    if r < 0.0 {
        r += tau;
    }
    if r >= tau {
        r = 0.0;
    }
    r + 0.0
}

/// A named UCS of the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NamedUcs {
    /// Trimmed, 1–64 characters, no control characters; unique.
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub angle: f64,
}

impl NamedUcs {
    pub fn ucs(&self) -> Ucs {
        Ucs {
            origin: self.origin,
            angle: self.angle,
        }
    }
}

/// Whether `name` (trimmed) can name a UCS.
pub fn ucs_name_ok(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name.chars().count() <= MOST_UCS_NAME && !name.chars().any(char::is_control)
}

/// What is wrong with a project's UCS settings, for a reader (KCAD, the
/// server); none when they hold: finite numbers within reach, angles in
/// [0, 2π), the current one not the world's (that is no setting), names
/// that hold and are unique, at most 100.
pub fn ucs_problem(ucs: Option<&Ucs>, named: &[NamedUcs]) -> Option<String> {
    let holds = |u: &Ucs| {
        u.origin.x.is_finite()
            && u.origin.y.is_finite()
            && u.origin.x.abs() <= UCS_REACH
            && u.origin.y.abs() <= UCS_REACH
            && u.angle.is_finite()
            && u.angle >= 0.0
            && u.angle < 2.0 * std::f64::consts::PI
    };
    if let Some(u) = ucs {
        if u.is_world() {
            return Some("Dünya UCS'i ayar olarak yazılmaz.".into());
        }
        if !holds(u) {
            return Some(
                "UCS'in başlangıcı 10 000 km içinde, açısı [0, 2π) aralığında olmalı.".into(),
            );
        }
    }
    if named.len() > MOST_NAMED_UCS {
        return Some(format!(
            "En çok {MOST_NAMED_UCS} adlı UCS olur, {} var.",
            named.len()
        ));
    }
    for (i, n) in named.iter().enumerate() {
        if !ucs_name_ok(&n.name) || n.name.trim() != n.name {
            return Some(format!("“{}” bir UCS adı olamaz.", n.name));
        }
        if named[..i].iter().any(|m| m.name == n.name) {
            return Some(format!("“{}” adlı iki UCS var.", n.name));
        }
        if !holds(&n.ucs()) {
            return Some(format!(
                "“{}” UCS'inin başlangıcı 10 000 km içinde, açısı [0, 2π) aralığında olmalı.",
                n.name
            ));
        }
    }
    None
}

/// The named UCSs a project keeps: those that hold, the first of a name, at most 100.
pub fn sanitized_named_ucs(named: Vec<NamedUcs>) -> Vec<NamedUcs> {
    let mut kept: Vec<NamedUcs> = Vec::with_capacity(named.len().min(MOST_NAMED_UCS));
    for n in named {
        if kept.len() == MOST_NAMED_UCS {
            break;
        }
        if ucs_problem(None, std::slice::from_ref(&n)).is_none()
            && !kept.iter().any(|k| k.name == n.name)
        {
            kept.push(n);
        }
    }
    kept
}

/// What the UCS becomes (docs/adr/0261 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum UcsTo {
    /// Dünya: the world's axes.
    World,
    /// A frame given as it is (Önceki, Python).
    Frame {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        origin: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        angle: f64,
    },
    /// Orijin: the origin moves, the axes keep their angle.
    Origin {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        point: Vec2,
    },
    /// Döndür: the axes turn by `angle` about the origin.
    Rotate {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        angle: f64,
    },
    /// İki nokta: the origin, and the x axis toward the second point.
    TwoPoints {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        origin: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        x_point: Vec2,
    },
    /// Nesne: aligned to an object, at the place clicked (§3).
    Object {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
    /// Adlı UCS: a named one.
    Named { name: String },
    /// Kaydet: the current UCS kept under a name; one of that name is replaced.
    Save { name: String },
    /// Sil: a named one removed; the current UCS stays.
    Delete { name: String },
}

/// Input of `cad.ucs.set` v1.
///
/// Refusals (`CommandError.code`), checked in this order: `non_finite`,
/// `far_point`, `invalid_ucs_name`, `same_points`, `invalid_uid`;
/// `invalid_revision`, `revision_conflict` (status `conflict`);
/// `entity_not_found`, `object_without_axis`, `unknown_ucs`,
/// `too_many_ucs`. Warning: `ucs_replaced`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UcsSet {
    pub to: UcsTo,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// The current UCS as the command says it: its frame and, when a named one
/// is the same frame, that name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UcsNow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub angle: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
}

/// Output of `cad.ucs.set` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UcsChanged {
    /// The current UCS afterwards.
    pub ucs: UcsNow,
    /// The named UCSs' names, in order.
    pub named: Vec<String>,
    /// Whether the settings changed.
    pub changed: bool,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
