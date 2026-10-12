//! İstatistik and Grafik over `fixtures/interaction/v1/charts.kcad`
//! (docs/adr/0227): the commands open their tabs on the layer at hand, a
//! chart's numbers, a press selecting its element's objects, the
//! selection marking the chart, a layer's other columns choosing the
//! columns again, a tool's table charted, and another drawing starting the
//! choices again; then the painted picture and the saved ones.

use kentos_domain::Slot;
use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Spec};

use super::{ChartTable, Event, Picture, Setting, Source, View};
use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::chart_scenes::{open_charts, parcels_of};

fn opened() -> App {
    let (mut app, _) = App::boot(None);
    open_charts(&mut app);
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Charts(e));
}

/// The chart's table as the Tablo view shows it.
fn table(app: &App) -> Vec<Vec<String>> {
    app.chart_refresh();
    let now = app.charts.chart.borrow();
    let t = now.table.as_ref().expect("a chart");
    let mut lines = vec![t.columns.clone()];
    lines.extend(t.rows.iter().cloned());
    lines
}

#[test]
fn the_commands_open_their_tabs_on_the_layer_at_hand() {
    let mut app = opened();
    let _ = app.update(Message::Run("data.statistics"));
    assert_eq!(app.bottom_tab, BottomTab::Statistics);
    assert!(app.command_expanded);
    assert_eq!(app.charts.layer.as_deref(), Some("parsel"));
    let _ = app.update(Message::Run("chart.pie"));
    assert_eq!(app.bottom_tab, BottomTab::Chart);
    app.chart_refresh();
    assert_eq!(app.charts.chart.borrow().kind, Some(ChartKind::Pie));
    // Pasta by its first column that is no number (Ada is text), counting.
    assert_eq!(table(&app)[0], ["Ada", "Nesne", "Pay (%)"]);
    // A line's points start each a row; another kind starts at the count again.
    let _ = app.update(Message::Run("chart.line"));
    assert_eq!(app.whole_spec().expect("a spec").aggregate, Aggregate::None);
    let _ = app.update(Message::Run("chart.bar"));
    assert_eq!(
        app.whole_spec().expect("a spec").aggregate,
        Aggregate::Count
    );
}

#[test]
fn a_press_selects_its_elements_objects_and_the_selection_marks_the_chart() {
    let mut app = opened();
    app.open_chart(
        Some("parsel".into()),
        None,
        Some(Spec {
            category: Some("Kullanım".into()),
            ..Spec::new(ChartKind::Bar)
        }),
    );
    let lines = table(&app);
    assert_eq!(lines[0], ["Kullanım", "Nesne"]);
    let ticaret = lines
        .iter()
        .skip(1)
        .position(|r| r[0] == "Ticaret")
        .expect("a Ticaret bar");
    assert_eq!(lines[ticaret + 1][1], "6");
    // The bar's press: its six parcels selected.
    ev(&mut app, Event::Press(Some(ticaret as u32)));
    let mut picked: Vec<Slot> = app.selection.ids().to_vec();
    picked.sort();
    let mut expected = parcels_of(&app, "T");
    expected.sort();
    assert_eq!(picked, expected);
    // Shift adds the İşyeri bar; Ctrl takes Ticaret away again.
    let isyeri = lines
        .iter()
        .skip(1)
        .position(|r| r[0] == "İşyeri")
        .expect("an İşyeri bar");
    app.modifiers = iced::keyboard::Modifiers::SHIFT;
    ev(&mut app, Event::Press(Some(isyeri as u32)));
    assert_eq!(app.selection.ids().len(), 6 + 14);
    app.modifiers = iced::keyboard::Modifiers::CTRL;
    ev(&mut app, Event::Press(Some(ticaret as u32)));
    assert_eq!(app.selection.ids().len(), 14);
    // The background clears it.
    app.modifiers = iced::keyboard::Modifiers::empty();
    ev(&mut app, Event::Press(None));
    assert!(app.selection.ids().is_empty());
    // The selection marks the chart: its picture changes.
    app.chart_refresh();
    let before = app.charts.chart.borrow().stamp;
    app.selection.set(parcels_of(&app, "S"));
    app.chart_refresh();
    assert!(app.charts.chart.borrow().stamp > before);
}

#[test]
fn secililer_chart_only_the_selected_objects() {
    let mut app = opened();
    app.open_chart(
        Some("parsel".into()),
        None,
        Some(Spec {
            category: Some("Kullanım".into()),
            ..Spec::new(ChartKind::Bar)
        }),
    );
    ev(&mut app, Event::Show(crate::features::Show::Selected));
    app.selection.set(parcels_of(&app, "S"));
    let lines = table(&app);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[1][..2], ["Sanayi".to_owned(), "3".to_owned()]);
}

#[test]
fn another_layer_keeps_the_kind_and_chooses_its_columns_again() {
    let mut app = opened();
    app.open_chart(
        Some("parsel".into()),
        None,
        Some(Spec {
            x: Some("$alan".into()),
            y: vec!["Değer".into()],
            ..Spec::new(ChartKind::Scatter)
        }),
    );
    ev(&mut app, Event::Layer("yol".into()));
    let spec = app.whole_spec().expect("a spec");
    assert_eq!(spec.kind, ChartKind::Scatter);
    // The roads' numbers: Genişlik and their length (lines have no area: no Alan column).
    assert_eq!(spec.x.as_deref(), Some("Genişlik"));
    assert_eq!(spec.y, vec!["$uzunluk".to_owned()]);
    let keys = app
        .with_chart_data(|d| d.columns.iter().map(|c| c.key.clone()).collect::<Vec<_>>())
        .expect("the roads' data");
    assert_eq!(keys, ["Ad", "Genişlik", "$uzunluk"]);
}

#[test]
fn a_setting_changes_the_spec_as_made_whole() {
    let mut app = opened();
    let _ = app.update(Message::Run("chart.bar"));
    ev(&mut app, Event::Set(Setting::Category("Kullanım".into())));
    ev(&mut app, Event::Set(Setting::Aggregate(Aggregate::Sum)));
    ev(&mut app, Event::Set(Setting::Value("$alan".into())));
    let spec = app.whole_spec().expect("a spec");
    assert_eq!(
        (
            spec.category.as_deref(),
            spec.aggregate,
            spec.value.as_deref()
        ),
        (Some("Kullanım"), Aggregate::Sum, Some("$alan"))
    );
    // Çizgi's Y takes columns one by one.
    let _ = app.update(Message::Run("chart.line"));
    ev(&mut app, Event::Set(Setting::X("Kat".into())));
    ev(&mut app, Event::Set(Setting::YToggle("Emsal".into())));
    let spec = app.whole_spec().expect("a spec");
    assert_eq!(spec.y.len(), 2, "{:?}", spec.y);
    ev(&mut app, Event::View(View::Table));
    assert!(app.chart_table_view());
}

#[test]
fn a_tools_table_opens_with_its_tools_chart_and_selects_nothing() {
    let mut app = opened();
    app.open_table_chart(ChartTable {
        title: "ROC ile doğrulama".into(),
        tool: "suitability.roc".into(),
        columns: vec![
            "Eşik".into(),
            "Doğru pozitif".into(),
            "Doğru pozitif oranı (%)".into(),
            "Hücre".into(),
            "Alan oranı (%)".into(),
        ],
        rows: vec![
            vec![
                "0.9".into(),
                "2".into(),
                "20".into(),
                "10".into(),
                "5".into(),
            ],
            vec![
                "0.5".into(),
                "7".into(),
                "70".into(),
                "40".into(),
                "20".into(),
            ],
            vec![
                "0.1".into(),
                "10".into(),
                "100".into(),
                "200".into(),
                "100".into(),
            ],
        ],
    });
    assert_eq!(app.charts.source, Source::Table);
    let spec = app.whole_spec().expect("a spec");
    assert_eq!(spec.kind, ChartKind::Line);
    assert_eq!(spec.x.as_deref(), Some("Alan oranı (%)"));
    app.chart_refresh();
    ev(&mut app, Event::Press(Some(0)));
    assert!(app.selection.ids().is_empty());
}

#[test]
fn statistics_count_their_summaries_and_a_value_selects_its_objects() {
    let mut app = opened();
    app.open_statistics(Some("parsel".into()));
    assert!(app.stats_counting());
    for _ in 0..64 {
        if !app.stats_counting() {
            break;
        }
        let _ = app.update(Message::Statistics(crate::statistics::Event::Count));
    }
    assert!(!app.stats_counting());
    {
        let now = app.charts.stats.borrow();
        assert_eq!(now.rows.len(), 63);
        let kat = now
            .summaries
            .iter()
            .flatten()
            .find(|s| s.key == "Kat")
            .expect("Kat's summary");
        // Two empty, one refused (“üç”).
        assert_eq!((kat.filled, kat.empty, kat.invalid), (60, 2, 1));
    }
    let _ = app.update(Message::Statistics(crate::statistics::Event::Field(
        "Kullanım".into(),
    )));
    app.stats_refresh();
    let at = app
        .charts
        .stats
        .borrow()
        .detail
        .as_ref()
        .expect("Kullanım's statistics")
        .values
        .iter()
        .position(|v| v.text == "Sanayi")
        .expect("Sanayi");
    let _ = app.update(Message::Statistics(crate::statistics::Event::Value(at)));
    assert_eq!(app.selection.ids().len(), 3);
    // Grafik: the field's bars in the Grafik tab.
    let _ = app.update(Message::Statistics(crate::statistics::Event::Chart));
    assert_eq!(app.bottom_tab, BottomTab::Chart);
    let spec = app.whole_spec().expect("a spec");
    assert_eq!(
        (spec.kind, spec.category.as_deref()),
        (ChartKind::Bar, Some("Kullanım"))
    );
}

#[test]
fn another_drawing_starts_the_choices_again() {
    let mut app = opened();
    let _ = app.update(Message::Run("chart.scatter"));
    ev(&mut app, Event::View(View::Table));
    open_charts(&mut app);
    assert_eq!(app.charts.view, View::Chart);
    assert!(app.charts.layer_spec.is_none());
}

#[test]
fn a_saved_picture_is_the_cores_svg_and_its_png() {
    let mut app = opened();
    app.open_chart(
        Some("parsel".into()),
        None,
        Some(Spec {
            category: Some("Kullanım".into()),
            ..Spec::new(ChartKind::Bar)
        }),
    );
    app.chart_refresh();
    let frame = super::canvas::room(iced::Size::new(480.0, 300.0));
    let palette = super::palette::paper(&frame.family);
    let svg = app
        .charts_work(|w| w.svg(&frame, &palette))
        .flatten()
        .expect("the SVG");
    assert!(svg.starts_with("<svg"));
    let dir = std::env::temp_dir().join(format!("kentos-grafik-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let png = dir.join("grafik.png");
    let file = super::export::write(&svg, Picture::Png, 480.0, 300.0, &png).expect("written");
    assert_eq!(file, "grafik.png");
    let bytes = std::fs::read(&png).expect("read back");
    // A PNG twice the room's size.
    assert_eq!(&bytes[1..4], b"PNG");
    let (w, h) = (
        u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
    );
    assert_eq!((w, h), (960, 600));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_picture_paints_the_surface_and_its_marks() {
    let mut app = opened();
    app.open_chart(
        Some("parsel".into()),
        None,
        Some(Spec {
            category: Some("Kullanım".into()),
            ..Spec::new(ChartKind::Bar)
        }),
    );
    app.chart_refresh();
    let frame = super::canvas::room(iced::Size::new(400.0, 240.0));
    let layout = app
        .charts_work(|w| w.plain_layout(&frame))
        .flatten()
        .expect("a layout");
    let t = kentos_ui::theme::Tokens::DARK;
    let pixels = super::paint::picture(&layout, &t, 800, 480, 2.0).expect("painted");
    assert_eq!(pixels.len(), 800 * 480 * 4);
    // Opaque everywhere, the corner the surface's colour.
    assert!(pixels.chunks_exact(4).all(|p| p[3] == 255));
    let ground = t.surface.into_rgba8();
    assert_eq!(pixels[..3], ground[..3]);
    // The first series' colour is somewhere: the bars.
    let blue = super::palette::DARK_SERIES[0];
    let (r, g, b) = ((blue >> 16) as u8, (blue >> 8) as u8, blue as u8);
    assert!(
        pixels
            .chunks_exact(4)
            .any(|p| p[0] == r && p[1] == g && p[2] == b)
    );
}

#[test]
fn the_chart_tokens_are_the_webs() {
    let css = include_str!("../../../web/src/styles/tokens.css");
    let light = css
        .find("[data-theme='light']")
        .or_else(|| css.find("[data-theme=\"light\"]"))
        .expect("the light theme");
    let read = |part: &str, name: &str| -> u32 {
        let at = part
            .find(&format!("{name}:"))
            .unwrap_or_else(|| panic!("{name}"));
        let rest = &part[at + name.len() + 1..];
        let hex = rest.trim_start().trim_start_matches('#');
        u32::from_str_radix(&hex[..6], 16).expect("a colour")
    };
    let (dark, light) = (&css[..light], &css[light..]);
    for k in 0..8 {
        let name = format!("--chart-{}", k + 1);
        assert_eq!(read(dark, &name), super::palette::DARK_SERIES[k], "{name}");
        assert_eq!(
            read(light, &name),
            super::palette::LIGHT_SERIES[k],
            "{name}"
        );
    }
    assert_eq!(read(dark, "--chart-other"), super::palette::DARK_OTHER);
    assert_eq!(read(light, "--chart-other"), super::palette::LIGHT_OTHER);
}

/// The picture of Saçılım's 100 000 points painted in the screen's pixels
/// at twice the room (a HiDPI screen), against 40 ms (docs/adr/0227 §11):
///
/// ```text
/// cargo test --release -p kentos-desktop charts::tests::painting_100000_points -- --ignored --nocapture
/// ```
#[test]
#[ignore = "a measurement: run by hand in release"]
fn painting_100000_points() {
    use kentos_geometry_core::ops::charts::Work;
    use kentos_geometry_core::ops::charts::data::{Cell, Column, DataSet, Kind};
    let n = 100_000;
    let column = |key: &str, f: &dyn Fn(usize) -> f64| Column {
        key: key.into(),
        label: key.into(),
        kind: Kind::Number,
        labelled: false,
        cells: (0..n)
            .map(|i| Cell::Value(format!("{:.2}", f(i)), None))
            .collect(),
    };
    let mut work = Work::new(DataSet {
        rows: n,
        columns: vec![
            column("X", &|i| {
                1000.0 * (i as f64 * 0.754_877_666_246_693).fract()
            }),
            column("Y", &|i| {
                1000.0 * (i as f64 * 0.569_840_290_998_053_2).fract()
            }),
        ],
    });
    let rows: Vec<u32> = (0..n as u32).collect();
    let spec = Spec {
        x: Some("X".into()),
        y: vec!["Y".into()],
        ..Spec::new(ChartKind::Scatter)
    };
    work.chart(&spec, &rows).expect("a scatter");
    let layout = work
        .lay_out(&super::canvas::room(iced::Size::new(900.0, 260.0)))
        .cloned()
        .expect("laid out");
    let t = kentos_ui::theme::Tokens::DARK;
    let mut ms: Vec<f64> = (0..7)
        .map(|_| {
            let start = std::time::Instant::now();
            let px = super::paint::picture(&layout, &t, 1800, 520, 2.0).expect("painted");
            assert_eq!(px.len(), 1800 * 520 * 4);
            start.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    ms.sort_by(f64::total_cmp);
    println!(
        "Saçılımın resmi, 100 000 nokta, 1800 × 520 piksel: {:.1} ms (en yavaşı {:.1}); bütçe 40 ms",
        ms[3], ms[6]
    );
}
