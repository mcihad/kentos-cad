//! Every route of the API with what it does, who may call it and the shapes it takes and gives: the source of
//! `docs/api/openapi.json` (tests.rs writes and checks it).

use kentos_contracts as c;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Map, Value, json};

pub(super) type Make = fn(&mut SchemaGenerator) -> Schema;

fn of<T: JsonSchema>(g: &mut SchemaGenerator) -> Schema {
    g.subschema_for::<T>()
}

/// What a request carries.
pub(super) enum In {
    No,
    Json(Make),
    /// The file's bytes as the body (`application/octet-stream`).
    Bytes(&'static str),
}

/// What a request is answered with when it succeeds.
pub(super) enum Out {
    Json(u16, Make),
    /// A shape this description does not spell out: its name in the generated TypeScript contracts and a word.
    Shape(u16, &'static str, &'static str),
    Empty(u16),
    /// A stored file's bytes (resumable, cacheable: docs/api/README.md §İndirme).
    File(&'static str),
    Svg,
    Redirect,
    Text(&'static str),
    Upgrade,
}

/// A query parameter: name, JSON type, required, what it is.
pub(super) struct P(pub &'static str, pub &'static str, pub bool, pub &'static str);

pub(super) struct Op {
    pub method: &'static str,
    pub path: &'static str,
    pub tag: &'static str,
    pub summary: &'static str,
    pub about: &'static str,
    pub public: bool,
    pub query: &'static [P],
    pub body: In,
    pub out: Out,
    /// Also answered: 409 (a conflict or an archived project) and 410 (in the trash).
    pub project: bool,
    /// Also answered: 429 (too many tries, or a limit of the account's).
    pub limited: bool,
}

const fn op(
    method: &'static str,
    path: &'static str,
    tag: &'static str,
    summary: &'static str,
    about: &'static str,
    body: In,
    out: Out,
) -> Op {
    Op {
        method,
        path,
        tag,
        summary,
        about,
        public: false,
        query: &[],
        body,
        out,
        project: false,
        limited: false,
    }
}

impl Op {
    const fn public(mut self) -> Self {
        self.public = true;
        self
    }
    const fn query(mut self, q: &'static [P]) -> Self {
        self.query = q;
        self
    }
    const fn project(mut self) -> Self {
        self.project = true;
        self
    }
    const fn limited(mut self) -> Self {
        self.limited = true;
        self
    }
}

const AUTH: &str = "Oturum";
const FILES: &str = "Bulut dosyaları";
const PROJECTS: &str = "Projeler";
const PROJECT_FILES: &str = "Proje revizyonları";
const JOBS: &str = "Sunucu işleri";
const PORTAL_ACCOUNT: &str = "Portal: hesap";
const DOSSIERS: &str = "Portal: dosyalar";
const ORGANIZATION: &str = "Portal: kurum";
const PLATFORM: &str = "Portal: platform yönetimi";
const NOTIFICATIONS: &str = "Bildirimler";
const SHEETS: &str = "Pafta şablonları";
const EXTENSIONS: &str = "Eklenti kataloğu";
const SYSTEM: &str = "Sistem";

/// The groups in the order they are read, with what each holds.
pub(super) const TAGS: &[(&str, &str)] = &[
    (AUTH, "Giriş, çıkış, programın taşıyıcı oturumu, kişisel erişim anahtarları ve açık oturumlar."),
    (FILES, "Hesabın parça parça yüklemesi, taranması; projenin ekleri ve çiziminin resmi (SVG)."),
    (PROJECTS, "Kurumların projeleri: liste, açma, komutlar (her değişiklik bir komuttur), olaylar ve nesneler."),
    (PROJECT_FILES, "Dosya olarak saklanan projenin KCAD revizyonları, yüklemesi, indirmesi ve yedekleri."),
    (JOBS, "Sunucuda çalışan uzun işler: dışa aktarma, içe aktarma, işleme ve denetim."),
    (PORTAL_ACCOUNT, "Portalda hesabın kendisi: meslek mensubu kaydı, profil, parola, katalog."),
    (DOSSIERS, "Belediye ile meslek mensubu arasındaki başvuru ve aktarım dosyaları, belgeleri ve yazışmaları."),
    (ORGANIZATION, "Kurumun portal ayarları, üyeleri ve meslek mensubu rehberi."),
    (PLATFORM, "Sistem yöneticisinin kurumları, hesapları, meslek mensubu doğrulaması ve kayıt defteri."),
    (NOTIFICATIONS, "Hesabın bildirimleri."),
    (SHEETS, "Pafta şablonu kitaplığı."),
    (EXTENSIONS, "Kurumun eklenti kataloğu (yükleme, inceleme, onay, kurulum)."),
    (SYSTEM, "Sağlık, API'nin kendi açıklaması, harita servisleri aracısı ve canlı bağlantı."),
];

const DOSSIER_LIST: &[P] = &[
    P("direction", "string", false, "`application` (başvuru) ya da `transfer` (aktarım)."),
    P("status", "string", false, "Virgülle ayrılmış durumlar (`draft,submitted,…`)."),
    P("q", "string", false, "Numara, başlık, ada/parsel ya da kişi adında arama."),
    P("mine", "boolean", false, "Yalnız çağıranın sorumlu olduğu dosyalar."),
    P("awaiting", "boolean", false, "Yalnız çağıran tarafın sırası olan dosyalar."),
    P("limit", "integer", false, "En çok kaç dosya (1–200, varsayılan 50)."),
    P("offset", "integer", false, "Kaç dosya atlanacak."),
];
const SEARCH: &[P] = &[
    P("q", "string", false, "Ad, e-posta ya da kısa adda arama."),
    P("status", "string", false, "Duruma göre süzme."),
    P("limit", "integer", false, "En çok kaç kayıt."),
];
const EVENTS: &[P] = &[
    P("after", "string", false, "Bu imleçten sonraki olaylar (bir önceki yanıtın `cursor`'ı); yoksa baştan."),
    P("limit", "integer", false, "En çok kaç olay."),
    P("wait", "integer", false, "Yeni olay yoksa en çok bu kadar saniye bekle (uzun yoklama)."),
];
const CANDIDATES: &[P] = &[P("q", "string", false, "Ad ya da e-postada arama.")];
const INLINE: &[P] =
    &[P("inline", "boolean", false, "`1`: PDF ya da resim tarayıcıda/telefonda yerinde gösterilir (indirilmez).")];
const PICTURE: &[P] = &[P("width", "integer", false, "Piksel genişlik, 64–4096 (varsayılan 1200).")];
const PART: &[P] = &[P(
    "offset",
    "integer",
    false,
    "Parçanın dosyadaki başlangıcı: o ana kadar gelen bayt sayısı (`receivedBytes`). Yoksa 0.",
)];

#[allow(clippy::too_many_lines)]
pub(super) fn ops() -> Vec<Op> {
    use In::{Bytes, Json as Body, No};
    use Out::{Empty, File, Json, Redirect, Shape, Svg, Text, Upgrade};
    vec![
        // ── Oturum ──────────────────────────────────────────────────────────
        op("GET", "/v1/auth/config", AUTH, "Giriş yolları",
            "Sunucunun hangi girişleri açtığı (yerel hesap, kurum kimliği/OIDC). Girişten önce sorulur.",
            No, Json(200, of::<c::AuthConfig>)).public(),
        op("POST", "/v1/auth/token", AUTH, "Program girişi: taşıyıcı oturum",
            "Masaüstü, mobil uygulama ya da yapay zekâ ajanı yerel hesabın kullanıcı adı ve parolasıyla girer, oturumu \
             `kentos_s_…` biçiminde bir taşıyıcı anahtar olarak alır (çerez değil). Sonraki her istekte \
             `Authorization: Bearer <token>` gönderilir; `x-kentos-client` başlığı gerekmez. Oturum 7 gün \
             kullanılmayınca, en geç 30 günde biter; `client` ve `label` hesabın oturum listesinde görünür. Yanlış \
             parola 401, art arda yanlışlar 429 (`Retry-After`).",
            Body(of::<c::TokenRequest>), Json(201, of::<c::TokenIssued>)).public().limited(),
        op("POST", "/v1/auth/login", AUTH, "Tarayıcı girişi (çerez)",
            "Tarayıcı içindir: `kentos_session` çerezini yazar ve hesabı döner. Çerezle yapılan her değiştirme \
             isteği `x-kentos-client: web` başlığını taşımalıdır (siteler arası istek koruması). Programlar \
             `POST /v1/auth/token` kullanır.",
            Body(of::<c::LoginRequest>), Json(200, of::<c::Me>)).public().limited(),
        op("POST", "/v1/auth/logout", AUTH, "Çıkış",
            "Taşıyıcı anahtarla çağrılırsa o oturumu kapatır; çerezle çağrılırsa çerezi siler. Her durumda 204.",
            No, Empty(204)).public(),
        op("GET", "/v1/auth/oidc/start", AUTH, "Kurum kimliğiyle giriş: başla",
            "Tarayıcıyı kurumun kimlik sağlayıcısına yönlendirir (OIDC açıksa).",
            No, Redirect).public().query(&[P("returnTo", "string", false, "Girişten sonra dönülecek uygulama yolu.")]),
        op("GET", "/v1/auth/oidc/callback", AUTH, "Kurum kimliğiyle giriş: dönüş",
            "Kimlik sağlayıcının tarayıcıyı geri gönderdiği adres; istemciler doğrudan çağırmaz.",
            No, Redirect).public(),
        op("GET", "/v1/me", AUTH, "Ben",
            "Oturumdaki hesap, üyelikleri (kurumlar ve rolleri) ve yetkileri. Bir anahtarın çalıştığını sınamanın en \
             kısa yolu.",
            No, Json(200, of::<c::Me>)),
        op("GET", "/v1/me/sessions", AUTH, "Açık oturumlar",
            "Hesabın tarayıcı (`cookie`) ve program (`bearer`) oturumları; bu isteğin geldiği oturum `current`.",
            No, Json(200, of::<Vec<c::SessionView>>)),
        op("DELETE", "/v1/me/sessions/{id}", AUTH, "Oturumu kapat",
            "Hesabın bir oturumunu kapatır (kaybolan telefon). O anahtar hemen 401 alır.",
            No, Empty(204)),
        op("GET", "/v1/me/tokens", AUTH, "Kişisel erişim anahtarları",
            "Hesabın `kentos_pat_…` anahtarları: adı, yetkisi (`read`/`write`), son kullanımı. Anahtarın kendisi \
             yalnız oluşturulduğunda bir kez gösterilir.",
            No, Json(200, of::<Vec<c::ApiTokenView>>)),
        op("POST", "/v1/me/tokens", AUTH, "Kişisel erişim anahtarı oluştur",
            "Betik ve yapay zekâ ajanları için adlandırılmış, süreli (1–365 gün) anahtar; hesap başına en çok 20. \
             `read` yalnız okur (`GET`/`HEAD`), `write` hesabın yapabildiği her şeyi yapar. Bir anahtarla yeni \
             anahtar üretilmez ve parola değiştirilmez (403). Yanıttaki `token` bir daha gösterilmez.",
            Body(of::<c::ApiTokenCreate>), Json(201, of::<c::ApiTokenCreated>)).limited(),
        op("DELETE", "/v1/me/tokens/{id}", AUTH, "Anahtarı geri al",
            "Anahtar hemen geçersiz olur.", No, Empty(204)),
        // ── Bulut dosyaları ─────────────────────────────────────────────────
        op("POST", "/v1/uploads", FILES, "Yükleme başlat",
            "Her boyuttaki dosyanın tek yolu (en çok 512 MiB). Dosyanın adı (uzantısıyla), boyutu ve SHA-256'sı \
             bildirilir; henüz bayt gönderilmez. Hesap aynı anda en çok 20 yükleme açık tutar; alınmayan yükleme \
             24 saatte silinir. Sonra baytlar `PUT /v1/uploads/{id}?offset=N` ile parça parça gönderilir.",
            Body(of::<c::UploadBegin>), Json(201, of::<c::UploadView>)).limited(),
        op("PUT", "/v1/uploads/{id}", FILES, "Yüklemeye parça gönder",
            "Gövde bir parçadır (`application/octet-stream`, en çok `partMax` = 32 MiB). `offset` o ana dek gelen \
             bayt sayısı olmalıdır; değilse 422 ve doğru yer iletide yazar. Yarıda kesilen parça hiç yazılmaz: \
             `GET /v1/uploads/{id}` ile `receivedBytes` sorulup oradan sürdürülür. Son parçada boyut ve SHA-256 \
             denetlenir, dosya taranır (izinli tür, içerik–uzantı uyumu, arşivde program, virüs); reddedilen dosya \
             saklanmaz (422, neden iletide) ve yükleme silinir. Geçen dosya `received: true` olur.",
            Bytes("Dosyanın `offset`'ten başlayan parçası."), Json(200, of::<c::UploadView>)).query(PART),
        op("GET", "/v1/uploads/{id}", FILES, "Yükleme ne durumda",
            "Gelen bayt sayısı (`receivedBytes`), tamamlandıysa tarama sonucu. Bir `PUT`'un yanıtı kaybolduysa \
             yeniden göndermeden önce bu sorulur.",
            No, Json(200, of::<c::UploadView>)),
        op("DELETE", "/v1/uploads/{id}", FILES, "Yüklemeden vazgeç", "Baytlar silinir.", No, Empty(204)),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/attachments", FILES, "Projenin ekleri",
            "Çizimin yanında saklanan raporlar, paftalar, çizelgeler, fotoğraflar ve kaynak dosyalar; yeniden \
             eskiye. Projeyi okuyan görür (`project.read`). `viewable` olanlar yerinde gösterilebilir.",
            No, Json(200, of::<Vec<c::AttachmentView>>)).project(),
        op("POST", "/v1/tenants/{tenant}/projects/{project}/attachments", FILES, "Projeye ek koy",
            "Tamamlanmış (`received`) bir yükleme projenin eki olur; yükleme bir kez alınır. Nesneleri \
             değiştirebilen ister (`feature.write`); arşivlenmiş projede 409.",
            Body(of::<c::AttachmentCreate>), Json(201, of::<c::AttachmentView>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/attachments/{attachment}", FILES, "Eki indir",
            "Ekin baytları (`project.download`). `ETag` dosyanın SHA-256'sıdır: `If-None-Match` ile değişmemişse \
             304. `Range` ile parça (206): yarıda kalan indirme sürdürülür, PDF/video telefonda parça parça açılır.",
            No, File("Ekin baytları.")).query(INLINE).project(),
        op("DELETE", "/v1/tenants/{tenant}/projects/{project}/attachments/{attachment}", FILES, "Eki kaldır",
            "`feature.write` ister.", No, Empty(204)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/preview.svg", FILES, "Projenin çizimi (SVG)",
            "Çizimin sunucuda çizilmiş tek SVG resmi: CAD görünümü olmayan telefon, web sayfası ve ajan içindir. \
             Dosya projesinde en yeni (ya da `revision`) revizyon, veritabanı projesinde şimdiki nesneler. Ölçü, \
             tablo, resim gibi nesneler çizilmez; sayıları `x-kentos-drawn`/`x-kentos-skipped` başlıklarında. \
             `ETag` ile değişmemişse 304. Projeyi okuyan görür; eski revizyon `project.history` ister.",
            No, Svg).query(&[
                P("width", "integer", false, "Piksel genişlik, 64–4096 (varsayılan 1200)."),
                P("revision", "integer", false, "Dosya projesinin revizyonu (varsayılan en yenisi)."),
            ]).project(),
        // ── Projeler ────────────────────────────────────────────────────────
        op("GET", "/v1/me/projects", PROJECTS, "Projelerim",
            "Çağıranın her kurumdaki kendi projeleri ve onunla paylaşılanlar.",
            No, Json(200, of::<c::ProjectList>)),
        op("GET", "/v1/me/catalog", PROJECTS, "Proje kataloğu (sayfalı)",
            "Katalog görünümlerinden birinin bir sayfası: aranmış, sıralanmış, erişime göre süzülmüş.",
            No, Json(200, of::<c::ProjectPage>)).query(&[
                P("view", "string", false, "Görünüm (`organization`, `mine`, `shared`, …)."),
                P("tenant", "string", false, "Kurum kimliği (kurum görünümü için)."),
                P("q", "string", false, "Ad ya da açıklamada arama."),
                P("type", "string", false, "Proje türü."),
                P("sort", "string", false, "Sıralama."),
                P("limit", "integer", false, "Sayfa boyu."),
                P("after", "string", false, "Bir önceki sayfanın imleci."),
            ]),
        op("GET", "/v1/tenants/{tenant}/projects", PROJECTS, "Kurumun projeleri",
            "Çağıranın kurumda görebildiği projeler.", No, Json(200, of::<c::ProjectList>)),
        op("POST", "/v1/tenants/{tenant}/projects", PROJECTS, "Proje oluştur",
            "Yeni proje: adı, ayarları (SRID, birimler), katman ağacı, saklama biçimi (`database`: nesne nesne; \
             `file`: KCAD revizyonları).",
            Body(of::<c::ProjectCreate>), Json(201, of::<c::ProjectInfo>)),
        op("POST", "/v1/tenants/{tenant}/commands", PROJECTS, "Projesiz komut",
            "Henüz projesi olmayan ürün komutları (`project.create` gibi); zarfın `projectId`'si boş.",
            Body(of::<c::CommandEnvelope>), Shape(201, "unknown", "Komutun kendi çıktısı.")),
        op("GET", "/v1/tenants/{tenant}/projects/{project}", PROJECTS, "Projeyi aç",
            "Açmak için nesnelerden önce gerekenler: ayarlar, katmanlar, stiller, çağıranın rolü ve yetkileri.",
            No, Json(200, of::<c::ProjectInfo>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/details", PROJECTS, "Proje künyesi",
            "Katalog kaydı, nesne ve katman sayıları, nesnelerin kapladığı alan.",
            No, Json(200, of::<c::ProjectDetails>)).project(),
        op("DELETE", "/v1/tenants/{tenant}/projects/{project}", PROJECTS, "Projeyi çöpe taşı",
            "`project.trash` komutunun eşi; çöpten geri alınabilir. Açık düzenleyicilere canlı haber gider.",
            No, Empty(204)).project(),
        op("POST", "/v1/tenants/{tenant}/projects/{project}/commands", PROJECTS, "Komut çalıştır",
            "Projeyi değiştiren her şey bir komuttur (`project.changes`, paylaşma, katalog ve yaşam döngüsü \
             komutları, `project.file.commit`). Zarf `commandName`, `version`, `requestId`, `idempotencyKey`, \
             `expectedVersions` ve `input` taşır; aynı `idempotencyKey` ile yeniden gönderilen istek ikinci kez \
             uygulanmaz. Beklenen sürüm tutmazsa 409 ve `conflicts`.",
            Body(of::<c::CommandEnvelope>), Shape(200, "unknown", "Komutun kendi çıktısı.")).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/events", PROJECTS, "Olaylar (uzun yoklama)",
            "Bir imleçten sonraki işlenmiş olaylar. `wait` ile yeni olay yoksa bir değişiklik gelene dek beklenir: \
             projeyi düz HTTP ile canlı izlemenin yolu.",
            No, Json(200, of::<c::EventPage>)).query(EVENTS).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/features", PROJECTS, "Nesneler (sayfalı)",
            "Veritabanı projesinin nesneleri; `ids` ile yalnız istenenlerin güncel kopyası.",
            No, Json(200, of::<c::FeaturePage>)).query(&[
                P("after", "string", false, "Bir önceki sayfanın imleci."),
                P("limit", "integer", false, "Sayfa boyu."),
                P("ids", "string", false, "Virgülle ayrılmış nesne kimlikleri."),
            ]).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/blocks", PROJECTS, "Blok tanımları",
            "Projenin blokları ve sürümleri; bir olay değişen blokları söylediğinde sorulur.",
            No, Json(200, of::<c::BlockList>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/snapshot", PROJECTS, "Projenin KCAD kopyası",
            "Veritabanı projesinin bir andaki hâli tek KCAD v2 dosyası olarak (`project.download`); \
             `x-kentos-revision` ve `x-kentos-event-cursor` aynı anı söyler.",
            No, File("KCAD v2 dosyası.")).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/access", PROJECTS, "Kimler erişebilir",
            "Projeyi kimin hangi rolle ve neden kullanabildiği (`project.share`).",
            No, Json(200, of::<c::ProjectAccessList>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/access/candidates", PROJECTS, "Paylaşılabilecek kişiler",
            "Projenin paylaşılabileceği kişiler, ad ya da e-postayla (`project.share`).",
            No, Json(200, of::<c::ShareCandidates>)).query(CANDIDATES).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/invitations", PROJECTS, "Davet bağlantıları",
            "Projenin açık davet bağlantıları (`project.share`).",
            No, Json(200, of::<c::ProjectInvitations>)).project(),
        op("POST", "/v1/invitations/accept", PROJECTS, "Daveti kabul et",
            "Oturumdaki hesap bağlantıdaki davetle projeye katılır.",
            Body(of::<c::InvitationAccept>), Json(200, of::<c::InvitationAccepted>)),
        // ── Proje revizyonları ──────────────────────────────────────────────
        op("GET", "/v1/tenants/{tenant}/projects/{project}/files", PROJECT_FILES, "Revizyonlar",
            "Dosya projesinin KCAD revizyonları, yeniden eskiye (`project.read`).",
            No, Json(200, of::<c::FileRevisions>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/files/{revision}", PROJECT_FILES, "Revizyonu indir",
            "Bir revizyonun baytları (`project.download`); `ETag` SHA-256, `Range: bytes=N-` ile sürdürülür (206).",
            No, File("KCAD v2 dosyası.")).project(),
        op("POST", "/v1/tenants/{tenant}/projects/{project}/uploads", PROJECT_FILES, "Revizyon yüklemesi başlat",
            "Dosya projesine yeni KCAD revizyonu için yükleme (`feature.write`); baytlar gelince \
             `project.file.commit` komutuyla revizyon olur. Proje dışı dosyalar için `/v1/uploads` kullanılır.",
            Body(of::<c::FileUploadBegin>), Json(201, of::<c::FileUpload>)).project(),
        op("PUT", "/v1/tenants/{tenant}/projects/{project}/uploads/{upload}", PROJECT_FILES, "Revizyon baytları",
            "Bütün dosya ya da `?offset=N` ile bir parçası. Boyut, SHA-256 ve KCAD v2 okunabilirliği denetlenir.",
            Bytes("KCAD v2 dosyası ya da parçası."), Json(200, of::<c::FileUpload>)).query(PART).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/uploads/{upload}", PROJECT_FILES, "Revizyon yüklemesi ne durumda",
            "Baytların gelip gelmediği; yanıtı kaybolan `PUT`'tan sonra sorulur.",
            No, Json(200, of::<c::FileUpload>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/checkpoints", PROJECT_FILES, "Yedekler",
            "Veritabanı projesinin yedekleri, yeniden eskiye (`project.history`).",
            No, Json(200, of::<c::ProjectCheckpoints>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/checkpoints/{checkpoint}", PROJECT_FILES, "Yedeği indir",
            "Yedeğin KCAD dosyası (`project.history` ve `project.download`).",
            No, File("KCAD v2 dosyası.")).project(),
        // ── Sunucu işleri ───────────────────────────────────────────────────
        op("GET", "/v1/tenants/{tenant}/projects/{project}/jobs", JOBS, "İşler",
            "Projenin işleri, yeniden eskiye.", No, Json(200, of::<c::ServerJobList>)).project(),
        op("POST", "/v1/tenants/{tenant}/projects/{project}/jobs", JOBS, "İş iste",
            "Dışa aktarma, içe aktarma, işleme ya da denetim işi; aynı `requestId` ile istenen iş geri verilir.",
            Body(of::<c::ServerJobCreate>), Json(201, of::<c::ServerJob>)).project().limited(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/jobs/{job}", JOBS, "İş ne durumda",
            "Durumu, ilerlemesi, sonucu ya da hatası.", No, Json(200, of::<c::ServerJob>)).project(),
        op("POST", "/v1/tenants/{tenant}/projects/{project}/jobs/{job}/cancel", JOBS, "İşi durdur",
            "Kuyruktaki ya da çalışan iş durdurulur.", No, Json(200, of::<c::ServerJob>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/jobs/{job}/artifact", JOBS, "İşin çıktısı",
            "Başarılı dışa aktarmanın dosyası (`project.download`); `ETag` SHA-256, `Range: bytes=N-` (206).",
            No, File("İşin ürettiği dosya.")).project(),
        op("POST", "/v1/tenants/{tenant}/projects/{project}/job-inputs", JOBS, "İş dosyası yüklemesi başlat",
            "İçe aktarma işinin dosyası için yer açar (`jobs.run` ve `feature.write`).",
            Body(of::<c::JobInputBegin>), Json(201, of::<c::JobInput>)).project(),
        op("PUT", "/v1/tenants/{tenant}/projects/{project}/job-inputs/{input}", JOBS, "İş dosyasının baytları",
            "Boyutu ve SHA-256'sı bildirilenle aynıysa saklanır.",
            Bytes("İçe aktarılacak dosya."), Json(200, of::<c::JobInput>)).project(),
        op("GET", "/v1/tenants/{tenant}/projects/{project}/job-inputs/{input}", JOBS, "İş dosyası ne durumda",
            "Baytların gelip gelmediği.", No, Json(200, of::<c::JobInput>)).project(),
        // ── Portal: hesap ───────────────────────────────────────────────────
        op("GET", "/v1/portal/me", PORTAL_ACCOUNT, "Portalda ben",
            "Hesap, portaldaki yüzleri (sistem yönetimi, kurum, meslek mensubu) ve meslek mensubu profili.",
            No, Json(200, of::<c::PortalMe>)),
        op("POST", "/v1/portal/me/password", PORTAL_ACCOUNT, "Parolayı değiştir",
            "Bütün oturumlar kapanır, bu oturum yeniden açılır (çerez). Erişim anahtarıyla yapılamaz (403).",
            Body(of::<c::PasswordChange>), Empty(204)).limited(),
        op("PUT", "/v1/portal/me/professional", PORTAL_ACCOUNT, "Meslek mensubu profili",
            "Oda sicil no, büro, telefon, il, rehberde görünme.",
            Body(of::<c::ProfessionalEdit>), Json(200, of::<c::PortalMe>)),
        op("GET", "/v1/portal/signup", PORTAL_ACCOUNT, "Kayıt açık mı",
            "Meslek mensubu kaydının açık olup olmadığı ve meslekler.",
            No, Json(200, of::<c::SignupInfo>)).public(),
        op("POST", "/v1/portal/signup", PORTAL_ACCOUNT, "Meslek mensubu kaydı",
            "Hesap açılır ve hemen oturum açılır (çerez); profil sistem yöneticisinin doğrulamasını bekler. Kayıt \
             kapalıysa 403.",
            Body(of::<c::Signup>), Json(201, of::<c::PortalMe>)).public().limited(),
        op("GET", "/v1/portal/catalog", PORTAL_ACCOUNT, "Portal kataloğu",
            "Meslekler, dosya türleri, belge türleri ve yasal süreler.", No, Json(200, of::<c::PortalCatalog>)).public(),
        op("GET", "/v1/portal/municipalities", PORTAL_ACCOUNT, "Başvuru alan belediyeler",
            "Meslek mensubunun başvurabileceği kurumlar ve aldıkları dosya türleri.",
            No, Json(200, of::<Vec<c::MunicipalityCard>>)),
        // ── Portal: dosyalar ────────────────────────────────────────────────
        op("GET", "/v1/me/dossiers", DOSSIERS, "Meslek mensubunun dosyaları",
            "Çağıranın başvuruları ve ona gelen aktarımlar.",
            No, Json(200, of::<c::DossierList>)).query(DOSSIER_LIST),
        op("POST", "/v1/me/dossiers", DOSSIERS, "Başvuru oluştur (taslak)",
            "Meslek mensubu bir belediyeye başvuru taslağı açar; belgeler eklenip `send` adımıyla gönderilir.",
            Body(of::<c::ApplicationCreate>), Shape(201, "{ id: string }", "Açılan dosyanın kimliği.")),
        op("GET", "/v1/me/dossiers/summary", DOSSIERS, "Meslek mensubunun özeti",
            "Sayılar ve sırası gelen dosyalar.", No, Json(200, of::<c::DossierDashboard>)),
        op("GET", "/v1/tenants/{tenant}/dossiers", DOSSIERS, "Kurumun dosyaları",
            "Kuruma gelen başvurular ve kurumun aktarımları.",
            No, Json(200, of::<c::DossierList>)).query(DOSSIER_LIST),
        op("POST", "/v1/tenants/{tenant}/dossiers", DOSSIERS, "Aktarım oluştur (taslak)",
            "Kurum bir meslek mensubuna iş aktarır (ölçüm, proje hazırlama…).",
            Body(of::<c::TransferCreate>), Shape(201, "{ id: string }", "Açılan dosyanın kimliği.")),
        op("GET", "/v1/dossiers/{id}", DOSSIERS, "Dosyanın ayrıntısı",
            "Künye, durum, yasal süre, turlar ve belgeleri, yazışmalar, çağıranın atabileceği adımlar.",
            No, Json(200, of::<c::DossierDetail>)),
        op("PATCH", "/v1/dossiers/{id}", DOSSIERS, "Taslağı düzenle",
            "Yalnız taslakken: başlık, açıklama, yer bilgisi.",
            Body(of::<c::DossierEdit>), Json(200, of::<c::DossierDetail>)),
        op("DELETE", "/v1/dossiers/{id}", DOSSIERS, "Taslağı sil", "Yalnız gönderilmemiş taslak.", No, Empty(204)),
        op("POST", "/v1/dossiers/{id}/actions", DOSSIERS, "Adım at",
            "Gönder, incelemeye al, eksik bildir, onayla, reddet, kabul et, teslim et, tamamla… Hangi adımların \
             atılabileceği ayrıntıdaki `actions`'tadır. `expectedStatus` tutmazsa 409 (bu arada başkası adım attı).",
            Body(of::<c::DossierActionRequest>), Json(200, of::<c::DossierDetail>)),
        op("POST", "/v1/dossiers/{id}/messages", DOSSIERS, "Yazışma ekle",
            "Dosyaya ileti; bekleyen ekler iletiyle gönderilir.",
            Body(of::<c::MessagePost>), Json(201, of::<c::DossierMessageView>)),
        op("POST", "/v1/dossiers/{id}/files", DOSSIERS, "Belge ekle (gövdeyle)",
            "Küçük belge için tek istek: gövde dosyanın baytlarıdır (en çok 200 MB). Dosya taranır; reddedilen \
             saklanmaz (422). Büyük ya da kesintili bağlantıda `/v1/uploads` ve `files/from-upload` kullanılır.",
            Bytes("Belgenin baytları."), Json(201, of::<c::DossierFileView>)).query(&[
                P("name", "string", true, "Dosyanın adı, uzantısıyla."),
                P("purpose", "string", true, "`package` (turun belgeleri) ya da `attachment` (sonraki iletiyle)."),
                P("docType", "string", false, "Belge türü (portal kataloğundan); yoksa addan bulunur."),
            ]),
        op("POST", "/v1/dossiers/{id}/files/from-upload", DOSSIERS, "Belge ekle (yüklemeden)",
            "Tamamlanmış bir `/v1/uploads` yüklemesi dosyanın bekleyen belgesi olur; kaydedilemezse yükleme \
             yeniden denemek için yerinde kalır.",
            Body(of::<c::DossierFileFromUpload>), Json(201, of::<c::DossierFileView>)),
        op("POST", "/v1/dossiers/{id}/files/from-project", DOSSIERS, "Belge ekle (bulut projesinden)",
            "Çağıranın bir bulut projesinin şimdiki hâli KCAD belgesi olarak eklenir.",
            Body(of::<c::FromProject>), Json(201, of::<c::DossierFileView>)),
        op("GET", "/v1/dossiers/{id}/files/{file}", DOSSIERS, "Belgeyi indir",
            "Belgenin baytları; `ETag`, `If-None-Match` (304), `Range` (206) ve `inline` ekler gibidir.",
            No, File("Belgenin baytları.")).query(INLINE),
        op("GET", "/v1/dossiers/{id}/files/{file}/preview.svg", DOSSIERS, "KCAD belgesinin çizimi (SVG)",
            "Dosyadaki KentOS proje dosyasının resmi; telefonda ve tarayıcıda açmadan görmek için.",
            No, Svg).query(PICTURE),
        op("DELETE", "/v1/dossiers/{id}/files/{file}", DOSSIERS, "Bekleyen belgeyi kaldır",
            "Gönderilmiş belge kaldırılamaz.", No, Empty(204)),
        op("POST", "/v1/dossiers/{id}/files/{file}/import", DOSSIERS, "KCAD belgesini projeye al",
            "Dosyadaki KCAD belgesi çağıranın yeni bir dosya projesi olur.",
            Body(of::<c::DossierFileImport>), Json(201, of::<c::DossierFileImported>)),
        // ── Portal: kurum ───────────────────────────────────────────────────
        op("GET", "/v1/tenants/{tenant}/portal", ORGANIZATION, "Kurumun portal ayarları",
            "Başvuru alıp almadığı, aldığı dosya türleri, iletişim.", No, Json(200, of::<c::OrganizationPortal>)),
        op("PUT", "/v1/tenants/{tenant}/portal", ORGANIZATION, "Portal ayarlarını değiştir",
            "Kurum yöneticisi.", Body(of::<c::OrganizationPortalEdit>), Json(200, of::<c::OrganizationPortal>)),
        op("GET", "/v1/tenants/{tenant}/portal/summary", ORGANIZATION, "Kurumun özeti",
            "Sayılar, süresi yaklaşan ve sırası gelen dosyalar.", No, Json(200, of::<c::DossierDashboard>)),
        op("GET", "/v1/tenants/{tenant}/members", ORGANIZATION, "Üyeler",
            "Kurumun üyeleri ve rolleri.", No, Json(200, of::<Vec<c::MemberView>>)),
        op("POST", "/v1/tenants/{tenant}/members", ORGANIZATION, "Üye ekle",
            "Kurum yöneticisi; yeni hesap ya da var olan hesap.",
            Body(of::<c::MemberCreate>), Json(201, of::<Vec<c::MemberView>>)),
        op("PATCH", "/v1/tenants/{tenant}/members/{user}", ORGANIZATION, "Üyeliği değiştir",
            "Rol ya da etkinlik; kurumun son sahibi düşürülemez.",
            Body(of::<c::MemberEdit>), Json(200, of::<Vec<c::MemberView>>)),
        op("GET", "/v1/tenants/{tenant}/professionals", ORGANIZATION, "Meslek mensubu rehberi",
            "Doğrulanmış ve rehberde görünmeyi seçmiş meslek mensupları.",
            No, Json(200, of::<Vec<c::ProfessionalCard>>)).query(&[
                P("q", "string", false, "Ad, büro ya da sicil noda arama."),
                P("profession", "string", false, "Meslek (`architect`, `surveyor`, …)."),
            ]),
        // ── Portal: platform yönetimi ───────────────────────────────────────
        op("GET", "/v1/platform/summary", PLATFORM, "Platform özeti",
            "Sistem yöneticisi: kurum, hesap, meslek mensubu ve dosya sayıları.", No, Json(200, of::<c::PlatformSummary>)),
        op("GET", "/v1/platform/tenants", PLATFORM, "Kurumlar", "Sistem yöneticisi.", No, Json(200, of::<Vec<c::PlatformTenant>>)),
        op("POST", "/v1/platform/tenants", PLATFORM, "Kurum aç",
            "Belediye, büro ya da şirket; sahibi yeni ya da var olan hesap.",
            Body(of::<c::PlatformTenantCreate>), Shape(201, "{ id: string }", "Açılan kurumun kimliği.")),
        op("GET", "/v1/platform/tenants/{id}", PLATFORM, "Kurumun ayrıntısı",
            "Üyeler, koltuklar, durum.", No, Json(200, of::<c::PlatformTenantDetail>)),
        op("PATCH", "/v1/platform/tenants/{id}", PLATFORM, "Kurumu değiştir",
            "Ad, koltuk sayısı, askıya alma.", Body(of::<c::PlatformTenantEdit>), Json(200, of::<c::PlatformTenantDetail>)),
        op("GET", "/v1/platform/users", PLATFORM, "Hesaplar", "Sistem yöneticisi.",
            No, Json(200, of::<Vec<c::PlatformUser>>)).query(SEARCH),
        op("POST", "/v1/platform/users", PLATFORM, "Hesap aç", "Yerel hesap.",
            Body(of::<c::PlatformUserCreate>), Shape(201, "{ id: string }", "Açılan hesabın kimliği.")),
        op("PATCH", "/v1/platform/users/{id}", PLATFORM, "Hesabı değiştir",
            "Kapatma/açma, yeni parola, sistem yöneticiliği.", Body(of::<c::PlatformUserEdit>), Empty(204)),
        op("GET", "/v1/platform/professionals", PLATFORM, "Meslek mensupları",
            "Doğrulama bekleyenler ve diğerleri.", No, Json(200, of::<Vec<c::ProfessionalView>>)).query(SEARCH),
        op("POST", "/v1/platform/professionals", PLATFORM, "Meslek mensubu kaydı aç",
            "Yönetici adına kayıt; yine doğrulama bekler.",
            Body(of::<c::Signup>), Shape(201, "{ id: string }", "Açılan hesabın kimliği.")),
        op("POST", "/v1/platform/professionals/{id}/review", PLATFORM, "Meslek mensubunu doğrula",
            "Doğrula, reddet, askıya al ya da yeniden incelemeye al.", Body(of::<c::ProfessionalReview>), Empty(204)),
        op("GET", "/v1/platform/settings", PLATFORM, "Platform ayarları",
            "Kaydın açık olup olmadığı.", No, Json(200, of::<c::PlatformSettings>)),
        op("PUT", "/v1/platform/settings", PLATFORM, "Platform ayarlarını değiştir",
            "Sistem yöneticisi.", Body(of::<c::PlatformSettings>), Json(200, of::<c::PlatformSettings>)),
        op("GET", "/v1/platform/events", PLATFORM, "Kayıt defteri",
            "Platformda kimin ne yaptığı; reddedilen dosyalar dahil.",
            No, Json(200, of::<Vec<c::PlatformEventView>>)).query(SEARCH),
        // ── Bildirimler ─────────────────────────────────────────────────────
        op("GET", "/v1/me/notifications", NOTIFICATIONS, "Bildirimlerim",
            "Yeniden eskiye; okunmamış sayısıyla.",
            No, Json(200, of::<c::Notifications>)).query(&[
                P("unread", "boolean", false, "Yalnız okunmamışlar."),
                P("limit", "integer", false, "En çok kaç bildirim."),
            ]),
        op("POST", "/v1/me/notifications/read", NOTIFICATIONS, "Okundu işaretle",
            "Belirli bildirimler ya da hepsi; listenin şimdiki hâli döner.",
            Body(of::<c::NotificationsRead>), Json(200, of::<c::Notifications>)),
        // ── Pafta şablonları ────────────────────────────────────────────────
        op("GET", "/v1/me/sheet-templates", SHEETS, "Şablonlarım",
            "Çağıranın ve onunla paylaşılan pafta şablonları.",
            No, Shape(200, "SheetTemplateList", "Şablonların listesi.")),
        op("GET", "/v1/me/sheet-templates/events", SHEETS, "Şablon olayları (uzun yoklama)",
            "Bir imleçten sonraki şablon olayları; `wait` ile beklenir.",
            No, Shape(200, "SheetTemplateEventPage", "Olay sayfası.")).query(EVENTS),
        op("GET", "/v1/sheet-templates/{id}", SHEETS, "Şablon",
            "Künyesi ve en yeni içeriği; `ETag` revizyondur, değişmemişse 304.",
            No, Shape(200, "SheetTemplate", "Şablonun künyesi ve içeriği.")),
        op("GET", "/v1/sheet-templates/{id}/access", SHEETS, "Şablonun paylaşımı",
            "Yalnız sahibi.", No, Shape(200, "SheetTemplateAccess", "Paylaşılan kişiler.")),
        op("GET", "/v1/sheet-templates/{id}/access/candidates", SHEETS, "Şablonun paylaşılabileceği kişiler",
            "Yalnız sahibi.", No, Shape(200, "SheetTemplateCandidates", "Aday kişiler.")).query(CANDIDATES),
        // ── Eklenti kataloğu ────────────────────────────────────────────────
        op("GET", "/v1/extension-catalogs/{catalog}/releases", EXTENSIONS, "Sürümler",
            "Katalogdaki eklenti sürümleri.", No, Shape(200, "ReleaseList", "Sürümlerin sayfası."))
            .query(&[
                P("q", "string", false, "Arama."),
                P("state", "string", false, "Durum."),
                P("extension", "string", false, "Eklenti kimliği."),
                P("offset", "integer", false, "Atlanacak sayı."),
                P("limit", "integer", false, "Sayfa boyu."),
            ]),
        op("POST", "/v1/extension-catalogs/{catalog}/releases", EXTENSIONS, "Sürüm yükle (taslak)",
            "`.kek` paketinin baytları; incelemeye taslak olarak girer (201, `Location`).",
            Bytes("`.kek` paketi."), Shape(201, "ReleaseView", "Sürümün kaydı.")),
        op("GET", "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}", EXTENSIONS, "Sürümün kaydı",
            "Durumu, incelemesi, imzası.", No, Shape(200, "ReleaseView", "Sürümün kaydı.")),
        op("PUT", "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}", EXTENSIONS, "Sürümü yenile",
            "Taslağın içeriği yeni `.kek` baytlarıyla değişir.",
            Bytes("`.kek` paketi."), Shape(200, "ReleaseView", "Sürümün kaydı.")),
        op("POST", "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}/moves", EXTENSIONS, "Akışta adım",
            "İncelemeye gönder, onayla, reddet, geri çek…",
            In::Json(|g| g.subschema_for::<serde_json::Value>()), Shape(200, "ReleaseView", "Sürümün kaydı.")),
        op("GET", "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}/install", EXTENSIONS, "Kurulum bileti",
            "Onaylı içerik ya da neden kurulmaması gerektiği.", No, Shape(200, "InstallTicket", "Kurulum bilgisi.")),
        op("GET", "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}/package", EXTENSIONS, "Paketi indir",
            "Onaylı paketin baytları.", No, File("`.kek` paketi.")),
        op("GET", "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}/sbom", EXTENSIONS, "Yazılım malzeme listesi",
            "Onaylı paketin SBOM'u (CycloneDX 1.6 JSON).", No, Shape(200, "CycloneDX", "SBOM belgesi.")),
        op("GET", "/v1/extension-catalogs/{catalog}/revocations", EXTENSIONS, "Geri çekilenler",
            "Kurulmaması gereken sürümlerin imzalı listesi.", No, Shape(200, "Revocations", "Geri çekilen sürümler.")),
        // ── Sistem ──────────────────────────────────────────────────────────
        op("GET", "/v1/health", SYSTEM, "Sağlık",
            "Sunucunun sürümü ve sözleşme sürümleri; veritabanı olmadan da yanıt verir.",
            No, Json(200, of::<c::Health>)).public(),
        op("GET", "/v1/openapi.json", SYSTEM, "Bu açıklama",
            "Bütün uçların OpenAPI 3.1 açıklaması (bu belge).", No, Text("application/json")).public(),
        op("GET", "/v1/llms.txt", SYSTEM, "Yapay zekâ ajanları için kısa yol",
            "Girişi, akışları ve kuralları anlatan kısa metin: bir ajanın ilk okuyacağı.",
            No, Text("text/plain")).public(),
        op("GET", "/v1/proxy", SYSTEM, "Harita servisi aracısı (GET)",
            "İzinli harita servislerine (WMS/WMTS/XYZ) tarayıcı adına istek.",
            No, File("Servisin yanıtı.")).query(&[P("url", "string", true, "Hedef adres.")]),
        op("POST", "/v1/proxy", SYSTEM, "Harita servisi aracısı (POST)",
            "İzinli harita servislerine (WFS) tarayıcı adına istek.",
            Bytes("Servise gidecek gövde."), File("Servisin yanıtı.")).query(&[P("url", "string", true, "Hedef adres.")]),
        op("GET", "/v1/ws", SYSTEM, "Canlı bağlantı (WebSocket)",
            "Açık projelerin olaylarını anında iletir.", No, Upgrade),
    ]
}

/// What a path parameter is, by its name and where it stands.
fn param_about(path: &str, name: &str) -> &'static str {
    match name {
        "tenant" => "Kurumun kimliği (UUID); `GET /v1/me` üyeliklerinde.",
        "project" => "Projenin kimliği (UUID).",
        "attachment" => "Ekin kimliği.",
        "revision" => "Revizyon numarası (1, 2, …).",
        "checkpoint" => "Yedeğin kimliği.",
        "job" => "İşin kimliği.",
        "input" => "İş dosyasının kimliği.",
        "upload" => "Revizyon yüklemesinin kimliği.",
        "user" => "Hesabın kimliği.",
        "file" => "Dosyadaki belgenin kimliği.",
        "catalog" => "Eklenti kataloğunun adı.",
        "extension" => "Eklentinin kimliği.",
        "version" => "Sürüm (semver).",
        "id" if path.starts_with("/v1/uploads") => "Yüklemenin kimliği.",
        "id" if path.starts_with("/v1/dossiers") => "Dosyanın (başvuru ya da aktarım) kimliği.",
        "id" if path.starts_with("/v1/me/sessions") => "Oturumun kimliği.",
        "id" if path.starts_with("/v1/me/tokens") => "Anahtarın kimliği.",
        "id" if path.starts_with("/v1/sheet-templates") => "Şablonun kimliği.",
        "id" if path.starts_with("/v1/platform/tenants") => "Kurumun kimliği.",
        _ => "Kaydın kimliği.",
    }
}

fn error(status: u16) -> (&'static str, &'static str) {
    match status {
        401 => ("Unauthenticated", "Oturum yok ya da bitti: yeniden giriş."),
        403 => ("Forbidden", "Hesabın bu iş için yetkisi yok (ileti hangi yetkinin eksik olduğunu söyler)."),
        404 => ("NotFound", "Yok ya da çağıran göremiyor (ikisi ayırt edilmez)."),
        409 => ("Conflict", "Çakışma: beklenen sürüm/durum değişmiş ya da proje arşivde."),
        410 => ("Gone", "Proje çöpte."),
        422 => ("Invalid", "Girdi geçersiz; `path` hangi alan olduğunu söyler."),
        _ => ("Limited", "Çok fazla deneme ya da hesabın sınırı; `Retry-After` kadar bekleyin."),
    }
}

/// The OpenAPI 3.1 document.
pub(super) fn document() -> Value {
    let mut settings = schemars::generate::SchemaSettings::draft2020_12();
    settings.definitions_path = "/components/schemas".into();
    settings.meta_schema = None;
    let mut g = settings.into_generator();
    let error_schema = g.subschema_for::<c::ApiError>().to_value();
    let mut paths = Map::new();
    for o in ops() {
        let mut parameters: Vec<Value> = Vec::new();
        for name in o.path.split('/').filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}'))) {
            parameters.push(json!({
                "name": name, "in": "path", "required": true,
                "description": param_about(o.path, name), "schema": { "type": "string" }
            }));
        }
        for P(name, kind, required, about) in o.query {
            parameters.push(json!({
                "name": name, "in": "query", "required": required, "description": about, "schema": { "type": kind }
            }));
        }
        let mut responses = Map::new();
        let (status, response) = match &o.out {
            Out::Json(s, make) => (*s, json!({ "description": "Başarılı.", "content": { "application/json": { "schema": make(&mut g).to_value() } } })),
            Out::Shape(s, name, about) => (*s, json!({ "description": about, "content": { "application/json": {
                "schema": { "type": "object", "description": format!("{about} TypeScript sözleşmesinde: `{name}`.") } } } })),
            Out::Empty(s) => (*s, json!({ "description": "Başarılı; gövde yok." })),
            Out::File(about) => (200, json!({
                "description": format!("{about} `ETag`, `Range` (206) ve `If-None-Match` (304) desteklenir."),
                "headers": { "ETag": { "schema": { "type": "string" } }, "Content-Range": { "schema": { "type": "string" } } },
                "content": { "application/octet-stream": { "schema": { "type": "string", "contentMediaType": "application/octet-stream" } } }
            })),
            Out::Svg => (200, json!({
                "description": "SVG resmi; `ETag` ile değişmemişse 304.",
                "headers": {
                    "ETag": { "schema": { "type": "string" } },
                    "x-kentos-drawn": { "description": "Çizilen nesne sayısı.", "schema": { "type": "integer" } },
                    "x-kentos-skipped": { "description": "Çizilmeyen nesne sayısı.", "schema": { "type": "integer" } }
                },
                "content": { "image/svg+xml": { "schema": { "type": "string" } } }
            })),
            Out::Redirect => (302, json!({ "description": "Yönlendirme (`Location`)." })),
            Out::Text(kind) => (200, json!({ "description": "Başarılı.", "content": { *kind: { "schema": { "type": "string" } } } })),
            Out::Upgrade => (101, json!({ "description": "WebSocket'e geçiş." })),
        };
        responses.insert(status.to_string(), response);
        if matches!(o.out, Out::File(_)) && o.method == "GET" {
            responses.insert("206".into(), json!({ "description": "İstenen parça (`Range`)." }));
            responses.insert("304".into(), json!({ "description": "Değişmedi (`If-None-Match`)." }));
        }
        if matches!(o.out, Out::Svg) {
            responses.insert("304".into(), json!({ "description": "Değişmedi (`If-None-Match`)." }));
        }
        let mut errors: Vec<u16> = Vec::new();
        if !o.public {
            errors.extend([401, 403, 404]);
        }
        if !matches!(o.body, In::No) || !o.query.is_empty() || o.path.contains('{') {
            errors.push(422);
        }
        if o.project {
            errors.extend([409, 410]);
        }
        if o.limited || o.path.starts_with("/v1/dossiers/{id}/actions") {
            errors.push(if o.limited { 429 } else { 409 });
        }
        errors.sort_unstable();
        errors.dedup();
        for e in errors {
            responses.insert(e.to_string(), json!({ "$ref": format!("#/components/responses/{}", error(e).0) }));
        }
        let mut operation = json!({
            "tags": [o.tag],
            "summary": o.summary,
            "description": o.about,
            "operationId": operation_id(o.method, o.path),
            "responses": responses,
        });
        if !parameters.is_empty() {
            operation["parameters"] = Value::Array(parameters);
        }
        match &o.body {
            In::No => {}
            In::Json(make) => {
                operation["requestBody"] = json!({ "required": true, "content": { "application/json": { "schema": make(&mut g).to_value() } } });
            }
            In::Bytes(about) => {
                operation["requestBody"] = json!({ "required": true, "description": about,
                    "content": { "application/octet-stream": { "schema": { "type": "string", "contentMediaType": "application/octet-stream" } } } });
            }
        }
        if o.public {
            operation["security"] = json!([]);
        }
        let entry = paths.entry(o.path.to_owned()).or_insert_with(|| json!({}));
        entry[o.method.to_lowercase()] = operation;
    }
    let mut schemas = g.take_definitions(true);
    schemas.sort_keys();
    let error_responses: Map<String, Value> = [401u16, 403, 404, 409, 410, 422, 429]
        .into_iter()
        .map(|s| {
            let (name, about) = error(s);
            (name.to_owned(), json!({ "description": about, "content": { "application/json": { "schema": error_schema.clone() } } }))
        })
        .collect();
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "KentOS Bulut API",
            "version": env!("CARGO_PKG_VERSION"),
            "summary": "Masaüstü, mobil uygulama, web portalı ve yapay zekâ ajanları için tek API.",
            "description": include_str!("intro.md"),
        },
        "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema",
        "servers": [{ "url": "/", "description": "Bu sunucu." }],
        "security": [{ "bearer": [] }, { "session": [] }],
        "tags": TAGS.iter().map(|(name, about)| json!({ "name": name, "description": about })).collect::<Vec<_>>(),
        "paths": paths,
        "components": {
            "securitySchemes": {
                "bearer": {
                    "type": "http", "scheme": "bearer",
                    "description": "`POST /v1/auth/token` ile alınan program oturumu (`kentos_s_…`), hesabın kişisel erişim \
                        anahtarı (`kentos_pat_…`) ya da kurum kimliğinin JWT'si. `Authorization: Bearer <anahtar>`."
                },
                "session": {
                    "type": "apiKey", "in": "cookie", "name": "kentos_session",
                    "description": "Tarayıcının oturum çerezi (`POST /v1/auth/login`). Çerezle yapılan her değiştirme isteği \
                        `x-kentos-client: web` başlığını taşır."
                }
            },
            "responses": error_responses,
            "schemas": schemas,
        }
    })
}

/// `GET /v1/tenants/{tenant}/projects` → `getTenantsProjects`: stable and readable.
pub(super) fn operation_id(method: &str, path: &str) -> String {
    let mut id = method.to_lowercase();
    for part in path.trim_start_matches("/v1/").split('/') {
        let (word, by) = match part.strip_prefix('{').and_then(|p| p.strip_suffix('}')) {
            Some(p) => (p, true),
            None => (part, false),
        };
        if by {
            id.push_str("By");
        }
        for piece in word.split(['-', '.', '_']) {
            let mut cs = piece.chars();
            if let Some(first) = cs.next() {
                id.extend(first.to_uppercase());
                id.push_str(cs.as_str());
            }
        }
    }
    id
}
