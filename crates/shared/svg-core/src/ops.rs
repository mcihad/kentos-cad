//! The SVG editor's path operations (Inkscape's Path menu) on shapes
//! (`apps/web/src/style/svg/pathOps.ts`): booleans (union, difference,
//! intersection, exclusion, division, cut path), combine and break apart,
//! object to path, stroke to path, inset and outset, simplify, reverse,
//! close and open. Booleans keep curves as curves (`boolean.rs`); stroke
//! outlines and offsets are fitted with cubics within a ten-thousandth of
//! the drawing (`stroke.rs`). Every operation returns the shapes to put in
//! place of the ones it used, or a message for the user.
//!
//! New shapes need new ids, which the page makes (`shapeId`: time and a
//! counter). The core names them by the order the TypeScript made them
//! ("\u{1}0", "\u{1}1" …) and says how many it made; the page puts fresh
//! ids in their place (`apps/web/src/style/svg/pathOps.ts`).

use kentos_geometry_core::api::json::{Json, ToJson, write_str};
use kentos_geometry_core::jsmath::{PI, atan2, js_hypot, js_max};

use crate::bezier::{
    flatten_cubic, flatten_sub_path_tol, reverse_sub_path, ring_signed_area, segment_count,
    segment_cubic, segment_is_line, winding_of,
};
use crate::boolean::{BoolOp, RegionInput, boolean_op, cut_path};
use crate::fit::{FitOptions, fit_polyline};
use crate::model::to_path;
use crate::shape::{Obj, PathNode, Pt, SubPath};
use crate::stroke::{Cap, Join, StrokeStyle, offset_region, stroke_outline};

/// Names for new shapes, in the order the page makes their ids.
#[derive(Default)]
pub struct Ids {
    pub count: usize,
}

impl Ids {
    pub fn fresh(&mut self) -> Json {
        let id = format!("\u{1}{}", self.count);
        self.count += 1;
        Json::Str(id)
    }
}

/// What an operation gives: shapes to add and ids to remove, with a note; or a message.
pub enum OpResult {
    Done {
        add: Vec<Obj>,
        remove: Vec<Json>,
        note: Option<String>,
    },
    Error(String),
}

impl ToJson for OpResult {
    fn write_json(&self, out: &mut String) {
        match self {
            OpResult::Done { add, remove, note } => {
                out.push_str("{\"add\":");
                add.write_json(out);
                out.push_str(",\"remove\":");
                remove.write_json(out);
                if let Some(n) = note {
                    out.push_str(",\"note\":");
                    write_str(out, n);
                }
                out.push('}');
            }
            OpResult::Error(e) => {
                out.push_str("{\"error\":");
                write_str(out, e);
                out.push('}');
            }
        }
    }
}

/// An operation's result with the number of ids the page is to make.
pub struct WithIds {
    pub result: OpResult,
    pub ids: usize,
}

kentos_geometry_core::json_struct!(out WithIds { result, ids });

fn error(e: &str) -> OpResult {
    OpResult::Error(e.to_string())
}

/// The fill rule a shape paints with (paths default to even-odd in this editor, SVG shapes to nonzero).
pub fn is_nonzero(s: &Obj) -> bool {
    match s.field("fillRule") {
        Some(Json::Null) | None => s.kind() != "path",
        Some(v) => matches!(v, Json::Str(ref r) if r == "nonzero"),
    }
}

fn region_of(s: &Obj) -> Result<Option<RegionInput>, String> {
    let p = to_path(s);
    if p.kind() != "path" {
        return Ok(None);
    }
    Ok(Some(RegionInput {
        subs: p.subs()?,
        nonzero: is_nonzero(s),
    }))
}

/// `s.id` as it is (text normally).
fn id_of(s: &Obj) -> Json {
    s.get("id").clone()
}

/// A path shape with the style of `from` (id new unless given).
fn path_like(from: &Obj, subs: &[SubPath], id: Json) -> Obj {
    let mut out = Obj::default();
    out.set("id", id);
    out.set_text("kind", "path");
    out.set_subs(subs);
    for k in [
        "fill",
        "stroke",
        "strokeWidth",
        "opacity",
        "dash",
        "cap",
        "join",
        "group",
        "name",
    ] {
        out.put(k, from.field(k));
    }
    out
}

const TEXT_NOTE: &str = "Yazılar yol işlemlerine katılmaz; seçimde kaldılar.";

fn usable(shapes: &[Obj]) -> Vec<&Obj> {
    shapes.iter().filter(|s| s.kind() != "text").collect()
}

/// A boolean on shapes given bottom to top. The result takes the bottom
/// shape's style and place (Inkscape does the same); division gives one
/// shape per piece.
pub fn boolean_shapes(op: &str, shapes: &[Obj], ids: &mut Ids) -> Result<OpResult, String> {
    let usable = usable(shapes);
    let note = (usable.len() < shapes.len()).then(|| TEXT_NOTE.to_string());
    if usable.len() < 2 {
        return Ok(error(if op == "union" {
            "Birleştirmek için en az iki şekil seçin (yazılar katılmaz)."
        } else {
            "Bu işlem için en az iki şekil seçin: alttaki ve üstündekiler (yazılar katılmaz)."
        }));
    }
    let mut inputs = Vec::new();
    for s in &usable {
        if let Some(r) = region_of(s)? {
            inputs.push(r);
        }
    }
    let results = boolean_op(BoolOp::named(op), &inputs);
    let bottom = usable[0];
    let remove: Vec<Json> = usable.iter().map(|s| id_of(s)).collect();
    if op == "division" {
        let add: Vec<Obj> = results
            .iter()
            .filter(|subs| !subs.is_empty())
            .map(|subs| path_like(bottom, subs, ids.fresh()))
            .collect();
        let note = note.unwrap_or_else(|| {
            if add.is_empty() {
                "Kesişen bir parça çıkmadı.".to_string()
            } else {
                format!("{} parça.", add.len())
            }
        });
        return Ok(OpResult::Done {
            add,
            remove,
            note: Some(note),
        });
    }
    let subs = results.into_iter().next().unwrap_or_default();
    if subs.is_empty() {
        let note = note.unwrap_or_else(|| {
            if op == "intersection" {
                "Ortak alan yok: şekiller silindi (Ctrl+Z geri alır).".to_string()
            } else {
                "Sonuç boş: şekiller silindi (Ctrl+Z geri alır).".to_string()
            }
        });
        return Ok(OpResult::Done {
            add: Vec::new(),
            remove,
            note: Some(note),
        });
    }
    Ok(OpResult::Done {
        add: vec![path_like(bottom, &subs, ids.fresh())],
        remove,
        note,
    })
}

/// The bottom shape's outline cut where the others cross it (open pieces, no fill).
pub fn cut_shapes(shapes: &[Obj], ids: &mut Ids) -> Result<OpResult, String> {
    let usable = usable(shapes);
    if usable.len() < 2 {
        return Ok(error(
            "Yolu kesmek için alttaki şekli ve üstünde onu kesen en az bir şekil seçin.",
        ));
    }
    let paths: Vec<Obj> = usable.iter().map(|s| to_path(s)).collect();
    let bottom = &paths[0];
    let mut cutters = Vec::new();
    for p in &paths[1..] {
        cutters.push(p.subs()?);
    }
    let pieces = cut_path(&bottom.subs()?, &cutters);
    let has_stroke = bottom.text("stroke") != Some("none");
    let stroke: Option<Json> = if has_stroke {
        bottom.field("stroke")
    } else if bottom.text("fill") != Some("none") {
        bottom.field("fill")
    } else {
        Some(Json::Str("fill".into()))
    };
    let width = if has_stroke {
        bottom.field("strokeWidth")
    } else {
        Some(Json::Num(js_max(bottom.num("strokeWidth"), 1.0)))
    };
    let add: Vec<Obj> = pieces
        .iter()
        .map(|sp| {
            let mut s = path_like(bottom, std::slice::from_ref(sp), ids.fresh());
            s.set_text("fill", "none");
            s.put("stroke", stroke.clone());
            s.put("strokeWidth", width.clone());
            s
        })
        .collect();
    let note = format!("{} parça.", add.len());
    Ok(OpResult::Done {
        add,
        remove: usable.iter().map(|s| id_of(s)).collect(),
        note: Some(note),
    })
}

/// Every selected shape's sub-paths in one path (the bottom shape's style).
pub fn combine_shapes(shapes: &[Obj], ids: &mut Ids) -> Result<OpResult, String> {
    let usable = usable(shapes);
    if usable.len() < 2 {
        return Ok(error("Tek yolda toplamak için en az iki şekil seçin."));
    }
    let mut subs = Vec::new();
    for s in &usable {
        subs.extend(to_path(s).subs()?);
    }
    Ok(OpResult::Done {
        add: vec![path_like(usable[0], &subs, ids.fresh())],
        remove: usable.iter().map(|s| id_of(s)).collect(),
        note: None,
    })
}

/// Sub-paths as separate shapes. With `keep_holes` a hole stays with the
/// sub-path around it (the letter O stays one shape).
pub fn break_apart(shape: &Obj, keep_holes: bool, ids: &mut Ids) -> Result<OpResult, String> {
    let p = to_path(shape);
    if p.kind() != "path" {
        return Ok(error("Yazı parçalara ayrılmaz."));
    }
    let subs = p.subs()?;
    if subs.len() < 2 {
        return Ok(error("Bu yol tek parça."));
    }
    let groups: Vec<Vec<SubPath>> = if keep_holes {
        hole_groups(&subs)
    } else {
        subs.iter().map(|sp| vec![sp.clone()]).collect()
    };
    let group = ids.fresh();
    let shared = match shape.get("group") {
        Json::Null => None,
        g => Some(g.clone()),
    };
    let add = groups
        .iter()
        .map(|g| {
            let mut s = path_like(&p, g, ids.fresh());
            let value = match &shared {
                Some(v) => Some(v.clone()),
                None => (groups.len() > 1).then(|| group.clone()),
            };
            s.put("group", value);
            s
        })
        .collect();
    Ok(OpResult::Done {
        add,
        remove: vec![id_of(shape)],
        note: None,
    })
}

/// Sub-paths grouped with the ones they contain at odd depth (holes).
fn hole_groups(subs: &[SubPath]) -> Vec<Vec<SubPath>> {
    let rings: Vec<Vec<Pt>> = subs
        .iter()
        .map(|sp| flatten_sub_path_tol(sp, 0.01))
        .collect();
    let area: Vec<f64> = rings.iter().map(|r| ring_signed_area(r).abs()).collect();
    let inside = |i: usize, j: usize| {
        i != j
            && !rings[i].is_empty()
            && rings[j].len() > 2
            && winding_of(&rings[j], rings[i][0]) != 0
    };
    let depth: Vec<usize> = (0..rings.len())
        .map(|i| (0..rings.len()).filter(|&j| inside(i, j)).count())
        .collect();
    // A Map: groups in the order their first sub-path arrived.
    let mut out: Vec<(usize, Vec<SubPath>)> = Vec::new();
    let mut put =
        |key: usize, sp: &SubPath, append: bool| match out.iter_mut().find(|(k, _)| *k == key) {
            Some((_, list)) if append => list.push(sp.clone()),
            Some((_, list)) => *list = vec![sp.clone()],
            None => out.push((key, vec![sp.clone()])),
        };
    for (i, sp) in subs.iter().enumerate() {
        if depth[i].is_multiple_of(2) {
            put(i, sp, true);
            continue;
        }
        // A hole joins the smallest even-depth ring around it.
        let mut host: Option<usize> = None;
        for j in 0..rings.len() {
            if depth[j].is_multiple_of(2) && inside(i, j) && host.is_none_or(|h| area[j] < area[h])
            {
                host = Some(j);
            }
        }
        match host {
            None => put(i, sp, false),
            Some(h) => put(h, sp, true),
        }
    }
    out.into_iter().map(|(_, list)| list).collect()
}

/// Rectangles and ellipses as editable paths (texts cannot: there are no letter outlines here).
pub fn shapes_to_path(shapes: &[Obj]) -> OpResult {
    let conv: Vec<&Obj> = shapes
        .iter()
        .filter(|s| s.kind() == "rect" || s.kind() == "ellipse")
        .collect();
    let texts = shapes.iter().any(|s| s.kind() == "text");
    if conv.is_empty() {
        return error(if texts {
            "Yazı yola çevrilemez (harf çizimleri yok)."
        } else {
            "Seçilenler zaten yol."
        });
    }
    OpResult::Done {
        add: conv.iter().map(|s| to_path(s)).collect(),
        remove: Vec::new(),
        note: texts.then(|| "Yazılar yola çevrilemez; yerinde kaldılar.".to_string()),
    }
}

/// Default stroke end and corner of a shape (as elementOf writes them).
fn cap_of(s: &Obj) -> Cap {
    match s.get("cap") {
        Json::Null => {
            if s.kind() == "path" {
                Cap::Round
            } else {
                Cap::Butt
            }
        }
        Json::Str(c) => Cap::named(c),
        _ => Cap::Butt,
    }
}

fn join_of(s: &Obj) -> Join {
    match s.get("join") {
        Json::Null => {
            if s.kind() == "path" {
                Join::Round
            } else {
                Join::Miter
            }
        }
        Json::Str(j) => Join::named(j),
        _ => Join::Bevel,
    }
}

fn dash_of(s: &Obj) -> Option<Vec<f64>> {
    match s.get("dash") {
        Json::Arr(items) => Some(
            items
                .iter()
                .map(|v| {
                    <f64 as kentos_geometry_core::api::json::FromJson>::from_json(v)
                        .unwrap_or(f64::NAN)
                })
                .collect(),
        ),
        _ => None,
    }
}

/// The stroke as a filled outline (its paint becomes the fill). A shape that
/// also has a fill keeps it as a shape below the outline, grouped with it.
pub fn stroke_to_path(shapes: &[Obj], ids: &mut Ids) -> Result<OpResult, String> {
    let mut add = Vec::new();
    let mut remove = Vec::new();
    let mut skipped = 0;
    for s in shapes {
        if s.kind() == "text" || s.text("stroke") == Some("none") || !(s.num("strokeWidth") > 0.0) {
            skipped += 1;
            continue;
        }
        let p = to_path(s);
        let outline = stroke_outline(
            &p.subs()?,
            &StrokeStyle {
                width: s.num("strokeWidth"),
                cap: cap_of(s),
                join: join_of(s),
                miter_limit: None,
                dash: dash_of(s),
            },
        );
        if outline.is_empty() {
            skipped += 1;
            continue;
        }
        let filled = s.text("fill") != Some("none");
        let group = if filled {
            match s.get("group") {
                Json::Null => Some(ids.fresh()),
                g => Some(g.clone()),
            }
        } else {
            s.field("group")
        };
        if filled {
            let mut below = s.clone();
            below.set_text("stroke", "none");
            below.put("group", group.clone());
            add.push(below);
        }
        let mut o = path_like(s, &outline, ids.fresh());
        o.put("fill", s.field("stroke"));
        o.set_text("stroke", "none");
        o.set_undefined("dash");
        o.set_undefined("cap");
        o.set_undefined("join");
        o.set_text("fillRule", "nonzero");
        o.put("group", group);
        add.push(o);
        remove.push(id_of(s));
    }
    if add.is_empty() {
        return Ok(error("Çizgisi olan bir şekil seçin (yazılar çevrilmez)."));
    }
    Ok(OpResult::Done {
        add,
        remove,
        note: (skipped > 0).then(|| format!("{skipped} şeklin çizgisi yok, olduğu gibi kaldı.")),
    })
}

/// Each shape's fill grown (d > 0) or shrunk (d < 0) by |d| drawing units.
pub fn offset_shapes(shapes: &[Obj], d: f64, join: Join) -> Result<OpResult, String> {
    let usable = usable(shapes);
    if usable.is_empty() {
        return Ok(error(
            "Büyütmek ya da küçültmek için bir şekil seçin (yazılar katılmaz).",
        ));
    }
    let mut add = Vec::new();
    let mut remove = Vec::new();
    let mut vanished = 0;
    for s in usable {
        let Some(region) = region_of(s)? else {
            continue;
        };
        let subs = offset_region(&region, d, join);
        remove.push(id_of(s));
        if subs.is_empty() {
            vanished += 1;
        } else {
            let mut o = path_like(s, &subs, id_of(s));
            o.set_text("fillRule", "nonzero");
            add.push(o);
        }
    }
    Ok(OpResult::Done {
        add,
        remove,
        note: (vanished > 0).then(|| format!("{vanished} şekil bu kadar küçültülünce kayboldu.")),
    })
}

/// Fewer nodes within `tol`: every sub-path flattened and fitted again with
/// cubics; corners sharper than `corner_deg` stay corners.
pub fn simplify_sub_path(sp: &SubPath, tol: f64, corner_deg: f64) -> SubPath {
    let n = segment_count(sp);
    if n < 1 {
        return sp.clone();
    }
    let mut pts: Vec<Pt> = vec![sp.nodes[0].pt()];
    let mut corners: Vec<f64> = Vec::new();
    let m = sp.nodes.len();
    let turn = |i: usize| -> f64 {
        let node = &sp.nodes[i];
        let prev = &sp.nodes[(i + m - 1) % m];
        let next = &sp.nodes[(i + 1) % m];
        let a = node.in_.unwrap_or([prev.x, prev.y]);
        let b = node.out.unwrap_or([next.x, next.y]);
        let u = [node.x - a[0], node.y - a[1]];
        let w = [b[0] - node.x, b[1] - node.y];
        (atan2(u[0] * w[1] - u[1] * w[0], u[0] * w[0] + u[1] * w[1]).abs() * 180.0) / PI
    };
    if !sp.closed || turn(0) > corner_deg {
        corners.push(0.0);
    }
    for i in 0..n {
        let c = segment_cubic(sp, i);
        if segment_is_line(sp, i) {
            pts.push(c[3]);
        } else {
            let (ps, _) = flatten_cubic(&c, tol / 10.0);
            pts.extend_from_slice(&ps[1..]);
        }
        let end = (i + 1) % m;
        if end != 0 && (turn(end) > corner_deg || (!sp.closed && end == m - 1)) {
            corners.push((pts.len() - 1) as f64);
        }
    }
    if sp.closed {
        pts.pop();
    }
    fit_polyline(
        &pts,
        tol,
        &FitOptions {
            closed: sp.closed,
            corners: &corners,
            corner_deg: 180.0,
        },
    )
}

pub fn simplify_shapes(shapes: &[Obj], tol: f64) -> Result<OpResult, String> {
    let usable = usable(shapes);
    if usable.is_empty() {
        return Ok(error("Sadeleştirmek için bir yol seçin."));
    }
    let mut before = 0;
    let mut after = 0;
    let mut add = Vec::new();
    for s in &usable {
        let mut p = to_path(s);
        let subs_in = p.subs()?;
        // A sub-path the fit cannot shorten stays as it was.
        let subs: Vec<SubPath> = subs_in
            .iter()
            .map(|sp| {
                let out = simplify_sub_path(sp, tol, 35.0);
                if out.nodes.len() < sp.nodes.len() {
                    out
                } else {
                    sp.clone()
                }
            })
            .collect();
        before += subs_in.iter().map(|sp| sp.nodes.len()).sum::<usize>();
        after += subs.iter().map(|sp| sp.nodes.len()).sum::<usize>();
        p.put("id", s.field("id"));
        p.set_subs(&subs);
        add.push(p);
    }
    Ok(OpResult::Done {
        add,
        remove: usable.iter().map(|s| id_of(s)).collect(),
        note: Some(format!("{before} düğüm → {after} düğüm.")),
    })
}

/// A per-sub-path change applied to every path among the shapes.
fn map_subs(shapes: &[Obj], f: fn(&SubPath) -> SubPath, empty: &str) -> Result<OpResult, String> {
    let usable: Vec<&Obj> = shapes
        .iter()
        .filter(|s| matches!(s.kind(), "path" | "rect" | "ellipse"))
        .collect();
    if usable.is_empty() {
        return Ok(error(empty));
    }
    let mut add = Vec::new();
    for s in &usable {
        let mut p = to_path(s);
        let subs: Vec<SubPath> = p.subs()?.iter().map(f).collect();
        p.put("id", s.field("id"));
        p.set_subs(&subs);
        add.push(p);
    }
    Ok(OpResult::Done {
        add,
        remove: usable.iter().map(|s| id_of(s)).collect(),
        note: None,
    })
}

pub fn reverse_shapes(shapes: &[Obj]) -> Result<OpResult, String> {
    map_subs(
        shapes,
        reverse_sub_path,
        "Yönünü çevirmek için bir yol seçin.",
    )
}

/// Open sub-paths closed (an end on the start merges into it).
pub fn close_sub_path(sp: &SubPath) -> SubPath {
    if sp.closed || sp.nodes.len() < 2 {
        return sp.clone();
    }
    let mut nodes = sp.nodes.clone();
    let k = nodes.len();
    let (a, b) = (&nodes[0], &nodes[k - 1]);
    if k > 2 && js_hypot(a.x - b.x, a.y - b.y) < 1e-9 {
        let b_in = b.in_;
        nodes.pop();
        if let Some(h) = b_in {
            nodes[0].in_ = Some(h);
        }
    }
    SubPath {
        closed: true,
        nodes,
    }
}

/// Closed sub-paths opened at their first node, keeping the shape (the closing segment stays).
pub fn open_sub_path(sp: &SubPath) -> SubPath {
    if !sp.closed || sp.nodes.len() < 2 {
        return sp.clone();
    }
    let mut nodes = sp.nodes.clone();
    let first = &mut nodes[0];
    let end = PathNode {
        in_: first.in_,
        ..PathNode::at(first.x, first.y)
    };
    first.in_ = None;
    nodes.push(end);
    SubPath {
        closed: false,
        nodes,
    }
}

pub fn close_shapes(shapes: &[Obj]) -> Result<OpResult, String> {
    map_subs(shapes, close_sub_path, "Kapatmak için bir yol seçin.")
}

pub fn open_shapes(shapes: &[Obj]) -> Result<OpResult, String> {
    map_subs(shapes, open_sub_path, "Açmak için kapalı bir yol seçin.")
}
