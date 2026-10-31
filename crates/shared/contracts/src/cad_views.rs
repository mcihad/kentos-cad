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
    /// Its architectural recipe (docs/adr/0774 §1): the level, phase, layer state, categories and scale a sheet's map
    /// linked to it draws with; none: the map's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recipe: Option<ViewRecipe>,
}

/// The longest layer state name a recipe names, in characters.
pub const MOST_RECIPE_STATE: usize = 64;
/// The largest scale denominator a recipe sets (1:100 000).
pub const MOST_RECIPE_SCALE: u32 = 100_000;

/// What kind of objects a view leaves out (docs/adr/0774 §1): its own category hide, the model untouched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ViewCategory {
    /// Dimensions and dimension sets' members.
    Dimensions,
    /// Tags.
    Tags,
    /// Texts, paragraphs and leaders.
    Texts,
    /// Hatches.
    Hatches,
    /// Block inserts: furniture, fixtures, symbols.
    Blocks,
    /// Spaces (their outline and label).
    Spaces,
    /// Grids and their bubbles.
    Grids,
    /// View marks (section, elevation, detail).
    Viewmarks,
}

impl ViewCategory {
    pub const ALL: [ViewCategory; 8] = [
        ViewCategory::Dimensions,
        ViewCategory::Tags,
        ViewCategory::Texts,
        ViewCategory::Hatches,
        ViewCategory::Blocks,
        ViewCategory::Spaces,
        ViewCategory::Grids,
        ViewCategory::Viewmarks,
    ];

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            ViewCategory::Dimensions => "Ölçüler",
            ViewCategory::Tags => "Etiketler",
            ViewCategory::Texts => "Yazılar",
            ViewCategory::Hatches => "Taramalar",
            ViewCategory::Blocks => "Bloklar ve donatı",
            ViewCategory::Spaces => "Mahaller",
            ViewCategory::Grids => "Akslar",
            ViewCategory::Viewmarks => "Görünüm işaretleri",
        }
    }

    /// The category an object of `kind` (its JSON `kind`) is in, if any.
    pub fn of_kind(kind: &str) -> Option<ViewCategory> {
        Some(match kind {
            "dimension" => ViewCategory::Dimensions,
            "tag" => ViewCategory::Tags,
            "text" | "leader" => ViewCategory::Texts,
            "hatch" => ViewCategory::Hatches,
            "insert" => ViewCategory::Blocks,
            "space" => ViewCategory::Spaces,
            "grid" => ViewCategory::Grids,
            "viewmark" => ViewCategory::Viewmarks,
            _ => return None,
        })
    }
}

/// The level a recipe draws (docs/adr/0716's map level): its id, the level under it faint with `below`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecipeLevel {
    pub id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub below: bool,
}

/// A named view's architectural recipe (docs/adr/0774 §1; Revit's view properties, Archicad's view settings): what a
/// sheet's map linked to the view draws. It is the view's, not the model's (no object changes) nor the person's (the
/// screen's level view, theme and layer panel stay theirs). `version` counts its changes: written by `cad.views.edit`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewRecipe {
    /// 1 for a new recipe, one more at each change; given in an input, it is ignored.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
    /// The level the map draws; none: the map's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<RecipeLevel>,
    /// The phases it shows (docs/adr/0346 §2); none: the project's phase view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub phase: Option<crate::arch::PhaseView>,
    /// A layer state of the project by its name, in this view only (docs/adr/0314 §1); none: the map's layers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_state: Option<String>,
    /// The categories left out, in their order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ViewCategory>>", optional))]
    pub hidden: Vec<ViewCategory>,
    /// The scale's denominator a linked map draws at (1:N), the view's middle kept; none: the view's box fitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<u32>,
    /// The walls' detail the map draws (docs/adr/0781 §1; Revit's Detail Level, Archicad's Partial Structure Display);
    /// none: its scale's, or the project's when it fixes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub detail: Option<crate::arch::WallDetail>,
    /// The map draws only the view's area, its frame's paper outside it empty (docs/adr/0781 §2; Revit's Crop Region).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub crop: bool,
    /// The pens the map draws its categories with (docs/adr/0783 §1; Archicad's pen sets, Revit's object styles in a
    /// view), each category once, in their order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RecipePen>>", optional))]
    pub pens: Vec<RecipePen>,
    /// Every line black, hatches grey (docs/adr/0783 §1); a pen's colour still wins.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mono: bool,
    /// The view template it follows by its id (docs/adr/0783 §2; Revit's assigned View Template): the template gives
    /// everything but the level and the crop, which stay the view's. An input may name it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<String>,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl ViewRecipe {
    /// Whether it changes nothing a map draws (its version aside).
    pub fn is_empty(&self) -> bool {
        self.level.is_none()
            && self.phase.is_none()
            && self.layer_state.is_none()
            && self.hidden.is_empty()
            && self.scale.is_none()
            && self.detail.is_none()
            && !self.crop
            && self.pens.is_empty()
            && !self.mono
            && self.template.is_none()
    }

    /// The pen of `category`, if the recipe gives one.
    pub fn pen(&self, category: PenCategory) -> Option<&RecipePen> {
        self.pens.iter().find(|p| p.of == category)
    }
}

/// The thickest pen a recipe gives, paper millimetres.
pub const MOST_PEN_WEIGHT: f64 = 5.0;
/// The most view templates a project keeps.
pub const MOST_VIEW_TEMPLATES: usize = 50;
/// The grey hatches draw in a black-and-white map (docs/adr/0783 §1).
pub const MONO_HATCH: &str = "#808080";
/// The black lines draw in a black-and-white map.
pub const MONO_LINE: &str = "#000000";

/// What a pen draws (docs/adr/0783 §1): the objects of a kind; `others` everything no category names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PenCategory {
    Walls,
    Openings,
    Columns,
    /// Stairs and ramps.
    Stairs,
    Slabs,
    Spaces,
    Dimensions,
    Tags,
    /// Texts, paragraphs and leaders.
    Texts,
    Hatches,
    /// Block inserts: furniture, fixtures, symbols.
    Blocks,
    Grids,
    Viewmarks,
    /// Every other object: lines, areas, curves.
    Others,
}

impl PenCategory {
    pub const ALL: [PenCategory; 14] = [
        PenCategory::Walls,
        PenCategory::Openings,
        PenCategory::Columns,
        PenCategory::Stairs,
        PenCategory::Slabs,
        PenCategory::Spaces,
        PenCategory::Dimensions,
        PenCategory::Tags,
        PenCategory::Texts,
        PenCategory::Hatches,
        PenCategory::Blocks,
        PenCategory::Grids,
        PenCategory::Viewmarks,
        PenCategory::Others,
    ];

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            PenCategory::Walls => "Duvarlar",
            PenCategory::Openings => "Kapı ve pencereler",
            PenCategory::Columns => "Kolonlar",
            PenCategory::Stairs => "Merdiven ve rampalar",
            PenCategory::Slabs => "Döşemeler",
            PenCategory::Spaces => "Mahaller",
            PenCategory::Dimensions => "Ölçüler",
            PenCategory::Tags => "Etiketler",
            PenCategory::Texts => "Yazılar",
            PenCategory::Hatches => "Taramalar",
            PenCategory::Blocks => "Bloklar ve donatı",
            PenCategory::Grids => "Akslar",
            PenCategory::Viewmarks => "Görünüm işaretleri",
            PenCategory::Others => "Öbür çizgiler",
        }
    }

    /// The category an object of `kind` (its JSON `kind`) draws with.
    pub fn of_kind(kind: &str) -> PenCategory {
        match kind {
            "wall" => PenCategory::Walls,
            "opening" => PenCategory::Openings,
            "column" => PenCategory::Columns,
            "stair" | "ramp" => PenCategory::Stairs,
            "slab" => PenCategory::Slabs,
            "space" => PenCategory::Spaces,
            "dimension" => PenCategory::Dimensions,
            "tag" => PenCategory::Tags,
            "text" | "leader" => PenCategory::Texts,
            "hatch" => PenCategory::Hatches,
            "insert" => PenCategory::Blocks,
            "grid" => PenCategory::Grids,
            "viewmark" => PenCategory::Viewmarks,
            _ => PenCategory::Others,
        }
    }
}

/// A pen of a recipe (docs/adr/0783 §1): a category's paper weight, its colour, or both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecipePen {
    pub of: PenCategory,
    /// Paper millimetres, 0 to 5; none: the objects' own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub weight: Option<f64>,
    /// `#rrggbb`; none: the objects' own (black with `mono`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
}

/// Whether `c` is a colour as `#rrggbb`.
pub fn pen_color_ok(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

/// A view template of the project (docs/adr/0783 §2): a recipe kept under a name, which views follow.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewTemplate {
    /// `t-` and its first name's FNV-1a; kept when renamed.
    pub id: String,
    /// Trimmed, 1–64 characters, no control characters; unique.
    pub name: String,
    /// What it draws: no level, crop or template.
    pub recipe: ViewRecipe,
}

/// The id a template's name gives: `t-` and the name's FNV-1a in eight lowercase hexadecimal digits.
pub fn template_id_of_name(name: &str) -> String {
    format!("t-{}", &view_id_of_name(name)[2..])
}

/// What is wrong with a template's recipe: the recipe's own rule, then no level, crop or template.
pub fn template_problem(r: &ViewRecipe) -> Option<String> {
    if let Some(why) = recipe_problem(r) {
        return Some(why);
    }
    if r.level.is_some() || r.crop || r.template.is_some() {
        return Some("Görünüş şablonu kat, kırpma ya da başka şablon taşımaz: bunlar görünümündür.".into());
    }
    None
}

/// What is wrong with a project's templates for a reader, against its views: at most 50, names and ids that hold and
/// are unique, recipes that hold, and every view's template one of them.
pub fn templates_problem(templates: &[ViewTemplate], views: &[NamedView]) -> Option<String> {
    if templates.len() > MOST_VIEW_TEMPLATES {
        return Some(format!("En çok {MOST_VIEW_TEMPLATES} görünüş şablonu olur, {} var.", templates.len()));
    }
    for (i, t) in templates.iter().enumerate() {
        if !view_name_ok(&t.name) || t.name.trim() != t.name {
            return Some(format!("“{}” bir şablon adı olamaz.", t.name));
        }
        if !view_name_ok(&t.id) || t.id.trim() != t.id {
            return Some(format!("“{}” şablonunun kimliği tutmuyor.", t.name));
        }
        if templates[..i].iter().any(|w| w.name == t.name || w.id == t.id) {
            return Some(format!("“{}” adlı ya da kimlikli iki şablon var.", t.name));
        }
        if let Some(why) = if t.recipe.version == 0 {
            Some("Görünüş tarifinin sürümü en az 1 olur.".to_owned())
        } else {
            template_problem(&t.recipe)
        } {
            return Some(format!("“{}” şablonunun tarifi tutmuyor: {why}", t.name));
        }
    }
    for v in views {
        if let Some(id) = v.recipe.as_ref().and_then(|r| r.template.as_ref())
            && !templates.iter().any(|t| &t.id == id)
        {
            return Some(format!("“{}” görünümünün şablonu “{id}” projede yok.", v.name));
        }
    }
    None
}

/// The templates a project keeps: those that hold, the first of a name or id, at most 50.
pub fn sanitized_view_templates(templates: Vec<ViewTemplate>) -> Vec<ViewTemplate> {
    let mut kept: Vec<ViewTemplate> = Vec::new();
    for t in templates {
        if kept.len() == MOST_VIEW_TEMPLATES {
            break;
        }
        if templates_problem(std::slice::from_ref(&t), &[]).is_none()
            && !kept.iter().any(|k| k.name == t.name || k.id == t.id)
        {
            kept.push(t);
        }
    }
    kept
}

/// A recipe as a map draws it (docs/adr/0783 §2): one following a template takes its fields, keeping its own version,
/// level and crop; the template's id is left out. One whose template is gone draws as it is.
pub fn resolved_recipe(r: &ViewRecipe, templates: &[ViewTemplate]) -> ViewRecipe {
    let Some(id) = &r.template else {
        return r.clone();
    };
    let Some(t) = templates.iter().find(|t| &t.id == id) else {
        return ViewRecipe { template: None, ..r.clone() };
    };
    ViewRecipe {
        version: r.version,
        level: r.level.clone(),
        crop: r.crop,
        template: None,
        ..t.recipe.clone()
    }
}

/// The named views as maps draw them: every recipe resolved by its template.
pub fn resolved_named_views(views: &[NamedView], templates: &[ViewTemplate]) -> Vec<NamedView> {
    views
        .iter()
        .map(|v| NamedView {
            recipe: v.recipe.as_ref().map(|r| resolved_recipe(r, templates)),
            ..v.clone()
        })
        .collect()
}

/// What is wrong with a recipe on its own (no project): a version, a level id, a state name, the categories once, a
/// scale; none when it holds.
pub fn recipe_problem(r: &ViewRecipe) -> Option<String> {
    if r.version == 0 {
        return Some("Görünüş tarifinin sürümü en az 1 olur.".into());
    }
    if r.is_empty() {
        return Some("Görünüş tarifi boş: kat, evre, katman durumu, gizli tür, ölçek, duvar ayrıntısı, kırpma, kalem, şablon ya da siyah beyazdan birini verin.".into());
    }
    if let Some(l) = &r.level
        && (l.id.trim().is_empty() || l.id.chars().count() > MOST_VIEW_NAME || l.id.chars().any(char::is_control))
    {
        return Some("Görünüş tarifinin katı bir kat kimliği olmalı.".into());
    }
    if let Some(s) = &r.layer_state
        && (s.trim().is_empty() || s.chars().count() > MOST_RECIPE_STATE || s.chars().any(char::is_control))
    {
        return Some("Görünüş tarifinin katman durumu bir durumun adı olmalı.".into());
    }
    for (i, c) in r.hidden.iter().enumerate() {
        if r.hidden[..i].contains(c) {
            return Some(format!("“{}” gizlenenlerde iki kez var.", c.label()));
        }
    }
    if let Some(s) = r.scale
        && !(1..=MOST_RECIPE_SCALE).contains(&s)
    {
        return Some("Görünüş tarifinin ölçeği 1:1 ile 1:100 000 arasında olur.".into());
    }
    for (i, p) in r.pens.iter().enumerate() {
        if r.pens[..i].iter().any(|q| q.of == p.of) {
            return Some(format!("“{}” kalemlerde iki kez var.", p.of.label()));
        }
        if p.weight.is_none() && p.color.is_none() {
            return Some(format!("“{}” kaleminin kalınlığı ya da rengi olmalı.", p.of.label()));
        }
        if let Some(w) = p.weight
            && !(0.0..=MOST_PEN_WEIGHT).contains(&w)
        {
            return Some("Kalemin kalınlığı 0 ile 5 mm arasında olur.".into());
        }
        if let Some(c) = &p.color
            && !pen_color_ok(c)
        {
            return Some("Kalemin rengi #rrggbb biçiminde olur.".into());
        }
    }
    if let Some(t) = &r.template {
        if t.trim().is_empty() || t.chars().count() > MOST_VIEW_NAME || t.chars().any(char::is_control) {
            return Some("Görünüş tarifinin şablonu bir şablonun kimliği ya da adı olmalı.".into());
        }
        if r.phase.is_some()
            || r.layer_state.is_some()
            || r.scale.is_some()
            || r.detail.is_some()
            || !r.hidden.is_empty()
            || !r.pens.is_empty()
            || r.mono
        {
            return Some("Şablona bağlı tarifte yalnız kat ve kırpma görünümündür; öbürleri şablondan gelir.".into());
        }
    }
    None
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
        if let Some(r) = &v.recipe
            && let Some(why) = recipe_problem(r)
        {
            return Some(format!("“{}” görünümünün tarifi tutmuyor: {why}", v.name));
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
    /// Alanı değiştir (docs/adr/0789 §1; Revit's crop region grips and Size Crop): the view's area moved, resized or
    /// turned, its id, name, place in the list and recipe kept. The maps linked to it follow; one undo step.
    Area {
        name: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        center: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        width: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        height: f64,
        /// Radians, any turn; kept in [0, 2π). None: the view's own twist.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        twist: Option<f64>,
    },
    /// Görünüş tarifi (docs/adr/0774 §2): the view's recipe set (its version one more) or, none, taken away; the maps
    /// linked to the view draw with it from then on.
    Recipe {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        recipe: Option<ViewRecipe>,
    },
    /// Görünüş şablonu (docs/adr/0783 §2): a template kept or changed under its name (its version one more) or, no
    /// recipe, removed: the views following it keep its fields as their own recipe.
    Template {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        recipe: Option<ViewRecipe>,
    },
    /// Şablonu yeniden adlandır: its id kept, the views following it too.
    TemplateRename { name: String, to: String },
}

/// Input of `cad.views.edit` v1.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite`,
/// `far_point`, `invalid_view_size`, `invalid_view_name`, `invalid_recipe`;
/// `invalid_revision`, `revision_conflict` (status `conflict`);
/// `unknown_view`, `view_exists`, `too_many_views`, `level_not_found` and `state_not_found` (a recipe's level or layer
/// state the project does not have), `unknown_template`, `template_exists`, `too_many_templates` (docs/adr/0783).
/// Warnings: `view_replaced`, `template_detached`. A view saved over one keeps its recipe. An area
/// change (`area`) is one undo step “Görünümün alanı” (docs/adr/0789 §1); the other changes are settings edits.
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
    /// The project's view templates afterwards (docs/adr/0783 §2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ViewTemplate>>", optional))]
    pub templates: Vec<ViewTemplate>,
    /// The template kept, changed or renamed; none otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<ViewTemplate>,
    /// Whether the settings changed.
    pub changed: bool,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
