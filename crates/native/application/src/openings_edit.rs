//! `cad.openings.edit` v1 (docs/adr/0341 §4, plan/cad.md `CAD-ARCH-15`, `19`): openings named by their persistent ids
//! changed as one undo step “Açıklığı düzenle”, all or none. Değiştir sets their own fields (Öznitelikler writes
//! through it); Taşı moves them along their walls; Çevir turns the side they open to or their hinge; Başka duvara puts
//! them in another wall; Türü değiştir gives them another type's values; Numarala marks them in their walls' order;
//! Kopyala copies them along their walls; Dağıt plans new copies from count/spacing or jamb margins. The shared cases are `fixtures/commands/v1/cad.openings.edit.json`.
//!
//! An opening that would run off its wall or into another, a field that makes none, a wall that is gone where one is
//! needed: nothing is written. Their jambs follow from their walls in the same step (`kentos_domain::opening_follow`).

use kentos_commands::{
    CommandResult, Entity, OpeningChange, OpeningEntity, OpeningFields, OpeningFlip, OpeningsEdit,
    OpeningsEdited, OpeningsOperation, WallEnd,
};
use kentos_domain::opening_follow;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::opening::{self, at_of_centre};
use kentos_geometry_core::ops::path::nearest_s;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::openings::{self, fail};

#[path = "opening_array.rs"]
mod array;

pub const STEP: &str = "Açıklığı düzenle";

/// What an edit writes: the openings changed (each its slot and where the input names it) and the new ones; Tipe
/// uygula's types and its report (docs/adr/0772), the warnings.
#[derive(Default)]
struct Work {
    updates: Vec<(Slot, OpeningEntity, String)>,
    added: Vec<(OpeningEntity, String)>,
    types: Option<Vec<kentos_commands::OpeningType>>,
    report: Option<kentos_commands::OpeningTypeReport>,
    warnings: Vec<kentos_commands::CommandWarning>,
}

/// Whether the opening is of the type `from` names: its style is the project type's id or KentOS's (by id or name), or
/// the words themselves.
fn of_type(doc: &Document, o: &OpeningFields, from: &str) -> bool {
    match crate::opening_types::project_type(doc, from) {
        Some(t) => o.style.as_deref() == Some(t.id.as_str()),
        None => openings::type_matches(o, from),
    }
}

/// Whether the opening is on `level` (its wall's; none: on none); any when no level is given.
fn on_level(doc: &Document, o: &OpeningFields, level: Option<&Option<String>>) -> bool {
    level.is_none_or(|want| *want == crate::opening_types::level_of(doc, o))
}

/// `o` with `c`'s fields.
fn changed(o: &OpeningFields, c: &OpeningChange) -> OpeningFields {
    let mut f = o.clone();
    if let Some(v) = c.width {
        f.width = v;
    }
    if let Some(v) = c.form {
        f.form = v;
    }
    if let Some(v) = c.leaves {
        f.leaves = v;
    }
    if let Some(v) = c.operation {
        f.operation = v;
    }
    if let Some(v) = c.hand {
        f.hand = v;
    }
    if let Some(v) = c.side {
        f.side = v;
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
    if let Some(v) = c.inset {
        f.inset = v;
    }
    if let Some(v) = &c.number {
        f.number.clone_from(v);
    }
    // Its construction (docs/adr/0772 §3): the fields given set, those unset taken away.
    if let Some(b) = &c.build
        && let Some(map) = serde_json::to_value(b).ok().and_then(|v| v.as_object().cloned())
    {
        for (k, v) in map {
            if let Some(field) = kentos_commands::OpeningTypeField::ALL.into_iter().find(|x| x.name() == k) {
                let _ = f.set_value(field, &v);
            }
        }
    }
    for k in &c.unset {
        if k.in_build() {
            let _ = f.set_value(*k, &serde_json::Value::Null);
        }
    }
    // A form changed keeps its depth and inset in step (docs/adr/0719 §1).
    if c.form.is_some() {
        openings::recess_follows_form(&mut f, c.depth.is_some(), c.inset.is_some());
    }
    f
}

fn finite_numbers(c: &OpeningChange) -> Result<(), Stop> {
    for (v, what, name) in [
        (c.width, "width", "Genişlik"),
        (c.swing.flatten(), "swing", "Açılma açısı"),
        (c.frame.flatten(), "frame", "Kasa"),
        (c.sill.flatten(), "sill", "Denizlik"),
        (c.head.flatten(), "head", "Lento"),
        (c.depth.flatten(), "depth", "Derinlik"),
        (c.inset.flatten(), "inset", "Yüzden uzaklık"),
    ] {
        if let Some(x) = v {
            checks::finite(
                x,
                name,
                "Sonlu bir sayı verin.",
                &format!("operation.{what}"),
            )?;
        }
    }
    Ok(())
}

/// Which openings an operation takes when no ids are given.
type Pick = Box<dyn Fn(&OpeningFields) -> bool>;

/// The openings an input names: the ids given, or with none every opening Türü değiştir's `from` or Numarala's form
/// names.
fn named_uids(doc: &Document, input: &OpeningsEdit) -> Result<Vec<String>, Stop> {
    if !input.uids.is_empty() {
        checks::uids(&input.uids, "Düzenlenecek açıklık verilmedi.")?;
        return Ok(input.uids.clone());
    }
    let pick: Option<Pick> = match &input.operation {
        OpeningsOperation::ReplaceType {
            from: Some(from), level, ..
        } => {
            // A project type's id, KentOS's type by id or name, or the words themselves (docs/adr/0772 §4).
            let id = crate::opening_types::project_type(doc, from).map(|t| t.id);
            let from = from.clone();
            let levels: Option<std::collections::HashMap<String, Option<String>>> = level.as_ref().map(|_| {
                doc.entities()
                    .filter_map(|e| match e {
                        Entity::Opening(o) => {
                            Some((o.opening.host.clone(), crate::opening_types::level_of(doc, &o.opening)))
                        }
                        _ => None,
                    })
                    .collect()
            });
            let level = level.clone();
            Some(Box::new(move |o| {
                let of = match &id {
                    Some(id) => o.style.as_deref() == Some(id.as_str()),
                    None => openings::type_matches(o, &from),
                };
                let here = match (&level, &levels) {
                    (Some(want), Some(map)) => map.get(&o.host).is_some_and(|l| l == want),
                    _ => true,
                };
                of && here
            }))
        }
        OpeningsOperation::Renumber { form, .. } => {
            let form = *form;
            Some(Box::new(move |o| form.is_none_or(|f| o.form == f)))
        }
        _ => None,
    };
    let Some(pick) = pick else {
        checks::uids(&input.uids, "Düzenlenecek açıklık verilmedi.")?;
        return Ok(Vec::new());
    };
    let all: Vec<String> = doc
        .entities()
        .filter(|e| matches!(e, Entity::Opening(o) if pick(&o.opening)))
        .filter_map(|e| doc.uid(Slot(e.base().id)).map(|u| u.to_string()))
        .collect();
    if all.is_empty() {
        let (code, words) = match &input.operation {
            OpeningsOperation::ReplaceType {
                from: Some(from), ..
            } => (
                codes::STYLE_NOT_USED,
                format!("Çizimde “{from}” türünde açıklık yok."),
            ),
            _ => (
                codes::NO_ENTITIES,
                "Çizimde numaralanacak açıklık yok.".to_owned(),
            ),
        };
        return Err(fail(code, words, "operation"));
    }
    Ok(all)
}

fn check(doc: &Document, input: &OpeningsEdit) -> Result<Work, Stop> {
    let uids = named_uids(doc, input)?;
    let found = checks::named(doc, &uids)?;
    let mut list: Vec<(usize, Slot, &OpeningEntity)> = Vec::with_capacity(found.len());
    for (at, slot, e, _) in &found {
        match e {
            Entity::Opening(o) => list.push((*at, *slot, o)),
            _ => {
                return Err(fail(
                    codes::NOT_AN_OPENING,
                    "Nesne kapı, pencere ya da boşluk değil; Açıklığı düzenle yalnız açıklıkları değiştirir."
                        .into(),
                    &format!("uids[{at}]"),
                ));
            }
        }
    }
    let mut work = Work::default();
    // Each opening's new fields; whether its place must stand on its wall (a move or a copy needs its wall).
    let mut next: Vec<(usize, Slot, &OpeningEntity, OpeningFields, bool)> = Vec::new();
    match &input.operation {
        OpeningsOperation::Set(c) => {
            if c == &OpeningChange::default() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Değiştirilecek bir alan verilmedi: genişlik, biçim, kanat, açılış, menteşe, yan, açı, açılım yayı, kasa, denizlik, lento, derinlik, yüzden uzaklık ya da numara verin.".into(),
                    "operation",
                ));
            }
            finite_numbers(c)?;
            for (at, slot, o) in &list {
                let mut f = changed(&o.opening, c);
                // A field of its type set on an opening following it becomes its own (docs/adr/0772 §1).
                if o.opening.follows_type() {
                    if c.form.is_some_and(|v| v != o.opening.form) {
                        return Err(fail(
                            codes::OPENING_TYPE_FORM,
                            "Açıklık bir tipi izliyor; biçim tipindir. Biçimi değiştirmek için önce tipten ayırın ya da başka tip verin.".into(),
                            "operation.form",
                        ));
                    }
                    let own: Vec<_> = kentos_commands::OpeningTypeField::ALL
                        .into_iter()
                        .filter(|k| f.value(*k) != o.opening.value(*k) && !f.overrides.contains(k))
                        .collect();
                    if !own.is_empty() {
                        f.overrides.extend(own.iter().copied());
                        f.overrides.sort();
                        work.warnings.push(kentos_commands::CommandWarning {
                            code: codes::OPENING_TYPE_OVERRIDE.into(),
                            message: format!(
                                "Açıklık tipinin {} alanında artık kendi değerini tutuyor (örnek istisnası); tipe döndürmek için Tipe döndür.",
                                own.iter().map(|k| format!("“{}”", k.label())).collect::<Vec<_>>().join(", ")
                            ),
                            path: Some(format!("uids[{at}]")),
                        });
                    }
                }
                next.push((*at, *slot, o, f, false));
            }
        }
        OpeningsOperation::Move { by, at: to } => {
            if by.is_none() && to.is_none() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Taşımanın uzaklığı verilmedi: duvar boyunca kaydırma (by) ya da yeni konum (at) verin."
                        .into(),
                    "operation",
                ));
            }
            for (v, what) in [(by, "by"), (to, "at")] {
                if let Some(x) = v {
                    checks::finite(
                        *x,
                        "Uzaklık",
                        "Sonlu bir sayı verin.",
                        &format!("operation.{what}"),
                    )?;
                }
            }
            for (at, slot, o) in &list {
                let mut f = o.opening.clone();
                f.at = to.unwrap_or(f.at) + by.unwrap_or(0.0);
                next.push((*at, *slot, o, f, true));
            }
        }
        OpeningsOperation::Distance {
            side,
            face,
            distance,
            to,
        } => {
            checks::finite(*distance, "Mesafe", "Sonlu bir sayı verin.", "operation.distance")?;
            if *distance < 0.0 {
                return Err(fail(
                    codes::INVALID_OPENING,
                    format!("Mesafe {distance} m; sıfır ya da daha büyük olmalı."),
                    "operation.distance",
                ));
            }
            let start = matches!(side, WallEnd::Start);
            for (at, slot, o) in &list {
                let path = format!("uids[{at}]");
                let m = crate::opening_distances::measure(doc, *slot, &o.opening, *face, &path)?;
                let now = match to {
                    kentos_commands::DistanceTo::Face => m.distance(start),
                    // docs/adr/0728 §1: from its middle to the grid crossing the face.
                    kentos_commands::DistanceTo::Grid => m.grid_distance(start),
                };
                let Some(now) = now else {
                    let side = if start { "başında" } else { "sonunda" };
                    return Err(fail(
                        codes::INVALID_OPENING,
                        match to {
                            kentos_commands::DistanceTo::Face => format!(
                                "Açıklığın {side} bu yüzde referans yok: yüzün ucu açıklığın gerisinde kalıyor. Öbür yüzü ya da yanı seçin."
                            ),
                            kentos_commands::DistanceTo::Grid => format!(
                                "Açıklığın {side} bu yüzü kesen bir aks yok; aksa mesafe ölçülemiyor. Öbür yanı seçin ya da yüze ölçün."
                            ),
                        },
                        &path,
                    ));
                };
                // The reference stays: the jamb goes to it plus (start) or less (end) the distance.
                let mut f = o.opening.clone();
                f.at += if start { distance - now } else { now - distance };
                // It stays on its edge: the reference was there.
                let line = host_line(doc, &f, &path)?;
                let p = &line.path;
                let (lo, hi) = (p.cum[m.edge], p.cum[m.edge] + kentos_geometry_core::ops::edges::edge_length(&p.edges[m.edge]));
                let (s0, s1) = opening_follow::span(&f);
                let tol = 1e-9 * p.length.max(1.0);
                if s0 < lo - tol || s1 > hi + tol {
                    return Err(fail(
                        codes::OPENING_OFF_WALL,
                        format!(
                            "Açıklık bu mesafeyle bulunduğu duvar kenarının dışına taşıyor: duvar boyunca {s0:.3}–{s1:.3} m'de, kenar {lo:.3}–{hi:.3} m. Daha küçük bir mesafe verin."
                        ),
                        &path,
                    ));
                }
                next.push((*at, *slot, o, f, true));
            }
        }
        OpeningsOperation::Flip { what } => {
            for (at, slot, o) in &list {
                let mut f = o.opening.clone();
                if matches!(what, OpeningFlip::Side | OpeningFlip::Both) {
                    f.side = f.side.other();
                }
                if matches!(what, OpeningFlip::Hand | OpeningFlip::Both) {
                    f.hand = f.hand.other();
                }
                next.push((*at, *slot, o, f, false));
            }
        }
        OpeningsOperation::Rehost { to, point } => {
            if let Some(p) = point {
                checks::point(*p, "Açıklığın noktası", "operation.point")?;
            }
            let (_, _, line) = openings::wall_at(doc, to, "operation.to")?;
            for (at, slot, o) in &list {
                let mut f = o.opening.clone();
                f.host.clone_from(to);
                let a = opening_follow::anchor(&f);
                match point {
                    Some(p) => {
                        let s = nearest_s(&line.path, CoreVec2::new(p.x, p.y));
                        f.at = at_of_centre(s, a, f.width);
                    }
                    None => {
                        let place: Vec<CoreVec2> =
                            f.place.iter().map(|q| CoreVec2::new(q.x, q.y)).collect();
                        if let Some(r) = opening::read_back(&line.path, &place, a, f.width) {
                            f.at = r.at;
                            if r.hand_turned {
                                f.hand = f.hand.other();
                            }
                            if r.side_turned {
                                f.side = f.side.other();
                            }
                        }
                    }
                }
                f.broken = None;
                next.push((*at, *slot, o, f, true));
            }
        }
        OpeningsOperation::ReplaceType { from, to, level } => {
            // The project's type, else KentOS's as before (its refusal `style_not_found`).
            let kind = match crate::opening_types::project_type(doc, to) {
                Some(t) => crate::opening_types::Found::Project(t),
                None => crate::opening_types::Found::Builtin(openings::find_type(to, "operation.to")?),
            };
            let mut any = false;
            for (at, slot, o) in &list {
                if from.as_deref().is_some_and(|k| !of_type(doc, &o.opening, k))
                    || !on_level(doc, &o.opening, level.as_ref())
                {
                    continue;
                }
                any = true;
                let mut f = o.opening.clone();
                match &kind {
                    // A project type's openings follow it, all its values theirs (docs/adr/0772 §1).
                    crate::opening_types::Found::Project(t) => {
                        f.overrides.clear();
                        f.take_type(t);
                    }
                    crate::opening_types::Found::Builtin(t) => {
                        t.apply(&mut f);
                        f.type_version = None;
                        f.overrides.clear();
                    }
                }
                next.push((*at, *slot, o, f, false));
            }
            if !any {
                return Err(fail(
                    codes::STYLE_NOT_USED,
                    format!(
                        "Verilen açıklıkların hiçbiri “{}” türünde değil.",
                        from.as_deref().unwrap_or_default()
                    ),
                    "operation.from",
                ));
            }
        }
        OpeningsOperation::Detach => {
            for (at, slot, o) in &list {
                typed(&o.opening, *at)?;
                let mut f = o.opening.clone();
                crate::opening_types_edit::detach_fields(&mut f);
                next.push((*at, *slot, o, f, false));
            }
        }
        OpeningsOperation::Refresh => {
            for (at, slot, o) in &list {
                typed(&o.opening, *at)?;
                let id = o.opening.style.clone().unwrap_or_default();
                let t = crate::opening_types::find_project(doc, &id, &format!("uids[{at}]"))?;
                let mut f = o.opening.clone();
                f.take_type(&t);
                next.push((*at, *slot, o, f, false));
            }
        }
        OpeningsOperation::Reset { fields } => {
            for (at, slot, o) in &list {
                typed(&o.opening, *at)?;
                let id = o.opening.style.clone().unwrap_or_default();
                let t = crate::opening_types::find_project(doc, &id, &format!("uids[{at}]"))?;
                let mut f = o.opening.clone();
                if fields.is_empty() {
                    f.overrides.clear();
                } else {
                    f.overrides.retain(|k| !fields.contains(k));
                }
                f.take_type(&t);
                next.push((*at, *slot, o, f, false));
            }
        }
        OpeningsOperation::Push { fields } => {
            let [(at, slot, source)] = list.as_slice() else {
                return Err(fail(
                    codes::INVALID_OPENING,
                    "Tipe uygulamak için tek bir açıklık verin; onun değerleri tipin olur.".into(),
                    "uids",
                ));
            };
            typed(&source.opening, *at)?;
            let id = source.opening.style.clone().unwrap_or_default();
            let old = crate::opening_types::find_project(doc, &id, &format!("uids[{at}]"))?;
            let pushed: Vec<_> = if fields.is_empty() { source.opening.overrides.clone() } else { fields.clone() };
            if pushed.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Açıklığın tipe uygulanacak kendi değeri yok: alanları verin ya da önce bir istisna yapın.".into(),
                    "operation.fields",
                ));
            }
            let mut new = old.clone();
            for k in &pushed {
                new.set_value(*k, &source.opening.value(*k))
                    .map_err(|w| fail(codes::INVALID_OPENING_TYPE, w, "operation.fields"))?;
            }
            if !pushed.contains(&kentos_commands::OpeningTypeField::Width) {
                let mut f = new.fields();
                crate::opening_types::keep_nominal(&old, &mut f);
                new.width = f.width;
            }
            if let Some((field, words)) = new.problem() {
                return Err(fail(codes::INVALID_OPENING_TYPE, words, &format!("operation.{field}")));
            }
            let changed_fields = new.differs_from(&old);
            if changed_fields.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!("“{}” tipi zaten açıklığın bu değerlerinde.", old.name),
                    "operation.fields",
                ));
            }
            new.version = old.version + 1;
            let mut own = (*source).clone();
            own.opening.overrides.retain(|k| !pushed.contains(k));
            let followed =
                crate::opening_types::follow(doc, &new, &changed_fields, Default::default(), Some((*slot, own)))?;
            let mut table = doc.opening_types().to_vec();
            if let Some(t) = table.iter_mut().find(|t| t.id == new.id) {
                *t = new;
            }
            work.warnings.extend(crate::opening_types::warnings(&followed.report));
            for (s, e) in followed.updates {
                let path = doc.uid(s).map(|u| u.to_string()).unwrap_or_default();
                work.updates.push((s, e, path));
            }
            work.types = Some(table);
            work.report = Some(followed.report);
        }
        OpeningsOperation::Renumber {
            prefix,
            start,
            form: _,
        } => {
            // In their walls' order (the walls' slots), then along each wall.
            let mut order: Vec<(Option<Slot>, f64, usize)> = list
                .iter()
                .enumerate()
                .map(|(k, (_, _, o))| {
                    let wall = kentos_domain::Uuid::parse_str(&o.opening.host)
                        .ok()
                        .and_then(|u| doc.slot_of(u));
                    (wall, opening_follow::span(&o.opening).0, k)
                })
                .collect();
            order.sort_by(|a, b| {
                (a.0.is_none(), a.0)
                    .cmp(&(b.0.is_none(), b.0))
                    .then(a.1.total_cmp(&b.1))
                    .then(a.2.cmp(&b.2))
            });
            for (n, (_, _, k)) in order.into_iter().enumerate() {
                let (at, slot, o) = list[k];
                let mut f = o.opening.clone();
                let number = u64::from(*start) + n as u64;
                f.number = Some(format!("{prefix}{number}"));
                next.push((at, slot, o, f, false));
            }
            next.sort_by_key(|(at, ..)| *at);
        }
        OpeningsOperation::Array { layout, numbering } => {
            let [(_, _, source)] = list.as_slice() else {
                return Err(fail(
                    codes::INVALID_OPENING,
                    "Açıklık dağıtmak için tam bir kaynak kapı, pencere veya boşluk verin; kaynak yerinde kalır.".into(),
                    "uids",
                ));
            };
            let line = host_line(doc, &source.opening, "uids[0]")?;
            work.added = array::copies(doc, source, &line, layout, numbering.as_ref())?;
        }
        OpeningsOperation::Copy { offsets } => {
            if offsets.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Kopyaların uzaklıkları verilmedi: duvar boyunca en az bir uzaklık verin."
                        .into(),
                    "operation.offsets",
                ));
            }
            for (k, d) in offsets.iter().enumerate() {
                checks::finite(
                    *d,
                    "Uzaklık",
                    "Sonlu bir sayı verin.",
                    &format!("operation.offsets[{k}]"),
                )?;
            }
            if list.len() * offsets.len() > kentos_commands::MAX_OPENINGS {
                return Err(fail(
                    codes::INVALID_OPENING,
                    format!(
                        "Bir çağrıda en çok {} açıklık kopyalanır.",
                        kentos_commands::MAX_OPENINGS
                    ),
                    "operation.offsets",
                ));
            }
            for (at, _, o) in &list {
                let line = host_line(doc, &o.opening, &format!("uids[{at}]"))?;
                for (k, d) in offsets.iter().enumerate() {
                    let mut f = o.opening.clone();
                    f.at += d;
                    f.number = None;
                    let path = format!("operation.offsets[{k}]");
                    openings::valid(&f, &path)?;
                    let f = openings::placed(&f, &line, &path)?;
                    let mut e = (*o).clone();
                    e.base.id = 0;
                    e.base.label = None;
                    e.base.label_pins.clear();
                    e.base.attachments.clear();
                    e.opening = f;
                    work.added.push((e, path));
                }
            }
        }
    }
    // Each one's fields, then its place on its wall (a wall gone keeps it where it stands, unless it must move).
    for (at, slot, o, f, must_stand) in next {
        let path = format!("uids[{at}]");
        openings::valid(&f, &path)?;
        let f = match doc.wall_line_of(&f.host) {
            Some(line) => openings::placed(&f, &line, &path)?,
            None if must_stand => {
                return Err(fail(
                    codes::OPENING_HOST_GONE,
                    "Açıklığın duvarı yok; duvar boyunca taşınamaz. Önce bir duvara taşıyın (Başka duvara).".into(),
                    &path,
                ));
            }
            None => opening_follow::placed_on(&f, None, false),
        };
        let mut e = o.clone();
        e.opening = f;
        work.updates.push((slot, e, path));
    }
    let changed: Vec<(Slot, &OpeningFields, String)> = work
        .updates
        .iter()
        .map(|(s, e, p)| (*s, &e.opening, p.clone()))
        .collect();
    let new: Vec<(&OpeningFields, String)> = work
        .added
        .iter()
        .map(|(e, p)| (&e.opening, p.clone()))
        .collect();
    openings::clear_of_others(doc, &changed, &new)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    // All or none: an opening on a locked layer refuses the whole change.
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc
                .layers()
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki açıklık düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    Ok(work)
}

/// `opening_not_typed` for an opening following no project type.
fn typed(o: &OpeningFields, at: usize) -> Result<(), Stop> {
    if o.follows_type() {
        Ok(())
    } else {
        Err(fail(
            codes::OPENING_NOT_TYPED,
            "Açıklık bir proje tipini izlemiyor; önce bir tip verin (Türü değiştir).".into(),
            &format!("uids[{at}]"),
        ))
    }
}

/// The base line of the wall `o` is in; `opening_host_gone` when it is gone.
fn host_line(
    doc: &Document,
    o: &OpeningFields,
    path: &str,
) -> Result<opening_follow::WallLine, Stop> {
    doc.wall_line_of(&o.host).ok_or_else(|| {
        fail(
            codes::OPENING_HOST_GONE,
            "Açıklığın duvarı yok; duvar boyunca kopyalanamaz. Önce bir duvara taşıyın (Başka duvara).".into(),
            path,
        )
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &OpeningsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &OpeningsEdit) -> CommandResult<OpeningsEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the opening tools draw as their preview.
pub fn plan_in(doc: &Document, input: &OpeningsEdit) -> CommandResult<OpeningsEdited> {
    match check(doc, input) {
        Ok(work) => {
            let updates: Vec<_> = work
                .updates
                .into_iter()
                .map(|(s, e, _)| (s, Entity::Opening(e)))
                .collect();
            let added: Vec<_> = work
                .added
                .into_iter()
                .map(|(e, _)| Entity::Opening(e))
                .collect();
            let followers = crate::dimension_sets::derived(doc, &updates, &added, &[]);
            CommandResult::Completed {
                output: OpeningsEdited {
                    openings: updates.into_iter().map(|(_, e)| e).collect(),
                    added,
                    followers,
                    report: work.report,
                    revision: doc.revision().to_string(),
                },
                warnings: work.warnings,
            }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: OpeningsEdit,
) -> CommandResult<OpeningsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_OPENINGS_EDIT,
        kentos_commands::CAD_OPENINGS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: OpeningsEdit,
) -> CommandResult<OpeningsEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let result = doc.transact(STEP, |d| {
        if let Some(types) = &work.types
            && d.set_opening_types(types.clone(), STEP).is_err()
        {
            return Err(kentos_domain::SlotsExhausted);
        }
        d.update_many(
            work.updates
                .iter()
                .map(|(s, e, _)| (*s, Entity::Opening(e.clone())))
                .collect(),
            STEP,
        );
        d.add_many(
            work.added
                .iter()
                .map(|(e, _)| Entity::Opening(e.clone()))
                .collect(),
            STEP,
        )
    });
    let slots = match result {
        Ok(slots) => slots,
        Err(_) => {
            return CommandResult::Failed {
                error: error(
                    codes::SLOTS_EXHAUSTED,
                    "Çizimde yeni nesneye yer kalmadı; çizimi kaydedip yeniden açın.".into(),
                    None,
                ),
            };
        }
    };
    let openings = work
        .updates
        .iter()
        .filter_map(|(s, _, _)| doc.get(*s).cloned())
        .collect();
    let added = slots.iter().filter_map(|s| doc.get(*s).cloned()).collect();
    CommandResult::Completed {
        output: OpeningsEdited {
            followers: Default::default(),
            openings,
            added,
            report: work.report,
            revision: doc.revision().to_string(),
        },
        warnings: work.warnings,
    }
}
