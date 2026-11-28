//! The checks after the second pass (docs/adr/8061 §11.1): cycles in the spatial tree and in the placement chains, too
//! deep a placement chain, and GlobalIds missing, malformed or said twice; each a finding with its line.

use std::collections::HashMap;

use super::graph::Counted;
use super::{Finding, Limits, Object, Severity};

/// Cycles in the tree (an object above itself through its parents and containers): one finding a cycle.
pub(super) fn cycles(
    objects: &[Object],
    line_of: &HashMap<u32, usize>,
    findings: &mut Vec<Finding>,
) {
    let up = |i: usize| objects[i].parent.or(objects[i].container);
    for members in functional_cycles(objects.len(), up) {
        let first = objects[members[0]].step;
        let steps: Vec<String> = members
            .iter()
            .map(|&i| format!("#{}", objects[i].step))
            .collect();
        findings.push(Finding {
            code: "tree_cycle",
            severity: Severity::Error,
            step: Some(first),
            line: line_of.get(&first).copied(),
            message: format!(
                "Yapı ağacında döngü var: {} birbirinin içinde. Döngüdeki nesneler ağacın kökünde gösterilmez; dosyayı yazan programla düzeltin.",
                steps.join(", ")
            ),
        });
    }
}

/// Cycles and too deep chains in the placements (`PlacementRelTo`).
pub(super) fn placement_cycles(
    placements: &HashMap<u32, u32>,
    limits: &Limits,
    line_of: &HashMap<u32, usize>,
    findings: &mut Vec<Finding>,
) {
    let ids: Vec<u32> = {
        let mut v: Vec<u32> = placements.keys().copied().collect();
        v.sort_unstable();
        v
    };
    let at: HashMap<u32, usize> = ids.iter().enumerate().map(|(i, s)| (*s, i)).collect();
    let up = |i: usize| placements.get(&ids[i]).and_then(|to| at.get(to)).copied();
    for members in functional_cycles(ids.len(), up) {
        let first = ids[members[0]];
        findings.push(Finding {
            code: "placement_cycle",
            severity: Severity::Error,
            step: Some(first),
            line: line_of.get(&first).copied(),
            message: format!(
                "#{first} yerleşimi kendine göre konmuş (PlacementRelTo zinciri döngü yapıyor, {} kayıt); bu yerleşimdeki nesnelerin yeri bilinemez.",
                members.len()
            ),
        });
    }
    // Depths, each chain walked once (memoised); a chain in a cycle has been said already.
    let mut depth: Vec<Option<usize>> = vec![None; ids.len()];
    for (start, &s) in ids.iter().enumerate() {
        let mut path = Vec::new();
        let mut i = Some(start);
        while let Some(k) = i {
            if depth[k].is_some() || path.len() > limits.depth || path.contains(&k) {
                break;
            }
            path.push(k);
            i = up(k);
        }
        let mut d = i.and_then(|k| depth[k]).unwrap_or(0);
        for &k in path.iter().rev() {
            d += 1;
            depth[k] = Some(d);
        }
        if d > limits.depth {
            findings.push(Finding {
                code: "placement_too_deep",
                severity: Severity::Warning,
                step: Some(s),
                line: line_of.get(&s).copied(),
                message: format!(
                    "#{s} yerleşiminin zinciri {} kattan derin; olağan dışı bir model.",
                    limits.depth
                ),
            });
            break;
        }
    }
}

/// The cycles of a graph where each node has at most one successor, in O(n): each cycle's members, in order.
fn functional_cycles(n: usize, up: impl Fn(usize) -> Option<usize>) -> Vec<Vec<usize>> {
    // 0: not seen, 1: on the walk now, 2: done.
    let mut state = vec![0u8; n];
    let mut out = Vec::new();
    for start in 0..n {
        if state[start] != 0 {
            continue;
        }
        let mut path = Vec::new();
        let mut i = Some(start);
        while let Some(k) = i {
            match state[k] {
                0 => {
                    state[k] = 1;
                    path.push(k);
                    i = up(k);
                }
                1 => {
                    if let Some(from) = path.iter().position(|&p| p == k) {
                        out.push(path[from..].to_vec());
                    }
                    break;
                }
                _ => break,
            }
        }
        for k in path {
            state[k] = 2;
        }
    }
    out
}

/// GlobalIds missing, malformed (22 characters of IFC's base 64, the first 0–3) or said twice.
pub(super) fn global_ids(
    objects: &[Object],
    line_of: &HashMap<u32, usize>,
    findings: &mut Vec<Finding>,
) {
    let mut seen: HashMap<&str, u32> = HashMap::new();
    let mut said = Counted::default();
    for o in objects {
        let line = line_of.get(&o.step).copied();
        let g = o.global_id.as_str();
        let (code, message) = if g.is_empty() {
            (
                "global_id_missing",
                format!("{} #{} kaydının GlobalId'si yok.", o.class, o.step),
            )
        } else if !valid_global_id(g) {
            (
                "global_id_invalid",
                format!(
                    "{} #{} kaydının GlobalId'si “{g}” biçimsiz (22 karakter, IFC'nin 64'lük alfabesi).",
                    o.class, o.step
                ),
            )
        } else if let Some(first) = seen.insert(g, o.step) {
            (
                "global_id_duplicate",
                format!(
                    "“{g}” GlobalId'si iki nesnede: #{first} ve #{}. Kimlik her nesnede tek olmalı.",
                    o.step
                ),
            )
        } else {
            continue;
        };
        said.add(findings, || Finding {
            code,
            severity: Severity::Error,
            step: Some(o.step),
            line,
            message,
        });
    }
    said.close(
        findings,
        "global_id_more",
        "nesnenin GlobalId'sinde sorun var",
    );
}

/// Whether `g` is a GlobalId as IFC writes it: 22 characters of `0-9A-Za-z_$`, the first 0–3 (128 bits).
pub(crate) fn valid_global_id(g: &str) -> bool {
    g.len() == 22
        && g.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'$')
        && matches!(g.as_bytes()[0], b'0'..=b'3')
}
