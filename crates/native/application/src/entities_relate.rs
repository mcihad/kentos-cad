//! `cad.entities.relate` v1 (docs/adr/0501): how each object of `a` stands to each of `b` by the standard DE-9IM
//! profile (`ops::de9im`), exact up to the floating point, with a predicate, a pattern or every pair's matrix. Objects
//! are read as a query reads them (`spatial_query::geometry_of`); pairs whose boxes are apart get their matrix without
//! noding. It writes nothing. The shared cases are `fixtures/commands/v1/cad.entities.relate.json`
//! (`scripts/fixtures/relate_cases.py`, GEOS through QGIS).

use std::collections::HashSet;

use kentos_contracts::{
    CommandResult, EntitiesRelate, EntitiesRelated, MAX_RELATE_PAIRS, RelatePredicate, RelatedPair,
};
use kentos_domain::Document;
use kentos_domain::Uuid;
use kentos_geometry_core::ops::de9im::geometry_of_standard;
use kentos_geometry_core::ops::de9im::{self, Matrix, Predicate};
use kentos_geometry_core::ops::spatial_query::Geometry;

use crate::ExecutionContext;
use crate::blocks::core_entity;
use crate::checks::{Stop, error};
use crate::codes;

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesRelate) -> CommandResult<()> {
    match read(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The pairs; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesRelate) -> CommandResult<EntitiesRelated> {
    answer(cx.doc, input)
}

/// The pairs. Writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesRelate,
) -> CommandResult<EntitiesRelated> {
    answer(cx.doc, &input)
}

fn predicate(p: RelatePredicate) -> Predicate {
    match p {
        RelatePredicate::Intersects => Predicate::Intersects,
        RelatePredicate::Disjoint => Predicate::Disjoint,
        RelatePredicate::Contains => Predicate::Contains,
        RelatePredicate::Within => Predicate::Within,
        RelatePredicate::Covers => Predicate::Covers,
        RelatePredicate::CoveredBy => Predicate::CoveredBy,
        RelatePredicate::Touches => Predicate::Touches,
        RelatePredicate::Crosses => Predicate::Crosses,
        RelatePredicate::Overlaps => Predicate::Overlaps,
        RelatePredicate::Equals => Predicate::Equals,
    }
}

/// An object as the profile reads it: its id and geometry, none for one it does not read.
type Read = (String, Option<Geometry>);

/// The objects of `a` and `b`, each once in the input's order.
fn read(doc: &Document, input: &EntitiesRelate) -> Result<(Vec<Read>, Vec<Read>), Stop> {
    for (list, name) in [(&input.a, "a"), (&input.b, "b")] {
        if list.is_empty() {
            return Err(Stop::failed(error(
                codes::NO_OBJECTS,
                format!(
                    "{name} listesi boş: ilişkisi sorulacak nesnelerin kalıcı kimliklerini verin."
                ),
                Some(name.into()),
            )));
        }
    }
    if input.predicate.is_some() && input.pattern.is_some() {
        return Err(Stop::failed(error(
            codes::INVALID_INPUT,
            "Hem yüklem hem desen verildi: birini verin (predicate ya da pattern) ya da hiçbirini (bütün çiftlerin matrisleri).".into(),
            Some("pattern".into()),
        )));
    }
    if let Some(p) = &input.pattern
        && Matrix([0; 9]).matches(p).is_none()
    {
        return Err(Stop::failed(error(
            codes::INVALID_PATTERN,
            format!(
                "“{p}” bir DE-9IM deseni değil: dokuz harf, her biri T, F, *, 0, 1 ya da 2 olmalı (içerme T*****FF* gibi)."
            ),
            Some("pattern".into()),
        )));
    }
    let objects = |list: &[String], name: &str| -> Result<Vec<Read>, Stop> {
        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(list.len());
        for (i, uid) in list.iter().enumerate() {
            if !seen.insert(uid.as_str()) {
                continue;
            }
            let found = Uuid::parse_str(uid)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|slot| doc.get(slot));
            let Some(e) = found else {
                return Err(Stop::failed(error(
                    codes::ENTITY_NOT_FOUND,
                    format!(
                        "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                    ),
                    Some(format!("{name}[{i}]")),
                )));
            };
            out.push((uid.clone(), geometry_of_standard(&core_entity(e).shape)));
        }
        Ok(out)
    };
    let (a, b) = (objects(&input.a, "a")?, objects(&input.b, "b")?);
    if a.len().saturating_mul(b.len()) > MAX_RELATE_PAIRS {
        return Err(Stop::failed(error(
            codes::TOO_MANY_PAIRS,
            format!(
                "{} × {} çift olurdu, en çok 1 000 000: nesneleri daha küçük kümelere bölün.",
                a.len(),
                b.len()
            ),
            Some("b".into()),
        )));
    }
    Ok((a, b))
}

/// [`execute`] over the document.
pub fn answer(doc: &Document, input: &EntitiesRelate) -> CommandResult<EntitiesRelated> {
    let (a, b) = match read(doc, input) {
        Ok(v) => v,
        Err(stop) => return stop.into(),
    };
    let mut unread = Vec::new();
    for (uid, g) in a.iter().chain(&b) {
        if g.is_none() && !unread.contains(uid) {
            unread.push(uid.clone());
        }
    }
    let want = input.predicate.map(predicate);
    let mut pairs = Vec::new();
    for (ua, ga) in &a {
        let Some(ga) = ga else { continue };
        for (ub, gb) in &b {
            let Some(gb) = gb else { continue };
            let m = de9im::relate_fast(ga, gb);
            let keep = match (&want, &input.pattern) {
                (Some(p), _) => p.holds(&m, de9im::dimension(ga), de9im::dimension(gb)),
                (None, Some(pattern)) => m.matches(pattern).unwrap_or(false),
                (None, None) => true,
            };
            if keep {
                pairs.push(RelatedPair {
                    a: ua.clone(),
                    b: ub.clone(),
                    matrix: m.text(),
                });
            }
        }
    }
    CommandResult::Completed {
        output: EntitiesRelated {
            pairs,
            unread,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
