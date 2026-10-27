//! Aks ağı (docs/adr/0707): a whole structural grid in one step. Its spacings across x and across y (“6 6 4.5”,
//! “3*6” for three of 6 m), its first labels, how far its grids reach past the outermost ones, its bubbles and, if
//! asked, a column at every crossing tied to its two grids; the system follows the pointer and a click puts it there,
//! at its angle (Açı, D), or Yerleştirme → İki nokta: the first click its origin, the second its x axis's direction.
//! `cad.grids.create`'s `system` computes and writes it in one undo step (“Aks”); its plan is the preview, the grids'
//! labels in their bubbles.

use kentos_commands::{
    CommandResult, DEFAULT_GRID_EXTENSION, Entity, GridBubbles, GridSystem, GridsCreate,
    SystemColumns,
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
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
    /// The planned system: its grids across x and y, its columns, its size along x and y.
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
            ask: None,
            drawn: Preview::default(),
            why: None,
            planned: None,
        }
    }
}

/// Spacings typed: lengths apart by spaces or semicolons, “n*d” (or n×d, nxd) n of them; none when one does not read
/// or is not above 0.
pub fn spacings(text: &str, length: impl Fn(&str) -> Option<f64>) -> Option<Vec<f64>> {
    let mut out = Vec::new();
    for token in text.split(|c: char| c.is_whitespace() || c == ';').filter(|t| !t.is_empty()) {
        let (count, value) = match token.split_once(['*', '×', 'x', 'X']) {
            Some((n, d)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => {
                (n.parse::<usize>().ok().filter(|n| (1..=MAX_REPEAT).contains(n))?, d)
            }
            _ => (1, token),
        };
        let d = length(value).filter(|d| d.is_finite() && *d > 0.0)?;
        out.extend(std::iter::repeat_n(d, count));
    }
    Some(out)
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

    /// The system at the origin (else the pointer), turned as it stands now.
    fn request(&self, cx: &Context<'_>) -> Option<GridsCreate> {
        let origin = self.origin.or(self.hover)?;
        let angle = if self.two_points && self.origin.is_none() {
            0.0
        } else {
            self.turn()
        };
        Some(GridsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            grids: Vec::new(),
            series: None,
            system: Some(GridSystem {
                origin: points::wire(origin),
                angle: (angle != 0.0).then_some(angle),
                x: self.x.clone(),
                y: self.y.clone(),
                x_start: (self.x_start != "1").then(|| self.x_start.clone()),
                y_start: (self.y_start != "A").then(|| self.y_start.clone()),
                extension: (self.extension != DEFAULT_GRID_EXTENSION).then_some(self.extension),
                bubbles: (self.bubbles != GridBubbles::Both).then_some(self.bubbles),
                bubble_size: None,
                // Etkin kat (docs/adr/0713): its columns' (and grids') level.
                arch: cx.level_arch(),
                columns: self.columns.then(|| SystemColumns {
                    width: self.width,
                    depth: (self.depth != self.width).then_some(self.depth),
                    tag_start: (!self.tag.trim().is_empty()).then(|| self.tag.trim().to_owned()),
                    ..SystemColumns::default()
                }),
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
                self.planned = Some((
                    self.x.len() + 1,
                    self.y.len() + 1,
                    columns,
                    self.x.iter().sum(),
                    self.y.iter().sum(),
                ));
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
        let mut first = format!("{nx} × {ny} aks");
        if columns > 0 {
            first.push_str(&format!(" · {columns} kolon"));
        }
        let mut out = vec![first, format!("{} × {}", f.length(sx), f.length(sy))];
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
                Ask::X => "x aralıklarını yazın (6 6 4.5 ya da 3*6)",
                Ask::Y => "y aralıklarını yazın (5 5 ya da 2*5)",
                Ask::XStart => "x akslarının ilk adını yazın (1, A …)",
                Ask::YStart => "y akslarının ilk adını yazın (A, 1 …)",
                Ask::Extension => "uzatmayı yazın",
                Ask::Angle => "açıyı derece olarak yazın",
                Ask::Width => "kolon enini yazın",
                Ask::Depth => "kolon boyunu yazın",
                Ask::Tag => "ilk kolonun adını yazın (S1); boş: adsız",
            };
            return Prompt::new(LABEL, words).option("Geri", "G");
        }
        let step = match (self.two_points, self.origin) {
            (true, Some(_)) => "x ekseninin doğrultusuna tıklayın ya da Enter ile yazın",
            (true, None) => "ağın başlangıcına tıklayın (ilk iki aksın kesişimi)",
            (false, _) => "ağı yerleştirmek için tıklayın (ilk iki aksın kesişimi)",
        };
        Prompt::new(LABEL, step)
            .option("X aralıkları", "X")
            .option("Y aralıkları", "Y")
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
        let mut rows = vec![
            Setting::text("X", "X aralıkları", spacings_text(&self.x, f)),
            Setting::text("Y", "Y aralıkları", spacings_text(&self.y, f)),
            Setting::text("A", "X akslarının ilk adı", self.x_start.clone()),
            Setting::text("B", "Y akslarının ilk adı", self.y_start.clone()),
            Setting::text("U", "Uzatma", f.length(self.extension)),
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
        ];
        if !self.two_points {
            rows.push(Setting::text("D", "Açı", format!("{}°", crate::fixed(self.angle.to_degrees(), 2))));
        }
        rows.push(Setting::toggle("K", "Kesişimlere kolon", self.columns));
        if self.columns {
            rows.push(Setting::text("E", "Kolon eni", f.length(self.width)));
            rows.push(Setting::text("O", "Kolon boyu", f.length(self.depth)));
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
            "Aralıklar boşlukla ayrılır; 3*6 üç kez 6 demektir. X aksları sayıyla, Y aksları harfle başlar (I ve O \
             atlanır). Kolonlar kesişimlerine akslarıyla bağlı yazılır; aks taşınınca kolon da gider.",
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
                "X" | "Y" => "Aralıkları sıfırdan büyük uzunluklar olarak boşlukla yazın: 6 6 4.5 ya da 3*6 (en çok 32 aralık).",
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
    }
}
