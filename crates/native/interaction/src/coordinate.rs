//! Koordinat oku (`crs.query`, docs/adr/0140): each click reads the point
//! under it, snapped as everywhere, and says it in the message log in the
//! project's formats: `Y=487012.000, X=4420000.000`, with the elevation
//! after them when the snapped point is one that has one, a point object or
//! a vertex of a line, a polyline or an area (docs/adr/0142): `, Z=12.500`.
//! Beside the cursor the same numbers follow it. Nothing is written; Esc,
//! Enter or a quick right click ends.

use kentos_domain::Slot;

use crate::Vec2;
use crate::elevation;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Marker, MarkerShape, Pointer, Preview, Tag, Tone, Tool};
use kentos_geometry_core::store::snap::SnapHit;

/// The tool's id; its command is `crs.query` (a menu command, Harita › Koordinatlar).
pub const ID: &str = "crsQuery";
pub const LABEL: &str = "Koordinat oku";

/// Koordinat oku.
#[derive(Clone, Debug, Default)]
pub struct CrsQuery {
    /// Where the pointer is, and its elevation when it stands on a point that has one.
    hover: Option<(Vec2, Option<f64>)>,
    /// The point read last, ringed.
    last: Option<Vec2>,
    format: Format,
}

impl CrsQuery {
    pub fn new() -> Self {
        Self::default()
    }

    /// The elevation of the point, or of the vertex of a line, a polyline or
    /// an area, a snap stands on (docs/adr/0142).
    fn elevation(snap: Option<SnapHit>, cx: &Context<'_>) -> Option<f64> {
        let hit = snap?;
        if !(0.0..=f64::from(u32::MAX)).contains(&hit.id) {
            return None;
        }
        elevation::elevation_at(cx.doc.get(Slot(hit.id as u32))?, hit.point)
    }
}

impl Tool for CrsQuery {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "okunacak noktaya tıklayın").note("Esc: bitir")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
        self.hover = Some((p.world, Self::elevation(p.snap, cx)));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let f = cx.format();
        self.format = f;
        let z = Self::elevation(p.snap, cx);
        let mut text = format!("Y={}, X={}", f.coord(p.world.x), f.coord(p.world.y));
        if let Some(z) = z {
            text.push_str(&format!(", Z={}", f.length_bare(z)));
        }
        cx.say(Level::Info, text);
        self.last = Some(p.world);
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
        Preview {
            markers: self
                .last
                .map(|at| Marker {
                    at,
                    shape: MarkerShape::Ring(6.0),
                    tone: Tone::Accent,
                })
                .into_iter()
                .collect(),
            tag: self.hover.map(|(at, z)| {
                let mut lines = vec![
                    format!("Y {}", f.coord(at.x)),
                    format!("X {}", f.coord(at.y)),
                ];
                if let Some(z) = z {
                    lines.push(format!("Z {}", f.length_bare(z)));
                }
                Tag { at, lines }
            }),
            ..Preview::default()
        }
    }
}
