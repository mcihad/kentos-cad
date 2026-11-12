//! `cad.sheets.create`: the gallery's common template creation, including pictures (ADR 0592).
use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
    sheets_edit,
};
use kentos_commands::{
    CommandResult, SheetPicture, SheetTemplateSource, SheetsCreate, SheetsEdit, SheetsEdited,
};
use kentos_sheet::{create, template};

fn prepare(cx: &ExecutionContext<'_>, input: &SheetsCreate) -> Result<SheetsEdit, Stop> {
    checks::revision(cx.doc, input.expected_revision.as_deref())?;
    let template=match &input.source {
        SheetTemplateSource::System{id}=>template::system_template(id).cloned().ok_or_else(||Stop::failed(error("unknown_template",format!("“{id}” sistem şablonu yok; cad.sheets.set(details=true) ile şablonları listeleyin."),Some("source.id".into()))))?,
        SheetTemplateSource::Custom{template}=>template::read_template(&template.to_string()).map_err(|e|Stop::failed(error(&e.code,e.message,e.path.map(|p|format!("source.template.{p}")))))?,
    };
    let options = serde_json::from_value(input.options.clone()).map_err(|e| {
        Stop::failed(error(
            "invalid_options",
            format!("Şablon seçenekleri okunamadı: {e}"),
            Some("options".into()),
        ))
    })?;
    let made = create::from_template(
        &template,
        &create::ids_for(&template, &input.id),
        &options,
        input.index,
    )
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
pub fn validate(cx: &ExecutionContext<'_>, input: &SheetsCreate) -> CommandResult<()> {
    match prepare(cx, input) {
        Ok(edit) => sheets_edit::validate(cx, &edit),
        Err(e) => e.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &SheetsCreate) -> CommandResult<SheetsEdited> {
    match prepare(cx, input) {
        Ok(edit) => sheets_edit::plan(cx, &edit),
        Err(e) => e.into(),
    }
}
pub fn execute(cx: &mut ExecutionContext<'_>, input: SheetsCreate) -> CommandResult<SheetsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SHEETS_CREATE,
        1,
        input,
        |cx, input| match prepare(cx, &input) {
            Ok(edit) => sheets_edit::execute(cx, edit),
            Err(e) => e.into(),
        },
    )
}
