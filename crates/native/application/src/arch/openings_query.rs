//! `cad.openings.query` v1 (read-only, docs/adr/0772 §2, §4): the drawing's openings with their types, the version they
//! took, their exceptions, their level and their four widths (rough opening, frame's outer width, leaf width, net clear
//! width), filtered by a selection, a type, a level and a form; and the project's types with their openings counted.
//! “Aynı tiptekileri seç” and the net clear width all read it. The shared cases are
//! `fixtures/commands/v1/cad.openings.query.json` (`scripts/fixtures/opening_types_cases.py`).

use kentos_commands::{
    CommandResult, Entity, OpeningQueryRow, OpeningTypeUse, OpeningsQueried, OpeningsQuery,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::opening_types::{self as types, Found};
use crate::openings::fail;

fn check(doc: &Document, input: &OpeningsQuery) -> Result<OpeningsQueried, Stop> {
    let scope: Option<Vec<Slot>> = if input.uids.is_empty() {
        None
    } else {
        checks::uids(&input.uids, "Okunacak açıklık verilmedi.")?;
        let found = checks::named(doc, &input.uids)?;
        let mut slots = Vec::with_capacity(found.len());
        for (at, slot, e, _) in found {
            if !matches!(e, Entity::Opening(_)) {
                return Err(fail(
                    codes::NOT_AN_OPENING,
                    "Nesne kapı, pencere ya da boşluk değil.".into(),
                    &format!("uids[{at}]"),
                ));
            }
            slots.push(slot);
        }
        Some(slots)
    };
    // The type named: its id (a project's, else KentOS's).
    let kind: Option<String> = match &input.kind {
        None => None,
        Some(k) => Some(match types::find(doc, k, "type")? {
            Found::Project(t) => t.id,
            Found::Builtin(t) => t.id,
        }),
    };
    if let Some(Some(level)) = &input.level {
        let known = doc
            .settings()
            .arch
            .as_ref()
            .is_some_and(|a| a.levels.iter().any(|l| l.id == *level));
        if !known {
            return Err(fail(
                codes::LEVEL_NOT_FOUND,
                format!("Projede “{level}” kimlikli kat yok."),
                "level",
            ));
        }
    }
    let project = doc.opening_types();
    let mut uses: Vec<OpeningTypeUse> = project
        .iter()
        .map(|t| OpeningTypeUse {
            kind: t.clone(),
            count: 0,
            stale: 0,
            exceptions: 0,
        })
        .collect();
    let mut rows = Vec::new();
    for e in doc.entities() {
        let Entity::Opening(o) = e else { continue };
        let f = &o.opening;
        let slot = Slot(o.base.id);
        let ty = f
            .style
            .as_deref()
            .and_then(|s| project.iter().position(|t| t.id == s));
        let following = f.follows_type();
        let stale =
            following && ty.is_some_and(|i| f.type_version.unwrap_or(0) < project[i].version);
        if let (Some(i), true) = (ty, following) {
            uses[i].count += 1;
            uses[i].stale += u32::from(stale);
            uses[i].exceptions += u32::from(!f.overrides.is_empty());
        }
        if scope.as_ref().is_some_and(|s| !s.contains(&slot)) {
            continue;
        }
        if kind
            .as_deref()
            .is_some_and(|k| f.style.as_deref() != Some(k))
        {
            continue;
        }
        if input.form.is_some_and(|form| form != f.form) {
            continue;
        }
        let level = types::level_of(doc, f);
        if let Some(want) = &input.level
            && *want != level
        {
            continue;
        }
        rows.push(OpeningQueryRow {
            uid: doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
            form: f.form,
            number: f.number.clone(),
            kind: f.style.clone(),
            type_name: f.style.as_deref().map(|s| doc.opening_type_name(Some(s))),
            type_version: f.type_version,
            stale,
            overrides: f.overrides.clone(),
            level,
            host: f.host.clone(),
            sizes: f.sizes(),
        });
    }
    Ok(OpeningsQueried {
        openings: rows,
        types: uses,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &OpeningsQuery) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &OpeningsQuery) -> CommandResult<OpeningsQueried> {
    query(cx.doc, input)
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: OpeningsQuery,
) -> CommandResult<OpeningsQueried> {
    query(cx.doc, &input)
}

/// The query on a document.
pub fn query(doc: &Document, input: &OpeningsQuery) -> CommandResult<OpeningsQueried> {
    match check(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
