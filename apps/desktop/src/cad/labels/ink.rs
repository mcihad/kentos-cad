//! Where a text's pieces go (`Ink`): onto the frame, or collected (the plot, the tests, the measures).

use super::*;

/// Where label pieces go: a canvas frame (drawn), or a list (a sheet PDF's map).
pub(super) trait Ink {
    /// The picture's size (a frame's own; a fenced map's is the camera's).
    fn size(&self) -> Size;
    fn piece(&mut self, piece: &Piece<'_>, halo: Color, of: Option<Slot>);
    fn shaped(&mut self,piece:&unicode::Piece<'_>);
    /// A quadrilateral of the screen filled (a multi-line text's mask, docs/adr/0182 §3).
    fn fill_quad(&mut self, quad: [Point; 4], color: Color, of: Option<Slot>);
    /// Segments stroked `width` wide in one colour (the texts too small to read, docs/adr/0626 §6).
    fn stroke_segments(&mut self, segments: &[[Point; 2]], color: Color, _width: f32) {
        for segment in segments {
            self.stroke_line(segment, false, color, None);
        }
    }
    /// Grid north and the scale bar, or a CAD project's coordinate axes.
    fn marks(
        &mut self,
        camera: &Camera,
        colors: &Colors,
        axes: kentos_interaction::Axes,
        ucs: Option<f64>,
        lift: f32,
    );
    /// A ring of the screen filled, and a line drawn: a grown leader's arrowhead (docs/adr/0205 §5),
    /// a placed label's background, shadow and callout (docs/adr/0212 §3.8). A list of pieces takes
    /// those of an object (`of`), on its layer.
    fn fill_ring(&mut self, ring: &[Point], color: Color, of: Option<Slot>);
    fn stroke_line(&mut self, pts: &[Point], closed: bool, color: Color, of: Option<Slot>);
}

impl Ink for Frame {
    fn shaped(&mut self,piece:&unicode::Piece<'_>) {unicode::paint(self,piece);}
    fn size(&self) -> Size {
        Frame::size(self)
    }

    fn piece(&mut self, piece: &Piece<'_>, halo: Color, _of: Option<Slot>) {
        draw(self, piece, halo);
    }

    fn fill_quad(&mut self, quad: [Point; 4], color: Color, _of: Option<Slot>) {
        let path = Path::new(|b| {
            b.move_to(quad[0]);
            for p in &quad[1..] {
                b.line_to(*p);
            }
            b.close();
        });
        self.fill(&path, color);
    }

    fn stroke_segments(&mut self, segments: &[[Point; 2]], color: Color, width: f32) {
        let path = Path::new(|b| {
            for [from, to] in segments {
                b.move_to(*from);
                b.line_to(*to);
            }
        });
        self.stroke(&path, Stroke::default().with_color(color).with_width(width));
    }

    fn marks(
        &mut self,
        camera: &Camera,
        colors: &Colors,
        axes: kentos_interaction::Axes,
        ucs: Option<f64>,
        lift: f32,
    ) {
        crate::map_marks::paint(self, camera, colors, axes, ucs, lift);
    }

    fn fill_ring(&mut self, ring: &[Point], color: Color, _of: Option<Slot>) {
        let Some((first, rest)) = ring.split_first() else {
            return;
        };
        let path = Path::new(|b| {
            b.move_to(*first);
            for p in rest {
                b.line_to(*p);
            }
            b.close();
        });
        self.fill(&path, color);
    }

    fn stroke_line(&mut self, pts: &[Point], closed: bool, color: Color, _of: Option<Slot>) {
        let Some((first, rest)) = pts.split_first() else {
            return;
        };
        let path = Path::new(|b| {
            b.move_to(*first);
            for p in rest {
                b.line_to(*p);
            }
            if closed {
                b.close();
            }
        });
        self.stroke(&path, Stroke::default().with_color(color).with_width(1.2));
    }
}

#[cfg(test)]
impl Collect<'_> {
    /// Points of the screen on the ground, and a wordless label of the object's layer in `color` to
    /// carry them (a placed label's background, shadow or callout); none for no object.
    pub(super) fn shape(
        &self,
        pts: &[Point],
        color: Color,
        of: Option<Slot>,
    ) -> Option<(Vec<[f64; 2]>, MapLabel)> {
        let layer = self.doc.get(of?)?.base().layer_id.clone();
        let ground: Vec<[f64; 2]> = pts
            .iter()
            .map(|p| {
                let w = self.camera.screen_to_world(f64::from(p.x), f64::from(p.y));
                [w.x, w.y]
            })
            .collect();
        let at = kentos_render_wgpu::Vec2::new(ground.first()?[0], ground.first()?[1]);
        let ink = crate::sheet_pdf::hex_of(color);
        Some((
            ground,
            MapLabel {
            shx: None,
                unicode:None,
                layer,
                text: String::new(),
                at,
                anchor: kentos_sheet::pdf::TextAnchor::LeftBaseline,
                rotation: 0.0,
                size: 0.0,
                weight: 400,
                italic: false,
                font: None,
                color: ink.clone(),
                halo: ink,
                mask: None,
                underline: None,
                fill: None,
                stroke: None,
                width_factor: 1.0,
                lean: 0.0,
            },
        ))
    }
}

#[cfg(test)]
impl Ink for Collect<'_> {
    fn shaped(&mut self,piece:&unicode::Piece<'_>) {unicode::collect(self,piece);}
    fn size(&self) -> Size {
        Size::ZERO
    }

    fn piece(&mut self, piece: &Piece<'_>, halo: Color, of: Option<Slot>) {
        if piece.size < SMALLEST || piece.text.is_empty() {
            return;
        }
        use kentos_sheet::pdf::TextAnchor;
        let layer = of
            .and_then(|slot| self.doc.get(slot))
            .map(|e| e.base().layer_id.clone())
            .unwrap_or_default();
        let at = self
            .camera
            .screen_to_world(f64::from(piece.at.x), f64::from(piece.at.y));
        // Clockwise on the screen (y down) is counter-clockwise on the paper.
        let rotation = -f64::from(piece.angle).to_degrees();
        let px = 1.0 / self.camera.scale.max(f64::MIN_POSITIVE);
        // A masked text's box (the web's `mask`): from below the baseline to above the letters,
        // a tenth of the height round it (a dimension's value along its line, no margin up and down).
        let mask = (piece.mask > 0.0).then(|| {
            let (w, h) = (f64::from(piece.mask) * px, f64::from(piece.size) * px);
            let along = matches!(piece.anchor, Anchor::CenterBaseline);
            let x0 = if along { -w / 2.0 } else { 0.0 };
            let m = h * 0.1;
            let v = if along { 0.0 } else { m };
            let (s, c) = rotation.to_radians().sin_cos();
            [
                [x0 - m, -0.23 * h - v],
                [x0 + w + m, -0.23 * h - v],
                [x0 + w + m, 1.15 * h + v],
                [x0 - m, 1.15 * h + v],
            ]
            .iter()
            .map(|[u, t]| [at.x + u * c - t * s, at.y + u * s + t * c])
            .collect()
        });
        self.labels.push(MapLabel {
            shx: None,
            unicode:None,
            layer,
            text: piece.text.to_owned(),
            at,
            anchor: match piece.anchor {
                Anchor::LeftBaseline => TextAnchor::LeftBaseline,
                Anchor::CenterBaseline => TextAnchor::CenterBaseline,
                Anchor::CenterMiddle => TextAnchor::CenterMiddle,
            },
            rotation,
            size: f64::from(piece.size) * crate::sheet_pdf::PX_MM,
            weight: match piece.font.weight {
                iced::font::Weight::Thin
                | iced::font::Weight::ExtraLight
                | iced::font::Weight::Light
                | iced::font::Weight::Normal => 400,
                iced::font::Weight::Medium => 500,
                iced::font::Weight::Semibold => 600,
                _ => 700,
            },
            italic: piece.font.style == iced::font::Style::Italic,
            font: match piece.font.family {
                iced::font::Family::Name(name) => DrawingFont::ALL
                    .into_iter()
                    .find(|f| drawing_fonts::family(*f) == name),
                _ => None,
            },
            color: crate::sheet_pdf::hex_of(piece.color),
            halo: crate::sheet_pdf::hex_of(halo),
            mask,
            fill: None,
            stroke: None,
            // Its width factor and slant on the paper too (docs/adr/0324 §6).
            width_factor: f64::from(piece.width_factor),
            lean: f64::from(piece.lean),
            underline: (piece.underline > 0.0).then(|| {
                // From the baseline's start 0.12 of the height down, 0.06 thick, the bar's length along.
                let (w, h) = (f64::from(piece.underline) * px, f64::from(piece.size) * px);
                let (s, c) = rotation.to_radians().sin_cos();
                [
                    [0.0, -0.12 * h],
                    [w, -0.12 * h],
                    [w, -0.18 * h],
                    [0.0, -0.18 * h],
                ]
                .iter()
                .map(|[u, t]| [at.x + u * c - t * s, at.y + u * s + t * c])
                .collect()
            }),
        });
    }

    fn fill_ring(&mut self, ring: &[Point], color: Color, of: Option<Slot>) {
        if let Some(label) = self.shape(ring, color, of) {
            self.labels.push(MapLabel {
            shx: None,
                fill: Some(label.0),
                ..label.1
            });
        }
    }

    fn stroke_line(&mut self, pts: &[Point], closed: bool, color: Color, of: Option<Slot>) {
        if let Some(label) = self.shape(pts, color, of) {
            self.labels.push(MapLabel {
            shx: None,
                stroke: Some((label.0, closed)),
                ..label.1
            });
        }
    }

    fn fill_quad(&mut self, quad: [Point; 4], color: Color, of: Option<Slot>) {
        use kentos_sheet::pdf::TextAnchor;
        let layer = of
            .and_then(|slot| self.doc.get(slot))
            .map(|e| e.base().layer_id.clone())
            .unwrap_or_default();
        let ground: Vec<[f64; 2]> = quad
            .iter()
            .map(|p| {
                let w = self.camera.screen_to_world(f64::from(p.x), f64::from(p.y));
                [w.x, w.y]
            })
            .collect();
        let at = self
            .camera
            .screen_to_world(f64::from(quad[0].x), f64::from(quad[0].y));
        // A mask without words: the PDF fills it in the paper's colour, under the layer's texts.
        self.labels.push(MapLabel {
            shx: None,
            unicode:None,
            layer,
            text: String::new(),
            at,
            anchor: TextAnchor::LeftBaseline,
            rotation: 0.0,
            size: 0.0,
            weight: 400,
            italic: false,
            font: None,
            color: crate::sheet_pdf::hex_of(color),
            halo: crate::sheet_pdf::hex_of(color),
            mask: Some(ground),
            underline: None,
            fill: None,
            stroke: None,
            width_factor: 1.0,
            lean: 0.0,
        });
    }

    fn marks(
        &mut self,
        _camera: &Camera,
        _colors: &Colors,
        _axes: kentos_interaction::Axes,
        _ucs: Option<f64>,
        _lift: f32,
    ) {
    }
}
