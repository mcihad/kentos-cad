//! The column engine against the tree evaluator it replaced (docs/adr/0100):
//! random sources, valid and broken, over random objects, in every result
//! mode. The two must agree on everything the language shows: the error and
//! its position, the fields read, each value's kind, its number to the bit,
//! its text and the text's UTF-16 length; one object at a time
//! (`Expr::evaluate`) as well as a table at a time (`rows::evaluate_rows`).
//! The generator is the web's (`apps/web/src/model/expression/cases.ts`),
//! with text outside the Basic Multilingual Plane (emoji) and Greek sigmas
//! added, and numbers that are not finite among the geometry values.
//!
//!   EXPRESSION_DIFF_CASES=200000 cargo test --release -p kentos-expression --test differential

mod reference;

use kentos_expression::rows::{
    As, Column, Layout, MEASURE_STRIDE, RowsInput, Table, evaluate_rows,
};
use kentos_expression::{Value, compile};

/// A small seeded generator (xorshift64*), so a failure can be replayed.
struct Gen(u64);

impl Gen {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// lo ..= hi.
    fn int(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo + 1) as u64) as i64
    }

    fn chance(&mut self, p: f64) -> bool {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64 <= p
    }

    fn pick<'t, T>(&mut self, list: &'t [T]) -> &'t T {
        &list[self.int(0, list.len() as i64 - 1) as usize]
    }

    fn num(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }
}

const WORDS: [&str; 11] = [
    "Parsel", "Ada", "Nitelik", "Kat", "Boş", "Yok", "Ağaç", "İl", "x2", "ıI", "Tür",
];
const BRACKETED: [&str; 5] = ["Tapu alanı", "Ada no", " Kat ", "a,b", "Yok değer"];
const ALPHABET: &str = "aAbBcCçÇdDeEfFgGğĞhHıIiİjJkKlLmMnNoOöÖpPrRsSşŞtTuUüÜvVyYzZ0123456789 .-_/";
const TEXTS: [&str; 41] = [
    "Arsa",
    "arsa",
    "ARSA",
    "Bahçe",
    "bahçe",
    "Çınar",
    "çınar",
    "İzmir",
    "izmir",
    "Işık",
    "ışık",
    "Iğdır",
    "Şişli",
    "ÖZEL",
    "özel",
    "Üsküdar",
    "a b",
    " Arsa ",
    "Arsa ",
    "a\\b",
    "{kenar}",
    "%%d",
    "^ 2",
    "é",
    "e",
    "E",
    "—",
    "\u{a0}x",
    "ΟΔΟΣ",
    "Ωmega",
    "ǿ",
    "x\u{200b}y",
    "kısa",
    "uzun",
    "",
    "a😀b",
    "😀",
    "ΣΑΣ ΣΑ",
    "I\u{307}",
    "ß",
    "İİi",
];
const NUMERIC: [&str; 29] = [
    "12",
    "598.50",
    "-0",
    "0",
    "1e3",
    " 7 ",
    "0x10",
    "1,5",
    "+.5",
    "5.",
    "-12.75",
    "1e400",
    "-1e400",
    "0.1",
    "1234567890.125",
    "3.14159265358979",
    "1e-7",
    "1e21",
    ".",
    "-",
    "12abc",
    "99.995",
    "1.005",
    "2.5",
    "-2.5",
    "0.000001234",
    "100",
    "9",
    "10",
];
const LAYERS: [&str; 4] = ["Parsel sınırı", "Yol ekseni", "İşaret", "d"];
const KINDS: [&str; 6] = [
    "Kapalı alan",
    "Çoklu çizgi",
    "Çizgi",
    "Nokta",
    "Daire",
    "Eğri",
];
const VARIABLES: [&str; 23] = [
    "alan",
    "ALAN",
    "uzunluk",
    "çevre",
    "CEVRE",
    "length",
    "perimeter",
    "köşe",
    "vertices",
    "tür",
    "type",
    "katman",
    "layer",
    "etiket",
    "label",
    "y",
    "x",
    "sıra",
    "row_number",
    "id",
    "ölçek",
    "scale",
    "nope",
];
const FUNCTIONS: [(&str, i64, i64); 35] = [
    ("yuvarla", 1, 2),
    ("ROUND", 1, 2),
    ("metin", 1, 2),
    ("format_number", 1, 2),
    ("sayı", 1, 1),
    ("to_real", 1, 1),
    ("tamsayı", 1, 1),
    ("int", 1, 1),
    ("mutlak", 1, 1),
    ("min", 1, 4),
    ("en_çok", 1, 4),
    ("max", 1, 10),
    ("büyük", 1, 1),
    ("upper", 1, 1),
    ("küçük", 1, 1),
    ("LOWER", 1, 1),
    ("kırp", 1, 1),
    ("uzunluk", 1, 1),
    ("len", 1, 1),
    ("parça", 2, 3),
    ("substr", 2, 3),
    ("doldur", 2, 3),
    ("lpad", 2, 3),
    ("değiştir", 3, 3),
    ("replace", 3, 3),
    ("içerir", 2, 2),
    ("başlar", 2, 2),
    ("biter", 2, 2),
    ("eğer", 3, 3),
    ("IF", 3, 3),
    ("boş", 1, 1),
    ("is_null", 1, 1),
    ("varsayılan", 2, 10),
    ("coalesce", 2, 4),
    ("foo", 1, 1),
];
const BINARY: [&str; 25] = [
    " + ", " - ", " * ", " / ", " % ", " || ", " = ", " == ", " != ", " <> ", " < ", " <= ", " > ",
    " >= ", " ve ", " and ", " veya ", " or ", " VE ", " Veya ", "+", "*", "<", "||", "-",
];
const NUMBERS: [&str; 26] = [
    "12",
    "0.5",
    ".5",
    "1e3",
    "1.5e-2",
    "598.50",
    "0",
    "1234567890.125",
    "3",
    "7",
    "100",
    "1e21",
    "2.5",
    "99.995",
    "1.005",
    "0.1",
    "1e-7",
    "4",
    "2",
    "1e400",
    "12.",
    "1e308",
    "1",
    "5",
    "6",
    "-1",
];
/// Values that are numbers without text (the engine's fast path): variables and literals.
const NUMERIC_ATOMS: [&str; 15] = [
    "$alan",
    "$uzunluk",
    "$köşe",
    "$sıra",
    "$id",
    "$y",
    "$x",
    "$ölçek",
    "1",
    "2",
    "4",
    "0",
    // Within 1e-9 of 1 (equal), and just outside it.
    "1.0000000005",
    "1.000000005",
    "($alan * 1.0000000005)",
];
const COMPARISONS: [&str; 8] = [" = ", " != ", " < ", " <= ", " > ", " >= ", " - ", " % "];

fn fields() -> Vec<&'static str> {
    WORDS
        .iter()
        .copied()
        .chain(BRACKETED.iter().map(|f| f.trim()))
        .collect()
}

fn word(g: &mut Gen) -> String {
    let letters: Vec<char> = ALPHABET.chars().collect();
    (0..g.int(0, 6)).map(|_| *g.pick(&letters)).collect()
}

fn text(g: &mut Gen) -> String {
    match g.int(0, 3) {
        0 => g.pick(&TEXTS).to_string(),
        1 => g.pick(&NUMERIC).to_string(),
        _ => word(g),
    }
}

fn quoted(g: &mut Gen, s: &str) -> String {
    let q = if g.chance(0.5) { '\'' } else { '"' };
    let doubled: String = s
        .chars()
        .flat_map(|c| if c == q { vec![q, q] } else { vec![c] })
        .collect();
    format!("{q}{doubled}{q}")
}

fn atom(g: &mut Gen, depth: u32) -> String {
    match g.int(0, 10) {
        0 | 1 => g.pick(&NUMBERS).to_string(),
        2 => {
            let t = text(g);
            quoted(g, &t)
        }
        3 => g.pick(&WORDS).to_string(),
        4 => format!("[{}]", g.pick(&BRACKETED)),
        5 => format!("${}", g.pick(&VARIABLES)),
        6 => g
            .pick(&[
                "doğru", "yanlış", "boş", "true", "false", "null", "DOĞRU", "Boş", "NULL",
            ])
            .to_string(),
        7 | 8 => {
            let &(name, lo, hi) = g.pick(&FUNCTIONS);
            // Mostly within the function's arity, now and then one too few or too many.
            let n = if g.chance(0.1) {
                *g.pick(&[lo - 1, hi + 1])
            } else {
                g.int(lo, hi)
            };
            let args: Vec<String> = (0..n.max(0)).map(|_| expr(g, depth + 1)).collect();
            let sep = *g.pick(&[", ", ","]);
            format!("{name}({})", args.join(sep))
        }
        9 => format!("({})", expr(g, depth + 1)),
        _ if g.chance(0.15) => {
            // Text past V8's longest string throws, and empties the whole expression.
            let inner = atom(g, depth + 1);
            let wrap = *g.pick(&[
                "boş(boş({}))",
                "eğer(doğru, 1, {})",
                "{} || 'x'",
                "değil değil {}",
                "varsayılan({}, 1)",
            ]);
            wrap.replace("{}", &format!("doldur({inner}, 1e12)"))
        }
        _ => format!(
            "{}{}",
            g.pick(&["-", "+", "değil ", "not ", "NOT ", "- "]),
            atom(g, depth + 1)
        ),
    }
}

fn expr(g: &mut Gen, depth: u32) -> String {
    if g.chance(0.15) {
        // Numbers against numbers, where the engine takes its fast path: equal values too.
        let a = *g.pick(&NUMERIC_ATOMS);
        let op = *g.pick(&COMPARISONS);
        let b = *g.pick(&NUMERIC_ATOMS);
        return format!("{a}{op}{b}");
    }
    if g.chance(0.08) {
        // A number function of an object's number with constant digits (the engine's direct paths).
        let f = *g.pick(&[
            "yuvarla({}, 2)",
            "round({}, 0)",
            "yuvarla({}, 12)",
            "yuvarla({})",
            "tamsayı({})",
            "mutlak({})",
            "min({}, 3)",
            "max({}, 1, 2)",
            "sayı({})",
        ]);
        return f.replace("{}", g.pick(&NUMERIC_ATOMS));
    }
    if g.chance(0.08) {
        // A text function whose value is part of its argument's text (borrowed where it was).
        let f = *g.pick(&[
            "kırp({})",
            "doldur({}, 2)",
            "eğer(doğru, {}, 'y')",
            "varsayılan({}, 'y')",
            "metin({})",
            "parça({}, 2)",
        ]);
        let arg = match g.int(0, 2) {
            0 => g.pick(&WORDS).to_string(),
            1 => format!("[{}]", g.pick(&BRACKETED)),
            _ => {
                let t = *g.pick(&["  Arsa ", "\u{a0}x ", "a", " 7 ", ""]);
                quoted(g, t)
            }
        };
        return f.replace("{}", &arg);
    }
    if depth > 3 || g.chance(0.45) {
        return atom(g, depth);
    }
    let a = expr(g, depth + 1);
    let op = *g.pick(&BINARY);
    format!("{a}{op}{}", expr(g, depth + 1))
}

/// A source: an expression, now and then cut short or with a stray character in it.
fn source(g: &mut Gen) -> String {
    let mut s: Vec<char> = expr(g, 0).chars().collect();
    if g.chance(0.12) {
        let k = g.int(0, s.len() as i64) as usize;
        s.truncate(k);
    }
    if g.chance(0.1) {
        let i = g.int(0, s.len() as i64) as usize;
        let stray = *g.pick(&[
            "#", "@", "[", "]", "$", "'", "\"", "(", ")", ",", " ", "\t", ".", "1.", "e", "!",
            "ve", "\n",
        ]);
        s.splice(i..i, stray.chars());
    }
    s.into_iter().collect()
}

/// An object as the table holds it.
struct Object {
    attrs: Vec<(&'static str, String)>,
    label: Option<String>,
    layer: &'static str,
    kind: &'static str,
    id: f64,
    vertices: f64,
    measures: [f64; MEASURE_STRIDE],
}

fn object(g: &mut Gen, id: usize) -> Object {
    let mut attrs = Vec::new();
    for f in fields() {
        if f != "Yok" && f != "Yok değer" && g.chance(0.6) {
            attrs.push((f, text(g)));
        }
    }
    let value = |g: &mut Gen| {
        let any = g.num(-1e4, 1e4);
        *g.pick(&[
            0.0,
            -0.0,
            12.5,
            598.5,
            1.0 / 3.0,
            1e21,
            1e-7,
            100.0,
            -5.0,
            123_456.789,
            0.1 + 0.2,
            2.5,
            1.0,
            2.0,
            4.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            any,
        ])
    };
    let flags = f64::from(
        u8::from(g.chance(0.8)) + 2 * u8::from(g.chance(0.6)) + 4 * u8::from(g.chance(0.9)),
    );
    Object {
        attrs,
        label: g.chance(0.5).then(|| text(g)),
        layer: g.pick(&LAYERS),
        kind: g.pick(&KINDS),
        id: if g.chance(0.05) { f64::NAN } else { id as f64 },
        vertices: if g.chance(0.2) {
            f64::NAN
        } else {
            g.int(0, 8) as f64
        },
        measures: [flags, value(g), value(g), value(g), value(g), 0.0],
    }
}

/// The table (the layout `rows` documents) for fields and needs.
fn table(
    fields: &[String],
    needs: kentos_expression::Needs,
    objects: &[Object],
) -> (String, Vec<i32>, Vec<f64>, Vec<f64>) {
    let mut texts = String::new();
    let mut lens = Vec::new();
    let mut numbers = Vec::new();
    let mut measures = Vec::new();
    let mut put = |v: Option<&str>| match v {
        None => lens.push(-1),
        Some(s) => {
            texts.push_str(s);
            lens.push(s.encode_utf16().count() as i32);
        }
    };
    for o in objects {
        for f in fields {
            put(o
                .attrs
                .iter()
                .find(|(k, _)| k == f)
                .map(|(_, v)| v.as_str()));
        }
        if needs.label {
            put(o.label.as_deref());
        }
        if needs.layer {
            put(Some(o.layer));
        }
        if needs.kind {
            put(Some(o.kind));
        }
        if needs.id {
            numbers.push(o.id);
        }
        if needs.vertices {
            numbers.push(o.vertices);
        }
        if needs.measured {
            measures.extend(o.measures);
        }
    }
    (texts, lens, numbers, measures)
}

/// Where two columns differ, if they do: kinds, numbers to the bit, texts and lengths.
fn differs(new: &Column, old: &Column) -> Option<String> {
    if new.kinds != old.kinds {
        return Some(format!("türler {:?} ≠ {:?}", new.kinds, old.kinds));
    }
    let bits = |c: &Column| c.numbers.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    if bits(new) != bits(old) {
        return Some(format!("sayılar {:?} ≠ {:?}", new.numbers, old.numbers));
    }
    if new.texts != old.texts || new.text_lens != old.text_lens {
        return Some(format!(
            "metinler {:?} {:?} ≠ {:?} {:?}",
            new.texts, new.text_lens, old.texts, old.text_lens
        ));
    }
    None
}

/// Two values the same to the bit (NaN equals NaN).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x.to_bits() == y.to_bits(),
        (a, b) => a == b,
    }
}

const MODES: [As; 5] = [As::Value, As::Number, As::Text, As::Bool, As::TextNumber];

#[test]
fn the_column_engine_gives_the_tree_evaluators_answers() {
    let cases: usize = std::env::var("EXPRESSION_DIFF_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000);
    let seed: u64 = std::env::var("EXPRESSION_DIFF_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x9e37_79b9_7f4a_7c15);
    let mut g = Gen(seed);
    let (mut errors, mut values) = (0, 0);
    for case in 0..cases {
        let src = source(&mut g);
        let what = format!("durum {case} {src:?}");
        let (new, old) = match (compile(&src), reference::compile(&src)) {
            (Err(a), Err(b)) => {
                assert_eq!((a.message, a.at), (b.message, b.at), "{what}");
                errors += 1;
                continue;
            }
            (Ok(a), Ok(b)) => (a, b),
            (a, b) => panic!("{what}: {:?} ≠ {:?}", a.err(), b.err()),
        };
        assert_eq!(new.fields, old.fields, "{what}");
        let n = g.int(1, 6) as usize;
        let objects: Vec<Object> = (0..n).map(|i| object(&mut g, i + 1)).collect();
        let (texts, lens, numbers, measures) = table(&new.fields, new.needs, &objects);
        let scale = if g.chance(0.3) { 500.0 } else { f64::NAN };
        let input = || RowsInput {
            n,
            texts: &texts,
            text_lens: &lens,
            numbers: &numbers,
            measures: &measures,
            scale,
        };
        assert_eq!(new.needs, old.needs, "{what}");
        for want in MODES {
            let a = evaluate_rows(&new, &input(), want).unwrap_or_else(|m| panic!("{what}: {m}"));
            let b = reference::evaluate_rows(&old, &input(), want)
                .unwrap_or_else(|m| panic!("{what}: {m}"));
            if let Some(d) = differs(&a, &b) {
                panic!("{what} [{want:?}]: {d}");
            }
        }
        // One object at a time, through its scope (each engine's own table).
        let rows = Table::new(input(), Layout::new(new.fields.len(), new.needs))
            .unwrap_or_else(|m| panic!("{what}: {m}"));
        let old_rows = reference::OldTable::new(
            input(),
            reference::OldLayout::new(old.fields.len(), old.needs),
        )
        .unwrap_or_else(|m| panic!("{what}: {m}"));
        for i in 0..n {
            let (row, old_row) = (rows.row(i, None), old_rows.row(i, None));
            let (a, b) = (new.evaluate(&row), old.evaluate(&old_row));
            assert!(same(&a, &b), "{what} nesne {}: {a:?} ≠ {b:?}", i + 1);
        }
        values += n;
    }
    assert!(
        values > cases && errors > cases / 20,
        "{values} değer, {errors} hata"
    );
}
