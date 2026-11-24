//! charts › tests's tests that need the CBS module's surfaces (its project's bottom tabs and panels; docs/adr/9199
//! §4.14): moved from the core's gis/charts/tests.rs with the module's package; the work they test is the core's.

use crate::gis;
use kentos_domain::Slot;
use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Spec};
use crate::charts::{ChartTable, Event, Setting, Source, View};
use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::chart_scenes::{open_charts, parcels_of};

fn opened() -> App {
    let (mut app, _) = App::boot(None);
    open_charts(&mut app);
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Charts(e)));
}

/// The chart's table as the Tablo view shows it.
fn table(app: &App) -> Vec<Vec<String>> {
    app.chart_refresh();
    let now = app.gis.charts.chart.borrow();
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
    assert_eq!(app.gis.charts.layer.as_deref(), Some("parsel"));
    let _ = app.update(Message::Run("chart.pie"));
    assert_eq!(app.bottom_tab, BottomTab::Chart);
    app.chart_refresh();
    assert_eq!(app.gis.charts.chart.borrow().kind, Some(ChartKind::Pie));
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
    let before = app.gis.charts.chart.borrow().stamp;
    app.selection.set(parcels_of(&app, "S"));
    app.chart_refresh();
    assert!(app.gis.charts.chart.borrow().stamp > before);
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
    assert_eq!(app.gis.charts.source, Source::Table);
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
        let _ = app.update(Message::Gis(gis::Event::Statistics(
            crate::statistics::Event::Count,
        )));
    }
    assert!(!app.stats_counting());
    {
        let now = app.gis.charts.stats.borrow();
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
    let _ = app.update(Message::Gis(gis::Event::Statistics(
        crate::statistics::Event::Field("Kullanım".into()),
    )));
    app.stats_refresh();
    let at = app
        .gis
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
    let _ = app.update(Message::Gis(gis::Event::Statistics(
        crate::statistics::Event::Value(at),
    )));
    assert_eq!(app.selection.ids().len(), 3);
    // Grafik: the field's bars in the Grafik tab.
    let _ = app.update(Message::Gis(gis::Event::Statistics(
        crate::statistics::Event::Chart,
    )));
    assert_eq!(app.bottom_tab, BottomTab::Chart);
    let spec = app.whole_spec().expect("a spec");
    assert_eq!(
        (spec.kind, spec.category.as_deref()),
        (ChartKind::Bar, Some("Kullanım"))
    );
}
