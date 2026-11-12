//! A road project's pay item quantities (plan/road.md Y11.4, docs/adr/0909 §4): the book's every item, its rules'
//! measures read now from the project's guardrails, culverts, walls and earthwork volumes, each source read once. Nothing
//! read is kept.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{CommandWarning, PayBookDef, PayLine, PayMeasure, PayQueried, PayRow};
use kentos_domain::Document;

use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::fail;

/// Only a road project has a pay item book.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "Poz metrajı yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

/// The project's book, or why there is none.
pub(crate) fn book(doc: &Document) -> Result<&PayBookDef, Stop> {
    doc.road_pay().ok_or_else(|| fail(codes::NO_PAY_BOOK, "Projede poz defteri yok; önce poz defterini yazın.".into(), ""))
}

/// A source as read: its kind, id, name, material, whether stale, and its measures (none when it does not read).
struct Source {
    kind: &'static str,
    word: &'static str,
    id: String,
    name: String,
    material: Option<String>,
    stale: bool,
    values: Option<Vec<(PayMeasure, f64)>>,
}

fn kind_of(m: PayMeasure) -> &'static str {
    use PayMeasure::*;
    match m {
        GuardrailLength => "guardrail",
        CulvertLength | CulvertPipes | CulvertConcrete => "culvert",
        WallLength | WallFace | WallConcrete => "wall",
        VolumeCut | VolumeFill | VolumeFactoredCut | VolumeFactoredFill => "volume",
    }
}

fn read_kind(doc: &Document, kind: &str) -> Vec<Source> {
    use PayMeasure::*;
    match kind {
        "guardrail" => doc
            .road_guardrails()
            .iter()
            .map(|g| {
                let r = crate::road_guardrails::read(doc, g).ok();
                Source {
                    kind: "guardrail",
                    word: "otokorkuluğu",
                    id: g.id.clone(),
                    name: g.name.clone(),
                    material: None,
                    stale: r.as_ref().is_some_and(|r| r.report.stale),
                    values: r.map(|r| vec![(GuardrailLength, r.report.left + r.report.right)]),
                }
            })
            .collect(),
        "culvert" => doc
            .road_culverts()
            .iter()
            .map(|c| {
                let r = crate::road_culverts::read(doc, c).ok();
                Source {
                    kind: "culvert",
                    word: "menfezi",
                    id: c.id.clone(),
                    name: c.name.clone(),
                    material: c.material.clone(),
                    stale: r.as_ref().is_some_and(|r| r.report.stale),
                    values: r.map(|r| {
                        let r = r.report;
                        vec![(CulvertLength, r.length), (CulvertPipes, r.pipes), (CulvertConcrete, r.concrete)]
                    }),
                }
            })
            .collect(),
        "wall" => doc
            .road_walls()
            .iter()
            .map(|w| {
                let r = crate::road_walls::read(doc, w).ok();
                Source {
                    kind: "wall",
                    word: "istinat duvarı",
                    id: w.id.clone(),
                    name: w.name.clone(),
                    material: w.material.clone(),
                    stale: r.as_ref().is_some_and(|r| r.report.stale),
                    values: r.map(|r| {
                        let r = r.report;
                        vec![(WallLength, r.length), (WallFace, r.face), (WallConcrete, r.concrete)]
                    }),
                }
            })
            .collect(),
        _ => doc
            .road_volumes()
            .iter()
            .map(|v| {
                let r = crate::road_volumes::report(doc, v);
                let read = r.problem.is_none();
                Source {
                    kind: "volume",
                    word: "kübajı",
                    id: v.id.clone(),
                    name: v.name.clone(),
                    material: None,
                    stale: read && r.stale,
                    values: read.then(|| {
                        vec![
                            (VolumeCut, r.cut),
                            (VolumeFill, r.fill),
                            (VolumeFactoredCut, r.factored_cut),
                            (VolumeFactoredFill, r.factored_fill),
                        ]
                    }),
                }
            })
            .collect(),
    }
}

/// The book's items now (or the one of `item`), and the warnings of the sources they read.
pub(crate) fn answer(doc: &Document, item: Option<&str>) -> Result<(PayQueried, Vec<CommandWarning>), Stop> {
    road_project(doc)?;
    let b = book(doc)?;
    let items: Vec<_> = match item {
        None => b.items.iter().collect(),
        Some(code) => vec![b.items.iter().find(|i| i.code == code).ok_or_else(|| {
            fail(codes::INVALID_PAY_BOOK, format!("“{code}” numaralı poz defterde yok."), "item")
        })?],
    };
    let mut kinds: BTreeMap<&'static str, Vec<Source>> = BTreeMap::new();
    let mut seen: BTreeSet<(&'static str, String)> = BTreeSet::new();
    let mut warnings = Vec::new();
    let mut rows = Vec::new();
    for it in items {
        let mut lines = Vec::new();
        for rule in b.rules.iter().filter(|r| r.item == it.code) {
            let kind = kind_of(rule.measure);
            let sources = kinds.entry(kind).or_insert_with(|| read_kind(doc, kind));
            for s in sources.iter().filter(|s| rule.material.is_none() || s.material == rule.material) {
                if seen.insert((s.kind, s.id.clone())) {
                    let words = match (&s.values, s.stale) {
                        (None, _) => Some((codes::PAY_SOURCE_UNREAD, format!("“{}” {} okunamıyor; metrajı yok.", s.name, s.word))),
                        (Some(_), true) => Some((
                            codes::PAY_SOURCE_STALE,
                            format!("“{}” {} koridoru değişti; metrajı eskimiş olabilir.", s.name, s.word),
                        )),
                        _ => None,
                    };
                    if let Some((code, message)) = words {
                        warnings.push(CommandWarning { code: code.into(), message, path: Some("rules".into()) });
                    }
                }
                let Some(value) = s.values.as_ref().and_then(|v| v.iter().find(|(m, _)| *m == rule.measure)).map(|x| x.1)
                else {
                    continue;
                };
                lines.push(PayLine {
                    kind: s.kind.into(),
                    id: s.id.clone(),
                    name: s.name.clone(),
                    measure: rule.measure,
                    value,
                    factor: rule.factor,
                    quantity: value * rule.factor,
                    stale: s.stale,
                });
            }
        }
        let quantity: f64 = lines.iter().map(|l| l.quantity).sum();
        rows.push(PayRow {
            code: it.code.clone(),
            name: it.name.clone(),
            unit: it.unit.clone(),
            quantity,
            price: it.price,
            amount: it.price.map(|p| p * quantity),
            lines,
        });
    }
    let total = rows.iter().filter_map(|r| r.amount).sum();
    let unpriced = rows.iter().filter(|r| r.price.is_none() && r.quantity > 0.0).count() as u32;
    Ok((
        PayQueried {
            source: b.source.clone(),
            version: b.version.clone(),
            revision: b.revision,
            currency: b.currency.clone(),
            rows,
            total,
            unpriced,
        },
        warnings,
    ))
}
