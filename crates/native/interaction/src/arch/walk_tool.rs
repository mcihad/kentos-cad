//! Measures a design route without writing drawing objects.

use kentos_commands::{ArchWalk, ArchWalked, CommandResult};
use kentos_geometry_core::tools::point_text::parse_length;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{
    Context, Flow, Label, Marker, MarkerShape, Pointer, Preview, Stroke, Tone, Tool,
};
use crate::tool_settings::Setting;

pub const ID: &str = "walkRoute";
pub const LABEL: &str = "Dolaşım güzergâhı";

#[derive(Clone, Debug)]
pub struct WalkTool {
    width: f64,
    furniture: bool,
    start: Option<Vec2>,
    end: Option<Vec2>,
    result: Option<ArchWalked>,
}

impl Default for WalkTool {
    fn default() -> Self {
        Self {
            width: 0.90,
            furniture: true,
            start: None,
            end: None,
            result: None,
        }
    }
}

impl WalkTool {
    fn take(&mut self, p: Vec2, cx: &mut Context<'_>) {
        if self.start.is_none() || self.end.is_some() {
            self.start = Some(p);
            self.end = None;
            self.result = None;
        } else {
            self.end = Some(p);
            self.measure(cx);
        }
    }

    fn measure(&mut self, cx: &mut Context<'_>) {
        self.result = None;
        let (Some(start), Some(end)) = (self.start, self.end) else {
            return;
        };
        let wire = |p: Vec2| kentos_commands::Vec2 { x: p.x, y: p.y };
        let input = ArchWalk {
            start: wire(start),
            end: wire(end),
            width: self.width,
            level: cx.level.map(str::to_owned),
            shell_only: !self.furniture,
        };
        let format = cx.format();
        match kentos_native_application::arch_walk::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                if output.blocked.is_some() {
                    cx.say(Level::Warn, format!(
                        "{} genişlikte güzergâh bulunamadı. Kırmızı işareti ve başlangıç/bitiş mahalini inceleyin.",
                        format.length(self.width),
                    ));
                } else {
                    cx.say(
                        Level::Success,
                        format!(
                            "Güzergâh {} · Geçiş profili {}.",
                            format.length(output.length),
                            format.length(self.width),
                        ),
                    );
                }
                self.result = Some(output);
            }
            CommandResult::Failed { error } => cx.say(Level::Warn, error.message),
            _ => {}
        }
    }
}

impl Tool for WalkTool {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            LABEL,
            match (self.start, self.end) {
                (Some(_), None) => "aynı katta bitişi belirtin",
                (Some(_), Some(_)) => "yeni başlangıcı belirtin veya Enter ile bitirin",
                _ => "bir mahal içinde başlangıcı belirtin",
            },
        )
    }

    fn point_count(&self) -> usize {
        usize::from(self.start.is_some()) + usize::from(self.end.is_some())
    }

    fn snaps(&self) -> bool {
        false
    }
    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.take(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.take(p, cx);
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(p) = cx.typed_point(text, self.start, None) else {
            return false;
        };
        self.take(p, cx);
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.result = None;
        self.end.take().is_some() || self.start.take().is_some()
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn document_changed(&mut self, _cx: &Context<'_>) {
        self.start = None;
        self.end = None;
        self.result = None;
    }

    fn settings(&self, format: &Format) -> Vec<Setting> {
        vec![
            Setting::text(
                "width",
                format!("Geçiş genişliği ({})", format.length_unit_label()),
                format.length_bare(self.width),
            ),
            Setting::toggle("furniture", "Donatıları hesaba kat", self.furniture),
            Setting::hint("design", "Kapsam", "Tasarım ölçümü · Yalnız etkin kat"),
        ]
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "width" => {
                let f = cx.format();
                let Some(width) = parse_length(value)
                    .map(|n| n.metres(|v| f.to_metres(v)))
                    .filter(|n| (0.05..=10.0).contains(n))
                else {
                    cx.say(
                        Level::Warn,
                        "Geçiş genişliğini 5 cm ile 10 m arasında yazın.",
                    );
                    return false;
                };
                self.width = width;
            }
            "furniture" => self.furniture = value == "yes",
            _ => return false,
        }
        self.measure(cx);
        true
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut preview = Preview {
            marks: self.start.into_iter().chain(self.end).collect(),
            ..Preview::default()
        };
        let Some(result) = &self.result else {
            return preview;
        };
        if let Some(at) = result.blocked {
            preview.markers.push(Marker {
                at: Vec2::new(at.x, at.y),
                shape: MarkerShape::Cross(10.0),
                tone: Tone::Danger,
            });
            preview.labels.push(Label {
                at: Vec2::new(at.x, at.y),
                text: "Geçişi inceleyin".into(),
                offset: [14.0, 4.0],
                tone: Tone::Danger,
            });
        } else if let Some(at) = self.end {
            preview.strokes.push(
                Stroke::solid(
                    result.points.iter().map(|p| Vec2::new(p.x, p.y)).collect(),
                    false,
                )
                .width(3.0),
            );
            preview.labels.push(Label {
                at,
                text: format!(
                    "{} · Geçiş {}",
                    format.length(result.length),
                    format.length(result.width)
                ),
                offset: [12.0, 4.0],
                tone: Tone::Accent,
            });
        }
        preview
    }
}
