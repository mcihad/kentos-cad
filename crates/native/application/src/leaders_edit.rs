//! `cad.leaders.edit` v1 (docs/adr/0326 §5): a leader's arms added and taken away, leaders' landings aligned
//! (MLEADERALIGN), block content leaders collected into one (MLEADERCOLLECT), a leader's note and content blocks, its
//! look and style, and what its tips follow; each as one undo step named after the edit. The document keeps the tips
//! and content blocks with their objects at every later step (`kentos_domain::leader_ties`). The web is paused
//! (docs/adr/0228): the desktop's handler alone, passing `fixtures/commands/v1/cad.leaders.edit.json`.

use std::collections::HashSet;

use kentos_commands::{
    CommandResult, Entity, EntityId, LeaderArm, LeaderArrow, LeaderEdit, LeaderEntity, LeadersEdit,
    LeadersEdited, LeadersPlan, SubRef, Vec2, apply_leader_style, leader_arrow_holds,
    leader_problem,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::jsmath::{PI, cos, sin};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_blank, is_uid_text};
pub use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The undo step's name: the edit's.
fn label(e: &LeaderEdit) -> &'static str {
    match e {
        LeaderEdit::AddArm { .. } => "Kol ekle",
        LeaderEdit::RemoveArm { .. } => "Kol sil",
        LeaderEdit::Align { .. } => "Kılavuzları hizala",
        LeaderEdit::Collect { .. } => "Kılavuzları topla",
        LeaderEdit::Content { .. } => "Kılavuzun notu",
        LeaderEdit::Blocks { .. } => "Kılavuzun blokları",
        LeaderEdit::Look { .. } => "Kılavuzun görünüşü",
        LeaderEdit::Style { .. } => "Kılavuz stili",
        LeaderEdit::Bind { .. } => "Kılavuzun ucu",
    }
}

fn bad_uid(uid: &str, path: &str) -> Stop {
    fail(
        codes::INVALID_UID,
        format!(
            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
        ),
        path,
    )
}

fn not_found(uid: &str, path: &str) -> Stop {
    fail(
        codes::ENTITY_NOT_FOUND,
        format!(
            "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
        ),
        path,
    )
}

/// The object `uid` names: its slot and id.
fn object_at(doc: &Document, uid: &str, path: &str) -> Result<(Slot, Uuid), Stop> {
    if !is_uid_text(uid) {
        return Err(bad_uid(uid, path));
    }
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u).map(|s| (s, u)))
        .ok_or_else(|| not_found(uid, path))
}

/// The leader `uid` names, on an unlocked layer.
fn leader_at(doc: &Document, uid: &str, path: &str) -> Result<(Slot, Uuid, LeaderEntity), Stop> {
    let (slot, u) = object_at(doc, uid, path)?;
    let Some(Entity::Leader(l)) = doc.get(slot) else {
        return Err(fail(
            codes::NOT_A_LEADER,
            "Verilen nesne bir kılavuz değil; bu komut yalnız kılavuzları değiştirir.".into(),
            path,
        ));
    };
    if doc.layers().is_locked(&l.base.layer_id) {
        let layers = doc.layers();
        let name = layers
            .get(&l.base.layer_id)
            .map_or(l.base.layer_id.as_str(), |n| n.name.as_str());
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki kılavuz değişmez. Kilidi Katmanlar panelinden açın."
            ),
            path,
        ));
    }
    Ok((slot, u, l.clone()))
}

/// The leaders `uids` names, each once, in order.
fn leaders_at(
    doc: &Document,
    uids: &[String],
    least: usize,
) -> Result<Vec<(Slot, Uuid, LeaderEntity)>, Stop> {
    if uids.is_empty() {
        return Err(fail(
            codes::NO_ENTITIES,
            "Kılavuz verilmedi. En az bir kılavuzun kimliğini verin.".into(),
            "edit.uids",
        ));
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (i, u) in uids.iter().enumerate() {
        if seen.insert(u.as_str()) {
            out.push(leader_at(doc, u, &format!("edit.uids[{i}]"))?);
        }
    }
    if out.len() < least {
        return Err(fail(
            codes::TOO_FEW_OBJECTS,
            format!("Bu düzenleme en az {least} kılavuz ister."),
            "edit.uids",
        ));
    }
    Ok(out)
}

fn invalid(words: String, path: &str) -> Stop {
    fail(codes::INVALID_LEADER, words, path)
}

/// The point of the place `tip` names, checked.
fn tip_point(doc: &Document, tip: &SubRef, path: &str) -> Result<Vec2, Stop> {
    let uid = Uuid::from_bytes(tip.uid.0);
    let Some(e) = doc.slot_of(uid).and_then(|s| doc.get(s)) else {
        return Err(not_found(&uid.to_string(), &format!("{path}.uid")));
    };
    let core = crate::blocks::core_entity(e);
    kentos_geometry_core::ops::subref::point(
        &core,
        kentos_domain::dimension_ties::core_place(tip.place),
    )
    .map(|p| Vec2 { x: p.x, y: p.y })
    .ok_or_else(|| {
        fail(
            codes::NO_SUCH_PLACE,
            "Nesnenin böyle bir yeri yok: köşe, uç ya da merkez nesnede olmalı.".into(),
            &format!("{path}.place"),
        )
    })
}

fn finite_points(pts: &[Vec2], path: &str) -> Result<(), Stop> {
    for (i, p) in pts.iter().enumerate() {
        checks::point(*p, "Kolun noktasının", &format!("{path}[{i}]"))?;
    }
    Ok(())
}

/// What the checks give: each leader changed, as it would be, and those taken away.
struct Checked {
    changed: Vec<(Slot, Uuid, LeaderEntity)>,
    removed: Vec<(Slot, Uuid)>,
}

fn check(doc: &Document, input: &LeadersEdit) -> Result<Checked, Stop> {
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    match &input.edit {
        LeaderEdit::AddArm { uid, pts, tip } => {
            let (slot, u, mut l) = leader_at(doc, uid, "edit.uid")?;
            if pts.is_empty() {
                return Err(invalid(
                    "Kolun ucu verilmedi: en az bir nokta verin.".into(),
                    "edit.pts",
                ));
            }
            finite_points(pts, "edit.pts")?;
            let mut arm = LeaderArm {
                pts: pts.clone(),
                tip: None,
            };
            if let Some(t) = tip {
                arm.pts[0] = tip_point(doc, t, "edit.tip")?;
                arm.tip = Some(SubRef {
                    broken: None,
                    ..t.clone()
                });
            }
            l.arms.push(arm);
            changed.push((slot, u, l));
        }
        LeaderEdit::RemoveArm { uid, arm } => {
            let (slot, u, mut l) = leader_at(doc, uid, "edit.uid")?;
            let k = *arm as usize;
            if k > l.arms.len() {
                return Err(fail(
                    codes::ARM_NOT_FOUND,
                    format!(
                        "Kılavuzun {} kolu var; {arm}. sıradaki yok (sıra 0'dan başlar).",
                        l.arms.len() + 1
                    ),
                    "edit.arm",
                ));
            }
            if l.arms.is_empty() {
                return Err(invalid(
                    "Kılavuzun tek kolu kalkmaz; kılavuzu silin.".into(),
                    "edit.arm",
                ));
            }
            if k == 0 {
                let next = l.arms.remove(0);
                let last = *l.pts.last().unwrap_or(&next.pts[0]);
                l.pts = next.pts;
                l.pts.push(last);
                l.tip = next.tip;
            } else {
                l.arms.remove(k - 1);
            }
            changed.push((slot, u, l));
        }
        LeaderEdit::Align {
            uids,
            angle,
            spacing,
        } => {
            checks::finite(
                *angle,
                "Doğrultu",
                "Doğrultuyu derece olarak sonlu bir sayıyla verin.",
                "edit.angle",
            )?;
            if let Some(s) = spacing {
                checks::finite(
                    *s,
                    "Aralık",
                    "Aralığı metre olarak sonlu bir sayıyla verin.",
                    "edit.spacing",
                )?;
                if *s <= 0.0 {
                    return Err(invalid(
                        format!("Aralık {s}; sıfırdan büyük olmalı."),
                        "edit.spacing",
                    ));
                }
            }
            let list = leaders_at(doc, uids, 2)?;
            let p0 = *list[0].2.pts.last().unwrap_or(&Vec2 { x: 0.0, y: 0.0 });
            let r = angle * PI / 180.0;
            let d = (cos(r), sin(r));
            // Each other's place along the line: where it falls on it, or its turn at the spacing on its side.
            let mut along: Vec<(usize, f64)> = list[1..]
                .iter()
                .enumerate()
                .map(|(i, (_, _, l))| {
                    let q = *l.pts.last().unwrap_or(&p0);
                    (i + 1, (q.x - p0.x) * d.0 + (q.y - p0.y) * d.1)
                })
                .collect();
            if let Some(s) = spacing {
                along.sort_by(|a, b| a.1.total_cmp(&b.1));
                let ahead: Vec<usize> = along
                    .iter()
                    .filter(|(_, t)| *t >= 0.0)
                    .map(|(i, _)| *i)
                    .collect();
                let behind: Vec<usize> = along
                    .iter()
                    .rev()
                    .filter(|(_, t)| *t < 0.0)
                    .map(|(i, _)| *i)
                    .collect();
                along = ahead
                    .iter()
                    .enumerate()
                    .map(|(k, i)| (*i, s * (k + 1) as f64))
                    .chain(
                        behind
                            .iter()
                            .enumerate()
                            .map(|(k, i)| (*i, -s * (k + 1) as f64)),
                    )
                    .collect();
                along.sort_by_key(|(i, _)| *i);
            }
            for (i, t) in along {
                let (slot, u, l) = &list[i];
                let mut next = l.clone();
                if let Some(last) = next.pts.last_mut() {
                    *last = Vec2 {
                        x: p0.x + t * d.0,
                        y: p0.y + t * d.1,
                    };
                }
                changed.push((*slot, *u, next));
            }
        }
        LeaderEdit::Collect { uids } => {
            let list = leaders_at(doc, uids, 2)?;
            if let Some(i) = list.iter().position(|(_, _, l)| l.blocks.is_empty()) {
                return Err(invalid(
                    "Yalnız blok içerikli kılavuzlar toplanır; notlu kılavuz toplanmaz.".into(),
                    &format!("edit.uids[{i}]"),
                ));
            }
            let (slot, u, mut first) = list[0].clone();
            for (s, o, l) in &list[1..] {
                let n = l.pts.len().saturating_sub(1).max(1);
                first.arms.push(LeaderArm {
                    pts: l.pts[..n].to_vec(),
                    tip: l.tip.clone(),
                });
                first.arms.extend(l.arms.iter().cloned());
                first.blocks.extend(l.blocks.iter().copied());
                removed.push((*s, *o));
            }
            changed.push((slot, u, first));
        }
        LeaderEdit::Content { uid, text } => {
            let (slot, u, mut l) = leader_at(doc, uid, "edit.uid")?;
            if let Some(t) = text {
                if is_blank(t) {
                    return Err(fail(
                        codes::EMPTY_TEXT,
                        "Not boş; notu kaldırmak için null verin.".into(),
                        "edit.text",
                    ));
                }
                if !l.blocks.is_empty() {
                    return Err(invalid(
                        "Blok içerikli kılavuzun notu olmaz: önce bloklarını kaldırın.".into(),
                        "edit.text",
                    ));
                }
            }
            l.text = text.clone();
            changed.push((slot, u, l));
        }
        LeaderEdit::Blocks { uid, blocks } => {
            let (slot, u, mut l) = leader_at(doc, uid, "edit.uid")?;
            let mut ids = Vec::with_capacity(blocks.len());
            for (i, b) in blocks.iter().enumerate() {
                let path = format!("edit.blocks[{i}]");
                let (s, o) = object_at(doc, b, &path)?;
                if !matches!(doc.get(s), Some(Entity::Insert(_))) {
                    return Err(fail(
                        codes::NOT_AN_INSERT,
                        "Verilen nesne bir blok yerleştirmesi değil; kılavuzun içeriği bloktur."
                            .into(),
                        &path,
                    ));
                }
                ids.push(EntityId(*o.as_bytes()));
            }
            l.blocks = ids;
            // A leader with blocks has no note (docs/adr/0326 §2).
            if !l.blocks.is_empty() {
                l.text = None;
            }
            changed.push((slot, u, l));
        }
        LeaderEdit::Look {
            uids,
            arrow,
            arrow_size,
            height,
            mask,
        } => {
            let arrow = match arrow.as_deref() {
                None => None,
                Some("filled") => Some(None),
                Some(name) => Some(Some(LeaderArrow::from_name(name).ok_or_else(|| {
                    invalid(
                        format!("“{name}” bir ok başı değil (filled, open, dot, none …)."),
                        "edit.arrow",
                    )
                })?)),
            };
            if let Some(s) = arrow_size
                && !leader_arrow_holds(*s)
            {
                return Err(invalid(
                    format!(
                        "Ok boyu {s}; notun yüksekliğinin {} ile {} katı olmalı.",
                        kentos_commands::MIN_LEADER_ARROW,
                        kentos_commands::MAX_LEADER_ARROW
                    ),
                    "edit.arrowSize",
                ));
            }
            if let Some(h) = height {
                checks::finite(
                    *h,
                    "Yükseklik",
                    "Yüksekliği metre olarak sonlu bir sayıyla verin.",
                    "edit.height",
                )?;
                if *h <= 0.0 {
                    return Err(invalid(
                        format!("Yükseklik {h}; sıfırdan büyük olmalı."),
                        "edit.height",
                    ));
                }
            }
            for (slot, u, l) in leaders_at(doc, uids, 1)? {
                let mut next = l.clone();
                if let Some(a) = arrow {
                    next.arrow = a;
                }
                if let Some(s) = arrow_size {
                    next.arrow_size = (*s != 1.0).then_some(*s);
                }
                if let Some(h) = height {
                    next.height = *h;
                }
                if let Some(m) = mask {
                    next.mask = *m;
                }
                changed.push((slot, u, next));
            }
        }
        LeaderEdit::Style { uids, style } => {
            let def = match style {
                Some(id) => Some(
                    doc.settings()
                        .leader_styles
                        .iter()
                        .find(|s| s.id == *id)
                        .cloned()
                        .ok_or_else(|| {
                            fail(
                                codes::STYLE_NOT_FOUND,
                                format!("“{id}” kimlikli kılavuz stili projede yok."),
                                "edit.style",
                            )
                        })?,
                ),
                None => None,
            };
            let scale = doc.settings().plot_scale;
            for (slot, u, l) in leaders_at(doc, uids, 1)? {
                let next = match &def {
                    Some(s) => apply_leader_style(s, &l, scale),
                    None => LeaderEntity {
                        leader_style: None,
                        ..l.clone()
                    },
                };
                changed.push((slot, u, next));
            }
        }
        LeaderEdit::Bind { uid, arm, tip } => {
            let (slot, u, mut l) = leader_at(doc, uid, "edit.uid")?;
            let k = *arm as usize;
            if k > l.arms.len() {
                return Err(fail(
                    codes::ARM_NOT_FOUND,
                    format!(
                        "Kılavuzun {} kolu var; {arm}. sıradaki yok (sıra 0'dan başlar).",
                        l.arms.len() + 1
                    ),
                    "edit.arm",
                ));
            }
            let (point, r) = match tip {
                Some(t) => (
                    Some(tip_point(doc, t, "edit.tip")?),
                    Some(SubRef {
                        broken: None,
                        ..t.clone()
                    }),
                ),
                None => (None, None),
            };
            if k == 0 {
                if let (Some(p), Some(first)) = (point, l.pts.first_mut()) {
                    *first = p;
                }
                l.tip = r;
            } else {
                let a = &mut l.arms[k - 1];
                if let (Some(p), Some(first)) = (point, a.pts.first_mut()) {
                    *first = p;
                }
                a.tip = r;
            }
            changed.push((slot, u, l));
        }
    }
    for (_, _, l) in &changed {
        if let Some((field, words)) = leader_problem(l) {
            return Err(invalid(words, &format!("edit.{field}")));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    changed.retain(|(slot, _, l)| doc.get(*slot) != Some(&Entity::Leader(l.clone())));
    Ok(Checked { changed, removed })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LeadersEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &LeadersEdit) -> CommandResult<LeadersPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (a tool's preview).
pub fn plan_in(doc: &Document, input: &LeadersEdit) -> CommandResult<LeadersPlan> {
    match check(doc, input) {
        Ok(c) => CommandResult::Completed {
            output: LeadersPlan {
                leaders: c
                    .changed
                    .into_iter()
                    .map(|(_, _, l)| Entity::Leader(l))
                    .collect(),
                removed: c.removed.iter().map(|(_, u)| u.to_string()).collect(),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: LeadersEdit) -> CommandResult<LeadersEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LEADERS_EDIT,
        kentos_commands::CAD_LEADERS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LeadersEdit,
) -> CommandResult<LeadersEdited> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let changed = c.changed.iter().map(|(_, u, _)| u.to_string()).collect();
    let removed: Vec<String> = c.removed.iter().map(|(_, u)| u.to_string()).collect();
    let name = label(&input.edit);
    let _: Result<(), ()> = doc.transact(name, |doc| {
        if !c.changed.is_empty() {
            doc.update_many(
                c.changed
                    .into_iter()
                    .map(|(s, _, l)| (s, Entity::Leader(l)))
                    .collect(),
                name,
            );
        }
        if !c.removed.is_empty() {
            doc.remove(&c.removed.iter().map(|(s, _)| *s).collect::<Vec<_>>());
        }
        Ok(())
    });
    CommandResult::Completed {
        output: LeadersEdited {
            changed,
            removed,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
