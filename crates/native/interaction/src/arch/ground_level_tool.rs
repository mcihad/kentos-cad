//! Zemin kotu (docs/adr/8174 §7; EK-4 4.30, Şekil 4.6): a click writes the site's ground levels at a place, the
//! natural ground's (tzk) over the graded ground's (tsk), each absolute and, in brackets, from ±0.00, beside a plan
//! level symbol (a circle and its cross); on the symbols' layer (MM-XXXX-SEMBOL), one undo step. The values are
//! written as text: a ground made again later leaves the marks as they are. The grounds are made once and kept while
//! the drawing and the site's grounds stay the same, so the preview reads them as the pointer moves.

use std::rc::Rc;

use kentos_commands::{EntitiesCreate, EntityGeometry, NewObject, SiteGround, TextAlign};
use kentos_native_application::site_ground::{self, SiteSurfaces};
use kentos_native_application::{ExecutionContext, create};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

pub const ID: &str = "groundLevel";

/// The attribute the mark's objects carry: what each is (tzk, tsk, simge).
pub const MARK_ATTR: &str = "Zemin kotu";

/// The grounds made for a drawing's revision and the site's grounds then.
#[derive(Debug)]
struct Made {
    revision: u64,
    grounds: (Option<SiteGround>, Option<SiteGround>),
    surfaces: SiteSurfaces,
}

/// Zemin kotu.
#[derive(Clone, Debug, Default)]
pub struct GroundLevelTool {
    hover: Option<Vec2>,
    /// What a click at `hover` writes.
    lines: Vec<String>,
    made: Option<Rc<Made>>,
}

impl GroundLevelTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The site's grounds as set now.
    fn grounds(cx: &Context<'_>) -> (Option<SiteGround>, Option<SiteGround>) {
        let site = cx.doc.settings().arch.as_ref().and_then(|a| a.site.as_ref());
        (site.and_then(|s| s.natural_ground.clone()), site.and_then(|s| s.graded_ground.clone()))
    }

    /// The grounds made, made again when the drawing or the site's grounds changed.
    fn made(&mut self, cx: &Context<'_>) -> Rc<Made> {
        let (revision, grounds) = (cx.doc.revision(), Self::grounds(cx));
        match &self.made {
            Some(m) if m.revision == revision && m.grounds == grounds => m.clone(),
            _ => {
                let m = Rc::new(Made { revision, grounds, surfaces: site_ground::surfaces(cx.doc) });
                self.made = Some(m.clone());
                m
            }
        }
    }

    /// The mark's lines at `at`; none where neither ground is.
    fn lines_at(&mut self, at: Vec2, cx: &Context<'_>) -> Vec<String> {
        let (natural, graded) = self.made(cx).surfaces.heights(at.x, at.y);
        let datum = cx.doc.settings().arch.as_ref().and_then(|a| a.site.as_ref()).and_then(|s| s.datum);
        site_ground::level_lines(natural, graded, datum)
    }

    /// Why nothing is written at a place with no ground.
    fn none_why(&mut self, cx: &Context<'_>) -> &'static str {
        if self.made(cx).surfaces.is_empty() {
            "Vaziyetin zemini yok: tabii ve tesviye zemini Arazi zemini ile kurun."
        } else {
            "Bu nokta zeminlerin dışında: ölçü noktalarının kapladığı yere tıklayın."
        }
    }

    /// The mark's objects at `at`: the symbol and the lines, `h` the drawing's text height.
    fn objects(at: Vec2, lines: &[String], h: f64, cx: &Context<'_>) -> Vec<NewObject> {
        let p = |x: f64, y: f64| kentos_commands::Vec2 { x: at.x + x, y: at.y + y };
        let object = |geometry, what: &str| NewObject {
            geometry,
            color: None,
            line_weight: None,
            line_type: None,
            line_pattern: None,
            attrs: Some(std::iter::once((MARK_ATTR.to_owned(), what.to_owned())).collect()),
            label: None,
            label_of: None,
            label_scale: None,
            symbol: None,
            order: None,
            transparency: None,
        };
        let line = |a, b| EntityGeometry::Line { a, b, zs: None, ms: None };
        let mut out = vec![
            object(EntityGeometry::Circle { c: p(0.0, 0.0), r: 0.4 * h }, "simge"),
            object(line(p(-0.7 * h, 0.0), p(0.7 * h, 0.0)), "simge"),
            object(line(p(0.0, -0.7 * h), p(0.0, 0.7 * h)), "simge"),
        ];
        // One line beside the symbol's middle; two over and under it, tzk on top.
        let placed: Vec<(f64, TextAlign)> = match lines.len() {
            1 => vec![(0.0, TextAlign::MiddleLeft)],
            _ => vec![(0.25 * h, TextAlign::BottomLeft), (-0.25 * h, TextAlign::TopLeft)],
        };
        for (text, (dy, align)) in lines.iter().zip(placed) {
            let what = text.split(' ').next().unwrap_or_default().to_owned();
            out.push(object(
                EntityGeometry::Text {
                    layout: None,
                    annotative: None,
                    p: p(1.0 * h, dy),
                    text: text.clone(),
                    height: h,
                    rotation: 0.0,
                    align: Some(align),
                    width_factor: None,
                    mask: true,
                    box_width: None,
                    line_spacing: None,
                    runs: Vec::new(),
                    face: crate::styles::text_face(cx),
                    path: None,
                },
                &what,
            ));
        }
        out
    }

    /// Writes the mark at `at`; says what it wrote, or why nothing.
    fn write(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let lines = self.lines_at(at, cx);
        if lines.is_empty() {
            let why = self.none_why(cx);
            cx.say(Level::Info, why);
            return;
        }
        let h = cx.annotation_height(kentos_commands::AnnotationKind::Text);
        let objects = Self::objects(at, &lines, h, cx);
        let written = crate::arch_layer::writing(&crate::arch_layer::Kind::Symbol, cx, |layer, cx| {
            let input = EntitiesCreate { layer_id: layer.to_owned(), objects, operation: None, expected_revision: None };
            let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        });
        if written {
            cx.say(Level::Success, format!("Zemin kotu yazıldı: {}.", lines.join(", ")));
            let datum = cx.doc.settings().arch.as_ref().and_then(|a| a.site.as_ref()).and_then(|s| s.datum);
            if datum.is_none() {
                cx.say(
                    Level::Info,
                    "±0,00'ın mutlak kotu yok: ±0,00'a göre farklar yazılmadı. Kotu Proje ayarları, İmar durumu'ndan \
                     verin.",
                );
            }
        }
    }
}

impl Tool for GroundLevelTool {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        "Zemin kotu"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(crate::arch_layer::Kind::Symbol)
    }

    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        Prompt::new("Zemin kotu", "tabii ve tesviye zemin kotunun yazılacağı noktaya tıklayın")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.lines = self.lines_at(p.world, cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.write(p.world, cx);
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn settings(&self, _f: &Format) -> Vec<Setting> {
        vec![Setting::hint(
            "P",
            "Zemin kotu",
            "Tıklanan noktada vaziyetin tabii zemin kotunu (tzk) ve tesviye zemin kotunu (tsk) yazar: önce mutlak kot, \
             parantezde ±0,00'a göre farkı (EK-4 4.30, Şekil 4.6). Zeminler Arazi zemini ile kurulur. Değerler yazı \
             olarak kalır: zemin sonradan değişirse işareti yeniden koyun.",
        )]
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            tag: self.hover.map(|at| Tag {
                at,
                lines: if self.lines.is_empty() { vec!["Zemin yok".to_owned()] } else { self.lines.clone() },
            }),
            tag_tone: if self.lines.is_empty() { Tone::Danger } else { Tone::Accent },
            ..Preview::default()
        }
    }
}
