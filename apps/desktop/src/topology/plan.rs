//! The Topoloji tab's rules apart from the view (docs/adr/0202 §5; the
//! web's ui/bottom/topologyPlan.ts, word for word): its words, the rows'
//! cells, which rows a filter shows, the count line, the box a row zooms to
//! and how Düzelt ▾ names a fix. The trace `topology-rules.json` reads the
//! rows on both platforms.

use std::fmt;

use kentos_contracts::TopologyRule;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::topology_rules::{Finding, Kind, MeasureKind};
use kentos_interaction::Format;

/// The tab's words (the web's `TOPOLOGY_TEXTS`).
pub mod texts {
    pub const CHECK: &str = "Denetle";
    pub const CHECK_HINT: &str = "Projenin topoloji kurallarını çizimde denetler; bulgular tabloda, satıra tıklamak bulguya gider.";
    pub const RULES: &str = "Kurallar…";
    pub const RULES_HINT: &str =
        "Topoloji kurallarını, toleransı ve istisnaları düzenler (proje ayarı).";
    pub const RULE_PICK: &str = "Kural";
    pub const ALL_RULES: &str = "Bütün kurallar";
    pub const OPEN: &str = "Açık";
    pub const EXCEPTIONS: &str = "İstisna";
    pub const ALL: &str = "Hepsi";
    pub const FIX: &str = "Düzelt";
    pub const FIX_HINT: &str =
        "Seçili bulguyu düzeltir: çizim tek geri alma adımında değişir, denetim yinelenir.";
    pub const FIX_NONE: &str = "Bu bulgunun düzeltmesi yok.";
    pub const MARK: &str = "İstisna yap";
    pub const MARK_HINT: &str =
        "Seçili bulguları bilerek bırakılmış sayar; projede saklanır, İstisna süzgecinde görünür.";
    pub const UNMARK: &str = "İstisnayı kaldır";
    pub const UNMARK_HINT: &str = "Seçili bulguların istisnasını kaldırır.";
    pub const NO_RULES: &str =
        "Projede topoloji kuralı yok. Kurallar… ile katmanlara kural ekleyin.";
    pub const STALE: &str =
        "Çizim son denetimden sonra değişti; sonuçlar eski olabilir. Denetle ile yenileyin.";
    pub const NONE_SHOWN: &str = "Bu süzgeçte bulgu yok.";
}

/// Before a check: how many rules a check runs.
pub fn not_checked(rules: usize) -> String {
    format!("Denetle'ye basın: {rules} kural denetlenecek.")
}

/// After a check without findings: how many rules and objects it looked at.
pub fn no_findings(rules: usize, objects: usize) -> String {
    format!("Bulgu yok: {rules} kural, {objects} nesne denetlendi.")
}

/// Rules whose layer (or other layer) the project no longer has.
pub fn missing_layers(n: usize) -> String {
    format!("{n} kuralın katmanı projede yok; denetlenmedi.")
}

/// The table's columns.
pub const COLUMNS: [&str; 6] = ["Sıra", "Katman", "Kural", "Sorun", "Nesneler", "Ölçü"];

/// What the rows show: the findings left open, those marked as exceptions, or all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    Open,
    Exception,
    All,
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Filter::Open => texts::OPEN,
            Filter::Exception => texts::EXCEPTIONS,
            Filter::All => texts::ALL,
        })
    }
}

/// A rule as a sentence: its kind's name, the other layer's name for “…”.
pub fn rule_text(rule: &TopologyRule, layer_name: impl Fn(&str) -> String) -> String {
    match Kind::of_key(rule.kind.key()) {
        Some(kind) => kind.named(rule.other.as_deref().map(&layer_name).as_deref()),
        None => rule.kind.key().to_owned(),
    }
}

/// The objects a finding names, as the drawing shows their ids.
pub fn objects_text(f: &Finding, ids: &[u32]) -> String {
    f.objects
        .iter()
        .map(|&k| format!("#{}", ids.get(k).copied().unwrap_or_default()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A finding's measure in the project's units; empty when it has none or it
/// is a distance of nothing (a T junction).
pub fn measure_text(f: &Finding, format: &Format) -> String {
    let Some(m) = f.measure else {
        return String::new();
    };
    match f.measure_kind {
        Some(MeasureKind::Distance) if m == 0.0 => String::new(),
        Some(MeasureKind::Area) => format.area(m),
        Some(MeasureKind::Angle) => format.angle(m),
        _ => format.length(m),
    }
}

/// The findings a filter and a rule (its place in the rules; none: every
/// rule) show, by their places.
pub fn shown_findings(findings: &[Finding], filter: Filter, rule: Option<usize>) -> Vec<usize> {
    findings
        .iter()
        .enumerate()
        .filter(|(_, f)| rule.is_none_or(|r| f.rule == r))
        .filter(|(_, f)| match filter {
            Filter::Open => !f.exception,
            Filter::Exception => f.exception,
            Filter::All => true,
        })
        .map(|(i, _)| i)
        .collect()
}

/// The count line: the rows shown, then how many are open and how many exceptions.
pub fn count_text(shown: usize, open: usize, exceptions: usize) -> String {
    format!("{shown} bulgu (açık {open}, istisna {exceptions})")
}

/// The least side of the box a row zooms to (m): a point's problem shows its neighbourhood.
pub const FINDING_VIEW: f64 = 10.0;

/// The box a row zooms to: the finding's, at least [`FINDING_VIEW`] on each
/// side, with a fifth of it again on each side, about its middle.
pub fn finding_view(b: &Bounds) -> Bounds {
    let (cx, cy) = ((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
    let w = (b.max_x - b.min_x).max(FINDING_VIEW) * 0.7;
    let h = (b.max_y - b.min_y).max(FINDING_VIEW) * 0.7;
    Bounds {
        min_x: cx - w,
        min_y: cy - h,
        max_x: cx + w,
        max_y: cy + h,
    }
}

/// A fix as Düzelt ▾ lists it: its name and what it acts on (the object an
/// area is subtracted from or merged into, the one a vertex is added to or
/// deleted) or how far an end or a point moves.
pub fn fix_label(f: &Finding, key: &str, ids: &[u32], length: impl Fn(f64) -> String) -> String {
    let label = kentos_geometry_core::ops::topology_rules::fix_label(key);
    let id = |k: Option<usize>| {
        k.and_then(|k| ids.get(k))
            .map_or_else(String::new, |id| format!(" (#{id})"))
    };
    match key {
        "subtractFirst" | "addVertex" => format!("{label}{}", id(f.objects.first().copied())),
        "subtractSecond" => format!("{label}{}", id(f.objects.get(1).copied())),
        "mergeNeighbour" | "deleteDuplicate" => format!("{label}{}", id(f.subject)),
        "snapEnd" | "snapToEnd" => match f.measure {
            Some(m) => format!("{label} ({})", length(m)),
            None => label.to_owned(),
        },
        _ => label.to_owned(),
    }
}
