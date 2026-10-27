//! `cad.sheets.fit`: source objects stay 1:1; only the paper's fixed view changes (ADR 0592).
use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
    geometry, sheets_edit,
};
use kentos_commands::{CommandResult, SheetsEdit, SheetsEdited, SheetsFit};
use kentos_domain::Document;
use kentos_geometry_core::ops::arrange::object_bounds;

/// Same canonical object boxes for UI context and script requests, including nested block geometry.
pub fn object_extent(
    doc: &Document,
    uids: Option<&[String]>,
) -> Result<Option<[f64; 4]>, Box<kentos_commands::CommandError>> {
    let entities = match uids {
        Some(ids) => checks::named(doc, ids)
            .map_err(|s| Box::new(s.into_error()))?
            .into_iter()
            .map(|(_, _, e, _)| e)
            .collect::<Vec<_>>(),
        None => doc.entities().collect(),
    };
    Ok(extent_of(doc, entities.into_iter()))
}

/// The box of `entities` as the paper fits them (nested blocks' geometry too); none for none with a finite box.
pub fn extent_of<'a>(
    doc: &Document,
    entities: impl Iterator<Item = &'a kentos_commands::Entity>,
) -> Option<[f64; 4]> {
    let blocks = crate::blocks::core_blocks(doc.blocks());
    let font = geometry::drawing_font(doc.settings().drawing_font);
    let mut extent: Option<[f64; 4]> = None;
    for e in entities {
        let b = object_bounds(&geometry::shape(e), &blocks, font);
        if ![b.min_x, b.min_y, b.max_x, b.max_y]
            .iter()
            .all(|x| x.is_finite())
        {
            continue;
        }
        extent = Some(match extent {
            None => [b.min_x, b.min_y, b.max_x, b.max_y],
            Some(a) => [
                a[0].min(b.min_x),
                a[1].min(b.min_y),
                a[2].max(b.max_x),
                a[3].max(b.max_y),
            ],
        });
    }
    extent
}
fn prepare(cx: &ExecutionContext<'_>, input: &SheetsFit) -> Result<SheetsEdit, Stop> {
    checks::revision(cx.doc, input.expected_revision.as_deref())?;
    if input.bounds.is_some() && input.uids.is_some() {
        return Err(Stop::failed(error(
            "ambiguous_source",
            "bounds ve uids birlikte verilemez.".into(),
            Some("bounds".into()),
        )));
    }
    let bounds = match input.bounds {
        Some(b) => b,
        None => object_extent(cx.doc, input.uids.as_deref())
            .map_err(|e| Stop::failed(*e))?
            .ok_or_else(|| {
                Stop::failed(error(
                    "no_entities",
                    "Sığdırılacak model nesnesi yok.".into(),
                    Some("uids".into()),
                ))
            })?,
    };
    let (book, _) = match cx.doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error("book_unreadable", e.message, None)))?,
        None => (Default::default(), Vec::new()),
    };
    let (owner, index) = book.find_item(&input.item).ok_or_else(|| {
        Stop::failed(error(
            "unknown_item",
            format!("“{}” öğesi kitapta yok.", input.item),
            Some("item".into()),
        ))
    })?;
    let item = &book.items_of(&owner).ok_or_else(|| {
        Stop::failed(error(
            "unknown_item",
            "Öğe sahibi yok.".into(),
            Some("item".into()),
        ))
    })?[index];
    let op = kentos_sheet::fit::map(item, bounds, input.margin_pct, input.standard_scales)
        .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    Ok(SheetsEdit {
        ops: vec![serde_json::to_value(op).unwrap_or_default()],
        pictures: Vec::new(),
        replace_pictures: false,
        expected_revision: input.expected_revision.clone(),
    })
}
pub fn validate(cx: &ExecutionContext<'_>, input: &SheetsFit) -> CommandResult<()> {
    match prepare(cx, input) {
        Ok(edit) => sheets_edit::validate(cx, &edit),
        Err(e) => e.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &SheetsFit) -> CommandResult<SheetsEdited> {
    match prepare(cx, input) {
        Ok(edit) => sheets_edit::plan(cx, &edit),
        Err(e) => e.into(),
    }
}
pub fn execute(cx: &mut ExecutionContext<'_>, input: SheetsFit) -> CommandResult<SheetsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SHEETS_FIT,
        1,
        input,
        |cx, input| match prepare(cx, &input) {
            Ok(edit) => sheets_edit::execute(cx, edit),
            Err(e) => e.into(),
        },
    )
}
