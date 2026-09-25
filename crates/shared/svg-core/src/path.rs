//! SVG path data as editable nodes (`apps/web/src/style/svg/pathData.ts`):
//! every command (lines, cubic and quadratic Béziers, smooth variants,
//! elliptic arcs) becomes nodes with optional cubic handles, in absolute
//! coordinates. A segment from node i to i+1 is a cubic with controls
//! (i.out ?? i) and (i+1.in ?? i+1): both missing makes it a straight line.

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::{PI, atan2, cos, js_max, js_round, pow, sin, tan};
use kentos_style_core::js::number;

use crate::shape::{PathNode, Pt, SubPath};

/// An affine matrix as SVG writes it: x' = a·x + c·y + e, y' = b·x + d·y + f.
pub type Matrix = [f64; 6];

pub const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

pub fn multiply(m: &Matrix, n: &Matrix) -> Matrix {
    [
        m[0] * n[0] + m[2] * n[1],
        m[1] * n[0] + m[3] * n[1],
        m[0] * n[2] + m[2] * n[3],
        m[1] * n[2] + m[3] * n[3],
        m[0] * n[4] + m[2] * n[5] + m[4],
        m[1] * n[4] + m[3] * n[5] + m[5],
    ]
}

pub fn apply(m: &Matrix, x: f64, y: f64) -> Pt {
    [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]
}

// ── Parsing ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum Token {
    Cmd(u8),
    Num(String),
}

fn is_cmd(c: u8) -> bool {
    matches!(
        c,
        b'M' | b'm'
            | b'L'
            | b'l'
            | b'H'
            | b'h'
            | b'V'
            | b'v'
            | b'C'
            | b'c'
            | b'S'
            | b's'
            | b'Q'
            | b'q'
            | b'T'
            | b't'
            | b'A'
            | b'a'
            | b'Z'
            | b'z'
    )
}

/// The length of a number at the start of `s` (`[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?`), or 0.
pub(crate) fn number_len(s: &[u8]) -> usize {
    let digits = |from: usize| s[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut i = 0;
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        i += 1;
    }
    let int = digits(i);
    if int > 0 {
        i += int;
        if i < s.len() && s[i] == b'.' {
            i += 1;
            i += digits(i);
        }
    } else if i < s.len() && s[i] == b'.' && digits(i + 1) > 0 {
        i += 1 + digits(i + 1);
    } else {
        return 0;
    }
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        let mut j = i + 1;
        if j < s.len() && (s[j] == b'-' || s[j] == b'+') {
            j += 1;
        }
        let e = digits(j);
        if e > 0 {
            i = j + e;
        }
    }
    i
}

/// The tokens of path data: command letters and numbers; anything else is skipped.
fn tokens(d: &str) -> Vec<Token> {
    let s = d.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if is_cmd(s[i]) {
            out.push(Token::Cmd(s[i]));
            i += 1;
            continue;
        }
        let n = number_len(&s[i..]);
        if n > 0 {
            // Numbers are ASCII: the slice is on character boundaries.
            out.push(Token::Num(d[i..i + n].to_string()));
            i += n;
        } else {
            i += 1;
        }
    }
    out
}

/// `Number(text)` for a number token.
fn read_number(t: &str) -> f64 {
    t.parse::<f64>().unwrap_or(f64::NAN)
}

struct Reader {
    tokens: Vec<Token>,
    i: usize,
}

/// A malformed tail: the parser keeps what was read.
struct Broken;

impl Reader {
    fn num(&mut self) -> Result<f64, Broken> {
        match self.tokens.get(self.i) {
            Some(Token::Num(t)) => {
                let v = read_number(t);
                self.i += 1;
                Ok(v)
            }
            _ => Err(Broken),
        }
    }

    /// An arc flag: compact flags ("011") take one digit at a time.
    fn flag(&mut self) -> Result<bool, Broken> {
        let t = match self.tokens.get(self.i) {
            Some(Token::Num(t)) => t.clone(),
            _ => return Err(Broken),
        };
        let b = t.as_bytes();
        if b.len() > 1 && (b[0] == b'0' || b[0] == b'1') && b[1].is_ascii_digit() {
            self.tokens[self.i] = Token::Num(t[1..].to_string());
            return Ok(b[0] == b'1');
        }
        self.i += 1;
        Ok(read_number(&t) != 0.0)
    }
}

struct Parser {
    subs: Vec<SubPath>,
    /// Index of the sub-path being drawn, if any.
    cur: Option<usize>,
    x: f64,
    y: f64,
}

impl Parser {
    fn ensure(&mut self) -> usize {
        match self.cur {
            Some(c) => c,
            None => {
                self.subs.push(SubPath {
                    nodes: vec![PathNode::at(self.x, self.y)],
                    closed: false,
                });
                let c = self.subs.len() - 1;
                self.cur = Some(c);
                c
            }
        }
    }

    fn line_to(&mut self, nx: f64, ny: f64) {
        let c = self.ensure();
        self.subs[c].nodes.push(PathNode::at(nx, ny));
        self.x = nx;
        self.y = ny;
    }

    fn cubic_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, nx: f64, ny: f64) {
        let c = self.ensure();
        let sp = &mut self.subs[c];
        if let Some(prev) = sp.nodes.last_mut() {
            prev.out = Some([c1x, c1y]);
        }
        sp.nodes.push(PathNode {
            in_: Some([c2x, c2y]),
            ..PathNode::at(nx, ny)
        });
        self.x = nx;
        self.y = ny;
    }
}

/// Nodes of every subpath in `d`; unknown text is skipped, a broken tail ends the path.
pub fn parse_path_data(d: &str) -> Vec<SubPath> {
    let mut r = Reader {
        tokens: tokens(d),
        i: 0,
    };
    let mut p = Parser {
        subs: Vec::new(),
        cur: None,
        x: 0.0,
        y: 0.0,
    };
    let mut start = (0.0, 0.0);
    let mut cmd: u8 = 0;
    let mut last_cubic: Option<Pt> = None;
    let mut last_quad: Option<Pt> = None;
    let _ = (|| -> Result<(), Broken> {
        while r.i < r.tokens.len() {
            match &r.tokens[r.i] {
                Token::Cmd(c) => {
                    cmd = *c;
                    r.i += 1;
                }
                Token::Num(_) if cmd == 0 => break,
                Token::Num(_) => {}
            }
            let rel = cmd.is_ascii_lowercase() && cmd != b'z';
            let ox = if rel { p.x } else { 0.0 };
            let oy = if rel { p.y } else { 0.0 };
            let mut cubic: Option<Pt> = None;
            let mut quad: Option<Pt> = None;
            match cmd.to_ascii_uppercase() {
                b'M' => {
                    let nx = ox + r.num()?;
                    let ny = oy + r.num()?;
                    p.subs.push(SubPath {
                        nodes: vec![PathNode::at(nx, ny)],
                        closed: false,
                    });
                    p.cur = Some(p.subs.len() - 1);
                    p.x = nx;
                    p.y = ny;
                    start = (nx, ny);
                    // Pairs after a moveto are linetos.
                    cmd = if rel { b'l' } else { b'L' };
                }
                b'L' => {
                    let nx = ox + r.num()?;
                    let ny = oy + r.num()?;
                    p.line_to(nx, ny);
                }
                b'H' => {
                    let nx = ox + r.num()?;
                    p.line_to(nx, p.y);
                }
                b'V' => {
                    let ny = oy + r.num()?;
                    p.line_to(p.x, ny);
                }
                b'C' => {
                    let c1x = ox + r.num()?;
                    let c1y = oy + r.num()?;
                    let c2x = ox + r.num()?;
                    let c2y = oy + r.num()?;
                    let nx = ox + r.num()?;
                    let ny = oy + r.num()?;
                    p.cubic_to(c1x, c1y, c2x, c2y, nx, ny);
                    cubic = Some([c2x, c2y]);
                }
                b'S' => {
                    let (x, y) = (p.x, p.y);
                    let c1: Pt = match last_cubic {
                        Some(l) => [2.0 * x - l[0], 2.0 * y - l[1]],
                        None => [x, y],
                    };
                    let c2x = ox + r.num()?;
                    let c2y = oy + r.num()?;
                    let nx = ox + r.num()?;
                    let ny = oy + r.num()?;
                    p.cubic_to(c1[0], c1[1], c2x, c2y, nx, ny);
                    cubic = Some([c2x, c2y]);
                }
                b'Q' => {
                    let qx = ox + r.num()?;
                    let qy = oy + r.num()?;
                    let nx = ox + r.num()?;
                    let ny = oy + r.num()?;
                    let (x, y) = (p.x, p.y);
                    p.cubic_to(
                        x + (2.0 / 3.0) * (qx - x),
                        y + (2.0 / 3.0) * (qy - y),
                        nx + (2.0 / 3.0) * (qx - nx),
                        ny + (2.0 / 3.0) * (qy - ny),
                        nx,
                        ny,
                    );
                    quad = Some([qx, qy]);
                }
                b'T' => {
                    let (x, y) = (p.x, p.y);
                    let q: Pt = match last_quad {
                        Some(l) => [2.0 * x - l[0], 2.0 * y - l[1]],
                        None => [x, y],
                    };
                    let nx = ox + r.num()?;
                    let ny = oy + r.num()?;
                    p.cubic_to(
                        x + (2.0 / 3.0) * (q[0] - x),
                        y + (2.0 / 3.0) * (q[1] - y),
                        nx + (2.0 / 3.0) * (q[0] - nx),
                        ny + (2.0 / 3.0) * (q[1] - ny),
                        nx,
                        ny,
                    );
                    quad = Some(q);
                }
                b'A' => {
                    let rx = r.num()?;
                    let ry = r.num()?;
                    let rot = r.num()?;
                    let large = r.flag()?;
                    let sweep = r.flag()?;
                    let nx = ox + r.num()?;
                    let ny = oy + r.num()?;
                    for c in arc_to_cubics(p.x, p.y, rx, ry, rot, large, sweep, nx, ny) {
                        p.cubic_to(c[0], c[1], c[2], c[3], c[4], c[5]);
                    }
                    p.x = nx;
                    p.y = ny;
                }
                b'Z' => {
                    if let Some(c) = p.cur {
                        let sp = &mut p.subs[c];
                        let n = sp.nodes.len();
                        // A closing segment that ends on the start merges into it.
                        if n > 1
                            && (sp.nodes[n - 1].x - sp.nodes[0].x).abs() < 1e-9
                            && (sp.nodes[n - 1].y - sp.nodes[0].y).abs() < 1e-9
                        {
                            if let Some(last_in) = sp.nodes[n - 1].in_ {
                                sp.nodes[0].in_ = Some(last_in);
                            }
                            sp.nodes.pop();
                        }
                        sp.closed = true;
                    }
                    p.x = start.0;
                    p.y = start.1;
                    p.cur = None;
                    cmd = 0;
                }
                _ => r.i += 1,
            }
            last_cubic = cubic;
            last_quad = quad;
        }
        Ok(())
    })();
    p.subs.retain(|s| !s.nodes.is_empty());
    p.subs
}

/// Cubic segments approximating an SVG arc (endpoint form, F.6.5 of the SVG
/// spec), each spanning at most 90°: [c1x, c1y, c2x, c2y, x, y] each.
#[allow(clippy::too_many_arguments)]
pub fn arc_to_cubics(
    x1: f64,
    y1: f64,
    rx_in: f64,
    ry_in: f64,
    rot_deg: f64,
    large: bool,
    sweep: bool,
    x2: f64,
    y2: f64,
) -> Vec<[f64; 6]> {
    let mut rx = rx_in.abs();
    let mut ry = ry_in.abs();
    if (x1 == x2 && y1 == y2) || rx == 0.0 || ry == 0.0 {
        return vec![[x1, y1, x2, y2, x2, y2]];
    }
    let phi = (rot_deg * PI) / 180.0;
    let cs = cos(phi);
    let sn = sin(phi);
    let dx = (x1 - x2) / 2.0;
    let dy = (y1 - y2) / 2.0;
    let x1p = cs * dx + sn * dy;
    let y1p = -sn * dx + cs * dy;
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let coef = (if large != sweep { 1.0 } else { -1.0 }) * js_max(0.0, num / den).sqrt();
    let cxp = (coef * rx * y1p) / ry;
    let cyp = (-coef * ry * x1p) / rx;
    let cx = cs * cxp - sn * cyp + (x1 + x2) / 2.0;
    let cy = sn * cxp + cs * cyp + (y1 + y2) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| atan2(ux * vy - uy * vx, ux * vx + uy * vy);
    let t1 = angle(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dt = angle(
        (x1p - cxp) / rx,
        (y1p - cyp) / ry,
        (-x1p - cxp) / rx,
        (-y1p - cyp) / ry,
    );
    if !sweep && dt > 0.0 {
        dt -= 2.0 * PI;
    } else if sweep && dt < 0.0 {
        dt += 2.0 * PI;
    }
    let n = js_max(1.0, (dt.abs() / (PI / 2.0) - 1e-9).ceil());
    let step = dt / n;
    let k = (4.0 / 3.0) * tan(step / 4.0);
    let point = |t: f64| -> Pt {
        [
            cx + rx * cos(t) * cs - ry * sin(t) * sn,
            cy + rx * cos(t) * sn + ry * sin(t) * cs,
        ]
    };
    let deriv = |t: f64| -> Pt {
        [
            -rx * sin(t) * cs - ry * cos(t) * sn,
            -rx * sin(t) * sn + ry * cos(t) * cs,
        ]
    };
    let mut out = Vec::new();
    // `for (let s = 0; s < n; s++)`: NaN runs no step.
    let mut s = 0.0;
    while s < n {
        let a = t1 + s * step;
        let b = a + step;
        let p0 = point(a);
        let p3 = if s == n - 1.0 { [x2, y2] } else { point(b) };
        let d0 = deriv(a);
        let d3 = deriv(b);
        out.push([
            p0[0] + k * d0[0],
            p0[1] + k * d0[1],
            p3[0] - k * d3[0],
            p3[1] - k * d3[1],
            p3[0],
            p3[1],
        ]);
        s += 1.0;
    }
    out
}

// ── Writing ────────────────────────────────────────────────────────────

/// A coordinate rounded to `digits` decimals, as `String(x)` writes it.
pub fn fixed(v: f64, digits: f64) -> String {
    let s = number::to_string(js_round(v * pow(10.0, digits)) / pow(10.0, digits));
    if s == "-0" { "0".to_string() } else { s }
}

/// Path data of subpaths, absolute, lines as L and curves as C.
pub fn path_data_of(subs: &[SubPath], digits: f64) -> String {
    let mut out = String::new();
    let p = |out: &mut String, q: Pt| {
        out.push_str(&fixed(q[0], digits));
        out.push(' ');
        out.push_str(&fixed(q[1], digits));
    };
    let seg = |out: &mut String, a: &PathNode, b: &PathNode| {
        if a.out.is_some() || b.in_.is_some() {
            out.push('C');
            p(out, a.out.unwrap_or([a.x, a.y]));
            out.push(' ');
            p(out, b.in_.unwrap_or([b.x, b.y]));
            out.push(' ');
            p(out, b.pt());
        } else {
            out.push('L');
            p(out, b.pt());
        }
    };
    for sp in subs {
        let n = sp.nodes.len();
        if n == 0 {
            continue;
        }
        out.push('M');
        p(&mut out, sp.nodes[0].pt());
        for i in 1..n {
            seg(&mut out, &sp.nodes[i - 1], &sp.nodes[i]);
        }
        if sp.closed {
            let last = &sp.nodes[n - 1];
            let first = &sp.nodes[0];
            if n > 1 && (last.out.is_some() || first.in_.is_some()) {
                seg(&mut out, last, first);
            }
            out.push('Z');
        }
    }
    out
}

// ── Geometry ───────────────────────────────────────────────────────────

pub fn transform_sub_paths(subs: &[SubPath], m: &Matrix) -> Vec<SubPath> {
    let t = |p: &Pt| apply(m, p[0], p[1]);
    subs.iter()
        .map(|sp| SubPath {
            closed: sp.closed,
            nodes: sp
                .nodes
                .iter()
                .map(|n| {
                    let [x, y] = apply(m, n.x, n.y);
                    PathNode {
                        x,
                        y,
                        in_: n.in_.as_ref().map(t),
                        out: n.out.as_ref().map(t),
                        // `...(n.type ? { type } : {})`: an empty type is dropped.
                        ty: n.ty.clone().filter(|s| !s.is_empty()),
                    }
                })
                .collect(),
        })
        .collect()
}

pub fn empty_box() -> Bounds {
    Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    }
}

pub fn grow_box(b: &mut Bounds, x: f64, y: f64) {
    if x < b.min_x {
        b.min_x = x;
    }
    if y < b.min_y {
        b.min_y = y;
    }
    if x > b.max_x {
        b.max_x = x;
    }
    if y > b.max_y {
        b.max_y = y;
    }
}

/// Points along every segment (curves sampled), for bounds and hit tests.
pub fn flatten_sub_path(sp: &SubPath, steps: usize) -> Vec<Pt> {
    let mut out = Vec::new();
    let n = sp.nodes.len();
    if n == 0 {
        return out;
    }
    out.push(sp.nodes[0].pt());
    let segs = if sp.closed { n } else { n - 1 };
    let st = steps as f64;
    for i in 0..segs {
        let a = &sp.nodes[i];
        let b = &sp.nodes[(i + 1) % n];
        if a.out.is_none() && b.in_.is_none() {
            out.push(b.pt());
            continue;
        }
        let c1 = a.out.unwrap_or([a.x, a.y]);
        let c2 = b.in_.unwrap_or([b.x, b.y]);
        for s in 1..=steps {
            let t = s as f64 / st;
            let u = 1.0 - t;
            out.push([
                u * u * u * a.x
                    + 3.0 * u * u * t * c1[0]
                    + 3.0 * u * t * t * c2[0]
                    + t * t * t * b.x,
                u * u * u * a.y
                    + 3.0 * u * u * t * c1[1]
                    + 3.0 * u * t * t * c2[1]
                    + t * t * t * b.y,
            ]);
        }
    }
    out
}

pub fn sub_paths_box(subs: &[SubPath]) -> Bounds {
    let mut b = empty_box();
    for sp in subs {
        for [x, y] in flatten_sub_path(sp, 16) {
            grow_box(&mut b, x, y);
        }
    }
    b
}
