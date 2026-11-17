//! Sağlık ve tanılama uçları: `/api/v1/health`, `/api/v1/info` (docs/adr/9002 §4). İmza uçları `signing_routes`'tadır.
//!
//! **Sürüm öneki `/api/v1` ZORUNLU** ve sürümsüz kardeşi YOK: servis yeni olduğu için eski kurulu istemcilerin borcunu
//! hiç üstlenmiyor.

use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

use crate::state::{AppState, PinEntryMode};

/// `/api/v1/health` yanıtı.
///
/// **Hassas veri KOYULMAZ**. Bu uç kimlik doğrulaması olmadan
/// çağrılabiliyor; sertifika sahibinin adı, kart seri numarası, dosya yolu ya da
/// yapılandırma ayrıntısı buraya konursa, tarayıcıdaki herhangi bir sayfa onu okur.
/// Burada duran her alan **kasten** seçildi ve yenisi eklenirken aynı soru sorulur:
/// *"bunu kötü niyetli bir site öğrenirse ne olur?"*
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub platform: &'static str,
    pub architecture: &'static str,
    /// Ajan (PIN penceresi) bağlı mı: son kalp atışından türer (15 sn tazelik).
    ///
    /// Portal buna bakıp "imza penceresi açılabilir" kararı verir; yanlış bir `true`, kullanıcıyı hiç açılmayacak bir
    /// pencereyi beklemeye bırakırdı.
    pub interactive_agent: bool,
    /// Bulunan cihaz sayısı — karta **gerçekten sorulur**.
    ///
    /// Kart yoksa ya da PKCS#11 yapılandırılmamışsa `0`. Sayı bir **iddiadır**:
    /// arayüz buna bakıp "kartınızı takın" diyor, yani uydurulmuş bir `1`, kullanıcıyı
    /// olmayan bir kartla imza atmaya çalıştırırdı.
    pub devices: u32,
}

/// `/api/v1/info` yanıtı — tanılama için.
///
/// Hassas veri gösterilmez; bu yüzden PKCS#11 kütüphane YOLU
/// burada YOK. Yol, kurulumun dosya sisteminden bir parça sızdırır ve saldırgana
/// hangi sürücünün kurulu olduğunu söyler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub service: &'static str,
    pub version: &'static str,
    pub api_version: &'static str,
    pub platform: &'static str,
    pub architecture: &'static str,
    /// Açık imza yöntemleri — arayüzün yöntem seçicisini çizmesi için.
    pub enabled_signature_methods: Vec<String>,

    /// PIN nereden alınıyor (docs/adr/9002 §4).
    pub pin_entry: PinEntryMode,

    /// **`true` ise PIN kuralından SAPILMIŞ demektir**: PIN tarayıcıdan geçiyor.
    ///
    /// Bu alan gizlenemez ve arayüz onu kullanıcıya **göstermek zorunda**. Sessiz bir
    /// sapma, sapma değil kusurdur.
    pub insecure_pin_entry: bool,

    /// KentOS sunucusunun açık anahtarı yüklü mü.
    ///
    /// `false` ise batch oluşturulamaz — imzalı istek doğrulanamıyor demektir.
    /// Arayüz bunu bir kurulum eksiği olarak gösterir, "sunucu bozuk" olarak değil.
    pub trust_configured: bool,

    /// Kart katmanı yüklendi mi. **Yol YOK, yalnız evet/hayır**.
    pub pkcs11_loaded: bool,
    /// **Yazılım kartı mı** (deneme oturumu). `true` ise üretilen imzaların hukukî değeri YOKTUR ve portal bunu
    /// kullanıcıya göstermek zorundadır.
    pub mock_token: bool,

    /// Denetim zinciri sağlam mı.
    pub audit_valid: bool,
}

const SERVICE_NAME: &str = "kentos-imza";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const API_VERSION: &str = "v1";

/// Derleme hedefinin işletim sistemi.
///
/// Çalışma zamanında tespit edilmez, **derleme zamanından** gelir: bir ikili tek
/// bir hedef için derlenir ve `std::env::consts` tam olarak onu söyler.
const PLATFORM: &str = std::env::consts::OS;
const ARCH: &str = std::env::consts::ARCH;

/// `/api/v1/health`.
async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: SERVICE_NAME,
        version: VERSION,
        platform: PLATFORM,
        architecture: ARCH,
        // **Damgadan türer, bayraktan DEĞİL**: agent çökerse bayrak `true`
        //    kalırdı ve KentOS hiç açılmayacak bir pencereyi beklettirirdi.
        interactive_agent: state.agent_bagli(),
        devices: cihaz_sayisi(&state).await,
    })
}

/// Karta sorup cihaz sayar — **hata yutulur**, sağlık ucu kart yüzünden düşmez.
async fn cihaz_sayisi(state: &AppState) -> u32 {
    let Some(m) = state.token().cloned() else {
        return 0;
    };
    // PKCS#11 bloke eder; async iş parçacığında çağırmak WebSocket'leri dondurur.
    tokio::task::spawn_blocking(move || m.devices().map(|d| d.len()).unwrap_or(0))
        .await
        .unwrap_or(0)
        .try_into()
        .unwrap_or(u32::MAX)
}

/// `/api/v1/info`.
async fn info(State(state): State<AppState>) -> Json<InfoResponse> {
    Json(InfoResponse {
        service: SERVICE_NAME,
        version: VERSION,
        api_version: API_VERSION,
        platform: PLATFORM,
        architecture: ARCH,
        // Liste sağlayıcı defterinden gelir. Mobil imza kapalıysa BURADA GÖRÜNMEZ
        // — seçilemeyen bir seçenek göstermek, kullanıcıyı hataya davet etmektir.
        enabled_signature_methods: state
            .providers()
            .enabled_methods()
            .iter()
            .map(|m| {
                serde_json::to_value(m)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default()
            })
            .collect(),
        pin_entry: state.pin_entry(),
        insecure_pin_entry: state.pin_entry() == PinEntryMode::Client,
        trust_configured: state.trust_configured(),
        pkcs11_loaded: state.token().is_some(),
        mock_token: state.token().is_some_and(|t| t.is_mock()),
        audit_valid: state.audit_valid(),
    })
}

/// Uygulamanın yönlendiricisi.
///
/// `#[must_use]` YAZILMAZ: `Router` zaten öyle işaretli ve clippy tekrarını
///    hata sayıyor.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/info", get(info))
        .with_state(state.clone())
        .merge(crate::signing_routes::router(state))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    fn durum() -> AppState {
        AppState::new(
            kentos_imza_signature::SignatureProviderRegistry::new().with(Box::new(
                kentos_imza_signature::MockElectronicSignatureProvider,
            )),
            PinEntryMode::Native,
            None,
            None,
        )
    }

    #[tokio::test]
    async fn saglik_yaniti_98_ile_ayni_sekilde() {
        let Json(h) = health(State(durum())).await;
        assert_eq!(h.status, "ok");
        assert_eq!(h.service, "kentos-imza");
        let j = serde_json::to_value(&h).unwrap();
        // Sağlık yanıtının anahtarları — camelCase.
        for anahtar in [
            "status",
            "service",
            "version",
            "platform",
            "architecture",
            "interactiveAgent",
            "devices",
        ] {
            assert!(j.get(anahtar).is_some(), "{anahtar} alanı yok");
        }
    }

    #[test]
    /// Sağlık yanıtı **hassas veri taşımaz**.
    fn saglik_yaniti_hassas_veri_tasimaz() {
        // Hassas veri konmaz: bu uç kimlik doğrulaması olmadan
        //    çağrılabiliyor; tarayıcıdaki herhangi bir sayfa yanıtı okuyabilir.
        //    Alan kümesi bu yüzden KAPALI bir listedir — yeni alan eklemek bu testi
        //    kırar ve ekleyen kişi "bunu kötü niyetli bir site öğrenirse ne olur?"
        //    sorusunu cevaplamak zorunda kalır.
        let h = HealthResponse {
            status: "ok",
            service: SERVICE_NAME,
            version: VERSION,
            platform: PLATFORM,
            architecture: ARCH,
            interactive_agent: false,
            devices: 0,
        };
        let j = serde_json::to_value(&h).unwrap();
        let alanlar: Vec<&String> = j.as_object().unwrap().keys().collect();
        assert_eq!(
            alanlar.len(),
            7,
            "sağlık yanıtına alan EKLENDİ: {alanlar:?}"
        );

        let metin = serde_json::to_string(&h).unwrap().to_lowercase();
        for yasak in [
            "pin",
            "certificate",
            "sertifika",
            "pkcs11",
            "path",
            "token",
            "serial",
        ] {
            assert!(
                !metin.contains(yasak),
                "sağlık yanıtında yasak alan: {yasak}"
            );
        }
    }

    #[tokio::test]
    /// `info` PKCS#11 **yolunu sızdırmaz**.
    async fn info_pkcs11_yolunu_sizdirmaz() {
        // Hassas veri gösterilmez: kütüphane yolu, kurulumun dosya
        //    sisteminden parça sızdırır ve hangi sürücünün kurulu olduğunu söyler.
        let Json(i) = info(State(durum())).await;
        let metin = serde_json::to_string(&i).unwrap();
        assert!(!metin.contains(".so") && !metin.contains(".dll") && !metin.contains('/'));
    }

    #[tokio::test]
    /// Mobil imza açık yöntemler arasında **yok**.
    async fn mobil_imza_acik_yontemlerde_yok() {
        // Mobil imza varsayılan olarak kapalı. Arayüz yöntem seçicisini
        // buradan çiziyor; listede görünmesi kullanıcıya seçilemeyen bir seçenek
        // göstermek olurdu.
        let Json(i) = info(State(durum())).await;
        assert_eq!(i.enabled_signature_methods, vec!["ELECTRONIC_SIGNATURE"]);
    }

    #[tokio::test]
    /// **`native` kipte `insecurePinEntry` FALSE** — ve `client` kipte TRUE.
    ///
    /// "Gizlenen bir sapma, sapma değil kusurdur" kuralının koddaki karşılığı: sapmayı **yanıtın kendisi** bildiriyor.
    async fn client_kipi_kendini_bildirir() {
        let Json(i) = info(State(durum())).await;
        assert!(!i.insecure_pin_entry);
        assert_eq!(i.pin_entry, PinEntryMode::Native);

        let acik = AppState::new(
            kentos_imza_signature::SignatureProviderRegistry::new(),
            PinEntryMode::Client,
            None,
            None,
        );
        let Json(i) = info(State(acik)).await;
        assert!(i.insecure_pin_entry, "client kipi kendini SÖYLEMELİ");
    }
}
