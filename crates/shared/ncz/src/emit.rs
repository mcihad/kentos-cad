//! NCZ records into the app's objects (docs/adr/0138). `format` hands over
//! the plugin's records; each becomes what KentOS has:
//!
//! | NCZ | KentOS |
//! |---|---|
//! | Point | a point; its name the label and the `Ad` attribute |
//! | Symbol, Block | a point; the symbol code (`Sembol`) or block name (`Blok`) as an attribute |
//! | Line | a line |
//! | Polyline | a polyline |
//! | Polygon, MapSheet, Triangle | a closed area, the repeated first vertex left out |
//! | Circle | a circle, centre and radius (not the plugin's 72 chords) |
//! | Arc | an arc, its ends from the plugin's own reading of the angles |
//! | Text | a text on its baseline, height and rotation |
//! | Netcad 8 smart object | its symbol: circles, lines and texts, turned and scaled as the object; its values as attributes of the symbol's first object |
//! | other smart objects | a point (no rectangle) or an area, the token as an attribute |
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
    LineType, NczReadOptions, PathEntity, PointEntity, TextEntity, Vec2,
};
use kentos_formats::math::{TAU, cos, hypot, norm_angle, rad, sin, sin_cos_deg};
use kentos_formats::report::Report;
use kentos_formats::watch::{STOPPED, Watch};
use kentos_geometry_core::text::{Font, width_em};

use crate::format::{self, Header, Kind, Outcome, RAD_TO_DEG, SmartClass};
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
    centred_texts: u32,
    /// Where each record read begins, and its layer: what `strays` looks at.
    places: Vec<(f64, f64, usize)>,
    bounds: Option<Bounds>,
}

impl format::Sink for Emitter {
    fn begin(&mut self, final_header: &Header) -> bool {
        self.fin = final_header.clone();
        true
    }

    fn entity(&mut self, e: &format::Entity) -> bool {
        let layer = self.layer_of(e);
        if self.out.len() >= self.limit {
            self.truncated += 1;
            return true;
        }
        match self.place(e, layer) {
            Ok(()) => {
                if let Some(c) = e.coords.first() {
                    self.places.push((c.x, c.y, layer));
                }
                // The pen: 0 is the thinnest line, a negative width the layer's.
                if e.line_width.is_finite() && e.line_width > 0.0 && e.line_width <= 1000.0 {
                    let mm = e.line_width / 10.0;
                    self.widths += 1;
                    self.widest = self.widest.max(mm);
                    self.thinnest = if self.widths == 1 { mm } else { self.thinnest.min(mm) };
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
            centred_texts: 0,
            places: Vec::new(),
            bounds: None,
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
                self.push(layer, Entity::Line(LineEntity { base: b, a: v(a), b: v(z) }));
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
                self.push(layer, Entity::Polyline(PathEntity { base: b, pts, bulges: None, holes: None }));
            }
            Kind::Polygon | Kind::MapSheet | Kind::Triangle => {
                let key = if e.kind == Kind::MapSheet { "Pafta" } else { "Etiket" };
                note(&mut b, key, &e.label);
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
                    }),
                );
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
        while pts.len() >= 2 && pts.last() == pts.first() {
            pts.pop();
        }
        if pts.len() < 3 {
            return Err("kapalı şekil üç köşeye ulaşmıyor");
        }
        for p in &pts {
            self.grow(*p);
        }
        self.push(layer, Entity::Polygon(PathEntity { base: b, pts, bulges: None, holes: None }));
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
                        ([a, z], false) => Entity::Line(LineEntity { base, a: *a, b: *z }),
                        (_, true) => Entity::Polygon(PathEntity { base, pts, bulges: None, holes: None }),
                        _ => Entity::Polyline(PathEntity { base, pts, bulges: None, holes: None }),
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
                        let w = width_em(line, font) * height;
                        let dx = match anchor {
                            Anchor::MiddleCentre => -w / 2.0,
                            Anchor::MiddleRight => -w,
                            Anchor::BaselineLeft | Anchor::MiddleLeft | Anchor::TopLeft => 0.0,
                        };
                        let dy = match anchor {
                            Anchor::BaselineLeft => 0.0,
                            Anchor::MiddleLeft | Anchor::MiddleCentre | Anchor::MiddleRight => -height / 2.0,
                            Anchor::TopLeft => -height,
                        } - i as f64 * LINE_PITCH * height;
                        if *anchor != Anchor::BaselineLeft {
                            self.centred_texts += 1;
                        }
                        let p = place.apply((at.0 + dx, at.1 + dy));
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
                            }),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Objects hundreds of kilometres from the rest of the drawing, said. A
    /// real UİP holds two copies of a transformer outline drawn at (0, 0), a
    /// slip in the source that Netcad shows too: the drawing's data, read as
    /// such, but zooming to what was imported then fits a 4 400 km box and
    /// the city is a dot in its corner. The bulk is where 96 % of the objects
    /// are; a stray is outside that box grown by twenty times its size and at
    /// least 100 km on every side (never the long tail of a real town).
    fn strays(&mut self) {
        if self.places.len() < 100 {
            return;
        }
        let percentile = |east: bool, q: f64| {
            let mut v: Vec<f64> = self.places.iter().map(|p| if east { p.0 } else { p.1 }).collect();
            let at = (q * (v.len() - 1) as f64) as usize;
            v.select_nth_unstable_by(at, f64::total_cmp);
            v[at]
        };
        let (x0, x1, y0, y1) = (percentile(true, 0.02), percentile(true, 0.98), percentile(false, 0.02), percentile(false, 0.98));
        let grow = (20.0_f64 * (x1 - x0).max(y1 - y0)).max(100_000.0);
        let far: Vec<&(f64, f64, usize)> = self
            .places
            .iter()
            .filter(|(x, y, _)| !(*x >= x0 - grow && *x <= x1 + grow && *y >= y0 - grow && *y <= y1 + grow))
            .collect();
        if far.is_empty() || far.len() * 100 > self.places.len() {
            return;
        }
        let mut per: BTreeMap<&str, u32> = BTreeMap::new();
        for p in &far {
            *per.entry(self.layers[p.2].name.as_str()).or_default() += 1;
        }
        let mut layers: Vec<String> = per.iter().take(3).map(|(n, k)| format!("{n} {k}")).collect();
        if per.len() > 3 {
            layers.push("…".into());
        }
        let first = far[0];
        // `+ 0.0` writes a negative zero as 0.
        let reason = format!(
            "çizimin geri kalanından çok uzakta ({}; ilki Y {:.0}, X {:.0} yakınında); görünüm onları dışarıda bırakır. Kaynakta yanlış yere düşmüşlerse silin.",
            layers.join(", "),
            first.0 + 0.0,
            first.1 + 0.0
        );
        self.report.note_n("Uzaktaki nesne", &reason, 0, far.len() as u32);
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
        if self.centred_texts > 0 {
            self.report.note_n(
                "Akıllı nesne yazısı",
                "KentOS yazıları sol alt köşeden yerleşir; ortalı yazıların konumu çizimin yazı tipiyle ölçülerek hesaplandı",
                0,
                self.centred_texts,
            );
        }
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
                    "nesnelerin kendi kalınlığı var ({:.2}–{:.2} mm); KentOS nesne başına kalınlığı henüz taşımıyor, katmanın kalınlığıyla çizilir",
                    self.thinnest, self.widest
                ),
                0,
                self.widths,
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
        self.strays();
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

    #[test]
    fn a_note_wraps_at_its_width() {
        let lines = wrapped("bir iki üç dört beş", 3.0, 1.0, Font::DEFAULT);
        assert!(lines.len() > 1);
        assert_eq!(lines.join(" "), "bir iki üç dört beş");
        assert_eq!(wrapped("a\n\nb", 100.0, 1.0, Font::DEFAULT), ["a", "", "b"]);
    }
}
