//! A Shapefile's .prj: its coordinate system as WKT 1 (the ESRI or the OGC
//! flavour). The reader recognises what Turkish survey data is in: the
//! TUREF (ITRF96) 3° TM zones, ED50's TM and UTM zones, WGS 84 UTM and the
//! geographic WGS 84 and TUREF; by an EPSG authority when the file gives
//! one, else by datum and projection parameters, compared exactly. Anything
//! else is shown as written and the app asks (CLAUDE.md §5): a name is
//! never guessed into a code, and nothing is transformed.

/// What a .prj says.
#[derive(Clone, Debug, PartialEq)]
pub struct Prj {
    /// The EPSG code, when recognised.
    pub srid: Option<u32>,
    /// The system's name as the file writes it ("TUREF_TM36").
    pub name: String,
}

/// A .prj larger than this is not a coordinate system (and is not parsed).
const MAX_LEN: usize = 64 * 1024;
/// Nesting of a WKT 1 definition (a projected system is three deep).
const MAX_DEPTH: u32 = 16;

#[derive(Debug)]
enum Item {
    Node(Node),
    Str(String),
    Num(f64),
    Word,
}

#[derive(Debug)]
struct Node {
    name: String,
    items: Vec<Item>,
}

impl Node {
    fn child(&self, name: &str) -> Option<&Node> {
        self.items.iter().find_map(|i| match i {
            Item::Node(n) if n.name.eq_ignore_ascii_case(name) => Some(n),
            _ => None,
        })
    }

    fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.items.iter().filter_map(move |i| match i {
            Item::Node(n) if n.name.eq_ignore_ascii_case(name) => Some(n),
            _ => None,
        })
    }

    /// The first quoted text (a node's name).
    fn text(&self) -> Option<&str> {
        self.items.iter().find_map(|i| match i {
            Item::Str(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// The first number.
    fn number(&self) -> Option<f64> {
        self.items.iter().find_map(|i| match i {
            Item::Num(v) => Some(*v),
            _ => None,
        })
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: u32,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.b.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn word(&mut self) -> Option<String> {
        self.ws();
        let start = self.i;
        while self
            .b
            .get(self.i)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            self.i += 1;
        }
        (self.i > start).then(|| String::from_utf8_lossy(&self.b[start..self.i]).into_owned())
    }

    fn node(&mut self) -> Option<Node> {
        let name = self.word()?;
        self.ws();
        let close = match self.b.get(self.i) {
            Some(b'[') => b']',
            Some(b'(') => b')',
            _ => return None,
        };
        self.i += 1;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return None;
        }
        let mut items = Vec::new();
        loop {
            self.ws();
            match *self.b.get(self.i)? {
                c if c == close => {
                    self.i += 1;
                    break;
                }
                b',' => self.i += 1,
                b'"' => {
                    self.i += 1;
                    let mut s = Vec::new();
                    loop {
                        match *self.b.get(self.i)? {
                            b'"' if self.b.get(self.i + 1) == Some(&b'"') => {
                                s.push(b'"');
                                self.i += 2;
                            }
                            b'"' => {
                                self.i += 1;
                                break;
                            }
                            c => {
                                s.push(c);
                                self.i += 1;
                            }
                        }
                    }
                    items.push(Item::Str(String::from_utf8_lossy(&s).into_owned()));
                }
                b'0'..=b'9' | b'-' | b'+' | b'.' => {
                    let start = self.i;
                    while self.b.get(self.i).is_some_and(|c| {
                        c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E')
                    }) {
                        self.i += 1;
                    }
                    let t = std::str::from_utf8(&self.b[start..self.i]).ok()?;
                    items.push(Item::Num(t.parse().ok()?));
                }
                c if c.is_ascii_alphabetic() => {
                    let back = self.i;
                    let _ = self.word()?;
                    self.ws();
                    if matches!(self.b.get(self.i), Some(b'[' | b'(')) {
                        self.i = back;
                        items.push(Item::Node(self.node()?));
                    } else {
                        items.push(Item::Word);
                    }
                }
                _ => return None,
            }
        }
        self.depth -= 1;
        Some(Node { name, items })
    }
}

/// A name compared as the rules compare names: lower case, ASCII letters and digits only.
fn key(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Datum {
    Turef,
    Ed50,
    Wgs84,
}

fn datum(geogcs: &Node) -> Option<Datum> {
    let k = key(geogcs.child("DATUM")?.text()?);
    if [
        "turef",
        "turkishnationalreferenceframe",
        "itrf96",
        "itrf1996",
    ]
    .iter()
    .any(|d| k.contains(d))
    {
        Some(Datum::Turef)
    } else if ["european1950", "europeandatum1950", "ed50"]
        .iter()
        .any(|d| k.contains(d))
    {
        Some(Datum::Ed50)
    } else if k.contains("wgs1984") || k.contains("wgs84") {
        Some(Datum::Wgs84)
    } else {
        None
    }
}

/// An EPSG code named by the outermost node itself (`AUTHORITY["EPSG","5256"]`).
fn authority(root: &Node) -> Option<u32> {
    root.children("AUTHORITY")
        .find_map(|a| match a.items.as_slice() {
            [Item::Str(org), Item::Str(code)]
                if org == "EPSG"
                    && !code.is_empty()
                    && code.bytes().all(|b| b.is_ascii_digit()) =>
            {
                code.parse().ok()
            }
            _ => None,
        })
}

/// A Transverse Mercator system of the known zones.
fn transverse_mercator(root: &Node, d: Datum) -> Option<u32> {
    if !key(root.child("PROJECTION")?.text()?).contains("transversemercator") {
        return None;
    }
    if root.child("UNIT")?.number()? != 1.0 {
        return None;
    }
    // Every parameter is a name and a number; the last of a repeated name counts.
    let mut params: Vec<(String, f64)> = Vec::new();
    for p in root.children("PARAMETER") {
        match p.items.as_slice() {
            [Item::Str(name), Item::Num(v), ..] => params.push((key(name), *v)),
            _ => return None,
        }
    }
    let param = |name: &str| -> Option<f64> {
        params
            .iter()
            .rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| *v)
    };
    let fe = param("falseeasting").unwrap_or(0.0);
    let fnorth = param("falsenorthing").unwrap_or(0.0);
    let lat0 = param("latitudeoforigin").unwrap_or(0.0);
    let k0 = param("scalefactor").unwrap_or(1.0);
    let cm = param("centralmeridian").unwrap_or(0.0);
    if fe != 500000.0 || fnorth != 0.0 || lat0 != 0.0 {
        return None;
    }
    let three = [27.0, 30.0, 33.0, 36.0, 39.0, 42.0, 45.0];
    let six = [27.0, 33.0, 39.0, 45.0];
    let zone = |step: f64| ((cm - 27.0) / step) as u32;
    match d {
        Datum::Turef if k0 == 1.0 && three.contains(&cm) => Some(5253 + zone(3.0)),
        Datum::Ed50 if k0 == 1.0 && three.contains(&cm) => Some(2319 + zone(3.0)),
        Datum::Ed50 if k0 == 0.9996 && six.contains(&cm) => Some(23035 + zone(6.0)),
        Datum::Wgs84 if k0 == 0.9996 && six.contains(&cm) => Some(32635 + zone(6.0)),
        _ => None,
    }
}

/// Reads a .prj's text.
pub fn read(bytes: &[u8]) -> Prj {
    let text = String::from_utf8_lossy(bytes);
    let shown: String = text.trim().chars().take(80).collect();
    if bytes.len() > MAX_LEN {
        return Prj {
            srid: None,
            name: shown,
        };
    }
    let mut p = Parser {
        b: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    let Some(root) = p.node() else {
        return Prj {
            srid: None,
            name: shown,
        };
    };
    p.ws();
    if p.i < p.b.len() {
        // Something after the definition: not one coordinate system.
        return Prj {
            srid: None,
            name: shown,
        };
    }
    let name = root.text().map(str::to_string).unwrap_or(shown);
    let projected = root.name.eq_ignore_ascii_case("PROJCS");
    let geographic = root.name.eq_ignore_ascii_case("GEOGCS");
    if !projected && !geographic {
        return Prj { srid: None, name };
    }
    let srid = authority(&root).or_else(|| {
        if projected {
            let d = datum(root.child("GEOGCS")?)?;
            transverse_mercator(&root, d)
        } else {
            match datum(&root)? {
                Datum::Wgs84 => Some(4326),
                Datum::Turef => Some(5252),
                Datum::Ed50 => None,
            }
        }
    });
    Prj { srid, name }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TM36_ESRI: &str = r#"PROJCS["TUREF_TM36",GEOGCS["GCS_TUREF",DATUM["D_Turkish_National_Reference_Frame",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",500000.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",36.0],PARAMETER["Scale_Factor",1.0],PARAMETER["Latitude_Of_Origin",0.0],UNIT["Meter",1.0]]"#;

    #[test]
    fn recognises_the_zones_of_turkish_survey_data() {
        let cases = [
            (TM36_ESRI.to_string(), Some(5256)),
            (TM36_ESRI.replace("36.0", "27.0"), Some(5253)),
            (TM36_ESRI.replace("36.0", "37.0"), None),
            (TM36_ESRI.replace("Turkish_National_Reference_Frame", "ITRF_1996"), Some(5256)),
            (TM36_ESRI.replace("D_Turkish_National_Reference_Frame", "D_European_1950"), Some(2322)),
            (
                TM36_ESRI
                    .replace("D_Turkish_National_Reference_Frame", "European_Datum_1950")
                    .replace("\"Scale_Factor\",1.0", "\"Scale_Factor\",0.9996")
                    .replace("36.0", "33.0"),
                Some(23036),
            ),
            (
                TM36_ESRI
                    .replace("D_Turkish_National_Reference_Frame", "WGS_1984")
                    .replace("\"Scale_Factor\",1.0", "\"Scale_Factor\",0.9996")
                    .replace("36.0", "39.0"),
                Some(32637),
            ),
            (TM36_ESRI.replace("UNIT[\"Meter\",1.0]", "UNIT[\"Foot_US\",0.3048006096012192]"), None),
            (TM36_ESRI.replace(",UNIT[\"Meter\",1.0]", ""), None),
            (TM36_ESRI.replace("500000.0", "0.0"), None),
            (
                r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#.into(),
                Some(4326),
            ),
            (
                r#"PROJCS["ITRF96 / TM30",GEOGCS["ITRF96",DATUM["International_Terrestrial_Reference_Frame_1996",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",30],PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AUTHORITY["EPSG","5254"]]"#.into(),
                Some(5254),
            ),
            ("LOCAL_CS[\"x\"]".into(), None),
            ("not a wkt".into(), None),
        ];
        for (wkt, srid) in cases {
            assert_eq!(read(wkt.as_bytes()).srid, srid, "{wkt}");
        }
        assert_eq!(read(TM36_ESRI.as_bytes()).name, "TUREF_TM36");
    }

    #[test]
    fn hostile_text_is_refused_quietly() {
        let deep = "PROJCS[".repeat(10_000);
        assert_eq!(read(deep.as_bytes()).srid, None);
        assert_eq!(read(b"PROJCS[\"a\",1e999999]").srid, None);
        assert_eq!(read(&[0xFF; 70_000]).srid, None);
        assert_eq!(read(b"PROJCS[\"unterminated").srid, None);
    }
}
