//! `road.terrains.cloud` v1 (plan/road.md Y15, docs/adr/0928 §2): a terrain's points taken from point clouds. The
//! clouds are scanned once (`pointcloud_areas::Scan`, every source of the clouds, or those an area's box meets); a
//! point of the classes and returns asked, inside the area if one is given, goes to the thinning
//! (`kentos_road::cloud_ground`); the kept points are written on the layer as multi-point objects with their heights
//! (one undo step “Buluttan arazi”), the layer joins the surface's points (or makes a new surface), and the surface's
//! definition records how they were chosen and what the thinning cost: the error is the points left out, scanned again,
//! against the surface made now. The surface's definition is a setting, as `road.terrains.create`'s. The shared cases
//! are `fixtures/commands/v1/road.terrains.cloud.json`.

use std::collections::HashSet;

use kentos_commands::{
    CLOUD_TERRAIN_PIECE, CloudHeights, CloudReturns, CommandResult, CommandWarning, CreateOperation, CrsPoint, CrsSide,
    CrsTransform, EntitiesCreate, Entity, EntityGeometry, HeightKind, HeightReference, MOST_CLOUD_TERRAIN_POINTS, NewObject,
    PointCloudEntity, PointPart, SurfaceCloudDef, SurfaceSources, TerrainSurfaceDef, TerrainsFromCloud,
    TerrainsFromCloudWritten, Vec2,
};
use kentos_domain::Document;
use kentos_geometry_core::vec2::Vec2 as P;
use kentos_road::cloud_ground::{self, Thinned, Thinner, Thinning};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::pointcloud_areas::{Place, Scan, place_in, ring_box};
use crate::road_terrains::{self as common, fail};


/// Classes that are not the ground and are never merged into it: a bridge deck and an overhead structure.
const NOT_GROUND: [u8; 2] = [17, 19];
/// Classes that are above the ground or noise: vegetation, a building, low and high noise.
const ABOVE_GROUND: [u8; 6] = [3, 4, 5, 6, 7, 18];

/// The request checked: the clouds, the area's ring, the surface it writes to and its definition so far.
struct Asked {
    chosen: Vec<(String, PointCloudEntity)>,
    ring: Option<Vec<P>>,
    classes: Vec<u8>,
    rule: Thinning,
    /// The project's surfaces with this one in place (a new one last), and where it is.
    list: Vec<TerrainSurfaceDef>,
    at: usize,
    new: bool,
    warnings: Vec<CommandWarning>,
}

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_CLOUD_TERRAIN, message, path)
}

/// An area's outer ring: a polygon's, or a closed polyline's, of straight edges.
fn area_ring(doc: &Document, uid: &str) -> Result<Vec<P>, Stop> {
    let names = [uid.to_owned()];
    let named = checks::named(doc, &names).map_err(|_| invalid(format!("Alan “{uid}” çizimde yok."), "area"))?;
    let (_, _, e, _) = named.into_iter().next().ok_or_else(|| invalid(format!("Alan “{uid}” çizimde yok."), "area"))?;
    let path = match e {
        Entity::Polygon(p) => p,
        Entity::Polyline(p) if p.closed => p,
        _ => return Err(invalid("Alan kapalı bir çoklu çizgi ya da alan olmalı.".into(), "area")),
    };
    if path.bulges.iter().flatten().any(|b| *b != 0.0) {
        return Err(invalid("Alanın kenarları düz olmalı; yaylı kenar alınmaz.".into(), "area"));
    }
    if path.pts.len() < 3 {
        return Err(invalid("Alanın en az üç köşesi olmalı.".into(), "area"));
    }
    Ok(path.pts.iter().map(|v| P { x: v.x, y: v.y }).collect())
}

fn ask(doc: &Document, input: &TerrainsFromCloud) -> Result<Asked, Stop> {
    common::road_project(doc)?;
    if input.clouds.is_empty() {
        return Err(invalid("En az bir nokta bulutu verin.".into(), "clouds"));
    }
    let chosen = crate::pointclouds::clouds(doc, &input.clouds).map_err(|s| match s {
        Stop::Failed(mut e) => {
            e.path = e.path.map(|p| p.replacen("uids", "clouds", 1));
            Stop::Failed(e)
        }
        other => other,
    })?
    .into_iter()
    .map(|(u, c)| (u, c.clone()))
    .collect::<Vec<_>>();
    if input.classes.is_empty() {
        return Err(invalid("En az bir sınıf verin (ASPRS'de 2 zemin).".into(), "classes"));
    }
    let mut classes = input.classes.clone();
    classes.sort_unstable();
    classes.dedup();
    if let Some(c) = classes.iter().find(|c| NOT_GROUND.contains(c)) {
        let what = if *c == 17 { "köprü tabliyesi (17)" } else { "üst yapı (19)" };
        return Err(fail(
            codes::CLOUD_CLASS_NOT_GROUND,
            format!("Sınıf {what} zemin değil; yer yüzeyiyle birleştirilmez. Sınıflardan çıkarın."),
            "classes",
        ));
    }
    let rule = Thinning { cell: input.cell, z_tolerance: input.z_tolerance, near: input.near };
    if let Err(e) = Thinner::new(rule.clone(), &[]) {
        let path = match e {
            cloud_ground::ThinningError::Cell => "cell",
            cloud_ground::ThinningError::ZTolerance => "zTolerance",
            cloud_ground::ThinningError::Near => "near",
        };
        return Err(invalid(format!("Seyreltme: {e}."), path));
    }
    let own = common::surfaces(doc);
    let (list, at, new) = match (&input.surface, &input.name) {
        (Some(_), Some(_)) | (None, None) => {
            return Err(invalid("Ya surface (projedeki yüzey) ya da name (yeni yüzey) verin, ikisini birden değil.".into(), "surface"));
        }
        (Some(s), None) => {
            if !input.breaklines.is_empty() || input.boundary.is_some() {
                return Err(invalid("breaklines ve boundary yalnız yeni yüzeyde verilir; var olan yüzeyin kendi tanımı kullanılır.".into(), "breaklines"));
            }
            let id = crate::road_profiles::surface_id(doc, s, "surface")?;
            let at = own.iter().position(|x| x.id == id).unwrap_or_default();
            (own.to_vec(), at, false)
        }
        (None, Some(name)) => {
            let id = (1..).map(|n| format!("t-{n}")).find(|id| own.iter().all(|s| &s.id != id)).unwrap_or_default();
            let mut list = own.to_vec();
            list.push(TerrainSurfaceDef {
                id,
                name: name.clone(),
                points: SurfaceSources::default(),
                breaklines: input.breaklines.clone(),
                boundary: input.boundary.clone(),
                voids: Vec::new(),
                tolerance: None,
                max_edge: None,
                built: None,
                clouds: Vec::new(),
                excluded: Vec::new(),
            });
            let at = list.len() - 1;
            (list, at, true)
        }
    };
    let ring = input.area.as_deref().map(|a| area_ring(doc, a)).transpose()?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    let above: Vec<String> = classes.iter().filter(|c| ABOVE_GROUND.contains(c)).map(u8::to_string).collect();
    if !above.is_empty() {
        warnings.push(CommandWarning {
            code: codes::CLOUD_CLASS_ABOVE_GROUND.into(),
            message: format!(
                "Sınıf {} zemin değil (bitki, bina ya da gürültü); yüzey onların noktalarını zemin sayar.",
                above.join(", ")
            ),
            path: Some("classes".into()),
        });
    }
    Ok(Asked { chosen, ring, classes, rule, list, at, new, warnings })
}

/// Whether a record's return is one asked.
fn return_kept(returns: CloudReturns, number: u8, count: u8) -> bool {
    match returns {
        CloudReturns::All => true,
        CloudReturns::First => number <= 1,
        CloudReturns::Last => number == count || count == 0,
        CloudReturns::Single => count <= 1,
    }
}

/// The clouds scanned: each point of the classes and returns asked in the area, in the scan's order, to `see`; the
/// points read (every class).
fn scan(
    doc: &Document,
    input: &TerrainsFromCloud,
    a: &Asked,
    warnings: &mut Vec<CommandWarning>,
    see: &mut dyn FnMut([f64; 3]),
) -> Result<u64, Stop> {
    let boxes = match &a.ring {
        Some(r) => vec![ring_box(r)],
        None => vec![[f64::NEG_INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::INFINITY]],
    };
    let chosen: Vec<(String, &PointCloudEntity)> = a.chosen.iter().map(|(u, c)| (u.clone(), c)).collect();
    let run = Scan {
        doc,
        drawing: input.drawing.as_deref(),
        chosen: &chosen,
        boxes: &boxes,
        what: "Bulutlar",
        path: "clouds",
    };
    let mut read = vec![0u64; a.chosen.len()];
    run.run(warnings, &mut read, &mut |s| {
        if !a.classes.contains(&s.layout.class(s.record)) {
            return;
        }
        let (n, c) = s.layout.returns(s.record);
        if !return_kept(input.returns, n, c) {
            return;
        }
        if let Some(r) = &a.ring
            && place_in(r, P { x: s.xyz[0], y: s.xyz[1] }) == Place::Outside
        {
            return;
        }
        see(s.xyz);
    })?;
    Ok(read.iter().sum())
}

/// The kept points as multi-point objects of at most [`CLOUD_TERRAIN_PIECE`] points.
fn objects_of(thinned: &Thinned) -> Vec<NewObject> {
    thinned
        .points
        .chunks(CLOUD_TERRAIN_PIECE)
        .map(|piece| {
            let first = piece[0].1;
            let parts: Vec<PointPart> =
                piece[1..].iter().map(|(_, p)| PointPart { p: Vec2 { x: p[0], y: p[1] }, z: Some(p[2]) }).collect();
            NewObject {
                geometry: EntityGeometry::Point {
                    p: Vec2 { x: first[0], y: first[1] },
                    z: Some(first[2]),
                    parts: (!parts.is_empty()).then_some(parts),
                },
                color: None,
                line_weight: None,
                line_type: None,
                attrs: None,
                label: None,
                label_of: None,
                label_scale: None,
                symbol: None,
                order: None,
                transparency: None,
            }
        })
        .collect()
}

/// The breaklines' paths in plan of `def` now.
fn breaklines_of(doc: &Document, def: &TerrainSurfaceDef) -> Vec<Vec<[f64; 2]>> {
    let made = common::make(doc, def);
    made.gathered.input.breaklines.iter().map(|b| b.points.iter().map(|p| [p[0], p[1]]).collect()).collect()
}

/// The heights of `points` turned from the clouds' (`from`, as the call said them) into the project's
/// (docs/adr/1032), as `cad.crs.transform` turns them: the project's horizontal system on both sides, the geoid models
/// between. With `dry`, points the grids do not reach are not refused (a trial before the scan).
fn turned(doc: &Document, from: &HeightReference, points: &mut [[f64; 3]], dry: bool) -> Result<(), Stop> {
    if points.is_empty() {
        return Ok(());
    }
    let input = CrsTransform {
        points: points.iter().map(|p| CrsPoint { x: p[0], y: p[1], z: Some(p[2]) }).collect(),
        from: Some(CrsSide { srid: None, heights: Some(from.clone()), epoch: None }),
        to: None,
        uncertainty: None,
        velocity: None,
    };
    let words = |t: &str| {
        t.replace("from tarafının", "bulutun")
            .replace("to tarafının", "projenin")
            .replace("from yüksekliği", "Bulutun kotları")
            .replace("to yüksekliği", "Projenin kotları")
    };
    match crate::crs_transform::answer(doc, &input) {
        CommandResult::Completed { output, .. } => {
            if !output.unreached.is_empty() && !dry {
                return Err(invalid(
                    format!(
                        "{} noktanın kotu çevrilemiyor: geoit ızgarasının dışında ya da boşluğunda. Alanı ızgaranın içine alın ya da kapsayan ızgarayı Proje ayarları › Izgaralar'a ekleyin.",
                        output.unreached.len()
                    ),
                    "heights",
                ));
            }
            for (p, q) in points.iter_mut().zip(&output.points) {
                if let Some(z) = q.as_ref().and_then(|q| q.z) {
                    p[2] = z;
                }
            }
            Ok(())
        }
        CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
            Err(fail(&error.code, words(&error.message), "heights"))
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => Err(Stop::Cancelled),
    }
}

/// The heights the call says are the clouds' (docs/adr/1032), checked before the clouds are read: a known kind, the
/// project's heights known, the grids the turn needs at hand (tried on the clouds' centre).
fn stated(doc: &Document, a: &Asked, heights: Option<&HeightReference>) -> Result<Option<CloudHeights>, Stop> {
    let Some(h) = heights else { return Ok(None) };
    if h.kind == HeightKind::Unknown {
        return Err(invalid("Bulutun kotlarının ne olduğunu söyleyin (elipsoit ya da ortometrik); bilinmiyorsa heights'ı bırakın.".into(), "heights"));
    }
    if let Some((at, why)) = h.problem() {
        return Err(fail(codes::INVALID_HEIGHTS, format!("Bulutun kotları: {why}."), &format!("heights.{at}")));
    }
    let Some(project) = doc.settings().heights.clone().filter(|p| crate::road_heights::System::of_ref(p).is_some()) else {
        return Err(fail(
            codes::HEIGHTS_UNKNOWN,
            "Bulutun kotları projenin kotlarına çevrilir; projenin kotlarının düşey sistemi söylenmemiş. Proje ayarları › Kotlar'da seçin.".into(),
            "heights",
        ));
    };
    let centre: Vec<[f64; 3]> = a
        .chosen
        .iter()
        .take(1)
        .map(|(_, c)| [(c.cloud.bounds[0] + c.cloud.bounds[3]) / 2.0, (c.cloud.bounds[1] + c.cloud.bounds[4]) / 2.0, 0.0])
        .collect();
    turned(doc, h, &mut centre.clone(), true)?;
    Ok(Some(CloudHeights { from: h.clone(), to: project }))
}

fn run(cx: &mut ExecutionContext<'_>, input: &TerrainsFromCloud) -> CommandResult<TerrainsFromCloudWritten> {
    let a = match ask(cx.doc, input) {
        Ok(a) => a,
        Err(stop) => return stop.into(),
    };
    let mut warnings = a.warnings.clone();
    // Docs/adr/1029: the files' vertical system, refused where it differs from the project's or the surface's others'.
    let others: Vec<(&str, Option<crate::road_heights::System>)> = a.list[a.at]
        .clouds
        .iter()
        .filter(|c| c.layer != input.layer_id)
        .map(|c| (c.layer.as_str(), crate::road_heights::System::of_record(c)))
        .collect();
    // Docs/adr/1032: the heights the call says, checked (and the turn tried) before the clouds are read.
    let heights = match stated(cx.doc, &a, input.heights.as_ref()) {
        Ok(h) => h,
        Err(stop) => return stop.into(),
    };
    let vertical = match crate::road_heights::of_clouds(
        cx.doc,
        input.drawing.as_deref(),
        &a.chosen,
        &others,
        input.heights.as_ref(),
        "clouds",
    ) {
        Ok(v) => v,
        Err(stop) => return stop.into(),
    };
    let lines = if a.rule.near.is_some() { breaklines_of(cx.doc, &a.list[a.at]) } else { Vec::new() };
    let mut thinner = match Thinner::new(a.rule.clone(), &lines) {
        Ok(t) => t,
        Err(e) => return invalid(format!("Seyreltme: {e}."), "cell").into(),
    };
    let scanned = match scan(cx.doc, input, &a, &mut warnings, &mut |p| thinner.feed(p)) {
        Ok(n) => n,
        Err(stop) => return stop.into(),
    };
    let mut thinned = thinner.finish();
    if thinned.read == 0 {
        return fail(
            codes::CLOUD_GROUND_EMPTY,
            "Bulutlarda istenen sınıf ve dönüşten, alanın içinde nokta yok; sınıfları, dönüşü ya da alanı denetleyin.".into(),
            "classes",
        )
        .into();
    }
    if thinned.points.len() as u64 > MOST_CLOUD_TERRAIN_POINTS {
        return invalid(
            format!(
                "Seyreltmeden sonra {} nokta kalıyor; bir çağrı en çok {MOST_CLOUD_TERRAIN_POINTS} nokta yazar. Hücreyi büyütün ya da alanı küçültün.",
                thinned.points.len()
            ),
            "cell",
        )
        .into();
    }
    let ring: Option<Vec<[f64; 2]>> = a.ring.as_ref().map(|r| r.iter().map(|p| [p.x, p.y]).collect());
    let Some(cover) = cloud_ground::coverage(&thinned, a.rule.cell, ring.as_deref()) else {
        return Stop::Cancelled.into();
    };
    // Docs/adr/1032: the kept points' heights into the project's. A turn by a geoid moves heights by a separation that
    // changes by centimetres a kilometre: the cells and their steepness were judged on the clouds' own heights alike.
    if let Some(h) = &heights {
        let mut kept: Vec<[f64; 3]> = thinned.points.iter().map(|(_, p)| *p).collect();
        if let Err(stop) = turned(cx.doc, &h.from, &mut kept, false) {
            return stop.into();
        }
        for ((_, p), q) in thinned.points.iter_mut().zip(kept) {
            *p = q;
        }
    }
    // The surface with the layer among its points and the record in place, judged before anything is written.
    let mut list = a.list.clone();
    let mut def = list[a.at].clone();
    if !def.points.layers.contains(&input.layer_id) {
        def.points.layers.push(input.layer_id.clone());
    }
    let mut record = SurfaceCloudDef {
        layer: input.layer_id.clone(),
        clouds: a.chosen.iter().map(|(u, _)| u.clone()).collect(),
        classes: a.classes.clone(),
        returns: input.returns,
        area: input.area.clone(),
        cell: a.rule.cell,
        z_tolerance: a.rule.z_tolerance,
        near: a.rule.near,
        read: thinned.read,
        kept: thinned.points.len() as u64,
        coverage: cover.share,
        density: cover.density,
        error_max: None,
        error_rms: None,
        outside: 0,
        vertical,
        heights: heights.clone(),
    };
    def.clouds.retain(|c| c.layer != input.layer_id);
    def.clouds.push(record.clone());
    list[a.at] = def.clone();
    if let Err(stop) = common::judged(&list, a.at, "").and_then(|()| checks::revision(cx.doc, input.expected_revision.as_deref())) {
        return stop.into();
    }
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: objects_of(&thinned),
        operation: Some(CreateOperation::CloudTerrain),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, warnings: w } => {
            for x in w {
                if !warnings.iter().any(|y| y.code == x.code) {
                    warnings.push(x);
                }
            }
            output
        }
        other => return crate::trim::relay(other),
    };
    // The surface made now, and the points left out against it.
    let made = common::make(cx.doc, &def);
    let surface = match made.built() {
        Ok(s) => s,
        Err(stop) => return stop.into(),
    };
    let kept: HashSet<u64> = thinned.points.iter().map(|x| x.0).collect();
    let mut order = 0u64;
    let mut left = Vec::new();
    let mut ignored = Vec::new();
    if let Err(stop) = scan(cx.doc, input, &a, &mut ignored, &mut |p| {
        if !kept.contains(&order) {
            left.push(p);
        }
        order += 1;
    }) {
        return stop.into();
    }
    if let Some(h) = &heights
        && let Err(stop) = turned(cx.doc, &h.from, &mut left, false)
    {
        return stop.into();
    }
    let r = cloud_ground::residuals(surface, left);
    if r.count > 0 {
        record.error_max = Some(r.max);
        record.error_rms = Some(r.rms);
    }
    record.outside = r.outside;
    def.clouds.retain(|c| c.layer != input.layer_id);
    def.clouds.push(record.clone());
    if a.new {
        def.built = Some(made.gathered.digest.clone());
    }
    let report = made.report(&def);
    list[a.at] = def;
    cx.doc.set_settings(common::with_surfaces(cx.doc, list));
    CommandResult::Completed {
        output: TerrainsFromCloudWritten {
            cloud: record,
            scanned,
            by_cell: thinned.by_cell,
            by_slope: thinned.by_slope,
            by_line: thinned.by_line,
            cells: cover.cells,
            occupied: cover.occupied,
            area: cover.area,
            uids: created.created,
            surface: report,
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}

/// Checks `input` against the document, reading no cloud.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsFromCloud) -> CommandResult<()> {
    match ask(cx.doc, input) {
        Ok(a) => CommandResult::Completed { output: (), warnings: a.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsFromCloud) -> CommandResult<TerrainsFromCloudWritten> {
    let mut copy = cx.doc.reading_copy();
    let mut planned = run(&mut ExecutionContext::new(&mut copy), input);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the points (one undo step) and the surface's definition (a setting).
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsFromCloud) -> CommandResult<TerrainsFromCloudWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_FROM_CLOUD,
        kentos_commands::ROAD_TERRAINS_FROM_CLOUD_VERSION,
        input,
        |cx, input| run(cx, &input),
    )
}
