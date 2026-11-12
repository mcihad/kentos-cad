//! Kaçış yolları çizimi (docs/adr/8035 §2; M35): the walks the fire rules measure (`fire_audit::walks`) drawn on the
//! plan of a storey, each from its farthest point to its exit: a line, an arrowhead at the exit and what it measured
//! against its limit (“tek yön 28,07 m > 15,00 m”).
//!
//! - A walk that breaks its limit is red, one that keeps it green; one the regulation does not ask (a dwelling
//!   building of four storeys or fewer, BYKHY 48/1) grey.
//! - Each mark carries the attribute `Kaçış` (its walk's words); drawn again, the old marks are taken away first, every
//!   storey's: the plan shows one storey's walks at a time.
//! - The marks are plain lines and texts: they do not follow the drawing; the tool is run again after a change.

use kentos_commands::{
    CommandResult, EntitiesCreate, EntityGeometry, NewObject, TextAlign, TextFace,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;

use crate::{ExecutionContext, create};

/// The attribute the marks carry: what a new drawing takes away.
pub const MARK_ATTR: &str = "Kaçış";
/// A walk that keeps its limit.
pub const KEPT_COLOR: &str = "#2e7d32";
/// A walk that breaks it.
pub const OVER_COLOR: &str = "#d32f2f";
/// A walk the regulation does not ask.
pub const UNASKED_COLOR: &str = "#757575";

/// What a drawing wrote.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    pub walks: usize,
    pub over: usize,
}

/// Metres with a decimal comma, as a plan writes them: “28,07”.
fn comma(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// A walk's words on the plan: “tek yön 28,07 m > 15,00 m”, “daire içi 12,40 m ≤ 20,00 m”, “9,80 m”.
pub fn words(w: &crate::fire_audit::Walk) -> String {
    match w.limit {
        Some(limit) => format!(
            "{} {} m {} {} m",
            w.words,
            comma(w.measure),
            if w.over { ">" } else { "≤" },
            comma(limit)
        ),
        None => format!("{} m", comma(w.length)),
    }
}

/// Draws `level`'s walks (every storey's when none) on the active layer, the old marks taken away first; one undo step
/// when the caller holds a group.
pub fn draw(doc: &mut Document, level: Option<&str>) -> Result<Drawn, String> {
    let old: Vec<Slot> = doc
        .entities()
        .filter(|e| e.base().attrs.contains_key(MARK_ATTR))
        .map(|e| Slot(e.base().id))
        .collect();
    if !old.is_empty() {
        doc.remove(&old);
    }
    let walks: Vec<crate::fire_audit::Walk> = crate::fire_audit::walks(doc)
        .into_iter()
        .filter(|w| level.is_none() || w.level.as_deref() == level)
        .filter(|w| w.path.len() >= 2)
        .collect();
    if walks.is_empty() {
        return Ok(Drawn::default());
    }
    let height = 2.5 / 1000.0 * doc.settings().plot_scale;
    let point = |v: V| kentos_commands::Vec2 { x: v.x, y: v.y };
    let mut objects = Vec::new();
    for w in &walks {
        let color = match (w.limit, w.over) {
            (None, _) => UNASKED_COLOR,
            (Some(_), true) => OVER_COLOR,
            (Some(_), false) => KEPT_COLOR,
        };
        let object = |geometry: EntityGeometry| NewObject {
            geometry,
            color: Some(color.to_owned()),
            line_weight: None,
            line_type: None,
            line_pattern: None,
            attrs: Some(std::iter::once((MARK_ATTR.to_owned(), w.words.to_owned())).collect()),
            label: None,
            label_of: None,
            label_scale: None,
            symbol: None,
            order: None,
            transparency: None,
        };
        let polyline = |pts: Vec<V>| EntityGeometry::Polyline {
            pts: pts.into_iter().map(point).collect(),
            bulges: None,
            zs: None,
            ms: None,
            parts: None,
            closed: false,
            widths: None,
        };
        objects.push(object(polyline(w.path.clone())));
        // The arrowhead at the exit: two strokes 25° off the last leg, as long as the text is high.
        let (a, b) = (w.path[w.path.len() - 2], w.path[w.path.len() - 1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx.hypot(dy);
        if len > 1e-9 {
            let (ux, uy) = (dx / len, dy / len);
            let s = height * 1.4;
            let side = |k: f64| {
                let (c, n) = (25f64.to_radians().cos(), 25f64.to_radians().sin() * k);
                V::new(b.x - s * (ux * c - uy * n), b.y - s * (uy * c + ux * n))
            };
            objects.push(object(polyline(vec![side(1.0), b, side(-1.0)])));
        }
        // What it measured, over its start, running the way the walk leaves it: into the room, not over the
        // neighbour's words beyond the wall.
        let p = w.path[0];
        let align = if w.path[1].x < p.x - 1e-9 {
            TextAlign::BottomRight
        } else {
            TextAlign::BottomLeft
        };
        objects.push(object(EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: kentos_commands::Vec2 {
                x: p.x,
                y: p.y + height * 0.5,
            },
            text: words(w),
            height,
            rotation: 0.0,
            align: Some(align),
            width_factor: None,
            mask: true,
            box_width: None,
            line_spacing: None,
            runs: Vec::new(),
            face: TextFace::default(),
            path: None,
        }));
    }
    let input = EntitiesCreate {
        layer_id: doc.layers().active().to_owned(),
        objects,
        operation: None,
        expected_revision: None,
    };
    match create::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { .. } => Ok(Drawn {
            walks: walks.len(),
            over: walks.iter().filter(|w| w.over).count(),
        }),
        CommandResult::Failed { error } | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Kaçış yolları yazılamadı.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_walk_says_its_measure_against_its_limit() {
        let w = crate::fire_audit::Walk {
            level: None,
            path: vec![V::new(0.0, 0.0), V::new(28.07, 0.0)],
            length: 28.07,
            measure: 28.07,
            limit: Some(15.0),
            words: "tek yön",
            over: true,
        };
        assert_eq!(words(&w), "tek yön 28,07 m > 15,00 m");
        let kept = crate::fire_audit::Walk {
            over: false,
            measure: 12.4,
            limit: Some(20.0),
            words: "daire içi",
            ..w.clone()
        };
        assert_eq!(words(&kept), "daire içi 12,40 m ≤ 20,00 m");
        let unasked = crate::fire_audit::Walk {
            limit: None,
            length: 9.8,
            ..w
        };
        assert_eq!(words(&unasked), "9,80 m");
    }
}
