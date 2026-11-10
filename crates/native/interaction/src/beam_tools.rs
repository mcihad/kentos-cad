//! Kiriş (docs/adr/3041 §1): a beam's axis clicked point by point, its band previewed as the plan will draw it; Enter or
//! a quick right click writes it. Its width, height, justification, top's offset and number come from its settings
//! (25/50 centred until changed: TBDY 2018 7.4.1.1's least web is 25 cm, a usual depth 50); on Etkin kat, on its EK-4
//! layer (MM-TASI-KIRIS, docs/adr/3008).

use kentos_commands::{BeamsCreate, NewBeam, WallJustify};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, beams_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

pub const BEAM_ID: &str = "beam";

/// What the tool asks for in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Width,
    Height,
    Offset,
    Tag,
}

/// Kiriş.
#[derive(Clone, Debug)]
pub struct BeamTool {
    width: f64,
    height: f64,
    justify: WallJustify,
    offset: f64,
    tag: Option<String>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
}

impl Default for BeamTool {
    fn default() -> Self {
        BeamTool {
            width: 0.25,
            height: 0.5,
            justify: WallJustify::Center,
            offset: 0.0,
            tag: None,
            pts: Vec::new(),
            hover: None,
            ask: None,
            drawn: Preview::default(),
            why: None,
        }
    }
}

fn justify_key(j: WallJustify) -> &'static str {
    match j {
        WallJustify::Left => "left",
        WallJustify::Center => "center",
        WallJustify::Right => "right",
    }
}

impl BeamTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The call that writes the axis `pts` on `layer_id`.
    fn made(&self, pts: &[Vec2], layer_id: String, cx: &Context<'_>) -> BeamsCreate {
        BeamsCreate {
            layer_id,
            beams: vec![NewBeam {
                pts: points::wire_all(pts),
                width: self.width,
                height: self.height,
                justify: (self.justify != WallJustify::Center).then_some(self.justify),
                offset: (self.offset != 0.0).then_some(self.offset),
                tag: self.tag.clone(),
                // Etkin kat (docs/adr/0713).
                arch: cx.new_arch(),
                ..NewBeam::default()
            }],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// The axis so far and the pointer.
    fn axis(&self) -> Vec<Vec2> {
        let mut pts = self.pts.clone();
        if let Some(h) = self.hover
            && pts.last() != Some(&h)
        {
            pts.push(h);
        }
        pts
    }

    /// The beam the plan would write, drawn; why there is none.
    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        let pts = self.axis();
        if pts.len() < 2 {
            return;
        }
        let input = self.made(
            &pts,
            crate::arch_layer::planned(&crate::arch_layer::Kind::Beam, cx),
            cx,
        );
        match beams_create::plan_in(cx.doc, &input) {
            kentos_commands::CommandResult::Completed { output, .. } => {
                for e in &output.beams {
                    self.drawn.strokes.extend(
                        Outline::of(
                            &kentos_native_application::geometry::shape(e),
                            None,
                            2.0,
                            Tone::Accent,
                        )
                        .strokes,
                    );
                }
            }
            kentos_commands::CommandResult::Failed { error }
            | kentos_commands::CommandResult::Conflict { error }
            | kentos_commands::CommandResult::NeedsInput { error } => {
                self.why = Some(error.message)
            }
            _ => {}
        }
    }

    /// Writes the axis on its layer, opened with it when missing (docs/adr/3008).
    fn write(&mut self, cx: &mut Context<'_>) {
        let pts = std::mem::take(&mut self.pts);
        let written =
            crate::arch_layer::writing(&crate::arch_layer::Kind::Beam, cx, |layer, cx| {
                let input = self.made(&pts, layer.to_owned(), cx);
                let result = beams_create::execute(&mut ExecutionContext::new(cx.doc), input);
                points::written(result, cx).is_some()
            });
        if written {
            match self.tag.take() {
                // The next beam the next number (K101 → K102), as the column tool's.
                Some(t) => {
                    cx.say(
                        Level::Success,
                        format!("Kiriş çizildi: {t} {}.", self.size()),
                    );
                    self.tag = Some(kentos_commands::next_label(&t));
                }
                None => cx.say(Level::Success, format!("Kiriş çizildi: {}.", self.size())),
            }
        }
    }

    /// Its size as a plan writes it ("25/50").
    fn size(&self) -> String {
        kentos_commands::BeamFields {
            width: self.width,
            height: self.height,
            ..Default::default()
        }
        .size_text()
    }

    /// A setting's value taken; false when it cannot be read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let length =
            |cx: &mut Context<'_>| cx.typed_length(value).filter(|v| v.is_finite() && *v > 0.0);
        match key {
            "E" => length(cx).map(|v| self.width = v).is_some(),
            "Y" => length(cx).map(|v| self.height = v).is_some(),
            "F" => {
                let t = js_trim(value);
                let v = if t.is_empty() {
                    Some(0.0)
                } else {
                    cx.typed_length(t).filter(|v| v.is_finite())
                };
                v.map(|v| self.offset = v).is_some()
            }
            "N" => {
                let t = js_trim(value);
                self.tag = (!t.is_empty()).then(|| t.to_owned());
                true
            }
            "H" => match value {
                "left" => {
                    self.justify = WallJustify::Left;
                    true
                }
                "center" => {
                    self.justify = WallJustify::Center;
                    true
                }
                "right" => {
                    self.justify = WallJustify::Right;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

impl Tool for BeamTool {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(crate::arch_layer::Kind::Beam)
    }

    fn id(&self) -> &'static str {
        BEAM_ID
    }

    fn label(&self) -> &'static str {
        "Kiriş"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
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
                Ask::Width => "kirişin gövde genişliğini yazın",
                Ask::Height => "kirişin yüksekliğini yazın (döşeme dahil)",
                Ask::Offset => {
                    "kirişin tepesinin kot farkını yazın (0: üst döşemenin betonarmesinde)"
                }
                Ask::Tag => "kirişin numarasını yazın (K101 gibi; boş: yok)",
            };
            return Prompt::new("Kiriş", words).option("Geri", "G");
        }
        let n = |v: f64| v.to_string();
        let step = match self.pts.len() {
            0 => "kirişin ekseninin başına tıklayın",
            1 => "sonraki noktaya tıklayın",
            _ => "sonraki noktaya tıklayın ya da Enter ile yazın",
        };
        Prompt::new("Kiriş", step)
            .option_with("Genişlik", "E", n(self.width))
            .option_with("Yükseklik", "Y", n(self.height))
            .option_with("Hiza", "H", self.justify.label())
            .option_with("Kot farkı", "F", n(self.offset))
            .option_with(
                "Numara",
                "N",
                self.tag.clone().unwrap_or_else(|| "Yok".into()),
            )
            .option("Bitir", "Enter")
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
        if self.pts.last() == Some(&p) {
            return true;
        }
        self.pts.push(p);
        if self.pts.len() == kentos_commands::MAX_BEAM_POINTS {
            self.write(cx);
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
                Ask::Width => "E",
                Ask::Height => "Y",
                Ask::Offset => "F",
                Ask::Tag => "N",
            };
            if !self.apply(field, text, cx) {
                cx.say(
                    Level::Warn,
                    "Değer okunamadı; genişlik ve yükseklik sıfırdan büyük bir uzunluktur.",
                );
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        self.ask = match key.as_str() {
            "E" => Some(Ask::Width),
            "Y" => Some(Ask::Height),
            "F" => Some(Ask::Offset),
            "N" => Some(Ask::Tag),
            "H" => {
                self.justify = match self.justify {
                    WallJustify::Center => WallJustify::Left,
                    WallJustify::Left => WallJustify::Right,
                    WallJustify::Right => WallJustify::Center,
                };
                self.see(cx);
                return true;
            }
            _ => {
                if let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover) {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        };
        true
    }

    fn settings(&self, f: &Format) -> Vec<Setting> {
        vec![
            Setting::text("E", "Genişlik", f.length(self.width)),
            Setting::text("Y", "Yükseklik", f.length(self.height)),
            Setting::choice(
                "H",
                "Hiza",
                crate::tool_settings::choices(
                    &[("center", "Orta"), ("left", "Sol"), ("right", "Sağ")],
                    justify_key(self.justify),
                ),
            ),
            Setting::text("F", "Kot farkı", f.length(self.offset)),
            Setting::text(
                "N",
                "Numara",
                self.tag.clone().unwrap_or_else(|| "Yok".into()),
            ),
            Setting::hint(
                "P",
                "Tepesi",
                "Kirişin tepesi katın üstündeki döşemenin betonarmesindedir; döşeme yoksa katın tepesinde. Altındaki duvar kirişin altında biter.",
            ),
        ]
    }

    fn cursor_settings(&self) -> &'static [&'static str] {
        &["E", "Y"]
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
            "Değer uygulanmadı: genişlik ve yükseklik sıfırdan büyük bir uzunluktur.",
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
        if !self.pts.is_empty() {
            self.pts.clear();
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.pts.len() >= 2 {
            self.write(cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        out.path = self.axis();
        out.tag = self.hover.map(|at| {
            let mut lines = vec![format!("Kiriş {}", self.size())];
            if let Some(why) = &self.why {
                lines.push(why.clone());
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
