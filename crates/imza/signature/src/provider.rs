//! Sağlayıcı soyutlaması (docs/adr/9002 §3).
//!
//! **Bu soyutlamanın varlık sebebi mobil imzadır**: bugün yazılan alan modeli ve tel sözleşmesi, mobil imza
//! eklendiğinde kırılmak zorunda kalmamalı. Bugün gerçek olan tek yöntem elektronik imza; ama akış bir PKCS#11
//! sağlayıcısına değil bu `trait`e bakarsa, mobil eklendiğinde akış hiç değişmez.
//!
//! **Yetenekler (`SignatureCapabilities`) sabit DEĞİL, sağlayıcı başına**:
//! elektronik imza yerel agent + fiziksel kart + PIN ister, mobil imza hiçbirini
//! istemez. Akış "PIN sor" adımını yönteme göre değil **yeteneğe** göre kurar; aksi
//! hâlde her yeni yöntem akışa bir `if` daha eklerdi.

use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::SignatureMethod;
use serde::{Deserialize, Serialize};

/// Bir sağlayıcının neyi yapıp neyi yapamadığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureCapabilities {
    /// Kullanıcının bilgisayarında yerel bir agent şart mı.
    ///
    /// Mobil imzada **`false`** olmak zorunda: *"Mobile Signature local
    /// KentOS İmza agent'a bağımlı tasarlanmamalıdır."* Bu alanı `true` sabitlemek,
    /// tam olarak o bağımlılığı mimariye gömmek olurdu.
    pub requires_local_agent: bool,
    /// Fiziksel kart/token gerekiyor mu.
    pub requires_physical_token: bool,
    /// PIN sorulacak mı.
    pub requires_pin: bool,

    /// Tek oturumda birden çok dosya imzalanabilir mi.
    pub supports_batch: bool,
    pub supports_multiple_files: bool,

    /// İnternet olmadan imza üretilebilir mi.
    ///
    /// Elektronik imzada `true`: kart yerelde. Mobil imzada `false` — operatöre
    /// ulaşmak gerekir. Akış, çevrimdışıyken hangi yöntemi sunacağını buradan bilir.
    pub supports_offline: bool,

    /// Kullanıcı sertifika seçebilir mi (takılı birden fazla kart ya da sertifika olabilir).
    pub supports_certificate_selection: bool,

    /// Kullanıcı etkileşimi gerektiriyor mu.
    pub interactive: bool,
}

/// Sağlayıcının o anki durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderState {
    Enabled,
    /// Özellik bayrağıyla kapalı ya da bu sürümde gerçek uygulaması yok.
    Disabled,
}

/// **İmza sağlayıcısı**.
///
/// Defterdeki yöntemin **tanıma** yüzeyi (`method` · `capabilities` · `state`). Kartla imzalama bu trait'te değil,
/// kart soyutlamasındadır ([`crate::token`]): boş bir `sign()` YAZILMADI, çünkü `unimplemented!()` taşıyan bir
/// metot, dosya ağacına bakan birine mobil imzanın hazır olduğunu düşündürürdü.
pub trait SignatureProvider: std::fmt::Debug + Send + Sync {
    /// Hangi yöntemi uyguluyor.
    fn method(&self) -> SignatureMethod;

    /// Yetenekleri.
    fn capabilities(&self) -> SignatureCapabilities;

    /// Bu sağlayıcı şu anda kullanılabilir mi.
    fn state(&self) -> ProviderState;

    /// İnsan tarafından okunabilir ad (tanılama ekranında görünür).
    fn name(&self) -> &'static str;
}

/// **Sağlayıcı defteri**.
///
/// **Kapalı bir yöntem için SESSİZ FALLBACK YAPILMAZ**: mobil imza kapalıyken
/// gelen bir istek elektronik imzaya çevrilmez, `KSI-2001` ile reddedilir. Sessiz
/// çevirme, kullanıcının seçtiğinden BAŞKA bir yöntemle imza atılması demektir — ve
/// imzanın hukukî dayanağı yöntemin kendisidir.
#[derive(Debug, Default)]
pub struct SignatureProviderRegistry {
    providers: Vec<Box<dyn SignatureProvider>>,
}

impl SignatureProviderRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Deftere bir sağlayıcı ekler.
    #[must_use]
    pub fn with(mut self, provider: Box<dyn SignatureProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// Yönteme göre sağlayıcı çözer.
    ///
    /// # Errors
    /// * Yöntem için hiç sağlayıcı yoksa → `KSI-2002` `SIGNATURE_PROVIDER_NOT_AVAILABLE`
    /// * Sağlayıcı var ama **kapalıysa** → `KSI-2001` `SIGNATURE_METHOD_NOT_SUPPORTED`
    ///
    /// İki hâl AYRI kod döndürür ve bu bilinçli: *"kapalı"* bir yapılandırma
    /// kararıdır (yönetici açabilir), *"yok"* ise bu sürümde uygulanmamıştır. Tek
    /// koda indirmek, destek ekibine yanlış yerde arattırırdı.
    pub fn resolve(&self, method: SignatureMethod) -> Result<&dyn SignatureProvider, ImzaError> {
        let Some(p) = self.providers.iter().find(|p| p.method() == method) else {
            return Err(ImzaError::new(
                ErrorCode::SignatureProviderNotAvailable,
                format!("no provider registered for {method:?}"),
            ));
        };
        match p.state() {
            ProviderState::Enabled => Ok(p.as_ref()),
            ProviderState::Disabled => Err(ImzaError::new(
                ErrorCode::SignatureMethodNotSupported,
                format!("provider {} is disabled for {method:?}", p.name()),
            )),
        }
    }

    /// Açık olan yöntemler — arayüzün yöntem seçicisini çizmek için.
    #[must_use]
    pub fn enabled_methods(&self) -> Vec<SignatureMethod> {
        self.providers
            .iter()
            .filter(|p| p.state() == ProviderState::Enabled)
            .map(|p| p.method())
            .collect()
    }
}
