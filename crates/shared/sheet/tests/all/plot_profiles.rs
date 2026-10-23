use kentos_sheet::{
    *,
    ops::{self, Op},
    plot_profile::*,
};
fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-profiles.json"
    ))
    .unwrap()
}
#[test]
fn twenty_native_tables_roundtrip_without_merging_key_spaces_and_undo_exactly() {
    for c in cases()["cases"].as_array().unwrap() {
        let p = read(&serde_json::to_vec(&c["exchange"]).unwrap()).unwrap();
        let bytes = write(&p).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            c["exchange"]
        );
        assert_eq!(
            serde_json::to_value(&p.entries[0].key).unwrap(),
            c["expected"]["key"]
        );
        assert_eq!(
            p.style(&Target::ModelLayer { id: "cizim".into() }),
            Some(&p.entries[0].style)
        );
        assert_eq!(
            p.style(&Target::PaperItem { id: "other".into() }).is_some(),
            c["expected"]["unassigned"].as_bool().unwrap()
        );
        let b = SheetBook::default();
        let a = ops::apply(
            &b,
            &Op::SavePlotProfiles(SavePlotProfiles { profiles: vec![p] }),
        )
        .unwrap();
        assert_eq!(ops::apply_all(&a.book, &a.inverse).unwrap().book, b);
    }
}
#[test]
fn mixed_keys_normal_aci_zero_unknown_attributes_and_binary_tables_are_explicit_refusals() {
    let mut p = read(&serde_json::to_vec(&cases()["cases"][0]["exchange"]).unwrap()).unwrap();
    p.entries[0].key = StyleKey::Named { name: "1".into() };
    assert_eq!(validate(&p).unwrap_err().code, "plot_style_key_mismatch");
    p.entries[0].key = StyleKey::Aci { index: 0 };
    assert!(validate(&p).is_err());
    p.kind = TableKind::Named;
    p.entries[0].key = StyleKey::Named {
        name: "Normal".into(),
    };
    assert!(validate(&p).is_err());
    assert_eq!(
        read(b"PIAFILEVERSION_2.0,CTBVER1,compress")
            .unwrap_err()
            .code,
        "unsupported_plot_profile"
    );
    let mut x = cases()["cases"][10]["exchange"].clone();
    x["profile"]["entries"][0]["style"]["dither"] = serde_json::json!(true);
    assert!(read(&serde_json::to_vec(&x).unwrap()).is_err());
    assert_eq!(PlotStyle::default().screening, 100);
    assert_eq!(
        PlotStyle::default(),
        serde_json::from_str::<PlotStyle>("{}").unwrap()
    );
}

#[test]
fn libraries_survive_no_sheets_transfer_preserves_source_keys_and_snapshots_pin_profiles() {
    for c in cases()["cases"].as_array().unwrap() {
        let mut profile = read(&serde_json::to_vec(&c["exchange"]).unwrap()).unwrap();
        profile.id = "box".into();
        profile.assignments[0].target = Target::ModelLayer { id: "box".into() };
        let mut book = SheetBook::default();
        book.plot_profiles.push(profile.clone());
        assert!(project::project_sheets(&book, |_| None).unwrap().is_some());
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../fixtures/sheet/v1/page-setups.json"
        ))
        .unwrap();
        let page: Sheet = serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap();
        book.sheets.push(page);
        if let ItemKind::Text(t) = &mut book.sheets[0].items[1].kind {
            t.content = "box".into();
        }
        let settings = print_setup::PlotSettings {
            profile: Some(profile.id.clone()),
            ..Default::default()
        };
        book.sheets[0].plot = Some(print_setup::AppliedPageSetup::capture(
            &book, None, settings,
        ));
        let mut n = 0;
        let copy = book_transfer::renew_ids(&book, || {
            n += 1;
            format!("fresh-{n}")
        });
        validate::validate_book(&copy).unwrap();
        let p = plot_profile::for_sheet(&copy, &copy.sheets[0].id)
            .unwrap()
            .unwrap();
        assert_eq!(
            p.assignments[0].target,
            Target::ModelLayer { id: "box".into() }
        );
        assert_eq!(
            p.assignments[1].target,
            Target::PaperItem {
                id: copy.sheets[0].items[0].id.clone()
            }
        );
        assert_eq!(copy.sheets[0].items[1].kind, book.sheets[0].items[1].kind);
        book.plot_profiles.clear();
        assert_eq!(
            plot_profile::for_sheet(&book, "page").unwrap(),
            Some(&profile)
        );
        let meta = template::system_templates()[0].meta.clone();
        let t = template::extract(
            &book,
            "page",
            &template::TemplateMeta {
                id: "my-print-template".into(),
                ..meta
            },
            &[],
        )
        .unwrap();
        assert!(t.sheet.plot.as_ref().unwrap().settings.profile_snapshot.is_some());
        let inst = template::instantiate(
            &t,
            &template::InstanceIds {
                sheet: "new-sheet".into(),
                items: vec!["new-box".into(), "new-text".into()],
                master: None,
                master_items: Vec::new(),
            },
            &template::InstanceOptions::default(),
        )
        .unwrap();
        let mut out = SheetBook::default();
        out.sheets.push(inst.sheet);
        let p = plot_profile::for_sheet(&out, "new-sheet").unwrap().unwrap();
        assert_eq!(
            p.assignments[1].target,
            Target::PaperItem {
                id: "new-box".into()
            }
        );
    }
    let case: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/page-setups.json"
    ))
    .unwrap();
    let mut b = SheetBook::default();
    b.page_setups
        .push(serde_json::from_value(case["cases"][0]["setup"].clone()).unwrap());
    assert!(project::project_sheets(&b, |_| None).unwrap().is_some());
}
