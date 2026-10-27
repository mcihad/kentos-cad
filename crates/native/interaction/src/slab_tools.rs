//! Döşeme (docs/adr/0722 §4, §6). The tool says what was clicked and chosen; `cad.slabs.create` and `cad.slabs.edit`
//! compute and write, one undo step each, and the plan is the preview.
//!
//! - Sınır (S): Dış yüzler finds the outline of the walls round the click (a storey's floor, to the walls' outer
//!   faces), Oda the room round it (to their inner faces: a wet room's lowered slab), Köşeleri çiz takes corners and
//!   Enter closes them. The slab the command would write is drawn with its area and its edge; a click writes it and the
//!   tool stays.
//! - Tür (T), Kalınlık (K), Kot farkı (F), Kenar ötelemesi (Ö), Kenar (E: Yok, Korkuluk, Parapet; along the free
//!   sides), Kenar eni (W), Malzeme (M), Metraj dışı (X), Kat (L).
//! - Delik (D): the room clicked (or the area drawn) is cut out of the slab under it: a shaft, a lowered wet room.

use kentos_commands::{
    CommandResult, Entity, NewSlab, NewSlabEdge, SlabBoundary, SlabEdgeKind, SlabRole, SlabsCreate, SlabsEdit,
    SlabsOperation, SpacePart, SpaceRing,
};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, slabs_create, slabs_edit, spaces};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Area, Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};

pub const SLAB_ID: &str = "slab";

/// How the outline is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Bound {
    #[default]
    Outline,
    Room,
    Draw,
}

impl Bound {
    fn label(self) -> &'static str {
        match self {
            Bound::Outline => "Dış yüzler",
            Bound::Room => "Oda",
            Bound::Draw => "Köşeleri çiz",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Bound::Outline => "outline",
            Bound::Room => "room",
            Bound::Draw => "draw",
        }
    }
}

/// What the tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Thickness,
    Offset,
    EdgeOffset,
    EdgeWidth,
    Material,
}

/// The icon of a role in Tür's menu, of an edge in Kenar's.
pub fn edge_icon(kind: Option<SlabEdgeKind>) -> Option<&'static str> {
    match kind {
        None => None,
        Some(SlabEdgeKind::Railing) => Some("slabRailing"),
        Some(SlabEdgeKind::Parapet) => Some("slabParapet"),
    }
}

/// Display rings for the preview fill.
fn rings_of(parts: &[SpacePart]) -> Vec<Vec<Vec2>> {
    let ring = |r: &SpaceRing| {
        let pts: Vec<Vec2> = r.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        kentos_geometry_core::entity::polygon_ring(&pts, r.bulges.as_deref())
    };
    parts.iter().flat_map(|p| std::iter::once(ring(&p.outer)).chain(p.holes.iter().map(ring))).collect()
}

/// Whether a point lies inside the rings (even-odd).
fn inside(rings: &[Vec<Vec2>], p: Vec2) -> bool {
    let mut c = false;
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                c = !c;
            }
        }
    }
    c
}

/// A slab drawn: its rings and its edge as the drawing shows them, and its fill.
pub fn drawn_slab(e: &Entity, tone: Tone, out: &mut Preview) {
    let Entity::Slab(s) = e else { return };
    if let Shape::Space { parts, space } = shape(e) {
        for piece in kentos_geometry_core::geom::space::pieces_with(&parts, &space, 1.0, false) {
            out.strokes.extend(Outline::of(&piece, None, 1.5, tone).strokes);
        }
    }
    out.areas.push(Area { rings: rings_of(&s.slab.parts), fill: 0.14, width: 0.0, dash: None, fill_tone: tone });
}

/// The length of a slab's edge's runs, metres.
pub fn edge_length(s: &kentos_commands::SlabFields) -> f64 {
    s.edge.as_ref().map_or(0.0, |e| {
        e.runs
            .iter()
            .map(|r| {
                let pts: Vec<Vec2> = r.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
                kentos_geometry_core::geom::bulge::bulge_path_length(&pts, r.bulges.as_deref(), false)
            })
            .sum()
    })
}

/// Döşeme.
#[derive(Clone, Debug)]
pub struct SlabTool {
    bound: Bound,
    role: SlabRole,
    thickness: f64,
    offset: f64,
    edge_offset: f64,
    edge: Option<SlabEdgeKind>,
    edge_width: Option<f64>,
    material: Option<String>,
    excluded: bool,
    /// Delik: cut out of the slab under the click.
    hole: bool,
    level: crate::architecture_level::Choice,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    /// What the plan would write: its area and its edge's length; or why it would not.
    area: Option<f64>,
    edge_len: f64,
    why: Option<String>,
    format: Format,
}

impl Default for SlabTool {
    fn default() -> Self {
        Self {
            bound: Bound::default(),
            role: SlabRole::default(),
            thickness: kentos_commands::SLAB_THICKNESS,
            offset: 0.0,
            edge_offset: 0.0,
            edge: None,
            edge_width: None,
            material: None,
            excluded: false,
            hole: false,
            level: Default::default(),
            pts: Vec::new(),
            hover: None,
            ask: None,
            drawn: Preview::default(),
            area: None,
            edge_len: 0.0,
            why: None,
            format: Format::default(),
        }
    }
}

impl SlabTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn draws(&self) -> bool {
        self.bound == Bound::Draw
    }

    /// The boundary a click finds: Delik cuts a room, never the whole outline.
    fn boundary(&self) -> SlabBoundary {
        if self.hole || self.bound == Bound::Room { SlabBoundary::Room } else { SlabBoundary::Outline }
    }

    fn corners_part(corners: &[Vec2]) -> Vec<SpacePart> {
        vec![SpacePart { outer: SpaceRing { pts: points::wire_all(corners), bulges: None }, holes: Vec::new() }]
    }

    fn new_slab(&self, seed: Option<Vec2>, corners: Option<&[Vec2]>) -> NewSlab {
        NewSlab {
            parts: corners.map(Self::corners_part),
            seed: seed.map(points::wire),
            boundary: seed.map(|_| self.boundary()),
            edge_offset: (self.edge_offset != 0.0).then_some(self.edge_offset),
            role: (!self.role.is_floor()).then_some(self.role),
            thickness: Some(self.thickness),
            offset: (self.offset != 0.0).then_some(self.offset),
            arch: self.level.arch(),
            material: self.material.clone(),
            edge: self.edge.map(|kind| NewSlabEdge { kind, width: self.edge_width }),
            excluded: self.excluded.then_some(true),
            attrs: None,
        }
    }

    fn create_input(&self, slab: NewSlab, cx: &Context<'_>) -> SlabsCreate {
        SlabsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            slabs: vec![slab],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// The slab under a point: the innermost visible one whose rings hold it.
    fn slab_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        let mut best: Option<(f64, Slot)> = None;
        for e in cx.doc.entities() {
            let Entity::Slab(s) = e else { continue };
            if !cx.doc.layers().is_visible(&e.base().layer_id) {
                continue;
            }
            if inside(&rings_of(&s.slab.parts), at) {
                let a = spaces::area_of(&s.slab.parts);
                if best.is_none_or(|(b, _)| a < b) {
                    best = Some((a, Slot(e.base().id)));
                }
            }
        }
        best.map(|(_, s)| s)
    }

    fn hole_input(&self, seed: Option<Vec2>, corners: Option<&[Vec2]>, cx: &Context<'_>) -> Result<(SlabsEdit, Slot), String> {
        let at = seed.or_else(|| corners.and_then(|c| c.first().copied())).ok_or_else(String::new)?;
        let slot = Self::slab_at(at, cx).ok_or_else(|| "Altında döşeme yok; deliği bir döşemenin içinde açın.".to_owned())?;
        let uid = cx.doc.uid(slot).ok_or_else(String::new)?.to_string();
        Ok((
            SlabsEdit {
                uids: vec![uid],
                operation: SlabsOperation::Subtract {
                    parts: corners.map(Self::corners_part),
                    seed: seed.map(points::wire),
                    boundary: seed.map(|_| SlabBoundary::Room),
                },
                expected_revision: None,
            },
            slot,
        ))
    }

    /// The corners drawn and the pointer, or the click: none until a drawn outline has three corners.
    fn hovered(&self) -> Option<(Option<Vec2>, Option<Vec<Vec2>>)> {
        let at = self.hover?;
        if self.draws() {
            let mut c = self.pts.clone();
            c.push(at);
            (c.len() >= 3).then_some((None, Some(c)))
        } else {
            Some((Some(at), None))
        }
    }

    /// The plan at the hover, drawn.
    fn see(&mut self, cx: &Context<'_>) {
        self.format = cx.format();
        self.level.see(cx.doc.settings());
        // Etkin kat while it has walls (docs/adr/0713).
        self.level.follow_where_walls(cx.level, cx.doc);
        self.drawn = Preview::default();
        self.area = None;
        self.edge_len = 0.0;
        self.why = None;
        if self.draws() {
            self.drawn.path = self.pts.clone();
            if let Some(at) = self.hover {
                self.drawn.path.push(at);
            }
        }
        let Some((seed, corners)) = self.hovered() else {
            return;
        };
        if self.hole {
            let (input, slot) = match self.hole_input(seed, corners.as_deref(), cx) {
                Ok(found) => found,
                Err(why) => {
                    self.why = (!why.is_empty()).then_some(why);
                    return;
                }
            };
            let before = cx
                .doc
                .get(slot)
                .and_then(|e| match e {
                    Entity::Slab(s) => Some(spaces::area_of(&s.slab.parts)),
                    _ => None,
                })
                .unwrap_or(0.0);
            match slabs_edit::plan_in(cx.doc, &input) {
                CommandResult::Completed { output, .. } => {
                    for e in &output.slabs {
                        drawn_slab(e, Tone::Accent, &mut self.drawn);
                        if let Entity::Slab(s) = e {
                            self.area = Some(before - spaces::area_of(&s.slab.parts));
                        }
                    }
                }
                CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                    self.why = Some(error.message)
                }
                CommandResult::Queued { .. } | CommandResult::Cancelled => {}
            }
            return;
        }
        let input = self.create_input(self.new_slab(seed, corners.as_deref()), cx);
        match slabs_create::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for e in &output.slabs {
                    drawn_slab(e, Tone::Accent, &mut self.drawn);
                    if let Entity::Slab(s) = e {
                        self.area = Some(spaces::area_of(&s.slab.parts));
                        self.edge_len = edge_length(&s.slab);
                    }
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.why = Some(error.message)
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// Writes the slab (or cuts the hole) at the click or the corners drawn.
    fn write(&mut self, seed: Option<Vec2>, corners: Option<&[Vec2]>, cx: &mut Context<'_>) -> bool {
        if self.hole {
            let input = match self.hole_input(seed, corners, cx) {
                Ok((input, _)) => input,
                Err(why) => {
                    if !why.is_empty() {
                        cx.say(Level::Warn, why);
                    }
                    return false;
                }
            };
            let cut = self.area;
            let result = slabs_edit::execute(&mut ExecutionContext::new(cx.doc), input);
            if points::written(result, cx).is_none() {
                return false;
            }
            let format = cx.format();
            cx.say(
                Level::Success,
                match cut {
                    Some(a) => format!("Döşemeden {} kesildi.", format.area(a)),
                    None => "Döşemeden delik kesildi.".to_owned(),
                },
            );
            self.pts.clear();
            return true;
        }
        let input = self.create_input(self.new_slab(seed, corners), cx);
        let area = self.area;
        let result = slabs_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_none() {
            return false;
        }
        let format = cx.format();
        cx.say(
            Level::Success,
            match area {
                Some(a) => format!("{} oluşturuldu: {}.", self.role.label(), format.area(a)),
                None => format!("{} oluşturuldu.", self.role.label()),
            },
        );
        self.pts.clear();
        true
    }

    fn bound_choices(&self) -> Vec<OptionChoice> {
        [Bound::Outline, Bound::Room, Bound::Draw]
            .iter()
            .filter(|b| !(self.hole && **b == Bound::Outline))
            .map(|b| OptionChoice {
                typed: b.name().into(),
                label: b.label().into(),
                icon: Some(match b {
                    Bound::Outline => "spaceGross",
                    Bound::Room => "spaceNet",
                    Bound::Draw => "revcloudPolygon",
                }),
                preview: None,
                checked: *b == self.bound || (self.hole && self.bound == Bound::Outline && *b == Bound::Room),
                command: None,
            })
            .collect()
    }

    fn role_choices(&self) -> Vec<OptionChoice> {
        SlabRole::ALL
            .iter()
            .map(|r| OptionChoice {
                typed: r.name().into(),
                label: r.label().into(),
                icon: Some("slab"),
                preview: None,
                checked: *r == self.role,
                command: None,
            })
            .collect()
    }

    fn edge_choices(&self) -> Vec<OptionChoice> {
        std::iter::once(None)
            .chain(SlabEdgeKind::ALL.iter().map(|k| Some(*k)))
            .map(|k| OptionChoice {
                typed: k.map_or("none", SlabEdgeKind::name).into(),
                label: k.map_or("Yok", SlabEdgeKind::label).into(),
                icon: edge_icon(k),
                preview: None,
                checked: k == self.edge,
                command: None,
            })
            .collect()
    }

    fn edge_width(&self) -> f64 {
        self.edge_width.or_else(|| self.edge.map(SlabEdgeKind::width)).unwrap_or(0.0)
    }

    fn cycle_bound(&mut self) {
        self.bound = match self.bound {
            Bound::Outline => Bound::Room,
            Bound::Room => Bound::Draw,
            Bound::Draw if self.hole => Bound::Room,
            Bound::Draw => Bound::Outline,
        };
        self.pts.clear();
    }

    /// A typed length set; false (and said) when it is no length.
    fn set_length(&mut self, ask: Ask, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(v) = cx.typed_length(text) else {
            cx.say(Level::Warn, format!("“{text}” bir uzunluk değil; 15 cm ya da 0.15 gibi yazın."));
            return false;
        };
        match ask {
            Ask::Thickness => self.thickness = v,
            Ask::Offset => self.offset = v,
            Ask::EdgeOffset => self.edge_offset = v,
            Ask::EdgeWidth => self.edge_width = Some(v),
            Ask::Material => {}
        }
        true
    }
}

impl Tool for SlabTool {
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// Kat follows the new active level at once while it has walls (docs/adr/0720 §4).
    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
    }

    fn id(&self) -> &'static str {
        SLAB_ID
    }

    fn label(&self) -> &'static str {
        "Döşeme"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snaps(&self) -> bool {
        self.draws()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.pts.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        let words = |label: &'static str| Prompt::new("Döşeme", label).option("Geri", "G");
        match self.ask {
            Some(Ask::Thickness) => return words("döşemenin kalınlığını yazın (15 cm gibi)"),
            Some(Ask::Offset) => return words("üst yüzün katın kotuna göre farkını yazın (-5 cm gibi)"),
            Some(Ask::EdgeOffset) => return words("kenarların ötelemesini yazın: artı dışa, eksi içe"),
            Some(Ask::EdgeWidth) => return words("kenarın enini yazın"),
            Some(Ask::Material) => return words("malzemeyi yazın (Betonarme gibi)"),
            None => {}
        }
        let step = match (self.hole, self.draws()) {
            (true, false) => "döşemeden kesilecek odaya tıklayın",
            (true, true) if self.pts.len() < 3 => "kesilecek alanın köşelerine tıklayın",
            (false, false) if self.bound == Bound::Room => "döşemesi yapılacak odaya tıklayın",
            (false, false) => "duvarların içine tıklayın: dış konturdan döşeme",
            (_, true) if self.pts.len() < 3 => "döşemenin köşelerine tıklayın",
            _ => "köşelere tıklayın, Enter ile kapatın",
        };
        let f = &self.format;
        let mut p = Prompt::new("Döşeme", step)
            .option_with("Sınır", "S", if self.hole && self.bound == Bound::Outline { "Oda" } else { self.bound.label() })
            .option_with("Tür", "T", self.role.label())
            .option_with("Kalınlık", "K", f.length(self.thickness))
            .option_with("Kot farkı", "F", f.length(self.offset))
            .option_with("Kenar ötelemesi", "Ö", f.length(self.edge_offset))
            .option_with("Kenar", "E", self.edge.map_or("Yok", SlabEdgeKind::label))
            .option_with("Kat", "L", self.level.label())
            .toggle("Metraj dışı", "X", self.excluded)
            .toggle("Delik", "D", self.hole);
        if self.edge.is_some() {
            p = p.option_with("Kenar eni", "W", f.length(self.edge_width()));
        }
        p = p.option_with("Malzeme", "M", self.material.clone().unwrap_or_else(|| "Yok".into()));
        if self.draws() && !self.pts.is_empty() {
            p = p.option("Geri al", "G");
        }
        p.option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(if self.draws() { p.world } else { p.raw });
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.draws() {
            self.accept_point(p.world, cx);
            return;
        }
        self.write(Some(p.raw), None, cx);
        self.see(cx);
    }

    fn accepts_points(&self) -> bool {
        self.draws()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.draws() {
            return false;
        }
        self.pts.push(p);
        self.see(cx);
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if let Some(ask) = self.ask {
            if key == "G" {
                self.ask = None;
                return true;
            }
            let value = js_trim(text);
            if ask == Ask::Material {
                self.material = (!value.is_empty()).then(|| value.to_owned());
            } else if !self.set_length(ask, value, cx) {
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "S" => self.cycle_bound(),
            "T" => {
                self.role = match self.role {
                    SlabRole::Floor => SlabRole::Balcony,
                    SlabRole::Balcony => SlabRole::Terrace,
                    SlabRole::Terrace => SlabRole::Floor,
                }
            }
            "E" => {
                self.edge = match self.edge {
                    None => Some(SlabEdgeKind::Railing),
                    Some(SlabEdgeKind::Railing) => Some(SlabEdgeKind::Parapet),
                    Some(SlabEdgeKind::Parapet) => None,
                };
                self.edge_width = None;
            }
            "K" => self.ask = Some(Ask::Thickness),
            "F" => self.ask = Some(Ask::Offset),
            "Ö" => self.ask = Some(Ask::EdgeOffset),
            "W" if self.edge.is_some() => self.ask = Some(Ask::EdgeWidth),
            "M" => self.ask = Some(Ask::Material),
            "X" => self.excluded = !self.excluded,
            "D" => self.hole = !self.hole,
            "L" => self.level.next(),
            "G" if self.draws() => return self.undo_step(cx),
            _ => {
                if self.draws()
                    && let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover)
                {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        }
        self.see(cx);
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        match key {
            "S" => self.bound_choices(),
            "T" => self.role_choices(),
            "E" => self.edge_choices(),
            "L" => self.level.setting().choices,
            _ => Vec::new(),
        }
    }

    fn settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::Setting;
        let mut rows = vec![
            Setting::choice("S", "Sınır", self.bound_choices()),
            Setting::choice("T", "Tür", self.role_choices()),
            Setting::text("K", "Kalınlık", f.length(self.thickness)),
            Setting::text("F", "Kot farkı", f.length(self.offset)),
            Setting::text("Ö", "Kenar ötelemesi", f.length(self.edge_offset)),
            Setting::choice("E", "Kenar", self.edge_choices()),
        ];
        if self.edge.is_some() {
            rows.push(Setting::text("W", "Kenar eni", f.length(self.edge_width())));
        }
        rows.extend([
            Setting::text("M", "Malzeme", self.material.clone().unwrap_or_default()),
            self.level.setting(),
            Setting::toggle("X", "Metraj dışı", self.excluded),
            Setting::toggle("D", "Delik (döşemeden kes)", self.hole),
        ]);
        if self.hole {
            rows.push(Setting::hint(
                "DH",
                "Delik",
                "Tıklanan oda ya da çizilen alan altındaki döşemeden kesilir; ıslak hacmin alçak döşemesini Oda ile ayrıca çizin.",
            ));
        }
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if matches!(key, "S" | "T" | "E" | "L") {
            return self.choose_option(key, value, cx);
        }
        match key {
            "K" | "F" | "Ö" | "W" => {
                let ask = match key {
                    "K" => Ask::Thickness,
                    "F" => Ask::Offset,
                    "Ö" => Ask::EdgeOffset,
                    _ => Ask::EdgeWidth,
                };
                if !self.set_length(ask, value.trim(), cx) {
                    return false;
                }
            }
            "M" => self.material = (!value.trim().is_empty()).then(|| value.trim().to_owned()),
            "X" if matches!(value, "yes" | "no") => self.excluded = value == "yes",
            "D" if matches!(value, "yes" | "no") => {
                self.hole = value == "yes";
                self.pts.clear();
            }
            _ => return false,
        }
        self.see(cx);
        true
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "S" => {
                let Some(b) = [Bound::Outline, Bound::Room, Bound::Draw].into_iter().find(|b| b.name() == typed || b.label() == typed)
                else {
                    return false;
                };
                if self.bound != b {
                    self.pts.clear();
                }
                self.bound = b;
            }
            "T" => {
                let Some(r) = SlabRole::ALL.into_iter().find(|r| r.name() == typed || r.label() == typed) else {
                    return false;
                };
                self.role = r;
            }
            "E" => {
                self.edge = match typed {
                    "none" | "Yok" => None,
                    _ => match SlabEdgeKind::ALL.into_iter().find(|k| k.name() == typed || k.label() == typed) {
                        Some(k) => Some(k),
                        None => return false,
                    },
                };
                self.edge_width = None;
            }
            "L" => {
                self.level.see(cx.doc.settings());
                if !self.level.choose(typed) {
                    return false;
                }
            }
            _ => return false,
        }
        self.see(cx);
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            return true;
        }
        if !self.pts.is_empty() {
            self.pts.clear();
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.draws() && self.pts.len() >= 3 {
            let corners = self.pts.clone();
            self.write(None, Some(&corners), cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let title = if self.hole { "Delik" } else { self.role.label() };
        let tag = self.hover.map(|at| Tag {
            at,
            lines: match (&self.why, self.area) {
                (Some(why), _) if !self.draws() || self.pts.len() >= 2 => vec![title.to_owned(), why.clone()],
                (_, Some(a)) if self.hole => vec![format!("Delik · {}", format.area(a))],
                (_, Some(a)) => {
                    let mut lines = vec![format!("{title} · {} · {}", format.area(a), format.length(self.thickness))];
                    if let Some(k) = self.edge {
                        lines.push(if self.edge_len > 0.0 {
                            format!("{} · {}", k.label(), format.length(self.edge_len))
                        } else {
                            format!("{}: serbest kenar yok", k.label())
                        });
                    }
                    if self.excluded {
                        lines.push("Metraj toplamına katılmaz".to_owned());
                    }
                    lines
                }
                _ => vec![title.to_owned()],
            },
        });
        Preview {
            tag,
            tag_tone: if self.why.is_some() && self.area.is_none() && !self.draws() { Tone::Danger } else { Tone::Accent },
            ..self.drawn.clone()
        }
    }
}
