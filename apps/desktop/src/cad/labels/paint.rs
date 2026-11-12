//! Painting the drawing's text: which labels may show in the view, a piece and a line of a text, the
//! texts themselves, grid north and the scale bar or the axes.

use super::*;

impl Labels<'_> {
    /// Whether a label may show in the view of `area` (true sizes, docs/adr/0626 §7): its anchor on the screen
    /// within its reach of the view, the reach a generous length for its letters (a text's own, a value's eight)
    /// at its height; a label without a height of its own reaches a quarter of the view.
    pub(super) fn near_view(&self, spot: &LabelSpot) -> bool {
        let area = Size::new(self.view.width as f32, self.view.height as f32);
        let margin = self.margin.get();
        let scale = self.view.map_px() as f32;
        let letters = |text: &str, height: f64| (text.chars().count() as f32 * 0.8 + 2.0) * height as f32 * scale;
        let (at, reach) = match spot {
            LabelSpot::Line {unicode:Some(_),..} => return true,
            LabelSpot::Text { slot, at, .. } => {
                let reach = match self.doc.get(*slot) {
                    Some(Entity::Text(t)) => {
                        let longest = t.text.lines().map(|l| l.chars().count()).max().unwrap_or(1);
                        let lines = t.text.lines().count().max(1) as f32;
                        let box_width = t.paragraph.box_width.map_or(0.0, |w| w as f32 * scale);
                        (longest as f32 * 0.8 + 2.0).max(lines * 1.6) * t.height as f32 * scale + box_width
                    }
                    _ => area.width.max(area.height) / 4.0,
                };
                (*at, reach)
            }
            LabelSpot::Dimension { slot, at, .. } => {
                let height = match self.doc.get(*slot) {
                    Some(Entity::Dimension(d)) => d.height,
                    _ => 0.0,
                };
                (*at, letters("00000000", height).max(area.width.max(area.height) / 16.0))
            }
            LabelSpot::PieceText { at, height, text, .. } => (*at, letters(text, *height)),
            LabelSpot::PieceDimension { at, height, .. } => (*at, letters("00000000", *height)),
            LabelSpot::Line { at, height, .. } | LabelSpot::ParagraphMask { at, height, .. } => {
                (*at, area.width.max(area.height) / 4.0 + *height as f32 * scale * 4.0)
            }
            LabelSpot::Placed { at, .. }
            | LabelSpot::PlacedLine { at, .. }
            | LabelSpot::PlacedLetter { at, .. }
            | LabelSpot::Cell { at, .. } => (*at, area.width.max(area.height) / 4.0),
            LabelSpot::PlacedCallout { .. } => return true,
        };
        let [x, y] = self.view.world_to_screen(at);
        let (x, y) = (x as f32, y as f32);
        let reach = reach + margin;
        x > -reach && y > -reach && x < area.width + reach && y < area.height + reach
    }

    /// Draws a piece, unless a map frame's text is anchored outside the frame (design §9a: one
    /// anchored inside is drawn and cut at the frame's edge).
    pub(super) fn draw(&self, frame: &mut impl Ink, piece: &Piece<'_>, halo: Color) {
        // A word of the level drawn faint under the active one (docs/adr/0715 §2), as faint as its lines.
        let faint;
        let piece = match self.current.get() {
            Some(slot) if !self.faded.is_empty() && self.faded.contains(&slot.0) => {
                faint = Piece {
                    color: Color {
                        a: piece.color.a * (1.0 - kentos_commands::PHASE_FADED as f32),
                        ..piece.color
                    },
                    ..*piece
                };
                &faint
            }
            _ => piece,
        };
        if let Some(fence) = &self.fence
            && !fence.holds(piece.at)
        {
            return;
        }
        if self.true_size {
            // True sizes (docs/adr/0626 §6): a piece away from the view is left out, a text too small to read is
            // a faint band along its line, the others are drawn without a halo (their masks keep its colour).
            let length = width_em(piece.text) * piece.size * piece.width_factor.max(0.1);
            let reach = length + piece.size * 2.0 + self.margin.get();
            let area = Size::new(self.view.width as f32, self.view.height as f32);
            if piece.at.x < -reach
                || piece.at.y < -reach
                || piece.at.x > area.width + reach
                || piece.at.y > area.height + reach
            {
                return;
            }
            if piece.size < GREEK_PX {
                if !piece.text.trim().is_empty() {
                    self.bands.borrow_mut().push(band(piece, length));
                }
                return;
            }
            frame.piece(piece, Color { a: 0.0, ..halo }, self.current.get());
            return;
        }
        frame.piece(piece, halo, self.current.get());
    }

    /// One line of a multi-line text (docs/adr/0182 §3): its letters
    /// `start..end` run by run from where its baseline starts, each run where
    /// the shared layout's advances put it (bold from the bold table, raised
    /// and lowered at 0.6), upright at 400 (600 bold, italic when italic),
    /// raised 0.4 and lowered 0.15 of the height, in the run's colour, its
    /// underline 0.12 of the height under the baseline. A text's face
    /// (docs/adr/0183 §2): its typeface, its bold and italic added to the
    /// runs', the line leaning by its slant about its baseline.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn line(
        &self,
        frame: &mut impl Ink,
        text: &str,
        runs: &[Run],
        (start, end): (usize, usize),
        at: Vec2,
        rotation: f64,
        height: f64,
        width_factor: f64,
        text_face: &kentos_commands::TextFace,
        ink: Color,
    ) {
        let letters: Vec<char> = text.chars().collect();
        let (start, end) = (start.min(letters.len()), end.min(letters.len()));
        let face = kentos_native_application::geometry::drawing_font(Some(
            text_face.font.unwrap_or(self.font),
        ));
        let own_bold = text_face.font.is_some() && text_face.bold;
        // The slant's tangent: a raised or lowered run starts that much further along.
        let slant = match (text_face.font, text_face.oblique) {
            (Some(_), Some(o)) => o.to_radians().tan(),
            _ => 0.0,
        };
        let r = rotation.to_radians();
        let (c, s) = (r.cos(), r.sin());
        let size = height * self.camera.get().scale;
        // Metres along the baseline: thousandths of an em, the height, the width factor.
        let em = height * width_factor / 1000.0;
        let look = |i: usize| -> Option<&Run> {
            runs.iter()
                .find(|r| (r.start as usize) <= i && i < r.end as usize)
        };
        let mut x = 0.0;
        let mut i = start;
        while i < end {
            let f = look(i);
            let mut j = i + 1;
            while j < end && look(j) == f {
                j += 1;
            }
            let bold = f.is_some_and(|r| r.bold) || own_bold;
            let script = f.and_then(|r| r.script);
            let (font, lean) = face_font(text_face, self.font, false, f);
            let color = f
                .and_then(|r| r.color.as_deref())
                .and_then(|name| self.colors.named(name))
                .unwrap_or(ink);
            let underlined = f.is_some_and(|r| r.underline);
            // What the run draws: its words, where along and how high, at what size, and its underline's length.
            let mut parts: Vec<(String, f64, f64, f64, f32)> = Vec::new();
            let width: f64;
            let stack = f.filter(|r| {
                script == Some(Script::Stack) && r.start as usize >= i && r.end as usize <= j
            });
            if let Some((q, mark, num, den)) =
                stack.and_then(|r| paragraph::stack_parts(&letters, r, face, bold))
            {
                // A stacked fraction (docs/adr/0324 §2): over and under, or side by side with `#`.
                let small = size * paragraph::SCRIPT_SIZE;
                let upper: String = letters[i..q].iter().collect();
                let lower: String = letters[q + 1..j].iter().collect();
                if mark == '#' {
                    let slash = advance(face, '/', bold, true);
                    parts.push((upper, x, paragraph::STACK_OVER * height, small, 0.0));
                    parts.push(("/".into(), x + num * em, 0.0, small, 0.0));
                    parts.push((
                        lower,
                        x + (num + slash) * em,
                        paragraph::STACK_UNDER * height,
                        small,
                        0.0,
                    ));
                    width = (num + slash + den) * em;
                } else {
                    let w = num.max(den);
                    parts.push((
                        upper,
                        x + (w - num) / 2.0 * em,
                        paragraph::STACK_OVER * height,
                        small,
                        0.0,
                    ));
                    parts.push((
                        lower,
                        x + (w - den) / 2.0 * em,
                        paragraph::STACK_UNDER * height,
                        small,
                        0.0,
                    ));
                    if mark == '/' {
                        // The bar: an underline 0.12 of its size under a blank's baseline.
                        let bar =
                            paragraph::STACK_BAR * height + 0.12 * height * paragraph::SCRIPT_SIZE;
                        let long = (w * em * self.camera.get().scale / width_factor) as f32;
                        parts.push((" ".into(), x, bar, small, long));
                    }
                    width = w * em;
                }
            } else {
                width = letters[i..j]
                    .iter()
                    .map(|ch| advance(face, *ch, bold, script.is_some()))
                    .sum::<f64>()
                    * em;
                let lift = match script {
                    Some(Script::Super) => 0.4 * height,
                    Some(Script::Sub) => -0.15 * height,
                    _ => 0.0,
                };
                let words: String = letters[i..j].iter().collect();
                let scaled = if script.is_some() { size * 0.6 } else { size };
                let line = if underlined {
                    (width * self.camera.get().scale / width_factor) as f32
                } else {
                    0.0
                };
                parts.push((words, x, lift, scaled, line));
            }
            for (words, along, lift, scaled, line) in parts {
                if words.trim().is_empty() && line == 0.0 {
                    continue;
                }
                let along = along + lift * slant;
                let place = Vec2::new(at.x + c * along - s * lift, at.y + s * along + c * lift);
                self.draw(
                    frame,
                    &Piece {
                        text: &words,
                        at: self.screen(place),
                        angle: (-r) as f32,
                        size: scaled as f32,
                        font,
                        anchor: Anchor::LeftBaseline,
                        color,
                        width_factor: width_factor as f32,
                        mask: 0.0,
                        underline: line,
                        lean,
                    },
                    self.colors.halo,
                );
            }
            x += width;
            i = j;
        }
    }

    /// The texts, then grid north and the scale bar or the axes.
    pub(super) fn paint(&self, frame: &mut impl Ink) {
        self.paint_texts(frame, 0.0);
        self.paint_marks(frame);
    }

    /// Grid north and the scale bar over the text, or a CAD project's coordinate axes, as the web's overlay
    /// (map_marks.rs); a sheet's map has its own, a twisted view draws them on its own camera.
    pub(super) fn paint_marks(&self, frame: &mut impl Ink) {
        if self.fence.is_none()
            && self.turned.is_none()
            && let Some(lift) = self.map_marks
        {
            let ucs = self.format.ucs.map(|u| u[2]);
            frame.marks(&self.view, &self.colors, self.format.axes, ucs, lift);
        }
    }

    /// The texts; under true sizes those `margin` pixels round the view too (docs/adr/0626 §7, labels/layer.rs).
    pub(super) fn paint_texts(&self, frame: &mut impl Ink, margin: f32) {
        self.margin.set(margin);
        // A map frame's labels keep apart on the camera's own picture.
        let size = match self.fence {
            Some(_) => Size::new(self.view.width as f32, self.view.height as f32),
            None => frame.size(),
        };
        let _ = size;
        let layers = self.doc.layers();
        // The label engine's label being drawn: its frame's look (docs/adr/0212 §3.8).
        let mut current: Option<Placed> = None;
        // The tables whose cells' fills are drawn this frame (docs/adr/0327 §1): under their words, once.
        let mut filled: std::collections::HashSet<Slot> = std::collections::HashSet::new();
        // The last table's shape as its table style draws it (docs/adr/1127 §1): its cells come one after another.
        let mut drawn_table: Option<(Slot, kentos_geometry_core::entity::Shape)> = None;
        // A part painted as bands alone before, in the same look, of the same object, is its bands again
        // (docs/adr/0786 §3); what a part draws besides is counted.
        let mut memo = self.band_memo.as_ref().map(|m| m.borrow_mut());
        if let Some(m) = memo.as_deref_mut() {
            m.begin(self.band_key, margin);
        }
        let frame = &mut Counted { ink: frame, ops: 0 };
        // A table's look besides its words, once a painting (`cells_print`).
        let tables: RefCell<HashMap<usize, u64>> = RefCell::default();
        let parts = self.spots.parts.iter().map(|p| (p, true));
        for (part, own) in parts.chain(std::iter::once((&self.spots.placed, false))) {
            let mut kept = memo.as_deref_mut().filter(|_| own);
            let of = part.first().map(|(spot, _)| slot_of(spot));
            let made = (self.bands.borrow().len(), frame.ops);
            if let (Some(m), Some(slot)) = (kept.as_deref_mut(), of)
                && let Some(bands) = m.bands_of(
                    part,
                    self.doc.get_shared(slot),
                    self.layer_color_of(slot),
                    || self.cells_print(slot, part, &tables),
                )
            {
                self.bands.borrow_mut().extend_from_slice(bands);
                continue;
            }
            // True sizes ask for a box round the view (docs/adr/0626 §7): what lies well outside the view and its margin
            // is passed over before it is laid out.
            for (spot, grow) in part.iter() {
                if self.true_size && !self.near_view(spot) {
                    continue;
                }
                // A grown label goes through the view zoomed about its anchor (docs/adr/0205 §5).
                self.camera.set(match self.held {
                    Some(held) => held_grown(self.view, *grow, held),
                    None => grown(self.view, *grow),
                });
                let slot = match spot {
                    LabelSpot::Dimension { slot, .. }
                    | LabelSpot::Text { slot, .. }
                    | LabelSpot::Placed { slot, .. }
                    | LabelSpot::PlacedLine { slot, .. }
                    | LabelSpot::PlacedLetter { slot, .. }
                    | LabelSpot::PlacedCallout { slot, .. }
                    | LabelSpot::PieceText { slot, .. }
                    | LabelSpot::PieceDimension { slot, .. }
                    | LabelSpot::Line { slot, .. }
                    | LabelSpot::ParagraphMask { slot, .. }
                    | LabelSpot::Cell { slot, .. } => *slot,
                };
                self.current.set(Some(slot));
                let Some(entity) = self.doc.get(slot) else {
                    continue;
                };
                let base = entity.base();
                let layer = layers.get(&base.layer_id);
                match (spot, entity) {
                    (
                        LabelSpot::Dimension {
                            at,
                            angle,
                            value,
                            unit,
                            prefix,
                            mask,
                            ..
                        },
                        Entity::Dimension(d),
                    ) => {
                        // Its value in its look's writing and typeface (docs/adr/0183 §3).
                        // A text written over it shows the value where it says `<>` (docs/adr/0321 §3).
                        let text = self.format.dimension_value(
                            prefix,
                            unit,
                            *value,
                            &d.look,
                            d.text.as_deref().filter(|t| !t.is_empty()),
                        );
                        // Its value's own colour when its look names one (docs/adr/0205 §6), else the object's.
                        let color = self.ink_of(
                            d.look
                                .text_color
                                .as_deref()
                                .or(base.color.as_deref())
                                .or(self.layer_color(layer)),
                        );
                        // In its style's SHX text style (docs/adr/1023): the font's strokes, its own box the mask.
                        if let Some(mut t) = kentos_native_application::shx_text::dimension_value(
                            self.doc.settings(),
                            d.look.dim_style.as_deref(),
                            kentos_commands::Vec2 { x: at.x, y: at.y },
                            *angle,
                            d.height,
                            &text,
                        ) {
                            t.mask = *mask;
                            if self.shx_text(frame, slot, &t, color) {
                                continue;
                            }
                        }
                        let size = (d.height * self.camera.get().scale) as f32;
                        let font = drawing_fonts::font(d.look.font.unwrap_or(self.font), 500, false);
                        self.draw(
                            frame,
                            &Piece {
                                text: &text,
                                at: self.screen(*at),
                                angle: (-angle.to_radians()) as f32,
                                size,
                                font,
                                anchor: Anchor::CenterBaseline,
                                color,
                                width_factor: 1.0,
                                mask: value_mask(*mask, &text, font, size),
                                underline: 0.0,
                                lean: 0.0,
                            },
                            self.colors.halo,
                        );
                    }
                    (
                        LabelSpot::Text {
                            at,
                            rotation,
                            width_factor,
                            mask,
                            ..
                        },
                        Entity::Text(t),
                    ) => {
                        let ink = self.text_ink(base.color.as_deref(), layer);
                        if self.shx_text(frame, slot, t, ink) {
                            continue;
                        }
                        // Its face (docs/adr/0183 §2), else the project's typeface, italic.
                        let (font, lean) = face_font(&t.face, self.font, true, None);
                        self.draw(
                            frame,
                            &Piece {
                                text: &t.text,
                                at: self.screen(*at),
                                angle: (-rotation.to_radians()) as f32,
                                size: (t.height * self.camera.get().scale) as f32,
                                font,
                                anchor: Anchor::LeftBaseline,
                                color: self.text_ink(base.color.as_deref(), layer),
                                width_factor: *width_factor as f32,
                                mask: (mask * self.camera.get().scale) as f32,
                                underline: 0.0,
                                lean,
                            },
                            self.colors.halo,
                        );
                    }
                    // A table's cell (docs/adr/0184 §2): its words in the table's face and colour, upright,
                    // a heading row's bold.
                    (
                        LabelSpot::Cell {
                            at,
                            rotation,
                            row,
                            col,
                            bold,
                            line,
                            ..
                        },
                        Entity::Table(t),
                    ) => {
                        // As its table style draws it where it sets nothing itself (docs/adr/1126 §3, 1127 §1).
                        if drawn_table.as_ref().is_none_or(|(of, _)| *of != slot) {
                            let shape = kentos_native_application::geometry::styled_table(t, self.doc.settings());
                            drawn_table = Some((slot, shape));
                        }
                        let drawn = kentos_commands::styled(t, self.doc.settings());
                        let t: &kentos_commands::TableEntity = &drawn;
                        let Some(words) = t.cells.get(*row).and_then(|r| r.get(*col)) else {
                            continue;
                        };
                        // The line of its words this place stands for (docs/adr/1030 §7).
                        let words = kentos_geometry_core::geom::table::cell_line(words, *line);
                        let geom = drawn_table
                            .as_ref()
                            .and_then(|(_, shape)| kentos_geometry_core::geom::table::table_geom(shape));
                        // Its cell style's height, else the table's (docs/adr/1127 §1).
                        let height = geom
                            .as_ref()
                            .and_then(|g| g.style_at(*row, *col).height)
                            .unwrap_or(t.height);
                        // Its cells' fills (docs/adr/0327 §1), a pixel in from their lines.
                        if t.styles.iter().any(|s| s.fill.is_some()) && filled.insert(slot) {
                            if let Some(g) = &geom {
                                for f in g.fills() {
                                    let q = f.corners.map(|p| self.screen(p));
                                    let (cx, cy) = (
                                        q.iter().map(|p| p.x).sum::<f32>() / 4.0,
                                        q.iter().map(|p| p.y).sum::<f32>() / 4.0,
                                    );
                                    let inset = q.map(|p| {
                                        let (dx, dy) = (cx - p.x, cy - p.y);
                                        let l = (dx * dx + dy * dy).sqrt().max(1.0);
                                        Point::new(p.x + dx / l * 1.0, p.y + dy / l * 1.0)
                                    });
                                    self.fill_cell(frame, inset, &f.fill, Some(slot));
                                }
                            }
                        }
                        let heading = Run {
                            start: 0,
                            end: 0,
                            bold: *bold,
                            italic: false,
                            underline: false,
                            script: None,
                            color: None,
                        };
                        let (font, lean) = face_font(&t.face, self.font, false, Some(&heading));
                        // A cell style's colour over the table's (docs/adr/0327 §1).
                        let own = t.cell_style(*row, *col).color;
                        let color = self.ink_of(
                            own.as_deref().or(base
                                .color
                                .as_deref()
                                .or(self.layer_color(layer))),
                        );
                        // In its face's SHX text style (docs/adr/1026), as a text is.
                        if let Some(w) = kentos_native_application::shx_text::words_in(
                            self.doc.settings(),
                            &t.face,
                            kentos_commands::Vec2 { x: at.x, y: at.y },
                            *rotation,
                            height,
                            words,
                        ) && self.shx_text(frame, slot, &w, color)
                        {
                            continue;
                        }
                        self.draw(
                            frame,
                            &Piece {
                                text: words,
                                at: self.screen(*at),
                                angle: (-rotation.to_radians()) as f32,
                                size: (height * self.camera.get().scale) as f32,
                                font,
                                anchor: Anchor::LeftBaseline,
                                color,
                                width_factor: 1.0,
                                mask: 0.0,
                                underline: 0.0,
                                lean,
                            },
                            self.colors.halo,
                        );
                    }
                    // A leader's note, as a text (docs/adr/0146 §5); grown, its arrowhead with it (docs/adr/0205 §5).
                    (
                        LabelSpot::Text {
                            at, rotation, mask, ..
                        },
                        Entity::Leader(l),
                    ) => {
                        if grow.k > 1.0 {
                            let color = self.ink_of(
                                base.color
                                    .as_deref()
                                    .or(self.layer_color(layer)),
                            );
                            self.grown_arrowhead(frame, l, grow.k, color);
                        }
                        let Some(note) = l.text.as_deref() else {
                            continue;
                        };
                        // In its leader style's SHX text style (docs/adr/1025).
                        if let Some(mut t) = kentos_native_application::shx_text::leader_note(
                            self.doc.settings(),
                            l.leader_style.as_deref(),
                            kentos_commands::Vec2 { x: at.x, y: at.y },
                            *rotation,
                            l.height,
                            note,
                        ) {
                            t.mask = *mask > 0.0;
                            let ink = self.text_ink(base.color.as_deref(), layer);
                            if self.shx_text(frame, slot, &t, ink) {
                                continue;
                            }
                        }
                        let (font, lean) = face_font(&Default::default(), self.font, true, None);
                        self.draw(
                            frame,
                            &Piece {
                                text: note,
                                at: self.screen(*at),
                                angle: (-rotation.to_radians()) as f32,
                                size: (l.height * self.camera.get().scale) as f32,
                                font,
                                anchor: Anchor::LeftBaseline,
                                color: self.text_ink(base.color.as_deref(), layer),
                                width_factor: 1.0,
                                mask: (mask * self.camera.get().scale) as f32,
                                underline: 0.0,
                                lean,
                            },
                            self.colors.halo,
                        );
                    }
                    // A block's texts and dimension values (docs/adr/0144), as its own objects draw theirs;
                    // an attribute's text the insert's value, else its default (§7).
                    (
                        LabelSpot::PieceText {
                            at,
                            rotation,
                            height,
                            text,
                            attribute,
                            width_factor,
                            mask,
                            face,
                            placed,
                            ink,
                            ..
                        },
                        _,
                    ) => {
                        let shown = kentos_interaction::spatial::shown_text(
                            text,
                            attribute.as_deref(),
                            &base.attrs,
                        );
                        if shown.is_empty() {
                            continue;
                        }
                        // A block's text in its style's embedded SHX font (docs/adr/1020), as a text object is.
                        if let Some(t) = placed
                            .as_deref()
                            .and_then(|s| kentos_native_application::shx_text::piece_text(s, &shown))
                        {
                            let ink = self.piece_ink(ink, base, layer);
                            if self.shx_text(frame, slot, &t, ink) {
                                continue;
                            }
                        }
                        let (font, lean) = face_font(&contract_face(face), self.font, true, None);
                        self.draw(
                            frame,
                            &Piece {
                                text: shown,
                                at: self.screen(*at),
                                angle: (-rotation.to_radians()) as f32,
                                size: (height * self.camera.get().scale) as f32,
                                font,
                                anchor: Anchor::LeftBaseline,
                                color: self.piece_ink(ink, base, layer),
                                width_factor: *width_factor as f32,
                                mask: (mask * self.camera.get().scale) as f32,
                                underline: 0.0,
                                lean,
                            },
                            self.colors.halo,
                        );
                    }
                    (
                        LabelSpot::PieceDimension {
                            at,
                            angle,
                            value,
                            height,
                            text,
                            unit,
                            prefix,
                            mask,
                            look,
                            ..
                        },
                        _,
                    ) => {
                        let look = kentos_native_application::geometry::contract_look(look.clone());
                        let text = self.format.dimension_value(
                            prefix,
                            unit,
                            *value,
                            &look,
                            text.as_deref().filter(|t| !t.is_empty()),
                        );
                        let color = self.ink_of(
                            look.text_color
                                .as_deref()
                                .or(base.color.as_deref())
                                .or(self.layer_color(layer)),
                        );
                        if let Some(mut t) = kentos_native_application::shx_text::dimension_value(
                            self.doc.settings(),
                            look.dim_style.as_deref(),
                            kentos_commands::Vec2 { x: at.x, y: at.y },
                            *angle,
                            *height,
                            &text,
                        ) {
                            t.mask = *mask;
                            if self.shx_text(frame, slot, &t, color) {
                                continue;
                            }
                        }
                        let size = (height * self.camera.get().scale) as f32;
                        let font = drawing_fonts::font(look.font.unwrap_or(self.font), 500, false);
                        self.draw(
                            frame,
                            &Piece {
                                text: &text,
                                at: self.screen(*at),
                                angle: (-angle.to_radians()) as f32,
                                size,
                                font,
                                anchor: Anchor::CenterBaseline,
                                color,
                                width_factor: 1.0,
                                mask: value_mask(*mask, &text, font, size),
                                underline: 0.0,
                                lean: 0.0,
                            },
                            self.colors.halo,
                        );
                    }
                    // A multi-line text's mask, then its lines (docs/adr/0182 §3); a leaning one's box
                    // leans from its corner (docs/adr/0183 §2).
                    (
                        LabelSpot::ParagraphMask {
                            at,
                            rotation,
                            width,
                            height,
                            lean,
                            placed,
                            ..
                        },
                        entity,
                    ) => {
                        // An SHX text's mask is its own box, drawn with its strokes (ADR 1125); a block's piece's
                        // too (docs/adr/1022).
                        let shx_ok = |t: &kentos_commands::TextEntity| {
                            kentos_native_application::shx_text::place_cached(self.doc.settings(), self.doc.styles(), t)
                                .is_some_and(|p| p.is_ok())
                        };
                        let piece = placed.as_deref().and_then(kentos_native_application::shx_text::placed_piece);
                        if matches!(entity, Entity::Text(t) if shx_ok(t)) || piece.as_ref().is_some_and(shx_ok) {
                            continue;
                        }
                        let r = rotation.to_radians();
                        let (u, v) = (Vec2::new(r.cos(), r.sin()), Vec2::new(-r.sin(), r.cos()));
                        let corner = |x: f64, y: f64| {
                            let x = x + y * lean;
                            self.screen(Vec2::new(
                                at.x + u.x * x + v.x * y,
                                at.y + u.y * x + v.y * y,
                            ))
                        };
                        let quad = [
                            corner(0.0, 0.0),
                            corner(*width, 0.0),
                            corner(*width, *height),
                            corner(0.0, *height),
                        ];
                        if self.fence.as_ref().is_none_or(|f| f.holds(quad[0])) {
                            frame.fill_quad(quad, self.colors.halo, Some(slot));
                        }
                    }
                    (LabelSpot::Line{unicode:Some(text),at,..},_)=> {
                        let color=self.ink_of(base.color.as_deref().or(self.layer_color(layer)));
                        self.draw_unicode(frame,text,*at,color);
                    }
                    (
                        LabelSpot::Line {
                            at,
                            rotation,
                            height,
                            width_factor,
                            start,
                            end,
                            piece,
                            face,
                            placed,
                            ink,
                            ..
                        },
                        entity,
                    ) => {
                        // A block's multi-line text in its SHX font, whole at its first line (docs/adr/1022).
                        if let Some(t) = placed
                            .as_deref()
                            .and_then(kentos_native_application::shx_text::placed_piece)
                            .filter(|t| {
                                kentos_native_application::shx_text::place_cached(self.doc.settings(), self.doc.styles(), t)
                                    .is_some_and(|p| p.is_ok())
                            })
                        {
                            if *start == 0 {
                                let ink = self.piece_ink(ink, base, layer);
                                self.shx_text(frame, slot, &t, ink);
                            }
                            continue;
                        }
                        let own;
                        let (text, runs, face): (&str, &[Run], kentos_commands::TextFace) =
                            match (piece, entity) {
                                (Some(p), _) => (
                                    &p.0,
                                    &p.1,
                                    face.as_ref().map(contract_face).unwrap_or_default(),
                                ),
                                (None, Entity::Text(t)) => {
                                    // An SHX text is drawn whole at its first line's place (ADR 1125).
                                    let ink = self.text_ink(base.color.as_deref(), layer);
                                    if (*start > 0 || self.shx_text(frame, slot, t, ink))
                                        && kentos_native_application::shx_text::place_cached(self.doc.settings(), self.doc.styles(), t)
                                            .is_some_and(|p| p.is_ok())
                                    {
                                        continue;
                                    }
                                    own = core_runs(&t.paragraph.runs).unwrap_or_default();
                                    (&t.text, &own, t.face.clone())
                                }
                                _ => continue,
                            };
                        // Its words' colour where a run gives none (docs/adr/8053 §14): a block's piece's, a text's own.
                        let default = match piece {
                            Some(_) => self.piece_ink(ink, base, layer),
                            None => self.text_ink(base.color.as_deref(), layer),
                        };
                        self.line(
                            frame,
                            text,
                            runs,
                            (*start, *end),
                            *at,
                            *rotation,
                            *height,
                            *width_factor,
                            &face,
                            default,
                        );
                    }
                    (
                        LabelSpot::Placed {
                            at,
                            angle,
                            width,
                            height,
                            class,
                            state,
                            ..
                        },
                        entity,
                    ) => {
                        current = self.placed_frame(entity, layer, *class, *state, slot);
                        if let Some(c) = &current {
                            self.placed_backdrop(
                                frame,
                                c,
                                *at,
                                *angle,
                                *width as f32,
                                *height as f32,
                                false,
                            );
                        }
                    }
                    (
                        LabelSpot::PlacedLine {
                            at,
                            angle,
                            size,
                            text,
                            ..
                        },
                        _,
                    ) => {
                        if let Some(c) = current.as_ref().filter(|c| c.slot == slot) {
                            self.placed_text(frame, c, text, *at, *angle, *size as f32);
                        }
                    }
                    (
                        LabelSpot::PlacedLetter {
                            at,
                            angle,
                            size,
                            letter,
                            advance,
                            ..
                        },
                        _,
                    ) => {
                        if let Some(c) = current.as_ref().filter(|c| c.slot == slot) {
                            self.placed_backdrop(
                                frame,
                                c,
                                *at,
                                *angle,
                                *advance as f32,
                                *size as f32,
                                true,
                            );
                            let mut buf = [0u8; 4];
                            self.placed_text(
                                frame,
                                c,
                                letter.encode_utf8(&mut buf),
                                *at,
                                *angle,
                                *size as f32,
                            );
                        }
                    }
                    (LabelSpot::PlacedCallout { from, to, .. }, _) => {
                        if let Some(c) = current.as_ref().filter(|c| c.slot == slot) {
                            let (a, b) = (self.screen(*from), self.screen(*to));
                            let pts = if c.manhattan {
                                vec![a, Point::new(b.x, a.y), b]
                            } else {
                                vec![a, b]
                            };
                            frame.stroke_line(&pts, false, c.callout, Some(c.slot));
                        }
                    }
                    (LabelSpot::Dimension { .. } | LabelSpot::Text { .. }, _) => {}
                    _ => {}
                }
            }
            // A part that drew nothing but bands keeps them for the next painting in this look.
            if let (Some(m), Some(slot)) = (kept, of)
                && frame.ops == made.1
                && let Some(entity) = self.doc.get_shared(slot)
            {
                m.keep(
                    part,
                    entity,
                    (self.layer_color_of(slot), self.cells_print(slot, part, &tables)),
                    &self.bands.borrow()[made.0..],
                );
            }
        }
        if let Some(m) = memo.as_deref_mut() {
            m.end();
        }
        let frame = &mut *frame.ink;
        // The view's own camera again for what follows.
        self.camera.set(self.view);
        // The multi-line text being written or edited, over the rest (docs/adr/0182 §4).
        if let Some(p) = &self.preview {
            if let Some(text)=&p.unicode {
                if let Some(r)=p.records.chunks_exact(kentos_geometry_core::store::labels::LABEL_STRIDE).next() {
                    self.draw_unicode(frame,text,Vec2::new(r[2],r[3]),self.colors.label);
                }
            } else {
            for r in p
                .records
                .chunks_exact(kentos_geometry_core::store::labels::LABEL_STRIDE)
            {
                let at = Vec2::new(r[2], r[3]);
                if r[1] == kentos_geometry_core::store::labels::LABEL_PARAGRAPH_MASK {
                    let rad = r[4].to_radians();
                    let (u, v) = (
                        Vec2::new(rad.cos(), rad.sin()),
                        Vec2::new(-rad.sin(), rad.cos()),
                    );
                    let lean = kentos_native_application::geometry::core_face(&p.face).lean();
                    let corner = |x: f64, y: f64| {
                        let x = x + y * lean;
                        self.screen(Vec2::new(
                            at.x + u.x * x + v.x * y,
                            at.y + u.y * x + v.y * y,
                        ))
                    };
                    let quad = [
                        corner(0.0, 0.0),
                        corner(r[5], 0.0),
                        corner(r[5], r[6]),
                        corner(0.0, r[6]),
                    ];
                    frame.fill_quad(quad, self.colors.halo, None);
                } else {
                    self.line(
                        frame,
                        &p.text,
                        &p.runs,
                        (r[7] as usize, r[8] as usize),
                        at,
                        r[4],
                        r[5],
                        r[6],
                        &p.face,
                        self.colors.label,
                    );
                }
            }
        }
        }
        // The texts too small to read, as faint bands, one stroke a colour and thickness (docs/adr/0626 §6): a
        // stroke's tessellation is linear where a fill's sweeps and sorts, and a butt-ended stroke as thick as the
        // band is its rectangle.
        if self.bands_out.is_none() {
            for (color, thick, segments) in self.band_looks() {
                frame.stroke_segments(&segments, color, thick);
            }
        }
    }
}
