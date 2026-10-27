//! `cad.grids.create`, `cad.columns.create`, `cad.stairs.create` and `cad.ramps.create` v1 (docs/adr/0343 §4, plan/cad.md
//! `CAD-ARCH-31`–`36`): grids, columns, stairs and ramps on a named layer, each call one undo step named for its kind
//! (“Aks”, “Kolon”, “Merdiven”, “Rampa”). The tools preview the plan's objects and write through `execute`; Python and
//! an agent give the same points and values and get the same objects. The shared cases are
//! `fixtures/commands/v1/cad.{grids,columns,stairs,ramps}.create.json`.
//!
//! The checks, in order (the first that fails answers): 1. at least one object (`no_objects`), at most
//! `MAX_ARCH_ELEMENTS` (the kind's `invalid_…`); 2. each object in the input's order: its numbers (`not_finite`), its
//! grids (columns: `invalid_uid`, `entity_not_found`, `not_a_grid`, `grids_parallel`), its fields (the kind's
//! `invalid_…`), its level (`level_not_found`); 3. the line weight; 4. the expected revision; 5. the layer; 6. the
//! attributes. How a stair's or a ramp's fields fit are warnings, never a refusal.
//!
//! A stair may be drawn from its height (docs/adr/0704 §1): its risers given or as many as keep each under its highest
//! riser, each `height / risers` high; its cut from a cut height; its walking line fitted to end at its last riser.
//! The stored stair is as before: its points (fitted), its risers, their height and its cut.
//!
//! A grid system (docs/adr/0707) writes its two families of grids and, if asked, a column at every crossing tied to
//! its two grids, in the same step: the grids get their persistent ids before they are written so that the columns
//! can name them.

use std::collections::BTreeMap;

use kentos_commands::{
    ArchInfo, ColumnEntity, ColumnFields, ColumnsCreate, ColumnsCreated, ColumnsPlan,
    CommandResult, CommandWarning, Entity, EntityBase, GridEntity, GridFields, GridSystem,
    GridsCreate, GridsCreated, GridsPlan, RampEntity, RampFields, RampsCreate, RampsCreated,
    RampsPlan, SeriesSide, StairEntity, StairFields, StairsCreate, StairsCreated, StairsPlan, Vec2,
};
use kentos_domain::{Document, Uuid};
use kentos_domain::arch_follow;
use kentos_geometry_core::geom::arch;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const GRID_STEP: &str = "Aks";
pub const COLUMN_STEP: &str = "Kolon";
pub const STAIR_STEP: &str = "Merdiven";
pub const RAMP_STEP: &str = "Rampa";

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

type Attrs = Option<BTreeMap<String, String>>;
/// The warnings and the objects as they would be stored, their `id` 0.
type Checked = (Vec<CommandWarning>, Vec<Entity>);
/// An object made: where the input names it, its attributes, and how it is built on its base.
type Made = (String, Attrs, Box<dyn FnOnce(EntityBase) -> Entity>);

/// The kind's `invalid_…` for fields that make none, the field under `path`.
fn valid(problem: Option<(String, String)>, code: &str, path: &str) -> Result<(), Stop> {
    match problem {
        Some((field, words)) => Err(fail(code, words, &format!("{path}.{field}"))),
        None => Ok(()),
    }
}

pub(crate) fn grid_valid(g: &GridFields, path: &str) -> Result<(), Stop> {
    valid(g.problem(), codes::INVALID_GRID, path)
}

pub(crate) fn column_valid(c: &ColumnFields, path: &str) -> Result<(), Stop> {
    valid(c.problem(), codes::INVALID_COLUMN, path)
}

pub(crate) fn stair_valid(s: &StairFields, path: &str) -> Result<(), Stop> {
    valid(s.problem(), codes::INVALID_STAIR, path)
}

pub(crate) fn ramp_valid(r: &RampFields, path: &str) -> Result<(), Stop> {
    valid(r.problem(), codes::INVALID_RAMP, path)
}

fn finite_each(values: &[(Option<f64>, &str, &str)], path: &str) -> Result<(), Stop> {
    for (v, field, name) in values {
        if let Some(x) = v {
            checks::finite(
                *x,
                name,
                "Sonlu bir sayı verin.",
                &format!("{path}.{field}"),
            )?;
        }
    }
    Ok(())
}

fn points(pts: &[Vec2], path: &str) -> Result<(), Stop> {
    for (k, p) in pts.iter().enumerate() {
        checks::point(
            *p,
            &format!("{}. noktanın", k + 1),
            &format!("{path}.pts[{k}]"),
        )?;
    }
    Ok(())
}

/// The stair's warnings: the core's checks (`stair_short`, `stair_long`, `stair_step_rule`) and, its level's height
/// known, `stair_rise_level`.
pub fn stair_warnings(doc: &Document, s: &StairFields, path: &str) -> Vec<CommandWarning> {
    let shape = kentos_formats::arch_elements::core_stair(s);
    let kentos_geometry_core::entity::Shape::Stair { pts, stair } = &shape else {
        return Vec::new();
    };
    let mut out: Vec<CommandWarning> = arch::stair_checks(pts, stair)
        .into_iter()
        .map(|c| CommandWarning {
            code: c.code.into(),
            message: c.words,
            path: Some(path.into()),
        })
        .collect();
    let height = s.arch.level.as_deref().and_then(|id| {
        doc.settings()
            .arch
            .as_ref()
            .and_then(|a| a.levels.iter().find(|l| l.id == id))
            .and_then(|l| l.height)
    });
    if let Some(h) = height {
        let rise = f64::from(s.risers) * s.riser_height;
        if (rise - h).abs() > 1e-3 {
            out.push(CommandWarning {
                code: codes::STAIR_RISE_LEVEL.into(),
                message: format!(
                    "{} rıht × {} m = {} m; katın yüksekliği {} m. Rıht sayısını ya da yüksekliğini düzeltin.",
                    s.risers,
                    kentos_geometry_core::display::fixed(s.riser_height, 3),
                    kentos_geometry_core::display::fixed(rise, 3),
                    kentos_geometry_core::display::fixed(h, 3)
                ),
                path: Some(path.into()),
            });
        }
    }
    out
}

/// The ramp's warnings: `ramp_slope_mismatch`, `ramp_no_run`.
pub fn ramp_warnings(r: &RampFields, path: &str) -> Vec<CommandWarning> {
    let shape = kentos_formats::arch_elements::core_ramp(r);
    let kentos_geometry_core::entity::Shape::Ramp { pts, ramp } = &shape else {
        return Vec::new();
    };
    arch::ramp_checks(pts, ramp)
        .into_iter()
        .map(|c| CommandWarning {
            code: c.code.into(),
            message: c.words,
            path: Some(path.into()),
        })
        .collect()
}

/// The grid with persistent id `uid`, at `path`: `invalid_uid`, `entity_not_found`, `not_a_grid`.
pub(crate) fn grid_named(doc: &Document, uid: &str, path: &str) -> Result<GridFields, Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin."
            ),
            path,
        ));
    }
    let found = kentos_domain::Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s));
    match found {
        None => Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!("“{uid}” kimlikli nesne çizimde yok. Var olan bir aksın kimliğini verin."),
            path,
        )),
        Some(Entity::Grid(g)) => Ok(g.grid.clone()),
        Some(_) => Err(fail(
            codes::NOT_A_GRID,
            "Nesne aks değil; kolon yalnız iki aksın kesişimine bağlanır.".into(),
            path,
        )),
    }
}

/// A column settled on two grids named at `path`: `grids_parallel` when they do not cross.
pub(crate) fn settled_on(
    doc: &Document,
    mut c: ColumnFields,
    has_at: bool,
    path: &str,
) -> Result<ColumnFields, Stop> {
    if let Some(ids) = c.grids.clone() {
        if ids.len() != 2 || ids[0] == ids[1] {
            return Err(fail(
                codes::INVALID_COLUMN,
                "Kolon iki ayrı aksın kesişimine bağlanır; iki aks kimliği verin.".into(),
                &format!("{path}.grids"),
            ));
        }
        let a = grid_named(doc, &ids[0], &format!("{path}.grids[0]"))?;
        grid_named(doc, &ids[1], &format!("{path}.grids[1]"))?;
        if !has_at && a.pts.len() == 2 {
            c.at = a.pts[0];
        }
        c = doc.column_settled(&c);
        if c.broken.is_some() {
            return Err(fail(
                codes::GRIDS_PARALLEL,
                "İki aks kesişmiyor (paralel ya da yayları ayrık); kolon onların kesişimine konamaz."
                    .into(),
                &format!("{path}.grids"),
            ));
        }
    }
    Ok(c)
}

fn base(layer: &str, color: &Option<String>, line_weight: Option<f64>, attrs: Attrs) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: layer.to_owned(),
        color: color.clone(),
        attrs: attrs.unwrap_or_default(),
        label: None,
        symbol: None,
        line_weight,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

/// The common tail of every create: the line weight, the revision, the layer, the attributes; the objects made.
fn finish(
    doc: &Document,
    layer: &str,
    line_weight: Option<f64>,
    expected: Option<&str>,
    made: Vec<Made>,
    color: &Option<String>,
    mut warnings: Vec<CommandWarning>,
) -> Result<Checked, Stop> {
    checks::line_weight(line_weight, "lineWeight")?;
    checks::revision(doc, expected)?;
    let mut out = checks::layer(doc, layer)?;
    let mut entities = Vec::with_capacity(made.len());
    for (path, attrs, make) in made {
        let attrs = checks::object_attrs(doc, layer, attrs.as_ref(), &format!("{path}.attrs"))?;
        entities.push(make(base(layer, color, line_weight, attrs)));
    }
    out.append(&mut warnings);
    Ok((out, entities))
}

fn too_many(n: usize, code: &str, what: &str, path: &str) -> Result<(), Stop> {
    if n > kentos_commands::MAX_ARCH_ELEMENTS {
        return Err(fail(
            code,
            format!(
                "Bir çağrıda en çok {} {what} yazılır; {n} verildi.",
                kentos_commands::MAX_ARCH_ELEMENTS
            ),
            path,
        ));
    }
    Ok(())
}

/// The label of the drawing's last grid (by slot), for the first unlabelled grid's.
fn last_grid_label(doc: &Document) -> Option<String> {
    doc.entities()
        .filter_map(|e| match e {
            Entity::Grid(g) => Some(g.grid.label.clone()),
            _ => None,
        })
        .last()
}

fn next_of(prev: &Option<String>) -> String {
    prev.as_deref()
        .map_or_else(|| "A".to_owned(), kentos_commands::next_label)
}

fn grids_check(doc: &Document, input: &GridsCreate) -> Result<Checked, Stop> {
    grids_with_ids(doc, input).map(|(checked, _)| checked)
}

/// The grids' checks and objects, with the persistent id each object gets (a system's columns name their grids').
fn grids_with_ids(doc: &Document, input: &GridsCreate) -> Result<(Checked, Vec<Uuid>), Stop> {
    let series = input.series.as_ref().map_or(0, |s| s.spacings.len() + 1);
    if input.grids.is_empty() && input.series.is_none() && input.system.is_none() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Yazılacak aks verilmedi. Akslar (grids), bir seri (series) ya da bir aks ağı (system) verin.".into(),
            "grids",
        ));
    }
    too_many(
        input.grids.len() + series,
        codes::INVALID_GRID,
        "aks",
        "grids",
    )?;
    let mut prev = last_grid_label(doc);
    let mut made: Vec<Made> = Vec::new();
    for (i, n) in input.grids.iter().enumerate() {
        let at = format!("grids[{i}]");
        points(&n.pts, &at)?;
        for (k, b) in n.bulges.iter().flatten().enumerate() {
            checks::finite(
                *b,
                "Büküm",
                "Sonlu bir sayı verin.",
                &format!("{at}.bulges[{k}]"),
            )?;
        }
        finite_each(&[(n.bubble_size, "bubbleSize", "Balon çapı")], &at)?;
        let g = GridFields {
            pts: n.pts.clone(),
            bulges: n.bulges.clone(),
            label: n.label.clone().unwrap_or_else(|| next_of(&prev)),
            bubbles: n.bubbles.unwrap_or_default(),
            bubble_size: n.bubble_size,
            arch: n.arch.clone().unwrap_or_default(),
        };
        grid_valid(&g, &at)?;
        crate::walls::level_known(doc, &g.arch, &format!("{at}.arch.level"))?;
        prev = Some(g.label.clone());
        made.push((
            at,
            n.attrs.clone(),
            Box::new(move |base| Entity::Grid(GridEntity { base, grid: g })),
        ));
    }
    if let Some(s) = &input.series {
        checks::point(s.from, "Serinin başının", "series.from")?;
        checks::point(s.to, "Serinin sonunun", "series.to")?;
        for (k, d) in s.spacings.iter().enumerate() {
            let path = format!("series.spacings[{k}]");
            checks::finite(*d, "Aralık", "Sonlu bir sayı verin.", &path)?;
            if *d <= 0.0 {
                return Err(fail(
                    codes::INVALID_GRID,
                    format!("Akslar arası {d}; sıfırdan büyük olmalı."),
                    &path,
                ));
            }
        }
        finite_each(&[(s.bubble_size, "bubbleSize", "Balon çapı")], "series")?;
        let (dx, dy) = (s.to.x - s.from.x, s.to.y - s.from.y);
        let len = dx.hypot(dy);
        if len == 0.0 {
            return Err(fail(
                codes::INVALID_GRID,
                "Serinin ilk aksının başı ve sonu aynı; aksın bir boyu olmalı.".into(),
                "series.to",
            ));
        }
        let (nx, ny) = match s.side.unwrap_or_default() {
            SeriesSide::Left => (-dy / len, dx / len),
            SeriesSide::Right => (dy / len, -dx / len),
        };
        let arch = s.arch.clone().unwrap_or_default();
        crate::walls::level_known(doc, &arch, "series.arch.level")?;
        let mut offset = 0.0;
        let mut label = s.start.clone().unwrap_or_else(|| next_of(&prev));
        for k in 0..=s.spacings.len() {
            if k > 0 {
                offset += s.spacings[k - 1];
                label = kentos_commands::next_label(&label);
            }
            let shift = |p: Vec2| Vec2 {
                x: p.x + nx * offset,
                y: p.y + ny * offset,
            };
            let g = GridFields {
                pts: vec![shift(s.from), shift(s.to)],
                bulges: None,
                label: label.clone(),
                bubbles: s.bubbles.unwrap_or_default(),
                bubble_size: s.bubble_size,
                arch: arch.clone(),
            };
            grid_valid(&g, "series")?;
            made.push((
                "series".into(),
                None,
                Box::new(move |base| Entity::Grid(GridEntity { base, grid: g })),
            ));
        }
    }
    let mut ids: Vec<Uuid> = made.iter().map(|_| Uuid::now_v7()).collect();
    if let Some(s) = &input.system {
        system_made(doc, s, &mut made, &mut ids)?;
    }
    let checked = finish(
        doc,
        &input.layer_id,
        input.line_weight,
        input.expected_revision.as_deref(),
        made,
        &input.color,
        Vec::new(),
    )?;
    Ok((checked, ids))
}

/// The positions along an axis of grids `gaps` apart from 0 on, added in order.
fn along(gaps: &[f64]) -> Vec<f64> {
    let mut at = 0.0;
    let mut out = Vec::with_capacity(gaps.len() + 1);
    out.push(0.0);
    for d in gaps {
        at += d;
        out.push(at);
    }
    out
}

/// A grid system's grids, those across x then those across y, then its columns row by row, with their ids
/// (docs/adr/0707).
fn system_made(
    doc: &Document,
    s: &GridSystem,
    made: &mut Vec<Made>,
    ids: &mut Vec<Uuid>,
) -> Result<(), Stop> {
    checks::point(s.origin, "Aks ağının başlangıcının", "system.origin")?;
    finite_each(
        &[
            (s.angle, "angle", "Açı"),
            (s.extension, "extension", "Uzatma"),
            (s.bubble_size, "bubbleSize", "Balon çapı"),
        ],
        "system",
    )?;
    for (axis, gaps) in [("x", &s.x), ("y", &s.y)] {
        for (k, d) in gaps.iter().enumerate() {
            let path = format!("system.{axis}[{k}]");
            checks::finite(*d, "Aralık", "Sonlu bir sayı verin.", &path)?;
            if *d <= 0.0 {
                return Err(fail(
                    codes::INVALID_GRID,
                    format!("Akslar arası {d}; sıfırdan büyük olmalı."),
                    &path,
                ));
            }
        }
    }
    if let Some(c) = &s.columns {
        finite_each(
            &[
                (Some(c.width), "width", "En"),
                (c.depth, "depth", "Boy"),
                (c.rotation, "rotation", "Dönüş"),
            ],
            "system.columns",
        )?;
    }
    let extension = s.extension.unwrap_or(kentos_commands::DEFAULT_GRID_EXTENSION);
    if extension < 0.0 {
        return Err(fail(
            codes::INVALID_GRID,
            format!("Uzatma {extension}; sıfır ya da daha büyük olmalı."),
            "system.extension",
        ));
    }
    if extension == 0.0 && (s.x.is_empty() || s.y.is_empty()) {
        return Err(fail(
            codes::INVALID_GRID,
            "Bir yönde tek aks varken uzatma sıfır olursa öbür yönün aksları nokta olur; uzatma verin.".into(),
            "system.extension",
        ));
    }
    for (start, path) in [(&s.x_start, "system.xStart"), (&s.y_start, "system.yStart")] {
        if start.as_deref().is_some_and(|t| t.trim().is_empty()) {
            return Err(fail(
                codes::INVALID_GRID,
                "Aksın ilk adı boş; 1, A gibi bir ad verin.".into(),
                path,
            ));
        }
    }
    too_many(
        made.len() + s.x.len() + s.y.len() + 2,
        codes::INVALID_GRID,
        "aks",
        "system",
    )?;
    let info = s.arch.clone().unwrap_or_default();
    crate::walls::level_known(doc, &info, "system.arch.level")?;
    let (xs, ys) = (along(&s.x), along(&s.y));
    let angle = s.angle.unwrap_or(0.0);
    let (cos, sin) = if angle == 0.0 {
        (1.0, 0.0)
    } else {
        (angle.cos(), angle.sin())
    };
    let o = s.origin;
    let world = |x: f64, y: f64| Vec2 {
        x: o.x + (x * cos - y * sin),
        y: o.y + (x * sin + y * cos),
    };
    let (x_last, y_last) = (xs[xs.len() - 1], ys[ys.len() - 1]);
    let family = |positions: &[f64], start: &Option<String>, first: &str, across_x: bool| {
        let mut label = start.clone().unwrap_or_else(|| first.to_owned());
        positions
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                if i > 0 {
                    label = kentos_commands::next_label(&label);
                }
                GridFields {
                    pts: if across_x {
                        vec![world(v, -extension), world(v, y_last + extension)]
                    } else {
                        vec![world(-extension, v), world(x_last + extension, v)]
                    },
                    bulges: None,
                    label: label.clone(),
                    bubbles: s.bubbles.unwrap_or_default(),
                    bubble_size: s.bubble_size,
                    arch: info.clone(),
                }
            })
            .collect::<Vec<GridFields>>()
    };
    let across_x = family(&xs, &s.x_start, "1", true);
    let across_y = family(&ys, &s.y_start, "A", false);
    let mut grid_ids: Vec<Uuid> = Vec::with_capacity(across_x.len() + across_y.len());
    for g in across_x.iter().chain(&across_y) {
        grid_valid(g, "system")?;
        let g = g.clone();
        made.push((
            "system".into(),
            None,
            Box::new(move |base| Entity::Grid(GridEntity { base, grid: g })),
        ));
        let id = Uuid::now_v7();
        grid_ids.push(id);
        ids.push(id);
    }
    let Some(c) = &s.columns else {
        return Ok(());
    };
    too_many(
        across_x.len() * across_y.len(),
        codes::INVALID_COLUMN,
        "kolon",
        "system.columns",
    )?;
    let edges_x: Vec<_> = across_x.iter().map(arch_follow::grid_edge).collect();
    let edges_y: Vec<_> = across_y.iter().map(arch_follow::grid_edge).collect();
    let mut tag = c.tag_start.clone();
    for (j, gy) in across_y.iter().enumerate() {
        for (i, gx) in across_x.iter().enumerate() {
            let (Some(ex), Some(ey)) = (&edges_x[i], &edges_y[j]) else {
                continue;
            };
            let Some(p) = arch::grid_crossings(ex, ey).into_iter().next() else {
                continue;
            };
            let own_tag = tag.clone();
            if let Some(t) = &tag {
                tag = Some(kentos_commands::next_label(t));
            }
            let column = ColumnFields {
                at: Vec2 { x: p.x, y: p.y },
                shape: c.shape.unwrap_or_default(),
                width: c.width,
                depth: c.depth,
                rotation: c.rotation.unwrap_or(angle),
                kind: c.kind.clone(),
                tag: own_tag,
                fill: c.fill.unwrap_or(false),
                grids: Some(vec![
                    grid_ids[i].to_string(),
                    grid_ids[across_x.len() + j].to_string(),
                ]),
                place: Some(arch_follow::place_of(&gx.label, &gy.label)),
                broken: None,
                arch: info.clone(),
            };
            column_valid(&column, "system.columns")?;
            made.push((
                "system.columns".into(),
                None,
                Box::new(move |base| Entity::Column(ColumnEntity { base, column })),
            ));
            ids.push(Uuid::now_v7());
        }
    }
    Ok(())
}

fn columns_check(doc: &Document, input: &ColumnsCreate) -> Result<Checked, Stop> {
    if input.columns.is_empty() && input.crossings.is_none() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Yazılacak kolon verilmedi. Kolonlar (columns) ya da aks kesişimleri (crossings) verin."
                .into(),
            "columns",
        ));
    }
    too_many(
        input.columns.len(),
        codes::INVALID_COLUMN,
        "kolon",
        "columns",
    )?;
    let mut tag = input.tag_start.clone();
    let mut next_tag = |own: &Option<String>| -> Option<String> {
        if own.is_some() {
            return own.clone();
        }
        let t = tag.clone()?;
        tag = Some(kentos_commands::next_label(&t));
        Some(t)
    };
    let mut made: Vec<Made> = Vec::new();
    for (i, n) in input.columns.iter().enumerate() {
        let at = format!("columns[{i}]");
        if let Some(p) = n.at {
            checks::point(p, "Kolonun merkezinin", &format!("{at}.at"))?;
        }
        finite_each(
            &[
                (Some(n.width), "width", "En"),
                (n.depth, "depth", "Boy"),
                (n.rotation, "rotation", "Dönüş"),
            ],
            &at,
        )?;
        if n.at.is_none() && n.grids.is_none() {
            return Err(fail(
                codes::INVALID_COLUMN,
                "Kolonun yeri verilmedi: merkezini (at) ya da iki aksını (grids) verin.".into(),
                &at,
            ));
        }
        let c = ColumnFields {
            at: n.at.unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
            shape: n.shape.unwrap_or_default(),
            width: n.width,
            depth: n.depth,
            rotation: n.rotation.unwrap_or(0.0),
            kind: n.kind.clone(),
            tag: next_tag(&n.tag),
            fill: n.fill.unwrap_or(false),
            grids: n.grids.clone(),
            place: None,
            broken: None,
            arch: n.arch.clone().unwrap_or_default(),
        };
        column_valid(&c, &at)?;
        let c = settled_on(doc, c, n.at.is_some(), &at)?;
        crate::walls::level_known(doc, &c.arch, &format!("{at}.arch.level"))?;
        made.push((
            at,
            n.attrs.clone(),
            Box::new(move |base| Entity::Column(ColumnEntity { base, column: c })),
        ));
    }
    if let Some(x) = &input.crossings {
        finite_each(
            &[
                (Some(x.width), "width", "En"),
                (x.depth, "depth", "Boy"),
                (x.rotation, "rotation", "Dönüş"),
            ],
            "crossings",
        )?;
        if x.grids.len() < 2 {
            return Err(fail(
                codes::INVALID_COLUMN,
                "Aks kesişimleri için en az iki aks verin.".into(),
                "crossings.grids",
            ));
        }
        let mut grids = Vec::with_capacity(x.grids.len());
        for (k, uid) in x.grids.iter().enumerate() {
            let g = grid_named(doc, uid, &format!("crossings.grids[{k}]"))?;
            grids.push((uid.clone(), arch_follow::grid_edge(&g), g));
        }
        let arch = x.arch.clone().unwrap_or_default();
        crate::walls::level_known(doc, &arch, "crossings.arch.level")?;
        for (i, (ua, ea, ga)) in grids.iter().enumerate() {
            for (ub, eb, gb) in &grids[i + 1..] {
                let (Some(ea), Some(eb)) = (ea, eb) else {
                    continue;
                };
                for p in arch::grid_crossings(ea, eb) {
                    let c = ColumnFields {
                        at: Vec2 { x: p.x, y: p.y },
                        shape: x.shape.unwrap_or_default(),
                        width: x.width,
                        depth: x.depth,
                        rotation: x.rotation.unwrap_or(0.0),
                        kind: x.kind.clone(),
                        tag: next_tag(&None),
                        fill: x.fill.unwrap_or(false),
                        grids: Some(vec![ua.clone(), ub.clone()]),
                        place: Some(arch_follow::place_of(&ga.label, &gb.label)),
                        broken: None,
                        arch: arch.clone(),
                    };
                    column_valid(&c, "crossings")?;
                    made.push((
                        "crossings".into(),
                        None,
                        Box::new(move |base| Entity::Column(ColumnEntity { base, column: c })),
                    ));
                }
            }
        }
        too_many(made.len(), codes::INVALID_COLUMN, "kolon", "crossings")?;
    }
    finish(
        doc,
        &input.layer_id,
        input.line_weight,
        input.expected_revision.as_deref(),
        made,
        &input.color,
        Vec::new(),
    )
}

fn stairs_check(doc: &Document, input: &StairsCreate) -> Result<Checked, Stop> {
    if input.stairs.is_empty() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Yazılacak merdiven verilmedi. En az bir merdivenin yürüme çizgisini ve ölçülerini verin."
                .into(),
            "stairs",
        ));
    }
    too_many(
        input.stairs.len(),
        codes::INVALID_STAIR,
        "merdiven",
        "stairs",
    )?;
    let mut warnings = Vec::new();
    let mut made: Vec<Made> = Vec::new();
    for (i, n) in input.stairs.iter().enumerate() {
        let at = format!("stairs[{i}]");
        points(&n.pts, &at)?;
        for (k, b) in n.bulges.iter().flatten().enumerate() {
            checks::finite(
                *b,
                "Büküm",
                "Sonlu bir sayı verin.",
                &format!("{at}.bulges[{k}]"),
            )?;
        }
        finite_each(
            &[
                (Some(n.width), "width", "Genişlik"),
                (Some(n.riser_height), "riserHeight", "Rıht yüksekliği"),
                (Some(n.tread), "tread", "Basamak derinliği"),
                (n.landing, "landing", "Sahanlık"),
                (n.height, "height", "Yükseklik"),
                (n.max_riser, "maxRiser", "En fazla rıht"),
                (n.cut_height, "cutHeight", "Kesme yüksekliği"),
            ],
            &at,
        )?;
        let (risers, riser_height, high) = designed(n, &at)?;
        if n.cut_height.is_some() && n.cut.is_some() {
            return Err(fail(
                codes::INVALID_STAIR,
                "Kesme çizgisi ya rıhtla (cut) ya yükseklikle (cutHeight) verilir; ikisini birden vermeyin.".into(),
                &format!("{at}.cutHeight"),
            ));
        }
        let mut s = StairFields {
            pts: n.pts.clone(),
            bulges: n.bulges.clone(),
            width: n.width,
            risers,
            riser_height,
            tread: n.tread,
            landing: n.landing,
            cut: n.cut,
            numbering: n.numbering.unwrap_or(false),
            railings: n.railings,
            arch: n.arch.clone().unwrap_or_default(),
        };
        stair_valid(&s, &at)?;
        if let Some(h) = n.cut_height {
            positive_or(h, "cutHeight", "Kesme yüksekliği", &at)?;
            // The last riser wholly under the cut plane; at least the first, at most the last.
            let under = (h / s.riser_height + 1e-9).floor();
            s.cut = Some(under.clamp(1.0, f64::from(s.risers)) as u32);
        }
        if n.fit == Some(true) {
            (s.pts, s.bulges) = fitted(&s, &at)?;
            stair_valid(&s, &at)?;
        }
        crate::walls::level_known(doc, &s.arch, &format!("{at}.arch.level"))?;
        warnings.extend(stair_warnings(doc, &s, &at));
        if let Some(max) = high {
            warnings.push(CommandWarning {
                code: codes::STAIR_RISER_HIGH.into(),
                message: format!(
                    "{} rıhtla rıht {} m; en fazla {} m verildi. Rıht sayısını artırın ya da boş bırakın (yükseklikten hesaplanır).",
                    s.risers,
                    kentos_geometry_core::display::fixed(s.riser_height, 3),
                    kentos_geometry_core::display::fixed(max, 3)
                ),
                path: Some(at.clone()),
            });
        }
        made.push((
            at,
            n.attrs.clone(),
            Box::new(move |base| Entity::Stair(StairEntity { base, stair: s })),
        ));
    }
    finish(
        doc,
        &input.layer_id,
        input.line_weight,
        input.expected_revision.as_deref(),
        made,
        &input.color,
        warnings,
    )
}

/// `v` above zero and at most 100, else `invalid_stair` at the field in the fields' own words.
fn positive_or(v: f64, field: &str, whose: &str, at: &str) -> Result<(), Stop> {
    if v.is_finite() && v > 0.0 && v <= 100.0 {
        return Ok(());
    }
    Err(fail(
        codes::INVALID_STAIR,
        format!("{whose} {v}; sıfırdan büyük, en çok 100 olmalı."),
        &format!("{at}.{field}"),
    ))
}

/// A stair's risers and their height (docs/adr/0704 §1): as given, or from its height: its risers given or as many as
/// keep each under its highest riser, each `height / risers` high. Third, the highest riser a given count exceeds.
fn designed(n: &kentos_commands::NewStair, at: &str) -> Result<(u32, f64, Option<f64>), Stop> {
    let Some(h) = n.height else {
        if n.max_riser.is_some() {
            return Err(fail(
                codes::INVALID_STAIR,
                "En fazla rıht yalnız yükseklikle kullanılır; merdivenin yüksekliğini verin.".into(),
                &format!("{at}.maxRiser"),
            ));
        }
        return Ok((n.risers, n.riser_height, None));
    };
    if n.riser_height != 0.0 {
        return Err(fail(
            codes::INVALID_STAIR,
            "Yükseklik verildi; rıht yüksekliği ondan hesaplanır. Rıht yüksekliğini vermeyin ya da yüksekliği kaldırın."
                .into(),
            &format!("{at}.riserHeight"),
        ));
    }
    positive_or(h, "height", "Merdivenin yüksekliği", at)?;
    let max = n.max_riser.unwrap_or(kentos_commands::DEFAULT_MAX_RISER);
    positive_or(max, "maxRiser", "En fazla rıht", at)?;
    let count = if n.risers > 0 {
        f64::from(n.risers)
    } else {
        (h / max - 1e-9).ceil()
    };
    if n.risers == 0 && count > f64::from(kentos_commands::MAX_RISERS) {
        return Err(fail(
            codes::INVALID_STAIR,
            format!(
                "{} m yükseklik en fazla {} m rıhtla {count} rıht ister; en çok {} olur. Rıhtı yükseltin ya da yüksekliği bölün.",
                kentos_geometry_core::display::fixed(h, 3),
                kentos_geometry_core::display::fixed(max, 3),
                kentos_commands::MAX_RISERS
            ),
            &format!("{at}.height"),
        ));
    }
    let riser = h / count;
    let high = (n.risers > 0 && n.max_riser.is_some() && riser > max + 1e-9).then_some(max);
    Ok((count as u32, riser, high))
}

/// The stair's walking line ending at its last riser (docs/adr/0704 §1), its last point moved along its last segment;
/// a curved stair's round its arc's centre (docs/adr/0709), its bulge with it.
fn fitted(s: &StairFields, at: &str) -> Result<(Vec<Vec2>, Option<Vec<f64>>), Stop> {
    let shape = kentos_formats::arch_elements::core_stair(s);
    let kentos_geometry_core::entity::Shape::Stair { pts, stair } = &shape else {
        unreachable!("core_stair makes a stair");
    };
    if stair.bulge != 0.0 {
        let Some((end, bulge)) = arch::stair_fit_arc(pts, stair) else {
            return Err(fail(
                codes::INVALID_STAIR,
                "Döner merdiven rıhtlara göre bitirilemiyor: rıhtlar yürüme çizgisinde tam bir turdan uzun. Yarıçapı büyütün, basamağı kısaltın ya da yüksekliği bölün."
                    .into(),
                &format!("{at}.fit"),
            ));
        };
        let mut out = s.pts.clone();
        out[1] = Vec2 { x: end.x, y: end.y };
        return Ok((out, Some(vec![bulge, 0.0])));
    }
    let Some(end) = arch::stair_fit_end(pts, stair) else {
        return Err(fail(
            codes::INVALID_STAIR,
            "Son kol rıhtlara göre bitirilemiyor: rıhtlar son kola varmadan bitiyor. Son dönüşü başa yaklaştırın, rıht ekleyin ya da son kolu tıklanan yerde bitirin."
                .into(),
            &format!("{at}.fit"),
        ));
    };
    let mut out = s.pts.clone();
    if let Some(last) = out.last_mut() {
        *last = Vec2 { x: end.x, y: end.y };
    }
    Ok((out, s.bulges.clone()))
}

/// The ramp's line ending where its sloped length is its rise over its slope (docs/adr/0705 §1), its last point moved
/// along its last segment.
fn ramp_fitted(r: &RampFields, at: &str) -> Result<Vec<Vec2>, Stop> {
    let refuse = |words: &str| Err(fail(codes::INVALID_RAMP, words.into(), &format!("{at}.fit")));
    if r.slope.is_none() {
        return refuse("Eğim verilmedi; son kol eğime göre bitirilemez. Eğimi verin ya da son kolu tıklanan yerde bitirin.");
    }
    if r.rise == 0.0 {
        return refuse("Kot farkı sıfır; eğimli uzunluk yok, son kol eğime göre bitirilemez. Kot farkını verin ya da son kolu tıklanan yerde bitirin.");
    }
    let shape = kentos_formats::arch_elements::core_ramp(r);
    let kentos_geometry_core::entity::Shape::Ramp { pts, ramp } = &shape else {
        unreachable!("core_ramp makes a ramp");
    };
    let Some(end) = arch::ramp_fit_end(pts, ramp) else {
        return refuse("Son kol eğime göre bitirilemiyor: eğimli uzunluk son kolun sahanlığı bitmeden doluyor. Son dönüşü başa yaklaştırın ya da son kolu tıklanan yerde bitirin.");
    };
    let mut out = r.pts.clone();
    if let Some(last) = out.last_mut() {
        *last = Vec2 { x: end.x, y: end.y };
    }
    Ok(out)
}

fn ramps_check(doc: &Document, input: &RampsCreate) -> Result<Checked, Stop> {
    if input.ramps.is_empty() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Yazılacak rampa verilmedi. En az bir rampanın orta çizgisini ve ölçülerini verin."
                .into(),
            "ramps",
        ));
    }
    too_many(input.ramps.len(), codes::INVALID_RAMP, "rampa", "ramps")?;
    let mut warnings = Vec::new();
    let mut made: Vec<Made> = Vec::new();
    for (i, n) in input.ramps.iter().enumerate() {
        let at = format!("ramps[{i}]");
        points(&n.pts, &at)?;
        finite_each(
            &[
                (Some(n.width), "width", "Genişlik"),
                (Some(n.rise), "rise", "Yükseklik"),
                (n.slope, "slope", "Eğim"),
                (n.base, "base", "Başlangıç kotu"),
                (n.landing, "landing", "Sahanlık"),
            ],
            &at,
        )?;
        let mut r = RampFields {
            pts: n.pts.clone(),
            width: n.width,
            rise: n.rise,
            slope: n.slope,
            base: n.base,
            landing: n.landing,
            railings: n.railings,
            arch: n.arch.clone().unwrap_or_default(),
        };
        ramp_valid(&r, &at)?;
        if n.fit == Some(true) {
            r.pts = ramp_fitted(&r, &at)?;
            ramp_valid(&r, &at)?;
        }
        crate::walls::level_known(doc, &r.arch, &format!("{at}.arch.level"))?;
        warnings.extend(ramp_warnings(&r, &at));
        made.push((
            at,
            n.attrs.clone(),
            Box::new(move |base| Entity::Ramp(RampEntity { base, ramp: r })),
        ));
    }
    finish(
        doc,
        &input.layer_id,
        input.line_weight,
        input.expected_revision.as_deref(),
        made,
        &input.color,
        warnings,
    )
}

fn validated(checked: Result<Checked, Stop>) -> CommandResult<()> {
    match checked {
        Ok((warnings, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

fn planned<P>(
    doc: &Document,
    checked: Result<Checked, Stop>,
    make: impl FnOnce(Vec<Entity>, String) -> P,
) -> CommandResult<P> {
    match checked {
        Ok((warnings, entities)) => CommandResult::Completed {
            output: make(entities, doc.revision().to_string()),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the objects as one undo step: their ids, slots and the revision.
fn written<P>(
    doc: &mut Document,
    checked: Result<Checked, Stop>,
    step: &str,
    make: impl FnOnce(Vec<String>, Vec<u32>, String) -> P,
) -> CommandResult<P> {
    let (warnings, entities) = match checked {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let slots = match doc.add_many(entities, step) {
        Ok(slots) => slots,
        Err(full) => {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
        }
    };
    let uids = slots
        .iter()
        .filter_map(|s| doc.uid(*s))
        .map(|u| u.to_string())
        .collect();
    CommandResult::Completed {
        output: make(
            uids,
            slots.iter().map(|s| s.0).collect(),
            doc.revision().to_string(),
        ),
        warnings,
    }
}

macro_rules! create_command {
    ($module:ident, $input:ty, $plan:ident, $field:ident, $created:ident, $check:ident, $step:expr, $id:ident, $version:ident) => {
        pub mod $module {
            use super::*;

            pub fn validate(cx: &ExecutionContext<'_>, input: &$input) -> CommandResult<()> {
                validated($check(cx.doc, input))
            }

            pub fn plan(cx: &ExecutionContext<'_>, input: &$input) -> CommandResult<$plan> {
                plan_in(cx.doc, input)
            }

            /// [`plan`] on a document: what the tool draws as its preview.
            pub fn plan_in(doc: &Document, input: &$input) -> CommandResult<$plan> {
                planned(doc, $check(doc, input), |$field, revision| $plan {
                    $field,
                    revision,
                })
            }

            pub fn execute(
                cx: &mut ExecutionContext<'_>,
                input: $input,
            ) -> CommandResult<$created> {
                crate::call::recorded(
                    cx,
                    kentos_commands::$id,
                    kentos_commands::$version,
                    input,
                    write_execute,
                )
            }

            fn write_execute(
                cx: &mut ExecutionContext<'_>,
                input: $input,
            ) -> CommandResult<$created> {
                let checked = $check(cx.doc, &input);
                written(cx.doc, checked, $step, |uids, ids, revision| $created {
                    uids,
                    ids,
                    revision,
                })
            }
        }
    };
}

/// `cad.grids.create`: the other kinds' shape, but its objects are written with the ids the check gave them, which a
/// grid system's columns name (docs/adr/0707).
pub mod grids_create {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &GridsCreate) -> CommandResult<()> {
        validated(grids_check(cx.doc, input))
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &GridsCreate) -> CommandResult<GridsPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: what the tool draws as its preview.
    pub fn plan_in(doc: &Document, input: &GridsCreate) -> CommandResult<GridsPlan> {
        planned(doc, grids_check(doc, input), |grids, revision| GridsPlan {
            grids,
            revision,
        })
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: GridsCreate) -> CommandResult<GridsCreated> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_GRIDS_CREATE,
            kentos_commands::CAD_GRIDS_CREATE_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(cx: &mut ExecutionContext<'_>, input: GridsCreate) -> CommandResult<GridsCreated> {
        let ((warnings, entities), ids) = match grids_with_ids(cx.doc, &input) {
            Ok(checked) => checked,
            Err(stop) => return stop.into(),
        };
        let slots = match cx
            .doc
            .add_many_with_uids(ids.into_iter().zip(entities).collect(), GRID_STEP)
        {
            Ok(slots) => slots,
            Err(refusal) => {
                return Stop::failed(error(codes::SLOTS_EXHAUSTED, refusal.to_string(), None))
                    .into();
            }
        };
        let uids = slots
            .iter()
            .filter_map(|s| cx.doc.uid(*s))
            .map(|u| u.to_string())
            .collect();
        CommandResult::Completed {
            output: GridsCreated {
                uids,
                ids: slots.iter().map(|s| s.0).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        }
    }
}
create_command!(
    columns_create,
    ColumnsCreate,
    ColumnsPlan,
    columns,
    ColumnsCreated,
    columns_check,
    COLUMN_STEP,
    CAD_COLUMNS_CREATE,
    CAD_COLUMNS_CREATE_VERSION
);
create_command!(
    stairs_create,
    StairsCreate,
    StairsPlan,
    stairs,
    StairsCreated,
    stairs_check,
    STAIR_STEP,
    CAD_STAIRS_CREATE,
    CAD_STAIRS_CREATE_VERSION
);
create_command!(
    ramps_create,
    RampsCreate,
    RampsPlan,
    ramps,
    RampsCreated,
    ramps_check,
    RAMP_STEP,
    CAD_RAMPS_CREATE,
    CAD_RAMPS_CREATE_VERSION
);

/// The architectural information an object of these kinds carries; none for another.
pub fn arch_of(e: &Entity) -> Option<&ArchInfo> {
    match e {
        Entity::Grid(g) => Some(&g.grid.arch),
        Entity::Column(c) => Some(&c.column.arch),
        Entity::Stair(s) => Some(&s.stair.arch),
        Entity::Ramp(r) => Some(&r.ramp.arch),
        _ => None,
    }
}
