//! Dikdörtgen duvar (plan/architecture.md ARCH-DRAW-01): a separate two-corner
//! tool. The chosen inner/outer/centre dimensions go to `cad.walls.rectangle`;
//! both the preview and the write use that command. Existing free-wall and
//! rectangle tools, their direction, offsets and rotation memory stay unchanged.

use kentos_commands::{
    CommandResult, Entity, RectangleFrame, WallRectangleReference, WallStyleDef, WallsRectangle,
    fixed_width,
};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::PI;
use kentos_geometry_core::tools::drawing::direction_angle;
use kentos_geometry_core::tools::point_text::{js_trim, parse_length_pair, parse_number};
use kentos_native_application::{ExecutionContext, walls_rectangle};

use crate::Vec2;
use crate::format::{Format, short_degrees};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, fold_tr};
use crate::tool::{Context, Flow, Memory, OptionChoice, Pointer, Preview, Stroke, Tag};
use crate::{Tool, wall};

pub const ID: &str = "wallRectangle";
pub const LABEL: &str = "Dikdörtgen duvar";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Size,
    Rotation,
    Width,
    Style,
    Reference,
}

#[derive(Clone, Debug, Default)]
pub struct WallRectangle {
    d: Taken,
    ask: Option<Ask>,
    reference: WallRectangleReference,
    rotation: f64,
    size: Option<(f64, f64)>,
    seen: Memory,
    format: Format,
    styles: Vec<WallStyleDef>,
    drawn: wall::Drawn,
    /// The reference ring comes from the planned wall, never a second UI calculation.
    ring: Vec<Vec2>,
}

impl WallRectangle {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = *cx.memory;
        self.format = cx.format();
        self.styles = wall::offered(cx.doc.settings());
        self.refresh(cx);
    }

    fn request(&self, a: Vec2, b: Vec2, cx: &Context<'_>) -> WallsRectangle {
        let style = wall::chosen(cx.memory, cx.doc.settings());
        let fixed = style
            .as_ref()
            .and_then(|s| fixed_width(&s.components))
            .is_some();
        WallsRectangle {
            layer_id: cx.doc.layers().active().to_owned(),
            frame: match self.size {
                Some((length, width)) => RectangleFrame::Size {
                    corner: points::wire(a),
                    length,
                    width,
                    toward: points::wire(b),
                },
                None => RectangleFrame::Corners {
                    a: points::wire(a),
                    b: points::wire(b),
                },
            },
            reference: self.reference,
            rotation: (self.rotation != 0.0).then_some(self.rotation),
            width: (!fixed).then_some(cx.memory.wall_width),
            style: style.map(|s| s.id),
            height: None,
            arch: None,
            attrs: cx.template_attrs(None),
            label: cx.template_label(),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            line_type: cx.draft.line_type,
            expected_revision: None,
        }
    }

    fn refresh(&mut self, cx: &Context<'_>) {
        self.drawn = wall::Drawn::default();
        self.ring.clear();
        let (Some(a), Some(b)) = (self.d.last(), self.d.hover) else {
            return;
        };
        if self.ask == Some(Ask::Rotation) {
            return;
        }
        match walls_rectangle::plan_in(cx.doc, &self.request(a, b, cx)) {
            CommandResult::Completed { output, .. } => {
                for e in &output.walls {
                    wall::add(e, &mut self.drawn);
                    if let Entity::Wall(w) = e {
                        self.ring = w.wall.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
                    }
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.drawn.why = Some(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    fn drop_draft(&mut self) {
        self.d.pts.clear();
        self.ask = None;
        self.ring.clear();
        self.drawn = wall::Drawn::default();
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.ask == Some(Ask::Rotation) {
            if let Some(a) = self.d.last() {
                if dist(a, p) > 1e-9 {
                    self.rotation = direction_angle(a, p);
                }
                self.ask = None;
                return true;
            }
            return false;
        }
        if self.ask.is_some() {
            return false;
        }
        self.d.begin(p, cx);
        let Some(a) = self.d.last() else {
            self.d.pts.push(p);
            self.d.hover = Some(p);
            return true;
        };
        let input = self.request(a, p, cx);
        let result = walls_rectangle::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            for id in out.ids {
                self.d.note(id, cx);
            }
            cx.say(
                Level::Success,
                format!(
                    "Dikdörtgen duvar eklendi; boy referansı: {}.",
                    self.reference.label()
                ),
            );
            self.drop_draft();
        }
        true
    }

    fn set_reference(&mut self, typed: &str, cx: &mut Context<'_>) {
        let key = fold_tr(js_trim(typed));
        if let Some(r) = WallRectangleReference::ALL
            .into_iter()
            .find(|r| fold_tr(r.label()) == key || fold_tr(r.name()) == key)
        {
            self.reference = r;
            self.ask = None;
        } else {
            cx.say(
                Level::Warn,
                "Boy referansını seçin: İç net, Dış ölçü veya Eksen.",
            );
        }
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = fold_tr(js_trim(text));
        if self.ask.is_some() && key == "G" {
            self.ask = None;
            return true;
        }
        if self.ask.is_none() {
            if key == "K" && self.size.is_some() {
                self.size = None;
                return true;
            }
            if key == "E"
                && wall::chosen(cx.memory, cx.doc.settings())
                    .as_ref()
                    .and_then(|s| fixed_width(&s.components))
                    .is_some()
            {
                cx.say(Level::Warn, "Bu stilin kalınlığı sabittir. Farklı kalınlık için Stil (S) seçeneğinden başka bir stil veya Yok seçin.");
                return true;
            }
            self.ask = match key.as_str() {
                "B" => Some(Ask::Size),
                "D" => Some(Ask::Rotation),
                "E" => Some(Ask::Width),
                "S" => Some(Ask::Style),
                "R" => Some(Ask::Reference),
                _ => None,
            };
            if self.ask.is_some() {
                return true;
            }
        }
        match self.ask {
            Some(Ask::Size) => {
                let Some((l, w)) = parse_length_pair(text) else {
                    return false;
                };
                let f = cx.format();
                let (l, w) = (l.metres(|n| f.to_metres(n)), w.metres(|n| f.to_metres(n)));
                if !(l.is_finite() && w.is_finite() && l > 0.0 && w > 0.0) {
                    cx.say(Level::Warn, "Uzunluk ve genişlik sıfırdan büyük olmalı.");
                } else {
                    self.size = Some((l, w));
                    self.ask = None;
                }
                true
            }
            Some(Ask::Rotation) => {
                let Some(deg) = parse_number(text).filter(|n| n.is_finite()) else {
                    return false;
                };
                self.rotation = deg * (PI / 180.0);
                self.ask = None;
                true
            }
            Some(Ask::Width) => {
                let Some(m) = points::plain_length(text, cx) else {
                    return false;
                };
                wall::typed(wall::Ask::Width, m, cx);
                if m > 0.0 && m <= 100.0 {
                    self.ask = None;
                }
                true
            }
            Some(Ask::Style) => {
                wall::choose("S", text, cx);
                self.ask = None;
                true
            }
            Some(Ask::Reference) => {
                self.set_reference(text, cx);
                true
            }
            None => match cx.typed_point(text, self.d.last(), self.d.hover) {
                Some(p) => self.accept(p, cx),
                None => false,
            },
        }
    }
}

impl Tool for WallRectangle {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn point_count(&self) -> usize {
        self.d.pts.len()
    }
    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }
    fn takes_words(&self) -> bool {
        matches!(self.ask, Some(Ask::Style | Ask::Reference))
    }
    fn accepts_points(&self) -> bool {
        self.ask.is_none() || self.ask == Some(Ask::Rotation)
    }
    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        let accepted = self.accept(p, cx);
        self.see(cx);
        accepted
    }
    fn prompt(&self) -> Prompt {
        if let Some(ask) = self.ask {
            return Prompt::new(
                LABEL,
                match ask {
                    Ask::Size => "uzunluk ve genişliği yazın; ör. 4,3 (seçilen boy referansında)",
                    Ask::Rotation => {
                        "dönme açısını yazın (derece); köşe seçiliyse yön de gösterebilirsiniz"
                    }
                    Ask::Width => "duvar kalınlığını yazın",
                    Ask::Style => "duvar stilinin adını yazın (Yok: stilsiz)",
                    Ask::Reference => "boy referansını seçin: İç net, Dış ölçü veya Eksen",
                },
            )
            .option("Geri", "G");
        }
        let text = if self.d.pts.is_empty() {
            "ilk köşeyi belirtin"
        } else if self.size.is_some() {
            "dikdörtgenin hangi yana açılacağını gösterin"
        } else {
            "karşı köşeyi belirtin"
        };
        let style = wall::chosen_in(&self.seen, &self.styles);
        let size = self
            .size
            .map(|(l, w)| format!("{} × {}", self.format.length_bare(l), self.format.length(w)));
        let prompt = Prompt::new(LABEL, text)
            .option_with("Boy referansı", "R", self.reference.label())
            .option_with(
                "Kalınlık",
                "E",
                self.format.length(wall::width(&self.seen, style)),
            )
            .option_with(
                "Stil",
                "S",
                style.map_or(wall::NO_STYLE.to_owned(), |s| s.name.clone()),
            );
        let prompt = match size {
            Some(s) => prompt
                .option_with("Boyutlar", "B", s)
                .option("Köşeler", "K"),
            None => prompt.option("Boyutlar", "B"),
        };
        prompt.option_with("Döndür", "D", short_degrees(self.rotation))
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(p.world);
        self.see(cx);
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.accept(p.world, cx);
        self.see(cx);
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let taken = self.typed(text, cx);
        self.see(cx);
        taken
    }
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key == "R" {
            WallRectangleReference::ALL
                .into_iter()
                .map(|r| OptionChoice {
                    label: r.label().to_owned(),
                    typed: r.name().to_owned(),
                    icon: None,
                    preview: None,
                    checked: r == self.reference,
                    command: None,
                })
                .collect()
        } else {
            wall::choices(key, &self.seen, &self.styles)
        }
    }
    fn settings(&self, format: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::{Setting, wall_rows};
        let mut rows = wall_rows(&self.seen, &self.styles, format);
        rows.retain(|r| matches!(r.key, "S" | "E"));
        rows.push(Setting::choice("R", "Boy referansı", self.option_choices("R")));
        rows.push(Setting::text("B", "Boylar", self.size.map(|(l,w)|
            format!("{},{}", format.length(l), format.length(w))).unwrap_or_default()));
        rows.push(Setting::text("D", "Açı (°)", crate::format::fixed(self.rotation * 180.0 / PI, 2)));
        rows
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if matches!(key, "S" | "E") {
            let taken = crate::tool_settings::wall_apply(key, value, cx);
            self.see(cx); return taken;
        }
        if key == "R" { return self.choose_option(key, value, cx); }
        if key == "B" && value.trim().is_empty() { self.size = None; self.see(cx); return true; }
        let ask = match key { "B" => Ask::Size, "D" => Ask::Rotation, _ => return false };
        let previous = self.ask;
        self.ask = Some(ask);
        let took = self.typed(value, cx) && self.ask.is_none();
        self.ask = previous;
        if !took { cx.say(Level::Warn, "Geçerli bir değer yazın; boylar için 4m,3m gibi."); }
        self.see(cx); took
    }
    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let taken = match key {
            "R" => {
                self.set_reference(typed, cx);
                true
            }
            "S" => {
                self.ask = None;
                wall::choose(key, typed, cx)
            }
            _ => false,
        };
        self.see(cx);
        taken
    }
    /// An explicit finish drops only an unfinished draft; it never places the second corner.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.drop_draft();
        self.see(cx);
        Flow::Exit
    }
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            self.see(cx);
            true
        } else {
            false
        }
    }
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let taken = if self.ask.take().is_some() {
            true
        } else if !self.d.pts.is_empty() {
            self.drop_draft();
            true
        } else {
            self.d.undo_last_made(cx)
        };
        self.see(cx);
        taken
    }
    fn preview(&self, format: &Format) -> Preview {
        let (Some(a), Some(at)) = (self.d.last(), self.d.hover) else {
            return Preview::default();
        };
        if self.ask == Some(Ask::Rotation) {
            return Preview {
                strokes: vec![Stroke::dashed(vec![a, at], false, [3.0, 3.0])],
                ..Preview::default()
            };
        }
        let mut strokes = self.drawn.strokes.clone();
        if !self.ring.is_empty() {
            strokes.push(Stroke::dashed(self.ring.clone(), true, [3.0, 3.0]));
        }
        let mut lines = vec![format!("Boy referansı: {}", self.reference.label())];
        if self.ring.len() == 4 {
            let (length, width) = self.size.unwrap_or_else(|| {
                (
                    dist(self.ring[0], self.ring[1]),
                    dist(self.ring[1], self.ring[2]),
                )
            });
            lines.push(format!(
                "{} × {}",
                format.length_bare(length),
                format.length(width)
            ));
        }
        if let Some(why) = &self.drawn.why {
            lines.push(why.clone());
        }
        Preview {
            areas: self.drawn.areas.clone(),
            strokes,
            tag: Some(Tag { at, lines }),
            ..Preview::default()
        }
    }
}
