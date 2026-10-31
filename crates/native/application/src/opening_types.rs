//! What the door and window type commands share (docs/adr/0772): a type named, a type's change, and the openings that
//! follow a type taking its new values. An opening follows a project type when its `style` is the type's id and it
//! carries the version it took (`typeVersion`); a field it keeps as its own (`overrides`) stays. One on a locked layer
//! stays at the older version and is skipped; one the change does not fit is refused, or with `skip` left behind.

use kentos_commands::{
    Entity, OpeningEntity, OpeningFields, OpeningKept, OpeningLeft, OpeningLeftWhy, OpeningType,
    OpeningTypeChange, OpeningTypeDef, OpeningTypeField, OpeningTypeRefused, OpeningTypeReport,
    builtin_opening_types, rough_of,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::checks::{self, Stop};
use crate::codes;
use crate::openings::{self, fail};

/// A type an input names: the project's (followed), else KentOS's (copied once).
pub(crate) enum Found {
    Project(Box<OpeningType>),
    Builtin(OpeningTypeDef),
}

/// The type `key` names: the project's by id, then name (case aside), then KentOS's; `opening_type_not_found` with the
/// names there are.
pub(crate) fn find(doc: &Document, key: &str, path: &str) -> Result<Found, Stop> {
    if let Some(t) = project_type(doc, key) {
        return Ok(Found::Project(Box::new(t)));
    }
    let folded = key.trim().to_lowercase();
    let all = builtin_opening_types();
    if let Some(t) = all
        .iter()
        .find(|t| t.id == key)
        .or_else(|| all.iter().find(|t| t.name.to_lowercase() == folded))
    {
        return Ok(Found::Builtin(t.clone()));
    }
    Err(not_found(doc, key, path))
}

fn not_found(doc: &Document, key: &str, path: &str) -> Stop {
    let mut names: Vec<String> = doc.opening_types().iter().map(|t| t.name.clone()).collect();
    names.extend(builtin_opening_types().into_iter().map(|t| t.name));
    fail(
        codes::OPENING_TYPE_NOT_FOUND,
        format!(
            "“{key}” adında ya da kimliğinde kapı ve pencere tipi yok. Var olanlar: {}.",
            names.join(", ")
        ),
        path,
    )
}

/// The project's type `key` names, by id then name (case aside).
pub(crate) fn project_type(doc: &Document, key: &str) -> Option<OpeningType> {
    let folded = key.trim().to_lowercase();
    let types = doc.opening_types();
    types
        .iter()
        .find(|t| t.id == key)
        .or_else(|| types.iter().find(|t| t.name.to_lowercase() == folded))
        .cloned()
}

/// The project's type `key` names; `opening_type_not_found` when it has none.
pub(crate) fn find_project(doc: &Document, key: &str, path: &str) -> Result<OpeningType, Stop> {
    project_type(doc, key).ok_or_else(|| {
        let names: Vec<String> = doc.opening_types().iter().map(|t| t.name.clone()).collect();
        fail(
            codes::OPENING_TYPE_NOT_FOUND,
            if names.is_empty() {
                format!("Projede “{key}” tipi yok; projede hiç kapı ve pencere tipi yok. Önce bir tip tanımlayın.")
            } else {
                format!("Projede “{key}” adında ya da kimliğinde tip yok. Var olanlar: {}.", names.join(", "))
            },
            path,
        )
    })
}

/// A KentOS type as a project type's starting values.
pub(crate) fn of_builtin(t: &OpeningTypeDef, id: String, name: String) -> OpeningType {
    OpeningType {
        id,
        name,
        version: 1,
        source: None,
        form: t.form,
        width: t.width,
        leaves: t.leaves,
        operation: t.operation,
        swing: t.swing,
        show_swing: true,
        frame: t.frame,
        sill: t.sill,
        head: t.head,
        depth: t.depth,
        build: None,
    }
}

/// `invalid_opening_type` at `path` with the type's own words.
pub(crate) fn invalid(path: &str, words: String) -> Stop {
    fail(codes::INVALID_OPENING_TYPE, words, path)
}

/// `t` with `c`'s fields (its version as it was); its numbers finite, its fields a type's.
pub(crate) fn changed(
    t: &OpeningType,
    c: &OpeningTypeChange,
    path: &str,
) -> Result<OpeningType, Stop> {
    for (v, name, what) in [
        (c.width, "width", "Kaba boşluk"),
        (c.nominal.map(|n| n.value), "nominal.value", "Ölçü"),
        (c.swing.flatten(), "swing", "Açılma açısı"),
        (c.frame.flatten(), "frame", "Kasa"),
        (c.sill.flatten(), "sill", "Denizlik"),
        (c.head.flatten(), "head", "Lento"),
        (c.depth.flatten(), "depth", "Derinlik"),
    ] {
        if let Some(x) = v {
            checks::finite(x, what, "Sonlu bir sayı verin.", &format!("{path}.{name}"))?;
        }
    }
    if c.width.is_some() && c.nominal.is_some() {
        return Err(invalid(
            &format!("{path}.nominal"),
            "Kaba boşluk (width) ile ölçü (nominal) birlikte verilmez; birini verin.".into(),
        ));
    }
    let mut n = t.clone();
    let mut f = t.fields();
    if let Some(v) = c.width {
        f.width = v;
    }
    if let Some(v) = c.leaves {
        f.leaves = v;
    }
    if let Some(v) = c.operation {
        f.operation = v;
    }
    if let Some(v) = c.swing {
        f.swing = v;
    }
    if let Some(v) = c.show_swing {
        f.show_swing = v;
    }
    if let Some(v) = c.frame {
        f.frame = v;
    }
    if let Some(v) = c.sill {
        f.sill = v;
    }
    if let Some(v) = c.head {
        f.head = v;
    }
    if let Some(v) = c.depth {
        f.depth = v;
    }
    let mut b = f.build.clone().unwrap_or_default();
    if let Some(given) = &c.build {
        let gv = serde_json::to_value(given).unwrap_or_default();
        if let Some(map) = gv.as_object() {
            for (k, v) in map {
                if let Some(field) = OpeningTypeField::ALL.into_iter().find(|f| f.name() == k) {
                    let mut tmp = f.clone();
                    tmp.build = Some(b.clone());
                    tmp.set_value(field, v)
                        .map_err(|w| invalid(&format!("{path}.build.{k}"), w))?;
                    b = tmp.build.unwrap_or_default();
                }
            }
        }
    }
    for (i, k) in c.unset.iter().enumerate() {
        if !k.in_build() {
            return Err(invalid(
                &format!("{path}.unset[{i}]"),
                format!(
                    "“{}” yapının alanı değil; yalnız yapının alanları kaldırılır.",
                    k.label()
                ),
            ));
        }
        let mut tmp = f.clone();
        tmp.build = Some(b.clone());
        let _ = tmp.set_value(*k, &serde_json::Value::Null);
        b = tmp.build.unwrap_or_default();
    }
    if let Some(nom) = c.nominal {
        b.size_by = Some(nom.by);
        f.width = rough_of(
            nom.by,
            nom.value,
            b.fit.unwrap_or(0.0),
            f.frame.unwrap_or(0.0),
        );
    }
    f.build = (!b.is_empty()).then_some(b);
    if c.width.is_none() && c.nominal.is_none() {
        keep_nominal(t, &mut f);
    }
    n.width = f.width;
    n.leaves = f.leaves;
    n.operation = f.operation;
    n.swing = f.swing;
    n.show_swing = f.show_swing;
    n.frame = f.frame;
    n.sill = f.sill;
    n.head = f.head;
    n.depth = f.depth;
    n.build = f.build;
    if let Some((field, words)) = n.problem() {
        return Err(invalid(&format!("{path}.{field}"), words));
    }
    Ok(n)
}

/// A type sized by its leaf or its frame's outer width keeps that size when its frame or its fitting gap change
/// (docs/adr/0772 §2): its rough opening follows from it. One sized by its rough opening keeps its rough opening.
pub(crate) fn keep_nominal(old: &OpeningType, f: &mut OpeningFields) {
    let by = f.build.as_ref().and_then(|b| b.size_by).unwrap_or_default();
    if by == kentos_commands::OpeningSizeBy::Rough || f.width != old.width {
        return;
    }
    let nominal = old.fields().sizes().nominal;
    let fit = f.build.as_ref().and_then(|b| b.fit).unwrap_or(0.0);
    f.width = rough_of(by, nominal, fit, f.frame.unwrap_or(0.0));
}

/// The level an opening stands on: its wall's.
pub(crate) fn level_of(doc: &Document, o: &OpeningFields) -> Option<String> {
    let slot = Uuid::parse_str(&o.host).ok().and_then(|u| doc.slot_of(u))?;
    doc.get(slot)?.own_level().map(str::to_owned)
}

/// The openings following the type `id`, slot by slot.
pub(crate) fn followers(doc: &Document, id: &str) -> Vec<(Slot, OpeningEntity)> {
    doc.entities()
        .filter_map(|e| match e {
            Entity::Opening(o)
                if o.opening.follows_type() && o.opening.style.as_deref() == Some(id) =>
            {
                Some((Slot(o.base.id), o.clone()))
            }
            _ => None,
        })
        .collect()
}

/// What following a type's change writes: the openings, each its slot, and every opening's fate.
pub(crate) struct Followed {
    pub(crate) updates: Vec<(Slot, OpeningEntity)>,
    pub(crate) report: OpeningTypeReport,
}

fn uid_of(doc: &Document, slot: Slot) -> String {
    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
}

/// The openings following `old` taking `new`'s values (docs/adr/0772 §4): `fields` the ones the change changed; the
/// opening at `own` (Tipe uygula's) takes them, its exceptions of `fields` gone. With `refused` stop, an opening the
/// change does not fit refuses all (`opening_type_refused`); with skip it stays at the older version.
pub(crate) fn follow(
    doc: &Document,
    new: &OpeningType,
    fields: &[OpeningTypeField],
    refused: OpeningTypeRefused,
    own: Option<(Slot, OpeningEntity)>,
) -> Result<Followed, Stop> {
    let mut report = OpeningTypeReport {
        fields: fields.to_vec(),
        ..OpeningTypeReport::default()
    };
    let mut list = followers(doc, &new.id);
    if let Some((slot, e)) = &own
        && let Some(x) = list.iter_mut().find(|(s, _)| s == slot)
    {
        x.1 = e.clone();
    }
    // Each one's new fields; those left behind kept apart.
    let mut next: Vec<(Slot, OpeningEntity, String)> = Vec::new();
    for (slot, e) in list {
        let uid = uid_of(doc, slot);
        if doc.layers().is_locked(&e.base.layer_id) {
            report.skipped.push(OpeningLeft {
                uid,
                why: OpeningLeftWhy::Locked,
                message:
                    "Kilitli katmanda; eski sürümde kaldı. Kilidi açıp tipi yeniden uygulayın."
                        .into(),
            });
            continue;
        }
        let mut f = e.opening.clone();
        let kept: Vec<OpeningTypeField> = fields
            .iter()
            .copied()
            .filter(|k| f.overrides.contains(k))
            .collect();
        f.take_type(new);
        let left = |why: OpeningLeftWhy, message: String| OpeningLeft {
            uid: uid.clone(),
            why,
            message,
        };
        if let Some((_, words)) = f.problem().filter(|(k, _)| !k.starts_with("place")) {
            let l = left(OpeningLeftWhy::Invalid, words);
            if refused == OpeningTypeRefused::Stop {
                return Err(refusal(&l));
            }
            report.refused.push(l);
            continue;
        }
        let placed = match doc.wall_line_of(&f.host) {
            Some(line) => match openings::placed(&f, &line, "") {
                Ok(p) => p,
                Err(stop) => {
                    let e = stop.into_error();
                    let why = if e.code == codes::RECESS_TOO_DEEP || e.code == codes::OPENING_POCKET_THIN {
                        OpeningLeftWhy::Invalid
                    } else {
                        OpeningLeftWhy::OffWall
                    };
                    let l = left(why, e.message);
                    if refused == OpeningTypeRefused::Stop {
                        return Err(refusal(&l));
                    }
                    report.refused.push(l);
                    continue;
                }
            },
            None => kentos_domain::opening_follow::placed_on(&f, None, false),
        };
        if !kept.is_empty() {
            report.kept.push(OpeningKept {
                uid: uid.clone(),
                fields: kept,
            });
        }
        let mut out = e.clone();
        out.opening = placed;
        next.push((slot, out, uid));
    }
    // Clear of the wall's other openings, one at a time: with skip the one in the way stays behind.
    loop {
        let changed: Vec<(Slot, &OpeningFields, String)> = next
            .iter()
            .map(|(s, e, uid)| (*s, &e.opening, uid.clone()))
            .collect();
        match openings::clear_of_others(doc, &changed, &[]) {
            Ok(()) => break,
            Err(stop) => {
                let e = stop.into_error();
                let uid = e.path.clone().unwrap_or_default();
                let l = OpeningLeft {
                    uid: uid.clone(),
                    why: OpeningLeftWhy::Overlap,
                    message: e.message,
                };
                if refused == OpeningTypeRefused::Stop {
                    return Err(refusal(&l));
                }
                report.kept.retain(|k| k.uid != uid);
                next.retain(|(_, _, u)| *u != uid);
                report.refused.push(l);
            }
        }
    }
    report.applied = next.iter().map(|(_, _, u)| u.clone()).collect();
    Ok(Followed {
        updates: next.into_iter().map(|(s, e, _)| (s, e)).collect(),
        report,
    })
}

fn refusal(l: &OpeningLeft) -> Stop {
    fail(
        codes::OPENING_TYPE_REFUSED,
        format!(
            "Tipi izleyen “{}” açıklığı değişikliği alamıyor: {} Hiçbir şey yazılmadı; açıklığı düzeltin, tipten ayırın ya da sığmayanları atlamayı (onRefused: skip) seçin.",
            l.uid, l.message
        ),
        "operation",
    )
}

/// The warnings a report says: openings that kept their own value of a changed field, those left behind.
pub(crate) fn warnings(report: &OpeningTypeReport) -> Vec<kentos_commands::CommandWarning> {
    let mut out = Vec::new();
    if !report.kept.is_empty() {
        out.push(kentos_commands::CommandWarning {
            code: codes::OPENING_TYPE_KEPT.into(),
            message: format!(
                "{} açıklık değişen alanda kendi değerini (örnek istisnası) korudu.",
                report.kept.len()
            ),
            path: None,
        });
    }
    let left = report.skipped.len() + report.refused.len();
    if left > 0 {
        out.push(kentos_commands::CommandWarning {
            code: codes::OPENING_TYPE_SKIPPED.into(),
            message: format!(
                "{left} açıklık güncellenmedi ({} kilitli katmanda, {} sığmadı); eski sürümde kaldı.",
                report.skipped.len(),
                report.refused.len()
            ),
            path: None,
        });
    }
    out
}
