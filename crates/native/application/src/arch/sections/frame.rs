//! The section model's frame (docs/adr/8041, 8171 §2.1): its columns and beams as solids, and the joins that keep two
//! solids from holding the same concrete. A slab's structure holds what lies in it: a column or a beam reaching into it
//! (their tops are its top, ADR 8041 §1) stops at its underside where it covers them, as the quantities already count a
//! beam (ADR 8041 §3: the beam's concrete is its part under the slab). A column cuts a beam; where two beams cross, the
//! one drawn first holds the crossing. The 3D view, the IFC (bodies and quantities) and the drawn section all read these
//! solids, so a join is made once, here. A solid nothing else reaches into is left as it was, bit for bit.

use std::collections::HashMap;

use kentos_geometry_core::geom::arrangement::{Area, Ring, Rule};
use kentos_geometry_core::geom::overlay::overlay;
use kentos_geometry_core::geom::region::{
    area_source, intersect_area_sets, net_area, subtract_areas,
};

use super::*;

/// The model's solids for a section (docs/adr/8042 §3), its frame joined: what the 3D view, the IFC and the drawn
/// section read.
pub fn model(doc: &Document) -> Model {
    build(doc, true)
}

/// [`model`] with its columns and beams as drawn, not joined: an underlay's elements are recognised by them, another
/// program joining its frame by its own rules (docs/adr/8102 §3; a structural model's beam runs over the columns).
pub fn unjoined_model(doc: &Document) -> Model {
    build(doc, false)
}

/// A slab as the section measures it: its rings, its top, its layers' thicknesses over and under its structure, its
/// structure's thickness.
pub(super) struct SlabAt {
    pub(super) rings: Vec<Vec<P>>,
    pub(super) top: f64,
    pub(super) over: f64,
    pub(super) thickness: f64,
}

impl SlabAt {
    /// Whether the slab's rings hold the point: the core's exact even-odd test (docs/adr/9401 §9).
    pub(super) fn holds(&self, at: P) -> bool {
        kentos_geometry_core::geometry::point_in_rings(at, &self.rings)
    }

    pub(super) fn structure_top(&self) -> f64 {
        self.top - self.over
    }
}

/// A footing a column stands on (docs/adr/8172 §3.2): its band, its top and its storey.
pub(super) struct Seat {
    at: SlabAt,
    level: Option<String>,
}

/// The footings of the model, the seats of the columns over them.
pub(super) fn seats(doc: &Document, entities: &[(Uuid, &Entity)]) -> Vec<Seat> {
    use kentos_geometry_core::entity::Shape;
    entities
        .iter()
        .filter_map(|(_, e)| match e {
            Entity::Beam(b) if b.beam.role.is_footing() => Some(&*b.beam),
            _ => None,
        })
        .filter_map(|b| {
            let top = doc.beam_top(b)?;
            let Shape::Beam { pts, beam } = kentos_formats::arch_elements::core_beam(b) else {
                return None;
            };
            let ring = kentos_geometry_core::geom::beam::band(&pts, &beam);
            (ring.len() >= 3).then(|| Seat {
                at: SlabAt {
                    rings: vec![ring],
                    top,
                    over: 0.0,
                    thickness: 0.0,
                },
                level: b.arch.level.clone(),
            })
        })
        .collect()
}

/// A column between the structure under it and the structure over its storey; over a footing of its storey it stands
/// on the footing's top (docs/adr/8172 §3.2), as a frame's column goes down to its footing.
pub(super) fn column(
    c: &kentos_commands::ColumnFields,
    slabs: &[SlabAt],
    seats: &[Seat],
    owner: Option<String>,
    m: &mut Model,
) {
    use kentos_geometry_core::entity::Shape;
    let Shape::Column { at, column } = kentos_formats::arch_elements::core_column(c) else {
        return;
    };
    let Some(foot) = kentos_geometry_core::geom::column_join::Foot::of(at, &column) else {
        return;
    };
    let (floor, storey) = floor_of(&m.levels, c.arch.level.as_deref());
    // Its own height (a post's, docs/adr/8070 §8), else its storey's.
    let height = c.height.or(storey);
    let centre = P::new(
        foot.ring.iter().map(|q| q.x).sum::<f64>() / foot.ring.len() as f64,
        foot.ring.iter().map(|q| q.y).sum::<f64>() / foot.ring.len() as f64,
    );
    let bottom = slabs
        .iter()
        .filter(|s| s.holds(centre) && (s.top - floor).abs() <= 0.5)
        .map(SlabAt::structure_top)
        .fold(floor, kentos_geometry_core::jsmath::js_min);
    // Down to the highest footing under its middle, of its storey, below where it would stand.
    let bottom = seats
        .iter()
        .filter(|f| f.level == c.arch.level && f.at.top <= bottom && f.at.holds(centre))
        .map(|f| f.at.top)
        .reduce(kentos_geometry_core::jsmath::js_max)
        .unwrap_or(bottom);
    let Some(h) = height else {
        m.left.insert((Left::NoHeight, owner));
        return;
    };
    let top = slabs
        .iter()
        .filter(|s| s.holds(centre) && (s.top - (floor + h)).abs() <= 0.5)
        .map(SlabAt::structure_top)
        .fold(floor + h, kentos_geometry_core::jsmath::js_min);
    if top - bottom <= 1e-6 {
        return;
    }
    let solid = Solid {
        rings: vec![foot.ring.clone()],
        height: Height::Prism { bottom, top },
        voids: Vec::new(),
        marks: Vec::new(),
        finish: None,
    };
    // A post on EK-2's Diğer (docs/adr/8070 §8): not the frame's column.
    let element = if c.member.is_some() {
        MEMBER_ELEMENT
    } else {
        "KOLN"
    };
    push(
        m,
        solid,
        material_look(element, c.material.as_deref(), None),
        owner,
    );
}

/// A profile's look (docs/adr/8173): on its covering's EK-2 element (Harpuşta, Cephe's silme) by its material (none:
/// GENEL), cut heavy as a mass and unhatched; never the reinforced concrete's union, which a band covering a wall is
/// not part of.
fn profile_look(c: &kentos_commands::Covering, material: Option<&str>) -> Look {
    let element = c.element_of();
    let m = material
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or_default();
    Look {
        key: format!("{element}:{}", fold(m)),
        layer: kentos_commands::ek4_layer(element, m),
        heavy: true,
        fill: None,
    }
}

/// The EK-2 element a post or a rail is drawn on (docs/adr/8070 §8): Diğer (Diğer ögeler için), EK-2 having no post's or
/// rail's own.
pub const MEMBER_ELEMENT: &str = "DIGR";

/// A beam between its underside and its top (docs/adr/8041 §1); a lintel and a bond beam alike.
pub(super) fn beam(
    doc: &Document,
    b: &kentos_commands::BeamFields,
    owner: Option<String>,
    m: &mut Model,
) {
    use kentos_geometry_core::entity::Shape;
    let (Some(top), Some(bottom)) = (doc.beam_top(b), doc.beam_underside(b)) else {
        return;
    };
    let Shape::Beam { pts, beam } = kentos_formats::arch_elements::core_beam(b) else {
        return;
    };
    let ring = kentos_geometry_core::geom::beam::band(&pts, &beam);
    if ring.len() < 3 || top - bottom <= 1e-6 {
        return;
    }
    let element = match b.role {
        // A profile (docs/adr/8173) on its covering's EK-2 element (Harpuşta, Cephe), by its material.
        _ if b.covering.is_some() => b
            .covering
            .as_ref()
            .map_or(MEMBER_ELEMENT, kentos_commands::Covering::element_of),
        // A rail (docs/adr/8070 §8), by its material.
        _ if b.member.is_some() => MEMBER_ELEMENT,
        BeamRole::Beam => "TASI",
        BeamRole::Lintel => "LENT",
        BeamRole::BondBeam => "HATL",
        // A footing on EK-2's Temel (docs/adr/8172).
        BeamRole::PadFooting | BeamRole::StripFooting => "TEML",
    };
    let made_of = (b.member.is_some() || b.covering.is_some())
        .then_some(b.material.as_deref())
        .flatten();
    // A sloped member (docs/adr/8130): its start where a level one's is (its underside over its floor, else its top
    // where it hangs), rising evenly to its end; its depth across its axis a level one's, so upright it is deeper.
    let height = match (b.rise, b.pts.as_slice()) {
        (Some(rise), [a, e]) => {
            let length = kentos_geometry_core::jsmath::js_hypot(e.x - a.x, e.y - a.y);
            let g = rise / length;
            let (dx, dy) = (g * (e.x - a.x) / length, g * (e.y - a.y) / length);
            let upright = (top - bottom) * (1.0 + g * g).sqrt();
            let start_top = if b.base_offset.is_some() {
                bottom + upright
            } else {
                top
            };
            Height::Sloped(sec::Sloped {
                top: start_top - dx * a.x - dy * a.y,
                dx,
                dy,
                thickness: upright,
                eaves: ring.clone(),
                roof: RAMP_FACES + m.solids.len(),
            })
        }
        _ => Height::Prism { bottom, top },
    };
    let solid = Solid {
        rings: vec![ring],
        height,
        voids: Vec::new(),
        marks: Vec::new(),
        finish: None,
    };
    let look = match &b.covering {
        Some(c) => profile_look(c, b.material.as_deref()),
        None => material_look(element, made_of, None),
    };
    push(m, solid, look, owner);
}

/// What a solid is to the joins: a column, or a beam with the order its object was drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Column,
    Beam(usize),
}

/// How deep two solids must overlap, upright, to be joined (metres): solids that only touch (a column standing on
/// the one under it, a beam under a slab's screed) are not.
const REACH: f64 = 1e-6;

/// How much area a cut must take away to be made (square metres): a cutter touching a solid's side leaves it as it is.
const TAKEN: f64 = 1e-9;

/// A piece of a joined solid: its footprint and its heights.
type Piece = (Vec<Area>, f64, f64);

/// The joins (docs/adr/8171 §2.1) of the model's upright columns and beams: each stops under the slab structures that
/// cover it (a part of it beyond a slab's edge keeps its height; a column through a slab goes on over it), a beam is cut
/// by the columns it meets and by the beams drawn before it that it crosses. A sloped solid (a raking beam, a sloped
/// slab) and an object's part (a chimney, a curtain wall's frame) are not joined.
pub(super) fn joins(doc: &Document, m: &mut Model) {
    let mut kinds: HashMap<String, Role> = HashMap::new();
    for (order, (uid, e)) in doc.entities_with_uids().enumerate() {
        let role = match e {
            Entity::Column(_) => Role::Column,
            // A profile (docs/adr/8173) covers a wall: no structure, left as it is.
            Entity::Beam(b) if b.beam.covering.is_none() => Role::Beam(order),
            _ => continue,
        };
        kinds.insert(uid.to_string(), role);
    }
    let upright = |m: &Model, i: usize| match m.solids[i].height {
        Height::Prism { bottom, top } if m.parts[i].is_none() && !m.solids[i].rings.is_empty() => {
            Some((bottom, top))
        }
        _ => None,
    };
    let slabs: Vec<(Vec<Area>, [f64; 4], f64, f64)> = m
        .structure
        .iter()
        .filter_map(|&s| {
            let (bottom, top) = upright(m, s)?;
            Some((
                areas(&exact_rings(m, s)),
                bounds(&m.solids[s].rings),
                bottom,
                top,
            ))
        })
        .collect();
    let frame: Vec<(usize, Role, f64, f64)> = (0..m.solids.len())
        .filter_map(|i| {
            let (bottom, top) = upright(m, i)?;
            Some((i, *kinds.get(m.owners[i].as_ref()?)?, bottom, top))
        })
        .collect();
    if frame.is_empty() {
        return;
    }
    // Each one's pieces: first under the slabs, then cut by the columns and the earlier beams.
    let mut pieces: Vec<Vec<Piece>> = frame
        .iter()
        .map(|&(i, _, bottom, top)| vec![(areas(&exact_rings(m, i)), bottom, top)])
        .collect();
    let mut changed = vec![false; frame.len()];
    for (k, list) in pieces.iter_mut().enumerate() {
        for (slab, sbox, sb, st) in &slabs {
            let mut next = Vec::new();
            for (fp, b, t) in list.drain(..) {
                let covered = if st.min(t) - sb.max(b) > REACH && meet(boxes(&fp), *sbox) {
                    intersect_area_sets(&[fp.clone(), slab.clone()])
                } else {
                    Vec::new()
                };
                if total(&covered) <= TAKEN {
                    next.push((fp, b, t));
                    continue;
                }
                changed[k] = true;
                if sb - b > REACH {
                    next.push((covered.clone(), b, *sb));
                }
                if t - st > REACH {
                    next.push((covered, *st, t));
                }
                let free = subtract_areas(&fp, slab);
                if total(&free) > TAKEN {
                    next.push((free, b, t));
                }
            }
            *list = next;
        }
    }
    let cutters: Vec<(Role, Vec<Piece>)> = frame
        .iter()
        .map(|&(_, r, _, _)| r)
        .zip(pieces.iter().cloned())
        .collect();
    for (k, &(_, role, _, _)) in frame.iter().enumerate() {
        let Role::Beam(mine) = role else { continue };
        for b in 0..pieces[k].len() {
            let (fp, bottom, top) = pieces[k][b].clone();
            let by: Vec<Area> = cutters
                .iter()
                .enumerate()
                .filter(|(j, (other, _))| {
                    *j != k
                        && match other {
                            Role::Column => true,
                            Role::Beam(theirs) => *theirs < mine,
                        }
                })
                .flat_map(|(_, (_, list))| list.iter())
                .filter(|(c, cb, ct)| {
                    top.min(*ct) - bottom.max(*cb) > REACH && meet(boxes(&fp), boxes(c))
                })
                .flat_map(|(c, _, _)| c.iter().cloned())
                .collect();
            if by.is_empty() {
                continue;
            }
            let left = subtract_areas(&fp, &by);
            if total(&fp) - total(&left) <= TAKEN {
                continue;
            }
            changed[k] = true;
            pieces[k][b] = (left, bottom, top);
        }
    }
    for (k, &(i, _, _, _)) in frame.iter().enumerate() {
        if !changed[k] {
            continue;
        }
        // A beam wholly inside columns keeps its body rather than none.
        let mut list = pieces[k].iter().filter(|(fp, _, _)| total(fp) > TAKEN);
        let Some(first) = list.next() else { continue };
        let more: Vec<Piece> = list.cloned().collect();
        set(m, i, first);
        for piece in &more {
            m.solids.push(m.solids[i].clone());
            m.looks.push(m.looks[i].clone());
            m.owners.push(m.owners[i].clone());
            m.parts.push(m.parts[i]);
            m.exact.push(None);
            set(m, m.solids.len() - 1, piece);
        }
    }
}

/// Solid `i` made the piece: its footprint (flattened, its arcs kept where it has any) and its heights.
fn set(m: &mut Model, i: usize, (fp, bottom, top): &Piece) {
    let exact: Vec<ExactRing> = fp
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
        .map(|r| {
            let n = r.pts.len();
            (
                r.pts.clone(),
                r.bulges.clone().unwrap_or_else(|| vec![0.0; n]),
            )
        })
        .collect();
    m.solids[i].rings = exact
        .iter()
        .map(|(pts, b)| kentos_geometry_core::entity::polygon_ring(pts, Some(b)))
        .collect();
    m.solids[i].height = Height::Prism {
        bottom: *bottom,
        top: *top,
    };
    let arcs = exact.iter().any(|(_, b)| b.iter().any(|x| *x != 0.0));
    m.exact[i] = arcs.then_some(exact);
}

/// A solid's footprint with its arcs: its own exact rings, else its rings with straight edges.
fn exact_rings(m: &Model, i: usize) -> Vec<ExactRing> {
    match &m.exact[i] {
        Some(e) if !e.is_empty() => e.clone(),
        _ => m.solids[i]
            .rings
            .iter()
            .map(|r| (r.clone(), vec![0.0; r.len()]))
            .collect(),
    }
}

/// Rings read by the even-odd rule (a solid's footprint) as areas: outer rings with their holes.
fn areas(rings: &[ExactRing]) -> Vec<Area> {
    let ring = |(pts, b): &ExactRing| Ring {
        pts: pts.clone(),
        bulges: b.iter().any(|x| *x != 0.0).then(|| b.clone()),
    };
    let kept: Vec<&ExactRing> = rings.iter().filter(|(pts, _)| pts.len() >= 3).collect();
    match kept.as_slice() {
        [] => Vec::new(),
        [one] => vec![Area {
            outer: ring(one),
            holes: Vec::new(),
        }],
        _ => {
            let sources: Vec<_> = kept
                .iter()
                .map(|r| {
                    area_source(&[Area {
                        outer: ring(r),
                        holes: Vec::new(),
                    }])
                })
                .collect();
            overlay(&sources, Rule::Odd)
        }
    }
}

/// The areas' net area, square metres.
fn total(list: &[Area]) -> f64 {
    list.iter().map(net_area).sum()
}

/// Areas' box: west, south, east, north (their arcs' chords: a box to meet by, not a measure).
fn boxes(list: &[Area]) -> [f64; 4] {
    list.iter()
        .flat_map(|a| a.outer.pts.iter())
        .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, q| {
            [b[0].min(q.x), b[1].min(q.y), b[2].max(q.x), b[3].max(q.y)]
        })
}

/// A footprint's box: west, south, east, north.
fn bounds(rings: &[Vec<P>]) -> [f64; 4] {
    rings
        .iter()
        .flatten()
        .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, q| {
            [b[0].min(q.x), b[1].min(q.y), b[2].max(q.x), b[3].max(q.y)]
        })
}

/// Whether two boxes overlap (touching is not).
fn meet(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}
