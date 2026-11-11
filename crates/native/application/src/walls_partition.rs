//! `cad.walls.partition`: explicit source/limits and measured face/axis spacing; straight or arc edges (docs/adr/0734).
use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
    codes, walls_create,
};
use kentos_commands::*;
use kentos_domain::Document;
use kentos_geometry_core::geom::{
    wall,
    wall_partition::{self, Edge, Extent},
};

fn fail(message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(
        codes::INVALID_WALL,
        message.into(),
        Some(path.into()),
    ))
}
/// Delegated appearance/layer errors describe the nested public request.
fn wall_paths<T>(mut result: CommandResult<T>) -> CommandResult<T> {
    let prefix = |path: &mut Option<String>| {
        if let Some(p) = path {
            *p = format!("wall.{p}");
        }
    };
    match &mut result {
        CommandResult::Completed { warnings, .. } => {
            for w in warnings {
                prefix(&mut w.path);
            }
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => prefix(&mut error.path),
        _ => {}
    }
    result
}
fn reference<'a>(doc: &'a Document, uid: &str, path: &str) -> Result<&'a WallEntity, Stop> {
    crate::openings::wall_at(doc, uid, path)
        .map(|(_, wall, _)| wall)
        .map_err(|mut stop| {
            if let Stop::Failed(e) = &mut stop
                && e.code == codes::NOT_A_WALL
            {
                e.message = "Referans nesnesi çizilebilir bir duvar olmalı.".into();
            }
            stop
        })
}
fn edge(doc: &Document, r: &WallEdgeRef, path: &str) -> Result<Edge, Stop> {
    let w = reference(doc, &r.wall, &format!("{path}.wall"))?;
    let count = if w.wall.closed {
        w.wall.pts.len()
    } else {
        w.wall.pts.len().saturating_sub(1)
    };
    if r.edge >= count {
        return Err(fail(
            "Duvar kenarı bulunamadı; kenar indisi sıfırdan başlar.",
            path,
        ));
    }
    // An arc edge partitions concentrically and limits as its arc (docs/adr/0734).
    let bulge = w
        .wall
        .bulges
        .as_ref()
        .and_then(|b| b.get(r.edge))
        .copied()
        .unwrap_or(0.0);
    let a = w.wall.pts[r.edge];
    let b = w.wall.pts[(r.edge + 1) % w.wall.pts.len()];
    Ok(Edge {
        a: kentos_geometry_core::Vec2::new(a.x, a.y),
        b: kentos_geometry_core::Vec2::new(b.x, b.y),
        bulge: if kentos_geometry_core::geom::bulge::is_arc_bulge(bulge) {
            bulge
        } else {
            0.0
        },
        left: wall::left_face(&kentos_formats::walls::spec(&w.wall)),
        width: w.wall.width,
    })
}
fn request(
    doc: &Document,
    input: &WallsPartition,
) -> Result<(WallsCreate, wall_partition::Placement), Stop> {
    checks::revision(doc, input.wall.expected_revision.as_deref()).map_err(|mut stop| {
        if let Stop::Conflict(e) = &mut stop {
            e.path = Some("wall.expectedRevision".into());
        }
        stop
    })?;
    if !input.distance.is_finite() || input.distance <= 0.0 {
        return Err(fail(
            "Uzaklık sıfırdan büyük sonlu bir uzunluk olmalı.",
            "distance",
        ));
    }
    if input.wall.walls.len() != 1 {
        return Err(fail("Bir bölme için tek duvar tanımı verin.", "wall.walls"));
    }
    let mut request = input.wall.clone();
    let new = &mut request.walls[0];
    if !new.pts.is_empty()
        || new.closed
        || new.bulges.is_some()
        || new.offset.is_some_and(|v| v != 0.0)
        || new.justify.is_some_and(|v| v != WallJustify::Center)
    {
        return Err(fail(
            "Bölme yolu komutla hesaplanır; pts boş, kapalı false, hiza eksen ve öteleme sıfır olmalı.",
            "wall.walls[0]",
        ));
    }
    let source = edge(doc, &input.source, "source")?;
    let host = reference(doc, &input.source.wall, "source.wall")?;
    if new.arch.is_none() {
        new.arch = Some(host.wall.arch.clone());
    }
    if new.height.is_none() {
        new.height = host.wall.height;
    }
    if let PartitionExtent::Between { start, end } = &input.extent {
        for (r, path) in [(start, "extent.start"), (end, "extent.end")] {
            let boundary = reference(doc, &r.wall, &format!("{path}.wall"))?;
            if boundary.wall.arch.level != host.wall.arch.level {
                return Err(fail("Kaynak ve sınır duvarları aynı katta olmalı.", path));
            }
        }
    }
    // Validate and resolve the same style and width as ordinary walls before computing face spacing.
    new.pts = vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 1.0, y: 0.0 }];
    let resolved = crate::walls::build(doc, new, "wall.walls[0]", &mut Vec::new())?;
    let extent = match &input.extent {
        PartitionExtent::Source => Extent::Source,
        PartitionExtent::Between { start, end } => Extent::Between(
            edge(doc, start, "extent.start")?,
            edge(doc, end, "extent.end")?,
        ),
        PartitionExtent::Length {
            anchor,
            length,
            direction,
        } => Extent::Length {
            anchor: *anchor,
            length: *length,
            direction: match direction {
                PartitionDirection::Forward => 1,
                PartitionDirection::Backward => -1,
                PartitionDirection::Both => 0,
            },
        },
    };
    let placement = wall_partition::locate(
        source,
        input.side == PartitionSide::Right,
        input.measure == PartitionMeasure::Axis,
        input.distance,
        resolved.width,
        extent,
    )
    .map_err(|m| fail(m, "extent"))?;
    new.pts = placement
        .path
        .iter()
        .map(|p| Vec2 { x: p.x, y: p.y })
        .collect();
    new.bulges = (placement.bulge != 0.0).then(|| vec![placement.bulge]);
    new.justify = Some(WallJustify::Center);
    Ok((request, placement))
}
pub fn plan_in(doc: &Document, input: &WallsPartition) -> CommandResult<WallPartitionPlan> {
    let (request, placement) = match request(doc, input) {
        Ok(v) => v,
        Err(e) => return e.into(),
    };
    match wall_paths(walls_create::plan_in(doc, &request)) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: WallPartitionPlan {
                wall: output,
                source_face: placement.face.map(|p| Vec2 { x: p.x, y: p.y }),
                source_face_bulge: (placement.face_bulge != 0.0).then_some(placement.face_bulge),
                dimension: placement.dimension.map(|p| Vec2 { x: p.x, y: p.y }),
                clear_distance: placement.clear,
                axis_distance: placement.axis,
            },
            warnings,
        },
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::Cancelled => CommandResult::Cancelled,
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &WallsPartition) -> CommandResult<WallPartitionPlan> {
    plan_in(cx.doc, input)
}
pub fn validate(cx: &ExecutionContext<'_>, input: &WallsPartition) -> CommandResult<()> {
    match plan_in(cx.doc, input) {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed {
            output: (),
            warnings,
        },
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::Cancelled => CommandResult::Cancelled,
    }
}
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: WallsPartition,
) -> CommandResult<WallsCreated> {
    crate::call::recorded(
        cx,
        CAD_WALLS_PARTITION,
        CAD_WALLS_PARTITION_VERSION,
        input,
        write,
    )
}
fn write(cx: &mut ExecutionContext<'_>, input: WallsPartition) -> CommandResult<WallsCreated> {
    let (request, placement) = match request(cx.doc, &input) {
        Ok(v) => v,
        Err(e) => return e.into(),
    };
    if input.keep && placement.bulge != 0.0 {
        return Stop::failed(error(
            codes::INVALID_CONSTRAINT,
            "Yay kenardan bölmede bu ölçü henüz korunamaz: bağ düz duvar kenarları arasında kurulur. Bölmeyi \
             korumasız yerleştirin ya da düz bir kenar seçin."
                .into(),
            Some("keep".into()),
        ))
        .into();
    }
    let mut result = wall_paths(walls_create::execute(cx, request));
    // Bu ölçüyü koru (docs/adr/8049 §1): the bond and its parameter in the same step.
    if input.keep
        && let CommandResult::Completed { output, .. } = &mut result
        && let Some(made) = output.uids.first()
    {
        let rule = match kept(&input, made, &placement) {
            Ok(rule) => rule,
            Err(stop) => return stop.into(),
        };
        match crate::constraints_edit::add_bond_given(
            cx.doc,
            rule,
            "Duvar",
            Some(input.distance),
            "keep",
        ) {
            Ok(id) => output.bonds.push(id),
            Err(stop) => return stop.into(),
        }
    }
    result
}

/// Bu ölçüyü koru's bond (docs/adr/8049 §1): from the source's face on the partition's side (net) or its axis to the
/// partition's face towards the source (the one on the source's side of its path) or its axis.
fn kept(
    input: &WallsPartition,
    made: &str,
    placement: &wall_partition::Placement,
) -> Result<kentos_commands::ConstraintRule, Stop> {
    use kentos_commands::{
        CadElement, ConstraintRule, DistanceAxis, ElementRef, EntityId, WallLine,
    };
    let id = |uid: &str| {
        kentos_domain::Uuid::parse_str(uid)
            .ok()
            .map(|u| EntityId(u.into_bytes()))
            .ok_or_else(|| fail("Duvarın kimliği yok.", "source.wall"))
    };
    let axis = input.measure == PartitionMeasure::Axis;
    let source_line = if axis {
        WallLine::Axis
    } else if input.side == PartitionSide::Left {
        WallLine::LeftFace
    } else {
        WallLine::RightFace
    };
    // The source's face is on the partition's left when the face's middle is left of its path.
    let [p, q] = placement.path;
    let [f0, f1] = placement.face;
    let face = kentos_geometry_core::Vec2::new((f0.x + f1.x) / 2.0, (f0.y + f1.y) / 2.0);
    let left = (q.x - p.x) * (face.y - p.y) - (q.y - p.y) * (face.x - p.x) > 0.0;
    let made_line = if axis {
        WallLine::Axis
    } else if left {
        WallLine::LeftFace
    } else {
        WallLine::RightFace
    };
    Ok(ConstraintRule::Distance {
        a: ElementRef {
            uid: id(&input.source.wall)?,
            element: CadElement::Wall {
                edge: input.source.edge as u32,
                line: source_line,
            },
        },
        b: ElementRef {
            uid: id(made)?,
            element: CadElement::Wall {
                edge: 0,
                line: made_line,
            },
        },
        axis: DistanceAxis::Aligned,
        left: false,
        parameter: String::new(),
    })
}
