//! Grafikler ve alan istatistikleri's pictures (docs/adr/0227) over a
//! district's parcels with typed fields and its roads
//! (fixtures/interaction/v1/charts.kcad, scripts/fixtures/chart_scene.py):
//! İstatistik with Kullanım chosen, then the six kinds: Çubuk by Kullanım
//! with the Ticaret parcels selected (their bar marked), Pasta of the
//! parcels' area by Kullanım, Histogram of the unit values, Kutu of the
//! unit values by Kullanım, Saçılım of the unit value against the area with
//! its trend, Çizgi of the mean unit value by floors, the pointer over a
//! bar (its tooltip), Saçılım's dragged box selecting the large parcels,
//! and the bars' Tablo.
//! The web's are `shots.mjs charts`, at the same places with the same values.
//! `tools_screens` takes them in the dark and the light theme at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=grafik-istatistik,grafik-cubuk,grafik-pasta,grafik-histogram,grafik-kutu,grafik-sacilim,grafik-cizgi,grafik-ipucu,grafik-sacilim-secim,grafik-tablo cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use kentos_commands::DocumentSnapshotV1;
use kentos_domain::Slot;
use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Spec};
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::charts::{Event, Setting, View};
use crate::document::Document;
use crate::tools_screens::Pointed;

pub(crate) const CHARTS: &str = include_str!("../../../fixtures/interaction/v1/charts.kcad");

/// The district open in `app`, the drawing fitted to it.
pub(crate) fn open_charts(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(CHARTS).expect("the drawing reads");
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1/charts.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Charts(e));
}

/// The parcels whose Kullanım is `code`.
pub(crate) fn parcels_of(app: &App, code: &str) -> Vec<Slot> {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model
        .by_layer("parsel")
        .filter(|e| e.base().attrs.get("Kullanım").map(String::as_str) == Some(code))
        .map(|e| Slot(e.base().id))
        .collect()
}

/// A chart of the parcels in the Grafik tab.
fn chart(app: &mut App, spec: Spec) {
    open_charts(app);
    app.open_chart(Some("parsel".into()), None, Some(spec));
}

/// The view fitted to the district, İstatistik's summaries counted.
fn settle(_s: &mut Snapshot, app: &mut App) {
    let _ = app.update(Message::Run("view.zoomExtents"));
    for _ in 0..64 {
        if !app.stats_counting() {
            break;
        }
        let _ = app.update(Message::Statistics(crate::statistics::Event::Count));
    }
}

fn spec(kind: ChartKind) -> Spec {
    Spec::new(kind)
}

/// Where element `e` of the chart on screen is in the window: its band's middle, or its point.
fn element_at(app: &App, e: u32) -> iced::Point {
    use kentos_geometry_core::ops::charts::layout::Target;
    let room = crate::charts::canvas::last_room(&app.charts).expect("the chart was drawn");
    let at = app
        .charts_work(|w| {
            w.layout().and_then(|l| {
                l.targets.iter().find_map(|t| match *t {
                    Target::Band {
                        x0,
                        x1,
                        y0,
                        y1,
                        elem,
                    } if elem == e => Some(((x0 + x1) / 2.0, (y0 + y1) / 2.0)),
                    Target::Point { x, y, elem } if elem == e => Some((x, y)),
                    _ => None,
                })
            })
        })
        .flatten()
        .expect("the element is on the chart");
    iced::Point::new(room.x + at.0, room.y + at.1)
}

/// The chart's bar named `category`: its element.
fn bar_of(app: &App, category: &str) -> u32 {
    app.chart_refresh();
    let now = app.charts.chart.borrow();
    let rows = &now.table.as_ref().expect("a chart").rows;
    rows.iter().position(|r| r[0] == category).expect("the bar") as u32
}

/// The view drawn once: the chart's canvas lays it out and keeps its room.
fn drawn(s: &mut Snapshot, app: &mut App) {
    settle(s, app);
    let _ = s.render(app.view(), &app.theme());
}

fn input(s: &mut Snapshot, app: &mut App, what: kentos_ui::snapshot::Input) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    s.input(app, App::view, &mut update, what);
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // İstatistik: every field of the parcels, Kullanım's values and their shares.
        (
            "grafik-istatistik",
            |app| {
                open_charts(app);
                app.open_statistics(Some("parsel".into()));
                let _ = app.update(Message::Statistics(crate::statistics::Event::Field(
                    "Kullanım".into(),
                )));
            },
            settle,
        ),
        // Çubuk: the parcels by Kullanım; the Ticaret parcels selected, their bar in the selection's colour.
        (
            "grafik-cubuk",
            |app| {
                chart(
                    app,
                    Spec {
                        category: Some("Kullanım".into()),
                        ..spec(ChartKind::Bar)
                    },
                );
                let t = parcels_of(app, "T");
                app.selection.set(t);
            },
            settle,
        ),
        // Pasta: the parcels' area by Kullanım.
        (
            "grafik-pasta",
            |app| {
                chart(
                    app,
                    Spec {
                        category: Some("Kullanım".into()),
                        aggregate: Aggregate::Sum,
                        value: Some("$alan".into()),
                        ..spec(ChartKind::Pie)
                    },
                );
            },
            settle,
        ),
        // Histogram of the unit values, the mean and the median marked.
        (
            "grafik-histogram",
            |app| {
                chart(
                    app,
                    Spec {
                        value: Some("Değer".into()),
                        ..spec(ChartKind::Histogram)
                    },
                );
            },
            settle,
        ),
        // Kutu: the unit values by Kullanım.
        (
            "grafik-kutu",
            |app| {
                chart(
                    app,
                    Spec {
                        value: Some("Değer".into()),
                        group: Some("Kullanım".into()),
                        ..spec(ChartKind::Box)
                    },
                );
            },
            settle,
        ),
        // Saçılım: the unit value against the area, its trend and R².
        (
            "grafik-sacilim",
            |app| {
                chart(
                    app,
                    Spec {
                        x: Some("$alan".into()),
                        y: vec!["Değer".into()],
                        ..spec(ChartKind::Scatter)
                    },
                );
            },
            settle,
        ),
        // Çizgi: the mean unit value by floors.
        (
            "grafik-cizgi",
            |app| {
                chart(
                    app,
                    Spec {
                        x: Some("Kat".into()),
                        y: vec!["Değer".into()],
                        aggregate: Aggregate::Mean,
                        ..spec(ChartKind::Line)
                    },
                );
                ev(app, Event::Set(Setting::Aggregate(Aggregate::Mean)));
            },
            settle,
        ),
        // The pointer over Konut's bar: the bar outlined, its tooltip (the count first, then the name).
        (
            "grafik-ipucu",
            |app| {
                chart(
                    app,
                    Spec {
                        category: Some("Kullanım".into()),
                        ..spec(ChartKind::Bar)
                    },
                );
            },
            |s, app| {
                drawn(s, app);
                let at = element_at(app, bar_of(app, "Konut"));
                input(s, app, kentos_ui::snapshot::Input::Move(at));
            },
        ),
        // Saçılım's box dragged over the large parcels: their points selected, marked, and in the drawing.
        (
            "grafik-sacilim-secim",
            |app| {
                chart(
                    app,
                    Spec {
                        x: Some("$alan".into()),
                        y: vec!["Değer".into()],
                        ..spec(ChartKind::Scatter)
                    },
                );
            },
            |s, app| {
                drawn(s, app);
                let room = crate::charts::canvas::last_room(&app.charts).expect("drawn");
                let (a, b) = (
                    iced::Point::new(room.x + room.width * 0.62, room.y + room.height * 0.12),
                    iced::Point::new(room.x + room.width * 0.97, room.y + room.height * 0.78),
                );
                input(s, app, kentos_ui::snapshot::Input::Drag(a, b));
            },
        ),
        // Tablo: the bars' numbers.
        (
            "grafik-tablo",
            |app| {
                chart(
                    app,
                    Spec {
                        category: Some("Kullanım".into()),
                        ..spec(ChartKind::Bar)
                    },
                );
                ev(app, Event::View(View::Table));
            },
            settle,
        ),
    ]
}
