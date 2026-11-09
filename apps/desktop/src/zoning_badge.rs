//! Emsal ve TAKS rozeti (docs/adr/3030; ADV-22, M30): the drawing's floor area ratio and coverage against the parcel's
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
    /// The last site plan read (docs/adr/3032), kept the same way.
    static SITE: RefCell<Option<(usize, u64, Option<kentos_domain::site::SiteReport>)>> = const { RefCell::new(None) };
}

/// The drawing's site plan (docs/adr/3032), read again only when the drawing changed.
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

/// The edges whose garden distance the footprint does not keep (docs/adr/3032), said: “2. kenar (Yan bahçe) 2,80 <
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
    // The garden distances not kept (docs/adr/3032).
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
