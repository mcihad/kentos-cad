//! An object built from its read fields: its geometry, rasters and point clouds with their styles and sources,
//! and a table's cells and source.

use super::*;

pub(super) fn build(
    r: &mut Reader<'_>,
    kind: Kind,
    index: usize,
    f: &mut Fields,
) -> Result<Entity, KcadError> {
    // The slot a reader gives: 1, 2, 3 … in file order (§6.6). A payload holds far fewer than 2³² objects.
    let slot = u32::try_from(index + 1).unwrap_or(u32::MAX);
    let look = std::mem::take(&mut f.object_look);
    if let Some(problem) = kentos_contracts::look_problem(
        &look,
        f.color.is_some(),
        f.line_weight.is_some(),
        f.uid.is_none(),
    ) {
        return Err(r.fail(Code::BadValue, &problem));
    }
    if look.wipeout.is_some() && !matches!(kind, Kind::Polygon) {
        let name = KINDS
            .iter()
            .find(|(_, k)| *k == kind)
            .map_or("?", |(n, _)| n);
        if let Some(problem) = kentos_contracts::wipeout_problem(&look, name) {
            return Err(r.fail(Code::BadValue, &problem));
        }
    }
    let base = EntityBase {
        look,
        id: slot,
        layer_id: required(r, f.layer_id.take(), "layerId")?,
        color: f.color.take(),
        attrs: required(r, f.attrs.take(), "attrs")?,
        label: f.label.take(),
        symbol: f.symbol.take(),
        line_weight: f.line_weight.take(),
        label_pins: f.label_pins.take().unwrap_or_default(),
        attachments: f.attachments.take().unwrap_or_default(),
    };
    Ok(match kind {
        Kind::Point => Entity::Point(PointEntity {
            base,
            p: required(r, f.p, "p")?,
            z: f.z,
            parts: f.point_parts.take(),
        }),
        Kind::Line => Entity::Line(LineEntity {
            base,
            a: required(r, f.a, "a")?,
            b: required(r, f.b, "b")?,
            za: f.za,
            zb: f.zb,
            ma: f.ma,
            mb: f.mb,
        }),
        Kind::Polyline | Kind::Polygon => {
            let pts = required(r, f.pts.take(), "pts")?;
            let zs = as_long_as(r, f.zs.take(), f.zs_at, pts.len())?;
            let ms = measures_as_long_as(r, f.ms.take(), f.ms_at, pts.len())?;
            let path = PathEntity {
                space_separator: f.space_separator.take(),
                base,
                pts,
                bulges: f.bulges.take(),
                holes: f.rings.take(),
                zs,
                ms,
                parts: f.parts.take(),
                closed: f.closed.take().unwrap_or(false),
                widths: f.widths.take(),
            };
            if kind == Kind::Polygon {
                Entity::Polygon(path)
            } else {
                Entity::Polyline(path)
            }
        }
        Kind::Circle => Entity::Circle(CircleEntity {
            base,
            c: required(r, f.c, "c")?,
            r: required(r, f.r, "r")?,
        }),
        Kind::Arc => Entity::Arc(ArcEntity {
            base,
            c: required(r, f.c, "c")?,
            r: required(r, f.r, "r")?,
            a0: required(r, f.a0, "a0")?,
            a1: required(r, f.a1, "a1")?,
        }),
        Kind::Ellipse => Entity::Ellipse(EllipseEntity {
            base,
            c: required(r, f.c, "c")?,
            major: required(r, f.major, "major")?,
            ratio: required(r, f.ratio, "ratio")?,
            t0: required(r, f.t0, "t0")?,
            t1: required(r, f.t1, "t1")?,
        }),
        Kind::Spline => Entity::Spline(SplineEntity {
            base,
            pts: required(r, f.pts.take(), "pts")?,
            closed: required(r, f.closed, "closed")?,
        }),
        Kind::Nurbs => {
            let e = kentos_contracts::NurbsEntity {
                base,
                degree: required(r, f.degree, "degree")?,
                ctrl: required(r, f.ctrl.take(), "ctrl")?,
                knots: required(r, f.knots.take(), "knots")?,
                weights: f.weights.take(),
                closed: f.closed.unwrap_or(false),
                periodic: f.periodic,
                fit: f.fit.take(),
                tangents: f.tangents.take(),
                fit_tolerance: f.fit_tolerance,
            };
            if let Some(problem) = e.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Nurbs(e)
        }
        Kind::Wall => {
            let offset = f.offset.unwrap_or(0.0);
            let e = kentos_contracts::WallEntity {
                base,
                wall: kentos_contracts::WallFields {
                    pts: required(r, f.pts.take(), "pts")?,
                    bulges: f.bulges.take(),
                    closed: f.closed.unwrap_or(false),
                    width: required(r, f.width, "width")?,
                    justify: required(r, f.wall_justify, "justify")?,
                    offset,
                    style: f.wall_style.take(),
                    components: std::mem::take(&mut f.wall_components),
                    caps: f.wall_caps.unwrap_or_default(),
                    fill: f.wall_fill.take(),
                    height: f.height,
                    arch: f.wall_arch.take().unwrap_or_default(),
                    joins: f.wall_joins.take().unwrap_or_default(),
                    cleanup: f.wall_cleanup.unwrap_or(true),
                },
            };
            if f.offset == Some(0.0) {
                return Err(r.fail(Code::BadValue, "duvarın ötelemesi 0 iken yazılmaz"));
            }
            if f.closed == Some(false) {
                return Err(r.fail(
                    Code::BadValue,
                    "closed false yazılmaz; alan yalnız true iken yazılır",
                ));
            }
            if let Some((_, problem)) = e.wall.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Wall(e)
        }
        Kind::Grid => {
            let grid = required(r, f.grid_line.take(), "grid")?;
            if let Some((_, problem)) = grid.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Grid(kentos_contracts::GridEntity { base, grid })
        }
        Kind::Column => {
            let column = required(r, f.column.take(), "column")?;
            if let Some((_, problem)) = column.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Column(kentos_contracts::ColumnEntity { base, column })
        }
        Kind::Stair => {
            let stair = required(r, f.stair.take(), "stair")?;
            if let Some((_, problem)) = stair.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Stair(kentos_contracts::StairEntity { base, stair })
        }
        Kind::Ramp => {
            let ramp = required(r, f.ramp.take(), "ramp")?;
            if let Some((_, problem)) = ramp.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Ramp(kentos_contracts::RampEntity { base, ramp })
        }
        Kind::Tag => {
            let tag = required(r, f.tag.take(), "tag")?;
            if let Some((_, problem)) = tag.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Tag(kentos_contracts::TagEntity { base, tag })
        }
        Kind::ViewMark => {
            let mark = required(r, f.viewmark.take(), "viewmark")?;
            if let Some((_, problem)) = mark.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::ViewMark(kentos_contracts::ViewMarkEntity { base, mark })
        }
        Kind::Slab => {
            let slab = required(r, f.slab.take(), "slab")?;
            if let Some((_, problem)) = slab.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Slab(kentos_contracts::SlabEntity { base, slab })
        }
        Kind::Beam => {
            let beam = required(r, f.beam.take(), "beam")?;
            if let Some((_, problem)) = beam.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Beam(kentos_contracts::BeamEntity { base, beam })
        }
        Kind::Roof => {
            let roof = required(r, f.roof.take(), "roof")?;
            if let Some((_, problem)) = roof.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Roof(kentos_contracts::RoofEntity { base, roof })
        }
        Kind::Railing => {
            let railing = required(r, f.railing.take(), "railing")?;
            if let Some((_, problem)) = railing.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Railing(kentos_contracts::RailingEntity { base, railing })
        }
        Kind::Space => {
            let rest = required(r, f.space.take(), "space")?;
            let fields = rest.with(required(r, f.space_parts.take(), "parts")?);
            if let Some((_, problem)) = fields.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Space(kentos_contracts::SpaceEntity {
                base,
                space: fields,
            })
        }
        Kind::Opening => {
            let rest = required(r, f.opening.take(), "opening")?;
            let fields = rest.with(
                required(r, f.opening_host.take(), "host")?,
                required(r, f.opening_at, "at")?,
                required(r, f.width, "width")?,
                required(r, f.opening_place.take(), "place")?,
            );
            if let Some((_, problem)) = fields.problem() {
                return Err(r.fail(Code::BadValue, &problem));
            }
            Entity::Opening(kentos_contracts::OpeningEntity {
                base,
                opening: fields,
            })
        }
        Kind::Xline | Kind::Ray => {
            let line = ConstructionEntity {
                base,
                p: required(r, f.p, "p")?,
                dir: required(r, f.dir, "dir")?,
            };
            if kind == Kind::Xline {
                Entity::Xline(line)
            } else {
                Entity::Ray(line)
            }
        }
        Kind::Text => {
            // A linked text names its object and its scale together (docs/adr/0175 §4).
            if f.label_of.is_some() != f.label_scale.is_some() {
                return Err(r.fail_at(
                    Code::BadValue,
                    f.link_at,
                    "bağlı yazının nesnesi ve ölçeği birlikte verilir",
                ));
            }
            let text = required(r, f.text.take(), "text")?;
            // The runs inside the text, in order, apart and one per format (docs/adr/0182 §1).
            if let Some(words) = f.paragraph.runs_problem(&text) {
                return Err(r.fail_at(Code::BadValue, f.runs_at, &words));
            }
            // Its layout: in its height's range, columns only in a box; one spelling, no empty layout (docs/adr/0324 §1).
            if let Some(l) = &f.paragraph.layout {
                if l.is_plain() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        f.layout_at,
                        "boş yazı düzeni yazılmaz; düzensiz yazıda alan yoktur",
                    ));
                }
                if let Some((_, words)) = l.problem(f.paragraph.box_width.is_some()) {
                    return Err(r.fail_at(Code::BadValue, f.layout_at, &words));
                }
            }
            // Its fields: in its letters, in order, their sources and formats right (docs/adr/0325 §1).
            if let Some(words) = kentos_contracts::text_fields_problem(&f.paragraph.fields, &text) {
                return Err(r.fail_at(Code::BadValue, f.fields_at, &words));
            }
            // Bold, italic and a slant need a typeface (docs/adr/0183 §2).
            if let Some((_, words)) = f.face.problem() {
                return Err(r.fail_at(Code::BadValue, f.face_at, &words));
            }
            // Its curve: a text of one line, not linked (docs/adr/0196 §1).
            if let Some(path) = &f.text_path
                && let Some(words) = kentos_contracts::text_path_problem(
                    path,
                    &text,
                    &f.paragraph,
                    f.label_of.is_some(),
                )
            {
                return Err(r.fail_at(Code::BadValue, f.path_at, &words));
            }
            Entity::Text(TextEntity {
                annotative: f.annotative.take(),
                base,
                p: required(r, f.p, "p")?,
                text,
                height: required(r, f.height, "height")?,
                rotation: required(r, f.rotation, "rotation")?,
                align: f.align,
                width_factor: f.width_factor,
                mask: f.mask.unwrap_or(false),
                label_of: f.label_of,
                label_scale: f.label_scale,
                paragraph: std::mem::take(&mut f.paragraph),
                face: std::mem::take(&mut f.face),
                path: f.text_path.take(),
            })
        }
        Kind::Dimension => {
            // What a style needs, and what only a slope has (docs/adr/0147).
            let style = f.style;
            let c = if matches!(
                style,
                Some(DimensionStyle::ArcLength | DimensionStyle::Jogged)
            ) {
                Some(required(r, f.c, "c")?)
            } else {
                f.c
            };
            let (za, zb) = if style == Some(DimensionStyle::Slope) {
                (
                    Some(required(r, f.za, "za")?),
                    Some(required(r, f.zb, "zb")?),
                )
            } else if style == Some(DimensionStyle::Elevation) {
                // A level's one elevation (docs/adr/0322).
                if f.zb.is_some() {
                    return Err(field_fail(
                        r,
                        "zb",
                        f.zb_at,
                        "kot ölçüsünün yalnız bir kotu (za) olur",
                    ));
                }
                (Some(required(r, f.za, "za")?), None)
            } else if f.za.is_some() || f.zb.is_some() {
                let (key, at) = if f.za.is_some() {
                    ("za", f.za_at)
                } else {
                    ("zb", f.zb_at)
                };
                return Err(field_fail(
                    r,
                    key,
                    at,
                    "kot (za, zb) yalnız eğim ve kot ölçüsünde yazılır",
                ));
            } else {
                (None, None)
            };
            if style == Some(DimensionStyle::Ordinate)
                && let Some(a) = f.angle
                && a != 0.0
                && a != 90.0
            {
                return Err(field_fail(
                    r,
                    "angle",
                    f.angle_at,
                    &format!("koordinat ölçüsünün ekseni {a}; 0 (Y) ya da 90 (X) olmalı"),
                ));
            }
            // A level is a Kot's only (docs/adr/0721 §2).
            if let Some((_, at)) = &f.dimension_level
                && style != Some(DimensionStyle::Elevation)
            {
                return Err(r.fail_at(Code::BadValue, *at, "katı yalnız kot ölçüsü anar"));
            }
            Entity::Dimension(DimensionEntity {
                annotative: f.annotative.take(),
                text_shift: f.text_shift.take(),
                breaks: std::mem::take(&mut f.breaks),
                base,
                a: required(r, f.a, "a")?,
                b: required(r, f.b, "b")?,
                offset: required(r, f.offset, "offset")?,
                height: required(r, f.height, "height")?,
                text: f.text.take(),
                style,
                angle: f.angle,
                c,
                mask: f.mask.unwrap_or(false),
                za,
                zb,
                refs: f.refs.take().map(Box::new),
                look: std::mem::take(&mut f.look),
                level: f.dimension_level.take().map(|(l, _)| Box::new(l)),
            })
        }
        Kind::Hatch => Entity::Hatch(HatchEntity {
            base,
            ring: required(r, f.ring.take(), "ring")?,
            holes: f.loops.take(),
            pattern: required(r, f.pattern.take(), "pattern")?,
            assoc: f.assoc.take(),
            boundary: f.hatch_boundary.take(),
            annotative: f.annotative.take(),
        }),
        Kind::Insert => {
            let scale = required(r, f.scale, "scale")?;
            // One way to write a block: the y factor only when it is not the x one.
            if let Some((y, at)) = f.y_scale
                && y == scale
            {
                return Err(r.fail_at(Code::BadValue, at, "y ölçeği x ölçeğine eşitse yazılmaz"));
            }
            Entity::Insert(InsertEntity {
                clipping: f.clipping.take(),
                arch: f.wall_arch.take().unwrap_or_default(),
                parameters: f.parameters.take().unwrap_or_default(),
                attribute_places: f.attribute_places.take().unwrap_or_default(),
                base,
                block: required(r, f.block, "block")?,
                p: required(r, f.p, "p")?,
                scale,
                y_scale: f.y_scale.map(|(y, _)| y),
                rotation: required(r, f.rotation, "rotation")?,
                mirror: f.mirror.unwrap_or(false),
            })
        }
        Kind::Leader => {
            let leader = LeaderEntity {
                tip: f.leader_tip.take(),
                leader_style: f.leader_style.take(),
                blocks: std::mem::take(&mut f.leader_blocks),
                arms: std::mem::take(&mut f.leader_arms),
                annotative: f.annotative.take(),
                base,
                pts: required(r, f.pts.take(), "pts")?,
                text: f.text.take(),
                height: required(r, f.height, "height")?,
                rotation: required(r, f.rotation, "rotation")?,
                arrow: f.arrow,
                arrow_size: f.arrow_size,
                mask: f.mask.unwrap_or(false),
            };
            if let Some((_, words)) = kentos_contracts::leader_problem(&leader) {
                return Err(r.fail_at(Code::BadValue, f.leader_at, &words));
            }
            Entity::Leader(leader)
        }
        Kind::Table => {
            // Bold, italic and a slant need a typeface (docs/adr/0183 §2).
            if let Some((_, words)) = f.face.problem() {
                return Err(r.fail_at(Code::BadValue, f.face_at, &words));
            }
            let mut source = f.source.take();
            if let Some(link) = f.table_link.take() {
                match &mut source {
                    Some(TableSource::File {
                        path,
                        digest,
                        copy,
                        overrides,
                        key_column,
                        ..
                    }) => {
                        *path = link.path;
                        *digest = link.digest;
                        *copy = link.copy;
                        *overrides = link.overrides;
                        *key_column = link.key_column;
                    }
                    _ => {
                        return Err(r.fail_at(
                            Code::BadValue,
                            f.table_at.source,
                            "bağ yalnız dosyadan gelen tablonun kaynağıyla yazılır",
                        ));
                    }
                }
            }
            let table = TableEntity {
                styles: std::mem::take(&mut f.table_styles),
                formulas: std::mem::take(&mut f.table_formulas),
                fields: std::mem::take(&mut f.table_fields),
                base,
                p: required(r, f.p, "p")?,
                rotation: required(r, f.rotation, "rotation")?,
                height: required(r, f.height, "height")?,
                rows: required(r, f.rows.take(), "rows")?,
                columns: required(r, f.columns.take(), "columns")?,
                cells: required(r, f.cells.take(), "cells")?,
                merges: f.merges.take().unwrap_or_default(),
                aligns: f.aligns.take(),
                header: f.header.unwrap_or(false),
                grid: f.grid,
                frame: f.frame,
                face: std::mem::take(&mut f.face),
                source,
                breaks: f.table_breaks.take(),
                table_style: f.table_style.take(),
            };
            // Its rows, columns, cells and ranges hold together (docs/adr/0184 §1).
            if let Some((field, words)) = table.problem() {
                return Err(field_fail(r, field, f.table_at.of(field), &words));
            }
            Entity::Table(table)
        }
        Kind::Image => {
            let image = ImageFields {
                p: required(r, f.p, "p")?,
                width: required(r, f.width, "width")?,
                height: required(r, f.height, "height")?,
                rotation: required(r, f.rotation, "rotation")?,
                mirror: f.mirror.unwrap_or(false),
                asset: f.asset.take(),
                file: f.file.take(),
                clip: f.clip.take(),
                opacity: f.opacity,
            };
            // Its size, its one source, its clip and opacity hold together (docs/adr/0192 §1).
            if let Some(words) = image.problem() {
                return Err(r.fail(Code::BadValue, &words));
            }
            Entity::Image(ImageEntity { base, image })
        }
        Kind::Raster => {
            let raster = RasterFields {
                affine: required(r, f.raster.affine, "affine")?,
                width: required(r, f.raster.width, "width")?,
                height: required(r, f.raster.height, "height")?,
                bands: required(r, f.raster.bands, "bands")?,
                sample: required(r, f.raster.sample, "sample")?,
                asset: f.asset.take(),
                file: f.file.take(),
                url: f.url.take(),
                srid: required(r, f.raster.srid, "srid")?,
                style: required(r, f.raster.style.take(), "style")?,
                opacity: f.opacity,
                dataset: f.raster.dataset.take(),
                georef: f.raster.georef.take(),
                tiles: f.raster.tiles.take(),
            };
            // Its place, size, bands, one source, look and opacity hold together (docs/adr/0204 §2).
            if let Some(words) = raster.problem() {
                return Err(r.fail(Code::BadValue, &words));
            }
            Entity::Raster(RasterEntity { base, raster })
        }
        Kind::PointCloud => {
            let cloud = PointCloudFields {
                sources: required(r, f.cloud.sources.take(), "sources")?,
                bounds: required(r, f.cloud.bounds, "bounds")?,
                count: required(r, f.cloud.count, "count")?,
                srid: required(r, f.raster.srid, "srid")?,
                style: required(r, f.cloud.style.take(), "style")?,
                opacity: f.opacity,
            };
            // Its files, bounds, points, look and opacity hold together (docs/adr/0207 §3).
            if let Some(words) = cloud.problem() {
                return Err(r.fail(Code::BadValue, &words));
            }
            Entity::PointCloud(PointCloudEntity { base, cloud })
        }
        Kind::Record => Entity::Record(RecordEntity { base }),
    })
}

/// Six numbers: a bounds' `[x₁, y₁, z₁, x₂, y₂, z₂]`.
pub(super) fn bounds6(r: &mut Reader<'_>) -> Result<[f64; 6], KcadError> {
    let at = r.position();
    let v = floats(r)?;
    v.as_slice().try_into().map_err(|_| {
        r.fail_at(
            Code::BadValue,
            at,
            &format!("kapsam 6 sayı olmalı, {} var", v.len()),
        )
    })
}

/// Sixteen numbers: a cloud file's place, row by row (schema 251, docs/adr/5100).
pub(super) fn pose16(r: &mut Reader<'_>) -> Result<[f64; 16], KcadError> {
    let at = r.position();
    let v = floats(r)?;
    v.as_slice().try_into().map_err(|_| {
        r.fail_at(Code::BadValue, at, &format!("konum 16 sayı olmalı, {} var", v.len()))
    })
}

/// A point cloud's files (§6.6, docs/adr/0207 §3): each its one source, format, points and bounds; from schema 251
/// its place (docs/adr/5100).
pub(super) fn cloud_sources(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Vec<CloudSource>, KcadError> {
    list(r, |r, _| {
        let (mut asset, mut file, mut url) = (None, None, None);
        let (mut format, mut count, mut bounds, mut pose) = (None, None, None, None);
        map(r, |r, key| {
            match key {
                "asset" => asset = Some(text(r)?),
                "file" => file = Some(text(r)?),
                "url" => url = Some(text(r)?),
                "format" => {
                    let names: Vec<(&str, CloudFormat)> =
                        CloudFormat::ALL.iter().map(|v| (v.name(), *v)).collect();
                    format = Some(named(r, &names)?);
                }
                "count" => count = Some(r.uint(u64::MAX)?),
                "bounds" => bounds = Some(bounds6(r)?),
                // Schema 251 (docs/adr/5100): the file's place in the drawing, 16 numbers row by row.
                "pose" if has.cloud_poses => pose = Some(pose16(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(CloudSource {
            asset,
            file,
            url,
            format: required(r, format, "format")?,
            count: required(r, count, "count")?,
            bounds: required(r, bounds, "bounds")?,
            pose,
        })
    })
}

/// A point cloud's look (§6.6, docs/adr/0207 §5): `render` and `size` always,
/// the rest only when not their default (one spelling: no `invert` or `rgb8`
/// false, no `sizeUnit` "px", no `shape` "round", no empty `hidden`).
pub(super) fn cloud_style(r: &mut Reader<'_>) -> Result<PointCloudStyle, KcadError> {
    let (mut render, mut size) = (None, None);
    let mut st = PointCloudStyle {
        render: CloudRender::Rgb,
        ramp: None,
        invert: false,
        min: None,
        max: None,
        hidden: Vec::new(),
        rgb8: false,
        size: 0.0,
        size_unit: PointSizeUnit::Px,
        shape: PointShape::Round,
    };
    let only_true = |r: &mut Reader<'_>, words: &str| -> Result<bool, KcadError> {
        let at = r.position();
        if !r.bool()? {
            return Err(r.fail_at(Code::BadValue, at, words));
        }
        Ok(true)
    };
    map(r, |r, key| {
        match key {
            "render" => {
                let names: Vec<(&str, CloudRender)> =
                    CloudRender::ALL.iter().map(|v| (v.name(), *v)).collect();
                render = Some(named(r, &names)?);
            }
            "size" => size = Some(r.float()?),
            "ramp" => st.ramp = Some(text(r)?),
            "min" => st.min = Some(r.float()?),
            "max" => st.max = Some(r.float()?),
            "invert" => {
                st.invert = only_true(
                    r,
                    "ters çevirme false yazılmaz; çevrilmemiş rampada alan yoktur",
                )?;
            }
            "rgb8" => {
                st.rgb8 = only_true(r, "rgb8 false yazılmaz; 16 bitlik renklerde alan yoktur")?;
            }
            "hidden" => {
                let at = r.position();
                let v = list(r, |r, _| Ok(r.uint(255)? as u8))?;
                if v.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "gizlenen sınıf listesi boş; gizlenen sınıf yoksa alan yazılmaz",
                    ));
                }
                st.hidden = v;
            }
            "sizeUnit" => {
                let at = r.position();
                let names: Vec<(&str, PointSizeUnit)> =
                    PointSizeUnit::ALL.iter().map(|v| (v.name(), *v)).collect();
                st.size_unit = named(r, &names)?;
                if st.size_unit == PointSizeUnit::Px {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boyun birimi “px” yazılmaz; piksel boyda alan yoktur",
                    ));
                }
            }
            "shape" => {
                let at = r.position();
                let names: Vec<(&str, PointShape)> =
                    PointShape::ALL.iter().map(|v| (v.name(), *v)).collect();
                st.shape = named(r, &names)?;
                if st.shape == PointShape::Round {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "biçim “round” yazılmaz; yuvarlak noktada alan yoktur",
                    ));
                }
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    st.render = required(r, render, "render")?;
    st.size = required(r, size, "size")?;
    Ok(st)
}

/// A raster's look (§6.6, docs/adr/0204 §4): `render` and `bands` always, the
/// rest only when not their default (one spelling: no `stretch` "none", no
/// `resampling` "bilinear", no `invert` false).
pub(super) fn raster_style(r: &mut Reader<'_>, multidim: bool) -> Result<RasterStyle, KcadError> {
    let (mut render, mut bands) = (None, None);
    let mut st = RasterStyle {
        render: RasterRender::Rgb,
        bands: Vec::new(),
        stretch: RasterStretch::None,
        min: None,
        max: None,
        ramp: None,
        invert: false,
        azimuth: None,
        altitude: None,
        z_factor: None,
        nodata: None,
        resampling: RasterResampling::Bilinear,
        edges: None,
    };
    map(r, |r, key| {
        match key {
            "render" => {
                let names: Vec<(&str, RasterRender)> =
                    RasterRender::ALL.iter().map(|v| (v.name(), *v)).collect();
                render = Some(named(r, &names)?);
            }
            "bands" => {
                bands = Some(list(r, |r, _| {
                    Ok(r.uint(u64::from(MAX_RASTER_BANDS))? as u32)
                })?)
            }
            "stretch" => {
                let at = r.position();
                let names: Vec<(&str, RasterStretch)> =
                    RasterStretch::ALL.iter().map(|v| (v.name(), *v)).collect();
                st.stretch = named(r, &names)?;
                if st.stretch == RasterStretch::None {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "gerdirme “none” yazılmaz; gerdirmesiz görünüşte alan yoktur",
                    ));
                }
            }
            "resampling" => {
                let at = r.position();
                let names: Vec<(&str, RasterResampling)> = RasterResampling::ALL
                    .iter()
                    .map(|v| (v.name(), *v))
                    .collect();
                st.resampling = named(r, &names)?;
                if st.resampling == RasterResampling::Bilinear {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "örnekleme “bilinear” yazılmaz; çift doğrusal görünüşte alan yoktur",
                    ));
                }
            }
            "invert" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "ters çevirme false yazılmaz; çevrilmemiş rampada alan yoktur",
                    ));
                }
                st.invert = true;
            }
            "ramp" => st.ramp = Some(text(r)?),
            "min" => st.min = Some(r.float()?),
            "max" => st.max = Some(r.float()?),
            "azimuth" => st.azimuth = Some(r.float()?),
            "altitude" => st.altitude = Some(r.float()?),
            "zFactor" => st.z_factor = Some(r.float()?),
            "nodata" => st.nodata = Some(r.float()?),
            // Schema 38: a mesh's lines (docs/adr/0243 §5).
            "edges" if multidim => st.edges = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    st.render = required(r, render, "render")?;
    st.bands = required(r, bands, "bands")?;
    Ok(st)
}

/// A raster's NetCDF variable and slice (§6.6, docs/adr/0243 §6): `variable`
/// always; `vector`, `mesh`, `dims` and `followTime` only when given (no empty
/// list, no false).
pub(super) fn raster_dataset(
    r: &mut Reader<'_>,
) -> Result<kentos_contracts::RasterDataset, KcadError> {
    let mut d = kentos_contracts::RasterDataset {
        variable: String::new(),
        vector: None,
        mesh: None,
        dims: Vec::new(),
        follow_time: false,
    };
    let mut variable = None;
    map(r, |r, key| {
        match key {
            "variable" => variable = Some(text(r)?),
            "vector" => d.vector = Some(text(r)?),
            "mesh" => d.mesh = Some(text(r)?),
            "followTime" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(Code::BadValue, at, "followTime false yazılmaz"));
                }
                d.follow_time = true;
            }
            "dims" => {
                let at = r.position();
                d.dims = list(r, |r, _| dataset_dim(r))?;
                if d.dims.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş dilim boyutu listesi yazılmaz"));
                }
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    d.variable = required(r, variable, "variable")?;
    Ok(d)
}

/// A raster's georeference record (§6.6, docs/adr/0518 §1): its JSON, read back exactly as the writer writes it
/// (no default written, integers and floats where the contract has them); the record's own rules hold.
pub(super) fn raster_georef(
    r: &mut Reader<'_>,
    crs: bool,
) -> Result<kentos_contracts::RasterGeoref, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    // Before schema 100 a record has no `crs` (docs/adr/0594 §1).
    if !crs && value.get("crs").is_some() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "oturtma kaydında bilinmeyen alan “crs”: hedeflerin sistemi tanım olarak şema 100'den beri yazılır",
        ));
    }
    let read: kentos_contracts::RasterGeoref =
        serde_json::from_value(value.clone()).map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("oturtma kaydı okunamadı: {words}"))
        })?;
    if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "oturtma kaydı yazıcının biçiminde değil (varsayılan değer ya da tam sayı yerine kesirli sayı yazılmış)",
        ));
    }
    if let Some(why) = read.problem() {
        return Err(r.fail_at(Code::BadValue, at, &why));
    }
    Ok(read)
}

/// A raster's GeoPackage tile table (§6.6, docs/adr/0627 §3): `table`, `zoom`, `x` and `y`, read back exactly as the
/// writer writes them.
pub(super) fn raster_tiles(r: &mut Reader<'_>) -> Result<kentos_contracts::RasterTiles, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let read: kentos_contracts::RasterTiles =
        serde_json::from_value(value.clone()).map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(
                code,
                at,
                &format!("rasterin karo tablosu okunamadı: {words}"),
            )
        })?;
    if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "rasterin karo tablosu yazıcının biçiminde değil (tam sayı yerine kesirli sayı yazılmış)",
        ));
    }
    Ok(read)
}

/// A dataset's slice dimension: `name`, `index`, `values`; `time` and `units` when given.
pub(super) fn dataset_dim(r: &mut Reader<'_>) -> Result<kentos_contracts::DatasetDim, KcadError> {
    let (mut name, mut index, mut values) = (None, None, None);
    let mut dim = kentos_contracts::DatasetDim {
        name: String::new(),
        index: 0,
        values: Vec::new(),
        time: false,
        units: None,
    };
    map(r, |r, key| {
        match key {
            "name" => name = Some(text(r)?),
            "index" => index = Some(r.uint(u64::from(u32::MAX))? as u32),
            "values" => values = Some(floats(r)?),
            "time" => {
                let at = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(Code::BadValue, at, "time false yazılmaz"));
                }
                dim.time = true;
            }
            "units" => dim.units = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    dim.name = required(r, name, "name")?;
    dim.index = required(r, index, "index")?;
    dim.values = required(r, values, "values")?;
    Ok(dim)
}

/// A table's merged range (§6.6): its `row`, `col`, `rows` and `cols`.
pub(super) fn cell_range(r: &mut Reader<'_>) -> Result<CellRange, KcadError> {
    let (mut row, mut col, mut rows, mut cols) = (None, None, None, None);
    let limit = u64::from(u32::MAX);
    map(r, |r, key| {
        let n = Some(r.uint(limit)? as u32);
        match key {
            "row" => row = n,
            "col" => col = n,
            "rows" => rows = n,
            "cols" => cols = n,
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(CellRange {
        row: required(r, row, "row")?,
        col: required(r, col, "col")?,
        rows: required(r, rows, "rows")?,
        cols: required(r, cols, "cols")?,
    })
}

/// Where a table's rows came from (§6.6): its `kind` and the objects' ids,
/// or a file's `name` and `sheet`.
pub(super) fn table_source(r: &mut Reader<'_>) -> Result<TableSource, KcadError> {
    let at = r.position();
    let (mut kind, mut objects, mut name, mut sheet) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "kind" => {
                kind = Some(named(
                    r,
                    &[
                        ("coordinates", 0u8),
                        ("areas", 1),
                        ("attributes", 2),
                        ("file", 3),
                    ],
                )?)
            }
            "objects" => objects = Some(list(r, |r, _| Ok(EntityId(id16(r)?)))?),
            "name" => name = Some(text(r)?),
            "sheet" => sheet = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let kind = required(r, kind, "kind")?;
    if kind == 3 {
        if objects.is_some() {
            return Err(r.fail_at(
                Code::BadValue,
                at,
                "dosyadan gelen tablonun kaynağında nesne yazılmaz",
            ));
        }
        return Ok(TableSource::File {
            path: None,
            overrides: Default::default(),
            key_column: None,
            digest: None,
            copy: None,
            name: required(r, name, "name")?,
            sheet,
        });
    }
    if name.is_some() || sheet.is_some() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "nesnelerden gelen tablonun kaynağında dosya adı yazılmaz",
        ));
    }
    let objects = required(r, objects, "objects")?;
    Ok(match kind {
        0 => TableSource::Coordinates { objects },
        1 => TableSource::Areas { objects },
        _ => TableSource::Attributes { objects },
    })
}
