//! `cad.forms.sources` v1 (Formun kaynakları, docs/adr/0763): how the sources of the layers a form lists
//! from stand, each state with its own code and way out, read-only. The layers are found on the drawing's
//! thread ([`targets`]); each source is asked apart ([`probe`]), so a host asks a database off its interface's
//! thread. The shared cases are `fixtures/commands/v1/cad.forms.sources.json`.

use std::path::Path;
use std::time::Duration;

use kentos_commands::{
    CommandResult, FormBody, FormItem, FormSource, FormSourceRetry, FormSourceState, FormsSourced,
    FormsSources, SourceKind, SourceLink,
};
use kentos_domain::Document;
use kentos_forms::codes as form_codes;
use kentos_geopackage::GeoPackage;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::forms::{self, fail};
use crate::provider::{self, Unreached};

/// How long a database is waited for when the input says nothing.
pub const WAIT: Duration = Duration::from_millis(5000);
const LEAST_MS: u32 = 100;
const MOST_MS: u32 = 30_000;

/// One layer a form lists from: its id, its name, the items that list from it and its source's link.
#[derive(Clone, Debug)]
pub struct Target {
    pub layer: String,
    pub name: String,
    pub items: Vec<String>,
    pub link: Option<SourceLink>,
}

/// The layers `input`'s form lists from, in the order its items first name them (a layer the project has
/// not is left to the form's preflight), and how long a database may be waited for; else why not, in words.
pub fn targets(doc: &Document, input: &FormsSources) -> Result<(Vec<Target>, Duration), String> {
    targeted(doc, input).map_err(|stop| stop.into_error().message)
}

fn targeted(doc: &Document, input: &FormsSources) -> Result<(Vec<Target>, Duration), Stop> {
    let wait = match input.wait_ms {
        None => WAIT,
        Some(ms) if (LEAST_MS..=MOST_MS).contains(&ms) => Duration::from_millis(u64::from(ms)),
        Some(ms) => {
            return Err(fail(
                codes::INVALID_INPUT,
                format!(
                    "waitMs {ms}: bekleme {LEAST_MS} ile {MOST_MS} milisaniye arasında verilir."
                ),
                "waitMs",
            ));
        }
    };
    let form = forms::form(doc.settings(), &input.form, "form")?;
    let (body, _) = forms::body(form, input.draft, "form")?;
    let mut out: Vec<Target> = Vec::new();
    for (layer, item) in listed(body) {
        let Some(node) = doc.layers().get(&layer) else {
            continue;
        };
        match out.iter_mut().find(|t| t.layer == layer) {
            Some(t) => {
                if !t.items.contains(&item) {
                    t.items.push(item);
                }
            }
            None => out.push(Target {
                link: doc
                    .settings()
                    .sources
                    .iter()
                    .find(|l| l.layer == layer)
                    .cloned(),
                name: node.name.clone(),
                layer,
                items: vec![item],
            }),
        }
    }
    Ok((out, wait))
}

/// The layer and the item of each list from another layer and each related item, in the form's order.
fn listed(body: &FormBody) -> Vec<(String, String)> {
    fn walk(items: &[FormItem], out: &mut Vec<(String, String)>) {
        for item in items {
            match item {
                FormItem::Field(f) => {
                    if let Some(l) = &f.lookup {
                        out.push((l.layer.clone(), f.id.clone()));
                    }
                }
                FormItem::Related { id, layer, .. } => out.push((layer.clone(), id.clone())),
                FormItem::Tab { items, .. } | FormItem::Group { items, .. } => walk(items, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&body.items, &mut out);
    out
}

/// How `target`'s source stands now: its file asked by its bytes, its database by a connection that waits
/// `wait` at most.
pub fn probe(target: &Target, wait: Duration) -> FormSource {
    let mut out = FormSource {
        layer: target.layer.clone(),
        name: target.name.clone(),
        items: target.items.clone(),
        kind: target.link.as_ref().map(|l| l.kind),
        table: target.link.as_ref().map(|l| l.table.clone()),
        state: FormSourceState::Local,
        code: None,
        message: None,
        retry: None,
    };
    let Some(link) = &target.link else {
        return out;
    };
    let name = &target.name;
    let said = |state, code: &str, words: String, retry| {
        (
            state,
            Some(code.to_owned()),
            Some(format!("{words} Liste son okunan kopyadan sunuluyor.")),
            Some(retry),
        )
    };
    let (state, code, message, retry) = match link.kind {
        SourceKind::Geopackage => {
            let path = Path::new(&link.path);
            let file = path
                .file_name()
                .map_or_else(|| link.path.clone(), |n| n.to_string_lossy().into_owned());
            if !path.is_file() {
                said(
                    FormSourceState::Missing,
                    form_codes::SOURCE_MISSING,
                    format!(
                        "“{name}” katmanının dosyası ({file}) yerinde yok; dosyayı yeniden bağlayın."
                    ),
                    FormSourceRetry::Relink,
                )
            } else {
                match GeoPackage::open(path, false).and_then(|g| g.layers()) {
                    Err(_) => said(
                        FormSourceState::Failed,
                        form_codes::SOURCE_FAILED,
                        format!(
                            "“{name}” katmanının dosyası ({file}) GeoPackage olarak okunamadı."
                        ),
                        FormSourceRetry::Retry,
                    ),
                    Ok(layers) if !layers.iter().any(|l| l.name == link.table) => said(
                        FormSourceState::Missing,
                        form_codes::SOURCE_MISSING,
                        format!(
                            "“{name}” katmanının tablosu (“{}”) {file} dosyasında yok; katmanı yeniden bağlayın.",
                            link.table
                        ),
                        FormSourceRetry::Relink,
                    ),
                    Ok(_) => match crate::file_digest::sha256(path) {
                        Err(_) => said(
                            FormSourceState::Failed,
                            form_codes::SOURCE_FAILED,
                            format!("“{name}” katmanının dosyası ({file}) okunamadı."),
                            FormSourceRetry::Retry,
                        ),
                        Ok((_, sha)) if sha != link.revision => said(
                            FormSourceState::Changed,
                            form_codes::SOURCE_CHANGED,
                            format!(
                                "“{name}” katmanının dosyası ({file}) okunduktan sonra değişti; kaynaktan yeniden okuyun."
                            ),
                            FormSourceRetry::Read,
                        ),
                        Ok(_) => (FormSourceState::Ready, None, None, None),
                    },
                }
            }
        }
        SourceKind::Postgis => match provider::postgis() {
            None => said(
                FormSourceState::Failed,
                form_codes::SOURCE_FAILED,
                format!(
                    "Bu ortamda veritabanı sağlayıcısı yok; “{name}” katmanının kaynağı sorulamadı."
                ),
                FormSourceRetry::Retry,
            ),
            Some(p) => match p.reach(&link.path, &link.table, wait) {
                Ok(()) => (FormSourceState::Ready, None, None, None),
                Err(r) => {
                    let (state, code, retry) = match r.problem {
                        Unreached::Offline => (
                            FormSourceState::Offline,
                            form_codes::SOURCE_OFFLINE,
                            FormSourceRetry::Retry,
                        ),
                        Unreached::Timeout => (
                            FormSourceState::Timeout,
                            form_codes::SOURCE_TIMEOUT,
                            FormSourceRetry::Retry,
                        ),
                        Unreached::Denied => (
                            FormSourceState::Denied,
                            form_codes::SOURCE_DENIED,
                            FormSourceRetry::Credentials,
                        ),
                        Unreached::Missing => (
                            FormSourceState::Missing,
                            form_codes::SOURCE_MISSING,
                            FormSourceRetry::Relink,
                        ),
                        Unreached::Failed => (
                            FormSourceState::Failed,
                            form_codes::SOURCE_FAILED,
                            FormSourceRetry::Retry,
                        ),
                    };
                    said(
                        state,
                        code,
                        format!(
                            "“{name}” katmanının veritabanı: {}.",
                            r.message.trim_end_matches('.')
                        ),
                        retry,
                    )
                }
            },
        },
    };
    out.state = state;
    out.code = code;
    out.message = message;
    out.retry = retry;
    out
}

fn answer(doc: &Document, input: &FormsSources) -> Result<FormsSourced, Stop> {
    let (targets, wait) = targeted(doc, input)?;
    Ok(FormsSourced {
        sources: targets.iter().map(|t| probe(t, wait)).collect(),
        revision: doc.revision().to_string(),
    })
}

/// Checks `input` against the document, asking no source.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsSources) -> CommandResult<()> {
    match targeted(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The sources' states; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsSources) -> CommandResult<FormsSourced> {
    match answer(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// As plan: a query writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsSources) -> CommandResult<FormsSourced> {
    plan(cx, &input)
}
