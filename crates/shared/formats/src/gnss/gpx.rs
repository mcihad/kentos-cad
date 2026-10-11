//! GPX 1.1 GNSS files (docs/adr/0169 §1, §6): waypoints, route points and
//! track points with their WGS 84 positions, heights, fix, satellites, HDOP,
//! name and time, read as the GPX 1.1 schema describes them; its routes and
//! tracks as lines of those points (docs/adr/0222 §6). Every point not read
//! is named with the reason. The independent reference is
//! `scripts/fixtures/gnss_gpx_cases.py` (fixtures/gnss/v1/gpx.json), read with
//! Python's expat. Values are their exact decimals rounded once to the
//! nearest float64 (CLAUDE.md §23).

use std::cmp::Ordering;

use kentos_contracts::{GnssLine, GnssPoint, GnssRead, LineError};
use roxmltree::Node;

use crate::field::exact::Exact;
use crate::text;
use crate::xml::{DEPTH, Rows, Unparsed, document};

/// What is said of a point not read; `{line}`, `{depth}`, `{root}`, `{lat}`,
/// `{lon}`, `{what}`, `{data}` and `{count}` are filled in.
pub const XML: &str = "Satır 1: dosya XML olarak okunamadı; GNSS noktaları okunmadı.";
pub const DEEP: &str =
    "Satır 1: XML öğeleri {depth} düzeyden derin iç içe; GNSS noktaları okunmadı.";
pub const ROOT: &str =
    "Satır {line}: XML dosyası GPX değil (kök öğe {root}); GNSS noktaları okunmadı.";
pub const POSITION: &str =
    "Satır {line}: konum (lat “{lat}”, lon “{lon}”) okunmuyor; nokta okunmadı.";
pub const VALUE: &str = "Satır {line}: {what} “{data}” okunmuyor; nokta okunmadı.";
pub const NOFIX: &str = "Satır {line}: konumu olmayan noktalar ({count} nokta, fix none) okunmadı.";

/// A node's first child element of a name.
fn child<'a, 'i>(node: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    node.children()
        .find(|c| c.is_element() && c.tag_name().name() == tag)
}

/// An element's text, trimmed.
fn text_of(node: Node<'_, '_>) -> String {
    node.text().unwrap_or_default().trim().to_owned()
}

/// Why a point is not read: what and the value's text.
struct Unread(&'static str, String);

/// A decimal child: none without one; not read when it is not one.
fn decimal(node: Node<'_, '_>, tag: &'static str) -> Result<Option<Exact>, Unread> {
    let Some(c) = child(node, tag) else {
        return Ok(None);
    };
    let t = text_of(c);
    Exact::decimal(&t).map(Some).ok_or(Unread(tag, t))
}

/// A point's height, geoid height, HDOP and satellites, in that order; the
/// first that is not one leaves the point out.
type Values = (Option<Exact>, Option<Exact>, Option<Exact>, Option<u32>);

fn values(n: Node<'_, '_>) -> Result<Values, Unread> {
    let ele = decimal(n, "ele")?;
    let geoid = decimal(n, "geoidheight")?;
    let hdop = decimal(n, "hdop")?;
    let sat = match child(n, "sat") {
        Some(c) => {
            let t = text_of(c);
            let ok = !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
            Some(
                ok.then(|| t.parse::<u32>().ok())
                    .flatten()
                    .ok_or(Unread("sat", t))?,
            )
        }
        None => None,
    };
    Ok((ele, geoid, hdop, sat))
}

/// A route or a track: its kind (`rte`, `trk`), its element and its parts (a track's `trkseg`s).
struct Run<'a, 'i> {
    kind: &'static str,
    node: Node<'a, 'i>,
    parts: usize,
}

/// A point to read: its kind, its element and, for a route's or a track's, its run and part.
type Place<'a, 'i> = (&'static str, Node<'a, 'i>, Option<(usize, usize)>);

/// The points in the file's order: wpt, rte/rtept, trk/trkseg/trkpt; and the routes and tracks.
fn points<'a, 'i>(root: Node<'a, 'i>) -> (Vec<Place<'a, 'i>>, Vec<Run<'a, 'i>>) {
    let mut out = Vec::new();
    let mut runs = Vec::new();
    let elements = |n: Node<'a, 'i>| n.children().filter(Node::is_element);
    for c in elements(root) {
        match c.tag_name().name() {
            "wpt" => out.push(("wpt", c, None)),
            "rte" => {
                let run = runs.len();
                runs.push(Run {
                    kind: "rte",
                    node: c,
                    parts: 1,
                });
                out.extend(
                    elements(c)
                        .filter(|p| p.tag_name().name() == "rtept")
                        .map(|p| ("rtept", p, Some((run, 0)))),
                );
            }
            "trk" => {
                let run = runs.len();
                let segments: Vec<Node<'a, 'i>> = elements(c)
                    .filter(|s| s.tag_name().name() == "trkseg")
                    .collect();
                runs.push(Run {
                    kind: "trk",
                    node: c,
                    parts: segments.len(),
                });
                for (part, seg) in segments.into_iter().enumerate() {
                    out.extend(
                        elements(seg)
                            .filter(|p| p.tag_name().name() == "trkpt")
                            .map(|p| ("trkpt", p, Some((run, part)))),
                    );
                }
            }
            _ => {}
        }
    }
    (out, runs)
}

/// GPX's fix as the window names it.
fn fix_name(fix: &str) -> String {
    match fix {
        "2d" => "2B",
        "3d" => "3B",
        "dgps" => "DGPS",
        "pps" => "PPS",
        other => other,
    }
    .to_owned()
}

/// A problem: `{line}`, then the placeholders in order.
fn said(text: &str, line: u32, fill: &[(&str, &str)]) -> LineError {
    let mut message = text.replace("{line}", &line.to_string());
    for (k, v) in fill {
        message = message.replace(k, v);
    }
    LineError { line, message }
}

/// Reads a GPX 1.1 file.
pub fn read(bytes: &[u8]) -> GnssRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc);
    let mut read = GnssRead {
        format: "gpx".to_owned(),
        encoding: enc.label().to_owned(),
        points: Vec::new(),
        problems: Vec::new(),
        lines: Vec::new(),
    };
    let doc = match document(&body) {
        Ok(doc) => doc,
        Err(Unparsed::NotXml) => {
            read.problems.push(said(XML, 1, &[]));
            return read;
        }
        Err(Unparsed::TooDeep) => {
            read.problems
                .push(said(DEEP, 1, &[("{depth}", &DEPTH.to_string())]));
            return read;
        }
    };
    let rows = Rows::new(doc.input_text());
    let line_of = |n: Node<'_, '_>| rows.row(n.range().start);
    let root = doc.root_element();
    if root.tag_name().name() != "gpx" {
        let line = line_of(root);
        read.problems
            .push(said(ROOT, line, &[("{root}", root.tag_name().name())]));
        return read;
    }
    let mut nofix: Option<(u32, usize)> = None;
    let (places, runs) = points(root);
    let mut lines: Vec<GnssLine> = runs
        .iter()
        .map(|r| GnssLine {
            kind: r.kind.to_owned(),
            name: child(r.node, "name").map(text_of).filter(|s| !s.is_empty()),
            parts: vec![Vec::new(); r.parts],
            line: line_of(r.node),
        })
        .collect();
    for (kind, n, run) in places {
        let line = line_of(n);
        let (lat_t, lon_t) = (
            n.attribute("lat").unwrap_or_default(),
            n.attribute("lon").unwrap_or_default(),
        );
        let (Some(lat), Some(lon)) = (Exact::decimal(lat_t), Exact::decimal(lon_t)) else {
            read.problems
                .push(said(POSITION, line, &[("{lat}", lat_t), ("{lon}", lon_t)]));
            continue;
        };
        if lat.size_cmp(90) == Ordering::Greater || lon.size_cmp(180) == Ordering::Greater {
            read.problems
                .push(said(POSITION, line, &[("{lat}", lat_t), ("{lon}", lon_t)]));
            continue;
        }
        let (ele, geoid, hdop, sat) = match values(n) {
            Ok(v) => v,
            Err(Unread(what, data)) => {
                read.problems
                    .push(said(VALUE, line, &[("{what}", what), ("{data}", &data)]));
                continue;
            }
        };
        let fix = child(n, "fix").map(text_of);
        if fix.as_deref() == Some("none") {
            nofix.get_or_insert((line, 0)).1 += 1;
            continue;
        }
        let name = child(n, "name").map(text_of).filter(|s| !s.is_empty());
        let time = child(n, "time").map(text_of).filter(|s| !s.is_empty());
        read.points.push(GnssPoint {
            kind: kind.to_owned(),
            name,
            lat: lat.value(),
            lon: lon.value(),
            height: ele.map(Exact::value),
            geoid: geoid.map(Exact::value),
            ellipsoidal: ele.zip(geoid).and_then(|(e, g)| e.sum(g)).map(Exact::value),
            time,
            fix: fix.filter(|f| !f.is_empty()).map(|f| fix_name(&f)),
            satellites: sat,
            hdop: hdop.map(Exact::value),
            line,
        });
        if let Some((run, part)) = run {
            let at = u32::try_from(read.points.len() - 1).unwrap_or(u32::MAX);
            lines[run].parts[part].push(at);
        }
    }
    // A line's parts are its points read; a part without one, and a line without a part, are not listed.
    for mut l in lines {
        l.parts.retain(|p| !p.is_empty());
        if !l.parts.is_empty() {
            read.lines.push(l);
        }
    }
    if let Some((line, count)) = nofix {
        read.problems
            .push(said(NOFIX, line, &[("{count}", &count.to_string())]));
        read.problems.sort_by_key(|p| p.line);
    }
    read
}
