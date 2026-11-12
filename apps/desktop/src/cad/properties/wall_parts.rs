//! Öznitelikler's rows for a wall's building parts (docs/adr/0771): the line that stays where it is when its width or
//! its style changes (Kalınlıkta sabit: a choice of the session, not of the wall), its ends' node decisions, and a
//! slab's bound outline (Sınır: Bağımsız kontur, Duvarlara bağlı, Mahale bağlı; Ayır, Kaynağa bağla). Every change goes
//! through `cad.walls.edit` and `cad.slabs.edit`. Mahale bağlı names the space and offers the spaces of the slab's
//! level, the nearest first (docs/adr/0791; ADR 0771 §7 left it to the command).

use std::sync::atomic::{AtomicU8, Ordering};

use kentos_commands::{
    Entity, SlabBoundary, SlabFace, SlabFields, SlabSourceKind, SlabsOperation, Vec2, WallFields,
    WallReference,
};
use kentos_domain::{Document, Slot};

/// The most spaces the Sınır row offers.
const MOST_SPACES: usize = 12;

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

/// A space as the Sınır row names it: its number and name, else “Mahal”.
pub(crate) fn space_words(s: &kentos_commands::SpaceFields) -> String {
    let words = [s.number.as_deref(), s.name.as_deref()]
        .into_iter()
        .flatten()
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if words.is_empty() { "Mahal".into() } else { words }
}

/// The spaces a slab may be bound to: those of its level (every one when it has none), by their persistent ids and
/// words, the nearest to the slab's label point first, at most [`MOST_SPACES`]; the one it is bound to is always
/// among them.
pub(crate) fn space_choices(doc: &Document, s: &SlabFields) -> Vec<(String, String)> {
    let at = kentos_native_application::spaces::label_point(&s.parts);
    let bound = s.bound.as_ref().and_then(|b| b.space.clone());
    let mut found: Vec<(f64, String, String)> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Space(sp) if s.arch.level.is_none() || sp.space.arch.level == s.arch.level => {
                let uid = doc.uid(Slot(e.base().id))?.to_string();
                let d = match (at, kentos_native_application::spaces::label_point(&sp.space.parts)) {
                    (Some(a), Some(b)) => (a.x - b.x).hypot(a.y - b.y),
                    _ => f64::INFINITY,
                };
                Some((d, uid, space_words(&sp.space)))
            }
            _ => None,
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.2.cmp(&b.2)));
    let mut out: Vec<(String, String)> = found.into_iter().map(|(_, u, w)| (u, w)).collect();
    if let Some(i) = bound.as_ref().and_then(|b| out.iter().position(|(u, _)| u == b))
        && i >= MOST_SPACES
    {
        let kept = out.remove(i);
        out.insert(0, kept);
    }
    out.truncate(MOST_SPACES);
    out
}

/// A slab's bound as the row says it; a space's words when it is bound to one (`spaces`, from [`space_choices`]).
pub(crate) fn bound_text(s: &SlabFields, spaces: &[(String, String)]) -> String {
    match &s.bound {
        None => "Bağımsız kontur".into(),
        Some(b) => {
            let named;
            let source = match (b.source, b.face) {
                (SlabSourceKind::Walls, SlabFace::Outline) => "Dış yüzlere bağlı",
                (SlabSourceKind::Walls, SlabFace::Room) => "Odaya bağlı",
                (SlabSourceKind::Space, _) => {
                    match b.space.as_ref().and_then(|u| spaces.iter().find(|(id, _)| id == u)) {
                        Some((_, words)) => {
                            named = format!("Mahale bağlı · {words}");
                            named.as_str()
                        }
                        None => "Mahale bağlı",
                    }
                }
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
/// (Ayır), bound to the walls' outer faces or a room's, read round the slab's label point, and bound to each space of
/// `spaces` (from [`space_choices`]; the slab's offset kept).
pub(crate) fn bound_choices(s: &SlabFields, spaces: &[(String, String)]) -> Vec<(String, bool, Option<SlabsOperation>)> {
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
    let bound_space = s.bound.as_ref().and_then(|b| b.space.as_deref());
    let mut out = vec![
        (
            "Bağımsız kontur".to_owned(),
            now.is_none(),
            now.is_some().then_some(SlabsOperation::Detach),
        ),
        (
            "Dış yüzlere bağlı".to_owned(),
            now == Some((SlabSourceKind::Walls, SlabFace::Outline)),
            bind(SlabBoundary::Outline),
        ),
        (
            "Odaya bağlı".to_owned(),
            now == Some((SlabSourceKind::Walls, SlabFace::Room)),
            bind(SlabBoundary::Room),
        ),
    ];
    for (uid, words) in spaces {
        let chosen = bound_space == Some(uid.as_str());
        out.push((
            format!("Mahale bağlı · {words}"),
            chosen,
            (!chosen).then(|| SlabsOperation::Bind {
                seed: None,
                boundary: None,
                space: Some(uid.clone()),
                offset,
            }),
        ));
    }
    out
}
