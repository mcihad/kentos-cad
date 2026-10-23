//! A source snapshot is real: mutate the caller/file, compare exact output, and read its PDF metadata independently.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use kentos_contracts::encoding::sha256_hex;
use kentos_domain::Document;
use kentos_plot::{
    book::{book_pdf_captured, book_pdf_at},
    provenance::Capture,
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/source-snapshot.json"
    ))
    .unwrap()
}
fn source(fx: &Value, c: &Value, root: &Path) -> Document {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    let mut model = fx["model"].clone();
    if let Some(unit) = c.get("unit") {
        model["settings"]["drawingUnit"] = unit.clone();
    }
    if c["image"] == true || c["missing"] == true {
        let file = root.join("logo.png");
        if c["missing"] != true {
            fs::copy(fixtures.join(fx["imageA"].as_str().unwrap()), &file).unwrap();
        }
        let mut paths = vec![file];
        if c["aliases"] == true {
            paths.push(PathBuf::from("logo.png"));
        }
        if c["second"] == true {
            fs::create_dir_all(root.join("other")).unwrap();
            fs::copy(
                fixtures.join(fx["imageB"].as_str().unwrap()),
                root.join("other/logo.png"),
            )
            .unwrap();
            paths.push(root.join("other/logo.png"));
        }
        for (n, file) in paths.iter().enumerate() {
            model["entities"].as_array_mut().unwrap().push(json!({"kind":"image","id":20+n,"layerId":"cizim","attrs":{},"file":file,"p":{"x":2.4,"y":-0.1},"width":0.2,"height":0.2,"rotation":0}));
        }
    }
    let mut doc = Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
    let book: kentos_sheet::SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    doc
}
fn change(doc: &mut Document, edit: &str) {
    let mut v = serde_json::to_value(doc.to_snapshot_v2()).unwrap();
    match edit {
        "line" => {
            v["entities"][0]["a"]["y"] = json!(10);
            v["entities"][0]["b"]["y"] = json!(10);
        }
        "title" => {
            v["sheets"]["book"]["sheets"][0]["items"][1]["kind"]["content"] = json!("SONRADAN")
        }
        "metadata" => {
            v["sheets"]["book"]["sheets"][0]["drawingInfo"] =
                json!({"number":"YENI","title":"Yeni başlık","revision":"R99"})
        }
        "layers" => v["layers"][0]["visible"] = json!(false),
        "variables" => v["settings"]["variables"] = json!([{"name":"notum","value":"SONRADAN"}]),
        _ => v["name"] = json!("SONRADAN"),
    }
    *doc = Document::from_snapshot_v2(serde_json::from_value(v).unwrap()).unwrap();
}
// A small independent PDF Info string reader, so the native test has no mandatory platform CLI.
fn info_keywords(data: &[u8]) -> String {
    let mut at = data.windows(9).position(|w| w == b"/Keywords").unwrap() + 9;
    while data[at].is_ascii_whitespace() {
        at += 1;
    }
    let mut bytes = Vec::new();
    if data[at] == b'<' {
        at += 1;
        let end = data[at..].iter().position(|b| *b == b'>').unwrap() + at;
        let hex = data[at..end]
            .iter()
            .copied()
            .filter(|b| !b.is_ascii_whitespace())
            .collect::<Vec<_>>();
        for p in hex.chunks(2) {
            let h = |b: u8| (b as char).to_digit(16).unwrap() as u8;
            bytes.push(h(p[0]) * 16 + h(*p.get(1).unwrap_or(&b'0')));
        }
    } else {
        assert_eq!(data[at], b'(');
        at += 1;
        let mut depth = 1;
        while depth > 0 {
            let ch = data[at];
            at += 1;
            match ch {
                b'\\' => {
                    let c = data[at];
                    at += 1;
                    match c {
                        b'n' => bytes.push(b'\n'),
                        b'r' => bytes.push(b'\r'),
                        b't' => bytes.push(b'\t'),
                        b'b' => bytes.push(8),
                        b'f' => bytes.push(12),
                        b'\n' => {}
                        b'\r' => {
                            if data.get(at) == Some(&b'\n') {
                                at += 1;
                            }
                        }
                        b'0'..=b'7' => {
                            let mut v = u16::from(c - b'0');
                            for _ in 0..2 {
                                if data.get(at).is_some_and(|b| matches!(b, b'0'..=b'7')) {
                                    v = v * 8 + u16::from(data[at] - b'0');
                                    at += 1;
                                } else {
                                    break;
                                }
                            }
                            bytes.push(v as u8);
                        }
                        c => bytes.push(c),
                    }
                }
                b'(' => {
                    depth += 1;
                    bytes.push(ch);
                }
                b')' => {
                    depth -= 1;
                    if depth > 0 {
                        bytes.push(ch);
                    }
                }
                c => bytes.push(c),
            }
        }
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        String::from_utf16(
            &bytes[2..]
                .chunks_exact(2)
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )
        .unwrap()
    } else {
        String::from_utf8(bytes).unwrap()
    }
}
fn pdf_metadata(bytes: &[u8], root: &Path) -> Value {
    let parsed: Value = serde_json::from_str(&info_keywords(bytes)).unwrap();
    let path = root.join("out.pdf");
    fs::write(&path, bytes).unwrap();
    if let Ok(output) = Command::new("pdfinfo").arg(&path).output() {
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let json = text
            .lines()
            .find_map(|l| l.strip_prefix("Keywords:"))
            .unwrap()
            .trim();
        assert_eq!(
            serde_json::from_str::<Value>(json).unwrap(),
            parsed,
            "Poppler and the independent Info reader agree"
        );
    }
    parsed
}

#[test]
fn sheet_snapshot_independent_cases_keep_document_resources_context_and_real_pdf_provenance() {
    let fx = fixture();
    assert!(fx["cases"].as_array().unwrap().len() >= 20);
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    for (n, c) in fx["cases"].as_array().unwrap().iter().enumerate() {
        let root =
            std::env::temp_dir().join(format!("kentos-pdf-snapshot-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut doc = source(&fx, c, &root);
        let before = doc.to_snapshot_v2();
        let origin = root.join("source.kcad").display().to_string();
        let user = c["author"].as_str().unwrap_or("Kaynak Ekibi");
        let date = c["date"].as_str().unwrap_or("2026-10-23");
        let mut captured = Capture::new(&doc, user, date, Some(&origin)).unwrap();
        let pin = captured.pin().clone();
        if let Some(dpi) = c["dpi"].as_u64() {
            captured.dpi_override(Some(dpi as u16)).unwrap();
            assert_eq!(captured.pin(), &pin);
        }
        if let Some(guard) = c["guard"].as_str() {
            let mut asked = pin.clone();
            let mut next_user = user;
            let mut next_date = date;
            match guard {
                "author" => next_user = "Başka ekip",
                "date" => next_date = "2027-01-01",
                "image" => {
                    fs::copy(
                        fixtures.join(fx["imageB"].as_str().unwrap()),
                        root.join("logo.png"),
                    )
                    .unwrap();
                }
                "document-only" => {
                    asked.resources_sha256 = None;
                    asked.context_sha256 = None;
                    fs::copy(
                        fixtures.join(fx["imageB"].as_str().unwrap()),
                        root.join("logo.png"),
                    )
                    .unwrap();
                }
                "document" => doc.set_name("YENI"),
                _ => {}
            }
            let next = Capture::new(&doc, next_user, next_date, Some(&origin)).unwrap();
            assert_eq!(
                !next.matches(&asked),
                c["expectMismatch"] == true,
                "{}",
                c["name"]
            );
            fs::remove_dir_all(root).unwrap();
            continue;
        }
        let mut expected = None;
        if c.get("change").is_none()
            || matches!(c["change"].as_str(), Some("after" | "delete-after"))
        {
            let pdf = book_pdf_captured(&captured, &[], None, None).unwrap();
            expected = Some(pdf.bytes);
        }
        if let Some(edit) = c["edit"].as_str() {
            change(&mut doc, edit);
        }
        match c["change"].as_str() {
            Some("before" | "after" | "appear") => {
                fs::copy(
                    fixtures.join(fx["imageB"].as_str().unwrap()),
                    root.join("logo.png"),
                )
                .unwrap();
            }
            Some("delete-before" | "delete-after") => {
                fs::remove_file(root.join("logo.png")).unwrap()
            }
            _ => {}
        }
        let output = book_pdf_captured(&captured, &[], None, None);
        if c["expectError"] == true {
            assert!(output.is_err(), "{}", c["name"]);
        } else {
            let output = output.unwrap();
            if let Some(expected) = expected {
                assert_eq!(
                    sha256_hex(&output.bytes),
                    sha256_hex(&expected),
                    "{}",
                    c["name"]
                );
            }
            let meta = pdf_metadata(&output.bytes, &root);
            assert_eq!(meta["format"], "kentos.sheet-source/1");
            assert_eq!(meta["source"], serde_json::to_value(&pin).unwrap());
            assert_eq!(meta["laterEditsExcluded"], true);
            assert_eq!(meta["context"]["user"], user);
            assert_eq!(meta["context"]["date"], date);
            assert_eq!(meta["fonts"].as_array().unwrap().len(), 22);
            if let Some(sha) = c["expectImageSha"].as_str() {
                assert!(
                    meta["images"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|i| i["sha256"] == sha)
                );
            }
            if let Some(count) = c["expectImages"].as_u64() {
                assert_eq!(meta["images"].as_array().unwrap().len() as u64, count);
            }
        }
        if c.get("edit").is_none() {
            assert_eq!(doc.to_snapshot_v2(), before);
        }
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn sheet_snapshot_plain_pdf_carries_the_original_semantic_revision_without_mutating_it() {
    let fx = fixture();
    let root = std::env::temp_dir().join(format!("kentos-single-source-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let doc = source(&fx, &json!({}), &root);
    let before = doc.to_snapshot_v2();
    let pdf = book_pdf_at(&doc, &[], "Test", "2026-10-23", None).unwrap();
    let meta = pdf_metadata(&pdf.bytes, &root);
    assert_eq!(meta["source"]["revision"], doc.revision().to_string());
    assert_eq!(doc.to_snapshot_v2(), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn sheet_snapshot_png_svg_and_pdf_carry_the_same_original_source_and_physical_size() {
    let fx = fixture();
    let root = std::env::temp_dir().join(format!("kentos-snapshot-formats-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let doc = source(&fx, &json!({}), &root);
    let captured = Capture::new(&doc, "Saha <A&B>", "2026-10-23", None).unwrap();
    let image = kentos_plot::preview::page_captured(&captured, "ground", 96).unwrap();
    let xml = roxmltree::Document::parse(&image.svg).unwrap();
    let metadata = xml
        .descendants()
        .find(|n| n.attribute("id") == Some("kentos-source"))
        .unwrap();
    let from_svg: Value = serde_json::from_str(metadata.text().unwrap()).unwrap();
    assert_eq!(from_svg, image.provenance);
    assert_eq!(
        from_svg["source"],
        serde_json::to_value(captured.pin()).unwrap()
    );
    assert_eq!(from_svg["context"]["user"], "Saha <A&B>");
    let mut at = 8;
    let mut from_png = None;
    let mut physical = None;
    while at + 12 <= image.png.len() {
        let len = u32::from_be_bytes(image.png[at..at + 4].try_into().unwrap()) as usize;
        let kind = &image.png[at + 4..at + 8];
        let bytes = &image.png[at + 8..at + 8 + len];
        if kind == b"iTXt" && bytes.starts_with(b"kentos.source\0") {
            let tail = &bytes[b"kentos.source\0".len()..];
            assert_eq!(&tail[..4], &[0, 0, 0, 0]);
            from_png = Some(serde_json::from_slice::<Value>(&tail[4..]).unwrap());
        }
        if kind == b"pHYs" {
            physical = Some((
                u32::from_be_bytes(bytes[..4].try_into().unwrap()),
                u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
                bytes[8],
            ));
        }
        at += len + 12;
    }
    assert_eq!(from_png, Some(from_svg.clone()));
    assert_eq!(physical, Some((3780, 3780, 1)));
    let pdf = book_pdf_captured(&captured, &["ground".into()], None, None).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&info_keywords(&pdf.bytes)).unwrap(),
        from_svg
    );
    assert_eq!(from_svg["laterEditsExcluded"], true);
    assert_eq!(from_svg["fonts"].as_array().unwrap().len(), 22);
    fs::remove_dir_all(root).unwrap();
}
