//! Renk planı lejandı (docs/adr/8176 §3.3): a click places the colour plan's legend of the active storey's rooms, its
//! top left there, on the legends' layer (MM-XXXX-LEJANT), the old legend taken away: one undo step. Without a colour
//! plan it says where to choose one. The tool leaves once the legend is placed.

use kentos_native_application::color_legend;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::wire;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

pub const ID: &str = "colorLegend";

/// Renk planı lejandı.
#[derive(Clone, Debug, Default)]
pub struct ColorLegendTool {
    hover: Option<Vec2>,
    /// The legend's rows on the active storey (none: no colour plan or no room), as the pointer last moved.
    rows: Option<usize>,
    done: bool,
}

impl ColorLegendTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn level(cx: &Context<'_>) -> Option<String> {
        cx.level.map(str::to_owned)
    }
}

impl Tool for ColorLegendTool {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        color_legend::STEP
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(crate::arch_layer::Kind::Legend)
    }

    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(color_legend::STEP, "lejandın sol üst köşesine tıklayın")
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        if cx.doc.color_plan().is_none() {
            cx.say(Level::Info, "Renk planı kapalı: önce Mahal ▸ Renk planı'ndan bir şema seçin.");
            return Flow::Exit;
        }
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        let level = Self::level(cx);
        self.rows = cx.doc.color_plan().map(|_| cx.doc.color_legend(level.as_deref()).len());
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let level = Self::level(cx);
        let at = wire(p.world);
        let group = cx.doc.begin_group(color_legend::STEP);
        let mut out = Err(String::new());
        let written = crate::arch_layer::writing(&crate::arch_layer::Kind::Legend, cx, |layer, cx| {
            out = color_legend::draw(cx.doc, level.as_deref(), at, layer);
            out.is_ok()
        });
        match (written, out) {
            (true, Ok(n)) => {
                cx.doc.end_group(group);
                cx.say(Level::Success, format!("Renk planı lejandı yerleşti: {n} satır."));
                self.done = true;
            }
            (_, Err(why)) => {
                cx.doc.cancel_group(group);
                if !why.is_empty() {
                    cx.say(Level::Warn, why);
                }
            }
            (false, Ok(_)) => cx.doc.cancel_group(group),
        }
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn settings(&self, _f: &Format) -> Vec<Setting> {
        vec![Setting::hint(
            "P",
            color_legend::STEP,
            "Etkin katın mahallerinin renk planı lejandını tıklanan yere çizer: her renk, mahal sayısı ve alanıyla. \
             Lejant yeniden çizilince eskisinin yerini alır. Renk planı Mahal ▸ Renk planı'ndan seçilir.",
        )]
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            tag: self.hover.map(|at| Tag {
                at,
                lines: vec![match self.rows {
                    Some(0) => "Bu katta mahal yok".to_owned(),
                    Some(n) => format!("Lejant: {n} satır"),
                    None => "Renk planı kapalı".to_owned(),
                }],
            }),
            tag_tone: if self.rows.is_some_and(|n| n > 0) { Tone::Accent } else { Tone::Danger },
            ..Preview::default()
        }
    }
}
