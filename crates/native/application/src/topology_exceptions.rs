//! `cad.topology.exceptions` (docs/adr/0589 §3): findings marked as left on purpose, or the mark taken away. A finding
//! is named by its lasting key as `cad.topology.check` gives it for the drawing as it is now; the exception keeps the
//! finding's rule, objects and place and the digest of the objects' geometry, so it lapses once the geometry changes.
//! The exceptions are the project's settings: writing them makes the drawing dirty and is no undo step.

use kentos_commands::{
    CommandResult, ProjectSettings, TopologyExceptionOperation, TopologyExceptions,
    TopologyExceptionsEdited,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::topology_check::{exception_of, gather, run};

/// The project's exceptions after the operation, and how many findings it changed.
struct Checked {
    exceptions: Vec<kentos_commands::TopologyException>,
    changed: usize,
}

fn check(doc: &Document, input: &TopologyExceptions) -> Result<Checked, Stop> {
    if input.keys.is_empty() {
        return Err(Stop::failed(error(
            codes::INVALID_INPUT,
            "Bulgu anahtarı verilmedi; cad.topology.check'in verdiği anahtarlardan en az birini verin.".into(),
            Some("keys".into()),
        )));
    }
    for (i, k) in input.keys.iter().enumerate() {
        if input.keys[..i].contains(k) {
            return Err(Stop::failed(error(
                codes::INVALID_INPUT,
                format!("“{k}” anahtarı birden çok kez verildi."),
                Some(format!("keys[{i}]")),
            )));
        }
    }
    let g = gather(doc, None);
    let found = run(&g);
    let mut chosen = Vec::with_capacity(input.keys.len());
    for (i, k) in input.keys.iter().enumerate() {
        let Some(f) = found.findings.iter().find(|f| g.key(f) == *k) else {
            return Err(Stop::failed(error(
                codes::FINDING_NOT_FOUND,
                format!(
                    "Çizimin bugünkü denetiminde “{k}” anahtarlı bulgu yok; önce cad.topology.check ile denetleyin."
                ),
                Some(format!("keys[{i}]")),
            )));
        };
        chosen.push(f);
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let settings = doc.settings().topology.clone().unwrap_or_default();
    let mut exceptions = settings.exceptions.clone();
    let mut changed = 0;
    for f in chosen {
        let rule = g.core_rules.get(f.rule).map_or("", |r| r.id.as_str());
        let naming = |x: &kentos_commands::TopologyException| {
            let core = kentos_geometry_core::ops::topology_rules::Exception {
                rule: x.rule.clone(),
                objects: x.objects.iter().map(|id| id.to_text()).collect(),
                at: kentos_geometry_core::vec2::Vec2::new(x.at.x, x.at.y),
            };
            g.same(&core, rule, f)
        };
        match input.operation {
            TopologyExceptionOperation::Mark => {
                if f.exception {
                    continue;
                }
                // A lapsed exception is marked again, on the geometry as it is now.
                exceptions.retain(|x| !naming(x));
                if let Some(x) = exception_of(&g, f) {
                    exceptions.push(x);
                    changed += 1;
                }
            }
            TopologyExceptionOperation::Unmark => {
                if !f.exception && !g.lapsed(f) {
                    continue;
                }
                exceptions.retain(|x| !naming(x));
                changed += 1;
            }
        }
    }
    Ok(Checked {
        exceptions,
        changed,
    })
}

fn edited(doc: &Document, checked: &Checked) -> TopologyExceptionsEdited {
    TopologyExceptionsEdited {
        changed: checked.changed.min(u32::MAX as usize) as u32,
        exceptions: checked.exceptions.len().min(u32::MAX as usize) as u32,
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TopologyExceptions) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &TopologyExceptions,
) -> CommandResult<TopologyExceptionsEdited> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: edited(cx.doc, &c),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: TopologyExceptions,
) -> CommandResult<TopologyExceptionsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_TOPOLOGY_EXCEPTIONS,
        kentos_commands::CAD_TOPOLOGY_EXCEPTIONS_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the document (docs/adr/0253 §1, §3).
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: TopologyExceptions,
) -> CommandResult<TopologyExceptionsEdited> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    if checked.changed > 0 {
        let doc = &mut *cx.doc;
        let mut topology = doc.settings().topology.clone().unwrap_or_default();
        topology.exceptions = checked.exceptions.clone();
        doc.set_settings(ProjectSettings {
            topology: topology.sanitized(),
            ..doc.settings().clone()
        });
    }
    CommandResult::Completed {
        output: edited(cx.doc, &checked),
        warnings: Vec::new(),
    }
}
