//! The project's and the office's door and window types (docs/adr/0772): K01 defined once, its doors following it.
//!
//! A type (`OpeningType`) is a door's, window's or bare opening's values with an id, a name, a version and where it
//! came from. An opening made with it keeps the type's id in its `style` and the version it last took in its
//! `typeVersion`: it follows the type. The fields it keeps its own value of are its `overrides` (örnek istisnası);
//! every other field the type governs (`OpeningTypeField`) is the type's. KentOS's own types
//! (`builtin_opening_types`) are copied once, never followed, as before.
//!
//! An opening's sizes are four different things (docs/adr/0772 §2): the rough opening in the wall (kaba boşluk, its
//! `width`, as it always was), the frame's or product's outer width (kasa dış eni: the rough opening less the fitting
//! gap on each side), the leaf width (kanat eni: inside the frame) and the net clear width a door leaves when open
//! (net geçiş: from the open leaf's face to the opposite stop). `build.sizeBy` says which of the first three the type's
//! nominal size names; “90 cm kapı” is then a 90 cm leaf, or a 90 cm rough opening, never both.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{OpeningFields, OpeningForm, OpeningOperation};

/// A leaf's thickness when a type gives none, metres: only the net clear width reads it (docs/adr/0772 §2).
pub const DEFAULT_LEAF_THICKNESS: f64 = 0.04;
/// How much two sliding leaves overlap, a share of the light between the frame's sides (as plan draws them).
pub const SLIDE_LAP: f64 = 0.05;
/// The most types a project keeps.
pub const MAX_OPENING_TYPES: usize = 1_000;
/// The most mullions or transoms a window has.
pub const MAX_PANES: u8 = 20;
/// The file an office's types travel in.
pub const OPENING_TYPES_FORMAT: &str = "kentos.opening-types";
pub const OPENING_TYPES_FORMAT_VERSION: u32 = 1;

/// Which of an opening's widths its nominal size names (docs/adr/0772 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningSizeBy {
    /// Kaba boşluk: the hole in the wall, jamb to jamb (an opening's `width`).
    #[default]
    Rough,
    /// Kasa dış eni: the product's outer width, the rough opening less the fitting gap each side.
    Unit,
    /// Kanat eni: the light inside the frame, the leaves' width.
    Leaf,
}

impl OpeningSizeBy {
    pub const ALL: [OpeningSizeBy; 3] = [
        OpeningSizeBy::Rough,
        OpeningSizeBy::Unit,
        OpeningSizeBy::Leaf,
    ];

    pub fn name(self) -> &'static str {
        match self {
            OpeningSizeBy::Rough => "rough",
            OpeningSizeBy::Unit => "unit",
            OpeningSizeBy::Leaf => "leaf",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OpeningSizeBy::Rough => "Kaba boşluk",
            OpeningSizeBy::Unit => "Kasa dış eni",
            OpeningSizeBy::Leaf => "Kanat eni",
        }
    }
}

/// A door's or window's construction (docs/adr/0772 §3), each field optional; none given draws it as before.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningBuild {
    /// Which width its nominal size names; absent: the rough opening.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub size_by: Option<OpeningSizeBy>,
    /// The fitting gap between the rough opening and the frame, each side, metres; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub fit: Option<f64>,
    /// A leaf's thickness, metres; absent: drawn as a line, the net clear width reads 0.04 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub leaf_thickness: Option<f64>,
    /// Two unequal leaves: the hinge-start leaf's share of the light, above 0, below 1; absent: halves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub split: Option<f64>,
    /// The frame's depth across the wall, metres; absent: the wall's thickness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub frame_depth: Option<f64>,
    /// With `frameDepth`: the frame's distance from the wall face it opens to (`side`), metres; absent: centred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub frame_offset: Option<f64>,
    /// A door's threshold (eşik), its depth across the wall, metres; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub threshold: Option<f64>,
    /// Its casing (pervaz, söve) on both wall faces beside each jamb, its width, metres; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub casing: Option<f64>,
    /// A window's sill board (denizlik), how far it reaches past the face opposite the side it opens to, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub sill_board: Option<f64>,
    /// A window's mullions (dikey kayıt): its panes are one more; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(max = 20), extend("x-unit" = "count")))]
    pub mullions: Option<u8>,
    /// A window's transoms (yatay kayıt); plan does not show them, schedules and tags do.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(max = 20), extend("x-unit" = "count")))]
    pub transoms: Option<u8>,
    /// The use zone in front of it on the side it opens to (kullanım alanı), its depth, metres; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub use_zone: Option<f64>,
}

impl OpeningBuild {
    pub fn is_empty(&self) -> bool {
        self == &OpeningBuild::default()
    }

    /// What is wrong with it for an opening `width` wide whose frame is `frame` wide: the field (under `build.`) and
    /// the words.
    pub fn problem(&self, width: f64, frame: f64) -> Option<(String, String)> {
        let at = |f: &str| format!("build.{f}");
        let length =
            |v: Option<f64>, name: &str, what: &str, zero: bool| -> Option<(String, String)> {
                let x = v?;
                let ok = x.is_finite()
                    && if zero { x >= 0.0 } else { x > 0.0 }
                    && x <= crate::MAX_RECESS_DEPTH;
                (!ok).then(|| {
                    (
                        at(name),
                        format!(
                            "{what} {x} m; {} en çok {} m olmalı.",
                            if zero {
                                "sıfır ya da daha büyük,"
                            } else {
                                "sıfırdan büyük,"
                            },
                            crate::MAX_RECESS_DEPTH
                        ),
                    )
                })
            };
        length(self.fit, "fit", "Montaj boşluğu", true)
            .or_else(|| length(self.leaf_thickness, "leafThickness", "Kanat kalınlığı", false))
            .or_else(|| length(self.frame_depth, "frameDepth", "Kasa derinliği", false))
            .or_else(|| length(self.frame_offset, "frameOffset", "Kasanın yüzden uzaklığı", true))
            .or_else(|| length(self.threshold, "threshold", "Eşik derinliği", false))
            .or_else(|| length(self.casing, "casing", "Pervaz genişliği", false))
            .or_else(|| length(self.sill_board, "sillBoard", "Denizlik çıkıntısı", false))
            .or_else(|| length(self.use_zone, "useZone", "Kullanım alanı derinliği", false))
            .or_else(|| {
                let s = self.split?;
                (!(s.is_finite() && s > 0.0 && s < 1.0)).then(|| {
                    (
                        at("split"),
                        format!("Kanatların payı {s}; 0 ile 1 arasında olmalı (ör. 0,6: birinci kanat ışığın %60'ı)."),
                    )
                })
            })
            .or_else(|| {
                [("mullions", self.mullions, "Dikey kayıt"), ("transoms", self.transoms, "Yatay kayıt")]
                    .into_iter()
                    .find_map(|(name, v, what)| {
                        let n = v?;
                        (n > MAX_PANES).then(|| (at(name), format!("{what} sayısı {n}; en çok {MAX_PANES} olmalı.")))
                    })
            })
            .or_else(|| {
                let fit = self.fit.unwrap_or(0.0);
                let light = width - 2.0 * fit - 2.0 * frame;
                (self.fit.is_some() && (light <= 0.0 || light.is_nan())).then(|| {
                    (
                        at("fit"),
                        format!(
                            "Montaj boşluğu ({fit} m) ve kasa ({frame} m) iki yandan kaba boşluğu ({width} m) dolduruyor; ışık kalmıyor."
                        ),
                    )
                })
            })
    }
}

/// A field a type governs: an opening following the type takes it, unless the opening keeps its own (docs/adr/0772 §1).
/// The form is the type's own and never an exception; an opening's place, wall, hand, side and number are its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningTypeField {
    Width,
    Leaves,
    Operation,
    Swing,
    ShowSwing,
    Frame,
    Sill,
    Head,
    Depth,
    SizeBy,
    Fit,
    LeafThickness,
    Split,
    FrameDepth,
    FrameOffset,
    Threshold,
    Casing,
    SillBoard,
    Mullions,
    Transoms,
    UseZone,
}

impl OpeningTypeField {
    pub const ALL: [OpeningTypeField; 21] = [
        OpeningTypeField::Width,
        OpeningTypeField::Leaves,
        OpeningTypeField::Operation,
        OpeningTypeField::Swing,
        OpeningTypeField::ShowSwing,
        OpeningTypeField::Frame,
        OpeningTypeField::Sill,
        OpeningTypeField::Head,
        OpeningTypeField::Depth,
        OpeningTypeField::SizeBy,
        OpeningTypeField::Fit,
        OpeningTypeField::LeafThickness,
        OpeningTypeField::Split,
        OpeningTypeField::FrameDepth,
        OpeningTypeField::FrameOffset,
        OpeningTypeField::Threshold,
        OpeningTypeField::Casing,
        OpeningTypeField::SillBoard,
        OpeningTypeField::Mullions,
        OpeningTypeField::Transoms,
        OpeningTypeField::UseZone,
    ];

    pub fn name(self) -> &'static str {
        use OpeningTypeField::*;
        match self {
            Width => "width",
            Leaves => "leaves",
            Operation => "operation",
            Swing => "swing",
            ShowSwing => "showSwing",
            Frame => "frame",
            Sill => "sill",
            Head => "head",
            Depth => "depth",
            SizeBy => "sizeBy",
            Fit => "fit",
            LeafThickness => "leafThickness",
            Split => "split",
            FrameDepth => "frameDepth",
            FrameOffset => "frameOffset",
            Threshold => "threshold",
            Casing => "casing",
            SillBoard => "sillBoard",
            Mullions => "mullions",
            Transoms => "transoms",
            UseZone => "useZone",
        }
    }

    pub fn label(self) -> &'static str {
        use OpeningTypeField::*;
        match self {
            Width => "Kaba boşluk",
            Leaves => "Kanat",
            Operation => "Açılış",
            Swing => "Açılma açısı",
            ShowSwing => "Açılım yayı",
            Frame => "Kasa",
            Sill => "Denizlik yüksekliği",
            Head => "Lento",
            Depth => "Derinlik",
            SizeBy => "Ölçü referansı",
            Fit => "Montaj boşluğu",
            LeafThickness => "Kanat kalınlığı",
            Split => "Kanat payı",
            FrameDepth => "Kasa derinliği",
            FrameOffset => "Kasanın yüzden uzaklığı",
            Threshold => "Eşik",
            Casing => "Pervaz",
            SillBoard => "Denizlik çıkıntısı",
            Mullions => "Dikey kayıt",
            Transoms => "Yatay kayıt",
            UseZone => "Kullanım alanı",
        }
    }

    /// Whether it lives in the construction (`build`).
    pub fn in_build(self) -> bool {
        use OpeningTypeField::*;
        !matches!(
            self,
            Width | Leaves | Operation | Swing | ShowSwing | Frame | Sill | Head | Depth
        )
    }
}

/// An office's or a project's door or window type (docs/adr/0772 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningType {
    /// Its lasting key: lower-case letters, digits and `-`, at most 64, not starting with `kentos-` (KentOS's own).
    pub id: String,
    /// What it is called (K01): unique in the project, case aside.
    pub name: String,
    /// Each change gives it the next; an opening that took an older one is behind (eski sürüm).
    #[cfg_attr(feature = "schema", schemars(range(min = 1), extend("x-unit" = "count")))]
    pub version: u32,
    /// Where it came from: the office library's name it was taken from; absent: made in this project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    pub form: OpeningForm,
    /// The rough opening, metres (an opening's `width`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u8>", optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 2), extend("x-unit" = "count")))]
    pub leaves: u8,
    #[serde(default, skip_serializing_if = "is_swing")]
    #[cfg_attr(feature = "ts", ts(as = "Option<OpeningOperation>", optional))]
    pub operation: OpeningOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub swing: Option<f64>,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub show_swing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub frame: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub sill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub head: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub build: Option<OpeningBuild>,
}

fn one() -> u8 {
    1
}
fn is_one(v: &u8) -> bool {
    *v == 1
}
fn yes() -> bool {
    true
}
fn is_yes(v: &bool) -> bool {
    *v
}
fn is_swing(o: &OpeningOperation) -> bool {
    *o == OpeningOperation::Swing
}

/// Whether `id` may be a project type's id.
pub fn opening_type_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.starts_with("kentos-")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A type's id from its name: lower-case ASCII letters and digits, Turkish letters folded, the rest `-`.
pub fn opening_type_id_of(name: &str) -> String {
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
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("tip");
    }
    if out.starts_with("kentos-") {
        out.insert_str(0, "tip-");
    }
    out.truncate(60);
    out
}

impl OpeningType {
    /// The fields of an opening of this type: `OpeningFields` with its values and nothing else.
    pub fn fields(&self) -> OpeningFields {
        let mut f = OpeningFields {
            host: String::new(),
            at: 0.0,
            anchor: Default::default(),
            width: self.width,
            form: self.form,
            leaves: self.leaves,
            operation: self.operation,
            hand: Default::default(),
            side: Default::default(),
            swing: self.swing,
            show_swing: self.show_swing,
            frame: self.frame,
            sill: self.sill,
            head: self.head,
            depth: self.depth,
            inset: None,
            style: Some(self.id.clone()),
            number: None,
            type_version: Some(self.version),
            overrides: Vec::new(),
            build: self.build.clone(),
            place: vec![crate::entity::Vec2 { x: 0.0, y: 0.0 }; 4],
            broken: None,
        };
        if f.build.as_ref().is_some_and(OpeningBuild::is_empty) {
            f.build = None;
        }
        f
    }

    /// What is wrong with it: the field and the words.
    pub fn problem(&self) -> Option<(String, String)> {
        if !opening_type_id_ok(&self.id) {
            return Some((
                "id".into(),
                format!(
                    "“{}” tip kimliği olamaz: küçük harf, rakam ve “-” ile en çok 64 harf, “kentos-” ile başlamadan.",
                    self.id
                ),
            ));
        }
        if self.version == 0 {
            return Some((
                "version".into(),
                "Tipin sürümü 1 ya da daha büyük olmalı.".into(),
            ));
        }
        if let Some(s) = &self.source
            && (s.trim().is_empty() || s.chars().count() > 128)
        {
            return Some((
                "source".into(),
                "Tipin kaynağı boş olamaz ve en çok 128 harftir.".into(),
            ));
        }
        if self.build.as_ref().is_some_and(OpeningBuild::is_empty) {
            return Some((
                "build".into(),
                "Boş yapı yazılmaz; yapının en az bir alanı olur.".into(),
            ));
        }
        let mut f = self.fields();
        f.host = "tip".into();
        f.problem()
    }

    /// Its value of `field`, as JSON (null: none).
    pub fn value(&self, field: OpeningTypeField) -> serde_json::Value {
        self.fields().value(field)
    }

    /// `field` set to `v` (its value as JSON, null: none).
    pub fn set_value(
        &mut self,
        field: OpeningTypeField,
        v: &serde_json::Value,
    ) -> Result<(), String> {
        let mut f = self.fields();
        f.set_value(field, v)?;
        self.width = f.width;
        self.leaves = f.leaves;
        self.operation = f.operation;
        self.swing = f.swing;
        self.show_swing = f.show_swing;
        self.frame = f.frame;
        self.sill = f.sill;
        self.head = f.head;
        self.depth = f.depth;
        self.build = f.build;
        Ok(())
    }

    /// The fields whose values differ between `self` and `other`.
    pub fn differs_from(&self, other: &OpeningType) -> Vec<OpeningTypeField> {
        OpeningTypeField::ALL
            .into_iter()
            .filter(|&k| self.value(k) != other.value(k))
            .collect()
    }
}

impl OpeningFields {
    /// Its value of a field a type governs, as JSON (null: none).
    pub fn value(&self, field: OpeningTypeField) -> serde_json::Value {
        use OpeningTypeField::*;
        use serde_json::json;
        let b = self.build.clone().unwrap_or_default();
        match field {
            Width => json!(self.width),
            Leaves => json!(self.leaves),
            Operation => json!(self.operation),
            Swing => json!(self.swing),
            ShowSwing => json!(self.show_swing),
            Frame => json!(self.frame),
            Sill => json!(self.sill),
            Head => json!(self.head),
            Depth => json!(self.depth),
            SizeBy => json!(b.size_by),
            Fit => json!(b.fit),
            LeafThickness => json!(b.leaf_thickness),
            Split => json!(b.split),
            FrameDepth => json!(b.frame_depth),
            FrameOffset => json!(b.frame_offset),
            Threshold => json!(b.threshold),
            Casing => json!(b.casing),
            SillBoard => json!(b.sill_board),
            Mullions => json!(b.mullions),
            Transoms => json!(b.transoms),
            UseZone => json!(b.use_zone),
        }
    }

    /// `field` set to `v` (its value as JSON, null: none); the words when `v` is not one.
    pub fn set_value(
        &mut self,
        field: OpeningTypeField,
        v: &serde_json::Value,
    ) -> Result<(), String> {
        use OpeningTypeField::*;
        fn read<T: serde::de::DeserializeOwned>(
            v: &serde_json::Value,
            field: OpeningTypeField,
        ) -> Result<T, String> {
            serde_json::from_value(v.clone())
                .map_err(|_| format!("“{}” için geçersiz değer: {v}", field.label()))
        }
        let mut b = self.build.clone().unwrap_or_default();
        match field {
            Width => self.width = read(v, field)?,
            Leaves => self.leaves = read(v, field)?,
            Operation => self.operation = read(v, field)?,
            Swing => self.swing = read(v, field)?,
            ShowSwing => self.show_swing = read(v, field)?,
            Frame => self.frame = read(v, field)?,
            Sill => self.sill = read(v, field)?,
            Head => self.head = read(v, field)?,
            Depth => self.depth = read(v, field)?,
            SizeBy => b.size_by = read(v, field)?,
            Fit => b.fit = read(v, field)?,
            LeafThickness => b.leaf_thickness = read(v, field)?,
            Split => b.split = read(v, field)?,
            FrameDepth => b.frame_depth = read(v, field)?,
            FrameOffset => b.frame_offset = read(v, field)?,
            Threshold => b.threshold = read(v, field)?,
            Casing => b.casing = read(v, field)?,
            SillBoard => b.sill_board = read(v, field)?,
            Mullions => b.mullions = read(v, field)?,
            Transoms => b.transoms = read(v, field)?,
            UseZone => b.use_zone = read(v, field)?,
        }
        self.build = (!b.is_empty()).then_some(b);
        Ok(())
    }

    /// Whether it follows a project type (docs/adr/0772 §1): its `style` and the version it took.
    pub fn follows_type(&self) -> bool {
        self.type_version.is_some() && self.style.is_some()
    }

    /// Takes `t`'s values for every field it does not keep as its own, the type's id and version; its form
    /// becomes the type's. An inset stays only on a flue.
    pub fn take_type(&mut self, t: &OpeningType) {
        self.form = t.form;
        for k in OpeningTypeField::ALL {
            if !self.overrides.contains(&k) {
                let _ = self.set_value(k, &t.value(k));
            }
        }
        if t.form != OpeningForm::Flue {
            self.inset = None;
        }
        self.style = Some(t.id.clone());
        self.type_version = Some(t.version);
    }

    /// Its sizes (docs/adr/0772 §2).
    pub fn sizes(&self) -> OpeningSizes {
        opening_sizes(self)
    }
}

/// An opening's widths (docs/adr/0772 §2), metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningSizes {
    /// Kaba boşluk: the hole in the wall (its `width`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rough: f64,
    /// Kasa dış eni: the rough opening less the fitting gap each side.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub unit: f64,
    /// Kanat eni: the light inside the frame.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub leaf: f64,
    /// Each leaf's width, from its start; none for one with no leaves.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub leaves: Vec<f64>,
    /// Net geçiş: what a door leaves free with all its leaves open, from the open leaf's face to the opposite stop; a
    /// bare opening its light; none for a window, a niche or a flue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub clear: Option<f64>,
    /// A two-leaf door's net clear width with only its first leaf open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub clear_first: Option<f64>,
    /// Which width its nominal size names.
    pub size_by: OpeningSizeBy,
    /// That width.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub nominal: f64,
}

/// What a leaf `l` wide, `t` thick, open `angle` degrees takes from the light it closes: its projection across the light
/// (`l·cos θ`, none past 90°) and its thickness (`t·sin θ`).
fn leaf_takes(l: f64, t: f64, angle: f64) -> f64 {
    let r = angle.to_radians();
    (l * libm::cos(r)).max(0.0) + t * libm::sin(r).max(0.0)
}

/// [`OpeningFields::sizes`].
pub fn opening_sizes(f: &OpeningFields) -> OpeningSizes {
    let b = f.build.clone().unwrap_or_default();
    let fit = b.fit.unwrap_or(0.0);
    let frame = f.frame.unwrap_or(0.0);
    let rough = f.width;
    let unit = rough - 2.0 * fit;
    let leaf = unit - 2.0 * frame;
    let t = b.leaf_thickness.unwrap_or(DEFAULT_LEAF_THICKNESS);
    let angle = f.swing_angle();
    let share = b.split.unwrap_or(0.5);
    let door_like = matches!(f.form, OpeningForm::Door | OpeningForm::Window);
    let leaves: Vec<f64> = if !door_like {
        Vec::new()
    } else {
        match (f.operation, f.leaves) {
            (OpeningOperation::Slide, 2) => {
                let w = leaf * (1.0 + SLIDE_LAP) / 2.0;
                vec![w, w]
            }
            (_, 2) => vec![leaf * share, leaf * (1.0 - share)],
            _ => vec![leaf],
        }
    };
    let (clear, clear_first) = match f.form {
        OpeningForm::Door => match (f.operation, leaves.as_slice()) {
            (OpeningOperation::Fixed, _) => (Some(0.0), None),
            (OpeningOperation::Slide, [_]) => (Some(leaf), None),
            (OpeningOperation::Slide, [w, _]) => (Some(leaf - w), None),
            (OpeningOperation::Swing, [w]) => (Some(leaf - leaf_takes(*w, t, angle)), None),
            (OpeningOperation::Swing, [a, b]) => (
                Some(leaf - leaf_takes(*a, t, angle) - leaf_takes(*b, t, angle)),
                Some(a - leaf_takes(*a, t, angle)),
            ),
            _ => (None, None),
        },
        OpeningForm::Opening => (Some(leaf), None),
        _ => (None, None),
    };
    let size_by = b.size_by.unwrap_or_default();
    let nominal = match size_by {
        OpeningSizeBy::Rough => rough,
        OpeningSizeBy::Unit => unit,
        OpeningSizeBy::Leaf => leaf,
    };
    OpeningSizes {
        rough,
        unit,
        leaf,
        leaves,
        clear: clear.map(|c| c.max(0.0)),
        clear_first: clear_first.map(|c| c.max(0.0)),
        size_by,
        nominal,
    }
}

/// The rough opening of a nominal size `value` named by `by`, with a fitting gap `fit` and a frame `frame` each side.
pub fn rough_of(by: OpeningSizeBy, value: f64, fit: f64, frame: f64) -> f64 {
    match by {
        OpeningSizeBy::Rough => value,
        OpeningSizeBy::Unit => value + 2.0 * fit,
        OpeningSizeBy::Leaf => value + 2.0 * fit + 2.0 * frame,
    }
}

/// What is wrong with a project's types: an id or name rule, then each type's.
pub fn opening_types_problem(types: &[OpeningType]) -> Option<(String, String)> {
    if types.len() > MAX_OPENING_TYPES {
        return Some((
            "openingTypes".into(),
            format!("Bir projede en çok {MAX_OPENING_TYPES} kapı ve pencere tipi olur."),
        ));
    }
    if let Some(why) = crate::annotation::names_problem(
        "açıklık tipi",
        types.iter().map(|t| (t.id.as_str(), t.name.as_str())),
    ) {
        return Some((
            "openingTypes".into(),
            format!("Kapı ve pencere tipleri geçersiz: {why}."),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, t) in types.iter().enumerate() {
        if !seen.insert(t.name.to_lowercase()) {
            return Some((
                format!("openingTypes[{i}].name"),
                format!(
                    "“{}” adında iki tip var; adlar büyük küçük harf ayırmadan tek olmalı.",
                    t.name
                ),
            ));
        }
        if let Some((field, words)) = t.problem() {
            return Some((format!("openingTypes[{i}].{field}"), words));
        }
    }
    None
}

/// An office's types as a file carries them (`kentos.opening-types` 1): their library's name and the types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningTypesFile {
    pub format: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    /// The library's name: the source its types take in a project.
    pub library: String,
    pub types: Vec<OpeningType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door(width: f64, frame: f64, build: OpeningBuild) -> OpeningFields {
        OpeningType {
            id: "k01".into(),
            name: "K01".into(),
            version: 1,
            source: None,
            form: OpeningForm::Door,
            width,
            leaves: 1,
            operation: OpeningOperation::Swing,
            swing: None,
            show_swing: true,
            frame: Some(frame),
            sill: None,
            head: Some(2.1),
            depth: None,
            build: Some(build),
        }
        .fields()
    }

    #[test]
    fn a_leaf_and_a_rough_opening_of_ninety_differ() {
        let leaf = door(
            rough_of(OpeningSizeBy::Leaf, 0.9, 0.01, 0.05),
            0.05,
            OpeningBuild {
                size_by: Some(OpeningSizeBy::Leaf),
                fit: Some(0.01),
                ..Default::default()
            },
        );
        let rough = door(0.9, 0.05, OpeningBuild::default());
        let (a, b) = (leaf.sizes(), rough.sizes());
        assert!((a.nominal - 0.9).abs() < 1e-12 && (b.nominal - 0.9).abs() < 1e-12);
        assert!((a.rough - 1.02).abs() < 1e-12);
        assert!((b.leaf - 0.8).abs() < 1e-12);
        // Open at 90°: the light less the leaf's thickness.
        assert!((a.clear.unwrap() - (0.9 - DEFAULT_LEAF_THICKNESS)).abs() < 1e-9);
    }

    #[test]
    fn a_type_follows_but_an_exception_stays() {
        let mut f = door(1.0, 0.05, OpeningBuild::default());
        f.overrides = vec![OpeningTypeField::Frame];
        f.frame = Some(0.07);
        let mut t = OpeningType {
            id: "k01".into(),
            name: "K01".into(),
            version: 2,
            source: None,
            form: OpeningForm::Door,
            width: 1.1,
            leaves: 1,
            operation: OpeningOperation::Swing,
            swing: None,
            show_swing: true,
            frame: Some(0.06),
            sill: None,
            head: Some(2.1),
            depth: None,
            build: None,
        };
        f.take_type(&t);
        assert_eq!(
            (f.width, f.frame, f.type_version),
            (1.1, Some(0.07), Some(2))
        );
        t.version = 0;
        assert!(t.problem().is_some());
        assert_eq!(opening_type_id_of("Kapı K01 / ıslak"), "kapi-k01-islak");
    }
}
