//! The web's calls (docs/adr/0202): the catalog of kinds, problems and
//! fixes, the check, and a fix. Objects come as the drawing holds them,
//! with their layers' ids and their persistent ids beside them.

use super::{
    Exception, Finding, Kind, MeasureKind, Objects, Rule, Spot, check, fix, fix_label,
    problem_label,
};
use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::arrangement::Area;
use crate::geom::intersect::Edge;
use crate::geometry::Bounds;
use crate::op;
use crate::vec2::Vec2;

/// Every problem a finding may name.
const PROBLEMS: [&str; 18] = [
    "overlap",
    "gap",
    "sliver",
    "duplicateEdge",
    "duplicatePoint",
    "dangle",
    "shortEdge",
    "smallAngle",
    "repeated",
    "zeroArea",
    "ringCrossing",
    "pathCrossing",
    "holeOutside",
    "holesOverlap",
    "missingVertex",
    "outside",
    "uncoveredBoundary",
    "notOnEnd",
];

/// Every fix a finding may offer, in the order the Düzelt menu lists them.
const FIXES: [&str; 12] = [
    "subtractFirst",
    "subtractSecond",
    "mergeNeighbour",
    "deletePart",
    "deleteDuplicate",
    "snapEnd",
    "removeVertex",
    "deleteObject",
    "repair",
    "addVertex",
    "clipOutside",
    "snapToEnd",
];

struct KindOut {
    key: &'static str,
    label: &'static str,
    takes: &'static str,
    between: bool,
    value: Option<&'static str>,
    default_value: Option<f64>,
    value_label: Option<&'static str>,
}

crate::json_struct!(out KindOut { key, label, takes, between, value, default_value => "defaultValue", value_label => "valueLabel" });

struct Named {
    key: &'static str,
    label: &'static str,
}

crate::json_struct!(out Named { key, label });

struct CatalogOut {
    kinds: Vec<KindOut>,
    problems: Vec<Named>,
    fixes: Vec<Named>,
}

crate::json_struct!(out CatalogOut { kinds, problems, fixes });

struct RuleIn {
    id: String,
    kind: String,
    layer: String,
    other: Option<String>,
    value: Option<f64>,
}

crate::json_struct!(RuleIn {
    id,
    kind,
    layer,
    other,
    value
});

struct ExceptionIn {
    rule: String,
    objects: Vec<String>,
    at: Vec2,
}

crate::json_struct!(ExceptionIn { rule, objects, at });

crate::json_struct!(Spot { part, ring, index });

struct FindingOut {
    rule: usize,
    problem: &'static str,
    label: &'static str,
    objects: Vec<usize>,
    at: Vec2,
    bounds: Bounds,
    regions: Vec<Area>,
    edges: Vec<Edge>,
    measure: Option<f64>,
    measure_kind: Option<&'static str>,
    fixes: Vec<Named>,
    exception: bool,
    spot: Option<Spot>,
    target: Option<Vec2>,
    subject: Option<usize>,
}

crate::json_struct!(out FindingOut {
    rule,
    problem,
    label,
    objects,
    at,
    bounds,
    regions,
    edges,
    measure,
    measure_kind => "measureKind",
    fixes,
    exception,
    spot,
    target,
    subject,
});

struct CheckedOut {
    findings: Vec<FindingOut>,
    looked: Vec<usize>,
}

crate::json_struct!(out CheckedOut { findings, looked });

/// What a fix needs of a finding, as the check wrote it.
struct FixIn {
    key: String,
}

crate::json_struct!(FixIn { key });

struct FindingIn {
    rule: usize,
    problem: String,
    objects: Vec<usize>,
    at: Vec2,
    regions: Vec<Area>,
    fixes: Vec<FixIn>,
    spot: Option<Spot>,
    target: Option<Vec2>,
    subject: Option<usize>,
}

crate::json_struct!(FindingIn {
    rule,
    problem,
    objects,
    at,
    regions,
    fixes,
    spot,
    target,
    subject
});

struct ChangeOut {
    object: usize,
    shape: Option<Entity>,
}

crate::json_struct!(out ChangeOut { object, shape });

fn named(key: &'static str, label: &'static str) -> Named {
    Named { key, label }
}

fn rules_of(rules: Vec<RuleIn>) -> Result<Vec<Rule>, String> {
    rules
        .into_iter()
        .map(|r| {
            Ok(Rule {
                kind: Kind::of_key(&r.kind)
                    .ok_or_else(|| format!("Bilinmeyen topoloji kuralı “{}”.", r.kind))?,
                id: r.id,
                layer: r.layer,
                other: r.other,
                value: r.value,
            })
        })
        .collect()
}

fn finding_out(f: Finding) -> FindingOut {
    FindingOut {
        rule: f.rule,
        problem: f.problem,
        label: problem_label(f.problem),
        objects: f.objects,
        at: f.at,
        bounds: f.bounds,
        regions: f.regions,
        edges: f.edges,
        measure: f.measure,
        measure_kind: f.measure_kind.map(MeasureKind::key),
        fixes: f.fixes.iter().map(|k| named(k, fix_label(k))).collect(),
        exception: f.exception,
        spot: f.spot,
        target: f.target,
        subject: f.subject,
    }
}

fn finding_in(f: FindingIn) -> Result<Finding, String> {
    let problem = PROBLEMS
        .into_iter()
        .find(|p| *p == f.problem)
        .ok_or_else(|| format!("Bilinmeyen sorun “{}”.", f.problem))?;
    let fixes = f
        .fixes
        .iter()
        .filter_map(|x| FIXES.into_iter().find(|k| *k == x.key))
        .collect();
    let mut out = Finding::new(f.rule, problem, f.objects, f.at);
    out.regions = f.regions;
    out.fixes = fixes;
    out.spot = f.spot;
    out.target = f.target;
    out.subject = f.subject;
    Ok(out)
}

fn shapes(entities: &[Entity]) -> Vec<Shape> {
    entities.iter().map(|e| e.shape.clone()).collect()
}

fn check_op(
    entities: &[Entity],
    layers: &[String],
    uids: &[String],
    rules: Vec<RuleIn>,
    tolerance: f64,
    exceptions: Vec<ExceptionIn>,
) -> Result<CheckedOut, String> {
    if layers.len() != entities.len() || uids.len() != entities.len() {
        return Err("Nesnelerin katmanları ve kimlikleri eksik.".to_owned());
    }
    let rules = rules_of(rules)?;
    let shapes = shapes(entities);
    let objects = Objects {
        shapes: &shapes,
        layers,
        uids,
    };
    let exceptions: Vec<Exception> = exceptions
        .into_iter()
        .map(|x| Exception {
            rule: x.rule,
            objects: x.objects,
            at: x.at,
        })
        .collect();
    let checked = check(&objects, &rules, tolerance, &exceptions);
    Ok(CheckedOut {
        findings: checked.findings.into_iter().map(finding_out).collect(),
        looked: checked.looked,
    })
}

fn fix_op(
    entities: &[Entity],
    layers: &[String],
    rules: Vec<RuleIn>,
    finding: FindingIn,
    key: &str,
) -> Result<Vec<ChangeOut>, String> {
    if layers.len() != entities.len() {
        return Err("Nesnelerin katmanları eksik.".to_owned());
    }
    let rules = rules_of(rules)?;
    let shapes = shapes(entities);
    let uids: Vec<String> = Vec::new();
    let objects = Objects {
        shapes: &shapes,
        layers,
        uids: &uids,
    };
    let finding = finding_in(finding)?;
    Ok(fix(&objects, &rules, &finding, key)?
        .into_iter()
        .map(|c| ChangeOut {
            object: c.object,
            shape: c.shape.map(Entity::new),
        })
        .collect())
}

pub(crate) const OPS: &[Op] = &[
    op!("topologyCatalog", || CatalogOut {
        kinds: Kind::ALL
            .into_iter()
            .map(|k| KindOut {
                key: k.key(),
                label: k.label(),
                takes: k.takes(),
                between: k.between(),
                value: k.value().map(|v| match v {
                    super::ValueKind::Length => "length",
                    super::ValueKind::Angle => "angle",
                }),
                default_value: k.default_value(),
                value_label: k.value_label(),
            })
            .collect(),
        problems: PROBLEMS
            .into_iter()
            .map(|p| named(p, problem_label(p)))
            .collect(),
        fixes: FIXES.into_iter().map(|k| named(k, fix_label(k))).collect(),
    }),
    op!(
        "topologyCheck",
        |entities: Vec<Entity>,
         layers: Vec<String>,
         uids: Vec<String>,
         rules: Vec<RuleIn>,
         tolerance: f64,
         exceptions: Vec<ExceptionIn>| {
            check_op(&entities, &layers, &uids, rules, tolerance, exceptions)
        }
    ),
    op!("topologyFix", |entities: Vec<Entity>,
                        layers: Vec<String>,
                        rules: Vec<RuleIn>,
                        finding: FindingIn,
                        key: String| {
        fix_op(&entities, &layers, rules, finding, &key)
    }),
];
