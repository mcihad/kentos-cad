//! Native, versioned plot tables: indexed colours and named styles are different key spaces.
use serde::{Serialize, Deserialize};
#[cfg(feature = "ts")]
use ts_rs::TS;
use crate::{
    Result, SheetError,
    style::{LineCap, LineJoin},
    SheetBook,
};
pub const SCHEMA: &str = "kentos.plot/1";
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotColor"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlotColor {
    #[default]
    Object,
    Black,
    Grayscale,
    Color {
        value: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotStyle"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlotStyle {
    #[serde(default)]
    pub color: PlotColor,
    #[serde(default = "full")]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="percent")))]
    pub screening: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
    pub line_weight: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
    pub dash: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="percent")))]
    pub opacity: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cap: Option<LineCap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub join: Option<LineJoin>,
}
fn full() -> u8 {
    100
}
impl PlotStyle {
    pub fn object() -> Self {
        Self::default()
    }
}
impl Default for PlotStyle {
    fn default() -> Self {
        Self {
            color: PlotColor::Object,
            screening: 100,
            line_weight: None,
            dash: None,
            opacity: None,
            cap: None,
            join: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotStyleKey"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum StyleKey {
    Aci {
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
        index: u8,
    },
    Named {
        name: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotStyleEntry"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyleEntry {
    pub key: StyleKey,
    pub style: PlotStyle,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotTableKind"))]
#[serde(rename_all = "camelCase")]
pub enum TableKind {
    ColorDependent,
    Named,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotTarget"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Target {
    ModelLayer { id: String },
    PaperItem { id: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotAssignment"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Assignment {
    pub target: Target,
    pub key: StyleKey,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotProfile"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlotProfile {
    pub id: String,
    pub name: String,
    pub kind: TableKind,
    pub entries: Vec<StyleEntry>,
    #[serde(default)]
    pub assignments: Vec<Assignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub default_style: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperSavePlotProfiles"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavePlotProfiles {
    pub profiles: Vec<PlotProfile>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exchange {
    schema: String,
    profile: PlotProfile,
}
fn name(v: &str) -> bool {
    !v.trim().is_empty() && v.len() <= 256 && !v.chars().any(char::is_control)
}
fn bad(code: &str, message: &str) -> SheetError {
    SheetError::new(code, message)
}
pub fn validate(p: &PlotProfile) -> Result<()> {
    if !crate::validate::valid_id(&p.id) || !name(&p.name) {
        return Err(bad(
            "bad_plot_profile",
            "Profil kimliği ve adı geçerli olmalı.",
        ));
    }
    if p.entries.len() > 4096 || p.assignments.len() > 100000 {
        return Err(bad(
            "plot_profile_too_large",
            "Profil 4096 stil / 100000 atama sınırını aşıyor.",
        ));
    }
    let mut keys = std::collections::BTreeSet::new();
    for e in &p.entries {
        let key = match (&p.kind, &e.key) {
            (TableKind::ColorDependent, StyleKey::Aci { index }) if *index > 0 => {
                format!("aci:{index}")
            }
            (TableKind::Named, StyleKey::Named { name: n })
                if name(n) && !n.eq_ignore_ascii_case("normal") =>
            {
                format!("named:{n}")
            }
            _ => {
                return Err(bad(
                    "plot_style_key_mismatch",
                    "CTB anahtarı ACI 1–255, STB anahtarı ayrı bir ad olmalı; Normal nesne görünüşüdür.",
                ));
            }
        };
        if !keys.insert(key) {
            return Err(bad(
                "duplicate_plot_style",
                "Aynı baskı stili iki kez tanımlanamaz.",
            ));
        }
        let s = &e.style;
        if s.screening > 100
            || s.opacity.is_some_and(|v| v > 100)
            || s.line_weight.is_some_and(|v| !(0..=100000).contains(&v))
            || s.dash
                .as_ref()
                .is_some_and(|v| v.len() > 64 || v.iter().any(|d| *d <= 0 || *d > 1000000))
        {
            return Err(bad(
                "bad_plot_style",
                "Tarama ve opaklık %0–100, kalınlık 0–100 mm, çizgi aralıkları pozitif olmalı.",
            ));
        }
        if let PlotColor::Color { value } = &s.color
            && !crate::style::is_color(value)
        {
            return Err(bad(
                "bad_plot_color",
                "Baskı rengi geçerli bir onaltılık renk olmalı.",
            ));
        }
    }
    if let Some(n) = &p.default_style
        && (p.kind != TableKind::Named
            || !p
                .entries
                .iter()
                .any(|e| e.key == StyleKey::Named { name: n.clone() }))
    {
        return Err(bad(
            "unknown_plot_style",
            "Varsayılan adlı baskı stili tabloda yok.",
        ));
    }
    let mut targets = std::collections::BTreeSet::new();
    for a in &p.assignments {
        let id = match &a.target {
            Target::ModelLayer { id } | Target::PaperItem { id } => id,
        };
        if !crate::validate::valid_id(id)
            || !targets.insert(format!("{:?}", a.target))
            || !p.entries.iter().any(|e| e.key == a.key)
        {
            return Err(bad(
                "bad_plot_assignment",
                "Hedef geçerli ve tek olmalı; atanan stil aynı profilde bulunmalı.",
            ));
        }
    }
    Ok(())
}
pub fn validate_book(b: &SheetBook) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for p in &b.plot_profiles {
        validate(p)?;
        if !ids.insert(&p.id) || !names.insert(&p.name) {
            return Err(bad(
                "duplicate_plot_profile",
                "Baskı profilinin kimliği ve adı tek olmalı.",
            ));
        }
    }
    Ok(())
}
pub fn read(bytes: &[u8]) -> Result<PlotProfile> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(bad(
            "plot_profile_too_large",
            "Profil 4 MiB sınırını aşıyor.",
        ));
    }
    let x:Exchange=serde_json::from_slice(bytes).map_err(|_|bad("unsupported_plot_profile","Geçerli bir kentos.plot/1 (.kplot) dosyası seçin; ikili CTB/STB bu okuyucuda desteklenmiyor."))?;
    if x.schema != SCHEMA {
        return Err(bad(
            "unknown_plot_profile_schema",
            "Baskı profilinin sürümü desteklenmiyor.",
        ));
    }
    validate(&x.profile)?;
    Ok(x.profile)
}
pub fn write(profile: &PlotProfile) -> Result<Vec<u8>> {
    validate(profile)?;
    serde_json::to_vec_pretty(&Exchange {
        schema: SCHEMA.into(),
        profile: profile.clone(),
    })
    .map_err(|_| bad("plot_profile_unwritable", "Profil yazılamadı."))
}
impl PlotProfile {
    pub fn style(&self, target: &Target) -> Option<&PlotStyle> {
        let key = self
            .assignments
            .iter()
            .find(|a| &a.target == target)
            .map(|a| a.key.clone())
            .or_else(|| {
                self.default_style
                    .as_ref()
                    .map(|n| StyleKey::Named { name: n.clone() })
            })?;
        self.entries.iter().find(|e| e.key == key).map(|e| &e.style)
    }
}

/// An applied/template profile can be self-contained; unresolved legacy/library references remain explicit errors.
pub fn for_sheet<'a>(book: &'a SheetBook, id: &str) -> Result<Option<&'a PlotProfile>> {
    let sheet = book
        .sheet(id)
        .ok_or_else(|| bad("unknown_sheet", "Pafta bulunamadı."))?;
    let Some(plot) = &sheet.plot else {
        return Ok(None);
    };
    let Some(id) = &plot.settings.profile else {
        return Ok(None);
    };
    plot.settings
        .profile_snapshot
        .as_ref()
        .filter(|p| &p.id == id)
        .or_else(|| book.plot_profiles.iter().find(|p| &p.id == id))
        .map(Some)
        .ok_or_else(|| {
            bad(
                "unknown_plot_profile",
                "Seçilen baskı profili bulunamadı; geçerli bir profil seçin.",
            )
        })
}
