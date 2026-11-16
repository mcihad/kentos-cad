//! Kaplama (`tool.covering`, docs/adr/8070 §10): a covering, a slab written as one (`cad.slabs.create` with its
//! `covering`), its plan the preview.
//!
//! - Sınır (S): Mahal takes the room clicked, the covering bound to it (its outline the room's, following it); Köşeleri
//!   çiz takes corners and Enter closes them.
//! - Tip (P): IFC 4.3's covering types, Asma tavan (CEILING) until changed; Kalınlık (E), Kot farkı (F: its top over its
//!   storey's floor level; Oto: on a room's ceiling for a ceiling, on its floor otherwise), Malzeme (M).
//! - First values, KentOS's own: a ceiling 12.5 mm (a common plasterboard thickness, docs/adr/8070 §10), another
//!   covering 2 cm (the floor build-up's kaplama, docs/adr/8040 §2).

use kentos_commands::{Covering, Entity, NewSlab, SlabsCreate, SpacePart, SpaceRing};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, slabs_create};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};

pub const ID: &str = "covering";

/// A ceiling's first thickness, metres: a common plasterboard's 12.5 mm (docs/adr/8070 §10).
pub const CEILING_THICKNESS: f64 = 0.0125;
/// Another covering's first thickness, metres: the floor build-up's kaplama (docs/adr/8040 §2).
pub const COVERING_THICKNESS: f64 = 0.02;

/// What the tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Thickness,
    Offset,
    Material,
}

/// Kaplama.
#[derive(Clone, Debug)]
pub struct CoveringTool {
    covering: Covering,
    /// Mahal: the room clicked; else the corners drawn.
    room: bool,
    thickness: f64,
    /// Its top over its storey's floor level; none: on the room's ceiling (a ceiling) or floor (another covering).
    offset: Option<f64>,
    material: Option<String>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    area: Option<f64>,
    why: Option<String>,
}

impl Default for CoveringTool {
    fn default() -> Self {
        Self {
            covering: Covering::ceiling(),
            room: true,
            thickness: CEILING_THICKNESS,
            offset: None,
            material: None,
            pts: Vec::new(),
            hover: None,
            ask: None,
            drawn: Preview::default(),
            area: None,
            why: None,
        }
    }
}

/// A type's covering: a ceiling a suspended one ("Asma tavan"), a user-defined one "Kaplama".
fn covering_of(predefined: &str) -> Covering {
    match predefined {
        "CEILING" => Covering::ceiling(),
        "USERDEFINED" => Covering {
            predefined: predefined.into(),
            name: Some("Kaplama".into()),
        },
        _ => Covering {
            predefined: predefined.into(),
            name: None,
        },
    }
}

impl CoveringTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn ceiling(&self) -> bool {
        self.covering.predefined == "CEILING"
    }

    /// The room under `at`, by its persistent id, with its fields: on the active storey first.
    fn room_at(at: Vec2, cx: &Context<'_>) -> Option<(String, kentos_commands::SpaceFields)> {
        let p = kentos_geometry_core::Vec2::new(at.x, at.y);
        let level = cx.new_arch().and_then(|a| a.level);
        let mut found: Option<(bool, String, kentos_commands::SpaceFields)> = None;
        for (uid, e) in cx.doc.entities_with_uids() {
            let Entity::Space(s) = e else { continue };
            if s.space.void || !cx.doc.layers().is_visible(&e.base().layer_id) {
                continue;
            }
            let holds = kentos_domain::arch_values::core_areas(&s.space.parts)
                .iter()
                .any(|a| kentos_geometry_core::geom::region::inside_area(a, p));
            if !holds {
                continue;
            }
            let here = s.space.arch.level == level;
            if found.as_ref().is_none_or(|(h, _, _)| here && !h) {
                found = Some((here, uid.to_string(), s.space.clone()));
            }
        }
        found.map(|(_, u, s)| (u, s))
    }

    /// Its storey's floor level, metres over ±0.00.
    fn storey_floor(level: Option<&str>, cx: &Context<'_>) -> (f64, Option<f64>) {
        cx.doc
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.levels.iter().find(|l| Some(l.id.as_str()) == level))
            .map_or((0.0, None), |l| (l.elevation, l.height))
    }

    /// The covering to write at the click or the corners drawn; why there is none.
    fn new_slab(&self, at: Option<Vec2>, corners: Option<&[Vec2]>, cx: &Context<'_>) -> Result<NewSlab, String> {
        let (space, arch, auto) = match (at, corners) {
            (Some(at), _) => {
                let (uid, s) = Self::room_at(at, cx).ok_or("Burada mahal yok; kaplamayı bir mahalin içine tıklayın.")?;
                let (level_floor, _) = Self::storey_floor(s.arch.level.as_deref(), cx);
                // On its ceiling (its asma tavan, its height, the slab over it) or on its floor (docs/adr/8040 §3).
                let h = cx.doc.room_height(&s);
                let base = match (&h, self.ceiling()) {
                    (Some(h), true) => h.ceiling + self.thickness,
                    (Some(h), false) => h.floor + self.thickness,
                    (None, _) => level_floor + self.thickness,
                };
                let arch = kentos_commands::ArchInfo { level: s.arch.level.clone(), ..Default::default() };
                (Some(uid), Some(arch), base - level_floor)
            }
            (None, _) => {
                let arch = cx.new_arch();
                let (level_floor, height) = Self::storey_floor(arch.as_ref().and_then(|a| a.level.as_deref()), cx);
                let _ = level_floor;
                // A ceiling drawn by hand under its storey's top; another covering on its floor.
                let auto = match (self.ceiling(), height) {
                    (true, Some(h)) => h,
                    _ => self.thickness,
                };
                (None, arch, auto)
            }
        };
        Ok(NewSlab {
            parts: corners.map(|c| {
                vec![SpacePart { outer: SpaceRing { pts: points::wire_all(c), bulges: None }, holes: Vec::new() }]
            }),
            space,
            thickness: Some(self.thickness),
            offset: Some(self.offset.unwrap_or(auto)),
            arch,
            material: self.material.clone(),
            covering: Some(self.covering.clone()),
            ..NewSlab::default()
        })
    }

    fn kind(&self) -> crate::arch_layer::Kind {
        crate::arch_layer::Kind::Covering(Covering::element(&self.covering.predefined))
    }

    fn call(&self, slab: NewSlab, cx: &Context<'_>) -> SlabsCreate {
        SlabsCreate {
            layer_id: crate::arch_layer::planned(&self.kind(), cx),
            slabs: vec![slab],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// Where it would go now: the click, or the corners drawn and the pointer.
    fn hovered(&self) -> Option<(Option<Vec2>, Option<Vec<Vec2>>)> {
        let at = self.hover?;
        if self.room {
            return Some((Some(at), None));
        }
        let mut c = self.pts.clone();
        c.push(at);
        (c.len() >= 3).then_some((None, Some(c)))
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.area = None;
        self.why = None;
        if !self.room {
            self.drawn.path = self.pts.clone();
            if let Some(at) = self.hover {
                self.drawn.path.push(at);
            }
        }
        let Some((at, corners)) = self.hovered() else { return };
        let slab = match self.new_slab(at, corners.as_deref(), cx) {
            Ok(s) => s,
            Err(why) => {
                self.why = Some(why);
                return;
            }
        };
        match slabs_create::plan_in(cx.doc, &self.call(slab, cx)) {
            kentos_commands::CommandResult::Completed { output, .. } => {
                for e in &output.slabs {
                    crate::slab_tools::drawn_slab(e, Tone::Accent, &mut self.drawn);
                    if let Entity::Slab(s) = e {
                        self.area = Some(kentos_native_application::spaces::area_of(&s.slab.parts));
                    }
                }
            }
            kentos_commands::CommandResult::Failed { error }
            | kentos_commands::CommandResult::Conflict { error }
            | kentos_commands::CommandResult::NeedsInput { error } => self.why = Some(error.message),
            _ => {}
        }
    }

    fn write(&mut self, at: Option<Vec2>, corners: Option<&[Vec2]>, cx: &mut Context<'_>) -> bool {
        let slab = match self.new_slab(at, corners, cx) {
            Ok(s) => s,
            Err(why) => {
                cx.say(Level::Warn, why);
                return false;
            }
        };
        let area = self.area;
        let kind = self.kind();
        let written = crate::arch_layer::writing(&kind, cx, |layer, cx| {
            let input = SlabsCreate { layer_id: layer.to_owned(), ..self.call(slab.clone(), cx) };
            let result = slabs_create::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        });
        if !written {
            return false;
        }
        let format = cx.format();
        let what = self.covering.words();
        cx.say(
            Level::Success,
            match area {
                Some(a) => format!("{what} oluşturuldu: {}.", format.area(a)),
                None => format!("{what} oluşturuldu."),
            },
        );
        self.pts.clear();
        true
    }

    fn type_choices(&self) -> Vec<OptionChoice> {
        Covering::TYPES
            .iter()
            .map(|t| OptionChoice {
                typed: (*t).into(),
                label: format!("{} ({t})", if *t == "CEILING" { "Asma tavan" } else { Covering::word(t) }),
                icon: None,
                preview: None,
                checked: *t == self.covering.predefined,
                command: None,
            })
            .collect()
    }

    fn bound_choices(&self) -> Vec<OptionChoice> {
        [("room", "Mahal", true), ("draw", "Köşeleri çiz", false)]
            .into_iter()
            .map(|(typed, label, room)| OptionChoice {
                typed: typed.into(),
                label: label.into(),
                icon: None,
                preview: None,
                checked: room == self.room,
                command: None,
            })
            .collect()
    }

    /// A length typed for `ask`; false when it does not read. Kot farkı empty or Oto: worked out from the room.
    fn set_length(&mut self, ask: Ask, value: &str, cx: &mut Context<'_>) -> bool {
        let words = upper_tr(js_trim(value));
        if ask == Ask::Offset && (words.is_empty() || words.starts_with("OTO")) {
            self.offset = None;
            return true;
        }
        let Some(v) = cx.typed_length(value).filter(|v| v.is_finite()) else {
            cx.say(Level::Warn, "Bir uzunluk yazın: 12.5mm ya da 0.02m gibi.");
            return false;
        };
        match ask {
            Ask::Thickness if v > 0.0 => self.thickness = v,
            Ask::Thickness => {
                cx.say(Level::Warn, "Kaplamanın kalınlığı sıfırdan büyük olmalı.");
                return false;
            }
            Ask::Offset => self.offset = Some(v),
            Ask::Material => {}
        }
        true
    }
}

impl Tool for CoveringTool {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(self.kind())
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        "Kaplama"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snaps(&self) -> bool {
        !self.room
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
        let words = |label: &'static str| Prompt::new("Kaplama", label).option("Geri", "G");
        match self.ask {
            Some(Ask::Thickness) => return words("kaplamanın kalınlığını yazın (12.5 mm gibi)"),
            Some(Ask::Offset) => {
                return words("üst yüzünün katın kotuna göre farkını yazın (boş ya da Oto: mahalin tavanında ya da tabanında)");
            }
            Some(Ask::Material) => return words("malzemesini yazın (Alçıpan, Parke gibi; boş: söylenmez)"),
            None => {}
        }
        let step = match (self.room, self.pts.len()) {
            (true, _) => "kaplanacak mahale tıklayın",
            (false, n) if n < 3 => "kaplamanın köşelerine tıklayın",
            _ => "köşelere tıklayın, Enter ile kapatın",
        };
        let tip = if self.ceiling() { "Asma tavan".to_owned() } else { Covering::word(&self.covering.predefined).to_owned() };
        let offset = self.offset.map_or_else(|| "Oto".to_owned(), |v| v.to_string());
        let mut p = Prompt::new("Kaplama", step)
            .option_with("Sınır", "S", if self.room { "Mahal" } else { "Köşeleri çiz" })
            .option_with("Tip", "P", tip)
            .option_with("Kalınlık", "E", self.thickness.to_string())
            .option_with("Kot farkı", "F", offset)
            .option_with("Malzeme", "M", self.material.clone().unwrap_or_else(|| "Yok".into()));
        if !self.room && !self.pts.is_empty() {
            p = p.option("Geri al", "G");
        }
        p.option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(if self.room { p.raw } else { p.world });
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if !self.room {
            self.accept_point(p.world, cx);
            return;
        }
        self.write(Some(p.raw), None, cx);
        self.see(cx);
    }

    fn accepts_points(&self) -> bool {
        !self.room
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.room {
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
            "S" => {
                self.room = !self.room;
                self.pts.clear();
            }
            "P" => {
                let at = Covering::TYPES.iter().position(|t| *t == self.covering.predefined).unwrap_or(0);
                return self.choose_option("P", Covering::TYPES[(at + 1) % Covering::TYPES.len()], cx);
            }
            "E" => self.ask = Some(Ask::Thickness),
            "F" => self.ask = Some(Ask::Offset),
            "M" => self.ask = Some(Ask::Material),
            "G" if !self.room => return self.undo_step(cx),
            _ => {
                if !self.room
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
            "P" => self.type_choices(),
            _ => Vec::new(),
        }
    }

    fn settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::Setting;
        vec![
            Setting::choice("S", "Sınır", self.bound_choices()),
            Setting::choice("P", "Tip", self.type_choices()),
            Setting::text("E", "Kalınlık", f.length(self.thickness)),
            Setting::text("F", "Kot farkı", self.offset.map_or_else(|| "Oto".to_owned(), |v| f.length(v))),
            Setting::text("M", "Malzeme", self.material.clone().unwrap_or_default()),
            Setting::hint(
                "K",
                "Kaplama",
                "Asma tavan, elle çizilen döşeme kaplaması, çatı kaplaması: döşeme değildir, duvar ve mahal yüksekliği ona \
                 göre değişmez; IFC'de kaplama olarak yazılır. Mahal ile tıklanan mahalin sınırını izler; Kot farkı Oto \
                 iken tavan mahalin tavanında, öbürleri tabanında durur.",
            ),
        ]
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if matches!(key, "S" | "P") {
            return self.choose_option(key, value, cx);
        }
        match key {
            "E" | "F" => {
                let ask = if key == "E" { Ask::Thickness } else { Ask::Offset };
                if !self.set_length(ask, value.trim(), cx) {
                    return false;
                }
            }
            "M" => self.material = (!value.trim().is_empty()).then(|| value.trim().to_owned()),
            _ => return false,
        }
        self.see(cx);
        true
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "S" => {
                let room = match typed {
                    "room" | "Mahal" => true,
                    "draw" | "Köşeleri çiz" => false,
                    _ => return false,
                };
                if room != self.room {
                    self.pts.clear();
                }
                self.room = room;
            }
            "P" => {
                let Some(t) = Covering::TYPES.iter().find(|t| **t == typed) else {
                    return false;
                };
                let was_ceiling = self.ceiling();
                self.covering = covering_of(t);
                // A ceiling's first thickness and another's, until one is typed (docs/adr/8070 §10).
                if self.ceiling() != was_ceiling {
                    self.thickness = if self.ceiling() { CEILING_THICKNESS } else { COVERING_THICKNESS };
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
        if !self.room && self.pts.len() >= 3 {
            let corners = self.pts.clone();
            self.write(None, Some(&corners), cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let title = self.covering.words();
        let tag = self.hover.map(|at| Tag {
            at,
            lines: match (&self.why, self.area) {
                (Some(why), _) => vec![title.clone(), why.clone()],
                (None, Some(a)) => vec![format!("{title} · {} · {}", format.area(a), format.length(self.thickness))],
                _ => vec![title.clone()],
            },
        });
        Preview {
            tag,
            tag_tone: if self.why.is_some() { Tone::Danger } else { Tone::Accent },
            ..self.drawn.clone()
        }
    }
}
