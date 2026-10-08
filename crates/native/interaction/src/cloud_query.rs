//! Nokta bulutu XYZ sor (`pointcloud.query`, docs/adr/0207 §7): each click
//! asks the host for the nearest point of the shown point clouds within 8
//! pixels of it, at full resolution ([`ViewChange::CloudQuery`]); the host
//! reads the nodes there off the interface's thread, says the point's facts
//! on the command line and gives the point back ([`Tool::cloud_found`]),
//! which the tool marks with its height beside it. Nothing is written; Esc,
//! Enter or a quick right click ends.

use crate::Vec2;
use crate::format::Format;
use crate::prompt::Prompt;
use crate::tool::{
    Context, Flow, Marker, MarkerShape, Pointer, Preview, Tag, Tone, Tool, ViewChange,
};

/// The tool's id; its command is `pointcloud.query`.
pub const ID: &str = "cloudQuery";
pub const LABEL: &str = "Nokta bulutu XYZ sor";
/// How near the click a point is looked for, in logical pixels.
pub const REACH_PX: f64 = 8.0;

/// Nokta bulutu XYZ sor.
#[derive(Clone, Debug, Default)]
pub struct CloudQuery {
    /// The place clicked last.
    asked: Option<Vec2>,
    /// The point the host found for it.
    found: Option<[f64; 3]>,
    format: Format,
}

impl CloudQuery {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Tool for CloudQuery {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "sorgulanacak yere tıklayın").note("Esc: bitir")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        Flow::Stay
    }

    fn pointer_move(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
        self.asked = Some(p.world);
        self.found = None;
        cx.view_changes.push(ViewChange::CloudQuery {
            at: p.world,
            reach: cx.view.world_length(REACH_PX),
        });
    }

    fn cloud_found(&mut self, found: Option<[f64; 3]>, _cx: &mut Context<'_>) {
        self.found = found;
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        let f = self.format;
        let mut markers: Vec<Marker> = self
            .asked
            .map(|at| Marker {
                at,
                shape: MarkerShape::Circle(REACH_PX as f32),
                tone: Tone::Snap,
            })
            .into_iter()
            .collect();
        let found = self.found.map(|[x, y, _]| Vec2::new(x, y));
        if let Some(at) = found {
            markers.push(Marker {
                at,
                shape: MarkerShape::Ring(5.0),
                tone: Tone::Accent,
            });
        }
        Preview {
            markers,
            tag: self.found.map(|[x, y, z]| Tag {
                at: Vec2::new(x, y),
                lines: vec![
                    format!("{} {}", f.east_label(), f.coord(x)),
                    format!("{} {}", f.north_label(), f.coord(y)),
                    format!("Z {}", f.length_bare(z)),
                ],
            }),
            ..Preview::default()
        }
    }
}
