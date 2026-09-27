//! What the status bar's cloud cells say and do, apart from their drawing
//! (the web's ui/statusbar/cellsPlan.ts; docs/adr/0113): the open cloud
//! project's save cell, its tip and what a click on it runs; the server
//! cell's words and tip; and the account menu behind the server cell, with
//! why an action on the open project is off. fixtures/cloud/v1/cells.json
//! holds every answer; both platforms play it (`cells_plan_tests.rs`).

use kentos_contracts::{Health, ProjectPermission};

/// A database project's save state (the web's `SaveState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatabaseState {
    Saved,
    Pending,
    Saving,
    /// The server did not answer; the work waits here.
    OfflinePending,
    Conflict,
    Error,
    ReadOnly,
    Deleted,
    Revoked,
    Archived,
}

impl DatabaseState {
    /// The state's name, the lamp's colour's key (the web's `data-state`).
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Saved => "saved",
            Self::Pending => "pending",
            Self::Saving => "saving",
            Self::OfflinePending => "offline_pending",
            Self::Conflict => "conflict",
            Self::Error => "error",
            Self::ReadOnly => "readonly",
            Self::Deleted => "deleted",
            Self::Revoked => "revoked",
            Self::Archived => "archived",
        }
    }

    /// The cell's words; `n` what waits (the conflicts in a conflict).
    pub(crate) fn text(self, n: usize) -> String {
        match self {
            Self::Saved => "Buluta kaydedildi".to_owned(),
            Self::Pending => format!("Kaydedilecek: {n}"),
            Self::Saving => "Kaydediliyor…".to_owned(),
            Self::OfflinePending => format!("Çevrimdışı: {n} bekliyor"),
            Self::Conflict => format!("Çakışma: {n}"),
            Self::Error => "Kayıt hatası".to_owned(),
            Self::ReadOnly => "Salt okunur".to_owned(),
            Self::Deleted => "Proje silindi".to_owned(),
            Self::Revoked => "Erişim kaldırıldı".to_owned(),
            Self::Archived => "Proje arşivde".to_owned(),
        }
    }
}

/// A file project's Kaydet state (the web's `FileSaveState`, docs/adr/0038).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileState {
    Saved,
    Pending,
    Encoding,
    Uploading,
    Verifying,
    Conflict,
    /// A newer revision on the server: the desktop does not follow a file
    /// project's events yet (docs/adr/0113); the fixture holds the web's case.
    #[allow(dead_code)]
    Outdated,
    Error,
    ReadOnly,
    Deleted,
    Revoked,
    Archived,
}

impl FileState {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Saved => "saved",
            Self::Pending => "pending",
            Self::Encoding => "encoding",
            Self::Uploading => "uploading",
            Self::Verifying => "verifying",
            Self::Conflict => "conflict",
            Self::Outdated => "outdated",
            Self::Error => "error",
            Self::ReadOnly => "readonly",
            Self::Deleted => "deleted",
            Self::Revoked => "revoked",
            Self::Archived => "archived",
        }
    }
}

/// A file project's Kaydet as its cell reads it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileSave {
    pub state: FileState,
    /// The revision the drawing is based on; "0" before the first.
    pub base: String,
    /// The upload, 0…1.
    pub progress: f64,
    /// The revision someone else saved when Kaydet found a conflict.
    pub conflict_actual: Option<String>,
    /// A newer revision on the server the drawing is not based on.
    pub newer_revision: Option<String>,
}

/// What the save cell shows: nothing without a cloud project.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SaveInput {
    /// No cloud project: the desktop draws no cell then (the web hides its own).
    #[allow(dead_code)]
    None,
    Database {
        state: DatabaseState,
        pending: usize,
        conflicts: usize,
    },
    File(FileSave),
}

/// JavaScript's `Math.round`: halves up.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// A file project's Kaydet as the cell says it.
pub(crate) fn file_save_text(f: &FileSave) -> String {
    let unknown = |r: &Option<String>| r.clone().unwrap_or_else(|| "?".to_owned());
    match f.state {
        FileState::Saved if f.base == "0" => "Henüz revizyon yok".to_owned(),
        FileState::Saved => format!("Buluta kaydedildi · r{}", f.base),
        FileState::Pending if f.base == "0" => "Kaydedilmedi".to_owned(),
        FileState::Pending => format!("Kaydedilmedi · r{} üstüne", f.base),
        FileState::Encoding => "Dosya hazırlanıyor…".to_owned(),
        FileState::Uploading => format!("Yükleniyor %{}", js_round(f.progress * 100.0)),
        FileState::Verifying => "Sunucu doğruluyor…".to_owned(),
        FileState::Conflict => format!("Çakışma: r{} kaydedilmiş", unknown(&f.conflict_actual)),
        FileState::Outdated => format!("Yeni revizyon: r{}", unknown(&f.newer_revision)),
        FileState::Error => "Kayıt hatası".to_owned(),
        FileState::ReadOnly => "Salt okunur".to_owned(),
        FileState::Deleted => "Proje silindi".to_owned(),
        FileState::Revoked => "Erişim kaldırıldı".to_owned(),
        FileState::Archived => "Proje arşivde".to_owned(),
    }
}

/// The save cell: hidden without a cloud project; its words and its state (the lamp's colour).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SaveView {
    pub hidden: bool,
    pub text: String,
    pub state: Option<&'static str>,
}

pub(crate) fn save_cell_view(s: &SaveInput) -> SaveView {
    match s {
        SaveInput::None => SaveView {
            hidden: true,
            text: String::new(),
            state: None,
        },
        SaveInput::File(f) => SaveView {
            hidden: false,
            text: file_save_text(f),
            state: Some(f.state.name()),
        },
        SaveInput::Database {
            state,
            pending,
            conflicts,
        } => SaveView {
            hidden: false,
            text: state.text(if *state == DatabaseState::Conflict {
                *conflicts
            } else {
                *pending
            }),
            state: Some(state.name()),
        },
    }
}

/// What a click on the save cell runs, or nothing: a conflict opens the
/// conflicts; a newer revision is offered, never loaded by itself;
/// otherwise it saves, except while read-only or while a Kaydet is on its
/// way. A deleted project or taken access still saves: Kaydet offers a local file.
pub(crate) fn save_cell_action(s: &SaveInput) -> Option<&'static str> {
    match s {
        SaveInput::None => None,
        SaveInput::Database { state, .. } => match state {
            DatabaseState::Conflict => Some("cloud.conflicts"),
            DatabaseState::ReadOnly => None,
            _ => Some("file.save"),
        },
        SaveInput::File(f) => match f.state {
            FileState::Conflict => Some("cloud.conflicts"),
            FileState::Outdated => Some("cloud.openNewest"),
            FileState::ReadOnly
            | FileState::Encoding
            | FileState::Uploading
            | FileState::Verifying => None,
            _ => Some("file.save"),
        },
    }
}

/// How long ago (milliseconds since 1970), in seconds under a minute, else
/// in minutes; “henüz yok” for never.
pub(crate) fn ago(at: Option<i64>, now: i64) -> String {
    let Some(at) = at else {
        return "henüz yok".to_owned();
    };
    #[allow(clippy::cast_precision_loss)]
    let s = js_round((now - at) as f64 / 1000.0);
    if s < 60.0 {
        format!("{s} sn önce")
    } else {
        format!("{} dk önce", js_round(s / 60.0))
    }
}

/// The live link's state (the web's `LinkState`); the desktop asks the
/// server by long polls, so it is online or offline, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkState {
    None,
    /// The web's socket's states; the desktop asks by long polls (docs/adr/0044).
    #[allow(dead_code)]
    Connecting,
    Online,
    #[allow(dead_code)]
    Reconnecting,
    Offline,
    AuthRequired,
}

pub(crate) fn link_text(link: LinkState) -> &'static str {
    match link {
        LinkState::None => "",
        LinkState::Connecting => "bağlanıyor",
        LinkState::Online => "canlı",
        LinkState::Reconnecting => "yeniden bağlanıyor",
        LinkState::Offline => "çevrimdışı",
        LinkState::AuthRequired => "oturum gerekli",
    }
}

/// A cell's tip: its title and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CellTip {
    pub title: &'static str,
    pub description: String,
}

/// What a database project's tip reads.
pub(crate) struct DatabaseTip<'a> {
    pub state: DatabaseState,
    /// “<workspace> › <project>”.
    pub place: &'a str,
    pub last_saved: Option<i64>,
    pub now: i64,
    pub link: LinkState,
    pub error: &'a str,
    /// Unsent changes survive here (the desktop's device draft always does).
    pub durable_drafts: bool,
}

/// A database project's cell tip: where it is, how it saves and when it last
/// did, the live link, the error, and whether drafts survive here.
pub(crate) fn database_tip(t: &DatabaseTip<'_>) -> CellTip {
    let title = "Bulut kaydı";
    let place = t.place;
    let description = match t.state {
        DatabaseState::Deleted => format!(
            "{place} sunucuda silindi. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin."
        ),
        DatabaseState::Revoked => format!(
            "{place} projesine erişiminiz kaldırıldı. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin. Erişim için proje sahibine başvurun."
        ),
        DatabaseState::Archived => format!(
            "{place} arşivlenmiş: salt okunurdur, değişiklikler buluta gönderilmiyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin. Proje sahibi ya da yöneticisi arşivden çıkarınca projeyi yeniden açın."
        ),
        _ => [
            format!("{place}. Değişiklikler kendiliğinden kaydedilir; Ctrl+S hemen gönderir."),
            format!(
                "Son kayıt: {}. Canlı bağlantı: {}.",
                ago(t.last_saved, t.now),
                link_text(t.link)
            ),
            t.error.to_owned(),
            if t.durable_drafts {
                String::new()
            } else {
                "Bu tarayıcı taslakları saklayamıyor: kaydedilmeden kapanırsa değişiklikler kaybolur."
                    .to_owned()
            },
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" "),
    };
    CellTip { title, description }
}

/// What a file project's tip reads.
pub(crate) struct FileTip<'a> {
    pub place: &'a str,
    pub base: &'a str,
    /// This window's last save: its revision and when.
    pub last_saved: Option<(&'a str, i64)>,
    pub now: i64,
    /// A newer revision on the server, and who saved it.
    pub newer: Option<(&'a str, &'a str)>,
    pub error: &'a str,
    pub link: LinkState,
    /// The drawing has unsaved changes (kept here as a recovery copy too).
    pub dirty: bool,
}

/// A file project's cell tip: where the drawing stands, and what Kaydet does.
pub(crate) fn file_tip(t: &FileTip<'_>) -> CellTip {
    let link = link_text(t.link);
    let lines = [
        format!(
            "{}, dosya olarak saklanıyor (KCAD revizyonları).",
            t.place
        ),
        // The number stays apart from its suffix, which would need Turkish
        // vowel harmony by the numeral's reading.
        if t.base == "0" {
            "Projenin henüz revizyonu yok.".to_owned()
        } else {
            format!("Çizimin dayandığı revizyon: {}.", t.base)
        },
        "Kendiliğinden kaydedilmez: Kaydet (Ctrl+S) yeni bir revizyon yazar; arada başkası kaydettiyse üzerine yazılmaz.".to_owned(),
        t.last_saved.map_or(String::new(), |(revision, at)| {
            format!(
                "Bu pencerenin son kaydı: revizyon {revision}, {}.",
                ago(Some(at), t.now)
            )
        }),
        t.newer.map_or(String::new(), |(revision, by)| {
            let by = if by.is_empty() {
                String::new()
            } else {
                format!(" ({by})")
            };
            format!("Sunucuda daha yeni revizyon var: {revision}{by}; açmak için tıklayın.")
        }),
        t.error.to_owned(),
        if link.is_empty() {
            String::new()
        } else {
            format!("Canlı bağlantı: {link}.")
        },
        if t.dirty {
            "Kaydedilmemiş değişiklikler bu cihazda kurtarma kopyası olarak da saklanıyor."
                .to_owned()
        } else {
            String::new()
        },
    ];
    CellTip {
        title: "Bulut kaydı: dosya projesi",
        description: lines
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// The server as its cell names it (the web's `ServerState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerState {
    Checking,
    Online,
    Offline,
    Incompatible,
}

pub(crate) fn server_text(state: ServerState) -> &'static str {
    match state {
        ServerState::Checking => "Sunucu…",
        ServerState::Online => "Sunucu: bağlı",
        ServerState::Offline => "Sunucu: yok",
        ServerState::Incompatible => "Sunucu: uyumsuz",
    }
}

/// The server cell's tip: the service, its build and its contract version
/// when it answers; when its contract is not this app's, the build with the
/// reason (which names both versions); the reason with what the drawing does
/// without it when it does not answer. `dev`: a development build, which
/// says how to start the server.
pub(crate) fn server_tip(
    state: ServerState,
    health: Option<&Health>,
    detail: &str,
    dev: bool,
) -> CellTip {
    let build = health.map_or(String::new(), |h| {
        let commit = h.commit.as_deref().map_or(String::new(), |c| {
            format!(" ({})", c.chars().take(8).collect::<String>())
        });
        format!("{} {}{commit}", h.service, h.version)
    });
    let text = match state {
        ServerState::Online => health.map_or(String::new(), |h| {
            format!("{build}, sözleşme sürümü {}.", h.contracts)
        }),
        ServerState::Incompatible => {
            if health.is_some() {
                format!("{build}. {detail}")
            } else {
                detail.to_owned()
            }
        }
        ServerState::Checking => "Sunucuya soruluyor…".to_owned(),
        ServerState::Offline => format!(
            "{detail} Çizim sunucusuz çalışır; kayıt yerel .kcad dosyasına yapılır.{}",
            if dev {
                " Geliştirmede sunucuyu “pnpm api” ile başlatın."
            } else {
                ""
            }
        ),
    };
    let lead = if text.is_empty() {
        String::new()
    } else {
        format!("{text} ")
    };
    CellTip {
        title: "KentOS sunucusu",
        description: format!("{lead}Hesap, bulut projeleri ve bağlantı denetimi için tıklayın."),
    }
}

/// The open project's actions and the right each needs.
pub(crate) const PROJECT_ACTIONS: [(&str, ProjectPermission); 4] = [
    ("cloud.history", ProjectPermission::History),
    ("cloud.share", ProjectPermission::Share),
    ("cloud.rename", ProjectPermission::Edit),
    ("cloud.delete", ProjectPermission::Delete),
];

/// A row of the account menu: its header, a separator, or a command with,
/// when it is off for want of a right, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AccountRow {
    Header(String),
    Separator,
    Command {
        command: &'static str,
        detail: Option<String>,
    },
}

fn command(command: &'static str) -> AccountRow {
    AccountRow::Command {
        command,
        detail: None,
    }
}

/// The server cell's menu: who is signed in (and the open project's
/// workspace), signing in or out, the cloud commands, the open project's
/// actions, and the server check. An action on the open project that is off
/// because the account lacks its right says which right, and whom to ask.
pub(crate) fn account_rows(
    user: Option<&str>,
    project: Option<&str>,
    disabled: impl Fn(&str) -> bool,
    may: impl Fn(ProjectPermission) -> bool,
) -> Vec<AccountRow> {
    let header = match (user, project) {
        (Some(user), Some(workspace)) => format!("{user} · {workspace}"),
        (Some(user), None) => user.to_owned(),
        (None, _) => "Oturum açılmadı".to_owned(),
    };
    let mut rows = vec![
        AccountRow::Header(header),
        command(if user.is_some() {
            "cloud.signOut"
        } else {
            "cloud.signIn"
        }),
        AccountRow::Separator,
        command("cloud.open"),
        command("cloud.upload"),
        command("cloud.uploadFile"),
    ];
    for (id, permission) in PROJECT_ACTIONS {
        let denied = project.is_some() && disabled(id) && !may(permission);
        rows.push(AccountRow::Command {
            command: id,
            detail: denied.then(|| {
                format!(
                    "Bu projede yetkiniz yok ({}); proje sahibine ya da yöneticisine başvurun.",
                    permission.name()
                )
            }),
        });
    }
    rows.push(AccountRow::Separator);
    rows.push(command("server.check"));
    rows
}
