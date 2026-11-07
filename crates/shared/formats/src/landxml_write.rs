//! LandXML 1.2 written (plan/road.md Y10.2, docs/adr/0908; ROAD-IO-05): a road project's TIN surfaces, horizontal
//! alignments (lines, circular curves and clothoids, with their km equations) and vertical alignments (PVIs with
//! symmetric, asymmetric and circular curves), as the schema (`http://www.landxml.org/schema/LandXML-1.2`) defines
//! them. A place is written “northing easting” as the schema says; lengths in metres, directions and angles in decimal
//! degrees (a direction clockwise from north). What the caller gives is already the geometry: this only writes it.

/// A TIN surface: its corners (x east, y north, z) and its triangles (indices into the corners).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WSurface {
    pub name: String,
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// A horizontal element's geometry; places (x east, y north).
#[derive(Clone, Debug, PartialEq)]
pub enum WGeom {
    Line {
        start: [f64; 2],
        end: [f64; 2],
    },
    /// A circular curve; its PI none where it turns half a circle or more.
    Curve {
        start: [f64; 2],
        end: [f64; 2],
        center: [f64; 2],
        pi: Option<[f64; 2]>,
        radius: f64,
        clockwise: bool,
    },
    /// A clothoid; a radius none at a tangent end.
    Spiral {
        start: [f64; 2],
        end: [f64; 2],
        pi: [f64; 2],
        radius_start: Option<f64>,
        radius_end: Option<f64>,
        clockwise: bool,
    },
}

/// An element of an alignment: its geometry, its internal station, length and directions (radians, from east
/// counterclockwise, as KentOS keeps them; written from north clockwise).
#[derive(Clone, Debug, PartialEq)]
pub struct WElement {
    pub geom: WGeom,
    pub station: f64,
    pub length: f64,
    pub dir_start: f64,
    pub dir_end: f64,
}

/// A km equation: its internal station, the km back and ahead, and whether the km increases.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WEquation {
    pub internal: f64,
    pub back: f64,
    pub ahead: f64,
    pub increasing: bool,
}

/// A PVI's curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WCurve {
    None,
    Parabola { length: f64 },
    Unsymmetric { length_in: f64, length_out: f64 },
    Circle { length: f64, radius: f64 },
}

/// A PVI: its station (internal), elevation and curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WPvi {
    pub station: f64,
    pub elevation: f64,
    pub curve: WCurve,
}

/// A vertical alignment of an alignment.
#[derive(Clone, Debug, PartialEq)]
pub struct WProfile {
    pub name: String,
    pub pvis: Vec<WPvi>,
}

/// A horizontal alignment: its name, its km at the start, its length, elements, equations and profiles.
#[derive(Clone, Debug, PartialEq)]
pub struct WAlignment {
    pub name: String,
    pub sta_start: f64,
    pub length: f64,
    pub elements: Vec<WElement>,
    pub equations: Vec<WEquation>,
    pub profiles: Vec<WProfile>,
    /// Its design speeds over stations, km/h (docs/adr/0988): `Roadways/Roadway/Speeds/DesignSpeed`.
    pub speeds: Vec<WSpeed>,
}

/// A design speed from `sta_start` to `sta_end` (stations), km/h.
#[derive(Clone, Debug, PartialEq)]
pub struct WSpeed {
    pub sta_start: f64,
    pub sta_end: f64,
    pub speed: f64,
}

/// A feature line (`PlanFeatures/PlanFeature`, a string of a string road): its name and its places (x, y, z).
#[derive(Clone, Debug, PartialEq)]
pub struct WFeature {
    pub name: String,
    pub points: Vec<[f64; 3]>,
}

/// The file's `CoordinateSystem` (docs/adr/1031): the horizontal system by its EPSG name and code (`name`,
/// `epsgCode`, `horizontalDatum`) or a system of the project's own by its name (`horizontalCoordinateSystemName`, as
/// buildingSMART's LandXML 1.2 profile writes a local one), the heights' system (`verticalCoordinateSystemName`,
/// `verticalDatum`) and in words what the heights are (`desc`). Each attribute only when known.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WCoordinateSystem {
    pub name: Option<String>,
    pub epsg_code: Option<u32>,
    pub horizontal_datum: Option<String>,
    pub horizontal_name: Option<String>,
    pub vertical_datum: Option<String>,
    pub vertical_name: Option<String>,
    pub desc: Option<String>,
}

/// What a file holds, and when and by what it is written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WLandXml {
    /// `YYYY-MM-DD` and `hh:mm:ss`.
    pub date: String,
    pub time: String,
    pub application: String,
    pub version: String,
    pub project: String,
    pub surfaces: Vec<WSurface>,
    pub alignments: Vec<WAlignment>,
    /// The feature lines, under one `PlanFeatures` named `features_name`.
    pub features: Vec<WFeature>,
    pub features_name: String,
    /// Absent: the file says no system (as before docs/adr/1031).
    pub coordinate_system: Option<WCoordinateSystem>,
}

fn escape(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// A number as the shortest text that reads back to it.
fn n(v: f64) -> String {
    if v == 0.0 { "0".into() } else { format!("{v}") }
}

/// A place, “northing easting”.
fn ne(p: [f64; 2]) -> String {
    format!("{} {}", n(p[1]), n(p[0]))
}

/// A direction (radians from east, counterclockwise) in decimal degrees from north, clockwise, in [0, 360).
pub fn north_degrees(theta: f64) -> f64 {
    let d = (90.0 - theta.to_degrees()).rem_euclid(360.0);
    if d >= 360.0 { 0.0 } else { d }
}

fn radius(r: Option<f64>) -> String {
    r.map_or_else(|| "INF".into(), n)
}

/// The file's text.
pub fn write(file: &WLandXml) -> String {
    let mut o = String::new();
    o.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    o.push_str(&format!(
        "<LandXML xmlns=\"http://www.landxml.org/schema/LandXML-1.2\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://www.landxml.org/schema/LandXML-1.2 http://www.landxml.org/schema/LandXML-1.2/LandXML-1.2.xsd\" \
         date=\"{}\" time=\"{}\" version=\"1.2\" language=\"Turkish\" readOnly=\"false\">\n",
        escape(&file.date),
        escape(&file.time)
    ));
    // The velocity's unit only where speeds are written (docs/adr/0988): a file without them stays as it was.
    let velocity = if file.alignments.iter().any(|a| !a.speeds.is_empty()) { " velocityUnit=\"kilometersPerHour\"" } else { "" };
    o.push_str(&format!(
        "  <Units>\n    <Metric areaUnit=\"squareMeter\" linearUnit=\"meter\" volumeUnit=\"cubicMeter\" temperatureUnit=\"celsius\" \
         pressureUnit=\"milliBars\" angularUnit=\"decimal degrees\" directionUnit=\"decimal degrees\"{velocity}/>\n  </Units>\n",
    ));
    if let Some(c) = &file.coordinate_system {
        let epsg = c.epsg_code.map(|e| e.to_string());
        o.push_str("  <CoordinateSystem");
        for (key, value) in [
            ("name", c.name.as_ref()),
            ("epsgCode", epsg.as_ref()),
            ("horizontalDatum", c.horizontal_datum.as_ref()),
            ("horizontalCoordinateSystemName", c.horizontal_name.as_ref()),
            ("verticalDatum", c.vertical_datum.as_ref()),
            ("verticalCoordinateSystemName", c.vertical_name.as_ref()),
            ("desc", c.desc.as_ref()),
        ] {
            if let Some(v) = value {
                o.push_str(&format!(" {key}=\"{}\"", escape(v)));
            }
        }
        o.push_str("/>\n");
    }
    o.push_str(&format!("  <Project name=\"{}\"/>\n", escape(&file.project)));
    o.push_str(&format!(
        "  <Application name=\"{}\" manufacturer=\"KentOS\" version=\"{}\"/>\n",
        escape(&file.application),
        escape(&file.version)
    ));
    if !file.surfaces.is_empty() {
        o.push_str("  <Surfaces>\n");
        for s in &file.surfaces {
            o.push_str(&format!("    <Surface name=\"{}\">\n      <Definition surfType=\"TIN\">\n        <Pnts>\n", escape(&s.name)));
            for (i, v) in s.vertices.iter().enumerate() {
                o.push_str(&format!("          <P id=\"{}\">{} {} {}</P>\n", i + 1, n(v[1]), n(v[0]), n(v[2])));
            }
            o.push_str("        </Pnts>\n        <Faces>\n");
            for t in &s.triangles {
                o.push_str(&format!("          <F>{} {} {}</F>\n", t[0] + 1, t[1] + 1, t[2] + 1));
            }
            o.push_str("        </Faces>\n      </Definition>\n    </Surface>\n");
        }
        o.push_str("  </Surfaces>\n");
    }
    if !file.alignments.is_empty() {
        o.push_str("  <Alignments>\n");
        for a in &file.alignments {
            o.push_str(&format!(
                "    <Alignment name=\"{}\" length=\"{}\" staStart=\"{}\">\n      <CoordGeom>\n",
                escape(&a.name),
                n(a.length),
                n(a.sta_start)
            ));
            for e in &a.elements {
                let head = format!(
                    "staStart=\"{}\" length=\"{}\" dirStart=\"{}\" dirEnd=\"{}\"",
                    n(e.station),
                    n(e.length),
                    n(north_degrees(e.dir_start)),
                    n(north_degrees(e.dir_end))
                );
                match &e.geom {
                    WGeom::Line { start, end } => o.push_str(&format!(
                        "        <Line staStart=\"{}\" length=\"{}\" dir=\"{}\">\n          <Start>{}</Start>\n          <End>{}</End>\n        </Line>\n",
                        n(e.station),
                        n(e.length),
                        n(north_degrees(e.dir_start)),
                        ne(*start),
                        ne(*end)
                    )),
                    WGeom::Curve { start, end, center, pi, radius, clockwise } => {
                        o.push_str(&format!(
                            "        <Curve rot=\"{}\" crvType=\"arc\" radius=\"{}\" {head}>\n          <Start>{}</Start>\n          <Center>{}</Center>\n          <End>{}</End>\n",
                            if *clockwise { "cw" } else { "ccw" },
                            n(*radius),
                            ne(*start),
                            ne(*center),
                            ne(*end)
                        ));
                        if let Some(p) = pi {
                            o.push_str(&format!("          <PI>{}</PI>\n", ne(*p)));
                        }
                        o.push_str("        </Curve>\n");
                    }
                    WGeom::Spiral { start, end, pi, radius_start, radius_end, clockwise } => o.push_str(&format!(
                        "        <Spiral rot=\"{}\" spiType=\"clothoid\" radiusStart=\"{}\" radiusEnd=\"{}\" {head}>\n          <Start>{}</Start>\n          <PI>{}</PI>\n          <End>{}</End>\n        </Spiral>\n",
                        if *clockwise { "cw" } else { "ccw" },
                        radius(*radius_start),
                        radius(*radius_end),
                        ne(*start),
                        ne(*pi),
                        ne(*end)
                    )),
                }
            }
            o.push_str("      </CoordGeom>\n");
            for q in &a.equations {
                o.push_str(&format!(
                    "      <StaEquation staInternal=\"{}\" staBack=\"{}\" staAhead=\"{}\" staIncrement=\"{}\"/>\n",
                    n(q.internal),
                    n(q.back),
                    n(q.ahead),
                    if q.increasing { "increasing" } else { "decreasing" }
                ));
            }
            for p in &a.profiles {
                o.push_str(&format!("      <Profile name=\"{0}\">\n        <ProfAlign name=\"{0}\">\n", escape(&p.name)));
                for v in &p.pvis {
                    let at = format!("{} {}", n(v.station), n(v.elevation));
                    o.push_str(&match v.curve {
                        WCurve::None => format!("          <PVI>{at}</PVI>\n"),
                        WCurve::Parabola { length } => format!("          <ParaCurve length=\"{}\">{at}</ParaCurve>\n", n(length)),
                        WCurve::Unsymmetric { length_in, length_out } => format!(
                            "          <UnsymParaCurve lengthIn=\"{}\" lengthOut=\"{}\">{at}</UnsymParaCurve>\n",
                            n(length_in),
                            n(length_out)
                        ),
                        WCurve::Circle { length, radius } => format!(
                            "          <CircCurve length=\"{}\" radius=\"{}\">{at}</CircCurve>\n",
                            n(length),
                            n(radius)
                        ),
                    });
                }
                o.push_str("        </ProfAlign>\n      </Profile>\n");
            }
            o.push_str("    </Alignment>\n");
        }
        o.push_str("  </Alignments>\n");
    }
    // Docs/adr/0988: each alignment's design speeds as a roadway's (LandXML 1.2, FHWA IHSDM), named after it.
    if file.alignments.iter().any(|a| !a.speeds.is_empty()) {
        o.push_str("  <Roadways>\n");
        for a in file.alignments.iter().filter(|a| !a.speeds.is_empty()) {
            o.push_str(&format!(
                "    <Roadway name=\"{0}\" alignmentRefs=\"{0}\">\n      <Speeds>\n",
                escape(&a.name)
            ));
            for s in &a.speeds {
                o.push_str(&format!(
                    "        <DesignSpeed staStart=\"{}\" staEnd=\"{}\" speed=\"{}\"/>\n",
                    n(s.sta_start),
                    n(s.sta_end),
                    n(s.speed)
                ));
            }
            o.push_str("      </Speeds>\n    </Roadway>\n");
        }
        o.push_str("  </Roadways>\n");
    }
    if !file.features.is_empty() {
        o.push_str(&format!("  <PlanFeatures name=\"{}\">\n", escape(&file.features_name)));
        for f in &file.features {
            o.push_str(&format!("    <PlanFeature name=\"{}\">\n      <CoordGeom>\n", escape(&f.name)));
            for w in f.points.windows(2) {
                let p = |q: [f64; 3]| format!("{} {} {}", n(q[1]), n(q[0]), n(q[2]));
                o.push_str(&format!("        <Line><Start>{}</Start><End>{}</End></Line>\n", p(w[0]), p(w[1])));
            }
            o.push_str("      </CoordGeom>\n    </PlanFeature>\n");
        }
        o.push_str("  </PlanFeatures>\n");
    }
    o.push_str("</LandXML>\n");
    o
}
