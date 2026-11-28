//! `cad.elementtypes.edit` v1 (docs/adr/8106 §2–§4; plan/bim-plan.md BIM-12): the project's column, beam, slab, roof,
//! railing and stair types made (from scratch, as a copy or from a drawn element), changed, renamed and removed, and
//! elements tied to a type, freed, taken back, pushed into it or brought to its latest version, as one undo step
//! “Eleman tipi” with every element following a changed type. Its plan is the preview: the table and every element's
//! fate, nothing written. The shared cases are `fixtures/commands/v1/cad.elementtypes.edit.json`
//! (`scripts/fixtures/element_types_cases.py`).

use kentos_commands::{
    CommandResult, CommandWarning, ElementKept, ElementLeft, ElementLeftWhy, ElementType,
    ElementTypeKind, ElementTypeRefused, ElementTypeReport, ElementTypesEdit, ElementTypesEdited,
    ElementTypesOperation, Entity, element_type_name_ok, element_type_values,
    element_types_problem, follow_type, opening_type_id_of, type_link,
};
use kentos_domain::{Document, Slot};
use serde_json::{Map, Value};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const STEP: &str = "Eleman tipi";

/// What the command writes: the table after it, the elements that follow, the report.
struct Work {
    types: Vec<ElementType>,
    id: Option<String>,
    updates: Vec<(Slot, Entity)>,
    report: ElementTypeReport,
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn invalid(path: &str, message: String) -> Stop {
    fail(codes::INVALID_ELEMENT_TYPE, message, path)
}

/// The type `key` names in `types`: by id, then by name (case aside, one of that name); `element_type_not_found`
/// with the types there are.
fn find(types: &[ElementType], key: &str, path: &str) -> Result<usize, Stop> {
    if let Some(i) = types.iter().position(|t| t.id == key) {
        return Ok(i);
    }
    let folded = key.trim().to_lowercase();
    let named: Vec<usize> = (0..types.len())
        .filter(|&i| types[i].name.to_lowercase() == folded)
        .collect();
    match named.as_slice() {
        [i] => Ok(*i),
        [] => {
            let there: Vec<String> = types
                .iter()
                .map(|t| format!("{} ({})", t.name, t.kind.words()))
                .collect();
            Err(fail(
                codes::ELEMENT_TYPE_NOT_FOUND,
                if there.is_empty() {
                    format!(
                        "“{key}” adında ya da kimliğinde eleman tipi yok; projede eleman tipi yok."
                    )
                } else {
                    format!(
                        "“{key}” adında ya da kimliğinde eleman tipi yok. Var olanlar: {}.",
                        there.join(", ")
                    )
                },
                path,
            ))
        }
        many => {
            let ids: Vec<String> = many
                .iter()
                .map(|&i| format!("{} ({})", types[i].id, types[i].kind.words()))
                .collect();
            Err(fail(
                codes::ELEMENT_TYPE_NOT_FOUND,
                format!(
                    "“{key}” adında birden çok tip var; kimliğiyle verin: {}.",
                    ids.join(", ")
                ),
                path,
            ))
        }
    }
}

/// Why `id` or `name` cannot be a new `kind` type's, or another's than `but`: the id taken, or the name among the
/// kind's.
fn taken(
    types: &[ElementType],
    kind: ElementTypeKind,
    id: &str,
    name: &str,
    but: Option<usize>,
) -> Option<(String, &'static str)> {
    let folded = name.to_lowercase();
    types
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != but)
        .find_map(|(_, t)| {
            if t.id == id {
                Some((
                    format!("Projede “{id}” kimlikli bir tip (“{}”) zaten var.", t.name),
                    "operation.id",
                ))
            } else if t.kind == kind && t.name.to_lowercase() == folded {
                Some((
                    format!(
                        "Projede “{}” adında bir {} tipi zaten var; başka bir ad verin.",
                        t.name,
                        kind.words()
                    ),
                    "operation.name",
                ))
            } else {
                None
            }
        })
}

/// A free id for `name`: from its name, then with `-2`, `-3` ….
fn free_id(types: &[ElementType], name: &str) -> String {
    let base = opening_type_id_of(name);
    let mut id = base.clone();
    let mut n = 2;
    while types.iter().any(|t| t.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

fn name_ok(name: &str, path: &str) -> Result<(), Stop> {
    if element_type_name_ok(name) {
        return Ok(());
    }
    Err(invalid(
        path,
        format!(
            "“{name}” tip adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok {} harf.",
            kentos_commands::MAX_ELEMENT_TYPE_NAME
        ),
    ))
}

/// `t`'s values as an element writes them (a default given left out, `1` as `1.0`), so a follower is no exception for
/// how its type was written.
fn canonical(t: &mut ElementType) {
    if let Some(values) = t.canonical_values() {
        t.values = values;
    }
}

fn uid_of(doc: &Document, slot: Slot) -> String {
    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
}

fn locked(doc: &Document, e: &Entity) -> bool {
    doc.layers().is_locked(&e.base().layer_id)
}

fn locked_stop(doc: &Document, e: &Entity, path: &str) -> Stop {
    let layer = &e.base().layer_id;
    let layers = doc.layers();
    let name = layers
        .get(layer)
        .map_or(layer.as_str(), |n| n.name.as_str());
    fail(
        codes::LAYER_LOCKED,
        format!(
            "“{name}” katmanı kilitli; üzerindeki eleman düzenlenemez. Kilidi Katmanlar panelinden açın."
        ),
        path,
    )
}

/// The elements following the type `id`, in the drawing's order: each its slot and itself.
fn followers(doc: &Document, id: &str) -> Vec<(Slot, Entity)> {
    doc.entities()
        .filter(|e| type_link(e).is_some_and(|l| l.id == id))
        .map(|e| (Slot(e.base().id), e.clone()))
        .collect()
}

/// The fields whose values `old` and `new` differ in, in their kind's order (a named one when the name changed).
fn changed_fields(old: &ElementType, new: &ElementType) -> Vec<String> {
    new.kind
        .fields()
        .iter()
        .filter(|f| old.value(f) != new.value(f))
        .map(|f| f.key.to_owned())
        .collect()
}

/// The elements following `new` taking it (docs/adr/8106 §3): `fields` the ones the change changed. An element on a
/// locked layer stays at the older version (skipped); one its kind's own rule refuses the new values refuses all with
/// `refused` stop, else stays (refused). `own` is an element given its own new fields first (Tipe uygula's), which
/// then follows like the others.
fn follow(
    doc: &Document,
    new: &ElementType,
    fields: &[String],
    refused: ElementTypeRefused,
    own: Option<(Slot, Entity, Vec<String>)>,
    work: &mut Work,
) -> Result<(), Stop> {
    for (slot, e) in followers(doc, &new.id) {
        let uid = uid_of(doc, slot);
        if locked(doc, &e) {
            work.report.skipped.push(ElementLeft {
                uid,
                why: ElementLeftWhy::Locked,
                message:
                    "Kilitli katmanda; eski sürümde kaldı. Kilidi açıp tipi yeniden uygulayın."
                        .into(),
            });
            continue;
        }
        let keep = match &own {
            Some((s, _, keep)) if *s == slot => keep.clone(),
            _ => type_link(&e)
                .map(|l| l.overrides.clone())
                .unwrap_or_default(),
        };
        match follow_type(&e, new, &keep) {
            Ok(typed) => {
                let kept: Vec<String> = fields
                    .iter()
                    .filter(|f| keep.contains(f))
                    .cloned()
                    .collect();
                if !kept.is_empty() {
                    work.report.kept.push(ElementKept {
                        uid: uid.clone(),
                        fields: kept,
                    });
                }
                work.report.applied.push(uid);
                work.updates.push((slot, typed));
            }
            Err((_, why)) => {
                let kind = ElementTypeKind::of(&e).map_or("eleman", ElementTypeKind::words);
                if refused == ElementTypeRefused::Stop {
                    return Err(fail(
                        codes::ELEMENT_TYPE_REFUSED,
                        format!(
                            "“{}” tipini izleyen “{uid}” kimlikli {kind} yeni değerlerle geçersiz oluyor: {why} Hiçbir şey \
                             yazılmadı; değeri düzeltin, o elemanı tipten ayırın ya da onRefused: skip ile onu eski \
                             sürümde bırakın.",
                            new.name
                        ),
                        "operation",
                    ));
                }
                work.report.refused.push(ElementLeft {
                    uid,
                    why: ElementLeftWhy::Invalid,
                    message: why,
                });
            }
        }
    }
    Ok(())
}

/// The elements an instance operation names, each once: where the input names it, its slot, itself, its id. Each must
/// be of a kind a type serves and on an open layer.
fn elements<'a>(
    doc: &'a Document,
    uids: &'a [String],
    nothing: &str,
) -> Result<Vec<(usize, Slot, &'a Entity, String)>, Stop> {
    checks::uids(uids, nothing)?;
    let found = checks::named(doc, uids)?;
    let mut out = Vec::with_capacity(found.len());
    for (at, slot, e, uid) in found {
        let path = format!("uids[{at}]");
        if ElementTypeKind::of(e).is_none() {
            return Err(fail(
                codes::ELEMENT_TYPE_KIND,
                format!(
                    "“{uid}” kimlikli nesne kolon, kiriş, döşeme, çatı, korkuluk ya da merdiven değil; eleman tipi yalnız bunlara verilir."
                ),
                &path,
            ));
        }
        if locked(doc, e) {
            return Err(locked_stop(doc, e, &path));
        }
        out.push((at, slot, e, uid.clone()));
    }
    Ok(out)
}

/// The type an element follows, from `types`; `element_not_typed` when it follows none, `element_type_not_found` when
/// its type is gone.
fn its_type<'t>(
    types: &'t [ElementType],
    e: &Entity,
    uid: &str,
    path: &str,
) -> Result<(usize, &'t ElementType), Stop> {
    let Some(link) = type_link(e) else {
        return Err(fail(
            codes::ELEMENT_NOT_TYPED,
            format!("“{uid}” kimlikli eleman bir proje tipini izlemiyor."),
            path,
        ));
    };
    types.iter().position(|t| t.id == link.id).map(|i| (i, &types[i])).ok_or_else(|| {
        fail(
            codes::ELEMENT_TYPE_NOT_FOUND,
            format!("“{uid}” kimlikli elemanın izlediği “{}” tipi projede yok; elemanı tipten ayırın.", link.id),
            path,
        )
    })
}

/// The fields `fields` names checked against `kind`'s: each a type field, given once.
fn fields_of(kind: ElementTypeKind, fields: &[String]) -> Result<(), Stop> {
    for (k, key) in fields.iter().enumerate() {
        if kind.field(key).is_none() {
            let known: Vec<&str> = kind.fields().iter().map(|f| f.key).collect();
            return Err(invalid(
                &format!("operation.fields[{k}]"),
                format!(
                    "“{key}” {} tipinin alanı değil; alanlar: {}.",
                    kind.words(),
                    known.join(", ")
                ),
            ));
        }
    }
    Ok(())
}

fn check(doc: &Document, input: &ElementTypesEdit) -> Result<Work, Stop> {
    let mut list = doc.element_types().to_vec();
    let mut work = Work {
        types: Vec::new(),
        id: None,
        updates: Vec::new(),
        report: ElementTypeReport::default(),
    };
    match &input.operation {
        ElementTypesOperation::Define {
            name,
            id,
            element,
            copy_of,
            from,
            values,
        } => {
            name_ok(name, "operation.name")?;
            let mut kind = *element;
            let mut base: Map<String, Value> = Map::new();
            let mut drawn: Option<(Slot, Entity, String)> = None;
            if copy_of.is_some() && from.is_some() {
                return Err(invalid(
                    "operation.from",
                    "Yeni tip ya bir tipin kopyasıdır (copyOf) ya da çizilmiş bir elemandan (from); ikisi birden verilmez.".into(),
                ));
            }
            if let Some(key) = copy_of {
                let i = find(&list, key, "operation.copyOf")?;
                base = list[i].values.clone();
                if kind.is_some_and(|k| k != list[i].kind) {
                    return Err(fail(
                        codes::ELEMENT_TYPE_KIND,
                        format!(
                            "“{}” bir {} tipi; yeni tipin türü ({}) onunla aynı olmalı.",
                            list[i].name,
                            list[i].kind.words(),
                            kind.map_or("", ElementTypeKind::words)
                        ),
                        "operation.element",
                    ));
                }
                kind = Some(list[i].kind);
            }
            if let Some(uid) = from {
                let one = std::slice::from_ref(uid);
                let found = elements(doc, one, "Tipe çevrilecek eleman verilmedi.")
                    .map_err(|s| s.at("operation.from"))?;
                let (_, slot, e, uid) = &found[0];
                let Some((k, values)) = element_type_values(e) else {
                    return Err(fail(
                        codes::ELEMENT_TYPE_KIND,
                        format!("“{uid}” kimlikli nesnenin tipi olmaz."),
                        "operation.from",
                    ));
                };
                if kind.is_some_and(|x| x != k) {
                    return Err(fail(
                        codes::ELEMENT_TYPE_KIND,
                        format!(
                            "“{uid}” kimlikli eleman bir {}; yeni tipin türü ({}) onunla aynı olmalı.",
                            k.words(),
                            kind.map_or("", ElementTypeKind::words)
                        ),
                        "operation.element",
                    ));
                }
                kind = Some(k);
                base = values;
                drawn = Some((*slot, (*e).clone(), uid.clone()));
            }
            let Some(kind) = kind else {
                return Err(invalid(
                    "operation.element",
                    "Tipin türü verilmeli (element: column, beam, slab, roof, railing ya da stair), ya da bir tipin kopyası (copyOf) ya da çizilmiş bir eleman (from).".into(),
                ));
            };
            for (k, v) in values {
                if v.is_null() {
                    base.remove(k);
                } else {
                    base.insert(k.clone(), v.clone());
                }
            }
            let new_id = match id {
                Some(id) => id.clone(),
                None => free_id(&list, name),
            };
            if let Some((why, path)) = taken(&list, kind, &new_id, name, None) {
                return Err(fail(codes::ELEMENT_TYPE_EXISTS, why, path));
            }
            let mut t = ElementType {
                id: new_id.clone(),
                name: name.clone(),
                kind,
                version: 1,
                values: base,
            };
            if let Some((at, why)) = t.problem() {
                return Err(invalid(&format!("operation.{at}"), why));
            }
            canonical(&mut t);
            if let Some((slot, e, uid)) = drawn {
                let typed =
                    follow_type(&e, &t, &[]).map_err(|(_, why)| invalid("operation.from", why))?;
                work.report.applied.push(uid);
                work.updates.push((slot, typed));
            }
            list.push(t);
            work.id = Some(new_id);
        }
        ElementTypesOperation::Update { kind: key, values } => {
            let i = find(&list, key, "operation.type")?;
            let old = list[i].clone();
            let mut t = old.clone();
            for (k, v) in values {
                if v.is_null() {
                    t.values.remove(k);
                } else {
                    t.values.insert(k.clone(), v.clone());
                }
            }
            if let Some((at, why)) = t.problem() {
                return Err(invalid(&format!("operation.{at}"), why));
            }
            canonical(&mut t);
            if t.values == old.values {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!(
                        "“{}” tipinin değerleri değişmedi; değişecek bir değer verin.",
                        old.name
                    ),
                    "operation.values",
                ));
            }
            t.version += 1;
            let fields = changed_fields(&old, &t);
            follow(doc, &t, &fields, input.on_refused, None, &mut work)?;
            work.id = Some(t.id.clone());
            list[i] = t;
        }
        ElementTypesOperation::Rename { kind: key, name } => {
            name_ok(name, "operation.name")?;
            let i = find(&list, key, "operation.type")?;
            if list[i].name == *name {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!("Tipin adı zaten “{name}”."),
                    "operation.name",
                ));
            }
            if let Some((why, path)) = taken(&list, list[i].kind, &list[i].id, name, Some(i)) {
                return Err(fail(codes::ELEMENT_TYPE_EXISTS, why, path));
            }
            list[i].name = name.clone();
            // A column following it takes the name as its type, unless that is its own (its exception).
            if list[i].kind == ElementTypeKind::Column {
                for (slot, mut e) in followers(doc, &list[i].id) {
                    let own =
                        type_link(&e).is_some_and(|l| l.overrides.iter().any(|k| k == "type"));
                    if own || locked(doc, &e) {
                        continue;
                    }
                    if let Entity::Column(c) = &mut e {
                        c.column.kind = Some(name.clone());
                    }
                    work.report.applied.push(uid_of(doc, slot));
                    work.updates.push((slot, e));
                }
            }
            work.id = Some(list[i].id.clone());
        }
        ElementTypesOperation::Remove { kind: key, detach } => {
            let i = find(&list, key, "operation.type")?;
            let those = followers(doc, &list[i].id);
            if !those.is_empty() && !detach {
                return Err(fail(
                    codes::ELEMENT_TYPE_IN_USE,
                    format!(
                        "“{}” tipini {} eleman izliyor; önce onları tipten ayırın ya da silerken ayırmayı seçin (detach).",
                        list[i].name,
                        those.len()
                    ),
                    "operation.type",
                ));
            }
            if let Some((_, e)) = those.iter().find(|(_, e)| locked(doc, e)) {
                return Err(locked_stop(doc, e, "operation.type"));
            }
            for (slot, mut e) in those {
                if let Some(a) = e.arch_info_mut() {
                    a.element_type = None;
                }
                work.report.applied.push(uid_of(doc, slot));
                work.updates.push((slot, e));
            }
            work.id = Some(list[i].id.clone());
            list.remove(i);
        }
        ElementTypesOperation::Assign { kind: key, uids } => {
            let i = find(&list, key, "operation.type")?;
            let t = &list[i];
            for (at, slot, e, uid) in elements(doc, uids, "Tipe bağlanacak eleman verilmedi.")? {
                let path = format!("uids[{at}]");
                if ElementTypeKind::of(e) != Some(t.kind) {
                    return Err(fail(
                        codes::ELEMENT_TYPE_KIND,
                        format!(
                            "“{}” bir {} tipi; “{uid}” kimlikli eleman {}.",
                            t.name,
                            t.kind.words(),
                            ElementTypeKind::of(e).map_or("", ElementTypeKind::words)
                        ),
                        &path,
                    ));
                }
                let typed = follow_type(e, t, &[])
                    .map_err(|(_, why)| fail(codes::ELEMENT_TYPE_REFUSED, why, &path))?;
                work.report.applied.push(uid);
                work.updates.push((slot, typed));
            }
            work.id = Some(t.id.clone());
        }
        ElementTypesOperation::Detach { uids } => {
            for (at, slot, e, uid) in elements(doc, uids, "Tipten ayrılacak eleman verilmedi.")? {
                if type_link(e).is_none() {
                    return Err(fail(
                        codes::ELEMENT_NOT_TYPED,
                        format!("“{uid}” kimlikli eleman bir proje tipini izlemiyor."),
                        &format!("uids[{at}]"),
                    ));
                }
                let mut e = e.clone();
                if let Some(a) = e.arch_info_mut() {
                    a.element_type = None;
                }
                work.report.applied.push(uid);
                work.updates.push((slot, e));
            }
        }
        ElementTypesOperation::Reset { uids, fields } => {
            for (at, slot, e, uid) in elements(doc, uids, "Tipe döndürülecek eleman verilmedi.")?
            {
                let path = format!("uids[{at}]");
                let (_, t) = its_type(&list, e, &uid, &path)?;
                fields_of(t.kind, fields)?;
                let link = type_link(e)
                    .cloned()
                    .unwrap_or_else(|| kentos_commands::TypeLink {
                        id: t.id.clone(),
                        version: t.version,
                        overrides: Vec::new(),
                    });
                let keep: Vec<String> = if fields.is_empty() {
                    Vec::new()
                } else {
                    link.overrides
                        .iter()
                        .filter(|k| !fields.contains(k))
                        .cloned()
                        .collect()
                };
                if keep == link.overrides && link.version >= t.version {
                    return Err(fail(
                        codes::NOTHING_TO_SET,
                        format!(
                            "“{uid}” kimlikli eleman zaten “{}” tipinin son sürümünde ve bu alanlarda tipi izliyor.",
                            t.name
                        ),
                        &path,
                    ));
                }
                let typed = follow_type(e, t, &keep)
                    .map_err(|(_, why)| fail(codes::ELEMENT_TYPE_REFUSED, why, &path))?;
                work.report.applied.push(uid);
                work.updates.push((slot, typed));
            }
        }
        ElementTypesOperation::Push { uid, fields } => {
            let one = std::slice::from_ref(uid);
            let found = elements(doc, one, "Değerleri tipe uygulanacak eleman verilmedi.")
                .map_err(|s| s.at("operation.uid"))?;
            let (_, slot, e, uid) = &found[0];
            let (i, t) = its_type(&list, e, uid, "operation.uid")?;
            fields_of(t.kind, fields).map_err(|s| s.at("operation.fields"))?;
            let link = type_link(e)
                .cloned()
                .unwrap_or_else(|| kentos_commands::TypeLink {
                    id: t.id.clone(),
                    version: t.version,
                    overrides: Vec::new(),
                });
            if let Some(f) = fields
                .iter()
                .filter_map(|k| t.kind.field(k))
                .find(|f| f.named)
            {
                return Err(invalid(
                    "operation.fields",
                    format!(
                        "{} tipin adıdır; tipe uygulanmaz, tipi yeniden adlandırın.",
                        f.label
                    ),
                ));
            }
            let pushed: Vec<String> = if fields.is_empty() {
                link.overrides
                    .iter()
                    .filter(|k| t.kind.field(k).is_some_and(|f| !f.named))
                    .cloned()
                    .collect()
            } else {
                fields.clone()
            };
            if pushed.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!(
                        "“{uid}” kimlikli elemanın “{}” tipinden ayrılan değeri yok; tipe uygulanacak bir şey yok.",
                        t.name
                    ),
                    "operation.uid",
                ));
            }
            let old = t.clone();
            let mut next = t.clone();
            for key in &pushed {
                let Some(f) = next.kind.field(key) else {
                    continue;
                };
                match kentos_commands::element_value(e, f) {
                    Some(v) => {
                        next.values.insert(key.clone(), v);
                    }
                    None => {
                        next.values.remove(key);
                    }
                }
            }
            if let Some((at, why)) = next.problem() {
                return Err(invalid(&format!("operation.{at}"), why));
            }
            canonical(&mut next);
            if next.values == old.values {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!(
                        "“{uid}” kimlikli elemanın bu değerleri zaten “{}” tipinin.",
                        t.name
                    ),
                    "operation.uid",
                ));
            }
            next.version += 1;
            let fields = changed_fields(&old, &next);
            let keep: Vec<String> = link
                .overrides
                .iter()
                .filter(|k| !pushed.contains(k))
                .cloned()
                .collect();
            follow(
                doc,
                &next,
                &fields,
                input.on_refused,
                Some((*slot, (*e).clone(), keep)),
                &mut work,
            )?;
            work.id = Some(next.id.clone());
            list[i] = next;
        }
        ElementTypesOperation::Refresh { uids } => {
            let mut those: Vec<(String, Slot, Entity, String)> = Vec::new();
            if uids.is_empty() {
                for e in doc.entities() {
                    let Some(link) = type_link(e) else { continue };
                    if list
                        .iter()
                        .any(|t| t.id == link.id && link.version < t.version)
                    {
                        let slot = Slot(e.base().id);
                        those.push(("operation".into(), slot, e.clone(), uid_of(doc, slot)));
                    }
                }
            } else {
                for (at, slot, e, uid) in
                    elements(doc, uids, "Tipi yeniden uygulanacak eleman verilmedi.")?
                {
                    let path = format!("uids[{at}]");
                    let (_, t) = its_type(&list, e, &uid, &path)?;
                    if type_link(e).is_some_and(|l| l.version < t.version) {
                        those.push((path, slot, e.clone(), uid));
                    }
                }
            }
            if those.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Tipinin eski sürümünde eleman yok; hepsi son sürümde.".into(),
                    if uids.is_empty() { "operation" } else { "uids" },
                ));
            }
            for (_, slot, e, uid) in those {
                let Some(link) = type_link(&e).cloned() else {
                    continue;
                };
                let Some(t) = list.iter().find(|t| t.id == link.id) else {
                    continue;
                };
                if locked(doc, &e) {
                    work.report.skipped.push(ElementLeft {
                        uid,
                        why: ElementLeftWhy::Locked,
                        message: "Kilitli katmanda; eski sürümde kaldı. Kilidi açıp tipi yeniden uygulayın.".into(),
                    });
                    continue;
                }
                match follow_type(&e, t, &link.overrides) {
                    Ok(typed) => {
                        work.report.applied.push(uid);
                        work.updates.push((slot, typed));
                    }
                    Err((_, why)) if input.on_refused == ElementTypeRefused::Skip => {
                        work.report.refused.push(ElementLeft {
                            uid,
                            why: ElementLeftWhy::Invalid,
                            message: why,
                        });
                    }
                    Err((_, why)) => {
                        return Err(fail(
                            codes::ELEMENT_TYPE_REFUSED,
                            format!(
                                "“{uid}” kimlikli eleman “{}” tipinin son sürümüyle geçersiz oluyor: {why} Hiçbir şey yazılmadı.",
                                t.name
                            ),
                            "operation",
                        ));
                    }
                }
            }
        }
    }
    if let Some((at, why)) = element_types_problem(&list) {
        return Err(invalid(&at, why));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    work.types = list;
    Ok(work)
}

/// The report's warnings: the elements that kept their own value of a changed field, those left behind.
fn warnings(report: &ElementTypeReport) -> Vec<CommandWarning> {
    let mut out = Vec::new();
    if !report.kept.is_empty() {
        out.push(CommandWarning {
            code: codes::ELEMENT_TYPE_KEPT.into(),
            message: format!(
                "{} eleman değişen alanda kendi değerini (örnek istisnası) korudu; öbür alanlarda tipi izliyor.",
                report.kept.len()
            ),
            path: Some("report.kept".into()),
        });
    }
    let left = report.skipped.len() + report.refused.len();
    if left > 0 {
        out.push(CommandWarning {
            code: codes::ELEMENT_TYPE_SKIPPED.into(),
            message: format!(
                "{left} eleman güncellenmedi (kilitli katmanda {}, geçersiz olan {}); eski sürümde kaldı. Tipi yeniden \
                 uygula ile alırlar.",
                report.skipped.len(),
                report.refused.len()
            ),
            path: Some("report.skipped".into()),
        });
    }
    out
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ElementTypesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ElementTypesEdit,
) -> CommandResult<ElementTypesEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: the type window's preview of the elements a change takes.
pub fn plan_in(doc: &Document, input: &ElementTypesEdit) -> CommandResult<ElementTypesEdited> {
    match check(doc, input) {
        Ok(work) => CommandResult::Completed {
            warnings: warnings(&work.report),
            output: ElementTypesEdited {
                types: work.types,
                id: work.id,
                report: work.report,
                revision: doc.revision().to_string(),
            },
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ElementTypesEdit,
) -> CommandResult<ElementTypesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ELEMENT_TYPES_EDIT,
        kentos_commands::CAD_ELEMENT_TYPES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ElementTypesEdit,
) -> CommandResult<ElementTypesEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let result = doc.transact(STEP, |d| {
        d.set_element_types(work.types.clone(), STEP)?;
        d.update_many(work.updates.clone(), STEP);
        Ok::<(), kentos_domain::Refusal>(())
    });
    if let Err(e) = result {
        return CommandResult::Failed {
            error: error(codes::INVALID_ELEMENT_TYPE, e.0, Some("operation".into())),
        };
    }
    CommandResult::Completed {
        warnings: warnings(&work.report),
        output: ElementTypesEdited {
            types: work.types,
            id: work.id,
            report: work.report,
            revision: doc.revision().to_string(),
        },
    }
}
