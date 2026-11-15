//! Where a rule was read and how much it is asked. The annexes' legal text is the Resmî Gazete's PDF (241 scanned
//! pages); KentOS reads its transcription (mevzuat/bim) and keeps the PDF's page numbers, the printed one being 4 more.

use serde::{Deserialize, Serialize};

/// How much a rule is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    /// Asked of every object of its class the model has (EK-6 Tablo 6.1–6.68, EK-7 Tablo 7.1–7.9): a class is not
    /// asked to be in every building, its fields are asked when it is.
    Mandatory,
    /// Asked when a condition holds: an administration's reading, a project's kind.
    Conditional,
    /// May be given (EK-6 Tablo 6.69–6.135, EK-7 Tablo 7.10–7.12); an administration may ask it (Md. 12/2, 13).
    Optional,
}

impl Level {
    /// As the rules window says it.
    pub fn named(self) -> &'static str {
        match self {
            Level::Mandatory => "Zorunlu",
            Level::Conditional => "Koşullu",
            Level::Optional => "İsteğe bağlı",
        }
    }
}

/// Where a rule was read: the annex, its table, clause or figure, the row and the annexes PDF's pages; for the
/// regulation's own text (annex 0) the article.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// 1…9 for EK-1…EK-9; 0 for the regulation's articles.
    pub annex: u8,
    /// A table: “2.2”, “6.66”.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    /// A clause of an annex: “4.11”, “5.5.3”.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clause: Option<String>,
    /// A figure: “4.7”.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figure: Option<String>,
    /// The regulation's article: “11/3”; with `act` the e-PYS regulation's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub article: Option<String>,
    /// Another act's short name for an article not of this regulation (“e-PYS”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<String>,
    /// The row in its table or figure, from 1 in printed order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<u32>,
    /// The annexes PDF's pages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<u32>,
}

impl Source {
    /// A table's row.
    pub fn table(annex: u8, table: &str, row: Option<u32>, page: u32) -> Self {
        Self {
            annex,
            table: Some(table.to_owned()),
            row,
            pages: vec![page],
            ..Self::default()
        }
    }

    /// A clause of an annex.
    pub fn clause(annex: u8, clause: &str, pages: &[u32]) -> Self {
        Self {
            annex,
            clause: Some(clause.to_owned()),
            pages: pages.to_vec(),
            ..Self::default()
        }
    }

    /// As a finding or the rules window says it: “EK-6 Tablo 6.66 satır 6 · PDF s. 107”, “EK-4 m. 4.11 · PDF s. 20”,
    /// “EK-4 Şekil 4.7 · PDF s. 28”, “Md. 11/3”, “e-PYS Md. 4/4”.
    pub fn cite(&self) -> String {
        let mut out = String::new();
        if self.annex == 0 {
            let mut parts: Vec<String> = Vec::new();
            if let Some(act) = &self.act {
                parts.push(act.clone());
            }
            if let Some(a) = &self.article {
                parts.push(format!("Md. {a}"));
            }
            out.push_str(&parts.join(" "));
        } else {
            out.push_str(&format!("EK-{}", self.annex));
            if let Some(t) = &self.table {
                out.push_str(&format!(" Tablo {t}"));
            } else if let Some(c) = &self.clause {
                out.push_str(&format!(" m. {c}"));
            } else if let Some(f) = &self.figure {
                out.push_str(&format!(" Şekil {f}"));
            }
            if let Some(r) = self.row {
                out.push_str(&format!(" satır {r}"));
            }
        }
        if !self.pages.is_empty() {
            out.push_str(" · PDF s. ");
            out.push_str(&pages_words(&self.pages));
        }
        out
    }
}

/// “107”, “20–21”, “43, 45”.
fn pages_words(pages: &[u32]) -> String {
    let mut ps = pages.to_vec();
    ps.sort_unstable();
    ps.dedup();
    match ps.as_slice() {
        [] => String::new(),
        [one] => one.to_string(),
        [first, .., last] if (*last - *first) as usize + 1 == ps.len() => format!("{first}–{last}"),
        _ => ps.iter().map(u32::to_string).collect::<Vec<_>>().join(", "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_says_its_annex_table_row_and_page() {
        assert_eq!(
            Source::table(6, "6.66", Some(6), 107).cite(),
            "EK-6 Tablo 6.66 satır 6 · PDF s. 107"
        );
        assert_eq!(
            Source::clause(4, "4.12.3", &[21, 20]).cite(),
            "EK-4 m. 4.12.3 · PDF s. 20–21"
        );
        let article = Source {
            article: Some("4/4".into()),
            act: Some("e-PYS".into()),
            ..Source::default()
        };
        assert_eq!(article.cite(), "e-PYS Md. 4/4");
        let figure = Source {
            annex: 4,
            figure: Some("4.7".into()),
            row: Some(3),
            pages: vec![28],
            ..Source::default()
        };
        assert_eq!(figure.cite(), "EK-4 Şekil 4.7 satır 3 · PDF s. 28");
        assert_eq!(pages_words(&[43, 45]), "43, 45");
    }
}
