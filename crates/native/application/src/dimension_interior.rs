//! `cad.dimensions.interior` v1 (docs/adr/0711): İç ölçü zinciri. The faces a cut crosses, in order along it, each
//! consecutive pair a linear dimension along the cut, both ends bound to the faces by the rule `cross`: a wall moved
//! changes the value, a wall lengthened leaves the chain where it is. Written through `cad.entities.create` as one
//! undo step “İç ölçü zinciri”. The core's `ops::interior_dims` finds the faces; the shared cases are
//! `fixtures/commands/v1/cad.dimensions.interior.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, DimensionLook, DimensionRefs, DimensionRule,
    DimensionStyle, DimensionsInterior, EntitiesCreatePlan, EntitiesCreated, Entity,
    EntityGeometry, EntityId, InteriorColumns, InteriorFaces, SubRef, Vec2 as Point,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::jsmath::{PI, cos, sin};
use kentos_geometry_core::ops::interior_dims::{self, Columns, Faces};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::creating;

/// The total row's distance past the chain, paper millimetres (a dimension set's row gap, docs/adr/0695).
pub const TOTAL_GAP_MM: f64 = 8.0;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn v(p: Point) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn wire(p: Vec2) -> Point {
    Point { x: p.x, y: p.y }
}

/// What the cut measures: the dimensions as `cad.entities.create` writes them, and how many curved faces it left out.
pub struct Chain {
    pub dimensions: Vec<EntityGeometry>,
    pub curved: usize,
}

/// The walls and columns the cut may cross: shown (not hidden or frozen), on `level` when one is named, near the cut.
fn sources(doc: &Document, input: &DimensionsInterior) -> (Vec<Slot>, Vec<Shape>) {
    let (a, b) = (input.from, input.to);
    let cut = [a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)];
    let meets = |r: [f64; 4]| r[0] <= cut[2] && cut[0] <= r[2] && r[1] <= cut[3] && cut[1] <= r[3];
    let mut slots = Vec::new();
    let mut shapes = Vec::new();
    for e in doc.entities() {
        let (level, near) = match e {
            Entity::Wall(w) => (
                w.wall.arch.level.as_deref(),
                // A curved wall's arc may bulge past its vertices' box: always looked at.
                w.wall
                    .bulges
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|k| *k != 0.0))
                    || meets(crate::walls::reach_box(&w.wall, 0.0)),
            ),
            Entity::Column(c) => {
                let f = &c.column;
                let r = f.width.hypot(f.depth.unwrap_or(f.width));
                (
                    f.arch.level.as_deref(),
                    meets([f.at.x - r, f.at.y - r, f.at.x + r, f.at.y + r]),
                )
            }
            _ => continue,
        };
        if !near
            || !doc.layers().is_visible(&e.base().layer_id)
            || input.level.as_deref().is_some_and(|l| level != Some(l))
        {
            continue;
        }
        slots.push(Slot(e.base().id));
        shapes.push(crate::geometry::shape(e));
    }
    (slots, shapes)
}

/// The checks of `cad.dimensions.interior` in the contract's order, and the chain it writes.
fn check(doc: &Document, input: &DimensionsInterior) -> Result<Chain, Stop> {
    checks::point(input.from, "Kesen çizginin başının", "from")?;
    checks::point(input.to, "Kesen çizginin sonunun", "to")?;
    if let Some(at) = input.at {
        checks::point(at, "Zincirin yerinin", "at")?;
    }
    if let Some(h) = input.height
        && !(h.is_finite() && h > 0.0)
    {
        return Err(refuse(
            codes::INVALID_HEIGHT,
            "Ölçü yazısının yüksekliği sıfırdan büyük, sonlu bir sayı olmalı.".into(),
            "height",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    if let Some(level) = input.level.as_deref() {
        let levels = doc
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.as_slice())
            .unwrap_or_default();
        if let Some(why) = kentos_commands::level_problem(Some(level), levels) {
            return Err(refuse(codes::LEVEL_NOT_FOUND, why, "level"));
        }
    }
    let (look, style_height): (DimensionLook, f64) =
        crate::dimension_create::style_of(doc, input.dimension_style.as_deref())?;
    let height = input.height.unwrap_or(style_height);
    let (slots, shapes) = sources(doc, input);
    let refs: Vec<&Shape> = shapes.iter().collect();
    let crossed = interior_dims::references(
        &refs,
        v(input.from),
        v(input.to),
        match input.faces.unwrap_or_default() {
            InteriorFaces::Outer => Faces::Outer,
            InteriorFaces::Skins => Faces::Skins,
        },
        match input.columns.unwrap_or_default() {
            InteriorColumns::Ends => Columns::Ends,
            InteriorColumns::Centers => Columns::Centers,
            InteriorColumns::None => Columns::None,
        },
    )
    .map_err(|why| refuse(codes::NO_DIMENSION, why, "to"))?;
    if crossed.refs.len() < 2 {
        return Err(refuse(
            codes::NO_DIMENSION,
            format!(
                "Kesen çizgi ölçülecek {} yüz kesiyor; zincir için en az iki gerekir. Duvarların ya da kolonların içinden geçen bir çizgi çizin.",
                crossed.refs.len()
            ),
            "to",
        ));
    }
    let angle = interior_dims::angle_of(v(input.from), v(input.to));
    let turn = angle * PI / 180.0;
    let n = Vec2::new(-sin(turn), cos(turn));
    let through = input.at.map(v).unwrap_or(v(input.from));
    let across = |p: Vec2| n.x * (through.x - p.x) + n.y * (through.y - p.y);
    let reference = |r: &interior_dims::Reference| -> Option<SubRef> {
        Some(SubRef {
            uid: EntityId(doc.uid(slots[r.source])?.into_bytes()),
            place: kentos_domain::dimension_ties::contract_place(r.place),
            path: Vec::new(),
            broken: None,
        })
    };
    let dimension = |first: &interior_dims::Reference,
                     second: &interior_dims::Reference,
                     offset: f64|
     -> Option<EntityGeometry> {
        Some(EntityGeometry::Dimension {
            annotative: None,
            text_shift: None,
            breaks: Vec::new(),
            a: wire(first.at),
            b: wire(second.at),
            offset,
            height,
            text: None,
            style: Some(DimensionStyle::Linear),
            angle: Some(angle),
            c: Some(input.from),
            mask: input.mask.unwrap_or(false),
            za: None,
            zb: None,
            refs: Some(Box::new(DimensionRefs {
                a: Some(reference(first)?),
                b: Some(reference(second)?),
                c: None,
                rule: Some(DimensionRule::Cross),
            })),
            look: look.clone(),
        })
    };
    let mut dimensions: Vec<EntityGeometry> = crossed
        .refs
        .windows(2)
        .filter_map(|w| dimension(&w[0], &w[1], across(w[0].at)))
        .collect();
    if input.total.unwrap_or(false) && crossed.refs.len() > 2 {
        let side = input.at.map_or(1.0, |at| {
            let d = n.x * (at.x - input.from.x) + n.y * (at.y - input.from.y);
            if d < 0.0 { -1.0 } else { 1.0 }
        });
        let gap = TOTAL_GAP_MM * doc.settings().plot_scale / 1000.0;
        let (first, last) = (&crossed.refs[0], &crossed.refs[crossed.refs.len() - 1]);
        dimensions.extend(dimension(first, last, across(first.at) + side * gap));
    }
    Ok(Chain {
        dimensions,
        curved: crossed.curved,
    })
}

/// The objects `cad.entities.create` writes.
fn objects(input: &DimensionsInterior, chain: &Chain) -> kentos_commands::EntitiesCreate {
    creating::input_as(
        &input.layer_id,
        chain.dimensions.clone(),
        &input.look,
        Some(CreateOperation::InteriorDimensions),
    )
}

/// The warning the curved faces left out carry.
fn curved_warning(chain: &Chain) -> Option<CommandWarning> {
    (chain.curved > 0).then(|| CommandWarning {
        code: codes::ARCH_CURVED_FACE_SKIPPED.into(),
        message: format!(
            "Kesen çizgi {} yaylı duvar yüzünü kesti; iç ölçü zinciri düz yüzleri ölçer, yaylılar atlandı.",
            chain.curved
        ),
        path: Some("to".into()),
    })
}

/// `result` with the curved faces' warning added, when it was written or planned.
fn said<T>(result: CommandResult<T>, warning: Option<CommandWarning>) -> CommandResult<T> {
    match (result, warning) {
        (
            CommandResult::Completed {
                output,
                mut warnings,
            },
            Some(w),
        ) => {
            warnings.push(w);
            CommandResult::Completed { output, warnings }
        }
        (result, _) => result,
    }
}

/// The chain `input` writes in `doc`, for a tool's preview; the refusal when none forms or the input is wrong.
pub fn chain_in(
    doc: &Document,
    input: &DimensionsInterior,
) -> Result<Chain, Box<kentos_commands::CommandError>> {
    check(doc, input).map_err(|stop| Box::new(stop.into_error()))
}

/// `cad.dimensions.interior` v1: İç ölçü zinciri.
pub mod dimensions_interior {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &DimensionsInterior) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(c) => said(
                creating::validate(cx, &objects(input, &c)),
                curved_warning(&c),
            ),
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &DimensionsInterior,
    ) -> CommandResult<EntitiesCreatePlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document.
    pub fn plan_in(
        doc: &Document,
        input: &DimensionsInterior,
    ) -> CommandResult<EntitiesCreatePlan> {
        match check(doc, input) {
            Ok(c) => said(creating::plan(doc, &objects(input, &c)), curved_warning(&c)),
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DimensionsInterior,
    ) -> CommandResult<EntitiesCreated> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_DIMENSIONS_INTERIOR,
            kentos_commands::CAD_DIMENSIONS_INTERIOR_VERSION,
            input,
            |cx, input| match check(cx.doc, &input) {
                Ok(c) => {
                    let warning = curved_warning(&c);
                    said(creating::execute(cx, objects(&input, &c)), warning)
                }
                Err(stop) => stop.into(),
            },
        )
    }
}
