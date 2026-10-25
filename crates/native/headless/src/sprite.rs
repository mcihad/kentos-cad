//! A MapLibre style's sprite (docs/adr/0635 §9): the icons its shape markers name (`kentos_maplibre::SpriteIcon`,
//! in CSS pixels) drawn with the drawing's own marker shapes (`kentos_native_style::shapes`, as the canvas and the
//! PDF draw them) into one PNG and its JSON index, at a pixel ratio (1 and 2, MapLibre's `@2x`). The same icons make
//! the same files.

use std::collections::BTreeMap;

use kentos_maplibre::SpriteIcon;
use kentos_native_style::shapes::{OPEN, shape_path};
use serde_json::json;
use tiny_skia::{Color, FillRule, Paint, Pixmap, Stroke, Transform};

/// A sprite's index and picture.
pub struct Sprite {
    pub json: Vec<u8>,
    pub png: Vec<u8>,
}

/// The widest row of icons, pixels at ratio 1.
const ROW: f64 = 512.0;

fn colour(hex: &str) -> Option<Color> {
    let h = hex.strip_prefix('#')?;
    let b = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    let a = if h.len() == 8 { b(6)? } else { 255 };
    Some(Color::from_rgba8(b(0)?, b(2)?, b(4)?, a))
}

/// The icons drawn at `ratio` into a sprite.
pub fn draw(icons: &[SpriteIcon], ratio: f64) -> Result<Sprite, String> {
    // Cells left to right, wrapping at the row's width: (x, y, side) each.
    let mut cells = Vec::with_capacity(icons.len());
    let (mut x, mut y, mut row_h, mut width) = (0u32, 0u32, 0u32, 1u32);
    for icon in icons {
        let side = ((icon.size + icon.stroke_width) * ratio).ceil().max(1.0) as u32 + 2;
        if x > 0 && f64::from(x + side) > ROW * ratio {
            x = 0;
            y += row_h;
            row_h = 0;
        }
        cells.push((x, y, side));
        x += side;
        row_h = row_h.max(side);
        width = width.max(x);
    }
    let height = (y + row_h).max(1);
    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| "Sprite'ın resmi kurulamadı.".to_owned())?;
    let mut index = BTreeMap::new();
    for (icon, &(cx, cy, side)) in icons.iter().zip(&cells) {
        let half = (icon.size * ratio / 2.0) as f32;
        let centre = (cx as f32 + side as f32 / 2.0, cy as f32 + side as f32 / 2.0);
        // The shapes are y up; the picture y down.
        let t = Transform::from_row(1.0, 0.0, 0.0, -1.0, centre.0, centre.1);
        if let Some(path) = shape_path(&icon.shape, half, half, [0.0; 4]) {
            let open = OPEN.contains(&icon.shape.as_str());
            if !open && let Some(c) = icon.fill.as_deref().and_then(colour) {
                let mut paint = Paint::default();
                paint.set_color(c);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::EvenOdd, t, None);
            }
            // An open shape is drawn by its line: its outline's colour, else its fill's.
            let line = icon
                .stroke
                .as_deref()
                .or(if open { icon.fill.as_deref() } else { None })
                .and_then(colour);
            if let Some(c) = line {
                let w = if icon.stroke_width > 0.0 {
                    icon.stroke_width
                } else if open {
                    1.0
                } else {
                    0.0
                };
                if w > 0.0 {
                    let mut paint = Paint::default();
                    paint.set_color(c);
                    paint.anti_alias = true;
                    let stroke = Stroke {
                        width: (w * ratio) as f32,
                        ..Stroke::default()
                    };
                    pixmap.stroke_path(&path, &paint, &stroke, t, None);
                }
            }
        }
        index.insert(
            icon.id.clone(),
            json!({ "width": side, "height": side, "x": cx, "y": cy, "pixelRatio": ratio }),
        );
    }
    let mut samples = Vec::with_capacity((width * height * 4) as usize);
    for p in pixmap.pixels() {
        let c = p.demultiply();
        samples.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    let png = kentos_formats::raster::png::encode(width, height, 4, &samples, 6)
        .map_err(|e| e.to_string())?;
    let mut json = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?;
    json.push('\n');
    Ok(Sprite {
        json: json.into_bytes(),
        png,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(shape: &str, id: &str) -> SpriteIcon {
        SpriteIcon {
            id: id.into(),
            shape: shape.into(),
            size: 10.0,
            fill: Some("#ff0000".into()),
            stroke: Some("#000000".into()),
            stroke_width: 1.0,
        }
    }

    #[test]
    fn icons_go_into_a_sprite_twice_as_large_at_2x() {
        let icons = [icon("star", "a"), icon("cross", "b"), icon("square", "c")];
        let one = draw(&icons, 1.0).expect("draws");
        let two = draw(&icons, 2.0).expect("draws");
        let i1: serde_json::Value = serde_json::from_slice(&one.json).expect("json");
        let i2: serde_json::Value = serde_json::from_slice(&two.json).expect("json");
        assert_eq!(i1["a"]["width"], 13);
        assert_eq!(i2["a"]["width"], 24);
        assert_eq!(i2["a"]["pixelRatio"], 2.0);
        assert_eq!(i1["b"]["x"], 13);
        assert_eq!(&one.png[1..4], b"PNG");
        // The same icons, the same files.
        assert_eq!(draw(&icons, 1.0).expect("draws").png, one.png);
    }
}
