//! GeoJSON in. The file is walked once, feature by feature (a collection of
//! a million features is never held as a JSON tree): each feature's
//! geometry becomes points, lines, paths and areas with holes, its
//! `properties` their text attributes. The reading rules are written out
//! in docs/adr/0046 and followed by the independent reader in
//! tools/formats/gis.py; both read fixtures/formats/v1/gis alike.
//!
//! - Point → point (with its height when the position has a third number);
//!   LineString → line (two positions) or path; Polygon → area, holes kept,
//!   the repeated closing position dropped, ring order as written; the
//!   Multi* kinds and GeometryCollection → one object per part.
//! - Heights on lines and areas are dropped (the app's paths are plane),
//!   and said; so are numbers after the third of a position.
//! - Attributes: text as it is, numbers as the file wrote them, true and
//!   false, nested values as compact JSON text; null leaves the attribute out.
//! - A part whose coordinates are not positions of finite numbers gives
//!   nothing and is reported with its line; the rest of the file is read.

use std::collections::BTreeMap;

use kentos_contracts::{CrsSource, DeclaredCrs, GeoJsonReadOptions, ImportResult, Vec2};

use crate::gis::{Collect, Shape, open_ring};
use crate::json::{JsonError, Reader, Value};

/// The geometry types of RFC 7946 §1.4.
const GEOMETRY_TYPES: [&str; 7] = [
    "Point",
    "MultiPoint",
    "LineString",
    "MultiLineString",
    "Polygon",
    "MultiPolygon",
    "GeometryCollection",
];

/// A position: x, y, the height when the file gave a third number, and
/// whether it gave more numbers than three (they are not kept).
#[derive(Clone, Copy, Debug)]
struct Pos {
    x: f64,
    y: f64,
    z: Option<f64>,
    more: bool,
}

/// Why coordinates cannot be read.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Bad {
    /// Not a position (fewer than two numbers, something else than a
    /// number) or not nested as the geometry type needs.
    Invalid,
    /// A number beyond float64 (`1e999`).
    NotFinite,
}

/// A geometry's `coordinates`, read before its `type` may be known (the
/// members of an object come in any order): positions in nested lists.
enum Nest {
    Pos(Pos),
    List(Vec<Nest>),
    Bad(Bad),
}

/// A geometry object as read.
enum Geom {
    /// `null` (a feature without a shape).
    Null,
    /// Something that is not an object.
    NotObject,
    Obj {
        kind: Option<String>,
        coords: Option<Nest>,
        /// A GeometryCollection's members; `Err` when `geometries` is not an array.
        parts: Option<Result<Vec<Geom>, ()>>,
    },
}

/// A feature's members as read.
#[derive(Default)]
struct Feature {
    kind: Option<String>,
    geometry: Option<Geom>,
    attrs: BTreeMap<String, String>,
    /// The `kentos` member's layer and label (non-empty text only).
    layer: Option<String>,
    label: Option<String>,
    /// Nested values written as JSON text, and null values left out.
    nested: u32,
    nulls: u32,
}

fn nest(r: &mut Reader) -> Result<Nest, JsonError> {
    if r.peek() != Some(b'[') {
        r.skip()?;
        return Ok(Nest::Bad(Bad::Invalid));
    }
    let mut n = 0usize;
    let mut xyz = [0.0f64; 3];
    let mut lists: Vec<Nest> = Vec::new();
    let mut other = false;
    r.array(|r| {
        match r.peek() {
            Some(b'[') => lists.push(nest(r)?),
            Some(b'-' | b'0'..=b'9') => {
                let t = r.number()?;
                if n < 3 {
                    // Correctly rounded, as every reader of the file should; 1e999 is infinite.
                    xyz[n] = t.parse::<f64>().unwrap_or(f64::NAN);
                }
                n += 1;
            }
            _ => {
                r.skip()?;
                other = true;
            }
        }
        Ok(())
    })?;
    Ok(if other || (n > 0 && !lists.is_empty()) {
        Nest::Bad(Bad::Invalid)
    } else if n > 0 {
        let z = (n >= 3).then_some(xyz[2]);
        if n < 2 {
            Nest::Bad(Bad::Invalid)
        } else if !xyz[0].is_finite() || !xyz[1].is_finite() || z.is_some_and(|z| !z.is_finite()) {
            Nest::Bad(Bad::NotFinite)
        } else {
            Nest::Pos(Pos {
                x: xyz[0],
                y: xyz[1],
                z,
                more: n > 3,
            })
        }
    } else {
        Nest::List(lists)
    })
}

fn text(v: Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

fn geometry(r: &mut Reader) -> Result<Geom, JsonError> {
    match r.peek() {
        Some(b'{') => {}
        Some(b'n') => {
            return match r.value()? {
                Value::Null => Ok(Geom::Null),
                _ => Ok(Geom::NotObject),
            };
        }
        _ => {
            r.skip()?;
            return Ok(Geom::NotObject);
        }
    }
    let mut kind = None;
    let mut coords = None;
    let mut parts = None;
    r.object(|r, key| {
        match key.as_str() {
            "type" => kind = text(r.value()?),
            "coordinates" => coords = Some(nest(r)?),
            "geometries" => parts = Some(members(r)?),
            _ => r.skip()?,
        }
        Ok(())
    })?;
    Ok(Geom::Obj {
        kind,
        coords,
        parts,
    })
}

/// A GeometryCollection's `geometries`.
fn members(r: &mut Reader) -> Result<Result<Vec<Geom>, ()>, JsonError> {
    if r.peek() != Some(b'[') {
        r.skip()?;
        return Ok(Err(()));
    }
    let mut list = Vec::new();
    r.array(|r| {
        list.push(geometry(r)?);
        Ok(())
    })?;
    Ok(Ok(list))
}

/// A feature's `properties` as text attributes (the last of a repeated key wins).
fn properties(r: &mut Reader, f: &mut Feature) -> Result<(), JsonError> {
    f.attrs.clear();
    f.nested = 0;
    f.nulls = 0;
    if r.peek() != Some(b'{') {
        return r.skip();
    }
    r.object(|r, key| {
        match r.peek() {
            Some(b'"') => {
                let s = r.string()?;
                f.attrs.insert(key, s);
            }
            Some(b'-' | b'0'..=b'9') => {
                let n = r.number()?;
                f.attrs.insert(key, n.to_string());
            }
            Some(b'{' | b'[') => {
                let mut s = String::new();
                r.compact(&mut s)?;
                f.attrs.insert(key, s);
                f.nested += 1;
            }
            _ => match r.value()? {
                Value::Bool(b) => {
                    f.attrs.insert(key, b.to_string());
                }
                _ => {
                    f.attrs.remove(&key);
                    f.nulls += 1;
                }
            },
        }
        Ok(())
    })
}

/// Reads one member of a feature; false when the key is not a feature's.
fn feature_member(r: &mut Reader, key: &str, f: &mut Feature) -> Result<bool, JsonError> {
    match key {
        "geometry" => f.geometry = Some(geometry(r)?),
        "properties" => properties(r, f)?,
        "kentos" => {
            // KentOS's own member (the GeoJSON writer's): the object's layer and label.
            let k = r.value()?;
            let member = |key: &str| {
                k.get(key)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            };
            f.layer = member("layer");
            f.label = member("label");
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Turns geometries into objects, reporting what gives none.
struct Shapes<'a> {
    out: Vec<Shape>,
    c: &'a mut Collect,
    line: u32,
}

impl Shapes<'_> {
    fn bad(&mut self, why: Bad, what: &str) {
        match why {
            Bad::NotFinite => self.c.report.skip(
                "Sonlu olmayan koordinat",
                &format!("bir sayı float64 aralığının dışında (ör. 1e999); {what} alınmadı"),
                self.line,
            ),
            Bad::Invalid => self.c.report.skip(
                "Geçersiz koordinat",
                &format!("konumlar en az iki sayılık diziler olmalı ve türün iç içe yapısına uymalı; {what} alınmadı"),
                self.line,
            ),
        }
    }

    /// Heights of lines and areas are not kept, nor numbers after the third.
    fn plane(&mut self, ps: &[Pos]) {
        if ps.iter().any(|p| p.z.is_some()) {
            self.c.report.note(
                "Z (yükseklik)",
                "çizgi ve alanların Z değerleri alınmadı: yalnız noktalar Z taşır",
                self.line,
            );
        }
        self.more(ps);
    }

    fn more(&mut self, ps: &[Pos]) {
        if ps.iter().any(|p| p.more) {
            self.c.report.note(
                "Konumun 4. ve sonraki sayıları",
                "alınmadı (ölçü değeri ya da başka veri)",
                self.line,
            );
        }
    }

    fn multi(&mut self, kind: &str) {
        self.c.report.note(
            kind,
            "parçaları ayrı nesneler olarak alındı; her parça özelliğin özniteliklerini taşır",
            self.line,
        );
    }

    fn geometry(&mut self, g: &Geom) {
        let (kind, coords, parts) = match g {
            Geom::Obj {
                kind,
                coords,
                parts,
            } => (kind, coords, parts),
            Geom::Null | Geom::NotObject => return self.bad(Bad::Invalid, "o parça"),
        };
        let Some(kind) = kind.as_deref() else {
            return self.c.report.skip(
                "Türü belirsiz geometri",
                "type üyesi yok ya da metin değil; alınmadı",
                self.line,
            );
        };
        if kind == "GeometryCollection" {
            return match parts {
                Some(Ok(list)) => {
                    self.multi("GeometryCollection");
                    for g in list {
                        self.geometry(g);
                    }
                }
                _ => self.bad(Bad::Invalid, "GeometryCollection"),
            };
        }
        if !GEOMETRY_TYPES.contains(&kind) {
            return self.c.report.skip(
                &format!("Geometri türü “{kind}”"),
                "GeoJSON'da böyle bir geometri türü yok; alınmadı",
                self.line,
            );
        }
        let Some(coords) = coords else {
            return self.bad(Bad::Invalid, kind);
        };
        match kind {
            "Point" => match coords {
                Nest::Pos(p) => {
                    self.more(&[*p]);
                    self.out.push(Shape::Point {
                        p: Vec2 { x: p.x, y: p.y },
                        z: p.z,
                    });
                }
                Nest::Bad(b) => self.bad(*b, "Point"),
                Nest::List(_) => self.bad(Bad::Invalid, "Point"),
            },
            "MultiPoint" => match coords {
                Nest::List(items) => {
                    self.multi("MultiPoint");
                    for i in items {
                        match i {
                            Nest::Pos(p) => {
                                self.more(&[*p]);
                                self.out.push(Shape::Point {
                                    p: Vec2 { x: p.x, y: p.y },
                                    z: p.z,
                                });
                            }
                            Nest::Bad(b) => self.bad(*b, "o nokta"),
                            Nest::List(_) => self.bad(Bad::Invalid, "o nokta"),
                        }
                    }
                }
                Nest::Bad(b) => self.bad(*b, "MultiPoint"),
                Nest::Pos(_) => self.bad(Bad::Invalid, "MultiPoint"),
            },
            "LineString" => self.line_string(coords),
            "MultiLineString" => match coords {
                Nest::List(items) => {
                    self.multi("MultiLineString");
                    for i in items {
                        self.line_string(i);
                    }
                }
                Nest::Bad(b) => self.bad(*b, "MultiLineString"),
                Nest::Pos(_) => self.bad(Bad::Invalid, "MultiLineString"),
            },
            "Polygon" => self.polygon(coords),
            // MultiPolygon: the last of the seven.
            _ => match coords {
                Nest::List(items) => {
                    self.multi("MultiPolygon");
                    for i in items {
                        self.polygon(i);
                    }
                }
                Nest::Bad(b) => self.bad(*b, "MultiPolygon"),
                Nest::Pos(_) => self.bad(Bad::Invalid, "MultiPolygon"),
            },
        }
    }

    fn line_string(&mut self, n: &Nest) {
        let ps = match positions(n) {
            Ok(ps) => ps,
            Err(b) => return self.bad(b, "o LineString"),
        };
        self.plane(&ps);
        let pts: Vec<Vec2> = ps.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect();
        match pts.len() {
            0 | 1 => self.c.report.skip(
                "Kısa LineString",
                "ikiden az konumu var; çizgi olamaz, alınmadı",
                self.line,
            ),
            2 => self.out.push(Shape::Line(pts[0], pts[1])),
            _ => self.out.push(Shape::Polyline(pts)),
        }
    }

    fn polygon(&mut self, n: &Nest) {
        let rings: Vec<Vec<Pos>> = match n {
            Nest::List(rs) => match rs.iter().map(positions).collect() {
                Ok(v) => v,
                Err(b) => return self.bad(b, "o Polygon"),
            },
            Nest::Bad(b) => return self.bad(*b, "o Polygon"),
            Nest::Pos(_) => return self.bad(Bad::Invalid, "o Polygon"),
        };
        for r in &rings {
            self.plane(r);
        }
        let mut rings = rings
            .into_iter()
            .map(|r| open_ring(r.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect()));
        let Some((outline, closed)) = rings.next() else {
            return self
                .c
                .report
                .skip("Boş Polygon", "halkası yok; alınmadı", self.line);
        };
        if outline.len() < 3 {
            return self.c.report.skip(
                "Kısa halka",
                "dış sınırın üçten az köşesi var; alan olamaz, alınmadı",
                self.line,
            );
        }
        let mut unclosed = !closed;
        let mut holes = Vec::new();
        for (ring, closed) in rings {
            if ring.len() < 3 {
                self.c.report.skip(
                    "Kısa delik",
                    "üçten az köşesi var; alan deliksiz alındı",
                    self.line,
                );
                continue;
            }
            unclosed |= !closed;
            holes.push(ring);
        }
        if unclosed {
            self.c.report.note(
                "Kapanmamış halka",
                "son konumu ilkiyle aynı değil (RFC 7946 kapalı ister); köşeleri olduğu gibi alındı",
                self.line,
            );
        }
        self.out.push(Shape::Polygon(outline, holes));
    }
}

/// A list of positions (a LineString's coordinates, a ring).
fn positions(n: &Nest) -> Result<Vec<Pos>, Bad> {
    match n {
        Nest::List(items) => items
            .iter()
            .map(|i| match i {
                Nest::Pos(p) => Ok(*p),
                Nest::Bad(b) => Err(*b),
                Nest::List(_) => Err(Bad::Invalid),
            })
            .collect(),
        Nest::Bad(b) => Err(*b),
        Nest::Pos(_) => Err(Bad::Invalid),
    }
}

/// Makes a feature's objects (an empty layer: the file's default layer).
fn emit(f: Feature, c: &mut Collect, line: u32) {
    if f.kind.as_deref() != Some("Feature") {
        return c.report.skip(
            "Feature olmayan öğe",
            "features dizisinde type “Feature” olmayan öğe; alınmadı",
            line,
        );
    }
    let g = match &f.geometry {
        None => {
            return c.report.skip(
                "Geometrisi olmayan Feature",
                "geometry üyesi yok; alınmadı",
                line,
            );
        }
        Some(Geom::Null) => {
            return c.report.skip(
                "Geometrisi boş Feature",
                "geometry null; alınacak şekil yok",
                line,
            );
        }
        Some(g) => g,
    };
    let mut s = Shapes {
        out: Vec::new(),
        c: &mut *c,
        line,
    };
    s.geometry(g);
    let out = std::mem::take(&mut s.out);
    if out.is_empty() {
        return;
    }
    let layer = f.layer.clone().unwrap_or_default();
    if f.nested > 0 {
        c.report.note_n(
            "İç içe öznitelik",
            "nesne ya da dizi değeri JSON metni olarak alındı",
            line,
            f.nested,
        );
    }
    if f.nulls > 0 {
        c.report
            .note_n("Boş öznitelik", "değeri null; alınmadı", line, f.nulls);
    }
    for shape in out {
        c.add(shape, &layer, &f.attrs, f.label.as_deref());
    }
}

/// The coordinate system a GeoJSON file declares: none is RFC 7946's WGS 84
/// longitude and latitude; the 2008 `crs` member names an EPSG code (or OGC
/// CRS84, the same axes as RFC 7946). What cannot be read says so, and the
/// app asks the user.
fn declared(crs: Option<&Value>) -> DeclaredCrs {
    let Some(v) = crs else {
        return DeclaredCrs {
            srid: Some(4326),
            text: "RFC 7946: WGS 84 boylam, enlem (dosyada crs üyesi yok)".into(),
            source: CrsSource::Rfc7946,
        };
    };
    let props = v.get("properties");
    let unknown = |text: String| DeclaredCrs {
        srid: None,
        text,
        source: CrsSource::GeoJsonCrs,
    };
    match v.get("type").and_then(Value::as_str) {
        Some("name") => match props.and_then(|p| p.get("name")).and_then(Value::as_str) {
            Some(n) => DeclaredCrs {
                srid: srid_of_name(n),
                text: n.to_string(),
                source: CrsSource::GeoJsonCrs,
            },
            None => unknown("crs üyesinde ad yok".into()),
        },
        Some("EPSG") => {
            let code = match props.and_then(|p| p.get("code")) {
                Some(Value::Num(n)) => n.parse::<u32>().ok(),
                Some(Value::Str(s)) if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) => {
                    s.parse::<u32>().ok()
                }
                _ => None,
            };
            match code {
                Some(c) => DeclaredCrs {
                    srid: Some(c),
                    text: format!("EPSG:{c}"),
                    source: CrsSource::GeoJsonCrs,
                },
                None => unknown("crs üyesinde EPSG kodu okunamadı".into()),
            }
        }
        _ => unknown("crs üyesi tanınmadı (ad ya da EPSG türü değil)".into()),
    }
}

/// The EPSG code of a CRS name ("urn:ogc:def:crs:EPSG::5256", "epsg:5256";
/// OGC CRS84 is WGS 84 longitude, latitude: 4326).
fn srid_of_name(name: &str) -> Option<u32> {
    let u = name.to_ascii_uppercase();
    let head = u.trim_end_matches(|c: char| c.is_ascii_digit());
    if head.len() < u.len() && (head.ends_with("EPSG::") || head.ends_with("EPSG:")) {
        return u[head.len()..].parse::<u32>().ok();
    }
    name.contains("CRS84").then_some(4326)
}

fn utf8(bytes: &[u8]) -> Result<&str, String> {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return Err("Dosya UTF-16 ile yazılmış; GeoJSON UTF-8 olmalıdır (RFC 7946 §1). Dosyayı UTF-8 olarak kaydedip yeniden deneyin.".into());
    }
    std::str::from_utf8(bytes).map_err(|e| {
        format!(
            "Dosya UTF-8 değil ({}. bayt geçersiz); GeoJSON UTF-8 olmalıdır (RFC 7946 §1). Dosyayı UTF-8 olarak kaydedip yeniden deneyin.",
            e.valid_up_to() + 1
        )
    })
}

fn broken(e: JsonError) -> String {
    format!(
        "GeoJSON okunamadı: satır {}: {}. Dosya bozuk ya da yarım kalmış olabilir; hiçbir şey alınmadı.",
        e.line, e.message
    )
}

/// Reads a GeoJSON file (a FeatureCollection, a Feature or a bare geometry).
pub fn read(bytes: &[u8], opts: &GeoJsonReadOptions) -> Result<ImportResult, String> {
    let src = utf8(bytes)?;
    let mut r = Reader::new(src);
    match r.peek() {
        Some(b'{') => {}
        None => return Err("Dosya boş.".into()),
        Some(_) => {
            return Err("GeoJSON değil: dosya bir JSON nesnesiyle ({) başlamıyor.".into());
        }
    }
    let mut c = Collect::new(opts.max_entities);
    let mut kind: Option<String> = None;
    let mut name: Option<String> = None;
    let mut crs: Option<Value> = None;
    let mut collection: Option<bool> = None;
    let mut features = 0u32;
    // A lone Feature's members, and a bare geometry's.
    let mut single = Feature::default();
    let mut coords: Option<Nest> = None;
    let mut parts: Option<Result<Vec<Geom>, ()>> = None;
    r.object(|r, key| {
        match key.as_str() {
            "type" => kind = text(r.value()?),
            "name" => name = text(r.value()?).filter(|s| !s.is_empty()),
            "crs" => crs = Some(r.value()?),
            "features" => {
                if collection.is_some() {
                    return Err(JsonError {
                        line: r.line(),
                        message: "features üyesi iki kez var".into(),
                    });
                }
                if r.peek() != Some(b'[') {
                    r.skip()?;
                    collection = Some(false);
                } else {
                    collection = Some(true);
                    r.array(|r| {
                        features += 1;
                        let line = r.line();
                        if r.peek() != Some(b'{') {
                            r.skip()?;
                            c.report.skip(
                                "Feature olmayan öğe",
                                "features dizisinde nesne olmayan öğe; alınmadı",
                                line,
                            );
                            return Ok(());
                        }
                        let mut f = Feature::default();
                        r.object(|r, key| {
                            if key == "type" {
                                f.kind = text(r.value()?);
                            } else if !feature_member(r, &key, &mut f)? {
                                r.skip()?;
                            }
                            Ok(())
                        })?;
                        emit(f, &mut c, line);
                        Ok(())
                    })?;
                }
            }
            "coordinates" => coords = Some(nest(r)?),
            "geometries" => parts = Some(members(r)?),
            k => {
                if !feature_member(r, k, &mut single)? {
                    r.skip()?;
                }
            }
        }
        Ok(())
    })
    .map_err(broken)?;
    r.end().map_err(broken)?;

    let declared = declared(crs.as_ref());
    let default_layer = name.clone().unwrap_or_else(|| {
        if opts.layer.is_empty() {
            "GeoJSON".to_string()
        } else {
            opts.layer.clone()
        }
    });
    let what = match kind.as_deref() {
        Some("FeatureCollection") => {
            if collection == Some(false) {
                return Err("GeoJSON okunamadı: features üyesi bir dizi değil. Dosya bozuk olabilir; hiçbir şey alınmadı.".into());
            }
            c.report.fact("Feature", features.to_string());
            "GeoJSON FeatureCollection".to_string()
        }
        Some("Feature") => {
            // A features member of a Feature is not its content.
            c = Collect::new(opts.max_entities);
            single.kind = Some("Feature".into());
            emit(single, &mut c, 1);
            "GeoJSON Feature".to_string()
        }
        Some(g) if GEOMETRY_TYPES.contains(&g) => {
            c = Collect::new(opts.max_entities);
            let geom = Geom::Obj {
                kind: Some(g.to_string()),
                coords,
                parts,
            };
            let mut s = Shapes {
                out: Vec::new(),
                c: &mut c,
                line: 1,
            };
            s.geometry(&geom);
            let out = std::mem::take(&mut s.out);
            let none = BTreeMap::new();
            for shape in out {
                c.add(shape, "", &none, None);
            }
            format!("GeoJSON geometri ({g})")
        }
        Some(other) => {
            return Err(format!(
                "GeoJSON değil: type “{other}” tanınmıyor (FeatureCollection, Feature ya da bir geometri türü bekleniyordu)."
            ));
        }
        None => {
            return Err("GeoJSON değil: en dıştaki nesnenin type üyesi yok.".into());
        }
    };
    c.report.fact("Biçim", what);
    if let Some(n) = &name {
        c.report.fact("Ad", n.clone());
    }
    c.report.fact(
        "Koordinat sistemi",
        match declared.srid {
            Some(s) if declared.source == CrsSource::GeoJsonCrs => {
                format!("{} (EPSG:{s})", declared.text)
            }
            _ => declared.text.clone(),
        },
    );
    Ok(c.finish(&default_layer, Some(declared)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::Entity;

    fn opts() -> GeoJsonReadOptions {
        GeoJsonReadOptions {
            layer: "dosya".into(),
            max_entities: 0,
        }
    }

    fn read_str(s: &str) -> ImportResult {
        read(s.as_bytes(), &opts()).expect("geojson")
    }

    #[test]
    fn names_its_crs_or_says_rfc_7946() {
        let cases = [
            (
                r#"{"type":"FeatureCollection","features":[]}"#,
                Some(4326),
                CrsSource::Rfc7946,
            ),
            (
                r#"{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"urn:ogc:def:crs:EPSG::5256"}},"features":[]}"#,
                Some(5256),
                CrsSource::GeoJsonCrs,
            ),
            (
                r#"{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"epsg:2320"}},"features":[]}"#,
                Some(2320),
                CrsSource::GeoJsonCrs,
            ),
            (
                r#"{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"urn:ogc:def:crs:OGC:1.3:CRS84"}},"features":[]}"#,
                Some(4326),
                CrsSource::GeoJsonCrs,
            ),
            (
                r#"{"type":"FeatureCollection","crs":{"type":"EPSG","properties":{"code":32636}},"features":[]}"#,
                Some(32636),
                CrsSource::GeoJsonCrs,
            ),
            (
                r#"{"type":"FeatureCollection","crs":{"type":"EPSG","properties":{"code":"5254"}},"features":[]}"#,
                Some(5254),
                CrsSource::GeoJsonCrs,
            ),
            (
                r#"{"type":"FeatureCollection","crs":{"type":"link","properties":{"href":"x"}},"features":[]}"#,
                None,
                CrsSource::GeoJsonCrs,
            ),
            (
                r#"{"type":"FeatureCollection","crs":null,"features":[]}"#,
                None,
                CrsSource::GeoJsonCrs,
            ),
            (
                r#"{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"TUREF / TM30"}},"features":[]}"#,
                None,
                CrsSource::GeoJsonCrs,
            ),
        ];
        for (text, srid, source) in cases {
            let d = read_str(text).declared_crs.expect("declared");
            assert_eq!((d.srid, d.source), (srid, source), "{text}");
        }
    }

    #[test]
    fn every_kind_of_geometry_becomes_objects_in_order() {
        let r = read_str(
            r#"{"type":"FeatureCollection","name":"Parseller","features":[
              {"type":"Feature","properties":{"ad":"P1","no":1.50,"ok":true,"yok":null,"iç":{"a":[1,"x"]}},"geometry":{"type":"Point","coordinates":[1,2,3]}},
              {"type":"Feature","kentos":{"layer":"yol","label":"Y1"},"properties":null,"geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]}},
              {"type":"Feature","geometry":{"coordinates":[[[0,0],[4,0],[4,4],[0,4],[0,0]],[[1,1],[2,1],[2,2],[1,1]],[[3,3],[3,3]]],"type":"Polygon"}},
              {"type":"Feature","geometry":{"type":"GeometryCollection","geometries":[{"type":"MultiPoint","coordinates":[[5,5],[6,"x"],[7,7,1e999]]},{"type":"Circle","coordinates":[0,0]}]}},
              {"type":"Feature","geometry":null},
              {"type":"Feature","geometry":{"type":"LineString","coordinates":[[0,0,9],[1,0],[2,1e999]]}}
            ]}"#,
        );
        let kinds: Vec<&str> = r
            .entities
            .iter()
            .map(|e| match e {
                Entity::Point(_) => "point",
                Entity::Line(_) => "line",
                Entity::Polygon(_) => "polygon",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["point", "line", "polygon", "point"]);
        let Entity::Point(p) = &r.entities[0] else {
            panic!()
        };
        assert_eq!((p.p.x, p.p.y, p.z), (1.0, 2.0, Some(3.0)));
        assert_eq!(p.base.layer_id, "Parseller");
        let attrs: Vec<(&str, &str)> = p
            .base
            .attrs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("ad", "P1"),
                ("iç", "{\"a\":[1,\"x\"]}"),
                ("no", "1.50"),
                ("ok", "true")
            ]
        );
        let Entity::Line(l) = &r.entities[1] else {
            panic!()
        };
        assert_eq!(
            (l.base.layer_id.as_str(), l.base.label.as_deref()),
            ("yol", Some("Y1"))
        );
        let Entity::Polygon(g) = &r.entities[2] else {
            panic!()
        };
        assert_eq!(g.pts.len(), 4);
        assert_eq!(g.holes.as_ref().map(|h| h.len()), Some(1));
        let skipped: Vec<&str> = r.report.skipped.iter().map(|i| i.what.as_str()).collect();
        assert!(skipped.contains(&"Kısa delik"), "{skipped:?}");
        assert!(skipped.contains(&"Geçersiz koordinat"), "{skipped:?}");
        assert!(skipped.contains(&"Sonlu olmayan koordinat"), "{skipped:?}");
        assert!(skipped.contains(&"Geometri türü “Circle”"), "{skipped:?}");
        assert!(skipped.contains(&"Geometrisi boş Feature"), "{skipped:?}");
        let noted: Vec<&str> = r.report.notes.iter().map(|i| i.what.as_str()).collect();
        assert!(
            noted.contains(&"İç içe öznitelik") && noted.contains(&"Boş öznitelik"),
            "{noted:?}"
        );
        assert_eq!(
            r.layers
                .iter()
                .map(|l| (l.name.as_str(), l.count))
                .collect::<Vec<_>>(),
            vec![("Parseller", 3), ("yol", 1)]
        );
    }

    #[test]
    fn a_lone_feature_or_geometry_is_read_whatever_the_member_order() {
        let r = read_str(
            r#"{"geometry":{"coordinates":[[1,2],[3,4],[5,6]],"type":"LineString"},"properties":{"a":"b"},"type":"Feature"}"#,
        );
        assert_eq!(r.entities.len(), 1);
        assert_eq!(r.layers[0].name, "dosya");
        let r = read_str(r#"{"coordinates":[10.5,20.25],"type":"Point"}"#);
        assert_eq!(r.entities.len(), 1);
    }

    #[test]
    fn broken_or_foreign_files_are_refused_with_the_reason() {
        for (bytes, why) in [
            (&b""[..], "boş"),
            (b"[1,2]", "başlamıyor"),
            (b"{\"type\":\"Topology\"}", "Topology"),
            (b"{\"features\":[]}", "type üyesi yok"),
            (
                b"{\"type\":\"FeatureCollection\",\"features\":{}}",
                "bir dizi değil",
            ),
            (
                b"{\"type\":\"FeatureCollection\",\"features\":[{\"type\":\"Feature\"",
                "satır 1",
            ),
            (
                b"{\"type\":\"FeatureCollection\",\"features\":[],\"features\":[]}",
                "iki kez",
            ),
            (b"\xFF\xFE{\0}\0", "UTF-16"),
            (
                b"{\"type\":\"Point\",\"coordinates\":[1,2],\"x\":\"\xC3\x28\"}",
                "UTF-8 değil",
            ),
        ] {
            let e = read(bytes, &opts()).expect_err(why);
            assert!(e.contains(why), "{why}: {e}");
        }
    }

    #[test]
    fn the_object_limit_stops_the_objects_not_the_reading() {
        let mut text = String::from("{\"type\":\"FeatureCollection\",\"features\":[");
        for i in 0..50 {
            if i > 0 {
                text.push(',');
            }
            text.push_str(&format!("{{\"type\":\"Feature\",\"geometry\":{{\"type\":\"Point\",\"coordinates\":[{i},0]}}}}"));
        }
        text.push_str("]}");
        let r = read(
            text.as_bytes(),
            &GeoJsonReadOptions {
                layer: "a".into(),
                max_entities: 10,
            },
        )
        .expect("geojson");
        assert_eq!(r.entities.len(), 10);
        let limit = r
            .report
            .skipped
            .iter()
            .find(|i| i.what == "Nesne sınırı")
            .expect("limit");
        assert_eq!(limit.count, 40);
    }
}
