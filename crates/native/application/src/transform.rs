//! `cad.entities.transform` v1 (docs/adr/0037, 0047): objects named by their
//! persistent ids moved, rotated, scaled, mirrored or aligned as one undo
//! step, in place or as copies. The desktop's handler over the native
//! document; the web's is `apps/web/src/product/entitiesTransform.ts`. Both
//! pass the shared cases in `fixtures/commands/v1/cad.entities.transform.json`.
//!
//! The modify tools (Taşı, Kopyala, Döndür, Ölçekle, Aynala, Hizala) make
//! the selection explicit here (TODOS.md CMD-07): they give the selected
//! objects' ids. The geometry is the shared core's, as on the web: the
//! matrix of the transform (`similarity`) and what it does to each kind of
//! object (`transform_shape`); nothing is computed here.
//!
//! Oturt's similarity, affine and projective transforms (docs/adr/0156)
//! are the core's warp (`ops::warp`): what it does to each kind, with the
//! paths' elevations; an object may change its kind (a circle becomes an
//! ellipse or a polyline). Kauçuk levha (docs/adr/0158) is the core's sheet
//! (`ops::rubber`) and its rules (`ops::warp::sheet_shape`): only vertices
//! move, kinds stay, how far a shape bends from its true image is counted.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one id; every id lowercase UUID text with hyphens (in order);
//! 2. the transform's numbers finite, in their order; a scale factor above
//!    zero; a mirror axis with a direction; an alignment's second pair whole,
//!    its points apart from the first pair's; an affine or projective
//!    transform not singular; a rubber sheet's links giving one sheet;
//! 3. the expected revision (every command's, `checks.rs`);
//! 4. every id names an object of the document (in order);
//! 5. not every object on a locked layer (with others, a warning);
//! 6. no point of an object beyond a projective transform's horizon;
//! 7. no coordinate carried past the largest float64 by the transform.
//!
//! Objects on a locked layer are neither changed nor copied. The web's tools
//! used to copy them onto their locked layer; ADR 0037 records the change.

use kentos_contracts::{
    CommandError, CommandResult, CommandWarning, EntitiesTransform, EntitiesTransformPlan,
    EntitiesTransformed, Entity, Transform,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::affine::{Affine, similarity};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::{js_hypot, js_max};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::ops::rubber::{Link, RubberError, Sheet};
use kentos_geometry_core::ops::transform::transform_shape;
use kentos_geometry_core::ops::warp::{CHORD, Warp, sheet_shape, warp_shape};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::elevation;
use crate::geometry::{edit_geometry, entity_of, shape, unlinked, with_shape};

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesTransform) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: each object as it would be, and the
/// revision to expect for exactly that; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesTransform,
) -> CommandResult<EntitiesTransformPlan> {
    let copy = input.copy.unwrap_or(false);
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: EntitiesTransformPlan {
                sources: checked.sources.iter().map(|(_, uid)| uid.clone()).collect(),
                entities: checked
                    .moved
                    .into_iter()
                    .map(|mut e| {
                        // A copy's slot is given when it is written; it writes no object's label.
                        if copy {
                            e.base_mut().id = 0;
                            e = unlinked(e);
                        }
                        e
                    })
                    .collect(),
                locked: checked.locked,
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes it as one undo step named after the tool:
/// the objects changed in place, or their copies added. Into the open
/// transaction or group, if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesTransform,
) -> CommandResult<EntitiesTransformed> {
    let copy = input.copy.unwrap_or(false);
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let label = label(&input.transform, copy);
    let (changed, created) = if copy {
        // A linked text's copy is a text of its own (docs/adr/0175 §4).
        let copies = checked.moved.into_iter().map(unlinked).collect();
        match cx.doc.add_many(copies, label) {
            Ok(slots) => {
                let doc = &*cx.doc;
                let created = slots
                    .iter()
                    .filter_map(|slot| doc.uid(*slot))
                    .map(|uid| uid.to_string())
                    .collect();
                (Vec::new(), created)
            }
            Err(full) => {
                return CommandResult::Failed {
                    error: CommandError {
                        code: codes::SLOTS_EXHAUSTED.into(),
                        message: full.to_string(),
                        path: None,
                        revision: None,
                    },
                };
            }
        }
    } else {
        let changes: Vec<(Slot, Entity)> = checked
            .sources
            .iter()
            .map(|(slot, _)| *slot)
            .zip(checked.moved)
            .collect();
        cx.doc.update_many(changes, label);
        let changed = checked.sources.into_iter().map(|(_, uid)| uid).collect();
        (changed, Vec::new())
    };
    CommandResult::Completed {
        output: EntitiesTransformed {
            changed,
            created,
            locked: checked.locked,
            revision: cx.doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}

/// The undo step's name: the tool's (docs/adr/0037).
pub fn label(transform: &Transform, copy: bool) -> &'static str {
    match transform {
        Transform::Move { .. } if copy => "Kopyala",
        Transform::Move { .. } => "Taşı",
        Transform::Rotate { .. } => "Döndür",
        Transform::Scale { .. } => "Ölçekle",
        Transform::Mirror { .. } => "Aynala",
        Transform::Align { .. } => "Hizala",
        Transform::Similarity { .. } | Transform::Affine { .. } | Transform::Projective { .. } => {
            "Oturt"
        }
        Transform::Rubbersheet { .. } => "Kauçuk levha",
    }
}

/// The affine of a transform, built by the shared core (`similarity`), as
/// the web's handler builds it through WASM. None for numbers the checks
/// would refuse first, and for Oturt's transforms (the core's warp).
pub fn affine(transform: &Transform) -> Option<Affine> {
    match *transform {
        Transform::Similarity { .. }
        | Transform::Affine { .. }
        | Transform::Projective { .. }
        | Transform::Rubbersheet { .. } => None,
        Transform::Move { dx, dy } => similarity("move", &[dx, dy]),
        Transform::Rotate { center, angle } => similarity("rotate", &[center.x, center.y, angle]),
        Transform::Scale { center, factor } => similarity("scale", &[center.x, center.y, factor]),
        Transform::Mirror { a, b } => similarity("mirror", &[a.x, a.y, b.x, b.y]),
        Transform::Align {
            source: s,
            target: t,
            source2,
            target2,
            scale,
        } => match (source2, target2) {
            (Some(s2), Some(t2)) => similarity(
                if scale == Some(true) {
                    "alignScale"
                } else {
                    "align"
                },
                &[s.x, s.y, t.x, t.y, s2.x, s2.y, t2.x, t2.y],
            ),
            (None, None) => similarity("align", &[s.x, s.y, t.x, t.y]),
            _ => None,
        },
    }
}

/// An alignment's own checks after its numbers (`invalid_align`): the second
/// pair whole, its points apart from the first pair's by the core's own
/// measure (`align`: within a nanometre the direction is lost).
fn check_align(
    source: kentos_contracts::Vec2,
    target: kentos_contracts::Vec2,
    source2: Option<kentos_contracts::Vec2>,
    target2: Option<kentos_contracts::Vec2>,
) -> Result<(), Stop> {
    let refuse = |message: &str, path: &str| {
        Err(Stop::Failed(checks::error(
            codes::INVALID_ALIGN,
            message.into(),
            Some(path.into()),
        )))
    };
    let apart = |a: kentos_contracts::Vec2, b: kentos_contracts::Vec2| {
        dist(Vec2::new(a.x, a.y), Vec2::new(b.x, b.y)) >= 1e-9
    };
    match (source2, target2) {
        (None, None) => Ok(()),
        (Some(_), None) => refuse(
            "Hizalamanın ikinci hedef noktası verilmedi; ikinci çift iki noktayla verilir. İkinci hedef noktasını verin ya da ikinci kaynak noktasını çıkarın.",
            "transform.target2",
        ),
        (None, Some(_)) => refuse(
            "Hizalamanın ikinci kaynak noktası verilmedi; ikinci çift iki noktayla verilir. İkinci kaynak noktasını verin ya da ikinci hedef noktasını çıkarın.",
            "transform.source2",
        ),
        (Some(s2), Some(t2)) => {
            if !apart(source, s2) {
                return refuse(
                    "Kaynak noktaları çakışıyor; kaynak doğrultusunun yönü yok. Birbirinden ayrı iki kaynak noktası verin.",
                    "transform.source2",
                );
            }
            if !apart(target, t2) {
                return refuse(
                    "Hedef noktaları çakışıyor; hedef doğrultusunun yönü yok. Birbirinden ayrı iki hedef noktası verin.",
                    "transform.target2",
                );
            }
            Ok(())
        }
    }
}

/// What may be written: the objects to transform (slot and id) and each as
/// it would be, those that stay on locked layers, and the warning about them.
struct Checked {
    sources: Vec<(Slot, String)>,
    moved: Vec<Entity>,
    locked: Vec<String>,
    warnings: Vec<CommandWarning>,
}

fn locked_message(n: usize) -> String {
    format!(
        "{n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."
    )
}

/// What a transform does once checked: the modify tools' matrix, Oturt's
/// warp (docs/adr/0156) or Kauçuk levha's sheet (docs/adr/0158).
enum How {
    Matrix(Affine),
    Warp(Warp),
    Sheet(Box<Sheet>),
}

/// Why a rubber sheet's links give no sheet, in the user's words.
fn links_message(e: RubberError) -> String {
    match e {
        RubberError::TooFew => "Kauçuk levha için en az 3 bağ gerekir. Bağ ekleyin.",
        RubberError::Duplicate => {
            "İki bağın kaynağı aynı nokta. Birini çıkarın ya da Kullan'dan bırakın."
        }
        RubberError::Collinear => {
            "Bağların kaynakları bir doğru üstünde; levha kurulamaz. Doğrunun dışında bir bağ ekleyin."
        }
        RubberError::Singular => {
            "Bağların denklem takımının tek çözümü yok. Birbirine çok yakın kaynakları birleştirin."
        }
        RubberError::TooMany => "En çok 1000 bağ alınır. Bağları azaltın.",
    }
    .into()
}

/// Kauçuk levha's links: finite (named by their place), then one sheet.
fn check_sheet(links: &[kentos_contracts::RubberLink]) -> Result<Sheet, Stop> {
    for (i, l) in links.iter().enumerate() {
        let n = i + 1;
        checks::point(
            l.from,
            &format!("{n}. bağın kaynağının"),
            &format!("transform.links[{i}].from"),
        )?;
        checks::point(
            l.to,
            &format!("{n}. bağın hedefinin"),
            &format!("transform.links[{i}].to"),
        )?;
    }
    let core: Vec<Link> = links
        .iter()
        .map(|l| Link {
            from: Vec2::new(l.from.x, l.from.y),
            to: Vec2::new(l.to.x, l.to.y),
        })
        .collect();
    Sheet::solve(&core).map_err(|e| {
        Stop::Failed(checks::error(
            codes::INVALID_LINKS,
            links_message(e),
            Some("transform.links".into()),
        ))
    })
}

/// The rubber sheet's warning: how many shapes bend over [`CHORD`] and the
/// largest bend, millimetres with one decimal by the display rule
/// (docs/adr/0149), a decimal comma.
pub fn bends_message(bent: usize, bend: f64) -> String {
    let mm = fixed(bend * 1000.0, 1).replace('.', ",");
    format!(
        "{bent} nesne gerçek görüntüsünden 0,1 mm'den çok sapıyor (en çok {mm} mm): kauçuk levha yalnız köşeleri taşır, kenarlar doğru, yaylar şişkinliğiyle kalır."
    )
}

const NUMBER_FIX: &str = "Dönüşümün sayılarını sonlu verin.";

/// Whether a linear part [a, b, c, d] (columns (a, b) and (c, d)) squashes
/// the plane: its determinant within 1e-12 of its columns' lengths' product.
fn singular([a, b, c, d]: [f64; 4]) -> bool {
    (a * d - b * c).abs() <= 1e-12 * js_hypot(a, b) * js_hypot(c, d)
}

fn refuse_singular() -> Stop {
    Stop::Failed(checks::error(
        codes::INVALID_TRANSFORM,
        "Dönüşüm tekil: doğrusal kısmı nesneleri bir doğruya ya da noktaya ezer. Dönüşümün sayılarını denetleyin ya da başka bir dönüşüm türü seçin.".into(),
        Some("transform".into()),
    ))
}

fn core(p: kentos_contracts::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// Oturt's centres and numbers, in their order: finite (named by their
/// place), then not singular.
fn check_warp(transform: &Transform) -> Result<Option<Warp>, Stop> {
    let centres = |from, to| -> Result<(), Stop> {
        checks::point(from, "Kaynak merkezinin", "transform.from")?;
        checks::point(to, "Hedef merkezinin", "transform.to")
    };
    let numbers = |values: &[f64], field: &str| -> Result<(), Stop> {
        for (i, &v) in values.iter().enumerate() {
            checks::finite(
                v,
                &format!("Dönüşümün {}. sayısı", i + 1),
                NUMBER_FIX,
                &format!("transform.{field}[{i}]"),
            )?;
        }
        Ok(())
    };
    Ok(Some(match *transform {
        Transform::Similarity { from, to, a, b } => {
            centres(from, to)?;
            checks::finite(a, "Dönüşümün a sayısı", NUMBER_FIX, "transform.a")?;
            checks::finite(b, "Dönüşümün b sayısı", NUMBER_FIX, "transform.b")?;
            if singular([a, b, -b, a]) {
                return Err(refuse_singular());
            }
            Warp::Similarity {
                from: core(from),
                to: core(to),
                a,
                b,
            }
        }
        Transform::Affine { from, to, m } => {
            centres(from, to)?;
            numbers(&m, "m")?;
            if singular(m) {
                return Err(refuse_singular());
            }
            Warp::Affine {
                from: core(from),
                to: core(to),
                m,
            }
        }
        Transform::Projective { from, to, h } => {
            centres(from, to)?;
            numbers(&h, "h")?;
            // The derivative at the source centre (w = 1 there).
            let [a1, a2, a3, b1, b2, b3, c1, c2] = h;
            if singular([a1 - a3 * c1, b1 - b3 * c1, a2 - a3 * c2, b2 - b3 * c2]) {
                return Err(refuse_singular());
            }
            Warp::Projective {
                from: core(from),
                to: core(to),
                h,
            }
        }
        _ => return Ok(None),
    }))
}

/// The transform's own checks, in its fields' order: finite numbers, then
/// what they mean. Its matrix or warp when they pass.
fn check_transform(transform: &Transform) -> Result<How, Stop> {
    if let Transform::Rubbersheet { links } = transform {
        return check_sheet(links).map(|s| How::Sheet(Box::new(s)));
    }
    if let Some(warp) = check_warp(transform)? {
        return Ok(How::Warp(warp));
    }
    let finite =
        |value: f64, what: &str, fix: &str, path: &str| checks::finite(value, what, fix, path);
    match *transform {
        Transform::Move { dx, dy } => {
            let fix = "Kaydırmayı sonlu bir sayıyla verin.";
            finite(dx, "Doğu (Y) yönündeki kaydırma", fix, "transform.dx")?;
            finite(dy, "Kuzey (X) yönündeki kaydırma", fix, "transform.dy")?;
        }
        Transform::Rotate { center, angle } => {
            checks::point(center, "Merkezin", "transform.center")?;
            finite(
                angle,
                "Dönme açısı",
                "Açıyı sonlu bir sayıyla verin.",
                "transform.angle",
            )?;
        }
        Transform::Scale { center, factor } => {
            checks::point(center, "Merkezin", "transform.center")?;
            finite(
                factor,
                "Ölçek faktörü",
                "Faktörü sonlu bir sayıyla verin.",
                "transform.factor",
            )?;
            if factor <= 0.0 {
                return Err(Stop::Failed(checks::error(
                    codes::INVALID_FACTOR,
                    "Ölçek faktörü sıfırdan büyük olmalı. Pozitif bir faktör verin.".into(),
                    Some("transform.factor".into()),
                )));
            }
        }
        Transform::Mirror { a, b } => {
            checks::point(a, "Eksenin ilk noktasının", "transform.a")?;
            checks::point(b, "Eksenin ikinci noktasının", "transform.b")?;
            // The core's own measure of the axis (`mirror`): zero leaves it no direction.
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            if dx * dx + dy * dy == 0.0 {
                return Err(Stop::Failed(checks::error(
                    codes::INVALID_AXIS,
                    "Simetri ekseninin iki noktası aynı; eksenin yönü yok. Birbirinden ayrı iki nokta verin.".into(),
                    Some("transform.b".into()),
                )));
            }
        }
        Transform::Align {
            source,
            target,
            source2,
            target2,
            ..
        } => {
            checks::point(source, "Birinci kaynak noktasının", "transform.source")?;
            checks::point(target, "Birinci hedef noktasının", "transform.target")?;
            if let Some(p) = source2 {
                checks::point(p, "İkinci kaynak noktasının", "transform.source2")?;
            }
            if let Some(p) = target2 {
                checks::point(p, "İkinci hedef noktasının", "transform.target2")?;
            }
            check_align(source, target, source2, target2)?;
        }
        Transform::Similarity { .. }
        | Transform::Affine { .. }
        | Transform::Projective { .. }
        | Transform::Rubbersheet { .. } => {}
    }
    affine(transform).map(How::Matrix).ok_or_else(|| {
        Stop::Failed(checks::error(
            codes::NOT_FINITE,
            "Dönüşüm kurulamadı.".into(),
            Some("transform".into()),
        ))
    })
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &EntitiesTransform) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Dönüştürülecek nesne verilmedi.")?;
    let how = check_transform(&input.transform)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut sources = Vec::new();
    let mut moved = Vec::new();
    let mut locked = Vec::new();
    let (mut overflow, mut beyond) = (false, false);
    let (mut curves, mut kept) = (0, 0);
    let (mut bent, mut bend) = (0, 0.0_f64);
    for (slot, entity, uid) in checks::objects(doc, &input.uids)? {
        if doc.layers().is_locked(&entity.base().layer_id) {
            locked.push(uid.clone());
            continue;
        }
        let before = shape(entity);
        let e = match &how {
            How::Matrix(m) => {
                let after = transform_shape(&before, m);
                overflow |= finite_shape(&before) && !finite_shape(&after);
                with_shape(entity, after)
            }
            How::Sheet(sheet) => {
                let zs: Vec<Vec<Option<f64>>> =
                    elevation::paths(entity).into_iter().map(|p| p.zs).collect();
                let (w, b) = sheet_shape(&before, &zs, sheet);
                overflow |= finite_shape(&before) && !finite_shape(&w.shape);
                kept += usize::from(w.kept);
                bent += usize::from(b > CHORD);
                bend = js_max(bend, b);
                // Kinds stay on a sheet; the elevations stay with their vertices.
                let mut e = with_shape(entity, w.shape);
                if let Some(e) = e.as_mut() {
                    elevation::assign(e, &w.zs);
                }
                e
            }
            How::Warp(warp) => {
                let zs: Vec<Vec<Option<f64>>> =
                    elevation::paths(entity).into_iter().map(|p| p.zs).collect();
                match warp_shape(&before, &zs, warp) {
                    Err(_) => {
                        beyond = true;
                        sources.push((slot, uid.clone()));
                        continue;
                    }
                    Ok(w) => {
                        overflow |= finite_shape(&before) && !finite_shape(&w.shape);
                        curves += usize::from(w.curves);
                        kept += usize::from(w.kept);
                        // A curve may come back of another kind: the object takes it, its own fields kept.
                        let mut e = with_shape(entity, w.shape.clone()).or_else(|| {
                            edit_geometry(w.shape).map(|g| entity_of(&g, entity.base().clone()))
                        });
                        if let Some(e) = e.as_mut() {
                            elevation::assign(e, &w.zs);
                        }
                        e
                    }
                }
            }
        };
        // The core keeps an object's kind for the modify tools; a shape it cannot write would be its fault.
        let Some(e) = e else {
            return Err(Stop::Failed(checks::error(
                codes::NOT_FINITE,
                "Nesne dönüştürülemedi.".into(),
                Some("transform".into()),
            )));
        };
        sources.push((slot, uid.clone()));
        moved.push(e);
    }
    if sources.is_empty() {
        return Err(Stop::Failed(checks::error(
            codes::LAYER_LOCKED,
            locked_message(locked.len()),
            Some("uids".into()),
        )));
    }
    if beyond {
        return Err(Stop::Failed(checks::error(
            codes::BEYOND_HORIZON,
            "Projektif dönüşümün ufku nesnelerin arasından geçiyor: bir nesnenin noktası ufkun ötesinde kalıyor, dönüştürülemez. O nesneleri dışarıda bırakın ya da kontrol noktalarını denetleyin.".into(),
            Some("transform".into()),
        )));
    }
    if overflow {
        return Err(Stop::Failed(checks::error(
            codes::NOT_FINITE,
            "Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin.".into(),
            Some("transform".into()),
        )));
    }
    let mut warnings = Vec::new();
    if !locked.is_empty() {
        warnings.push(CommandWarning {
            code: codes::LAYER_LOCKED.into(),
            message: locked_message(locked.len()),
            path: Some("uids".into()),
        });
    }
    if curves > 0 {
        warnings.push(CommandWarning {
            code: codes::WARP_CURVES.into(),
            message: format!("{curves} nesnenin eğrileri 0,1 mm'lik köşelere açıldı; dönüşüm benzerlik değil, eğri olarak kalamazlar."),
            path: Some("transform".into()),
        });
    }
    if kept > 0 {
        warnings.push(CommandWarning {
            code: codes::WARP_SHAPES.into(),
            message: format!("{kept} yazı, not, blok, ölçü ya da tarama deseni yerinde döndürülüp ölçeklendi; dönüşüm benzerlik değil, biçimleri eğilmez."),
            path: Some("transform".into()),
        });
    }
    if bent > 0 {
        warnings.push(CommandWarning {
            code: codes::RUBBER_BENDS.into(),
            message: bends_message(bent, bend),
            path: Some("transform".into()),
        });
    }
    Ok(Checked {
        sources,
        moved,
        locked,
        warnings,
    })
}

/// Every number of a shape's geometry is finite (the fields the web's
/// handler walks: points, radii, angles, bulges, heights, offsets, the
/// hatch pattern's angle and spacing).
pub(crate) fn finite_shape(s: &Shape) -> bool {
    fn pt(p: &Vec2) -> bool {
        p.x.is_finite() && p.y.is_finite()
    }
    fn pts(ps: &[Vec2]) -> bool {
        ps.iter().all(pt)
    }
    fn values(vs: &Option<Vec<f64>>) -> bool {
        vs.iter().flatten().all(|v| v.is_finite())
    }
    match s {
        Shape::Point { p, z, parts } => {
            pt(p)
                && z.is_none_or(f64::is_finite)
                && parts
                    .iter()
                    .flatten()
                    .all(|q| pt(&q.p) && q.z.is_none_or(f64::is_finite))
        }
        Shape::Insert {
            p, scale, rotation, ..
        } => pt(p) && scale.is_finite() && rotation.is_finite(),
        Shape::Line { a, b } => pt(a) && pt(b),
        Shape::Polyline {
            pts: p,
            bulges,
            holes,
            parts,
        } => {
            pts(p)
                && values(bulges)
                && holes
                    .iter()
                    .flatten()
                    .all(|h| pts(&h.pts) && values(&h.bulges))
                && parts
                    .iter()
                    .flatten()
                    .all(|q| pts(&q.pts) && values(&q.bulges))
        }
        Shape::Polygon {
            pts: p,
            bulges,
            holes,
            parts,
        } => {
            let ring_ok = |p: &[Vec2], bulges: &Option<Vec<f64>>, holes: &Option<Vec<Ring>>| {
                pts(p)
                    && values(bulges)
                    && holes
                        .iter()
                        .flatten()
                        .all(|h| pts(&h.pts) && values(&h.bulges))
            };
            // Every part of a multi-part area too (docs/adr/0143).
            ring_ok(p, bulges, holes)
                && parts
                    .iter()
                    .flatten()
                    .all(|q| ring_ok(&q.pts, &q.bulges, &q.holes))
        }
        Shape::Circle { c, r } => pt(c) && r.is_finite(),
        Shape::Arc { c, r, a0, a1 } => pt(c) && r.is_finite() && a0.is_finite() && a1.is_finite(),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => pt(c) && pt(major) && ratio.is_finite() && t0.is_finite() && t1.is_finite(),
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => pt(p) && pt(dir),
        Shape::Spline { pts: p, .. } => pts(p),
        Shape::Table {
            p,
            rotation,
            height,
            rows,
            columns,
            ..
        } => {
            pt(p)
                && rotation.is_finite()
                && height.is_finite()
                && rows.iter().chain(columns).all(|x| x.is_finite())
        }
        Shape::Text {
            p,
            height,
            rotation,
            ..
        } => pt(p) && height.is_finite() && rotation.is_finite(),
        Shape::Leader {
            pts: p,
            height,
            rotation,
            ..
        } => pts(p) && height.is_finite() && rotation.is_finite(),
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            angle,
            c,
            ..
        } => {
            pt(a)
                && pt(b)
                && offset.is_finite()
                && height.is_finite()
                && angle.is_none_or(f64::is_finite)
                && c.as_ref().is_none_or(pt)
        }
        Shape::Hatch {
            ring,
            holes,
            pattern,
        } => {
            pts(ring)
                && holes.iter().flatten().all(|h| pts(h))
                && pattern.angle.is_finite()
                && pattern.spacing.is_finite()
        }
    }
}
