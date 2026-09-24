//! The TypeScript's `expression.test.ts`, on the core.

use std::collections::BTreeMap;

use super::rows::{As, BOOL, Column, EMPTY, NUMBER, RowsInput, TEXT, evaluate_rows};
use super::*;

struct Object {
    attrs: BTreeMap<&'static str, &'static str>,
    measured: Measured,
    vertices: Option<f64>,
    kind: &'static str,
    layer: &'static str,
    label: Option<&'static str>,
    id: f64,
}

struct Scoped<'a> {
    o: &'a Object,
    fields: &'a [String],
    index: f64,
}

impl Scope for Scoped<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        self.o.attrs.get(self.fields[i].as_str()).copied()
    }
    fn measured(&self) -> Measured {
        self.o.measured
    }
    fn vertices(&self) -> Option<f64> {
        self.o.vertices
    }
    fn kind(&self) -> &str {
        self.o.kind
    }
    fn layer(&self) -> &str {
        self.o.layer
    }
    fn label(&self) -> Option<&str> {
        self.o.label
    }
    fn index(&self) -> f64 {
        self.index
    }
    fn id(&self) -> f64 {
        self.o.id
    }
    fn scale(&self) -> Option<f64> {
        None
    }
}

fn parcel() -> Object {
    Object {
        attrs: BTreeMap::from([
            ("Parsel", "12"),
            ("Ada", "1245"),
            ("Nitelik", "Arsa"),
            ("Tapu alanı", "598.50"),
            ("Boş", ""),
        ]),
        measured: Measured {
            length: Some(100.0),
            area: Some(600.0),
            anchor: Some((10.0, 15.0)),
        },
        vertices: Some(4.0),
        kind: "Kapalı alan",
        layer: "Parsel sınırı",
        label: Some("12"),
        id: 7.0,
    }
}

fn line() -> Object {
    Object {
        attrs: BTreeMap::new(),
        measured: Measured {
            length: Some(5.0),
            area: None,
            anchor: Some((0.0, 0.0)),
        },
        vertices: Some(2.0),
        kind: "Çizgi",
        layer: "Yol ekseni",
        label: None,
        id: 8.0,
    }
}

fn run_on(src: &str, o: &Object, index: f64) -> Value<'static> {
    let e = compile(src).unwrap_or_else(|e| panic!("{src}: {}", e.text()));
    e.evaluate(&Scoped {
        o,
        fields: &e.fields,
        index,
    })
    .into_owned()
}

fn run(src: &str) -> Value<'static> {
    run_on(src, &parcel(), 1.0)
}

fn error(src: &str) -> Option<String> {
    compile(src).err().map(|e| e.text())
}

fn num(x: f64) -> Value<'static> {
    Value::Num(x)
}

fn text(s: &str) -> Value<'_> {
    Value::text(s)
}

#[test]
fn reads_fields_bracketed_names_and_geometry_variables() {
    assert_eq!(run("Parsel"), text("12"));
    assert_eq!(run("[Tapu alanı]"), text("598.50"));
    assert_eq!(run("$alan"), num(600.0));
    assert_eq!(run("$uzunluk"), num(100.0));
    assert_eq!(run("$çevre"), num(100.0));
    assert_eq!(run("$CEVRE"), num(100.0));
    assert_eq!(run("$köşe"), num(4.0));
    assert_eq!(run("$tür"), text("Kapalı alan"));
    assert_eq!(run("$katman"), text("Parsel sınırı"));
    assert_eq!(run("$etiket"), text("12"));
    assert_eq!(run_on("$sıra", &parcel(), 5.0), num(5.0));
    assert_eq!(run_on("$uzunluk", &line(), 1.0), num(5.0));
    assert_eq!(run_on("$alan", &line(), 1.0), Value::Null);
    assert_eq!(run("Yok"), Value::Null);
}

#[test]
fn does_arithmetic_on_text_numbers_and_joins_text() {
    let Value::Num(d) = run("[Tapu alanı] - $alan") else {
        panic!()
    };
    assert!((d + 1.5).abs() < 1e-12);
    assert_eq!(run("Parsel + 1"), num(13.0));
    assert_eq!(run("Ada || \"/\" || Parsel"), text("1245/12"));
    assert_eq!(run("Nitelik + \" parsel\""), text("Arsa parsel"));
    assert_eq!(run("2 + 3 * 4 - (1 + 1) / 2"), num(13.0));
    assert_eq!(run("-$alan"), num(-600.0));
    assert_eq!(run("7 % 4"), num(3.0));
    assert_eq!(run("1 / 0"), Value::Null);
    assert_eq!(run("Nitelik * 2"), Value::Null);
    assert_eq!(run("Yok + 1"), Value::Null);
    assert_eq!(run("0.1 + 0.2 || ''"), text("0.3"));
}

#[test]
fn compares_and_combines_conditions_in_turkish_and_english() {
    let yes = Value::Bool(true);
    let no = Value::Bool(false);
    assert_eq!(run("Nitelik = 'Arsa' ve $alan > 500"), yes);
    assert_eq!(run("Nitelik = 'arsa'"), no);
    assert_eq!(run("Nitelik = 'Tarla' veya Parsel = 12"), yes);
    assert_eq!(run("değil $alan > 500"), no);
    assert_eq!(run("not ($alan < 500) and true"), yes);
    assert_eq!(run("Parsel = 12.0"), yes);
    assert_eq!(run("Parsel <> 12"), no);
    assert_eq!(run("Nitelik < 'Bahçe'"), yes);
    assert_eq!(run("Yok = boş"), yes);
    assert_eq!(run("[Boş] = boş"), yes);
    assert_eq!(run("Parsel = boş"), no);
    assert_eq!(run("Yok > 5"), no);
    assert_eq!(run("doğru ve yanlış"), no);
}

#[test]
fn calls_functions_by_turkish_or_english_name() {
    assert_eq!(run("yuvarla(12.345, 2)"), num(12.35));
    assert_eq!(run("YUVARLA(1.005, 2)"), num(1.01));
    assert_eq!(run("round($alan / 7)"), num(86.0));
    assert_eq!(run("metin(452.1, 2)"), text("452.10"));
    assert_eq!(
        run_on("'P' || doldur($sıra, 5)", &parcel(), 12.0),
        text("P00012")
    );
    assert_eq!(run("doldur(Parsel, 4, '_')"), text("__12"));
    assert_eq!(run("parça(\"P00012\", 2, 3)"), text("000"));
    assert_eq!(run("büyük(\"kadıköy\")"), text("KADIKÖY"));
    assert_eq!(run("küçük(\"IŞIK\")"), text("ışık"));
    assert_eq!(run("içerir(Nitelik, 'ARS')"), Value::Bool(true));
    assert_eq!(run("başlar('Çınar', 'cin')"), Value::Bool(true));
    assert_eq!(run("eğer($alan > 1000, 'büyük', 'küçük')"), text("küçük"));
    assert_eq!(run("boş(Yok)"), Value::Bool(true));
    assert_eq!(run("varsayılan(Yok, [Boş], 'yok')"), text("yok"));
    assert_eq!(run("max(1, Parsel, 3)"), num(12.0));
    assert_eq!(run("tamsayı(-2.7)"), num(-2.0));
    assert_eq!(run("değiştir('1245/12', '/', '-')"), text("1245-12"));
    assert_eq!(run("sayı('abc')"), Value::Null);
}

#[test]
fn explains_mistakes_with_a_position() {
    let has = |src: &str, part: &str| {
        let e = error(src).unwrap_or_default();
        assert!(e.contains(part), "{src}: {e}");
    };
    assert_eq!(error("").as_deref(), Some("İfade boş."));
    assert_eq!(
        error("Nitelik = 'Arsa").as_deref(),
        Some("11. karakterde: Tırnak kapanmamış.")
    );
    has("(1 + 2", "Parantez kapanmamış");
    has("1 2", "3. karakterde: Beklenmeyen “2”");
    assert_eq!(error("foo(1)").as_deref(), Some("Bilinmeyen işlev: foo()."));
    has("yuvarla()", "1 ya da 2 değer alır; 0 verildi");
    assert_eq!(
        error("$nope").as_deref(),
        Some("Bilinmeyen değişken: $nope.")
    );
    has("1 +", "yarım kalmış");
    has("ve 1", "burada kullanılamaz");
    has("[Tapu", "“]” bekleniyordu");
    has("1 # 2", "Anlaşılmayan karakter: “#”");
    has("1.50 x", "Beklenmeyen “x”");
    has("1 1.50", "Beklenmeyen “1.5”");
    has("min()", "en az 1 değer alır");
    // Positions are in UTF-16 code units, as the dialog counts.
    has("'😀' 2", "6. karakterde");
}

#[test]
fn lists_the_fields_it_reads() {
    let e = compile("Nitelik = 'Arsa' ve [Tapu alanı] > 0 ve Kat > 1 ve Nitelik != ''")
        .unwrap_or_else(|e| panic!("{}", e.text()));
    assert_eq!(e.fields, ["Nitelik", "Tapu alanı", "Kat"]);
    assert_eq!(e.needs, Needs::default());
    let e = compile("$alan + $köşe || $katman || $etiket || $tür || $id || $sıra || $ölçek")
        .unwrap_or_else(|e| panic!("{}", e.text()));
    assert!(
        e.needs.measured
            && e.needs.vertices
            && e.needs.layer
            && e.needs.label
            && e.needs.kind
            && e.needs.id
            && e.needs.index
            && e.needs.scale
    );
}

#[test]
fn evaluates_a_table_of_objects_in_one_call() {
    let e = compile("[Tapu alanı] - $alan || ' ' || $etiket || $katman")
        .unwrap_or_else(|e| panic!("{}", e.text()));
    // Two objects: text slots Tapu alanı, label, layer; measures six each.
    let texts = "598.50".to_string() + "12" + "Parsel sınırı" + "Yol ekseni";
    let input = RowsInput {
        n: 2,
        texts: &texts,
        text_lens: &[6, 2, 13, -1, -1, 10],
        numbers: &[],
        measures: &[2.0, 0.0, 600.0, 0.0, 0.0, 0.0, 1.0, 5.0, 0.0, 0.0, 0.0, 0.0],
        scale: f64::NAN,
    };
    let c = evaluate_rows(&e, &input, As::Value).unwrap_or_else(|m| panic!("{m}"));
    assert_eq!(c.kinds, [TEXT, TEXT]);
    assert_eq!(c.texts, "-1.5 12Parsel sınırı Yol ekseni");
    assert_eq!(c.text_lens, [20, 11]);
    let e = compile("$sıra * 2 > 2").unwrap_or_else(|e| panic!("{}", e.text()));
    let c = evaluate_rows(
        &e,
        &RowsInput {
            n: 3,
            texts: "",
            text_lens: &[],
            numbers: &[],
            measures: &[],
            scale: f64::NAN,
        },
        As::Value,
    )
    .unwrap_or_else(|m| panic!("{m}"));
    assert_eq!(
        c,
        Column {
            kinds: vec![BOOL, BOOL, BOOL],
            numbers: vec![0.0, 1.0, 1.0],
            texts: String::new(),
            text_lens: vec![],
        }
    );
    let e = compile("$id + 0.5").unwrap_or_else(|e| panic!("{}", e.text()));
    let c = evaluate_rows(
        &e,
        &RowsInput {
            n: 1,
            texts: "",
            text_lens: &[],
            numbers: &[41.0],
            measures: &[],
            scale: f64::NAN,
        },
        As::Value,
    )
    .unwrap_or_else(|m| panic!("{m}"));
    assert_eq!((c.kinds[0], c.numbers[0]), (NUMBER, 41.5));
    // A table that does not fit the expression is refused.
    assert!(
        evaluate_rows(
            &e,
            &RowsInput {
                n: 2,
                texts: "",
                text_lens: &[],
                numbers: &[1.0],
                measures: &[],
                scale: f64::NAN,
            },
            As::Value,
        )
        .is_err()
    );
    // Each value as the caller wants it; "empty" stays empty for text and true/false.
    let e = compile("eğer($sıra = 1, 'a', eğer($sıra = 2, boş, '12'))")
        .unwrap_or_else(|e| panic!("{}", e.text()));
    let three = RowsInput {
        n: 3,
        texts: "",
        text_lens: &[],
        numbers: &[],
        measures: &[],
        scale: f64::NAN,
    };
    let kinds = |want| {
        evaluate_rows(&e, &three, want)
            .map(|c| c.kinds)
            .unwrap_or_default()
    };
    assert_eq!(kinds(As::Number), [EMPTY, EMPTY, NUMBER]);
    assert_eq!(kinds(As::Text), [TEXT, EMPTY, TEXT]);
    assert_eq!(kinds(As::Bool), [BOOL, EMPTY, BOOL]);
    // The number a value's text reads as: 12 significant digits.
    let e = compile("2 / 3").unwrap_or_else(|e| panic!("{}", e.text()));
    let one = RowsInput { n: 1, ..three };
    let c = evaluate_rows(&e, &one, As::TextNumber).unwrap_or_default();
    assert_eq!(c.numbers, [0.666666666667]);
}
