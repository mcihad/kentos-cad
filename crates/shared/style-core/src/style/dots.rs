//! Nokta yoğunluğu's dots (docs/adr/0213 §2.4): an area's dots are drawn
//! from its box uniformly and kept when they fall inside (even-odd, holes
//! and parts too), from a SplitMix64 seeded by the renderer's seed, the
//! area's own vertices (FNV-1a over their 64 bits) and the value's place in
//! the list: the same area gets the same dots in every build, on every
//! platform, and other dots when it changes. The inside test asks only the
//! edges of the dot's horizontal band (the answer is the full test's).

use kentos_geometry_core::Vec2;
use kentos_geometry_core::jsmath::{js_max, js_min};

use super::place::positive;

/// SplitMix64 (Steele, Lea, Flood 2014).
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// In [0, 1): the upper 53 bits over 2⁵³.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / 9_007_199_254_740_992.0
    }
}

/// FNV-1a over the rings' coordinates, eight little-endian bytes each.
pub fn rings_hash(parts: &[&[Vec<Vec2>]]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for rings in parts {
        for ring in rings.iter() {
            for p in ring {
                for v in [p.x, p.y] {
                    for b in v.to_bits().to_le_bytes() {
                        h ^= u64::from(b);
                        h = h.wrapping_mul(0x0000_0100_0000_01B3);
                    }
                }
            }
        }
    }
    h
}

/// The first state of a value's dots: the seed, the area and the value's place.
pub fn seed_of(seed: u64, area: u64, field: usize) -> u64 {
    area ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (field as u64 + 1).wrapping_mul(0xD1B5_4A32_D192_ED03)
}

/// An area's edges by horizontal band, for the inside test.
pub struct Inside {
    edges: Vec<[Vec2; 2]>,
    bands: Vec<Vec<u32>>,
    y0: f64,
    band: f64,
    pub min: Vec2,
    pub max: Vec2,
}

impl Inside {
    pub fn new(parts: &[&[Vec<Vec2>]]) -> Option<Inside> {
        let mut edges = Vec::new();
        let (mut min, mut max) = (
            Vec2::new(f64::INFINITY, f64::INFINITY),
            Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for rings in parts {
            for ring in rings.iter() {
                let n = ring.len();
                if n < 3 {
                    continue;
                }
                for i in 0..n {
                    let a = ring[i];
                    let b = ring[(i + 1) % n];
                    min = Vec2::new(js_min(min.x, a.x), js_min(min.y, a.y));
                    max = Vec2::new(js_max(max.x, a.x), js_max(max.y, a.y));
                    if a.y != b.y {
                        edges.push([a, b]);
                    }
                }
            }
        }
        if edges.is_empty() || !positive(max.x - min.x) || !positive(max.y - min.y) {
            return None;
        }
        let count = (edges.len() / 4).clamp(1, 256);
        let band = (max.y - min.y) / count as f64;
        let mut bands = vec![Vec::new(); count];
        for (k, [a, b]) in edges.iter().enumerate() {
            let lo = js_max(((js_min(a.y, b.y) - min.y) / band).floor(), 0.0) as usize;
            let hi = js_max(((js_max(a.y, b.y) - min.y) / band).floor(), 0.0) as usize;
            for list in bands
                .iter_mut()
                .take(hi.min(count - 1) + 1)
                .skip(lo.min(count - 1))
            {
                list.push(k as u32);
            }
        }
        Some(Inside {
            edges,
            bands,
            y0: min.y,
            band,
            min,
            max,
        })
    }

    /// Even-odd: an odd number of edges crosses the ray to the right of `p`.
    pub fn contains(&self, p: Vec2) -> bool {
        let k = ((p.y - self.y0) / self.band).floor();
        if k.is_nan() || k < 0.0 {
            return false;
        }
        let Some(list) = self.bands.get(k as usize) else {
            return false;
        };
        let mut odd = false;
        for &e in list {
            let [a, b] = self.edges[e as usize];
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                odd = !odd;
            }
        }
        odd
    }
}

/// `count` dots inside, in the order drawn (fewer if the tries run out:
/// `50 · count + 1000`).
pub fn dots(inside: &Inside, count: usize, state: u64, out: &mut Vec<Vec2>) {
    let mut rng = SplitMix64(state);
    let (w, h) = (inside.max.x - inside.min.x, inside.max.y - inside.min.y);
    let tries = 50usize.saturating_mul(count).saturating_add(1000);
    let mut placed = 0;
    for _ in 0..tries {
        if placed == count {
            break;
        }
        let x = inside.min.x + rng.unit() * w;
        let y = inside.min.y + rng.unit() * h;
        let p = Vec2::new(x, y);
        if inside.contains(p) {
            out.push(p);
            placed += 1;
        }
    }
}

/// A value's dots: `round(v / dotValue)`, halves up; none for a missing or negative value.
pub fn count_of(v: Option<f64>, dot_value: f64) -> usize {
    match v {
        Some(v) if v > 0.0 && dot_value > 0.0 => {
            let n = (v / dot_value + 0.5).floor();
            if n.is_finite() { n as usize } else { 0 }
        }
        _ => 0,
    }
}
