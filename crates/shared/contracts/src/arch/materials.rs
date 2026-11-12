//! The project's building materials (docs/adr/0785, ADV-02b): one table of identities a wall component and a slab bind
//! to by id (`materialId`). A material says its name, its category, how Metraj counts it, and how its cut is drawn: its
//! hatch and its cut pen. A bound component's or slab's `material` text, hatch and pen are the material's; when the
//! material changes, everything bound to it follows in the same undo step (`cad.materials.edit`). A component that
//! binds to none keeps its own text material as before.
//!
//! Revit keeps a material's identity and its cut pattern in one table (Material Browser: Identity, Graphics); Archicad
//! a Building Material's cut fill, its pens and its intersection priority. KentOS keeps the identity and the cut's look in
//! the material, the joining priority on the component (docs/adr/0340), so one material can be a core in one wall and a
//! lining in another.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{ComponentEdge, ComponentHatch, WallComponent};

/// The most materials a project keeps.
pub const MAX_MATERIALS: usize = 256;
/// The longest material name and description, in characters.
pub const MAX_MATERIAL_NAME: usize = 128;

/// What a material is for (Revit's material class, the role ADR 0771 §7 named).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MaterialCategory {
    /// Taşıyıcı: concrete, masonry, timber that carries.
    Structure,
    /// Yalıtım: thermal or acoustic insulation.
    Insulation,
    /// Kaplama: plaster, tiles, screed, cladding.
    Finish,
    /// Membran: a membrane or a vapour barrier.
    Membrane,
    /// Diğer.
    #[default]
    Other,
}

impl MaterialCategory {
    pub const ALL: [MaterialCategory; 5] = [
        MaterialCategory::Structure,
        MaterialCategory::Insulation,
        MaterialCategory::Finish,
        MaterialCategory::Membrane,
        MaterialCategory::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MaterialCategory::Structure => "Taşıyıcı",
            MaterialCategory::Insulation => "Yalıtım",
            MaterialCategory::Finish => "Kaplama",
            MaterialCategory::Membrane => "Membran",
            MaterialCategory::Other => "Diğer",
        }
    }

    pub fn is_other(&self) -> bool {
        *self == MaterialCategory::Other
    }
}

/// How Metraj counts a material: by its face area (m²: plaster, membrane, paint) or its volume (m³: concrete, masonry).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MaterialQuantity {
    Area,
    #[default]
    Volume,
}

impl MaterialQuantity {
    pub const ALL: [MaterialQuantity; 2] = [MaterialQuantity::Area, MaterialQuantity::Volume];

    pub fn label(self) -> &'static str {
        match self {
            MaterialQuantity::Area => "Alan (m²)",
            MaterialQuantity::Volume => "Hacim (m³)",
        }
    }

    pub fn is_volume(&self) -> bool {
        *self == MaterialQuantity::Volume
    }
}

/// A project material (Malzeme): its stable id, the name it is shown and written with, its category, how it is counted,
/// its cut's hatch and its cut pen (the lines along a bound component's sides).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchMaterial {
    /// Lower-case letters, digits and “-”, at most 64; it never changes.
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "MaterialCategory::is_other")]
    #[cfg_attr(feature = "ts", ts(as = "Option<MaterialCategory>", optional))]
    pub category: MaterialCategory,
    #[serde(default, skip_serializing_if = "MaterialQuantity::is_volume")]
    #[cfg_attr(feature = "ts", ts(as = "Option<MaterialQuantity>", optional))]
    pub quantity: MaterialQuantity,
    /// Its cut's hatch; absent: drawn without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hatch: Option<ComponentHatch>,
    /// Its cut pen; absent: the wall's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cut: Option<ComponentEdge>,
}

/// An office material library's format (docs/adr/0791): a file outside the drawing, as KentOS's block library (ADR 0318)
/// and the office's door and window types (ADR 0772) are.
pub const MATERIALS_FORMAT: &str = "kentos.materials";
pub const MATERIALS_FORMAT_VERSION: u32 = 1;

/// An office material library (Malzeme kitaplığı, docs/adr/0791): its name and its materials, each as a project keeps
/// it. KentOS gives one (`kentos-temel`); an office keeps its own in its folders and writes the project's out to one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MaterialsFile {
    /// `kentos.materials`.
    pub format: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    /// The library's name.
    pub library: String,
    pub materials: Vec<ArchMaterial>,
}

impl MaterialsFile {
    /// What is wrong with it: the field and the words; none when it holds. Its materials hold as a project's do
    /// (`materials_problem`), its name is not blank.
    pub fn problem(&self) -> Option<(String, String)> {
        if self.format != MATERIALS_FORMAT || self.version != MATERIALS_FORMAT_VERSION {
            return Some((
                "format".into(),
                format!(
                    "Malzeme kitaplığı {MATERIALS_FORMAT} {MATERIALS_FORMAT_VERSION} biçiminde olmalı; “{}” {} verildi.",
                    self.format, self.version
                ),
            ));
        }
        if self.library.trim().is_empty() {
            return Some(("library".into(), "Malzeme kitaplığının adı boş olmamalı.".into()));
        }
        materials_problem(&self.materials)
    }
}

/// Whether a material's id holds: lower-case ASCII letters, digits and “-”, 1 to 64.
pub fn material_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A material's id from its name: lower-case ASCII letters and digits, Turkish letters folded, the rest one `-` (none at
/// either end), at most 60; `malzeme` when nothing is left.
pub fn material_id_of(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' | 'i' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => '-',
        };
        if c == '-' && (out.is_empty() || out.ends_with('-')) {
            continue;
        }
        out.push(c);
    }
    out.truncate(60);
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() { "malzeme".into() } else { out }
}

/// A material name's problem: blank, edged with spaces, a control character, too long.
pub fn material_name_problem(name: &str) -> Option<String> {
    if name.trim().is_empty()
        || name.trim() != name
        || name.chars().any(char::is_control)
        || name.chars().count() > MAX_MATERIAL_NAME
    {
        return Some(format!(
            "“{name}” malzeme adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok {MAX_MATERIAL_NAME} harf."
        ));
    }
    None
}

fn colour_ok(c: &Option<String>) -> bool {
    c.as_deref().is_none_or(crate::hatch::is_hex_colour)
}

impl ArchMaterial {
    /// What is wrong with it: the field and the words; none when it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if !material_id_ok(&self.id) {
            return Some((
                "id".into(),
                format!("“{}” malzeme kimliği olamaz: küçük harf, rakam ve “-” ile en çok 64 harf.", self.id),
            ));
        }
        if let Some(words) = material_name_problem(&self.name) {
            return Some(("name".into(), words));
        }
        if let Some(h) = &self.hatch {
            if h.pattern.trim().is_empty() {
                return Some(("hatch.pattern".into(), format!("“{}” malzemesinin tarama deseni boş.", self.name)));
            }
            if !(h.scale.is_finite() && h.scale > 0.0) {
                return Some(("hatch.scale".into(), format!("“{}” malzemesinin tarama ölçeği sıfırdan büyük olmalı.", self.name)));
            }
            if !h.angle.is_finite() {
                return Some(("hatch.angle".into(), format!("“{}” malzemesinin tarama açısı sonlu olmalı.", self.name)));
            }
            if !colour_ok(&h.color) {
                return Some(("hatch.color".into(), format!("“{}” malzemesinin tarama rengi #RRGGBB olmalı.", self.name)));
            }
        }
        if let Some(e) = &self.cut {
            if !colour_ok(&e.color) {
                return Some(("cut.color".into(), format!("“{}” malzemesinin kesit kaleminin rengi #RRGGBB olmalı.", self.name)));
            }
            if e
                .line_weight
                .is_some_and(|w| !(w.is_finite() && (0.0..=crate::MAX_LINE_WEIGHT).contains(&w)))
            {
                return Some((
                    "cut.lineWeight".into(),
                    format!("“{}” malzemesinin kesit kalemi 0 ile 100 mm arasında olmalı.", self.name),
                ));
            }
        }
        None
    }

    /// The component bound to it: its `material`, `hatch` and `edge` the material's (the rest the component's own).
    pub fn bind(&self, c: &WallComponent) -> WallComponent {
        WallComponent {
            material: Some(self.name.clone()),
            material_id: Some(self.id.clone()),
            hatch: self.hatch.clone(),
            edge: self.cut.clone(),
            ..c.clone()
        }
    }
}

/// What is wrong with a project's materials: the field (under `materials`) and the words; none when they hold. Ids and
/// names (case aside) are each once.
pub fn materials_problem(list: &[ArchMaterial]) -> Option<(String, String)> {
    if list.len() > MAX_MATERIALS {
        return Some((
            "materials".into(),
            format!("Bir projede en çok {MAX_MATERIALS} malzeme olur; {} verildi.", list.len()),
        ));
    }
    for (i, m) in list.iter().enumerate() {
        if let Some((field, words)) = m.problem() {
            return Some((format!("materials[{i}].{field}"), words));
        }
        let folded = m.name.to_lowercase();
        if let Some(j) = list[..i].iter().position(|o| o.id == m.id) {
            return Some((
                format!("materials[{i}].id"),
                format!("“{}” kimliği iki malzemede ({}. ve {}.) var.", m.id, j + 1, i + 1),
            ));
        }
        if list[..i].iter().any(|o| o.name.to_lowercase() == folded) {
            return Some((
                format!("materials[{i}].name"),
                format!("Projede “{}” adında bir malzeme zaten var; her malzemenin adı ayrı olmalı.", m.name),
            ));
        }
    }
    None
}

/// Whether a component or a slab names a material id that holds (the contract's check of `materialId`).
pub fn material_ref_problem(id: &Option<String>) -> Option<String> {
    match id {
        Some(id) if !material_id_ok(id) => Some(format!(
            "“{id}” malzeme kimliği olamaz: küçük harf, rakam ve “-” ile en çok 64 harf."
        )),
        _ => None,
    }
}
