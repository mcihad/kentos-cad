//! A multi-line text's indents, tab stops and columns from Öznitelikler (docs/adr/0324 §3): each row's value read,
//! the texts changed in one step “Değiştir” through `cad.entities.edit`, the command's refusal said as it is.

use kentos_commands::{ParagraphLayout, TextColumns, TextEntity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::TextPlace;
use kentos_native_application::geometry::shape;

use crate::properties::change_texts;

/// A column's gap when columns are first given, in the text's height (AutoCAD's own is a little under one).
pub const COLUMN_GAP: f64 = 1.0;

/// The texts in `slots` with their layout changed by `change`; one with nothing left has none (an empty layout
/// takes it away, docs/adr/0324 §3).
fn change_layouts(
    doc: &mut Document,
    slots: &[Slot],
    change: impl Fn(&TextEntity, &mut ParagraphLayout, kentos_geometry_core::text::Font),
) -> Vec<String> {
    change_texts(doc, slots, |t, font| {
        let had = t.paragraph.layout.as_deref().cloned().unwrap_or_default();
        let mut l = had.clone();
        change(t, &mut l, font);
        // One spelling: a zero indent is no indent.
        l.indent = l.indent.filter(|v| *v != 0.0);
        l.left_indent = l.left_indent.filter(|v| *v != 0.0);
        if l == had {
            return None;
        }
        let mut t = t.clone();
        t.paragraph.layout = Some(Box::new(l));
        Some(t)
    })
}

/// The first line's indent from the others', in heights (below 0 a hanging indent).
pub fn set_indent(doc: &mut Document, slots: &[Slot], heights: f64) -> Vec<String> {
    change_layouts(doc, slots, |_, l, _| l.indent = Some(heights))
}

/// Every line's indent from the box's left but each paragraph's first, in heights.
pub fn set_left_indent(doc: &mut Document, slots: &[Slot], heights: f64) -> Vec<String> {
    change_layouts(doc, slots, |_, l, _| l.left_indent = Some(heights))
}

/// The tab stops, in heights; none for MTEXT's own (every 4 heights).
pub fn set_tabs(doc: &mut Document, slots: &[Slot], stops: &[f64]) -> Vec<String> {
    change_layouts(doc, slots, |_, l, _| l.tabs = stops.to_vec())
}

/// `count` columns (1: none). Given first, each is as tall as the text's lines over `count`, a height apart.
pub fn set_columns(doc: &mut Document, slots: &[Slot], count: u32) -> Vec<String> {
    change_layouts(doc, slots, |t, l, font| {
        if count < 2 {
            l.columns = None;
            return;
        }
        let (gap, height) = match l.columns {
            Some(c) => (c.gap, c.height),
            None => {
                let mut one = t.clone();
                if let Some(lay) = one.paragraph.layout.as_deref_mut() {
                    lay.columns = None;
                }
                let s = shape(&kentos_commands::Entity::Text(one));
                let tall = TextPlace::of(&s).map_or(t.height, |p| {
                    let laid = p.layout(font);
                    laid.depth.max(1) as f64 * laid.pitch
                });
                (COLUMN_GAP, (tall / f64::from(count)).max(t.height))
            }
        };
        l.columns = Some(TextColumns { count, gap, height });
    })
}

/// The columns' height, metres.
pub fn set_column_height(doc: &mut Document, slots: &[Slot], metres: f64) -> Vec<String> {
    change_layouts(doc, slots, |_, l, _| {
        if let Some(c) = l.columns.as_mut() {
            c.height = metres;
        }
    })
}

/// The gap between columns, in heights.
pub fn set_column_gap(doc: &mut Document, slots: &[Slot], heights: f64) -> Vec<String> {
    change_layouts(doc, slots, |_, l, _| {
        if let Some(c) = l.columns.as_mut() {
            c.gap = heights;
        }
    })
}

/// Tab stops as typed: numbers in heights apart by spaces or semicolons; empty, “Varsayılan” or “-” for none.
pub fn parse_tabs(text: &str) -> Result<Vec<f64>, String> {
    let t = crate::js_trim(text);
    if t.is_empty() || t == "-" || t.eq_ignore_ascii_case("varsayılan") || t.starts_with("Her ") {
        return Ok(Vec::new());
    }
    t.split(|c: char| c == ';' || c.is_whitespace())
        .filter(|w| !w.is_empty())
        .map(|w| {
            match w.parse::<f64>() {
                Ok(v) if v.is_finite() => Ok(v),
                _ => {
                    Err(format!(
                        "“{w}” bir sayı değil: sekme duraklarını yükseklik cinsinden, boşluk ya da noktalı virgülle yazın (4; 8; 12)."
                    ))
                }
            }
        })
        .collect()
}

/// Tab stops as Öznitelikler shows them: “4; 8; 12”, or MTEXT's own.
pub fn tabs_text(stops: &[f64]) -> String {
    if stops.is_empty() {
        return format!("Her {} yükseklikte", kentos_commands::DEFAULT_TAB);
    }
    stops
        .iter()
        .map(|v| format!("{v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_stops_read_as_typed() {
        assert_eq!(parse_tabs("4; 8 12"), Ok(vec![4.0, 8.0, 12.0]));
        assert_eq!(parse_tabs(" "), Ok(Vec::new()));
        assert_eq!(parse_tabs("Her 4 yükseklikte"), Ok(Vec::new()));
        assert!(parse_tabs("4; a").is_err());
        assert_eq!(tabs_text(&[2.5, 6.0]), "2.5; 6");
        assert_eq!(tabs_text(&[]), "Her 4 yükseklikte");
    }
}
