//! JSON for the WASM boundary and the golden fixtures (docs/adr/0008),
//! without serde: the core's types read and write themselves through two
//! small traits and the `json_struct!` / `json_tagged!` macros. serde's
//! per-type visitor and serializer code was 42 % of the WASM package.
//!
//! - Numbers parse with `str::parse::<f64>` (correctly rounded) and are
//!   written in their shortest round-trip form, so values cross bit for bit.
//! - NaN and ±∞ are written as the strings `"#NaN"`, `"#Inf"` and `"#-Inf"`
//!   (`apps/web/src/wasm/core.ts` turns them back into numbers, and writes them so
//!   in arguments). A bare `null` where a number is required reads as NaN.
//! - A missing object field reads as `null`; `Option` fields that are
//!   `None` are left out, as TypeScript leaves out undefined properties.
//! - Unknown fields are ignored (an Entity carries id, layer and attributes).

use std::fmt::Write;

/// A parsed JSON value. Objects keep their fields in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

const NULL: Json = Json::Null;

impl Json {
    pub fn parse(text: &str) -> Result<Json, String> {
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
            text,
        };
        let v = p.value(0)?;
        p.ws();
        if p.i != p.s.len() {
            return Err(p.err("fazladan karakter"));
        }
        Ok(v)
    }

    /// A field of an object; `null` when absent (or when this is not an object).
    pub fn get(&self, key: &str) -> &Json {
        match self {
            Json::Obj(fields) => fields
                .iter()
                .rev()
                .find(|(k, _)| k == key)
                .map_or(&NULL, |(_, v)| v),
            _ => &NULL,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Bool(_) => "mantıksal değer",
            Json::Num(_) => "sayı",
            Json::Str(_) => "metin",
            Json::Arr(_) => "dizi",
            Json::Obj(_) => "nesne",
        }
    }
}

/// Nesting deeper than this is refused (a hostile input cannot overflow the stack).
const MAX_DEPTH: usize = 64;

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    text: &'a str,
}

impl Parser<'_> {
    fn err(&self, what: &str) -> String {
        format!("JSON okunamadı ({}. karakter): {what}", self.i + 1)
    }

    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> Result<(), String> {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            Ok(())
        } else {
            Err(self.err("beklenmeyen değer"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err(self.err("çok derin iç içe yapı"));
        }
        self.ws();
        match self.s.get(self.i) {
            None => Err(self.err("beklenmedik son")),
            Some(b'n') => self.eat("null").map(|_| Json::Null),
            Some(b't') => self.eat("true").map(|_| Json::Bool(true)),
            Some(b'f') => self.eat("false").map(|_| Json::Bool(false)),
            Some(b'"') => self.string().map(Json::Str),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(items));
                        }
                        _ => return Err(self.err("',' ya da ']' bekleniyordu")),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut fields = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(fields));
                }
                loop {
                    self.ws();
                    if self.s.get(self.i) != Some(&b'"') {
                        return Err(self.err("alan adı bekleniyordu"));
                    }
                    let key = self.string()?;
                    self.ws();
                    if self.s.get(self.i) != Some(&b':') {
                        return Err(self.err("':' bekleniyordu"));
                    }
                    self.i += 1;
                    fields.push((key, self.value(depth + 1)?));
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(fields));
                        }
                        _ => return Err(self.err("',' ya da '}' bekleniyordu")),
                    }
                }
            }
            Some(_) => self.number(),
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        while self.i < self.s.len()
            && matches!(
                self.s[self.i],
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
            )
        {
            self.i += 1;
        }
        let t = &self.text[start..self.i];
        // Rust's parser is correctly rounded: the nearest float64, as JSON.parse gives.
        // JSON numbers start with '-' or a digit ('.5' and '+1' are not JSON).
        let starts_well = t
            .bytes()
            .next()
            .is_some_and(|b| b == b'-' || b.is_ascii_digit());
        match t.parse::<f64>() {
            Ok(x) if starts_well => Ok(Json::Num(x)),
            _ => Err(self.err("sayı okunamadı")),
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .text
            .get(self.i..self.i + 4)
            .ok_or_else(|| self.err("eksik \\u kaçışı"))?;
        let v = u32::from_str_radix(h, 16).map_err(|_| self.err("geçersiz \\u kaçışı"))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // opening quote
        let mut out = String::new();
        loop {
            let run = self.i;
            while self.i < self.s.len() && self.s[self.i] != b'"' && self.s[self.i] != b'\\' {
                self.i += 1;
            }
            out.push_str(&self.text[run..self.i]);
            match self.s.get(self.i) {
                None => return Err(self.err("kapanmamış metin")),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(_) => {
                    self.i += 1;
                    let c = *self
                        .s
                        .get(self.i)
                        .ok_or_else(|| self.err("kapanmamış metin"))?;
                    self.i += 1;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi)
                                && self.s[self.i..].starts_with(b"\\u")
                            {
                                self.i += 2;
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF)
                            } else {
                                hi
                            };
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        _ => return Err(self.err("geçersiz kaçış")),
                    }
                }
            }
        }
    }
}

// ── Writing ─────────────────────────────────────────────────────────────

/// A finite number in the shortest form that reads back to the same bits;
/// exponents outside [1e-6, 1e21), as JavaScript prints them.
pub fn write_number(out: &mut String, x: f64) {
    if x.is_nan() {
        out.push_str("\"#NaN\"");
    } else if x == f64::INFINITY {
        out.push_str("\"#Inf\"");
    } else if x == f64::NEG_INFINITY {
        out.push_str("\"#-Inf\"");
    } else if x == 0.0 {
        out.push_str(if x.is_sign_negative() { "-0" } else { "0" });
    } else {
        let a = x.abs();
        // Writing to a String cannot fail.
        let _ = if (1e-6..1e21).contains(&a) {
            write!(out, "{x}")
        } else {
            write!(out, "{x:e}")
        };
    }
}

pub fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// ── The two traits ──────────────────────────────────────────────────────

pub trait FromJson: Sized {
    fn from_json(v: &Json) -> Result<Self, String>;
}

pub trait ToJson {
    fn write_json(&self, out: &mut String);
    /// Left out when a struct field (`None`, as TypeScript leaves out undefined).
    fn is_absent(&self) -> bool {
        false
    }
}

pub fn to_string<T: ToJson + ?Sized>(v: &T) -> String {
    let mut out = String::new();
    v.write_json(&mut out);
    out
}

fn expected(what: &str, v: &Json) -> String {
    format!("{what} bekleniyordu, {} geldi", v.kind())
}

impl FromJson for f64 {
    fn from_json(v: &Json) -> Result<f64, String> {
        match v {
            Json::Num(x) => Ok(*x),
            // JSON.stringify writes NaN and ±∞ as null.
            Json::Null => Ok(f64::NAN),
            Json::Str(s) if s == "#NaN" => Ok(f64::NAN),
            Json::Str(s) if s == "#Inf" => Ok(f64::INFINITY),
            Json::Str(s) if s == "#-Inf" => Ok(f64::NEG_INFINITY),
            _ => Err(expected("sayı", v)),
        }
    }
}

impl FromJson for usize {
    fn from_json(v: &Json) -> Result<usize, String> {
        match v {
            Json::Num(x) if *x >= 0.0 && x.fract() == 0.0 && *x < 9.007_199_254_740_992e15 => {
                Ok(*x as usize)
            }
            _ => Err(expected("sıfır ya da pozitif tamsayı", v)),
        }
    }
}

impl FromJson for bool {
    fn from_json(v: &Json) -> Result<bool, String> {
        match v {
            Json::Bool(b) => Ok(*b),
            _ => Err(expected("mantıksal değer", v)),
        }
    }
}

impl FromJson for String {
    fn from_json(v: &Json) -> Result<String, String> {
        match v {
            Json::Str(s) => Ok(s.clone()),
            _ => Err(expected("metin", v)),
        }
    }
}

impl<T: FromJson> FromJson for Option<T> {
    fn from_json(v: &Json) -> Result<Option<T>, String> {
        match v {
            Json::Null => Ok(None),
            v => T::from_json(v).map(Some),
        }
    }
}

impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(v: &Json) -> Result<Vec<T>, String> {
        match v {
            Json::Arr(items) => items.iter().map(T::from_json).collect(),
            _ => Err(expected("dizi", v)),
        }
    }
}

impl<T: FromJson, const N: usize> FromJson for [T; N] {
    fn from_json(v: &Json) -> Result<[T; N], String> {
        let items: Vec<T> = Vec::from_json(v)?;
        items
            .try_into()
            .map_err(|_| format!("{N} öğeli dizi bekleniyordu"))
    }
}

impl ToJson for f64 {
    fn write_json(&self, out: &mut String) {
        write_number(out, *self);
    }
}

impl ToJson for Json {
    fn write_json(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => b.write_json(out),
            Json::Num(x) => write_number(out, *x),
            Json::Str(s) => write_str(out, s),
            Json::Arr(items) => items.write_json(out),
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(out, k);
                    out.push(':');
                    v.write_json(out);
                }
                out.push('}');
            }
        }
    }
}

macro_rules! int_to_json {
    ($($t:ty),*) => {$(
        impl ToJson for $t {
            fn write_json(&self, out: &mut String) {
                let _ = write!(out, "{self}");
            }
        }
    )*};
}
int_to_json!(i32, i64, u32, u64, usize);

impl ToJson for bool {
    fn write_json(&self, out: &mut String) {
        out.push_str(if *self { "true" } else { "false" });
    }
}

impl ToJson for str {
    fn write_json(&self, out: &mut String) {
        write_str(out, self);
    }
}

impl ToJson for String {
    fn write_json(&self, out: &mut String) {
        write_str(out, self);
    }
}

impl ToJson for &str {
    fn write_json(&self, out: &mut String) {
        write_str(out, self);
    }
}

impl<T: ToJson> ToJson for Option<T> {
    fn write_json(&self, out: &mut String) {
        match self {
            Some(v) => v.write_json(out),
            None => out.push_str("null"),
        }
    }
    fn is_absent(&self) -> bool {
        self.is_none()
    }
}

impl<T: ToJson> ToJson for [T] {
    fn write_json(&self, out: &mut String) {
        out.push('[');
        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            v.write_json(out);
        }
        out.push(']');
    }
}

impl<T: ToJson> ToJson for Vec<T> {
    fn write_json(&self, out: &mut String) {
        self.as_slice().write_json(out);
    }
}

impl<T: ToJson, const N: usize> ToJson for [T; N] {
    fn write_json(&self, out: &mut String) {
        self.as_slice().write_json(out);
    }
}

/// Writes `"name":value` for a struct field unless it is absent.
/// An Option written as `null` instead of being left out (a TypeScript field set to `null`).
pub struct Nullable<'a, T>(pub &'a Option<T>);

impl<T: ToJson> ToJson for Nullable<'_, T> {
    fn write_json(&self, out: &mut String) {
        self.0.write_json(out);
    }
}

pub fn field<T: ToJson + ?Sized>(out: &mut String, first: &mut bool, name: &str, v: &T) {
    if v.is_absent() {
        return;
    }
    if !*first {
        out.push(',');
    }
    *first = false;
    write_str(out, name);
    out.push(':');
    v.write_json(out);
}

/// Reads a struct field, naming it in the error.
pub fn read_field<T: FromJson>(v: &Json, name: &str) -> Result<T, String> {
    T::from_json(v.get(name)).map_err(|e| format!("“{name}”: {e}"))
}

/// The JSON name of a field: its Rust name, or the one given after `=>`.
#[macro_export]
macro_rules! json_name {
    ($f:ident) => {
        stringify!($f)
    };
    ($f:ident, $n:literal) => {
        $n
    };
}

/// A struct read from and written as a JSON object:
/// `json_struct!(Vec2 { x, y })`, `json_struct!(Bounds { min_x => "minX", … })`.
/// `json_struct!(out Layout { … })` only writes (results holding borrowed text).
#[macro_export]
macro_rules! json_struct {
    (out $t:ident { $($f:ident $(=> $n:literal)?),* $(,)? }) => {
        impl $crate::api::json::ToJson for $t {
            fn write_json(&self, out: &mut String) {
                out.push('{');
                let mut first = true;
                $($crate::api::json::field(out, &mut first, $crate::json_name!($f $(, $n)?), &self.$f);)*
                out.push('}');
            }
        }
    };
    ($t:ident { $($f:ident $(=> $n:literal)?),* $(,)? }) => {
        $crate::json_struct!(out $t { $($f $(=> $n)?),* });
        impl $crate::api::json::FromJson for $t {
            fn from_json(v: &$crate::api::json::Json) -> Result<$t, String> {
                if !matches!(v, $crate::api::json::Json::Obj(_)) {
                    return Err(format!("{} nesnesi bekleniyordu", stringify!($t)));
                }
                Ok($t { $($f: $crate::api::json::read_field(v, $crate::json_name!($f $(, $n)?))?,)* })
            }
        }
    };
}

/// An enum tagged by a field, like TypeScript's discriminated unions:
/// `json_tagged!(Edge, "kind", Seg => "seg" { a, b }, Arc => "arc" { c, r, a0, sweep })`.
#[macro_export]
macro_rules! json_tagged {
    ($t:ident, $tag:literal, $($v:ident => $name:literal { $($f:ident $(=> $n:literal)?),* $(,)? }),* $(,)?) => {
        impl $t {
            /// The tag and fields, without the braces (to share an object with more fields).
            #[allow(dead_code)]
            pub fn write_fields(&self, out: &mut String, first: &mut bool) {
                match self {
                    $($t::$v { $($f),* } => {
                        $crate::api::json::field(out, first, $tag, $name);
                        $($crate::api::json::field(out, first, $crate::json_name!($f $(, $n)?), $f);)*
                    })*
                }
            }
            /// The JSON names of a variant's fields, by tag.
            #[allow(dead_code)]
            pub fn field_names(tag: &str) -> &'static [&'static str] {
                match tag {
                    $($name => &[$($crate::json_name!($f $(, $n)?)),*],)*
                    _ => &[],
                }
            }
        }
        impl $crate::api::json::ToJson for $t {
            fn write_json(&self, out: &mut String) {
                out.push('{');
                let mut first = true;
                self.write_fields(out, &mut first);
                out.push('}');
            }
        }
        impl $crate::api::json::FromJson for $t {
            fn from_json(v: &$crate::api::json::Json) -> Result<$t, String> {
                let tag: String = $crate::api::json::read_field(v, $tag)?;
                match tag.as_str() {
                    $($name => Ok($t::$v { $($f: $crate::api::json::read_field(v, $crate::json_name!($f $(, $n)?))?),* }),)*
                    other => Err(format!("{}: bilinmeyen tür “{}”", stringify!($t), other)),
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct P {
        x: f64,
        y_axis: f64,
        note: Option<String>,
    }
    crate::json_struct!(P { x, y_axis => "yAxis", note });

    #[derive(Debug, PartialEq)]
    enum Shape {
        Line { a: [f64; 2], b: [f64; 2] },
        Circle { r: f64 },
    }
    crate::json_tagged!(Shape, "kind", Line => "line" { a, b }, Circle => "circle" { r });

    #[test]
    fn numbers_keep_their_bits_and_non_finite_values() {
        let v = vec![
            0.1,
            -0.0,
            1e21,
            1.5e-7,
            4426815.485128365,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            3.0,
        ];
        let s = to_string(&v);
        assert_eq!(
            s,
            r##"[0.1,-0,1e21,1.5e-7,4426815.485128365,"#NaN","#Inf","#-Inf",3]"##
        );
        let back: Vec<f64> = Vec::from_json(&Json::parse(&s).unwrap()).unwrap();
        for (x, b) in v.iter().zip(&back) {
            assert!(
                x.to_bits() == b.to_bits() || (x.is_nan() && b.is_nan()),
                "{x} → {b}"
            );
        }
        for x in [
            f64::MIN_POSITIVE,
            5e-324,
            f64::MAX,
            123456789.12345678,
            1e-6,
            9.999999999999999e20,
            4420187.52,
        ] {
            let t = to_string(&x);
            assert_eq!(
                f64::from_json(&Json::parse(&t).unwrap()).unwrap().to_bits(),
                x.to_bits(),
                "{t}"
            );
        }
        // JSON.stringify writes NaN as null: it reads back as NaN where a number is needed.
        assert!(f64::from_json(&Json::Null).unwrap().is_nan());
    }

    #[test]
    fn structs_and_tagged_enums_read_and_write_like_typescript() {
        let p = P {
            x: 1.5,
            y_axis: -2.0,
            note: None,
        };
        assert_eq!(to_string(&p), r#"{"x":1.5,"yAxis":-2}"#);
        let q = P::from_json(
            &Json::parse(r#"{"yAxis":3,"x":0.25,"extra":[1,{"a":null}],"note":"a\"b\\c\nç😀"}"#)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            q,
            P {
                x: 0.25,
                y_axis: 3.0,
                note: Some("a\"b\\c\nç😀".into())
            }
        );
        assert_eq!(
            to_string(&q),
            r#"{"x":0.25,"yAxis":3,"note":"a\"b\\c\nç😀"}"#
        );
        let s = vec![
            Shape::Line {
                a: [0.5, 1.0],
                b: [2.0, -3.0],
            },
            Shape::Circle { r: 2.0 },
        ];
        let text = to_string(&s);
        assert_eq!(
            text,
            r#"[{"kind":"line","a":[0.5,1],"b":[2,-3]},{"kind":"circle","r":2}]"#
        );
        assert_eq!(
            Vec::<Shape>::from_json(&Json::parse(&text).unwrap()).unwrap(),
            s
        );
        assert!(
            Shape::from_json(&Json::parse(r#"{"kind":"ellipse"}"#).unwrap())
                .unwrap_err()
                .contains("bilinmeyen tür")
        );
        assert!(
            P::from_json(&Json::parse(r#"{"x":"a"}"#).unwrap())
                .unwrap_err()
                .contains("“x”")
        );
    }

    #[test]
    fn bad_json_is_refused_with_its_position() {
        for bad in [
            "",
            "[1,",
            "{\"a\" 1}",
            "[1 2]",
            "tru",
            "\"abc",
            "[1]x",
            "-",
            "[.5]",
        ] {
            let e = Json::parse(bad).unwrap_err();
            assert!(e.starts_with("JSON okunamadı"), "{bad}: {e}");
        }
        let deep = "[".repeat(100) + &"]".repeat(100);
        assert!(Json::parse(&deep).unwrap_err().contains("derin"));
    }
}
