//! A dimension's look (docs/adr/0183 §3), the contract's `DimensionLook`:
//! its arrowheads, its parts' sizes as multiples of its value's height and
//! where its value stands, which the layout reads; the style it follows,
//! the value's decimals, unit, prefix, suffix and typeface, which the core
//! carries for its owner (the host writes the value).

use crate::api::json::{Flat, Json, field, write_str};
use crate::text::Font;

/// A dimension's sizes when it names none, times its value's height: the
/// look dimensions always had (docs/adr/0147).
pub const DEFAULT_TICK: f64 = 0.6;
pub const DEFAULT_ARROW: f64 = 1.0;
pub const DEFAULT_EXT_OFFSET: f64 = 0.5;
pub const DEFAULT_EXT_BEYOND: f64 = 0.5;
pub const DEFAULT_TEXT_GAP: f64 = 0.35;

/// The arrowheads other than the oblique tick (the field's absence).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrow {
    Closed,
    Open,
    Dot,
    None,
}

impl Arrow {
    pub const ALL: [Arrow; 4] = [Arrow::Closed, Arrow::Open, Arrow::Dot, Arrow::None];

    pub fn name(self) -> &'static str {
        match self {
            Arrow::Closed => "closed",
            Arrow::Open => "open",
            Arrow::Dot => "dot",
            Arrow::None => "none",
        }
    }

    pub fn from_name(name: &str) -> Option<Arrow> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    pub style: Option<String>,
    pub arrow: Option<Arrow>,
    pub arrow_size: Option<f64>,
    pub ext_offset: Option<f64>,
    pub ext_beyond: Option<f64>,
    pub text_gap: Option<f64>,
    /// The value's middle on the line, the line hidden under it (`textPlace: "centre"`).
    pub centre: bool,
    pub decimals: Option<u32>,
    /// `m`, `cm` or `mm`, as the contract's `DrawingUnit` names it.
    pub unit: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub font: Option<Font>,
    /// Its lines (docs/adr/0205 §6); none: the object's colour, hairlines,
    /// continuous.
    pub lines: Option<Box<LookLines>>,
    /// Its options, its moved value and its breaks (docs/adr/0321); boxed, as few dimensions have them.
    pub more: Option<Box<LookMore>>,
}

/// Where a break is: 0 the dimension line, 1 the first extension line, 2 the second; the fraction along it.
pub type LookBreak = (u8, f64);

/// A dimension's options (the contract's `DimensionOptions`, carried as its JSON for its owner, the layout reading
/// `fit` and `breakSize`), its value moved along and across its own direction, and its breaks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LookMore {
    pub options: Option<Json>,
    pub shift: Option<(f64, f64)>,
    pub breaks: Vec<LookBreak>,
}

impl LookMore {
    /// Its box, or none when it says nothing.
    pub fn boxed(self) -> Option<Box<LookMore>> {
        (self != LookMore::default()).then(|| Box::new(self))
    }
}

/// The names of a break's lines, by their number.
pub const BREAK_LINES: [&str; 3] = ["dimension", "first", "second"];

/// A dimension's lines (docs/adr/0205 §6), as the contract names them: the
/// dimension line's and arrowheads' colour (`#RRGGBB`), weight (paper mm)
/// and type (`continuous`, `dashed`, `dashdot`, `dotted`); the extension
/// lines'; the value's colour. Boxed in its look: few dimensions have them,
/// and every shape is as large as its largest kind.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LookLines {
    pub dim_line_color: Option<String>,
    pub dim_line_weight: Option<f64>,
    pub dim_line_type: Option<String>,
    pub ext_color: Option<String>,
    pub ext_weight: Option<f64>,
    pub ext_line_type: Option<String>,
    pub text_color: Option<String>,
}

impl LookLines {
    /// Naming nothing (a look without lines of its own).
    pub const NONE: LookLines = LookLines {
        dim_line_color: None,
        dim_line_weight: None,
        dim_line_type: None,
        ext_color: None,
        ext_weight: None,
        ext_line_type: None,
        text_color: None,
    };

    /// Its box, or none when it names nothing.
    pub fn boxed(self) -> Option<Box<LookLines>> {
        (self != LookLines::default()).then(|| Box::new(self))
    }
}

impl Look {
    /// The arrowhead's (or tick's) length, times the value's height.
    pub fn arrow_size(&self) -> f64 {
        self.arrow_size.unwrap_or(if self.arrow.is_none() {
            DEFAULT_TICK
        } else {
            DEFAULT_ARROW
        })
    }

    pub fn ext_offset(&self) -> f64 {
        self.ext_offset.unwrap_or(DEFAULT_EXT_OFFSET)
    }

    pub fn ext_beyond(&self) -> f64 {
        self.ext_beyond.unwrap_or(DEFAULT_EXT_BEYOND)
    }

    pub fn text_gap(&self) -> f64 {
        self.text_gap.unwrap_or(DEFAULT_TEXT_GAP)
    }

    /// Its options' `fit`: `auto`, `outside`, or none (between the extension lines).
    pub fn fit(&self) -> Option<&str> {
        match self.more.as_deref()?.options.as_ref()?.get("fit") {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// A break's width, times the value's height.
    pub fn break_size(&self) -> f64 {
        let size = self
            .more
            .as_deref()
            .and_then(|m| m.options.as_ref())
            .map(|o| o.get("breakSize").clone());
        match size {
            Some(Json::Num(x)) => x,
            _ => DEFAULT_BREAK_SIZE,
        }
    }

    /// The value moved along and across its own direction, metres.
    pub fn shift(&self) -> Option<(f64, f64)> {
        self.more.as_deref()?.shift
    }

    pub fn breaks(&self) -> &[LookBreak] {
        self.more.as_deref().map_or(&[], |m| m.breaks.as_slice())
    }
}

/// A break's default width, times the value's height (docs/adr/0321 §1).
pub const DEFAULT_BREAK_SIZE: f64 = 1.0;

impl Flat for Look {
    const NAMES: &'static [&'static str] = &[
        "dimStyle",
        "arrow",
        "arrowSize",
        "extOffset",
        "extBeyond",
        "textGap",
        "textPlace",
        "decimals",
        "unit",
        "prefix",
        "suffix",
        "font",
        "dimLineColor",
        "dimLineWeight",
        "dimLineType",
        "extColor",
        "extWeight",
        "extLineType",
        "textColor",
        "options",
        "textShift",
        "breaks",
    ];

    fn read_flat(v: &Json) -> Result<Look, String> {
        let text = |k: &str| match v.get(k) {
            Json::Null => Ok(None),
            Json::Str(s) => Ok(Some(s.clone())),
            _ => Err(format!("“{k}”: metin bekleniyordu")),
        };
        let number = |k: &str| match v.get(k) {
            Json::Null => Ok(None),
            Json::Num(x) => Ok(Some(*x)),
            _ => Err(format!("“{k}”: sayı bekleniyordu")),
        };
        let arrow = match text("arrow")? {
            None => None,
            Some(name) => Some(
                Arrow::from_name(&name)
                    .ok_or_else(|| format!("“arrow”: bilinmeyen ok “{name}”"))?,
            ),
        };
        let decimals = match number("decimals")? {
            None => None,
            Some(d) if d >= 0.0 && d.fract() == 0.0 && d <= f64::from(u32::MAX) => Some(d as u32),
            Some(d) => return Err(format!("“decimals”: {d} basamak sayısı değil")),
        };
        Ok(Look {
            style: text("dimStyle")?,
            arrow,
            arrow_size: number("arrowSize")?,
            ext_offset: number("extOffset")?,
            ext_beyond: number("extBeyond")?,
            text_gap: number("textGap")?,
            centre: text("textPlace")?.as_deref() == Some("centre"),
            decimals,
            unit: text("unit")?,
            prefix: text("prefix")?,
            suffix: text("suffix")?,
            font: text("font")?.map(|f| Font::from_id(&f)),
            lines: LookLines {
                dim_line_color: text("dimLineColor")?,
                dim_line_weight: number("dimLineWeight")?,
                dim_line_type: text("dimLineType")?,
                ext_color: text("extColor")?,
                ext_weight: number("extWeight")?,
                ext_line_type: text("extLineType")?,
                text_color: text("textColor")?,
            }
            .boxed(),
            more: read_more(v)?,
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        let name = |out: &mut String, first: &mut bool, k: &str, v: &str| {
            if !*first {
                out.push(',');
            }
            *first = false;
            write_str(out, k);
            out.push(':');
            write_str(out, v);
        };
        field(out, first, "dimStyle", &self.style);
        if let Some(a) = self.arrow {
            name(out, first, "arrow", a.name());
        }
        field(out, first, "arrowSize", &self.arrow_size);
        field(out, first, "extOffset", &self.ext_offset);
        field(out, first, "extBeyond", &self.ext_beyond);
        field(out, first, "textGap", &self.text_gap);
        if self.centre {
            name(out, first, "textPlace", "centre");
        }
        field(out, first, "decimals", &self.decimals.map(f64::from));
        field(out, first, "unit", &self.unit);
        field(out, first, "prefix", &self.prefix);
        field(out, first, "suffix", &self.suffix);
        if let Some(f) = self.font {
            name(out, first, "font", f.id());
        }
        if let Some(l) = &self.lines {
            field(out, first, "dimLineColor", &l.dim_line_color);
            field(out, first, "dimLineWeight", &l.dim_line_weight);
            field(out, first, "dimLineType", &l.dim_line_type);
            field(out, first, "extColor", &l.ext_color);
            field(out, first, "extWeight", &l.ext_weight);
            field(out, first, "extLineType", &l.ext_line_type);
            field(out, first, "textColor", &l.text_color);
        }
        if let Some(m) = &self.more {
            field(out, first, "options", &m.options);
            if let Some((along, across)) = m.shift {
                let mut text = String::from("{");
                let mut f = true;
                field(&mut text, &mut f, "along", &along);
                field(&mut text, &mut f, "across", &across);
                text.push('}');
                if !*first {
                    out.push(',');
                }
                *first = false;
                write_str(out, "textShift");
                out.push(':');
                out.push_str(&text);
            }
            if !m.breaks.is_empty() {
                if !*first {
                    out.push(',');
                }
                *first = false;
                write_str(out, "breaks");
                out.push_str(":[");
                for (i, (line, t)) in m.breaks.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('{');
                    let mut f = true;
                    field(out, &mut f, "line", BREAK_LINES[usize::from(*line).min(2)]);
                    field(out, &mut f, "t", t);
                    out.push('}');
                }
                out.push(']');
            }
        }
    }
}

/// The options, the moved value and the breaks of a dimension's JSON (docs/adr/0321).
fn read_more(v: &Json) -> Result<Option<Box<LookMore>>, String> {
    let options = match v.get("options") {
        Json::Null => None,
        o @ Json::Obj(_) => Some(o.clone()),
        _ => return Err("“options”: nesne bekleniyordu".into()),
    };
    let shift = match v.get("textShift") {
        Json::Null => None,
        s => match (s.get("along"), s.get("across")) {
            (Json::Num(a), Json::Num(b)) => Some((*a, *b)),
            _ => return Err("“textShift”: along ve across sayı olmalı".into()),
        },
    };
    let breaks = match v.get("breaks") {
        Json::Null => Vec::new(),
        Json::Arr(items) => items
            .iter()
            .map(|b| match (b.get("line"), b.get("t")) {
                (Json::Str(line), Json::Num(t)) => BREAK_LINES
                    .iter()
                    .position(|n| n == line)
                    .map(|k| (k as u8, *t))
                    .ok_or_else(|| format!("“breaks”: bilinmeyen çizgi “{line}”")),
                _ => Err("“breaks”: line ve t bekleniyordu".into()),
            })
            .collect::<Result<_, _>>()?,
        _ => return Err("“breaks”: dizi bekleniyordu".into()),
    };
    Ok(LookMore {
        options,
        shift,
        breaks,
    }
    .boxed())
}
