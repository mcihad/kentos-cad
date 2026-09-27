//! What the catalog shows and offers for a project, decided apart from
//! drawing it (the web's app/cloud/catalogPlan.ts, docs/adr/0028): the texts
//! of a list's row, the selected project's pane (favourite toggle, chips,
//! tabs, fact rows, actions with the reason each one is off), the window's
//! main button, and the questions and lines of the lifecycle's actions.
//! Pure: the window draws it, and fixtures/cloud/v1/catalog.json pins it for
//! both platforms. Dates are the device's local time (local_time.rs).

use kentos_contracts::{
    CatalogSort, CatalogView, ProjectPermission, ProjectState, ProjectStorage, ProjectSummary,
    ProjectType,
};

use super::local_time::{Zone, day, when};
use super::words;

/// Why an archived project cannot be changed.
pub const ARCHIVED_TEXT: &str = "Arşivlenmiş proje değiştirilemez; önce arşivden çıkarın.";

/// What an empty list says when a search or the type filter is on.
pub const EMPTY_SEARCH: &str =
    "Aramanıza uyan proje yok. Başka sözcüklerle ya da tür süzgeci olmadan deneyin.";

/// What “Kurum projeleri” says without an organisation.
pub const NO_ORGANIZATION: &str =
    "Etkin üyeliğiniz olan bir kurum yok; kurum projeleri burada görünür.";

/// What choosing a type means, and does not (said wherever one is chosen:
/// Proje bilgileri, catalog_forms_view.rs).
pub const TYPE_HINT: &str = "Tür yalnız projeleri bulmak ve düzenlemek içindir: bir modül açmaz, mevzuata uygunluk ya da resmî onay anlamına gelmez, projenin nasıl saklandığını değiştirmez.";

/// The types a project can have, in the web's order (`PROJECT_TYPES`).
pub const PROJECT_TYPES: [ProjectType; 7] = [
    ProjectType::Cad,
    ProjectType::Gis,
    ProjectType::LandReadjustment,
    ProjectType::ZoningPlan,
    ProjectType::Subdivision,
    ProjectType::Road,
    ProjectType::Architecture,
];

/// A project type as the interface names it (the web's `TYPE_LABEL`).
pub fn type_label(t: ProjectType) -> &'static str {
    match t {
        ProjectType::Cad => "Genel CAD",
        ProjectType::Gis => "CBS",
        ProjectType::LandReadjustment => "18 uygulaması",
        ProjectType::ZoningPlan => "İmar planı",
        ProjectType::Subdivision => "İfraz / tevhit",
        ProjectType::Road => "Yol",
        ProjectType::Architecture => "Mimari",
    }
}

/// A project's state (the web's `STATE_LABEL`).
pub fn state_label(s: ProjectState) -> &'static str {
    match s {
        ProjectState::Active => "Etkin",
        ProjectState::Archived => "Arşivde",
        ProjectState::Trashed => "Çöp kutusunda",
    }
}

/// An order of a list (the web's `SORT_LABEL`).
pub fn sort_label(s: CatalogSort) -> &'static str {
    match s {
        CatalogSort::Updated => "Son değişiklik",
        CatalogSort::Name => "Ad",
        CatalogSort::Created => "Oluşturulma",
        CatalogSort::Opened => "Son açılma",
        CatalogSort::Trashed => "Çöpe taşınma",
    }
}

/// What the trash's list says above it: how long it keeps a project.
pub fn trash_note(days: u32) -> String {
    let days = if days == 0 { 30 } else { days };
    format!(
        "Çöp kutusundaki projeler, taşındıktan {days} gün sonra kalıcı olarak silinir; o zamana kadar proje sahibi ya da kurum yöneticisi geri yükleyebilir."
    )
}

/// Why the account may not do something to a project: the right it needs.
pub fn denied_text(name: &str, what: &str, permission: ProjectPermission) -> String {
    format!(
        "“{name}” projesinde {what} yetkiniz yok ({}); proje sahibine ya da yöneticisine başvurun.",
        permission.name()
    )
}

/// An action of the selected project's pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailAction {
    Share,
    Edit,
    Download,
    Duplicate,
    Convert,
    Archive,
    Unarchive,
    Trash,
    Purge,
}

impl DetailAction {
    /// Its id, as the web's plan names it.
    #[cfg(test)]
    pub fn id(self) -> &'static str {
        match self {
            Self::Share => "share",
            Self::Edit => "edit",
            Self::Download => "download",
            Self::Duplicate => "duplicate",
            Self::Convert => "convert",
            Self::Archive => "archive",
            Self::Unarchive => "unarchive",
            Self::Trash => "trash",
            Self::Purge => "purge",
        }
    }
}

/// One action: what it says, its icon (the web's name), why it is off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionPlan {
    pub id: DetailAction,
    pub label: &'static str,
    pub icon: &'static str,
    /// Why it is off; none when it can be taken.
    pub why: Option<String>,
    /// A removing action: marked, not amber.
    pub danger: bool,
}

/// The selected project's pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailPlan {
    /// The favourite toggle's label; none in the trash.
    pub favorite: Option<&'static str>,
    pub chips: Vec<&'static str>,
    /// The pane's tabs; none in the trash (nothing to show a history of).
    pub tabs: Option<[&'static str; 2]>,
    /// The fact rows shown, in order (their values are drawn by the pane).
    pub facts: Vec<&'static str>,
    pub actions: Vec<ActionPlan>,
}

fn may(p: &ProjectSummary, permission: ProjectPermission) -> bool {
    p.access.permissions.contains(&permission)
}

/// The selected project's pane: `open` when it is the project open here.
pub fn detail_plan(p: &ProjectSummary, open: bool) -> DetailPlan {
    let trashed = p.state == ProjectState::Trashed;
    let needs = |permission: ProjectPermission, what: &str| {
        (!may(p, permission)).then(|| denied_text(&p.name, what, permission))
    };
    let writable = |permission: ProjectPermission, what: &str| {
        if p.state == ProjectState::Archived {
            Some(ARCHIVED_TEXT.to_owned())
        } else {
            needs(permission, what)
        }
    };
    let action = |id, label, icon, why, danger| ActionPlan {
        id,
        label,
        icon,
        why,
        danger,
    };
    let to_database = p.storage == ProjectStorage::File;
    use DetailAction as A;
    use ProjectPermission as P;
    let actions = if trashed {
        vec![action(
            A::Purge,
            "Kalıcı olarak sil…",
            "trash",
            needs(P::Delete, "kalıcı olarak silme"),
            true,
        )]
    } else {
        vec![
            action(
                A::Share,
                "Paylaş…",
                "share",
                needs(P::Share, "paylaşma"),
                false,
            ),
            action(
                A::Edit,
                "Bilgileri düzenle…",
                "edit",
                writable(P::Edit, "bilgileri değiştirme"),
                false,
            ),
            action(
                A::Download,
                ".kcad olarak indir",
                "export",
                needs(P::Download, "indirme"),
                false,
            ),
            action(
                A::Duplicate,
                "Kopyasını oluştur…",
                "copy",
                needs(P::Download, "kopyalama"),
                false,
            ),
            if to_database {
                action(
                    A::Convert,
                    "PostGIS'e aktar…",
                    "server",
                    needs(P::Download, "dönüştürme"),
                    false,
                )
            } else {
                action(
                    A::Convert,
                    "Dosya projesine çevir…",
                    "save",
                    needs(P::Download, "dönüştürme"),
                    false,
                )
            },
            if p.state == ProjectState::Archived {
                action(
                    A::Unarchive,
                    "Arşivden çıkar",
                    "archive",
                    needs(P::Edit, "arşivden çıkarma"),
                    false,
                )
            } else {
                action(
                    A::Archive,
                    "Arşivle…",
                    "archive",
                    needs(P::Edit, "arşivleme"),
                    false,
                )
            },
            action(
                A::Trash,
                "Çöpe taşı…",
                "trash",
                needs(P::Delete, "çöpe taşıma"),
                true,
            ),
        ]
    };
    let mut facts = vec![
        "Çalışma alanı",
        "Sahibi",
        "Rolünüz",
        "Koordinat sistemi",
        "Alan birimi",
    ];
    // Nothing of a project in the trash is counted (it does not open).
    if !trashed {
        facts.extend(["Nesne", "Kapsam"]);
    }
    facts.extend(["Oluşturan", "Son değişiklik", "Revizyon", "Saklama"]);
    if p.archived_at.is_some() {
        facts.push("Arşivlenme");
    }
    if p.trashed_at.is_some() {
        facts.push("Çöpe taşınma");
    }
    if trashed {
        facts.push("Kalıcı silinme");
    }
    let mut chips = vec![type_label(p.project_type)];
    if p.state != ProjectState::Active {
        chips.push(state_label(p.state));
    }
    if open {
        chips.push("Şu anda açık");
    }
    DetailPlan {
        favorite: (!trashed).then_some(if p.favorite {
            "Favorilerde"
        } else {
            "Favorilere ekle"
        }),
        chips,
        tabs: (!trashed).then_some(["Bilgiler", "Geçmiş"]),
        facts,
        actions,
    }
}

/// The window's one amber button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimaryPlan {
    pub label: &'static str,
    pub enabled: bool,
    /// The button's tip: why it is off, or what it does to this project.
    pub why: String,
}

/// The window's main button for the list `view` and the selected project.
pub fn primary_plan(view: CatalogView, p: Option<&ProjectSummary>) -> PrimaryPlan {
    const NONE: &str = "Önce listeden bir proje seçin.";
    if view == CatalogView::Trash {
        let ok = p.is_some_and(|p| may(p, ProjectPermission::Delete));
        let why = match p {
            None => NONE.to_owned(),
            Some(_) if ok => String::new(),
            Some(p) => format!(
                "“{}” projesini geri yükleme yetkiniz yok (project.delete).",
                p.name
            ),
        };
        return PrimaryPlan {
            label: "Geri yükle",
            enabled: ok,
            why,
        };
    }
    let why = match p {
        None => NONE,
        Some(p) if p.state == ProjectState::Archived => "Arşivlenmiş proje salt okunur açılır.",
        Some(_) => "",
    };
    PrimaryPlan {
        label: "Aç",
        enabled: p.is_some(),
        why: why.to_owned(),
    }
}

/// One project of a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowPlan {
    /// After the name: the favourite star's tip, “Açık”, “Arşivde”.
    pub marks: Vec<&'static str>,
    /// Under the name: the type and where (or whose) it is; in the trash, who moved it there.
    pub sub: Vec<String>,
    /// On the right: the role (shared list), when it goes for good (trash), the time the list is ordered by.
    pub side: Vec<String>,
}

/// The time a row shows: the one its list is ordered by.
fn time_of(p: &ProjectSummary, sort: CatalogSort, zone: &Zone) -> String {
    match sort {
        CatalogSort::Opened => p
            .opened_at
            .as_deref()
            .map_or_else(String::new, |t| when(t, zone)),
        CatalogSort::Created => when(&p.created_at, zone),
        CatalogSort::Trashed => p
            .trashed_at
            .as_deref()
            .and_then(|t| day(t, zone))
            .map_or_else(String::new, |d| format!("Silinme: {d}")),
        CatalogSort::Updated | CatalogSort::Name => when(&p.updated_at, zone),
    }
}

/// One project in the list `view` ordered by `sort`; `place` is where it is
/// as the account names it.
pub fn row_plan(
    p: &ProjectSummary,
    view: CatalogView,
    sort: CatalogSort,
    open: bool,
    place: &str,
    zone: &Zone,
) -> RowPlan {
    let mut marks = Vec::new();
    if p.favorite {
        marks.push("Favorilerinizde");
    }
    if open {
        marks.push("Açık");
    }
    if p.state == ProjectState::Archived && view != CatalogView::Archived {
        marks.push(state_label(ProjectState::Archived));
    }
    let sub = if view == CatalogView::Trash {
        vec![
            p.trashed_by_name
                .as_deref()
                .filter(|n| !n.is_empty())
                .map_or_else(|| place.to_owned(), |n| format!("Çöpe taşıyan: {n}")),
        ]
    } else {
        let whose = if matches!(view, CatalogView::Organization | CatalogView::Shared) {
            let owner = if p.owner_name.is_empty() {
                "görünmüyor"
            } else {
                p.owner_name.as_str()
            };
            format!("Sahibi: {owner}")
        } else {
            place.to_owned()
        };
        vec![type_label(p.project_type).to_owned(), whose]
    };
    let mut side = Vec::new();
    if view == CatalogView::Shared {
        side.push(words::role(p.access.role).to_owned());
    }
    if view == CatalogView::Trash
        && let Some(d) = p.purge_after.as_deref().and_then(|t| day(t, zone))
    {
        side.push(format!("{d} tarihinde silinir"));
    }
    side.push(time_of(p, sort, zone));
    RowPlan { marks, sub, side }
}

/// A lifecycle question: its title, question, what it means, and the removing answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub title: &'static str,
    pub message: String,
    pub details: Vec<String>,
    pub action: &'static str,
}

const STOPS_SAVING: &str = "Projeyi şu anda açık tutanların kaydı durur; gönderilmemiş değişiklikleri kendi cihazlarında kalır.";

/// Moving to the trash (`project.trash`): `days` how long the trash keeps
/// it, when known; `open` when it is open here.
pub fn trash_question(name: &str, tenant: &str, days: Option<u32>, open: bool) -> Question {
    let until = days.filter(|d| *d > 0).map_or_else(
        || "saklama süresi dolana kadar".to_owned(),
        |d| format!("{d} gün"),
    );
    let mut details = vec![
        "Proje listelerden kalkar; kimse açamaz ve değiştiremez.".to_owned(),
        STOPS_SAVING.to_owned(),
        format!(
            "Hiçbir şey silinmez: proje sahibi ya da kurum yöneticisi {until} içinde Çöp kutusu’ndan geri yükleyebilir; sonra proje kalıcı olarak silinir."
        ),
    ];
    if open {
        details.push(
            "Proje şu anda sizde açık: çizim ekranda kalır, dilerseniz yerel bir dosyaya kaydedin."
                .to_owned(),
        );
    }
    Question {
        title: "Çöp kutusuna taşı",
        message: format!(
            "“{name}” projesi ({tenant}) erişimi olan herkes için çöp kutusuna taşınsın mı?"
        ),
        details,
        action: "Çöpe taşı",
    }
}

/// Removing a project in the trash for good (`project.purge`).
pub fn purge_question(name: &str) -> Question {
    Question {
        title: "Kalıcı olarak sil",
        message: format!("“{name}” projesi kalıcı olarak silinsin mi? Bu işlem geri alınamaz."),
        details: vec![
            "Nesneler, katmanlar, paylaşımlar, komut günlüğü ve olaylar silinir; yalnız kimin ne zaman sildiğini söyleyen denetim kaydı kalır.".to_owned(),
            "Önceden indirilmiş kopyalar ve sunucu yedekleri bu işlemle silinmez.".to_owned(),
        ],
        action: "Kalıcı olarak sil",
    }
}

/// Archiving (`project.archive`): read-only for everyone until it is unarchived.
pub fn archive_question(name: &str) -> Question {
    Question {
        title: "Projeyi arşivle",
        message: format!("“{name}” projesi arşivlensin mi?"),
        details: vec![
            "Proje salt okunur olur: nesneleri, adı ve bilgileri değişmez; açılabilir, paylaşımı değiştirilebilir, kopyası oluşturulabilir.".to_owned(),
            STOPS_SAVING.to_owned(),
            "Arşivlenmişler listesinde durur; proje sahibi ya da yöneticisi arşivden çıkarabilir.".to_owned(),
        ],
        action: "Arşivle",
    }
}

/// The lines the catalog's actions write: to the log and under the list
/// (the web's `CATALOG_LINES`).
pub mod lines {
    use super::{Zone, day};

    pub fn trashed(name: &str, purge_after: Option<&str>, zone: &Zone) -> String {
        match purge_after.and_then(|t| day(t, zone)) {
            Some(d) => {
                format!("“{name}” çöp kutusuna taşındı; {d} tarihine kadar geri yüklenebilir.")
            }
            None => format!("“{name}” çöp kutusuna taşındı."),
        }
    }
    pub fn trash_failed(name: &str, why: &str) -> String {
        format!("“{name}” çöp kutusuna taşınamadı: {why}")
    }
    pub fn trashed_status(name: &str) -> String {
        format!("“{name}” çöp kutusuna taşındı.")
    }
    pub fn purged(name: &str, objects: &str) -> String {
        format!("“{name}” kalıcı olarak silindi ({objects} nesne).")
    }
    pub fn purge_failed(name: &str, why: &str) -> String {
        format!("“{name}” kalıcı olarak silinemedi: {why}")
    }
    pub fn purged_status(name: &str) -> String {
        format!("“{name}” kalıcı olarak silindi.")
    }
    pub fn archived(name: &str) -> String {
        format!("“{name}” arşivlendi.")
    }
    pub fn archive_failed(name: &str, why: &str) -> String {
        format!("“{name}” arşivlenemedi: {why}")
    }
    pub fn archived_status(name: &str) -> String {
        format!("“{name}” arşivlendi; Arşivlenmişler listesinde duruyor.")
    }
    pub fn unarchived(name: &str) -> String {
        format!("“{name}” arşivden çıkarıldı.")
    }
    pub fn unarchived_status(name: &str) -> String {
        format!("“{name}” arşivden çıkarıldı; yeniden düzenlenebilir.")
    }
    pub fn restored(name: &str) -> String {
        format!("“{name}” çöp kutusundan geri yüklendi.")
    }
    pub fn restored_status(name: &str) -> String {
        format!("“{name}” geri yüklendi; listelerinde yeniden görünür.")
    }
    pub fn favorite_added(name: &str) -> String {
        format!("“{name}” favorilere eklendi.")
    }
    pub fn favorite_removed(name: &str) -> String {
        format!("“{name}” favorilerden çıkarıldı.")
    }
    pub fn downloaded(name: &str) -> String {
        format!("“{name}” indirildi.")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::{Value, json};

    use super::*;

    fn fixture() -> Value {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cloud/v1/catalog.json");
        let text = std::fs::read_to_string(path).expect("catalog.json reads");
        serde_json::from_str(&text).expect("catalog.json is JSON")
    }

    /// The fixture's zone: Europe/Istanbul, three hours east all year since 2016.
    fn istanbul() -> Zone {
        Zone::fixed(3 * 3600)
    }

    /// The fixture's project with a case's fields over it.
    fn project(base: &Value, over: &Value) -> ProjectSummary {
        let mut p = base.clone();
        if let (Some(p), Some(over)) = (p.as_object_mut(), over.as_object()) {
            for (k, v) in over {
                p.insert(k.clone(), v.clone());
            }
        }
        serde_json::from_value(p).expect("a project summary")
    }

    fn view_of(v: &Value) -> CatalogView {
        serde_json::from_value(v.clone()).expect("a view")
    }

    fn sort_of(v: &Value) -> CatalogSort {
        serde_json::from_value(v.clone()).expect("a sort")
    }

    #[test]
    fn the_lists_and_labels_are_the_webs() {
        let f = fixture();
        assert_eq!(f["format"], "kentos.catalog");
        assert_eq!(f["version"], 1);
        let views = f["views"].as_array().expect("views");
        assert_eq!(
            views.iter().map(|v| view_of(&v["id"])).collect::<Vec<_>>(),
            words::VIEWS.to_vec(),
            "the lists, in order"
        );
        for v in views {
            let text = words::view(view_of(&v["id"]));
            let id = &v["id"];
            assert_eq!(text.label, v["label"], "{id}");
            assert_eq!(text.empty, v["empty"], "{id}");
            assert_eq!(text.note, v["note"].as_str().unwrap_or(""), "{id}");
            let sorts: Vec<CatalogSort> = v["sorts"]
                .as_array()
                .expect("sorts")
                .iter()
                .map(sort_of)
                .collect();
            assert_eq!(text.sorts, sorts.as_slice(), "{id}");
        }
        for (key, label) in f["sorts"].as_object().expect("sorts") {
            assert_eq!(sort_label(sort_of(&json!(key))), label, "{key}");
        }
        for (key, label) in f["states"].as_object().expect("states") {
            let state: ProjectState = serde_json::from_value(json!(key)).expect("a state");
            assert_eq!(state_label(state), label, "{key}");
        }
        let types = f["types"].as_object().expect("types");
        assert_eq!(types.len(), PROJECT_TYPES.len());
        for (key, label) in types {
            let named: ProjectType = serde_json::from_value(json!(key)).expect("a type");
            assert_eq!(type_label(named), label, "{key}");
        }
        // The web's order, as the file writes it (a JSON map here keeps its keys sorted).
        let text = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cloud/v1/catalog.json"),
        )
        .expect("reads");
        let block = text.split("\"types\":").nth(1).expect("the types");
        let block = &block[..block.find('}').expect("its end")];
        let order: Vec<ProjectType> = block
            .split(',')
            .filter_map(|pair| pair.split(':').next())
            .map(|key| {
                serde_json::from_str::<ProjectType>(key.trim().trim_start_matches('{').trim())
            })
            .collect::<Result<_, _>>()
            .expect("the types' keys");
        assert_eq!(order, PROJECT_TYPES, "the web's order");
        assert_eq!(f["typeHint"], TYPE_HINT);
        assert_eq!(f["emptySearch"], EMPTY_SEARCH);
        assert_eq!(f["noOrganization"], NO_ORGANIZATION);
        assert_eq!(f["tips"]["archived"], ARCHIVED_TEXT);
        let denied = f["tips"]["denied"]
            .as_str()
            .expect("a template")
            .replace("{name}", "Ada 101")
            .replace("{what}", "paylaşma")
            .replace("{permission}", "project.share");
        assert_eq!(
            denied_text("Ada 101", "paylaşma", ProjectPermission::Share),
            denied
        );
    }

    #[test]
    fn the_selected_projects_pane_is_the_webs() {
        let f = fixture();
        for c in f["details"].as_array().expect("details") {
            let id = &c["id"];
            let p = project(&f["project"], &c["project"]);
            let plan = detail_plan(&p, c["open"].as_bool().unwrap_or(false));
            let got = json!({
                "favorite": plan.favorite,
                "chips": plan.chips,
                "tabs": plan.tabs,
                "facts": plan.facts,
                "actions": plan.actions.iter().map(|a| json!({
                    "id": a.id.id(),
                    "label": a.label,
                    "icon": a.icon,
                    "why": a.why,
                    "danger": a.danger,
                })).collect::<Vec<_>>(),
            });
            assert_eq!(got, c["expect"], "{id}");
        }
    }

    #[test]
    fn the_main_button_is_the_webs() {
        let f = fixture();
        for c in f["primary"].as_array().expect("primary") {
            let id = &c["id"];
            let p = (!c["project"].is_null()).then(|| project(&f["project"], &c["project"]));
            let plan = primary_plan(view_of(&c["view"]), p.as_ref());
            let got = json!({"label": plan.label, "enabled": plan.enabled, "why": plan.why});
            assert_eq!(got, c["expect"], "{id}");
        }
    }

    #[test]
    fn a_rows_texts_are_the_webs() {
        let f = fixture();
        assert_eq!(f["timeZone"], "Europe/Istanbul");
        let zone = istanbul();
        for c in f["rows"].as_array().expect("rows") {
            let id = &c["id"];
            let p = project(&f["project"], &c["project"]);
            let plan = row_plan(
                &p,
                view_of(&c["view"]),
                sort_of(&c["sort"]),
                c["open"].as_bool().unwrap_or(false),
                c["place"].as_str().unwrap_or(""),
                &zone,
            );
            let got = json!({"marks": plan.marks, "sub": plan.sub, "side": plan.side});
            assert_eq!(got, c["expect"], "{id}");
        }
    }

    #[test]
    fn the_questions_and_lines_are_the_webs() {
        let f = fixture();
        let q = &f["questions"];
        let seen = |question: Question| {
            json!({
                "title": question.title,
                "message": question.message,
                "details": question.details,
                "action": question.action,
            })
        };
        for c in q["trash"].as_array().expect("trash") {
            let a = &c["args"];
            let got = trash_question(
                a["name"].as_str().unwrap_or(""),
                a["tenant"].as_str().unwrap_or(""),
                a["days"].as_u64().and_then(|d| u32::try_from(d).ok()),
                a["open"].as_bool().unwrap_or(false),
            );
            assert_eq!(seen(got), c["expect"], "{}", c["id"]);
        }
        for c in q["purge"].as_array().expect("purge") {
            let got = purge_question(c["args"]["name"].as_str().unwrap_or(""));
            assert_eq!(seen(got), c["expect"], "{}", c["id"]);
        }
        for c in q["archive"].as_array().expect("archive") {
            let got = archive_question(c["args"]["name"].as_str().unwrap_or(""));
            assert_eq!(seen(got), c["expect"], "{}", c["id"]);
        }
        let zone = istanbul();
        let all = f["lines"].as_array().expect("lines");
        for c in all {
            let args: Vec<Option<&str>> = c["args"]
                .as_array()
                .expect("args")
                .iter()
                .map(Value::as_str)
                .collect();
            let a = |i: usize| args.get(i).copied().flatten().unwrap_or("");
            let got = match c["line"].as_str().unwrap_or("") {
                "trashed" => lines::trashed(a(0), args.get(1).copied().flatten(), &zone),
                "trashFailed" => lines::trash_failed(a(0), a(1)),
                "trashedStatus" => lines::trashed_status(a(0)),
                "purged" => lines::purged(a(0), a(1)),
                "purgeFailed" => lines::purge_failed(a(0), a(1)),
                "purgedStatus" => lines::purged_status(a(0)),
                "archived" => lines::archived(a(0)),
                "archiveFailed" => lines::archive_failed(a(0), a(1)),
                "archivedStatus" => lines::archived_status(a(0)),
                "unarchived" => lines::unarchived(a(0)),
                "unarchivedStatus" => lines::unarchived_status(a(0)),
                "restored" => lines::restored(a(0)),
                "restoredStatus" => lines::restored_status(a(0)),
                "favoriteAdded" => lines::favorite_added(a(0)),
                "favoriteRemoved" => lines::favorite_removed(a(0)),
                "downloaded" => lines::downloaded(a(0)),
                other => panic!("a line the desktop does not write: {other}"),
            };
            assert_eq!(got, c["text"].as_str().unwrap_or(""), "{}", c["line"]);
        }
        assert_eq!(all.len(), 17, "every line is played");
    }
}
