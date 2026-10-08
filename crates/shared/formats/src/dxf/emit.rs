//! DXF entities to the app's objects. Each entity is mapped through the
//! transform it lives under (its object coordinate system, then every block
//! insertion above it) and becomes the most faithful kind the model has:
//! a circle stays a circle under rotation and uniform scale and becomes an
//! ellipse under a non-uniform one; bulges stay bulges unless the map
//! stretches them (then the arcs are sampled, and that is reported).
//! Identity maps copy coordinates bit for bit.

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_contracts::blocks::{nesting, turn_of};
use kentos_contracts::{
    ArcEntity, AttributeDefinition, BlockDefinition, BlockId, Bounds, CircleEntity,
    ConstructionEntity, EllipseEntity, Entity, EntityBase, HatchEntity, HatchPattern,
    HatchPatternType, InsertEntity, LineEntity, MAX_LINE_SPACING, MAX_LINE_WEIGHT,
    MAX_WIDTH_FACTOR, MIN_LINE_SPACING, Paragraph, PathEntity, PointEntity, RingGeometry,
    SplineEntity, TableEntity, TextAlign, TextEntity, Vec2, width_factor_ok,
};

use super::aci;
use super::dimension;
use super::entity::{Color, Kind, P3, Parsed, Vertex, Weight};
use super::hatch::{Edge, Hatch, Path};
use super::justify;
use super::leaders::DimStyle;
use super::strings::{mtext_content, text_codes};
use super::xdata::{Meta, caret_decode};
use crate::geom::{
    Similarity, Tf, arc_points, arc_steps, bulge_path_points, bulge_path_zs, dist, ellipse_from,
    finite, has_arcs, ocs_tf, ocs_z, ring_area, ring_contains, to_core, v,
};
use crate::gis::CLOSING_Z;
use crate::math::{TAU, atan2, cos, deg, hypot, norm_angle, rad, sin, sin_cos_deg};
use crate::nurbs;
use crate::report::Report;

mod notes;
mod pattern;

/// Blocks nest at most this deep (a block that inserts itself is caught earlier).
const MAX_DEPTH: usize = 24;
/// A dimension's text height when its style gives none (DIMTXT's metric default, drawing units).
const DIMENSION_HEIGHT: f64 = 2.5;
/// Columns and rows of a MINSERT array opened at most.
const MAX_ARRAY: i64 = 10_000;
/// Curves sampled into points stay within this of the true curve (1 mm).
const SAMPLE_TOLERANCE: f64 = 1e-3;
/// Width of a DXF text in ems: measured in Arimo, the app's face with Arial's widths, the metrics of
/// AutoCAD's Standard style (the file does not say which face drew it).
fn text_em(text: &str) -> f64 {
    kentos_geometry_core::text::width_em(text, kentos_geometry_core::text::Font::from_id("arimo"))
}

#[derive(Clone, Debug)]
pub struct Block {
    /// The name as the file spells it (the library's key is upper case).
    pub name: String,
    /// What the block is (group 4), when the file says.
    pub description: String,
    pub base: P3,
    pub entities: Vec<Parsed>,
    pub xref: bool,
}

/// What the tables and blocks sections defined (read-only while entities are converted).
#[derive(Default)]
pub struct Library {
    pub blocks: HashMap<String, Block>,
    /// The blocks' upper-case names in the file's order.
    pub order: Vec<String>,
    /// Upper-case layer name → the layer's colour for BYBLOCK children of inserts on it.
    pub layer_colors: HashMap<String, String>,
    /// Upper-case layer name → the layer's line weight (mm), for BYBLOCK children of inserts on it.
    pub layer_weights: HashMap<String, f64>,
    /// Upper-case layer name → the name as the LAYER table spells it (DXF layer names ignore case).
    pub layer_names: HashMap<String, String>,
    /// Upper-case text style name → fixed height (0: none).
    pub style_heights: HashMap<String, f64>,
    /// Upper-case dimension style name → what it says of a leader (docs/adr/0146 §8).
    pub dim_styles: HashMap<String, DimStyle>,
    /// A block record's handle → its block's name (a leader's arrowhead block).
    pub block_records: HashMap<u64, String>,
    /// Upper-case text style name → its STYLE record (docs/adr/0183 §7).
    pub text_records: HashMap<String, super::styles::StyleRecord>,
    /// A STYLE record's handle → its upper-case name (a DIMSTYLE's DIMTXSTY).
    pub style_handles: HashMap<u64, String>,
    /// Upper-case dimension style name → its name as written and its variables.
    pub dim_records: HashMap<String, (String, super::styles::DimVars)>,
    /// An LTYPE record's handle → its line type (a DIMSTYLE's DIMLTYPE, docs/adr/0205 §6).
    pub ltype_handles: HashMap<u64, kentos_contracts::LineType>,
}

/// Where an entity lives: the transform down to world XY, the Z map of
/// point elevations, and what layer-0 and BYBLOCK children of an insert take.
#[derive(Clone)]
pub struct Ctx {
    pub tf: Tf,
    pub z_scale: f64,
    pub z_offset: f64,
    pub layer: Option<String>,
    pub byblock: String,
    /// The insert's line weight, for its BYBLOCK children (none: their layer's).
    pub byblock_weight: Option<f64>,
    /// Upper-case names of the blocks being inserted (cycle guard).
    pub chain: Vec<String>,
    /// Inside a dimension's block: its definition points are not drawing content.
    pub in_dimension: bool,
    /// Reading a block kept as a definition (docs/adr/0144 §5): its objects
    /// go on layer 0, their own layer's colour and weight written on them.
    pub definition: bool,
}

impl Ctx {
    /// The world Z of a Z in the object's own frame, under the inserts above it.
    pub fn z(&self, local: f64) -> f64 {
        self.z_offset + self.z_scale * local
    }

    pub fn model() -> Ctx {
        Ctx {
            tf: Tf::IDENTITY,
            z_scale: 1.0,
            z_offset: 0.0,
            layer: None,
            byblock: "ink".to_string(),
            byblock_weight: None,
            chain: Vec::new(),
            in_dimension: false,
            definition: false,
        }
    }

    /// Where a kept block's own objects are read: its own frame, nothing
    /// above it; `name` (upper case) catches an insert of itself.
    pub fn definition(name: &str) -> Ctx {
        Ctx {
            chain: vec![name.to_string()],
            definition: true,
            ..Ctx::model()
        }
    }
}

pub struct Out {
    pub entities: Vec<Entity>,
    pub report: Report,
    pub per_layer: HashMap<String, u32>,
    pub bounds: Option<Bounds>,
    pub limit: usize,
    pub truncated: u32,
    /// Entities and array cells walked so far, and how many may be.
    pub visits: u64,
    pub visit_limit: u64,
    /// The walk stopped at `visit_limit`.
    pub exhausted: bool,
    /// Model space objects by their DXF handle (index into `entities`), and
    /// the polygons KentOS data names as holes of another (index, owner handle).
    pub handles: HashMap<u64, usize>,
    pub holes: Vec<(usize, u64)>,
    /// The attribute definitions of the block being read (docs/adr/0144 §7).
    pub attributes: Vec<AttributeDefinition>,
}

impl Out {
    pub fn new(limit: usize, visit_limit: u64) -> Out {
        Out {
            entities: Vec::new(),
            report: Report::default(),
            per_layer: HashMap::new(),
            bounds: None,
            limit,
            truncated: 0,
            visits: 0,
            visit_limit,
            exhausted: false,
            handles: HashMap::new(),
            holes: Vec::new(),
            attributes: Vec::new(),
        }
    }

    /// Counts one step of the walk; false once the walk may go no further.
    fn visit(&mut self) -> bool {
        if self.visits >= self.visit_limit {
            self.exhausted = true;
            return false;
        }
        self.visits += 1;
        true
    }

    fn extend_bounds(&mut self, p: Vec2) {
        if !finite(p) {
            return;
        }
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
}

fn base(layer: &str, color: Option<String>, line_weight: Option<f64>) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.to_string(),
        color,
        attrs: BTreeMap::new(),
        label: None,
        symbol: None,
        line_weight,
    }
}

fn xy(p: P3) -> Vec2 {
    v(p[0], p[1])
}

/// A TEXT's, an ATTRIB's or an ATTDEF's groups that place it.
struct Text<'a> {
    ext: P3,
    p: P3,
    p2: Option<P3>,
    height: f64,
    rotation: f64,
    text: &'a str,
    halign: i64,
    valign: i64,
    width: f64,
    style: &'a str,
}

/// A text at `anchor` whose baseline runs at `rotation` degrees (object
/// coordinates) as `m` places it: its anchor, turn (degrees, from 0 up to
/// 360), height and what its width factor is multiplied by (a non-uniform
/// scale widens or narrows it: its baseline scales by one factor, its height
/// by another); none when `m` flattens it.
fn mapped_text(m: Tf, anchor: Vec2, rotation: f64, height: f64) -> Option<(Vec2, f64, f64, f64)> {
    let (s, c) = sin_cos_deg(rotation);
    let d = m.linear(v(c, s));
    let u = m.linear(v(-s, c));
    let ld = hypot(d.x, d.y);
    if !(ld > 0.0) {
        return None;
    }
    // The file's own angle when nothing turns it (bit for bit), else the mapped baseline's.
    let mut rot = if m.is_identity() {
        rotation
    } else {
        deg(atan2(d.y, d.x))
    };
    // Mirrored text stays readable, as the app's own mirror does (MIRRTEXT 0).
    if m.det() < 0.0 {
        rot += 180.0;
    }
    let rot = if (0.0..360.0).contains(&rot) {
        rot
    } else {
        rot.rem_euclid(360.0)
    };
    let area = (d.x * u.y - d.y * u.x).abs();
    if !(area > 0.0) {
        return None;
    }
    let (h, widen) = if m.is_identity() {
        (height, 1.0)
    } else {
        (height * area / ld, ld * ld / area)
    };
    Some((
        m.apply(anchor),
        if rot >= 360.0 { 0.0 } else { rot },
        h,
        widen,
    ))
}

/// A width factor as a text holds it: none for 1 (to a billionth, what a
/// turn or a uniform scale leaves of it), at most `MAX_WIDTH_FACTOR`.
fn width_factor_of(f: f64) -> Option<f64> {
    ((f - 1.0).abs() > 1e-9).then_some(f.min(MAX_WIDTH_FACTOR))
}

/// Where a text stands (docs/adr/0145 §7), in object coordinates `m` places.
struct Frame {
    m: Tf,
    /// The point of the text `align` names.
    anchor: Vec2,
    rotation: f64,
    height: f64,
    align: Option<TextAlign>,
    /// 1: neither narrowed nor widened.
    width_factor: f64,
}

/// The world transform of an entity in object coordinates at `elevation`.
fn ocs(ctx: &Ctx, extrusion: P3, elevation: f64) -> Option<Tf> {
    Some(ocs_tf(extrusion, elevation)?.then(&ctx.tf))
}

/// Whether two angles (radians) name the same direction, to a billionth of a radian.
fn same_angle(a: f64, b: f64) -> bool {
    let d = (a - b).rem_euclid(TAU);
    d < 1e-9 || TAU - d < 1e-9
}

/// What KentOS's extended data adds to the objects an entity became: the
/// label, attributes and symbol, and the app's own name of a colour DXF
/// holds as a number (only while the number is still what it names: a
/// colour changed in another program wins), a text's mask. An insert's
/// ATTRIB values are its attributes already and win over KentOS's data
/// under the same tag.
fn apply_meta(meta: &Meta, e: &mut Entity) {
    let b = e.base_mut();
    if meta.label.is_some() {
        b.label.clone_from(&meta.label);
    }
    for (k, v) in &meta.attrs {
        b.attrs.entry(k.clone()).or_insert_with(|| v.clone());
    }
    if meta.symbol.is_some() {
        b.symbol.clone_from(&meta.symbol);
    }
    if let (Some(app), Some(read)) = (&meta.color, &b.color)
        && aci::from_app(app).0.read_back() == *read
    {
        b.color = Some(app.clone());
    }
    // The exact weight, while the entity's 370 still is the DXF weight it rounds to (an edit elsewhere wins).
    if let (Some(app), Some(read)) = (meta.line_weight, b.line_weight)
        && (0.0..=MAX_LINE_WEIGHT).contains(&app)
        && crate::dxf::writer::rounded_weight(app) == read
    {
        b.line_weight = Some(app);
    }
    if !meta.no_z.is_empty() {
        no_elevations(e, &meta.no_z);
    }
    // A mask is KentOS's alone: DXF's TEXT has none (docs/adr/0145 §7).
    if meta.mask
        && let Entity::Text(t) = e
    {
        t.mask = true;
    }
    // A multi-line text's exact turn, while its MTEXT's direction still gives it (docs/adr/0182 §5).
    if let (Some(turn), Entity::Text(t)) = (meta.note_turn, e) {
        let d = (turn - t.rotation).rem_euclid(360.0);
        if d < 1e-9 || 360.0 - d < 1e-9 {
            t.rotation = turn;
        }
    }
}

/// The vertices KentOS's data says have no elevation lose the 0 DXF wrote for
/// them, while it still is 0 (a height edited in another program wins).
fn no_elevations(e: &mut Entity, vertices: &[usize]) {
    match e {
        Entity::Line(l) => {
            if vertices.contains(&0) && l.za == Some(0.0) {
                l.za = None;
            }
            if vertices.contains(&1) && l.zb == Some(0.0) {
                l.zb = None;
            }
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let Some(zs) = p.zs.as_mut() else { return };
            for &i in vertices {
                if let Some(z) = zs.get_mut(i)
                    && *z == Some(0.0)
                {
                    *z = None;
                }
            }
            if zs.iter().all(Option::is_none) {
                p.zs = None;
            }
        }
        _ => {}
    }
}

/// The id the reader gives the `n`th block it keeps (1, 2, …): the app gives
/// each a new one when it takes them in.
fn block_id(n: usize) -> BlockId {
    BlockId((n as u128).to_be_bytes())
}

/// Leaves out every insert inside a definition that places a block holding
/// that definition (a cycle through other blocks; one holding itself is
/// caught as it is read), each said.
fn break_cycles(defs: &mut [BlockDefinition], report: &mut Report) {
    let index: HashMap<BlockId, usize> = defs.iter().enumerate().map(|(i, d)| (d.id, i)).collect();
    loop {
        let mut cut = None;
        'find: for (a, d) in defs.iter().enumerate() {
            for (k, e) in d.entities.iter().enumerate() {
                if let Entity::Insert(i) = e
                    && let Some(&b) = index.get(&i.block)
                    && reaches(defs, &index, b, a)
                {
                    cut = Some((a, k));
                    break 'find;
                }
            }
        }
        let Some((a, k)) = cut else {
            return;
        };
        defs[a].entities.remove(k);
        report.skip(
            "Blok (INSERT)",
            &format!(
                "“{}” bloğu başka bloklar yoluyla kendini içeriyor; o yerleştirme alınmadı",
                defs[a].name
            ),
            0,
        );
    }
}

/// Whether the definition at `from` places the one at `to`, directly or
/// through others.
fn reaches(
    defs: &[BlockDefinition],
    index: &HashMap<BlockId, usize>,
    from: usize,
    to: usize,
) -> bool {
    let mut seen = vec![false; defs.len()];
    let mut stack = vec![from];
    while let Some(d) = stack.pop() {
        if d == to {
            return true;
        }
        if std::mem::replace(&mut seen[d], true) {
            continue;
        }
        for e in &defs[d].entities {
            if let Entity::Insert(i) = e
                && let Some(&b) = index.get(&i.block)
            {
                stack.push(b);
            }
        }
    }
    false
}

/// Points that tell where an object lies (for the extent shown before the import).
fn anchor_points(e: &Entity) -> Vec<Vec2> {
    match e {
        Entity::Point(p) => vec![p.p],
        Entity::Line(l) => vec![l.a, l.b],
        Entity::Polyline(p) | Entity::Polygon(p) => p.pts.clone(),
        Entity::Circle(c) => vec![v(c.c.x - c.r, c.c.y - c.r), v(c.c.x + c.r, c.c.y + c.r)],
        Entity::Arc(a) => vec![v(a.c.x - a.r, a.c.y - a.r), v(a.c.x + a.r, a.c.y + a.r)],
        Entity::Ellipse(e) => {
            let r = hypot(e.major.x, e.major.y);
            vec![v(e.c.x - r, e.c.y - r), v(e.c.x + r, e.c.y + r)]
        }
        Entity::Spline(s) => s.pts.clone(),
        Entity::Xline(x) | Entity::Ray(x) => vec![x.p],
        Entity::Text(t) => vec![t.p],
        Entity::Dimension(d) => vec![d.a, d.b],
        Entity::Leader(l) => l.pts.clone(),
        Entity::Hatch(h) => h.ring.clone(),
        Entity::Insert(i) => vec![i.p],
        Entity::Table(t) => crate::blocks::table_corners(t),
        Entity::Image(i) => crate::blocks::image_corners(i),
        Entity::Raster(r) => crate::blocks::raster_corners(r),
    }
}

/// Where an INSERT puts its block: the point, the scales, the turn in
/// degrees and, for a MINSERT, its columns, rows and their spacing.
pub struct Placed {
    pub p: P3,
    pub scale: P3,
    pub rotation: f64,
    pub array: (i64, i64, f64, f64),
}

pub struct Emitter<'l> {
    pub lib: &'l Library,
    pub out: Out,
    /// Blocks kept as definitions (all but Blokları patlat's; docs/adr/0144
    /// §5): upper-case name → the id the reader gave it.
    pub kept: HashMap<String, BlockId>,
    /// Kept blocks with objects on a layer other than 0, and the first one's line.
    off_layer: HashMap<BlockId, u32>,
    /// Blokları patlat: every insert is opened, none kept.
    explode: bool,
    /// A definition's objects are being read: they are not the drawing's.
    defining: bool,
    /// Leaders and notes waiting for each other (docs/adr/0146 §8).
    pending: notes::Pending,
    /// The project's typeface: a style's KentOS has no family for (docs/adr/0183 §7).
    pub project_font: kentos_contracts::DrawingFont,
    /// The text and dimension styles the objects follow, by upper-case name,
    /// in the order met: each one's place is its reader id's number.
    pub text_used: Vec<String>,
    pub dim_used: Vec<String>,
    /// What AutoCAD's own blocks (“_…”) said while read: the drawing's only
    /// if an insert places them (`keep_used`), a leader's arrow block not.
    aside: HashMap<BlockId, Report>,
}

impl<'l> Emitter<'l> {
    pub fn new(lib: &'l Library, out: Out, explode: bool) -> Emitter<'l> {
        Emitter {
            lib,
            out,
            kept: HashMap::new(),
            off_layer: HashMap::new(),
            explode,
            defining: false,
            pending: notes::Pending::default(),
            project_font: kentos_contracts::DrawingFont::Barlow,
            text_used: Vec::new(),
            dim_used: Vec::new(),
            aside: HashMap::new(),
        }
    }

    /// The reader's id of the text style `key` (upper case), numbered as met.
    pub fn text_style_id(&mut self, key: &str) -> String {
        let at = match self.text_used.iter().position(|k| k == key) {
            Some(i) => i,
            None => {
                self.text_used.push(key.to_owned());
                self.text_used.len() - 1
            }
        };
        format!("dxf-text-{}", at + 1)
    }

    /// The reader's id of the dimension style `key` (upper case), numbered as met.
    pub fn dim_style_id(&mut self, key: &str) -> String {
        let at = match self.dim_used.iter().position(|k| k == key) {
            Some(i) => i,
            None => {
                self.dim_used.push(key.to_owned());
                self.dim_used.len() - 1
            }
        };
        format!("dxf-dim-{}", at + 1)
    }

    /// A text's face from its STYLE (7) and its own slant (51), or as KentOS
    /// wrote it (its KENTOS data), its style the reader's (docs/adr/0183 §7).
    /// Standard is Standart: no style, the project's typeface; a record
    /// KentOS wrote for a styleless face gives its face and no style; one the
    /// file does not have gives nothing.
    fn face_of(
        &mut self,
        style: &str,
        own_oblique: Option<f64>,
        meta: Option<&Meta>,
    ) -> kentos_contracts::TextFace {
        let key = style.to_uppercase();
        let record = self
            .lib
            .text_records
            .get(&key)
            .filter(|r| !super::styles::is_standard(&r.name))
            .cloned();
        let id = record
            .as_ref()
            .filter(|r| r.is_style())
            .map(|_| self.text_style_id(&key));
        if let Some(mut f) = meta
            .and_then(|m| m.face.as_deref())
            .and_then(|j| serde_json::from_str::<kentos_contracts::TextFace>(j).ok())
        {
            f.text_style = id;
            return f;
        }
        let Some(r) = record else {
            return Default::default();
        };
        let (font, bold, italic, oblique) = match &r.kentos {
            Some(k) => (k.font, k.bold, k.italic, k.oblique),
            None => {
                let (s, _) =
                    super::styles::text_style(&r, String::new(), self.project_font, &|x| x, 1000.0);
                (s.font, s.bold, s.italic, s.oblique)
            }
        };
        let oblique = match own_oblique {
            Some(0.0) => None,
            Some(o) => kentos_contracts::oblique_holds(o).then_some(o),
            None => oblique,
        };
        kentos_contracts::TextFace {
            text_style: id,
            font: Some(font),
            bold,
            italic,
            oblique,
        }
    }

    /// The dimension style `style` names, as its DIMSTYLE's record: its
    /// reader id (none for Standard: Standart, its look the dimension's own)
    /// and variables; none for one the file does not have.
    fn dim_record(&mut self, style: &str) -> Option<(Option<String>, super::styles::DimVars)> {
        let key = style.to_uppercase();
        let (name, vars) = self.lib.dim_records.get(&key)?.clone();
        let id = (!super::styles::is_standard(&name)).then(|| self.dim_style_id(&key));
        Some((id, vars))
    }

    /// A KentOS dimension's look (its KENTOS data), its style the reader's.
    fn kentos_look(&mut self, style: &str, meta: Option<&Meta>) -> kentos_contracts::DimensionLook {
        let mut look: kentos_contracts::DimensionLook = meta
            .and_then(|m| m.look.as_deref())
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();
        look.dim_style = self
            .lib
            .dim_records
            .get(&style.to_uppercase())
            .filter(|(_, v)| v.kentos.is_some())
            .map(|_| style.to_uppercase())
            .map(|key| self.dim_style_id(&key));
        look
    }

    /// Another program's dimension's look: its DIMSTYLE's variables under
    /// its own changes (DSTYLE), as KentOS's style would give it.
    fn look_of_vars(
        &mut self,
        style: &str,
        overrides: super::styles::DimVars,
    ) -> kentos_contracts::DimensionLook {
        let Some((id, vars)) = self.dim_record(style) else {
            return Default::default();
        };
        let vars = overrides.over(&vars);
        let font = vars
            .dimtxsty
            .and_then(|h| self.lib.style_handles.get(&h))
            .and_then(|name| self.lib.text_records.get(name))
            .filter(|r| !super::styles::is_standard(&r.name))
            .and_then(|r| {
                r.kentos.as_ref().map(|k| k.font).or_else(|| {
                    super::styles::family_of(r.family.as_deref().unwrap_or(&r.file))
                        .or_else(|| super::styles::family_of(&r.file))
                })
            });
        let (def, _) = super::styles::dimension_style(
            &vars,
            id.clone().unwrap_or_default(),
            style,
            &self.lib.block_records,
            &self.lib.ltype_handles,
            font,
            &|x| x,
            1000.0,
        );
        kentos_contracts::DimensionLook {
            dim_style: id,
            ..def.look()
        }
    }

    /// The texts pushed since `start` given their face (docs/adr/0183 §7).
    fn give_faces(
        &mut self,
        start: usize,
        style: &str,
        own_oblique: Option<f64>,
        meta: Option<&Meta>,
    ) {
        if self.out.entities.len() <= start {
            return;
        }
        let face = self.face_of(style, own_oblique, meta);
        if face == Default::default() {
            return;
        }
        for e in &mut self.out.entities[start..] {
            if let Entity::Text(t) = e {
                t.face = face.clone();
            }
        }
    }

    /// Reads every block the file names (not an anonymous one, not an
    /// external reference) as a definition, in the file's order: its objects
    /// in its own frame, on layer 0 (docs/adr/0144 §5), ids 1, 2, … as the
    /// reader numbers them. An insert inside one that would make a block hold
    /// itself is left out, and said. AutoCAD's own blocks (a name starting
    /// with “_”: dimension arrows) are left out unless an insert places them.
    /// None, and the file's inserts opened, when blocks nest deeper than a
    /// drawing allows (the caller reads again with Blokları patlat).
    pub fn define_blocks(&mut self) -> Option<Vec<BlockDefinition>> {
        let lib = self.lib;
        let names: Vec<&String> = lib
            .order
            .iter()
            .filter(|n| !n.starts_with('*') && lib.blocks.get(*n).is_some_and(|b| !b.xref))
            .collect();
        for (i, n) in names.iter().enumerate() {
            self.kept.insert((*n).clone(), block_id(i + 1));
        }
        let mut defs = Vec::with_capacity(names.len());
        for n in names {
            let block = &lib.blocks[n];
            let off = block
                .entities
                .iter()
                .find(|x| x.common.layer != "0" && !matches!(x.kind, Kind::Unsupported(_)));
            if let Some(x) = off {
                self.off_layer.insert(self.kept[n], x.line);
            }
            let drawing = std::mem::take(&mut self.out.entities);
            let handles = std::mem::take(&mut self.out.handles);
            let holes = std::mem::take(&mut self.out.holes);
            let pending = std::mem::take(&mut self.pending);
            // What AutoCAD's own block says is set aside until an insert is known to place it.
            let own = n.starts_with('_');
            let outer = own.then(|| std::mem::take(&mut self.out.report));
            self.defining = true;
            let ctx = Ctx::definition(n);
            for x in &block.entities {
                self.emit(x, &ctx);
            }
            self.notes_done();
            // The rings KentOS wrote as polylines of their own go back into their polygons, as in the drawing.
            self.merge_holes();
            self.defining = false;
            if let Some(outer) = outer {
                let said = std::mem::replace(&mut self.out.report, outer);
                self.aside.insert(self.kept[n], said);
            }
            let entities = std::mem::replace(&mut self.out.entities, drawing);
            self.out.handles = handles;
            self.out.holes = holes;
            self.pending = pending;
            defs.push(BlockDefinition {
                id: self.kept[n],
                name: block.name.clone(),
                base: v(block.base[0], block.base[1]),
                entities,
                attributes: std::mem::take(&mut self.out.attributes),
                description: (!block.description.trim().is_empty())
                    .then(|| block.description.clone()),
            });
        }
        break_cycles(&mut defs, &mut self.out.report);
        // Deeper than a drawing's blocks may nest (the rule of `blocks::nesting`).
        let index: HashMap<BlockId, usize> =
            defs.iter().enumerate().map(|(i, d)| (d.id, i)).collect();
        nesting(&defs, &index).ok()?;
        for d in &mut defs {
            for (k, e) in d.entities.iter_mut().enumerate() {
                e.base_mut().id = k as u32 + 1;
            }
        }
        Some(defs)
    }

    /// Leaves out AutoCAD's own blocks (“_…”) that no insert places, in the
    /// drawing or in a definition kept; says how many blocks came and how many
    /// of them nothing places.
    pub fn keep_used(&mut self, defs: Vec<BlockDefinition>) -> Vec<BlockDefinition> {
        let placed = |id: BlockId, entities: &[Entity]| {
            entities
                .iter()
                .any(|e| matches!(e, Entity::Insert(i) if i.block == id))
        };
        let used: Vec<bool> = defs
            .iter()
            .map(|d| {
                placed(d.id, &self.out.entities) || defs.iter().any(|o| placed(d.id, &o.entities))
            })
            .collect();
        let kept: Vec<(BlockDefinition, bool)> = defs
            .into_iter()
            .zip(used)
            .filter(|(d, used)| *used || !d.name.starts_with('_'))
            .collect();
        // What AutoCAD's own blocks kept said; the others' goes with them.
        for (d, _) in &kept {
            if let Some(said) = self.aside.remove(&d.id) {
                self.out.report.absorb(said);
            }
        }
        self.aside.clear();
        let unused = kept.iter().filter(|(_, used)| !used).count();
        // An insert draws a definition's objects on its own layer (docs/adr/0144 §1): an object on a
        // layer of its own keeps that layer's look, not whether it is hidden or locked.
        for (d, _) in &kept {
            if let Some(&line) = self.off_layer.get(&d.id) {
                self.note(
                    "Blok (BLOCK)",
                    "tanımında 0 dışındaki katmanlarda nesne var: yerleştirmenin katmanında, kendi katmanlarının renk ve kalınlığıyla çizilirler; o katmanların gizliliği ve kilidi uygulanmaz",
                    line,
                );
            }
        }
        if !kept.is_empty() {
            let mut fact = format!("{} tanım", kept.len());
            if unused > 0 {
                fact.push_str(&format!(" ({unused} tanesi yerleştirilmemiş)"));
            }
            self.out.report.fact("Blok", fact);
        }
        kept.into_iter().map(|(d, _)| d).collect()
    }

    fn push(&mut self, e: Entity) {
        // A definition's objects are counted and placed by its inserts, not here.
        if self.defining {
            self.out.entities.push(e);
            return;
        }
        if self.out.entities.len() >= self.out.limit {
            self.out.truncated += 1;
            return;
        }
        let layer = &e.base().layer_id;
        *self.out.per_layer.entry(layer.clone()).or_insert(0) += 1;
        self.out.report.count(e.kind());
        for p in anchor_points(&e) {
            self.out.extend_bounds(p);
        }
        self.out.entities.push(e);
    }

    fn skip(&mut self, what: &str, reason: &str, line: u32) {
        self.out.report.skip(what, reason, line);
    }

    fn note(&mut self, what: &str, reason: &str, line: u32) {
        self.out.report.note(what, reason, line);
    }

    /// The layer an object goes on: children on layer 0 take the insert's layer.
    /// Spelled as the LAYER table spells it: "parsel" on an entity is the table's "PARSEL".
    /// A definition's object on 0 has none of its own (`""`, the block's): its
    /// insert's layer draws it and Patlat puts it there (docs/adr/0144 §3); one
    /// on another layer names it, for Patlat (the import maps it to the
    /// drawing's layer), while its insert's layer draws it.
    fn layer_of(&self, e: &Parsed, ctx: &Ctx) -> String {
        let layer = match (&ctx.layer, e.common.layer.as_str()) {
            (Some(l), "0") => l.clone(),
            (_, name) => self
                .lib
                .layer_names
                .get(&name.to_uppercase())
                .cloned()
                .unwrap_or_else(|| name.to_string()),
        };
        if ctx.definition && layer == "0" {
            String::new()
        } else {
            layer
        }
    }

    /// Its own line weight (docs/adr/0139): none for BYLAYER and the drawing's
    /// default, the insert's for BYBLOCK, else the group's millimetres. In a
    /// definition BYLAYER on a layer other than 0 is that layer's weight, and
    /// BYBLOCK at its top is none (the insert's, when it is drawn).
    fn weight_of(&self, e: &Parsed, ctx: &Ctx) -> Option<f64> {
        match e.common.weight {
            Weight::ByLayer if ctx.definition => {
                let layer = Self::own_layer(e, ctx)?;
                self.lib.layer_weights.get(&layer.to_uppercase()).copied()
            }
            Weight::ByLayer | Weight::Default => None,
            Weight::ByBlock if ctx.definition && ctx.layer.is_none() => None,
            Weight::ByBlock => ctx.byblock_weight,
            Weight::Mm(w) => Some(w),
        }
    }

    /// In a definition, the layer whose look a BYLAYER object keeps: its
    /// own, or an opened insert's for a child on 0; none on 0 itself (the
    /// kept block's inserts give it theirs).
    fn own_layer<'e>(e: &'e Parsed, ctx: &'e Ctx) -> Option<&'e str> {
        let layer = match (&ctx.layer, e.common.layer.as_str()) {
            (Some(l), "0") => l.as_str(),
            (_, name) => name,
        };
        (layer != "0").then_some(layer)
    }

    /// What an insert hands its BYBLOCK children: its own weight, or its layer's.
    fn insert_weight(&self, e: &Parsed, ctx: &Ctx, layer: &str) -> Option<f64> {
        match e.common.weight {
            Weight::ByLayer => self.lib.layer_weights.get(&layer.to_uppercase()).copied(),
            Weight::ByBlock => ctx.byblock_weight,
            Weight::Default => None,
            Weight::Mm(w) => Some(w),
        }
    }

    /// The elevations of an object's vertices from the Z the file gives them
    /// (`locals`, in the object's own frame) under the inserts above it
    /// (docs/adr/0142): none, unless some vertex is above or below 0, or
    /// something says the zeros are heights: `own` (an elevation the object
    /// itself has, a LWPOLYLINE's 38), an insert set at a height, or KentOS's
    /// data. A file's 0 is no elevation: the Z of a 2D drawing is 0. Once one
    /// vertex has an elevation every vertex has its own, 0 too; a Z the maps
    /// make infinite is none.
    fn heights(
        ctx: &Ctx,
        e: &Parsed,
        locals: impl Iterator<Item = f64> + Clone,
        own: bool,
    ) -> Option<Vec<Option<f64>>> {
        let world = |z: f64| Some(ctx.z(z)).filter(|z| z.is_finite());
        let says = own || ctx.z_offset != 0.0 || e.meta.as_ref().is_some_and(|m| m.z);
        // Most drawings have none: look before making a list.
        let some = locals.clone().filter_map(world);
        if !(some.clone().any(|z| z != 0.0) || (says && some.clone().next().is_some())) {
            return None;
        }
        Some(locals.map(world).collect())
    }

    /// The colour override: none for BYLAYER, the insert's colour for BYBLOCK.
    /// In a definition BYLAYER on a layer other than 0 is that layer's colour
    /// (its look kept on layer 0), and BYBLOCK at its top is none (the
    /// insert's colour, when it is drawn).
    fn color_of(&self, e: &Parsed, ctx: &Ctx) -> Option<String> {
        match e.common.color {
            Color::ByLayer if ctx.definition => {
                let layer = Self::own_layer(e, ctx)?;
                self.lib.layer_colors.get(&layer.to_uppercase()).cloned()
            }
            Color::ByLayer => None,
            Color::ByBlock if ctx.definition && ctx.layer.is_none() => None,
            Color::ByBlock => Some(ctx.byblock.clone()),
            Color::Aci(n) => Some(aci::color(n)),
            Color::True(rgb) => Some(aci::true_color(rgb)),
        }
    }

    pub fn emit(&mut self, e: &Parsed, ctx: &Ctx) {
        let start = self.out.entities.len();
        self.emit_kind(e, ctx);
        let end = self.out.entities.len();
        if let Some(meta) = &e.meta {
            for x in &mut self.out.entities[start..end] {
                apply_meta(meta, x);
            }
        }
        // Holes and the polygons they belong to are linked by handle, in model space or in a
        // kept block's own objects (as KentOS writes them).
        let own = ctx.chain.is_empty() || (ctx.definition && ctx.chain.len() == 1);
        if own && end == start + 1 {
            if let Some(h) = e.handle {
                self.out.handles.insert(h, start);
            }
            if let Some(owner) = e.meta.as_ref().and_then(|m| m.hole_of) {
                self.out.holes.push((start, owner));
            }
        }
    }

    /// Moves the polygons KentOS data names as holes into their polygons
    /// ("adalı alan") once every object is read. A hole whose polygon is
    /// missing (not in the file, or not a polygon) stays a polygon of its own.
    pub fn merge_holes(&mut self) {
        // A definition's objects are not counted (its inserts are).
        let counted = !self.defining;
        let out = &mut self.out;
        let mut gone = vec![false; out.entities.len()];
        for (hole, owner) in std::mem::take(&mut out.holes) {
            let Some(&o) = out.handles.get(&owner) else {
                continue;
            };
            if o == hole || gone[o] || gone[hole] {
                continue;
            }
            let ring = match &out.entities[hole] {
                Entity::Polygon(h) => RingGeometry {
                    pts: h.pts.clone(),
                    bulges: h.bulges.clone(),
                    zs: h.zs.clone(),
                },
                _ => continue,
            };
            if let Entity::Polygon(p) = &mut out.entities[o] {
                p.holes.get_or_insert_with(Vec::new).push(ring);
                gone[hole] = true;
            }
        }
        if !gone.contains(&true) {
            return;
        }
        let mut i = 0;
        let (per_layer, report) = (&mut out.per_layer, &mut out.report);
        out.entities.retain(|e| {
            let keep = !gone[i];
            i += 1;
            if !keep
                && counted
                && let Entity::Polygon(p) = e
            {
                if let Some(n) = per_layer.get_mut(&p.base.layer_id) {
                    *n -= 1;
                }
                report.uncount("polygon");
            }
            keep
        });
    }

    fn emit_kind(&mut self, e: &Parsed, ctx: &Ctx) {
        if !self.out.visit() {
            return;
        }
        if e.common.paper {
            self.skip(
                "Kâğıt uzayı nesnesi",
                "pafta düzenindeki (kâğıt uzayı) nesneler alınmaz; yalnız model uzayı alınır",
                e.line,
            );
            return;
        }
        if e.common.invisible {
            self.skip(&e.name, "görünmez olarak işaretli", e.line);
            return;
        }
        let layer = self.layer_of(e, ctx);
        let color = self.color_of(e, ctx);
        let weight = self.weight_of(e, ctx);
        let b = || base(&layer, color.clone(), weight);
        let ext = e.common.extrusion;
        match &e.kind {
            Kind::Line { a, b: bb } => {
                let zs = Self::heights(ctx, e, [a[2], bb[2]].into_iter(), false);
                let (za, zb) = match zs.as_deref() {
                    Some([za, zb]) => (*za, *zb),
                    _ => (None, None),
                };
                let (a, bb) = (ctx.tf.apply(xy(*a)), ctx.tf.apply(xy(*bb)));
                self.push(Entity::Line(LineEntity {
                    base: b(),
                    a,
                    b: bb,
                    za,
                    zb,
                }));
            }
            Kind::Point { p } => {
                if ctx.in_dimension {
                    return;
                }
                let z = ctx.z_offset + ctx.z_scale * p[2];
                // KentOS data says when an elevation of 0 is data, not the lack of one (as it does of a line's and a path's).
                let kept = e.meta.as_ref().is_some_and(|m| m.z);
                self.push(Entity::Point(PointEntity {
                    base: b(),
                    p: ctx.tf.apply(xy(*p)),
                    z: (z != 0.0 || kept).then_some(z),
                    parts: None,
                }));
            }
            Kind::Circle { c, r } => {
                let Some(m) = ocs(ctx, ext, c[2]) else {
                    return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line);
                };
                if !(*r > 0.0) {
                    return self.skip(&e.name, "yarıçapı sıfır ya da negatif", e.line);
                }
                self.circle_or_arc(m, xy(*c), *r, None, b(), e);
            }
            Kind::Arc { c, r, a0, a1 } => {
                let Some(m) = ocs(ctx, ext, c[2]) else {
                    return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line);
                };
                if !(*r > 0.0) {
                    return self.skip(&e.name, "yarıçapı sıfır ya da negatif", e.line);
                }
                let span = (a1 - a0).rem_euclid(360.0);
                let before = self.out.entities.len();
                self.circle_or_arc(m, xy(*c), *r, (span != 0.0).then_some((*a0, *a1)), b(), e);
                // KentOS data holds the radians the degrees round; they count while the arc still has those angles.
                if let (Some((x0, x1)), true) =
                    (e.meta.as_ref().and_then(|m| m.arc), m.is_identity())
                    && let Some(Entity::Arc(arc)) = self.out.entities.get_mut(before)
                    && same_angle(arc.a0, x0)
                    && same_angle(arc.a1, x1)
                {
                    (arc.a0, arc.a1) = (x0, x1);
                }
            }
            Kind::Ellipse {
                c,
                major,
                ratio,
                t0,
                t1,
            } => self.ellipse(ctx, ext, *c, *major, *ratio, *t0, *t1, b(), e),
            Kind::LwPolyline {
                pts,
                bulges,
                closed,
                elevation,
                width,
            } => {
                let Some(m) = ocs(ctx, ext, *elevation) else {
                    return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line);
                };
                if *width {
                    self.note(
                        "Genişlikli çoklu çizgi",
                        "genişlik (kalınlık) alınmadı; çizgi ekseniyle geldi",
                        e.line,
                    );
                }
                // An elevation of the plane is every vertex's (a mirrored plane's Z runs the other way).
                let zs = Self::heights(
                    ctx,
                    e,
                    pts.iter().map(|p| ocs_z(ext, p[0], p[1], *elevation)),
                    *elevation != 0.0,
                );
                let pts: Vec<Vec2> = pts.iter().map(|p| v(p[0], p[1])).collect();
                self.path(m, pts, bulges.clone(), *closed, zs, b(), e);
            }
            Kind::Polyline {
                flags,
                verts,
                elevation,
                width,
            } => self.polyline(ctx, ext, *flags, verts, *elevation, *width, b(), e),
            Kind::Spline {
                flags,
                degree,
                knots,
                weights,
                ctrl,
                fit,
            } => self.spline(ctx, *flags, *degree, knots, weights, ctrl, fit, b(), e),
            Kind::Text {
                p,
                p2,
                height,
                rotation,
                text,
                halign,
                valign,
                width,
                style,
                oblique,
                hidden,
                tag,
                prompt,
                constant,
            } => {
                // An attribute definition (ATTDEF, docs/adr/0144 §7).
                if let Some(prompt) = prompt {
                    let def = Text {
                        ext,
                        p: *p,
                        p2: *p2,
                        height: *height,
                        rotation: *rotation,
                        text,
                        halign: *halign,
                        valign: *valign,
                        width: *width,
                        style,
                    };
                    return self.attribute_definition(
                        ctx,
                        &def,
                        tag,
                        prompt,
                        *hidden,
                        *constant,
                        b(),
                        e,
                    );
                }
                if *hidden {
                    return self.skip(
                        "Görünmez öznitelik (ATTRIB)",
                        "blok özniteliği görünmez olarak işaretli",
                        e.line,
                    );
                }
                let start = self.out.entities.len();
                self.text(
                    ctx,
                    ext,
                    *p,
                    *p2,
                    *height,
                    *rotation,
                    text,
                    *halign,
                    *valign,
                    *width,
                    style,
                    b(),
                    e,
                );
                self.give_faces(start, style, *oblique, e.meta.as_ref());
            }
            Kind::MText {
                p,
                height,
                attach,
                xdir,
                rotation,
                text,
                width,
                spacing,
                fill,
                style,
            } => {
                let start = self.out.entities.len();
                self.mtext(
                    ctx,
                    ext,
                    *p,
                    *height,
                    *attach,
                    *xdir,
                    *rotation,
                    text,
                    *width,
                    *spacing,
                    *fill,
                    style,
                    b(),
                    e,
                );
                self.give_faces(start, style, None, e.meta.as_ref());
                // A leader's note (docs/adr/0146 §8).
                if let Some(h) = e.handle {
                    self.mtext_written(h, start);
                }
            }
            Kind::Face { pts, solid } => {
                // SOLID and TRACE are in object coordinates and run 1 2 4 3; 3DFACE is in world coordinates.
                let m = if *solid {
                    ocs(ctx, ext, pts[0][2])
                } else {
                    Some(ctx.tf)
                };
                let Some(m) = m else {
                    return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line);
                };
                let order: [usize; 4] = if *solid { [0, 1, 3, 2] } else { [0, 1, 2, 3] };
                let mut ring: Vec<Vec2> = order.iter().map(|&i| m.apply(xy(pts[i]))).collect();
                ring.dedup();
                if ring.len() > 1 && ring.first() == ring.last() {
                    ring.pop();
                }
                if ring.len() < 3 {
                    return self.skip(&e.name, "alanı olmayan (köşeleri çakışık) yüzey", e.line);
                }
                self.push(Entity::Polygon(PathEntity {
                    base: b(),
                    pts: ring,
                    bulges: None,
                    holes: None,
                    zs: None,
                    parts: None,
                }));
                if *solid {
                    self.note(
                        "Dolu alan (SOLID, TRACE)",
                        "kapalı alan olarak alındı; dolgusu katman stilinden gelir",
                        e.line,
                    );
                } else {
                    self.note(
                        "3B yüz (3DFACE)",
                        "kapalı alan olarak alındı; Z değerleri alınmadı",
                        e.line,
                    );
                }
            }
            Kind::Insert {
                name,
                p,
                scale,
                rotation,
                cols,
                rows,
                dc,
                dr,
                attribs,
                bad_attribs,
            } => {
                let attrs: BTreeMap<String, String> = attribs
                    .iter()
                    .filter_map(|a| match &a.kind {
                        Kind::Text { tag, text, .. } if !tag.is_empty() => {
                            Some((tag.clone(), text.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                let placed = Placed {
                    p: *p,
                    scale: *scale,
                    rotation: *rotation,
                    array: (*cols, *rows, *dc, *dr),
                };
                // A KentOS table (docs/adr/0184 §7): its INSERT carries it; else its block's lines and words.
                if ctx.chain.is_empty()
                    && let Some(json) = e.meta.as_ref().and_then(|m| m.table.as_deref())
                {
                    match self.table_of(e, ctx, &layer, json, &placed) {
                        Ok(t) => return self.push(Entity::Table(t)),
                        Err(why) => self.note("Tablo", why, e.line),
                    }
                }
                // A KentOS text along a curve (docs/adr/0196 §5): its INSERT carries it; else its block's letters.
                if ctx.chain.is_empty()
                    && let Some(json) = e.meta.as_ref().and_then(|m| m.along.as_deref())
                {
                    match self.curved_of(e, ctx, &layer, json, &placed) {
                        Ok(t) => return self.push(Entity::Text(t)),
                        Err(why) => self.note("Eğri boyunca yazı", why, e.line),
                    }
                }
                if self.keep_insert(ctx, e, &layer, name, &placed, attrs) {
                    // Its values are the insert's attributes (docs/adr/0144 §7): the insert shows
                    // those its definition defines; any other shown one comes in as a text too.
                    let defined = self.defined_tags(name);
                    let mut other = false;
                    for a in attribs {
                        if let Kind::Text { tag, hidden, .. } = &a.kind
                            && (*hidden || defined.contains(tag))
                        {
                            continue;
                        }
                        other = true;
                        // Attributes are already placed in the insert's own frame.
                        self.emit(a, ctx);
                    }
                    if other {
                        self.note(
                            "Blok özniteliği (ATTRIB)",
                            "tanımında karşılığı olmayan özniteliğin değeri yerleştirmenin özniteliği oldu; görünen yazısı ayrıca yazı olarak alındı",
                            e.line,
                        );
                    }
                } else {
                    self.insert(
                        ctx, e, &layer, name, *p, *scale, *rotation, *cols, *rows, *dc, *dr,
                    );
                    // Attributes are already placed in the insert's own frame.
                    for a in attribs {
                        self.emit(a, ctx);
                    }
                }
                for (reason, line) in bad_attribs {
                    self.skip("Blok özniteliği (ATTRIB)", reason, *line);
                }
            }
            Kind::Hatch(h) => self.hatch(ctx, ext, h, b(), e),
            Kind::Block { block, what } => self.anonymous_block(ctx, e, &layer, block, what),
            Kind::Dimension {
                block,
                groups,
                style,
                own_style,
                overrides,
            } => {
                // A dimension KentOS wrote comes back as the same dimension, while nothing moved it
                // (at the top of the file, in the plane); another program's ordinate (from the
                // origin), arc length and jogged radius come in as KentOS's own (docs/adr/0147 §8);
                // otherwise its block draws it, as any other's.
                let what = match groups.entity.as_str() {
                    "ARC_DIMENSION" => "Yay uzunluğu ölçüsü (ARC_DIMENSION)",
                    "LARGE_RADIAL_DIMENSION" => "Kırıklı yarıçap ölçüsü (LARGE_RADIAL_DIMENSION)",
                    _ => "Ölçü (DIMENSION)",
                };
                let meta = e.meta.as_ref();
                let own = meta.and_then(|m| m.dimension.as_ref());
                let flat = ctx.tf.is_identity() && ext == [0.0, 0.0, 1.0];
                if let Some(k) = own {
                    let mask = meta.is_some_and(|m| m.mask);
                    match dimension::read_back(groups, k, b(), mask).filter(|_| flat) {
                        Some(mut d) => {
                            // Its look as KentOS wrote it, its style the reader's (docs/adr/0183 §7).
                            d.look = self.kentos_look(style, meta);
                            self.push(Entity::Dimension(d))
                        }
                        None => {
                            self.note(
                                what,
                                "KentOS ölçüsü başka bir programda değiştirilmiş; ölçü olarak geri alınamadı",
                                e.line,
                            );
                            self.anonymous_block(ctx, e, &layer, block, what);
                        }
                    }
                    return;
                }
                // Its style's text height (DIMTXT × DIMSCALE, the entity's own changes first) and fill.
                let named = style;
                let style = own_style.over(self.dim_style(named));
                let height = style.text_length().unwrap_or(DIMENSION_HEIGHT);
                let mask = style.fill == Some(1);
                match dimension::foreign(groups, b(), height, mask) {
                    dimension::Foreign::Taken(mut d) if flat => {
                        // Its look from its DIMSTYLE and its own changes (docs/adr/0183 §7).
                        d.look = self.look_of_vars(named, (**overrides).clone());
                        self.push(Entity::Dimension(*d))
                    }
                    dimension::Foreign::Taken(_) => {
                        self.anonymous_block(ctx, e, &layer, block, what);
                    }
                    dimension::Foreign::Block(why) => {
                        if let Some(why) = why {
                            self.note(what, why, e.line);
                        }
                        self.anonymous_block(ctx, e, &layer, block, what);
                    }
                }
            }
            Kind::Xline { p, dir, ray } => {
                let d = ctx.tf.linear(xy(*dir));
                let l = hypot(d.x, d.y);
                if !(l > 0.0) {
                    return self.skip(&e.name, "doğrultusu yok (dünya düzlemine dik)", e.line);
                }
                // A direction the file gives as a unit vector stays as written (bit for bit).
                let dir = if (l - 1.0).abs() <= 4.0 * f64::EPSILON {
                    d
                } else {
                    v(d.x / l, d.y / l)
                };
                let c = ConstructionEntity {
                    base: b(),
                    p: ctx.tf.apply(xy(*p)),
                    dir,
                };
                self.push(if *ray {
                    Entity::Ray(c)
                } else {
                    Entity::Xline(c)
                });
            }
            Kind::Leader { .. } => self.leader(ctx, b(), e),
            Kind::MLeader(m) => self.mleader(ctx, m, b(), e),
            Kind::Unsupported(name) => {
                let reason = match name.as_str() {
                    "IMAGE" | "WIPEOUT" | "OLE2FRAME" | "OLEFRAME" | "PDFUNDERLAY"
                    | "DWFUNDERLAY" | "DGNUNDERLAY" => {
                        "raster görüntü ve gömülü/altlık nesneleri alınmaz"
                    }
                    "3DSOLID" | "BODY" | "REGION" | "SURFACE" | "PLANESURFACE"
                    | "EXTRUDEDSURFACE" | "LOFTEDSURFACE" | "REVOLVEDSURFACE" | "SWEPTSURFACE"
                    | "MESH" | "POLYFACE" => "3B katı, yüzey ve bölge nesneleri alınmaz",
                    "MLINE" => {
                        "çoklu çizgi (MLINE) alınmaz; AutoCAD'de patlatıp (EXPLODE) yeniden kaydedin"
                    }
                    "VIEWPORT" => "görünüm pencereleri pafta düzenine aittir",
                    "SHAPE" => "şekil (SHAPE) yazı tipi dosyası gerektirir; alınmaz",
                    "TOLERANCE" => "geometrik tolerans çerçevesi alınmaz",
                    _ => "bu nesne türü tanınmıyor",
                };
                self.skip(name, reason, e.line);
            }
        }
    }

    /// A circle (or an arc from `angles`, degrees counter-clockwise) under a map: a circle or arc while the map keeps shapes, else an ellipse.
    fn circle_or_arc(
        &mut self,
        m: Tf,
        c: Vec2,
        r: f64,
        angles: Option<(f64, f64)>,
        b: EntityBase,
        e: &Parsed,
    ) {
        let center = m.apply(c);
        match m.similarity() {
            Some(Similarity {
                scale,
                angle,
                mirror,
            }) => {
                let r = r * scale;
                match angles {
                    None => self.push(Entity::Circle(CircleEntity {
                        base: b,
                        c: center,
                        r,
                    })),
                    Some((a0, a1)) => {
                        let (a0, a1) = (rad(a0), rad(a1));
                        let (s, t) = if mirror {
                            (angle - a1, angle - a0)
                        } else {
                            (a0 + angle, a1 + angle)
                        };
                        self.push(Entity::Arc(ArcEntity {
                            base: b,
                            c: center,
                            r,
                            a0: norm_angle(s),
                            a1: norm_angle(t),
                        }));
                    }
                }
            }
            None => {
                let u = m.linear(v(r, 0.0));
                let w = m.linear(v(0.0, r));
                let (t0, t1) = match angles {
                    None => (0.0, 0.0),
                    Some((a0, a1)) => {
                        let (a0, a1) = (rad(a0), rad(a1));
                        (a0, if a1 > a0 { a1 } else { a1 + TAU })
                    }
                };
                match ellipse_from(center, u, w, t0, t1, angles.is_none()) {
                    Some(el) => {
                        self.push(Entity::Ellipse(EllipseEntity {
                            base: b,
                            c: el.c,
                            major: el.major,
                            ratio: el.ratio,
                            t0: el.t0,
                            t1: el.t1,
                        }));
                        self.note(
                            &e.name,
                            "eşit olmayan ölçekle eklendiği için elips oldu",
                            e.line,
                        );
                    }
                    None => self.skip(
                        &e.name,
                        "görüş doğrultusuna dik (çizgi gibi görünen) çember",
                        e.line,
                    ),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ellipse(
        &mut self,
        ctx: &Ctx,
        ext: P3,
        c: P3,
        major: P3,
        ratio: f64,
        t0: f64,
        t1: f64,
        b: EntityBase,
        e: &Parsed,
    ) {
        if !(ratio > 0.0 && ratio <= 1.0) {
            return self.skip(&e.name, "eksen oranı 0 ile 1 arasında değil", e.line);
        }
        let full = (t1 - t0).abs() >= TAU - 1e-9 || t1 == t0;
        if ctx.tf.is_identity() && ext == [0.0, 0.0, 1.0] && major[2] == 0.0 {
            // The model keeps DXF's own form: copy it.
            let (t0, t1) = if full {
                (0.0, 0.0)
            } else {
                (norm_angle(t0), norm_angle(t1))
            };
            if !(hypot(major[0], major[1]) > 0.0) {
                return self.skip(&e.name, "büyük ekseni sıfır", e.line);
            }
            return self.push(Entity::Ellipse(EllipseEntity {
                base: b,
                c: xy(c),
                major: xy(major),
                ratio,
                t0,
                t1,
            }));
        }
        // The minor axis is the normal × major (both in world coordinates); only their plan views count.
        let l = (ext[0] * ext[0] + ext[1] * ext[1] + ext[2] * ext[2]).sqrt();
        if !(l > 0.0) {
            return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line);
        }
        let n = [ext[0] / l, ext[1] / l, ext[2] / l];
        let minor = [
            (n[1] * major[2] - n[2] * major[1]) * ratio,
            (n[2] * major[0] - n[0] * major[2]) * ratio,
        ];
        let u = ctx.tf.linear(v(major[0], major[1]));
        let w = ctx.tf.linear(v(minor[0], minor[1]));
        let (t0, t1) = if t1 > t0 { (t0, t1) } else { (t0, t1 + TAU) };
        match ellipse_from(ctx.tf.apply(xy(c)), u, w, t0, t1, full) {
            Some(el) => self.push(Entity::Ellipse(EllipseEntity {
                base: b,
                c: el.c,
                major: el.major,
                ratio: el.ratio,
                t0: el.t0,
                t1: el.t1,
            })),
            None => self.skip(
                &e.name,
                "görüş doğrultusuna dik (çizgi gibi görünen) elips",
                e.line,
            ),
        }
    }

    /// A vertex path (LWPOLYLINE, POLYLINE) in object coordinates, with the
    /// elevation of each vertex when it has them (`heights`): bulges kept while
    /// the map keeps shapes.
    #[allow(clippy::too_many_arguments)]
    fn path(
        &mut self,
        m: Tf,
        pts: Vec<Vec2>,
        mut bulges: Vec<f64>,
        closed: bool,
        zs: Option<Vec<Option<f64>>>,
        b: EntityBase,
        e: &Parsed,
    ) {
        let mut pts = pts;
        let mut zs = zs.filter(|z| z.len() == pts.len());
        bulges.resize(pts.len(), 0.0);
        // A closed path that repeats its first vertex: the repeat is not a corner.
        if closed && pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
            bulges.pop();
            if let Some(z) = zs.as_mut() {
                // Its own height goes with it; the first vertex keeps its own, and that is said when they differ.
                let last = z.pop().flatten();
                if last.is_some() && last != z.first().copied().flatten() {
                    self.note(CLOSING_Z.0, CLOSING_Z.1, e.line);
                }
            }
        }
        if pts.len() < 2 {
            return self.skip(&e.name, "iki köşesi yok", e.line);
        }
        let (pts, bulges, zs) = match (has_arcs(&bulges), m.similarity()) {
            (_, Some(s)) => {
                let pts: Vec<Vec2> = pts.iter().map(|p| m.apply(*p)).collect();
                let bulges: Vec<f64> = if s.mirror {
                    bulges.iter().map(|x| -x).collect()
                } else {
                    bulges
                };
                (pts, bulges, zs)
            }
            (false, None) => (pts.iter().map(|p| m.apply(*p)).collect(), bulges, zs),
            (true, None) => {
                // A stretched arc is an elliptic arc; the model's paths hold circular arcs only.
                let ring = bulge_path_points(&pts, &bulges, closed);
                // The points sampled along an arc edge take the elevation the edge has there.
                let zs = zs.map(|z| bulge_path_zs(&pts, &bulges, &z, closed));
                self.note(
                    &e.name,
                    "yaylı kenarları eşit olmayan ölçekle eklendiği için noktalara bölündü",
                    e.line,
                );
                let n = ring.len();
                (ring.iter().map(|p| m.apply(*p)).collect(), vec![0.0; n], zs)
            }
        };
        let zs = zs.filter(|z| z.len() == pts.len() && z.iter().any(Option::is_some));
        let open = !closed;
        let mut bulges = bulges;
        if open {
            bulges.truncate(pts.len().saturating_sub(1));
        }
        let bulges = bulges.iter().any(|&x| x != 0.0).then_some(bulges);
        if closed && (pts.len() >= 3 || bulges.is_some()) {
            self.push(Entity::Polygon(PathEntity {
                base: b,
                pts,
                bulges,
                holes: None,
                zs,
                parts: None,
            }));
        } else {
            self.push(Entity::Polyline(PathEntity {
                base: b,
                pts,
                bulges,
                holes: None,
                zs,
                parts: None,
            }));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn polyline(
        &mut self,
        ctx: &Ctx,
        ext: P3,
        flags: i64,
        verts: &[Vertex],
        elevation: f64,
        width: bool,
        b: EntityBase,
        e: &Parsed,
    ) {
        if flags & (16 | 64) != 0 {
            return self.skip(
                "3B ağ (POLYLINE)",
                "çokyüzlü ve ızgara ağları alınmaz",
                e.line,
            );
        }
        // Spline frame control points are not on the curve.
        let verts: Vec<&Vertex> = verts.iter().filter(|x| x.flags & 16 == 0).collect();
        let closed = flags & 1 == 1;
        if width {
            self.note(
                "Genişlikli çoklu çizgi",
                "genişlik (kalınlık) alınmadı; çizgi ekseniyle geldi",
                e.line,
            );
        }
        if flags & 8 != 0 {
            // 3D polyline: world coordinates, straight segments, a height at each vertex.
            let pts: Vec<Vec2> = verts.iter().map(|x| xy(x.p)).collect();
            let zs = Self::heights(ctx, e, verts.iter().map(|x| x.p[2]), false);
            return self.path(ctx.tf, pts, Vec::new(), closed, zs, b, e);
        }
        let Some(m) = ocs(ctx, ext, elevation) else {
            return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line);
        };
        // A 2D polyline's elevation is its plane's: every vertex's, as a LWPOLYLINE's.
        let zs = Self::heights(
            ctx,
            e,
            verts.iter().map(|x| ocs_z(ext, x.p[0], x.p[1], elevation)),
            elevation != 0.0,
        );
        let pts: Vec<Vec2> = verts.iter().map(|x| xy(x.p)).collect();
        let bulges: Vec<f64> = verts.iter().map(|x| x.bulge).collect();
        self.path(m, pts, bulges, closed, zs, b, e);
    }

    #[allow(clippy::too_many_arguments)]
    fn spline(
        &mut self,
        ctx: &Ctx,
        flags: i64,
        degree: i64,
        knots: &[f64],
        weights: &[f64],
        ctrl: &[P3],
        fit: &[P3],
        b: EntityBase,
        e: &Parsed,
    ) {
        // KentOS data: the curve is KentOS's own through the fit points, and whether it is closed.
        let own = e.meta.as_ref().and_then(|m| m.curve);
        let closed = own.unwrap_or(flags & 1 == 1);
        if fit.len() >= 2 {
            let mut pts: Vec<Vec2> = fit.iter().map(|p| ctx.tf.apply(xy(*p))).collect();
            if closed && pts.len() > 2 && pts.first() == pts.last() {
                pts.pop();
            }
            if own.is_none() {
                self.note("Eğri (SPLINE)", "geçiş noktalarından KentOS eğrisi (Catmull-Rom) olarak kuruldu; noktalar arasındaki biçim küçük farklar gösterebilir", e.line);
            }
            if ctx.tf.similarity().is_none() {
                self.note(
                    "Eğri (SPLINE)",
                    "eşit olmayan ölçekle eklendi; geçiş noktaları dönüştürüldü, eğri yaklaşık",
                    e.line,
                );
            }
            return self.push(Entity::Spline(SplineEntity {
                base: b,
                pts,
                closed,
            }));
        }
        let Ok(p) = usize::try_from(degree) else {
            return self.skip("Eğri (SPLINE)", "derecesi geçersiz", e.line);
        };
        if p > nurbs::MAX_DEGREE {
            return self.skip(
                "Eğri (SPLINE)",
                &format!(
                    "derecesi {p}; en çok {} okunur (AutoCAD en çok 11 yazar)",
                    nurbs::MAX_DEGREE
                ),
                e.line,
            );
        }
        if p == 0 || ctrl.len() < 2 || knots.len() != ctrl.len() + p + 1 {
            return self.skip(
                "Eğri (SPLINE)",
                "denetim noktası ve düğüm sayıları tutarsız",
                e.line,
            );
        }
        // B-splines keep their shape under affine maps: transform the control points, then sample.
        let cps: Vec<Vec2> = ctrl.iter().map(|q| ctx.tf.apply(xy(*q))).collect();
        let w = (flags & 4 != 0 && weights.len() == cps.len()).then_some(weights);
        let pts = if p == 1 && w.is_none() {
            cps
        } else {
            match nurbs::sample(p, knots, &cps, w, SAMPLE_TOLERANCE) {
                Some(pts) => {
                    self.note(
                        "Denetim noktalı eğri (SPLINE)",
                        "1 mm içinde çoklu çizgiye çevrildi",
                        e.line,
                    );
                    pts
                }
                None => {
                    return self.skip(
                        "Eğri (SPLINE)",
                        "eğri hesaplanamadı (düğümler geçersiz)",
                        e.line,
                    );
                }
            }
        };
        self.path(Tf::IDENTITY, pts, Vec::new(), closed, None, b, e);
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        ctx: &Ctx,
        ext: P3,
        p: P3,
        p2: Option<P3>,
        height: f64,
        rotation: f64,
        text: &str,
        halign: i64,
        valign: i64,
        width: f64,
        style: &str,
        b: EntityBase,
        e: &Parsed,
    ) {
        let words = text_codes(&caret_decode(text));
        if words.trim().is_empty() {
            return self.skip(&e.name, "boş yazı", e.line);
        }
        let t = Text {
            ext,
            p,
            p2,
            height,
            rotation,
            text,
            halign,
            valign,
            width,
            style,
        };
        match self.text_frame(ctx, &t, &words, e) {
            Ok(frame) => self.push_text(&frame, words, false, Paragraph::default(), b),
            Err(why) => self.skip(&e.name, why, e.line),
        }
    }

    /// Where a text stands (docs/adr/0145 §7; `words` its decoded text): a
    /// justified one (72 and 73, an attribute's 72 and 74) on its alignment
    /// point, 11 (10 when the file has none), with that alignment; an aligned
    /// (72 = 3) or fitted (72 = 5) one from 10 towards 11, its height or its
    /// width factor worked out from its width in Arimo, and said. Its width
    /// factor is its 41 when that may be one (else 1, said). Why not, when it
    /// cannot be placed.
    fn text_frame(
        &mut self,
        ctx: &Ctx,
        t: &Text<'_>,
        words: &str,
        e: &Parsed,
    ) -> Result<Frame, &'static str> {
        let mut height = if t.height > 0.0 {
            t.height
        } else {
            self.lib
                .style_heights
                .get(&t.style.to_uppercase())
                .copied()
                .unwrap_or(0.0)
        };
        if !(height > 0.0) {
            return Err("yazı yüksekliği yok");
        }
        let Some(m) = ocs(ctx, t.ext, t.p[2]) else {
            return Err("doğrultusu (210) geçersiz");
        };
        let mut width_factor = t.width;
        if !width_factor_ok(width_factor) {
            self.note(
                "Yazı genişliği (41)",
                "0'dan büyük ve en çok 100 olmayan genişlik çarpanı yerine 1 alındı",
                e.line,
            );
            width_factor = 1.0;
        }
        let start = v(t.p[0], t.p[1]);
        let second = t.p2.map(|q| v(q[0], q[1]));
        if t.halign == justify::ALIGNED || t.halign == justify::FIT {
            let mut rotation = t.rotation;
            let em = text_em(words);
            if let Some(q) = second {
                let run = hypot(q.x - start.x, q.y - start.y);
                if run > 0.0 && em > 0.0 {
                    rotation = deg(atan2(q.y - start.y, q.x - start.x));
                    if t.halign == justify::ALIGNED {
                        height = run / (em * width_factor);
                        self.note(
                            "Hizalı yazı (72 = 3)",
                            "iki noktasının arasına yerleşti; yüksekliği yazının Arimo'daki genişliğinden hesaplandı",
                            e.line,
                        );
                    } else {
                        width_factor = (run / (em * height)).min(MAX_WIDTH_FACTOR);
                        self.note(
                            "Sığdırılmış yazı (72 = 5)",
                            "iki noktasının arasına yerleşti; genişlik çarpanı yazının Arimo'daki genişliğinden hesaplandı",
                            e.line,
                        );
                    }
                }
            }
            return Ok(Frame {
                m,
                anchor: start,
                rotation,
                height,
                align: None,
                width_factor,
            });
        }
        let align = justify::align_of(t.halign, t.valign);
        // AutoCAD writes a justified text's start (10) as it draws it; where the text stands is 11.
        let anchor = match align {
            Some(_) => second.unwrap_or(start),
            None => start,
        };
        Ok(Frame {
            m,
            anchor,
            rotation: t.rotation,
            height,
            align,
            width_factor,
        })
    }

    /// An attribute definition (ATTDEF, docs/adr/0144 §7). A constant one is
    /// a text wherever it is (the block's fixed text). Otherwise, in a block
    /// read as a definition it is one of the definition's attributes: its
    /// tag, prompt and default, placed as a text is (docs/adr/0145 §7: an
    /// aligned or fitted one measured by its default, else its tag); an invisible
    /// one is not taken (the inserts' values stay their attributes); one
    /// without a tag, or with a tag taken before, is left out; each is said.
    /// Outside a definition (the drawing, an opened block) it is no drawing
    /// object: nothing, as ever (an insert's ATTRIBs carry the values).
    #[allow(clippy::too_many_arguments)]
    fn attribute_definition(
        &mut self,
        ctx: &Ctx,
        t: &Text<'_>,
        tag: &str,
        prompt: &str,
        hidden: bool,
        constant: bool,
        b: EntityBase,
        e: &Parsed,
    ) {
        const WHAT: &str = "Blok öznitelik tanımı (ATTDEF)";
        if constant {
            self.note(
                "Sabit öznitelik (ATTDEF)",
                "değeri değişmeyen öznitelik yazı olarak alındı",
                e.line,
            );
            return self.text(
                ctx, t.ext, t.p, t.p2, t.height, t.rotation, t.text, t.halign, t.valign, t.width,
                t.style, b, e,
            );
        }
        if !(self.defining && ctx.definition && ctx.tf.is_identity()) {
            return;
        }
        if hidden {
            return self.skip(
                "Görünmez öznitelik tanımı (ATTDEF)",
                "tanıma alınmadı; yerleştirmelerdeki değerleri öznitelik olarak kalır",
                e.line,
            );
        }
        let tag = tag.trim();
        if tag.is_empty() {
            return self.skip(WHAT, "etiketi yok", e.line);
        }
        if self.out.attributes.iter().any(|a| a.tag == tag) {
            let why = format!("“{tag}” etiketi blokta ikinci kez var; ilki alındı");
            return self.skip(WHAT, &why, e.line);
        }
        let value = text_codes(&caret_decode(t.text));
        let words = if value.trim().is_empty() {
            tag.to_owned()
        } else {
            value.clone()
        };
        let placed = self.text_frame(ctx, t, &words, e).map(|f| {
            let mapped = mapped_text(f.m, f.anchor, f.rotation, f.height);
            (mapped, f)
        });
        match placed {
            Err(why) => self.skip(WHAT, why, e.line),
            Ok((None, _)) => self.skip(WHAT, "doğrultusu (210) geçersiz", e.line),
            Ok((Some((p, rotation, height, widen)), f)) => {
                let prompt = text_codes(&caret_decode(prompt));
                self.out.attributes.push(AttributeDefinition {
                    tag: tag.to_owned(),
                    prompt: (!prompt.trim().is_empty()).then_some(prompt),
                    value: (!value.trim().is_empty()).then_some(value),
                    p,
                    height,
                    rotation,
                    align: f.align,
                    width_factor: width_factor_of(f.width_factor * widen),
                });
            }
        }
    }

    /// The tags a block's attribute definitions show (visible, not constant):
    /// an insert of it shows their ATTRIBs itself (docs/adr/0144 §7).
    fn defined_tags(&self, name: &str) -> HashSet<String> {
        let Some(block) = self.lib.blocks.get(&name.to_uppercase()) else {
            return HashSet::new();
        };
        block
            .entities
            .iter()
            .filter_map(|x| match &x.kind {
                Kind::Text {
                    prompt: Some(_),
                    hidden: false,
                    constant: false,
                    tag,
                    ..
                } if !tag.trim().is_empty() => Some(tag.trim().to_owned()),
                _ => None,
            })
            .collect()
    }

    /// A text where `f` stands it; `mask`: over a mask of its own (an
    /// MTEXT's background fill; KentOS's data gives a TEXT's); `paragraph`:
    /// a multi-line text's box width (the file's, along its baseline),
    /// spacing and formats (docs/adr/0182 §5).
    fn push_text(
        &mut self,
        f: &Frame,
        text: String,
        mask: bool,
        paragraph: Paragraph,
        b: EntityBase,
    ) {
        let Some((p, rotation, height, widen)) = mapped_text(f.m, f.anchor, f.rotation, f.height)
        else {
            return;
        };
        // The box widens as the baseline does: its height's scale times the widening (1 unturned, unscaled).
        let along = height / f.height * widen;
        self.push(Entity::Text(TextEntity {
            base: b,
            p,
            text,
            height,
            rotation,
            align: f.align,
            width_factor: width_factor_of(f.width_factor * widen),
            mask,
            label_of: None,
            label_scale: None,
            paragraph: Paragraph {
                box_width: paragraph.box_width.map(|w| w * along),
                ..paragraph
            },
            face: Default::default(),
            path: None,
        }));
    }

    #[allow(clippy::too_many_arguments)]
    fn mtext(
        &mut self,
        ctx: &Ctx,
        ext: P3,
        p: P3,
        height: f64,
        attach: i64,
        xdir: Option<P3>,
        rotation: Option<f64>,
        text: &str,
        width: f64,
        spacing: f64,
        fill: i64,
        style: &str,
        b: EntityBase,
        e: &Parsed,
    ) {
        let content = mtext_content(&caret_decode(text));
        if content.text.trim().is_empty() {
            return self.skip("Çok satırlı yazı (MTEXT)", "boş yazı", e.line);
        }
        let height = if height > 0.0 {
            height
        } else {
            self.lib
                .style_heights
                .get(&style.to_uppercase())
                .copied()
                .unwrap_or(0.0)
        };
        if !(height > 0.0) {
            return self.skip("Çok satırlı yazı (MTEXT)", "yazı yüksekliği yok", e.line);
        }
        // Direction: the x axis vector (world), else the rotation (radians) in the object plane.
        let dir = match (xdir, rotation) {
            (Some(x), _) if hypot(x[0], x[1]) > 0.0 => {
                let l = hypot(x[0], x[1]);
                v(x[0] / l, x[1] / l)
            }
            (_, Some(r)) => {
                let Some(o) = ocs_tf(ext, 0.0) else {
                    return self.skip(
                        "Çok satırlı yazı (MTEXT)",
                        "doğrultusu (210) geçersiz",
                        e.line,
                    );
                };
                let d = o.linear(v(cos(r), sin(r)));
                let l = hypot(d.x, d.y).max(f64::MIN_POSITIVE);
                v(d.x / l, d.y / l)
            }
            _ => v(1.0, 0.0),
        };
        let angle = deg(atan2(dir.y, dir.x));
        // One multi-line text (docs/adr/0182 §5): its attachment point is its
        // box's (the first line's top, the box's middle or bottom; left,
        // centre or right), its width the box's, 44 its line spacing.
        let frame = Frame {
            m: ctx.tf,
            anchor: v(p[0], p[1]),
            rotation: angle,
            height,
            align: Some(justify::attachment(attach)),
            width_factor: content.width_factor.unwrap_or(1.0),
        };
        let paragraph = Paragraph {
            box_width: (width.is_finite() && width > 0.0).then_some(width),
            line_spacing: (spacing.is_finite() && spacing > 0.0 && spacing != 1.0)
                .then(|| spacing.clamp(MIN_LINE_SPACING, MAX_LINE_SPACING)),
            runs: content.runs,
        };
        // A background fill (90: 1 its colour, 2 the drawing's) is a mask; 16 is a frame alone.
        let mask = fill & 3 != 0;
        self.push_text(&frame, content.text, mask, paragraph, b);
        if !content.dropped.is_empty() {
            self.note(
                "Çok satırlı yazı (MTEXT)",
                &format!(
                    "biçimlendirmesinden kaldırılan: {}",
                    content.dropped.join(", ")
                ),
                e.line,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn insert(
        &mut self,
        ctx: &Ctx,
        e: &Parsed,
        layer: &str,
        name: &str,
        p: P3,
        scale: P3,
        rotation: f64,
        cols: i64,
        rows: i64,
        dc: f64,
        dr: f64,
    ) {
        let key = name.to_uppercase();
        let Some(block) = self.lib.blocks.get(&key) else {
            return self.skip(
                "Blok (INSERT)",
                &format!("“{name}” blok tanımı dosyada yok"),
                e.line,
            );
        };
        if block.xref {
            return self.skip(
                "Dış başvuru (XREF)",
                "dış başvurulu çizimler alınmaz; AutoCAD'de bağlayıp (BIND) yeniden kaydedin",
                e.line,
            );
        }
        if ctx.chain.contains(&key) || ctx.chain.len() >= MAX_DEPTH {
            return self.skip(
                "Blok (INSERT)",
                &format!("“{name}” bloğu kendini içeriyor ya da çok derin iç içe"),
                e.line,
            );
        }
        let Some(o) = ocs_tf(e.common.extrusion, p[2]) else {
            return self.skip("Blok (INSERT)", "doğrultusu (210) geçersiz", e.line);
        };
        let [sx, sy, sz] = scale;
        if sx == 0.0 || sy == 0.0 {
            return self.skip("Blok (INSERT)", "ölçeği sıfır", e.line);
        }
        let byblock = match e.common.color {
            Color::ByLayer => self
                .lib
                .layer_colors
                .get(&layer.to_uppercase())
                .cloned()
                .unwrap_or_else(|| "ink".to_string()),
            Color::ByBlock => ctx.byblock.clone(),
            Color::Aci(n) => aci::color(n),
            Color::True(rgb) => aci::true_color(rgb),
        };
        let byblock_weight = self.insert_weight(e, ctx, layer);
        let nz = super::extrusion_z(e.common.extrusion);
        let mut chain = ctx.chain.clone();
        chain.push(key);
        // An array insert (MINSERT); a huge one stops at the object limit or at the walk's.
        let (ncols, nrows) = (cols.clamp(1, MAX_ARRAY), rows.clamp(1, MAX_ARRAY));
        if ncols < cols || nrows < rows {
            self.skip("Blok dizisi (MINSERT)", &format!("{cols} × {rows} dizinin en çok {MAX_ARRAY} sütunu ve satırı açıldı; kalanı alınmadı"), e.line);
        }
        let (ncols, nrows) = (ncols as usize, nrows as usize);
        for cell in 0..ncols * nrows {
            if !self.out.visit() {
                break;
            }
            if self.out.entities.len() >= self.out.limit {
                self.out.truncated += 1;
                break;
            }
            let (col, row) = ((cell % ncols) as f64, (cell / ncols) as f64);
            let local = Tf::translate(-block.base[0], -block.base[1])
                .then(&Tf::scale(sx, sy))
                .then(&Tf::translate(col * dc, row * dr))
                .then(&Tf::rotate_deg(rotation))
                .then(&Tf::translate(p[0], p[1]))
                .then(&o)
                .then(&ctx.tf);
            let child = Ctx {
                tf: local,
                z_scale: ctx.z_scale * sz * nz,
                z_offset: ctx.z_offset + ctx.z_scale * nz * (p[2] - sz * block.base[2]),
                layer: Some(layer.to_string()),
                byblock: byblock.clone(),
                byblock_weight,
                chain: chain.clone(),
                in_dimension: ctx.in_dimension,
                definition: ctx.definition,
            };
            for child_entity in &block.entities {
                self.emit(child_entity, &child);
            }
        }
        if ctx.chain.is_empty() && self.explode {
            self.note(
                "Blok (INSERT)",
                "Blokları patlat seçili: bloklar patlatılarak, içindeki nesneler olarak alındı",
                e.line,
            );
        }
    }

    /// The KentOS table an INSERT carries in KentOS's data (docs/adr/0184
    /// §7), where the INSERT is now: another program may have moved it. Its
    /// corner and turn are KentOS's exact ones while the INSERT's are still
    /// what KentOS wrote (else the INSERT's), its sizes times the INSERT's
    /// scale. Why not, for the report: the INSERT is read as its block's lines
    /// and words then.
    fn table_of(
        &self,
        e: &Parsed,
        ctx: &Ctx,
        layer: &str,
        json: &str,
        placed: &Placed,
    ) -> Result<TableEntity, &'static str> {
        let [sx, sy, _] = placed.scale;
        let uniform = sx > 0.0 && (sx - sy).abs() <= sx * 1e-9;
        if placed.array.0 * placed.array.1 > 1
            || super::extrusion_z(e.common.extrusion) != 1.0
            || !uniform
        {
            return Err(
                "başka bir programda aynalanmış, eğik düzleme ya da eşit olmayan ölçekle yerleştirilmiş; çizgi ve yazıları alındı",
            );
        }
        let broken = "KentOS verisi okunamadı; çizgi ve yazıları alındı";
        let mut value: serde_json::Value = serde_json::from_str(json).map_err(|_| broken)?;
        let map = value.as_object_mut().ok_or(broken)?;
        map.insert("id".into(), 0.into());
        map.insert("layerId".into(), layer.into());
        map.insert(
            "attrs".into(),
            serde_json::Value::Object(Default::default()),
        );
        let mut t: TableEntity = serde_json::from_value(value).map_err(|_| broken)?;
        let near = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0);
        if !(near(t.p.x, placed.p[0]) && near(t.p.y, placed.p[1])) {
            t.p = v(placed.p[0], placed.p[1]);
        }
        if !near(t.rotation, placed.rotation) {
            t.rotation = placed.rotation;
        }
        if !near(sx, 1.0) {
            t.height *= sx;
            for x in t.rows.iter_mut().chain(t.columns.iter_mut()) {
                *x *= sx;
            }
        }
        t.base = base(layer, self.color_of(e, ctx), self.weight_of(e, ctx));
        // A schedule's objects are the writing drawing's: the objects read come with ids of their own, so the
        // table is its own now (docs/adr/0184 §7); a file's source is still the file.
        if t.source
            .as_ref()
            .is_some_and(|s| !matches!(s, kentos_contracts::TableSource::File { .. }))
        {
            t.source = None;
        }
        if t.shape().problem().is_some() || t.face.problem().is_some() {
            return Err("KentOS verisindeki tablo geçersiz; çizgi ve yazıları alındı");
        }
        Ok(t)
    }

    /// A text along a curve from its INSERT's KentOS data (docs/adr/0196 §5):
    /// where the INSERT is, turned and scaled (evenly) as it is; an
    /// arrayed, mirrored, unevenly scaled or slanted INSERT keeps its letters.
    fn curved_of(
        &self,
        e: &Parsed,
        ctx: &Ctx,
        layer: &str,
        json: &str,
        placed: &Placed,
    ) -> Result<kentos_contracts::TextEntity, &'static str> {
        let [sx, sy, _] = placed.scale;
        let uniform = sx > 0.0 && (sx - sy).abs() <= sx * 1e-9;
        if placed.array.0 * placed.array.1 > 1
            || super::extrusion_z(e.common.extrusion) != 1.0
            || !uniform
        {
            return Err(
                "başka bir programda aynalanmış, eğik düzleme ya da eşit olmayan ölçekle yerleştirilmiş; harfleri alındı",
            );
        }
        let broken = "KentOS verisi okunamadı; harfleri alındı";
        let mut value: serde_json::Value = serde_json::from_str(json).map_err(|_| broken)?;
        let map = value.as_object_mut().ok_or(broken)?;
        map.insert("id".into(), 0.into());
        map.insert("layerId".into(), layer.into());
        map.insert(
            "attrs".into(),
            serde_json::Value::Object(Default::default()),
        );
        let mut t: kentos_contracts::TextEntity =
            serde_json::from_value(value).map_err(|_| broken)?;
        let near = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0);
        if !(near(t.p.x, placed.p[0]) && near(t.p.y, placed.p[1])) {
            t.p = v(placed.p[0], placed.p[1]);
        }
        if !near(t.rotation, placed.rotation) {
            t.rotation = placed.rotation;
        }
        if !near(sx, 1.0) {
            t.height *= sx;
            if let Some(c) = t.path.as_mut() {
                for q in &mut c.pts {
                    *q = v(q.x * sx, q.y * sx);
                }
            }
        }
        t.base = base(layer, self.color_of(e, ctx), self.weight_of(e, ctx));
        // The written file keeps no link (docs/adr/0175 §4).
        t.label_of = None;
        t.label_scale = None;
        let Some(path) = t.path.as_ref() else {
            return Err(broken);
        };
        if kentos_contracts::text_path_problem(path, &t.text, &t.paragraph, false).is_some()
            || t.face.problem().is_some()
            || t.text.trim().is_empty()
            || !(t.height > 0.0)
        {
            return Err("KentOS verisindeki eğri boyunca yazı geçersiz; harfleri alındı");
        }
        Ok(t)
    }

    /// An insert of a block kept as a definition, written as one
    /// (docs/adr/0144 §5): where it is, its scale (the same in X and Y), its
    /// turn and its mirroring, its attributes' values. What a definition's
    /// insert cannot say (an array, unequal scales, a slanted plane, an
    /// insert inside an opened one) is opened instead, and said why; false
    /// then. The insert's Z is not kept (a block's insert is flat).
    fn keep_insert(
        &mut self,
        ctx: &Ctx,
        e: &Parsed,
        layer: &str,
        name: &str,
        placed: &Placed,
        attrs: BTreeMap<String, String>,
    ) -> bool {
        let key = name.to_uppercase();
        let Some(&id) = self.kept.get(&key) else {
            if name.starts_with('*') && !self.explode && ctx.tf.is_identity() {
                self.note(
                    "Adsız blok (INSERT)",
                    "adsız blok (dinamik blok ya da grup) blok olarak tutulmaz; patlatılarak alındı",
                    e.line,
                );
            }
            return false;
        };
        let why = if !ctx.tf.is_identity() {
            Some("açılan bir bloğun içinde; onunla birlikte patlatılarak alındı")
        } else if placed.array.0 * placed.array.1 > 1 {
            Some("blok dizisi (MINSERT) patlatılarak alındı")
        } else if super::extrusion_z(e.common.extrusion) != 1.0 {
            Some("eğik düzlemdeki yerleştirme patlatılarak alındı")
        } else {
            let [sx, sy, _] = placed.scale;
            let equal = sx != 0.0 && (sx.abs() - sy.abs()).abs() <= sx.abs() * 1e-9;
            (!equal).then_some("X ve Y ölçeği eşit olmayan yerleştirme patlatılarak alındı")
        };
        if let Some(why) = why {
            if !ctx.definition || ctx.tf.is_identity() {
                self.note("Blok (INSERT)", why, e.line);
            }
            return false;
        }
        if ctx.chain.contains(&key) {
            self.skip(
                "Blok (INSERT)",
                &format!("“{name}” bloğu kendini içeriyor; o yerleştirme alınmadı"),
                e.line,
            );
            return true;
        }
        let [sx, sy, _] = placed.scale;
        // Mirrored when the two scales have unlike signs; a negative X is a
        // half turn of the mirror in the block's x axis (the model's).
        let mirror = (sx < 0.0) != (sy < 0.0);
        let degrees = placed.rotation + if sx < 0.0 { 180.0 } else { 0.0 };
        if placed.p[2] != 0.0 && !ctx.definition {
            self.note(
                "Blok (INSERT)",
                "yerleştirmenin yüksekliği (Z) alınmadı",
                e.line,
            );
        }
        // KentOS's exact turn while the file's degrees are still those it wrote (an edit elsewhere wins).
        let exact = e
            .meta
            .as_ref()
            .and_then(|m| m.turn)
            .filter(|t| sx > 0.0 && deg(*t) == placed.rotation);
        let mut base = base(layer, self.color_of(e, ctx), self.weight_of(e, ctx));
        base.attrs = attrs;
        self.push(Entity::Insert(InsertEntity {
            base,
            block: id,
            p: v(placed.p[0], placed.p[1]),
            scale: sx.abs(),
            rotation: exact.unwrap_or_else(|| turn_of(degrees)),
            mirror,
        }));
        true
    }

    fn anonymous_block(&mut self, ctx: &Ctx, e: &Parsed, layer: &str, block: &str, what: &str) {
        let key = block.to_uppercase();
        let Some(b) = self.lib.blocks.get(&key) else {
            return self.skip(
                what,
                "çizim bloğu dosyada yok; ölçü yeniden kurulamadı",
                e.line,
            );
        };
        let Some(o) = ocs_tf(e.common.extrusion, 0.0) else {
            return self.skip(what, "doğrultusu (210) geçersiz", e.line);
        };
        let byblock = match e.common.color {
            Color::ByLayer => self
                .lib
                .layer_colors
                .get(&layer.to_uppercase())
                .cloned()
                .unwrap_or_else(|| "ink".to_string()),
            Color::ByBlock => ctx.byblock.clone(),
            Color::Aci(n) => aci::color(n),
            Color::True(rgb) => aci::true_color(rgb),
        };
        let mut chain = ctx.chain.clone();
        chain.push(key);
        let child = Ctx {
            tf: o.then(&ctx.tf),
            layer: Some(layer.to_string()),
            byblock,
            byblock_weight: self.insert_weight(e, ctx, layer),
            chain,
            in_dimension: true,
            ..ctx.clone()
        };
        for x in &b.entities {
            self.emit(x, &child);
        }
        self.note(
            what,
            "çizgi ve yazılara patlatılarak alındı (KentOS ölçüsü olarak düzenlenemez)",
            e.line,
        );
    }

    fn hatch(&mut self, ctx: &Ctx, ext: P3, h: &Hatch, b: EntityBase, e: &Parsed) {
        let Some(m) = ocs(ctx, ext, h.elevation) else {
            return self.skip("Tarama (HATCH)", "doğrultusu (210) geçersiz", e.line);
        };
        let mut rings: Vec<Vec<Vec2>> = Vec::new();
        let mut curved = false;
        for path in &h.paths {
            let Boundary {
                pts: ring,
                curved: arcs,
                rough,
            } = path_points(path);
            curved |= arcs;
            if rough {
                self.note("Tarama (HATCH)", "sınırdaki bir eğri hesaplanamadı (düğümleri geçersiz ya da derecesi çok yüksek); denetim noktalarından geçen çizgiyle alındı", e.line);
            }
            let mut ring: Vec<Vec2> = ring.into_iter().map(|p| m.apply(p)).collect();
            ring.dedup();
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            if ring.len() >= 3 && ring.iter().all(|p| finite(*p)) {
                rings.push(ring);
            }
        }
        if rings.is_empty() {
            return self.skip("Tarama (HATCH)", "sınırı okunamadı ya da alanı yok", e.line);
        }
        if curved {
            self.note("Tarama (HATCH)", "sınırdaki yaylar ve eğriler parçalı alındı (72 parça/tur, uygulamanın taramaları gibi)", e.line);
        }
        if h.assoc {
            self.note(
                "Tarama (HATCH)",
                "ilişkili tarama ilişkisiz alındı; sınır nesnelerini izlemez",
                e.line,
            );
        }
        let mut b = b;
        let mut pattern = self.pattern(m, h, &mut b, e);
        // KentOS data holds a pattern or a gradient exactly; they count while the groups still say it.
        if m.is_identity() {
            pattern = pattern::exact(pattern, e.meta.as_ref().and_then(|x| x.hatch.as_deref()));
        }
        // KentOS data holds the angle and spacing the pattern's offsets round; they count while the pattern still has them.
        if let (Some((angle, spacing)), true) =
            (e.meta.as_ref().and_then(|x| x.pattern), m.is_identity())
            && matches!(
                pattern.kind,
                HatchPatternType::Solid | HatchPatternType::Lines | HatchPatternType::Cross
            )
        {
            // Line families repeat every 180°: compare doubled angles.
            let same = pattern.kind == HatchPatternType::Solid
                || (same_angle(rad(pattern.angle) * 2.0, rad(angle) * 2.0)
                    && (pattern.spacing - spacing).abs() <= 1e-9 * spacing.abs());
            if same {
                (pattern.angle, pattern.spacing) = (angle, spacing);
            }
        }
        // Nesting: even depth is hatched, odd depth is an island of its container.
        let core: Vec<_> = rings.iter().map(|r| to_core(r)).collect();
        let area: Vec<f64> = core.iter().map(|r| ring_area(r)).collect();
        let mut order: Vec<usize> = (0..rings.len()).collect();
        order.sort_by(|&a, &b| area[b].total_cmp(&area[a]));
        let mut parent: Vec<Option<usize>> = vec![None; rings.len()];
        let mut depth = vec![0usize; rings.len()];
        for (k, &i) in order.iter().enumerate() {
            let probe = rings[i][0];
            // The smallest larger ring around it is its container.
            if let Some(&j) = order[..k]
                .iter()
                .rev()
                .find(|&&j| ring_contains(&core[j], probe))
            {
                parent[i] = Some(j);
                depth[i] = depth[j] + 1;
            }
        }
        let max_depth = match h.style {
            2 => 0,
            1 => 1,
            _ => usize::MAX,
        };
        // Hatched rings and their islands in the order the file lists them.
        for i in 0..rings.len() {
            if !depth[i].is_multiple_of(2) || depth[i] > max_depth {
                continue;
            }
            let holes: Vec<Vec<Vec2>> = (0..rings.len())
                .filter(|&j| parent[j] == Some(i) && depth[j] <= max_depth)
                .map(|j| rings[j].clone())
                .collect();
            self.push(Entity::Hatch(HatchEntity {
                base: b.clone(),
                ring: rings[i].clone(),
                holes: (!holes.is_empty()).then_some(holes),
                pattern: pattern.clone(),
                assoc: None,
            }));
        }
    }

    /// The pattern (docs/adr/0186 §9, `pattern.rs`), what is approximated said.
    fn pattern(&mut self, m: Tf, h: &Hatch, b: &mut EntityBase, e: &Parsed) -> HatchPattern {
        let own = b.color.clone().or_else(|| {
            self.lib
                .layer_colors
                .get(&b.layer_id.to_uppercase())
                .cloned()
        });
        let taken = pattern::pattern_of(m, h, own.as_deref());
        for n in &taken.notes {
            self.note("Tarama (HATCH)", n, e.line);
        }
        if taken.colour.is_some() {
            b.color = taken.colour;
        }
        taken.pattern
    }
}

/// A clockwise arc: DXF stores it mirrored (angles negated). The start that
/// meets the previous edge decides when a writer stored it the other way.
fn arc_edge(c: Vec2, r: f64, a0: f64, a1: f64, ccw: bool, prev: Option<Vec2>, out: &mut Vec<Vec2>) {
    let at = |d: f64| {
        let (s, co) = sin_cos_deg(d);
        v(c.x + r * co, c.y + r * s)
    };
    let (start, end, sweep) = if ccw {
        let span = (a1 - a0).rem_euclid(360.0);
        (a0, a1, if span == 0.0 { 360.0 } else { span })
    } else {
        let mirrored = (-a0, -a1);
        let as_is = (a0, a1);
        let (s, e) = match prev {
            Some(p) if dist(at(as_is.0), p) < dist(at(mirrored.0), p) => as_is,
            _ => mirrored,
        };
        let span = (s - e).rem_euclid(360.0);
        (s, e, -(if span == 0.0 { 360.0 } else { span }))
    };
    out.push(at(start));
    arc_points(c, r, rad(start), rad(sweep), Some(at(end)), out);
}

/// A hatch boundary path as points (object coordinates).
struct Boundary {
    pts: Vec<Vec2>,
    /// It had arcs, elliptic arcs or splines (sampled).
    curved: bool,
    /// A spline edge could not be evaluated: its control points stand in for it.
    rough: bool,
}

/// A boundary path's points: a polyline path exactly as the app samples its
/// own bulged rings (the shared core), edge paths with the same step.
fn path_points(path: &Path) -> Boundary {
    match path {
        Path::Poly { pts, bulges } => {
            let pts: Vec<Vec2> = pts.iter().map(|p| v(p[0], p[1])).collect();
            Boundary {
                pts: bulge_path_points(&pts, bulges, true),
                curved: has_arcs(bulges),
                rough: false,
            }
        }
        Path::Edges(edges) => {
            let mut out: Vec<Vec2> = Vec::new();
            let mut curved = false;
            let mut rough = false;
            for edge in edges {
                let prev = out.last().copied();
                match edge {
                    Edge::Line { a, b } => {
                        out.push(v(a[0], a[1]));
                        out.push(v(b[0], b[1]));
                    }
                    Edge::Arc { c, r, a0, a1, ccw } => {
                        curved = true;
                        arc_edge(v(c[0], c[1]), *r, *a0, *a1, *ccw, prev, &mut out);
                    }
                    Edge::Ellipse {
                        c,
                        major,
                        ratio,
                        a0,
                        a1,
                        ccw,
                    } => {
                        curved = true;
                        let (a0, a1) = if *ccw { (*a0, *a1) } else { (-*a0, -*a1) };
                        // Stored angles to parameters of the ellipse.
                        let param = |a: f64| atan2(sin(rad(a)) / ratio.max(1e-12), cos(rad(a)));
                        let (t0, mut t1) = (param(a0), param(a1));
                        if *ccw {
                            if t1 <= t0 {
                                t1 += TAU;
                            }
                        } else if t1 >= t0 {
                            t1 -= TAU;
                        }
                        let (mx, my) = (major[0], major[1]);
                        let (nx, ny) = (-my * ratio, mx * ratio);
                        let n = arc_steps(t1 - t0);
                        for i in 0..=n {
                            let t = t0 + (t1 - t0) * i as f64 / n as f64;
                            out.push(v(
                                c[0] + mx * cos(t) + nx * sin(t),
                                c[1] + my * cos(t) + ny * sin(t),
                            ));
                        }
                    }
                    Edge::Spline {
                        degree,
                        knots,
                        ctrl,
                        weights,
                    } => {
                        curved = true;
                        let cps: Vec<Vec2> = ctrl.iter().map(|p| v(p[0], p[1])).collect();
                        let w = (weights.len() == cps.len()).then_some(weights.as_slice());
                        if let Some(pts) = nurbs::sample(*degree, knots, &cps, w, SAMPLE_TOLERANCE)
                        {
                            out.extend(pts);
                        } else {
                            rough = true;
                            out.extend(cps);
                        }
                    }
                }
            }
            Boundary {
                pts: out,
                curved,
                rough,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clockwise_arc_edge_is_read_mirrored() {
        // A clockwise quarter from 90° to 0° is stored as 270° → 360°.
        let mut out = Vec::new();
        arc_edge(v(0.0, 0.0), 1.0, 270.0, 360.0, false, None, &mut out);
        assert!(
            dist(out[0], v(0.0, 1.0)) < 1e-12
                && dist(*out.last().expect("end"), v(1.0, 0.0)) < 1e-12,
            "{out:?}"
        );
        // Counter-clockwise: as stored.
        let mut out = Vec::new();
        arc_edge(v(0.0, 0.0), 2.0, 0.0, 90.0, true, None, &mut out);
        assert!(
            dist(out[0], v(2.0, 0.0)) < 1e-12
                && dist(*out.last().expect("end"), v(0.0, 2.0)) < 1e-12
        );
        assert_eq!(out.len(), 19);
    }

    #[test]
    fn boundary_paths_say_when_they_were_sampled_or_stood_in_for() {
        // A polyline path: bulges sampled as the app samples its own rings; straight ones are not curved.
        let b = path_points(&Path::Poly {
            pts: vec![[0.0, 0.0], [2.0, 0.0]],
            bulges: vec![1.0, 1.0],
        });
        assert_eq!((b.pts.len(), b.curved, b.rough), (72, true, false));
        let b = path_points(&Path::Poly {
            pts: vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0]],
            bulges: vec![0.0; 3],
        });
        assert_eq!((b.pts.len(), b.curved, b.rough), (3, false, false));
        // A spline edge whose knots do not fit its control points: the control points stand in, and that is said.
        let spline = Edge::Spline {
            degree: 3,
            knots: vec![0.0, 1.0],
            ctrl: vec![[0.0, 0.0], [1.0, 1.0], [2.0, 0.0], [3.0, 1.0]],
            weights: Vec::new(),
        };
        let b = path_points(&Path::Edges(vec![
            Edge::Line {
                a: [3.0, 1.0],
                b: [0.0, 0.0],
            },
            spline,
        ]));
        assert_eq!((b.pts.len(), b.curved, b.rough), (6, true, true));
    }
}
