//! Emsal ve TAKS rozeti (docs/adr/8030; ADV-22, M30): the drawing's floor area ratio and coverage against the parcel's
//! zoning, live in the status bar while the project has a zoning, in the danger colour when one passes its limit;
//! clicked, Proje ayarları on its Mimari page, where İmar durumu is typed and the ratios are listed.
//!
//! The report reads every room's contour (`Document::zoning_report`): it is worked out once for each revision of the
//! drawing, not at every frame.

use std::cell::RefCell;

use iced::Element;
use iced::widget::text;
use kentos_ui::label;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::Tip;
use kentos_ui::widget::status_bar::Readout;

use crate::app::{App, Message};

thread_local! {
    /// The last report: the drawing it was read from (its address), its revision and the report.
    static LAST: RefCell<Option<(usize, u64, Option<kentos_domain::zoning::ZoningReport>)>> = const { RefCell::new(None) };
    /// The last site plan read (docs/adr/8032), kept the same way.
    static SITE: RefCell<Option<(usize, u64, Option<kentos_domain::site::SiteReport>)>> = const { RefCell::new(None) };
    /// The last plan decisions' lines (docs/adr/8072 §5), kept the same way.
    static PLAN: RefCell<Option<(usize, u64, Vec<(String, bool)>)>> = const { RefCell::new(None) };
}

/// Parselin plan kararları for İmar durumu (docs/adr/8072 §5; `cad.zoning.parcel` on the site plan's parcel): each
/// decision with its values as written, its plan (the source and its approval) and its notes; the parcel's area under
/// none, two decisions saying different things over the same piece, the typed emsal or TAKS the plan writes otherwise
/// (these in the warning colour), what else is said. Empty without the site plan's parcel. Read again only when the
/// drawing changed.
pub fn plan_lines(doc: &kentos_domain::Document) -> Vec<(String, bool)> {
    let key = (std::ptr::from_ref(doc) as usize, doc.revision());
    PLAN.with(|last| {
        let mut last = last.borrow_mut();
        match &*last {
            Some((at, rev, lines)) if (*at, *rev) == key => lines.clone(),
            _ => {
                let lines = plan_words(doc);
                *last = Some((key.0, key.1, lines.clone()));
                lines
            }
        }
    })
}

/// A decision field's words as İmar durumu says them.
fn field_words(field: &str) -> &str {
    match field {
        "Kullanim" => "Kullanım",
        "KatAdedi" => "Kat adedi",
        "OnCekme" => "Ön çekme",
        "YanCekme" => "Yan çekme",
        "ArkaCekme" => "Arka çekme",
        "Belirtilmeyen" => "Planda verilmeyen",
        other => other,
    }
}

fn plan_words(doc: &kentos_domain::Document) -> Vec<(String, bool)> {
    if doc.settings().arch.as_ref().and_then(|a| a.site.as_ref()).and_then(|s| s.parcel.as_ref()).is_none() {
        return Vec::new();
    }
    let input = kentos_commands::ZoningParcel::default();
    let a = match kentos_native_application::zoning_parcel::plan_in(doc, &input) {
        kentos_commands::CommandResult::Completed { output, .. } => output,
        _ => return Vec::new(),
    };
    let m2 = |v: f64| format!("{} m²", n2(v));
    let mut out: Vec<(String, bool)> = Vec::new();
    for d in &a.decisions {
        let values: Vec<String> = d.values.iter().map(|v| format!("{} {}", field_words(&v.field), v.value)).collect();
        out.push((format!("{} ({}): {}", d.id, m2(d.share), values.join(" · ")), false));
        if let Some(p) = &d.plan {
            let mut source = p.id.clone();
            if let Some(n) = &p.name {
                source.push_str(&format!(" {n}"));
            }
            let mut said: Vec<String> = Vec::new();
            if let Some(st) = &p.status {
                said.push(st.clone());
            }
            if let Some(o) = &p.approved {
                said.push(format!("onay {o}"));
            }
            if let Some(b) = &p.basis {
                said.push(b.clone());
            }
            if !said.is_empty() {
                source.push_str(&format!(" ({})", said.join(", ")));
            }
            out.push((format!("Kaynak: {source}"), false));
        }
        for n in &d.notes {
            out.push((format!("Plan notu {}: {}", n.number, n.text), false));
        }
    }
    if a.uncovered > 0.0 {
        out.push((format!("Hiçbir kararın altında olmayan alan: {}", m2(a.uncovered)), true));
    }
    for c in &a.conflicts {
        out.push((
            format!(
                "Çelişki: {} aynı yerde ({}) {} için “{}” yazıyor; ikisi de gösterilir, KentOS seçmez.",
                c.decisions.join(" ve "),
                m2(c.area),
                field_words(&c.field),
                c.values.join("” ve “")
            ),
            true,
        ));
    }
    for g in &a.against {
        let typed = if g.field == "KAKS" { "emsal" } else { "TAKS" };
        out.push((format!("İmar durumunda {typed} {}; plan kararı {} {}.", n2(g.typed), g.field, g.plan.join(" / ")), true));
    }
    out.extend(a.notes.iter().map(|n| (n.clone(), false)));
    out
}

/// The drawing's site plan (docs/adr/8032), read again only when the drawing changed.
pub fn site(doc: &kentos_domain::Document) -> Option<kentos_domain::site::SiteReport> {
    let key = (std::ptr::from_ref(doc) as usize, doc.revision());
    SITE.with(|last| {
        let mut last = last.borrow_mut();
        match &*last {
            Some((at, rev, r)) if (*at, *rev) == key => r.clone(),
            _ => {
                let r = doc.site_report();
                *last = Some((key.0, key.1, r.clone()));
                r
            }
        }
    })
}

/// The edges whose garden distance the footprint does not keep (docs/adr/8032), said: “2. kenar (Yan bahçe) 2,80 <
/// 3,00 m”.
pub fn short_edges(doc: &kentos_domain::Document) -> Vec<String> {
    site(doc).map_or_else(Vec::new, |r| {
        r.edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.short())
            .map(|(i, e)| {
                format!(
                    "{}. kenar ({}) {} < {} m",
                    i + 1,
                    e.kind.label(),
                    n2(e.drawn.unwrap_or(0.0)),
                    n2(e.required)
                )
            })
            .collect()
    })
}

/// The drawing's report, read again only when the drawing changed.
fn report(doc: &kentos_domain::Document) -> Option<kentos_domain::zoning::ZoningReport> {
    let key = (std::ptr::from_ref(doc) as usize, doc.revision());
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        match &*last {
            Some((at, rev, r)) if (*at, *rev) == key => r.clone(),
            _ => {
                let r = doc.zoning_report();
                *last = Some((key.0, key.1, r.clone()));
                r
            }
        }
    })
}

fn n2(v: f64) -> String {
    // An empty sum is −0 (IEEE): said as 0.
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.2}").replace('.', ",")
}

/// The badge's words: the ratios with their limits when `full`, else the ratios alone (the limits in its tip); and
/// whether one passes its limit.
pub fn words(doc: &kentos_domain::Document, full: bool) -> Option<(String, bool)> {
    let r = report(doc)?;
    let part = |name: &str, v: f64, l: Option<f64>| match l {
        Some(l) if full => format!("{name} {}/{}", n2(v), n2(l)),
        _ => format!("{name} {}", n2(v)),
    };
    Some((
        format!(
            "{} · {}",
            part("Emsal", r.far(), r.far_limit),
            part("TAKS", r.coverage(), r.coverage_limit)
        ),
        r.far_over() || r.coverage_over() || site(doc).is_some_and(|s| s.short()),
    ))
}

/// İmar durumu's lines in Proje ayarları: each one's words, value and whether it passes its limit.
pub fn summary(doc: &kentos_domain::Document) -> Vec<(String, String, bool)> {
    let Some(r) = report(doc) else {
        return Vec::new();
    };
    let m2 = |v: f64| format!("{} m²", n2(v));
    let mut out = vec![
        (
            "Katlar alanı (emsale dahil)".to_owned(),
            m2(r.included),
            false,
        ),
        (
            "Emsal harici (%30 içinde)".to_owned(),
            match r.cap {
                Some(c) => format!("{} · sınır {}", m2(r.exempt), m2(c)),
                None => m2(r.exempt),
            },
            r.over > 0.0,
        ),
    ];
    if r.over > 0.0 {
        out.push(("Sınırı aşan, emsale eklenen".to_owned(), m2(r.over), true));
    }
    if r.outside > 0.0 {
        out.push((
            "Emsal harici (%30'a sayılmaz)".to_owned(),
            m2(r.outside),
            false,
        ));
    }
    out.push((
        "Emsal".to_owned(),
        match r.far_limit {
            Some(l) => format!("{} / {}", n2(r.far()), n2(l)),
            None => n2(r.far()),
        },
        r.far_over(),
    ));
    out.push(("Taban alanı (zemin kat)".to_owned(), m2(r.footprint), false));
    out.push((
        "TAKS".to_owned(),
        match r.coverage_limit {
            Some(l) => format!("{} / {}", n2(r.coverage()), n2(l)),
            None => n2(r.coverage()),
        },
        r.coverage_over(),
    ));
    out
}

/// The status bar's cell: its words with the limits (`Some(true)`), without (`Some(false)`), or its icon alone and a
/// “!” past a limit (`None`, the bar tight: the numbers in its tip); none without a zoning.
pub fn cell(app: &App, shown: Option<bool>) -> Option<Element<'_, Message>> {
    let doc = app.document.as_ref()?;
    let (words, over) = words(&doc.model, shown.unwrap_or(false))?;
    let words = match shown {
        Some(_) => words,
        None if over => "!".to_owned(),
        None => String::new(),
    };
    let r = report(&doc.model)?;
    let limit = |l: Option<f64>| l.map_or_else(|| "verilmedi".to_owned(), n2);
    let body = format!(
        "Emsal {} (sınır {}), TAKS {} (sınır {}). Katlar alanı {} m², emsal harici {} m², taban alanı {} m², parsel {} m². \
         {}Tıklayın: Proje ayarları › Mimari.",
        n2(r.far()),
        limit(r.far_limit),
        n2(r.coverage()),
        limit(r.coverage_limit),
        n2(r.floors),
        n2(r.exempt + r.outside),
        n2(r.footprint),
        n2(r.parcel),
        if over { "Sınır aşıldı. " } else { "" }
    );
    // The garden distances not kept (docs/adr/8032).
    let short = short_edges(&doc.model);
    let body = if short.is_empty() {
        body
    } else {
        format!("Bahçe mesafesi dar: {}. {body}", short.join("; "))
    };
    Some(
        Readout::new(
            label::strong(words).style(move |theme: &iced::Theme| text::Style {
                color: over.then(|| Tokens::of(theme).danger),
            }),
        )
        .icon(crate::icons::from_web(Some("zoningTable")))
        .tip(Tip::new("Emsal ve TAKS").body(body))
        .on_press(Message::Run("arch.settings"))
        .into(),
    )
}
