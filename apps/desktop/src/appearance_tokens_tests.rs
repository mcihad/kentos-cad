//! The desktop's two shared themes are the web's (DESIGN.md §3.1–3.2,
//! docs/adr/0129): KentOS UI's dark and light tokens are read against the
//! web's `tokens.css`, the one place the palette is written for both. A
//! colour changed on one side and not on the other fails here.

use iced::Color;
use kentos_ui::theme::{Mode, Tokens};

const CSS: &str = include_str!("../../web/src/styles/tokens.css");

/// A theme's block in the stylesheet: `:root[data-theme='dark'] { … }`.
fn block(theme: &str) -> &'static str {
    let head = format!(":root[data-theme='{theme}'] {{");
    let start = CSS
        .find(&head)
        .unwrap_or_else(|| panic!("the {theme} block"));
    let body = &CSS[start + head.len()..];
    &body[..body.find("\n}").expect("the block's end")]
}

/// A colour token's value in a block: `#rrggbb`, or `rgba(r, g, b, a)`.
fn token(block: &str, name: &str) -> [f32; 4] {
    let key = format!("{name}:");
    let line = block
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(&key))
        .unwrap_or_else(|| panic!("{name} in the block"));
    let value = line[key.len()..].trim().trim_end_matches(';').trim();
    if let Some(hex) = value.strip_prefix('#') {
        let rgb = u32::from_str_radix(hex, 16).expect("a hex colour");
        let byte = |shift: u32| ((rgb >> shift) & 0xff) as f32 / 255.0;
        return [byte(16), byte(8), byte(0), 1.0];
    }
    let parts: Vec<f32> = value
        .trim_start_matches("rgba(")
        .trim_end_matches(')')
        .split(',')
        .map(|p| p.trim().parse().expect("a number"))
        .collect();
    [
        parts[0] / 255.0,
        parts[1] / 255.0,
        parts[2] / 255.0,
        parts[3],
    ]
}

/// `top` laid on `under`, as the browser draws a translucent colour.
fn over(under: [f32; 4], top: [f32; 4]) -> [f32; 4] {
    let mix = |i: usize| under[i] + (top[i] - under[i]) * top[3];
    [mix(0), mix(1), mix(2), 1.0]
}

fn bytes(c: [f32; 4]) -> [u8; 3] {
    [0, 1, 2].map(|i| (c[i] * 255.0).round() as u8)
}

fn same(name: &str, desktop: Color, web: [f32; 4]) {
    let d = desktop.into_rgba8();
    let w = bytes(web);
    for i in 0..3 {
        assert!(
            (i16::from(d[i]) - i16::from(w[i])).abs() <= 1,
            "{name}: desktop {:02x}{:02x}{:02x}, web {:02x}{:02x}{:02x}",
            d[0],
            d[1],
            d[2],
            w[0],
            w[1],
            w[2],
        );
    }
}

#[test]
fn the_dark_and_light_themes_are_the_webs() {
    for (mode, theme) in [(Mode::Dark, "dark"), (Mode::Light, "light")] {
        let css = block(theme);
        let t = Tokens::base(mode);
        let web = |name| token(css, name);
        same(&format!("{theme} window"), t.window, web("--c-menubar"));
        same(&format!("{theme} surface"), t.surface, web("--c-panel"));
        same(
            &format!("{theme} surface_alt"),
            t.surface_alt,
            web("--c-panel-head"),
        );
        same(&format!("{theme} header"), t.header, web("--c-panel-head"));
        same(
            &format!("{theme} surface_hover"),
            t.surface_hover,
            over(web("--c-panel"), web("--c-hover")),
        );
        same(&format!("{theme} field"), t.field, web("--c-field"));
        same(&format!("{theme} border"), t.border, web("--c-line"));
        same(
            &format!("{theme} border_strong"),
            t.border_strong(),
            web("--c-line-strong"),
        );
        same(&format!("{theme} text"), t.text, web("--c-text"));
        same(&format!("{theme} muted"), t.muted, web("--c-text-2"));
        same(&format!("{theme} popover"), t.popover, web("--c-popover"));
        same(&format!("{theme} success"), t.success, web("--c-ok"));
        same(&format!("{theme} warning"), t.warning, web("--c-warn"));
        same(&format!("{theme} danger"), t.danger, web("--c-danger"));
        same(&format!("{theme} info"), t.info, web("--c-info"));
    }
}
