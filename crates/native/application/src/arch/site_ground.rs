//! The site's ground (docs/adr/8174 §5): a terrain surface made from a layer's survey points, set as the site's natural
//! or graded ground (the site's own: its layers; an architecture project keeps no road settings); the garden's
//! earthwork, the graded ground over the natural one within the parcel and out of the building's footprint (hafriyat,
//! PAİY 57/2-a); and ±0.00 from the natural ground at the building's corners, their arithmetic mean (PAİY 12/4: "Tabii
//! zeminden kotlandırmada, ± 0.00 kotu binanın köşe kotlarının aritmetik ortalaması alınarak bulunur"). The building is
//! its footprint level's walls' outline: the lowest level at or over ±0.00 with walls (its zemin kat).

use kentos_commands::{Entity, ProjectSettings, SiteGround, SurfaceSources, TerrainSurfaceDef, Vec2};
use kentos_domain::{Document, Uuid};
use kentos_road::surface::{Region, Surface};
use kentos_road::volume_materials;

/// The natural ground's surface id, and its name.
pub const NATURAL_ID: &str = "tabii-zemin";
pub const NATURAL_NAME: &str = "Tabii zemin";
/// The graded ground's.
pub const GRADED_ID: &str = "tesviye-zemin";
pub const GRADED_NAME: &str = "Tesviye zemin";

/// A ground made: its triangles, its plan area (m²) and its lowest and highest heights (m, absolute).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Made {
    pub triangles: usize,
    pub area: f64,
    pub low: f64,
    pub high: f64,
}

/// The garden's earthwork, the graded ground over the natural one within the parcel and out of the building's footprint:
/// the cut and the fill (m³) and the area both grounds cover there (m²).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Earthwork {
    pub cut: f64,
    pub fill: f64,
    pub area: f64,
}

/// The surface of `ground`, made now (its heights absolute): the terrain surface of its layers' points with a height,
/// the road's own making (`road_terrains`); none when they make none.
pub fn made_of(doc: &Document, ground: &SiteGround, graded: bool) -> Option<Surface> {
    crate::road_terrains::made_of(doc, &definition(ground, graded).ok()?).1
}

/// `ground` as a terrain surface's definition: its id and name, its layers its points' sources; serde's defaults for
/// the rest (the contract derives no Default).
fn definition(ground: &SiteGround, graded: bool) -> Result<TerrainSurfaceDef, String> {
    let (id, name) = if graded { (GRADED_ID, GRADED_NAME) } else { (NATURAL_ID, NATURAL_NAME) };
    serde_json::from_value(serde_json::json!({
        "id": id, "name": name, "points": SurfaceSources { layers: ground.layers.clone(), uids: Vec::new() },
    }))
    .map_err(|e| e.to_string())
}

/// The settings with the site's natural ground (or `graded`) made of the points on `layer`, the surface made; or why
/// none is: too few points, or the other ground's layer (the earthwork is the two's difference).
pub fn with_ground(doc: &Document, layer: &str, graded: bool) -> Result<(ProjectSettings, Made), String> {
    let name = if graded { GRADED_NAME } else { NATURAL_NAME };
    let ground = SiteGround::of_layer(layer);
    let site = doc.settings().arch.as_ref().and_then(|a| a.site.as_ref());
    let other = site.and_then(|s| if graded { s.natural_ground.as_ref() } else { s.graded_ground.as_ref() });
    if other == Some(&ground) {
        return Err(format!(
            "Bu katman {} katmanı: tabii ve tesviye zemin ayrı noktalardan kurulur, hafriyat ikisinin farkıdır.",
            if graded { "tabii zeminin" } else { "tesviye zeminin" }
        ));
    }
    let surface = made_of(doc, &ground, graded)
        .ok_or_else(|| format!("Bu katmanın noktalarından {} kurulamadı: kotlu en az üç nokta gerekir.", name.to_lowercase()))?;
    let made = made(&surface);
    let mut settings = doc.settings().clone();
    let site = settings.arch.get_or_insert_with(Default::default).site.get_or_insert_with(Default::default);
    if graded {
        site.graded_ground = Some(ground);
    } else {
        site.natural_ground = Some(ground);
    }
    Ok((settings, made))
}

fn made(s: &Surface) -> Made {
    let f = s.figures();
    Made { triangles: f.triangles, area: f.area, low: f.min_z.unwrap_or(0.0), high: f.max_z.unwrap_or(0.0) }
}


/// The parcel's outline: the site's parcel, a closed line or a polygon by its persistent id.
fn parcel(doc: &Document) -> Option<Vec<[f64; 2]>> {
    let uid = doc.settings().arch.as_ref()?.site.as_ref()?.parcel.as_deref()?;
    let slot = doc.slot_of(Uuid::parse_str(uid).ok()?)?;
    match doc.get(slot)? {
        Entity::Polyline(p) | Entity::Polygon(p) if p.pts.len() >= 3 => Some(p.pts.iter().map(|q| [q.x, q.y]).collect()),
        _ => None,
    }
}

/// The garden's earthwork when the site has both grounds and a parcel: within it, the building's footprint (when
/// its walls enclose one) left out.
pub fn earthwork(doc: &Document) -> Option<Earthwork> {
    let site = doc.settings().arch.as_ref()?.site.as_ref()?;
    let natural = made_of(doc, site.natural_ground.as_ref()?, false)?;
    let graded = made_of(doc, site.graded_ground.as_ref()?, true)?;
    let ring = parcel(doc)?;
    let holes = building_corners(doc).map(|c| c.iter().map(|p| [p.x, p.y]).collect()).into_iter().collect();
    let triangles = |s: &Surface| -> Vec<_> { s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect() };
    let m = volume_materials::measure(&triangles(&graded), &triangles(&natural), &[Region { outer: ring, holes }]);
    let m = m.first()?;
    Some(Earthwork { cut: m.cut, fill: m.fill, area: m.covered })
}

/// The level whose walls make the building's footprint: the lowest at or over ±0.00 with walls (its zemin kat), else
/// the highest under it; none in a project without levels or with no wall on one (every wall then).
fn footprint_level(doc: &Document) -> Option<String> {
    let walled: std::collections::HashSet<&str> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Wall(w) => w.wall.arch.level.as_deref(),
            _ => None,
        })
        .collect();
    let mut levels: Vec<_> =
        doc.settings().arch.as_ref()?.levels.iter().filter(|l| walled.contains(l.id.as_str())).collect();
    levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    levels.iter().find(|l| l.elevation >= -0.005).or(levels.last()).map(|l| l.id.clone())
}

/// The building's corners: the outermost outline of its footprint level's walls' line work to their outer faces, found
/// from a place just off a wall's side, the first a wall encloses (inside an L as inside a box); a vertex on a straight
/// run is not a corner. None when the walls enclose nothing.
pub fn building_corners(doc: &Document) -> Option<Vec<Vec2>> {
    let level = footprint_level(doc);
    let level = level.as_deref();
    for e in doc.entities() {
        let Entity::Wall(w) = e else { continue };
        if level.is_some() && w.wall.arch.level.as_deref() != level {
            continue;
        }
        // Past the band whichever side of its line it lies.
        let off = w.wall.width + 0.05;
        for seg in w.wall.pts.windows(2) {
            let (a, b) = (&seg[0], &seg[1]);
            let len = (b.x - a.x).hypot(b.y - a.y);
            if len <= 1e-9 {
                continue;
            }
            let (nx, ny) = (-(b.y - a.y) / len * off, (b.x - a.x) / len * off);
            for s in [1.0, -1.0] {
                let seed = Vec2 { x: (a.x + b.x) / 2.0 + s * nx, y: (a.y + b.y) / 2.0 + s * ny };
                if let Some(part) = doc.slab_from_walls(seed, level, false) {
                    return Some(corners(&part.outer.pts));
                }
            }
        }
    }
    None
}

/// A ring's corners: its closing repeat and every vertex on a straight run (its edges' turn under a hundredth of a
/// degree) left out.
fn corners(ring: &[Vec2]) -> Vec<Vec2> {
    let mut pts = ring.to_vec();
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    let n = pts.len();
    (0..n)
        .filter(|&i| {
            let (p, c, q) = (&pts[(i + n - 1) % n], &pts[i], &pts[(i + 1) % n]);
            let (ax, ay, bx, by) = (c.x - p.x, c.y - p.y, q.x - c.x, q.y - c.y);
            (ax * by - ay * bx).abs() > 2e-4 * ax.hypot(ay) * bx.hypot(by)
        })
        .map(|i| pts[i])
        .collect()
}

/// ±0.00 from the natural ground (PAİY 12/4): the mean of its heights at the building's corners; none without the
/// ground or the building, or with a corner off the ground.
pub fn datum_from_corners(doc: &Document) -> Option<f64> {
    let site = doc.settings().arch.as_ref()?.site.as_ref()?;
    let natural = made_of(doc, site.natural_ground.as_ref()?, false)?;
    let heights: Option<Vec<f64>> =
        building_corners(doc)?.iter().map(|p| natural.height(p.x, p.y).z).collect();
    let heights = heights?;
    (!heights.is_empty()).then(|| heights.iter().sum::<f64>() / heights.len() as f64)
}

/// The site's natural and graded ground made now, either absent (not set, or its points make none).
#[derive(Clone, Debug, Default)]
pub struct SiteSurfaces {
    natural: Option<Surface>,
    graded: Option<Surface>,
}

impl SiteSurfaces {
    /// Whether neither ground is made.
    pub fn is_empty(&self) -> bool {
        self.natural.is_none() && self.graded.is_none()
    }

    /// The natural and the graded ground's heights at (x, y), absolute; none off a ground.
    pub fn heights(&self, x: f64, y: f64) -> (Option<f64>, Option<f64>) {
        let at = |s: &Option<Surface>| s.as_ref().and_then(|s| s.height(x, y).z);
        (at(&self.natural), at(&self.graded))
    }
}

/// The site's grounds made now.
pub fn surfaces(doc: &Document) -> SiteSurfaces {
    let Some(site) = doc.settings().arch.as_ref().and_then(|a| a.site.as_ref()) else {
        return SiteSurfaces::default();
    };
    SiteSurfaces {
        natural: site.natural_ground.as_ref().and_then(|g| made_of(doc, g, false)),
        graded: site.graded_ground.as_ref().and_then(|g| made_of(doc, g, true)),
    }
}

/// A ground level mark's lines (EK-4 4.30, Şekil 4.6: "tzk 849.85 (-5.65)" over "tsk 850.10 (-4.40)"): the natural
/// ground's height at a place (tzk) and the graded ground's (tsk), each absolute and, with ±0.00's elevation, in
/// brackets its difference from it, as the drawing writes levels (a point, two decimals). A ground not there gives
/// no line.
pub fn level_lines(natural: Option<f64>, graded: Option<f64>, datum: Option<f64>) -> Vec<String> {
    let line = |word: &str, z: f64| match datum {
        Some(d) if (z - d).abs() < 0.005 => format!("{word} {z:.2} (±0.00)"),
        Some(d) => format!("{word} {z:.2} ({:+.2})", z - d),
        None => format!("{word} {z:.2}"),
    };
    natural.map(|z| line("tzk", z)).into_iter().chain(graded.map(|z| line("tsk", z))).collect()
}
