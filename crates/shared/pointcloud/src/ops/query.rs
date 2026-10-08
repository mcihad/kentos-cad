//! XYZ sor (docs/adr/0207 §7): the point nearest a place in plan, among a
//! node's records at full resolution, and what its record holds. The host
//! reads every node near the place (`geom::pointcloud::near`) and keeps the
//! nearest over them all: a smaller distance wins, an equal one keeps the
//! point met first (nodes coarser first, records in their order).

use crate::record::Layout;

/// What XYZ sor says of a point.
#[derive(Clone, Debug, PartialEq)]
pub struct Facts {
    pub xyz: [f64; 3],
    pub class: u8,
    pub intensity: u16,
    /// Return number and number of returns.
    pub returns: (u8, u8),
    pub rgb: Option<[u16; 3]>,
    pub gps_time: Option<f64>,
    pub source_id: u16,
}

impl Facts {
    /// The facts of record `r` (of `layout`, coordinates by `scale` and `offset`).
    pub fn of(layout: &Layout, r: &[u8], scale: [f64; 3], offset: [f64; 3]) -> Facts {
        Facts {
            xyz: [
                f64::from(layout.x(r)) * scale[0] + offset[0],
                f64::from(layout.y(r)) * scale[1] + offset[1],
                f64::from(layout.z(r)) * scale[2] + offset[2],
            ],
            class: layout.class(r),
            intensity: layout.intensity(r),
            returns: layout.returns(r),
            rgb: layout.rgb(r),
            gps_time: layout.gps.map(|_| layout.gps_time(r)),
            source_id: layout.source_id(r),
        }
    }
}

/// The record of `records` nearest `at` in plan within `reach`: its index
/// and squared distance; an equal distance keeps the earlier record.
pub fn nearest(
    layout: &Layout,
    records: &[u8],
    scale: [f64; 3],
    offset: [f64; 3],
    at: [f64; 2],
    reach: f64,
) -> Option<(usize, f64)> {
    if layout.len == 0 || !(reach >= 0.0) {
        return None;
    }
    let r2 = reach * reach;
    let mut best: Option<(usize, f64)> = None;
    for (i, r) in records.chunks_exact(layout.len).enumerate() {
        let dx = f64::from(layout.x(r)) * scale[0] + offset[0] - at[0];
        let dy = f64::from(layout.y(r)) * scale[1] + offset[1] - at[1];
        let d = dx * dx + dy * dy;
        if d <= r2 && best.is_none_or(|(_, b)| d < b) {
            best = Some((i, d));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_record_and_its_facts() {
        let layout = Layout::new(6, 30);
        let mut records = vec![0u8; 3 * 30];
        for (i, (x, y, z)) in [(100, 100, 5), (103, 100, 6), (101, 101, 7)]
            .iter()
            .enumerate()
        {
            let r = &mut records[i * 30..(i + 1) * 30];
            layout.set_xyz(r, *x, *y, *z);
            layout.set_class(r, 2);
        }
        let scale = [0.01, 0.01, 0.01];
        let offset = [1000.0, 2000.0, 0.0];
        let (i, d) = nearest(&layout, &records, scale, offset, [1001.0, 2001.0], 0.05).unwrap();
        assert_eq!(i, 0);
        assert!(d < 1e-20);
        // Out of reach.
        assert!(nearest(&layout, &records, scale, offset, [1010.0, 2010.0], 0.05).is_none());
        let f = Facts::of(&layout, &records[30..60], scale, offset);
        assert!((f.xyz[0] - 1001.03).abs() < 1e-9);
        assert_eq!(f.class, 2);
        assert_eq!(f.gps_time, Some(0.0));
    }
}
