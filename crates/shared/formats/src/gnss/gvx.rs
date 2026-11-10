//! A GNSS Vector Exchange file (NGS, GVX 1.0, 4 February 2021; docs/adr/1740): baseline vectors with their full
//! covariance, as a processor (or an RTK controller) gives them for a network adjustment. Each `GNSS_VECTOR` runs from
//! its `INITIAL_POINT_ID`'s point to its `TERMINAL_POINT_ID`'s; a point's `NAME` is the mark. `ECEF_DELTAS` are the
//! mark-to-mark components (the antenna heights applied) in metres; `CORRELATION_MATRIX` gives the standard deviations
//! `SDX`, `SDY`, `SDZ` (m) and the correlations `PXY`, `PXZ`, `PYZ`, so the covariance is xx = SDX², xy = PXY·SDX·SDY
//! and so on. Times are GPS time (`YYYY-MM-DDThh:mm:ss.ss`).
//!
//! A `SESSION` (docs/adr/4920) holds vectors processed together: its `CROSS_CORRELATION_MATRIX` (`ORDER` “XYZ”) has a
//! `CCM_BLOCK` for each pair of its vectors (`VECTOR_ID_ROW`, `VECTOR_ID_COL`), nine correlations row by row: the rows
//! the row vector's X, Y, Z, the columns the column vector's (the document's Figure A1 against A3). Its vectors are those
//! its blocks name (GVX does not mark a vector's session). A session naming fewer than two vectors, or fewer than its
//! `TOTAL_VECTORS`, is not kept; one without all n(n − 1)/2 blocks is kept and said to be incomplete. The independent
//! reference is `scripts/fixtures/map_survey_network_cases.py` (Python's ElementTree).

use kentos_contracts::{BaselineSolution, covariance_ok, survey_time};

use crate::text;
use crate::xml::{self, Node, Rows};

/// A vector read: its id in the file, its marks, ΔX ΔY ΔZ (m), its covariance xx xy xz yy yz zz (m²), when it was
/// observed (GPS time, `…Z`), how and with what it was made, how many epochs and satellites, and the line it is on.
#[derive(Clone, Debug, PartialEq)]
pub struct GvxVector {
    pub id: String,
    pub from: String,
    pub to: String,
    pub d: [f64; 3],
    pub covariance: [f64; 6],
    pub start: Option<String>,
    pub end: Option<String>,
    pub solution: Option<BaselineSolution>,
    pub software: Option<String>,
    pub epochs: Option<u32>,
    pub satellites: Option<u32>,
    pub line: u32,
}

/// A session kept: its id, its time (GPS, `…Z`), its vectors' ids in the file's order, its blocks (row vector, column
/// vector, nine correlations row by row) and the line it is on.
#[derive(Clone, Debug, PartialEq)]
pub struct GvxSession {
    pub id: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub vectors: Vec<String>,
    pub blocks: Vec<(String, String, [f64; 9])>,
    pub line: u32,
}

impl GvxSession {
    /// Whether every pair of its vectors has its block.
    pub fn complete(&self) -> bool {
        let n = self.vectors.len();
        self.blocks.len() == n * (n - 1) / 2
    }
}

/// A file's vectors in its order, its sessions and what was not read: sessions not kept, sessions kept incomplete.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GvxRead {
    pub vectors: Vec<GvxVector>,
    pub sessions: Vec<GvxSession>,
    pub ignored: Vec<String>,
    pub incomplete: Vec<String>,
}

/// Whether the bytes are a GVX file: XML whose root is `GVX`.
pub fn is_gvx(bytes: &[u8]) -> bool {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc);
    let head: String = body.chars().take(4096).collect();
    head.trim_start().starts_with('<') && head.contains("<GVX")
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

fn text_of(n: Option<Node<'_, '_>>) -> Option<String> {
    n.and_then(|n| n.text()).map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned)
}

/// The text of the element at `path` below `n`.
fn at(n: Node<'_, '_>, path: &[&str]) -> Option<String> {
    let mut here = n;
    for p in path {
        here = child(here, p)?;
    }
    text_of(Some(here))
}

/// GVX's `Datetime` (GPS time) as the archive's `…Z` time.
fn gps_time(t: &str) -> Option<String> {
    let z = if t.ends_with('Z') { t.to_owned() } else { format!("{t}Z") };
    survey_time(&z).map(|_| z)
}

/// The vectors of a GVX 1.0 file, or why it is not read (the line where it is not).
pub fn read(bytes: &[u8]) -> Result<GvxRead, String> {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc);
    let doc = xml::document(&body).map_err(|why| match why {
        xml::Unparsed::NotXml => "GVX dosyası okunamadı: geçerli bir XML değil.".to_owned(),
        xml::Unparsed::TooDeep => format!("GVX dosyası okunamadı: öğeler {} düzeyden derin.", xml::DEPTH),
    })?;
    let rows = Rows::new(&body);
    let line = |n: Node<'_, '_>| rows.row(n.range().start);
    let root = doc.root_element();
    if root.tag_name().name() != "GVX" {
        return Err(format!("Kök öğe “{}”; GVX dosyasının kökü GVX'tir.", root.tag_name().name()));
    }
    match root.attribute("VERSION").map(str::trim) {
        Some("1.0") => {}
        Some(v) => return Err(format!("GVX {v} sürümü okunmaz; KentOS GVX 1.0'ı (NGS, 4 Şubat 2021) okur.")),
        None => return Err("GVX öğesinin VERSION'ı yok; KentOS GVX 1.0'ı okur.".into()),
    }
    for rs in root.children().filter(|c| c.is_element() && c.tag_name().name() == "REFERENCE_SYSTEM") {
        let unit = at(rs, &["LINEAR_UNIT", "NAME"]);
        if unit.as_deref().is_some_and(|u| u != "meters") {
            return Err(format!(
                "Satır {}: uzunluk birimi “{}”; GVX'te uzunluk yalnız meters olur.",
                line(rs),
                unit.unwrap_or_default()
            ));
        }
    }
    // The points by their id: the mark's name.
    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for p in root.children().filter(|c| c.is_element() && c.tag_name().name() == "POINT") {
        let Some(id) = at(p, &["ID"]) else {
            return Err(format!("Satır {}: POINT öğesinin ID'si yok.", line(p)));
        };
        let Some(name) = at(p, &["NAME"]) else {
            return Err(format!("Satır {}: “{id}” noktasının NAME'i (işaretin adı) yok.", line(p)));
        };
        if names.insert(id.clone(), name).is_some() {
            return Err(format!("Satır {}: “{id}” nokta kimliği iki kez verilmiş.", line(p)));
        }
    }
    // The setups: how a vector was made and with which software.
    let mut setups: std::collections::HashMap<String, (Option<BaselineSolution>, Option<String>)> =
        std::collections::HashMap::new();
    for s in root.children().filter(|c| c.is_element() && c.tag_name().name() == "SURVEY_SETUP") {
        let Some(id) = at(s, &["ID"]) else { continue };
        let solution = match at(s, &["SOLUTION_TYPE"]).as_deref() {
            Some("Post-processed") => Some(BaselineSolution::PostProcessed),
            Some("Single-baseRTK") => Some(BaselineSolution::SingleBaseRtk),
            Some("NetworkRTK") => Some(BaselineSolution::NetworkRtk),
            _ => None,
        };
        let software = match (at(s, &["PROCESSING_SOFTWARE", "NAME"]), at(s, &["PROCESSING_SOFTWARE", "VERSION"])) {
            (Some(n), Some(v)) => Some(format!("{n} {v}")),
            (Some(n), None) => Some(n),
            _ => None,
        };
        setups.insert(id, (solution, software));
    }
    let mut out = GvxRead::default();
    let mut seen = std::collections::HashSet::new();
    for v in root.children().filter(|c| c.is_element() && c.tag_name().name() == "GNSS_VECTOR") {
        let n = line(v);
        let Some(id) = at(v, &["ID"]) else {
            return Err(format!("Satır {n}: GNSS_VECTOR öğesinin ID'si yok."));
        };
        if !seen.insert(id.clone()) {
            return Err(format!("Satır {n}: “{id}” vektör kimliği iki kez verilmiş."));
        }
        let mark = |tag: &str| -> Result<String, String> {
            let pid = at(v, &[tag]).ok_or_else(|| format!("Satır {n}: “{id}” vektörünün {tag}'si yok."))?;
            names.get(&pid).cloned().ok_or_else(|| format!("Satır {n}: “{id}” vektörünün {tag}'si “{pid}” dosyada bir POINT değil."))
        };
        let (from, to) = (mark("INITIAL_POINT_ID")?, mark("TERMINAL_POINT_ID")?);
        if from == to {
            return Err(format!("Satır {n}: “{id}” vektörünün iki ucu aynı işaret (“{from}”)."));
        }
        let number = |path: &[&str]| -> Result<f64, String> {
            let t = at(v, path).ok_or_else(|| format!("Satır {n}: “{id}” vektörünün {} değeri yok.", path.join("/")))?;
            t.parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .ok_or_else(|| format!("Satır {n}: “{id}” vektörünün {} değeri “{t}” sayı değil.", path.join("/")))
        };
        let d = [number(&["ECEF_DELTAS", "DX"])?, number(&["ECEF_DELTAS", "DY"])?, number(&["ECEF_DELTAS", "DZ"])?];
        let sd = [
            number(&["CORRELATION_MATRIX", "SDX"])?,
            number(&["CORRELATION_MATRIX", "SDY"])?,
            number(&["CORRELATION_MATRIX", "SDZ"])?,
        ];
        if sd.iter().any(|s| *s <= 0.0) {
            return Err(format!("Satır {n}: “{id}” vektörünün standart sapmaları 0'dan büyük olmalı."));
        }
        let p = [
            number(&["CORRELATION_MATRIX", "PXY"])?,
            number(&["CORRELATION_MATRIX", "PXZ"])?,
            number(&["CORRELATION_MATRIX", "PYZ"])?,
        ];
        if p.iter().any(|r| !(-1.0..=1.0).contains(r)) {
            return Err(format!("Satır {n}: “{id}” vektörünün korelasyonları −1 ile 1 arasında olmalı."));
        }
        let covariance = [
            sd[0] * sd[0],
            p[0] * sd[0] * sd[1],
            p[1] * sd[0] * sd[2],
            sd[1] * sd[1],
            p[2] * sd[1] * sd[2],
            sd[2] * sd[2],
        ];
        if !covariance_ok(&covariance) {
            return Err(format!("Satır {n}: “{id}” vektörünün kovaryansı pozitif tanımlı değil."));
        }
        let time = |tag: &str| -> Result<Option<String>, String> {
            match at(v, &["OBSERVATION_TIME", tag]) {
                None => Ok(None),
                Some(t) => gps_time(&t)
                    .map(Some)
                    .ok_or_else(|| format!("Satır {n}: “{id}” vektörünün gözlem zamanı “{t}” YYYY-AA-GGTss:dd:sn biçiminde değil.")),
            }
        };
        let (start, end) = (time("START")?, time("END")?);
        let (solution, software) = at(v, &["SURVEY_SETUP_ID"]).and_then(|s| setups.get(&s).cloned()).unwrap_or((None, None));
        let count = |path: &[&str]| at(v, path).and_then(|t| t.parse::<u32>().ok());
        out.vectors.push(GvxVector {
            id,
            from,
            to,
            d,
            covariance,
            start,
            end,
            solution,
            software,
            epochs: count(&["QUALITY_CONTROL", "EPOCHS_USED"]),
            satellites: count(&["QUALITY_CONTROL", "SATELLITE_USED", "TOTAL"]),
            line: n,
        });
    }
    if out.vectors.is_empty() {
        return Err("GVX dosyasında GNSS_VECTOR yok.".into());
    }
    sessions(root, &line, &mut out)?;
    Ok(out)
}

/// The file's sessions (docs/adr/4920): each kept with its vectors and blocks, or said not to be.
fn sessions(root: Node<'_, '_>, line: &dyn Fn(Node<'_, '_>) -> u32, out: &mut GvxRead) -> Result<(), String> {
    let order: std::collections::HashMap<&str, usize> =
        out.vectors.iter().enumerate().map(|(k, v)| (v.id.as_str(), k)).collect();
    let mut taken: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut kept = Vec::new();
    for s in root.children().filter(|c| c.is_element() && c.tag_name().name() == "SESSION") {
        let n = line(s);
        let id = s.attribute("ID").map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned);
        let Some(id) = id else {
            return Err(format!("Satır {n}: SESSION öğesinin ID'si yok."));
        };
        let time = |tag: &str| -> Result<Option<String>, String> {
            match at(s, &["SESSION_TIME", tag]) {
                None => Ok(None),
                Some(t) => gps_time(&t)
                    .map(Some)
                    .ok_or_else(|| format!("Satır {n}: “{id}” oturumunun zamanı “{t}” YYYY-AA-GGTss:dd:sn biçiminde değil.")),
            }
        };
        let (start, end) = (time("START")?, time("END")?);
        let mut blocks: Vec<(String, String, [f64; 9])> = Vec::new();
        let mut pairs = std::collections::HashSet::new();
        if let Some(m) = child(s, "CROSS_CORRELATION_MATRIX") {
            let axes = m.attribute("ORDER").map_or("XYZ", str::trim);
            if axes != "XYZ" {
                return Err(format!("Satır {}: “{id}” oturumunun ORDER'ı “{axes}”; KentOS XYZ sırasını okur.", line(m)));
            }
            for b in m.children().filter(|c| c.is_element() && c.tag_name().name() == "CCM_BLOCK") {
                let bn = line(b);
                let end_of = |name: &str| -> Result<String, String> {
                    let v = b.attribute(name).map(str::trim).unwrap_or_default();
                    if order.contains_key(v) {
                        Ok(v.to_owned())
                    } else {
                        Err(format!("Satır {bn}: “{id}” oturumunun bloğundaki {name} “{v}” dosyada bir GNSS_VECTOR değil."))
                    }
                };
                let (row, col) = (end_of("VECTOR_ID_ROW")?, end_of("VECTOR_ID_COL")?);
                if row == col {
                    return Err(format!("Satır {bn}: “{id}” oturumunun bloğu “{row}” vektörünü kendisiyle eşliyor."));
                }
                let pair = if row < col { (row.clone(), col.clone()) } else { (col.clone(), row.clone()) };
                if !pairs.insert(pair) {
                    return Err(format!("Satır {bn}: “{id}” oturumunda “{row}” ile “{col}” bloğu iki kez verilmiş."));
                }
                let text = text_of(child(b, "CORRELATIONS")).unwrap_or_default();
                let values: Vec<f64> = text.split(',').filter_map(|t| t.trim().parse::<f64>().ok()).collect();
                if values.len() != 9 || text.split(',').count() != 9 {
                    return Err(format!("Satır {bn}: “{id}” oturumunun “{row}”–“{col}” bloğu virgülle ayrılmış 9 sayı olmalı."));
                }
                if !values.iter().all(|r| r.is_finite() && (-1.0..=1.0).contains(r)) {
                    return Err(format!("Satır {bn}: “{id}” oturumunun “{row}”–“{col}” korelasyonları −1 ile 1 arasında olmalı."));
                }
                let mut r = [0.0; 9];
                r.copy_from_slice(&values);
                blocks.push((row, col, r));
            }
        }
        let mut vectors: Vec<String> = blocks.iter().flat_map(|(a, b, _)| [a.clone(), b.clone()]).collect();
        vectors.sort_by_key(|v| order.get(v.as_str()).copied().unwrap_or(usize::MAX));
        vectors.dedup();
        let total = s.attribute("TOTAL_VECTORS").and_then(|t| t.trim().parse::<usize>().ok());
        if vectors.len() < 2 || total.is_some_and(|t| t != vectors.len()) {
            out.ignored.push(match total {
                Some(t) if vectors.len() >= 2 => format!(
                    "“{id}” oturumunun TOTAL_VECTORS'ı {t}, blokları {} vektör anıyor; oturumun korelasyonları alınmadı.",
                    vectors.len()
                ),
                _ => format!("“{id}” oturumu iki vektörü bile anmıyor; korelasyonları alınmadı."),
            });
            continue;
        }
        for v in &vectors {
            if let Some(other) = taken.insert(v.clone(), id.clone()) {
                return Err(format!("Satır {n}: “{v}” vektörü hem “{other}” hem “{id}” oturumunda."));
            }
        }
        let session = GvxSession { id, start, end, vectors, blocks, line: n };
        if !session.complete() {
            let k = session.vectors.len();
            out.incomplete.push(format!(
                "“{}” oturumunun {} vektörü için {} blok olmalı, {} var; oturum korelasyonlarıyla dengelenemez.",
                session.id,
                k,
                k * (k - 1) / 2,
                session.blocks.len()
            ));
        }
        kept.push(session);
    }
    out.sessions = kept;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<GVX VERSION="1.0">
  <REFERENCE_SYSTEM><ID>153</ID><NAME>IGS14</NAME><LINEAR_UNIT><NAME>meters</NAME></LINEAR_UNIT></REFERENCE_SYSTEM>
  <SURVEY_SETUP><ID>S1</ID><SOLUTION_TYPE>Post-processed</SOLUTION_TYPE>
    <PROCESSING_SOFTWARE><NAME>Isleyici</NAME><VERSION>5.90</VERSION></PROCESSING_SOFTWARE></SURVEY_SETUP>
  <POINT><ID>P1</ID><NAME>ANKR</NAME></POINT>
  <POINT><ID>P2</ID><NAME>N1</NAME></POINT>
  <GNSS_VECTOR><ID>V1</ID><INITIAL_POINT_ID>P1</INITIAL_POINT_ID><TERMINAL_POINT_ID>P2</TERMINAL_POINT_ID>
    <SURVEY_SETUP_ID>S1</SURVEY_SETUP_ID>
    <OBSERVATION_TIME><START>2026-11-03T08:00:00.00</START><END>2026-11-03T10:00:00.00</END></OBSERVATION_TIME>
    <QUALITY_CONTROL><EPOCHS_USED>481</EPOCHS_USED><SATELLITE_USED><TOTAL>17</TOTAL></SATELLITE_USED></QUALITY_CONTROL>
    <ECEF_DELTAS><DX>1000.5</DX><DY>-200.25</DY><DZ>300.125</DZ></ECEF_DELTAS>
    <CORRELATION_MATRIX><SDX>0.002</SDX><SDY>0.004</SDY><SDZ>0.003</SDZ><PXY>-0.5</PXY><PXZ>0.25</PXZ><PYZ>0.1</PYZ></CORRELATION_MATRIX>
  </GNSS_VECTOR>
</GVX>"#;

    #[test]
    fn a_vector_is_read_with_its_covariance_marks_and_times() {
        assert!(is_gvx(ONE.as_bytes()));
        let r = read(ONE.as_bytes()).expect("read");
        assert_eq!(r.vectors.len(), 1);
        let v = &r.vectors[0];
        assert_eq!((v.id.as_str(), v.from.as_str(), v.to.as_str()), ("V1", "ANKR", "N1"));
        assert_eq!(v.d, [1000.5, -200.25, 300.125]);
        assert_eq!(v.covariance, [0.002 * 0.002, -0.5 * 0.002 * 0.004, 0.25 * 0.002 * 0.003, 0.004 * 0.004, 0.1 * 0.004 * 0.003, 0.003 * 0.003]);
        assert_eq!(v.start.as_deref(), Some("2026-11-03T08:00:00.00Z"));
        assert_eq!(v.solution, Some(BaselineSolution::PostProcessed));
        assert_eq!(v.software.as_deref(), Some("Isleyici 5.90"));
        assert_eq!((v.epochs, v.satellites), (Some(481), Some(17)));
        assert!(r.ignored.is_empty() && r.incomplete.is_empty() && r.sessions.is_empty());
    }

    #[test]
    fn a_wrong_file_says_where() {
        assert!(read(b"not xml").is_err());
        assert!(read(ONE.replace("VERSION=\"1.0\"", "VERSION=\"2.0\"").as_bytes()).unwrap_err().contains("2.0"));
        assert!(read(ONE.replace("<NAME>meters</NAME>", "<NAME>feet</NAME>").as_bytes()).unwrap_err().contains("feet"));
        assert!(read(ONE.replace("<PXY>-0.5</PXY>", "<PXY>-1.5</PXY>").as_bytes()).unwrap_err().contains("V1"));
        assert!(read(ONE.replace("<TERMINAL_POINT_ID>P2", "<TERMINAL_POINT_ID>P9").as_bytes()).unwrap_err().contains("P9"));
        assert!(read(ONE.replace("<NAME>N1</NAME>", "<NAME>ANKR</NAME>").as_bytes()).unwrap_err().contains("aynı işaret"));
        let crossed = ONE.replace("</GVX>", "<SESSION ID=\"S\" TOTAL_VECTORS=\"2\"><CROSS_CORRELATION_MATRIX ORDER=\"XYZ\"/></SESSION></GVX>");
        assert_eq!(read(crossed.as_bytes()).expect("read").ignored.len(), 1);
    }

    /// Three vectors from one base: a session's blocks, row by row, its members in the file's order.
    #[test]
    fn a_session_keeps_its_blocks_and_says_when_one_is_missing() {
        let mut three = ONE.to_owned();
        let v1 = &ONE[ONE.find("<GNSS_VECTOR>").unwrap()..ONE.find("</GVX>").unwrap()];
        for k in [2, 3] {
            three = three.replace("</GVX>", &format!("{}</GVX>", v1.replace("<ID>V1</ID>", &format!("<ID>V{k}</ID>"))));
        }
        let block = |r: &str, c: &str| {
            format!("<CCM_BLOCK VECTOR_ID_ROW=\"{r}\" VECTOR_ID_COL=\"{c}\"><CORRELATIONS>0.1,0,0,0,0.2,0,0,0,0.3</CORRELATIONS></CCM_BLOCK>")
        };
        let session = |blocks: &str| {
            three.replace(
                "</GVX>",
                &format!("<SESSION ID=\"O1\" TOTAL_VECTORS=\"3\"><SESSION_TIME><START>2026-11-03T08:00:00.00</START><END>2026-11-03T10:00:00.00</END></SESSION_TIME><CROSS_CORRELATION_MATRIX ORDER=\"XYZ\">{blocks}</CROSS_CORRELATION_MATRIX></SESSION></GVX>"),
            )
        };
        let full = read(session(&[block("V2", "V3"), block("V1", "V2"), block("V1", "V3")].concat()).as_bytes()).expect("read");
        assert_eq!(full.sessions.len(), 1);
        let s = &full.sessions[0];
        assert_eq!(s.vectors, ["V1", "V2", "V3"]);
        assert!(s.complete() && full.incomplete.is_empty());
        assert_eq!(s.blocks[0], ("V2".into(), "V3".into(), [0.1, 0.0, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.3]));
        assert_eq!(s.start.as_deref(), Some("2026-11-03T08:00:00.00Z"));
        let short = read(session(&[block("V1", "V2"), block("V1", "V3")].concat()).as_bytes()).expect("read");
        assert_eq!((short.sessions.len(), short.incomplete.len()), (1, 1));
        assert!(read(session(&[block("V1", "V2"), block("V2", "V1")].concat()).as_bytes()).unwrap_err().contains("iki kez"));
        assert!(read(session(&block("V1", "V9")).as_bytes()).unwrap_err().contains("V9"));
        assert!(read(session(&block("V1", "V2").replace("0.3", "1.3")).as_bytes()).unwrap_err().contains("−1 ile 1"));
        assert_eq!(read(session(&block("V1", "V2")).as_bytes()).expect("read").ignored.len(), 1);
    }
}
