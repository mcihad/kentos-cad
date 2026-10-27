//! `cad.sheets.levels` (“Kat planı paftaları”, docs/adr/0718): one sheet per level from one template through the sheet
//! core's own operations (`kentos_sheet::levels`), the designer's Yeni ▾ making the same; the maps share the chosen
//! levels' objects' box. One change of the book, as `cad.sheets.create`. The shared cases are
//! `fixtures/commands/v1/cad.sheets.levels.json`.

use std::collections::BTreeSet;

use kentos_commands::{
    CommandResult, Entity, SheetPicture, SheetTemplateSource, SheetsEdit, SheetsEdited, SheetsLevels,
};
use kentos_domain::Document;
use kentos_sheet::{create, levels, template};

use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
    sheets_edit, sheets_fit,
};

/// The template when none is given: KentOS's architectural sheet.
pub const DEFAULT_TEMPLATE: &str = "sys:cad-mimari";

/// The box of the objects on `levels` (docs/adr/0715 §1's rule: an opening its wall's, a tag its object's …); none
/// when no object is on them.
pub fn level_extent(doc: &Document, levels: &BTreeSet<String>) -> Option<[f64; 4]> {
    let by_uid = |u: &str| {
        kentos_domain::Uuid::parse_str(u)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| doc.get(s))
    };
    let on: Vec<&Entity> = doc
        .entities()
        .filter(|e| e.level(by_uid).is_some_and(|l| levels.contains(l)))
        .collect();
    sheets_fit::extent_of(doc, on.into_iter())
}

/// The project's levels the input names (all of them without names), id and name, each once in the order given.
fn chosen(doc: &Document, input: &SheetsLevels) -> Result<Vec<(String, String)>, Stop> {
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.as_slice())
        .unwrap_or_default();
    if levels.is_empty() {
        return Err(Stop::failed(error(
            crate::codes::LEVEL_NOT_FOUND,
            "Projede kat yok; kat planı paftası için önce Proje ayarları › Mimari'de kat ekleyin.".into(),
            Some("levels".into()),
        )));
    }
    let Some(ids) = &input.levels else {
        return Ok(levels.iter().map(|l| (l.id.clone(), l.name.clone())).collect());
    };
    if ids.is_empty() {
        return Err(Stop::failed(error(
            crate::codes::LEVEL_NOT_FOUND,
            "Kat listesi boş; katların kimliklerini verin ya da bütün katlar için alanı bırakın.".into(),
            Some("levels".into()),
        )));
    }
    let mut out: Vec<(String, String)> = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let Some(l) = levels.iter().find(|l| &l.id == id) else {
            return Err(Stop::failed(error(
                crate::codes::LEVEL_NOT_FOUND,
                kentos_commands::level_problem(Some(id), levels)
                    .unwrap_or_else(|| format!("Projede “{id}” kimlikli kat yok.")),
                Some(format!("levels[{i}]")),
            )));
        };
        if !out.iter().any(|(o, _)| o == id) {
            out.push((l.id.clone(), l.name.clone()));
        }
    }
    Ok(out)
}

fn prepare(cx: &ExecutionContext<'_>, input: &SheetsLevels) -> Result<SheetsEdit, Stop> {
    checks::revision(cx.doc, input.expected_revision.as_deref())?;
    let chosen = chosen(cx.doc, input)?;
    let template = match &input.source {
        None => template::system_template(DEFAULT_TEMPLATE).cloned().ok_or_else(|| {
            Stop::failed(error(
                "unknown_template",
                format!("“{DEFAULT_TEMPLATE}” sistem şablonu yok."),
                Some("source".into()),
            ))
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
    let options: template::InstanceOptions = serde_json::from_value(input.options.clone()).map_err(|e| {
        Stop::failed(error(
            "invalid_options",
            format!("Şablon seçenekleri okunamadı: {e}"),
            Some("options".into()),
        ))
    })?;
    if options.name.is_some() {
        return Err(Stop::failed(error(
            "invalid_options",
            "Kat planı paftalarının adları katlarından gelir (“A-101 Zemin kat planı”); options.name verilmez.".into(),
            Some("options.name".into()),
        )));
    }
    let ids: BTreeSet<String> = chosen.iter().map(|(id, _)| id.clone()).collect();
    let bounds = level_extent(cx.doc, &ids).or_else(|| sheets_fit::extent_of(cx.doc, cx.doc.entities()));
    let (book, _) = match cx.doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error("book_unreadable", e.message, None)))?,
        None => (Default::default(), Vec::new()),
    };
    let how = levels::LevelSheetsOptions {
        below: input.below,
        again: input.again,
        first: input.first.clone(),
        bounds,
    };
    let made = levels::level_sheets(&book, &template, &options, &chosen, &how, |sheet| {
        create::ids_for(&template, sheet)
    })
    .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    Ok(SheetsEdit {
        ops: made
            .ops
            .iter()
            .map(|op| serde_json::to_value(op).unwrap_or_default())
            .collect(),
        pictures: made
            .pictures
            .into_iter()
            .map(|p| {
                Ok(SheetPicture {
                    sha256: p.meta.sha256,
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
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SheetsLevels) -> CommandResult<()> {
    match prepare(cx, input) {
        Ok(edit) => sheets_edit::validate(cx, &edit),
        Err(e) => e.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SheetsLevels) -> CommandResult<SheetsEdited> {
    match prepare(cx, input) {
        Ok(edit) => sheets_edit::plan(cx, &edit),
        Err(e) => e.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: SheetsLevels) -> CommandResult<SheetsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SHEETS_LEVELS,
        kentos_commands::CAD_SHEETS_LEVELS_VERSION,
        input,
        |cx, input| match prepare(cx, &input) {
            Ok(edit) => sheets_edit::execute(cx, edit),
            Err(e) => e.into(),
        },
    )
}
