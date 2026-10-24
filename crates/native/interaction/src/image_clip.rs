//! Resmi kırp (docs/adr/0192 §5, 0622 §9; the web's `tools/imageClipTool.ts`): a picture's edge is clicked; its
//! boundary is drawn as a rectangle by two corners (Dikdörtgen, D) or as a polygon by its corners and Enter (Çokgen,
//! Ç); Kaldır (K) takes the clip away. What is drawn goes to `cad.images.clip`, which cuts it to the picture in the
//! picture's own fractions and writes one step “Resmi kırp”, the picture in its place; the tool waits for the next
//! picture. Esc and Ctrl+Z take back the last corner, then the picture; Esc with none leaves.

use kentos_commands::{Entity, ImageClipBy, ImagesClip, Vec2 as Wire};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::image::shown;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, images_clip};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tone, Tool};

/// The tool's id: its command is `tool.imageClip`.
pub const ID: &str = "imageClip";
pub const LABEL: &str = "Resmi kırp";
/// A click on no picture.
pub const NO_PICTURE: &str =
    "Tıklanan yerde kilitsiz bir resim yok; kırpılacak resmin kenarına tıklayın.";

/// A point as the command takes it.
fn wire(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

/// A picture not on a locked layer: what the tool clips.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(e, Entity::Image(_)) && edge::unlocked(e, doc)
}

pub struct ImageClip {
    picture: Option<(Slot, Shape)>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    polygon: bool,
}

impl Default for ImageClip {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageClip {
    pub fn new() -> Self {
        Self {
            picture: None,
            pts: Vec::new(),
            hover: None,
            polygon: false,
        }
    }

    /// Writes what was drawn through `cad.images.clip` (docs/adr/0622 §9) and waits for the next picture; a refusal
    /// is said and the boundary is drawn again.
    fn write(&mut self, clip: ImageClipBy, cx: &mut Context<'_>) {
        let Some((slot, _)) = &self.picture else {
            return;
        };
        let removed = matches!(clip, ImageClipBy::Remove);
        let input = ImagesClip {
            uid: edge::uid(cx.doc, *slot),
            clip,
            expected_revision: None,
        };
        let result = images_clip::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_none() {
            self.pts.clear();
            return;
        }
        cx.say(
            Level::Success,
            if removed {
                format!("{LABEL}: kırpma kaldırıldı.")
            } else {
                format!("{LABEL}: resim kırpıldı.")
            },
        );
        self.picture = None;
        self.pts.clear();
    }

    /// The rectangle of two corners.
    fn rectangle(a: Vec2, b: Vec2) -> Vec<Vec2> {
        vec![a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)]
    }
}

impl Tool for ImageClip {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let Some((_, s)) = &self.picture else {
            return Prompt::new(LABEL, "kırpılacak resmin kenarına tıklayın");
        };
        let step = match (self.polygon, self.pts.len()) {
            (false, 0) => "sınırın ilk köşesine tıklayın",
            (false, _) => "sınırın karşı köşesine tıklayın",
            (true, n) if n < 3 => "sınırın köşelerine tıklayın",
            (true, _) => "sonraki köşeye tıklayın ya da Enter ile bitirin",
        };
        let prompt = Prompt::new(LABEL, step)
            .toggle("Dikdörtgen", "D", !self.polygon)
            .toggle("Çokgen", "Ç", self.polygon);
        if matches!(s, Shape::Image { clip: Some(_), .. }) {
            prompt.option("Kaldır", "K")
        } else {
            prompt
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.picture.is_some()) + self.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.polygon = cx.memory.image_clip_polygon;
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        self.picture.is_some()
    }

    fn cursor(&self) -> Cursor {
        if self.picture.is_some() {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        if self.picture.is_none() {
            edge::hover(p, cx, editable);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.picture.is_none() {
            match edge::pick_said(p, cx, editable)
                .and_then(|slot| Some((slot, shape(cx.doc.get(slot)?))))
            {
                Some(found) => {
                    self.picture = Some(found);
                    cx.selection.set_hover(None);
                }
                None => cx.say(Level::Warn, NO_PICTURE),
            }
            return;
        }
        self.pts.push(p.world);
        if !self.polygon && self.pts.len() == 2 {
            let clip = ImageClipBy::Rectangle {
                a: wire(self.pts[0]),
                b: wire(self.pts[1]),
            };
            self.write(clip, cx);
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.picture.is_none() {
            return false;
        }
        match upper_tr(js_trim(text)).as_str() {
            "D" => {
                self.polygon = false;
                self.pts.clear();
            }
            "Ç" | "C" => {
                self.polygon = true;
                self.pts.clear();
            }
            "K" if matches!(&self.picture, Some((_, Shape::Image { clip: Some(_), .. }))) => {
                self.write(ImageClipBy::Remove, cx);
            }
            _ => return false,
        }
        cx.memory.image_clip_polygon = self.polygon;
        true
    }

    /// Enter: a polygon of three corners or more is the boundary; else the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.polygon && self.pts.len() >= 3 {
            let points = std::mem::take(&mut self.pts)
                .into_iter()
                .map(wire)
                .collect();
            self.write(ImageClipBy::Polygon { points }, cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Esc: the last corner, then the picture go first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.pts.pop().is_some() {
            return true;
        }
        self.picture.take().is_some()
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = Preview::default();
        let Some((_, s)) = &self.picture else {
            return out;
        };
        // The picture's part shown now, then the boundary drawn so far.
        out.strokes
            .push(Stroke::solid(shown(s), true).width(2.0).tone(Tone::Snap));
        let mut ring = self.pts.clone();
        if let Some(h) = self.hover {
            if !self.polygon && ring.len() == 1 {
                ring = Self::rectangle(ring[0], h);
            } else if !ring.is_empty() {
                ring.push(h);
            }
        }
        if ring.len() >= 2 {
            out.strokes
                .push(Stroke::dashed(ring, true, [6.0, 4.0]).tone(Tone::Accent));
        }
        out
    }
}
