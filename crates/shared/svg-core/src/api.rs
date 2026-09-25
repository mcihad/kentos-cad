//! The SVG core's operations for the page (docs/adr/0008 “SVG
//! düzenleyicisi”): the editor's own WASM package (`kentos-svg-wasm`) looks
//! a name up here; arguments and results are JSON, as in the geometry
//! core's table. Names are those of the TypeScript functions they replace
//! (`apps/web/src/style/svg/*.ts`).

use kentos_geometry_core::api::Op;
use kentos_geometry_core::api::json::{FromJson, Json, read_field};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::op;

use crate::arrange::{
    TransformSpec, Unit, align_moves, align_reference, anchor_point, copies_of, distribute_moves,
    invertible, mirror_matrix, polar_array, rect_array, rotate_about as rotate_about_point,
    scale_about, skew_about, transform_moves, units_of,
};
use crate::bezier::{
    Cubic, bez, bez_deriv, bez_tangent, cubic_length, flatten_cubic, flatten_sub_path_tol,
    nearest_on_cubic, param_at_distance, ring_signed_area, split_cubic, winding_of,
};
use crate::boolean::{BoolOp, RegionInput, boolean_op, cut_path};
use crate::export::{SvgTextOptions, export_box, png_size, write};
use crate::fit::{fit_one, fit_run};
use crate::import::{ImportOptions, color_usage, doc_from_svg_tree, map_colors};
use crate::model::{
    box_to_box, element_of, regular_polygon, rotate_about, serialize_doc, shape_box, shapes_box,
    to_path, transform_shape,
};
use crate::nodes::{
    Corner, Joined, NodeRef, align_nodes, at, break_at_nodes, corner_at, corner_nodes,
    delete_nodes, delete_segments, distribute_nodes, fillet_distance, fillet_radius,
    insert_mid_nodes, join_ends, move_nodes, node_type_of, refresh_auto, segments_to,
    set_node_type,
};
use crate::ops::{
    Ids, WithIds, boolean_shapes, break_apart, close_shapes, close_sub_path, combine_shapes,
    cut_shapes, offset_shapes, open_shapes, open_sub_path, reverse_shapes, shapes_to_path,
    simplify_shapes, simplify_sub_path, stroke_to_path,
};
use crate::path::{
    Matrix, apply, arc_to_cubics, flatten_sub_path, multiply, parse_path_data, path_data_of,
    sub_paths_box, transform_sub_paths,
};
use crate::shape::{Obj, Pt, SubPath};
use crate::stroke::{Cap, Join, StrokeStyle, offset_region, stroke_outline};
use crate::trace::{Ring, nest_rings, ring_area, simplify_ring};
use crate::values::{
    XmlTree, is_near_black, parse_css, parse_length, read_color, read_paint, read_transform,
    to_user, view_box_of, view_box_transform, with_alpha,
};

/// Sub-paths with the nodes to choose next, or a message (`{ subs, refs } | { error }`).
struct Edited {
    subs: Option<Vec<SubPath>>,
    refs: Option<Vec<NodeRef>>,
    error: Option<String>,
}
kentos_geometry_core::json_struct!(out Edited { subs, refs, error });

impl From<Joined> for Edited {
    fn from(j: Joined) -> Edited {
        match j {
            Joined::Done(subs, refs) => Edited {
                subs: Some(subs),
                refs: Some(refs),
                error: None,
            },
            Joined::Error(e) => Edited {
                subs: None,
                refs: None,
                error: Some(e),
            },
        }
    }
}

/// Sub-paths, or a message (`SubPath[] | { error }`).
enum SubsOr {
    Subs(Vec<SubPath>),
    Error(String),
}

impl kentos_geometry_core::api::json::ToJson for SubsOr {
    fn write_json(&self, out: &mut String) {
        match self {
            SubsOr::Subs(s) => s.write_json(out),
            SubsOr::Error(e) => {
                out.push_str("{\"error\":");
                kentos_geometry_core::api::json::write_str(out, e);
                out.push('}');
            }
        }
    }
}

/// `nestRings`'s answer.
struct NestedJson {
    shapes: Vec<NestedShape>,
    removed: usize,
}
struct NestedShape {
    outer: Ring,
    holes: Vec<Ring>,
}
kentos_geometry_core::json_struct!(out NestedShape { outer, holes });
kentos_geometry_core::json_struct!(out NestedJson { shapes, removed });

struct Page {
    width: f64,
    height: f64,
}
kentos_geometry_core::json_struct!(Page { width, height });

fn field_text(v: &Json, k: &str) -> String {
    match v.get(k) {
        Json::Str(s) => s.clone(),
        _ => String::new(),
    }
}

fn node_index(i: f64) -> Result<usize, String> {
    at(i).ok_or_else(|| "Düğüm yok.".to_string())
}

impl FromJson for RegionInput {
    fn from_json(v: &Json) -> Result<RegionInput, String> {
        Ok(RegionInput {
            subs: read_field(v, "subs")?,
            nonzero: matches!(v.get("fillRule"), Json::Str(r) if r == "nonzero"),
        })
    }
}

fn name(v: &Json) -> &str {
    match v {
        Json::Str(s) => s,
        _ => "",
    }
}

impl FromJson for StrokeStyle {
    fn from_json(v: &Json) -> Result<StrokeStyle, String> {
        Ok(StrokeStyle {
            width: read_field(v, "width")?,
            cap: Cap::named(name(v.get("cap"))),
            join: Join::named(name(v.get("join"))),
            miter_limit: read_field(v, "miterLimit")?,
            dash: read_field(v, "dash")?,
        })
    }
}

/// `flattenCubic`'s answer.
struct Flat {
    pts: Vec<Pt>,
    ts: Vec<f64>,
}
kentos_geometry_core::json_struct!(out Flat { pts, ts });

/// The drawing, as far as the file writer reads it.
struct Doc {
    width: f64,
    height: f64,
    shapes: Vec<Obj>,
}
kentos_geometry_core::json_struct!(Doc {
    width,
    height,
    shapes
});

fn with_ids<F: FnOnce(&mut Ids) -> Result<crate::ops::OpResult, String>>(
    f: F,
) -> Result<WithIds, String> {
    let mut ids = Ids::default();
    let result = f(&mut ids)?;
    Ok(WithIds {
        result,
        ids: ids.count,
    })
}

pub static OPS: &[Op] = &[
    // pathData.ts
    op!("parsePathData", |d: String| parse_path_data(&d)),
    op!("arcToCubics", |x1: f64,
                        y1: f64,
                        rx: f64,
                        ry: f64,
                        rot: f64,
                        large: bool,
                        sweep: bool,
                        x2: f64,
                        y2: f64| {
        arc_to_cubics(x1, y1, rx, ry, rot, large, sweep, x2, y2)
    }),
    op!("pathDataOf", |subs: Vec<SubPath>, digits: Option<f64>| {
        path_data_of(&subs, digits.unwrap_or(3.0))
    }),
    op!("transformSubPaths", |subs: Vec<SubPath>, m: Matrix| {
        transform_sub_paths(&subs, &m)
    }),
    op!("multiply", |m: Matrix, n: Matrix| multiply(&m, &n)),
    op!("apply", |m: Matrix, x: f64, y: f64| apply(&m, x, y)),
    op!("flattenSubPath", |sp: SubPath, steps: Option<usize>| {
        flatten_sub_path(&sp, steps.unwrap_or(16))
    }),
    op!("subPathsBox", |subs: Vec<SubPath>| sub_paths_box(&subs)),
    // bezier.ts
    op!("bez", |c: Cubic, t: f64| bez(&c, t)),
    op!("bezDeriv", |c: Cubic, t: f64| bez_deriv(&c, t)),
    op!("bezTangent", |c: Cubic, t: f64| bez_tangent(&c, t)),
    op!("splitCubic", |c: Cubic, t: f64| {
        let (a, b) = split_cubic(&c, t);
        [a, b]
    }),
    op!("flattenCubic", |c: Cubic, tol: f64| {
        let (pts, ts) = flatten_cubic(&c, tol);
        Flat { pts, ts }
    }),
    op!("flattenSubPathTol", |sp: SubPath, tol: f64| {
        flatten_sub_path_tol(&sp, tol)
    }),
    op!("cubicLength", |c: Cubic,
                        t0: Option<f64>,
                        t1: Option<f64>| {
        cubic_length(&c, t0.unwrap_or(0.0), t1.unwrap_or(1.0))
    }),
    op!("nearestOnCubic", |c: Cubic, p: Pt| nearest_on_cubic(&c, p)),
    op!("paramAtDistance", |c: Cubic, dist: f64, from_end: bool| {
        param_at_distance(&c, dist, from_end)
    }),
    op!("ringSignedArea", |pts: Vec<Pt>| ring_signed_area(&pts)),
    op!("windingOf", |ring: Vec<Pt>, p: Pt| winding_of(&ring, p)),
    // fitCurve.ts
    op!("fitRun", |pts: Vec<Pt>,
                   tol: f64,
                   t1: Option<Pt>,
                   t2: Option<Pt>| fit_run(
        &pts, tol, t1, t2
    )),
    op!("fitOne", |pts: Vec<Pt>, t1: Pt, t2: Pt| fit_one(
        &pts, t1, t2
    )),
    // pathBool.ts
    op!("booleanOp", |op: String, inputs: Vec<RegionInput>| {
        boolean_op(BoolOp::named(&op), &inputs)
    }),
    op!("cutPath", |target: Vec<SubPath>,
                    cutters: Vec<Vec<SubPath>>| {
        cut_path(&target, &cutters)
    }),
    // pathStroke.ts
    op!("strokeOutline", |subs: Vec<SubPath>, st: StrokeStyle| {
        stroke_outline(&subs, &st)
    }),
    op!(
        "offsetRegion",
        |input: RegionInput, d: f64, join: Option<String>| {
            offset_region(&input, d, Join::named(join.as_deref().unwrap_or("round")))
        }
    ),
    // pathOps.ts (shapes that get new ids name them for the page to fill in)
    op!("booleanShapes", |op: String, shapes: Vec<Obj>| with_ids(
        |ids| boolean_shapes(&op, &shapes, ids)
    )),
    op!("cutShapes", |shapes: Vec<Obj>| with_ids(|ids| cut_shapes(
        &shapes, ids
    ))),
    op!("combineShapes", |shapes: Vec<Obj>| with_ids(|ids| {
        combine_shapes(&shapes, ids)
    })),
    op!("breakApart", |shape: Obj, keep_holes: bool| with_ids(
        |ids| break_apart(&shape, keep_holes, ids)
    )),
    op!("shapesToPath", |shapes: Vec<Obj>| with_ids(|_| Ok(
        shapes_to_path(&shapes)
    ))),
    op!("strokeToPath", |shapes: Vec<Obj>| with_ids(|ids| {
        stroke_to_path(&shapes, ids)
    })),
    op!(
        "offsetShapes",
        |shapes: Vec<Obj>, d: f64, join: Option<String>| {
            with_ids(|_| offset_shapes(&shapes, d, Join::named(join.as_deref().unwrap_or("round"))))
        }
    ),
    op!(
        "simplifySubPath",
        |sp: SubPath, tol: f64, corner_deg: Option<f64>| simplify_sub_path(
            &sp,
            tol,
            corner_deg.unwrap_or(35.0)
        )
    ),
    op!("simplifyShapes", |shapes: Vec<Obj>, tol: f64| with_ids(
        |_| simplify_shapes(&shapes, tol)
    )),
    op!("reverseShapes", |shapes: Vec<Obj>| with_ids(|_| {
        reverse_shapes(&shapes)
    })),
    op!("closeSubPath", |sp: SubPath| close_sub_path(&sp)),
    op!("openSubPath", |sp: SubPath| open_sub_path(&sp)),
    op!("closeShapes", |shapes: Vec<Obj>| with_ids(
        |_| close_shapes(&shapes)
    )),
    op!("openShapes", |shapes: Vec<Obj>| with_ids(|_| open_shapes(
        &shapes
    ))),
    // svgModel.ts
    op!("elementOf", |s: Obj, fill: String, stroke: String| {
        element_of(&s, &fill, &stroke)
    }),
    op!("serializeDoc", |doc: Doc| serialize_doc(
        doc.width,
        doc.height,
        &doc.shapes
    )),
    op!("toPath", |s: Obj| to_path(&s)),
    op!("shapeBox", |s: Obj| shape_box(&s)),
    op!("shapesBox", |shapes: Vec<Obj>| shapes_box(&shapes)),
    op!("transformShape", |s: Obj, m: Matrix| transform_shape(
        &s, &m
    )),
    op!("transformShapes", |shapes: Vec<Obj>, m: Matrix| {
        shapes
            .iter()
            .map(|s| transform_shape(s, &m))
            .collect::<Result<Vec<_>, _>>()
    }),
    op!("boxToBox", |a: Bounds, b: Bounds| box_to_box(&a, &b)),
    op!("rotation", |deg: f64, cx: f64, cy: f64| rotate_about(
        deg, cx, cy
    )),
    op!(
        "regularPolygon",
        |cx: f64, cy: f64, r: f64, sides: f64, inner: Option<f64>| regular_polygon(
            cx, cy, r, sides, inner
        )
    ),
    // nodeOps.ts
    op!("nodeTypeOf", |sp: SubPath, i: f64| {
        node_index(i).and_then(|i| node_type_of(&sp, i).ok_or_else(|| "Düğüm yok.".to_string()))
    }),
    op!("refreshAuto", |subs: Vec<SubPath>| {
        let mut subs = subs;
        refresh_auto(&mut subs);
        subs
    }),
    op!("setNodeType", |subs: Vec<SubPath>,
                        refs: Vec<NodeRef>,
                        ty: String| {
        set_node_type(&subs, &refs, &ty)
    }),
    op!("moveNodes", |subs: Vec<SubPath>,
                      refs: Vec<NodeRef>,
                      dx: f64,
                      dy: f64| move_nodes(
        &subs, &refs, dx, dy
    )),
    op!(
        "insertMidNodes",
        |subs: Vec<SubPath>, refs: Vec<NodeRef>| {
            let (subs, refs) = insert_mid_nodes(&subs, &refs);
            Edited {
                subs: Some(subs),
                refs: Some(refs),
                error: None,
            }
        }
    ),
    op!(
        "deleteNodes",
        |subs: Vec<SubPath>, refs: Vec<NodeRef>, keep_shape: Option<bool>| delete_nodes(
            &subs,
            &refs,
            keep_shape.unwrap_or(true)
        )
    ),
    op!("joinEnds", |subs: Vec<SubPath>,
                     refs: Vec<NodeRef>,
                     merge: bool| Edited::from(
        join_ends(&subs, &refs, merge)
    )),
    op!("breakAtNodes", |subs: Vec<SubPath>, refs: Vec<NodeRef>| {
        break_at_nodes(&subs, &refs)
    }),
    op!(
        "deleteSegments",
        |subs: Vec<SubPath>, refs: Vec<NodeRef>| match delete_segments(&subs, &refs) {
            Ok(s) => SubsOr::Subs(s),
            Err(e) => SubsOr::Error(e),
        }
    ),
    op!("segmentsTo", |subs: Vec<SubPath>,
                       refs: Vec<NodeRef>,
                       kind: String| {
        segments_to(&subs, &refs, kind == "line")
    }),
    op!("cornerAt", |sp: SubPath, i: f64| at(i)
        .and_then(|i| corner_at(&sp, i))),
    op!("filletDistance", |c: Corner, r: f64| fillet_distance(&c, r)),
    op!("filletRadius", |c: Corner, d: f64| fillet_radius(&c, d)),
    op!("cornerNodes", |subs: Vec<SubPath>,
                        refs: Vec<NodeRef>,
                        mode: String,
                        size: f64| {
        Edited::from(corner_nodes(&subs, &refs, mode == "fillet", size))
    }),
    op!("alignNodes", |subs: Vec<SubPath>,
                       refs: Vec<NodeRef>,
                       axis: String,
                       to: String| align_nodes(
        &subs, &refs, &axis, &to
    )),
    op!(
        "distributeNodes",
        |subs: Vec<SubPath>, refs: Vec<NodeRef>, axis: String| distribute_nodes(
            &subs, &refs, &axis
        )
    ),
    // arrange.ts
    op!("unitsOf", |shapes: Vec<Obj>, order: Vec<String>| units_of(
        &shapes, &order
    )),
    op!("alignReference", |units: Vec<Unit>,
                           to: String,
                           page: Page| {
        align_reference(&units, &to, page.width, page.height)
    }),
    op!("alignMoves", |units: Vec<Unit>,
                       side: String,
                       to: String,
                       page: Page,
                       as_one: Option<bool>| {
        align_moves(
            &units,
            &side,
            &to,
            page.width,
            page.height,
            as_one.unwrap_or(false),
        )
    }),
    op!("distributeMoves", |units: Vec<Unit>, how: String| {
        distribute_moves(&units, &how)
    }),
    op!("anchorPoint", |b: Bounds, a: String| anchor_point(&b, &a)),
    op!("scaleAbout", |sx: f64, sy: f64, p: Pt| scale_about(
        sx, sy, p
    )),
    op!("skewAbout", |ax: f64, ay: f64, p: Pt| skew_about(ax, ay, p)),
    op!("rotateAbout", |deg: f64, p: Pt, ccw: bool| {
        rotate_about_point(deg, p, ccw)
    }),
    op!(
        "transformMoves",
        |units: Vec<Unit>, spec: TransformSpec, separately: bool| transform_moves(
            &units, &spec, separately
        )
    ),
    op!("invertible", |m: Matrix| invertible(&m)),
    op!("rectArray", |b: Bounds, spec: Json| {
        Ok::<_, String>(rect_array(
            &b,
            read_field(&spec, "rows")?,
            read_field(&spec, "cols")?,
            read_field(&spec, "dx")?,
            read_field(&spec, "dy")?,
            field_text(&spec, "mode") == "gap",
        ))
    }),
    op!("polarArray", |b: Bounds, spec: Json| {
        Ok::<_, String>(polar_array(
            &b,
            read_field(&spec, "count")?,
            read_field(&spec, "angle")?,
            read_field(&spec, "centre")?,
            crate::shape::truthy(spec.get("rotate")),
            crate::shape::truthy(spec.get("ccw")),
        ))
    }),
    op!("mirrorMatrix", |axis: String, p: Pt, deg: Option<f64>| {
        mirror_matrix(&axis, p, deg.unwrap_or(0.0))
    }),
    op!("copiesOf", |shapes: Vec<Obj>, matrices: Vec<Matrix>| {
        let mut ids = Ids::default();
        copies_of(&shapes, &matrices, &mut ids).map(|shapes| Copies {
            shapes,
            ids: ids.count,
        })
    }),
    // trace.ts (the bitmap itself crosses as bytes: kentos-svg-wasm)
    op!("ringArea", |r: Ring| ring_area(&r)),
    op!("nestRings", |rings: Vec<Ring>, min_area: Option<f64>| {
        let n = nest_rings(rings, min_area.unwrap_or(0.0));
        NestedJson {
            shapes: n
                .shapes
                .into_iter()
                .map(|(outer, holes)| NestedShape { outer, holes })
                .collect(),
            removed: n.removed,
        }
    }),
    op!("simplifyRing", |r: Ring, tol: f64| simplify_ring(&r, tol)),
    // svgValues.ts
    op!("readColor", |v: String| read_color(&v)),
    op!("readPaint", |v: Option<String>| read_paint(v.as_deref())),
    op!("readTransform", |v: Option<String>| read_transform(
        v.as_deref()
    )),
    op!(
        "viewBoxTransform",
        |vb: Vec<f64>, w: f64, h: f64, par: Option<String>| {
            view_box_transform(&vb, w, h, par.as_deref().unwrap_or(""))
        }
    ),
    op!("viewBoxOf", |v: Option<String>| view_box_of(v.as_deref())),
    op!("parseLength", |v: Option<String>| parse_length(
        v.as_deref()
    )),
    op!("toUser", |v: Option<String>,
                   percent: f64,
                   em: Option<f64>| to_user(
        v.as_deref(),
        percent,
        em.unwrap_or(16.0)
    )),
    op!("parseCss", |text: String, order_from: Option<f64>| {
        parse_css(&text, order_from.unwrap_or(0.0))
    }),
    op!("isNearBlack", |hex: String| is_near_black(&hex)),
    op!("withAlpha", |hex: String, alpha: f64| with_alpha(
        &hex, alpha
    )),
    // importSvg.ts (the tree crosses flat; new shapes and groups are named for the page to fill in)
    op!("docFromSvgTree", |tree: XmlTree, opts: ImportOptions| {
        doc_from_svg_tree(&tree, &opts)
    }),
    op!("colorUsage", |doc: Obj| {
        let shapes: Vec<Obj> = read_field(&doc_tree(&doc), "shapes")?;
        color_usage(&shapes)
    }),
    op!("mapColors", |doc: Obj,
                      symbol: Option<String>,
                      second: Option<String>| {
        map_colors(&doc, symbol.as_deref(), second.as_deref())
    }),
    // exportSvg.ts (the PNG bytes cross as bytes: kentos-svg-wasm)
    op!("exportBox", |doc: Obj, only: Option<Vec<String>>| {
        let shapes: Vec<Obj> = read_field(&doc_tree(&doc), "shapes")?;
        let only = only.map(|ids| ids.into_iter().collect::<std::collections::HashSet<_>>());
        export_box(doc.num("width"), doc.num("height"), &shapes, only.as_ref())
    }),
    op!("svgText", |doc: Obj, opts: SvgTextOptions| write(
        &doc, &opts, false
    )
    .map(|w| w.text)),
    op!("sourceText", |doc: Obj| write(
        &doc,
        &SvgTextOptions::default(),
        true
    )),
    op!("pngSize", |w: f64, h: f64, spec: Json| {
        let px = match &spec {
            Json::Obj(f) if f.iter().any(|(k, _)| k == "px") => {
                Some(read_field::<f64>(&spec, "px")?)
            }
            _ => None,
        };
        let width_mm = match spec.get("widthMm") {
            Json::Null => None,
            _ => Some(read_field::<f64>(&spec, "widthMm")?),
        };
        Ok::<_, String>(png_size(w, h, px, read_field(&spec, "dpi")?, width_mm))
    }),
];

/// A drawing's fields as a JSON tree (to read its shapes).
fn doc_tree(doc: &Obj) -> Json {
    Json::Obj(
        doc.0
            .iter()
            .filter_map(|(k, v)| v.clone().map(|v| (k.clone(), v)))
            .collect(),
    )
}

/// `copiesOf`'s shapes with the number of ids the page is to make.
struct Copies {
    shapes: Vec<Obj>,
    ids: usize,
}
kentos_geometry_core::json_struct!(out Copies { shapes, ids });

/// The id of an operation by name, or None when this build has no such operation.
pub fn find(name: &str) -> Option<usize> {
    OPS.iter().position(|o| o.name == name)
}

/// Runs operation `id` on JSON arguments.
pub fn run(id: usize, args: &str) -> Result<String, String> {
    match OPS.get(id) {
        Some(op) => (op.run)(args),
        None => Err(format!("SVG çekirdeğinde {id} numaralı işlem yok.")),
    }
}

/// Runs an operation by name (the fixtures).
pub fn run_named(name: &str, args: &str) -> Result<String, String> {
    match find(name) {
        Some(id) => run(id, args),
        None => Err(format!("SVG çekirdeğinde “{name}” işlemi yok.")),
    }
}
