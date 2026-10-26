//! GRIB1 and GRIB2 files read by KentOS's own code (docs/adr/0645 §4). Every message's sections are read but its data
//! (each field one chunk, decoded only when read); fields on a regular latitude–longitude grid (GRIB1 grid 0, GRIB2
//! template 3.0) become variables of a chunked [`Model`]: one per parameter and level type, over a time dimension of
//! every valid time and, for a level type with several levels, a level dimension; a field the file does not hold for a
//! time or level is fill. Grids of other projections (rotated, Gaussian, Lambert, Mercator, polar stereographic) and
//! JPEG 2000 packing are left out with their reason.

pub mod ccsds;
pub mod decode;
pub mod tables;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use super::chunked::{Chunked, Chunks, Data, Decoder, Dim, Fetch, Model, Place, Variable};
use super::netcdf::{Attr, NcType};

/// The most messages a file may hold.
const MESSAGES_MOST: usize = 200_000;

/// An unsigned big-endian number of `n` bytes at `at`.
pub fn sec_u(b: &[u8], at: usize, n: usize) -> u64 {
    let mut v = 0u64;
    for k in 0..n {
        v = (v << 8) | u64::from(*b.get(at + k).unwrap_or(&0));
    }
    v
}

/// A sign-and-magnitude number of `n` bytes at `at` (GRIB's signed numbers).
pub fn sec_int(b: &[u8], at: usize, n: usize) -> i64 {
    let v = sec_u(b, at, n);
    let top = 1u64 << (8 * n - 1);
    if v & top != 0 {
        -((v & (top - 1)) as i64)
    } else {
        v as i64
    }
}

/// A field's regular latitude–longitude grid: points, corners and steps in degrees, scanning mode.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub ni: u64,
    pub nj: u64,
    pub lat1: f64,
    pub lon1: f64,
    pub lat2: f64,
    pub lon2: f64,
    pub di: f64,
    pub dj: f64,
    pub scan: u8,
}

impl Grid {
    fn key(&self) -> String {
        format!(
            "{}x{}|{:.6}|{:.6}|{:.6}|{:.6}|{}",
            self.ni, self.nj, self.lat1, self.lon1, self.lat2, self.lon2, self.scan
        )
    }

    /// The columns' longitudes west to east and the rows' latitudes in the file's row order.
    fn axes(&self) -> (Vec<f64>, Vec<f64>) {
        let ni = self.ni.max(1) as f64;
        let nj = self.nj.max(1) as f64;
        let (w, mut e) = if self.scan & 0x80 != 0 {
            (self.lon2, self.lon1)
        } else {
            (self.lon1, self.lon2)
        };
        if e < w {
            e += 360.0;
        }
        let di = if self.ni > 1 {
            (e - w) / (ni - 1.0)
        } else {
            self.di
        };
        let lons = (0..self.ni).map(|i| round6(w + i as f64 * di)).collect();
        let dj = if self.nj > 1 {
            (self.lat2 - self.lat1) / (nj - 1.0)
        } else {
            0.0
        };
        let lats = (0..self.nj)
            .map(|j| round6(self.lat1 + j as f64 * dj))
            .collect();
        (lons, lats)
    }
}

fn round6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

/// A field of a message: what it is and where its sections lie (from the message's start).
#[derive(Clone, Debug)]
pub struct Message {
    pub discipline: u8,
    pub category: u8,
    pub number: u8,
    /// GRIB1's table version and parameter.
    pub table: u8,
    pub level_type: u16,
    pub level: f64,
    /// Seconds since 1970 of the reference and valid times.
    pub reference: i64,
    pub valid: i64,
    pub grid: Grid,
    pub grid_ok: Result<(), String>,
    pub repr: u16,
    pub repr_at: usize,
    pub data_at: usize,
    pub bitmap: Option<Vec<u8>>,
    pub decimal: i32,
    pub member: Option<u32>,
    pub stat: Option<u8>,
}

/// A message read whole (its bytes): edition and fields.
pub struct Read {
    pub edition: u8,
    pub fields: Vec<Message>,
}

/// Days from 1970-01-01 to a civil date.
fn days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

fn moment(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> i64 {
    days(y, mo, d) * 86_400 + h * 3600 + mi * 60 + s
}

/// Seconds of `n` units of GRIB's time unit code.
fn unit_seconds(unit: u8) -> Option<i64> {
    Some(match unit {
        0 => 60,
        1 => 3600,
        2 => 86_400,
        10 => 3 * 3600,
        11 => 6 * 3600,
        12 => 12 * 3600,
        13 => 1,
        254 => 1,
        _ => return None,
    })
}

/// A message's fields from its whole bytes (section 0 at the start).
pub fn read_message(b: &[u8]) -> Result<Read, String> {
    if !b.starts_with(b"GRIB") || b.len() < 8 {
        return Err("GRIB iletisi değil".into());
    }
    match b[7] {
        1 => Ok(Read {
            edition: 1,
            fields: vec![grib1_field(b)?],
        }),
        2 => Ok(Read {
            edition: 2,
            fields: grib2_fields(b)?,
        }),
        e => Err(format!("GRIB'in {e}. sürümü okunmuyor")),
    }
}

fn grib1_field(b: &[u8]) -> Result<Message, String> {
    let pds = 8usize;
    let plen = sec_u(b, pds, 3) as usize;
    let table = *b.get(pds + 3).unwrap_or(&0);
    let flags = *b.get(pds + 7).unwrap_or(&0);
    let param = *b.get(pds + 8).unwrap_or(&0);
    let ltype = *b.get(pds + 9).unwrap_or(&0);
    let lval = sec_u(b, pds + 10, 2) as f64;
    let (yc, mo, d, h, mi) = (
        i64::from(b[pds + 12]),
        i64::from(b[pds + 13]),
        i64::from(b[pds + 14]),
        i64::from(b[pds + 15]),
        i64::from(b[pds + 16]),
    );
    let unit = b[pds + 17];
    let (p1, p2, tri) = (i64::from(b[pds + 18]), i64::from(b[pds + 19]), b[pds + 20]);
    let century = i64::from(*b.get(pds + 24).unwrap_or(&20));
    let decimal = sec_int(b, pds + 26, 2) as i32;
    let year = (century - 1) * 100 + yc;
    let reference = moment(year, mo, d, h, mi, 0);
    let us = unit_seconds(unit).unwrap_or(3600);
    let step = match tri {
        2..=5 => p2,
        10 => (p1 << 8) | p2,
        _ => p1,
    };
    let valid = reference + step * us;
    let mut at = pds + plen;
    let mut grid = Grid {
        ni: 0,
        nj: 0,
        lat1: 0.0,
        lon1: 0.0,
        lat2: 0.0,
        lon2: 0.0,
        di: 0.0,
        dj: 0.0,
        scan: 0,
    };
    let mut grid_ok = Err("GRIB1 alanının ızgara tanımı (GDS) yok".to_owned());
    if flags & 0x80 != 0 {
        let glen = sec_u(b, at, 3) as usize;
        let kind = *b.get(at + 5).unwrap_or(&255);
        if kind == 0 {
            let g = at + 6;
            grid = Grid {
                ni: sec_u(b, g, 2),
                nj: sec_u(b, g + 2, 2),
                lat1: sec_int(b, g + 4, 3) as f64 / 1000.0,
                lon1: sec_int(b, g + 7, 3) as f64 / 1000.0,
                lat2: sec_int(b, g + 11, 3) as f64 / 1000.0,
                lon2: sec_int(b, g + 14, 3) as f64 / 1000.0,
                di: sec_u(b, g + 17, 2) as f64 / 1000.0,
                dj: sec_u(b, g + 19, 2) as f64 / 1000.0,
                scan: *b.get(g + 21).unwrap_or(&0),
            };
            grid_ok = Ok(());
        } else {
            grid_ok = Err(format!(
                "GRIB1'in {kind} numaralı ızgara türü okunmuyor (yalnız düzenli enlem-boylam)"
            ));
        }
        at += glen;
    }
    let bitmap = if flags & 0x40 != 0 {
        let blen = sec_u(b, at, 3) as usize;
        let table_ref = sec_u(b, at + 4, 2);
        let bm = (table_ref == 0).then(|| b.get(at + 6..at + blen).unwrap_or(&[]).to_vec());
        at += blen;
        bm
    } else {
        None
    };
    Ok(Message {
        discipline: 0,
        category: 0,
        number: param,
        table,
        level_type: u16::from(ltype),
        level: lval,
        reference,
        valid,
        grid,
        grid_ok,
        repr: 0,
        repr_at: 0,
        data_at: at,
        bitmap,
        decimal,
        member: None,
        stat: (2..=5).contains(&tri).then_some(tri),
    })
}

fn grib2_fields(b: &[u8]) -> Result<Vec<Message>, String> {
    let discipline = b[6];
    let total = sec_u(b, 8, 8) as usize;
    let mut at = 16usize;
    let mut out = Vec::new();
    let mut reference = 0i64;
    let mut grid = None::<(Grid, Result<(), String>)>;
    let mut prod: Option<Product2> = None;
    let mut repr: Option<(u16, usize)> = None;
    let mut bitmap: Option<Vec<u8>> = None;
    while at + 4 <= total.min(b.len()) {
        if &b[at..at + 4] == b"7777" {
            break;
        }
        let len = sec_u(b, at, 4) as usize;
        let num = *b.get(at + 4).ok_or("GRIB2 bölümü eksik")?;
        if len < 5 {
            return Err("GRIB2 bölümünün boyu bozuk".into());
        }
        let s = &b[at..(at + len).min(b.len())];
        match num {
            1 => {
                reference = moment(
                    sec_u(s, 12, 2) as i64,
                    sec_u(s, 14, 1) as i64,
                    sec_u(s, 15, 1) as i64,
                    sec_u(s, 16, 1) as i64,
                    sec_u(s, 17, 1) as i64,
                    sec_u(s, 18, 1) as i64,
                );
            }
            3 => grid = Some(grid2(s)),
            4 => prod = Some(product2(s, reference)?),
            5 => repr = Some((sec_u(s, 9, 2) as u16, at)),
            6 => {
                let ind = *s.get(5).unwrap_or(&255);
                match ind {
                    0 => bitmap = Some(s.get(6..).unwrap_or(&[]).to_vec()),
                    254 => {}
                    _ => bitmap = None,
                }
            }
            7 => {
                let (Some((g, ok)), Some(p), Some((repr_t, repr_at))) = (&grid, &prod, &repr)
                else {
                    return Err(
                        "GRIB2 iletisinde veri bölümünden önce ızgara, ürün ya da gösterim yok"
                            .into(),
                    );
                };
                let decimal = sec_int(&b[*repr_at..], 17, 2) as i32;
                out.push(Message {
                    discipline,
                    category: p.0,
                    number: p.1,
                    table: 0,
                    level_type: p.2,
                    level: p.3,
                    reference,
                    valid: p.4,
                    grid: g.clone(),
                    grid_ok: ok.clone(),
                    repr: *repr_t,
                    repr_at: *repr_at,
                    data_at: at,
                    bitmap: bitmap.clone(),
                    decimal,
                    member: p.5,
                    stat: p.6,
                });
            }
            _ => {}
        }
        at += len;
    }
    Ok(out)
}

/// GRIB2's grid definition: a regular latitude–longitude one, or why not.
fn grid2(s: &[u8]) -> (Grid, Result<(), String>) {
    let template = sec_u(s, 12, 2);
    let mut g = Grid {
        ni: 0,
        nj: 0,
        lat1: 0.0,
        lon1: 0.0,
        lat2: 0.0,
        lon2: 0.0,
        di: 0.0,
        dj: 0.0,
        scan: 0,
    };
    if template != 0 {
        let name = match template {
            1 => "döndürülmüş enlem-boylam",
            10 => "Mercator",
            20 => "kutupsal stereografik",
            30 => "Lambert konik",
            40 => "Gauss",
            50 => "küresel harmonik",
            90 => "uzay görüntüsü",
            _ => "bilinmeyen",
        };
        return (
            g,
            Err(format!(
                "GRIB2'nin 3.{template} ({name}) ızgarası okunmuyor (yalnız düzenli enlem-boylam); `gdalwarp -t_srs EPSG:4326` ile çevirin"
            )),
        );
    }
    let t = 14usize;
    let ni = sec_u(s, t + 16, 4);
    let nj = sec_u(s, t + 20, 4);
    let basic = sec_u(s, t + 24, 4);
    let sub = sec_u(s, t + 28, 4);
    let unit = if basic == 0 || basic == 0xFFFF_FFFF || sub == 0xFFFF_FFFF || sub == 0 {
        1e-6
    } else {
        basic as f64 / sub as f64
    };
    g = Grid {
        ni,
        nj,
        lat1: sec_int(s, t + 32, 4) as f64 * unit,
        lon1: sec_int(s, t + 36, 4) as f64 * unit,
        lat2: sec_int(s, t + 41, 4) as f64 * unit,
        lon2: sec_int(s, t + 45, 4) as f64 * unit,
        di: sec_u(s, t + 49, 4) as f64 * unit,
        dj: sec_u(s, t + 53, 4) as f64 * unit,
        scan: *s.get(t + 57).unwrap_or(&0),
    };
    let ok = if g.scan & 0x10 != 0 {
        Err(
            "GRIB2 alanı satır satır yön değiştirerek taranmış (boustrophedon); okunmuyor"
                .to_owned(),
        )
    } else if ni == 0 || nj == 0 || ni == 0xFFFF_FFFF || nj == 0xFFFF_FFFF {
        Err(
            "GRIB2 alanının ızgarası düzensiz (nokta sayısı satırdan satıra değişiyor); okunmuyor"
                .to_owned(),
        )
    } else {
        Ok(())
    };
    (g, ok)
}

/// GRIB2's product definition: category, number, level type and value, valid time, ensemble member, statistic.
type Product2 = (u8, u8, u16, f64, i64, Option<u32>, Option<u8>);

/// GRIB2's product definition: category, number, level type and value, valid time, ensemble member, statistic.
fn product2(s: &[u8], reference: i64) -> Result<Product2, String> {
    let template = sec_u(s, 7, 2);
    let t = 9usize;
    let category = *s.get(t).unwrap_or(&255);
    let number = *s.get(t + 1).unwrap_or(&255);
    if !matches!(template, 0 | 1 | 8 | 11) {
        return Err(format!("GRIB2'nin 4.{template} ürün şablonu okunmuyor"));
    }
    let unit = *s.get(t + 8).unwrap_or(&1);
    let fct = sec_int(s, t + 9, 4);
    let ltype = u16::from(*s.get(t + 13).unwrap_or(&255));
    // A missing scale factor (all bits set) counts as 0.
    let lscale = if sec_u(s, t + 14, 1) == 0xFF {
        0
    } else {
        sec_int(s, t + 14, 1) as i32
    };
    let lvalue = sec_int(s, t + 15, 4);
    let level = if ltype == 255 || sec_u(s, t + 15, 4) == 0xFFFF_FFFF {
        0.0
    } else {
        lvalue as f64 * decode::power(10.0, -lscale)
    };
    let member = matches!(template, 1 | 11).then(|| u32::from(*s.get(t + 26).unwrap_or(&0)));
    let mut valid = reference + fct * unit_seconds(unit).unwrap_or(3600);
    let mut stat = None;
    if matches!(template, 8 | 11) {
        let e = t + if template == 11 { 28 } else { 25 };
        valid = moment(
            sec_u(s, e, 2) as i64,
            sec_u(s, e + 2, 1) as i64,
            sec_u(s, e + 3, 1) as i64,
            sec_u(s, e + 4, 1) as i64,
            sec_u(s, e + 5, 1) as i64,
            sec_u(s, e + 6, 1) as i64,
        );
        stat = Some(*s.get(e + 12).unwrap_or(&255));
    }
    Ok((category, number, ltype, round6(level), valid, member, stat))
}

/// A field to place: its variable key, time, level, message's place.
struct Found {
    key: String,
    name: String,
    attrs: Vec<(String, Attr)>,
    grid_key: String,
    grid: Grid,
    valid: i64,
    level_type: u16,
    level: f64,
    offset: u64,
    len: u64,
    field: u32,
}

/// A GRIB file's model: every message read from `fetch`.
pub fn open(fetch: Arc<dyn Fetch>, size: u64) -> Result<Model, String> {
    let mut found = Vec::new();
    let mut notes: BTreeSet<String> = BTreeSet::new();
    let mut at = 0u64;
    let mut edition = 0u8;
    let mut count = 0usize;
    while at + 16 <= size {
        let head = fetch.range(at, 16)?;
        if !head.starts_with(b"GRIB") {
            // Bytes between messages: look on for the next “GRIB” in the next 64 KB.
            let n = (64u64 << 10).min(size - at);
            let window = fetch.range(at, n)?;
            match window.windows(4).skip(1).position(|w| w == b"GRIB") {
                Some(p) => {
                    at += p as u64 + 1;
                    continue;
                }
                None => {
                    at += n.saturating_sub(3).max(1);
                    continue;
                }
            }
        }
        let ed = head[7];
        let len = match ed {
            1 => sec_u(&head, 4, 3),
            2 => sec_u(&head, 8, 8),
            e => return Err(format!("GRIB'in {e}. sürümü okunmuyor.")),
        };
        if len < 16 || at + len > size {
            return Err(
                "GRIB iletisinin boyu dosyanın dışını gösteriyor; dosya kesik olabilir.".into(),
            );
        }
        count += 1;
        if count > MESSAGES_MOST {
            return Err(format!("GRIB dosyasında {MESSAGES_MOST} iletiden çok var."));
        }
        edition = edition.max(ed);
        let bytes = fetch.range(at, len)?;
        let read = read_message(&bytes)?;
        for (k, m) in read.fields.iter().enumerate() {
            let (name, attrs) = tables::describe(ed, m);
            if let Err(e) = &m.grid_ok {
                notes.insert(format!("“{name}”: {e}."));
                continue;
            }
            if ed == 2 && m.repr == 40 {
                notes.insert(format!(
                    "“{name}”: JPEG 2000 ile paketlenmiş; `grib_set -r -s packingType=grid_simple girdi.grib2 çıktı.grib2` ile çevirin."
                ));
                continue;
            }
            let member = m.member.map(|n| format!("_m{n}")).unwrap_or_default();
            let stat = m.stat.map(|s| format!("|s{s}")).unwrap_or_default();
            found.push(Found {
                key: format!("{name}{member}{stat}|{}", m.level_type),
                name: format!("{name}{member}"),
                attrs,
                grid_key: m.grid.key(),
                grid: m.grid.clone(),
                valid: m.valid,
                level_type: m.level_type,
                level: m.level,
                offset: at,
                len,
                field: k as u32,
            });
        }
        at += len;
    }
    if found.is_empty() && notes.is_empty() {
        return Err("GRIB dosyasında okunacak alan yok.".into());
    }
    build(edition, found, notes.into_iter().collect())
}

fn build(edition: u8, found: Vec<Found>, notes: Vec<String>) -> Result<Model, String> {
    let mut dims: Vec<Dim> = Vec::new();
    let mut vars: Vec<Variable> = Vec::new();
    // Time: every valid time.
    let times: BTreeSet<i64> = found.iter().map(|f| f.valid).collect();
    let times: Vec<i64> = times.into_iter().collect();
    let time_dim = dims.len();
    dims.push(Dim {
        name: "time".into(),
        len: times.len() as u64,
    });
    vars.push(Variable {
        name: "time".into(),
        dims: vec![time_dim],
        kind: NcType::Double,
        attrs: vec![
            ("standard_name".into(), Attr::Text("time".into())),
            (
                "units".into(),
                Attr::Text("seconds since 1970-01-01 00:00:00".into()),
            ),
            ("calendar".into(), Attr::Text("proleptic_gregorian".into())),
            ("axis".into(), Attr::Text("T".into())),
        ],
        data: Data::Values(times.iter().map(|&t| t as f64).collect()),
    });
    let time_at: HashMap<i64, u64> = times
        .iter()
        .enumerate()
        .map(|(k, &t)| (t, k as u64))
        .collect();
    // Grids: their two axes.
    let mut grids: BTreeMap<String, (usize, usize, Grid)> = BTreeMap::new();
    for f in &found {
        if grids.contains_key(&f.grid_key) {
            continue;
        }
        let n = grids.len();
        let suffix = if n == 0 {
            String::new()
        } else {
            format!("_{}", n + 1)
        };
        let (lons, lats) = f.grid.axes();
        let lat_dim = dims.len();
        dims.push(Dim {
            name: format!("latitude{suffix}"),
            len: lats.len() as u64,
        });
        let lon_dim = dims.len();
        dims.push(Dim {
            name: format!("longitude{suffix}"),
            len: lons.len() as u64,
        });
        vars.push(Variable {
            name: format!("latitude{suffix}"),
            dims: vec![lat_dim],
            kind: NcType::Double,
            attrs: vec![
                ("standard_name".into(), Attr::Text("latitude".into())),
                ("units".into(), Attr::Text("degrees_north".into())),
                ("axis".into(), Attr::Text("Y".into())),
            ],
            data: Data::Values(lats),
        });
        vars.push(Variable {
            name: format!("longitude{suffix}"),
            dims: vec![lon_dim],
            kind: NcType::Double,
            attrs: vec![
                ("standard_name".into(), Attr::Text("longitude".into())),
                ("units".into(), Attr::Text("degrees_east".into())),
                ("axis".into(), Attr::Text("X".into())),
            ],
            data: Data::Values(lons),
        });
        grids.insert(f.grid_key.clone(), (lat_dim, lon_dim, f.grid.clone()));
    }
    // Variables: one per parameter and level type (and grid).
    let mut groups: BTreeMap<String, Vec<&Found>> = BTreeMap::new();
    for f in &found {
        groups
            .entry(format!("{}|{}", f.key, f.grid_key))
            .or_default()
            .push(f);
    }
    let mut level_dims: HashMap<(u16, Vec<i64>), usize> = HashMap::new();
    let mut names: BTreeSet<String> = vars.iter().map(|v| v.name.clone()).collect();
    for fs in groups.values() {
        let first = fs[0];
        let (lat_dim, lon_dim, grid) = grids[&first.grid_key].clone();
        let levels: BTreeSet<i64> = fs.iter().map(|f| (f.level * 1e6).round() as i64).collect();
        let levels: Vec<i64> = levels.into_iter().collect();
        let (lname, lunits, positive) = tables::level_name(edition, first.level_type);
        let mut vdims = vec![time_dim];
        let mut name = first.name.clone();
        let mut attrs = first.attrs.clone();
        let level_dim = if levels.len() > 1 {
            let key = (first.level_type, levels.clone());
            let d = *level_dims.entry(key).or_insert_with(|| {
                let d = dims.len();
                let dname = if names.contains(lname) || dims.iter().any(|x| x.name == lname) {
                    format!("{lname}_{}", d)
                } else {
                    lname.to_owned()
                };
                dims.push(Dim {
                    name: dname.clone(),
                    len: levels.len() as u64,
                });
                let mut a = vec![
                    ("units".into(), Attr::Text(lunits.into())),
                    ("axis".into(), Attr::Text("Z".into())),
                ];
                if let Some(p) = positive {
                    a.push(("positive".into(), Attr::Text(p.into())));
                }
                vars.push(Variable {
                    name: dname.clone(),
                    dims: vec![d],
                    kind: NcType::Double,
                    attrs: a,
                    data: Data::Values(levels.iter().map(|&l| l as f64 / 1e6).collect()),
                });
                names.insert(dname);
                d
            });
            vdims.push(d);
            Some(d)
        } else {
            let v = levels[0] as f64 / 1e6;
            name = tables::with_level(&name, edition, first.level_type, v);
            attrs.push((
                "level".into(),
                Attr::Numbers {
                    kind: NcType::Double,
                    values: vec![v],
                },
            ));
            attrs.push(("level_units".into(), Attr::Text(lunits.into())));
            None
        };
        vdims.push(lat_dim);
        vdims.push(lon_dim);
        let mut uniq = name.clone();
        let mut k = 2;
        while names.contains(&uniq) {
            uniq = format!("{name}_{k}");
            k += 1;
        }
        names.insert(uniq.clone());
        let mut table = HashMap::new();
        for f in fs {
            let t = time_at[&f.valid];
            let mut coords = vec![t];
            if level_dim.is_some() {
                let l = (f.level * 1e6).round() as i64;
                coords.push(levels.iter().position(|&x| x == l).unwrap_or(0) as u64);
            }
            coords.push(0);
            coords.push(0);
            table.insert(
                coords,
                Place::Range {
                    offset: f.offset,
                    len: f.len,
                    mask: f.field,
                },
            );
        }
        let mut chunk = vec![1u64; vdims.len()];
        let n = chunk.len();
        chunk[n - 2] = grid.nj;
        chunk[n - 1] = grid.ni;
        attrs.push((
            "_FillValue".into(),
            Attr::Numbers {
                kind: NcType::Double,
                values: vec![f64::NAN],
            },
        ));
        vars.push(Variable {
            name: uniq,
            dims: vdims,
            kind: NcType::Double,
            attrs,
            data: Data::Chunked(Chunked {
                chunk,
                decoder: Decoder::Grib,
                chunks: Chunks::Table(table),
                fortran: false,
                fill: f64::NAN,
            }),
        });
    }
    Ok(Model {
        format: format!("GRIB{edition}"),
        dims,
        attrs: vec![("Conventions".into(), Attr::Text("CF-1.8".into()))],
        vars,
        notes,
    })
}
