//! `cad.layers.columns` v1 (docs/adr/9318): a data layer's table and its fields kept together. `add` adds nullable
//! columns through the table's driver (`Transaction::add_columns`, in a transaction of their own) and, once the table
//! read again has them, the layer's fields; `refresh` takes the layer's fields to the table's columns as it is now. The
//! fields change as one undo step named after the action; a column added stays in the table, as a row
//! `cad.layers.session` writes stays. The protocol's plan runs on a reading copy: it reads the table and adds nothing
//! (docs/adr/0532 §4). The shared cases are `fixtures/commands/v1/cad.layers.columns.json`
//! (`scripts/fixtures/layer_columns_command_cases.py`).

use std::sync::Arc;

use kentos_commands::{
    CommandResult, DataLayer, LayerColumnsAction, LayerField, LayerNode, LayerNodeType, LayersColumns,
    LayersColumnsDone, fields_problem, fold,
};
use kentos_data::{DataError, Request, Source};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub fn label(action: LayerColumnsAction) -> &'static str {
    match action {
        LayerColumnsAction::Add => "Kaynağa sütun ekle",
        LayerColumnsAction::Refresh => "Kaynak şemasını yenile",
    }
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn unreachable(e: impl std::fmt::Display) -> Stop {
    Stop::failed(error(codes::SOURCE_UNREACHABLE, format!("Tablo okunamadı ya da değişikliği reddetti: {e}"), None))
}

/// The new columns asked (`add`); none for `refresh`.
fn asked(input: &LayersColumns) -> &[LayerField] {
    match input.action {
        LayerColumnsAction::Add => input.fields.as_deref().unwrap_or_default(),
        LayerColumnsAction::Refresh => &[],
    }
}

/// The input's own checks, in the contract's order (before the revision).
fn needs(input: &LayersColumns) -> Result<(), Stop> {
    if input.layer.trim().is_empty() {
        return Err(fail(
            codes::NO_LAYER,
            "Sütunların ekleneceği ya da şemasının yenileneceği veri katmanının kimliğini (layer) verin.".into(),
            "layer",
        ));
    }
    let fields = asked(input);
    if input.action == LayerColumnsAction::Add && fields.is_empty() {
        return Err(fail(codes::NO_COLUMNS, "Tabloya eklenecek sütunları (fields) verin.".into(), "fields"));
    }
    for (i, f) in fields.iter().enumerate() {
        if let Some(why) = crate::layers_create::name_problem(&f.name) {
            return Err(fail(codes::INVALID_NAME, format!("Sütunun adı alınamaz: {why}."), &format!("fields.{i}.name")));
        }
        if fields[..i].iter().any(|o| fold(&o.name) == fold(&f.name)) {
            return Err(fail(
                codes::COLUMN_EXISTS,
                format!("“{}” iki kez verildi; her sütuna ayrı bir ad verin.", f.name),
                &format!("fields.{i}.name"),
            ));
        }
    }
    Ok(())
}

/// The data layer the input acts on and its table, checked against the document in the contract's order; with `add`,
/// against the layer's fields and their rules too.
fn checked(doc: &Document, input: &LayersColumns) -> Result<(LayerNode, DataLayer), Stop> {
    needs(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let id = input.layer.trim();
    let Some(node) = doc.layers().get(id) else {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            "layer",
        ));
    };
    if node.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!("“{}” bir katman grubu; sütunlar bir veri katmanının tablosunun olur.", node.name),
            "layer",
        ));
    }
    let Some(data) = node.data.clone() else {
        return Err(fail(
            codes::NOT_A_DATA_LAYER,
            format!("“{}” bir veri katmanı değil; nesneleri çizimdedir, sütunu yoktur. Alanlar penceresinde alan ekleyin.", node.name),
            "layer",
        ));
    };
    if input.action == LayerColumnsAction::Add && data.read_only {
        return Err(fail(
            codes::DATA_LAYER_READ_ONLY,
            format!("“{}” salt okunur tutuluyor; tablosuna sütun eklenmez.", node.name),
            "layer",
        ));
    }
    let fields = asked(input);
    for (i, f) in fields.iter().enumerate() {
        let taken = node.fields.iter().any(|o| fold(&o.name) == fold(&f.name))
            || fold(&f.name) == fold(&data.key)
            || data.geometry.as_ref().is_some_and(|g| fold(&g.column) == fold(&f.name));
        if taken {
            return Err(fail(
                codes::COLUMN_EXISTS,
                format!("“{}” katmanda ya da tablonun anahtarında, geometrisinde var; başka bir ad verin.", f.name),
                &format!("fields.{i}.name"),
            ));
        }
    }
    if !fields.is_empty() {
        let mut all = node.fields.clone();
        all.extend(fields.iter().cloned());
        if let Some(why) = fields_problem(&all) {
            return Err(fail(codes::INVALID_FIELDS, why, "fields"));
        }
    }
    Ok((node.clone(), data))
}

/// The table's source, read through its driver.
fn source_of(data: &DataLayer) -> Result<Arc<dyn Source>, Stop> {
    kentos_data::registry().source(&data.address).map_err(unreachable)
}

/// What `add` would do: the table's columns against the new ones, whether its source and the account may add one, and
/// its rows; nothing written.
fn add_planned(
    doc: &Document,
    input: &LayersColumns,
    node: &LayerNode,
    data: &DataLayer,
) -> Result<(LayersColumnsDone, Arc<dyn Source>), Stop> {
    let source = source_of(data)?;
    let schema = source.schema(&data.table).map_err(unreachable)?;
    let fields = asked(input);
    for (i, f) in fields.iter().enumerate() {
        if schema.fields.iter().any(|c| fold(&c.name) == fold(&f.name)) {
            return Err(fail(
                codes::COLUMN_EXISTS,
                format!(
                    "“{}” katmanının tablosunda “{}” sütunu var ama katmanın alanı değil: Kaynak şemasını yenile ile katmana alın.",
                    node.name, f.name
                ),
                &format!("fields.{i}.name"),
            ));
        }
    }
    let caps = source.capabilities(&data.table).map_err(unreachable)?;
    if !caps.add_columns {
        return Err(fail(
            codes::COLUMNS_NOT_PERMITTED,
            format!(
                "“{}” katmanının tablosuna sütun eklenemez: PostgreSQL'de tablonun sahibi olmak gerekir; salt okunur dosyaya ve sorguya sütun eklenmez. Tablo değişmedi.",
                node.name
            ),
            "layer",
        ));
    }
    let rows = source.count(&data.table, &Request::default(), true).ok();
    let mut after = node.fields.clone();
    after.extend(fields.iter().cloned());
    Ok((
        LayersColumnsDone {
            layer: node.id.clone(),
            table: data.table.clone(),
            added: fields.iter().map(|f| f.name.clone()).collect(),
            removed: Vec::new(),
            fields: after,
            rows,
            revision: doc.revision().to_string(),
        },
        source,
    ))
}

/// What `refresh` would do: the layer's fields kept where the table still has their column (in the layer's order, its
/// rules kept), the table's other columns after them as fields (in the table's order), the rest off the layer.
fn refresh_planned(doc: &Document, node: &LayerNode, data: &DataLayer) -> Result<LayersColumnsDone, Stop> {
    let source = source_of(data)?;
    let schema = source.schema(&data.table).map_err(unreachable)?;
    let (columns, _left) = crate::data_describe::fields_of(&schema);
    let has = |name: &str| columns.iter().any(|c| c.name == name);
    let mut after: Vec<LayerField> = node.fields.iter().filter(|f| has(&f.name)).cloned().collect();
    let removed: Vec<String> = node.fields.iter().filter(|f| !has(&f.name)).map(|f| f.name.clone()).collect();
    let mut added = Vec::new();
    for c in columns {
        if !node.fields.iter().any(|f| f.name == c.name) {
            added.push(c.name.clone());
            after.push(c);
        }
    }
    Ok(LayersColumnsDone {
        layer: node.id.clone(),
        table: data.table.clone(),
        added,
        removed,
        fields: after,
        rows: None,
        revision: doc.revision().to_string(),
    })
}

fn planned(doc: &Document, input: &LayersColumns) -> Result<LayersColumnsDone, Stop> {
    let (node, data) = checked(doc, input)?;
    match input.action {
        LayerColumnsAction::Add => add_planned(doc, input, &node, &data).map(|(done, _)| done),
        LayerColumnsAction::Refresh => refresh_planned(doc, &node, &data),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LayersColumns) -> CommandResult<()> {
    match checked(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; the table is read, nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersColumns) -> CommandResult<LayersColumnsDone> {
    match planned(cx.doc, input) {
        Ok(done) => CommandResult::Completed { output: done, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// Adds the columns (or reads the table again) and writes the layer's fields, one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersColumns) -> CommandResult<LayersColumnsDone> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_COLUMNS,
        kentos_commands::CAD_LAYERS_COLUMNS_VERSION,
        input,
        write_execute,
    )
}

/// The columns added in their own transaction; refused by the database (an account that does not own the table) as
/// `columns_not_permitted`, any other failure as `source_unreachable`; the table read again has them.
fn add_columns(source: &dyn Source, node: &LayerNode, data: &DataLayer, fields: &[LayerField]) -> Result<(), Stop> {
    let columns: Vec<kentos_data::Field> = fields
        .iter()
        .map(|f| kentos_data::Field::new(&f.name, crate::layer_export::field_kind(f.kind, f.length)))
        .collect();
    let mut tx = source.begin().map_err(unreachable)?;
    if let Err(e) = tx.add_columns(&data.table, &columns) {
        let _ = tx.rollback();
        return Err(match e {
            DataError::Denied(why) | DataError::Unsupported(why) => fail(
                codes::COLUMNS_NOT_PERMITTED,
                format!("“{}” katmanının tablosuna sütun eklenemedi: {why}. Tablo değişmedi.", node.name),
                "layer",
            ),
            other => unreachable(other),
        });
    }
    tx.commit().map_err(unreachable)?;
    let schema = source.schema(&data.table).map_err(unreachable)?;
    match fields.iter().find(|f| schema.field(&f.name).is_none()) {
        Some(f) => Err(unreachable(format!("tablo yeniden okunduğunda “{}” sütunu yok", f.name))),
        None => Ok(()),
    }
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: LayersColumns) -> CommandResult<LayersColumnsDone> {
    let (node, data) = match checked(cx.doc, &input) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let done = match input.action {
        LayerColumnsAction::Add => {
            let (done, source) = match add_planned(cx.doc, &input, &node, &data) {
                Ok(p) => p,
                Err(stop) => return stop.into(),
            };
            // The protocol's plan runs this on a reading copy: no column added then.
            if !cx.dry()
                && let Err(stop) = add_columns(source.as_ref(), &node, &data, asked(&input))
            {
                return stop.into();
            }
            done
        }
        LayerColumnsAction::Refresh => match refresh_planned(cx.doc, &node, &data) {
            Ok(done) => done,
            Err(stop) => return stop.into(),
        },
    };
    if done.added.is_empty() && done.removed.is_empty() {
        return CommandResult::Completed { output: done, warnings: Vec::new() };
    }
    let doc = &mut *cx.doc;
    let id = node.id.clone();
    let fields = done.fields.clone();
    match doc.transact(label(input.action), |doc| doc.set_layer_fields(&id, fields, &[]).map(|_| ())) {
        Ok(()) => CommandResult::Completed {
            output: LayersColumnsDone { revision: doc.revision().to_string(), ..done },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}
