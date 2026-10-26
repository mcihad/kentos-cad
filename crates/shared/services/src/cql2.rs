//! A filter in CQL2 text read into CQL2 JSON (OGC 21-065, Basic CQL2 and the comparison operators; docs/adr/0655 §4), for
//! a service that declares CQL2's JSON encoding and not its text one. Comparisons, `LIKE`, `IN`, `BETWEEN`, `IS [NOT]
//! NULL`, `AND`, `OR`, `NOT`, parentheses; properties plain or in double quotes, texts in single quotes, numbers, `TRUE`
//! and `FALSE`, `DATE('…')` and `TIMESTAMP('…')`. A spatial or temporal function is refused with its name: KentOS does
//! not send what it cannot read.

use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Word(String),
    Quoted(String),
    Text(String),
    Num(f64),
    Op(&'static str),
    Open,
    Close,
    Comma,
}

fn tokens(text: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '(' || ch == ')' || ch == ',' {
            out.push(match ch {
                '(' => Tok::Open,
                ')' => Tok::Close,
                _ => Tok::Comma,
            });
            i += 1;
        } else if ch == '\'' || ch == '"' {
            let mut s = String::new();
            i += 1;
            loop {
                match c.get(i) {
                    None => return Err("kapanmamış tırnak".into()),
                    Some(&q) if q == ch && c.get(i + 1) == Some(&ch) => {
                        s.push(ch);
                        i += 2;
                    }
                    Some(&q) if q == ch => {
                        i += 1;
                        break;
                    }
                    Some(&q) => {
                        s.push(q);
                        i += 1;
                    }
                }
            }
            out.push(if ch == '\'' {
                Tok::Text(s)
            } else {
                Tok::Quoted(s)
            });
        } else if "<>=".contains(ch) {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            let op = match two.as_str() {
                "<>" => "<>",
                "<=" => "<=",
                ">=" => ">=",
                _ => match ch {
                    '<' => "<",
                    '>' => ">",
                    _ => "=",
                },
            };
            i += op.len();
            out.push(Tok::Op(op));
        } else if ch.is_ascii_digit()
            || ((ch == '-' || ch == '.') && c.get(i + 1).is_some_and(|d| d.is_ascii_digit()))
        {
            let start = i;
            i += 1;
            while i < c.len()
                && (c[i].is_ascii_digit()
                    || matches!(c[i], '.' | 'e' | 'E')
                    || ((c[i] == '-' || c[i] == '+') && matches!(c[i - 1], 'e' | 'E')))
            {
                i += 1;
            }
            let s: String = c[start..i].iter().collect();
            out.push(Tok::Num(
                s.parse().map_err(|_| format!("“{s}” bir sayı değil"))?,
            ));
        } else if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < c.len()
                && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '.' || c[i] == ':')
            {
                i += 1;
            }
            out.push(Tok::Word(c[start..i].iter().collect()));
        } else {
            return Err(format!("beklenmeyen “{ch}”"));
        }
    }
    Ok(out)
}

struct P {
    t: Vec<Tok>,
    i: usize,
}

impl P {
    fn peek_word(&self, w: &str) -> bool {
        matches!(self.t.get(self.i), Some(Tok::Word(x)) if x.eq_ignore_ascii_case(w))
    }
    fn word(&mut self, w: &str) -> bool {
        let yes = self.peek_word(w);
        if yes {
            self.i += 1;
        }
        yes
    }
    fn expect(&mut self, t: Tok, what: &str) -> Result<(), String> {
        if self.t.get(self.i) == Some(&t) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("{what} bekleniyordu"))
        }
    }
    fn or(&mut self) -> Result<Value, String> {
        let mut args = vec![self.and()?];
        while self.word("OR") {
            args.push(self.and()?);
        }
        Ok(if args.len() == 1 {
            args.remove(0)
        } else {
            json!({"op": "or", "args": args})
        })
    }
    fn and(&mut self) -> Result<Value, String> {
        let mut args = vec![self.not()?];
        while self.word("AND") {
            args.push(self.not()?);
        }
        Ok(if args.len() == 1 {
            args.remove(0)
        } else {
            json!({"op": "and", "args": args})
        })
    }
    fn not(&mut self) -> Result<Value, String> {
        if self.word("NOT") {
            return Ok(json!({"op": "not", "args": [self.not()?]}));
        }
        self.predicate()
    }
    fn predicate(&mut self) -> Result<Value, String> {
        if self.t.get(self.i) == Some(&Tok::Open) {
            self.i += 1;
            let e = self.or()?;
            self.expect(Tok::Close, "“)”")?;
            return Ok(e);
        }
        let a = self.operand()?;
        let negated = |v: Value, neg: bool| {
            if neg {
                json!({"op": "not", "args": [v]})
            } else {
                v
            }
        };
        if let Some(Tok::Op(op)) = self.t.get(self.i).cloned() {
            self.i += 1;
            let b = self.operand()?;
            return Ok(json!({"op": op, "args": [a, b]}));
        }
        if self.word("IS") {
            let neg = self.word("NOT");
            if !self.word("NULL") {
                return Err("IS'ten sonra NULL bekleniyordu".into());
            }
            return Ok(negated(json!({"op": "isNull", "args": [a]}), neg));
        }
        let neg = self.word("NOT");
        if self.word("LIKE") {
            let b = self.operand()?;
            return Ok(negated(json!({"op": "like", "args": [a, b]}), neg));
        }
        if self.word("IN") {
            self.expect(Tok::Open, "“(”")?;
            let mut list = vec![self.operand()?];
            while self.t.get(self.i) == Some(&Tok::Comma) {
                self.i += 1;
                list.push(self.operand()?);
            }
            self.expect(Tok::Close, "“)”")?;
            return Ok(negated(json!({"op": "in", "args": [a, list]}), neg));
        }
        if self.word("BETWEEN") {
            let lo = self.operand()?;
            if !self.word("AND") {
                return Err("BETWEEN'de AND bekleniyordu".into());
            }
            let hi = self.operand()?;
            return Ok(negated(json!({"op": "between", "args": [a, lo, hi]}), neg));
        }
        if neg {
            return Err("NOT'tan sonra LIKE, IN ya da BETWEEN bekleniyordu".into());
        }
        // A boolean on its own: a property or a literal.
        Ok(a)
    }
    fn operand(&mut self) -> Result<Value, String> {
        let Some(t) = self.t.get(self.i).cloned() else {
            return Err("süzgeç yarıda bitti".into());
        };
        self.i += 1;
        Ok(match t {
            Tok::Num(x) => json!(x),
            Tok::Text(s) => json!(s),
            Tok::Quoted(s) => json!({"property": s}),
            Tok::Word(w) if w.eq_ignore_ascii_case("TRUE") => json!(true),
            Tok::Word(w) if w.eq_ignore_ascii_case("FALSE") => json!(false),
            Tok::Word(w) if self.t.get(self.i) == Some(&Tok::Open) => {
                let key = match w.to_ascii_uppercase().as_str() {
                    "DATE" => "date",
                    "TIMESTAMP" => "timestamp",
                    _ => return Err(format!("“{w}” işlevi CQL2 JSON'a çevrilmiyor")),
                };
                self.i += 1;
                let Some(Tok::Text(s)) = self.t.get(self.i).cloned() else {
                    return Err(format!("{w} bir tarih metni bekliyor"));
                };
                self.i += 1;
                self.expect(Tok::Close, "“)”")?;
                json!({ key: s })
            }
            Tok::Word(w) => json!({"property": w}),
            _ => return Err("bir değer ya da özellik bekleniyordu".into()),
        })
    }
}

/// `text` (CQL2 text) as CQL2 JSON, compact.
pub fn to_json(text: &str) -> Result<String, String> {
    let mut p = P {
        t: tokens(text)?,
        i: 0,
    };
    let e = p.or()?;
    if p.i != p.t.len() {
        return Err("süzgeçte fazladan sözcük var".into());
    }
    Ok(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(t: &str) -> Value {
        serde_json::from_str(&to_json(t).expect(t)).expect("json")
    }

    #[test]
    fn basic_cql2_reads_into_json() {
        assert_eq!(
            j("alan > 500"),
            json!({"op": ">", "args": [{"property": "alan"}, 500.0]})
        );
        assert_eq!(
            j("mahalle = 'Çankaya' AND (ada >= 10 OR NOT ada < 3)"),
            json!({"op": "and", "args": [
                {"op": "=", "args": [{"property": "mahalle"}, "Çankaya"]},
                {"op": "or", "args": [
                    {"op": ">=", "args": [{"property": "ada"}, 10.0]},
                    {"op": "not", "args": [{"op": "<", "args": [{"property": "ada"}, 3.0]}]}]}]})
        );
        assert_eq!(
            j("\"Ada No\" <> 5"),
            json!({"op": "<>", "args": [{"property": "Ada No"}, 5.0]})
        );
        assert_eq!(
            j("malik IS NOT NULL"),
            json!({"op": "not", "args": [{"op": "isNull", "args": [{"property": "malik"}]}]})
        );
        assert_eq!(
            j("ad LIKE 'A%'"),
            json!({"op": "like", "args": [{"property": "ad"}, "A%"]})
        );
        assert_eq!(
            j("tur IN ('a', 'b''c')"),
            json!({"op": "in", "args": [{"property": "tur"}, ["a", "b'c"]]})
        );
        assert_eq!(
            j("no NOT BETWEEN -1 AND 2.5e1"),
            json!({"op": "not", "args": [{"op": "between", "args": [{"property": "no"}, -1.0, 25.0]}]})
        );
        assert_eq!(
            j("tescil < DATE('2020-01-01')"),
            json!({"op": "<", "args": [{"property": "tescil"}, {"date": "2020-01-01"}]})
        );
        assert_eq!(
            j("aktif = TRUE"),
            json!({"op": "=", "args": [{"property": "aktif"}, true]})
        );
    }

    #[test]
    fn what_cannot_be_read_is_refused() {
        for bad in [
            "S_INTERSECTS(geom, POINT(1 2))",
            "alan >",
            "(a = 1",
            "a = 'x",
            "a = 1 b",
            "a ! 2",
            "a NOT = 1",
        ] {
            assert!(to_json(bad).is_err(), "{bad}");
        }
    }
}
