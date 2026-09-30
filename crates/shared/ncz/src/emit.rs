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
//! | Netcad 8 smart object | its symbol: circles, lines and texts, turned and scaled as the object; its values as attributes of the symbol's first object |
//! | other smart objects | a point (no rectangle) or an area, the token as an attribute |
//!
//! A point keeps its height, and a line, a polyline and an area the height of
//! each vertex (docs/adr/0142), when the record has any (`heights`): a record
//! whose Z is 0 everywhere is a 2D one, and the vertices the reader made up
//! (a rectangle's, a sheet's, a triangle's two others) have none to give.
//!
//! A record's own colour is kept only where it is not its layer's; black and
//! white are the app's `ink`, which is black on paper and white on a dark
//! drawing, as DXF colour 7 is. The texts Netcad writes on an area (a parcel
//! number, an id) are not drawn by Netcad and are not labels here either:
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
/// and what is all but black or white, the app's `ink`. Netcad draws a plan
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
/// so is what Netcad writes for a vertex it has no height for); a Z that is not a
/// number is none. Only a record whose vertices the file gave (a line, a
/// polyline, an area drawn point by point) says what its heights are.
fn heights(coords: &[format::Coord]) -> Option<Vec<Option<f64>>> {
    let zs: Vec<Option<f64>> = coords.iter().map(|c| c.z.is_finite().then_some(c.z)).collect();
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
                // gives it (docs/adr/0139). Zero, a negative width (Netcad's own DXF export
                // writes those as the thinnest line) or one past 100 mm is the layer's.
                if e.line_width.is_finite() && e.line_width > 0.0 && e.line_width <= 1000.0 {
                    let mm = e.line_width / 10.0;
                    self.widths += 1;
                    self.widest = self.widest.max(mm);
                    self.thinnest = if self.widths == 1 { mm } else { self.thinnest.min(mm) };
                    // Every object the record made: a smart object's symbol is drawn in its pen.
                    for made in &mut self.out[first..] {
                        made.base_mut().line_weight = Some(mm);
                    }
                }
            }
            Err(why) => self.report.skip(kind_words(e.kind), why, 0),
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
            id: 0,
            layer_id: l.name.clone(),
            color: own.filter(|c| *c != layers),
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: None,
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
        let finite = |c: &format::Coord| c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD;
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
                    Kind::Symbol => note(&mut b, "Sembol", &e.label),
                    _ => note(&mut b, "Blok", &e.label),
                }
                self.grow(v(c));
                self.push(layer, Entity::Point(PointEntity { base: b, p: v(c), z }));
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
                self.push(layer, Entity::Line(LineEntity { base: b, a: v(a), b: v(z), za, zb }));
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
                self.push(layer, Entity::Polyline(PathEntity { base: b, pts, bulges: None, holes: None, zs, parts: None }));
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
                    self.push_smart(layer, Entity::Point(PointEntity { base: b, p: v(c), z: None }));
                } else {
                    self.area(e, layer, b)?;
                    self.layers[layer].smart += 1;
                }
            }
            Kind::Circle => {
                let c = e.coords.first().filter(|c| finite(c)).ok_or(BAD)?;
                if !(e.radius.is_finite() && e.radius > 0.0 && e.radius <= WORLD) {
                    return Err("yarıçapı sayı değil ya da sıfır");
                }
                self.grow(v(c));
                self.push(layer, Entity::Circle(CircleEntity { base: b, c: v(c), r: e.radius }));
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
                        base: b,
                        p: v(c),
                        text: e.label.clone(),
                        height: e.text_height,
                        rotation,
                        align: None,
                        width_factor: None,
                        mask: false,
                    }),
                );
            }
        }
        Ok(())
    }

    /// A pafta: the file keeps the box of its cell in the file's zone, and the cell is a turned
    /// quadrilateral there; its four corners when the file names the zone and the box is a
    /// grid cell's, the box otherwise (`sheet`).
    fn sheet(&mut self, e: &format::Entity, layer: usize, b: EntityBase) -> Result<(), &'static str> {
        let (xs, ys) = (e.coords.iter().map(|c| c.x), e.coords.iter().map(|c| c.y));
        let target = [xs.clone().fold(f64::INFINITY, f64::min), ys.clone().fold(f64::INFINITY, f64::min), xs.fold(f64::NEG_INFINITY, f64::max), ys.fold(f64::NEG_INFINITY, f64::max)];
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
                self.push(layer, Entity::Polygon(PathEntity { base: b, pts, bulges: None, holes: None, zs: None, parts: None }));
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
    fn area(&mut self, e: &format::Entity, layer: usize, b: EntityBase) -> Result<(), &'static str> {
        let finite = |c: &format::Coord| c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD;
        if !e.coords.iter().all(finite) {
            return Err("koordinatı sayı değil ya da ±100 000 km dışında");
        }
        let mut pts: Vec<Vec2> = e.coords.iter().map(|c| Vec2 { x: c.x, y: c.y }).collect();
        // Only a ring the file drew point by point has heights; a rectangle's and a sheet's corners are made up here.
        let mut zs = if e.kind == Kind::Polygon { heights(&e.coords) } else { None };
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
        self.push(layer, Entity::Polygon(PathEntity { base: b, pts, bulges: None, holes: None, zs, parts: None }));
        Ok(())
    }

    /// The arc the plugin draws (`_approximate_arc`): the angles in degrees
    /// when either is past a full turn in radians, radians otherwise; the end
    /// brought past the start a turn at a time; nothing when the sweep is
    /// empty, over ten turns, or ends where it began (its chord check). More
    /// than a whole turn that does not close covers the circle: a circle.
    fn arc(&mut self, e: &format::Entity, layer: usize, b: EntityBase) -> Result<(), &'static str> {
        let c = e
            .coords
            .first()
            .filter(|c| c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD)
            .ok_or("koordinatı sayı değil ya da ±100 000 km dışında")?;
        let (r, start, end) = (e.radius, e.start_angle, e.end_angle);
        if !(r.is_finite() && r > 0.0 && r <= WORLD && start.is_finite() && end.is_finite()) {
            return Err("yarıçapı ya da açıları sayı değil");
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
        if sweep <= 0.0 || sweep > 3600.0 {
            return Err("yay açıklığı boş ya da on turdan fazla");
        }
        // The plugin's last point is `start + sweep * steps / steps`, and a
        // chord under a nanometre between its ends is no arc to it.
        let steps = ((48.0 * sweep.max(1.0)) / 360.0).floor().max(8.0);
        let last = s_deg + sweep * steps / steps;
        let (a, z) = (rad(s_deg), rad(last));
        let chord = hypot(cos(z) * r - cos(a) * r, sin(z) * r - sin(a) * r);
        if chord < 1e-9 {
            return Err("yayın iki ucu aynı noktada (tam tur)");
        }
        let centre = Vec2 { x: c.x, y: c.y };
        for p in [
            Vec2 { x: c.x - r, y: c.y - r },
            Vec2 { x: c.x + r, y: c.y + r },
        ] {
            self.grow(p);
        }
        if sweep >= 360.0 {
            self.full_turns += 1;
            self.push(layer, Entity::Circle(CircleEntity { base: b, c: centre, r }));
            return Ok(());
        }
        // The ends as the file holds them: radians unconverted, else the degrees'.
        let (a0, a1) = if radians {
            (norm_angle(start), norm_angle(end))
        } else {
            (norm_angle(rad(start)), norm_angle(rad(end)))
        };
        self.push(layer, Entity::Arc(ArcEntity { base: b, c: centre, r, a0, a1 }));
        Ok(())
    }

    /// A Netcad 8 smart object as the symbol it is: its strokes turned,
    /// scaled and placed as the object; its values as attributes of the
    /// symbol's first object. A class this reader does not draw is a point.
    fn planet(&mut self, e: &format::Entity, layer: usize, mut b: EntityBase) -> Result<(), &'static str> {
        let c = e
            .coords
            .first()
            .filter(|c| c.x.is_finite() && c.y.is_finite() && c.x.abs() <= WORLD && c.y.abs() <= WORLD)
            .ok_or("koordinatı sayı değil ya da ±100 000 km dışında")?;
        let at = Vec2 { x: c.x, y: c.y };
        b.attrs = properties(e);
        let font = self.font;
        let measure = move |t: &str, h: f64| width_em(t, font) * h;
        let Some(symbol) = symbols::planet_symbol(e, &measure) else {
            *self.undrawn.entry(e.smart).or_default() += 1;
            self.grow(at);
            self.push_smart(layer, Entity::Point(PointEntity { base: b, p: at, z: None }));
            return Ok(());
        };
        let mut size = e.scale;
        if !(size.is_finite() && size > 0.0 && size <= 1000.0) {
            size = 1.0; // Netcad's own default object size
            self.unsized_objects += 1;
        }
        let rotation = if e.rotation.is_finite() { upright(e.rotation) } else { 0.0 };
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
                    self.grow(Vec2 { x: c.x - r, y: c.y - r });
                    self.grow(Vec2 { x: c.x + r, y: c.y + r });
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
                        ([a, z], false) => Entity::Line(LineEntity { base, a: *a, b: *z, za: None, zb: None }),
                        (_, true) => Entity::Polygon(PathEntity { base, pts, bulges: None, holes: None, zs: None, parts: None }),
                        _ => Entity::Polyline(PathEntity { base, pts, bulges: None, holes: None, zs: None, parts: None }),
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
                                base,
                                p,
                                text: line.clone(),
                                height: height * size,
                                rotation: place.rotation,
                                align: align_of(*anchor),
                                width_factor: None,
                                mask: false,
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
        for (t, &n) in header.dropped.iter().enumerate() {
            self.report.skip_n(
                &type_words(t),
                "kayıt okunamadı: türü için kısa, koordinatı ±100 000 km dışında, metni ya da yüksekliği olmayan, alanı sıfır ya da tek noktalı",
                0,
                n as u32,
            );
        }
        for (t, &n) in header.unsupported.iter().enumerate() {
            self.report
                .skip_n(&format!("NCZ türü {t}"), "bu okuyucunun tanımadığı geometri türü; okunmadı", 0, n as u32);
        }
        if header.swept_entities > 0 {
            self.report.note_n(
                "Netcad 8 düzeni",
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
                SmartClass::Settlement => "sembolü çizildi: dairesi, nizamı, kat sayısı ve bahçe mesafeleri; değerleri dairenin öznitelikleri",
                SmartClass::Construction => "sembolü çizildi: TAKS ve KAKS dairesi ya da Emsal, Hmax, Yençok satırları; değerleri ilk nesnesinin öznitelikleri",
                SmartClass::Road => "sembolü çizildi: dairesi ve genişliği; değeri dairenin özniteliği",
                SmartClass::PlanNote => "kutusu ve metni çizildi (RTF biçimlendirmesi kaldırıldı, metin kutuya sarıldı)",
                SmartClass::FunctionName => "adı yazı olarak çizildi; değeri yazının özniteliği",
                SmartClass::None | SmartClass::Other => "sembolü çizildi",
            };
            self.report.note_n(&format!("Akıllı nesne ({})", class.name()), how, 0, n);
        }
        for (class, &n) in &self.undrawn {
            self.report.note_n(
                &format!("Akıllı nesne ({})", class.name()),
                "sembolü çizilemedi (sınıfı tanınmıyor ya da gösterecek değeri yok); yerinde nokta olarak alındı, değerleri öznitelik",
                0,
                n,
            );
        }
        self.report
            .note_n("Akıllı nesne boyutu", "okunamadı; Netcad'in varsayılanı olan 1 ile çizildi", 0, self.unsized_objects);
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
                &format!("bulunamadı, pafta dosyanın sakladığı sınırlayıcı kutu olarak çizildi: {}", why.reason()),
                0,
                n,
            );
        }
        self.report
            .note_n("Yay", "tam turdan geniş olduğu için daire olarak alındı", 0, self.full_turns);
        self.report
            .note_n("Yazı", "dönüklüğü sayı değildi; yatay alındı", 0, self.flat_rotation);
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
            let mut listed: Vec<String> = tables.iter().take(6).map(|t| format!("{}: {} satır", t.table_ref, t.rows.len())).collect();
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
    /// first (the symbols read over the plan, as in Netcad; a layer counts as
    /// one when most of its objects draw one), then the rest; each in the
    /// file's table order, then by name.
    fn layers(&self) -> Vec<ImportLayer> {
        let mut order: Vec<usize> = (0..self.layers.len()).filter(|&i| self.layers[i].count > 0).collect();
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
                    name: l.name.clone(),
                    color: l.color.map_or_else(|| "ink".to_owned(), app_color),
                    visible: true,
                    locked: false,
                    line_type: LineType::Continuous,
                    line_weight: None,
                    count: l.count,
                    kinds: BTreeMap::new(),
                    bounds: None,
                }
            })
            .collect()
    }
}

/// The values of a smart object as attributes: its class, and every value the
/// user entered and did not switch off, under the label Netcad gives it. A
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
        if e.smart == SmartClass::Settlement && matches!(p.name.as_str(), "txtOn" | "txtArka" | "txtYan") {
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

/// Reads a Netcad NCZ drawing. `watch` hears how far the read is, in
/// thousandths (`PROGRESS_TOTAL`), and may stop it (`Err(STOPPED)`). A file
/// that holds no NCZ blocks at all is refused with the reason; one whose
/// blocks hold no geometry reads as empty, and the report says why.
pub fn read(bytes: &[u8], opts: &NczReadOptions, watch: &mut dyn Watch) -> Result<ImportResult, String> {
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
        return Err("Bu dosyada Netcad NCZ çizimi bulunamadı: katman tablosu, sürüm ya da geometri kaydı yok. Dosyanın bir .ncz olduğunu denetleyin; Netcad'de açılıyorsa DXF olarak kaydedip onu deneyin.".into());
    }
    em.tally(&header, &tables);
    let mut report = std::mem::take(&mut em.report);
    report.fact(
        "Sürüm",
        version(&fin).map_or_else(|| "belirtilmemiş".to_owned(), |v| format!("Netcad {v}")),
    );
    report.fact("Karakter kodlaması", "Windows-1254 (Türkçe)");
    if let Some(d) = crs::declared(&fin) {
        report.fact("Koordinat sistemi (dosya)", d.text.clone());
    }
    let layers = em.layers();
    let mut entities = std::mem::take(&mut em.out);
    for e in &mut entities {
        e.base_mut().id = 0;
    }
    let mut result = ImportResult {
        entities,
        layers,
        report: report.import(),
        bounds: em.bounds,
        declared_crs: crs::declared(&fin),
        view: None,
        blocks: Vec::new(),
    };
    kentos_formats::import::summarise(&mut result);
    let _ = watch.step(crate::PROGRESS_TOTAL, crate::PROGRESS_TOTAL);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A record of `kind` on one layer whose vertices the file gave (x easting, y northing, z).
    fn record(kind: Kind, coords: &[(f64, f64, f64)]) -> format::Entity {
        format::Entity {
            kind,
            layer_name: "KOT".into(),
            coords: coords.iter().map(|&(x, y, z)| format::Coord { x, y, z }).collect(),
            ..format::Entity::default()
        }
    }

    #[test]
    fn heights_come_from_the_vertices_the_file_gave_and_from_nothing_else() {
        use crate::format::Sink;
        let mut em = Emitter::new(&NczReadOptions::default());
        let ring = |z: [f64; 4]| [(0.0, 0.0, z[0]), (10.0, 0.0, z[1]), (10.0, 10.0, z[2]), (0.0, 10.0, z[3]), (0.0, 0.0, z[0])];
        for e in [
            // 0-2: lines: both ends, a 2D one (Z 0 is no elevation), one end at 0 with the other above it
            record(Kind::Line, &[(1.0, 2.0, 105.5), (3.0, 4.0, 107.25)]),
            record(Kind::Line, &[(1.0, 2.0, 0.0), (3.0, 4.0, 0.0)]),
            record(Kind::Line, &[(1.0, 2.0, 0.0), (3.0, 4.0, 12.5)]),
            // 3-5: polylines: a 0 among the heights is a height; all 0; a Z that is not a number leaves its vertex without
            record(Kind::Polyline, &[(0.0, 0.0, 10.0), (10.0, 0.0, 0.0), (10.0, 10.0, 12.5)]),
            record(Kind::Polyline, &[(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)]),
            record(Kind::Polyline, &[(0.0, 0.0, 10.0), (10.0, 0.0, f64::NAN), (10.0, 10.0, -2.5)]),
            // 6-7: areas drawn point by point: the closing vertex goes with its height; all 0
            record(Kind::Polygon, &ring([1.0, 2.0, 3.0, 4.0])),
            record(Kind::Polygon, &ring([0.0; 4])),
            // 8-10: corners the reader made up (a sheet's, a rectangle's) or knows one of (a triangle's) have no height to give
            record(Kind::MapSheet, &ring([5.0, 5.0, 5.0, 5.0])),
            record(Kind::Triangle, &[(0.0, 0.0, 5.0), (10.0, 0.0, 0.0), (0.0, 10.0, 0.0)]),
            record(Kind::SmartObject, &ring([7.0, 7.0, 7.0, 7.0])),
        ] {
            assert!(em.entity(&e));
        }
        let report = std::mem::take(&mut em.report).import();
        assert!(report.skipped.is_empty());
        let out = &em.out;
        let Entity::Line(l) = &out[0] else { panic!("{:?}", out[0]) };
        assert_eq!((l.za, l.zb), (Some(105.5), Some(107.25)));
        for i in [1, 2] {
            let Entity::Line(l) = &out[i] else { panic!("{:?}", out[i]) };
            assert_eq!((l.za, l.zb), if i == 1 { (None, None) } else { (Some(0.0), Some(12.5)) });
        }
        let zs = |i: usize| match &out[i] {
            Entity::Polyline(p) | Entity::Polygon(p) => p.zs.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(zs(3), Some(vec![Some(10.0), Some(0.0), Some(12.5)]));
        assert_eq!(zs(4), None);
        assert_eq!(zs(5), Some(vec![Some(10.0), None, Some(-2.5)]));
        assert!(matches!(out[6], Entity::Polygon(_)));
        assert_eq!(zs(6), Some(vec![Some(1.0), Some(2.0), Some(3.0), Some(4.0)]));
        assert_eq!(zs(7), None);
        assert_eq!((zs(8), zs(9), zs(10)), (None, None, None));
        // The heights are counted where every import says it: as a fact of the file.
        let mut result = ImportResult { entities: out.clone(), layers: Vec::new(), report, bounds: None, declared_crs: None, view: None, blocks: Vec::new() };
        kentos_formats::import::summarise(&mut result);
        assert!(result.report.source.iter().any(|f| (f.label.as_str(), f.value.as_str()) == ("Kotlu nesne", "5")), "{:?}", result.report.source);
    }

    #[test]
    fn a_note_wraps_at_its_width() {
        let lines = wrapped("bir iki üç dört beş", 3.0, 1.0, Font::DEFAULT);
        assert!(lines.len() > 1);
        assert_eq!(lines.join(" "), "bir iki üç dört beş");
        assert_eq!(wrapped("a\n\nb", 100.0, 1.0, Font::DEFAULT), ["a", "", "b"]);
    }
}
