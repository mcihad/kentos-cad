//! `cad.entities.vertex` v1 (docs/adr/0279): Köşeyi sil, Ortasına köşe ekle, Düz kenar yap and Yaya dönüştür on an
//! object named by its persistent id and a place on it. The geometry is the shared core's (`ops::vertex`'s
//! `remove_vertex`, `insert_vertex`; the bulges set here as the grip menu always set them); an area's holes and a
//! multi-part area's other parts stay; with Topolojik düzenleme the neighbours follow (`topology_follow`, as Esnet);
//! the write is `cad.entities.edit`'s operation of the action, one undo step. The shared cases are
//! `fixtures/commands/v1/cad.entities.vertex.json` (`scripts/fixtures/vertex_command_cases.py`).
//!
//! docs/adr/0288 adds Yaya dönüştür through a point (`through`, the core's `bulge_via`) or of a radius (`radius`,
//! `bulge_of_radius`), on a straight or an arc edge, and Kenar genişliği (`width`): a polyline edge's start and end
//! widths, PEDIT's vertex Width, written with the geometry.

use kentos_commands::{
    CommandResult, CommandWarning, EditOperation, EntitiesEdit, EntitiesVertex, EntitiesVertexPlan,
    EntitiesVertexed, EntityEdit, SubPlace, VertexAction,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::{Shape, area_parts, replace_part};
use kentos_geometry_core::geom::bulge::{
    bulge_at, bulge_of_radius, bulge_ring_area, bulge_via, is_arc_bulge, segment_mid,
};
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::offset::path_like;
use kentos_geometry_core::ops::subref::point;
use kentos_geometry_core::ops::vertex::{insert_vertex, remove_vertex};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error, is_uid_text};
use crate::geometry::{edit_geometry, shape};
use crate::{ExecutionContext, codes, edit};

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesVertex) -> CommandResult<()> {
    match check(cx.doc, cx.store, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The objects as execute would write them.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesVertex,
) -> CommandResult<EntitiesVertexPlan> {
    let checked = match check(cx.doc, cx.store, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let Some(change) = checked.edit else {
        return CommandResult::Completed {
            output: EntitiesVertexPlan {
                changed: Vec::new(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        };
    };
    match edit::plan_in(cx.doc, &change) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesVertexPlan {
                changed: output.changed,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => crate::trim::relay(other),
    }
}

/// Edits the vertex or the edge, one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesVertex,
) -> CommandResult<EntitiesVertexed> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ENTITIES_VERTEX,
        kentos_commands::CAD_ENTITIES_VERTEX_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesVertex,
) -> CommandResult<EntitiesVertexed> {
    let checked = match check(cx.doc, cx.store, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let Some(change) = checked.edit else {
        return CommandResult::Completed {
            output: EntitiesVertexed {
                changed: Vec::new(),
                ids: Vec::new(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        };
    };
    match edit::execute(cx, change) {
        CommandResult::Completed { output, warnings } => {
            let ids = output
                .changed
                .iter()
                .filter_map(|u| Uuid::parse_str(u).ok().and_then(|u| cx.doc.slot_of(u)))
                .map(|s| s.0)
                .collect();
            CommandResult::Completed {
                output: EntitiesVertexed {
                    changed: output.changed,
                    ids,
                    revision: output.revision,
                },
                warnings: [checked.warnings, warnings].concat(),
            }
        }
        other => crate::trim::relay(other),
    }
}

/// The undo step's name: the action's.
pub fn label(action: VertexAction) -> &'static str {
    edit::label(operation(action))
}

fn operation(action: VertexAction) -> EditOperation {
    match action {
        VertexAction::Remove => EditOperation::VertexRemove,
        VertexAction::Insert => EditOperation::VertexAdd,
        VertexAction::Straighten => EditOperation::StraightEdge,
        VertexAction::Arc => EditOperation::ArcEdge,
        VertexAction::Width => EditOperation::EdgeWidth,
    }
}

struct Checked {
    /// The edit to write; none when the edge already is as the action would make it.
    edit: Option<EntitiesEdit>,
    warnings: Vec<CommandWarning>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// A path's vertices, its bulges and whether it is closed.
type PathOf<'a> = (&'a [Vec2], Option<&'a [f64]>, bool);

/// A path's vertices and bulges, and whether it is closed.
fn path_of(s: &Shape) -> Option<PathOf<'_>> {
    match s {
        Shape::Polyline {
            pts,
            bulges,
            closed,
            ..
        } => Some((pts, bulges.as_deref(), *closed)),
        Shape::Polygon { pts, bulges, .. } => Some((pts, bulges.as_deref(), true)),
        _ => None,
    }
}

fn check(doc: &Document, store: Option<&Store>, input: &EntitiesVertex) -> Result<Checked, Stop> {
    if !is_uid_text(&input.uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).",
                input.uid
            ),
            "uid",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    options(input)?;
    let Some((slot, e)) = Uuid::parse_str(&input.uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| Some((s, doc.get(s)?)))
    else {
        return Err(refuse(
            codes::ENTITY_NOT_FOUND,
            format!(
                "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                input.uid
            ),
            "uid",
        ));
    };
    let layers = doc.layers();
    let layer = &e.base().layer_id;
    if layers.is_locked(layer) {
        let name = layers
            .get(layer)
            .map_or(layer.as_str(), |n| n.name.as_str());
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
            ),
            "uid",
        ));
    }
    let whole = shape(e);
    if input.action == VertexAction::Width && !matches!(whole, Shape::Polyline { .. }) {
        return Err(refuse(
            codes::NOT_A_POLYLINE,
            "Kenar genişliği yalnız çoklu çizgide olur; alanın ve çizginin genişliği yoktur."
                .into(),
            "uid",
        ));
    }
    let line = matches!(whole, Shape::Line { .. });
    let pathlike = matches!(whole, Shape::Polyline { .. } | Shape::Polygon { .. });
    if !(pathlike || line && input.action == VertexAction::Insert) {
        return Err(refuse(
            codes::NOT_A_PATH,
            "Köşe ve kenar işlemleri çoklu çizgide ve kapalı alanda yapılır (çizgiye yalnız köşe eklenir).".into(),
            "uid",
        ));
    }
    // The place: a vertex for Köşeyi sil, an edge's middle for the others; Köşe ekle takes a place along the edge too.
    let (part, ring, index) = match (input.action, input.place) {
        (VertexAction::Remove, SubPlace::Vertex { part, ring, index }) => (part, ring, index),
        (VertexAction::Remove, _) => {
            return Err(refuse(
                codes::WRONG_PLACE,
                "Köşeyi sil'in yeri bir köşedir (vertex).".into(),
                "place",
            ));
        }
        (_, SubPlace::Mid { part, ring, index }) => (part, ring, index),
        (
            VertexAction::Insert,
            SubPlace::Along {
                part, ring, index, ..
            },
        ) => (part, ring, index),
        _ => {
            return Err(refuse(
                codes::WRONG_PLACE,
                "Köşe eklemenin yeri bir kenarın ortası (mid) ya da kenar üzerinde bir yerdir (along); kenarı düzeltme ve yaya dönüştürmenin yeri kenarın ortasıdır."
                    .into(),
                "place",
            ));
        }
    };
    if ring > 0 {
        return Err(refuse(
            codes::HOLE_EDIT,
            "Deliğin köşe ve kenarları bu işlemle değişmez; deliği tutamacıyla sürükleyin ya da Patlat'la düzenleyin."
                .into(),
            "place.ring",
        ));
    }
    let no_place = || {
        refuse(
            codes::NO_SUCH_PLACE,
            "Nesnede verilen yer yok: parça, köşe ya da kenar sırası nesnenin dışında.".into(),
            "place",
        )
    };
    let parts = area_parts(&whole);
    let s = parts.get(part as usize).cloned().ok_or_else(no_place)?;
    let (index, part) = (index as usize, part as usize);
    // A place along the edge: its point is the core's (`ops::subref::point`, docs/adr/0263 §1).
    let along = match input.place {
        SubPlace::Along { .. } => Some(
            point(
                &crate::blocks::core_entity(e),
                kentos_domain::dimension_ties::core_place(input.place),
            )
            .ok_or_else(no_place)?,
        ),
        _ => None,
    };
    let edited = if line {
        let Shape::Line { a, b } = s else {
            return Err(no_place());
        };
        if index != 0 {
            return Err(no_place());
        }
        let at = along.unwrap_or(Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0));
        geometry(insert_vertex(&s, 0, at))?
    } else {
        let (pts, bulges, closed) = path_of(&s).ok_or_else(no_place)?;
        let n = pts.len();
        let edges = if closed { n } else { n.saturating_sub(1) };
        match input.action {
            VertexAction::Remove => {
                if index >= n {
                    return Err(no_place());
                }
                match remove_vertex(&s, index) {
                    Geometry::Ok(e) => e.shape,
                    Geometry::Error(why) => {
                        let code = if closed {
                            codes::TOO_FEW_CORNERS
                        } else {
                            codes::TOO_FEW_POINTS
                        };
                        return Err(refuse(code, why, "place"));
                    }
                }
            }
            VertexAction::Insert => {
                if index >= edges {
                    return Err(no_place());
                }
                let at = along.unwrap_or_else(|| {
                    segment_mid(pts[index], pts[(index + 1) % n], bulge_at(bulges, index))
                });
                geometry(insert_vertex(&s, index, at))?
            }
            VertexAction::Width => {
                if index >= edges {
                    return Err(no_place());
                }
                if parts.len() > 1 {
                    return Err(refuse(
                        codes::INVALID_POLYLINE,
                        "Genişlikli çoklu çizgi tek parçalı olmalı; parçalarına ayırıp kenarın parçasında deneyin.".into(),
                        "uid",
                    ));
                }
                let Some(edited) = widened(&s, index, input.widths.unwrap_or_default()) else {
                    return Ok(Checked {
                        edit: None,
                        warnings: Vec::new(),
                    });
                };
                edited
            }
            VertexAction::Arc if input.through.is_some() || input.radius.is_some() => {
                if index >= edges {
                    return Err(no_place());
                }
                let (a, b) = (pts[index], pts[(index + 1) % n]);
                let bulge = match (input.through, input.radius) {
                    (Some(m), _) => bulge_via(a, Vec2::new(m.x, m.y), b).ok_or_else(|| {
                        refuse(
                            codes::INVALID_ARC_EDGE,
                            "Yayın geçeceği nokta kenarın doğrusunda ya da ucunda; doğrunun dışında bir nokta verin.".into(),
                            "through",
                        )
                    })?,
                    (None, Some(r)) => bulge_of_radius(a, b, r).ok_or_else(|| {
                        refuse(
                            codes::INVALID_ARC_EDGE,
                            "Yarıçap kirişin yarısından küçük; en az kirişin yarısı kadar bir yarıçap verin.".into(),
                            "radius",
                        )
                    })?,
                    (None, None) => 0.5,
                };
                if bulge == bulge_at(bulges, index) {
                    return Ok(Checked {
                        edit: None,
                        warnings: Vec::new(),
                    });
                }
                with_bulge(&s, pts, bulges, index, bulge)
            }
            VertexAction::Straighten | VertexAction::Arc => {
                if index >= edges {
                    return Err(no_place());
                }
                let arc = is_arc_bulge(bulge_at(bulges, index));
                let bulge = match input.action {
                    VertexAction::Straighten if !arc => None,
                    VertexAction::Straighten => Some(0.0),
                    _ if arc => None,
                    // Outside of a counter-clockwise ring is right of travel, where a positive bulge bows.
                    _ if closed && bulge_ring_area(pts, bulges) <= 0.0 => Some(-0.5),
                    _ => Some(0.5),
                };
                let Some(bulge) = bulge else {
                    return Ok(Checked {
                        edit: None,
                        warnings: Vec::new(),
                    });
                };
                with_bulge(&s, pts, bulges, index, bulge)
            }
        }
    };
    // The object's geometry stays but for what the action gives: a hole stays, the other parts too.
    let edited = keep_holes(&s, edited);
    let edited = if parts.len() > 1 {
        replace_part(&whole, part, edited).ok_or_else(no_place)?
    } else {
        edited
    };
    let follow = match (input.topology, store) {
        (Some(t), Some(store)) => crate::topology_follow::neighbours(
            doc,
            store,
            t.points,
            &[(slot, whole.clone(), edited.clone())],
        ),
        (Some(t), None) => {
            let own = crate::geometry_store::store_of(doc);
            crate::topology_follow::neighbours(
                doc,
                &own,
                t.points,
                &[(slot, whole.clone(), edited.clone())],
            )
        }
        (None, _) => None,
    };
    let geometry = edit_geometry(edited).ok_or_else(no_place)?;
    let mut changes = vec![EntityEdit::Update {
        uid: input.uid.clone(),
        geometry,
    }];
    let mut warnings = Vec::new();
    if let Some(n) = follow {
        changes.extend(n.changes);
        if n.locked > 0 {
            warnings.push(CommandWarning {
                code: codes::NEIGHBOUR_LOCKED.into(),
                message: format!(
                    "Kilitli katmandaki {} komşu nesne değişmedi; ortak sınır ayrıldı.",
                    n.locked
                ),
                path: None,
            });
        }
        if n.invalid > 0 {
            warnings.push(CommandWarning {
                code: codes::NEIGHBOUR_INVALID.into(),
                message: format!(
                    "{} komşu nesne geçersiz kalacağı için değişmedi (açık yolda 2'den, halkada 3'ten az köşe).",
                    n.invalid
                ),
                path: None,
            });
        }
    }
    Ok(Checked {
        edit: Some(EntitiesEdit {
            operation: operation(input.action),
            changes,
            expected_revision: input.expected_revision.clone(),
        }),
        warnings,
    })
}

/// docs/adr/0288's options, before the object is looked up: Yaya dönüştür's point or radius only with it and not
/// together, finite, a radius not zero; Kenar genişliği's widths only with it, given, zero or more and finite.
fn options(input: &EntitiesVertex) -> Result<(), Stop> {
    let arc =
        |message: &str, path: &str| Err(refuse(codes::INVALID_ARC_EDGE, message.into(), path));
    let wide = |message: &str| Err(refuse(codes::INVALID_POLYLINE, message.into(), "widths"));
    if input.action != VertexAction::Arc {
        if input.through.is_some() {
            return arc(
                "Yayın geçeceği nokta yalnız Yaya dönüştür'de (arc) verilir.",
                "through",
            );
        }
        if input.radius.is_some() {
            return arc("Yarıçap yalnız Yaya dönüştür'de (arc) verilir.", "radius");
        }
    }
    if input.through.is_some() && input.radius.is_some() {
        return arc(
            "Yay ya bir noktadan ya da bir yarıçapla verilir; ikisi birlikte verilmez.",
            "radius",
        );
    }
    if let Some(m) = input.through
        && !(m.x.is_finite() && m.y.is_finite())
    {
        return arc(
            "Yayın geçeceği noktanın koordinatları sayı olmalı.",
            "through",
        );
    }
    if let Some(r) = input.radius
        && (!r.is_finite() || r == 0.0)
    {
        return arc(
            "Yarıçap sıfırdan farklı bir sayı olmalı (eksi: kenarın soluna).",
            "radius",
        );
    }
    match (input.action, input.widths) {
        (VertexAction::Width, None) => {
            wide("Kenar genişliği başlangıç ve bitiş genişlikleriyle (widths) verilir.")
        }
        (VertexAction::Width, Some([s, e]))
            if !(s.is_finite() && e.is_finite() && s >= 0.0 && e >= 0.0) =>
        {
            wide("Genişlikler sıfır ya da büyük sayılar olmalı.")
        }
        (VertexAction::Width, _) | (_, None) => Ok(()),
        (_, Some(_)) => wide("Genişlikler yalnız Kenar genişliği'nde (width) verilir."),
    }
}

/// The polyline with edge `segment`'s widths set (one pair a vertex, the closing edge's last); none when they are
/// already so. Widths all zero are none.
fn widened(s: &Shape, segment: usize, pair: [f64; 2]) -> Option<Shape> {
    let Shape::Polyline {
        pts,
        bulges,
        closed,
        widths,
        ..
    } = s
    else {
        return None;
    };
    let mut all: Vec<[f64; 2]> = widths
        .clone()
        .unwrap_or_else(|| vec![[0.0, 0.0]; pts.len()]);
    all.resize(pts.len(), [0.0, 0.0]);
    if all[segment] == pair {
        return None;
    }
    all[segment] = pair;
    let widths = all
        .iter()
        .any(|[a, b]| *a != 0.0 || *b != 0.0)
        .then_some(all);
    Some(Shape::Polyline {
        pts: pts.clone(),
        bulges: bulges.clone(),
        holes: None,
        parts: None,
        closed: *closed,
        widths,
    })
}

/// The core's answer as a shape; its refusal as `no_such_place` (a place that is not there, or one at a vertex:
/// a vertex goes inside an edge).
fn geometry(g: Result<Geometry, String>) -> Result<Shape, Stop> {
    match g {
        Ok(Geometry::Ok(e)) => Ok(e.shape),
        Ok(Geometry::Error(why)) | Err(why) => Err(refuse(codes::NO_SUCH_PLACE, why, "place")),
    }
}

/// The path with edge `segment` bent to `bulge`; its bulges kept only while one is an arc; of `like`'s kind.
fn with_bulge(
    like: &Shape,
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    segment: usize,
    bulge: f64,
) -> Shape {
    let all: Vec<f64> = (0..pts.len())
        .map(|i| {
            if i == segment {
                bulge
            } else {
                bulge_at(bulges, i)
            }
        })
        .collect();
    let bulges = all.iter().any(|&b| is_arc_bulge(b)).then_some(all);
    path_like(like, pts.to_vec(), bulges)
}

/// An area's holes kept on its edited ring.
fn keep_holes(before: &Shape, edited: Shape) -> Shape {
    match (before, edited) {
        (
            Shape::Polygon {
                holes: Some(holes), ..
            },
            Shape::Polygon { pts, bulges, .. },
        ) => Shape::Polygon {
            pts,
            bulges,
            holes: Some(holes.clone()),
            parts: None,
        },
        (_, edited) => edited,
    }
}
