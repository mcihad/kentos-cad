//! The columns read back into objects (`Cursor`, `unpack`): each kind's geometry, and what a kind may carry.

use super::*;

impl<'c> Cursor<'c> {
    pub(super) fn new(cols: &'c Columns) -> Self {
        Self {
            cols,
            int: 0,
            float: 0,
            unit: 0,
            text: 0,
        }
    }

    pub(super) fn int(&mut self) -> Result<u32, KcadError> {
        let v = *self
            .cols
            .ints
            .get(self.int)
            .ok_or_else(|| broken("tam sayılar erken bitti"))?;
        self.int += 1;
        Ok(v)
    }

    pub(super) fn usize(&mut self) -> Result<usize, KcadError> {
        Ok(self.int()? as usize)
    }

    pub(super) fn float(&mut self) -> Result<f64, KcadError> {
        let v = *self
            .cols
            .floats
            .get(self.float)
            .ok_or_else(|| broken("sayılar erken bitti"))?;
        self.float += 1;
        Ok(v)
    }

    pub(super) fn point(&mut self) -> Result<Vec2, KcadError> {
        Ok(Vec2 {
            x: self.float()?,
            y: self.float()?,
        })
    }

    pub(super) fn points(&mut self) -> Result<Vec<Vec2>, KcadError> {
        let n = self.usize()?;
        if n.saturating_mul(2) > self.cols.floats.len() - self.float {
            return Err(broken("nokta listesi sayılardan uzun"));
        }
        (0..n).map(|_| self.point()).collect()
    }

    pub(super) fn numbers(&mut self) -> Result<Vec<f64>, KcadError> {
        let n = self.usize()?;
        let end = self
            .float
            .checked_add(n)
            .filter(|&end| end <= self.cols.floats.len())
            .ok_or_else(|| broken("sayı listesi sayılardan uzun"))?;
        let out = self.cols.floats[self.float..end].to_vec();
        self.float = end;
        Ok(out)
    }

    /// A polygon's or a part's holes (their count, then each hole's flags and lists).
    pub(super) fn holes(&mut self) -> Result<Vec<RingGeometry>, KcadError> {
        let h = self.usize()?;
        if h > self.cols.ints.len() {
            return Err(broken("delik sayısı tam sayılardan fazla"));
        }
        let mut rings = Vec::with_capacity(h);
        for _ in 0..h {
            let hole = self.int()?;
            if hole & !(HOLE_BULGES | HOLE_ELEVATIONS) != 0 {
                return Err(broken(&format!("deliğin bayrakları {hole:#x}")));
            }
            rings.push(RingGeometry {
                pts: self.points()?,
                bulges: if hole & HOLE_BULGES != 0 {
                    Some(self.numbers()?)
                } else {
                    None
                },
                zs: if hole & HOLE_ELEVATIONS != 0 {
                    Some(self.elevations()?)
                } else {
                    None
                },
            });
        }
        Ok(rings)
    }

    /// A list of elevations (its length, then the numbers): NaN is a vertex without one.
    pub(super) fn elevations(&mut self) -> Result<Vec<Option<f64>>, KcadError> {
        let n = self.usize()?;
        let end = self
            .float
            .checked_add(n)
            .filter(|&end| end <= self.cols.floats.len())
            .ok_or_else(|| broken("kot listesi sayılardan uzun"))?;
        let out = self.cols.floats[self.float..end]
            .iter()
            .map(|&z| (!z.is_nan()).then_some(z))
            .collect();
        self.float = end;
        Ok(out)
    }

    /// The next text; `where_` names it when it is not valid Unicode (a lone surrogate).
    pub(super) fn text(&mut self, where_: impl FnOnce() -> String) -> Result<String, KcadError> {
        let units = *self
            .cols
            .text_lengths
            .get(self.text)
            .ok_or_else(|| broken("metinler erken bitti"))? as usize;
        let end = self
            .unit
            .checked_add(units)
            .filter(|&end| end <= self.cols.text.len())
            .ok_or_else(|| broken("metin uzunlukları metinden uzun"))?;
        let s = String::from_utf16(&self.cols.text[self.unit..end]).map_err(|_| {
            KcadError::unwritable(
                Code::InvalidUtf8,
                &where_(),
                "metinde eşi olmayan bir UTF-16 vekili var; KCAD 2 yalnız geçerli Unicode metin yazar",
            )
        })?;
        self.unit = end;
        self.text += 1;
        Ok(s)
    }

    /// The layer table at the head of the columns.
    pub(super) fn table(&mut self) -> Result<Vec<String>, KcadError> {
        let n = self.usize()?;
        if n > self.cols.text_lengths.len() {
            return Err(broken("katman tablosu metinlerden uzun"));
        }
        (0..n)
            .map(|i| self.text(|| format!("katman {}", i + 1)))
            .collect()
    }

    pub(super) fn at_end(&self) -> bool {
        self.int == self.cols.ints.len()
            && self.float == self.cols.floats.len()
            && self.unit == self.cols.text.len()
            && self.text == self.cols.text_lengths.len()
    }
}

/// The objects the columns hold, as the contract has them, with their
/// persistent ids; the slots are 1, 2, 3 … as a file reader gives them.
/// Columns that do not hold together are refused (`Code::BadColumns`); a
/// text that is not valid Unicode is refused with its place.
pub fn unpack(cols: &Columns) -> Result<(Vec<Entity>, Vec<EntityId>), KcadError> {
    let n = cols.kinds.len();
    if cols.uids.len() != n * 16 {
        return Err(broken("kimlikler nesne sayısıyla uyuşmuyor"));
    }
    let mut c = Cursor::new(cols);
    let table = c.table()?;
    let mut entities = Vec::with_capacity(n);
    let mut uids = Vec::with_capacity(n);
    for (i, &k) in cols.kinds.iter().enumerate() {
        entities.push(object(&mut c, &table, i, k)?);
        let mut id = [0u8; 16];
        id.copy_from_slice(&cols.uids[i * 16..i * 16 + 16]);
        uids.push(EntityId(id));
    }
    if !c.at_end() {
        return Err(broken("nesnelerden sonra fazladan değer var"));
    }
    Ok((entities, uids))
}

/// The `i`th object, of kind `k`. Out of line: see `Encoder::object`.
#[inline(never)]
pub(super) fn object(
    c: &mut Cursor<'_>,
    table: &[String],
    i: usize,
    k: u8,
) -> Result<Entity, KcadError> {
    let kind = *KINDS
        .get(usize::from(k))
        .ok_or_else(|| broken(&format!("{}. nesnenin türü {k}", i + 1)))?;
    let place = |field: &str| format!("entities/{i} ({kind}) › {field}");
    let layer = table
        .get(c.usize()?)
        .ok_or_else(|| broken(&format!("{}. nesnenin katmanı tabloda yok", i + 1)))?
        .clone();
    let flags = c.int()?;
    let attr_count = c.usize()?;
    if attr_count > c.cols.text_lengths.len() {
        return Err(broken("öznitelik sayısı metinlerden fazla"));
    }
    let line_weight = (flags & WEIGHT != 0).then(|| c.float()).transpose()?;
    let color = (flags & COLOR != 0)
        .then(|| c.text(|| place("color")))
        .transpose()?;
    let label = (flags & LABEL != 0)
        .then(|| c.text(|| place("label")))
        .transpose()?;
    let symbol = (flags & SYMBOL != 0)
        .then(|| c.text(|| place("symbol")))
        .transpose()?;
    let mut label_pins = Vec::new();
    if flags & PINS != 0 {
        let n = c.int()?;
        for _ in 0..n {
            let bits = c.int()?;
            if bits & !(PIN_CLASS | PIN_AT | PIN_ROTATION | PIN_HIDDEN) != 0 {
                return Err(broken(&format!(
                    "{}. nesnenin etiket iğnesinin bayrakları {bits:#x}",
                    i + 1
                )));
            }
            let class = (bits & PIN_CLASS != 0)
                .then(|| c.text(|| place("labelPins/class")))
                .transpose()?;
            let at = match bits & PIN_AT != 0 {
                true => Some(kentos_contracts::Vec2 {
                    x: c.float()?,
                    y: c.float()?,
                }),
                false => None,
            };
            let rotation = (bits & PIN_ROTATION != 0).then(|| c.float()).transpose()?;
            label_pins.push(kentos_contracts::LabelPin {
                class,
                at,
                rotation,
                hidden: (bits & PIN_HIDDEN != 0).then_some(true),
            });
        }
    }
    let mut attachments = Vec::new();
    if flags & ATTACHMENTS != 0 {
        let n = c.int()?;
        for _ in 0..n {
            let bits = c.int()?;
            if bits & !(ATT_ASSET | ATT_FILE | ATT_URL | ATT_NOTE) != 0 {
                return Err(broken(&format!(
                    "{}. nesnenin ekinin bayrakları {bits:#x}",
                    i + 1
                )));
            }
            let name = c.text(|| place("attachments/name"))?;
            let mut field = |bit: u32, key: &str| {
                (bits & bit != 0)
                    .then(|| c.text(|| place(&format!("attachments/{key}"))))
                    .transpose()
            };
            let asset = field(ATT_ASSET, "asset")?;
            let file = field(ATT_FILE, "file")?;
            let url = field(ATT_URL, "url")?;
            let note = field(ATT_NOTE, "note")?;
            attachments.push(kentos_contracts::Attachment {
                name,
                asset,
                file,
                url,
                note,
            });
        }
    }
    let mut look = kentos_contracts::ObjectLook::default();
    if flags & LOOK != 0 {
        let bits = c.int()?;
        let known = LOOK_TYPE
            | LOOK_SCALE
            | LOOK_TRANSPARENCY
            | LOOK_ORDER
            | LOOK_BY_LAYER
            | LOOK_WIPEOUT
            | LOOK_FRAME
            | LOOK_PATTERN;
        if bits & !known != 0 || (bits & LOOK_FRAME != 0 && bits & LOOK_WIPEOUT == 0) {
            return Err(broken(&format!(
                "{}. nesnenin görünüşünün bayrakları {bits:#x}",
                i + 1
            )));
        }
        if bits & LOOK_TYPE != 0 {
            let at = c.usize()?;
            look.line_type = Some(
                *LineType::ALL
                    .get(at)
                    .ok_or_else(|| broken(&format!("{}. nesnenin çizgi tipi {at}", i + 1)))?,
            );
        }
        look.wipeout = (bits & LOOK_WIPEOUT != 0).then_some(kentos_contracts::Wipeout {
            frame: bits & LOOK_FRAME != 0,
        });
        look.line_type_scale = (bits & LOOK_SCALE != 0).then(|| c.float()).transpose()?;
        look.transparency = (bits & LOOK_TRANSPARENCY != 0)
            .then(|| c.float())
            .transpose()?;
        if bits & LOOK_ORDER != 0 {
            let o = c.float()?;
            if o.fract() != 0.0 || !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&o) {
                return Err(broken(&format!("{}. nesnenin çizim sırası {o}", i + 1)));
            }
            look.order = Some(o as i32);
        }
        if bits & LOOK_BY_LAYER != 0 {
            let mask = c.int()?;
            if mask >> LookPart::ALL.len() != 0 {
                return Err(broken(&format!(
                    "{}. nesnenin katmana göre parçaları {mask:#x}",
                    i + 1
                )));
            }
            look.by_layer = LookPart::ALL
                .iter()
                .enumerate()
                .filter(|(k, _)| mask & 1 << k != 0)
                .map(|(_, p)| *p)
                .collect();
        }
        if bits & LOOK_PATTERN != 0 {
            look.line_pattern = Some(c.text(|| place("linePattern"))?);
        }
    }
    let mut attrs = BTreeMap::new();
    for _ in 0..attr_count {
        let key = c.text(|| place("attrs"))?;
        let value = c.text(|| place(&format!("attrs/{key}")))?;
        attrs.insert(key, value);
    }
    let base = EntityBase {
        look,
        id: count(i + 1),
        layer_id: layer,
        color,
        attrs,
        label,
        symbol,
        line_weight,
        label_pins,
        attachments,
    };
    let known = COLOR | LABEL | SYMBOL | WEIGHT | PINS | ATTACHMENTS | LOOK | allowed(k);
    if flags & !known != 0 {
        return Err(broken(&format!(
            "{}. nesnenin bayrakları {flags:#x}",
            i + 1
        )));
    }
    geometry(c, k, base, flags, &place)
}

/// The optional-field flags a kind may have.
pub(super) fn allowed(kind: u8) -> u32 {
    match kind {
        0 => OPT[0] | OPT[1],
        // A line's measures are schema 39's (docs/adr/0224).
        1 => OPT[0] | OPT[1] | OPT[2] | OPT[3],
        // A polyline's parts are schema 17's (docs/adr/0174), its measures schema 39's (docs/adr/0224;
        // a polygon's are carried for the encoder to refuse), its closed and widths schema 46's
        // (docs/adr/0258; a polygon's alike).
        2 => OPT[..8].iter().fold(0, |m, b| m | b),
        3 => OPT[..8].iter().fold(0, |m, b| m | b),
        // A text's alignment, width factor and mask, schema 18's link (docs/adr/0175 §4), schema 20's box,
        // spacing and runs, schema 21's face (docs/adr/0183), schema 25's curve and its bulges
        // (docs/adr/0196); a leader's three and schema 30's arrowhead size (docs/adr/0205 §7).
        // Schema 63's overrides (docs/adr/0322 §5), schema 64's scales (docs/adr/0323), schema 65's layout
        // (docs/adr/0324), schema 66's fields (docs/adr/0325).
        10 => OPT[..18].iter().fold(0, |m, b| m | b),
        // Schema 67's arms, tip, blocks and style (docs/adr/0326).
        14 => OPT[0] | OPT[1] | OPT[2] | OPT[3] | OPT[4] | OPT[5],
        // A dimension: text, style, angle, c, schema 9's mask, za, zb (docs/adr/0147), schema 21's look,
        // schema 30's lines (docs/adr/0205 §6).
        11 => OPT.iter().fold(0, |m, b| m | b) | LINES | REFS | MORE,
        // A hatch: holes, schema 23's name, scale, families, gradient and tie (docs/adr/0186), schema 69's boundary
        // and bounds (docs/adr/0328).
        12 => OPT[..7].iter().fold(0, |m, b| m | b),
        // An insert: mirror, its own y factor (docs/adr/0315 §3), its moved attributes (docs/adr/0317 §1), its
        // parameter values (docs/adr/0318 §3), its clipping (docs/adr/0351 §1), its level and phase (docs/adr/0733 §1).
        13 => OPT[0] | OPT[1] | OPT[2] | OPT[3] | OPT[4] | OPT[5],
        // A table: merges, aligns, header, grid, its face, source, frame (docs/adr/0184); schema 68's cells
        // (docs/adr/0327).
        15 => OPT[..12].iter().fold(0, |m, b| m | b),
        // A picture: mirror, asset, file, clip, opacity (docs/adr/0192).
        16 => OPT[..5].iter().fold(0, |m, b| m | b),
        // A raster: asset, file, opacity (docs/adr/0204), url (docs/adr/0207), dataset (docs/adr/0243), georeference
        // record (docs/adr/0518), tile table (docs/adr/0627).
        17 => OPT[..7].iter().fold(0, |m, b| m | b),
        // A point cloud: opacity (docs/adr/0207).
        18 => OPT[0],
        _ => 0,
    }
}

pub(super) fn geometry(
    c: &mut Cursor<'_>,
    kind: u8,
    base: EntityBase,
    flags: u32,
    place: &dyn Fn(&str) -> String,
) -> Result<Entity, KcadError> {
    let has = |bit: usize| flags & OPT[bit] != 0;
    Ok(match kind {
        0 => {
            let p = c.point()?;
            let z = if has(0) { Some(c.float()?) } else { None };
            let parts = if has(1) {
                let q = c.usize()?;
                if q > c.cols.ints.len() {
                    return Err(broken("nokta sayısı tam sayılardan fazla"));
                }
                let mut parts = Vec::with_capacity(q);
                for _ in 0..q {
                    let flag = c.int()?;
                    if flag > 1 {
                        return Err(broken(&format!("noktanın bayrağı {flag:#x}")));
                    }
                    let p = c.point()?;
                    let z = if flag == 1 { Some(c.float()?) } else { None };
                    parts.push(PointPart { p, z });
                }
                Some(parts)
            } else {
                None
            };
            Entity::Point(PointEntity { base, p, z, parts })
        }
        1 => Entity::Line(LineEntity {
            base,
            a: c.point()?,
            b: c.point()?,
            za: if has(0) { Some(c.float()?) } else { None },
            zb: if has(1) { Some(c.float()?) } else { None },
            ma: if has(2) { Some(c.float()?) } else { None },
            mb: if has(3) { Some(c.float()?) } else { None },
        }),
        2 | 3 => {
            let pts = c.points()?;
            let bulges = if has(0) { Some(c.numbers()?) } else { None };
            let zs = if has(2) { Some(c.elevations()?) } else { None };
            let ms = if has(4) { Some(c.elevations()?) } else { None };
            let holes = if has(1) { Some(c.holes()?) } else { None };
            let parts = if has(3) {
                let q = c.usize()?;
                if q > c.cols.ints.len() {
                    return Err(broken("parça sayısı tam sayılardan fazla"));
                }
                let mut parts = Vec::with_capacity(q);
                for _ in 0..q {
                    let part = c.int()?;
                    if part & !(PART_BULGES | PART_ELEVATIONS | PART_HOLES | PART_MEASURES) != 0 {
                        return Err(broken(&format!("parçanın bayrakları {part:#x}")));
                    }
                    let pts = c.points()?;
                    let bulges = if part & PART_BULGES != 0 {
                        Some(c.numbers()?)
                    } else {
                        None
                    };
                    let zs = if part & PART_ELEVATIONS != 0 {
                        Some(c.elevations()?)
                    } else {
                        None
                    };
                    let ms = if part & PART_MEASURES != 0 {
                        Some(c.elevations()?)
                    } else {
                        None
                    };
                    let holes = if part & PART_HOLES != 0 {
                        Some(c.holes()?)
                    } else {
                        None
                    };
                    parts.push(AreaPart {
                        pts,
                        bulges,
                        holes,
                        zs,
                        ms,
                    });
                }
                Some(parts)
            } else {
                None
            };
            let closed = has(5);
            let widths = if has(6) {
                let flat = c.numbers()?;
                if flat.len() % 2 != 0 {
                    return Err(broken("genişlik listesinin sayısı tek"));
                }
                Some(flat.chunks_exact(2).map(|w| [w[0], w[1]]).collect())
            } else {
                None
            };
            let path = PathEntity {
                space_separator: if has(7) {
                    Some(serde_json::from_str(&c.text(|| place("spaceSeparator"))?)
                        .map_err(|_|broken("mahal ayırıcı rolü okunamadı"))?)
                } else {None},
                base,
                pts,
                bulges,
                holes,
                zs,
                ms,
                parts,
                closed,
                widths,
            };
            if kind == 3 {
                Entity::Polygon(path)
            } else {
                Entity::Polyline(path)
            }
        }
        4 => Entity::Circle(CircleEntity {
            base,
            c: c.point()?,
            r: c.float()?,
        }),
        5 => Entity::Arc(ArcEntity {
            base,
            c: c.point()?,
            r: c.float()?,
            a0: c.float()?,
            a1: c.float()?,
        }),
        6 => Entity::Ellipse(EllipseEntity {
            base,
            c: c.point()?,
            major: c.point()?,
            ratio: c.float()?,
            t0: c.float()?,
            t1: c.float()?,
        }),
        7 => {
            let pts = c.points()?;
            let closed = match c.int()? {
                0 => false,
                1 => true,
                v => return Err(broken(&format!("eğrinin kapalılığı {v}"))),
            };
            Entity::Spline(SplineEntity { base, pts, closed })
        }
        8 | 9 => {
            let line = ConstructionEntity {
                base,
                p: c.point()?,
                dir: c.point()?,
            };
            if kind == 8 {
                Entity::Xline(line)
            } else {
                Entity::Ray(line)
            }
        }
        10 => {
            let (p, height, rotation) = (c.point()?, c.float()?, c.float()?);
            let text = c.text(|| place("text"))?;
            let align = if has(0) {
                let v = c.usize()?;
                Some(
                    *TextAlign::ALL
                        .get(v)
                        .ok_or_else(|| broken(&format!("yazı hizası {v}")))?,
                )
            } else {
                None
            };
            let width_factor = if has(1) { Some(c.float()?) } else { None };
            let (label_of, label_scale) = if has(3) {
                let of = c.text(|| place("labelOf"))?;
                let id = EntityId::parse(&of)
                    .ok_or_else(|| broken(&format!("bağlı yazının nesnesi “{of}”")))?;
                (Some(id), Some(c.float()?))
            } else {
                (None, None)
            };
            let box_width = if has(4) { Some(c.float()?) } else { None };
            let line_spacing = if has(5) { Some(c.float()?) } else { None };
            let runs = if has(6) {
                let n = c.usize()?;
                let mut runs = Vec::with_capacity(n.min(1024));
                for _ in 0..n {
                    let (start, end, bits) = (c.int()?, c.int()?, c.int()?);
                    let color = if bits & RUN_COLOR != 0 {
                        Some(c.text(|| place("runs"))?)
                    } else {
                        None
                    };
                    runs.push(TextRun {
                        start,
                        end,
                        bold: bits & RUN_BOLD != 0,
                        italic: bits & RUN_ITALIC != 0,
                        underline: bits & RUN_UNDERLINE != 0,
                        script: match bits & RUN_SCRIPT {
                            RUN_SUPER => Some(TextScript::Super),
                            RUN_SUB => Some(TextScript::Sub),
                            RUN_SCRIPT => Some(TextScript::Stack),
                            _ => None,
                        },
                        color,
                    });
                }
                runs
            } else {
                Vec::new()
            };
            let text_style = if has(7) {
                Some(c.text(|| place("textStyle"))?)
            } else {
                None
            };
            let font = if has(8) {
                Some(font_at(c.usize()?)?)
            } else {
                None
            };
            let oblique = if has(11) { Some(c.float()?) } else { None };
            let path = if has(12) {
                let pts = c.points()?;
                let bulges = if has(13) { Some(c.numbers()?) } else { None };
                Some(kentos_contracts::TextPath { pts, bulges })
            } else {
                None
            };
            let overrides: Vec<String> = if has(14) {
                let t = c.text(|| place("overrides"))?;
                serde_json::from_str(&t).map_err(|_| broken("yazının geçersiz kılınanları"))?
            } else {
                Vec::new()
            };
            let annotative: Option<Box<kentos_contracts::Annotative>> = if has(15) {
                let t = c.text(|| place("annotative"))?;
                Some(serde_json::from_str(&t).map_err(|_| broken("yazının ölçekleri"))?)
            } else {
                None
            };
            let layout: Option<Box<kentos_contracts::ParagraphLayout>> = if has(16) {
                let t = c.text(|| place("layout"))?;
                Some(serde_json::from_str(&t).map_err(|_| broken("yazının düzeni"))?)
            } else {
                None
            };
            let fields: Vec<kentos_contracts::TextField> = if has(17) {
                let t = c.text(|| place("fields"))?;
                serde_json::from_str(&t).map_err(|_| broken("yazının alanları"))?
            } else {
                Vec::new()
            };
            Entity::Text(TextEntity {
                annotative,
                base,
                p,
                text,
                height,
                rotation,
                align,
                width_factor,
                mask: has(2),
                label_of,
                label_scale,
                paragraph: Paragraph {
                    fields,
                    layout,
                    box_width,
                    line_spacing,
                    runs,
                },
                face: TextFace {
                    overrides,
                    text_style,
                    font,
                    bold: has(9),
                    italic: has(10),
                    oblique,
                },
                path,
            })
        }
        11 => {
            let (a, b) = (c.point()?, c.point()?);
            let (offset, height) = (c.float()?, c.float()?);
            let text = if has(0) {
                Some(c.text(|| place("text"))?)
            } else {
                None
            };
            let style = if has(1) {
                let v = c.usize()?;
                Some(
                    *DIMENSION_STYLES
                        .get(v)
                        .ok_or_else(|| broken(&format!("ölçü türü {v}")))?,
                )
            } else {
                None
            };
            let angle = if has(2) { Some(c.float()?) } else { None };
            let corner = if has(3) { Some(c.point()?) } else { None };
            let za = if has(5) { Some(c.float()?) } else { None };
            let zb = if has(6) { Some(c.float()?) } else { None };
            let dim_style = if has(7) {
                Some(c.text(|| place("dimStyle"))?)
            } else {
                None
            };
            let arrow = if has(8) {
                let v = c.usize()?;
                Some(
                    *DimensionArrow::ALL
                        .get(v)
                        .ok_or_else(|| broken(&format!("ölçü oku {v}")))?,
                )
            } else {
                None
            };
            let mut size = |k: usize| {
                if has(k) {
                    c.float().map(Some)
                } else {
                    Ok(None)
                }
            };
            let (arrow_size, ext_offset, ext_beyond, text_gap) =
                (size(9)?, size(10)?, size(11)?, size(12)?);
            let decimals = if has(14) { Some(c.int()?) } else { None };
            let unit = if has(15) {
                let v = c.usize()?;
                Some(
                    *UNITS
                        .get(v)
                        .ok_or_else(|| broken(&format!("ölçü birimi {v}")))?,
                )
            } else {
                None
            };
            let prefix = if has(16) {
                Some(c.text(|| place("prefix"))?)
            } else {
                None
            };
            let suffix = if has(17) {
                Some(c.text(|| place("suffix"))?)
            } else {
                None
            };
            let font = if has(18) {
                Some(font_at(c.usize()?)?)
            } else {
                None
            };
            // Its lines (docs/adr/0205 §6): the line flags, then the types, weights and colours.
            let line = if flags & LINES != 0 { c.int()? } else { 0 };
            let on = |bit: u32| line & bit != 0;
            let mut line_type = |bit: u32| -> Result<Option<LineType>, KcadError> {
                if !on(bit) {
                    return Ok(None);
                }
                let v = c.usize()?;
                LINE_TYPES
                    .get(v)
                    .copied()
                    .map(Some)
                    .ok_or_else(|| broken(&format!("ölçünün çizgi tipi {v}")))
            };
            let (dim_line_type, ext_line_type) = (line_type(LINE_TYPE)?, line_type(EXT_LINE_TYPE)?);
            let dim_line_weight = if on(LINE_WEIGHT) {
                Some(c.float()?)
            } else {
                None
            };
            let ext_weight = if on(EXT_WEIGHT) {
                Some(c.float()?)
            } else {
                None
            };
            let dim_line_color = if on(LINE_COLOR) {
                Some(c.text(|| place("dimLineColor"))?)
            } else {
                None
            };
            let ext_color = if on(EXT_COLOR) {
                Some(c.text(|| place("extColor"))?)
            } else {
                None
            };
            let text_color = if on(TEXT_COLOR) {
                Some(c.text(|| place("textColor"))?)
            } else {
                None
            };
            // What its points follow (docs/adr/0249 §4).
            let refs = if flags & REFS != 0 {
                let mask = c.int()?;
                if mask & !63 != 0 {
                    return Err(broken(&format!("ölçünün başvuru maskesi {mask}")));
                }
                let rule = match mask & 56 {
                    0 => None,
                    8 => Some(kentos_contracts::DimensionRule::Curve),
                    16 => Some(kentos_contracts::DimensionRule::Edges),
                    24 => Some(kentos_contracts::DimensionRule::Pair),
                    32 => Some(kentos_contracts::DimensionRule::Cross),
                    _ => return Err(broken(&format!("ölçünün başvuru maskesi {mask}"))),
                };
                let mut slots = [None, None, None];
                for (i, slot) in slots.iter_mut().enumerate() {
                    if mask & 1 << i == 0 {
                        continue;
                    }
                    let written = c.usize()?;
                    let in_block = written & IN_BLOCK as usize != 0;
                    let kind = written & !(IN_BLOCK as usize);
                    let (_, n) = *PLACE_KINDS
                        .get(kind)
                        .ok_or_else(|| broken(&format!("başvurunun yer türü {kind}")))?;
                    let mut numbers = [0; 3];
                    for x in &mut numbers[..n] {
                        *x = c.int()?;
                    }
                    let [part, ring, index] = numbers;
                    let spot = match kind {
                        0 => SubPlace::Node { part },
                        1 => SubPlace::Start,
                        2 => SubPlace::End,
                        3 => SubPlace::Center,
                        4 => SubPlace::Vertex { part, ring, index },
                        5 => SubPlace::Mid { part, ring, index },
                        _ => SubPlace::Along {
                            part,
                            ring,
                            index,
                            t: c.float()?,
                        },
                    };
                    let mut path = Vec::new();
                    if in_block {
                        let n = c.usize()?;
                        if n > kentos_contracts::SUBREF_PATH_DEPTH {
                            return Err(broken(&format!("başvurunun blok yolu {n} düzey")));
                        }
                        for _ in 0..n {
                            path.push(c.int()?);
                        }
                    }
                    let is_broken = match c.int()? {
                        0 => false,
                        1 => true,
                        v => return Err(broken(&format!("başvurunun kopma bayrağı {v}"))),
                    };
                    let t = c.text(|| place("refs"))?;
                    let uid = EntityId::parse(&t)
                        .ok_or_else(|| broken(&format!("başvurunun nesnesinin kimliği “{t}”")))?;
                    let why = if is_broken {
                        Some(c.text(|| place("refs"))?)
                    } else {
                        None
                    };
                    *slot = Some(SubRef {
                        uid,
                        place: spot,
                        path,
                        broken: why,
                    });
                }
                let [ra, rb, rc] = slots;
                Some(DimensionRefs {
                    a: ra,
                    b: rb,
                    c: rc,
                    rule,
                })
            } else {
                None
            };
            // Its options, moved value and breaks (docs/adr/0321).
            let mut more: serde_json::Map<String, serde_json::Value> = if flags & MORE != 0 {
                let t = c.text(|| place("options"))?;
                match serde_json::from_str(&t) {
                    Ok(serde_json::Value::Object(m)) => m,
                    _ => return Err(broken("ölçünün seçenekleri")),
                }
            } else {
                serde_json::Map::new()
            };
            let mut part = |key: &str| more.remove(key);
            let options: Option<Box<kentos_contracts::DimensionOptions>> = match part("options") {
                Some(v) => {
                    Some(serde_json::from_value(v).map_err(|_| broken("ölçünün seçenekleri"))?)
                }
                None => None,
            };
            let text_shift: Option<kentos_contracts::TextShift> = match part("textShift") {
                Some(v) => Some(serde_json::from_value(v).map_err(|_| broken("değerin kayması"))?),
                None => None,
            };
            let breaks: Vec<kentos_contracts::DimensionBreak> = match part("breaks") {
                Some(v) => serde_json::from_value(v).map_err(|_| broken("ölçünün kırmaları"))?,
                None => Vec::new(),
            };
            let overrides: Vec<String> = match part("overrides") {
                Some(v) => {
                    serde_json::from_value(v).map_err(|_| broken("ölçünün geçersiz kılınanları"))?
                }
                None => Vec::new(),
            };
            let annotative: Option<Box<kentos_contracts::Annotative>> = match part("annotative") {
                Some(v) => Some(serde_json::from_value(v).map_err(|_| broken("ölçünün ölçekleri"))?),
                None => None,
            };
            let level: Option<Box<kentos_contracts::DimensionLevel>> = match part("level") {
                Some(v) => Some(serde_json::from_value(v).map_err(|_| broken("kotun katı"))?),
                None => None,
            };
            Entity::Dimension(DimensionEntity {
                annotative,
                text_shift,
                breaks,
                base,
                a,
                b,
                offset,
                height,
                text,
                style,
                angle,
                c: corner,
                mask: has(4),
                za,
                zb,
                refs: refs.map(Box::new),
                look: DimensionLook {
                    overrides,
                    options,
                    dim_style,
                    arrow,
                    arrow_size,
                    ext_offset,
                    ext_beyond,
                    text_gap,
                    text_place: has(13).then_some(DimensionTextPlace::Centre),
                    decimals,
                    unit,
                    prefix,
                    suffix,
                    font,
                    dim_line_color,
                    dim_line_weight,
                    dim_line_type,
                    ext_color,
                    ext_weight,
                    ext_line_type,
                    text_color,
                },
                level,
            })
        }
        12 => {
            let ring = c.points()?;
            let v = c.usize()?;
            let kind = *HATCH_PATTERNS
                .get(v)
                .ok_or_else(|| broken(&format!("tarama deseni {v}")))?;
            let (angle, spacing) = (c.float()?, c.float()?);
            let holes = if has(0) {
                let h = c.usize()?;
                if h > c.cols.ints.len() {
                    return Err(broken("delik sayısı tam sayılardan fazla"));
                }
                Some((0..h).map(|_| c.points()).collect::<Result<Vec<_>, _>>()?)
            } else {
                None
            };
            let name = if has(1) {
                Some(c.text(|| place("pattern/name"))?)
            } else {
                None
            };
            let scale = if has(2) { Some(c.float()?) } else { None };
            let lines = if has(3) {
                let n = c.usize()?;
                if n > c.cols.ints.len() {
                    return Err(broken("desen ailesi sayısı tam sayılardan fazla"));
                }
                let mut lines = Vec::with_capacity(n);
                for _ in 0..n {
                    let d = c.usize()?;
                    let angle = c.float()?;
                    let origin = [c.float()?, c.float()?];
                    let offset = [c.float()?, c.float()?];
                    let end = c
                        .float
                        .checked_add(d)
                        .filter(|&end| end <= c.cols.floats.len())
                        .ok_or_else(|| broken("kesik listesi sayılardan uzun"))?;
                    let dashes = c.cols.floats[c.float..end].to_vec();
                    c.float = end;
                    lines.push(PatternLine {
                        angle,
                        origin,
                        offset,
                        dashes,
                    });
                }
                Some(lines)
            } else {
                None
            };
            let gradient = if has(4) {
                let v = c.usize()?;
                let shape = *GRADIENT_SHAPES
                    .get(v)
                    .ok_or_else(|| broken(&format!("degrade biçimi {v}")))?;
                let inverted = match c.int()? {
                    0 => false,
                    1 => true,
                    v => return Err(broken(&format!("degradenin ters bayrağı {v}"))),
                };
                Some(HatchGradient {
                    shape,
                    inverted,
                    color2: c.text(|| place("pattern/gradient/color2"))?,
                })
            } else {
                None
            };
            let assoc = if has(5) {
                let (i, k) = (c.usize()?, c.usize()?);
                if i.saturating_add(k) > c.cols.text_lengths.len() {
                    return Err(broken("taramanın nesne sayısı metinlerden fazla"));
                }
                let outer = {
                    let t = c.text(|| place("assoc/outer"))?;
                    if t.is_empty() {
                        None
                    } else {
                        Some(EntityId::parse(&t).ok_or_else(|| {
                            broken(&format!("taramanın nesnesinin kimliği “{t}”"))
                        })?)
                    }
                };
                let mut id = |what: &str| -> Result<EntityId, KcadError> {
                    let t = c.text(|| place(what))?;
                    EntityId::parse(&t)
                        .ok_or_else(|| broken(&format!("taramanın nesnesinin kimliği “{t}”")))
                };
                let islands = (0..i)
                    .map(|_| id("assoc/islands"))
                    .collect::<Result<Vec<_>, _>>()?;
                let cutouts = (0..k)
                    .map(|_| id("assoc/cutouts"))
                    .collect::<Result<Vec<_>, _>>()?;
                Some(HatchAssoc {
                    gap: None,
                    outer,
                    bounds: Vec::new(),
                    islands,
                    cutouts,
                    seed: c.point()?,
                })
            } else {
                None
            };
            let mut assoc = assoc;
            let (mut origin, mut hatch_annotative) = (None, None);
            let boundary = if has(6) {
                #[derive(serde::Deserialize)]
                struct More {
                    #[serde(default)]
                    boundary: Option<Vec<kentos_contracts::HatchLoop>>,
                    #[serde(default)]
                    bounds: Vec<EntityId>,
                    #[serde(default)]
                    gap: Option<f64>,
                    #[serde(default)]
                    origin: Option<kentos_contracts::Vec2>,
                    #[serde(default)]
                    annotative: Option<Box<kentos_contracts::Annotative>>,
                }
                let t = c.text(|| place("boundary"))?;
                let more: More = serde_json::from_str(&t)
                    .map_err(|e| broken(&format!("taramanın sınırı okunamadı: {e}")))?;
                if !more.bounds.is_empty() || more.gap.is_some() {
                    match &mut assoc {
                        Some(a) => {
                            a.bounds = more.bounds;
                            a.gap = more.gap;
                        }
                        None => return Err(broken("ilişkisi olmayan taramanın sınır nesneleri")),
                    }
                }
                origin = more.origin;
                hatch_annotative = more.annotative;
                more.boundary
            } else {
                None
            };
            Entity::Hatch(HatchEntity {
                base,
                ring,
                holes,
                pattern: HatchPattern {
                    kind,
                    angle,
                    spacing,
                    name,
                    scale,
                    lines,
                    gradient,
                    origin,
                },
                assoc,
                boundary,
                annotative: hatch_annotative,
            })
        }
        15 => {
            let p = c.point()?;
            let (rotation, height) = (c.float()?, c.float()?);
            let rows = c.numbers()?;
            let columns = c.numbers()?;
            let r = c.usize()?;
            if r > c.cols.ints.len() {
                return Err(broken("tablonun satır sayısı tam sayılardan fazla"));
            }
            let mut cells = Vec::with_capacity(r);
            for i in 0..r {
                let len = c.usize()?;
                if len > c.cols.text_lengths.len() {
                    return Err(broken("tablonun hücre sayısı metinlerden fazla"));
                }
                cells.push(
                    (0..len)
                        .map(|j| c.text(|| place(&format!("cells/{i}/{j}"))))
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            let merges = if has(0) {
                let q = c.usize()?;
                if q > c.cols.ints.len() {
                    return Err(broken("birleşik alan sayısı tam sayılardan fazla"));
                }
                (0..q)
                    .map(|_| {
                        Ok(kentos_contracts::CellRange {
                            row: c.int()?,
                            col: c.int()?,
                            rows: c.int()?,
                            cols: c.int()?,
                        })
                    })
                    .collect::<Result<Vec<_>, KcadError>>()?
            } else {
                Vec::new()
            };
            let aligns = if has(1) {
                let a = c.usize()?;
                if a > c.cols.ints.len() {
                    return Err(broken("hiza sayısı tam sayılardan fazla"));
                }
                Some(
                    (0..a)
                        .map(|_| {
                            let v = c.usize()?;
                            TableAlign::ALL
                                .get(v)
                                .copied()
                                .ok_or_else(|| broken(&format!("sütun hizası {v}")))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                None
            };
            let grid = if has(3) {
                let v = c.usize()?;
                Some(
                    *TableGrid::ALL
                        .get(v)
                        .ok_or_else(|| broken(&format!("tablo çizgileri {v}")))?,
                )
            } else {
                None
            };
            let text_style = if has(4) {
                Some(c.text(|| place("textStyle"))?)
            } else {
                None
            };
            let font = if has(5) {
                Some(font_at(c.usize()?)?)
            } else {
                None
            };
            let oblique = if has(8) { Some(c.float()?) } else { None };
            let frame = if has(10) { Some(c.float()?) } else { None };
            let mut source = if has(9) {
                match c.int()? {
                    3 => {
                        let sheet = match c.int()? {
                            0 => false,
                            1 => true,
                            v => return Err(broken(&format!("kaynağın sayfa bayrağı {v}"))),
                        };
                        let name = c.text(|| place("source/name"))?;
                        let sheet = if sheet {
                            Some(c.text(|| place("source/sheet"))?)
                        } else {
                            None
                        };
                        Some(TableSource::File {
                            path: None,
                            copy: None,
                            digest: None,
                            overrides: Default::default(),
                            key_column: None,
                            name,
                            sheet,
                        })
                    }
                    kind @ 0..=2 => {
                        let k = c.usize()?;
                        if k > c.cols.text_lengths.len() {
                            return Err(broken("kaynak nesne sayısı metinlerden fazla"));
                        }
                        let objects = (0..k)
                            .map(|_| {
                                let t = c.text(|| place("source/objects"))?;
                                EntityId::parse(&t).ok_or_else(|| {
                                    broken(&format!("kaynak nesnenin kimliği “{t}”"))
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        Some(match kind {
                            0 => TableSource::Coordinates { objects },
                            1 => TableSource::Areas { objects },
                            _ => TableSource::Attributes { objects },
                        })
                    }
                    v => return Err(broken(&format!("tablonun kaynağı {v}"))),
                }
            } else {
                None
            };
            let more: TableCells = if has(11) {
                let t = c.text(|| place("styles"))?;
                serde_json::from_str(&t).map_err(|_| broken("tablonun hücre verisi"))?
            } else {
                TableCells::default()
            };
            if let Some(schedule) = more.schedule {
                source = Some(schedule);
            }
            if let Some(l) = more.link
                && let Some(TableSource::File {
                    path,
                    digest,
                    copy,
                    overrides,
                    key_column,
                    ..
                }) = &mut source
            {
                *path = l.path;
                *digest = l.digest;
                *copy = l.copy;
                *overrides = l.overrides;
                *key_column = l.key_column;
            }
            Entity::Table(TableEntity {
                breaks: more.breaks,
                table_style: more.table_style,
                styles: more.styles,
                formulas: more.formulas,
                fields: more.fields,
                base,
                p,
                rotation,
                height,
                rows,
                columns,
                cells,
                merges,
                aligns,
                header: has(2),
                grid,
                frame,
                face: TextFace {
                    overrides: Vec::new(),
                    text_style,
                    font,
                    bold: has(6),
                    italic: has(7),
                    oblique,
                },
                source,
            })
        }
        16 => {
            let p = c.point()?;
            let (width, height, rotation) = (c.float()?, c.float()?, c.float()?);
            let asset = if has(1) {
                Some(c.text(|| place("asset"))?)
            } else {
                None
            };
            let file = if has(2) {
                Some(c.text(|| place("file"))?)
            } else {
                None
            };
            let clip = if has(3) { Some(c.points()?) } else { None };
            let opacity = if has(4) { Some(c.float()?) } else { None };
            Entity::Image(ImageEntity {
                base,
                image: ImageFields {
                    p,
                    width,
                    height,
                    rotation,
                    mirror: has(0),
                    asset,
                    file,
                    clip,
                    opacity,
                },
            })
        }
        17 => {
            let (width, height, bands) = (c.int()?, c.int()?, c.int()?);
            let sample = *RASTER_SAMPLES
                .get(c.usize()?)
                .ok_or_else(|| broken("rasterin örnek türü"))?;
            let srid = c.int()?;
            let render = *RASTER_RENDERS
                .get(c.usize()?)
                .ok_or_else(|| broken("rasterin görünüş türü"))?;
            let n = c.usize()?;
            if n > 4 {
                return Err(broken("rasterin görünüşünün bantları"));
            }
            let mut look_bands = Vec::with_capacity(n);
            for _ in 0..n {
                look_bands.push(c.int()?);
            }
            let stretch = *RASTER_STRETCHES
                .get(c.usize()?)
                .ok_or_else(|| broken("rasterin gerdirmesi"))?;
            let look = c.int()?;
            let mut affine = [0.0; 6];
            for v in &mut affine {
                *v = c.float()?;
            }
            let opacity = if has(2) { Some(c.float()?) } else { None };
            let mut number = |bit: u32| -> Result<Option<f64>, KcadError> {
                if look & bit != 0 {
                    Ok(Some(c.float()?))
                } else {
                    Ok(None)
                }
            };
            let (min, max) = (number(LOOK_MIN)?, number(LOOK_MAX)?);
            let (azimuth, altitude) = (number(LOOK_AZIMUTH)?, number(LOOK_ALTITUDE)?);
            let (z_factor, nodata) = (number(LOOK_Z_FACTOR)?, number(LOOK_NODATA)?);
            let ramp = if look & LOOK_RAMP != 0 {
                Some(c.text(|| place("style.ramp"))?)
            } else {
                None
            };
            let edges = if look & LOOK_EDGES != 0 {
                Some(c.text(|| place("style.edges"))?)
            } else {
                None
            };
            let style = kentos_contracts::RasterStyle {
                render,
                bands: look_bands,
                stretch,
                min,
                max,
                ramp,
                invert: look & LOOK_INVERT != 0,
                azimuth,
                altitude,
                z_factor,
                nodata,
                resampling: if look & LOOK_NEAREST != 0 {
                    kentos_contracts::RasterResampling::Nearest
                } else {
                    kentos_contracts::RasterResampling::Bilinear
                },
                edges,
            };
            let asset = if has(0) {
                Some(c.text(|| place("asset"))?)
            } else {
                None
            };
            let file = if has(1) {
                Some(c.text(|| place("file"))?)
            } else {
                None
            };
            let url = if has(3) {
                Some(c.text(|| place("url"))?)
            } else {
                None
            };
            let dataset = if has(4) {
                let n = c.usize()?;
                if n > kentos_contracts::MAX_DATASET_DIMS {
                    return Err(broken("rasterin veri setinin boyutları"));
                }
                let has = c.int()?;
                let variable = c.text(|| place("dataset.variable"))?;
                let vector = if has & 2 != 0 {
                    Some(c.text(|| place("dataset.vector"))?)
                } else {
                    None
                };
                let mesh = if has & 4 != 0 {
                    Some(c.text(|| place("dataset.mesh"))?)
                } else {
                    None
                };
                let mut dims = Vec::with_capacity(n);
                for _ in 0..n {
                    let index = c.int()?;
                    let k = c.usize()?;
                    if k > kentos_contracts::MAX_DIM_VALUES {
                        return Err(broken("rasterin veri setinin boyutunun değerleri"));
                    }
                    let dh = c.int()?;
                    let mut values = Vec::with_capacity(k);
                    for _ in 0..k {
                        values.push(c.float()?);
                    }
                    let name = c.text(|| place("dataset.dims.name"))?;
                    let units = if dh & 2 != 0 {
                        Some(c.text(|| place("dataset.dims.units"))?)
                    } else {
                        None
                    };
                    dims.push(kentos_contracts::DatasetDim {
                        name,
                        index,
                        values,
                        time: dh & 1 != 0,
                        units,
                    });
                }
                Some(kentos_contracts::RasterDataset {
                    variable,
                    vector,
                    mesh,
                    dims,
                    follow_time: has & 1 != 0,
                })
            } else {
                None
            };
            let georef = if has(5) {
                let text = c.text(|| place("georef"))?;
                Some(Box::new(
                    serde_json::from_str::<kentos_contracts::RasterGeoref>(&text)
                        .map_err(|_| broken("rasterin oturtma kaydı"))?,
                ))
            } else {
                None
            };
            let tiles = if has(6) {
                let text = c.text(|| place("tiles"))?;
                Some(
                    serde_json::from_str::<kentos_contracts::RasterTiles>(&text)
                        .map_err(|_| broken("rasterin karo tablosu"))?,
                )
            } else {
                None
            };
            Entity::Raster(RasterEntity {
                base,
                raster: RasterFields {
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
            })
        }
        // A NURBS curve (docs/adr/0259): its numbers as written, checked as the file's reader checks them.
        19 => {
            let degree = c.int()?;
            let ctrl = c.points()?;
            let knots = c.numbers()?;
            let flags = c.int()?;
            let on = |b: u32| flags & b != 0;
            let weights = if on(1) { Some(c.numbers()?) } else { None };
            let fit = if on(8) { Some(c.points()?) } else { None };
            let fit_tolerance = if on(16) { Some(c.float()?) } else { None };
            let start = if on(32) { Some(c.point()?) } else { None };
            let end = if on(64) { Some(c.point()?) } else { None };
            let e = kentos_contracts::NurbsEntity {
                base,
                degree,
                ctrl,
                knots,
                weights,
                closed: on(2),
                periodic: on(4),
                fit,
                tangents: (start.is_some() || end.is_some()).then_some([start, end]),
                fit_tolerance,
            };
            if let Some(problem) = e.problem() {
                return Err(broken(&problem));
            }
            Entity::Nurbs(e)
        }
        // A record: nothing more (docs/adr/0410 §2).
        20 => Entity::Record(kentos_contracts::RecordEntity { base }),
        // A wall (docs/adr/0339 §4): its path, then the rest of its source as one JSON text; checked as the file's
        // reader checks it.
        21 => {
            let pts = c.points()?;
            let bulges = if c.int()? & 1 != 0 {
                Some(c.numbers()?)
            } else {
                None
            };
            let text = c.text(|| "duvarın alanları".into())?;
            let mut v: serde_json::Value =
                serde_json::from_str(&text).map_err(|_| broken("duvarın alanları"))?;
            if let Some(o) = v.as_object_mut() {
                o.insert("pts".into(), serde_json::to_value(&pts).unwrap_or_default());
                if let Some(b) = bulges {
                    o.insert("bulges".into(), serde_json::to_value(b).unwrap_or_default());
                }
            }
            let wall: kentos_contracts::WallFields =
                serde_json::from_value(v).map_err(|_| broken("duvarın alanları"))?;
            if let Some((_, problem)) = wall.problem() {
                return Err(broken(&problem));
            }
            Entity::Wall(kentos_contracts::WallEntity { base, wall })
        }
        24..=27 => {
            let what = match kind {
                24 => "aksın alanları",
                25 => "kolonun alanları",
                26 => "merdivenin alanları",
                _ => "rampanın alanları",
            };
            let text = c.text(|| what.into())?;
            let parsed = |s: &str| -> Result<Entity, KcadError> {
                Ok(match kind {
                    24 => {
                        let grid: kentos_contracts::GridFields = serde_json::from_str(s).map_err(|_| broken(what))?;
                        if let Some((_, problem)) = grid.problem() {
                            return Err(broken(&problem));
                        }
                        Entity::Grid(kentos_contracts::GridEntity { base: base.clone(), grid })
                    }
                    25 => {
                        let column: kentos_contracts::ColumnFields =
                            serde_json::from_str(s).map_err(|_| broken(what))?;
                        if let Some((_, problem)) = column.problem() {
                            return Err(broken(&problem));
                        }
                        Entity::Column(kentos_contracts::ColumnEntity { base: base.clone(), column })
                    }
                    26 => {
                        let stair: kentos_contracts::StairFields = serde_json::from_str(s).map_err(|_| broken(what))?;
                        if let Some((_, problem)) = stair.problem() {
                            return Err(broken(&problem));
                        }
                        Entity::Stair(kentos_contracts::StairEntity { base: base.clone(), stair })
                    }
                    _ => {
                        let ramp: kentos_contracts::RampFields = serde_json::from_str(s).map_err(|_| broken(what))?;
                        if let Some((_, problem)) = ramp.problem() {
                            return Err(broken(&problem));
                        }
                        Entity::Ramp(kentos_contracts::RampEntity { base: base.clone(), ramp })
                    }
                })
            };
            parsed(&text)?
        }
        28 => {
            let text = c.text(|| "etiketin alanları".into())?;
            let tag: kentos_contracts::TagFields =
                serde_json::from_str(&text).map_err(|_| broken("etiketin alanları"))?;
            if let Some((_, problem)) = tag.problem() {
                return Err(broken(&problem));
            }
            Entity::Tag(kentos_contracts::TagEntity { base, tag })
        }
        29 => {
            let text = c.text(|| "görünüm işaretinin alanları".into())?;
            let mark: kentos_contracts::ViewMarkFields =
                serde_json::from_str(&text).map_err(|_| broken("görünüm işaretinin alanları"))?;
            if let Some((_, problem)) = mark.problem() {
                return Err(broken(&problem));
            }
            Entity::ViewMark(kentos_contracts::ViewMarkEntity { base, mark })
        }
        30 => {
            let text = c.text(|| "döşemenin alanları".into())?;
            let slab: kentos_contracts::SlabFields =
                serde_json::from_str(&text).map_err(|_| broken("döşemenin alanları"))?;
            if let Some((_, problem)) = slab.problem() {
                return Err(broken(&problem));
            }
            Entity::Slab(kentos_contracts::SlabEntity { base, slab })
        }
        31 => {
            let text = c.text(|| "kirişin alanları".into())?;
            let beam: kentos_contracts::BeamFields =
                serde_json::from_str(&text).map_err(|_| broken("kirişin alanları"))?;
            if let Some((_, problem)) = beam.problem() {
                return Err(broken(&problem));
            }
            Entity::Beam(kentos_contracts::BeamEntity { base, beam })
        }
        32 => {
            let text = c.text(|| "çatının alanları".into())?;
            let roof: kentos_contracts::RoofFields =
                serde_json::from_str(&text).map_err(|_| broken("çatının alanları"))?;
            if let Some((_, problem)) = roof.problem() {
                return Err(broken(&problem));
            }
            Entity::Roof(kentos_contracts::RoofEntity { base, roof })
        }
        33 => {
            let text = c.text(|| "korkuluğun alanları".into())?;
            let railing: kentos_contracts::RailingFields =
                serde_json::from_str(&text).map_err(|_| broken("korkuluğun alanları"))?;
            if let Some((_, problem)) = railing.problem() {
                return Err(broken(&problem));
            }
            Entity::Railing(kentos_contracts::RailingEntity { base, railing })
        }
        23 => {
            let text = c.text(|| "mahalin alanları".into())?;
            let space: kentos_contracts::SpaceFields =
                serde_json::from_str(&text).map_err(|_| broken("mahalin alanları"))?;
            if let Some((_, problem)) = space.problem() {
                return Err(broken(&problem));
            }
            Entity::Space(kentos_contracts::SpaceEntity { base, space })
        }
        22 => {
            let place = c.points()?;
            let text = c.text(|| "açıklığın alanları".into())?;
            let mut v: serde_json::Value =
                serde_json::from_str(&text).map_err(|_| broken("açıklığın alanları"))?;
            if let Some(o) = v.as_object_mut() {
                o.insert("place".into(), serde_json::to_value(&place).unwrap_or_default());
            }
            let opening: kentos_contracts::OpeningFields =
                serde_json::from_value(v).map_err(|_| broken("açıklığın alanları"))?;
            if let Some((_, problem)) = opening.problem() {
                return Err(broken(&problem));
            }
            Entity::Opening(kentos_contracts::OpeningEntity { base, opening })
        }
        18 => {
            let srid = c.int()?;
            let n = c.usize()?;
            if n > kentos_contracts::MAX_CLOUD_SOURCES {
                return Err(broken("nokta bulutunun dosya sayısı"));
            }
            let render = *kentos_contracts::CloudRender::ALL
                .get(c.usize()?)
                .ok_or_else(|| broken("nokta bulutunun görünüş türü"))?;
            let look = c.int()?;
            let k = c.usize()?;
            if k > 256 {
                return Err(broken("nokta bulutunun gizlenen sınıfları"));
            }
            let mut hidden = Vec::with_capacity(k);
            for _ in 0..k {
                hidden.push(
                    u8::try_from(c.int()?)
                        .map_err(|_| broken("nokta bulutunun gizlenen sınıfı"))?,
                );
            }
            let mut kinds = Vec::with_capacity(n);
            for _ in 0..n {
                let format = *kentos_contracts::CloudFormat::ALL
                    .get(c.usize()?)
                    .ok_or_else(|| broken("nokta bulutu dosyasının biçimi"))?;
                kinds.push((format, c.int()?));
            }
            let mut bounds = [0.0; 6];
            for v in &mut bounds {
                *v = c.float()?;
            }
            let points = c.float()? as u64;
            let size = c.float()?;
            let opacity = if has(0) { Some(c.float()?) } else { None };
            let min = if look & CLOUD_MIN != 0 {
                Some(c.float()?)
            } else {
                None
            };
            let max = if look & CLOUD_MAX != 0 {
                Some(c.float()?)
            } else {
                None
            };
            let mut numbers = Vec::with_capacity(n);
            for (_, source) in &kinds {
                let count = c.float()? as u64;
                let mut b = [0.0; 6];
                for v in &mut b {
                    *v = c.float()?;
                }
                let pose = if source & SOURCE_POSE != 0 {
                    let mut m = [0.0; 16];
                    for v in &mut m {
                        *v = c.float()?;
                    }
                    Some(m)
                } else {
                    None
                };
                numbers.push((count, b, pose));
            }
            let ramp = if look & CLOUD_RAMP != 0 {
                Some(c.text(|| place("style.ramp"))?)
            } else {
                None
            };
            let mut sources = Vec::with_capacity(n);
            for (i, ((format, source), (count, b, pose))) in kinds.into_iter().zip(numbers).enumerate() {
                let t = c.text(|| place(&format!("sources[{i}]")))?;
                let (asset, file, url) = match source & !SOURCE_POSE {
                    SOURCE_ASSET => (Some(t), None, None),
                    SOURCE_FILE => (None, Some(t), None),
                    SOURCE_URL => (None, None, Some(t)),
                    _ => return Err(broken("nokta bulutu dosyasının kaynağı")),
                };
                sources.push(kentos_contracts::CloudSource {
                    asset,
                    file,
                    url,
                    format,
                    count,
                    bounds: b,
                    pose,
                });
            }
            let style = kentos_contracts::PointCloudStyle {
                render,
                ramp,
                invert: look & CLOUD_INVERT != 0,
                min,
                max,
                hidden,
                rgb8: look & CLOUD_RGB8 != 0,
                size,
                size_unit: if look & CLOUD_METRES != 0 {
                    kentos_contracts::PointSizeUnit::M
                } else {
                    kentos_contracts::PointSizeUnit::Px
                },
                shape: if look & CLOUD_SQUARE != 0 {
                    kentos_contracts::PointShape::Square
                } else {
                    kentos_contracts::PointShape::Round
                },
            };
            Entity::PointCloud(kentos_contracts::PointCloudEntity {
                base,
                cloud: kentos_contracts::PointCloudFields {
                    sources,
                    bounds,
                    count: points,
                    srid,
                    style,
                    opacity,
                },
            })
        }
        13 => {
            let p = c.point()?;
            let (scale, rotation) = (c.float()?, c.float()?);
            let y_scale = if has(1) { Some(c.float()?) } else { None };
            let text = c.text(|| place("block"))?;
            let block =
                BlockId::parse(&text).ok_or_else(|| broken(&format!("blok kimliği “{text}”")))?;
            let mut attribute_places = std::collections::BTreeMap::new();
            if has(2) {
                let n = c.int()?;
                for _ in 0..n {
                    let tag = c.text(|| place("attributePlaces"))?;
                    let at = c.point()?;
                    attribute_places.insert(tag, at);
                }
                if attribute_places.is_empty() {
                    return Err(broken("boş öznitelik yerleri"));
                }
            }
            let parameters = if has(3) {
                let text = c.text(|| place("parameters"))?;
                let values: std::collections::BTreeMap<String, kentos_contracts::ParamValue> =
                    serde_json::from_str(&text)
                        .map_err(|_| broken("parametre değerleri okunamadı"))?;
                if values.is_empty() {
                    return Err(broken("boş parametre değerleri"));
                }
                values
            } else {
                Default::default()
            };
            let clipping = if has(4) {
                let text = c.text(|| place("clipping"))?;
                let clip: kentos_contracts::InsertClip =
                    serde_json::from_str(&text).map_err(|_| broken("kırpma sınırı okunamadı"))?;
                if clip.problem().is_some() {
                    return Err(broken("kırpma sınırı geçersiz"));
                }
                Some(clip)
            } else {
                None
            };
            let arch = if has(5) {
                let text = c.text(|| place("arch"))?;
                let arch: kentos_contracts::ArchInfo = serde_json::from_str(&text)
                    .map_err(|_| broken("yerleştirmenin katı ve tadilat durumu okunamadı"))?;
                if arch.is_empty() {
                    return Err(broken("boş kat ve tadilat durumu"));
                }
                arch
            } else {
                Default::default()
            };
            Entity::Insert(InsertEntity {
                arch,
                clipping,
                parameters,
                attribute_places,
                base,
                block,
                p,
                scale,
                y_scale,
                rotation,
                mirror: has(0),
            })
        }
        _ => {
            let pts = c.points()?;
            let (height, rotation) = (c.float()?, c.float()?);
            let text = if has(0) {
                Some(c.text(|| place("text"))?)
            } else {
                None
            };
            let arrow = if has(1) {
                let v = c.usize()?;
                Some(
                    *LeaderArrow::ALL
                        .get(v)
                        .ok_or_else(|| broken(&format!("kılavuz oku {v}")))?,
                )
            } else {
                None
            };
            let arrow_size = if has(3) { Some(c.float()?) } else { None };
            let annotative: Option<Box<kentos_contracts::Annotative>> = if has(4) {
                let t = c.text(|| place("annotative"))?;
                Some(serde_json::from_str(&t).map_err(|_| broken("kılavuzun ölçekleri"))?)
            } else {
                None
            };
            let more: Multileader = if has(5) {
                let t = c.text(|| place("arms"))?;
                serde_json::from_str(&t).map_err(|_| broken("kılavuzun kolları"))?
            } else {
                Multileader::default()
            };
            Entity::Leader(LeaderEntity {
                tip: more.tip,
                leader_style: more.leader_style,
                blocks: more.blocks,
                arms: more.arms,
                annotative,
                base,
                pts,
                text,
                height,
                rotation,
                arrow,
                arrow_size,
                mask: has(2),
            })
        }
    })
}
