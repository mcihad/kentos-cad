//! The locator (docs/adr/0229 §6): an index of a project's doors, streets
//! and areas, built from what a host reads of its drawing (each object in
//! the drawing's order, its attributes' texts as the address data names
//! them). Doors are grouped into streets by (street, neighbourhood,
//! district); a street drawn with no door of its own is a group of its
//! lines. Names are found by their similarity (`Names`), places by packed
//! R-trees; a street's and a neighbourhood's place is where §6 puts it.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::fold::{Tag, fold, tokens};
use super::parse::{area_value, door_number, door_value, street_value, words_key};
use super::similar::Names;
use crate::entity::Shape;
use crate::geom::bulge::bulge_ring_area;
use crate::geom::intersect::closest_on_edge;
use crate::geometry::{Bounds, dist};
use crate::jsmath::{js_hypot, js_max};
use crate::ops::generate::pole::pole;
use crate::ops::spatial_query::{Geometry, geometry_of, in_or_on};
use crate::store::rtree::{Nearer, NearestQueue, PackedTree};
use crate::tools::point_calc::Walk;
use crate::vec2::Vec2;

/// An area layer's level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    Neighbourhood,
    District,
    Province,
}

/// A door as the host reads it: its place and its attributes' texts (empty when the field is not named or not set).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DoorInput {
    /// Its place in the drawing's order.
    pub order: u32,
    pub at: Vec2,
    pub street: String,
    pub door: String,
    pub neighbourhood: String,
    pub district: String,
    pub province: String,
    pub postcode: String,
    pub building: String,
    pub code: String,
}

/// A street's line as the host reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct StreetInput {
    pub order: u32,
    pub shape: Shape,
    pub name: String,
    /// Empty: the neighbourhood area its middle is in says it.
    pub neighbourhood: String,
    pub district: String,
}

/// An area as the host reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaInput {
    pub order: u32,
    pub level: Level,
    pub shape: Shape,
    pub name: String,
    /// A neighbourhood's district.
    pub district: String,
}

/// What a host gives a locator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Input {
    pub doors: Vec<DoorInput>,
    pub streets: Vec<StreetInput>,
    pub areas: Vec<AreaInput>,
    /// The project's unit per metre (1 in metres, 1000 in a millimetre project).
    pub per_metre: f64,
    /// Objects of the door layer that are no points, of the street layer no lines, of an area layer no areas.
    pub not_doors: usize,
    pub not_streets: usize,
    pub not_areas: usize,
}

/// The texts an address shows, as the data writes them (trimmed).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Texts {
    pub street: String,
    pub door: String,
    pub neighbourhood: String,
    pub district: String,
    pub province: String,
    pub postcode: String,
    pub building: String,
    pub code: String,
}

/// A door read.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Door {
    pub order: u32,
    pub at: Vec2,
    pub key: String,
    pub number: Option<u64>,
    pub kind: Option<Tag>,
    pub postcode: String,
    pub code: String,
    pub texts: Texts,
    pub group: usize,
}

/// A street's line read.
pub(super) struct Line {
    pub order: u32,
    pub geometry: Geometry,
    pub length: f64,
    pub middle: Vec2,
    pub name: String,
    pub kind: Option<Tag>,
    pub neighbourhood: String,
    pub district: String,
    pub texts: Texts,
}

/// An area read.
pub(super) struct Area {
    pub order: u32,
    pub level: Level,
    pub shape: Shape,
    pub geometry: Geometry,
    pub size: f64,
    pub name: String,
    pub district: String,
    pub name_text: String,
    pub district_text: String,
    /// Its pole, found when first asked.
    pub pole: OnceLock<Option<Vec2>>,
}

/// A street: its doors, or the lines of one with no door.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Group {
    pub name: String,
    pub kind: Option<Tag>,
    pub neighbourhood: String,
    pub district: String,
    /// Its doors in the drawing's order.
    pub doors: Vec<usize>,
    /// A street with no door: its lines in the drawing's order.
    pub lines: Vec<usize>,
    pub order: u32,
    pub postcodes: Vec<String>,
}

/// A neighbourhood's name and where its place comes from.
pub(super) struct Hood {
    pub key: String,
    pub text: String,
    pub district_text: String,
    pub province_text: String,
    pub order: u32,
}

/// A project's address data, indexed (§6).
pub struct Locator {
    pub(super) per_metre: f64,
    pub(super) doors: Vec<Door>,
    pub(super) lines: Vec<Line>,
    pub(super) areas: Vec<Area>,
    pub(super) groups: Vec<Group>,
    /// Street names and, for each, its groups in their order.
    pub(super) names: Names,
    pub(super) name_groups: Vec<Vec<usize>>,
    /// Neighbourhood names and what each is.
    pub(super) hoods: Names,
    pub(super) hood_info: Vec<Hood>,
    /// Districts' keys, as their words.
    pub(super) districts: Vec<Vec<String>>,
    pub(super) codes: HashMap<String, Vec<usize>>,
    pub(super) door_tree: PackedTree,
    pub(super) line_tree: PackedTree,
    /// What was left out: no street name, no door number.
    pub empty_streets: usize,
    pub unread_doors: usize,
    pub not_doors: usize,
    pub not_streets: usize,
    pub not_areas: usize,
    pub unnamed_lines: usize,
    pub unnamed_areas: usize,
}

fn trim(s: &str) -> String {
    s.trim().to_owned()
}

fn point_bounds(p: Vec2) -> Bounds {
    Bounds {
        min_x: p.x,
        min_y: p.y,
        max_x: p.x,
        max_y: p.y,
    }
}

/// The distance from `p` to a box (0 inside it).
pub(super) fn box_gap(b: &Bounds, p: Vec2) -> f64 {
    let dx = js_max(js_max(b.min_x - p.x, 0.0), p.x - b.max_x);
    let dy = js_max(js_max(b.min_y - p.y, 0.0), p.y - b.max_y);
    js_hypot(dx, dy)
}

/// The distance from `p` to a line's edges and the nearest point (the first of equal ones in the edges' order).
pub(super) fn line_gap(g: &Geometry, p: Vec2) -> (f64, Vec2) {
    let mut best = (f64::INFINITY, p);
    for e in &g.edges {
        let c = closest_on_edge(e, p);
        if c.d < best.0 {
            best = (c.d, c.p);
        }
    }
    best
}

impl Locator {
    /// The locator of what a host read (§6).
    pub fn build(input: &Input) -> Locator {
        let per_metre = if input.per_metre.is_finite() && input.per_metre > 0.0 {
            input.per_metre
        } else {
            1.0
        };
        let mut loc = Locator {
            per_metre,
            doors: Vec::new(),
            lines: Vec::new(),
            areas: Vec::new(),
            groups: Vec::new(),
            names: Names::default(),
            name_groups: Vec::new(),
            hoods: Names::default(),
            hood_info: Vec::new(),
            districts: Vec::new(),
            codes: HashMap::new(),
            door_tree: PackedTree::empty(),
            line_tree: PackedTree::empty(),
            empty_streets: 0,
            unread_doors: 0,
            not_doors: input.not_doors,
            not_streets: input.not_streets,
            not_areas: input.not_areas,
            unnamed_lines: 0,
            unnamed_areas: 0,
        };
        // Areas first: a line's neighbourhood may come from one.
        let mut areas: Vec<&AreaInput> = input.areas.iter().collect();
        areas.sort_by_key(|a| a.order);
        for a in areas {
            let Some(geometry) = geometry_of(&a.shape).filter(|g| !g.areas.is_empty()) else {
                loc.not_areas += 1;
                continue;
            };
            let name = match a.level {
                Level::Neighbourhood => area_value(&a.name),
                _ => words_key(&a.name),
            };
            if name.is_empty() {
                loc.unnamed_areas += 1;
                continue;
            }
            let size: f64 = geometry
                .areas
                .iter()
                .map(|ar| {
                    bulge_ring_area(&ar.outer.pts, ar.outer.bulges.as_deref()).abs()
                        - ar.holes
                            .iter()
                            .map(|h| bulge_ring_area(&h.pts, h.bulges.as_deref()).abs())
                            .sum::<f64>()
                })
                .sum();
            loc.areas.push(Area {
                order: a.order,
                level: a.level,
                shape: a.shape.clone(),
                geometry,
                size,
                name,
                district: words_key(&a.district),
                name_text: trim(&a.name),
                district_text: trim(&a.district),
                pole: OnceLock::new(),
            });
        }
        // Doors.
        let mut doors: Vec<&DoorInput> = input.doors.iter().collect();
        doors.sort_by_key(|d| d.order);
        let mut keys: HashMap<(String, String, String), usize> = HashMap::new();
        for d in doors {
            let Some((street, kind)) = street_value(&d.street) else {
                loc.empty_streets += 1;
                continue;
            };
            let Some(key) = door_value(&d.door) else {
                loc.unread_doors += 1;
                continue;
            };
            let neighbourhood = area_value(&d.neighbourhood);
            let district = words_key(&d.district);
            let at = loc.doors.len();
            let group = *keys
                .entry((street.clone(), neighbourhood.clone(), district.clone()))
                .or_insert_with(|| {
                    loc.groups.push(Group {
                        name: street.clone(),
                        kind,
                        neighbourhood: neighbourhood.clone(),
                        district: district.clone(),
                        doors: Vec::new(),
                        lines: Vec::new(),
                        order: d.order,
                        postcodes: Vec::new(),
                    });
                    loc.groups.len() - 1
                });
            let postcode = trim(&d.postcode);
            let g = &mut loc.groups[group];
            g.doors.push(at);
            if !postcode.is_empty() && !g.postcodes.contains(&postcode) {
                g.postcodes.push(postcode.clone());
            }
            let code = trim(&d.code);
            if !code.is_empty() {
                loc.codes.entry(code.clone()).or_default().push(at);
            }
            loc.doors.push(Door {
                order: d.order,
                at: d.at,
                number: door_number(&key),
                key,
                kind,
                postcode,
                code,
                texts: Texts {
                    street: trim(&d.street),
                    door: trim(&d.door),
                    neighbourhood: trim(&d.neighbourhood),
                    district: trim(&d.district),
                    province: trim(&d.province),
                    postcode: trim(&d.postcode),
                    building: trim(&d.building),
                    code: trim(&d.code),
                },
                group,
            });
        }
        // Streets' lines.
        let mut streets: Vec<&StreetInput> = input.streets.iter().collect();
        streets.sort_by_key(|s| s.order);
        for s in streets {
            let (Some(walk), Some(geometry)) = (Walk::new(&s.shape, false), geometry_of(&s.shape))
            else {
                loc.not_streets += 1;
                continue;
            };
            let Some((name, kind)) = street_value(&s.name) else {
                loc.unnamed_lines += 1;
                continue;
            };
            let length = walk.length();
            let middle = walk.frame(length / 2.0).0;
            let own = area_value(&s.neighbourhood);
            let (neighbourhood, hood_text) = if own.is_empty() {
                loc.areas
                    .iter()
                    .find(|a| a.level == Level::Neighbourhood && in_or_on(&a.geometry, middle))
                    .map_or((String::new(), String::new()), |a| {
                        (a.name.clone(), a.name_text.clone())
                    })
            } else {
                (own, trim(&s.neighbourhood))
            };
            loc.lines.push(Line {
                order: s.order,
                geometry,
                length,
                middle,
                name,
                kind,
                neighbourhood,
                district: words_key(&s.district),
                texts: Texts {
                    street: trim(&s.name),
                    neighbourhood: hood_text,
                    district: trim(&s.district),
                    ..Texts::default()
                },
            });
        }
        // Streets drawn with no door of their own.
        let compatible = |a: &str, b: &str| a == b || a.is_empty() || b.is_empty();
        let mut door_streets: HashMap<String, Vec<usize>> = HashMap::new();
        for (g, group) in loc.groups.iter().enumerate() {
            door_streets.entry(group.name.clone()).or_default().push(g);
        }
        let mut line_keys: HashMap<(String, String, String), usize> = HashMap::new();
        for (i, l) in loc.lines.iter().enumerate() {
            let with_doors = door_streets.get(&l.name).is_some_and(|gs| {
                gs.iter().any(|&g| {
                    let g = &loc.groups[g];
                    compatible(&g.neighbourhood, &l.neighbourhood)
                        && compatible(&g.district, &l.district)
                })
            });
            if with_doors {
                continue;
            }
            let key = (l.name.clone(), l.neighbourhood.clone(), l.district.clone());
            match line_keys.get(&key) {
                Some(&g) => loc.groups[g].lines.push(i),
                None => {
                    loc.groups.push(Group {
                        name: l.name.clone(),
                        kind: l.kind,
                        neighbourhood: l.neighbourhood.clone(),
                        district: l.district.clone(),
                        doors: Vec::new(),
                        lines: vec![i],
                        order: l.order,
                        postcodes: Vec::new(),
                    });
                    line_keys.insert(key, loc.groups.len() - 1);
                }
            }
        }
        // Groups in the order of their first objects; doors told their group's new place.
        let mut by_order: Vec<usize> = (0..loc.groups.len()).collect();
        by_order.sort_by_key(|&g| loc.groups[g].order);
        let mut place = vec![0; loc.groups.len()];
        for (new, &old) in by_order.iter().enumerate() {
            place[old] = new;
        }
        let mut old: Vec<Option<Group>> = std::mem::take(&mut loc.groups)
            .into_iter()
            .map(Some)
            .collect();
        loc.groups = by_order.iter().filter_map(|&g| old[g].take()).collect();
        for d in &mut loc.doors {
            d.group = place[d.group];
        }
        for (g, group) in loc.groups.iter().enumerate() {
            let at = loc.names.add(&group.name);
            if at == loc.name_groups.len() {
                loc.name_groups.push(Vec::new());
            }
            loc.name_groups[at].push(g);
        }
        // Neighbourhoods: from their areas, then the doors', then the lines'.
        let hood = |loc: &mut Locator,
                    key: &str,
                    text: &str,
                    district: &str,
                    province: &str,
                    order: u32| {
            if key.is_empty() || loc.hoods.exact(key).is_some() {
                return;
            }
            loc.hoods.add(key);
            loc.hood_info.push(Hood {
                key: key.to_owned(),
                text: text.to_owned(),
                district_text: district.to_owned(),
                province_text: province.to_owned(),
                order,
            });
        };
        let area_hoods: Vec<(String, String, String, u32)> = loc
            .areas
            .iter()
            .filter(|a| a.level == Level::Neighbourhood)
            .map(|a| {
                (
                    a.name.clone(),
                    a.name_text.clone(),
                    a.district_text.clone(),
                    a.order,
                )
            })
            .collect();
        for (k, t, d, o) in area_hoods {
            hood(&mut loc, &k, &t, &d, "", o);
        }
        let door_hoods: Vec<(String, String, String, String, u32)> = loc
            .doors
            .iter()
            .map(|d| {
                (
                    loc.groups[d.group].neighbourhood.clone(),
                    d.texts.neighbourhood.clone(),
                    d.texts.district.clone(),
                    d.texts.province.clone(),
                    d.order,
                )
            })
            .collect();
        for (k, t, d, p, o) in door_hoods {
            hood(&mut loc, &k, &t, &d, &p, o);
        }
        let line_hoods: Vec<(String, String, String, u32)> = loc
            .lines
            .iter()
            .map(|l| {
                (
                    l.neighbourhood.clone(),
                    l.texts.neighbourhood.clone(),
                    l.texts.district.clone(),
                    l.order,
                )
            })
            .collect();
        for (k, t, d, o) in line_hoods {
            hood(&mut loc, &k, &t, &d, "", o);
        }
        // Districts.
        let mut districts: Vec<String> = Vec::new();
        let mut add = |k: &str| {
            if !k.is_empty() && !districts.iter().any(|d| d == k) {
                districts.push(k.to_owned());
            }
        };
        for g in &loc.groups {
            add(&g.district);
        }
        for a in &loc.areas {
            match a.level {
                Level::District => add(&a.name),
                Level::Neighbourhood => add(&a.district),
                Level::Province => {}
            }
        }
        loc.districts = districts
            .iter()
            .map(|d| d.split(' ').map(str::to_owned).collect())
            .collect();
        // Places.
        let entries: Vec<(u32, Bounds)> = loc
            .doors
            .iter()
            .enumerate()
            .map(|(i, d)| (i as u32, point_bounds(d.at)))
            .collect();
        loc.door_tree = PackedTree::build(&entries);
        let entries: Vec<(u32, Bounds)> = loc
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| (i as u32, l.geometry.bounds))
            .collect();
        loc.line_tree = PackedTree::build(&entries);
        loc
    }

    /// The project's unit per metre.
    pub fn per_metre(&self) -> f64 {
        self.per_metre
    }

    /// How many doors, streets (groups), lines, neighbourhoods and areas it holds.
    pub fn counts(&self) -> Counts {
        Counts {
            doors: self.doors.len(),
            streets: self.groups.len(),
            lines: self.lines.len(),
            neighbourhoods: self.hoods.len(),
            areas: self.areas.len(),
        }
    }

    /// A group's street-level place (§6): its middle door, or its longest line's middle.
    pub(super) fn street_place(&self, g: usize) -> Vec2 {
        let group = &self.groups[g];
        if !group.doors.is_empty() {
            let mut doors: Vec<&Door> = group.doors.iter().map(|&i| &self.doors[i]).collect();
            doors.sort_by(|a, b| {
                a.number
                    .unwrap_or(u64::MAX)
                    .cmp(&b.number.unwrap_or(u64::MAX))
                    .then_with(|| a.key.cmp(&b.key))
                    .then_with(|| a.order.cmp(&b.order))
            });
            return doors[(doors.len() - 1) / 2].at;
        }
        let mut best: Option<&Line> = None;
        for &i in &group.lines {
            let l = &self.lines[i];
            if best.is_none_or(|b| l.length > b.length) {
                best = Some(l);
            }
        }
        best.map_or(Vec2::new(0.0, 0.0), |l| l.middle)
    }

    /// A neighbourhood's place (§6): its largest area's pole, else its doors' mean, else its first street's place.
    pub(super) fn hood_place(&self, key: &str) -> Option<Vec2> {
        let mut best: Option<&Area> = None;
        for a in &self.areas {
            if a.level == Level::Neighbourhood
                && a.name == key
                && best.is_none_or(|b| a.size > b.size)
            {
                best = Some(a);
            }
        }
        if let Some(a) = best {
            let at = a
                .pole
                .get_or_init(|| pole(&a.shape, 0.1 * self.per_metre).map(|(p, _)| p));
            if let Some(p) = at {
                return Some(*p);
            }
        }
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0usize);
        for d in &self.doors {
            if self.groups[d.group].neighbourhood == key {
                sx += d.at.x;
                sy += d.at.y;
                n += 1;
            }
        }
        if n > 0 {
            return Some(Vec2::new(sx / n as f64, sy / n as f64));
        }
        (0..self.groups.len())
            .find(|&g| self.groups[g].doors.is_empty() && self.groups[g].neighbourhood == key)
            .map(|g| self.street_place(g))
    }

    /// The nearest door within `reach` (project units), equal ones by the drawing's order.
    pub(super) fn nearest_door(&self, p: Vec2, reach: f64) -> Option<(usize, f64)> {
        struct Search<'a> {
            loc: &'a Locator,
            p: Vec2,
            found: Option<(usize, f64)>,
        }
        impl Nearer for Search<'_> {
            fn gap(&self, b: &Bounds) -> f64 {
                box_gap(b, self.p)
            }
            fn measure(&mut self, item: u32) -> Option<(f64, u32, u32)> {
                let d = &self.loc.doors[item as usize];
                let gap = dist(d.at, self.p);
                Some((gap, d.order, item))
            }
            fn take(&mut self, token: u32) -> bool {
                let d = &self.loc.doors[token as usize];
                self.found = Some((token as usize, dist(d.at, self.p)));
                false
            }
        }
        let mut s = Search {
            loc: self,
            p,
            found: None,
        };
        let mut queue = NearestQueue::default();
        self.door_tree.nearest(&mut queue, reach, &mut s);
        s.found
    }

    /// The nearest line within `reach` and its nearest point, equal ones by the drawing's order.
    pub(super) fn nearest_line(&self, p: Vec2, reach: f64) -> Option<(usize, f64, Vec2)> {
        struct Search<'a> {
            loc: &'a Locator,
            p: Vec2,
            found: Option<(usize, f64, Vec2)>,
        }
        impl Nearer for Search<'_> {
            fn gap(&self, b: &Bounds) -> f64 {
                box_gap(b, self.p)
            }
            fn measure(&mut self, item: u32) -> Option<(f64, u32, u32)> {
                let l = &self.loc.lines[item as usize];
                let (d, _) = line_gap(&l.geometry, self.p);
                d.is_finite().then_some((d, l.order, item))
            }
            fn take(&mut self, token: u32) -> bool {
                let l = &self.loc.lines[token as usize];
                let (d, q) = line_gap(&l.geometry, self.p);
                self.found = Some((token as usize, d, q));
                false
            }
        }
        let mut s = Search {
            loc: self,
            p,
            found: None,
        };
        let mut queue = NearestQueue::default();
        self.line_tree.nearest(&mut queue, reach, &mut s);
        s.found
    }

    /// The first area of a level, in the drawing's order, that holds `p` (its edge too).
    pub(super) fn area_at(&self, level: Level, p: Vec2) -> Option<&Area> {
        self.areas.iter().find(|a| {
            a.level == level && box_gap(&a.geometry.bounds, p) == 0.0 && in_or_on(&a.geometry, p)
        })
    }

    /// Whether a neighbourhood text already says it is one (it has a MAHALLE or KÖY tag).
    pub fn tagged_hood(text: &str) -> bool {
        tokens(&fold(text))
            .iter()
            .filter_map(|t| t.word())
            .any(|w| super::fold::tag_of(w).is_some_and(Tag::is_area))
    }
}

/// What a locator holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub doors: usize,
    pub streets: usize,
    pub lines: usize,
    pub neighbourhoods: usize,
    pub areas: usize,
}
