//! `cad.domains.export` v1 (Değer listelerini dışa aktar, docs/adr/0439 §7): the project's value lists as a
//! package, read-only; `cad.domains.edit`'s import takes it into another project. The shared cases are
//! `fixtures/commands/v1/cad.domains.export.json`.

use kentos_contracts::{
    CommandResult, DOMAINS_PACKAGE, DOMAINS_PACKAGE_VERSION, DomainsExport, DomainsPackage,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

/// The package of `input`'s lists.
fn package(doc: &Document, input: &DomainsExport) -> Result<DomainsPackage, Stop> {
    let all = &doc.settings().domains;
    let domains = if input.ids.is_empty() {
        all.clone()
    } else {
        let mut out = Vec::new();
        for (i, id) in input.ids.iter().enumerate() {
            let Some(d) = all.iter().find(|d| d.id == *id) else {
                return Err(Stop::failed(error(
                    codes::UNKNOWN_DOMAIN,
                    format!(
                        "“{id}” kimlikli bir değer listesi yok. Değer listeleri penceresinden var olan bir listeyi seçin."
                    ),
                    Some(format!("ids/{i}")),
                )));
            };
            if !out
                .iter()
                .any(|x: &kentos_contracts::FieldDomain| x.id == d.id)
            {
                out.push(d.clone());
            }
        }
        out
    };
    Ok(DomainsPackage {
        format: DOMAINS_PACKAGE.into(),
        version: DOMAINS_PACKAGE_VERSION,
        domains,
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &DomainsExport) -> CommandResult<()> {
    match package(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The package; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &DomainsExport) -> CommandResult<DomainsPackage> {
    match package(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// As plan: a query writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: DomainsExport,
) -> CommandResult<DomainsPackage> {
    plan(cx, &input)
}
