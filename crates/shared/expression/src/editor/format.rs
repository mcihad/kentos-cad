//! Biçimle (docs/adr/0603): an expression written on lines the eye can follow. A part that fits on a line stays on it;
//! a longer `durum` puts each branch on its own line, a longer call each argument, a longer chain of `ve` or `veya`
//! each operand. What is written reads as the same expression (the flow's tree, `flow::read`). An expression with
//! comments is left as it is: the tree has no place for them.

use super::flow::{T, read};
use super::lex::{Class, tokens};
use crate::js::text::utf16_len;
use crate::parser::BinOp;

/// The width a line keeps to, in characters.
const WIDTH: usize = 60;
/// A level of indentation.
const STEP: usize = 2;

/// The expression on lines; why not, in Turkish, when it cannot be.
pub fn format(src: &str) -> Result<String, String> {
    if tokens(src).iter().any(|t| t.class == Class::Comment) {
        return Err("Yorumlu ifade biçimlenmez: yorumlar kaybolurdu.".into());
    }
    let t = read(src).map_err(|e| e.text())?;
    Ok(lines(&t, 0))
}

fn pad(n: usize) -> String {
    " ".repeat(n)
}

/// `t` written from column `at`, its later lines indented from the line's own indentation `at`.
fn lines(t: &T, at: usize) -> String {
    let one = t.text();
    if at + utf16_len(&one) <= WIDTH && !one.contains('\n') {
        return one;
    }
    match t {
        T::Case(whens, otherwise) => {
            let mut s = String::from("durum");
            for (c, v) in whens {
                s.push('\n');
                s.push_str(&pad(at + STEP));
                s.push_str("eğer ");
                s.push_str(&lines(c, at + STEP));
                s.push('\n');
                s.push_str(&pad(at + 2 * STEP));
                s.push_str("ise ");
                s.push_str(&lines(v, at + 2 * STEP));
            }
            if let Some(e) = otherwise {
                s.push('\n');
                s.push_str(&pad(at + STEP));
                s.push_str("yoksa ");
                s.push_str(&lines(e, at + STEP));
            }
            s.push('\n');
            s.push_str(&pad(at));
            s.push_str("son");
            s
        }
        T::Call(f, args) if !args.is_empty() => {
            let name = super::flow::T::Call(*f, Vec::new()).text();
            let name = name.trim_end_matches("()");
            let mut s = format!("{name}(");
            for (i, a) in args.iter().enumerate() {
                s.push('\n');
                s.push_str(&pad(at + STEP));
                s.push_str(&lines(a, at + STEP));
                if i + 1 < args.len() {
                    s.push(',');
                }
            }
            s.push('\n');
            s.push_str(&pad(at));
            s.push(')');
            s
        }
        T::Bin(op @ (BinOp::And | BinOp::Or), ..) => {
            let word = if *op == BinOp::And { "ve" } else { "veya" };
            // The chain written left to right: a ve b ve c is (a ve b) ve c.
            let mut parts: Vec<&T> = Vec::new();
            let mut left = t;
            while let T::Bin(o, a, b) = left
                && o == op
            {
                parts.push(b);
                left = a;
            }
            parts.push(left);
            parts.reverse();
            let level = t.level();
            let mut s = String::new();
            for (i, p) in parts.iter().enumerate() {
                if i > 0 {
                    s.push('\n');
                    s.push_str(&pad(at));
                    s.push_str(word);
                    s.push(' ');
                }
                // The first is the chain's left operand; the rest its right ones, in parentheses when as loose.
                let least = if i == 0 { level } else { level + 1 };
                let inner = at + if i == 0 { 0 } else { word.len() + 1 };
                if p.level() < least {
                    s.push('(');
                    s.push_str(&lines(p, inner + 1));
                    s.push(')');
                } else {
                    s.push_str(&lines(p, inner));
                }
            }
            s
        }
        _ => one,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &str, b: &str) {
        assert_eq!(
            read(a).ok().map(|t| t.text()),
            read(b).ok().map(|t| t.text()),
            "{b}"
        );
    }

    #[test]
    fn a_long_expression_goes_on_lines_and_reads_the_same() {
        assert_eq!(format("1 + 2").as_deref(), Ok("1 + 2"));
        let src = "durum eğer Nitelik = 'Arsa' ve $alan > 500 ve Ada içinde (101, 102, 103) ise yuvarla($alan / 1000, 2) eğer Nitelik = 'Tarla' ise 1 yoksa 0 son";
        let out = format(src).expect("formats");
        assert!(out.lines().count() > 3, "{out}");
        assert!(out.lines().all(|l| l.chars().count() <= WIDTH), "{out}");
        same(src, &out);
        let chain = "(a veya b) ve uzun_bir_alan_adi = 'birinci değer' ve başka_bir_alan_adı = 'ikinci uzun değer' ve c";
        let out = format(chain).expect("formats");
        same(chain, &out);
        let call = "dizi_metin(dizi_her(metin_dizi(Etiketler), büyük(@öğe) || ' / ' || küçük(@öğe)), ', ')";
        let out = format(call).expect("formats");
        same(call, &out);
        assert!(format("1 // yorum").is_err());
        assert!(format("yuvarla(1").is_err());
    }
}
