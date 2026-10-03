//! Paper sizes and standard map scales: data (`data/papers.json`), the same
//! file on every platform.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::model::{Orientation, Page, Paper};
use crate::units::{Margins, SizeUm, Um};

/// A paper of the table, portrait.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PaperSize {
    pub id: Paper,
    /// “A4”.
    pub name: String,
    pub width: Um,
    pub height: Um,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Table {
    #[allow(dead_code)]
    schema: String,
    #[allow(dead_code)]
    source: String,
    papers: Vec<PaperSize>,
    margins: Margins,
    scales: Vec<u32>,
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    // The file is the crate's own and a test reads it; a broken one leaves the tables empty, never a panic.
    T.get_or_init(|| serde_json::from_str(include_str!("../data/papers.json")).unwrap_or_default())
}

/// Every paper of the table, portrait, in its order.
pub fn paper_sizes() -> &'static [PaperSize] {
    &table().papers
}

/// The standard scales' denominators, from the largest scale (1/100) to the smallest.
pub fn standard_scales() -> &'static [u32] {
    &table().scales
}

pub fn is_standard_scale(scale: u32) -> bool {
    standard_scales().contains(&scale)
}

/// The margins a new page gets.
pub fn default_margins() -> Margins {
    table().margins
}

/// A paper's size as it lies; none for `custom` or a paper the table lacks.
pub fn paper_size(paper: Paper, orientation: Orientation) -> Option<SizeUm> {
    let p = paper_sizes().iter().find(|p| p.id == paper)?;
    let (short, long) = (p.width.min(p.height), p.width.max(p.height));
    Some(match orientation {
        Orientation::Portrait => SizeUm {
            width: short,
            height: long,
        },
        Orientation::Landscape => SizeUm {
            width: long,
            height: short,
        },
    })
}

/// A page of a standard paper with the default margins.
pub fn standard_page(paper: Paper, orientation: Orientation) -> Option<Page> {
    Some(Page {
        paper,
        orientation,
        size: paper_size(paper, orientation)?,
        margins: default_margins(),
        background: None,
    })
}

/// The orientation a size has: landscape when wider than tall.
pub fn orientation_of(size: SizeUm) -> Orientation {
    if size.width > size.height {
        Orientation::Landscape
    } else {
        Orientation::Portrait
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_every_standard_paper() {
        for p in Paper::STANDARD {
            assert!(paper_size(p, Orientation::Portrait).is_some(), "{p:?}");
        }
        assert_eq!(
            paper_size(Paper::A3, Orientation::Landscape),
            Some(SizeUm {
                width: 420_000,
                height: 297_000
            })
        );
        assert_eq!(paper_size(Paper::Custom, Orientation::Portrait), None);
        assert_eq!(standard_scales().first(), Some(&100));
        assert!(is_standard_scale(2500));
        assert!(!is_standard_scale(855));
        assert_eq!(default_margins().left, 10_000);
    }
}
