//! Km labels (docs/adr/0901 §5, §6): the alignment's own station is its horizontal length from the start; the km the
//! user reads starts at a given km and passes through station equations. An equation is written as the km where it
//! is (`back`, read in the region before it), the km after it (`ahead`) and whether the km grows or falls after it.
//! Regions are numbered from 0 (before the first equation). A km may fall in more than one region; a question that
//! wants one place names its region.

/// A station equation as given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Equation {
    pub back: f64,
    pub ahead: f64,
    pub increasing: bool,
}

/// The km of an alignment: the km at its start and its equations, in order along it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stationing {
    pub start: f64,
    pub equations: Vec<Equation>,
}

/// A region: from a station on, km = ahead + dir·(station − at).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub at: f64,
    pub ahead: f64,
    pub dir: f64,
    /// Where it ends (the next equation's station, or the alignment's length).
    pub until: f64,
}

/// Why km labels do not fit an alignment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StationingError {
    /// An equation's back km falls outside its region (index of the equation).
    Outside { equation: usize },
    NotFinite { equation: Option<usize> },
}

impl Stationing {
    /// The regions on an alignment `length` long.
    pub fn regions(&self, length: f64) -> Result<Vec<Region>, StationingError> {
        if !self.start.is_finite() {
            return Err(StationingError::NotFinite { equation: None });
        }
        let mut out = vec![Region {
            at: 0.0,
            ahead: self.start,
            dir: 1.0,
            until: length,
        }];
        for (i, q) in self.equations.iter().enumerate() {
            if !q.back.is_finite() || !q.ahead.is_finite() {
                return Err(StationingError::NotFinite { equation: Some(i) });
            }
            let last = out.last().copied().unwrap_or(out[0]);
            let at = last.at + last.dir * (q.back - last.ahead);
            // Strictly after the region's start and strictly before the end.
            if !(at > last.at && at < length) {
                return Err(StationingError::Outside { equation: i });
            }
            if let Some(l) = out.last_mut() {
                l.until = at;
            }
            out.push(Region {
                at,
                ahead: q.ahead,
                dir: if q.increasing { 1.0 } else { -1.0 },
                until: length,
            });
        }
        Ok(out)
    }

    /// The km at a station (the later region at an equation).
    pub fn km(regions: &[Region], station: f64) -> f64 {
        let i = regions.partition_point(|r| r.at <= station).saturating_sub(1);
        let r = regions[i.min(regions.len() - 1)];
        r.ahead + r.dir * (station - r.at)
    }

    /// The region index a station falls in.
    pub fn region_of(regions: &[Region], station: f64) -> usize {
        regions.partition_point(|r| r.at <= station).saturating_sub(1).min(regions.len().saturating_sub(1))
    }

    /// Every station a km falls on, with its region.
    pub fn stations(regions: &[Region], km: f64) -> Vec<(usize, f64)> {
        regions
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                let s = r.at + r.dir * (km - r.ahead);
                let last = i + 1 == regions.len();
                let inside = s >= r.at - 1e-9 && (s < r.until - 1e-9 || (last && s <= r.until + 1e-9));
                inside.then_some((i, s.clamp(r.at, r.until)))
            })
            .collect()
    }
}

/// A km written the Turkish way: `2+345.678` (`-0+012.000` below zero), with `digits` decimals.
pub fn km_text(km: f64, digits: usize) -> String {
    let scale = 10f64.powi(digits as i32);
    let rounded = (km.abs() * scale).round() / scale;
    let whole = (rounded / 1000.0).floor();
    let rest = rounded - whole * 1000.0;
    let sign = if km < 0.0 && rounded > 0.0 { "-" } else { "" };
    let width = if digits > 0 { 4 + digits } else { 3 };
    format!("{sign}{}+{:0width$.digits$}", whole as i64, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_equation_jumps_and_a_falling_region_maps_twice() {
        let st = Stationing {
            start: 1000.0,
            equations: vec![
                Equation { back: 1200.0, ahead: 1500.0, increasing: true },
                Equation { back: 1600.0, ahead: 1650.0, increasing: false },
            ],
        };
        let r = st.regions(500.0).expect("fits");
        assert_eq!(r.iter().map(|r| r.at).collect::<Vec<_>>(), vec![0.0, 200.0, 300.0]);
        assert_eq!(Stationing::km(&r, 250.0), 1550.0);
        assert_eq!(Stationing::km(&r, 400.0), 1550.0);
        assert_eq!(Stationing::stations(&r, 1550.0), vec![(1, 250.0), (2, 400.0)]);
        assert_eq!(km_text(2345.6784, 3), "2+345.678");
        assert_eq!(km_text(-12.0, 2), "-0+012.00");
        assert_eq!(km_text(999.9996, 3), "1+000.000");
    }
}
