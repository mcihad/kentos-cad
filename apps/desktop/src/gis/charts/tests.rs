//! İstatistik and Grafik over `fixtures/interaction/v1/charts.kcad`
//! (docs/adr/0227): the commands open their tabs on the layer at hand, a
//! chart's numbers, a press selecting its element's objects, the
//! selection marking the chart, a layer's other columns choosing the
//! columns again, a tool's table charted, and another drawing starting the
//! choices again; then the painted picture and the saved ones.

use crate::gis;
use kentos_geometry_core::ops::charts::model::{ChartKind, Spec};

use super::{Event, Picture, View};
use crate::app::{App, Message};
use crate::chart_scenes::open_charts;

fn opened() -> App {
    let (mut app, _) = App::boot(None);
    open_charts(&mut app);
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Charts(e)));
}

#[test]
fn another_drawing_starts_the_choices_again() {
    let mut app = opened();
    let _ = app.update(Message::Run("chart.scatter"));
    ev(&mut app, Event::View(View::Table));
    open_charts(&mut app);
    assert_eq!(app.gis.charts.view, View::Chart);
    assert!(app.gis.charts.layer_spec.is_none());
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
    assert!(pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    let ground = t.surface.into_rgba8();
    assert_eq!(pixels[..3], ground[..3]);
    // The first series' colour is somewhere: the bars.
    let blue = super::palette::DARK_SERIES[0];
    let (r, g, b) = ((blue >> 16) as u8, (blue >> 8) as u8, blue as u8);
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] == r && p[1] == g && p[2] == b)
    );
}

#[test]
fn the_chart_tokens_are_the_portals() {
    let css = include_str!("../../../../portal/src/styles/tokens.css");
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
