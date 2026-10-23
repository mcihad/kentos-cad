//! Independent drawing-set cases: atomic refusal, inverses, standard fields and live index order.
use kentos_sheet::{
    drawing_set::{DrawingIndex, rows},
    SheetBook,
    ops::{Op, apply_all},
    validate::read_book,
};
use serde_json::Value;
#[test]
fn independent_drawing_set_cases_keep_metadata_indices_and_exact_inverse() {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/drawing-set.json"
    ))
    .unwrap();
    assert!(fx["cases"].as_array().unwrap().len() >= 20);
    let views: Vec<kentos_contracts::NamedView> =
        serde_json::from_value(fx["namedViews"].clone()).unwrap();
    for case in fx["cases"].as_array().unwrap() {
        let before = read_book(&case["book"].to_string()).unwrap();
        let ops: Vec<Op> = serde_json::from_value(case["ops"].clone()).unwrap();
        let answer = apply_all(&before, &ops);
        if let Some(code) = case["code"].as_str() {
            assert_eq!(answer.unwrap_err().code, code, "{}", case["name"]);
            continue;
        }
        let after = answer.unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
        let index: DrawingIndex = serde_json::from_value(case["index"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(rows(&after.book, &index, &views)).unwrap(),
            case["rows"],
            "{}",
            case["name"]
        );
        let metadata = after
            .book
            .sheets
            .iter()
            .map(|s| (s.id.clone(), serde_json::to_value(&s.drawing_info).unwrap()))
            .collect::<serde_json::Map<_, _>>();
        assert_eq!(
            Value::Object(metadata),
            case["metadata"],
            "{}",
            case["name"]
        );
        assert_eq!(
            apply_all(&after.book, &after.inverse).unwrap().book,
            before,
            "{} inverse",
            case["name"]
        );
        let reopened: SheetBook = read_book(&serde_json::to_string(&after.book).unwrap()).unwrap();
        assert_eq!(reopened, after.book);
    }
}
