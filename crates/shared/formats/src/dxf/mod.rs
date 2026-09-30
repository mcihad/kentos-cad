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
//! 3DFACE, a HATCH and the like is dropped. Units are reported, never
//! applied: survey drawings often declare millimetres while holding metres
//! (§23: no silent rescale). KentOS's own extended data (`xdata`) gives back
//! what a KentOS export could not say in DXF.

pub mod aci;
mod dimension;
mod emit;
mod entity;
mod hatch;
mod justify;
mod lexer;
mod strings;
mod writer;
pub mod xdata;

/// A dimension laid out as the drawing shows it (its measured value, prefix
/// and unit): the desktop's DXF export gives each dimension without its own
/// text the value it shows, as the web does with the same function.
pub use dimension::layout as dimension_layout;
pub(crate) use writer::{Definitions, Objects};
pub use writer::{WriteInput, input_from_json, write};

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_contracts::{DxfReadOptions, ImportLayer, ImportResult, LineType};

use crate::num::{parse_int, parse_real};
use crate::text::Encoding;
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
    line_weight: Option<f64>,
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
        1 => "inç",
        2 => "fit",
        4 => "milimetre",
        5 => "santimetre",
        6 => "metre",
        7 => "kilometre",
        _ => "başka bir birim",
    }
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
fn classify_name(name: &str) -> LineType {
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
                    let def = LayerDef {
                        name: name(),
                        color,
                        visible: aci_ >= 0 && flags & 1 == 0,
                        locked: flags & 4 != 0,
                        line_type,
                        line_weight: weight,
                    };
                    let key = def.name.to_uppercase();
                    self.lib.layer_colors.insert(key.clone(), def.color.clone());
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
                }
                "STYLE" => {
                    let h = g(40).and_then(|x| parse_real(x.text())).unwrap_or(0.0);
                    self.lib.style_heights.insert(name().to_uppercase(), h);
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
                    xref: flags & 4 != 0,
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
pub fn read_watched(
    bytes: &[u8],
    opts: &DxfReadOptions,
    watch: &mut dyn Watch,
) -> Result<ImportResult, String> {
    match read_once(bytes, opts, watch, false)? {
        Some(result) => Ok(result),
        // The blocks nest deeper than a drawing's may: read again, every insert opened.
        None => {
            let opened = DxfReadOptions {
                explode_blocks: true,
                ..opts.clone()
            };
            read_once(bytes, &opened, watch, true)?
                .ok_or_else(|| "Bloklar açılarak da okunamadı.".to_string())
        }
    }
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
    let lib = std::mem::take(&mut rd.lib);
    let mut em = Emitter::new(&lib, out, opts.explode_blocks);
    for (kind, reason, line) in block_skips {
        em.out.report.skip(&kind, &reason, line);
    }
    if deep {
        em.out.report.note(
            "Blok",
            &format!("bloklar {} düzeyden derin iç içe; bloklar patlatılarak alındı", kentos_contracts::MAX_BLOCK_DEPTH),
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
    if pending_entities {
        let model = Ctx::model();
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
                Ok(e) => em.emit(&e, &model),
                Err((reason, line)) => em.out.report.skip(&kind, &reason, line),
            }
        }
    }
    em.merge_holes();
    let blocks = if opts.explode_blocks {
        Vec::new()
    } else {
        em.keep_used(defs)
    };
    let mut out = em.out;
    // Layers the objects landed on: the table's (in its order), then any it lacked.
    let mut layers: Vec<ImportLayer> = Vec::new();
    let mut listed: HashSet<String> = HashSet::new();
    for l in &rd.layers {
        if let Some(&count) = out.per_layer.get(&l.name) {
            listed.insert(l.name.clone());
            layers.push(ImportLayer {
                name: l.name.clone(),
                color: l.color.clone(),
                visible: l.visible,
                locked: l.locked,
                line_type: l.line_type,
                line_weight: l.line_weight,
                count,
                kinds: BTreeMap::new(),
                bounds: None,
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
            name: name.clone(),
            color: "ink".to_string(),
            visible: true,
            locked: false,
            line_type: LineType::Continuous,
            line_weight: None,
            count,
            kinds: BTreeMap::new(),
            bounds: None,
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
    if let Some(u) = rd.units {
        out.report.fact("Birim ($INSUNITS)", units_name(u));
        if u != 0 && u != 6 {
            out.report.note("Birim", &format!("dosya birimini {} olarak bildiriyor; koordinatlar ölçeklenmeden alındı (metre sayıldı)", units_name(u)), 0);
        }
    }
    let mut result = ImportResult {
        entities: out.entities,
        layers,
        report: out.report.import(),
        bounds: out.bounds,
        declared_crs: None,
        view: None,
        blocks,
    };
    crate::import::summarise(&mut result);
    Ok(Some(result))
}
