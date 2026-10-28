//! Live wall dimension recipes (ADR0695). GUI, Python and MCP share these handlers.
use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    CommandResult, CommandWarning, DimensionSet, DimensionSetBreak, DimensionSetEdit,
    DimensionSetFace, DimensionSetId, DimensionSetRounding, DimensionSetRow, DimensionSetRowNote,
    DimensionSetRowPlace, DimensionSetScaleNote, DimensionSetSpan, DimensionSetsChanged,
    DimensionSetsCreateInput, DimensionSetsEditInput, DimensionSetsPlan, DimensionSetsQueried,
    DimensionSetsQueryInput, Entity, EntityId, Vec2, dimension_sets_problem, scales_problem,
};
use kentos_domain::dimension_sets::PlannedDimensionSetMember;
use kentos_domain::dimension_spans::SpanRefusal;
use kentos_domain::{Document, Uuid};
use kentos_domain::dimension_layout::member_value;
use kentos_geometry_core::ops::dimension_layout::rounding_gap;
use kentos_geometry_core::ops::dimension_span::Why;
use kentos_project::format::Format;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};

fn fail(message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(
        "invalid_dimension_set",
        message.into(),
        Some(path.into()),
    ))
}
fn source_error(why: DimensionSetBreak) -> Stop {
    let message = match why {
        DimensionSetBreak::HostGone => "Kaynak duvar bulunamadı. Başka bir duvar seçin.",
        DimensionSetBreak::UnsupportedWall => {
            "Duvarın bu yüzü köşesinde pahlandığı ya da kenarı kalmadığı için ölçülemiyor. Başka bir kenar ya da yüz seçin."
        }
        DimensionSetBreak::OpeningUnplaced => {
            "Duvarın bir açıklığı yerleştirilemedi. Önce açıklığın kaynak bağını düzeltin."
        }
        DimensionSetBreak::MixedSources => {
            "Duvarlar farklı katlarda; cephe ölçüsünün duvarları tek bir katta olmalı."
        }
    };
    fail(message, "wall")
}
fn recipe_ok(set: &DimensionSet) -> Result<(), Stop> {
    let mut recipe = set.clone();
    recipe.members.clear(); // membership is derived, never caller input
    if let Some((_, why)) = dimension_sets_problem(&[recipe]) {
        return Err(fail(why, "operation"));
    }
    Ok(())
}
/// The edges a create measures, each with its face and its first row's place (docs/adr/0700): the given edge, or
/// with `around` every straight edge, a closed wall's outer face, each row as far out as `at` is from its edge.
fn measured_edges(
    doc: &Document,
    wall: Uuid,
    input: &DimensionSetsCreateInput,
) -> Result<Vec<(Option<u32>, DimensionSetFace, Vec2)>, Stop> {
    let Some(entity) = doc.slot_of(wall).and_then(|slot| doc.get(slot)) else {
        return Err(source_error(DimensionSetBreak::HostGone));
    };
    let Entity::Wall(w) = entity else {
        return Err(fail("Kaynak bir duvar değil. Ölçülecek duvarı seçin.", "wall"));
    };
    let edges = doc
        .dimension_set_wall_edges(wall)
        .ok_or_else(|| source_error(DimensionSetBreak::UnsupportedWall))?;
    // An open two-point wall's only edge keeps no number, as recipes before edges have it.
    let stored = |k: u32| (w.wall.closed || w.wall.pts.len() > 2).then_some(k);
    if let Some(k) = input.edge {
        if k as usize >= edges.count {
            return Err(fail(
                format!(
                    "Duvarda {k} numaralı kenar yok; kenarlar 0 ile {} arasında.",
                    edges.count.saturating_sub(1)
                ),
                "edge",
            ));
        }
        if !edges.straight.contains(&k) {
            return Err(fail(
                format!("{k} numaralı kenar yay; ölçü dizisi yalnız düz kenarı ölçer."),
                "edge",
            ));
        }
    }
    if !input.around {
        let edge = match input.edge {
            Some(k) => stored(k),
            None if edges.count > 1 || w.wall.closed => {
                return Err(fail(
                    format!(
                        "Duvarın {} kenarı var; ölçülecek kenarı (edge) verin ya da bütün kenarları (around) seçin.",
                        edges.count
                    ),
                    "edge",
                ));
            }
            None => None,
        };
        return Ok(vec![(edge, input.face, input.at)]);
    }
    let Some(&first) = edges.straight.first() else {
        return Err(fail(
            "Duvarın düz kenarı yok; ölçü dizisi yalnız düz kenarı ölçer.",
            "around",
        ));
    };
    // A closed wall's outer face is on the right of a counter-clockwise path.
    let face = match (w.wall.closed, edges.counter_clockwise) {
        (true, true) => DimensionSetFace::Right,
        (true, false) => DimensionSetFace::Left,
        (false, _) => input.face,
    };
    let line = |k: u32| {
        doc.dimension_set_wall_face(wall, face, stored(k))
            .map_err(source_error)
    };
    // Outward from the face: its left normal on the left face, the right one on the right.
    let outward = |[a, b]: [Vec2; 2]| {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l = dx.hypot(dy);
        let n = (-dy / l, dx / l);
        match face {
            DimensionSetFace::Left => n,
            DimensionSetFace::Right => (-n.0, -n.1),
        }
    };
    let base = line(input.edge.unwrap_or(first))?;
    let n = outward(base);
    let distance = ((input.at.x - base[0].x) * n.0 + (input.at.y - base[0].y) * n.1).abs();
    edges
        .straight
        .iter()
        .map(|&k| {
            let [a, b] = line(k)?;
            let n = outward([a, b]);
            let at = Vec2 {
                x: (a.x + b.x) / 2.0 + n.0 * distance,
                y: (a.y + b.y) / 2.0 + n.1 * distance,
            };
            Ok((stored(k), face, at))
        })
        .collect()
}
fn create_recipes(
    doc: &Document,
    input: &DimensionSetsCreateInput,
) -> Result<Vec<DimensionSet>, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::point(input.at, "Ölçü sırasının yeri", "at")?;
    if let Some(why) = scales_problem(&input.scales) {
        return Err(fail(why, "scales"));
    }
    checks::layer(doc, &input.layer_id)?;
    let (look, height) = crate::dimension_create::style_of(doc, input.dimension_style.as_deref())?;
    if input.space.is_some() || !input.walls.is_empty() {
        return span_recipes(doc, input, look, height);
    }
    let Some(wall) = &input.wall else {
        return Err(fail(
            "Ölçülecek duvarı (wall), cephenin duvarlarını (walls) ya da mahali (space) verin.",
            "wall",
        ));
    };
    if !checks::is_uid_text(wall) {
        return Err(fail("Geçerli bir duvar kimliği verin.", "wall"));
    }
    if input.direction.is_some() || !input.reference.is_faces() {
        return Err(fail(
            "Doğrultu (direction) ve ölçülen çizgi (reference) yalnız cephe (walls) ve mahal (space) dizisinde verilir.",
            "direction",
        ));
    }
    let wall = Uuid::parse_str(wall).map_err(|_| fail("Duvar kimliği geçersiz.", "wall"))?;
    measured_edges(doc, wall, input)?
        .into_iter()
        .map(|(edge, face, at)| {
            let set = DimensionSet {
                id: DimensionSetId([1; 16]), // internal proposal only; a plan allocates no identity
                wall: EntityId(wall.into_bytes()),
                edge,
                face,
                rows: input.rows.clone(),
                openings: input.openings,
                layer_id: input.layer_id.clone(),
                at,
                gap_mm: input.gap_mm,
                height,
                look: look.clone(),
                row_places: Vec::new(),
                members: Vec::new(),
                suppressed: Vec::new(),
                broken: None,
                span: None,
                scales: input.scales.clone(),
                tight: input.tight,
            };
            recipe_ok(&set)?;
            doc.dimension_set_plan(&set).map_err(source_error)?;
            Ok(set)
        })
        .collect()
}

fn uid_at(text: &str, path: &str) -> Result<EntityId, Stop> {
    if !checks::is_uid_text(text) {
        return Err(fail("Geçerli bir kimlik verin.", path));
    }
    Uuid::parse_str(text)
        .map(|u| EntityId(u.into_bytes()))
        .map_err(|_| fail("Kimlik geçersiz.", path))
}

/// Why a façade or space recipe cannot be made (docs/adr/0729 §3), its source named.
fn span_error(r: SpanRefusal, path: &str) -> Stop {
    let named = r.source.map(|id| format!(" ({id})")).unwrap_or_default();
    let message = match (r.broken, r.why) {
        (DimensionSetBreak::UnsupportedWall, Some(Why::Oblique)) => format!(
            "Kaynağın{named} bir kenarı ölçü doğrultusuna ne paralel ne dik. Cephe ve mahal ölçüsü yalnız doğrultuya paralel ve dik kenarları ölçer: doğrultuyu duvarlara göre verin ya da eğik duvarı ayrı ölçün."
        ),
        (DimensionSetBreak::UnsupportedWall, Some(Why::Arc)) => format!(
            "Kaynağın{named} yaylı bir kenarı var; cephe ve mahal ölçüsü yaylı kenarı ölçmez. Yaylı duvarı tek duvar ölçüsüyle ya da ayrı ölçün."
        ),
        (DimensionSetBreak::UnsupportedWall, Some(Why::Drawn)) => format!(
            "Kaynak duvarın{named} bir yüzü köşesinde pahlanıyor ya da duvar çizilemiyor; ölçülemez."
        ),
        (DimensionSetBreak::UnsupportedWall, None) if r.source.is_some() => format!(
            "Kaynak{named} bir duvar değil (mahal dizisinde bir mahal değil). Doğru nesneyi seçin."
        ),
        (DimensionSetBreak::UnsupportedWall, None) => {
            "Ölçü doğrultusu, yeri ya da sıra aralığı geçersiz.".to_owned()
        }
        (DimensionSetBreak::MixedSources, _) => format!(
            "Duvarlar{named} farklı katlarda; cephe ölçüsünün duvarları tek bir katta (ya da hepsi katsız) olmalı. Duvarlara aynı katı verin ya da ayrı ölçün."
        ),
        (DimensionSetBreak::HostGone, _) => {
            "Kaynak bulunamadı. Duvarları ya da mahali yeniden seçin.".to_owned()
        }
        (DimensionSetBreak::OpeningUnplaced, _) => {
            "Duvarın bir açıklığı yerleştirilemedi. Önce açıklığın kaynak bağını düzeltin.".to_owned()
        }
    };
    let mut e = error("invalid_dimension_set", message, Some(path.into()));
    e.uids = r.source.map(|id| vec![id.to_string()]).unwrap_or_default();
    Stop::failed(e)
}

/// A path's longest straight edge's direction; none when it has none.
fn longest_edge(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Option<Vec2> {
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    let mut best: Option<(f64, Vec2)> = None;
    for k in 0..edges {
        if bulges.and_then(|b| b.get(k)).is_some_and(|b| *b != 0.0) {
            continue;
        }
        let (a, b) = (pts[k], pts[(k + 1) % n]);
        let l = (b.x - a.x).hypot(b.y - a.y);
        if l > 0.0 && best.is_none_or(|(m, _)| l > m) {
            best = Some((
                l,
                Vec2 {
                    x: (b.x - a.x) / l,
                    y: (b.y - a.y) / l,
                },
            ));
        }
    }
    best.map(|(_, d)| d)
}

/// A direction turned to read left to right, or upward (docs/adr/0729 §5): rows are written along it from its low end.
pub(crate) fn readable(d: Vec2) -> Vec2 {
    if d.x < -1e-9 || (d.x.abs() <= 1e-9 && d.y < 0.0) {
        Vec2 {
            x: -d.x + 0.0,
            y: -d.y + 0.0,
        }
    } else {
        d
    }
}

/// The points of a façade's walls' faces or a space's outer rings: what its four sides go round.
fn body_points(doc: &Document, set: &DimensionSet) -> Vec<Vec2> {
    let mut out = Vec::new();
    for id in set.sources() {
        let Some(e) = doc
            .slot_of(Uuid::from_bytes(id.0))
            .and_then(|slot| doc.get(slot))
        else {
            continue;
        };
        match e {
            Entity::Wall(w) => {
                let pts: Vec<_> = w
                    .wall
                    .pts
                    .iter()
                    .map(|p| kentos_geometry_core::vec2::Vec2::new(p.x, p.y))
                    .collect();
                let spec = crate::walls::core_spec(&w.wall);
                if let Ok(lines) =
                    kentos_geometry_core::geom::wall::lines(&pts, w.wall.bulges.as_deref(), &spec)
                {
                    for l in [&lines[0], &lines[lines.len() - 1]] {
                        out.extend(l.pts.iter().map(|p| Vec2 { x: p.x, y: p.y }));
                    }
                }
            }
            Entity::Space(s) => {
                for part in &s.space.parts {
                    out.extend(part.outer.pts.iter().copied());
                }
            }
            _ => {}
        }
    }
    out
}

/// A façade's or a space's four sides (docs/adr/0729 §5): along the direction south and north of the bodies, square to
/// it east and west, each first row as far out from the outermost body as `at`.
fn four_sides(doc: &Document, base: &DimensionSet) -> Vec<DimensionSet> {
    let Some(span) = &base.span else {
        return vec![base.clone()];
    };
    let pts = body_points(doc, base);
    let Some(origin) = pts.first().copied() else {
        return vec![base.clone()];
    };
    let d = span.direction;
    let n = Vec2 { x: -d.y, y: d.x };
    let t = |p: Vec2| (p.x - origin.x) * d.x + (p.y - origin.y) * d.y;
    let o = |p: Vec2| (p.x - origin.x) * n.x + (p.y - origin.y) * n.y;
    let (mut t0, mut t1, mut o0, mut o1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for p in &pts {
        (t0, t1) = (t0.min(t(*p)), t1.max(t(*p)));
        (o0, o1) = (o0.min(o(*p)), o1.max(o(*p)));
    }
    let (ta, oa) = (t(base.at), o(base.at));
    let reach = if oa < o0 {
        o0 - oa
    } else if oa > o1 {
        oa - o1
    } else if ta < t0 {
        t0 - ta
    } else if ta > t1 {
        ta - t1
    } else {
        (oa - o0).min(o1 - oa).min(ta - t0).min(t1 - ta)
    };
    let at = |tt: f64, oo: f64| Vec2 {
        x: origin.x + d.x * tt + n.x * oo,
        y: origin.y + d.y * tt + n.y * oo,
    };
    let (tm, om) = ((t0 + t1) * 0.5, (o0 + o1) * 0.5);
    [
        (d, at(tm, o0 - reach)),
        (n, at(t1 + reach, om)),
        (d, at(tm, o1 + reach)),
        (n, at(t0 - reach, om)),
    ]
    .into_iter()
    .map(|(direction, place)| {
        let mut set = base.clone();
        set.at = place;
        if let Some(s) = &mut set.span {
            s.direction = readable(direction);
        }
        set
    })
    .collect()
}

/// A façade's or a space's recipes (docs/adr/0729 §1, §5): its sources checked, its direction, one recipe or four.
fn span_recipes(
    doc: &Document,
    input: &DimensionSetsCreateInput,
    look: kentos_commands::DimensionLook,
    height: f64,
) -> Result<Vec<DimensionSet>, Stop> {
    if input.edge.is_some() {
        return Err(fail(
            "Kenar (edge) yalnız tek duvar ölçüsünde verilir; cephe ve mahal doğrultusuyla ölçülür.",
            "edge",
        ));
    }
    let (wall, walls, space, path) = if let Some(space) = &input.space {
        if input.wall.is_some() || !input.walls.is_empty() {
            return Err(fail(
                "Mahal ölçüsüne duvar verilmez: ya mahali (space) ya duvarları (walls) verin.",
                "space",
            ));
        }
        (uid_at(space, "space")?, Vec::new(), true, "space")
    } else {
        let mut all = Vec::new();
        if let Some(w) = &input.wall {
            all.push(uid_at(w, "wall")?);
        }
        for (i, w) in input.walls.iter().enumerate() {
            let id = uid_at(w, &format!("walls[{i}]"))?;
            if all.contains(&id) {
                return Err(fail("Aynı duvar iki kez verildi.", &format!("walls[{i}]")));
            }
            all.push(id);
        }
        (all[0], all[1..].to_vec(), false, "walls")
    };
    for id in std::iter::once(&wall).chain(walls.iter()) {
        let e = doc
            .slot_of(Uuid::from_bytes(id.0))
            .and_then(|slot| doc.get(slot));
        let fits = match e {
            None => {
                return Err(span_error(
                    SpanRefusal {
                        broken: DimensionSetBreak::HostGone,
                        source: Some(*id),
                        why: None,
                    },
                    path,
                ));
            }
            Some(Entity::Wall(_)) => !space,
            Some(Entity::Space(_)) => space,
            Some(_) => false,
        };
        if !fits {
            return Err(span_error(
                SpanRefusal {
                    broken: DimensionSetBreak::UnsupportedWall,
                    source: Some(*id),
                    why: None,
                },
                path,
            ));
        }
    }
    let first = doc
        .slot_of(Uuid::from_bytes(wall.0))
        .and_then(|slot| doc.get(slot));
    let level = match first {
        Some(Entity::Wall(w)) => w.wall.arch.level.clone(),
        Some(Entity::Space(s)) => s.space.arch.level.clone(),
        _ => None,
    };
    let direction = match input.direction {
        Some(d) => {
            checks::point(d, "Ölçü doğrultusu", "direction")?;
            let l = d.x.hypot(d.y);
            if l.is_nan() || l <= 1e-12 {
                return Err(fail("Ölçü doğrultusu sıfır olamaz.", "direction"));
            }
            Vec2 {
                x: d.x / l,
                y: d.y / l,
            }
        }
        None => match first {
            Some(Entity::Wall(w)) => longest_edge(&w.wall.pts, w.wall.bulges.as_deref(), w.wall.closed),
            Some(Entity::Space(s)) => s.space.parts.first().and_then(|part| {
                longest_edge(&part.outer.pts, part.outer.bulges.as_deref(), true)
            }),
            _ => None,
        }
        .ok_or_else(|| fail("Kaynağın düz kenarı yok; ölçü doğrultusunu (direction) verin.", "direction"))?,
    };
    let direction = readable(direction);
    let base = DimensionSet {
        id: DimensionSetId([1; 16]),
        wall,
        edge: None,
        face: DimensionSetFace::Left,
        rows: input.rows.clone(),
        openings: input.openings,
        layer_id: input.layer_id.clone(),
        at: input.at,
        gap_mm: input.gap_mm,
        height,
        look,
        row_places: Vec::new(),
        members: Vec::new(),
        suppressed: Vec::new(),
        broken: None,
        span: Some(DimensionSetSpan {
            walls,
            space,
            direction,
            reference: input.reference,
            level,
        }),
        scales: input.scales.clone(),
        tight: input.tight,
    };
    let sets = if input.around { four_sides(doc, &base) } else { vec![base] };
    for set in &sets {
        recipe_ok(set)?;
        let (members, _) = doc.dimension_span_plan(set).map_err(|r| span_error(r, path))?;
        if members.is_empty() {
            return Err(fail(
                "Bu kaynakta seçilen sıralarda ölçülecek aralık yok: başka sıra seçin ya da kaynakları değiştirin.",
                "rows",
            ));
        }
    }
    Ok(sets)
}

/// The plan's account of its values (docs/adr/0729 §6, docs/adr/0730 §2): those that do not fit, those the recipe moved,
/// and the same at each sheet scale, by their place among the members.
fn value_notes(members: &[PlannedDimensionSetMember]) -> (Vec<u32>, Vec<u32>, Vec<DimensionSetScaleNote>) {
    let pick = |f: &dyn Fn(&PlannedDimensionSetMember) -> bool| -> Vec<u32> {
        members
            .iter()
            .enumerate()
            .filter(|(_, m)| f(m))
            .map(|(i, _)| i as u32)
            .collect()
    };
    let tight = pick(&|m| m.notes.own.tight);
    let placed = pick(&|m| m.notes.own.placed);
    let scales = members
        .first()
        .map(|first| {
            first
                .notes
                .scales
                .iter()
                .enumerate()
                .map(|(k, (scale, _))| DimensionSetScaleNote {
                    scale: *scale,
                    tight: pick(&|m| m.notes.scales.get(k).is_some_and(|n| n.1.tight)),
                    placed: pick(&|m| m.notes.scales.get(k).is_some_and(|n| n.1.placed)),
                })
                .collect()
        })
        .unwrap_or_default();
    (tight, placed, scales)
}

/// The rows whose values, each rounded from its own measurement, add up to other than the overall reads (docs/adr/0730
/// §3): a row running from the overall's first place to its last without a gap, its values and the overall's plain
/// decimal numbers with as many decimals.
fn rounding(doc: &Document, set: &DimensionSet, members: &[PlannedDimensionSetMember]) -> Vec<DimensionSetRounding> {
    let format = Format::of(doc.settings());
    let Some(overall) = members.iter().find(|m| m.segment.row == DimensionSetRow::Overall) else {
        return Vec::new();
    };
    let shown = |m: &PlannedDimensionSetMember| member_value(&format, &m.dimension).1;
    let total = shown(overall);
    let mut out = Vec::new();
    for row in set.rows.iter().copied().filter(|r| *r != DimensionSetRow::Overall) {
        let chain: Vec<&PlannedDimensionSetMember> =
            members.iter().filter(|m| m.segment.row == row).collect();
        let (Some(first), Some(last)) = (chain.first(), chain.last()) else {
            continue;
        };
        if first.segment.a != overall.segment.a
            || last.segment.b != overall.segment.b
            || chain.windows(2).any(|w| w[0].segment.b != w[1].segment.a)
        {
            continue;
        }
        let parts: Vec<String> = chain.iter().map(|m| shown(m)).collect();
        let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        if let Some(sum) = rounding_gap(&refs, &total) {
            out.push(DimensionSetRounding {
                row,
                parts: sum,
                overall: total.clone(),
            });
        }
    }
    out
}

/// A row's name as the user reads it.
fn row_name(row: DimensionSetRow) -> &'static str {
    match row {
        DimensionSetRow::Openings => "Açıklıklar",
        DimensionSetRow::Walls => "Duvarlar",
        DimensionSetRow::Grids => "Akslar",
        DimensionSetRow::Overall => "Toplam",
    }
}

/// `plan` with `more`'s members and accounts after its own, their places counted on.
fn merge(out: &mut DimensionSetsPlan, more: DimensionSetsPlan) {
    let base = out.entities.len() as u32;
    out.tight.extend(more.tight.iter().map(|i| i + base));
    out.placed.extend(more.placed.iter().map(|i| i + base));
    for note in more.scales {
        let k = match out.scales.iter().position(|n| n.scale == note.scale) {
            Some(k) => k,
            None => {
                out.scales.push(DimensionSetScaleNote {
                    scale: note.scale,
                    tight: Vec::new(),
                    placed: Vec::new(),
                });
                out.scales.len() - 1
            }
        };
        let mine = &mut out.scales[k];
        mine.tight.extend(note.tight.iter().map(|i| i + base));
        mine.placed.extend(note.placed.iter().map(|i| i + base));
    }
    out.rounding.extend(more.rounding);
    out.entities.extend(more.entities);
    out.spans.extend(more.spans);
    out.rows.extend(more.rows);
    for wall in more.skipped {
        if !out.skipped.contains(&wall) {
            out.skipped.push(wall);
        }
    }
}

/// A plan's warnings: dimensions that do not fit, walls at a façade left unmeasured.
fn plan_warnings(plan: &DimensionSetsPlan) -> Vec<CommandWarning> {
    let mut out = Vec::new();
    if !plan.tight.is_empty() {
        let moved = plan.placed.len();
        out.push(CommandWarning {
            code: crate::codes::DIMENSION_TEXT_TIGHT.into(),
            message: if moved == 0 {
                format!(
                    "{} ölçünün değeri iki uzatma çizgisinin arasına sığmıyor; gizlenmedi. Sıra aralığını ya da ölçü stilinin yazı yerini değiştirin.",
                    plan.tight.len()
                )
            } else {
                format!(
                    "{} ölçünün değeri iki uzatma çizgisinin arasına sığmıyor: {} tanesi yana, üste ya da alta alındı, {} tanesi yerinde kaldı.",
                    plan.tight.len(),
                    moved,
                    plan.tight.len().saturating_sub(moved)
                )
            },
            path: None,
        });
    }
    for r in &plan.rounding {
        out.push(CommandWarning {
            code: crate::codes::DIMENSION_SET_ROUNDING.into(),
            message: format!(
                "{} sırasının yuvarlanmış değerlerinin toplamı {}; toplam ölçü {} yazıyor. Her değer kendi ölçüsünden yuvarlanır, fark dağıtılmaz.",
                row_name(r.row),
                r.parts,
                r.overall
            ),
            path: None,
        });
    }
    if !plan.skipped.is_empty() {
        out.push(CommandWarning {
            code: crate::codes::DIMENSION_SET_WALL_SKIPPED.into(),
            message: format!(
                "Cepheye değen {} duvar eğik ya da yaylı kenarı yüzünden ölçülmedi.",
                plan.skipped.len()
            ),
            path: None,
        });
    }
    out
}

fn member_entities(doc: &Document, set: &DimensionSet) -> BTreeMap<String, Entity> {
    set.members
        .iter()
        .filter_map(|m| {
            let uid = Uuid::from_bytes(m.uid.0);
            Some((uid.to_string(), doc.get(doc.slot_of(uid)?)?.clone()))
        })
        .collect()
}
fn existing(doc: &Document, id: DimensionSetId) -> Result<&DimensionSet, Stop> {
    doc.dimension_set(id).ok_or_else(|| {
        Stop::failed(error(
            "dimension_set_not_found",
            "Ölçü grubu bulunamadı; yeniden seçin.".into(),
            Some("id".into()),
        ))
    })
}
fn check_edit(doc: &Document, input: &DimensionSetsEditInput) -> Result<DimensionSet, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let before = existing(doc, input.id)?;
    let mut checked = HashSet::new();
    for layer in std::iter::once(&before.layer_id).chain(before.members.iter().filter_map(|m| {
        let uid = Uuid::from_bytes(m.uid.0);
        doc.get(doc.slot_of(uid)?).map(|e| &e.base().layer_id)
    })) {
        if checked.insert(layer) {
            checks::layer(doc, layer)?;
        }
    }
    let mut set = before.clone();
    match &input.operation {
        DimensionSetEdit::Move { at, row } => {
            checks::point(*at, "Ölçü sırasının yeri", "operation.at")?;
            if let Some(row) = row {
                if !set.rows.contains(row) {
                    return Err(fail("Bu sıra grupta yok.", "operation.row"));
                }
                match set.row_places.iter_mut().find(|p| p.row == *row) {
                    Some(place) => place.at = *at,
                    None => set
                        .row_places
                        .push(DimensionSetRowPlace { row: *row, at: *at }),
                }
            } else {
                let delta = (at.x - set.at.x, at.y - set.at.y);
                set.at = *at;
                for place in &mut set.row_places {
                    place.at.x += delta.0;
                    place.at.y += delta.1;
                }
            }
        }
        DimensionSetEdit::RowOffset { row, offset } => {
            if !offset.is_finite() {
                return Err(fail("Sıra uzaklığı sonlu olmalı.", "operation.offset"));
            }
            doc.set_dimension_set_row_offset(&mut set, *row, *offset)
                .map_err(source_error)?;
        }
        DimensionSetEdit::Set {
            face,
            rows,
            openings,
            gap_mm,
            dimension_style,
            layer_id,
            reference,
            scales,
            tight,
        } => {
            if let Some(scales) = scales {
                if let Some(why) = scales_problem(scales) {
                    return Err(fail(why, "operation.scales"));
                }
                set.scales = scales.clone();
            }
            if let Some(tight) = tight {
                set.tight = *tight;
            }
            if let Some(face) = face {
                if set.span.is_some() {
                    return Err(fail(
                        "Cephe ve mahal dizisinde yüz, sıraların yanından seçilir; ölçülen çizgi için reference verin.",
                        "operation.face",
                    ));
                }
                set.face = *face;
            }
            if let Some(reference) = reference {
                let Some(span) = &mut set.span else {
                    return Err(fail(
                        "Ölçülen çizgi (reference) yalnız cephe ve mahal dizisinde verilir.",
                        "operation.reference",
                    ));
                };
                span.reference = *reference;
            }
            if let Some(rows) = rows {
                set.rows = rows.clone();
                set.row_places.retain(|p| rows.contains(&p.row));
                set.members.retain(|m| rows.contains(&m.row));
                set.suppressed.retain(|m| rows.contains(&m.row));
            }
            if let Some(openings) = openings {
                set.openings = *openings;
            }
            if let Some(gap) = gap_mm {
                set.gap_mm = *gap;
            }
            if let Some(layer) = layer_id {
                checks::layer(doc, layer)?;
                set.layer_id = layer.clone();
            }
            if let Some(name) = dimension_style {
                (set.look, set.height) = crate::dimension_create::style_of(doc, Some(name))?;
            }
        }
        DimensionSetEdit::Sources { walls } => {
            let Some(span) = &set.span else {
                return Err(fail(
                    "Kaynak duvarlar yalnız cephe dizisinde değiştirilir.",
                    "operation.walls",
                ));
            };
            if span.space {
                return Err(fail(
                    "Mahal dizisinin kaynağı mahaldir; duvar verilmez.",
                    "operation.walls",
                ));
            }
            let mut ids: Vec<EntityId> = Vec::new();
            for (i, w) in walls.iter().enumerate() {
                let path = format!("operation.walls[{i}]");
                let id = uid_at(w, &path)?;
                if ids.contains(&id) {
                    return Err(fail("Aynı duvar iki kez verildi.", &path));
                }
                match doc.slot_of(Uuid::from_bytes(id.0)).and_then(|slot| doc.get(slot)) {
                    Some(Entity::Wall(_)) => ids.push(id),
                    found => {
                        return Err(span_error(
                            SpanRefusal {
                                broken: if found.is_none() {
                                    DimensionSetBreak::HostGone
                                } else {
                                    DimensionSetBreak::UnsupportedWall
                                },
                                source: Some(id),
                                why: None,
                            },
                            &path,
                        ));
                    }
                }
            }
            let Some((first, rest)) = ids.split_first() else {
                return Err(fail("Cephenin en az bir duvarı olmalı.", "operation.walls"));
            };
            set.wall = *first;
            if let Some(span) = &mut set.span {
                span.walls = rest.to_vec();
            }
        }
        DimensionSetEdit::RestoreHidden => set.suppressed.clear(),
        DimensionSetEdit::ResetRows => set.row_places.clear(),
        DimensionSetEdit::ResetText { row } => {
            if row.is_some_and(|r| !set.rows.contains(&r)) {
                return Err(fail("Bu sıra grupta yok.", "operation.row"));
            }
        }
        DimensionSetEdit::Detach | DimensionSetEdit::Delete => return Ok(set),
    }
    recipe_ok(&set)?;
    // Text reset can still repair text placement when a source is broken.
    if !matches!(input.operation, DimensionSetEdit::ResetText { .. }) {
        if set.span.is_some() {
            doc.dimension_span_plan(&set)
                .map_err(|r| span_error(r, "operation"))?;
        } else {
            doc.dimension_set_plan(&set).map_err(source_error)?;
        }
    }
    Ok(set)
}
fn planned(doc: &Document, set: &DimensionSet, existing: bool) -> Result<DimensionSetsPlan, Stop> {
    let (members, notes) = if set.span.is_some() {
        let (members, notes) = doc
            .dimension_span_plan(set)
            .map_err(|r| span_error(r, "operation"))?;
        (members, Some(notes))
    } else {
        (doc.dimension_set_plan(set).map_err(source_error)?, None)
    };
    let kept: HashSet<_> = members.iter().filter_map(|m| m.uid).collect();
    let removed = set
        .members
        .iter()
        .filter(|m| !kept.contains(&m.uid))
        .map(|m| m.uid.to_string())
        .collect();
    let reference = set.span.as_ref().map(|s| s.reference).unwrap_or_default();
    let (tight, placed, scales) = value_notes(&members);
    Ok(DimensionSetsPlan {
        set: existing.then_some(set.id),
        entities: members
            .iter()
            .map(|m| Entity::Dimension(m.dimension.clone()))
            .collect(),
        spans: members.iter().map(|m| m.segment.clone()).collect(),
        uids: members
            .iter()
            .filter_map(|m| m.uid.map(|u| u.to_string()))
            .collect(),
        removed,
        revision: doc.revision().to_string(),
        rows: notes
            .as_ref()
            .map(|n| {
                n.rows
                    .iter()
                    .map(|r| DimensionSetRowNote {
                        row: r.row,
                        points: r.points as u32,
                        sources: r.sources as u32,
                        dimensions: r.spans as u32,
                        reference,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        skipped: notes
            .map(|n| n.skipped.iter().map(|id| id.to_string()).collect())
            .unwrap_or_default(),
        tight,
        placed,
        scales,
        rounding: rounding(doc, set, &members),
    })
}
fn success<T>(output: T) -> CommandResult<T> {
    CommandResult::Completed {
        output,
        warnings: Vec::new(),
    }
}
fn written_error(message: String) -> Stop {
    fail(message, "operation")
}
fn visible_result<T>(doc: &Document, layer: &str, output: T) -> CommandResult<T> {
    CommandResult::Completed {
        output,
        warnings: checks::layer(doc, layer).unwrap_or_default(),
    }
}
fn with_warnings<T>(result: CommandResult<T>, extra: Vec<CommandWarning>) -> CommandResult<T> {
    match result {
        CommandResult::Completed { output, mut warnings } => {
            warnings.extend(extra);
            CommandResult::Completed { output, warnings }
        }
        other => other,
    }
}
fn empty_plan(doc: &Document, set: Option<DimensionSetId>) -> DimensionSetsPlan {
    DimensionSetsPlan {
        set,
        entities: Vec::new(),
        spans: Vec::new(),
        removed: Vec::new(),
        uids: Vec::new(),
        revision: doc.revision().to_string(),
        rows: Vec::new(),
        skipped: Vec::new(),
        tight: Vec::new(),
        placed: Vec::new(),
        scales: Vec::new(),
        rounding: Vec::new(),
    }
}

pub mod create {
    use super::*;
    pub fn validate(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsCreateInput,
    ) -> CommandResult<()> {
        match create_recipes(cx.doc, input) {
            Ok(_) => match plan_in(cx.doc, input) {
                CommandResult::Completed { warnings, .. } => CommandResult::Completed {
                    output: (),
                    warnings,
                },
                _ => visible_result(cx.doc, &input.layer_id, ()),
            },
            Err(e) => e.into(),
        }
    }
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsCreateInput,
    ) -> CommandResult<DimensionSetsPlan> {
        plan_in(cx.doc, input)
    }
    pub fn plan_in(
        doc: &Document,
        input: &DimensionSetsCreateInput,
    ) -> CommandResult<DimensionSetsPlan> {
        let plans = create_recipes(doc, input).and_then(|sets| {
            sets.iter()
                .map(|s| planned(doc, s, false))
                .collect::<Result<Vec<_>, _>>()
        });
        match plans {
            // Every edge's chain, in edge order (docs/adr/0700); a façade's four sides in order (docs/adr/0729).
            Ok(plans) => {
                let mut out = empty_plan(doc, None);
                for plan in plans {
                    merge(&mut out, plan);
                }
                let warnings = plan_warnings(&out);
                with_warnings(visible_result(doc, &input.layer_id, out), warnings)
            }
            Err(e) => e.into(),
        }
    }
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsCreateInput,
    ) -> CommandResult<DimensionSetsChanged> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_DIMENSION_SETS_CREATE,
            kentos_commands::CAD_DIMENSION_SETS_VERSION,
            input,
            write,
        )
    }
    fn write(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsCreateInput,
    ) -> CommandResult<DimensionSetsChanged> {
        let sets = match create_recipes(cx.doc, &input) {
            Ok(s) => s,
            Err(e) => return e.into(),
        };
        let mut said = empty_plan(cx.doc, None);
        for set in &sets {
            if let Ok(plan) = planned(cx.doc, set, false) {
                merge(&mut said, plan);
            }
        }
        let warnings = plan_warnings(&said);
        // Every set in one undo step: all written, or none (docs/adr/0700).
        let written = cx
            .doc
            .transact(kentos_domain::dimension_sets::labels::ADD, |doc| {
                sets.into_iter()
                    .map(|mut set| {
                        set.id = DimensionSetId(Uuid::now_v7().into_bytes());
                        let id = set.id;
                        doc.add_dimension_set(set).map(|()| id)
                    })
                    .collect::<Result<Vec<_>, _>>()
            });
        let ids = match written {
            Ok(ids) => ids,
            Err(why) => return written_error(why.to_string()).into(),
        };
        let uids: Vec<String> = ids
            .iter()
            .filter_map(|id| cx.doc.dimension_set(*id))
            .flat_map(|s| s.members.iter().map(|m| m.uid.to_string()))
            .collect();
        with_warnings(visible_result(
            cx.doc,
            &input.layer_id,
            DimensionSetsChanged {
                set: ids[0],
                sets: if ids.len() > 1 || input.around {
                    ids
                } else {
                    Vec::new()
                },
                created: uids.clone(),
                uids,
                changed: Vec::new(),
                removed: Vec::new(),
                revision: cx.doc.revision().to_string(),
            },
        ), warnings)
    }
}
pub mod edit {
    use super::*;
    pub fn validate(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsEditInput,
    ) -> CommandResult<()> {
        match check_edit(cx.doc, input) {
            Ok(s) => visible_result(cx.doc, &s.layer_id, ()),
            Err(e) => e.into(),
        }
    }
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsEditInput,
    ) -> CommandResult<DimensionSetsPlan> {
        let set = match check_edit(cx.doc, input) {
            Ok(s) => s,
            Err(e) => return e.into(),
        };
        let before = match existing(cx.doc, input.id) {
            Ok(set) => set,
            Err(e) => return e.into(),
        };
        let mut out = match input.operation {
            DimensionSetEdit::Delete
            | DimensionSetEdit::Detach
            | DimensionSetEdit::ResetText { .. } => empty_plan(cx.doc, Some(set.id)),
            _ => match planned(cx.doc, &set, true) {
                Ok(o) => o,
                Err(e) => return e.into(),
            },
        };
        match input.operation {
            DimensionSetEdit::Delete => {
                out.removed = before.members.iter().map(|m| m.uid.to_string()).collect()
            }
            DimensionSetEdit::Detach => {
                out.uids = before.members.iter().map(|m| m.uid.to_string()).collect()
            }
            DimensionSetEdit::ResetText { row } => {
                // Each value's place taken as the recipe's last: the plan places it again (docs/adr/0730 §2).
                let current = |uid: Uuid| match cx.doc.slot_of(uid).and_then(|s| cx.doc.get(s)) {
                    Some(Entity::Dimension(d)) => Some(d.clone()),
                    _ => None,
                };
                let mut probe = before.clone();
                for m in &mut probe.members {
                    if row.is_none_or(|r| r == m.row)
                        && let Some(d) = current(Uuid::from_bytes(m.uid.0))
                    {
                        m.shift = d.text_shift;
                    }
                }
                let again = cx.doc.dimension_set_plan(&probe).ok();
                for member in &before.members {
                    if row.is_some_and(|r| r != member.row) {
                        continue;
                    }
                    let uid = Uuid::from_bytes(member.uid.0);
                    let Some(mut d) = current(uid) else {
                        continue;
                    };
                    let place = again
                        .as_ref()
                        .and_then(|ms| ms.iter().find(|p| p.uid == Some(member.uid)))
                        .and_then(|p| p.dimension.text_shift);
                    if d.text_shift != place {
                        d.text_shift = place;
                        out.entities.push(Entity::Dimension(d));
                        out.uids.push(uid.to_string());
                        out.spans.push(member.segment());
                    }
                }
            }
            _ => {
                let kept: HashSet<_> = out.uids.iter().collect();
                out.removed = before
                    .members
                    .iter()
                    .map(|m| m.uid.to_string())
                    .filter(|u| !kept.contains(u))
                    .collect();
            }
        }
        let warnings = plan_warnings(&out);
        with_warnings(visible_result(cx.doc, &set.layer_id, out), warnings)
    }
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsEditInput,
    ) -> CommandResult<DimensionSetsChanged> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_DIMENSION_SETS_EDIT,
            kentos_commands::CAD_DIMENSION_SETS_VERSION,
            input,
            write,
        )
    }
    fn write(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsEditInput,
    ) -> CommandResult<DimensionSetsChanged> {
        let set = match check_edit(cx.doc, &input) {
            Ok(s) => s,
            Err(e) => return e.into(),
        };
        let old = match existing(cx.doc, input.id) {
            Ok(set) => set.clone(),
            Err(e) => return e.into(),
        };
        let before = member_entities(cx.doc, &old);
        let result_layer = set.layer_id.clone();
        // What the plan says of the rows written (docs/adr/0729 §5, §6), said with the write too.
        let said = match input.operation {
            DimensionSetEdit::Detach | DimensionSetEdit::Delete | DimensionSetEdit::ResetText { .. } => {
                Vec::new()
            }
            _ => planned(cx.doc, &set, true)
                .map(|plan| plan_warnings(&plan))
                .unwrap_or_default(),
        };
        let result = cx
            .doc
            .transact("Duvar ölçülerini düzenle", |doc| -> Result<(), String> {
                match &input.operation {
                    DimensionSetEdit::Detach | DimensionSetEdit::Delete => {
                        doc.remove_dimension_set(set.id)
                            .map_err(|e| e.to_string())?;
                        if matches!(input.operation, DimensionSetEdit::Delete) {
                            let slots: Vec<_> = old
                                .members
                                .iter()
                                .filter_map(|m| doc.slot_of(Uuid::from_bytes(m.uid.0)))
                                .collect();
                            doc.remove(&slots);
                        }
                    }
                    DimensionSetEdit::ResetText { row } => {
                        doc.reset_dimension_set_texts(set.id, *row)
                            .map_err(|e| e.to_string())?;
                    }
                    _ => {
                        doc.update_dimension_set(set).map_err(|e| e.to_string())?;
                    }
                }
                Ok(())
            });
        if let Err(message) = result {
            return written_error(message).into();
        }
        let after = cx
            .doc
            .dimension_set(input.id)
            .map(|s| member_entities(cx.doc, s))
            .unwrap_or_else(|| {
                before
                    .keys()
                    .filter_map(|u| {
                        let uid = Uuid::parse_str(u).ok()?;
                        Some((u.clone(), cx.doc.get(cx.doc.slot_of(uid)?)?.clone()))
                    })
                    .collect()
            });
        let uids = cx
            .doc
            .dimension_set(input.id)
            .map(|s| s.members.iter().map(|m| m.uid.to_string()).collect())
            .unwrap_or_else(|| {
                old.members
                    .iter()
                    .map(|m| m.uid.to_string())
                    .filter(|u| after.contains_key(u))
                    .collect()
            });
        let created = after
            .keys()
            .filter(|u| !before.contains_key(*u))
            .cloned()
            .collect();
        let removed = before
            .keys()
            .filter(|u| !after.contains_key(*u))
            .cloned()
            .collect();
        let changed = after
            .iter()
            .filter(|(u, e)| before.get(*u).is_some_and(|b| b != *e))
            .map(|(u, _)| u.clone())
            .collect();
        with_warnings(
            visible_result(
                cx.doc,
                &result_layer,
                DimensionSetsChanged {
                    set: input.id,
                    sets: Vec::new(),
                    uids,
                    created,
                    removed,
                    changed,
                    revision: cx.doc.revision().to_string(),
                },
            ),
            said,
        )
    }
}
pub mod query {
    use super::*;
    fn read(doc: &Document, input: &DimensionSetsQueryInput) -> DimensionSetsQueried {
        DimensionSetsQueried {
            sets: doc
                .dimension_sets()
                .iter()
                .filter(|s| {
                    (input.ids.is_empty() || input.ids.contains(&s.id))
                        && input.wall.is_none_or(|w| s.sources().contains(&w))
                })
                .map(|s| (**s).clone())
                .collect(),
            revision: doc.revision().to_string(),
        }
    }
    pub fn validate(
        _cx: &ExecutionContext<'_>,
        _input: &DimensionSetsQueryInput,
    ) -> CommandResult<()> {
        success(())
    }
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsQueryInput,
    ) -> CommandResult<DimensionSetsQueried> {
        success(read(cx.doc, input))
    }
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsQueryInput,
    ) -> CommandResult<DimensionSetsQueried> {
        success(read(cx.doc, &input))
    }
}

/// Preview the complete derived delta without overloading walls/openings result kinds.
pub(crate) fn derived(
    doc: &Document,
    updates: &[(kentos_domain::Slot, Entity)],
    adds: &[Entity],
    removes: &[kentos_domain::Slot],
) -> kentos_commands::DerivedEntitiesPlan {
    let delta = doc.preview_followers(updates, adds, removes);
    kentos_commands::DerivedEntitiesPlan {
        added: delta.added.into_iter().map(|(_, e)| e).collect(),
        updated: delta.updated.into_iter().map(|(_, e)| e).collect(),
        removed: delta
            .removed
            .into_iter()
            .filter_map(|s| doc.uid(s).map(|u| u.to_string()))
            .collect(),
    }
}
