//! Nokta and Kot noktası: the web's `PointTool` without and with an
//! elevation (`askZ`, `apps/web/src/tools/drawTools.ts`) on its
//! `PointInputTool` base, step for step (docs/adr/0032, 0057):
//!
//! - Nokta: every point, clicked (snaps, ortho and polar tracking applied)
//!   or typed (the shared grammar), writes one point object through
//!   `cad.point.create`: its own object and its own undo step; the point's
//!   echo is its message;
//! - Kot noktası: after each point the elevation is typed (clicks wait
//!   meanwhile; “Kot?” beside the point); the point goes on the spot
//!   elevations' layer (`kot`) with the elevation, its text (two decimals)
//!   and its attributes (Tür, Z (m));
//! - neither holds a point, so a confirm (Enter, Space, a quick right click)
//!   leaves;
//! - Ctrl+Z takes the newest point back as an undo while the drawing has not
//!   changed since; with nothing of its own it undoes the drawing.
//!
//! Nothing else is drawn while they run but the cursor and the snap marker,
//! as on the web.

use std::collections::BTreeMap;

use kentos_contracts::PointCreate;
use kentos_geometry_core::tools::point_text::{parse_number, point_from_text};
use kentos_native_application::{ExecutionContext, point};

use crate::Vec2;
use crate::format::{Format, fixed};
use crate::points::{self, Taken, wire};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tag, Tool};

/// The point tool's id: its command is `tool.point`.
pub const ID: &str = "point";
pub const LABEL: &str = "Nokta";
/// The spot elevation tool's id: its command is `tool.spot` (docs/adr/0057).
pub const SPOT_ID: &str = "spot";
pub const SPOT_LABEL: &str = "Kot noktası";
/// The layer spot elevations go on (the web's `LAYERS.spot`, the project template's).
pub const SPOT_LAYER: &str = "kot";

/// The point tool, or with `spot` the spot elevation tool.
#[derive(Clone, Debug, Default)]
pub struct Point {
    d: Taken,
    spot: bool,
    /// Kot noktası: the point waiting for its elevation.
    pending_z: Option<Vec2>,
}

impl Point {
    pub fn new() -> Self {
        Self::default()
    }

    /// Kot noktası: a point, then its elevation.
    pub fn spot() -> Self {
        Self {
            spot: true,
            ..Self::default()
        }
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.spot {
            self.pending_z = Some(p);
        } else {
            self.write(p, None, cx);
        }
    }

    /// Writes one point through the product command `cad.point.create`
    /// (docs/adr/0032): the active layer, or the spot elevations' with its
    /// elevation, text and attributes, explicit in its input (CMD-07); the
    /// desktop has no current colour, so the layer's applies.
    fn write(&mut self, p: Vec2, z: Option<f64>, cx: &mut Context<'_>) {
        let input = PointCreate {
            layer_id: if self.spot {
                SPOT_LAYER.to_owned()
            } else {
                cx.doc.layers().active().to_owned()
            },
            p: wire(p),
            z,
            label: z.map(|z| fixed(z, 2)),
            color: None,
            attrs: z.map(|z| {
                BTreeMap::from([
                    ("Tür".to_owned(), "Kot noktası".to_owned()),
                    ("Z (m)".to_owned(), fixed(z, 3)),
                ])
            }),
            expected_revision: None,
        };
        let result = point::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(written) = points::written(result, cx) {
            self.d.note(written.id, cx);
        }
    }
}

impl Tool for Point {
    fn id(&self) -> &'static str {
        if self.spot { SPOT_ID } else { ID }
    }

    fn label(&self) -> &'static str {
        if self.spot { SPOT_LABEL } else { LABEL }
    }

    fn prompt(&self) -> Prompt {
        let step = if self.pending_z.is_some() {
            "kot değerini yazın (m)"
        } else {
            "nokta konumunu belirtin"
        };
        Prompt::new(self.label(), step)
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    /// While an elevation is asked for, a click waits.
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.pending_z.is_some() {
            return;
        }
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(p) = self.pending_z {
            let Some(z) = parse_number(text) else {
                return false;
            };
            self.write(p, Some(z), cx);
            self.pending_z = None;
            return true;
        }
        match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    /// It never holds a point: a confirm leaves (the web's `PointInputTool.confirm`),
    /// an elevation not yet typed with it.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    /// “Kot?” beside the point waiting for its elevation.
    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            tag: self.pending_z.map(|at| Tag {
                at,
                lines: vec!["Kot?".to_owned()],
            }),
            ..Preview::default()
        }
    }
}
