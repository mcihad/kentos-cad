//! Öznitelikler's rows for a wall's building parts (docs/adr/0771): the line that stays where it is when its width or
//! its style changes (Kalınlıkta sabit: a choice of the session, not of the wall), its ends' node decisions, and a
//! slab's bound outline (Sınır: Bağımsız kontur, Duvarlara bağlı, Mahale bağlı; Ayır, Kaynağa bağla). Every change goes
//! through `cad.walls.edit` and `cad.slabs.edit`.

use std::sync::atomic::{AtomicU8, Ordering};

use kentos_commands::{
    SlabBoundary, SlabFace, SlabFields, SlabSourceKind, SlabsOperation, Vec2, WallFields,
    WallReference,
};

/// The line a width or style change keeps, this session (index into `WallReference::ALL`).
static KEEP: AtomicU8 = AtomicU8::new(0);

/// The line a width or style change keeps now.
pub(crate) fn keep() -> WallReference {
    WallReference::ALL[usize::from(KEEP.load(Ordering::Relaxed)).min(WallReference::ALL.len() - 1)]
}

pub(crate) fn set_keep(r: WallReference) {
    let k = WallReference::ALL.iter().position(|x| *x == r).unwrap_or(0);
    KEEP.store(k as u8, Ordering::Relaxed);
}

/// An end's node decision as the row says it: Otomatik, or each component's passes.
pub(crate) fn node_text(pass: Option<&Vec<u8>>) -> String {
    match pass {
        None => "Otomatik".into(),
        Some(p) if p.iter().all(|&x| x == 0) => "Yüzde durur".into(),
        Some(p) if p.iter().all(|&x| x >= kentos_commands::MAX_NODE_PASS) => {
            "Konaktan geçer".into()
        }
        Some(p) => format!(
            "Elle: {}",
            p.iter().map(u8::to_string).collect::<Vec<_>>().join(", ")
        ),
    }
}

/// The choices an end's node decision offers: by the priorities, every component stopping at the host's face, every
/// component passing the whole host; the decision for a wall of these components.
pub(crate) fn node_choices(w: &WallFields) -> [(&'static str, Option<Vec<u8>>); 3] {
    let n = w.components.len().max(1);
    [
        ("Otomatik", None),
        ("Yüzde durur", Some(vec![0; n])),
        (
            "Konaktan geçer",
            Some(vec![kentos_commands::MAX_NODE_PASS; n]),
        ),
    ]
}

/// A slab's bound as the row says it.
pub(crate) fn bound_text(s: &SlabFields) -> String {
    match &s.bound {
        None => "Bağımsız kontur".into(),
        Some(b) => {
            let source = match (b.source, b.face) {
                (SlabSourceKind::Walls, SlabFace::Outline) => "Dış yüzlere bağlı",
                (SlabSourceKind::Walls, SlabFace::Room) => "Odaya bağlı",
                (SlabSourceKind::Space, _) => "Mahale bağlı",
            };
            if b.broken.is_some() {
                format!("Kopuk · {source}")
            } else {
                source.into()
            }
        }
    }
}

/// The operations a slab's Sınır row offers, each with its label and whether it is the slab's now: Bağımsız kontur
/// (Ayır), bound to the walls' outer faces or a room's, read round the slab's label point.
pub(crate) fn bound_choices(s: &SlabFields) -> Vec<(&'static str, bool, Option<SlabsOperation>)> {
    let at =
        kentos_native_application::spaces::label_point(&s.parts).map(|p| Vec2 { x: p.x, y: p.y });
    let offset = s.bound.as_ref().map(|b| b.offset).filter(|o| *o != 0.0);
    let now = s.bound.as_ref().map(|b| (b.source, b.face));
    let bind = |boundary: SlabBoundary| {
        at.map(|seed| SlabsOperation::Bind {
            seed: Some(seed),
            boundary: Some(boundary),
            space: None,
            offset,
        })
    };
    vec![
        (
            "Bağımsız kontur",
            now.is_none(),
            now.is_some().then_some(SlabsOperation::Detach),
        ),
        (
            "Dış yüzlere bağlı",
            now == Some((SlabSourceKind::Walls, SlabFace::Outline)),
            bind(SlabBoundary::Outline),
        ),
        (
            "Odaya bağlı",
            now == Some((SlabSourceKind::Walls, SlabFace::Room)),
            bind(SlabBoundary::Room),
        ),
    ]
}
