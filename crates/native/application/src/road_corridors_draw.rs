//! `road.corridors.draw` v1 (plan/road.md Y7, docs/adr/0905 §6): a corridor's slope plan (Netcad's şevli kotlu plan)
//! drawn on a layer, one undo step “Şevli plan” (through `cad.entities.create`'s write): its feature lines but the axis
//! as polylines carrying their elevations; on each side solved at two consecutive stations the slope's ticks at every
//! multiple of `tick` metres of station between them, from the top of the slope (the daylight in a cut, the hinge in a
//! fill, as the nearer station's side is) toward its foot, the even multiples whole and the odd ones half; the axis
//! elevation (red, two decimals) at each station solved, along the road and readable. The shared cases are
//! `fixtures/commands/v1/road.corridors.draw.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CorridorsDraw, CorridorsDrawn, CreateOperation, EntitiesCreate, EntityGeometry,
    NewObject, TextAlign, TextFace, Vec2,
};
use kentos_road::corridor::{RegionCrosses, place};
use kentos_road::section::template::{Kind, SideSection};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_corridors as common;
use crate::road_terrains::fail;

const RED: &str = "#d62828";

struct Checked {
    id: String,
    tick: f64,
    height: f64,
    lines: Vec<Vec<[f64; 3]>>,
    regions: Vec<RegionCrosses>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &kentos_domain::Document, input: &CorridorsDraw) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_corridors(), &input.corridor, "corridor")?;
    let tick = input.tick.unwrap_or(5.0);
    let height = input.height.unwrap_or(2.5);
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !positive(tick) {
        return Err(fail(codes::INVALID_LABEL, "Şev taramasının aralığı (tick) 0'dan büyük olmalı.".into(), "tick"));
    }
    if !positive(height) {
        return Err(fail(codes::INVALID_LABEL, "Yazı yüksekliği (height) 0'dan büyük olmalı.".into(), "height"));
    }
    let made = common::make(doc, def, true)?;
    if made.regions.iter().all(|r| r.crosses.iter().all(|c| c.section.is_none())) {
        return Err(fail(
            codes::NO_TERRAIN,
            format!("“{}” koridorunda hiçbir kesit çözülmedi: yüzeyi ve boykesiti denetleyin.", def.name),
            "corridor",
        ));
    }
    let lines = made
        .report
        .features
        .iter()
        .filter(|f| f.code != "axis" && f.points.len() >= 2)
        .map(|f| f.points.iter().map(|p| [p[1], p[2], p[3]]).collect())
        .collect();
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    warnings.extend(made.warnings.into_iter().filter(|w| w.code != codes::CORRIDOR_SURFACE_GONE));
    Ok(Checked { id: def.id.clone(), tick, height, lines, regions: made.regions, warnings })
}

fn object(geometry: EntityGeometry, color: Option<&str>, weight: Option<f64>) -> NewObject {
    NewObject {
        geometry,
        color: color.map(str::to_owned),
        line_weight: weight,
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

/// The ticks of one side between two crosses' sides `a` and `b` (stations `s0`, `s1`, plan places by `pa`, `pb`).
fn ticks(
    sides: [&SideSection; 2],
    places: [&dyn Fn(f64) -> [f64; 2]; 2],
    stations: [f64; 2],
    tick: f64,
    out: &mut Vec<[[f64; 2]; 2]>,
) {
    let [a, b] = sides;
    let (Some(da), Some(db)) = (a.daylight, b.daylight) else { return };
    if a.kind == Kind::Open || b.kind == Kind::Open {
        return;
    }
    let [s0, s1] = stations;
    let (ha, hb) = (places[0](a.hinge[0]), places[1](b.hinge[0]));
    let (ya, yb) = (places[0](da[0]), places[1](db[0]));
    let mut k = (s0 / tick - 1e-9).ceil();
    while k * tick < s1 - 1e-9 {
        let t = (k * tick - s0) / (s1 - s0);
        let lerp = |p: [f64; 2], q: [f64; 2]| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
        let (h, d) = (lerp(ha, hb), lerp(ya, yb));
        let cut = if t < 0.5 { a.kind == Kind::Cut } else { b.kind == Kind::Cut };
        let (top, foot) = if cut { (d, h) } else { (h, d) };
        let share = if (k as i64).rem_euclid(2) == 0 { 1.0 } else { 0.5 };
        out.push([top, [top[0] + (foot[0] - top[0]) * share, top[1] + (foot[1] - top[1]) * share]]);
        k += 1.0;
    }
}

fn objects(c: &Checked, h: f64) -> (Vec<NewObject>, u32, u32) {
    let mut out: Vec<NewObject> = c
        .lines
        .iter()
        .map(|pts| {
            object(
                EntityGeometry::Polyline {
                    pts: pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
                    bulges: None,
                    zs: Some(pts.iter().map(|p| Some(p[2])).collect()),
                    ms: None,
                    parts: None,
                    closed: false,
                    widths: None,
                },
                None,
                Some(0.25),
            )
        })
        .collect();
    let mut marks: Vec<[[f64; 2]; 2]> = Vec::new();
    for region in &c.regions {
        for w in region.crosses.windows(2) {
            let (Some(sa), Some(sb)) = (&w[0].section, &w[1].section) else { continue };
            let pa = |o: f64| place(&w[0].line, o);
            let pb = |o: f64| place(&w[1].line, o);
            let stations = [w[0].line.station, w[1].line.station];
            ticks([&sa.left, &sb.left], [&pa, &pb], stations, c.tick, &mut marks);
            ticks([&sa.right, &sb.right], [&pa, &pb], stations, c.tick, &mut marks);
        }
    }
    let ticks = marks.len() as u32;
    for [a, b] in marks {
        out.push(object(
            EntityGeometry::Line { a: Vec2 { x: a[0], y: a[1] }, b: Vec2 { x: b[0], y: b[1] }, zs: None, ms: None },
            None,
            Some(0.13),
        ));
    }
    let mut elevations = 0;
    for region in &c.regions {
        for cross in &region.crosses {
            let Some(sec) = &cross.section else { continue };
            let at = place(&cross.line, 0.0);
            // Along the road, turned to read from below or the right.
            let mut deg = cross.line.direction.to_degrees().rem_euclid(360.0);
            if deg > 90.0 && deg <= 270.0 {
                deg -= 180.0;
            } else if deg > 270.0 {
                deg -= 360.0;
            }
            out.push(object(
                EntityGeometry::Text {
                    layout: None,
                    annotative: None,
                    p: Vec2 { x: at[0], y: at[1] },
                    text: format!("{:.2}", sec.axis),
                    height: h,
                    rotation: deg,
                    align: Some(TextAlign::BottomCenter),
                    width_factor: None,
                    mask: false,
                    box_width: None,
                    line_spacing: None,
                    runs: Vec::new(),
                    face: TextFace::default(),
                    path: None,
                },
                Some(RED),
                None,
            ));
            elevations += 1;
        }
    }
    (out, ticks, elevations)
}

fn write(cx: &mut ExecutionContext<'_>, input: &CorridorsDraw, c: Checked) -> CommandResult<CorridorsDrawn> {
    let h = c.height * cx.doc.settings().plot_scale / 1000.0;
    let (objects, ticks, elevations) = objects(&c, h);
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects,
        operation: Some(CreateOperation::SlopePlan),
        expected_revision: None,
    };
    let created = match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, .. } => output,
        other => return crate::trim::relay(other),
    };
    CommandResult::Completed {
        output: CorridorsDrawn {
            corridor: c.id,
            lines: c.lines.len() as u32,
            ticks,
            elevations,
            tick: c.tick,
            uids: created.created,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &CorridorsDraw) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &CorridorsDraw) -> CommandResult<CorridorsDrawn> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), input, c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.uids.clear();
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the slope plan: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CorridorsDraw) -> CommandResult<CorridorsDrawn> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_CORRIDORS_DRAW,
        kentos_commands::ROAD_CORRIDORS_DRAW_VERSION,
        input,
        |cx, input| {
            let r = match check(cx.doc, &input) {
                Ok(c) => write(cx, &input, c),
                Err(stop) => stop.into(),
            };
            crate::road_drawings::kept(cx, kentos_commands::ROAD_CORRIDORS_DRAW, &input, (kentos_commands::RoadSourceKind::Corridor, "corridor"), r, |o| &o.uids)
        },
    )
}
