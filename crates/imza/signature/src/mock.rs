//! Defterin mock sağlayıcıları: yöntemi ve yetenekleri tanıtırlar, **imza ÜRETMEZLER** (docs/adr/9002 §3).
//!
//! Bu sağlayıcılar akışı, durum makinesini ve arayüzü kartsız sınamak içindir. Bir mock'un ürettiği baytın hiçbir
//! hukukî değeri yoktur ve o baytın gerçek bir imza sanılması en tehlikeli kusur olurdu; bu yüzden defterin
//! sağlayıcılarında imzalama yüzeyi yoktur. İmzalama kart soyutlamasından geçer ([`crate::token`]); onun yazılım
//! taklidi ayrı bir modüldür ve yalnız `mock-token` özelliğiyle derlenir ([`crate::mock_token`]).
//!
//! Mock'lar `cfg(test)` ARKASINDA DEĞİL, sürüm derlemesine de giriyor: güvenlik **kayıt defterinden** gelir, mock
//! elektronik sağlayıcı ancak yapılandırma onu deftere koyarsa devreye girer.

use kentos_imza_domain::SignatureMethod;

use crate::provider::{ProviderState, SignatureCapabilities, SignatureProvider};

/// Kartsız elektronik imza taklidi.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockElectronicSignatureProvider;

impl SignatureProvider for MockElectronicSignatureProvider {
    fn method(&self) -> SignatureMethod {
        SignatureMethod::ElectronicSignature
    }

    fn capabilities(&self) -> SignatureCapabilities {
        // Yetenekler GERÇEĞİ taklit eder: mock, gerçek kartla aynı akışı
        //    üretmezse kartsız doğrulanan akış üretimde başka türlü çalışır.
        SignatureCapabilities {
            requires_local_agent: true,
            requires_physical_token: true,
            requires_pin: true,
            supports_batch: true,
            supports_multiple_files: true,
            supports_offline: true,
            supports_certificate_selection: true,
            interactive: true,
        }
    }

    fn state(&self) -> ProviderState {
        ProviderState::Enabled
    }

    fn name(&self) -> &'static str {
        "MockElectronicSignatureProvider"
    }
}

/// Mobil imza taklidi.
///
/// **Varsayılan durumu `Disabled`**. Bu, özellik bayrağının **sunucu tarafındaki**
/// karşılığıdır: arayüzün bayrağı tek güvenlik mekanizması olamaz; arayüz düğmeyi gizlese bile defter
/// isteği reddetmek zorunda.
#[derive(Debug, Clone, Copy)]
pub struct MockMobileSignatureProvider {
    state: ProviderState,
}

impl Default for MockMobileSignatureProvider {
    fn default() -> Self {
        Self {
            state: ProviderState::Disabled,
        }
    }
}

impl MockMobileSignatureProvider {
    /// **Varsayılan**: yöntem defterde VAR ama KAPALI.
    ///
    /// Hiç kaydetmemekle aynı şey DEĞİL: kayıtsız yöntem `KSI-2002` ("sağlayıcı
    /// yok") döner, kapalı yöntem `KSI-2001` ("bu sürümde desteklenmiyor"). İkisi
    /// destek ekibini farklı yerlere gönderir.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            state: ProviderState::Disabled,
        }
    }

    /// Yalnız geliştirme için açık örnek.
    ///
    /// Adı açıkça `enabled_for_development`: `new(true)` gibi bir imza, çağrı
    /// yerine bakan birine bunun sıradan bir seçenek olduğunu düşündürürdü.
    #[must_use]
    pub const fn enabled_for_development() -> Self {
        Self {
            state: ProviderState::Enabled,
        }
    }
}

impl SignatureProvider for MockMobileSignatureProvider {
    fn method(&self) -> SignatureMethod {
        SignatureMethod::MobileSignature
    }

    fn capabilities(&self) -> SignatureCapabilities {
        // `requires_local_agent: false` — mimarî şart. Mobil imza yerel
        //    agent'a bağımlı OLMAMALI; bu alan `true` olsaydı akış, telefonuyla
        //    imzalayan bir kullanıcıdan da agent kurmasını isterdi.
        SignatureCapabilities {
            requires_local_agent: false,
            requires_physical_token: false,
            requires_pin: false,
            supports_batch: true,
            supports_multiple_files: true,
            // Operatöre ulaşmak gerekir — çevrimdışı çalışmaz.
            supports_offline: false,
            // Sertifika operatörde; kullanıcı seçmez.
            supports_certificate_selection: false,
            interactive: true,
        }
    }

    fn state(&self) -> ProviderState {
        self.state
    }

    fn name(&self) -> &'static str {
        "MockMobileSignatureProvider"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use kentos_imza_core::ErrorCode;

    use super::*;
    use crate::provider::SignatureProviderRegistry;

    fn varsayilan_defter() -> SignatureProviderRegistry {
        SignatureProviderRegistry::new()
            .with(Box::new(MockElectronicSignatureProvider))
            .with(Box::new(MockMobileSignatureProvider::default()))
    }

    #[test]
    /// Elektronik imza AÇIK, mobil **kapalı** — defterin varsayılanı.
    fn elektronik_acik_mobil_kapali() {
        let d = varsayilan_defter();
        assert_eq!(
            d.enabled_methods(),
            vec![SignatureMethod::ElectronicSignature]
        );
    }

    #[test]
    /// Kapalı yöntemde **sessiz fallback YOK** — `KSI-2001`.
    fn mobil_kapaliyken_fallback_yok_ksi_2001() {
        // Mobil imza kapalıyken gelen istek başka bir yönteme sessizce
        //    çevrilmez. Sessiz çevirme, kullanıcının SEÇTİĞİNDEN başka bir yöntemle
        //    imza atılması demektir — imzanın hukukî dayanağı yöntemin kendisidir.
        let d = varsayilan_defter();
        let hata = d.resolve(SignatureMethod::MobileSignature).unwrap_err();
        assert_eq!(hata.code, "KSI-2001");
        assert!(!hata.retryable, "kapalı yöntem tekrar denenerek açılmaz");

        // Ve elektronik imzaya DÜŞMEDİ — defter hâlâ ikisini ayırt ediyor.
        assert!(d.resolve(SignatureMethod::ElectronicSignature).is_ok());
    }

    #[test]
    /// Hiç kayıtlı olmayan yöntem `KSI-2002` döner — `2001` **değil**.
    fn kayitsiz_yontem_2002_doner() {
        // "kapalı" ile "yok" AYRI kodlar: ilkini yönetici açabilir, ikincisi bu
        //    sürümde uygulanmamıştır. Tek koda indirmek destek ekibine yanlış yerde
        //    arattırırdı.
        let d = SignatureProviderRegistry::new().with(Box::new(MockElectronicSignatureProvider));
        let hata = d.resolve(SignatureMethod::MobileSignature).unwrap_err();
        assert_eq!(hata.code, ErrorCode::SignatureProviderNotAvailable.code());
        assert_eq!(hata.code, "KSI-2002");
    }

    #[test]
    /// Mobil imza yerel agent **istemez** — mimarî şart.
    fn mobil_yerel_agent_istemez() {
        // Mobil imza yerel KentOS İmza agent'ına bağımlı tasarlanmaz; bu, mimarîde
        //    tek satırla bozulabilecek bir kural.
        let m = MockMobileSignatureProvider::default().capabilities();
        assert!(!m.requires_local_agent);
        assert!(!m.requires_physical_token);
        assert!(!m.requires_pin);
        assert!(
            !m.supports_offline,
            "operatöre ulaşmadan mobil imza atılamaz"
        );

        let e = MockElectronicSignatureProvider.capabilities();
        assert!(e.requires_local_agent);
        assert!(e.requires_physical_token);
        assert!(e.requires_pin);
        assert!(e.supports_offline, "kart yereldedir");
    }

    #[test]
    fn gelistirme_kipinde_mobil_acilabilir() {
        // Geliştirmede mock mobil sağlayıcı açılabilir.
        let d = SignatureProviderRegistry::new().with(Box::new(
            MockMobileSignatureProvider::enabled_for_development(),
        ));
        assert!(d.resolve(SignatureMethod::MobileSignature).is_ok());
        assert_eq!(d.enabled_methods(), vec![SignatureMethod::MobileSignature]);
    }

    #[test]
    fn her_saglayici_etkilesimli_hicbir_imza_sessiz_tamamlanmaz() {
        // Hiçbir imza isteği kullanıcı etkileşimi olmadan tamamlanmaz.
        for c in [
            MockElectronicSignatureProvider.capabilities(),
            MockMobileSignatureProvider::default().capabilities(),
        ] {
            assert!(c.interactive);
        }
    }
}
