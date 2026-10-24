//! ASCII DXF: the reader (HEADER, TABLES, BLOCKS, ENTITIES; everything else
//! is passed over) and the writer (`writer`). The drawing's code page
//! decides how strings decode; blocks are exploded into their objects with
//! the full insertion transform (nested, arrays); objects in paper space
//! and kinds the model cannot hold are counted and reported, never a reason
//! to stop. Heights are kept where the model has a place for them
//! (docs/adr/0142): a POINT's Z, a LINE's two, a 3D POLYLINE's at each vertex,
//! and a LWPOLYLINE's or 2D POLYLINE's elevation at every vertex (blocks give
//! them their insert's Z too); a Z of 0 is no elevation, as in any 2D
//! drawing, unless KentOS's data says it is one. The Z of a SPLINE, a
//! 3DFACE, a HATCH and the like is dropped. Units are reported and converted
//! according to explicit read options: native imports honour declared units;
//! the low-level `unit: None` mode explicitly keeps raw metres.
//! KentOS's own extended data (`xdata`) gives back
//! what a KentOS export could not say in DXF.

pub mod aci;
mod block_units;
mod dimension;
mod diagnostics;
mod emit;
mod entity;
mod hatch;
mod justify;
mod leaders;
mod lexer;
mod layouts;
mod ordering;
pub mod preflight;
pub mod polyline_policy;
mod spatial;
pub use polyline_policy::write_checked;
mod strings;
mod styles;
mod writer;
pub mod xdata;

/// A dimension laid out as the drawing shows it (its measured value, prefix
/// and unit): the desktop's DXF export gives each dimension without its own
/// text the value it shows, as the web does with the same function.
pub use dimension::layout as dimension_layout;
pub(crate) use writer::{Definitions, Objects};
pub use writer::{WriteInput, input_from_json, write};

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_contracts::{
    BlockUnit, DxfReadOptions, ImportLayer, ImportResult, LineType, UnitDecision, UnitSource,
};

use crate::num::{parse_int, parse_real};
use crate::report::Report;
use crate::text::Encoding;
use crate::units::{self, Scale};
use crate::watch::{Quiet, STOPPED, Watch};
use emit::{Block, Ctx, Emitter, Library, Out};
use entity::P3;
use lexer::{Lexer, Pair};
use strings::Decoder;

/// Objects read at most, unless the caller says otherwise.
const DEFAULT_LIMIT: usize = 1_000_000;
/// Objects visited while blocks are exploded, per object the limit allows:
/// nested inserts multiply, and a block whose content is all left out
/// (paper space, attribute definitions) would otherwise be walked without end.
const VISITS_PER_OBJECT: u64 = 8;

/// The z component of a normalised extrusion (±1 for the common planes).
pub(crate) fn extrusion_z(n: P3) -> f64 {
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if l > 0.0 { n[2] / l } else { 1.0 }
}

/// A layer from the LAYER table.
struct LayerDef {
    name: String,
    color: String,
    visible: bool,
    locked: bool,
    line_type: LineType,
    /// The table's pattern it draws with (docs/adr/0313 §6).
    pattern: Option<String>,
    line_weight: Option<f64>,
    /// Printed: DXF's 290 (docs/adr/0260 §4); absent, printed.
    plot: bool,
    /// Frozen: 70 bit 1 (docs/adr/0311 §1); its eye is the colour's sign alone.
    frozen: bool,
}

/// AutoCAD's name for a DXF version.
fn version_name(v: &str) -> String {
    let name = match v {
        "AC1006" => "R10",
        "AC1009" => "R11/R12",
        "AC1012" => "R13",
        "AC1014" => "R14",
        "AC1015" => "2000",
        "AC1018" => "2004",
        "AC1021" => "2007",
        "AC1024" => "2010",
        "AC1027" => "2013",
        "AC1032" => "2018",
        _ => {
            return if v.is_empty() {
                "belirtilmemiş".to_string()
            } else {
                v.to_string()
            };
        }
    };
    format!("AutoCAD {name} ({v})")
}

/// $INSUNITS as words.
fn units_name(u: i64) -> &'static str {
    match u {
        0 => "birimsiz",
        _ => crate::units::insunits(u).map_or("başka bir birim", |(_, name)| name),
    }
}

/// The unit the file's values are read in, the scale from them to metres
/// and what the report says of it (docs/adr/0257 §1, docs/adr/0165 §2):
///
/// - a unit chosen (`fileUnit`) is the values' unit, whatever the file says;
/// - an explicit known-length reference calibrates model values to metres;
/// - low-level raw mode (`unit` none) reads metres and reports other declarations;
/// - native callers always supply a project unit and honour declared units;
/// - a file that declares none, or a code no one defines, is not scaled: its
///   numbers are in the local project's unit (metres with a system), assumed.
///
/// `records` are the block records' units (BLOCK_RECORD's group 70, docs/adr/0257
/// §2): those other than the file's are listed; an insert's scale holds the
/// ratio. Definitions are normalised in their own units and insert factors
/// compensated, preserving the source drawing while making blocks reusable.
fn decide_units(
    declared: Option<i64>,
    opts: &DxfReadOptions,
    records: &[(String, i64)],
    report: &mut Report,
) -> (Scale, UnitDecision) {
    if let Some(u) = declared {
        report.fact("Birim ($INSUNITS)", units_name(u));
    }
    let known = declared.and_then(|u| units::insunits(u).map(|(s, name)| (u, s, name)));
    let (used, scale, name, source, assumed) = if let Some((reference,factor))=opts.scale_reference.and_then(|r|r.factor().map(|f|(r,f))) {
        let code=kentos_contracts::FileUnit::ALL.into_iter().find(|u|units::insunits(u.code()).is_some_and(|(s,_)|s.factor()==factor)).map_or(0,|u|u.code());
        report.about("Ölçek doğrulaması", &format!("Dosyadaki {} birim = {} m; kaynak uzunlukları ×{factor} ile metreye çevrildi. Blokların kendi fiziksel birimi korunur.",reference.source_length,reference.real_length));
        (code,units::insunits(code).map_or(Scale{mul:factor,div:1.},|(s,_)|s),if code==0{"özel ölçü"}else{units_name(code)},UnitSource::Chosen,false)
    } else { match (opts.file_unit, opts.unit) {
        (Some(chosen), project) => {
            let code = chosen.code();
            let (scale, name) = units::insunits(code).unwrap_or((Scale::ONE, "metre"));
            let said = match known {
                Some((u, _, declared_name)) if u != code => Some(format!(
                    "dosya {declared_name} bildiriyor; değerler seçilen birimde, {name} okundu"
                )),
                Some(_) => None,
                None => Some(format!(
                    "dosya birim bildirmiyor; değerler seçilen birimde, {name} okundu"
                )),
            };
            // A local project in the chosen unit itself has nothing more to say.
            if let Some(said) = said.filter(|_| {
                project.is_none_or(|p| units::insunits_of(p) != code) || known.is_some()
            }) {
                report.about("Birim", &said);
            }
            (code, scale, name, UnitSource::Chosen, false)
        }
        (None, None) => {
            if let Some(u) = declared.filter(|u| *u != 0 && *u != 6) {
                report.about(
                    "Birim",
                    &format!("dosya birimini {} olarak bildiriyor; koordinatlar ölçeklenmeden alındı (metre sayıldı)", units_name(u)),
                );
            }
            (6, Scale::ONE, "metre", UnitSource::Metres, known.is_none())
        }
        (None, Some(project)) => {
            let own = units::name_of(project);
            match known {
                Some((u, scale, name)) => {
                    if name != own {
                        report.about(
                            "Birim",
                            &format!("dosya {name} biriminde; değerler çizimin birimine, {own}ye çevrildi"),
                        );
                    }
                    (u, scale, name, UnitSource::Declared, false)
                }
                None => {
                    let why = match declared {
                        Some(0) | None => "dosya birim bildirmiyor".to_owned(),
                        Some(u) => format!("dosyanın bildirdiği birim ({u}) tanınmıyor"),
                    };
                    report.about(
                        "Birim",
                        &format!("{why}; değerler çizimin biriminde ({own}) sayıldı"),
                    );
                    (
                        units::insunits_of(project),
                        units::from_unit(project),
                        own,
                        UnitSource::Project,
                        true,
                    )
                }
            }
        }
    }
    };
    // AutoCAD's own blocks (model and paper space, a dimension's or a table's) are in the drawing's unit.
    let file = declared.unwrap_or(0);
    let blocks: Vec<BlockUnit> = records
        .iter()
        .filter(|(name, code)| *code != 0 && *code != file && !name.starts_with('*'))
        .map(|(name, code)| BlockUnit {
            name: name.clone(),
            code: *code,
        })
        .collect();
    if !blocks.is_empty() {
        let listed: Vec<String> = blocks
            .iter()
            .map(|b| format!("{} ({})", b.name, units_name(b.code)))
            .collect();
        report.about(
            "Blok birimi",
            &format!(
                "{}: blok kendi birimiyle metreye çevrildi; mevcut yerleştirmelerin boyu ölçek oranı düzeltilerek korundu",
                listed.join(", ")
            ),
        );
    }
    let decision = UnitDecision {
        scale_reference:opts.scale_reference,
        declared: declared.filter(|u| *u != 0),
        used,
        name: name.to_owned(),
        source,
        factor: scale.factor(),
        assumed,
        blocks,
    };
    (scale, decision)
}

/// An LTYPE record's pattern (docs/adr/0313 §6): each 49 an element, its 74 a text (2) or a shape (4) after it, its
/// 75 a shape's number, 340 its STYLE record's handle (`#2A`, its name taken once the table is read), 46 its size,
/// 50 its turn (74 bit 1 absolute), 44 and 45 its shift, 9 a text; lengths in the file's unit. None for ByBlock,
/// ByLayer, Continuous and a type without a gap.
fn ltype_pattern(
    groups: &[lexer::Pair<'_>],
    name: &str,
    dec: Decoder,
) -> Option<kentos_contracts::LineTypeDef> {
    use kentos_contracts::{PatternElement, PatternPlace, PatternShape, PatternText};
    if ["BYBLOCK", "BYLAYER", "CONTINUOUS"].contains(&name.to_uppercase().as_str()) {
        return None;
    }
    let description = groups
        .iter()
        .find(|x| x.code == 3)
        .map(|x| dec.string(x.value))
        .unwrap_or_default();
    struct Part {
        length: f64,
        flags: i64,
        shape: i64,
        style: Option<String>,
        place: PatternPlace,
        text: String,
    }
    let mut parts: Vec<Part> = Vec::new();
    for g in groups {
        let real = || parse_real(g.text()).unwrap_or(0.0);
        match (g.code, parts.last_mut()) {
            (49, _) => parts.push(Part {
                length: real(),
                flags: 0,
                shape: 0,
                style: None,
                place: PatternPlace {
                    scale: 1.0,
                    rotation: 0.0,
                    absolute: false,
                    upright: false,
                    x: 0.0,
                    y: 0.0,
                },
                text: String::new(),
            }),
            (74, Some(p)) => p.flags = parse_int(g.text()).unwrap_or(0),
            (75, Some(p)) => p.shape = parse_int(g.text()).unwrap_or(0),
            (340, Some(p)) => p.style = Some(format!("#{}", g.text().trim().to_uppercase())),
            (46, Some(p)) => p.place.scale = real(),
            (50, Some(p)) => p.place.rotation = real(),
            (44, Some(p)) => p.place.x = real(),
            (45, Some(p)) => p.place.y = real(),
            (9, Some(p)) => p.text = dec.string(g.value),
            _ => {}
        }
    }
    if !parts.iter().any(|p| p.length < 0.0) {
        return None;
    }
    let mut elements = Vec::new();
    for p in parts {
        elements.push(PatternElement {
            length: p.length,
            text: None,
            shape: None,
        });
        let mut place = p.place;
        place.absolute = p.flags & 1 != 0;
        if !(place.scale.is_finite() && place.scale > 0.0) {
            place.scale = 1.0;
        }
        let style = p.style.unwrap_or_default();
        if p.flags & 2 != 0 {
            elements.push(PatternElement {
                length: 0.0,
                text: Some(PatternText {
                    text: p.text,
                    style,
                    place,
                }),
                shape: None,
            });
        } else if p.flags & 4 != 0 {
            elements.push(PatternElement {
                length: 0.0,
                text: None,
                shape: Some(PatternShape {
                    name: p.shape.to_string(),
                    file: style,
                    place,
                }),
            });
        }
    }
    elements.truncate(kentos_contracts::MAX_PATTERN_ELEMENTS);
    Some(kentos_contracts::LineTypeDef {
        name: name.to_owned(),
        description,
        elements,
        unit: kentos_contracts::PatternUnit::Drawing,
        source: Some("DXF".to_owned()),
    })
}

/// A line type from its LTYPE pattern (dash lengths: positive dash, negative gap, zero dot).
fn classify_pattern(elements: &[f64]) -> LineType {
    let dots = elements.contains(&0.0);
    let dashes = elements.iter().any(|&e| e > 0.0);
    match (dashes, dots) {
        (false, false) => LineType::Continuous,
        (true, true) => LineType::Dashdot,
        (false, true) => LineType::Dotted,
        (true, false) => LineType::Dashed,
    }
}

/// A line type from its name, where the LTYPE table does not say.
pub(crate) fn classify_name(name: &str) -> LineType {
    let n = name.to_uppercase();
    if n.contains("DASHDOT")
        || n.contains("CENTER")
        || n.contains("PHANTOM")
        || n.contains("DIVIDE")
    {
        LineType::Dashdot
    } else if n.contains("DOT") {
        LineType::Dotted
    } else if n.contains("DASH") || n.contains("HIDDEN") {
        LineType::Dashed
    } else {
        LineType::Continuous
    }
}

struct Reader<'a> {
    lex: Lexer<'a>,
    dec: Decoder,
    version: String,
    codepage: String,
    units: Option<i64>,
    /// $LTSCALE (docs/adr/0313 §6).
    ltscale: Option<f64>,
    layers: Vec<LayerDef>,
    ltypes: HashMap<String, LineType>,
    lib: Library,
}

impl<'a> Reader<'a> {
    /// Takes groups up to the section's ENDSEC.
    fn skip_section(&mut self) -> Result<(), String> {
        while let Some(p) = self.lex.next()? {
            if p.is(0, "ENDSEC") {
                break;
            }
        }
        Ok(())
    }

    fn header(&mut self) -> Result<(), String> {
        let mut var = String::new();
        while let Some(p) = self.lex.next()? {
            match p.code {
                0 if p.is(0, "ENDSEC") => break,
                9 => var = p.text().to_uppercase(),
                1 if var == "$ACADVER" => self.version = p.text().to_uppercase(),
                3 if var == "$DWGCODEPAGE" => self.codepage = p.text().to_uppercase(),
                70 if var == "$INSUNITS" => self.units = parse_int(p.text()),
                40 if var == "$LTSCALE" => self.ltscale = parse_real(p.text()),
                _ => {}
            }
        }
        Ok(())
    }

    fn tables(&mut self) -> Result<(), String> {
        while let Some(p) = self.lex.next()? {
            if p.code != 0 {
                continue;
            }
            let record = p.text().to_uppercase();
            if record == "ENDSEC" {
                break;
            }
            let groups = self.lex.until_zero()?;
            let g = |code: i32| groups.iter().find(|x| x.code == code);
            let name = || g(2).map(|x| self.dec.string(x.value)).unwrap_or_default();
            match record.as_str() {
                "LAYER" => {
                    let flags = g(70).and_then(|x| parse_int(x.text())).unwrap_or(0);
                    let aci_ = g(62).and_then(|x| parse_int(x.text())).unwrap_or(7);
                    let mut color = match g(420).and_then(|x| parse_int(x.text())) {
                        Some(rgb) => aci::true_color(rgb),
                        None => {
                            aci::color(u8::try_from(aci_.unsigned_abs().clamp(1, 255)).unwrap_or(7))
                        }
                    };
                    // KentOS data names the app's colour (a theme token) while the number is still what it names.
                    if let Some(app) = entity::xdata_of(&groups, self.dec).and_then(|m| m.color)
                        && aci::from_app(&app).0.read_back() == color
                    {
                        color = app;
                    }
                    let ltype = g(6).map(|x| self.dec.string(x.value)).unwrap_or_default();
                    let line_type = self
                        .ltypes
                        .get(&ltype.to_uppercase())
                        .copied()
                        .unwrap_or_else(|| classify_name(&ltype));
                    let weight = g(370)
                        .and_then(|x| parse_int(x.text()))
                        .filter(|&w| w > 0)
                        .map(|w| w as f64 / 100.0);
                    // A pattern of the table (docs/adr/0313 §6), by the name it has there.
                    let pattern = self
                        .lib
                        .patterns
                        .iter()
                        .find(|p| p.name.eq_ignore_ascii_case(&ltype))
                        .map(|p| p.name.clone());
                    let def = LayerDef {
                        name: name(),
                        color,
                        visible: aci_ >= 0,
                        frozen: flags & 1 != 0,
                        locked: flags & 4 != 0,
                        line_type,
                        pattern,
                        line_weight: weight,
                        plot: g(290).and_then(|x| parse_int(x.text())) != Some(0),
                    };
                    let key = def.name.to_uppercase();
                    self.lib.layer_colors.insert(key.clone(), def.color.clone());
                    self.lib.layer_line_types.insert(key.clone(), def.line_type);
                    if let Some(p) = &def.pattern {
                        self.lib.layer_patterns.insert(key.clone(), p.clone());
                    }
                    if let Some(w) = def.line_weight {
                        self.lib.layer_weights.insert(key.clone(), w);
                    }
                    self.lib.layer_names.insert(key, def.name.clone());
                    self.layers.push(def);
                }
                "LTYPE" => {
                    let elements: Vec<f64> = groups
                        .iter()
                        .filter(|x| x.code == 49)
                        .filter_map(|x| parse_real(x.text()))
                        .collect();
                    let n = name();
                    let t = if elements.is_empty() {
                        classify_name(&n)
                    } else {
                        classify_pattern(&elements)
                    };
                    self.ltypes.insert(n.to_uppercase(), t);
                    self.lib.ltypes.insert(n.to_uppercase(), t);
                    // Its pattern as the project keeps it (docs/adr/0313 §6): a type with a gap, not a built-in name.
                    if let Some(p) = ltype_pattern(&groups, &n, self.dec) {
                        self.lib.patterns.push(p);
                    }
                    // By its handle too: a DIMSTYLE names its lines' types so (docs/adr/0205 §6).
                    if let Some(h) = g(5).and_then(|x| u64::from_str_radix(x.text(), 16).ok()) {
                        self.lib.ltype_handles.insert(h, t);
                    }
                }
                "STYLE" => {
                    let h = g(40).and_then(|x| parse_real(x.text())).unwrap_or(0.0);
                    self.lib.style_heights.insert(name().to_uppercase(), h);
                    // The style itself (docs/adr/0183 §7), and its handle for a DIMSTYLE's DIMTXSTY.
                    if let Some(handle) = g(5).and_then(|x| u64::from_str_radix(x.text(), 16).ok())
                    {
                        self.lib.style_handles.insert(handle, name().to_uppercase());
                    }
                    self.lib.text_records.insert(
                        name().to_uppercase(),
                        styles::style_record(&groups, self.dec),
                    );
                }
                // A leader's arrow size and arrow block (docs/adr/0146 §8); the style itself (docs/adr/0183 §7).
                "DIMSTYLE" => {
                    let style = leaders::table_style(&groups);
                    self.lib.dim_styles.insert(name().to_uppercase(), style);
                    self.lib.dim_records.insert(
                        name().to_uppercase(),
                        (name(), styles::dim_record(&groups, self.dec)),
                    );
                }
                "BLOCK_RECORD" => {
                    if let Some(h) = g(5).and_then(|x| u64::from_str_radix(x.text(), 16).ok()) {
                        self.lib.block_records.insert(h, name());
                    }
                    // The block's insertion unit (docs/adr/0257 §2).
                    if let Some(code) = g(70).and_then(|x| parse_int(x.text()))
                        && code != 0
                    {
                        self.lib.block_units.push((name(), code));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Reads the entity whose type was just taken (with the VERTEX or ATTRIB entities that follow it).
    fn entity(
        &mut self,
        name: &str,
        line: u32,
    ) -> Result<Result<entity::Parsed, (String, u32)>, String> {
        let groups = self.lex.until_zero()?;
        let mut after: Vec<(u32, Vec<Pair<'a>>)> = Vec::new();
        let follows = name == "POLYLINE"
            || (name == "INSERT"
                && groups
                    .iter()
                    .any(|g| g.code == 66 && parse_int(g.text()) == Some(1)));
        if follows {
            while let Some(p) = self.lex.peek()? {
                if p.is(0, "VERTEX") || p.is(0, "ATTRIB") {
                    self.lex.next()?;
                    let g = self.lex.until_zero()?;
                    after.push((p.line, g));
                } else if p.is(0, "SEQEND") {
                    self.lex.next()?;
                    self.lex.until_zero()?;
                    break;
                } else {
                    break;
                }
            }
        }
        let fit_data = self.version.as_str() >= "AC1024";
        Ok(
            entity::parse(name, line, &groups, self.dec, after, fit_data)
                .map_err(|u| (u.reason, line)),
        )
    }

    fn blocks(&mut self, skipped: &mut Vec<(String, String, u32)>) -> Result<(), String> {
        while let Some(p) = self.lex.next()? {
            if p.code != 0 {
                continue;
            }
            if p.is(0, "ENDSEC") {
                break;
            }
            if !p.is(0, "BLOCK") {
                continue;
            }
            let head = self.lex.until_zero()?;
            let g = |code: i32| head.iter().find(|x| x.code == code);
            let name = g(2).map(|x| self.dec.string(x.value)).unwrap_or_default();
            let description = g(4).map(|x| self.dec.string(x.value)).unwrap_or_default();
            let coord = |code: i32| g(code).and_then(|x| parse_real(x.text())).unwrap_or(0.0);
            let flags = g(70).and_then(|x| parse_int(x.text())).unwrap_or(0);
            let base = [coord(10), coord(20), coord(30)];
            let mut entities = Vec::new();
            while let Some(q) = self.lex.peek()? {
                if q.code != 0 {
                    self.lex.next()?;
                    continue;
                }
                if q.is(0, "ENDBLK") {
                    self.lex.next()?;
                    self.lex.until_zero()?;
                    break;
                }
                if q.is(0, "ENDSEC") {
                    break;
                }
                self.lex.next()?;
                let kind = q.text().to_uppercase();
                match self.entity(&kind, q.line)? {
                    Ok(e) => entities.push(e),
                    Err((reason, line)) => skipped.push((kind, reason, line)),
                }
            }
            let key = name.to_uppercase();
            if !self.lib.blocks.contains_key(&key) {
                self.lib.order.push(key.clone());
            }
            self.lib.blocks.insert(
                key,
                Block {
                    name,
                    description,
                    base,
                    entities,
                    xref: flags & 4 != 0 && g(1).is_some_and(|p| !p.text().trim().is_empty()),
                    xref_path: g(1).map(|p| self.dec.string(p.value)).unwrap_or_default(),
                    overlay: flags & 8 != 0,
                },
            );
        }
        Ok(())
    }
}

/// The encoding of the drawing's strings: UTF-8 from AutoCAD 2007 on or
/// when an older file's non-ASCII bytes are UTF-8 (some programs write it
/// whatever the version), else the declared code page (Turkish when none).
/// An ASCII-only file reads the same either way; the report then names the
/// declared code page, not UTF-8.
fn encoding(bytes: &[u8], version: &str, codepage: &str, out: &mut Out) -> Encoding {
    if version >= "AC1021" || (!bytes.is_ascii() && std::str::from_utf8(bytes).is_ok()) {
        return Encoding::Utf8;
    }
    match codepage {
        "UTF-8" | "UTF8" => Encoding::Utf8,
        "ANSI_1254" | "" => Encoding::Windows1254,
        "ANSI_1252" => Encoding::Windows1252,
        other => {
            out.report.note("Kod sayfası", &format!("{other} desteklenmiyor; yazılar Windows-1252 olarak okundu, Latin dışı harfler bozuk görünebilir"), 0);
            Encoding::Windows1252
        }
    }
}

/// Objects read between two looks at the watch.
const WATCH_EVERY: u32 = 2048;

/// Reads an ASCII DXF file.
pub fn read(bytes: &[u8], opts: &DxfReadOptions) -> Result<ImportResult, String> {
    read_watched(bytes, opts, &mut Quiet)
}

/// Reads an ASCII DXF file, telling `watch` how far it is (thousandths of
/// the file's bytes) and stopping when it says so (`Err(STOPPED)`).
/// How many AutoCAD constraint groups a DXF holds: the objects named `ACDBASSOC2DCONSTRAINTGROUP` (docs/adr/0338 §4).
fn constraint_groups(bytes: &[u8]) -> usize {
    const NAME: &[u8] = b"ACDBASSOC2DCONSTRAINTGROUP";
    // Only where an `A` stands: a large file is read once, byte by byte, without a window at every place.
    let mut count = 0;
    let mut i = 0;
    while let Some(k) = bytes[i..]
        .iter()
        .position(|b| b.eq_ignore_ascii_case(&b'A'))
    {
        let at = i + k;
        if bytes
            .get(at..at + NAME.len())
            .is_some_and(|w| w.eq_ignore_ascii_case(NAME))
        {
            count += 1;
        }
        i = at + 1;
    }
    count
}

pub fn read_watched(bytes: &[u8], opts: &DxfReadOptions, watch: &mut dyn Watch) -> Result<ImportResult, String> {
    read_checked_watched(bytes, opts, watch).map_err(|e| e.message)
}

pub fn read_checked(bytes: &[u8], opts: &DxfReadOptions) -> Result<ImportResult, preflight::ReadError> {
    read_checked_watched(bytes, opts, &mut Quiet)
}

/// One validation pass; parser errors and cancellation keep their structured code.
pub fn read_checked_watched(bytes: &[u8], opts: &DxfReadOptions, watch: &mut dyn Watch) -> Result<ImportResult, preflight::ReadError> {
    if opts.scale_reference.is_some_and(|r|r.factor().is_none()) || (opts.scale_reference.is_some() && opts.file_unit.is_some()) {
        return Err(preflight::ReadError{code:"invalid_scale",message:"Ölçek doğrulamasında iki uzunluk da sıfırdan büyük ve sonlu olmalı; dosya birimi ile ölçü doğrulamasından yalnız birini seçin.".into()});
    }
    preflight::check(bytes, watch)?;
    let read = (|| {
        let mut result = match read_once(bytes, opts, watch, false)? {
            Some(result) => result,
            None => {
                let opened = DxfReadOptions { explode_blocks: true, ..opts.clone() };
                read_once(bytes, &opened, watch, true)?.ok_or_else(|| "Bloklar açılarak da okunamadı.".to_owned())?
            }
        };
        diagnostics::enrich(&mut result.report, bytes, watch)?;
        Ok(result)
    })();
    let result = read.map_err(|message: String| preflight::ReadError { code: if message == STOPPED { "cancelled" } else { "import_unreadable" }, message })?;
    if !opts.allow_polyline_loss && let Some(error) = polyline_policy::refusal(result.report.notes.iter().chain(&result.report.skipped)) { return Err(error); }
    Ok(result)
}

/// One reading; none when the blocks nest too deep to keep (the caller reads
/// again with them opened). `deep`: this is that reading (said in the report).
fn read_once(
    bytes: &[u8],
    opts: &DxfReadOptions,
    watch: &mut dyn Watch,
    deep: bool,
) -> Result<Option<ImportResult>, String> {
    let total = bytes.len().max(1) as u64;
    let mut ask = |pos: usize| watch.step((pos as u64).min(total) * 950 / total, 1000);
    if bytes.starts_with(b"AutoCAD Binary DXF") {
        return Err("Bu dosya ikili (binary) DXF. KentOS ASCII DXF okur: dosyayı AutoCAD'de DXF olarak, “ASCII” seçeneğiyle kaydedip yeniden deneyin.".into());
    }
    if bytes.len() >= 6 && bytes.starts_with(b"AC1") && bytes[3..6].iter().all(u8::is_ascii_digit) {
        return Err("Bu bir DWG dosyası. DWG kapalı (tescilli) bir biçimdir ve KentOS okuyamaz; dosyayı AutoCAD ya da Netcad'de DXF olarak kaydedip onu açın.".into());
    }
    let limit = if opts.max_entities == 0 {
        DEFAULT_LIMIT
    } else {
        opts.max_entities as usize
    };
    let mut out = Out::new(limit, (limit as u64).saturating_mul(VISITS_PER_OBJECT));
    // Until the header says otherwise: UTF-8 when the bytes are, else Turkish Windows.
    let initial = encoding(bytes, "", "", &mut out);
    let mut rd = Reader {
        lex: Lexer::new(bytes),
        dec: Decoder { enc: initial },
        version: String::new(),
        codepage: String::new(),
        units: None,
        ltscale: None,
        layers: Vec::new(),
        ltypes: HashMap::new(),
        lib: Library::default(),
    };
    let mut saw_section = false;
    let mut block_skips: Vec<(String, String, u32)> = Vec::new();
    // The ENTITIES section streams through the emitter; it needs the library, so it is read last (DXF order).
    let mut pending_entities = false;
    while let Some(p) = rd.lex.next()? {
        if p.code == 999 {
            continue;
        }
        if p.code != 0 {
            if !saw_section {
                return Err(format!(
                    "Satır {}: bu bir DXF dosyası değil (ilk grup “0 SECTION” olmalı).",
                    p.line
                ));
            }
            continue;
        }
        if p.is(0, "EOF") {
            break;
        }
        if !p.is(0, "SECTION") {
            if !saw_section {
                return Err(format!(
                    "Satır {}: bu bir DXF dosyası değil (ilk grup “0 SECTION” olmalı).",
                    p.line
                ));
            }
            continue;
        }
        saw_section = true;
        if !ask(rd.lex.position()) {
            return Err(STOPPED.into());
        }
        let Some(name) = rd.lex.next()? else { break };
        match name.text().to_uppercase().as_str() {
            "HEADER" => {
                rd.header()?;
                rd.dec = Decoder {
                    enc: encoding(bytes, &rd.version, &rd.codepage, &mut out),
                };
            }
            "TABLES" => rd.tables()?,
            "BLOCKS" => rd.blocks(&mut block_skips)?,
            "ENTITIES" => {
                pending_entities = true;
                break;
            }
            _ => rd.skip_section()?,
        }
    }
    if !saw_section {
        return Err("Dosyada DXF bölümü yok; bir ASCII DXF dosyası seçin.".into());
    }
    let order = ordering::mapping(bytes)?;
    let mut lib = std::mem::take(&mut rd.lib);
    for block in lib.blocks.values_mut() { ordering::reorder(&mut block.entities, &order); }
    let mut em = Emitter::new(&lib, out, opts.explode_blocks);
    em.project_font = opts
        .drawing_font
        .unwrap_or(kentos_contracts::DrawingFont::Barlow);
    em.closed_lines = !opts.closed_polylines.is_area();
    for (kind, reason, line) in block_skips {
        em.out.report.skip(&kind, &reason, line);
    }
    // AutoCAD's constraints (docs/adr/0338 §4): their objects are not read; the geometry comes as drawn.
    let groups = constraint_groups(bytes);
    if groups > 0 {
        em.out.report.note(
            "Kısıt",
            &format!(
                "dosyadaki {groups} AutoCAD kısıt grubu (ACDBASSOC2DCONSTRAINTGROUP) okunmadı; geometri kısıtsız alındı"
            ),
            0,
        );
    }
    if deep {
        em.out.report.note(
            "Blok",
            &format!(
                "bloklar {} düzeyden derin iç içe; bloklar patlatılarak alındı",
                kentos_contracts::MAX_BLOCK_DEPTH
            ),
            0,
        );
    }
    // Blocks kept as definitions, read before the drawing's objects place them (docs/adr/0144 §5).
    let mut defs = Vec::new();
    if !opts.explode_blocks {
        match em.define_blocks() {
            Some(d) => defs = d,
            None => return Ok(None),
        }
    }
    let model = Ctx::model();
    let mut buffered = Vec::new();
    let mut paper_entities = Vec::new();
    if pending_entities {
        let mut seen = 0u32;
        while let Some(p) = rd.lex.next()? {
            if p.code != 0 {
                continue;
            }
            if p.is(0, "ENDSEC") {
                break;
            }
            seen += 1;
            if seen.is_multiple_of(WATCH_EVERY) && !ask(rd.lex.position()) {
                return Err(STOPPED.into());
            }
            let kind = p.text().to_uppercase();
            match rd.entity(&kind, p.line)? {
                Ok(e) if e.common.paper => paper_entities.push(e),
                Ok(e) if !order.is_empty() => {
                    if buffered.len() >= limit { return Err("Çizim sırası için nesne sınırı aşıldı; dosyayı bölün.".into()); }
                    buffered.push(e);
                }
                Ok(e) => em.emit(&e, &model),
                Err((reason, line)) => em.out.report.skip(&kind, &reason, line),
            }
        }
    }
    if !order.is_empty() {
        ordering::reorder(&mut buffered, &order);
        for e in &buffered { em.emit(e, &model); }
    }
    em.notes_done();
    em.merge_holes();
    // After the holes: the associative hatches are tied to their objects by place (docs/adr/0333 §2).
    em.tie_hatches();
    // And the architectural objects' references to each other (docs/adr/0347 §2).
    em.tie_arch();
    let blocks = if opts.explode_blocks {
        Vec::new()
    } else {
        em.keep_used(defs)
    };
    let (text_used, dim_used, project_font) = (
        std::mem::take(&mut em.text_used),
        std::mem::take(&mut em.dim_used),
        em.project_font,
    );
    let mut out = em.out;
    // Layers the objects landed on: the table's (in its order), then any it lacked.
    let mut layers: Vec<ImportLayer> = Vec::new();
    let mut listed: HashSet<String> = HashSet::new();
    for l in &rd.layers {
        if let Some(&count) = out.per_layer.get(&l.name) {
            listed.insert(l.name.clone());
            layers.push(ImportLayer {
                line_pattern: l.pattern.clone(),
                name: l.name.clone(),
                color: l.color.clone(),
                visible: l.visible,
                locked: l.locked,
                line_type: l.line_type,
                line_weight: l.line_weight,
                plot: (!l.plot).then_some(false),
                frozen: l.frozen.then_some(true),
                count,
                kinds: BTreeMap::new(),
                bounds: None,
                fields: Vec::new(),
            });
        }
    }
    let mut extra: Vec<(&String, &u32)> = out
        .per_layer
        .iter()
        .filter(|(n, _)| !listed.contains(*n))
        .collect();
    extra.sort();
    for (name, &count) in extra {
        layers.push(ImportLayer {
            line_pattern: None,
            frozen: None,
            plot: None,
            name: name.clone(),
            color: "ink".to_string(),
            visible: true,
            locked: false,
            line_type: LineType::Continuous,
            line_weight: None,
            count,
            kinds: BTreeMap::new(),
            bounds: None,
            fields: Vec::new(),
        });
    }
    if out.truncated > 0 {
        out.report.skip(
            "Nesne sınırı",
            &format!("ilk {limit} nesne alındı; kalanlar alınmadı (dosyayı katmanlara bölün)"),
            0,
        );
    }
    if out.exhausted {
        out.report.skip(
            "Blok (INSERT)",
            &format!("iç içe bloklar {} nesneden fazlasını dolaştırdı; kalan eklemeler açılmadı (blokları AutoCAD'de patlatıp ya da dosyayı bölüp yeniden kaydedin)", out.visit_limit),
            0,
        );
    }
    if !ask(bytes.len()) {
        return Err(STOPPED.into());
    }
    out.report.fact("Sürüm", version_name(&rd.version));
    out.report.fact("Karakter kodlaması", rd.dec.enc.label());
    let (scale, unit) = decide_units(rd.units, opts, &lib.block_units, &mut out.report);
    let mut blocks = blocks;
    // Each block's unit (docs/adr/0315 §4): its record's insertion unit, else the file's when the file says one.
    for b in &mut blocks {
        let code = lib
            .block_units
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&b.name))
            .map(|(_, c)| *c)
            .or((!unit.assumed).then_some(unit.used));
        use kentos_contracts::BlockUnits as B;
        b.units = match code {
            Some(1) => Some(B::In),
            Some(2) => Some(B::Ft),
            Some(4) => Some(B::Mm),
            Some(5) => Some(B::Cm),
            Some(6) => Some(B::M),
            _ => None,
        };
    }
    block_units::normalize(&mut out.entities, &mut blocks, scale, &lib.block_units);
    let block_names=blocks.iter().map(|b|(b.id,b.name.as_str())).collect::<std::collections::HashMap<_,_>>();
    let mut reuse_scales=std::collections::BTreeMap::new();
    for entity in &out.entities {
        if let kentos_contracts::Entity::Insert(i)=entity {
            let sx=i.scale.abs();let sy=i.y_scale.unwrap_or(i.scale).abs();
            if let Some(name)=block_names.get(&i.block) && !name.starts_with('*') && (!(0.5..=2.).contains(&sx) || !(0.5..=2.).contains(&sy)) {
                reuse_scales.entry((*name).to_owned()).or_insert((sx,sy));
            }
        }
    }
    if !reuse_scales.is_empty(){
        let samples=reuse_scales.iter().take(5).map(|(name,(x,y))|format!("{name}: X ×{x:.6}, Y ×{y:.6}")).collect::<Vec<_>>().join("; ");
        out.report.about("Blok ölçeği",&format!("Mevcut blok örnekleri tanımlarından farklı boyutta: {samples}. Yeni blok ölçek 1 ile tanımın fiziksel boyutunda eklenir. Bu fark beklenmiyorsa dosya birimini veya bilinen gerçek uzunluğu denetleyin."));
    }

    if !scale.is_one() && let Some(b) = &mut out.bounds {
        units::bounds(b, scale);
    }
    // The styles the objects follow (docs/adr/0183 §7): their sizes paper mm at the project's scale.
    let plot = opts
        .scale
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or(1000.0);
    let metres = |x: f64| scale.apply(x);
    let mut text_styles = Vec::new();
    for (i, key) in text_used.iter().enumerate() {
        let Some(r) = lib.text_records.get(key) else {
            continue;
        };
        let (style, unmapped) = styles::text_style(
            r,
            format!("dxf-text-{}", i + 1),
            project_font,
            &metres,
            plot,
        );
        if let Some(face) = unmapped {
            out.report.note(
                "Yazı stili (STYLE)",
                &format!(
                    "“{}” stilinin yazı tipi “{face}” KentOS'ta yok; projenin yazı tipiyle alındı",
                    r.name
                ),
                0,
            );
        }
        text_styles.push(style);
    }
    let mut dimension_styles = Vec::new();
    for (i, key) in dim_used.iter().enumerate() {
        let Some((name, vars)) = lib.dim_records.get(key) else {
            continue;
        };
        let font = vars
            .dimtxsty
            .and_then(|h| lib.style_handles.get(&h))
            .and_then(|n| lib.text_records.get(n))
            .filter(|r| !styles::is_standard(&r.name))
            .and_then(|r| {
                r.kentos.as_ref().map(|k| k.font).or_else(|| {
                    styles::family_of(r.family.as_deref().unwrap_or(&r.file))
                        .or_else(|| styles::family_of(&r.file))
                })
            });
        let (style, said) = styles::dimension_style(
            vars,
            format!("dxf-dim-{}", i + 1),
            name,
            &lib.block_records,
            &lib.ltype_handles,
            font,
            &metres,
            plot,
        );
        if let Some(block) = said.arrow {
            out.report.note(
                "Ölçü stili (DIMSTYLE)",
                &format!("“{name}” stilinin ok bloğu “{block}” KentOS'ta yok; dolu ok alındı"),
                0,
            );
        }
        if let Some(f) = said.lfac {
            out.report.note(
                "Ölçü stili (DIMSTYLE)",
                &format!("“{name}” stilinin ölçü çarpanı (DIMLFAC) {f}; KentOS 1, 100 (cm) ve 1000'i (mm) bilir, değer projenin biriminde yazılır"),
                0,
            );
        }
        dimension_styles.push(style);
    }
    // The table's patterns (docs/adr/0313 §6): lengths in the project's unit, a complex element's style its name.
    let per = opts
        .unit
        .unwrap_or(kentos_contracts::DrawingUnit::M)
        .per_metre();
    let to_project = |x: f64| metres(x) * per;
    let line_types: Vec<kentos_contracts::LineTypeDef> = lib
        .patterns
        .iter()
        .map(|t| {
            let mut t = t.clone();
            for e in &mut t.elements {
                e.length = to_project(e.length);
                let place = e
                    .text
                    .as_mut()
                    .map(|x| (&mut x.place, &mut x.style))
                    .or(e.shape.as_mut().map(|s| (&mut s.place, &mut s.file)));
                if let Some((place, style)) = place {
                    place.scale = to_project(place.scale);
                    place.x = to_project(place.x);
                    place.y = to_project(place.y);
                    if let Some(h) = style
                        .strip_prefix('#')
                        .and_then(|h| u64::from_str_radix(h, 16).ok())
                    {
                        *style = lib.style_handles.get(&h).cloned().unwrap_or_default();
                    }
                }
            }
            t
        })
        .collect();
    let mut source_handles = vec![None; out.entities.len()];
    for (handle, index) in &out.handles { if let Some(slot) = source_handles.get_mut(*index) { *slot = Some(format!("{handle:X}")); } }
    let layouts = layouts::read(bytes, &lib, paper_entities, rd.dec, opts, scale, &mut out.report, watch)?;
    let mut result = ImportResult {
        source_handles,
        layouts,
        line_type_scale: rd
            .ltscale
            .filter(|s| s.is_finite() && *s > 0.0 && *s != 1.0),
        line_types,
        entities: out.entities,
        layers,
        report: out.report.import(),
        bounds: out.bounds,
        declared_crs: None,
        view: None,
        blocks,
        text_styles,
        dimension_styles,
        unit: Some(unit),
    };
    crate::import::summarise_cad(&mut result);
    Ok(Some(result))
}
