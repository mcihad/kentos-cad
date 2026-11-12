//! A chart's colours (docs/adr/0227 §10; the web's `ui/charts/drawChart.ts`):
//! the roles of the core's drawing list as the theme gives them, the chart
//! tokens `--chart-1` … `--chart-8` and `--chart-other` of the web's
//! `tokens.css` (a test keeps them equal), and the light theme's colours on
//! white paper for a saved picture.

use iced::Color;
use kentos_geometry_core::ops::charts::layout::Role;
use kentos_geometry_core::ops::charts::svg::Palette;
use kentos_ui::theme::Tokens;

/// The categorical order on the dark surface (`tokens.css`, dark).
pub const DARK_SERIES: [u32; 8] = [
    0x3987e5, 0xd95926, 0x199e70, 0xc98500, 0xd55181, 0x008300, 0x9085e9, 0xe66767,
];
pub const DARK_OTHER: u32 = 0x5d6875;
/// The same hues on the light surface (`tokens.css`, light).
pub const LIGHT_SERIES: [u32; 8] = [
    0x2a78d6, 0xeb6834, 0x1baf7a, 0xeda100, 0xe87ba4, 0x008300, 0x4a3aa7, 0xe34948,
];
pub const LIGHT_OTHER: u32 = 0x9aa3ad;

fn hex(rgb: u32) -> Color {
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// A role's colour in the theme.
pub fn color(t: &Tokens, role: Role) -> Color {
    let (series, other) = if t.is_dark {
        (&DARK_SERIES, DARK_OTHER)
    } else {
        (&LIGHT_SERIES, LIGHT_OTHER)
    };
    match role {
        Role::Surface => t.surface,
        Role::Grid => t.border,
        Role::Axis => t.border_strong(),
        Role::Ink1 => t.text,
        Role::Ink2 => t.muted,
        Role::Ink3 => t.faint,
        Role::Series(k) => series.get(k as usize).map_or(t.muted, |c| hex(*c)),
        Role::Other => hex(other),
        Role::Selection => t.accent,
    }
}

fn css(rgb: u32) -> String {
    format!("#{rgb:06x}")
}

/// The light theme's colours on white paper and the interface's typeface: a
/// saved picture's (the web's `PAPER`).
pub fn paper(font: &str) -> Palette {
    Palette {
        background: "#ffffff".into(),
        surface: "#ffffff".into(),
        grid: "#d3d9df".into(),
        axis: "#b5bfc9".into(),
        ink1: "#1b232c".into(),
        ink2: "#4d5966".into(),
        ink3: "#7a8591".into(),
        series: LIGHT_SERIES.iter().map(|c| css(*c)).collect(),
        other: css(LIGHT_OTHER),
        selection: "#1f4a96".into(),
        font: font.to_owned(),
    }
}
