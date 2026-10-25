//! Grafikler ve alan istatistikleri (docs/adr/0227) against the independent
//! reference in `fixtures/charts/v1/cases.json`
//! (`scripts/fixtures/chart_cases.py`, no KentOS code): every field's
//! summary and full statistics, every chart's model (titles, notes, the
//! elements' rows, the table and what is drawn), the axes' ticks, four
//! significant digits and the text's estimated width. The layout's
//! properties are checked below the cases.

use kentos_geometry_core::ops::charts::data::{Cell, Column, DataSet, Kind};
use kentos_geometry_core::ops::charts::fields::{detail, summary};
use kentos_geometry_core::ops::charts::fonts::{fit, width};
use kentos_geometry_core::ops::charts::json::spec_from_json;
use kentos_geometry_core::ops::charts::layout::{Frame, Prim, Target, hit, lay_out};
use kentos_geometry_core::ops::charts::model::{Body, Color, Model};
use kentos_geometry_core::ops::charts::ticks::{nice, sig4};
use kentos_geometry_core::ops::charts::{Work, data::Read};
use kentos_geometry_core::ops::statistics::read_number;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/charts/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.chart-cases");
    file
}

fn data_of(v: &Value) -> DataSet {
    let columns = v["columns"]
        .as_array()
        .expect("columns")
        .iter()
        .map(|c| Column {
            key: c["key"].as_str().expect("key").to_owned(),
            label: c["label"].as_str().expect("label").to_owned(),
            kind: Kind::from_word(c["kind"].as_str().expect("kind")).expect("kind word"),
            labelled: c["labelled"].as_bool().unwrap_or(false),
            cells: c["cells"]
                .as_array()
                .expect("cells")
                .iter()
                .map(|cell| match cell {
                    Value::Null => Cell::Empty,
                    v if v.get("invalid").is_some() => {
                        Cell::Invalid(v["invalid"].as_str().expect("text").to_owned())
                    }
                    v => Cell::Value(
                        v["v"].as_str().expect("value").to_owned(),
                        v.get("s").and_then(Value::as_str).map(str::to_owned),
                    ),
                })
                .collect(),
        })
        .collect();
    DataSet {
        rows: v["rows"].as_u64().expect("rows") as usize,
        columns,
    }
}

fn rows_of(v: &Value) -> Vec<u32> {
    v.as_array()
        .expect("rows")
        .iter()
        .map(|r| r.as_u64().expect("row") as u32)
        .collect()
}

fn opt(v: Option<String>) -> Value {
    v.map_or(Value::Null, Value::String)
}

#[test]
fn every_field_gives_the_reference_statistics() {
    let file = cases();
    let mut n = 0;
    for case in file["fields"].as_array().expect("fields") {
        let data = match &case["data"] {
            Value::String(name) => data_of(&file["data"][name.as_str()]),
            v => data_of(v),
        };
        let col = &data.columns[case["column"].as_u64().expect("column") as usize];
        let read = Read::of(col);
        let rows = rows_of(&case["rows"]);
        let name = case["name"].as_str().unwrap_or_default();
        let s = summary(col, &read, &rows);
        let mut got = json!({
            "key": s.key, "label": s.label, "kind": s.kind, "objects": s.objects, "filled": s.filled,
            "empty": s.empty, "invalid": s.invalid, "distinct": s.distinct,
        });
        for (k, v) in [
            ("min", s.min),
            ("max", s.max),
            ("mean", s.mean),
            ("sum", s.sum),
        ] {
            if let Some(v) = v {
                got[k] = Value::String(v);
            }
        }
        assert_eq!(got, case["summary"], "{name}: summary");
        let d = detail(col, &read, &rows);
        let stats: Vec<Value> = d.stats.iter().map(|s| json!([s.label, s.value])).collect();
        assert_eq!(
            Value::Array(stats),
            case["detail"]["stats"],
            "{name}: stats"
        );
        let values: Vec<Value> = d
            .values
            .iter()
            .map(|v| json!({"text": v.text, "count": v.count, "share": v.share, "mark": v.mark.word(), "rows": v.rows}))
            .collect();
        assert_eq!(
            Value::Array(values),
            case["detail"]["values"],
            "{name}: values"
        );
        assert_eq!(json!(d.more), case["detail"]["more"], "{name}: more");
        n += 1;
    }
    assert!(n >= 25, "{n}");
}

fn color(c: Color) -> Value {
    match c {
        Color::Series(k) => json!(k),
        Color::Other => json!("other"),
    }
}

fn body(m: &Model) -> Value {
    match &m.body {
        Body::Bars { bars, integer } => json!({
            "bars": bars.iter().map(|b| json!({
                "label": b.label, "value": b.value.map(|v| v.text()).unwrap_or_default(),
                "objects": b.objects, "other": b.other,
            })).collect::<Vec<_>>(),
            "integer": integer,
        }),
        Body::Pie {
            slices,
            total,
            word,
        } => json!({
            "slices": slices.iter().map(|s| json!({
                "label": s.label, "value": s.value.text(), "share": s.share, "color": color(s.color),
            })).collect::<Vec<_>>(),
            "total": total.text(),
            "word": word,
        }),
        Body::Histogram {
            bins,
            min,
            max,
            mean,
            median,
            ..
        } => json!({
            "bins": bins.iter().map(|b| json!({"lo": b.lo, "hi": b.hi, "count": b.count})).collect::<Vec<_>>(),
            "min": min.text(), "max": max.text(),
            "mean": opt(mean.map(|v| v.text())), "median": opt(median.map(|v| v.text())),
        }),
        Body::Boxes { boxes } => json!({
            "boxes": boxes.iter().map(|b| if b.n == 0 {
                json!({"label": b.label, "n": 0, "outliers": []})
            } else {
                json!({
                    "label": b.label, "n": b.n, "min": b.min.text(), "q1": b.q1.text(),
                    "median": b.median.text(), "q3": b.q3.text(), "max": b.max.text(),
                    "low": b.low.text(), "high": b.high.text(),
                    "mean": opt(b.mean.map(|v| v.text())),
                    "outliers": b.outliers.iter().map(|(v, e)| json!([v.text(), e])).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
        }),
        Body::Scatter {
            x,
            x_range,
            y_range,
            trend,
            ..
        } => json!({
            "trend": opt(trend.as_ref().map(|t| t.text.clone())),
            "xRange": x_range.map(|(a, b)| json!([a.text(), b.text()])).unwrap_or(Value::Null),
            "yRange": y_range.map(|(a, b)| json!([a.text(), b.text()])).unwrap_or(Value::Null),
            "count": x.len(),
        }),
        Body::Lines {
            series,
            x_range,
            y_range,
        } => json!({
            "series": series.iter().map(|s| json!({
                "label": s.label, "color": color(s.color),
                "points": s.points.iter().map(|p| json!([p.x.text(), p.y.text(), p.element])).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "xRange": x_range.map(|(a, b)| json!([a.text(), b.text()])).unwrap_or(Value::Null),
            "yRange": y_range.map(|(a, b)| json!([a.text(), b.text()])).unwrap_or(Value::Null),
        }),
    }
}

#[test]
fn every_chart_gives_the_reference_model() {
    let file = cases();
    let mut n = 0;
    for case in file["charts"].as_array().expect("charts") {
        let name = case["name"].as_str().unwrap_or_default();
        let data = data_of(&file["data"][case["data"].as_str().expect("data")]);
        let spec = spec_from_json(&case["spec"].to_string()).expect("spec");
        let rows = rows_of(&case["rows"]);
        let mut work = Work::new(data);
        let m = work.chart(&spec, &rows).expect("chart").clone();
        let want = &case["want"];
        assert_eq!(json!(m.kind.word()), want["kind"], "{name}: kind");
        assert_eq!(json!(m.x_title), want["xTitle"], "{name}: x title");
        assert_eq!(json!(m.y_title), want["yTitle"], "{name}: y title");
        assert_eq!(json!(m.notes), want["notes"], "{name}: notes");
        let elements: Vec<Vec<u32>> = m.elements.iter().map(|e| e.rows.clone()).collect();
        assert_eq!(json!(elements), want["elements"], "{name}: elements");
        assert_eq!(
            json!({"columns": m.table.columns, "rows": m.table.rows}),
            want["table"],
            "{name}: table"
        );
        assert_eq!(body(&m), want["body"], "{name}: body");
        n += 1;
    }
    assert!(n >= 25, "{n}");
}

#[test]
fn axes_take_the_reference_ticks() {
    for case in cases()["ticks"].as_array().expect("ticks") {
        let num = |k: &str| read_number(case[k].as_str().expect("number")).expect("decimal");
        let got = nice(
            num("lo"),
            num("hi"),
            case["max"].as_u64().expect("max") as u32,
            case["integer"].as_bool().expect("integer"),
            case["zero"].as_bool().expect("zero"),
        )
        .map(|a| json!({"lo": a.lo.text(), "hi": a.hi.text(), "step": a.step.text(), "labels": a.labels}));
        assert_eq!(got.unwrap_or(Value::Null), case["want"], "{case}");
    }
}

#[test]
fn four_significant_digits_as_the_reference() {
    for case in cases()["sig4"].as_array().expect("sig4") {
        let x = case["x"].as_f64().expect("x");
        assert_eq!(json!(sig4(x)), case["want"], "{x}");
    }
}

#[test]
fn text_widths_as_the_reference() {
    for case in cases()["widths"].as_array().expect("widths") {
        let text = case["text"].as_str().expect("text");
        let size = case["size"].as_f64().expect("size") as f32;
        let family = case["family"].as_str().expect("family");
        let w = width(text, size, family);
        let want = case["width"].as_f64().expect("width") as f32;
        assert!((w - want).abs() < 1e-3, "{text} {family}: {w} {want}");
        let room = case["room"].as_f64().expect("room") as f32;
        assert_eq!(
            json!(fit(text, room, size, family)),
            case["fit"],
            "{text} {family}"
        );
    }
}

// ── The layout's properties ─────────────────────────────────────────────

fn frame(width: f32, height: f32) -> Frame {
    Frame {
        width,
        height,
        font: 12.0,
        family: "Plus Jakarta Sans".to_owned(),
    }
}

/// Every chart of the cases laid out in a wide and a narrow frame: its
/// primitives stay in the frame, every element is a target a click finds
/// at its own place, and a pie's slices close the ring.
#[test]
fn layouts_stay_in_their_frame_and_find_their_elements() {
    let file = cases();
    for case in file["charts"].as_array().expect("charts") {
        let name = case["name"].as_str().unwrap_or_default();
        let data = data_of(&file["data"][case["data"].as_str().expect("data")]);
        let spec = spec_from_json(&case["spec"].to_string()).expect("spec");
        let rows = rows_of(&case["rows"]);
        let mut work = Work::new(data);
        let m = work.chart(&spec, &rows).expect("chart").clone();
        for (w, h) in [(900.0, 320.0), (420.0, 190.0)] {
            let l = lay_out(&m, &[], &frame(w, h));
            for p in &l.prims {
                let inside =
                    |x: f32, y: f32| (-1.0..=w + 1.0).contains(&x) && (-1.0..=h + 1.0).contains(&y);
                match p {
                    Prim::Rect {
                        x, y, w: rw, h: rh, ..
                    } => {
                        assert!(
                            inside(*x, *y) && inside(x + rw, y + rh),
                            "{name} {w}: {p:?}"
                        )
                    }
                    Prim::Line { x1, y1, x2, y2, .. } => {
                        assert!(inside(*x1, *y1) && inside(*x2, *y2), "{name} {w}: {p:?}")
                    }
                    Prim::Circle { x, y, .. } => assert!(inside(*x, *y), "{name} {w}: {p:?}"),
                    Prim::Text { x, y, .. } => assert!(inside(*x, *y), "{name} {w}: {p:?}"),
                    _ => {}
                }
            }
            // A band's or a wedge's middle and a point's place find their element (a point
            // may hide a band beneath it).
            for t in &l.targets {
                let (x, y, e) = match t {
                    Target::Band {
                        x0,
                        x1,
                        y0,
                        y1,
                        elem,
                    } => ((x0 + x1) / 2.0, (y0 + y1) / 2.0, *elem),
                    Target::Wedge {
                        cx,
                        cy,
                        r0,
                        r1,
                        a0,
                        a1,
                        elem,
                    } => {
                        let (a, r) = ((a0 + a1) / 2.0, (r0 + r1) / 2.0);
                        (cx + r * a.sin(), cy - r * a.cos(), *elem)
                    }
                    Target::Point { x, y, elem } => (*x, *y, *elem),
                };
                let got = hit(&l, x, y);
                if matches!(t, Target::Band { .. }) && got != Some(e) {
                    // A point drawn over the band's middle wins.
                    assert!(got.is_some(), "{name} {w}: band {e}");
                    continue;
                }
                assert_eq!(got, Some(e), "{name} {w}: {t:?}");
            }
            if let Body::Pie { .. } = &m.body {
                let wedges: Vec<(f32, f32)> = l
                    .prims
                    .iter()
                    .filter_map(|p| match p {
                        Prim::Wedge { a0, a1, .. } => Some((*a0, *a1)),
                        _ => None,
                    })
                    .collect();
                if let (Some(first), Some(last)) = (wedges.first(), wedges.last()) {
                    assert!(first.0.abs() < 1e-6, "{name}");
                    assert!((last.1 - 2.0 * std::f32::consts::PI).abs() < 1e-5, "{name}");
                    for pair in wedges.windows(2) {
                        assert!((pair[0].1 - pair[1].0).abs() < 1e-5, "{name}");
                    }
                }
            }
        }
    }
}

/// The selection marks the elements holding a selected row; a saved
/// picture is an SVG document of the chart, unmarked.
#[test]
fn the_selection_marks_its_elements() {
    let file = cases();
    let case = file["charts"]
        .as_array()
        .expect("charts")
        .iter()
        .find(|c| c["name"] == "cubuk-sayi")
        .expect("cubuk-sayi");
    let mut work = Work::new(data_of(&file["data"]["parseller"]));
    let spec = spec_from_json(&case["spec"].to_string()).expect("spec");
    let rows: Vec<u32> = (0..12).collect();
    work.chart(&spec, &rows).expect("chart");
    // Konut, Park, Sanayi, Ticaret, (boş): row 2 is a Ticaret.
    assert!(work.select(&[2]));
    let l = work.lay_out(&frame(800.0, 300.0)).expect("layout").clone();
    let fills: Vec<(i32, String, f32)> = l
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Rect {
                fill, alpha, elem, ..
            } if *elem >= 0 => Some((*elem, fill.word(), *alpha)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills.iter().find(|f| f.0 == 3).map(|f| f.1.clone()),
        Some("selection".into())
    );
    assert!(fills.iter().filter(|f| f.0 != 3).all(|f| f.2 < 1.0));
    assert_eq!(
        work.tip(3).last(),
        Some(&("Seçili".to_owned(), "1".to_owned()))
    );
    assert!(!work.select(&[2]), "the same selection marks nothing new");
    let palette = kentos_geometry_core::ops::charts::svg::Palette {
        background: "#ffffff".into(),
        surface: "#ffffff".into(),
        grid: "#e1e0d9".into(),
        axis: "#c3c2b7".into(),
        ink1: "#0b0b0b".into(),
        ink2: "#52514e".into(),
        ink3: "#898781".into(),
        series: vec!["#2a78d6".into()],
        other: "#898781".into(),
        selection: "#1f4a96".into(),
        font: "Plus Jakarta Sans".into(),
    };
    let svg = work.svg(&frame(640.0, 360.0), &palette).expect("svg");
    assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
    assert!(
        svg.contains("Konut") && !svg.contains("#1f4a96"),
        "unmarked"
    );
}

/// The reference's charts as SVG pictures in `KENTOS_CHART_SVG` (a folder),
/// light and dark, for looking at the layout:
/// `KENTOS_CHART_SVG=.run/shots/grafik-svg cargo test -p kentos-geometry-core --test all charts::pictures -- --ignored`.
#[test]
#[ignore = "writes pictures"]
fn pictures() {
    let Ok(dir) = std::env::var("KENTOS_CHART_SVG") else {
        return;
    };
    std::fs::create_dir_all(&dir).expect("folder");
    let file = cases();
    let light = kentos_geometry_core::ops::charts::svg::Palette {
        background: "#f4f6f8".into(),
        surface: "#f4f6f8".into(),
        grid: "#e1e4e8".into(),
        axis: "#c3c9d0".into(),
        ink1: "#1b232c".into(),
        ink2: "#4d5966".into(),
        ink3: "#7a8591".into(),
        series: [
            "#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#4a3aa7", "#e34948",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect(),
        other: "#9aa3ad".into(),
        selection: "#1f4a96".into(),
        font: "Plus Jakarta Sans".into(),
    };
    let dark = kentos_geometry_core::ops::charts::svg::Palette {
        background: "#212932".into(),
        surface: "#212932".into(),
        grid: "#2d3641".into(),
        axis: "#3d4856".into(),
        ink1: "#d6dde5".into(),
        ink2: "#9ba7b5".into(),
        ink3: "#6d7988".into(),
        series: [
            "#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect(),
        other: "#5d6875".into(),
        selection: "#4c7fe0".into(),
        font: "Plus Jakarta Sans".into(),
    };
    for case in file["charts"].as_array().expect("charts") {
        let name = case["name"].as_str().unwrap_or_default();
        let data = data_of(&file["data"][case["data"].as_str().expect("data")]);
        let spec = spec_from_json(&case["spec"].to_string()).expect("spec");
        let rows = rows_of(&case["rows"]);
        let mut work = Work::new(data);
        work.chart(&spec, &rows).expect("chart");
        for (theme, palette) in [("acik", &light), ("koyu", &dark)] {
            let svg = work.svg(&frame(760.0, 300.0), palette).expect("svg");
            std::fs::write(format!("{dir}/{name}-{theme}.svg"), svg).expect("write");
        }
    }
}

/// Columns of a kind, every cell empty: what the specs read is their kinds.
fn shaped(columns: &[(&str, Kind)]) -> DataSet {
    DataSet {
        rows: 0,
        columns: columns
            .iter()
            .map(|(key, kind)| Column {
                key: (*key).to_owned(),
                label: (*key).to_owned(),
                kind: *kind,
                labelled: false,
                cells: Vec::new(),
            })
            .collect(),
    }
}

#[test]
fn a_spec_is_made_whole_for_its_kind_and_its_data() {
    use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Sort, Spec};
    use kentos_geometry_core::ops::charts::specs::complete;
    let data = shaped(&[
        ("Ada", Kind::Number),
        ("Kullanım", Kind::Text),
        ("Kat", Kind::Number),
        ("$alan", Kind::Number),
    ]);
    // Çubuk: the first column that is no number is the category, the first number the value.
    let bar = complete(&Spec::new(ChartKind::Bar), &data);
    assert_eq!(bar.category.as_deref(), Some("Kullanım"));
    assert_eq!(bar.value.as_deref(), Some("Ada"));
    assert_eq!(
        (bar.aggregate, bar.sort),
        (Aggregate::Count, Sort::Category)
    );
    // Pasta offers the count and the sum only; a column gone is chosen again.
    let pie = complete(
        &Spec {
            aggregate: Aggregate::Mean,
            category: Some("Yok".into()),
            ..Spec::new(ChartKind::Pie)
        },
        &data,
    );
    assert_eq!(pie.aggregate, Aggregate::Count);
    assert_eq!(pie.category.as_deref(), Some("Kullanım"));
    // Saçılım: Y is a number other than X.
    let scatter = complete(&Spec::new(ChartKind::Scatter), &data);
    assert_eq!(scatter.x.as_deref(), Some("Ada"));
    assert_eq!(scatter.y, vec!["Kat".to_owned()]);
    // Çizgi: its points each a row; Y keeps its numbers; a series with several Ys goes.
    let line = complete(
        &Spec {
            y: vec!["Kullanım".into(), "Kat".into(), "$alan".into()],
            series: Some("Kullanım".into()),
            ..Spec::new(ChartKind::Line)
        },
        &data,
    );
    assert_eq!(line.aggregate, Aggregate::None);
    assert_eq!(line.y, vec!["Kat".to_owned(), "$alan".to_owned()]);
    assert_eq!(line.series, None);
    // Kutu: a group the data has not is dropped.
    let boxes = complete(
        &Spec {
            group: Some("Mahalle".into()),
            ..Spec::new(ChartKind::Box)
        },
        &data,
    );
    assert_eq!((boxes.value.as_deref(), boxes.group), (Some("Ada"), None));
    // Without numbers: the count, nothing to take a value from.
    let texts = shaped(&[("Ad", Kind::Text)]);
    let bar = complete(
        &Spec {
            aggregate: Aggregate::Sum,
            ..Spec::new(ChartKind::Bar)
        },
        &texts,
    );
    assert_eq!((bar.value, bar.aggregate), (None, Aggregate::Count));
}

#[test]
fn a_column_and_a_tools_table_start_their_own_charts() {
    use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind};
    use kentos_geometry_core::ops::charts::specs::{for_column, for_table};
    let data = shaped(&[("Kullanım", Kind::Text), ("Alan", Kind::Number)]);
    let h = for_column(&data, "Alan");
    assert_eq!(
        (h.kind, h.value.as_deref()),
        (ChartKind::Histogram, Some("Alan"))
    );
    let b = for_column(&data, "Kullanım");
    assert_eq!(
        (b.kind, b.category.as_deref()),
        (ChartKind::Bar, Some("Kullanım"))
    );
    // ROC ile doğrulama's curve, either way it counts its false positives.
    for x in ["Alan oranı (%)", "Yanlış pozitif oranı (%)"] {
        let roc = shaped(&[
            ("Eşik", Kind::Number),
            ("Doğru pozitif", Kind::Number),
            ("Doğru pozitif oranı (%)", Kind::Number),
            ("Hücre", Kind::Number),
            (x, Kind::Number),
        ]);
        let s = for_table("suitability.roc", &roc);
        assert_eq!(s.kind, ChartKind::Line);
        assert_eq!(s.x.as_deref(), Some(x));
        assert_eq!(s.y, vec!["Doğru pozitif oranı (%)".to_owned()]);
        assert_eq!(s.aggregate, Aggregate::None);
    }
    let profile = shaped(&[
        ("Çizgi", Kind::Text),
        ("Uzaklık (m)", Kind::Number),
        ("Y", Kind::Number),
        ("X", Kind::Number),
        ("Değer", Kind::Number),
    ]);
    let s = for_table("multidim.profile", &profile);
    assert_eq!(
        (s.x.as_deref(), s.y.clone(), s.series.as_deref()),
        (Some("Uzaklık (m)"), vec!["Değer".to_owned()], Some("Çizgi"))
    );
    let series = shaped(&[
        ("Adım", Kind::Number),
        ("Zaman", Kind::Date),
        ("Köprü", Kind::Number),
        ("Baraj", Kind::Number),
    ]);
    let s = for_table("multidim.series", &series);
    assert_eq!(s.x.as_deref(), Some("Adım"));
    assert_eq!(s.y, vec!["Köprü".to_owned(), "Baraj".to_owned()]);
    let histogram = shaped(&[
        ("Aralık", Kind::Number),
        ("Alt sınır", Kind::Number),
        ("Üst sınır", Kind::Number),
        ("Sayı", Kind::Number),
    ]);
    let s = for_table("raster.histogram", &histogram);
    assert_eq!(
        (
            s.kind,
            s.category.as_deref(),
            s.value.as_deref(),
            s.aggregate
        ),
        (
            ChartKind::Bar,
            Some("Alt sınır"),
            Some("Sayı"),
            Aggregate::Sum
        )
    );
    // DBSCAN taraması's k-distance graph (docs/adr/0639 §3): the distance by rank, a line.
    let graph = shaped(&[
        ("Sıra", Kind::Number),
        ("3. en yakın uzaklık (m)", Kind::Number),
        ("Dirsek", Kind::Text),
    ]);
    let s = for_table("stats.dbscanSweep", &graph);
    assert_eq!(
        (s.kind, s.x.as_deref(), s.y.clone()),
        (
            ChartKind::Line,
            Some("Sıra"),
            vec!["3. en yakın uzaklık (m)".to_owned()]
        )
    );
    // Another tool, or a table without the columns its tool names: bars by the first column.
    let s = for_table("multidim.profile", &histogram);
    assert_eq!(
        (
            s.kind,
            s.category.as_deref(),
            s.value.as_deref(),
            s.aggregate
        ),
        (
            ChartKind::Bar,
            Some("Aralık"),
            Some("Alt sınır"),
            Aggregate::Sum
        )
    );
    let s = for_table("stats.summary", &shaped(&[("Grup", Kind::Text)]));
    assert_eq!((s.value, s.aggregate), (None, Aggregate::Count));
}

/// Alan and Uzunluk read from the store's measure records in the project's
/// unit (docs/adr/0165 §2: the tabs show readings): m² and m, or mm² and mm
/// in a project in millimetres; a column no object has a value of is left
/// out, an object without the measure has an empty cell.
#[test]
fn the_geometry_columns_speak_the_projects_unit() {
    use kentos_geometry_core::ops::charts::data::geometry_columns;
    use kentos_geometry_core::store::draw::{HAS_AREA, HAS_LENGTH, MEASURE_STRIDE};
    // A 2 × 1.5 area (its perimeter 7) and a line 4 long.
    let mut records = vec![0.0; 2 * MEASURE_STRIDE];
    records[0] = f64::from(HAS_AREA | HAS_LENGTH);
    records[1] = 7.0;
    records[2] = 3.0;
    records[MEASURE_STRIDE] = f64::from(HAS_LENGTH);
    records[MEASURE_STRIDE + 1] = 4.0;
    let cells = |c: &Column| -> Vec<String> {
        c.cells
            .iter()
            .map(|cell| match cell {
                Cell::Value(v, _) => v.clone(),
                _ => String::new(),
            })
            .collect()
    };
    for (k, area, length, areas, lengths) in [
        (
            1.0,
            "Alan (m²)",
            "Uzunluk (m)",
            ["3.000", ""],
            ["7.000", "4.000"],
        ),
        (
            100.0,
            "Alan (cm²)",
            "Uzunluk (cm)",
            ["30000.000", ""],
            ["700.000", "400.000"],
        ),
        (
            1000.0,
            "Alan (mm²)",
            "Uzunluk (mm)",
            ["3000000.000", ""],
            ["7000.000", "4000.000"],
        ),
    ] {
        let columns = geometry_columns(&records, k);
        let got: Vec<(&str, &str, Vec<String>)> = columns
            .iter()
            .map(|c| (c.key.as_str(), c.label.as_str(), cells(c)))
            .collect();
        let want = vec![
            ("$alan", area, areas.map(String::from).to_vec()),
            ("$uzunluk", length, lengths.map(String::from).to_vec()),
        ];
        assert_eq!(got, want, "{k}");
    }
    // Points only: no column.
    let points = vec![0.0; MEASURE_STRIDE];
    assert!(geometry_columns(&points, 1000.0).is_empty());
}
