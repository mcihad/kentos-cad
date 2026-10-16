//! Koşullu biçim (docs/adr/0413 §1): how the attribute table shows a layer's rows and cells by
//! conditions, QGIS's conditional formatting. Each rule is a condition over the record (the
//! expression language, the layer's fields by name), where it paints (the whole row, or one field's
//! cell), and what it gives: a background colour, a text colour, bold. The first rule that holds
//! gives each of them for a cell; a field's rules come before the row's. The rules here are the
//! readers', the command's and the server's; whether a condition compiles and a field exists is the
//! command's.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::hatch::is_hex_colour;

/// The most rules a layer has.
pub const MOST_TABLE_STYLES: usize = 50;
/// The longest condition, in characters.
pub const MAX_CONDITION: usize = 2000;

fn is_false(b: &bool) -> bool {
    !*b
}

/// One rule of a layer's table (docs/adr/0413 §1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableStyle {
    /// The field whose cell it paints, by name; absent: the whole row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    /// When it holds: an expression over the record (`Kat > 3 ve Durum = 'kotu'`).
    pub condition: String,
    /// The cell's background, `#RRGGBB`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub background: Option<String>,
    /// The text's colour, `#RRGGBB`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Bold text. Only `true` is written.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub bold: bool,
}

/// What a cell shows by the rules that hold for it (docs/adr/0413 §1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CellLook {
    pub background: Option<String>,
    pub color: Option<String>,
    pub bold: bool,
}

impl CellLook {
    /// Whether it changes nothing.
    pub fn is_plain(&self) -> bool {
        self.background.is_none() && self.color.is_none() && !self.bold
    }
}

/// What the rules give the cell of `field` (none: a row's own look) in a row, `holds[i]` saying
/// whether rule `i` holds for it: the field's rules first, then the row's, the first that gives
/// each of the three.
pub fn cell_look(styles: &[TableStyle], holds: &[bool], field: Option<&str>) -> CellLook {
    let mut look = CellLook::default();
    let mut bold_given = false;
    let order = styles
        .iter()
        .enumerate()
        .filter(|(_, s)| field.is_some() && s.field.as_deref() == field)
        .chain(styles.iter().enumerate().filter(|(_, s)| s.field.is_none()));
    for (i, s) in order {
        if !holds.get(i).copied().unwrap_or(false) {
            continue;
        }
        if look.background.is_none() {
            look.background.clone_from(&s.background);
        }
        if look.color.is_none() {
            look.color.clone_from(&s.color);
        }
        if !bold_given && s.bold {
            look.bold = true;
            bold_given = true;
        }
    }
    look
}

/// Why a layer's rules break the rules, with the rule's place; none when they keep them.
pub fn table_styles_problem(styles: &[TableStyle]) -> Option<(usize, String)> {
    if styles.len() > MOST_TABLE_STYLES {
        return Some((
            MOST_TABLE_STYLES,
            format!(
                "Bir katmanın tablosunda en çok {MOST_TABLE_STYLES} biçim kuralı olur; {} verildi.",
                styles.len()
            ),
        ));
    }
    for (i, s) in styles.iter().enumerate() {
        let n = i + 1;
        if s.condition.trim().is_empty() {
            return Some((i, format!("{n}. kuralın koşulu boş.")));
        }
        if s.condition.chars().count() > MAX_CONDITION {
            return Some((
                i,
                format!("{n}. kuralın koşulu en çok {MAX_CONDITION} karakter olabilir."),
            ));
        }
        if s.field.as_deref().is_some_and(|f| f.trim().is_empty()) {
            return Some((
                i,
                format!("{n}. kuralın alanı boş; satır için alanı vermeyin."),
            ));
        }
        for (colour, word) in [(&s.background, "zemin"), (&s.color, "yazı")] {
            if let Some(c) = colour
                && !is_hex_colour(c)
            {
                return Some((
                    i,
                    format!("{n}. kuralın {word} rengi “{c}” #RRGGBB biçiminde değil."),
                ));
            }
        }
        if s.background.is_none() && s.color.is_none() && !s.bold {
            return Some((
                i,
                format!(
                    "{n}. kural hiçbir şey vermiyor: zemin rengi, yazı rengi ya da kalınlık verin."
                ),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(field: Option<&str>, bg: Option<&str>, color: Option<&str>, bold: bool) -> TableStyle {
        TableStyle {
            field: field.map(str::to_owned),
            condition: "Kat > 3".into(),
            background: bg.map(str::to_owned),
            color: color.map(str::to_owned),
            bold,
        }
    }

    #[test]
    fn the_fields_rules_come_before_the_rows_and_the_first_that_gives_wins() {
        let styles = [
            rule(None, Some("#FFEEAA"), None, false),
            rule(Some("Kat"), None, Some("#AA0000"), true),
            rule(Some("Kat"), Some("#00FF00"), Some("#0000FF"), false),
        ];
        let all = [true, true, true];
        assert_eq!(
            cell_look(&styles, &all, Some("Kat")),
            CellLook {
                background: Some("#00FF00".into()),
                color: Some("#AA0000".into()),
                bold: true
            }
        );
        assert_eq!(
            cell_look(&styles, &all, Some("Ad")),
            CellLook {
                background: Some("#FFEEAA".into()),
                color: None,
                bold: false
            }
        );
        assert!(cell_look(&styles, &[false, false, false], Some("Kat")).is_plain());
        assert_eq!(table_styles_problem(&styles), None);
        assert!(table_styles_problem(&[rule(None, None, None, false)]).is_some());
        assert!(table_styles_problem(&[rule(None, Some("red"), None, false)]).is_some());
    }
}
