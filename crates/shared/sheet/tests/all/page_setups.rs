use kentos_sheet::{
    *,
    ops::{self, Op},
    print_setup::*,
};
fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/page-setups.json"
    ))
    .unwrap()
}
#[test]
fn twenty_named_setups_apply_snapshot_paper_geometry_and_undo_exactly() {
    for c in cases()["cases"].as_array().unwrap() {
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
        let before = book.clone();
        let setup: NamedPageSetup = serde_json::from_value(c["setup"].clone()).unwrap();
        let done = ops::apply_all(
            &book,
            &[
                Op::SavePageSetups(SavePageSetups {
                    setups: vec![setup.clone()],
                }),
                Op::ApplyPageSetup(ApplyPageSetup {
                    sheet: "page".into(),
                    setup: "office".into(),
                }),
            ],
        )
        .unwrap();
        let p = plan_for(&done.book, "page").unwrap().unwrap();
        let expected = &c["expected"];
        assert!((p.factor - expected["factor"].as_f64().unwrap()).abs() < 1e-10);
        for (i, v) in p.translate.iter().enumerate() {
            assert!(
                (v - expected["translate"][i].as_f64().unwrap()).abs() < 1e-7,
                "{}",
                c["name"]
            );
        }
        for (i, v) in [p.source.left, p.source.top, p.source.width, p.source.height]
            .iter()
            .enumerate()
        {
            assert_eq!(i64::from(*v), expected["source"][i].as_i64().unwrap());
        }
        let stripped = ops::apply(
            &done.book,
            &Op::SavePageSetups(SavePageSetups { setups: vec![] }),
        )
        .unwrap();
        assert_eq!(stripped.book.sheets[0].plot, done.book.sheets[0].plot);
        assert_eq!(plan_for(&stripped.book, "page").unwrap(), Some(p));
        assert_eq!(
            ops::apply_all(&done.book, &done.inverse).unwrap().book,
            before
        );
        let bytes = serde_json::to_vec(&done.book).unwrap();
        assert_eq!(
            serde_json::from_slice::<SheetBook>(&bytes).unwrap(),
            done.book
        );
    }
}
#[test]
fn invalid_setups_refuse_without_mutation_and_a_missing_profile_never_prints_silently() {
    let c = &cases()["cases"][0];
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(c["sheet"].clone()).unwrap());
    let mut setup: NamedPageSetup = serde_json::from_value(c["setup"].clone()).unwrap();
    for bad in [0., -1., f64::INFINITY, 1e7] {
        setup.settings.scale = PlotScale::Ratio {
            ratio: viewport::ScaleRatio {
                paper: bad,
                model: 1.,
            },
        };
        assert!(
            ops::apply(
                &book,
                &Op::SavePageSetups(SavePageSetups {
                    setups: vec![setup.clone()]
                })
            )
            .is_err()
        );
    }
    setup.settings = PlotSettings::default();
    let op = Op::SavePageSetups(SavePageSetups {
        setups: vec![setup.clone(), setup.clone()],
    });
    assert!(ops::apply(&book, &op).is_err());
    setup.settings.profile = Some("missing".into());
    book.sheets[0].plot = Some(AppliedPageSetup {
        source: None,
        settings: setup.settings,
    });
    assert_eq!(
        plan_for(&book, "page").unwrap_err().code,
        "unknown_plot_profile"
    );
}
