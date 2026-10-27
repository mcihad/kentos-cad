//! Mahal and Mahali böl (docs/adr/0342 §5, plan/cad.md `CAD-ARCH-23`, `24`, `25`). The tools say what was clicked and
//! chosen; `cad.spaces.create` and `cad.spaces.edit` compute and write, one undo step each, and the plan is the preview.
//!
//! - Mahal: with the pointer inside walls, the space the command would find there is drawn with its area; a click
//!   writes it and the tool stays, its number one more (Z-01, Z-02 …). Yöntem (Y) the method (the project's, Net,
//!   Duvar ekseni, Brüt), Numara (N), Ad (A) and Kullanım (K) typed. Çiz (Ç) draws the boundary instead: clicks give
//!   its corners, Enter closes it and writes; Geri al (G) takes the last corner back.
//! - Mahali böl: a space, then the cut line's two points; the pieces are drawn, the second point writes.

use kentos_commands::{
    CommandResult, Entity, NewSpace, SpaceColumns, SpaceMethod, SpacePart, SpaceRing, SpacesCreate, SpacesEdit,
    SpacesOperation,
};
use kentos_domain::Slot;
use kentos_geometry_core::text::edit::increment;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, spaces, spaces_create, spaces_edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Area, Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};

pub const SPACE_ID: &str = "space";
pub const SPLIT_ID: &str = "spaceSplit";
pub const REBOUND_ID: &str = "spaceRebound";
pub const SOURCES_ID: &str = "spaceSources";

/// The icon of a method in Yöntem's menu.
pub fn method_icon(m: Option<SpaceMethod>) -> &'static str {
    match m {
        None => "spaceMethod",
        Some(SpaceMethod::Net) => "spaceNet",
        Some(SpaceMethod::Centerline) => "spaceCenterline",
        Some(SpaceMethod::Gross) => "spaceGross",
    }
}

/// One set of persistent column policies shared by both room tools.
pub fn column_choices(value: Option<SpaceColumns>) -> Vec<OptionChoice> {
    let value = value.unwrap_or(SpaceColumns::Net);
    SpaceColumns::ALL.iter().map(|&m| OptionChoice { typed:m.name().into(),label:m.label().into(),
        icon:None,preview:None,checked:m==value,command:None }).collect()
}

/// Display rings for the preview fill; analytic source rings remain untouched.
fn rings_of(parts: &[SpacePart]) -> Vec<Vec<Vec2>> {
    let ring = |r: &SpaceRing| {
        let pts: Vec<Vec2> = r.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        kentos_geometry_core::entity::polygon_ring(&pts, r.bulges.as_deref())
    };
    parts
        .iter()
        .flat_map(|p| std::iter::once(ring(&p.outer)).chain(p.holes.iter().map(ring)))
        .collect()
}

/// A space drawn: its outline and its fill.
fn drawn_space(e: &Entity, tone: Tone, out: &mut Preview) {
    let Entity::Space(s) = e else { return };
    out.strokes
        .extend(Outline::of(&shape(e), None, 2.0, tone).strokes);
    // A void's cross (docs/adr/0708), as the drawing shows it.
    if s.space.void {
        use kentos_geometry_core::geom::arrangement::{Area as CoreArea, Ring};
        let ring = |r: &kentos_commands::SpaceRing| Ring {
            pts: r.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect(),
            bulges: r.bulges.clone(),
        };
        let parts: Vec<CoreArea> = s
            .space
            .parts
            .iter()
            .map(|p| CoreArea { outer: ring(&p.outer), holes: p.holes.iter().map(ring).collect() })
            .collect();
        for line in kentos_geometry_core::geom::space::void_cross(&parts) {
            out.strokes.extend(Outline::of(&line, None, 1.5, tone).strokes);
        }
    }
    out.areas.push(Area {
        rings: rings_of(&s.space.parts),
        fill: 0.16,
        width: 0.0,
        dash: None,
        fill_tone: tone,
    });
}

/// What the tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Number,
    Name,
    Usage,
}

/// Mahal.
#[derive(Clone, Debug, Default)]
pub struct SpaceTool {
    /// None: the project's.
    method: Option<SpaceMethod>,
    columns: Option<SpaceColumns>,
    number: Option<String>,
    name: Option<String>,
    usage: Option<String>,
    /// Boşluk (docs/adr/0708): the space written is a void — a shaft, a lift shaft, an opening in the floor.
    void: bool,
    level: crate::space_level::Choice,
    sources: crate::space_sources::Choice,
    /// Çiz: the boundary's corners so far.
    draw: bool,
    pts: Vec<Vec2>,
    ask: Option<Ask>,
    hover: Option<Vec2>,
    drawn: Preview,
    /// The area of the space the plan would write, m²; or why it would not.
    area: Option<f64>,
    why: Option<String>,
    choice_revision: Option<u64>,
    choice_level: Option<String>,
    names: Vec<String>,
    usages: Vec<String>,
    first_from_level: Option<String>,
}

impl SpaceTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn method_label(&self) -> &'static str {
        self.method.map_or("Projenin", SpaceMethod::label)
    }

    fn level_label(&self) -> String {
        self.level.label()
    }

    fn level_choices(&self) -> Vec<OptionChoice> {
        self.level.setting().choices
    }

    fn new_space(&self, seed: Option<Vec2>, corners: Option<&[Vec2]>) -> NewSpace {
        NewSpace {
            sources: if seed.is_some() {self.sources.sources()} else {None},
            seed: seed.map(points::wire),
            scope: Some(if seed.is_some() { self.level.scope() } else { Default::default() }),
            parts: corners.map(|c| {
                vec![SpacePart {
                    outer: SpaceRing {
                        pts: points::wire_all(c),
                        bulges: None,
                    },
                    holes: Vec::new(),
                }]
            }),
            method: self.method,
            columns: Some(if seed.is_some() { self.columns.unwrap_or(SpaceColumns::Net) } else { SpaceColumns::Ignore }),
            number: self.number.clone(),
            name: self.name.clone(),
            usage: self.usage.clone(),
            arch: self.level.arch(),
            void: self.void.then_some(true),
            ..NewSpace::default()
        }
    }

    fn input_for(&self, space: NewSpace, cx: &Context<'_>) -> SpacesCreate {
        SpacesCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            spaces: vec![space],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// The input at the hover: a seed there, or the corners drawn and the pointer.
    fn hovered(&self, cx: &Context<'_>) -> Option<SpacesCreate> {
        let at = self.hover?;
        if self.draw {
            let mut c = self.pts.clone();
            c.push(at);
            (c.len() >= 3).then(|| self.input_for(self.new_space(None, Some(&c)), cx))
        } else {
            Some(self.input_for(self.new_space(Some(at), None), cx))
        }
    }

    /// The plan at the hover, drawn.
    fn see(&mut self, cx: &Context<'_>) {
        self.level.see(cx.doc.settings());
        // Etkin kat while it has walls (docs/adr/0713).
        self.level.follow_where_walls(cx.level, cx.doc);
        let level = self.level.id();
        if self.choice_revision != Some(cx.doc.revision()) || self.choice_level != level {
            let arch = cx.doc.settings().arch.as_ref();
            self.names = arch.map_or_else(Vec::new, |a| a.space_names.clone());
            self.usages = arch.map_or_else(Vec::new, |a| a.space_usages.clone());
            self.first_from_level = level
                .as_deref()
                .and_then(|id| arch.and_then(|a| a.space_first_number(id)));
            self.choice_level = level;
            self.choice_revision = Some(cx.doc.revision());
        }
        self.drawn = Preview::default();
        self.area = None;
        self.why = None;
        if self.draw {
            self.drawn.path = self.pts.clone();
            if let Some(at) = self.hover {
                self.drawn.path.push(at);
            }
        }
        let Some(input) = self.hovered(cx) else {
            return;
        };
        match spaces_create::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for e in &output.spaces {
                    drawn_space(e, Tone::Accent, &mut self.drawn);
                    if let Entity::Space(s) = e {
                        self.area = Some(spaces::area_of(&s.space.parts));
                    }
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.why = Some(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// Written: the number one more, the drawn corners gone.
    fn write(&mut self, input: SpacesCreate, cx: &mut Context<'_>) -> bool {
        let result = spaces_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_none() {
            return false;
        }
        let kind = if self.void { "Boşluk" } else { "Mahal" };
        let what = match (&self.number, &self.name) {
            (Some(n), Some(a)) => format!("{kind} {n} {a} oluşturuldu."),
            (Some(n), None) => format!("{kind} {n} oluşturuldu."),
            (None, Some(a)) => format!("{kind} “{a}” oluşturuldu."),
            (None, None) => format!("{kind} oluşturuldu."),
        };
        cx.say(Level::Success, what);
        if let Some(n) = &self.number {
            self.number = increment(n).or_else(|| Some(n.clone()));
        }
        self.pts.clear();
        true
    }
}

impl Tool for SpaceTool {
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.sources.capture(cx);
        self.see(cx);
        Flow::Stay
    }

    /// Kat follows the new active level at once while it has walls (docs/adr/0720 §4).
    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
    }

    fn id(&self) -> &'static str {
        SPACE_ID
    }

    fn label(&self) -> &'static str {
        "Mahal"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snaps(&self) -> bool {
        self.draw
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
        match self.ask {
            Some(Ask::Number) => {
                return Prompt::new("Mahal", "mahalin numarasını yazın (Z-01 gibi)")
                    .option("Geri", "G");
            }
            Some(Ask::Name) => {
                return Prompt::new("Mahal", "mahalin adını yazın").option("Geri", "G");
            }
            Some(Ask::Usage) => {
                return Prompt::new("Mahal", "mahalin kullanımını yazın").option("Geri", "G");
            }
            None => {}
        }
        let step = if !self.draw {
            "duvarlarla çevrili bir yere tıklayın"
        } else if self.pts.len() < 3 {
            "sınırın köşelerine tıklayın"
        } else {
            "köşelere tıklayın, Enter ile kapatın"
        };
        let mut p = Prompt::new("Mahal", step)
            .option_with("Yöntem", "Y", self.method_label())
            .option_with("Kat", "L", self.level_label())
            .option_with(
                "Numara",
                "N",
                self.number.clone().unwrap_or_else(|| "Yok".into()),
            )
            .option_with("Ad", "A", self.name.clone().unwrap_or_else(|| "Yok".into()))
            .option_with(
                "Kullanım",
                "K",
                self.usage.clone().unwrap_or_else(|| "Yok".into()),
            )
            .toggle("Boşluk", "V", self.void)
            .toggle("Çiz", "Ç", self.draw);
        if self.draw && !self.pts.is_empty() {
            p = p.option("Geri al", "G");
        }
        p.option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(if self.draw { p.world } else { p.raw });
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.draw {
            self.accept_point(p.world, cx);
            return;
        }
        let input = self.input_for(self.new_space(Some(p.raw), None), cx);
        self.write(input, cx);
        self.see(cx);
    }

    fn accepts_points(&self) -> bool {
        self.draw
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.draw {
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
            let value = (!value.is_empty()).then(|| value.to_owned());
            match ask {
                Ask::Number => self.number = value,
                Ask::Name => self.name = value,
                Ask::Usage => self.usage = value,
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "B" if !self.draw => {self.sources.toggle();}
            "Y" => {
                self.method = match self.method {
                    None => Some(SpaceMethod::Net),
                    Some(SpaceMethod::Net) => Some(SpaceMethod::Centerline),
                    Some(SpaceMethod::Centerline) => Some(SpaceMethod::Gross),
                    Some(SpaceMethod::Gross) => None,
                }
            }
            "N" => self.ask = Some(Ask::Number),
            "A" => self.ask = Some(Ask::Name),
            "K" => self.ask = Some(Ask::Usage),
            "V" => self.void = !self.void,
            "L" => self.level.next(),
            "D" if !self.draw => {
                self.columns = Some(match self.columns.unwrap_or(SpaceColumns::Net) {
                    SpaceColumns::Ignore => SpaceColumns::Net, SpaceColumns::Net => SpaceColumns::All, SpaceColumns::All => SpaceColumns::Ignore,
                });
            }
            "Ç" => {
                self.draw = !self.draw;
                self.pts.clear();
            }
            "G" if self.draw => return self.undo_step(cx),
            _ => {
                // A typed corner while drawing.
                if self.draw
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
        if key == "B" && !self.draw {return self.sources.choices();}
        if key == "D" && !self.draw { return column_choices(self.columns); }
        if key == "L" {
            return self.level_choices();
        }
        if key != "Y" {
            return Vec::new();
        }
        std::iter::once((None, "Projenin"))
            .chain(SpaceMethod::ALL.iter().map(|m| (Some(*m), m.label())))
            .map(|(m, label)| OptionChoice {
                typed: label.to_owned(),
                label: label.to_owned(),
                icon: Some(method_icon(m)),
                preview: None,
                checked: m == self.method,
                command: None,
            })
            .collect()
    }

    fn settings(&self, _f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::{Setting, choices};
        let mut rows = vec![
            Setting::choice("Y", "Alan yöntemi", self.option_choices("Y")),
            self.level.setting(),
            if self.draw {
                Setting::text("B", "Kaynaklar", "Çizilmiş sınır").read_only()
            } else {
                self.sources.setting()
            },
            if self.draw {
                Setting::text("D", "Kolon alanı", "Düşme").read_only()
            } else {
                Setting::choice("D", "Kolon alanı", column_choices(self.columns))
            },
            Setting::text("N", "Numara", self.number.clone().unwrap_or_default()),
            Setting::text("A", "Ad", self.name.clone().unwrap_or_default()),
            Setting::text("K", "Kullanım", self.usage.clone().unwrap_or_default()),
            Setting::toggle("V", "Boşluk (şaft)", self.void),
            Setting::choice(
                "Ç",
                "Sınır",
                choices(
                    &[("find", "Duvarlardan bul"), ("draw", "Köşeleri çiz")],
                    if self.draw { "draw" } else { "find" },
                ),
            ),
        ];
        if !self.names.is_empty() {
            rows.push(crate::tool_settings::suggestions(
                "LA",
                "Ad listesi",
                &self.names,
                self.name.as_deref(),
            ));
        }
        if !self.usages.is_empty() {
            rows.push(crate::tool_settings::suggestions(
                "LK",
                "Kullanım listesi",
                &self.usages,
                self.usage.as_deref(),
            ));
        }
        if self.first_from_level.is_some() {
            let mut row = Setting::choice(
                "P",
                "Kat öneki",
                choices(&[("use", "Bu katın başlangıcını al")], ""),
            );
            row.value = "Başlangıç al".into();
            rows.push(row);
        }
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if matches!(key, "Y" | "L" | "D" | "B") {
            return self.choose_option(key, value, cx);
        }
        let text = (!value.trim().is_empty()).then(|| value.trim().to_owned());
        match key {
            "LA" if self.names.iter().any(|s| Some(s) == text.as_ref()) => self.name = text,
            "LK" if self.usages.iter().any(|s| Some(s) == text.as_ref()) => self.usage = text,
            "P" if value == "use" => {
                let Some(first) = self.level.id().as_deref().and_then(|id| {
                    cx.doc
                        .settings()
                        .arch
                        .as_ref()
                        .and_then(|a| a.space_first_number(id))
                }) else {
                    return false;
                };
                self.number = Some(first);
            }
            "N" => self.number = text,
            "A" => self.name = text,
            "K" => self.usage = text,
            "V" if matches!(value, "yes" | "no") => self.void = value == "yes",
            "Ç" if matches!(value, "find" | "draw") => {
                if !self.pts.is_empty() && self.draw != (value == "draw") {
                    cx.say(
                        Level::Warn,
                        "Önce mahal sınırını tamamlayın veya Escape ile taslağı bırakın.",
                    );
                    return false;
                }
                self.draw = value == "draw";
            }
            _ => return false,
        }
        self.see(cx);
        true
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key == "B" {
            if self.draw || !self.sources.choose(typed) { return false; }
            self.see(cx);return true;
        }
        if key == "D" {
            if self.draw { return false; }
            let Some(value) = SpaceColumns::ALL.into_iter().find(|m|m.name()==typed) else { return false; };
            self.columns=Some(value);self.see(cx);return true;
        }
        if key == "L" {
            self.level.see(cx.doc.settings());
            if !self.level.choose(typed) { return false; }
            self.see(cx);
            return true;
        }
        if key != "Y" {
            return false;
        }
        self.method = SpaceMethod::ALL
            .iter()
            .copied()
            .find(|m| m.label() == typed);
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
        if self.draw && self.pts.len() >= 3 {
            let corners = self.pts.clone();
            let input = self.input_for(self.new_space(None, Some(&corners)), cx);
            self.write(input, cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let tag = self.hover.map(|at| Tag {
            at,
            lines: match (&self.why, self.area) {
                (Some(why), _) if !self.draw || self.pts.len() >= 2 => {
                    vec!["Mahal".to_owned(), why.clone()]
                }
                (_, Some(a)) if self.void => vec![format!("Boşluk · {}", format.area(a)), "Alanı mahal toplamına katılmaz".to_owned()],
                (_, Some(a)) => vec![format!("Mahal · {}", format.area(a))],
                _ => vec!["Mahal".to_owned()],
            },
        });
        Preview {
            tag,
            tag_tone: if self.why.is_some() && self.area.is_none() && !self.draw {
                Tone::Danger
            } else {
                Tone::Accent
            },
            ..self.drawn.clone()
        }
    }
}

/// Mahali böl.
#[derive(Clone, Debug, Default)]
pub struct SpaceSplitTool {
    space: Option<Slot>,
    first: Option<Vec2>,
    hover: Option<Vec2>,
    drawn: Preview,
    why: Option<String>,
}

impl SpaceSplitTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The space under the point: the innermost whose rings hold it.
    fn space_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        let mut best: Option<(f64, Slot)> = None;
        for e in cx.doc.entities() {
            let Entity::Space(s) = e else { continue };
            if !cx.doc.layers().is_visible(&e.base().layer_id) {
                continue;
            }
            let rings = rings_of(&s.space.parts);
            if inside(&rings, at) {
                let a = spaces::area_of(&s.space.parts);
                if best.is_none_or(|(b, _)| a < b) {
                    best = Some((a, Slot(e.base().id)));
                }
            }
        }
        best.map(|(_, s)| s)
    }

    fn edit_input(&self, a: Vec2, b: Vec2, cx: &Context<'_>) -> Option<SpacesEdit> {
        let uid = cx.doc.uid(self.space?)?.to_string();
        Some(SpacesEdit {
            uids: vec![uid],
            operation: SpacesOperation::Split {
                line: vec![points::wire(a), points::wire(b)],
            },
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        let (Some(a), Some(b)) = (self.first, self.hover) else {
            if let Some(Entity::Space(_)) = self.space.and_then(|s| cx.doc.get(s)) {
                let e = cx.doc.get(self.space.unwrap_or(Slot(0))).cloned();
                if let Some(e) = e {
                    self.drawn
                        .strokes
                        .extend(Outline::of(&shape(&e), None, 2.0, Tone::Accent).strokes);
                }
            }
            return;
        };
        self.drawn.guides.push([a, b]);
        let Some(input) = self.edit_input(a, b, cx) else {
            return;
        };
        match spaces_edit::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for e in &output.spaces {
                    drawn_space(e, Tone::Accent, &mut self.drawn);
                }
                for e in &output.added {
                    drawn_space(e, Tone::Snap, &mut self.drawn);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.why = Some(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }
}

/// Whether the point is inside the rings by the even-odd rule.
fn inside(rings: &[Vec<Vec2>], p: Vec2) -> bool {
    let mut odd = false;
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                odd = !odd;
            }
        }
    }
    odd
}

impl Tool for SpaceSplitTool {
    fn id(&self) -> &'static str {
        SPLIT_ID
    }

    fn label(&self) -> &'static str {
        "Mahali böl"
    }

    fn cursor(&self) -> Cursor {
        if self.space.is_none() {
            Cursor::Pick
        } else {
            Cursor::Cross
        }
    }

    fn snaps(&self) -> bool {
        self.space.is_some()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.first
    }

    fn point_count(&self) -> usize {
        usize::from(self.first.is_some())
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.first.take().is_some() || self.space.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        let step = match (self.space, self.first) {
            (None, _) => "bölünecek mahalin içine tıklayın",
            (Some(_), None) => "bölme çizgisinin ilk noktasını verin",
            (Some(_), Some(_)) => "bölme çizgisinin ikinci noktasını verin",
        };
        Prompt::new("Mahali böl", step).option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.space.is_none() {
            let hit = Self::space_at(p.raw, cx);
            cx.selection.set_hover(hit);
            self.hover = Some(p.raw);
            self.drawn = Preview::default();
            return;
        }
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.space.is_none() {
            match Self::space_at(p.raw, cx) {
                Some(s) => self.space = Some(s),
                None => cx.say(Level::Warn, "Mahali böl: bir mahalin içine tıklayın."),
            }
            self.see(cx);
            return;
        }
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        self.space.is_some()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.space.is_none() {
            return false;
        }
        let Some(a) = self.first else {
            self.first = Some(p);
            self.see(cx);
            return true;
        };
        let Some(input) = self.edit_input(a, p, cx) else {
            return true;
        };
        let result = spaces_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            cx.say(
                Level::Success,
                format!("Mahal {} parçaya bölündü.", 1 + out.added.len()),
            );
            self.space = None;
            self.first = None;
        }
        self.see(cx);
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.space.is_none() {
            return false;
        }
        match cx.typed_point(text, self.first, self.hover) {
            Some(p) => self.accept_point(p, cx),
            None => false,
        }
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let tag = match (&self.why, self.first, self.hover) {
            (Some(why), Some(_), Some(at)) => Some(Tag {
                at,
                lines: vec!["Mahali böl".to_owned(), why.clone()],
            }),
            _ => None,
        };
        Preview {
            tag,
            tag_tone: Tone::Danger,
            ..self.drawn.clone()
        }
    }
}

/// Pick a safe new source point without replacing the room or its metadata.
#[derive(Default)]
pub struct SpaceReboundTool {
    space: Option<Slot>,
    hover: Option<Vec2>,
    drawn: Preview,
    why: Option<String>,
}
impl SpaceReboundTool {
    fn request(&self, at: Vec2, cx: &Context<'_>) -> Option<SpacesEdit> {
        Some(SpacesEdit {uids:vec![cx.doc.uid(self.space?)?.to_string()],
            operation:SpacesOperation::Rebound {seed:Some(points::wire(at))},expected_revision:None})
    }
    fn see(&mut self, at: Vec2, cx: &Context<'_>) {
        self.hover=Some(at); self.drawn=Preview::default(); self.why=None;
        let Some(Entity::Space(s))=self.space.and_then(|slot|cx.doc.get(slot)) else{return};
        let mut source=s.space.clone(); source.seed=Some(points::wire(at));
        if !kentos_domain::space_follow::SpaceSearch::new(cx.doc).seed_is_room(&source) {
            self.why=Some("İç nokta duvarın üzerinde değil, kullanılabilir odanın içinde olmalı.".into()); return;
        }
        let Some(input)=self.request(at,cx) else{return};
        match spaces_edit::plan_in(cx.doc,&input) {
            CommandResult::Completed {output,..} => {
                for e in output.spaces {drawn_space(&e,Tone::Accent,&mut self.drawn);}
            }
            CommandResult::Failed {error} | CommandResult::Conflict {error} |
                CommandResult::NeedsInput {error} => self.why=Some(error.message),
            _=>{},
        }
    }
    fn write(&mut self, at: Vec2, cx: &mut Context<'_>) -> bool {
        self.see(at,cx);
        if let Some(why)=&self.why {cx.say(Level::Warn,why);return false;}
        let Some(input)=self.request(at,cx) else{return false};
        if points::written(spaces_edit::execute(&mut ExecutionContext::new(cx.doc),input),cx).is_none() {return false;}
        self.space=None; self.hover=None; self.drawn=Preview::default();
        cx.say(Level::Success,"Mahalın iç noktası ve sınırı güncellendi."); true
    }
}
impl Tool for SpaceReboundTool {
    fn id(&self)->&'static str {REBOUND_ID}
    fn label(&self)->&'static str {"İç nokta seç"}
    fn activate(&mut self,cx:&mut Context<'_>)->Flow {
        if let [slot]=cx.selection.ids() && matches!(cx.doc.get(*slot),Some(Entity::Space(_))) {
            self.space=Some(*slot);
        }
        Flow::Stay
    }
    fn cursor(&self)->Cursor {if self.space.is_some(){Cursor::Cross}else{Cursor::Pick}}
    fn snaps(&self)->bool {false}
    fn point_count(&self)->usize {usize::from(self.space.is_some())}
    fn undo_step(&mut self,_:&mut Context<'_>)->bool {
        let had=self.space.take().is_some();
        self.hover=None;self.drawn=Preview::default();self.why=None;had
    }
    fn accepts_points(&self)->bool {self.space.is_some()}
    fn prompt(&self)->Prompt {
        Prompt::new(self.label(),if self.space.is_some(){"yeni oda iç noktasına tıklayın; önizleme yazmaz"}
            else{"sınırı yeniden bulunacak mahalin içine tıklayın"}).option("Bitir","Enter")
    }
    fn pointer_move(&mut self,p:&Pointer,cx:&mut Context<'_>) {
        if self.space.is_some(){self.see(p.raw,cx);}
        else {cx.selection.set_hover(SpaceSplitTool::space_at(p.raw,cx));}
    }
    fn pointer_down(&mut self,p:&Pointer,cx:&mut Context<'_>) {
        if self.space.is_some(){self.write(p.raw,cx);}
        else {self.space=SpaceSplitTool::space_at(p.raw,cx);self.hover=None;}
    }
    fn accept_point(&mut self,p:Vec2,cx:&mut Context<'_>)->bool {self.write(p,cx)}
    fn input(&mut self,text:&str,cx:&mut Context<'_>)->bool {
        let Some(p)=cx.typed_point(text,None,self.hover) else{return false};
        self.write(p,cx)
    }
    fn confirm(&mut self,cx:&mut Context<'_>)->Flow {
        if self.space.is_none(){return Flow::Exit;}
        if let Some(at)=self.hover && self.write(at,cx){Flow::Exit}else{Flow::Stay}
    }
    fn preview(&self,_:&Format)->Preview {
        let tag=self.hover.map(|at|Tag {at,lines:vec![self.why.clone().unwrap_or_else(||"İç nokta seç · tıklama uygular · Esc vazgeçer".into())]});
        Preview {tag,tag_tone:if self.why.is_some(){Tone::Danger}else{Tone::Accent},..self.drawn.clone()}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_is_inside_a_ring_and_not_in_its_hole() {
        let sq = |a: f64, b: f64| {
            vec![
                Vec2::new(a, a),
                Vec2::new(b, a),
                Vec2::new(b, b),
                Vec2::new(a, b),
            ]
        };
        let rings = vec![sq(0.0, 10.0), sq(4.0, 6.0)];
        assert!(inside(&rings, Vec2::new(1.0, 1.0)));
        assert!(!inside(&rings, Vec2::new(5.0, 5.0)));
        assert!(!inside(&rings, Vec2::new(11.0, 5.0)));
    }
}

/// Review and replace the persistent UUID source set of one existing room.
#[derive(Default)]
pub struct SpaceSourcesTool {
    history: Vec<(Vec<String>,bool)>,
    revision: Option<u64>,
    space: Option<Slot>,
    ids: Vec<String>,
    all: bool,
    drawn: Preview,
    why: Option<String>,
}
impl SpaceSourcesTool {
    fn target(&mut self, slot:Slot, cx:&Context<'_>) {
        let Some(Entity::Space(s))=cx.doc.get(slot) else{return};
        self.history.clear();
        self.space=Some(slot);self.all=s.space.sources.is_none();
        self.ids=s.space.sources.clone().unwrap_or_default();self.see(cx);
    }
    fn request(&self,cx:&Context<'_>)->Option<SpacesEdit> {
        Some(SpacesEdit {uids:vec![cx.doc.uid(self.space?)?.to_string()],
            operation:SpacesOperation::Set(kentos_commands::SpaceChange {
                sources:Some((!self.all).then(||self.ids.clone())),..Default::default()}),expected_revision:None})
    }
    fn see(&mut self,cx:&Context<'_>) {
        self.revision=Some(cx.doc.revision());
        self.drawn=Preview::default();self.why=None;
        let Some(input)=self.request(cx) else{return};
        match spaces_edit::plan_in(cx.doc,&input) {
            CommandResult::Completed {output,..}=>for e in output.spaces {drawn_space(&e,Tone::Accent,&mut self.drawn)},
            CommandResult::Failed {error}|CommandResult::Conflict {error}|CommandResult::NeedsInput {error}=>self.why=Some(error.message),
            _=>{},
        }
        for id in self.ids.iter().filter(|_| !self.all) {
            if let Ok(uid)=kentos_domain::Uuid::parse_str(id) && let Some(slot)=cx.doc.slot_of(uid)
                && let Some(e)=cx.doc.get(slot) {
                self.drawn.strokes.extend(Outline::of(&shape(e),Some([4.,3.]),2.,Tone::Snap).strokes);
            }
        }
    }
}
impl Tool for SpaceSourcesTool {
    fn id(&self)->&'static str {SOURCES_ID}
    fn label(&self)->&'static str {"Mahal kaynakları"}
    fn cursor(&self)->Cursor {Cursor::Pick}
    fn snaps(&self)->bool {false}
    fn activate(&mut self,cx:&mut Context<'_>)->Flow {
        if let [slot]=cx.selection.ids() && matches!(cx.doc.get(*slot),Some(Entity::Space(_))) {self.target(*slot,cx);}
        Flow::Stay
    }
    fn point_count(&self)->usize {usize::from(self.space.is_some())}
    fn prompt(&self)->Prompt {
        Prompt::new(self.label(),if self.space.is_some(){"duvar, kolon veya mahal ayırıcıya tıklayın; Enter uygular"}
            else{"kaynakları değişecek mahalin içine tıklayın"}).option("Kaynaklar","B").option("Uygula","Enter")
    }
    fn pointer_move(&mut self,p:&Pointer,cx:&mut Context<'_>) {
        if self.revision!=Some(cx.doc.revision()){self.see(cx);}
        let hit=if self.space.is_none(){SpaceSplitTool::space_at(p.raw,cx)}else{
            cx.spatial.hits(p.raw,cx.pick_tolerance()).into_iter().find(|&slot|
                cx.doc.get(slot).is_some_and(kentos_domain::space_follow::is_source))
        };
        cx.selection.set_hover(hit);
    }
    fn undo_step(&mut self,cx:&mut Context<'_>)->bool {
        if let Some((ids,all))=self.history.pop(){self.ids=ids;self.all=all;self.see(cx);true}else{false}
    }
    fn pointer_down(&mut self,p:&Pointer,cx:&mut Context<'_>) {
        if self.space.is_none() {if let Some(slot)=SpaceSplitTool::space_at(p.raw,cx){self.target(slot,cx);}return;}
        let hit=cx.spatial.hits(p.raw,cx.pick_tolerance()).into_iter().find(|&slot|
            cx.doc.get(slot).is_some_and(kentos_domain::space_follow::is_source));
        let Some(id)=hit.and_then(|slot|cx.doc.uid(slot)).map(|id|id.to_string()) else{
            cx.say(Level::Info,"Kaynak olarak duvar, kolon veya mahal ayırıcı seçin.");return;
        };
        self.history.push((self.ids.clone(),self.all));
        if self.all {self.all=false;self.ids.clear();}
        if let Some(i)=self.ids.iter().position(|u|u==&id){self.ids.remove(i);}else{self.ids.push(id);self.ids.sort();}
        self.see(cx);
    }
    fn settings(&self,_:&Format)->Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::{Setting,choices};
        let mut rows=vec![Setting::choice("B","Kaynaklar",choices(&[("all","Tüm uygun kaynaklar"),("selected","Seçilenler")],if self.all{"all"}else{"selected"})),
            Setting::text("N","Seçilen",self.ids.len().to_string()).read_only()];
        if let Some(why)=&self.why {rows.push(Setting::notice("E","Durum",why));}rows
    }
    fn configure(&mut self,key:&str,value:&str,cx:&mut Context<'_>)->bool {
        if key!="B" || !matches!(value,"all"|"selected"){return false;}
        if self.all != (value=="all") {self.history.push((self.ids.clone(),self.all));}
        self.all=value=="all";self.see(cx);true
    }
    fn input(&mut self,text:&str,cx:&mut Context<'_>)->bool {
        if upper_tr(text.trim())!="B"{return false;}
        self.history.push((self.ids.clone(),self.all));
        self.all = !self.all;self.see(cx);true
    }
    fn confirm(&mut self,cx:&mut Context<'_>)->Flow {
        self.see(cx);
        if let Some(why)=&self.why {cx.say(Level::Warn,why);return Flow::Stay;}
        let Some(input)=self.request(cx) else{cx.say(Level::Info,"Önce mahal seçin.");return Flow::Stay;};
        if points::written(spaces_edit::execute(&mut ExecutionContext::new(cx.doc),input),cx).is_some(){Flow::Exit}else{Flow::Stay}
    }
    fn preview(&self,_:&Format)->Preview {self.drawn.clone()}
}
