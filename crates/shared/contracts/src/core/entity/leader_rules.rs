//! A leader's rules (docs/adr/0326, 0205 §7): how many arms it has, what makes its arms, blocks and note wrong
//! together, and how long its arrowhead may be (moved out of entity.rs, which is over the size limit, CLAUDE.md §8).

use super::LeaderEntity;

/// The most arms a leader has.
pub const MAX_LEADER_ARMS: usize = 64;

/// What is wrong with a leader's arms, blocks and note together (docs/adr/0326): the field and the refusal's words.
pub fn leader_problem(l: &LeaderEntity) -> Option<(&'static str, String)> {
    if l.arms.len() + 1 > MAX_LEADER_ARMS {
        return Some(("arms", format!("Bir kılavuzun en çok {MAX_LEADER_ARMS} kolu olur; {} verildi.", l.arms.len() + 1)));
    }
    if let Some(i) = l.arms.iter().position(|a| a.pts.is_empty()) {
        return Some(("arms", format!("{}. kolun ucu yok: her kolun en az bir noktası olmalı.", i + 2)));
    }
    if l.arms.iter().flat_map(|a| &a.pts).any(|p| !(p.x.is_finite() && p.y.is_finite())) {
        return Some(("arms", "Kolun noktaları sonlu olmalı.".to_owned()));
    }
    if !l.blocks.is_empty() && l.text.is_some() {
        return Some(("blocks", "Blok içerikli kılavuzun notu olmaz: notu ya da blokları kaldırın.".to_owned()));
    }
    let mut seen = std::collections::HashSet::new();
    if l.blocks.iter().any(|b| !seen.insert(*b)) {
        return Some(("blocks", "Kılavuzun bir bloğu iki kez verilmiş.".to_owned()));
    }
    None
}

/// The shortest and the longest a leader's arrowhead may be, times its
/// note's height (docs/adr/0205 §7).
pub const MIN_LEADER_ARROW: f64 = 0.1;
pub const MAX_LEADER_ARROW: f64 = 10.0;

/// Whether `size` may be a leader's arrowhead size.
pub fn leader_arrow_holds(size: f64) -> bool {
    size.is_finite() && (MIN_LEADER_ARROW..=MAX_LEADER_ARROW).contains(&size)
}
