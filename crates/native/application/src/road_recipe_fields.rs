//! A recipe layer's fields in another project (plan/map-cad.md MAP-20, docs/adr/4860; `road.project.produce`): the
//! fields, the value lists they link and the subtypes a recipe keeps (`road.project.graph`), checked; given to a layer
//! Tariften üret adds, each list found here by its name with the same definition and linked, added under the first
//! free name `<name>_1`, `<name>_2` … when this project's list of that name holds other values (never merged into it),
//! else added; a layer found here (by its name or given) keeps its own fields, the recipe's it lacks or holds as another
//! kind said; the lists added for a layer no drawing went on taken away with it.

use std::collections::HashMap;

use kentos_commands::{
    FieldDomain, LayerField, RoadRecipeLayer, domain_id_of, domain_uses, domains_problem, fields_problem, link_problem,
    subtypes_problem,
};
use kentos_domain::{Document, Refusal};

use crate::checks::{Stop, error};
use crate::codes;

/// How a recipe's value list went here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Went {
    /// This project's list of its name holds the same: its fields link that one.
    Linked,
    /// Added with its name.
    Added,
    /// This project's list of its name holds other values: added under another name.
    Renamed { here: String },
}

/// A recipe list as it went here: its name in the recipe, its id here, how.
#[derive(Clone, Debug)]
pub(crate) struct Listed {
    pub(crate) name: String,
    pub(crate) id: String,
    pub(crate) went: Went,
}

/// What an added layer took of its recipe layer's fields.
#[derive(Clone, Debug, Default)]
pub(crate) struct Given {
    pub(crate) fields: usize,
    pub(crate) subtypes: bool,
    pub(crate) lists: Vec<Listed>,
}

impl Given {
    /// The names here of the lists added for it.
    pub(crate) fn added(&self) -> Vec<String> {
        self.lists
            .iter()
            .filter_map(|l| match &l.went {
                Went::Linked => None,
                Went::Added => Some(l.name.clone()),
                Went::Renamed { here } => Some(here.clone()),
            })
            .collect()
    }
}

/// Whether two lists hold the same: their kind, values or range, its ends and their unit (ids, names and descriptions
/// aside).
fn same_list(a: &FieldDomain, b: &FieldDomain) -> bool {
    a.kind == b.kind
        && a.values == b.values
        && a.min == b.min
        && a.max == b.max
        && a.min_exclusive == b.min_exclusive
        && a.max_exclusive == b.max_exclusive
        && a.unit == b.unit
}

/// What does not hold in a recipe layer's fields (docs/adr/4860), said: a field's own rules or two of a name; a bound
/// table's column (the binding is its project's, a recipe keeps none); the lists' rules; a field linking a list the
/// recipe does not define, one it cannot link, or whose values are not its list's; subtypes that do not fit the fields.
pub(crate) fn problem(def: &RoadRecipeLayer) -> Option<String> {
    if let Some(p) = fields_problem(&def.fields) {
        return Some(format!("alanlarında: {p}"));
    }
    if let Some(f) = def.fields.iter().find(|f| f.column.is_some()) {
        return Some(format!(
            "“{}” alanı bağlı tablonun sütununu taşıyor; bağ projesinindir, tarifte tutulmaz.",
            f.label()
        ));
    }
    if let Some((_, p)) = domains_problem(&def.domains) {
        return Some(format!("değer listelerinde: {p}"));
    }
    for f in &def.fields {
        let Some(id) = &f.domain else { continue };
        let Some(d) = def.domains.iter().find(|d| d.id == *id) else {
            return Some(format!("“{}” alanının bağlandığı “{id}” değer listesi tarifte yok.", f.label()));
        };
        if let Some(p) = link_problem(d, f) {
            return Some(p);
        }
        if d.applied_to(f) != *f {
            return Some(format!("“{}” alanının değerleri bağlandığı “{}” listesininkiyle aynı değil.", f.label(), d.name));
        }
    }
    let s = def.subtypes.as_ref()?;
    subtypes_problem(&def.fields, s).map(|(_, p)| format!("alt türlerinde: {p}"))
}

fn refused(def: &RoadRecipeLayer, r: Refusal) -> Stop {
    Stop::failed(error(
        codes::INVALID_PROJECT_RECIPE,
        format!("Tarifin “{}” katmanının alanları verilemedi: {}", def.name, r.0),
        Some("layers".into()),
    ))
}

/// The first free name `<name>_1`, `<name>_2` … among `taken` (case folded as the project's lists' names are).
fn free_name(name: &str, taken: &[FieldDomain]) -> String {
    (1..)
        .map(|k| format!("{name}_{k}"))
        .find(|n| !taken.iter().any(|d| d.name.to_lowercase() == n.to_lowercase()))
        .unwrap_or_else(|| name.to_owned())
}

/// Gives layer `id` (just added for `def`) the recipe layer's fields, the lists they link and its subtypes, in the
/// step named `label` (docs/adr/4860). Each list a field links: this project's of its name holding the same is linked;
/// one holding other values is left as it is and the recipe's added as `<name>_1` (the first free); none of its name,
/// the recipe's added with its id when free (else one made of its name). Refused, nothing given: what the project's
/// lists, the fields or the subtypes do not take.
pub(crate) fn give(doc: &mut Document, id: &str, def: &RoadRecipeLayer, label: &str) -> Result<Given, Stop> {
    if def.fields.is_empty() {
        return Ok(Given::default());
    }
    let before = doc.settings().domains.clone();
    let mut next = before.clone();
    let mut here: HashMap<String, String> = HashMap::new();
    let mut lists = Vec::new();
    for f in &def.fields {
        let Some(rid) = &f.domain else { continue };
        let Some(r) = def.domains.iter().find(|d| d.id == *rid) else { continue };
        if here.contains_key(rid) {
            continue;
        }
        let folded = r.name.to_lowercase();
        let (to, went) = match next.iter().find(|d| d.name.to_lowercase() == folded) {
            Some(d) if same_list(d, r) => (d.id.clone(), Went::Linked),
            Some(_) => {
                let name = free_name(&r.name, &next);
                let taken: Vec<&str> = next.iter().map(|d| d.id.as_str()).collect();
                let new_id = domain_id_of(&name, &taken);
                next.push(FieldDomain { id: new_id.clone(), name: name.clone(), ..r.clone() });
                (new_id, Went::Renamed { here: name })
            }
            None => {
                let new_id = if next.iter().any(|d| d.id == r.id) {
                    let taken: Vec<&str> = next.iter().map(|d| d.id.as_str()).collect();
                    domain_id_of(&r.name, &taken)
                } else {
                    r.id.clone()
                };
                next.push(FieldDomain { id: new_id.clone(), ..r.clone() });
                (new_id, Went::Added)
            }
        };
        here.insert(rid.clone(), to.clone());
        lists.push(Listed { name: r.name.clone(), id: to, went });
    }
    if next != before {
        doc.set_domains(next.clone(), label).map_err(|r| refused(def, r))?;
    }
    let fields: Vec<LayerField> = def
        .fields
        .iter()
        .map(|f| match f.domain.as_ref().and_then(|rid| here.get(rid)).and_then(|to| next.iter().find(|d| d.id == *to)) {
            Some(d) => d.applied_to(f),
            None => f.clone(),
        })
        .collect();
    let count = fields.len();
    doc.set_layer_fields(id, fields, &[]).map_err(|r| refused(def, r))?;
    if let Some(s) = &def.subtypes {
        doc.set_layer_subtypes(id, Some(s.clone()), label).map_err(|r| refused(def, r))?;
    }
    Ok(Given { fields: count, subtypes: def.subtypes.is_some(), lists })
}

/// What an added layer took of its recipe layer's fields, said: “Alanları (4) ve alt türleri tarifteki gibi. “Kaplama
/// türü” değer listesi de tarifteki tanımıyla eklendi.”; none without fields.
pub(crate) fn said(g: &Given) -> Option<String> {
    if g.fields == 0 {
        return None;
    }
    let mut out = format!("Alanları ({}){} tarifteki gibi.", g.fields, if g.subtypes { " ve alt türleri" } else { "" });
    for l in &g.lists {
        out.push(' ');
        out.push_str(&match &l.went {
            Went::Linked => format!("Alanları bu projenin aynı değerli “{}” listesine bağlandı.", l.name),
            Went::Added => format!("“{}” değer listesi de tarifteki tanımıyla eklendi.", l.name),
            Went::Renamed { here } => format!(
                "Bu projenin “{}” listesi başka değerler taşıyor, değişmedi; tarifinki “{here}” adıyla eklendi.",
                l.name
            ),
        });
    }
    Some(out)
}

/// A layer found here for `def` (by its name or given) keeps its own fields; when they are not the recipe's, what of
/// the recipe's it lacks and what it holds as another kind, said (docs/adr/4860); none when it holds them all as the
/// recipe does.
pub(crate) fn differences(doc: &Document, id: &str, def: &RoadRecipeLayer) -> Option<String> {
    let node = doc.layers().get(id)?;
    let mut missing = Vec::new();
    let mut other = Vec::new();
    for f in &def.fields {
        match node.fields.iter().find(|h| h.name == f.name) {
            None => missing.push(format!("“{}”", f.name)),
            Some(h) if h.kind != f.kind => {
                other.push(format!("“{}” onda {}, tarifte {}", f.name, h.kind.label(), f.kind.label()))
            }
            Some(_) => {}
        }
    }
    if missing.is_empty() && other.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if !missing.is_empty() {
        let word = if missing.len() > 1 { "alanları" } else { "alanı" };
        parts.push(format!("tarifin {} {word} onda yok", missing.join(", ")));
    }
    if !other.is_empty() {
        parts.push(other.join("; "));
    }
    Some(format!(
        "“{}” katmanı bu projede bulundu ve kendi alanlarını korur: {}. Alanları Katman özellikleri › Alanlar'da düzenleyin \
         ya da çizimin katmanını layers ile verin.",
        node.name,
        parts.join("; ")
    ))
}

/// Takes away the lists `added` (their ids here) no field links any more (their layer taken away again), in the step
/// named `label`.
pub(crate) fn remove_unused(doc: &mut Document, added: &[String], label: &str) {
    let unused: Vec<&String> = added.iter().filter(|id| domain_uses(doc.layers().nodes(), id).is_empty()).collect();
    if unused.is_empty() {
        return;
    }
    let next: Vec<FieldDomain> =
        doc.settings().domains.iter().filter(|d| !unused.contains(&&d.id)).cloned().collect();
    let _ = doc.set_domains(next, label);
}
