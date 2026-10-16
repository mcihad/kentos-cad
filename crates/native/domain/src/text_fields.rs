//! A text's fields kept with their sources (docs/adr/0325 §2). Before a step is recorded (`commit`, with the linked
//! texts and the associative hatches), its changes are read:
//!
//! - a text the step edited by hand (its letters changed, its fields not) keeps the fields its edit did not touch,
//!   shifted with the letters before them; a field whose letters the edit touched goes (its value stays as text);
//! - a text whose field reads an object the step added, changed or removed, a text the step added or gave new fields,
//!   and every text with a project's or a layer's field has its fields written again: a value that changed takes its
//!   range's place, the runs and fields after it shift; a value that cannot be read is `####` with the reason;
//! - a text whose field reads another text is written after that one; texts that read each other in a ring are broken
//!   (`cycle`).
//!
//! The changes join the step: one undo takes them back with the rest. The rule a value is written by is
//! [`Document::field_value`]; `scripts/fixtures/field_cases.py` writes it again without KentOS's code.

use std::collections::{BTreeSet, HashSet, VecDeque};
use std::sync::Arc;

use kentos_contracts::{
    BROKEN_FIELD, DrawingUnit, Entity, FieldBroken, FieldFormat, FieldSource, FieldUnit,
    ObjectValue, ProjectValue, TextEntity, TextField, TextRun, VariableValue,
};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::{
    Entity as CoreEntity, entity_anchor, entity_area, entity_length, is_closed_path,
};

use crate::document::Document;
use crate::history::Op;
use crate::identity::Uuid;
use crate::store::Stored;

/// A field's value: its letters, or why there are none.
pub type FieldValue = Result<String, FieldBroken>;

/// Where letter `p` of a text goes when the letters `start..end` become `len` letters: before them it stays, after
/// them it shifts, inside them it stays inside the new ones. Letters put in between two (`start == end`) push a range's
/// start (`end_of` false) at that place along, not a range's end (`end_of` true): a range ending there stays as it is.
pub fn shifted(p: u32, start: u32, end: u32, len: u32, end_of: bool) -> u32 {
    if start == end {
        return if p > start || (p == start && !end_of) {
            p + len
        } else {
            p
        };
    }
    if p <= start {
        p
    } else if p >= end {
        p - (end - start) + len
    } else {
        start + (p - start).min(len)
    }
}

/// `text` with its letters `start..end` replaced by `with`, its runs and fields after them shifted, a run inside
/// them kept inside the new letters (one that comes to nothing goes). Field `own` is the one being written: its range
/// becomes the new letters; any other field the range overlaps goes.
pub fn replace_letters(
    text: &TextEntity,
    start: u32,
    end: u32,
    with: &str,
    own: Option<usize>,
) -> TextEntity {
    let letters: Vec<char> = text.text.chars().collect();
    let len = with.chars().count() as u32;
    let mut out = text.clone();
    out.text = letters[..start as usize]
        .iter()
        .copied()
        .chain(with.chars())
        .chain(letters[end as usize..].iter().copied())
        .collect();
    out.paragraph.runs = text
        .paragraph
        .runs
        .iter()
        .filter_map(|r| {
            let (s, e) = (
                shifted(r.start, start, end, len, false),
                shifted(r.end, start, end, len, true),
            );
            (s < e).then(|| TextRun {
                start: s,
                end: e,
                ..r.clone()
            })
        })
        .collect();
    out.paragraph.fields = text
        .paragraph
        .fields
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            if Some(i) == own {
                return Some(TextField {
                    start,
                    end: start + len,
                    ..f.clone()
                });
            }
            // A field the letters touch goes; one they are put in between keeps out of them.
            let apart = if start == end {
                f.end <= start || f.start >= start
            } else {
                f.end <= start || f.start >= end
            };
            apart.then(|| TextField {
                start: shifted(f.start, start, end, len, false),
                end: shifted(f.end, start, end, len, true),
                ..f.clone()
            })
        })
        .collect();
    out
}

/// The fields of `before` kept through a hand edit that turned its letters into `after`'s: the letters both keep at
/// their start and end are found; a field wholly in them stays (shifted), one the changed letters touch goes.
pub fn fields_through_edit(before: &TextEntity, after: &str) -> Vec<TextField> {
    let a: Vec<char> = before.text.chars().collect();
    let b: Vec<char> = after.chars().collect();
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (from, to, len) = (
        head as u32,
        (a.len() - tail) as u32,
        (b.len() - tail - head) as u32,
    );
    before
        .paragraph
        .fields
        .iter()
        .filter_map(|f| {
            if f.end <= from {
                Some(f.clone())
            } else if f.start >= to {
                let d = |p: u32| p - to + from + len;
                Some(TextField {
                    start: d(f.start),
                    end: d(f.end),
                    ..f.clone()
                })
            } else {
                None
            }
        })
        .collect()
}

/// A number in metres (or square metres) written by `format`, the project's decimals and unit given.
pub fn number_text(metres: f64, unit_metres: f64, decimals: usize, comma: bool) -> String {
    let t = fixed(metres / unit_metres, decimals);
    if comma { t.replace('.', ",") } else { t }
}

/// The drawing unit's written name.
pub fn unit_name(unit: DrawingUnit) -> &'static str {
    match unit {
        DrawingUnit::Mm => "mm",
        DrawingUnit::Cm => "cm",
        DrawingUnit::M => "m",
        DrawingUnit::In => "in",
        DrawingUnit::Ft => "ft",
    }
}

/// The core's reading of a contract object.
fn core(e: &Entity) -> Option<CoreEntity> {
    let json = serde_json::to_string(e).ok()?;
    CoreEntity::from_json(&Json::parse(&json).ok()?).ok()
}

impl Document {
    /// The value field `field` of a text writes now, its format applied (docs/adr/0325 §1); `letters` reads a text's
    /// letters by persistent id (the document's, or another text written before it in the same pass).
    pub fn field_value(
        &self,
        field: &TextField,
        letters: &dyn Fn(Uuid) -> Option<String>,
    ) -> FieldValue {
        let plain = FieldFormat::default();
        let format = field.format.as_ref().unwrap_or(&plain);
        let s = self.settings();
        let raw: FieldValue = match &field.source {
            FieldSource::Object { uid, value, name } => {
                let uid = Uuid::from_bytes(uid.0);
                let Some(object) = self.store.slot_of(uid).and_then(|s| self.store.get(s)) else {
                    return Err(FieldBroken::Removed);
                };
                let entity = &*object.entity;
                let base = entity.base();
                let drawing = s.drawing_unit.unwrap_or_default();
                let length_unit = format
                    .unit
                    .map_or(1.0 / drawing.per_metre(), FieldUnit::metres);
                let length_decimals = format
                    .decimals
                    .map_or(s.length_decimals as usize, usize::from);
                let number = |v: Option<f64>, area: bool| -> FieldValue {
                    let v = v.filter(|v| v.is_finite()).ok_or(FieldBroken::NoValue)?;
                    Ok(if area {
                        let unit = format.unit.map_or_else(
                            || match s.area_unit {
                                kentos_contracts::AreaUnit::M2 => 1.0,
                                kentos_contracts::AreaUnit::Donum => 1000.0,
                                kentos_contracts::AreaUnit::Ha => 10_000.0,
                            },
                            FieldUnit::metres,
                        );
                        number_text(
                            v,
                            unit,
                            format
                                .decimals
                                .map_or(s.area_decimals as usize, usize::from),
                            format.comma,
                        )
                    } else {
                        number_text(v, length_unit, length_decimals, format.comma)
                    })
                };
                match value {
                    ObjectValue::Layer => self
                        .layers()
                        .get(&base.layer_id)
                        .map(|l| l.name.clone())
                        .filter(|n| !n.is_empty())
                        .ok_or(FieldBroken::NoValue),
                    ObjectValue::Label => base
                        .label
                        .clone()
                        .filter(|l| !l.is_empty())
                        .ok_or(FieldBroken::NoValue),
                    ObjectValue::Attribute => match name.as_deref().and_then(|n| base.attrs.get(n))
                    {
                        None => Err(FieldBroken::NotNamed),
                        Some(v) if v.is_empty() => Err(FieldBroken::NoValue),
                        Some(v) => Ok(v.clone()),
                    },
                    ObjectValue::Text => match entity {
                        Entity::Text(_) => letters(uid)
                            .filter(|t| !t.is_empty())
                            .ok_or(FieldBroken::NoValue),
                        _ => Err(FieldBroken::NoValue),
                    },
                    ObjectValue::Area
                    | ObjectValue::Perimeter
                    | ObjectValue::Length
                    | ObjectValue::X
                    | ObjectValue::Y => {
                        let shape = core(entity).map(|c| c.shape).ok_or(FieldBroken::NoValue)?;
                        match value {
                            ObjectValue::Area => number(entity_area(&shape), true),
                            ObjectValue::Perimeter => {
                                let closed =
                                    entity_area(&shape).is_some() || is_closed_path(&shape);
                                number(closed.then(|| entity_length(&shape)).flatten(), false)
                            }
                            ObjectValue::Length => number(entity_length(&shape), false),
                            ObjectValue::X => number(entity_anchor(&shape).map(|p| p.x), false),
                            _ => number(entity_anchor(&shape).map(|p| p.y), false),
                        }
                    }
                }
            }
            FieldSource::Project { value, name } => match value {
                ProjectValue::Name => Some(self.name.clone())
                    .filter(|n| !n.is_empty())
                    .ok_or(FieldBroken::NoValue),
                ProjectValue::Scale => Ok(format!("1:{}", fixed(s.plot_scale, 0))),
                ProjectValue::Unit => Ok(unit_name(s.drawing_unit.unwrap_or_default()).to_owned()),
                ProjectValue::Crs => Ok(match &s.custom_crs {
                    Some(c) => c.name.clone(),
                    None if s.srid == 0 => "Yerel".to_owned(),
                    None => format!("EPSG:{}", s.srid),
                }),
                ProjectValue::Variable => {
                    match name
                        .as_deref()
                        .and_then(|n| s.variables.iter().find(|v| v.name == n))
                    {
                        None => Err(FieldBroken::NotNamed),
                        Some(v) => match &v.value {
                            VariableValue::Null => Err(FieldBroken::NoValue),
                            VariableValue::Bool(b) => {
                                Ok(if *b { "Evet" } else { "Hayır" }.to_owned())
                            }
                            VariableValue::Number(x) => Ok(match format.decimals {
                                Some(d) => number_text(*x, 1.0, usize::from(d), format.comma),
                                None => fixed(*x, kentos_geometry_core::display::NOISE_DECIMALS)
                                    .trim_end_matches('0')
                                    .trim_end_matches('.')
                                    .to_owned(),
                            }),
                            VariableValue::Text(t) if t.is_empty() => Err(FieldBroken::NoValue),
                            VariableValue::Text(t) => Ok(t.clone()),
                        },
                    }
                }
            },
        };
        raw.map(|v| format!("{}{v}{}", format.prefix, format.suffix))
    }

    /// `text` with every field written again: a value that changed takes its range's place,
    /// one that cannot be read is `####`. A field that reads a text on `ring` (the texts whose fields read each other
    /// in a ring with this one) is broken. `letters` as [`Document::field_value`]'s.
    pub fn fields_written(
        &self,
        text: &TextEntity,
        ring: &HashSet<Uuid>,
        letters: &dyn Fn(Uuid) -> Option<String>,
    ) -> TextEntity {
        let mut out = text.clone();
        for i in 0..text.paragraph.fields.len() {
            let f = out.paragraph.fields[i].clone();
            let value = match f.source.object().map(|u| Uuid::from_bytes(u.0)) {
                Some(uid)
                    if matches!(
                        f.source,
                        FieldSource::Object {
                            value: ObjectValue::Text,
                            ..
                        }
                    ) && ring.contains(&uid) =>
                {
                    Err(FieldBroken::Cycle)
                }
                _ => self.field_value(&f, letters),
            };
            let (letters_now, broken) = match value {
                Ok(v) => (v, None),
                Err(b) => (BROKEN_FIELD.to_owned(), Some(b)),
            };
            let own: String = out
                .text
                .chars()
                .skip(f.start as usize)
                .take((f.end - f.start) as usize)
                .collect();
            if own != letters_now {
                out = replace_letters(&out, f.start, f.end, &letters_now, Some(i));
            }
            out.paragraph.fields[i].broken = broken;
        }
        out
    }

    /// The texts whose fields read each other in a ring with the text of persistent id `of`: following its text
    /// fields' sources (texts with fields) back to it.
    pub fn field_ring(&self, of: Uuid) -> HashSet<Uuid> {
        // The texts each reaches, then those that reach `of` back.
        let reads = |uid: Uuid| -> Vec<Uuid> {
            let Some(Entity::Text(t)) = self
                .store
                .slot_of(uid)
                .and_then(|s| self.store.get(s))
                .map(|s| &*s.entity)
            else {
                return Vec::new();
            };
            t.paragraph
                .fields
                .iter()
                .filter_map(|f| match &f.source {
                    FieldSource::Object {
                        uid,
                        value: ObjectValue::Text,
                        ..
                    } => Some(Uuid::from_bytes(uid.0)),
                    _ => None,
                })
                .collect()
        };
        // Depth-first from `of`, keeping the path; a path that comes back to `of` puts its texts on the ring.
        let mut ring = HashSet::new();
        let mut stack: Vec<(Uuid, Vec<Uuid>)> = vec![(of, reads(of))];
        let mut on_path: Vec<Uuid> = vec![of];
        let mut seen: HashSet<Uuid> = HashSet::from([of]);
        while let Some((_, next)) = stack.last_mut() {
            match next.pop() {
                Some(n) if n == of => {
                    ring.extend(on_path.iter().copied());
                }
                Some(n) if seen.insert(n) => {
                    on_path.push(n);
                    let more = reads(n);
                    stack.push((n, more));
                }
                Some(_) => {}
                None => {
                    stack.pop();
                    on_path.pop();
                }
            }
        }
        ring
    }

    /// Whether the text of persistent id `from` reads the text `to`, by its own text fields or through others'.
    pub fn field_reaches(&self, from: Uuid, to: Uuid) -> bool {
        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut next = vec![from];
        while let Some(u) = next.pop() {
            if u == to {
                return true;
            }
            if !seen.insert(u) {
                continue;
            }
            if let Some(Entity::Text(t)) = self
                .store
                .slot_of(u)
                .and_then(|s| self.store.get(s))
                .map(|s| &*s.entity)
            {
                for f in &t.paragraph.fields {
                    if let FieldSource::Object {
                        uid,
                        value: ObjectValue::Text,
                        ..
                    } = &f.source
                    {
                        next.push(Uuid::from_bytes(uid.0));
                    }
                }
            }
        }
        false
    }

    /// The text of persistent id `uid` with its fields written again now, or none when it is no text with fields.
    pub fn text_with_fields_written(&self, uid: Uuid) -> Option<TextEntity> {
        let Entity::Text(t) = &*self.store.get(self.store.slot_of(uid)?)?.entity else {
            return None;
        };
        if t.paragraph.fields.is_empty() {
            return None;
        }
        let letters = |u: Uuid| match self
            .store
            .slot_of(u)
            .and_then(|s| self.store.get(s))
            .map(|s| &*s.entity)
        {
            Some(Entity::Text(t)) => Some(t.text.clone()),
            _ => None,
        };
        Some(self.fields_written(t, &self.field_ring(uid), &letters))
    }

    /// What keeps the texts' fields with their sources after `ops` (a step about to be recorded), applied: the step
    /// takes them in.
    pub(crate) fn follow_fields(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_fields() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut queue: VecDeque<Uuid> = VecDeque::new();
        let mut pending: BTreeSet<Uuid> = BTreeSet::new();
        let mut changed: Vec<Uuid> = Vec::new();
        for op in ops {
            match op {
                Op::Add(s) => {
                    changed.push(s.uid);
                    if let Entity::Text(t) = &*s.entity
                        && !t.paragraph.fields.is_empty()
                        && pending.insert(s.uid)
                    {
                        queue.push_back(s.uid);
                    }
                }
                Op::Remove(s) => changed.push(s.uid),
                Op::Update { before, after } => {
                    changed.push(after.uid);
                    let (Entity::Text(was), Entity::Text(now)) = (&*before.entity, &*after.entity)
                    else {
                        continue;
                    };
                    // A hand edit: the fields it did not touch kept, shifted.
                    if was.text != now.text
                        && was.paragraph.fields == now.paragraph.fields
                        && !now.paragraph.fields.is_empty()
                    {
                        let mut kept = now.clone();
                        kept.paragraph.fields = fields_through_edit(was, &now.text);
                        if let Some(current) = self.store.get(after.slot()).cloned() {
                            let op = Op::Update {
                                after: Stored {
                                    uid: current.uid,
                                    entity: Arc::new(Entity::Text(kept)),
                                },
                                before: current,
                            };
                            self.apply_op(&op);
                            out.push(op);
                        }
                    }
                    if (was.paragraph.fields != now.paragraph.fields || was.text != now.text)
                        && !now.paragraph.fields.is_empty()
                        && pending.insert(after.uid)
                    {
                        queue.push_back(after.uid);
                    }
                }
                _ => {}
            }
        }
        // The texts that read what changed, and those read every step (a project's or a layer's field).
        for uid in changed {
            for t in self.store.fielded_from(uid).collect::<Vec<_>>() {
                if let Some(u) = self.uid(t)
                    && pending.insert(u)
                {
                    queue.push_back(u);
                }
            }
        }
        for slot in self.store.always_fielded_slots().collect::<Vec<_>>() {
            if let Some(u) = self.uid(slot)
                && pending.insert(u)
            {
                queue.push_back(u);
            }
        }
        // Each text after the texts it reads; what a written text changes, the texts that read it follow.
        let mut budget = 8 * (self.store.fielded_count() + 1);
        while let Some(uid) = queue.pop_front() {
            if budget == 0 {
                break;
            }
            budget -= 1;
            self.write_fields_of(uid, &mut pending, &mut queue, &mut out, &mut HashSet::new());
        }
        out
    }

    /// Writes the fields of text `uid` if it waits (`pending`), the waiting texts it reads first.
    fn write_fields_of(
        &mut self,
        uid: Uuid,
        pending: &mut BTreeSet<Uuid>,
        queue: &mut VecDeque<Uuid>,
        out: &mut Vec<Op>,
        visiting: &mut HashSet<Uuid>,
    ) {
        if !pending.contains(&uid) || !visiting.insert(uid) {
            return;
        }
        let Some(current) = self
            .store
            .slot_of(uid)
            .and_then(|s| self.store.get(s))
            .cloned()
        else {
            pending.remove(&uid);
            return;
        };
        let Entity::Text(text) = &*current.entity else {
            pending.remove(&uid);
            return;
        };
        let ring = self.field_ring(uid);
        for f in &text.paragraph.fields {
            if let FieldSource::Object {
                uid: of,
                value: ObjectValue::Text,
                ..
            } = &f.source
            {
                let of = Uuid::from_bytes(of.0);
                if !ring.contains(&of) {
                    self.write_fields_of(of, pending, queue, out, visiting);
                }
            }
        }
        pending.remove(&uid);
        let Some(next) = self.text_with_fields_written(uid) else {
            return;
        };
        if next != *text {
            let letters_changed = next.text != text.text;
            let op = Op::Update {
                after: Stored {
                    uid: current.uid,
                    entity: Arc::new(Entity::Text(next.clone())),
                },
                before: current,
            };
            self.apply_op(&op);
            out.push(op);
            if letters_changed {
                for t in self.store.fielded_from(uid).collect::<Vec<_>>() {
                    if let Some(u) = self.uid(t)
                        && pending.insert(u)
                    {
                        queue.push_back(u);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_shift_around_a_replaced_range() {
        assert_eq!(shifted(2, 3, 5, 4, false), 2);
        assert_eq!(shifted(5, 3, 5, 4, false), 7);
        assert_eq!(shifted(4, 3, 5, 0, true), 3);
        assert_eq!(shifted(4, 3, 5, 4, true), 4);
        // Put in at 3: a start there moves on, an end there stays.
        assert_eq!(shifted(3, 3, 3, 2, false), 5);
        assert_eq!(shifted(3, 3, 3, 2, true), 3);
    }
}
