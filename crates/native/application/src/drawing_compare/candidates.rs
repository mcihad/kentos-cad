//! Bounded proposals only. Persistent identities, design data and revision counts are never changed here.

use std::collections::HashSet;

use kentos_commands::{DrawingMatchCandidate, DrawingMatchSearch, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::compare::Prepared;

use crate::checks::{Stop, error};
use crate::codes;

const MAX_SCOPE: usize = 5000;
const MAX_WORK: u64 = 2_000_000;

fn invalid(message: &str, path: &str) -> Stop {
    Stop::failed(error(
        codes::INVALID_INPUT,
        message.into(),
        Some(path.into()),
    ))
}

pub(super) fn validate(ask: &DrawingMatchSearch) -> Result<(), Stop> {
    if !ask.search.is_finite() || !(0.0..=10.0).contains(&ask.search) {
        return Err(Stop::failed(error(
            codes::INVALID_TOLERANCE,
            "Aday arama uzaklığı 0 ile 10 metre arasında sonlu bir sayı olmalı.".into(),
            Some("matchSearch.search".into()),
        )));
    }
    if ask.key.as_ref().is_some_and(|key| {
        key.trim().is_empty() || key.len() > 128 || key.chars().any(char::is_control)
    }) {
        return Err(invalid(
            "Kaynak kimliği alanı boş olmayan, en çok 128 baytlık bir öznitelik adı olmalı.",
            "matchSearch.key",
        ));
    }
    Ok(())
}

fn scoped<'a>(
    doc: &'a Document,
    slots: &[Slot],
    scope: Option<&[String]>,
    path: &str,
) -> Result<Vec<(String, &'a Entity)>, Stop> {
    let wanted = scope.map(|uids| uids.iter().cloned().collect::<HashSet<_>>());
    if scope.map_or(slots.len(), <[String]>::len) > MAX_SCOPE {
        return Err(Stop::failed(error(codes::INVALID_LIMIT,
            "Aday araması her tarafta en çok 5000 eşlenmemiş nesne inceler; oldUids/newUids kapsamını daraltın.".into(), Some(path.into()))));
    }
    if let Some(wanted) = &wanted
        && wanted.len() != scope.map_or(0, <[String]>::len)
    {
        return Err(invalid(
            "Aday kapsamındaki aynı kimliği bir kez verin.",
            path,
        ));
    }
    let out: Vec<_> = slots
        .iter()
        .filter_map(|&s| {
            let id = doc.uid(s)?.to_string();
            if wanted.as_ref().is_some_and(|w| !w.contains(&id)) {
                return None;
            }
            Some((id, doc.get(s)?))
        })
        .collect();
    if wanted.as_ref().is_some_and(|w| w.len() != out.len()) {
        return Err(invalid(
            "Aday kapsamı yalnız bu taraftaki eşlenmemiş nesnelerin UUID'lerini içerebilir.",
            path,
        ));
    }
    Ok(out)
}

/// Returns proposals and whether the output list was cut. Ambiguity is counted over the complete search,
/// including proposals beyond the output limit. A work limit refuses the search instead of hiding uncertainty.
pub(super) fn search(
    drawings: (&Document, &Document),
    unmatched: (&[Slot], &[Slot]),
    ask: &DrawingMatchSearch,
    tolerance: f64,
    limit: usize,
    mut stopped: impl FnMut() -> bool,
) -> Result<(Vec<DrawingMatchCandidate>, bool), Stop> {
    validate(ask)?;
    if stopped() {
        return Err(Stop::Cancelled);
    }
    let old = scoped(
        drawings.0,
        unmatched.0,
        ask.old_uids.as_deref(),
        "matchSearch.oldUids",
    )?;
    let new = scoped(
        drawings.1,
        unmatched.1,
        ask.new_uids.as_deref(),
        "matchSearch.newUids",
    )?;
    let old_shapes: Vec<_> = old.iter().map(|(_, e)| crate::geometry::shape(e)).collect();
    let new_shapes: Vec<_> = new.iter().map(|(_, e)| crate::geometry::shape(e)).collect();
    let old_prepared: Vec<_> = old_shapes.iter().map(Prepared::new).collect();
    let new_prepared: Vec<_> = new_shapes.iter().map(Prepared::new).collect();
    let old_complexity: Vec<_> = old_prepared.iter().map(Prepared::complexity).collect();
    let new_complexity: Vec<_> = new_prepared.iter().map(Prepared::complexity).collect();
    let old_bounds: Vec<_> = old_prepared.iter().map(Prepared::bounds).collect();
    let new_bounds: Vec<_> = new_prepared.iter().map(Prepared::bounds).collect();
    let mut old_degree = vec![0usize; old.len()];
    let mut new_degree = vec![0usize; new.len()];
    let mut kept = Vec::new();
    let mut total = 0usize;
    let mut work = 0u64;
    let key_of = |entity: &Entity| {
        ask.key
            .as_ref()
            .and_then(|key| entity.base().attrs.get(key))
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let old_keys: Vec<_> = old.iter().map(|(_, e)| key_of(e)).collect();
    let new_keys: Vec<_> = new.iter().map(|(_, e)| key_of(e)).collect();
    let too_much = || {
        Stop::failed(error(codes::TOO_MANY_PAIRS,
        "Aday araması hesap sınırına ulaştı; matchSearch.oldUids/newUids kapsamını veya arama uzaklığını daraltın. Kısmi eşleme uygulanmadı.".into(), Some("matchSearch".into())))
    };
    for (i, a) in old_prepared.iter().enumerate() {
        for (j, b) in new_prepared.iter().enumerate() {
            if stopped() {
                return Err(Stop::Cancelled);
            }
            work = work.saturating_add(1);
            if work > MAX_WORK {
                return Err(too_much());
            }
            let keyed = match (&old_keys[i], &new_keys[j]) {
                (Some(a), Some(b)) if a != b => continue,
                (Some(_), Some(_)) => true,
                _ => false,
            };
            if !keyed {
                let (ax0, ay0, ax1, ay1) = old_bounds[i];
                let (bx0, by0, bx1, by1) = new_bounds[j];
                let s = ask.search;
                if ax0 - s > bx1 || ax1 + s < bx0 || ay0 - s > by1 || ay1 + s < by0 {
                    continue;
                }
            }
            work = work.saturating_add(old_complexity[i].saturating_mul(new_complexity[j]));
            if work > MAX_WORK {
                return Err(too_much());
            }
            let (distance, same_geometry) = a.difference(b, tolerance);
            if !keyed && !distance.is_some_and(|d| d <= ask.search) {
                continue;
            }
            old_degree[i] += 1;
            new_degree[j] += 1;
            total += 1;
            if kept.len() < limit {
                kept.push((
                    i,
                    j,
                    DrawingMatchCandidate {
                        old_uid: old[i].0.clone(),
                        new_uid: new[j].0.clone(),
                        evidence: if keyed { "sourceKey" } else { "geometry" }.into(),
                        ambiguous: false,
                        same_geometry,
                        distance,
                    },
                ));
            }
        }
    }
    Ok((
        kept.into_iter()
            .map(|(i, j, mut candidate)| {
                candidate.ambiguous = old_degree[i] > 1 || new_degree[j] > 1;
                candidate
            })
            .collect(),
        total > limit,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn drawing(prefix: u16, xs: &[f64], keys: &[&str]) -> Document {
        let mut value: Value =
            serde_json::from_str(include_str!("../../../../../fixtures/kcad/v2/sheets.json"))
                .unwrap();
        value["sheets"] = Value::Null;
        value["entities"] = json!(xs.iter().enumerate().map(|(i, x)| json!({
            "kind":"line", "id":i+1, "layerId":"0", "attrs":{"handle":keys.get(i).unwrap_or(&"")},
            "a":{"x":500000.0+x,"y":4400000.0}, "b":{"x":500001.0+x,"y":4400000.0}
        })).collect::<Vec<_>>());
        value["uids"] = json!(
            (0..xs.len())
                .map(|i| format!("0192f5a0-7c3e-7d4a-9b1e-{prefix:04x}{i:08x}"))
                .collect::<Vec<_>>()
        );
        Document::from_snapshot_v2(serde_json::from_value(value).unwrap()).unwrap()
    }

    fn ask(search: f64, key: bool) -> DrawingMatchSearch {
        DrawingMatchSearch {
            search,
            key: key.then(|| "handle".into()),
            old_uids: None,
            new_uids: None,
        }
    }

    fn proposals(
        a: &Document,
        b: &Document,
        ask: &DrawingMatchSearch,
        limit: usize,
    ) -> (Vec<DrawingMatchCandidate>, bool) {
        let diff = super::super::diff(a, b, 0.001);
        search(
            (a, b),
            (&diff.removed, &diff.added),
            ask,
            0.001,
            limit,
            || false,
        )
        .unwrap_or_else(|_| panic!("proposal search failed"))
    }

    #[test]
    fn different_ids_are_only_suggestions_and_do_not_change_the_drawing_or_counts() {
        let a = drawing(1, &[0.], &[]);
        let b = drawing(2, &[0.0005], &[]);
        let before = b.to_snapshot_v2();
        let (rows, cut) = proposals(&a, &b, &ask(0.01, false), 100);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].evidence, "geometry");
        assert!(rows[0].same_geometry);
        assert!(!rows[0].ambiguous);
        assert!(!cut);
        assert!((rows[0].distance.unwrap() - 0.0005).abs() < 1e-9);
        assert_eq!(b.to_snapshot_v2(), before);
        let counts = super::super::diff(&a, &b, 0.001);
        assert_eq!(
            (
                counts.added.len(),
                counts.removed.len(),
                counts.changed.len()
            ),
            (1, 1, 0)
        );
    }

    #[test]
    fn ambiguity_counts_even_candidates_omitted_by_output_limit() {
        let a = drawing(1, &[0.], &[]);
        let b = drawing(2, &[0.0001, 0.0002], &[]);
        let (rows, cut) = proposals(&a, &b, &ask(0.01, false), 1);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].ambiguous);
        assert!(cut);
        let (rows, _) = proposals(&b, &a, &ask(0.01, false), 10);
        assert!(rows.iter().all(|c| c.ambiguous));
    }

    #[test]
    fn source_key_suggests_a_moved_object_and_unequal_keys_reject_proximity() {
        let a = drawing(1, &[0.], &["AB"]);
        let b = drawing(2, &[100.], &["AB"]);
        let (rows, _) = proposals(&a, &b, &ask(0.001, true), 10);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].evidence, "sourceKey");
        assert!(!rows[0].same_geometry);
        let different = drawing(2, &[0.], &["CD"]);
        assert!(
            proposals(&a, &different, &ask(0.001, true), 10)
                .0
                .is_empty()
        );
    }

    #[test]
    fn duplicate_source_keys_stay_ambiguous_and_empty_keys_use_geometry() {
        let a = drawing(1, &[0., 10.], &["AB", "AB"]);
        let b = drawing(2, &[100.], &["AB"]);
        let (duplicate, _) = proposals(&a, &b, &ask(0., true), 10);
        assert_eq!(duplicate.len(), 2);
        assert!(duplicate.iter().all(|c| c.ambiguous));
        let empty = drawing(1, &[0.], &[""]);
        let near = drawing(2, &[0.], &[""]);
        assert_eq!(
            proposals(&empty, &near, &ask(0., true), 10).0[0].evidence,
            "geometry"
        );
    }

    #[test]
    fn explicit_scope_is_respected_and_invalid_scope_is_refused() {
        let a = drawing(1, &[0.], &[]);
        let b = drawing(2, &[0., 0.0002], &[]);
        let mut input = ask(0.01, false);
        input.new_uids = Some(vec![b.uid(Slot(1)).unwrap().to_string()]);
        let (rows, _) = proposals(&a, &b, &input, 10);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].ambiguous);
        input.new_uids = Some(Vec::new());
        assert!(proposals(&a, &b, &input, 10).0.is_empty());
        input.new_uids = Some(vec![a.uid(Slot(1)).unwrap().to_string()]);
        let diff = super::super::diff(&a, &b, 0.001);
        assert!(
            search(
                (&a, &b),
                (&diff.removed, &diff.added),
                &input,
                0.001,
                10,
                || false
            )
            .is_err()
        );
    }

    #[test]
    fn cancellation_and_invalid_numbers_never_return_partial_suggestions() {
        let a = drawing(1, &[0.], &[]);
        let b = drawing(2, &[0.], &[]);
        let diff = super::super::diff(&a, &b, 0.001);
        assert!(matches!(
            search(
                (&a, &b),
                (&diff.removed, &diff.added),
                &ask(0.01, false),
                0.001,
                10,
                || true
            ),
            Err(Stop::Cancelled)
        ));
        for radius in [-1., f64::NAN, f64::INFINITY, 10.001] {
            assert!(validate(&ask(radius, false)).is_err());
        }
    }

    #[test]
    fn expensive_curve_alignment_is_refused_before_returning_a_partial_answer() {
        let curve = |prefix| {
            let mut snapshot = drawing(prefix, &[0.], &[]).to_snapshot_v2();
            let mut entity = json!({"kind":"polyline","id":1,"layerId":"0","attrs":{},"pts":[]});
            entity["pts"] = json!(
                (0..2000)
                    .map(|i| json!({"x":500000.+i as f64*0.001,"y":4400000.}))
                    .collect::<Vec<_>>()
            );
            snapshot.entities[0] = serde_json::from_value(entity).unwrap();
            Document::from_snapshot_v2(snapshot).unwrap()
        };
        let a = curve(1);
        let b = curve(2);
        let diff = super::super::diff(&a, &b, 0.001);
        let result = search(
            (&a, &b),
            (&diff.removed, &diff.added),
            &ask(0.01, false),
            0.001,
            10,
            || false,
        );
        assert!(matches!(result, Err(Stop::Failed(error)) if error.code == codes::TOO_MANY_PAIRS));
    }
}
