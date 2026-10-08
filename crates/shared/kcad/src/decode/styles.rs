//! Schema 21's named text and dimension styles (docs/adr/0183, §6.2 of the
//! spec): each its id, name and values, a false flag not written; the list
//! checked whole by the contract's rule (`text_styles_problem`,
//! `dimension_styles_problem`): no empty or repeated id, no empty, padded,
//! repeated (whatever the letters' case) or reserved name, values in their
//! bounds.

use kentos_contracts::{
    DimensionArrow, DimensionStyleDef, DimensionTextPlace, DrawingFont, DrawingUnit, TextStyleDef,
    dimension_styles_problem, text_styles_problem,
};

use super::{list, map, named, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

fn font(r: &mut Reader<'_>) -> Result<DrawingFont, KcadError> {
    let names: Vec<(&str, DrawingFont)> = DrawingFont::ALL.iter().map(|d| (d.id(), *d)).collect();
    named(r, &names)
}

/// A flag a style writes only when true.
fn flag(r: &mut Reader<'_>, name: &str) -> Result<bool, KcadError> {
    let at = r.position();
    if !r.bool()? {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("{name} false yazılmaz; alan yoksa stil öyle değildir"),
        ));
    }
    Ok(true)
}

pub(super) fn text_styles(r: &mut Reader<'_>) -> Result<Vec<TextStyleDef>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (mut id, mut name, mut face) = (None, None, None);
        let mut style = TextStyleDef {
            id: String::new(),
            name: String::new(),
            font: DrawingFont::Barlow,
            bold: false,
            italic: false,
            oblique: None,
            height: None,
            width_factor: None,
            font_file: None,
        };
        map(r, |r, key| {
            match key {
                "id" => id = Some(text(r)?),
                "name" => name = Some(text(r)?),
                "font" => face = Some(font(r)?),
                "bold" => style.bold = flag(r, "bold")?,
                "italic" => style.italic = flag(r, "italic")?,
                "oblique" => style.oblique = Some(r.float()?),
                "height" => style.height = Some(r.float()?),
                "widthFactor" => style.width_factor = Some(r.float()?),
                "fontFile" => style.font_file = Some(text(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        style.id = required(r, id, "id")?;
        style.name = required(r, name, "name")?;
        style.font = required(r, face, "font")?;
        Ok(style)
    })?;
    match text_styles_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}

/// The dimension styles; `lines` says whether the schema has their line
/// fields (30 and up, docs/adr/0205 §6).
pub(super) fn dimension_styles(
    r: &mut Reader<'_>,
    lines: bool,
) -> Result<Vec<DimensionStyleDef>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (mut id, mut name, mut height) = (None, None, None);
        let mut style = DimensionStyleDef {
            id: String::new(),
            name: String::new(),
            height: 0.0,
            arrow: None,
            arrow_size: None,
            ext_offset: None,
            ext_beyond: None,
            text_gap: None,
            text_place: None,
            decimals: None,
            unit: None,
            prefix: None,
            suffix: None,
            font: None,
            dim_line_color: None,
            dim_line_weight: None,
            dim_line_type: None,
            ext_color: None,
            ext_weight: None,
            ext_line_type: None,
            text_color: None,
        };
        map(r, |r, key| {
            match key {
                "id" => id = Some(text(r)?),
                "name" => name = Some(text(r)?),
                "height" => height = Some(r.float()?),
                "arrow" => {
                    let names: Vec<(&str, DimensionArrow)> =
                        DimensionArrow::ALL.iter().map(|a| (a.name(), *a)).collect();
                    style.arrow = Some(named(r, &names)?)
                }
                "arrowSize" => style.arrow_size = Some(r.float()?),
                "extOffset" => style.ext_offset = Some(r.float()?),
                "extBeyond" => style.ext_beyond = Some(r.float()?),
                "textGap" => style.text_gap = Some(r.float()?),
                "textPlace" => {
                    style.text_place = Some(named(r, &[("centre", DimensionTextPlace::Centre)])?)
                }
                "decimals" => style.decimals = Some(r.uint(u64::from(u32::MAX))? as u32),
                "unit" => {
                    style.unit = Some(named(
                        r,
                        &[
                            ("mm", DrawingUnit::Mm),
                            ("cm", DrawingUnit::Cm),
                            ("m", DrawingUnit::M),
                        ],
                    )?)
                }
                "prefix" => style.prefix = Some(text(r)?),
                "suffix" => style.suffix = Some(text(r)?),
                "font" => style.font = Some(font(r)?),
                "dimLineColor" if lines => style.dim_line_color = Some(text(r)?),
                "extColor" if lines => style.ext_color = Some(text(r)?),
                "textColor" if lines => style.text_color = Some(text(r)?),
                "dimLineWeight" if lines => style.dim_line_weight = Some(r.float()?),
                "extWeight" if lines => style.ext_weight = Some(r.float()?),
                "dimLineType" if lines => style.dim_line_type = Some(super::line_type_named(r)?),
                "extLineType" if lines => style.ext_line_type = Some(super::line_type_named(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        style.id = required(r, id, "id")?;
        style.name = required(r, name, "name")?;
        style.height = required(r, height, "height")?;
        Ok(style)
    })?;
    match dimension_styles_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}
