//! Çizgiden duvar (docs/adr/0339 §6; `CAD-ARCH-09`): walls from the selected line work, selection first as the modify
//! tools (`modify`).
//!
//! - Tek tek: a wall along each line, polyline, area's ring, arc and circle, of Duvar's width, justification and
//!   style (`Memory::wall_*`); Çiftler (Ç): two parallel lines at most En çok genişlik (E in that mode) apart make
//!   the wall between them, as wide as they are apart;
//! - Kaynağı koru (K): the lines stay; else those a wall covers whole go;
//! - the walls to come are shown from the command's plan, the lines left beside the cursor; Enter, Uygula or a quick
//!   right click writes them through `cad.walls.convert` in one undo step “Çizgiden duvar”.

use kentos_commands::{
    CommandResult, WallJustify, WallRectangleReference, WallsFromLines, WallsFromMode,
};
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, walls_from_lines};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory, OptionChoice, Pointer, Preview, Tag};
use crate::wall::{self, Ask, Drawn};

pub const ID: &str = "wallFromLines";
pub const LABEL: &str = "Çizgiden duvar";

/// What was planned, and what for: the drawing's generation and the options.
#[derive(Clone, Debug, Default)]
struct Plan {
    key: (u64, [u64; 7]),
    drawn: Drawn,
    walls: usize,
    left: usize,
    removed: usize,
    refused: Option<String>,
    revision: Option<String>,
    shared_note: Option<String>,
}

#[derive(Default)]
pub struct WallFromLines {
    uids: Vec<String>,
    ask: Option<Ask>,
    hover: Option<Vec2>,
    plan: Plan,
    seen: Memory,
    format: Format,
    offer: Vec<kentos_commands::WallStyleDef>,
}

/// The options a plan depends on, as numbers.
fn options_key(m: &Memory) -> [u64; 7] {
    [
        m.wall_width.to_bits(),
        m.wall_justify as u64,
        m.wall_style.unwrap_or(0),
        u64::from(m.wall_pairs),
        u64::from(m.wall_contours),
        u64::from(m.wall_keep_sources),
        m.wall_pair_max.to_bits(),
    ]
}

impl WallFromLines {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    fn request(&self, cx: &Context<'_>) -> WallsFromLines {
        let m = &*cx.memory;
        let style = wall::chosen(m, cx.doc.settings());
        let fixed = style
            .as_ref()
            .and_then(|s| kentos_commands::fixed_width(&s.components))
            .is_some();
        WallsFromLines {
            uids: self.uids.clone(),
            mode: if m.wall_contours {
                WallsFromMode::Contours
            } else if m.wall_pairs {
                WallsFromMode::Pairs
            } else {
                WallsFromMode::Each
            },
            width: (!m.wall_pairs && !fixed).then_some(m.wall_width),
            justify: (!m.wall_pairs && m.wall_justify != WallJustify::Center)
                .then_some(m.wall_justify),
            style: style.map(|s| s.id),
            keep_sources: m.wall_keep_sources,
            max_width: m.wall_pairs.then_some(m.wall_pair_max),
            angle: None,
            layer_id: None,
            expected_revision: if m.wall_contours {
                self.plan.revision.clone()
            } else {
                None
            },
        }
    }

    fn replan(&mut self, cx: &Context<'_>) {
        let key = (cx.doc.generation(), options_key(cx.memory));
        if self.uids.is_empty() || self.plan.key == key {
            return;
        }
        let mut plan = Plan {
            key,
            revision: Some(cx.doc.revision().to_string()),
            ..Plan::default()
        };
        let mut input = self.request(cx);
        input.expected_revision = None;
        match walls_from_lines::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, warnings } => {
                plan.shared_note = warnings
                    .iter()
                    .find(|w| w.code == "wall_shared_boundary")
                    .map(|w| w.message.clone());
                plan.walls = output.walls.len();
                plan.left = output.left.len();
                plan.removed = output.removed.len();
                if plan.shared_note.is_some() {
                    wall::add_joined(&output.walls, &mut plan.drawn);
                } else {
                    for e in output.walls.iter().take(crate::modify::MAX_GHOSTS) {
                        wall::add(e, &mut plan.drawn);
                    }
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                plan.refused = Some(error.message);
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
        self.plan = plan;
    }

    fn finding(&self) -> String {
        let p = &self.plan;
        if let Some(why) = &p.refused {
            return why.clone();
        }
        let mut out = format!("{} duvar", p.walls);
        if self.seen.wall_contours && self.seen.wall_keep_sources {
            out.push_str("; kaynaklar korunur");
        }
        if p.removed > 0 {
            out.push_str(&format!("; {} çizgi silinir", p.removed));
        }
        if p.left > 0 {
            out.push_str(&format!("; {} çizgi kalır", p.left));
        }
        if let Some(note) = &p.shared_note {
            out.push_str("; ");
            out.push_str(note);
        }
        out
    }
}

impl Stages for WallFromLines {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = *cx.memory;
        self.format = cx.format();
        self.offer = wall::offered(cx.doc.settings());
        self.replan(cx);
        if cx.memory.wall_contours && self.hover.is_none() {
            self.hover = cx
                .selection
                .ids()
                .iter()
                .find_map(|s| cx.doc.get(*s))
                .and_then(walls_from_lines::path_of)
                .and_then(|(pts, _, _)| pts.first().map(|p| Vec2::new(p.x, p.y)));
        }
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.ask = None;
        self.plan = Plan::default();
        self.uids = cx
            .selection
            .ids()
            .iter()
            .filter_map(|&s| cx.doc.uid(s).map(|u| u.to_string()))
            .collect();
        if self.uids.is_empty() {
            cx.say(Level::Warn, "Çizgiden duvar: önce çizgileri seçin.");
            return Flow::Exit;
        }
        self.see(cx);
        Flow::Stay
    }

    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        prompt.note("duvara çevrilecek çizgileri seçin")
    }

    fn snaps(&self) -> bool {
        false
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        if let Some(ask) = self.ask {
            return match ask {
                Ask::Width if self.seen.wall_pairs => Prompt::new(
                    LABEL,
                    "iki çizginin en çok ne kadar arayla duvar olacağını yazın",
                )
                .option("Geri", "G"),
                ask => wall::asked(ask),
            };
        }
        let m = &self.seen;
        let style = wall::chosen_in(m, &self.offer);
        let method = if m.wall_contours {
            "Kapalı kontur"
        } else if m.wall_pairs {
            "Çiftler"
        } else {
            "Tek tek"
        };
        let reference = WallRectangleReference::ALL
            .into_iter()
            .find(|r| r.justify() == m.wall_justify)
            .unwrap_or_default();
        let prompt = Prompt::new(LABEL, self.finding()).option_with("Yöntem", "M", method);
        let prompt = if m.wall_pairs {
            prompt
                .option_with("En çok genişlik", "E", self.format.length(m.wall_pair_max))
                .option_with(
                    "Stil",
                    "S",
                    style.map_or(wall::NO_STYLE.to_owned(), |s| s.name.clone()),
                )
        } else {
            prompt
                .option_with("Genişlik", "E", self.format.length(wall::width(m, style)))
                .option_with(
                    if m.wall_contours { "Referans" } else { "Hiza" },
                    "H",
                    if m.wall_contours {
                        reference.label()
                    } else {
                        m.wall_justify.label()
                    },
                )
                .option_with(
                    "Stil",
                    "S",
                    style.map_or(wall::NO_STYLE.to_owned(), |s| s.name.clone()),
                )
        };
        prompt
            .toggle("Kaynağı koru", "K", m.wall_keep_sources)
            .option("Uygula", "Enter")
    }

    fn pointer(&mut self, p: &Pointer, down: bool, _cx: &mut Context<'_>) -> Option<Flow> {
        if down {
            return Some(Flow::Stay);
        }
        self.hover = Some(p.raw);
        None
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        let key = upper_tr(js_trim(text));
        if let Some(ask) = self.ask {
            if key == "G" {
                self.ask = None;
                return Some(Flow::Stay);
            }
            if ask == Ask::Style {
                self.ask = None;
                wall::choose("S", text, cx);
            } else if let Some((_, metres)) = points::plain_value(text, cx) {
                self.ask = None;
                if ask == Ask::Width && cx.memory.wall_pairs {
                    if metres > 0.0 && metres <= 100.0 {
                        cx.memory.wall_pair_max = metres;
                    } else {
                        cx.say(
                            Level::Warn,
                            "En çok genişlik sıfırdan büyük ve en çok 100 m olmalı.",
                        );
                    }
                } else {
                    wall::typed(ask, metres, cx);
                }
            } else {
                cx.say(Level::Warn, "Bir sayı yazın.");
            }
            self.see(cx);
            return Some(Flow::Stay);
        }
        match key.as_str() {
            "Ç" | "C" => {
                cx.memory.wall_pairs = !cx.memory.wall_pairs;
                cx.memory.wall_contours = false;
            }
            "M" => {
                let next = if cx.memory.wall_contours {
                    "each"
                } else if cx.memory.wall_pairs {
                    "contours"
                } else {
                    "pairs"
                };
                self.configure("M", next, cx);
            }
            "K" => cx.memory.wall_keep_sources = !cx.memory.wall_keep_sources,
            "H" if !cx.memory.wall_pairs => wall::next_justify(cx),
            "E" | "S" => self.ask = wall::ask_of(&key),
            _ => return None,
        }
        self.see(cx);
        Some(Flow::Stay)
    }

    fn typed_points(&self) -> bool {
        false
    }

    fn takes_points(&self) -> bool {
        false
    }

    fn takes_words(&self) -> bool {
        self.ask == Some(Ask::Style)
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if self.ask.is_some() || (key == "H" && self.seen.wall_pairs) {
            return Vec::new();
        }
        if key == "M" || (key == "H" && self.seen.wall_contours) || key == "K" {
            return self
                .settings(&self.format)
                .into_iter()
                .find(|s| s.key == key)
                .map_or_else(Vec::new, |s| s.choices);
        }
        wall::choices(key, &self.seen, &self.offer)
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key == "M" || key == "K" || (key == "H" && cx.memory.wall_contours) {
            return self.configure(key, typed, cx);
        }
        if !matches!(key, "H" | "S") {
            return false;
        }
        self.ask = None;
        wall::choose(key, typed, cx);
        self.see(cx);
        true
    }

    fn settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::{Setting, choices};
        let m = &self.seen;
        let mut rows = vec![Setting::choice(
            "M",
            "Yöntem",
            choices(
                &[
                    ("each", "Tek tek"),
                    ("pairs", "Çiftler"),
                    ("contours", "Kapalı kontur"),
                ],
                if m.wall_contours {
                    "contours"
                } else if m.wall_pairs {
                    "pairs"
                } else {
                    "each"
                },
            ),
        )];
        if m.wall_pairs {
            rows.push(Setting::text(
                "E",
                "En çok genişlik",
                f.length(m.wall_pair_max),
            ));
            rows.push(Setting::choice(
                "S",
                "Stil",
                wall::choices("S", m, &self.offer),
            ));
        } else {
            rows.extend(
                crate::tool_settings::wall_rows(m, &self.offer, f)
                    .into_iter()
                    .filter(|s| s.key != "Ö" && s.key != "H"),
            );
            rows.push(if m.wall_contours {
                let selected = WallRectangleReference::ALL
                    .into_iter()
                    .find(|r| r.justify() == m.wall_justify)
                    .unwrap_or_default();
                Setting::choice(
                    "H",
                    "Referans",
                    choices(
                        &[
                            ("inner", "İç net"),
                            ("outer", "Dış ölçü"),
                            ("center", "Eksen"),
                        ],
                        selected.name(),
                    ),
                )
            } else {
                Setting::choice("H", "Hiza", wall::choices("H", m, &self.offer))
            });
        }
        rows.push(Setting::choice(
            "K",
            "Kaynak",
            choices(
                &[("keep", "Koru"), ("remove", "Sil")],
                if m.wall_keep_sources {
                    "keep"
                } else {
                    "remove"
                },
            ),
        ));
        rows
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let canonical = self
            .settings(&cx.format())
            .into_iter()
            .find(|s| s.key == key)
            .and_then(|s| {
                s.choices.into_iter().find(|c| {
                    upper_tr(&c.typed) == upper_tr(value.trim())
                        || upper_tr(&c.label) == upper_tr(value.trim())
                })
            })
            .map(|c| c.typed);
        let value = canonical.as_deref().unwrap_or(value);
        self.ask = None;
        let taken = match (key, value) {
            ("M", "each" | "pairs" | "contours") => {
                cx.memory.wall_pairs = value == "pairs";
                let was_contours = cx.memory.wall_contours;
                cx.memory.wall_contours = value == "contours";
                if value == "contours" && !was_contours {
                    cx.memory.wall_justify = WallRectangleReference::Inner.justify();
                }
                true
            }
            ("K", "keep" | "remove") => {
                cx.memory.wall_keep_sources = value == "keep";
                true
            }
            ("H", _) if cx.memory.wall_contours => match WallRectangleReference::ALL
                .into_iter()
                .find(|r| r.name() == value)
            {
                Some(r) => {
                    cx.memory.wall_justify = r.justify();
                    true
                }
                None => false,
            },
            ("E", _) if cx.memory.wall_pairs => match cx
                .typed_length(value)
                .filter(|n| n.is_finite() && *n > 0.0 && *n <= 100.0)
            {
                Some(n) => {
                    cx.memory.wall_pair_max = n;
                    true
                }
                None => false,
            },
            ("E" | "S" | "H", _) => crate::tool_settings::wall_apply(key, value, cx),
            _ => false,
        };
        if !taken {
            cx.say(
                Level::Warn,
                "Değer uygulanmadı; geçerli bir yöntem, referans veya pozitif uzunluk seçin.",
            );
        }
        self.see(cx);
        taken
    }

    /// Enter: the walls written in one step, and the tool leaves; a refusal is said and the tool stays.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.ask.is_some() {
            self.ask = None;
            return Flow::Stay;
        }
        let input = self.request(cx);
        let result = walls_from_lines::execute(&mut ExecutionContext::new(cx.doc), input);
        match points::written(result, cx) {
            Some(done) => {
                let mut said = format!("{LABEL}: {} duvar eklendi", done.uids.len());
                if !done.removed.is_empty() {
                    said.push_str(&format!(", {} çizgi silindi", done.removed.len()));
                }
                if !done.left.is_empty() {
                    said.push_str(&format!(", {} çizgi duvara çevrilmedi", done.left.len()));
                }
                cx.say(Level::Success, format!("{said}."));
                Flow::Exit
            }
            None => Flow::Stay,
        }
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    fn stage_preview(&self, hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        let tag = hover.map(|at| Tag {
            at,
            lines: if self.seen.wall_contours {
                let mut lines = vec![String::new()];
                for word in self.finding().split_whitespace() {
                    if lines
                        .last()
                        .is_some_and(|s| s.chars().count() + word.chars().count() > 62)
                    {
                        lines.push(String::new());
                    }
                    if let Some(last) = lines.last_mut() {
                        if !last.is_empty() {
                            last.push(' ');
                        }
                        last.push_str(word);
                    }
                }
                lines.push("Önizleme · Enter: uygula · Esc: vazgeç".into());
                lines
            } else {
                vec![self.finding(), "Enter: uygula".to_owned()]
            },
        });
        Some(Preview {
            areas: self.plan.drawn.areas.clone(),
            strokes: self.plan.drawn.strokes.clone(),
            tag,
            tag_tone: if self.seen.wall_contours && self.plan.refused.is_some() {
                crate::tool::Tone::Danger
            } else {
                crate::tool::Tone::Accent
            },
            ..Preview::default()
        })
    }
}
