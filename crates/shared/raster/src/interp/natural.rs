//! Doğal komşu (docs/adr/0232 §6): Sibson's coordinates of a place, by
//! Watson's method on the Delaunay triangulation. The triangles whose
//! circle holds the place (the cavity) are found from its triangle; the
//! cavity's boundary points are its natural neighbours. Each one's share is
//! the area of the polygon its new Voronoi cell takes from its old one: the
//! circumcentre of the place with the boundary edge coming into the point,
//! the circumcentres of the cavity's triangles round the point in turn, and
//! the circumcentre of the place with the boundary edge going out. Every
//! centre is worked out relative to the place.

use kentos_geometry_core::geom::delaunay::{NONE, next};
use kentos_geometry_core::predicates::incircle;
use kentos_geometry_core::vec2::Vec2;

/// A triangulation as Watson's method reads it: its triangles and twins.
pub struct Mesh<'a> {
    pub triangles: &'a [u32],
    pub halfedges: &'a [u32],
}

/// The circumcentre of a, b, c relative to the origin they are given in.
fn center(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (ex, ey) = (c.x - a.x, c.y - a.y);
    let bl = dx * dx + dy * dy;
    let cl = ex * ex + ey * ey;
    let d = 0.5 / (dx * ey - dy * ex);
    Vec2::new(a.x + (ey * bl - dy * cl) * d, a.y + (dx * cl - ex * bl) * d)
}

/// The circumcentre of the origin, b and c.
fn center0(b: Vec2, c: Vec2) -> Vec2 {
    let d = 2.0 * (b.x * c.y - b.y * c.x);
    let (bl, cl) = (b.x * b.x + b.y * b.y, c.x * c.x + c.y * c.y);
    Vec2::new((bl * c.y - cl * b.y) / d, (cl * b.x - bl * c.x) / d)
}

/// The cavity of `q` from triangle `start` (which holds it): the triangles
/// whose circle holds `q` strictly, found across their edges. `cavity` and
/// `out` are the caller's buffers.
pub fn cavity(
    mesh: &Mesh<'_>,
    xy: &[Vec2],
    q: Vec2,
    start: u32,
    cavity: &mut Vec<u32>,
    out: &mut Vec<u32>,
) {
    cavity.clear();
    out.clear();
    cavity.push(start);
    let mut i = 0;
    while i < cavity.len() {
        let t = cavity[i];
        i += 1;
        for k in 0..3 {
            let tw = mesh.halfedges[(3 * t + k) as usize];
            if tw == NONE {
                continue;
            }
            let u = tw / 3;
            if cavity.contains(&u) || out.contains(&u) {
                continue;
            }
            let c = |j: u32| xy[mesh.triangles[(3 * u + j) as usize] as usize];
            if incircle(c(0), c(1), c(2), q) > 0.0 {
                cavity.push(u);
            } else {
                out.push(u);
            }
        }
    }
}

/// The natural neighbours of `q` in `cavity` (all of it holding `q` in its
/// circles) and their shares (not yet divided by their sum), in `shares`:
/// none when `q` is not inside the cavity's boundary.
pub fn shares(
    mesh: &Mesh<'_>,
    xy: &[Vec2],
    q: Vec2,
    cavity: &[u32],
    shares: &mut Vec<(u32, f64)>,
) -> bool {
    shares.clear();
    let inside = |t: u32| cavity.contains(&t);
    let boundary = |h: u32| {
        let tw = mesh.halfedges[h as usize];
        tw == NONE || !inside(tw / 3)
    };
    let rel = |i: u32| {
        let p = xy[i as usize];
        Vec2::new(p.x - q.x, p.y - q.y)
    };
    // The boundary half-edges (the cavity on their left), by the point they start at.
    let mut edges: Vec<u32> = Vec::new();
    for &t in cavity {
        for k in 0..3 {
            let h = 3 * t + k;
            if boundary(h) {
                edges.push(h);
            }
        }
    }
    let starting = |v: u32| {
        edges
            .iter()
            .copied()
            .find(|&h| mesh.triangles[h as usize] == v)
    };
    let centre_of = |t: u32| {
        let c = |j: u32| rel(mesh.triangles[(3 * t + j) as usize]);
        center(c(0), c(1), c(2))
    };
    let mut ring: Vec<Vec2> = Vec::new();
    for &h_in in &edges {
        // The point the boundary edge comes into, and its edge going out.
        let v = mesh.triangles[next(h_in) as usize];
        let Some(h_out) = starting(v) else {
            return false;
        };
        let g_in = center0(rel(mesh.triangles[h_in as usize]), rel(v));
        let g_out = center0(rel(v), rel(mesh.triangles[next(h_out) as usize]));
        ring.clear();
        ring.push(g_in);
        // Round v from the edge coming in to the edge going out, triangle by triangle.
        let mut h = next(h_in);
        for _ in 0..cavity.len() + 1 {
            ring.push(centre_of(h / 3));
            if boundary(h) {
                break;
            }
            h = next(mesh.halfedges[h as usize]);
        }
        if h != h_out {
            return false;
        }
        ring.push(g_out);
        let mut twice = 0.0;
        for k in 0..ring.len() {
            let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
            twice += a.x * b.y - a.y * b.x;
        }
        if !twice.is_finite() {
            return false;
        }
        shares.push((v, twice.abs()));
    }
    true
}
