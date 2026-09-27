//! Tokens of an expression (the TypeScript's `tokenize`). The source is
//! read in UTF-16 code units, as JavaScript reads it, so the positions in
//! error messages are the ones the dialog shows (1-based). Names take
//! letters, digits and `_` (letters: Unicode Alphabetic; JavaScript's `\p{L}`
//! leaves out a few combining signs of other scripts and the letter-like
//! numerals, which only matters outside the Latin, Greek and Cyrillic scripts).

use super::CompileError;
use crate::js::number;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(f64),
    Str(String),
    Field(String),
    Var(String),
    Word(String),
    Op(&'static str),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub t: Tok,
    /// 1-based position of its first code unit.
    pub at: usize,
}

impl Token {
    /// How an error message names the token.
    pub fn describe(&self) -> String {
        match &self.t {
            Tok::End => "ifadenin sonu".into(),
            Tok::Num(v) => format!("“{}”", number::to_string(*v)),
            Tok::Str(v) | Tok::Field(v) | Tok::Word(v) => format!("“{v}”"),
            Tok::Op(v) => format!("“{v}”"),
            Tok::Var(v) => format!("“${v}”"),
        }
    }
}

const OPS: [&str; 17] = [
    "<=", ">=", "!=", "<>", "==", "||", "=", "<", ">", "+", "-", "*", "/", "%", "(", ")", ",",
];

fn err(message: impl Into<String>, at: usize) -> CompileError {
    CompileError {
        message: message.into(),
        at,
    }
}

/// The code point at `i` and its length in units (a lone surrogate reads as U+FFFD).
fn code_point(u: &[u16], i: usize) -> Option<(char, usize)> {
    let a = *u.get(i)?;
    if (0xd800..0xdc00).contains(&a)
        && let Some(&b) = u.get(i + 1)
        && (0xdc00..0xe000).contains(&b)
    {
        let c = 0x10000 + ((u32::from(a) - 0xd800) << 10) + (u32::from(b) - 0xdc00);
        return Some((char::from_u32(c).unwrap_or('\u{fffd}'), 2));
    }
    Some((char::from_u32(u32::from(a)).unwrap_or('\u{fffd}'), 1))
}

/// `[\p{L}_][\p{L}\p{N}_]*` at `i`: its length in units (0: no name there).
fn word(u: &[u16], i: usize) -> usize {
    let mut j = i;
    while let Some((c, n)) = code_point(u, j) {
        let ok = c == '_' || c.is_alphabetic() || (j > i && c.is_numeric());
        if !ok {
            break;
        }
        j += n;
    }
    j - i
}

fn is_digit(u: &[u16], i: usize) -> bool {
    u.get(i)
        .is_some_and(|&c| (u16::from(b'0')..=u16::from(b'9')).contains(&c))
}

fn unit_is(u: &[u16], i: usize, c: u8) -> bool {
    u.get(i) == Some(&u16::from(c))
}

/// One code unit as the error message shows it.
fn unit_text(c: u16) -> String {
    String::from_utf16_lossy(&[c])
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, CompileError> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < u.len() {
        let c = u[i];
        if char::from_u32(u32::from(c)).is_some_and(crate::js::text::is_space) {
            i += 1;
            continue;
        }
        let at = i + 1;
        if is_digit(&u, i) || (unit_is(&u, i, b'.') && is_digit(&u, i + 1)) {
            // /\d*\.?\d+(?:[eE][+-]?\d+)?/: digits, a fraction only with digits after the
            // point, an exponent only with digits.
            let mut j = i;
            while is_digit(&u, j) {
                j += 1;
            }
            if unit_is(&u, j, b'.') && is_digit(&u, j + 1) {
                j += 1;
                while is_digit(&u, j) {
                    j += 1;
                }
            }
            if unit_is(&u, j, b'e') || unit_is(&u, j, b'E') {
                let mut k = j + 1;
                if unit_is(&u, k, b'+') || unit_is(&u, k, b'-') {
                    k += 1;
                }
                if is_digit(&u, k) {
                    while is_digit(&u, k) {
                        k += 1;
                    }
                    j = k;
                }
            }
            let s = String::from_utf16_lossy(&u[i..j]);
            out.push(Token {
                t: Tok::Num(s.parse().unwrap_or(f64::NAN)),
                at,
            });
            i = j;
            continue;
        }
        if c == u16::from(b'\'') || c == u16::from(b'"') {
            let mut s: Vec<u16> = Vec::new();
            let mut j = i + 1;
            loop {
                if j >= u.len() {
                    return Err(err("Tırnak kapanmamış.", at));
                }
                if u[j] == c {
                    if u.get(j + 1) == Some(&c) {
                        s.push(c);
                        j += 2;
                        continue;
                    }
                    break;
                }
                s.push(u[j]);
                j += 1;
            }
            out.push(Token {
                t: Tok::Str(String::from_utf16_lossy(&s)),
                at,
            });
            i = j + 1;
            continue;
        }
        if c == u16::from(b'[') {
            let Some(k) = u[i..].iter().position(|&x| x == u16::from(b']')) else {
                return Err(err("“]” bekleniyordu: alan adı kapanmamış.", at));
            };
            let j = i + k;
            let name = crate::js::text::trim(&String::from_utf16_lossy(&u[i + 1..j])).to_string();
            if name.is_empty() {
                return Err(err("Köşeli parantez içinde alan adı yok.", at));
            }
            out.push(Token {
                t: Tok::Field(name),
                at,
            });
            i = j + 1;
            continue;
        }
        if c == u16::from(b'$') {
            let n = word(&u, i + 1);
            if n == 0 {
                return Err(err("“$” işaretinden sonra değişken adı bekleniyordu.", at));
            }
            out.push(Token {
                t: Tok::Var(String::from_utf16_lossy(&u[i + 1..i + 1 + n])),
                at,
            });
            i += 1 + n;
            continue;
        }
        let n = word(&u, i);
        if n > 0 {
            out.push(Token {
                t: Tok::Word(String::from_utf16_lossy(&u[i..i + n])),
                at,
            });
            i += n;
            continue;
        }
        let Some(op) = OPS.iter().find(|o| {
            let o: Vec<u16> = o.encode_utf16().collect();
            u[i..].starts_with(&o)
        }) else {
            return Err(err(
                format!("Anlaşılmayan karakter: “{}”.", unit_text(c)),
                at,
            ));
        };
        out.push(Token { t: Tok::Op(op), at });
        i += op.len();
    }
    out.push(Token {
        t: Tok::End,
        at: u.len() + 1,
    });
    Ok(out)
}
