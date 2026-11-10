//! A map work item's typed layers' profile (plan/map-cad.md MAP-01, docs/adr/4880): a data profile (docs/adr/0541) by
//! name, narrowed to the classes the item holds. The profile types the classes' layers (each its kind of geometry and its
//! fields, checked as `cad.profiles.check` checks them); the item's other layers stay free drawing layers. A profile
//! not built in is the drawing's copy or a file in the project's folder, as for `cad.profiles.apply`.

use kentos_commands::{DataProfile, MapWork, ProfileFinding, WorkItem, WorkProfile, WorkProfileRow};
use kentos_domain::Document;

use crate::checks::{Stop, error};
use crate::codes;
use crate::map_work::item_of_layer;
use crate::profile_files::Shelf;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The profile `name`, found as `cad.profiles.apply` finds it; its refusal at `at`.
pub(crate) fn load(doc: &Document, name: &str, drawing: Option<&str>, at: &str) -> Result<DataProfile, Stop> {
    crate::profiles::profile_only(Some(name), None, Shelf::of(doc, drawing)).map_err(|stop| stop.at(at))
}

/// The profile with only `classes` (none: all of it), the value lists they bind and the relations between two of them;
/// the address data only when every class is kept. A class not in the profile is refused at `operation.classes[i]`.
pub(crate) fn narrowed(p: &DataProfile, classes: &[String]) -> Result<DataProfile, Stop> {
    if classes.is_empty() {
        return Ok(p.clone());
    }
    for (i, c) in classes.iter().enumerate() {
        if !p.classes.iter().any(|k| k.id == *c) {
            let have: Vec<&str> = p.classes.iter().map(|k| k.id.as_str()).collect();
            return Err(fail(
                codes::UNKNOWN_PROFILE_CLASS,
                format!("“{}” profilinde “{c}” sınıfı yok; sınıfları: {}.", p.name, have.join(", ")),
                &format!("operation.classes[{i}]"),
            ));
        }
    }
    let mut out = p.clone();
    out.classes.retain(|k| classes.contains(&k.id));
    out.relations.retain(|r| classes.contains(&r.parent) && classes.contains(&r.child));
    let bound: Vec<&str> =
        out.classes.iter().flat_map(|k| k.fields.iter().filter_map(|f| f.domain.as_deref())).collect();
    out.domains.retain(|d| bound.contains(&d.id.as_str()));
    out.address = None;
    Ok(out)
}

/// The item's profile narrowed, when it has one and it is found.
pub(crate) fn of_item(doc: &Document, item: &WorkItem, drawing: Option<&str>) -> Option<Result<DataProfile, Stop>> {
    let WorkProfile { name, classes } = item.profile.as_ref()?;
    Some(load(doc, name, drawing, "operation.status").and_then(|p| narrowed(&p, classes)))
}

/// The item's typed layers' findings: those of its classes (a value list, a relation or the address data the project
/// lacks is the project's, not the item's).
pub(crate) fn findings(doc: &Document, p: &DataProfile) -> (kentos_commands::ProfilesReport, Vec<ProfileFinding>) {
    let report = crate::profiles::report(doc, p);
    let mine = report.findings.iter().filter(|f| f.class.is_some()).cloned().collect();
    (report, mine)
}

/// How the item's profile stands, for `map.work.query`.
pub(crate) fn row(doc: &Document, work: &MapWork, item: &WorkItem, drawing: Option<&str>) -> Option<WorkProfileRow> {
    let name = item.profile.as_ref()?.name.clone();
    let Some(Ok(p)) = of_item(doc, item, drawing) else {
        return Some(WorkProfileRow {
            name,
            found: false,
            title: None,
            version: None,
            classes: Vec::new(),
            findings: Vec::new(),
            free: Vec::new(),
        });
    };
    let (report, mut mine) = findings(doc, &p);
    mine.truncate(kentos_commands::WORK_CHANGES_LISTED);
    // The item's layers holding objects that no class of it types.
    let typed: Vec<&str> = report.classes.iter().filter_map(|c| c.layer.as_deref()).collect();
    let mut free: Vec<String> = Vec::new();
    for e in doc.entities() {
        let layer = &e.base().layer_id;
        if typed.contains(&layer.as_str()) || free.contains(layer) {
            continue;
        }
        if item_of_layer(doc, work, layer).is_some_and(|i| i.id == item.id) {
            free.push(layer.clone());
        }
    }
    free.sort();
    Some(WorkProfileRow {
        name,
        found: true,
        title: Some(report.title.clone()),
        version: Some(report.version),
        classes: report.classes,
        findings: mine,
        free,
    })
}
