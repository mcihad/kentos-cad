//! `cad.annotations.write` v1 (docs/adr/0622 §9): Koordinat yaz, Köşelere koordinat yaz and Km yaz laid out with the
//! shared core (`ops::coordinate_labels`, `ops::stationing`) and written as `cad.entities.create` writes them, one
//! undo step named after the tool. The shared cases are `fixtures/commands/v1/cad.annotations.write.json`
//! (`scripts/fixtures/annotations_write_cases.py`).

use std::collections::BTreeMap;

use kentos_commands::{
    AnnotationKind, AnnotationWork, AnnotationsWrite, AnnotationsWritten, CommandResult,
    CreateOperation, EntitiesCreate, EntitiesCreatePlan, Entity, EntityGeometry, LabelLean,
    NewObject, StationText, TextAlign, TextFace, Vec2 as Wire,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::ops::coordinate_labels::{
    self as coords, Direction, LabelUnits, Labels, Options, Place,
};
use kentos_geometry_core::ops::lrs;
use kentos_geometry_core::ops::stationing::{self as stationing, Look, Rules, Side};
use kentos_geometry_core::tools::point_calc::route_of;
use kentos_geometry_core::vec2::Vec2;
use kentos_project::format::{Axes, Format};

use crate::checks::{self, Stop, error, is_uid_text};
use crate::elevation::elevation_at;
use crate::geometry::{drawing_font, route, shape};
use crate::table_schedule::listed;
use crate::{ExecutionContext, codes, create};

/// The tick's half length, paper mm (Km yaz's İşaret).
pub const TICK_MM: f64 = 2.0;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn w(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

fn object_at<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<&'d Entity, Stop> {
    if !is_uid_text(uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            path,
        ));
    }
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| {
            refuse(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                path,
            )
        })
}

/// The template a project's type starts with: east over north, as Netcad writes them.
pub fn first_template(cad: bool) -> &'static str {
    if cad { "{X}|{Y}" } else { "{Y}|{X}" }
}

/// The name a point object gives the place `at` (docs/adr/0185 §2): its label, the k-th of its other points
/// “label (k+1)”; none elsewhere.
pub fn name_at(e: &Entity, at: Vec2) -> Option<String> {
    let Entity::Point(p) = e else {
        return None;
    };
    let label = p.base.label.as_deref().map(|l| l.trim_matches(' '))?;
    if label.is_empty() {
        return None;
    }
    let near = |q: &kentos_commands::Vec2| (q.x - at.x).abs() <= 1e-6 && (q.y - at.y).abs() <= 1e-6;
    if near(&p.p) {
        return Some(label.to_owned());
    }
    let k = p.parts.iter().flatten().position(|q| near(&q.p))?;
    Some(format!("{label} ({})", k + 2))
}

/// The place `p` with the elevation and the name the visible objects there have (1 µm, in the document's order):
/// the first elevation, the first point's name.
pub fn place_at(doc: &Document, p: Vec2) -> Place {
    let layers = doc.layers();
    let (mut z, mut name) = (None, None);
    for e in doc.entities() {
        if !layers.is_visible(&e.base().layer_id) {
            continue;
        }
        if z.is_none() {
            z = elevation_at(e, p);
        }
        if name.is_none() {
            name = name_at(e, p);
        }
        if z.is_some() && name.is_some() {
            break;
        }
    }
    Place {
        p,
        z,
        name,
        centre: None,
    }
}

/// The height in the drawing of a text of `kind`: the paper mm given or the project's, times the scale.
fn height_of(doc: &Document, input: &AnnotationsWrite, kind: AnnotationKind) -> Result<f64, Stop> {
    let settings = doc.settings();
    let mm = match input.height {
        Some(h) => {
            checks::finite(h, "Yükseklik", "Sonlu bir sayı verin.", "height")?;
            if h <= 0.0 {
                return Err(refuse(
                    codes::INVALID_HEIGHT,
                    "Yazı yüksekliği sıfırdan büyük olmalı.",
                    "height",
                ));
            }
            h
        }
        None => settings.annotation_mm(kind),
    };
    Ok(mm / 1000.0 * settings.plot_scale)
}

fn width_factor(input: &AnnotationsWrite) -> Result<Option<f64>, Stop> {
    match input.width_factor {
        Some(f) => {
            checks::finite(
                f,
                "Genişlik çarpanı",
                "Sonlu bir sayı verin.",
                "widthFactor",
            )?;
            if f <= 0.0 {
                return Err(refuse(
                    codes::INVALID_WIDTH_FACTOR,
                    "Genişlik çarpanı sıfırdan büyük olmalı.",
                    "widthFactor",
                ));
            }
            Ok((f != 1.0).then_some(f))
        }
        None => Ok(None),
    }
}

fn text(
    p: Vec2,
    words: String,
    height: f64,
    rotation: f64,
    align: Option<kentos_geometry_core::text::TextAlign>,
    face: &TextFace,
    width_factor: Option<f64>,
) -> EntityGeometry {
    EntityGeometry::Text {
        layout: None,
        annotative: None,
        p: w(p),
        text: words,
        height,
        rotation,
        align: align.and_then(|a| TextAlign::from_name(a.name())),
        width_factor,
        mask: false,
        box_width: None,
        line_spacing: None,
        runs: Vec::new(),
        face: face.clone(),
        path: None,
    }
}

fn line(a: Vec2, b: Vec2) -> EntityGeometry {
    EntityGeometry::Line {
        ms: None,
        a: w(a),
        b: w(b),
        zs: None,
    }
}

/// What is written and how it is said.
struct Laid {
    objects: Vec<(EntityGeometry, Option<BTreeMap<String, String>>)>,
    operation: CreateOperation,
    count: usize,
    skipped: usize,
}

#[allow(clippy::too_many_arguments)]
fn coordinates_of(
    doc: &Document,
    at: Option<&[Wire]>,
    ids: Option<&[String]>,
    template: Option<&str>,
    decimals: Option<u8>,
    leader: bool,
    lean: LabelLean,
    input: &AnnotationsWrite,
) -> Result<Laid, Stop> {
    let settings = doc.settings();
    let f = Format::of(settings);
    let places: Vec<Place> = match (at, ids) {
        (Some(at), None) => {
            if at.is_empty() {
                return Err(refuse(
                    codes::TOO_FEW_POINTS,
                    "Koordinatı yazılacak en az bir yer verin.",
                    "work.at",
                ));
            }
            let mut out = Vec::new();
            for (i, p) in at.iter().enumerate() {
                checks::point(*p, "Yerin", &format!("work.at[{i}]"))?;
                out.push(place_at(doc, Vec2::new(p.x, p.y)));
            }
            out
        }
        (None, Some(ids)) => {
            let mut objects = Vec::new();
            for (i, uid) in ids.iter().enumerate() {
                objects.push(listed(object_at(doc, uid, &format!("work.ids[{i}]"))?));
            }
            let places = coords::places(&objects);
            if places.is_empty() {
                return Err(refuse(
                    codes::NOT_A_PATH,
                    "Köşelere koordinat yaz: seçimde nokta, çizgi, çoklu çizgi ya da alan yok.",
                    "work.ids",
                ));
            }
            places
        }
        _ => {
            return Err(refuse(
                codes::INVALID_INPUT,
                "Koordinatın yerlerini (at) ya da köşeleri yazılacak nesneleri (ids) verin; ikisi birden değil.",
                "work.at",
            ));
        }
    };
    let face = input.face.clone().unwrap_or_default();
    let width = width_factor(input)?;
    let height = height_of(doc, input, AnnotationKind::Coordinate)?;
    let template = template.filter(|t| !t.trim().is_empty()).map_or_else(
        || first_template(f.axes == Axes::Cad).to_owned(),
        str::to_owned,
    );
    let options = Options {
        template,
        decimals: decimals.map_or(f.length_decimals, usize::from),
        height,
        leader,
        direction: match lean {
            LabelLean::Auto => Direction::Auto,
            LabelLean::NorthEast => Direction::NorthEast,
            LabelLean::NorthWest => Direction::NorthWest,
            LabelLean::SouthWest => Direction::SouthWest,
            LabelLean::SouthEast => Direction::SouthEast,
        },
        font: drawing_font(face.font.or(settings.drawing_font)),
        bold: face.bold,
        width_factor: width.unwrap_or(1.0),
    };
    let units = LabelUnits {
        cad: f.axes == Axes::Cad,
        per_metre: f.unit.per_metre(),
    };
    let labels: Labels = coords::labels(&places, &options, &units);
    if labels.labels.is_empty() {
        return Err(refuse(
            codes::NOTHING_DERIVED,
            "Koordinat yaz: şablon bu yerlere satır bırakmadı (adı ya da kotu yok).",
            "work.template",
        ));
    }
    let mut objects = Vec::new();
    for l in &labels.labels {
        if let Some(leader) = &l.leader {
            objects.push((
                EntityGeometry::Polyline {
                    ms: None,
                    pts: leader.iter().copied().map(w).collect(),
                    bulges: None,
                    zs: None,
                    parts: None,
                    closed: false,
                    widths: None,
                },
                None,
            ));
        }
        for t in &l.texts {
            objects.push((
                text(t.p, t.text.clone(), height, 0.0, t.align, &face, width),
                None,
            ));
        }
    }
    Ok(Laid {
        objects,
        operation: CreateOperation::Coordinates,
        count: labels.labels.len(),
        skipped: labels.skipped,
    })
}

#[allow(clippy::too_many_arguments)]
fn stations_of(
    doc: &Document,
    id: &str,
    interval: f64,
    start: f64,
    reverse: bool,
    measures: bool,
    ends: bool,
    words: StationText,
    tick: bool,
    section: f64,
    point: Option<f64>,
    input: &AnnotationsWrite,
) -> Result<Laid, Stop> {
    let e = object_at(doc, id, "work.id")?;
    for (v, what, path) in [
        (interval, "Aralık", "work.interval"),
        (start, "Başlangıç", "work.start"),
        (section, "Enkesit", "work.section"),
    ] {
        checks::finite(v, what, "Sonlu bir sayı verin.", path)?;
    }
    if let Some(p) = point {
        checks::finite(p, "Nokta", "Sonlu bir sayı verin.", "work.point")?;
    }
    let s = shape(e);
    if route_of(&s).is_none() {
        return Err(refuse(codes::NO_ROUTE, stationing::NO_ROUTE, "work.id"));
    }
    let settings = doc.settings();
    let scale = settings.plot_scale / 1000.0;
    let height = height_of(doc, input, AnnotationKind::Station)?;
    let face = input.face.clone().unwrap_or_default();
    let width = width_factor(input)?;
    let rules = Rules {
        interval,
        start,
        reverse,
        ends,
        decimals: Format::of(settings).length_decimals,
    };
    let look = Look {
        text: match words {
            StationText::Left => Some(Side::Left),
            StationText::Right => Some(Side::Right),
            StationText::None => None,
        },
        height,
        tick: if tick { TICK_MM * scale } else { 0.0 },
        section,
        point,
    };
    let measured = route(e).filter(|r| measures && lrs::measure_range(r).is_some());
    let plan = match &measured {
        Some(m) => stationing::measure_stationing(m, &rules, &look),
        None => stationing::stationing(&s, &rules, &look),
    }
    .map_err(|why| {
        let code = if why == stationing::BAD_INTERVAL {
            codes::INVALID_DISTANCE
        } else {
            codes::TOO_MANY_POINTS
        };
        refuse(code, why, "work.interval")
    })?;
    let km = |t: &str| Some(BTreeMap::from([("Km".to_owned(), t.to_owned())]));
    let mut objects = Vec::new();
    for t in &plan.ticks {
        objects.push((line(t.a, t.b), None));
    }
    for t in &plan.texts {
        objects.push((
            text(
                t.p,
                t.text.clone(),
                height,
                t.rotation,
                t.align,
                &face,
                width,
            ),
            None,
        ));
    }
    for sct in &plan.sections {
        objects.push((line(sct.a, sct.b), km(&sct.km)));
    }
    for p in &plan.points {
        objects.push((
            EntityGeometry::Point {
                p: w(p.p),
                z: None,
                parts: None,
            },
            km(&p.km),
        ));
    }
    if objects.is_empty() {
        return Err(refuse(
            codes::NOTHING_DERIVED,
            "Km yaz: yazılacak bir şey yok; Yazı, İşaret, Enkesit ya da Nokta’yı açın.",
            "work.text",
        ));
    }
    Ok(Laid {
        objects,
        operation: CreateOperation::Stations,
        count: plan.stations.len(),
        skipped: 0,
    })
}

/// Eğri boyunca yazı: the text on the curve's piece where the click puts it (docs/adr/0196 §4).
fn along_of(
    doc: &Document,
    id: &str,
    at: Wire,
    words: &str,
    align: Option<TextAlign>,
    input: &AnnotationsWrite,
) -> Result<Laid, Stop> {
    let e = object_at(doc, id, "work.id")?;
    checks::point(at, "Tıklanan yerin", "work.at")?;
    if !crate::text_curves::CURVE_KINDS.contains(&e.kind()) {
        return Err(refuse(
            codes::NOT_A_PATH,
            "Yazının izleyeceği nesne çizgi, yay, daire, alan ya da eğri olmalı.",
            "work.id",
        ));
    }
    let words = words.trim();
    if words.is_empty() {
        return Err(refuse(
            codes::EMPTY_TEXT,
            "Eğri boyunca yazılacak yazıyı verin; yazı boş olamaz.",
            "work.text",
        ));
    }
    let height = height_of(doc, input, AnnotationKind::Text)?;
    let factor = width_factor(input)?;
    let align = align.unwrap_or(TextAlign::BottomCenter);
    let font = drawing_font(doc.settings().drawing_font);
    let face = input.face.clone().unwrap_or_default();
    let (p, rotation, path) = crate::text_curves::placed(
        words,
        height,
        factor.unwrap_or(1.0),
        &face,
        &shape(e),
        Vec2::new(at.x, at.y),
        align.along(),
        font,
    )
    .ok_or_else(|| {
        refuse(
            codes::NOT_A_PATH,
            "Yazının izleyeceği nesnenin yolu yok.",
            "work.id",
        )
    })?;
    let geometry = EntityGeometry::Text {
        layout: None,
        annotative: None,
        p,
        text: words.to_owned(),
        height,
        rotation,
        align: Some(align),
        width_factor: factor,
        mask: false,
        box_width: None,
        line_spacing: None,
        runs: Vec::new(),
        face,
        path: Some(path),
    };
    Ok(Laid {
        objects: vec![(geometry, None)],
        operation: CreateOperation::TextAlong,
        count: 1,
        skipped: 0,
    })
}

fn lay(doc: &Document, input: &AnnotationsWrite) -> Result<Laid, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    match &input.work {
        AnnotationWork::Coordinates {
            at,
            ids,
            template,
            decimals,
            leader,
            lean,
        } => coordinates_of(
            doc,
            at.as_deref(),
            ids.as_deref(),
            template.as_deref(),
            *decimals,
            *leader,
            *lean,
            input,
        ),
        AnnotationWork::Stations {
            id,
            interval,
            start,
            reverse,
            measures,
            ends,
            text,
            tick,
            section,
            point,
        } => stations_of(
            doc, id, *interval, *start, *reverse, *measures, *ends, *text, *tick, *section, *point,
            input,
        ),
        AnnotationWork::Along {
            id,
            at,
            text,
            align,
        } => along_of(doc, id, *at, text, *align, input),
    }
}

fn create_input(laid: &Laid, input: &AnnotationsWrite) -> EntitiesCreate {
    EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: laid
            .objects
            .iter()
            .map(|(geometry, attrs)| NewObject {
                line_type: None,
                line_weight: None,
                geometry: geometry.clone(),
                color: input.color.clone(),
                attrs: attrs.clone(),
                label: None,
                label_of: None,
                label_scale: None,
                symbol: None,
                order: None,
                transparency: None,
            })
            .collect(),
        operation: Some(laid.operation),
        expected_revision: input.expected_revision.clone(),
    }
}

/// The work's checks and `cad.entities.create`'s.
pub fn validate(cx: &ExecutionContext<'_>, input: &AnnotationsWrite) -> CommandResult<()> {
    match lay(cx.doc, input) {
        Ok(laid) => create::validate(cx, &create_input(&laid, input)),
        Err(stop) => stop.into(),
    }
}

/// The objects as they would be written.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &AnnotationsWrite,
) -> CommandResult<EntitiesCreatePlan> {
    match lay(cx.doc, input) {
        Ok(laid) => create::plan(cx, &create_input(&laid, input)),
        Err(stop) => stop.into(),
    }
}

/// Writes the annotations, one undo step named after the tool.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: AnnotationsWrite,
) -> CommandResult<AnnotationsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ANNOTATIONS_WRITE,
        kentos_commands::CAD_ANNOTATIONS_WRITE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: AnnotationsWrite,
) -> CommandResult<AnnotationsWritten> {
    let laid = match lay(cx.doc, &input) {
        Ok(laid) => laid,
        Err(stop) => return stop.into(),
    };
    match create::write_in(cx, create_input(&laid, &input)) {
        CommandResult::Completed { output, warnings } => {
            let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
            CommandResult::Completed {
                output: AnnotationsWritten {
                    ids: output.ids,
                    uids: output.created,
                    count: count(laid.count),
                    skipped: count(laid.skipped),
                    revision: output.revision,
                },
                warnings,
            }
        }
        other => crate::trim::relay(other),
    }
}
