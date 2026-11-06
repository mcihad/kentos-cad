//! A RINEX observation file's header (docs/adr/1340 §4; RINEX 2.11, 3.05 and 4.00 keep the same columns): the
//! marker, receiver, antenna and its height to the ARP with its eccentricity (`ANTENNA: DELTA H/E/N`), the first and
//! last observation times with their time system, the interval and the approximate position. Each header line's label
//! is in columns 61–80. The observations after `END OF HEADER` are not read here (a processor's work, docs/adr/1340
//! “Sonra”). The independent reference is `scripts/fixtures/map_survey_cases.py`.

use kentos_contracts::{
    AntennaReference, RinexFacts, RinexHeader, SurveyAntenna, SurveyReceiver, TimeSystem, survey_time,
};

use crate::text;

const VERSION_TYPE: &str = "RINEX VERSION / TYPE";

/// The characters `from`..`to` of a line (fewer at its end), trimmed.
fn cols(line: &str, from: usize, to: usize) -> String {
    line.chars().skip(from).take(to.saturating_sub(from)).collect::<String>().trim().to_owned()
}

fn label(line: &str) -> String {
    cols(line, 60, 80)
}

fn lines(bytes: &[u8]) -> Vec<String> {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc);
    body.split('\n').map(|l| l.trim_end_matches('\r').to_owned()).collect()
}

/// Whether the file is a RINEX file: its first line's label says so.
pub fn is_rinex(bytes: &[u8]) -> bool {
    lines(bytes).first().is_some_and(|l| label(l) == VERSION_TYPE)
}

fn number(line: &str, from: usize, to: usize, n: usize, what: &str) -> Result<f64, String> {
    let t = cols(line, from, to);
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("Satır {n}: {what} “{t}” sayı değil."))
}

fn some(t: String) -> Option<String> {
    (!t.is_empty()).then_some(t)
}

/// A `TIME OF FIRST OBS` / `TIME OF LAST OBS` line: the time as `YYYY-MM-DDThh:mm:ss.fffffffZ` and its system.
fn time(line: &str, n: usize) -> Result<(String, Option<TimeSystem>), String> {
    let bad = || format!("Satır {n}: gözlem zamanı okunmuyor.");
    let int = |from: usize| cols(line, from, from + 6).parse::<u32>().map_err(|_| bad());
    let (y, mo, d, h, mi) = (int(0)?, int(6)?, int(12)?, int(18)?, int(24)?);
    let sec = cols(line, 30, 43);
    let (whole, fraction) = sec.split_once('.').unwrap_or((sec.as_str(), ""));
    if whole.is_empty() || whole.len() > 2 || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let sec = if fraction.is_empty() { format!("{whole:0>2}") } else { format!("{whole:0>2}.{fraction}") };
    let text = format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{sec}Z");
    if survey_time(&text).is_none() {
        return Err(bad());
    }
    Ok((text, TimeSystem::from_rinex(&cols(line, 48, 51))))
}

/// Reads a RINEX observation file's header; why it cannot, in words.
pub fn read_header(bytes: &[u8]) -> Result<RinexHeader, String> {
    let all = lines(bytes);
    let first = all.first().map(String::as_str).unwrap_or_default();
    if label(first) != VERSION_TYPE {
        return Err("Dosya RINEX değil: ilk satırda “RINEX VERSION / TYPE” yok.".into());
    }
    let kind = cols(first, 20, 21);
    if kind != "O" {
        return Err(format!("RINEX dosyası gözlem dosyası değil (tür “{kind}”); oturum gözlem dosyasından okunur."));
    }
    let version = cols(first, 0, 9);
    if version.parse::<f64>().is_err() {
        return Err(format!("Satır 1: RINEX sürümü “{version}” sayı değil."));
    }
    let mut out = RinexHeader { facts: RinexFacts { version, ..RinexFacts::default() }, ..RinexHeader::default() };
    let mut ended = false;
    for (i, line) in all.iter().enumerate().skip(1) {
        let n = i + 1;
        match label(line).as_str() {
            "END OF HEADER" => {
                ended = true;
                break;
            }
            "MARKER NAME" => out.marker = some(cols(line, 0, 60)),
            "MARKER NUMBER" => out.facts.marker_number = some(cols(line, 0, 20)),
            "REC # / TYPE / VERS" => {
                out.receiver = some(cols(line, 20, 40)).map(|kind| SurveyReceiver {
                    kind,
                    serial: some(cols(line, 0, 20)),
                    firmware: some(cols(line, 40, 60)),
                });
            }
            "ANT # / TYPE" => {
                let a = out.antenna.get_or_insert_with(|| SurveyAntenna {
                    kind: None,
                    serial: None,
                    height: 0.0,
                    reference: AntennaReference::Arp,
                    radius: None,
                    offset: None,
                    east: None,
                    north: None,
                });
                a.serial = some(cols(line, 0, 20));
                a.kind = some(cols(line, 20, 40));
            }
            "ANTENNA: DELTA H/E/N" => {
                let h = number(line, 0, 14, n, "anten yüksekliği")?;
                let e = number(line, 14, 28, n, "doğu dış merkezliği")?;
                let no = number(line, 28, 42, n, "kuzey dış merkezliği")?;
                let a = out.antenna.get_or_insert_with(|| SurveyAntenna {
                    kind: None,
                    serial: None,
                    height: 0.0,
                    reference: AntennaReference::Arp,
                    radius: None,
                    offset: None,
                    east: None,
                    north: None,
                });
                a.height = h;
                a.east = (e != 0.0).then_some(e);
                a.north = (no != 0.0).then_some(no);
            }
            "APPROX POSITION XYZ" => {
                out.facts.approximate = Some([
                    number(line, 0, 14, n, "yaklaşık X")?,
                    number(line, 14, 28, n, "yaklaşık Y")?,
                    number(line, 28, 42, n, "yaklaşık Z")?,
                ]);
            }
            "INTERVAL" => out.facts.interval = Some(number(line, 0, 10, n, "aralık")?),
            "TIME OF FIRST OBS" => {
                let (t, system) = time(line, n)?;
                out.start = Some(t);
                out.time_system = system;
            }
            "TIME OF LAST OBS" => out.end = Some(time(line, n)?.0),
            _ => {}
        }
    }
    if !ended {
        return Err("RINEX başlığının sonu (END OF HEADER) yok; dosya kesilmiş olabilir.".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(body: &str, label: &str) -> String {
        format!("{body:<60}{label:<20}\n")
    }

    #[test]
    fn a_rinex_3_header_fills_a_session() {
        let mut f = String::new();
        f += &line("     3.05           OBSERVATION DATA    M", "RINEX VERSION / TYPE");
        f += &line("N1", "MARKER NAME");
        f += &line("5012345", "MARKER NUMBER");
        f += &line("5123K40001          TRIMBLE R10         5.52", "REC # / TYPE / VERS");
        f += &line("1441112222          TRM57971.00     NONE", "ANT # / TYPE");
        f += &line("        1.5320        0.0000        0.0000", "ANTENNA: DELTA H/E/N");
        f += &line("  2026    11     3     9    30    0.0000000     GPS", "TIME OF FIRST OBS");
        f += &line("  2026    11     3    10    30    0.0000000     GPS", "TIME OF LAST OBS");
        f += &line("", "END OF HEADER");
        let h = read_header(f.as_bytes()).expect("a header");
        assert!(is_rinex(f.as_bytes()));
        assert_eq!(h.marker.as_deref(), Some("N1"));
        assert_eq!(h.facts.version, "3.05");
        let a = h.antenna.expect("an antenna");
        assert_eq!((a.kind.as_deref(), a.height, a.east), (Some("TRM57971.00     NONE"), 1.532, None));
        assert_eq!(h.start.as_deref(), Some("2026-11-03T09:30:00.0000000Z"));
        assert_eq!(h.time_system, Some(TimeSystem::Gps));
        assert_eq!(h.receiver.map(|r| r.kind), Some("TRIMBLE R10".into()));
    }

    #[test]
    fn a_navigation_file_and_a_cut_header_are_refused() {
        let nav = line("     3.05           NAVIGATION DATA     G", "RINEX VERSION / TYPE");
        assert!(read_header(nav.as_bytes()).is_err());
        let cut = line("     2.11           OBSERVATION DATA    G", "RINEX VERSION / TYPE") + &line("N1", "MARKER NAME");
        assert!(read_header(cut.as_bytes()).unwrap_err().contains("END OF HEADER"));
    }
}
