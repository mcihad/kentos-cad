//! `file.import.landxml` v1 (plan/road.md Y2.5, docs/adr/0900 §14): a LandXML file's TIN surface taken into a road
//! project. Its points, breaklines and boundaries become objects on one layer (one undo step “LandXML al”, through
//! `cad.entities.create`'s write), and a terrain surface of them is defined and accepted as `road.terrains.create`
//! would (a project setting). The file's faces are not kept: the surface is made again by `kentos.cdt/1`, and the
//! answer says how far the file's faces and it differ at the faces' centroids. The whole write is one step of the
//! document, all or nothing: a surface that is not made takes nothing in. The shared cases are
//! `fixtures/commands/v1/file.import.landxml.json`.

use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, ImportLandxml, LandxmlImported,
    NewObject, SurfaceSources, TerrainSurfaceDef, Vec2,
};
use kentos_formats::landxml::{self, LxSurface};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as common, fail};

/// What the checks read: the file's surface and its unit, the terrain's name, the warnings so far.
struct Read {
    surface: LxSurface,
    unit: String,
    name: String,
    warnings: Vec<CommandWarning>,
}

fn left(message: String) -> CommandWarning {
    CommandWarning {
        code: codes::LANDXML_LEFT.into(),
        message,
        path: Some("path".into()),
    }
}

fn check(doc: &kentos_domain::Document, input: &ImportLandxml) -> Result<Read, Stop> {
    common::terrain_project(doc)?;
    let path = Path::new(&input.path);
    if !path.exists() {
        return Err(fail(codes::FILE_NOT_FOUND, format!("{} yok; dosyanın yolunu denetleyin.", input.path), "path"));
    }
    let bytes = std::fs::read(path).map_err(|e| {
        fail(
            codes::IMPORT_UNREADABLE,
            format!("{} okunamadı: {e}. Dosyanın izinlerini denetleyin.", input.path),
            "path",
        )
    })?;
    let file = landxml::read(&bytes).map_err(|e| {
        fail(codes::IMPORT_UNREADABLE, format!("{} LandXML olarak okunamadı: {}.", input.path, e.words()), "path")
    })?;
    let names: Vec<String> = file.surfaces.iter().map(|s| format!("“{}”", s.name)).collect();
    let chosen = match &input.surface {
        Some(name) => file.surfaces.iter().find(|s| s.name == *name).ok_or_else(|| {
            fail(
                codes::LANDXML_NO_SURFACE,
                format!("“{name}” adlı yüzey dosyada yok; dosyanın yüzeyleri: {}.", names.join(", ")),
                "surface",
            )
        })?,
        None => file
            .surfaces
            .first()
            .ok_or_else(|| fail(codes::LANDXML_NO_SURFACE, "Dosyada yüzey yok.".into(), "path"))?,
    };
    if chosen.points.is_empty() {
        return Err(fail(
            codes::LANDXML_NO_SURFACE,
            format!("“{}” yüzeyinin noktası yok.", chosen.name),
            "surface",
        ));
    }
    let mut warnings = Vec::new();
    if chosen.broken_faces > 0 {
        warnings.push(left(format!("{} üçgen dosyada olmayan bir noktayı anıyor; alınmadı.", chosen.broken_faces)));
    }
    if chosen.twice > 0 {
        warnings.push(left(format!("{} nokta kimliği iki kez yazılmış; ikincisi alınmadı.", chosen.twice)));
    }
    if chosen.boundaries.iter().filter(|b| !b.void).count() > 1 {
        warnings.push(left("Dosyada birden çok dış sınır var; yalnız ilki alındı.".into()));
    }
    warnings.extend(checks::layer(doc, &input.layer_id)?);
    let name = input
        .name
        .clone()
        .unwrap_or_else(|| if chosen.name.trim().is_empty() { "LandXML".to_owned() } else { chosen.name.trim().to_owned() });
    // The name judged as the surface will stand.
    let mut list = common::surfaces(doc).to_vec();
    list.push(TerrainSurfaceDef {
        id: free_id(&list),
        name: name.clone(),
        points: SurfaceSources {
            layers: vec![],
            uids: vec!["x".into()],
        },
        breaklines: SurfaceSources::default(),
        boundary: None,
        voids: vec![],
        tolerance: None,
        max_edge: None,
        built: None,
        clouds: Vec::new(),
        excluded: Vec::new(),
        edits: Vec::new(),
    });
    common::judged(&list, list.len() - 1, "")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Read {
        surface: chosen.clone(),
        unit: file.unit,
        name,
        warnings,
    })
}

fn free_id(list: &[TerrainSurfaceDef]) -> String {
    (1..)
        .map(|n| format!("t-{n}"))
        .find(|id| list.iter().all(|s| &s.id != id))
        .unwrap_or_default()
}

fn object(geometry: EntityGeometry) -> NewObject {
    NewObject {
        geometry,
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
}

fn v(p: [f64; 2]) -> Vec2 {
    Vec2 { x: p[0], y: p[1] }
}

/// The objects: the points, the breaklines, the outer boundary (the first), the voids; and how many of each.
fn objects(s: &LxSurface) -> (Vec<NewObject>, usize, usize, bool, usize) {
    let mut out: Vec<NewObject> = s
        .points
        .iter()
        .map(|p| object(EntityGeometry::Point { p: v([p.x, p.y]), z: Some(p.z), parts: None }))
        .collect();
    for line in &s.breaklines {
        out.push(object(EntityGeometry::Polyline {
            pts: line.iter().map(|p| v([p[0], p[1]])).collect(),
            bulges: None,
            zs: Some(line.iter().map(|p| Some(p[2])).collect()),
            ms: None,
            parts: None,
            closed: false,
            widths: None,
        }));
    }
    let area = |pts: &[[f64; 2]]| {
        object(EntityGeometry::Polygon {
            pts: pts.iter().map(|p| v(*p)).collect(),
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        })
    };
    let outer = s.boundaries.iter().find(|b| !b.void);
    if let Some(b) = outer {
        out.push(area(&b.points));
    }
    let voids: Vec<_> = s.boundaries.iter().filter(|b| b.void).collect();
    for b in &voids {
        out.push(area(&b.points));
    }
    (out, s.points.len(), s.breaklines.len(), outer.is_some(), voids.len())
}

fn write(cx: &mut ExecutionContext<'_>, input: &ImportLandxml, read: Read) -> CommandResult<LandxmlImported> {
    let (objects, points, lines, outer, voids) = objects(&read.surface);
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects,
        operation: Some(CreateOperation::LandXml),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    let uids = created.created.clone();
    let take = |from: usize, n: usize| uids.get(from..from + n).unwrap_or_default().to_vec();
    let mut list = common::surfaces(cx.doc).to_vec();
    let mut def = TerrainSurfaceDef {
        id: free_id(&list),
        name: read.name.clone(),
        points: SurfaceSources {
            layers: vec![],
            uids: take(0, points),
        },
        breaklines: SurfaceSources {
            layers: vec![],
            uids: take(points, lines),
        },
        boundary: outer.then(|| take(points + lines, 1)).and_then(|u| u.into_iter().next()),
        voids: take(points + lines + usize::from(outer), voids),
        tolerance: None,
        max_edge: None,
        built: None,
        clouds: Vec::new(),
        excluded: Vec::new(),
        edits: Vec::new(),
    };
    let made = common::make(cx.doc, &def);
    let surface = match made.built() {
        Ok(s) => s,
        Err(stop) => return stop.into(),
    };
    // The file's faces against the surface made again, at their centroids.
    let s = &read.surface;
    let kept: Vec<_> = s.faces.iter().filter(|f| !f.ignored).collect();
    let differs = kept
        .iter()
        .filter_map(|f| {
            let [a, b, c] = f.points.map(|i| &s.points[i]);
            let (x, y, z) = ((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0, (a.z + b.z + c.z) / 3.0);
            surface.height(x, y).z.map(|h| (h - z).abs())
        })
        .reduce(f64::max);
    def.built = Some(made.gathered.digest.clone());
    let terrain = made.report(&def);
    list.push(def);
    let doc = &mut *cx.doc;
    doc.set_settings(common::with_surfaces(doc, list));
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    CommandResult::Completed {
        output: LandxmlImported {
            surface: s.name.clone(),
            unit: read.unit,
            points: count(points),
            breaklines: count(lines),
            boundary: outer,
            voids: count(voids),
            faces: count(kept.len()),
            ignored: count(s.faces.len() - kept.len()),
            differs,
            terrain,
            uids,
            revision: doc.revision().to_string(),
        },
        warnings: read.warnings,
    }
}

/// Checks `input`, reading the file.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportLandxml) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(r) => CommandResult::Completed {
            output: (),
            warnings: r.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would take in, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportLandxml) -> CommandResult<LandxmlImported> {
    let read = match check(cx.doc, input) {
        Ok(r) => r,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), input, read);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.uids.clear();
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Reads the file and takes it in: one step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportLandxml) -> CommandResult<LandxmlImported> {
    crate::call::recorded(
        cx,
        kentos_commands::FILE_IMPORT_LANDXML,
        kentos_commands::FILE_IMPORT_LANDXML_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ImportLandxml) -> CommandResult<LandxmlImported> {
    match check(cx.doc, &input) {
        Ok(read) => write(cx, &input, read),
        Err(stop) => stop.into(),
    }
}
