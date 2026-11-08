//! `cad.tags.create` and `cad.tags.check` v1 (docs/adr/0344 §1, §2, §4; plan/cad.md `CAD-ARCH-20`, `39`): tags put on
//! objects, one undo step “Etiket”, and the tags and numbers checked, read-only. The Etiket tool previews the plan's
//! tags and writes through `execute`; Python and an agent give the same objects and get the same tags. The shared cases
//! are `fixtures/commands/v1/cad.tags.{create,check}.json`.
//!
//! `cad.tags.create`'s checks, in order (the first that fails answers): 1. at least one tag (`no_objects`), at most
//! `MAX_TAGS` (`invalid_tag`); 2. each tag in the input's order: its object's id (`invalid_uid`), its place
//! (`not_finite`), its object (`entity_not_found`, `not_taggable`), its fields (`invalid_tag`); 3. `all`: its kinds,
//! layers (`layer_not_found`), architectural levels (`level_not_found`) and fields (`invalid_tag`), and at least one object it takes when no tag is given
//! (`no_objects`); 4. the line weight; 5. the expected revision; 6. the layer.
//!
//! A tag's words and the point it ties to are written as the document writes them; its place, when not given, is its
//! object's point for a space, else 4 times its words' height above it. The interactive tools give an opening's tag
//! its place clear of the walls, the swings and the other tags (`opening_places`, docs/adr/3028).

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, NewTag, TagEntity, TagFields, TagFinding,
    TagShape, TaggedKind, TagsCheck, TagsChecked, TagsCreate, TagsCreated, TagsEdit, TagsEdited,
    TagsPlan, Vec2,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const STEP: &str = "Etiket";
pub const EDIT_STEP: &str = "Etiketi düzenle";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

struct Checked {
    warnings: Vec<CommandWarning>,
    tags: Vec<Entity>,
    refreshed: Vec<(Slot, Entity)>,
}

/// The objects the drawing's tags tag, by their persistent ids.
fn tagged(doc: &Document) -> BTreeSet<String> {
    doc.entities()
        .filter_map(|e| match e {
            Entity::Tag(t) => Some(t.tag.of.clone()),
            _ => None,
        })
        .collect()
}

/// A tag's words' height on paper at the project's plot scale, metres.
fn words_height(doc: &Document, size: Option<f64>) -> f64 {
    size.unwrap_or(kentos_commands::TAG_SIZE_MM) * doc.settings().plot_scale / 1000.0
}

/// A tag of `target` from what was given, its words and point written as the document writes them.
fn tag_of(
    doc: &Document,
    read: &kentos_domain::tag_read::TagRead<'_>,
    of: &str,
    target: &Entity,
    n: &NewTag,
) -> Option<TagFields> {
    let anchor = doc.tag_anchor(target);
    let at = match (n.at, anchor) {
        (Some(at), _) => at,
        (None, Some(a)) if target.kind() == "space" => a,
        (None, Some(a)) => Vec2 {
            x: a.x,
            y: a.y + 4.0 * words_height(doc, n.size),
        },
        (None, None) => return None,
    };
    Some(TagFields {
        of: of.to_owned(),
        at,
        content: n.content.clone(),
        shape: n
            .shape
            .unwrap_or_else(|| kentos_commands::default_shape(target.kind())),
        size: n.size,
        leader: n.leader.unwrap_or(false),
        text: read.text(target, n.content.as_deref()),
        anchor,
        broken: None,
    })
}

/// Where the tags of the openings `of` go (docs/adr/3028; ADV-16's “etiketler hiçbir çizginin üstüne binmez”): each
/// beside its wall, clear of the walls, columns and fixtures, the doors' swings, the drawing's tags and the ones placed
/// before it; with `content`, `shape` and `size` as the tags will have them. The interactive tools give these to
/// `cad.tags.create` as the tags' `at`; the command's own place, for Python and agents, stays docs/adr/0344's.
pub fn opening_places(
    doc: &Document,
    of: &[String],
    content: Option<&str>,
    shape: Option<TagShape>,
    size: Option<f64>,
) -> Vec<(String, Vec2)> {
    let read = kentos_domain::tag_read::TagRead::new(doc);
    let mut places = Places::default();
    let mut out = Vec::new();
    for uid in of {
        let Some(target) = doc.tag_target(uid) else {
            continue;
        };
        let Entity::Opening(o) = &target else {
            continue;
        };
        let mut t = TagFields {
            of: uid.clone(),
            at: Vec2 { x: 0.0, y: 0.0 },
            content: content.map(str::to_owned),
            shape: shape.unwrap_or_else(|| kentos_commands::default_shape("opening")),
            size,
            leader: false,
            text: read.text(&target, content),
            anchor: doc.tag_anchor(&target),
            broken: None,
        };
        let Some(at) = places.opening_place(doc, &o.opening, tag_half(doc, &t)) else {
            continue;
        };
        t.at = at;
        places.take(doc, &t);
        out.push((uid.clone(), at));
    }
    out
}

/// A tag's frame's half width and half height on the drawing at the project's scale (docs/adr/3028): its words' box
/// and pad, a hexagon's points, a circle's radius, an ellipse round its first line.
fn tag_half(doc: &Document, t: &TagFields) -> (f64, f64) {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::tag::{Frame, PAD, half_box, height, radius};
    let Shape::Tag { tag: spec, .. } = kentos_formats::arch_elements::core_tag(t) else {
        return (0.0, 0.0);
    };
    let scale = doc.settings().plot_scale;
    let h = height(&spec, scale);
    let (w, b) = half_box(&spec, scale);
    let pad = PAD * h;
    match spec.frame {
        Frame::Circle => {
            let r = radius(&spec, scale);
            (r, r)
        }
        Frame::Hexagon => (w + pad + (b + pad) * 0.577_350_269_189_625_8, b + pad),
        Frame::Ellipse => (std::f64::consts::SQRT_2 * (w + pad), b + pad + h),
        _ => (w + pad, b + pad),
    }
}

/// What an opening's tag must keep clear of (docs/adr/3028; ADV-16's “etiketler hiçbir çizginin üstüne binmez”): the
/// walls, columns and fixtures, the doors' swings, the dimensions' lines and values and the tags' frames, the drawing's
/// and those placed before. Read once, when the first opening's tag asks.
#[derive(Default)]
struct Places {
    read: bool,
    obstacles: Vec<crate::door_audit::Obstacle>,
    doors: Vec<crate::door_audit::Door>,
    rooms: crate::opening_codes::Rooms,
    /// The dimensions' lines, each with what crossing it costs: an extension line little, a dimension line more.
    lines: Vec<([kentos_geometry_core::Vec2; 2], f64)>,
    /// The tags' frames and the dimensions' values: x0, y0, x1, y1.
    taken: Vec<[f64; 4]>,
}

/// Whether the segment `a`–`b` crosses the box `[x0, y0, x1, y1]` (Liang–Barsky).
fn crosses(a: kentos_geometry_core::Vec2, b: kentos_geometry_core::Vec2, r: [f64; 4]) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, a.x - r[0]),
        (dx, r[2] - a.x),
        (-dy, a.y - r[1]),
        (dy, r[3] - a.y),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

/// Whether the segments `a`–`b` and `c`–`d` cross.
fn segments_cross(
    a: kentos_geometry_core::Vec2,
    b: kentos_geometry_core::Vec2,
    c: kentos_geometry_core::Vec2,
    d: kentos_geometry_core::Vec2,
) -> bool {
    let side = |p: kentos_geometry_core::Vec2,
                q: kentos_geometry_core::Vec2,
                r: kentos_geometry_core::Vec2| {
        (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
    };
    let (d1, d2, d3, d4) = (side(c, d, a), side(c, d, b), side(a, b, c), side(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

impl Places {
    fn ready(&mut self, doc: &Document) {
        if self.read {
            return;
        }
        self.read = true;
        self.obstacles = crate::door_audit::obstacles(doc);
        self.doors = crate::door_audit::doors(doc);
        self.rooms = crate::opening_codes::rooms(doc);
        for e in doc.entities() {
            match e {
                Entity::Tag(t) => {
                    let (hw, hh) = tag_half(doc, &t.tag);
                    self.taken.push([
                        t.tag.at.x - hw,
                        t.tag.at.y - hh,
                        t.tag.at.x + hw,
                        t.tag.at.y + hh,
                    ]);
                }
                Entity::Dimension(_) => {
                    let shape = crate::geometry::shape(e);
                    let Some(g) = kentos_geometry_core::entity::dimension_geom(&shape) else {
                        continue;
                    };
                    let Some(l) = kentos_geometry_core::geom::dimension::layout_dimension(&g)
                    else {
                        continue;
                    };
                    self.lines.extend(
                        l.lines
                            .iter()
                            .enumerate()
                            .map(|(i, line)| (*line, if l.ext.contains(&i) { 0.05 } else { 0.5 })),
                    );
                    // Its value: a few figures wide, one high, standing on its baseline (`text_at`) toward the
                    // reader's above.
                    let r = l.rotation.to_radians();
                    let (along, across) = (2.0 * g.height, 0.6 * g.height);
                    let c = kentos_geometry_core::Vec2::new(
                        l.text_at.x - r.sin() * g.height / 2.0,
                        l.text_at.y + r.cos() * g.height / 2.0,
                    );
                    let hw = r.cos().abs() * along + r.sin().abs() * across;
                    let hh = r.sin().abs() * along + r.cos().abs() * across;
                    self.taken.push([c.x - hw, c.y - hh, c.x + hw, c.y + hh]);
                }
                _ => {}
            }
        }
    }

    /// A tag made in this call: later ones keep clear of it.
    fn take(&mut self, doc: &Document, t: &TagFields) {
        let (hw, hh) = tag_half(doc, t);
        self.taken
            .push([t.at.x - hw, t.at.y - hh, t.at.x + hw, t.at.y + hh]);
    }

    /// How much a tag's frame `b` on `level` runs into: a wall, column, fixture or swing 1 each, a dimension line ½
    /// and an extension line 0.05, a tag's or a value's frame ten times its overlap's area (m²: a tenth of a square
    /// metre as much as a wall); 0: clear.
    fn cost(&self, b: [f64; 4], level: &Option<String>) -> f64 {
        use kentos_geometry_core::Vec2 as V;
        let ring = vec![
            V::new(b[0], b[1]),
            V::new(b[2], b[1]),
            V::new(b[2], b[3]),
            V::new(b[0], b[3]),
        ];
        let mut cost = 0.0;
        for o in &self.taken {
            let (w, h) = (
                b[2].min(o[2]) - b[0].max(o[0]),
                b[3].min(o[3]) - b[1].max(o[1]),
            );
            if w > 0.0 && h > 0.0 {
                cost += 10.0 * w * h;
            }
        }
        cost += self
            .obstacles
            .iter()
            .filter(|o| o.level == *level && crate::door_audit::overlaps(&ring, o))
            .count() as f64;
        cost += self
            .doors
            .iter()
            .filter(|d| {
                d.level == *level
                    && d.sweeps
                        .iter()
                        .any(|s| crate::door_audit::zones_meet(&ring, s))
            })
            .count() as f64;
        cost += self
            .lines
            .iter()
            .filter(|([a, c], _)| crosses(*a, *c, b))
            .map(|(_, w)| w)
            .sum::<f64>();
        cost
    }

    /// Where an opening's tag of half size `half` goes: beside its wall on the face a door does not open to (a
    /// window's: the face its room is on), its middle across from the opening's; else the other face, a door's beyond
    /// its swing; else beside the opening along the wall, then a frame's height farther out, each face in turn. Each
    /// place is weighed by what it runs into (`cost`), how far it went and whether it is seen across another wall; the
    /// lightest wins: a tag near its opening over a clear place far from it.
    fn opening_place(
        &mut self,
        doc: &Document,
        f: &kentos_commands::OpeningFields,
        half: (f64, f64),
    ) -> Option<Vec2> {
        use kentos_geometry_core::Vec2 as V;
        self.ready(doc);
        let (c, left, right) = crate::opening_codes::sides(f)?;
        let unit = |a: V, b: V| {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let l = dx.hypot(dy).max(1e-12);
            V::new(dx / l, dy / l)
        };
        let n_left = unit(c, left);
        let n_right = V::new(-n_left.x, -n_left.y);
        let u = V::new(-n_left.y, n_left.x);
        // Half the wall: from the middle to a face (the side points are 0.3 m beyond).
        let t = (c.x - left.x).hypot(c.y - left.y) - 0.3;
        let level = crate::opening_types::level_of(doc, f);
        let swing_left = f.side == kentos_commands::OpeningSide::Left;
        let (first, second) = match f.form {
            kentos_commands::OpeningForm::Door => {
                if swing_left {
                    (n_right, n_left)
                } else {
                    (n_left, n_right)
                }
            }
            _ => {
                let (l, r) = (
                    crate::opening_codes::in_room(&self.rooms, &level, left),
                    crate::opening_codes::in_room(&self.rooms, &level, right),
                );
                match (l, r) {
                    (true, false) => (n_left, n_right),
                    (false, true) => (n_right, n_left),
                    _ if swing_left => (n_right, n_left),
                    _ => (n_left, n_right),
                }
            }
        };
        let (hw, hh) = half;
        let reach = |n: V| hw * n.x.abs() + hh * n.y.abs();
        let gap = 0.5 * words_height(doc, None);
        let base = |n: V| {
            let swing = f.form == kentos_commands::OpeningForm::Door
                && (n.x * n_left.x + n.y * n_left.y > 0.0) == swing_left;
            t + gap + reach(n) + if swing { f.width } else { 0.0 }
        };
        let along = f.width / 2.0 + gap + reach(u);
        // A tag seen across another wall is read as that room's: the line from its opening's face to it crosses only
        // its own wall.
        let host = kentos_domain::Uuid::parse_str(&f.host)
            .ok()
            .and_then(|u| doc.slot_of(u));
        let behind = |from: V, to: V| {
            self.obstacles.iter().any(|o| {
                o.level == level
                    && Some(o.slot) != host
                    && std::iter::once(&o.outer).chain(&o.holes).any(|ring| {
                        (0..ring.len())
                            .any(|i| segments_cross(from, to, ring[i], ring[(i + 1) % ring.len()]))
                    })
            })
        };
        let mut best: Option<(f64, V)> = None;
        for k in 0..3 {
            for n in [first, second] {
                let d = base(n) + k as f64 * (2.0 * reach(n) + gap);
                let face = V::new(c.x + n.x * t, c.y + n.y * t);
                // Along the wall a quarter step at a time, nearest first, out to two steps.
                for q in [
                    0_i32, 1, -1, 2, -2, 3, -3, 4, -4, 5, -5, 6, -6, 7, -7, 8, -8,
                ] {
                    let s = along * f64::from(q) / 4.0;
                    let p = V::new(c.x + n.x * d + u.x * s, c.y + n.y * d + u.y * s);
                    // What it runs into, then how far it went: a step along the wall as much as a little
                    // overlap, a frame farther out twice that.
                    let mut cost = self.cost([p.x - hw, p.y - hh, p.x + hw, p.y + hh], &level)
                        + 0.15 * f64::from(q.abs()) / 4.0
                        + 0.3 * f64::from(k);
                    if behind(face, p) {
                        cost += 2.0;
                    }
                    if cost <= 0.0 {
                        return Some(Vec2 { x: p.x, y: p.y });
                    }
                    if best.is_none_or(|(b, _)| cost < b - 1e-9) {
                        best = Some((cost, p));
                    }
                }
            }
        }
        best.map(|(_, p)| Vec2 { x: p.x, y: p.y })
    }
}

fn invalid(words: String, path: &str) -> Stop {
    fail(codes::INVALID_TAG, words, path)
}

/// Outer None means unknown (for example, an orphan opening), inner None means
/// explicitly unassigned. Never infer a floor from CAD layers or coordinates.
fn source_level<'a>(
    entity: &'a Entity,
    by_uid: impl Fn(&str) -> Option<&'a Entity>,
) -> Option<Option<&'a str>> {
    let arch = match entity {
        Entity::Wall(e) => &e.wall.arch,
        Entity::Space(e) => &e.space.arch,
        Entity::Slab(e) => &e.slab.arch,
        Entity::Column(e) => &e.column.arch,
        Entity::Grid(e) => &e.grid.arch,
        Entity::Stair(e) => &e.stair.arch,
        Entity::Ramp(e) => &e.ramp.arch,
        Entity::Opening(e) => match by_uid(&e.opening.host)? {
            Entity::Wall(w) => &w.wall.arch,
            _ => return None,
        },
        Entity::Insert(_) => return Some(None),
        _ => return None,
    };
    Some(arch.level.as_deref())
}

fn check(doc: &Document, input: &TagsCreate) -> Result<Checked, Stop> {
    if input.tags.is_empty() && input.all.is_none() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Etiketlenecek nesne verilmedi. Etiketler (tags) ya da türler (all) verin.".into(),
            "tags",
        ));
    }
    if input.tags.len() > kentos_commands::MAX_TAGS {
        return Err(invalid(
            format!(
                "Bir çağrıda en çok {} etiket yazılır; {} verildi.",
                kentos_commands::MAX_TAGS,
                input.tags.len()
            ),
            "tags",
        ));
    }
    let read = kentos_domain::tag_read::TagRead::new(doc);
    let mut made: Vec<TagFields> = Vec::new();
    let mut done: BTreeSet<String> = tagged(doc);
    let mut explicit = BTreeSet::new();
    let mut refreshed = Vec::new();
    for (i, n) in input.tags.iter().enumerate() {
        let at = format!("tags[{i}]");
        if !checks::is_uid_text(&n.of) {
            return Err(fail(
                codes::INVALID_UID,
                format!(
                    "“{}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin.",
                    n.of
                ),
                &format!("{at}.of"),
            ));
        }
        if let Some(p) = n.at {
            checks::point(p, "Etiketin yerinin", &format!("{at}.at"))?;
        }
        if let Some(s) = n.size {
            checks::finite(
                s,
                "Yazı yüksekliği",
                "Sonlu bir sayı verin.",
                &format!("{at}.size"),
            )?;
        }
        let Some(target) = doc.tag_target(&n.of) else {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{}” kimlikli nesne çizimde yok. Var olan bir nesnenin kimliğini verin.",
                    n.of
                ),
                &format!("{at}.of"),
            ));
        };
        if TaggedKind::of_kind(target.kind()).is_none() {
            return Err(fail(
                codes::NOT_TAGGABLE,
                "Bu nesne etiketlenmez: açıklık, mahal, kolon, aks, duvar, merdiven, rampa ya da blok etiketlenir.".into(),
                &format!("{at}.of"),
            ));
        }
        let Some(t) = tag_of(doc, &read, &n.of, &target, n) else {
            return Err(invalid(
                "Nesnenin bağ noktası bulunamadı; etiketin yerini (at) verin.".into(),
                &format!("{at}.at"),
            ));
        };
        if let Some((field, words)) = t.problem() {
            return Err(invalid(words, &format!("{at}.{field}")));
        }
        done.insert(n.of.clone());
        explicit.insert(n.of.clone());
        made.push(t);
    }
    if let Some(all) = &input.all {
        if all.kinds.is_empty() {
            return Err(fail(
                codes::NO_OBJECTS,
                "Etiketlenecek tür verilmedi; en az bir tür verin.".into(),
                "all.kinds",
            ));
        }
        for (k, l) in all.layers.iter().flatten().enumerate() {
            if doc.layers().get(l).is_none() {
                return Err(fail(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{l}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    &format!("all.layers[{k}]"),
                ));
            }
        }
        let levels = doc
            .settings()
            .arch
            .as_ref()
            .map_or(&[][..], |a| a.levels.as_slice());
        for (index, level) in all.levels.iter().flatten().enumerate() {
            if let Some(problem) = kentos_commands::level_problem(level.as_deref(), levels) {
                return Err(fail(
                    codes::LEVEL_NOT_FOUND,
                    problem,
                    &format!("all.levels[{index}]"),
                ));
            }
        }
        let probe = TagFields {
            of: "-".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            content: all.content.clone(),
            shape: all.shape.unwrap_or_default(),
            size: all.size,
            leader: false,
            text: String::new(),
            anchor: None,
            broken: None,
        };
        if let Some((field, words)) = probe.problem() {
            return Err(invalid(words, &format!("all.{field}")));
        }
        let kinds: Vec<&str> = all.kinds.iter().map(|k| k.kind()).collect();
        let n = NewTag {
            of: String::new(),
            at: None,
            content: all.content.clone(),
            shape: all.shape,
            size: all.size,
            leader: None,
        };
        let mut found = 0usize;
        // One pass indexes the target layer. Do not scan every tag per source.
        let mut existing: BTreeMap<&str, Vec<&TagEntity>> = BTreeMap::new();
        if all.refresh == Some(true) {
            for e in doc.entities() {
                if let Entity::Tag(t) = e
                    && t.base.layer_id == input.layer_id
                {
                    existing.entry(&t.tag.of).or_default().push(t);
                }
            }
        }
        let by_uid = |u: &str| {
            kentos_domain::Uuid::parse_str(u)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| doc.get(s))
        };
        let source_levels = all
            .levels
            .as_ref()
            .map(|ls| ls.iter().map(|l| l.as_deref()).collect::<BTreeSet<_>>());
        for e in doc.entities() {
            let layer = &e.base().layer_id;
            if !kinds.contains(&e.kind())
                || !doc.layers().is_visible(layer)
                || all.layers.as_ref().is_some_and(|ls| !ls.contains(layer))
                || source_levels.as_ref().is_some_and(|ls| {
                    !source_level(e, by_uid).is_some_and(|level| ls.contains(&level))
                })
                || all
                    .phases
                    .as_ref()
                    .is_some_and(|ps| !e.phase(by_uid).is_some_and(|p| ps.contains(&p)))
            {
                continue;
            }
            let Some(uid) = doc.uid(Slot(e.base().id)).map(|u| u.to_string()) else {
                continue;
            };
            if done.contains(&uid) {
                if !explicit.contains(&uid)
                    && let Some(tags) = existing.get(uid.as_str())
                {
                    for tag in tags {
                        let mut next = (*tag).clone();
                        if let Some(content) = &all.content {
                            next.tag.content = Some(content.clone());
                        }
                        if let Some(shape) = all.shape {
                            next.tag.shape = shape;
                        }
                        if let Some(size) = all.size {
                            next.tag.size = Some(size);
                        }
                        next.tag = read.settled(&next.tag, false);
                        if next != **tag {
                            refreshed.push((Slot(next.base.id), Entity::Tag(next)));
                            found += 1;
                            if made.len() + refreshed.len() > kentos_commands::MAX_TAGS {
                                return Err(invalid(
                                    format!(
                                        "Bir çağrıda en çok {} etiket yazılır.",
                                        kentos_commands::MAX_TAGS
                                    ),
                                    "all",
                                ));
                            }
                        }
                    }
                }
                continue;
            }
            if let Some(t) = tag_of(doc, &read, &uid, e, &n) {
                done.insert(uid);
                made.push(t);
                found += 1;
                if made.len() + refreshed.len() > kentos_commands::MAX_TAGS {
                    return Err(invalid(
                        format!(
                            "Bir çağrıda en çok {} etiket yazılır.",
                            kentos_commands::MAX_TAGS
                        ),
                        "all",
                    ));
                }
            }
        }
        if found == 0 && input.tags.is_empty() {
            return Err(fail(
                codes::NO_OBJECTS,
                if all.refresh == Some(true) {
                    "Verilen kapsamda yeni ya da hedef katmanda değişecek etiket yok.".into()
                } else {
                    "Verilen türlerde etiketsiz ve görünen nesne yok.".into()
                },
                "all.kinds",
            ));
        }
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    // Tags with no words yet (a door without a number): written, they fill when their objects get the values.
    let empty = made.iter().filter(|t| t.text.is_empty()).count()
        + refreshed
            .iter()
            .filter(|(_, e)| matches!(e, Entity::Tag(t) if t.tag.text.is_empty()))
            .count();
    if empty > 0 {
        warnings.push(CommandWarning {
            code: codes::TAG_EMPTY.into(),
            message: format!(
                "{empty} etiketin yazısı şimdilik boş: nesnelerinin şablondaki değerleri yok (numarasız açıklık gibi). Değerler verilince etiketler kendiliğinden dolar; açıklıklara Açıklıkları numarala ile numara verin."
            ),
            path: None,
        });
    }
    let entities = made
        .into_iter()
        .map(|tag| {
            Entity::Tag(TagEntity {
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: input.layer_id.clone(),
                    color: input.color.clone(),
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                    line_weight: input.line_weight,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
                tag,
            })
        })
        .collect();
    Ok(Checked {
        warnings,
        tags: entities,
        refreshed,
    })
}

pub mod create {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &TagsCreate) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(Checked { warnings, .. }) => CommandResult::Completed {
                output: (),
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &TagsCreate) -> CommandResult<TagsPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: what the Etiket tool draws as its preview.
    pub fn plan_in(doc: &Document, input: &TagsCreate) -> CommandResult<TagsPlan> {
        match check(doc, input) {
            Ok(Checked {
                warnings,
                tags,
                refreshed,
            }) => CommandResult::Completed {
                output: TagsPlan {
                    tags,
                    refreshed: refreshed.into_iter().map(|(_, e)| e).collect(),
                    revision: doc.revision().to_string(),
                },
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: TagsCreate) -> CommandResult<TagsCreated> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_TAGS_CREATE,
            kentos_commands::CAD_TAGS_CREATE_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: TagsCreate,
    ) -> CommandResult<TagsCreated> {
        let Checked {
            warnings,
            tags,
            refreshed,
        } = match check(cx.doc, &input) {
            Ok(checked) => checked,
            Err(stop) => return stop.into(),
        };
        let refreshed_uids = refreshed
            .iter()
            .filter_map(|(s, _)| cx.doc.uid(*s))
            .map(|u| u.to_string())
            .collect();
        let slots = match cx
            .doc
            .transact(STEP, |doc| -> Result<_, kentos_domain::SlotsExhausted> {
                let slots = doc.add_many(tags, STEP)?;
                doc.update_many(refreshed, STEP);
                Ok(slots)
            }) {
            Ok(slots) => slots,
            Err(full) => {
                return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
            }
        };
        CommandResult::Completed {
            output: TagsCreated {
                uids: slots
                    .iter()
                    .filter_map(|s| cx.doc.uid(*s))
                    .map(|u| u.to_string())
                    .collect(),
                ids: slots.iter().map(|s| s.0).collect(),
                refreshed: refreshed_uids,
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        }
    }
}

/// `cad.tags.edit`'s checks, in its order; the tags as they would be.
fn edit_check(doc: &Document, input: &TagsEdit) -> Result<Vec<(Slot, Entity)>, Stop> {
    checks::uids(&input.uids, "Düzenlenecek etiket verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    for (at, _, e, _) in &found {
        if !matches!(e, Entity::Tag(_)) {
            return Err(fail(
                codes::NOT_A_TAG,
                "Nesne etiket değil; Etiketi düzenle yalnız etiketleri değiştirir.".into(),
                &format!("uids[{at}]"),
            ));
        }
    }
    let c = &input.change;
    if c.content.is_none() && c.shape.is_none() && c.size.is_none() && c.leader.is_none() {
        return Err(fail(
            codes::NOTHING_TO_SET,
            "Değiştirilecek bir alan verilmedi.".into(),
            "change",
        ));
    }
    if let Some(Some(s)) = c.size {
        checks::finite(s, "Yazı yüksekliği", "Sonlu bir sayı verin.", "change.size")?;
    }
    let read = kentos_domain::tag_read::TagRead::new(doc);
    let mut out = Vec::with_capacity(found.len());
    for (at, slot, e, _) in &found {
        let Entity::Tag(t) = e else { continue };
        let mut t = t.clone();
        if let Some(v) = &c.content {
            t.tag.content = v.clone();
        }
        if let Some(v) = c.shape {
            t.tag.shape = v;
        }
        if let Some(v) = c.size {
            t.tag.size = v;
        }
        if let Some(v) = c.leader {
            t.tag.leader = v;
        }
        if let Some((field, words)) = t.tag.problem() {
            let path = if field == "content" || field == "size" {
                format!("change.{field}")
            } else {
                format!("uids[{at}]")
            };
            return Err(invalid(words, &path));
        }
        t.tag = read.settled(&t.tag, false);
        out.push((*slot, Entity::Tag(t)));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc
                .layers()
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki etiket düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    Ok(out)
}

pub mod edit {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &TagsEdit) -> CommandResult<()> {
        match edit_check(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &TagsEdit) -> CommandResult<TagsEdited> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: what Öznitelikler previews.
    pub fn plan_in(doc: &Document, input: &TagsEdit) -> CommandResult<TagsEdited> {
        match edit_check(doc, input) {
            Ok(work) => CommandResult::Completed {
                output: TagsEdited {
                    tags: work.into_iter().map(|(_, e)| e).collect(),
                    revision: doc.revision().to_string(),
                },
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: TagsEdit) -> CommandResult<TagsEdited> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_TAGS_EDIT,
            kentos_commands::CAD_TAGS_EDIT_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(cx: &mut ExecutionContext<'_>, input: TagsEdit) -> CommandResult<TagsEdited> {
        let work = match edit_check(cx.doc, &input) {
            Ok(work) => work,
            Err(stop) => return stop.into(),
        };
        let tags = work.iter().map(|(_, e)| e.clone()).collect();
        cx.doc.update_many(work, EDIT_STEP);
        CommandResult::Completed {
            output: TagsEdited {
                tags,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        }
    }
}

/// What `cad.tags.check` finds (docs/adr/0344 §2).
fn findings(doc: &Document, input: &TagsCheck) -> Result<Vec<TagFinding>, Stop> {
    for (k, l) in input.layers.iter().flatten().enumerate() {
        if doc.layers().get(l).is_none() {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{l}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &format!("layers[{k}]"),
            ));
        }
    }
    let takes = |e: &Entity| {
        input
            .layers
            .as_ref()
            .is_none_or(|ls| ls.contains(&e.base().layer_id))
    };
    let uid = |e: &Entity| {
        doc.uid(Slot(e.base().id))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    let point = |e: &Entity| match e {
        Entity::Tag(t) => t.tag.at,
        _ => doc.tag_anchor(e).unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
    };
    let tags = tagged(doc);
    let mut duplicates: Vec<(String, Vec<&Entity>)> = Vec::new();
    // Keep the original first-appearance order without a quadratic group lookup.
    let mut groups = BTreeMap::new();
    let mut missing_numbers = Vec::new();
    let (mut no_type, mut off, mut gone, mut untagged) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for e in doc.entities() {
        if !takes(e) {
            continue;
        }
        let number = match e {
            Entity::Opening(o) => o.opening.number.as_ref().map(|n| ("opening", n)),
            Entity::Space(s) => s.space.number.as_ref().map(|n| ("space", n)),
            _ => None,
        };
        if let Some((kind, n)) = number.filter(|(_, n)| !n.is_empty()) {
            let key = format!("{kind}\u{0}{n}");
            let index = *groups.entry(key.clone()).or_insert_with(|| {
                let index = duplicates.len();
                duplicates.push((key, Vec::new()));
                index
            });
            duplicates[index].1.push(e);
        }
        if input.missing_space_numbers == Some(true)
            && matches!(e, Entity::Space(s) if s.space.number.as_deref().is_none_or(|n| n.trim().is_empty()))
        {
            missing_numbers.push(e);
        }
        match e {
            Entity::Opening(o) => {
                if o.opening.style.as_deref().is_none_or(str::is_empty) {
                    no_type.push(e);
                }
                if o.opening.broken.is_some() {
                    off.push(e);
                }
            }
            Entity::Tag(t) if doc.tag_target(&t.tag.of).is_none() => gone.push(e),
            _ => {}
        }
        if matches!(e, Entity::Opening(_) | Entity::Space(_)) && !tags.contains(&uid(e)) {
            untagged.push(e);
        }
    }
    let mut out = Vec::new();
    let mut push = |code: &str, message: String, list: Vec<&Entity>| {
        out.push(TagFinding {
            code: code.into(),
            message,
            uids: list.iter().map(|e| uid(e)).collect(),
            at: list.first().map_or(Vec2 { x: 0.0, y: 0.0 }, |e| point(e)),
        });
    };
    for (key, list) in duplicates {
        if list.len() > 1 {
            let (kind, n) = key.split_once('\u{0}').unwrap_or(("", ""));
            let what = if kind == "opening" {
                "açıklıkta"
            } else {
                "mahalde"
            };
            push(
                codes::TAG_DUPLICATE_NUMBER,
                format!("“{n}” numarası {} {what}.", list.len()),
                list,
            );
        }
    }
    for e in missing_numbers {
        let Entity::Space(s) = e else { continue };
        let name = s.space.name.as_deref().unwrap_or("Adsız");
        let level = s.space.arch.level.as_ref().map_or("Kat atanmamış", |id| {
            doc.settings()
                .arch
                .as_ref()
                .and_then(|a| a.levels.iter().find(|l| l.id == *id))
                .map_or(id.as_str(), |l| l.name.as_str())
        });
        push(
            codes::TAG_MISSING_SPACE_NUMBER,
            format!("“{name}” mahalının numarası yok ({level})."),
            vec![e],
        );
    }
    for e in no_type {
        let n = match e {
            Entity::Opening(o) => o.opening.number.clone().unwrap_or_default(),
            _ => String::new(),
        };
        let words = if n.is_empty() {
            "Numarasız açıklığın türü yok.".to_owned()
        } else {
            format!("“{n}” açıklığının türü yok.")
        };
        push(codes::TAG_MISSING_TYPE, words, vec![e]);
    }
    for e in off {
        let why = match e {
            Entity::Opening(o) => o.opening.broken.map_or("", |b| b.words()),
            _ => "",
        };
        push(codes::TAG_HOST_BROKEN, why.to_owned(), vec![e]);
    }
    for e in gone {
        let words = match e {
            Entity::Tag(t) if !t.tag.text.is_empty() => {
                format!(
                    "“{}” etiketinin nesnesi çizimde yok.",
                    t.tag.text.replace('\n', " ")
                )
            }
            _ => "Etiketin nesnesi çizimde yok.".to_owned(),
        };
        push(codes::TAG_TARGET_GONE, words, vec![e]);
    }
    for e in untagged {
        let (what, n) = match e {
            Entity::Opening(o) => ("açıklık", o.opening.number.clone()),
            Entity::Space(s) => ("mahal", s.space.number.clone()),
            _ => ("", None),
        };
        let words = match n.filter(|n| !n.is_empty()) {
            Some(n) => format!("“{n}” {what} etiketsiz."),
            None => format!("Numarasız {what} etiketsiz."),
        };
        push(codes::TAG_UNTAGGED, words, vec![e]);
    }
    Ok(out)
}

pub mod check_tags {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &TagsCheck) -> CommandResult<()> {
        match findings(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &TagsCheck) -> CommandResult<TagsChecked> {
        plan_in(cx.doc, input)
    }

    pub fn plan_in(doc: &Document, input: &TagsCheck) -> CommandResult<TagsChecked> {
        match findings(doc, input) {
            Ok(findings) => CommandResult::Completed {
                output: TagsChecked {
                    findings,
                    revision: doc.revision().to_string(),
                },
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(cx: &mut ExecutionContext<'_>, input: TagsCheck) -> CommandResult<TagsChecked> {
        plan_in(cx.doc, &input)
    }
}

/// The frame a new tag of `kind` takes when the tool names none.
pub fn shape_for(kind: &str) -> TagShape {
    kentos_commands::default_shape(kind)
}

#[cfg(test)]
mod level_scope_tests {
    use super::source_level;
    use kentos_commands::Entity;
    use serde_json::{Value, json};

    fn entity(kind: &str, fields: Value) -> Entity {
        let mut e = json!({"kind":kind,"id":1,"layerId":"drawing","attrs":{}});
        e.as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        serde_json::from_value(e).unwrap()
    }

    #[test]
    fn all_eight_source_kinds_use_explicit_architectural_floor_only() {
        let fixtures = [
            (
                "wall",
                json!({"pts":[{"x":0.,"y":0.},{"x":5.,"y":0.}],"width":0.2}),
            ),
            ("space", json!({"parts":[]})),
            ("column", json!({"at":{"x":0.,"y":0.},"width":0.4})),
            (
                "grid",
                json!({"pts":[{"x":0.,"y":0.},{"x":5.,"y":0.}],"name":"A"}),
            ),
            (
                "stair",
                json!({"pts":[{"x":0.,"y":0.},{"x":5.,"y":0.}],"width":1.,"risers":16,"riserHeight":0.175,"tread":0.3}),
            ),
            (
                "ramp",
                json!({"pts":[{"x":0.,"y":0.},{"x":5.,"y":0.}],"width":1.,"rise":0.5}),
            ),
        ];
        for (kind, fields) in fixtures {
            let unassigned = entity(kind, fields.clone());
            assert_eq!(source_level(&unassigned, |_| None), Some(None), "{kind}");
            let mut assigned = fields;
            assigned["arch"] = json!({"level":"ground"});
            let assigned = entity(kind, assigned);
            assert_eq!(
                source_level(&assigned, |_| None),
                Some(Some("ground")),
                "{kind}"
            );
        }
        let wall = entity(
            "wall",
            json!({"pts":[{"x":0.,"y":0.},{"x":5.,"y":0.}],"width":0.2,"arch":{"level":"ground"}}),
        );
        let opening = entity(
            "opening",
            json!({"host":"wall-uid","at":2.,"width":1.,"place":[{"x":1.,"y":0.1},{"x":1.,"y":-0.1},{"x":2.,"y":0.1},{"x":2.,"y":-0.1}]}),
        );
        assert_eq!(
            source_level(&opening, |u| (u == "wall-uid").then_some(&wall)),
            Some(Some("ground"))
        );
        assert_eq!(source_level(&opening, |_| None), None);
        let wrong = entity("space", json!({"parts":[],"arch":{"level":"ground"}}));
        assert_eq!(source_level(&opening, |_| Some(&wrong)), None);
        let block = entity(
            "insert",
            json!({"block":"00000000-0000-4000-8000-000000000001","p":{"x":0.,"y":0.},"scale":1.,"rotation":0.}),
        );
        assert_eq!(source_level(&block, |_| None), Some(None));
    }
}
