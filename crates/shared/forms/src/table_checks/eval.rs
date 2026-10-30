//! A table constraint judged over a record (docs/adr/0755 §3) with SQL's three-valued meaning in each dialect:
//! SQLite's affinities and storage classes, PostgreSQL's types. Where the answer hangs on what KentOS cannot know (a
//! collation's order, a float's last bit, a function it does not have) the value is not known, and a constraint not
//! known to be false holds.

use std::cmp::Ordering;
use std::collections::HashMap;

use super::Dialect;
use super::parse::{Aff, Bin, Cmp, Expr, Match, Ty};
use super::text::{
    compile_regex, exact_of, pg_like, sqlite_glob, sqlite_like, sqlite_number, sqlite_text, trimmed,
};
use super::value::{Dec, Order, Real, T3, V, Val, clock, day, stamp};

/// What a judgement reads: the dialect, a column's value by its name as the constraint's columns write it, and the
/// constraint's regular expressions compiled once.
pub(super) struct Judge<'a> {
    pub d: Dialect,
    pub columns: &'a [String],
    pub value: &'a dyn Fn(&str) -> Val,
    pub regexes: &'a HashMap<(String, bool), Option<regex_lite::Regex>>,
}

impl Judge<'_> {
    /// The column a name is, as the dialect compares names: SQLite's whatever the case of ASCII letters, PostgreSQL's
    /// unquoted in lowercase.
    fn column(&self, text: &str, quoted: bool) -> Option<&str> {
        let found = match self.d {
            Dialect::Sqlite => self.columns.iter().find(|c| c.eq_ignore_ascii_case(text)),
            Dialect::Postgres if quoted => self.columns.iter().find(|c| *c == text),
            Dialect::Postgres => {
                let lower = text.to_ascii_lowercase();
                self.columns.iter().find(|c| **c == lower)
            }
        };
        found.map(String::as_str)
    }

    /// The truth a constraint's value is: SQLite's as the number it is cast to (zero false), PostgreSQL's boolean.
    pub(super) fn truth(&self, v: &Val) -> T3 {
        match (&v.v, self.d) {
            (V::Null, _) => T3::Null,
            (V::Bool(b), _) => T3::of(*b),
            (V::Int(i), Dialect::Sqlite) => T3::of(*i != 0),
            (V::Real(r), Dialect::Sqlite) => match r.exact {
                Some(e) => T3::of(!e.is_zero()),
                None if r.approx != 0.0 => T3::True,
                None => T3::Unknown,
            },
            (V::Text(t), Dialect::Sqlite) => match sqlite_number(t) {
                Some(n) => self.truth(&Val::of(n)),
                // A text without a leading number is zero.
                None => {
                    let s = t.trim_start_matches(|c: char| c.is_ascii_whitespace());
                    if s.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '+' | '-' | '.')) {
                        T3::Unknown
                    } else {
                        T3::False
                    }
                }
            },
            _ => T3::Unknown,
        }
    }

    fn truth_val(&self, t: T3) -> Val {
        Val::of(match (t, self.d) {
            (T3::Null, _) => V::Null,
            (T3::Unknown, _) => V::Unknown,
            (t, Dialect::Sqlite) => V::Int(i64::from(t == T3::True)),
            (t, Dialect::Postgres) => V::Bool(t == T3::True),
        })
    }

    pub(super) fn eval(&self, e: &Expr) -> Val {
        match e {
            Expr::Null => Val::null(),
            Expr::Bool(b) => match self.d {
                Dialect::Sqlite => Val::of(V::Int(i64::from(*b))),
                Dialect::Postgres => Val::of(V::Bool(*b)),
            },
            Expr::Num(t) => Val::of(self.number(t)),
            Expr::Str(s) => Val::of(V::Text(s.clone())),
            Expr::Name {
                text,
                quoted,
                double,
            } => match self.column(text, *quoted) {
                Some(c) => (self.value)(c),
                None if self.d == Dialect::Sqlite && *double => Val::of(V::Text(text.clone())),
                None => Val::unknown(),
            },
            Expr::Opaque => Val::unknown(),
            Expr::Cast(e, ty) => self.cast(self.eval(e), ty),
            Expr::Collate(e, name) => Val {
                coll: Some(name.to_ascii_lowercase()),
                ..self.eval(e)
            },
            Expr::Neg(e) => Val::of(self.negated(self.eval(e).v)),
            Expr::Pos(e) => {
                let v = self.eval(e);
                match (&v.v, self.d) {
                    (V::Int(_) | V::Real(_) | V::Null, _) | (_, Dialect::Sqlite) => Val {
                        aff: Aff::None,
                        ..v
                    },
                    _ => Val::unknown(),
                }
            }
            Expr::BitNot(e) => Val::of(match (self.eval(e).v, self.d) {
                (V::Null, _) => V::Null,
                (V::Int(i), Dialect::Sqlite) => V::Int(!i),
                _ => V::Unknown,
            }),
            Expr::Not(e) => {
                let t = self.truth(&self.eval(e));
                self.truth_val(t.not())
            }
            Expr::And(a, b) => {
                let t = self.truth(&self.eval(a)).and(self.truth(&self.eval(b)));
                self.truth_val(t)
            }
            Expr::Or(a, b) => {
                let t = self.truth(&self.eval(a)).or(self.truth(&self.eval(b)));
                self.truth_val(t)
            }
            Expr::Bin(Bin::Cmp(c), a, b) => {
                let o = self.compare(&self.eval(a), &self.eval(b));
                self.truth_val(holds(*c, o))
            }
            Expr::Bin(op, a, b) => Val::of(self.arithmetic(*op, self.eval(a).v, self.eval(b).v)),
            Expr::Same { a, b, negate } => {
                let t = self.same(&self.eval(a), &self.eval(b));
                self.truth_val(if *negate { t.not() } else { t })
            }
            Expr::IsNull { e, not } => {
                let t = match self.eval(e).v {
                    V::Null => T3::True,
                    V::Unknown => T3::Unknown,
                    _ => T3::False,
                };
                self.truth_val(if *not { t.not() } else { t })
            }
            Expr::IsBool { e, not, value } => {
                let truth = self.truth(&self.eval(e));
                let t = match (truth, value) {
                    (T3::Unknown, _) => T3::Unknown,
                    (t, Some(true)) => T3::of(t == T3::True),
                    (t, Some(false)) => T3::of(t == T3::False),
                    (t, None) => T3::of(t == T3::Null),
                };
                self.truth_val(if *not { t.not() } else { t })
            }
            Expr::In { e, not, list } => {
                let x = self.eval(e);
                let t = if list.is_empty() {
                    T3::False
                } else if matches!(x.v, V::Null) {
                    T3::Null
                } else {
                    // SQLite: `a IN (x, y)` is `a = +x OR a = +y`, the list without affinity.
                    any(list.iter().map(|el| {
                        let y = Val {
                            aff: Aff::None,
                            ..self.eval(el)
                        };
                        holds(Cmp::Eq, self.compare(&x, &y))
                    }))
                };
                self.truth_val(if *not { t.not() } else { t })
            }
            Expr::Quantified { e, op, all, array } => {
                let x = self.eval(e);
                let t = match self.eval(array).v {
                    V::Null => T3::Null,
                    V::Array(items) => {
                        let each = items
                            .into_iter()
                            .map(|el| holds(*op, self.compare(&x, &Val::of(el))));
                        if *all { every(each) } else { any(each) }
                    }
                    _ => T3::Unknown,
                };
                self.truth_val(t)
            }
            Expr::Between {
                e,
                not,
                symmetric,
                lo,
                hi,
            } => {
                let (x, lo, hi) = (self.eval(e), self.eval(lo), self.eval(hi));
                let within = |lo: &Val, hi: &Val| {
                    holds(Cmp::Ge, self.compare(&x, lo)).and(holds(Cmp::Le, self.compare(&x, hi)))
                };
                let mut t = within(&lo, &hi);
                if *symmetric {
                    t = t.or(within(&hi, &lo));
                }
                self.truth_val(if *not { t.not() } else { t })
            }
            Expr::Like {
                e,
                not,
                how,
                pattern,
                escape,
            } => {
                let t = self.matched(e, *how, pattern, escape.as_deref());
                self.truth_val(if *not { t.not() } else { t })
            }
            Expr::Case {
                operand,
                whens,
                otherwise,
            } => {
                let o = operand.as_ref().map(|o| self.eval(o));
                for (w, then) in whens {
                    let w = self.eval(w);
                    let t = match &o {
                        Some(o) => holds(Cmp::Eq, self.compare(o, &w)),
                        None => self.truth(&w),
                    };
                    match t {
                        T3::True => return self.eval(then),
                        T3::Unknown => return Val::unknown(),
                        T3::False | T3::Null => {}
                    }
                }
                otherwise.as_ref().map_or_else(Val::null, |e| self.eval(e))
            }
            Expr::Call(name, args) => {
                let args: Vec<Val> = args.iter().map(|a| self.eval(a)).collect();
                Val::of(self.call(name, args))
            }
            Expr::Array(items) => Val::of(V::Array(items.iter().map(|x| self.eval(x).v).collect())),
        }
    }

    /// A number as the dialect reads its literal: a whole one that fits an integer, else an exact decimal (SQLite's
    /// real, with the float it is).
    fn number(&self, t: &str) -> V {
        if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
            return u64::from_str_radix(hex, 16).map_or(V::Unknown, |u| V::Int(u as i64));
        }
        if !t.contains(['.', 'e', 'E'])
            && let Ok(i) = t.parse::<i64>()
        {
            return V::Int(i);
        }
        Real::parse(t).map_or(V::Unknown, V::Real)
    }

    fn negated(&self, v: V) -> V {
        match v {
            V::Null => V::Null,
            V::Int(i) => i.checked_neg().map_or(V::Unknown, V::Int),
            V::Real(r) => V::Real(Real {
                exact: r.exact.and_then(Dec::neg),
                approx: -r.approx,
            }),
            V::Text(t) if self.d == Dialect::Sqlite => {
                sqlite_number(&t).map_or(V::Unknown, |n| self.negated(n))
            }
            _ => V::Unknown,
        }
    }

    /// The numbers of an arithmetic's operands: SQLite reads a text whole as its number.
    fn operand(&self, v: V) -> V {
        match v {
            V::Text(t) if self.d == Dialect::Sqlite => sqlite_number(&t).unwrap_or(V::Unknown),
            V::Int(_) | V::Real(_) | V::Null => v,
            _ => V::Unknown,
        }
    }

    fn arithmetic(&self, op: Bin, a: V, b: V) -> V {
        if matches!(a, V::Null) || matches!(b, V::Null) {
            return V::Null;
        }
        if op == Bin::Concat {
            let text = |v: &V| match (v, self.d) {
                (_, Dialect::Sqlite) => sqlite_text(v),
                (V::Text(t), _) => Some(t.clone()),
                (V::Int(i), _) => Some(i.to_string()),
                (V::Bool(b), _) => Some(b.to_string()),
                _ => None,
            };
            return match (text(&a), text(&b)) {
                (Some(x), Some(y)) => V::Text(x + &y),
                _ => V::Unknown,
            };
        }
        let (a, b) = (self.operand(a), self.operand(b));
        let zero_divisor = || match self.d {
            Dialect::Sqlite => V::Null,
            Dialect::Postgres => V::Unknown,
        };
        match (a, b) {
            (V::Int(x), V::Int(y)) => {
                let r = match op {
                    Bin::Add => x.checked_add(y),
                    Bin::Sub => x.checked_sub(y),
                    Bin::Mul => x.checked_mul(y),
                    Bin::Div if y == 0 => return zero_divisor(),
                    Bin::Div => x.checked_div(y),
                    Bin::Rem if y == 0 => return zero_divisor(),
                    Bin::Rem => x.checked_rem(y),
                    Bin::BitAnd if self.d == Dialect::Sqlite => Some(x & y),
                    Bin::BitOr if self.d == Dialect::Sqlite => Some(x | y),
                    Bin::Shl if self.d == Dialect::Sqlite && (0..=62).contains(&y) => {
                        x.checked_mul(1i64 << y)
                    }
                    Bin::Shr if self.d == Dialect::Sqlite && (0..=62).contains(&y) => Some(x >> y),
                    _ => None,
                };
                r.map_or(V::Unknown, V::Int)
            }
            (a @ (V::Int(_) | V::Real(_)), b @ (V::Int(_) | V::Real(_))) => {
                let real = |v: &V| match v {
                    V::Int(i) => Real::of_int(*i),
                    V::Real(r) => *r,
                    _ => unreachable!(),
                };
                let (x, y) = (real(&a), real(&b));
                let both = |f: fn(Dec, Dec) -> Option<Dec>| {
                    x.exact.zip(y.exact).and_then(|(p, q)| f(p, q))
                };
                let (exact, approx) = match op {
                    Bin::Add => (both(Dec::add), x.approx + y.approx),
                    Bin::Sub => (both(Dec::sub), x.approx - y.approx),
                    Bin::Mul => (both(Dec::mul), x.approx * y.approx),
                    Bin::Div => {
                        if y.exact.is_some_and(Dec::is_zero) || y.approx == 0.0 {
                            return zero_divisor();
                        }
                        (None, x.approx / y.approx)
                    }
                    _ => return V::Unknown,
                };
                if approx.is_finite() {
                    V::Real(Real { exact, approx })
                } else {
                    V::Unknown
                }
            }
            _ => V::Unknown,
        }
    }

    /// How two values compare, with SQLite's affinities and storage classes or PostgreSQL's types.
    pub(super) fn compare(&self, a: &Val, b: &Val) -> Order {
        if matches!(a.v, V::Null) || matches!(b.v, V::Null) {
            return Order::Null;
        }
        if matches!(a.v, V::Unknown) || matches!(b.v, V::Unknown) {
            return Order::Unknown;
        }
        let coll = a.coll.as_deref().or(b.coll.as_deref());
        match self.d {
            Dialect::Sqlite => {
                let (mut x, mut y) = (a.v.clone(), b.v.clone());
                let loose = |aff: Aff| matches!(aff, Aff::Text | Aff::Blob | Aff::None);
                if a.aff.numeric() && loose(b.aff) {
                    y = numeric_affinity(y);
                } else if b.aff.numeric() && loose(a.aff) {
                    x = numeric_affinity(x);
                } else if a.aff == Aff::Text && b.aff == Aff::None {
                    y = text_affinity(y);
                } else if b.aff == Aff::Text && a.aff == Aff::None {
                    x = text_affinity(x);
                }
                sqlite_order(&x, &y, coll)
            }
            Dialect::Postgres => pg_order(&a.v, &b.v, coll),
        }
    }

    /// `IS NOT DISTINCT FROM`: two NULLs are the same, a NULL and a value are not.
    fn same(&self, a: &Val, b: &Val) -> T3 {
        match (&a.v, &b.v) {
            (V::Unknown, _) | (_, V::Unknown) => T3::Unknown,
            (V::Null, V::Null) => T3::True,
            (V::Null, _) | (_, V::Null) => T3::False,
            _ => match self.compare(a, b) {
                Order::Eq => T3::True,
                Order::Lt | Order::Gt | Order::Ne => T3::False,
                Order::Null | Order::Unknown => T3::Unknown,
            },
        }
    }

    fn matched(&self, e: &Expr, how: Match, pattern: &Expr, escape: Option<&Expr>) -> T3 {
        let (s, p) = (self.eval(e).v, self.eval(pattern).v);
        let esc = escape.map(|x| self.eval(x).v);
        if matches!(s, V::Null) || matches!(p, V::Null) || matches!(esc, Some(V::Null)) {
            return T3::Null;
        }
        let text = |v: &V| match (self.d, v) {
            (Dialect::Sqlite, v) => sqlite_text(v),
            (Dialect::Postgres, V::Text(t)) => Some(t.clone()),
            _ => None,
        };
        let (Some(s), Some(p)) = (text(&s), text(&p)) else {
            return T3::Unknown;
        };
        let one = |v: &Option<V>| -> Option<Option<char>> {
            match v {
                None => Some(None),
                Some(V::Text(t)) => {
                    let mut c = t.chars();
                    match (c.next(), c.next()) {
                        (None, _) => Some(None),
                        (Some(x), None) => Some(Some(x)),
                        _ => None,
                    }
                }
                _ => None,
            }
        };
        match (self.d, how) {
            (Dialect::Sqlite, Match::Like) => match one(&esc) {
                Some(esc) => T3::of(sqlite_like(&s, &p, esc)),
                None => T3::Unknown,
            },
            (Dialect::Sqlite, Match::Glob) if esc.is_none() => T3::of(sqlite_glob(&s, &p)),
            (Dialect::Postgres, Match::Like | Match::ILike) => {
                let esc = match &esc {
                    None => Some('\\'),
                    Some(_) => match one(&esc) {
                        Some(e) => e,
                        None => return T3::Unknown,
                    },
                };
                if how == Match::ILike {
                    if !s.is_ascii() || !p.is_ascii() {
                        return T3::Unknown;
                    }
                    return pg_like(&s.to_ascii_lowercase(), &p.to_ascii_lowercase(), esc)
                        .map_or(T3::Unknown, T3::of);
                }
                pg_like(&s, &p, esc).map_or(T3::Unknown, T3::of)
            }
            (Dialect::Postgres, Match::Regex { insensitive }) if esc.is_none() => {
                if !s.is_ascii() {
                    return T3::Unknown;
                }
                let compiled;
                let re = match self.regexes.get(&(p.clone(), insensitive)) {
                    Some(r) => r.as_ref(),
                    None => {
                        compiled = compile_regex(&p, insensitive);
                        compiled.as_ref()
                    }
                };
                re.map_or(T3::Unknown, |r| T3::of(r.is_match(&s)))
            }
            _ => T3::Unknown,
        }
    }

    fn cast(&self, v: Val, ty: &Ty) -> Val {
        if matches!(v.v, V::Null) {
            let aff = match ty {
                Ty::Affinity(a) => *a,
                _ => Aff::None,
            };
            return Val {
                v: V::Null,
                aff,
                coll: None,
            };
        }
        if matches!(v.v, V::Unknown) {
            return Val::unknown();
        }
        match (self.d, ty) {
            (Dialect::Sqlite, Ty::Affinity(aff)) => Val {
                v: sqlite_cast(v.v, *aff),
                aff: *aff,
                coll: None,
            },
            (Dialect::Postgres, ty) => Val::of(pg_cast(v.v, ty)),
            _ => Val::unknown(),
        }
    }

    fn call(&self, name: &str, args: Vec<Val>) -> V {
        let sqlite = self.d == Dialect::Sqlite;
        let vals: Vec<V> = args.into_iter().map(|a| a.v).collect();
        if !matches!(name, "coalesce" | "ifnull" | "nullif") {
            if vals.iter().any(|v| matches!(v, V::Unknown)) {
                return V::Unknown;
            }
            if vals.iter().any(|v| matches!(v, V::Null)) {
                return V::Null;
            }
        }
        let text = |v: &V| -> Option<String> {
            match v {
                V::Text(t) => Some(t.clone()),
                V::Int(i) if sqlite => Some(i.to_string()),
                _ => None,
            }
        };
        let int = |v: &V| match v {
            V::Int(i) => Some(*i),
            _ => None,
        };
        let count = |n: usize| i64::try_from(n).map_or(V::Unknown, V::Int);
        let out = match (name, vals.as_slice()) {
            ("length", [x]) => text(x).map(|t| count(t.chars().count())),
            ("char_length" | "character_length", [x]) if !sqlite => {
                text(x).map(|t| count(t.chars().count()))
            }
            ("octet_length", [x]) => {
                text(x).and_then(|t| (sqlite || t.is_ascii()).then(|| count(t.len())))
            }
            ("lower" | "upper", [x]) => text(x).and_then(|t| {
                (sqlite || t.is_ascii()).then(|| {
                    V::Text(if name == "lower" {
                        t.to_ascii_lowercase()
                    } else {
                        t.to_ascii_uppercase()
                    })
                })
            }),
            ("trim" | "ltrim" | "rtrim" | "btrim", [x, rest @ ..])
                if rest.len() <= 1 && (name != "btrim" || !sqlite) =>
            {
                let set = match rest {
                    [] => Some(" ".to_owned()),
                    [s] => text(s),
                    _ => None,
                };
                text(x)
                    .zip(set)
                    .map(|(t, set)| V::Text(trimmed(&t, &set, name != "rtrim", name != "ltrim")))
            }
            ("abs", [x]) => match self.operand(x.clone()) {
                V::Int(i) => i.checked_abs().map(V::Int),
                V::Real(r) => Some(V::Real(Real {
                    exact: r.exact.and_then(Dec::abs),
                    approx: r.approx.abs(),
                })),
                _ => None,
            },
            ("coalesce", items) if !items.is_empty() => Some(coalesce(items)),
            ("ifnull", items @ [_, _]) if sqlite => Some(coalesce(items)),
            ("nullif", [a, b]) => Some(
                match self.compare(&Val::of(a.clone()), &Val::of(b.clone())) {
                    Order::Eq => V::Null,
                    Order::Unknown => V::Unknown,
                    _ => a.clone(),
                },
            ),
            ("substr" | "substring", [x, start, rest @ ..])
                if rest.len() <= 1 && (name == "substr" || !sqlite) =>
            {
                let len = match rest {
                    [] => Some(None),
                    [n] => int(n).filter(|n| *n >= 0).map(Some),
                    _ => None,
                };
                match (text(x), int(start).filter(|s| *s >= 1), len) {
                    (Some(t), Some(s), Some(n)) => {
                        let from = usize::try_from(s - 1).unwrap_or(usize::MAX);
                        let it = t.chars().skip(from);
                        Some(V::Text(match n {
                            Some(n) => it.take(usize::try_from(n).unwrap_or(usize::MAX)).collect(),
                            None => it.collect(),
                        }))
                    }
                    _ => None,
                }
            }
            ("replace", [x, from, to]) => match (text(x), text(from), text(to)) {
                (Some(t), Some(f), Some(r)) => {
                    Some(V::Text(if f.is_empty() { t } else { t.replace(&f, &r) }))
                }
                _ => None,
            },
            ("instr", [x, y]) if sqlite => text(x).zip(text(y)).map(|(t, s)| position(&t, &s)),
            ("strpos", [x, y]) if !sqlite => text(x).zip(text(y)).map(|(t, s)| position(&t, &s)),
            ("typeof", [x]) if sqlite => Some(V::Text(
                match x {
                    V::Int(_) => "integer",
                    V::Real(_) => "real",
                    V::Text(_) => "text",
                    _ => return V::Unknown,
                }
                .to_owned(),
            )),
            ("round", [x, rest @ ..]) if !sqlite && rest.len() <= 1 => {
                let digits = match rest {
                    [] => Some(0),
                    [d] => int(d)
                        .and_then(|d| u32::try_from(d).ok())
                        .filter(|d| *d <= 30),
                    _ => None,
                };
                match (x, digits) {
                    (V::Int(i), Some(_)) => Some(V::Int(*i)),
                    (V::Real(r), Some(d)) => r.exact.and_then(|e| e.round(d)).map(|e| {
                        V::Real(Real {
                            exact: Some(e),
                            approx: e.text().parse().unwrap_or(f64::NAN),
                        })
                    }),
                    _ => None,
                }
            }
            ("date", [V::Text(t)]) if sqlite => day(t).map(|_| V::Text(t.clone())),
            _ => None,
        };
        out.unwrap_or(V::Unknown)
    }
}

/// The first value that is not NULL; not known when one before it is not known.
fn coalesce(items: &[V]) -> V {
    for v in items {
        match v {
            V::Null => {}
            V::Unknown => return V::Unknown,
            other => return other.clone(),
        }
    }
    V::Null
}

/// Where `needle` begins in `hay`, counted in characters from 1; 0 when it does not.
fn position(hay: &str, needle: &str) -> V {
    match hay.find(needle) {
        Some(at) => i64::try_from(hay[..at].chars().count() + 1).map_or(V::Unknown, V::Int),
        None => V::Int(0),
    }
}

/// Whether a comparison holds for an order.
fn holds(c: Cmp, o: Order) -> T3 {
    match o {
        Order::Null => T3::Null,
        Order::Unknown => T3::Unknown,
        Order::Ne => match c {
            Cmp::Eq => T3::False,
            Cmp::Ne => T3::True,
            _ => T3::Unknown,
        },
        o => T3::of(match c {
            Cmp::Eq => o == Order::Eq,
            Cmp::Ne => o != Order::Eq,
            Cmp::Lt => o == Order::Lt,
            Cmp::Le => o != Order::Gt,
            Cmp::Gt => o == Order::Gt,
            Cmp::Ge => o != Order::Lt,
        }),
    }
}

fn any(items: impl Iterator<Item = T3>) -> T3 {
    items.fold(T3::False, T3::or)
}

fn every(items: impl Iterator<Item = T3>) -> T3 {
    items.fold(T3::True, T3::and)
}

/// SQLite's numeric affinity on a value: a text that reads whole as a number becomes it.
fn numeric_affinity(v: V) -> V {
    match v {
        V::Text(t) => sqlite_number(&t).unwrap_or(V::Text(t)),
        other => other,
    }
}

/// SQLite's text affinity on a value: an integer becomes its digits; a real's text is SQLite's own (not known).
fn text_affinity(v: V) -> V {
    match v {
        V::Int(i) => V::Text(i.to_string()),
        V::Real(_) => V::Unknown,
        other => other,
    }
}

/// Two numbers' order: whole numbers exactly; others where their exact decimals and their floats agree and the floats
/// are not a few bits apart, else not known.
fn number_order(a: &V, b: &V) -> Order {
    if let (V::Int(x), V::Int(y)) = (a, b) {
        return Order::of(x.cmp(y));
    }
    let approx = |v: &V| match v {
        V::Int(i) => *i as f64,
        V::Real(r) => r.approx,
        _ => f64::NAN,
    };
    let (x, y) = (approx(a), approx(b));
    let Some(float) = x.partial_cmp(&y) else {
        return Order::Unknown;
    };
    let close = x != y && (x - y).abs() <= 4.0 * f64::EPSILON * x.abs().max(y.abs());
    match exact_of(a)
        .zip(exact_of(b))
        .and_then(|(p, q)| Dec::order(p, q))
    {
        Some(exact) if exact == float && !close => Order::of(exact),
        Some(_) => Order::Unknown,
        None if float != Ordering::Equal && !close => Order::of(float),
        None => Order::Unknown,
    }
}

/// SQLite's order of two values after their affinities: NULL first, then numbers, then texts (by the collation:
/// bytes, ASCII letters' case aside, trailing blanks aside), then blobs.
fn sqlite_order(a: &V, b: &V, coll: Option<&str>) -> Order {
    let num = |v: &V| matches!(v, V::Int(_) | V::Real(_));
    match (a, b) {
        (V::Unknown, _) | (_, V::Unknown) => Order::Unknown,
        _ if num(a) && num(b) => number_order(a, b),
        (_, V::Text(_)) if num(a) => Order::Lt,
        (V::Text(_), _) if num(b) => Order::Gt,
        (V::Text(x), V::Text(y)) => match coll {
            None | Some("binary") => Order::of(x.as_bytes().cmp(y.as_bytes())),
            Some("nocase") => Order::of(
                x.to_ascii_lowercase()
                    .as_bytes()
                    .cmp(y.to_ascii_lowercase().as_bytes()),
            ),
            Some("rtrim") => Order::of(
                x.trim_end_matches(' ')
                    .as_bytes()
                    .cmp(y.trim_end_matches(' ').as_bytes()),
            ),
            Some(_) => Order::Unknown,
        },
        _ => Order::Unknown,
    }
}

/// A text as a value of another's type, as PostgreSQL reads a literal: none when it does not read so.
fn pg_coerced(t: &str, like: &V) -> Option<V> {
    let s = t.trim();
    Some(match like {
        V::Int(_) => V::Int(s.parse().ok()?),
        V::Real(_) => V::Real(Real::parse(s)?),
        V::Bool(_) => V::Bool(pg_bool(s)?),
        V::Date(_) => V::Date(day(s)?),
        V::Time(_) => V::Time(clock(s)?),
        V::Stamp(..) => {
            let (d, c) = stamp(s)?;
            V::Stamp(d, c)
        }
        _ => return None,
    })
}

fn pg_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "t" | "true" | "y" | "yes" | "on" | "1" => Some(true),
        "f" | "false" | "n" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// PostgreSQL's order of two values by their types: a text's order hangs on the collation (only equality is known,
/// all of it under `C`), a day is midnight of its date and time.
fn pg_order(a: &V, b: &V, coll: Option<&str>) -> Order {
    let num = |v: &V| matches!(v, V::Int(_) | V::Real(_));
    let moment = |v: &V| match v {
        V::Date(d) => Some((*d, 0u64)),
        V::Stamp(d, c) => Some((*d, *c)),
        _ => None,
    };
    match (a, b) {
        _ if num(a) && num(b) => number_order(a, b),
        (V::Text(x), V::Text(y)) => match coll {
            None | Some("default") => {
                if x == y {
                    Order::Eq
                } else {
                    Order::Ne
                }
            }
            Some("c" | "posix") => Order::of(x.as_bytes().cmp(y.as_bytes())),
            Some(_) => Order::Unknown,
        },
        (V::Bool(x), V::Bool(y)) => Order::of(x.cmp(y)),
        (V::Time(x), V::Time(y)) => Order::of(x.cmp(y)),
        (V::Text(t), other) => {
            pg_coerced(t, other).map_or(Order::Unknown, |v| pg_order(&v, other, coll))
        }
        (other, V::Text(t)) => {
            pg_coerced(t, other).map_or(Order::Unknown, |v| pg_order(other, &v, coll))
        }
        _ => match (moment(a), moment(b)) {
            (Some(x), Some(y)) => Order::of(x.cmp(&y)),
            _ => Order::Unknown,
        },
    }
}

/// SQLite's `CAST` to an affinity, where it is exact: a whole integer text to an integer, an integer to its text, a
/// real truncated toward zero; texts with a number's start and reals' texts are not known.
fn sqlite_cast(v: V, aff: Aff) -> V {
    match (aff, v) {
        (Aff::Integer, V::Int(i)) => V::Int(i),
        // Toward zero, where the decimal and the float SQLite holds agree.
        (Aff::Integer, V::Real(r)) => match r.exact.and_then(Dec::trunc) {
            Some(t) if r.approx.abs() < 9.0e15 && r.approx.trunc() == t as f64 => V::Int(t),
            _ => V::Unknown,
        },
        (Aff::Integer, V::Text(t)) => {
            let s = t.trim();
            s.parse::<i64>().map_or(V::Unknown, V::Int)
        }
        (Aff::Text, v @ V::Text(_)) => v,
        (Aff::Text, V::Int(i)) => V::Text(i.to_string()),
        (Aff::Real, V::Int(i)) => V::Real(Real::of_int(i)),
        (Aff::Real, v @ V::Real(_)) => v,
        (Aff::Real, V::Text(t)) => match sqlite_number(&t) {
            Some(V::Int(i)) => V::Real(Real::of_int(i)),
            Some(r @ V::Real(_)) => r,
            _ => V::Unknown,
        },
        (Aff::Numeric, V::Int(i)) => V::Int(i),
        (Aff::Numeric, V::Text(t)) => match t.trim().parse::<i64>() {
            Ok(i) => V::Int(i),
            Err(_) => V::Unknown,
        },
        _ => V::Unknown,
    }
}

/// PostgreSQL's cast, where its meaning is known.
fn pg_cast(v: V, ty: &Ty) -> V {
    match (ty, v) {
        (Ty::Text, V::Text(t)) => V::Text(t),
        (Ty::Text, V::Int(i)) => V::Text(i.to_string()),
        (Ty::Text, V::Bool(b)) => V::Text(b.to_string()),
        (Ty::Int, V::Int(i)) => V::Int(i),
        (Ty::Int, V::Text(t)) => t.trim().parse().map_or(V::Unknown, V::Int),
        // To the nearest; a half rounds away from zero for numeric and to even for a float: not known.
        (Ty::Int, V::Real(r)) => match r.exact {
            Some(e) if !e.is_half() => match e.round(0).and_then(Dec::whole) {
                Some(w) if r.approx.abs() < 9.0e15 && r.approx.round() == w as f64 => V::Int(w),
                _ => V::Unknown,
            },
            _ => V::Unknown,
        },
        (Ty::Numeric, V::Int(i)) => V::Real(Real::of_int(i)),
        (Ty::Numeric, v @ V::Real(_)) => v,
        (Ty::Numeric, V::Text(t)) => Real::parse(&t).map_or(V::Unknown, V::Real),
        (Ty::Float, V::Int(i)) => V::Real(Real {
            exact: None,
            approx: i as f64,
        }),
        (Ty::Float, V::Real(r)) => V::Real(Real {
            exact: None,
            approx: r.approx,
        }),
        (Ty::Float, V::Text(t)) => Real::parse(&t).map_or(V::Unknown, |r| {
            V::Real(Real {
                exact: None,
                approx: r.approx,
            })
        }),
        (Ty::Bool, v @ V::Bool(_)) => v,
        (Ty::Bool, V::Text(t)) => pg_bool(t.trim()).map_or(V::Unknown, V::Bool),
        (Ty::Date, v @ V::Date(_)) => v,
        (Ty::Date, V::Text(t)) => day(t.trim()).map_or(V::Unknown, V::Date),
        (Ty::Date, V::Stamp(d, _)) => V::Date(d),
        (Ty::Time, v @ V::Time(_)) => v,
        (Ty::Time, V::Text(t)) => clock(t.trim()).map_or(V::Unknown, V::Time),
        (Ty::Stamp, v @ V::Stamp(..)) => v,
        (Ty::Stamp, V::Date(d)) => V::Stamp(d, 0),
        (Ty::Stamp, V::Text(t)) => stamp(t.trim()).map_or(V::Unknown, |(d, c)| V::Stamp(d, c)),
        (Ty::Uuid, V::Text(t)) if is_uuid(&t) => V::Text(t.to_ascii_lowercase()),
        (Ty::Array(of), V::Array(items)) => {
            V::Array(items.into_iter().map(|x| pg_cast(x, of)).collect())
        }
        (Ty::Array(of), V::Text(t)) => array_literal(&t).map_or(V::Unknown, |items| {
            V::Array(items.into_iter().map(|x| pg_cast(x, of)).collect())
        }),
        (_, V::Null) => V::Null,
        _ => V::Unknown,
    }
}

fn is_uuid(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

/// PostgreSQL's array text (`{a,"b c",NULL}`) as its texts; none for a nested or unreadable one.
fn array_literal(t: &str) -> Option<Vec<V>> {
    let inner = t.trim().strip_prefix('{')?.strip_suffix('}')?;
    let mut out = Vec::new();
    if inner.trim().is_empty() {
        return Some(out);
    }
    let mut chars = inner.chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            chars.next();
        }
        if chars.peek() == Some(&'"') {
            chars.next();
            let mut s = String::new();
            loop {
                match chars.next()? {
                    '\\' => s.push(chars.next()?),
                    '"' => break,
                    c => s.push(c),
                }
            }
            out.push(V::Text(s));
            while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                chars.next();
            }
        } else {
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c == ',' {
                    break;
                }
                if matches!(c, '{' | '}' | '"' | '\\') {
                    return None;
                }
                s.push(c);
                chars.next();
            }
            let s = s.trim();
            if s.is_empty() {
                return None;
            }
            out.push(if s.eq_ignore_ascii_case("null") {
                V::Null
            } else {
                V::Text(s.to_owned())
            });
        }
        match chars.next() {
            None => return Some(out),
            Some(',') => {}
            Some(_) => return None,
        }
    }
}
