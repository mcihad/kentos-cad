//! Parçala, Çizimi temizle and Özellik kopyala through the session, over the
//! native document (docs/adr/0140): the three ways of cutting, what a
//! cleaning finds and shows first, one property source and its targets; each
//! write one undo step named after the tool, a locked layer refused, nothing
//! to do said plainly. The drawing is the traces' `edits.kcad`: the lines 1
//! and 2 crossing at (−20, 12), the line 3 at x = −4, the L-shaped polyline 4
//! (attribute Ad = Yol) from (0, 14) to (0, 4) to (10, 4), the 10 m line 11
//! from (−28, −12) to (−18, −12) and the line 13 on the locked layer
//! `kilitli`. Expected values are worked out by hand.

mod common;

use common::{Bench, E, N, rel};
use kentos_contracts::{Entity, EntityBase, LineEntity, Vec2 as Wire};
use kentos_domain::Slot;
use kentos_interaction::{Level, MarkerShape, Tone};

const EDITS: &str = include_str!("../../../../fixtures/interaction/v1/edits.kcad");
const EMPTY: &str = include_str!("../../../../fixtures/interaction/v1/empty.kcad");

fn bench() -> Bench {
    let mut b = Bench::on(EDITS);
    b.draft.snap = false;
    b
}

fn preview(b: &Bench) -> kentos_interaction::Preview {
    b.session.preview(&Default::default()).expect("a preview")
}

/// A line's ends, east and north differences from (E, N).
fn ends(b: &Bench, slot: Slot) -> [[f64; 2]; 2] {
    let Some(Entity::Line(l)) = b.doc.get(slot) else {
        panic!("line {slot:?}: {:?}", b.doc.get(slot));
    };
    [rel(l.a), rel(l.b)]
}

fn length(l: [[f64; 2]; 2]) -> f64 {
    (l[1][0] - l[0][0]).hypot(l[1][1] - l[0][1])
}

// ── Kesişimlerden ───────────────────────────────────────────────────────────

#[test]
fn split_cuts_the_selection_where_it_crosses_and_keeps_the_data() {
    let mut b = bench();
    // The polyline 4 (Ad = Yol) and a line across its long leg.
    let across = b.add_line("cizim", [-5.0, 8.0], [5.0, 8.0]);
    b.selection.set([Slot(4), across]);
    let before = b.doc.len();
    b.start("split");
    assert_eq!(
        b.session.prompt().text(),
        "Parçala: Enter ya da sağ tıkla parçalayın [2 nesne 4 parçaya bölünecek; Uygula (Enter); Eşit parçalara (E) / Uzunluktan (U)]"
    );
    assert_eq!(b.options(), ["Enter", "E", "U"]);
    // The pieces are drawn in turn, a ring at each cut.
    b.move_to(20.0, 20.0);
    let p = preview(&b);
    assert_eq!(p.strokes.len(), 4);
    assert!(p.strokes.iter().any(|s| s.tone == Tone::Snap));
    assert_eq!(
        p.markers
            .iter()
            .filter(|m| matches!(m.shape, MarkerShape::Ring(_)))
            .count(),
        2
    );
    assert_eq!(p.tag.expect("a tag").lines, ["2 nesne", "4 parça"]);
    assert_eq!(b.doc.len(), before, "nothing is written yet");
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("2 nesne kesişim yerlerinden 4 parçaya bölündü.")
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.len(), before + 2);
    // The first piece keeps its place and id; every piece its data.
    // (A piece of one edge is a line; the core's `sub_path` decides.)
    let first = b.doc.get(Slot(4)).expect("the object stays in its place");
    assert_eq!(
        first.base().attrs.get("Ad").map(String::as_str),
        Some("Yol")
    );
    let with_data = b
        .doc
        .entities()
        .filter(|e| e.base().attrs.get("Ad").is_some_and(|v| v == "Yol"))
        .count();
    assert_eq!(with_data, 2, "the piece made from it has the data too");
    assert_eq!(b.selected().len(), 4, "the pieces are the selection");
    // The line across: cut at x = 0, the first piece in its place.
    assert_eq!(ends(&b, across), [[-5.0, 8.0], [0.0, 8.0]]);
    assert_eq!(b.doc.undo().as_deref(), Some("Parçala"));
    assert_eq!(b.doc.len(), before);
    assert_eq!(ends(&b, across), [[-5.0, 8.0], [5.0, 8.0]]);
}

#[test]
fn split_with_nothing_crossing_says_so_and_changes_nothing() {
    let mut b = bench();
    b.selection.set([Slot(1), Slot(3)]);
    let rev = b.doc.revision();
    b.start("split");
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Parçala: seçilen nesneler birbirini kesmiyor")
    );
    assert!(
        b.session.is_running(),
        "the tool waits: a method may follow"
    );
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Seçilen nesneler birbirini kesmiyor")
    );
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.revision(), rev);
}

#[test]
fn split_picks_first_and_a_locked_layer_is_left_out() {
    let mut b = bench();
    // The crossing pair and the locked line 13 across nothing.
    b.start("split");
    assert!(b.session.prompt().text().starts_with(
        "Parçala: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili)"
    ));
    b.click(-26.0, 12.0);
    b.click(-20.0, 8.0);
    b.click(8.0, -15.0);
    assert_eq!(b.selected(), [1, 2, 13]);
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("1 nesne kilitli katmanda olduğu için atlandı.")
    );
    assert!(b.session.is_running());
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("2 nesne kesişim yerlerinden 4 parçaya bölündü.")
    );
    assert_eq!(
        ends(&b, Slot(13)),
        [[8.0, -12.0], [8.0, -18.0]],
        "the locked line is as it was"
    );
}

// ── Eşit parçalara, Uzunluktan ──────────────────────────────────────────────

#[test]
fn equal_parts_cut_the_clicked_object_at_the_typed_count() {
    let mut b = bench();
    b.start("split");
    // The ribbon sends the method's letter right after the tool starts.
    assert!(b.type_text("E"));
    assert_eq!(
        b.session.prompt().text(),
        "Parçala: bölünecek nesneye tıklayın [parça sayısı 4; Kesişimlerden (K) / Uzunluktan (U)]"
    );
    assert_eq!(b.options(), ["K", "U"]);
    // Hovered by its edge, as the edge tools do.
    b.move_to(-23.0, -12.0);
    assert_eq!(b.selection.hover(), Some(Slot(11)));
    b.click(-23.0, -12.0);
    assert_eq!(
        b.session.prompt().text().split(" [").next(),
        Some("Parçala: parça sayısını yazın (Enter: 4 parça)")
    );
    let p = preview(&b);
    assert_eq!(p.strokes.len(), 4, "the pieces at the kept count");
    assert_eq!(p.tag.expect("a tag").lines, ["4 parça"]);
    // A typed count cuts at once, and stays.
    let before = b.doc.len();
    assert!(b.type_text("5"));
    assert_eq!(b.last_text(), Some("Nesne 5 eşit parçaya bölündü."));
    assert_eq!(b.memory.split_parts, 5);
    assert_eq!(b.doc.len(), before + 4);
    assert_eq!(ends(&b, Slot(11)), [[-28.0, -12.0], [-26.0, -12.0]]);
    assert_eq!(
        b.session.tool_id(),
        "split",
        "the tool waits for the next object"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Parçala"));
    assert_eq!(b.doc.len(), before);
    // Enter cuts with the kept count; with none picked, Enter leaves.
    b.click(-23.0, -12.0);
    b.confirm();
    assert_eq!(b.last_text(), Some("Nesne 5 eşit parçaya bölündü."));
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn a_bad_count_or_a_locked_object_is_refused() {
    let mut b = bench();
    b.start("split");
    assert!(b.type_text("e"));
    let rev = b.doc.revision();
    b.click(8.0, -15.0);
    assert_eq!(
        b.last_text(),
        Some(
            "Bölünecek bir çizgi, açık çoklu çizgi, yay ya da daireye tıklayın; alanları önce Patlat ile çizgilere ayırın."
        ),
        "the locked line is not picked"
    );
    b.click(-23.0, -12.0);
    assert!(b.type_text("1"));
    assert_eq!(
        b.last_text(),
        Some("Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.")
    );
    assert!(b.type_text("2.5"));
    assert_eq!(b.memory.split_parts, 4, "the kept count stays");
    assert_eq!(b.doc.revision(), rev);
    // Esc drops the picked object and the tool stays; then Esc leaves.
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.session.tool_id(), "split");
    assert!(!b.run(|s, cx| s.cancel(cx)));
}

#[test]
fn by_length_measures_from_the_end_nearer_the_click() {
    let mut b = bench();
    b.start("split");
    assert!(b.type_text("U"));
    assert!(b.session.prompt().text().contains("[uzunluk 10.000 m;"));
    // Near the east end of the 10 m line: 3 m pieces from there, 1 m left at the west.
    b.click(-19.0, -12.0);
    assert_eq!(
        b.session.prompt().text().split(" [").next(),
        Some("Parçala: parça uzunluğunu yazın (Enter: 10.000 m)")
    );
    // A length as long as the object leaves it whole: said, and the object stays picked.
    let rev = b.doc.revision();
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Uzunluk (10.000 m) nesnenin boyundan (10.000 m) kısa olmalı; nesne bölünmedi.")
    );
    assert_eq!(b.doc.revision(), rev);
    // The measuring end is ringed, the pieces are drawn.
    b.move_to(-10.0, 0.0);
    let p = preview(&b);
    assert!(p.markers.iter().any(|m| m.tone == Tone::Snap));
    assert!(b.type_text("3"));
    assert_eq!(b.memory.split_length, 3.0);
    assert_eq!(
        b.last_text(),
        Some("Nesne 4 parçaya bölündü: 3.000 m uzunluğunda parçalar, kalan son parçada.")
    );
    assert_eq!(ends(&b, Slot(11)), [[-28.0, -12.0], [-27.0, -12.0]]);
    let lengths: Vec<f64> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Line(l) if l.base.id >= 14 => Some(length([rel(l.a), rel(l.b)])),
            _ => None,
        })
        .collect();
    assert_eq!(lengths.len(), 3);
    assert!(lengths.iter().all(|l| (l - 3.0).abs() < 1e-9));
    assert_eq!(b.doc.undo().as_deref(), Some("Parçala"));
}

#[test]
fn a_method_letter_works_over_a_selection_too() {
    let mut b = bench();
    b.selection.set([Slot(1), Slot(2)]);
    b.start("split");
    assert!(b.type_text("E"));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Parçala: bölünecek nesneye tıklayın")
    );
    // K goes back to the crossings of the selection.
    assert!(b.type_text("K"));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Parçala: Enter ya da sağ tıkla")
    );
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("2 nesne kesişim yerlerinden 4 parçaya bölündü.")
    );
}

// ── Çizimi temizle ──────────────────────────────────────────────────────────

/// A drawing with a repeated line, an empty one and a path that repeats a vertex.
fn untidy() -> Bench {
    let mut b = Bench::on(EMPTY);
    b.draft.snap = false;
    b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    b.add_line("cizim", [0.0, 5.0], [10.0, 5.0]);
    b.add_line("cizim", [3.0, 3.0], [3.0, 3.0]);
    b.add_path(
        "cizim",
        &[[0.0, 10.0], [5.0, 10.0], [5.0, 10.0], [10.0, 10.0]],
        false,
    );
    b
}

#[test]
fn cleanup_shows_what_it_found_then_cleans_in_one_step() {
    let mut b = untidy();
    let before = b.doc.len();
    b.start("cleanup");
    assert_eq!(
        b.last_text(),
        Some(
            "Çizimi temizle: 1 yinelenen, 1 boş nesne, 1 tekrarlanan köşe. Enter ile temizleyin (bütün çizim, 5 nesne)."
        )
    );
    assert_eq!(
        b.session.prompt().text(),
        "Çizimi temizle: 1 yinelenen, 1 boş nesne, 1 tekrarlanan köşe [Temizle (Enter)]"
    );
    // The repeat dashed in danger, a cross at the empty line and the repeated vertex.
    let p = preview(&b);
    assert!(
        p.strokes
            .iter()
            .any(|s| s.tone == Tone::Danger && s.dash.is_some())
    );
    assert_eq!(p.markers.len(), 2);
    assert_eq!(b.doc.len(), before, "nothing is written before the confirm");
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Çizimi temizle: 1 yinelenen, 1 boş nesne silindi; 1 tekrarlanan köşe atıldı.")
    );
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.len(), before - 2);
    // The first of the repeats stays: slot 1. The path lost its repeated vertex.
    assert!(b.doc.get(Slot(1)).is_some());
    assert!(b.doc.get(Slot(2)).is_none());
    assert_eq!(b.path_pts(Slot(5)).len(), 3);
    assert_eq!(b.doc.undo().as_deref(), Some("Çizimi temizle"));
    assert_eq!(b.doc.len(), before);
    assert_eq!(b.path_pts(Slot(5)).len(), 4);
}

#[test]
fn cleanup_of_a_selection_looks_only_at_it() {
    let mut b = untidy();
    b.selection.set([Slot(1), Slot(2)]);
    b.start("cleanup");
    assert_eq!(
        b.last_text(),
        Some("Çizimi temizle: 1 yinelenen. Enter ile temizleyin (seçili 2 nesne).")
    );
    b.confirm();
    assert_eq!(b.last_text(), Some("Çizimi temizle: 1 yinelenen silindi."));
    assert!(
        b.doc.get(Slot(4)).is_some(),
        "the empty line was not looked at"
    );
}

#[test]
fn a_clean_drawing_is_said_and_the_tool_leaves() {
    let mut b = Bench::on(EMPTY);
    b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    b.add_line("cizim", [0.0, 5.0], [10.0, 5.0]);
    let rev = b.doc.revision();
    b.start("cleanup");
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(
        b.last_text(),
        Some(
            "Çizimi temizle: yinelenen ya da boş nesne, tekrarlanan köşe bulunamadı; çizim temiz."
        )
    );
    assert_eq!(b.doc.revision(), rev);
}

#[test]
fn cleanup_leaves_a_locked_layer_alone_and_repeats_on_another_layer_are_not_repeats() {
    let mut b = bench();
    b.add_line("kilitli", [0.0, 40.0], [10.0, 40.0]);
    b.add_line("kilitli", [0.0, 40.0], [10.0, 40.0]);
    // The same line on two layers is two meanings.
    b.add_line("cizim", [0.0, 45.0], [10.0, 45.0]);
    b.add_line("parsel", [0.0, 45.0], [10.0, 45.0]);
    let rev = b.doc.revision();
    b.start("cleanup");
    assert_eq!(
        b.last_text(),
        Some(
            "Çizimi temizle: yinelenen ya da boş nesne, tekrarlanan köşe bulunamadı; çizim temiz."
        )
    );
    assert!(
        b.log
            .iter()
            .any(|l| l.text
                == "3 nesne kilitli ya da gizli katmanda olduğu için dışarıda bırakıldı.")
    );
    assert_eq!(b.doc.revision(), rev);
}

// ── Özellik kopyala ─────────────────────────────────────────────────────────

/// A line on the layer `parsel` with its own colour, weight and symbol.
fn styled(b: &mut Bench, at: [f64; 2], to: [f64; 2]) -> Slot {
    let line = Entity::Line(LineEntity {
        base: EntityBase {
            color: Some("#3E63DD".to_owned()),
            line_weight: Some(0.7),
            symbol: Some("agac".to_owned()),
            attrs: [("Ad".to_owned(), "Kaynak".to_owned())].into(),
            ..common::base("parsel")
        },
        a: Wire {
            x: E + at[0],
            y: N + at[1],
        },
        b: Wire {
            x: E + to[0],
            y: N + to[1],
        },
        za: None,
        zb: None,
    });
    b.doc.add(line).expect("a slot")
}

#[test]
fn match_properties_gives_layer_colour_weight_and_symbol_not_the_data() {
    let mut b = bench();
    let source = styled(&mut b, [0.0, 40.0], [10.0, 40.0]);
    b.start("matchProperties");
    assert_eq!(
        b.session.prompt().text(),
        "Özellik kopyala: özellikleri alınacak nesneye tıklayın"
    );
    b.click(5.0, 40.0);
    assert_eq!(
        b.last_text(),
        Some(
            "Kaynak: “Parsel” katmanındaki nesne. Özellikleri verilecek nesnelere tıklayın ya da pencereyle seçin."
        )
    );
    assert_eq!(
        b.session.prompt().text(),
        "Özellik kopyala: özellik verilecek nesnelere tıklayın ya da pencereyle seçin [kaynak: “Parsel” katmanı; Bitir (Enter)]"
    );
    b.move_to(20.0, 40.0);
    let p = preview(&b);
    assert_eq!(
        p.tag.expect("a tag").lines,
        [
            "Katman: Parsel",
            "Renk: #3E63DD",
            "Kalınlık: 0.700 mm",
            "Sembol: agac"
        ]
    );
    assert!(
        p.strokes.iter().any(|s| s.tone == Tone::Snap),
        "the source is ringed"
    );
    // A click on the line 12 (on Çizim, no data of its own): one step.
    b.click(-18.0, -18.0);
    assert_eq!(
        b.last_text(),
        Some("Özellikler 1 nesneye kopyalandı: “Parsel” katmanı, renk, kalınlık, sembol.")
    );
    let Some(target) = b.doc.get(Slot(12)) else {
        panic!("the line");
    };
    let base = target.base();
    assert_eq!(base.layer_id, "parsel");
    assert_eq!(base.color.as_deref(), Some("#3E63DD"));
    assert_eq!(base.line_weight, Some(0.7));
    assert_eq!(base.symbol.as_deref(), Some("agac"));
    assert!(base.attrs.is_empty(), "the attributes are not copied");
    assert_eq!(
        b.session.tool_id(),
        "matchProperties",
        "the tool waits for the next target"
    );
    // A window is one step for everything in it: the lines 1 and 2, left to right.
    b.drag([-30.0, 4.0], [-6.0, 20.0]);
    assert_eq!(
        b.last_text(),
        Some("Özellikler 2 nesneye kopyalandı: “Parsel” katmanı, renk, kalınlık, sembol.")
    );
    for slot in [1, 2] {
        assert_eq!(
            b.doc.get(Slot(slot)).map(|e| e.base().layer_id.as_str()),
            Some("parsel")
        );
    }
    // The same again changes nothing and says so.
    b.click(-18.0, -18.0);
    assert_eq!(
        b.last_text(),
        Some("Nesneler zaten kaynağın katmanında, renginde, kalınlığında ve sembolünde.")
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Özellik kopyala"));
    assert_eq!(
        b.doc.get(Slot(1)).map(|e| e.base().layer_id.as_str()),
        Some("cizim")
    );
    assert_eq!(
        b.doc.get(Slot(12)).map(|e| e.base().layer_id.as_str()),
        Some("parsel")
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Özellik kopyala"));
    assert_eq!(
        b.doc.get(Slot(12)).map(|e| e.base().layer_id.as_str()),
        Some("cizim")
    );
    assert!(source.0 > 13);
    // Enter ends the tool.
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn match_properties_refuses_a_locked_target_and_esc_goes_back_to_the_source() {
    let mut b = bench();
    styled(&mut b, [0.0, 40.0], [10.0, 40.0]);
    b.start("matchProperties");
    // A click on nothing says what it wants.
    b.click(30.0, 30.0);
    assert_eq!(
        b.last_text(),
        Some("Özellikleri alınacak bir nesneye tıklayın.")
    );
    b.click(5.0, 40.0);
    let rev = b.doc.revision();
    // The locked line 13: the command's words.
    b.click(8.0, -15.0);
    assert!(b.last_text().expect("said").contains("kilitli"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.revision(), rev);
    // Esc drops the source, the tool stays; the next Esc leaves.
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(
        b.session.prompt().text(),
        "Özellik kopyala: özellikleri alınacak nesneye tıklayın"
    );
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.session.tool_id(), "select");
}

/// Çizimi temizle over a big drawing of distinct objects (the core compares
/// each with the ones kept): run by hand with `--release -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run by hand"]
fn cleanup_time_on_a_big_drawing() {
    for n in [5_000usize, 20_000] {
        let mut b = Bench::on(EMPTY);
        for i in 0..n {
            let x = (i % 200) as f64;
            let y = (i / 200) as f64 * 0.5;
            b.add_line("cizim", [x, y], [x + 0.4, y]);
        }
        let t = std::time::Instant::now();
        b.start("cleanup");
        println!("{n} nesne: {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    }
}
