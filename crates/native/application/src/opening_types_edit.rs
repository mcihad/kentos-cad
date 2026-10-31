//! `cad.openingtypes.edit` v1 (docs/adr/0772 §1, §4): the project's door and window types made, changed, renamed,
//! removed, taken from an office library and given as one, as one undo step “Açıklık tipi” with every opening following
//! a changed type. Its plan is the preview: the table and every opening's fate, nothing written. The shared cases are
//! `fixtures/commands/v1/cad.openingtypes.edit.json` (`scripts/fixtures/opening_types_cases.py`).

use kentos_commands::{
    CommandResult, Entity, OPENING_TYPES_FORMAT, OPENING_TYPES_FORMAT_VERSION, OpeningEntity,
    OpeningForm, OpeningType, OpeningTypeConflict, OpeningTypeReport, OpeningTypesEdit,
    OpeningTypesEdited, OpeningTypesFile, OpeningTypesOperation, builtin_opening_types,
    opening_type_id_of, opening_type_id_ok,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::opening_types::{self as types, Found, invalid};
use crate::openings::fail;

pub const STEP: &str = "Açıklık tipi";

/// What the command writes: the table after it, the openings that follow, the report, an export's library.
struct Work {
    types: Vec<OpeningType>,
    updates: Vec<(Slot, OpeningEntity)>,
    report: OpeningTypeReport,
    library: Option<OpeningTypesFile>,
}

fn name_ok(name: &str, path: &str) -> Result<(), Stop> {
    if name.trim().is_empty()
        || name.trim() != name
        || name.chars().count() > 128
        || name.chars().any(char::is_control)
    {
        return Err(invalid(
            path,
            format!(
                "“{name}” tip adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 128 harf."
            ),
        ));
    }
    Ok(())
}

fn taken(types: &[OpeningType], id: &str, name: &str, but: Option<usize>) -> Option<String> {
    let folded = name.to_lowercase();
    types
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != but)
        .find_map(|(_, t)| {
            if t.id == id {
                Some(format!(
                    "Projede “{id}” kimlikli bir tip (“{}”) zaten var.",
                    t.name
                ))
            } else if t.name.to_lowercase() == folded {
                Some(format!(
                    "Projede “{}” adında bir tip zaten var; başka bir ad verin.",
                    t.name
                ))
            } else {
                None
            }
        })
}

/// A free id for `name` in `types`: from its name, then with `-2`, `-3` ….
fn free_id(types: &[OpeningType], name: &str) -> String {
    let base = opening_type_id_of(name);
    let mut id = base.clone();
    let mut n = 2;
    while types.iter().any(|t| t.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

fn check(doc: &Document, input: &OpeningTypesEdit) -> Result<Work, Stop> {
    let mut list = doc.opening_types().to_vec();
    let mut work = Work {
        types: Vec::new(),
        updates: Vec::new(),
        report: OpeningTypeReport::default(),
        library: None,
    };
    match &input.operation {
        OpeningTypesOperation::Define {
            name,
            id,
            copy_of,
            form,
            change,
        } => {
            name_ok(name, "operation.name")?;
            let id = match id {
                Some(id) if !opening_type_id_ok(id) => {
                    return Err(invalid(
                        "operation.id",
                        format!(
                            "“{id}” tip kimliği olamaz: küçük harf, rakam ve “-” ile en çok 64 harf, “kentos-” ile başlamadan."
                        ),
                    ));
                }
                Some(id) => id.clone(),
                None => free_id(&list, name),
            };
            if let Some(why) = taken(&list, &id, name, None) {
                return Err(fail(codes::OPENING_TYPE_EXISTS, why, "operation.name"));
            }
            let mut base = match copy_of {
                Some(key) => match types::find(doc, key, "operation.copyOf")? {
                    Found::Project(t) => OpeningType {
                        id: id.clone(),
                        name: name.clone(),
                        version: 1,
                        source: None,
                        ..*t
                    },
                    Found::Builtin(t) => types::of_builtin(&t, id.clone(), name.clone()),
                },
                None => {
                    let form = form.unwrap_or(OpeningForm::Door);
                    let t = builtin_opening_types()
                        .into_iter()
                        .find(|t| t.form == form)
                        .unwrap_or_else(|| builtin_opening_types().remove(0));
                    types::of_builtin(&t, id.clone(), name.clone())
                }
            };
            if let Some(f) = form
                && copy_of.is_some()
                && *f != base.form
            {
                return Err(fail(
                    codes::OPENING_TYPE_FORM,
                    format!(
                        "Kopyalanan tip {}; biçimi {} olamaz. Biçimi verilmeden kopyalayın ya da o biçimde bir tip seçin.",
                        base.form.label().to_lowercase(),
                        f.label().to_lowercase()
                    ),
                    "operation.form",
                ));
            }
            base.form = form.unwrap_or(base.form);
            let made = types::changed(&base, change, "operation.change")?;
            list.push(made);
        }
        OpeningTypesOperation::Update { kind, change } => {
            let old = types::find_project(doc, kind, "operation.type")?;
            if change.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Tipin değişecek bir alanı verilmedi: kaba boşluk ya da ölçü, kanat, açılış, açı, kasa, denizlik, lento, derinlik ya da yapı verin.".into(),
                    "operation.change",
                ));
            }
            let mut new = types::changed(&old, change, "operation.change")?;
            let fields = new.differs_from(&old);
            if fields.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!(
                        "“{}” tipi zaten bu değerlerde; hiçbir şey değişmiyor.",
                        old.name
                    ),
                    "operation.change",
                ));
            }
            new.version = old.version + 1;
            let followed = types::follow(doc, &new, &fields, input.on_refused, None)?;
            work.updates = followed.updates;
            work.report = followed.report;
            if let Some(t) = list.iter_mut().find(|t| t.id == new.id) {
                *t = new;
            }
        }
        OpeningTypesOperation::Rename { kind, name } => {
            let old = types::find_project(doc, kind, "operation.type")?;
            name_ok(name, "operation.name")?;
            let i = list.iter().position(|t| t.id == old.id).unwrap_or(0);
            if let Some(why) = taken(&list, "\u{0}", name, Some(i)) {
                return Err(fail(codes::OPENING_TYPE_EXISTS, why, "operation.name"));
            }
            if old.name == *name {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!("Tipin adı zaten “{name}”."),
                    "operation.name",
                ));
            }
            list[i].name.clone_from(name);
        }
        OpeningTypesOperation::Remove { kind, detach } => {
            let old = types::find_project(doc, kind, "operation.type")?;
            let followers = types::followers(doc, &old.id);
            if !followers.is_empty() && !detach {
                return Err(fail(
                    codes::OPENING_TYPE_IN_USE,
                    format!(
                        "“{}” tipini {} açıklık izliyor; silmek için onları tipten ayırın (detach: true) ya da önce başka tip verin.",
                        old.name,
                        followers.len()
                    ),
                    "operation.type",
                ));
            }
            for (slot, mut e) in followers {
                if doc.layers().is_locked(&e.base.layer_id) {
                    return Err(fail(
                        codes::LAYER_LOCKED,
                        format!(
                            "Tipi izleyen bir açıklık kilitli katmanda; tipten ayrılamaz. Kilidi açın ya da tipi silmeyin (“{}”).",
                            doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
                        ),
                        "operation.type",
                    ));
                }
                detach_fields(&mut e.opening);
                work.report
                    .applied
                    .push(doc.uid(slot).map(|u| u.to_string()).unwrap_or_default());
                work.updates.push((slot, e));
            }
            list.retain(|t| t.id != old.id);
        }
        OpeningTypesOperation::Import {
            library,
            on_conflict,
        } => {
            if library.format != OPENING_TYPES_FORMAT
                || library.version != OPENING_TYPES_FORMAT_VERSION
            {
                return Err(fail(
                    codes::IMPORT_UNREADABLE,
                    format!(
                        "Dosya bir KentOS kapı ve pencere tipleri kitaplığı değil (“{}” sürüm {}); “{OPENING_TYPES_FORMAT}” sürüm {OPENING_TYPES_FORMAT_VERSION} beklenir.",
                        library.format, library.version
                    ),
                    "operation.library",
                ));
            }
            name_ok(&library.library, "operation.library.library")?;
            if library.types.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Kitaplıkta tip yok.".into(),
                    "operation.library.types",
                ));
            }
            for (k, t) in library.types.iter().enumerate() {
                let path = format!("operation.library.types[{k}]");
                let mut t = t.clone();
                t.source = Some(library.library.clone());
                if let Some((field, words)) = t.problem() {
                    return Err(invalid(&format!("{path}.{field}"), words));
                }
                let same = list.iter().position(|o| o.id == t.id);
                let clash = taken(&list, &t.id, &t.name, None);
                match (clash, on_conflict) {
                    (None, _) => list.push(t),
                    (Some(_), OpeningTypeConflict::Skip) => {}
                    (Some(why), OpeningTypeConflict::Refuse) => {
                        return Err(fail(
                            codes::OPENING_TYPE_EXISTS,
                            why,
                            &format!("{path}.name"),
                        ));
                    }
                    (Some(why), OpeningTypeConflict::Replace) => {
                        let Some(i) = same.filter(|i| list[*i].source == t.source) else {
                            return Err(fail(
                                codes::OPENING_TYPE_EXISTS,
                                format!(
                                    "{why} Yalnız aynı kitaplıktan, aynı kimlikli tip değiştirilir."
                                ),
                                &format!("{path}.name"),
                            ));
                        };
                        let old = list[i].clone();
                        if let Some(other) = taken(&list, "\u{0}", &t.name, Some(i)) {
                            return Err(fail(
                                codes::OPENING_TYPE_EXISTS,
                                other,
                                &format!("{path}.name"),
                            ));
                        }
                        if old.form != t.form {
                            return Err(fail(
                                codes::OPENING_TYPE_FORM,
                                format!(
                                    "Kitaplıktaki “{}” tipinin biçimi projedekinden farklı; değiştirilemez.",
                                    t.name
                                ),
                                &format!("{path}.form"),
                            ));
                        }
                        let fields = t.differs_from(&old);
                        let mut new = t.clone();
                        new.version = old.version + 1;
                        if fields.is_empty() {
                            new.version = old.version;
                        } else {
                            let followed =
                                types::follow(doc, &new, &fields, input.on_refused, None)?;
                            work.updates.extend(followed.updates);
                            let r = followed.report;
                            work.report.applied.extend(r.applied);
                            work.report.kept.extend(r.kept);
                            work.report.skipped.extend(r.skipped);
                            work.report.refused.extend(r.refused);
                            for f in r.fields {
                                if !work.report.fields.contains(&f) {
                                    work.report.fields.push(f);
                                }
                            }
                        }
                        list[i] = new;
                    }
                    (Some(_), OpeningTypeConflict::Rename) => {
                        let mut name = t.name.clone();
                        let mut n = 2;
                        while taken(&list, "\u{0}", &name, None).is_some() {
                            name = format!("{} ({n})", t.name);
                            n += 1;
                        }
                        t.id = free_id(&list, &name);
                        t.name = name;
                        list.push(t);
                    }
                }
            }
            work.report.fields.sort();
        }
        OpeningTypesOperation::Export {
            library,
            types: keys,
        } => {
            name_ok(library, "operation.library")?;
            let chosen: Vec<OpeningType> = if keys.is_empty() {
                list.clone()
            } else {
                keys.iter()
                    .enumerate()
                    .map(|(i, k)| types::find_project(doc, k, &format!("operation.types[{i}]")))
                    .collect::<Result<_, _>>()?
            };
            if chosen.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Projede kitaplığa yazılacak tip yok.".into(),
                    "operation.types",
                ));
            }
            work.library = Some(OpeningTypesFile {
                format: OPENING_TYPES_FORMAT.into(),
                version: OPENING_TYPES_FORMAT_VERSION,
                library: library.clone(),
                types: chosen,
            });
        }
    }
    if let Some((field, words)) = kentos_commands::opening_types_problem(&list) {
        return Err(invalid(&field, words));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    work.types = list;
    Ok(work)
}

/// An opening left with its values, following nothing.
pub(crate) fn detach_fields(f: &mut kentos_commands::OpeningFields) {
    f.style = None;
    f.type_version = None;
    f.overrides.clear();
}

pub fn validate(cx: &ExecutionContext<'_>, input: &OpeningTypesEdit) -> CommandResult<()> {
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
    input: &OpeningTypesEdit,
) -> CommandResult<OpeningTypesEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: the type window's preview of the openings a change takes.
pub fn plan_in(doc: &Document, input: &OpeningTypesEdit) -> CommandResult<OpeningTypesEdited> {
    match check(doc, input) {
        Ok(work) => CommandResult::Completed {
            warnings: types::warnings(&work.report),
            output: OpeningTypesEdited {
                types: work.types,
                report: work.report,
                library: work.library,
                revision: doc.revision().to_string(),
            },
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: OpeningTypesEdit,
) -> CommandResult<OpeningTypesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_OPENING_TYPES_EDIT,
        kentos_commands::CAD_OPENING_TYPES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: OpeningTypesEdit,
) -> CommandResult<OpeningTypesEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    if work.library.is_none() {
        let result = doc.transact(STEP, |d| {
            d.set_opening_types(work.types.clone(), STEP)?;
            d.update_many(
                work.updates
                    .iter()
                    .map(|(s, e)| (*s, Entity::Opening(e.clone())))
                    .collect(),
                STEP,
            );
            Ok::<(), kentos_domain::Refusal>(())
        });
        if let Err(e) = result {
            return CommandResult::Failed {
                error: error(codes::INVALID_OPENING_TYPE, e.0, Some("operation".into())),
            };
        }
    }
    CommandResult::Completed {
        warnings: types::warnings(&work.report),
        output: OpeningTypesEdited {
            types: work.types,
            report: work.report,
            library: work.library,
            revision: doc.revision().to_string(),
        },
    }
}
