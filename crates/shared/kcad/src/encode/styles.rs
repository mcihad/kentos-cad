//! Schema 21's named text and dimension styles (docs/adr/0183, §6.2 of the
//! spec), checked whole as a reader checks them (`text_styles_problem`,
//! `dimension_styles_problem`); each style a map in encoded key order, a
//! false flag and an absent value not written.

use kentos_contracts::{
    DimensionStyleDef, TextStyleDef, dimension_styles_problem, text_styles_problem,
};

use super::Encoder;
use super::objects::Val;
use crate::cbor::{Seg, key_order};
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    /// One style's fields, sorted as the file keeps them.
    fn style_map(&mut self, mut f: Vec<(&'static str, Val<'d>)>) -> Result<(), KcadError> {
        f.sort_by(|a, b| key_order(a.0, b.0));
        self.open(f.len(), true)?;
        for (k, v) in f {
            self.key(k);
            self.at(Seg::Name(k), |e| e.val(v))?;
        }
        self.close();
        Ok(())
    }

    pub(super) fn text_styles(&mut self, styles: &'d [TextStyleDef]) -> Result<(), KcadError> {
        if let Some(problem) = text_styles_problem(styles) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(styles.len(), false)?;
        for (i, s) in styles.iter().enumerate() {
            let mut f = vec![
                ("id", Val::Text(&s.id)),
                ("name", Val::Text(&s.name)),
                ("font", Val::Name(s.font.id())),
            ];
            if s.bold {
                f.push(("bold", Val::Bool(true)));
            }
            if s.italic {
                f.push(("italic", Val::Bool(true)));
            }
            for (key, v) in [
                ("oblique", s.oblique),
                ("height", s.height),
                ("widthFactor", s.width_factor),
            ] {
                if let Some(x) = v {
                    f.push((key, Val::Float(x)));
                }
            }
            if let Some(file) = &s.font_file {
                f.push(("fontFile", Val::Text(file)));
            }
            self.at(Seg::Index(i), |e| e.style_map(f))?;
        }
        self.close();
        Ok(())
    }

    pub(super) fn dimension_styles(
        &mut self,
        styles: &'d [DimensionStyleDef],
    ) -> Result<(), KcadError> {
        if let Some(problem) = dimension_styles_problem(styles) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(styles.len(), false)?;
        for (i, s) in styles.iter().enumerate() {
            let mut f = vec![
                ("id", Val::Text(&s.id)),
                ("name", Val::Text(&s.name)),
                ("height", Val::Float(s.height)),
            ];
            if let Some(a) = s.arrow {
                f.push(("arrow", Val::Name(a.name())));
            }
            for (key, v) in [
                ("arrowSize", s.arrow_size),
                ("extOffset", s.ext_offset),
                ("extBeyond", s.ext_beyond),
                ("textGap", s.text_gap),
            ] {
                if let Some(x) = v {
                    f.push((key, Val::Float(x)));
                }
            }
            if s.text_place.is_some() {
                f.push(("textPlace", Val::Name("centre")));
            }
            if let Some(d) = s.decimals {
                f.push(("decimals", Val::Uint(u64::from(d))));
            }
            if let Some(u) = s.unit {
                f.push(("unit", Val::Name(u.mark())));
            }
            if let Some(t) = &s.prefix {
                f.push(("prefix", Val::Text(t)));
            }
            if let Some(t) = &s.suffix {
                f.push(("suffix", Val::Text(t)));
            }
            if let Some(font) = s.font {
                f.push(("font", Val::Name(font.id())));
            }
            self.at(Seg::Index(i), |e| e.style_map(f))?;
        }
        self.close();
        Ok(())
    }
}
