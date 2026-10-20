//! A layer's look as a QGIS layer style file (docs/adr/0555, plan/gis.md CBS-17): what `file.export.qml` writes and
//! `file.import.qml` reads. The writing and reading are the formats core's (`kentos_formats::qml`); here, as for SLD
//! (`crate::sld`), the drawing gives the layer's renderer (or its simple look as a single symbol), the project
//! library's symbols for the renderer's references and the geometry its objects have; and a QML's renderer goes to the
//! layer through `cad.layers.renderer`'s write, one undo step “Katman stili”.

use kentos_commands::{
    CommandResult, CommandWarning, ImportQml, LayerNodeType, LayersRenderer, QmlImported,
};
use kentos_domain::Document;
use kentos_formats::qml::{self, Written};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::sld::{kinds_of, library_symbol, simple_renderer};

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The layer's look as a QML, or why it has none.
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
    let written = qml::write(&renderer, kinds_of(doc, layer), &|id| {
        library_symbol(doc, id)
    });
    if written.rules == 0 {
        return Err(Box::new(error(
            codes::NOTHING_TO_EXPORT,
            format!(
                "“{}” katmanının görünüşünde QGIS'in buradaki simge katmanlarıyla çizilebilecek bir sınıf ya da kural yok: {}",
                node.name,
                written.lost.join("; ")
            ),
            Some("layer".into()),
        )));
    }
    Ok((node.name.clone(), written))
}

/// The QML's text: the input's, or its file's.
fn text_of(input: &ImportQml) -> Result<String, Stop> {
    match (&input.path, &input.text) {
        (Some(_), Some(_)) | (None, None) => Err(refuse(
            codes::INVALID_INPUT,
            "QML'in ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.".into(),
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

/// The renderer the QML gives, what it lost, and the classes and rules taken.
fn read(input: &ImportQml) -> Result<qml::Read, Stop> {
    let text = text_of(input)?;
    let read = qml::read(&text).map_err(|why| refuse(codes::INVALID_QML, why, "path"))?;
    if let Some(why) = crate::layers_renderer::renderer_problem(&read.renderer) {
        return Err(refuse(
            codes::INVALID_QML,
            format!("QML'in görünüşü KentOS'un görünüşü olamadı: {why}"),
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
        message: format!("QML'in bazı parçaları alınamadı: {}.", lost.join("; ")),
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
    input: &ImportQml,
    write: bool,
) -> Result<(QmlImported, Vec<CommandWarning>), Stop> {
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
        QmlImported {
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
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportQml) -> CommandResult<()> {
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
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportQml) -> CommandResult<QmlImported> {
    let mut copy = cx.doc.clone();
    let mut cx = ExecutionContext::new(&mut copy);
    match run(&mut cx, input, false) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// Makes the QML's look the layer's, one undo step “Katman stili” when it differs.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportQml) -> CommandResult<QmlImported> {
    // One step of the drawing and its own row in the journal (docs/adr/0253 §3, 0548 §4).
    crate::call::recorded(
        cx,
        kentos_commands::FILE_IMPORT_QML,
        kentos_commands::FILE_IMPORT_QML_VERSION,
        input,
        written_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_execute(cx: &mut ExecutionContext<'_>, input: ImportQml) -> CommandResult<QmlImported> {
    match run(cx, &input, true) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}
