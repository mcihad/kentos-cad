//! `cad.topology.check` (docs/adr/0589): the project's topology rules run over the drawing, read only. The objects are
//! those of the layers the rules name; a bound layer's part (docs/adr/0510) takes the rows of its source round the
//! area (the band, docs/adr/0578) as neighbours, so a finding between a part's object and a row outside it is found; a
//! finding of the rows outside alone is not the part's and is left out. When a part's neighbours cannot all be read
//! the check says it is not complete. Each finding has its lasting key (docs/adr/0524 §1); the check's stamp is the
//! state it saw. An exception bound to its objects' geometry (schema 99) lapses once the geometry changes. A fix of
//! one finding is previewed: the objects it changes, their measures before and after, the bound sources it touches.
//! The desktop's Topoloji tab gathers and checks through [`gather`] and [`run`].

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{
    CommandResult, CommandWarning, SourceLink, TOPOLOGY_CHECK_LIMIT, TOPOLOGY_CHECK_LIMIT_MAX,
    TopologyCheck, TopologyChecked, TopologyException, TopologyFinding, TopologyFixChange,
    TopologyFixPreview, TopologyFixSource, TopologyGap, TopologySourceStamp, TopologyStamp, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::{Shape, entity_area, entity_bounds, entity_length};
use kentos_geometry_core::ops::topology_rules::{
    self as core, Checked, Exception, Finding, Kind, Objects, Rule, fix, incremental,
};
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geometry::{edit_geometry, shape};

/// The objects a check looks at: the drawing's (with their slots) first, then the rows read round bound parts.
#[derive(Clone, Debug, Default)]
pub struct Gathered {
    pub shapes: Vec<Shape>,
    pub layers: Vec<String>,
    pub uids: Vec<String>,
    /// Each object's slot in the drawing; none for a row outside a part.
    pub slots: Vec<Option<Slot>>,
    /// The rules checked, as the project names them and as the core takes them.
    pub rules: Vec<kentos_commands::TopologyRule>,
    pub core_rules: Vec<Rule>,
    pub tolerance: f64,
    /// The exceptions that hold, and those whose objects' geometry changed since they were marked.
    pub exceptions: Vec<Exception>,
    pub lapsed: Vec<Exception>,
    /// Rules whose layer or other layer the project does not have.
    pub missing: usize,
    /// How many of the objects are the drawing's.
    pub drawn: usize,
    pub incomplete: Vec<TopologyGap>,
    pub stamp: TopologyStamp,
}

impl Gathered {
    pub fn objects(&self) -> Objects<'_> {
        Objects {
            shapes: &self.shapes,
            layers: &self.layers,
            uids: &self.uids,
        }
    }

    /// A finding's lasting key (docs/adr/0524 §1).
    pub fn key(&self, f: &Finding) -> String {
        let uids: Vec<&str> = f
            .objects
            .iter()
            .map(|&k| self.uids.get(k).map_or("", String::as_str))
            .collect();
        let rule = self.core_rules.get(f.rule).map_or("", |r| r.id.as_str());
        incremental::finding_key(rule, &uids, f.at, self.tolerance)
    }

    /// Whether the object is a row outside a part.
    pub fn outside(&self, k: usize) -> bool {
        self.slots.get(k).is_some_and(Option::is_none)
    }

    /// The digest of the objects' geometry (docs/adr/0589 §3), in the finding's order.
    pub fn digest(&self, objects: &[usize]) -> String {
        digest_of(objects.iter().filter_map(|&k| self.shapes.get(k)))
    }

    /// Whether `f` is the finding a lapsed exception was marked on: its rule, its objects, its place.
    pub fn lapsed(&self, f: &Finding) -> bool {
        let rule = self.core_rules.get(f.rule).map_or("", |r| r.id.as_str());
        self.lapsed.iter().any(|x| self.same(x, rule, f))
    }

    /// Whether the exception `x` names `f`.
    pub fn same(&self, x: &Exception, rule: &str, f: &Finding) -> bool {
        x.rule == rule
            && x.objects.len() == f.objects.len()
            && x.objects
                .iter()
                .zip(&f.objects)
                .all(|(u, &k)| self.uids.get(k).is_some_and(|v| v == u))
            && (x.at.x - f.at.x).hypot(x.at.y - f.at.y) <= self.tolerance
    }
}

/// The digest of shapes' geometry: the SHA-256 of their canonical JSON one after another, its first 8 bytes in
/// lowercase hexadecimal.
pub fn digest_of<'a>(shapes: impl Iterator<Item = &'a Shape>) -> String {
    let mut h = Sha256::new();
    for s in shapes {
        let text = edit_geometry(s.clone())
            .and_then(|g| serde_json::to_value(g).ok())
            .map(|v| kentos_commands::canonical::canonical_json(&v))
            .unwrap_or_default();
        h.update(text.as_bytes());
        h.update([0u8]);
    }
    h.finalize()[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn sixteen(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// The project's rules as the core takes them.
pub fn core_rules(rules: &[kentos_commands::TopologyRule]) -> Vec<Rule> {
    rules
        .iter()
        .filter_map(|r| {
            Some(Rule {
                id: r.id.clone(),
                kind: Kind::of_key(r.kind.key())?,
                layer: r.layer.clone(),
                other: r.other.clone(),
                value: r.value,
            })
        })
        .collect()
}

/// The objects of the layers the rules (`only`: those of these ids; none: all) name, the rows round their bound
/// parts, the exceptions that hold and those that lapsed, and the state seen.
pub fn gather(doc: &Document, only: Option<&[String]>) -> Gathered {
    let settings = doc.settings().topology.clone().unwrap_or_default();
    let rules: Vec<kentos_commands::TopologyRule> = settings
        .rules
        .iter()
        .filter(|r| only.is_none_or(|ids| ids.contains(&r.id)))
        .cloned()
        .collect();
    let leaves: Vec<String> = doc.layers().leaves().iter().map(|n| n.id.clone()).collect();
    let missing = rules
        .iter()
        .filter(|r| {
            !leaves.contains(&r.layer) || r.other.as_ref().is_some_and(|o| !leaves.contains(o))
        })
        .count();
    let mut named: Vec<&str> = Vec::new();
    for r in &rules {
        for l in std::iter::once(r.layer.as_str()).chain(r.other.as_deref()) {
            if !named.contains(&l) {
                named.push(l);
            }
        }
    }
    let mut g = Gathered {
        tolerance: settings.tolerance(),
        core_rules: core_rules(&rules),
        missing,
        ..Gathered::default()
    };
    for (uid, e) in doc.entities_with_uids() {
        if named.contains(&e.base().layer_id.as_str()) {
            g.shapes.push(shape(e));
            g.layers.push(e.base().layer_id.clone());
            g.uids.push(uid.to_string());
            g.slots.push(Some(Slot(e.base().id)));
        }
    }
    g.drawn = g.shapes.len();
    // The bound layers the rules name, in the rules' order of their layers.
    let mut sources = Vec::new();
    for layer in &named {
        let Some(link) = doc.settings().sources.iter().find(|s| s.layer == *layer) else {
            continue;
        };
        let band = band_of(doc, link, &mut g);
        sources.push(TopologySourceStamp {
            layer: link.layer.clone(),
            revision: link.revision.clone(),
            band,
        });
    }
    // The exceptions: one bound to its objects' geometry holds while that geometry is the same.
    let place: HashMap<&str, usize> = g
        .uids
        .iter()
        .enumerate()
        .map(|(k, u)| (u.as_str(), k))
        .collect();
    let mut exceptions = Vec::new();
    let mut lapsed = Vec::new();
    for x in &settings.exceptions {
        let core = Exception {
            rule: x.rule.clone(),
            objects: x.objects.iter().map(|id| id.to_text()).collect(),
            at: kentos_geometry_core::vec2::Vec2::new(x.at.x, x.at.y),
        };
        let now: Option<Vec<usize>> = core
            .objects
            .iter()
            .map(|u| place.get(u.as_str()).copied())
            .collect();
        match (&x.digest, now) {
            (Some(d), Some(objects)) if *d != g.digest(&objects) => lapsed.push(core),
            _ => exceptions.push(core),
        }
    }
    g.exceptions = exceptions;
    g.lapsed = lapsed;
    let rules_digest = {
        let v = serde_json::json!({
            "rules": rules,
            "tolerance": g.tolerance,
            "exceptions": settings.exceptions,
        });
        sixteen(kentos_commands::canonical::canonical_json(&v).as_bytes())
    };
    g.rules = rules;
    g.stamp = TopologyStamp {
        revision: doc.revision().to_string(),
        rules: rules_digest,
        sources,
    };
    g
}

/// The rows round `link`'s part taken as neighbours; the digest of what was read; a gap when they could not all be.
fn band_of(doc: &Document, link: &SourceLink, g: &mut Gathered) -> Option<String> {
    let extent = link.extent?;
    let band = crate::sources::band_of(&extent);
    // A part's object reaching out of the band has neighbours no read gave.
    let beyond = g
        .shapes
        .iter()
        .zip(&g.layers)
        .filter(|(s, l)| {
            *l == &link.layer && {
                let b = entity_bounds(s);
                b.min_x < band[0] || b.min_y < band[1] || b.max_x > band[2] || b.max_y > band[3]
            }
        })
        .count();
    if beyond > 0 {
        g.incomplete.push(TopologyGap {
            layer: link.layer.clone(),
            reason: format!(
                "parçanın {beyond} nesnesi okunan bandın dışına uzanıyor; oradaki komşuları okunmadı. Alanı genişletin."
            ),
        });
    }
    match crate::sources::band_rows(doc, link)? {
        Ok(rows) => {
            let mut sorted: Vec<(String, String)> = rows
                .iter()
                .map(|(u, e)| (u.to_string(), digest_of(std::iter::once(&shape(e)))))
                .collect();
            sorted.sort();
            let digest = sixteen(
                sorted
                    .iter()
                    .map(|(u, d)| format!("{u}:{d};"))
                    .collect::<String>()
                    .as_bytes(),
            );
            for (uid, e) in rows {
                g.shapes.push(shape(&e));
                g.layers.push(link.layer.clone());
                g.uids.push(uid.to_string());
                g.slots.push(None);
            }
            Some(digest)
        }
        Err(why) => {
            g.incomplete.push(TopologyGap {
                layer: link.layer.clone(),
                reason: format!("alanın dışındaki komşular okunamadı: {why}."),
            });
            None
        }
    }
}

/// The rules over what was gathered; the findings of rows outside the parts alone left out.
pub fn run(g: &Gathered) -> Checked {
    let mut found = core::check(&g.objects(), &g.core_rules, g.tolerance, &g.exceptions);
    found
        .findings
        .retain(|f| f.objects.is_empty() || f.objects.iter().any(|&k| !g.outside(k)));
    found
}

fn count(n: usize) -> u32 {
    n.min(u32::MAX as usize) as u32
}

/// A finding as the command says it.
pub fn finding_out(g: &Gathered, f: &Finding) -> TopologyFinding {
    let rule = g.core_rules.get(f.rule);
    TopologyFinding {
        key: g.key(f),
        rule: rule.map_or_else(String::new, |r| r.id.clone()),
        kind: rule.map_or_else(String::new, |r| r.kind.key().to_owned()),
        problem: f.problem.to_owned(),
        uids: f
            .objects
            .iter()
            .filter_map(|&k| g.uids.get(k).cloned())
            .collect(),
        outside: f
            .objects
            .iter()
            .filter(|&&k| g.outside(k))
            .filter_map(|&k| g.uids.get(k).cloned())
            .collect(),
        at: Vec2 {
            x: f.at.x,
            y: f.at.y,
        },
        measure: f.measure,
        measure_kind: f.measure_kind.map(|m| m.key().to_owned()),
        fixes: f.fixes.iter().map(|s| (*s).to_owned()).collect(),
        exception: f.exception,
        lapsed: !f.exception && g.lapsed(f),
    }
}

/// What a fix of `f` would do; why it cannot.
pub fn preview(
    doc: &Document,
    g: &Gathered,
    f: &Finding,
    key: &str,
) -> Result<TopologyFixPreview, String> {
    let Some(fix_key) = f.fixes.iter().copied().find(|k| *k == key) else {
        return Err(format!(
            "Bu bulgu “{key}” düzeltmesini sunmuyor; sunduğu düzeltmeler: {}.",
            if f.fixes.is_empty() {
                "yok".to_owned()
            } else {
                f.fixes.join(", ")
            }
        ));
    };
    let changes = fix(&g.objects(), &g.core_rules, f, fix_key)?;
    if changes.iter().any(|c| g.outside(c.object)) {
        return Err(
            "Bu düzeltme alanın dışındaki bir satırı değiştirir; alanı genişletip satırı çalışma kopyasına alın."
                .to_owned(),
        );
    }
    let mut out = Vec::with_capacity(changes.len());
    let mut by_layer: BTreeMap<String, u32> = BTreeMap::new();
    for c in &changes {
        let (Some(uid), Some(layer), Some(before)) = (
            g.uids.get(c.object),
            g.layers.get(c.object),
            g.shapes.get(c.object),
        ) else {
            continue;
        };
        *by_layer.entry(layer.clone()).or_default() += 1;
        let area = entity_area(before);
        let (area_after, length_after) = match &c.shape {
            Some(s) => (entity_area(s), entity_length(s)),
            None => (None, None),
        };
        out.push(TopologyFixChange {
            uid: uid.clone(),
            layer: layer.clone(),
            removed: c.shape.is_none(),
            area_before: area,
            area_after: area.and(area_after),
            length_before: if area.is_none() {
                entity_length(before)
            } else {
                None
            },
            length_after: if area.is_none() { length_after } else { None },
        });
    }
    let sources = by_layer
        .into_iter()
        .filter_map(|(layer, rows)| {
            let link = doc.settings().sources.iter().find(|s| s.layer == layer)?;
            Some(TopologyFixSource {
                layer,
                table: link.table.clone(),
                rows,
            })
        })
        .collect();
    Ok(TopologyFixPreview {
        key: String::new(),
        fix: fix_key.to_owned(),
        changes: out,
        sources,
    })
}

/// What the command answers; why not.
fn check_in(
    doc: &Document,
    input: &TopologyCheck,
) -> Result<(TopologyChecked, Vec<CommandWarning>), Stop> {
    let limit = input.limit.unwrap_or(TOPOLOGY_CHECK_LIMIT);
    if limit == 0 || limit > TOPOLOGY_CHECK_LIMIT_MAX {
        return Err(Stop::failed(error(
            codes::INVALID_LIMIT,
            format!("Liste sınırı 1 ile {TOPOLOGY_CHECK_LIMIT_MAX} arasında bir tam sayı olmalı."),
            Some("limit".into()),
        )));
    }
    let known: Vec<&str> = doc
        .settings()
        .topology
        .as_ref()
        .map(|t| t.rules.iter().map(|r| r.id.as_str()).collect())
        .unwrap_or_default();
    if let Some(ids) = &input.rules {
        for (i, id) in ids.iter().enumerate() {
            if !known.contains(&id.as_str()) {
                return Err(Stop::failed(error(
                    codes::TOPOLOGY_RULE_NOT_FOUND,
                    format!("Projede “{id}” kimlikli topoloji kuralı yok."),
                    Some(format!("rules[{i}]")),
                )));
            }
        }
    }
    let g = gather(doc, input.rules.as_deref());
    let found = run(&g);
    let preview = match &input.preview {
        None => None,
        Some(ask) => {
            let Some(f) = found.findings.iter().find(|f| g.key(f) == ask.key) else {
                return Err(Stop::failed(error(
                    codes::FINDING_NOT_FOUND,
                    format!(
                        "Çizimin bugünkü denetiminde “{}” anahtarlı bulgu yok; önce cad.topology.check ile denetleyin.",
                        ask.key
                    ),
                    Some("preview.key".into()),
                )));
            };
            let mut p = preview(doc, &g, f, &ask.fix).map_err(|why| {
                Stop::failed(error(
                    codes::FIX_UNAVAILABLE,
                    why,
                    Some("preview.fix".into()),
                ))
            })?;
            p.key = ask.key.clone();
            Some(p)
        }
    };
    let findings: Vec<TopologyFinding> =
        found.findings.iter().map(|f| finding_out(&g, f)).collect();
    let exceptions = findings.iter().filter(|f| f.exception).count();
    let lapsed = findings.iter().filter(|f| f.lapsed).count();
    let total = findings.len();
    let neighbours = g.shapes.len() - g.drawn;
    let mut warnings: Vec<CommandWarning> = g
        .incomplete
        .iter()
        .map(|gap| CommandWarning {
            code: codes::TOPOLOGY_INCOMPLETE.into(),
            message: format!(
                "“{}” katmanının denetimi eksik: {} Katman temiz sayılmaz.",
                doc.layers()
                    .get(&gap.layer)
                    .map_or(gap.layer.as_str(), |n| n.name.as_str()),
                gap.reason
            ),
            path: None,
        })
        .collect();
    if lapsed > 0 {
        warnings.push(CommandWarning {
            code: codes::EXCEPTION_LAPSED.into(),
            message: format!(
                "{lapsed} istisnanın nesnelerinin geometrisi istisna yapıldıktan sonra değişti; bulguları yeniden açık. Gözden geçirip yeniden istisna yapın ya da düzeltin."
            ),
            path: None,
        });
    }
    Ok((
        TopologyChecked {
            findings: findings.into_iter().take(limit as usize).collect(),
            total: count(total),
            open: count(total - exceptions),
            exceptions: count(exceptions),
            lapsed: count(lapsed),
            rules: count(g.rules.len()),
            missing: count(g.missing),
            objects: count(g.drawn),
            neighbours: count(neighbours),
            complete: g.incomplete.is_empty(),
            incomplete: g.incomplete.clone(),
            stamp: g.stamp.clone(),
            preview,
        },
        warnings,
    ))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TopologyCheck) -> CommandResult<()> {
    match check_in(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &TopologyCheck) -> CommandResult<TopologyChecked> {
    match check_in(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: TopologyCheck,
) -> CommandResult<TopologyChecked> {
    plan(cx, &input)
}

/// The exception a finding is marked with: its rule, its objects, its place and its objects' geometry.
pub fn exception_of(g: &Gathered, f: &Finding) -> Option<TopologyException> {
    let rule = g.core_rules.get(f.rule)?.id.clone();
    let objects = f
        .objects
        .iter()
        .map(|&k| {
            g.uids
                .get(k)
                .and_then(|u| kentos_commands::EntityId::parse(u))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(TopologyException {
        rule,
        objects,
        at: Vec2 {
            x: f.at.x,
            y: f.at.y,
        },
        digest: Some(g.digest(&f.objects)),
    })
}
