//! `cad.modules.query` v1 (docs/adr/0775): the project's room modules with each instance's state (its version against
//! the module's, its objects, its local exceptions, the keys whose objects are gone, a broken link), or the module and
//! instance an object belongs to, and the objects changed by hand away from the source (docs/adr/0779 §3); a module of
//! several storeys its storeys and each instance's levels (docs/adr/0796 §1). Read-only. The shared cases are
//! `fixtures/commands/v1/cad.modules.query.json`.

use kentos_commands::{CommandResult, ModuleInstanceRow, ModuleRow, ModulesQueried, ModulesQuery};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::openings::fail;

fn present(doc: &Document, uid: &str) -> bool {
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .is_some()
}

/// The answer over `doc`.
pub fn query(doc: &Document, input: &ModulesQuery) -> CommandResult<ModulesQueried> {
    match answer(doc, input) {
        Ok(out) => CommandResult::Completed {
            output: out,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

fn answer(doc: &Document, input: &ModulesQuery) -> Result<ModulesQueried, Stop> {
    let all = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.modules.as_slice())
        .unwrap_or(&[]);
    let mut chosen: Vec<&kentos_commands::ArchModule> = match &input.module {
        Some(key) => {
            let folded = key.trim().to_lowercase();
            let m = all
                .iter()
                .find(|m| &m.id == key)
                .or_else(|| all.iter().find(|m| m.name.to_lowercase() == folded))
                .ok_or_else(|| {
                    fail(
                        codes::MODULE_NOT_FOUND,
                        format!("Projede “{key}” adlı ya da kimlikli oda modülü yok."),
                        "module",
                    )
                })?;
            vec![m]
        }
        None => all.iter().collect(),
    };
    let mut instance = None;
    let mut key = None;
    let mut drifted = false;
    if let Some(uid) = &input.uid {
        checks::uids(std::slice::from_ref(uid), "")
            .and_then(|()| checks::named(doc, std::slice::from_ref(uid)).map(|_| ()))
            .map_err(|s| {
                let mut e = s.into_error();
                e.path = Some("uid".into());
                Stop::failed(e)
            })?;
        let found = chosen.iter().find_map(|m| {
            m.instances.iter().find_map(|i| {
                i.members
                    .iter()
                    .find(|o| &o.uid == uid)
                    .map(|o| (m.id.clone(), i.id.clone(), o.key.clone()))
            })
        });
        let Some((mid, iid, k)) = found else {
            return Err(fail(
                codes::NOT_IN_MODULE,
                "Nesne bir oda modülünün örneğinde değil.".into(),
                "uid",
            ));
        };
        chosen.retain(|m| m.id == mid);
        if let Some(i) = chosen.first().and_then(|m| m.instance(&iid)) {
            drifted = crate::modules_edit::drifted_keys(doc, chosen[0], i).contains(&k);
        }
        instance = Some(iid);
        key = Some(k);
    }
    let order = crate::modules_edit::storey_order(doc);
    let modules = chosen
        .into_iter()
        .map(|m| ModuleRow {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version,
            members: u32::try_from(m.members.len()).unwrap_or(u32::MAX),
            storeys: (m.storeys() > 1).then(|| m.storeys()),
            removed: m.removed,
            instances: m
                .instances
                .iter()
                .map(|i| ModuleInstanceRow {
                    id: i.id.clone(),
                    version: i.version,
                    stale: i.version < m.version,
                    objects: u32::try_from(
                        i.members.iter().filter(|o| present(doc, &o.uid)).count(),
                    )
                    .unwrap_or(0),
                    exceptions: i
                        .members
                        .iter()
                        .filter(|o| i.exceptions.contains(&o.key))
                        .map(|o| o.uid.clone())
                        .collect(),
                    missing: i
                        .members
                        .iter()
                        .filter(|o| !present(doc, &o.uid))
                        .map(|o| o.key.clone())
                        .collect(),
                    drifted: crate::modules_edit::drifted_uids(doc, m, i),
                    broken: i.broken,
                    unit: i.unit.clone(),
                    level: i.level.clone(),
                    levels: if m.storeys() > 1 {
                        (0..m.storeys())
                            .filter_map(|n| {
                                crate::modules_edit::storey_level(&order, i.level.as_deref(), n)
                                    .flatten()
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    mirror: i.mirror,
                })
                .collect(),
        })
        .collect();
    Ok(ModulesQueried {
        modules,
        instance,
        key,
        drifted,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ModulesQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ModulesQuery) -> CommandResult<ModulesQueried> {
    query(cx.doc, input)
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ModulesQuery,
) -> CommandResult<ModulesQueried> {
    query(cx.doc, &input)
}
