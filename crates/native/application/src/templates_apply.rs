//! `cad.templates.apply` v1 (docs/adr/0346 §1; plan/cad.md `CAD-ARCH-41`): a work template's parts the drawing lacks — its
//! unit (only on a drawing without objects), scale, levels and space method, its drawing standard's layers and styles,
//! its block library's blocks, KentOS's wall styles and its sheets — applied, keeping what the drawing has. The layers,
//! styles and blocks are one undo step “Mimari şablonu”; the settings and the sheet book are no undo step (docs/adr/0251).
//! The shared cases are `fixtures/commands/v1/cad.templates.apply.json`.
//!
//! Checks, in order: the template's name (`unknown_template`), the unit (`invalid_template_unit`: mm or cm), the expected
//! revision.

use kentos_contracts::{
    BlockEditOperation, BlocksEdit, CommandResult, CommandWarning, DrawingUnit, StandardsApply,
    TemplateSheet, TemplatesApplied, TemplatesApply, WorkTemplate, builtin_template,
    builtin_wall_styles,
};
use kentos_domain::{Document, Uuid};
use serde_json::{Value, json};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const STEP: &str = "Mimari şablonu";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn check(doc: &Document, input: &TemplatesApply) -> Result<WorkTemplate, Stop> {
    let Some(t) = builtin_template(&input.name) else {
        return Err(fail(
            codes::UNKNOWN_TEMPLATE,
            format!(
                "“{}” adlı çalışma şablonu yok; KentOS'un şablonu: mimari.",
                input.name
            ),
            "name",
        ));
    };
    if let Some(u) = input.unit
        && !matches!(u, DrawingUnit::Mm | DrawingUnit::Cm)
    {
        return Err(fail(
            codes::INVALID_TEMPLATE_UNIT,
            "Şablonun birimi mm ya da cm olur.".into(),
            "unit",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(t)
}

/// A template's sheet as the sheet core keeps it: A3 landscape, one map frame filling its margins at its scale, its
/// output a 300 dpi picture named after the sheet (printed to PDF by `file.export.pdf`).
pub fn sheet_json(s: &TemplateSheet) -> Value {
    json!({
        "export": {"dpi": 300, "fileName": "[% @pafta_adi %]", "format": "png"},
        "guides": [],
        "id": s.id,
        "items": [{
            "bindings": [], "constraints": {"h": "left", "relativeTo": "margins", "v": "top"},
            "frame": {"height": 277000, "left": 20000, "top": 10000, "width": 390000},
            "hidden": false, "id": format!("{}-harita", s.id),
            "kind": {"clipToAtlasFeature": false, "grids": [], "labelBand": 0, "layers": {"type": "all"},
                     "overviewFrame": {"cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500},
                     "type": "map", "view": {"center": {"x": 0.0, "y": 0.0}, "rotation": 0, "scale": s.scale, "type": "fixed"}},
            "locked": false, "name": s.frame, "opacity": 100, "padding": 0, "printable": true, "rotation": 0
        }],
        "name": s.name,
        "page": {"margins": {"bottom": 10000, "left": 20000, "right": 10000, "top": 10000}, "orientation": "landscape",
                 "paper": "a3", "size": {"height": 297000, "width": 420000}},
        "snapGrid": {"enabled": false, "spacing": 5000, "visible": false},
        "variables": [],
        "variants": []
    })
}

fn stop_of<T>(result: CommandResult<T>) -> Result<T, Stop> {
    match result {
        CommandResult::Completed { output, .. } => Ok(output),
        CommandResult::Failed { error } | CommandResult::NeedsInput { error } => Err(Stop::failed(error)),
        CommandResult::Conflict { error } => Err(Stop::conflict(error)),
        CommandResult::Queued { .. } | CommandResult::Cancelled => Err(Stop::failed(error(
            codes::INVALID_OPERATION,
            "Şablonun bir parçası yazılamadı.".into(),
            None,
        ))),
    }
}

/// The template's parts written to `doc`; `fresh` gives new layers their ids.
fn write(
    doc: &mut Document,
    input: &TemplatesApply,
    t: &WorkTemplate,
    fresh: &dyn Fn(usize) -> String,
) -> Result<(TemplatesApplied, Vec<CommandWarning>), Stop> {
    let mut out = TemplatesApplied::default();
    let mut warnings = Vec::new();
    // 1. The standard's layers and styles, and the library's blocks: one undo step.
    let group = doc.begin_group(STEP);
    let standard = crate::standards::standard_of(Some(&t.standard), None)?;
    let applied = crate::standards::write(
        doc,
        &StandardsApply {
            name: Some(t.standard.clone()),
            standard: None,
            layers: true,
            restyle: false,
            rename: false,
            styles: true,
            parent: None,
            expected_revision: None,
        },
        &standard,
        fresh,
    );
    let applied = match stop_of(applied) {
        Ok(a) => a,
        Err(stop) => {
            doc.cancel_group(group);
            return Err(stop);
        }
    };
    out.layers = applied
        .created
        .iter()
        .map(|id| doc.layers().get(id).map_or_else(|| id.clone(), |n| n.name.clone()))
        .collect();
    out.styles = applied
        .text_styles
        .iter()
        .chain(&applied.dimension_styles)
        .cloned()
        .collect();
    let import: BlocksEdit = serde_json::from_value(json!({
        "operation": "import",
        "name": t.blocks,
    }))
    .map_err(|e| Stop::failed(error(codes::INVALID_OPERATION, e.to_string(), None)))?;
    debug_assert_eq!(import.operation, BlockEditOperation::Import);
    let blocks = crate::blocks_library::write(&mut ExecutionContext::new(doc), &import);
    match stop_of(blocks) {
        Ok(b) => {
            out.blocks = b
                .added
                .iter()
                .filter_map(|id| doc.block(*id).map(|d| d.name.clone()))
                .collect();
        }
        Err(stop) => {
            doc.cancel_group(group);
            return Err(stop);
        }
    }
    doc.end_group(group);
    // 2. The settings: no undo step.
    let mut s = doc.settings().clone();
    let empty = doc.entities().next().is_none();
    let unit = input.unit.unwrap_or(t.unit);
    let now = s.drawing_unit.unwrap_or_default();
    if now != unit {
        if empty {
            s.drawing_unit = Some(unit);
        } else {
            warnings.push(CommandWarning {
                code: codes::TEMPLATE_UNIT_KEPT.into(),
                message: format!(
                    "Çizimde nesne var; birimi ({}) değişmedi. Birimi Proje ayarları ya da Birimi değiştir ile değiştirin.",
                    now.mark()
                ),
                path: Some("unit".into()),
            });
        }
    }
    out.unit = s.drawing_unit.unwrap_or_default();
    s.plot_scale = t.plot_scale;
    let mut arch = s.arch.clone().unwrap_or_default();
    for l in &t.levels {
        if !arch.levels.iter().any(|x| x.id == l.id || x.name == l.name) {
            arch.levels.push(l.clone());
            out.levels.push(l.name.clone());
        }
    }
    if arch.space_method.is_none() {
        arch.space_method = Some(t.space_method);
    }
    s.arch = (!arch.is_empty()).then_some(arch);
    for w in builtin_wall_styles() {
        if !s.wall_styles.iter().any(|x| x.id == w.id || x.name == w.name) {
            out.wall_styles.push(w.name.clone());
            s.wall_styles.push(w);
        }
    }
    doc.set_settings(s);
    // 3. The sheets the book lacks: no undo step.
    let have: Vec<String> = crate::sheets_edit::sheets_in(doc)
        .unwrap_or_default()
        .into_iter()
        .map(|i| i.id)
        .collect();
    let ops: Vec<Value> = t
        .sheets
        .iter()
        .filter(|sh| !have.contains(&sh.id))
        .map(|sh| {
            out.sheets.push(sh.name.clone());
            json!({ "op": "addSheet", "sheet": sheet_json(sh) })
        })
        .collect();
    if !ops.is_empty() {
        crate::sheets_edit::apply_ops(doc, ops)?;
    }
    out.revision = doc.revision().to_string();
    Ok((out, warnings))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TemplatesApply) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The preview: the same write on a reading copy; the drawing stays.
pub fn plan(cx: &ExecutionContext<'_>, input: &TemplatesApply) -> CommandResult<TemplatesApplied> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document.
pub fn plan_in(doc: &Document, input: &TemplatesApply) -> CommandResult<TemplatesApplied> {
    let t = match check(doc, input) {
        Ok(t) => t,
        Err(stop) => return stop.into(),
    };
    let mut copy = doc.reading_copy();
    match write(&mut copy, input, &t, &|_| "-".to_owned()) {
        Ok((mut output, warnings)) => {
            output.revision = doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: TemplatesApply) -> CommandResult<TemplatesApplied> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_TEMPLATES_APPLY,
        kentos_contracts::CAD_TEMPLATES_APPLY_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: TemplatesApply) -> CommandResult<TemplatesApplied> {
    let t = match check(cx.doc, &input) {
        Ok(t) => t,
        Err(stop) => return stop.into(),
    };
    match write(cx.doc, &input, &t, &|_| Uuid::now_v7().to_string()) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}
