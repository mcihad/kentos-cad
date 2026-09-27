//! What the catalog's project forms say and decide, apart from their drawing
//! (the web's app/cloud/formsPlan.ts; docs/adr/0028, 0039, 0112): Proje
//! bilgileri, Kopyasını oluştur and a new project in the other storage mode.
//! Which workspaces a form offers, when its button is on, what it sends and
//! the lines it writes. The server checks and normalizes every value; the
//! forms only offer them. fixtures/cloud/v1/forms.json holds every answer;
//! both platforms play it (`forms_plan_tests.rs`).

use kentos_cloud::ApiFailure;
use kentos_contracts::{MembershipView, ProjectMetadataUpdate, ProjectStorage, ProjectType};
use kentos_expression::js::text::{is_space, trim};

use super::actions::reason;
use super::words;

/// Names are at most this long, descriptions this long (the fields' limits; the server's too).
pub(crate) const NAME_MAX: usize = 200;
pub(crate) const DESCRIPTION_MAX: usize = 2000;

pub(crate) const CANCEL: &str = "Vazgeç";
pub(crate) const SAVING: &str = "Kaydediliyor…";
pub(crate) const PLACE: &str = "Çalışma alanı";
pub(crate) const NO_PLACE: &str =
    "Proje açabileceğiniz bir çalışma alanınız yok (project.create); kurum yöneticinize başvurun.";

/// The catalog fields shared by the forms (and the upload's).
pub(crate) mod fields {
    pub(crate) const TYPE: &str = "Tür";
    pub(crate) const DESCRIPTION: &str = "Açıklama";
    pub(crate) const DESCRIPTION_PLACEHOLDER: &str = "İsteğe bağlı: işin konusu, yeri, dayanağı";
    pub(crate) const TAGS: &str = "Etiketler";
    pub(crate) const TAGS_PLACEHOLDER: &str = "Virgülle ayırın: Kadıköy, 2026";
    /// The web's name for the type field (its screen reader's).
    #[cfg(test)]
    pub(crate) const TYPE_LABEL: &str = "Proje türü";
}

/// Proje bilgileri.
pub(crate) mod metadata {
    pub(crate) const TITLE: &str = "Proje bilgileri";
    pub(crate) const NAME: &str = "Proje adı";
    pub(crate) const SAVE: &str = "Kaydet";

    pub(crate) fn saved(name: &str) -> String {
        format!("“{name}” projesinin bilgileri kaydedildi.")
    }
}

/// Yeniden adlandır: the open project's window says these (actions.rs).
pub(crate) mod rename {
    pub(crate) const TITLE: &str = "Bulut projesini yeniden adlandır";
    pub(crate) const NAME: &str = "Yeni ad";
    pub(crate) const HINT: &str = "Projeye erişimi olan herkes yeni adı görür.";
    pub(crate) const SAVE: &str = "Yeniden adlandır";

    pub(crate) fn saved(name: &str) -> String {
        format!("Proje “{name}” olarak yeniden adlandırıldı.")
    }

    pub(crate) fn waiting(name: &str) -> String {
        format!("Yeni ad (“{name}”) bu cihazda bekliyor; sunucuya ulaşılınca kaydedilir.")
    }
}

/// Kopyasını oluştur.
pub(crate) mod duplicate {
    use super::count_text;

    pub(crate) const TITLE: &str = "Projenin kopyasını oluştur";
    pub(crate) const CONSEQUENCES: [&str; 3] = [
        "Katmanlar, ayarlar, stiller, açıklama, tür, etiketler ve bütün nesneler kalıcı kimlikleriyle kopyalanır.",
        "Geçmiş (komut günlüğü, olaylar), paylaşımlar ve favoriler kopyalanmaz; arşivlenmiş proje etkin bir kopya olur.",
        "Kopya sizin olur: siz paylaşana kadar yalnız size ve kurum politikasıyla kurum yöneticilerine görünür.",
    ];
    pub(crate) const NAME: &str = "Kopyanın adı";
    pub(crate) const MAKE: &str = "Kopyasını oluştur";
    pub(crate) const RUNNING: &str = "Kopyalanıyor…";
    /// The web's names for the fields (its screen reader's).
    #[cfg(test)]
    pub(crate) const NAME_LABEL: &str = "Kopyanın adı";
    #[cfg(test)]
    pub(crate) const PLACE_LABEL: &str = "Kopyanın çalışma alanı";

    pub(crate) fn lead(name: &str) -> String {
        format!("“{name}” yeni bir proje olarak kopyalanır.")
    }

    pub(crate) fn done(name: &str, objects: &str) -> String {
        format!(
            "“{name}” oluşturuldu: {} nesne kopyalandı.",
            count_text(objects)
        )
    }
}

/// A new project in the other storage mode.
pub(crate) mod convert {
    pub(crate) const NAME: &str = "Yeni projenin adı (isteğe bağlı)";
    #[cfg(test)]
    pub(crate) const NAME_LABEL: &str = "Yeni projenin adı";
    #[cfg(test)]
    pub(crate) const PLACE_LABEL: &str = "Yeni projenin çalışma alanı";
}

/// A count the server sends as decimal text, grouped as Turkish writes it
/// (1.234.567); anything else as it came.
pub(crate) fn count_text(objects: &str) -> String {
    if objects.is_empty() || !objects.bytes().all(|b| b.is_ascii_digit()) {
        return objects.to_owned();
    }
    let mut out = String::with_capacity(objects.len() + objects.len() / 3);
    for (i, c) in objects.chars().enumerate() {
        if i > 0 && (objects.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// The name a copy is offered.
pub(crate) fn copy_name(name: &str) -> String {
    format!("{name} (kopya)")
}

/// The tag field read as the web reads it (`parseTags`): split at commas,
/// each part trimmed with its inner spaces made one, the empty ones dropped.
pub(crate) fn parse_tags(text: &str) -> Vec<String> {
    text.split(',')
        .map(|part| {
            trim(part)
                .split(is_space)
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|t| !t.is_empty())
        .collect()
}

/// A project's catalog information as a form shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Metadata {
    pub name: String,
    pub project_type: ProjectType,
    pub description: String,
    pub tags: Vec<String>,
}

/// What the metadata form changes of the version shown: only what differs
/// (the name trimmed, the tags in order).
pub(crate) fn metadata_patch(shown: &Metadata, now: &Metadata) -> ProjectMetadataUpdate {
    let name = trim(&now.name);
    ProjectMetadataUpdate {
        name: (name != shown.name).then(|| name.to_owned()),
        project_type: (now.project_type != shown.project_type).then_some(now.project_type),
        description: (now.description != shown.description).then(|| now.description.clone()),
        tags: (now.tags != shown.tags).then(|| now.tags.clone()),
    }
}

/// Whether a patch changes anything.
pub(crate) fn changes(patch: &ProjectMetadataUpdate) -> bool {
    patch.name.is_some()
        || patch.project_type.is_some()
        || patch.description.is_some()
        || patch.tags.is_some()
}

/// Kaydet is on while the name is not empty and something changed.
pub(crate) fn metadata_savable(name: &str, patch: &ProjectMetadataUpdate) -> bool {
    !trim(name).is_empty() && changes(patch)
}

/// Yeniden adlandır is on while the new name is not empty and not the name it has.
pub(crate) fn rename_savable(typed: &str, current: &str) -> bool {
    let v = trim(typed);
    !v.is_empty() && v != current
}

/// Kopyasını oluştur is on while there is a workspace for it and a name.
pub(crate) fn duplicate_savable(places: usize, name: &str) -> bool {
    places > 0 && !trim(name).is_empty()
}

/// A workspace a form offers: its tenant and its name in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Place {
    pub tenant_id: String,
    pub label: String,
}

/// The workspaces the account may open projects in (active, with a seat,
/// `project.create`), the source's first, then the rest in the account's
/// order; the list is off while it offers fewer than two.
pub(crate) fn creatable_places(memberships: &[MembershipView], first: &str) -> Vec<Place> {
    let mut places: Vec<&MembershipView> = memberships
        .iter()
        .filter(|m| m.active && m.seat && m.capabilities.iter().any(|c| c == "project.create"))
        .collect();
    // A stable sort: the source's first, the rest keep their order.
    places.sort_by_key(|m| m.tenant_id != first);
    places
        .into_iter()
        .map(|m| Place {
            tenant_id: m.tenant_id.clone(),
            label: words::workspace(m.tenant_kind, &m.tenant_name, true),
        })
        .collect()
}

/// What the form converting a project to the other storage mode says (docs/adr/0039).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConvertForm {
    pub to: ProjectStorage,
    pub title: &'static str,
    pub lead: String,
    pub consequences: Vec<String>,
    pub placeholder: String,
    pub running: &'static str,
}

/// “PostGIS'e aktar” (a file project) or “Dosya projesine çevir” (a database
/// project). `open_dirty`: this project is the one open here, with unsaved
/// changes (they do not go to the new database project).
pub(crate) fn convert_form(name: &str, storage: ProjectStorage, open_dirty: bool) -> ConvertForm {
    let db = storage == ProjectStorage::File;
    let mut consequences = vec![
        if db {
            "Projenin en yeni revizyonu aktarılır: her nesne kalıcı kimliğiyle; ayarlar, katmanlar ve stiller dosyadan. Analitik CAD tanımları korunur, GIS çizgisine indirgenmez.".to_owned()
        } else {
            "Projenin şimdiki hâli (tek anlık görüntüsü) yeni dosya projesinin 1. revizyonu olur."
                .to_owned()
        },
        format!(
            "“{name}” olduğu gibi kalır: aynı çizimin iki yazılabilir sahibi olmaz, yeni proje başka bir projedir."
        ),
        "Yeni proje sizin olur; geçmiş, paylaşım ve favoriler gelmez. Açıklama, tür ve etiketler gelir."
            .to_owned(),
    ];
    if db {
        consequences.push("Sunucunun almadığı bir nesne (±1 000 000 000 sınırını aşan değer) varsa hiçbir proje oluşturulmaz ve nesne söylenir.".to_owned());
    }
    if db && open_dirty {
        consequences.push("Açık çizimdeki kaydedilmemiş değişiklikler aktarılmaz: önce Kaydet ile yeni revizyon yazın.".to_owned());
    }
    ConvertForm {
        to: if db {
            ProjectStorage::Database
        } else {
            ProjectStorage::File
        },
        title: if db {
            "PostGIS'e aktar"
        } else {
            "Dosya projesine çevir"
        },
        lead: format!(
            "“{name}” projesinden {} yeni bir proje oluşturulur.",
            if db {
                "nesne nesne veritabanında saklanan"
            } else {
                "dosya olarak (KCAD revizyonları) saklanan"
            }
        ),
        consequences,
        placeholder: format!("{name} ({})", if db { "PostGIS" } else { "dosya" }),
        running: if db {
            "Veritabanına aktarılıyor…"
        } else {
            "Dosya projesi oluşturuluyor…"
        },
    }
}

/// The log's line for a new project made in the other storage mode.
pub(crate) fn converted_line(name: &str, objects: &str, to: ProjectStorage) -> String {
    format!(
        "“{name}” oluşturuldu: {} nesne{}.",
        count_text(objects),
        if to == ProjectStorage::Database {
            " veritabanına aktarıldı"
        } else {
            ", 1. revizyon"
        }
    )
}

/// A failed request as a sentence: the server's words, or what to do when it gave none.
pub(crate) fn failure_reason(e: &ApiFailure) -> String {
    reason(e)
}

/// Why a conversion was refused: an object the server did not take is named
/// by its place in the file (`entities[i]` in the error's path).
pub(crate) fn convert_reason(e: &ApiFailure) -> String {
    let index = e
        .path
        .as_deref()
        .and_then(|p| p.strip_prefix("entities["))
        .and_then(|rest| rest.split(']').next())
        .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|n| n.parse::<u64>().ok());
    match index {
        Some(i) => format!(
            "{} (dosyanın {}. nesnesi; hiçbir proje oluşturulmadı).",
            reason(e),
            i + 1
        ),
        None => reason(e),
    }
}
