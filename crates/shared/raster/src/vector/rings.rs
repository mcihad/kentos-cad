//! A region's rings (docs/adr/0234 §4): its boundary along the cells' sides,
//! walked with the region on the left (cell space, v down: a top side runs
//! −u, a bottom side +u, a left side +v, a right side −v). At a corner the
//! walk goes straight on while the cell ahead on the left is the region's
//! and the one ahead on the right is not, turns right when the one ahead on
//! the right is the region's (two diagonal cells of one region: the walk
//! crosses through the corner), and turns left otherwise. A region so has
//! one outer ring (negative area in cell space) and its holes (positive);
//! a hole touching the outline at a corner is its own ring. Only the turns
//! are vertices.

use super::label::Regions;

/// Directions in cell space: east (+u), south (+v), west (−u), north (−v).
const DIRS: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// The sides of a cell and the direction each is walked in, as bits of the
/// walked mark: top (west), left (south), bottom (east), right (north).
fn side_of(dir: usize) -> u8 {
    // The side whose walk runs in `dir`: east the bottom, south the left, west the top, north the right.
    match dir {
        0 => 4,
        1 => 2,
        2 => 1,
        _ => 8,
    }
}

/// A ring's corners in cell space, its outline or a hole.
pub type Ring = Vec<[i64; 2]>;

/// Twice a ring's signed area in cell space.
pub fn twice_area(r: &[[i64; 2]]) -> i128 {
    let n = r.len();
    let mut s: i128 = 0;
    for k in 0..n {
        let [u0, v0] = r[k];
        let [u1, v1] = r[(k + 1) % n];
        s += i128::from(u0) * i128::from(v1) - i128::from(u1) * i128::from(v0);
    }
    s
}

/// Starts a ring at its top-most, then left-most corner.
fn canonical(mut r: Ring) -> Ring {
    let k = (0..r.len())
        .min_by_key(|&k| (r[k][1], r[k][0]))
        .unwrap_or(0);
    r.rotate_left(k);
    r
}

/// The rings of every region: for each (in their numbering) its outline
/// first, then its holes by their first corners.
pub fn region_rings(regions: &Regions) -> Result<Vec<Vec<Ring>>, String> {
    let (w, h) = (regions.width as i64, regions.height as i64);
    let labels = &regions.labels;
    let at = |i: i64, j: i64| -> u32 {
        if i < 0 || j < 0 || i >= w || j >= h {
            0
        } else {
            labels[(j * w + i) as usize]
        }
    };
    let mut walked = vec![0u8; labels.len()];
    let mut out: Vec<Vec<Ring>> = vec![Vec::new(); regions.values.len()];
    for j in 0..h {
        for i in 0..w {
            let l = at(i, j);
            if l == 0 {
                continue;
            }
            // Each side of this cell that is a boundary and not yet walked starts a ring.
            for dir in 0..4 {
                let (side, (ni, nj)) = match dir {
                    0 => (4u8, (i, j + 1)), // bottom: walked east
                    1 => (2u8, (i - 1, j)), // left: walked south
                    2 => (1u8, (i, j - 1)), // top: walked west
                    _ => (8u8, (i + 1, j)), // right: walked north
                };
                if at(ni, nj) == l || walked[(j * w + i) as usize] & side != 0 {
                    continue;
                }
                let ring = walk(&at, &mut walked, w, l, (i, j), dir)?;
                out[l as usize - 1].push(canonical(ring));
            }
        }
    }
    for rings in &mut out {
        // The outline first (the one ring of negative area), then the holes by their first corners.
        let k = rings
            .iter()
            .position(|r| twice_area(r) < 0)
            .ok_or("Bir bölgenin dış sınırı bulunamadı.")?;
        let outline = rings.swap_remove(k);
        rings.sort_by_key(|r| (r[0][1], r[0][0]));
        rings.insert(0, outline);
    }
    Ok(out)
}

/// The start of a side's walk: (vertex, direction) for a cell's side walked in `dir`.
fn start_of(i: i64, j: i64, dir: usize) -> (i64, i64) {
    match dir {
        0 => (i, j + 1),     // bottom: from its left end
        1 => (i, j),         // left: from its top end
        2 => (i + 1, j),     // top: from its right end
        _ => (i + 1, j + 1), // right: from its bottom end
    }
}

/// The cell whose side is walked from `vertex` in `dir` (the cell on the left).
fn left_cell(vertex: (i64, i64), dir: usize) -> (i64, i64) {
    let (x, y) = vertex;
    match dir {
        0 => (x, y - 1),     // east: the cell above
        1 => (x, y),         // south: the cell to the east
        2 => (x - 1, y),     // west: the cell below
        _ => (x - 1, y - 1), // north: the cell to the west
    }
}

/// The cell ahead on the right of a walk leaving `vertex` in `dir`.
fn right_cell(vertex: (i64, i64), dir: usize) -> (i64, i64) {
    let (x, y) = vertex;
    match dir {
        0 => (x, y),         // east: below
        1 => (x - 1, y),     // south: to the west
        2 => (x - 1, y - 1), // west: above
        _ => (x, y - 1),     // north: to the east
    }
}

fn walk(
    at: &dyn Fn(i64, i64) -> u32,
    walked: &mut [u8],
    w: i64,
    l: u32,
    cell: (i64, i64),
    dir0: usize,
) -> Result<Ring, String> {
    let start = start_of(cell.0, cell.1, dir0);
    let mut v = start;
    let mut dir = dir0;
    let mut ring: Ring = Vec::new();
    let limit = (walked.len() as u64 + 1) * 4;
    let mut steps = 0u64;
    loop {
        // Walk this side: mark it, move to its far end.
        let (ci, cj) = left_cell(v, dir);
        walked[(cj * w + ci) as usize] |= side_of(dir);
        v = (v.0 + DIRS[dir].0, v.1 + DIRS[dir].1);
        steps += 1;
        if steps > limit {
            return Err("Bölgenin sınırı kapanmadı.".into());
        }
        // The next side from v: right if the cell ahead on the right is the region's, straight if the one ahead on the left is, else left.
        let right = (dir + 1) % 4;
        let left = (dir + 3) % 4;
        let next = {
            let (ri, rj) = right_cell(v, dir);
            let (li, lj) = left_cell(v, dir);
            if at(ri, rj) == l {
                right
            } else if at(li, lj) == l {
                dir
            } else {
                left
            }
        };
        if next != dir {
            ring.push([v.0, v.1]);
        }
        dir = next;
        if v == start && dir == dir0 {
            break;
        }
    }
    Ok(ring)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::label::Labeler;

    fn rings_of(rows: &[&[f64]], eight: bool) -> Vec<Vec<Ring>> {
        let mut l = Labeler::new(rows[0].len() as u32, rows.len() as u32, eight).unwrap();
        for r in rows {
            l.row(r).unwrap();
        }
        region_rings(&l.finish(1000).unwrap()).unwrap()
    }

    #[test]
    fn a_cell_and_a_block() {
        let r = rings_of(&[&[1.0]], false);
        assert_eq!(r, vec![vec![vec![[0, 0], [0, 1], [1, 1], [1, 0]]]]);
        assert_eq!(twice_area(&r[0][0]), -2);
        let n = f64::NAN;
        let r = rings_of(&[&[n, n, n], &[n, 4.0, 4.0], &[n, 4.0, n]], false);
        assert_eq!(
            r,
            vec![vec![vec![[1, 1], [1, 3], [2, 3], [2, 2], [3, 2], [3, 1]]]]
        );
    }

    #[test]
    fn a_hole_and_a_corner() {
        // A ring of 1s round a 0: the 1s' outline and hole, the 0's own outline.
        let r = rings_of(
            &[&[1.0, 1.0, 1.0], &[1.0, 0.0, 1.0], &[1.0, 1.0, 1.0]],
            false,
        );
        assert_eq!(r[0][0], vec![[0, 0], [0, 3], [3, 3], [3, 0]]);
        assert_eq!(r[0][1], vec![[1, 1], [2, 1], [2, 2], [1, 2]]);
        assert!(twice_area(&r[0][1]) > 0);
        assert_eq!(r[1], vec![vec![[1, 1], [1, 2], [2, 2], [2, 1]]]);
        // A C whose ends meet at a corner: by sides one region with a hole touching its outline there.
        let n = f64::NAN;
        let rows: [&[f64]; 3] = [&[1.0, 1.0, 1.0], &[1.0, n, n], &[1.0, 1.0, n]];
        let r = rings_of(&rows, false);
        assert_eq!(
            r[0].len(),
            1,
            "the gap at (2, 1)–(2, 2) is outside: no hole"
        );
        let rows: [&[f64]; 4] = [
            &[1.0, 1.0, 1.0, n],
            &[1.0, n, 1.0, n],
            &[1.0, 1.0, n, 1.0],
            &[n, n, 1.0, 1.0],
        ];
        let four = rings_of(&rows, false);
        // By sides the lower-right block is apart; the C's pocket (1, 1) is closed by sides, a hole
        // touching the outline at the corner (2, 2), where the walks cross.
        assert_eq!(four.len(), 2);
        assert_eq!(four[0].len(), 2);
        assert_eq!(four[0][1], vec![[1, 1], [2, 1], [2, 2], [1, 2]]);
        assert!(four[0][0].contains(&[2, 2]));
        // By corners one region with two holes, (1, 1) and (2, 2), touching each other at (2, 2).
        let eight = rings_of(&rows, true);
        assert_eq!(eight.len(), 1);
        assert_eq!(eight[0].len(), 3);
        assert!(twice_area(&eight[0][0]) < 0);
        assert!(eight[0][1..].iter().all(|r| twice_area(r) == 2));
    }
}
