//! Kurtarma (docs/adr/1013; plan/cad-advanced.md CAD-ADV-18): what can be taken out of a `.kcad` the reader refuses,
//! apart from the reader's strict rules and never by loosening them.
//!
//! The container is read leniently (a wrong checksum, a short or over-long file), the payload as a plain CBOR tree
//! that stops where the bytes stop. What is kept is then decided by the strict reader itself: the drawing is put
//! together again from the parts that pass it, so whatever comes out opens as any file does. The document's keys are
//! tried first without the objects (a key that keeps the drawing from reading goes, named); the objects are then put
//! back half by half, an object that does not read alone being left out with the reader's own words. Nothing is
//! written here: the caller writes the recovered copy beside the source, never over it.

use crate::KcadError;

use crate::cbor::{ARRAY, BYTES, MAP, NINT, SIMPLE, TAG, TEXT, UINT};
use crate::header::{FIXED_HEADER, HASH_SIZE, MAGIC_PREFIX};
use crate::watch::Quiet;
use kentos_contracts::DocumentSnapshotV2;
use sha2::{Digest, Sha256};

/// A CBOR item as the bytes hold it: a scalar's own bytes, a container's items.
#[derive(Clone, Debug)]
enum Item {
    /// A scalar (number, text, bytes, simple value, float) as its encoded bytes.
    Raw(Vec<u8>),
    Array(Vec<Item>),
    Map(Vec<(Item, Item)>),
    Tag(u64, Box<Item>),
}

/// The tree's parser: it reads what is there and stops where the bytes stop or stop making sense, keeping every
/// item it finished. `cut` says whether it stopped early.
struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
    cut: bool,
}

const MAX_DEPTH: usize = 64;

impl<'a> Parser<'a> {
    fn head(&mut self) -> Option<(u8, u64, usize)> {
        let start = self.pos;
        let first = *self.data.get(self.pos)?;
        let (major, info) = (first >> 5, first & 0x1f);
        let n = match info {
            0..=23 => {
                self.pos += 1;
                u64::from(info)
            }
            24..=27 => {
                let len = 1usize << (info - 24);
                let bytes = self.data.get(self.pos + 1..self.pos + 1 + len)?;
                self.pos += 1 + len;
                bytes.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b))
            }
            _ => return None,
        };
        Some((major, n, start))
    }

    /// One item, or none when the bytes end (or go wrong) inside it.
    fn item(&mut self, depth: usize) -> Option<Item> {
        if depth > MAX_DEPTH {
            return None;
        }
        let (major, n, start) = self.head()?;
        match major {
            UINT | NINT | SIMPLE => Some(Item::Raw(self.data[start..self.pos].to_vec())),
            BYTES | TEXT => {
                let end = self.pos.checked_add(usize::try_from(n).ok()?)?;
                self.data.get(self.pos..end)?;
                self.pos = end;
                Some(Item::Raw(self.data[start..end].to_vec()))
            }
            ARRAY => {
                let mut items = Vec::new();
                for _ in 0..n {
                    match self.item(depth + 1) {
                        // A part cut inside is kept as far as it goes (the reader then says what it lacks); nothing
                        // after the cut is read, at any level.
                        Some(i) => {
                            items.push(i);
                            if self.cut {
                                break;
                            }
                        }
                        None => {
                            self.cut = true;
                            break;
                        }
                    }
                }
                Some(Item::Array(items))
            }
            MAP => {
                let mut pairs = Vec::new();
                for _ in 0..n {
                    let Some(k) = self.item(depth + 1) else {
                        self.cut = true;
                        break;
                    };
                    if self.cut {
                        break;
                    }
                    let Some(v) = self.item(depth + 1) else {
                        self.cut = true;
                        break;
                    };
                    pairs.push((k, v));
                    if self.cut {
                        break;
                    }
                }
                Some(Item::Map(pairs))
            }
            TAG => Some(Item::Tag(n, Box::new(self.item(depth + 1)?))),
            _ => None,
        }
    }
}

fn head(out: &mut Vec<u8>, major: u8, n: u64) {
    let m = major << 5;
    match n {
        0..=23 => out.push(m | n as u8),
        24..=0xff => out.extend_from_slice(&[m | 24, n as u8]),
        0x100..=0xffff => {
            out.push(m | 25);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(m | 26);
            out.extend_from_slice(&(n as u32).to_be_bytes());
        }
        _ => {
            out.push(m | 27);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }
}

fn write(item: &Item, out: &mut Vec<u8>) {
    match item {
        Item::Raw(bytes) => out.extend_from_slice(bytes),
        Item::Array(items) => {
            head(out, ARRAY, items.len() as u64);
            for i in items {
                write(i, out);
            }
        }
        Item::Map(pairs) => {
            head(out, MAP, pairs.len() as u64);
            for (k, v) in pairs {
                write(k, out);
                write(v, out);
            }
        }
        Item::Tag(t, inner) => {
            head(out, TAG, *t);
            write(inner, out);
        }
    }
}

/// A text item's text.
fn text(item: &Item) -> Option<String> {
    let Item::Raw(bytes) = item else { return None };
    let mut p = Parser { data: bytes, pos: 0, cut: false };
    let (major, n, _) = p.head()?;
    (major == TEXT).then(|| String::from_utf8_lossy(&bytes[p.pos..p.pos + n as usize]).into_owned())
}

/// One part the recovery left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lost {
    /// `document.<key>` for a part of the drawing, `entities[i]` for an object (its place in the file).
    pub part: String,
    /// The object's persistent id when it could be read (32 hex digits).
    pub uid: Option<String>,
    /// Why: the strict reader's words.
    pub reason: String,
}

/// What the recovery made of a file.
#[derive(Clone, Debug)]
pub struct Salvaged {
    /// The drawing as recovered: every part of it passed the strict reader.
    pub document: DocumentSnapshotV2,
    /// What was wrong with the container (a checksum, a cut, bytes after the end), in words.
    pub container: Vec<String>,
    /// The document's keys kept, in the file's order.
    pub kept: Vec<String>,
    /// The objects the file held (as far as it could be read) and how many came back.
    pub objects_read: usize,
    pub objects_kept: usize,
    /// Each part left out.
    pub lost: Vec<Lost>,
    /// Whether the file was whole: the strict reader read it as it is.
    pub whole: bool,
}

fn fail(what: &str) -> KcadError {
    KcadError::new(crate::error::Code::Malformed, what)
}

/// The reader's refusal as one line: its code and its words.
fn said(e: &KcadError) -> String {
    format!("{}: {}", e.code.as_str(), e.message)
}

/// The whole payload of a root map whose document is `document`.
fn payload(root: &[(Item, Item)], document: &[(Item, Item)]) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, MAP, root.len() as u64);
    for (k, v) in root {
        write(k, &mut out);
        if text(k).as_deref() == Some("document") {
            write(&Item::Map(document.to_vec()), &mut out);
        } else {
            write(v, &mut out);
        }
    }
    out
}

fn strict(root: &[(Item, Item)], document: &[(Item, Item)]) -> Result<DocumentSnapshotV2, KcadError> {
    crate::decode::payload(&payload(root, document), &mut Quiet)
}

/// The object's id as hex: an object is `{kind: {…, uid: 16 bytes, …}}`.
fn uid_of(object: &Item) -> Option<String> {
    let Item::Map(outer) = object else { return None };
    let (_, inner) = outer.first()?;
    let Item::Map(pairs) = inner else { return None };
    let (_, v) = pairs.iter().find(|(k, _)| text(k).as_deref() == Some("uid"))?;
    let Item::Raw(bytes) = v else { return None };
    let mut p = Parser { data: bytes, pos: 0, cut: false };
    let (major, n, _) = p.head()?;
    (major == BYTES && n == 16).then(|| bytes[p.pos..p.pos + 16].iter().map(|b| format!("{b:02x}")).collect())
}

/// Recovers what it can of `data`, a `.kcad` the reader may refuse.
pub fn salvage(data: &[u8]) -> Result<Salvaged, KcadError> {
    if let Ok(document) = crate::decode(data) {
        let kept = Vec::new();
        let n = document.entities.len();
        return Ok(Salvaged { document, container: Vec::new(), kept, objects_read: n, objects_kept: n, lost: Vec::new(), whole: true });
    }
    if data.len() < FIXED_HEADER || data[..5] != MAGIC_PREFIX {
        return Err(fail("Dosya bir KCAD değil ya da başlığı okunamıyor; kurtarılacak bir şey yok."));
    }
    let mut container = Vec::new();
    let header_len = usize::from(u16::from_le_bytes([data[12], data[13]]));
    let codec = data[15];
    if codec != crate::header::CODEC_NONE {
        return Err(fail("Dosya sıkıştırılmış yazılmış; sıkıştırılmış yük kurtarılmaz."));
    }
    let declared = u64::from_le_bytes(data[16..24].try_into().expect("eight bytes"));
    if header_len < FIXED_HEADER || header_len > data.len() {
        return Err(fail("Başlığın boyu dosyadan uzun; yükün nerede başladığı bilinmiyor."));
    }
    let available = data.len() - header_len;
    let take = usize::try_from(declared).unwrap_or(usize::MAX).min(available);
    if take < usize::try_from(declared).unwrap_or(usize::MAX) {
        container.push(format!(
            "dosya kesik: yükün {} baytı bekleniyordu, {} bayt var; özet de yok",
            declared, available
        ));
    } else {
        let end = header_len + take;
        let rest = data.len() - end;
        if rest < HASH_SIZE {
            container.push("dosyanın sonundaki özet eksik; yük özetsiz okundu".into());
        } else {
            let sum = Sha256::digest(&data[..end]);
            if sum[..] != data[end..end + HASH_SIZE] {
                container.push("dosyanın özeti tutmuyor: baytlar yazıldıktan sonra değişmiş".into());
            }
            if rest > HASH_SIZE {
                container.push(format!("dosyanın sonunda {} fazla bayt var; okunmadı", rest - HASH_SIZE));
            }
        }
    }
    let body = &data[header_len..header_len + take];
    let mut parser = Parser { data: body, pos: 0, cut: false };
    let Some(Item::Map(root)) = parser.item(0) else {
        return Err(fail("Yükün başı okunamıyor; kurtarılacak bir çizim yok."));
    };
    if parser.cut {
        container.push("yük kesik: kesildiği yerden sonrası okunmadı".into());
    }
    let Some(document) = root.iter().find(|(k, _)| text(k).as_deref() == Some("document")).map(|(_, v)| v.clone())
    else {
        return Err(fail("Yükte çizim (document) yok; kurtarılacak bir şey yok."));
    };
    let Item::Map(mut doc) = document else {
        return Err(fail("Yükün çizimi bir harita değil; kurtarılamaz."));
    };
    let mut lost = Vec::new();
    // The keys the reader needs and the file lost (a cut inside the objects takes `settings` and `activeLayer`, which
    // come after them): an empty drawing's, said. The project's own coordinate system is then not known.
    fill_required(&mut doc, &mut lost)?;
    // The objects aside; an empty list in their place while the rest is tried.
    let at = doc.iter().position(|(k, _)| text(k).as_deref() == Some("entities"));
    let objects: Vec<Item> = match at.map(|i| doc[i].1.clone()) {
        Some(Item::Array(items)) => items,
        _ => Vec::new(),
    };
    let objects_read = objects.len();
    let empty = Item::Array(Vec::new());
    match at {
        Some(i) => doc[i].1 = empty.clone(),
        None => {
            doc.push((Item::Raw(text_item("entities")), empty.clone()));
            doc.sort_by(|a, b| crate::cbor::key_order(&text(&a.0).unwrap_or_default(), &text(&b.0).unwrap_or_default()));
        }
    }
    // The document's keys: drop, last first, any one whose leaving makes the rest read.
    let mut reason = match strict(&root, &doc) {
        Ok(_) => None,
        Err(e) => Some(said(&e)),
    };
    while let Some(why) = reason.clone() {
        let mut dropped = false;
        // A list (block definitions, layers, sheets…): its items that read are kept, one by one, before the key goes.
        for i in (0..doc.len()).rev() {
            let key = text(&doc[i].0).unwrap_or_default();
            if key == "entities" {
                continue;
            }
            let Item::Array(items) = doc[i].1.clone() else { continue };
            if items.is_empty() {
                continue;
            }
            // Only when the list is what keeps the drawing from reading: without it (an empty list is not written),
            // the rest reads.
            let mut without = doc.clone();
            without.remove(i);
            if strict(&root, &without).is_err() {
                continue;
            }
            let (kept, out) = bisect(&root, &doc, i, items, &format!("document.{key}"), |_| None);
            let mut next = doc.clone();
            if kept.is_empty() {
                next.remove(i);
            } else {
                next[i].1 = Item::Array(kept);
            }
            if strict(&root, &next).is_ok() {
                lost.extend(out);
                doc = next;
                reason = None;
                dropped = true;
                break;
            }
        }
        if dropped {
            continue;
        }
        for i in (0..doc.len()).rev() {
            let key = text(&doc[i].0).unwrap_or_default();
            if key == "entities" {
                continue;
            }
            let mut fewer = doc.clone();
            fewer.remove(i);
            if strict(&root, &fewer).is_ok() {
                lost.push(Lost { part: format!("document.{key}"), uid: None, reason: why.clone() });
                doc = fewer;
                reason = None;
                dropped = true;
                break;
            }
        }
        if !dropped {
            return Err(fail(&format!(
                "Çizimin zorunlu bir bölümü okunamıyor; kurtarılamaz. Okuyucunun sözü: {why}"
            )));
        }
    }
    // The objects back, half by half.
    let slot = doc.iter().position(|(k, _)| text(k).as_deref() == Some("entities")).expect("put there");
    let (kept_objects, out) = bisect(&root, &doc, slot, objects, "entities", uid_of);
    lost.extend(out);
    doc[slot].1 = Item::Array(kept_objects.clone());
    let document = strict(&root, &doc)?;
    let kept = doc.iter().filter_map(|(k, _)| text(k)).collect();
    lost.sort_by_key(|l| (!l.part.starts_with("document."), entity_place(&l.part)));
    Ok(Salvaged {
        document,
        container,
        kept,
        objects_read,
        objects_kept: kept_objects.len(),
        lost,
        whole: false,
    })
}

/// The keys a drawing must have (the reader's `required`), and an empty drawing's values for them.
const REQUIRED: [&str; 7] = ["name", "layers", "origin", "styles", "entities", "settings", "activeLayer"];

fn defaults() -> Result<Vec<(Item, Item)>, KcadError> {
    let empty = serde_json::json!({
        "format": "kentos.document", "version": 2, "name": "Kurtarılan çizim",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 1000},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [{"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
                    "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "0", "entities": [], "uids": [], "styles": {"items": [], "categories": []}
    });
    let doc: DocumentSnapshotV2 = serde_json::from_value(empty).map_err(|e| fail(&format!("boş çizim kurulamadı: {e}")))?;
    let bytes = crate::encode(&doc)?;
    let header_len = usize::from(u16::from_le_bytes([bytes[12], bytes[13]]));
    let body = &bytes[header_len..bytes.len() - HASH_SIZE];
    let mut p = Parser { data: body, pos: 0, cut: false };
    let Some(Item::Map(root)) = p.item(0) else { return Err(fail("boş çizim okunamadı")) };
    match root.into_iter().find(|(k, _)| text(k).as_deref() == Some("document")).map(|(_, v)| v) {
        Some(Item::Map(pairs)) => Ok(pairs),
        _ => Err(fail("boş çizimin belgesi yok")),
    }
}

/// The first layer's id of a `layers` item.
fn first_layer(layers: &Item) -> Option<Item> {
    let Item::Array(items) = layers else { return None };
    let Item::Map(pairs) = items.first()? else { return None };
    pairs.iter().find(|(k, _)| text(k).as_deref() == Some("id")).map(|(_, v)| v.clone())
}

fn fill_required(doc: &mut Vec<(Item, Item)>, lost: &mut Vec<Lost>) -> Result<(), KcadError> {
    let has = |doc: &[(Item, Item)], key: &str| doc.iter().any(|(k, _)| text(k).as_deref() == Some(key));
    if REQUIRED.iter().all(|k| has(doc, k)) {
        return Ok(());
    }
    let base = defaults()?;
    for key in REQUIRED {
        if has(doc, key) || key == "entities" {
            continue;
        }
        let value = if key == "activeLayer" {
            doc.iter()
                .find(|(k, _)| text(k).as_deref() == Some("layers"))
                .and_then(|(_, v)| first_layer(v))
        } else {
            None
        };
        let value = match value {
            Some(v) => v,
            None => base.iter().find(|(k, _)| text(k).as_deref() == Some(key)).map(|(_, v)| v.clone()).ok_or_else(|| fail("boş çizimin anahtarı yok"))?,
        };
        let words = match key {
            "settings" => "dosyada kalmadı (kesik ya da bozuk); boş çizimin ayarları kondu: koordinat sistemi SRID 0, birim ve basamaklar varsayılan. Projenin koordinat sistemini yeniden atayın.",
            "activeLayer" => "dosyada kalmadı (kesik ya da bozuk); etkin katman ilk katman yapıldı.",
            "layers" => "dosyada kalmadı (kesik ya da bozuk); tek bir “0” katmanı kondu.",
            _ => "dosyada kalmadı (kesik ya da bozuk); boş çizimin değeri kondu.",
        };
        lost.push(Lost { part: format!("document.{key}"), uid: None, reason: words.into() });
        doc.push((Item::Raw(text_item(key)), value));
    }
    // Back in the profile's key order.
    doc.sort_by(|a, b| crate::cbor::key_order(&text(&a.0).unwrap_or_default(), &text(&b.0).unwrap_or_default()));
    Ok(())
}

/// The items of the list at `doc[slot]` that read, put back half by half in the file's order (a bad item among n
/// costs about log n strict reads); each one that does not read alone is lost as `<part>[i]`, with the reader's words.
fn bisect(
    root: &[(Item, Item)],
    doc: &[(Item, Item)],
    slot: usize,
    items: Vec<Item>,
    part: &str,
    uid: impl Fn(&Item) -> Option<String>,
) -> (Vec<Item>, Vec<Lost>) {
    let mut doc = doc.to_vec();
    let mut kept: Vec<Item> = Vec::new();
    let mut lost = Vec::new();
    let mut pending: Vec<(usize, Vec<Item>)> = vec![(0, items)];
    while let Some((first, batch)) = pending.pop() {
        if batch.is_empty() {
            continue;
        }
        let mut trial = kept.clone();
        trial.extend(batch.iter().cloned());
        doc[slot].1 = Item::Array(trial.clone());
        match strict(root, &doc) {
            Ok(_) => kept = trial,
            Err(e) if batch.len() == 1 => {
                lost.push(Lost { part: format!("{part}[{first}]"), uid: uid(&batch[0]), reason: said(&e) })
            }
            Err(_) => {
                let half = batch.len() / 2;
                let (a, b) = batch.split_at(half);
                // Taken in the file's order: the first half is tried first.
                pending.push((first + half, b.to_vec()));
                pending.push((first, a.to_vec()));
            }
        }
    }
    (kept, lost)
}

fn entity_place(part: &str) -> usize {
    part.strip_prefix("entities[").and_then(|s| s.strip_suffix(']')).and_then(|n| n.parse().ok()).unwrap_or(0)
}

fn text_item(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, TEXT, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
    out
}

