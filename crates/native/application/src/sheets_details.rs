//! `cad.sheets.details` (“Detay paftaları”, docs/adr/1090 §2): general 2D detail sheets through the sheet core's own
//! operations (`kentos_sheet::details`); a model table's part (docs/adr/1030 §7) as a detail's source, so a broken
//! table runs on over sheets (docs/adr/1090 §3). One change of the book, as `cad.sheets.levels`. The shared cases are
//! `fixtures/commands/v1/cad.sheets.details.json`.

use kentos_commands::{
    CommandResult, DetailRequest, SheetPicture, SheetTemplateSource, SheetsDetailed, SheetsDetails,
    SheetsEdit,
};
use kentos_domain::Document;
use kentos_geometry_core::geom::table::table_geom;
use kentos_sheet::{
    create,
    details::{self, DetailSheetsOptions, SheetDetail},
    template,
};

use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
    geometry::shape,
    sheets_edit,
};

/// The template when none is given: KentOS's ISO technical sheet.
pub const DEFAULT_TEMPLATE: &str = "sys:cad-teknik";

/// The model box of part `part` of the table `uid` (docs/adr/1090 §3): its outline's corners from the table's top left
/// along its rows, its frame band and half a text height around it.
pub(crate) fn table_part_bounds(
    doc: &Document,
    uid: &str,
    part: u32,
    path: &str,
) -> Result<[f64; 4], Stop> {
    let wanted = [uid.to_owned()];
    let named = checks::named(doc, &wanted).map_err(|_| {
        Stop::failed(error(
            crate::codes::ENTITY_NOT_FOUND,
            format!("“{uid}” kimlikli nesne çizimde yok."),
            Some(format!("{path}.table")),
        ))
    })?;
    let entity = named[0].2;
    let s = shape(entity);
    let Some(t) = table_geom(&s) else {
        return Err(Stop::failed(error(
            "not_a_table",
            "Detayın kaynağı bir model tablosu olmalı; tabloyu seçin.".into(),
            Some(format!("{path}.table")),
        )));
    };
    let parts = t.parts();
    let Some(p) = parts.get(part as usize) else {
        return Err(Stop::failed(error(
            "table_part_missing",
            format!(
                "Tablonun {} parçası var; parça 0 ile {} arasında olmalı.",
                parts.len(),
                parts.len().saturating_sub(1)
            ),
            Some(format!("{path}.table.part")),
        )));
    };
    let width: f64 = t.columns.iter().sum();
    let depth: f64 = p
        .rows
        .iter()
        .map(|&r| t.rows.get(r).copied().unwrap_or(0.0))
        .sum();
    let grow = t.frame.unwrap_or(0.0).max(0.0) + t.height.max(0.0) / 2.0;
    let corners = [
        t.point(p.x, 0.0),
        t.point(p.x + width, 0.0),
        t.point(p.x + width, depth),
        t.point(p.x, depth),
    ];
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for c in corners {
        b = [b[0].min(c.x), b[1].min(c.y), b[2].max(c.x), b[3].max(c.y)];
    }
    Ok([b[0] - grow, b[1] - grow, b[2] + grow, b[3] + grow])
}

/// A request's detail: its extent or its table part's, its layers, overrides and annotation read by the sheet core.
fn detail_of(doc: &Document, r: &DetailRequest, path: &str) -> Result<SheetDetail, Stop> {
    let bounds = match (&r.bounds, &r.table) {
        (Some(b), None) => *b,
        (None, Some(t)) => table_part_bounds(doc, &t.table, t.part, path)?,
        _ => {
            return Err(Stop::failed(error(
                "ambiguous_source",
                "Detayın kaynağı ya bounds (model kapsamı) ya da table (tablo parçası) olmalı; ikisinden yalnız biri."
                    .into(),
                Some(path.to_owned()),
            )));
        }
    };
    let read = |what: &str, v: &serde_json::Value| {
        Stop::failed(error(
            "bad_value",
            format!("Detayın {what} okunamadı: {v}"),
            Some(format!("{path}.{what}")),
        ))
    };
    Ok(SheetDetail {
        name: r.name.clone(),
        bounds,
        ratio: r.ratio,
        layers: match &r.layers {
            Some(v) => Some(serde_json::from_value(v.clone()).map_err(|_| read("layers", v))?),
            None => None,
        },
        layer_overrides: match &r.layer_overrides {
            Some(list) => list
                .iter()
                .map(|v| serde_json::from_value(v.clone()).map_err(|_| read("layerOverrides", v)))
                .collect::<Result<_, _>>()?,
            None => Vec::new(),
        },
        annotation: match &r.annotation {
            Some(v) => Some(serde_json::from_value(v.clone()).map_err(|_| read("annotation", v))?),
            None => None,
        },
        locked: r.locked.unwrap_or(true),
    })
}

fn prepare(
    cx: &ExecutionContext<'_>,
    input: &SheetsDetails,
) -> Result<(SheetsEdit, details::DetailSheets), Stop> {
    checks::revision(cx.doc, input.expected_revision.as_deref())?;
    let wanted: Vec<SheetDetail> = input
        .details
        .iter()
        .enumerate()
        .map(|(i, r)| detail_of(cx.doc, r, &format!("details[{i}]")))
        .collect::<Result<_, _>>()?;
    let template = match &input.source {
        None => template::system_template(DEFAULT_TEMPLATE).cloned().ok_or_else(|| {
            Stop::failed(error("unknown_template", format!("“{DEFAULT_TEMPLATE}” sistem şablonu yok."), Some("source".into())))
        })?,
        Some(SheetTemplateSource::System { id }) => template::system_template(id).cloned().ok_or_else(|| {
            Stop::failed(error(
                "unknown_template",
                format!("“{id}” sistem şablonu yok; cad.sheets.set(details=true) ile şablonları listeleyin."),
                Some("source.id".into()),
            ))
        })?,
        Some(SheetTemplateSource::Custom { template }) => template::read_template(&template.to_string())
            .map_err(|e| Stop::failed(error(&e.code, e.message, e.path.map(|p| format!("source.template.{p}")))))?,
    };
    let options: template::InstanceOptions = serde_json::from_value(input.options.clone())
        .map_err(|e| {
            Stop::failed(error(
                "invalid_options",
                format!("Şablon seçenekleri okunamadı: {e}"),
                Some("options".into()),
            ))
        })?;
    if options.name.is_some() || options.center.is_some() || options.scale.is_some() {
        return Err(Stop::failed(error(
            "invalid_options",
            "Detay paftalarının adları, merkezleri ve ölçekleri detaylardan gelir; options yalnız kâğıt ve şablon yanıtlarını alır."
                .into(),
            Some("options".into()),
        )));
    }
    let (book, _) = match cx.doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error("book_unreadable", e.message, None)))?,
        None => (Default::default(), Vec::new()),
    };
    let how = DetailSheetsOptions {
        first: input.first.clone(),
        title: input.title.clone(),
    };
    let made = details::detail_sheets(&book, &template, &options, &wanted, &how, |sheet| {
        create::ids_for(&template, sheet)
    })
    .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    let edit = SheetsEdit {
        ops: made
            .ops
            .iter()
            .map(|op| serde_json::to_value(op).unwrap_or_default())
            .collect(),
        pictures: made
            .pictures
            .iter()
            .map(|p| {
                Ok(SheetPicture {
                    sha256: p.meta.sha256.clone(),
                    bytes: template::base64_decode(&p.data).ok_or_else(|| {
                        Stop::failed(error(
                            "bad_asset",
                            "Şablon resmi okunamadı.".into(),
                            Some("source.template.assets".into()),
                        ))
                    })?,
                })
            })
            .collect::<Result<_, Stop>>()?,
        replace_pictures: false,
        expected_revision: input.expected_revision.clone(),
    };
    Ok((edit, made))
}

fn detailed(
    result: CommandResult<kentos_commands::SheetsEdited>,
    made: &details::DetailSheets,
) -> CommandResult<SheetsDetailed> {
    match result {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: SheetsDetailed {
                edited: output,
                made: made
                    .sheets
                    .iter()
                    .map(|s| serde_json::to_value(s).unwrap_or_default())
                    .collect(),
                details: made
                    .details
                    .iter()
                    .map(|d| serde_json::to_value(d).unwrap_or_default())
                    .collect(),
            },
            warnings,
        },
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Failed { error } => CommandResult::Failed { error },
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SheetsDetails) -> CommandResult<()> {
    match prepare(cx, input) {
        Ok((edit, _)) => sheets_edit::validate(cx, &edit),
        Err(e) => e.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SheetsDetails) -> CommandResult<SheetsDetailed> {
    match prepare(cx, input) {
        Ok((edit, made)) => detailed(sheets_edit::plan(cx, &edit), &made),
        Err(e) => e.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SheetsDetails,
) -> CommandResult<SheetsDetailed> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SHEETS_DETAILS,
        kentos_commands::CAD_SHEETS_DETAILS_VERSION,
        input,
        |cx, input| match prepare(cx, &input) {
            Ok((edit, made)) => detailed(sheets_edit::execute(cx, edit), &made),
            Err(e) => e.into(),
        },
    )
}
