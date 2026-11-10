//! The project's table styles as DXF TABLESTYLE objects (docs/adr/1128 §3), in the ACAD_TABLESTYLE dictionary under
//! their names. The groups follow Autodesk's DXF reference for TABLESTYLE: version, description, flow direction,
//! flags, the table's horizontal and vertical cell margins, the title and heading suppressed flags, then a cell
//! style each for the data, the title and the heading rows (that order: Open Design's specification, as LibreDWG's
//! `dwg2.spec` reads it): text style name, text height, cell alignment, text colour, fill colour and whether it is
//! on, data and unit type, and per border (top, horizontal inside, bottom, left, vertical inside, right) its
//! lineweight, visibility and colour. Sizes are drawing units at the plot scale (paper mm in a table style of
//! KentOS's); colours ACI. What these groups cannot say exactly (one margin pair for all three cell styles, an RGB
//! colour, a weight between DXF's) is said in the report; the KENTOS data of each object carries the style
//! itself, so KentOS reads it back exactly.

use kentos_contracts::{TableAlign, TableCellStyleDef, TableStyleDef, TextStyleDef};

use super::Out;
use super::layers::line_weight;
use super::styles::StyleNames;
use crate::dxf::{aci, xdata};
use crate::report::Report;

/// DXF tables' metric default cell margin, paper mm (the table style dialog's 1.5).
const DEFAULT_MARGIN_MM: f64 = 1.5;
/// AcValue's general data type (90) and the unitless unit type (91).
const GENERAL: i64 = 512;
const UNITLESS: i64 = 0;
/// ByBlock: a colour (0) and a lineweight (-2) the cell does not set.
const BY_BLOCK: i64 = 0;
const WEIGHT_BY_BLOCK: i64 = -2;
/// The ACAD_TABLESTYLE dictionary's handle (template.rs).
const DICTIONARY: &str = "15";

/// The ACAD_TABLESTYLE dictionary's entries: each style's name (as a DXF name) and its object's handle.
pub fn entries(styles: &[TableStyleDef], names: &StyleNames) -> Vec<(String, String)> {
    styles
        .iter()
        .filter_map(|s| Some((super::layers::valid_name(&s.name), format!("{:X}", names.table_handle(&s.id)?))))
        .collect()
}

/// A colour of a cell style as ACI (0, ByBlock, when it sets none), and whether the number changes it.
fn colour(c: Option<&str>) -> (i64, bool) {
    match c {
        None => (BY_BLOCK, false),
        Some(c) => {
            let (dxf, _) = aci::from_app(c);
            (i64::from(dxf.aci), aci::color(dxf.aci).to_lowercase() != c.to_lowercase())
        }
    }
}

/// Cell alignment (170): KentOS's cells are middle aligned; a cell style without one takes the row's own (left for
/// data, centred for the title and the heading).
fn alignment(a: Option<TableAlign>, data: bool) -> i64 {
    match a {
        Some(TableAlign::Left) => 4,
        Some(TableAlign::Center) => 5,
        Some(TableAlign::Right) => 6,
        None if data => 4,
        None => 5,
    }
}

/// Writes each style's TABLESTYLE object; what the groups round is said in `report`.
pub fn write(
    out: &mut Out,
    styles: &[TableStyleDef],
    names: &StyleNames,
    text_styles: &[TextStyleDef],
    paper: f64,
    report: &mut Report,
) {
    // Paper mm in drawing units at the plot scale.
    let length = |mm: f64| mm / 1000.0 * paper;
    for s in styles {
        let Some(handle) = names.table_handle(&s.id) else { continue };
        let mut lost: Vec<&str> = Vec::new();
        out.str(0, "TABLESTYLE");
        out.handle(5, handle);
        out.str(102, "{ACAD_REACTORS");
        out.str(330, DICTIONARY);
        out.str(102, "}");
        out.str(330, DICTIONARY);
        out.str(100, "AcDbTableStyle");
        out.int(280, 0);
        out.str(3, &super::blocks::one_line(&s.name));
        out.int(70, 0);
        out.int(71, 0);
        // One margin pair for the table: the data cells', else the title's, else the heading's.
        let cells = [&s.data, &s.title, &s.header];
        let margin = |f: fn(&TableCellStyleDef) -> Option<f64>| {
            let given: Vec<f64> = cells.iter().filter_map(|c| c.as_ref().and_then(f)).collect();
            (given.first().copied(), given.windows(2).any(|w| w[0] != w[1]))
        };
        let (h, h_differ) = margin(|c| c.horizontal_margin);
        let (v, v_differ) = margin(|c| c.vertical_margin);
        if h_differ || v_differ {
            lost.push("hücre stillerinin farklı kenar boşlukları tek değer olarak yazıldı (veri hücresininki)");
        }
        out.real(40, length(h.unwrap_or(DEFAULT_MARGIN_MM)));
        out.real(41, length(v.unwrap_or(DEFAULT_MARGIN_MM)));
        out.int(280, 0);
        out.int(281, 0);
        for (i, cell) in cells.iter().enumerate() {
            let default = TableCellStyleDef::default();
            let c = cell.as_ref().unwrap_or(&default);
            let style = c
                .text_style
                .as_deref()
                .and_then(|n| names.text_named(text_styles, n))
                .unwrap_or("Standard");
            if c.text_style.is_some() && style == "Standard" {
                lost.push("projede olmayan yazı stili Standard yazıldı");
            }
            out.str(7, style);
            // A cell style without a height: the table's own; the format's data and heading default (0,18 in), the
            // title's (0,25 in) as paper mm.
            out.real(140, length(c.height.unwrap_or(if i == 1 { 6.35 } else { 4.572 })));
            out.int(170, alignment(c.align, i == 0));
            let (text, text_changed) = colour(c.color.as_deref());
            out.int(62, text);
            let (fill, fill_changed) = match c.fill.as_deref() {
                Some(f) => colour(Some(f)),
                None => (7, false),
            };
            out.int(63, fill);
            out.int(283, i64::from(c.fill.is_some()));
            out.int(90, GENERAL);
            out.int(91, UNITLESS);
            let (weight, weight_changed) = match c.border_weight {
                Some(w) => line_weight(w),
                None => (WEIGHT_BY_BLOCK, false),
            };
            let (border, border_changed) = colour(c.border_color.as_deref());
            for k in 0..6 {
                out.int(274 + k, weight);
            }
            for k in 0..6 {
                out.int(284 + k, 1);
            }
            for k in 0..6 {
                out.int(64 + k, border);
            }
            if text_changed || fill_changed || border_changed {
                lost.push("renkler en yakın ACI rengi olarak yazıldı");
            }
            if weight_changed {
                lost.push("kenar kalınlığı DXF'in en yakın çizgi kalınlığı olarak yazıldı");
            }
        }
        let meta = xdata::Meta { table_style: serde_json::to_string(s).ok(), ..Default::default() };
        out.xdata(&xdata::groups(&meta));
        lost.sort_unstable();
        lost.dedup();
        for why in lost {
            report.note("Tablo stili", &format!("“{}”: {why}; KentOS verisi stili tam taşır", s.name), 0);
        }
    }
}
