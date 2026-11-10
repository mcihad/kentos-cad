//! `cad.profiles.compare` v1, the catalog's migration report (docs/adr/4940; plan/map-cad.md MAP-04): what moving the
//! drawing's data from one object catalog to another would do, said without writing anything. A class is followed by
//! its persistent id, else by its field codes; no class is guessed from a name or a place. The shared cases are
//! `fixtures/commands/v1/cad.profiles.compare.json`.
//!
//! Checks, in order: the old catalog (`unknown_profile`, `invalid_profile`), then the new one.

use std::collections::{BTreeSet, HashMap};

use kentos_commands::{
    CatalogClassChange, CatalogClassChangeKind, CatalogCodeChange, CatalogCodeChangeKind,
    CatalogDecisionChange, CatalogDecisionChangeKind, CatalogFieldChange, CatalogFieldChangeKind,
    CommandResult, ComparedCatalog, DataProfile, Entity, LayerField, MAX_COMPARE_UIDS,
    ProfileClass, ProfilesCompare, ProfilesCompared, builtin_profile, fold_name, profile_problem,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::profile_files::Shelf;
use crate::profiles::{Place, find, path_words};

/// The command whose recorded decisions the report follows (docs/adr/4140).
const RECONCILE: &str = "map.features.reconcile";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// A side's catalog: the profile itself when given, else the built-in or the project's of the name.
fn catalog(
    name: Option<&str>,
    given: Option<&DataProfile>,
    shelf: Shelf<'_>,
    side: &str,
    given_path: &str,
    which: &str,
) -> Result<DataProfile, Stop> {
    let p = match (given, name) {
        (Some(p), _) => {
            if let Some(why) = profile_problem(p) {
                return Err(fail(
                    codes::INVALID_PROFILE,
                    format!("{which} katalog geçersiz: {why}"),
                    given_path,
                ));
            }
            return Ok(p.clone());
        }
        (None, Some(n)) => match builtin_profile(n) {
            Some(p) => p,
            None => match crate::profile_files::load_profile(shelf, n) {
                Ok(Some(p)) => p,
                Ok(None) => {
                    return Err(fail(
                        codes::UNKNOWN_PROFILE,
                        format!(
                            "{which} katalog “{n}” bulunamadı: KentOS'un profilleri tucbs-temel, harita-varliklari, imar-plani; projenin profilleri Veri profili penceresinde listelenir."
                        ),
                        side,
                    ));
                }
                Err(why) => {
                    return Err(fail(
                        codes::INVALID_PROFILE,
                        format!("{which} katalog geçersiz: {why}"),
                        side,
                    ));
                }
            },
        },
        (None, None) => {
            return Err(fail(
                codes::UNKNOWN_PROFILE,
                format!("{which} kataloğu adıyla ({side}) ya da kendisiyle ({given_path}) verin."),
                side,
            ));
        }
    };
    if let Some(why) = profile_problem(&p) {
        return Err(fail(
            codes::INVALID_PROFILE,
            format!("{which} katalog geçersiz: {why}"),
            side,
        ));
    }
    Ok(p)
}

fn catalogs(doc: &Document, input: &ProfilesCompare) -> Result<(DataProfile, DataProfile), Stop> {
    let shelf = Shelf::of(doc, input.drawing.as_deref());
    let old = catalog(
        input.from.as_deref(),
        input.from_profile.as_ref(),
        shelf,
        "from",
        "fromProfile",
        "Eski",
    )?;
    let new = catalog(
        input.to.as_deref(),
        input.to_profile.as_ref(),
        shelf,
        "to",
        "toProfile",
        "Yeni",
    )?;
    Ok((old, new))
}

fn side(p: &DataProfile) -> ComparedCatalog {
    ComparedCatalog {
        name: p.name.clone(),
        title: p.title.clone(),
        release: p.release,
    }
}

fn value_of<'a>(e: &'a Entity, key: &str) -> Option<&'a str> {
    e.base()
        .attrs
        .get(key)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
}

/// A field's kind and list, as a row says it (`metin`, `metin, sebeke`).
fn described(f: &LayerField) -> String {
    let kind = f.kind.label().to_lowercase();
    match &f.domain {
        Some(d) => format!("{kind}, {d}"),
        None => kind,
    }
}

fn folded_path(path: &[String]) -> Vec<String> {
    path.iter().map(|n| fold_name(n.trim())).collect()
}

fn joined_codes(codes: &[String]) -> String {
    if codes.is_empty() {
        "yok".into()
    } else {
        codes.join(", ")
    }
}

/// A class's objects: those of its layer, in the drawing's order (persistent id, entity).
type Objects<'a> = Vec<(String, &'a Entity)>;

fn objects_of<'a>(doc: &'a Document, c: &ProfileClass) -> Objects<'a> {
    let Place::Layer(layer) = find(doc.layers().nodes(), &c.layer) else {
        return Vec::new();
    };
    doc.entities_with_uids()
        .filter(|(_, e)| e.base().layer_id == layer)
        .map(|(u, e)| (u.to_string(), e))
        .collect()
}

fn uids_of(list: &[&str]) -> Vec<String> {
    list.iter()
        .take(MAX_COMPARE_UIDS)
        .map(|u| (*u).to_owned())
        .collect()
}

/// The fields' rows of an old class that leads to one new class.
fn field_rows(
    old: &ProfileClass,
    new: &ProfileClass,
    objects: &Objects<'_>,
) -> Vec<CatalogFieldChange> {
    let holding = |name: &str| -> Vec<&str> {
        objects
            .iter()
            .filter(|(_, e)| value_of(e, name).is_some())
            .map(|(u, _)| u.as_str())
            .collect()
    };
    let lacking = |name: &str| -> Vec<&str> {
        objects
            .iter()
            .filter(|(_, e)| value_of(e, name).is_none())
            .map(|(u, _)| u.as_str())
            .collect()
    };
    let row = |f: &str,
               kind,
               was: String,
               becomes: String,
               touched: Vec<&str>,
               decide: bool,
               note: String| {
        CatalogFieldChange {
            class: old.id.clone(),
            to_class: new.id.clone(),
            field: f.to_owned(),
            kind,
            was,
            becomes,
            objects: touched.len() as u32,
            uids: uids_of(&touched),
            decide,
            note,
        }
    };
    let mut out = Vec::new();
    for f in &old.fields {
        let Some(g) = new.fields.iter().find(|g| g.name == f.name) else {
            let touched = holding(&f.name);
            let n = touched.len();
            let note = if n > 0 {
                format!("Yeni sınıfta yok; {n} nesnenin değeri taşınamaz.")
            } else {
                "Yeni sınıfta yok.".to_owned()
            };
            out.push(row(
                &f.name,
                CatalogFieldChangeKind::Removed,
                described(f),
                String::new(),
                touched,
                n > 0,
                note,
            ));
            continue;
        };
        let (kind, touched, words) = if g.kind != f.kind {
            (
                CatalogFieldChangeKind::KindChanged,
                holding(&f.name),
                ("Tür değişir", "değer yeniden okunmalı"),
            )
        } else if g.domain != f.domain {
            (
                CatalogFieldChangeKind::ListChanged,
                holding(&f.name),
                ("Değer listesi değişir", "değer yeniden okunmalı"),
            )
        } else if g.required && !f.required {
            (
                CatalogFieldChangeKind::NowRequired,
                lacking(&f.name),
                ("Zorunlu olur", "nesnede boş"),
            )
        } else {
            continue;
        };
        let n = touched.len();
        let note = if n > 0 {
            format!("{}; {n} {}.", words.0, words.1)
        } else {
            format!("{}.", words.0)
        };
        out.push(row(
            &f.name,
            kind,
            described(f),
            described(g),
            touched,
            n > 0,
            note,
        ));
    }
    for g in &new.fields {
        if old.fields.iter().any(|f| f.name == g.name) {
            continue;
        }
        let touched = lacking(&g.name);
        let n = touched.len();
        let note = match (g.required, n > 0) {
            (true, true) => format!("Yeni zorunlu alan; {n} nesnede boş."),
            (true, false) => "Yeni zorunlu alan.".to_owned(),
            (false, _) => "Yeni alan.".to_owned(),
        };
        let decide = g.required && n > 0;
        out.push(row(
            &g.name,
            CatalogFieldChangeKind::Added,
            String::new(),
            described(g),
            touched,
            decide,
            note,
        ));
    }
    out
}

/// The parts of a single-target class's change, in order: layer, geometry, codes, fields.
fn parts(old: &ProfileClass, new: &ProfileClass, fields: usize) -> Vec<String> {
    let mut out = Vec::new();
    if folded_path(&old.layer) != folded_path(&new.layer) {
        out.push(format!(
            "katman {} → {}",
            path_words(&old.layer),
            path_words(&new.layer)
        ));
    }
    if old.geometry != new.geometry {
        out.push(format!(
            "geometri {} → {}",
            old.geometry.word(),
            new.geometry.word()
        ));
    }
    if old.codes != new.codes {
        out.push(format!(
            "kodlar {} → {}",
            joined_codes(&old.codes),
            joined_codes(&new.codes)
        ));
    }
    if fields > 0 {
        out.push(format!("{fields} alan değişikliği"));
    }
    out
}

fn after_colon(parts: &[String]) -> String {
    if parts.is_empty() {
        String::new()
    } else {
        format!(": {}", parts.join("; "))
    }
}

/// The codes' rows of the two catalogs' value lists.
fn code_rows(doc: &Document, old: &DataProfile, new: &DataProfile) -> Vec<CatalogCodeChange> {
    // The objects holding each (list, code), through the old catalog's fields linked to a list.
    let mut holders: HashMap<(String, String), BTreeSet<usize>> = HashMap::new();
    let order: Vec<String> = doc
        .entities_with_uids()
        .map(|(u, _)| u.to_string())
        .collect();
    let place: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, u)| (u.as_str(), i))
        .collect();
    for c in &old.classes {
        let linked: Vec<&LayerField> = c.fields.iter().filter(|f| f.domain.is_some()).collect();
        if linked.is_empty() {
            continue;
        }
        for (u, e) in objects_of(doc, c) {
            for f in &linked {
                let Some(v) = value_of(e, &f.name) else {
                    continue;
                };
                let codes: Vec<&str> = if f.multiple {
                    v.split(';')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .collect()
                } else {
                    vec![v]
                };
                for code in codes {
                    holders
                        .entry((f.domain.clone().unwrap_or_default(), code.to_owned()))
                        .or_default()
                        .insert(place[u.as_str()]);
                }
            }
        }
    }
    let held = |domain: &str, code: &str| -> Vec<&str> {
        holders
            .get(&(domain.to_owned(), code.to_owned()))
            .map(|s| s.iter().map(|&i| order[i].as_str()).collect())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    let row = |domain: &str,
               code: &str,
               kind,
               label: &str,
               to_label: &str,
               touched: Vec<&str>,
               decide: bool,
               note: String| CatalogCodeChange {
        domain: domain.to_owned(),
        code: code.to_owned(),
        kind,
        label: label.to_owned(),
        to_label: to_label.to_owned(),
        objects: touched.len() as u32,
        uids: uids_of(&touched),
        decide,
        note,
    };
    for d in &old.domains {
        let Some(values) = &d.values else { continue };
        let nd = new.domains.iter().find(|x| x.id == d.id);
        let new_values = nd.and_then(|x| x.values.as_ref());
        for v in values {
            let found = new_values.and_then(|vs| vs.iter().find(|w| w.code == v.code));
            match found {
                None => {
                    if v.retired {
                        continue;
                    }
                    let touched = held(&d.id, &v.code);
                    let n = touched.len();
                    let head = if new_values.is_some() {
                        "Yeni listede yok"
                    } else {
                        "Liste yeni katalogda yok"
                    };
                    let note = if n > 0 {
                        format!("{head}; {n} nesnede kullanılıyor.")
                    } else {
                        format!("{head}.")
                    };
                    out.push(row(
                        &d.id,
                        &v.code,
                        CatalogCodeChangeKind::Removed,
                        &v.label,
                        "",
                        touched,
                        n > 0,
                        note,
                    ));
                }
                Some(w) if w.retired && !v.retired => {
                    let touched = held(&d.id, &v.code);
                    let n = touched.len();
                    let note = if n > 0 {
                        format!("Yeni listede kullanımdan kalkmış; {n} nesnede kullanılıyor.")
                    } else {
                        "Yeni listede kullanımdan kalkmış.".to_owned()
                    };
                    out.push(row(
                        &d.id,
                        &v.code,
                        CatalogCodeChangeKind::Retired,
                        &v.label,
                        &w.label,
                        touched,
                        n > 0,
                        note,
                    ));
                }
                Some(w) if w.label != v.label => {
                    let note = format!("Etiket “{}” → “{}”.", v.label, w.label);
                    out.push(row(
                        &d.id,
                        &v.code,
                        CatalogCodeChangeKind::Relabelled,
                        &v.label,
                        &w.label,
                        held(&d.id, &v.code),
                        false,
                        note,
                    ));
                }
                Some(_) => {}
            }
        }
        if let Some(vs) = new_values {
            for w in vs {
                if w.retired || values.iter().any(|v| v.code == w.code) {
                    continue;
                }
                out.push(row(
                    &d.id,
                    &w.code,
                    CatalogCodeChangeKind::Added,
                    &w.label,
                    &w.label,
                    Vec::new(),
                    false,
                    "Yeni kod.".into(),
                ));
            }
        }
    }
    for nd in &new.domains {
        if old.domains.iter().any(|d| d.id == nd.id) {
            continue;
        }
        let Some(vs) = &nd.values else { continue };
        for w in vs.iter().filter(|w| !w.retired) {
            out.push(row(
                &nd.id,
                &w.code,
                CatalogCodeChangeKind::Added,
                &w.label,
                &w.label,
                Vec::new(),
                false,
                "Yeni kod.".into(),
            ));
        }
    }
    out
}

pub fn report(doc: &Document, old: &DataProfile, new: &DataProfile) -> ProfilesCompared {
    // Each old class's targets: the new class of its id, else those holding one of its codes.
    let targets: Vec<Vec<usize>> = old
        .classes
        .iter()
        .map(|c| match new.classes.iter().position(|n| n.id == c.id) {
            Some(i) => vec![i],
            None => new
                .classes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.codes.iter().any(|k| c.codes.contains(k)))
                .map(|(i, _)| i)
                .collect(),
        })
        .collect();
    let by_id: Vec<bool> = old
        .classes
        .iter()
        .map(|c| new.classes.iter().any(|n| n.id == c.id))
        .collect();
    let mut classes = Vec::new();
    let mut fields = Vec::new();
    // Each old class's row's kind, its targets' ids and its removed fields, for the decisions.
    let mut kinds: HashMap<&str, (CatalogClassChangeKind, Vec<String>, Vec<String>)> =
        HashMap::new();
    let mut layer_class: HashMap<String, usize> = HashMap::new();
    for (i, c) in old.classes.iter().enumerate() {
        if let Place::Layer(l) = find(doc.layers().nodes(), &c.layer) {
            layer_class.entry(l).or_insert(i);
        }
        let objects = objects_of(doc, c);
        let ids: Vec<&str> = objects.iter().map(|(u, _)| u.as_str()).collect();
        let n = ids.len();
        let t = &targets[i];
        let to: Vec<String> = t.iter().map(|&k| new.classes[k].id.clone()).collect();
        let to_codes: Vec<String> = t
            .iter()
            .flat_map(|&k| new.classes[k].codes.clone())
            .collect();
        let (kind, to_layer, decide, note, rows) = match t.len() {
            0 => (
                CatalogClassChangeKind::Retired,
                String::new(),
                n > 0,
                "Yeni katalogda yok.".to_owned(),
                Vec::new(),
            ),
            1 => {
                let target = &new.classes[t[0]];
                let rows = field_rows(c, target, &objects);
                let p = parts(c, target, rows.len());
                let kind = if by_id[i] {
                    if p.is_empty() {
                        CatalogClassChangeKind::Same
                    } else {
                        CatalogClassChangeKind::Changed
                    }
                } else {
                    let shared = new
                        .classes
                        .iter()
                        .any(|x| x.id == target.id && old.classes.iter().any(|o| o.id == x.id))
                        || targets
                            .iter()
                            .enumerate()
                            .any(|(j, tj)| j != i && !by_id[j] && tj.contains(&t[0]));
                    if shared {
                        CatalogClassChangeKind::Merged
                    } else {
                        CatalogClassChangeKind::Renamed
                    }
                };
                let note = match kind {
                    CatalogClassChangeKind::Same => "Aynı.".to_owned(),
                    CatalogClassChangeKind::Changed => format!("Değişti: {}.", p.join("; ")),
                    CatalogClassChangeKind::Renamed => {
                        format!("“{}” olarak sürer{}.", target.id, after_colon(&p))
                    }
                    _ => format!("“{}” sınıfında birleşir{}.", target.id, after_colon(&p)),
                };
                let decide = n > 0 && c.geometry != target.geometry;
                (kind, path_words(&target.layer), decide, note, rows)
            }
            _ => {
                let moves: Vec<String> = c
                    .codes
                    .iter()
                    .map(|k| {
                        let went = t.iter().find(|&&j| new.classes[j].codes.contains(k));
                        format!(
                            "{k} → {}",
                            went.map_or("yok", |&j| new.classes[j].id.as_str())
                        )
                    })
                    .collect();
                (
                    CatalogClassChangeKind::Split,
                    String::new(),
                    n > 0,
                    format!("Kodları bölünür: {}.", moves.join(", ")),
                    Vec::new(),
                )
            }
        };
        let removed: Vec<String> = rows
            .iter()
            .filter(|r| r.kind == CatalogFieldChangeKind::Removed)
            .map(|r| r.field.clone())
            .collect();
        kinds.insert(c.id.as_str(), (kind, to.clone(), removed));
        fields.extend(rows);
        classes.push(CatalogClassChange {
            from: Some(c.id.clone()),
            to,
            kind,
            title: c.title.clone(),
            layer: path_words(&c.layer),
            to_layer,
            codes: c.codes.clone(),
            to_codes,
            objects: n as u32,
            uids: uids_of(&ids),
            decide,
            note,
        });
    }
    for (k, n) in new.classes.iter().enumerate() {
        if targets.iter().any(|t| t.contains(&k)) {
            continue;
        }
        classes.push(CatalogClassChange {
            from: None,
            to: vec![n.id.clone()],
            kind: CatalogClassChangeKind::Added,
            title: n.title.clone(),
            layer: path_words(&n.layer),
            to_layer: path_words(&n.layer),
            codes: n.codes.clone(),
            to_codes: n.codes.clone(),
            objects: 0,
            uids: Vec::new(),
            decide: false,
            note: "Yeni sınıf.".into(),
        });
    }
    let codes = code_rows(doc, old, new);
    // The recorded reconcile decisions, by number.
    let mut notes: Vec<_> = doc
        .settings()
        .review_notes
        .iter()
        .filter(|n| n.status == kentos_commands::ReviewStatus::Resolved)
        .filter_map(|n| n.decision.as_ref().map(|d| (n.number, d)))
        .filter(|(_, d)| d.command == RECONCILE && d.reject.is_none())
        .collect();
    notes.sort_by_key(|(n, _)| *n);
    let mut decisions = Vec::new();
    for (number, d) in notes {
        let class = match d
            .class
            .as_deref()
            .filter(|c| old.classes.iter().any(|x| x.id == *c))
        {
            Some(c) => Some(c.to_owned()),
            None => d
                .asset
                .as_deref()
                .and_then(|a| Uuid::parse_str(a).ok())
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| doc.get(s))
                .and_then(|e| layer_class.get(&e.base().layer_id))
                .map(|&i| old.classes[i].id.clone()),
        };
        let Some(class) = class else { continue };
        let (ck, to, removed) = &kinds[class.as_str()];
        let dropped: Vec<String> = d
            .fields
            .keys()
            .filter(|k| removed.contains(k))
            .cloned()
            .collect();
        let kind = match ck {
            CatalogClassChangeKind::Retired => CatalogDecisionChangeKind::ClassRetired,
            CatalogClassChangeKind::Split => CatalogDecisionChangeKind::ClassSplit,
            _ if !dropped.is_empty() => CatalogDecisionChangeKind::FieldDropped,
            CatalogClassChangeKind::Renamed | CatalogClassChangeKind::Merged => {
                CatalogDecisionChangeKind::Follows
            }
            _ => CatalogDecisionChangeKind::Kept,
        };
        let note = match kind {
            CatalogDecisionChangeKind::Kept => "Karar korunur.".to_owned(),
            CatalogDecisionChangeKind::Follows => {
                format!("Karar “{}” sınıfıyla sürer.", to.join(", "))
            }
            CatalogDecisionChangeKind::FieldDropped => {
                format!("Verilen alan kalkar: {}.", dropped.join(", "))
            }
            CatalogDecisionChangeKind::ClassRetired => {
                "Kararın sınıfı yeni katalogda yok.".to_owned()
            }
            CatalogDecisionChangeKind::ClassSplit => {
                "Kararın sınıfı bölünür: yeni sınıfı seçilmeli.".to_owned()
            }
        };
        let decide = !matches!(
            kind,
            CatalogDecisionChangeKind::Kept | CatalogDecisionChangeKind::Follows
        );
        decisions.push(CatalogDecisionChange {
            number,
            candidate: d.candidate.clone(),
            asset: d.asset.clone(),
            class,
            to: to.clone(),
            kind,
            fields: dropped,
            decide,
            note,
        });
    }
    let decide = classes.iter().filter(|r| r.decide).count()
        + fields.iter().filter(|r| r.decide).count()
        + codes.iter().filter(|r| r.decide).count()
        + decisions.iter().filter(|r| r.decide).count();
    ProfilesCompared {
        from: side(old),
        to: side(new),
        classes,
        fields,
        codes,
        decisions,
        decide: decide as u32,
        notes: vec![
            "Rapor çizime bir şey yazmaz: sınıflar kalıcı kimlikleriyle, yoksa saha kodlarıyla izlenir; addan ya da konumdan tahmin yapılmaz.".into(),
            "İki kataloğun karşılaştırılması mevzuata ya da TUCBS kayıtlarına uygunluk iddiası değildir.".into(),
        ],
        revision: doc.revision().to_string(),
    }
}

fn run(doc: &Document, input: &ProfilesCompare) -> CommandResult<ProfilesCompared> {
    match catalogs(doc, input) {
        Ok((old, new)) => CommandResult::Completed {
            output: report(doc, &old, &new),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesCompare) -> CommandResult<()> {
    match catalogs(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The report; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesCompare) -> CommandResult<ProfilesCompared> {
    run(cx.doc, input)
}

/// The report; reads only.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ProfilesCompare,
) -> CommandResult<ProfilesCompared> {
    run(cx.doc, &input)
}
