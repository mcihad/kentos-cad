//! A layer's Shapefile written as its objects come (docs/adr/9112 §1): the files [`super::write::write`] makes for
//! the same objects, without the layer standing whole in memory.
//!
//! A Shapefile's head and its columns need every row first: the shape type takes elevations and measures if any row
//! has them, a point set is a multipoint one if any row has several points, a column's width is its longest value. So
//! each object, sorted into its geometry families as the whole writer sorts it, goes to a file of its own family beside
//! the target (`.<name>.kentos-gecici`): its parts and its attributes. What decides the head is gathered on the way:
//! the extent, elevations and measures, the attributes' names. At the end each set's .dbf is worked out and written
//! from that file in two or three passes, its .shp and .shx in one; every file first as `<file>.part`, put in its
//! place once all are written. Letting go removes what was begun; a set there before stays as it was.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use kentos_contracts::{Entity, LayerField};

use super::write::{
    DbfEncoding, Family, NO_M, SHAPEFILE, Shape, Written, content, curved, dbf_head, dbf_record,
    file_name, header, left_kind, shape_type, sort,
};
use crate::blocks::Placing;
use crate::export_geom::geometry;
use crate::gis::Ring;
use crate::report::Report;

const TOO_BIG: &str =
    "Shapefile'ın bir dosyası 2 GB'ı aşıyor; katmanı bölün ya da GeoPackage'a yazın.";

/// The least and greatest of values seen; (0, 0) when none was.
#[derive(Clone, Copy, Debug)]
struct Span(f64, f64);

impl Default for Span {
    fn default() -> Self {
        Span(f64::INFINITY, f64::NEG_INFINITY)
    }
}

impl Span {
    fn take(&mut self, v: f64) {
        self.0 = self.0.min(v);
        self.1 = self.1.max(v);
    }

    fn get(self) -> (f64, f64) {
        if self.0 > self.1 {
            (0.0, 0.0)
        } else {
            (self.0, self.1)
        }
    }
}

/// A family's rows on disk and what they decide.
struct Spool {
    path: PathBuf,
    file: Option<BufWriter<File>>,
    rows: usize,
    keys: BTreeSet<String>,
    z: bool,
    m: bool,
    multi: bool,
    /// Rows a vertex of which has no elevation (written 0 when the set has elevations).
    missing_z: u32,
    x: Span,
    y: Span,
    zs: Span,
    ms: Span,
}

fn io(e: std::io::Error) -> String {
    e.to_string()
}

fn u32s(out: &mut impl Write, n: usize) -> std::io::Result<()> {
    out.write_all(&u32::try_from(n).unwrap_or(u32::MAX).to_le_bytes())
}

fn text(out: &mut impl Write, s: &str) -> std::io::Result<()> {
    u32s(out, s.len())?;
    out.write_all(s.as_bytes())
}

fn heights(out: &mut impl Write, zs: &[Option<f64>]) -> std::io::Result<()> {
    for z in zs {
        out.write_all(&[u8::from(z.is_some())])?;
        out.write_all(&z.unwrap_or(0.0).to_le_bytes())?;
    }
    Ok(())
}

impl Spool {
    fn open(path: PathBuf) -> std::io::Result<Spool> {
        Ok(Spool {
            file: Some(BufWriter::new(File::create(&path)?)),
            path,
            rows: 0,
            keys: BTreeSet::new(),
            z: false,
            m: false,
            multi: false,
            missing_z: 0,
            x: Span::default(),
            y: Span::default(),
            zs: Span::default(),
            ms: Span::default(),
        })
    }

    /// Keeps a row: its shape (none: a null shape) and its attributes; what decides the head is gathered.
    fn put(
        &mut self,
        family: Option<Family>,
        shape: Option<&Shape>,
        attrs: &BTreeMap<String, String>,
    ) -> std::io::Result<()> {
        let Some(out) = self.file.as_mut() else {
            return Ok(());
        };
        out.write_all(&[u8::from(shape.is_some())])?;
        if let Some(s) = shape {
            let has = |zs: &crate::gis::Zs| zs.iter().flatten().any(Option::is_some);
            if s.parts.iter().any(|r| has(&r.zs)) {
                self.z = true;
            }
            if s.parts.iter().any(|r| has(&r.ms)) {
                self.m = true;
            }
            if s.parts
                .iter()
                .any(|r| r.zs.as_ref().is_none_or(|z| z.iter().any(Option::is_none)))
            {
                self.missing_z += 1;
            }
            if family == Some(Family::Point) && s.parts.len() > 1 {
                self.multi = true;
            }
            u32s(out, s.parts.len())?;
            for r in &s.parts {
                u32s(out, r.pts.len())?;
                out.write_all(&[u8::from(r.zs.is_some()) | (u8::from(r.ms.is_some()) << 1)])?;
                for (i, p) in r.pts.iter().enumerate() {
                    out.write_all(&p.x.to_le_bytes())?;
                    out.write_all(&p.y.to_le_bytes())?;
                    let z = r.zs.as_ref().and_then(|z| z.get(i).copied().flatten());
                    let m = r.ms.as_ref().and_then(|m| m.get(i).copied().flatten());
                    self.x.take(p.x);
                    self.y.take(p.y);
                    self.zs.take(z.unwrap_or(0.0));
                    let m = m.unwrap_or(NO_M);
                    if m >= -1e38 {
                        self.ms.take(m);
                    }
                }
                if let Some(zs) = &r.zs {
                    heights(out, zs)?;
                }
                if let Some(ms) = &r.ms {
                    heights(out, ms)?;
                }
            }
        }
        u32s(out, attrs.len())?;
        for (k, v) in attrs {
            text(out, k)?;
            text(out, v)?;
            if !self.keys.contains(k) {
                self.keys.insert(k.clone());
            }
        }
        self.rows += 1;
        Ok(())
    }

    /// Ends the writing: the rows can be read back.
    fn close(&mut self) -> std::io::Result<()> {
        match self.file.take() {
            Some(mut f) => f.flush(),
            None => Ok(()),
        }
    }
}

/// A row as a spool keeps it: its shape (none: a null shape) and its attributes.
type Row = (Option<Shape>, BTreeMap<String, String>);

/// What a walk over the spools hands each row to.
type Each<'e> = dyn FnMut(Option<&Shape>, &BTreeMap<String, String>) -> Result<(), String> + 'e;

/// The rows of a spool read back in their order.
struct Back {
    file: BufReader<File>,
}

fn read_u32(r: &mut impl Read) -> std::io::Result<usize> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b) as usize)
}

fn read_f64(r: &mut impl Read) -> std::io::Result<f64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(f64::from_le_bytes(b))
}

fn read_text(r: &mut impl Read) -> std::io::Result<String> {
    let n = read_u32(r)?;
    let mut b = vec![0u8; n];
    r.read_exact(&mut b)?;
    String::from_utf8(b).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn read_heights(r: &mut impl Read, n: usize) -> std::io::Result<Vec<Option<f64>>> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut flag = [0u8; 1];
        r.read_exact(&mut flag)?;
        let v = read_f64(r)?;
        out.push((flag[0] != 0).then_some(v));
    }
    Ok(out)
}

impl Back {
    fn open(path: &Path) -> std::io::Result<Back> {
        Ok(Back {
            file: BufReader::new(File::open(path)?),
        })
    }

    /// The next row: its shape and attributes; none at the end.
    fn next(&mut self) -> std::io::Result<Option<Row>> {
        let mut flag = [0u8; 1];
        if self.file.read(&mut flag)? == 0 {
            return Ok(None);
        }
        let r = &mut self.file;
        let shape = if flag[0] != 0 {
            let parts = read_u32(r)?;
            let mut s = Shape {
                parts: Vec::with_capacity(parts),
            };
            for _ in 0..parts {
                let n = read_u32(r)?;
                let mut has = [0u8; 1];
                r.read_exact(&mut has)?;
                let mut pts = Vec::with_capacity(n);
                for _ in 0..n {
                    let x = read_f64(r)?;
                    let y = read_f64(r)?;
                    pts.push(kentos_contracts::Vec2 { x, y });
                }
                let zs = if has[0] & 1 != 0 {
                    Some(read_heights(r, n)?)
                } else {
                    None
                };
                let ms = if has[0] & 2 != 0 {
                    Some(read_heights(r, n)?)
                } else {
                    None
                };
                s.parts.push(Ring { pts, zs, ms });
            }
            Some(s)
        } else {
            None
        };
        let n = read_u32(r)?;
        let mut attrs = BTreeMap::new();
        for _ in 0..n {
            let k = read_text(r)?;
            let v = read_text(r)?;
            attrs.insert(k, v);
        }
        Ok(Some((shape, attrs)))
    }
}

/// Each row of the spools, in their order, handed to `each`.
fn walk_spools(spools: &[&Spool], each: &mut Each<'_>) -> Result<(), String> {
    for s in spools {
        let mut back = Back::open(&s.path).map_err(io)?;
        while let Some((shape, attrs)) = back.next().map_err(io)? {
            each(shape.as_ref(), &attrs)?;
        }
    }
    Ok(())
}

/// A file being written beside its place, and where it goes.
struct Part {
    part: PathBuf,
    path: PathBuf,
}

/// One layer's objects written as Shapefiles as they come.
pub struct Stream {
    folder: PathBuf,
    base: String,
    fields: Vec<LayerField>,
    prj: Option<String>,
    encoding: DbfEncoding,
    blocks: Placing,
    families: BTreeMap<Family, Spool>,
    nulls: Option<Spool>,
    parts: Vec<Part>,
    rep: Report,
    out: Written,
    failed: Option<String>,
}

impl Stream {
    /// Begins the layer's sets in `folder`, named after `name` (`_nokta`, `_cizgi`, `_alan` after it when the layer has
    /// several geometry families), its fields as given, `prj` the system's text (none writes no .prj).
    pub fn begin(
        folder: &Path,
        name: &str,
        fields: &[LayerField],
        prj: Option<String>,
        encoding: DbfEncoding,
        blocks: Placing,
    ) -> Stream {
        Stream {
            folder: folder.to_path_buf(),
            base: file_name(name),
            fields: fields.to_vec(),
            prj,
            encoding,
            blocks,
            families: BTreeMap::new(),
            nulls: None,
            parts: Vec::new(),
            rep: Report::default(),
            out: Written::default(),
            failed: None,
        }
    }

    fn spool_path(&self, tag: &str) -> PathBuf {
        self.folder
            .join(format!(".{}{tag}.kentos-gecici", self.base))
    }

    /// Writes an object: its shapes into their families' sets, a record without a geometry as a null shape; one the
    /// format cannot hold is counted and left out. The first failure ends the writing: [`Stream::finish`] says it.
    pub fn put(&mut self, e: &Entity) {
        if self.failed.is_some() {
            return;
        }
        if let Err(why) = self.put_one(e) {
            self.failed = Some(why);
        }
    }

    fn put_one(&mut self, e: &Entity) -> Result<(), String> {
        let attrs = &e.base().attrs;
        if matches!(e, Entity::Record(_)) {
            if self.nulls.is_none() {
                self.nulls = Some(Spool::open(self.spool_path("_kayit")).map_err(io)?);
            }
            if let Some(s) = self.nulls.as_mut() {
                s.put(None, None, attrs).map_err(io)?;
            }
            self.rep.count(e.kind());
            return Ok(());
        }
        let Some(g) = geometry(e, &self.blocks, &mut self.rep, &SHAPEFILE) else {
            *self.out.left.entry(left_kind(e).to_owned()).or_default() += 1;
            return Ok(());
        };
        if curved(e, &self.blocks) {
            self.out.approximated += 1;
        }
        let mut sorted = BTreeMap::new();
        sort(g, &mut sorted);
        for (family, shape) in sorted {
            if shape.parts.is_empty() {
                continue;
            }
            if !self.families.contains_key(&family) {
                let spool = Spool::open(self.spool_path(family.suffix())).map_err(io)?;
                self.families.insert(family, spool);
            }
            if let Some(s) = self.families.get_mut(&family) {
                s.put(Some(family), Some(&shape), attrs).map_err(io)?;
            }
        }
        self.rep.count(e.kind());
        Ok(())
    }

    fn part(&mut self, name: &str, ext: &str) -> Result<File, String> {
        let path = self.folder.join(format!("{name}.{ext}"));
        let part = self.folder.join(format!("{name}.{ext}.part"));
        let file = File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        self.parts.push(Part { part, path });
        Ok(file)
    }

    /// Writes the sets and puts them in place: their files; what could not be written as it was is in what comes back.
    pub fn finish(mut self) -> Result<(Vec<PathBuf>, Written), String> {
        let result = self.write_sets();
        let spools: Vec<PathBuf> = self
            .families
            .values()
            .map(|s| s.path.clone())
            .chain(self.nulls.iter().map(|s| s.path.clone()))
            .collect();
        let finished = result.and_then(|files| {
            for p in &self.parts {
                std::fs::rename(&p.part, &p.path)
                    .map_err(|e| format!("{}: {e}", p.path.display()))?;
            }
            Ok(files)
        });
        for s in spools {
            let _ = std::fs::remove_file(s);
        }
        match finished {
            Ok(files) => Ok((files, std::mem::take(&mut self.out))),
            Err(why) => {
                for p in &self.parts {
                    let _ = std::fs::remove_file(&p.part);
                }
                Err(why)
            }
        }
    }

    /// Lets the writing go: nothing of it stays.
    pub fn abandon(mut self) {
        for s in self.families.values_mut().chain(self.nulls.iter_mut()) {
            let _ = s.close();
            let _ = std::fs::remove_file(&s.path);
        }
        for p in &self.parts {
            let _ = std::fs::remove_file(&p.part);
        }
    }

    fn write_sets(&mut self) -> Result<Vec<PathBuf>, String> {
        if let Some(why) = self.failed.take() {
            return Err(why);
        }
        for s in self.families.values_mut().chain(self.nulls.iter_mut()) {
            s.close().map_err(io)?;
        }
        if self.families.is_empty() && self.nulls.is_none() {
            return Ok(Vec::new());
        }
        let several = self.families.len() > 1;
        let families: Vec<Family> = self.families.keys().copied().collect();
        // The rows without a geometry go with the first set, as the whole writer puts them.
        let mut groups: Vec<Option<Family>> = families.iter().copied().map(Some).collect();
        if groups.is_empty() {
            groups.push(None);
        }
        let mut files = Vec::new();
        let mut zero_z = 0u32;
        for (i, family) in groups.into_iter().enumerate() {
            let name = match family {
                Some(f) if several => format!("{}{}", self.base, f.suffix()),
                _ => self.base.clone(),
            };
            let mut spools: Vec<&Spool> = family
                .and_then(|f| self.families.get(&f))
                .into_iter()
                .collect();
            if i == 0
                && let Some(n) = &self.nulls
            {
                spools.push(n);
            }
            let rows: usize = spools.iter().map(|s| s.rows).sum();
            let keys: BTreeSet<String> =
                spools.iter().flat_map(|s| s.keys.iter().cloned()).collect();
            let enc = self.encoding;
            let mut out = std::mem::take(&mut self.out);
            let cols =
                super::columns::columns(&name, &self.fields, &keys, enc, &mut out, &mut |each| {
                    walk_spools(&spools, &mut |_, attrs| {
                        each(attrs);
                        Ok(())
                    })
                });
            self.out = out;
            let cols = cols?;
            let family_spool = family
                .and_then(|f| self.families.get(&f))
                .map(|s| (s.z, s.m, s.multi, s.missing_z, s.x, s.y, s.zs, s.ms));
            let spool_paths: Vec<PathBuf> = spools.iter().map(|s| s.path.clone()).collect();
            let mut set_files = Vec::new();
            if let (Some(f), Some((z, m, multi, missing_z, x, y, zs, ms))) = (family, family_spool)
            {
                if z {
                    zero_z += missing_z;
                }
                let (t, _) = shape_type(f, multi, z, m);
                let (x0, x1) = x.get();
                let (y0, y1) = y.get();
                let (z0, z1) = if z { zs.get() } else { (0.0, 0.0) };
                let (m0, m1) = if m { ms.get() } else { (0.0, 0.0) };
                let bounds = [x0, y0, x1, y1, z0, z1, m0, m1];
                let shp = self.part(&name, "shp")?;
                let shx = self.part(&name, "shx")?;
                write_shapes(shp, shx, t, z, m, rows, bounds, &spool_paths)?;
                set_files.push(self.folder.join(format!("{name}.shp")));
                set_files.push(self.folder.join(format!("{name}.shx")));
            }
            let dbf = self.part(&name, "dbf")?;
            let mut out = std::mem::take(&mut self.out);
            let written = write_dbf(dbf, &cols, rows, enc, &mut out, &spool_paths);
            self.out = out;
            written?;
            set_files.push(self.folder.join(format!("{name}.dbf")));
            if family.is_some()
                && let Some(prj) = self.prj.clone()
            {
                let mut f = self.part(&name, "prj")?;
                f.write_all(prj.as_bytes()).map_err(io)?;
                set_files.push(self.folder.join(format!("{name}.prj")));
            }
            let mut f = self.part(&name, "cpg")?;
            f.write_all(enc.cpg().as_bytes()).map_err(io)?;
            set_files.push(self.folder.join(format!("{name}.cpg")));
            files.extend(set_files);
        }
        let out = &mut self.out;
        let rep = &mut self.rep;
        rep.note_n(
            "Kotsuz köşe",
            "kotu olan nesnelerle aynı Shapefile'daki kotsuz köşeler 0 kotla yazıldı (Shapefile'da köşenin kotsuz olması yok)",
            0,
            zero_z,
        );
        rep.note_n(
            "Boş metin",
            "boş metin ile değerin yokluğu dBASE'te ayrılamaz: ikisi de boşlukla yazıldı, geri okumada değer yoktur",
            0,
            out.blank_text,
        );
        rep.note_n(
            "Sondaki boşluk",
            "metnin sonundaki boşlukları dBASE okuyucuları siler",
            0,
            out.trailing_spaces,
        );
        rep.note_n(
            "Kısaltılan metin",
            "dBASE metin sütunu en çok 254 bayt alır; uzun metin kesildi",
            0,
            out.cut,
        );
        rep.note_n(
            "Kod sayfasında olmayan harf",
            "Windows-1254'te olmayan harfler ? olarak yazıldı",
            0,
            out.replaced,
        );
        rep.note_n(
            "Sütun",
            "dBASE en çok 255 sütun alır; fazlası yazılmadı",
            0,
            out.columns_left,
        );
        for r in &out.renamed {
            rep.note(
                "Sütun adı",
                &format!("“{}” özniteliği “{}” sütununa yazıldı (dBASE adı en çok 10 bayt, büyük küçük harf ayrılmaz)", r.attribute, r.column),
                0,
            );
        }
        for a in &out.as_text {
            rep.note("Sütun türü", &format!("{}: {}", a.column, a.reason), 0);
        }
        out.report = std::mem::take(rep).export();
        Ok(files)
    }
}

/// The .shp and .shx of the rows the spools hold, the .shp's head written last (its length known then).
#[allow(clippy::too_many_arguments)]
fn write_shapes(
    shp: File,
    shx: File,
    t: i32,
    z: bool,
    m: bool,
    rows: usize,
    bounds: [f64; 8],
    spools: &[PathBuf],
) -> Result<(), String> {
    let mut main = BufWriter::new(shp);
    let mut index = BufWriter::new(shx);
    main.write_all(&[0u8; 100]).map_err(io)?;
    index
        .write_all(&header(t, 100 + 8 * rows, bounds)?)
        .map_err(io)?;
    let mut offset = 100usize;
    let mut number = 0usize;
    for path in spools {
        let mut back = Back::open(path).map_err(io)?;
        while let Some((shape, _)) = back.next().map_err(io)? {
            number += 1;
            let c = content(t, shape.as_ref(), z, m);
            let (Ok(n), Ok(words), Ok(at)) = (
                i32::try_from(number),
                i32::try_from(c.len() / 2),
                i32::try_from(offset / 2),
            ) else {
                return Err(TOO_BIG.into());
            };
            main.write_all(&n.to_be_bytes()).map_err(io)?;
            main.write_all(&words.to_be_bytes()).map_err(io)?;
            main.write_all(&c).map_err(io)?;
            index.write_all(&at.to_be_bytes()).map_err(io)?;
            index.write_all(&words.to_be_bytes()).map_err(io)?;
            offset += 8 + c.len();
        }
    }
    let head = header(t, offset, bounds)?;
    main.seek(SeekFrom::Start(0)).map_err(io)?;
    main.write_all(&head).map_err(io)?;
    main.flush().map_err(io)?;
    index.flush().map_err(io)
}

/// The .dbf of the rows the spools hold.
fn write_dbf(
    dbf: File,
    cols: &[super::write::Column],
    rows: usize,
    enc: DbfEncoding,
    out: &mut Written,
    spools: &[PathBuf],
) -> Result<(), String> {
    let mut file = BufWriter::new(dbf);
    file.write_all(&dbf_head(cols, rows, enc)?).map_err(io)?;
    let mut record = Vec::new();
    for path in spools {
        let mut back = Back::open(path).map_err(io)?;
        while let Some((_, attrs)) = back.next().map_err(io)? {
            record.clear();
            dbf_record(cols, &attrs, enc, out, &mut record);
            file.write_all(&record).map_err(io)?;
        }
    }
    file.write_all(&[0x1A]).map_err(io)?;
    file.flush().map_err(io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shp::write::{Input, Layer, write};

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kentos-shp-stream-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    fn objects() -> Vec<Entity> {
        let text = r#"[
          {"kind":"polygon","id":1,"layerId":"p","attrs":{"ada":"101","alan":"625.50","ad":"Konut alanı","not":"ilk"},
           "pts":[{"x":0,"y":0},{"x":0,"y":10},{"x":10,"y":10},{"x":10,"y":0}]},
          {"kind":"polygon","id":2,"layerId":"p","attrs":{"ada":"9","alan":"12.125","ad":"Park"},
           "pts":[{"x":20,"y":0},{"x":20,"y":5},{"x":25,"y":5}],"zs":[1.5,2.0,null]},
          {"kind":"point","id":3,"layerId":"p","attrs":{"ada":"7","ad":"Direk"},"p":{"x":3,"y":4},"z":12.0},
          {"kind":"line","id":5,"layerId":"p","attrs":{"ad":"Yol çok uzun bir ad"},"a":{"x":0,"y":0},"b":{"x":5,"y":5}}
        ]"#;
        let mut objects = serde_json::from_str::<crate::wire::Objects>(text)
            .expect("objects")
            .0;
        // A record without a geometry (its attributes a refused integer and a name).
        let mut base = objects[2].base().clone();
        base.attrs = [
            ("ada".to_owned(), "x12".to_owned()),
            ("ad".to_owned(), "Kayıt".to_owned()),
        ]
        .into_iter()
        .collect();
        objects.insert(3, Entity::Record(kentos_contracts::RecordEntity { base }));
        objects
    }

    fn fields() -> Vec<LayerField> {
        serde_json::from_str(r#"[{"name":"ada","kind":"integer"},{"name":"alan","kind":"decimal"},{"name":"ad","kind":"text"}]"#)
            .expect("fields")
    }

    /// The same objects streamed give the whole writer's files byte for byte: three sets (points, lines, areas) with
    /// the record's row in the first, elevations, a refused integer turned text, a decimal's width, an extra attribute.
    #[test]
    fn a_streamed_layer_is_the_whole_writers_files() {
        let dir = scratch("ayni");
        let objects = objects();
        let fields = fields();
        let whole = write(&Input {
            layers: vec![Layer {
                id: "p",
                name: "parseller",
                fields: &fields,
                objects: objects.iter().collect(),
            }],
            blocks: &[],
            prj: Some("PROJCS[\"x\"]".into()),
            encoding: DbfEncoding::Windows1254,
        })
        .expect("written whole");
        let mut s = Stream::begin(
            &dir,
            "parseller",
            &fields,
            Some("PROJCS[\"x\"]".into()),
            DbfEncoding::Windows1254,
            Placing::new(&[]),
        );
        for e in &objects {
            s.put(e);
        }
        let (files, out) = s.finish().expect("streamed");
        let mut expected = 0;
        for set in &whole.sets {
            for (ext, bytes) in &set.files {
                let path = dir.join(format!("{}.{ext}", set.name));
                assert!(files.contains(&path), "{path:?} in {files:?}");
                assert_eq!(
                    &std::fs::read(&path).expect("a file"),
                    bytes,
                    "{}.{ext}",
                    set.name
                );
                expected += 1;
            }
        }
        assert_eq!(files.len(), expected);
        assert_eq!(out.report, whole.report);
        let left: Vec<_> = std::fs::read_dir(&dir)
            .expect("the folder")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        assert_eq!(left.len(), expected, "no spool and no part left: {left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Let go, nothing of it stays.
    #[test]
    fn a_stream_let_go_leaves_nothing() {
        let dir = scratch("birak");
        let fields = fields();
        let mut s = Stream::begin(
            &dir,
            "parseller",
            &fields,
            None,
            DbfEncoding::Utf8,
            Placing::new(&[]),
        );
        for e in &objects() {
            s.put(e);
        }
        s.abandon();
        assert_eq!(std::fs::read_dir(&dir).expect("the folder").count(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
