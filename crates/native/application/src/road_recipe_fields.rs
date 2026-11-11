//! A recipe layer's fields in another project (plan/map-cad.md MAP-20, docs/adr/4860; `road.project.produce`): the
//! fields, the value lists they link and the subtypes a recipe keeps (`road.project.graph`), checked; given to a layer
//! Tariften üret adds, each list found here by its name with the same definition and linked, added under the first
//! free name `<name>_1`, `<name>_2` … when this project's list of that name holds other values (never merged into it),
//! else added; a layer found here (by its name or given) keeps its own fields, the recipe's it lacks or holds as another
//! kind said; the lists added for a layer no drawing went on taken away with it.
//!
//! Docs/adr/5240: its value combinations, record rules, indexes and forms (each form's draft, bound to the fields by
//! their keys) are checked against its fields and given too; a form's id taken here gets the first free `<id>-2`,
//! `<id>-3` …. With `addFields` a layer found here takes the recipe's fields it lacks, after its own, the lists they
//! link going as an added layer's do; nothing else of it changes.

use std::collections::HashMap;

use kentos_commands::{
    FieldDomain, LayerField, ProjectForm, RoadRecipeLayer, body_problem, contingents_problem, domain_id_of, domain_uses,
    domains_problem, fields_problem, form_id_ok, form_name_ok, layer_indexes_problem, layer_rules_problem, link_problem,
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
    /// Docs/adr/5240: its value combinations, record rules and indexes, and the forms added (id here, name).
    pub(crate) contingents: usize,
    pub(crate) rules: usize,
    pub(crate) indexes: usize,
    pub(crate) forms: Vec<(String, String)>,
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
    if let Some(s) = def.subtypes.as_ref()
        && let Some((_, p)) = subtypes_problem(&def.fields, s)
    {
        return Some(format!("alt türlerinde: {p}"));
    }
    // Docs/adr/5240: its value combinations, record rules, indexes and forms against its fields.
    if let Some((_, p)) = contingents_problem(&def.fields, def.subtypes.as_ref(), &def.contingents) {
        return Some(format!("değer bileşimlerinde: {p}"));
    }
    if let Some((_, p)) = layer_rules_problem(&def.fields, &def.rules) {
        return Some(format!("kayıt kurallarında: {p}"));
    }
    if let Some((_, p)) = layer_indexes_problem(&def.fields, &def.indexes) {
        return Some(format!("dizinlerinde: {p}"));
    }
    let keys: Vec<&str> = def.fields.iter().map(LayerField::key).collect();
    for f in &def.forms {
        if !form_id_ok(&f.id) || !form_name_ok(&f.name) {
            return Some(format!("“{}” formunun kimliği ya da adı geçersiz.", f.name));
        }
        if let Some(p) = body_problem(&f.draft) {
            return Some(format!("“{}” formunda: {p}", f.name));
        }
        let form = as_form(f, &def.id);
        if let Some(k) = form.bound_keys().into_iter().find(|k| !keys.contains(k)) {
            return Some(format!("“{}” formu katmanın alanlarında olmayan “{k}” alanına bağlı.", f.name));
        }
    }
    if def.forms.iter().enumerate().any(|(i, f)| def.forms[..i].iter().any(|g| g.id == f.id)) {
        return Some("iki formun kimliği aynı.".to_owned());
    }
    None
}

/// A recipe form as this project keeps a form of layer `layer`: its draft, no release.
fn as_form(f: &kentos_commands::RoadRecipeForm, layer: &str) -> ProjectForm {
    ProjectForm {
        id: f.id.clone(),
        name: f.name.clone(),
        layer: layer.to_owned(),
        draft: f.draft.clone(),
        releases: Vec::new(),
        enforce: false,
        withdrawn: false,
        comments: Vec::new(),
    }
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
    let (fields, lists) = linked(doc, def, &def.fields, label)?;
    let count = fields.len();
    doc.set_layer_fields(id, fields, &[]).map_err(|r| refused(def, r))?;
    if let Some(s) = &def.subtypes {
        doc.set_layer_subtypes(id, Some(s.clone()), label).map_err(|r| refused(def, r))?;
    }
    // Docs/adr/5240: its value combinations, record rules, indexes and forms.
    if !def.contingents.is_empty() {
        doc.set_layer_contingents(id, def.contingents.clone(), label).map_err(|r| refused(def, r))?;
    }
    if !def.rules.is_empty() {
        doc.set_layer_rules(id, def.rules.clone(), label).map_err(|r| refused(def, r))?;
    }
    if !def.indexes.is_empty() {
        doc.set_layer_indexes(id, def.indexes.clone(), label).map_err(|r| refused(def, r))?;
    }
    let mut forms = Vec::new();
    if !def.forms.is_empty() {
        let mut settings = doc.settings().clone();
        for f in &def.forms {
            let taken = |c: &str| settings.forms.iter().any(|g| g.id == c);
            let here = if taken(&f.id) {
                (2..).map(|k| format!("{}-{k}", f.id)).find(|c| !taken(c)).unwrap_or_else(|| f.id.clone())
            } else {
                f.id.clone()
            };
            settings.forms.push(ProjectForm { id: here.clone(), ..as_form(f, id) });
            forms.push((here, f.name.clone()));
        }
        if let Some(p) = kentos_commands::forms_problem(&settings.forms) {
            return Err(refused(def, Refusal(p)));
        }
        doc.set_settings(settings);
    }
    Ok(Given {
        fields: count,
        subtypes: def.subtypes.is_some(),
        lists,
        contingents: def.contingents.len(),
        rules: def.rules.len(),
        indexes: def.indexes.len(),
        forms,
    })
}

/// Docs/adr/5240 (`addFields`): the recipe's fields layer `id` (found here) lacks by their names, added after its own,
/// the lists they link as an added layer's go; their names and the lists as they went. Nothing else of it changes.
pub(crate) fn add_missing(
    doc: &mut Document,
    id: &str,
    def: &RoadRecipeLayer,
    label: &str,
) -> Result<(Vec<String>, Vec<Listed>), Stop> {
    let Some(node) = doc.layers().get(id) else { return Ok((Vec::new(), Vec::new())) };
    let own = node.fields.clone();
    let missing: Vec<LayerField> = def.fields.iter().filter(|f| !own.iter().any(|h| h.name == f.name)).cloned().collect();
    if missing.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let (fields, lists) = linked(doc, def, &missing, label)?;
    let names = fields.iter().map(|f| f.name.clone()).collect();
    let mut next = own;
    next.extend(fields);
    doc.set_layer_fields(id, next, &[]).map_err(|r| refused(def, r))?;
    Ok((names, lists))
}

/// The recipe `fields` with the lists they link here (the module's rules), the lists written in the step `label`.
fn linked(doc: &mut Document, def: &RoadRecipeLayer, wanted: &[LayerField], label: &str) -> Result<(Vec<LayerField>, Vec<Listed>), Stop> {
    let before = doc.settings().domains.clone();
    let mut next = before.clone();
    let mut here: HashMap<String, String> = HashMap::new();
    let mut lists = Vec::new();
    for f in wanted {
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
    let fields: Vec<LayerField> = wanted
        .iter()
        .map(|f| match f.domain.as_ref().and_then(|rid| here.get(rid)).and_then(|to| next.iter().find(|d| d.id == *to)) {
            Some(d) => d.applied_to(f),
            None => f.clone(),
        })
        .collect();
    Ok((fields, lists))
}

/// What an added layer took of its recipe layer's fields, said: “Alanları (4) ve alt türleri tarifteki gibi. “Kaplama
/// türü” değer listesi de tarifteki tanımıyla eklendi.”; none without fields.
pub(crate) fn said(g: &Given) -> Option<String> {
    if g.fields == 0 {
        return None;
    }
    let mut out = format!("Alanları ({}){} tarifteki gibi.", g.fields, if g.subtypes { " ve alt türleri" } else { "" });
    // Docs/adr/5240: its value combinations, record rules, indexes and forms.
    let mut more = Vec::new();
    for (n, word) in [(g.contingents, "değer bileşimi"), (g.rules, "kayıt kuralı"), (g.indexes, "dizin")] {
        if n > 0 {
            more.push(format!("{n} {word}"));
        }
    }
    if !more.is_empty() {
        out.push_str(&format!(" {} de tarifteki gibi.", and_list(&more)));
    }
    if !g.forms.is_empty() {
        let names: Vec<String> = g.forms.iter().map(|(_, n)| format!("“{n}”")).collect();
        out.push_str(&format!(
            " {} {} taslak olarak eklendi; yayımlamak bu projenindir.",
            and_list(&names),
            if names.len() > 1 { "formları" } else { "formu" }
        ));
    }
    for l in &g.lists {
        out.push(' ');
        out.push_str(&list_said(l));
    }
    Some(out)
}

/// `a, b ve c`.
fn and_list(items: &[String]) -> String {
    match items.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} ve {last}", rest.join(", ")),
        Some((last, _)) => last.clone(),
        None => String::new(),
    }
}

/// How a recipe list went here, said.
fn list_said(l: &Listed) -> String {
    match &l.went {
        Went::Linked => format!("Alanları bu projenin aynı değerli “{}” listesine bağlandı.", l.name),
        Went::Added => format!("“{}” değer listesi de tarifteki tanımıyla eklendi.", l.name),
        Went::Renamed { here } => {
            format!("Bu projenin “{}” listesi başka değerler taşıyor, değişmedi; tarifinki “{here}” adıyla eklendi.", l.name)
        }
    }
}

/// The recipe's fields a found layer took (docs/adr/5240), said: “Tarifin “Tür” ve “Derinlik” alanları “Sıfır hattı”
/// katmanının alanlarının sonuna eklendi.” and the lists.
pub(crate) fn added_said(layer: &str, names: &[String], lists: &[Listed]) -> String {
    let quoted: Vec<String> = names.iter().map(|n| format!("“{n}”")).collect();
    let mut out = format!(
        "Tarifin {} {} “{layer}” katmanının alanlarının sonuna eklendi; katmanın kendi alanları ve öbür tanımları değişmedi.",
        and_list(&quoted),
        if names.len() > 1 { "alanları" } else { "alanı" }
    );
    for l in lists {
        out.push(' ');
        out.push_str(&list_said(l));
    }
    out
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

/// Takes away the forms `added` (their ids here) with their layer taken away again (docs/adr/5240).
pub(crate) fn remove_forms(doc: &mut Document, added: &[String]) {
    if added.is_empty() {
        return;
    }
    let mut settings = doc.settings().clone();
    settings.forms.retain(|f| !added.contains(&f.id));
    doc.set_settings(settings);
}
