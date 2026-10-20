//! A layer's look as SLD (docs/adr/0516, plan/gis.md CBS-17): what `file.export.sld` writes and `file.import.sld`
//! reads. The writing and reading are the formats core's (`kentos_formats::sld`); here the drawing gives the layer's
//! renderer (or its simple look as a single symbol, as the drawing draws it), the project library's symbols for the
//! renderer's references, and the geometry its objects have; and an SLD's renderer goes to the layer through
//! `cad.layers.renderer`'s write, one undo step “Katman stili”.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, ImportSld, LayerNodeType, LayerStyle, LayersRenderer,
    LineType, PointSymbol, SldImported,
};
use kentos_domain::Document;
use kentos_formats::sld::{self, Kinds, Written};
use serde_json::{Value, json};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The layer types' dashes, paper mm (the native style's `line_type_dash`, the web's `LINE_TYPE_DASH`; the native
/// style's tests hold the two to one table).
pub fn line_type_dash(line_type: LineType) -> Option<&'static [f64]> {
    match line_type {
        LineType::Continuous => None,
        LineType::Dashed => Some(&[3.0, 1.5]),
        LineType::Dashdot => Some(&[5.0, 1.2, 0.6, 1.2]),
        LineType::Dotted => Some(&[0.6, 1.2]),
    }
}

/// A layer's simple look as a single symbol renderer: its colour, line type and weight, fill, point symbol (as
/// `kentos_native_style::simple::symbols_of_layer_style` draws it).
pub fn simple_renderer(style: &LayerStyle) -> Value {
    let line = |id: &str| {
        json!({
            "id": id, "type": "simpleLine", "color": style.color, "width": style.line_weight,
            "dash": line_type_dash(style.line_type), "unit": "mm", "cap": "butt", "join": "miter",
        })
    };
    let mut fill = Vec::new();
    if let Some(f) = &style.fill {
        fill.push(json!({ "id": "f", "type": "simpleFill", "color": f }));
    }
    fill.push(line("o"));
    let (symbol, size) = style
        .point
        .as_ref()
        .map_or((PointSymbol::Ring, 7.0), |p| (p.symbol, p.size));
    let shape = match symbol {
        PointSymbol::Ring => "ring",
        PointSymbol::Cross => "cross",
        PointSymbol::Triangle => "triangle",
    };
    json!({ "type": "single", "symbols": {
        "line": { "type": "line", "layers": [line("l")] },
        "fill": { "type": "fill", "layers": fill },
        "marker": { "type": "marker", "layers": [{
            "id": "p", "type": "shape", "shape": shape, "size": size, "unit": "px",
            "stroke": style.color, "strokeWidth": 1.3, "fill": null,
        }] },
    } })
}

/// The geometry the layer's objects have; all three when it has none.
pub fn kinds_of(doc: &Document, layer: &str) -> Kinds {
    let mut k = Kinds {
        point: false,
        line: false,
        area: false,
    };
    for e in doc.entities().filter(|e| e.base().layer_id == layer) {
        match e {
            Entity::Point(_) => k.point = true,
            Entity::Polygon(_) | Entity::Circle(_) | Entity::Ellipse(_) | Entity::Hatch(_) => {
                k.area = true;
            }
            Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Arc(_)
            | Entity::Spline(_)
            | Entity::Nurbs(_) => {
                k.line = true;
            }
            _ => {}
        }
    }
    if !(k.point || k.line || k.area) {
        return Kinds::ALL;
    }
    k
}

/// A symbol of the project's library by id.
pub(crate) fn library_symbol(doc: &Document, id: &str) -> Option<Value> {
    doc.styles()
        .items
        .iter()
        .find(|it| it.get("id").and_then(Value::as_str) == Some(id))
        .and_then(|it| it.get("symbol").cloned())
}

/// The layer's look as an SLD, or why it has none.
pub fn export(
    doc: &Document,
    layer: &str,
) -> Result<(String, Written), Box<kentos_commands::CommandError>> {
    let Some(node) = doc.layers().get(layer) else {
        return Err(Box::new(error(
            codes::LAYER_NOT_FOUND,
            format!(
                "“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
            ),
            Some("layer".into()),
        )));
    };
    if node.kind != LayerNodeType::Layer {
        return Err(Box::new(error(
            codes::NOT_A_LAYER,
            format!(
                "“{}” bir katman grubu; görünüş katmanındır. Grubun bir katmanını verin.",
                node.name
            ),
            Some("layer".into()),
        )));
    }
    let renderer = node
        .style
        .renderer
        .clone()
        .unwrap_or_else(|| simple_renderer(&node.style));
    let written = sld::write(&node.name, &renderer, kinds_of(doc, layer), &|id| {
        library_symbol(doc, id)
    });
    if written.rules == 0 {
        return Err(Box::new(error(
            codes::NOTHING_TO_EXPORT,
            format!(
                "“{}” katmanının görünüşünde SLD'nin söyleyebileceği bir kural yok: {}",
                node.name,
                written.lost.join("; ")
            ),
            Some("layer".into()),
        )));
    }
    Ok((node.name.clone(), written))
}

/// The SLD's text: the input's, or its file's.
fn text_of(input: &ImportSld) -> Result<String, Stop> {
    match (&input.path, &input.text) {
        (Some(_), Some(_)) | (None, None) => Err(refuse(
            codes::INVALID_INPUT,
            "SLD'nin ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.".into(),
            "path",
        )),
        (None, Some(t)) => Ok(t.clone()),
        (Some(p), None) => std::fs::read(p)
            .map_err(|e| {
                refuse(
                    codes::FILE_UNREADABLE,
                    format!("“{p}” okunamadı: {e}. Dosyanın yerinde olduğunu denetleyin."),
                    "path",
                )
            })
            .map(|b| String::from_utf8_lossy(&b).into_owned()),
    }
}

/// The renderer the SLD gives, what it lost, and the rules taken.
fn read(input: &ImportSld) -> Result<sld::Read, Stop> {
    let text = text_of(input)?;
    let read = sld::read(&text).map_err(|why| refuse(codes::INVALID_SLD, why, "path"))?;
    if let Some(why) = crate::layers_renderer::renderer_problem(&read.renderer) {
        return Err(refuse(
            codes::INVALID_SLD,
            format!("SLD'nin görünüşü KentOS'un görünüşü olamadı: {why}"),
            "path",
        ));
    }
    Ok(read)
}

fn warnings_of(lost: &[String]) -> Vec<CommandWarning> {
    if lost.is_empty() {
        return Vec::new();
    }
    vec![CommandWarning {
        code: codes::IMPORT_LOSSES.into(),
        message: format!("SLD'nin bazı parçaları alınamadı: {}.", lost.join("; ")),
        path: Some("path".into()),
    }]
}

/// Whether a `cad.layers.renderer` answer changed the layer, or how it stopped.
fn changed_of<T>(r: CommandResult<T>, changed: impl Fn(&T) -> bool) -> Result<bool, Stop> {
    match r {
        CommandResult::Completed { output, .. } => Ok(changed(&output)),
        CommandResult::Failed { error } => Err(Stop::failed(error)),
        CommandResult::Conflict { error } => Err(Stop::conflict(error)),
        _ => Err(Stop::Cancelled),
    }
}

fn run(
    cx: &mut ExecutionContext<'_>,
    input: &ImportSld,
    write: bool,
) -> Result<(SldImported, Vec<CommandWarning>), Stop> {
    let read = read(input)?;
    checks::revision(cx.doc, input.expected_revision.as_deref())?;
    let renderer_input = LayersRenderer {
        layer: input.layer.clone(),
        renderer: Some(read.renderer.clone()),
        expected_revision: None,
    };
    let changed = if write {
        changed_of(crate::layers_renderer::execute(cx, renderer_input), |o| {
            o.changed
        })?
    } else {
        changed_of(crate::layers_renderer::plan(cx, &renderer_input), |o| {
            o.changed
        })?
    };
    let kind = read.renderer["type"].as_str().unwrap_or("").to_owned();
    Ok((
        SldImported {
            layer: input.layer.clone(),
            kind,
            rules: read.rules as u32,
            renderer: read.renderer,
            changed,
            lost: read.lost.clone(),
            revision: cx.doc.revision().to_string(),
        },
        warnings_of(&read.lost),
    ))
}

/// Checks `input` against the drawing, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportSld) -> CommandResult<()> {
    let mut copy = cx.doc.clone();
    let mut cx = ExecutionContext::new(&mut copy);
    match run(&mut cx, input, false) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would make the layer's look; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportSld) -> CommandResult<SldImported> {
    let mut copy = cx.doc.clone();
    let mut cx = ExecutionContext::new(&mut copy);
    match run(&mut cx, input, false) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// Makes the SLD's look the layer's, one undo step “Katman stili” when it differs.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportSld) -> CommandResult<SldImported> {
    // One step of the drawing and its own row in the journal (docs/adr/0253 §3, 0548 §4).
    crate::call::recorded(
        cx,
        kentos_commands::FILE_IMPORT_SLD,
        kentos_commands::FILE_IMPORT_SLD_VERSION,
        input,
        written_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_execute(cx: &mut ExecutionContext<'_>, input: ImportSld) -> CommandResult<SldImported> {
    match run(cx, &input, true) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}
