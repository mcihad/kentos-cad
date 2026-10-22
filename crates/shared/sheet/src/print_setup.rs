//! Named page setups and an explicit paper-to-output transform. Model and viewport scales stay untouched.
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;
use crate::{
    Page, SheetBook, SheetError, Result,
    units::{RectUm, PointUm},
    viewport::ScaleRatio,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotArea"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlotArea {
    #[default]
    Layout,
    Extents,
    Window {
        rect: RectUm,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotScale"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlotScale {
    Ratio { ratio: ScaleRatio },
    Fit,
}
impl Default for PlotScale {
    fn default() -> Self {
        Self::Ratio {
            ratio: ScaleRatio {
                paper: 1.,
                model: 1.,
            },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPrintDevice"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum PrintDevice {
    #[default]
    Pdf,
    System {
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotSettings"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlotSettings {
    #[serde(default)]
    pub area: PlotArea,
    #[serde(default)]
    pub scale: PlotScale,
    #[serde(default)]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
    pub offset: PointUm,
    #[serde(default)]
    pub center: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default)]
    pub device: PrintDevice,
}
impl Default for PlotSettings {
    fn default() -> Self {
        Self {
            area: PlotArea::Layout,
            scale: Default::default(),
            offset: Default::default(),
            center: false,
            profile: None,
            device: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperNamedPageSetup"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamedPageSetup {
    pub id: String,
    pub name: String,
    pub page: Page,
    #[serde(default)]
    pub settings: PlotSettings,
}

/// Applying a reusable setup captures its options; editing/removing the preset cannot silently change issued sheets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperAppliedPageSetup"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppliedPageSetup {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    pub settings: PlotSettings,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperSavePageSetups"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavePageSetups {
    pub setups: Vec<NamedPageSetup>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperApplyPageSetup"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyPageSetup {
    pub sheet: String,
    pub setup: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperSetSheetPlot"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetSheetPlot {
    pub sheet: String,
    #[serde(default)]
    pub plot: Option<AppliedPageSetup>,
}

pub fn validate_settings(s: &PlotSettings) -> Result<()> {
    if let PlotArea::Window { rect } = &s.area
        && (rect.width <= 0
            || rect.height <= 0
            || [rect.left, rect.top, rect.width, rect.height]
                .iter()
                .any(|v| i64::from(*v).abs() > i64::from(crate::validate::FRAME_LIMIT)))
    {
        return Err(SheetError::new(
            "bad_print_area",
            "Baskı penceresi pozitif boylu ve kâğıt koordinat sınırında olmalı.",
        ));
    }
    if let PlotScale::Ratio { ratio } = &s.scale
        && (!ratio.valid() || !(1e-6..=1e6).contains(&(ratio.paper / ratio.model)))
    {
        return Err(SheetError::new(
            "bad_plot_scale",
            "Baskı oranı pozitif, sonlu ve 1/1000000–1000000 aralığında olmalı.",
        ));
    }
    if [s.offset[0], s.offset[1]]
        .iter()
        .any(|v| i64::from(*v).abs() > i64::from(crate::validate::FRAME_LIMIT))
    {
        return Err(SheetError::new(
            "out_of_range",
            "Baskı kayması kâğıt koordinat sınırını aşıyor.",
        ));
    }
    if let Some(profile) = &s.profile
        && (!crate::validate::valid_id(profile) || profile.trim().is_empty())
    {
        return Err(SheetError::new(
            "bad_plot_profile",
            "Baskı profilinin kimliği boş veya geçersiz.",
        ));
    }
    if let PrintDevice::System { name } = &s.device
        && (name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control))
    {
        return Err(SheetError::new(
            "bad_print_device",
            "Hedef yazıcı adı boş veya geçersiz.",
        ));
    }
    Ok(())
}
pub fn validate_book(book: &SheetBook) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for setup in &book.page_setups {
        if !crate::validate::valid_id(&setup.id) || !ids.insert(&setup.id) {
            return Err(SheetError::new(
                "duplicate_id",
                "Sayfa düzeninin kimliği geçersiz veya yineleniyor.",
            ));
        }
        if setup.name.trim().is_empty()
            || setup.name.len() > 256
            || setup.name.chars().any(char::is_control)
            || !names.insert(setup.name.trim())
        {
            return Err(SheetError::new(
                "duplicate_name",
                "Sayfa düzeninin adı geçersiz veya yineleniyor.",
            ));
        }
        crate::validate::validate_page("pageSetups.page", &setup.page)?;
        validate_settings(&setup.settings)?;
    }
    for sheet in &book.sheets {
        if let Some(plot) = &sheet.plot {
            validate_settings(&plot.settings)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub source: RectUm,
    pub clip: RectUm,
    pub factor: f64,
    pub translate: [f64; 2],
}
/// Layouts always print 1:1. Window/extents use an explicit paper ratio or fit, separately from all viewport ratios.
pub fn plan(page: &Page, settings: &PlotSettings, extents: Option<RectUm>) -> Result<Plan> {
    validate_settings(settings)?;
    if settings.area == PlotArea::Layout {
        return Ok(Plan {
            source: page.rect(),
            clip: page.margin_rect(),
            factor: 1.,
            translate: [f64::from(settings.offset[0]), f64::from(settings.offset[1])],
        });
    }
    let source = match settings.area {
        PlotArea::Window { rect } => rect,
        PlotArea::Extents => extents.unwrap_or(page.rect()),
        PlotArea::Layout => page.rect(),
    };
    let clip = page.margin_rect();
    let factor = match settings.scale {
        PlotScale::Fit => (f64::from(clip.width) / f64::from(source.width))
            .min(f64::from(clip.height) / f64::from(source.height)),
        PlotScale::Ratio { ratio } => ratio.paper / ratio.model,
    };
    if !factor.is_finite() || factor <= 0. {
        return Err(SheetError::new(
            "bad_plot_scale",
            "Baskı alanı sayfanın yazılabilir bölümüne sığmıyor.",
        ));
    }
    let center = if settings.center {
        [
            (f64::from(clip.width) - f64::from(source.width) * factor) / 2.,
            (f64::from(clip.height) - f64::from(source.height) * factor) / 2.,
        ]
    } else {
        [0., 0.]
    };
    Ok(Plan {
        source,
        clip,
        factor,
        translate: [
            f64::from(clip.left + settings.offset[0]) - f64::from(source.left) * factor + center[0],
            f64::from(clip.top + settings.offset[1]) - f64::from(source.top) * factor + center[1],
        ],
    })
}

/// Printed item frames, including the master relaid out on this paper. Groups themselves carry no ink.
pub fn extents(book: &SheetBook, sheet: &str) -> Result<Option<RectUm>> {
    let scene = crate::scene::Scene::new(book, sheet)?;
    let mut bounds: Option<[f64; 4]> = None;
    for (item, _) in scene.all().filter(|(i, _)| {
        i.printable && scene.shown(i) && !matches!(i.kind, crate::ItemKind::Group(_))
    }) {
        for p in crate::units::corners(&item.frame, item.rotation) {
            bounds = Some(match bounds {
                None => [p[0], p[1], p[0], p[1]],
                Some(b) => [
                    b[0].min(p[0]),
                    b[1].min(p[1]),
                    b[2].max(p[0]),
                    b[3].max(p[1]),
                ],
            });
        }
    }
    Ok(bounds.map(|b| {
        RectUm::new(
            b[0].floor() as i32,
            b[1].floor() as i32,
            (b[2].ceil() - b[0].floor()).max(1.) as i32,
            (b[3].ceil() - b[1].floor()).max(1.) as i32,
        )
    }))
}
pub fn plan_for(book: &SheetBook, sheet: &str) -> Result<Option<Plan>> {
    let sheet = book
        .sheet(sheet)
        .ok_or_else(|| SheetError::new("unknown_sheet", "Pafta bulunamadı."))?;
    sheet
        .plot
        .as_ref()
        .map(|plot| {
            if plot.settings.profile.is_some() {
                return Err(SheetError::new(
                    "unknown_plot_profile",
                    "Baskı profili kitapta bulunamadı; geçerli bir profil seçin.",
                ));
            }
            plan(&sheet.page, &plot.settings, extents(book, &sheet.id)?)
        })
        .transpose()
}
