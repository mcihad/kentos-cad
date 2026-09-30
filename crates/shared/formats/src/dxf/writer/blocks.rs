//! The drawing's blocks as DXF blocks (docs/adr/0144 §5). Every definition
//! an insert of the export places, and every one nested in those, is a
//! BLOCK with its block record, the ones it holds written before it (a
//! reader that resolves an INSERT as it meets it finds its block). A
//! definition's objects are owned by its record and keep their layer, which
//! Patlat and AutoCAD's EXPLODE put them back on (none of its own is layer
//! 0); BYBLOCK where they have no colour or weight of their own, so an
//! insert draws them in its own, as KentOS does. A name keeps its letters:
//! a character DXF refuses becomes "_" (“*” among them: a name starting
//! with it is an anonymous block), and a name DXF's case-blind comparison
//! takes for another's gets " (2)"; every change is said. A definition's
//! attribute definitions (§7) follow its objects as ATTDEFs; a tag's white
//! space and control characters become "_" (DXF's tags have none), and said.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{BlockDefinition, BlockId, Bounds, Entity, InsertEntity, Vec2};

use super::layers::{REFUSED, key, valid_name};
use super::{Handles, Justified, Out};
use crate::geom::v;
use crate::math::{cos, sin};
use crate::report::Report;

/// A definition written: its base point, the extent of its objects in its
/// own coordinates (none when it draws nothing), and its attributes' tags
/// with the DXF tags they were written as (§7).
pub(super) struct Written {
    pub base: Vec2,
    pub extent: Option<Bounds>,
    pub tags: Vec<(String, String)>,
}

/// A definition's attribute tags as DXF writes them (§7): white space and
/// control characters "_", a tag taken by an earlier one "_2", "_3" …; every
/// change said.
pub(super) fn tags(
    def: &BlockDefinition,
    name: &str,
    report: &mut Report,
) -> Vec<(String, String)> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(def.attributes.len());
    for a in &def.attributes {
        let base: String = a
            .tag
            .chars()
            .map(|c| {
                if c.is_whitespace() || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let mut tag = base.clone();
        let mut k = 2;
        while taken.contains(&tag.to_uppercase()) {
            tag = format!("{base}_{k}");
            k += 1;
        }
        taken.insert(tag.to_uppercase());
        if tag != a.tag {
            report.note(
                "Öznitelik etiketi",
                &format!(
                    "“{name}” bloğunun “{}” etiketi “{tag}” olarak yazıldı (DXF'te etiket boşluk içermez, büyük küçük harf ayırmaz)",
                    a.tag
                ),
                0,
            );
        }
        out.push((a.tag.clone(), tag));
    }
    out
}

/// The DXF name of every definition written, and what changed on the way (said).
pub(super) fn names<'b>(
    blocks: impl IntoIterator<Item = &'b BlockDefinition>,
    report: &mut Report,
) -> HashMap<BlockId, String> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut out = HashMap::new();
    for b in blocks {
        let base = if b.name.trim().is_empty() {
            "Blok".to_owned()
        } else {
            valid_name(&b.name)
        };
        let mut name = base.clone();
        let mut k = 2;
        while taken.contains(&key(&name)) {
            let suffix = format!(" ({k})");
            let room = 255 - suffix.chars().count();
            name = base.chars().take(room).chain(suffix.chars()).collect();
            k += 1;
        }
        if name != b.name {
            let mut why: Vec<&str> = Vec::new();
            if b.name
                .chars()
                .any(|c| c.is_control() || REFUSED.contains(&c))
            {
                why.push("DXF'in kabul etmediği karakterler “_” oldu");
            }
            if b.name.trim() != b.name
                || b.name.trim().is_empty()
                || b.name.trim().chars().count() > 255
            {
                why.push("boşlukları ya da uzunluğu düzeltildi");
            }
            if name != base {
                why.push("aynı adlı başka bir blok var; DXF blok adları büyük/küçük harf ayırmaz");
            }
            report.note(
                "Blok adı",
                &format!(
                    "“{}” bloğu “{name}” adıyla yazıldı ({})",
                    b.name,
                    why.join("; ")
                ),
                0,
            );
        }
        taken.insert(key(&name));
        out.insert(b.id, name);
    }
    out
}

/// The definitions the objects place, and those nested in them, each after
/// the ones it holds. A definition that holds itself (through others) is
/// written once; its insert of itself is left for the writer to say.
pub(super) fn order<'b>(
    blocks: &'b [BlockDefinition],
    entities: &[Entity],
) -> Vec<&'b BlockDefinition> {
    let by_id: HashMap<BlockId, &BlockDefinition> = blocks.iter().map(|b| (b.id, b)).collect();
    let mut done: HashSet<BlockId> = HashSet::new();
    let mut open: HashSet<BlockId> = HashSet::new();
    let mut out = Vec::new();
    let placed = entities.iter().filter_map(|e| match e {
        Entity::Insert(i) => Some(i.block),
        _ => None,
    });
    for root in placed {
        if done.contains(&root) || !by_id.contains_key(&root) {
            continue;
        }
        // Depth first without recursion: each open definition and the next of its objects to look at.
        open.insert(root);
        let mut stack: Vec<(BlockId, usize)> = vec![(root, 0)];
        while let Some(top) = stack.last_mut() {
            let def = by_id[&top.0];
            let mut next = None;
            while let Some(e) = def.entities.get(top.1) {
                top.1 += 1;
                if let Entity::Insert(i) = e
                    && by_id.contains_key(&i.block)
                    && !done.contains(&i.block)
                    && !open.contains(&i.block)
                {
                    next = Some(i.block);
                    break;
                }
            }
            let current = top.0;
            match next {
                Some(child) => {
                    open.insert(child);
                    stack.push((child, 0));
                }
                None => {
                    stack.pop();
                    open.remove(&current);
                    done.insert(current);
                    out.push(def);
                }
            }
        }
    }
    out
}

/// Where a point of a definition lands under an insert: from the base
/// point, mirrored in the x axis when the insert is, scaled, turned, moved
/// to the insert's point (`kentos_geometry_core::block`'s similarity).
pub(super) fn placed(i: &InsertEntity, base: Vec2, q: Vec2) -> Vec2 {
    let flip = if i.mirror { -1.0 } else { 1.0 };
    let (x, y) = (i.scale * (q.x - base.x), flip * i.scale * (q.y - base.y));
    let (s, c) = (sin(i.rotation), cos(i.rotation));
    v(i.p.x + c * x - s * y, i.p.y + s * x + c * y)
}

/// A definition's BLOCK: its head (name, base point, description), its
/// objects (`body`, owned by `record`), ENDBLK. The description is one line;
/// a line break in it becomes a space, and is said.
pub(super) fn block(
    out: &mut Out,
    handles: &mut Handles,
    report: &mut Report,
    (record, name): (u64, &str),
    def: &BlockDefinition,
    body: &Out,
    tags: &[(String, String)],
) {
    let h = handles.take();
    out.str(0, "BLOCK");
    out.handle(5, h);
    out.handle(330, record);
    out.str(100, "AcDbEntity");
    out.str(8, "0");
    out.str(100, "AcDbBlockBegin");
    out.str(2, name);
    // 2: the block has attribute definitions.
    out.int(70, if def.attributes.is_empty() { 0 } else { 2 });
    out.xyz(10, def.base);
    out.str(3, name);
    out.str(1, "");
    if let Some(d) = def.description.as_deref().filter(|d| !d.is_empty()) {
        let line: String = d
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        if line != d {
            report.note(
                "Blok açıklaması",
                "DXF'te açıklama tek satırdır; satır sonları boşluk oldu",
                0,
            );
        }
        out.str(4, &line);
    }
    out.s.push_str(&body.s);
    // Its attribute definitions (§7), after its objects: on 0, the insert's.
    for (a, (_, tag)) in def.attributes.iter().zip(tags) {
        let (given, asked) = (
            a.value.as_deref().unwrap_or(""),
            a.prompt.as_deref().unwrap_or(""),
        );
        let (value, prompt) = (one_line(given), one_line(asked));
        if value != given || prompt != asked {
            report.note("Öznitelik", ONE_LINE, 0);
        }
        let h = handles.take();
        out.str(0, "ATTDEF");
        out.handle(5, h);
        out.handle(330, record);
        out.str(100, "AcDbEntity");
        out.str(8, "0");
        out.str(100, "AcDbText");
        // A block's definition shows its attributes' tags.
        let vertical = out.text(
            &value,
            tag,
            &Justified {
                p: a.p,
                height: a.height,
                rotation: a.rotation,
                align: a.align,
                width_factor: a.width_factor,
            },
        );
        out.str(100, "AcDbAttributeDefinition");
        out.str(3, &prompt);
        out.str(2, tag);
        out.int(70, 0);
        if vertical != 0 {
            out.int(74, vertical);
        }
    }
    let h = handles.take();
    out.str(0, "ENDBLK");
    out.handle(5, h);
    out.handle(330, record);
    out.str(100, "AcDbEntity");
    out.str(8, "0");
    out.str(100, "AcDbBlockEnd");
}

/// What an attribute's text lost on the way to DXF's one line (said).
pub(super) const ONE_LINE: &str =
    "satır sonları ve denetim karakterleri boşluk oldu (DXF'te öznitelik tek satırdır)";

/// A text as DXF's one line holds it: control characters become spaces.
pub(super) fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
