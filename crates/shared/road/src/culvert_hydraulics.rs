//! A culvert's headwater and the road's overtopping (docs/adr/0947, plan/road.md Y41; ROAD-DRAIN-08): the water level
//! a flow raises in front of a culvert, by inlet and by outlet control as FHWA's HDS-5 writes them; where it passes the
//! road's crest, the flow the road itself carries as a weir; and on the crest the water's depth and speed, classed by
//! the flood hazard curves (AIDR guideline 7-3, after Smith and Wasko 2012). SI units; the coefficients are the user's.
//!
//! - **Inlet control** (HDS-5 appendix A): with X = Ku·Q/(A·√D), Ku = 1.811, D the barrel's rise and A its area,
//!   unsubmerged HW/D = Hc/D + K·X^M + Ks·S (form 1) or K·X^M (form 2), submerged HW/D = c·X² + Y + Ks·S; unsubmerged
//!   to X = 3.5, submerged from 4.0, linear between. Hc is the specific head at critical depth.
//! - **Outlet control** (HDS-5's full-barrel approximation): HW = TW' + (1 + ke + 2g·n²·L/R^(4/3))·V²/(2g) − S·L,
//!   TW' = max(TW, (dc + D)/2) at the outlet.
//! - The headwater is the larger of the two, above the inlet's invert.
//! - **Overtopping:** a weir over the road's crest, Q = Cd·L·h^(3/2). With both, the headwater is the one at which the
//!   culvert and the weir together carry the flow (each rises with the headwater: found by bisection).
//! - **On the crest** the flow passes critical depth: y = (q²/g)^(1/3), v = √(g·y), and so depth × speed = q, the
//!   flow per metre of road. Hazard H1 (generally safe) to H6 by the limits of depth × speed, depth and speed.

/// The SI form of HDS-5's inlet equations: X = Ku·Q/(A·√D) is the US value the coefficients were fitted to.
pub const KU: f64 = 1.811;

/// Gravity (m/s²).
pub const G: f64 = 9.81;

/// A barrel's cross section (one cell).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Section {
    Circle { diameter: f64 },
    Box { width: f64, height: f64 },
}

impl Section {
    /// Its rise D.
    pub fn rise(&self) -> f64 {
        match *self {
            Section::Circle { diameter } => diameter,
            Section::Box { height, .. } => height,
        }
    }

    /// Its full area and wetted perimeter.
    pub fn full(&self) -> (f64, f64) {
        match *self {
            Section::Circle { diameter } => (std::f64::consts::PI * diameter * diameter / 4.0, std::f64::consts::PI * diameter),
            Section::Box { width, height } => (width * height, 2.0 * (width + height)),
        }
    }

    /// The area and top width at depth `y`.
    fn wet(&self, y: f64) -> (f64, f64) {
        match *self {
            Section::Circle { diameter } => {
                let theta = 2.0 * (1.0 - 2.0 * y / diameter).clamp(-1.0, 1.0).acos();
                (diameter * diameter / 8.0 * (theta - theta.sin()), diameter * (theta / 2.0).sin())
            }
            Section::Box { width, .. } => (width * y, width),
        }
    }

    /// The critical depth of a flow `q` (m³/s): Q²/g = A³/T; at most the rise (a full box), the circle's by bisection
    /// (A³/T grows without bound to the crown).
    pub fn critical(&self, q: f64) -> f64 {
        if q <= 0.0 {
            return 0.0;
        }
        match *self {
            Section::Box { width, height } => ((q / width).powi(2) / G).cbrt().min(height),
            Section::Circle { diameter } => {
                let target = q * q / G;
                let (mut lo, mut hi) = (0.0, diameter);
                for _ in 0..200 {
                    let y = 0.5 * (lo + hi);
                    let (a, t) = self.wet(y);
                    if t <= 0.0 || a * a * a / t > target {
                        hi = y;
                    } else {
                        lo = y;
                    }
                }
                0.5 * (lo + hi)
            }
        }
    }

    /// The specific head at critical depth: y + A/(2T).
    pub fn critical_head(&self, q: f64) -> f64 {
        let y = self.critical(q);
        let (a, t) = self.wet(y);
        if t > 0.0 && y < self.rise() { y + a / (2.0 * t) } else { y + (q / self.full().0).powi(2) / (2.0 * G) }
    }
}

/// An inlet's HDS-5 coefficients: its equation form (1 or 2), K, M, c, Y and the slope correction Ks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inlet {
    pub form: u8,
    pub k: f64,
    pub m: f64,
    pub c: f64,
    pub y: f64,
    pub ks: f64,
}

/// HDS-5 table A.1's rows that KentOS names (docs/adr/0947): concrete pipes with a square edge in a headwall, a groove
/// end in a headwall or projecting; concrete boxes with 30°–75°, 90° and 15°, or 0° wingwall flares, or a 90° headwall
/// with chamfers.
pub const INLETS: [(&str, Inlet); 7] = [
    ("pipe-square-headwall", Inlet { form: 1, k: 0.0098, m: 2.0, c: 0.0398, y: 0.67, ks: -0.5 }),
    ("pipe-groove-headwall", Inlet { form: 1, k: 0.0018, m: 2.0, c: 0.0292, y: 0.74, ks: -0.5 }),
    ("pipe-groove-projecting", Inlet { form: 1, k: 0.0045, m: 2.0, c: 0.0317, y: 0.69, ks: -0.5 }),
    ("box-wingwall-30-75", Inlet { form: 1, k: 0.026, m: 1.0, c: 0.0347, y: 0.81, ks: -0.5 }),
    ("box-wingwall-90-15", Inlet { form: 1, k: 0.061, m: 0.75, c: 0.0400, y: 0.80, ks: -0.5 }),
    ("box-wingwall-0", Inlet { form: 1, k: 0.061, m: 0.75, c: 0.0423, y: 0.82, ks: -0.5 }),
    ("box-headwall-chamfer", Inlet { form: 2, k: 0.515, m: 0.667, c: 0.0375, y: 0.79, ks: -0.5 }),
];

/// The inlet of `name` among [`INLETS`].
pub fn inlet(name: &str) -> Option<Inlet> {
    INLETS.iter().find(|(n, _)| *n == name).map(|(_, i)| *i)
}

/// A hazard class's words (AIDR guideline 7-3): who and what the flow is unsafe for.
pub fn hazard_words(class: u8) -> &'static str {
    match class {
        1 => "H1: araç, insan ve yapı için genelde güvenli",
        2 => "H2: küçük araçlar için güvensiz",
        3 => "H3: araçlar, çocuklar ve yaşlılar için güvensiz",
        4 => "H4: araçlar ve insanlar için güvensiz",
        5 => "H5: araçlar ve insanlar için güvensiz, yapılar hasar görebilir",
        _ => "H6: araçlar ve insanlar için güvensiz, yapılar yıkılabilir",
    }
}

/// A culvert: its section, cells, length and slope (m/m, falling positive), Manning's n, inlet and entrance loss.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Culvert {
    pub section: Section,
    pub cells: u32,
    pub length: f64,
    pub slope: f64,
    pub n: f64,
    pub inlet: Inlet,
    pub ke: f64,
}

/// Which control sets the headwater.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Inlet,
    Outlet,
}

impl Culvert {
    fn per_cell(&self, q: f64) -> f64 {
        q / f64::from(self.cells.max(1))
    }

    /// The headwater (m above the inlet's invert) by inlet control; none without a flow (the equations' Ks·S would
    /// leave the still water below the invert).
    pub fn inlet_headwater(&self, q: f64) -> f64 {
        if q <= 0.0 {
            return 0.0;
        }
        let qc = self.per_cell(q);
        let d = self.section.rise();
        let (a, _) = self.section.full();
        let x = KU * qc / (a * d.sqrt());
        let i = self.inlet;
        let unsubmerged = |x: f64| match i.form {
            1 => self.section.critical_head(qc) / d + i.k * x.powf(i.m) + i.ks * self.slope,
            _ => i.k * x.powf(i.m),
        };
        let submerged = |x: f64| i.c * x * x + i.y + i.ks * self.slope;
        let ratio = if x <= 3.5 {
            unsubmerged(x)
        } else if x >= 4.0 {
            submerged(x)
        } else {
            let (u, s) = (unsubmerged(3.5), submerged(4.0));
            u + (s - u) * (x - 3.5) / 0.5
        };
        d * ratio
    }

    /// The headwater (m above the inlet's invert) by outlet control, the tailwater `tw` above the outlet's invert; without a
    /// flow the tailwater's level (HDS-5's (dc + D)/2 is for a flowing barrel, not a still one).
    pub fn outlet_headwater(&self, q: f64, tw: f64) -> f64 {
        if q <= 0.0 {
            return (tw - self.slope * self.length).max(0.0);
        }
        let qc = self.per_cell(q);
        let d = self.section.rise();
        let (a, p) = self.section.full();
        let r = a / p;
        let v = qc / a;
        let dc = self.section.critical(qc);
        let tw_eff = tw.max((dc + d) / 2.0);
        let losses = (1.0 + self.ke + 2.0 * G * self.n * self.n * self.length / r.powf(4.0 / 3.0)) * v * v / (2.0 * G);
        tw_eff + losses - self.slope * self.length
    }

    /// The headwater (m above the inlet's invert) and the control that sets it. With no flow, the tailwater's level
    /// (HDS-5's (dc + D)/2 is for a flowing barrel, not a still one).
    pub fn headwater(&self, q: f64, tw: f64) -> (f64, Control) {
        if q <= 0.0 {
            return ((tw - self.slope * self.length).max(0.0), Control::Outlet);
        }
        let (hi, ho) = (self.inlet_headwater(q), self.outlet_headwater(q, tw));
        if hi >= ho { (hi, Control::Inlet) } else { (ho, Control::Outlet) }
    }

    /// The flow the culvert passes at headwater `hw` (m above the inlet's invert): the headwater rises with the flow,
    /// so by bisection.
    pub fn flow_at(&self, hw: f64, tw: f64) -> f64 {
        if hw <= 0.0 {
            return 0.0;
        }
        let mut hi = 1.0;
        while self.headwater(hi, tw).0 < hw && hi < 1e7 {
            hi *= 2.0;
        }
        let mut lo = 0.0;
        for _ in 0..200 {
            let m = 0.5 * (lo + hi);
            if self.headwater(m, tw).0 < hw {
                lo = m;
            } else {
                hi = m;
            }
        }
        0.5 * (lo + hi)
    }
}

/// The road's crest as a weir: its height above the inlet's invert, its length along the road and Cd (SI, Q = Cd·L·h^1.5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weir {
    pub crest: f64,
    pub length: f64,
    pub cd: f64,
}

/// A flow hazard class (1 to 6) by the depth (m) and speed (m/s): AIDR guideline 7-3's limits.
pub fn hazard(depth: f64, speed: f64) -> u8 {
    let dv = depth * speed;
    const LIMITS: [(f64, f64, f64); 5] = [(0.3, 0.3, 2.0), (0.6, 0.5, 2.0), (0.6, 1.2, 2.0), (1.0, 2.0, 2.0), (4.0, 4.0, 4.0)];
    for (k, (p, d, v)) in LIMITS.iter().enumerate() {
        if dv <= *p && depth <= *d && speed <= *v {
            return k as u8 + 1;
        }
    }
    6
}

/// A flow through a culvert and over the road.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Performance {
    /// The headwater (m above the inlet's invert) and the control at the culvert's flow; the inlet's and the outlet's
    /// headwater at that flow.
    pub headwater: f64,
    pub control: Control,
    pub inlet: f64,
    pub outlet: f64,
    /// What the culvert and the road carry (m³/s).
    pub culvert: f64,
    pub overtopping: f64,
    /// Over the crest: the head (m), the depth and speed at critical flow, the flow per metre, the hazard class (0
    /// where the road is not overtopped).
    pub head: f64,
    pub depth: f64,
    pub speed: f64,
    pub unit: f64,
    pub hazard: u8,
}

/// A flow `q` (m³/s) through `culvert` against the tailwater `tw`, the road `weir` taking what the culvert cannot
/// below the crest.
pub fn perform(culvert: &Culvert, weir: &Weir, q: f64, tw: f64) -> Performance {
    let (hw, control) = culvert.headwater(q, tw);
    if hw <= weir.crest || weir.length <= 0.0 || weir.cd <= 0.0 {
        return Performance {
            headwater: hw,
            control,
            inlet: culvert.inlet_headwater(q),
            outlet: culvert.outlet_headwater(q, tw),
            culvert: q,
            overtopping: 0.0,
            head: 0.0,
            depth: 0.0,
            speed: 0.0,
            unit: 0.0,
            hazard: 0,
        };
    }
    // The head over the crest at which the culvert and the road together carry q.
    let total = |h: f64| culvert.flow_at(weir.crest + h, tw) + weir.cd * weir.length * h.powf(1.5);
    let (mut lo, mut hi) = (0.0, hw - weir.crest);
    for _ in 0..200 {
        let m = 0.5 * (lo + hi);
        if total(m) < q {
            lo = m;
        } else {
            hi = m;
        }
    }
    let h = 0.5 * (lo + hi);
    let through = culvert.flow_at(weir.crest + h, tw);
    let over = q - through;
    let unit = over / weir.length;
    let depth = (unit * unit / G).cbrt();
    let speed = (G * depth).sqrt();
    Performance {
        headwater: weir.crest + h,
        control: culvert.headwater(through, tw).1,
        inlet: culvert.inlet_headwater(through),
        outlet: culvert.outlet_headwater(through, tw),
        culvert: through,
        overtopping: over,
        head: h,
        depth,
        speed,
        unit,
        hazard: hazard(depth, speed),
    }
}
