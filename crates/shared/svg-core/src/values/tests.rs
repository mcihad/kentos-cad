use super::css::strip_comments;
use super::*;
use kentos_geometry_core::api::json::{FromJson, Json};

fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[test]
fn numbers_read_as_javascript_reads_them() {
    for (text, want) in [
        ("0x1F", 31.0),
        ("0b101", 5.0),
        ("-0x1F", f64::NAN),
        ("+0x1F", f64::NAN),
        (" 12 ", 12.0),
        ("", 0.0),
        ("1e3", 1000.0),
        ("-Infinity", f64::NEG_INFINITY),
        ("infinity", f64::NAN),
        ("1_000", f64::NAN),
        ("0x", f64::NAN),
    ] {
        assert!(
            same(js_number(text), want),
            "Number({text:?}) = {}",
            js_number(text)
        );
    }
    for (text, want) in [
        ("3.5px", 3.5),
        ("-.5e2x", -50.0),
        ("1e", 1.0),
        ("x", f64::NAN),
        ("  -Infinityx", f64::NEG_INFINITY),
        ("0x10", 0.0),
    ] {
        assert!(
            same(parse_float(text), want),
            "parseFloat({text:?}) = {}",
            parse_float(text)
        );
    }
    for (text, want) in [
        ("ff", 255.0),
        ("-1", -1.0),
        ("0x1f", 31.0),
        ("0x", f64::NAN),
        ("g", f64::NAN),
        (" 7g", 7.0),
    ] {
        assert!(
            same(parse_hex(text), want),
            "parseInt({text:?}, 16) = {}",
            parse_hex(text)
        );
    }
}

#[test]
fn near_black_slices_code_units_and_reads_signs() {
    assert!(is_near_black("#1D1D1B"));
    assert!(!is_near_black("#313131"));
    // parseInt reads "-1" as −1: the TypeScript took this for black too.
    assert!(is_near_black("#-1-1-1"));
    assert!(!is_near_black("#é10000"));
    assert!(!is_near_black("fill"));
}

#[test]
fn colours_and_paints() {
    assert_eq!(
        read_color("#abc"),
        Some(Color {
            hex: "#AABBCC".into(),
            alpha: 1.0
        })
    );
    assert_eq!(
        read_color("rgb(10% 20% 30% / 0.5)"),
        Some(Color {
            hex: "#1A334D".into(),
            alpha: 0.5
        })
    );
    assert_eq!(
        read_color("hsl(120, 100%, 25%)").map(|c| c.hex),
        Some("#008000".into())
    );
    assert_eq!(read_color("url(#a)"), None);
    // A reference chain: each reference's fallback is the next, then the plain paint.
    let red = Flat::Color(Color {
        hex: "#FF0000".into(),
        alpha: 1.0,
    });
    assert_eq!(
        raw_paint(Some(" url(#a) url('#b')  red ")),
        Some(RawPaint::Url {
            ids: vec!["a".into(), "b".into()],
            fallback: Some(red)
        })
    );
    assert_eq!(
        raw_paint(Some("url(#a) inherit")),
        Some(RawPaint::Url {
            ids: vec!["a".into()],
            fallback: None
        })
    );
    assert_eq!(
        raw_paint(Some("url(foo)")),
        Some(RawPaint::Plain(Flat::None))
    );
    // A tail over two lines does not match `.*$`: the whole value is an unreadable url().
    assert_eq!(
        raw_paint(Some("url(#a) red\nblue")),
        Some(RawPaint::Plain(Flat::None))
    );
    // A non-ASCII character where "url(" would end.
    assert_eq!(raw_paint(Some("ur€(#a)")), None);
    assert_eq!(read_paint(Some("#000")), Some("fill".into()));
    assert_eq!(
        read_paint(Some("rgba(255, 0, 0, 0.5)")),
        Some("#FF000080".into())
    );
}

#[test]
fn style_sheets() {
    // `<!--|-->` is removed in one pass: what a removal joins is not looked at again.
    assert_eq!(strip_comments("-<!--->"), "-->");
    assert_eq!(strip_comments("a /* b */ c /* open"), "a  c /* open");
    let rules = parse_css(
        "#x.k rect > .a, g { fill: red !important; stroke: blue } @media print { .a { fill: none } } a:hover { x: y }",
        3.0,
    );
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].order, 3.0);
    assert_eq!(rules[0].selectors.len(), 2);
    assert_eq!(rules[0].selectors[0].spec, 10000.0 + 100.0 + 1.0 + 100.0);
    assert_eq!(
        rules[0].decls[0],
        Decl {
            prop: "fill".into(),
            value: "red".into(),
            important: true
        }
    );
    // The compound keeps the order its tag and id were named in (the TypeScript object's keys).
    let json = kentos_geometry_core::api::json::to_string(
        &parse_css("#i.c[a] {}", 0.0)[0].selectors[0].parts[0].0,
    );
    assert_eq!(json, r#"{"classes":["c"],"attrs":[{"name":"a"}],"id":"i"}"#);
    let json = kentos_geometry_core::api::json::to_string(
        &parse_css("rect#i {}", 0.0)[0].selectors[0].parts[0].0,
    );
    assert_eq!(json, r#"{"classes":[],"attrs":[],"tag":"rect","id":"i"}"#);
}

#[test]
fn a_file_crosses_flat_and_its_parents_come_first() {
    let tree = XmlTree::from_json(&Json::parse(r##"[["svg",{},null,-1],["g",{"id":"a"},null,0],["#text",{},"x",1],["rect",{},null,0]]"##).unwrap()).unwrap();
    assert_eq!(tree.nodes[0].children, vec![1, 3]);
    assert_eq!(tree.nodes[1].children, vec![2]);
    assert_eq!(tree.nodes[2].text.as_deref(), Some("x"));
    // A parent after its child (or none but for the root) is refused.
    assert!(
        XmlTree::from_json(
            &Json::parse(r#"[["svg",{},null,-1],["g",{},null,2],["g",{},null,1]]"#).unwrap()
        )
        .is_err()
    );
    assert!(
        XmlTree::from_json(&Json::parse(r#"[["svg",{},null,-1],["g",{},null,-1]]"#).unwrap())
            .is_err()
    );
    assert!(XmlTree::from_json(&Json::parse("[]").unwrap()).is_err());
}
