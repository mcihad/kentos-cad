//! The parcel's zoning (PAİY 4/1-tt, 4/1-cccc, 5/5, 5/6, 5/8): the floors' area and the footprint the facts carry
//! against the emsal and TAKS of the project's İmar durumu, run as a cloud job runs the rule.

use super::*;

const RULE: &str = "imar-durumu";

/// The sample building with an İmar durumu: a 300 m² parcel, `far`, `coverage`, the floors' area and the footprint.
fn zoned(far: Option<f64>, coverage: Option<f64>, floors: f64, footprint: f64) -> Value {
    facts(move |a| {
        a["zoning"] = json!({
            "parcel": 300.0, "far": far, "coverage": coverage, "floors": floors, "included": floors,
            "exempt": 0.0, "outside": 0.0, "cap": far.map(|f| 0.3 * f * 300.0), "over": 0.0, "footprint": footprint
        });
    })
}

#[test]
fn the_emsal_and_taks_are_read_against_the_zoning() {
    let python = need_python!();
    let found = findings(&python, RULE, zoned(Some(1.5), Some(0.4), 486.0, 130.0));
    assert_eq!(
        said(&found),
        [(Severity::Error, None), (Severity::Error, None)],
        "{found:?}"
    );
    assert!(
        says(&found[0], &["Emsal 1.62 > 1.50", "PAİY md. 4/1-tt ve 5/5"]),
        "{}",
        found[0].message
    );
    assert!(
        says(&found[1], &["TAKS 0.43 > 0.40", "PAİY md. 4/1-cccc ve 5/5"]),
        "{}",
        found[1].message
    );
    assert!(findings(&python, RULE, zoned(Some(1.5), Some(0.4), 400.0, 110.0)).is_empty());
    // Where İmar durumu's emsal and TAKS are read from is said with the article (docs/adr/8140); none said, the words
    // are as above.
    let mut sourced = zoned(Some(1.5), Some(0.4), 486.0, 130.0);
    sourced["project"]["architecture"]["zoning"]["farSource"] = json!("UIP-1 Merkez uygulama imar planı, karar K1");
    sourced["project"]["architecture"]["zoning"]["coverageSource"] = json!("İmar durum belgesi 12.3.2025/1234");
    let found = findings(&python, RULE, sourced);
    assert!(
        says(&found[0], &["PAİY md. 4/1-tt ve 5/5; imar durumu (UIP-1 Merkez uygulama imar planı, karar K1)."]),
        "{}",
        found[0].message
    );
    assert!(
        says(&found[1], &["PAİY md. 4/1-cccc ve 5/5; imar durumu (İmar durum belgesi 12.3.2025/1234)."]),
        "{}",
        found[1].message
    );
    // No TAKS given: 5/6's 60 % (an error), 40 % for a detached or block layout (a warning, the layout is not known).
    let found = findings(&python, RULE, zoned(Some(1.5), None, 400.0, 190.0));
    assert_eq!(said(&found), [(Severity::Error, None)]);
    assert!(
        says(&found[0], &["%60'ı geçemez", "PAİY md. 5/6"]),
        "{}",
        found[0].message
    );
    let found = findings(&python, RULE, zoned(Some(1.5), None, 400.0, 150.0));
    assert_eq!(said(&found), [(Severity::Warning, None)]);
    // The building's layout said (docs/adr/8141): ayrık, blok or ikiz binds the 40 %, bitişik the 60 %.
    let laid = |layout: &str| {
        let mut f = zoned(Some(1.5), None, 400.0, 150.0);
        f["project"]["architecture"]["zoning"]["layout"] = json!(layout);
        f
    };
    for layout in ["detached", "block", "twin"] {
        let found = findings(&python, RULE, laid(layout));
        assert_eq!(said(&found), [(Severity::Error, None)], "{layout}");
        assert!(says(&found[0], &["5/6'ya göre %40'ı geçemez"]), "{}", found[0].message);
    }
    assert!(findings(&python, RULE, laid("attached")).is_empty());
    // The sample building as drawn writes no İmar durumu: nothing said.
    assert!(findings(&python, RULE, facts(|_| {})).is_empty());
}
