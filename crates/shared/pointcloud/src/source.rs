//! A cloud opened from its bytes (docs/adr/0207 §1, §2): LAS, LAZ or COPC.
//!
//! [`Opening`] asks for the bytes it needs one run at a time (the header, the
//! VLRs, each EVLR's head and the payloads it reads, LAZ's chunk table, COPC's
//! hierarchy pages) and ends in a [`Cloud`]: what the file is, the system it
//! names, and how its points are read: COPC by nodes, LAZ by chunks, LAS by
//! runs of records. Full reading ([`Cloud::runs`]) gives every point once, in
//! the file's order (COPC's: the nodes in key order).

use laz::LazVlr;
use serde::Serialize;

use crate::chunks::{self, Chunk};
use crate::copc::{self, Hierarchy, Info, Key};
use crate::crs::{self, CloudCrs};
use crate::las::{self, EVLR_HEAD, HEADER_1_4, Header, MAX_VLR_BYTES, Vlr};
use crate::record::Layout;
use crate::{ByteStore, Need, PcError, Result, Step};

/// LAS records read as one run when the file is not compressed.
pub const LAS_RUN: u64 = 65_536;
/// The most bytes the VLRs together may take.
const VLRS_MOST: u64 = 64 * 1024 * 1024;
/// EVLRs whose payload is read: the system's, COPC's hierarchy, extra bytes.
fn wanted(user: &str, record: u16) -> bool {
    (user == "LASF_Projection" && matches!(record, 2112 | 34735..=34737))
        || (user == copc::COPC_USER && record == copc::HIERARCHY_RECORD)
        || (user == "LASF_Spec" && record == 4)
}

/// What a cloud's file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Las,
    Laz,
    Copc,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Las => "las",
            Kind::Laz => "laz",
            Kind::Copc => "copc",
        }
    }
}

/// A run of points to read: the bytes and the points they hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub need: Need,
    pub count: u64,
    /// The first point's index in the file's order.
    pub first: u64,
    /// The COPC node it is, when it is one.
    pub node: Option<Key>,
}

/// A cloud read from its file.
#[derive(Clone, Debug)]
pub struct Cloud {
    pub kind: Kind,
    pub size: u64,
    pub head: Header,
    pub vlrs: Vec<Vlr>,
    pub evlrs: Vec<Vlr>,
    pub layout: Layout,
    pub crs: CloudCrs,
    pub laz: Option<LazVlr>,
    /// LAZ's chunks (not read for COPC).
    pub chunks: Vec<Chunk>,
    pub info: Option<Info>,
    pub hierarchy: Option<Hierarchy>,
}

/// What the window shows of a cloud before it is added (docs/adr/0207 §9).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudInfo {
    pub kind: Kind,
    /// `1.2`, `1.4`.
    pub version: String,
    pub format: u8,
    pub record_len: u16,
    pub extra: u16,
    pub count: u64,
    pub bounds: [f64; 6],
    pub scale: [f64; 3],
    pub offset: [f64; 3],
    pub rgb: bool,
    pub crs: CloudCrs,
    /// Whether an index has to be made (not COPC).
    pub needs_index: bool,
    pub nodes: usize,
    pub chunks: usize,
    pub software: String,
}

impl Cloud {
    /// The window's view of it.
    pub fn info(&self) -> CloudInfo {
        CloudInfo {
            kind: self.kind,
            version: format!("{}.{}", self.head.major, self.head.minor),
            format: self.head.format,
            record_len: self.head.record_len,
            extra: self.layout.extra() as u16,
            count: self.head.count,
            bounds: self.bounds(),
            scale: self.head.scale,
            offset: self.head.offset,
            rgb: self.layout.rgb.is_some(),
            crs: self.crs.clone(),
            needs_index: self.kind != Kind::Copc,
            nodes: self.hierarchy.as_ref().map_or(0, |h| h.nodes.len()),
            chunks: self.chunks.len(),
            software: self.head.software.clone(),
        }
    }

    /// `[x₁, y₁, z₁, x₂, y₂, z₂]` from the header.
    pub fn bounds(&self) -> [f64; 6] {
        let h = &self.head;
        [h.min[0], h.min[1], h.min[2], h.max[0], h.max[1], h.max[2]]
    }

    /// Every point, in runs, in the file's order (COPC: its nodes by key).
    pub fn runs(&self) -> Vec<Run> {
        match self.kind {
            Kind::Copc => {
                let mut first = 0;
                self.hierarchy
                    .as_ref()
                    .map(|h| {
                        h.sorted()
                            .into_iter()
                            .filter(|n| n.count > 0)
                            .map(|n| {
                                let r = Run {
                                    need: Need {
                                        offset: n.offset,
                                        len: n.bytes,
                                    },
                                    count: n.count,
                                    first,
                                    node: Some(n.key),
                                };
                                first += n.count;
                                r
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            Kind::Laz => self
                .chunks
                .iter()
                .map(|c| Run {
                    need: Need {
                        offset: c.offset,
                        len: c.bytes,
                    },
                    count: c.count,
                    first: c.first,
                    node: None,
                })
                .collect(),
            Kind::Las => {
                let len = u64::from(self.head.record_len);
                let mut out = Vec::new();
                let mut first = 0;
                while first < self.head.count {
                    let n = LAS_RUN.min(self.head.count - first);
                    out.push(Run {
                        need: Need {
                            offset: u64::from(self.head.data_offset) + first * len,
                            len: n * len,
                        },
                        count: n,
                        first,
                        node: None,
                    });
                    first += n;
                }
                out
            }
        }
    }

    /// A run's records from its bytes (decompressed when LAZ or COPC), appended to `out`.
    pub fn records(&self, run: &Run, bytes: &[u8], out: &mut Vec<u8>) -> Result<()> {
        match &self.laz {
            Some(vlr) => chunks::decompress(vlr, bytes, run.count as usize, out),
            None => {
                let want = run.count as usize * self.layout.len;
                let b = bytes
                    .get(..want)
                    .ok_or_else(|| PcError::new("LAS'ın noktaları kısa okundu."))?;
                out.extend_from_slice(b);
                Ok(())
            }
        }
    }

    /// A run's records for a picture (`chunks::selection_for`): positions,
    /// returns and what the look `needs` decompressed, the other fields' bytes
    /// undefined.
    pub fn view_records(
        &self,
        run: &Run,
        bytes: &[u8],
        out: &mut Vec<u8>,
        needs: crate::nodes::Needs,
    ) -> Result<()> {
        match &self.laz {
            Some(vlr) => chunks::decompress_selected(
                vlr,
                bytes,
                run.count as usize,
                out,
                chunks::selection_for(needs),
            ),
            None => self.records(run, bytes, out),
        }
    }

    /// A COPC node's run.
    pub fn node_run(&self, key: Key) -> Option<Run> {
        let n = self.hierarchy.as_ref()?.nodes.get(&key)?;
        (n.count > 0).then_some(Run {
            need: Need {
                offset: n.offset,
                len: n.bytes,
            },
            count: n.count,
            first: 0,
            node: Some(key),
        })
    }

    /// A coordinate's world value.
    #[inline]
    pub fn world(&self, axis: usize, v: i32) -> f64 {
        f64::from(v) * self.head.scale[axis] + self.head.offset[axis]
    }
}

/// The header and records read so far.
#[derive(Clone, Debug)]
struct Partial {
    head: Header,
    vlrs: Vec<Vlr>,
    evlrs: Vec<Vlr>,
}

/// Where opening stands.
#[derive(Clone, Debug)]
enum Stage {
    Header,
    Vlrs(Header),
    /// The next EVLR's head at `at`, `left` of them still to look at.
    Evlrs {
        p: Partial,
        at: u64,
        left: u32,
    },
    /// An EVLR's payload of `len` bytes after its head at `at`.
    EvlrBody {
        p: Partial,
        at: u64,
        left: u32,
        user: String,
        record: u16,
        description: String,
        len: u64,
    },
    TableOffset(Opened),
    TableAtEnd(Opened),
    Table(Opened, u64),
    Pages(Opened),
    Done,
}

/// The records read and what they say of the points.
#[derive(Clone, Debug)]
struct Opened {
    head: Header,
    vlrs: Vec<Vlr>,
    evlrs: Vec<Vlr>,
    laz: Option<LazVlr>,
    info: Option<Info>,
    hierarchy: Option<Hierarchy>,
}

/// A cloud being opened from its bytes.
#[derive(Debug)]
pub struct Opening {
    size: u64,
    store: ByteStore,
    stage: Stage,
}

impl Opening {
    /// Opening a file of `size` bytes.
    pub fn new(size: u64) -> Opening {
        Opening {
            size,
            store: ByteStore::new(),
            stage: Stage::Header,
        }
    }

    /// Hands over bytes that were at `offset`.
    pub fn put(&mut self, offset: u64, bytes: Vec<u8>) {
        self.store.put(offset, bytes);
    }

    fn get(&self, need: Need) -> Option<&[u8]> {
        self.store.get(need.offset, need.len)
    }

    /// The bytes needed next, or the cloud.
    pub fn step(&mut self) -> Result<Step<Cloud>> {
        loop {
            let stage = std::mem::replace(&mut self.stage, Stage::Done);
            match stage {
                Stage::Header => {
                    let need = Need {
                        offset: 0,
                        len: (HEADER_1_4 as u64).min(self.size),
                    };
                    let Some(b) = self.get(need) else {
                        self.stage = Stage::Header;
                        return Ok(Step::Need(need));
                    };
                    let head = las::header(b, self.size)?;
                    self.stage = Stage::Vlrs(head);
                }
                Stage::Vlrs(head) => {
                    let len = u64::from(head.data_offset) - u64::from(head.header_size);
                    if len > VLRS_MOST {
                        return Err(PcError::new("VLR'ler 64 MB'tan büyük."));
                    }
                    let need = Need {
                        offset: u64::from(head.header_size),
                        len,
                    };
                    let Some(b) = self.get(need) else {
                        self.stage = Stage::Vlrs(head);
                        return Ok(Step::Need(need));
                    };
                    let vlrs = las::vlrs(&head, b)?;
                    let (at, left) = (head.evlr_start, head.evlr_count);
                    self.stage = Stage::Evlrs {
                        p: Partial {
                            head,
                            vlrs,
                            evlrs: Vec::new(),
                        },
                        at,
                        left,
                    };
                }
                Stage::Evlrs { p, at, left } => {
                    if left == 0 {
                        match self.after_records(p)? {
                            Ok(stage) => self.stage = stage,
                            Err(cloud) => {
                                self.store.clear();
                                return Ok(Step::Done(*cloud));
                            }
                        }
                        continue;
                    }
                    if at
                        .checked_add(EVLR_HEAD as u64)
                        .is_none_or(|e| e > self.size)
                    {
                        return Err(PcError::new("Bir EVLR dosyanın dışında."));
                    }
                    let need = Need {
                        offset: at,
                        len: EVLR_HEAD as u64,
                    };
                    let Some(b) = self.get(need) else {
                        self.stage = Stage::Evlrs { p, at, left };
                        return Ok(Step::Need(need));
                    };
                    let mut r = crate::bytes::Read::new(b);
                    r.u16()?;
                    let user = r.text(16)?;
                    let record = r.u16()?;
                    let len = r.u64()?;
                    let description = r.text(32)?;
                    let body = at + EVLR_HEAD as u64;
                    if body.checked_add(len).is_none_or(|e| e > self.size) {
                        return Err(PcError::new(format!(
                            "EVLR ({user}) dosyanın dışına taşıyor."
                        )));
                    }
                    if wanted(&user, record) {
                        if len > MAX_VLR_BYTES {
                            return Err(PcError::new(format!(
                                "EVLR ({user}) {} MB; en çok {} MB okunur.",
                                len >> 20,
                                MAX_VLR_BYTES >> 20
                            )));
                        }
                        self.stage = Stage::EvlrBody {
                            p,
                            at,
                            left,
                            user,
                            record,
                            description,
                            len,
                        };
                    } else {
                        self.stage = Stage::Evlrs {
                            p,
                            at: body + len,
                            left: left - 1,
                        };
                    }
                }
                Stage::EvlrBody {
                    mut p,
                    at,
                    left,
                    user,
                    record,
                    description,
                    len,
                } => {
                    let body = at + EVLR_HEAD as u64;
                    let need = Need { offset: body, len };
                    let Some(b) = self.get(need).map(<[u8]>::to_vec) else {
                        self.stage = Stage::EvlrBody {
                            p,
                            at,
                            left,
                            user,
                            record,
                            description,
                            len,
                        };
                        return Ok(Step::Need(need));
                    };
                    p.evlrs.push(Vlr {
                        user,
                        record,
                        description,
                        data: b,
                        extended: true,
                    });
                    self.stage = Stage::Evlrs {
                        p,
                        at: body + len,
                        left: left - 1,
                    };
                }
                Stage::TableOffset(o) => {
                    let need = Need {
                        offset: u64::from(o.head.data_offset),
                        len: 8,
                    };
                    let Some(b) = self.get(need) else {
                        self.stage = Stage::TableOffset(o);
                        return Ok(Step::Need(need));
                    };
                    let at = crate::bytes::Read::new(b).i64()?;
                    if at > i64::from(o.head.data_offset) && (at as u64) < self.size {
                        self.stage = Stage::Table(o, at as u64);
                    } else {
                        // The writer could not go back: LASzip writes the offset at the end too.
                        self.stage = Stage::TableAtEnd(o);
                    }
                }
                Stage::TableAtEnd(o) => {
                    let need = Need {
                        offset: self.size.saturating_sub(8),
                        len: 8.min(self.size),
                    };
                    let Some(b) = self.get(need) else {
                        self.stage = Stage::TableAtEnd(o);
                        return Ok(Step::Need(need));
                    };
                    let at = crate::bytes::Read::new(b).i64()?;
                    if at > i64::from(o.head.data_offset) && (at as u64) < self.size {
                        self.stage = Stage::Table(o, at as u64);
                    } else {
                        return Err(PcError::new(
                            "LAZ dosyasının parça tablosu yok (yazılırken kesilmiş olabilir).",
                        ));
                    }
                }
                Stage::Table(o, at) => {
                    let end = if o.head.evlr_count > 0 && o.head.evlr_start > at {
                        o.head.evlr_start
                    } else {
                        self.size
                    };
                    let len = end - at;
                    if len > VLRS_MOST {
                        return Err(PcError::new("LAZ'ın parça tablosu 64 MB'tan büyük."));
                    }
                    let need = Need { offset: at, len };
                    let Some(b) = self.get(need) else {
                        self.stage = Stage::Table(o, at);
                        return Ok(Step::Need(need));
                    };
                    let vlr = o
                        .laz
                        .as_ref()
                        .ok_or_else(|| PcError::new("LAZ'ın VLR'si yok."))?;
                    let entries = chunks::table(vlr, b)?;
                    let found = chunks::locate(&o.head, vlr, &entries)?;
                    if found
                        .last()
                        .is_some_and(|c| c.offset.saturating_add(c.bytes) > self.size)
                    {
                        return Err(PcError::new("LAZ'ın son parçası dosyanın dışında."));
                    }
                    self.store.clear();
                    return Ok(Step::Done(self.finish(o, found)));
                }
                Stage::Pages(mut o) => {
                    let pending = o
                        .hierarchy
                        .as_ref()
                        .and_then(|h| h.pending.first().copied());
                    match pending {
                        Some((offset, len)) => {
                            if offset.checked_add(len).is_none_or(|e| e > self.size)
                                || len > MAX_VLR_BYTES
                            {
                                return Err(PcError::new(
                                    "COPC'nin hiyerarşi sayfası dosyanın dışında.",
                                ));
                            }
                            let need = Need { offset, len };
                            let Some(b) = self.get(need).map(<[u8]>::to_vec) else {
                                self.stage = Stage::Pages(o);
                                return Ok(Step::Need(need));
                            };
                            if let Some(h) = o.hierarchy.as_mut() {
                                h.take_page((offset, len), &b, self.size)?;
                            }
                            self.stage = Stage::Pages(o);
                        }
                        None => {
                            let total = o.hierarchy.as_ref().map_or(0, Hierarchy::count);
                            if total != o.head.count {
                                return Err(PcError::new(format!(
                                    "COPC'nin hiyerarşisi {total} nokta sayıyor, başlık {} diyor.",
                                    o.head.count
                                )));
                            }
                            self.store.clear();
                            return Ok(Step::Done(self.finish(o, Vec::new())));
                        }
                    }
                }
                Stage::Done => {
                    return Err(PcError::new("Bulut zaten açıldı."));
                }
            }
        }
    }

    /// What comes after the VLRs and EVLRs: LAZ wants its chunk table, COPC its
    /// pages; LAS is done (the cloud, as the error side).
    fn after_records(&self, p: Partial) -> Result<std::result::Result<Stage, Box<Cloud>>> {
        let laz = if p.head.compressed {
            Some(chunks::laz_vlr(&p.head, &p.vlrs)?)
        } else {
            None
        };
        let info = p
            .vlrs
            .first()
            .filter(|v| v.is(copc::COPC_USER, copc::INFO_RECORD))
            .map(|v| Info::read(&v.data))
            .transpose()?;
        let copc = info.is_some() && p.head.compressed && matches!(p.head.format, 6..=8);
        let o = Opened {
            hierarchy: info.as_ref().filter(|_| copc).map(Hierarchy::new),
            head: p.head,
            vlrs: p.vlrs,
            evlrs: p.evlrs,
            laz,
            info: info.filter(|_| copc),
        };
        Ok(if copc {
            Ok(Stage::Pages(o))
        } else if o.laz.is_some() {
            Ok(Stage::TableOffset(o))
        } else {
            Err(Box::new(self.finish(o, Vec::new())))
        })
    }

    fn finish(&self, o: Opened, found: Vec<Chunk>) -> Cloud {
        let all: Vec<Vlr> = o.vlrs.iter().chain(&o.evlrs).cloned().collect();
        let crs = crs::of(&all);
        let kind = if o.info.is_some() {
            Kind::Copc
        } else if o.laz.is_some() {
            Kind::Laz
        } else {
            Kind::Las
        };
        Cloud {
            kind,
            size: self.size,
            layout: Layout::new(o.head.format, usize::from(o.head.record_len)),
            head: o.head,
            vlrs: o.vlrs,
            evlrs: o.evlrs,
            crs,
            laz: o.laz,
            chunks: found,
            info: o.info,
            hierarchy: o.hierarchy,
        }
    }
}
