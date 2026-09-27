//! The column engine against the tree evaluator it replaced (docs/adr/0100):
//! random sources, valid and broken, over random objects, in every result
//! mode. The two must agree on everything the language shows: the error and
//! its position, the fields read, each value's kind, its number to the bit,
//! its text and the text's UTF-16 length; one object at a time
//! (`Expr::evaluate`) as well as a table at a time (`rows::evaluate_rows`).
//! The language since docs/adr/0100 §4 (durum, içinde, arasında, gibi,
//! benzer, boş as a test, `^`, the new functions) the reference never had:
//! there the column engine is held to one object at a time, its own tree
//! walker, and `fixtures/expression/v2/language.json` holds the values.
//! The generator is the web's (`apps/web/src/model/expression/cases.ts`),
//! with text outside the Basic Multilingual Plane (emoji) and Greek sigmas
//! added, and numbers that are not finite among the geometry values.
//!
//!   EXPRESSION_DIFF_CASES=200000 cargo test --release -p kentos-expression --test differential

mod reference;

use kentos_expression::rows::{
    As, Column, Layout, MEASURE_STRIDE, RowsInput, Table, evaluate_rows, put,
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

/// Patterns for `gibi` and `benzer`: wildcards, escapes, Turkish letters, emoji.
const PATTERNS: [&str; 16] = [
    "'A%'", "'%a%'", "'_r%'", "'%'", "''", "'\\%'", "'%\\_%'", "'Ç%'", "'ç_n%'", "'%sa'", "'__'",
    "'a😀%'", "'_'", "'İ%'", "'%ı%'", "'1%'",
];
/// The functions since docs/adr/0100 §4, with their arities.
const FRESH_FUNCTIONS: [(&str, i64, i64); 17] = [
    ("kök", 1, 1),
    ("sqrt", 1, 1),
    ("tavan", 1, 1),
    ("ceil", 1, 1),
    ("taban", 1, 1),
    ("floor", 1, 1),
    ("pi", 0, 0),
    ("sol", 2, 2),
    ("left", 2, 2),
    ("sağ", 2, 2),
    ("right", 2, 2),
    ("bul", 2, 2),
    ("strpos", 2, 2),
    ("birleştir", 1, 5),
    ("concat", 1, 3),
    ("sağdoldur", 2, 3),
    ("rpad", 2, 3),
];

/// `değil ` before içinde, arasında, gibi and benzer, now and then.
fn not(g: &mut Gen) -> &'static str {
    if g.chance(0.3) {
        g.pick(&["değil ", "not ", "NOT "])
    } else {
        ""
    }
}

/// A piece of the language since docs/adr/0100 §4, around random operands.
fn fresh(g: &mut Gen, depth: u32) -> String {
    let d = depth + 1;
    match g.int(0, 9) {
        0 => {
            let mut s = g.pick(&["durum", "DURUM", "case"]).to_string();
            for _ in 0..g.int(1, 3) {
                let when = *g.pick(&["eğer", "when", "EĞER"]);
                let condition = expr(g, d, true);
                let then = *g.pick(&["ise", "then", "İSE"]);
                let value = expr(g, d, true);
                s += &format!(" {when} {condition} {then} {value}");
            }
            if g.chance(0.6) {
                let otherwise = *g.pick(&["yoksa", "else"]);
                s += &format!(" {otherwise} {}", expr(g, d, true));
            }
            format!("{s} {}", g.pick(&["son", "end", "SON"]))
        }
        1 => {
            let a = atom(g, d, true);
            let not = not(g);
            let word = *g.pick(&["içinde", "in", "İÇİNDE"]);
            let items: Vec<String> = (0..g.int(1, 4)).map(|_| expr(g, d, true)).collect();
            format!("{a} {not}{word} ({})", items.join(", "))
        }
        2 => {
            let a = atom(g, d, true);
            let not = not(g);
            let word = *g.pick(&["arasında", "between"]);
            let low = atom(g, d, true);
            let and = *g.pick(&["ve", "and"]);
            format!("{a} {not}{word} {low} {and} {}", atom(g, d, true))
        }
        3 => {
            let a = atom(g, d, true);
            let not = not(g);
            let word = *g.pick(&["gibi", "like", "benzer", "ilike", "ILIKE"]);
            let pattern = if g.chance(0.7) {
                g.pick(&PATTERNS).to_string()
            } else {
                atom(g, d, true)
            };
            format!("{a} {not}{word} {pattern}")
        }
        4 => {
            let a = atom(g, d, true);
            let test = *g.pick(&["boş", "boş değil", "IS NULL", "is not null", "BOŞ DEĞİL"]);
            format!("{a} {test}")
        }
        5 | 6 => {
            let sign = *g.pick(&["", "-"]);
            let base = atom(g, d, true);
            format!("{sign}{base} ^ {}", atom(g, d, true))
        }
        _ => {
            let &(name, lo, hi) = g.pick(&FRESH_FUNCTIONS);
            let n = if g.chance(0.1) {
                *g.pick(&[lo - 1, hi + 1])
            } else {
                g.int(lo, hi)
            };
            let args: Vec<String> = (0..n.max(0)).map(|_| expr(g, d, true)).collect();
            format!("{name}({})", args.join(", "))
        }
    }
}

/// An operand; with `new`, now and then a piece of the language since
/// docs/adr/0100 §4 (near the top only, so the sources stay small).
fn atom(g: &mut Gen, depth: u32, new: bool) -> String {
    if new && depth < 6 && g.chance(0.3) {
        return fresh(g, depth);
    }
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
            let args: Vec<String> = (0..n.max(0)).map(|_| expr(g, depth + 1, new)).collect();
            let sep = *g.pick(&[", ", ","]);
            format!("{name}({})", args.join(sep))
        }
        9 => format!("({})", expr(g, depth + 1, new)),
        _ if g.chance(0.15) => {
            // Text past V8's longest string throws, and empties the whole expression.
            let inner = atom(g, depth + 1, new);
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
            atom(g, depth + 1, new)
        ),
    }
}

fn expr(g: &mut Gen, depth: u32, new: bool) -> String {
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
        return atom(g, depth, new);
    }
    let a = expr(g, depth + 1, new);
    let op = *g.pick(&BINARY);
    format!("{a}{op}{}", expr(g, depth + 1, new))
}

/// A source: an expression, now and then cut short or with a stray character
/// in it; with `new`, in the language since docs/adr/0100 §4 too.
fn source(g: &mut Gen, new: bool) -> String {
    let e = if new && g.chance(0.5) {
        fresh(g, 0)
    } else {
        expr(g, 0, new)
    };
    let mut s: Vec<char> = e.chars().collect();
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
        let src = source(&mut g, false);
        let what = format!("durum {case} {src:?}");
        let (new, old) = match (compile(&src), reference::compile(&src)) {
            // A stray quote can bare a random word: `x in (…)` is the new language's (docs/adr/0100 §4).
            (Ok(_), Err(b)) if b.message == reference::OUTSIDE => continue,
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

#[test]
fn the_new_words_give_the_same_values_by_column_and_by_object() {
    let cases: usize = std::env::var("EXPRESSION_DIFF_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000);
    let seed: u64 = std::env::var("EXPRESSION_DIFF_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x2545_f491_4f6c_dd1d);
    let mut g = Gen(seed);
    let (mut errors, mut values, mut fresh) = (0, 0, 0);
    let mut tmp = String::new();
    for case in 0..cases {
        let src = source(&mut g, true);
        let what = format!("durum {case} {src:?}");
        let Ok(e) = compile(&src) else {
            errors += 1;
            continue;
        };
        if reference::compile(&src).is_err_and(|b| b.message == reference::OUTSIDE) {
            fresh += 1;
        }
        let n = g.int(1, 6) as usize;
        let objects: Vec<Object> = (0..n).map(|i| object(&mut g, i + 1)).collect();
        let (texts, lens, numbers, measures) = table(&e.fields, e.needs, &objects);
        let scale = if g.chance(0.3) { 500.0 } else { f64::NAN };
        let input = || RowsInput {
            n,
            texts: &texts,
            text_lens: &lens,
            numbers: &numbers,
            measures: &measures,
            scale,
        };
        let rows = Table::new(input(), Layout::new(e.fields.len(), e.needs))
            .unwrap_or_else(|m| panic!("{what}: {m}"));
        // One object at a time, each value as every mode asks.
        let mut each: Vec<Column> = MODES.iter().map(|_| Column::default()).collect();
        for i in 0..n {
            let row = rows.row(i, None);
            let v = e.evaluate(&row);
            for (column, want) in each.iter_mut().zip(MODES) {
                put(column, Some(v.view()), want, &mut tmp);
            }
        }
        for (b, want) in each.iter().zip(MODES) {
            let a = evaluate_rows(&e, &input(), want).unwrap_or_else(|m| panic!("{what}: {m}"));
            if let Some(d) = differs(&a, b) {
                panic!("{what} [{want:?}]: {d}");
            }
        }
        values += n;
    }
    assert!(
        values > cases && errors > cases / 20 && fresh > cases / 5,
        "{values} değer, {errors} hata, {fresh} yeni"
    );
}
