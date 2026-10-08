//! Objects as the shared geometry core takes them, and back
//! (docs/adr/0029, 0037): the contract's [`Entity`] to the core's [`Shape`],
//! and a shape the core computed for an object back into that object. The
//! desktop's geometry store (`kentos_interaction::spatial`) and the
//! transform command read objects through here. Nothing is computed: each
//! field is carried over as it is, float64 bit for bit.

use kentos_contracts::{
    ArcEntity, AreaPart, BlockId, CircleEntity, ConstructionEntity, DimensionEntity,
    DimensionStyle, DrawingFont, EllipseEntity, Entity, EntityBase, EntityGeometry, EntityId,
    GradientShape, HatchAssoc, HatchEntity, HatchGradient, HatchPattern as ContractPattern,
    HatchPatternType, ImageEntity, ImageFields, InsertEntity, LeaderArrow, LeaderEntity,
    LineEntity, PathEntity, PatternLine, PointEntity, RasterEntity, RasterFields, RasterSample,
    RingGeometry, SplineEntity, TableAlign, TableEntity, TableGrid, TableSource, TextEntity,
    Vec2 as Point,
};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::{
    Attrs, HatchAssoc as CoreAssoc, HatchPattern, Part, PointPart as CorePoint, Shape,
};
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::dimension::{Arrow, Look, LookLines};
use kentos_geometry_core::geom::hatch_pattern::{
    Gradient as CoreGradient, PatternLine as CoreLine,
};
use kentos_geometry_core::text::face::Face;
use kentos_geometry_core::text::paragraph::{Run, Script};
use kentos_geometry_core::text::{Font, TextAlign};

/// A multi-line text's runs as the core takes them (docs/adr/0182); none for none.
pub fn core_runs(runs: &[kentos_contracts::TextRun]) -> Option<Vec<Run>> {
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

/// The core's runs as the contract writes them.
pub fn contract_runs(runs: Option<Vec<Run>>) -> Vec<kentos_contracts::TextRun> {
    runs.unwrap_or_default()
        .into_iter()
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
            color: r.color,
        })
        .collect()
}

/// A table's source as the core carries it, unread (docs/adr/0184 §5): the
/// contract's JSON (`kind`, then `objects` or `name` and `sheet`).
pub fn core_source(source: Option<&TableSource>) -> Option<Json> {
    let source = source?;
    let mut fields = vec![("kind".to_owned(), Json::Str(source.kind().to_owned()))];
    match source {
        TableSource::File { name, sheet } => {
            fields.push(("name".to_owned(), Json::Str(name.clone())));
            if let Some(s) = sheet {
                fields.push(("sheet".to_owned(), Json::Str(s.clone())));
            }
        }
        _ => fields.push((
            "objects".to_owned(),
            Json::Arr(
                source
                    .objects()
                    .iter()
                    .map(|id| Json::Str(id.to_text()))
                    .collect(),
            ),
        )),
    }
    Some(Json::Obj(fields))
}

/// The source the core carried, as the contract writes it; none for one it cannot read.
pub fn contract_source(source: Option<Json>) -> Option<TableSource> {
    let source = source?;
    let text = |k: &str| match source.get(k) {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    };
    if text("kind")? == "file" {
        return Some(TableSource::File {
            name: text("name")?,
            sheet: text("sheet"),
        });
    }
    let Json::Arr(ids) = source.get("objects") else {
        return None;
    };
    let objects = ids
        .iter()
        .map(|id| match id {
            Json::Str(s) => kentos_contracts::EntityId::parse(s),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(match text("kind")?.as_str() {
        "coordinates" => TableSource::Coordinates { objects },
        "areas" => TableSource::Areas { objects },
        "attributes" => TableSource::Attributes { objects },
        _ => return None,
    })
}

/// A table as the core takes it (docs/adr/0184 §1).
pub fn core_table(t: &TableEntity) -> Shape {
    Shape::Table {
        p: v(&t.p),
        rotation: t.rotation,
        height: t.height,
        rows: t.rows.clone(),
        columns: t.columns.clone(),
        cells: t.cells.clone(),
        merges: (!t.merges.is_empty()).then(|| t.merges.iter().map(core_range).collect()),
        aligns: t
            .aligns
            .as_ref()
            .map(|a| a.iter().map(|x| x.name().to_owned()).collect()),
        header: t.header.then_some(true),
        grid: t.grid.map(|g| g.name().to_owned()),
        frame: t.frame,
        source: core_source(t.source.as_ref()),
        face: core_face(&t.face),
    }
}

fn core_range(m: &kentos_contracts::CellRange) -> kentos_geometry_core::entity::CellRange {
    kentos_geometry_core::entity::CellRange {
        row: m.row as usize,
        col: m.col as usize,
        rows: m.rows as usize,
        cols: m.cols as usize,
    }
}

fn contract_range(m: kentos_geometry_core::entity::CellRange) -> kentos_contracts::CellRange {
    let n = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
    kentos_contracts::CellRange {
        row: n(m.row),
        col: n(m.col),
        rows: n(m.rows),
        cols: n(m.cols),
    }
}

/// A table's alignments and grid as the contract names them: none for a name it does not know.
fn contract_aligns(aligns: Option<Vec<String>>) -> Option<Vec<TableAlign>> {
    aligns.map(|a| {
        a.iter()
            .map(|n| TableAlign::from_name(n).unwrap_or_default())
            .collect()
    })
}

/// A text's face as the core takes it (docs/adr/0183 §2).
pub fn core_face(f: &kentos_contracts::TextFace) -> Face {
    Face {
        style: f.text_style.clone(),
        font: f.font.map(|d| Font::from_id(d.id())),
        bold: f.bold,
        italic: f.italic,
        oblique: f.oblique,
    }
}

/// The core's face as the contract writes it.
pub fn contract_face(f: Face) -> kentos_contracts::TextFace {
    kentos_contracts::TextFace {
        text_style: f.style,
        font: f.font.and_then(|c| DrawingFont::from_id(c.id())),
        bold: f.bold,
        italic: f.italic,
        oblique: f.oblique,
    }
}

/// A dimension's look as the core takes it (docs/adr/0183 §3).
pub fn core_look(l: &kentos_contracts::DimensionLook) -> Look {
    Look {
        style: l.dim_style.clone(),
        arrow: l.arrow.and_then(|a| Arrow::from_name(a.name())),
        arrow_size: l.arrow_size,
        ext_offset: l.ext_offset,
        ext_beyond: l.ext_beyond,
        text_gap: l.text_gap,
        centre: l.text_place == Some(kentos_contracts::DimensionTextPlace::Centre),
        decimals: l.decimals,
        unit: l.unit.map(|u| u.mark().to_owned()),
        prefix: l.prefix.clone(),
        suffix: l.suffix.clone(),
        font: l.font.map(|d| Font::from_id(d.id())),
        lines: LookLines {
            dim_line_color: l.dim_line_color.clone(),
            dim_line_weight: l.dim_line_weight,
            dim_line_type: l.dim_line_type.map(|t| t.name().to_owned()),
            ext_color: l.ext_color.clone(),
            ext_weight: l.ext_weight,
            ext_line_type: l.ext_line_type.map(|t| t.name().to_owned()),
            text_color: l.text_color.clone(),
        }
        .boxed(),
    }
}

/// The core's look as the contract writes it.
pub fn contract_look(l: Look) -> kentos_contracts::DimensionLook {
    use kentos_contracts::DrawingUnit;
    let lines = l.lines.map(|b| *b).unwrap_or_default();
    kentos_contracts::DimensionLook {
        dim_style: l.style,
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
        unit: l.unit.as_deref().and_then(|u| match u {
            "mm" => Some(DrawingUnit::Mm),
            "cm" => Some(DrawingUnit::Cm),
            "m" => Some(DrawingUnit::M),
            _ => None,
        }),
        prefix: l.prefix,
        suffix: l.suffix,
        font: l.font.and_then(|c| DrawingFont::from_id(c.id())),
        dim_line_color: lines.dim_line_color,
        dim_line_weight: lines.dim_line_weight,
        dim_line_type: lines
            .dim_line_type
            .as_deref()
            .and_then(kentos_contracts::LineType::from_name),
        ext_color: lines.ext_color,
        ext_weight: lines.ext_weight,
        ext_line_type: lines
            .ext_line_type
            .as_deref()
            .and_then(kentos_contracts::LineType::from_name),
        text_color: lines.text_color,
    }
}

fn v(p: &Point) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn points(pts: &[Point]) -> Vec<Vec2> {
    pts.iter().map(v).collect()
}

fn ring(r: &RingGeometry) -> Ring {
    Ring {
        pts: points(&r.pts),
        bulges: r.bulges.clone(),
    }
}

/// A multi-point object's points past its first as the core takes them (docs/adr/0174).
fn core_points(parts: &Option<Vec<kentos_contracts::PointPart>>) -> Option<Vec<CorePoint>> {
    parts.as_ref().map(|ps| {
        ps.iter()
            .map(|q| CorePoint { p: v(&q.p), z: q.z })
            .collect()
    })
}

/// The core's points of a multi-point object back (docs/adr/0174).
fn points_back(parts: Option<Vec<CorePoint>>) -> Option<Vec<kentos_contracts::PointPart>> {
    parts.map(|ps| {
        ps.into_iter()
            .map(|q| kentos_contracts::PointPart { p: p(q.p), z: q.z })
            .collect()
    })
}

/// A part of a multi-part area or polyline as the core takes it (docs/adr/0143, 0174).
fn part(p: &AreaPart) -> Part {
    Part {
        pts: points(&p.pts),
        bulges: p.bulges.clone(),
        holes: p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
    }
}

/// A dimension style as the core names it (as files write it).
fn style_name(style: DimensionStyle) -> &'static str {
    style.name()
}

fn pattern_name(kind: HatchPatternType) -> &'static str {
    match kind {
        HatchPatternType::Solid => "solid",
        HatchPatternType::Lines => "lines",
        HatchPatternType::Cross => "cross",
        HatchPatternType::Pattern => "pattern",
        HatchPatternType::Gradient => "gradient",
    }
}

fn gradient_name(shape: GradientShape) -> &'static str {
    match shape {
        GradientShape::Linear => "linear",
        GradientShape::Cylinder => "cylinder",
        GradientShape::Spherical => "spherical",
    }
}

fn gradient_of(name: &str) -> Option<GradientShape> {
    Some(match name {
        "linear" => GradientShape::Linear,
        "cylinder" => GradientShape::Cylinder,
        "spherical" => GradientShape::Spherical,
        _ => return None,
    })
}

/// A hatch's pattern as the geometry core holds it (docs/adr/0186 §1).
pub fn core_pattern(p: &ContractPattern) -> HatchPattern {
    HatchPattern {
        kind: pattern_name(p.kind).to_owned(),
        angle: p.angle,
        spacing: p.spacing,
        name: p.name.clone(),
        scale: p.scale,
        lines: p.lines.as_ref().map(|ls| {
            ls.iter()
                .map(|l| CoreLine {
                    angle: l.angle,
                    origin: l.origin,
                    offset: l.offset,
                    dashes: (!l.dashes.is_empty()).then(|| l.dashes.clone()),
                })
                .collect()
        }),
        gradient: p.gradient.as_ref().map(|g| CoreGradient {
            shape: gradient_name(g.shape).to_owned(),
            inverted: g.inverted.then_some(true),
            color2: g.color2.clone(),
        }),
    }
}

/// The core's pattern in the contract; none for a kind or a gradient's shape it does not know.
pub fn contract_pattern(p: HatchPattern) -> Option<ContractPattern> {
    Some(ContractPattern {
        kind: pattern_of(&p.kind)?,
        angle: p.angle,
        spacing: p.spacing,
        name: p.name,
        scale: p.scale,
        lines: p.lines.map(|ls| {
            ls.into_iter()
                .map(|l| PatternLine {
                    angle: l.angle,
                    origin: l.origin,
                    offset: l.offset,
                    dashes: l.dashes.unwrap_or_default(),
                })
                .collect()
        }),
        gradient: match p.gradient {
            Some(g) => Some(HatchGradient {
                shape: gradient_of(&g.shape)?,
                inverted: g.inverted == Some(true),
                color2: g.color2,
            }),
            None => None,
        },
    })
}

/// A hatch's tie to its objects as the core holds it: their ids as text (docs/adr/0186 §6).
pub fn core_assoc(a: &HatchAssoc) -> CoreAssoc {
    let ids = |list: &[EntityId]| {
        (!list.is_empty()).then(|| list.iter().map(EntityId::to_text).collect())
    };
    CoreAssoc {
        outer: a.outer.to_text(),
        islands: ids(&a.islands),
        cutouts: ids(&a.cutouts),
        seed: v(&a.seed),
    }
}

/// The core's tie in the contract; none when an id is not one.
pub fn contract_assoc(a: CoreAssoc) -> Option<HatchAssoc> {
    let ids = |list: Option<Vec<String>>| -> Option<Vec<EntityId>> {
        list.unwrap_or_default()
            .iter()
            .map(|t| EntityId::parse(t))
            .collect()
    };
    Some(HatchAssoc {
        outer: EntityId::parse(&a.outer)?,
        islands: ids(a.islands)?,
        cutouts: ids(a.cutouts)?,
        seed: Point {
            x: a.seed.x,
            y: a.seed.y,
        },
    })
}

/// A dimension style by the name the core carries; None for a name the contract does not know.
fn style_of(name: &str) -> Option<DimensionStyle> {
    DimensionStyle::from_name(name)
}

/// A hatch pattern's type by the name the core carries; None for a name the contract does not know.
fn pattern_of(name: &str) -> Option<HatchPatternType> {
    Some(match name {
        "solid" => HatchPatternType::Solid,
        "lines" => HatchPatternType::Lines,
        "cross" => HatchPatternType::Cross,
        "pattern" => HatchPatternType::Pattern,
        "gradient" => HatchPatternType::Gradient,
        _ => return None,
    })
}

/// The drawing's typeface as the geometry core measures text in it (the
/// project's `drawingFont`; Barlow without one): what text boxes, the
/// geometry store and a polar array's middle are measured in.
pub fn drawing_font(font: Option<DrawingFont>) -> Font {
    Font::from_id(match font {
        None | Some(DrawingFont::Barlow) => "barlow",
        Some(DrawingFont::Arimo) => "arimo",
        Some(DrawingFont::Overpass) => "overpass",
        Some(DrawingFont::Quicksand) => "quicksand",
        Some(DrawingFont::ArchitectsDaughter) => "architects-daughter",
        Some(DrawingFont::CourierPrime) => "courier-prime",
        Some(DrawingFont::PlexMono) => "plex-mono",
    })
}

/// A picture's fields as the geometry core takes them (docs/adr/0192).
pub fn core_image(i: &ImageFields) -> Shape {
    Shape::Image {
        p: v(&i.p),
        width: i.width,
        height: i.height,
        rotation: i.rotation,
        mirror: i.mirror.then_some(true),
        asset: i.asset.clone(),
        file: i.file.clone(),
        clip: i.clip.as_deref().map(points),
        opacity: i.opacity,
    }
}

/// A picture the core computed as the contract holds it; none for another shape.
pub fn contract_image(shape: Shape) -> Option<ImageFields> {
    match shape {
        Shape::Image {
            p: at,
            width,
            height,
            rotation,
            mirror,
            asset,
            file,
            clip,
            opacity,
        } => Some(ImageFields {
            p: p(at),
            width,
            height,
            rotation,
            mirror: mirror == Some(true),
            asset,
            file,
            clip: clip.map(back),
            opacity,
        }),
        _ => None,
    }
}

/// A raster's fields as the geometry core takes them (docs/adr/0204 §2): its
/// look as the contract's JSON, which the core carries.
pub fn core_raster(r: &RasterFields) -> Shape {
    Shape::Raster {
        affine: r.affine,
        width: f64::from(r.width),
        height: f64::from(r.height),
        bands: f64::from(r.bands),
        sample: r.sample.name().to_owned(),
        asset: r.asset.clone(),
        file: r.file.clone(),
        url: r.url.clone(),
        srid: f64::from(r.srid),
        style: Json::parse(&r.style.to_json_text()).unwrap_or(Json::Null),
        opacity: r.opacity,
    }
}

/// A raster the core computed as the contract holds it; none for another
/// shape, or numbers and a look the contract cannot hold.
pub fn contract_raster(shape: Shape) -> Option<RasterFields> {
    let Shape::Raster {
        affine,
        width,
        height,
        bands,
        sample,
        asset,
        file,
        url,
        srid,
        style,
        opacity,
    } = shape
    else {
        return None;
    };
    let whole =
        |v: f64| (v >= 0.0 && v.fract() == 0.0 && v <= f64::from(u32::MAX)).then_some(v as u32);
    Some(RasterFields {
        affine,
        width: whole(width)?,
        height: whole(height)?,
        bands: whole(bands)?,
        sample: RasterSample::from_name(&sample)?,
        asset,
        file,
        url,
        srid: whole(srid)?,
        style: kentos_contracts::RasterStyle::from_json_text(
            &kentos_geometry_core::api::json::to_string(&style),
        )?,
        opacity,
    })
}

/// A point cloud's fields as the geometry core takes them (docs/adr/0207 §3):
/// its files and look as the contract's JSON, which the core carries.
pub fn core_cloud(c: &kentos_contracts::PointCloudFields) -> Shape {
    Shape::PointCloud {
        bounds: c.bounds,
        count: c.count as f64,
        sources: Json::parse(&kentos_contracts::sources_json_text(&c.sources))
            .unwrap_or(Json::Null),
        srid: f64::from(c.srid),
        style: Json::parse(&c.style.to_json_text()).unwrap_or(Json::Null),
        opacity: c.opacity,
    }
}

/// A point cloud the core carried as the contract holds it; none for another
/// shape, or numbers, files and a look the contract cannot hold.
pub fn contract_cloud(shape: Shape) -> Option<kentos_contracts::PointCloudFields> {
    let Shape::PointCloud {
        bounds,
        count,
        sources,
        srid,
        style,
        opacity,
    } = shape
    else {
        return None;
    };
    let whole = |v: f64| (v >= 0.0 && v.fract() == 0.0 && v.is_finite()).then_some(v as u64);
    Some(kentos_contracts::PointCloudFields {
        sources: kentos_contracts::sources_from_json_text(
            &kentos_geometry_core::api::json::to_string(&sources),
        )?,
        bounds,
        count: whole(count)?,
        srid: u32::try_from(whole(srid)?).ok()?,
        style: kentos_contracts::PointCloudStyle::from_json_text(
            &kentos_geometry_core::api::json::to_string(&style),
        )?,
        opacity,
    })
}

/// An object's geometry as the geometry core takes it.
pub fn shape(entity: &Entity) -> Shape {
    match entity {
        Entity::Point(p) => Shape::Point {
            p: v(&p.p),
            z: p.z,
            // Every point of a multi-point object (docs/adr/0174).
            parts: core_points(&p.parts),
        },
        Entity::Line(l) => Shape::Line {
            a: v(&l.a),
            b: v(&l.b),
        },
        Entity::Polyline(p) => Shape::Polyline {
            pts: points(&p.pts),
            bulges: p.bulges.clone(),
            holes: p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
            // Every part of a multi-part polyline (docs/adr/0174).
            parts: p.parts.as_ref().map(|ps| ps.iter().map(part).collect()),
        },
        Entity::Polygon(p) => Shape::Polygon {
            pts: points(&p.pts),
            bulges: p.bulges.clone(),
            holes: p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
            parts: p.parts.as_ref().map(|ps| ps.iter().map(part).collect()),
        },
        Entity::Circle(c) => Shape::Circle { c: v(&c.c), r: c.r },
        Entity::Leader(l) => Shape::Leader {
            pts: points(&l.pts),
            text: l.text.clone(),
            height: l.height,
            rotation: l.rotation,
            arrow: l.arrow.map(|a| a.name().to_owned()),
            arrow_size: l.arrow_size,
            mask: l.mask.then_some(true),
        },
        Entity::Table(t) => core_table(t),
        Entity::Image(i) => core_image(&i.image),
        Entity::Raster(r) => core_raster(&r.raster),
        Entity::PointCloud(c) => core_cloud(&c.cloud),
        Entity::Arc(a) => Shape::Arc {
            c: v(&a.c),
            r: a.r,
            a0: a.a0,
            a1: a.a1,
        },
        Entity::Ellipse(e) => Shape::Ellipse {
            c: v(&e.c),
            major: v(&e.major),
            ratio: e.ratio,
            t0: e.t0,
            t1: e.t1,
        },
        Entity::Spline(s) => Shape::Spline {
            pts: points(&s.pts),
            closed: s.closed,
        },
        Entity::Xline(c) => Shape::Xline {
            p: v(&c.p),
            dir: v(&c.dir),
        },
        Entity::Ray(c) => Shape::Ray {
            p: v(&c.p),
            dir: v(&c.dir),
        },
        Entity::Text(t) => Shape::Text {
            p: v(&t.p),
            text: t.text.clone(),
            height: t.height,
            rotation: t.rotation,
            align: t.align.and_then(|a| TextAlign::from_name(a.name())),
            width_factor: t.width_factor,
            mask: t.mask.then_some(true),
            box_width: t.paragraph.box_width,
            line_spacing: t.paragraph.line_spacing,
            runs: core_runs(&t.paragraph.runs),
            face: core_face(&t.face),
            path: t.path.as_ref().map(core_curve),
        },
        Entity::Dimension(d) => Shape::Dimension {
            a: v(&d.a),
            b: v(&d.b),
            offset: d.offset,
            height: d.height,
            text: d.text.clone(),
            style: d.style.map(|s| style_name(s).to_owned()),
            angle: d.angle,
            c: d.c.as_ref().map(v),
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
            assoc: h.assoc.as_ref().map(core_assoc),
        },
        // Its attributes too: what its block's attribute texts show (docs/adr/0144 §7).
        Entity::Insert(i) => Shape::Insert {
            block: i.block.to_text(),
            p: v(&i.p),
            scale: i.scale,
            rotation: i.rotation,
            mirror: i.mirror.then_some(true),
            attrs: Some(Attrs(
                i.base
                    .attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), Json::Str(v.clone())))
                    .collect(),
            )),
        },
    }
}

/// A text's curve for the core (docs/adr/0196).
pub fn core_curve(c: &kentos_contracts::TextPath) -> kentos_geometry_core::text::along::Curve {
    kentos_geometry_core::text::along::Curve {
        pts: points(&c.pts),
        bulges: c.bulges.clone(),
    }
}

/// A core curve as the contract's.
pub fn contract_curve(c: kentos_geometry_core::text::along::Curve) -> kentos_contracts::TextPath {
    kentos_contracts::TextPath {
        pts: back(c.pts),
        bulges: c.bulges,
    }
}

fn p(v: Vec2) -> Point {
    Point { x: v.x, y: v.y }
}

fn back(pts: Vec<Vec2>) -> Vec<Point> {
    pts.into_iter().map(p).collect()
}

fn ring_back(r: Ring) -> RingGeometry {
    RingGeometry {
        pts: back(r.pts),
        bulges: r.bulges,
        zs: None,
    }
}

/// The core's holes back, each keeping the elevations of the hole it was
/// when its vertex count is the same (a transform moves each vertex and
/// keeps it; docs/adr/0142).
fn holes_back(
    holes: Option<Vec<Ring>>,
    before: Option<&[RingGeometry]>,
) -> Option<Vec<RingGeometry>> {
    holes.map(|hs| {
        hs.into_iter()
            .enumerate()
            .map(|(i, h)| {
                let mut ring = ring_back(h);
                ring.zs = before
                    .and_then(|b| b.get(i))
                    .filter(|b| b.pts.len() == ring.pts.len())
                    .and_then(|b| b.zs.clone());
                ring
            })
            .collect()
    })
}

/// The core's parts back, each keeping its own elevations and its holes'
/// as `holes_back` does (docs/adr/0143).
fn parts_back(parts: Option<Vec<Part>>, before: Option<&[AreaPart]>) -> Option<Vec<AreaPart>> {
    parts.map(|ps| {
        ps.into_iter()
            .enumerate()
            .map(|(i, p)| {
                let was = before.and_then(|b| b.get(i));
                let pts = back(p.pts);
                let zs = was
                    .filter(|w| w.pts.len() == pts.len())
                    .and_then(|w| w.zs.clone());
                AreaPart {
                    holes: holes_back(p.holes, was.and_then(|w| w.holes.as_deref())),
                    pts,
                    bulges: p.bulges,
                    zs,
                }
            })
            .collect()
    })
}

/// `entity` with `shape` as its geometry: the core's answer for it (a moved,
/// rotated, scaled or mirrored copy of its own shape). Every other field is
/// kept, as the web's `withGeometry` keeps them; the geometry fields are the
/// shape's, float64 bit for bit. None when the shape is of another kind: the
/// core's transforms never change a kind.
pub fn with_shape(entity: &Entity, shape: Shape) -> Option<Entity> {
    let mut out = entity.clone();
    match (&mut out, shape) {
        (Entity::Point(e), Shape::Point { p: at, z, parts }) => {
            e.p = p(at);
            e.z = z;
            // Every point too, with its elevation (docs/adr/0174).
            e.parts = points_back(parts);
        }
        (Entity::Line(e), Shape::Line { a, b }) => {
            e.a = p(a);
            e.b = p(b);
        }
        (
            Entity::Polyline(e),
            Shape::Polyline {
                pts,
                bulges,
                holes,
                parts,
            },
        ) => {
            e.pts = back(pts);
            e.bulges = bulges;
            let before = e.holes.take();
            e.holes = holes_back(holes, before.as_deref());
            // Every part too, with its elevations (docs/adr/0174).
            let was = e.parts.take();
            e.parts = parts_back(parts, was.as_deref());
            if e.zs.as_ref().is_some_and(|zs| zs.len() != e.pts.len()) {
                e.zs = None;
            }
        }
        (
            Entity::Polygon(e),
            Shape::Polygon {
                pts,
                bulges,
                holes,
                parts: None,
            },
        ) => {
            e.pts = back(pts);
            e.bulges = bulges;
            // A transform moves each vertex and keeps it: the elevations stay
            // with their vertices, the holes' too (docs/adr/0142).
            let before = e.holes.take();
            e.holes = holes_back(holes, before.as_deref());
            e.parts = None;
            if e.zs.as_ref().is_some_and(|zs| zs.len() != e.pts.len()) {
                e.zs = None;
            }
        }
        (
            Entity::Polygon(e),
            Shape::Polygon {
                pts,
                bulges,
                holes,
                parts: Some(parts),
            },
        ) => {
            e.pts = back(pts);
            e.bulges = bulges;
            let before = e.holes.take();
            e.holes = holes_back(holes, before.as_deref());
            // Every part too, with its elevations and its holes' (docs/adr/0143).
            let was = e.parts.take();
            e.parts = parts_back(Some(parts), was.as_deref());
            if e.zs.as_ref().is_some_and(|zs| zs.len() != e.pts.len()) {
                e.zs = None;
            }
        }
        (Entity::Circle(e), Shape::Circle { c, r }) => {
            e.c = p(c);
            e.r = r;
        }
        // A transform keeps the block and composes the rest (docs/adr/0144 §3).
        (
            Entity::Insert(e),
            Shape::Insert {
                p: at,
                scale,
                rotation,
                mirror,
                ..
            },
        ) => {
            e.p = p(at);
            e.scale = scale;
            e.rotation = rotation;
            e.mirror = mirror.unwrap_or(false);
        }
        (Entity::Arc(e), Shape::Arc { c, r, a0, a1 }) => {
            e.c = p(c);
            e.r = r;
            e.a0 = a0;
            e.a1 = a1;
        }
        (
            Entity::Ellipse(e),
            Shape::Ellipse {
                c,
                major,
                ratio,
                t0,
                t1,
            },
        ) => {
            e.c = p(c);
            e.major = p(major);
            e.ratio = ratio;
            e.t0 = t0;
            e.t1 = t1;
        }
        (Entity::Spline(e), Shape::Spline { pts, closed }) => {
            e.pts = back(pts);
            e.closed = closed;
        }
        (Entity::Xline(e), Shape::Xline { p: at, dir })
        | (Entity::Ray(e), Shape::Ray { p: at, dir }) => {
            e.p = p(at);
            e.dir = p(dir);
        }
        (
            Entity::Text(e),
            Shape::Text {
                p: at,
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
                path,
            },
        ) => {
            // The face is the object's own (docs/adr/0183): the core carries it through unchanged.
            e.p = p(at);
            e.text = text;
            e.height = height;
            e.rotation = rotation;
            e.align = align.and_then(|a| kentos_contracts::TextAlign::from_name(a.name()));
            e.width_factor = width_factor;
            e.mask = mask == Some(true);
            e.paragraph = kentos_contracts::Paragraph {
                box_width,
                line_spacing,
                runs: contract_runs(runs),
            };
            e.path = path.map(contract_curve);
        }
        (
            Entity::Dimension(e),
            Shape::Dimension {
                a,
                b,
                offset,
                height,
                text,
                style: _,
                angle,
                c,
                mask,
                za,
                zb,
                look: _,
            },
        ) => {
            // The style and the look are the object's own: the core carries them through unchanged.
            e.a = p(a);
            e.b = p(b);
            e.offset = offset;
            e.height = height;
            e.text = text;
            e.angle = angle;
            e.c = c.map(p);
            e.mask = mask == Some(true);
            e.za = za;
            e.zb = zb;
        }
        (
            Entity::Hatch(e),
            Shape::Hatch {
                ring,
                holes,
                pattern,
                assoc,
            },
        ) => {
            // The core carries the pattern's type through; it turns, scales and reflects
            // its definition, and moves the point it follows from (docs/adr/0186 §8).
            e.ring = back(ring);
            e.holes = holes.map(|hs| hs.into_iter().map(back).collect());
            if let Some(p) = contract_pattern(pattern) {
                e.pattern = p;
            }
            e.assoc = assoc.and_then(contract_assoc);
        }
        (
            Entity::Leader(e),
            Shape::Leader {
                pts,
                text,
                height,
                rotation,
                arrow: _,
                arrow_size: _,
                mask,
            },
        ) => {
            // The arrowhead is the object's own: the core carries its name and size through unchanged.
            e.pts = back(pts);
            e.text = text;
            e.height = height;
            e.rotation = rotation;
            e.mask = mask == Some(true);
        }
        (
            Entity::Table(e),
            Shape::Table {
                p: at,
                rotation,
                height,
                rows,
                columns,
                cells,
                merges,
                aligns,
                header,
                grid,
                frame,
                source: _,
                face: _,
            },
        ) => {
            // Its face and source are the object's own: the core carries them through unchanged.
            e.p = p(at);
            e.rotation = rotation;
            e.height = height;
            e.rows = rows;
            e.columns = columns;
            e.cells = cells;
            e.merges = merges
                .unwrap_or_default()
                .into_iter()
                .map(contract_range)
                .collect();
            e.aligns = contract_aligns(aligns);
            e.header = header == Some(true);
            e.grid = grid.as_deref().and_then(TableGrid::from_name);
            e.frame = frame;
        }
        (Entity::Image(e), s @ Shape::Image { .. }) => e.image = contract_image(s)?,
        (Entity::Raster(e), s @ Shape::Raster { .. }) => e.raster = contract_raster(s)?,
        (Entity::PointCloud(e), s @ Shape::PointCloud { .. }) => e.cloud = contract_cloud(s)?,
        _ => return None,
    }
    Some(out)
}

/// Written elevations as an object holds them (docs/adr/0142): a list with
/// none in it is no list.
fn held(zs: Option<Vec<Option<f64>>>) -> Option<Vec<Option<f64>>> {
    zs.filter(|z| z.iter().any(Option::is_some))
}

/// An object copied into a new one (Kopyala, Dizi, a block's definition,
/// Yapıştır): a linked text's copy writes no object's label, it is a text of
/// its own (docs/adr/0175 §4); an associative hatch's copy follows no
/// objects (docs/adr/0186 §6); every other object as it is.
pub fn unlinked(mut entity: Entity) -> Entity {
    match &mut entity {
        Entity::Text(text) => {
            text.label_of = None;
            text.label_scale = None;
        }
        Entity::Hatch(hatch) => hatch.assoc = None,
        _ => {}
    }
    entity
}

/// An object of `geometry` with the fields every object has from `base`:
/// what `cad.entities.edit` writes (docs/adr/0047). A polyline has no holes.
pub fn entity_of(geometry: &EntityGeometry, base: EntityBase) -> Entity {
    match geometry.clone() {
        EntityGeometry::Point { p, z, parts } => Entity::Point(PointEntity { base, p, z, parts }),
        EntityGeometry::Line { a, b, zs } => {
            let z = |k: usize| zs.as_ref().and_then(|zs| zs.get(k).copied().flatten());
            Entity::Line(LineEntity {
                base,
                a,
                b,
                za: z(0),
                zb: z(1),
            })
        }
        EntityGeometry::Polyline {
            pts,
            bulges,
            zs,
            parts,
        } => Entity::Polyline(PathEntity {
            base,
            pts,
            bulges,
            holes: None,
            zs: held(zs),
            // Every part as written, its elevations held as the polyline's (docs/adr/0174).
            parts: parts.map(|ps| {
                ps.into_iter()
                    .map(|mut p| {
                        p.zs = held(p.zs);
                        p.holes = None;
                        p
                    })
                    .collect()
            }),
        }),
        EntityGeometry::Polygon {
            pts,
            bulges,
            holes,
            zs,
            parts,
        } => {
            let holes_held = |hs: Option<Vec<RingGeometry>>| {
                hs.map(|hs| {
                    hs.into_iter()
                        .map(|mut h| {
                            h.zs = held(h.zs);
                            h
                        })
                        .collect::<Vec<_>>()
                })
            };
            Entity::Polygon(PathEntity {
                base,
                pts,
                bulges,
                holes: holes_held(holes),
                zs: held(zs),
                // Every part as written, its elevations and its holes' held as the area's (docs/adr/0143).
                parts: parts.map(|ps| {
                    ps.into_iter()
                        .map(|mut p| {
                            p.zs = held(p.zs);
                            p.holes = holes_held(p.holes);
                            p
                        })
                        .collect()
                }),
            })
        }
        EntityGeometry::Circle { c, r } => Entity::Circle(CircleEntity { base, c, r }),
        EntityGeometry::Arc { c, r, a0, a1 } => Entity::Arc(ArcEntity { base, c, r, a0, a1 }),
        EntityGeometry::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => Entity::Ellipse(EllipseEntity {
            base,
            c,
            major,
            ratio,
            t0,
            t1,
        }),
        EntityGeometry::Spline { pts, closed } => {
            Entity::Spline(SplineEntity { base, pts, closed })
        }
        EntityGeometry::Xline { p, dir } => Entity::Xline(ConstructionEntity { base, p, dir }),
        EntityGeometry::Ray { p, dir } => Entity::Ray(ConstructionEntity { base, p, dir }),
        // A width factor of 1 is no width factor (docs/adr/0145): one spelling.
        EntityGeometry::Text {
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
            path,
        } => Entity::Text(TextEntity {
            base,
            p,
            text,
            height,
            rotation,
            path,
            align,
            width_factor: width_factor.filter(|w| *w != 1.0),
            mask,
            // A geometry is no link: a text written or moved by a command follows no object (docs/adr/0175 §4).
            label_of: None,
            label_scale: None,
            // A spacing of 1 is no spacing (docs/adr/0182): one spelling.
            paragraph: kentos_contracts::Paragraph {
                box_width,
                line_spacing: line_spacing.filter(|s| *s != 1.0),
                runs,
            },
            face,
        }),
        EntityGeometry::Dimension {
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
        }),
        EntityGeometry::Hatch {
            ring,
            holes,
            pattern,
            assoc,
        } => Entity::Hatch(HatchEntity {
            base,
            ring,
            holes,
            pattern,
            assoc,
        }),
        EntityGeometry::Insert {
            block,
            p,
            scale,
            rotation,
            mirror,
        } => Entity::Insert(InsertEntity {
            base,
            block,
            p,
            scale,
            rotation,
            mirror,
        }),
        EntityGeometry::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            arrow_size,
            mask,
        } => Entity::Leader(LeaderEntity {
            base,
            pts,
            text,
            height,
            rotation,
            arrow,
            arrow_size,
            mask,
        }),
        EntityGeometry::Table {
            p,
            rotation,
            height,
            rows,
            columns,
            cells,
            merges,
            aligns,
            header,
            grid,
            frame,
            face,
            source,
        } => Entity::Table(TableEntity {
            base,
            p,
            rotation,
            height,
            rows,
            columns,
            cells,
            merges,
            aligns,
            header,
            grid,
            frame,
            face,
            source,
        }),
        EntityGeometry::Image(image) => Entity::Image(ImageEntity { base, image }),
        EntityGeometry::Raster(raster) => Entity::Raster(RasterEntity { base, raster }),
        EntityGeometry::PointCloud(cloud) => {
            Entity::PointCloud(kentos_contracts::PointCloudEntity { base, cloud })
        }
    }
}

/// A shape the core computed as `cad.entities.edit` takes it: its kind and
/// geometry fields, float64 bit for bit; a polyline's holes are left out.
/// None for a dimension style, a hatch pattern or a block id the contract
/// does not know (the core carries them as names).
pub fn edit_geometry(shape: Shape) -> Option<EntityGeometry> {
    Some(match shape {
        Shape::Point { p: at, z, parts } => EntityGeometry::Point {
            p: p(at),
            z,
            parts: points_back(parts),
        },
        Shape::Line { a, b } => EntityGeometry::Line {
            a: p(a),
            b: p(b),
            zs: None,
        },
        Shape::Polyline {
            pts, bulges, parts, ..
        } => EntityGeometry::Polyline {
            pts: back(pts),
            bulges,
            zs: None,
            parts: parts_back(parts, None).map(|ps| {
                ps.into_iter()
                    .map(|mut q| {
                        q.holes = None;
                        q
                    })
                    .collect()
            }),
        },
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => EntityGeometry::Polygon {
            pts: back(pts),
            bulges,
            holes: holes.map(|hs| hs.into_iter().map(ring_back).collect()),
            zs: None,
            parts: parts_back(parts, None),
        },
        Shape::Circle { c, r } => EntityGeometry::Circle { c: p(c), r },
        Shape::Arc { c, r, a0, a1 } => EntityGeometry::Arc { c: p(c), r, a0, a1 },
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => EntityGeometry::Ellipse {
            c: p(c),
            major: p(major),
            ratio,
            t0,
            t1,
        },
        Shape::Spline { pts, closed } => EntityGeometry::Spline {
            pts: back(pts),
            closed,
        },
        Shape::Xline { p: at, dir } => EntityGeometry::Xline {
            p: p(at),
            dir: p(dir),
        },
        Shape::Ray { p: at, dir } => EntityGeometry::Ray {
            p: p(at),
            dir: p(dir),
        },
        Shape::Text {
            p: at,
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
            path,
        } => EntityGeometry::Text {
            p: p(at),
            text,
            height,
            rotation,
            align: align.and_then(|a| kentos_contracts::TextAlign::from_name(a.name())),
            width_factor,
            mask: mask == Some(true),
            box_width,
            line_spacing,
            runs: contract_runs(runs),
            face: contract_face(face),
            path: path.map(contract_curve),
        },
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
        } => EntityGeometry::Dimension {
            a: p(a),
            b: p(b),
            offset,
            height,
            text,
            style: match style {
                Some(name) => Some(style_of(&name)?),
                None => None,
            },
            angle,
            c: c.map(p),
            mask: mask == Some(true),
            za,
            zb,
            look: contract_look(look),
        },
        Shape::Hatch {
            ring,
            holes,
            pattern,
            assoc,
        } => EntityGeometry::Hatch {
            ring: back(ring),
            holes: holes.map(|hs| hs.into_iter().map(back).collect()),
            pattern: contract_pattern(pattern)?,
            assoc: match assoc {
                Some(a) => Some(contract_assoc(a)?),
                None => None,
            },
        },
        // An arrowhead the contract does not name is none of a leader's (docs/adr/0146 §1).
        Shape::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            arrow_size,
            mask,
        } => EntityGeometry::Leader {
            pts: back(pts),
            text,
            height,
            rotation,
            arrow: match arrow {
                Some(name) => Some(LeaderArrow::from_name(&name)?),
                None => None,
            },
            arrow_size,
            mask: mask == Some(true),
        },
        // A block's id the contract does not read is none of the drawing's.
        Shape::Insert {
            block,
            p: at,
            scale,
            rotation,
            mirror,
            ..
        } => EntityGeometry::Insert {
            block: BlockId::parse(&block)?,
            p: p(at),
            scale,
            rotation,
            mirror: mirror == Some(true),
        },
        // A table's alignment or lines the contract does not name are none of its (docs/adr/0184 §1).
        Shape::Table {
            p: at,
            rotation,
            height,
            rows,
            columns,
            cells,
            merges,
            aligns,
            header,
            grid,
            frame,
            source,
            face,
        } => EntityGeometry::Table {
            p: p(at),
            rotation,
            height,
            rows,
            columns,
            cells,
            merges: merges
                .unwrap_or_default()
                .into_iter()
                .map(contract_range)
                .collect(),
            aligns: match aligns {
                Some(a) => Some(
                    a.iter()
                        .map(|n| TableAlign::from_name(n))
                        .collect::<Option<Vec<_>>>()?,
                ),
                None => None,
            },
            header: header == Some(true),
            grid: match grid {
                Some(g) => Some(TableGrid::from_name(&g)?),
                None => None,
            },
            frame,
            face: contract_face(face),
            source: contract_source(source),
        },
        s @ Shape::Image { .. } => EntityGeometry::Image(contract_image(s)?),
        s @ Shape::Raster { .. } => EntityGeometry::Raster(contract_raster(s)?),
        s @ Shape::PointCloud { .. } => EntityGeometry::PointCloud(contract_cloud(s)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(json: &str) -> Entity {
        serde_json::from_str(json).expect("an object")
    }

    /// Every kind to the core's shape and back is the same object, bit for
    /// bit; a shape of another kind is refused.
    #[test]
    fn every_kind_goes_to_the_core_and_back_unchanged() {
        let objects = [
            r#"{"kind":"point","id":1,"layerId":"a","attrs":{"Ad":"P1"},"label":"P1","p":{"x":486512.34,"y":-0.0},"z":850.5}"#,
            r##"{"kind":"line","id":2,"layerId":"a","attrs":{},"color":"#E5484D","a":{"x":1,"y":2},"b":{"x":3,"y":4}}"##,
            r#"{"kind":"polyline","id":3,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0}],"bulges":[0.5]}"#,
            r#"{"kind":"polygon","id":4,"layerId":"a","attrs":{"Ada":"104"},"symbol":"s","pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}],"bulges":[0,0,-0.1]}]}"#,
            r#"{"kind":"circle","id":5,"layerId":"a","attrs":{},"c":{"x":1,"y":2},"r":3}"#,
            r#"{"kind":"arc","id":6,"layerId":"a","attrs":{},"c":{"x":1,"y":2},"r":3,"a0":0.5,"a1":2}"#,
            r#"{"kind":"ellipse","id":7,"layerId":"a","attrs":{},"c":{"x":1,"y":2},"major":{"x":10,"y":-3},"ratio":0.4,"t0":1,"t1":2}"#,
            r#"{"kind":"spline","id":8,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":1},{"x":2,"y":0}],"closed":true}"#,
            r#"{"kind":"xline","id":9,"layerId":"a","attrs":{},"p":{"x":1,"y":2},"dir":{"x":0.6,"y":0.8}}"#,
            r#"{"kind":"ray","id":10,"layerId":"a","attrs":{},"p":{"x":1,"y":2},"dir":{"x":-1,"y":0}}"#,
            r#"{"kind":"text","id":11,"layerId":"a","attrs":{},"p":{"x":1,"y":2},"text":"Ada 104","height":2,"rotation":-30}"#,
            r#"{"kind":"dimension","id":12,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":-2,"height":0.5,"text":"12,5 m","style":"linear","angle":90}"#,
            r#"{"kind":"hatch","id":13,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"cross","angle":30,"spacing":0.5}}"#,
            r#"{"kind":"hatch","id":16,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"pattern":{"type":"pattern","angle":15,"spacing":1,"name":"ANSI33","scale":0.5,"lines":[{"angle":45,"origin":[0,0],"offset":[0,6.35]},{"angle":45,"origin":[4.490128,0],"offset":[0,6.35],"dashes":[3.175,-1.5875]}]},"assoc":{"outer":"0192a3b4-c5d6-7e8f-9012-3456789abcde","islands":["0192a3b4-c5d6-7e8f-9012-3456789abcdf"],"seed":{"x":1,"y":0.5}}}"#,
            r##"{"kind":"hatch","id":17,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"pattern":{"type":"gradient","angle":90,"spacing":1,"gradient":{"shape":"spherical","inverted":true,"color2":"#FFFFFF"}}}"##,
        ];
        for text in objects {
            let e = entity(text);
            let again = with_shape(&e, shape(&e)).expect("the same kind");
            assert_eq!(
                serde_json::to_string(&again).unwrap(),
                serde_json::to_string(&e).unwrap(),
                "{text}"
            );
        }
        let point = entity(objects[0]);
        assert_eq!(
            with_shape(&point, shape(&entity(objects[1]))),
            None,
            "a line is not a point"
        );
        // −0 is carried over as −0.
        let Some(Entity::Point(p)) = with_shape(&point, shape(&point)) else {
            panic!("a point");
        };
        assert!(p.p.y == 0.0 && p.p.y.is_sign_negative());
    }

    /// Every kind's geometry as the edit command takes it, written back
    /// onto its object, is that object (Esnet writes dimensions and
    /// hatches too); a name the contract does not know is no geometry.
    #[test]
    fn every_kind_is_written_as_the_core_gives_it() {
        let objects = [
            r#"{"kind":"point","id":1,"layerId":"a","attrs":{},"p":{"x":486512.34,"y":-0.0},"z":850.5}"#,
            r#"{"kind":"polyline","id":3,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0}],"bulges":[0.5]}"#,
            r#"{"kind":"polygon","id":4,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}],"bulges":[0,0,-0.1]}]}"#,
            r#"{"kind":"arc","id":6,"layerId":"a","attrs":{},"c":{"x":1,"y":2},"r":3,"a0":0.5,"a1":2}"#,
            r#"{"kind":"text","id":11,"layerId":"a","attrs":{},"p":{"x":1,"y":2},"text":"Ada 104","height":2,"rotation":-30}"#,
            r#"{"kind":"dimension","id":12,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":-2,"height":0.5,"text":"12,5 m","style":"linear","angle":90}"#,
            r#"{"kind":"dimension","id":14,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":2,"height":0.5,"style":"angular","c":{"x":1,"y":1}}"#,
            r#"{"kind":"hatch","id":13,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"cross","angle":30,"spacing":0.5}}"#,
            r#"{"kind":"leader","id":15,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":3},{"x":9,"y":3}],"text":"Ø150 PVC","height":2,"rotation":-15,"arrow":"open","mask":true}"#,
            r#"{"kind":"leader","id":16,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":3}],"height":2.5,"rotation":0}"#,
        ];
        for text in objects {
            let e = entity(text);
            let g = edit_geometry(shape(&e)).expect("a geometry");
            assert_eq!(
                serde_json::to_string(&entity_of(&g, e.base().clone())).unwrap(),
                serde_json::to_string(&e).unwrap(),
                "{text}"
            );
        }
        let odd = Shape::Hatch {
            ring: vec![Vec2::new(0.0, 0.0); 3],
            holes: None,
            pattern: HatchPattern::user("dots", 0.0, 1.0),
            assoc: None,
        };
        assert_eq!(edit_geometry(odd), None);
    }

    /// A multi-part area (docs/adr/0143) to the core and back, and through
    /// the edit command's geometry, is itself: every part, its arcs and
    /// holes; a move keeps each vertex's elevation, a part's and its holes'.
    #[test]
    fn a_multi_part_area_goes_to_the_core_and_back_with_its_elevations() {
        let text = r#"{"kind":"polygon","id":4,"layerId":"a","attrs":{"Ada":"104"},"pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"zs":[1,null,3],"parts":[{"pts":[{"x":10,"y":0},{"x":14,"y":0},{"x":14,"y":4},{"x":10,"y":4}],"bulges":[0,0.5,0,0],"holes":[{"pts":[{"x":11,"y":1},{"x":12,"y":1},{"x":12,"y":2}],"zs":[7,8,null]}],"zs":[4,5,6,-0.0]}]}"#;
        let e = entity(text);
        let again = with_shape(&e, shape(&e)).expect("the same kind");
        assert_eq!(
            serde_json::to_string(&again).unwrap(),
            serde_json::to_string(&e).unwrap()
        );
        // Moved 100 m east: every elevation stays with its vertex.
        let moved = kentos_geometry_core::ops::transform::transform_shape(
            &shape(&e),
            &kentos_geometry_core::geom::affine::translation(100.0, 0.0),
        );
        let Some(Entity::Polygon(p)) = with_shape(&e, moved) else {
            panic!("an area");
        };
        let part = &p.parts.as_ref().expect("parts")[0];
        assert_eq!(part.pts[0].x, 110.0);
        assert_eq!(
            part.zs,
            Some(vec![Some(4.0), Some(5.0), Some(6.0), Some(-0.0)])
        );
        assert_eq!(
            part.holes.as_ref().expect("a hole")[0].zs,
            Some(vec![Some(7.0), Some(8.0), None])
        );
        assert_eq!(p.zs, Some(vec![Some(1.0), None, Some(3.0)]));
        // Through the edit command's geometry: the parts come along, their elevations as the geometry says.
        let g = edit_geometry(shape(&e)).expect("a geometry");
        let EntityGeometry::Polygon {
            parts: Some(parts), ..
        } = &g
        else {
            panic!("{g:?}");
        };
        assert_eq!(parts.len(), 1);
        let written = entity_of(&g, e.base().clone());
        let Entity::Polygon(w) = written else {
            panic!()
        };
        assert_eq!(w.parts.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            w.parts.as_ref().unwrap()[0].pts,
            p.parts.as_ref().unwrap()[0]
                .pts
                .iter()
                .map(|q| Point {
                    x: q.x - 100.0,
                    y: q.y
                })
                .collect::<Vec<_>>()
        );
    }
}
