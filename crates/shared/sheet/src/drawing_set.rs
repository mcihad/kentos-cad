//! Standard drawing-set metadata and deterministic sheet/view index rows.
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;
use crate::{
    error::{Result, SheetError},
    model::SheetBook,
    ItemKind,
};

pub use kentos_contracts::DrawingInfo;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperSetDrawingInfo"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetDrawingInfo {
    pub id: String,
    /// None removes metadata and restores legacy variable/title behavior.
    pub info: Option<DrawingInfo>,
}
/// One atomic metadata batch also supports exchanging two sheet numbers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperSetDrawingSet"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetDrawingSet {
    pub entries: Vec<SetDrawingInfo>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperDrawingIndex"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrawingIndex {
    /// The chosen subset and its descendants; root selects every sheet in book order.
    #[serde(default)]
    pub subset: Vec<String>,
    /// False: a row per sheet. True: a row per placed model viewport, including master viewports.
    #[serde(default)]
    pub views: bool,
}
pub fn validate_info(info: &DrawingInfo, path: &str) -> Result<()> {
    for (key, value, limit) in [
        ("number", &info.number, 64),
        ("title", &info.title, 240),
        ("revision", &info.revision, 64),
        ("status", &info.status, 120),
    ] {
        if value.chars().count() > limit
            || value.chars().any(char::is_control)
            || value.trim() != value
        {
            return Err(SheetError::at(
                "bad_drawing_info",
                format!("{path}.{key}"),
                format!(
                    "Pafta alanı başta/sonda boşluk veya kontrol karakteri içeremez; en çok {limit} karakter girin."
                ),
            ));
        }
    }
    validate_subset(&info.subset, path)
}
pub fn validate_subset(subset: &[String], path: &str) -> Result<()> {
    if subset.len() > 12
        || subset.iter().any(|v| {
            v.trim().is_empty()
                || v.trim() != v
                || v.chars().count() > 120
                || v.contains(['/', '\\'])
                || v.chars().any(char::is_control)
        })
    {
        return Err(SheetError::at(
            "bad_drawing_info",
            format!("{path}.subset"),
            "Alt küme en çok 12 dolu düzeyden oluşmalı; düzey adında / veya kontrol karakteri kullanmayın.",
        ));
    }
    Ok(())
}
pub fn validate_book(book: &SheetBook) -> Result<()> {
    let mut numbers = std::collections::BTreeSet::new();
    for s in &book.sheets {
        if let Some(info) = &s.drawing_info {
            validate_info(info, &format!("sheets.{}.drawingInfo", s.id))?;
            if !info.number.is_empty() && !numbers.insert(crate::expr::fold(&info.number)) {
                return Err(SheetError::at(
                    "duplicate_sheet_number",
                    s.id.clone(),
                    format!(
                        "“{}” pafta numarası tekrar ediyor; her paftaya ayrı numara verin.",
                        info.number
                    ),
                ));
            }
        }
    }
    Ok(())
}
/// Six standard cells: number, title, revision, status, subset and model view name.
/// Values are literal data; they are never evaluated as expressions copied from a title.
pub fn rows(
    book: &SheetBook,
    source: &DrawingIndex,
    named_views: &[kentos_contracts::NamedView],
) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for sheet in &book.sheets {
        let info = sheet.drawing_info.clone().unwrap_or_default();
        if !info.subset.starts_with(&source.subset) {
            continue;
        }
        let base = vec![
            info.number,
            if info.title.is_empty() {
                sheet.name.clone()
            } else {
                info.title
            },
            info.revision,
            info.status,
            info.subset.join(" / "),
        ];
        if source.views {
            let Ok(scene) = crate::scene::Scene::new(book, &sheet.id) else {
                continue;
            };
            for (item, _) in scene.all() {
                let mut parent = item.group.as_deref().and_then(|id| scene.item(id));
                let mut printable = item.printable;
                let mut depth = 0;
                while let Some(p) = parent {
                    printable &= p.printable;
                    depth += 1;
                    if depth > scene.sheet.items.len() + scene.master.len() {
                        break;
                    }
                    parent = p.group.as_deref().and_then(|id| scene.item(id));
                }
                if !scene.shown(item) || !printable {
                    continue;
                }
                if let ItemKind::Map(m) = &item.kind {
                    if m.overview_of.is_some()
                        || (m.source.is_drawing()
                            && matches!(&m.view, crate::MapView::Fixed(v) if v.center.is_none()))
                    {
                        continue;
                    }
                    let view = match &m.source {
                        crate::viewport::Source::ViewLink { view } => {
                            named_views.iter().find(|v| v.id == view.id).map_or_else(
                                || format!("{} (kaynak eksik)", view.name),
                                |v| v.name.clone(),
                            )
                        }
                        crate::viewport::Source::ViewSnapshot { view } => view.name.clone(),
                        _ => item.name.clone(),
                    };
                    let mut row = base.clone();
                    row.push(view);
                    out.push(row);
                }
            }
        } else {
            let mut row = base;
            row.push(String::new());
            out.push(row);
        }
    }
    out
}
