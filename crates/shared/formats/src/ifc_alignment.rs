//! IFC 4.3 alignments written (plan/road.md Y10.3, docs/adr/0908 §8; ROAD-IO-06): an ISO 10303-21 file of the
//! `IFC4X3_ADD2` schema holding a road project's horizontal alignments and their vertical alignments, both as the
//! business logic (`IfcAlignmentHorizontal`/`IfcAlignmentVertical` and their `IfcAlignmentSegment`s with design
//! parameters) and as geometry (`IfcCompositeCurve` and `IfcGradientCurve` of `IfcCurveSegment`s over `IfcLine`,
//! `IfcCircle`, `IfcClothoid` and `IfcPolynomialCurve`), each layout ending in a zero-length segment. The mapping from
//! the design parameters to the curve segments is the one IfcOpenShell's alignment API makes (buildingSMART's concept
//! templates); the km at the start and the km equations are `IfcReferent`s with `Pset_Stationing`; the project's
//! system an `IfcMapConversion` to an `IfcProjectedCRS`. Lengths in metres, angles in radians.

/// A horizontal segment's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HKind {
    Line,
    Arc,
    Clothoid,
}

/// A horizontal segment: its start, its direction at the start and at the end (radians from x, counterclockwise),
/// its radii at the start and the end (positive turning counterclockwise, 0 infinite) and its length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HSeg {
    pub kind: HKind,
    pub start: [f64; 2],
    pub dir: f64,
    pub dir_end: f64,
    pub r0: f64,
    pub r1: f64,
    pub length: f64,
}

/// A vertical segment's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VKind {
    Grade,
    Parabola,
    Circle,
}

/// A vertical segment: its distance along the horizontal at the start, its horizontal length, its height and its
/// gradients at the start and the end, its radius (the parabola's constant or the circle's; positive where the gradient
/// grows, 0 for a grade).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VSeg {
    pub kind: VKind,
    pub dist: f64,
    pub length: f64,
    pub z0: f64,
    pub g0: f64,
    pub g1: f64,
    pub radius: f64,
}

/// A vertical alignment: its name, segments and end (distance along, height, gradient).
#[derive(Clone, Debug, PartialEq)]
pub struct Vertical {
    pub name: String,
    pub segments: Vec<VSeg>,
    pub end: [f64; 3],
}

/// A km equation: the distance along where it is, the km back and ahead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Equation {
    pub dist: f64,
    pub back: f64,
    pub ahead: f64,
}

/// An alignment: its name, horizontal segments and end (place and direction), its km at the start and equations, its
/// vertical alignments (the first on it, the others on child alignments).
#[derive(Clone, Debug, PartialEq)]
pub struct Alignment {
    pub name: String,
    pub horizontal: Vec<HSeg>,
    pub end: [f64; 2],
    pub end_dir: f64,
    pub start_km: f64,
    pub equations: Vec<Equation>,
    pub verticals: Vec<Vertical>,
}

/// The file: the project's name, its system (`EPSG:5254` and its words), when it is written, by what, its alignments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IfcFile {
    pub project: String,
    pub crs: Option<(String, String)>,
    /// `YYYY-MM-DDThh:mm:ss`.
    pub stamp: String,
    pub application: String,
    pub alignments: Vec<Alignment>,
}

/// A real as STEP writes it: always with a point.
fn real(v: f64) -> String {
    let v = if v == 0.0 { 0.0 } else { v };
    let t = format!("{v}");
    if t.contains('.') || t.contains('e') || t.contains("inf") || t.contains("NaN") { t } else { format!("{t}.") }
}

/// A string as STEP writes it: an apostrophe doubled, a backslash doubled, what is not printable ASCII as `\X2\…\X0\`.
fn text(s: &str) -> String {
    let mut out = String::from("'");
    let mut wide = String::new();
    let flush = |wide: &mut String, out: &mut String| {
        if !wide.is_empty() {
            out.push_str("\\X2\\");
            out.push_str(wide);
            out.push_str("\\X0\\");
            wide.clear();
        }
    };
    for c in s.chars() {
        if (' '..='~').contains(&c) {
            flush(&mut wide, &mut out);
            match c {
                '\'' => out.push_str("''"),
                '\\' => out.push_str("\\\\"),
                c => out.push(c),
            }
        } else {
            let mut units = [0u16; 2];
            for u in c.encode_utf16(&mut units) {
                wide.push_str(&format!("{u:04X}"));
            }
        }
    }
    flush(&mut wide, &mut out);
    out.push('\'');
    out
}

const GUID: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

/// An IFC GlobalId (22 characters) of a 128-bit number: the first character its top 2 bits, then 6 bits each.
pub fn guid(n: u128) -> String {
    let mut out = String::with_capacity(22);
    out.push(GUID[(n >> 126) as usize] as char);
    for i in (0..21).rev() {
        out.push(GUID[((n >> (6 * i)) & 63) as usize] as char);
    }
    out
}

/// FNV-1a over 128 bits: the GlobalIds are the same for the same file.
fn fnv(seed: &str, k: u64) -> u128 {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for b in seed.bytes().chain(k.to_le_bytes()) {
        h ^= b as u128;
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    h
}

struct Step {
    lines: Vec<String>,
    seed: String,
    ids: u64,
}

impl Step {
    fn add(&mut self, body: impl Into<String>) -> usize {
        self.lines.push(body.into());
        self.lines.len()
    }
    fn gid(&mut self) -> String {
        self.ids += 1;
        text(&guid(fnv(&self.seed, self.ids)))
    }
    fn point2(&mut self, p: [f64; 2]) -> usize {
        self.add(format!("IFCCARTESIANPOINT(({},{}))", real(p[0]), real(p[1])))
    }
    fn dir2(&mut self, d: [f64; 2]) -> usize {
        self.add(format!("IFCDIRECTION(({},{}))", real(d[0]), real(d[1])))
    }
    fn place2(&mut self, p: [f64; 2], d: [f64; 2]) -> usize {
        let (a, b) = (self.point2(p), self.dir2(d));
        self.add(format!("IFCAXIS2PLACEMENT2D(#{a},#{b})"))
    }
    fn origin2(&mut self) -> usize {
        self.place2([0.0, 0.0], [1.0, 0.0])
    }
    fn line(&mut self) -> usize {
        let (p, d) = (self.point2([0.0, 0.0]), self.dir2([1.0, 0.0]));
        let v = self.add(format!("IFCVECTOR(#{d},1.)"));
        self.add(format!("IFCLINE(#{p},#{v})"))
    }
    fn segment(&mut self, transition: &str, placement: usize, start: f64, length: f64, parent: usize) -> usize {
        self.add(format!(
            "IFCCURVESEGMENT(.{transition}.,#{placement},IFCLENGTHMEASURE({}),IFCLENGTHMEASURE({}),#{parent})",
            real(start),
            real(length)
        ))
    }
    fn refs(ids: &[usize]) -> String {
        ids.iter().map(|i| format!("#{i}")).collect::<Vec<_>>().join(",")
    }
}

fn curvature(r: f64) -> f64 {
    if r == 0.0 { 0.0 } else { 1.0 / r }
}

fn same_angle(a: f64, b: f64) -> bool {
    let d = (a - b).rem_euclid(std::f64::consts::TAU);
    d.min(std::f64::consts::TAU - d) < 1e-9
}

/// The transition from a segment to the next (IfcOpenShell's rule: the position within 1 mm, then the direction, then
/// the curvature); a layout's last segment, the zero-length one, `DISCONTINUOUS`.
fn transition(position: bool, gradient: bool, curvature: bool) -> &'static str {
    match (position, gradient, curvature) {
        (false, _, _) => "DISCONTINUOUS",
        (true, false, _) => "CONTINUOUS",
        (true, true, false) => "CONTSAMEGRADIENT",
        (true, true, true) => "CONTSAMEGRADIENTSAMECURVATURE",
    }
}

/// A horizontal segment's curve segment (IfcOpenShell's `_map_alignment_horizontal_segment`).
fn horizontal_curve(st: &mut Step, s: &HSeg, code: &str) -> usize {
    let placement = st.place2(s.start, [crate::math::cos(s.dir), crate::math::sin(s.dir)]);
    match s.kind {
        HKind::Line => {
            let parent = st.line();
            st.segment(code, placement, 0.0, s.length, parent)
        }
        HKind::Arc => {
            let pos = st.origin2();
            let parent = st.add(format!("IFCCIRCLE(#{pos},{})", real(s.r0.abs())));
            st.segment(code, placement, 0.0, s.length * s.r0.signum(), parent)
        }
        HKind::Clothoid => {
            let (r0, r1, l) = (s.r0, s.r1, s.length);
            let f = (if r1 == 0.0 { 0.0 } else { l / r1 }) - (if r0 == 0.0 { 0.0 } else { l / r0 });
            let a = l / f.abs().sqrt() * f.signum();
            let offset = if (r0.abs() < r1.abs() && r0 != 0.0) || r1 == 0.0 {
                -l - if r1 != 0.0 { l * r0 / (r1 - r0) } else { 0.0 }
            } else if r0 != 0.0 {
                l * r1 / (r0 - r1)
            } else {
                0.0
            };
            let pos = st.origin2();
            let parent = st.add(format!("IFCCLOTHOID(#{pos},{})", real(a)));
            st.segment(code, placement, offset, l, parent)
        }
    }
}

/// The polynomial's length over [0, L] (IfcOpenShell's `_polynomial_length`).
fn polynomial_length(b: f64, c: f64, l: f64) -> f64 {
    let (qa, qb, qc) = (4.0 * c * c, 4.0 * b * c, b * b + 1.0);
    let f = |x: f64| {
        let v1 = (qb + 2.0 * qa * x) / (4.0 * qa);
        let v2 = (qa * x * x + qb * x + qc).sqrt();
        let v3 = (4.0 * qa * qc - qb * qb) / (8.0 * libm::pow(qa, 1.5));
        let v4 = libm::log((2.0 * qa * x + qb + 2.0 * (qa * (qa * x * x + qb * x + qc)).sqrt()).abs());
        v1 * v2 + v3 * v4
    };
    f(l) - f(0.0)
}

/// A vertical segment's curve segment (IfcOpenShell's `_map_alignment_vertical_segment`).
fn vertical_curve(st: &mut Step, v: &VSeg, code: &str) -> usize {
    let t0 = crate::math::atan(v.g0);
    let placement = st.place2([v.dist, v.z0], [crate::math::cos(t0), crate::math::sin(t0)]);
    match v.kind {
        VKind::Grade => {
            let parent = st.line();
            st.segment(code, placement, 0.0, v.length / crate::math::cos(t0), parent)
        }
        VKind::Parabola => {
            let pos = st.origin2();
            let c = (v.g1 - v.g0) / (2.0 * v.length);
            let parent = st.add(format!(
                "IFCPOLYNOMIALCURVE(#{pos},(0.,1.),({},{},{}),$)",
                real(v.z0),
                real(v.g0),
                real(c)
            ));
            st.segment(code, placement, 0.0, polynomial_length(v.g0, c, v.length), parent)
        }
        VKind::Circle => {
            let (mut a0, mut a1) = (t0, crate::math::atan(v.g1));
            let (radius, x, y) = if a0 < a1 {
                let r = v.length / (crate::math::sin(a1) - crate::math::sin(a0));
                let (x, y) = (-r * crate::math::sin(a0), r * crate::math::cos(a0));
                a0 += 1.5 * std::f64::consts::PI;
                a1 += 1.5 * std::f64::consts::PI;
                (r, x, y)
            } else {
                let r = v.length / (crate::math::sin(a0) - crate::math::sin(a1));
                let (x, y) = (r * crate::math::sin(a0), -r * crate::math::cos(a0));
                a0 += 0.5 * std::f64::consts::PI;
                a1 += 0.5 * std::f64::consts::PI;
                (r, x, y)
            };
            let pos = st.place2([x, y], [1.0, 0.0]);
            let parent = st.add(format!("IFCCIRCLE(#{pos},{})", real(radius)));
            st.segment(code, placement, radius * a0, radius * (a1 - a0), parent)
        }
    }
}

fn vertical_type(k: VKind) -> &'static str {
    match k {
        VKind::Grade => "CONSTANTGRADIENT",
        VKind::Parabola => "PARABOLICARC",
        VKind::Circle => "CIRCULARARC",
    }
}

fn vcurvature(v: &VSeg) -> f64 {
    if v.kind == VKind::Grade { 0.0 } else { curvature(v.radius) }
}

/// The business logic and the gradient curve of a vertical alignment over `base`: the layout's and the curve's ids.
fn write_vertical(st: &mut Step, vertical: &Vertical, base: usize) -> (usize, usize) {
    let g = st.gid();
    let layout = st.add(format!("IFCALIGNMENTVERTICAL({g},$,{},$,$,$,$)", text(&vertical.name)));
    let mut segments = Vec::new();
    let mut curves = Vec::new();
    for (i, v) in vertical.segments.iter().enumerate() {
        let next = vertical.segments.get(i + 1);
        let code = match next {
            Some(n) => transition(
                (v.dist + v.length - n.dist).abs() <= 0.001,
                (v.g1 - n.g0).abs() < 1e-9,
                (vcurvature(v) - vcurvature(n)).abs() < 1e-12,
            ),
            None => transition(true, (v.g1 - vertical.end[2]).abs() < 1e-9, vcurvature(v).abs() < 1e-12),
        };
        let radius = if v.kind == VKind::Grade { "$".to_owned() } else { real(v.radius) };
        let params = st.add(format!(
            "IFCALIGNMENTVERTICALSEGMENT($,$,{},{},{},{},{},{radius},.{}.)",
            real(v.dist),
            real(v.length),
            real(v.z0),
            real(v.g0),
            real(v.g1),
            vertical_type(v.kind)
        ));
        let g = st.gid();
        segments.push(st.add(format!("IFCALIGNMENTSEGMENT({g},$,$,$,$,$,$,#{params})")));
        curves.push(vertical_curve(st, v, code));
    }
    // The zero-length last segment.
    let [d, z, gr] = vertical.end;
    let params = st.add(format!(
        "IFCALIGNMENTVERTICALSEGMENT($,$,{},0.,{},{},{},$,.CONSTANTGRADIENT.)",
        real(d),
        real(z),
        real(gr),
        real(gr)
    ));
    let g = st.gid();
    segments.push(st.add(format!("IFCALIGNMENTSEGMENT({g},$,$,$,$,$,$,#{params})")));
    curves.push(vertical_curve(st, &VSeg { kind: VKind::Grade, dist: d, length: 0.0, z0: z, g0: gr, g1: gr, radius: 0.0 }, "DISCONTINUOUS"));
    let g = st.gid();
    st.add(format!("IFCRELNESTS({g},$,$,$,#{layout},({}))", Step::refs(&segments)));
    let gradient = st.add(format!("IFCGRADIENTCURVE(({}),.F.,#{base},$)", Step::refs(&curves)));
    (layout, gradient)
}

/// The file's text.
pub fn write(file: &IfcFile) -> String {
    let mut st = Step { lines: Vec::new(), seed: format!("{}|{}", file.project, file.stamp), ids: 0 };
    // The project, its units and contexts.
    let length = st.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)");
    let angle = st.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)");
    let area = st.add("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)");
    let units = st.add(format!("IFCUNITASSIGNMENT((#{length},#{angle},#{area}))"));
    let o3 = st.add("IFCCARTESIANPOINT((0.,0.,0.))");
    let z3 = st.add("IFCDIRECTION((0.,0.,1.))");
    let x3 = st.add("IFCDIRECTION((1.,0.,0.))");
    let world = st.add(format!("IFCAXIS2PLACEMENT3D(#{o3},#{z3},#{x3})"));
    let model = st.add(format!("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{world},$)"));
    let axis = st.add(format!("IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Axis','Model',*,*,*,*,#{model},$,.MODEL_VIEW.,$)"));
    if let Some((name, words)) = &file.crs {
        let crs = st.add(format!("IFCPROJECTEDCRS({},{},$,$,$,$,#{length})", text(name), text(words)));
        st.add(format!("IFCMAPCONVERSION(#{model},#{crs},0.,0.,0.,1.,0.,1.)"));
    }
    let g = st.gid();
    let project = st.add(format!("IFCPROJECT({g},$,{},$,$,$,$,(#{model}),#{units})", text(&file.project)));
    let place = st.add(format!("IFCLOCALPLACEMENT($,#{world})"));
    let mut tops = Vec::new();
    for a in &file.alignments {
        // The horizontal layout.
        let g = st.gid();
        let horizontal = st.add(format!("IFCALIGNMENTHORIZONTAL({g},$,$,$,$,$,$)"));
        let mut segments = Vec::new();
        let mut curves = Vec::new();
        for (i, s) in a.horizontal.iter().enumerate() {
            let (next_dir, next_k) = match a.horizontal.get(i + 1) {
                Some(n) => (n.dir, curvature(n.r0)),
                None => (a.end_dir, 0.0),
            };
            let code = transition(true, same_angle(s.dir_end, next_dir), (curvature(s.r1) - next_k).abs() < 1e-12);
            let p = st.point2(s.start);
            let kind = match s.kind {
                HKind::Line => "LINE",
                HKind::Arc => "CIRCULARARC",
                HKind::Clothoid => "CLOTHOID",
            };
            let params = st.add(format!(
                "IFCALIGNMENTHORIZONTALSEGMENT($,$,#{p},{},{},{},{},$,.{kind}.)",
                real(s.dir),
                real(s.r0),
                real(s.r1),
                real(s.length)
            ));
            let g = st.gid();
            segments.push(st.add(format!("IFCALIGNMENTSEGMENT({g},$,$,$,$,$,$,#{params})")));
            curves.push(horizontal_curve(&mut st, s, code));
        }
        let p = st.point2(a.end);
        let params = st.add(format!("IFCALIGNMENTHORIZONTALSEGMENT($,$,#{p},{},0.,0.,0.,$,.LINE.)", real(a.end_dir)));
        let g = st.gid();
        segments.push(st.add(format!("IFCALIGNMENTSEGMENT({g},$,$,$,$,$,$,#{params})")));
        let last = HSeg { kind: HKind::Line, start: a.end, dir: a.end_dir, dir_end: a.end_dir, r0: 0.0, r1: 0.0, length: 0.0 };
        curves.push(horizontal_curve(&mut st, &last, "DISCONTINUOUS"));
        let g = st.gid();
        st.add(format!("IFCRELNESTS({g},$,$,$,#{horizontal},({}))", Step::refs(&segments)));
        let composite = st.add(format!("IFCCOMPOSITECURVE(({}),.F.)", Step::refs(&curves)));
        let foot = st.add(format!("IFCSHAPEREPRESENTATION(#{axis},'FootPrint','Curve2D',(#{composite}))"));
        // The first vertical on the alignment, the others on child alignments.
        let mut layouts = vec![horizontal];
        let mut reps = vec![foot];
        let mut children = Vec::new();
        for (k, v) in a.verticals.iter().enumerate() {
            let (layout, gradient) = write_vertical(&mut st, v, composite);
            let rep = st.add(format!("IFCSHAPEREPRESENTATION(#{axis},'Axis','Curve3D',(#{gradient}))"));
            if k == 0 {
                layouts.push(layout);
                reps.push(rep);
            } else {
                let shape = st.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"));
                let g = st.gid();
                let child = st.add(format!(
                    "IFCALIGNMENT({g},$,{},$,$,#{place},#{shape},.NOTDEFINED.)",
                    text(&format!("{} / {}", a.name, v.name))
                ));
                let g = st.gid();
                st.add(format!("IFCRELNESTS({g},$,$,$,#{child},(#{layout}))"));
                children.push(child);
            }
        }
        let shape = st.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({}))", Step::refs(&reps)));
        let g = st.gid();
        let alignment = st.add(format!("IFCALIGNMENT({g},$,{},$,$,#{place},#{shape},.NOTDEFINED.)", text(&a.name)));
        let g = st.gid();
        st.add(format!("IFCRELNESTS({g},$,$,$,#{alignment},({}))", Step::refs(&layouts)));
        if !children.is_empty() {
            let g = st.gid();
            st.add(format!("IFCRELAGGREGATES({g},$,$,$,#{alignment},({}))", Step::refs(&children)));
        }
        // The km: a referent at the start and one at each equation, on the horizontal curve.
        let mut referents = Vec::new();
        let marks = std::iter::once((0.0, a.start_km, None)).chain(a.equations.iter().map(|q| (q.dist, q.ahead, Some(q.back))));
        for (dist, km, back) in marks {
            let at = st.add(format!("IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE({}),$,$,$,#{composite})", real(dist)));
            let lin = st.add(format!("IFCAXIS2PLACEMENTLINEAR(#{at},$,$)"));
            let placement = st.add(format!("IFCLINEARPLACEMENT($,#{lin},$)"));
            let g = st.gid();
            let referent = st.add(format!("IFCREFERENT({g},$,{},$,$,#{placement},$,.STATION.)", text(&format!("km {}", real(km)))));
            let mut props = vec![st.add(format!("IFCPROPERTYSINGLEVALUE('Station',$,IFCLENGTHMEASURE({}),$)", real(km)))];
            if let Some(b) = back {
                props.push(st.add(format!("IFCPROPERTYSINGLEVALUE('IncomingStation',$,IFCLENGTHMEASURE({}),$)", real(b))));
            }
            let g = st.gid();
            let pset = st.add(format!("IFCPROPERTYSET({g},$,'Pset_Stationing',$,({}))", Step::refs(&props)));
            let g = st.gid();
            st.add(format!("IFCRELDEFINESBYPROPERTIES({g},$,$,$,(#{referent}),#{pset})"));
            referents.push(referent);
        }
        let g = st.gid();
        st.add(format!("IFCRELNESTS({g},$,$,$,#{alignment},({}))", Step::refs(&referents)));
        tops.push(alignment);
    }
    if !tops.is_empty() {
        let g = st.gid();
        st.add(format!("IFCRELAGGREGATES({g},$,$,$,#{project},({}))", Step::refs(&tops)));
    }
    let mut out = String::new();
    out.push_str("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [Alignment-basedReferenceView]'),'2;1');\n");
    out.push_str(&format!(
        "FILE_NAME({},{},(''),(''),{},{},'');\nFILE_SCHEMA(('IFC4X3_ADD2'));\nENDSEC;\nDATA;\n",
        text(&format!("{}.ifc", file.project)),
        text(&file.stamp),
        text(&file.application),
        text(&file.application)
    ));
    for (i, l) in st.lines.iter().enumerate() {
        out.push_str(&format!("#{}={};\n", i + 1, l));
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_guid_has_22_characters_of_its_alphabet() {
        let g = guid(u128::MAX);
        assert_eq!(g.len(), 22);
        assert_eq!(g, "3$$$$$$$$$$$$$$$$$$$$$");
        assert_eq!(guid(0), "0000000000000000000000");
    }

    #[test]
    fn strings_and_reals_are_written_as_step_writes_them() {
        assert_eq!(text("Ana aks'ı"), "'Ana aks''\\X2\\0131\\X0\\'");
        assert_eq!(real(150.0), "150.");
        assert_eq!(real(-0.0), "0.");
        assert_eq!(real(0.25), "0.25");
    }
}
