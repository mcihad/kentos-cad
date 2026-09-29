//! The drawing's block definitions (docs/adr/0144), kept as undoable state
//! like its objects, as the web's `CadDocument` keeps them
//! (apps/web/src/model/document.ts): a definition is added at the end,
//! changed in place (renamed, its base point moved, redefined) or removed,
//! each one undo step or part of the open transaction or group.
//!
//! The block rules (`kentos_contracts::blocks`) hold after every edit: one
//! that would break them is refused with nothing changed, and a definition an
//! insert uses (in the drawing or in another definition) is not removed. The
//! document does not check that an object added to it names a known block;
//! the product commands that write inserts do (docs/adr/0144 §1).

use std::sync::Arc;

use kentos_contracts::blocks::{self as rules, BlockFault};
use kentos_contracts::{BlockDefinition, BlockId, Entity};

use crate::document::Document;
use crate::edit::{Refusal, labels};
use crate::history::Op;

impl Document {
    /// The block definitions in the drawing's order.
    pub fn blocks(&self) -> &[Arc<BlockDefinition>] {
        &self.blocks
    }

    /// The definition with this id.
    pub fn block(&self, id: BlockId) -> Option<&BlockDefinition> {
        self.blocks.iter().find(|b| b.id == id).map(|b| &**b)
    }

    /// The definition with this name, compared as names are (Turkish case folded).
    pub fn block_named(&self, name: &str) -> Option<&BlockDefinition> {
        let key = rules::name_key(name);
        self.blocks
            .iter()
            .find(|b| rules::name_key(&b.name) == key)
            .map(|b| &**b)
    }

    /// How many inserts of the definition the drawing's own objects hold
    /// (not counting those inside other definitions).
    pub fn block_uses(&self, id: BlockId) -> usize {
        self.entities()
            .filter(|e| matches!(e, Entity::Insert(i) if i.block == id))
            .count()
    }

    /// Adds a definition after the others: one undo step “Blok tanımla”.
    /// Refused, with nothing changed, when it breaks a block rule (its name
    /// or id taken, an insert of an unknown block in it, a cycle, too deep).
    pub fn add_block(&mut self, block: BlockDefinition) -> Result<(), Refusal> {
        let mut next: Vec<&BlockDefinition> = self.blocks.iter().map(|b| &**b).collect();
        next.push(&block);
        refused(&next)?;
        let op = Op::BlockAdd {
            index: self.blocks.len(),
            block: Arc::new(block),
        };
        self.record(vec![op], labels::BLOCK_ADD);
        Ok(())
    }

    /// Replaces the definition with the same id, in its place: one undo step
    /// “Blok değiştir”. Every insert of it shows the new one. `false` when no
    /// definition has the id or nothing changes; refused as `add_block` is.
    pub fn update_block(&mut self, block: BlockDefinition) -> Result<bool, Refusal> {
        let Some(index) = self.blocks.iter().position(|b| b.id == block.id) else {
            return Ok(false);
        };
        if *self.blocks[index] == block {
            return Ok(false);
        }
        let mut next: Vec<&BlockDefinition> = self.blocks.iter().map(|b| &**b).collect();
        next[index] = &block;
        refused(&next)?;
        let op = Op::BlockUpdate {
            before: self.blocks[index].clone(),
            after: Arc::new(block),
        };
        self.record(vec![op], labels::BLOCK_CHANGE);
        Ok(true)
    }

    /// Removes a definition no insert uses: one undo step “Blok sil”; undo
    /// puts it back in its place. `false` when no definition has the id.
    pub fn remove_block(&mut self, id: BlockId) -> Result<bool, Refusal> {
        let Some(index) = self.blocks.iter().position(|b| b.id == id) else {
            return Ok(false);
        };
        if let Some(reason) = self.block_removal_refused(id) {
            return Err(Refusal(reason));
        }
        let op = Op::BlockRemove {
            index,
            block: self.blocks[index].clone(),
        };
        self.record(vec![op], labels::BLOCK_REMOVE);
        Ok(true)
    }

    /// Why [`Document::remove_block`] would refuse, or none: an insert of the
    /// definition in the drawing, or in another definition.
    pub fn block_removal_refused(&self, id: BlockId) -> Option<String> {
        let name = &self.block(id)?.name;
        let placed = self.block_uses(id);
        if placed > 0 {
            return Some(format!(
                "“{name}” bloğu çizimde {placed} kez yerleştirilmiş; silinemez. Önce yerleştirmelerini silin."
            ));
        }
        self.blocks
            .iter()
            .find(|b| b.id != id && rules::uses(&b.entities, id) > 0)
            .map(|holder| {
                format!(
                    "“{name}” bloğu “{}” bloğunun içinde kullanılıyor; silinemez.",
                    holder.name
                )
            })
    }

    /// Applies a definition op (history.rs); the definitions are found by id.
    pub(crate) fn apply_block_op(&mut self, op: &Op) {
        match op {
            Op::BlockAdd { index, block } => {
                let at = (*index).min(self.blocks.len());
                self.blocks.insert(at, block.clone());
            }
            Op::BlockRemove { block, .. } => self.blocks.retain(|b| b.id != block.id),
            Op::BlockUpdate { after, .. } => {
                if let Some(slot) = self.blocks.iter_mut().find(|b| b.id == after.id) {
                    *slot = after.clone();
                }
            }
            _ => {}
        }
    }
}

/// The refusal a list of definitions earns, in the documents' words.
fn refused(blocks: &[&BlockDefinition]) -> Result<(), Refusal> {
    let checked = rules::definitions(blocks).and_then(|index| {
        for (i, block) in blocks.iter().enumerate() {
            rules::inserts(&block.entities, Some(i), &index)?;
        }
        rules::nesting(blocks, &index).map(|_| ())
    });
    checked.map_err(|fault: BlockFault| {
        Refusal(fault.message(|i| blocks.get(i).map_or("", |b| b.name.as_str())))
    })
}
