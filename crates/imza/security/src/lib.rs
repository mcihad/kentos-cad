//! KentOS İmza güvenlik katmanı (docs/adr/9002 §5).
//!
//! Üç kapı, üçü de imza başlamadan geçilmek zorunda:
//!
//! 1. [`signed_request`] — istek gerçekten KentOS sunucusundan mı geliyor,
//! 2. [`replay`] — bu istek daha önce kullanıldı mı,
//! 3. [`digest`] — belge, istek üretildiğinden beri değişti mi.
//!
//! **Üçü de geçmeden imza başlamaz.** Sıra da önemlidir: imzalı zarf doğrulanmadan gövdedeki hiçbir alana (süre, dosya
//! listesi, özetler) güvenilemez — onlar saldırganın yazdığı veri olabilir.
//!
//! Zarfı **üreten** taraf ([`issue`]) yalnız `issue` özelliğiyle derlenir ve yalnız sunucu açar.

pub mod digest;
#[cfg(feature = "issue")]
pub mod issue;
pub mod replay;
pub mod signed_request;

pub use replay::ReplayGuard;
pub use signed_request::{
    DEFAULT_AUDIENCE, DEFAULT_ISSUER, Header, PayloadFile, PayloadPackage, SignedRequestPayload,
    TrustConfig, verify,
};
