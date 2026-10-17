//! PAT files (docs/adr/0331 §1), as AutoCAD's documentation writes them: a header `*NAME[,description]`, then one
//! line a family: `angle, x-origin, y-origin, delta-x, delta-y [, dash-1, dash-2, …]` (a dash drawn when positive, a
//! gap when negative, a dot when 0). A `;` starts a comment to the line's end. The file says no unit: the reader is
//! told it and keeps the numbers as written. What does not read is said with its line; a pattern with a family that
//! does not read is left out whole. Written back in the shortest form that reads back. The shared cases are
//! `fixtures/hatch/v1/pat.json` (`scripts/fixtures/hatch_pattern_file_cases.py`).

use kentos_contracts::{
    HatchPatternDef, HatchPatternUnit, MAX_PATTERN_DASHES, MAX_PATTERN_LINES, MAX_PATTERN_NAME,
    MAX_PATTERN_SIZE, PatternLine, same_hatch_pattern_name,
};

/// A line that does not read, and why (Turkish).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatProblem {
    pub line: u32,
    pub message: String,
}

/// A file's patterns in its order, and what of it does not read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatRead {
    pub patterns: Vec<HatchPatternDef>,
    pub problems: Vec<PatProblem>,
}

/// A PAT number: `.5`, `-0.25`, `1e-2`.
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

/// A family's numbers, or why they do not read.
fn family(parts: &[&str]) -> Result<PatternLine, String> {
    if parts.len() < 5 {
        return Err(format!(
            "bir aile en az beş sayıdır (açı, x, y, dx, dy), {} var",
            parts.len()
        ));
    }
    let mut values = Vec::with_capacity(parts.len());
    for p in parts {
        let Some(v) = number(p) else {
            return Err(format!("“{}” bir sayı değil", p.trim()));
        };
        if !(v.abs() <= MAX_PATTERN_SIZE) {
            return Err(format!(
                "“{}” çok büyük; en çok {} olur",
                p.trim(),
                MAX_PATTERN_SIZE as i64
            ));
        }
        values.push(v);
    }
    if values.len() - 5 > MAX_PATTERN_DASHES {
        return Err(format!(
            "bir ailenin en çok {MAX_PATTERN_DASHES} kesiği olur, {} var",
            values.len() - 5
        ));
    }
    if values[4] == 0.0 {
        return Err("çizgiler arası (dy) 0 olamaz".into());
    }
    if values.len() > 5 && values[5..].iter().all(|v| *v == 0.0) {
        return Err("kesiklerin hepsi 0 olamaz; en az birinin uzunluğu olmalı".into());
    }
    Ok(PatternLine {
        angle: values[0],
        origin: [values[1], values[2]],
        offset: [values[3], values[4]],
        dashes: values[5..].to_vec(),
    })
}

struct Current {
    name: String,
    description: String,
    line: u32,
    lines: Vec<PatternLine>,
    bad: bool,
}

/// The patterns of a PAT file's text, its numbers in `unit`, each from `source`.
pub fn read_pat(text: &str, unit: HatchPatternUnit, source: &str) -> PatRead {
    let mut out = PatRead::default();
    let mut current: Option<Current> = None;
    let finish = |c: Option<Current>, out: &mut PatRead| {
        let Some(c) = c else { return };
        if c.bad {
            return;
        }
        if c.lines.is_empty() {
            out.problems.push(PatProblem {
                line: c.line,
                message: format!("“{}” deseninin ailesi yok", c.name),
            });
            return;
        }
        if c.lines.len() > MAX_PATTERN_LINES {
            out.problems.push(PatProblem {
                line: c.line,
                message: format!(
                    "“{}”: bir desen en çok {MAX_PATTERN_LINES} ailedir, {} var",
                    c.name,
                    c.lines.len()
                ),
            });
            return;
        }
        if out
            .patterns
            .iter()
            .any(|p| same_hatch_pattern_name(&p.name, &c.name))
        {
            out.problems.push(PatProblem {
                line: c.line,
                message: format!("“{}” dosyada iki kez var; ilki alındı", c.name),
            });
            return;
        }
        out.patterns.push(HatchPatternDef {
            name: c.name,
            description: c.description,
            unit,
            lines: c.lines,
            source: Some(source.to_owned()),
        });
    };
    for (i, raw) in text.lines().enumerate() {
        let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(head) = line.strip_prefix('*') {
            finish(current.take(), &mut out);
            let (name, description) = head.split_once(',').unwrap_or((head, ""));
            let name = name.trim();
            let bad = |name: String| Current {
                name,
                description: String::new(),
                line: n,
                lines: Vec::new(),
                bad: true,
            };
            if name.is_empty() {
                out.problems.push(PatProblem {
                    line: n,
                    message: "desenin adı boş".into(),
                });
                current = Some(bad(String::new()));
                continue;
            }
            if name.chars().count() > MAX_PATTERN_NAME {
                let short: String = name.chars().take(16).collect();
                out.problems.push(PatProblem {
                    line: n,
                    message: format!("“{short}…”: desenin adı en çok {MAX_PATTERN_NAME} harf olur"),
                });
                current = Some(bad(name.to_owned()));
                continue;
            }
            current = Some(Current {
                name: name.to_owned(),
                description: description.trim().to_owned(),
                line: n,
                lines: Vec::new(),
                bad: false,
            });
            continue;
        }
        let Some(c) = current.as_mut() else {
            out.problems.push(PatProblem {
                line: n,
                message: "aile bir *AD satırının altında olmalı".into(),
            });
            continue;
        };
        if c.bad {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        match family(&parts) {
            Ok(f) => c.lines.push(f),
            Err(why) => {
                out.problems.push(PatProblem {
                    line: n,
                    message: format!("“{}”: {why}; desen alınmadı", c.name),
                });
                c.bad = true;
            }
        }
    }
    finish(current.take(), &mut out);
    out
}

/// The shortest PAT form that reads back: no trailing zeros, `.5` not `0.5`, no exponent.
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

/// The patterns as a PAT file.
pub fn write_pat(patterns: &[HatchPatternDef]) -> String {
    let mut out = String::new();
    for p in patterns {
        out.push('*');
        out.push_str(&p.name);
        if !p.description.is_empty() {
            out.push(',');
            out.push_str(&p.description);
        }
        out.push('\n');
        for l in &p.lines {
            let values = [l.angle, l.origin[0], l.origin[1], l.offset[0], l.offset[1]]
                .into_iter()
                .chain(l.dashes.iter().copied());
            out.push_str(&values.map(short).collect::<Vec<_>>().join(","));
            out.push('\n');
        }
    }
    out
}
