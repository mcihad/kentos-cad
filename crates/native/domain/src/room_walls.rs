//! A room's wall covering thickness (docs/adr/8076 §7; EK-6 Tablo 6.58 `Pset_SpaceCoveringRequirements.
//! WallCoveringThickness`): the finish of the walls' faces it lies along. A wall lists its components from its left
//! face (`WallFields::components`); a room along its left face sees the first, along its right face the last. A
//! component of a finish's strength (priority 5 and weaker; plaster and boards, docs/adr/0340 §1) is a covering, its
//! thickness the room's; a face whose outer component is stronger has no covering.
//!
//! - **Along a face:** a straight edge of the room's rings that lies on a straight run of a wall of its storey, at the
//!   face's distance from the wall's path (within a micrometre: a room made from the walls has its faces' lines) and
//!   parallel to it.
//! - **The room's:** the thickness more than half of its edges on faces hold, by length; none where none does, or no
//!   edge lies on a face (a room drawn apart from its walls, a curved wall).

use kentos_contracts::{Entity, SpaceFields, Vec2, WallFields};

use crate::document::Document;

/// How close an edge lies to a face's line, metres: a room made from the walls has its faces' own lines.
const ON: f64 = 1e-6;

/// The least strength of a finish (docs/adr/0340 §1: a core's 1, a finish's 5).
const FINISH: u8 = 5;

impl Document {
    /// The room's wall covering thickness, metres; none where its walls' faces do not say one.
    pub fn room_wall_finish(&self, s: &SpaceFields) -> Option<f64> {
        let edges: Vec<(Vec2, Vec2)> = s
            .parts
            .iter()
            .flat_map(|p| std::iter::once(&p.outer).chain(&p.holes))
            .flat_map(|r| {
                let n = r.pts.len();
                (0..n).filter_map(move |i| {
                    let straight = r.bulges.as_ref().and_then(|b| b.get(i)).is_none_or(|b| *b == 0.0);
                    (straight && n >= 2).then(|| (r.pts[i], r.pts[(i + 1) % n]))
                })
            })
            .filter(|(a, b)| len(sub(*b, *a)) > ON)
            .collect();
        if edges.is_empty() {
            return None;
        }
        let walls: Vec<&WallFields> = self
            .entities()
            .filter_map(|e| match e {
                Entity::Wall(w) if w.wall.arch.level == s.arch.level => Some(&w.wall),
                _ => None,
            })
            .collect();
        // (thickness or none for a face without a covering, length along it).
        let mut along: Vec<(Option<f64>, f64)> = Vec::new();
        for (a, b) in edges {
            let mid = Vec2 { x: (a.x + b.x) / 2.0, y: (a.y + b.y) / 2.0 };
            let dir = unit(sub(b, a));
            if let Some(t) = walls.iter().find_map(|w| facing(w, mid, dir)) {
                along.push((t, len(sub(b, a))));
            }
        }
        let total: f64 = along.iter().map(|(_, l)| l).sum();
        let mut held: Vec<(f64, f64)> = Vec::new();
        for (t, l) in &along {
            let Some(t) = t else { continue };
            match held.iter_mut().find(|(x, _)| (*x - *t).abs() <= 1e-9) {
                Some(h) => h.1 += l,
                None => held.push((*t, *l)),
            }
        }
        held.into_iter().find(|(_, l)| *l > total / 2.0).map(|(t, _)| t)
    }
}

/// Whether the edge through `mid` along `dir` lies on one of the wall's faces: then the covering it sees there (its
/// outer component's thickness when that is a finish, else none).
fn facing(w: &WallFields, mid: Vec2, dir: Vec2) -> Option<Option<f64>> {
    let n = w.pts.len();
    let runs = if w.closed { n } else { n.saturating_sub(1) };
    let left = w.offset + w.justify.left_face(w.width);
    for i in 0..runs {
        let straight = w.bulges.as_ref().and_then(|b| b.get(i)).is_none_or(|b| *b == 0.0);
        let (p, q) = (w.pts[i], w.pts[(i + 1) % n]);
        let length = len(sub(q, p));
        if !straight || length <= ON {
            continue;
        }
        let d = unit(sub(q, p));
        if (d.x * dir.y - d.y * dir.x).abs() > ON {
            continue;
        }
        let rel = sub(mid, p);
        let along = rel.x * d.x + rel.y * d.y;
        if along < -ON || along > length + ON {
            continue;
        }
        // Positive to the path's left, as the faces' distances are.
        let side = d.x * rel.y - d.y * rel.x;
        if (side - left).abs() <= ON {
            return Some(cover(w, w.components.first()));
        }
        if (side - (left - w.width)).abs() <= ON {
            return Some(cover(w, w.components.last()));
        }
    }
    None
}

/// The covering a face's outer component makes: its thickness (one without a thickness takes what the others leave of
/// the wall's width) when it is a finish; none for a stronger one, or a wall without components.
fn cover(w: &WallFields, c: Option<&kentos_contracts::WallComponent>) -> Option<f64> {
    let c = c?;
    if c.priority.unwrap_or(kentos_contracts::DEFAULT_COMPONENT_PRIORITY) < FINISH {
        return None;
    }
    let t = match c.thickness {
        Some(t) => t,
        None => w.width - w.components.iter().filter_map(|c| c.thickness).sum::<f64>(),
    };
    (t > 0.0).then_some(t)
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 { x: a.x - b.x, y: a.y - b.y }
}

fn len(v: Vec2) -> f64 {
    (v.x * v.x + v.y * v.y).sqrt()
}

fn unit(v: Vec2) -> Vec2 {
    let l = len(v);
    Vec2 { x: v.x / l, y: v.y / l }
}

#[cfg(test)]
mod tests {
    use crate::wall_height::tests::document;

    /// A wall on the ground floor along `pts`, 0.20 wide on its path: İç sıva 2 cm on its left face, brick, Dış sıva
    /// 3 cm on its right (a finish 5, the brick 1); `inner` the left face's component's strength.
    fn wall(pts: [[f64; 2]; 2], inner: u8) -> serde_json::Value {
        serde_json::json!({
            "kind": "wall",
            "pts": [{"x": pts[0][0], "y": pts[0][1]}, {"x": pts[1][0], "y": pts[1][1]}],
            "width": 0.2, "justify": "center", "arch": {"level": "zemin"},
            "components": [
                {"name": "İç sıva", "thickness": 0.02, "priority": inner},
                {"name": "Tuğla", "priority": 1},
                {"name": "Dış sıva", "thickness": 0.03, "priority": 5}
            ]
        })
    }

    /// The room 0–4 × 1–4 on the ground floor.
    fn room() -> kentos_contracts::SpaceFields {
        serde_json::from_value(serde_json::json!({
            "parts": [{"outer": {"pts": [
                {"x": 0.0, "y": 1.0}, {"x": 4.0, "y": 1.0}, {"x": 4.0, "y": 4.0}, {"x": 0.0, "y": 4.0}
            ]}}],
            "arch": {"level": "zemin"}
        }))
        .unwrap()
    }

    #[test]
    fn a_room_s_wall_covering_is_the_finish_of_the_faces_it_lies_along() {
        // Under it a wall eastwards, its left face at y 1; over it one westwards, its left face at y 4: both show the
        // room their 2 cm İç sıva.
        let doc = document(serde_json::json!([wall([[0.0, 0.9], [4.0, 0.9]], 5), wall([[4.0, 4.1], [0.0, 4.1]], 5)]));
        assert_eq!(doc.room_wall_finish(&room()), Some(0.02));
        // The one over it eastwards: its right face, 3 cm Dış sıva, holds half the length, not more: none.
        let doc = document(serde_json::json!([wall([[0.0, 0.9], [4.0, 0.9]], 5), wall([[0.0, 4.1], [4.0, 4.1]], 5)]));
        assert_eq!(doc.room_wall_finish(&room()), None);
        // A third along its west side with 2 cm: 7 m of 11 hold 2 cm.
        let doc = document(serde_json::json!([
            wall([[0.0, 0.9], [4.0, 0.9]], 5),
            wall([[0.0, 4.1], [4.0, 4.1]], 5),
            wall([[-0.1, 4.0], [-0.1, 1.0]], 5)
        ]));
        assert_eq!(doc.room_wall_finish(&room()), Some(0.02));
        // A face whose outer layer is the core (its strength 1) has no covering: no thickness holds.
        let doc = document(serde_json::json!([wall([[0.0, 0.9], [4.0, 0.9]], 1), wall([[4.0, 4.1], [0.0, 4.1]], 1)]));
        assert_eq!(doc.room_wall_finish(&room()), None);
        // A room 5 cm off its walls' faces lies along none.
        let doc = document(serde_json::json!([wall([[0.0, 0.85], [4.0, 0.85]], 5), wall([[4.0, 4.15], [0.0, 4.15]], 5)]));
        assert_eq!(doc.room_wall_finish(&room()), None);
    }
}
