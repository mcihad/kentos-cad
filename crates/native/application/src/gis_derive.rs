//! The drawing as a traceable GIS dataset (docs/adr/0530, plan/gis.md CBS-16): the conversion `file.export.gis` and
//! `cad.gis.derivation` use, over the `kentos-geopackage` crate. Nothing here writes the document.
//!
//! The drawing is the source and the dataset a derivative of it. Each object becomes at most one row; a row carries
//! its source's lasting id (`kentos_uid`, and `kentos_uids` as every KentOS GeoPackage), its kind (`kentos_kind`),
//! how far its chords are from the curve it stands for (`kentos_deviation`, metres) and the digest of the source as
//! it was (`kentos_digest`). Geometry is only straight: a circle, an arc, a polyline's bulged edge, an ellipse, a
//! fit-point curve and a NURBS are chords within the tolerance (`kentos_geometry_core::ops::derive`), Z and M
//! carried along them. Texts and blocks are not features of the layers: with `annotations: table` they go to their
//! own tables (a block's insertions as points with the block's attributes as columns, texts as points with their
//! words); everything else a GIS has no geometry of the same meaning for is left out and counted with a reason.
//!
//! Each table records its derivation in the GeoPackage's metadata (`DerivationRecord`), so a later reader can tell
//! the geometry is a derivative and `check` can tell whether it still is the drawing's: a row's source digest is the
//! SHA-256 of the object's canonical JSON without its look (and, for an insert, its block's attribute definitions).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kentos_commands::canonical::canonical_json;
use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    DERIVATION_DIGEST, DERIVATION_FORMAT, DERIVATION_LIMIT_DEFAULT, DERIVATION_LIMIT_MAX,
    DERIVATION_STANDARD, DerivationEntry, DerivationLayer, DerivationRecord, DerivationSource,
    DerivationStatus, DerivationTable, Entity, GisAnnotations, GisDerived, GisLeft,
    GisTableExported, GisTableRole,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::jsmath::TAU;
use kentos_geometry_core::ops::derive as chords;
use kentos_geometry_core::vec2::Vec2 as CorePoint;
use kentos_geopackage::{
    self as gpkg, ColumnKind, Coord, Curve, DataType, GeoPackage, Geometry, Query, Value,
};
use serde_json::Value as J;

use crate::geopackage as gp;

/// What a derivation is asked for.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// The farthest a chord may come from its curve (m).
    pub tolerance: f64,
    pub annotations: GisAnnotations,
}

/// A column's value that is not an attribute: a text or a number.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Text(Option<String>),
    Real(f64),
}

/// One object as a row of the derived dataset.
#[derive(Clone, Debug)]
pub struct DerivedRow {
    pub role: GisTableRole,
    /// Which table of its role: the layer's id, the block's name, empty for the texts.
    pub key: String,
    /// The source's kind (`circle`, `polyline` …).
    pub kind: &'static str,
    pub geometry: Option<Geometry>,
    /// The farthest the row's chords come from the curve (m); 0 for a source's own points.
    pub deviation: f64,
    /// How the deviation is known: `exact`, `bound`, `sampled`, or `none`.
    pub basis: &'static str,
    /// Whether the geometry is chords of a curve.
    pub chorded: bool,
    /// The columns of the row that are not attributes (a block's attributes, a text's words), in their order.
    pub extras: Vec<(String, Cell)>,
}

/// What an object comes to.
pub enum Outcome {
    Row(Box<DerivedRow>),
    /// Not written: its kind and why.
    Left {
        kind: &'static str,
        reason: &'static str,
    },
}

fn cv(p: &kentos_commands::Vec2) -> CorePoint {
    CorePoint::new(p.x, p.y)
}

fn at<T: Copy>(v: &Option<Vec<Option<T>>>, i: usize) -> Option<T> {
    v.as_ref().and_then(|v| v.get(i).copied().flatten())
}

/// `a` to `b` at fraction `f`; at a vertex (`f` 0) its own value, between two without one of them nothing.
fn lerp(a: Option<f64>, b: Option<f64>, f: f64) -> Option<f64> {
    if f == 0.0 {
        return a;
    }
    match (a, b) {
        (Some(x), Some(y)) => Some(x + (y - x) * f),
        _ => None,
    }
}

fn coord(p: &kentos_commands::Vec2, z: Option<f64>, m: Option<f64>) -> Coord {
    Coord {
        x: p.x,
        y: p.y,
        z,
        m,
    }
}

fn core_coord(p: CorePoint) -> Coord {
    Coord::xy(p.x, p.y)
}

/// A path as coordinates, its arcs chords, Z and M taken along each edge; a closed path's ring does not repeat its
/// first point. The deviation is the farthest any chord comes from its arc.
fn path(
    pts: &[kentos_commands::Vec2],
    bulges: Option<&Vec<f64>>,
    zs: &Option<Vec<Option<f64>>>,
    ms: &Option<Vec<Option<f64>>>,
    closed: bool,
    tol: f64,
) -> (Vec<Coord>, f64) {
    let core: Vec<CorePoint> = pts.iter().map(cv).collect();
    let run = chords::path_chords(&core, bulges.map(Vec::as_slice), closed, tol);
    let n = pts.len();
    let coords = run
        .pts
        .iter()
        .zip(&run.at)
        .map(|(p, (i, f))| {
            let j = (i + 1) % n;
            Coord {
                x: p.x,
                y: p.y,
                z: lerp(at(zs, *i), at(zs, j), *f),
                m: lerp(at(ms, *i), at(ms, j), *f),
            }
        })
        .collect();
    (coords, run.deviation)
}

/// A closed ring as a line that repeats its first point.
fn closed_line(mut coords: Vec<Coord>) -> Curve {
    if let Some(first) = coords.first().copied() {
        coords.push(first);
    }
    Curve::Line(coords)
}

/// An area's outer ring and holes as closed lines.
fn rings(
    pts: &[kentos_commands::Vec2],
    bulges: Option<&Vec<f64>>,
    zs: &Option<Vec<Option<f64>>>,
    holes: Option<&Vec<kentos_commands::RingGeometry>>,
    tol: f64,
) -> (Vec<Curve>, f64) {
    let (outer, mut dev) = path(pts, bulges, zs, &None, true, tol);
    let mut out = vec![closed_line(outer)];
    for h in holes.into_iter().flatten() {
        let (ring, d) = path(&h.pts, h.bulges.as_ref(), &h.zs, &None, true, tol);
        dev = if d > dev { d } else { dev };
        out.push(closed_line(ring));
    }
    (out, dev)
}

/// A closed curve's points as a polygon (its ring closed, a repeated last point dropped first), an open one's as a line.
fn ring_or_line(mut pts: Vec<CorePoint>, closed: bool) -> Option<Geometry> {
    if pts.len() < 2 {
        return None;
    }
    if closed {
        if pts.len() > 2
            && let (Some(f), Some(l)) = (pts.first(), pts.last())
            && f.x == l.x
            && f.y == l.y
        {
            pts.pop();
        }
        if pts.len() < 3 {
            return None;
        }
        let coords: Vec<Coord> = pts.into_iter().map(core_coord).collect();
        Some(Geometry::Polygon(vec![closed_line(coords)]))
    } else {
        Some(Geometry::Curve(Curve::Line(
            pts.into_iter().map(core_coord).collect(),
        )))
    }
}

/// Why an object kind is not a row of the dataset.
fn left(e: &Entity) -> (&'static str, &'static str) {
    let kind = e.kind();
    let reason = match e {
        Entity::Dimension(_) => {
            "ölçü: ölçülen değer çizimde kaynağından hesaplanır, GIS'te geometrisi ve anlamı yok"
        }
        Entity::Hatch(_) => {
            "tarama: bir desen ve sınır, GIS alanı değil; sınırı çizimde alan olarak varsa o yazılır"
        }
        Entity::Leader(_) => "kılavuz: ok ve not, GIS'te geometrisi yok",
        Entity::Table(_) => "tablo: hücreleri çizimde, GIS'te karşılığı yok",
        Entity::Image(_) | Entity::Raster(_) | Entity::PointCloud(_) => {
            "resim, raster ve nokta bulutu: kendi dosyaları var, vektör özelliği değil"
        }
        Entity::Xline(_) | Entity::Ray(_) => "yardımcı çizgi: sonsuz, GIS'te geometrisi yok",
        Entity::Text(_) | Entity::Insert(_) => "annotations: skip verildi",
        Entity::Wall(_)
        | Entity::Opening(_)
        | Entity::Space(_)
        | Entity::Grid(_)
        | Entity::Column(_)
        | Entity::Stair(_)
        | Entity::Ramp(_)
        | Entity::Tag(_)
        | Entity::ViewMark(_) => "mimari nesne: bu dilimde GIS türevi yok",
        _ => "bu türün GIS türevi yok",
    };
    (kind, reason)
}

fn plain(
    e: &Entity,
    kind: &'static str,
    geometry: Option<Geometry>,
    deviation: f64,
    basis: &'static str,
    chorded: bool,
) -> Outcome {
    Outcome::Row(Box::new(DerivedRow {
        role: GisTableRole::Layer,
        key: e.base().layer_id.clone(),
        kind,
        geometry,
        deviation,
        basis,
        chorded,
        extras: Vec::new(),
    }))
}

/// An object as a row of the derived dataset, or why it is not one.
pub fn derive_entity(doc: &Document, e: &Entity, o: &Options) -> Outcome {
    let tol = o.tolerance;
    let kind = e.kind();
    match e {
        Entity::Record(_) => plain(e, kind, None, 0.0, "none", false),
        Entity::Point(p) => {
            let first = coord(&p.p, p.z, None);
            let g = match &p.parts {
                Some(parts) if !parts.is_empty() => {
                    let mut all = vec![first];
                    all.extend(parts.iter().map(|q| coord(&q.p, q.z, None)));
                    Geometry::MultiPoint(all)
                }
                _ => Geometry::Point(Some(first)),
            };
            plain(e, kind, Some(g), 0.0, "none", false)
        }
        Entity::Line(l) => plain(
            e,
            kind,
            Some(Geometry::Curve(Curve::Line(vec![
                coord(&l.a, l.za, l.ma),
                coord(&l.b, l.zb, l.mb),
            ]))),
            0.0,
            "none",
            false,
        ),
        Entity::Polyline(p) => {
            let one = |pts: &[kentos_commands::Vec2],
                       bulges: Option<&Vec<f64>>,
                       zs: &Option<Vec<Option<f64>>>,
                       ms: &Option<Vec<Option<f64>>>,
                       closed: bool| {
                let (mut coords, dev) = path(pts, bulges, zs, ms, closed, tol);
                if closed && let Some(f) = coords.first().copied() {
                    coords.push(f);
                }
                (Curve::Line(coords), dev)
            };
            let (first, mut dev) = one(&p.pts, p.bulges.as_ref(), &p.zs, &p.ms, p.closed);
            let g = match &p.parts {
                Some(parts) if !parts.is_empty() => {
                    let mut all = vec![first];
                    for q in parts {
                        let (c, d) = one(&q.pts, q.bulges.as_ref(), &q.zs, &q.ms, false);
                        dev = if d > dev { d } else { dev };
                        all.push(c);
                    }
                    Geometry::MultiCurve(all)
                }
                _ => Geometry::Curve(first),
            };
            plain(
                e,
                kind,
                Some(g),
                dev,
                if dev > 0.0 { "exact" } else { "none" },
                dev > 0.0,
            )
        }
        Entity::Polygon(p) => {
            let (first, mut dev) = rings(&p.pts, p.bulges.as_ref(), &p.zs, p.holes.as_ref(), tol);
            let g = match &p.parts {
                Some(parts) if !parts.is_empty() => {
                    let mut all = vec![first];
                    for q in parts {
                        let (r, d) = rings(&q.pts, q.bulges.as_ref(), &q.zs, q.holes.as_ref(), tol);
                        dev = if d > dev { d } else { dev };
                        all.push(r);
                    }
                    Geometry::MultiPolygon(all)
                }
                _ => Geometry::Polygon(first),
            };
            plain(
                e,
                kind,
                Some(g),
                dev,
                if dev > 0.0 { "exact" } else { "none" },
                dev > 0.0,
            )
        }
        Entity::Circle(c) => {
            if !(c.r > 0.0 && c.r.is_finite()) {
                return Outcome::Left {
                    kind,
                    reason: "yarıçapı geçerli değil",
                };
            }
            let run = chords::arc_chords(cv(&c.c), c.r, 0.0, TAU, tol);
            let n = run.pts.len() - 1;
            let ring: Vec<Coord> = run.pts[..n].iter().map(|p| core_coord(*p)).collect();
            plain(
                e,
                kind,
                Some(Geometry::Polygon(vec![closed_line(ring)])),
                run.deviation,
                "exact",
                true,
            )
        }
        Entity::Arc(a) => {
            if !(a.r > 0.0 && a.r.is_finite()) {
                return Outcome::Left {
                    kind,
                    reason: "yarıçapı geçerli değil",
                };
            }
            let mut sweep = (a.a1 - a.a0).rem_euclid(TAU);
            if sweep == 0.0 {
                sweep = TAU;
            }
            let run = chords::arc_chords(cv(&a.c), a.r, a.a0, sweep, tol);
            plain(
                e,
                kind,
                Some(Geometry::Curve(Curve::Line(
                    run.pts.iter().map(|p| core_coord(*p)).collect(),
                ))),
                run.deviation,
                "exact",
                true,
            )
        }
        Entity::Ellipse(el) => {
            let g = kentos_geometry_core::entity::ellipse_geom(
                cv(&el.c),
                cv(&el.major),
                el.ratio,
                el.t0,
                el.t1,
            );
            let full = kentos_geometry_core::geom::ellipse::is_full_ellipse(&g);
            match ring_or_line(chords::ellipse_chords(&g, tol), full) {
                Some(geometry) => plain(e, kind, Some(geometry), tol, "bound", true),
                None => Outcome::Left {
                    kind,
                    reason: "elipsin kirişleri çıkmadı",
                },
            }
        }
        Entity::Spline(s) => {
            let pts: Vec<CorePoint> = s.pts.iter().map(cv).collect();
            match ring_or_line(chords::spline_chords(&pts, s.closed, tol), s.closed) {
                Some(geometry) => plain(e, kind, Some(geometry), tol, "bound", true),
                None => Outcome::Left {
                    kind,
                    reason: "eğrinin kirişleri çıkmadı",
                },
            }
        }
        Entity::Nurbs(n) => {
            let shape = crate::geometry::shape(e);
            match kentos_geometry_core::geom::nurbs::of(&shape) {
                Some(curve) => match ring_or_line(curve.tessellate(tol), n.closed) {
                    Some(geometry) => plain(e, kind, Some(geometry), tol, "sampled", true),
                    None => Outcome::Left {
                        kind,
                        reason: "NURBS'ın kirişleri çıkmadı",
                    },
                },
                None => Outcome::Left {
                    kind,
                    reason: "NURBS'ın sayıları bir eğri yapmıyor",
                },
            }
        }
        Entity::Text(t) if o.annotations == GisAnnotations::Table => {
            Outcome::Row(Box::new(DerivedRow {
                role: GisTableRole::Annotations,
                key: String::new(),
                kind,
                geometry: Some(Geometry::Point(Some(coord(&t.p, None, None)))),
                deviation: 0.0,
                basis: "none",
                chorded: false,
                extras: vec![
                    ("text".into(), Cell::Text(Some(t.text.clone()))),
                    ("height".into(), Cell::Real(t.height)),
                    ("rotation_deg".into(), Cell::Real(t.rotation)),
                ],
            }))
        }
        Entity::Insert(i) if o.annotations == GisAnnotations::Table => {
            let Some(def) = doc.block(i.block) else {
                return Outcome::Left {
                    kind,
                    reason: "yerleştirmenin bloğu çizimde yok",
                };
            };
            let attrs = &i.base.attrs;
            let mut extras = vec![
                ("scale".to_owned(), Cell::Real(i.scale)),
                (
                    "rotation_deg".to_owned(),
                    Cell::Real(i.rotation.to_degrees()),
                ),
            ];
            for a in &def.attributes {
                let value = attrs.get(&a.tag).cloned().or_else(|| a.value.clone());
                if !extras.iter().any(|(k, _)| k == &a.tag) {
                    extras.push((a.tag.clone(), Cell::Text(value)));
                }
            }
            Outcome::Row(Box::new(DerivedRow {
                role: GisTableRole::Blocks,
                key: def.name.clone(),
                kind,
                geometry: Some(Geometry::Point(Some(coord(&i.p, None, None)))),
                deviation: 0.0,
                basis: "none",
                chorded: false,
                extras,
            }))
        }
        other => {
            let (kind, reason) = left(other);
            Outcome::Left { kind, reason }
        }
    }
}

/// The keys of an object's JSON that are its look or its display, not what a GIS row is made of.
const LOOK_KEYS: [&str; 15] = [
    "id",
    "color",
    "lineWeight",
    "label",
    "symbol",
    "labelPins",
    "attachments",
    "lineType",
    "linePattern",
    "lineTypeScale",
    "transparency",
    "order",
    "byLayer",
    "wipeout",
    "annotative",
];

/// The source's digest (`DERIVATION_DIGEST`): SHA-256 of the object's canonical JSON without its look; for an
/// insert with its block's name and attribute definitions.
pub fn digest(doc: &Document, e: &Entity) -> String {
    let mut v = serde_json::to_value(e).unwrap_or(J::Null);
    if let J::Object(m) = &mut v {
        for k in LOOK_KEYS {
            m.remove(k);
        }
        if let Entity::Insert(i) = e
            && let Some(def) = doc.block(i.block)
        {
            m.insert(
                "definition".into(),
                serde_json::json!({
                    "name": def.name,
                    "attributes": serde_json::to_value(&def.attributes).unwrap_or(J::Null),
                }),
            );
        }
    }
    sha256_hex(canonical_json(&v).as_bytes())
}

// ── Writing ─────────────────────────────────────────────────────────────────

/// A column of a derived table and where its values come from.
#[derive(Clone, Debug)]
enum Key {
    Uid,
    Kind,
    Deviation,
    Digest,
    Layer,
    Extra(String),
    Attr(String),
}

struct Cols {
    columns: Vec<gpkg::Column>,
    keys: Vec<Key>,
    taken: BTreeSet<String>,
    renamed: Vec<(String, String)>,
}

impl Cols {
    fn new() -> Cols {
        Cols {
            columns: Vec::new(),
            keys: Vec::new(),
            taken: ["fid".to_owned(), "geom".to_owned()].into(),
            renamed: Vec::new(),
        }
    }

    fn add(&mut self, name: &str, kind: ColumnKind, key: Key) {
        let mut col = name.to_owned();
        let mut n = 2;
        while !self.taken.insert(col.to_ascii_lowercase()) {
            col = format!("{name}_{n}");
            n += 1;
        }
        if col != name {
            self.renamed.push((name.to_owned(), col.clone()));
        }
        self.keys.push(key);
        self.columns.push(gpkg::Column {
            name: col,
            decl: String::new(),
            kind,
            not_null: false,
            default: None,
        });
    }
}

/// What [`write`] wrote.
#[derive(Clone, Debug, Default)]
pub struct Written {
    pub tables: Vec<GisTableExported>,
    pub left: Vec<GisLeft>,
    /// A column written under another name: table, from, to.
    pub renamed: Vec<(String, String, String)>,
    pub chorded: u32,
    pub max_deviation: f64,
}

struct Group<'a> {
    role: GisTableRole,
    key: String,
    rows: Vec<(Uuid, &'a Entity, DerivedRow, String)>,
}

fn utc_now() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64);
    let text = kentos_geometry_core::time::write(ms, false);
    let text = if text.len() == 10 {
        format!("{text}T00:00:00")
    } else {
        text
    };
    let text = if text.len() == 19 {
        format!("{text}.000")
    } else {
        text
    };
    format!("{text}Z")
}

fn real(v: f64) -> Value {
    Value::Real(v)
}

/// Writes the layers (by id, leaves; every layer with objects when empty) to a new GeoPackage at `path` as a derived
/// dataset (docs/adr/0530 §2).
pub fn write(
    doc: &Document,
    layers: &[String],
    o: &Options,
    path: &Path,
) -> Result<Written, String> {
    let ids: Vec<String> = if layers.is_empty() {
        doc.layers().leaves().iter().map(|n| n.id.clone()).collect()
    } else {
        layers.to_vec()
    };
    let mut groups: Vec<Group<'_>> = Vec::new();
    let mut index: BTreeMap<(u8, String), usize> = BTreeMap::new();
    let mut left: BTreeMap<&'static str, (u32, &'static str)> = BTreeMap::new();
    for id in &ids {
        let Some(node) = doc.layers().get(id) else {
            continue;
        };
        for (uid, e) in doc.by_layer_with_uids(id) {
            match derive_entity(doc, e, o) {
                Outcome::Row(row) => {
                    let slot = (row.role as u8, row.key.clone());
                    let at = *index.entry(slot).or_insert_with(|| {
                        groups.push(Group {
                            role: row.role,
                            key: row.key.clone(),
                            rows: Vec::new(),
                        });
                        groups.len() - 1
                    });
                    groups[at].rows.push((uid, e, *row, node.name.clone()));
                }
                Outcome::Left { kind, reason } => {
                    let entry = left.entry(kind).or_insert((0, reason));
                    entry.0 += 1;
                }
            }
        }
    }
    let mut gp_file = GeoPackage::create(path).map_err(|e| e.0)?;
    let srs = gp::srs_of(doc);
    gp_file.add_srs(&srs).map_err(|e| e.0)?;
    let mut out = Written {
        left: left
            .into_iter()
            .map(|(kind, (count, reason))| GisLeft {
                kind: kind.to_owned(),
                count,
                reason: reason.to_owned(),
            })
            .collect(),
        ..Written::default()
    };
    let mut names: BTreeSet<String> = BTreeSet::new();
    let made = utc_now();
    for g in groups {
        let first_layer = if g.role == GisTableRole::Layer {
            doc.layers().get(&g.key)
        } else {
            None
        };
        let mut table = match g.role {
            GisTableRole::Layer => {
                first_layer.map_or_else(|| g.key.clone(), |n| n.name.trim().to_owned())
            }
            GisTableRole::Blocks => format!("blok_{}", g.key),
            GisTableRole::Annotations => "kentos_annotations".to_owned(),
        };
        if table.is_empty()
            || table.to_ascii_lowercase().starts_with("gpkg_")
            || table.starts_with("rtree_")
            || table.to_ascii_lowercase().starts_with("kentos_uids")
        {
            table = format!("katman_{table}");
        }
        let base = table.clone();
        let mut k = 2;
        while !names.insert(table.to_ascii_lowercase()) {
            table = format!("{base}_{k}");
            k += 1;
        }
        let mut geoms: Vec<Option<Geometry>> =
            g.rows.iter().map(|r| r.2.geometry.clone()).collect();
        let type_name = gp::fit(&mut geoms);
        let (mut z, mut m) = (false, false);
        let (mut all_z, mut all_m) = (true, true);
        for geometry in geoms.iter().flatten() {
            let (gz, gm) = geometry.dims();
            z |= gz;
            m |= gm;
            all_z &= gz;
            all_m &= gm;
        }
        let flag = |any: bool, all: bool| {
            if !any {
                0
            } else if all {
                1
            } else {
                2
            }
        };
        // The columns: the derivation's own, then the role's, the layer's fields, the other attributes.
        let mut cols = Cols::new();
        cols.add("kentos_uid", ColumnKind::Text(None), Key::Uid);
        cols.add("kentos_kind", ColumnKind::Text(None), Key::Kind);
        cols.add("kentos_deviation", ColumnKind::Real, Key::Deviation);
        cols.add("kentos_digest", ColumnKind::Text(Some(64)), Key::Digest);
        if g.role != GisTableRole::Layer {
            cols.add("kentos_layer", ColumnKind::Text(None), Key::Layer);
        }
        let mut extra_names: Vec<(String, bool)> = Vec::new();
        for (_, _, row, _) in &g.rows {
            for (n, c) in &row.extras {
                if !extra_names.iter().any(|(k, _)| k == n) {
                    extra_names.push((n.clone(), matches!(c, Cell::Real(_))));
                }
            }
        }
        for (n, is_real) in &extra_names {
            let kind = if *is_real {
                ColumnKind::Real
            } else {
                ColumnKind::Text(None)
            };
            cols.add(n, kind, Key::Extra(n.clone()));
        }
        if let Some(node) = first_layer {
            for f in &node.fields {
                cols.add(&f.name, gp::column_of(f).kind, Key::Attr(f.name.clone()));
            }
        }
        let mut extra_attrs: BTreeSet<String> = BTreeSet::new();
        for (_, e, _, _) in &g.rows {
            extra_attrs.extend(e.base().attrs.keys().cloned());
        }
        for k in extra_attrs {
            if extra_names.iter().any(|(n, _)| n == &k)
                || cols
                    .keys
                    .iter()
                    .any(|key| matches!(key, Key::Attr(a) if a == &k))
            {
                continue;
            }
            cols.add(&k, ColumnKind::Text(None), Key::Attr(k.clone()));
        }
        for (from, to) in &cols.renamed {
            out.renamed.push((table.clone(), from.clone(), to.clone()));
        }
        let mut max_dev = 0.0f64;
        let mut chorded = 0u32;
        let mut kinds: BTreeMap<String, String> = BTreeMap::new();
        let rows: Vec<gpkg::NewRow> = g
            .rows
            .iter()
            .zip(geoms)
            .map(|((uid, e, row, layer_name), geometry)| {
                if row.chorded {
                    chorded += 1;
                }
                if row.deviation > max_dev {
                    max_dev = row.deviation;
                }
                kinds
                    .entry(row.kind.to_owned())
                    .or_insert_with(|| row.basis.to_owned());
                let values = cols
                    .columns
                    .iter()
                    .zip(&cols.keys)
                    .map(|(c, key)| match key {
                        Key::Uid => Value::Text(uid.to_string()),
                        Key::Kind => Value::Text(row.kind.to_owned()),
                        Key::Deviation => real(row.deviation),
                        Key::Digest => Value::Text(digest(doc, e)),
                        Key::Layer => Value::Text(layer_name.clone()),
                        Key::Extra(n) => match row.extras.iter().find(|(k, _)| k == n) {
                            Some((_, Cell::Text(Some(t)))) => Value::Text(t.clone()),
                            Some((_, Cell::Real(r))) => real(*r),
                            _ => Value::Null,
                        },
                        Key::Attr(k) => gp::cell(e.base().attrs.get(k), &c.kind),
                    })
                    .collect();
                gpkg::NewRow {
                    fid: None,
                    geometry,
                    values,
                }
            })
            .collect();
        let uids: Vec<String> = g.rows.iter().map(|r| r.0.to_string()).collect();
        let def = gpkg::TableDef {
            name: table.clone(),
            data_type: if type_name.is_some() {
                DataType::Features
            } else {
                DataType::Attributes
            },
            identifier: Some(table.clone()),
            description: Some("KentOS çiziminin türevi (kentos.derivation)".to_owned()),
            srs_id: type_name.as_ref().map(|_| srs.id),
            geometry: type_name.clone().map(|t| gpkg::GeometryColumn {
                name: "geom".into(),
                type_name: t,
                z: flag(z, all_z),
                m: flag(m, all_m),
            }),
            columns: cols.columns,
        };
        gp_file
            .write_table(&def, &rows, Some(&uids))
            .map_err(|e| e.0)?;
        let record = DerivationRecord {
            format: DERIVATION_FORMAT.to_owned(),
            version: 1,
            table: table.clone(),
            role: g.role,
            layer: first_layer.map(|n| DerivationLayer {
                id: n.id.clone(),
                name: n.name.clone(),
            }),
            block: (g.role == GisTableRole::Blocks).then(|| g.key.clone()),
            layers: ids.clone(),
            algorithm: chords::ALGORITHM.to_owned(),
            tolerance: o.tolerance,
            unit: "m".to_owned(),
            source: source_of(doc),
            rows: rows.len() as u32,
            max_deviation: max_dev,
            kinds,
            renamed: cols
                .renamed
                .iter()
                .map(|(from, to)| (to.clone(), from.clone()))
                .collect(),
            digest: DERIVATION_DIGEST.to_owned(),
            engine: format!("KentOS {}", env!("CARGO_PKG_VERSION")),
            made: made.clone(),
        };
        gp_file
            .add_metadata(
                &table,
                DERIVATION_STANDARD,
                "application/json",
                &serde_json::to_string(&record).unwrap_or_default(),
                &made,
            )
            .map_err(|e| e.0)?;
        out.chorded += chorded;
        if max_dev > out.max_deviation {
            out.max_deviation = max_dev;
        }
        out.tables.push(GisTableExported {
            table,
            role: g.role,
            source: match g.role {
                GisTableRole::Layer => Some(g.key.clone()),
                GisTableRole::Blocks => Some(g.key.clone()),
                GisTableRole::Annotations => None,
            },
            rows: rows.len() as u32,
            geometry: type_name,
            chorded,
            max_deviation: max_dev,
        });
    }
    gp_file.close().map_err(|e| e.0)?;
    Ok(out)
}

fn source_of(doc: &Document) -> DerivationSource {
    DerivationSource {
        project: doc.project_id().map(|u| u.to_string()),
        name: doc.name().to_owned(),
        revision: doc.revision().to_string(),
        srid: doc.settings().srid,
    }
}

// ── Reading a derivation back ───────────────────────────────────────────────

/// The derivation record `table` of an open file carries, none when it has none or one this version does not read.
pub fn record_of(file: &GeoPackage, table: &str) -> Option<DerivationRecord> {
    file.metadata(table)
        .ok()?
        .into_iter()
        .filter(|(standard, _)| standard == DERIVATION_STANDARD)
        .filter_map(|(_, text)| serde_json::from_str::<DerivationRecord>(&text).ok())
        .find(|r| r.problem().is_none())
}

/// The derivation record of `table` of the file at `path`, none when the file is not readable or the table is not a
/// derivative (docs/adr/0530 §4: a derivative's table is never bound).
pub fn derived_table(path: &Path, table: &str) -> Option<DerivationRecord> {
    let file = GeoPackage::open(path, false).ok()?;
    record_of(&file, table)
}

/// Whether any of the file's tables is a derivative.
pub fn derived_tables(path: &Path) -> Vec<(String, DerivationRecord)> {
    let Ok(file) = GeoPackage::open(path, false) else {
        return Vec::new();
    };
    let Ok(layers) = file.layers() else {
        return Vec::new();
    };
    layers
        .into_iter()
        .filter_map(|l| record_of(&file, &l.name).map(|r| (l.name, r)))
        .collect()
}

/// What a check could not do: its code and message.
pub type Refusal = (&'static str, String);

fn column_of(layer: &gpkg::Layer, name: &str) -> Option<usize> {
    layer.columns.iter().position(|c| c.name == name)
}

fn text_at(row: &gpkg::Row, i: Option<usize>) -> Option<String> {
    match row.values.get(i?)? {
        Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

/// A geometry with the Z and M the file wrote as NaN (a vertex without one in a geometry that has them) as none, as
/// the derivation holds them.
fn clean(g: &Geometry) -> Geometry {
    let coord = |c: &Coord| Coord {
        x: c.x,
        y: c.y,
        z: c.z.filter(|v| v.is_finite()),
        m: c.m.filter(|v| v.is_finite()),
    };
    fn curve(c: &Curve, f: &dyn Fn(&Coord) -> Coord) -> Curve {
        match c {
            Curve::Line(p) => Curve::Line(p.iter().map(f).collect()),
            Curve::Arc(p) => Curve::Arc(p.iter().map(f).collect()),
            Curve::Compound(parts) => Curve::Compound(parts.iter().map(|c| curve(c, f)).collect()),
        }
    }
    match g {
        Geometry::Point(p) => Geometry::Point(p.as_ref().map(coord)),
        Geometry::Curve(c) => Geometry::Curve(curve(c, &coord)),
        Geometry::Polygon(r) => Geometry::Polygon(r.iter().map(|c| curve(c, &coord)).collect()),
        Geometry::MultiPoint(p) => Geometry::MultiPoint(p.iter().map(coord).collect()),
        Geometry::MultiCurve(c) => {
            Geometry::MultiCurve(c.iter().map(|c| curve(c, &coord)).collect())
        }
        Geometry::MultiPolygon(p) => Geometry::MultiPolygon(
            p.iter()
                .map(|r| r.iter().map(|c| curve(c, &coord)).collect())
                .collect(),
        ),
        Geometry::Collection(g) => Geometry::Collection(g.iter().map(clean).collect()),
    }
}

/// The fields of a file's row that differ from the row the source gives now: `geometry` and the columns' names.
fn differences(
    layer: &gpkg::Layer,
    row: &gpkg::Row,
    expected: &DerivedRow,
    entity: &Entity,
    renamed: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut fields = Vec::new();
    let want = expected
        .geometry
        .clone()
        .and_then(|g| match &layer.geometry {
            Some(col) => gp::fit_one(g, &col.type_name).ok().map(|g| clean(&g)),
            None => None,
        });
    let have = match &row.geometry {
        Some(Ok(g)) => Some(clean(g)),
        _ => None,
    };
    if !crate::sources::same_geometry(want.as_ref(), have.as_ref(), 1e-9) {
        fields.push("geometry".to_owned());
    }
    let attrs = &entity.base().attrs;
    for (c, v) in layer.columns.iter().zip(&row.values) {
        // The derivation's own columns are not the source's; a column written under another name is the source's.
        let source = renamed.get(&c.name).unwrap_or(&c.name);
        if c.name.starts_with("kentos_") && !renamed.contains_key(&c.name) {
            continue;
        }
        let want = match expected.extras.iter().find(|(k, _)| k == source) {
            Some((_, Cell::Text(t))) => gp::cell(t.as_ref(), &c.kind),
            Some((_, Cell::Real(r))) => Value::Real(*r),
            None => gp::cell(attrs.get(source), &c.kind),
        };
        if !gp::same_cell(v, &want, &c.kind) {
            fields.push(c.name.clone());
        }
    }
    fields
}

/// Every row of the derived dataset at `path` against the object it came from in `doc` (docs/adr/0530 §4).
pub fn check(doc: &Document, path: &Path, limit: Option<u32>) -> Result<GisDerived, Refusal> {
    let limit = limit
        .unwrap_or(DERIVATION_LIMIT_DEFAULT)
        .min(DERIVATION_LIMIT_MAX) as usize;
    let (bytes, sha256) = std::fs::File::open(path)
        .and_then(|f| kentos_commands::encoding::sha256_reader_hex(std::io::BufReader::new(f)))
        .map_err(|e| {
            (
                "import_unreadable",
                format!("“{}” okunamadı: {e}", path.display()),
            )
        })?;
    let file = GeoPackage::open(path, false).map_err(|e| {
        (
            "import_unreadable",
            format!("{} GeoPackage olarak okunamadı: {}", path.display(), e.0),
        )
    })?;
    let layers = file.layers().map_err(|e| {
        (
            "import_unreadable",
            format!("{} okunamadı: {}", path.display(), e.0),
        )
    })?;
    let mut tables = Vec::new();
    let mut entries: Vec<DerivationEntry> = Vec::new();
    let mut source: Option<DerivationSource> = None;
    let mut algorithm = String::new();
    let mut tolerance = 0.0;
    for layer in &layers {
        let Some(rec) = record_of(&file, &layer.name) else {
            continue;
        };
        if source.is_none() {
            source = Some(rec.source.clone());
            algorithm = rec.algorithm.clone();
            tolerance = rec.tolerance;
        }
        let uids = file
            .uids(&layer.name)
            .map_err(|e| ("import_unreadable", e.0))?;
        let rows = file
            .rows(layer, &Query::default())
            .map_err(|e| ("import_unreadable", e.0))?;
        let opts = Options {
            tolerance: rec.tolerance,
            annotations: GisAnnotations::Table,
        };
        let (uid_col, kind_col, dev_col, digest_col) = (
            column_of(layer, "kentos_uid"),
            column_of(layer, "kentos_kind"),
            column_of(layer, "kentos_deviation"),
            column_of(layer, "kentos_digest"),
        );
        let mut t = DerivationTable {
            table: layer.name.clone(),
            role: rec.role,
            layer: rec.layer.as_ref().map(|l| l.id.clone()),
            rows: rows.len() as u32,
            current: 0,
            stale: 0,
            edited: 0,
            source_gone: 0,
            not_exported: 0,
            over_tolerance: 0,
            tolerance: rec.tolerance,
            max_deviation: 0.0,
        };
        // The lasting ids the table holds, listed or not in the drawing.
        let mut seen: BTreeSet<Uuid> = BTreeSet::new();
        for row in &rows {
            let uid_text = uids
                .get(&row.fid)
                .cloned()
                .or_else(|| text_at(row, uid_col))
                .unwrap_or_default();
            let kind = text_at(row, kind_col);
            if let Some(Value::Real(d)) = dev_col.and_then(|i| row.values.get(i)) {
                if *d > t.max_deviation {
                    t.max_deviation = *d;
                }
                if *d > rec.tolerance * (1.0 + 1e-9) {
                    t.over_tolerance += 1;
                }
            }
            let mut entry = |status: DerivationStatus, fields: Vec<String>, reason: String| {
                entries.push(DerivationEntry {
                    table: layer.name.clone(),
                    fid: Some(row.fid),
                    uid: uid_text.clone(),
                    status,
                    kind: kind.clone(),
                    fields,
                    reason,
                });
                status
            };
            let status = match Uuid::parse_str(&uid_text)
                .ok()
                .and_then(|u| doc.slot_of(u).map(|s| (u, s)))
                .and_then(|(u, s)| doc.get(s).map(|e| (u, e)))
            {
                None => entry(
                    DerivationStatus::SourceGone,
                    Vec::new(),
                    if uid_text.is_empty() {
                        "satırın kalıcı kimliği yok; kaynağı bulunamaz".to_owned()
                    } else {
                        "kaynak nesne çizimde yok".to_owned()
                    },
                ),
                Some((u, e)) => {
                    seen.insert(u);
                    let now = digest(doc, e);
                    let recorded = text_at(row, digest_col);
                    if recorded.as_deref() != Some(now.as_str()) {
                        entry(
                            DerivationStatus::Stale,
                            Vec::new(),
                            if recorded.is_none() {
                                "satırda kaynak özeti yok; güncel olduğu doğrulanamaz".to_owned()
                            } else {
                                "kaynak nesne dışa aktarmadan sonra değişti".to_owned()
                            },
                        )
                    } else {
                        match derive_entity(doc, e, &opts) {
                            Outcome::Row(expected)
                                if expected.role == rec.role
                                    && (rec.role != GisTableRole::Layer
                                        || rec
                                            .layer
                                            .as_ref()
                                            .is_some_and(|l| l.id == expected.key)) =>
                            {
                                let fields = differences(layer, row, &expected, e, &rec.renamed);
                                if fields.is_empty() {
                                    t.current += 1;
                                    DerivationStatus::Current
                                } else {
                                    let reason = format!(
                                        "satır GIS'te değiştirilmiş: {}; kaynak nesne değişmedi",
                                        fields.join(", ")
                                    );
                                    entry(DerivationStatus::Edited, fields, reason)
                                }
                            }
                            _ => entry(
                                DerivationStatus::Stale,
                                Vec::new(),
                                "kaynak nesne artık bu tablonun satırı olarak türetilemiyor"
                                    .to_owned(),
                            ),
                        }
                    }
                }
            };
            match status {
                DerivationStatus::Stale => t.stale += 1,
                DerivationStatus::Edited => t.edited += 1,
                DerivationStatus::SourceGone => t.source_gone += 1,
                _ => {}
            }
        }
        // The drawing's objects the table should hold and does not.
        let scope: Vec<&String> = match rec.role {
            GisTableRole::Layer => rec.layer.iter().map(|l| &l.id).collect(),
            _ => rec.layers.iter().collect(),
        };
        for id in scope {
            if doc.layers().get(id).is_none() {
                continue;
            }
            for (uid, e) in doc.by_layer_with_uids(id) {
                if seen.contains(&uid) {
                    continue;
                }
                let cheap = match rec.role {
                    GisTableRole::Layer => true,
                    GisTableRole::Blocks => matches!(e, Entity::Insert(_)),
                    GisTableRole::Annotations => matches!(e, Entity::Text(_)),
                };
                if !cheap {
                    continue;
                }
                if let Outcome::Row(expected) = derive_entity(doc, e, &opts)
                    && expected.role == rec.role
                    && match rec.role {
                        GisTableRole::Layer => {
                            rec.layer.as_ref().is_some_and(|l| l.id == expected.key)
                        }
                        GisTableRole::Blocks => Some(&expected.key) == rec.block.as_ref(),
                        GisTableRole::Annotations => true,
                    }
                {
                    t.not_exported += 1;
                    entries.push(DerivationEntry {
                        table: layer.name.clone(),
                        fid: None,
                        uid: uid.to_string(),
                        status: DerivationStatus::NotExported,
                        kind: Some(expected.kind.to_owned()),
                        fields: Vec::new(),
                        reason: "nesne dışa aktarmadan sonra yapıldı ya da bu katmana taşındı; türevde satırı yok".to_owned(),
                    });
                }
            }
        }
        tables.push(t);
    }
    let Some(source) = source else {
        return Err((
            "not_a_derivation",
            format!(
                "{} dosyasında KentOS'un türetim kaydı yok; file.export.gis'in yazdığı bir dosya değil.",
                path.display()
            ),
        ));
    };
    let sum = |f: fn(&DerivationTable) -> u32| tables.iter().map(f).sum::<u32>();
    let (current, stale, edited, source_gone, not_exported) = (
        sum(|t| t.current),
        sum(|t| t.stale),
        sum(|t| t.edited),
        sum(|t| t.source_gone),
        sum(|t| t.not_exported),
    );
    // The rows that do not match first, in the file's order within a status; the matching ones are not listed
    // singly (their count is in the tables).
    let rank = |s: DerivationStatus| match s {
        DerivationStatus::Stale => 0,
        DerivationStatus::Edited => 1,
        DerivationStatus::SourceGone => 2,
        DerivationStatus::NotExported => 3,
        DerivationStatus::Current => 4,
    };
    entries.sort_by_key(|e| rank(e.status));
    let total = entries.len();
    entries.truncate(limit);
    let same_project = match (doc.project_id(), source.project.as_deref()) {
        (Some(a), Some(b)) => a.to_string() == b,
        _ => doc.name() == source.name,
    };
    Ok(GisDerived {
        file: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        bytes,
        sha256,
        algorithm,
        tolerance,
        source,
        same_project,
        tables,
        current,
        stale,
        edited,
        source_gone,
        not_exported,
        unlisted: (total - entries.len()) as u32,
        entries,
        revision: doc.revision().to_string(),
    })
}
