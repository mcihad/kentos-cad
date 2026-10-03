//! Every operation (design §4): applied to the base book
//! (`fixtures/sheet/v1/ops/book.json`), its inverse takes the book back to
//! the very same value; the refusals have their codes. The results are
//! recorded in `fixtures/sheet/v1/ops/cases.json` (the resulting book's
//! SHA-256, the inverse and the undo step's name; `KENTOS_WRITE_SHEET=1`
//! rewrites it): the WASM binding is held to the same file.

mod common;

use common::*;
use kentos_sheet::ops::{Applied, Op, apply, apply_all};
use kentos_sheet::template::{sha256_hex, system_template};
use kentos_sheet::validate::{read_book, validate_book};
use kentos_sheet::*;
use serde_json::{Value, json};

fn base() -> SheetBook {
    let text = std::fs::read_to_string(fixtures().join("ops/book.json")).unwrap();
    read_book(&text).unwrap()
}

/// The cases: a name, the operation as JSON (the web sends the same), and the error code when it is refused.
fn cases() -> Vec<(&'static str, Value, Option<&'static str>)> {
    let page = |paper: &str, orientation: &str, w: i32, h: i32| {
        json!({ "paper": paper, "orientation": orientation, "size": { "width": w, "height": h },
                "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } })
    };
    let text_item = |id: &str, name: &str| {
        json!({ "id": id, "name": name, "frame": { "left": 20000, "top": 60000, "width": 80000, "height": 15000 },
                "kind": { "type": "text", "content": "Eklenen" } })
    };
    let template = serde_json::to_value(system_template("sys:genel-a4-dikey").unwrap()).unwrap();
    vec![
        (
            "sayfa ekle",
            json!({ "op": "addSheet", "index": 1, "sheet": { "id": "s3", "name": "Pafta 3", "page": page("a4", "portrait", 210000, 297000) } }),
            None,
        ),
        (
            "sayfa sil",
            json!({ "op": "removeSheet", "id": "s2" }),
            None,
        ),
        (
            "sayfaya ad",
            json!({ "op": "renameSheet", "id": "s1", "name": "Ana pafta" }),
            None,
        ),
        (
            "sayfa taşı",
            json!({ "op": "moveSheet", "id": "s2", "to": 0 }),
            None,
        ),
        (
            "sayfa çoğalt",
            json!({ "op": "duplicateSheet", "id": "s1", "newId": "s9", "name": "Pafta 1 kopyası",
                    "itemIds": ["x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11"] }),
            None,
        ),
        (
            "sayfa çoğalt: kimlik eksik",
            json!({ "op": "duplicateSheet", "id": "s1", "newId": "s9", "name": "K", "itemIds": ["x1"] }),
            Some("ids_missing"),
        ),
        (
            "sayfayı değiştir",
            json!({ "op": "replaceSheet", "sheet": { "id": "s2", "name": "Yeni", "page": page("a3", "landscape", 420000, 297000) } }),
            None,
        ),
        (
            "kâğıt A1 yatay",
            json!({ "op": "setPage", "owner": { "kind": "sheet", "id": "s1" }, "page": page("a1", "landscape", 841000, 594000) }),
            None,
        ),
        (
            "kâğıt yerleşimsiz",
            json!({ "op": "setPage", "owner": { "kind": "sheet", "id": "s1" }, "page": page("a4", "portrait", 210000, 297000), "relayout": false }),
            None,
        ),
        (
            "ana sayfanın kâğıdı",
            json!({ "op": "setPage", "owner": { "kind": "master", "id": "M1" }, "page": page("a4", "portrait", 210000, 297000) }),
            None,
        ),
        (
            "kâğıt boyu uyuşmuyor",
            json!({ "op": "setPage", "owner": { "kind": "sheet", "id": "s1" }, "page": page("a4", "portrait", 200000, 297000) }),
            Some("page_size_mismatch"),
        ),
        (
            "ana sayfa ekle",
            json!({ "op": "addMaster", "master": { "id": "M2", "name": "İkinci", "page": page("a4", "portrait", 210000, 297000) } }),
            None,
        ),
        (
            "kullanılan ana sayfa silinmez",
            json!({ "op": "removeMaster", "id": "M1" }),
            Some("master_in_use"),
        ),
        (
            "ana sayfayı değiştir",
            json!({ "op": "replaceMaster", "master": { "id": "M1", "name": "Ana", "page": page("a3", "landscape", 420000, 297000) } }),
            None,
        ),
        (
            "ana sayfa seç",
            json!({ "op": "setSheetMaster", "sheet": "s2", "master": "M1" }),
            None,
        ),
        (
            "ana sayfadan ayır",
            json!({ "op": "detachMaster", "sheet": "s1", "itemIds": ["d1", "d2"] }),
            None,
        ),
        (
            "öğe ekle",
            json!({ "op": "addItems", "to": { "kind": "sheet", "id": "s2" }, "items": [text_item("a1", "Eklenen")], "index": 0 }),
            None,
        ),
        (
            "aynı ad eklenmez",
            json!({ "op": "addItems", "to": { "kind": "sheet", "id": "s2" }, "items": [text_item("a1", "Not")] }),
            Some("duplicate_name"),
        ),
        (
            "aynı kimlik eklenmez",
            json!({ "op": "addItems", "to": { "kind": "sheet", "id": "s2" }, "items": [text_item("m1", "Başka")] }),
            Some("duplicate_id"),
        ),
        (
            "öğe yerleştir",
            json!({ "op": "insertItems", "to": { "kind": "sheet", "id": "s2" }, "entries": [{ "index": 0, "item": text_item("a2", "İlk") }] }),
            None,
        ),
        (
            "grubu sil",
            json!({ "op": "removeItems", "ids": ["g1"] }),
            None,
        ),
        (
            "devam çerçevesi silinmez",
            json!({ "op": "removeItems", "ids": ["tc1"] }),
            Some("item_in_use"),
        ),
        (
            "haritayı sil (bağlar kopuk kalır)",
            json!({ "op": "removeItems", "ids": ["m1"] }),
            None,
        ),
        (
            "grubu taşı",
            json!({ "op": "moveItems", "ids": ["g1"], "delta": [5000, -2500] }),
            None,
        ),
        (
            "kilitli taşınmaz",
            json!({ "op": "moveItems", "ids": ["p1"], "delta": [1000, 0] }),
            Some("item_locked"),
        ),
        (
            "iki sayfadan taşınmaz",
            json!({ "op": "moveItems", "ids": ["t1", "u1"], "delta": [1000, 0] }),
            Some("mixed_owners"),
        ),
        (
            "boyutlandır",
            json!({ "op": "resizeItem", "id": "t1", "handle": "se", "to": [400000, 85000] }),
            None,
        ),
        (
            "dönük boyutlandır",
            json!({ "op": "resizeItem", "id": "na1", "handle": "e", "to": [400000, 50000] }),
            None,
        ),
        (
            "grubu boyutlandır",
            json!({ "op": "resizeItem", "id": "g1", "handle": "se", "to": [400000, 160000] }),
            None,
        ),
        (
            "oranlı, ortadan",
            json!({ "op": "resizeItem", "id": "t1", "handle": "ne", "to": [395000, 55000], "keepAspect": true, "fromCenter": true }),
            None,
        ),
        (
            "döndür",
            json!({ "op": "rotateItems", "ids": ["t1"], "angle": 30000 }),
            None,
        ),
        (
            "birlikte döndür",
            json!({ "op": "rotateItems", "ids": ["t1", "sb1"], "angle": -90000 }),
            None,
        ),
        (
            "bir nokta çevresinde döndür",
            json!({ "op": "rotateItems", "ids": ["g1"], "angle": 45000, "around": [300000, 100000] }),
            None,
        ),
        (
            "çerçeveler",
            json!({ "op": "setFrames", "frames": [{ "id": "t1", "frame": { "left": 1000, "top": 2000, "width": 3000, "height": 4000 }, "rotation": 5000 }] }),
            None,
        ),
        (
            "özellik",
            json!({ "op": "setItemProps", "id": "t1", "patch": { "kind": { "type": "text", "content": "Yeni başlık", "align": "center" }, "border": { "width": 350 } } }),
            None,
        ),
        (
            "haritanın görünüşü atlasa",
            json!({ "op": "setItemProps", "id": "m1", "patch": { "kind": { "type": "map", "view": { "type": "atlas", "policy": { "type": "fit", "marginPct": 10 } } } } }),
            None,
        ),
        (
            "isteğe bağlı alan silinir",
            json!({ "op": "setItemProps", "id": "r1", "patch": { "kind": { "type": "shape", "fill": null } } }),
            None,
        ),
        (
            "kimlik yamayla değişmez",
            json!({ "op": "setItemProps", "id": "t1", "patch": { "id": "zz" } }),
            Some("bad_patch"),
        ),
        (
            "tür yamayla değişmez",
            json!({ "op": "setItemProps", "id": "t1", "patch": { "kind": { "type": "line" } } }),
            Some("kind_change"),
        ),
        (
            "bilinmeyen alan yamada",
            json!({ "op": "setItemProps", "id": "t1", "patch": { "kind": { "type": "text", "zoom": 2 } } }),
            Some("bad_patch"),
        ),
        (
            "aralık dışı yamada",
            json!({ "op": "setItemProps", "id": "t1", "patch": { "opacity": 150 } }),
            Some("out_of_range"),
        ),
        (
            "ad ver",
            json!({ "op": "renameItem", "id": "t1", "name": "Ana başlık" }),
            None,
        ),
        (
            "aynı ad verilmez",
            json!({ "op": "renameItem", "id": "t1", "name": "Harita" }),
            Some("duplicate_name"),
        ),
        (
            "öne getir",
            json!({ "op": "reorder", "ids": ["t1"], "to": "front" }),
            None,
        ),
        (
            "arkaya gönder",
            json!({ "op": "reorder", "ids": ["g1"], "to": "back" }),
            None,
        ),
        (
            "bir öne",
            json!({ "op": "reorder", "ids": ["sb1", "na1"], "to": "forward" }),
            None,
        ),
        (
            "bir arkaya",
            json!({ "op": "reorder", "ids": ["ln1"], "to": "backward" }),
            None,
        ),
        (
            "sıra",
            json!({ "op": "setOrder", "owner": { "kind": "sheet", "id": "s1" },
                         "order": ["p1", "tc1", "tb1", "ln1", "g1", "r2", "r1", "t1", "na1", "sb1", "m1"] }),
            None,
        ),
        (
            "eksik sıra",
            json!({ "op": "setOrder", "owner": { "kind": "sheet", "id": "s1" }, "order": ["m1"] }),
            Some("bad_order"),
        ),
        (
            "grupla",
            json!({ "op": "group", "ids": ["sb1", "na1"], "group": { "id": "g2", "name": "İşaretler", "frame": { "left": 0, "top": 0, "width": 1000, "height": 1000 }, "kind": { "type": "group" } } }),
            None,
        ),
        ("grubu çöz", json!({ "op": "ungroup", "id": "g1" }), None),
        (
            "öğeler",
            json!({ "op": "setItems", "owner": { "kind": "sheet", "id": "s2" }, "items": [] }),
            None,
        ),
        (
            "kilitle",
            json!({ "op": "lock", "ids": ["t1", "p1"], "value": true }),
            None,
        ),
        (
            "gizle",
            json!({ "op": "hide", "ids": ["t1"], "value": true }),
            None,
        ),
        (
            "sola hizala",
            json!({ "op": "align", "ids": ["t1", "sb1", "g1"], "edge": "left", "to": "selection" }),
            None,
        ),
        (
            "sayfanın ortasına",
            json!({ "op": "align", "ids": ["t1"], "edge": "center", "to": "page" }),
            None,
        ),
        (
            "kenar boşluğunun altına",
            json!({ "op": "align", "ids": ["t1", "na1"], "edge": "bottom", "to": "margins" }),
            None,
        ),
        (
            "anahtara göre ortala",
            json!({ "op": "align", "ids": ["t1", "sb1"], "edge": "middle", "to": "keyItem", "key": "m1" }),
            None,
        ),
        (
            "aralıkları eşitle",
            json!({ "op": "distribute", "ids": ["sb1", "t1", "g1"], "axis": "y", "mode": "gaps" }),
            None,
        ),
        (
            "merkezleri eşitle",
            json!({ "op": "distribute", "ids": ["sb1", "t1", "g1", "ln1"], "axis": "y", "mode": "centers" }),
            None,
        ),
        (
            "iki öğe dağıtılmaz",
            json!({ "op": "distribute", "ids": ["sb1", "t1"], "axis": "x", "mode": "gaps" }),
            Some("no_items"),
        ),
        (
            "aynı genişlik",
            json!({ "op": "matchSize", "ids": ["t1", "sb1"], "dimension": "width" }),
            None,
        ),
        (
            "aynı boy",
            json!({ "op": "matchSize", "ids": ["sb1", "t1"], "dimension": "both", "key": "tb1" }),
            None,
        ),
        (
            "kılavuz ekle",
            json!({ "op": "addGuide", "owner": { "kind": "sheet", "id": "s1" }, "guide": { "id": "k3", "axis": "y", "at": 50000 } }),
            None,
        ),
        (
            "kılavuz taşı",
            json!({ "op": "moveGuide", "owner": { "kind": "sheet", "id": "s1" }, "id": "k1", "at": 160000 }),
            None,
        ),
        (
            "kilitli kılavuz taşınmaz",
            json!({ "op": "moveGuide", "owner": { "kind": "sheet", "id": "s1" }, "id": "k2", "at": 1 }),
            Some("item_locked"),
        ),
        (
            "kılavuz sil",
            json!({ "op": "removeGuide", "owner": { "kind": "sheet", "id": "s1" }, "id": "k1" }),
            None,
        ),
        (
            "ızgara",
            json!({ "op": "setSnapGrid", "sheet": "s1", "grid": { "spacing": 2500, "visible": true, "enabled": true } }),
            None,
        ),
        (
            "atlas",
            json!({ "op": "setAtlas", "sheet": "s1", "atlas": { "layer": "parsel", "sort": [{ "expression": "parsel_no" }] } }),
            None,
        ),
        (
            "dışa aktarma",
            json!({ "op": "setExport", "sheet": "s1", "export": { "dpi": 600, "format": "png", "fileName": "[% @ada %]" } }),
            None,
        ),
        (
            "şablon uygula",
            json!({ "op": "applyTemplate", "sheet": "s2", "template": template,
                    "ids": { "sheet": "yok", "items": ["y1", "y2", "y3", "y4", "y5"] },
                    "options": { "paper": { "paper": "a3", "orientation": "landscape" }, "values": [{ "name": "kurum", "value": "Belediye" }] } }),
            None,
        ),
        (
            "pafta değişkenleri",
            json!({ "op": "saveVariables", "sheet": "s1", "variables": [{ "name": "ada", "value": "202" }, { "name": "parsel", "value": "7" }] }),
            None,
        ),
        (
            "proje değişkenleri",
            json!({ "op": "saveVariables", "variables": [] }),
            None,
        ),
        (
            "yanlış türde değer",
            json!({ "op": "saveVariables", "variables": [{ "name": "say", "kind": "number", "value": "on" }] }),
            Some("bad_variable_value"),
        ),
        (
            "resim ekle",
            json!({ "op": "addAssets", "assets": [{ "sha256": "0000000000000000000000000000000000000000000000000000000000000001", "kind": "jpeg", "name": "a.jpg", "width": 10, "height": 10, "bytes": 100 }] }),
            None,
        ),
        (
            "kullanılan resim kaldırılmaz",
            json!({ "op": "removeAssets", "sha256": ["6c3e3d6d9c8f1c7a2b5e4d3c2b1a09f8e7d6c5b4a39281706f5e4d3c2b1a0918"] }),
            Some("asset_in_use"),
        ),
        // Layout variants (design §3.2a).
        (
            "dikey yerleşim düzeni (bu kâğıtta geçerli değil)",
            json!({ "op": "addVariant", "owner": { "kind": "sheet", "id": "s1" }, "id": "dikey", "name": "Dikey kâğıt", "when": { "orientation": "portrait" } }),
            None,
        ),
        (
            "yatay yerleşim düzeni (hemen geçerli)",
            json!({ "op": "addVariant", "owner": { "kind": "sheet", "id": "s1" }, "id": "yatay", "name": "Yatay kâğıt", "when": { "orientation": "landscape", "minWidth": 400000 } }),
            None,
        ),
        (
            "ana sayfanın yerleşim düzeni",
            json!({ "op": "addVariant", "owner": { "kind": "master", "id": "M1" }, "id": "dikey", "name": "Dikey", "when": { "orientation": "portrait" }, "index": 0 }),
            None,
        ),
        (
            "düzen kimliği boş olamaz",
            json!({ "op": "addVariant", "owner": { "kind": "sheet", "id": "s1" }, "id": "", "name": "Boş", "when": { "orientation": "portrait" } }),
            Some("duplicate_id"),
        ),
        (
            "düzenin koşulu ters olamaz",
            json!({ "op": "addVariant", "owner": { "kind": "sheet", "id": "s1" }, "id": "ters", "name": "Ters", "when": { "orientation": "portrait", "minWidth": 300000, "maxWidth": 200000 } }),
            Some("out_of_range"),
        ),
        (
            "olmayan düzen silinmez",
            json!({ "op": "removeVariant", "owner": { "kind": "sheet", "id": "s1" }, "id": "yok" }),
            Some("unknown_variant"),
        ),
        (
            "olmayan düzen değişmez",
            json!({ "op": "setVariant", "owner": { "kind": "sheet", "id": "s1" }, "id": "yok", "name": "X" }),
            Some("unknown_variant"),
        ),
    ]
}

#[test]
fn the_base_book_is_read_in_its_normal_form() {
    let b = base();
    let g = b.item("g1").unwrap();
    // The group's frame is its children's together; colours lowercase.
    assert_eq!(g.frame, RectUm::new(300_000, 100_000, 80_000, 40_000));
    match &b.item("r1").unwrap().kind {
        ItemKind::Shape(s) => assert_eq!(s.fill.as_deref(), Some("#e0e0e0")),
        _ => unreachable!(),
    }
    validate_book(&b).unwrap();
}

#[test]
fn every_operation_comes_back_with_its_inverse() {
    let book = base();
    let mut recorded = Vec::new();
    let mut failures = Vec::new();
    for (name, op_json, error) in cases() {
        let op: Op = match serde_json::from_value(op_json.clone()) {
            Ok(op) => op,
            Err(e) => {
                failures.push(format!("{name}: işlem okunamadı: {e}"));
                continue;
            }
        };
        // The operation as JSON goes and comes back the same.
        assert_eq!(
            serde_json::from_value::<Op>(serde_json::to_value(&op).unwrap()).unwrap(),
            op,
            "{name}"
        );
        match (apply(&book, &op), error) {
            (Ok(a), None) => {
                if let Err(e) = validate_book(&a.book) {
                    failures.push(format!("{name}: sonuç geçersiz: {e}"));
                }
                match apply_all(&a.book, &a.inverse) {
                    Ok(back) if back.book == book => {}
                    Ok(back) => failures.push(format!(
                        "{name}: tersi kitabı geri getirmedi\n  beklenen: {}\n  gelen:    {}",
                        serde_json::to_string(&book).unwrap(),
                        serde_json::to_string(&back.book).unwrap()
                    )),
                    Err(e) => failures.push(format!("{name}: tersi uygulanamadı: {e}")),
                }
                let book_json = serde_json::to_string(&a.book).unwrap();
                recorded.push(json!({
                    "name": name,
                    "op": op_json,
                    "expect": {
                        "bookSha256": sha256_hex(book_json.as_bytes()),
                        "inverse": a.inverse,
                        "label": a.label
                    }
                }));
            }
            (Err(e), Some(code)) => {
                if e.code != code {
                    failures.push(format!(
                        "{name}: {code} bekleniyordu, {} geldi ({})",
                        e.code, e.message
                    ));
                }
                recorded.push(json!({ "name": name, "op": op_json, "expect": { "error": code } }));
            }
            (Ok(_), Some(code)) => failures.push(format!("{name}: {code} bekleniyordu, uygulandı")),
            (Err(e), None) => failures.push(format!("{name}: uygulanamadı: {e}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let file = fixtures().join("ops/cases.json");
    let text = serde_json::to_string_pretty(&json!({
        "schema": "kentos.sheet.fixture.ops/1",
        "book": "book.json",
        "cases": recorded
    }))
    .unwrap()
        + "\n";
    if writing() {
        std::fs::write(&file, &text).unwrap();
    } else {
        let have = std::fs::read_to_string(&file).unwrap_or_default();
        assert!(
            have == text,
            "{} güncel değil: KENTOS_WRITE_SHEET=1 ile yazın ve farkı okuyun",
            file.display()
        );
    }
}

#[test]
fn a_detached_masters_items_get_free_names_and_the_page_layout() {
    let book = base();
    let op: Op = serde_json::from_value(
        json!({ "op": "detachMaster", "sheet": "s1", "itemIds": ["d1", "d2"] }),
    )
    .unwrap();
    let a = apply(&book, &op).unwrap();
    let s = a.book.sheet("s1").unwrap();
    assert!(s.master.is_none());
    assert_eq!(s.items[0].id, "d1");
    assert_eq!(s.items[1].name, "Antet");
    // Laid out on the sheet's page: the same A3, so in the same place.
    assert_eq!(
        s.items[1].frame,
        RectUm::new(230_000, 247_000, 180_000, 40_000)
    );
}

#[test]
fn a_rotated_resize_keeps_the_opposite_edge_in_place() {
    let book = base();
    let before = book.item("na1").unwrap().clone();
    let op: Op = serde_json::from_value(
        json!({ "op": "resizeItem", "id": "na1", "handle": "e", "to": [420000, 40000] }),
    )
    .unwrap();
    let a = apply(&book, &op).unwrap();
    let after = a.book.item("na1").unwrap();
    // The west edge's middle stays where it was on the paper (within the rounding of the centre).
    let west = |it: &Item| {
        let c = it.frame.center();
        rotate([f64::from(it.frame.left), c[1]], c, it.rotation)
    };
    let (w0, w1) = (west(&before), west(after));
    assert!(
        (w0[0] - w1[0]).abs() <= 1.0 && (w0[1] - w1[1]).abs() <= 1.0,
        "{w0:?} {w1:?}"
    );
    assert!(after.frame.width > before.frame.width);
    assert_eq!(after.frame.height, before.frame.height);
}

#[test]
fn moving_a_group_moves_its_children() {
    let book = base();
    let op: Op =
        serde_json::from_value(json!({ "op": "moveItems", "ids": ["g1"], "delta": [5000, -2500] }))
            .unwrap();
    let a = apply(&book, &op).unwrap();
    assert_eq!(
        a.book.item("r1").unwrap().frame,
        RectUm::new(305_000, 97_500, 30_000, 20_000)
    );
    assert_eq!(
        a.book.item("g1").unwrap().frame,
        RectUm::new(305_000, 97_500, 80_000, 40_000)
    );
    assert_eq!(a.label, "Taşı: Grup");
}

#[test]
fn reorder_forward_and_backward_step_past_one_item() {
    let book = base();
    let order = |b: &SheetBook| {
        b.sheet("s1")
            .unwrap()
            .items
            .iter()
            .map(|i| i.id.clone())
            .collect::<Vec<_>>()
    };
    let fwd: Op =
        serde_json::from_value(json!({ "op": "reorder", "ids": ["sb1"], "to": "forward" }))
            .unwrap();
    let a = apply(&book, &fwd).unwrap();
    assert_eq!(&order(&a.book)[..3], ["m1", "na1", "sb1"]);
    let back: Op =
        serde_json::from_value(json!({ "op": "reorder", "ids": ["t1"], "to": "backward" }))
            .unwrap();
    let a = apply(&book, &back).unwrap();
    assert_eq!(&order(&a.book)[..4], ["m1", "sb1", "t1", "na1"]);
}

/// A portrait layout made on portrait paper keeps its own arrangement: landscape comes back
/// exactly, and an edit made on portrait stays there (design §3.2a). Every step's inverse puts
/// the book back as it was.
#[test]
fn a_variant_keeps_its_own_arrangement() {
    let page = |paper: &str, orientation: &str, w: i32, h: i32| {
        json!({ "paper": paper, "orientation": orientation, "size": { "width": w, "height": h },
                "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } })
    };
    let s1 = || json!({ "kind": "sheet", "id": "s1" });
    let mut book = base();
    let landscape = book.sheet("s1").unwrap().items.clone();
    let step = |book: &mut SheetBook, op: Value| -> Applied {
        let op: Op = serde_json::from_value(op).unwrap();
        let a = apply(book, &op).unwrap();
        let back = apply_all(&a.book, &a.inverse).unwrap();
        assert_eq!(back.book, *book, "{}: tersi kitabı geri getirmedi", a.label);
        *book = a.book.clone();
        a
    };
    // To A4 portrait with no variant yet: the items follow their constraints.
    let a = step(
        &mut book,
        json!({ "op": "setPage", "owner": s1(), "page": page("a4", "portrait", 210000, 297000) }),
    );
    let plain_portrait = a.book.sheet("s1").unwrap().items.clone();
    // A portrait layout from this arrangement: in force at once, nothing moves.
    let a = step(
        &mut book,
        json!({ "op": "addVariant", "owner": s1(), "id": "dikey", "name": "Dikey kâğıt", "when": { "orientation": "portrait" } }),
    );
    let s = a.book.sheet("s1").unwrap();
    assert_eq!(s.active_variant.as_deref(), Some("dikey"));
    assert_eq!(s.items, plain_portrait);
    assert_eq!(
        s.variants[0].reference,
        SizeUm {
            width: 210_000,
            height: 297_000
        }
    );
    // A fix on portrait goes into the portrait layout.
    let a = step(
        &mut book,
        json!({ "op": "moveItems", "ids": ["t1"], "delta": [0, 30000] }),
    );
    let moved = a.book.item("t1").unwrap().frame;
    let rec = a.book.sheet("s1").unwrap().variants[0]
        .frames
        .iter()
        .find(|f| f.item == "t1")
        .unwrap()
        .frame;
    assert_eq!(
        rec, moved,
        "the reference is this paper: the record is the frame itself"
    );
    assert_eq!(a.label, "Taşı: Başlık");
    // Back to A3 landscape: the base layout, exactly as it was; no variant in force.
    let a = step(
        &mut book,
        json!({ "op": "setPage", "owner": s1(), "page": page("a3", "landscape", 420000, 297000) }),
    );
    let s = a.book.sheet("s1").unwrap();
    assert_eq!(s.active_variant, None);
    assert_eq!(s.base_layout, None);
    assert_eq!(s.items, landscape);
    // Portrait again: the fix is there.
    let a = step(
        &mut book,
        json!({ "op": "setPage", "owner": s1(), "page": page("a4", "portrait", 210000, 297000) }),
    );
    assert_eq!(a.book.item("t1").unwrap().frame, moved);
    // Removing the layout in force brings back the base layout carried to this paper.
    let a = step(
        &mut book,
        json!({ "op": "removeVariant", "owner": s1(), "id": "dikey" }),
    );
    let s = a.book.sheet("s1").unwrap();
    assert!(s.variants.is_empty() && s.active_variant.is_none() && s.base_layout.is_none());
    assert_eq!(s.items, plain_portrait);
    assert_eq!(a.label, "Yerleşim düzenini sil: Dikey kâğıt");
}
