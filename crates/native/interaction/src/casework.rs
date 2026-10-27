//! Mutfak dizisi (docs/adr/0706): a straight kitchen run along the line its cabinets' backs stand on. Its start, its
//! end (or its length typed), then the side the cabinets stand on, where the pointer is (Enter takes the side shown).
//! Modül (M), Derinlik (D), Evye (E), Ocak (O), Üst dolap (U) and Yan (Y) are also in the tool's settings pane.
//! `cad.casework.run` computes and writes the run in one undo step; its plan, the cabinets drawn from their blocks
//! with their widths, is the preview.
//!
//! Evye and Ocak start Otomatik: the sink in the second module of a run of two or more, the cooktop the second from
//! the end, or the last, at least one module from the sink (a run of three or more); the module chosen is shown in
//! the settings and the tag, never written unseen.

use kentos_commands::{
    CASEWORK_SINK_MODULE, CaseworkModuleKind, CaseworkRun, CaseworkSide, CommandResult,
    DEFAULT_CASEWORK_DEPTH, DEFAULT_CASEWORK_MODULE,
};
use kentos_geometry_core::geom::casework::module_count;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::{ExecutionContext, casework_run};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::{Setting, choices};

pub const ID: &str = "casework";
pub const LABEL: &str = "Mutfak dizisi";

/// The cabinet block's widths (its width parameter's rule: 0.30 to 1.20 m by 0.15).
const WIDTHS: [f64; 7] = [0.30, 0.45, 0.60, 0.75, 0.90, 1.05, 1.20];

/// Where the sink or the cooktop goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spot {
    Auto,
    None,
    At(u32),
}

/// The value a typed answer asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Module,
    Depth,
    Sink,
    Cooktop,
}

#[derive(Clone, Debug)]
pub struct Casework {
    from: Option<Vec2>,
    to: Option<Vec2>,
    hover: Option<Vec2>,
    /// Yan: none takes the pointer's side once the end is chosen.
    side: Option<CaseworkSide>,
    /// Modül, Derinlik and Üst dolap: the session's memory's, kept here for the settings pane.
    module: f64,
    depth: f64,
    upper: bool,
    sink: Spot,
    cooktop: Spot,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
    /// The planned run: its modules' kinds and widths, its filler, its line's length and its depth.
    planned: Option<Planned>,
}

#[derive(Clone, Debug)]
struct Planned {
    modules: Vec<(CaseworkModuleKind, f64)>,
    filler: f64,
    length: f64,
    depth: f64,
}

impl Default for Casework {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            hover: None,
            side: None,
            module: DEFAULT_CASEWORK_MODULE,
            depth: DEFAULT_CASEWORK_DEPTH,
            upper: false,
            sink: Spot::Auto,
            cooktop: Spot::Auto,
            ask: None,
            drawn: Preview::default(),
            why: None,
            planned: None,
        }
    }
}

/// A length as the drawing's unit says it, in centimetres for a module's width.
fn centimetres(v: f64) -> String {
    format!("{} cm", crate::fixed(v * 100.0, 0))
}

/// The sink's and cooktop's modules for a run `length` long: the auto ones resolved (see the module's words).
fn spots(sink: Spot, cooktop: Spot, module: f64, length: f64) -> (Option<u32>, Option<u32>) {
    let with_sink = module_count(length, module, Some(CASEWORK_SINK_MODULE));
    let without = module_count(length, module, None);
    let sink = match sink {
        Spot::None => None,
        Spot::At(k) => Some(k),
        Spot::Auto => (with_sink >= 2).then_some(2),
    };
    let count = if sink.is_some() { with_sink } else { without } as u32;
    let cooktop = match cooktop {
        Spot::None => None,
        Spot::At(k) => Some(k),
        Spot::Auto if count >= 3 => {
            let mut order = vec![count - 1, count];
            order.extend((1..count - 1).rev());
            order
                .into_iter()
                .find(|&k| sink.is_none_or(|s| k.abs_diff(s) >= 2))
        }
        Spot::Auto => None,
    };
    (sink, cooktop)
}

impl Casework {
    pub fn new() -> Self {
        Self::default()
    }

    /// The run's end now: the chosen end, else the pointer.
    fn end(&self) -> Option<Vec2> {
        self.to.or(self.hover)
    }

    /// The side the cabinets stand on: Yan's, else the pointer's once the end is chosen, else the left.
    fn side_at(&self, at: Option<Vec2>) -> CaseworkSide {
        if let Some(side) = self.side {
            return side;
        }
        match (self.from, self.to, at) {
            (Some(a), Some(b), Some(p)) => {
                let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
                if cross < 0.0 {
                    CaseworkSide::Right
                } else {
                    CaseworkSide::Left
                }
            }
            _ => CaseworkSide::Left,
        }
    }

    fn request(&self, from: Vec2, to: Vec2, side: CaseworkSide, cx: &Context<'_>) -> CaseworkRun {
        let module = self.module;
        let length = (to.x - from.x).hypot(to.y - from.y);
        let (sink, cooktop) = spots(self.sink, self.cooktop, module, length);
        CaseworkRun {
            layer_id: cx.doc.layers().active().to_owned(),
            from: points::wire(from),
            to: points::wire(to),
            side: (side == CaseworkSide::Right).then_some(side),
            module: (module != DEFAULT_CASEWORK_MODULE).then_some(module),
            depth: (self.depth != DEFAULT_CASEWORK_DEPTH).then_some(self.depth),
            sink,
            cooktop,
            upper: self.upper.then_some(true),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    fn see(&mut self, cx: &mut Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        self.planned = None;
        let (Some(from), Some(to)) = (self.from, self.end()) else {
            return;
        };
        if from == to {
            return;
        }
        let input = self.request(from, to, self.side_at(self.hover), cx);
        match casework_run::drawn_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for (shape, dashed) in &output.shapes {
                    self.drawn.strokes.extend(
                        Outline::of(shape, dashed.then_some([5.0, 4.0]), 1.5, Tone::Accent).strokes,
                    );
                }
                self.planned = Some(Planned {
                    modules: output.plan.modules.iter().map(|m| (m.kind, m.width)).collect(),
                    filler: output.plan.filler,
                    length: (to.x - from.x).hypot(to.y - from.y),
                    depth: self.depth,
                });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.why = Some(error.message),
            _ => self.why = Some("Önizleme hazır değil; aracı yeniden başlatın.".into()),
        }
    }

    /// The planned run in words: its modules, its sink and cooktop, its filler, its countertop.
    fn design_words(&self, f: &Format) -> Vec<String> {
        let Some(p) = &self.planned else {
            return Vec::new();
        };
        let base: Vec<f64> = p
            .modules
            .iter()
            .filter(|(kind, _)| *kind != CaseworkModuleKind::Sink)
            .map(|&(_, w)| w)
            .collect();
        let mut first = format!("{} modül", p.modules.len());
        if let Some(w) = base.first() {
            first.push_str(&format!(" · {}", centimetres(*w)));
        }
        let mut rows = vec![first];
        let at = |kind: CaseworkModuleKind| {
            p.modules
                .iter()
                .position(|(k, _)| *k == kind)
                .map(|i| i + 1)
        };
        let mut parts = Vec::new();
        if let Some(k) = at(CaseworkModuleKind::Sink) {
            parts.push(format!("evye {k}. modül"));
        }
        if let Some(k) = at(CaseworkModuleKind::Cooktop) {
            parts.push(format!("ocak {k}. modül"));
        }
        if !parts.is_empty() {
            rows.push(parts.join(" · "));
        }
        let mut last = format!("Tezgâh {} × {}", f.length(p.length), f.length(p.depth));
        if p.filler > 0.0 {
            last.push_str(&format!(" · dolgu {}", f.length(p.filler)));
        }
        rows.push(last);
        rows
    }

    /// Writes the run with its end chosen, on `side`.
    fn write(&mut self, side: CaseworkSide, cx: &mut Context<'_>) -> bool {
        let (Some(from), Some(to)) = (self.from, self.to) else {
            return false;
        };
        let words = self.design_words(&cx.format());
        let input = self.request(from, to, side, cx);
        let result = casework_run::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(out) = points::written(result, cx) else {
            self.see(cx);
            return false;
        };
        let mut said = format!("Mutfak dizisi çizildi: {}.", words.join("; "));
        if !out.blocks.is_empty() {
            said.push_str(&format!(
                " KentOS kitaplığından getirilen bloklar: {}.",
                out.blocks.join(", ")
            ));
        }
        cx.say(Level::Success, said);
        self.from = None;
        self.to = None;
        self.see(cx);
        true
    }

    /// The choices of Evye (`E`) or Ocak (`O`): Otomatik with the module it takes now, Yok, and the run's modules.
    fn spot_choices(&self, sink: bool) -> Vec<OptionChoice> {
        let spot = if sink { self.sink } else { self.cooktop };
        let kind = if sink {
            CaseworkModuleKind::Sink
        } else {
            CaseworkModuleKind::Cooktop
        };
        let now = self.planned.as_ref().and_then(|p| {
            p.modules
                .iter()
                .position(|(k, _)| *k == kind)
                .map(|i| i + 1)
        });
        let count = self.planned.as_ref().map_or(0, |p| p.modules.len());
        let auto = match (spot, now) {
            (Spot::Auto, Some(k)) => format!("Otomatik ({k}. modül)"),
            (Spot::Auto, None) if self.planned.is_some() => "Otomatik (dizi kısa: yok)".into(),
            _ => "Otomatik".into(),
        };
        let mut out = vec![
            OptionChoice {
                typed: "auto".into(),
                label: auto,
                checked: spot == Spot::Auto,
                icon: None,
                preview: None,
                command: None,
            },
            OptionChoice {
                typed: "none".into(),
                label: "Yok".into(),
                checked: spot == Spot::None,
                icon: None,
                preview: None,
                command: None,
            },
        ];
        let mut top = count as u32;
        if let Spot::At(k) = spot {
            top = top.max(k);
        }
        out.extend((1..=top).map(|k| OptionChoice {
            typed: k.to_string(),
            label: format!("{k}. modül"),
            checked: spot == Spot::At(k),
            icon: None,
            preview: None,
            command: None,
        }));
        out
    }

    /// Applies one value: false when it does not read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let words = upper_tr(js_trim(value));
        let spot = |words: &str| -> Option<Spot> {
            match words {
                "AUTO" | "OTOMATİK" | "OTO" => Some(Spot::Auto),
                "NONE" | "YOK" | "0" => Some(Spot::None),
                w => parse_number(w)
                    .filter(|v| *v >= 1.0 && *v <= 100.0 && v.fract() == 0.0)
                    .map(|v| Spot::At(v as u32)),
            }
        };
        match key {
            "M" => match cx.typed_length(value).filter(|v| v.is_finite()) {
                Some(v) => match WIDTHS.iter().copied().find(|w| (w - v).abs() <= 1e-9) {
                    Some(w) => {
                        self.module = w;
                        cx.memory.casework_module = w;
                        true
                    }
                    None => false,
                },
                _ => false,
            },
            "D" => match cx.typed_length(value).filter(|v| v.is_finite()) {
                Some(v) if (0.30 - 1e-9..=1.20 + 1e-9).contains(&v) => {
                    self.depth = v;
                    cx.memory.casework_depth = v;
                    true
                }
                _ => false,
            },
            "E" => spot(&words).map(|s| self.sink = s).is_some(),
            "O" => spot(&words).map(|s| self.cooktop = s).is_some(),
            "U" => match words.as_str() {
                "YES" | "AÇIK" | "EVET" => {
                    self.upper = true;
                    cx.memory.casework_upper = true;
                    true
                }
                "NO" | "KAPALI" | "HAYIR" => {
                    self.upper = false;
                    cx.memory.casework_upper = false;
                    true
                }
                _ => false,
            },
            "Y" => match words.as_str() {
                "CURSOR" | "İMLEÇ" | "" => {
                    self.side = None;
                    true
                }
                "LEFT" | "SOL" => {
                    self.side = Some(CaseworkSide::Left);
                    true
                }
                "RIGHT" | "SAĞ" => {
                    self.side = Some(CaseworkSide::Right);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

impl Tool for Casework {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.module = cx.memory.casework_module;
        self.depth = cx.memory.casework_depth;
        self.upper = cx.memory.casework_upper;
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        if self.to.is_none() { self.from } else { None }
    }

    fn snaps(&self) -> bool {
        self.to.is_none()
    }

    fn point_count(&self) -> usize {
        usize::from(self.from.is_some()) + usize::from(self.to.is_some())
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.to.take().is_some() || self.from.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        if let Some(ask) = self.ask {
            let words = match ask {
                Ask::Module => "modül enini yazın (30–120 cm, 15 cm adımla)",
                Ask::Depth => "tezgâh derinliğini yazın (30–120 cm)",
                Ask::Sink => "evyenin modülünü yazın (Yok, Otomatik ya da sıra numarası)",
                Ask::Cooktop => "ocağın modülünü yazın (Yok, Otomatik ya da sıra numarası)",
            };
            return Prompt::new(LABEL, words).option("Geri", "G");
        }
        let step = match (self.from, self.to) {
            (None, _) => "dizinin başına tıklayın (dolapların sırtı bu çizgide)",
            (Some(_), None) => "dizinin sonuna tıklayın ya da boyunu yazın",
            _ => "dolapların duracağı tarafa tıklayın ya da Enter ile yazın",
        };
        let spot = |s: Spot| match s {
            Spot::Auto => "Otomatik".to_owned(),
            Spot::None => "Yok".to_owned(),
            Spot::At(k) => format!("{k}. modül"),
        };
        Prompt::new(LABEL, step)
            .option("Modül", "M")
            .option("Derinlik", "D")
            .option_with("Evye", "E", spot(self.sink))
            .option_with("Ocak", "O", spot(self.cooktop))
            .option("Üst dolap", "U")
            .option_with(
                "Yan",
                "Y",
                match self.side {
                    None => "İmleç tarafı",
                    Some(CaseworkSide::Left) => "Sol",
                    Some(CaseworkSide::Right) => "Sağ",
                },
            )
            .option("Yaz", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.to.is_some() {
            let side = self.side_at(Some(p.world));
            self.write(side, cx);
            return;
        }
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        self.to.is_none()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.from.is_none() {
            self.from = Some(p);
        } else if self.from != Some(p) {
            self.to = Some(p);
        } else {
            cx.say(Level::Info, "Dizinin sonu başıyla aynı nokta; başka bir noktaya tıklayın.");
        }
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
            let field = match ask {
                Ask::Module => "M",
                Ask::Depth => "D",
                Ask::Sink => "E",
                Ask::Cooktop => "O",
            };
            if !self.configure(field, text, cx) {
                return true;
            }
            self.ask = None;
            return true;
        }
        self.ask = match key.as_str() {
            "M" => Some(Ask::Module),
            "D" => Some(Ask::Depth),
            "E" => Some(Ask::Sink),
            "O" => Some(Ask::Cooktop),
            "U" => {
                self.upper = !self.upper;
                cx.memory.casework_upper = self.upper;
                self.see(cx);
                return true;
            }
            "Y" => {
                self.side = match self.side {
                    None => Some(CaseworkSide::Left),
                    Some(CaseworkSide::Left) => Some(CaseworkSide::Right),
                    Some(CaseworkSide::Right) => None,
                };
                self.see(cx);
                return true;
            }
            _ => {
                if self.to.is_none()
                    && let Some(p) = cx.typed_point(text, self.from, self.hover)
                {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        };
        true
    }

    fn settings(&self, f: &Format) -> Vec<Setting> {
        let module_choices: Vec<OptionChoice> = WIDTHS
            .iter()
            .map(|&w| OptionChoice {
                typed: centimetres(w),
                label: centimetres(w),
                checked: (self.module - w).abs() <= 1e-9,
                icon: None,
                preview: None,
                command: None,
            })
            .collect();
        let mut rows = vec![
            Setting::choice("M", "Modül eni", module_choices),
            Setting::text("D", "Tezgâh derinliği", f.length(self.depth)),
            Setting::choice("E", "Evye", self.spot_choices(true)),
            Setting::choice("O", "Ocak", self.spot_choices(false)),
            Setting::toggle("U", "Üst dolap", self.upper),
            Setting::choice(
                "Y",
                "Yan",
                choices(
                    &[
                        ("cursor", "İmleç tarafı"),
                        ("left", "Çizginin solu"),
                        ("right", "Çizginin sağı"),
                    ],
                    match self.side {
                        None => "cursor",
                        Some(CaseworkSide::Left) => "left",
                        Some(CaseworkSide::Right) => "right",
                    },
                ),
            ),
        ];
        let words = self.design_words(f);
        if !words.is_empty() {
            rows.push(Setting::hint("T", "Dizi", words.join("; ")));
        }
        rows.push(Setting::hint(
            "P",
            "Bloklar",
            "Alt dolap, Evye (90 cm modül), Ocak ve Üst dolap çizimde bu adlarla varsa onlar, yoksa KentOS \
             kitaplığınınkiler aynı adımda getirilir. Ölçüler KentOS'un değerleridir.",
        ));
        if let Some(why) = &self.why {
            rows.push(Setting::notice("W", "Yazılamıyor", why.clone()));
        }
        rows
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.settings(&Format::default())
            .into_iter()
            .find(|s| s.key == key)
            .map_or_else(Vec::new, |s| s.choices)
    }

    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if self.apply(key, value, cx) {
            self.ask = None;
            self.see(cx);
            return true;
        }
        cx.say(
            Level::Warn,
            match key {
                "M" => "Modül eni 30, 45, 60, 75, 90, 105 ya da 120 cm olmalı.",
                "D" => "Tezgâh derinliği 30 ile 120 cm arasında olmalı.",
                "E" | "O" => "Yok, Otomatik ya da modülün sıra numarasını (1, 2 …) yazın.",
                _ => "Geçerli bir değer seçin.",
            },
        );
        false
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            return true;
        }
        if self.to.take().is_some() || self.from.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.ask.take().is_some() {
            return Flow::Stay;
        }
        if self.to.is_some() {
            let side = self.side_at(self.hover);
            self.write(side, cx);
            return Flow::Stay;
        }
        if self.from.is_some() {
            cx.say(Level::Info, "Önce dizinin sonuna tıklayın ya da boyunu yazın.");
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        if let Some(from) = self.from {
            out.path = vec![from];
            if let Some(to) = self.end() {
                out.path.push(to);
            }
        }
        out.tag = self.hover.filter(|_| self.from.is_some()).map(|at| {
            let mut lines = vec![LABEL.to_owned()];
            if let Some(why) = &self.why {
                lines.extend(crate::wall_dimensions::wrapped(why));
            } else {
                lines.extend(self.design_words(format));
            }
            Tag { at, lines }
        });
        out.tag_tone = if self.why.is_some() {
            Tone::Danger
        } else {
            Tone::Accent
        };
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_spots_keep_a_module_between_sink_and_cooktop() {
        // 3.60 m of 60 cm modules with a 90 cm sink: 5 modules (0.90 + 4 × 0.60 = 3.30).
        assert_eq!(spots(Spot::Auto, Spot::Auto, 0.60, 3.60), (Some(2), Some(4)));
        // 2.40 m: 3 modules with the sink; the cooktop has no module apart from it.
        assert_eq!(spots(Spot::Auto, Spot::Auto, 0.60, 2.40), (Some(2), None));
        // 3.00 m: 4 modules; the last is two from the sink.
        assert_eq!(spots(Spot::Auto, Spot::Auto, 0.60, 3.00), (Some(2), Some(4)));
        // Too short for two modules with the sink: neither.
        assert_eq!(spots(Spot::Auto, Spot::Auto, 0.60, 1.40), (None, None));
        // No sink: the cooktop second from the end.
        assert_eq!(spots(Spot::None, Spot::Auto, 0.60, 1.80), (None, Some(2)));
        // A sink chosen by hand: the cooktop keeps away from it.
        assert_eq!(spots(Spot::At(4), Spot::Auto, 0.60, 3.60), (Some(4), Some(2)));
    }
}
