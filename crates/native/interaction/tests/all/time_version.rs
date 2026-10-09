//! Yeni sürüm oluştur and Sona erdir (docs/adr/0210 §7) with a date typed, as
//! the web's `apps/web/src/tools/timeVersionTool.test.ts`: the values written,
//! the copies selected, one undo step; a date field of the layer's
//! (docs/adr/0199) takes the day alone (§3); a date the objects do not span is
//! refused and nothing is written. The traces (`time-version.json`) play the
//! same in both apps with the time slider.

use crate::common;

use common::Bench;
use kentos_domain::Slot;

/// A layer “Çizim” ranged in time (`bas` – `bit`, key `no`), its `bit` a date
/// field when `dated`; two points, one ending in 2030, one open.
fn drawing(dated: bool) -> (Bench, Slot, Slot) {
    let fields = if dated {
        r#","fields":[{"name":"no","kind":"text"},{"name":"bas","kind":"text"},{"name":"bit","kind":"date"}]"#
    } else {
        ""
    };
    let text = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Sürüm",
        "settings":{{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis"}},
        "origin":{{"x":487000,"y":4420000}},
        "layers":[{{"id":"cizim","name":"Çizim","type":"layer","visible":true,"locked":false,"expanded":true,
          "style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"children":[],
          "time":{{"start":"bas","end":"bit","key":"no"}}{fields}}}],
        "activeLayer":"cizim",
        "entities":[
          {{"id":1,"kind":"point","layerId":"cizim","attrs":{{"no":"1","bas":"2010-01-01","bit":"2030-01-01"}},"p":{{"x":487000,"y":4420000}}}},
          {{"id":2,"kind":"point","layerId":"cizim","attrs":{{"no":"2","bas":"2012-06-01T08:00:00"}},"p":{{"x":487005,"y":4420000}}}}
        ],
        "styles":{{"items":[],"categories":[]}}}}"#
    );
    let b = Bench::on(&text);
    (b, Slot(1), Slot(2))
}

fn attr(b: &Bench, slot: Slot, key: &str) -> Option<String> {
    b.doc
        .get(slot)
        .and_then(|e| e.base().attrs.get(key).cloned())
}

#[test]
fn a_new_version_ends_the_objects_at_the_date_typed_and_its_copies_are_selected() {
    let (mut b, one, two) = drawing(false);
    b.selection.set(vec![one, two]);
    b.start("timeVersion");
    assert!(b.type_text("15.06.2020 14:30"));
    assert_eq!(attr(&b, one, "bit").as_deref(), Some("2020-06-15T14:30:00"));
    assert_eq!(attr(&b, two, "bit").as_deref(), Some("2020-06-15T14:30:00"));
    let copies: Vec<_> = b
        .selected()
        .into_iter()
        .map(|s| b.doc.get(Slot(s)).expect("a copy").base().attrs.clone())
        .collect();
    assert_eq!(copies.len(), 2);
    assert_eq!(
        copies[0].get("bas").map(String::as_str),
        Some("2020-06-15T14:30:00")
    );
    assert_eq!(copies[0].get("bit").map(String::as_str), Some("2030-01-01"));
    assert_eq!(
        copies[1].get("bas").map(String::as_str),
        Some("2020-06-15T14:30:00")
    );
    assert_eq!(copies[1].get("bit"), None);
    assert_eq!(
        b.last_text(),
        Some(
            "Yeni sürüm: 2 nesne 15.06.2020 tarihinde sona erdi, 2 yeni sürümü yazıldı ve seçildi."
        )
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Yeni sürüm oluştur"));
    assert_eq!(b.doc.len(), 2);
    assert_eq!(attr(&b, one, "bit").as_deref(), Some("2030-01-01"));
}

#[test]
fn a_date_field_takes_the_day_alone() {
    let (mut b, one, _) = drawing(true);
    b.selection.set(vec![one]);
    b.start("timeEnd");
    assert!(b.type_text("2020-06-15T14:30"));
    assert_eq!(attr(&b, one, "bit").as_deref(), Some("2020-06-15"));
    assert_eq!(b.last_text(), Some("Sona erdi: 1 nesne, 15.06.2020."));
}

#[test]
fn a_date_the_objects_do_not_span_is_refused_and_nothing_is_written() {
    let (mut b, one, two) = drawing(false);
    b.selection.set(vec![one, two]);
    b.start("timeEnd");
    assert!(b.type_text("2011-01-01"));
    assert_eq!(
        b.last_text(),
        Some(
            "1 nesnenin zamanı 01.01.2011 anını içermiyor: başlangıcı bu tarihten önce, bitişi sonra olmalı. Hiçbir nesne yazılmadı."
        )
    );
    assert_eq!(attr(&b, one, "bit").as_deref(), Some("2030-01-01"));
    assert_eq!(attr(&b, two, "bit"), None);
    assert!(!b.doc.can_undo());
}
