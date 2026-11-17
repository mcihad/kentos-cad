//! İmza uçları (docs/adr/9002 §4).
//!
//! **Batch oluşturmak İMZALI İSTEK ister**: tarayıcının localhost servisine erişebilmesi imza yetkisi anlamına
//! gelmez. Güven anahtarı yapılandırılmamışsa
//! `POST /signing/batches` **reddedilir** — ham gövde kabul eden bir "kolaylık yolu"
//! bırakmak, tarayıcıdaki herhangi bir sayfanın imza isteği üretebilmesi demekti.
//!
//! **Yollar `/api/v1` önekli** ve sürümsüz kardeşi YOK.

use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::{
    BatchRecord, BatchResult, BatchStatus, CertificateInfo, DeviceInfo, FileSignature,
    SigningBatch, StateMachine as _,
};
use kentos_imza_signature::Token;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::state::{AppState, PinEntryMode, now_rfc3339};

// ─────────────────────────────────────────────────────────────────────────────
// Hata → HTTP
// ─────────────────────────────────────────────────────────────────────────────

/// `ImzaError`i HTTP yanıtına çevirir.
///
/// **Durum kodu hata SINIFINDAN türetilir, elle seçilmez**: her uçta `400` mü
/// `409` mu diye karar vermek, aynı hatanın iki uçta iki farklı kodla dönmesine yol
/// açardı ve istemci ikisini ayrı hata sanardı.
struct Hata(ImzaError);

impl IntoResponse for Hata {
    fn into_response(self) -> Response {
        let kod = match self.0.code.as_str() {
            // Kimlik/yetki sınıfı — imzalı istek geçersiz ya da tekrar edilmiş.
            "KSI-1502" | "KSI-1503" => StatusCode::UNAUTHORIZED,
            "KSI-1501" => StatusCode::GONE,
            "KSI-1500" | "KSI-1600" | "KSI-1601" => StatusCode::BAD_REQUEST,
            "KSI-1101" | "KSI-1201" => StatusCode::NOT_FOUND,
            // PIN hataları `403` DEĞİL: `403` "yetkin yok" der ve kullanıcı
            //    yanlış PIN girdiğini anlamaz. `422` = "isteği anladım, içeriği
            //    kabul edilemez" — ve gövdedeki `KSI-1302` gerçeği söyler.
            "KSI-1300" | "KSI-1301" | "KSI-1302" | "KSI-1303" => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (kod, Json(self.0)).into_response()
    }
}

impl From<ImzaError> for Hata {
    fn from(e: ImzaError) -> Self {
        Self(e)
    }
}

type Sonuc<T> = Result<T, Hata>;

// ─────────────────────────────────────────────────────────────────────────────
// Cihaz ve sertifika
// ─────────────────────────────────────────────────────────────────────────────

fn token(state: &AppState) -> Result<&std::sync::Arc<dyn Token>, ImzaError> {
    state.token().ok_or_else(|| {
        ImzaError::new(
            ErrorCode::DeviceError,
            "PKCS#11 kütüphanesi yapılandırılmadı (KENTOS_IMZA_PKCS11)",
        )
    })
}

/// `GET /api/v1/devices`.
async fn devices(State(state): State<AppState>) -> Sonuc<Json<Vec<DeviceInfo>>> {
    let m = token(&state)?.clone();
    // PKCS#11 **bloke eden** bir C API'sidir; async yürütücünün iş parçacığında
    //    çağırmak, kart cevap verene kadar bütün WebSocket'leri dondururdu.
    let cihazlar = tokio::task::spawn_blocking(move || m.devices())
        .await
        .map_err(gorev_hatasi)??;
    Ok(Json(cihazlar))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SertifikaSorgu {
    /// Hangi yuva — verilmezse ilk cihaz.
    slot_id: Option<u64>,
}

/// `GET /api/v1/certificates`.
async fn certificates(
    State(state): State<AppState>,
    Query(q): Query<SertifikaSorgu>,
) -> Sonuc<Json<Vec<CertificateInfo>>> {
    let m = token(&state)?.clone();
    let liste = tokio::task::spawn_blocking(move || {
        let slot = match q.slot_id {
            Some(s) => s,
            None => {
                m.devices()?
                    .first()
                    .ok_or_else(|| {
                        ImzaError::new(ErrorCode::DeviceNotFound, "takılı bir cihaz bulunamadı")
                    })?
                    .slot_id
            }
        };
        m.certificates(slot)
    })
    .await
    .map_err(gorev_hatasi)??;
    Ok(Json(liste))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PinDogrulaIstek {
    slot_id: u64,
    pin: String,
}

/// PIN gövdede geliyor — `Debug` onu ASLA basmaz.
impl std::fmt::Debug for PinDogrulaIstek {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinDogrulaIstek")
            .field("slot_id", &self.slot_id)
            .field("pin", &"<gizlendi>")
            .finish()
    }
}

/// `POST /api/v1/devices/pin-check` — **PIN'i imzalamadan doğrular** (kullanıcı
/// isteği, 2026-08-31).
///
/// **TARAYICIDAN ÇAĞRILAMAZ** — `batch_pin` ile aynı gerekçe: `Origin` taşıyan
/// istek reddedilir. Bu uç PIN alıyor; tarayıcıya açmak, `native` kipi anlamsız
/// kılardı.
///
/// **Batch'e DOKUNMAZ**: hiçbir durum geçişi yapmaz, hiçbir olay yaymaz. Amacı
/// tam olarak budur — yanlış PIN yüzünden batch'in uç duruma düşmesini ve
/// kullanıcının bütün akışı web'den yeniden başlatmasını önlemek.
async fn pin_dogrula(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(istek): Json<PinDogrulaIstek>,
) -> Sonuc<StatusCode> {
    if headers.contains_key(axum::http::header::ORIGIN) {
        return Err(ImzaError::new(
            ErrorCode::PinError,
            "PIN doğrulama ucu tarayıcıdan çağrılamaz (yerli agent'a aittir)",
        )
        .into());
    }
    let m = token(&state)?.clone();
    // `spawn_blocking`: kart cevabı saniyeler sürebilir ve olay döngüsünü
    //    bloklamak bütün WebSocket'leri dondururdu (bkz. `devices`).
    tokio::task::spawn_blocking(move || m.verify_pin(istek.slot_id, &istek.pin))
        .await
        .map_err(gorev_hatasi)??;
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Batch yaşam döngüsü
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchOlusturIstek {
    /// KentOS sunucusunun **Ed25519 ile imzaladığı** JWS-compact zarf.
    signed_request: String,
}

/// Batch'in istemciye dönen hâli: batch'in kendisi, şimdiye kadar **doğrulanmış** imzalar (dosya kimliğiyle) ve son
/// hata.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchYanit {
    #[serde(flatten)]
    pub batch: SigningBatch,
    pub results: Vec<FileSignature>,
    pub last_error: Option<ImzaError>,
    /// İmzalanması gereken toplam dosya.
    pub signing_required_count: usize,
}

/// `POST /api/v1/signing/batches`.
async fn batch_olustur(
    State(state): State<AppState>,
    Json(istek): Json<BatchOlusturIstek>,
) -> Sonuc<(StatusCode, Json<BatchYanit>)> {
    let guven = state.trust().ok_or_else(|| {
        ImzaError::new(
            ErrorCode::RequestSignatureInvalid,
            "güven anahtarı yapılandırılmadı — imzalı istek doğrulanamıyor",
        )
    })?;

    // **Önce imza**, sonra içerik. Sıra tersine dönerse, doğrulanmamış bir gövdenin alanlarına göre karar vermiş
    // oluruz.
    let payload =
        kentos_imza_security::signed_request::verify(&istek.signed_request, guven, &now_rfc3339())?;

    let batch = payload.to_batch()?;
    // Replay kapısı — aynı `transactionId`/`nonce` ikinci kez geçmez.
    state.insert_batch(batch.clone(), &payload.nonce)?;
    state.emit(
        "batch_created",
        Some(&batch),
        serde_json::json!({
            "packages": batch.package_count(),
            "files": batch.file_count(),
            "userId": payload.user_id,
        }),
    );
    let girdi = state.batch(&batch.id)?;
    Ok((StatusCode::CREATED, Json(yanit(girdi))))
}

/// `GET /api/v1/signing/batches/{id}`.
async fn batch_getir(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sonuc<Json<BatchYanit>> {
    Ok(Json(yanit(state.batch(&id)?)))
}

/// `POST /api/v1/signing/batches/{id}/open`.
///
/// Kullanıcı imza penceresini AÇTI demektir: `CREATED → VALIDATING → WAITING_FOR_USER → READY`. Bu adım olmadan hiçbir
/// imza atılamaz. **İdempotent**: diyalog iki kez açılırsa ikinci çağrı aynı sonucu döner; aksi hâlde ağ tekrarı
/// akışı kırardı. Kural (özet biçimi, dosya durumları) alan modelindedir: [`BatchRecord::open`].
async fn batch_ac(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sonuc<Json<BatchYanit>> {
    if state.batch(&id)?.batch.status == BatchStatus::Ready {
        return Ok(Json(yanit(state.batch(&id)?)));
    }
    state.update_batch(&id, BatchRecord::open)?;

    let girdi = state.batch(&id)?;
    state.emit(
        "batch_opened",
        Some(&girdi.batch),
        serde_json::json!({ "files": girdi.batch.file_count() }),
    );
    Ok(Json(yanit(girdi)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImzalaIstek {
    /// Tarayıcının önerdiği yuva ve sertifika. `native` kipte isteğe bağlıdır: seçimi ajan penceresinde kullanıcı
    /// yapar ve **o seçim bağlayıcıdır**; `client` kipte ikisi de zorunludur.
    #[serde(default)]
    slot_id: Option<u64>,
    #[serde(default)]
    certificate_id: Option<String>,
    /// **Yalnız `client` PIN kipinde.** `native` kipte bu alan gelse bile KULLANILMAZ.
    #[serde(default)]
    pin: Option<String>,
}

// `Debug` KASTEN türetilmedi: `{:?}` ile loglanan bir yapı PIN'i loga yazardı.
impl std::fmt::Debug for ImzalaIstek {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImzalaIstek")
            .field("slot_id", &self.slot_id)
            .field("certificate_id", &self.certificate_id)
            .field("pin", &"<gizlendi>")
            .finish()
    }
}

/// `POST /api/v1/signing/batches/{id}/sign`.
///
/// **202 döner ve arka planda çalışır**: imzalama kart hızında ilerler ve ilerleme
/// WebSocket'ten akar. Senkron beklemek, çok dosyalı bir batch'te tarayıcıyı
/// dakikalarca açık bir istekte tutardı.
async fn batch_imzala(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(istek): Json<ImzalaIstek>,
) -> Sonuc<(StatusCode, Json<BatchYanit>)> {
    let girdi = state.batch(&id)?;
    if girdi.batch.status != BatchStatus::Ready {
        return Err(ImzaError::new(
            ErrorCode::RequestError,
            format!(
                "batch imzalamaya hazır değil (durum: {:?}) — önce /open çağrılmalı",
                girdi.batch.status
            ),
        )
        .into());
    }

    // ── PIN nereden gelir (docs/adr/9002 §4) ─────────────────────────────────
    //
    // İki kip de GERÇEK ve ikisi de aynı akışı kullanır: fark yalnız `pin_required` olayına KİMİN cevap verdiğidir.
    //   · `NATIVE` — ajan penceresi, PIN'i `POST …/pin` ile getirir (varsayılan)
    //   · `CLIENT` — tarayıcı, PIN'i istek gövdesinde getirir (geliştirme; sapma)
    let kip = state.pin_entry();

    // **Agent kontrolü PKCS#11'den ÖNCE** ve sıra bilinçli: eksik sürücü bir
    //    KURULUM sorunudur (yönetici bir kez çözer), kapalı bir PIN penceresi ise
    //    kullanıcının kendi çözebileceği bir durumdur ve her gün olabilir. Sürücü
    //    hatasını önce göstermek, kullanıcıyı yanlış yere gönderirdi.
    if kip == PinEntryMode::Native && !state.agent_bagli() {
        // Agent yoksa imza BAŞLAMAZ. Başlatıp beklemek, kullanıcıyı hiç
        //    açılmayacak bir pencereyi beklemeye bırakırdı.
        return Err(ImzaError::new(
            ErrorCode::PinError,
            "KentOS İmza ajanı (PIN penceresi) çalışmıyor; ajanı başlatın",
        )
        .into());
    }

    let modul = token(&state)?.clone();

    // Kanal, olay YAYINLANMADAN ÖNCE açılır: agent `pin_required`i milisaniyeler
    //    içinde alıyor ve "bekleyen yok" cevabıyla karşılaşmamalı.
    let pin_kanali = if kip == PinEntryMode::Native {
        Some(state.pin_bekle(&id)?)
    } else {
        None
    };

    state.emit(
        "pin_required",
        Some(&girdi.batch),
        serde_json::json!({
            "pinEntry": kip,
            "title": girdi.batch.title,
            "totalFiles": girdi.batch.signing_required_count(),
            "certificateId": istek.certificate_id,
            "slotId": istek.slot_id,
        }),
    );

    let istemci_pin = istek.pin.clone();
    let arka = state.clone();
    let batch_id = id.clone();
    let mut secim: Option<(u64, String)> = istek.slot_id.zip(istek.certificate_id.clone());

    tokio::spawn(async move {
        // ── PIN'i çöz ────────────────────────────────────────────────────────
        let pin = match pin_kanali {
            // `NATIVE`: agent penceresini bekle.
            Some(rx) => match tokio::time::timeout(crate::state::PIN_BEKLEME, rx).await {
                Ok(Ok(crate::state::PinCevabi::Girildi {
                    pin,
                    secim: penceredeki,
                })) => {
                    // Ajan penceresindeki seçim BAĞLAYICI: kullanıcı imzayı orada onayladı ve orada gördüğü
                    //    imzalayan ile gerçekte imzalayan aynı olmak zorunda.
                    if let Some(s) = penceredeki {
                        secim = Some((s.slot_id, s.certificate_id));
                    }
                    pin
                }
                Ok(Ok(crate::state::PinCevabi::Iptal)) => {
                    arka.pin_bekleyeni_dusur(&batch_id);
                    // `Cancelled` — `Failed` DEĞİL. Vazgeçmek bir hata değildir ve
                    //    kullanıcıya hata olarak gösterilirse "bir şey bozuldu" sanır.
                    batch_bitir(
                        &arka,
                        &batch_id,
                        BatchStatus::Cancelled,
                        ImzaError::new(
                            ErrorCode::PinCancelled,
                            "kullanıcı PIN penceresini kapattı",
                        ),
                    );
                    return;
                }
                // **Süre dolması ile kanalın düşmesi AYNI ŞEY DEĞİL** ve tek dala
                //    yığılmışlardı. Süre dolduysa kullanıcı pencereyi hiç görmemiş
                //    olabilir (başka uygulamanın altında kaldı) — bu `Expired`. Kanal
                //    düştüyse pencere kapandı/çöktü — bu gerçek bir `Failed`. İkisini
                //    ayırmak, kullanıcıya ne yapacağını söyleyebilmek demek.
                Err(_) => {
                    arka.pin_bekleyeni_dusur(&batch_id);
                    batch_bitir(
                        &arka,
                        &batch_id,
                        BatchStatus::Expired,
                        ImzaError::new(
                            ErrorCode::PinCancelled,
                            "PIN penceresi süresinde yanıtlamadı — imza penceresini açık tutup yeniden deneyin",
                        ),
                    );
                    return;
                }
                Ok(Err(_)) => {
                    arka.pin_bekleyeni_dusur(&batch_id);
                    batch_dusur(
                        &arka,
                        &batch_id,
                        ImzaError::new(
                            ErrorCode::PinCancelled,
                            "PIN penceresi beklenmedik biçimde kapandı",
                        ),
                    );
                    return;
                }
            },
            // `CLIENT`: PIN gövdede geldi.
            None => match istemci_pin {
                Some(p) if p.chars().count() >= 4 => p,
                _ => {
                    batch_dusur(
                        &arka,
                        &batch_id,
                        ImzaError::new(ErrorCode::PinCancelled, "PIN girilmedi"),
                    );
                    return;
                }
            },
        };

        let Some((slot_id, certificate_id)) = secim else {
            batch_dusur(
                &arka,
                &batch_id,
                ImzaError::new(
                    ErrorCode::CertificateNotFound,
                    "imzalayacak kart ve sertifika seçilmedi",
                ),
            );
            return;
        };
        let calisma = crate::signing::SignRun {
            batch_id: batch_id.clone(),
            slot_id,
            certificate_id,
            pin,
        };
        // PKCS#11 bloke eder — async yürütücüde çağırmak WebSocket'leri dondurur.
        let sonuc = tokio::task::spawn_blocking({
            let arka = arka.clone();
            move || crate::signing::run(&arka, &modul, calisma)
        })
        .await;

        if let Ok(Err(e)) = sonuc {
            // Motor kendi içinde dosya hatalarını zaten yayınlıyor; buraya düşen
            //    şey akışın TAMAMINI durduran hatadır (kart yok, sertifika yok…).
            batch_dusur(&arka, &batch_id, e);
        }
    });

    Ok((StatusCode::ACCEPTED, Json(yanit(state.batch(&id)?))))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PinIstek {
    pin: Option<String>,
    /// Kullanıcı vazgeçti mi.
    #[serde(default)]
    cancelled: bool,
    /// Yerli pencerede seçilen yuva — verilirse `/sign`dakini EZER.
    slot_id: Option<u64>,
    /// Yerli pencerede seçilen sertifika — verilirse `/sign`dakini EZER.
    certificate_id: Option<String>,
}

// `Debug` KASTEN türetilmedi.
impl std::fmt::Debug for PinIstek {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinIstek")
            .field("pin", &"<gizlendi>")
            .field("cancelled", &self.cancelled)
            .field("slot_id", &self.slot_id)
            .field("certificate_id", &self.certificate_id)
            .finish()
    }
}

/// `POST /api/v1/signing/batches/{id}/pin` — **yerli agent PIN'i getirir**.
///
/// **TARAYICIDAN ÇAĞRILAMAZ.** İstek bir `Origin` başlığı taşıyorsa reddedilir:
/// tarayıcı `Origin`i **zorla** ekler ve saldırgan sayfa onu silemez, yani eksik
/// `Origin` isteğin tarayıcıdan gelmediğinin işaretidir. Bu kapı olmasaydı `native`
/// kip, `client` kipin daha dolambaçlı bir hâli olurdu — PIN yine tarayıcıdan gelirdi.
///
/// PIN hiçbir yere yazılmaz: gövdeden okunur, kanala girer, imzalama fonksiyonuna
/// geçer ve düşer.
async fn batch_pin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(istek): Json<PinIstek>,
) -> Sonuc<StatusCode> {
    if headers.contains_key(axum::http::header::ORIGIN) {
        // Kasten `403` ve kasten AÇIKLAMASIZ: bu uç tarayıcıya ait değil.
        return Err(ImzaError::new(
            ErrorCode::PinError,
            "PIN ucu tarayıcıdan çağrılamaz (yerli agent'a aittir)",
        )
        .into());
    }
    let cevap = if istek.cancelled {
        crate::state::PinCevabi::Iptal
    } else {
        match istek.pin {
            Some(p) if p.chars().count() >= 4 => crate::state::PinCevabi::Girildi {
                pin: p,
                // Seçim ancak İKİSİ birden gelirse geçerli: yalnız yuva ya da
                //    yalnız sertifika, yarım bir seçimdir ve yanlış anahtarla
                //    imzalamaya yol açardı.
                secim: match (istek.slot_id, istek.certificate_id) {
                    (Some(s), Some(c)) => Some(crate::state::CihazSecimi {
                        slot_id: s,
                        certificate_id: c,
                    }),
                    _ => None,
                },
            },
            _ => crate::state::PinCevabi::Iptal,
        }
    };
    state.pin_ver(&id, cevap)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/agent/heartbeat` — yerli agent "yaşıyorum" der.
///
/// `interactiveAgent` bu işaretten TÜREİR. Bir bayrak yerine damga
/// kullanılmasının sebebi: agent çökerse bayrak `true` kalır ve KentOS hiç
/// açılmayacak bir pencereyi beklemeye başlardı.
async fn agent_heartbeat(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Sonuc<StatusCode> {
    if headers.contains_key(axum::http::header::ORIGIN) {
        return Err(
            ImzaError::new(ErrorCode::ServiceError, "agent ucu tarayıcıdan çağrılamaz").into(),
        );
    }
    state.agent_isareti();
    Ok(StatusCode::NO_CONTENT)
}

/// Batch'i hata ile düşürür ve olayı yayınlar.
/// Batch'i bir UÇ duruma çeker, sebebi kaydeder ve olayı yayınlar.
///
/// **GEÇİŞİN SONUCU ARTIK ATILMIYOR — ve bu ölçülmüş bir üretim kusurunun
/// düzeltmesi (2026-08-31).** Eski hâli `let _ = state.transition(id, Failed)`
/// yazıyordu ve `Ready → Failed` tabloda YOKTU: kullanıcı PIN penceresini
/// kapattığında geçiş sessizce reddediliyor, batch sonsuza kadar `Ready` kalıyor,
/// istemci onu bitmiş saymadığı için beş dakika yokluyordu. Kullanıcının gördüğü
/// şey buydu: *"imza iptal edilince sistem kilitleniyor, sonsuza kadar imza atmaya
/// çalışıyor."*
///
/// **Uç durumu ÇAĞIRAN seçer** çünkü ayrım kullanıcıya görünür:
/// `Cancelled` = vazgeçti · `Expired` = süre doldu · `Failed` = gerçekten hata.
/// Üçünü `Failed`e yığmak, vazgeçen kullanıcıya "hata" raporlamaktı.
///
/// Reddedilen bir geçiş artık **loglanır ve zorlanır**: uç duruma çekilemeyen bir
/// batch, hiç kimsenin kurtaramayacağı bir batch'tir. Sessizce geçmek, kusurun
/// kendisini geri getirirdi.
fn batch_bitir(state: &AppState, batch_id: &str, hedef: BatchStatus, e: ImzaError) {
    let batch = state.batch(batch_id).ok().map(|g| g.batch);
    if let Err(hata) = state.set_last_error(batch_id, e.clone()) {
        warn!(batch = %batch_id, hata = %hata.technical_message, "son hata kaydedilemedi");
    }
    if let Err(hata) = state.transition(batch_id, hedef) {
        // Buraya düşmek bir TASARIM hatasıdır: geçiş tablosu bu yolu tanımıyor.
        //    Sessizce geçmek batch'i asılı bırakır, o yüzden GÜRÜLTÜLÜ ve ardından
        //    `Failed`e zorluyoruz — asılı kalmaktansa yanlış etiketli bitmek iyidir.
        warn!(
            batch = %batch_id, hedef = hedef.name(), hata = %hata.technical_message,
            "uç duruma geçilemedi — Failed'e zorlanıyor"
        );
        if let Err(h2) = state.transition(batch_id, BatchStatus::Failed) {
            error!(
                batch = %batch_id, hata = %h2.technical_message,
                "batch UÇ DURUMA ÇEKİLEMEDİ — istemci bunu asılı görecek"
            );
        }
    }
    let olay = match hedef {
        BatchStatus::Cancelled => "batch_cancelled",
        BatchStatus::Expired => "batch_expired",
        _ => "batch_failed",
    };
    state.emit(
        olay,
        batch.as_ref(),
        serde_json::json!({ "error": e, "signed": 0 }),
    );
}

/// Gerçek bir hata — `Failed`. İptal ve süre dolması için `batch_bitir` kullanın.
fn batch_dusur(state: &AppState, batch_id: &str, e: ImzaError) {
    batch_bitir(state, batch_id, BatchStatus::Failed, e);
}

/// `POST /api/v1/signing/batches/{id}/cancel`.
///
/// **İptal, ATILMIŞ imzaları geri almaz**. Kullanıcı üç dosyanın ikisini
/// imzalamışsa iptal yalnız kalanı durdurur; üretilmiş iki imza `result`ta durmaya
/// devam eder — bir imza atıldıktan sonra "atılmamış" sayılamaz.
async fn batch_iptal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sonuc<Json<BatchYanit>> {
    let girdi = state.batch(&id)?;
    if kentos_imza_domain::StateMachine::is_terminal(girdi.batch.status) {
        // Zaten bitmiş bir batch'i iptal etmek hata değil, sonuçsuzdur.
        return Ok(Json(yanit(girdi)));
    }
    // Bekleyen bir PIN isteği varsa düşür: pencere açık kalıp cevapsız beklemesin.
    state.pin_bekleyeni_dusur(&id);
    state.transition(&id, BatchStatus::Cancelled)?;
    let girdi = state.batch(&id)?;
    state.emit(
        "batch_cancelled",
        Some(&girdi.batch),
        serde_json::json!({ "signedSoFar": girdi.results.len() }),
    );
    Ok(Json(yanit(girdi)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeslimIstek {
    /// KentOS sonucu kabul etti mi.
    accepted: bool,
    /// Kabul edilmediyse sebebi — kullanıcıya gösterilir.
    reason: Option<String>,
}

/// `POST /api/v1/signing/batches/{id}/acknowledge` — **KentOS sunucusu teslim aldı**.
///
/// **Bu uç olmadan hiçbir batch `COMPLETED` olmaz** ve bu bilinçli: imza atılmış olması, sunucunun onu ALDIĞI anlamına
/// gelmez. Zincirin son adımları (sunucunun doğrulaması ve kaydı) sunucuda olur. Kendini tamamlanmış ilan eden bir
/// istemci, teslim edilememiş bir imzayı "bitti" gösterir — kullanıcı belgesini imzalanmış sanar, sunucuda ise hiçbir
/// kayıt yoktur.
///
/// **Bu uç sunucunun onayını TAŞIR, ÜRETMEZ.** Tarayıcı sonucu sunucuya yollar, sunucu doğrular ve tarayıcı burada
/// yalnız *"kabul edildi/edilmedi"* der. Servis sunucuya kendi bağlanmaz: yalnız geri döngü dinler, dışarı çağrı
/// yapmaz.
async fn batch_teslim(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(istek): Json<TeslimIstek>,
) -> Sonuc<Json<BatchYanit>> {
    let hedef = state.update_batch(&id, |r| r.acknowledge(istek.accepted))?;
    let girdi = state.batch(&id)?;
    let toplam = girdi.batch.signing_required_count();
    let imzali = girdi.results.len();
    let olay = match hedef {
        BatchStatus::Completed => "delivery_completed",
        // Kısmi başarı meşru bir sonuçtur — iki imza atıldıysa ikisi de sayılır.
        BatchStatus::PartiallyCompleted => "batch_partially_completed",
        _ => "delivery_failed",
    };
    state.emit(
        olay,
        Some(&girdi.batch),
        serde_json::json!({
            "accepted": istek.accepted,
            "reason": istek.reason,
            "signed": imzali,
            "total": toplam,
        }),
    );
    if hedef == BatchStatus::Completed {
        // **Yük BOŞ OLAMAZ — ölçülmüş kusur.** Sayılar yalnız bir ÖNCEKİ `delivery_completed` olayına konuyordu;
        //    `batch_completed` `{}` ile yayılıyordu ve onu dinleyen ajan bildirimde HER ZAMAN "0 belge" yazıyordu. Bir
        //    olayın "aynı bilgi bir öncekinde var" diye boş bırakılması, o olayı TEK BAŞINA dinleyen her tüketiciyi
        //    sessizce yanlış besler.
        state.emit(
            "batch_completed",
            Some(&girdi.batch),
            serde_json::json!({ "signed": imzali, "total": toplam }),
        );
    }
    Ok(Json(yanit(girdi)))
}

/// `GET /api/v1/signing/batches/{id}/result` — sunucuya teslim edilecek gövde ([`BatchResult`]): imzalar dosya
/// kimlikleriyle, imzalayan sertifika ve son hata. Masaüstü aynı gövdeyi süreç içinde kurar.
async fn batch_sonuc(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sonuc<Json<BatchResult>> {
    Ok(Json(state.batch(&id)?.result()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Doğrulama
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DogrulaIstek {
    /// İmzalanan özet — hex.
    digest: String,
    digest_algorithm: String,
    /// İmza baytları — base64.
    signature: String,
    /// İmzalayanın `SubjectPublicKeyInfo` DER'i — base64.
    public_key_der: String,
}

/// `POST /api/v1/verify`.
///
/// **Yöntemden bağımsız**: girdisi anahtar + özet + imza. Elektronik imza da mobil
/// imza da aynı kapıdan geçer.
async fn dogrula(
    Json(istek): Json<DogrulaIstek>,
) -> Sonuc<Json<kentos_imza_domain::VerificationResult>> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    let ozet = hex::decode(istek.digest.trim())
        .map_err(|e| ImzaError::new(ErrorCode::DocumentError, format!("özet hex değil: {e}")))?;
    let imza = BASE64.decode(istek.signature.trim()).map_err(|e| {
        ImzaError::new(
            ErrorCode::VerificationError,
            format!("imza base64 değil: {e}"),
        )
    })?;
    let spki = BASE64.decode(istek.public_key_der.trim()).map_err(|e| {
        ImzaError::new(
            ErrorCode::CertificateError,
            format!("SPKI base64 değil: {e}"),
        )
    })?;
    Ok(Json(kentos_imza_signature::verify::verify_raw(
        &spki,
        &ozet,
        &imza,
        &istek.digest_algorithm,
    )?))
}

// ─────────────────────────────────────────────────────────────────────────────
// WebSocket
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/events` — WebSocket olay akışı.
///
/// **Yalnız YAYIN yönü kullanılıyor**: istemciden gelen mesajlar yok sayılır.
/// Bir soketten komut kabul etmek, `POST` uçlarındaki bütün doğrulamayı (imzalı
/// istek, replay, durum makinesi) atlayan ikinci bir kapı açardı.
async fn events(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |soket| olay_akisi(soket, state))
}

async fn olay_akisi(mut soket: WebSocket, state: AppState) {
    let mut abone = state.subscribe();

    // Bağlanır bağlanmaz bir "merhaba": istemci soketin gerçekten açık olduğunu
    // ilk olayı beklemeden bilsin.
    let merhaba = serde_json::json!({
        "eventType": "service_started",
        "timestamp": now_rfc3339(),
        "payload": { "pinEntry": state.pin_entry(), "auditValid": state.audit_valid() },
    });
    if soket
        .send(Message::Text(merhaba.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    loop {
        match abone.recv().await {
            Ok(olay) => {
                let Ok(metin) = serde_json::to_string(&olay) else {
                    continue;
                };
                if soket.send(Message::Text(metin.into())).await.is_err() {
                    break;
                }
            }
            // **`Lagged` bağlantıyı KAPATMAZ.** Yavaş bir istemci olay kaçırır ve
            //    bunu BİLİR: kaçırdığını söyleyen bir olay yollanır, akış devam eder.
            //    Sessizce devam etmek, arayüzün eksik bir tabloyu tam sanmasıydı.
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                let uyari = serde_json::json!({
                    "eventType": "events_lagged",
                    "timestamp": now_rfc3339(),
                    "payload": { "missed": n },
                });
                if soket
                    .send(Message::Text(uyari.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Denetim
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/audit` — hash-zincirli denetim kaydı.
///
/// Vitrin ve tanılama için. Zincirin **kendi doğrulamasını** da döner: kayıtları
/// göstermek ama sağlamlığını göstermemek, okuyana kanıtlanmamış bir güven verirdi.
async fn audit(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "valid": state.audit_valid(),
        "events": state.audit_events(),
    }))
}

// ─────────────────────────────────────────────────────────────────────────────

fn yanit(girdi: BatchRecord) -> BatchYanit {
    let signing_required_count = girdi.batch.signing_required_count();
    BatchYanit {
        results: kentos_imza_domain::wire::file_signatures(&girdi.batch, &girdi.results),
        last_error: girdi.last_error,
        signing_required_count,
        batch: girdi.batch,
    }
}

fn gorev_hatasi(e: tokio::task::JoinError) -> ImzaError {
    ImzaError::new(
        ErrorCode::ServiceError,
        format!("kart işlemi tamamlanamadı: {e}"),
    )
}

/// İmza uçlarının yönlendiricisi.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/devices", get(devices))
        .route("/api/v1/certificates", get(certificates))
        .route("/api/v1/devices/pin-check", post(pin_dogrula))
        .route("/api/v1/signing/batches", post(batch_olustur))
        .route("/api/v1/signing/batches/{id}", get(batch_getir))
        .route("/api/v1/signing/batches/{id}/open", post(batch_ac))
        .route("/api/v1/signing/batches/{id}/sign", post(batch_imzala))
        .route("/api/v1/signing/batches/{id}/cancel", post(batch_iptal))
        .route("/api/v1/signing/batches/{id}/pin", post(batch_pin))
        .route("/api/v1/agent/heartbeat", post(agent_heartbeat))
        .route(
            "/api/v1/signing/batches/{id}/acknowledge",
            post(batch_teslim),
        )
        .route("/api/v1/signing/batches/{id}/result", get(batch_sonuc))
        .route("/api/v1/verify", post(dogrula))
        .route("/api/v1/events", get(events))
        .route("/api/v1/audit", get(audit))
        .with_state(state)
}
