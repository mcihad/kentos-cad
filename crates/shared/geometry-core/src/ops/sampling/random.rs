//! The seeded numbers of Rastgele noktalar and Düzenli noktalar' shift
//! (docs/adr/0220 §3, §4), the same on every target: SplitMix64 spreads the
//! seed over four 64-bit words, xoshiro256** gives the numbers, a number in
//! [0, 1) is the top 53 bits over 2⁵³. Integer arithmetic and one division
//! only, so the independent reference repeats it bit for bit.

/// A run's numbers.
pub struct Sequence {
    s: [u64; 4],
}

fn splitmix(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Sequence {
    pub fn new(seed: u64) -> Sequence {
        let mut x = seed;
        let s = [
            splitmix(&mut x),
            splitmix(&mut x),
            splitmix(&mut x),
            splitmix(&mut x),
        ];
        Sequence { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// A number in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / 9_007_199_254_740_992.0
    }
}

/// The numbers of lattice point (i, j) of Düzenli noktalar (§3): its own
/// sequence, so a point shifts the same way in every area and whatever the
/// order the areas are read in.
pub fn at_lattice(seed: u64, i: i64, j: i64) -> Sequence {
    Sequence::new(
        seed ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
    )
}

/// A typed seed as a sequence's: its whole part, none below zero.
pub fn seed_of(v: f64) -> u64 {
    if v.is_finite() && v > 0.0 {
        crate::jsmath::js_min(v, 9_007_199_254_740_991.0) as u64
    } else {
        0
    }
}
