//! Named table styles (docs/adr/1126): AutoCAD's TABLESTYLE reduced to what a KentOS table can carry. A style has
//! three cell styles, as AutoCAD's Standard has (Title, Header, Data); each may name a text style (by name), a text
//! height, an alignment, a text and a fill colour, horizontal and vertical margins and its borders' weight and colour,
//! all on the paper. A table names its style by id (`TableEntity::table_style`); where the table has none of its own,
//! the drawing takes the style's alignment, colours and the data cells' text style ([`styled`]). Heights, margins and
//! borders are kept, compared by the office standard and written back, not yet drawn (docs/adr/1126 §4).

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::annotation::names_problem;
use crate::hatch::is_hex_colour;
use crate::{CellRange, CellStyle, ProjectSettings, TableAlign, TableEntity, TextFace};

/// The largest text height, margin and border weight a cell style gives, paper millimetres.
pub const MAX_TABLE_STYLE_SIZE: f64 = 100.0;
/// The longest text style name a cell style names.
pub const MAX_TABLE_TEXT_STYLE_NAME: usize = 255;

/// One of a table style's cell styles (AutoCAD's Title, Header or Data), every field optional.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableCellStyleDef {
    /// The text style its words are written in, by its name (AutoCAD's group 7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_style: Option<String>,
    /// Its words' height on the paper, mm (group 140).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    /// Its words' horizontal alignment (group 170's left, centre or right).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub align: Option<TableAlign>,
    /// Its words' colour, `#rrggbb` (group 62).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Its background, `#rrggbb` (groups 63 and 283).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    /// The room between its words and its left and right borders, paper mm (group 40).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub horizontal_margin: Option<f64>,
    /// The room between its words and its top and bottom borders, paper mm (group 41).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub vertical_margin: Option<f64>,
    /// Its borders' weight on the paper, mm, 0 the thinnest line (groups 274–279).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub border_weight: Option<f64>,
    /// Its borders' colour, `#rrggbb` (groups 64–69).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub border_color: Option<String>,
}

/// A named table style (docs/adr/1126 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableStyleDef {
    /// One of its kind among the project's table styles.
    pub id: String,
    /// Its name: trimmed, one of its kind whatever its letters' case.
    pub name: String,
    /// The title row's cells: the first row when it is one merged range across every column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub title: Option<TableCellStyleDef>,
    /// The heading row's cells: the first row after the title, when the table has a heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub header: Option<TableCellStyleDef>,
    /// Every other row's cells.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub data: Option<TableCellStyleDef>,
}

impl TableCellStyleDef {
    /// What is wrong with it, said for `what` (“başlık hücresi”).
    pub fn problem(&self, what: &str) -> Option<String> {
        if let Some(name) = &self.text_style
            && (name.trim().is_empty()
                || name.trim() != name
                || name.len() > MAX_TABLE_TEXT_STYLE_NAME
                || name.chars().any(char::is_control))
        {
            return Some(format!(
                "{what} yazı stili adı “{name}” geçersiz: boş olmayan, kenarı boşluksuz, en çok {MAX_TABLE_TEXT_STYLE_NAME} bayt olmalı"
            ));
        }
        // A height over 0, a margin or a weight 0 or more; each finite and at most MAX_TABLE_STYLE_SIZE.
        let size = |v: Option<f64>, positive: bool, field: &str| -> Option<String> {
            let v = v?;
            let low = if positive { v > 0.0 } else { v >= 0.0 };
            (!(v.is_finite() && low && v <= MAX_TABLE_STYLE_SIZE)).then(|| {
                let from = if positive { "0'dan büyük" } else { "0 ile" };
                format!("{what} {field} {v}; {from} en çok {MAX_TABLE_STYLE_SIZE} mm olmalı")
            })
        };
        size(self.height, true, "yazı yüksekliği")
            .or_else(|| size(self.horizontal_margin, false, "yatay kenar boşluğu"))
            .or_else(|| size(self.vertical_margin, false, "dikey kenar boşluğu"))
            .or_else(|| size(self.border_weight, false, "kenar çizgisi kalınlığı"))
            .or_else(|| {
                [("yazı rengi", &self.color), ("dolgu rengi", &self.fill), ("kenar çizgisi rengi", &self.border_color)]
                    .into_iter()
                    .find_map(|(field, c)| {
                        c.as_deref()
                            .filter(|c| !is_hex_colour(c))
                            .map(|c| format!("{what} {field} “{c}” #rrggbb olmalı"))
                    })
            })
    }
}

impl TableStyleDef {
    /// What is wrong with it.
    pub fn problem(&self) -> Option<String> {
        [("başlık hücresi", &self.title), ("sütun başı hücresi", &self.header), ("veri hücresi", &self.data)]
            .into_iter()
            .find_map(|(what, cell)| {
                cell.as_ref()?
                    .problem(&format!("“{}” tablo stilinin {what}", self.name))
            })
    }
}

/// What is wrong with the project's table styles: ids and names as every style table's (`names_problem`), then
/// each style's own.
pub fn table_style_defs_problem(styles: &[TableStyleDef]) -> Option<String> {
    names_problem("tablo stili", styles.iter().map(|s| (s.id.as_str(), s.name.as_str())))
        .or_else(|| styles.iter().find_map(TableStyleDef::problem))
}

/// The rows of `t` each cell style takes: the title (its first row when that row is one merged range across every
/// column of a table of more than one column), the heading (the first row after the title when `t.header`), the rest
/// data; each as a first row and a count.
pub fn table_style_rows(t: &TableEntity) -> [(usize, usize); 3] {
    let n = t.rows.len();
    let cols = t.columns.len();
    let title = usize::from(
        n > 0
            && cols > 1
            && t.merges.iter().any(|m| m.row == 0 && m.col == 0 && m.rows == 1 && m.cols as usize == cols),
    );
    let header = usize::from(t.header && n > title);
    [(0, title), (title, header), (title + header, n - title - header)]
}

/// `t` as its table style draws it (docs/adr/1126 §3): what the table sets itself stays; where it sets nothing the
/// style's cell styles give each kind of row its alignment (the data rows only when the table has no column
/// alignments), its words' colour and its fill, under every cell style of the table's own; and the data cells' text
/// style its face, when the table has no style and no face of its own. A table that names no style, or one the
/// project does not have, is itself.
pub fn styled<'a>(t: &'a TableEntity, settings: &ProjectSettings) -> Cow<'a, TableEntity> {
    let Some(style) = t
        .table_style
        .as_deref()
        .and_then(|id| settings.table_styles.iter().find(|s| s.id == id))
    else {
        return Cow::Borrowed(t);
    };
    let cols = t.columns.len() as u32;
    let mut given: Vec<CellStyle> = Vec::new();
    for ((first, count), (cell, kind)) in table_style_rows(t)
        .into_iter()
        .zip([(&style.title, 0), (&style.header, 1), (&style.data, 2)])
    {
        let Some(cell) = cell else { continue };
        if count == 0 || cols == 0 {
            continue;
        }
        let align = cell.align.filter(|_| kind != 2 || t.aligns.is_none());
        if align.is_none() && cell.color.is_none() && cell.fill.is_none() {
            continue;
        }
        given.push(CellStyle {
            range: CellRange { row: first as u32, col: 0, rows: count as u32, cols },
            align,
            bold: None,
            color: cell.color.clone(),
            fill: cell.fill.clone(),
            decimals: None,
        });
    }
    let face = (t.face.text_style.is_none() && t.face.font.is_none())
        .then(|| style.data.as_ref()?.text_style.as_deref())
        .flatten()
        .and_then(|name| settings.text_styles.iter().find(|s| s.name.eq_ignore_ascii_case(name)))
        .map(|s| TextFace {
            text_style: Some(s.id.clone()),
            font: Some(s.font),
            bold: s.bold,
            italic: s.italic,
            oblique: s.oblique,
            overrides: Vec::new(),
        });
    if given.is_empty() && face.is_none() {
        return Cow::Borrowed(t);
    }
    let mut out = t.clone();
    given.extend(out.styles);
    out.styles = given;
    if let Some(face) = face {
        out.face = face;
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn settings(styles: serde_json::Value) -> ProjectSettings {
        let mut s: ProjectSettings = serde_json::from_value(json!({"srid":5256,"lengthDecimals":3,"areaDecimals":2,
            "areaUnit":"m2","angleUnit":"grad","plotScale":100.0})).unwrap();
        s.table_styles = serde_json::from_value(styles).unwrap();
        s.text_styles = serde_json::from_value(json!([{"id":"t1","name":"A-TABLO","font":"arimo","bold":true}])).unwrap();
        s
    }

    fn table(extra: serde_json::Value) -> TableEntity {
        let mut t = json!({"id":1,"layerId":"l","attrs":{},"p":{"x":0.,"y":0.},"rotation":0.,"height":2.5,
            "rows":[5.,5.,5.,5.],"columns":[20.,20.,20.],"cells":[["Başlık","",""],["A","B","C"],["1","2","3"],["4","5","6"]],
            "merges":[{"row":0,"col":0,"rows":1,"cols":3}],"header":true,"tableStyle":"ts"});
        for (k, v) in extra.as_object().unwrap() {
            t[k] = v.clone();
        }
        serde_json::from_value(t).unwrap()
    }

    const STYLE: &str = r##"[{"id":"ts","name":"OFIS","title":{"align":"center","fill":"#eeeeee"},
        "header":{"align":"right","color":"#ff0000"},"data":{"align":"right","textStyle":"a-tablo","height":2.0,
        "horizontalMargin":1.5,"borderWeight":0.25,"borderColor":"#00ff00"}}]"##;

    #[test]
    fn rows_are_title_header_and_data_as_autocad_names_them() {
        let t = table(json!({}));
        assert_eq!(table_style_rows(&t), [(0, 1), (1, 1), (2, 2)]);
        // No merged first row: no title; no heading: all data.
        let t = table(json!({"merges":[],"header":false}));
        assert_eq!(table_style_rows(&t), [(0, 0), (0, 0), (0, 4)]);
        let t = table(json!({"merges":[{"row":0,"col":0,"rows":1,"cols":2}]}));
        assert_eq!(table_style_rows(&t), [(0, 0), (0, 1), (1, 3)]);
    }

    #[test]
    fn the_style_fills_in_what_the_table_does_not_set() {
        let s = settings(serde_json::from_str(STYLE).unwrap());
        let t = table(json!({}));
        let drawn = styled(&t, &s);
        assert_eq!(drawn.cell_style(0, 0).fill.as_deref(), Some("#eeeeee"));
        assert_eq!(drawn.cell_style(0, 0).align, Some(TableAlign::Center));
        assert_eq!(drawn.cell_style(1, 2).color.as_deref(), Some("#ff0000"));
        assert_eq!(drawn.cell_style(1, 2).align, Some(TableAlign::Right));
        assert_eq!(drawn.cell_style(3, 1).align, Some(TableAlign::Right));
        assert_eq!(drawn.face.text_style.as_deref(), Some("t1"));
        assert_eq!(drawn.face.font, Some(crate::DrawingFont::Arimo));
        assert!(drawn.face.bold);
        // Its own cell style, column alignments and face stay over the style's.
        let own = table(json!({"styles":[{"row":3,"col":1,"rows":1,"cols":1,"align":"left","fill":"#123456"}],
            "aligns":["left","center","left"],"textStyle":"other"}));
        let drawn = styled(&own, &s);
        assert_eq!(drawn.cell_style(3, 1).align, Some(TableAlign::Left));
        assert_eq!(drawn.cell_style(3, 1).fill.as_deref(), Some("#123456"));
        assert_eq!(drawn.cell_style(2, 1).align, None, "the data rows keep the table's column alignments");
        assert_eq!(drawn.face.text_style.as_deref(), Some("other"));
        // No style, or one the project lacks: the table itself.
        let none = table(json!({"tableStyle": null}));
        assert!(matches!(styled(&none, &s), Cow::Borrowed(_)));
        let gone = table(json!({"tableStyle": "missing"}));
        assert!(matches!(styled(&gone, &s), Cow::Borrowed(_)));
    }

    #[test]
    fn a_style_is_checked_field_by_field() {
        let mut list: Vec<TableStyleDef> = serde_json::from_str(STYLE).unwrap();
        assert_eq!(table_style_defs_problem(&list), None);
        let base = list[0].clone();
        for (field, value) in [("height", json!(0.0)), ("height", json!(101.0)), ("horizontalMargin", json!(-1.0)),
            ("borderWeight", json!(f64::MAX)), ("color", json!("red")), ("borderColor", json!("#12345")),
            ("textStyle", json!(" X")), ("textStyle", json!(""))]
        {
            let mut v = serde_json::to_value(&base).unwrap();
            v["data"][field] = value;
            list[0] = serde_json::from_value(v).unwrap();
            assert!(table_style_defs_problem(&list).is_some(), "{field}");
        }
        list[0] = base.clone();
        list.push(TableStyleDef { id: "ts2".into(), ..base.clone() });
        assert!(table_style_defs_problem(&list).unwrap().contains("OFIS"), "a name twice, case aside");
        let unknown = serde_json::from_value::<TableStyleDef>(json!({"id":"x","name":"X","data":{"flow":"up"}}));
        assert!(unknown.is_err(), "an unknown field is refused, never dropped");
    }
}
