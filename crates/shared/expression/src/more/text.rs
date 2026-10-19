//! Text functions (docs/adr/0600 §3). Characters are Unicode scalars; Turkish letters are one each. `satırlara_böl`
//! and `ortak_parça` follow QGIS's `QgsStringUtils` (`wordWrap`, `longestCommonSubstring`); `başlık` QGIS's rule (each space-separated word longer than one letter) with Turkish casing.

use crate::compound::shown;
use crate::js::text::{MAX_STRING_UNITS, is_space, push_lower_tr, push_upper_tr, utf16_len};
use crate::library::Func;
use crate::scalar::{R, Scratch, V, as_text, equals, to_number};

use super::unaccent_table::BARE;

fn bare(c: char) -> char {
    match BARE.binary_search_by(|(k, _)| k.cmp(&c)) {
        Ok(i) => BARE[i].1,
        Err(_) => c,
    }
}

/// QGIS's word wrap: `length` positive is the most characters a line holds, negative the least before a break.
fn word_wrap(out: &mut String, text: &str, length: i64, delimiter: Option<&str>) {
    if text.is_empty() || length == 0 {
        out.push_str(text);
        return;
    }
    let want = length.unsigned_abs() as usize;
    let delim: Vec<char> = delimiter.unwrap_or("").chars().collect();
    let lines: Vec<&str> = text.split('\n').collect();
    for (li, line) in lines.iter().enumerate() {
        let cs: Vec<char> = line.chars().collect();
        let n = cs.len();
        let hit_at = |i: usize| -> bool {
            if delim.is_empty() {
                is_space(cs[i]) || cs[i] == '\u{200B}'
            } else {
                cs.len() >= i + delim.len() && cs[i..i + delim.len()] == delim[..]
            }
        };
        let dlen = if delim.is_empty() { 1 } else { delim.len() };
        if n <= want {
            out.extend(cs.iter());
        } else {
            let (mut cur, mut last_hit): (usize, Option<usize>) = (0, Some(0));
            while cur < n {
                let forward = |from: usize| (from..n).find(|&i| hit_at(i));
                let hit = if length > 0 {
                    let mut h = if cur + want >= n {
                        None
                    } else {
                        (0..=(cur + want).min(n - 1)).rev().find(|&i| hit_at(i))
                    };
                    if h == last_hit || h.is_none() {
                        h = if cur + want >= n {
                            None
                        } else {
                            forward(cur + want)
                        };
                    }
                    last_hit = h;
                    h
                } else if cur + want >= n {
                    None
                } else {
                    forward(cur + want)
                };
                match hit {
                    Some(h) if h >= cur => {
                        out.extend(cs[cur..h].iter());
                        out.push('\n');
                        cur = h + dlen;
                    }
                    _ => {
                        out.extend(cs[cur..].iter());
                        cur = n;
                    }
                }
            }
        }
        if li + 1 < lines.len() {
            out.push('\n');
        }
    }
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// QGIS's longest common substring (case counts); the first longest part met in the first text.
fn common_part(out: &mut String, a: &str, b: &str) {
    if a.is_empty() || b.is_empty() {
        return;
    }
    let (orig, s2): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if orig == s2 {
        out.push_str(a);
        return;
    }
    let s1 = &orig;
    let mut prev = vec![0usize; s2.len()];
    let mut cur = vec![0usize; s2.len()];
    let (mut best, mut end) = (0usize, 0usize);
    for (i, c1) in s1.iter().enumerate() {
        for (j, c2) in s2.iter().enumerate() {
            cur[j] = if c1 == c2 {
                if i == 0 || j == 0 { 1 } else { 1 + prev[j - 1] }
            } else {
                0
            };
            if cur[j] > best {
                best = cur[j];
                end = i;
            }
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    if best > 0 {
        out.extend(orig[end + 1 - best..=end].iter());
    }
}

/// A `%n` place at byte `i` of `b` (n 1–99): its number and its length.
fn place_at(b: &[u8], i: usize) -> Option<(u32, usize)> {
    if b[i] != b'%' || i + 1 >= b.len() || !b[i + 1].is_ascii_digit() {
        return None;
    }
    let one = u32::from(b[i + 1] - b'0');
    match b.get(i + 2) {
        Some(d) if d.is_ascii_digit() => Some((one * 10 + u32::from(d - b'0'), 3)),
        _ => (one > 0).then_some((one, 2)),
    }
}

/// `biçimle`: the k-th value takes the k-th lowest place number the pattern has, every place of it (QGIS's chained
/// `arg`; a value that itself holds `%n` is written as it is). One pass, without a copy of the pattern.
fn format(out: &mut String, pattern: &str, values: &[V], buf: &mut String) {
    let b = pattern.as_bytes();
    let mut numbers: [u32; 16] = [0; 16];
    let mut known = 0usize;
    let mut i = 0;
    while i < b.len() {
        if let Some((n, len)) = place_at(b, i) {
            if n > 0 && !numbers[..known].contains(&n) && known < numbers.len() {
                numbers[known] = n;
                known += 1;
            }
            i += len;
        } else {
            i += 1;
        }
    }
    numbers[..known].sort_unstable();
    let mut i = 0;
    let mut from = 0;
    while i < b.len() {
        if let Some((n, len)) = place_at(b, i) {
            let value = numbers[..known]
                .iter()
                .position(|&m| m == n)
                .and_then(|k| values.get(k));
            if let Some(&v) = value {
                out.push_str(&pattern[from..i]);
                out.push_str(shown(as_text(v, buf)));
                i += len;
                from = i;
                continue;
            }
            i += len;
        } else {
            i += 1;
        }
    }
    out.push_str(&pattern[from..]);
}

fn trim_with(t: &str, chars: Option<&str>, start: bool) -> usize {
    let keep = |c: char| match chars {
        Some(set) => !set.contains(c),
        None => !is_space(c),
    };
    if start {
        t.char_indices()
            .find(|(_, c)| keep(*c))
            .map_or(t.len(), |(i, _)| i)
    } else {
        t.char_indices()
            .rev()
            .find(|(_, c)| keep(*c))
            .map_or(0, |(i, c)| i + c.len_utf8())
    }
}

pub(super) fn call<'a>(f: Func, args: &[V<'a>], out: &mut String, s: &mut Scratch) -> R<'a> {
    let arg = |i: usize| super::arg(args, i);
    let mark = out.len();
    match f {
        Func::NullIf => {
            return if equals(arg(0), arg(1), s) {
                R::V(V::Null)
            } else {
                R::V(arg(0))
            };
        }
        Func::ConcatWs => {
            let sep = shown(as_text(arg(0), &mut s.a)).to_owned();
            let mut first = true;
            for &v in &args[1.min(args.len())..] {
                if v == V::Null {
                    continue;
                }
                if !first {
                    out.push_str(&sep);
                }
                first = false;
                out.push_str(shown(as_text(v, &mut s.b)));
            }
            return super::made(out, mark);
        }
        Func::Char => {
            return match to_number(arg(0))
                .and_then(|x| char::from_u32(x as u32).filter(|_| x >= 0.0))
            {
                Some(c) => {
                    out.push(c);
                    R::Made
                }
                None => R::V(V::Null),
            };
        }
        _ => {}
    }
    if arg(0) == V::Null {
        return R::V(V::Null);
    }
    let t = shown(as_text(arg(0), &mut s.a));
    match f {
        Func::Ltrim | Func::Rtrim => {
            let set = if args.len() > 1 {
                Some(shown(as_text(arg(1), &mut s.b)).to_owned())
            } else {
                None
            };
            if f == Func::Ltrim {
                out.push_str(&t[trim_with(t, set.as_deref(), true)..]);
            } else {
                out.push_str(&t[..trim_with(t, set.as_deref(), false)]);
            }
        }
        Func::Repeat => {
            let Some(n) = to_number(arg(1)) else {
                return R::V(V::Null);
            };
            // As QGIS: a count that is not above 0 gives an empty value.
            if n < 1.0 {
                return R::V(V::Null);
            }
            let n = n.trunc();
            if n * utf16_len(t) as f64 > MAX_STRING_UNITS as f64 {
                return R::Thrown;
            }
            for _ in 0..n as usize {
                out.push_str(t);
            }
        }
        Func::Reverse => out.extend(t.chars().rev()),
        Func::Title => {
            for (k, word) in t.split(' ').enumerate() {
                if k > 0 {
                    out.push(' ');
                }
                let mut cs = word.chars();
                match (cs.next(), word.chars().count() > 1) {
                    (Some(c), true) => {
                        push_upper_tr(out, c.encode_utf8(&mut [0; 4]));
                        push_lower_tr(out, cs.as_str());
                    }
                    _ => out.push_str(word),
                }
            }
        }
        Func::SubstrCount => {
            let part = shown(as_text(arg(1), &mut s.b)).to_owned();
            let overlap = args.len() > 2 && crate::scalar::truthy(arg(2));
            if part.is_empty() {
                return R::V(V::Num(0.0));
            }
            let mut count = 0usize;
            let mut from = 0usize;
            while let Some(i) = t[from..].find(&part) {
                count += 1;
                from += i + if overlap {
                    t[from + i..].chars().next().map_or(1, char::len_utf8)
                } else {
                    part.len()
                };
            }
            return R::V(V::Num(count as f64));
        }
        Func::Format => format(out, t, &args[1..], &mut s.c),
        Func::Ascii => {
            return R::V(
                t.chars()
                    .next()
                    .map_or(V::Null, |c| V::Num(f64::from(u32::from(c)))),
            );
        }
        Func::Unaccent => out.extend(t.chars().map(bare)),
        Func::WordWrap => {
            let Some(n) = to_number(arg(1)) else {
                return R::V(V::Null);
            };
            let delim = if args.len() > 2 {
                Some(shown(as_text(arg(2), &mut s.b)).to_owned())
            } else {
                None
            };
            let text = t.to_owned();
            word_wrap(
                out,
                &text,
                n.trunc() as i64,
                delim.as_deref().filter(|d| !d.is_empty()),
            );
        }
        Func::Levenshtein | Func::Hamming => {
            if arg(1) == V::Null {
                return R::V(V::Null);
            }
            let a: Vec<char> = t.chars().collect();
            let b: Vec<char> = shown(as_text(arg(1), &mut s.b)).chars().collect();
            if f == Func::Hamming {
                if a.len() != b.len() {
                    return R::V(V::Null);
                }
                return R::V(V::Num(
                    a.iter().zip(&b).filter(|(x, y)| x != y).count() as f64
                ));
            }
            return R::V(V::Num(levenshtein(&a, &b) as f64));
        }
        Func::CommonPart => {
            if arg(1) == V::Null {
                return R::V(V::Null);
            }
            let a = t.to_owned();
            let b = shown(as_text(arg(1), &mut s.b)).to_owned();
            common_part(out, &a, &b);
        }
        _ => return R::V(V::Null),
    }
    super::made(out, mark)
}
