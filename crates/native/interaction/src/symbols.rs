//! Mimari işaretler (docs/adr/0721): Kuzey oku and Malzeme lejandı, each one click through its command
//! (`cad.north.create`, `cad.legends.materials`); the ghost at the cursor is the command's plan: the block it would
//! place (the drawing's, or KentOS's library's it would add) as it would be placed. Revizyon işaretle is Revizyon
//! bulutu with a revision's number (`revcloud.rs`).
//!
//! - **Kuzey oku**: Yön (Y): Plan kuzeyi (the drawing's up), Coğrafi kuzey (the project's meridian at the point) or
//!   Açı (A, degrees counter-clockwise from the plan's north); Boyut (B): its width on paper, mm. The tool leaves once
//!   the arrow is placed.
//! - **Malzeme lejandı**: a click places the legend's top left corner; Yükseklik (H): its text's height on paper, mm;
//!   Yenile (Y), when the drawing has a legend: made again in its place, nothing placed. The tool leaves after it.

use kentos_commands::{
    BlockDefinition, CommandResult, Entity, LegendsMaterials, NorthCreate, NorthDirection, MAX_NORTH_SIZE, NORTH_SIZE,
};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::{ExecutionContext, legends, north};

use crate::Vec2;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tool};
use crate::tool_settings::{Setting, choices};

/// The ghost of an insert of `def`: its pieces, texts aside, placed as `insert` places them.
pub(crate) fn definition_ghost(def: &BlockDefinition, insert: &Entity) -> Vec<Stroke> {
    let Entity::Insert(i) = insert else {
        return Vec::new();
    };
    let mut m = kentos_geometry_core::block::placement_xy(
        kentos_geometry_core::Vec2::new(i.p.x, i.p.y),
        i.scale,
        i.y_scale.unwrap_or(i.scale),
        i.rotation,
        i.mirror,
    );
    // The definition's pieces stand off its base point: the base goes to `p`.
    m[4] -= m[0] * def.base.x + m[2] * def.base.y;
    m[5] -= m[1] * def.base.x + m[3] * def.base.y;
    let similar = kentos_geometry_core::block::uniform(i.scale, i.y_scale);
    let mut paths = Vec::new();
    for e in &def.entities {
        if matches!(e, Entity::Text(_)) {
            continue;
        }
        let placed = kentos_geometry_core::block::place(&kentos_native_application::geometry::shape(e), &m, similar);
        kentos_geometry_core::store::tools::outline_paths(&placed, &mut paths);
    }
    crate::modify::ghosts(&paths).0
}

/// Where a north arrow points, as the tool keeps it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Way {
    Plan,
    True,
    Angle,
}

/// What the tool asks for next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Angle,
    Size,
    Height,
}

/// The north arrow tool's id: its command is `tool.northArrow`.
pub const NORTH_ID: &str = "northArrow";
pub const NORTH_LABEL: &str = "Kuzey oku";

/// Kuzey oku.
#[derive(Clone, Debug)]
pub struct NorthArrow {
    way: Way,
    /// Açı's degrees, counter-clockwise from the plan's north.
    degrees: f64,
    /// Its width on paper, mm.
    size: f64,
    ask: Option<Ask>,
    hover: Option<Vec2>,
    ghost: Vec<Stroke>,
    done: bool,
}

impl Default for NorthArrow {
    fn default() -> Self {
        Self {
            way: Way::Plan,
            degrees: 0.0,
            size: NORTH_SIZE,
            ask: None,
            hover: None,
            ghost: Vec::new(),
            done: false,
        }
    }
}

impl NorthArrow {
    pub fn new() -> Self {
        Self::default()
    }

    fn north(&self) -> NorthDirection {
        match self.way {
            Way::Plan => NorthDirection::Plan,
            Way::True => NorthDirection::True,
            Way::Angle => NorthDirection::Angle {
                angle: self.degrees.to_radians(),
            },
        }
    }

    fn request(&self, at: Vec2, cx: &Context<'_>) -> NorthCreate {
        NorthCreate {
            at: wire(at),
            layer_id: cx.doc.layers().active().to_owned(),
            north: self.north(),
            size: (self.size != NORTH_SIZE).then_some(self.size),
            color: cx.draft.color_text(),
            expected_revision: None,
        }
    }

    /// The ghost at `at`: the command's plan.
    fn see(&mut self, at: Option<Vec2>, cx: &Context<'_>) {
        self.ghost.clear();
        let Some(at) = at else { return };
        if let CommandResult::Completed { output, .. } = north::plan_in(cx.doc, &self.request(at, cx)) {
            let Entity::Insert(i) = &output.insert else { return };
            let def = output.block.or_else(|| cx.doc.block(i.block).cloned());
            if let Some(def) = def {
                self.ghost = definition_ghost(&def, &output.insert);
            }
        }
    }

    fn place(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let input = self.request(at, cx);
        let result = north::execute(&mut ExecutionContext::with_store(cx.doc, cx.spatial.store()), input);
        if let Some(out) = points::written(result, cx) {
            let words = match out.convergence {
                Some(c) => format!(
                    "Kuzey oku yerleşti: coğrafi kuzey, planın kuzeyinden {}° (meridyen yakınsaması).",
                    js_number(fixed_degrees(c))
                ),
                None if out.rotation != 0.0 => format!(
                    "Kuzey oku yerleşti: planın kuzeyinden {}°.",
                    js_number(fixed_degrees(out.rotation))
                ),
                None => "Kuzey oku yerleşti: planın kuzeyi.".to_owned(),
            };
            cx.say(Level::Success, words);
            self.done = true;
        }
    }

    fn way_choices(&self) -> Vec<crate::tool::OptionChoice> {
        let selected = match self.way {
            Way::Plan => "plan",
            Way::True => "coğrafi",
            Way::Angle => "açı",
        };
        choices(&[("plan", "Plan kuzeyi"), ("coğrafi", "Coğrafi kuzey"), ("açı", "Açı")], selected)
    }
}

/// Degrees of `radians`, to four places.
fn fixed_degrees(radians: f64) -> f64 {
    (radians.to_degrees() * 1e4).round() / 1e4
}

impl Tool for NorthArrow {
    fn id(&self) -> &'static str {
        NORTH_ID
    }

    fn label(&self) -> &'static str {
        NORTH_LABEL
    }

    fn prompt(&self) -> Prompt {
        match self.ask {
            Some(Ask::Angle) => Prompt::new(NORTH_LABEL, "kuzeyin açısını derece olarak yazın (saat yönünün tersine)"),
            Some(_) => Prompt::new(NORTH_LABEL, "okun kâğıttaki enini milimetre olarak yazın"),
            None => {
                let way = match self.way {
                    Way::Plan => "Plan kuzeyi".to_owned(),
                    Way::True => "Coğrafi kuzey".to_owned(),
                    Way::Angle => format!("{}°", js_number(self.degrees)),
                };
                Prompt::new(NORTH_LABEL, "okun yerini belirtin")
                    .option_with("Yön", "Y", way)
                    .option("Açı", "A")
                    .option_with("Boyut", "B", format!("{} mm", js_number(self.size)))
            }
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(self.hover, cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.ask.is_none() {
            self.place(p.world, cx);
        }
    }

    fn option_choices(&self, key: &str) -> Vec<crate::tool::OptionChoice> {
        if key == "Y" { self.way_choices() } else { Vec::new() }
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        key == "Y" && self.configure("Y", typed, cx)
    }

    fn settings(&self, _f: &Format) -> Vec<Setting> {
        let mut out = vec![Setting::choice("Y", "Yön", self.way_choices())];
        if self.way == Way::Angle {
            out.push(Setting::text("A", "Açı", format!("{}°", js_number(self.degrees))));
        }
        out.push(Setting::text("B", "Boyut", format!("{} mm", js_number(self.size))));
        out
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let text = js_trim(value).trim_end_matches(['°', ' ']).trim_end_matches("mm").trim();
        match key {
            "Y" => {
                self.way = match js_trim(value) {
                    "plan" => Way::Plan,
                    "coğrafi" => Way::True,
                    "açı" => Way::Angle,
                    _ => return false,
                };
            }
            "A" => {
                let Some(d) = parse_number(text).filter(|d| d.is_finite()) else {
                    cx.say(Level::Warn, "Açıyı derece olarak bir sayıyla yazın.");
                    return false;
                };
                self.way = Way::Angle;
                self.degrees = d;
            }
            "B" => match parse_number(text) {
                Some(s) if s > 0.0 && s <= MAX_NORTH_SIZE => self.size = s,
                _ => {
                    cx.say(Level::Warn, "Boyu 0'dan büyük, en çok 200 mm bir sayıyla yazın.");
                    return false;
                }
            },
            _ => return false,
        }
        self.see(self.hover, cx);
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(ask) = self.ask.take() {
            let key = if ask == Ask::Angle { "A" } else { "B" };
            self.configure(key, text, cx);
            return true;
        }
        match upper_tr(js_trim(text)).as_str() {
            "Y" => {
                // Yön steps through its three choices.
                self.way = match self.way {
                    Way::Plan => Way::True,
                    Way::True => Way::Angle,
                    Way::Angle => Way::Plan,
                };
                self.see(self.hover, cx);
                true
            }
            "A" => {
                self.ask = Some(Ask::Angle);
                true
            }
            "B" => {
                self.ask = Some(Ask::Size);
                true
            }
            _ => match cx.typed_point(text, None, self.hover) {
                Some(p) => {
                    self.place(p, cx);
                    true
                }
                None => false,
            },
        }
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            strokes: self.ghost.clone(),
            ..Preview::default()
        }
    }

    fn finished(&self) -> bool {
        self.done
    }
}

/// The material legend tool's id: its command is `tool.materialLegend`.
pub const LEGEND_ID: &str = "materialLegend";
pub const LEGEND_LABEL: &str = "Malzeme lejandı";

/// Malzeme lejandı.
#[derive(Clone, Debug)]
pub struct MaterialLegend {
    /// Its text's height on paper, mm.
    height: f64,
    ask: Option<Ask>,
    hover: Option<Vec2>,
    ghost: Vec<Stroke>,
    /// The drawing has a legend to make again (Yenile).
    has: bool,
    done: bool,
}

impl Default for MaterialLegend {
    fn default() -> Self {
        Self {
            height: legends::HEIGHT,
            ask: None,
            hover: None,
            ghost: Vec::new(),
            has: false,
            done: false,
        }
    }
}

impl MaterialLegend {
    pub fn new() -> Self {
        Self::default()
    }

    fn request(&self, at: Option<Vec2>, cx: &Context<'_>) -> LegendsMaterials {
        LegendsMaterials {
            at: at.map(wire),
            layer_id: cx.doc.layers().active().to_owned(),
            height: (self.height != legends::HEIGHT).then_some(self.height),
            expected_revision: None,
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.has = cx.doc.block_named(kentos_commands::LEGEND_BLOCK).is_some();
        self.ghost.clear();
        let Some(at) = self.hover else { return };
        if let CommandResult::Completed { output, .. } = legends::plan_in(cx.doc, &self.request(Some(at), cx))
            && let Some(insert) = &output.insert
        {
            self.ghost = definition_ghost(&output.block, insert);
        }
    }

    fn make(&mut self, at: Option<Vec2>, cx: &mut Context<'_>) {
        let input = self.request(at, cx);
        let result = legends::execute(&mut ExecutionContext::with_store(cx.doc, cx.spatial.store()), input);
        if let Some(out) = points::written(result, cx) {
            let n = out.materials.len();
            cx.say(
                Level::Success,
                if out.remade && at.is_none() {
                    format!("Malzeme lejandı yerinde yenilendi: {n} malzeme.")
                } else {
                    format!("Malzeme lejandı yerleşti: {n} malzeme.")
                },
            );
            self.done = true;
        }
    }
}

impl Tool for MaterialLegend {
    fn id(&self) -> &'static str {
        LEGEND_ID
    }

    fn label(&self) -> &'static str {
        LEGEND_LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.ask.is_some() {
            return Prompt::new(LEGEND_LABEL, "yazının kâğıttaki yüksekliğini milimetre olarak yazın");
        }
        let prompt = Prompt::new(LEGEND_LABEL, "lejandın sol üst köşesini belirtin")
            .option_with("Yükseklik", "H", format!("{} mm", js_number(self.height)));
        if self.has { prompt.option("Yenile", "Y") } else { prompt }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.ask.is_none() {
            self.make(Some(p.world), cx);
        }
    }

    fn settings(&self, _f: &Format) -> Vec<Setting> {
        vec![Setting::text("H", "Yükseklik", format!("{} mm", js_number(self.height)))]
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if key != "H" {
            return false;
        }
        let text = js_trim(value).trim_end_matches("mm").trim();
        match parse_number(text) {
            Some(h) if h > 0.0 && h <= legends::MAX_HEIGHT => {
                self.height = h;
                self.see(cx);
                true
            }
            _ => {
                cx.say(Level::Warn, "Yüksekliği 0'dan büyük, en çok 50 mm bir sayıyla yazın.");
                false
            }
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            self.configure("H", text, cx);
            return true;
        }
        match upper_tr(js_trim(text)).as_str() {
            "H" => {
                self.ask = Some(Ask::Height);
                true
            }
            "Y" if self.has => {
                self.make(None, cx);
                true
            }
            _ => match cx.typed_point(text, None, self.hover) {
                Some(p) => {
                    self.make(Some(p), cx);
                    true
                }
                None => false,
            },
        }
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            strokes: self.ghost.clone(),
            ..Preview::default()
        }
    }

    fn finished(&self) -> bool {
        self.done
    }
}
