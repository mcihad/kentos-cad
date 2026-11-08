//! `road.terrains.cutfill` v1 (docs/adr/1840; plan/map-cad.md MAP-15): Kazı ve dolgu bölgeleri, where a comparison
//! surface lies below a base and above it (`kentos_road::volume_zones::cut_fill`), written on a layer in one undo step
//! “Kazı ve dolgu bölgeleri” through `cad.entities.create`'s write. Each zone is two objects with the same rings: a
//! solid hatch, its fill (a closed area is filled only in its layer's one fill colour), then the closed area with its
//! holes, the zone's data for the geometry tools and a GIS layer. Both carry its kind, band, plan area and volume as
//! attributes, the cut in KentOS's red and the fill in its blue, a deeper band's hatch less transparent. The drawing is
//! kept with both surfaces' stamps (as the zero line's, docs/adr/1720). Harita and Yol projects. The shared cases are
//! `fixtures/commands/v1/road.terrains.cutfill.json`.
//!
//! Asked for (docs/adr/2340), each zone's kind and volume is written inside it (the core's interior point, GEOS's rule),
//! the project's text height; and a quantity table beside the zones (the zones, then the cut's, the fill's and the net
//! totals), the project's table height, its columns fitted as the Tablo tool fits them.
//!
//! The checks, in the contract's order: a map or road project; the layer; the expected revision; the interval; the
//! surfaces, two different ones; both made; the bands' count; a zone at all.

use std::collections::BTreeMap;

use kentos_commands::{
    AnnotationKind, CommandResult, CommandWarning, CreateOperation, CutFillKind, CutFillLabel, EntitiesCreate,
    EntityBase, EntityGeometry, HatchPattern, HatchPatternType, MOST_CUTFILL_BANDS, NewObject, RingGeometry, TableAlign,
    TableEntity, TerrainCutFillZone, TerrainsCutFill, TerrainsCutFilled, TextAlign, Vec2,
};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::ops::convert::points::interior_point;
use kentos_road::surface::Surface;
use kentos_road::volume::Triangle;
use kentos_road::volume_zones::{TooManyBands, cut_fill};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

/// The cut's colour and the fill's: the red and the blue KentOS's road drawings already use (road_sections_draw.rs).
const CUT: &str = "#d62828";
const FILL: &str = "#2563eb";

struct Checked {
    out: TerrainsCutFilled,
    /// The surfaces' names, for the zones' attributes.
    names: (String, String),
    warnings: Vec<CommandWarning>,
}

fn triangles(s: &Surface) -> Vec<Triangle> {
    s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect()
}

/// A number as Rust writes an f64 (the shortest that reads back the same).
fn num(v: f64) -> String {
    format!("{v}")
}

fn check(doc: &kentos_domain::Document, input: &TerrainsCutFill) -> Result<Checked, Stop> {
    terrains::terrain_project(doc)?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    if let Some(i) = input.interval
        && !(i.is_finite() && i > 0.0)
    {
        return Err(fail(
            codes::INVALID_ANALYSIS,
            "Aralık 0'dan büyük bir sayı olmalı (m); bantsız kazı ve dolgu için boş bırakın.".into(),
            "interval",
        ));
    }
    let base = crate::road_profiles::surface_id(doc, &input.base, "base")?;
    let comparison = crate::road_profiles::surface_id(doc, &input.comparison, "comparison")?;
    if base == comparison {
        return Err(fail(
            codes::INVALID_ANALYSIS,
            "Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.".into(),
            "comparison",
        ));
    }
    let def = |id: &str| terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled);
    let (b, c) = (def(&base)?, def(&comparison)?);
    let (made_b, made_c) = (terrains::make(doc, &b), terrains::make(doc, &c));
    let (sb, sc) = (made_b.built()?, made_c.built()?);
    let z = match cut_fill(&triangles(sc), &triangles(sb), input.interval, MOST_CUTFILL_BANDS) {
        Ok(z) => z,
        Err(TooManyBands(n)) => {
            return Err(fail(
                codes::INVALID_ANALYSIS,
                format!(
                    "Bu aralık iki yüzeyin farkını {n} banda böler; en çok {MOST_CUTFILL_BANDS} bant olur. Daha büyük bir \
                     aralık verin."
                ),
                "interval",
            ));
        }
    };
    if z.zones.is_empty() {
        return Err(fail(
            codes::NO_CUT_FILL,
            "İki yüzey örtüşmüyor ya da örtüştükleri her yerde çakışıyor: kazı ya da dolgu bölgesi yok (karşılaştırma \
             tabanın kapladığı yerde olmalı)."
                .into(),
            "comparison",
        ));
    }
    let (least, largest) = (z.least.unwrap_or(0.0), z.largest.unwrap_or(0.0));
    let zones = z
        .zones
        .into_iter()
        .map(|zone| {
            let kind = if zone.band < 0 { CutFillKind::Cut } else { CutFillKind::Fill };
            let band = input.interval.map(|i| (zone.band as f64 * i, (zone.band + 1) as f64 * i));
            TerrainCutFillZone {
                kind,
                from: band.map(|b| b.0),
                to: band.map(|b| b.1),
                area: zone.area,
                volume: zone.volume.abs(),
                ring: zone.outer,
                holes: zone.holes,
            }
        })
        .collect();
    Ok(Checked {
        out: TerrainsCutFilled {
            base,
            comparison,
            interval: input.interval,
            zones,
            cut: z.cut,
            fill: z.fill,
            net: z.fill - z.cut,
            cut_area: z.cut_area,
            fill_area: z.fill_area,
            coincident: z.coincident,
            covered: z.covered,
            uncovered: z.uncovered,
            least,
            largest,
            labels: Vec::new(),
            table: Vec::new(),
            uids: Vec::new(),
            revision: doc.revision().to_string(),
        },
        names: (b.name.clone(), c.name.clone()),
        warnings,
    })
    .map(|mut c| {
        if input.labels.unwrap_or(false) {
            c.out.labels = c
                .out
                .zones
                .iter()
                .filter_map(|z| label_at(z).map(|at| CutFillLabel { at, text: label_text(z) }))
                .collect();
        }
        if input.table.unwrap_or(false) {
            c.out.table = table_cells(&c.out);
        }
        c
    })
}

/// A zone's words: its kind and its volume to two decimals (m³).
fn label_text(z: &TerrainCutFillZone) -> String {
    format!("{} {:.2} m³", if z.kind == CutFillKind::Cut { "Kazı" } else { "Dolgu" }, z.volume)
}

/// A zone's label place: the core's interior point of its rings.
fn label_at(z: &TerrainCutFillZone) -> Option<[f64; 2]> {
    let v = |r: &[[f64; 2]]| r.iter().map(|p| kentos_geometry_core::vec2::Vec2::new(p[0], p[1])).collect::<Vec<_>>();
    let shape = Shape::Polygon {
        pts: v(&z.ring),
        bulges: None,
        holes: (!z.holes.is_empty())
            .then(|| z.holes.iter().map(|h| Ring { pts: v(h), bulges: None }).collect()),
        parts: None,
    };
    interior_point(&shape).map(|p| [p.x, p.y])
}

/// The quantity table's cells: the heading, each zone (its number, kind, band when banded, area and volume), then the
/// cut's, the fill's and the net totals; numbers to two decimals.
fn table_cells(out: &TerrainsCutFilled) -> Vec<Vec<String>> {
    let banded = out.interval.is_some();
    let two = |v: f64| format!("{v:.2}");
    let mut rows = vec![if banded {
        ["Bölge", "Tür", "Alt (m)", "Üst (m)", "Alan (m²)", "Hacim (m³)"].map(String::from).to_vec()
    } else {
        ["Bölge", "Tür", "Alan (m²)", "Hacim (m³)"].map(String::from).to_vec()
    }];
    for (k, z) in out.zones.iter().enumerate() {
        let mut row = vec![(k + 1).to_string(), if z.kind == CutFillKind::Cut { "Kazı" } else { "Dolgu" }.to_owned()];
        if banded {
            row.push(z.from.map(two).unwrap_or_default());
            row.push(z.to.map(two).unwrap_or_default());
        }
        row.push(two(z.area));
        row.push(two(z.volume));
        rows.push(row);
    }
    let blank = if banded { 3 } else { 1 };
    for (name, area, volume) in
        [("Kazı", Some(out.cut_area), out.cut), ("Dolgu", Some(out.fill_area), out.fill), ("Net (dolgu − kazı)", None, out.net)]
    {
        let mut row = vec![name.to_owned()];
        row.extend(std::iter::repeat_n(String::new(), blank));
        row.push(area.map(two).unwrap_or_default());
        row.push(two(volume));
        rows.push(row);
    }
    rows
}

/// A zone's transparency: without bands one half; with bands, on each side the band at zero 0.75, the deepest 0.25.
fn transparency(out: &TerrainsCutFilled, zone: &TerrainCutFillZone) -> f64 {
    let (Some(i), Some(from)) = (out.interval, zone.from) else { return 0.5 };
    let band = (from / i).round();
    let side: Vec<f64> = out.zones.iter().filter(|z| z.kind == zone.kind).filter_map(|z| z.from).map(|f| (f / i).round()).collect();
    let (depth, deepest) = match zone.kind {
        CutFillKind::Cut => (-band - 1.0, side.iter().map(|b| -b - 1.0).fold(0.0, f64::max)),
        CutFillKind::Fill => (band, side.iter().copied().fold(0.0, f64::max)),
    };
    if deepest > 0.0 { 0.75 - 0.5 * depth / deepest } else { 0.5 }
}

/// A bare object: its geometry alone.
fn bare(geometry: EntityGeometry) -> NewObject {
    NewObject {
        geometry,
        color: None,
        line_weight: None,
        line_type: None,
        line_pattern: None,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

/// The labels and the quantity table (docs/adr/2340): each label in the middle of its place, masked, the project's text
/// height; the table's top left corner two of its heights right of the zones' box at its top, its columns fitted.
fn annotations(doc: &kentos_domain::Document, out: &TerrainsCutFilled) -> Vec<NewObject> {
    let settings = doc.settings();
    let text = settings.annotation_height(AnnotationKind::Text);
    let mut objects: Vec<NewObject> = out
        .labels
        .iter()
        .map(|l| {
            bare(EntityGeometry::Text {
                layout: None,
                annotative: None,
                p: Vec2 { x: l.at[0], y: l.at[1] },
                text: l.text.clone(),
                height: text,
                rotation: 0.0,
                align: Some(TextAlign::MiddleCenter),
                width_factor: None,
                mask: true,
                box_width: None,
                line_spacing: None,
                runs: Vec::new(),
                face: Default::default(),
                path: None,
            })
        })
        .collect();
    if out.table.is_empty() {
        return objects;
    }
    let all = out.zones.iter().flat_map(|z| z.ring.iter());
    let (x1, y1) = all.fold((f64::MIN, f64::MIN), |(x, y), p| (x.max(p[0]), y.max(p[1])));
    let h = settings.annotation_height(AnnotationKind::Table);
    let columns = out.table.first().map_or(0, Vec::len);
    let aligns: Vec<TableAlign> = (0..columns).map(|j| if j < 2 { TableAlign::Left } else { TableAlign::Right }).collect();
    let mut t = TableEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: String::new(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: Vec2 { x: x1 + 2.0 * h, y: y1 },
        rotation: 0.0,
        height: h,
        rows: Vec::new(),
        columns: Vec::new(),
        cells: Vec::new(),
        merges: Vec::new(),
        aligns: Some(aligns),
        header: true,
        grid: None,
        frame: None,
        face: Default::default(),
        source: None,
        styles: Vec::new(),
        formulas: Vec::new(),
        fields: Vec::new(),
        breaks: None,
        table_style: None,
    };
    doc.take_rows(&mut t, out.table.clone());
    objects.push(bare(EntityGeometry::Table {
        p: t.p,
        rotation: t.rotation,
        height: t.height,
        rows: t.rows,
        columns: t.columns,
        cells: t.cells,
        merges: Vec::new(),
        aligns: t.aligns,
        header: true,
        grid: None,
        frame: None,
        face: Default::default(),
        source: None,
        styles: None,
        formulas: None,
        fields: None,
        table_style: None,
    }));
    objects
}

/// Each zone's hatch, then its closed area.
fn objects(out: &TerrainsCutFilled, names: &(String, String)) -> Vec<NewObject> {
    let pts = |r: &[[f64; 2]]| r.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect::<Vec<Vec2>>();
    out.zones
        .iter()
        .flat_map(|z| {
            let cut = z.kind == CutFillKind::Cut;
            let mut attrs = BTreeMap::from([
                ("Tür".to_owned(), if cut { "Kazı" } else { "Dolgu" }.to_owned()),
                ("Alan (m²)".to_owned(), num(z.area)),
                ("Hacim (m³)".to_owned(), num(z.volume)),
                ("Taban".to_owned(), names.0.clone()),
                ("Karşılaştırma".to_owned(), names.1.clone()),
            ]);
            if let (Some(from), Some(to)) = (z.from, z.to) {
                attrs.insert("Alt sınır (m)".to_owned(), num(from));
                attrs.insert("Üst sınır (m)".to_owned(), num(to));
            }
            let object = |geometry: EntityGeometry, transparency: Option<f64>| NewObject {
                geometry,
                color: Some(if cut { CUT } else { FILL }.to_owned()),
                line_weight: None,
                line_type: None,
                line_pattern: None,
                attrs: Some(attrs.clone()),
                label: None,
                label_of: None,
                label_scale: None,
                symbol: None,
                order: None,
                transparency,
            };
            let hatch = EntityGeometry::Hatch {
                ring: pts(&z.ring),
                holes: (!z.holes.is_empty()).then(|| z.holes.iter().map(|h| pts(h)).collect()),
                pattern: HatchPattern::user(HatchPatternType::Solid, 0.0, 1.0),
                assoc: None,
                boundary: None,
                annotative: None,
            };
            let area = EntityGeometry::Polygon {
                pts: pts(&z.ring),
                bulges: None,
                holes: (!z.holes.is_empty())
                    .then(|| z.holes.iter().map(|h| RingGeometry { pts: pts(h), bulges: None, zs: None }).collect()),
                zs: None,
                parts: None,
            };
            [object(hatch, Some(transparency(out, z))), object(area, None)]
        })
        .collect()
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsCutFill) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// The zones execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsCutFill) -> CommandResult<TerrainsCutFilled> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// Draws `input` now, checked and written in the step it runs in, its drawing not kept (`road.project.rebuild` keeps it,
/// docs/adr/0911 §3, 1720).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &TerrainsCutFill) -> CommandResult<TerrainsCutFilled> {
    write(cx, input.clone())
}

/// Writes the zones, one undo step “Kazı ve dolgu bölgeleri”, and keeps the drawing with both surfaces' stamps: the
/// project's tree says when a surface changed since, and `road.project.rebuild` draws it again.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsCutFill) -> CommandResult<TerrainsCutFilled> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_CUTFILL,
        kentos_commands::ROAD_TERRAINS_CUTFILL_VERSION,
        input,
        |cx, input| {
            let r = write(cx, input.clone());
            crate::road_drawings::kept(
                cx,
                kentos_commands::ROAD_TERRAINS_CUTFILL,
                &input,
                (kentos_commands::RoadSourceKind::Surface, "base"),
                r,
                |o| &o.uids,
            )
        },
    )
}

fn write(cx: &mut ExecutionContext<'_>, input: TerrainsCutFill) -> CommandResult<TerrainsCutFilled> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut written = objects(&c.out, &c.names);
    written.extend(annotations(cx.doc, &c.out));
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: written,
        operation: Some(CreateOperation::TerrainCutFill),
        expected_revision: None,
    };
    match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: TerrainsCutFilled { uids: output.created, revision: output.revision, ..c.out },
            warnings,
        },
        other => crate::trim::relay(other),
    }
}
