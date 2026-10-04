//! Metin dosyası yerleştir (docs/adr/0145 §6): the file's lines against the
//! shared cases (fixtures/text/v1/file.json, written from the rule by
//! scripts/fixtures/text_cases.py; the web's are
//! apps/web/src/model/textFile.test.ts), and the tool over the session: the
//! file asked for as it starts, the lines written 1.5 heights apart in one
//! step, a cancelled picker or a refused file leaving. Expected places are
//! worked out by hand.

use crate::common;

use common::{Bench, N, rel};
use kentos_contracts::Entity;
use kentos_geometry_core::api::json::Json;
use kentos_interaction::ViewChange;
use kentos_interaction::text_file::{MAX_BYTES, Refused, lines};

const CASES: &str = include_str!("../../../../../fixtures/text/v1/file.json");

fn text(v: &Json) -> Option<&str> {
    match v {
        Json::Str(s) => Some(s),
        _ => None,
    }
}

fn bytes_of(c: &Json) -> Vec<u8> {
    match text(c.get("hex")) {
        Some(hex) => (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
            .collect(),
        None => text(c.get("text")).unwrap_or_default().as_bytes().to_vec(),
    }
}

#[test]
fn the_files_lines_are_read_as_the_rule_says() {
    let file = Json::parse(CASES).expect("JSON");
    let Json::Arr(cases) = file.get("cases") else {
        panic!("cases");
    };
    assert!(cases.len() >= 10);
    for c in cases {
        let name = text(c.get("name")).unwrap_or_default();
        let got = lines("satirlar.txt", &bytes_of(c));
        let want = c.get("expect");
        if let Some(kind) = text(want.get("error")) {
            let kind_of = |k: Refused| match k {
                Refused::TooBig => "tooBig",
                Refused::NotUtf8 => "notUtf8",
                Refused::TooMany => "tooMany",
                Refused::Empty => "empty",
            };
            assert_eq!(got.map_err(|e| kind_of(e.kind)).err(), Some(kind), "{name}");
            continue;
        }
        let got = got.unwrap_or_else(|e| panic!("{name}: {}", e.message));
        match want.get("lines") {
            Json::Arr(expected) => {
                let expected: Vec<&str> = expected.iter().filter_map(text).collect();
                assert_eq!(got, expected, "{name}");
            }
            _ => {
                let count = match want.get("count") {
                    Json::Num(n) => *n as usize,
                    _ => 0,
                };
                assert_eq!(
                    (
                        got.len(),
                        got.first().map(String::as_str),
                        got.last().map(String::as_str)
                    ),
                    (count, text(want.get("first")), text(want.get("last"))),
                    "{name}"
                );
            }
        }
    }
}

#[test]
fn more_than_one_megabyte_is_refused_with_its_size() {
    let big = vec![b'a'; MAX_BYTES + 1];
    let e = lines("büyük.txt", &big).expect_err("refused");
    assert_eq!(e.kind, Refused::TooBig);
    assert_eq!(
        e.message,
        "“büyük.txt” 1 MB'tan büyük (1.0 MB); en çok 1 MB okunur. Dosyayı bölüp yeniden deneyin."
    );
    assert!(lines("tam.txt", &vec![b'a'; MAX_BYTES]).is_ok());
}

#[test]
fn the_file_is_asked_for_and_its_lines_are_written_one_under_the_other() {
    let mut b = Bench::new("placeTextFile");
    assert!(
        b.views.contains(&ViewChange::OpenTextFile),
        "the file asked for: {:?}",
        b.views
    );
    assert_eq!(
        b.session.prompt().text(),
        "Metin dosyası yerleştir: metin dosyasını seçin"
    );
    let count = b.doc.entities().count();
    b.run(|s, cx| {
        s.file_given(
            Some(("satirlar.txt", "Ada 101\n\nAda 103\n".as_bytes())),
            cx,
        )
    });
    assert_eq!(
        b.session.prompt().text(),
        "Metin dosyası yerleştir: ilk satırın başlangıcına tıklayın [“satirlar.txt”: 2 yazı; Yazı'nın seçenekleriyle: 2.5 mm, 0°, sol taban]"
    );
    // The texts' boxes follow the pointer, the empty line's place left
    // empty: sol taban, the first stands on the pointer, the other 7.5 m down.
    b.move_to(2.0, 4.0);
    let boxes = b
        .session
        .preview(&Default::default())
        .expect("a preview")
        .strokes;
    let lowest = |i: usize| {
        boxes[i]
            .pts
            .iter()
            .map(|p| p.y - N)
            .fold(f64::INFINITY, f64::min)
    };
    assert_eq!(boxes.len(), 2, "{boxes:?}");
    assert!((lowest(0) - 4.0).abs() < 1e-9, "{boxes:?}");
    assert!((lowest(1) + 3.5).abs() < 1e-9, "{boxes:?}");
    b.click(2.0, 4.0);
    // 2.5 mm at 1:1000 is 2.5 m; the third line is 2 × 1.5 × 2.5 = 7.5 m down.
    let texts: Vec<(String, [f64; 2])> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Text(t) => Some((t.text.clone(), rel(t.p))),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        [
            ("Ada 101".to_owned(), [2.0, 4.0]),
            ("Ada 103".to_owned(), [2.0, -3.5])
        ]
    );
    assert_eq!(b.doc.entities().count(), count + 2);
    assert_eq!(
        b.last_text(),
        Some("“satirlar.txt”: 2 satır yazı olarak yerleştirildi.")
    );
    assert!(!b.session.is_running(), "the tool leaves");
    assert_eq!(b.doc.undo().as_deref(), Some("Metin dosyası yerleştir"));
}

#[test]
fn a_cancelled_picker_or_a_refused_file_leaves_without_writing() {
    let mut b = Bench::new("placeTextFile");
    b.run(|s, cx| s.file_given(None, cx));
    assert!(!b.session.is_running());
    let mut b = Bench::new("placeTextFile");
    b.run(|s, cx| s.file_given(Some(("a.txt", &[0xfe, 0x41][..])), cx));
    assert!(!b.session.is_running());
    assert_eq!(
        b.last_text(),
        Some("“a.txt” UTF-8 değil. Dosyayı UTF-8 olarak kaydedip yeniden deneyin.")
    );
}
