//! KentOS İmza servisi (docs/adr/9002 §4): portalın tarayıcıdan geldiği yolun yerel köprüsü.
//!
//! Bu ikili çekirdek servistir: yalnız geri döngüde dinleyen API'yi ayağa kaldırır. PIN'i **ajanın penceresi** alır
//! (`kentos-imza-agent`, ayrı süreç): **servis pencere AÇMAZ**. Bu bir üslup tercihi değil: Windows'ta bir servis
//! kullanıcı oturumunda değildir ve açtığı pencere görünmez; kullanıcı imza beklerken hiçbir şey olmaz.
//!
//! Yapılandırma ortamdan ve yapılandırma dosyasından okunur (`config_file`); ortam her zaman kazanır.

use std::{process::ExitCode, sync::Arc};

use kentos_imza_api::{AppState, DEFAULT_PORT, LoopbackAddr, OriginAllowlist, PinEntryMode};
use kentos_imza_signature::{
    MockElectronicSignatureProvider, MockMobileSignatureProvider, SignatureProviderRegistry, Token,
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod config_file;

/// Port (varsayılan 32147).
const PORT_ENV: &str = "KENTOS_IMZA_PORT";
/// PKCS#11 kütüphanesinin **mutlak** yolu. Verilmezse sistem dizinlerindeki bilinen sürücüler denenir; hiçbiri yoksa
/// servis yine kalkar ama kart uçları `KSI-1100` döner: kartı olmayan bir makinede `/health` ve `/info`nun çalışması,
/// kurulumu teşhis etmenin tek yolu.
const PKCS11_ENV: &str = "KENTOS_IMZA_PKCS11";
/// PIN giriş kipi: `native` (varsayılan) ya da `client` (geliştirme; PIN tarayıcıdan geçer).
const PIN_ENTRY_ENV: &str = "KENTOS_IMZA_PIN_ENTRY";
/// KentOS sunucusunun Ed25519 **açık** anahtarı (standart base64, 32 bayt).
const TRUST_KEY_ENV: &str = "KENTOS_IMZA_TRUST_KEY";
const TRUST_ISSUER_ENV: &str = "KENTOS_IMZA_TRUST_ISSUER";
const TRUST_AUDIENCE_ENV: &str = "KENTOS_IMZA_TRUST_AUDIENCE";
/// İzinli tarayıcı kaynakları — virgülle ayrılmış, şema + ad + port tam eşleşir.
const ORIGINS_ENV: &str = "KENTOS_IMZA_ORIGINS";
/// Deneme kartı: yalnız `mock-token` özelliğiyle derlenmiş ikilide; `1` yazılınca gerçek kart yerine yazılım kartı.
const MOCK_ENV: &str = "KENTOS_IMZA_DENEME_KARTI";

/// Geliştirme kaynakları: portalın yerel adresleri. Kurumun gerçek adresi `KENTOS_IMZA_ORIGINS` ile verilir; buraya
/// yazılmaz, çünkü her kurum başka bir alan adında çalışır.
const VARSAYILAN_ORIGINS: &str =
    "http://localhost:8797,http://127.0.0.1:8797,http://localhost:5173,http://127.0.0.1:5173";

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Yapılandırma dosyası, ortam okumalarından ÖNCE yüklenir: servis bir servis olarak başlatıldığında (systemd
    //    kullanıcı birimi, oturum açılışı) kabuğun ortamı yoktur ve dosya olmadan güven anahtarı gelmez.
    let ayarlar = config_file::Ayarlar::yukle();
    if let Some(yol) = ayarlar.yol() {
        info!(dosya = %yol.display(), "yapılandırma dosyası okundu");
    }

    let port = match ayarlar.var(PORT_ENV) {
        Ok(v) => match v.parse::<u16>() {
            Ok(p) => p,
            Err(e) => {
                // Bozuk porta SESSİZCE varsayılanla devam EDİLMEZ: yönetici servisin yazdığı portta olduğunu sanır.
                error!(deger = %v, hata = %e, "{PORT_ENV} sayı değil — servis başlatılmadı");
                return ExitCode::FAILURE;
            }
        },
        Err(_) => DEFAULT_PORT,
    };

    // ── PIN kipi ─────────────────────────────────────────────────────────────
    //
    // Varsayılan `native`: ajan servisle birlikte kurulur, PIN hiç tarayıcıdan geçmez (docs/adr/9002 §4). `client`
    //    kipi geliştirme içindir; sapma gizlenmez: `/info` `insecurePinEntry: true` döner ve açılışta uyarılır.
    let pin_entry = match ayarlar.var(PIN_ENTRY_ENV).as_deref() {
        Ok("native") | Err(_) => {
            info!("PIN giriş kipi NATIVE — PIN ajanın penceresinden alınacak");
            PinEntryMode::Native
        }
        Ok("client") => {
            warn!(
                "PIN giriş kipi CLIENT — PIN tarayıcıdan geçecek. Yalnız geliştirme içindir; \
                 kurumsal kurulumda KENTOS_IMZA_PIN_ENTRY'yi kaldırın."
            );
            PinEntryMode::Client
        }
        Ok(other) => {
            error!(deger = %other, "{PIN_ENTRY_ENV} 'native' ya da 'client' olmalı");
            return ExitCode::FAILURE;
        }
    };

    let token = match kart(&ayarlar) {
        Ok(t) => t,
        Err(kod) => return kod,
    };

    // ── Güven anahtarı ───────────────────────────────────────────────────────
    let trust = match ayarlar.var(TRUST_KEY_ENV) {
        Ok(b64) => {
            let issuer = ayarlar
                .var(TRUST_ISSUER_ENV)
                .unwrap_or_else(|_| kentos_imza_security::DEFAULT_ISSUER.into());
            let audience = ayarlar
                .var(TRUST_AUDIENCE_ENV)
                .unwrap_or_else(|_| kentos_imza_security::DEFAULT_AUDIENCE.into());
            match kentos_imza_security::TrustConfig::from_base64(&b64, issuer, audience) {
                Ok(t) => {
                    info!("KentOS sunucusunun güven anahtarı yüklendi");
                    Some(t)
                }
                Err(e) => {
                    // Bozuk anahtarla DEVAM EDİLMEZ: yönetici korumanın açık olduğunu sanırdı.
                    error!(hata = %e.technical_message, "{TRUST_KEY_ENV} çözülemedi");
                    return ExitCode::FAILURE;
                }
            }
        }
        Err(_) => {
            warn!(
                "{TRUST_KEY_ENV} verilmedi — imzalı istek doğrulanamaz; batch oluşturma reddedilecek"
            );
            None
        }
    };

    // Mobil imza defterde VAR ama KAPALI — `resolve` onu `KSI-2001` ile reddeder. Hiç kaydetmemek `KSI-2002`
    //    ("yok") derdi ve bu yanlış olurdu: yöntem var, bu sürümde açık değil.
    let providers = SignatureProviderRegistry::new()
        .with(Box::new(MockElectronicSignatureProvider))
        .with(Box::new(MockMobileSignatureProvider::disabled()));

    let state = AppState::new(providers, pin_entry, trust, token);
    let origins = OriginAllowlist::parse(
        &ayarlar
            .var(ORIGINS_ENV)
            .unwrap_or_else(|_| VARSAYILAN_ORIGINS.to_owned()),
    );

    // Adres TİP tarafından geri döngüye kilitli: `0.0.0.0` yazmanın yolu yok.
    let addr = LoopbackAddr::ipv4(port);
    info!(%addr, surum = env!("CARGO_PKG_VERSION"), "KentOS İmza başlatılıyor");

    match kentos_imza_api::serve(addr, state, origins).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!(%addr, hata = %e, "servis dinlemeye başlayamadı");
            ExitCode::FAILURE
        }
    }
}

/// Kart katmanı: deneme kartı istendiyse (ve ikili onunla derlendiyse) yazılım kartı, yoksa PKCS#11.
fn kart(ayarlar: &config_file::Ayarlar) -> Result<Option<Arc<dyn Token>>, ExitCode> {
    if ayarlar.var(MOCK_ENV).as_deref() == Ok("1") {
        #[cfg(feature = "mock-token")]
        {
            warn!(
                "DENEME KARTI açık: imzalar yazılım kartıyla atılır ve hukukî değeri YOKTUR (PIN {}).",
                kentos_imza_signature::mock_token::MOCK_PIN
            );
            let kart = kentos_imza_signature::mock_token::MockToken::new()
                .with_delay(std::time::Duration::from_millis(700));
            return Ok(Some(Arc::new(kart)));
        }
        #[cfg(not(feature = "mock-token"))]
        {
            error!(
                "{MOCK_ENV}=1 verildi ama bu ikili deneme kartıyla derlenmedi (mock-token özelliği)"
            );
            return Err(ExitCode::FAILURE);
        }
    }
    let yol = match ayarlar.var(PKCS11_ENV) {
        Ok(y) => std::path::PathBuf::from(y),
        Err(_) => match kentos_imza_pkcs11::well_known_libraries()
            .into_iter()
            .next()
        {
            Some(y) => {
                info!(yol = %y.display(), "{PKCS11_ENV} verilmedi; sistemdeki sürücü kullanılıyor");
                y
            }
            None => {
                info!(
                    "{PKCS11_ENV} verilmedi ve sistemde bilinen bir sürücü yok — kart uçları kapalı"
                );
                return Ok(None);
            }
        },
    };
    match kentos_imza_pkcs11::ProviderConfig::new("varsayilan", yol)
        .and_then(kentos_imza_pkcs11::Pkcs11Module::load)
    {
        Ok(m) => {
            info!("PKCS#11 modülü yüklendi");
            Ok(Some(Arc::new(m)))
        }
        Err(e) => {
            // Servis DÜŞMEZ ama sebep açıkça yazılır: kullanıcı "imza servisi bozuk" yerine "sürücü yüklenemedi"yi
            //    görmeli.
            error!(kod = %e.code, hata = %e.technical_message, "PKCS#11 yüklenemedi");
            Ok(None)
        }
    }
}
