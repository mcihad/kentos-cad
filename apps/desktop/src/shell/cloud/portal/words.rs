//! What Portal dosyaları calls things (docs/adr/9001 §3, the web portal's words): a dossier's number line, the other
//! side, the place as surveyors say it, the days of a deadline, a package's title and line, a file's and a message's
//! line, and what a KentOS project file becomes. No Iced here.

use kentos_commands::{
    DossierDetail, DossierDirection, DossierFileView, DossierMessageView, DossierRound,
    DossierSide, DossierSite, DossierStatus, DossierSummary, LegalClock, PortalCatalog,
};

use super::facts::profession_label;
use super::plan::{Tone, closed};
use crate::cloud::local_time::{self, Zone};

/// “Aktarım 2026/0007, Proje düzeltme”; a draft has no number yet.
pub fn number_line(s: &DossierSummary) -> String {
    let what = match s.direction {
        DossierDirection::Application => "Başvuru",
        DossierDirection::Transfer => "Aktarım",
    };
    match &s.number {
        Some(n) => format!("{what} {n}, {}", s.kind_label),
        None => format!("{what} taslağı, {}", s.kind_label),
    }
}

/// The other side, as the reader names it: the municipality's name, or the professional with their profession.
pub fn counterpart(
    s: &DossierSummary,
    side: DossierSide,
    catalog: Option<&PortalCatalog>,
) -> String {
    match side {
        DossierSide::Municipality => {
            let p = profession_label(catalog, s.profession);
            if p.is_empty() {
                s.professional_name.clone()
            } else {
                format!("{}, {p}", s.professional_name)
            }
        }
        DossierSide::Professional => s.tenant_name.clone(),
    }
}

/// The place as surveyors say it: “Cumhuriyet Mahallesi, 412 ada 7 parsel”; the address, else the district.
pub fn site_text(site: &DossierSite) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(n) = site.neighbourhood.as_deref().filter(|n| !n.is_empty()) {
        parts.push(n.to_owned());
    }
    let parcel = [
        site.block.as_deref().map(|b| format!("{b} ada")),
        site.parcel.as_deref().map(|p| format!("{p} parsel")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    if !parcel.is_empty() {
        parts.push(parcel);
    }
    if parts.is_empty()
        && let Some(a) = site.address.as_deref().filter(|a| !a.is_empty())
    {
        parts.push(a.to_owned());
    }
    if parts.is_empty() {
        let place = [site.district.as_deref(), site.province.as_deref()]
            .into_iter()
            .flatten()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        if !place.is_empty() {
            parts.push(place);
        }
    }
    parts.join(", ")
}

/// A deadline in days: “5 gün kaldı”, “bugün son gün”, “2 gün geçti”.
pub fn days_left(n: i64) -> String {
    match n {
        1.. => format!("{n} gün kaldı"),
        0 => "bugün son gün".to_owned(),
        _ => format!("{} gün geçti", -n),
    }
}

/// A day the server writes as YYYY-MM-DD, as the lists write days: “27.11.2026”; the text as it is otherwise.
pub fn day_text(iso: &str) -> String {
    match day_parts(iso) {
        Some((y, m, d)) => format!("{d:02}.{m:02}.{y}"),
        None => iso.to_owned(),
    }
}

fn day_parts(iso: &str) -> Option<(i64, i64, i64)> {
    let mut parts = iso.get(..10)?.split('-');
    let y = parts.next()?.parse().ok()?;
    let m = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    Some((y, m, d))
}

/// Days from `today` (seconds since 1970 of any moment of the day, in the device's time) to the day `iso`.
pub fn days_until(iso: &str, today: i64) -> Option<i64> {
    let (y, m, d) = day_parts(iso)?;
    let target = local_time::seconds_of(y, m, d).div_euclid(86_400);
    Some(target - today.div_euclid(86_400))
}

/// Now in the device's time, as seconds read as if UTC (for [`days_until`]).
pub fn local_now() -> i64 {
    let now = crate::cloud::now_ms() / 1000;
    now + i64::from(Zone::system().offset_at(now))
}

/// A legal deadline as one line and its tone: past is red, three days or fewer amber.
pub fn clock_line(c: &LegalClock) -> (Tone, String) {
    let tone = if c.days_left < 0 {
        Tone::Danger
    } else if c.days_left <= 3 {
        Tone::Warning
    } else {
        Tone::Info
    };
    let line = format!(
        "{}: {}, {}.",
        c.label,
        day_text(&c.due_on),
        days_left(i64::from(c.days_left))
    );
    (tone, line)
}

/// A transfer's requested delivery day, with the days left while it is open.
pub fn due_line(s: &DossierSummary, today: i64) -> Option<String> {
    let due = s.due_on.as_deref()?;
    let left = days_until(due, today);
    Some(match left {
        Some(n) if !closed(s.status) => {
            format!("İstenen teslim: {}, {}.", day_text(due), days_left(n))
        }
        _ => format!("İstenen teslim: {}.", day_text(due)),
    })
}

/// The package being prepared, as its side calls it.
pub fn pending_title(d: &DossierDetail) -> String {
    let s = &d.summary;
    match (s.direction, d.side) {
        (DossierDirection::Application, _) if s.round > 0 => {
            format!("Hazırlanan {}. sürüm", d.pending_round)
        }
        (DossierDirection::Application, _) => "Hazırlanan başvuru paketi".to_owned(),
        (DossierDirection::Transfer, DossierSide::Municipality) => "Gönderilecek paket".to_owned(),
        (DossierDirection::Transfer, DossierSide::Professional) => {
            format!("Hazırlanan {}. teslim", d.pending_round.saturating_sub(1))
        }
    }
}

/// A sent package's title: the first application, a corrected version, the municipality's package, a delivery.
pub fn round_title(d: &DossierDetail, r: &DossierRound) -> String {
    match d.summary.direction {
        DossierDirection::Application if r.round == 1 => "İlk başvuru".to_owned(),
        DossierDirection::Application => format!("{}. sürüm (düzeltilmiş)", r.round),
        DossierDirection::Transfer if r.round == 1 => "Belediyenin gönderdiği paket".to_owned(),
        DossierDirection::Transfer => format!("{}. teslim", r.round - 1),
    }
}

/// A sent package's line: when it went and how many files it has.
pub fn round_line(r: &DossierRound) -> String {
    match &r.sent_at {
        Some(at) => format!(
            "{} gönderildi, {} dosya.",
            local_time::when(at, Zone::system()),
            r.files.len()
        ),
        None => format!("{} dosya.", r.files.len()),
    }
}

/// When there is no package yet.
pub fn no_package(d: &DossierDetail) -> &'static str {
    if d.side == DossierSide::Professional && d.summary.direction == DossierDirection::Transfer {
        "Belediye işi dosyasız gönderdi; ayrıntı açıklamada ve yazışmada."
    } else {
        "Gönderilen paketler burada sürüm sürüm görünür."
    }
}

/// Where a KentOS project file opens, and what opening it does and does not change.
pub fn import_text(d: &DossierDetail, file: &str) -> String {
    let place = if d.side == DossierSide::Municipality {
        format!("{} kurumunun projeleri", d.organization.name)
    } else {
        "kişisel çalışma alanınız".to_owned()
    };
    format!(
        "“{file}” dosyası {place} içinde yeni bir dosya projesi olarak açılır ve çizim alanına gelir. Dosyadaki \
         çizim olduğu gibi kalır; projede yaptığınız değişiklik bu dosyayı değiştirmez."
    )
}

/// Whether a file of the dossier opens as a project here: a sent KentOS project file, when the server allows it.
pub fn importable(d: &DossierDetail, f: &DossierFileView) -> bool {
    f.is_project && f.state == "final" && d.can_import
}

/// A file's line under its name: its document kind, size, who added it and the project it was taken from.
pub fn file_line(f: &DossierFileView) -> String {
    let source = f
        .source
        .as_ref()
        .map(|s| {
            format!(
                " Projeden alındı: {}, revizyon {}.",
                s.project_name, s.revision
            )
        })
        .unwrap_or_default();
    format!(
        "{}, {}. {} ekledi.{source}",
        f.doc_type_label,
        crate::cloud::words::size_text(usize::try_from(f.size).unwrap_or(usize::MAX)),
        f.added_by_name
    )
}

/// A message's kind when it is not a plain one: a deficiency list or a decision's reason.
pub fn message_kind(m: &DossierMessageView) -> Option<&'static str> {
    match m.kind.as_str() {
        "deficiency" => Some("Eksik ve yanlışlar"),
        "decision" => Some("Karar gerekçesi"),
        _ => None,
    }
}

/// A message's head: who wrote it, for which side, when.
pub fn message_head(d: &DossierDetail, m: &DossierMessageView) -> String {
    let side = match m.side {
        DossierSide::Municipality => d.organization.name.as_str(),
        DossierSide::Professional => "Meslek mensubu",
    };
    format!(
        "{}, {side}, {}",
        m.author_name,
        local_time::when(&m.at, Zone::system())
    )
}

/// Why no message can be written now.
pub fn thread_closed(d: &DossierDetail) -> &'static str {
    if d.summary.status == DossierStatus::Draft {
        "Yazışma dosya gönderilince açılır."
    } else {
        "Dosya kapandı; yazışma da kapandı."
    }
}
