//! Korkuluk (docs/adr/8045 §3): a free-standing railing on Etkin kat, on its EK-4 layer (MM-KORK-CUBUK, MM-KORK-CAM,
//! MM-KORK-DOLU; docs/adr/8008).
//!
//! - Çizgi (S) Çiz: its line clicked point by point, the middle of its top rail, its top rail previewed as the plan will
//!   draw it; Enter or a quick right click writes it, Kapat (K) or a click on its first point writes it closed.
//! - Çizgi (S) Boşluktan: a click inside a gallery's void (a hole of a slab on Etkin kat) takes the void's free sides,
//!   those no wall's face runs along (as a balcony's railing, docs/adr/0722 §4), open where a stair from the storey
//!   below arrives (its top riser's line): the railing runs along them on the slab, its top rail's face on the edge
//!   (PAİY 38/1 names "galeri boşluğu"). A void all round free is one closed railing; otherwise one railing a stretch,
//!   written together. A stretch shorter than 0.10 m gets none: PAİY 38/4's 0.10 m passes through no such gap.
//! - Yükseklik (Y) 1.10 m and Aralık (A) 0.10 m until changed: PAİY 38/1 "döşeme kotundan itibaren en az 1.10 metre",
//!   38/4 "en fazla 0.10 metre çapında geçişe izin verilir". A lower railing or a wider gap is written and said in red.
//! - Dolgu (D): Dikey çubuk, Cam, Dolu; Küpeşte (E) 5 cm, a balcony's railing's (docs/adr/0722); Kot farkı (F).

use kentos_commands::{
    Entity, NewRailing, RAILING_GAP, RAILING_HEIGHT, RAILING_WIDTH, RailingInfill, RailingsCreate,
};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, railings_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

pub const RAILING_ID: &str = "railing";

/// How its line comes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    /// Clicked point by point.
    Draw,
    /// Round a gallery's void, a hole of a slab on Etkin kat.
    Void,
}

/// What the tool asks for in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Height,
    Width,
    Gap,
    Offset,
}

/// Korkuluk.
#[derive(Clone, Debug)]
pub struct RailingTool {
    line: Line,
    height: f64,
    width: f64,
    infill: RailingInfill,
    gap: f64,
    offset: f64,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
}

impl Default for RailingTool {
    fn default() -> Self {
        RailingTool {
            line: Line::Draw,
            height: RAILING_HEIGHT,
            width: RAILING_WIDTH,
            infill: RailingInfill::Balusters,
            gap: RAILING_GAP,
            offset: 0.0,
            pts: Vec::new(),
            hover: None,
            ask: None,
            drawn: Preview::default(),
            why: None,
        }
    }
}

/// A railing's line and whether it closes.
type Piece = kentos_native_application::railing_voids::Piece;

impl RailingTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn kind(&self) -> crate::arch_layer::Kind {
        crate::arch_layer::Kind::Railing(self.infill)
    }

    /// The call that writes a railing along each of `pieces` on `layer_id`.
    fn call(&self, pieces: &[Piece], layer_id: String, cx: &Context<'_>) -> RailingsCreate {
        let railings = pieces
            .iter()
            .map(|(pts, closed)| NewRailing {
                pts: points::wire_all(pts),
                closed: closed.then_some(true),
                height: Some(self.height),
                width: Some(self.width),
                infill: (self.infill != RailingInfill::Balusters).then_some(self.infill),
                // PAİY 38/4's 0.10 m is what an absent gap reads.
                gap: (self.infill == RailingInfill::Balusters
                    && (self.gap - RAILING_GAP).abs() > 1e-12)
                    .then_some(self.gap),
                offset: (self.offset != 0.0).then_some(self.offset),
                // Etkin kat (docs/adr/0713).
                arch: cx.new_arch(),
                attrs: None,
            })
            .collect();
        RailingsCreate {
            layer_id,
            railings,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// The railings along the gallery's void under `at`: a hole of a slab on Etkin kat (any slab's without levels), its
    /// free sides open where a stair from below arrives, moved half the top rail's width onto the slab; why there are
    /// none.
    fn round_void(&self, at: Vec2, cx: &Context<'_>) -> Result<Vec<Piece>, String> {
        for e in cx.doc.entities() {
            let Entity::Slab(s) = e else {
                continue;
            };
            if cx.level.is_some() && s.slab.arch.level.as_deref() != cx.level {
                continue;
            }
            for hole in s.slab.parts.iter().flat_map(|p| &p.holes) {
                let ring: Vec<Vec2> = hole.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
                if ring.len() < 3 || !kentos_geometry_core::geometry::point_in_polygon(at, &ring) {
                    continue;
                }
                if hole
                    .bulges
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|x| *x != 0.0))
                {
                    return Err(
                        "Yay kenarlı boşluğun korkuluğu kendiliğinden çizilmiyor; Çizgi'yi Çiz yapıp noktalarla çizin."
                            .into(),
                    );
                }
                // Its free sides, open where a stair arrives, no stretch under 0.10 m (docs/adr/8045 §3): Denetim's
                // own reading of the void.
                let out = kentos_native_application::railing_voids::void_railings(
                    cx.doc, &s.slab, hole, cx.level, self.width,
                );
                if out.is_empty() {
                    return Err(
                        "Bu boşluğun bütün kenarları duvar ya da merdiven çıkışı: korkuluk gereken kenarı yok.".into(),
                    );
                }
                return Ok(out);
            }
        }
        Err(
            "Burada döşeme boşluğu yok: galeri boşluğunun içine tıklayın ya da Çizgi'yi Çiz yapın."
                .into(),
        )
    }

    /// The line so far and the pointer.
    fn line_now(&self) -> Vec<Vec2> {
        let mut pts = self.pts.clone();
        if let Some(h) = self.hover
            && pts.last() != Some(&h)
        {
            pts.push(h);
        }
        pts
    }

    /// What PAİY 38 says of its height and gap; none when it holds.
    fn rule_words(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.height < RAILING_HEIGHT - 1e-9 {
            out.push("PAİY 38/1: korkuluk döşemeden en az 1,10 m yükselir".into());
        }
        if self.infill == RailingInfill::Balusters && self.gap > RAILING_GAP + 1e-9 {
            out.push("PAİY 38/4: boşluklardan en çok 0,10 m çaplı geçiş olur".into());
        }
        out
    }

    /// The railing the plan would write now, drawn; why there is none.
    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        let pieces = match self.line {
            Line::Void => match self.hover.map(|h| self.round_void(h, cx)) {
                Some(Ok(pieces)) => pieces,
                Some(Err(why)) => {
                    self.why = Some(why);
                    return;
                }
                None => return,
            },
            Line::Draw => vec![(self.line_now(), false)],
        };
        if pieces.iter().all(|(pts, _)| pts.len() < 2) {
            return;
        }
        let input = self.call(&pieces, crate::arch_layer::planned(&self.kind(), cx), cx);
        match railings_create::plan_in(cx.doc, &input) {
            kentos_commands::CommandResult::Completed { output, .. } => {
                for e in &output.railings {
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

    /// Writes a railing along each of `pieces` on its layer, opened with it when missing (docs/adr/8008), as one undo
    /// step; says what it wrote.
    fn write(&mut self, pieces: &[Piece], cx: &mut Context<'_>) {
        let kind = self.kind();
        let written = crate::arch_layer::writing(&kind, cx, |layer, cx| {
            let input = self.call(pieces, layer.to_owned(), cx);
            let result = railings_create::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        });
        if !written {
            return;
        }
        let length: f64 = pieces
            .iter()
            .map(|(pts, closed)| {
                kentos_commands::RailingFields {
                    pts: points::wire_all(pts),
                    closed: *closed,
                    ..Default::default()
                }
                .length()
            })
            .sum();
        let f = cx.format();
        let how_many = if pieces.len() > 1 {
            format!("{} parça, toplam ", pieces.len())
        } else {
            String::new()
        };
        let mut words = format!(
            "Korkuluk çizildi: {how_many}{} boyunda, {} yüksek, {}",
            f.length(length),
            f.length(self.height),
            self.infill.label().to_lowercase()
        );
        if self.infill == RailingInfill::Balusters {
            words.push_str(&format!(" (aralık {})", f.length(self.gap)));
        }
        words.push('.');
        let rules = self.rule_words();
        if rules.is_empty() {
            cx.say(Level::Success, words);
        } else {
            cx.say(Level::Warn, format!("{words} {}.", rules.join("; ")));
        }
    }

    /// Writes the line drawn so far, closed or open.
    fn write_line(&mut self, closed: bool, cx: &mut Context<'_>) {
        let pts = std::mem::take(&mut self.pts);
        self.write(&[(pts, closed)], cx);
    }

    /// A setting's value taken; false when it cannot be read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let length =
            |cx: &mut Context<'_>| cx.typed_length(value).filter(|v| v.is_finite() && *v > 0.0);
        match key {
            "S" => match value {
                "draw" => {
                    self.line = Line::Draw;
                    true
                }
                "void" => {
                    self.line = Line::Void;
                    self.pts.clear();
                    true
                }
                _ => false,
            },
            "D" => match value {
                "balusters" => {
                    self.infill = RailingInfill::Balusters;
                    true
                }
                "glass" => {
                    self.infill = RailingInfill::Glass;
                    true
                }
                "solid" => {
                    self.infill = RailingInfill::Solid;
                    true
                }
                _ => false,
            },
            "Y" => length(cx).map(|v| self.height = v).is_some(),
            "E" => length(cx).map(|v| self.width = v).is_some(),
            "A" => length(cx).map(|v| self.gap = v).is_some(),
            "F" => {
                let t = js_trim(value);
                let v = if t.is_empty() {
                    Some(0.0)
                } else {
                    cx.typed_length(t).filter(|v| v.is_finite())
                };
                v.map(|v| self.offset = v).is_some()
            }
            _ => false,
        }
    }
}

impl Tool for RailingTool {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(self.kind())
    }

    fn id(&self) -> &'static str {
        RAILING_ID
    }

    fn label(&self) -> &'static str {
        "Korkuluk"
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
                Ask::Height => {
                    "döşemesinden küpeştenin üstüne yüksekliğini yazın (PAİY 38/1: en az 1,10 m)"
                }
                Ask::Width => "küpeştenin enini yazın",
                Ask::Gap => "çubuklar arasındaki net boşluğu yazın (PAİY 38/4: en çok 0,10 m)",
                Ask::Offset => {
                    "döşemesinin katın döşemesinden kot farkını yazın (0: katın döşemesinde)"
                }
            };
            return Prompt::new("Korkuluk", words).option("Geri", "G");
        }
        let n = |v: f64| v.to_string();
        let step = match (self.line, self.pts.len()) {
            (Line::Void, _) => "galeri boşluğunun içine tıklayın",
            (Line::Draw, 0) => "çizgisinin başına tıklayın",
            (Line::Draw, 1) => "sonraki noktaya tıklayın",
            (Line::Draw, _) => "sonraki noktaya tıklayın ya da Enter ile yazın",
        };
        let line = match self.line {
            Line::Draw => "Çiz",
            Line::Void => "Boşluktan",
        };
        let mut p = Prompt::new("Korkuluk", step)
            .option_with("Çizgi", "S", line)
            .option_with("Yükseklik", "Y", n(self.height))
            .option_with("Dolgu", "D", self.infill.label());
        if self.infill == RailingInfill::Balusters {
            p = p.option_with("Aralık", "A", n(self.gap));
        }
        p = p.option_with("Küpeşte", "E", n(self.width)).option_with(
            "Kot farkı",
            "F",
            n(self.offset),
        );
        if self.line == Line::Draw {
            if self.pts.len() >= 3 {
                p = p.option("Kapat", "K");
            }
            p = p.option("Bitir", "Enter");
        }
        p
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.line == Line::Void {
            match self.round_void(p.world, cx) {
                Ok(pieces) => self.write(&pieces, cx),
                Err(why) => cx.say(Level::Info, why),
            }
            self.see(cx);
            return;
        }
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        self.line == Line::Draw
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.line != Line::Draw {
            return false;
        }
        if self.pts.last() == Some(&p) {
            return true;
        }
        // Its first point again closes it round.
        if self.pts.len() >= 3 && self.pts.first() == Some(&p) {
            self.write_line(true, cx);
            self.see(cx);
            return true;
        }
        self.pts.push(p);
        if self.pts.len() == kentos_commands::MAX_RAILING_POINTS {
            self.write_line(false, cx);
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
                Ask::Height => "Y",
                Ask::Width => "E",
                Ask::Gap => "A",
                Ask::Offset => "F",
            };
            if !self.apply(field, text, cx) {
                cx.say(
                    Level::Warn,
                    "Değer okunamadı; yükseklik, küpeşte eni ve aralık sıfırdan büyük bir uzunluktur.",
                );
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        self.ask = match key.as_str() {
            "S" => {
                let next = if self.line == Line::Draw {
                    "void"
                } else {
                    "draw"
                };
                self.apply("S", next, cx);
                self.see(cx);
                return true;
            }
            "D" => {
                let next = match self.infill {
                    RailingInfill::Balusters => "glass",
                    RailingInfill::Glass => "solid",
                    RailingInfill::Solid => "balusters",
                };
                self.apply("D", next, cx);
                self.see(cx);
                return true;
            }
            "K" if self.line == Line::Draw && self.pts.len() >= 3 => {
                self.write_line(true, cx);
                self.see(cx);
                return true;
            }
            "Y" => Some(Ask::Height),
            "E" => Some(Ask::Width),
            "A" if self.infill == RailingInfill::Balusters => Some(Ask::Gap),
            "F" => Some(Ask::Offset),
            _ => {
                if self.line == Line::Draw
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
        let mut out = vec![
            Setting::choice(
                "S",
                "Çizgi",
                crate::tool_settings::choices(
                    &[("draw", "Çiz"), ("void", "Boşluktan")],
                    if self.line == Line::Draw {
                        "draw"
                    } else {
                        "void"
                    },
                ),
            ),
            Setting::text("Y", "Yükseklik", f.length(self.height)),
            Setting::choice(
                "D",
                "Dolgu",
                crate::tool_settings::choices(
                    &[
                        ("balusters", "Dikey çubuk"),
                        ("glass", "Cam"),
                        ("solid", "Dolu"),
                    ],
                    self.infill.name(),
                ),
            ),
        ];
        if self.infill == RailingInfill::Balusters {
            out.push(Setting::text("A", "Aralık", f.length(self.gap)));
        }
        out.extend([
            Setting::text("E", "Küpeşte", f.length(self.width)),
            Setting::text("F", "Kot farkı", f.length(self.offset)),
            Setting::hint(
                "P",
                "PAİY 38",
                "Balkon, teras, galeri boşluğu ve sahanlıkların boş kenarlarında, döşemeden en az 1,10 m yüksekliğe kadar \
                 korkuluk zorunludur (38/1); boşluklarından en çok 0,10 m çaplı bir cisim geçebilir (38/4). Boşluktan: \
                 döşemedeki boşluğun içine tıklayın; korkuluk kenarı boyunca, döşemenin üstünde yürür.",
            ),
        ]);
        out
    }

    fn cursor_settings(&self) -> &'static [&'static str] {
        &["Y"]
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
            "Değer uygulanmadı: yükseklik, küpeşte eni ve aralık sıfırdan büyük bir uzunluktur.",
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
        if self.line == Line::Draw && self.pts.len() >= 2 {
            self.write_line(false, cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        // Boşluktan's railings are its drawn strokes, one a stretch.
        if self.line == Line::Draw {
            out.path = self.line_now();
        }
        out.tag = self.hover.map(|at| {
            let mut first = format!(
                "Korkuluk {} · {}",
                format.length(self.height),
                self.infill.label().to_lowercase()
            );
            if self.infill == RailingInfill::Balusters {
                first.push_str(&format!(", aralık {}", format.length(self.gap)));
            }
            let mut lines = vec![first];
            lines.extend(self.rule_words());
            if let Some(why) = &self.why {
                lines.push(why.clone());
            }
            Tag { at, lines }
        });
        out.tag_tone = if self.why.is_some() || !self.rule_words().is_empty() {
            Tone::Danger
        } else {
            Tone::Accent
        };
        out
    }
}
