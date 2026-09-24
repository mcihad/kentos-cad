//! Functions and variables of the expression language (the TypeScript's
//! `EXPR_FUNCTIONS` and `EXPR_VARIABLES`). Names are matched
//! Turkish-insensitively (yuvarla = YUVARLA, $çevre = $cevre); every
//! function also answers to its English (QGIS) name.

use kentos_geometry_core::jsmath::{js_max, js_max_all, js_min, js_min_all, js_round, js_sign};

use std::borrow::Cow;
use std::mem;

use super::value::{Value, into_text, is_empty, to_number, to_text, truthy};
use crate::js::{number, text};

/// A variable: what it reads of the object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Var {
    Area,
    Length,
    Vertices,
    Kind,
    Layer,
    Label,
    Y,
    X,
    Index,
    Id,
    Scale,
}

pub struct VarDef {
    pub var: Var,
    /// Name without "$", as shown.
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
}

pub static VARIABLES: &[VarDef] = &[
    VarDef {
        var: Var::Area,
        name: "alan",
        aliases: &[],
        description: "Alan (m²): kapalı alan (delikler düşülür), daire, tam elips, tarama",
    },
    VarDef {
        var: Var::Length,
        name: "uzunluk",
        aliases: &["çevre", "length", "perimeter"],
        description: "Uzunluk ya da çevre (m)",
    },
    VarDef {
        var: Var::Vertices,
        name: "köşe",
        aliases: &["vertices"],
        description: "Köşe sayısı (delikler dahil)",
    },
    VarDef {
        var: Var::Kind,
        name: "tür",
        aliases: &["type"],
        description: "Nesne türü: “Kapalı alan”, “Çizgi” …",
    },
    VarDef {
        var: Var::Layer,
        name: "katman",
        aliases: &["layer"],
        description: "Katman adı",
    },
    VarDef {
        var: Var::Label,
        name: "etiket",
        aliases: &["label"],
        description: "Çizimde görünen etiket (parsel no, nokta adı)",
    },
    VarDef {
        var: Var::Y,
        name: "y",
        aliases: &[],
        description: "Y (sağa): nesnenin yer noktası",
    },
    VarDef {
        var: Var::X,
        name: "x",
        aliases: &[],
        description: "X (yukarı): nesnenin yer noktası",
    },
    VarDef {
        var: Var::Index,
        name: "sıra",
        aliases: &["row_number"],
        description: "Bu çalıştırmadaki sırası: 1, 2, 3 …",
    },
    VarDef {
        var: Var::Id,
        name: "id",
        aliases: &[],
        description: "Nesne numarası",
    },
    VarDef {
        var: Var::Scale,
        name: "ölçek",
        aliases: &["scale"],
        description: "Çizim ölçeğinin paydası (1/1000 için 1000); yalnızca sembol çizilirken. Metreyi kâğıt mm’sine çevirir: m × 1000 / $ölçek",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Func {
    Round,
    Text,
    Number,
    Int,
    Abs,
    Min,
    Max,
    Upper,
    Lower,
    Trim,
    Length,
    Substr,
    Pad,
    Replace,
    Contains,
    Starts,
    Ends,
    If,
    Empty,
    Coalesce,
}

pub struct FuncDef {
    pub func: Func,
    pub name: &'static str,
    /// English (QGIS) and spelling variants.
    pub aliases: &'static [&'static str],
    /// Least and most arguments (None: any number).
    pub arity: (usize, Option<usize>),
    pub signature: &'static str,
    pub description: &'static str,
}

pub static FUNCTIONS: &[FuncDef] = &[
    FuncDef {
        func: Func::Round,
        name: "yuvarla",
        aliases: &["round"],
        arity: (1, Some(2)),
        signature: "yuvarla(sayı, basamak)",
        description: "Verilen ondalık basamağa yuvarlar: yuvarla(12.345, 2) → 12.35",
    },
    FuncDef {
        func: Func::Text,
        name: "metin",
        aliases: &["to_string", "text", "format_number"],
        arity: (1, Some(2)),
        signature: "metin(değer, basamak)",
        description: "Metne çevirir; basamak verilirse sabit ondalıkla: metin(452.1, 2) → \"452.10\"",
    },
    FuncDef {
        func: Func::Number,
        name: "sayı",
        aliases: &["to_real", "to_number", "number"],
        arity: (1, Some(1)),
        signature: "sayı(metin)",
        description: "Metindeki sayıyı okur; sayı değilse boş",
    },
    FuncDef {
        func: Func::Int,
        name: "tamsayı",
        aliases: &["int", "to_int", "tam"],
        arity: (1, Some(1)),
        signature: "tamsayı(sayı)",
        description: "Ondalık kısmı atar",
    },
    FuncDef {
        func: Func::Abs,
        name: "mutlak",
        aliases: &["abs"],
        arity: (1, Some(1)),
        signature: "mutlak(sayı)",
        description: "Mutlak değer",
    },
    FuncDef {
        func: Func::Min,
        name: "min",
        aliases: &["en_az", "enaz"],
        arity: (1, None),
        signature: "min(a, b, …)",
        description: "En küçük sayı",
    },
    FuncDef {
        func: Func::Max,
        name: "max",
        aliases: &["en_çok", "encok"],
        arity: (1, None),
        signature: "max(a, b, …)",
        description: "En büyük sayı",
    },
    FuncDef {
        func: Func::Upper,
        name: "büyük",
        aliases: &["upper"],
        arity: (1, Some(1)),
        signature: "büyük(metin)",
        description: "Büyük harfe çevirir (Türkçe: i → İ)",
    },
    FuncDef {
        func: Func::Lower,
        name: "küçük",
        aliases: &["lower"],
        arity: (1, Some(1)),
        signature: "küçük(metin)",
        description: "Küçük harfe çevirir (Türkçe: I → ı)",
    },
    FuncDef {
        func: Func::Trim,
        name: "kırp",
        aliases: &["trim"],
        arity: (1, Some(1)),
        signature: "kırp(metin)",
        description: "Baştaki ve sondaki boşlukları siler",
    },
    FuncDef {
        func: Func::Length,
        name: "uzunluk",
        aliases: &["length", "len"],
        arity: (1, Some(1)),
        signature: "uzunluk(metin)",
        description: "Metnin karakter sayısı (nesne uzunluğu için $uzunluk)",
    },
    FuncDef {
        func: Func::Substr,
        name: "parça",
        aliases: &["substr", "parca"],
        arity: (2, Some(3)),
        signature: "parça(metin, başlangıç, uzunluk)",
        description: "Metnin bir parçası; ilk karakter 1: parça(\"P00012\", 2, 3) → \"000\"",
    },
    FuncDef {
        func: Func::Pad,
        name: "doldur",
        aliases: &["lpad"],
        arity: (2, Some(3)),
        signature: "doldur(değer, uzunluk, karakter)",
        description: "Soldan doldurur: doldur(12, 5, \"0\") → \"00012\" (karakter verilmezse 0)",
    },
    FuncDef {
        func: Func::Replace,
        name: "değiştir",
        aliases: &["replace", "degistir"],
        arity: (3, Some(3)),
        signature: "değiştir(metin, aranan, yeni)",
        description: "Metindeki her aranan parçayı yenisiyle değiştirir",
    },
    FuncDef {
        func: Func::Contains,
        name: "içerir",
        aliases: &["contains", "icerir"],
        arity: (2, Some(2)),
        signature: "içerir(metin, aranan)",
        description: "Metin aranan parçayı içeriyor mu (büyük/küçük harf ve Türkçe harf farkı gözetilmez)",
    },
    FuncDef {
        func: Func::Starts,
        name: "başlar",
        aliases: &["starts_with", "baslar"],
        arity: (2, Some(2)),
        signature: "başlar(metin, aranan)",
        description: "Metin aranan parçayla başlıyor mu (harf farkı gözetilmez)",
    },
    FuncDef {
        func: Func::Ends,
        name: "biter",
        aliases: &["ends_with"],
        arity: (2, Some(2)),
        signature: "biter(metin, aranan)",
        description: "Metin aranan parçayla bitiyor mu (harf farkı gözetilmez)",
    },
    FuncDef {
        func: Func::If,
        name: "eğer",
        aliases: &["if", "eger"],
        arity: (3, Some(3)),
        signature: "eğer(koşul, doğruysa, yanlışsa)",
        description: "Koşula göre iki değerden birini verir",
    },
    FuncDef {
        func: Func::Empty,
        name: "boş",
        aliases: &["is_empty", "bos", "is_null"],
        arity: (1, Some(1)),
        signature: "boş(değer)",
        description: "Değer boş mu (alan yok ya da boş metin)",
    },
    FuncDef {
        func: Func::Coalesce,
        name: "varsayılan",
        aliases: &["coalesce", "varsayilan"],
        arity: (2, None),
        signature: "varsayılan(a, b, …)",
        description: "Boş olmayan ilk değer",
    },
];

/// The lookup key of a name: Turkish-folded, white space removed.
fn key(name: &str) -> String {
    text::fold_turkish(name)
        .chars()
        .filter(|&c| !text::is_space(c))
        .collect()
}

pub fn find_function(name: &str) -> Option<&'static FuncDef> {
    let k = key(name);
    FUNCTIONS.iter().find(|f| {
        std::iter::once(f.name)
            .chain(f.aliases.iter().copied())
            .any(|n| key(n) == k)
    })
}

pub fn find_variable(name: &str) -> Option<&'static VarDef> {
    let k = key(name);
    VARIABLES.iter().find(|v| {
        std::iter::once(v.name)
            .chain(v.aliases.iter().copied())
            .any(|n| key(n) == k)
    })
}

/// What JavaScript would throw at (a string past V8's longest): the whole
/// expression is then empty, as the TypeScript's `evaluate` caught it.
#[derive(Debug)]
pub struct Thrown;

const POW10: [f64; 13] = [
    1.0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12,
];

/// `Math.max(0, Math.min(12, Math.round(d)))` as an index (d from `to_number`: never NaN).
fn digits(d: f64) -> usize {
    js_max(0.0, js_min(12.0, js_round(d))) as usize
}

/// A position or length for `slice` and `padStart`: ∞ and anything past the text clamp.
fn units(x: f64, cap: usize) -> usize {
    if x >= cap as f64 { cap } else { x as usize }
}

/// Numeric function: empty in, empty out; a non-finite result is empty.
fn numeric(args: &[Value], f: impl Fn(&mut dyn Iterator<Item = f64>) -> f64) -> Value<'static> {
    if args.iter().any(|a| to_number(a).is_none()) {
        return Value::Null;
    }
    let r = f(&mut args.iter().filter_map(to_number));
    if r.is_finite() {
        Value::Num(r)
    } else {
        Value::Null
    }
}

fn fold(v: &Value) -> String {
    text::fold_turkish(&to_text(v))
}

fn owned(s: String) -> Value<'static> {
    Value::Text(Cow::Owned(s))
}

/// Calls a function on evaluated arguments (`args.len()` is within its
/// arity). A result that is one of the arguments, or a part of one, is
/// moved out of `args` and keeps borrowing what it borrowed.
pub fn call<'a>(f: Func, args: &mut [Value<'a>]) -> Result<Value<'a>, Thrown> {
    let take =
        |args: &mut [Value<'a>], i: usize| args.get_mut(i).map(mem::take).unwrap_or_default();
    let num = |args: &[Value], i: usize| args.get(i).and_then(to_number);
    let null = Value::Null;
    let is_null = |args: &[Value], i: usize| matches!(args.get(i), Some(Value::Null) | None);
    Ok(match f {
        Func::Round => {
            let d = if args.len() > 1 {
                num(args, 1)
            } else {
                Some(0.0)
            };
            let (Some(v), Some(d)) = (num(args, 0), d) else {
                return Ok(null);
            };
            let f = POW10[digits(d)];
            Value::Num(js_round((v + js_sign(v) * f64::EPSILON * v.abs()) * f) / f)
        }
        Func::Text => {
            if args.len() < 2 {
                return Ok(Value::Text(into_text(take(args, 0))));
            }
            let (Some(v), Some(d)) = (num(args, 0), num(args, 1)) else {
                return Ok(null);
            };
            owned(number::to_fixed(v, digits(d) as u32))
        }
        Func::Number => num(args, 0).map_or(null, Value::Num),
        Func::Int => numeric(args, |n| n.next().unwrap_or(f64::NAN).trunc()),
        Func::Abs => numeric(args, |n| n.next().unwrap_or(f64::NAN).abs()),
        Func::Min => numeric(args, |n| js_min_all(n)),
        Func::Max => numeric(args, |n| js_max_all(n)),
        Func::Upper | Func::Lower if is_null(args, 0) => null,
        Func::Upper => owned(text::upper_tr(&to_text(&args[0]))),
        Func::Lower => owned(text::lower_tr(&to_text(&args[0]))),
        Func::Trim if is_null(args, 0) => null,
        Func::Trim => match into_text(take(args, 0)) {
            Cow::Borrowed(b) => Value::Text(Cow::Borrowed(text::trim(b))),
            Cow::Owned(o) => owned(text::trim(&o).to_string()),
        },
        Func::Length if is_null(args, 0) => null,
        Func::Length => Value::Num(text::utf16_len(&to_text(&args[0])) as f64),
        Func::Substr => {
            let len = if args.len() > 2 {
                num(args, 2)
            } else {
                Some(f64::INFINITY)
            };
            let (Some(from), Some(len)) = (num(args, 1), len) else {
                return Ok(null);
            };
            if args[0] == Value::Null {
                return Ok(null);
            }
            let t = to_text(&args[0]);
            let total = text::utf16_len(&t);
            let start = js_max(0.0, js_round(from) - 1.0);
            let end = if len == f64::INFINITY {
                None
            } else {
                Some(units(start + js_max(0.0, js_round(len)), total))
            };
            owned(text::slice(&t, units(start, total), end))
        }
        Func::Pad => {
            let Some(len) = num(args, 1) else {
                return Ok(null);
            };
            if args[0] == Value::Null {
                return Ok(null);
            }
            let fill = match args.get(2) {
                None => u16::from(b'0'),
                Some(c) => text::first_unit(&to_text(c)).unwrap_or(u16::from(b'0')),
            };
            let t = into_text(take(args, 0));
            let target = js_max(0.0, js_round(len));
            if target <= text::utf16_len(&t) as f64 {
                return Ok(Value::Text(t));
            }
            if target > text::MAX_STRING_UNITS as f64 {
                return Err(Thrown);
            }
            let padded = match t {
                Cow::Owned(o) => text::pad_start_owned(o, target as usize, fill),
                Cow::Borrowed(b) => text::pad_start(b, target as usize, fill),
            };
            owned(padded.ok_or(Thrown)?)
        }
        Func::Replace if is_null(args, 0) => null,
        Func::Replace => owned(
            text::split_join(&to_text(&args[0]), &to_text(&args[1]), &to_text(&args[2]))
                .ok_or(Thrown)?,
        ),
        Func::Contains | Func::Starts | Func::Ends if is_null(args, 0) => Value::Bool(false),
        Func::Contains | Func::Starts | Func::Ends => {
            let (s, t) = (fold(&args[0]), fold(&args[1]));
            Value::Bool(match f {
                Func::Contains => s.contains(&t),
                Func::Starts => s.starts_with(&t),
                _ => s.ends_with(&t),
            })
        }
        Func::If => {
            let pick = if truthy(&args[0]) { 1 } else { 2 };
            take(args, pick)
        }
        Func::Empty => Value::Bool(is_empty(&args[0])),
        Func::Coalesce => args
            .iter_mut()
            .find(|a| !is_empty(a))
            .map(mem::take)
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &'static str) -> Value<'static> {
        Value::text(s)
    }

    #[test]
    fn functions_behave_as_the_typescript_did() {
        let n = Value::Num;
        let c = |f, mut a: Vec<Value<'static>>| call(f, &mut a).unwrap_or(Value::text("thrown"));
        assert_eq!(c(Func::Round, vec![n(12.345), n(2.0)]), n(12.35));
        assert_eq!(c(Func::Round, vec![n(1.005), n(2.0)]), n(1.01));
        assert_eq!(c(Func::Text, vec![n(452.1), n(2.0)]), t("452.10"));
        assert_eq!(c(Func::Pad, vec![n(12.0), n(5.0)]), t("00012"));
        assert_eq!(c(Func::Pad, vec![t("12"), n(4.0), t("_")]), t("__12"));
        assert_eq!(c(Func::Substr, vec![t("P00012"), n(2.0), n(3.0)]), t("000"));
        assert_eq!(c(Func::Upper, vec![t("kadıköy")]), t("KADIKÖY"));
        assert_eq!(c(Func::Lower, vec![t("IŞIK")]), t("ışık"));
        assert_eq!(
            c(Func::Contains, vec![t("Arsa"), t("ARS")]),
            Value::Bool(true)
        );
        assert_eq!(
            c(Func::Starts, vec![t("Çınar"), t("cin")]),
            Value::Bool(true)
        );
        assert_eq!(c(Func::Max, vec![n(1.0), t("12"), n(3.0)]), n(12.0));
        assert_eq!(c(Func::Int, vec![n(-2.7)]), n(-2.0));
        assert_eq!(
            c(Func::Replace, vec![t("1245/12"), t("/"), t("-")]),
            t("1245-12")
        );
        assert_eq!(c(Func::Number, vec![t("abc")]), Value::Null);
        assert_eq!(
            c(Func::Coalesce, vec![Value::Null, t(""), t("yok")]),
            t("yok")
        );
        assert_eq!(c(Func::Pad, vec![t("x"), n(1e12)]), t("thrown"));
        assert!(find_function("YUVARLA").is_some() && find_function("en_çok").is_some());
        assert!(find_variable("CEVRE").is_some() && find_variable("nope").is_none());
    }
}
