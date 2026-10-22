use kentos_sheet::{
    *,
    plot_profile::{self, apply},
    display::{self, Prim, RenderInputs, RenderMode},
    ops::{self, Op},
};
#[test]
fn twenty_ink_profiles_apply_to_paper_and_map_without_touching_the_source() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-ink.json"
    ))
    .unwrap();
    for c in data["cases"].as_array().unwrap() {
        let profile: plot_profile::PlotProfile =
            serde_json::from_value(c["profile"].clone()).unwrap();
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
        book = ops::apply_all(
            &book,
            &[
                Op::SavePlotProfiles(plot_profile::SavePlotProfiles {
                    profiles: vec![profile.clone()],
                }),
                Op::SavePageSetups(print_setup::SavePageSetups {
                    setups: vec![serde_json::from_value(c["setup"].clone()).unwrap()],
                }),
                Op::ApplyPageSetup(print_setup::ApplyPageSetup {
                    sheet: "page".into(),
                    setup: "office".into(),
                }),
            ],
        )
        .unwrap()
        .book;
        let before = book.clone();
        let list = display::display_list(
            &book,
            "page",
            &RenderInputs {
                mode: RenderMode::Export,
                ..Default::default()
            },
        )
        .unwrap();
        let Prim::Rect(r) = list
            .prims
            .iter()
            .find(|p| matches!(p,Prim::Rect(r) if r.item=="box"))
            .unwrap()
        else {
            panic!()
        };
        let expected = c["expected"]["boxColor"].as_str().unwrap();
        let actual = r.fill.as_deref().unwrap();
        assert_eq!(&actual[..7], &expected[..7]);
        let alpha = profile.entries[0].style.opacity.unwrap_or(100);
        let group = list
            .prims
            .iter()
            .find_map(|p| match p {
                Prim::PushGroup(g) if g.item == "box" => Some(g.opacity),
                _ => None,
            })
            .unwrap_or(100);
        assert_eq!(group, alpha);
        let pen = r.stroke.as_ref().unwrap();
        assert_eq!(
            f64::from(pen.width) / 1000.,
            c["expected"]["widthMm"].as_f64().unwrap()
        );
        assert_eq!(serde_json::to_value(pen.cap).unwrap(), c["expected"]["cap"]);
        assert_eq!(
            serde_json::to_value(pen.join).unwrap(),
            c["expected"]["join"]
        );
        assert_eq!(
            apply::color("#ffffff", &profile.entries[0].style),
            "#ffffff"
        );
        assert_eq!(apply::color("#😊aa", &profile.entries[0].style), "#😊aa");
        let mut content = pdf::MapContent::Vector(pdf::VectorMap {
            layers: vec![pdf::MapLayerContent {
                id: "cizim".into(),
                name: "Çizim".into(),
                paths: vec![pdf::MapPath {
                    points: vec![[0., 0.], [5., 0.]],
                    closed: false,
                    holes: vec![],
                    fill: None,
                    stroke: Some(pdf::MapStroke {
                        color: "#000000".into(),
                        width: 0.25,
                        dash: vec![],
                        cap: LineCap::Round,
                        join: LineJoin::Round,
                    }),
                }],
                texts: vec![],
            }],
        });
        apply::map(&mut content, &profile);
        let pdf::MapContent::Vector(v) = content else {
            panic!()
        };
        let pen = v.layers[0].paths[0].stroke.as_ref().unwrap();
        assert_eq!(pen.color, c["expected"]["modelColor"].as_str().unwrap());
        assert_eq!(pen.width, c["expected"]["widthMm"].as_f64().unwrap());
        assert_eq!(
            serde_json::to_value(&pen.dash).unwrap(),
            c["expected"]["dashMm"]
        );
        assert_eq!(book, before);
    }
}

#[test]
fn plot_opacity_replaces_own_transparency_but_preserves_parent_groups() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-ink.json"
    ))
    .unwrap();
    let c = &data["cases"][0];
    for alpha in [0, 5, 50, 100] {
        let mut profile: plot_profile::PlotProfile =
            serde_json::from_value(c["profile"].clone()).unwrap();
        profile.entries[0].style.opacity = Some(alpha);
        let mut book = SheetBook::default();
        let mut sheet: Sheet = serde_json::from_value(c["sheet"].clone()).unwrap();
        sheet.items[0].opacity = 20;
        sheet.items[0].group = Some("parent".into());
        if let ItemKind::Shape(s) = &mut sheet.items[0].kind {
            s.fill = Some("#2255aa40".into());
        }
        let mut group = Item::new(
            "parent",
            "Üst grup",
            RectUm::new(0, 0, 1000, 1000),
            ItemKind::Group(GroupItem {}),
        );
        group.opacity = 60;
        sheet.items.push(group);
        book.sheets.push(sheet);
        book.plot_profiles.push(profile.clone());
        book.sheets[0].plot = Some(print_setup::AppliedPageSetup::capture(
            &book,
            None,
            print_setup::PlotSettings {
                profile: Some(profile.id),
                ..Default::default()
            },
        ));
        let before = book.clone();
        let list = display::display_list(
            &book,
            "page",
            &RenderInputs {
                mode: RenderMode::Export,
                ..Default::default()
            },
        )
        .unwrap();
        let opacity = list
            .prims
            .iter()
            .find_map(|p| match p {
                Prim::PushGroup(g) if g.item == "box" => Some(g.opacity),
                _ => None,
            })
            .unwrap_or(100);
        assert_eq!(opacity, (u16::from(alpha) * 60 / 100) as u8);
        let fill = list
            .prims
            .iter()
            .find_map(|p| match p {
                Prim::Rect(r) if r.item == "box" => r.fill.as_deref(),
                _ => None,
            })
            .unwrap();
        assert_eq!(fill.len(), 7, "alpha must be owned by the one item group");
        assert_eq!(book, before);
    }
}
