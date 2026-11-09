//! Ruhsat takımı (docs/adr/3037): a phase's set from the MO sheet in MO's order, numbered on its code, each sheet
//! answering the digital project regulation's file name fields; the first sheet holds PAİY 57(10)'s table and the set's
//! live list instead of a map; the storeys' codes and the paper follow EK-2 and the maps' sizes.

use kentos_sheet::permit::{self, Page, Phase};
use kentos_sheet::template::{PaperChoice, system_template};
use kentos_sheet::{ItemKind, MapLevel, Orientation, Paper, SheetBook, VarValue, create, ops};

fn answer<'a>(sheet: &'a kentos_sheet::Sheet, name: &str) -> Option<&'a VarValue> {
    sheet
        .variables
        .iter()
        .find(|v| v.name == name)
        .map(|v| &v.value)
}

#[test]
fn storeys_codes_and_papers_follow_ek_2_and_the_maps() {
    let levels = [
        ("zemin".to_owned(), 0.0),
        ("bodrum".to_owned(), -3.0),
        ("k1".to_owned(), 3.0),
        ("k2".to_owned(), 6.0),
        ("bodrum2".to_owned(), -6.0),
    ];
    let codes = permit::level_codes(&levels);
    let code = |id: &str| codes.iter().find(|(l, _)| l == id).map(|(_, c)| c.as_str());
    assert_eq!(
        [
            code("zemin"),
            code("k1"),
            code("k2"),
            code("bodrum"),
            code("bodrum2")
        ],
        [
            Some("K00"),
            Some("K01"),
            Some("K02"),
            Some("B01"),
            Some("B02")
        ]
    );
    let a3 = PaperChoice {
        paper: Paper::A3,
        orientation: Orientation::Landscape,
    };
    assert_eq!(permit::paper_for(&[[110.0, 80.0]]), Some(a3));
    // 400 mm wide does not fit A2's 364 mm map: A1's 611.
    assert_eq!(
        permit::paper_for(&[[110.0, 80.0], [400.0, 300.0]]).map(|p| p.paper),
        Some(Paper::A1)
    );
    assert_eq!(permit::paper_for(&[[1000.0, 800.0]]), None);
    assert_eq!((Phase::Permit.code(), Phase::Permit.scale()), ("UP", 100));
    assert_eq!(
        (Phase::Application.code(), Phase::Application.scale()),
        ("UP", 50)
    );
    assert_eq!(Phase::Preliminary.code(), "ÖP");
}

#[test]
fn a_permit_set_is_made_in_mo_s_order_with_its_file_names_fields() {
    let template = system_template(permit::TEMPLATE).expect("the MO sheet");
    let paper = PaperChoice {
        paper: Paper::A3,
        orientation: Orientation::Landscape,
    };
    let at = |x: f64, y: f64| Some(kentos_sheet::GroundPoint { x, y });
    let pages = [
        Page::info(),
        Page::site(Some("zemin".into()), at(5.0, 4.0), 500),
        Page::plan("zemin", "Zemin kat", "K00", at(5.5, 4.0), 100),
        Page::plan("k1", "1. kat", "K01", at(5.5, 4.0), 100),
    ];
    let made = permit::permit_sheets(
        &SheetBook::default(),
        template,
        Phase::Permit,
        paper,
        &pages,
        true,
        |s| create::ids_for(template, s),
    )
    .unwrap();
    let numbers: Vec<&str> = made.made.iter().map(|m| m.number.as_str()).collect();
    assert_eq!(numbers, ["UP-01", "UP-02", "UP-03", "UP-04"]);
    let book = made.ops.iter().fold(SheetBook::default(), |b, op| {
        ops::apply(&b, op).unwrap().book
    });
    assert_eq!(book.sheets.len(), 4);
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "UP-01 Proje bilgileri",
            "UP-02 Vaziyet planı",
            "UP-03 Zemin kat planı",
            "UP-04 1. kat planı"
        ]
    );
    for s in &book.sheets {
        assert_eq!(s.page.paper, Paper::A3);
        let info = s.drawing_info.as_ref().unwrap();
        assert_eq!(info.subset, ["Mimari", "Ruhsat takımı"]);
        assert_eq!(
            answer(s, "notlar"),
            Some(&VarValue::Null),
            "the drawing's notes, not the template's"
        );
    }
    let text = |s: &str| Some(VarValue::Text(s.into()));
    let fields = |i: usize| {
        let s = &book.sheets[i];
        (
            answer(s, "dosya_turu").cloned(),
            answer(s, "dosya_tipi").cloned(),
            answer(s, "kat_konum").cloned(),
        )
    };
    assert_eq!(fields(0), (text("GN"), text("MMC"), text("NNN")));
    assert_eq!(fields(1), (text("PL"), text("VAZ"), text("NNN")));
    assert_eq!(fields(2), (text("PL"), text("MMC"), text("K00")));
    assert_eq!(fields(3), (text("PL"), text("MMC"), text("K01")));
    assert!(
        book.sheets
            .iter()
            .all(|s| answer(s, "sira_no") == text("01").as_ref())
    );
    // The first sheet: no map, no north arrow; PAİY 57(10)'s table and the set's list.
    let kinds = |i: usize| -> Vec<&'static str> {
        book.sheets[i]
            .items
            .iter()
            .map(|it| it.kind.type_name())
            .collect()
    };
    assert!(
        !kinds(0).contains(&"map") && !kinds(0).contains(&"northArrow"),
        "{:?}",
        kinds(0)
    );
    assert!(kinds(0).contains(&"table"));
    assert!(
        kinds(1).contains(&"northArrow"),
        "a georeferenced project's site plan has its north"
    );
    // The plans' maps draw their storeys at the phase's scale; the site plan the ground storey at 1/500.
    let map = |i: usize| {
        book.sheets[i]
            .items
            .iter()
            .find_map(|it| match &it.kind {
                ItemKind::Map(m) => Some(m.clone()),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(
        map(1).level,
        Some(MapLevel {
            id: "zemin".into(),
            below: false
        })
    );
    assert_eq!(
        map(3).level,
        Some(MapLevel {
            id: "k1".into(),
            below: false
        })
    );
    let scale = |i: usize| match &map(i).view {
        kentos_sheet::MapView::Fixed(f) => f.scale,
        other => panic!("{other:?}"),
    };
    assert_eq!((scale(1), scale(2)), (500, 100));
    assert!(
        map(1).view_locked && map(3).view_locked,
        "the phase's scale is locked"
    );
    // Made again over the first set: the numbers go on, the ids do not clash.
    let again = permit::permit_sheets(
        &book,
        template,
        Phase::Permit,
        paper,
        &pages[..1],
        false,
        |s| create::ids_for(template, s),
    )
    .unwrap();
    assert_eq!(again.made[0].number, "UP-05");
    // A project with no coordinate system: no north arrow (the CAD profile hides it).
    let local = permit::permit_sheets(
        &SheetBook::default(),
        template,
        Phase::Permit,
        paper,
        &pages[1..2],
        false,
        |s| create::ids_for(template, s),
    )
    .unwrap();
    let local = local.ops.iter().fold(SheetBook::default(), |b, op| {
        ops::apply(&b, op).unwrap().book
    });
    assert!(
        local.sheets[0]
            .items
            .iter()
            .all(|i| i.kind.type_name() != "northArrow")
    );
}

/// The calculation sheet (EK-2 2.8.1, “Mimari hesap raporu”): a locked map on each table at the plot scale, its size the
/// table's on paper, a caption over it, from the left in rows; EK-2's HS, MIR, NNN.
#[test]
fn the_calculation_sheet_shows_each_table_at_the_plot_scale() {
    // A row while the width holds: 80 + 8 + 80 < 190 mm (A3's room), the third under them.
    let places = permit::shelf(Paper::A3, &[[80.0, 50.0], [80.0, 50.0], [80.0, 40.0]]).unwrap();
    assert_eq!(places, [[30.0, 26.0], [118.0, 26.0], [30.0, 90.0]]);
    assert_eq!(
        permit::shelf(Paper::A3, &[[200.0, 10.0]]),
        None,
        "wider than the room"
    );
    assert_eq!(
        permit::shelf(Paper::A3, &[[50.0, 300.0]]),
        None,
        "taller than the room"
    );
    let template = system_template(permit::TEMPLATE).expect("the MO sheet");
    let paper = PaperChoice {
        paper: Paper::A3,
        orientation: Orientation::Landscape,
    };
    let page = Page::tables(
        vec![
            ("Otopark hesabı".into(), [20.0, 0.0, 28.0, 5.0]),
            ("Sığınak hesabı".into(), [20.0, -7.0, 28.0, -2.0]),
        ],
        100,
    );
    let made = permit::permit_sheets(
        &SheetBook::default(),
        template,
        Phase::Permit,
        paper,
        &[page],
        true,
        |s| create::ids_for(template, s),
    )
    .unwrap();
    assert_eq!(made.made[0].title, "Mimari hesap raporu");
    let book = made.ops.iter().fold(SheetBook::default(), |b, op| {
        ops::apply(&b, op).unwrap().book
    });
    let s = &book.sheets[0];
    let text = |v: &str| Some(VarValue::Text(v.into()));
    assert_eq!(
        (
            answer(s, "dosya_turu").cloned(),
            answer(s, "dosya_tipi").cloned(),
            answer(s, "kat_konum").cloned()
        ),
        (text("HS"), text("MIR"), text("NNN"))
    );
    let maps: Vec<&kentos_sheet::Item> = s
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Map(_)))
        .collect();
    assert_eq!(maps.len(), 2, "a map a table, the template's own gone");
    for (m, centre) in maps.iter().zip([[24.0, 2.5], [24.0, -4.5]]) {
        // 8 × 5 m at 1/100: 80 × 50 mm, 1.5 mm around it (its outer lines are not clipped, docs/adr/3038).
        assert_eq!((m.frame.width, m.frame.height), (83_000, 53_000));
        let ItemKind::Map(map) = &m.kind else {
            unreachable!()
        };
        assert!(map.view_locked && map.level.is_none());
        let kentos_sheet::MapView::Fixed(f) = &map.view else {
            panic!("fixed")
        };
        assert_eq!(f.scale, 100);
        let c = f.center.unwrap();
        assert_eq!([c.x, c.y], centre);
    }
    assert!(s.items.iter().any(
        |i| matches!(&i.kind, ItemKind::Text(t) if t.content == "Sığınak hesabı — Ölçek 1/100")
    ));
    assert!(
        s.items.iter().all(|i| i.kind.type_name() != "northArrow"),
        "no north on a sheet of tables"
    );
}

#[test]
fn a_list_is_a_sheet_of_its_own_paper_without_a_caption_or_a_scale() {
    // Mahal listesi (docs/adr/3038): EK-2's Liste (LS) of kind MHL, on the paper it fits while the set is on A2.
    let template = system_template(permit::TEMPLATE).expect("the MO sheet");
    let a2 = PaperChoice {
        paper: Paper::A2,
        orientation: Orientation::Landscape,
    };
    let a3 = PaperChoice {
        paper: Paper::A3,
        orientation: Orientation::Landscape,
    };
    let pages = [
        Page::info(),
        Page::list(
            "Mahal listesi",
            "MHL",
            vec![("Mahal listesi".into(), [30.0, -10.0, 45.0, 0.0])],
            100,
            a3.clone(),
            false,
        ),
    ];
    let made = permit::permit_sheets(
        &SheetBook::default(),
        template,
        Phase::Permit,
        a2.clone(),
        &pages,
        true,
        |s| create::ids_for(template, s),
    )
    .unwrap();
    assert_eq!(made.made[1].title, "Mahal listesi");
    let book = made.ops.iter().fold(SheetBook::default(), |b, op| {
        ops::apply(&b, op).unwrap().book
    });
    let (first, list) = (&book.sheets[0], &book.sheets[1]);
    assert_eq!(first.page.paper, Paper::A2);
    assert_eq!(list.page.paper, Paper::A3, "its own paper");
    let text = |v: &str| Some(VarValue::Text(v.into()));
    assert_eq!(
        (
            answer(list, "dosya_turu").cloned(),
            answer(list, "dosya_tipi").cloned(),
            answer(list, "olcek").cloned()
        ),
        (text("LS"), text("MHL"), text("—")),
        "a list has no scale"
    );
    let maps: Vec<&kentos_sheet::Item> = list
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Map(_)))
        .collect();
    assert_eq!(maps.len(), 1);
    // 15 × 10 m at 1/100: 150 × 100 mm, 1.5 mm around it.
    assert_eq!(
        (maps[0].frame.width, maps[0].frame.height),
        (153_000, 103_000)
    );
    assert!(
        !list
            .items
            .iter()
            .any(|i| matches!(&i.kind, ItemKind::Text(t) if t.content.contains("Ölçek 1/"))),
        "no caption"
    );
}
