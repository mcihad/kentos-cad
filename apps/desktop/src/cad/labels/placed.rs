//! The label engine's placed labels (docs/adr/0212): their frames, backdrops, lines and letters; the
//! bands of texts too small to read and what a table's cells are painted from.

use super::*;

impl Labels<'_> {
    /// The bands painted, taken, a colour and thickness together in the order they first came (docs/adr/0626 §6).
    pub(super) fn band_looks(&self) -> Vec<(Color, f32, Vec<[Point; 2]>)> {
        let bands = std::mem::take(&mut *self.bands.borrow_mut());
        let mut by_look: Vec<(Color, f32, Vec<[Point; 2]>)> = Vec::new();
        for band in bands {
            match by_look
                .iter_mut()
                .find(|(c, w, _)| *c == band.color && *w == band.thick)
            {
                Some((_, _, segments)) => segments.push(band.line),
                None => by_look.push((band.color, band.thick, vec![band.line])),
            }
        }
        by_look
    }

    /// What a table's cells in `part` are painted from besides their places (docs/adr/0786 §3): their words, the
    /// table's face, height, colour, styles and merges, as one number; none for a part of anything else.
    pub(super) fn cells_print(
        &self,
        slot: Slot,
        part: &[(LabelSpot, Grow)],
        tables: &RefCell<HashMap<usize, u64>>,
    ) -> Option<u64> {
        let Some(Entity::Table(t)) = self.doc.get(slot) else {
            return None;
        };
        let look = *tables
            .borrow_mut()
            .entry(t as *const _ as usize)
            .or_insert_with(|| {
                let mut h = Fx::default();
                format!("{:?}{:?}{:?}{:?}{:?}", t.face, t.base, t.styles, t.merges, t.aligns).hash(&mut h);
                // Its table style's cells too (docs/adr/1126 §3): a changed style draws it again.
                let style = t.table_style.as_deref().and_then(|id| self.doc.settings().table_styles.iter().find(|s| s.id == id));
                format!("{:?}{style:?}", t.table_style).hash(&mut h);
                (t.height.to_bits(), t.header).hash(&mut h);
                h.finish()
            });
        let mut h = Fx(look);
        for (spot, _) in part {
            let LabelSpot::Cell { row, col, line, .. } = spot else {
                return None;
            };
            (row, col, line, t.cells.get(*row).and_then(|r| r.get(*col))).hash(&mut h);
        }
        Some(h.finish())
    }

    /// The colour an object's layer gives its text: its layer's (none without a layer).
    pub(super) fn layer_color_of(&self, slot: Slot) -> Option<String> {
        let entity = self.doc.get(slot)?;
        let layer = self.doc.layers().get(&entity.base().layer_id);
        self.layer_color(layer).map(str::to_owned)
    }

    /// The look of the label engine's label whose frame this is (docs/adr/0212
    /// §3.8): its class's style (its layer's rule's, its single label's, else
    /// its kind's default) on this canvas, in its state's colour.
    pub(super) fn placed_frame(
        &self,
        entity: &Entity,
        layer: Option<&kentos_commands::LayerNode>,
        class: u16,
        state: u32,
        slot: Slot,
    ) -> Option<Placed> {
        let style = match layer.and_then(|l| l.style.labels.as_ref()) {
            Some(l) if l.mode == kentos_commands::LabelsMode::Rules => {
                l.classes.get(class as usize).map(|c| c.style.clone())?
            }
            _ => layer
                .and_then(|l| l.style.label.clone())
                .or_else(|| default_label(entity.kind()))?,
        };
        let named = |c: &str| match c {
            "label" => Some(self.colors.label),
            other => self.colors.named(other),
        };
        let unplaced = state & UNPLACED != 0;
        let mut color = if unplaced {
            Color::from_rgb8(0xef, 0x6b, 0x61)
        } else {
            match (&style.color, style.ink) {
                (Some(c), _) => named(c).unwrap_or(self.colors.label),
                (None, Some(LabelInk::Fg)) => self.colors.fg,
                (None, Some(LabelInk::FgDim)) => self.colors.fg_dim,
                (None, _) => self.colors.label,
            }
        };
        let faint = state & HIDDEN != 0;
        if faint {
            color.a *= 0.35;
        }
        let masked = state & MASKED != 0;
        let background = match &style.background {
            Some(b) => Some(Backdrop {
                shape: b.shape,
                fill: b.fill.as_deref().and_then(named),
                stroke: b.stroke.as_deref().and_then(named),
                pad: b.padding.unwrap_or(2.0) as f32,
            }),
            None if masked => Some(Backdrop {
                shape: kentos_commands::LabelShape::Rect,
                fill: Some(self.colors.halo),
                stroke: None,
                pad: 1.0,
            }),
            None => None,
        };
        let halo = match &style.halo {
            Some(h) if h.width <= 0.0 => None,
            Some(h) => Some((
                h.width as f32,
                h.color
                    .as_deref()
                    .and_then(named)
                    .unwrap_or(self.colors.halo),
            )),
            None => Some((HALO, self.colors.halo)),
        };
        let shadow = style.shadow.as_ref().filter(|_| !unplaced).map(|h| {
            let mut c = h.color.as_deref().and_then(named).unwrap_or(Color::BLACK);
            c.a *= h.opacity.unwrap_or(0.5) as f32;
            (h.dx as f32, -h.dy as f32, c)
        });
        Some(Placed {
            slot,
            color,
            halo: if background.as_ref().is_some_and(|b| b.fill.is_some()) {
                None
            } else {
                halo
            },
            font: drawing_fonts::font(
                self.font,
                style.weight.unwrap_or(500),
                style.italic == Some(true),
            ),
            shadow,
            background,
            callout: style
                .callout
                .as_ref()
                .and_then(|c| c.color.as_deref().and_then(named))
                .unwrap_or(color),
            manhattan: style
                .callout
                .as_ref()
                .is_some_and(|c| c.kind == kentos_commands::CalloutKind::Manhattan),
            pinned: self.pinned && state & PINNED != 0,
        })
    }

    /// A placed label's backdrop: its background or a contour's mask, `w`
    /// × `h` px round `at` turned `angle` degrees (a curved label's every
    /// letter's: `letter`), its shadow first; a pinned label's outline.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn placed_backdrop(
        &self,
        frame: &mut impl Ink,
        c: &Placed,
        at: Vec2,
        angle: f64,
        w: f32,
        h: f32,
        letter: bool,
    ) {
        let m = self.screen(at);
        let a = -(angle.to_radians() as f32);
        let quad = |dx: f32, dy: f32, hw: f32, hh: f32| {
            let (s, cs) = a.sin_cos();
            let at =
                |u: f32, v: f32| Point::new(m.x + dx + u * cs - v * s, m.y + dy + u * s + v * cs);
            [at(-hw, -hh), at(hw, -hh), at(hw, hh), at(-hw, hh)]
        };
        if let Some(b) = &c.background {
            let (hw, hh) = (w / 2.0 + b.pad, h / 2.0 + b.pad);
            // A rectangle as a quadrilateral (a sheet's PDF takes it as a mask); a round one or an ellipse as a ring.
            let ring = |dx: f32, dy: f32| -> Vec<Point> {
                let (s, cs) = a.sin_cos();
                let at = |u: f32, v: f32| {
                    Point::new(m.x + dx + u * cs - v * s, m.y + dy + u * s + v * cs)
                };
                match b.shape {
                    kentos_commands::LabelShape::Ellipse => (0..32)
                        .map(|i| {
                            let t = i as f32 / 32.0 * std::f32::consts::TAU;
                            at(
                                hw * std::f32::consts::SQRT_2 * t.cos(),
                                hh * std::f32::consts::SQRT_2 * t.sin(),
                            )
                        })
                        .collect(),
                    _ => {
                        let r = (hh).min(6.0).min(hw);
                        let mut pts = Vec::with_capacity(24);
                        for (cx, cy, from) in [
                            (hw - r, hh - r, 0.0f32),
                            (-hw + r, hh - r, 90.0),
                            (-hw + r, -hh + r, 180.0),
                            (hw - r, -hh + r, 270.0),
                        ] {
                            for k in 0..=5 {
                                let t = (from + k as f32 * 18.0).to_radians();
                                pts.push(at(cx + r * t.cos(), cy + r * t.sin()));
                            }
                        }
                        pts
                    }
                }
            };
            let round = b.shape != kentos_commands::LabelShape::Rect;
            if let Some((dx, dy, sc)) = c.shadow
                && b.fill.is_some()
            {
                if round {
                    frame.fill_ring(&ring(dx, dy), sc, Some(c.slot));
                } else {
                    frame.fill_ring(&quad(dx, dy, hw, hh), sc, Some(c.slot));
                }
            }
            if let Some(f) = b.fill {
                if round {
                    frame.fill_ring(&ring(0.0, 0.0), f, Some(c.slot));
                } else {
                    frame.fill_ring(&quad(0.0, 0.0, hw, hh), f, Some(c.slot));
                }
            }
            if let Some(st) = b.stroke {
                if round {
                    frame.stroke_line(&ring(0.0, 0.0), true, st, Some(c.slot));
                } else {
                    frame.stroke_line(&quad(0.0, 0.0, hw, hh), true, st, Some(c.slot));
                }
            }
        }
        if c.pinned && !letter {
            let q = quad(0.0, 0.0, w / 2.0 + 3.0, h / 2.0 + 3.0);
            frame.stroke_line(&q, true, self.colors.accent(), None);
        }
    }

    /// One line or letter of a placed label: its shadow, then itself in its halo.
    pub(super) fn placed_text(
        &self,
        frame: &mut impl Ink,
        c: &Placed,
        text: &str,
        at: Vec2,
        angle: f64,
        size: f32,
    ) {
        let s = self.screen(at);
        let piece = |at: Point, color: Color| Piece {
            text,
            at,
            angle: -(angle.to_radians() as f32),
            size,
            font: c.font,
            anchor: Anchor::CenterMiddle,
            color,
            width_factor: 1.0,
            mask: 0.0,
            underline: 0.0,
            lean: 0.0,
        };
        if let Some((dx, dy, sc)) = c.shadow {
            frame.piece(
                &piece(Point::new(s.x + dx, s.y + dy), sc),
                Color::TRANSPARENT,
                Some(c.slot),
            );
        }
        let halo = c.halo.map_or(Color::TRANSPARENT, |(_, h)| h);
        frame.piece(&piece(s, c.color), halo, Some(c.slot));
    }
}
