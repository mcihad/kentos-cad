//! The cloud interface's words, as the web says them (apps/web/src/app/cloud/
//! catalog.ts, sharing.ts, session.ts; ui/cloud/ConflictDialog.ts): roles,
//! workspaces, lists, how a project is kept, where a save stands, why a
//! conflict happened, and a server's time as “3 saat önce”.

use std::time::{SystemTime, UNIX_EPOCH};

use kentos_contracts::{
    CatalogSort, CatalogView, ConflictReason, ProjectRole, ProjectStorage, TenantKind,
};

/// A role as the interface names it.
pub fn role(role: ProjectRole) -> &'static str {
    match role {
        ProjectRole::Owner => "Sahip",
        ProjectRole::Manager => "Yönetici",
        ProjectRole::Editor => "Düzenleyici",
        ProjectRole::Commenter => "Yorumcu",
        ProjectRole::Viewer => "Görüntüleyici",
    }
}

/// How the interface names a workspace: an organisation by its name, one's
/// own personal space “Kişisel”, someone else's (a project shared from it)
/// by its person (the web's `workspaceName`).
pub fn workspace(kind: TenantKind, name: &str, own: bool) -> String {
    match kind {
        TenantKind::Organization => name.to_owned(),
        TenantKind::Personal if own => "Kişisel".to_owned(),
        TenantKind::Personal => format!("{name} (kişisel alan)"),
    }
}

/// The badge of how a project is kept.
pub fn storage_badge(storage: ProjectStorage) -> &'static str {
    match storage {
        ProjectStorage::File => "Dosya",
        ProjectStorage::Database => "PostGIS",
    }
}

/// How a project is kept, in a sentence (the web's `STORAGE_TEXT`).
pub fn storage_title(storage: ProjectStorage) -> &'static str {
    match storage {
        ProjectStorage::File => "Dosya (KCAD revizyonları)",
        ProjectStorage::Database => "Yönetilen PostGIS veritabanı",
    }
}

/// What a storage mode means (the web's `STORAGE_TEXT` detail).
pub fn storage_detail(storage: ProjectStorage) -> &'static str {
    match storage {
        ProjectStorage::File => {
            "Proje sunucuda değişmez .kcad revizyonları olarak saklanır; her kayıt yeni bir revizyondur ve dayandığı revizyonla karşılaştırılır, arada başkası kaydettiyse üzerine yazılmaz. Paylaşım alıcıya dosya deposuna ayrı bir erişim vermez."
        }
        ProjectStorage::Database => {
            "Nesneler sunucudaki veritabanında tek tek saklanır; her kayıt tek işlemde yazılır ve erişimi olan herkes hemen görür. Paylaşım alıcıya veritabanı hesabı ya da parolası vermez."
        }
    }
}

/// A list of the catalog: its name, its orders (the first is its own) and
/// what it says empty (the web's `VIEWS`).
pub struct ViewText {
    pub label: &'static str,
    pub sorts: &'static [CatalogSort],
    pub empty: &'static str,
    pub note: &'static str,
}

/// The lists, in the web's order.
pub const VIEWS: [CatalogView; 7] = [
    CatalogView::Recent,
    CatalogView::Favorites,
    CatalogView::Mine,
    CatalogView::Organization,
    CatalogView::Shared,
    CatalogView::Archived,
    CatalogView::Trash,
];

const BY_CHANGE: &[CatalogSort] = &[
    CatalogSort::Updated,
    CatalogSort::Name,
    CatalogSort::Created,
];

pub fn view(view: CatalogView) -> ViewText {
    match view {
        CatalogView::Recent => ViewText {
            label: "Son kullanılanlar",
            sorts: &[CatalogSort::Opened, CatalogSort::Updated, CatalogSort::Name],
            empty: "Henüz açtığınız bir bulut projesi yok. Açtığınız ve oluşturduğunuz projeler burada, en yenisi üstte durur.",
            note: "",
        },
        CatalogView::Favorites => ViewText {
            label: "Favoriler",
            sorts: BY_CHANGE,
            empty: "Favori projeniz yok. Bir projeyi yıldızına tıklayarak buraya ekleyin; favorileriniz yalnız size görünür.",
            note: "Favorileriniz yalnız size görünür.",
        },
        CatalogView::Mine => ViewText {
            label: "Projelerim",
            sorts: BY_CHANGE,
            empty: "Sahibi olduğunuz bir proje yok. Açık çizimi Dosya → Buluta yükle ile gönderebilirsiniz.",
            note: "",
        },
        CatalogView::Organization => ViewText {
            label: "Kurum projeleri",
            sorts: BY_CHANGE,
            empty: "Bu kurumda size açık bir proje yok: sizin açtıklarınız ve sizinle paylaşılanlar burada görünür.",
            note: "",
        },
        CatalogView::Shared => ViewText {
            label: "Benimle paylaşılanlar",
            sorts: BY_CHANGE,
            empty: "Sizinle paylaşılmış bir proje yok. Biri bir projeyi sizinle paylaşınca burada, sahibinin adı ve rolünüzle görünür.",
            note: "Başkalarının sizinle paylaştığı projeler; sahibi ve rolünüz yanında yazar.",
        },
        CatalogView::Archived => ViewText {
            label: "Arşivlenmişler",
            sorts: BY_CHANGE,
            empty: "Arşivlenmiş bir proje yok. Arşivlenen proje salt okunur olur ve burada durur.",
            note: "Arşivlenmiş projeler salt okunurdur: açılır, kopyalanır; arşivden çıkarmak proje sahibinin ya da yöneticisinindir.",
        },
        // Its note says how long the trash keeps a project (plan.rs `trash_note`).
        CatalogView::Trash => ViewText {
            label: "Çöp kutusu",
            sorts: &[CatalogSort::Trashed, CatalogSort::Name],
            empty: "Çöp kutusu boş. Geri yükleyebileceğiniz (sahibi ya da kurum yöneticisi olduğunuz) silinmiş projeler burada durur.",
            note: "",
        },
    }
}

/// Why the server no longer takes this account's work on a kept project (the web's words).
pub fn ended(ended: kentos_cloud::replica::Ended) -> &'static str {
    use kentos_cloud::replica::Ended;
    match ended {
        Ended::Deleted => "Çöp kutusunda",
        Ended::Revoked => "Erişiminiz kaldırıldı",
        Ended::Archived => "Arşivlendi",
    }
}

/// A time in milliseconds since 1970 (a copy's), said from now.
pub fn ago_ms(ms: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    let then = i64::try_from(ms / 1000).unwrap_or(i64::MAX);
    match now - then {
        ..60 => "az önce".to_owned(),
        60..3600 => format!("{} dakika önce", (now - then) / 60),
        3600..86_400 => format!("{} saat önce", (now - then) / 3600),
        86_400..604_800 => format!("{} gün önce", (now - then) / 86_400),
        _ => date(then),
    }
}

/// A decimal count as tr-TR writes it (`toLocaleString`): 1284 → “1.284”;
/// anything else as it is.
pub fn grouped(count: &str) -> String {
    if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
        return count.to_owned();
    }
    let mut out = String::new();
    for (i, d) in count.chars().enumerate() {
        if i > 0 && (count.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(d);
    }
    out
}

/// A size as the web writes it (`sizeText`, tr-TR): “512 bayt”, “1,5 KB”, “12,3 MB”.
pub fn size_text(bytes: usize) -> String {
    // tr-TR: “.” groups thousands, “,” is the decimal mark; one decimal at most, half away from zero.
    fn number(v: f64) -> String {
        let tenths = (v * 10.0).round() as u64;
        let (whole, tenth) = (tenths / 10, tenths % 10);
        let digits = whole.to_string();
        let mut grouped = String::new();
        for (i, d) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                grouped.push('.');
            }
            grouped.push(d);
        }
        if tenth == 0 {
            grouped
        } else {
            format!("{grouped},{tenth}")
        }
    }
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{} bayt", number(b))
    } else if bytes < 1024 * 1024 {
        format!("{} KB", number(b / 1024.0))
    } else {
        format!("{} MB", number(b / (1024.0 * 1024.0)))
    }
}

/// Why an object could not be saved (the web's ConflictDialog.ts `REASON`).
pub fn reason(reason: ConflictReason) -> &'static str {
    match reason {
        ConflictReason::Changed => "başkası değiştirdi",
        ConflictReason::Deleted => "başkası sildi",
        // The same object (the same persistent id) is on the server already:
        // someone else brought it back first.
        ConflictReason::Exists => "sunucuda zaten var",
        ConflictReason::Project => "proje bilgileri değişti",
    }
}

/// An object kind as a row names it (the web's conflict dialog).
pub fn kind(kind: &str) -> &'static str {
    match kind {
        "point" => "Nokta",
        "line" => "Çizgi",
        "polyline" => "Çoklu çizgi",
        "polygon" => "Alan",
        "circle" => "Daire",
        "arc" => "Yay",
        "ellipse" => "Elips",
        "spline" => "Eğri",
        "xline" => "Yardımcı çizgi",
        "ray" => "Işın",
        "text" => "Yazı",
        "dimension" => "Ölçü",
        "hatch" => "Tarama",
        _ => "Nesne",
    }
}

/// Seconds since 1970 of an RFC 3339 time (`2026-09-26T09:41:08.123Z`,
/// `…+03:00`); none for anything else.
pub fn epoch(text: &str) -> Option<i64> {
    let b = text.trim().as_bytes();
    let num = |at: usize, len: usize| -> Option<i64> {
        let part = b.get(at..at + len)?;
        part.iter()
            .all(u8::is_ascii_digit)
            .then(|| part.iter().fold(0i64, |n, d| n * 10 + i64::from(d - b'0')))
    };
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b'T' | b't' | b' ') {
        return None;
    }
    let (year, month, day) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (hour, minute, second) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    // Past the seconds: a fraction, then Z or an offset.
    let mut at = 19;
    if b.get(at) == Some(&b'.') {
        at += 1;
        while b.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
    }
    let offset = match b.get(at)? {
        b'Z' | b'z' if at + 1 == b.len() => 0,
        sign @ (b'+' | b'-') if at + 6 == b.len() && b[at + 3] == b':' => {
            let minutes = num(at + 1, 2)? * 60 + num(at + 4, 2)?;
            if *sign == b'+' {
                minutes * 60
            } else {
                -minutes * 60
            }
        }
        _ => return None,
    };
    // Days from the civil date (Howard Hinnant's algorithm).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

/// A server's time as the lists say it, from now: “az önce”, “5 dakika önce”,
/// “3 saat önce”, “2 gün önce”, then its date (26.09.2026).
pub fn ago(text: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    ago_from(text, now)
}

pub fn ago_from(text: &str, now: i64) -> String {
    let Some(then) = epoch(text) else {
        return String::new();
    };
    match now - then {
        ..60 => "az önce".to_owned(),
        60..3600 => format!("{} dakika önce", (now - then) / 60),
        3600..86_400 => format!("{} saat önce", (now - then) / 3600),
        86_400..604_800 => format!("{} gün önce", (now - then) / 86_400),
        _ => date(then),
    }
}

/// A server's time as a date in the device's time (the web's `day`:
/// 26.09.2026); none for anything else.
pub fn day(text: &str) -> Option<String> {
    super::local_time::day(text, super::local_time::Zone::system())
}

/// The date of a time in the device's time, as the lists write it (26.09.2026).
fn date(seconds: i64) -> String {
    super::local_time::date_of(seconds, super::local_time::Zone::system())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_times_read_with_their_offset() {
        assert_eq!(epoch("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(epoch("2026-09-26T09:41:08.123456Z"), Some(1_790_415_668));
        // The same moment written in Turkey's offset.
        assert_eq!(epoch("2026-09-26T12:41:08+03:00"), Some(1_790_415_668));
        assert_eq!(epoch("2000-02-29T23:59:59-01:30"), Some(951_874_199));
        for bad in [
            "",
            "dün",
            "2026-09-26",
            "2026-13-01T00:00:00Z",
            "2026-09-26T09:41:08",
            "2026-09-26T09:41:08Zx",
        ] {
            assert_eq!(epoch(bad), None, "{bad}");
        }
    }

    #[test]
    fn times_are_said_from_now() {
        let now = epoch("2026-09-26T12:00:00Z").unwrap();
        let say = |t: &str| ago_from(t, now);
        assert_eq!(say("2026-09-26T11:59:30Z"), "az önce");
        assert_eq!(say("2026-09-26T11:55:00Z"), "5 dakika önce");
        assert_eq!(say("2026-09-26T09:00:00Z"), "3 saat önce");
        assert_eq!(say("2026-09-24T12:00:00Z"), "2 gün önce");
        assert_eq!(say("2026-08-01T08:00:00Z"), "01.08.2026");
        assert_eq!(say("bozuk"), "");
    }

    #[test]
    fn counts_are_grouped_as_the_web_groups_them() {
        assert_eq!(grouped("1284"), "1.284");
        assert_eq!(grouped("12"), "12");
        assert_eq!(grouped("1234567"), "1.234.567");
        assert_eq!(grouped("12a"), "12a");
    }

    #[test]
    fn sizes_are_written_as_on_the_web() {
        assert_eq!(size_text(0), "0 bayt");
        assert_eq!(size_text(1000), "1.000 bayt");
        assert_eq!(size_text(1536), "1,5 KB");
        assert_eq!(size_text(2048), "2 KB");
        assert_eq!(size_text(5 * 1024 * 1024 + 300 * 1024), "5,3 MB");
        assert_eq!(size_text(1500 * 1024 * 1024), "1.500 MB");
    }

    #[test]
    fn workspaces_are_named_as_on_the_web() {
        assert_eq!(
            workspace(TenantKind::Organization, "Harita Bürosu", true),
            "Harita Bürosu"
        );
        assert_eq!(
            workspace(TenantKind::Personal, "Ayşe Yılmaz", true),
            "Kişisel"
        );
        assert_eq!(
            workspace(TenantKind::Personal, "Ayşe Yılmaz", false),
            "Ayşe Yılmaz (kişisel alan)"
        );
    }
}
