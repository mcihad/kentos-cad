//! A text's face (docs/adr/0183 §2), the contract's `TextFace`: the style it
//! follows (an id the core carries and does not read), its typeface, bold,
//! italic and slant. With a typeface the text is measured in it (bold in the
//! bold table) and its box leans with its letters; without one it is measured
//! in the project's typeface, as texts always were.

use super::Font;
use crate::api::json::{Flat, Json, field, write_str};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Face {
    pub style: Option<String>,
    pub font: Option<Font>,
    pub bold: bool,
    pub italic: bool,
    /// Degrees, positive with the letters' tops to the right; never 0.
    pub oblique: Option<f64>,
}

impl Face {
    /// The typeface it is measured in: its own, else `drawing` (the project's).
    pub fn font_or(&self, drawing: Font) -> Font {
        self.font.unwrap_or(drawing)
    }

    /// Its bold, which counts only with a typeface of its own.
    pub fn is_bold(&self) -> bool {
        self.font.is_some() && self.bold
    }

    /// How far a point `up` metres over the baseline moves along it: the
    /// slant's tangent times `up` (0 without a typeface or a slant).
    pub fn lean(&self) -> f64 {
        match (self.font, self.oblique) {
            (Some(_), Some(o)) => crate::jsmath::tan(o * crate::jsmath::PI / 180.0),
            _ => 0.0,
        }
    }
}

impl Flat for Face {
    const NAMES: &'static [&'static str] = &["textStyle", "font", "bold", "italic", "oblique"];

    fn read_flat(v: &Json) -> Result<Face, String> {
        let text = |k: &str| match v.get(k) {
            Json::Null => Ok(None),
            Json::Str(s) => Ok(Some(s.clone())),
            _ => Err(format!("“{k}”: metin bekleniyordu")),
        };
        let flag = |k: &str| match v.get(k) {
            Json::Null => Ok(false),
            Json::Bool(b) => Ok(*b),
            _ => Err(format!("“{k}”: doğru ya da yanlış bekleniyordu")),
        };
        let oblique = match v.get("oblique") {
            Json::Null => None,
            Json::Num(x) => Some(*x),
            _ => return Err("“oblique”: sayı bekleniyordu".into()),
        };
        Ok(Face {
            style: text("textStyle")?,
            font: text("font")?.map(|f| Font::from_id(&f)),
            bold: flag("bold")?,
            italic: flag("italic")?,
            oblique,
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        field(out, first, "textStyle", &self.style);
        if let Some(f) = self.font {
            if !*first {
                out.push(',');
            }
            *first = false;
            write_str(out, "font");
            out.push(':');
            write_str(out, f.id());
        }
        if self.bold {
            field(out, first, "bold", &true);
        }
        if self.italic {
            field(out, first, "italic", &true);
        }
        field(out, first, "oblique", &self.oblique);
    }
}
