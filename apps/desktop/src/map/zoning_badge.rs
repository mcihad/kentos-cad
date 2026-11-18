//! Emsal ve TAKS rozeti (docs/adr/8030; ADV-22, M30): the drawing's floor area ratio and coverage against the parcel's
//! zoning, live in the status bar while the project has a zoning, in the danger colour when one passes its limit;
//! clicked, Proje ayarları on its Mimari page, where İmar durumu is typed and the ratios are listed.
//!
//! The report reads every room's contour (`Document::zoning_report`): it is worked out once for each change of the
//! drawing, not at every frame. The readings are kept by the opened drawing (its session, unique while the program
//! runs: the drawing lives at the same address whichever is open) and its generation (which changes with every change,
//! the ones from outside too).

use std::cell::RefCell;

use iced::Element;
use iced::widget::text;
use kentos_ui::label;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::Tip;
use kentos_ui::widget::status_bar::Readout;

use crate::app::{App, Message};
use crate::document::Document;

thread_local! {
    /// The last report: the drawing it was read from (its session and generation, [`key`]) and the report.
    static LAST: RefCell<Option<((u64, u64), Option<kentos_domain::zoning::ZoningReport>)>> = const { RefCell::new(None) };
    /// The last site plan read (docs/adr/8032), kept the same way.
    static SITE: RefCell<Option<((u64, u64), Option<kentos_domain::site::SiteReport>)>> = const { RefCell::new(None) };
    /// The last plan decisions' lines and what each decision gives to take (docs/adr/8072 §5, 8140), kept the same way.
    static PLAN: RefCell<Option<((u64, u64), PlanRead)>> = const { RefCell::new(None) };
}

/// What a reading is kept by: the opened drawing (its session) and its generation.
fn key(doc: &Document) -> (u64, u64) {
    (doc.session, doc.model.generation())
}

/// The reading of `doc` kept in `cache`, made by `read` when the drawing is another or changed.
fn kept<T: Clone>(
    cache: &'static std::thread::LocalKey<RefCell<Option<((u64, u64), T)>>>,
    doc: &Document,
    read: impl FnOnce(&kentos_domain::Document) -> T,
) -> T {
    let key = key(doc);
    cache.with(|last| {
        let mut last = last.borrow_mut();
        match &*last {
            Some((at, r)) if *at == key => r.clone(),
            _ => {
                let r = read(&doc.model);
                *last = Some((key, r.clone()));
                r
            }
        }
    })
}

/// The plan decisions on the parcel as İmar durumu shows them: the lines, and the values each decision gives that can be
/// taken, with the line they follow.
#[derive(Clone, Debug, Default)]
struct PlanRead {
    lines: Vec<(String, bool)>,
    takes: Vec<(usize, PlanTake)>,
}

/// What a zoning plan decision on the parcel gives that İmar durumu and Plan notları keep (docs/adr/8140): its emsal
/// (KAKS) and TAKS, the plan's building height (Hmax, m) and storey count (KatAdedi), each only when it is written as a
/// number the field takes; and where they come from: the plan (its id and name, state, approval and basis) and the
/// decision.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanTake {
    /// KararKimlik.
    pub decision: String,
    pub far: Option<f64>,
    pub coverage: Option<f64>,
    pub height: Option<f64>,
    pub storeys: Option<f64>,
    /// At most [`kentos_commands::MAX_ZONING_SOURCE`] characters, one line.
    pub source: String,
}

impl PlanTake {
    /// What it writes where, in the form's words: “Emsal 1,20 ve TAKS 0,40 İmar durumu'na, kat adedi 3 Plan
    /// notları'na.”; `held` says where they are: “… İmar durumu'nda, … Plan notları'nda.”.
    pub fn words(&self, held: bool) -> String {
        let mut zoning: Vec<String> = Vec::new();
        if let Some(v) = self.far {
            zoning.push(format!("emsal {}", n2(v)));
        }
        if let Some(v) = self.coverage {
            zoning.push(format!("TAKS {}", n2(v)));
        }
        let mut notes: Vec<String> = Vec::new();
        if let Some(v) = self.height {
            notes.push(format!("bina yüksekliği {} m", n2(v)));
        }
        if let Some(v) = self.storeys {
            notes.push(format!("kat adedi {v}"));
        }
        let (to_zoning, to_notes) = if held {
            ("İmar durumu'nda", "Plan notları'nda")
        } else {
            ("İmar durumu'na", "Plan notları'na")
        };
        let mut parts: Vec<String> = Vec::new();
        if !zoning.is_empty() {
            parts.push(format!("{} {to_zoning}", zoning.join(" ve ")));
        }
        if !notes.is_empty() {
            parts.push(format!("{} {to_notes}", notes.join(" ve ")));
        }
        let mut out = parts.join(", ");
        if let Some(first) = out.chars().next() {
            out.replace_range(..first.len_utf8(), &first.to_uppercase().to_string());
        }
        out.push('.');
        out
    }
}

/// Parselin plan kararları for İmar durumu (docs/adr/8072 §5; `cad.zoning.parcel` on the site plan's parcel): each
/// decision with its values as written, its plan (the source and its approval) and its notes; the parcel's area under
/// none, two decisions saying different things over the same piece, the typed emsal or TAKS the plan writes otherwise
/// (these in the warning colour), what else is said. Empty without the site plan's parcel. Read again only when the
/// drawing changed.
pub fn plan_lines(doc: &Document) -> Vec<(String, bool)> {
    kept(&PLAN, doc, plan_words).lines
}

/// The values each decision on the parcel gives to take (docs/adr/8140), with the line of [`plan_lines`] after which
/// its button stands: a decision whose plan is not drawn or written gives none (its source could not be said), nor one
/// that writes no number a field takes.
pub fn plan_takes(doc: &Document) -> Vec<(usize, PlanTake)> {
    kept(&PLAN, doc, plan_words).takes
}

/// A plan as İmar durumu says where a decision comes from: “UIP-1 Merkez uygulama imar planı (yururlukte, onay
/// 2024-05-14, Meclis kararı 2024/112)”.
fn plan_source(p: &kentos_commands::PlanSource) -> String {
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
    source
}

/// A decision's values to take (docs/adr/8140): each written as a number in its field's bounds (the emsal over 0 and at
/// most 50, the TAKS at most 1, the building height at most 1000 m, the storeys a whole number from 1 to 200), never a
/// value read otherwise; none without its plan or without such a value. The source names the plan by its id (its state
/// only when it is not in force), its approval and the decision: “UIP-1, onay 2024-05-14, karar K1”, whole in a plan
/// note's Kaynak; the plan's name and basis stay in its Kaynak line. None past a source's 200 characters.
fn plan_take(d: &kentos_commands::ParcelDecision) -> Option<PlanTake> {
    let p = d.plan.as_ref()?;
    let value = |field: &str| {
        d.values
            .iter()
            .find(|v| v.field == field)
            .and_then(|v| kentos_native_application::zoning_parcel::plan_number(&v.value))
    };
    let far = value("KAKS").filter(|v| *v > 0.0 && *v <= 50.0);
    let coverage = value("TAKS").filter(|v| *v > 0.0 && *v <= 1.0);
    let height = value("Hmax").filter(|v| *v > 0.0 && *v <= kentos_commands::PLAN_BUILDING_HEIGHT_MAX);
    let storeys = value("KatAdedi")
        .filter(|v| v.fract() == 0.0 && (1.0..=kentos_commands::PLAN_STOREYS_MAX).contains(v));
    if far.is_none() && coverage.is_none() && height.is_none() && storeys.is_none() {
        return None;
    }
    let mut source = p.id.clone();
    if let Some(st) = p.status.as_ref().filter(|st| st.as_str() != "yururlukte") {
        source.push_str(&format!(" ({st})"));
    }
    if let Some(o) = &p.approved {
        source.push_str(&format!(", onay {o}"));
    }
    source.push_str(&format!(", karar {}", d.id));
    if source.chars().count() > kentos_commands::MAX_ZONING_SOURCE || source.chars().any(char::is_control) {
        return None;
    }
    Some(PlanTake { decision: d.id.clone(), far, coverage, height, storeys, source })
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

fn plan_words(doc: &kentos_domain::Document) -> PlanRead {
    if doc.settings().arch.as_ref().and_then(|a| a.site.as_ref()).and_then(|s| s.parcel.as_ref()).is_none() {
        return PlanRead::default();
    }
    let input = kentos_commands::ZoningParcel::default();
    let a = match kentos_native_application::zoning_parcel::plan_in(doc, &input) {
        kentos_commands::CommandResult::Completed { output, .. } => output,
        _ => return PlanRead::default(),
    };
    let m2 = |v: f64| format!("{} m²", n2(v));
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut takes: Vec<(usize, PlanTake)> = Vec::new();
    for d in &a.decisions {
        let values: Vec<String> = d.values.iter().map(|v| format!("{} {}", field_words(&v.field), v.value)).collect();
        out.push((format!("{} ({}): {}", d.id, m2(d.share), values.join(" · ")), false));
        if let Some(p) = &d.plan {
            out.push((format!("Kaynak: {}", plan_source(p)), false));
        }
        for n in &d.notes {
            out.push((format!("Plan notu {}: {}", n.number, n.text), false));
        }
        if let Some(t) = plan_take(d) {
            takes.push((out.len() - 1, t));
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
    PlanRead { lines: out, takes }
}

/// The drawing's site plan (docs/adr/8032), read again only when the drawing changed.
pub fn site(doc: &Document) -> Option<kentos_domain::site::SiteReport> {
    kept(&SITE, doc, kentos_domain::Document::site_report)
}

/// The edges whose garden distance the footprint does not keep (docs/adr/8032), said: “2. kenar (Yan bahçe) 2,80 <
/// 3,00 m”.
pub fn short_edges(doc: &Document) -> Vec<String> {
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
fn report(doc: &Document) -> Option<kentos_domain::zoning::ZoningReport> {
    kept(&LAST, doc, kentos_domain::Document::zoning_report)
}

fn n2(v: f64) -> String {
    // An empty sum is −0 (IEEE): said as 0.
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.2}").replace('.', ",")
}

/// The badge's words: the ratios with their limits when `full`, else the ratios alone (the limits in its tip); and
/// whether one passes its limit.
pub fn words(doc: &Document, full: bool) -> Option<(String, bool)> {
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
pub fn summary(doc: &Document) -> Vec<(String, String, bool)> {
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
    let (words, over) = words(doc, shown.unwrap_or(false))?;
    let words = match shown {
        Some(_) => words,
        None if over => "!".to_owned(),
        None => String::new(),
    };
    let r = report(doc)?;
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
    let short = short_edges(doc);
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
