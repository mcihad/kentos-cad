//! A data layer whose rows are texts (docs/adr/9114 §1): each row a text at its point, its words, height, angle,
//! alignment and width factor read from columns of the row. A drawing layer's texts moved into a source become such a
//! table (`cad.layers.migrate`, docs/adr/9114 §2); the layer draws its rows as the drawing's texts are drawn.
//!
//! The columns KentOS writes are `metin`, `yukseklik` (metres), `aci` (degrees counter-clockwise from east), `hiza`
//! (Yazı's names: “sol alt”, “orta”, “sağ üst” …) and `genislik` (the width factor), as the national exchange format
//! keeps a text (BÖHHBÜY Ek-2, UVDF §3.2.6.1 Yazi: Metin, Aci, Boyut, GenislikFaktoru, DayamaNoktasi). A table of
//! another program names its own columns: the mapping says which they are.

use serde::{Deserialize, Serialize};

use crate::TextAlign;

/// Which columns of a data layer's rows make its texts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataText {
    /// The column of the words.
    pub text: String,
    /// The column of the height, metres; absent, or empty or not over 0 in a row: the project's text height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
    /// The column of the angle, degrees counter-clockwise from east; absent or empty: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<String>,
    /// The column of the alignment: Yazı's names (“sol alt”, “orta”, “sağ üst” …) or the contract's (`bottomLeft`);
    /// absent, empty or another word: the left of the baseline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// The column of the width factor; absent, empty, 1 or out of (0, 100]: none (1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
}

/// A text's parts as a row gives them.
#[derive(Clone, Debug, PartialEq)]
pub struct TextParts {
    pub text: String,
    /// Metres, over 0.
    pub height: f64,
    /// Degrees counter-clockwise from east.
    pub rotation: f64,
    /// None: the left of the baseline.
    pub align: Option<TextAlign>,
    /// None: 1.
    pub width_factor: Option<f64>,
}

/// A value a text writes into a column.
#[derive(Clone, Debug, PartialEq)]
pub enum TextCell {
    Text(String),
    Number(f64),
}

impl DataText {
    /// The columns KentOS writes a text table with (docs/adr/9114 §2).
    pub fn written() -> DataText {
        DataText {
            text: "metin".into(),
            height: Some("yukseklik".into()),
            angle: Some("aci".into()),
            align: Some("hiza".into()),
            width: Some("genislik".into()),
        }
    }

    /// The columns it reads, in its order.
    pub fn columns(&self) -> Vec<&str> {
        let mut out = vec![self.text.as_str()];
        out.extend(
            [&self.height, &self.angle, &self.align, &self.width]
                .into_iter()
                .flatten()
                .map(String::as_str),
        );
        out
    }

    /// A row as a text: `cell` reads a column's value as its text (none: empty). None for a row without words.
    pub fn read(
        &self,
        cell: impl Fn(&str) -> Option<String>,
        default_height: f64,
    ) -> Option<TextParts> {
        let text = cell(&self.text).filter(|t| !t.trim().is_empty())?;
        let number = |column: &Option<String>| {
            column
                .as_deref()
                .and_then(&cell)
                .and_then(|v| number_of(&v))
        };
        Some(TextParts {
            text,
            height: number(&self.height)
                .filter(|h| *h > 0.0)
                .unwrap_or(default_height),
            rotation: number(&self.angle).unwrap_or(0.0),
            align: self
                .align
                .as_deref()
                .and_then(&cell)
                .and_then(|v| align_of(&v)),
            // 1 is none, as a text writes it (docs/adr/0145: producers do not write 1).
            width_factor: number(&self.width)
                .filter(|w| *w > 0.0 && *w <= crate::MAX_WIDTH_FACTOR && *w != 1.0),
        })
    }

    /// A text's values for its columns: the words, the height, the angle, the alignment's name and the width factor.
    pub fn cells(&self, parts: &TextParts) -> Vec<(String, TextCell)> {
        let mut out = vec![(self.text.clone(), TextCell::Text(parts.text.clone()))];
        let mut put = |column: &Option<String>, value: TextCell| {
            if let Some(c) = column {
                out.push((c.clone(), value));
            }
        };
        put(&self.height, TextCell::Number(parts.height));
        put(&self.angle, TextCell::Number(parts.rotation));
        put(
            &self.align,
            TextCell::Text(align_words(parts.align).to_owned()),
        );
        put(
            &self.width,
            TextCell::Number(parts.width_factor.unwrap_or(1.0)),
        );
        out
    }

    /// What is wrong with it, in Turkish.
    pub fn problem(&self) -> Option<String> {
        let columns = self.columns();
        if columns
            .iter()
            .any(|c| c.trim().is_empty() || c.len() > crate::MAX_DATA_TEXT)
        {
            return Some("yazı sütunlarının adı boş ya da çok uzun".into());
        }
        if let Some(c) = columns
            .iter()
            .enumerate()
            .find_map(|(i, c)| columns[..i].contains(c).then_some(*c))
        {
            return Some(format!("“{c}” sütunu yazının iki parçası olamaz"));
        }
        None
    }
}

/// A number as a cell writes it: a point or a comma before its fraction.
fn number_of(v: &str) -> Option<f64> {
    let t = v.trim();
    let t = if t.contains('.') {
        t.to_owned()
    } else {
        t.replace(',', ".")
    };
    t.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// The alignments as Yazı names them (kentos-interaction's `ALIGNS`): top, middle, bottom, the baseline; left, centre,
/// right. None is the left of the baseline.
const ALIGN_WORDS: [(Option<TextAlign>, &str); 12] = [
    (Some(TextAlign::TopLeft), "sol üst"),
    (Some(TextAlign::TopCenter), "orta üst"),
    (Some(TextAlign::TopRight), "sağ üst"),
    (Some(TextAlign::MiddleLeft), "sol orta"),
    (Some(TextAlign::MiddleCenter), "orta"),
    (Some(TextAlign::MiddleRight), "sağ orta"),
    (Some(TextAlign::BottomLeft), "sol alt"),
    (Some(TextAlign::BottomCenter), "orta alt"),
    (Some(TextAlign::BottomRight), "sağ alt"),
    (None, "sol taban"),
    (Some(TextAlign::BaselineCenter), "orta taban"),
    (Some(TextAlign::BaselineRight), "sağ taban"),
];

/// An alignment's name as a text table keeps it: “sol taban”, “orta”, “sağ üst”.
pub fn align_words(a: Option<TextAlign>) -> &'static str {
    ALIGN_WORDS
        .iter()
        .find(|(b, _)| *b == a)
        .map_or("sol taban", |(_, w)| w)
}

/// The alignment a cell names: Yazı's names, written together or apart, with or without the Turkish marks (“sağ üst”,
/// “sagust”, “Sağ-üst”; “orta” and “orta orta” the middle), or the contract's (`topRight`). None for the left of the
/// baseline and for any other word.
pub fn align_of(cell: &str) -> Option<TextAlign> {
    if let Some(a) = TextAlign::from_name(cell.trim()) {
        return Some(a);
    }
    let folded = fold(cell);
    if folded == "ortaorta" {
        return Some(TextAlign::MiddleCenter);
    }
    ALIGN_WORDS
        .iter()
        .find(|(_, w)| fold(w) == folded)
        .and_then(|(a, _)| *a)
}

/// A name folded as Yazı folds it: lower case, Turkish letters without their marks, nothing but letters.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            _ => c.to_lowercase().collect(),
        })
        .map(|c| match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            c => c,
        })
        .filter(char::is_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row<'a>(cells: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |c| {
            cells
                .iter()
                .find(|(n, _)| *n == c)
                .map(|(_, v)| (*v).to_owned())
        }
    }

    #[test]
    fn a_row_reads_as_a_text_and_writes_back() {
        let t = DataText::written();
        let parts = t
            .read(
                row(&[
                    ("metin", "Ada 101"),
                    ("yukseklik", "2.5"),
                    ("aci", "33.3"),
                    ("hiza", "sağ üst"),
                    ("genislik", "0,8"),
                ]),
                2.0,
            )
            .expect("words");
        assert_eq!(
            parts,
            TextParts {
                text: "Ada 101".into(),
                height: 2.5,
                rotation: 33.3,
                align: Some(TextAlign::TopRight),
                width_factor: Some(0.8)
            }
        );
        let cells = t.cells(&parts);
        assert_eq!(
            cells[0],
            ("metin".to_owned(), TextCell::Text("Ada 101".into()))
        );
        assert_eq!(
            cells[3],
            ("hiza".to_owned(), TextCell::Text("sağ üst".into()))
        );
        assert_eq!(
            t.read(
                |c| cells.iter().find(|(n, _)| n == c).map(|(_, v)| match v {
                    TextCell::Text(s) => s.clone(),
                    TextCell::Number(n) => n.to_string(),
                }),
                2.0
            ),
            Some(parts)
        );
    }

    #[test]
    fn empty_and_strange_cells_take_the_defaults() {
        let t = DataText::written();
        assert!(
            t.read(row(&[("metin", "  ")]), 2.0).is_none(),
            "no words, no text"
        );
        let p = t
            .read(
                row(&[
                    ("metin", "Not"),
                    ("yukseklik", "-1"),
                    ("aci", "yok"),
                    ("hiza", "çapraz"),
                    ("genislik", "0"),
                ]),
                2.0,
            )
            .expect("words");
        assert_eq!(
            (p.height, p.rotation, p.align, p.width_factor),
            (2.0, 0.0, None, None)
        );
        let only = DataText {
            text: "ad".into(),
            height: None,
            angle: None,
            align: None,
            width: None,
        };
        assert_eq!(only.columns(), ["ad"]);
        assert_eq!(only.cells(&p).len(), 1);
    }

    #[test]
    fn alignments_are_read_as_yazi_names_them_and_as_the_contract_does() {
        for (a, w) in ALIGN_WORDS {
            assert_eq!(align_of(w), a, "{w}");
            assert_eq!(align_words(a), w);
            if let Some(a) = a {
                assert_eq!(align_of(a.name()), Some(a));
            }
        }
        assert_eq!(align_of("Sağ-Üst"), Some(TextAlign::TopRight));
        assert_eq!(align_of("SAGUST"), Some(TextAlign::TopRight));
        assert_eq!(align_of("orta orta"), Some(TextAlign::MiddleCenter));
        assert_eq!(align_of("ORTA ALT"), Some(TextAlign::BottomCenter));
    }

    #[test]
    fn a_mapping_names_each_column_once() {
        assert!(DataText::written().problem().is_none());
        let twice = DataText {
            angle: Some("metin".into()),
            ..DataText::written()
        };
        assert_eq!(
            twice.problem().as_deref(),
            Some("“metin” sütunu yazının iki parçası olamaz")
        );
        let blank = DataText {
            text: " ".into(),
            ..DataText::written()
        };
        assert!(blank.problem().is_some());
    }
}
