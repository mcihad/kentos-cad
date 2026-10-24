//! `cad.texts.edit` v1 (docs/adr/0325 §3): Bul ve değiştir, Numarala (AutoCAD's TCOUNT), Yazıları birleştir
//! (TXT2MTXT) and Satırlara ayır, each as one undo step named after the edit. Letters are counted as Unicode scalar
//! values; a text's runs and fields move with its letters by the document's rule (`kentos_domain::text_fields`). The
//! web is paused (docs/adr/0228): the desktop's handler alone, passing `fixtures/commands/v1/cad.texts.edit.json`.
//!
//! Where a text's top is: its first baseline is `rise(up, height, lines, pitch)` under its point (the core's
//! `text::paragraph::rise`, `up` its alignment's share of the height), its top a height over that; line `k` is `k`
//! pitches under the first. Combined and split texts stand at their top, aligned to the top at the same share along.

use std::collections::HashSet;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, NumberPlace, TextAlign, TextEdit, TextEntity, TextField,
    TextRun, TextsEdit, TextsEdited, TextsPlan, Vec2,
};
use kentos_domain::text_fields::replace_letters;
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::jsmath::{PI, cos, sin};
use kentos_geometry_core::text::edit::find_all;
use kentos_geometry_core::text::paragraph::{PITCH, rise};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_blank};
pub use crate::codes;
use crate::fields_edit::{broken_warnings, text_at, written};

/// The largest number Numarala writes.
const MOST: i64 = 1_000_000_000_000_000;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The undo step's name: the edit's.
fn label(e: &TextEdit) -> &'static str {
    match e {
        TextEdit::Replace { .. } => "Bul ve değiştir",
        TextEdit::Number { .. } => "Numarala",
        TextEdit::Combine { .. } => "Yazıları birleştir",
        TextEdit::Split { .. } => "Satırlara ayır",
        TextEdit::Fit { .. } => "Eğriye oturt",
        TextEdit::Turn { .. } => "Doğrultuya döndür",
        TextEdit::Straighten { .. } => "Düzleştir",
    }
}

/// What the checks give: the texts changed and added, as they would be, the texts taken away, how many places were
/// replaced, and the warnings.
#[derive(Default)]
struct Checked {
    changed: Vec<(Slot, Uuid, TextEntity)>,
    added: Vec<TextEntity>,
    removed: Vec<(Slot, Uuid)>,
    replaced: Option<u32>,
    skipped: Option<u32>,
    warnings: Vec<CommandWarning>,
}

/// The texts `uids` names, each once, in order, every one unlocked.
fn texts(doc: &Document, uids: &[String]) -> Result<Vec<(Slot, Uuid, TextEntity)>, Stop> {
    if uids.is_empty() {
        return Err(fail(
            codes::NO_ENTITIES,
            "Yazı verilmedi. En az bir yazının kimliğini verin.".into(),
            "edit.uids",
        ));
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (i, u) in uids.iter().enumerate() {
        if seen.insert(u.as_str()) {
            out.push(text_at(doc, u, &format!("edit.uids[{i}]"), true)?);
        }
    }
    Ok(out)
}

/// `n` with at least `digits` digits, zeros before, a minus before them.
pub fn number_text(n: i64, digits: usize) -> String {
    let body = format!("{:0digits$}", n.unsigned_abs());
    if n < 0 { format!("-{body}") } else { body }
}

/// The top alignment at the same share along as `align`.
fn top_of(align: Option<TextAlign>) -> TextAlign {
    match align.map_or(0.0, |a| {
        kentos_geometry_core::text::TextAlign::from_name(a.name()).map_or(0.0, |c| c.along())
    }) {
        0.5 => TextAlign::TopCenter,
        1.0 => TextAlign::TopRight,
        _ => TextAlign::TopLeft,
    }
}

/// How far a text's top stands over its point along its up direction, with `lines` lines.
fn over(t: &TextEntity, lines: usize) -> f64 {
    let up = t
        .align
        .and_then(|a| kentos_geometry_core::text::TextAlign::from_name(a.name()))
        .map_or(0.0, |a| a.up());
    let pitch = PITCH * t.height * t.paragraph.line_spacing.unwrap_or(1.0);
    t.height - rise(up, t.height, lines, pitch)
}

/// The text's up direction.
fn up_dir(t: &TextEntity) -> (f64, f64) {
    let r = t.rotation * PI / 180.0;
    (-sin(r), cos(r))
}

/// `t`'s paragraphs: each one's first letter, the one past its last, and the line it starts at (its paragraphs'
/// lines laid out in its box; without one, its line breaks).
fn paragraphs(doc: &Document, t: &TextEntity) -> Vec<(usize, usize, usize)> {
    let letters: Vec<char> = t.text.chars().collect();
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in letters.iter().enumerate() {
        if *c == '\n' {
            out.push((start, i, 0));
            start = i + 1;
        }
    }
    out.push((start, letters.len(), 0));
    if t.paragraph.box_width.is_some() {
        // The first laid out line of each paragraph, by the core's wrapping.
        let lines = crate::geometry::text_lines(doc, t);
        for p in &mut out {
            p.2 = lines.iter().position(|(s, _)| *s >= p.0).unwrap_or(0);
        }
    } else {
        for (k, p) in out.iter_mut().enumerate() {
            p.2 = k;
        }
    }
    out
}

/// How many lines `t` is laid out in.
fn line_count(doc: &Document, t: &TextEntity) -> usize {
    if t.paragraph.box_width.is_some() {
        crate::geometry::text_lines(doc, t).len().max(1)
    } else {
        t.text.split('\n').count()
    }
}

/// Letters `a..b` of `t` as a text of their own: the runs over them cut to them, the fields wholly in them.
fn slice(t: &TextEntity, a: usize, b: usize) -> TextEntity {
    let (a32, b32) = (a as u32, b as u32);
    let mut out = t.clone();
    out.text = t.text.chars().skip(a).take(b - a).collect();
    out.paragraph.runs = t
        .paragraph
        .runs
        .iter()
        .filter_map(|r| {
            let (s, e) = (r.start.max(a32), r.end.min(b32));
            (s < e).then(|| TextRun {
                start: s - a32,
                end: e - a32,
                ..r.clone()
            })
        })
        .collect();
    out.paragraph.fields = t
        .paragraph
        .fields
        .iter()
        .filter(|f| f.start >= a32 && f.end <= b32)
        .map(|f| TextField {
            start: f.start - a32,
            end: f.end - a32,
            ..f.clone()
        })
        .collect();
    out
}

/// The object `uid` names (a curve, an edge), at `path`.
fn object_of<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<&'d Entity, Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            path,
        ));
    }
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| {
            fail(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                path,
            )
        })
}

/// Eğriye oturt, Doğrultuya döndür, Düzleştir (docs/adr/0196 §4): the texts the edit takes, as they become, and how
/// many it left; none taken is refused with `why`.
fn on_curves(
    c: &mut Checked,
    list: Vec<(Slot, Uuid, TextEntity)>,
    why: &str,
    mut made: impl FnMut(&TextEntity) -> Option<TextEntity>,
) -> Result<(), Stop> {
    let given = list.len();
    for (slot, u, t) in list {
        if let Some(next) = made(&t) {
            c.changed.push((slot, u, next));
        }
    }
    if c.changed.is_empty() {
        return Err(fail(codes::TEXT_NOT_TAKEN, why.into(), "edit.uids"));
    }
    let left = given - c.changed.len();
    c.skipped = (left > 0).then(|| u32::try_from(left).unwrap_or(u32::MAX));
    Ok(())
}

fn check(doc: &Document, input: &TextsEdit) -> Result<Checked, Stop> {
    let mut c = Checked::default();
    match &input.edit {
        TextEdit::Replace {
            uids,
            find,
            with,
            match_case,
            whole_word,
        } => {
            if find.is_empty() {
                return Err(fail(
                    codes::EMPTY_TEXT,
                    "Aranan yazı boş. Bulunacak harfleri verin.".into(),
                    "edit.find",
                ));
            }
            let mut targets: Vec<(Slot, Uuid, TextEntity)> = Vec::new();
            if uids.is_empty() {
                for e in doc.entities() {
                    if let Entity::Text(t) = e
                        && doc.layers().is_visible(&t.base.layer_id)
                        && let Some(u) = doc.uid(Slot(t.base.id))
                    {
                        targets.push((Slot(t.base.id), u, t.clone()));
                    }
                }
            } else {
                let mut seen = HashSet::new();
                for (i, u) in uids.iter().enumerate() {
                    if seen.insert(u.as_str()) {
                        targets.push(text_at(doc, u, &format!("edit.uids[{i}]"), false)?);
                    }
                }
            }
            let (mut locked, mut blank, mut count, mut open) = (0, 0, 0u32, 0);
            for (slot, u, t) in targets {
                let hits = find_all(&t.text, find, !match_case, *whole_word);
                if hits.is_empty() {
                    if !doc.layers().is_locked(&t.base.layer_id) {
                        open += 1;
                    }
                    continue;
                }
                if doc.layers().is_locked(&t.base.layer_id) {
                    locked += 1;
                    continue;
                }
                open += 1;
                let mut next = t.clone();
                for (s, e) in hits.iter().rev() {
                    next = replace_letters(&next, *s as u32, *e as u32, with, None);
                }
                if is_blank(&next.text) {
                    blank += 1;
                    continue;
                }
                count += hits.len() as u32;
                c.changed.push((slot, u, next));
            }
            if !uids.is_empty() && open == 0 && locked > 0 {
                return Err(fail(
                    codes::LAYER_LOCKED,
                    "Verilen yazıların hepsi kilitli katmanlarda; hiçbiri değişmez. Kilidi Katmanlar panelinden açın.".into(),
                    "edit.uids",
                ));
            }
            if locked > 0 {
                c.warnings.push(CommandWarning {
                    code: codes::LAYER_LOCKED.into(),
                    message: format!("{locked} yazı kilitli katmanda olduğu için değişmedi."),
                    path: Some("edit.uids".into()),
                });
            }
            if blank > 0 {
                c.warnings.push(CommandWarning {
                    code: codes::TEXT_LEFT_BLANK.into(),
                    message: format!(
                        "{blank} yazı değiştirince boş kalacağı için olduğu gibi bırakıldı."
                    ),
                    path: Some("edit.with".into()),
                });
            }
            c.replaced = Some(count);
        }
        TextEdit::Fit { uids, curve } => {
            let list = texts(doc, uids)?;
            let e = object_of(doc, curve, "edit.curve")?;
            if !crate::text_curves::CURVE_KINDS.contains(&e.kind()) {
                return Err(fail(
                    codes::NOT_A_PATH,
                    "Yazıların oturacağı nesne çizgi, yay, daire, alan ya da eğri olmalı.".into(),
                    "edit.curve",
                ));
            }
            let curve = crate::geometry::shape(e);
            let font = crate::geometry::drawing_font(doc.settings().drawing_font);
            on_curves(
                &mut c,
                list,
                "Verilen yazıların hiçbiri eğriye oturmaz: yalnız tek satırlık, nesneye bağlı olmayan yazılar oturur.",
                |t| {
                    crate::text_curves::fits(t)
                        .then(|| crate::text_curves::fitted(t, &curve, font))
                        .flatten()
                },
            )?;
        }
        TextEdit::Turn { uids, edge, at } => {
            let list = texts(doc, uids)?;
            let e = object_of(doc, edge, "edit.edge")?;
            checks::point(*at, "Tıklanan yerin", "edit.at")?;
            let no_edge = || {
                fail(
                    codes::NOT_A_PATH,
                    "Doğrultusu alınacak nesnenin kenarı yok; çizgi, yay ya da alan verin.".into(),
                    "edit.edge",
                )
            };
            if !crate::text_curves::EDGE_KINDS.contains(&e.kind()) {
                return Err(no_edge());
            }
            let degrees = crate::text_curves::turn_toward(
                &crate::geometry::shape(e),
                kentos_geometry_core::vec2::Vec2::new(at.x, at.y),
            )
            .ok_or_else(no_edge)?;
            on_curves(
                &mut c,
                list,
                "Verilen yazıların hiçbiri döndürülmez: eğri boyunca yazıyı önce Düzleştir.",
                |t| {
                    t.path.is_none().then(|| TextEntity {
                        rotation: degrees,
                        ..t.clone()
                    })
                },
            )?;
        }
        TextEdit::Straighten { uids } => {
            let list = texts(doc, uids)?;
            let font = crate::geometry::drawing_font(doc.settings().drawing_font);
            on_curves(
                &mut c,
                list,
                "Verilen yazıların hiçbiri eğri boyunca değil; düzleştirilecek yazı yok.",
                |t| {
                    t.path
                        .is_some()
                        .then(|| crate::text_curves::straightened(t, font))
                        .flatten()
                },
            )?;
        }
        TextEdit::Number {
            uids,
            start,
            step,
            digits,
            prefix,
            suffix,
            place,
        } => {
            let list = texts(doc, uids)?;
            let last = (list.len().saturating_sub(1)) as i64;
            let reach = step.checked_mul(last).and_then(|d| start.checked_add(d));
            if digits.is_some_and(|d| d > 12)
                || start.abs() > MOST
                || reach.is_none_or(|r| r.abs() > MOST)
            {
                return Err(fail(
                    codes::INVALID_NUMBER,
                    format!("Numaralar ±{MOST} içinde, basamak en çok 12 olmalı."),
                    "edit.start",
                ));
            }
            for (i, (slot, u, t)) in list.into_iter().enumerate() {
                let n = start + step * i as i64;
                let body = format!(
                    "{prefix}{}{suffix}",
                    number_text(n, usize::from(digits.unwrap_or(1)))
                );
                let next = match place {
                    NumberPlace::Replace => {
                        let mut next = t.clone();
                        next.text = body;
                        next.paragraph.runs.clear();
                        next.paragraph.fields.clear();
                        next
                    }
                    NumberPlace::Prefix => replace_letters(&t, 0, 0, &body, None),
                    NumberPlace::Suffix => {
                        let end = t.text.chars().count() as u32;
                        replace_letters(&t, end, end, &body, None)
                    }
                };
                c.changed.push((slot, u, next));
            }
        }
        TextEdit::Combine { uids } => {
            let mut list = texts(doc, uids)?;
            if list.len() < 2 {
                return Err(fail(
                    codes::TOO_FEW_OBJECTS,
                    "Birleştirmek için en az iki yazı gerekir.".into(),
                    "edit.uids",
                ));
            }
            // From the highest down, then from the left.
            list.sort_by(|a, b| {
                b.2.p
                    .y
                    .total_cmp(&a.2.p.y)
                    .then(a.2.p.x.total_cmp(&b.2.p.x))
            });
            let (slot, uid, first) = list[0].clone();
            let mut next = first.clone();
            let lines = line_count(doc, &first);
            let (vx, vy) = up_dir(&first);
            let h = over(&first, lines);
            next.p = Vec2 {
                x: first.p.x + h * vx,
                y: first.p.y + h * vy,
            };
            next.align = Some(top_of(first.align));
            next.path = None;
            next.label_of = None;
            next.label_scale = None;
            for (s, u, t) in &list[1..] {
                let at = next.text.chars().count() as u32 + 1;
                next.text.push('\n');
                next.text.push_str(&t.text);
                next.paragraph
                    .runs
                    .extend(t.paragraph.runs.iter().map(|r| TextRun {
                        start: r.start + at,
                        end: r.end + at,
                        ..r.clone()
                    }));
                next.paragraph
                    .fields
                    .extend(t.paragraph.fields.iter().map(|f| TextField {
                        start: f.start + at,
                        end: f.end + at,
                        ..f.clone()
                    }));
                c.removed.push((*s, *u));
            }
            // Touching runs of one format are one.
            next.paragraph.runs = merged(next.paragraph.runs);
            c.changed.push((slot, uid, next));
        }
        TextEdit::Split { uid } => {
            let (slot, u, t) = text_at(doc, uid, "edit.uid", true)?;
            let why = if t.path.is_some() {
                Some("eğri boyunca yazı")
            } else if t
                .paragraph
                .layout
                .as_ref()
                .is_some_and(|l| l.columns.is_some())
            {
                Some("sütunlu yazı")
            } else if t.annotative.is_some() {
                Some("ölçekli yazı")
            } else if !t.text.contains('\n') {
                Some("tek paragraflı yazı")
            } else {
                None
            };
            if let Some(why) = why {
                return Err(fail(
                    codes::NOT_SPLITTABLE,
                    format!("{why} satırlara ayrılmaz."),
                    "edit.uid",
                ));
            }
            let lines = line_count(doc, &t);
            let (vx, vy) = up_dir(&t);
            let pitch = PITCH * t.height * t.paragraph.line_spacing.unwrap_or(1.0);
            let top = over(&t, lines);
            for (a, b, line) in paragraphs(doc, &t) {
                if b == a || t.text.chars().skip(a).take(b - a).all(char::is_whitespace) {
                    continue;
                }
                let mut piece = slice(&t, a, b);
                let d = top - line as f64 * pitch;
                piece.p = Vec2 {
                    x: t.p.x + d * vx,
                    y: t.p.y + d * vy,
                };
                piece.align = Some(top_of(t.align));
                piece.label_of = None;
                piece.label_scale = None;
                c.added.push(piece);
            }
            c.removed.push((slot, u));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (_, u, t) in &mut c.changed {
        if !t.paragraph.fields.is_empty() {
            *t = written(doc, *u, t);
        }
        broken_warnings(t, &mut c.warnings);
    }
    c.changed
        .retain(|(slot, _, t)| doc.get(*slot) != Some(&Entity::Text(t.clone())));
    Ok(c)
}

/// Runs in order with touching runs of one format joined.
fn merged(runs: Vec<TextRun>) -> Vec<TextRun> {
    let mut out: Vec<TextRun> = Vec::with_capacity(runs.len());
    for r in runs {
        if let Some(last) = out.last_mut()
            && last.end == r.start
            && (TextRun {
                start: 0,
                end: 0,
                ..last.clone()
            }) == (TextRun {
                start: 0,
                end: 0,
                ..r.clone()
            })
        {
            last.end = r.end;
            continue;
        }
        out.push(r);
    }
    out
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TextsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &TextsEdit) -> CommandResult<TextsPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (a window's preview).
pub fn plan_in(doc: &Document, input: &TextsEdit) -> CommandResult<TextsPlan> {
    match check(doc, input) {
        Ok(c) => CommandResult::Completed {
            output: TextsPlan {
                texts: c
                    .changed
                    .into_iter()
                    .map(|(_, _, t)| Entity::Text(t))
                    .chain(c.added.into_iter().map(Entity::Text))
                    .collect(),
                removed: c.removed.iter().map(|(_, u)| u.to_string()).collect(),
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: TextsEdit) -> CommandResult<TextsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_TEXTS_EDIT,
        kentos_commands::CAD_TEXTS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: TextsEdit) -> CommandResult<TextsEdited> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let changed: Vec<String> = c.changed.iter().map(|(_, u, _)| u.to_string()).collect();
    let removed: Vec<String> = c.removed.iter().map(|(_, u)| u.to_string()).collect();
    let name = label(&input.edit);
    let wrote: Result<Vec<Slot>, Stop> = doc.transact(name, |doc| {
        if !c.changed.is_empty() {
            doc.update_many(
                c.changed
                    .into_iter()
                    .map(|(s, _, t)| (s, Entity::Text(t)))
                    .collect(),
                name,
            );
        }
        let slots = if c.added.is_empty() {
            Vec::new()
        } else {
            doc.add_many(c.added.into_iter().map(Entity::Text).collect(), name)
                .map_err(|full| {
                    Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None))
                })?
        };
        if !c.removed.is_empty() {
            doc.remove(&c.removed.iter().map(|(s, _)| *s).collect::<Vec<_>>());
        }
        Ok(slots)
    });
    let slots = match wrote {
        Ok(s) => s,
        Err(stop) => return stop.into(),
    };
    let added = slots
        .iter()
        .filter_map(|s| doc.uid(*s))
        .map(|u| u.to_string())
        .collect();
    CommandResult::Completed {
        output: TextsEdited {
            changed,
            added,
            removed,
            replaced: c.replaced,
            skipped: c.skipped,
            revision: doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}
