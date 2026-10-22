//! The DXF writer (CLAUDE.md §9.7, docs/adr/0009): the app's objects as an
//! ASCII DXF in the AutoCAD 2007 format (AC1021). That version is the
//! oldest one with UTF-8 text (every Turkish letter as it is, no code page
//! to guess) and it has true colours (the app's hex colours exactly) and
//! line weights; AutoCAD 2007 and later, BricsCAD, Netcad, QGIS/GDAL,
//! LibreCAD and ezdxf read it. The file has what AutoCAD needs of an
//! AutoCAD 2000+ drawing (handles, owners, the symbol tables, the model
//! and paper space blocks, the root dictionary and layouts) and units in
//! metres ($INSUNITS 6), or in a local project's unit (4 millimetres,
//! 5 centimetres; docs/adr/0165 §2), every coordinate and length scaled.
//!
//! Coordinates are written as the shortest decimal that reads back to the
//! same float64 (`num::dxf_real`): this crate's reader gets every
//! coordinate back bit for bit, and with KentOS's extended data (`xdata`)
//! the same objects. What another program cannot show as KentOS does
//! (labels, attributes, symbols, layer styles) is counted in the report.
//! The blocks the objects place are BLOCKs, their inserts INSERTs
//! (docs/adr/0144 §5, `blocks.rs`).

mod blocks;
mod entities;
mod hatch_ties;
mod input;
mod layers;
mod layouts;
mod styles;

/// The line weight (mm) a DXF holds for `mm` once written: the nearest of
/// AutoCAD's weights, read back as a reader reads group 370 (docs/adr/0139).
pub fn rounded_weight(mm: f64) -> f64 {
    layers::line_weight(mm).0 as f64 / 100.0
}
mod template;

pub(crate) use input::{Definitions, Objects};
pub use input::{WriteInput, input_from_json};

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use kentos_contracts::{
    BlockId, Bounds, DrawingUnit, DxfWriteInput, ExportReport, TextAlign, Vec2,
};

use super::justify;
use crate::num::dxf_real;
use crate::report::Report;
use crate::units;
use blocks::Written;
use entities::Writer;
use layers::Layers;
use template::MODEL_SPACE;

/// DXF text as AutoCAD writes it: the group code right-aligned in three
/// places, then the value, each on its own CRLF-ended line. The methods
/// stay out of line: they are called from a few hundred places, and the
/// formats module is loaded over the network (CLAUDE.md §20).
#[derive(Default)]
pub(crate) struct Out {
    s: String,
}

impl Out {
    #[inline(never)]
    fn code(&mut self, code: i32) {
        let pad = match code {
            0..=9 => "  ",
            10..=99 => " ",
            _ => "",
        };
        self.s.push_str(pad);
        let _ = write!(self.s, "{code}\r\n");
    }

    #[inline(never)]
    pub fn str(&mut self, code: i32, value: &str) {
        self.code(code);
        self.s.push_str(value);
        self.s.push_str("\r\n");
    }

    #[inline(never)]
    pub fn int(&mut self, code: i32, value: i64) {
        self.code(code);
        let _ = write!(self.s, "{value}\r\n");
    }

    #[inline(never)]
    pub fn real(&mut self, code: i32, value: f64) {
        self.str(code, &dxf_real(value));
    }

    #[inline(never)]
    pub fn handle(&mut self, code: i32, h: u64) {
        self.code(code);
        let _ = write!(self.s, "{h:X}\r\n");
    }

    /// A 2D point (x in `code`, y in `code` + 10).
    pub fn xy(&mut self, code: i32, p: Vec2) {
        self.real(code, p.x);
        self.real(code + 10, p.y);
    }

    /// A point in the plane z = 0.
    pub fn xyz(&mut self, code: i32, p: Vec2) {
        self.xy(code, p);
        self.real(code + 20, 0.0);
    }

    pub fn section(&mut self, name: &str) {
        self.str(0, "SECTION");
        self.str(2, name);
    }

    pub fn xdata(&mut self, groups: &[(i32, String)]) {
        for (code, v) in groups {
            self.str(*code, v);
        }
    }

    /// A text's AcDbText groups (a TEXT's, an ATTRIB's or an ATTDEF's;
    /// docs/adr/0145 §7): where it starts (10; `shown` measured, what a
    /// program that does not justify draws there), its height, its value,
    /// turn (none at 0) and width factor (41, none at 1); justified, its
    /// horizontal justification (72, none at 0) and its alignment point (11),
    /// where it stands. Its vertical justification is returned: it follows the
    /// next subclass marker (a TEXT's 73, an attribute's 74; none at 0).
    pub fn text(&mut self, value: &str, shown: &str, t: &Justified) -> i64 {
        let start = justify::start(t.p, shown, t.height, t.rotation, t.align, t.width_factor);
        self.xyz(10, start);
        self.real(40, t.height);
        self.str(1, value);
        if t.rotation != 0.0 {
            self.real(50, t.rotation);
        }
        if let Some(f) = t.width_factor {
            self.real(41, f);
        }
        let Some(a) = t.align else {
            return 0;
        };
        let (h, v) = justify::groups_of(a);
        if h != 0 {
            self.int(72, h);
        }
        self.xyz(11, t.p);
        v
    }
}

/// Where a text stands and how (docs/adr/0145 §7), as `Out::text` writes it.
pub(crate) struct Justified {
    pub p: Vec2,
    pub height: f64,
    pub rotation: f64,
    pub align: Option<TextAlign>,
    pub width_factor: Option<f64>,
}

/// Handles for the drawing's own records and entities, after the template's.
pub(crate) struct Handles {
    next: u64,
}

impl Handles {
    pub fn take(&mut self) -> u64 {
        let h = self.next;
        self.next += 1;
        h
    }
}

/// The view a file opens with: the middle of the drawing, all of it on screen.
fn view(extent: Option<&Bounds>) -> ((f64, f64), f64) {
    match extent {
        Some(b) => {
            let (w, h) = (b.max_x - b.min_x, b.max_y - b.min_y);
            let height = (h.max(w / 1.34) * 1.1).max(1.0);
            (
                ((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0),
                height,
            )
        }
        None => ((0.0, 0.0), 100.0),
    }
}

struct Header<'a> {
    seed: u64,
    extent: Option<&'a Bounds>,
    decimals: u32,
    grads: bool,
    /// Size of point marks in drawing units, when the drawing has points.
    point_size: Option<f64>,
    /// $INSUNITS: metres, or a local project's unit (docs/adr/0165 §2).
    insunits: i64,
    /// $LTSCALE: the project's line type scale (docs/adr/0313 §6).
    ltscale: f64,
}

fn header(out: &mut Out, h: &Header) {
    out.section("HEADER");
    let var = |out: &mut Out, name: &str| out.str(9, name);
    var(out, "$ACADVER");
    out.str(1, "AC1021");
    var(out, "$ACADMAINTVER");
    out.int(70, 25);
    // AutoCAD 2007 and later read text as UTF-8 whatever the code page says; Turkish is named for older tools.
    var(out, "$DWGCODEPAGE");
    out.str(3, "ANSI_1254");
    var(out, "$INSBASE");
    out.xyz(10, Vec2 { x: 0.0, y: 0.0 });
    let (min, max) = match h.extent {
        Some(b) => (
            Vec2 {
                x: b.min_x,
                y: b.min_y,
            },
            Vec2 {
                x: b.max_x,
                y: b.max_y,
            },
        ),
        None => (Vec2 { x: 1e20, y: 1e20 }, Vec2 { x: -1e20, y: -1e20 }),
    };
    var(out, "$EXTMIN");
    out.xyz(10, min);
    var(out, "$EXTMAX");
    out.xyz(10, max);
    if h.extent.is_some() {
        var(out, "$LIMMIN");
        out.xy(10, min);
        var(out, "$LIMMAX");
        out.xy(10, max);
    }
    var(out, "$LTSCALE");
    out.real(40, h.ltscale);
    var(out, "$TEXTSTYLE");
    out.str(7, "Standard");
    var(out, "$CLAYER");
    out.str(8, "0");
    var(out, "$CELTYPE");
    out.str(6, "ByLayer");
    var(out, "$CECOLOR");
    out.int(62, 256);
    var(out, "$CELTSCALE");
    out.real(40, 1.0);
    var(out, "$DIMSTYLE");
    out.str(2, "Standard");
    // Decimal lengths with the project's decimals; angles in grads or degrees, from east, counter-clockwise.
    var(out, "$LUNITS");
    out.int(70, 2);
    var(out, "$LUPREC");
    out.int(70, i64::from(h.decimals.min(8)));
    var(out, "$AUNITS");
    out.int(70, if h.grads { 2 } else { 0 });
    var(out, "$AUPREC");
    out.int(70, 4);
    var(out, "$ANGBASE");
    out.real(50, 0.0);
    var(out, "$ANGDIR");
    out.int(70, 0);
    // Points as small crosses of a fixed size on paper (a dot is invisible on a map).
    var(out, "$PDMODE");
    out.int(70, if h.point_size.is_some() { 2 } else { 0 });
    var(out, "$PDSIZE");
    out.real(40, h.point_size.unwrap_or(0.0));
    var(out, "$HANDSEED");
    out.handle(5, h.seed);
    var(out, "$TILEMODE");
    out.int(70, 1);
    var(out, "$MEASUREMENT");
    out.int(70, 1);
    var(out, "$CELWEIGHT");
    out.int(370, -1);
    var(out, "$LWDISPLAY");
    out.int(290, 0);
    var(out, "$INSUNITS");
    out.int(70, h.insunits);
    var(out, "$PSTYLEMODE");
    out.int(290, 1);
    var(out, "$EXTNAMES");
    out.int(290, 1);
    out.str(0, "ENDSEC");
}

/// Writes the objects and their layers as an AutoCAD 2007 DXF, with what
/// was written by kind and what changed on the way (Turkish, for the user).
pub fn write(input: &DxfWriteInput) -> (Vec<u8>, ExportReport) {
    // A local project's drawing is written in its unit (docs/adr/0165 §2).
    let unit = input.unit.unwrap_or(DrawingUnit::M);
    let to = units::to_unit(unit);
    let scaled;
    let input = if to.is_one() {
        input
    } else {
        scaled = in_unit(input, to);
        &scaled
    };
    let mut report = Report::default();
    // The caller's notes first (docs/adr/0319 §5).
    for n in &input.notes {
        report.note_item(n.clone());
    }
    // A wall goes as the lines and hatches it draws at its detail (docs/adr/0339 §4): DXF has no wall of KentOS's.
    let plain;
    let mut wholes = BTreeMap::new();
    let input = match plain_walls(input, &mut report) {
        Some((p, w)) => {
            plain = p;
            wholes = w;
            &plain
        }
        None => input,
    };
    let scale = if input.scale.is_finite() && input.scale > 0.0 {
        input.scale
    } else {
        report.note(
            "Çizim ölçeği",
            "geçersizdi; çizgi tipi desenleri 1:1000'e göre boyutlandı",
            0,
        );
        1000.0
    };
    let mut handles = Handles {
        next: template::FIRST_FREE,
    };
    let papers = layouts::allocate(&input.layouts, &mut handles);
    // The associative hatches and their sources take their handles first (docs/adr/0333 §3).
    let ties = hatch_ties::ties_of(input, &mut handles);
    let mut layers = Layers::new(&input.layers, &mut report);
    let viewport_no_plot = if input
        .layouts
        .iter()
        .flat_map(|p| &p.viewports)
        .any(|v| !v.frame_plot)
    {
        layers.viewport_frame_layer(false)
    } else {
        "0".to_owned()
    };
    let viewport_plot = if input
        .layouts
        .iter()
        .flat_map(|p| &p.viewports)
        .any(|v| v.frame_plot)
    {
        layers.viewport_frame_layer(true)
    } else {
        "0".to_owned()
    };
    let mut body = Out::default();
    // The blocks (the drawing's and the dimensions' own) and their block records.
    let mut blocks = Out::default();
    let mut records = Vec::new();
    // The blocks the objects place, each written before the inserts of it (docs/adr/0144 §5).
    let order = blocks::order(&input.blocks, &input.entities);
    let names = blocks::names(order.iter().copied(), &mut report);
    let mut defined: HashMap<BlockId, Written> = HashMap::new();
    let mut points = false;
    let no_values = BTreeMap::new();
    let no_wholes = BTreeMap::new();
    // The dimensions' own blocks are numbered on their own: *D1, *D2 … after the drawing's;
    // the tables' (docs/adr/0184 §7) too: *U1, *U2 ….
    let mut dimensions = 0usize;
    let mut anonymous = 0usize;
    // The line types dimensions' lines take, their LTYPE records' handles given as they are met (docs/adr/0205 §6).
    let mut ltypes: Vec<(kentos_contracts::LineType, u64)> = Vec::new();
    // AutoCAD's arrow blocks leaders' arrowheads need, written as they are met (docs/adr/0205 §7).
    let mut arrows: Vec<(kentos_contracts::LeaderArrow, u64)> = Vec::new();
    // What an insert's attribute texts show, where (docs/adr/0144 §7): the shared core's.
    let placing = crate::blocks::Placing::with_attributes(&input.blocks);
    // The STYLE and DIMSTYLE records' names: the project's styles, the styleless faces' (docs/adr/0183 §7).
    let style_names = styles::StyleNames::new(
        &input.text_styles,
        &input.dimension_styles,
        input
            .entities
            .iter()
            .chain(input.blocks.iter().flat_map(|b| b.entities.iter())),
    );
    for def in order {
        let Some(name) = names.get(&def.id) else {
            continue;
        };
        let record = handles.take();
        let mut objects = Out::default();
        let extent = {
            let mut w = Writer {
                out: &mut objects,
                uids: &input.uids,
                blocks: &mut blocks,
                records: &mut records,
                dimensions: &mut dimensions,
                anonymous: &mut anonymous,
                values: &no_values,
                decimals: input.length_decimals,
                grads: input.grads,
                handles: &mut handles,
                layers: &layers,
                report: &mut report,
                extent: None,
                points: false,
                owner: record,
                paper: None,
                defining: true,
                names: &names,
                defined: &defined,
                placing: &placing,
                styles: &style_names,
                per_metre: unit.per_metre(),
                ltypes: &mut ltypes,
                arrows: &mut arrows,
                pen: entities::Pen::default(),
                forms: Default::default(),
                ties: &ties,
                wholes: &no_wholes,
            };
            for e in &def.entities {
                w.entity(e);
            }
            points |= w.points;
            w.extent
        };
        let tags = blocks::tags(def, name, &mut report);
        blocks::block(
            &mut blocks,
            &mut handles,
            &mut report,
            (record, name),
            def,
            &objects,
            &tags,
        );
        // Its insertion unit (docs/adr/0315 §4): the file's, its numbers being in it; 0 for a unitless block.
        let code = if def.units == Some(kentos_contracts::BlockUnits::Unitless) {
            0
        } else {
            units::insunits_of(unit)
        };
        records.push((record, name.clone(), code));
        defined.insert(
            def.id,
            Written {
                base: def.base,
                extent,
                tags,
            },
        );
    }
    let extent = {
        let mut w = Writer {
            out: &mut body,
            uids: &input.uids,
            blocks: &mut blocks,
            records: &mut records,
            dimensions: &mut dimensions,
            anonymous: &mut anonymous,
            values: &input.dimension_values,
            decimals: input.length_decimals,
            grads: input.grads,
            handles: &mut handles,
            layers: &layers,
            report: &mut report,
            extent: None,
            points: false,
            owner: MODEL_SPACE,
            paper: None,
            defining: false,
            names: &names,
            defined: &defined,
            placing: &placing,
            styles: &style_names,
            per_metre: unit.per_metre(),
            ltypes: &mut ltypes,
            arrows: &mut arrows,
            pen: entities::Pen::default(),
            forms: input.annotations,
            ties: &ties,
            wholes: &wholes,
        };
        // In their drawing order (docs/adr/0260 §5): AutoCAD draws a file's objects in its order.
        let mut drawn: Vec<&kentos_contracts::Entity> = input.entities.iter().collect();
        if drawn.iter().any(|e| e.base().look.order.is_some()) {
            drawn.sort_by_key(|e| e.base().look.order.unwrap_or(0));
        }
        for e in drawn {
            w.entity(e);
        }
        points |= w.points;
        w.extent
    };
    let mut paper_body = Out::default();
    for paper in &papers {
        let mut objects = paper.data.entities.clone();
        for e in &mut objects { units::entity(e, units::Scale { mul: 1000., div: 1. }); }
        {
            let mut w = Writer {
                out: &mut paper_body, uids: &input.uids, blocks: &mut blocks, records: &mut records,
                dimensions: &mut dimensions, anonymous: &mut anonymous, values: &no_values,
                decimals: input.length_decimals, grads: input.grads, handles: &mut handles,
                layers: &layers, report: &mut report, extent: None, points: false,
                owner: paper.record, paper: Some(&paper.data.name), defining: false,
                names: &names, defined: &defined, placing: &placing, styles: &style_names,
                per_metre: 1000., ltypes: &mut ltypes, arrows: &mut arrows,
                pen: entities::Pen::default(), forms: input.annotations, ties: &ties, wholes: &no_wholes,
            };
            for e in &objects { w.entity(e); }
        }
        // The overall paper editor is viewport 1. DWG does not persist DXF
        // viewport IDs; a real first record keeps model frames numbered >=2.
        let editor = kentos_contracts::CadViewport {
            locked: false,
            paper_clip_mm: Vec::new(),
            frame_plot: true,
            paper_center_mm: Vec2 {
                x: paper.data.width_mm / 2.,
                y: paper.data.height_mm / 2.,
            },
            width_mm: paper.data.width_mm,
            height_mm: paper.data.height_mm,
            view_center: Vec2 { x: 0., y: 0. },
            view_height: paper.data.height_mm / 1000.,
            twist: 0.,
        };
        layouts::viewport(
            &mut paper_body,
            &mut handles,
            paper,
            &editor,
            1,
            unit.per_metre(),
            (&viewport_plot, &viewport_no_plot),
        );
        for (id, view) in paper.data.viewports.iter().enumerate() {
            layouts::viewport(
                &mut paper_body,
                &mut handles,
                paper,
                view,
                id + 2,
                unit.per_metre(),
                (&viewport_plot, &viewport_no_plot),
            );
        }
    }

    let mut tables = Out::default();
    tables.section("TABLES");
    let (centre, height) = view(extent.as_ref());
    template::vport_table(&mut tables, centre, height);
    // Sizes on paper (line type patterns, point marks) in drawing units: metres at the plot scale, in the unit.
    let paper = scale * unit.per_metre();
    // The dimension styles' line types too (docs/adr/0205 §6).
    for s in &input.dimension_styles {
        for t in [s.dim_line_type, s.ext_line_type].into_iter().flatten() {
            entities::ltype_handle(&mut ltypes, &mut handles, t);
        }
    }
    layers.ltype_table(
        &mut tables,
        &mut handles,
        paper,
        &ltypes,
        &input.line_types,
        &mut report,
    );
    layers.layer_table(&mut tables, &mut handles);
    // A style's paper mm in drawing units (docs/adr/0183 §7).
    let length = |mm: f64| mm / 1000.0 * paper;
    styles::style_table(
        &mut tables,
        &mut handles,
        &style_names,
        &input.text_styles,
        &length,
    );
    template::view_ucs(&mut tables);
    template::appid_table(&mut tables);
    styles::dimstyle_table(
        &mut tables,
        &mut handles,
        &style_names,
        &input.dimension_styles,
        &length,
        unit.per_metre(),
        &ltypes,
    );
    template::block_record_table(&mut tables, &records, &papers);
    tables.str(0, "ENDSEC");

    // The header is written last: it holds the next free handle.
    let mut out = Out::default();
    header(
        &mut out,
        &Header {
            seed: handles.next,
            extent: extent.as_ref(),
            decimals: input.length_decimals,
            grads: input.grads,
            point_size: points.then_some(1.5 * paper / 1000.0),
            insunits: units::insunits_of(unit),
            ltscale: input.line_type_scale.unwrap_or(1.0),
        },
    );
    template::classes(&mut out);
    out.s.push_str(&tables.s);
    template::blocks(&mut out, &blocks.s, &papers);
    out.section("ENTITIES");
    out.s.push_str(&body.s);
    out.s.push_str(&paper_body.s);
    out.str(0, "ENDSEC");
    template::objects(&mut out, &papers);
    out.str(0, "EOF");
    (out.s.into_bytes(), report.export())
}

/// The objects and definitions to write in a local project's unit: every
/// coordinate and length scaled, the rest as it is.
/// `input` with every wall as its plain pieces at the input's detail and scale, said in the report; none without walls.
/// With KentOS's data (`annotations.metadata`, docs/adr/0347 §2) each architectural object stays and its pieces go in
/// `wholes` by its id: the writer draws them in an anonymous block whose INSERT carries the object.
fn plain_walls(
    input: &DxfWriteInput,
    report: &mut Report,
) -> Option<(DxfWriteInput, entities::Wholes)> {
    let walls = input
        .entities
        .iter()
        .filter(|e| matches!(e, kentos_contracts::Entity::Wall(_)))
        .count();
    let openings = input
        .entities
        .iter()
        .filter(|e| matches!(e, kentos_contracts::Entity::Opening(_)))
        .count();
    let spaces = input
        .entities
        .iter()
        .filter(|e| matches!(e, kentos_contracts::Entity::Space(_)))
        .count();
    let elements = input
        .entities
        .iter()
        .filter(|e| crate::arch_elements::core_of(e).is_some())
        .count();
    if walls == 0 && openings == 0 && spaces == 0 && elements == 0 {
        return None;
    }
    let scale = if input.scale.is_finite() && input.scale > 0.0 { input.scale } else { 100.0 };
    let detail = input
        .wall_detail
        .unwrap_or_else(|| kentos_contracts::WallDetail::at_scale(scale));
    let view = kentos_geometry_core::geom::wall::View {
        detail: crate::walls::detail(detail),
        scale,
    };
    let mut out = input.clone();
    out.entities = Vec::with_capacity(input.entities.len() + walls * 6);
    let mut left = 0;
    let keep = input.annotations.metadata;
    let mut wholes = entities::Wholes::new();
    let mut put = |out: &mut DxfWriteInput, e: &kentos_contracts::Entity, pieces: Vec<kentos_contracts::Entity>| {
        if keep {
            out.entities.push(e.clone());
            let uid = input.uids.get(&e.base().id).map(|u| u.to_text());
            wholes.insert(e.base().id, (uid, pieces));
        } else {
            out.entities.extend(pieces);
        }
    };
    // Joined as the drawing shows them (docs/adr/0340 §5).
    let plain = crate::walls::plain_joined(&input.entities, view, |id| {
        input.uids.get(&id).map(|u| u.to_text())
    });
    for (e, pieces) in input.entities.iter().zip(plain) {
        match (e, pieces) {
            (
                kentos_contracts::Entity::Wall(_) | kentos_contracts::Entity::Opening(_),
                Some(pieces),
            ) => put(&mut out, e, pieces),
            (kentos_contracts::Entity::Wall(_), None) => left += 1,
            // A space's rings, each a closed polyline (docs/adr/0342).
            (kentos_contracts::Entity::Space(s), _) => {
                let mut pieces = Vec::new();
                for p in &s.space.parts {
                    for r in std::iter::once(&p.outer).chain(&p.holes) {
                        let mut base = s.base.clone();
                        base.attachments.clear();
                        base.label_pins.clear();
                        pieces.push(kentos_contracts::Entity::Polyline(kentos_contracts::PathEntity {
                            base,
                            pts: r.pts.clone(),
                            bulges: r.bulges.clone(),
                            holes: None,
                            zs: None,
                            ms: None,
                            parts: None,
                            closed: true,
                            widths: None,
                        }));
                    }
                }
                put(&mut out, e, pieces);
            }
            // A grid's, column's, stair's or ramp's pieces (docs/adr/0343 §5).
            (e, _) if crate::arch_elements::core_of(e).is_some() => {
                put(&mut out, e, crate::arch_elements::plain_pieces(e, scale).unwrap_or_default());
            }
            _ => out.entities.push(e.clone()),
        }
    }
    if keep {
        if left > 0 {
            report.skip("Duvar", "çizilemeyen duvar (yayı duvarın genişliğinden dar) yazılmadı", 0);
        }
        return Some((out, wholes));
    }
    if elements > 0 {
        report.note_n(
            "Mimari nesne",
            "DXF'te KentOS'un aksı, kolonu, merdiveni, rampası, etiketi ve görünüm işareti yoktur: çizimlerinin çizgileri, daireleri, yazıları ve taramaları olarak yazıldı; etiketlerin ve işaretlerin bağları, ölçüleri ve aks bağları taşınmaz, geri okunmaz",
            0,
            u32::try_from(elements).unwrap_or(u32::MAX),
        );
    }
    if spaces > 0 {
        report.note_n(
            "Mahal",
            "DXF'te KentOS'un mahali yoktur: mahaller sınırlarının kapalı çoklu çizgileri olarak yazıldı; numaraları, adları, ölçme yöntemleri ve kaplamaları taşınmaz, mahal olarak geri okunmaz",
            0,
            u32::try_from(spaces).unwrap_or(u32::MAX),
        );
    }
    if openings > 0 {
        report.note_n(
            "Açıklık",
            "DXF'te KentOS'un açıklığı yoktur: kapılar, pencereler ve boşluklar simgelerinin çizgileri ve yayları olarak yazıldı, duvarlar onların yerinde açık; türleri, numaraları ve yükseklikleri taşınmaz, açıklık olarak geri okunmaz",
            0,
            u32::try_from(openings).unwrap_or(u32::MAX),
        );
    }
    if walls > 0 {
        report.note_n(
            "Duvar",
            &format!(
                "DXF'te KentOS'un duvarı yoktur: duvarlar {} ayrıntıda, birleşimleriyle çizgiler ve taramalar olarak yazıldı; stilleri, bileşenleri, katları, yükseklikleri ve birleşimleri taşınmaz, duvar olarak geri okunmaz",
                detail.label().to_lowercase()
            ),
            0,
            u32::try_from(walls - left).unwrap_or(u32::MAX),
        );
    }
    if left > 0 {
        report.skip("Duvar", "çizilemeyen duvar (yayı duvarın genişliğinden dar) yazılmadı", 0);
    }
    Some((out, wholes))
}

fn in_unit(input: &DxfWriteInput, to: units::Scale) -> DxfWriteInput {
    let mut scaled = input.clone();
    for e in &mut scaled.entities {
        units::entity(e, to);
    }
    for b in &mut scaled.blocks {
        units::block(b, to);
    }
    scaled
}
