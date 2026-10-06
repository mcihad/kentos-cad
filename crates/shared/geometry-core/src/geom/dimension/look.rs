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
}

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
    }
}
