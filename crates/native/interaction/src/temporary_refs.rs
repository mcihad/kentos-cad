//! The temporary distances' second reference and a wall length's fixed end, as the session keeps them (docs/adr/0728
//! §2, §3): a side with a face and a grid shows the one chosen, a small square at its reference switching it; an edge
//! whose two ends can move keeps the one chosen. Kept by the object's persistent id for the whole session, never
//! written to the drawing or its undo history (Revit's “saved on a per-session basis”).

use std::collections::HashMap;

use kentos_commands::{DistanceTo, WallAnchor};
use kentos_domain::Uuid;

use crate::Vec2;
use crate::tool::Context;

/// How near a square a click takes it, logical pixels.
const REACH_PX: f64 = 8.0;

/// The session's choices: each side's reference by (object, edge or face, side), each edge's fixed end.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choices {
    refs: HashMap<(Uuid, u32, u32), DistanceTo>,
    anchors: HashMap<(Uuid, u32), WallAnchor>,
}

impl Choices {
    /// The reference chosen for `side` of `uid`'s `edge` (a wall's edge, an opening's face, 0 for a column).
    pub fn to(&self, uid: Uuid, edge: u32, side: u32) -> Option<DistanceTo> {
        self.refs.get(&(uid, edge, side)).copied()
    }

    pub fn set_to(&mut self, uid: Uuid, edge: u32, side: u32, to: DistanceTo) {
        self.refs.insert((uid, edge, side), to);
    }

    /// The end chosen to stay when `uid`'s `edge` changes its length.
    pub fn anchor(&self, uid: Uuid, edge: u32) -> Option<WallAnchor> {
        self.anchors.get(&(uid, edge)).copied()
    }

    pub fn set_anchor(&mut self, uid: Uuid, edge: u32, anchor: WallAnchor) {
        self.anchors.insert((uid, edge), anchor);
    }
}

/// The reference to show when a side has `face` and `grid` candidates: the chosen one, else the face; none without
/// either.
pub fn shown(face: bool, grid: bool, chosen: Option<DistanceTo>) -> Option<DistanceTo> {
    match (face, grid) {
        (true, true) => Some(chosen.unwrap_or(DistanceTo::Face)),
        (true, false) => Some(DistanceTo::Face),
        (false, true) => Some(DistanceTo::Grid),
        (false, false) => None,
    }
}

/// The other candidate.
pub fn other(to: DistanceTo) -> DistanceTo {
    match to {
        DistanceTo::Face => DistanceTo::Grid,
        DistanceTo::Grid => DistanceTo::Face,
    }
}

/// What a square on a temporary dimension does when clicked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Switch {
    /// The side's reference becomes `to`.
    Reference { uid: Uuid, edge: u32, side: u32, to: DistanceTo },
    /// The edge's fixed end becomes `anchor`.
    Anchor { uid: Uuid, edge: u32, anchor: WallAnchor },
}

impl Switch {
    /// Kept in `choices`.
    pub fn apply(self, choices: &mut Choices) {
        match self {
            Switch::Reference { uid, edge, side, to } => choices.set_to(uid, edge, side, to),
            Switch::Anchor { uid, edge, anchor } => choices.set_anchor(uid, edge, anchor),
        }
    }
}

/// Of the squares within reach of `screen`, the nearest's switch.
pub fn square_at(squares: &[(Vec2, Switch)], screen: [f64; 2], cx: &Context<'_>) -> Option<Switch> {
    squares
        .iter()
        .filter_map(|(at, switch)| {
            let p = cx.view.to_screen(*at);
            let d = (p[0] - screen[0]).hypot(p[1] - screen[1]);
            (d <= REACH_PX).then_some((d, *switch))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, s)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_side_shows_its_face_until_its_grid_is_chosen() {
        assert_eq!(shown(true, true, None), Some(DistanceTo::Face));
        assert_eq!(shown(true, true, Some(DistanceTo::Grid)), Some(DistanceTo::Grid));
        assert_eq!(shown(false, true, Some(DistanceTo::Face)), Some(DistanceTo::Grid), "only the grid");
        assert_eq!(shown(true, false, Some(DistanceTo::Grid)), Some(DistanceTo::Face), "only the face");
        assert_eq!(shown(false, false, None), None);
        let mut c = Choices::default();
        let uid = Uuid::from_u128(7);
        Switch::Reference {
            uid,
            edge: 1,
            side: 0,
            to: DistanceTo::Grid,
        }
        .apply(&mut c);
        assert_eq!(c.to(uid, 1, 0), Some(DistanceTo::Grid));
        assert_eq!(c.to(uid, 0, 0), None, "per edge");
        Switch::Anchor {
            uid,
            edge: 1,
            anchor: WallAnchor::End,
        }
        .apply(&mut c);
        assert_eq!(c.anchor(uid, 1), Some(WallAnchor::End));
    }
}
