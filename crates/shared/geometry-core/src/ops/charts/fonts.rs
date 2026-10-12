//! How wide a chart's text is (docs/adr/0227 §5): an estimate from the
//! interface typeface's average letter and its widest digit, measured in
//! the web's own faces (the letters on the Turkish interface sample, as
//! KentOS UI's `typography` keeps them; the digits in Chrome, 12 Oct 2026).
//! The layout places and shortens labels with it, so both platforms lay a
//! chart out alike; letters are counted 8 % wider so a label never runs
//! past its room.

/// A family's average letter and widest digit (em).
fn advances(family: &str) -> (f32, f32) {
    match family {
        "Plus Jakarta Sans" => (0.472, 0.732),
        "Inter" => (0.483, 0.646),
        "IBM Plex Sans" => (0.456, 0.600),
        "Source Sans 3" => (0.425, 0.497),
        "Noto Sans" => (0.480, 0.572),
        "Roboto" => (0.454, 0.562),
        // The system's face is unknown: a wide sans (DejaVu Sans), as KentOS UI counts it.
        _ => (0.520, 0.636),
    }
}

/// A text's estimated width in pixels at `size` px.
pub fn width(text: &str, size: f32, family: &str) -> f32 {
    let (letter, digit) = advances(family);
    let em: f32 = text
        .chars()
        .map(|c| match c {
            '0'..='9' => digit,
            '.' | ',' | ':' | ';' | '\'' | '|' | '!' | 'i' | 'l' | 'ı' => 0.30,
            ' ' => 0.26,
            '-' | '+' | '−' => 0.55,
            '%' => 0.98,
            '…' => 0.85,
            _ => letter * 1.08,
        })
        .sum();
    em * size
}

/// The text shortened with “…” to fit `room` pixels (whole when it fits;
/// “…” alone when not even one character does).
pub fn fit(text: &str, room: f32, size: f32, family: &str) -> String {
    if width(text, size, family) <= room {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    for n in (1..chars.len()).rev() {
        let mut t: String = chars[..n].iter().collect::<String>().trim_end().to_owned();
        t.push('…');
        if width(&t, size, family) <= room {
            return t;
        }
    }
    "…".to_owned()
}
