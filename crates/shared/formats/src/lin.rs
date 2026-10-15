//! LIN files (docs/adr/0313 §2), as AutoCAD's documentation writes them: a
//! header `*NAME[,description]`, then `A,` and at most twelve descriptors, a
//! dash (a positive length), a gap (negative), a dot (0) or a complex element
//! in brackets (`["text",STYLE,S=…,R=…|A=…|U=…,X=…,Y=…]`, `[SHAPE,file.shx,…]`)
//! that takes no length of its own. Comments start with `;;`. What does not
//! read is said with its line; the rest is read. Written back in the
//! shortest form that reads back. The shared cases are
//! `fixtures/linetypes/v1/cases.json` (`scripts/fixtures/line_type_cases.py`).

use kentos_contracts::{
    LineTypeDef, MAX_PATTERN_ELEMENTS, PatternElement, PatternPlace, PatternShape, PatternText,
    PatternUnit, same_line_type_name,
};

/// A line that does not read, and why (Turkish).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinProblem {
    pub line: u32,
    pub message: String,
}

/// A file's line types in its order, and what of it does not read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinRead {
    pub types: Vec<LineTypeDef>,
    pub problems: Vec<LinProblem>,
}

/// A LIN number: `.5`, `-0.25`, `1e-2`.
fn number(text: &str) -> Option<f64> {
    let t = text.trim();
    let digits = t.trim_start_matches(['+', '-']);
    let (mantissa, exponent) = match digits.find(['e', 'E']) {
        Some(i) => (&digits[..i], Some(&digits[i + 1..])),
        None => (digits, None),
    };
    let (whole, frac) = match mantissa.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (mantissa, None),
    };
    let all_digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    let ok = all_digits(whole)
        && frac.is_none_or(all_digits)
        && !(whole.is_empty() && frac.is_none_or(str::is_empty))
        && exponent.is_none_or(|e| {
            let e = e.trim_start_matches(['+', '-']);
            !e.is_empty() && all_digits(e)
        })
        && t.len() - digits.len() <= 1;
    if !ok {
        return None;
    }
    t.parse::<f64>().ok()
}

/// A rotation in degrees: `0`, `90d`, `1.5708r`, `100g`.
fn angle(text: &str) -> Option<f64> {
    let t = text.trim().to_lowercase();
    let (body, k) = if let Some(b) = t.strip_suffix('d') {
        (b.to_owned(), 1.0)
    } else if let Some(b) = t.strip_suffix('r') {
        (b.to_owned(), 180.0 / std::f64::consts::PI)
    } else if let Some(b) = t.strip_suffix('g') {
        (b.to_owned(), 0.9)
    } else {
        (t, 1.0)
    };
    number(&body).map(|v| v * k)
}

/// The descriptors: split at commas outside brackets and quotes; whether they all close.
fn split_top(body: &str) -> (Vec<String>, bool) {
    let (mut out, mut cur, mut depth, mut quoted) = (Vec::new(), String::new(), 0i32, false);
    for c in body.chars() {
        match c {
            '"' => quoted = !quoted,
            '[' if !quoted => depth += 1,
            ']' if !quoted => depth -= 1,
            ',' if depth == 0 && !quoted => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    (out, depth == 0 && !quoted)
}

/// A bracketed element, or why it does not read.
fn complex_element(text: &str) -> Result<PatternElement, String> {
    let t = text.trim();
    let inner = &t[1..t.len().saturating_sub(1).max(1)];
    let (parts, _) = split_top(inner);
    let parts: Vec<&str> = parts.iter().map(|p| p.trim()).collect();
    if parts.len() < 2 {
        return Err(
            "karmaşık öğe en az iki parça ister (yazı ve stil ya da şekil ve dosya)".into(),
        );
    }
    let mut place = PatternPlace {
        scale: 1.0,
        rotation: 0.0,
        absolute: false,
        upright: false,
        x: 0.0,
        y: 0.0,
    };
    for r in &parts[2..] {
        let Some((k, v)) = r.split_once('=') else {
            return Err(format!("“{r}” bir dönüşüm değil (S=, R=, A=, U=, X=, Y=)"));
        };
        let (k, v) = (k.trim().to_uppercase(), v.trim());
        match k.as_str() {
            "S" => match number(v) {
                Some(s) if s > 0.0 => place.scale = s,
                _ => return Err(format!("ölçek “{v}” 0'dan büyük bir sayı değil")),
            },
            "R" | "A" | "U" => {
                let Some(a) = angle(v) else {
                    return Err(format!("açı “{v}” okunmuyor"));
                };
                place.rotation = a;
                place.absolute = k == "A";
                if k == "U" {
                    place.upright = true;
                }
            }
            "X" | "Y" => {
                let Some(s) = number(v) else {
                    return Err(format!("kayma “{v}” bir sayı değil"));
                };
                if k == "X" {
                    place.x = s;
                } else {
                    place.y = s;
                }
            }
            other => {
                return Err(format!(
                    "“{other}” bilinen bir dönüşüm değil (S, R, A, U, X, Y)"
                ));
            }
        }
    }
    let (head, second) = (parts[0], parts[1]);
    if let Some(quoted) = head.strip_prefix('"') {
        let Some(text) = quoted.strip_suffix('"') else {
            return Err("yazı tırnakla kapanmıyor".into());
        };
        return Ok(PatternElement {
            length: 0.0,
            text: Some(PatternText {
                text: text.to_owned(),
                style: second.to_owned(),
                place,
            }),
            shape: None,
        });
    }
    Ok(PatternElement {
        length: 0.0,
        text: None,
        shape: Some(PatternShape {
            name: head.to_owned(),
            file: second.to_owned(),
            place,
        }),
    })
}

/// Reads a LIN file; `source` is its name, kept on each type.
pub fn read_lin(text: &str, source: &str) -> LinRead {
    let mut out = LinRead::default();
    let mut header: Option<(String, String, u32)> = None;
    let say = |out: &mut LinRead, line: u32, message: String| {
        out.problems.push(LinProblem { line, message });
    };
    for (i, raw) in text.lines().enumerate() {
        let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let line = raw.trim();
        if line.is_empty() || line.starts_with(";;") {
            continue;
        }
        if let Some(head) = line.strip_prefix('*') {
            if let Some((name, _, at)) = header.take() {
                say(
                    &mut out,
                    at,
                    format!("“{name}” çizgi tipinin deseni yok (A, satırı)"),
                );
            }
            let (name, desc) = head.split_once(',').unwrap_or((head, ""));
            let name = name.trim();
            if name.is_empty() {
                say(&mut out, n, "çizgi tipinin adı boş".into());
                continue;
            }
            header = Some((name.to_owned(), desc.trim().to_owned(), n));
            continue;
        }
        let Some((name, desc, _)) = header.take() else {
            say(&mut out, n, "desen bir *AD satırının altında olmalı".into());
            continue;
        };
        let (parts, closed) = split_top(line);
        if !closed {
            say(
                &mut out,
                n,
                format!("“{name}”: köşeli parantez ya da tırnak kapanmıyor"),
            );
            continue;
        }
        if !parts[0].trim().eq_ignore_ascii_case("A") {
            say(
                &mut out,
                n,
                format!("“{name}”: desen A ile başlamalı (hizalama)"),
            );
            continue;
        }
        let mut elements = Vec::new();
        let mut why = None;
        for p in &parts[1..] {
            let p = p.trim();
            if p.starts_with('[') {
                match complex_element(p) {
                    Ok(e) => elements.push(e),
                    Err(w) => {
                        why = Some(w);
                        break;
                    }
                }
            } else {
                match number(p) {
                    Some(v) => elements.push(PatternElement {
                        length: v,
                        text: None,
                        shape: None,
                    }),
                    None => {
                        why = Some(format!("“{p}” bir uzunluk değil"));
                        break;
                    }
                }
            }
        }
        if let Some(why) = why {
            say(&mut out, n, format!("“{name}”: {why}"));
            continue;
        }
        if elements.len() > MAX_PATTERN_ELEMENTS {
            say(
                &mut out,
                n,
                format!(
                    "“{name}”: desen en çok {MAX_PATTERN_ELEMENTS} öğe olur, {} var",
                    elements.len()
                ),
            );
            continue;
        }
        if elements
            .iter()
            .find(|e| !e.is_complex())
            .is_none_or(|e| e.length < 0.0)
        {
            say(
                &mut out,
                n,
                format!("“{name}”: desen bir çizgi ya da noktayla başlamalı"),
            );
            continue;
        }
        if out
            .types
            .iter()
            .any(|t| same_line_type_name(&t.name, &name))
        {
            say(
                &mut out,
                n,
                format!("“{name}” dosyada iki kez var; ilki alındı"),
            );
            continue;
        }
        out.types.push(LineTypeDef {
            name,
            description: desc,
            elements,
            unit: PatternUnit::Drawing,
            source: Some(source.to_owned()),
        });
    }
    if let Some((name, _, at)) = header {
        say(
            &mut out,
            at,
            format!("“{name}” çizgi tipinin deseni yok (A, satırı)"),
        );
    }
    out
}

/// The shortest form that reads back: no trailing zeros, `.5` for `0.5`.
fn short(x: f64) -> String {
    if x == x.trunc() && x.abs() < 1e15 {
        return format!("{}", x as i64);
    }
    let t = format!("{x}");
    if let Some(rest) = t.strip_prefix("0.") {
        format!(".{rest}")
    } else if let Some(rest) = t.strip_prefix("-0.") {
        format!("-.{rest}")
    } else {
        t
    }
}

fn element_text(e: &PatternElement) -> String {
    let (head, place) = match (&e.text, &e.shape) {
        (Some(t), _) => (format!("\"{}\",{}", t.text, t.style), &t.place),
        (None, Some(s)) => (format!("{},{}", s.name, s.file), &s.place),
        (None, None) => return short(e.length),
    };
    let mut parts = vec![head];
    if place.scale != 1.0 {
        parts.push(format!("S={}", short(place.scale)));
    }
    if place.upright {
        parts.push(format!("U={}", short(place.rotation)));
    } else if place.absolute {
        parts.push(format!("A={}", short(place.rotation)));
    } else if place.rotation != 0.0 {
        parts.push(format!("R={}", short(place.rotation)));
    }
    if place.x != 0.0 {
        parts.push(format!("X={}", short(place.x)));
    }
    if place.y != 0.0 {
        parts.push(format!("Y={}", short(place.y)));
    }
    format!("[{}]", parts.join(","))
}

/// The types as a LIN file.
pub fn write_lin(types: &[LineTypeDef]) -> String {
    let mut out = String::new();
    for t in types {
        out.push('*');
        out.push_str(&t.name);
        if !t.description.is_empty() {
            out.push(',');
            out.push_str(&t.description);
        }
        out.push('\n');
        out.push_str("A,");
        out.push_str(
            &t.elements
                .iter()
                .map(element_text)
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}
