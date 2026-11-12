//! KentOS's library symbols in a command's own step (docs/adr/0721): a north arrow or a revision's delta is the
//! drawing's block of that name when it has one, else KentOS's library's definition (`kentos-temel`), added in the
//! same undo step as what places it, under an id the drawing does not have. A symbol's size is said on paper: its
//! insert's scale is the size times the drawing's scale over the block's width, its drawing east to west, texts
//! aside.

use std::sync::OnceLock;

use kentos_commands::blocks::name_key;
use kentos_commands::{BlockDefinition, BlockId, Entity};
use kentos_domain::Document;

/// A symbol's block: the drawing's own, or the library's to add (`added`).
pub(crate) struct Symbol {
    pub def: BlockDefinition,
    pub added: bool,
}

/// KentOS's own library's definitions, read once (Mutfak dizisi's too, docs/adr/0706).
pub(crate) fn library() -> &'static [BlockDefinition] {
    static BLOCKS: OnceLock<Vec<BlockDefinition>> = OnceLock::new();
    BLOCKS.get_or_init(|| {
        let text = crate::blocks_library::BUILTIN_LIBRARIES[0].1;
        serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|file| serde_json::from_value(file["blocks"].clone()).ok())
            .unwrap_or_default()
    })
}

/// The block `name` places: the drawing's (Turkish case folded), else the library's under `fresh`'s id when the
/// drawing has its own id; none when neither has it.
pub(crate) fn symbol(doc: &Document, name: &str, fresh: impl FnOnce() -> BlockId) -> Option<Symbol> {
    if let Some(b) = doc.block_named(name) {
        return Some(Symbol {
            def: b.clone(),
            added: false,
        });
    }
    let key = name_key(name);
    let mut def = library().iter().find(|b| name_key(&b.name) == key)?.clone();
    if doc.block(def.id).is_some() {
        def.id = fresh();
    }
    Some(Symbol { def, added: true })
}

/// A definition's width east to west at scale 1: its drawing's, texts and attributes aside; 1 when it draws nothing
/// else.
pub(crate) fn width(def: &BlockDefinition) -> f64 {
    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    for e in &def.entities {
        if matches!(e, Entity::Text(_)) {
            continue;
        }
        let b = kentos_geometry_core::entity::entity_bounds(&crate::geometry::shape(e));
        min = min.min(b.min_x);
        max = max.max(b.max_x);
    }
    let w = max - min;
    if w.is_finite() && w > 0.0 { w } else { 1.0 }
}

/// An insert's scale that makes `def` `paper_mm` millimetres across on paper at the drawing's scale.
pub(crate) fn scale_for(doc: &Document, def: &BlockDefinition, paper_mm: f64) -> f64 {
    paper_mm / 1000.0 * doc.settings().plot_scale / width(def)
}
