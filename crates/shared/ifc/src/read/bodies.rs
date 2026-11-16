//! The bodies of a file read (feature `geometry`; docs/adr/8061 §5, §7): the engine's triangulation of every product
//! with a shape, its openings cut (`IfcRelVoidsElement`), as world coordinates in metres and in float64: the engine
//! keeps each mesh as float32 positions about a float64 origin, and the two are added here, so a model on TUREF
//! coordinates keeps its millimetres (docs/adr/8061 §5 "Büyük koordinat"). A body the engine cannot make is listed
//! with its reason, never passed over (CLAUDE.md §7). Every body is made about the model's middle (the engine's
//! relative-to-centre anchor) and the anchor is added back here in float64. A mesh is a view of the body: its area and volume are not the
//! object's measure (CLAUDE.md §23.3), the file's quantity sets and the source model are.
//!
//! In this item the bodies are a trial (plan/ifc-uyum.md IFC-07): their boxes and volumes are measured against another
//! reader's; the viewer (IFC-12) takes them into the product.

use ifc_lite_core::{EntityDecoder, SchemaVersion};
use ifc_lite_geometry::GeometryRouter;
use rustc_hash::FxHashMap;

use super::{Model, ReadError, guarded, header};

/// Ask the host whether to stop once in this many bodies.
const CANCEL_EVERY: usize = 32;

/// One object's body: its object (an index into [`Model::objects`]), its corners in world metres, its triangles, and
/// how far a corner may be from where the file puts it through the engine's float32 numbers, in metres (`rounding`).
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub object: usize,
    pub positions: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub rounding: f64,
}

/// An object with a shape whose body could not be made, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct NotShown {
    pub object: usize,
    pub reason: String,
}

/// The bodies, and the objects with a shape that have none.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bodies {
    pub bodies: Vec<Body>,
    pub not_shown: Vec<NotShown>,
}

impl Model {
    /// Every body of the file read from `bytes` (the same bytes), asking `cancel` now and then whether to stop.
    /// Openings are not bodies: they cut their hosts.
    pub fn bodies(&self, bytes: &[u8], cancel: &dyn Fn() -> bool) -> Result<Bodies, ReadError> {
        let schema = header::version(self.summary.family);
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            // Every position about the model's middle, whatever its size: the engine casts to float32 after taking
            // this anchor off, and a model on TUREF coordinates kept absolute would lose 0.16 m (the trial, §7). The
            // anchor is added back in float64. The engine's own density (Medium): a finer one does not divide a
            // profile's arcs finer.
            let scale = decoder.length_unit_scale();
            let bounds = ifc_lite_core::scan_model_bounds(bytes);
            let anchor = if bounds.is_valid() {
                let (x, y, z) = bounds.centroid();
                (x * scale, y * scale, z * scale)
            } else {
                (0.0, 0.0, 0.0)
            };
            let router = GeometryRouter::with_scale_and_rtc(scale, anchor);
            let voids = self.voids(schema);
            let mut out = Bodies::default();
            let mut n = 0;
            for (i, o) in self.objects.iter().enumerate() {
                if !o.has_shape || schema.is_subtype_of(&o.class, "IfcFeatureElementSubtraction") {
                    continue;
                }
                n += 1;
                if n % CANCEL_EVERY == 0 && cancel() {
                    return Err(ReadError::Cancelled);
                }
                let made = decoder
                    .decode_by_id(o.step)
                    .map_err(|e| e.to_string())
                    .and_then(|e| router.process_element_with_voids(&e, &mut decoder, &voids).map_err(|e| e.to_string()));
                match made {
                    Ok(mesh) if !mesh.indices.is_empty() => {
                        let [ox, oy, oz] = [mesh.origin[0] + anchor.0, mesh.origin[1] + anchor.1, mesh.origin[2] + anchor.2];
                        let positions = mesh
                            .positions
                            .chunks_exact(3)
                            .map(|p| [ox + f64::from(p[0]), oy + f64::from(p[1]), oz + f64::from(p[2])])
                            .collect();
                        let triangles = mesh.indices.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect();
                        let rounding = rounding(&mesh, scale);
                        out.bodies.push(Body { object: i, positions, triangles, rounding });
                    }
                    Ok(_) => out.not_shown.push(NotShown { object: i, reason: "gövdesi boş çıktı".to_owned() }),
                    Err(reason) => out.not_shown.push(NotShown { object: i, reason }),
                }
            }
            Ok(out)
        })
    }

    /// The openings of each host, by record number (`IfcRelVoidsElement`).
    fn voids(&self, schema: SchemaVersion) -> FxHashMap<u32, Vec<u32>> {
        let mut out: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        for r in &self.relations {
            if let Some(host) = r.relating
                && schema.is_subtype_of(&r.class, "IfcRelVoidsElement")
            {
                out.entry(host).or_default().extend(&r.related);
            }
        }
        out
    }
}

/// How far the engine's float32 numbers may put a body's corner from where the file puts it, in metres. The engine
/// stores a corner as float32 twice with float64 arithmetic between (`transform_mesh_world_framed`): first in the
/// object's own frame as its kernel builds it, scaled from the file's unit to metres in float32 when the file is not in
/// metres (the scale factor's own rounding and the product's: two more), then in the world about its anchor. Each
/// float32 rounding is at most half a step, 2⁻²⁴ of the number's size: one at the object frame's reach in metres
/// (three with the scaling), one at the world positions' reach.
fn rounding(mesh: &ifc_lite_geometry::Mesh, scale: f64) -> f64 {
    let world = mesh.positions.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let local = mesh.local_bounds.map_or(world, |b| b.iter().fold(0.0f32, |m, v| m.max(v.abs())));
    let times = if scale == 1.0 { 1.0 } else { 3.0 };
    (times * f64::from(local) + f64::from(world)) * f64::from(f32::EPSILON) / 2.0
}

/// A body's volume (the divergence theorem over its triangles) and area; the volume's sign follows the triangles'
/// winding, which the engine does not promise, so its size is what counts.
pub fn volume_and_area(b: &Body) -> (f64, f64) {
    let (mut v, mut a) = (0.0, 0.0);
    // About the first corner: small numbers, whatever the coordinates.
    let o = b.positions.first().copied().unwrap_or([0.0; 3]);
    for t in &b.triangles {
        let [p, q, r] = t.map(|k| {
            let c = b.positions[k as usize];
            [c[0] - o[0], c[1] - o[1], c[2] - o[2]]
        });
        let cross = [q[1] * r[2] - q[2] * r[1], q[2] * r[0] - q[0] * r[2], q[0] * r[1] - q[1] * r[0]];
        v += (p[0] * cross[0] + p[1] * cross[1] + p[2] * cross[2]) / 6.0;
        let (u, w) = ([q[0] - p[0], q[1] - p[1], q[2] - p[2]], [r[0] - p[0], r[1] - p[1], r[2] - p[2]]);
        let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        a += (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() / 2.0;
    }
    (v.abs(), a)
}
