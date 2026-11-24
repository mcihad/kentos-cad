//! Kübaj's mass ledger and departures, the largest rows, and the approvals' history and states with
//! their charts.

use super::*;

/// A residue in m³ with its sign at `decimals`: “0”, “+1”, “−0,01”.
pub(super) fn residue(v: f64, decimals: u32) -> String {
    let d = decimals as usize;
    if v.abs() < 0.5 * 10f64.powi(-(decimals as i32)) {
        "0".into()
    } else if v < 0.0 {
        format!("−{}", grouped(-v, d))
    } else {
        format!("+{}", grouped(v, d))
    }
}

/// The ledger in a sentence (docs/adr/0949): the heaviest group of the cut and the fill, how the cut crowds, the
/// largest stretch and why, the rounding's residue.
pub(super) fn ledger_words(l: &kentos_commands::VolumeLedgerRead) -> String {
    let mut out = String::new();
    for (k, word) in [(0usize, "Kazının"), (1, "Dolgunun")] {
        if let Some(g) = l.groups.iter().filter(|g| g.shares[k].is_some()).max_by(|a, b| a.values[k].total_cmp(&b.values[k])) {
            out += &format!("{word} %{} kadarı {} grubunda. ", grouped(100.0 * g.shares[k].unwrap_or(0.0), 1), g.label);
        }
    }
    if let Some(c) = l.concentration.iter().find(|c| c.column == "cut") {
        out += &format!(
            "Yolun %{}'i kazının yarısını taşıyor (Gini {}). ",
            grouped(100.0 * c.half, 1),
            grouped(c.gini, 2)
        );
    }
    if let Some(s) = l.top.iter().find(|t| t.column == "cut").and_then(|t| t.stretches.first()) {
        let why = if s.reason == "section" {
            format!("kesiti medyanın {} katı", grouped(s.area_factor, 1))
        } else {
            format!("boyu medyanın {} katı", grouped(s.length_factor, 1))
        };
        out += &format!("En büyük kazı aralığı {} – {}: %{}, {why}. ", s.from_text, s.to_text, grouped(100.0 * s.share, 1));
    }
    let off: Vec<String> = l
        .columns
        .iter()
        .filter(|c| c.residue.abs() >= 0.5 * 10f64.powi(-(l.decimals as i32)))
        .map(|c| format!("{} {} m³", c.words.to_lowercase(), residue(c.residue, l.decimals)))
        .collect();
    if off.is_empty() {
        out += "Yuvarlanmış satırlar yuvarlanmış toplamı tutuyor.";
    } else {
        out += &format!("Yuvarlanmış satırlar toplamdan farklı: {}; toplam yuvarlanmamış değerden yazılır.", off.join(", "));
    }
    out
}

/// The groups: each column's sum and share; the totals; the rounded rows' residue.
pub(super) fn ledger_table(
    l: &kentos_commands::VolumeLedgerRead,
    factored: bool,
) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let d = l.decimals as usize;
    // By the factor rows the factored volumes are the point (ROAD-VOL-21): they come first among the others.
    let extra: Vec<usize> = if factored { (2..l.columns.len()).collect() } else { (4..l.columns.len()).collect() };
    let longest = l.groups.iter().map(|g| g.label.chars().count()).max().unwrap_or(0) as f32;
    let mut cols = vec![
        col("Grup", (7.0 * longest + 16.0).clamp(150.0, 330.0)),
        num("Kazı (m³)", 96.0),
        num("Pay", 56.0),
        num("Dolgu (m³)", 96.0),
        num("Pay", 56.0),
    ];
    for &k in &extra {
        cols.push(num(&l.columns[k].words, 110.0));
    }
    let share = |v: Option<f64>| v.map_or_else(|| "—".into(), |s| format!("%{}", grouped(100.0 * s, 1)));
    let mut rows: Vec<Vec<String>> = l
        .groups
        .iter()
        .map(|g| {
            let mut r = vec![g.label.clone(), grouped(g.values[0], d), share(g.shares[0]), grouped(g.values[1], d), share(g.shares[1])];
            for &k in &extra {
                r.push(grouped(g.values[k], d));
            }
            r
        })
        .collect();
    let mut total = vec!["Toplam".into(), grouped(l.columns[0].total, d), "%100".into(), grouped(l.columns[1].total, d), "%100".into()];
    let mut off = vec!["Yuvarlanmış satırlar farkı".into(), residue(l.columns[0].residue, l.decimals), String::new(), residue(l.columns[1].residue, l.decimals), String::new()];
    for &k in &extra {
        total.push(grouped(l.columns[k].total, d));
        off.push(residue(l.columns[k].residue, l.decimals));
    }
    rows.push(total);
    rows.push(off);
    road_dialog::sheet_fill(cols, rows, "Grup yok")
}

/// The cut's and the fill's largest stretches, with why each is large.
pub(super) fn largest_table(l: &kentos_commands::VolumeLedgerRead) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let rows: Vec<Vec<String>> = l
        .top
        .iter()
        .flat_map(|t| {
            let word = if t.column == "cut" { "Kazı" } else { "Dolgu" };
            t.stretches.iter().map(move |s| {
                vec![
                    word.to_owned(),
                    format!("{} – {}", s.from_text, s.to_text),
                    grouped(s.volume, 1),
                    format!("%{}", grouped(100.0 * s.share, 1)),
                    format!("%{}", grouped(100.0 * s.cumulative, 1)),
                    grouped(s.length, 1),
                    grouped(s.mean_area, 1),
                    format!("{}×", grouped(s.length_factor, 2)),
                    format!("{}×", grouped(s.area_factor, 2)),
                    if s.reason == "section" { "kesit" } else { "boy" }.to_owned(),
                ]
            })
        })
        .collect();
    road_dialog::sheet_fill(
        vec![
            col("Sütun", 58.0),
            col("Aralık", 170.0),
            num("Hacim (m³)", 90.0),
            num("Pay", 56.0),
            num("Birikimli", 70.0),
            num("Boy (m)", 64.0),
            num("Ort. alan (m²)", 92.0),
            num("Boy / medyan", 92.0),
            num("Alan / medyan", 96.0),
            col("Neden", 56.0),
        ],
        rows,
        "Hacimli aralık yok",
    )
}

impl canvas::Program<Message> for Book<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let l = self.read;
        let small = Pixels(typography::caption());
        let text = |frame: &mut canvas::Frame, words: String, at: Point, color: Color, center: bool| {
            frame.fill_text(Text {
                content: words,
                position: at,
                color,
                size: small,
                font: typography::ui(),
                align_x: if center { iced::alignment::Horizontal::Center.into() } else { iced::alignment::Horizontal::Left.into() },
                ..Text::default()
            });
        };
        let split = bounds.width * 0.62;
        // The groups' bars.
        let n = l.groups.len().max(1);
        let top = l.groups.iter().map(|g| g.values[0]).fold(0.0, f64::max);
        let bottom = l.groups.iter().map(|g| g.values[1]).fold(0.0, f64::max);
        let whole = (top + bottom).max(1e-9);
        let (left, pad, foot) = (12.0_f32, 22.0_f32, 18.0_f32);
        let h = bounds.height - 2.0 * pad - foot;
        let axis = pad + h * (top / whole) as f32;
        let slot = (split - left - 8.0) / n as f32;
        let bar = (slot * 0.7).min(56.0);
        frame.stroke(&Path::line(Point::new(left, axis), Point::new(split - 8.0, axis)), Stroke::default().with_color(t.muted).with_width(1.0));
        for (i, g) in l.groups.iter().enumerate() {
            let x = left + slot * (i as f32 + 0.5);
            for (k, color, up) in [(0usize, t.danger, true), (1, t.accent, false)] {
                let v = g.values[k];
                if v <= 0.0 {
                    continue;
                }
                let len = h * (v / whole) as f32;
                let y = if up { axis - len } else { axis };
                frame.fill_rectangle(Point::new(x - bar / 2.0, y), iced::Size::new(bar, len), Color { a: 0.85, ..color });
                if let Some(s) = g.shares[k].filter(|s| *s >= 0.05) {
                    let at = if up { Point::new(x, y - 15.0) } else { Point::new(x, y + len + 2.0) };
                    text(&mut frame, format!("%{}", grouped(100.0 * s, 0)), at, color, true);
                }
            }
            // The group's first km under the chart, thinned so they do not run together.
            let every = ((n as f32 * 70.0 / (split - left)).ceil() as usize).max(1);
            if i % every == 0 {
                // A km step's own first km (its first section may lie past it), a region's name.
                let words = if g.label.starts_with("Bölge") || g.label == "Katsayısız" {
                    g.label.clone()
                } else {
                    g.label.split(" – ").next().unwrap_or(&g.from_text).to_owned()
                };
                text(&mut frame, words, Point::new(x, bounds.height - foot), t.muted, true);
            }
        }
        text(&mut frame, "Kazı ↑ · Dolgu ↓ (m³)".into(), Point::new(left, 4.0), t.muted, false);
        // The concentration: the share of the volume against the share of the road, densest first.
        let (cx0, cy0) = (split + 34.0, pad);
        let side = (bounds.width - cx0 - 14.0).min(bounds.height - pad - foot - 8.0).max(40.0);
        let at = |p: [f64; 2]| Point::new(cx0 + side * p[0] as f32, cy0 + side * (1.0 - p[1]) as f32);
        frame.stroke(&Path::rectangle(Point::new(cx0, cy0), iced::Size::new(side, side)), Stroke::default().with_color(t.border).with_width(1.0));
        frame.stroke(
            &Path::line(at([0.0, 0.0]), at([1.0, 1.0])),
            Stroke { line_dash: canvas::LineDash { segments: &[4.0, 4.0], offset: 0 }, ..Stroke::default().with_color(t.muted).with_width(1.0) },
        );
        let half = at([0.0, 0.5]);
        frame.stroke(&Path::line(half, Point::new(cx0 + side, half.y)), Stroke::default().with_color(t.faint).with_width(1.0));
        let mut legend = Vec::new();
        for c in &l.concentration {
            let color = if c.column == "cut" { t.danger } else { t.accent };
            let line = Path::new(|b| {
                for (j, p) in c.curve.iter().enumerate() {
                    if j == 0 { b.move_to(at(*p)) } else { b.line_to(at(*p)) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(color).with_width(2.0));
            let mark = at([c.half, 0.5]);
            frame.stroke(&Path::line(mark, at([c.half, 0.0])), Stroke::default().with_color(color).with_width(1.0));
            frame.fill(&Path::circle(mark, 3.5), color);
            legend.push((format!("{} {}", if c.column == "cut" { "Kazı" } else { "Dolgu" }, grouped(c.gini, 2)), color));
        }
        // The axes in words would not fit a narrow window: the title names the measure, the banner says the shares.
        text(&mut frame, "Yoğunlaşma (Gini)".into(), Point::new(cx0, 4.0), t.muted, false);
        for (j, (words, color)) in legend.into_iter().enumerate() {
            let at = Point::new(cx0 + side - 74.0, cy0 + side - 34.0 + 15.0 * j as f32);
            frame.fill_rectangle(Point::new(at.x - 10.0, at.y + 6.0), iced::Size::new(8.0, 3.0), color);
            text(&mut frame, words, at, t.text, false);
        }
        vec![frame.into_geometry()]
    }
}

/// How an approval stands: “Geçerli”, “Yerine yenisi geldi · 2026-10-15” or “Kaldırıldı · 2026-10-10 · why”.
pub(super) fn approval_state(e: &kentos_commands::VolumeHistoryEntry) -> String {
    match (&e.approval.ended, e.standing) {
        (_, true) | (None, _) => "Geçerli".into(),
        (Some(end), false) => {
            let mut words = if end.revoked { "Kaldırıldı".to_owned() } else { "Yerine yenisi geldi".to_owned() };
            if let Some(at) = &end.at {
                words += &format!(" · {at}");
            }
            if let Some(n) = &end.note {
                words += &format!(" · {n}");
            }
            words
        }
    }
}

/// The approvals, oldest first, each against the one before it; today against the last.
pub(super) fn history_table(h: &kentos_commands::VolumeHistoryRead) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let change = |c: Option<&kentos_commands::VolumeChange>| -> [String; 3] {
        match c {
            None => ["—".into(), "—".into(), "—".into()],
            Some(c) => [signed(c.cut_change), signed(c.fill_change), c.rows.len().to_string()],
        }
    };
    let mut rows: Vec<Vec<String>> = h
        .approvals
        .iter()
        .map(|e| {
            let a = &e.approval;
            let [dc, df, n] = change(e.against.as_ref());
            vec![
                if a.number == 0 { "—".into() } else { a.number.to_string() },
                a.at.clone(),
                approval_state(e),
                grouped(a.cut, 1),
                grouped(a.fill, 1),
                dc,
                df,
                n,
                a.note.clone().unwrap_or_default(),
            ]
        })
        .collect();
    match (&h.today, &h.problem) {
        (Some(t), _) => {
            let [dc, df, n] = change(h.since.as_ref());
            rows.push(vec!["Bugün".into(), String::new(), "Son onaya göre".into(), grouped(t.cut, 1), grouped(t.fill, 1), dc, df, n, String::new()]);
        }
        (None, Some(why)) => rows.push(vec!["Bugün".into(), String::new(), why.clone(), "—".into(), "—".into(), String::new(), String::new(), String::new(), String::new()]),
        _ => {}
    }
    road_dialog::sheet_fill(
        vec![
            col("#", 40.0),
            col("Gün", 90.0),
            col("Durum", 200.0),
            num("Kazı (m³)", 90.0),
            num("Dolgu (m³)", 86.0),
            num("Kazı farkı", 80.0),
            num("Dolgu farkı", 80.0),
            num("Değişen aralık", 96.0),
            col("Not", 180.0),
        ],
        rows,
        "Onay yok",
    )
}

impl canvas::Program<Message> for Approvals<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let h = self.read;
        let small = Pixels(typography::caption());
        let text = |frame: &mut canvas::Frame, words: String, at: Point, color: Color| {
            frame.fill_text(Text {
                content: words,
                position: at,
                color,
                size: small,
                font: typography::ui(),
                align_x: iced::alignment::Horizontal::Center.into(),
                ..Text::default()
            });
        };
        let mut points: Vec<(String, f64, f64, u8)> = h
            .approvals
            .iter()
            .map(|e| {
                let a = &e.approval;
                let state = match (&a.ended, e.standing) {
                    (_, true) | (None, _) => 0,
                    (Some(end), _) if end.revoked => 2,
                    _ => 1,
                };
                let name = if a.number == 0 { a.at.clone() } else { format!("#{} · {}", a.number, a.at) };
                (name, a.cut, a.fill, state)
            })
            .collect();
        if let Some(today) = &h.today {
            points.push(("Bugün".into(), today.cut, today.fill, 3));
        }
        if points.is_empty() {
            text(&mut frame, "Onay yok".into(), Point::new(bounds.width / 2.0, bounds.height / 2.0), t.muted);
            return vec![frame.into_geometry()];
        }
        // Two strips, the cut's over the fill's, each on its own range: from 0 a hakediş's +96 m³ on 25.878 m³ was
        // a flat line.
        let (left, top, foot, gap) = (78.0_f32, 20.0_f32, 22.0_f32, 14.0_f32);
        let w = bounds.width - left - 16.0;
        let strip = (bounds.height - top - foot - gap) / 2.0;
        let slot = w / points.len() as f32;
        for (k, color, y0) in [(1usize, t.danger, top), (2, t.accent, top + strip + gap)] {
            let value = |p: &(String, f64, f64, u8)| if k == 1 { p.1 } else { p.2 };
            let (lo, hi) = points.iter().map(value).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)));
            let span = (hi - lo).max(1e-3 * hi.abs().max(1.0));
            let (lo, hi) = (lo - 0.25 * span, hi + 0.35 * span);
            let at = |i: usize, v: f64| Point::new(left + slot * (i as f32 + 0.5), y0 + strip * (1.0 - ((v - lo) / (hi - lo)) as f32));
            frame.stroke(
                &Path::rectangle(Point::new(left, y0), iced::Size::new(w, strip)),
                Stroke::default().with_color(Color { a: 0.35, ..t.border }).with_width(1.0),
            );
            frame.fill_text(Text {
                content: if k == 1 { "Kazı (m³)".into() } else { "Dolgu (m³)".into() },
                position: Point::new(left - 6.0, y0 + strip / 2.0 - 7.0),
                color,
                size: small,
                align_x: iced::alignment::Horizontal::Right.into(),
                ..Text::default()
            });
            let line = Path::new(|b| {
                for (i, p) in points.iter().enumerate() {
                    let q = at(i, value(p));
                    if i == 0 { b.move_to(q) } else { b.line_to(q) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(color).with_width(1.6));
            for (i, p) in points.iter().enumerate() {
                let q = at(i, value(p));
                text(&mut frame, grouped(value(p), 1), Point::new(q.x, q.y - 20.0), color);
                match p.3 {
                    0 => frame.fill(&Path::circle(q, 5.0), color),
                    3 => frame.fill(&Path::rectangle(Point::new(q.x - 4.5, q.y - 4.5), iced::Size::new(9.0, 9.0)), color),
                    _ => {
                        frame.fill(&Path::circle(q, 5.0), t.surface);
                        frame.stroke(&Path::circle(q, 5.0), Stroke::default().with_color(color).with_width(1.6));
                        if p.3 == 2 {
                            let d = 6.0;
                            frame.stroke(&Path::line(Point::new(q.x - d, q.y - d), Point::new(q.x + d, q.y + d)), Stroke::default().with_color(t.danger).with_width(1.6));
                            frame.stroke(&Path::line(Point::new(q.x - d, q.y + d), Point::new(q.x + d, q.y - d)), Stroke::default().with_color(t.danger).with_width(1.6));
                        }
                    }
                }
            }
        }
        let at = |i: usize, _: f64| Point::new(left + slot * (i as f32 + 0.5), 0.0);
        for (i, p) in points.iter().enumerate() {
            let q = at(i, 0.0);
            text(&mut frame, p.0.clone(), Point::new(q.x, bounds.height - foot + 4.0), if p.3 == 3 { t.text } else { t.muted });
        }
        frame.fill_text(Text { content: "● geçerli  ○ yerine yenisi geldi  ⊗ kaldırıldı  ■ bugün".into(), position: Point::new(left, 2.0), color: t.muted, size: small, ..Text::default() });
        vec![frame.into_geometry()]
    }
}

/// The states' totals in a sentence (docs/adr/0957).
pub(super) fn states_words(st: &kentos_commands::VolumeStatesRead) -> String {
    let mass = match (st.cut_mass, st.fill_mass) {
        (Some(a), Some(b)) => format!(" Kütle: kazı {} t, dolgu {} t.", grouped(a, 0), grouped(b, 0)),
        _ => " Kütle her satırda yoğunluk verilince okunur.".into(),
    };
    format!(
        "Kazı yerinde {} m³, gevşek {} m³; dolgu sıkışık {} m³, istediği gevşek {} m³; fazla (gevşek) {} m³.{mass}",
        grouped(st.cut, 0),
        grouped(st.cut_loose, 0),
        grouped(st.fill, 0),
        grouped(st.fill_loose, 0),
        grouped(st.balance, 0),
    )
}

/// Each factor row's earthwork in its states (docs/adr/0957): the cut in place → loosened, the fill compacted → the
/// loose it asks, the bank it asks; a row whose factors or densities the soil cannot have is marked (the warning says
/// why; the sources are in the form's densities).
pub(super) fn states_table(st: &kentos_commands::VolumeStatesRead) -> Element<'_, Message> {
    let m3 = |v: Option<f64>| v.map_or_else(|| "—".into(), |v| grouped(v, 0));
    let k = |v: Option<f64>| v.map_or_else(|| "—".into(), |v| grouped(v, 3));
    let mut rows: Vec<Vec<String>> = st
        .rows
        .iter()
        .map(|r| {
            vec![
                format!("{}{} – {}", if r.checks.is_empty() { "" } else { "⚠ " }, r.from_text, r.to_text),
                match &r.densities {
                    Some(d) => format!("{} / {} / {}", grouped(d.bank, 2), grouped(d.loose, 2), grouped(d.compacted, 2)),
                    None => format!("{} · {}", k(r.cut_factor), k(r.fill_factor)),
                },
                format!("{} → {}", grouped(r.cut, 0), grouped(r.cut_loose, 0)),
                format!("{} → {}", grouped(r.fill, 0), grouped(r.fill_loose, 0)),
                m3(r.fill_bank),
            ]
        })
        .collect();
    if st.unfactored_cut > 0.0 || st.unfactored_fill > 0.0 {
        rows.push(vec![
            "katsayısız".into(),
            "—".into(),
            format!("{} → {}", grouped(st.unfactored_cut, 0), grouped(st.unfactored_cut, 0)),
            format!("{} → {}", grouped(st.unfactored_fill, 0), grouped(st.unfactored_fill, 0)),
            "—".into(),
        ]);
    }
    road_dialog::table(
        vec![
            road_dialog::col("Km", 152.0),
            road_dialog::col("ρ y/g/s · katsayı", 103.0),
            road_dialog::num("Kazı → gevşek", 115.0),
            road_dialog::num("Dolgu → ister gevşek", 135.0),
            road_dialog::Col { title: "ister yerinde", width: None, number: true },
        ],
        rows,
        "Katsayı satırı yok",
    )
}
