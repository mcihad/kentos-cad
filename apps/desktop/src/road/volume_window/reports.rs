//! Kübaj's sheet beside the window: its approvals' sources, rules and layers, the budget, progress and
//! comparison tables and their charts.

use super::*;

/// What moved since an approval, as a sentence after the approval's (docs/adr/1041): " Onaydan beri değişen: aks g-1
/// (1 → 2), arazi t-1 (df9eb6e9… → 2751c2db…)."; nothing where none moved; that the approval does not keep its sources
/// where it was made before it could.
pub(crate) fn approval_sources_words(a: &kentos_commands::VolumeApprovalReport) -> String {
    use kentos_commands::VolumeSourceKind as K;
    if a.approved.sources.is_none() {
        return " Bu onay hesaplandığı kaynakları saklamıyor (eski onay).".to_owned();
    }
    if a.moved.is_empty() {
        return String::new();
    }
    let value = |m: &kentos_commands::VolumeSourceMove, v: &Option<String>| match v {
        None => "yok".to_owned(),
        Some(d) if m.source == K::Surface => format!("{}…", &d[..d.len().min(8)]),
        Some(v) => v.clone(),
    };
    let words: Vec<String> = a
        .moved
        .iter()
        .map(|m| {
            let what = match m.source {
                K::Corridor => "koridor",
                K::Alignment => "aks",
                K::Profile => "boykesit",
                K::Template => "tip kesit",
                K::Superelevation => "dever",
                K::Surface => "arazi",
                K::Algorithm => "algoritma",
            };
            format!("{what} {} ({} → {})", m.id, value(m, &m.then), value(m, &m.now))
        })
        .collect();
    format!(" Onaydan beri değişen: {}.", words.join(", "))
}

/// The road project's volumes as saved.
pub(super) fn doc_volumes(app: &App) -> Vec<VolumeDef> {
    app.document.as_ref().map_or_else(Vec::new, |d| d.model.road_volumes().to_vec())
}

/// The line under the mass diagram (the diagram has its own legend): the rule the volume measures by and, as the
/// query's basis says it (docs/adr/1037), between which lines and sides.
pub(super) fn rule_words(rep: &VolumeReport) -> String {
    use kentos_commands::{VolumeLine, VolumeSides};
    let Some(b) = kentos_native_application::road_volumes::basis_of(rep) else { return "Kural: —".to_owned() };
    let line = |l: VolumeLine| match l {
        VolumeLine::Road => "yol üstü",
        VolumeLine::Formation => "formasyon",
        VolumeLine::Ground => "zemin",
        VolumeLine::Topsoil => "sıyrılmış zemin",
    };
    let sides = match b.sides {
        VolumeSides::Daylights => "şev uçları arası",
        VolumeSides::Corridor => "koridor yüzeyi",
    };
    let given = if b.given { "" } else { " (kuralsız)" };
    format!("Kural: {}{given}: {}–{}, {sides}", b.name, line(b.top), line(b.bottom))
}

/// The pavement layers' volumes (docs/adr/0975 §2): each layer's name, material and volume in all.
pub(super) fn layers_table(rep: &VolumeReport) -> Element<'_, Message> {
    road_dialog::table(
        vec![road_dialog::col("Tabaka", 200.0), road_dialog::col("Malzeme kodu", 140.0), road_dialog::num("Hacim (m³)", 120.0)],
        rep.layers
            .iter()
            .map(|l| vec![l.name.clone(), l.material.clone().unwrap_or_default(), grouped(l.volume, 2)])
            .collect(),
        "Tip kesitlerde üstyapı yok",
    )
}

/// The sheet's rows as the window shows them: km, cut and fill, the factors, the running totals and the mass ordinate.
pub(super) fn sheet_table(rep: &VolumeReport) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let (strips, benches) = (rep.strip.is_some(), rep.bench.is_some());
    let mut cols = vec![col("Baş km", 88.0), col("Son km", 88.0), num("Kazı (m³)", 88.0), num("Dolgu (m³)", 88.0)];
    if strips {
        cols.push(num("B. toprak (m³)", 108.0));
    }
    if benches {
        cols.push(num("Diş (m³)", 80.0));
    }
    cols.extend([num("Kabarma", 72.0), num("Sıkışma", 72.0), num("Σ kazı (m³)", 100.0), num("Σ dolgu (m³)", 100.0), num("Kütle (m³)", 100.0)]);
    let opt = |v: Option<f64>, d: usize| v.map_or_else(|| "—".to_owned(), |x| grouped(x, d));
    let rows = rep
        .rows
        .iter()
        .map(|r| {
            let mut cells = vec![r.from_text.clone(), r.to_text.clone(), opt(r.cut, 1), opt(r.fill, 1)];
            if strips {
                cells.push(opt(r.strip, 1));
            }
            if benches {
                cells.push(opt(r.bench, 1));
            }
            cells.extend([
                r.cut_factor.map_or_else(|| "—".into(), |x| trimmed(x).replace('.', ",")),
                r.fill_factor.map_or_else(|| "—".into(), |x| trimmed(x).replace('.', ",")),
                grouped(r.total_cut, 1),
                grouped(r.total_fill, 1),
                grouped(r.mass, 1),
            ]);
            cells
        })
        .collect();
    road_dialog::sheet(cols, rows, "Kesit aralığı yok", 220.0)
}

/// What a line of the mass diagram reads of a row.
pub(super) type Value = fn(&kentos_commands::VolumeRow) -> f64;

impl canvas::Program<Message> for Mass<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let caption = |frame: &mut canvas::Frame, words: String, at: Point, color: Color| {
            frame.fill_text(Text {
                content: words,
                position: at,
                color,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                ..Text::default()
            });
        };
        let Some(rep) = self.preview.report.as_ref().filter(|r| !r.rows.is_empty()) else {
            caption(&mut frame, "Koridoru ve yöntemi seçince kütle diyagramı burada çizilir.".into(), Point::new(16.0, 20.0), t.muted);
            return vec![frame.into_geometry()];
        };
        let (s0, s1) = (rep.rows[0].from, rep.rows[rep.rows.len() - 1].to);
        let mut lo: f64 = 0.0;
        let mut hi: f64 = 0.0;
        for r in &rep.rows {
            for v in [r.total_cut, r.total_fill, r.mass] {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        if hi - lo < 1e-9 {
            hi = lo + 1.0;
        }
        let (left, pad) = (64.0, 12.0);
        let (wd, ht) = (bounds.width as f64 - left - pad, bounds.height as f64 - 2.0 * pad - 14.0);
        let at = |s: f64, v: f64| {
            Point::new((left + (s - s0) / (s1 - s0).max(1e-9) * wd) as f32, (pad + (hi - v) / (hi - lo) * ht) as f32)
        };
        // Zoomed in a picture (docs/adr/0937 §3): round volumes at least 40 px apart, finer as the chart is laid out
        // larger; their labels and the legend stay on the part shown.
        let part = crate::picture::Shown::of(bounds.size());
        let step = crate::picture::step(40.0 * (hi - lo) / ht.max(1.0));
        let mut volumes = Vec::new();
        let mut v = (lo / step).ceil() * step;
        while v <= hi + 1e-9 {
            let a = at(s0, v);
            if part.has_y(a.y) {
                frame.stroke(&Path::line(a, at(s1, v)), Stroke::default().with_color(Color { a: 0.35, ..t.border }).with_width(1.0));
                volumes.push(v);
            }
            v += step;
        }
        frame.stroke(&Path::line(at(s0, 0.0), at(s1, 0.0)), Stroke::default().with_color(t.muted).with_width(1.0));
        let green = Color::from_rgb8(0x16, 0xa3, 0x4a);
        let series: [(Value, Color); 3] = [(|r| r.total_cut, t.danger), (|r| r.total_fill, t.accent), (|r| r.mass, green)];
        for (k, (value, color)) in series.iter().enumerate() {
            let line = Path::new(|b| {
                b.move_to(at(s0, 0.0));
                for r in &rep.rows {
                    b.line_to(at(r.to, value(r)));
                }
            });
            let width = if k == 2 { 1.4 } else { 2.0 };
            frame.stroke(&line, Stroke::default().with_color(*color).with_width(width));
        }
        // The stretches without volume, under the axis.
        let orange = Color::from_rgb8(0xea, 0x58, 0x0c);
        for g in &rep.gaps {
            let (a, b) = (at(g[0].max(s0), lo), at(g[1].min(s1), lo));
            frame.fill_rectangle(Point::new(a.x, a.y + 2.0), iced::Size::new((b.x - a.x).max(2.0), 4.0), orange);
        }
        let label_x = part.left(4.0);
        for v in volumes {
            let a = at(s0, v);
            crate::picture::label(&mut frame, grouped(v, 0), Point::new(label_x, a.y - 7.0), t.muted, false, (label_x > 4.5).then_some(t.field.scale_alpha(0.9)));
        }
        // The legend at the bottom of the part shown, on its own ground where it moved up over the drawing.
        let base = part.bottom(bounds.height - 18.0, 12.0);
        let mut x = part.left(left as f32 + 8.0);
        // Both curves loosened (docs/adr/0957): the cut, and what the fill asks. Each entry as wide as its words.
        let entries = [("Σ kazı (gevşek)", t.danger), ("Σ dolgunun istediği (gevşek)", t.accent), ("Kütle ordinatı", green)];
        let wide = |w: &str| 18.0 + kentos_ui::theme::typography::current().text_width(w, kentos_ui::theme::typography::caption(), false) + 14.0;
        let total: f32 = entries.iter().map(|e| wide(e.0)).sum();
        if base < bounds.height - 18.5 || x > left as f32 + 8.5 {
            frame.fill(&Path::rectangle(Point::new(x - 6.0, base - 4.0), iced::Size::new(total + 8.0, 22.0)), t.field.scale_alpha(0.9));
        }
        for (words, color) in entries {
            frame.fill_rectangle(Point::new(x, base + 6.0), iced::Size::new(14.0, 3.0), color);
            caption(&mut frame, words.into(), Point::new(x + 18.0, base), t.text);
            x += wide(words);
        }
        vec![frame.into_geometry()]
    }
}

/// The budget in a sentence (docs/adr/0941): the bounds at 95 %, their shares, the method's and the arithmetic's.
pub(super) fn budget_words(b: &kentos_commands::VolumeUncertaintyReport) -> String {
    let share = |s: &kentos_commands::VolumeSpread| {
        if s.value.abs() > 0.0 { format!(" (%{})", grouped(100.0 * s.bound / s.value.abs(), 1)) } else { String::new() }
    };
    let method = match (b.other_method, b.other_cut, b.other_fill) {
        (Some(m), Some(c), Some(f)) => {
            let name = METHODS.iter().find(|x| x.0 == m).map_or("öbür yöntem", |x| x.1);
            format!(" {name} kazıyı {} m³, dolguyu {} m³ farklı verir.", signed(c - b.cut.value), signed(f - b.fill.value))
        }
        _ => String::new(),
    };
    format!(
        "Ölçme belirsizliği (σ {} m, {} nokta, %95): kazı ±{} m³{}, dolgu ±{} m³{}, net ±{} m³.{method} Aritmetiğin payı {:.0e} m³.",
        grouped(b.sigma, 3),
        b.points,
        grouped(b.cut.bound, 1),
        share(&b.cut),
        grouped(b.fill.bound, 1),
        share(&b.fill),
        grouped(b.net.bound, 1),
        b.numeric
    )
}

/// The budget's table: the cut, the fill and their net with σ, the shift's part, the 95 % bound, its share, accepted.
pub(super) fn budget_table(b: &kentos_commands::VolumeUncertaintyReport) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let row = |name: &str, s: &kentos_commands::VolumeSpread| {
        vec![
            name.to_owned(),
            grouped(s.value, 1),
            grouped(s.sigma, 1),
            signed(s.bias),
            grouped(s.bound, 1),
            if s.value.abs() > 0.0 { format!("%{}", grouped(100.0 * s.bound / s.value.abs(), 2)) } else { "—".into() },
            match s.accepted {
                Some(true) => "tutar".into(),
                Some(false) => "aşıyor".into(),
                None => "—".into(),
            },
        ]
    };
    road_dialog::table(
        vec![
            col("Hacim", 64.0),
            num("Değer (m³)", 96.0),
            num("σ (m³)", 72.0),
            num("Kayma (m³)", 84.0),
            num("%95 sınır (m³)", 104.0),
            num("Oran", 64.0),
            col("Kabul", 64.0),
        ],
        vec![row("Kazı", &b.cut), row("Dolgu", &b.fill), row("Net", &b.net)],
        "",
    )
}

/// The progress in a sentence (docs/adr/0942): the share of the cut and the fill done, what is left, what went past the
/// design, and the period's work.
pub(super) fn progress_words(
    p: &kentos_commands::VolumeProgressReport,
    surfaces: &[(String, String)],
) -> String {
    let name = |id: &str| surfaces.iter().find(|s| s.0 == id).map_or_else(|| id.to_owned(), |s| s.1.clone());
    let n = &p.now;
    let part = |what: &str, share: Option<f64>, done: f64, left: f64| match share {
        Some(s) => format!("{what} %{} yapıldı ({} m³), {} m³ kaldı", grouped(s, 1), grouped(done, 0), grouped(left, 0)),
        None => format!("{what} yok"),
    };
    let mut out = format!(
        "{}: {}; {}.",
        name(&p.survey),
        part("kazının", p.cut_share, n.cut_done, n.cut_left),
        part("dolgunun", p.fill_share, n.fill_done, n.fill_left)
    );
    if n.cut_over >= 0.5 || n.fill_over >= 0.5 {
        out += &format!(" Tasarımın ötesinde: kazı {} m³, dolgu {} m³.", grouped(n.cut_over, 0), grouped(n.fill_over, 0));
    }
    if let (Some(prev), Some(d)) = (&p.previous, &p.period) {
        out += &format!(" Dönemde (önceki ölçü “{}”) kazı {} m³, dolgu {} m³.", name(prev), signed(d.cut_done), signed(d.fill_done));
    }
    out
}

/// The progress by stretch: done, left and past the design, for the cut and the fill (m³); the plan area the survey
/// leaves uncovered (m²).
pub(super) fn progress_table(p: &kentos_commands::VolumeProgressReport) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let rows = p
        .stretches
        .iter()
        .map(|s| {
            let a = &s.amounts;
            let share = |done: f64, left: f64| {
                if done + left > 0.05 { format!("%{}", grouped(100.0 * done / (done + left), 0)) } else { "—".into() }
            };
            vec![
                s.from_text.clone(),
                s.to_text.clone(),
                grouped(a.cut_done, 1),
                grouped(a.cut_left, 1),
                grouped(a.cut_over, 1),
                share(a.cut_done, a.cut_left),
                grouped(a.fill_done, 1),
                grouped(a.fill_left, 1),
                grouped(a.fill_over, 1),
                share(a.fill_done, a.fill_left),
                if s.uncovered >= 0.05 { grouped(s.uncovered, 1) } else { "—".into() },
            ]
        })
        .collect();
    road_dialog::sheet(
        vec![
            col("Baş km", 88.0),
            col("Son km", 88.0),
            num("Kazı yapılan", 96.0),
            num("Kazı kalan", 88.0),
            num("Fazla kazı", 84.0),
            num("Kazı %", 64.0),
            num("Dolgu yapılan", 100.0),
            num("Dolgu kalan", 92.0),
            num("Fazla dolgu", 88.0),
            num("Dolgu %", 68.0),
            num("Örtülmeyen (m²)", 116.0),
        ],
        rows,
        "Kesit aralığı yok",
        220.0,
    )
}

impl canvas::Program<Message> for Works<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let caption = |frame: &mut canvas::Frame, words: String, at: Point, color: Color| {
            frame.fill_text(Text {
                content: words,
                position: at,
                color,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                ..Text::default()
            });
        };
        let bars: Vec<_> = self.report.stretches.iter().filter(|s| s.to - s.from > 1e-6).collect();
        let (Some(first), Some(last)) = (bars.first(), bars.last()) else {
            caption(&mut frame, "Koridorun kesit aralığı yok.".into(), Point::new(16.0, 20.0), t.muted);
            return vec![frame.into_geometry()];
        };
        let (s0, s1) = (first.from, last.to);
        let per = |v: f64, s: &kentos_commands::StretchProgress| v / (s.to - s.from);
        let (mut lo, mut hi): (f64, f64) = (0.0, 0.0);
        for s in &bars {
            let a = &s.amounts;
            hi = hi.max(per(a.cut_done + a.cut_left + a.cut_over, s));
            lo = lo.min(-per(a.fill_done + a.fill_left + a.fill_over, s));
        }
        if hi - lo < 1e-9 {
            hi = lo + 1.0;
        }
        let (left, pad) = (64.0, 12.0);
        let (wd, ht) = (bounds.width as f64 - left - pad, bounds.height as f64 - 2.0 * pad - 14.0);
        let at = |s: f64, v: f64| {
            Point::new((left + (s - s0) / (s1 - s0).max(1e-9) * wd) as f32, (pad + (hi - v) / (hi - lo) * ht) as f32)
        };
        let part = crate::picture::Shown::of(bounds.size());
        let step = crate::picture::step(40.0 * (hi - lo) / ht.max(1.0));
        let mut marks = Vec::new();
        let mut v = (lo / step).ceil() * step;
        while v <= hi + 1e-9 {
            let a = at(s0, v);
            if part.has_y(a.y) {
                frame.stroke(&Path::line(a, at(s1, v)), Stroke::default().with_color(Color { a: 0.35, ..t.border }).with_width(1.0));
                marks.push(v);
            }
            v += step;
        }
        let orange = Color::from_rgb8(0xea, 0x58, 0x0c);
        // Left: the colour washed towards white, seen on the light and the dark ground alike.
        let pale = |c: Color| Color { r: c.r + (1.0 - c.r) * 0.62, g: c.g + (1.0 - c.g) * 0.62, b: c.b + (1.0 - c.b) * 0.62, a: 1.0 };
        let grey = Color { a: 0.55, ..t.muted };
        // A bar's piece from `a` to `b` m³/m.
        let piece = |frame: &mut canvas::Frame, s: &kentos_commands::StretchProgress, a: f64, b: f64, color: Color| {
            if (b - a).abs() < 1e-12 {
                return;
            }
            let (p, q) = (at(s.from, a.max(b)), at(s.to, a.min(b)));
            frame.fill_rectangle(p, iced::Size::new((q.x - p.x - 1.0).max(1.0), (q.y - p.y).max(0.5)), color);
        };
        for s in &bars {
            let a = &s.amounts;
            let (done, left, over) = (per(a.cut_done, s), per(a.cut_left, s), per(a.cut_over, s));
            piece(&mut frame, s, 0.0, done, t.danger);
            piece(&mut frame, s, done, done + left, pale(t.danger));
            piece(&mut frame, s, done + left, done + left + over, orange);
            let (done, left, over) = (per(a.fill_done, s), per(a.fill_left, s), per(a.fill_over, s));
            piece(&mut frame, s, 0.0, -done, t.accent);
            piece(&mut frame, s, -done, -done - left, pale(t.accent));
            piece(&mut frame, s, -done - left, -done - left - over, orange);
            if s.uncovered >= 0.05 {
                let (p, q) = (at(s.from, lo), at(s.to, lo));
                frame.fill_rectangle(Point::new(p.x, p.y + 2.0), iced::Size::new((q.x - p.x - 1.0).max(1.0), 4.0), grey);
            }
        }
        frame.stroke(&Path::line(at(s0, 0.0), at(s1, 0.0)), Stroke::default().with_color(t.muted).with_width(1.0));
        let label_x = part.left(4.0);
        for v in marks {
            let a = at(s0, v);
            crate::picture::label(&mut frame, grouped(v.abs(), 0), Point::new(label_x, a.y - 7.0), t.muted, false, (label_x > 4.5).then_some(t.field.scale_alpha(0.9)));
        }
        let base = part.bottom(bounds.height - 18.0, 12.0);
        let mut x = part.left(16.0);
        // The legend under the part shown, after the axis's unit.
        let legend = [
            ("Kazı yapılan", t.danger),
            ("Kazı kalan", pale(t.danger)),
            ("Dolgu yapılan", t.accent),
            ("Dolgu kalan", pale(t.accent)),
            ("Tasarımın ötesi", orange),
            ("Örtülmeyen", grey),
        ];
        let wide = |words: &str| 16.0 + 6.4 * words.chars().count() as f32 + 18.0;
        if base < bounds.height - 18.5 || x > 16.5 {
            let all: f32 = legend.iter().map(|(w, _)| wide(w)).sum();
            frame.fill(&Path::rectangle(Point::new(x - 6.0, base - 4.0), iced::Size::new(all + 50.0, 22.0)), t.field.scale_alpha(0.9));
        }
        caption(&mut frame, "m³/m:".into(), Point::new(x, base), t.muted);
        x += 44.0;
        for (words, color) in legend {
            frame.fill_rectangle(Point::new(x, base + 3.0), iced::Size::new(12.0, 9.0), color);
            caption(&mut frame, words.into(), Point::new(x + 16.0, base), t.text);
            x += wide(words);
        }
        vec![frame.into_geometry()]
    }
}

/// The comparison in a sentence (docs/adr/0943): each alternative's departures and its volumes' changes.
pub(super) fn compare_words(c: &kentos_commands::VolumesCompared) -> String {
    let base = c.alternatives.first().map_or("", |a| a.name.as_str());
    let mut out = format!("Taban “{base}”.");
    for a in c.alternatives.iter().skip(1) {
        let Some(g) = &a.against else { continue };
        let stretch = |d: &kentos_commands::DepartureRow, unit: &str| {
            format!("km {}–{} (en çok {} {unit})", d.from_text, d.to_text, grouped(d.largest, 2))
        };
        let plan = if g.plan.is_empty() {
            "aks aynı".to_owned()
        } else {
            format!("aks {} yerde ayrılıyor: {}", g.plan.len(), g.plan.iter().map(|d| stretch(d, "m")).collect::<Vec<_>>().join(", "))
        };
        let height = if g.height.is_empty() {
            String::new()
        } else {
            format!("; kot {}", g.height.iter().map(|d| stretch(d, "m")).collect::<Vec<_>>().join(", "))
        };
        let volumes = g.figures_change.map_or_else(String::new, |f| format!("; kazı {} m³, dolgu {} m³", signed(f.cut), signed(f.fill)));
        out += &format!(" “{}”: {plan}{height}{volumes}.", a.name);
    }
    out
}

/// The comparison's table: each alternative's length, volumes, changes against the base, approval and departures.
pub(super) fn compare_table(c: &kentos_commands::VolumesCompared) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let ranges = |list: &[kentos_commands::DepartureRow]| {
        if list.is_empty() { "—".to_owned() } else { list.iter().map(|d| format!("{}–{}", d.from_text, d.to_text)).collect::<Vec<_>>().join(", ") }
    };
    let rows = c
        .alternatives
        .iter()
        .map(|a| {
            let f = a.figures;
            let g = a.against.as_ref();
            let change = |pick: fn(&kentos_commands::AlternativeFigures) -> f64| {
                g.and_then(|g| g.figures_change.as_ref()).map_or_else(|| "—".to_owned(), |x| signed(pick(x)))
            };
            vec![
                a.name.clone(),
                a.length.map_or_else(|| "—".into(), |v| grouped(v, 1)),
                f.map_or_else(|| "—".into(), |f| grouped(f.cut, 0)),
                f.map_or_else(|| "—".into(), |f| grouped(f.fill, 0)),
                if g.is_some() { change(|x| x.cut) } else { "taban".into() },
                if g.is_some() { change(|x| x.fill) } else { "taban".into() },
                a.approved.as_ref().map_or_else(
                    || "—".into(),
                    |p| if p.changed { format!("{} · kazı {}", p.at, signed(p.cut_change)) } else { format!("{} · aynı", p.at) },
                ),
                g.map_or_else(|| "—".into(), |g| ranges(&g.plan)),
                g.map_or_else(|| "—".into(), |g| ranges(&g.height)),
            ]
        })
        .collect();
    road_dialog::sheet(
        vec![
            col("Kübaj", 120.0),
            num("Aks (m)", 76.0),
            num("Kazı (m³)", 84.0),
            num("Dolgu (m³)", 84.0),
            num("Δ kazı", 80.0),
            num("Δ dolgu", 80.0),
            col("Onay", 150.0),
            col("Planda ayrılan km", 200.0),
            col("Kotta ayrılan km", 200.0),
        ],
        rows,
        "Karşılaştırılacak kübaj yok",
        220.0,
    )
}
