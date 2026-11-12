//! The picture Lejant saves (the web's `savePng`): white paper, black ink,
//! twice its size; where everything goes is `legend_layout`'s, held to
//! `fixtures/style/v1/legend.json`. The symbols are drawn by the drawing's
//! own styled pipelines (`thumbs.rs`), so the picture is what the map draws.
//!
//! It is drawn off the window, by KentOS UI's snapshot renderer on a device
//! of its own, in strips of lines (a tall legend is more than one texture);
//! the strips are stacked into one PNG.

use std::sync::Arc;

use iced::font::{Family, Weight};
use iced::widget::text::LineHeight;
use iced::widget::{Column, Row, container, space, stack, text};
use iced::{Color, Element, Font, Length, Padding, Size};
use kentos_native_style::StylePalette;
use kentos_native_style::legend::{BOTTOM, LegendLayout, LegendRow, LegendText, PAPER, ROW, TOP};
use kentos_native_style::library::StyleLibrary;
use kentos_ui::snapshot::Snapshot;
use serde_json::Value;

use crate::style::images::Images;
use crate::style::thumbs::{Look, Thumbs};

/// The tallest legend saved, in logical pixels: twice this is the tallest
/// picture a browser draws (32 767 pixels), so the web keeps the same limit.
pub const MAX_HEIGHT: i64 = 16_383;

/// How many lines (headings and rows) the tallest legend holds.
pub fn max_lines() -> i64 {
    (MAX_HEIGHT - TOP - BOTTOM) / ROW
}

/// Lines drawn at once: 60 lines of 30 pixels at 2× are 3 600 pixels, within every GPU's textures.
const STRIP: usize = 60;
/// A strip's height at most: the heading above its lines and the margin below.
const STRIP_HEIGHT: i64 = TOP + STRIP as i64 * ROW + BOTTOM;
/// Draws of a strip while its pictures still wait for the atlas.
const TRIES: usize = 20;

/// What the picture needs, owned, so it is drawn off the window's thread.
pub struct Sheet {
    pub layout: LegendLayout,
    /// Each entry's symbol, in the order of the layout's entries.
    pub symbols: Vec<Option<Value>>,
    /// Each entry's fixed scale (CSS px per paper mm: Orantılı sembol's rows, docs/adr/0213 §2.2).
    pub scales: Vec<Option<f64>>,
    pub library: StyleLibrary,
    pub images: Arc<Images>,
}

fn color(hex: &str) -> Color {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// “700 12px Arial, …”: the weight and the size.
fn weight_and_size(font: &str) -> (u16, f32) {
    let mut parts = font.split_whitespace();
    let weight = parts.next().and_then(|w| w.parse().ok()).unwrap_or(400);
    let size = parts
        .next()
        .and_then(|s| s.trim_end_matches("px").parse().ok())
        .unwrap_or(12.0);
    (weight, size)
}

/// Where Arimo (Arial's metric twin, shipped with KentOS) puts its baseline
/// in a line as tall as the text: half the leading above the ascent.
const BASELINE: f32 = 0.8465;

/// A text of the layout, its top where its baseline falls on `baseline`
/// (pixels from the top of its box).
fn words<'a>(t: &LegendText, baseline: f32) -> Element<'a, ()> {
    let (weight, size) = weight_and_size(&t.font);
    let font = Font {
        family: Family::Name("Arimo"),
        weight: if weight >= 600 {
            Weight::Bold
        } else {
            Weight::Normal
        },
        ..Font::DEFAULT
    };
    let ink = color(&t.color);
    container(
        text(t.text.clone())
            .font(font)
            .size(size)
            .line_height(LineHeight::Relative(1.0))
            .color(ink),
    )
    .padding(Padding {
        top: (baseline - BASELINE * size).max(0.0),
        ..Padding::ZERO
    })
    .into()
}

/// A line of 30 pixels at `row_top`: a heading, or an entry's framed
/// picture and words, where the layout puts them.
fn line<'a>(
    row: &LegendRow,
    row_top: i64,
    symbol: Option<&Value>,
    px_per_mm: Option<f64>,
    thumbs: &Thumbs,
    look: &Look<'_>,
) -> Element<'a, ()> {
    let mut out = Row::new().height(Length::Fixed(ROW as f32));
    let mut x = 0;
    if let (Some(p), Some(symbol)) = (&row.picture, symbol) {
        let frame = color(&p.frame);
        let width = p.frame_width as f32;
        let (w, h) = (p.w as f32, p.h as f32);
        // The frame over the picture, as the web strokes it after drawing.
        let framed = stack![
            thumbs.picture(symbol, None, (w, h), px_per_mm, look),
            container(space())
                .width(Length::Fixed(w))
                .height(Length::Fixed(h))
                .style(move |_| container::Style {
                    border: iced::Border {
                        color: frame,
                        width,
                        radius: 0.0.into(),
                    },
                    ..container::Style::default()
                }),
        ];
        out = out
            .push(space().width(Length::Fixed(p.x as f32)))
            .push(container(framed).padding(Padding {
                top: (p.y - row_top) as f32,
                ..Padding::ZERO
            }));
        x = p.x + p.w;
    }
    out.push(space().width(Length::Fixed((row.label.x - x).max(0) as f32)))
        .push(words(&row.label, (row.label.y - row_top) as f32))
        .into()
}

/// One strip: lines `from..to`, with the heading above the first strip and
/// the margin below the last.
fn strip<'a>(
    sheet: &Sheet,
    (from, to): (usize, usize),
    entries_before: usize,
    thumbs: &Thumbs,
    look: &Look<'_>,
) -> Element<'a, ()> {
    let layout = &sheet.layout;
    let mut column = Column::new().width(Length::Fixed(layout.width as f32));
    if from == 0 {
        let head = Row::new()
            .height(Length::Fixed(TOP as f32))
            .push(space().width(Length::Fixed(layout.heading.x as f32)))
            .push(words(&layout.heading, layout.heading.y as f32))
            .push(space::horizontal())
            .push(words(&layout.name, layout.name.y as f32))
            .push(space().width(Length::Fixed((layout.width - layout.name.x) as f32)));
        column = column.push(head);
    }
    let mut entry = entries_before;
    for (i, row) in layout.rows[from..to].iter().enumerate() {
        let (symbol, px_per_mm) = if row.kind == "entry" {
            let s = sheet.symbols.get(entry).and_then(Option::as_ref);
            let k = sheet.scales.get(entry).copied().flatten();
            entry += 1;
            (s, k)
        } else {
            (None, None)
        };
        let row_top = TOP + (from + i) as i64 * ROW;
        column = column.push(line(row, row_top, symbol, px_per_mm, thumbs, look));
    }
    let background = color(&layout.background);
    container(column)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(background.into()),
            ..container::Style::default()
        })
        .into()
}

/// The legend as PNG bytes, or why it could not be drawn.
pub fn png(sheet: &Sheet) -> Result<Vec<u8>, String> {
    let layout = &sheet.layout;
    let scale = layout.scale as f32;
    let (w, h) = (
        (layout.width as f32 * scale) as usize,
        (layout.height as f32 * scale) as usize,
    );
    let size = Size::new(layout.width as f32, STRIP_HEIGHT as f32);
    // Unit tests draw with the software renderer: GPU devices opened by tests
    // in parallel crash drivers (KentOS UI's snapshot); pictures for the owner
    // name a backend (KENTOS_SNAPSHOT_BACKEND=wgpu).
    let snapshot = if cfg!(test) && std::env::var_os("KENTOS_SNAPSHOT_BACKEND").is_none() {
        Snapshot::software(size)
    } else {
        Snapshot::new(size)
    };
    let mut snapshot = snapshot
        .map_err(|_| "Lejant çizilemedi: çizim aygıtı açılmadı.".to_owned())?
        .scale(scale);
    let palette = StylePalette {
        fg: PAPER.fg.to_owned(),
        fg_dim: PAPER.fg_dim.to_owned(),
        ink: PAPER.ink.to_owned(),
        paper: PAPER.paper.to_owned(),
    };
    let look = Look {
        palette: &palette,
        library: &sheet.library,
        images: &sheet.images,
    };
    let thumbs = Thumbs::default();
    let theme = iced::Theme::Light;
    let mut rgba = vec![255u8; w * h * 4];
    let mut y = 0usize;
    let mut from = 0usize;
    let mut entries = 0usize;
    let count = layout.rows.len();
    loop {
        let to = (from + STRIP).min(count);
        let mut image = snapshot.render(strip(sheet, (from, to), entries, &thumbs, &look), &theme);
        for _ in 0..TRIES {
            if !thumbs.pending() {
                break;
            }
            image = snapshot.render(strip(sheet, (from, to), entries, &thumbs, &look), &theme);
        }
        // The strip's own height; the rest of the texture is white paper.
        let mut logical = (to - from) as i64 * ROW;
        if from == 0 {
            logical += TOP;
        }
        if to == count {
            logical += BOTTOM;
        }
        let rows = ((logical as f32 * scale) as usize).min(image.height as usize);
        let stride = (image.width as usize).min(w) * 4;
        for r in 0..rows {
            if y + r >= h {
                break;
            }
            let src = r * image.width as usize * 4;
            let dst = (y + r) * w * 4;
            rgba[dst..dst + stride].copy_from_slice(&image.rgba[src..src + stride]);
        }
        y += rows;
        entries += layout.rows[from..to]
            .iter()
            .filter(|r| r.kind == "entry")
            .count();
        if to == count {
            break;
        }
        from = to;
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, w as u32, h as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("Lejant PNG'ye yazılamadı: {e}"))?;
        writer
            .write_image_data(&rgba)
            .map_err(|e| format!("Lejant PNG'ye yazılamadı: {e}"))?;
    }
    Ok(bytes)
}
