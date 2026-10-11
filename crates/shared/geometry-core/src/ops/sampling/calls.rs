//! The Örnekleme tools' calls for the web (`model/ops/sampling.ts`): whole
//! lists in, flat lists of numbers out (a grid of a hundred thousand cells
//! crosses as four arrays, not as objects), one call a run.

use super::along::{Spacing, how_many, marks};
use super::cover::Cover;
use super::grid::{self, Kind, Piece, Spec};
use super::regular::regular;
use super::scatter::{Target, scatter};
use super::subdivide::subdivide;
use super::walk::walks_of;
use super::{MOST, count_of, extent, inside_of, random::seed_of, target_of};
use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape};
use crate::op;
use crate::ops::convert::{Elevations, Zs};
use crate::vec2::Vec2;

/// Why a grid has nothing to cover.
pub const NO_EXTENT: &str = "Kapsamda kutusu olan nesne yok.";

pub const TOO_MANY_POINTS: &str =
    "1 000 000'dan çok nokta yazılırdı; sayıyı ya da aralığı değiştirin.";

/// A run's results, or why it was refused: `{ <name> }` or `{ refused }`.
struct Run<T: ToJson>(Result<T, String>, &'static str);

impl<T: ToJson> ToJson for Run<T> {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(v) => field(out, &mut first, self.1, v),
            Err(why) => field(out, &mut first, "refused", why),
        }
        out.push('}');
    }
}

/// A grid as flat lists: each piece's row and column (0: none), its number of points, then every point's x and y.
struct GridOut(Vec<Piece>);

impl ToJson for GridOut {
    fn write_json(&self, out: &mut String) {
        let num = |v: Option<u32>| v.map_or(0.0, f64::from);
        let rows: Vec<f64> = self.0.iter().map(|p| num(p.row)).collect();
        let cols: Vec<f64> = self.0.iter().map(|p| num(p.col)).collect();
        let counts: Vec<f64> = self.0.iter().map(|p| p.pts.len() as f64).collect();
        let xy: Vec<f64> = self
            .0
            .iter()
            .flat_map(|p| p.pts.iter().flat_map(|q| [q.x, q.y]))
            .collect();
        out.push('{');
        let mut first = true;
        field(out, &mut first, "rows", &rows);
        field(out, &mut first, "cols", &cols);
        field(out, &mut first, "counts", &counts);
        field(out, &mut first, "xy", &xy);
        out.push('}');
    }
}

/// Points as flat lists: each point's object, its x and y; elevations,
/// distances, bearings and paths (from 1 in an object of more than one, 0
/// otherwise) where the tool gives them.
#[derive(Default)]
struct PointsOut {
    at: Vec<f64>,
    xy: Vec<f64>,
    z: Option<Vec<Option<f64>>>,
    s: Option<Vec<Option<f64>>>,
    bearing: Option<Vec<f64>>,
    path: Option<Vec<f64>>,
    too_close: Option<f64>,
    thin: Option<f64>,
    thin_lost: Option<f64>,
}

impl ToJson for PointsOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "at", &self.at);
        field(out, &mut first, "xy", &self.xy);
        field(out, &mut first, "z", &self.z);
        field(out, &mut first, "s", &self.s);
        field(out, &mut first, "bearing", &self.bearing);
        field(out, &mut first, "path", &self.path);
        field(out, &mut first, "tooClose", &self.too_close);
        field(out, &mut first, "thin", &self.thin);
        field(out, &mut first, "thinLost", &self.thin_lost);
        out.push('}');
    }
}

/// A piece and its elevations as the web reads them: `{ shape, zs? }`.
struct PieceOut(Elevations);

impl ToJson for PieceOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "shape", &Entity::new(self.0.shape.clone()));
        field(out, &mut first, "zs", &self.0.zs);
        out.push('}');
    }
}

fn shapes(entities: &[Entity]) -> Vec<Shape> {
    entities.iter().map(|e| e.shape.clone()).collect()
}

fn zs_at(zs: &[Option<Zs>], i: usize) -> Option<&Zs> {
    zs.get(i).and_then(Option::as_ref)
}

/// Ağ oluştur (§2) over the objects' box; `keep` keeps the cells that meet them.
pub fn grid_of(shapes: &[Shape], spec: &Spec, keep: bool) -> Result<Vec<Piece>, String> {
    let Some(b) = extent(shapes) else {
        return Err(NO_EXTENT.to_owned());
    };
    let got = if keep && spec.kind != Kind::Lines {
        let cover = Cover::new(shapes);
        let mut near = Vec::new();
        let mut meets = |ring: &[Vec2]| cover.meets(ring, &mut near);
        grid::pieces(&b, spec, Some(&mut meets))
    } else {
        grid::pieces(&b, spec, None)
    };
    got.map_err(str::to_owned)
}

fn random_run(
    entities: &[Entity],
    zs: &[Option<Zs>],
    counts: &[Option<f64>],
    min: f64,
    tries: f64,
    seed: f64,
) -> Result<PointsOut, String> {
    let n: Vec<u64> = (0..entities.len())
        .map(|i| count_of(counts.get(i).copied().flatten()).unwrap_or(0))
        .collect();
    if n.iter().map(|&c| c as f64).sum::<f64>() > MOST as f64 {
        return Err(TOO_MANY_POINTS.to_owned());
    }
    let targets: Vec<_> = entities
        .iter()
        .enumerate()
        .map(|(i, e)| target_of(&e.shape, zs_at(zs, i)))
        .collect();
    let tries = if tries.is_finite() && tries >= 1.0 {
        crate::jsmath::js_min(tries, 1000.0) as u32
    } else {
        1
    };
    let got = scatter(&targets, &n, min, tries, seed_of(seed));
    let (mut at, mut xy, mut z, mut s, mut path) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (i, dots) in got.dots.iter().enumerate() {
        let many = matches!(&targets[i], Target::Paths(w) if w.len() > 1);
        for d in dots {
            at.push(i as f64);
            xy.extend([d.p.x, d.p.y]);
            z.push(d.z);
            s.push(d.s);
            path.push(if many { (d.path + 1) as f64 } else { 0.0 });
        }
    }
    Ok(PointsOut {
        at,
        xy,
        z: Some(z),
        s: Some(s),
        path: Some(path),
        too_close: Some(got.refused as f64),
        thin: Some(got.thin as f64),
        thin_lost: Some(got.thin_lost as f64),
        ..PointsOut::default()
    })
}

fn along_run(
    entities: &[Entity],
    zs: &[Option<Zs>],
    spacing: &Spacing,
    ends: bool,
) -> Result<PointsOut, String> {
    let walks: Vec<_> = entities
        .iter()
        .enumerate()
        .map(|(i, e)| walks_of(&e.shape, zs_at(zs, i)))
        .collect();
    let total: f64 = walks
        .iter()
        .flatten()
        .map(|w| how_many(w.length, spacing, ends))
        .sum();
    if total > MOST as f64 {
        return Err(TOO_MANY_POINTS.to_owned());
    }
    let (mut at, mut xy, mut z, mut s, mut bearing, mut path) = (
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    for (i, w) in walks.iter().enumerate() {
        let many = w.len() > 1;
        for m in marks(w, spacing, ends) {
            at.push(i as f64);
            xy.extend([m.p.x, m.p.y]);
            z.push(m.z);
            s.push(Some(m.s));
            bearing.push(m.bearing);
            path.push(if many { (m.path + 1) as f64 } else { 0.0 });
        }
    }
    Ok(PointsOut {
        at,
        xy,
        z: Some(z),
        s: Some(s),
        bearing: Some(bearing),
        path: Some(path),
        ..PointsOut::default()
    })
}

pub(crate) static OPS: &[Op] = &[
    op!("sampleGrid", |entities: Vec<Entity>,
                       kind: String,
                       dx: f64,
                       dy: f64,
                       side: f64,
                       snap: bool,
                       keep: bool| {
        let Some(kind) = Kind::from_key(&kind) else {
            return Err(format!("Bilinmeyen ağ biçimi: {kind}"));
        };
        let spec = Spec {
            kind,
            dx,
            dy,
            side,
            snap,
        };
        Run(
            grid_of(&shapes(&entities), &spec, keep).map(GridOut),
            "grid",
        )
    }),
    op!("sampleRegular", |entities: Vec<Entity>,
                          dx: f64,
                          dy: f64,
                          snap: bool,
                          shift: f64,
                          seed: f64| {
        let objects: Vec<_> = entities.iter().map(|e| inside_of(&e.shape)).collect();
        let got = regular(&objects, dx, dy, snap, shift, seed_of(seed)).map(|lists| {
            let mut o = PointsOut::default();
            for (i, pts) in lists.iter().enumerate() {
                for p in pts {
                    o.at.push(i as f64);
                    o.xy.extend([p.x, p.y]);
                }
            }
            o
        });
        Run(got.map_err(str::to_owned), "points")
    }),
    op!("sampleRandom", |entities: Vec<Entity>,
                         zs: Vec<Option<Zs>>,
                         counts: Vec<Option<f64>>,
                         min: f64,
                         tries: f64,
                         seed: f64| {
        Run(
            random_run(&entities, &zs, &counts, min, tries, seed),
            "points",
        )
    }),
    op!("sampleAlong", |entities: Vec<Entity>,
                        zs: Vec<Option<Zs>>,
                        mode: String,
                        step: f64,
                        start: f64,
                        end: f64,
                        parts: f64,
                        ends: bool| {
        let spacing = match mode.as_str() {
            "interval" => Spacing::Interval { step, start, end },
            "parts" => Spacing::Parts(if parts >= 1.0 { parts as u64 } else { 1 }),
            _ => return Err(format!("Bilinmeyen yerleşim: {mode}")),
        };
        Run(along_run(&entities, &zs, &spacing, ends), "points")
    }),
    op!("sampleSubdivide", |entities: Vec<Entity>,
                            zs: Vec<Option<Zs>>,
                            max: f64| {
        let max = if max.is_finite() && max >= 1.0 {
            max as usize
        } else {
            1
        };
        entities
            .iter()
            .enumerate()
            .map(|(i, e)| {
                subdivide(&e.shape, zs_at(&zs, i), max)
                    .into_iter()
                    .map(PieceOut)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }),
];
