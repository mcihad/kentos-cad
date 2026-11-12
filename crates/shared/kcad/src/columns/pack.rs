//! Objects packed into the columns (`Packer`): each kind's fields and geometry laid out as the table says.

use super::*;

impl Packer {
    pub(super) fn int(&mut self, v: u32) {
        self.out.ints.push(v);
    }

    pub(super) fn float(&mut self, x: f64) {
        self.out.floats.push(x);
    }

    pub(super) fn point(&mut self, p: &Vec2) {
        let Vec2 { x, y } = *p;
        self.out.floats.extend([x, y]);
    }

    /// A list of points: its length, then its coordinates.
    pub(super) fn points(&mut self, list: &[Vec2]) {
        self.int(count(list.len()));
        self.out.floats.reserve(list.len() * 2);
        for p in list {
            self.point(p);
        }
    }

    /// A list of numbers (bulges): its length, then the numbers.
    pub(super) fn numbers(&mut self, list: &[f64]) {
        self.int(count(list.len()));
        self.out.floats.extend_from_slice(list);
    }

    /// A polygon's or a part's holes: their count, then each hole's flags,
    /// vertices, bulges and elevations.
    pub(super) fn holes(&mut self, holes: &[RingGeometry]) {
        self.int(count(holes.len()));
        for RingGeometry { pts, bulges, zs } in holes {
            let mut hole = 0;
            if bulges.is_some() {
                hole |= HOLE_BULGES;
            }
            if zs.is_some() {
                hole |= HOLE_ELEVATIONS;
            }
            self.int(hole);
            self.points(pts);
            if let Some(b) = bulges {
                self.numbers(b);
            }
            if let Some(z) = zs {
                self.elevations(z);
            }
        }
    }

    /// A list of elevations: its length, then the numbers; NaN stands for a
    /// vertex without one.
    pub(super) fn elevations(&mut self, list: &[Option<f64>]) {
        self.int(count(list.len()));
        self.out
            .floats
            .extend(list.iter().map(|z| z.unwrap_or(f64::NAN)));
    }

    pub(super) fn text(&mut self, s: &str) {
        let start = self.out.text.len();
        self.out.text.extend(s.encode_utf16());
        let units = self.out.text.len() - start;
        self.out.text_lengths.push(count(units));
    }

    /// One object, its layer already a table index. Out of line: see `Encoder::object`.
    #[inline(never)]
    pub(super) fn object(&mut self, entity: &Entity, uid: &EntityId, layer: u32) {
        self.out.kinds.push(kind_index(entity));
        self.out.uids.extend_from_slice(&uid.0);
        // Every field is named, so a field added to the contract cannot be left out unnoticed.
        let EntityBase {
            id: _slot,
            layer_id: _,
            color,
            attrs,
            label,
            symbol,
            line_weight,
            label_pins,
            attachments,
            look,
        } = entity.base();
        let flags_at = self.out.ints.len() + 1;
        self.out.ints.extend([layer, 0, count(attrs.len())]);
        let mut flags = 0;
        if let Some(w) = line_weight {
            flags |= WEIGHT;
            self.float(*w);
        }
        for (bit, value) in [(COLOR, color), (LABEL, label), (SYMBOL, symbol)] {
            if let Some(v) = value {
                flags |= bit;
                self.text(v);
            }
        }
        if !label_pins.is_empty() {
            flags |= PINS;
            self.int(count(label_pins.len()));
            for p in label_pins {
                let bits = [
                    (PIN_CLASS, p.class.is_some()),
                    (PIN_AT, p.at.is_some()),
                    (PIN_ROTATION, p.rotation.is_some()),
                    (PIN_HIDDEN, p.hidden == Some(true)),
                ]
                .iter()
                .filter(|(_, on)| *on)
                .fold(0, |m, (b, _)| m | b);
                self.int(bits);
                if let Some(c) = &p.class {
                    self.text(c);
                }
                if let Some(a) = p.at {
                    self.float(a.x);
                    self.float(a.y);
                }
                if let Some(r) = p.rotation {
                    self.float(r);
                }
            }
        }
        if !attachments.is_empty() {
            flags |= ATTACHMENTS;
            self.int(count(attachments.len()));
            for a in attachments {
                let fields = [
                    (ATT_ASSET, &a.asset),
                    (ATT_FILE, &a.file),
                    (ATT_URL, &a.url),
                    (ATT_NOTE, &a.note),
                ];
                let bits = fields
                    .iter()
                    .filter(|(_, v)| v.is_some())
                    .fold(0, |m, (b, _)| m | b);
                self.int(bits);
                self.text(&a.name);
                for (_, v) in fields {
                    if let Some(v) = v {
                        self.text(v);
                    }
                }
            }
        }
        if !look.is_empty() {
            flags |= LOOK;
            let bits = [
                (LOOK_TYPE, look.line_type.is_some()),
                (LOOK_SCALE, look.line_type_scale.is_some()),
                (LOOK_TRANSPARENCY, look.transparency.is_some()),
                (LOOK_ORDER, look.order.is_some()),
                (LOOK_BY_LAYER, !look.by_layer.is_empty()),
                (LOOK_WIPEOUT, look.wipeout.is_some()),
                (LOOK_FRAME, look.wipeout.is_some_and(|w| w.frame)),
                (LOOK_PATTERN, look.line_pattern.is_some()),
            ]
            .iter()
            .filter(|(_, on)| *on)
            .fold(0, |m, (b, _)| m | b);
            self.int(bits);
            if let Some(t) = look.line_type {
                self.int(count(
                    LineType::ALL.iter().position(|x| *x == t).unwrap_or(0),
                ));
            }
            for x in [look.line_type_scale, look.transparency]
                .into_iter()
                .flatten()
            {
                self.float(x);
            }
            if let Some(o) = look.order {
                self.float(f64::from(o));
            }
            if !look.by_layer.is_empty() {
                let mask = look.by_layer.iter().fold(0, |m, p| {
                    m | 1 << LookPart::ALL.iter().position(|x| x == p).unwrap_or(0)
                });
                self.int(mask);
            }
            if let Some(p) = &look.line_pattern {
                self.text(p);
            }
        }
        for (key, value) in attrs {
            self.text(key);
            self.text(value);
        }
        flags |= self.geometry(entity);
        self.out.ints[flags_at] = flags;
    }

    /// The kind's own fields; returns their flags.
    pub(super) fn geometry(&mut self, entity: &Entity) -> u32 {
        let mut flags = 0;
        match entity {
            Entity::Point(PointEntity {
                base: _,
                p,
                z,
                parts,
            }) => {
                self.point(p);
                if let Some(z) = z {
                    flags |= OPT[0];
                    self.float(*z);
                }
                // A multi-point object's other points (docs/adr/0174): each its flag, place and elevation.
                if let Some(parts) = parts {
                    flags |= OPT[1];
                    self.int(count(parts.len()));
                    for PointPart { p, z } in parts {
                        self.int(u32::from(z.is_some()));
                        self.point(p);
                        if let Some(z) = z {
                            self.float(*z);
                        }
                    }
                }
            }
            Entity::Line(LineEntity {
                base: _,
                a,
                b,
                za,
                zb,
                ma,
                mb,
            }) => {
                self.point(a);
                self.point(b);
                for (bit, z) in [(OPT[0], za), (OPT[1], zb), (OPT[2], ma), (OPT[3], mb)] {
                    if let Some(z) = z {
                        flags |= bit;
                        self.float(*z);
                    }
                }
            }
            Entity::Polyline(path) | Entity::Polygon(path) => {
                let PathEntity {
                    space_separator,
                    base: _,
                    pts,
                    bulges,
                    holes,
                    zs,
                    ms,
                    parts,
                    closed,
                    widths,
                } = path;
                self.points(pts);
                if let Some(b) = bulges {
                    flags |= OPT[0];
                    self.numbers(b);
                }
                if let Some(z) = zs {
                    flags |= OPT[2];
                    self.elevations(z);
                }
                if let Some(m) = ms {
                    flags |= OPT[4];
                    self.elevations(m);
                }
                if let Some(holes) = holes {
                    flags |= OPT[1];
                    self.holes(holes);
                }
                if let Some(parts) = parts {
                    flags |= OPT[3];
                    self.int(count(parts.len()));
                    for AreaPart {
                        pts,
                        bulges,
                        holes,
                        zs,
                        ms,
                    } in parts
                    {
                        let mut part = 0;
                        if bulges.is_some() {
                            part |= PART_BULGES;
                        }
                        if zs.is_some() {
                            part |= PART_ELEVATIONS;
                        }
                        if holes.is_some() {
                            part |= PART_HOLES;
                        }
                        if ms.is_some() {
                            part |= PART_MEASURES;
                        }
                        self.int(part);
                        self.points(pts);
                        if let Some(b) = bulges {
                            self.numbers(b);
                        }
                        if let Some(z) = zs {
                            self.elevations(z);
                        }
                        if let Some(m) = ms {
                            self.elevations(m);
                        }
                        if let Some(holes) = holes {
                            self.holes(holes);
                        }
                    }
                }
                // A closed and a wide polyline (docs/adr/0258): after its parts.
                if *closed {
                    flags |= OPT[5];
                }
                if let Some(widths) = widths {
                    flags |= OPT[6];
                    let flat: Vec<f64> = widths.iter().flatten().copied().collect();
                    self.numbers(&flat);
                }
                if let Some(role) = space_separator {
                    flags |= OPT[7];
                    self.text(&serde_json::to_string(role).unwrap_or_default());
                }
            }
            Entity::Circle(CircleEntity { base: _, c, r }) => {
                self.point(c);
                self.float(*r);
            }
            Entity::Arc(ArcEntity {
                base: _,
                c,
                r,
                a0,
                a1,
            }) => {
                self.point(c);
                self.out.floats.extend([*r, *a0, *a1]);
            }
            Entity::Ellipse(EllipseEntity {
                base: _,
                c,
                major,
                ratio,
                t0,
                t1,
            }) => {
                self.point(c);
                self.point(major);
                self.out.floats.extend([*ratio, *t0, *t1]);
            }
            Entity::Spline(SplineEntity {
                base: _,
                pts,
                closed,
            }) => {
                self.points(pts);
                self.int(u32::from(*closed));
            }
            // Every field named, as the base's (docs/adr/0259).
            Entity::Nurbs(kentos_contracts::NurbsEntity {
                base: _,
                degree,
                ctrl,
                knots,
                weights,
                closed,
                periodic,
                fit,
                tangents,
                fit_tolerance,
            }) => {
                let [start, end] = tangents.unwrap_or([None, None]);
                let flags = [
                    (1, weights.is_some()),
                    (2, *closed),
                    (4, *periodic),
                    (8, fit.is_some()),
                    (16, fit_tolerance.is_some()),
                    (32, start.is_some()),
                    (64, end.is_some()),
                ]
                .iter()
                .filter(|(_, on)| *on)
                .fold(0, |m, (b, _)| m | b);
                self.int(*degree);
                self.points(ctrl);
                self.numbers(knots);
                self.int(flags);
                if let Some(w) = weights {
                    self.numbers(w);
                }
                if let Some(f) = fit {
                    self.points(f);
                }
                if let Some(t) = fit_tolerance {
                    self.float(*t);
                }
                for t in [start, end].into_iter().flatten() {
                    self.point(&t);
                }
            }
            Entity::Xline(line) | Entity::Ray(line) => {
                let ConstructionEntity { base: _, p, dir } = line;
                self.point(p);
                self.point(dir);
            }
            Entity::Text(TextEntity {
                annotative,
                base: _,
                p,
                text,
                height,
                rotation,
                align,
                width_factor,
                mask,
                label_of,
                label_scale,
                paragraph,
                face,
                path,
            }) => {
                self.point(p);
                self.out.floats.extend([*height, *rotation]);
                self.text(text);
                if let Some(a) = align {
                    flags |= OPT[0];
                    let at = TextAlign::ALL.iter().position(|x| x == a).unwrap_or(0);
                    self.int(count(at));
                }
                if let Some(w) = width_factor {
                    flags |= OPT[1];
                    self.float(*w);
                }
                if *mask {
                    flags |= OPT[2];
                }
                // A linked text's object, as its text, and scale (docs/adr/0175 §4); the codec checks the pair.
                if label_of.is_some() || label_scale.is_some() {
                    flags |= OPT[3];
                    self.text(&label_of.map(|u| u.to_text()).unwrap_or_default());
                    self.float(label_scale.unwrap_or(f64::NAN));
                }
                // A multi-line text's box, line spacing and runs (docs/adr/0182): each run its
                // range, its flags (`run_flags`) and, with RUN_COLOR, its colour.
                if let Some(w) = paragraph.box_width {
                    flags |= OPT[4];
                    self.float(w);
                }
                if let Some(s) = paragraph.line_spacing {
                    flags |= OPT[5];
                    self.float(s);
                }
                if !paragraph.runs.is_empty() {
                    flags |= OPT[6];
                    self.int(count(paragraph.runs.len()));
                    for r in &paragraph.runs {
                        self.int(r.start);
                        self.int(r.end);
                        self.int(run_flags(r));
                        if let Some(c) = &r.color {
                            self.text(c);
                        }
                    }
                }
                // Its face (docs/adr/0183 §2): the style's id, the typeface's place, bold and italic as
                // flags, the slant.
                if let Some(id) = &face.text_style {
                    flags |= OPT[7];
                    self.text(id);
                }
                if let Some(f) = face.font {
                    flags |= OPT[8];
                    self.int(font_place(f));
                }
                if face.bold {
                    flags |= OPT[9];
                }
                if face.italic {
                    flags |= OPT[10];
                }
                if let Some(o) = face.oblique {
                    flags |= OPT[11];
                    self.float(o);
                }
                // Its curve (docs/adr/0196 §1): its vertices, then its bulges when one bends.
                if let Some(c) = path {
                    flags |= OPT[12];
                    self.points(&c.pts);
                    if let Some(b) = &c.bulges {
                        flags |= OPT[13];
                        self.numbers(b);
                    }
                }
                // Its overrides (docs/adr/0322 §5), as a JSON list of their names.
                if !face.overrides.is_empty() {
                    flags |= OPT[14];
                    self.text(&serde_json::to_string(&face.overrides).unwrap_or_default());
                }
                // Its scales (docs/adr/0323), as their JSON.
                if let Some(a) = annotative {
                    flags |= OPT[15];
                    self.text(&serde_json::to_string(a).unwrap_or_default());
                }
                // Its indents, tabs and columns (docs/adr/0324 §1), as their JSON.
                if let Some(l) = &paragraph.layout {
                    flags |= OPT[16];
                    self.text(&serde_json::to_string(l).unwrap_or_default());
                }
                // Its fields (docs/adr/0325 §1), as their JSON.
                if !paragraph.fields.is_empty() {
                    flags |= OPT[17];
                    self.text(&serde_json::to_string(&paragraph.fields).unwrap_or_default());
                }
            }
            Entity::Dimension(DimensionEntity {
                annotative,
                text_shift,
                breaks,
                base: _,
                a,
                b,
                offset,
                height,
                text,
                style,
                angle,
                c,
                mask,
                za,
                zb,
                refs,
                look,
                level,
            }) => {
                self.point(a);
                self.point(b);
                self.out.floats.extend([*offset, *height]);
                if let Some(t) = text {
                    flags |= OPT[0];
                    self.text(t);
                }
                if let Some(s) = style {
                    flags |= OPT[1];
                    let at = DIMENSION_STYLES.iter().position(|x| x == s).unwrap_or(0);
                    self.int(count(at));
                }
                if let Some(x) = angle {
                    flags |= OPT[2];
                    self.float(*x);
                }
                if let Some(c) = c {
                    flags |= OPT[3];
                    self.point(c);
                }
                if *mask {
                    flags |= OPT[4];
                }
                if let Some(z) = za {
                    flags |= OPT[5];
                    self.float(*z);
                }
                if let Some(z) = zb {
                    flags |= OPT[6];
                    self.float(*z);
                }
                // Its look (docs/adr/0183 §3), field by field in the contract's order.
                if let Some(id) = &look.dim_style {
                    flags |= OPT[7];
                    self.text(id);
                }
                if let Some(a) = look.arrow {
                    flags |= OPT[8];
                    let at = DimensionArrow::ALL
                        .iter()
                        .position(|x| *x == a)
                        .unwrap_or(0);
                    self.int(count(at));
                }
                for (k, v) in [
                    (9, look.arrow_size),
                    (10, look.ext_offset),
                    (11, look.ext_beyond),
                    (12, look.text_gap),
                ] {
                    if let Some(x) = v {
                        flags |= OPT[k];
                        self.float(x);
                    }
                }
                if look.text_place.is_some() {
                    flags |= OPT[13];
                }
                if let Some(d) = look.decimals {
                    flags |= OPT[14];
                    self.int(d);
                }
                if let Some(u) = look.unit {
                    flags |= OPT[15];
                    self.int(count(UNITS.iter().position(|x| *x == u).unwrap_or(0)));
                }
                if let Some(t) = &look.prefix {
                    flags |= OPT[16];
                    self.text(t);
                }
                if let Some(t) = &look.suffix {
                    flags |= OPT[17];
                    self.text(t);
                }
                if let Some(f) = look.font {
                    flags |= OPT[18];
                    self.int(font_place(f));
                }
                if look.has_lines() {
                    flags |= LINES;
                    let mut line = 0;
                    for (bit, on) in [
                        (LINE_COLOR, look.dim_line_color.is_some()),
                        (LINE_WEIGHT, look.dim_line_weight.is_some()),
                        (LINE_TYPE, look.dim_line_type.is_some()),
                        (EXT_COLOR, look.ext_color.is_some()),
                        (EXT_WEIGHT, look.ext_weight.is_some()),
                        (EXT_LINE_TYPE, look.ext_line_type.is_some()),
                        (TEXT_COLOR, look.text_color.is_some()),
                    ] {
                        if on {
                            line |= bit;
                        }
                    }
                    self.int(line);
                    for t in [look.dim_line_type, look.ext_line_type]
                        .into_iter()
                        .flatten()
                    {
                        self.int(line_type_place(t));
                    }
                    for w in [look.dim_line_weight, look.ext_weight]
                        .into_iter()
                        .flatten()
                    {
                        self.float(w);
                    }
                    for c in [&look.dim_line_color, &look.ext_color, &look.text_color]
                        .into_iter()
                        .flatten()
                    {
                        self.text(c);
                    }
                }
                if let Some(refs) = refs {
                    flags |= REFS;
                    let mask = [&refs.a, &refs.b, &refs.c]
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.is_some())
                        .fold(0, |m, (i, _)| m | 1 << i);
                    // The rule in bits 3 to 5 (docs/adr/0320 §1): the curve's 3, the edges' 4, the pair's both
                    // (docs/adr/0322), the cut's 5 (docs/adr/0711).
                    let rule = match refs.rule {
                        None => 0,
                        Some(kentos_contracts::DimensionRule::Curve) => 1 << 3,
                        Some(kentos_contracts::DimensionRule::Edges) => 1 << 4,
                        Some(kentos_contracts::DimensionRule::Pair) => 3 << 3,
                        Some(kentos_contracts::DimensionRule::Cross) => 1 << 5,
                    };
                    self.int(mask | rule);
                    for (_, r) in refs.each() {
                        let (kind, numbers) = match r.place {
                            SubPlace::Node { part } => (0, [part, 0, 0]),
                            SubPlace::Start => (1, [0; 3]),
                            SubPlace::End => (2, [0; 3]),
                            SubPlace::Center => (3, [0; 3]),
                            SubPlace::Vertex { part, ring, index } => (4, [part, ring, index]),
                            SubPlace::Mid { part, ring, index } => (5, [part, ring, index]),
                            SubPlace::Along {
                                part, ring, index, ..
                            } => (6, [part, ring, index]),
                        };
                        let in_block = !r.path.is_empty();
                        self.int(kind + if in_block { IN_BLOCK } else { 0 });
                        for x in &numbers[..PLACE_KINDS[kind as usize].1] {
                            self.int(*x);
                        }
                        if let SubPlace::Along { t, .. } = r.place {
                            self.float(t);
                        }
                        if in_block {
                            self.int(r.path.len() as u32);
                            for x in &r.path {
                                self.int(*x);
                            }
                        }
                        self.int(u32::from(r.broken.is_some()));
                        self.text(&r.uid.to_text());
                        if let Some(b) = &r.broken {
                            self.text(b);
                        }
                    }
                }
                if look.options.is_some()
                    || text_shift.is_some()
                    || !breaks.is_empty()
                    || !look.overrides.is_empty()
                    || annotative.is_some()
                    || level.is_some()
                {
                    flags |= MORE;
                    let mut more = serde_json::Map::new();
                    // A Kot's level (docs/adr/0721 §2), as its JSON.
                    if let Some(l) = level {
                        more.insert("level".into(), serde_json::to_value(l).unwrap_or_default());
                    }
                    if let Some(a) = annotative {
                        more.insert(
                            "annotative".into(),
                            serde_json::to_value(a).unwrap_or_default(),
                        );
                    }
                    if !look.overrides.is_empty() {
                        more.insert(
                            "overrides".into(),
                            serde_json::to_value(&look.overrides).unwrap_or_default(),
                        );
                    }
                    if let Some(o) = &look.options {
                        more.insert(
                            "options".into(),
                            serde_json::to_value(o).unwrap_or_default(),
                        );
                    }
                    if let Some(t) = text_shift {
                        more.insert(
                            "textShift".into(),
                            serde_json::to_value(t).unwrap_or_default(),
                        );
                    }
                    if !breaks.is_empty() {
                        more.insert(
                            "breaks".into(),
                            serde_json::to_value(breaks).unwrap_or_default(),
                        );
                    }
                    self.text(&serde_json::Value::Object(more).to_string());
                }
            }
            Entity::Hatch(HatchEntity {
                base: _,
                ring,
                holes,
                pattern,
                assoc,
                boundary,
                annotative,
            }) => {
                let HatchPattern {
                    kind,
                    angle,
                    spacing,
                    name,
                    scale,
                    lines,
                    gradient,
                    origin,
                } = pattern;
                self.points(ring);
                let at = HATCH_PATTERNS.iter().position(|x| x == kind).unwrap_or(0);
                self.int(count(at));
                self.out.floats.extend([*angle, *spacing]);
                if let Some(holes) = holes {
                    flags |= OPT[0];
                    self.int(count(holes.len()));
                    for hole in holes {
                        self.points(hole);
                    }
                }
                // A pattern's name, scale and families, a gradient, the objects it follows (docs/adr/0186).
                if let Some(n) = name {
                    flags |= OPT[1];
                    self.text(n);
                }
                if let Some(x) = scale {
                    flags |= OPT[2];
                    self.float(*x);
                }
                if let Some(lines) = lines {
                    flags |= OPT[3];
                    self.int(count(lines.len()));
                    for l in lines {
                        self.int(count(l.dashes.len()));
                        self.out.floats.extend([
                            l.angle,
                            l.origin[0],
                            l.origin[1],
                            l.offset[0],
                            l.offset[1],
                        ]);
                        self.out.floats.extend_from_slice(&l.dashes);
                    }
                }
                if let Some(g) = gradient {
                    flags |= OPT[4];
                    let shape = GRADIENT_SHAPES
                        .iter()
                        .position(|x| *x == g.shape)
                        .unwrap_or(0);
                    self.int(count(shape));
                    self.int(u32::from(g.inverted));
                    self.text(&g.color2);
                }
                if let Some(a) = assoc {
                    flags |= OPT[5];
                    self.int(count(a.islands.len()));
                    self.int(count(a.cutouts.len()));
                    // A face's tie names no closed object (docs/adr/0328 §3): its id empty, its bounds below.
                    self.text(&a.outer.map(|o| o.to_text()).unwrap_or_default());
                    for id in a.islands.iter().chain(&a.cutouts) {
                        self.text(&id.to_text());
                    }
                    self.out.floats.extend([a.seed.x, a.seed.y]);
                }
                // Schema 69's exact boundary and a face's bounds (docs/adr/0328): one JSON object.
                let bounds = assoc.as_ref().map_or(&[][..], |a| &a.bounds[..]);
                let gap = assoc.as_ref().and_then(|a| a.gap);
                if boundary.is_some() || !bounds.is_empty() || gap.is_some() || origin.is_some() || annotative.is_some()
                {
                    flags |= OPT[6];
                    let mut more = serde_json::Map::new();
                    // Schema 71 (docs/adr/0331): the pattern's own origin and the hatch's scales.
                    if let Some(o) = origin {
                        more.insert("origin".into(), serde_json::to_value(o).unwrap_or_default());
                    }
                    if let Some(a) = annotative {
                        more.insert("annotative".into(), serde_json::to_value(a).unwrap_or_default());
                    }
                    if let Some(b) = boundary {
                        more.insert("boundary".into(), serde_json::to_value(b).unwrap_or_default());
                    }
                    if !bounds.is_empty() {
                        more.insert("bounds".into(), serde_json::to_value(bounds).unwrap_or_default());
                    }
                    if let Some(g) = gap {
                        more.insert("gap".into(), serde_json::json!(g));
                    }
                    self.text(&serde_json::Value::Object(more).to_string());
                }
            }
            Entity::Insert(InsertEntity {
                base: _,
                block,
                p,
                scale,
                y_scale,
                rotation,
                mirror,
                attribute_places,
                parameters,
                clipping,
                arch,
            }) => {
                self.point(p);
                self.out.floats.extend([*scale, *rotation]);
                if let Some(y) = y_scale {
                    flags |= OPT[1];
                    self.out.floats.push(*y);
                }
                self.text(&block.to_text());
                if *mirror {
                    flags |= OPT[0];
                }
                // Its moved attributes (docs/adr/0317 §1): a count, then each tag and point.
                if !attribute_places.is_empty() {
                    flags |= OPT[2];
                    self.int(count(attribute_places.len()));
                    for (tag, at) in attribute_places {
                        self.text(tag);
                        self.point(at);
                    }
                }
                // Its parameter values (docs/adr/0318 §3): their JSON text.
                if !parameters.is_empty() {
                    flags |= OPT[3];
                    self.text(&serde_json::to_string(parameters).unwrap_or_default());
                }
                // Its clipping (docs/adr/0351 §1): its JSON text.
                if let Some(c) = clipping {
                    flags |= OPT[4];
                    self.text(&serde_json::to_string(c).unwrap_or_default());
                }
                // Its level and phase (docs/adr/0733 §1): their JSON text.
                if !arch.is_empty() {
                    flags |= OPT[5];
                    self.text(&serde_json::to_string(arch).unwrap_or_default());
                }
            }
            Entity::Image(ImageEntity {
                base: _,
                image:
                    ImageFields {
                        p,
                        width,
                        height,
                        rotation,
                        mirror,
                        asset,
                        file,
                        clip,
                        opacity,
                    },
            }) => {
                self.point(p);
                self.out.floats.extend([*width, *height, *rotation]);
                if *mirror {
                    flags |= OPT[0];
                }
                if let Some(a) = asset {
                    flags |= OPT[1];
                    self.text(a);
                }
                if let Some(f) = file {
                    flags |= OPT[2];
                    self.text(f);
                }
                if let Some(clip) = clip {
                    flags |= OPT[3];
                    self.points(clip);
                }
                if let Some(o) = opacity {
                    flags |= OPT[4];
                    self.float(*o);
                }
            }
            Entity::Raster(RasterEntity {
                base: _,
                raster:
                    RasterFields {
                        affine,
                        width,
                        height,
                        bands,
                        sample,
                        asset,
                        file,
                        url,
                        srid,
                        style,
                        opacity,
                        dataset,
                        georef,
                        tiles,
                    },
            }) => {
                let place = RASTER_SAMPLES.iter().position(|s| s == sample).unwrap_or(0);
                self.out
                    .ints
                    .extend([*width, *height, *bands, count(place), *srid]);
                // The look field by field: its kind, bands, stretch and what it has of the rest.
                let render = RASTER_RENDERS
                    .iter()
                    .position(|r| *r == style.render)
                    .unwrap_or(0);
                self.out
                    .ints
                    .extend([count(render), count(style.bands.len())]);
                self.out.ints.extend(&style.bands);
                let stretch = RASTER_STRETCHES
                    .iter()
                    .position(|r| *r == style.stretch)
                    .unwrap_or(0);
                let numbers = [
                    (LOOK_MIN, style.min),
                    (LOOK_MAX, style.max),
                    (LOOK_AZIMUTH, style.azimuth),
                    (LOOK_ALTITUDE, style.altitude),
                    (LOOK_Z_FACTOR, style.z_factor),
                    (LOOK_NODATA, style.nodata),
                ];
                let mut look = numbers
                    .iter()
                    .filter(|(_, v)| v.is_some())
                    .fold(0, |m, (b, _)| m | b);
                if style.ramp.is_some() {
                    look |= LOOK_RAMP;
                }
                if style.invert {
                    look |= LOOK_INVERT;
                }
                if style.resampling == kentos_contracts::RasterResampling::Nearest {
                    look |= LOOK_NEAREST;
                }
                if style.edges.is_some() {
                    look |= LOOK_EDGES;
                }
                self.out.ints.extend([count(stretch), look]);
                self.out.floats.extend(affine);
                if let Some(o) = opacity {
                    flags |= OPT[2];
                    self.float(*o);
                }
                for (_, v) in numbers {
                    if let Some(v) = v {
                        self.float(v);
                    }
                }
                if let Some(r) = &style.ramp {
                    self.text(r);
                }
                if let Some(e) = &style.edges {
                    self.text(e);
                }
                if let Some(a) = asset {
                    flags |= OPT[0];
                    self.text(a);
                }
                if let Some(f) = file {
                    flags |= OPT[1];
                    self.text(f);
                }
                if let Some(u) = url {
                    flags |= OPT[3];
                    self.text(u);
                }
                // A NetCDF variable and its slice (docs/adr/0243 §6): its dimensions' count and
                // what it has, each dimension's index, value count and what it has; their values;
                // the names and units.
                if let Some(d) = dataset {
                    flags |= OPT[4];
                    let has = u32::from(d.follow_time)
                        | (u32::from(d.vector.is_some()) << 1)
                        | (u32::from(d.mesh.is_some()) << 2);
                    self.out.ints.extend([count(d.dims.len()), has]);
                    self.text(&d.variable);
                    if let Some(v) = &d.vector {
                        self.text(v);
                    }
                    if let Some(m) = &d.mesh {
                        self.text(m);
                    }
                    for dim in &d.dims {
                        let has = u32::from(dim.time) | (u32::from(dim.units.is_some()) << 1);
                        self.out
                            .ints
                            .extend([dim.index, count(dim.values.len()), has]);
                        for &v in &dim.values {
                            self.float(v);
                        }
                        self.text(&dim.name);
                        if let Some(u) = &dim.units {
                            self.text(u);
                        }
                    }
                }
                // Its georeference record (docs/adr/0518 §1), as one JSON text.
                if let Some(g) = georef {
                    flags |= OPT[5];
                    self.text(&serde_json::to_string(g).unwrap_or_default());
                }
                // The GeoPackage tile table it shows (docs/adr/0627 §3), as one JSON text.
                if let Some(t) = tiles {
                    flags |= OPT[6];
                    self.text(&serde_json::to_string(t).unwrap_or_default());
                }
            }
            // A record has no fields of its own (docs/adr/0410 §2).
            Entity::Record(_) => {}
            // A wall (docs/adr/0339 §4): its path, then the rest of its source as one JSON text.
            Entity::Wall(kentos_contracts::WallEntity { base: _, wall }) => {
                self.points(&wall.pts);
                self.int(u32::from(wall.bulges.is_some()));
                if let Some(b) = &wall.bulges {
                    self.numbers(b);
                }
                let mut rest = serde_json::to_value(wall).unwrap_or_default();
                if let Some(o) = rest.as_object_mut() {
                    o.remove("pts");
                    o.remove("bulges");
                }
                self.text(&rest.to_string());
            }
            // A grid, a column, a stair or a ramp (docs/adr/0343 §1): its fields as one JSON text.
            Entity::Grid(e) => self.text(&serde_json::to_value(&e.grid).unwrap_or_default().to_string()),
            Entity::Column(e) => self.text(&serde_json::to_value(&e.column).unwrap_or_default().to_string()),
            Entity::Stair(e) => self.text(&serde_json::to_value(&e.stair).unwrap_or_default().to_string()),
            Entity::Ramp(e) => self.text(&serde_json::to_value(&e.ramp).unwrap_or_default().to_string()),
            // A tag (docs/adr/0344 §1): its fields as one JSON text.
            Entity::Tag(e) => self.text(&serde_json::to_value(&e.tag).unwrap_or_default().to_string()),
            // A view mark (docs/adr/0345 §1): its fields as one JSON text.
            Entity::ViewMark(e) => self.text(
                &serde_json::to_value(&e.mark)
                    .unwrap_or_default()
                    .to_string(),
            ),
            Entity::Slab(e) => self.text(
                &serde_json::to_value(&e.slab)
                    .unwrap_or_default()
                    .to_string(),
            ),
            // A beam (docs/adr/8041 §1): its fields as one JSON text.
            Entity::Beam(e) => self.text(
                &serde_json::to_value(&e.beam)
                    .unwrap_or_default()
                    .to_string(),
            ),
            // A roof (docs/adr/8044 §1): its fields as one JSON text.
            Entity::Roof(e) => self.text(
                &serde_json::to_value(&e.roof)
                    .unwrap_or_default()
                    .to_string(),
            ),
            // A railing (docs/adr/8045 §3): its fields as one JSON text.
            Entity::Railing(e) => self.text(
                &serde_json::to_value(&e.railing)
                    .unwrap_or_default()
                    .to_string(),
            ),
            // A space (docs/adr/0342 §1): its source as one JSON text.
            Entity::Space(kentos_contracts::SpaceEntity { base: _, space }) => {
                self.text(&serde_json::to_value(space).unwrap_or_default().to_string());
            }
            // An opening (docs/adr/0341 §1): its jambs, then the rest of its source as one JSON text.
            Entity::Opening(kentos_contracts::OpeningEntity { base: _, opening }) => {
                self.points(&opening.place);
                let mut rest = serde_json::to_value(opening).unwrap_or_default();
                if let Some(o) = rest.as_object_mut() {
                    o.remove("place");
                }
                self.text(&rest.to_string());
            }
            Entity::PointCloud(kentos_contracts::PointCloudEntity {
                base: _,
                cloud:
                    kentos_contracts::PointCloudFields {
                        sources,
                        bounds,
                        count: points,
                        srid,
                        style,
                        opacity,
                    },
            }) => {
                let render = kentos_contracts::CloudRender::ALL
                    .iter()
                    .position(|r| *r == style.render)
                    .unwrap_or(0);
                let mut look = 0;
                for (bit, on) in [
                    (CLOUD_MIN, style.min.is_some()),
                    (CLOUD_MAX, style.max.is_some()),
                    (CLOUD_RAMP, style.ramp.is_some()),
                    (CLOUD_INVERT, style.invert),
                    (CLOUD_RGB8, style.rgb8),
                    (
                        CLOUD_METRES,
                        style.size_unit == kentos_contracts::PointSizeUnit::M,
                    ),
                    (
                        CLOUD_SQUARE,
                        style.shape == kentos_contracts::PointShape::Square,
                    ),
                ] {
                    if on {
                        look |= bit;
                    }
                }
                self.out.ints.extend([
                    *srid,
                    count(sources.len()),
                    count(render),
                    look,
                    count(style.hidden.len()),
                ]);
                self.out
                    .ints
                    .extend(style.hidden.iter().map(|&c| u32::from(c)));
                for s in sources {
                    let format = kentos_contracts::CloudFormat::ALL
                        .iter()
                        .position(|f| *f == s.format)
                        .unwrap_or(0);
                    let source = if s.asset.is_some() {
                        SOURCE_ASSET
                    } else if s.file.is_some() {
                        SOURCE_FILE
                    } else {
                        SOURCE_URL
                    };
                    let placed = if s.pose.is_some() { SOURCE_POSE } else { 0 };
                    self.out.ints.extend([count(format), source | placed]);
                }
                self.out.floats.extend(bounds);
                self.out.floats.extend([*points as f64, style.size]);
                if let Some(o) = opacity {
                    flags |= OPT[0];
                    self.float(*o);
                }
                for v in [style.min, style.max].into_iter().flatten() {
                    self.float(v);
                }
                for s in sources {
                    self.float(s.count as f64);
                    self.out.floats.extend(s.bounds);
                    if let Some(m) = &s.pose {
                        self.out.floats.extend(m);
                    }
                }
                if let Some(r) = &style.ramp {
                    self.text(r);
                }
                for s in sources {
                    if let Some(t) = s.asset.as_ref().or(s.file.as_ref()).or(s.url.as_ref()) {
                        self.text(t);
                    }
                }
            }
            Entity::Leader(LeaderEntity {
                tip,
                leader_style,
                arms,
                blocks,
                annotative,
                base: _,
                pts,
                text,
                height,
                rotation,
                arrow,
                arrow_size,
                mask,
            }) => {
                self.points(pts);
                self.out.floats.extend([*height, *rotation]);
                if let Some(t) = text {
                    flags |= OPT[0];
                    self.text(t);
                }
                if let Some(a) = arrow {
                    flags |= OPT[1];
                    let at = LeaderArrow::ALL.iter().position(|x| x == a).unwrap_or(0);
                    self.int(count(at));
                }
                if *mask {
                    flags |= OPT[2];
                }
                if let Some(s) = arrow_size {
                    flags |= OPT[3];
                    self.float(*s);
                }
                // Its scales (docs/adr/0323), as their JSON.
                if let Some(a) = annotative {
                    flags |= OPT[4];
                    self.text(&serde_json::to_string(a).unwrap_or_default());
                }
                // Its arms, tip, blocks and style (docs/adr/0326), as one JSON object of those it has.
                if !arms.is_empty() || tip.is_some() || !blocks.is_empty() || leader_style.is_some() {
                    flags |= OPT[5];
                    self.text(&multileader_json(arms, tip.as_ref(), blocks, leader_style.as_deref()));
                }
            }
            Entity::Table(TableEntity {
                styles,
                formulas,
                fields,
                base: _,
                p,
                rotation,
                height,
                rows,
                columns,
                cells,
                merges,
                aligns,
                header,
                grid,
                frame,
                face,
                source,
                breaks,
                table_style,
            }) => {
                self.point(p);
                self.out.floats.extend([*rotation, *height]);
                self.numbers(rows);
                self.numbers(columns);
                // Every cell row with its length: a ragged one reaches the encoder, which refuses it.
                self.int(count(cells.len()));
                for row in cells {
                    self.int(count(row.len()));
                    for words in row {
                        self.text(words);
                    }
                }
                if !merges.is_empty() {
                    flags |= OPT[0];
                    self.int(count(merges.len()));
                    for m in merges {
                        self.out.ints.extend([m.row, m.col, m.rows, m.cols]);
                    }
                }
                if let Some(list) = aligns {
                    flags |= OPT[1];
                    self.int(count(list.len()));
                    for a in list {
                        self.int(count(
                            TableAlign::ALL.iter().position(|x| x == a).unwrap_or(0),
                        ));
                    }
                }
                if *header {
                    flags |= OPT[2];
                }
                if let Some(g) = grid {
                    flags |= OPT[3];
                    self.int(count(
                        TableGrid::ALL.iter().position(|x| x == g).unwrap_or(0),
                    ));
                }
                if let Some(id) = &face.text_style {
                    flags |= OPT[4];
                    self.text(id);
                }
                if let Some(f) = face.font {
                    flags |= OPT[5];
                    self.int(font_place(f));
                }
                if face.bold {
                    flags |= OPT[6];
                }
                if face.italic {
                    flags |= OPT[7];
                }
                if let Some(o) = face.oblique {
                    flags |= OPT[8];
                    self.float(o);
                }
                // The frame's width (docs/adr/0184 §1) after the slant; its source's ints and texts are its own streams'.
                if let Some(f) = frame {
                    flags |= OPT[10];
                    self.float(*f);
                }
                if let Some(source) = source.as_ref().filter(|s| {
                    !matches!(
                        s,
                        TableSource::Blocks { .. }
                            | TableSource::Schedule { .. }
                            | TableSource::Extraction(_)
                            | TableSource::Zoning {}
                            | TableSource::Parking {}
                            | TableSource::Shelter {}
                    )
                }) {
                    flags |= OPT[9];
                    match source {
                        TableSource::File { name, sheet, .. } => {
                            self.int(3);
                            self.int(u32::from(sheet.is_some()));
                            self.text(name);
                            if let Some(s) = sheet {
                                self.text(s);
                            }
                        }
                        _ => {
                            let kind = match source {
                                TableSource::Coordinates { .. } => 0,
                                TableSource::Areas { .. } => 1,
                                _ => 2,
                            };
                            self.int(kind);
                            self.int(count(source.objects().len()));
                            for id in source.objects() {
                                self.text(&id.to_text());
                            }
                        }
                    }
                }
                // Schema 68's styles, formulas, fields, linked file and block schedule (docs/adr/0327): one JSON object.
                if let Some(j) = table_cells_json(styles, formulas, fields, source.as_ref(), breaks.as_ref(), table_style.as_deref()) {
                    flags |= OPT[11];
                    self.text(&j);
                }
            }
        }
        flags
    }
}

/// Objects as columns, taking them: each is dropped once written, so a large
/// drawing is in memory about once, not twice. The layer table lists the
/// layer ids in the order the objects first use them.
pub fn pack(entities: Vec<Entity>, uids: Vec<EntityId>) -> Columns {
    let mut table: HashMap<String, u32> = HashMap::new();
    let mut order: Vec<&str> = Vec::new();
    for e in &entities {
        let id = e.base().layer_id.as_str();
        if !table.contains_key(id) {
            table.insert(id.to_owned(), count(order.len()));
            order.push(id);
        }
    }
    let mut packer = Packer {
        out: Columns::default(),
    };
    packer.int(count(order.len()));
    for id in &order {
        packer.text(id);
    }
    packer.out.kinds.reserve(entities.len());
    packer.out.uids.reserve(entities.len() * 16);
    for (entity, uid) in entities.into_iter().zip(uids) {
        let layer = table.get(&entity.base().layer_id).copied().unwrap_or(0);
        packer.object(&entity, &uid, layer);
    }
    packer.out
}
