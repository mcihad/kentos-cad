//! The drawing scale an opened CAD drawing takes (docs/adr/0626 §5): asked of nobody, read from the drawing. Its
//! texts' and dimensions' heights say what they are on paper: a drawing whose texts are 250 mm high is drawn for
//! 1:100 when they are the usual 2.5 mm on paper (a DIMSCALE of 100 makes its dimensions' values so). The middle
//! height (the median) over 2.5 mm, the nearest of the usual scales; none to read, the unit's own.

use kentos_commands::{DrawingUnit, Entity};

/// What a text is on paper when nothing says otherwise, metres (2.5 mm: ADR 0205 §1's text and dimension).
pub const PAPER_TEXT: f64 = 0.0025;

/// The scales an opened drawing is snapped to (1:N): a drawing's from 1:1 to a plan's 1:50 000.
pub const SCALES: [f64; 18] = [
    1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 250.0, 500.0, 1000.0, 2000.0, 2500.0,
    5000.0, 10_000.0, 25_000.0, 50_000.0,
];

/// The scale for `entities` (heights in metres): the middle of their texts', dimensions' and leaders' heights
/// over [`PAPER_TEXT`], snapped to the nearest of [`SCALES`] (by ratio); none of them, the unit's own
/// (`kentos_project::wizard::cad_default_scale`) or 1:1000 on a coordinate system.
pub fn opened_scale(entities: &[Entity], unit: DrawingUnit, on_system: bool) -> f64 {
    let mut heights: Vec<f64> = entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.height),
            Entity::Dimension(d) => Some(d.height),
            Entity::Leader(l) => Some(l.height),
            _ => None,
        })
        .filter(|h| h.is_finite() && *h > 0.0)
        .collect();
    if heights.is_empty() {
        return if on_system {
            1000.0
        } else {
            kentos_project::wizard::cad_default_scale(unit)
        };
    }
    heights.sort_by(f64::total_cmp);
    let middle = heights[(heights.len() - 1) / 2];
    nearest(middle / PAPER_TEXT)
}

/// The usual scale nearest `raw` by ratio (a twice too large one as far as a half); under 1:1, 1:1.
pub fn nearest(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 1.0 {
        return 1.0;
    }
    let r = raw.ln();
    SCALES
        .iter()
        .copied()
        .min_by(|a, b| (a.ln() - r).abs().total_cmp(&(b.ln() - r).abs()))
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(height: f64) -> Entity {
        serde_json::from_value(serde_json::json!({
            "kind": "text", "id": 0, "layerId": "0", "attrs": {}, "p": { "x": 0.0, "y": 0.0 }, "text": "A",
            "height": height, "rotation": 0.0
        }))
        .expect("a text")
    }

    #[test]
    fn the_texts_middle_height_says_the_scale() {
        // 2.5 mm texts: 1:1. 250 mm: 1:100. 2.5 m: 1:1000. One odd text does not move the middle.
        assert_eq!(opened_scale(&[text(0.0025)], DrawingUnit::Mm, false), 1.0);
        assert_eq!(
            opened_scale(&[text(0.25), text(0.25), text(5.0)], DrawingUnit::Mm, false),
            100.0
        );
        assert_eq!(opened_scale(&[text(2.5)], DrawingUnit::M, true), 1000.0);
        // 150 mm is nearer 1:50 than 1:100 by ratio (60: 1.2 and 1.67 times away).
        assert_eq!(opened_scale(&[text(0.15)], DrawingUnit::Mm, false), 50.0);
        // Under 2.5 mm on paper at 1:1: still 1:1.
        assert_eq!(opened_scale(&[text(0.001)], DrawingUnit::Mm, false), 1.0);
    }

    #[test]
    fn a_drawing_without_texts_takes_its_units_scale() {
        assert_eq!(opened_scale(&[], DrawingUnit::Mm, false), 1.0);
        assert_eq!(opened_scale(&[], DrawingUnit::Cm, false), 50.0);
        assert_eq!(opened_scale(&[], DrawingUnit::M, false), 100.0);
        assert_eq!(opened_scale(&[], DrawingUnit::M, true), 1000.0);
        assert_eq!(nearest(f64::NAN), 1.0);
        assert_eq!(nearest(3.0), 2.0);
        assert_eq!(nearest(4.0), 5.0);
    }
}
