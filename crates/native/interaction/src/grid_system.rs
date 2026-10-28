//! Aks ağı (docs/adr/0707): a whole structural grid in one step. Its spacings across x and across y (“6 6 4.5”,
//! “3*6” for three of 6 m, “18/3” for 18 m in three), its first labels, how far its grids reach past the outermost ones,
//! its bubbles and, if asked, a column at every crossing tied to its two grids; the system follows the pointer and a
//! click puts it there, at its angle (Açı, D), or Yerleştirme → İki nokta: the first click its origin, the second its x
//! axis's direction, Eşit böl the x gaps splitting the second click's distance equally. `cad.grids.create`'s `system`
//! computes and writes it in one undo step (“Aks”); its plan is the preview, the grids' labels in their bubbles.
//!
//! Biçim Eğrisel (docs/adr/0725 §1): radial grids from the clicked centre, the first at its angle (or towards the
//! second click with İki nokta), the next its angles (degrees) round, counter-clockwise or clockwise, and arcs from the
//! first radius out by their gaps; `radial` writes it. Kesik (C): every grid only its ends so long on paper (§5). The
//! columns may be round or L, put by any of their box's nine points (§2, §3).

use kentos_commands::{
    ColumnAnchor, ColumnShape, CommandResult, DEFAULT_GRID_EXTENSION, Entity, GridBubbles, GridSystem, GridsCreate,
    RadialGridSystem, SystemColumns,
};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::{ExecutionContext, grids_create};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Memory, OptionChoice, Pointer, Preview, Tag, Tone, Tool, Values};
use crate::tool_settings::{Setting, choices};

pub const ID: &str = "gridSystem";
pub const LABEL: &str = "Aks ağı";

/// The most grids one spacing token repeats (“3*6”).
const MAX_REPEAT: usize = 100;

/// What a typed answer asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    X,
    Y,
    XStart,
    YStart,
    Extension,
    Angle,
    Width,
    Depth,
    Tag,
    Radius,
    Ends,
}

#[derive(Clone, Debug)]
pub struct GridSystemTool {
    origin: Option<Vec2>,
    hover: Option<Vec2>,
    /// İki nokta: the first click is the origin, the second the x axis's direction.
    two_points: bool,
    /// The turn when placed with one click, radians counter-clockwise.
    angle: f64,
    x: Vec<f64>,
    y: Vec<f64>,
    x_start: String,
    y_start: String,
    extension: f64,
    bubbles: GridBubbles,
    columns: bool,
    width: f64,
    depth: f64,
    tag: String,
    /// Eğrisel (docs/adr/0725 §1): its radial grids' turns (radians, above 0), clockwise or not, its first arc's radius
    /// and the gaps of its arcs out.
    radial: bool,
    turns: Vec<f64>,
    clockwise: bool,
    radius: f64,
    rings: Vec<f64>,
    /// İki nokta, Eşit böl: the x gaps split the second click's distance equally.
    equal: bool,
    /// Kesik: every grid's ends on paper, millimetres (docs/adr/0725 §5).
    ends: Option<f64>,
    /// The columns' shape, L legs (none: half their sides) and anchor (docs/adr/0725 §2, §3).
    shape: ColumnShape,
    legs: Option<[f64; 2]>,
    anchor: ColumnAnchor,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
    /// The planned system: its grids across x and y (radial and arcs), its columns, its size along x and y (its turn in
    /// degrees and its outer radius).
    planned: Option<(usize, usize, usize, f64, f64)>,
}

impl Default for GridSystemTool {
    fn default() -> Self {
        Self {
            origin: None,
            hover: None,
            two_points: false,
            angle: 0.0,
            x: vec![6.0, 6.0, 6.0],
            y: vec![5.0, 5.0],
            x_start: "1".into(),
            y_start: "A".into(),
            extension: DEFAULT_GRID_EXTENSION,
            bubbles: GridBubbles::Both,
            columns: false,
            width: 0.3,
            depth: 0.5,
            tag: String::new(),
            radial: false,
            turns: vec![30f64.to_radians(); 3],
            clockwise: false,
            radius: 6.0,
            rings: vec![4.0, 4.0],
            equal: false,
            ends: None,
            shape: ColumnShape::Rect,
            legs: None,
            anchor: ColumnAnchor::Center,
            ask: None,
            drawn: Preview::default(),
            why: None,
            planned: None,
        }
    }
}

/// Spacings typed: lengths apart by spaces or semicolons, “n*d” (or n×d, nxd) n of them, “L/n” L split in n equal
/// gaps (docs/adr/0725 §4); none when one does not read or is not above 0.
pub fn spacings(text: &str, length: impl Fn(&str) -> Option<f64>) -> Option<Vec<f64>> {
    let count_of = |n: &str| {
        (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
            .then(|| n.parse::<usize>().ok().filter(|n| (1..=MAX_REPEAT).contains(n)))
    };
    let mut out = Vec::new();
    for token in text.split(|c: char| c.is_whitespace() || c == ';').filter(|t| !t.is_empty()) {
        if let Some((whole, n)) = token.rsplit_once('/') {
            let n = count_of(n)??;
            let d = length(whole).filter(|d| d.is_finite() && *d > 0.0)? / n as f64;
            out.extend(std::iter::repeat_n(d, n));
            continue;
        }
        let (count, value) = match token.split_once(['*', '×', 'x', 'X']) {
            Some((n, d)) if count_of(n).is_some() => (count_of(n)??, d),
            _ => (1, token),
        };
        let d = length(value).filter(|d| d.is_finite() && *d > 0.0)?;
        out.extend(std::iter::repeat_n(d, count));
    }
    Some(out)
}

/// A radial system's turns as the settings show them: degrees, runs of one value as “n*d”.
fn turns_text(turns: &[f64]) -> String {
    let degrees: Vec<f64> = turns.iter().map(|t| (t.to_degrees() * 1e6).round() / 1e6).collect();
    let mut words = Vec::new();
    let mut i = 0;
    while i < degrees.len() {
        let mut j = i + 1;
        while j < degrees.len() && degrees[j] == degrees[i] {
            j += 1;
        }
        let value = format!("{}°", crate::fixed(degrees[i], 2).trim_end_matches('0').trim_end_matches(['.', ',']));
        words.push(if j - i > 1 { format!("{}*{value}", j - i) } else { value });
        i = j;
    }
    if words.is_empty() {
        "Tek aks".into()
    } else {
        words.join(" ")
    }
}

/// Spacings as the settings show them, in the drawing's unit with the digits they need: runs of one value as “n*d”.
fn spacings_text(gaps: &[f64], f: &Format) -> String {
    let mut words = Vec::new();
    let mut i = 0;
    while i < gaps.len() {
        let mut j = i + 1;
        while j < gaps.len() && gaps[j] == gaps[i] {
            j += 1;
        }
        let value = f.plain(gaps[i]);
        words.push(if j - i > 1 {
            format!("{}*{value}", j - i)
        } else {
            value
        });
        i = j;
    }
    if words.is_empty() {
        "Tek aks".into()
    } else {
        words.join(" ")
    }
}

impl GridSystemTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn remember(&self, memory: &mut Memory) {
        if let (Some(x), Some(y)) = (Values::from_slice(&self.x), Values::from_slice(&self.y)) {
            memory.grid_system_x = x;
            memory.grid_system_y = y;
        }
        memory.grid_system_extension = self.extension;
        memory.grid_system_columns = self.columns;
        memory.grid_system_column = (self.width, self.depth);
    }

    /// The turn now: the pointer's direction from the origin with İki nokta, else the one typed.
    fn turn(&self) -> f64 {
        match (self.two_points, self.origin, self.hover) {
            (true, Some(o), Some(h)) if (h.x, h.y) != (o.x, o.y) => (h.y - o.y).atan2(h.x - o.x),
            _ => self.angle,
        }
    }

    /// The columns asked: their sizes, shape, L legs (half their sides unless typed) and anchor.
    fn system_columns(&self) -> Option<SystemColumns> {
        self.columns.then(|| SystemColumns {
            shape: (self.shape != ColumnShape::Rect).then_some(self.shape),
            width: self.width,
            depth: (self.shape != ColumnShape::Round && self.depth != self.width).then_some(self.depth),
            legs: (self.shape == ColumnShape::LShape)
                .then(|| self.legs.unwrap_or([self.depth / 2.0, self.width / 2.0])),
            anchor: (self.anchor != ColumnAnchor::Center).then_some(self.anchor),
            tag_start: (!self.tag.trim().is_empty()).then(|| self.tag.trim().to_owned()),
            ..SystemColumns::default()
        })
    }

    /// The x gaps: with İki nokta and Eşit böl, the second click's distance split equally among as many.
    fn x_gaps(&self) -> Vec<f64> {
        match (self.two_points && self.equal, self.origin, self.hover) {
            (true, Some(o), Some(h)) if !self.x.is_empty() => {
                let total = (h.x - o.x).hypot(h.y - o.y);
                if total > 0.0 {
                    vec![total / self.x.len() as f64; self.x.len()]
                } else {
                    self.x.clone()
                }
            }
            _ => self.x.clone(),
        }
    }

    /// The system at the origin (else the pointer), turned as it stands now.
    fn request(&self, cx: &Context<'_>) -> Option<GridsCreate> {
        let origin = self.origin.or(self.hover)?;
        let angle = if self.two_points && self.origin.is_none() {
            0.0
        } else {
            self.turn()
        };
        let sign = if self.clockwise { -1.0 } else { 1.0 };
        Some(GridsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            grids: Vec::new(),
            series: None,
            system: (!self.radial).then(|| GridSystem {
                origin: points::wire(origin),
                angle: (angle != 0.0).then_some(angle),
                x: self.x_gaps(),
                y: self.y.clone(),
                x_start: (self.x_start != "1").then(|| self.x_start.clone()),
                y_start: (self.y_start != "A").then(|| self.y_start.clone()),
                extension: (self.extension != DEFAULT_GRID_EXTENSION).then_some(self.extension),
                bubbles: (self.bubbles != GridBubbles::Both).then_some(self.bubbles),
                bubble_size: None,
                ends: self.ends,
                // Etkin kat (docs/adr/0713): its columns' (and grids') level.
                arch: cx.level_arch(),
                columns: self.system_columns(),
            }),
            radial: self.radial.then(|| RadialGridSystem {
                center: points::wire(origin),
                angle: (angle != 0.0).then_some(angle),
                turns: self.turns.iter().map(|t| sign * t).collect(),
                radius: self.radius,
                rings: self.rings.clone(),
                radial_start: (self.x_start != "1").then(|| self.x_start.clone()),
                arc_start: (self.y_start != "A").then(|| self.y_start.clone()),
                extension: (self.extension != DEFAULT_GRID_EXTENSION).then_some(self.extension),
                bubbles: (self.bubbles != GridBubbles::Both).then_some(self.bubbles),
                bubble_size: None,
                ends: self.ends,
                arch: cx.level_arch(),
                columns: self.system_columns(),
            }),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        self.planned = None;
        let Some(input) = self.request(cx) else {
            return;
        };
        match grids_create::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                crate::arch_tools::flight_drawn(&output.grids, cx.doc.settings().plot_scale, &mut self.drawn);
                let grids = output.grids.iter().filter(|e| matches!(e, Entity::Grid(_))).count();
                let columns = output.grids.len() - grids;
                self.planned = Some(if self.radial {
                    (
                        self.turns.len() + 1,
                        self.rings.len() + 1,
                        columns,
                        self.turns.iter().sum::<f64>().to_degrees(),
                        self.radius + self.rings.iter().sum::<f64>(),
                    )
                } else {
                    (
                        self.x.len() + 1,
                        self.y.len() + 1,
                        columns,
                        self.x_gaps().iter().sum(),
                        self.y.iter().sum(),
                    )
                });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.why = Some(error.message),
            _ => self.why = Some("Önizleme hazır değil; aracı yeniden başlatın.".into()),
        }
    }

    fn words(&self, f: &Format) -> Vec<String> {
        let Some((nx, ny, columns, sx, sy)) = self.planned else {
            return Vec::new();
        };
        let mut first = if self.radial {
            format!("{nx} radyal × {ny} yay")
        } else {
            format!("{nx} × {ny} aks")
        };
        if columns > 0 {
            first.push_str(&format!(" · {columns} kolon"));
        }
        let mut out = vec![
            first,
            if self.radial {
                format!(
                    "{}° · R {} … {}",
                    crate::fixed(sx, 2),
                    f.length(self.radius),
                    f.length(sy)
                )
            } else {
                format!("{} × {}", f.length(sx), f.length(sy))
            },
        ];
        let turn = self.turn();
        if turn != 0.0 {
            out.push(format!("Açı {}°", crate::fixed(turn.to_degrees(), 2)));
        }
        out
    }

    fn write(&mut self, cx: &mut Context<'_>) -> bool {
        let Some(input) = self.request(cx) else {
            return false;
        };
        let words = self.words(&cx.format());
        let result = grids_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_none() {
            self.see(cx);
            return false;
        }
        cx.say(Level::Success, format!("Aks ağı çizildi: {}.", words.join("; ")));
        self.remember(cx.memory);
        self.origin = None;
        self.see(cx);
        true
    }

    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let words = upper_tr(js_trim(value));
        match key {
            "F" if matches!(value, "orthogonal" | "radial") => {
                self.radial = value == "radial";
                self.origin = None;
                true
            }
            "X" if self.radial => {
                let turns = if words.is_empty() || words.starts_with("TEK") {
                    Some(Vec::new())
                } else {
                    spacings(value, |t| parse_number(js_trim(t).trim_end_matches('°'))).map(|d| {
                        d.into_iter().map(f64::to_radians).collect::<Vec<_>>()
                    })
                };
                match turns.filter(|t| t.len() <= Values::MAX && t.iter().sum::<f64>() < std::f64::consts::TAU) {
                    Some(t) => {
                        self.turns = t;
                        true
                    }
                    None => false,
                }
            }
            "Y" if self.radial => {
                let gaps = if words.is_empty() || words.starts_with("TEK") {
                    Some(Vec::new())
                } else {
                    spacings(value, |t| cx.typed_length(t))
                };
                match gaps.filter(|g| g.len() <= Values::MAX) {
                    Some(g) => {
                        self.rings = g;
                        true
                    }
                    None => false,
                }
            }
            "R" => match cx.typed_length(value).filter(|v| v.is_finite() && *v > 0.0) {
                Some(v) => {
                    self.radius = v;
                    true
                }
                None => false,
            },
            "N" if matches!(value, "ccw" | "cw") => {
                self.clockwise = value == "cw";
                true
            }
            "Q" => match words.as_str() {
                "YES" | "AÇIK" | "EVET" => {
                    self.equal = true;
                    true
                }
                "NO" | "KAPALI" | "HAYIR" => {
                    self.equal = false;
                    true
                }
                _ => false,
            },
            "C" => match crate::arch_tools::read_ends(value) {
                Some(v) => {
                    self.ends = v;
                    true
                }
                None => false,
            },
            "M" => match ColumnShape::ALL.iter().find(|s| s.name() == value || upper_tr(s.label()) == words) {
                Some(s) => {
                    self.shape = *s;
                    true
                }
                None => false,
            },
            "J" => match ColumnAnchor::ALL.iter().find(|a| a.name() == value || upper_tr(a.label()) == words) {
                Some(a) => {
                    self.anchor = *a;
                    true
                }
                None => false,
            },
            "V" => {
                if words.is_empty() || words.starts_with("YARI") {
                    self.legs = None;
                    return true;
                }
                let read: Option<Vec<f64>> = value
                    .split(|c: char| c.is_whitespace() || c == ';')
                    .filter(|t| !t.is_empty())
                    .map(|t| cx.typed_length(t).filter(|v| v.is_finite() && *v > 0.0))
                    .collect();
                match read.as_deref() {
                    Some([a, b]) => {
                        self.legs = Some([*a, *b]);
                        true
                    }
                    Some([a]) => {
                        self.legs = Some([*a, *a]);
                        true
                    }
                    _ => false,
                }
            }
            "X" | "Y" => {
                let gaps = if words.is_empty() || words.starts_with("TEK") {
                    Some(Vec::new())
                } else {
                    spacings(value, |t| cx.typed_length(t))
                };
                match gaps.filter(|g| g.len() <= Values::MAX) {
                    Some(g) if key == "X" => self.x = g,
                    Some(g) => self.y = g,
                    None => return false,
                }
                true
            }
            "A" | "B" if !js_trim(value).is_empty() => {
                let label = js_trim(value).to_owned();
                if key == "A" {
                    self.x_start = label;
                } else {
                    self.y_start = label;
                }
                true
            }
            "U" => match cx.typed_length(value).filter(|v| v.is_finite() && *v >= 0.0) {
                Some(v) => {
                    self.extension = v;
                    true
                }
                None => false,
            },
            "D" => match parse_number(js_trim(value).trim_end_matches('°')).filter(|v| v.is_finite()) {
                Some(deg) => {
                    self.angle = deg.to_radians();
                    true
                }
                None => false,
            },
            "P" if matches!(value, "click" | "two") => {
                self.two_points = value == "two";
                true
            }
            "L" => match value {
                "both" | "start" | "end" | "none" => {
                    self.bubbles = match value {
                        "start" => GridBubbles::Start,
                        "end" => GridBubbles::End,
                        "none" => GridBubbles::None,
                        _ => GridBubbles::Both,
                    };
                    true
                }
                _ => false,
            },
            "K" => match words.as_str() {
                "YES" | "AÇIK" | "EVET" => {
                    self.columns = true;
                    true
                }
                "NO" | "KAPALI" | "HAYIR" => {
                    self.columns = false;
                    true
                }
                _ => false,
            },
            "E" | "O" => match cx.typed_length(value).filter(|v| v.is_finite() && *v > 0.0) {
                Some(v) => {
                    if key == "E" {
                        if self.depth == self.width {
                            self.depth = v;
                        }
                        self.width = v;
                    } else {
                        self.depth = v;
                    }
                    true
                }
                None => false,
            },
            "S" => {
                self.tag = js_trim(value).to_owned();
                true
            }
            _ => false,
        }
    }
}

impl Tool for GridSystemTool {
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
        let m = &cx.memory;
        if !m.grid_system_x.is_empty() || !m.grid_system_y.is_empty() {
            self.x = m.grid_system_x.as_slice().to_vec();
            self.y = m.grid_system_y.as_slice().to_vec();
        }
        self.extension = m.grid_system_extension;
        self.columns = m.grid_system_columns;
        (self.width, self.depth) = m.grid_system_column;
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.origin
    }

    fn point_count(&self) -> usize {
        usize::from(self.origin.is_some())
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.origin.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        if let Some(ask) = self.ask {
            let words = match ask {
                Ask::X if self.radial => "radyal akslar arası açıları derece olarak yazın (30 30 ya da 3*30 ya da 90/3)",
                Ask::Y if self.radial => "yaylar arası aralıkları yazın (4 4 ya da 2*4)",
                Ask::X => "x aralıklarını yazın (6 6 4.5, 3*6 ya da 18/3)",
                Ask::Y => "y aralıklarını yazın (5 5, 2*5 ya da 10/2)",
                Ask::XStart => "x akslarının ilk adını yazın (1, A …)",
                Ask::YStart => "y akslarının ilk adını yazın (A, 1 …)",
                Ask::Extension => "uzatmayı yazın",
                Ask::Angle => "açıyı derece olarak yazın",
                Ask::Width => "kolon enini yazın",
                Ask::Depth => "kolon boyunu yazın",
                Ask::Tag => "ilk kolonun adını yazın (S1); boş: adsız",
                Ask::Radius => "ilk yayın yarıçapını yazın",
                Ask::Ends => "kesik aksların uçlarını kâğıtta mm olarak yazın (boş ya da 0: bütün aks)",
            };
            return Prompt::new(LABEL, words).option("Geri", "G");
        }
        let step = match (self.radial, self.two_points, self.origin) {
            (true, true, Some(_)) => "ilk radyal aksın doğrultusuna tıklayın ya da Enter ile yazın",
            (true, true, None) | (true, false, _) => "eğrisel ağın merkezine tıklayın",
            (false, true, Some(_)) => "x ekseninin doğrultusuna tıklayın ya da Enter ile yazın",
            (false, true, None) => "ağın başlangıcına tıklayın (ilk iki aksın kesişimi)",
            (false, false, _) => "ağı yerleştirmek için tıklayın (ilk iki aksın kesişimi)",
        };
        Prompt::new(LABEL, step)
            .option_with("Biçim", "F", if self.radial { "Eğrisel" } else { "Dik" })
            .option(if self.radial { "Açılar" } else { "X aralıkları" }, "X")
            .option(if self.radial { "Yay aralıkları" } else { "Y aralıkları" }, "Y")
            .option("Açı", "D")
            .option("Uzatma", "U")
            .option("Kolonlar", "K")
            .option(if self.origin.is_some() { "Yerleştir" } else { "Bitir" }, "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.hover = Some(p);
        if self.two_points && self.origin.is_none() {
            self.origin = Some(p);
            self.see(cx);
            return true;
        }
        self.write(cx);
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
                Ask::X => "X",
                Ask::Y => "Y",
                Ask::XStart => "A",
                Ask::YStart => "B",
                Ask::Extension => "U",
                Ask::Angle => "D",
                Ask::Width => "E",
                Ask::Depth => "O",
                Ask::Tag => "S",
                Ask::Radius => "R",
                Ask::Ends => "C",
            };
            self.configure(field, text, cx);
            return true;
        }
        self.ask = match key.as_str() {
            "X" => Some(Ask::X),
            "Y" => Some(Ask::Y),
            "A" => Some(Ask::XStart),
            "B" => Some(Ask::YStart),
            "U" => Some(Ask::Extension),
            "D" => Some(Ask::Angle),
            "E" => Some(Ask::Width),
            "O" => Some(Ask::Depth),
            "S" => Some(Ask::Tag),
            "R" if self.radial => Some(Ask::Radius),
            "C" => Some(Ask::Ends),
            "F" => {
                self.radial = !self.radial;
                self.origin = None;
                self.see(cx);
                return true;
            }
            "K" => {
                self.columns = !self.columns;
                self.see(cx);
                return true;
            }
            "P" => {
                self.two_points = !self.two_points;
                self.origin = None;
                self.see(cx);
                return true;
            }
            _ => {
                if let Some(p) = cx.typed_point(text, self.origin, self.hover) {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        };
        true
    }

    fn settings(&self, f: &Format) -> Vec<Setting> {
        let mut rows = vec![Setting::choice(
            "F",
            "Biçim",
            choices(
                &[("orthogonal", "Dik"), ("radial", "Eğrisel")],
                if self.radial { "radial" } else { "orthogonal" },
            ),
        )];
        if self.radial {
            rows.extend([
                Setting::text("R", "İlk yarıçap", f.length(self.radius)),
                Setting::text("X", "Açı aralıkları", turns_text(&self.turns)),
                Setting::choice(
                    "N",
                    "Dönüş yönü",
                    choices(
                        &[("ccw", "Saat yönünün tersi"), ("cw", "Saat yönü")],
                        if self.clockwise { "cw" } else { "ccw" },
                    ),
                ),
                Setting::text("Y", "Yay aralıkları", spacings_text(&self.rings, f)),
                Setting::text("A", "İlk radyal aks", self.x_start.clone()),
                Setting::text("B", "İlk yay", self.y_start.clone()),
            ]);
        } else {
            rows.extend([
                Setting::text("X", "X aralıkları", spacings_text(&self.x, f)),
                Setting::text("Y", "Y aralıkları", spacings_text(&self.y, f)),
                Setting::text("A", "X akslarının ilk adı", self.x_start.clone()),
                Setting::text("B", "Y akslarının ilk adı", self.y_start.clone()),
            ]);
        }
        rows.extend([
            Setting::text("U", "Uzatma", f.length(self.extension)),
            Setting::text("C", "Kesik uçlar", crate::arch_tools::ends_words(self.ends)),
            Setting::choice(
                "L",
                "Balonlar",
                choices(
                    &[
                        ("both", "İki uçta"),
                        ("start", "Başta"),
                        ("end", "Sonda"),
                        ("none", "Yok"),
                    ],
                    match self.bubbles {
                        GridBubbles::Both => "both",
                        GridBubbles::Start => "start",
                        GridBubbles::End => "end",
                        GridBubbles::None => "none",
                    },
                ),
            ),
            Setting::choice(
                "P",
                "Yerleştirme",
                choices(
                    &[("click", "Tek tık, açı sabit"), ("two", "İki nokta: açı imleçten")],
                    if self.two_points { "two" } else { "click" },
                ),
            ),
        ]);
        if !self.two_points {
            rows.push(Setting::text("D", "Açı", format!("{}°", crate::fixed(self.angle.to_degrees(), 2))));
        } else if !self.radial {
            rows.push(Setting::toggle("Q", "Eşit böl (ikinci tıklama x boyunu verir)", self.equal));
        }
        rows.push(Setting::toggle("K", "Kesişimlere kolon", self.columns));
        if self.columns {
            rows.push(Setting::choice(
                "M",
                "Kolon biçimi",
                choices(&ColumnShape::ALL.map(|s| (s.name(), s.label())), self.shape.name()),
            ));
            rows.push(Setting::text(
                "E",
                if self.shape == ColumnShape::Round { "Kolon çapı" } else { "Kolon eni" },
                f.length(self.width),
            ));
            if self.shape != ColumnShape::Round {
                rows.push(Setting::text("O", "Kolon boyu", f.length(self.depth)));
            }
            if self.shape == ColumnShape::LShape {
                let [a, b] = self.legs.unwrap_or([self.depth / 2.0, self.width / 2.0]);
                rows.push(Setting::text(
                    "V",
                    "L kolları",
                    if self.legs.is_some() {
                        format!("{} {}", f.plain(a), f.plain(b))
                    } else {
                        format!("Yarısı ({} {})", f.plain(a), f.plain(b))
                    },
                ));
            }
            rows.push(Setting::choice(
                "J",
                "Kolon hizası",
                choices(&ColumnAnchor::ALL.map(|a| (a.name(), a.label())), self.anchor.name()),
            ));
            rows.push(Setting::text(
                "S",
                "İlk kolonun adı",
                if self.tag.is_empty() { "Adsız".into() } else { self.tag.clone() },
            ));
        }
        let words = self.words(f);
        if !words.is_empty() {
            rows.push(Setting::hint("T", "Ağ", words.join("; ")));
        }
        rows.push(Setting::hint(
            "H",
            "Aks ağı",
            if self.radial {
                "Açılar ve aralıklar boşlukla ayrılır; 3*30 üç kez 30°, 90/3 doksan derece üç eşit açı demektir. Radyal \
                 akslar sayıyla, yaylar harfle başlar. Kolonlar kesişimlerine akslarıyla bağlı, radyal aksın \
                 doğrultusunda yazılır; aks taşınınca kolon da gider."
            } else {
                "Aralıklar boşlukla ayrılır; 3*6 üç kez 6, 18/3 on sekizin üç eşit aralığı demektir. X aksları sayıyla, \
                 Y aksları harfle başlar (I ve O atlanır). Kolonlar kesişimlerine akslarıyla bağlı yazılır; aks \
                 taşınınca kolon da gider."
            },
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
            self.remember(cx.memory);
            self.see(cx);
            return true;
        }
        cx.say(
            Level::Warn,
            match key {
                "X" if self.radial => "Açıları sıfırdan büyük derecelerle boşlukla yazın (30 30, 3*30 ya da 90/3); toplamı 360°'den küçük olmalı.",
                "X" | "Y" => "Aralıkları sıfırdan büyük uzunluklar olarak boşlukla yazın: 6 6 4.5, 3*6 ya da 18/3 (en çok 32 aralık).",
                "R" => "Yarıçap sıfırdan büyük bir uzunluk olmalı.",
                "C" => "Kesik uçları 0'dan büyük, en çok 1000 mm yazın; boş ya da 0 bütün aksı çizer.",
                "V" => "L kolonun iki kolunu sıfırdan büyük uzunluklar olarak boşlukla yazın (0.25 0.30); boş: yarısı.",
                "U" => "Uzatma sıfır ya da daha büyük bir uzunluk olmalı.",
                "D" => "Açıyı derece olarak bir sayıyla yazın; örneğin 30.",
                "E" | "O" => "Kolon ölçüsü sıfırdan büyük bir uzunluk olmalı.",
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
        if self.origin.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.ask.take().is_some() {
            return Flow::Stay;
        }
        // Enter writes only a system whose origin is chosen (İki nokta); else it ends the tool: a click has already
        // placed the system, and a second one at the same place would be a copy.
        if self.origin.is_some() {
            self.write(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        if let (true, Some(o), Some(h)) = (self.two_points, self.origin, self.hover) {
            out.path = vec![o, h];
        }
        out.tag = self.hover.map(|at| {
            let mut lines = vec![LABEL.to_owned()];
            if let Some(why) = &self.why {
                lines.extend(crate::wall_dimensions::wrapped(why));
            } else {
                lines.extend(self.words(format));
            }
            Tag { at, lines }
        });
        out.tag_tone = if self.why.is_some() { Tone::Danger } else { Tone::Accent };
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metres(t: &str) -> Option<f64> {
        parse_number(t)
    }

    #[test]
    fn spacings_read_repeats_and_refuse_what_is_not_a_length() {
        assert_eq!(spacings("6 6 4.5", metres), Some(vec![6.0, 6.0, 4.5]));
        assert_eq!(spacings("3*6 4.5", metres), Some(vec![6.0, 6.0, 6.0, 4.5]));
        assert_eq!(spacings("2x5;3", metres), Some(vec![5.0, 5.0, 3.0]));
        assert_eq!(spacings("", metres), Some(vec![]));
        assert_eq!(spacings("6 -1", metres), None);
        assert_eq!(spacings("0*6", metres), None);
        assert_eq!(spacings("altı", metres), None);
        // L/n: L in n equal gaps (docs/adr/0725 §4).
        assert_eq!(spacings("18/3", metres), Some(vec![6.0, 6.0, 6.0]));
        assert_eq!(spacings("10/4 3", metres), Some(vec![2.5, 2.5, 2.5, 2.5, 3.0]));
        assert_eq!(spacings("18/0", metres), None);
        assert_eq!(spacings("-6/2", metres), None);
    }
}
