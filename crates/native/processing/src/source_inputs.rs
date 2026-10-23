//! İşlemler's features inputs read from their source (docs/adr/0592 §1, plan/gis.md CBS-23): `{ "scope": "source",
//! "layer": … }` reads a bound layer's whole table (not the part the drawing holds), `{ "scope": "source", "path": …,
//! "table": … }` a GeoPackage's or a PostGIS database's table, each optionally narrowed by `filter` and `bbox`. The
//! records come a part at a time as `cad.sources.query` reads them (`sources_query::stage`) and go into the run's copy:
//! a bound layer's into the layer itself in place of the part it held, a table's into a layer of its own with the
//! table's columns as fields. The input then names that layer, and the tool runs as on any layer; what it reads is
//! said in the answer's `inputs` (the table, the records read, the file's SHA-256).

use kentos_commands::{CommandError, ProcessingRun, SourceKind, codes};
use kentos_domain::{Document, NewLayer, Slot};
use kentos_native_application::sources_query::{StageFrom, stage};
use serde_json::{Value, json};

/// An input read from its source, as the answer says it.
pub(crate) struct Staged {
    /// The layer of the copy the records went into.
    pub layer: String,
    /// The table read (`file › table`, `PostGIS: host › table`).
    pub table: String,
    /// The bound layer's source, when it was one.
    pub source: Option<String>,
    /// The file's SHA-256 when the table is a GeoPackage's (what was read); none for a database.
    pub revision: Option<String>,
    pub rows: u64,
    /// The file or the database's address read.
    pub path: Option<String>,
}

fn refused(code: &str, message: String, path: String) -> Box<CommandError> {
    Box::new(CommandError {
        code: code.into(),
        message,
        path: Some(path),
        ..CommandError::default()
    })
}

/// The parameters given as source inputs, by name.
pub(crate) fn source_params(input: &ProcessingRun) -> Vec<String> {
    input
        .parameters
        .iter()
        .filter(|(_, v)| v.get("scope").and_then(Value::as_str) == Some("source"))
        .map(|(k, _)| k.clone())
        .collect()
}

/// The checks of a source input that read nothing: it needs a sink (the run is made on a copy and its result goes to
/// a file or a database), and names a bound layer of the drawing, or a path and a table.
pub(crate) fn check_sources(
    doc: &Document,
    input: &ProcessingRun,
) -> Result<(), Box<CommandError>> {
    for name in source_params(input) {
        let at = format!("parameters.{name}");
        if input.sink.is_none() {
            return Err(refused(
                codes::INVALID_PARAMETERS,
                format!(
                    "“{name}” kaynağından okunuyor: çalıştırma bir kopyada yapılır ve sonucu bir GeoPackage'a ya da \
                     PostGIS'e yazılır; sink verin."
                ),
                at,
            ));
        }
        let v = &input.parameters[&name];
        match (
            v.get("layer").and_then(Value::as_str),
            v.get("path").and_then(Value::as_str),
            v.get("table").and_then(Value::as_str),
        ) {
            (Some(layer), None, None) => {
                let Some(node) = doc.layers().get(layer) else {
                    return Err(refused(
                        codes::LAYER_NOT_FOUND,
                        format!("“{layer}” kimlikli katman çizimde yok."),
                        format!("{at}.layer"),
                    ));
                };
                if !doc.settings().sources.iter().any(|s| s.layer == layer) {
                    return Err(refused(
                        codes::INVALID_INPUT,
                        format!(
                            "“{}” katmanı bir kaynağa bağlı değil; kaynağından okunamaz (katmanı scope: layer ile verin).",
                            node.name
                        ),
                        format!("{at}.layer"),
                    ));
                }
            }
            (None, Some(_), Some(_)) => {}
            _ => {
                return Err(refused(
                    codes::INVALID_PARAMETERS,
                    format!(
                        "“{name}”: kaynak bir bağlı katmanla (layer) ya da bir dosya veya adres ve tabloyla (path, \
                         table) verilir, ikisi birden değil."
                    ),
                    at,
                ));
            }
        }
        if let Some(b) = v.get("bbox")
            && !b.as_array().is_some_and(|l| {
                l.len() == 4 && l.iter().all(|x| x.as_f64().is_some_and(f64::is_finite))
            })
        {
            return Err(refused(
                codes::INVALID_PARAMETERS,
                format!("“{name}”: kutu (bbox) dört sonlu sayıdır: batı, güney, doğu, kuzey."),
                format!("{at}.bbox"),
            ));
        }
    }
    Ok(())
}

/// The source inputs read into `copy`, and the input with each of them naming its layer of the copy. A table read into
/// a layer of its own makes `kaynak-<parameter><suffix>` (a batch row's suffix keeps rows' layers apart, docs/adr/0593
/// §2). `Err(None)`: stopped.
pub(crate) fn stage_inputs(
    copy: &mut Document,
    input: &ProcessingRun,
    suffix: &str,
    stop: &dyn Fn() -> bool,
) -> Result<(ProcessingRun, Vec<Staged>), Option<Box<CommandError>>> {
    check_sources(copy, input).map_err(Some)?;
    let mut out = input.clone();
    let mut staged = Vec::new();
    for name in source_params(input) {
        let at = format!("parameters.{name}");
        let v = input.parameters[&name].clone();
        let filter = v.get("filter").and_then(Value::as_str).map(str::to_owned);
        let bbox: Option<[f64; 4]> = v.get("bbox").and_then(Value::as_array).map(|l| {
            let n = |i: usize| l[i].as_f64().unwrap_or(0.0);
            [n(0), n(1), n(2), n(3)]
        });
        let link = v
            .get("layer")
            .and_then(Value::as_str)
            .and_then(|l| copy.settings().sources.iter().find(|s| s.layer == l))
            .cloned();
        let (path, table) = match &link {
            Some(l) => (l.path.clone(), l.table.clone()),
            None => (
                v.get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                v.get("table")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            ),
        };
        let from = match &link {
            Some(l) => StageFrom::Layer(l),
            None => StageFrom::Table {
                address: &path,
                table: &table,
            },
        };
        // The input's time (docs/adr/0595 §3): a bound layer's own time fields unless it names them.
        let time = match v.get("time") {
            None => None,
            Some(t) => {
                let mut q: kentos_commands::QueryTime =
                    serde_json::from_value(t.clone()).map_err(|e| {
                        Some(refused(
                            codes::INVALID_PARAMETERS,
                            format!("“{name}”: zaman (time) okunamadı: {e}."),
                            format!("{at}.time"),
                        ))
                    })?;
                if q.fields.is_none() {
                    q.fields = link
                        .as_ref()
                        .and_then(|l| copy.layers().get(&l.layer))
                        .and_then(|n| n.time.clone());
                }
                Some(q)
            }
        };
        let settings = copy.settings().clone();
        let (staging, read) = stage(
            &settings,
            &from,
            filter.as_deref(),
            bbox,
            time.as_ref(),
            &at,
            stop,
        )?;
        let database = matches!(link.as_ref().map(|l| l.kind), Some(SourceKind::Postgis))
            || path.starts_with("postgres://")
            || path.starts_with("postgresql://");
        let shown = match &read.source.path {
            Some(p) if database => {
                let host = p.split_once("://").map_or(p.as_str(), |(_, rest)| rest);
                format!("PostGIS: {host} › {table}")
            }
            Some(p) => format!(
                "{} › {table}",
                std::path::Path::new(p)
                    .file_name()
                    .map_or_else(|| p.clone(), |n| n.to_string_lossy().into_owned())
            ),
            None => table.clone(),
        };
        // The layer the records go into: a bound layer itself (its part out first), else a layer of their own.
        let layer = match &link {
            Some(l) => {
                let held: Vec<Slot> = copy
                    .by_layer_with_uids(&l.layer)
                    .filter_map(|(u, _)| copy.slot_of(u))
                    .collect();
                copy.remove(&held);
                l.layer.clone()
            }
            None => {
                // Named apart from the table (“bina (kaynak)”): a tool's output layer named as the table is a new layer, not
                // this one reused by its name (docs/adr/0593 §2).
                let short = table.rsplit('.').next().unwrap_or(&table);
                let mut new = NewLayer::layer(format!("{short} (kaynak)"));
                new.id = Some(format!("kaynak-{name}{suffix}"));
                new.fields = staging.fields.clone().unwrap_or_default();
                copy.add_layer(new, None, false)
                    .map_err(|r| Some(refused(codes::TOOL_FAILED, r.0, at.clone())))?
            }
        };
        // An object of the drawing with a record's lasting id (the same table imported) gives way in the copy.
        let taken: Vec<Slot> = staging
            .objects
            .iter()
            .filter_map(|(u, _)| copy.slot_of(*u))
            .collect();
        copy.remove(&taken);
        let rows = staging.objects.len() as u64;
        let objects = staging
            .objects
            .into_iter()
            .map(|(u, mut e)| {
                e.base_mut().layer_id = layer.clone();
                (u, e)
            })
            .collect();
        copy.add_many_with_uids(objects, "Kaynaktan oku")
            .map_err(|r| Some(refused(codes::TOOL_FAILED, r.0, at.clone())))?;
        let revision = (!database)
            .then(|| read.source.path.as_deref().and_then(file_sha256))
            .flatten();
        let mut named = json!({ "scope": "layer", "layerId": layer });
        if let Some(kinds) = v.get("kinds") {
            named["kinds"] = kinds.clone();
        }
        out.parameters.insert(name, named);
        staged.push(Staged {
            layer,
            table: shown,
            source: link.map(|l| l.id),
            revision,
            rows,
            path: read.source.path.clone(),
        });
    }
    Ok((out, staged))
}

/// The inputs `staged` read from their source said in a run's answer and its record: the table, the records read,
/// the file's SHA-256 (docs/adr/0592 §1, 0593 §3).
pub(crate) fn said(ran: &mut kentos_commands::ProcessingRan, staged: &[Staged]) {
    for s in staged {
        for i in ran
            .inputs
            .iter_mut()
            .chain(ran.record.inputs.iter_mut())
            .filter(|i| i.layer == s.layer)
        {
            i.source = s.source.clone();
            i.revision = s.revision.clone();
            i.table = Some(s.table.clone());
            i.rows = Some(s.rows);
            i.path = s.path.clone();
        }
    }
}

/// A record's values with each source input as it was given, not the layer of the copy it was read into: giving them
/// to `cad.processing.run` reads the source again (docs/adr/0593 §3).
pub(crate) fn given(
    parameters: &mut std::collections::BTreeMap<String, Value>,
    input: &ProcessingRun,
) {
    for name in source_params(input) {
        parameters.insert(name.clone(), input.parameters[&name].clone());
    }
}

/// A file's SHA-256, read as it streams.
fn file_sha256(path: &str) -> Option<String> {
    let f = std::fs::File::open(path).ok()?;
    kentos_commands::encoding::sha256_reader_hex(std::io::BufReader::new(f))
        .ok()
        .map(|(_, sha)| sha)
}
