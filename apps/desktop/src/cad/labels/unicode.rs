//! Draw the shared CAD glyph contours; never send their source back through
//! Iced's UI text shaper. Camera subtraction precedes the final f32 cast.
use super::*;
use kentos_geometry_core::text::shaping::OutlineOp;
use kentos_interaction::spatial::UnicodeText;

pub(super) struct Piece<'a> {
    pub text: &'a Rc<UnicodeText>,
    pub at: Vec2,
    pub camera: Camera,
    pub color: Color,
    pub runs: Vec<Color>,
    pub halo: Color,
    pub of: Option<Slot>,
}
fn screen(piece: &Piece<'_>, p: [f64; 2]) -> Point {
    let [x, y] = piece.camera.world_to_screen(piece.at);
    Point::new(
        (x + p[0] * piece.camera.scale) as f32,
        (y - p[1] * piece.camera.scale) as f32,
    )
}
fn color(piece: &Piece<'_>, scalar: u32) -> Color {
    let i = piece.text.runs.partition_point(|r| r.end <= scalar);
    piece
        .text
        .runs
        .get(i)
        .filter(|r| r.start <= scalar)
        .and_then(|_| piece.runs.get(i))
        .copied()
        .unwrap_or(piece.color)
}
pub(super) fn paint(frame: &mut Frame, piece: &Piece<'_>) {
    let Ok(plan) = &piece.text.plan else {
        let p = screen(piece, [0., 0.]);
        let s = (piece.text.height * piece.camera.scale).clamp(8., 64.) as f32;
        let path = Path::new(|b| {
            b.move_to(p);
            b.line_to(Point::new(p.x + s, p.y - s));
            b.move_to(Point::new(p.x, p.y - s));
            b.line_to(Point::new(p.x + s, p.y));
        });
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(Color::from_rgb8(237, 103, 95))
                .with_width(2.),
        );
        return;
    };
    if let Some(points) = piece.text.mask_outline() {
        let points = points.map(|p| screen(piece, p));
        <Frame as Ink>::fill_quad(frame, points, piece.halo, piece.of);
    }
    for (glyph, ops) in plan.glyphs.iter().zip(&piece.text.contours) {
        if ops.is_empty() {
            continue;
        }
        let path = Path::new(|b| {
            for op in ops {
                match *op {
                    OutlineOp::Move(p) => b.move_to(screen(piece, p)),
                    OutlineOp::Line(p) => b.line_to(screen(piece, p)),
                    OutlineOp::Quad(a, p) => {
                        b.quadratic_curve_to(screen(piece, a), screen(piece, p))
                    }
                    OutlineOp::Cubic(a, c, p) => {
                        b.bezier_curve_to(screen(piece, a), screen(piece, c), screen(piece, p))
                    }
                    OutlineOp::Close => b.close(),
                }
            }
        });
        frame.fill(&path, color(piece, glyph.glyph.scalars.start));
    }
    for rule in &plan.rules {
        <Frame as Ink>::fill_quad(
            frame,
            rule.corners.map(|p| screen(piece, p)),
            piece.runs.get(rule.run).copied().unwrap_or(piece.color),
            piece.of,
        );
    }
}
#[cfg(test)]
pub(super) fn collect(target: &mut Collect<'_>, piece: &Piece<'_>) {
    let Some((_, mut label)) = target.shape(&[screen(piece, [0., 0.])], piece.color, piece.of)
    else {
        return;
    };
    label.text = piece.text.source.clone();
    label.size = piece.text.height * piece.camera.scale * crate::sheet_pdf::PX_MM;
    label.unicode = Some(kentos_plot::maps::UnicodeLabel {
        text: piece.text.clone(),
        scale: piece.camera.scale / target.camera.scale,
        rotation: 0.,
        colors: piece.runs.iter().map(|c|Some(crate::sheet_pdf::hex_of(*c))).collect(),
    });
    label.halo = crate::sheet_pdf::hex_of(piece.halo);
    label.mask = piece.text.mask_outline().map(|points| {
        points
            .map(|p| {
                let p = screen(piece, p);
                let p = target
                    .camera
                    .screen_to_world(f64::from(p.x), f64::from(p.y));
                [p.x, p.y]
            })
            .to_vec()
    });
    target.labels.push(label);
}
impl Labels<'_> {
    pub(super) fn draw_unicode(
        &self,
        frame: &mut impl Ink,
        text: &Rc<UnicodeText>,
        at: Vec2,
        color: Color,
    ) {
        if self
            .fence
            .as_ref()
            .is_some_and(|f| !f.holds(self.screen(at)))
        {
            return;
        }
        let faint = self
            .current
            .get()
            .is_some_and(|slot| self.faded.contains(&slot.0));
        let tint = |mut c: Color| {
            if faint {
                c.a *= 1. - kentos_commands::PHASE_FADED as f32;
            }
            c
        };
        frame.shaped(&Piece {
            text,
            at,
            camera: self.camera.get(),
            color: tint(color),
            runs: text
                .runs
                .iter()
                .map(|r| tint(r.color.as_deref().map_or(color, |c| self.ink_of(Some(c)))))
                .collect(),
            halo: self.colors.halo,
            of: self.current.get(),
        });
    }
}
