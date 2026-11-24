//! `cad.parcels.report` v1 and `cad.parcels.table` v1 (docs/adr/9306): Parsel alan çizelgesi. `report` lists the
//! parcels' corners in order with a point number and their Y, X, the edges' lengths and the area computed from the
//! coordinates, the record's area apart, and the table's words; it writes nothing. `table` writes those words as one
//! table at `at` (one undo step “Parsel alan çizelgesi tablosu”). The rules, as the independent reference
//! (`scripts/fixtures/parcels_report_cases.py`) writes them:
//!
//! - the scope: the areas `uids` names in its order, else every area on the parcel layer in the drawing's order;
//! - a parcel's rings: each part's outer ring, then its holes; a ring runs clockwise as the map shows it (stored
//!   counter-clockwise, by its signed area with its arcs, it is walked backwards, each arc turning the other way), from
//!   its northernmost corner, of two as far north the western one, of two at one place the first;
//! - a corner's name: with `names: points` the first named point object of the drawing (a multi-point object's k-th
//!   point named “ad (k)”) within 1 µm; else, and with `names: numbers` always, `prefix` and a number from `start` up by
//!   one, past every name the drawing's points have and the report has given; a place already named in the report
//!   (1 µm, the first) keeps its name;
//! - an edge: the chord to the ring's next corner; an arc's radius and its length along the arc;
//! - the areas: the core's (`bulge_ring_area`, `entity_area`): the shoelace from the coordinates as stored and each
//!   arc's segment; a parcel's is its outer rings' less its holes'. The record as written beside it, never over it
//!   (CLAUDE.md §7, §23.1): its value when a plain decimal, the difference and Parsel alan denetimi's tolerance;
//! - the words: coordinates and lengths in metres with the project's length decimals, areas in square metres with its
//!   area decimals, by the display rule (`display::fixed`): words to read, never read back.

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    CellRange, CellStyle, CommandResult, CommandWarning, CornerNamed, CornerNames, Entity,
    EntityBase, MAX_CORNER_PREFIX, MAX_RECORD_FIELD, MAX_TABLE_SIZE, PARCEL_AREA_RULE,
    PARCEL_LAYER, ParcelReported, ParcelsReport, ParcelsReported, ParcelsTable, ParcelsTablePlan,
    ParcelsTabled, ReportCorner, ReportRing, TableAlign, TableEntity,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::{Shape, entity_area};
use kentos_geometry_core::geom::bulge::{bulge_arc, bulge_ring_area};
use kentos_geometry_core::geom::table::{PAD, table_geom};
use kentos_geometry_core::jsmath::js_hypot;
use kentos_geometry_core::ops::table::one_line;
use kentos_geometry_core::text::width_em_in;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::{drawing_font, shape};
use crate::parcels_check::{RECORDED_FIELD, recorded_value, tolerance};

/// The undo step's name.
pub const STEP: &str = "Parsel alan çizelgesi tablosu";
/// The table's title.
pub const TITLE: &str = "Parsel alan çizelgesi";
/// Two places this close are one corner, m (the elevation rules' “same place”, docs/adr/0142).
const TOUCH: f64 = 1e-6;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Places by the cell of a grid `TOUCH` wide, each with the order it came in: the first within `TOUCH` of a place.
#[derive(Default)]
struct Places {
    cells: HashMap<(i64, i64), Vec<usize>>,
    at: Vec<Vec2>,
}

impl Places {
    fn cell(p: Vec2) -> (i64, i64) {
        ((p.x / TOUCH).floor() as i64, (p.y / TOUCH).floor() as i64)
    }

    fn put(&mut self, p: Vec2) -> usize {
        let i = self.at.len();
        self.at.push(p);
        self.cells.entry(Self::cell(p)).or_default().push(i);
        i
    }

    /// The first place put within `TOUCH` of `p`.
    fn near(&self, p: Vec2) -> Option<usize> {
        let (cx, cy) = Self::cell(p);
        let mut best: Option<usize> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &i in self.cells.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    let q = self.at[i];
                    if js_hypot(q.x - p.x, q.y - p.y) <= TOUCH && best.is_none_or(|b| i < b) {
                        best = Some(i);
                    }
                }
            }
        }
        best
    }
}

/// The names the drawing's points give, in the drawing's order: a point's label, a multi-point object's k-th point
/// “label (k)”; unnamed points give none.
fn point_names(doc: &Document) -> (Places, Vec<String>) {
    let mut places = Places::default();
    let mut names = Vec::new();
    for e in doc.entities() {
        let Entity::Point(p) = e else { continue };
        let Some(label) = p
            .base
            .label
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
        else {
            continue;
        };
        let label = one_line(label).words;
        places.put(Vec2::new(p.p.x, p.p.y));
        names.push(label.clone());
        for (k, part) in p.parts.iter().flatten().enumerate() {
            places.put(Vec2::new(part.p.x, part.p.y));
            names.push(format!("{label} ({})", k + 2));
        }
    }
    (places, names)
}

/// How the report's corners are named: the drawing's named points, the places named so far and the next number.
struct Namer<'a> {
    names: CornerNames,
    prefix: &'a str,
    next: u64,
    points: Places,
    point_names: Vec<String>,
    taken: HashSet<String>,
    given: Places,
    given_names: Vec<(String, CornerNamed)>,
    named: usize,
    numbered: usize,
}

impl<'a> Namer<'a> {
    fn new(doc: &Document, names: CornerNames, prefix: &'a str, start: u32) -> Self {
        let (points, point_names) = point_names(doc);
        let taken = point_names.iter().cloned().collect();
        Self {
            names,
            prefix,
            next: u64::from(start),
            points,
            point_names,
            taken,
            given: Places::default(),
            given_names: Vec::new(),
            named: 0,
            numbered: 0,
        }
    }

    fn name(&mut self, p: Vec2) -> (String, CornerNamed) {
        if let Some(i) = self.given.near(p) {
            return self.given_names[i].clone();
        }
        let from_point = match self.names {
            CornerNames::Points => self.points.near(p).map(|i| self.point_names[i].clone()),
            CornerNames::Numbers => None,
        };
        let named = match from_point {
            Some(name) => {
                self.named += 1;
                (name, CornerNamed::Point)
            }
            None => {
                let name = loop {
                    let candidate = format!("{}{}", self.prefix, self.next);
                    self.next += 1;
                    if !self.taken.contains(&candidate) {
                        break candidate;
                    }
                };
                self.numbered += 1;
                (name, CornerNamed::Number)
            }
        };
        self.taken.insert(named.0.clone());
        self.given.put(p);
        self.given_names.push(named.clone());
        named
    }
}

/// A ring as stored: its part, its hole's number in the part (none: the outer ring), its vertices and bulges.
struct Stored<'s> {
    part: u32,
    hole: Option<u32>,
    pts: &'s [Vec2],
    bulges: Option<&'s [f64]>,
}

fn rings_of(s: &Shape) -> Vec<Stored<'_>> {
    let mut out = Vec::new();
    if let Shape::Polygon {
        pts,
        bulges,
        holes,
        parts,
    } = s
    {
        out.push(Stored {
            part: 0,
            hole: None,
            pts,
            bulges: bulges.as_deref(),
        });
        for (h, ring) in holes.iter().flatten().enumerate() {
            out.push(Stored {
                part: 0,
                hole: Some(count(h)),
                pts: &ring.pts,
                bulges: ring.bulges.as_deref(),
            });
        }
        for (k, part) in parts.iter().flatten().enumerate() {
            let k = count(k + 1);
            out.push(Stored {
                part: k,
                hole: None,
                pts: &part.pts,
                bulges: part.bulges.as_deref(),
            });
            for (h, ring) in part.holes.iter().flatten().enumerate() {
                out.push(Stored {
                    part: k,
                    hole: Some(count(h)),
                    pts: &ring.pts,
                    bulges: ring.bulges.as_deref(),
                });
            }
        }
    }
    out
}

/// A ring's corners clockwise from the northernmost, each with the bulge of its edge to the next.
fn clockwise(ring: &Stored<'_>) -> Vec<(Vec2, f64)> {
    let n = ring.pts.len();
    let bulge = |i: usize| ring.bulges.and_then(|b| b.get(i).copied()).unwrap_or(0.0);
    let walked: Vec<(Vec2, f64)> = if bulge_ring_area(ring.pts, ring.bulges) > 0.0 {
        // Counter-clockwise as stored: backwards, the edge from pts[n−k] to pts[n−k−1] is stored edge n−k−1 turned.
        (0..n)
            .map(|k| (ring.pts[(n - k) % n], -bulge((2 * n - k - 1) % n)))
            .collect()
    } else {
        (0..n).map(|k| (ring.pts[k], bulge(k))).collect()
    };
    let start = (0..n).fold(0, |best, k| {
        let (p, b) = (walked[k].0, walked[best].0);
        if p.y > b.y || (p.y == b.y && p.x < b.x) {
            k
        } else {
            best
        }
    });
    (0..n).map(|k| walked[(start + k) % n]).collect()
}

/// The checked input: the record's attribute, the prefix and the parcels.
struct Checked<'d> {
    field: String,
    prefix: String,
    parcels: Vec<&'d Entity>,
}

fn check<'d>(doc: &'d Document, input: &ParcelsReport) -> Result<Checked<'d>, Stop> {
    let prefix = input.prefix.clone().unwrap_or_default();
    if prefix.chars().count() > MAX_CORNER_PREFIX {
        return Err(fail(
            codes::INVALID_INPUT,
            format!(
                "Numara öneki “{prefix}” {MAX_CORNER_PREFIX} harften uzun; daha kısa bir önek yazın."
            ),
            "prefix",
        ));
    }
    if prefix.chars().any(char::is_control) {
        return Err(fail(
            codes::INVALID_INPUT,
            "Numara önekinde satır sonu ya da denetim karakteri var; önek tek satırlık bir yazı olmalı.".into(),
            "prefix",
        ));
    }
    let field = input.field.as_deref().map(str::trim).unwrap_or_default();
    if field.chars().count() > MAX_RECORD_FIELD {
        return Err(fail(
            codes::INVALID_INPUT,
            format!(
                "Kayıtlı alanın öznitelik adı {MAX_RECORD_FIELD} harften uzun; tapu alanını tutan özniteliğin adını yazın."
            ),
            "field",
        ));
    }
    let field = if field.is_empty() {
        RECORDED_FIELD
    } else {
        field
    }
    .to_owned();
    let parcels: Vec<&Entity> = match &input.uids {
        Some(uids) => {
            let mut out = Vec::new();
            for (i, uid) in uids.iter().enumerate() {
                let path = format!("uids[{i}]");
                if !checks::is_uid_text(uid) {
                    return Err(fail(
                        codes::INVALID_UID,
                        format!("“{uid}” geçerli bir nesne kimliği değil."),
                        &path,
                    ));
                }
                let Some(e) = Uuid::parse_str(uid)
                    .ok()
                    .and_then(|u| doc.slot_of(u))
                    .and_then(|s| doc.get(s))
                else {
                    return Err(fail(
                        codes::ENTITY_NOT_FOUND,
                        format!("“{uid}” kimlikli nesne çizimde yok."),
                        &path,
                    ));
                };
                if !matches!(e, Entity::Polygon(_)) {
                    return Err(fail(
                        codes::NOT_A_PARCEL,
                        format!(
                            "“{uid}” bir alan değil; parsel alan çizelgesi yalnız alanları listeler."
                        ),
                        &path,
                    ));
                }
                out.push(e);
            }
            out
        }
        None => doc
            .by_layer(PARCEL_LAYER)
            .filter(|e| matches!(e, Entity::Polygon(_)))
            .collect(),
    };
    Ok(Checked {
        field,
        prefix,
        parcels,
    })
}

/// A parcel's heading: “101 ada 7 parsel”, its label, or “Alan 3” by its place in the report; its mahalle after it.
fn heading(ada: &str, parsel: &str, mahalle: &str, label: Option<&str>, place: usize) -> String {
    let mut words: Vec<String> = Vec::new();
    if !ada.is_empty() {
        words.push(format!("{ada} ada"));
    }
    if !parsel.is_empty() {
        words.push(format!("{parsel} parsel"));
    }
    let mut out = if !words.is_empty() {
        words.join(" ")
    } else {
        label.map_or_else(|| format!("Alan {place}"), str::to_owned)
    };
    if !mahalle.is_empty() {
        out.push_str(&format!(" ({mahalle})"));
    }
    one_line(&out).words
}

/// A ring's words above its corners: none for the first part's outer ring.
fn ring_words(part: u32, hole: Option<u32>) -> Option<String> {
    match (part, hole) {
        (0, None) => None,
        (0, Some(h)) => Some(format!("Delik {}", h + 1)),
        (k, None) => Some(format!("Parça {}", k + 1)),
        (k, Some(h)) => Some(format!("Parça {}, delik {}", k + 1, h + 1)),
    }
}

/// The report and the table's words, with the ranges to merge and the rows to make bold.
struct Worked {
    report: ParcelsReported,
    merges: Vec<CellRange>,
    bold: Vec<u32>,
    columns: usize,
}

fn work(doc: &Document, input: &ParcelsReport, checked: Checked<'_>) -> Worked {
    let settings = doc.settings();
    let (ld, ad) = (
        (settings.length_decimals as usize).min(12),
        (settings.area_decimals as usize).min(12),
    );
    let mut namer = Namer::new(
        doc,
        input.names.unwrap_or_default(),
        &checked.prefix,
        input.start.unwrap_or(1),
    );
    let mut parcels = Vec::new();
    for (i, e) in checked.parcels.iter().enumerate() {
        let attr = |k: &str| {
            e.base()
                .attrs
                .get(k)
                .map(|v| v.trim().to_owned())
                .unwrap_or_default()
        };
        let s = shape(e);
        let rings: Vec<ReportRing> = rings_of(&s)
            .iter()
            .map(|ring| {
                let listed = clockwise(ring);
                let n = listed.len();
                let corners = (0..n)
                    .map(|k| {
                        let ((p, b), (q, _)) = (listed[k], listed[(k + 1) % n]);
                        let (name, named) = namer.name(p);
                        let arc = bulge_arc(p, q, b);
                        ReportCorner {
                            name,
                            named,
                            at: kentos_commands::Vec2 { x: p.x, y: p.y },
                            length: js_hypot(q.x - p.x, q.y - p.y),
                            radius: arc.map(|a| a.r),
                            arc: arc.map(|a| a.r * a.sweep.abs()),
                        }
                    })
                    .collect();
                ReportRing {
                    part: ring.part,
                    hole: ring.hole,
                    corners,
                    area: bulge_ring_area(ring.pts, ring.bulges).abs(),
                }
            })
            .collect();
        let area = entity_area(&s).unwrap_or(0.0);
        let text = e
            .base()
            .attrs
            .get(&checked.field)
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty());
        let recorded = text.as_deref().and_then(recorded_value);
        let difference = recorded.map(|r| area - r);
        let limit = tolerance(area);
        let status = match (&text, difference) {
            (None, _) => "noRecord",
            (Some(_), None) => "invalidRecord",
            (Some(_), Some(d)) if d.abs() <= limit => "within",
            _ => "outside",
        };
        let (ada, parsel, mahalle) = (attr("Ada"), attr("Parsel"), attr("Mahalle"));
        let label = e
            .base()
            .label
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty());
        parcels.push(ParcelReported {
            uid: doc
                .uid(Slot(e.base().id))
                .map(|u| u.to_string())
                .unwrap_or_default(),
            il: attr("İl"),
            ilce: attr("İlçe"),
            heading: heading(&ada, &parsel, &mahalle, label, i + 1),
            mahalle,
            ada,
            parsel,
            rings,
            area,
            recorded_text: text.map(|t| one_line(&t).words),
            recorded,
            difference,
            tolerance: limit,
            status: status.to_owned(),
        });
    }
    let lengths = input.lengths.unwrap_or(true);
    let radii = lengths
        && parcels
            .iter()
            .flat_map(|p| &p.rings)
            .flat_map(|r| &r.corners)
            .any(|c| c.radius.is_some());
    let m = 3 + usize::from(lengths) + usize::from(radii);
    let srid = (settings.srid != 0).then_some(settings.srid);
    let mut cells: Vec<Vec<String>> = Vec::new();
    let mut merges = Vec::new();
    let mut bold = Vec::new();
    let row = |words: Vec<String>| -> Vec<String> {
        let mut r = words;
        r.resize(m, String::new());
        r
    };
    let across = |cells: &Vec<Vec<String>>, cols: usize| CellRange {
        row: count(cells.len()),
        col: 0,
        rows: 1,
        cols: count(cols),
    };
    // The title across the table, the heading row (bold, centred).
    merges.push(across(&cells, m));
    cells.push(row(vec![match srid {
        Some(s) => format!("{TITLE} (EPSG:{s})"),
        None => TITLE.to_owned(),
    }]));
    bold.push(count(cells.len()));
    let mut heads = vec!["Nokta".to_owned(), "Y (m)".to_owned(), "X (m)".to_owned()];
    if lengths {
        heads.push("Kenar (m)".to_owned());
    }
    if radii {
        heads.push("Yarıçap (m)".to_owned());
    }
    cells.push(heads);
    // An area's row: its words across all columns but the last, its value in the last.
    let area_row =
        |cells: &mut Vec<Vec<String>>, merges: &mut Vec<CellRange>, words: &str, value: String| {
            merges.push(across(cells, m - 1));
            let mut r = row(vec![words.to_owned()]);
            r[m - 1] = value;
            cells.push(r);
        };
    for p in &parcels {
        bold.push(count(cells.len()));
        merges.push(across(&cells, m));
        cells.push(row(vec![p.heading.clone()]));
        for ring in &p.rings {
            if let Some(words) = ring_words(ring.part, ring.hole) {
                merges.push(across(&cells, m));
                cells.push(row(vec![words]));
            }
            for c in &ring.corners {
                let mut r = vec![c.name.clone(), fixed(c.at.x, ld), fixed(c.at.y, ld)];
                if lengths {
                    r.push(fixed(c.length, ld));
                }
                if radii {
                    r.push(c.radius.map(|v| fixed(v, ld)).unwrap_or_default());
                }
                cells.push(r);
            }
        }
        area_row(
            &mut cells,
            &mut merges,
            "Hesap alanı (m²)",
            fixed(p.area, ad),
        );
        if let Some(text) = &p.recorded_text {
            area_row(&mut cells, &mut merges, "Tapu alanı (m²)", text.clone());
        }
        if let Some(d) = p.difference {
            let shown = fixed(d, ad);
            let signed = if d > 0.0 && shown.bytes().any(|b| (b'1'..=b'9').contains(&b)) {
                format!("+{shown}")
            } else {
                shown
            };
            area_row(&mut cells, &mut merges, "Fark (m²)", signed);
            area_row(
                &mut cells,
                &mut merges,
                "Yanılma sınırı (m²)",
                format!("±{}", fixed(p.tolerance, ad)),
            );
        }
    }
    let sum: f64 = parcels.iter().map(|p| p.area).sum();
    if parcels.len() > 1 {
        area_row(
            &mut cells,
            &mut merges,
            "Toplam hesap alanı (m²)",
            fixed(sum, ad),
        );
    }
    let places = namer.given.at.len();
    Worked {
        report: ParcelsReported {
            srid,
            rule: PARCEL_AREA_RULE.to_owned(),
            field: checked.field,
            parcels,
            area: sum,
            places: count(places),
            named: count(namer.named),
            numbered: count(namer.numbered),
            cells,
        },
        merges,
        bold,
        columns: m,
    }
}

pub mod report {
    use super::*;

    fn run(doc: &Document, input: &ParcelsReport) -> Result<ParcelsReported, Stop> {
        let checked = check(doc, input)?;
        Ok(work(doc, input, checked).report)
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &ParcelsReport) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// The report; reads only.
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &ParcelsReport,
    ) -> CommandResult<ParcelsReported> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over a document: the window's table.
    pub fn plan_in(doc: &Document, input: &ParcelsReport) -> CommandResult<ParcelsReported> {
        match run(doc, input) {
            Ok(output) => CommandResult::Completed {
                output,
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// The report; reads only.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ParcelsReport,
    ) -> CommandResult<ParcelsReported> {
        plan_in(cx.doc, &input)
    }
}

// The table write (`cad.parcels.table`), apart from the report (`cad.parcels.report`).
#[path = "parcels_report_table.rs"]
pub mod table;
