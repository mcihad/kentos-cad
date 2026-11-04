//! Bound nested expansion before constructing the core's flattened definitions.
use super::{recognizable, refuse};
use crate::checks::Stop;
use kentos_commands::{BlockId, Entity, codes};
use kentos_domain::Document;
use kentos_geometry_core::block::Blocks;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const LIMIT: usize = 200_000;
struct Review<'a, F> {
    doc: &'a Document,
    stopped: &'a mut F,
    stack: HashSet<BlockId>,
    costs: HashMap<BlockId, usize>,
    definitions: Vec<Arc<kentos_commands::BlockDefinition>>,
    work: usize,
}
impl<F: FnMut() -> bool> Review<'_, F> {
    fn charge(&mut self, amount: usize) -> Result<(), Stop> {
        if (self.stopped)() {
            return Err(Stop::Cancelled);
        }
        self.work = self.work.saturating_add(amount);
        if self.work > LIMIT {
            return Err(refuse(
                codes::RECOGNITION_SCOPE_LIMIT,
                "İç blokların açılmış toplam içeriği 200 000 iş sınırını aşıyor; daha küçük bir kapsam seçin. Hiçbir parça atlanmadı.",
                "source",
            ));
        }
        Ok(())
    }
    fn visit(&mut self, id: BlockId) -> Result<usize, Stop> {
        self.charge(1)?;
        if let Some(cost) = self.costs.get(&id) {
            return Ok(*cost);
        }
        if self.stack.len() >= 16 || !self.stack.insert(id) {
            return Err(refuse(
                codes::RECOGNITION_UNSUPPORTED,
                "İç blokta döngü veya 16 düzey sınırını aşan yol var; kaynak değiştirilmedi.",
                "source",
            ));
        }
        let b = self.doc.block(id).ok_or_else(|| {
            refuse(
                codes::RECOGNITION_UNSUPPORTED,
                "İç blok tanımı bulunamadı; kaynak değiştirilmedi.",
                "source",
            )
        })?;
        if b.dynamic.is_some() || b.xref.is_some() {
            return Err(refuse(
                codes::RECOGNITION_UNSUPPORTED,
                "Bu tanıma statik iç bloklar içindir; dinamik veya dış referans davranışı yok sayılmaz.",
                "source",
            ));
        }
        let mut cost = 1usize;
        for e in &b.entities {
            self.charge(1)?;
            if !recognizable(e) {
                return Err(refuse(
                    codes::RECOGNITION_UNSUPPORTED,
                    "İç blok desteklenmeyen bir parça içeriyor. Tanım bütünü atlanmadan reddedildi; analitik 2B çizgi, yay, çember, çoklu çizgi, düz yazı ve statik blok kullanılabilir.",
                    "source",
                ));
            }
            cost = cost.saturating_add(match e {
                Entity::Insert(i) => self.visit(i.block)?,
                Entity::Polyline(p) => p.pts.len().max(1),
                Entity::Text(t) => 1 + t.text.len() / 128,
                _ => 1,
            });
            if cost > LIMIT {
                self.charge(LIMIT)?;
            }
        }
        for a in &b.attributes {
            let size = 1 + a.value.as_ref().map_or(0, |v| v.len() / 128);
            cost = cost.saturating_add(size);
            self.charge(size)?;
        }
        self.charge(cost)?; // Cached flattening of each definition is charged separately.
        self.stack.remove(&id);
        self.costs.insert(id, cost);
        self.definitions.push(Arc::new(b.clone()));
        Ok(cost)
    }
}

pub(super) fn checked(
    doc: &Document,
    ids: impl Iterator<Item = BlockId>,
    stopped: &mut impl FnMut() -> bool,
) -> Result<Blocks, Stop> {
    let mut review = Review {
        doc,
        stopped,
        stack: HashSet::new(),
        costs: HashMap::new(),
        definitions: Vec::new(),
        work: 0,
    };
    for id in ids {
        let cost = review.visit(id)?;
        review.charge(cost)?; // Every placed expansion, including repeated roots, is bounded.
    }
    Ok(crate::blocks::core_blocks_in(&review.definitions, crate::geometry::drawing_font(doc.settings().drawing_font)))
}
