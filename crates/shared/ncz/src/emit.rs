//! NCZ records into the app's objects (docs/adr/0138). `format` hands over
//! the plugin's records; each becomes what KentOS has:
//!
//! | NCZ | KentOS |
//! |---|---|
//! | Point | a point; its name the label and the `Ad` attribute |
//! | Symbol, Block | a point; the symbol code (`Sembol`) or block name (`Blok`) as an attribute |
//! | Line | a line |
//! | Polyline | a polyline |
//! | Polygon, Triangle | a closed area, the repeated first vertex left out |
//! | MapSheet | a closed area: its cell's four corners in the zone the file names (`sheet`), or the box the file keeps; the sheet's name the `Pafta` attribute |
//! | Circle | a circle, centre and radius (not the plugin's 72 chords) |
//! | Arc | an arc, its ends from the plugin's own reading of the angles |
//! | Text | a text on its baseline, height and rotation |
//! | NCZ 8 smart object | its symbol: circles, lines and texts, turned and scaled as the object; its values as attributes of the symbol's first object |
//! | other smart objects | a point (no rectangle) or an area, the token as an attribute |
//!
//! A point keeps its height, and a line, a polyline and an area the height of
//! each vertex (docs/adr/0142), when the record has any (`heights`): a record
//! whose Z is 0 everywhere is a 2D one, and the vertices the reader made up
//! (a rectangle's, a sheet's, a triangle's two others) have none to give.
//!
//! A record's own colour is kept only where it is not its layer's; black and
//! white are the app's `ink`, which is black on paper and white on a dark
//! drawing, as DXF colour 7 is. The texts an NCZ holds on an area (a parcel
//! number, an id) are not drawn by its own program and are not labels here either:
//! they are the `Etiket` attribute, or every area of a plan would show a
//! number beside the plan's own number texts.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_contracts::{
    ArcEntity, Bounds, CircleEntity, Entity, EntityBase, ImportLayer, ImportResult, LineEntity,
    LineType, NczReadOptions, PathEntity, PointEntity, TextAlign, TextEntity, Vec2,
};
use kentos_formats::math::{TAU, cos, hypot, norm_angle, rad, sin, sin_cos_deg};
use kentos_formats::report::Report;
use kentos_formats::watch::{STOPPED, Watch};
use kentos_geometry_core::text::{Font, width_em};

use crate::format::{self, Header, Kind, Outcome, RAD_TO_DEG, SmartClass};
use crate::sheet::{self, Kept, Zone};
use crate::symbols::{self, Anchor, Stroke};
use crate::{attributes, crs};

/// Objects read at most, unless the caller says otherwise.
const DEFAULT_LIMIT: usize = 1_000_000;
/// The reference's validity bound on a coordinate, in metres: ±100 000 km.
const WORLD: f64 = 100_000_000.0;
/// A wrapped note's lines stand this many text heights apart (the height a
/// plan note's text gets is worked out for it, `symbols::plan_note`).
const LINE_PITCH: f64 = 1.45;

/// Why a circle's or an arc's radius is no radius, if it is not.
fn radius(r: f64) -> Result<(), &'static str> {
    if !r.is_finite() {
        Err("yarıçapı sayı değil")
    } else if r <= 0.0 {
        Err("yarıçapı sıfır ya da eksi")
    } else if r > WORLD {
        Err("yarıçapı 100 000 km'den büyük")
    } else {
        Ok(())
    }
}

/// What the report calls a record kind.
fn kind_words(k: Kind) -> &'static str {
    match k {
        Kind::Point => "Nokta",
        Kind::Line => "Çizgi",
        Kind::Polyline => "Çoklu çizgi",
        Kind::Polygon => "Kapalı alan",
        Kind::Circle => "Daire",
        Kind::Arc => "Yay",
        Kind::Text => "Yazı",
        Kind::Symbol => "Sembol",
        Kind::Block => "Blok",
        Kind::MapSheet => "Pafta",
        Kind::Triangle => "Üçgen",
        Kind::SmartObject => "Akıllı nesne",
    }
}

/// A geometry type byte's words, for a record the reference drops or does not know.
fn type_words(t: usize) -> String {
    match t {
        1 => "Nokta".into(),
        2 => "Çizgi".into(),
        3 => "Daire".into(),
        4 => "Yay".into(),
        5 => "Yazı".into(),
        6 => "Sembol".into(),
        7 | 9 => "Çoklu çizgi".into(),
        10 => "Kapalı alan".into(),
        11 => "Pafta".into(),
        12 => "Üçgen".into(),
        13 => "Blok".into(),
        15 => "Akıllı nesne".into(),
        _ => format!("NCZ türü {t}"),
    }
}

/// A colour of the file as the app writes one: `#RRGGBB`; black and white,
/// and what is all but black or white, the app's `ink`. its own program draws a plan
/// in black on a white page, and not always in exact black: the Sivas UİP's
/// settlement symbols are #101410, its Emsal layer #000100. On a dark drawing
/// those would vanish; `ink` is black on paper and white on a dark drawing,
/// as DXF colour 7 is. A grey anyone can tell from black (#404040) stays.
fn app_color(argb: u32) -> String {
    let (r, g, b) = ((argb >> 16) & 255, (argb >> 8) & 255, argb & 255);
    if r.max(g).max(b) <= 0x20 || r.min(g).min(b) >= 0xF0 {
        return "ink".to_owned();
    }
    format!("#{:06X}", argb & 0xFF_FFFF)
}

/// What the file calls its version, when it reads as one: a build number
/// (`5.2.0.1035N`, `8.5.6.1095`). The reference takes the first block that
/// CLAIMS the version's type, so a file with none hands over whatever bytes
/// follow; a report is not the place for them.
fn version(h: &Header) -> Option<&str> {
    let v = h.version_name.as_str();
    let versionlike = !v.is_empty()
        && v.len() <= 32
        && v.as_bytes()[0].is_ascii_digit()
        && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.');
    versionlike.then_some(v)
}

/// A turn, scale and place: a symbol's space into the drawing's.
struct Place {
    at: Vec2,
    size: f64,
    /// Degrees, counter-clockwise from east.
    rotation: f64,
    sin: f64,
    cos: f64,
}

impl Place {
    fn new(at: Vec2, size: f64, rotation: f64) -> Self {
        let (s, c) = sin_cos_deg(rotation);
        Self {
            at,
            size,
            rotation,
            sin: s,
            cos: c,
        }
    }

    fn apply(&self, (x, y): (f64, f64)) -> Vec2 {
        let (x, y) = (x * self.size, y * self.size);
        Vec2 {
            x: self.at.x + (x * self.cos - y * self.sin),
            y: self.at.y + (x * self.sin + y * self.cos),
        }
    }
}

/// The elevations a record's vertices hold, in the objects' form (docs/adr/0142):
/// none unless some vertex has a Z above or below 0 (a 2D drawing's Z is 0, and
/// so is what an NCZ holds for a vertex it has no height for); a Z that is not a
/// number is none. Only a record whose vertices the file gave (a line, a
/// polyline, an area drawn point by point) says what its heights are.
fn heights(coords: &[format::Coord]) -> Option<Vec<Option<f64>>> {
    let zs: Vec<Option<f64>> = coords
        .iter()
        .map(|c| c.z.is_finite().then_some(c.z))
        .collect();
    zs.iter().flatten().any(|&z| z != 0.0).then_some(zs)
}

/// A rotation in [0°, 360°): the file's, a NaN none.
fn upright(rotation: f64) -> f64 {
    let r = rotation.rem_euclid(360.0);
    if r >= 360.0 { 0.0 } else { r }
}

struct Layer {
    name: String,
    color: Option<u32>,
    count: u32,
    /// Objects of it that draw a smart object (its symbol, or its point).
    smart: u32,
}

/// Each NCZ record, into the objects of the result.
struct Emitter {
    fin: Header,
    font: Font,
    limit: usize,
    out: Vec<Entity>,
    report: Report,
    layers: Vec<Layer>,
    by_name: HashMap<String, usize>,
    truncated: u32,
    renamed: BTreeSet<String>,
    /// Symbol points drawn with the zoning legend's mark (`legend_mark`), and the codes without one, counted.
    legend: (u32, BTreeMap<String, u32>),
    widths: u32,
    thinnest: f64,
    widest: f64,
    full_turns: u32,
    flat_rotation: u32,
    drawn: BTreeMap<SmartClass, u32>,
    undrawn: BTreeMap<SmartClass, u32>,
    unsized_objects: u32,
    bounds: Option<Bounds>,
    /// The zone the file's MPROJ names, for its sheets' frames; or why there is none.
    zone: Result<Zone, Kept>,
    /// Sheets drawn as their cell, and those that keep the file's box, by why.
    framed: u32,
    boxed: BTreeMap<Kept, u32>,
    /// Records not taken, by kind and reason: how many, and the first one's place when it has one (plan/render.md
    /// RM-17): the report says where to look in the source.
    skipped: BTreeMap<(&'static str, &'static str), (u32, Option<Vec2>)>,
}

impl format::Sink for Emitter {
    fn begin(&mut self, final_header: &Header) -> bool {
        self.fin = final_header.clone();
        self.zone = sheet::zone(final_header);
        true
    }

    fn entity(&mut self, e: &format::Entity) -> bool {
        let layer = self.layer_of(e);
        if self.out.len() >= self.limit {
            self.truncated += 1;
            return true;
        }
        let first = self.out.len();
        match self.place(e, layer) {
            Ok(()) => {
                // The pen, tenths of a millimetre: the object's own weight, as the C++ reader
                // gives it (docs/adr/0139). Zero, a negative width (the NCZ program's own DXF export
                // writes those as the thinnest line) or one past 100 mm is the layer's.
                if e.line_width.is_finite() && e.line_width > 0.0 && e.line_width <= 1000.0 {
                    let mm = e.line_width / 10.0;
                    self.widths += 1;
                    self.widest = self.widest.max(mm);
                    self.thinnest = if self.widths == 1 {
                        mm
                    } else {
                        self.thinnest.min(mm)
                    };
                    // Every object the record made: a smart object's symbol is drawn in its pen.
                    for made in &mut self.out[first..] {
                        made.base_mut().line_weight = Some(mm);
                    }
                }
            }
            Err(why) => {
                let at = e.coords.first().filter(|c| {
                    c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD
                });
                let entry = self.skipped.entry((kind_words(e.kind), why)).or_insert((0, None));
                entry.0 += 1;
                if entry.1.is_none() {
                    entry.1 = at.map(|c| Vec2 { x: c.x, y: c.y });
                }
            }
        }
        true
    }
}

impl Emitter {
    fn new(opts: &NczReadOptions) -> Self {
        Self {
            fin: Header::default(),
            font: Font::from_id(&opts.drawing_font),
            limit: if opts.max_entities == 0 {
                DEFAULT_LIMIT
            } else {
                opts.max_entities as usize
            },
            out: Vec::new(),
            report: Report::default(),
            layers: Vec::new(),
            by_name: HashMap::new(),
            truncated: 0,
            renamed: BTreeSet::new(),
            legend: (0, BTreeMap::new()),
            widths: 0,
            thinnest: 0.0,
            widest: 0.0,
            full_turns: 0,
            flat_rotation: 0,
            drawn: BTreeMap::new(),
            undrawn: BTreeMap::new(),
            unsized_objects: 0,
            bounds: None,
            zone: Err(Kept::Unsaid),
            framed: 0,
            boxed: BTreeMap::new(),
            skipped: BTreeMap::new(),
        }
    }

    /// The layer a record lands on: its name in the file without control
    /// characters, or `KATMAN_<code>` when no table names its code.
    fn layer_of(&mut self, e: &format::Entity) -> usize {
        let clean: String = e.layer_name.chars().filter(|c| !c.is_control()).collect();
        if clean.len() != e.layer_name.len() {
            self.renamed.insert(e.layer_name.clone());
        }
        let name = if clean.trim().is_empty() {
            format!("KATMAN_{}", e.layer_code)
        } else {
            clean
        };
        if let Some(&i) = self.by_name.get(&name) {
            return i;
        }
        let i = self.layers.len();
        self.layers.push(Layer {
            name: name.clone(),
            // The layer's OWN colour, what a record of colour code 0 on it
            // takes, from the tables as they stand at the end of the file.
            color: format::layer_color(&self.fin, e.layer_code),
            count: 0,
            smart: 0,
        });
        self.by_name.insert(name, i);
        i
    }

    fn base(&self, e: &format::Entity, layer: usize) -> EntityBase {
        let l = &self.layers[layer];
        let own = e.color.map(app_color);
        let layers = l.color.map(app_color).unwrap_or_else(|| "ink".to_owned());
        EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: l.name.clone(),
            color: own.filter(|c| *c != layers),
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        }
    }

    fn grow(&mut self, p: Vec2) {
        let b = self.bounds.get_or_insert(Bounds {
            min_x: p.x,
            min_y: p.y,
            max_x: p.x,
            max_y: p.y,
        });
        b.min_x = b.min_x.min(p.x);
        b.min_y = b.min_y.min(p.y);
        b.max_x = b.max_x.max(p.x);
        b.max_y = b.max_y.max(p.y);
    }

    fn push(&mut self, layer: usize, e: Entity) {
        self.report.count(e.kind());
        self.layers[layer].count += 1;
        self.out.push(e);
    }

    /// `push`, for an object that draws a smart object.
    fn push_smart(&mut self, layer: usize, e: Entity) {
        self.layers[layer].smart += 1;
        self.push(layer, e);
    }

    fn place(&mut self, e: &format::Entity, layer: usize) -> Result<(), &'static str> {
        const BAD: &str = "koordinatı sayı değil ya da ±100 000 km dışında";
        let finite = |c: &format::Coord| {
            c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD
        };
        let v = |c: &format::Coord| Vec2 { x: c.x, y: c.y };
        let mut b = self.base(e, layer);
        let note = |b: &mut EntityBase, key: &str, value: &str| {
            if !value.is_empty() {
                b.attrs.insert(key.to_owned(), value.to_owned());
            }
        };
        match e.kind {
            Kind::Point | Kind::Symbol | Kind::Block => {
                let c = e.coords.first().filter(|c| finite(c)).ok_or(BAD)?;
                let mut z = None;
                match e.kind {
                    Kind::Point => {
                        // The reference's height, unless it was the pen width it fell back to.
                        if !e.z_is_width && c.z.is_finite() && c.z != 0.0 {
                            z = Some(c.z);
                        }
                        if !e.name.is_empty() {
                            b.label = Some(e.name.clone());
                            note(&mut b, "Ad", &e.name);
                        }
                    }
                    Kind::Symbol => {
                        note(&mut b, "Sembol", &e.label);
                        // The zoning legend's mark the code stands for, from KentOS's MPYY symbols (docs/adr/9218).
                        match legend_mark(e.label.trim()) {
                            Some((symbol, read)) => {
                                b.symbol = Some(symbol.to_owned());
                                if let Some((key, value)) = read {
                                    b.attrs.insert(key.to_owned(), value.to_owned());
                                }
                                self.legend.0 += 1;
                            }
                            None if !e.label.trim().is_empty() => {
                                *self.legend.1.entry(e.label.trim().to_owned()).or_default() += 1;
                            }
                            None => {}
                        }
                    }
                    _ => note(&mut b, "Blok", &e.label),
                }
                self.grow(v(c));
                self.push(
                    layer,
                    Entity::Point(PointEntity {
                        base: b,
                        p: v(c),
                        z,
                        parts: None,
                    }),
                );
            }
            Kind::Line => {
                let [a, z] = [e.coords.first(), e.coords.get(1)].map(|c| c.filter(|c| finite(c)));
                let (a, z) = (a.ok_or(BAD)?, z.ok_or(BAD)?);
                self.grow(v(a));
                self.grow(v(z));
                let (za, zb) = match heights(&[*a, *z]).as_deref() {
                    Some([za, zb]) => (*za, *zb),
                    _ => (None, None),
                };
                self.push(
                    layer,
                    Entity::Line(LineEntity {
                        ma: None,
                        mb: None,
                        base: b,
                        a: v(a),
                        b: v(z),
                        za,
                        zb,
                    }),
                );
            }
            Kind::Polyline => {
                if !e.coords.iter().all(finite) {
                    return Err(BAD);
                }
                if e.coords.len() < 2 {
                    return Err("iki noktası yok");
                }
                note(&mut b, "Etiket", &e.label);
                let pts: Vec<Vec2> = e.coords.iter().map(v).collect();
                for p in &pts {
                    self.grow(*p);
                }
                let zs = heights(&e.coords);
                self.push(
                    layer,
                    Entity::Polyline(PathEntity {
                        space_separator: None,
                        ms: None,
                        base: b,
                        pts,
                        bulges: None,
                        holes: None,
                        zs,
                        parts: None,
                        closed: false,
                        widths: None,
                    }),
                );
            }
            Kind::MapSheet => {
                note(&mut b, "Pafta", &e.label);
                self.sheet(e, layer, b)?;
            }
            Kind::Polygon | Kind::Triangle => {
                note(&mut b, "Etiket", &e.label);
                self.area(e, layer, b)?;
            }
            Kind::SmartObject if e.smart != SmartClass::None => self.planet(e, layer, b)?,
            Kind::SmartObject => {
                // A notation anchored at a point, or the reference's rectangle: a smart
                // object still, so its layer is drawn over the plan as the symbols' are.
                note(&mut b, "Akıllı nesne", &e.label);
                if e.coords.len() == 1 {
                    let c = e.coords.first().filter(|c| finite(c)).ok_or(BAD)?;
                    self.grow(v(c));
                    self.push_smart(
                        layer,
                        Entity::Point(PointEntity {
                            base: b,
                            p: v(c),
                            z: None,
                            parts: None,
                        }),
                    );
                } else {
                    self.area(e, layer, b)?;
                    self.layers[layer].smart += 1;
                }
            }
            Kind::Circle => {
                let c = e.coords.first().filter(|c| finite(c)).ok_or(BAD)?;
                radius(e.radius)?;
                self.grow(v(c));
                self.push(
                    layer,
                    Entity::Circle(CircleEntity {
                        base: b,
                        c: v(c),
                        r: e.radius,
                    }),
                );
            }
            Kind::Arc => self.arc(e, layer, b)?,
            Kind::Text => {
                let c = e.coords.first().filter(|c| finite(c)).ok_or(BAD)?;
                let rotation = if e.rotation.is_finite() {
                    upright(e.rotation)
                } else {
                    self.flat_rotation += 1;
                    0.0
                };
                self.grow(v(c));
                self.push(
                    layer,
                    Entity::Text(TextEntity {
                        annotative: None,
                        base: b,
                        p: v(c),
                        text: e.label.clone(),
                        height: e.text_height,
                        rotation,
                        align: None,
                        width_factor: None,
                        mask: false,
                        label_of: None,
                        label_scale: None,
                        paragraph: Default::default(),
                        face: Default::default(),
                        path: None,
                    }),
                );
            }
        }
        Ok(())
    }

    /// A pafta: the file keeps the box of its cell in the file's zone, and the cell is a turned
    /// quadrilateral there; its four corners when the file names the zone and the box is a
    /// grid cell's, the box otherwise (`sheet`).
    fn sheet(
        &mut self,
        e: &format::Entity,
        layer: usize,
        b: EntityBase,
    ) -> Result<(), &'static str> {
        let (xs, ys) = (e.coords.iter().map(|c| c.x), e.coords.iter().map(|c| c.y));
        let target = [
            xs.clone().fold(f64::INFINITY, f64::min),
            ys.clone().fold(f64::INFINITY, f64::min),
            xs.fold(f64::NEG_INFINITY, f64::max),
            ys.fold(f64::NEG_INFINITY, f64::max),
        ];
        let frame = match &self.zone {
            Ok(zone) => sheet::frame(&zone.tm, target).ok_or(Kept::NoCell),
            Err(why) => Err(*why),
        };
        match frame {
            Ok(corners) => {
                let pts: Vec<Vec2> = corners.iter().map(|&[x, y]| Vec2 { x, y }).collect();
                for p in &pts {
                    self.grow(*p);
                }
                self.push(
                    layer,
                    Entity::Polygon(PathEntity {
                        space_separator: None,
                        ms: None,
                        base: b,
                        pts,
                        bulges: None,
                        holes: None,
                        zs: None,
                        parts: None,
                        closed: false,
                        widths: None,
                    }),
                );
                self.framed += 1;
            }
            Err(why) => {
                self.area(e, layer, b)?;
                *self.boxed.entry(why).or_default() += 1;
            }
        }
        Ok(())
    }

    /// A closed run as an area: the repeated first vertex left out.
    fn area(
        &mut self,
        e: &format::Entity,
        layer: usize,
        b: EntityBase,
    ) -> Result<(), &'static str> {
        let finite = |c: &format::Coord| {
            c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD
        };
        if !e.coords.iter().all(finite) {
            return Err("koordinatı sayı değil ya da ±100 000 km dışında");
        }
        let mut pts: Vec<Vec2> = e.coords.iter().map(|c| Vec2 { x: c.x, y: c.y }).collect();
        // Only a ring the file drew point by point has heights; a rectangle's and a sheet's corners are made up here.
        let mut zs = if e.kind == Kind::Polygon {
            heights(&e.coords)
        } else {
            None
        };
        while pts.len() >= 2 && pts.last() == pts.first() {
            pts.pop();
            if let Some(z) = zs.as_mut() {
                z.pop();
            }
        }
        if pts.len() < 3 {
            return Err("kapalı şekil üç köşeye ulaşmıyor");
        }
        for p in &pts {
            self.grow(*p);
        }
        let zs = zs.filter(|z| z.iter().any(Option::is_some));
        self.push(
            layer,
            Entity::Polygon(PathEntity {
                space_separator: None,
                ms: None,
                base: b,
                pts,
                bulges: None,
                holes: None,
                zs,
                parts: None,
                closed: false,
                widths: None,
            }),
        );
        Ok(())
    }

    /// The arc the plugin draws (`_approximate_arc`): the angles in degrees
    /// when either is past a full turn in radians, radians otherwise; the end
    /// brought past the start a turn at a time; nothing when the sweep is
    /// empty or over ten turns. A whole turn or more covers the circle: a
    /// circle, also when its ends meet (the reference's chord check lets that
    /// fall); ends that meet after next to no sweep are no arc.
    fn arc(&mut self, e: &format::Entity, layer: usize, b: EntityBase) -> Result<(), &'static str> {
        let c = e
            .coords
            .first()
            .filter(|c| {
                c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD
            })
            .ok_or("koordinatı sayı değil ya da ±100 000 km dışında")?;
        let (r, start, end) = (e.radius, e.start_angle, e.end_angle);
        radius(r)?;
        if !(start.is_finite() && end.is_finite()) {
            return Err("açıları sayı değil");
        }
        let radians = start.abs() <= TAU + 0.001 && end.abs() <= TAU + 0.001;
        let (s_deg, mut e_deg) = if radians {
            (start * RAD_TO_DEG, end * RAD_TO_DEG)
        } else {
            (start, end)
        };
        // `while end < start: end += 360.0`, a turn at a time as the plugin adds it.
        let mut turns = 0u32;
        while e_deg < s_deg {
            if turns == 10_000 {
                return Err("açıları birbirinden çok uzak");
            }
            e_deg += 360.0;
            turns += 1;
        }
        let sweep = e_deg - s_deg;
        if sweep <= 0.0 {
            return Err("açıklığı boş");
        }
        if sweep > 3600.0 {
            return Err("açıklığı on turdan geniş");
        }
        // The plugin's last point is `start + sweep * steps / steps`, and a
        // chord under a nanometre between its ends is no arc to it.
        let steps = ((48.0 * sweep.max(1.0)) / 360.0).floor().max(8.0);
        let last = s_deg + sweep * steps / steps;
        let (a, z) = (rad(s_deg), rad(last));
        let chord = hypot(cos(z) * r - cos(a) * r, sin(z) * r - sin(a) * r);
        // Ends that meet: a sweep near a whole turn (359.999…°) covers the circle, as more than a turn does; one near
        // nothing is no arc at all. The reference lets both fall (plan/render.md RM-17: a full turn was lost).
        let closed = chord < 1e-9;
        if closed && sweep < 180.0 {
            return Err("açıklığı boş");
        }
        let centre = Vec2 { x: c.x, y: c.y };
        for p in [
            Vec2 {
                x: c.x - r,
                y: c.y - r,
            },
            Vec2 {
                x: c.x + r,
                y: c.y + r,
            },
        ] {
            self.grow(p);
        }
        if sweep >= 360.0 || closed {
            self.full_turns += 1;
            self.push(
                layer,
                Entity::Circle(CircleEntity {
                    base: b,
                    c: centre,
                    r,
                }),
            );
            return Ok(());
        }
        // The ends as the file holds them: radians unconverted, else the degrees'.
        let (a0, a1) = if radians {
            (norm_angle(start), norm_angle(end))
        } else {
            (norm_angle(rad(start)), norm_angle(rad(end)))
        };
        self.push(
            layer,
            Entity::Arc(ArcEntity {
                base: b,
                c: centre,
                r,
                a0,
                a1,
            }),
        );
        Ok(())
    }

    /// An NCZ 8 smart object as the symbol it is: its strokes turned,
    /// scaled and placed as the object; its values as attributes of the
    /// symbol's first object. A class this reader does not draw is a point.
    fn planet(
        &mut self,
        e: &format::Entity,
        layer: usize,
        mut b: EntityBase,
    ) -> Result<(), &'static str> {
        let c = e
            .coords
            .first()
            .filter(|c| {
                c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD
            })
            .ok_or("koordinatı sayı değil ya da ±100 000 km dışında")?;
        let at = Vec2 { x: c.x, y: c.y };
        b.attrs = properties(e);
        let font = self.font;
        let measure = move |t: &str, h: f64| width_em(t, font) * h;
        let Some(symbol) = symbols::planet_symbol(e, &measure) else {
            *self.undrawn.entry(e.smart).or_default() += 1;
            self.grow(at);
            self.push_smart(
                layer,
                Entity::Point(PointEntity {
                    base: b,
                    p: at,
                    z: None,
                    parts: None,
                }),
            );
            return Ok(());
        };
        let mut size = e.scale;
        if !(size.is_finite() && size > 0.0 && size <= 1000.0) {
            size = 1.0; // the NCZ program's own default object size
            self.unsized_objects += 1;
        }
        let rotation = if e.rotation.is_finite() {
            upright(e.rotation)
        } else {
            0.0
        };
        let place = Place::new(at, size, rotation);
        *self.drawn.entry(e.smart).or_default() += 1;
        // The first object carries the values; the rest only draw.
        let mut plain = b.clone();
        plain.attrs.clear();
        let mut first = Some(b);
        let mut next_base = || first.take().unwrap_or_else(|| plain.clone());
        for stroke in &symbol.strokes {
            match stroke {
                Stroke::Circle { centre, radius } => {
                    let c = place.apply(*centre);
                    let r = radius * size;
                    self.grow(Vec2 {
                        x: c.x - r,
                        y: c.y - r,
                    });
                    self.grow(Vec2 {
                        x: c.x + r,
                        y: c.y + r,
                    });
                    let base = next_base();
                    self.push_smart(layer, Entity::Circle(CircleEntity { base, c, r }));
                }
                Stroke::Polyline { points, closed } => {
                    let pts: Vec<Vec2> = points.iter().map(|p| place.apply(*p)).collect();
                    for p in &pts {
                        self.grow(*p);
                    }
                    let base = next_base();
                    let entity = match (pts.as_slice(), closed) {
                        ([a, z], false) => Entity::Line(LineEntity {
                            ma: None,
                            mb: None,
                            base,
                            a: *a,
                            b: *z,
                            za: None,
                            zb: None,
                        }),
                        (_, true) => Entity::Polygon(PathEntity {
                            space_separator: None,
                            ms: None,
                            base,
                            pts,
                            bulges: None,
                            holes: None,
                            zs: None,
                            parts: None,
                            closed: false,
                            widths: None,
                        }),
                        _ => Entity::Polyline(PathEntity {
                            space_separator: None,
                            ms: None,
                            base,
                            pts,
                            bulges: None,
                            holes: None,
                            zs: None,
                            parts: None,
                            closed: false,
                            widths: None,
                        }),
                    };
                    self.push_smart(layer, entity);
                }
                Stroke::Text {
                    text,
                    at,
                    height,
                    anchor,
                    wrap,
                } => {
                    let lines = if *wrap > 0.0 {
                        wrapped(text, *wrap, *height, font)
                    } else {
                        text.split('\n').map(str::to_owned).collect()
                    };
                    for (i, line) in lines.iter().enumerate() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        // Its anchor is its alignment (docs/adr/0145 §7): each line stands on its
                        // own, a pitch under the one before.
                        let p = place.apply((at.0, at.1 - i as f64 * LINE_PITCH * height));
                        self.grow(p);
                        let base = next_base();
                        self.push_smart(
                            layer,
                            Entity::Text(TextEntity {
                                annotative: None,
                                base,
                                p,
                                text: line.clone(),
                                height: height * size,
                                rotation: place.rotation,
                                align: align_of(*anchor),
                                width_factor: None,
                                mask: false,
                                label_of: None,
                                label_scale: None,
                                paragraph: Default::default(),
                                face: Default::default(),
                                path: None,
                            }),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// The report's lines for what the reference counts or lets fall.
    fn tally(&mut self, header: &Header, tables: &[attributes::Table]) {
        for (&(what, why), &(n, at)) in &self.skipped {
            // `+ 0.0` writes a negative zero as 0; the file's easting is Y, its northing X.
            let reason = match at {
                Some(at) => format!("{why}; ilki Y {:.0}, X {:.0} yakınında", at.x + 0.0, at.y + 0.0),
                None => why.to_owned(),
            };
            self.report.skip_n(what, &reason, 0, n);
        }
        for (t, &n) in header.dropped.iter().enumerate() {
            self.report.skip_n(
                &type_words(t),
                "kayıt okunamadı: türü için kısa, koordinatı ±100 000 km dışında, metni ya da yüksekliği olmayan, alanı sıfır ya da tek noktalı",
                0,
                n as u32,
            );
        }
        for (t, &n) in header.unsupported.iter().enumerate() {
            self.report.skip_n(
                &format!("NCZ türü {t}"),
                "geometri türünü ne bu okuyucu ne başvuru okuyucuları tanıyor; içeriği okunmadı",
                0,
                n as u32,
            );
        }
        for (t, &n) in header.empty.iter().enumerate() {
            self.report.note_n(
                &format!("NCZ türü {t}"),
                "kayıt boş: başlığından sonrası sıfır, çizimde karşılığı yok",
                0,
                n as u32,
            );
        }
        if header.swept_entities > 0 {
            self.report.note_n(
                "NCZ 8 düzeni",
                &format!(
                    "nesne, eski NCZ okuyucularının atladığı {} bölümden okundu (ayarlar geometrinin arasında)",
                    header.swept_blocks
                ),
                0,
                header.swept_entities as u32,
            );
        }
        for (class, &n) in &self.drawn {
            let how = match class {
                SmartClass::Settlement => {
                    "sembolü çizildi: dairesi, nizamı, kat sayısı ve bahçe mesafeleri; değerleri dairenin öznitelikleri"
                }
                SmartClass::Construction => {
                    "sembolü çizildi: TAKS ve KAKS dairesi ya da Emsal, Hmax, Yençok satırları; değerleri ilk nesnesinin öznitelikleri"
                }
                SmartClass::Road => {
                    "sembolü çizildi: dairesi ve genişliği; değeri dairenin özniteliği"
                }
                SmartClass::PlanNote => {
                    "kutusu ve metni çizildi (RTF biçimlendirmesi kaldırıldı, metin kutuya sarıldı)"
                }
                SmartClass::FunctionName => "adı yazı olarak çizildi; değeri yazının özniteliği",
                SmartClass::None | SmartClass::Other => "sembolü çizildi",
            };
            self.report
                .note_n(&format!("Akıllı nesne ({})", class.name()), how, 0, n);
        }
        for (class, &n) in &self.undrawn {
            self.report.note_n(
                &format!("Akıllı nesne ({})", class.name()),
                "sembolü çizilemedi (sınıfı tanınmıyor ya da gösterecek değeri yok); yerinde nokta olarak alındı, değerleri öznitelik",
                0,
                n,
            );
        }
        self.report.note_n(
            "Akıllı nesne boyutu",
            "okunamadı; NCZ'nin varsayılanı olan 1 ile çizildi",
            0,
            self.unsized_objects,
        );
        self.report.note_n(
            "Akıllı nesne (nokta)",
            "okunabilir bir dikdörtgeni yok; yerinde nokta olarak alındı, adı öznitelik",
            0,
            header.point_smart_objects as u32,
        );
        self.report.note_n(
            "Izgara işareti (S0)",
            "akıllı nesnenin kendisi çizdiği için ayrıca alınmadı",
            0,
            header.smart_marks as u32,
        );
        if self.widths > 0 {
            self.report.note_n(
                "Çizgi kalınlığı",
                &format!(
                    "nesnenin kendi kalınlığıyla alındı ({:.2}–{:.2} mm); kalınlığı olmayanlar katmanınkiyle çizilir",
                    self.thinnest, self.widest
                ),
                0,
                self.widths,
            );
        }
        if let Ok(zone) = &self.zone {
            self.report.note_n(
                "Pafta çerçevesi",
                &format!("dosyanın bildirdiği {} sisteminde gerçek biçimiyle, dönük dörtgen olarak çizildi: dosya bir paftanın yalnız sınırlayıcı kutusunu saklar", zone.name),
                0,
                self.framed,
            );
        }
        for (why, &n) in &self.boxed {
            self.report.skip_n(
                "Pafta çerçevesinin gerçek biçimi",
                &format!(
                    "bulunamadı, pafta dosyanın sakladığı sınırlayıcı kutu olarak çizildi: {}",
                    why.reason()
                ),
                0,
                n,
            );
        }
        self.report.note_n(
            "Yay",
            "tam tur ya da daha geniş olduğu için daire olarak alındı",
            0,
            self.full_turns,
        );
        self.report.note_n(
            "Yazı",
            "dönüklüğü sayı değildi; yatay alındı",
            0,
            self.flat_rotation,
        );
        if !self.renamed.is_empty() {
            self.report.note_n(
                "Katman adı",
                "denetim karakterleri atıldı",
                0,
                self.renamed.len() as u32,
            );
        }
        if !tables.is_empty() {
            let rows: usize = tables.iter().map(|t| t.rows.len()).sum();
            let mut listed: Vec<String> = tables
                .iter()
                .take(6)
                .map(|t| format!("{}: {} satır", t.table_ref, t.rows.len()))
                .collect();
            if tables.len() > 6 {
                listed.push("…".into());
            }
            self.report.skip_n(
                "Öznitelik tablosu (@TAB)",
                &format!(
                    "{} tablo ({}); satırları dosyadaki bir nesneye bağlanmadığı için alınmadı",
                    tables.len(),
                    listed.join(", ")
                ),
                0,
                rows as u32,
            );
        }
        if self.truncated > 0 {
            self.report.skip_n(
                "Nesne sınırı",
                &format!("ilk {} nesne alındı; kalanlar alınmadı", self.limit),
                0,
                self.truncated,
            );
        }
    }

    /// The layers the objects landed on, in the order they are made in the
    /// drawing, whose first layer is drawn on top: the layers of smart objects
    /// first (the symbols read over the plan, as in its own program; a layer counts as
    /// one when most of its objects draw one), then the rest; each in the
    /// file's table order, then by name.
    fn layers(&self) -> Vec<ImportLayer> {
        let mut order: Vec<usize> = (0..self.layers.len())
            .filter(|&i| self.layers[i].count > 0)
            .collect();
        let symbols = |l: &Layer| 2 * l.smart >= l.count;
        let table: HashMap<&str, usize> = self
            .fin
            .layer_names
            .iter()
            .enumerate()
            .rev()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        order.sort_by(|&a, &b| {
            let (la, lb) = (&self.layers[a], &self.layers[b]);
            if symbols(la) != symbols(lb) {
                return symbols(lb).cmp(&symbols(la));
            }
            match (table.get(la.name.as_str()), table.get(lb.name.as_str())) {
                (Some(x), Some(y)) => x.cmp(y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => la.name.cmp(&lb.name),
            }
        });
        order
            .into_iter()
            .map(|i| {
                let l = &self.layers[i];
                ImportLayer {
                    line_pattern: None,
                    frozen: None,
                    plot: None,
                    name: l.name.clone(),
                    color: l.color.map_or_else(|| "ink".to_owned(), app_color),
                    visible: true,
                    locked: false,
                    line_type: LineType::Continuous,
                    line_weight: None,
                    count: l.count,
                    kinds: BTreeMap::new(),
                    bounds: None,
                    fields: Vec::new(),
                }
            })
            .collect()
    }
}

/// The values of a smart object as attributes: its class, and every value the
/// user entered and did not switch off, under the label the NCZ gives it. A
/// settlement symbol's three distances say which garden they are.
fn properties(e: &format::Entity) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    attrs.insert("Akıllı nesne".to_owned(), e.smart.name().to_owned());
    for p in &e.properties {
        let value = p.value.trim();
        if !p.user || p.null || value.is_empty() || p.name == "rtfData" {
            continue;
        }
        let mut key = if p.display.trim().is_empty() {
            p.name.clone()
        } else {
            p.display.trim().to_owned()
        };
        if e.smart == SmartClass::Settlement
            && matches!(p.name.as_str(), "txtOn" | "txtArka" | "txtYan")
        {
            key.push_str(" bahçe");
        }
        attrs.insert(key, value.to_owned());
    }
    attrs
}

/// `text` broken to `width` at the spaces, measured in the drawing's face;
/// paragraphs stay on their own lines, and a word longer than the width
/// stands alone on one.
/// The alignment a symbol's text anchor names (docs/adr/0145 §7).
fn align_of(anchor: Anchor) -> Option<TextAlign> {
    match anchor {
        Anchor::BaselineLeft => None,
        Anchor::MiddleLeft => Some(TextAlign::MiddleLeft),
        Anchor::MiddleCentre => Some(TextAlign::MiddleCenter),
        Anchor::MiddleRight => Some(TextAlign::MiddleRight),
        Anchor::TopLeft => Some(TextAlign::TopLeft),
    }
}

fn wrapped(text: &str, width: f64, height: f64, font: Font) -> Vec<String> {
    let mut out = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ').filter(|w| !w.is_empty()) {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && width_em(&candidate, font) * height > width {
                out.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = candidate;
            }
        }
        out.push(line);
    }
    out
}

/// Reads an NCZ drawing. `watch` hears how far the read is, in
/// thousandths (`PROGRESS_TOTAL`), and may stop it (`Err(STOPPED)`). A file
/// that holds no NCZ blocks at all is refused with the reason; one whose
/// blocks hold no geometry reads as empty, and the report says why.
/// The zoning legend's mark a Netcad zoning plan's symbol code stands for (plan/render.md RM-18, docs/adr/9218): the
/// Mekânsal Planlar Yapım Yönetmeliği area symbol of KentOS's system library (MPYY, uygulama imar planı) whose centroid
/// mark it is, drawn at the point (a fill symbol on a point draws its centroid marks there), and the attribute that
/// mark reads (a trade area's level: T1, T2).
///
/// Read from the Sivas plan: each code stands inside one kind of area (S75 in 63 kindergarten areas, S96 in the 277
/// mosque areas…), and its drawing in Netcad's own DXF is that legend's mark (ANA in a triangle, a dome). Netcad's
/// drawings are not copied: the marks are KentOS's own of the regulation's legend. Codes without one stay a point with
/// their code: S101 (2 141 points in parks, which Netcad draws empty), the private institutions' Ö marks (S180, S181,
/// S185), which the legend has no row for, and S15 (one point).
fn legend_mark(code: &str) -> Option<(&'static str, Option<(&'static str, &'static str)>)> {
    let symbol = match code {
        // A trade area's mark reads its level: T1, T2.
        "S6" => return Some(("mpyy.uip.calisma.ticaret-alani", Some(("Kademe", "1")))),
        "S7" => return Some(("mpyy.uip.calisma.ticaret-alani", Some(("Kademe", "2")))),
        "S1" => "mpyy.uip.ortak-ozel-kanun.teknoloji-gelistirme-bolgesi",
        "S9" => "mpyy.uip.calisma.ticaret-konut-alani",
        "S10" => "mpyy.uip.calisma.ticaret-turizm-alani",
        "S12" => "mpyy.uip.calisma.belediye-hizmet-alani",
        "S13" => "mpyy.uip.calisma.idari-hizmet-alani",
        "S14" => "mpyy.uip.calisma.resmi-kurum-alani",
        "S16" => "mpyy.uip.calisma.akaryakit-ve-servis-istasyonu-alani",
        "S17" => "mpyy.uip.calisma.kucuk-sanayi-alani",
        "S18" => "mpyy.uip.calisma.depolama-alani",
        "S20" => "mpyy.uip.calisma.pazar-alani",
        "S21" => "mpyy.uip.calisma.askeri-alan",
        "S22" => "mpyy.uip.calisma.beton-santrali",
        "S25" => "mpyy.uip.turizm.otel-alani",
        "S32" => "mpyy.uip.turizm.gunubirlik-tesis-alani",
        "S59" => "mpyy.uip.bugunku.tarimsal-nitelikli-alan",
        "S75" => "mpyy.uip.egitim.anaokulu-alani",
        "S76" => "mpyy.uip.egitim.ilkokul-alani",
        "S77" => "mpyy.uip.egitim.ortaokul-alani",
        "S78" => "mpyy.uip.egitim.lise-alani",
        "S79" => "mpyy.uip.egitim.ozel-egitim-alani",
        "S80" => "mpyy.uip.egitim.mesleki-ve-teknik-ogretim-tesisi-alani",
        "S81" => "mpyy.uip.egitim.yuksek-ogretim-tesisi-alani",
        "S82" => "mpyy.uip.egitim.halk-egitim-merkezi",
        "S83" => "mpyy.uip.saglik.saglik-tesisi-alani",
        "S84" => "mpyy.uip.saglik.ozel-saglik-tesisi-alani",
        "S86" => "mpyy.uip.saglik.aile-sagligi-merkezi",
        "S87" => "mpyy.uip.sosyal-kulturel.sosyal-tesis-alani",
        "S88" => "mpyy.uip.sosyal-kulturel.kulturel-tesis-alani",
        "S90" => "mpyy.uip.sosyal-kulturel.acik-spor-tesisi-alani",
        "S91" => "mpyy.uip.sosyal-kulturel.kapali-spor-tesisi-alani",
        "S96" => "mpyy.uip.ibadet.cami",
        "S103" => "mpyy.uip.acik-yesil.rekreasyon-alani",
        "S108" => "mpyy.uip.acik-yesil.meydan",
        "S113" => "mpyy.uip.acik-yesil.mezarlik-alani",
        "S114" => "mpyy.uip.acik-yesil.korunacak-bahce",
        "S118" => "mpyy.uip.karayollari.genel-otopark-alani",
        "S122" => "mpyy.uip.ortak-karayollari.terminal-otogar",
        "S146" => "mpyy.uip.enerji.yenilenebilir-enerji-kaynaklarina-dayali-uretim-tesisi-alani",
        "S152" => "mpyy.uip.enerji.trafo-alani",
        "S153" => "mpyy.uip.karayollari.elektronik-haberlesme-altyapi-alani",
        "S155" => "mpyy.uip.ortak-atik.su-kaynaklari-toplama-yeri-kaptaj-alani",
        "S160" => "mpyy.uip.ortak-atik.teknik-altyapi-alani",
        "S164" => "mpyy.uip.calisma.ticaret-turizm-konut-alani",
        "S165" => "mpyy.uip.calisma.toplu-isyerleri",
        "S175" => "mpyy.uip.calisma.ticaret-alani",
        _ => return None,
    };
    Some((symbol, None))
}

/// The height Netcad writes a parcel's number at from the parcel's label, metres: what its own DXF of the Sivas plan
/// shows at the 1:1000 an NCZ opens at (docs/adr/9217).
const PARCEL_NUMBER_HEIGHT: f64 = 3.0;

/// The fewest `ada/parsel` labelled areas that make a layer a cadastre's (docs/adr/9217).
const LEAST_PARCELS: usize = 10;

/// Whether a label is a parcel's `ada/parsel`: the layers whose areas mostly carry one are the cadastre's. A plan's
/// areas carry labels too (`70`, `İRT_A952`, `OA_2_1`), and Netcad writes none of them on the plan.
fn block_and_parcel(label: &str) -> bool {
    label.trim().split_once('/').is_some_and(|(ada, _)| !ada.is_empty() && ada.chars().all(|c| c.is_ascii_digit()))
        && parcel_number(label).is_some()
}

/// A parcel's number as Netcad writes it on the plan from the parcel's label: the parcel's part of `ada/parsel`
/// (`1588/10` → `10`, `103/TH4` → `TH4`), a label without an ada whole (`6587`); none for words (`TESCİL HARİCİ`).
fn parcel_number(label: &str) -> Option<&str> {
    let label = label.trim();
    let number = match label.split_once('/') {
        Some((ada, parsel)) if !ada.is_empty() && ada.chars().all(|c| c.is_ascii_digit()) => parsel,
        Some(_) => return None,
        None => label,
    };
    (!number.is_empty()
        && number.chars().count() <= 16
        && !number.chars().any(char::is_whitespace)
        && number.chars().any(|c| c.is_ascii_digit()))
    .then_some(number)
}

/// A ring's area centroid (its first point the origin of the sums, for precision at map coordinates).
fn area_centroid(pts: &[Vec2]) -> Option<Vec2> {
    let o = *pts.first()?;
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..pts.len() {
        let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
        let (px, py, qx, qy) = (p.x - o.x, p.y - o.y, q.x - o.x, q.y - o.y);
        let c = px * qy - qx * py;
        a += c;
        cx += (px + qx) * c;
        cy += (py + qy) * c;
    }
    (a.abs() > 1e-12 && a.is_finite()).then(|| Vec2 { x: o.x + cx / (3.0 * a), y: o.y + cy / (3.0 * a) })
}

/// Whether `p` is inside the ring (even-odd).
fn in_ring(pts: &[Vec2], p: Vec2) -> bool {
    let mut odd = false;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            odd = !odd;
        }
    }
    odd
}

/// The parcel numbers Netcad shows from the parcels' labels that no text shows (plan/render.md RM-18, docs/adr/9216
/// §4, 9217); how many parcels a text of their layer numbers already, and how many were left without one.
///
/// Netcad writes each parcel's number at the area centroid of the parcel, centred, 3 m high, whether or not a text of
/// its own numbers it: the Sivas plan's DXF shows the number twice on most of its parcels. The owner's choice (24
/// November 2026): only the parcels no text numbers get it. A layer is a cadastre's when at least [`LEAST_PARCELS`] of
/// its areas, and most of its labelled ones, carry an `ada/parsel` label.
///
/// Only a parcel whose centroid lies in it alone gets it. Where parcels lie over one another (an old parcel under its
/// divisions), Netcad does not write the label's number: on 183 of the Sivas plan's 1 199 such parcels it writes another
/// one, from a field this reader cannot tie to the parcel (docs/adr/0138). A number left out is better than a wrong one.
fn parcel_numbers(entities: &[Entity]) -> (Vec<Entity>, usize, usize) {
    struct Area<'a> {
        ring: &'a [Vec2],
        bounds: [f64; 4],
        /// Its label and the number it writes, for a parcel's label.
        parcel: Option<(&'a str, &'a str)>,
        shown: bool,
    }
    // Each layer's areas, its labelled ones and how many of those carry `ada/parsel`.
    let mut layers: BTreeMap<&str, (Vec<Area<'_>>, usize, usize)> = BTreeMap::new();
    for e in entities {
        let Entity::Polygon(p) = e else { continue };
        if p.pts.len() < 3 {
            continue;
        }
        let label = p.base.attrs.get("Etiket");
        let entry = layers.entry(p.base.layer_id.as_str()).or_default();
        entry.1 += usize::from(label.is_some());
        entry.2 += usize::from(label.is_some_and(|l| block_and_parcel(l)));
        let bounds = p.pts.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, q| {
            [b[0].min(q.x), b[1].min(q.y), b[2].max(q.x), b[3].max(q.y)]
        });
        let parcel = label.and_then(|l| parcel_number(l).map(|n| (l.trim(), n)));
        entry.0.push(Area { ring: &p.pts, bounds, parcel, shown: false });
    }
    layers.retain(|_, (_, labelled, slashed)| *slashed >= LEAST_PARCELS && 2 * *slashed >= *labelled);
    if layers.is_empty() {
        return (Vec::new(), 0, 0);
    }
    // The areas by the 64 m cells their boxes cover; a point is looked for in its own cell's.
    const CELL: f64 = 64.0;
    let cell = |v: f64| (v / CELL).floor() as i64;
    let mut grids: HashMap<&str, HashMap<(i64, i64), Vec<usize>>> = HashMap::new();
    for (layer, (areas, _, _)) in &layers {
        let grid = grids.entry(layer).or_default();
        for (i, a) in areas.iter().enumerate() {
            for x in cell(a.bounds[0])..=cell(a.bounds[2]) {
                for y in cell(a.bounds[1])..=cell(a.bounds[3]) {
                    grid.entry((x, y)).or_default().push(i);
                }
            }
        }
    }
    let inside = |a: &Area<'_>, p: Vec2| {
        (a.bounds[0]..=a.bounds[2]).contains(&p.x) && (a.bounds[1]..=a.bounds[3]).contains(&p.y) && in_ring(a.ring, p)
    };
    for e in entities {
        let Entity::Text(t) = e else { continue };
        let layer = t.base.layer_id.as_str();
        let (Some((areas, _, _)), Some(grid)) = (layers.get_mut(layer), grids.get(layer)) else { continue };
        let text = t.text.trim();
        for &i in grid.get(&(cell(t.p.x), cell(t.p.y))).map_or(&[][..], Vec::as_slice) {
            let a = &areas[i];
            if !a.shown && a.parcel.is_some_and(|(label, number)| text == number || text == label) && inside(a, t.p) {
                areas[i].shown = true;
            }
        }
    }
    let mut out = Vec::new();
    let (mut shown, mut shared) = (0, 0);
    for (layer, (areas, _, _)) in &layers {
        let grid = &grids[layer];
        for (i, a) in areas.iter().enumerate() {
            let Some((_, number)) = a.parcel else { continue };
            if a.shown {
                shown += 1;
                continue;
            }
            let Some(at) = area_centroid(a.ring) else { continue };
            let alone = grid
                .get(&(cell(at.x), cell(at.y)))
                .is_some_and(|near| near.iter().filter(|&&j| inside(&areas[j], at)).eq([&i]));
            if !alone {
                shared += 1;
                continue;
            }
            out.push(Entity::Text(TextEntity {
                annotative: None,
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: (*layer).to_owned(),
                    color: None,
                    attrs: BTreeMap::new(),
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
                p: at,
                text: number.to_owned(),
                height: PARCEL_NUMBER_HEIGHT,
                rotation: 0.0,
                align: Some(TextAlign::MiddleCenter),
                width_factor: None,
                mask: false,
                label_of: None,
                label_scale: None,
                paragraph: Default::default(),
                face: Default::default(),
                path: None,
            }));
        }
    }
    (out, shown, shared)
}

pub fn read(
    bytes: &[u8],
    opts: &NczReadOptions,
    watch: &mut dyn Watch,
) -> Result<ImportResult, String> {
    if bytes.starts_with(b"KCAD") {
        return Err("Bu bir KentOS çizimi (.kcad); içe aktarılmaz, Dosya → Aç ile açılır.".into());
    }
    let mut em = Emitter::new(opts);
    let mut header = Header::default();
    match format::read(bytes, &mut em, &mut header, watch) {
        Outcome::Complete => {}
        Outcome::Cancelled | Outcome::Stopped => return Err(STOPPED.into()),
    }
    let tables = attributes::tables(bytes, watch, 950, 50).ok_or_else(|| STOPPED.to_owned())?;
    let fin = em.fin.clone();
    let recognised = !fin.layer_names.is_empty()
        || fin.mproj
        || !fin.version_name.is_empty()
        || header.dropped.iter().any(|&n| n > 0)
        || header.unsupported.iter().any(|&n| n > 0)
        || !em.out.is_empty();
    if !recognised {
        return Err("Bu dosyada NCZ çizimi bulunamadı: katman tablosu, sürüm ya da geometri kaydı yok. Dosyanın bir .ncz olduğunu denetleyin; kendi programında açılıyorsa DXF olarak kaydedip onu deneyin.".into());
    }
    em.tally(&header, &tables);
    let mut report = std::mem::take(&mut em.report);
    report.fact(
        "Sürüm",
        version(&fin).map_or_else(|| "belirtilmemiş".to_owned(), |v| format!("NCZ {v}")),
    );
    report.fact("Karakter kodlaması", "Windows-1254 (Türkçe)");
    if let Some(d) = crs::declared(&fin) {
        report.fact("Koordinat sistemi (dosya)", d.text.clone());
    }
    let layers = em.layers();
    let mut entities = std::mem::take(&mut em.out);
    let (numbers, shown, shared) = parcel_numbers(&entities);
    if !numbers.is_empty() || shared > 0 {
        report.about(
            "Parsel numarası",
            &format!(
                "{} parselin numarası yalnız alanın etiketindeydi; Netcad'in gösterdiği gibi alanın ağırlık merkezine \
                 {} m yüksekliğinde yazıldı. Numarası bir yazıyla zaten görünen {} parsele dokunulmadı. Ağırlık \
                 merkezi başka bir parselin içinde de kalan {} parsele yazılmadı: üst üste parsellerde Netcad'in \
                 gösterdiği numara etiketten değil, bu dosyada parsele bağlanamayan bir alandan geliyor.",
                numbers.len(),
                PARCEL_NUMBER_HEIGHT,
                shown,
                shared
            ),
        );
    }
    entities.extend(numbers);
    let (marked, unmarked) = std::mem::take(&mut em.legend);
    if marked > 0 || !unmarked.is_empty() {
        let left: u32 = unmarked.values().sum();
        let codes: Vec<String> = unmarked.iter().map(|(code, n)| format!("{code} ({n})")).collect();
        report.about(
            "Plan sembolü",
            &format!(
                "{marked} sembol noktası Mekânsal Planlar Yapım Yönetmeliği lejantının işaretiyle çizildi (KentOS'un \
                 MPYY sembolleri). {left} nokta yalnız kodunu taşıyor; lejantta karşılığı olmayan kodlar: {}.",
                if codes.is_empty() { "yok".to_owned() } else { codes.join(", ") }
            ),
        );
    }
    for e in &mut entities {
        e.base_mut().id = 0;
    }
    let mut result = ImportResult {
        layouts: Vec::new(),
        source_handles: Vec::new(),
        line_type_scale: None,
        line_types: Vec::new(),
        entities,
        layers,
        report: report.import(),
        bounds: em.bounds,
        declared_crs: crs::declared(&fin),
        view: None,
        blocks: Vec::new(),
        text_styles: Vec::new(),
        dimension_styles: Vec::new(),
        table_styles: Vec::new(),
        unit: None,
    };
    kentos_formats::import::summarise(&mut result);
    let _ = watch.step(crate::PROGRESS_TOTAL, crate::PROGRESS_TOTAL);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(layer: &str, label: Option<&str>) -> EntityBase {
        EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer.into(),
            color: None,
            attrs: label.map(|l| BTreeMap::from([("Etiket".to_owned(), l.to_owned())])).unwrap_or_default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        }
    }

    /// A 20 m square parcel at `x`, labelled.
    fn parcel(layer: &str, x: f64, label: &str) -> Entity {
        let pts = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)].map(|(a, b)| Vec2 { x: x + a, y: b });
        Entity::Polygon(PathEntity {
            space_separator: None,
            ms: None,
            base: base(layer, Some(label)),
            pts: pts.to_vec(),
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
            closed: true,
            widths: None,
        })
    }

    fn text(layer: &str, x: f64, y: f64, s: &str) -> Entity {
        Entity::Text(TextEntity {
            annotative: None,
            base: base(layer, None),
            p: Vec2 { x, y },
            text: s.into(),
            height: 2.5,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
            path: None,
        })
    }

    /// Every legend mark a code stands for is a symbol of KentOS's system library, an area symbol of the uygulama
    /// imar planı with a centroid mark (docs/adr/9218); a trade area's level is read by its mark.
    #[test]
    fn the_legend_marks_are_the_system_librarys_area_symbols() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../native/style/assets/system-library.kstil");
        let library: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).expect("the library")).expect("json");
        let items = library["items"].as_array().expect("items");
        let mut found = 0;
        for n in 0..=200 {
            let code = format!("S{n}");
            let Some((id, read)) = legend_mark(&code) else { continue };
            found += 1;
            let item = items.iter().find(|i| i["id"] == id).unwrap_or_else(|| panic!("{code}: {id} is not in the library"));
            let layers = item["symbol"]["layers"].as_array().expect("layers");
            assert_eq!(item["symbol"]["type"], "fill", "{code}");
            assert!(layers.iter().any(|l| l["type"] == "centroidMarker"), "{code}: {id} has a centroid mark");
            if let Some((key, _)) = read {
                assert!(item.to_string().contains(&format!("[{key}]")), "{code}: its mark reads {key}");
            }
        }
        assert_eq!(found, 47);
        assert_eq!(legend_mark("S6"), Some(("mpyy.uip.calisma.ticaret-alani", Some(("Kademe", "1")))));
        assert_eq!(legend_mark("S75"), Some(("mpyy.uip.egitim.anaokulu-alani", None)));
        assert_eq!(legend_mark("S101"), None, "Netcad draws a park's mark empty");
        assert_eq!(legend_mark("S180"), None, "a private institution's Ö mark has no row in the legend");
    }

    #[test]
    fn a_parcels_number_is_the_part_after_its_block() {
        assert_eq!(parcel_number("1588/10"), Some("10"));
        assert_eq!(parcel_number(" 103/TH4 "), Some("TH4"));
        assert_eq!(parcel_number("6587"), Some("6587"));
        assert_eq!(parcel_number("TESCİL HARİCİ"), None);
        assert_eq!(parcel_number("KORUNMA"), None);
        assert_eq!(parcel_number("A/5"), None, "a block is a number");
        assert_eq!(parcel_number("12/"), None);
        assert!(block_and_parcel("1588/10") && !block_and_parcel("6587") && !block_and_parcel("OA_2_1"));
    }

    /// Of twelve parcels, the eight a text of their layer numbers keep only it; the four others get their number at
    /// their centroid, centred, 3 m high. A layer of a few labelled areas, or of words, gets none.
    #[test]
    fn only_the_parcels_no_text_numbers_get_their_label_written() {
        let mut all = Vec::new();
        for i in 0..12 {
            let x = f64::from(i) * 30.0;
            all.push(parcel("KADASTRO", x, &format!("101/{i}")));
            if i % 3 != 0 {
                // The number inside, as Netcad's own texts are; one of them the whole label.
                all.push(text("KADASTRO", x + 8.0, 9.0, &if i == 1 { "101/1".to_owned() } else { i.to_string() }));
            }
        }
        // The number of another parcel inside this one does not number it; nor does a text of another layer.
        all.push(text("KADASTRO", 3.0 * 30.0 + 5.0, 5.0, "7"));
        all.push(text("YAZI", 6.0 * 30.0 + 5.0, 5.0, "6"));
        for i in 0..5 {
            all.push(parcel("AZ", 1000.0 + f64::from(i) * 30.0, &format!("5/{i}")));
        }
        for i in 0..12 {
            all.push(parcel("PLAN", 2000.0 + f64::from(i) * 30.0, "KONUT"));
            // A plan's areas numbered without a block: Netcad writes none of them.
            all.push(parcel("PL_KONUT", 3000.0 + f64::from(i) * 30.0, &(70 + i).to_string()));
        }
        // A parcel under another (its centroid in both): not written, whatever the label says.
        all.push(parcel("KADASTRO", 400.0, "101/40"));
        all.push(parcel("KADASTRO", 405.0, "101/41"));
        let (numbers, shown, shared) = parcel_numbers(&all);
        assert_eq!((shown, shared), (8, 2));
        let written: Vec<(&str, f64, f64)> = numbers
            .iter()
            .map(|e| match e {
                Entity::Text(t) => {
                    assert_eq!((t.height, t.align, t.base.layer_id.as_str()), (3.0, Some(TextAlign::MiddleCenter), "KADASTRO"));
                    (t.text.as_str(), t.p.x, t.p.y)
                }
                _ => panic!("a text"),
            })
            .collect();
        assert_eq!(written, [("0", 10.0, 10.0), ("3", 100.0, 10.0), ("6", 190.0, 10.0), ("9", 280.0, 10.0)]);
    }

    #[test]
    fn black_and_white_are_ink() {
        assert_eq!(app_color(0xFF00_0000), "ink");
        assert_eq!(app_color(0xFFFF_FFFF), "ink");
        assert_eq!(app_color(0xFF10_1410), "ink", "all but black");
        assert_eq!(app_color(0xFF00_0100), "ink");
        assert_eq!(app_color(0xFFF5_F5F5), "ink", "all but white");
        assert_eq!(app_color(0xFF40_4040), "#404040", "a grey stays");
        assert_eq!(app_color(0xFF00_00FF), "#0000FF");
    }

    #[test]
    fn a_version_is_a_build_number() {
        let h = |v: &str| Header {
            version_name: v.into(),
            ..Header::default()
        };
        assert_eq!(version(&h("8.5.6.1095")), Some("8.5.6.1095"));
        assert_eq!(version(&h("5.2.0.1035N")), Some("5.2.0.1035N"));
        assert_eq!(version(&h("À÷PA,²A@TAB23")), None);
    }

    #[test]
    fn a_symbol_turns_and_scales_about_its_anchor() {
        let p = Place::new(Vec2 { x: 100.0, y: 50.0 }, 0.5, 90.0);
        assert_eq!(p.apply((10.0, 0.0)), Vec2 { x: 100.0, y: 55.0 });
        assert_eq!(p.apply((0.0, 10.0)), Vec2 { x: 95.0, y: 50.0 });
    }

    /// The city plan NCZ lost a whole turn whose ends meet (plan/render.md RM-17); what still falls says why and where.
    #[test]
    fn a_full_turn_whose_ends_meet_is_a_circle_and_what_falls_says_where() {
        use crate::format::Sink;
        let mut em = Emitter::new(&NczReadOptions::default());
        let arc = |x: f64, y: f64, r: f64, a0: f64, a1: f64| format::Entity {
            radius: r,
            start_angle: a0,
            end_angle: a1,
            ..record(Kind::Arc, &[(x, y, 0.0)])
        };
        let circle = |x: f64, y: f64, r: f64| format::Entity { radius: r, ..record(Kind::Circle, &[(x, y, 0.0)]) };
        let pi = std::f64::consts::PI;
        for e in [
            // A whole turn short of a hair, in radians: its ends meet.
            arc(100.0, 200.0, 5.0, -pi, pi - 1e-13),
            // Next to no sweep: no arc.
            arc(300.0, 400.0, 5.0, 1.0, 1.0 + 1e-13),
            // A radius of nothing; two that are no number.
            arc(500.0, 600.0, 0.0, 0.0, 1.0),
            circle(700.0, 800.0, f64::NAN),
            circle(701.0, 801.0, f64::NAN),
        ] {
            em.entity(&e);
        }
        assert!(
            matches!(em.out.as_slice(), [Entity::Circle(c)] if c.r == 5.0 && c.c == Vec2 { x: 100.0, y: 200.0 }),
            "{:?}",
            em.out
        );
        em.tally(&Header::default(), &[]);
        let report = std::mem::take(&mut em.report).import();
        let skipped: Vec<(&str, u32, &str)> =
            report.skipped.iter().map(|i| (i.what.as_str(), i.count, i.reason.as_str())).collect();
        for want in [
            ("Yay", 1, "açıklığı boş; ilki Y 300, X 400 yakınında"),
            ("Yay", 1, "yarıçapı sıfır ya da eksi; ilki Y 500, X 600 yakınında"),
            ("Daire", 2, "yarıçapı sayı değil; ilki Y 700, X 800 yakınında"),
        ] {
            assert!(skipped.contains(&want), "{want:?} in {skipped:?}");
        }
        assert!(report.notes.iter().any(|i| i.what == "Yay"
            && i.count == 1
            && i.reason == "tam tur ya da daha geniş olduğu için daire olarak alındı"));
    }

    /// A record of `kind` on one layer whose vertices the file gave (x easting, y northing, z).
    fn record(kind: Kind, coords: &[(f64, f64, f64)]) -> format::Entity {
        format::Entity {
            kind,
            layer_name: "KOT".into(),
            coords: coords
                .iter()
                .map(|&(x, y, z)| format::Coord { x, y, z })
                .collect(),
            ..format::Entity::default()
        }
    }

    #[test]
    fn heights_come_from_the_vertices_the_file_gave_and_from_nothing_else() {
        use crate::format::Sink;
        let mut em = Emitter::new(&NczReadOptions::default());
        let ring = |z: [f64; 4]| {
            [
                (0.0, 0.0, z[0]),
                (10.0, 0.0, z[1]),
                (10.0, 10.0, z[2]),
                (0.0, 10.0, z[3]),
                (0.0, 0.0, z[0]),
            ]
        };
        for e in [
            // 0-2: lines: both ends, a 2D one (Z 0 is no elevation), one end at 0 with the other above it
            record(Kind::Line, &[(1.0, 2.0, 105.5), (3.0, 4.0, 107.25)]),
            record(Kind::Line, &[(1.0, 2.0, 0.0), (3.0, 4.0, 0.0)]),
            record(Kind::Line, &[(1.0, 2.0, 0.0), (3.0, 4.0, 12.5)]),
            // 3-5: polylines: a 0 among the heights is a height; all 0; a Z that is not a number leaves its vertex without
            record(
                Kind::Polyline,
                &[(0.0, 0.0, 10.0), (10.0, 0.0, 0.0), (10.0, 10.0, 12.5)],
            ),
            record(Kind::Polyline, &[(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)]),
            record(
                Kind::Polyline,
                &[(0.0, 0.0, 10.0), (10.0, 0.0, f64::NAN), (10.0, 10.0, -2.5)],
            ),
            // 6-7: areas drawn point by point: the closing vertex goes with its height; all 0
            record(Kind::Polygon, &ring([1.0, 2.0, 3.0, 4.0])),
            record(Kind::Polygon, &ring([0.0; 4])),
            // 8-10: corners the reader made up (a sheet's, a rectangle's) or knows one of (a triangle's) have no height to give
            record(Kind::MapSheet, &ring([5.0, 5.0, 5.0, 5.0])),
            record(
                Kind::Triangle,
                &[(0.0, 0.0, 5.0), (10.0, 0.0, 0.0), (0.0, 10.0, 0.0)],
            ),
            record(Kind::SmartObject, &ring([7.0, 7.0, 7.0, 7.0])),
        ] {
            assert!(em.entity(&e));
        }
        let report = std::mem::take(&mut em.report).import();
        assert!(report.skipped.is_empty());
        let out = &em.out;
        let Entity::Line(l) = &out[0] else {
            panic!("{:?}", out[0])
        };
        assert_eq!((l.za, l.zb), (Some(105.5), Some(107.25)));
        for i in [1, 2] {
            let Entity::Line(l) = &out[i] else {
                panic!("{:?}", out[i])
            };
            assert_eq!(
                (l.za, l.zb),
                if i == 1 {
                    (None, None)
                } else {
                    (Some(0.0), Some(12.5))
                }
            );
        }
        let zs = |i: usize| match &out[i] {
            Entity::Polyline(p) | Entity::Polygon(p) => p.zs.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(zs(3), Some(vec![Some(10.0), Some(0.0), Some(12.5)]));
        assert_eq!(zs(4), None);
        assert_eq!(zs(5), Some(vec![Some(10.0), None, Some(-2.5)]));
        assert!(matches!(out[6], Entity::Polygon(_)));
        assert_eq!(
            zs(6),
            Some(vec![Some(1.0), Some(2.0), Some(3.0), Some(4.0)])
        );
        assert_eq!(zs(7), None);
        assert_eq!((zs(8), zs(9), zs(10)), (None, None, None));
        // The heights are counted where every import says it: as a fact of the file.
        let mut result = ImportResult {
        layouts: Vec::new(),
        source_handles: Vec::new(),
            line_type_scale: None,
            line_types: Vec::new(),
            entities: out.clone(),
            layers: Vec::new(),
            report,
            bounds: None,
            declared_crs: None,
            view: None,
            blocks: Vec::new(),
            text_styles: Vec::new(),
            dimension_styles: Vec::new(),
            table_styles: Vec::new(),
            unit: None,
        };
        kentos_formats::import::summarise(&mut result);
        assert!(
            result
                .report
                .source
                .iter()
                .any(|f| (f.label.as_str(), f.value.as_str()) == ("Kotlu nesne", "5")),
            "{:?}",
            result.report.source
        );
    }

    #[test]
    fn a_note_wraps_at_its_width() {
        let lines = wrapped("bir iki üç dört beş", 3.0, 1.0, Font::DEFAULT);
        assert!(lines.len() > 1);
        assert_eq!(lines.join(" "), "bir iki üç dört beş");
        assert_eq!(wrapped("a\n\nb", 100.0, 1.0, Font::DEFAULT), ["a", "", "b"]);
    }
}
