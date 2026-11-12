//! `cad.topology.exceptions` (docs/adr/0589 §3, 1260 §1): findings marked as left on purpose (`mark`), confirmed as
//! errors to be put right (`error`), or the mark taken away (`unmark`). A finding is named by its lasting key as
//! `cad.topology.check` gives it for the drawing as it is now; the mark keeps the finding's rule, objects and place, the
//! digest of the objects' geometry and the rule's version, so it lapses once either changes; and the reviewer's reason
//! (required when the project has a quality profile) and what an exception covers. The marks are the project's
//! settings: writing them makes the drawing dirty and is no undo step.

use kentos_commands::{
    CommandResult, ProjectSettings, TopologyDecision, TopologyExceptionOperation,
    TopologyExceptions, TopologyExceptionsEdited, TopologyScope, reason_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::topology_check::{exception_of, gather, mark_of, run};

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
    let marking = input.operation != TopologyExceptionOperation::Unmark;
    match &input.reason {
        Some(_) if !marking => {
            return Err(Stop::failed(error(
                codes::INVALID_INPUT,
                "Gerekçe yalnız işaretlerken (mark, error) yazılır; unmark gerekçe almaz.".into(),
                Some("reason".into()),
            )));
        }
        Some(r) => {
            if let Some(why) = reason_problem(r) {
                return Err(Stop::failed(error(
                    codes::INVALID_INPUT,
                    format!("İşaretin {why}; 1 ile 500 karakter arasında, boş olmayan bir gerekçe yazın."),
                    Some("reason".into()),
                )));
            }
        }
        None => {}
    }
    if input.scope.is_some() && input.operation != TopologyExceptionOperation::Mark {
        return Err(Stop::failed(error(
            codes::INVALID_INPUT,
            "Kapsam yalnız istisnanın (mark) alanıdır.".into(),
            Some("scope".into()),
        )));
    }
    if marking
        && input.reason.is_none()
        && let Some(p) = doc
            .settings()
            .topology
            .as_ref()
            .and_then(|t| t.profile.as_ref())
    {
        return Err(Stop::failed(error(
            codes::REASON_REQUIRED,
            format!(
                "Projenin kalite profili (“{}”) her istisnanın ve hata işaretinin gerekçesini ister; reason'ı yazın.",
                p.name
            ),
            Some("reason".into()),
        )));
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
    let anywhere = input.scope == Some(TopologyScope::Objects);
    for f in chosen {
        let rule = g.core_rules.get(f.rule).map_or("", |r| r.id.as_str());
        let naming = |x: &kentos_commands::TopologyException| g.same(&mark_of(x), rule, f);
        match input.operation {
            TopologyExceptionOperation::Mark | TopologyExceptionOperation::Error => {
                let error = input.operation == TopologyExceptionOperation::Error;
                // The same decision, reason and cover holding already: nothing to write.
                if g.review(f).is_some_and(|r| {
                    r.lapse.is_none()
                        && r.error == error
                        && r.reason == input.reason
                        && r.mark.anywhere == anywhere
                }) || (!error && f.exception && input.reason.is_none() && !anywhere)
                {
                    continue;
                }
                // A lapsed or another mark is replaced, on the geometry and the rule as they are now.
                exceptions.retain(|x| !naming(x));
                if let Some(mut x) = exception_of(&g, f) {
                    x.decision = error.then_some(TopologyDecision::Error);
                    x.reason = input.reason.clone();
                    x.scope = anywhere.then_some(TopologyScope::Objects);
                    exceptions.push(x);
                    changed += 1;
                }
            }
            TopologyExceptionOperation::Unmark => {
                if g.review(f).is_none() && !f.exception {
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
