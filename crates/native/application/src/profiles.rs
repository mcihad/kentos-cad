//! `cad.profiles.apply` and `cad.profiles.check` v1 (docs/adr/0541; plan/gis.md CBS-34): a data profile's layers,
//! fields, value lists, relations and address data the drawing lacks, added keeping what it has; and where the drawing's
//! data departs from the profile, said without changing anything. The profile is data (`catalog/v1/profiles/`): no
//! column name is written here. The shared cases are `fixtures/commands/v1/cad.profiles.{apply,check}.json`.
//!
//! Checks, in order: the profile (`unknown_profile`, `invalid_profile`), then for apply the expected revision.

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_commands::{
    AddressLocator, CommandResult, CommandWarning, DataProfile, Entity, FieldDomain, LayerField,
    LayerNode, LayerNodeType, LayerStyle, MAX_FINDING_UIDS, ProfileClass, ProfileClassSummary,
    ProfileFinding, ProfileFindingKind, ProfileMapping, ProfileRelation, ProfilesApplied,
    ProfilesApply, ProfilesCheck, ProfilesReport, ProjectRelation, builtin_profile, check_value,
    fold_name, follow_domains, mapping_problem, profile_problem, relations_problem, through,
};
use kentos_domain::{Document, NewLayer, default_style};
use kentos_geometry_core::entity::entity_area;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::profile_files::Shelf;

pub const STEP: &str = "Veri profili";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The profile a command names, as the profile's own document says it: `profile` when given, else the built-in `name`,
/// else the project's of that name (the drawing's copy, docs/adr/0735, else the file in `drawing`'s folder,
/// docs/adr/0558).
pub(crate) fn profile_only(
    name: Option<&str>,
    given: Option<&DataProfile>,
    shelf: Shelf<'_>,
) -> Result<DataProfile, Stop> {
    let profile = match (given, name) {
        (Some(p), _) => p.clone(),
        (None, Some(n)) => match builtin_profile(n) {
            Some(p) => p,
            None => match crate::profile_files::load_profile(shelf, n) {
                Ok(Some(p)) => p,
                Ok(None) => {
                    let carried = crate::profile_files::carried_words(
                        shelf,
                        kentos_commands::ProfileFileKind::Profile,
                    );
                    return Err(fail(
                        codes::UNKNOWN_PROFILE,
                        match crate::profile_files::folder_of(shelf.drawing) {
                            Some(_) => format!(
                                "“{n}” adlı veri profili yok; KentOS'un profili: tucbs-temel{carried}; çizimin “{}” klasöründe de “{n}{}” yok.",
                                kentos_commands::PROFILE_FOLDER,
                                kentos_commands::PROFILE_SUFFIX
                            ),
                            None => format!(
                                "“{n}” adlı veri profili yok; KentOS'un profili: tucbs-temel{carried}."
                            ),
                        },
                        "name",
                    ));
                }
                Err(why) => {
                    return Err(fail(
                        codes::INVALID_PROFILE,
                        format!("Veri profili geçersiz: {why}"),
                        "profile",
                    ));
                }
            },
        },
        (None, None) => {
            return Err(fail(
                codes::UNKNOWN_PROFILE,
                "Profili adıyla (name: tucbs-temel) ya da kendisini (profile) verin.".into(),
                "name",
            ));
        }
    };
    if let Some(why) = profile_problem(&profile) {
        return Err(fail(
            codes::INVALID_PROFILE,
            format!("Veri profili geçersiz: {why}"),
            "profile",
        ));
    }
    Ok(profile)
}

/// The profile a command works with and the mapping it was read through.
pub(crate) struct Resolved {
    pub profile: DataProfile,
    pub mapping: Option<String>,
}

/// The profile of a command's input; with a mapping (`mapping`, else the project's `mapping_name`) the profile read
/// through it (docs/adr/0558): the mapping must hold for the profile, and the profile it gives holds too.
pub(crate) fn resolve(
    name: Option<&str>,
    given: Option<&DataProfile>,
    mapping: Option<&ProfileMapping>,
    mapping_name: Option<&str>,
    shelf: Shelf<'_>,
) -> Result<Resolved, Stop> {
    let profile = profile_only(name, given, shelf)?;
    let Some(m) = mapping_for(&profile, mapping, mapping_name, shelf)? else {
        return Ok(Resolved {
            profile,
            mapping: None,
        });
    };
    let through = through(&profile, &m);
    if let Some(why) = profile_problem(&through) {
        return Err(fail(
            codes::INVALID_PROFILE_MAPPING,
            format!("Eşleme geçersiz: profil eşlemeyle okununca tutmuyor: {why}"),
            "mapping",
        ));
    }
    Ok(Resolved {
        profile: through,
        mapping: Some(m.name),
    })
}

/// The mapping of a command's input (`mapping`, else the project's `mapping_name`), held for `profile`; none without.
pub(crate) fn mapping_for(
    profile: &DataProfile,
    mapping: Option<&ProfileMapping>,
    mapping_name: Option<&str>,
    shelf: Shelf<'_>,
) -> Result<Option<ProfileMapping>, Stop> {
    let found: Option<ProfileMapping> = match (mapping, mapping_name) {
        (Some(m), _) => Some(m.clone()),
        (None, Some(n)) => match crate::profile_files::load_mapping(shelf, n) {
            Ok(Some(m)) => Some(m),
            Ok(None) => {
                let carried = crate::profile_files::carried_words(
                    shelf,
                    kentos_commands::ProfileFileKind::Mapping,
                );
                return Err(fail(
                    codes::UNKNOWN_MAPPING,
                    match crate::profile_files::folder_of(shelf.drawing) {
                        Some(_) => format!(
                            "“{n}” adlı eşleme yok: çizimin “{}” klasöründe “{n}{}” yok{carried}.",
                            kentos_commands::PROFILE_FOLDER,
                            kentos_commands::MAPPING_SUFFIX
                        ),
                        None if carried.is_empty() => format!(
                            "“{n}” adlı eşleme bulunamaz: çizimin dosyası yok; eşlemeyi kendisiyle (mapping) verin."
                        ),
                        None => format!("“{n}” adlı eşleme yok{carried}."),
                    },
                    "mappingName",
                ));
            }
            Err(why) => {
                return Err(fail(
                    codes::INVALID_PROFILE_MAPPING,
                    format!("Eşleme geçersiz: {why}"),
                    "mapping",
                ));
            }
        },
        (None, None) => None,
    };
    if let Some(m) = &found
        && let Some(why) = mapping_problem(m, profile)
    {
        return Err(fail(
            codes::INVALID_PROFILE_MAPPING,
            format!("Eşleme geçersiz: {why}"),
            "mapping",
        ));
    }
    Ok(found)
}

/// Where a class's layer path leads in the tree.
pub(crate) enum Place {
    /// The layer, by its id.
    Layer(String),
    /// The path's last name is a group, or one before it a layer: the class has no layer there.
    Blocked(String),
    /// Not there: the deepest group found (none: the tree's top) and how many of the path's names it covers.
    Missing(Option<String>, usize),
}

pub(crate) fn find(nodes: &[LayerNode], path: &[String]) -> Place {
    let mut here = nodes;
    let mut parent: Option<String> = None;
    for (i, name) in path.iter().enumerate() {
        let last = i + 1 == path.len();
        let key = fold_name(name.trim());
        let Some(n) = here.iter().find(|n| fold_name(n.name.trim()) == key) else {
            return Place::Missing(parent, i);
        };
        match (n.kind, last) {
            (LayerNodeType::Layer, true) => return Place::Layer(n.id.clone()),
            (LayerNodeType::Group, false) => {
                parent = Some(n.id.clone());
                here = &n.children;
            }
            _ => return Place::Blocked(n.id.clone()),
        }
    }
    Place::Missing(parent, path.len())
}

pub(crate) fn path_words(path: &[String]) -> String {
    path.join(" › ")
}

/// A field of the layer that stands for the profile's field: the same name, or the same key.
pub(crate) fn layer_field<'a>(fields: &'a [LayerField], name: &str) -> Option<&'a LayerField> {
    fields
        .iter()
        .find(|f| f.name == name)
        .or_else(|| fields.iter().find(|f| f.key() == name))
}

fn kind_word(f: &LayerField) -> String {
    f.kind.label().to_lowercase()
}

fn warn(code: &str, message: String) -> CommandWarning {
    CommandWarning {
        code: code.into(),
        message,
        path: None,
    }
}

// ── Apply ────────────────────────────────────────────────────────────────────

fn check_apply(doc: &Document, input: &ProfilesApply) -> Result<DataProfile, Stop> {
    let p = resolve(
        input.name.as_deref(),
        input.profile.as_ref(),
        input.mapping.as_ref(),
        input.mapping_name.as_deref(),
        Shelf::of(doc, input.drawing.as_deref()),
    )?
    .profile;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(p)
}

/// The profile's parts written to `doc`.
fn write(
    doc: &mut Document,
    p: &DataProfile,
) -> Result<(ProfilesApplied, Vec<CommandWarning>), Stop> {
    let mut out = ProfilesApplied::default();
    let mut warnings = Vec::new();
    // The value lists the project will have: its own first (kept), then the profile's it lacks.
    let mut settings = doc.settings().clone();
    let mut domains: Vec<FieldDomain> = settings.domains.clone();
    for d in &p.domains {
        match domains.iter().find(|x| x.id == d.id) {
            Some(have) if have.kind != d.kind || have.values != d.values => warnings.push(warn(
                codes::PROFILE_DOMAIN_DIFFERS,
                format!(
                    "Projenin “{}” değer listesi profilinkinden farklı; projeninki korundu.",
                    have.name
                ),
            )),
            Some(_) => {}
            None => {
                domains.push(d.clone());
                out.domains.push(d.id.clone());
            }
        }
    }
    // 1. The layers and their fields: one undo step.
    let group = doc.begin_group(STEP);
    let mut layer_of: HashMap<&str, String> = HashMap::new();
    for c in &p.classes {
        match place_class(doc, c, &domains, &mut out, &mut warnings) {
            Ok(Some(id)) => {
                layer_of.insert(c.id.as_str(), id);
            }
            Ok(None) => {}
            Err(stop) => {
                doc.cancel_group(group);
                return Err(stop);
            }
        }
    }
    doc.end_group(group);
    // 2. The value lists, relations and address data: project settings, no undo step.
    settings = doc.settings().clone();
    let mut changed_settings = !out.domains.is_empty();
    settings.domains = domains;
    let mut relations = settings.relations.clone();
    for r in &p.relations {
        let (Some(parent), Some(child)) = (
            layer_of.get(r.parent.as_str()),
            layer_of.get(r.child.as_str()),
        ) else {
            continue;
        };
        let key = relation_key(doc, child, r);
        let held = relations
            .iter()
            .any(|x| x.id == r.id || (x.parent == *parent && x.child == *child && x.field == key));
        if !held {
            relations.push(ProjectRelation {
                id: r.id.clone(),
                name: r.name.clone(),
                parent: parent.clone(),
                child: child.clone(),
                field: key,
                on_delete: Default::default(),
            });
            out.relations.push(r.id.clone());
        }
    }
    if !out.relations.is_empty() {
        if let Some(why) = relations_problem(&relations) {
            warnings.push(warn(
                codes::PROFILE_LAYER_SKIPPED,
                format!("Profilin ilişkileri yazılamadı: {why}"),
            ));
            out.relations.clear();
        } else {
            settings.relations = relations;
            changed_settings = true;
        }
    }
    if let Some(locator) = locator_of(p, &layer_of)
        && !settings
            .address_locators
            .iter()
            .any(|l| l.id == locator.id || l.name == locator.name)
    {
        let mut all = settings.address_locators.clone();
        all.push(locator);
        if kentos_commands::address_locators_problem(&all).is_none() {
            settings.address_locators = all;
            out.address = true;
            changed_settings = true;
        }
    }
    if changed_settings {
        doc.set_settings(settings);
    }
    out.changed = changed_settings || !out.layers.is_empty() || !out.fields.is_empty();
    out.revision = doc.revision().to_string();
    Ok((out, warnings))
}

/// The child layer's key for a relation's field: its own field's key when it has the field.
fn relation_key(doc: &Document, child: &str, r: &ProfileRelation) -> String {
    doc.layers()
        .get(child)
        .and_then(|n| layer_field(&n.fields, &r.field))
        .map_or_else(|| r.field.clone(), |f| f.key().to_owned())
}

/// The profile's address data with its classes' layers; none when a class it reads has no layer.
fn locator_of(p: &DataProfile, layer_of: &HashMap<&str, String>) -> Option<AddressLocator> {
    let a = p.address.as_ref()?;
    let layer = |class: &str| layer_of.get(class).cloned();
    let mut doors = a.doors.clone();
    if let Some(d) = doors.as_mut() {
        d.layer = layer(&d.layer)?;
    }
    let mut streets = a.streets.clone();
    if let Some(s) = streets.as_mut() {
        s.layer = layer(&s.layer)?;
    }
    let mut areas = a.areas.clone();
    for x in &mut areas {
        x.layer = layer(&x.layer)?;
    }
    Some(AddressLocator {
        id: format!("profil-{}", p.name),
        name: p.title.clone(),
        doors,
        streets,
        areas,
        min_score: a.min_score,
        reach: a.reach,
    })
}

/// A class's layer found or added, its missing fields added; its id, none when the class was skipped.
fn place_class(
    doc: &mut Document,
    c: &ProfileClass,
    domains: &[FieldDomain],
    out: &mut ProfilesApplied,
    warnings: &mut Vec<CommandWarning>,
) -> Result<Option<String>, Stop> {
    let fields = follow_domains(&c.fields, domains).0;
    match find(doc.layers().nodes(), &c.layer) {
        Place::Layer(id) => {
            let Some(node) = doc.layers().get(&id) else {
                return Ok(None);
            };
            let mut have = node.fields.clone();
            let mut added = Vec::new();
            for f in &fields {
                match layer_field(&have, &f.name) {
                    Some(h) if h.kind != f.kind => warnings.push(warn(
                        codes::PROFILE_FIELD_DIFFERS,
                        format!(
                            "“{}” katmanının “{}” alanı {}, profilde {}; katmanınki korundu.",
                            node.name,
                            h.name,
                            kind_word(h),
                            kind_word(f)
                        ),
                    )),
                    Some(_) => {}
                    None => added.push(f.clone()),
                }
            }
            if added.is_empty() {
                return Ok(Some(id));
            }
            if node.locked {
                warnings.push(warn(
                    codes::PROFILE_LAYER_SKIPPED,
                    format!(
                        "“{}” katmanı kilitli; profilin alanları eklenmedi.",
                        node.name
                    ),
                ));
                return Ok(Some(id));
            }
            let name = node.name.clone();
            for f in &added {
                out.fields.push(format!("{name}.{}", f.name));
            }
            have.extend(added);
            doc.set_layer_fields(&id, have, &[])
                .map_err(|r| fail(codes::LAYER_REFUSED, r.0, "profile"))?;
            Ok(Some(id))
        }
        Place::Blocked(id) => {
            let name = doc.layers().get(&id).map_or(id.clone(), |n| n.name.clone());
            warnings.push(warn(
                codes::PROFILE_LAYER_SKIPPED,
                format!(
                    "“{}” sınıfının yolunda ({}) “{name}” var ama katman değil ya da grup değil; sınıf atlandı.",
                    c.title,
                    path_words(&c.layer)
                ),
            ));
            Ok(None)
        }
        Place::Missing(mut parent, from) => {
            for (i, name) in c.layer.iter().enumerate().skip(from) {
                let last = i + 1 == c.layer.len();
                let mut new = if last {
                    let mut l = NewLayer::layer(name.trim());
                    l.style = LayerStyle {
                        color: c.color.clone(),
                        ..default_style()
                    };
                    l.fields = fields.clone();
                    l
                } else {
                    NewLayer::group(name.trim())
                };
                new.id = None;
                let id = doc
                    .add_layer_at(new, parent.as_deref(), None, None, false)
                    .map_err(|r| fail(codes::LAYER_REFUSED, r.0, "profile"))?;
                out.layers.push(id.clone());
                parent = Some(id);
            }
            Ok(parent)
        }
    }
}

pub mod apply {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesApply) -> CommandResult<()> {
        match check_apply(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// The preview: the same write on a reading copy; the drawing stays.
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &ProfilesApply,
    ) -> CommandResult<ProfilesApplied> {
        plan_in(cx.doc, input)
    }

    pub fn plan_in(doc: &Document, input: &ProfilesApply) -> CommandResult<ProfilesApplied> {
        let p = match check_apply(doc, input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        let mut copy = doc.reading_copy();
        match write(&mut copy, &p) {
            Ok((mut output, warnings)) => {
                output.revision = doc.revision().to_string();
                CommandResult::Completed { output, warnings }
            }
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ProfilesApply,
    ) -> CommandResult<ProfilesApplied> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_PROFILES_APPLY,
            kentos_commands::CAD_PROFILES_APPLY_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: ProfilesApply,
    ) -> CommandResult<ProfilesApplied> {
        let p = match check_apply(cx.doc, &input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        match write(cx.doc, &p) {
            Ok((output, warnings)) => CommandResult::Completed { output, warnings },
            Err(stop) => stop.into(),
        }
    }
}

// ── Check ────────────────────────────────────────────────────────────────────

/// A number as the messages write it: two decimals, a decimal comma.
fn m2(x: f64) -> String {
    format!("{x:.2}").replace('.', ",")
}

fn finding(
    kind: ProfileFindingKind,
    class: Option<&str>,
    layer: Option<&str>,
    subject: Option<&str>,
    uids: Vec<String>,
    fixable: bool,
    message: String,
) -> ProfileFinding {
    let count = uids.len() as u32;
    let mut uids = uids;
    uids.truncate(MAX_FINDING_UIDS);
    ProfileFinding {
        kind,
        class: class.map(str::to_owned),
        layer: layer.map(str::to_owned),
        subject: subject.map(str::to_owned),
        value: None,
        uids,
        count,
        geometric: None,
        registered: None,
        fixable,
        message,
    }
}

/// An object's value of a field, trimmed; none when empty.
fn value_of<'a>(e: &'a Entity, key: &str) -> Option<&'a str> {
    e.base()
        .attrs
        .get(key)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
}

/// Values grouped in their first object's order: (value, the objects').
fn grouped(rows: Vec<(String, String)>) -> Vec<(String, Vec<String>)> {
    let mut order: Vec<String> = Vec::new();
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (value, uid) in rows {
        if !by.contains_key(&value) {
            order.push(value.clone());
        }
        by.entry(value).or_default().push(uid);
    }
    order
        .into_iter()
        .map(|v| {
            let uids = by.remove(&v).unwrap_or_default();
            (v, uids)
        })
        .collect()
}

/// The report of how the drawing's data stands to the profile.
pub fn report(doc: &Document, p: &DataProfile) -> ProfilesReport {
    let mut findings: Vec<ProfileFinding> = Vec::new();
    let settings = doc.settings();
    for d in &p.domains {
        if !settings.domains.iter().any(|x| x.id == d.id) {
            findings.push(finding(
                ProfileFindingKind::MissingDomain,
                None,
                None,
                Some(&d.id),
                Vec::new(),
                true,
                format!(
                    "“{}” değer listesi projede yok; Veri profilini uygula onu ekler.",
                    d.name
                ),
            ));
        }
    }
    let per_metre = settings.drawing_unit.unwrap_or_default().metres();
    let square = (per_metre.0 / per_metre.1) * (per_metre.0 / per_metre.1);
    let mut summaries = Vec::new();
    let mut layer_of: HashMap<&str, String> = HashMap::new();
    for c in &p.classes {
        let start = findings.len();
        let layer = match find(doc.layers().nodes(), &c.layer) {
            Place::Layer(id) => Some(id),
            Place::Blocked(id) => {
                let name = doc.layers().get(&id).map_or(id.clone(), |n| n.name.clone());
                findings.push(finding(
                    ProfileFindingKind::MissingLayer,
                    Some(&c.id),
                    None,
                    None,
                    Vec::new(),
                    false,
                    format!(
                        "“{}” sınıfının yolunda ({}) “{name}” var ama katman değil ya da grup değil.",
                        c.title,
                        path_words(&c.layer)
                    ),
                ));
                None
            }
            Place::Missing(..) => {
                findings.push(finding(
                    ProfileFindingKind::MissingLayer,
                    Some(&c.id),
                    None,
                    None,
                    Vec::new(),
                    true,
                    format!(
                        "“{}” sınıfının katmanı ({}) çizimde yok; Veri profilini uygula onu ekler.",
                        c.title,
                        path_words(&c.layer)
                    ),
                ));
                None
            }
        };
        let mut objects = 0u32;
        if let Some(id) = &layer {
            layer_of.insert(c.id.as_str(), id.clone());
            objects = class_findings(doc, p, c, id, square, &mut findings);
        }
        summaries.push(ProfileClassSummary {
            class: c.id.clone(),
            title: c.title.clone(),
            layer,
            objects,
            findings: (findings.len() - start) as u32,
        });
    }
    relation_findings(doc, p, &layer_of, &mut findings, &mut summaries);
    if p.address.is_some() {
        let id = format!("profil-{}", p.name);
        if !settings
            .address_locators
            .iter()
            .any(|l| l.id == id || l.name == p.title)
        {
            findings.push(finding(
                ProfileFindingKind::MissingAddress,
                None,
                None,
                Some(&id),
                Vec::new(),
                true,
                "Profilin adres verisi projede yok; Veri profilini uygula onu ekler.".into(),
            ));
        }
    }
    ProfilesReport {
        profile: p.name.clone(),
        title: p.title.clone(),
        version: p.version,
        basis: p.basis.clone(),
        mapping: None,
        classes: summaries,
        clean: findings.is_empty(),
        findings,
    }
}

/// The findings of one class's layer and objects; the number of objects on the layer.
fn class_findings(
    doc: &Document,
    p: &DataProfile,
    c: &ProfileClass,
    id: &str,
    square: f64,
    findings: &mut Vec<ProfileFinding>,
) -> u32 {
    let Some(node) = doc.layers().get(id) else {
        return 0;
    };
    let name = node.name.clone();
    let fields = follow_domains(&c.fields, &p.domains).0;
    // The layer's fields.
    let mut keys: Vec<(LayerField, String)> = Vec::new();
    for f in &fields {
        match layer_field(&node.fields, &f.name) {
            None => findings.push(finding(
                ProfileFindingKind::MissingField,
                Some(&c.id),
                Some(id),
                Some(&f.name),
                Vec::new(),
                true,
                format!(
                    "“{name}” katmanında “{}” alanı yok; Veri profilini uygula onu ekler.",
                    f.name
                ),
            )),
            Some(h) if h.kind != f.kind => {
                findings.push(finding(
                    ProfileFindingKind::FieldKind,
                    Some(&c.id),
                    Some(id),
                    Some(&f.name),
                    Vec::new(),
                    false,
                    format!(
                        "“{name}” katmanının “{}” alanı {}, profilde {}.",
                        h.name,
                        kind_word(h),
                        kind_word(f)
                    ),
                ));
                keys.push((f.clone(), h.name.clone()));
            }
            Some(h) => keys.push((f.clone(), h.name.clone())),
        }
    }
    let objects: Vec<(String, &Entity)> = doc
        .entities_with_uids()
        .filter(|(_, e)| e.base().layer_id == id)
        .map(|(u, e)| (u.to_string(), e))
        .collect();
    // The objects' geometry.
    let wrong: Vec<String> = objects
        .iter()
        .filter(|(_, e)| !c.geometry.holds(e.kind()))
        .map(|(u, _)| u.clone())
        .collect();
    if !wrong.is_empty() {
        let n = wrong.len();
        findings.push(finding(
            ProfileFindingKind::WrongGeometry,
            Some(&c.id),
            Some(id),
            None,
            wrong,
            false,
            format!("“{name}” katmanında {n} nesne {} değil.", c.geometry.word()),
        ));
    }
    // Required, then values, field by field.
    for (f, key) in &keys {
        if f.required {
            let empty: Vec<String> = objects
                .iter()
                .filter(|(_, e)| value_of(e, key).is_none())
                .map(|(u, _)| u.clone())
                .collect();
            if !empty.is_empty() {
                let n = empty.len();
                findings.push(finding(
                    ProfileFindingKind::RequiredEmpty,
                    Some(&c.id),
                    Some(id),
                    Some(&f.name),
                    empty,
                    false,
                    format!(
                        "“{name}” katmanında {n} nesnenin “{}” alanı boş; alan zorunlu.",
                        f.name
                    ),
                ));
            }
        }
        let mut bad: Vec<(String, String)> = Vec::new();
        let mut why: HashMap<String, String> = HashMap::new();
        for (u, e) in &objects {
            if let Some(v) = value_of(e, key)
                && let Err(r) = check_value(f, v)
            {
                why.entry(v.to_owned()).or_insert(r.message);
                bad.push((v.to_owned(), u.clone()));
            }
        }
        for (v, uids) in grouped(bad) {
            let n = uids.len();
            let mut x = finding(
                ProfileFindingKind::BadValue,
                Some(&c.id),
                Some(id),
                Some(&f.name),
                uids,
                false,
                format!(
                    "“{name}” katmanında {n} nesnenin “{}” değeri “{v}” kurala uymuyor: {}",
                    f.name,
                    why.get(&v).map_or("", String::as_str)
                ),
            );
            x.value = Some(v);
            findings.push(x);
        }
    }
    // Identifiers.
    if let Some(k) = &c.key
        && let Some((_, key)) = keys.iter().find(|(f, _)| &f.name == k)
    {
        let rows: Vec<(String, String)> = objects
            .iter()
            .filter_map(|(u, e)| value_of(e, key).map(|v| (v.to_owned(), u.clone())))
            .collect();
        for (v, uids) in grouped(rows) {
            if uids.len() < 2 {
                continue;
            }
            let n = uids.len();
            let mut x = finding(
                ProfileFindingKind::DuplicateKey,
                Some(&c.id),
                Some(id),
                Some(k),
                uids,
                false,
                format!("“{name}” katmanında “{v}” kimliği {n} nesnede var; “{k}” tekil olmalı."),
            );
            x.value = Some(v);
            findings.push(x);
        }
    }
    // Registered areas.
    if let Some(r) = &c.registered_area
        && let Some((f, key)) = keys.iter().find(|(f, _)| f.name == r.field)
    {
        let ident = c
            .key
            .as_ref()
            .and_then(|k| keys.iter().find(|(f, _)| &f.name == k))
            .map(|(_, key)| key.clone());
        for (u, e) in &objects {
            if !c.geometry.holds(e.kind()) {
                continue;
            }
            let Some(v) = value_of(e, key) else {
                continue;
            };
            let Ok(text) = check_value(f, v) else {
                continue;
            };
            let Ok(registered) = text.parse::<f64>() else {
                continue;
            };
            let Some(area) = entity_area(&crate::geometry::shape(e)) else {
                continue;
            };
            let geometric = area * square;
            let diff = geometric - registered;
            if diff.abs() <= r.tolerance {
                continue;
            }
            let who = ident
                .as_deref()
                .and_then(|k| value_of(e, k))
                .map_or_else(|| "Nesnenin".to_owned(), |v| format!("“{v}” nesnesinin"));
            let mut x = finding(
                ProfileFindingKind::AreaDifference,
                Some(&c.id),
                Some(id),
                Some(&r.field),
                vec![u.clone()],
                false,
                format!(
                    "{who} geometrik alanı {} m², kayıtlı alanı {} m²; fark {} m² profilin toleransını ({} m²) aşıyor. Hiçbiri öbürünün yerine yazılmaz.",
                    m2(geometric),
                    m2(registered),
                    m2(diff),
                    m2(r.tolerance)
                ),
            );
            x.geometric = Some(geometric);
            x.registered = Some(registered);
            findings.push(x);
        }
    }
    objects.len() as u32
}

/// The relations the project lacks and the links that name no parent object.
fn relation_findings(
    doc: &Document,
    p: &DataProfile,
    layer_of: &HashMap<&str, String>,
    findings: &mut Vec<ProfileFinding>,
    summaries: &mut [ProfileClassSummary],
) {
    let class = |id: &str| p.classes.iter().find(|c| c.id == id);
    for r in &p.relations {
        let (Some(parent), Some(child)) = (
            layer_of.get(r.parent.as_str()),
            layer_of.get(r.child.as_str()),
        ) else {
            continue;
        };
        let key = relation_key(doc, child, r);
        let held =
            doc.settings().relations.iter().any(|x| {
                x.id == r.id || (x.parent == *parent && x.child == *child && x.field == key)
            });
        if !held {
            findings.push(finding(
                ProfileFindingKind::MissingRelation,
                None,
                None,
                Some(&r.id),
                Vec::new(),
                true,
                format!(
                    "“{}” ilişkisi projede yok; Veri profilini uygula onu ekler.",
                    r.name
                ),
            ));
        }
        let parents: HashSet<String> = doc
            .entities_with_uids()
            .filter(|(_, e)| e.base().layer_id == *parent)
            .map(|(u, _)| u.to_string())
            .collect();
        let attr = doc
            .layers()
            .get(child)
            .and_then(|n| n.fields.iter().find(|f| f.key() == key))
            .map_or_else(|| key.clone(), |f| f.name.clone());
        let broken: Vec<String> = doc
            .entities_with_uids()
            .filter(|(_, e)| e.base().layer_id == *child)
            .filter(|(_, e)| value_of(e, &attr).is_some_and(|v| !parents.contains(v)))
            .map(|(u, _)| u.to_string())
            .collect();
        if broken.is_empty() {
            continue;
        }
        let n = broken.len();
        let (pt, ct) = (
            class(&r.parent).map_or("", |c| c.title.as_str()),
            class(&r.child).map_or("", |c| c.title.as_str()),
        );
        findings.push(finding(
            ProfileFindingKind::RelationBroken,
            Some(&r.child),
            Some(child),
            Some(&r.id),
            broken,
            false,
            format!(
                "“{}” ilişkisinde {n} {ct} nesnesinin “{}” alanı {pt} katmanında olmayan bir nesneyi anıyor.",
                r.name, r.field
            ),
        ));
        if let Some(s) = summaries.iter_mut().find(|s| s.class == r.child) {
            s.findings += 1;
        }
    }
}

pub mod check {
    use super::*;

    fn resolved(doc: &Document, input: &ProfilesCheck) -> Result<Resolved, Stop> {
        resolve(
            input.name.as_deref(),
            input.profile.as_ref(),
            input.mapping.as_ref(),
            input.mapping_name.as_deref(),
            Shelf::of(doc, input.drawing.as_deref()),
        )
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesCheck) -> CommandResult<()> {
        match resolved(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesCheck) -> CommandResult<ProfilesReport> {
        run(cx.doc, input)
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ProfilesCheck,
    ) -> CommandResult<ProfilesReport> {
        run(cx.doc, &input)
    }

    pub fn run(doc: &Document, input: &ProfilesCheck) -> CommandResult<ProfilesReport> {
        match resolved(doc, input) {
            Ok(r) => {
                let mut report = report(doc, &r.profile);
                report.mapping = r.mapping;
                CommandResult::Completed {
                    output: report,
                    warnings: Vec::new(),
                }
            }
            Err(stop) => stop.into(),
        }
    }
}
