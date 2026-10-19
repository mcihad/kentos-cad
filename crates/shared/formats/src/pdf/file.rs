//! A PDF file's objects (ISO 32000-1 §7.5): the cross-reference table or stream (and the hybrid's), followed back
//! through `Prev`; objects in object streams; a file whose cross-references are broken read again by finding every
//! `n g obj` in it. An encrypted file is refused.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::filter;
use super::object::{Dict, Lexer, Obj, Token, is_white};

#[derive(Clone, Copy, Debug)]
enum Entry {
    /// At this byte.
    At(usize),
    /// The `i`th object of object stream `stream`.
    InStream(u32, u32),
}

/// An object stream's objects with their numbers.
type ObjectStream = Vec<(u32, Obj)>;

/// A PDF file opened.
pub struct PdfFile<'a> {
    bytes: &'a [u8],
    entries: HashMap<u32, Entry>,
    pub trailer: Dict,
    streams: RefCell<HashMap<u32, Rc<ObjectStream>>>,
    /// Every `n g obj` found by reading the whole file, made the first time an entry's place holds another object.
    scanned: RefCell<Option<HashMap<u32, usize>>>,
}

/// The most references a resolution follows.
const MAX_HOPS: usize = 32;

fn find_last(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

fn find_from(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay.get(from..)?.windows(needle.len()).position(|w| w == needle).map(|i| i + from)
}

impl<'a> PdfFile<'a> {
    pub fn open(bytes: &'a [u8]) -> Result<Self, String> {
        if !bytes.starts_with(b"%PDF-") && find_from(&bytes[..bytes.len().min(1024)], b"%PDF-", 0).is_none() {
            return Err("dosya bir PDF değil (%PDF- başlığı yok)".into());
        }
        let mut file = PdfFile {
            bytes,
            entries: HashMap::new(),
            trailer: Dict::new(),
            streams: RefCell::default(),
            scanned: RefCell::default(),
        };
        let read = file.read_xrefs();
        if read.is_err() || !file.trailer.contains_key("Root") || file.entries.is_empty() {
            file.entries.clear();
            file.rebuild();
        }
        if file.trailer.contains_key("Encrypt") {
            return Err("PDF şifreli; şifresi açılmış bir kopyasını verin".into());
        }
        if !file.trailer.contains_key("Root") {
            return Err("PDF'in kataloğu bulunamadı: dosya bozuk".into());
        }
        Ok(file)
    }

    fn read_xrefs(&mut self) -> Result<(), String> {
        let tail_from = self.bytes.len().saturating_sub(2048);
        let at = find_last(&self.bytes[tail_from..], b"startxref").ok_or("startxref yok")? + tail_from;
        let mut lx = Lexer::new(self.bytes, at + 9);
        let Some(Token::Obj(Obj::Num(n))) = lx.token()? else {
            return Err("startxref'ten sonra sayı yok".into());
        };
        let mut next = Some(n as usize);
        let mut seen = Vec::new();
        while let Some(off) = next.take() {
            if seen.contains(&off) || seen.len() > 64 || off >= self.bytes.len() {
                break;
            }
            seen.push(off);
            let trailer = self.read_section(off)?;
            if let Some(Obj::Num(x)) = trailer.get("XRefStm") {
                let _ = self.read_section(*x as usize);
            }
            for (k, v) in &trailer {
                if !matches!(k.as_str(), "Prev" | "XRefStm" | "Type" | "W" | "Index" | "Filter" | "DecodeParms" | "Length") {
                    self.trailer.entry(k.clone()).or_insert_with(|| v.clone());
                }
            }
            next = trailer.get("Prev").and_then(Obj::num).map(|p| p as usize);
        }
        Ok(())
    }

    /// One cross-reference section (a table and its trailer, or a stream); its trailer.
    fn read_section(&mut self, off: usize) -> Result<Dict, String> {
        let mut lx = Lexer::new(self.bytes, off);
        lx.skip_white();
        if self.bytes[lx.pos..].starts_with(b"xref") {
            lx.pos += 4;
            loop {
                let save = lx.pos;
                match lx.token()? {
                    Some(Token::Word(w)) if w == "trailer" => break,
                    Some(Token::Obj(Obj::Num(start))) => {
                        let Some(Token::Obj(Obj::Num(count))) = lx.token()? else {
                            return Err("xref alt bölümünün sayısı yok".into());
                        };
                        for i in 0..count as u32 {
                            let Some(Token::Obj(Obj::Num(o))) = lx.token()? else { return Err("xref satırı".into()) };
                            let _ = lx.token()?;
                            let kind = lx.token()?;
                            let num = start as u32 + i;
                            if matches!(kind, Some(Token::Word(ref w)) if w == "n") && o > 0.0 {
                                self.entries.entry(num).or_insert(Entry::At(o as usize));
                            }
                        }
                    }
                    _ => {
                        let _ = save;
                        return Err("xref tablosu bozuk".into());
                    }
                }
            }
            match lx.object()? {
                Obj::Dict(d) => Ok(d),
                _ => Err("trailer bir sözlük değil".into()),
            }
        } else {
            let (_, obj) = self.indirect_at(off)?;
            let Obj::Stream(d, raw) = obj else {
                return Err("xref akışı bir akış değil".into());
            };
            // Its filters and their parameters are direct (ISO 32000-1 §7.5.8.2); its references (the trailer's Root)
            // are kept as references, followed once the entries are read.
            let data = filter::decode(&d, &raw).map_err(|e| format!("xref akışı: {e:?}"))?;
            let w: Vec<usize> = d
                .get("W")
                .and_then(Obj::array)
                .map(|a| a.iter().map(|x| x.num().unwrap_or(0.0) as usize).collect())
                .unwrap_or_default();
            if w.len() < 3 {
                return Err("xref akışının W'si yok".into());
            }
            let size = d.get("Size").and_then(Obj::num).unwrap_or(0.0) as u32;
            let index: Vec<u32> = d
                .get("Index")
                .and_then(Obj::array)
                .map(|a| a.iter().map(|x| x.num().unwrap_or(0.0) as u32).collect())
                .unwrap_or_else(|| vec![0, size]);
            let width: usize = w.iter().sum();
            let field = |row: &[u8], from: usize, n: usize, default: usize| -> usize {
                if n == 0 {
                    return default;
                }
                row[from..from + n].iter().fold(0usize, |a, &b| a << 8 | usize::from(b))
            };
            let mut rows = data.chunks_exact(width.max(1));
            for pair in index.chunks(2) {
                let (start, count) = (pair[0], pair.get(1).copied().unwrap_or(0));
                for i in 0..count {
                    let Some(row) = rows.next() else { break };
                    let kind = field(row, 0, w[0], 1);
                    let a = field(row, w[0], w[1], 0);
                    let b = field(row, w[0] + w[1], w[2], 0);
                    let num = start + i;
                    match kind {
                        1 if a > 0 => {
                            self.entries.entry(num).or_insert(Entry::At(a));
                        }
                        2 => {
                            self.entries.entry(num).or_insert(Entry::InStream(a as u32, b as u32));
                        }
                        _ => {}
                    }
                }
            }
            Ok(d)
        }
    }

    /// Every `n g obj` in the file (the last of a number wins), and the trailer: the last `trailer` dictionary, else a
    /// catalog found among the objects.
    fn rebuild(&mut self) {
        for (n, at) in self.scan() {
            self.entries.insert(n, Entry::At(at));
        }
        let b = self.bytes;
        if let Some(t) = find_last(b, b"trailer") {
            let mut lx = Lexer::new(b, t + 7);
            if let Ok(Obj::Dict(d)) = lx.object() {
                self.trailer = d;
            }
        }
        if !self.trailer.contains_key("Root") {
            let nums: Vec<u32> = self.entries.keys().copied().collect();
            for n in nums {
                if let Ok(o) = self.get(n)
                    && o.dict().and_then(|d| d.get("Type")).and_then(Obj::name) == Some("Catalog")
                {
                    self.trailer.insert("Root".into(), Obj::Ref(n, 0));
                    break;
                }
            }
        }
    }

    /// Where each `n g obj` stands, reading the whole file (the last of a number wins).
    fn scan(&self) -> HashMap<u32, usize> {
        let mut out = HashMap::new();
        let b = self.bytes;
        let mut i = 0;
        while let Some(at) = find_from(b, b"obj", i) {
            i = at + 3;
            // Back over “g ” and “n ”.
            let mut j = at;
            while j > 0 && is_white(b[j - 1]) {
                j -= 1;
            }
            let g_end = j;
            while j > 0 && b[j - 1].is_ascii_digit() {
                j -= 1;
            }
            if j == g_end {
                continue;
            }
            let g_start = j;
            while j > 0 && is_white(b[j - 1]) {
                j -= 1;
            }
            let n_end = j;
            while j > 0 && b[j - 1].is_ascii_digit() {
                j -= 1;
            }
            if j == n_end || g_start == n_end {
                continue;
            }
            if let Ok(n) = String::from_utf8_lossy(&b[j..n_end]).parse::<u32>() {
                out.insert(n, j);
            }
        }
        out
    }

    /// The indirect object at a byte: its number and the object (a stream with its raw bytes).
    fn indirect_at(&self, off: usize) -> Result<(u32, Obj), String> {
        let mut lx = Lexer::new(self.bytes, off);
        let Some(Token::Obj(Obj::Num(n))) = lx.token()? else { return Err(format!("{off}. baytta nesne yok")) };
        let _ = lx.token()?;
        match lx.token()? {
            Some(Token::Word(w)) if w == "obj" => {}
            _ => return Err(format!("{off}. baytta ‘obj’ yok")),
        }
        let obj = lx.object()?;
        let save = lx.pos;
        if let Obj::Dict(d) = &obj
            && let Ok(Some(Token::Word(w))) = lx.token()
            && w == "stream"
        {
            let mut start = lx.pos;
            if self.bytes.get(start) == Some(&b'\r') {
                start += 1;
            }
            if self.bytes.get(start) == Some(&b'\n') {
                start += 1;
            }
            let declared = match d.get("Length") {
                Some(Obj::Num(l)) => Some(*l as usize),
                Some(Obj::Ref(r, _)) if *r != n as u32 => self.get(*r).ok().and_then(|o| o.num()).map(|l| l as usize),
                _ => None,
            };
            let end = match declared {
                Some(l)
                    if start + l <= self.bytes.len()
                        && find_from(&self.bytes[start + l..(start + l + 32).min(self.bytes.len())], b"endstream", 0)
                            .is_some() =>
                {
                    start + l
                }
                _ => {
                    let e = find_from(self.bytes, b"endstream", start).ok_or("akışın sonu yok")?;
                    let mut e2 = e;
                    if e2 > start && self.bytes[e2 - 1] == b'\n' {
                        e2 -= 1;
                    }
                    if e2 > start && self.bytes[e2 - 1] == b'\r' {
                        e2 -= 1;
                    }
                    e2
                }
            };
            return Ok((n as u32, Obj::Stream(d.clone(), self.bytes[start..end].to_vec())));
        }
        let _ = save;
        Ok((n as u32, obj))
    }

    /// Object `n`, read (a missing one is null).
    pub fn get(&self, n: u32) -> Result<Obj, String> {
        match self.entries.get(&n).copied() {
            None => Ok(Obj::Null),
            Some(Entry::At(off)) => match self.indirect_at(off) {
                Ok((found, obj)) if found == n => Ok(obj),
                // A cross-reference pointing elsewhere (a file edited by hand, a broken writer): the object where the
                // file's own text has it.
                _ => {
                    let at = self.scanned.borrow_mut().get_or_insert_with(|| self.scan()).get(&n).copied();
                    let Some(at) = at.filter(|a| *a != off) else {
                        return Err(format!("{n}. nesne yerinde değil"));
                    };
                    match self.indirect_at(at)? {
                        (found, obj) if found == n => Ok(obj),
                        (found, _) => Err(format!("{n}. nesnenin yerinde {found}. nesne var")),
                    }
                }
            },
            Some(Entry::InStream(stream, i)) => {
                let objects = self.object_stream(stream)?;
                Ok(objects
                    .iter()
                    .find(|(k, _)| *k == n)
                    .or_else(|| objects.get(i as usize))
                    .map(|(_, o)| o.clone())
                    .unwrap_or(Obj::Null))
            }
        }
    }

    fn object_stream(&self, n: u32) -> Result<Rc<ObjectStream>, String> {
        if let Some(s) = self.streams.borrow().get(&n) {
            return Ok(s.clone());
        }
        let Some(Entry::At(off)) = self.entries.get(&n).copied() else {
            return Err(format!("{n}. nesne akışı yok"));
        };
        let (_, Obj::Stream(d, raw)) = self.indirect_at(off)? else {
            return Err(format!("{n}. nesne bir akış değil"));
        };
        let d = self.resolved_dict(&d);
        let data = filter::decode(&d, &raw).map_err(|e| format!("nesne akışı: {e:?}"))?;
        let count = d.get("N").and_then(Obj::num).unwrap_or(0.0) as usize;
        let first = d.get("First").and_then(Obj::num).unwrap_or(0.0) as usize;
        let mut lx = Lexer::new(&data, 0);
        let mut heads = Vec::with_capacity(count);
        for _ in 0..count {
            let (Some(Token::Obj(Obj::Num(k))), Some(Token::Obj(Obj::Num(o)))) = (lx.token()?, lx.token()?) else { break };
            heads.push((k as u32, o as usize));
        }
        let mut out = Vec::with_capacity(heads.len());
        for (k, o) in heads {
            let mut lx = Lexer::new(&data, first + o);
            out.push((k, lx.object().unwrap_or(Obj::Null)));
        }
        let rc = Rc::new(out);
        self.streams.borrow_mut().insert(n, rc.clone());
        Ok(rc)
    }

    /// `o` with a reference followed (through references to references).
    pub fn resolve(&self, o: &Obj) -> Obj {
        let mut cur = o.clone();
        for _ in 0..MAX_HOPS {
            match cur {
                Obj::Ref(n, _) => cur = self.get(n).unwrap_or(Obj::Null),
                other => return other,
            }
        }
        Obj::Null
    }

    /// A dictionary with its values' references followed one level (for a stream's filters and lengths).
    pub fn resolved_dict(&self, d: &Dict) -> Dict {
        d.iter().map(|(k, v)| (k.clone(), self.resolve(v))).collect()
    }

    /// A stream's bytes decoded; none for another object.
    pub fn stream_data(&self, o: &Obj) -> Result<Option<Vec<u8>>, filter::FilterError> {
        match self.resolve(o) {
            Obj::Stream(d, raw) => filter::decode(&self.resolved_dict(&d), &raw).map(Some),
            _ => Ok(None),
        }
    }

    /// A dictionary's value, followed.
    pub fn field(&self, d: &Dict, key: &str) -> Obj {
        d.get(key).map(|v| self.resolve(v)).unwrap_or(Obj::Null)
    }
}
