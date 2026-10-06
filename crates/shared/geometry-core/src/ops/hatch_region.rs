//! A hatch's region (docs/adr/0186 §5): the closed object's area around the
//! point clicked inside it (the seed), less its islands' areas and the boxes
//! of the texts and inserts left open; the part that holds the seed, else
//! the largest. Tarama, Çoklu tara and an associative hatch following its
//! objects (§6, both documents) cut it here.

use crate::api::Op;
use crate::api::json::Json;
use crate::block::Blocks;
use crate::entity::{Entity, Shape, TextPlace, polygon_ring};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::region::{inside_area, intersect_areas, net_area, subtract_areas};
use crate::jsmath::{js_max, js_min};
use crate::op;
use crate::ops::areas::areas_of_entity;
use crate::text::Font;
use crate::vec2::Vec2;

/// A text's box left open is this share of its height wider all round.
pub const TEXT_MARGIN: f64 = 0.25;

/// What a text or an insert leaves open in a hatch (§4): a text's box a
/// quarter of its height wider all round, an insert's box; none for any
/// other object.
pub fn cutout(shape: &Shape, blocks: &Blocks, font: Font) -> Option<Vec<Vec2>> {
    match shape {
        Shape::Text { height, .. } => {
            TextPlace::of(shape).map(|t| t.outline_grown(font, height * TEXT_MARGIN))
        }
        Shape::Insert { .. } => blocks.outline(shape, font),
        _ => None,
    }
}

/// A region as a hatch holds it: its ring and holes, arcs as chords; which
/// of the islands and the cutouts given reach into it, by their place.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub ring: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
    pub islands: Vec<usize>,
    pub cutouts: Vec<usize>,
}

crate::json_struct!(out Region { ring, holes, islands, cutouts });

/// The place of the largest area (the first of equals).
fn largest(areas: &[Area]) -> usize {
    let mut best = 0;
    let mut size = f64::NEG_INFINITY;
    for (i, a) in areas.iter().enumerate() {
        let s = net_area(a);
        if s > size {
            size = s;
            best = i;
        }
    }
    best
}

/// Whether two areas share some of their insides.
fn overlap(a: &Area, b: &Area) -> bool {
    intersect_areas(&[a.clone(), b.clone()])
        .iter()
        .any(|x| net_area(x) > 0.0)
}

/// A base area cut by what reaches into it: the parts left, and which of
/// the islands (each an object's areas) and cutouts (boxes) reached in.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub parts: Vec<Area>,
    pub islands: Vec<usize>,
    pub cutouts: Vec<usize>,
}

crate::json_struct!(out Cut { parts, islands, cutouts });

/// `base` less the islands' areas and the cutouts' boxes that reach into it.
pub fn cut_base(base: &Area, islands: &[Vec<Area>], cutouts: &[Vec<Vec2>]) -> Cut {
    let mut cutters = Vec::new();
    let mut used_islands = Vec::new();
    for (i, areas) in islands.iter().enumerate() {
        let reach: Vec<&Area> = areas.iter().filter(|a| overlap(base, a)).collect();
        if !reach.is_empty() {
            used_islands.push(i);
            cutters.extend(reach.into_iter().cloned());
        }
    }
    let mut used_cutouts = Vec::new();
    for (i, c) in cutouts.iter().enumerate() {
        if c.len() < 3 {
            continue;
        }
        let a = Area {
            outer: Ring {
                pts: c.clone(),
                bulges: None,
            },
            holes: Vec::new(),
        };
        if overlap(base, &a) {
            used_cutouts.push(i);
            cutters.push(a);
        }
    }
    Cut {
        parts: subtract_areas(std::slice::from_ref(base), &cutters),
        islands: used_islands,
        cutouts: used_cutouts,
    }
}

/// The place of the part that holds `seed`, else of the largest; none of
/// no parts or an empty one.
pub fn pick_index(parts: &[Area], seed: Vec2) -> Option<usize> {
    if parts.is_empty() {
        return None;
    }
    let k = parts
        .iter()
        .position(|a| inside_area(a, seed))
        .unwrap_or_else(|| largest(parts));
    parts.get(k).filter(|a| net_area(a) > 0.0).map(|_| k)
}

/// The part that holds `seed`, else the largest (`pick_index`).
pub fn pick(parts: &[Area], seed: Vec2) -> Option<&Area> {
    pick_index(parts, seed).and_then(|k| parts.get(k))
}

/// A part as a hatch holds it, arcs as chords, with the islands and cutouts that reached in.
pub fn region_of(a: &Area, cut: &Cut) -> Region {
    Region {
        ring: polygon_ring(&a.outer.pts, a.outer.bulges.as_deref()),
        holes: a
            .holes
            .iter()
            .map(|h| polygon_ring(&h.pts, h.bulges.as_deref()))
            .collect(),
        islands: cut.islands.clone(),
        cutouts: cut.cutouts.clone(),
    }
}

/// A point inside an area (Çoklu tara's seed, §4): on the line across the
/// middle of its outer ring's height, the middle of the widest stretch
/// inside (even–odd with its holes); none for an empty area.
pub fn seed_of(a: &Area) -> Option<Vec2> {
    let outer = polygon_ring(&a.outer.pts, a.outer.bulges.as_deref());
    let rings: Vec<Vec<Vec2>> = std::iter::once(outer)
        .chain(
            a.holes
                .iter()
                .map(|h| polygon_ring(&h.pts, h.bulges.as_deref())),
        )
        .collect();
    let (lo, hi) = rings[0]
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (js_min(lo, p.y), js_max(hi, p.y))
        });
    if !(hi > lo) {
        return None;
    }
    // Off every vertex's height: a ring's vertex on the line would count twice or not at all.
    let mut y = (lo + hi) / 2.0;
    for k in 0..16 {
        if !rings.iter().flatten().any(|p| p.y == y) {
            break;
        }
        y = lo + (hi - lo) * (0.5 + f64::from(k + 1) * 0.0137);
    }
    let mut xs: Vec<f64> = Vec::new();
    for ring in &rings {
        for i in 0..ring.len() {
            let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
            if (p.y <= y) != (q.y <= y) {
                xs.push(p.x + (y - p.y) / (q.y - p.y) * (q.x - p.x));
            }
        }
    }
    xs.sort_by(f64::total_cmp);
    xs.chunks_exact(2)
        .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])))
        .filter(|w| w[1] > w[0])
        .map(|w| Vec2::new((w[0] + w[1]) / 2.0, y))
}

/// The region of a hatch (§5): `outer`'s area that holds `seed` (else its
/// largest), less the `islands`' areas and the `cutouts`' boxes that reach
/// into it; the part that holds the seed, else the largest. None when
/// `outer` encloses nothing or nothing is left.
pub fn region(
    outer: &Shape,
    islands: &[Shape],
    cutouts: &[Vec<Vec2>],
    seed: Vec2,
) -> Option<Region> {
    let parts = areas_of_entity(outer);
    let base = pick(&parts, seed)?;
    let islands: Vec<Vec<Area>> = islands.iter().map(areas_of_entity).collect();
    let cut = cut_base(base, &islands, cutouts);
    pick(&cut.parts, seed).map(|a| region_of(a, &cut))
}

/// `region` of objects as the documents hold them: the cutouts' boxes from
/// the texts and inserts (their blocks' definitions given), in the
/// drawing's typeface.
pub fn region_of_entities(
    outer: &Entity,
    islands: &[Entity],
    cutouts: &[Entity],
    blocks: &Blocks,
    seed: Vec2,
    font: Font,
) -> Option<Region> {
    let islands: Vec<Shape> = islands.iter().map(|e| e.shape.clone()).collect();
    let boxes: Vec<Vec<Vec2>> = cutouts
        .iter()
        .map(|e| cutout(&e.shape, blocks, font).unwrap_or_default())
        .collect();
    region(&outer.shape, &islands, &boxes, seed)
}

/// The blocks' definitions given to an operation (the contract's list), or none.
fn blocks_of(v: &Json) -> Result<Blocks, String> {
    match v {
        Json::Arr(_) => Blocks::from_json(v),
        _ => Ok(Blocks::default()),
    }
}

pub(crate) static OPS: &[Op] = &[
    // The cutouts' boxes come from `hatchCutout` (a text's or an insert's).
    op!("hatchRegion", |outer: Entity,
                        islands: Vec<Entity>,
                        boxes: Vec<Vec<Vec2>>,
                        seed: Vec2| {
        let islands: Vec<Shape> = islands.into_iter().map(|e| e.shape).collect();
        region(&outer.shape, &islands, &boxes, seed)
    }),
    // The tools' two halves of `hatchRegion`: the cut once, the part under the cursor each move.
    op!("hatchCut", |base: Area,
                     islands: Vec<Vec<Area>>,
                     cutouts: Vec<Vec<Vec2>>| {
        cut_base(&base, &islands, &cutouts)
    }),
    op!("hatchPick", |parts: Vec<Area>, seed: Vec2| pick_index(
        &parts, seed
    )),
    op!("hatchSeed", |area: Area| seed_of(&area)),
    op!(
        "hatchCutout",
        |e: Entity, blocks: Json, font: Option<String>| {
            blocks_of(&blocks).map(|b| {
                let font = font.as_deref().map_or(Font::DEFAULT, Font::from_id);
                cutout(&e.shape, &b, font)
            })
        }
    ),
];
