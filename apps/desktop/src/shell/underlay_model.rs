//! An underlay model's file as read (docs/adr/8101; plan/ifc-uyum.md IFC-09): checked against the digest the drawing
//! keeps, its bodies made by the reading engine and placed by the rule the command placed it with
//! (`kentos_ifc::underlay::place`), and its plan at a level. Mimari's Altlık modeller reads underlays with it and 3B
//! görünüm shows them (scene3d_building.rs); what reads them and when is the module's (docs/adr/9199 §4.9).

use std::sync::Arc;
use std::time::{Duration, Instant};

use kentos_commands::UnderlayPlacement;
use kentos_commands::encoding::sha256_hex;
use kentos_ifc::read::{self, Model};
use kentos_ifc::underlay::{self, MapFrame, Mesh, PlanLine};

/// One body placed in the drawing's metres: its object (an index into the file's objects), its corners, its triangles.
pub struct Body {
    pub owner: u32,
    pub positions: Arc<Vec<[f64; 3]>>,
    pub triangles: Vec<[u32; 3]>,
}

/// An underlay's file as read: its digest and schema, the model and its bytes (an object's details are read from
/// them when asked), the bodies placed, the objects with a shape that have no body and why, how long the read took,
/// and the bodies' box in the drawing (x, y, z from, x, y, z to).
pub struct Loaded {
    pub sha256: String,
    pub schema: String,
    pub model: Model,
    pub bytes: Vec<u8>,
    pub bodies: Vec<Body>,
    pub not_shown: Vec<(usize, String)>,
    pub took: Duration,
    pub bounds: Option<[f64; 6]>,
}

impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loaded")
            .field("schema", &self.schema)
            .field("objects", &self.model.objects.len())
            .field("bodies", &self.bodies.len())
            .field("not_shown", &self.not_shown.len())
            .field("took", &self.took)
            .finish_non_exhaustive()
    }
}

impl Loaded {
    /// The objects with a shape: those with a body and those without.
    pub fn with_shape(&self) -> usize {
        self.bodies.len() + self.not_shown.len()
    }

    /// What its bodies and the file take in memory, about, in bytes: the file, the corners and triangles.
    pub fn memory(&self) -> usize {
        self.bytes.len()
            + self.bodies.iter().map(|b| b.positions.len() * 24 + b.triangles.len() * 12).sum::<usize>()
    }

    /// The object `owner` as the list says it: its class and its name.
    pub fn object_words(&self, owner: usize) -> String {
        match self.model.objects.get(owner) {
            Some(o) => match o.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
                Some(n) => format!("{} {n}", o.class),
                None => o.class.clone(),
            },
            None => format!("#{owner}"),
        }
    }
}

/// The file `file` read, checked against `sha256` and placed `placement` on `project`; why not, in the interface's
/// words.
pub fn load(file: &str, sha256: &str, placement: UnderlayPlacement, project: Result<MapFrame, String>) -> Result<Loaded, String> {
    let started = Instant::now();
    let bytes = std::fs::read(file)
        .map_err(|_| format!("{file} okunamadı: dosya yok ya da açılamıyor. Yerini Yeniden oku ile verin."))?;
    let digest = sha256_hex(&bytes);
    if digest != sha256 {
        return Err("Dosya eklendiğinden beri değişmiş (SHA-256'sı başka): gösterilen revizyon o değil. Yeniden oku \
                    yeni revizyonu alır."
            .to_owned());
    }
    let model = read::read(&bytes, &read::Limits::default(), &|| false).map_err(|e| e.to_string())?;
    let unit = model.summary.units.length.as_ref().map_or(f64::NAN, |u| u.to_si);
    let frame = model.summary.location.as_ref().and_then(read::Location::frame);
    let core = match placement {
        UnderlayPlacement::Map => underlay::Placement::Map,
        UnderlayPlacement::Origin => underlay::Placement::Origin,
    };
    let t = underlay::place(frame.as_ref(), unit, project.as_ref().map_err(String::as_str), core)?;
    let made = model.bodies(&bytes, &|| false).map_err(|e| e.to_string())?;
    let mut bounds: Option<[f64; 6]> = None;
    let bodies: Vec<Body> = made
        .bodies
        .into_iter()
        .map(|b| {
            let positions: Vec<[f64; 3]> = b.positions.iter().map(|p| t.apply(*p)).collect();
            for p in &positions {
                let k = bounds.get_or_insert([p[0], p[1], p[2], p[0], p[1], p[2]]);
                for i in 0..3 {
                    k[i] = k[i].min(p[i]);
                    k[i + 3] = k[i + 3].max(p[i]);
                }
            }
            Body { owner: u32::try_from(b.object).unwrap_or(u32::MAX), positions: Arc::new(positions), triangles: b.triangles }
        })
        .collect();
    let not_shown = made.not_shown.into_iter().map(|n| (n.object, n.reason)).collect();
    Ok(Loaded { sha256: digest, schema: model.summary.schema.clone(), model, bytes, bodies, not_shown, took: started.elapsed(), bounds })
}

/// The plan of `loaded` at a level: cut at `cut`, the edges under it down to `bottom`.
pub fn plan_of(loaded: &Loaded, cut: f64, bottom: f64) -> Vec<PlanLine> {
    let meshes: Vec<Mesh<'_>> = loaded
        .bodies
        .iter()
        .map(|b| Mesh { owner: b.owner, positions: &b.positions, triangles: &b.triangles })
        .collect();
    underlay::plan(&meshes, cut, bottom)
}
