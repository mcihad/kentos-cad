//! Vector tiles' labels (docs/adr/0208 §9): the candidates a tile gives and
//! which of them a frame shows. A point label is wrapped to its maximum
//! width and set by its anchor and offset; a line label follows its line
//! letter by letter at the symbol spacing (upright, bending at most 45°
//! from one letter to the next); a label that would overlap one already
//! placed is dropped (the style's later layers first, as MapLibre places
//! them), and the same text of the same layer is not repeated close by
//! (the tiles' buffers give a line twice at their edges). Letters are
//! measured in the overlay's face (Arimo) with the geometry core's tables.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::text::{Font, width_em_in};

use crate::style::color::Rgba;

/// Where a point label stands from its point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Center,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Anchor {
    pub fn from_name(name: &str) -> Anchor {
        match name {
            "left" => Anchor::Left,
            "right" => Anchor::Right,
            "top" => Anchor::Top,
            "bottom" => Anchor::Bottom,
            "top-left" => Anchor::TopLeft,
            "top-right" => Anchor::TopRight,
            "bottom-left" => Anchor::BottomLeft,
            "bottom-right" => Anchor::BottomRight,
            _ => Anchor::Center,
        }
    }

    /// The share of the box left of and above the point.
    fn shares(self) -> (f64, f64) {
        match self {
            Anchor::Center => (0.5, 0.5),
            Anchor::Left => (0.0, 0.5),
            Anchor::Right => (1.0, 0.5),
            Anchor::Top => (0.5, 0.0),
            Anchor::Bottom => (0.5, 1.0),
            Anchor::TopLeft => (0.0, 0.0),
            Anchor::TopRight => (1.0, 0.0),
            Anchor::BottomLeft => (0.0, 1.0),
            Anchor::BottomRight => (1.0, 1.0),
        }
    }
}

/// A label's look: sizes in screen pixels, `letter_spacing`, `max_width`
/// and `offset` in ems as the style writes them.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelStyle {
    pub size: f64,
    pub color: Rgba,
    pub opacity: f64,
    /// The halo's colour and width.
    pub halo: Option<(Rgba, f64)>,
    pub bold: bool,
    pub italic: bool,
    pub letter_spacing: f64,
    pub max_width: f64,
    pub anchor: Anchor,
    pub offset: [f64; 2],
    pub padding: f64,
    pub allow_overlap: bool,
    /// Its layer's place in the style: a later layer is placed first.
    pub priority: u32,
    pub layer: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Placement {
    /// At a point (the project's system).
    Point(Vec2),
    /// Along a line, every `spacing` pixels (`center`: once, at its middle).
    Line {
        path: Vec<Vec2>,
        spacing: f64,
        center: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub text: String,
    pub style: LabelStyle,
    pub place: Placement,
    /// The tile it came from.
    pub tile: u64,
}

/// The view a frame shows: the project's point at the screen's top left,
/// its pixels a unit, the screen's size in pixels (Y down on the screen).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub x0: f64,
    pub y0: f64,
    pub px_per_unit: f64,
    pub width: f64,
    pub height: f64,
}

impl Screen {
    pub fn to_screen(&self, p: Vec2) -> [f64; 2] {
        [
            (p.x - self.x0) * self.px_per_unit,
            (self.y0 - p.y) * self.px_per_unit,
        ]
    }
}

/// A line of a point label: its text, its left baseline point on the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub x: f64,
    pub y: f64,
}

/// A letter of a line label: its centre on the baseline, its turn
/// (radians, clockwise on the screen from its X axis).
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub ch: char,
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

/// A label the frame shows: its candidate's index and where its text goes.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub candidate: usize,
    pub lines: Vec<Line>,
    pub glyphs: Vec<Glyph>,
}

/// The screen's boxes taken, in a grid of 64-pixel cells.
pub struct Collision {
    cell: f64,
    cols: usize,
    rows: usize,
    cells: Vec<Vec<[f64; 4]>>,
}

impl Collision {
    pub fn new(width: f64, height: f64) -> Collision {
        let cell = 64.0;
        let cols = ((width / cell).ceil() as usize).clamp(1, 512);
        let rows = ((height / cell).ceil() as usize).clamp(1, 512);
        Collision {
            cell,
            cols,
            rows,
            cells: vec![Vec::new(); cols * rows],
        }
    }

    fn range(&self, b: &[f64; 4]) -> (usize, usize, usize, usize) {
        let c = |v: f64, n: usize| ((v / self.cell).floor().max(0.0) as usize).min(n - 1);
        (
            c(b[0], self.cols),
            c(b[1], self.rows),
            c(b[2], self.cols),
            c(b[3], self.rows),
        )
    }

    pub fn hits(&self, b: &[f64; 4]) -> bool {
        let (c1, r1, c2, r2) = self.range(b);
        (r1..=r2).any(|r| {
            (c1..=c2).any(|c| {
                self.cells[r * self.cols + c]
                    .iter()
                    .any(|o| b[0] < o[2] && o[0] < b[2] && b[1] < o[3] && o[1] < b[3])
            })
        })
    }

    pub fn take(&mut self, b: [f64; 4]) {
        let (c1, r1, c2, r2) = self.range(&b);
        for r in r1..=r2 {
            for c in c1..=c2 {
                self.cells[r * self.cols + c].push(b);
            }
        }
    }
}

fn face() -> Font {
    Font::from_id("arimo")
}

/// A text's width in pixels at `size`, with its letter spacing.
pub fn text_width(text: &str, style: &LabelStyle) -> f64 {
    let n = text.chars().count() as f64;
    (width_em_in(text, face(), style.bold) + style.letter_spacing * (n - 1.0).max(0.0)) * style.size
}

/// The lines of a point label within its maximum width (words kept whole).
pub fn wrap(text: &str, style: &LabelStyle) -> Vec<String> {
    let most = style.max_width * style.size;
    let mut lines: Vec<String> = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let tried = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && text_width(&tried, style) > most {
                lines.push(std::mem::take(&mut line));
                line = word.to_owned();
            } else {
                line = tried;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

/// The point and the turn at `d` along a screen path, and the segment it is on.
fn along(path: &[[f64; 2]], cum: &[f64], d: f64) -> Option<([f64; 2], f64)> {
    let i = cum.partition_point(|&c| c <= d).saturating_sub(1);
    let (a, b) = (path.get(i)?, path.get(i + 1)?);
    let len = cum[i + 1] - cum[i];
    let t = if len > 0.0 { (d - cum[i]) / len } else { 0.0 };
    Some((
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t],
        (b[1] - a[1]).atan2(b[0] - a[0]),
    ))
}

/// Places the candidates (owned or borrowed: a host's tiles keep theirs) on
/// the screen, the later style layers first, into `out` (cleared first).
pub fn place<C: std::borrow::Borrow<Candidate>>(
    cands: &[C],
    screen: &Screen,
    out: &mut Vec<Placed>,
) {
    let cands: Vec<&Candidate> = cands.iter().map(std::borrow::Borrow::borrow).collect();
    out.clear();
    let mut taken = Collision::new(screen.width, screen.height);
    let mut order: Vec<usize> = (0..cands.len()).collect();
    order.sort_by(|&a, &b| {
        cands[b]
            .style
            .priority
            .cmp(&cands[a].style.priority)
            .then(a.cmp(&b))
    });
    // Placed texts by layer, for the repeats.
    let mut shown: Vec<(&str, &str, [f64; 2])> = Vec::new();
    let near = |shown: &Vec<(&str, &str, [f64; 2])>, c: &Candidate, at: [f64; 2], within: f64| {
        shown.iter().any(|(t, l, p)| {
            *t == c.text && *l == c.style.layer && (p[0] - at[0]).hypot(p[1] - at[1]) < within
        })
    };
    let margin = 64.0;
    let on_screen = |p: [f64; 2]| {
        p[0] > -margin
            && p[1] > -margin
            && p[0] < screen.width + margin
            && p[1] < screen.height + margin
    };
    for i in order {
        let c = cands[i];
        let s = &c.style;
        match &c.place {
            Placement::Point(p) => {
                let at = screen.to_screen(*p);
                if !on_screen(at) || near(&shown, c, at, s.size * 4.0) {
                    continue;
                }
                let lines = wrap(&c.text, s);
                if lines.is_empty() {
                    continue;
                }
                let lh = s.size * 1.2;
                let w = lines.iter().map(|l| text_width(l, s)).fold(0.0, f64::max);
                let h = lh * lines.len() as f64;
                let (fx, fy) = s.anchor.shares();
                let left = at[0] + s.offset[0] * s.size - fx * w;
                let top = at[1] + s.offset[1] * s.size - fy * h;
                let b = [
                    left - s.padding,
                    top - s.padding,
                    left + w + s.padding,
                    top + h + s.padding,
                ];
                if !s.allow_overlap && taken.hits(&b) {
                    continue;
                }
                taken.take(b);
                shown.push((&c.text, &s.layer, at));
                out.push(Placed {
                    candidate: i,
                    lines: lines
                        .into_iter()
                        .enumerate()
                        .map(|(k, text)| {
                            let lw = text_width(&text, s);
                            Line {
                                x: left
                                    + (w - lw)
                                        * if fx == 0.0 {
                                            0.0
                                        } else if fx == 1.0 {
                                            1.0
                                        } else {
                                            0.5
                                        },
                                y: top + lh * k as f64 + s.size * 0.95,
                                text,
                            }
                        })
                        .collect(),
                    glyphs: Vec::new(),
                });
            }
            Placement::Line {
                path,
                spacing,
                center,
            } => {
                let mut pts: Vec<[f64; 2]> = path.iter().map(|p| screen.to_screen(*p)).collect();
                if pts.len() < 2 || !pts.iter().any(|p| on_screen(*p)) {
                    continue;
                }
                let w = text_width(&c.text, s);
                let mut cum = Vec::with_capacity(pts.len());
                let mut total = 0.0;
                cum.push(0.0);
                for k in 1..pts.len() {
                    total += (pts[k][0] - pts[k - 1][0]).hypot(pts[k][1] - pts[k - 1][1]);
                    cum.push(total);
                }
                if total < w + s.size {
                    continue;
                }
                // Upright: read left to right.
                if pts[pts.len() - 1][0] < pts[0][0] {
                    pts.reverse();
                    cum.clear();
                    cum.push(0.0);
                    let mut t = 0.0;
                    for k in 1..pts.len() {
                        t += (pts[k][0] - pts[k - 1][0]).hypot(pts[k][1] - pts[k - 1][1]);
                        cum.push(t);
                    }
                }
                let n = if *center {
                    1
                } else {
                    ((total / spacing).floor() as usize).max(1)
                };
                for k in 0..n {
                    let mid = total * (k as f64 + 0.5) / n as f64;
                    let start = mid - w / 2.0;
                    if start < 0.0 || mid + w / 2.0 > total {
                        continue;
                    }
                    let Some((m, _)) = along(&pts, &cum, mid) else {
                        continue;
                    };
                    if !on_screen(m) || near(&shown, c, m, spacing.max(s.size * 8.0) * 0.5) {
                        continue;
                    }
                    let mut glyphs = Vec::with_capacity(c.text.chars().count());
                    let mut boxes = Vec::with_capacity(glyphs.capacity());
                    let mut d = start;
                    let mut last_angle: Option<f64> = None;
                    let mut ok = true;
                    let mut buf = [0u8; 4];
                    for ch in c.text.chars() {
                        let cw = width_em_in(ch.encode_utf8(&mut buf), face(), s.bold) * s.size;
                        let Some((p, angle)) = along(&pts, &cum, d + cw / 2.0) else {
                            ok = false;
                            break;
                        };
                        if let Some(prev) = last_angle {
                            let mut turn = (angle - prev).abs();
                            if turn > std::f64::consts::PI {
                                turn = 2.0 * std::f64::consts::PI - turn;
                            }
                            if turn > std::f64::consts::FRAC_PI_4 {
                                ok = false;
                                break;
                            }
                        }
                        last_angle = Some(angle);
                        let half = s.size * 0.6 + s.padding;
                        boxes.push([p[0] - half, p[1] - half, p[0] + half, p[1] + half]);
                        glyphs.push(Glyph {
                            ch,
                            x: p[0],
                            y: p[1],
                            angle,
                        });
                        d += cw + s.letter_spacing * s.size;
                    }
                    if !ok || (!s.allow_overlap && boxes.iter().any(|b| taken.hits(b))) {
                        continue;
                    }
                    for b in boxes {
                        taken.take(b);
                    }
                    shown.push((&c.text, &s.layer, m));
                    out.push(Placed {
                        candidate: i,
                        lines: Vec::new(),
                        glyphs,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(priority: u32) -> LabelStyle {
        LabelStyle {
            size: 12.0,
            color: [0.0, 0.0, 0.0, 1.0],
            opacity: 1.0,
            halo: None,
            bold: false,
            italic: false,
            letter_spacing: 0.0,
            max_width: 10.0,
            anchor: Anchor::Center,
            offset: [0.0, 0.0],
            padding: 2.0,
            allow_overlap: false,
            priority,
            layer: format!("l{priority}"),
        }
    }

    #[test]
    fn overlapping_labels_give_way_to_the_later_layers() {
        let screen = Screen {
            x0: 0.0,
            y0: 100.0,
            px_per_unit: 1.0,
            width: 200.0,
            height: 100.0,
        };
        let cands = vec![
            Candidate {
                text: "Ankara".into(),
                style: style(1),
                place: Placement::Point(Vec2::new(100.0, 50.0)),
                tile: 0,
            },
            Candidate {
                text: "Çankaya".into(),
                style: style(5),
                place: Placement::Point(Vec2::new(104.0, 52.0)),
                tile: 0,
            },
            Candidate {
                text: "Uzak".into(),
                style: style(1),
                place: Placement::Point(Vec2::new(20.0, 90.0)),
                tile: 0,
            },
        ];
        let mut out = Vec::new();
        place(&cands, &screen, &mut out);
        let shown: Vec<usize> = out.iter().map(|p| p.candidate).collect();
        assert_eq!(shown, vec![1, 2]);
        // The text's centre is the point's: the screen's (104, 48).
        let l = &out[0].lines[0];
        let w = text_width("Çankaya", &cands[1].style);
        assert!((l.x + w / 2.0 - 104.0).abs() < 1e-9);
    }

    #[test]
    fn a_line_label_follows_its_line_upright() {
        let screen = Screen {
            x0: 0.0,
            y0: 100.0,
            px_per_unit: 1.0,
            width: 400.0,
            height: 100.0,
        };
        // Drawn right to left: the letters still read left to right.
        let path = vec![Vec2::new(390.0, 50.0), Vec2::new(10.0, 50.0)];
        let cands = vec![Candidate {
            text: "Atatürk Bulvarı".into(),
            style: style(3),
            place: Placement::Line {
                path,
                spacing: 1000.0,
                center: false,
            },
            tile: 0,
        }];
        let mut out = Vec::new();
        place(&cands, &screen, &mut out);
        assert_eq!(out.len(), 1);
        let g = &out[0].glyphs;
        assert_eq!(g.len(), "Atatürk Bulvarı".chars().count());
        assert!(g.windows(2).all(|w| w[1].x > w[0].x));
        assert!(
            g.iter()
                .all(|x| x.angle.abs() < 1e-12 && (x.y - 50.0).abs() < 1e-12)
        );
        let wrapped = wrap(
            "Kızılay Meydanı Güvenpark",
            &LabelStyle {
                max_width: 6.0,
                ..style(1)
            },
        );
        assert!(wrapped.len() >= 2);
    }
}
