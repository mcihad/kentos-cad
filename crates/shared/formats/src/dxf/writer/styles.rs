//! The project's text and dimension styles in a DXF (docs/adr/0183 §7):
//! each text style a STYLE record (its typeface's file and ACAD's family,
//! bold and italic flags; its fixed height in drawing units, width and
//! slant), each dimension style a DIMSTYLE record (its value's height,
//! arrowheads as DIMTSZ ticks or the filled arrow, sizes, the value's place,
//! decimals, prefix and suffix, unit). A text with a typeface of its own and
//! no style of the project names a `KENTOS_‹AİLE›[_B][_I]` record, its KENTOS
//! data the mark of no style (`styleless`). A project's style's record
//! carries the style itself in its KENTOS data, so KentOS reads it back
//! exactly. Standard is Standart, written as before styles: a drawing
//! without styles is the file it was.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{
    DimensionArrow, DimensionStyleDef, DimensionTextPlace, DrawingFont, DrawingUnit, Entity,
    TextFace, TextStyleDef,
};

use super::Out;
use super::template::{record, table};
use crate::num::dxf_real;

/// ACAD's flags of a TrueType style's extended data (1071): italic, bold,
/// and the pitch and family AutoCAD writes for a TrueType face (34).
const ACAD_ITALIC: i64 = 0x0100_0000;
const ACAD_BOLD: i64 = 0x0200_0000;
const ACAD_TRUETYPE: i64 = 34;

/// A family's file as AutoCAD names Arial's and Courier's, KentOS's others as `‹Aile›.ttf`.
pub fn font_file(font: DrawingFont, bold: bool, italic: bool) -> String {
    let style = match (bold, italic) {
        (false, false) => "",
        (true, false) => "bd",
        (false, true) => "i",
        (true, true) => "bi",
    };
    match font {
        DrawingFont::Arimo => format!("arial{style}.ttf"),
        DrawingFont::CourierPrime => format!("cour{style}.ttf"),
        other => format!("{}.ttf", other.label().replace(' ', "")),
    }
}

/// A family as ACAD's data names it (what AutoCAD shows and maps to an installed face).
fn family(font: DrawingFont) -> &'static str {
    match font {
        DrawingFont::Arimo => "Arial",
        DrawingFont::CourierPrime => "Courier New",
        other => other.label(),
    }
}

/// A name a DXF symbol table takes: no `<>/\":;?*|=,` and backquote, trimmed, not empty.
fn table_name(name: &str) -> String {
    let n: String = name
        .trim()
        .chars()
        .map(|c| {
            if "<>/\\\":;?*|=,`".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    if n.is_empty() { "Stil".to_owned() } else { n }
}

/// The records' names, by the style's id (and a styleless face's typeface).
#[derive(Default)]
pub struct StyleNames {
    text: HashMap<String, String>,
    synthetic: BTreeMap<(usize, bool, bool), String>,
    dims: HashMap<String, String>,
}

impl StyleNames {
    /// The names of `text_styles` and `dimension_styles`, each unique (case
    /// aside) and not Standard's, and those the styleless faces of `entities`
    /// and the typefaces of their dimensions' values need.
    pub fn new<'a>(
        text_styles: &[TextStyleDef],
        dimension_styles: &[DimensionStyleDef],
        entities: impl Iterator<Item = &'a Entity>,
    ) -> StyleNames {
        let mut taken: Vec<String> = vec!["STANDARD".to_owned()];
        let mut unique = |name: &str| {
            let base = table_name(name);
            let mut n = base.clone();
            let mut i = 2;
            while taken.contains(&n.to_uppercase()) {
                n = format!("{base} {i}");
                i += 1;
            }
            taken.push(n.to_uppercase());
            n
        };
        let mut names = StyleNames::default();
        for s in text_styles {
            names.text.insert(s.id.clone(), unique(&s.name));
        }
        for s in dimension_styles {
            names.dims.insert(s.id.clone(), unique(&s.name));
        }
        for e in entities {
            let face = match e {
                Entity::Text(kentos_contracts::TextEntity { face, .. })
                | Entity::Table(kentos_contracts::TableEntity { face, .. })
                    if face
                        .text_style
                        .as_ref()
                        .is_none_or(|id| !names.text.contains_key(id)) =>
                {
                    face.font.map(|f| (f, face.bold, face.italic))
                }
                Entity::Dimension(d) => d.look.font.map(|f| (f, false, false)),
                _ => None,
            };
            let Some((font, bold, italic)) = face else {
                continue;
            };
            let key = (
                DrawingFont::ALL
                    .iter()
                    .position(|f| *f == font)
                    .unwrap_or(0),
                bold,
                italic,
            );
            names.synthetic.entry(key).or_insert_with(|| {
                let mut n = format!("KENTOS_{}", font.id().replace('-', "_").to_uppercase());
                if bold {
                    n.push_str("_B");
                }
                if italic {
                    n.push_str("_I");
                }
                unique(&n)
            });
        }
        names
    }

    /// The STYLE a dimension's value names in its block (7): its typeface's record, or Standard.
    pub fn value(&self, font: Option<DrawingFont>) -> &str {
        font.and_then(|font| {
            self.synthetic.get(&(
                DrawingFont::ALL
                    .iter()
                    .position(|f| *f == font)
                    .unwrap_or(0),
                false,
                false,
            ))
        })
        .map_or("Standard", String::as_str)
    }

    /// The STYLE a text names (7): its style's, its typeface's, or Standard.
    pub fn text(&self, face: &TextFace) -> &str {
        if let Some(n) = face.text_style.as_ref().and_then(|id| self.text.get(id)) {
            return n;
        }
        match face.font {
            Some(font) => self
                .synthetic
                .get(&(
                    DrawingFont::ALL
                        .iter()
                        .position(|f| *f == font)
                        .unwrap_or(0),
                    face.bold,
                    face.italic,
                ))
                .map_or("Standard", String::as_str),
            None => "Standard",
        }
    }

    /// The DIMSTYLE a dimension names (3): its style's, or Standard.
    pub fn dimension(&self, id: Option<&String>) -> &str {
        id.and_then(|id| self.dims.get(id))
            .map_or("Standard", String::as_str)
    }
}

/// One STYLE record's groups (after its head): name, flags, sizes, file.
fn style_groups(out: &mut Out, name: &str, height: f64, width: f64, oblique: f64, file: &str) {
    out.str(2, name);
    out.int(70, 0);
    out.real(40, height);
    out.real(41, width);
    out.real(50, oblique);
    out.int(71, 0);
    out.real(42, 2.5);
    out.str(3, file);
    out.str(4, "");
}

/// A STYLE record's extended data: ACAD's family and flags, KENTOS's style (or the mark of none).
fn style_data(out: &mut Out, font: DrawingFont, bold: bool, italic: bool, kentos: &str) {
    let flags =
        ACAD_TRUETYPE | if bold { ACAD_BOLD } else { 0 } | if italic { ACAD_ITALIC } else { 0 };
    let mut x = vec![
        (1001, "ACAD".to_owned()),
        (1000, family(font).to_owned()),
        (1071, flags.to_string()),
    ];
    x.extend(super::super::xdata::groups(&super::super::xdata::Meta {
        face: Some(kentos.to_owned()),
        ..Default::default()
    }));
    out.xdata(&x);
}

/// The STYLE table: Standard (Arial, every Turkish letter; as before
/// styles), the project's styles, the styleless faces' records. `length`
/// turns paper mm into drawing units.
pub fn style_table(
    out: &mut Out,
    handles: &mut super::Handles,
    names: &StyleNames,
    styles: &[TextStyleDef],
    length: &dyn Fn(f64) -> f64,
) {
    table(
        out,
        "STYLE",
        super::template::STYLE_TABLE,
        1 + styles.len() + names.synthetic.len(),
    );
    record(
        out,
        "STYLE",
        super::template::STYLE,
        super::template::STYLE_TABLE,
        "AcDbTextStyleTableRecord",
    );
    style_groups(out, "Standard", 0.0, 1.0, 0.0, "arial.ttf");
    for s in styles {
        let Some(name) = names.text.get(&s.id) else {
            continue;
        };
        let h = handles.take();
        record(
            out,
            "STYLE",
            h,
            super::template::STYLE_TABLE,
            "AcDbTextStyleTableRecord",
        );
        style_groups(
            out,
            name,
            s.height.map_or(0.0, length),
            s.width_factor.unwrap_or(1.0),
            s.oblique.unwrap_or(0.0),
            &s.font_file
                .clone()
                .unwrap_or_else(|| font_file(s.font, s.bold, s.italic)),
        );
        let json = serde_json::to_string(s).unwrap_or_default();
        style_data(out, s.font, s.bold, s.italic, &json);
    }
    for ((font, bold, italic), name) in &names.synthetic {
        let font = DrawingFont::ALL
            .get(*font)
            .copied()
            .unwrap_or(DrawingFont::Barlow);
        let h = handles.take();
        record(
            out,
            "STYLE",
            h,
            super::template::STYLE_TABLE,
            "AcDbTextStyleTableRecord",
        );
        // A styleless face's record: no style of the project (its KENTOS data says none).
        style_groups(out, name, 0.0, 1.0, 0.0, &font_file(font, *bold, *italic));
        style_data(out, font, *bold, *italic, super::super::xdata::STYLELESS);
    }
    out.str(0, "ENDTAB");
}

/// What the report says of open arrows and dots: DXF's arrowhead blocks are not written.
pub const ARROW_NOTE: &str = "açık ok ve nokta DXF'in ok bloğu olmadan yazıldı (ölçü stilinde dolu ok); ölçünün kendi bloğu onları çizer, başka bir program ölçüyü yeniden çizerse dolu ok gösterir, KentOS kendi görünüşünü geri okur";

/// DIMPOST: the prefix and suffix around the number (“<>”); none without either.
pub fn dimpost(prefix: Option<&str>, suffix: Option<&str>) -> Option<String> {
    (prefix.is_some() || suffix.is_some()).then(|| {
        format!(
            "{}<>{}",
            prefix.unwrap_or_default(),
            suffix.unwrap_or_default()
        )
    })
}

/// DIMLFAC: the value's unit per the file's (`per_metre` of the file's unit
/// make a metre); none when the value is in the file's own unit.
pub fn dimlfac(unit: Option<DrawingUnit>, per_metre: f64) -> Option<f64> {
    unit.map(|u| u.per_metre() / per_metre)
        .filter(|f| *f != 1.0)
}

/// A DIMSTYLE record's variables over Standard's: its value's height,
/// arrowheads (DIMTSZ for ticks: half a 45° tick's length; DIMASZ 0 for
/// none), arrow size, extension lines' offset and reach, the value's gap and
/// place, decimals, prefix and suffix around “<>”, unit (DIMLFAC); `length`
/// turns paper mm into drawing units, `per_metre` of them make a metre.
fn dimension_vars(
    s: &DimensionStyleDef,
    length: &dyn Fn(f64) -> f64,
    per_metre: f64,
) -> Vec<(i32, String)> {
    let look = s.look();
    let h = length(s.height);
    let size = look.arrow_size_or_default() * h;
    let (asz, tsz) = match s.arrow {
        None => (size, size * std::f64::consts::FRAC_1_SQRT_2),
        Some(DimensionArrow::None) => (0.0, 0.0),
        Some(_) => (size, 0.0),
    };
    let mut v = vec![
        (40, "1.0".to_owned()),
        (140, dxf_real(h)),
        (41, dxf_real(asz)),
        (142, dxf_real(tsz)),
        (42, dxf_real(look.ext_offset_or_default() * h)),
        (44, dxf_real(look.ext_beyond_or_default() * h)),
        (147, dxf_real(look.text_gap_or_default() * h)),
        (
            77,
            if s.text_place == Some(DimensionTextPlace::Centre) {
                "0"
            } else {
                "1"
            }
            .to_owned(),
        ),
    ];
    if let Some(d) = s.decimals {
        v.push((271, d.to_string()));
    }
    if let Some(post) = dimpost(s.prefix.as_deref(), s.suffix.as_deref()) {
        v.push((3, post));
    }
    if let Some(f) = dimlfac(s.unit, per_metre) {
        v.push((144, dxf_real(f)));
    }
    v
}

/// The DIMSTYLE table: Standard (as before styles) and the project's styles.
pub fn dimstyle_table(
    out: &mut Out,
    handles: &mut super::Handles,
    names: &StyleNames,
    styles: &[DimensionStyleDef],
    length: &dyn Fn(f64) -> f64,
    per_metre: f64,
) {
    super::template::dimstyle_head(out, 1 + styles.len());
    super::template::dimstyle_standard(out);
    for s in styles {
        let Some(name) = names.dims.get(&s.id) else {
            continue;
        };
        let h = handles.take();
        let json = serde_json::to_string(s).unwrap_or_default();
        super::template::dimstyle_record(
            out,
            h,
            name,
            &dimension_vars(s, length, per_metre),
            Some(&json),
        );
    }
    out.str(0, "ENDTAB");
}
