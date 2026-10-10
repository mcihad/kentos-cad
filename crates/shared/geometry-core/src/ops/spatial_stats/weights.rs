//! Komşuluk (docs/adr/0238 §7): each place's neighbours with their weights,
//! sorted by order: a fixed distance band (1), inverse distance within the
//! band (1 / max(d, 1 m)) or the k nearest (1). The band, when not given, is
//! the largest nearest-neighbour distance, so every place has a neighbour.

use super::MOST_PAIRS;
use super::kdtree::{KdTree, dist2};
use super::nearest::nearest_distances;
use crate::jsmath::js_max;
use crate::vec2::Vec2;

/// How neighbours are found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Concept {
    Band,
    Inverse,
    Nearest,
}

impl Concept {
    pub fn from_key(key: &str) -> Option<Concept> {
        Some(match key {
            "band" => Concept::Band,
            "inverse" => Concept::Inverse,
            "nearest" => Concept::Nearest,
            _ => return None,
        })
    }
}

/// The neighbour lists, the band used (none for the k nearest) and the k used.
pub struct Neighbours {
    pub lists: Vec<Vec<(u32, f64)>>,
    pub band: Option<f64>,
    pub k: usize,
}

/// The refusal when the band brings too many pairs.
pub fn too_many(what: &str) -> String {
    format!(
        "{what} çok geniş: komşu çiftleri {MOST_PAIRS}'u aşıyor; {}",
        if what == "Bant" {
            "bandı küçültün ya da k en yakın komşuyu seçin."
        } else {
            "yarıçapı küçültün."
        }
    )
}

/// The places' neighbours; `own`: each place is its own neighbour with
/// weight 1 too (Gi\*), in its order among the others.
pub fn neighbours(
    pts: &[Vec2],
    concept: Concept,
    band: Option<f64>,
    k: usize,
    own: bool,
) -> Result<Neighbours, String> {
    let n = pts.len();
    let tree = KdTree::new(pts);
    let mut lists: Vec<Vec<(u32, f64)>> = Vec::with_capacity(n);
    let mut pairs = 0usize;
    match concept {
        Concept::Nearest => {
            let k = k.min(n.saturating_sub(1));
            for (i, &p) in pts.iter().enumerate() {
                let mut list: Vec<(u32, f64)> = tree
                    .nearest(p, k, Some(i as u32))
                    .into_iter()
                    .map(|(_, j)| (j, 1.0))
                    .collect();
                if own {
                    list.push((i as u32, 1.0));
                }
                list.sort_unstable_by_key(|&(j, _)| j);
                lists.push(list);
            }
            Ok(Neighbours {
                lists,
                band: None,
                k,
            })
        }
        Concept::Band | Concept::Inverse => {
            let band = match band {
                Some(b) => b,
                None => nearest_distances(pts)
                    .into_iter()
                    .fold(0.0, |m, d| if d > m { d } else { m }),
            };
            let mut found = Vec::new();
            for (i, &p) in pts.iter().enumerate() {
                found.clear();
                tree.within(p, band, Some(i as u32), &mut found);
                pairs += found.len();
                if pairs > MOST_PAIRS {
                    return Err(too_many("Bant"));
                }
                if own {
                    found.push(i as u32);
                }
                found.sort_unstable();
                let list = found
                    .iter()
                    .map(|&j| {
                        let w = if concept == Concept::Inverse && j as usize != i {
                            1.0 / js_max(dist2(p, pts[j as usize]).sqrt(), 1.0)
                        } else {
                            1.0
                        };
                        (j, w)
                    })
                    .collect();
                lists.push(list);
            }
            Ok(Neighbours {
                lists,
                band: Some(band),
                k,
            })
        }
    }
}
