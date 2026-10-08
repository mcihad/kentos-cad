//! A point cloud's plan and its octree's nodes in view (docs/adr/0207 §3, §6).
//!
//! The drawing knows a cloud by the rectangle of its bounds: picked by its
//! edge, snapped at its corners and edges' middles, never moved. Its points
//! are the points pass's, which each frame draws the octree's nodes the view
//! meets, refined while a node's point spacing on the screen is wider than a
//! point ([`visible`]); both platforms' passes and loaders use it.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::entity::Shape;
use crate::jsmath::js_max;
use crate::vec2::Vec2;

/// A cloud's bounds (`[x₁, y₁, z₁, x₂, y₂, z₂]`); none for any other shape or bounds not finite.
pub fn bounds(shape: &Shape) -> Option<[f64; 6]> {
    match shape {
        Shape::PointCloud { bounds, .. } if bounds.iter().all(|v| v.is_finite()) => Some(*bounds),
        _ => None,
    }
}

/// The name the points pass and the hosts know a cloud's files by: `cloud:`
/// and the FNV-1a 64 of their JSON text (equal files share their nodes).
pub fn cloud_key(sources: &crate::api::json::Json) -> String {
    let text = crate::api::json::to_string(sources);
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("cloud:{h:016x}")
}

/// The plan's corners, counter-clockwise from the lower left.
pub fn corners(shape: &Shape) -> Option<[Vec2; 4]> {
    let [x1, y1, _, x2, y2, _] = bounds(shape)?;
    Some([
        Vec2::new(x1, y1),
        Vec2::new(x2, y1),
        Vec2::new(x2, y2),
        Vec2::new(x1, y2),
    ])
}

/// An octree node's key: depth and place at that depth (EPT's, COPC's).
pub type NodeKey = [i32; 4];

/// The octree of a cloud (or of one file of a virtual cloud): its root cube and nodes.
#[derive(Clone, Debug, Default)]
pub struct Octree {
    /// The root cube's least corner and side.
    pub low: [f64; 3],
    pub side: f64,
    /// The root's point spacing (halved each level).
    pub spacing: f64,
    pub keys: Vec<NodeKey>,
    pub counts: Vec<u64>,
    index: HashMap<NodeKey, u32>,
}

impl Octree {
    /// An octree of a root cube centred at `center`, half its side `half`, and its nodes.
    pub fn new(
        center: [f64; 3],
        half: f64,
        spacing: f64,
        nodes: impl IntoIterator<Item = (NodeKey, u64)>,
    ) -> Octree {
        let mut t = Octree {
            low: [center[0] - half, center[1] - half, center[2] - half],
            side: 2.0 * half,
            spacing,
            ..Octree::default()
        };
        for (k, c) in nodes {
            t.index.insert(k, t.keys.len() as u32);
            t.keys.push(k);
            t.counts.push(c);
        }
        t
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// A node's index by its key.
    pub fn find(&self, key: NodeKey) -> Option<u32> {
        self.index.get(&key).copied()
    }

    /// A node's side.
    pub fn node_side(&self, key: NodeKey) -> f64 {
        self.side / (1u64 << key[0]) as f64
    }

    /// A node's least corner.
    pub fn node_low(&self, key: NodeKey) -> [f64; 3] {
        let s = self.node_side(key);
        [
            self.low[0] + f64::from(key[1]) * s,
            self.low[1] + f64::from(key[2]) * s,
            self.low[2] + f64::from(key[3]) * s,
        ]
    }

    /// A node's centre.
    pub fn node_center(&self, key: NodeKey) -> [f64; 3] {
        let s = self.node_side(key);
        let l = self.node_low(key);
        [l[0] + s / 2.0, l[1] + s / 2.0, l[2] + s / 2.0]
    }
}

/// Whether two plan boxes `[x₁, y₁, x₂, y₂]` meet.
#[inline]
fn meets(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

/// The nodes to draw for a view `[x₁, y₁, x₂, y₂]` at `px_per_m` device
/// pixels a metre, points `point_px` device pixels wide: from the root, the
/// nodes whose cube meets the view, coarser first and nearer the view's
/// middle first among equals, going down while a node's spacing on the
/// screen is wider than a point, until `budget` points (the root always).
/// Indices into the tree, in the order to load and draw them.
pub fn visible(
    tree: &Octree,
    view: [f64; 4],
    px_per_m: f64,
    point_px: f64,
    budget: u64,
    out: &mut Vec<u32>,
) {
    out.clear();
    let Some(root) = tree.find([0, 0, 0, 0]) else {
        return;
    };
    let mid = [(view[0] + view[2]) / 2.0, (view[1] + view[3]) / 2.0];
    let plan = |key: NodeKey| -> [f64; 4] {
        let l = tree.node_low(key);
        let s = tree.node_side(key);
        [l[0], l[1], l[0] + s, l[1] + s]
    };
    let far = |key: NodeKey| -> u64 {
        let b = plan(key);
        let dx = (b[0] + b[2]) / 2.0 - mid[0];
        let dy = (b[1] + b[3]) / 2.0 - mid[1];
        (dx * dx + dy * dy).to_bits()
    };
    if !meets(plan(tree.keys[root as usize]), view) {
        return;
    }
    let mut heap: BinaryHeap<Reverse<(i32, u64, NodeKey, u32)>> = BinaryHeap::new();
    let k0 = tree.keys[root as usize];
    heap.push(Reverse((k0[0], far(k0), k0, root)));
    let mut used = 0u64;
    while let Some(Reverse((d, _, key, i))) = heap.pop() {
        let count = tree.counts[i as usize];
        if !out.is_empty() && used + count > budget {
            continue;
        }
        used += count;
        out.push(i);
        let spacing_px = tree.spacing / (1u64 << d) as f64 * px_per_m;
        if !(spacing_px > point_px) {
            continue;
        }
        for o in 0..8 {
            let c = [
                d + 1,
                key[1] * 2 + (o & 1),
                key[2] * 2 + ((o >> 1) & 1),
                key[3] * 2 + ((o >> 2) & 1),
            ];
            if let Some(j) = tree.find(c)
                && meets(plan(c), view)
            {
                heap.push(Reverse((c[0], far(c), c, j)));
            }
        }
    }
}

/// The nodes whose plan comes within `reach` of `at`: every node a point
/// within `reach` of `at` (in plan) may be in, since COPC's nodes together
/// hold every point once (XYZ sor, docs/adr/0207 §7). Indices into the tree,
/// coarser first, then in the order of their keys.
pub fn near(tree: &Octree, at: [f64; 2], reach: f64, out: &mut Vec<u32>) {
    out.clear();
    let Some(root) = tree.find([0, 0, 0, 0]) else {
        return;
    };
    let touches = |key: NodeKey| -> bool {
        let l = tree.node_low(key);
        let s = tree.node_side(key);
        // The plan's distance from `at` to the node's square, along each axis (0 inside).
        let dx = js_max(js_max(l[0] - at[0], at[0] - (l[0] + s)), 0.0);
        let dy = js_max(js_max(l[1] - at[1], at[1] - (l[1] + s)), 0.0);
        dx * dx + dy * dy <= reach * reach
    };
    if !touches(tree.keys[root as usize]) {
        return;
    }
    let mut level = vec![(tree.keys[root as usize], root)];
    while !level.is_empty() {
        level.sort_unstable_by_key(|(k, _)| *k);
        let mut next = Vec::new();
        for &(key, i) in &level {
            out.push(i);
            for o in 0..8 {
                let c = [
                    key[0] + 1,
                    key[1] * 2 + (o & 1),
                    key[2] * 2 + ((o >> 1) & 1),
                    key[3] * 2 + ((o >> 2) & 1),
                ];
                if let Some(j) = tree.find(c)
                    && touches(c)
                {
                    next.push((c, j));
                }
            }
        }
        level = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nodes_near_a_place() {
        let t = tree();
        let mut out = Vec::new();
        // Near the lower left corner: the root, the four lower-left children in z, and the node at depth 2.
        near(&t, [10.0, 10.0], 1.0, &mut out);
        let keys: Vec<NodeKey> = out.iter().map(|&i| t.keys[i as usize]).collect();
        assert_eq!(
            keys,
            vec![[0, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 1], [2, 0, 0, 0]]
        );
        // A reach that crosses the middle meets every child.
        near(&t, [50.0, 50.0], 0.5, &mut out);
        assert_eq!(out.len(), 1 + 8);
        near(&t, [200.0, 200.0], 1.0, &mut out);
        assert!(out.is_empty());
    }

    fn tree() -> Octree {
        let mut nodes = vec![([0, 0, 0, 0], 100u64)];
        for o in 0..8 {
            nodes.push(([1, o & 1, (o >> 1) & 1, (o >> 2) & 1], 50));
        }
        nodes.push(([2, 0, 0, 0], 10));
        nodes.push(([2, 3, 3, 0], 10));
        Octree::new([50.0, 50.0, 50.0], 50.0, 100.0 / 128.0, nodes)
    }

    #[test]
    fn coarse_first_down_while_points_are_apart() {
        let t = tree();
        let mut out = Vec::new();
        // Far away: a node's spacing is under a pixel; the root only.
        visible(&t, [0.0, 0.0, 100.0, 100.0], 0.5, 2.0, 1_000_000, &mut out);
        assert_eq!(out, vec![0]);
        // Close: everything in view, coarser first; the left bottom quarter's corner first.
        visible(&t, [0.0, 0.0, 30.0, 30.0], 100.0, 2.0, 1_000_000, &mut out);
        let keys: Vec<NodeKey> = out.iter().map(|&i| t.keys[i as usize]).collect();
        assert_eq!(
            keys,
            vec![[0, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 1], [2, 0, 0, 0]]
        );
        // The budget stops the finer ones; the root is drawn whatever it holds.
        visible(&t, [0.0, 0.0, 30.0, 30.0], 100.0, 2.0, 120, &mut out);
        assert_eq!(out, vec![0]);
        visible(
            &t,
            [200.0, 200.0, 300.0, 300.0],
            100.0,
            2.0,
            1_000_000,
            &mut out,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn a_clouds_plan() {
        let s = Shape::PointCloud {
            bounds: [1.0, 2.0, 3.0, 4.0, 6.0, 8.0],
            count: 5.0,
            sources: crate::api::json::Json::Null,
            srid: 0.0,
            style: crate::api::json::Json::Null,
            opacity: None,
        };
        let c = corners(&s).unwrap();
        assert_eq!(c[0], Vec2::new(1.0, 2.0));
        assert_eq!(c[2], Vec2::new(4.0, 6.0));
    }

    /// Kind 21 and a raster's address (docs/adr/0207 §1, §3): packed as the web
    /// packs them and read back as they were.
    #[test]
    fn a_cloud_and_a_rasters_address_are_packed_and_read_back() {
        use crate::api::json::Json;
        use crate::store::{Packer, Store};
        let cloud = Shape::PointCloud {
            bounds: [0.0, 0.0, -0.5, 2.0, 2.0, 1.0],
            count: 15.0,
            sources: Json::parse(
                r#"[{"url":"https://ornek.test/a.copc.laz","format":"copc","count":10,"bounds":[0,0,0,1,1,1]}]"#,
            )
            .unwrap(),
            srid: 5254.0,
            style: Json::parse(r#"{"render":"classification","hidden":[7],"size":2}"#).unwrap(),
            opacity: Some(0.8),
        };
        let raster = Shape::Raster {
            affine: [0.5, 0.0, 0.0, -0.5, 486_500.0, 4_420_300.0],
            width: 400.0,
            height: 300.0,
            bands: 1.0,
            sample: "f32".into(),
            asset: None,
            file: None,
            url: Some("https://ornek.test/dem.tif".into()),
            srid: 5254.0,
            style: Json::parse(r#"{"render":"ramp","bands":[1]}"#).unwrap(),
            opacity: None,
        };
        let mut out = Packer::default();
        out.object(1.0, "a", false, &cloud);
        assert_eq!(out.nums[3], 21.0);
        out.object(2.0, "a", false, &raster);
        let mut back = Store::new();
        back.put_packed(&out.nums, &out.strings).expect("reads");
        assert_eq!(back.get(1.0).map(|it| &it.shape), Some(&cloud));
        assert_eq!(back.get(2.0).map(|it| &it.shape), Some(&raster));
        // The first number that is no kind.
        let mut bad = Packer::default();
        bad.object(3.0, "a", false, &cloud);
        bad.nums[3] = 22.0;
        assert!(
            Store::new()
                .put_packed(&bad.nums, &bad.strings)
                .is_err_and(|e| e.contains("bilinmeyen bir nesne türü (22)"))
        );
    }
}
