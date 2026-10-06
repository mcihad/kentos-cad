//! Blocks for the formats that have none (docs/adr/0144 §5): an insert as
//! its block's objects placed by the shared core (`kentos_geometry_core::block`:
//! nested blocks opened, quarter turns exact), each back as the contract's
//! object of its kind. Objects cross field for field, as the native
//! application's conversion does (no JSON in between: the formats module is
//! downloaded with every drawing a page opens, CLAUDE.md §20). The core's
//! pieces are flat: a placed piece has no elevation but a point's.

use kentos_contracts::{
    ArcEntity, AreaPart, BlockDefinition, CircleEntity, ConstructionEntity, DimensionEntity,
    DimensionStyle, EllipseEntity, Entity, EntityBase, HatchEntity, HatchPattern, HatchPatternType,
    InsertEntity, LeaderArrow, LeaderEntity, LineEntity, PathEntity, PointEntity, RingGeometry,
    SplineEntity, TableEntity, TextEntity, Vec2,
};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::{Attribute, Blocks, Definition};
use kentos_geometry_core::entity::{
    Attrs, Entity as CoreEntity, HatchPattern as CorePattern, Part, PointPart as CorePoint, Shape,
};
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::dimension::{Arrow, Look};
use kentos_geometry_core::text::Font;
use kentos_geometry_core::text::face::Face;
use kentos_geometry_core::text::paragraph::{Run, Script};

/// A drawing's blocks, flattened once, ready to place inserts.
pub struct Placing(Blocks);

impl Placing {
    pub fn new(blocks: &[BlockDefinition]) -> Placing {
        Self::of(blocks, false)
    }

    /// With the definitions' attribute definitions (docs/adr/0144 §7): the
    /// DXF writer's ATTRIBs (`attribute_texts`).
    pub fn with_attributes(blocks: &[BlockDefinition]) -> Placing {
        Self::of(blocks, true)
    }

    fn of(blocks: &[BlockDefinition], attributes: bool) -> Placing {
        Placing(Blocks::new(
            blocks
                .iter()
                .map(|b| Definition {
                    id: b.id.to_text(),
                    base: core(b.base),
                    entities: b
                        .entities
                        .iter()
                        .map(|e| CoreEntity::new(shape(e)))
                        .collect(),
                    // Attributes are texts, which GeoJSON leaves out of a block.
                    attributes: if attributes {
                        b.attributes
                            .iter()
                            .map(|a| Attribute {
                                tag: a.tag.clone(),
                                value: a.value.clone().unwrap_or_default(),
                                p: core(a.p),
                                height: a.height,
                                rotation: a.rotation,
                                align: a.align.and_then(core_align),
                                width_factor: a.width_factor,
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                })
                .collect(),
        ))
    }

    /// An insert's attribute texts as it shows them (§7): each tag of its
    /// block's attribute definitions, in their order, with its text placed,
    /// the insert's value or else the default (empty when neither).
    pub fn attribute_texts(&self, i: &InsertEntity) -> Vec<(String, TextEntity)> {
        let mut insert = shape(&Entity::Insert(i.clone()));
        if let Shape::Insert { attrs, .. } = &mut insert {
            *attrs = Some(Attrs(
                i.base
                    .attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), Json::Str(v.clone())))
                    .collect(),
            ));
        }
        self.0
            .expand(&insert)
            .into_iter()
            .filter_map(|piece| {
                let tag = piece.attribute?;
                let Shape::Text {
                    p,
                    text,
                    height,
                    rotation,
                    align,
                    width_factor,
                    mask,
                    box_width,
                    line_spacing,
                    runs,
                    face: _,
                } = piece.shape
                else {
                    return None;
                };
                let base = EntityBase {
                    id: 0,
                    layer_id: String::new(),
                    color: None,
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                    line_weight: None,
                };
                Some((
                    tag,
                    TextEntity {
                        base,
                        p: back(p),
                        text,
                        height,
                        rotation,
                        align: align.and_then(contract_align),
                        width_factor,
                        mask: mask == Some(true),
                        label_of: None,
                        label_scale: None,
                        paragraph: kentos_contracts::Paragraph {
                            box_width,
                            line_spacing,
                            runs: contract_runs(runs.as_deref()),
                        },
                        face: Default::default(),
                    },
                ))
            })
            .collect()
    }

    /// An insert's objects placed, in its block's order; none for a block
    /// the drawing does not have. Each has no id, layer or attribute of its
    /// own (the insert's object carries them).
    pub fn placed(&self, i: &InsertEntity) -> Vec<Entity> {
        self.0
            .expand(&shape(&Entity::Insert(i.clone())))
            .iter()
            .filter_map(|piece| entity(&piece.shape))
            .collect()
    }
}

/// A text's alignment as the geometry core names it (docs/adr/0145).
pub(crate) fn core_align(a: kentos_contracts::TextAlign) -> Option<kentos_geometry_core::text::TextAlign> {
    kentos_geometry_core::text::TextAlign::from_name(a.name())
}

/// The core's alignment as the contract names it.
fn contract_align(a: kentos_geometry_core::text::TextAlign) -> Option<kentos_contracts::TextAlign> {
    kentos_contracts::TextAlign::from_name(a.name())
}

/// A multi-line text's runs as the core takes them (docs/adr/0182); none for none.
pub(crate) fn core_runs(runs: &[kentos_contracts::TextRun]) -> Option<Vec<Run>> {
    (!runs.is_empty()).then(|| {
        runs.iter()
            .map(|r| Run {
                start: r.start,
                end: r.end,
                bold: r.bold,
                italic: r.italic,
                underline: r.underline,
                script: r.script.map(|s| match s {
                    kentos_contracts::TextScript::Super => Script::Super,
                    kentos_contracts::TextScript::Sub => Script::Sub,
                }),
                color: r.color.clone(),
            })
            .collect()
    })
}

/// A text's face as the core takes it (docs/adr/0183 §2).
pub(crate) fn core_face(f: &kentos_contracts::TextFace) -> Face {
    Face {
        style: f.text_style.clone(),
        font: f.font.map(|d| Font::from_id(d.id())),
        bold: f.bold,
        italic: f.italic,
        oblique: f.oblique,
    }
}

/// The core's face as the contract writes it.
pub(crate) fn contract_face(f: &Face) -> kentos_contracts::TextFace {
    kentos_contracts::TextFace {
        text_style: f.style.clone(),
        font: f
            .font
            .and_then(|c| kentos_contracts::DrawingFont::from_id(c.id())),
        bold: f.bold,
        italic: f.italic,
        oblique: f.oblique,
    }
}

/// A table as the core takes it (docs/adr/0184 §1).
pub fn core_table(t: &TableEntity) -> Shape {
    Shape::Table {
        p: core(t.p),
        rotation: t.rotation,
        height: t.height,
        rows: t.rows.clone(),
        columns: t.columns.clone(),
        cells: t.cells.clone(),
        merges: (!t.merges.is_empty()).then(|| {
            t.merges
                .iter()
                .map(|m| kentos_geometry_core::entity::CellRange {
                    row: m.row as usize,
                    col: m.col as usize,
                    rows: m.rows as usize,
                    cols: m.cols as usize,
                })
                .collect()
        }),
        aligns: t
            .aligns
            .as_ref()
            .map(|a| a.iter().map(|x| x.name().to_owned()).collect()),
        header: t.header.then_some(true),
        grid: t.grid.map(|g| g.name().to_owned()),
        frame: t.frame,
        // Its corners and lines are what these formats ask of the core.
        source: None,
        face: core_face(&t.face),
    }
}

/// A picture's frame's corners (docs/adr/0192 §1), for what it covers.
pub fn image_corners(i: &kentos_contracts::ImageEntity) -> Vec<Vec2> {
    let f = kentos_geometry_core::geom::image::Frame::new(
        core(i.image.p),
        i.image.width,
        i.image.height,
        i.image.rotation,
        i.image.mirror,
    );
    f.corners().iter().map(|p| back(*p)).collect()
}

/// A table's four corners (docs/adr/0184 §2): top left, top right, bottom
/// right, bottom left.
pub fn table_corners(t: &TableEntity) -> Vec<Vec2> {
    kentos_geometry_core::geom::table::table_geom(&core_table(t))
        .map(|g| g.outline().iter().map(|p| back(*p)).collect())
        .unwrap_or_else(|| vec![t.p])
}

/// A dimension's look as the core takes it (docs/adr/0183 §3).
pub(crate) fn core_look(l: &kentos_contracts::DimensionLook) -> Look {
    Look {
        style: l.dim_style.clone(),
        arrow: l.arrow.and_then(|a| Arrow::from_name(a.name())),
        arrow_size: l.arrow_size,
        ext_offset: l.ext_offset,
        ext_beyond: l.ext_beyond,
        text_gap: l.text_gap,
        centre: l.text_place.is_some(),
        decimals: l.decimals,
        unit: l.unit.map(|u| u.mark().to_owned()),
        prefix: l.prefix.clone(),
        suffix: l.suffix.clone(),
        font: l.font.map(|d| Font::from_id(d.id())),
    }
}

/// The core's look as the contract writes it.
pub(crate) fn contract_look(l: &Look) -> kentos_contracts::DimensionLook {
    use kentos_contracts::DrawingUnit;
    kentos_contracts::DimensionLook {
        dim_style: l.style.clone(),
        arrow: l
            .arrow
            .and_then(|a| kentos_contracts::DimensionArrow::from_name(a.name())),
        arrow_size: l.arrow_size,
        ext_offset: l.ext_offset,
        ext_beyond: l.ext_beyond,
        text_gap: l.text_gap,
        text_place: l
            .centre
            .then_some(kentos_contracts::DimensionTextPlace::Centre),
        decimals: l.decimals,
        unit: match l.unit.as_deref() {
            Some("mm") => Some(DrawingUnit::Mm),
            Some("cm") => Some(DrawingUnit::Cm),
            Some("m") => Some(DrawingUnit::M),
            _ => None,
        },
        prefix: l.prefix.clone(),
        suffix: l.suffix.clone(),
        font: l
            .font
            .and_then(|c| kentos_contracts::DrawingFont::from_id(c.id())),
    }
}

/// The core's runs as the contract writes them.
pub(crate) fn contract_runs(runs: Option<&[Run]>) -> Vec<kentos_contracts::TextRun> {
    runs.unwrap_or_default()
        .iter()
        .map(|r| kentos_contracts::TextRun {
            start: r.start,
            end: r.end,
            bold: r.bold,
            italic: r.italic,
            underline: r.underline,
            script: r.script.map(|s| match s {
                Script::Super => kentos_contracts::TextScript::Super,
                Script::Sub => kentos_contracts::TextScript::Sub,
            }),
            color: r.color.clone(),
        })
        .collect()
}

fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn back(p: CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

fn points(pts: &[Vec2]) -> Vec<CoreVec2> {
    pts.iter().map(|p| core(*p)).collect()
}

fn points_back(pts: &[CoreVec2]) -> Vec<Vec2> {
    pts.iter().map(|p| back(*p)).collect()
}

fn ring(r: &RingGeometry) -> Ring {
    Ring {
        pts: points(&r.pts),
        bulges: r.bulges.clone(),
    }
}

fn ring_back(r: &Ring) -> RingGeometry {
    RingGeometry {
        pts: points_back(&r.pts),
        bulges: r.bulges.clone(),
        zs: None,
    }
}

fn style_name(s: DimensionStyle) -> &'static str {
    s.name()
}

fn style_of(name: &str) -> Option<DimensionStyle> {
    DimensionStyle::from_name(name)
}

fn pattern_name(k: HatchPatternType) -> &'static str {
    match k {
        HatchPatternType::Solid => "solid",
        HatchPatternType::Lines => "lines",
        HatchPatternType::Cross => "cross",
        HatchPatternType::Pattern => "pattern",
        HatchPatternType::Gradient => "gradient",
    }
}

fn pattern_of(name: &str) -> HatchPatternType {
    match name {
        "solid" => HatchPatternType::Solid,
        "cross" => HatchPatternType::Cross,
        "pattern" => HatchPatternType::Pattern,
        "gradient" => HatchPatternType::Gradient,
        _ => HatchPatternType::Lines,
    }
}

/// A hatch's pattern as the core holds it, its families and gradient too
/// (docs/adr/0186): the contract's own JSON, read by the core.
fn core_pattern(p: &HatchPattern) -> CorePattern {
    serde_json::to_string(p)
        .ok()
        .and_then(|t| Json::parse(&t).ok())
        .and_then(|j| <CorePattern as FromJson>::from_json(&j).ok())
        .unwrap_or_else(|| CorePattern::user(pattern_name(p.kind), p.angle, p.spacing))
}

/// The core's pattern in the contract, its families and gradient too.
fn contract_pattern(p: &CorePattern) -> HatchPattern {
    serde_json::from_str(&kentos_geometry_core::api::json::to_string(p))
        .unwrap_or_else(|_| HatchPattern::user(pattern_of(&p.kind), p.angle, p.spacing))
}

/// An object's geometry as the core takes it.
fn shape(e: &Entity) -> Shape {
    let path = |p: &PathEntity| {
        (
            points(&p.pts),
            p.bulges.clone(),
            p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
        )
    };
    match e {
        Entity::Point(p) => Shape::Point {
            p: core(p.p),
            z: p.z,
            // Every point of a multi-point object (docs/adr/0174).
            parts: p.parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|q| CorePoint {
                        p: core(q.p),
                        z: q.z,
                    })
                    .collect()
            }),
        },
        Entity::Line(l) => Shape::Line {
            a: core(l.a),
            b: core(l.b),
        },
        Entity::Polyline(p) => {
            let (pts, bulges, holes) = path(p);
            Shape::Polyline {
                pts,
                bulges,
                holes,
                // Every part of a multi-part polyline (docs/adr/0174).
                parts: p.parts.as_ref().map(|ps| {
                    ps.iter()
                        .map(|part| Part {
                            pts: points(&part.pts),
                            bulges: part.bulges.clone(),
                            holes: None,
                        })
                        .collect()
                }),
            }
        }
        Entity::Polygon(p) => {
            let (pts, bulges, holes) = path(p);
            Shape::Polygon {
                pts,
                bulges,
                holes,
                parts: p.parts.as_ref().map(|ps| {
                    ps.iter()
                        .map(|part| Part {
                            pts: points(&part.pts),
                            bulges: part.bulges.clone(),
                            holes: part.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
                        })
                        .collect()
                }),
            }
        }
        Entity::Circle(c) => Shape::Circle {
            c: core(c.c),
            r: c.r,
        },
        Entity::Leader(l) => Shape::Leader {
            pts: points(&l.pts),
            text: l.text.clone(),
            height: l.height,
            rotation: l.rotation,
            arrow: l.arrow.map(|a| a.name().to_owned()),
            mask: l.mask.then_some(true),
        },
        Entity::Arc(a) => Shape::Arc {
            c: core(a.c),
            r: a.r,
            a0: a.a0,
            a1: a.a1,
        },
        Entity::Ellipse(e) => Shape::Ellipse {
            c: core(e.c),
            major: core(e.major),
            ratio: e.ratio,
            t0: e.t0,
            t1: e.t1,
        },
        Entity::Spline(s) => Shape::Spline {
            pts: points(&s.pts),
            closed: s.closed,
        },
        Entity::Xline(x) => Shape::Xline {
            p: core(x.p),
            dir: core(x.dir),
        },
        Entity::Ray(x) => Shape::Ray {
            p: core(x.p),
            dir: core(x.dir),
        },
        Entity::Text(t) => Shape::Text {
            p: core(t.p),
            text: t.text.clone(),
            height: t.height,
            rotation: t.rotation,
            align: t.align.and_then(core_align),
            width_factor: t.width_factor,
            mask: t.mask.then_some(true),
            box_width: t.paragraph.box_width,
            line_spacing: t.paragraph.line_spacing,
            runs: core_runs(&t.paragraph.runs),
            face: core_face(&t.face),
        },
        Entity::Dimension(d) => Shape::Dimension {
            a: core(d.a),
            b: core(d.b),
            offset: d.offset,
            height: d.height,
            text: d.text.clone(),
            style: d.style.map(|s| style_name(s).to_owned()),
            angle: d.angle,
            c: d.c.map(core),
            mask: d.mask.then_some(true),
            za: d.za,
            zb: d.zb,
            look: core_look(&d.look),
        },
        Entity::Hatch(h) => Shape::Hatch {
            ring: points(&h.ring),
            holes: h
                .holes
                .as_ref()
                .map(|hs| hs.iter().map(|r| points(r)).collect()),
            pattern: core_pattern(&h.pattern),
            // A block's hatch follows nothing (docs/adr/0186 §6).
            assoc: None,
        },
        Entity::Table(t) => core_table(t),
        // A picture is no block's piece (docs/adr/0192 §1); its frame for what it covers.
        Entity::Image(i) => Shape::Image {
            p: core(i.image.p),
            width: i.image.width,
            height: i.image.height,
            rotation: i.image.rotation,
            mirror: i.image.mirror.then_some(true),
            asset: i.image.asset.clone(),
            file: i.image.file.clone(),
            clip: i
                .image
                .clip
                .as_ref()
                .map(|c| c.iter().map(|q| core(*q)).collect()),
            opacity: i.image.opacity,
        },
        // Its attributes show as texts, which these formats leave out of a block.
        Entity::Insert(i) => Shape::Insert {
            block: i.block.to_text(),
            p: core(i.p),
            scale: i.scale,
            rotation: i.rotation,
            mirror: i.mirror.then_some(true),
            attrs: None,
        },
    }
}

/// A placed piece as the contract's object (never an insert: the core opens them).
fn entity(s: &Shape) -> Option<Entity> {
    let base = EntityBase {
        id: 0,
        layer_id: String::new(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
    };
    let path =
        |base, pts: &[CoreVec2], bulges: &Option<Vec<f64>>, holes: &Option<Vec<Ring>>| PathEntity {
            base,
            pts: points_back(pts),
            bulges: bulges.clone(),
            holes: holes.as_ref().map(|hs| hs.iter().map(ring_back).collect()),
            zs: None,
            parts: None,
        };
    Some(match s {
        Shape::Point { p, z, parts } => Entity::Point(PointEntity {
            base,
            p: back(*p),
            z: *z,
            parts: parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|q| kentos_contracts::PointPart {
                        p: back(q.p),
                        z: q.z,
                    })
                    .collect()
            }),
        }),
        Shape::Line { a, b } => Entity::Line(LineEntity {
            base,
            a: back(*a),
            b: back(*b),
            za: None,
            zb: None,
        }),
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
        } => Entity::Polyline(PathEntity {
            parts: parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|part| AreaPart {
                        pts: points_back(&part.pts),
                        bulges: part.bulges.clone(),
                        holes: None,
                        zs: None,
                    })
                    .collect()
            }),
            ..path(base, pts, bulges, holes)
        }),
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => Entity::Polygon(PathEntity {
            parts: parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|part| AreaPart {
                        pts: points_back(&part.pts),
                        bulges: part.bulges.clone(),
                        holes: part
                            .holes
                            .as_ref()
                            .map(|hs| hs.iter().map(ring_back).collect()),
                        zs: None,
                    })
                    .collect()
            }),
            ..path(base, pts, bulges, holes)
        }),
        Shape::Circle { c, r } => Entity::Circle(CircleEntity {
            base,
            c: back(*c),
            r: *r,
        }),
        Shape::Arc { c, r, a0, a1 } => Entity::Arc(ArcEntity {
            base,
            c: back(*c),
            r: *r,
            a0: *a0,
            a1: *a1,
        }),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => Entity::Ellipse(EllipseEntity {
            base,
            c: back(*c),
            major: back(*major),
            ratio: *ratio,
            t0: *t0,
            t1: *t1,
        }),
        Shape::Xline { p, dir } => Entity::Xline(ConstructionEntity {
            base,
            p: back(*p),
            dir: back(*dir),
        }),
        Shape::Ray { p, dir } => Entity::Ray(ConstructionEntity {
            base,
            p: back(*p),
            dir: back(*dir),
        }),
        Shape::Spline { pts, closed } => Entity::Spline(SplineEntity {
            base,
            pts: points_back(pts),
            closed: *closed,
        }),
        Shape::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            mask,
        } => Entity::Leader(LeaderEntity {
            base,
            pts: points_back(pts),
            text: text.clone(),
            height: *height,
            rotation: *rotation,
            arrow: arrow.as_deref().and_then(LeaderArrow::from_name),
            mask: *mask == Some(true),
        }),
        Shape::Text {
            p,
            text,
            height,
            rotation,
            align,
            width_factor,
            mask,
            box_width,
            line_spacing,
            runs,
            face,
        } => Entity::Text(TextEntity {
            base,
            p: back(*p),
            text: text.clone(),
            height: *height,
            rotation: *rotation,
            align: align.and_then(contract_align),
            width_factor: *width_factor,
            mask: *mask == Some(true),
            label_of: None,
            label_scale: None,
            paragraph: kentos_contracts::Paragraph {
                box_width: *box_width,
                line_spacing: *line_spacing,
                runs: contract_runs(runs.as_deref()),
            },
            face: contract_face(face),
        }),
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            text,
            style,
            angle,
            c,
            mask,
            za,
            zb,
            look,
        } => Entity::Dimension(DimensionEntity {
            base,
            a: back(*a),
            b: back(*b),
            offset: *offset,
            height: *height,
            text: text.clone(),
            style: style.as_deref().and_then(style_of),
            angle: *angle,
            c: c.map(back),
            mask: *mask == Some(true),
            za: *za,
            zb: *zb,
            look: contract_look(look),
        }),
        Shape::Hatch {
            ring,
            holes,
            pattern,
            ..
        } => Entity::Hatch(HatchEntity {
            base,
            ring: points_back(ring),
            holes: holes
                .as_ref()
                .map(|hs| hs.iter().map(|r| points_back(r)).collect()),
            pattern: contract_pattern(pattern),
            assoc: None,
        }),
        // A block holds no table (docs/adr/0184 §1); the core opens inserts.
        Shape::Insert { .. } | Shape::Table { .. } | Shape::Image { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::BlockId;

    fn object(json: &str) -> Entity {
        serde_json::from_str(json).expect("an object")
    }

    /// Every kind crosses to the core and back unchanged (a placed piece is flat: no elevations but a point's).
    #[test]
    fn every_kind_crosses_and_comes_back() {
        for json in [
            r#"{"kind":"point","id":0,"layerId":"","attrs":{},"p":{"x":1,"y":2},"z":3.5}"#,
            r#"{"kind":"line","id":0,"layerId":"","attrs":{},"a":{"x":1,"y":2},"b":{"x":3,"y":4}}"#,
            r#"{"kind":"polyline","id":0,"layerId":"","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":0}],"bulges":[0.5,0]}"#,
            r#"{"kind":"polygon","id":0,"layerId":"","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]}],"parts":[{"pts":[{"x":9,"y":0},{"x":10,"y":0},{"x":10,"y":1}],"bulges":[0,0.25,0]}]}"#,
            r#"{"kind":"circle","id":0,"layerId":"","attrs":{},"c":{"x":1,"y":2},"r":3}"#,
            r#"{"kind":"arc","id":0,"layerId":"","attrs":{},"c":{"x":1,"y":2},"r":3,"a0":0.5,"a1":2}"#,
            r#"{"kind":"ellipse","id":0,"layerId":"","attrs":{},"c":{"x":1,"y":2},"major":{"x":3,"y":0},"ratio":0.5,"t0":0,"t1":1}"#,
            r#"{"kind":"spline","id":0,"layerId":"","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":1},{"x":2,"y":0}],"closed":false}"#,
            r#"{"kind":"xline","id":0,"layerId":"","attrs":{},"p":{"x":1,"y":2},"dir":{"x":0,"y":1}}"#,
            r#"{"kind":"ray","id":0,"layerId":"","attrs":{},"p":{"x":1,"y":2},"dir":{"x":1,"y":0}}"#,
            r#"{"kind":"text","id":0,"layerId":"","attrs":{},"p":{"x":1,"y":2},"text":"Ağaç","height":2.5,"rotation":30}"#,
            r#"{"kind":"dimension","id":0,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":5,"y":0},"offset":1,"height":0.5,"text":"5.00","style":"linear","angle":0}"#,
            r#"{"kind":"hatch","id":0,"layerId":"","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"cross","angle":45,"spacing":0.75}}"#,
        ] {
            let e = object(json);
            assert_eq!(entity(&shape(&e)), Some(e), "{json}");
        }
    }

    /// An insert of a block holding a point and a nested block: placed by the
    /// core, mirrored and turned a quarter (exact), the nested one opened.
    #[test]
    fn an_insert_is_its_blocks_objects_placed() {
        let id = |n: u8| BlockId([n; 16]);
        let block = |n: u8, base: (f64, f64), entities: Vec<Entity>| BlockDefinition {
            id: id(n),
            name: format!("B{n}"),
            base: Vec2 {
                x: base.0,
                y: base.1,
            },
            entities,
            attributes: Vec::new(),
            description: None,
        };
        let inner = block(
            2,
            (0.0, 0.0),
            vec![object(
                r#"{"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":1,"y":0}}"#,
            )],
        );
        let outer = block(
            1,
            (1.0, 2.0),
            vec![
                object(r#"{"kind":"point","id":1,"layerId":"","attrs":{},"p":{"x":1,"y":3}}"#),
                Entity::Insert(InsertEntity {
                    base: object(
                        r#"{"kind":"point","id":2,"layerId":"","attrs":{},"p":{"x":0,"y":0}}"#,
                    )
                    .base()
                    .clone(),
                    block: id(2),
                    p: Vec2 { x: 1.0, y: 2.0 },
                    scale: 2.0,
                    rotation: 0.0,
                    mirror: false,
                }),
            ],
        );
        let placing = Placing::new(&[outer, inner]);
        let i = InsertEntity {
            base: object(r#"{"kind":"point","id":7,"layerId":"a","attrs":{},"p":{"x":0,"y":0}}"#)
                .base()
                .clone(),
            block: id(1),
            p: Vec2 { x: 100.0, y: 200.0 },
            scale: 3.0,
            rotation: std::f64::consts::FRAC_PI_2,
            mirror: true,
        };
        // (1, 3) is (0, 1) from the base: mirrored (0, −1), scaled (0, −3), a quarter turn (3, 0).
        // The nested line (0, 0)–(2, 0) from the base: mirrored and scaled (0, 0)–(6, 0), turned (0, 0)–(0, 6).
        let got = placing.placed(&i);
        assert_eq!(
            got,
            [
                object(r#"{"kind":"point","id":0,"layerId":"","attrs":{},"p":{"x":103,"y":200}}"#),
                object(
                    r#"{"kind":"line","id":0,"layerId":"","attrs":{},"a":{"x":100,"y":200},"b":{"x":100,"y":206}}"#
                ),
            ]
        );
        let unknown = InsertEntity { block: id(9), ..i };
        assert!(placing.placed(&unknown).is_empty());
    }
}
