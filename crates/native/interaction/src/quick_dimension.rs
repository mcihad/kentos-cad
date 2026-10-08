//! Hızlı ölçü (docs/adr/0147 §7, AutoCAD's QDIM; the web's `QuickDimensionTool`,
//! `apps/web/src/tools/quickDimensionTool.ts`): the selected lines, polylines
//! and areas measured at once, an aligned dimension along each straight edge
//! and an arc length along each arc; out of an area, into a hole, on the
//! cursor's side of an open path; an edge two objects share measured once.
//! The shared core gives them (`quick_dimensions`), in the drawing's order.
//!
//! Selection first, as the modify tools (`modify`); then the cursor shows
//! where they go (its distance to the nearest edge, no snapping) or a
//! distance is typed, and all of them are written in one step “Ekle” through
//! `cad.entities.create`. Zemin (Z) is Ölçülendirme's (`Memory::dimension_mask`).

use kentos_contracts::{DimensionStyle, EntityGeometry};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::geom::dimension::{QuickDimensions, layout_dimension, quick_dimensions};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{core_look, shape};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{MAX_GHOSTS, Modify, Stages};
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::styles;
use crate::tool::{Context, Flow, OptionChoice, Pointer, Preview, Stroke, Tag};

pub const ID: &str = "quickDimension";
const LABEL: &str = "Hızlı ölçü";
/// What is said when the selection has nothing to measure.
const NOTHING: &str = "Seçimde ölçülecek çizgi, çoklu çizgi ya da alan yok.";

#[derive(Default)]
pub struct QuickDimension {
    /// The selection in the drawing's order, as the core reads it.
    shapes: Vec<Shape>,
    /// The cursor as it is, no snap (the web's `hover`).
    hover: Option<Vec2>,
    height: f64,
    mask: bool,
    /// The dimension style's look (docs/adr/0183 §4); none for Standart.
    look: kentos_contracts::DimensionLook,
    /// The dimension styles as of the last call: a CAD project's Stil.
    styles: styles::Seen,
    /// Stil: a dimension style's name is being typed.
    asking_style: bool,
}

impl QuickDimension {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The dimensions at `at`, in the style's look.
    fn quick(&self, at: Vec2, typed: Option<f64>) -> QuickDimensions {
        let mut q = quick_dimensions(&self.shapes, at, typed, self.height);
        let look = core_look(&self.look);
        for d in &mut q.dimensions {
            d.look = look.clone();
        }
        q
    }

    /// Writes them all in one step and leaves.
    fn write(&mut self, at: Vec2, typed: Option<f64>, cx: &mut Context<'_>) -> Flow {
        let QuickDimensions {
            dimensions,
            skipped,
        } = self.quick(at, typed);
        if dimensions.is_empty() {
            cx.say(Level::Warn, NOTHING);
            return Flow::Exit;
        }
        let mask = cx.memory.dimension_mask;
        let geometries = dimensions
            .iter()
            .map(|d| EntityGeometry::Dimension {
                a: wire(d.a),
                b: wire(d.b),
                offset: d.offset,
                height: d.height,
                text: None,
                style: d.style.as_deref().and_then(DimensionStyle::from_name),
                angle: None,
                c: d.c.map(wire),
                mask,
                za: None,
                zb: None,
                look: self.look.clone(),
            })
            .collect();
        if let Some(out) = points::write_objects(geometries, None, cx) {
            let left = if skipped > 0 {
                format!("; {skipped} nesne atlandı (yalnız çizgi, çoklu çizgi ve alan ölçülür)")
            } else {
                String::new()
            };
            cx.say(
                Level::Success,
                format!("Hızlı ölçü: {} ölçü eklendi{left}.", out.created.len()),
            );
        }
        Flow::Exit
    }
}

impl Stages for QuickDimension {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn see(&mut self, cx: &Context<'_>) {
        let (look, height) =
            styles::dimension_look(cx, styles::standard_dimension_height(cx.doc.settings()));
        self.height = height;
        self.look = look;
        self.styles = styles::Seen::dimension(cx);
        self.mask = cx.memory.dimension_mask;
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        let mut slots: Vec<Slot> = cx.selection.ids().to_vec();
        slots.sort();
        self.shapes = slots
            .iter()
            .filter_map(|&s| cx.doc.get(s))
            .map(shape)
            .collect();
        let measured = |s: &Shape| {
            matches!(
                s,
                Shape::Line { .. } | Shape::Polyline { .. } | Shape::Polygon { .. }
            )
        };
        if !self.shapes.iter().any(measured) {
            cx.say(Level::Warn, NOTHING);
            return Flow::Exit;
        }
        Flow::Stay
    }

    /// The cursor's distance to the nearest edge places them: a snap would put it on an edge.
    fn snaps(&self) -> bool {
        false
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        if self.asking_style {
            return Prompt::new(LABEL, "ölçü stilini menüden seçin ya da adını yazın").option_with(
                "Stil",
                "S",
                self.styles.chosen.clone(),
            );
        }
        Prompt::new(LABEL, "ölçülerin yerini gösterin ya da uzaklık yazın")
            .option_if(self.styles.shown, "Stil", "S", self.styles.chosen.clone())
            .option_with("Zemin", "Z", if self.mask { "açık" } else { "kapalı" })
    }

    /// Stil's menu: Standart, the project's dimension styles and their window (docs/adr/0183 §4).
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key == "S" && self.styles.shown {
            return self
                .styles
                .choices(styles::DIMENSION_STYLES_ENTRY, styles::DIMENSION_STYLES);
        }
        Vec::new()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key != "S" || !styles::shown(cx.doc.settings()) {
            return false;
        }
        if styles::take_dimension(typed, cx) {
            self.asking_style = false;
        }
        self.see(cx);
        true
    }

    /// The cursor as it is, for a typed distance (an open path's side is the cursor's).
    fn pointer(&mut self, p: &Pointer, down: bool, _cx: &mut Context<'_>) -> Option<Flow> {
        if !down {
            self.hover = Some(p.raw);
        }
        None
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        self.write(p, None, cx)
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        if self.asking_style {
            // A name the project has none of is said; the tool waits for another.
            if styles::take_dimension(js_trim(text), cx) {
                self.asking_style = false;
            }
            return Some(Flow::Stay);
        }
        if upper_tr(js_trim(text)) == "Z" {
            cx.memory.dimension_mask = !cx.memory.dimension_mask;
            return Some(Flow::Stay);
        }
        if upper_tr(js_trim(text)) == "S" && styles::shown(cx.doc.settings()) {
            self.asking_style = true;
            return Some(Flow::Stay);
        }
        let n = points::plain_length(text, cx)?;
        // Without a cursor, the origin gives an open path's side (the web's).
        let at = self.hover.unwrap_or(Vec2::new(0.0, 0.0));
        Some(self.write(at, Some(n), cx))
    }

    /// Enter while a style's name is asked for: back to placing them.
    /// A style's name is words: Space types a space (docs/adr/0183 §4).
    fn takes_words(&self) -> bool {
        self.asking_style
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.asking_style {
            self.asking_style = false;
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// A typed number is a distance, never a point.
    fn typed_points(&self) -> bool {
        false
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    /// The dimensions where the cursor would put them (as the modify tools'
    /// ghosts, a very large selection previews its first ones), their count
    /// and distance beside the cursor.
    fn stage_preview(&self, hover: Option<Vec2>, format: &Format) -> Option<Preview> {
        let hover = hover?;
        let QuickDimensions { dimensions, .. } = self.quick(hover, None);
        let strokes = dimensions
            .iter()
            .take(MAX_GHOSTS)
            .filter_map(layout_dimension)
            .flat_map(|l| {
                // A style's filled arrowheads and dots, outlined (docs/adr/0183 §3).
                let fills = l
                    .fills
                    .into_iter()
                    .flatten()
                    .map(|ring| Stroke::solid(ring, true));
                l.lines
                    .into_iter()
                    .map(|[p, q]| Stroke::solid(vec![p, q], false))
                    .chain(fills)
                    .collect::<Vec<_>>()
            })
            .collect();
        let tag = dimensions.first().map(|d| Tag {
            at: hover,
            lines: vec![
                format!("{} ölçü", dimensions.len()),
                format.length(d.offset.abs()),
            ],
        });
        Some(Preview {
            strokes,
            tag,
            ..Preview::default()
        })
    }
}
