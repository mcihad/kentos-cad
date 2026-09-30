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
//! takes for another's gets " (2)"; every change is said.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{BlockDefinition, BlockId, Bounds, Entity, InsertEntity, Vec2};

use super::layers::{REFUSED, key, valid_name};
use super::{Handles, Out};
use crate::geom::v;
use crate::math::{cos, sin};
use crate::report::Report;

/// A definition written: its base point and the extent of its objects in
/// its own coordinates (none when it draws nothing).
pub(super) struct Written {
    pub base: Vec2,
    pub extent: Option<Bounds>,
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
) {
    let h = handles.take();
    out.str(0, "BLOCK");
    out.handle(5, h);
    out.handle(330, record);
    out.str(100, "AcDbEntity");
    out.str(8, "0");
    out.str(100, "AcDbBlockBegin");
    out.str(2, name);
    out.int(70, 0);
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
    let h = handles.take();
    out.str(0, "ENDBLK");
    out.handle(5, h);
    out.handle(330, record);
    out.str(100, "AcDbEntity");
    out.str(8, "0");
    out.str(100, "AcDbBlockEnd");
}
