//! Renk planı lejandı (docs/adr/8176 §3.3): the colour plan's legend of a storey's rooms, drawn with its top left at a
//! place: its title (the scheme and the storey), then for each row a swatch filled in its colour (a room not filled,
//! its outline only) and its words (“Oturma — 2 mahal, 36,40 m²”), then the total. Each object carries the attribute
//! `Renk planı`; drawn again, the old legend is taken away first: one legend in the drawing, as the plan shows one
//! storey at a time. Plain objects: they do not follow the rooms; the legend is drawn again after a change.
//!
//! The layout in the text's height `H` (2.5 mm on paper at the drawing's scale): the title `1.2H` high, row `i`'s
//! swatch `4H` wide and `2H` high, its top `2.4H + 3H·i` below the place, its words `5H` right of the place, level
//! with the swatch's middle.

use kentos_commands::{
    CommandResult, EntitiesCreate, EntityGeometry, HatchPattern, HatchPatternType, NewObject,
    TextAlign, Vec2,
};
use kentos_domain::Document;

use crate::{ExecutionContext, create};

/// The attribute the legend's objects carry: what a new legend takes away.
pub const MARK_ATTR: &str = "Renk planı";
/// The undo step's name.
pub const STEP: &str = "Renk planı lejandı";

/// Metres as a legend writes an area: 36,40. By the display rule (docs/adr/0149): the room's tag writes the same
/// area the same way.
fn m2(v: f64) -> String {
    kentos_geometry_core::display::fixed(v, 2).replace('.', ",")
}

/// A row's words: its label, its rooms and their area.
pub fn row_words(label: &str, rooms: usize, area: f64) -> String {
    format!("{label} — {rooms} mahal, {} m²", m2(area))
}

/// Draws the legend of the rooms on `level` (every room's: none) with its top left at `at`, on `layer`, the old legend
/// taken away first; how many rows it has. One undo step when the caller holds a group. None without a colour plan or
/// a room.
pub fn draw(
    doc: &mut Document,
    level: Option<&str>,
    at: Vec2,
    layer: &str,
) -> Result<usize, String> {
    let Some(scheme) = doc.color_plan() else {
        return Err("Renk planı kapalı: önce Mahal ▸ Renk planı'ndan bir şema seçin.".into());
    };
    let rows = doc.color_legend(level);
    if rows.is_empty() {
        return Err("Bu katta mahal yok: renk planının lejandı mahallerden yapılır.".into());
    }
    crate::marks::take_away(doc, MARK_ATTR, "Eski lejant")?;
    let h = 2.5 / 1000.0 * doc.settings().plot_scale;
    let p = |x: f64, y: f64| Vec2 {
        x: at.x + x,
        y: at.y + y,
    };
    let object = |geometry, color: Option<&str>, what: &str| NewObject {
        geometry,
        color: color.map(str::to_owned),
        line_weight: None,
        line_type: None,
        line_pattern: None,
        attrs: Some(std::iter::once((MARK_ATTR.to_owned(), what.to_owned())).collect()),
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    };
    let text = |at: Vec2, words: String, height: f64, align: TextAlign| EntityGeometry::Text {
        layout: None,
        annotative: None,
        p: at,
        text: words,
        height,
        rotation: 0.0,
        align: Some(align),
        width_factor: None,
        mask: false,
        box_width: None,
        line_spacing: None,
        runs: Vec::new(),
        face: Default::default(),
        path: None,
    };
    let level_name = level.and_then(|id| {
        doc.settings()
            .arch
            .as_ref()
            .and_then(|a| a.levels.iter().find(|l| l.id == id))
            .map(|l| l.name.clone())
    });
    let title = match &level_name {
        Some(name) => format!("Mahal renk planı: {} ({name})", scheme.label()),
        None => format!("Mahal renk planı: {}", scheme.label()),
    };
    let mut objects = vec![object(
        text(p(0.0, 0.0), title, 1.2 * h, TextAlign::TopLeft),
        None,
        "başlık",
    )];
    let (mut rooms, mut area) = (0, 0.0);
    for (i, r) in rows.iter().enumerate() {
        let top = -(2.4 * h + 3.0 * h * i as f64);
        let ring = vec![
            p(0.0, top - 2.0 * h),
            p(4.0 * h, top - 2.0 * h),
            p(4.0 * h, top),
            p(0.0, top),
        ];
        if let Some(color) = r.color {
            let fill = EntityGeometry::Hatch {
                ring: ring.clone(),
                holes: None,
                pattern: HatchPattern::user(HatchPatternType::Solid, 0.0, 1.0),
                assoc: None,
                boundary: None,
                annotative: None,
            };
            objects.push(object(fill, Some(color), &r.label));
        }
        let outline = EntityGeometry::Polyline {
            pts: ring,
            bulges: None,
            zs: None,
            ms: None,
            parts: None,
            closed: true,
            widths: None,
        };
        objects.push(object(outline, None, &r.label));
        let words = row_words(&r.label, r.rooms, r.area);
        objects.push(object(
            text(p(5.0 * h, top - h), words, h, TextAlign::MiddleLeft),
            None,
            &r.label,
        ));
        rooms += r.rooms;
        area += r.area;
    }
    let total_top = -(2.4 * h + 3.0 * rows.len() as f64 * h);
    objects.push(object(
        text(
            p(5.0 * h, total_top - h),
            row_words("Toplam", rooms, area),
            h,
            TextAlign::MiddleLeft,
        ),
        None,
        "toplam",
    ));
    let input = EntitiesCreate {
        layer_id: layer.to_owned(),
        objects,
        operation: None,
        expected_revision: None,
    };
    match create::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { .. } => Ok(rows.len()),
        CommandResult::Failed { error } | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Renk planının lejandı yazılamadı.".into()),
    }
}

#[cfg(test)]
mod tests {
    /// A room area that is a half at the digits shown is written as the room's own tag writes it (docs/adr/0149,
    /// docs/adr/9401 §9): 12.125 m² is 12,13, not 12,12.
    #[test]
    fn a_half_is_written_by_the_display_rule() {
        assert_eq!(
            super::row_words("Konut", 3, 12.125),
            "Konut — 3 mahal, 12,13 m²"
        );
        assert_eq!(
            super::row_words("Konut", 1, 36.4),
            "Konut — 1 mahal, 36,40 m²"
        );
    }
}
