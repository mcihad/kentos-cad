//! Mutfak dizisi (docs/adr/0706, 0727): a kitchen run along the line its cabinets' backs stand on, straight, an L or a
//! U (Biçim). Its start, its corners (each square to the leg before: the pointer goes along the square), its end (or a
//! leg's length typed); a straight run then takes the side the cabinets stand on, where the pointer is (Enter takes
//! the side shown), an L or a U the side it turns to, written with its end. Modül (M), Derinlik (D), Köşe (K), Evye
//! (E), Ocak (O), Fırın (F), Bulaşık makinesi (L), Buzdolabı (Z), Üst dolap (U) and Yan (Y) are also in the tool's
//! settings pane. `cad.casework.run` computes and writes the run in one undo step; its plan, the cabinets drawn from
//! their blocks with their widths, is the preview.
//!
//! Evye, Ocak and Bulaşık makinesi start Otomatik (the dishwasher none): the sink in the second module of a run of two
//! or more, the cooktop the second from the end, or the last, at least one module from the sink (a run of three or
//! more), the dishwasher next to the sink (after it, else before it); none in a blind corner's module (the sink the
//! next clear one); the module chosen is shown in the settings and the tag, never written unseen.

use kentos_commands::{
    CaseworkCorner, CaseworkEnd, CaseworkModule, CaseworkModuleKind, CaseworkRun, CaseworkSide,
    CommandResult, DEFAULT_CASEWORK_DEPTH, DEFAULT_CASEWORK_MODULE,
};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::{ExecutionContext, casework_run, codes};

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

/// Where the sink, the cooktop or the dishwasher goes.
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
    Washer,
}

#[derive(Clone, Debug)]
pub struct Casework {
    /// The points placed: the start, the corners, the end.
    pts: Vec<Vec2>,
    /// The pointer, square to the leg before once a leg is placed.
    hover: Option<Vec2>,
    /// Yan of a straight run: none takes the pointer's side once the end is chosen.
    side: Option<CaseworkSide>,
    /// Biçim (0 straight, 1 an L, 2 a U), Köşe, Modül, Derinlik and Üst dolap: the session's memory's.
    corners: u8,
    corner: CaseworkCorner,
    module: f64,
    depth: f64,
    upper: bool,
    sink: Spot,
    cooktop: Spot,
    washer: Spot,
    oven: bool,
    fridge: Option<CaseworkEnd>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
    /// The planned run: its modules, its fillers, its path's length and its depth.
    planned: Option<Planned>,
}

#[derive(Clone, Debug)]
struct Planned {
    modules: Vec<CaseworkModule>,
    filler: f64,
    length: f64,
    depth: f64,
    /// What it stands before: a door, a passage or a low window of its wall (`casework_clash`, said before writing).
    clashes: Vec<String>,
}

impl Default for Casework {
    fn default() -> Self {
        Self {
            pts: Vec::new(),
            hover: None,
            side: None,
            corners: 0,
            corner: CaseworkCorner::Blind,
            module: DEFAULT_CASEWORK_MODULE,
            depth: DEFAULT_CASEWORK_DEPTH,
            upper: false,
            sink: Spot::Auto,
            cooktop: Spot::Auto,
            washer: Spot::None,
            oven: false,
            fridge: None,
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

/// The modules a run lays, by its plan; none when it lays none.
fn laid(run: &CaseworkRun, cx: &Context<'_>) -> Option<Vec<CaseworkModule>> {
    match casework_run::plan_in(cx.doc, run) {
        CommandResult::Completed { output, .. } => Some(output.modules),
        _ => None,
    }
}

/// The modules of `kind` (by number) a run's modules hold.
fn number_of(modules: &[CaseworkModule], kind: CaseworkModuleKind) -> Option<u32> {
    modules.iter().find(|m| m.kind == kind).map(|m| m.number)
}

impl Casework {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many points the path takes: its start, its corners, its end.
    fn needed(&self) -> usize {
        usize::from(self.corners) + 2
    }

    /// The path now: the points placed, then the pointer while one is still to place.
    fn path(&self) -> Vec<Vec2> {
        let mut out = self.pts.clone();
        if out.len() < self.needed()
            && let Some(h) = self.hover
        {
            out.push(h);
        }
        out
    }

    /// The pointer square to the leg before: on the line through the last point across the last leg; as it is before
    /// a leg is placed.
    fn squared(&self, p: Vec2) -> Vec2 {
        let n = self.pts.len();
        if n < 2 {
            return p;
        }
        let (a, b) = (self.pts[n - 2], self.pts[n - 1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx.hypot(dy);
        if len.is_nan() || len <= 0.0 {
            return p;
        }
        let (nx, ny) = (-dy / len, dx / len);
        let t = (p.x - b.x) * nx + (p.y - b.y) * ny;
        Vec2::new(b.x + nx * t, b.y + ny * t)
    }

    /// The side the cabinets stand on: an L's or U's turn; a straight run's Yan, else the pointer's once its end is
    /// chosen, else the left.
    fn side_at(&self, path: &[Vec2], at: Option<Vec2>) -> CaseworkSide {
        if path.len() >= 3 {
            let (a, b, c) = (path[0], path[1], path[2]);
            let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
            return if cross < 0.0 {
                CaseworkSide::Right
            } else {
                CaseworkSide::Left
            };
        }
        if let Some(side) = self.side {
            return side;
        }
        match (path, at) {
            ([a, b], Some(p)) if self.pts.len() == 2 => {
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

    /// The run along `path` on `side`, its sink, cooktop and dishwasher as chosen (Otomatik resolved by the run's
    /// plan).
    fn request(&self, path: &[Vec2], side: CaseworkSide, cx: &Context<'_>) -> CaseworkRun {
        let n = path.len();
        let mut run = CaseworkRun {
            layer_id: cx.doc.layers().active().to_owned(),
            from: points::wire(path[0]),
            through: (n > 2).then(|| path[1..n - 1].iter().map(|p| points::wire(*p)).collect()),
            to: points::wire(path[n - 1]),
            side: (side == CaseworkSide::Right).then_some(side),
            module: (self.module != DEFAULT_CASEWORK_MODULE).then_some(self.module),
            depth: (self.depth != DEFAULT_CASEWORK_DEPTH).then_some(self.depth),
            corner: (n > 2 && self.corner == CaseworkCorner::Open).then_some(CaseworkCorner::Open),
            sink: None,
            cooktop: None,
            dishwasher: None,
            oven: None,
            fridge: self.fridge,
            per_module: None,
            upper: self.upper.then_some(true),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        };
        // A module in a blind corner (its front behind the next leg's cabinets) takes nothing Otomatik: the run as it
        // would be with `k` there, whether module `k` is clear (docs/adr/0727 §7).
        let clear = |probe: &CaseworkRun, k: u32| {
            laid(probe, cx).is_some_and(|m| m.iter().any(|q| q.number == k && q.blind != Some(true)))
        };
        run.sink = match self.sink {
            Spot::None => None,
            Spot::At(k) => Some(k),
            Spot::Auto => {
                let probe = CaseworkRun {
                    sink: Some(2),
                    ..run.clone()
                };
                let count = laid(&probe, cx).map_or(0, |m| m.len() as u32);
                // The second module, else the next clear one, else the first.
                (count >= 2)
                    .then(|| {
                        (2..=count).chain([1]).find(|&k| {
                            let probe = CaseworkRun {
                                sink: Some(k),
                                ..run.clone()
                            };
                            clear(&probe, k)
                        })
                    })
                    .flatten()
            }
        };
        run.cooktop = match self.cooktop {
            Spot::None => None,
            Spot::At(k) => Some(k),
            Spot::Auto => laid(&run, cx).and_then(|m| {
                let count = m.len() as u32;
                if count < 3 {
                    return None;
                }
                let mut order = vec![count - 1, count];
                order.extend((1..count - 1).rev());
                order.into_iter().find(|&k| {
                    let probe = CaseworkRun {
                        cooktop: Some(k),
                        oven: self.oven.then_some(true),
                        ..run.clone()
                    };
                    run.sink.is_none_or(|s| k.abs_diff(s) >= 2) && clear(&probe, k)
                })
            }),
        };
        run.oven = (self.oven && run.cooktop.is_some()).then_some(true);
        run.dishwasher = match self.washer {
            Spot::None => None,
            Spot::At(k) => Some(k),
            Spot::Auto => run.sink.and_then(|s| {
                let count = laid(&run, cx).map_or(0, |m| m.len() as u32);
                [s + 1, s.saturating_sub(1)].into_iter().find(|&k| {
                    let probe = CaseworkRun {
                        dishwasher: Some(k),
                        ..run.clone()
                    };
                    k >= 1 && k <= count && Some(k) != run.cooktop && clear(&probe, k)
                })
            }),
        };
        run
    }

    fn see(&mut self, cx: &mut Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        self.planned = None;
        let path = self.path();
        if path.len() < 2 || path.windows(2).any(|w| w[0] == w[1]) {
            return;
        }
        let input = self.request(&path, self.side_at(&path, self.hover), cx);
        match casework_run::drawn_in(cx.doc, &input) {
            CommandResult::Completed { output, warnings } => {
                for (shape, dashed) in &output.shapes {
                    self.drawn.strokes.extend(
                        Outline::of(shape, dashed.then_some([5.0, 4.0]), 1.5, Tone::Accent).strokes,
                    );
                }
                self.planned = Some(Planned {
                    modules: output.plan.modules.clone(),
                    filler: output.plan.filler,
                    length: output.plan.legs.iter().map(|l| l.length).sum(),
                    depth: self.depth,
                    clashes: warnings
                        .into_iter()
                        .filter(|w| w.code == codes::CASEWORK_CLASH)
                        .map(|w| w.message)
                        .collect(),
                });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.why = Some(error.message),
            _ => self.why = Some("Önizleme hazır değil; aracı yeniden başlatın.".into()),
        }
    }

    /// The planned run in words: its modules, its sink, cooktop and appliances, its filler, its countertop.
    fn design_words(&self, f: &Format) -> Vec<String> {
        self.planned
            .as_ref()
            .map_or_else(Vec::new, |p| run_words(p, self.fridge, f))
    }

    /// Writes the run along the points placed, on `side`.
    fn write(&mut self, side: CaseworkSide, cx: &mut Context<'_>) -> bool {
        if self.pts.len() < self.needed() {
            return false;
        }
        let input = self.request(&self.pts.clone(), side, cx);
        let result = casework_run::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(out) = points::written(result, cx) else {
            self.see(cx);
            return false;
        };
        // In the words of the run written: a point typed last leaves the preview's plan short of it.
        let written = Planned {
            modules: out.modules,
            filler: out.filler,
            length: out.legs.iter().map(|l| l.length).sum(),
            depth: self.depth,
            clashes: Vec::new(),
        };
        let words = run_words(&written, self.fridge, &cx.format());
        let mut said = format!("Mutfak dizisi çizildi: {}.", words.join("; "));
        if !out.blocks.is_empty() {
            said.push_str(&format!(
                " KentOS kitaplığından getirilen bloklar: {}.",
                out.blocks.join(", ")
            ));
        }
        cx.say(Level::Success, said);
        self.pts.clear();
        self.see(cx);
        true
    }

    /// The choices of Evye (`E`), Ocak (`O`) or Bulaşık makinesi (`L`): Otomatik with the module it takes now, Yok,
    /// and the run's modules.
    fn spot_choices(&self, key: &str) -> Vec<OptionChoice> {
        let (spot, kind) = match key {
            "E" => (self.sink, CaseworkModuleKind::Sink),
            "O" => (self.cooktop, CaseworkModuleKind::Cooktop),
            _ => (self.washer, CaseworkModuleKind::Dishwasher),
        };
        let now = self.planned.as_ref().and_then(|p| number_of(&p.modules, kind));
        let count = self.planned.as_ref().map_or(0, |p| p.modules.len());
        let auto = match (spot, now) {
            (Spot::Auto, Some(k)) => format!("Otomatik ({k}. modül)"),
            (Spot::Auto, None) if self.planned.is_some() => "Otomatik (yer yok: yok)".into(),
            _ => "Otomatik".into(),
        };
        let choice = |typed: String, label: String, checked: bool| OptionChoice {
            typed,
            label,
            checked,
            icon: None,
            preview: None,
            command: None,
        };
        let mut out = vec![
            choice("auto".into(), auto, spot == Spot::Auto),
            choice("none".into(), "Yok".into(), spot == Spot::None),
        ];
        let mut top = count as u32;
        if let Spot::At(k) = spot {
            top = top.max(k);
        }
        out.extend((1..=top).map(|k| choice(k.to_string(), format!("{k}. modül"), spot == Spot::At(k))));
        out
    }

    /// Applies one value: false when it does not read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let words = upper_tr(js_trim(value));
        let raw = js_trim(value).to_lowercase();
        let spot = |words: &str| -> Option<Spot> {
            match words {
                "AUTO" | "OTOMATİK" | "OTO" => Some(Spot::Auto),
                "NONE" | "YOK" | "0" => Some(Spot::None),
                w => parse_number(w)
                    .filter(|v| *v >= 1.0 && *v <= 100.0 && v.fract() == 0.0)
                    .map(|v| Spot::At(v as u32)),
            }
        };
        let flag = |on: &mut bool, words: &str| match words {
            "YES" | "AÇIK" | "EVET" => {
                *on = true;
                true
            }
            "NO" | "KAPALI" | "HAYIR" => {
                *on = false;
                true
            }
            _ => false,
        };
        match key {
            "B" => {
                let corners = match raw.as_str() {
                    "straight" | "düz" | "duz" | "0" => 0,
                    "l" | "1" => 1,
                    "u" | "2" => 2,
                    _ => return false,
                };
                self.corners = corners;
                cx.memory.casework_corners = corners;
                self.pts.truncate(self.needed().saturating_sub(1).min(self.pts.len()));
                true
            }
            "K" => {
                self.corner = match raw.as_str() {
                    "blind" | "kör" | "kor" => CaseworkCorner::Blind,
                    "open" | "açık" | "acik" => CaseworkCorner::Open,
                    _ => return false,
                };
                cx.memory.casework_corner = self.corner;
                true
            }
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
            "L" => spot(&words).map(|s| self.washer = s).is_some(),
            "F" => flag(&mut self.oven, &words),
            "Z" => {
                self.fridge = match raw.as_str() {
                    "none" | "yok" => None,
                    "start" | "başta" | "basta" => Some(CaseworkEnd::Start),
                    "end" | "sonda" => Some(CaseworkEnd::End),
                    _ => return false,
                };
                true
            }
            "U" => {
                let mut on = self.upper;
                if !flag(&mut on, &words) {
                    return false;
                }
                self.upper = on;
                cx.memory.casework_upper = on;
                true
            }
            "Y" => match raw.as_str() {
                "cursor" | "imleç" | "" => {
                    self.side = None;
                    true
                }
                "left" | "sol" => {
                    self.side = Some(CaseworkSide::Left);
                    true
                }
                "right" | "sağ" => {
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
        self.corners = cx.memory.casework_corners.min(2);
        self.corner = cx.memory.casework_corner;
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        if self.pts.len() < self.needed() {
            self.pts.last().copied()
        } else {
            None
        }
    }

    fn snaps(&self) -> bool {
        self.pts.len() < self.needed()
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
        if let Some(ask) = self.ask {
            let words = match ask {
                Ask::Module => "modül enini yazın (30–120 cm, 15 cm adımla)",
                Ask::Depth => "tezgâh derinliğini yazın (30–120 cm)",
                Ask::Sink => "evyenin modülünü yazın (Yok, Otomatik ya da sıra numarası)",
                Ask::Cooktop => "ocağın modülünü yazın (Yok, Otomatik ya da sıra numarası)",
                Ask::Washer => "bulaşık makinesinin modülünü yazın (Yok, Otomatik ya da sıra numarası)",
            };
            return Prompt::new(LABEL, words).option("Geri", "G");
        }
        let n = self.pts.len();
        let step = if n == 0 {
            "dizinin başına tıklayın (dolapların sırtı bu çizgide)".to_owned()
        } else if n < self.needed() - 1 {
            format!("{}. köşeye tıklayın ya da kolun boyunu yazın", n)
        } else if n < self.needed() {
            if self.corners > 0 {
                "dizinin sonuna tıklayın ya da kolun boyunu yazın (dizi köşenin döndüğü yanda)".to_owned()
            } else {
                "dizinin sonuna tıklayın ya da boyunu yazın".to_owned()
            }
        } else {
            "dolapların duracağı tarafa tıklayın ya da Enter ile yazın".to_owned()
        };
        let spot = |s: Spot| match s {
            Spot::Auto => "Otomatik".to_owned(),
            Spot::None => "Yok".to_owned(),
            Spot::At(k) => format!("{k}. modül"),
        };
        let mut prompt = Prompt::new(LABEL, step)
            .option_with(
                "Biçim",
                "B",
                match self.corners {
                    0 => "Düz",
                    1 => "L",
                    _ => "U",
                },
            )
            .option("Modül", "M")
            .option("Derinlik", "D");
        if self.corners > 0 {
            prompt = prompt.option_with(
                "Köşe",
                "K",
                match self.corner {
                    CaseworkCorner::Blind => "Kör",
                    CaseworkCorner::Open => "Açık",
                },
            );
        }
        prompt = prompt
            .option_with("Evye", "E", spot(self.sink))
            .option_with("Ocak", "O", spot(self.cooktop))
            .option_with("Bulaşık makinesi", "L", spot(self.washer))
            .option_with("Fırın", "F", if self.oven { "açık" } else { "kapalı" })
            .option_with(
                "Buzdolabı",
                "Z",
                match self.fridge {
                    None => "Yok",
                    Some(CaseworkEnd::Start) => "Başta",
                    Some(CaseworkEnd::End) => "Sonda",
                },
            )
            .option("Üst dolap", "U");
        if self.corners == 0 {
            prompt = prompt.option_with(
                "Yan",
                "Y",
                match self.side {
                    None => "İmleç tarafı",
                    Some(CaseworkSide::Left) => "Sol",
                    Some(CaseworkSide::Right) => "Sağ",
                },
            );
        }
        prompt.option("Yaz", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(if self.pts.len() < self.needed() {
            self.squared(p.world)
        } else {
            p.world
        });
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.pts.len() >= self.needed() {
            let path = self.pts.clone();
            let side = self.side_at(&path, Some(p.world));
            self.write(side, cx);
            return;
        }
        self.accept_point(self.squared(p.world), cx);
    }

    fn accepts_points(&self) -> bool {
        self.pts.len() < self.needed()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        let p = self.squared(p);
        if self.pts.last() == Some(&p) {
            cx.say(Level::Info, "Bu nokta bir öncekiyle aynı; başka bir noktaya tıklayın.");
            return true;
        }
        self.pts.push(p);
        // An L or a U is written with its end: its side is the side it turns to.
        if self.corners > 0 && self.pts.len() == self.needed() {
            let path = self.pts.clone();
            let side = self.side_at(&path, None);
            self.write(side, cx);
            return true;
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
                Ask::Washer => "L",
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
            "L" => Some(Ask::Washer),
            "B" => {
                let next = match self.corners {
                    0 => "l",
                    1 => "u",
                    _ => "straight",
                };
                self.apply("B", next, cx);
                self.see(cx);
                return true;
            }
            "K" if self.corners > 0 => {
                self.corner = match self.corner {
                    CaseworkCorner::Blind => CaseworkCorner::Open,
                    CaseworkCorner::Open => CaseworkCorner::Blind,
                };
                cx.memory.casework_corner = self.corner;
                self.see(cx);
                return true;
            }
            "F" => {
                self.oven = !self.oven;
                self.see(cx);
                return true;
            }
            "Z" => {
                self.fridge = match self.fridge {
                    None => Some(CaseworkEnd::Start),
                    Some(CaseworkEnd::Start) => Some(CaseworkEnd::End),
                    Some(CaseworkEnd::End) => None,
                };
                self.see(cx);
                return true;
            }
            "U" => {
                self.upper = !self.upper;
                cx.memory.casework_upper = self.upper;
                self.see(cx);
                return true;
            }
            "Y" if self.corners == 0 => {
                self.side = match self.side {
                    None => Some(CaseworkSide::Left),
                    Some(CaseworkSide::Left) => Some(CaseworkSide::Right),
                    Some(CaseworkSide::Right) => None,
                };
                self.see(cx);
                return true;
            }
            _ => {
                if self.pts.len() < self.needed()
                    && let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover)
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
            Setting::choice(
                "B",
                "Biçim",
                choices(
                    &[("straight", "Düz"), ("l", "L"), ("u", "U")],
                    match self.corners {
                        0 => "straight",
                        1 => "l",
                        _ => "u",
                    },
                ),
            ),
            Setting::choice("M", "Modül eni", module_choices),
            Setting::text("D", "Tezgâh derinliği", f.length(self.depth)),
        ];
        if self.corners > 0 {
            rows.push(Setting::choice(
                "K",
                "Köşe",
                choices(
                    &[("blind", "Kör köşe"), ("open", "Açık köşe")],
                    match self.corner {
                        CaseworkCorner::Blind => "blind",
                        CaseworkCorner::Open => "open",
                    },
                ),
            ));
        }
        rows.extend([
            Setting::choice("E", "Evye", self.spot_choices("E")),
            Setting::choice("O", "Ocak", self.spot_choices("O")),
            Setting::toggle("F", "Fırın (ocağın altında)", self.oven),
            Setting::choice("L", "Bulaşık makinesi", self.spot_choices("L")),
            Setting::choice(
                "Z",
                "Buzdolabı",
                choices(
                    &[("none", "Yok"), ("start", "Başta"), ("end", "Sonda")],
                    match self.fridge {
                        None => "none",
                        Some(CaseworkEnd::Start) => "start",
                        Some(CaseworkEnd::End) => "end",
                    },
                ),
            ),
            Setting::toggle("U", "Üst dolap", self.upper),
        ]);
        if self.corners == 0 {
            rows.push(Setting::choice(
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
            ));
        }
        let words = self.design_words(f);
        if !words.is_empty() {
            rows.push(Setting::hint("T", "Dizi", words.join("; ")));
        }
        rows.push(Setting::hint(
            "P",
            "Bloklar",
            "Alt dolap, Evye (90 cm modül), Ocak, Fırın, Bulaşık makinesi, Buzdolabı ve Üst dolap çizimde bu adlarla \
             varsa onlar, yoksa KentOS kitaplığınınkiler aynı adımda getirilir. Dizi duvarın yüzüne bağlanır ve onu \
             izler.",
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
                "E" | "O" | "L" => "Yok, Otomatik ya da modülün sıra numarasını (1, 2 …) yazın.",
                "B" => "Biçim Düz, L ya da U olmalı.",
                "K" => "Köşe Kör ya da Açık olmalı.",
                "Z" => "Buzdolabı Yok, Başta ya da Sonda olmalı.",
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
        if self.pts.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.ask.take().is_some() {
            return Flow::Stay;
        }
        if self.pts.len() >= self.needed() {
            let path = self.pts.clone();
            let side = self.side_at(&path, self.hover);
            self.write(side, cx);
            return Flow::Stay;
        }
        if !self.pts.is_empty() {
            cx.say(
                Level::Info,
                if self.pts.len() + 1 < self.needed() {
                    "Önce köşeye tıklayın ya da kolun boyunu yazın."
                } else {
                    "Önce dizinin sonuna tıklayın ya da boyunu yazın."
                },
            );
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        out.path = self.path();
        out.tag = self.hover.filter(|_| !self.pts.is_empty()).map(|at| {
            let mut lines = vec![LABEL.to_owned()];
            if let Some(why) = &self.why {
                lines.extend(crate::wall_dimensions::wrapped(why));
            } else {
                lines.extend(self.design_words(format));
                // Written, the clash goes to the log as the command's warning; before, it is the tag's.
                for clash in self.planned.iter().flat_map(|p| &p.clashes) {
                    lines.extend(crate::wall_dimensions::wrapped(clash));
                }
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

/// A run in words: its modules, its sink, cooktop and appliances, a blind corner's appliance, its countertop, its filler
/// and its fridge.
fn run_words(p: &Planned, fridge: Option<CaseworkEnd>, f: &Format) -> Vec<String> {
    let mut first = format!("{} modül", p.modules.len());
    if let Some(m) = p.modules.iter().find(|m| m.kind == CaseworkModuleKind::Base) {
        first.push_str(&format!(" · {}", centimetres(m.width)));
    }
    let mut rows = vec![first];
    let mut parts = Vec::new();
    if let Some(k) = number_of(&p.modules, CaseworkModuleKind::Sink) {
        parts.push(format!("evye {k}."));
    }
    if let Some(m) = p.modules.iter().find(|m| m.kind == CaseworkModuleKind::Cooktop) {
        parts.push(if m.oven == Some(true) {
            format!("ocak ve fırın {}.", m.number)
        } else {
            format!("ocak {}.", m.number)
        });
    }
    if let Some(k) = number_of(&p.modules, CaseworkModuleKind::Dishwasher) {
        parts.push(format!("bulaşık {k}."));
    }
    if !parts.is_empty() {
        rows.push(format!("{} modül", parts.join(" · ")));
    }
    let blind: Vec<String> = p
        .modules
        .iter()
        .filter(|m| m.blind == Some(true) && m.kind != CaseworkModuleKind::Base)
        .map(|m| format!("{}.", m.number))
        .collect();
    if !blind.is_empty() {
        rows.push(format!("Kör köşede: {} modül; önü kapalı", blind.join(", ")));
    }
    let mut last = format!("Tezgâh {} × {}", f.length(p.length), f.length(p.depth));
    if p.filler > 0.0 {
        last.push_str(&format!(" · dolgu {}", f.length(p.filler)));
    }
    if let Some(end) = fridge {
        last.push_str(match end {
            CaseworkEnd::Start => " · buzdolabı başta",
            CaseworkEnd::End => " · buzdolabı sonda",
        });
    }
    rows.push(last);
    rows
}
