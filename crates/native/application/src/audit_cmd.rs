//! `cad.drawing.audit` v1 (docs/adr/0306; AutoCAD's AUDIT and PURGE, Denetle): what the drawing carries broken and
//! the named resources nothing uses, reported; with `fix` the broken objects repaired as one undo step “Denetle”, with
//! `purge` the unused styles removed (a project setting). The rules are the document's (`kentos_domain::audit`,
//! `layer_purge`); the shared cases are `fixtures/commands/v1/cad.drawing.audit.json`.

use kentos_contracts::{
    AuditCode, AuditFinding, AuditPurge, CommandResult, CommandWarning, DrawingAudit,
    DrawingAudited, ResourceKind, UnusedResource,
};
use kentos_domain::audit::AuditKind;
use kentos_domain::layer_purge;
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Denetle";

fn code(kind: AuditKind) -> AuditCode {
    match kind {
        AuditKind::BrokenLink => AuditCode::BrokenLink,
        AuditKind::BrokenHatch => AuditCode::BrokenHatch,
        AuditKind::MissingCutout => AuditCode::MissingCutout,
        AuditKind::HatchTextCycle => AuditCode::HatchTextCycle,
        AuditKind::BrokenDimension => AuditCode::BrokenDimension,
        AuditKind::MissingTextStyle => AuditCode::MissingTextStyle,
        AuditKind::MissingDimensionStyle => AuditCode::MissingDimensionStyle,
    }
}

/// What the audit finds and what it would write.
struct Found {
    findings: Vec<AuditFinding>,
    unused: Vec<UnusedResource>,
    /// The objects to repair, in the drawing's order.
    repairs: Vec<Slot>,
    /// The styles to remove, by their ids.
    text_styles: Vec<String>,
    dimension_styles: Vec<String>,
    warnings: Vec<CommandWarning>,
}

fn find(doc: &Document, input: &DrawingAudit) -> Found {
    let layers = doc.layers();
    let uid = |slot: Slot| doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
    let mut findings = Vec::new();
    let mut repairs: Vec<Slot> = Vec::new();
    let mut locked = 0;
    for f in doc.audit() {
        let held = doc
            .get(f.slot)
            .is_some_and(|e| layers.is_locked(&e.base().layer_id));
        let fixed = input.fix && !held;
        if fixed && repairs.last() != Some(&f.slot) {
            repairs.push(f.slot);
        }
        if input.fix
            && held
            && findings
                .last()
                .is_none_or(|l: &AuditFinding| l.uid != uid(f.slot))
        {
            locked += 1;
        }
        findings.push(AuditFinding {
            code: code(f.kind),
            uid: uid(f.slot),
            message: f.message,
            fixed,
        });
    }
    let mut warnings = Vec::new();
    if locked > 0 {
        warnings.push(CommandWarning {
            code: codes::LAYER_LOCKED.into(),
            message: format!(
                "{locked} nesne kilitli katmanda olduğu için onarılmadı. Onarmak için katmanın kilidini Katmanlar panelinden açın."
            ),
            path: Some("fix".into()),
        });
    }
    let mut unused = Vec::new();
    let found = layer_purge::found(&layer_purge::source_of(doc, Vec::new()));
    for (kind, list) in [
        (ResourceKind::Layer, &found.layers),
        (ResourceKind::Group, &found.groups),
        (ResourceKind::Block, &found.blocks),
    ] {
        unused.extend(list.iter().filter(|f| !f.locked).map(|f| UnusedResource {
            kind,
            id: f.id.clone(),
            name: f.text.clone(),
            purged: false,
        }));
    }
    let (text, dims) = doc.unused_styles();
    let settings = doc.settings();
    let purge_text = input.purge.contains(&AuditPurge::TextStyles);
    let purge_dims = input.purge.contains(&AuditPurge::DimensionStyles);
    for id in &text {
        let name = settings
            .text_styles
            .iter()
            .find(|s| &s.id == id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        unused.push(UnusedResource {
            kind: ResourceKind::TextStyle,
            id: id.clone(),
            name,
            purged: purge_text,
        });
    }
    for id in &dims {
        let name = settings
            .dimension_styles
            .iter()
            .find(|s| &s.id == id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        unused.push(UnusedResource {
            kind: ResourceKind::DimensionStyle,
            id: id.clone(),
            name,
            purged: purge_dims,
        });
    }
    Found {
        findings,
        unused,
        repairs,
        text_styles: if purge_text { text } else { Vec::new() },
        dimension_styles: if purge_dims { dims } else { Vec::new() },
        warnings,
    }
}

fn check(doc: &Document, input: &DrawingAudit) -> Result<Found, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(find(doc, input))
}

/// The writes: the repairs as one step, the styles' table.
fn write(cx: &mut ExecutionContext<'_>, found: Found) -> CommandResult<DrawingAudited> {
    let Found {
        findings,
        unused,
        repairs,
        text_styles,
        dimension_styles,
        warnings,
    } = found;
    let doc = &mut *cx.doc;
    let repaired: Vec<(Slot, kentos_contracts::Entity)> = repairs
        .iter()
        .filter_map(|slot| doc.audit_repaired(*slot).map(|e| (*slot, e)))
        .collect();
    let changed: Vec<String> = repaired
        .iter()
        .filter_map(|(slot, _)| doc.uid(*slot).map(|u| u.to_string()))
        .collect();
    if !repaired.is_empty() {
        doc.name_step(LABEL);
        doc.update_many(repaired, LABEL);
    }
    if !text_styles.is_empty() || !dimension_styles.is_empty() {
        let mut settings = doc.settings().clone();
        settings
            .text_styles
            .retain(|s| !text_styles.contains(&s.id));
        settings
            .dimension_styles
            .retain(|s| !dimension_styles.contains(&s.id));
        doc.set_settings(settings);
    }
    CommandResult::Completed {
        output: DrawingAudited {
            findings,
            unused,
            changed,
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

/// `cad.drawing.audit` v1: Denetle.
pub mod drawing_audit {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &DrawingAudit) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(found) => CommandResult::Completed {
                output: (),
                warnings: found.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &DrawingAudit) -> CommandResult<DrawingAudited> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document alone: the write on a copy, what it reports.
    pub fn plan_in(doc: &Document, input: &DrawingAudit) -> CommandResult<DrawingAudited> {
        if let Err(stop) = check(doc, input) {
            return stop.into();
        }
        let mut copy = doc.clone();
        match execute(&mut ExecutionContext::new(&mut copy), input.clone()) {
            CommandResult::Completed {
                mut output,
                warnings,
            } => {
                output.revision = doc.revision().to_string();
                CommandResult::Completed { output, warnings }
            }
            other => other,
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DrawingAudit,
    ) -> CommandResult<DrawingAudited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_DRAWING_AUDIT,
            kentos_contracts::CAD_DRAWING_AUDIT_VERSION,
            input,
            |cx, input| match check(cx.doc, &input) {
                Ok(found) => write(cx, found),
                Err(stop) => stop.into(),
            },
        )
    }
}
