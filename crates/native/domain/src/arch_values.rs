//! What an object's tag and schedule read of it (docs/adr/0344 §1, §3): one value rule for both.
//!
//! - An opening: its number, type (its style), form, width, height (head − sill, with a head), sill (absent 0), head,
//!   area (width × height), and its wall's style, level and phase.
//! - A space: its number, name, usage, area and perimeter (its rings', holes off), height (its own, else its level's),
//!   volume (area × height).
//! - A column: its tag, place, type and width. A grid: its label. A stair: its risers. A ramp: its slope (given, else
//!   its rise over its sloped length). An insert: its block's name as its type.
//! - A wall: its style (as type and style), its width (as thickness and width), its base line's length, height (its
//!   own, else its level's), net face area (length × height less its openings' areas) and volume (net face × width).
//! - Each: its level's name and phase, its attribute by name.
//!
//! Written: a length in the project's unit and length decimals; an area in its area unit and decimals, a volume in m³
//! with the area decimals (a tag writes their unit after them, a schedule its heading); a count whole; a slope `%`
//! and its value to two decimals, its trailing zeros dropped. The point a tag ties to: an opening's middle, a space's
//! label place, a column's centre, an insert's point, another's middle edge's middle (edge ⌊(n − 1)/2⌋ of its path,
//! an arc by its arc). `scripts/fixtures/tag_command_cases.py` writes the same without KentOS's code.

use kentos_contracts::{ArchInfo, AreaUnit, Entity, OpeningForm, SpacePart, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::bulge::{bulge_at, bulge_path_length, segment_mid};

use crate::document::Document;
use crate::text_fields::number_text;

/// A value an object gives a tag or a schedule.
#[derive(Clone, Debug, PartialEq)]
pub enum ArchValue {
    Text(String),
    Length(f64),
    Area(f64),
    Volume(f64),
    Count(u64),
    Percent(f64),
}

impl ArchValue {
    /// The number it adds to a sum; none for words.
    pub fn number(&self) -> Option<f64> {
        match self {
            ArchValue::Text(_) => None,
            ArchValue::Length(v)
            | ArchValue::Area(v)
            | ArchValue::Volume(v)
            | ArchValue::Percent(v) => Some(*v),
            ArchValue::Count(n) => Some(*n as f64),
        }
    }
}

fn core(p: &[Vec2]) -> Vec<CoreVec2> {
    p.iter().map(|q| CoreVec2::new(q.x, q.y)).collect()
}

/// The core's areas of a space's parts.
pub fn core_areas(parts: &[SpacePart]) -> Vec<Area> {
    let ring = |r: &kentos_contracts::SpaceRing| Ring {
        pts: core(&r.pts),
        bulges: r.bulges.clone(),
    };
    parts
        .iter()
        .map(|p| Area {
            outer: ring(&p.outer),
            holes: p.holes.iter().map(ring).collect(),
        })
        .collect()
}

/// A path's middle edge's middle: edge ⌊(n − 1)/2⌋, an arc by its arc.
fn path_middle(pts: &[Vec2], bulges: Option<&[f64]>) -> Option<Vec2> {
    if pts.len() < 2 {
        return pts.first().copied();
    }
    let i = (pts.len() - 1) / 2;
    let p = segment_mid(
        CoreVec2::new(pts[i].x, pts[i].y),
        CoreVec2::new(pts[i + 1].x, pts[i + 1].y),
        bulge_at(bulges, i),
    );
    Some(Vec2 { x: p.x, y: p.y })
}

/// Slope's words: `%` and its value to two decimals, the trailing zeros dropped.
pub fn percent_text(v: f64) -> String {
    let t = fixed(v, 2);
    let t = if t.contains('.') {
        t.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        t
    };
    format!("%{t}")
}

impl Document {
    fn level_name(&self, arch: &ArchInfo) -> Option<String> {
        let id = arch.level.as_deref()?;
        Some(
            self.settings()
                .arch
                .as_ref()
                .and_then(|a| a.levels.iter().find(|l| l.id == id))
                .map_or_else(|| id.to_owned(), |l| l.name.clone()),
        )
    }

    fn level_height(&self, arch: &ArchInfo) -> Option<f64> {
        let id = arch.level.as_deref()?;
        self.settings()
            .arch
            .as_ref()?
            .levels
            .iter()
            .find(|l| l.id == id)?
            .height
    }

    /// The room's assigned floor; absence is unknown, not project zero.
    fn room_floor_elevation(&self, arch: &ArchInfo) -> Option<f64> {
        let id = arch.level.as_deref()?;
        self.settings()
            .arch
            .as_ref()?
            .levels
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.elevation)
    }

    /// An opening type's name: the project's type of that id (docs/adr/0772), else KentOS's type of that id or name,
    /// else the words themselves.
    pub fn opening_type_name(&self, style: Option<&str>) -> String {
        let s = style.unwrap_or_default();
        if let Some(t) = self
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.opening_types.iter().find(|t| t.id == s))
        {
            return t.name.clone();
        }
        kentos_contracts::opening_type(s).map_or_else(|| s.to_owned(), |t| t.name)
    }

    /// A wall style's name: the project's style of that id, else KentOS's, else the id itself.
    pub fn wall_style_name(&self, style: Option<&str>) -> String {
        let s = style.unwrap_or_default();
        let own = self
            .settings()
            .wall_styles
            .iter()
            .find(|w| w.id == s)
            .map(|w| w.name.clone());
        own.or_else(|| {
            kentos_contracts::builtin_wall_styles()
                .into_iter()
                .find(|w| w.id == s)
                .map(|w| w.name)
        })
        .unwrap_or_else(|| s.to_owned())
    }

    fn wall_of(&self, host: &str) -> Option<kentos_contracts::WallFields> {
        let uid = crate::identity::Uuid::parse_str(host).ok()?;
        let slot = self.store.slot_of(uid)?;
        match &*self.store.get(slot)?.entity {
            Entity::Wall(w) => Some(w.wall.clone()),
            _ => None,
        }
    }

    /// A wall's base line length and net face area (length × height less its openings with a head).
    fn wall_faces(
        &self,
        uid: Option<&str>,
        w: &kentos_contracts::WallFields,
    ) -> (f64, Option<f64>) {
        let length = bulge_path_length(&core(&w.pts), w.bulges.as_deref(), w.closed);
        let height = w.height.or_else(|| self.level_height(&w.arch));
        let openings: f64 = uid
            .map(|u| {
                self.openings_of(u)
                    .iter()
                    .filter_map(|(_, o)| o.head.map(|h| o.width * (h - o.sill.unwrap_or(0.0))))
                    .sum()
            })
            .unwrap_or(0.0);
        (length, height.map(|h| length * h - openings))
    }

    /// What `e` gives the place `place` (a tag's or a schedule's field name; `attr` with `attr` its attribute's name).
    pub fn arch_value(&self, e: &Entity, place: &str, attr: Option<&str>) -> Option<ArchValue> {
        use ArchValue::*;
        let text = |v: &Option<String>| v.clone().filter(|t| !t.is_empty()).map(Text);
        if place == "attr" {
            return e
                .base()
                .attrs
                .get(attr?)
                .filter(|v| !v.is_empty())
                .cloned()
                .map(Text);
        }
        if place == "count" {
            return Some(Count(1));
        }
        let uid = self.uid_of_entity(e);
        match e {
            Entity::Opening(o) => {
                let x = &o.opening;
                let wall = self.wall_of(&x.host);
                let height = x.head.map(|h| h - x.sill.unwrap_or(0.0));
                match place {
                    "number" => text(&x.number),
                    "type" | "style" => {
                        text(&x.style).map(|_| Text(self.opening_type_name(x.style.as_deref())))
                    }
                    "form" => Some(Text(x.form.label().to_owned())),
                    "width" => Some(Length(x.width)),
                    // Its widths (docs/adr/0772 §2): never the one for the other.
                    "roughWidth" => Some(Length(x.width)),
                    "unitWidth" => matches!(x.form, OpeningForm::Door | OpeningForm::Window)
                        .then(|| Length(x.sizes().unit)),
                    "leafWidth" => matches!(x.form, OpeningForm::Door | OpeningForm::Window)
                        .then(|| Length(x.sizes().leaf)),
                    "clearWidth" => x.sizes().clear.map(Length),
                    "typeVersion" => x.type_version.map(|v| Text(format!("s{v}"))),
                    "height" => height.map(Length),
                    "sill" => Some(Length(x.sill.unwrap_or(0.0))),
                    "head" => x.head.map(Length),
                    "area" => height.map(|h| Area(x.width * h)),
                    // An unstyled wall says its thickness, with its unit (docs/adr/0344's 27 October addendum).
                    "wall" => wall.map(|w| match w.style.filter(|s| !s.is_empty()) {
                        Some(s) => Text(self.wall_style_name(Some(&s))),
                        None => Text(format!(
                            "{} {}",
                            self.arch_value_text(&Length(w.width), false),
                            crate::text_fields::unit_name(self.settings().drawing_unit.unwrap_or_default())
                        )),
                    }),
                    "level" => wall.and_then(|w| self.level_name(&w.arch)).map(Text),
                    "phase" => wall
                        .and_then(|w| w.arch.phase)
                        .map(|p| Text(p.label().to_owned())),
                    _ => None,
                }
            }
            Entity::Space(s) => {
                let x = &s.space;
                let areas = core_areas(&x.parts);
                let area = kentos_geometry_core::geom::space::area(&areas);
                let height = x.height.or_else(|| self.level_height(&x.arch));
                match place {
                    "number" => text(&x.number),
                    "name" => text(&x.name),
                    "usage" => text(&x.usage),
                    "area" => Some(Area(area)),
                    "netArea" => crate::space_follow::SpaceSearch::new(self).net_area(x).map(Area),
                    "perimeter" => {
                        Some(Length(kentos_geometry_core::geom::space::perimeter(&areas)))
                    }
                    "height" => height.map(Length),
                    "volume" => height.map(|h| Volume(area * h)),
                    "floorElevation" => self.room_floor_elevation(&x.arch).map(Length),
                    // Floor-to-floor level height is not a stated room ceiling.
                    "ceilingElevation" => self
                        .room_floor_elevation(&x.arch)
                        .zip(x.height)
                        .map(|(floor, height)| Length(floor + height)),
                    "level" => self.level_name(&x.arch).map(Text),
                    "phase" => x.arch.phase.map(|p| Text(p.label().to_owned())),
                    // Its finishes (docs/adr/0733 §3).
                    "floorFinish" => text(&x.finishes.floor),
                    "baseFinish" => text(&x.finishes.base),
                    "wallFinish" => text(&x.finishes.wall),
                    "ceilingFinish" => text(&x.finishes.ceiling),
                    // Its baseboard: Metraj's rule (docs/adr/0722 §5); a void has none.
                    "baseboard" => (!x.void).then(|| {
                        Length(crate::passages::baseboard(
                            &x.parts,
                            x.arch.level.as_deref(),
                            &crate::passages::passages(self),
                        ))
                    }),
                    _ => None,
                }
            }
            Entity::Column(c) => {
                let x = &c.column;
                match place {
                    "tag" => text(&x.tag),
                    "place" => text(&x.place),
                    "type" => text(&x.kind),
                    "width" => Some(Length(x.width)),
                    "level" => self.level_name(&x.arch).map(Text),
                    "phase" => x.arch.phase.map(|p| Text(p.label().to_owned())),
                    _ => None,
                }
            }
            Entity::Wall(w) => {
                let x = &w.wall;
                let (length, net) = self.wall_faces(uid.as_deref(), x);
                match place {
                    "type" | "style" => {
                        text(&x.style).map(|_| Text(self.wall_style_name(x.style.as_deref())))
                    }
                    "thickness" | "width" => Some(Length(x.width)),
                    "length" => Some(Length(length)),
                    "height" => x.height.or_else(|| self.level_height(&x.arch)).map(Length),
                    "area" => net.map(Area),
                    "volume" => net.map(|n| Volume(n * x.width)),
                    "level" => self.level_name(&x.arch).map(Text),
                    "phase" => x.arch.phase.map(|p| Text(p.label().to_owned())),
                    _ => None,
                }
            }
            Entity::Grid(g) => match place {
                "label" => Some(Text(g.grid.label.clone())),
                "level" => self.level_name(&g.grid.arch).map(Text),
                "phase" => g.grid.arch.phase.map(|p| Text(p.label().to_owned())),
                _ => None,
            },
            Entity::Stair(s) => match place {
                "risers" => Some(Count(u64::from(s.stair.risers))),
                "width" => Some(Length(s.stair.width)),
                "level" => self.level_name(&s.stair.arch).map(Text),
                "phase" => s.stair.arch.phase.map(|p| Text(p.label().to_owned())),
                _ => None,
            },
            Entity::Ramp(r) => match place {
                "slope" => {
                    let x = &r.ramp;
                    x.slope
                        .or_else(|| {
                            let pts = core(&x.pts);
                            let spec = kentos_geometry_core::geom::arch::RampSpec {
                                width: x.width,
                                rise: x.rise,
                                slope: x.slope,
                                base: x.base,
                                landing: x.landing,
                                railings: None,
                            };
                            let run = kentos_geometry_core::geom::arch::ramp_run(
                                &kentos_geometry_core::geom::arch::Walk::of(&pts),
                                &spec,
                            );
                            (run > 0.0).then(|| x.rise / run * 100.0)
                        })
                        .map(Percent)
                }
                "width" => Some(Length(r.ramp.width)),
                "level" => self.level_name(&r.ramp.arch).map(Text),
                "phase" => r.ramp.arch.phase.map(|p| Text(p.label().to_owned())),
                _ => None,
            },
            Entity::Insert(i) => match place {
                "type" => self
                    .blocks()
                    .iter()
                    .find(|b| b.id == i.block)
                    .map(|b| Text(b.name.clone())),
                // A fixture's (docs/adr/0733 §2): its block's card category, its level and phase, the space it stands in.
                "category" => self
                    .blocks()
                    .iter()
                    .find(|b| b.id == i.block)
                    .and_then(|b| b.card.as_ref()?.category.clone())
                    .filter(|c| !c.is_empty())
                    .map(Text),
                "level" => self.level_name(&i.arch).map(Text),
                "phase" => i.arch.phase.map(|p| Text(p.label().to_owned())),
                "space" => {
                    let level = i.arch.level.as_deref();
                    self.space_at(self.fixture_middle(i), level)
                        .or_else(|| self.space_at(i.p, level))
                        .map(Text)
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// The box a fixture's block draws as the insert places it (docs/adr/0772 §5): its four corners; none when its
    /// block draws nothing.
    pub fn fixture_footprint(&self, i: &kentos_contracts::InsertEntity) -> Option<[kentos_contracts::Vec2; 4]> {
        use kentos_geometry_core::api::json::{FromJson, Json};
        use kentos_geometry_core::entity::{Entity as CoreEntity, entity_bounds};
        let b = self.blocks().iter().find(|b| b.id == i.block)?;
        let mut seen: Option<[f64; 4]> = None;
        for e in &b.entities {
            let Some(core) = serde_json::to_string(e)
                .ok()
                .and_then(|t| Json::parse(&t).ok())
                .and_then(|j| CoreEntity::from_json(&j).ok())
            else {
                continue;
            };
            let r = entity_bounds(&core.shape);
            if ![r.min_x, r.min_y, r.max_x, r.max_y].iter().all(|v| v.is_finite()) {
                continue;
            }
            seen = Some(match seen {
                None => [r.min_x, r.min_y, r.max_x, r.max_y],
                Some(o) => [o[0].min(r.min_x), o[1].min(r.min_y), o[2].max(r.max_x), o[3].max(r.max_y)],
            });
        }
        let [x0, y0, x1, y1] = seen?;
        let m = kentos_geometry_core::block::placement_xy(
            kentos_geometry_core::Vec2::new(i.p.x, i.p.y),
            i.scale,
            i.y_scale.unwrap_or(i.scale),
            i.rotation,
            i.mirror,
        );
        let at = |x: f64, y: f64| {
            let (x, y) = (x - b.base.x, y - b.base.y);
            kentos_contracts::Vec2 {
                x: m[0] * x + m[2] * y + m[4],
                y: m[1] * x + m[3] * y + m[5],
            }
        };
        Some([at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1)])
    }

    /// Where a fixture stands (docs/adr/0733 §2): the middle of its block's drawing as the insert places it, so that a
    /// cabinet whose point is on the wall's face stands in its room; its point when its block draws nothing.
    pub fn fixture_middle(&self, i: &kentos_contracts::InsertEntity) -> kentos_contracts::Vec2 {
        use kentos_geometry_core::api::json::{FromJson, Json};
        use kentos_geometry_core::entity::{Entity as CoreEntity, entity_bounds};
        let Some(b) = self.blocks().iter().find(|b| b.id == i.block) else {
            return i.p;
        };
        let mut seen: Option<[f64; 4]> = None;
        for e in &b.entities {
            let Some(core) = serde_json::to_string(e)
                .ok()
                .and_then(|t| Json::parse(&t).ok())
                .and_then(|j| CoreEntity::from_json(&j).ok())
            else {
                continue;
            };
            let r = entity_bounds(&core.shape);
            if ![r.min_x, r.min_y, r.max_x, r.max_y].iter().all(|v| v.is_finite()) {
                continue;
            }
            seen = Some(match seen {
                None => [r.min_x, r.min_y, r.max_x, r.max_y],
                Some(o) => [o[0].min(r.min_x), o[1].min(r.min_y), o[2].max(r.max_x), o[3].max(r.max_y)],
            });
        }
        let Some([x0, y0, x1, y1]) = seen else {
            return i.p;
        };
        let (x, y) = ((x0 + x1) / 2.0 - b.base.x, (y0 + y1) / 2.0 - b.base.y);
        let m = kentos_geometry_core::block::placement_xy(
            kentos_geometry_core::Vec2::new(i.p.x, i.p.y),
            i.scale,
            i.y_scale.unwrap_or(i.scale),
            i.rotation,
            i.mirror,
        );
        kentos_contracts::Vec2 {
            x: m[0] * x + m[2] * y + m[4],
            y: m[1] * x + m[3] * y + m[5],
        }
    }

    /// The space `p` stands in on `level` (none: among the spaces without a level), by its number and name; the first
    /// one in the drawing's order that holds it, a void none (docs/adr/0733 §2).
    pub fn space_at(&self, p: kentos_contracts::Vec2, level: Option<&str>) -> Option<String> {
        let at = kentos_geometry_core::Vec2::new(p.x, p.y);
        self.entities().find_map(|e| {
            let Entity::Space(s) = e else { return None };
            let x = &s.space;
            if x.void || x.arch.level.as_deref() != level {
                return None;
            }
            core_areas(&x.parts)
                .iter()
                .any(|a| kentos_geometry_core::geom::region::inside_area(a, at))
                .then(|| {
                    [x.number.as_deref(), x.name.as_deref()]
                        .into_iter()
                        .flatten()
                        .filter(|w| !w.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .filter(|w| !w.is_empty())
        })
    }

    /// A value's number in the unit its words show (a length in the drawing's unit, an area in the project's), not
    /// rounded; none for words.
    pub fn arch_value_number(&self, v: &ArchValue) -> Option<f64> {
        let s = self.settings();
        match v {
            ArchValue::Text(_) | ArchValue::Percent(_) => None,
            ArchValue::Length(m) => Some(m * s.drawing_unit.unwrap_or_default().per_metre()),
            ArchValue::Area(m) => Some(
                m / match s.area_unit {
                    AreaUnit::M2 => 1.0,
                    AreaUnit::Donum => 1000.0,
                    AreaUnit::Ha => 10_000.0,
                },
            ),
            ArchValue::Volume(m) => Some(*m),
            ArchValue::Count(n) => Some(*n as f64),
        }
    }

    /// A value's words; `unit`: an area's and a volume's unit after them (a tag's).
    pub fn arch_value_text(&self, v: &ArchValue, unit: bool) -> String {
        let s = self.settings();
        let drawing = s.drawing_unit.unwrap_or_default();
        let (area_metres, area_unit) = match s.area_unit {
            AreaUnit::M2 => (1.0, "m²"),
            AreaUnit::Donum => (1000.0, "dönüm"),
            AreaUnit::Ha => (10_000.0, "ha"),
        };
        match v {
            ArchValue::Text(t) => t.clone(),
            ArchValue::Length(m) => number_text(
                *m,
                1.0 / drawing.per_metre(),
                s.length_decimals as usize,
                false,
            ),
            ArchValue::Area(m) => {
                let t = number_text(*m, area_metres, s.area_decimals as usize, false);
                if unit { format!("{t} {area_unit}") } else { t }
            }
            ArchValue::Volume(m) => {
                let t = number_text(*m, 1.0, s.area_decimals as usize, false);
                if unit { format!("{t} m³") } else { t }
            }
            ArchValue::Count(n) => n.to_string(),
            ArchValue::Percent(p) => percent_text(*p),
        }
    }

    /// The point a tag of `e` ties to; none for an object no tag is put on.
    pub fn tag_anchor(&self, e: &Entity) -> Option<Vec2> {
        match e {
            Entity::Opening(o) => {
                let p = &o.opening.place;
                (p.len() == 4).then(|| Vec2 {
                    x: (p[0].x + p[1].x + p[2].x + p[3].x) / 4.0,
                    y: (p[0].y + p[1].y + p[2].y + p[3].y) / 4.0,
                })
            }
            Entity::Space(s) => {
                kentos_geometry_core::geom::space::label_at(&core_areas(&s.space.parts))
                    .map(|p| Vec2 { x: p.x, y: p.y })
            }
            Entity::Column(c) => Some(c.column.at),
            Entity::Insert(i) => Some(i.p),
            Entity::Grid(g) => path_middle(&g.grid.pts, g.grid.bulges.as_deref()),
            Entity::Wall(w) => path_middle(&w.wall.pts, w.wall.bulges.as_deref()),
            Entity::Stair(s) => path_middle(&s.stair.pts, s.stair.bulges.as_deref()),
            Entity::Ramp(r) => path_middle(&r.ramp.pts, None),
            _ => None,
        }
    }

    /// A tag's words of `e`: its template (none: its kind's) filled with `e`'s values.
    pub fn tag_text(&self, e: &Entity, content: Option<&str>) -> String {
        crate::tag_read::TagRead::new(self).text(e, content)
    }

    fn uid_of_entity(&self, e: &Entity) -> Option<String> {
        self.uid(crate::identity::Slot(e.base().id))
            .map(|u| u.to_string())
    }
}
