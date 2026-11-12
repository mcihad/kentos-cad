//! Room renumbering computes order and collision checks once for every surface.
use crate::checks::Stop;
use crate::{ExecutionContext, checks, codes, spaces};
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::text::edit::increment;
use std::collections::HashSet;

pub const STEP: &str = "Mahalleri numarala";

struct Work {
    updates: Vec<(Slot, Entity)>,
    uids: Vec<String>,
}
fn invalid(message: &str, path: &str) -> Stop {
    spaces::fail(codes::INVALID_SPACE_NUMBER, message.into(), path)
}
fn check(doc: &Document, input: &SpacesNumber) -> Result<Work, Stop> {
    checks::uids(&input.uids, "Numaralanacak mahalleri seçin.")?;
    if input.uids.len() > MAX_SPACES {
        return Err(invalid(
            "Bir çağrıda en çok 10000 mahal numaralanabilir; seçimi daraltın.",
            "uids",
        ));
    }
    if !valid_space_number(&input.first) {
        return Err(invalid(
            "İlk numara en çok 256 karakter olmalı ve rakamla bitmeli: Z-001 gibi. Başında/sonunda boşluk veya kontrol karakteri kullanmayın.",
            "first",
        ));
    }
    let mut distinct = HashSet::with_capacity(input.uids.len());
    for (at, uid) in input.uids.iter().enumerate() {
        if !distinct.insert(uid) {
            return Err(invalid(
                "Aynı mahal seçimde birden çok kez verilmiş; her mahalı bir kez seçin.",
                &format!("uids[{at}]"),
            ));
        }
    }
    let named = checks::named(doc, &input.uids)?;
    let mut list = Vec::with_capacity(named.len());
    let mut selected = HashSet::with_capacity(named.len());
    for (at, slot, e, uid) in named {
        if !matches!(e, Entity::Space(_)) {
            return Err(spaces::fail(
                codes::NOT_A_SPACE,
                "Numaralandırma yalnız mahal nesnelerini kabul eder; seçimi düzeltin.".into(),
                &format!("uids[{at}]"),
            ));
        }
        selected.insert(slot);
        if doc.layers().is_locked(&e.base().layer_id) {
            return Err(spaces::fail(
                codes::LAYER_LOCKED,
                "Seçilen mahalın katmanı kilitli; numaralandırmadan önce kilidini açın.".into(),
                &format!("uids[{at}]"),
            ));
        }
        list.push((slot, e.clone(), uid.to_string()));
    }
    if let SpaceNumberOrder::Direction { from, to } = input.order {
        checks::point(from, "Yön başlangıcının", "order.from")?;
        checks::point(to, "Yön bitişinin", "order.to")?;
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let length = dx.hypot(dy);
        if !length.is_finite() || length == 0. {
            return Err(invalid(
                "Numaralandırma yönü için birbirinden farklı, sonlu iki nokta verin.",
                "order",
            ));
        }
        let (ux, uy) = (dx / length, dy / length);
        let mut ranked = Vec::with_capacity(list.len());
        for item in list {
            let Some(p) = doc.tag_anchor(&item.1) else {
                return Err(invalid(
                    "Mahalın etiket konumu hesaplanamadı; sınırını düzeltin veya tıklama sırasını kullanın.",
                    "uids",
                ));
            };
            let (x, y) = (p.x - from.x, p.y - from.y);
            let (along, across) = (x * ux + y * uy, -x * uy + y * ux);
            if !along.is_finite() || !across.is_finite() {
                return Err(invalid(
                    "Mahal ve yön koordinatları sıralanamayacak kadar büyük; çizime yakın bir yön verin.",
                    "order",
                ));
            }
            // Treat -0 and +0 alike; the final UUID tie break is independent of input order.
            ranked.push((
                if along == 0. { 0. } else { along },
                if across == 0. { 0. } else { across },
                item,
            ));
        }
        ranked.sort_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then(a.1.total_cmp(&b.1))
                .then(a.2.2.cmp(&b.2.2))
        });
        list = ranked.into_iter().map(|(_, _, item)| item).collect();
    }
    // Existing duplicates among selected rooms may be repaired in one operation.
    // Numbers outside the selection, including hidden/other-level rooms, reserve their exact strings.
    let used: HashSet<&str> = doc
        .entities()
        .filter(|e| !selected.contains(&Slot(e.base().id)))
        .filter_map(|e| {
            if let Entity::Space(s) = e {
                s.space.number.as_deref()
            } else {
                None
            }
        })
        .collect();
    let mut number = input.first.clone();
    let mut updates = Vec::with_capacity(list.len());
    let mut uids = Vec::with_capacity(list.len());
    let count = list.len();
    for (at, (slot, mut e, uid)) in list.into_iter().enumerate() {
        if number.chars().count() > 256 {
            return Err(invalid(
                "Numara dizisi 256 karakteri aşıyor; daha kısa bir ilk numara kullanın.",
                "first",
            ));
        }
        if used.contains(number.as_str()) {
            return Err(spaces::fail(
                codes::SPACE_NUMBER_CONFLICT,
                format!(
                    "“{number}” numarası seçim dışındaki bir mahalde var. İlk numarayı değiştirin veya o mahalı da seçin."
                ),
                "first",
            ));
        }
        if let Entity::Space(s) = &mut e {
            s.space.number = Some(number.clone());
        }
        updates.push((slot, e));
        uids.push(uid);
        if at + 1 < count {
            number = increment(&number).ok_or_else(|| {
                invalid(
                    "Numara dizisi artırılamadı; rakamla biten bir ilk numara verin.",
                    "first",
                )
            })?;
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Work { updates, uids })
}
fn output(doc: &Document, work: Work) -> SpacesEdited {
    SpacesEdited {
        spaces: work.updates.into_iter().map(|(_, e)| e).collect(),
        kept: work.uids,
        added: vec![],
        removed: vec![],
        decisions: vec![],
        revision: doc.revision().to_string(),
    }
}
pub fn validate(cx: &ExecutionContext<'_>, input: &SpacesNumber) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: vec![],
        },
        Err(why) => why.into(),
    }
}
pub fn plan_in(doc: &Document, input: &SpacesNumber) -> CommandResult<SpacesEdited> {
    match check(doc, input) {
        Ok(work) => CommandResult::Completed {
            output: output(doc, work),
            warnings: vec![],
        },
        Err(why) => why.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &SpacesNumber) -> CommandResult<SpacesEdited> {
    plan_in(cx.doc, input)
}
pub fn execute(cx: &mut ExecutionContext<'_>, input: SpacesNumber) -> CommandResult<SpacesEdited> {
    crate::call::recorded(
        cx,
        CAD_SPACES_NUMBER,
        CAD_SPACES_NUMBER_VERSION,
        input,
        write,
    )
}
fn write(cx: &mut ExecutionContext<'_>, input: SpacesNumber) -> CommandResult<SpacesEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(why) => return why.into(),
    };
    cx.doc.update_many(work.updates.clone(), STEP);
    CommandResult::Completed {
        output: output(cx.doc, work),
        warnings: vec![],
    }
}
