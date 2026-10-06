//! Resim ekle (docs/adr/0192 §5; the web's `tools/imageInsertTool.ts`):
//!
//! - as the tool starts it asks the host for a PNG or JPEG
//!   (`ViewChange::OpenImageFile`); the host gives it back read, or none
//!   (the picker cancelled; the tool leaves);
//! - then the picture's lower left corner, then a second point: its width
//!   the distance, its turn the direction (the core's `ops::image::placed`),
//!   its height from the picture's own shape; a typed number after the
//!   corner is the width, unturned;
//! - Bağlı (B), where the host knows the file's path: the file's path is
//!   written and its bytes are not kept; otherwise the picture is kept in the
//!   project's library (an edit, no undo step; docs/adr/0092);
//! - one step “Resim ekle” (`cad.entities.create`'s `image`), on the active
//!   layer, and the tool leaves.
//!
//! While it waits for the second point the frame follows the pointer.

use kentos_contracts::{CreateOperation, EntityGeometry, ImageFields, Vec2 as Point};
use kentos_geometry_core::geom::image::Frame;
use kentos_geometry_core::ops::image::placed;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, ImageFile, Pointer, Preview, Stroke, Tag, Tone, Tool, ViewChange,
};

/// The tool's id: its command is `tool.imageInsert`.
pub const ID: &str = "imageInsert";
pub const LABEL: &str = "Resim ekle";

pub struct ImageInsert {
    file: Option<ImageFile>,
    corner: Option<Vec2>,
    hover: Option<Vec2>,
    linked: bool,
    done: bool,
}

impl Default for ImageInsert {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageInsert {
    pub fn new() -> Self {
        Self {
            file: None,
            corner: None,
            hover: None,
            linked: false,
            done: false,
        }
    }

    /// The picture's height over its width.
    fn aspect(&self) -> f64 {
        self.file
            .as_ref()
            .map_or(1.0, |f| f64::from(f.height) / f64::from(f.width.max(1)))
    }

    /// Whether it links the file: Bağlı on, and a path to link.
    fn links(&self) -> bool {
        self.linked && self.file.as_ref().is_some_and(|f| f.path.is_some())
    }

    /// Places the picture from its corner to `q` and leaves; why not otherwise.
    fn write(&mut self, q: Vec2, cx: &mut Context<'_>) {
        let (Some(corner), Some(file)) = (self.corner, self.file.clone()) else {
            return;
        };
        let at = match placed(corner, q, self.aspect()) {
            Ok(at) => at,
            Err(why) => return cx.say(Level::Warn, why),
        };
        let links = self.links();
        if !links {
            // The picture in the project's library, once (docs/adr/0192 §2).
            if !kentos_native_application::edit::has_picture(cx.doc, &file.id) {
                let mut styles = cx.doc.styles().clone();
                styles.items.extend(file.library.items.iter().cloned());
                cx.doc.set_styles(styles);
            }
        }
        let geometry = EntityGeometry::Image(ImageFields {
            p: Point {
                x: corner.x,
                y: corner.y,
            },
            width: at.width,
            height: at.height,
            rotation: at.rotation,
            mirror: false,
            asset: (!links).then(|| file.id.clone()),
            file: if links { file.path.clone() } else { None },
            clip: None,
            opacity: None,
        });
        if points::write_objects(vec![geometry], Some(CreateOperation::Image), cx).is_none() {
            return;
        }
        let f = cx.format();
        cx.say(
            Level::Success,
            format!(
                "{LABEL}: {} yerleştirildi ({} × {}).",
                file.name,
                f.length(at.width),
                f.length(at.height)
            ),
        );
        self.done = true;
    }
}

impl Tool for ImageInsert {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let Some(file) = &self.file else {
            return Prompt::new(LABEL, "resim dosyasını seçin (PNG ya da JPEG)");
        };
        let step = if self.corner.is_none() {
            format!("“{}” için sol alt köşeye tıklayın", file.name)
        } else {
            "genişliği yazın ya da ikinci noktaya tıklayın (yön dönüştür)".to_owned()
        };
        let prompt = Prompt::new(LABEL, step);
        if file.path.is_some() {
            prompt.toggle("Bağlı", "B", self.linked)
        } else {
            prompt
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.corner.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.linked = cx.memory.image_linked;
        cx.view_changes.push(ViewChange::OpenImageFile);
        Flow::Stay
    }

    fn image_given(&mut self, file: Option<ImageFile>, _cx: &mut Context<'_>) {
        match file {
            Some(file) => self.file = Some(file),
            None => self.done = true,
        }
    }

    fn pointer_move(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.hover = Some(p.world);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.file.is_none() {
            return;
        }
        match self.corner {
            None => self.corner = Some(p.world),
            Some(_) => self.write(p.world, cx),
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        if self.file.is_none() {
            return false;
        }
        if upper_tr(t) == "B" && self.file.as_ref().is_some_and(|f| f.path.is_some()) {
            self.linked = !self.linked;
            cx.memory.image_linked = self.linked;
            return true;
        }
        let Some(corner) = self.corner else {
            return false;
        };
        match cx.typed_length(t) {
            Some(w) if w > 0.0 && w.is_finite() => {
                self.write(Vec2::new(corner.x + w, corner.y), cx);
                true
            }
            _ => false,
        }
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc: the corner goes first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.corner.take().is_some()
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = Preview::default();
        let (Some(corner), Some(hover)) = (self.corner, self.hover) else {
            return out;
        };
        let Ok(at) = placed(corner, hover, self.aspect()) else {
            return out;
        };
        let frame = Frame::new(corner, at.width, at.height, at.rotation, false);
        let [a, b, c, d] = frame.corners();
        out.strokes
            .push(Stroke::dashed(vec![a, b, c, d], true, [6.0, 4.0]).tone(Tone::Accent));
        // Its diagonals, as a picture's place is drawn before its pixels.
        out.strokes
            .push(Stroke::solid(vec![a, c], false).tone(Tone::Snap));
        out.strokes
            .push(Stroke::solid(vec![b, d], false).tone(Tone::Snap));
        out.tag = Some(Tag {
            at: hover,
            lines: vec![format!(
                "{} × {}",
                format.length(at.width),
                format.length(at.height)
            )],
        });
        out
    }
}
