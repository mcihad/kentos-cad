//! The drawing of a Netcad 8 smart object. Netcad calls these "akıllı nesne":
//! the settlement, construction, road-width, plan-note and function-name
//! symbols its Planet module places on a zoning plan. A file keeps only their
//! properties (`nizam=AYRIK`, `kat=3`, `taks=0.4`) and Netcad draws the symbol
//! when it shows them; its own DXF export leaves most of them out.
//!
//! WHAT THEY LOOK LIKE was taken from Netcad's documentation of the tools that
//! make them ("2.4 Sembol İşlemleri", wiki.netcad.com.tr), measured against
//! the symbol's radius: ten metres at an object size of one, which is what the
//! record's own bounding box says (20 × size across, on all 15 722 objects of
//! the Sivas UİP):
//!
//! - Yerleşim: a circle; the front garden above a short dash, the side garden
//!   below it, the order (A, B, BL…) left of it and the storeys right.
//! - Yapılaşma: a circle split by a line, TAKS over KAKS (`0.30-0.40` when a
//!   minimum is set); or, for Emsal, `E=1.00` and its lines, bare.
//! - Yol: a circle; the width's whole metres, and its two decimals raised,
//!   small and underlined: `17⁰⁰`.
//! - Plan Notu: its RTF as text, wrapped to the note's box, and the box.
//! - Fonksiyon Adı: the function's name.
//!
//! Built in the symbol's own space (metres, anchor at the origin, size one);
//! `emit` scales and turns each stroke by the object's size and rotation.

use crate::format::{Entity, SmartClass};

/// The symbol's radius at an object size of one, in metres.
const R: f64 = 10.0;

/// Where a text sits on its point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    BaselineLeft,
    MiddleLeft,
    MiddleCentre,
    MiddleRight,
    TopLeft,
}

/// One stroke of a symbol, in the symbol's own space.
#[derive(Clone, Debug, PartialEq)]
pub enum Stroke {
    Circle {
        centre: (f64, f64),
        radius: f64,
    },
    /// Two points are a line; `closed` a box.
    Polyline {
        points: Vec<(f64, f64)>,
        closed: bool,
    },
    Text {
        text: String,
        at: (f64, f64),
        height: f64,
        anchor: Anchor,
        /// A text that breaks to this width; 0 does not wrap. Paragraphs are
        /// on their own lines (`\n`).
        wrap: f64,
    },
}

/// A drawn symbol: what it reads as (the summary of the report) and its strokes.
#[derive(Clone, Debug, PartialEq)]
pub struct Symbol {
    pub summary: String,
    pub strokes: Vec<Stroke>,
}

/// The value of property `name` when the object shows it: present, not
/// switched off by its `chk…IsNull`, not empty.
fn shown(e: &Entity, name: &str) -> Option<String> {
    let p = e.properties.iter().find(|p| p.name == name)?;
    if p.null {
        return None;
    }
    let v = p.value.trim_matches(' ');
    (!v.is_empty()).then(|| v.to_owned())
}

/// The value of property `name` whatever its switch says: the symbol's own
/// settings (`choiceType`, `numberOfDecimalPlace`) have none.
fn setting<'e>(e: &'e Entity, name: &str) -> Option<&'e str> {
    e.properties
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.value.as_str())
}

/// `value` as a number, `.` or `,` for the decimal point.
fn number(value: &str) -> Option<f64> {
    let v = value.replace(',', ".");
    // Rust's parser takes what the C++ `from_chars` took and no more: no
    // leading `+`, no spaces.
    if v.starts_with('+') || v.is_empty() {
        return None;
    }
    v.parse::<f64>().ok().filter(|x| x.is_finite())
}

/// `value` with `decimals` places, as Netcad prints a TAKS or a road width
/// (`0.4` is `0.40`); the value as written when it is not a number.
fn fixed(value: &str, decimals: usize) -> String {
    match number(value) {
        Some(v) => format!("{v:.prec$}", prec = decimals.min(6)),
        None => value.to_owned(),
    }
}

/// The web's `foldTurkish`, enough for three words: upper case, Turkish
/// letters as plain ones.
fn fold(text: &str) -> String {
    text.trim()
        .to_uppercase()
        .chars()
        .map(|c| match c {
            'Ç' => 'C',
            'Ğ' => 'G',
            'İ' => 'I',
            'Ö' => 'O',
            'Ş' => 'S',
            'Ü' => 'U',
            c => c,
        })
        .collect()
}

/// The letters a settlement symbol writes for its order. Only the three
/// Netcad's documentation shows and a real plan uses are shortened (`A`,
/// `B`, `BL`); any other order is written out, which is readable, rather
/// than abbreviated by a guess, which could be wrong.
fn order_code(nizam: &str) -> String {
    match fold(nizam).as_str() {
        "AYRIK" => "A".to_owned(),
        "BITISIK" => "B".to_owned(),
        "BLOK" => "BL".to_owned(),
        _ => nizam.to_owned(),
    }
}

fn circle(r: f64) -> Stroke {
    Stroke::Circle {
        centre: (0.0, 0.0),
        radius: r,
    }
}

fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> Stroke {
    Stroke::Polyline {
        points: vec![(x0, y0), (x1, y1)],
        closed: false,
    }
}

fn text(words: impl Into<String>, anchor: Anchor, x: f64, y: f64, height: f64) -> Stroke {
    Stroke::Text {
        text: words.into(),
        at: (x, y),
        height,
        anchor,
        wrap: 0.0,
    }
}

/// `Yerleşim`: the garden distances over and under a dash, the order and the
/// storeys either side of it: `5 / A–3 / 3`.
fn settlement(e: &Entity) -> Option<Symbol> {
    let on = shown(e, "txtOn");
    let arka = shown(e, "txtArka");
    let yan = shown(e, "txtYan");
    let kat = shown(e, "kat");
    let code = shown(e, "nizam")
        .map(|n| order_code(&n))
        .unwrap_or_default();
    let mut top = on.unwrap_or_default();
    if let Some(a) = arka {
        top = if top.is_empty() {
            a
        } else {
            format!("{top}-{a}")
        };
    }

    let h = 0.45 * R;
    let mut strokes = vec![circle(R)];
    match &kat {
        Some(k) => {
            if !code.is_empty() {
                strokes.push(text(code.clone(), Anchor::MiddleRight, -0.16 * R, 0.0, h));
            }
            strokes.push(line(-0.1 * R, 0.0, 0.1 * R, 0.0));
            strokes.push(text(k.clone(), Anchor::MiddleLeft, 0.16 * R, 0.0, h));
        }
        None if !code.is_empty() => {
            strokes.push(text(code.clone(), Anchor::MiddleCentre, 0.0, 0.0, h))
        }
        None => {}
    }
    if !top.is_empty() {
        strokes.push(text(top, Anchor::MiddleCentre, 0.0, 0.52 * R, h));
    }
    if let Some(y) = &yan {
        strokes.push(text(y.clone(), Anchor::MiddleCentre, 0.0, -0.52 * R, h));
    }
    if strokes.len() == 1 {
        return None; // nothing to say
    }
    let middle = match &kat {
        Some(k) => format!("{code}-{k}"),
        None => code,
    };
    Some(Symbol {
        summary: format!("yerleşim {middle}"),
        strokes,
    })
}

/// `Yapılaşma`: TAKS over KAKS in a split circle, or Emsal, Hmax and Yençok
/// as lines of text.
fn construction(e: &Entity) -> Option<Symbol> {
    let mut decimals = 2usize;
    if let Some(d) = setting(e, "numberOfDecimalPlace").and_then(number)
        && (0.0..=6.0).contains(&d)
    {
        decimals = d as usize;
    }
    let pair = |value: &str, minimum: &str| -> String {
        let Some(v) = shown(e, value) else {
            return String::new();
        };
        match shown(e, minimum) {
            Some(m) => format!("{}-{}", fixed(&m, decimals), fixed(&v, decimals)),
            None => fixed(&v, decimals),
        }
    };
    let taks = pair("taks", "minTaks");
    let kaks = pair("kaks", "minKaks");
    let ratios =
        setting(e, "choiceType").unwrap_or("1") != "0" && (!taks.is_empty() || !kaks.is_empty());

    if ratios {
        let range = shown(e, "minTaks").is_some() || shown(e, "minKaks").is_some();
        let h = if range { 0.27 } else { 0.48 } * R;
        let mut strokes = vec![circle(R)];
        if !taks.is_empty() && !kaks.is_empty() {
            let w = if range { 0.9 } else { 0.62 } * R;
            strokes.push(line(-w, 0.0, w, 0.0));
            strokes.push(text(taks.clone(), Anchor::MiddleCentre, 0.0, 0.36 * R, h));
            strokes.push(text(kaks.clone(), Anchor::MiddleCentre, 0.0, -0.36 * R, h));
        } else {
            let one = if taks.is_empty() { &kaks } else { &taks };
            strokes.push(text(one.clone(), Anchor::MiddleCentre, 0.0, 0.0, h));
        }
        return Some(Symbol {
            summary: format!("yapılaşma TAKS {taks}, KAKS {kaks}"),
            strokes,
        });
    }

    let mut lines: Vec<String> = Vec::new();
    if let Some(v) = shown(e, "emsal") {
        lines.push(format!("E={}", fixed(&v, decimals)));
    }
    if let Some(v) = shown(e, "hmax") {
        let unit = shown(e, "HmaxType")
            .map(|u| format!(" {u}"))
            .unwrap_or_default();
        lines.push(format!("Hmax={}{unit}", fixed(&v, decimals)));
    }
    // In metres unless the symbol names its unit (`Kat`): the plan's own text
    // beside such a symbol reads `Yençok=15.50 m`.
    if let Some(v) = shown(e, "yEncok") {
        let unit = shown(e, "YencokType").unwrap_or_else(|| "m".to_owned());
        lines.push(format!("Yençok={} {unit}", fixed(&v, decimals)));
    }
    if lines.is_empty() {
        return None;
    }
    let n = lines.len() as f64;
    let strokes = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let y = (-0.15 + ((n - 1.0) / 2.0 - i as f64) * 0.93) * R;
            text(l.clone(), Anchor::MiddleLeft, -1.65 * R, y, 0.5 * R)
        })
        .collect();
    Some(Symbol {
        summary: format!("yapılaşma {}", lines.join(", ")),
        strokes,
    })
}

/// `Yol`: the width's whole metres, its decimals raised, small, underlined.
/// `underline` is the width of the decimals at their height, from the
/// drawing's typeface.
fn road(e: &Entity, measure: &dyn Fn(&str, f64) -> f64) -> Option<Symbol> {
    let w = shown(e, "genislik")?;
    let full = fixed(&w, 2);
    let (whole, frac) = match full.find('.') {
        Some(dot) => (full[..dot].to_owned(), full[dot + 1..].to_owned()),
        None => (full.clone(), "00".to_owned()),
    };
    let small = 0.28 * R;
    let under = measure(&frac, small);
    let strokes = vec![
        circle(R),
        text(whole, Anchor::MiddleRight, -0.05 * R, 0.0, 0.55 * R),
        text(frac, Anchor::BaselineLeft, 0.12 * R, 0.06 * R, small),
        line(0.12 * R, 0.02 * R, 0.12 * R + under, 0.02 * R),
    ];
    Some(Symbol {
        summary: format!("yol genişliği {full} m"),
        strokes,
    })
}

/// `Fonksiyon Adı`: the function's name, from its anchor.
fn function_name(e: &Entity) -> Option<Symbol> {
    let name = shown(e, "adi")?;
    Some(Symbol {
        summary: format!("fonksiyon adı {name}"),
        strokes: vec![text(name, Anchor::MiddleLeft, 0.0, 0.05 * R, 0.5 * R)],
    })
}

/// `Plan Notu`: its text in its box, the box's top left at the anchor.
fn plan_note(e: &Entity) -> Option<Symbol> {
    let data = setting(e, "rtfData")?;
    let w = number(setting(e, "width")?)?;
    let h = number(setting(e, "height")?)?;
    if !(w > 0.0 && h > 0.0 && w <= 100_000.0 && h <= 100_000.0) {
        return None;
    }
    let body = rtf_text(&base64_decode(data));
    if body.is_empty() {
        return None;
    }
    // As tall as the note's paragraphs fit its box, and never wider than the
    // longest line allows: the RTF's own point sizes are relative to a page
    // Netcad scales into the box, so the box is what is known.
    let lines = body.split('\n').count();
    let longest = body
        .split('\n')
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    let mut height = h / (lines as f64 * 1.45);
    if longest > 0 {
        height = height.min(w / (longest as f64 * 0.62));
    }
    height = height.max(h / 400.0);
    let first: String = body
        .split('\n')
        .next()
        .unwrap_or("")
        .chars()
        .take(40)
        .collect();
    Some(Symbol {
        summary: format!("plan notu: {first}"),
        strokes: vec![
            Stroke::Polyline {
                points: vec![(0.0, 0.0), (w, 0.0), (w, -h), (0.0, -h)],
                closed: true,
            },
            Stroke::Text {
                text: body,
                at: (0.02 * w, -0.02 * h),
                height,
                anchor: Anchor::TopLeft,
                wrap: 0.96 * w,
            },
        ],
    })
}

/// The drawing of a Netcad 8 smart object, or nothing for a class this reader
/// does not draw (which is then read as a point with its properties).
/// `measure(text, height)` is a text's width in the drawing's typeface.
pub fn planet_symbol(e: &Entity, measure: &dyn Fn(&str, f64) -> f64) -> Option<Symbol> {
    match e.smart {
        SmartClass::Settlement => settlement(e),
        SmartClass::Construction => construction(e),
        SmartClass::Road => road(e, measure),
        SmartClass::FunctionName => function_name(e),
        SmartClass::PlanNote => plan_note(e),
        SmartClass::None | SmartClass::Other => None,
    }
}

/// Base64, as `rtfData` is kept; empty for text that is not.
pub fn base64_decode(input: &str) -> Vec<u8> {
    let value = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for &c in input.as_bytes() {
        if c == b'=' {
            break;
        }
        match value(c) {
            Some(v) => {
                acc = (acc << 6) | v;
                bits += 6;
                if bits >= 8 {
                    bits -= 8;
                    out.push(((acc >> bits) & 0xFF) as u8);
                }
            }
            None if matches!(c, b'\r' | b'\n' | b' ') => {}
            None => return Vec::new(),
        }
    }
    out
}

/// Windows-1254 for the byte `\'hh` names: Latin-1 but for its six Turkish letters.
fn cp1254(byte: u8) -> char {
    crate::format::legacy_char(byte)
}

/// The plain text of an RTF document, paragraphs on their own lines: what a
/// plan note says, without its fonts. Group destinations (font and colour
/// tables, pictures, `\*` groups) are skipped; `\uN` is decoded, and `\'hh`
/// as Windows-1254, which is what a Turkish Netcad writes.
pub fn rtf_text(rtf: &[u8]) -> String {
    const SKIP: [&[u8]; 16] = [
        b"fonttbl",
        b"colortbl",
        b"stylesheet",
        b"info",
        b"pict",
        b"object",
        b"themedata",
        b"colorschememapping",
        b"latentstyles",
        b"datastore",
        b"xmlnstbl",
        b"listtable",
        b"listoverridetable",
        b"rsidtbl",
        b"generator",
        b"header",
    ];
    #[derive(Clone, Copy)]
    struct Group {
        skip: bool,
        uc: i64,
    }
    let mut stack = vec![Group { skip: false, uc: 1 }];
    let mut out = String::new();
    let mut fallback = 0i64; // characters still to drop after a `\u`
    let mut group_head = false;
    let mut i = 0usize;
    let skipping = |stack: &[Group]| stack.last().is_some_and(|g| g.skip);
    while i < rtf.len() {
        let c = rtf[i];
        match c {
            b'{' => {
                let top = stack
                    .last()
                    .copied()
                    .unwrap_or(Group { skip: false, uc: 1 });
                stack.push(top);
                group_head = true;
                i += 1;
                continue;
            }
            b'}' => {
                if stack.len() > 1 {
                    stack.pop();
                }
                group_head = false;
                i += 1;
                continue;
            }
            b'\r' | b'\n' => {
                i += 1;
                continue;
            }
            b'\\' => {}
            _ => {
                group_head = false;
                if fallback > 0 {
                    fallback -= 1;
                } else if !skipping(&stack) {
                    out.push(char::from(c));
                }
                i += 1;
                continue;
            }
        }
        // A control sequence.
        let Some(&next) = rtf.get(i + 1) else { break };
        if matches!(next, b'\\' | b'{' | b'}') {
            if fallback > 0 {
                fallback -= 1;
            } else if !skipping(&stack) {
                out.push(char::from(next));
            }
            i += 2;
            continue;
        }
        if next == b'*' {
            if let Some(top) = stack.last_mut() {
                top.skip = true; // an ignorable destination
            }
            i += 2;
            continue;
        }
        if next == b'\'' {
            let hex = rtf
                .get(i + 2..i + 4)
                .and_then(|h| std::str::from_utf8(h).ok());
            if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                if fallback > 0 {
                    fallback -= 1;
                } else if !skipping(&stack) {
                    out.push(cp1254(byte));
                }
            }
            i += 4;
            continue;
        }
        if next == b'~' {
            if !skipping(&stack) {
                out.push(' ');
            }
            i += 2;
            continue;
        }
        if !next.is_ascii_alphabetic() {
            i += 2; // `\-`, `\_` and the like: nothing to show
            continue;
        }
        let mut j = i + 1;
        while j < rtf.len() && rtf[j].is_ascii_alphabetic() {
            j += 1;
        }
        let word = &rtf[i + 1..j];
        let mut arg: Option<i64> = None;
        if j < rtf.len() && (rtf[j] == b'-' || rtf[j].is_ascii_digit()) {
            let mut k = j + usize::from(rtf[j] == b'-');
            while k < rtf.len() && rtf[k].is_ascii_digit() {
                k += 1;
            }
            if let Some(v) = std::str::from_utf8(&rtf[j..k])
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
            {
                arg = Some(v);
                j = k;
            }
        }
        if j < rtf.len() && rtf[j] == b' ' {
            j += 1; // the delimiter belongs to the word
        }
        i = j;
        if group_head
            && SKIP.contains(&word)
            && let Some(top) = stack.last_mut()
        {
            top.skip = true;
        }
        group_head = false;
        match (word, arg) {
            (b"par" | b"line", _) => {
                if !skipping(&stack) {
                    out.push('\n');
                }
            }
            (b"tab", _) => {
                if !skipping(&stack) {
                    out.push(' ');
                }
            }
            (b"uc", Some(n)) => {
                if let Some(top) = stack.last_mut() {
                    top.uc = n.clamp(0, 8);
                }
            }
            (b"u", Some(n)) => {
                let cp = if n < 0 { n + 65536 } else { n };
                if !skipping(&stack)
                    && let Some(ch) = u32::try_from(cp).ok().and_then(char::from_u32)
                {
                    out.push(ch);
                }
                fallback = stack.last().map_or(1, |g| g.uc);
            }
            _ => {}
        }
    }
    // Leading and trailing breaks and spaces say nothing.
    out.trim_matches(|c| c == '\n' || c == ' ').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::SmartProperty;

    fn object(class: SmartClass, props: &[(&str, &str, bool)]) -> Entity {
        Entity {
            smart: class,
            properties: props
                .iter()
                .map(|(n, v, null)| SmartProperty {
                    name: (*n).into(),
                    value: (*v).into(),
                    display: String::new(),
                    user: true,
                    null: *null,
                })
                .collect(),
            ..Entity::default()
        }
    }

    fn texts(s: &Symbol) -> Vec<&str> {
        s.strokes
            .iter()
            .filter_map(|st| match st {
                Stroke::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn width(t: &str, h: f64) -> f64 {
        t.chars().count() as f64 * 0.5 * h
    }

    #[test]
    fn a_settlement_reads_its_order_storeys_and_gardens() {
        let e = object(
            SmartClass::Settlement,
            &[
                ("nizam", "AYRIK", false),
                ("kat", "3", false),
                ("txtOn", "5", false),
                ("txtArka", "", false),
                ("txtYan", "3", false),
            ],
        );
        assert_eq!(
            texts(&planet_symbol(&e, &width).expect("drawn")),
            ["A", "3", "5", "3"]
        );
        let off = object(
            SmartClass::Settlement,
            &[
                ("nizam", "BİTİŞİK", false),
                ("kat", "4", false),
                ("txtOn", "0", true),
            ],
        );
        assert_eq!(
            texts(&planet_symbol(&off, &width).expect("drawn")),
            ["B", "4"]
        );
    }

    #[test]
    fn construction_shows_ratios_ranges_and_emsal() {
        let ratios = object(
            SmartClass::Construction,
            &[
                ("choiceType", "1", false),
                ("taks", "0.3", false),
                ("kaks", "0.9", false),
            ],
        );
        assert_eq!(
            texts(&planet_symbol(&ratios, &width).expect("drawn")),
            ["0.30", "0.90"]
        );
        let range = object(
            SmartClass::Construction,
            &[
                ("taks", "0.40", false),
                ("minTaks", "0.30", false),
                ("kaks", "0.90", false),
                ("minKaks", "0.65", false),
            ],
        );
        assert_eq!(
            texts(&planet_symbol(&range, &width).expect("drawn")),
            ["0.30-0.40", "0.65-0.90"]
        );
        let emsal = object(
            SmartClass::Construction,
            &[
                ("choiceType", "0", false),
                ("emsal", "1.5", false),
                ("yEncok", "12.5", false),
                ("hmax", "0", true),
            ],
        );
        assert_eq!(
            texts(&planet_symbol(&emsal, &width).expect("drawn")),
            ["E=1.50", "Yençok=12.50 m"]
        );
    }

    #[test]
    fn a_road_writes_its_decimals_raised() {
        let e = object(SmartClass::Road, &[("genislik", "7.5", false)]);
        assert_eq!(
            texts(&planet_symbol(&e, &width).expect("drawn")),
            ["7", "50"]
        );
        assert!(planet_symbol(&object(SmartClass::Road, &[]), &width).is_none());
    }

    #[test]
    fn rtf_is_read_as_its_text() {
        let rtf = br"{\rtf1\ansi\ansicpg1254{\fonttbl{\f0 Arial;}}{\colortbl;\red0\green0\blue0;}\f0\fs20 PLAN NOTLARI\par 1. Bu alanda \u304\'ddmar Kanunu uygulan\u305\'fdr.\par }";
        assert_eq!(
            rtf_text(rtf),
            "PLAN NOTLARI\n1. Bu alanda İmar Kanunu uygulanır."
        );
        assert_eq!(base64_decode("UExBTg=="), b"PLAN");
        assert!(base64_decode("?!").is_empty());
    }
}
