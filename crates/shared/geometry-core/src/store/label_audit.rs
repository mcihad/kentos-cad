//! What a placing came to, counted (docs/adr/0533 §6): of the labels [`Store::place_labels`] shows, how many were
//! placed, left without a place, hidden by hand, pinned, forced over another, how many placed boxes really cross
//! another, and how many repeat a text another placed label of the same layer already has. The placing's own
//! states say the first five; the last two are read back from the labels as drawn, so a regression in the engine's
//! room keeping shows here whatever the states say.

use std::collections::HashMap;

use crate::labels::engine::{CURVED, HIDDEN, OVERLAPPING, PINNED, UNPLACED};
use crate::labels::geom::Obb;
use crate::vec2::Vec2;

use super::labels::LABEL_STRIDE;
use super::placing::{LABEL_PLACED, LABEL_PLACED_LINE, Shown};

/// One label of a placing, as drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditedLabel {
    /// Its object's store id.
    pub id: f64,
    /// Its class's index.
    pub class: u16,
    /// The engine's state bits.
    pub state: u32,
    /// Its block's middle on the ground.
    pub at: [f64; 2],
    /// Its box crosses a placed label's (a curved label is not boxed here).
    pub collides: bool,
    /// Another placed label of the same object's layer and class has the same text and was placed first.
    pub repeats: bool,
}

impl AuditedLabel {
    pub fn placed(&self) -> bool {
        self.state & (UNPLACED | HIDDEN) == 0
    }

    pub fn unplaced(&self) -> bool {
        self.state & UNPLACED != 0
    }

    pub fn hidden(&self) -> bool {
        self.state & HIDDEN != 0
    }

    pub fn pinned(&self) -> bool {
        self.state & PINNED != 0
    }

    pub fn forced(&self) -> bool {
        self.state & OVERLAPPING != 0
    }
}

/// The labels of a placing at `scale` px per metre, each with what is the matter with it. `layer_of` names an
/// object's layer (a text repeats within a layer and class).
pub fn audit(shown: &Shown, scale: f64, layer_of: &dyn Fn(f64) -> u32) -> Vec<AuditedLabel> {
    struct Frame {
        id: f64,
        class: u16,
        state: u32,
        at: [f64; 2],
        boxed: Option<Obb>,
        text: String,
    }
    let mut frames: Vec<Frame> = Vec::new();
    for r in shown.records.chunks_exact(LABEL_STRIDE) {
        if r[1] == LABEL_PLACED {
            let state = r[8] as u32;
            // Boxes in pixels (the ground's metres times the scale); a curved label has none.
            let boxed = (state & CURVED == 0).then(|| {
                Obb::new(
                    Vec2::new(r[2] * scale, r[3] * scale),
                    r[4].to_radians(),
                    r[5] / 2.0,
                    r[6] / 2.0,
                )
            });
            frames.push(Frame {
                id: r[0],
                class: r[7] as u16,
                state,
                at: [r[2], r[3]],
                boxed,
                text: String::new(),
            });
        } else if r[1] == LABEL_PLACED_LINE
            && let Some(f) = frames.last_mut()
            && let Some(t) = shown.texts.get(r[6] as usize)
        {
            if !f.text.is_empty() {
                f.text.push('\n');
            }
            f.text.push_str(t);
        }
    }
    let placed: Vec<usize> = (0..frames.len())
        .filter(|&i| frames[i].state & (UNPLACED | HIDDEN) == 0)
        .collect();
    let mut collides = vec![false; frames.len()];
    // A grid of the boxes' extents finds the neighbours of each.
    let cell = 64.0;
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let aabbs: Vec<Option<[f64; 4]>> = frames
        .iter()
        .map(|f| f.boxed.as_ref().map(Obb::aabb))
        .collect();
    for &i in &placed {
        let Some(b) = aabbs[i] else { continue };
        for gx in (b[0] / cell).floor() as i64..=(b[2] / cell).floor() as i64 {
            for gy in (b[1] / cell).floor() as i64..=(b[3] / cell).floor() as i64 {
                grid.entry((gx, gy)).or_default().push(i);
            }
        }
    }
    for list in grid.values() {
        for (n, &i) in list.iter().enumerate() {
            for &j in &list[n + 1..] {
                if collides[i] && collides[j] {
                    continue;
                }
                if let (Some(a), Some(b)) = (&frames[i].boxed, &frames[j].boxed)
                    && a.overlaps(b)
                {
                    collides[i] = true;
                    collides[j] = true;
                }
            }
        }
    }
    let mut seen: HashMap<(u32, u16, &str), ()> = HashMap::new();
    let mut repeats = vec![false; frames.len()];
    for &i in &placed {
        let f = &frames[i];
        if f.text.is_empty() {
            continue;
        }
        if seen
            .insert((layer_of(f.id), f.class, f.text.as_str()), ())
            .is_some()
        {
            repeats[i] = true;
        }
    }
    frames
        .iter()
        .enumerate()
        .map(|(i, f)| AuditedLabel {
            id: f.id,
            class: f.class,
            state: f.state,
            at: f.at,
            collides: collides[i],
            repeats: repeats[i],
        })
        .collect()
}
