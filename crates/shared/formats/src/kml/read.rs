//! KML and KMZ in (docs/adr/0222 §3): OGC KML 2.2 with Google's `gx` tracks,
//! read with roxmltree (its depth guarded, `xml`), a KMZ archive's `doc.kml`
//! (else its first `.kml`). Each Folder holding placemarks is a layer named by
//! its path (“Yollar / Ana yollar”); placemarks outside every Folder go to
//! the Document's layer. A Placemark is an object: a Point, LineString,
//! LinearRing (a closed path), Polygon (an outline and its holes),
//! MultiGeometry (one object of each kind its members are: a multi-point, a
//! multi-part path, a multi-part area) or a `gx:Track` and `gx:MultiTrack`.
//! Its name is the Ad attribute, its description Açıklama, its
//! ExtendedData's values attributes of their names, its time Zaman (or
//! Başlangıç and Bitiş). Its style (inline, else the Document's by `#id`, a
//! StyleMap's normal pair) gives its colour (a line's LineStyle, an area's
//! outline or fill, a point's IconStyle) and its line weight. Coordinates are
//! WGS 84 longitude and latitude as the file writes them, each the nearest
//! float64 to its decimals; an altitude is the object's elevation only where
//! the altitude mode is absolute (above sea level), else it is said and left.
//! Overlays, models, network links and tours are not objects: counted and
//! said. Nothing here panics on any input.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{CrsSource, DeclaredCrs, Entity, ImportResult, Vec2};
use roxmltree::Node;

use super::style::{Style, kml_color, line_weight};
use crate::gis::{CLOSING_Z, Collect, Ring, Shape, open_ring};
use crate::text;
use crate::xml::{DEPTH, Rows, Unparsed, document};
use crate::zip;

/// The layer of placemarks outside every Folder when the Document has no name.
pub const DEFAULT_LAYER: &str = "KML";

/// A position as the file writes it: longitude, latitude, altitude.
type Position = (f64, f64, Option<f64>);

/// The text of a KML file or a KMZ archive's KML; what the file is.
fn kml_text(bytes: &[u8]) -> Result<(String, String), String> {
    if !zip::sniff(bytes) {
        let (enc, bom) = text::sniff(bytes);
        return Ok((text::decode(&bytes[bom..], enc), "KML".into()));
    }
    let limits = zip::Limits::default();
    let listed = zip::list(bytes, &limits).map_err(|e| format!("KMZ okunamadı: {e}"))?;
    let at_root = |l: &zip::Listed| !l.name.contains('/');
    let is_kml = |l: &zip::Listed| l.name.to_lowercase().ends_with(".kml");
    let chosen = listed
        .iter()
        .find(|l| l.name.eq_ignore_ascii_case("doc.kml"))
        .or_else(|| listed.iter().find(|l| at_root(l) && is_kml(l)))
        .or_else(|| listed.iter().find(|l| is_kml(l)))
        .ok_or("KMZ arşivinde .kml dosyası yok; hiçbir şey alınmadı.")?;
    let name = chosen.name.clone();
    let mut entries = zip::unpack(bytes, &listed, &limits, |l| l.name == name)
        .map_err(|e| format!("KMZ okunamadı: {e}"))?;
    let entry = entries.pop().ok_or("KMZ arşivindeki KML açılamadı.")?;
    let (enc, bom) = text::sniff(&entry.data);
    Ok((
        text::decode(&entry.data[bom..], enc),
        format!("KMZ ({name})"),
    ))
}

fn is(n: &Node<'_, '_>, name: &str) -> bool {
    n.is_element() && n.tag_name().name() == name
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| is(c, name))
}

fn text_of(n: Option<Node<'_, '_>>) -> Option<String> {
    let t: String = n?
        .descendants()
        .filter(|d| d.is_text())
        .filter_map(|d| d.text())
        .collect();
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// The positions of a `coordinates` text: `lon,lat[,alt]` tuples apart by
/// white space (a space after a comma, as some writers put, is mended).
fn positions(t: &str) -> Option<Vec<Position>> {
    let mut tuples: Vec<String> = Vec::new();
    for word in t.split_whitespace() {
        match tuples.last_mut() {
            Some(last) if last.ends_with(',') || word.starts_with(',') => last.push_str(word),
            _ => tuples.push(word.to_owned()),
        }
    }
    tuples
        .iter()
        .map(|tuple| {
            let mut it = tuple.split(',').map(|v| v.trim().parse::<f64>());
            let lon = it.next()?.ok()?;
            let lat = it.next()?.ok()?;
            let alt = match it.next() {
                None => None,
                Some(v) => Some(v.ok()?),
            };
            if it.next().is_some() {
                return None;
            }
            let ok = lon.is_finite()
                && lat.is_finite()
                && lon.abs() <= 180.0
                && lat.abs() <= 90.0
                && alt.is_none_or(f64::is_finite);
            ok.then_some((lon, lat, alt))
        })
        .collect()
}

/// Whether a geometry's altitude mode is absolute (its altitudes are heights above sea level).
fn absolute(g: Node<'_, '_>) -> bool {
    text_of(child(g, "altitudeMode")).as_deref() == Some("absolute")
}

/// What a placemark's geometry gave: shapes of each kind, and what to say.
#[derive(Default)]
struct Shapes {
    points: Vec<(Vec2, Option<f64>)>,
    paths: Vec<Ring>,
    areas: Vec<(Ring, Vec<Ring>)>,
    /// Altitudes not taken (not absolute): rings and points.
    relative: u32,
    /// Coordinates that could not be read: the element's name.
    bad: Vec<&'static str>,
    closing_z: bool,
    /// Rings of fewer than three distinct corners.
    thin: u32,
}

impl Shapes {
    fn ring(&mut self, pts: &[Position], abs: bool) -> Ring {
        if !abs && pts.iter().any(|p| p.2.is_some()) {
            self.relative += 1;
        }
        Ring::new(
            pts.iter().map(|&(x, y, _)| Vec2 { x, y }).collect(),
            pts.iter().map(|p| p.2.filter(|_| abs)),
        )
    }

    fn coordinates(&mut self, g: Node<'_, '_>, what: &'static str) -> Option<Vec<Position>> {
        let t = text_of(child(g, "coordinates")).unwrap_or_default();
        match positions(&t) {
            Some(p) => Some(p),
            None => {
                self.bad.push(what);
                None
            }
        }
    }

    /// A closed ring of a Polygon, without its closing point; none for fewer
    /// than three corners (consecutive repeats counted once).
    fn closed(&mut self, pts: &[Position], abs: bool) -> Option<Ring> {
        let (r, _, differs) = open_ring(self.ring(pts, abs));
        self.closing_z |= differs;
        let mut distinct = r.pts.clone();
        distinct.dedup();
        if distinct.len() < 3 {
            self.thin += 1;
            return None;
        }
        Some(r)
    }

    fn geometry(&mut self, g: Node<'_, '_>, skipped: &mut BTreeMap<&'static str, u32>) {
        let abs = absolute(g);
        match g.tag_name().name() {
            "Point" => {
                if let Some(pts) = self.coordinates(g, "Point") {
                    match pts.as_slice() {
                        [(x, y, z)] => {
                            if !abs && z.is_some() {
                                self.relative += 1;
                            }
                            self.points.push((Vec2 { x: *x, y: *y }, z.filter(|_| abs)));
                        }
                        _ => self.bad.push("Point"),
                    }
                }
            }
            "LineString" => {
                if let Some(pts) = self.coordinates(g, "LineString") {
                    if pts.len() >= 2 {
                        let r = self.ring(&pts, abs);
                        self.paths.push(r);
                    } else {
                        self.bad.push("LineString");
                    }
                }
            }
            // A lone ring is a closed path.
            "LinearRing" => {
                if let Some(pts) = self.coordinates(g, "LinearRing") {
                    if pts.len() >= 3 {
                        let r = self.ring(&pts, abs);
                        self.paths.push(r);
                    } else {
                        self.bad.push("LinearRing");
                    }
                }
            }
            // An outline whose coordinates are not read leaves the area out, said once as the Polygon's;
            // a hole's leaves the hole out, said as its LinearRing's.
            "Polygon" => {
                let Some(outer) = child(g, "outerBoundaryIs").and_then(|o| child(o, "LinearRing"))
                else {
                    self.bad.push("Polygon");
                    return;
                };
                let Some(pts) = self.coordinates(outer, "Polygon") else {
                    return;
                };
                let Some(outer) = self.closed(&pts, abs) else {
                    return;
                };
                let mut holes = Vec::new();
                for ring in g
                    .children()
                    .filter(|c| is(c, "innerBoundaryIs"))
                    .filter_map(|i| child(i, "LinearRing"))
                {
                    if let Some(pts) = self.coordinates(ring, "LinearRing")
                        && let Some(h) = self.closed(&pts, abs)
                    {
                        holes.push(h);
                    }
                }
                self.areas.push((outer, holes));
            }
            "MultiGeometry" => {
                for member in g.children().filter(Node::is_element) {
                    self.geometry(member, skipped);
                }
            }
            // Google's track: a position per gx:coord, “lon lat alt”.
            "Track" => {
                let pts: Option<Vec<Position>> = g
                    .children()
                    .filter(|c| is(c, "coord"))
                    .map(|c| {
                        let t = text_of(Some(c)).unwrap_or_default();
                        positions(&t.split_whitespace().collect::<Vec<_>>().join(","))
                            .and_then(|p| (p.len() == 1).then(|| p[0]))
                    })
                    .collect();
                match pts {
                    Some(pts) if pts.len() >= 2 => {
                        let r = self.ring(&pts, abs);
                        self.paths.push(r);
                    }
                    _ => self.bad.push("gx:Track"),
                }
            }
            "MultiTrack" => {
                for t in g.children().filter(|c| is(c, "Track")) {
                    self.geometry(t, skipped);
                }
            }
            "Model" => *skipped.entry("Model").or_default() += 1,
            _ => {}
        }
    }

    /// One object of each kind the geometry gave.
    fn objects(self) -> Vec<Shape> {
        let mut out = Vec::new();
        out.extend(Shape::points(self.points));
        out.extend(Shape::lines(self.paths));
        out.extend(Shape::area(self.areas));
        out
    }
}

/// A style's place: a Style's own, or a StyleMap's normal pair's.
#[derive(Default)]
struct Styles {
    own: HashMap<String, Style>,
    maps: HashMap<String, String>,
}

impl Styles {
    fn of(root: Node<'_, '_>) -> Styles {
        let mut s = Styles::default();
        for n in root.descendants().filter(|n| n.is_element()) {
            let Some(id) = n.attribute("id") else {
                continue;
            };
            match n.tag_name().name() {
                "Style" => {
                    s.own.insert(id.to_owned(), Style::of(n));
                }
                "StyleMap" => {
                    let normal = n.children().filter(|c| is(c, "Pair")).find(|p| {
                        text_of(child(*p, "key")).as_deref().unwrap_or("normal") == "normal"
                    });
                    if let Some(pair) = normal {
                        if let Some(url) = text_of(child(pair, "styleUrl")) {
                            s.maps.insert(id.to_owned(), url);
                        } else if let Some(inline) = child(pair, "Style") {
                            let key = format!("{id}#normal");
                            s.own.insert(key.clone(), Style::of(inline));
                            s.maps.insert(id.to_owned(), format!("#{key}"));
                        }
                    }
                }
                _ => {}
            }
        }
        s
    }

    /// The style a `styleUrl` names in this file (`#id`); a StyleMap's normal one.
    fn named(&self, url: &str) -> Option<&Style> {
        let id = url.strip_prefix('#')?;
        if let Some(s) = self.own.get(id) {
            return Some(s);
        }
        let mapped = self.maps.get(id)?.strip_prefix('#')?;
        self.own.get(mapped)
    }
}

/// What the walk carries down: the Folders' names and whether they are shown.
#[derive(Clone)]
struct Place {
    path: Vec<String>,
    visible: bool,
}

struct Walk<'s> {
    c: Collect,
    styles: &'s Styles,
    /// Layers hidden by a Folder's visibility.
    hidden: BTreeMap<String, bool>,
    skipped: BTreeMap<&'static str, u32>,
    relative: u32,
    bad: BTreeMap<&'static str, (u32, u32)>,
    mixed: u32,
    closing_z: u32,
    thin: u32,
    external: u32,
    unnamed: u32,
}

impl Walk<'_> {
    fn container(&mut self, n: Node<'_, '_>, place: &Place, line_of: &dyn Fn(Node<'_, '_>) -> u32) {
        for c in n.children().filter(Node::is_element) {
            match c.tag_name().name() {
                "Document" => self.container(c, place, line_of),
                "Folder" => {
                    let name = text_of(child(c, "name")).unwrap_or_else(|| {
                        self.unnamed += 1;
                        "Adsız klasör".to_owned()
                    });
                    let visible =
                        place.visible && text_of(child(c, "visibility")).as_deref() != Some("0");
                    let mut path = place.path.clone();
                    path.push(name);
                    self.container(c, &Place { path, visible }, line_of);
                }
                "Placemark" => self.placemark(c, place, line_of(c)),
                "NetworkLink" => *self.skipped.entry("NetworkLink").or_default() += 1,
                "GroundOverlay" => *self.skipped.entry("GroundOverlay").or_default() += 1,
                "ScreenOverlay" => *self.skipped.entry("ScreenOverlay").or_default() += 1,
                "PhotoOverlay" => *self.skipped.entry("PhotoOverlay").or_default() += 1,
                "Tour" => *self.skipped.entry("gx:Tour").or_default() += 1,
                _ => {}
            }
        }
    }

    fn placemark(&mut self, p: Node<'_, '_>, place: &Place, line: u32) {
        let mut shapes = Shapes::default();
        for g in p.children().filter(Node::is_element) {
            if matches!(
                g.tag_name().name(),
                "Point"
                    | "LineString"
                    | "LinearRing"
                    | "Polygon"
                    | "MultiGeometry"
                    | "Track"
                    | "MultiTrack"
                    | "Model"
            ) {
                shapes.geometry(g, &mut self.skipped);
            }
        }
        self.relative += u32::from(shapes.relative > 0);
        self.closing_z += u32::from(shapes.closing_z);
        self.thin += shapes.thin;
        for what in shapes.bad.drain(..) {
            let e = self.bad.entry(what).or_insert((0, line));
            e.0 += 1;
        }
        let objects = shapes.objects();
        if objects.len() > 1 {
            self.mixed += 1;
        }
        let mut attrs = BTreeMap::new();
        if let Some(name) = text_of(child(p, "name")) {
            attrs.insert("Ad".to_owned(), name);
        }
        if let Some(d) = text_of(child(p, "description")) {
            attrs.insert("Açıklama".to_owned(), d);
        }
        if let Some(when) = child(p, "TimeStamp").and_then(|t| text_of(child(t, "when"))) {
            attrs.insert("Zaman".to_owned(), when);
        }
        if let Some(span) = child(p, "TimeSpan") {
            if let Some(b) = text_of(child(span, "begin")) {
                attrs.insert("Başlangıç".to_owned(), b);
            }
            if let Some(e) = text_of(child(span, "end")) {
                attrs.insert("Bitiş".to_owned(), e);
            }
        }
        if let Some(data) = child(p, "ExtendedData") {
            for d in data
                .descendants()
                .filter(|d| is(d, "Data") || is(d, "SimpleData"))
            {
                let Some(name) = d.attribute("name").map(str::trim).filter(|s| !s.is_empty())
                else {
                    continue;
                };
                let value = if is(&d, "Data") {
                    text_of(child(d, "value"))
                } else {
                    text_of(Some(d))
                };
                attrs.insert(name.to_owned(), value.unwrap_or_default());
            }
        }
        // Its own style, else the one its styleUrl names.
        let inline = child(p, "Style").map(Style::of);
        let url = text_of(child(p, "styleUrl"));
        if inline.is_none() && url.as_deref().is_some_and(|u| !u.starts_with('#')) {
            self.external += 1;
        }
        let style = inline
            .as_ref()
            .or_else(|| url.as_deref().and_then(|u| self.styles.named(u)));
        let layer = place.path.join(" / ");
        if !place.visible {
            self.hidden.insert(layer.clone(), true);
        }
        for shape in objects {
            let before = self.c.entities.len();
            let (color, weight) = match (&shape, style) {
                (_, None) => (None, None),
                (Shape::Point { .. } | Shape::Points(_), Some(s)) => {
                    (s.icon_color.or(s.line_color), None)
                }
                (Shape::Polygon(..) | Shape::Parts(_), Some(s)) => (
                    if s.outline {
                        s.line_color.or(s.poly_color)
                    } else {
                        s.poly_color.or(s.line_color)
                    },
                    s.line_width.map(line_weight),
                ),
                (_, Some(s)) => (s.line_color, s.line_width.map(line_weight)),
            };
            self.c.add(shape, &layer, &attrs, None);
            if let Some(e) = self.c.entities.get_mut(before) {
                let base = Entity::base_mut(e);
                base.color = color.map(kml_color);
                base.line_weight = weight;
            }
        }
    }
}

/// Reads a KML file or a KMZ archive: its objects in WGS 84 longitude and
/// latitude, at most `max` (0: the readers' limit).
pub fn read(bytes: &[u8], max: u32) -> Result<ImportResult, String> {
    let (body, what) = kml_text(bytes)?;
    let doc = match document(&body) {
        Ok(doc) => doc,
        Err(Unparsed::NotXml) => {
            return Err("KML okunamadı: dosya XML değil; hiçbir şey alınmadı.".into());
        }
        Err(Unparsed::TooDeep) => {
            return Err(format!(
                "KML okunamadı: XML öğeleri {DEPTH} düzeyden derin iç içe; hiçbir şey alınmadı."
            ));
        }
    };
    let root = doc.root_element();
    if root.tag_name().name() != "kml" {
        return Err(format!(
            "KML değil: kök öğe “{}” (kml bekleniyordu); hiçbir şey alınmadı.",
            root.tag_name().name()
        ));
    }
    let styles = Styles::of(root);
    let document_node = child(root, "Document");
    let name = document_node.and_then(|d| text_of(child(d, "name")));
    let mut walk = Walk {
        c: Collect::new(max),
        styles: &styles,
        hidden: BTreeMap::new(),
        skipped: BTreeMap::new(),
        relative: 0,
        bad: BTreeMap::new(),
        mixed: 0,
        closing_z: 0,
        thin: 0,
        external: 0,
        unnamed: 0,
    };
    let rows = Rows::new(doc.input_text());
    let line_of = |n: Node<'_, '_>| rows.row(n.range().start);
    walk.container(
        root,
        &Place {
            path: Vec::new(),
            visible: true,
        },
        &line_of,
    );
    let mut c = walk.c;
    for (what, n) in &walk.skipped {
        let reason = match *what {
            "NetworkLink" => "başka bir dosyaya ya da adrese bağlantı; uzaktaki içerik alınmaz",
            "GroundOverlay" | "ScreenOverlay" | "PhotoOverlay" => {
                "resim kaplaması; alınmadı (resmi Resim ekle ile yerleştirin)"
            }
            "Model" => "3B model; alınmadı",
            _ => "gezinti; alınmadı",
        };
        c.report.skip_n(what, reason, 0, *n);
    }
    for (what, (n, line)) in &walk.bad {
        c.report.skip_n(
            what,
            "koordinatları okunmuyor (boylam, enlem[, yükseklik] bekleniyordu); alınmadı",
            *line,
            *n,
        );
    }
    if walk.thin > 0 {
        c.report.skip_n(
            "Halka",
            "üçten az farklı köşesi var; alınmadı",
            0,
            walk.thin,
        );
    }
    if walk.closing_z > 0 {
        c.report.note_n(CLOSING_Z.0, CLOSING_Z.1, 0, walk.closing_z);
    }
    if walk.relative > 0 {
        c.report.note_n(
            "Yükseklik",
            "yükseklik kipi absolute değil (zemine ya da yere göre); kot olarak alınmadı",
            0,
            walk.relative,
        );
    }
    if walk.unnamed > 0 {
        c.report.note_n(
            "Adsız klasör",
            "adı olmayan klasör “Adsız klasör” adıyla katman yolunda yer aldı",
            0,
            walk.unnamed,
        );
    }
    if walk.mixed > 0 {
        c.report.note_n(
            "Karışık MultiGeometry",
            "türleri ayrı nesneler oldu (noktalar, çizgiler, alanlar)",
            0,
            walk.mixed,
        );
    }
    if walk.external > 0 {
        c.report.note_n(
            "Dış biçem",
            "başka dosyadaki biçeme bağlı; renk alınmadı",
            0,
            walk.external,
        );
    }
    c.report.fact("Biçim", what);
    if let Some(n) = &name {
        c.report.fact("Ad", n.clone());
    }
    c.report
        .fact("Koordinat sistemi", "WGS 84 boylam, enlem (EPSG:4326)");
    let default_layer = name.clone().unwrap_or_else(|| DEFAULT_LAYER.to_owned());
    let declared = DeclaredCrs {
        srid: Some(4326),
        text: "WGS 84 (KML)".into(),
        source: CrsSource::Kml,
    };
    let mut result = c.finish(&default_layer, Some(declared));
    // Each layer's colour: its objects' most common (the first met of the most), else the ink.
    let mut tally: HashMap<&str, Vec<(String, u32)>> = HashMap::new();
    for e in &result.entities {
        let b = e.base();
        if let Some(color) = &b.color {
            let list = tally.entry(b.layer_id.as_str()).or_default();
            match list.iter_mut().find(|(c, _)| c == color) {
                Some((_, n)) => *n += 1,
                None => list.push((color.clone(), 1)),
            }
        }
    }
    let colours: HashMap<String, String> = tally
        .into_iter()
        .filter_map(|(layer, list)| {
            let most = list.iter().map(|(_, n)| *n).max()?;
            let (c, _) = list.into_iter().find(|(_, n)| *n == most)?;
            Some((layer.to_owned(), c))
        })
        .collect();
    for layer in &mut result.layers {
        if let Some(c) = colours.get(&layer.name) {
            layer.color = c.clone();
        }
        if walk.hidden.contains_key(&layer.name) {
            layer.visible = false;
        }
    }
    Ok(result)
}
