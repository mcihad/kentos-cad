//! The display list (design §8): a sheet turned into primitives the
//! platforms only paint — rectangles, paths of lines and arcs, positioned
//! text lines, pictures, map frames and clips. Every primitive names the
//! item it comes from (selection and highlighting use it). The core lays
//! out everything but a map's content: grids and their labels, scale bars
//! (ADR 0110's 1-2-5 rule), north arrows, legends, tables, coordinate lists,
//! title blocks, frames, text lines, the master page's items and the
//! `‹ad?›` marks. The same book and inputs give the same list, byte for byte.

mod basic;
mod border;
mod inputs;
mod legend;
pub(crate) mod map;
mod marks;
pub use marks::{DateSource, DeclinationSource, NorthInfo, NorthMissing};
mod table;
mod title;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub use inputs::*;
pub(crate) use map::MapFrame;

use crate::error::Result;
use crate::expr::{self, Eval, Scope};
use crate::kinds::*;
use crate::model::*;
use crate::scene::Scene;
use crate::style::{Stroke, TextStyle};
use crate::text;
use crate::units::*;

// ── Primitives ───────────────────────────────────────────────────────────

/// An arc of an ellipse: its centre, radii, the ellipse's turn, the start angle and the sweep (clockwise on the paper, from the ellipse's own +x axis).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ArcSeg {
    pub center: PointUm,
    pub radius: [Um; 2],
    pub rotation: Mdeg,
    pub start: Mdeg,
    pub sweep: Mdeg,
}

/// A path's step: move, line, arc (a line to its start first, as Canvas does) or close.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Seg {
    M(PointUm),
    L(PointUm),
    A(ArcSeg),
    Z,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RectPrim {
    pub item: ItemId,
    /// Before its turn, about its own centre.
    pub rect: RectUm,
    pub rotation: Mdeg,
    pub radius: Um,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stroke: Option<Stroke>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PathPrim {
    pub item: ItemId,
    pub segments: Vec<Seg>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stroke: Option<Stroke>,
    /// Even-odd filling (holes); otherwise non-zero.
    #[serde(default)]
    pub even_odd: bool,
}

/// A line of text: drawn from `at` (the start of its baseline), turned by `rotation` about that point, left-aligned.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TextPrim {
    pub item: ItemId,
    pub text: String,
    pub at: PointUm,
    /// A `DRAWING_FONTS` id.
    pub font: String,
    pub size: Um,
    pub weight: u16,
    pub italic: bool,
    pub color: String,
    pub rotation: Mdeg,
    /// The width the core measured (without kerning).
    pub width: Um,
    /// A light outline under the letters (labels over a map).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub halo: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ImagePrim {
    pub item: ItemId,
    /// The asset's SHA-256.
    pub asset: String,
    pub rect: RectUm,
    pub rotation: Mdeg,
    pub opacity: u8,
}

/// What a map frame shows: the host paints its content into the clip with this view.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapViewPrim {
    /// None: the map has no place yet (a template's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub center: Option<GroundPoint>,
    pub scale: u32,
    /// The content's turn in the frame (the frame's own turn is the primitive's).
    pub rotation: Mdeg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapPrim {
    pub item: ItemId,
    /// The content's rectangle, before the frame's turn about its own centre.
    pub clip: RectUm,
    pub rotation: Mdeg,
    pub view: MapViewPrim,
    pub layers: MapLayers,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs: Option<String>,
    /// The ground the clip covers: min east, min north, max east, max north.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub extent: Option<[f64; 4]>,
    /// Clip the content to this atlas object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub clip_feature: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ClipPrim {
    pub item: ItemId,
    pub rect: RectUm,
    pub rotation: Mdeg,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct GroupPrim {
    pub item: ItemId,
    /// 0–100.
    pub opacity: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ItemTag {
    pub item: ItemId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Prim {
    Rect(RectPrim),
    Path(PathPrim),
    Text(TextPrim),
    Image(ImagePrim),
    Map(MapPrim),
    PushClip(ClipPrim),
    PopClip(ItemTag),
    PushGroup(GroupPrim),
    PopGroup(ItemTag),
}

impl Prim {
    pub fn item(&self) -> &str {
        match self {
            Prim::Rect(p) => &p.item,
            Prim::Path(p) => &p.item,
            Prim::Text(p) => &p.item,
            Prim::Image(p) => &p.item,
            Prim::Map(p) => &p.item,
            Prim::PushClip(p) => &p.item,
            Prim::PopClip(p) => &p.item,
            Prim::PushGroup(p) => &p.item,
            Prim::PopGroup(p) => &p.item,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct DisplayList {
    pub sheet: SheetId,
    pub size: SizeUm,
    /// The paper's colour.
    pub paper: String,
    pub prims: Vec<Prim>,
    /// The master page's items (drawn first; not selectable).
    pub master_items: Vec<ItemId>,
}

/// Something laid out that did not come out as it should (the preflight's raw material).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub item: ItemId,
    pub code: &'static str,
    pub detail: String,
}

// ── The pen ──────────────────────────────────────────────────────────────

/// Draws an item's primitives in its own unturned frame; the item's turn is applied here.
pub(crate) struct Pen<'a> {
    pub prims: &'a mut Vec<Prim>,
    pub item: ItemId,
    pub center: [f64; 2],
    pub rot: Mdeg,
}

impl Pen<'_> {
    pub fn pt(&self, x: f64, y: f64) -> PointUm {
        let q = rotate([x, y], self.center, self.rot);
        [round_um(q[0]), round_um(q[1])]
    }

    fn turned_rect(&self, r: &RectUm) -> RectUm {
        if norm_mdeg(i64::from(self.rot)) == 0 {
            return *r;
        }
        let c = rotate(r.center(), self.center, self.rot);
        RectUm::new(
            round_um(c[0] - f64::from(r.width) / 2.0),
            round_um(c[1] - f64::from(r.height) / 2.0),
            r.width,
            r.height,
        )
    }

    pub fn rect(&mut self, r: RectUm, fill: Option<&str>, stroke: Option<&Stroke>, radius: Um) {
        if fill.is_none() && stroke.is_none() {
            return;
        }
        let rect = self.turned_rect(&r);
        self.prims.push(Prim::Rect(RectPrim {
            item: self.item.clone(),
            rect,
            rotation: self.rot,
            radius,
            fill: fill.map(str::to_owned),
            stroke: stroke.cloned(),
        }));
    }

    /// A path through `pts` (the item's own coordinates), closed or not.
    pub fn path(
        &mut self,
        pts: &[[f64; 2]],
        closed: bool,
        fill: Option<&str>,
        stroke: Option<&Stroke>,
    ) {
        if pts.len() < 2 || (fill.is_none() && stroke.is_none()) {
            return;
        }
        let mut segs: Vec<Seg> = Vec::with_capacity(pts.len() + 1);
        for (i, p) in pts.iter().enumerate() {
            let q = self.pt(p[0], p[1]);
            segs.push(if i == 0 { Seg::M(q) } else { Seg::L(q) });
        }
        if closed {
            segs.push(Seg::Z);
        }
        self.segments(segs, fill, stroke, false);
    }

    /// Separate lines in one path.
    pub fn lines(&mut self, lines: &[[[f64; 2]; 2]], stroke: &Stroke) {
        if lines.is_empty() {
            return;
        }
        let mut segs = Vec::with_capacity(lines.len() * 2);
        for l in lines {
            segs.push(Seg::M(self.pt(l[0][0], l[0][1])));
            segs.push(Seg::L(self.pt(l[1][0], l[1][1])));
        }
        self.segments(segs, None, Some(stroke), false);
    }

    pub fn segments(
        &mut self,
        segments: Vec<Seg>,
        fill: Option<&str>,
        stroke: Option<&Stroke>,
        even_odd: bool,
    ) {
        self.prims.push(Prim::Path(PathPrim {
            item: self.item.clone(),
            segments,
            fill: fill.map(str::to_owned),
            stroke: stroke.cloned(),
            even_odd,
        }));
    }

    /// An arc step in the item's coordinates.
    pub fn arc(&self, c: [f64; 2], rx: f64, ry: f64, start: Mdeg, sweep: Mdeg) -> Seg {
        Seg::A(ArcSeg {
            center: self.pt(c[0], c[1]),
            radius: [round_um(rx), round_um(ry)],
            rotation: self.rot,
            start,
            sweep,
        })
    }

    pub fn ellipse(
        &mut self,
        c: [f64; 2],
        rx: f64,
        ry: f64,
        fill: Option<&str>,
        stroke: Option<&Stroke>,
    ) {
        if fill.is_none() && stroke.is_none() {
            return;
        }
        let start = self.pt(c[0] + rx, c[1]);
        let segs = vec![Seg::M(start), self.arc(c, rx, ry, 0, FULL_TURN), Seg::Z];
        self.segments(segs, fill, stroke, false);
    }

    /// A line of text whose left end on the baseline is `at`, turned by `extra` on top of the item's turn.
    pub fn text(
        &mut self,
        text: &str,
        at: [f64; 2],
        style: &TextStyle,
        size: Um,
        width: i64,
        extra: Mdeg,
    ) {
        if text.is_empty() {
            return;
        }
        self.prims.push(Prim::Text(TextPrim {
            item: self.item.clone(),
            text: text.to_owned(),
            at: self.pt(at[0], at[1]),
            font: font_of(style),
            size,
            weight: style.weight,
            italic: style.italic,
            color: style.color.clone(),
            rotation: norm_mdeg(i64::from(self.rot) + i64::from(extra)),
            width: sat(width),
            halo: None,
        }));
    }

    /// Every line of a laid-out block.
    pub fn block(&mut self, b: &text::Block, style: &TextStyle) {
        for l in &b.lines {
            self.text(
                &l.text,
                [l.x as f64, l.baseline as f64],
                style,
                b.size,
                l.width,
                0,
            );
        }
    }

    pub fn image(&mut self, asset: &str, r: RectUm, opacity: u8) {
        let rect = self.turned_rect(&r);
        self.prims.push(Prim::Image(ImagePrim {
            item: self.item.clone(),
            asset: asset.to_owned(),
            rect,
            rotation: self.rot,
            opacity,
        }));
    }

    pub fn push_clip(&mut self, r: RectUm) {
        let rect = self.turned_rect(&r);
        self.prims.push(Prim::PushClip(ClipPrim {
            item: self.item.clone(),
            rect,
            rotation: self.rot,
        }));
    }

    pub fn pop_clip(&mut self) {
        self.prims.push(Prim::PopClip(ItemTag {
            item: self.item.clone(),
        }));
    }
}

/// The face's id as the table has it (an unknown one is drawn as Barlow).
fn font_of(style: &TextStyle) -> String {
    if text::has_font(&style.font) {
        style.font.clone()
    } else {
        "barlow".to_owned()
    }
}

// ── The context of one sheet's drawing ───────────────────────────────────

/// Everything an item's drawing reads: the scene, the inputs, the scope of its values, the maps as they are drawn.
pub(crate) struct Ctx<'a> {
    pub book: &'a SheetBook,
    pub scene: &'a Scene<'a>,
    pub inputs: &'a RenderInputs,
    pub scope: Scope,
    pub maps: BTreeMap<ItemId, MapFrame>,
    /// Continuation frames' rows, filled by the tables they continue.
    pub continued: std::cell::RefCell<BTreeMap<ItemId, table::Chunk>>,
}

impl Ctx<'_> {
    /// The scope for an item: a text tied to a map reads that map's scale as `@olcek`.
    pub fn scope_for(&self, item: &Item) -> Scope {
        let mut s = self.scope.clone();
        if let Some(m) = item.kind.map_link().and_then(|id| self.maps.get(id)) {
            s.builtins
                .retain(|(n, _)| n != "OLCEK" && n != "OLCEK_PAYDA");
            s.builtin("olcek", VarValue::Text(format!("1/{}", m.scale)));
            s.builtin("olcek_payda", VarValue::Number(f64::from(m.scale)));
        }
        s
    }

    /// `content` with its `[% … %]` parts written; what could not be is noted.
    pub fn render(&self, item: &Item, content: &str, notes: &mut Vec<Note>) -> String {
        if !expr::has_parts(content) {
            return content.to_owned();
        }
        let r = expr::render(content, &self.scope_for(item));
        for p in r.problems {
            notes.push(eval_note(&item.id, &p.expression, &p.eval));
        }
        r.text
    }
}

pub(crate) fn eval_note(item: &str, expression: &str, e: &Eval) -> Note {
    let (code, detail) = match e {
        Eval::Missing(name) => ("value_missing", name.clone()),
        Eval::Error(msg) => ("expression_error", format!("{expression}: {msg}")),
        _ => ("value_null", expression.to_owned()),
    };
    Note {
        item: item.to_owned(),
        code,
        detail,
    }
}

/// The built-in values of a sheet (design §7).
fn builtins(book: &SheetBook, sheet: &Sheet, inputs: &RenderInputs, scope: &mut Scope) {
    let text = |s: &str| {
        if s.trim().is_empty() {
            VarValue::Null
        } else {
            VarValue::Text(s.to_owned())
        }
    };
    let project = &inputs.project;
    scope.builtin("proje_adi", text(&project.name));
    scope.builtin("kullanici", text(&project.user));
    scope.builtin("tarih", text(&project.date));
    let crs = inputs
        .crs
        .as_ref()
        .map_or(project.crs_name.as_str(), |c| c.name.as_str());
    let paper = match sheet.page.paper {
        Paper::Custom => format!(
            "{} × {} mm",
            mm_text(i64::from(sheet.page.size.width)),
            mm_text(i64::from(sheet.page.size.height))
        ),
        p => p.id().to_uppercase(),
    };
    scope.builtin("kagit", VarValue::Text(paper));
    scope.builtin("koordinat_sistemi", text(crs));
    let (index, count) = match &inputs.page {
        Some(p) => (p.index, p.count),
        None => (
            book.sheet_index(&sheet.id).map_or(1, |i| i as u32 + 1),
            book.sheets.len().max(1) as u32,
        ),
    };
    match &inputs.atlas {
        Some(a) => {
            scope.builtin("pafta_adi", text(&a.name));
            scope.builtin("sayfa", VarValue::Number(f64::from(a.index)));
            scope.builtin("sayfa_sayisi", VarValue::Number(f64::from(a.count)));
            scope.builtin("atlas_sayfa", VarValue::Number(f64::from(a.index)));
            scope.builtin("atlas_sayfa_sayisi", VarValue::Number(f64::from(a.count)));
            scope.builtin("atlas_adi", text(&a.name));
            scope.builtin("atlas_kimlik", text(&a.feature.id));
            scope.atlas = a
                .feature
                .attributes
                .iter()
                .map(|x| (x.name.clone(), x.value.clone()))
                .collect();
            scope.fields = scope.atlas.clone();
        }
        None => {
            scope.builtin("pafta_adi", text(&sheet.name));
            scope.builtin("sayfa", VarValue::Number(f64::from(index)));
            scope.builtin("sayfa_sayisi", VarValue::Number(f64::from(count)));
        }
    }
}

/// The items as drawn: their bindings applied (maps first, so `@olcek` is the bound scale).
fn bound(item: &Item, scope: &Scope, notes: &mut Vec<Note>) -> Item {
    if item.bindings.is_empty() {
        return item.clone();
    }
    let mut it = item.clone();
    for b in &item.bindings {
        match expr::evaluate(&b.expression, scope) {
            Eval::Value(v) => {
                if !crate::bind::apply(&mut it, &b.property, &v) {
                    notes.push(Note {
                        item: item.id.clone(),
                        code: "binding_error",
                        detail: format!("{}: {}", b.property, expr::value_text(&v)),
                    });
                }
            }
            e => notes.push(eval_note(&item.id, &b.expression, &e)),
        }
    }
    it
}

/// The display list of a sheet with the host's inputs.
pub fn display_list(
    book: &SheetBook,
    sheet_id: &str,
    inputs: &RenderInputs,
) -> Result<DisplayList> {
    build(book, sheet_id, inputs).map(|(l, _)| l)
}

/// No silent box (design §6, “Eksik karakter”): a text's character that no drawing face has is
/// written as the “?” it is drawn as (its width was measured so), a control character as a
/// space; every character its face lacks is noted (`glyph_missing`) with what draws it. A
/// note's detail is `chars`, the face's label and the face that draws them (empty: “?”),
/// separated by `\u{1f}`.
fn glyphs(prims: &mut [Prim], notes: &mut Vec<Note>) {
    let mut found: BTreeMap<(ItemId, String, String), String> = BTreeMap::new();
    for p in prims.iter_mut() {
        let Prim::Text(t) = p else {
            continue;
        };
        let Some(f) = text::face(&TextStyle {
            font: t.font.clone(),
            size: t.size,
            weight: t.weight,
            italic: t.italic,
            color: String::new(),
        }) else {
            continue;
        };
        let mut drawn = String::with_capacity(t.text.len());
        for c in t.text.chars() {
            let other = match text::glyph(f, c) {
                text::Glyph::Own(d) => {
                    drawn.push(d);
                    continue;
                }
                text::Glyph::Other(d, g) => {
                    drawn.push(d);
                    g.label()
                }
                text::Glyph::Missing(_) => {
                    drawn.push('?');
                    String::new()
                }
            };
            let chars = found.entry((t.item.clone(), f.label(), other)).or_default();
            if !chars.contains(c) {
                chars.push(c);
            }
        }
        t.text = drawn;
    }
    for ((item, face, other), chars) in found {
        notes.push(Note {
            item,
            code: "glyph_missing",
            detail: format!("{chars}\u{1f}{face}\u{1f}{other}"),
        });
    }
}

/// The paper's colours a map's content is drawn in on a sheet, on the screen and in a PDF alike
/// (design §9a): white paper, black ink, whatever the interface's theme. A style's own colour is
/// drawn as it is; what is the theme's ink on the drawing area (`fg`, `fg-dim`, `ink`), and the
/// drawing's text, take these (the web's `LEGEND_PAPER`, app/sheet/mapFrames.ts `paperPalette`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PaperPalette {
    pub paper: String,
    pub ink: String,
    pub fg: String,
    pub fg_dim: String,
    /// The drawing's text (labels, text objects, dimension values in the default ink).
    pub label: String,
    /// The halo round the drawing's text.
    pub label_halo: String,
}

/// [`PaperPalette`]'s values.
pub mod paper {
    pub const PAPER: &str = "#ffffff";
    pub const INK: &str = "#000000";
    pub const FG: &str = "#111111";
    pub const FG_DIM: &str = "#555555";
    pub const LABEL: &str = FG;
    pub const LABEL_HALO: &str = PAPER;
}

pub fn paper_palette() -> PaperPalette {
    PaperPalette {
        paper: paper::PAPER.into(),
        ink: paper::INK.into(),
        fg: paper::FG.into(),
        fg_dim: paper::FG_DIM.into(),
        label: paper::LABEL.into(),
        label_halo: paper::LABEL_HALO.into(),
    }
}

/// A sheet's export file name (design §9): its `export.file_name` (`[% … %]` text, by default
/// `[% @pafta_adi %]`) written in the sheet's scope, as its texts are written (`‹ad?›` for a
/// value it lacks); a fixed name as it is; the sheet's name when it writes nothing. The host
/// takes out what its file system refuses.
pub fn export_name(book: &SheetBook, sheet_id: &str, inputs: &RenderInputs) -> Result<String> {
    let scene = Scene::new(book, sheet_id)?;
    let sheet = scene.sheet;
    let template = sheet.export.file_name.trim();
    if template.is_empty() {
        return Ok(sheet.name.clone());
    }
    if !expr::has_parts(template) {
        return Ok(template.to_owned());
    }
    let (scope, _, _) = sheet_scope(book, &scene, inputs, &mut Vec::new());
    let written = expr::render(template, &scope).text;
    let written = written.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(if written.is_empty() {
        sheet.name.clone()
    } else {
        written
    })
}

/// What a north arrow of a sheet shows and from what (design §8a; WASM `northInfo`): its map's
/// centre on the ground, the convergence, the declination with its source, the date it was
/// worked out for and where the date came from. An item that is not a north arrow of the sheet
/// (its own or its master page's) is refused.
pub fn north_info(
    book: &SheetBook,
    sheet_id: &str,
    item_id: &str,
    inputs: &RenderInputs,
) -> Result<NorthInfo> {
    let scene = Scene::new(book, sheet_id)?;
    let mut notes = Vec::new();
    let (scope, drawn, maps) = sheet_scope(book, &scene, inputs, &mut notes);
    let ctx = Ctx {
        book,
        scene: &scene,
        inputs,
        scope,
        maps,
        continued: std::cell::RefCell::new(BTreeMap::new()),
    };
    let Some((item, _)) = drawn.iter().find(|(i, _)| i.id == item_id) else {
        return Err(crate::error::SheetError::new(
            "unknown_item",
            format!("Paftada “{item_id}” öğesi yok."),
        ));
    };
    let item = bound(item, &ctx.scope, &mut notes);
    match &item.kind {
        ItemKind::NorthArrow(n) => Ok(marks::north_info(&ctx, n)),
        _ => Err(crate::error::SheetError::new(
            "not_north_arrow",
            format!("“{}” bir kuzey oku değil.", item.name),
        )),
    }
}

/// The display list and what did not come out right.
pub fn build(
    book: &SheetBook,
    sheet_id: &str,
    inputs: &RenderInputs,
) -> Result<(DisplayList, Vec<Note>)> {
    let scene = Scene::new(book, sheet_id)?;
    let sheet = scene.sheet;
    let mut notes = Vec::new();
    let (scope, mut drawn, maps) = sheet_scope(book, &scene, inputs, &mut notes);
    let ctx = Ctx {
        book,
        scene: &scene,
        inputs,
        scope,
        maps,
        continued: std::cell::RefCell::new(BTreeMap::new()),
    };
    // Every item but the maps with its bindings (the maps' bound the scope's scale).
    for (it, _) in drawn.iter_mut() {
        if !matches!(it.kind, ItemKind::Map(_)) {
            *it = bound(it, &ctx.scope, &mut notes);
        }
    }
    build_with(&ctx, &scene, sheet, inputs, drawn, notes)
}

/// The sheet's scope (its variables, the project's, the built-in values and the scale its
/// first map is drawn at), the items with the maps' bindings applied, and the maps as drawn.
fn sheet_scope(
    book: &SheetBook,
    scene: &Scene<'_>,
    inputs: &RenderInputs,
    notes: &mut Vec<Note>,
) -> (Scope, Vec<(Item, bool)>, BTreeMap<ItemId, MapFrame>) {
    let sheet = scene.sheet;
    let mut scope = Scope {
        sheet: sheet.variables.clone(),
        project: book.variables.clone(),
        ..Scope::default()
    };
    builtins(book, sheet, inputs, &mut scope);
    // Maps first: their bound views decide `@olcek` and what scale bars and north arrows read.
    let mut drawn: Vec<(Item, bool)> = Vec::new();
    for (it, master) in scene.all() {
        let b = if matches!(it.kind, ItemKind::Map(_)) {
            bound(it, &scope, notes)
        } else {
            it.clone()
        };
        drawn.push((b, master));
    }
    let mut maps = BTreeMap::new();
    for (it, _) in &drawn {
        if let ItemKind::Map(m) = &it.kind
            && let Some(f) = map::frame_of(it, m, inputs)
        {
            maps.insert(it.id.clone(), f);
        }
    }
    // An overview with no place of its own follows the map it shows.
    for (it, _) in &drawn {
        if let ItemKind::Map(m) = &it.kind
            && let Some(target) = &m.overview_of
            && maps.get(&it.id).is_some_and(|f| f.center.is_none())
            && let Some(c) = maps.get(target).and_then(|t| t.center)
            && let Some(f) = maps.get_mut(&it.id)
        {
            f.center = Some(c);
        }
    }
    let primary = drawn.iter().find_map(|(it, _)| match &it.kind {
        ItemKind::Map(m) if m.overview_of.is_none() => maps.get(&it.id).map(|f| f.scale),
        _ => None,
    });
    match primary {
        Some(s) => {
            scope.builtin("olcek", VarValue::Text(format!("1/{s}")));
            scope.builtin("olcek_payda", VarValue::Number(f64::from(s)));
        }
        None => {
            scope.builtin("olcek", VarValue::Null);
            scope.builtin("olcek_payda", VarValue::Null);
        }
    }
    (scope, drawn, maps)
}

fn build_with(
    ctx: &Ctx<'_>,
    scene: &Scene<'_>,
    sheet: &Sheet,
    inputs: &RenderInputs,
    drawn: Vec<(Item, bool)>,
    mut notes: Vec<Note>,
) -> Result<(DisplayList, Vec<Note>)> {
    // Tables that continue elsewhere lay their rows out first, so a continuation frame below them in the order has its rows.
    for (it, _) in &drawn {
        if let ItemKind::Table(_) | ItemKind::CoordinateList(_) = &it.kind {
            table::prepare(ctx, it, &mut notes);
        }
    }
    let mut prims = Vec::new();
    let export = inputs.mode == RenderMode::Export;
    let list_of = |master: bool| -> &[Item] { if master { &scene.master } else { &sheet.items } };
    for (it, master) in &drawn {
        if it.is_group()
            || !shown(list_of(*master), it, &drawn, *master)
            || (export && !printable(list_of(*master), it))
        {
            continue;
        }
        let opacity = opacity(list_of(*master), it);
        if opacity < 100 {
            prims.push(Prim::PushGroup(GroupPrim {
                item: it.id.clone(),
                opacity,
            }));
        }
        let mut pen = Pen {
            prims: &mut prims,
            item: it.id.clone(),
            center: it.frame.center(),
            rot: it.rotation,
        };
        draw_item(ctx, it, &mut pen, &mut notes);
        if opacity < 100 {
            prims.push(Prim::PopGroup(ItemTag {
                item: it.id.clone(),
            }));
        }
    }
    glyphs(&mut prims, &mut notes);
    let list = DisplayList {
        sheet: sheet.id.clone(),
        size: sheet.page.size,
        paper: sheet
            .page
            .background
            .clone()
            .unwrap_or_else(|| "#ffffff".to_owned()),
        prims,
        master_items: scene.master.iter().map(|i| i.id.clone()).collect(),
    };
    Ok((list, notes))
}

/// Shown: neither it nor a group above it hidden (a bound `hidden` counts).
fn shown(list: &[Item], it: &Item, drawn: &[(Item, bool)], master: bool) -> bool {
    if it.hidden {
        return false;
    }
    let mut up = it.group.clone();
    let mut n = 0;
    while let Some(g) = up {
        let parent = drawn
            .iter()
            .find(|(x, m)| *m == master && x.id == g)
            .map(|(x, _)| x)
            .or_else(|| list.iter().find(|x| x.id == g));
        match parent {
            Some(p) if p.hidden => return false,
            Some(p) => up = p.group.clone(),
            None => break,
        }
        n += 1;
        if n > list.len() {
            break;
        }
    }
    true
}

fn printable(list: &[Item], it: &Item) -> bool {
    let mut cur = Some(it);
    let mut n = 0;
    while let Some(x) = cur {
        if !x.printable {
            return false;
        }
        n += 1;
        if n > list.len() + 1 {
            break;
        }
        cur = x
            .group
            .as_deref()
            .and_then(|g| list.iter().find(|y| y.id == g));
    }
    true
}

/// An item's opacity with its groups' (multiplied).
fn opacity(list: &[Item], it: &Item) -> u8 {
    let mut o = u32::from(it.opacity.min(100));
    let mut up = it
        .group
        .as_deref()
        .and_then(|g| list.iter().find(|y| y.id == g));
    let mut n = 0;
    while let Some(g) = up {
        o = o * u32::from(g.opacity.min(100)) / 100;
        n += 1;
        if n > list.len() {
            break;
        }
        up = g
            .group
            .as_deref()
            .and_then(|x| list.iter().find(|y| y.id == x));
    }
    o as u8
}

fn draw_item(ctx: &Ctx, it: &Item, pen: &mut Pen, notes: &mut Vec<Note>) {
    // The frame's background, then the content, then the frame's line (a map's goes around its content).
    if let Some(f) = &it.fill
        && !matches!(it.kind, ItemKind::Map(_))
    {
        pen.rect(it.frame, Some(f), None, 0);
    }
    match &it.kind {
        ItemKind::Map(m) => map::draw(ctx, it, m, pen, notes),
        ItemKind::Text(t) => basic::text(ctx, it, t, pen, notes),
        ItemKind::ScaleBar(s) => marks::scale_bar(ctx, it, s, pen, notes),
        ItemKind::NorthArrow(n) => marks::north_arrow(ctx, it, n, pen, notes),
        ItemKind::Legend(l) => legend::draw(ctx, it, l, pen, notes),
        ItemKind::Picture(p) => basic::picture(ctx, it, p, pen, notes),
        ItemKind::Shape(s) => basic::shape(it, s, pen),
        ItemKind::Line(l) => basic::line(it, l, pen),
        ItemKind::Table(_) | ItemKind::CoordinateList(_) => table::draw(ctx, it, pen, notes),
        ItemKind::TitleBlock(t) => title::draw(ctx, it, t, pen, notes),
        ItemKind::Border(b) => border::draw(it, b, pen, notes),
        ItemKind::Group(_) => {}
    }
    if let Some(b) = &it.border
        && !matches!(it.kind, ItemKind::Map(_))
    {
        pen.rect(it.frame, None, Some(b), 0);
    }
}
